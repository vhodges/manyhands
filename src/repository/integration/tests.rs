use super::{Error, artifacts, prepare, resolution, validate};
use std::{cell::Cell, collections::BTreeSet, path::Path};

fn commit(repo: &git2::Repository, text: &str, parent: Option<git2::Oid>) -> git2::Oid {
    let blob = repo.blob(text.as_bytes()).unwrap();
    let mut builder = repo.treebuilder(None).unwrap();
    builder.insert("file.txt", blob, 0o100644).unwrap();
    let tree = repo.find_tree(builder.write().unwrap()).unwrap();
    let signature =
        git2::Signature::new("Fixture", "fixture@example.test", &git2::Time::new(0, 0)).unwrap();
    let parents = parent.map(|oid| repo.find_commit(oid).unwrap());
    repo.commit(
        None,
        &signature,
        &signature,
        "fixture",
        &tree,
        &parents.iter().collect::<Vec<_>>(),
    )
    .unwrap()
}

fn fixture() -> (tempfile::TempDir, git2::Repository, git2::Oid, git2::Oid) {
    let dir = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();
    repo.config()
        .unwrap()
        .set_bool("core.autocrlf", false)
        .unwrap();
    let base = commit(&repo, "one\ntwo\nthree\nfour\nfive\n", None);
    let local = commit(&repo, "ONE\ntwo\nthree\nfour\nfive\n", Some(base));
    let incoming = commit(&repo, "one\ntwo\nthree\nfour\nFIVE\n", Some(base));
    repo.reference("refs/heads/main", local, true, "fixture")
        .unwrap();
    repo.set_head("refs/heads/main").unwrap();
    repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .unwrap();
    (dir, repo, local, incoming)
}

fn objects(repo: &git2::Repository) -> BTreeSet<git2::Oid> {
    let mut result = BTreeSet::new();
    repo.odb()
        .unwrap()
        .foreach(|oid| {
            result.insert(*oid);
            true
        })
        .unwrap();
    result
}

#[test]
fn memory_preparation_has_no_persistent_objects_refs_or_index_effect() {
    let (dir, repo, local, incoming) = fixture();
    let before = objects(&repo);
    let index = std::fs::read(repo.path().join("index")).unwrap();
    let refs = std::fs::read(repo.path().join("refs/heads/main")).unwrap();
    let worktree = std::fs::read(dir.path().join("file.txt")).unwrap();
    let prepare::MergePreparation::Clean(prepared) =
        prepare::prepare_clean_merge(dir.path(), local, incoming).unwrap()
    else {
        panic!("clean merge expected")
    };
    let expected_blob =
        git2::Oid::hash_object(git2::ObjectType::Blob, b"ONE\ntwo\nthree\nfour\nFIVE\n").unwrap();
    assert!(!repo.odb().unwrap().exists(expected_blob));
    assert_eq!(objects(&repo), before);
    assert_eq!(std::fs::read(repo.path().join("index")).unwrap(), index);
    assert_eq!(
        std::fs::read(repo.path().join("refs/heads/main")).unwrap(),
        refs
    );
    assert_eq!(
        std::fs::read(dir.path().join("file.txt")).unwrap(),
        worktree
    );
    // Positive control: the separate import really persists the generated blob.
    prepare::import_prepared_tree(&repo, &prepared).unwrap();
    assert!(repo.odb().unwrap().exists(expected_blob));
}

#[test]
fn owner_loss_between_preparation_and_mutation_has_no_git_effect() {
    let (dir, repo, local, incoming) = fixture();
    let authority = Cell::new(true);
    let prepare::MergePreparation::Clean(prepared) =
        prepare::prepare_clean_merge(dir.path(), local, incoming).unwrap()
    else {
        panic!("clean merge expected")
    };
    let _lease = crate::repository::repository_lease(
        &repo,
        dir.path(),
        crate::repository::RepositoryOperation::RepositorySnapshot,
    )
    .unwrap();
    let before = objects(&repo);
    let index = std::fs::read(repo.path().join("index")).unwrap();
    let worktree = std::fs::read(dir.path().join("file.txt")).unwrap();
    authority.set(false);
    let owner = || {
        if authority.get() {
            Ok(())
        } else {
            Err(Error::ExternalChange)
        }
    };
    assert_eq!(
        prepare::import_owned_tree(&repo, &prepared, owner),
        Err(Error::ExternalChange)
    );
    assert_eq!(
        artifacts::fast_forward(&repo, "refs/heads/main", local, incoming, owner, || {}),
        Err(Error::ExternalChange)
    );
    assert_eq!(objects(&repo), before);
    assert_eq!(repo.refname_to_id("refs/heads/main").unwrap(), local);
    assert_eq!(std::fs::read(repo.path().join("index")).unwrap(), index);
    assert_eq!(
        std::fs::read(dir.path().join("file.txt")).unwrap(),
        worktree
    );
    assert!(!repo.path().join("refs/heads/main.lock").exists());
}

#[test]
fn mutable_path_preflight_refuses_unproven_paths_without_writes() {
    let (dir, repo, _, _) = fixture();
    let before = objects(&repo);
    let index = std::fs::read(repo.path().join("index")).unwrap();
    for path in ["../escape", "/absolute", "", ".git"] {
        assert!(
            resolution::conflict_worktree_digest(dir.path(), Path::new(path)).is_err(),
            "unsafe fixture path accepted: {path}"
        );
        assert!(resolution::resolution_preflight(&repo, &[(0, path.as_bytes())]).is_err());
    }
    // Absence is a valid preimage; token/canonical authorization belongs to the
    // adapter, not this filesystem-image reader.
    assert!(
        resolution::conflict_worktree_digest(dir.path(), Path::new("missing-parent/file")).is_ok()
    );
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("file.txt", dir.path().join("alias")).unwrap();
        assert!(resolution::conflict_worktree_digest(dir.path(), Path::new("alias")).is_err());
        assert!(resolution::resolution_preflight(&repo, &[(0, b"alias")]).is_err());
    }
    assert_eq!(objects(&repo), before);
    assert_eq!(std::fs::read(repo.path().join("index")).unwrap(), index);
}

const TICKET_PATH: &str = ".manyhands/tickets/01BX5ZZKBKACTAV9WEVGEMMVRZ/ticket.md";

/// The closure a fixture ticket holds, by letter: `N` none, `A` a closure,
/// `B` the same instant by another author, `T` another instant by the same
/// author (a closure is compared by both fields).
fn ticket_text(closure: char, body: &str) -> String {
    let (closed_at, closed_by) = match closure {
        'N' => ("", ""),
        'A' => ("2026-01-01T00:00:00Z", "original author"),
        'B' => ("2026-01-01T00:00:00Z", "different author"),
        'T' => ("2026-02-01T00:00:00Z", "original author"),
        other => panic!("closure {other}"),
    };
    let status = if closure == 'N' { "open" } else { "closed" };
    let closure_fields = if closure == 'N' {
        String::new()
    } else {
        format!("\nclosed_at: \"{closed_at}\"\nclosed_by: \"{closed_by}\"")
    };
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: \"01BX5ZZKBKACTAV9WEVGEMMVRZ\"\ntitle: \"Ticket\"\ntype: \"cycle\"\nstatus: \"{status}\"{closure_fields}\n---\n\n{body}\n"
    )
}

fn commit_ticket(repo: &git2::Repository, text: &str, parents: &[git2::Oid]) -> git2::Oid {
    let blob = repo.blob(text.as_bytes()).unwrap();
    let mut tree = repo.treebuilder(None).unwrap();
    tree.insert("ticket.md", blob, 0o100644).unwrap();
    let mut directory = repo.treebuilder(None).unwrap();
    directory
        .insert(
            "01BX5ZZKBKACTAV9WEVGEMMVRZ",
            tree.write().unwrap(),
            0o040000,
        )
        .unwrap();
    let mut tickets = repo.treebuilder(None).unwrap();
    tickets
        .insert("tickets", directory.write().unwrap(), 0o040000)
        .unwrap();
    let mut root = repo.treebuilder(None).unwrap();
    root.insert(".manyhands", tickets.write().unwrap(), 0o040000)
        .unwrap();
    let tree = repo.find_tree(root.write().unwrap()).unwrap();
    let signature =
        git2::Signature::new("Fixture", "fixture@example.test", &git2::Time::new(0, 0)).unwrap();
    let parents = parents
        .iter()
        .map(|oid| repo.find_commit(*oid).unwrap())
        .collect::<Vec<_>>();
    repo.commit(
        None,
        &signature,
        &signature,
        "fixture",
        &tree,
        &parents.iter().collect::<Vec<_>>(),
    )
    .unwrap()
}

/// R-12. A primary side gives the result's closure three sides' worth of
/// freedom: the primary's own when it holds one, otherwise none or the local
/// or the incoming side's, never the base's alone. No primary side (a primary
/// target, window 0, a local synchronization) keeps every side's closure.
#[test]
fn provisional_closures_may_change_primary_ones_may_not() {
    // base, local, incoming, primary (None: no primary side is given), result,
    // accepted. Rows marked strict pin today's rule and are exempt from
    // fail-first.
    type Row = (char, char, char, Option<char>, char, bool);
    let rows: &[Row] = &[
        // The primary holds a closure: exactly it.
        ('N', 'N', 'N', Some('A'), 'A', true),
        ('N', 'N', 'N', Some('A'), 'N', false),
        ('N', 'N', 'N', Some('A'), 'B', false),
        ('N', 'N', 'N', Some('A'), 'T', false),
        ('A', 'A', 'A', Some('A'), 'A', true),
        ('A', 'A', 'A', Some('T'), 'T', true),
        ('A', 'A', 'A', Some('T'), 'A', false),
        ('N', 'A', 'B', Some('T'), 'A', false),
        // The primary is open: a provisional closure may change.
        ('A', 'A', 'A', Some('N'), 'N', true),
        ('A', 'A', 'A', Some('N'), 'A', true),
        ('N', 'A', 'N', Some('N'), 'A', true),
        ('N', 'N', 'A', Some('N'), 'A', true),
        ('N', 'A', 'B', Some('N'), 'B', true),
        ('N', 'A', 'B', Some('N'), 'A', true),
        ('N', 'A', 'B', Some('N'), 'N', true),
        ('A', 'B', 'T', Some('N'), 'B', true),
        ('A', 'B', 'T', Some('N'), 'T', true),
        ('A', 'B', 'T', Some('N'), 'N', true),
        // The base's closure alone, or one no side holds, is refused.
        ('A', 'N', 'N', Some('N'), 'A', false),
        ('A', 'N', 'N', Some('N'), 'N', true),
        ('A', 'B', 'T', Some('N'), 'A', false),
        ('N', 'N', 'N', Some('N'), 'T', false),
        // No primary side: today's rule, each side's closure is kept (strict).
        ('N', 'N', 'N', None, 'N', true),
        ('N', 'A', 'N', None, 'A', true),
        ('N', 'A', 'N', None, 'N', false),
        ('N', 'N', 'A', None, 'A', true),
        ('A', 'A', 'A', None, 'A', true),
        ('A', 'A', 'A', None, 'N', false),
        ('A', 'N', 'N', None, 'A', true),
        ('A', 'N', 'N', None, 'N', false),
        ('N', 'A', 'B', None, 'A', false),
        ('N', 'A', 'A', None, 'B', false),
        ('N', 'N', 'N', None, 'A', true),
    ];
    let mut wrong = Vec::new();
    for &(base, local, incoming, primary, result, accepted) in rows {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let base_oid = commit_ticket(&repo, &ticket_text(base, "base"), &[]);
        let local_oid = commit_ticket(&repo, &ticket_text(local, "local"), &[base_oid]);
        let incoming_oid = commit_ticket(&repo, &ticket_text(incoming, "incoming"), &[base_oid]);
        let primary_oid =
            primary.map(|closure| commit_ticket(&repo, &ticket_text(closure, "primary"), &[]));
        let result_text = ticket_text(result, "result");
        let result_oid = commit_ticket(&repo, &result_text, &[local_oid, incoming_oid]);
        let blob = |text: &str| Some(repo.blob(text.as_bytes()).unwrap());
        let label = format!("{base}{local}{incoming} primary {primary:?} -> {result}");
        let resolution = validate::validates_resolution_sides(
            &repo,
            TICKET_PATH.as_bytes(),
            [
                blob(&ticket_text(base, "base")),
                blob(&ticket_text(local, "local")),
                blob(&ticket_text(incoming, "incoming")),
            ],
            primary_oid,
            result_text.as_bytes(),
        )
        .unwrap();
        let external = validate::validates_external_integration(
            &repo,
            local_oid,
            incoming_oid,
            primary_oid,
            result_oid,
        )
        .unwrap();
        if resolution != accepted {
            wrong.push(format!("resolution {label}: {resolution}"));
        }
        if external != accepted {
            wrong.push(format!("external {label}: {external}"));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}
