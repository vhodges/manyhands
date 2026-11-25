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

/// A ticket as a product save writes it: serialized canonically, holding the
/// closure `closure` (as in `ticket_text`) and `body`.
fn canonical_ticket(closure: char, body: &str) -> String {
    let (closed_at, closed_by) = match closure {
        'N' => (None, None),
        'A' => (Some("2026-01-01T00:00:00Z"), Some("original author")),
        'B' => (Some("2026-01-01T00:00:00Z"), Some("different author")),
        'T' => (Some("2026-02-01T00:00:00Z"), Some("original author")),
        other => panic!("closure {other}"),
    };
    crate::canonical::serialize_item(&crate::canonical::CanonicalItem::Ticket(
        crate::canonical::Ticket {
            id: "01BX5ZZKBKACTAV9WEVGEMMVRZ".parse().unwrap(),
            title: "Ticket".into(),
            ticket_type: "cycle".into(),
            // Only the two closure fields move with a closure.
            status: "open".into(),
            project: None,
            team: None,
            closed_at: closed_at.map(|at| {
                time::OffsetDateTime::parse(at, &time::format_description::well_known::Rfc3339)
                    .unwrap()
            }),
            closed_by: closed_by.map(str::to_owned),
            body: body.into(),
            unknown: serde_yaml::Mapping::new(),
        },
    ))
    .unwrap()
}

/// A commit of a tree holding exactly `files`, which may be nested.
fn commit_files(
    repo: &git2::Repository,
    files: &[(&str, String)],
    parents: &[git2::Oid],
) -> git2::Oid {
    let empty = repo.treebuilder(None).unwrap().write().unwrap();
    let empty = repo.find_tree(empty).unwrap();
    let mut update = git2::build::TreeUpdateBuilder::new();
    for (path, text) in files {
        update.upsert(
            path,
            repo.blob(text.as_bytes()).unwrap(),
            git2::FileMode::Blob,
        );
    }
    let tree = repo
        .find_tree(update.create_updated(repo, &empty).unwrap())
        .unwrap();
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

/// R-30 and Part G, on the function a ticket context's synchronization calls
/// after `prepare_clean_merge`: a clean merge whose result lacks the closure
/// the fetched primary holds is given it (G7), and a conflict of the ticket's
/// closure alone, between two provisional closures, settles with the remote's
/// (G1). Rows G2 to G6 and the Primary stage pin what stays: the conflict, or
/// the merge as it is (exempt from fail-first).
#[test]
fn closure_only_conflicts_settle_with_the_remote_closure() {
    /// A side of a row: the ticket's closure and body, and its other files.
    type Side = (char, &'static str, &'static [(&'static str, &'static str)]);
    #[derive(Debug, PartialEq)]
    enum Outcome {
        Conflict,
        /// A clean merge whose ticket is the fixture holding this closure.
        Clean(char),
    }
    use Outcome::{Clean, Conflict};
    const NONE: &[(&str, &str)] = &[];
    // row, base, ours, theirs, the fetched primary (none: it holds no ticket),
    // whether this is the Context stage, the outcome.
    #[allow(clippy::type_complexity)]
    let rows: &[(&str, Side, Side, Side, Option<char>, bool, Outcome)] = &[
        (
            "G1 two different provisional closures",
            ('N', "b", NONE),
            ('A', "b", NONE),
            ('B', "b", NONE),
            Some('N'),
            true,
            Clean('B'),
        ),
        (
            "G1 the primary holds no ticket",
            ('N', "b", NONE),
            ('A', "b", NONE),
            ('B', "b", NONE),
            None,
            true,
            Clean('B'),
        ),
        (
            "G1 another instant",
            ('N', "b", NONE),
            ('T', "b", NONE),
            ('A', "b", NONE),
            Some('N'),
            true,
            Clean('A'),
        ),
        (
            "G1 both changed a closure the base held",
            ('A', "b", NONE),
            ('B', "b", NONE),
            ('T', "b", NONE),
            Some('N'),
            true,
            Clean('T'),
        ),
        (
            "G1 with another path changed cleanly by each side",
            ('N', "b", NONE),
            ('A', "b", &[("ours.txt", "ours\n")]),
            ('B', "b", &[("theirs.txt", "theirs\n")]),
            Some('N'),
            true,
            Clean('B'),
        ),
        (
            "G2 the ticket's body conflicts too",
            ('N', "b\n", NONE),
            ('A', "ours\n", NONE),
            ('B', "theirs\n", NONE),
            Some('N'),
            true,
            Conflict,
        ),
        (
            "G2 another path conflicts",
            ('N', "b", &[("other.txt", "base\n")]),
            ('A', "b", &[("other.txt", "ours\n")]),
            ('B', "b", &[("other.txt", "theirs\n")]),
            Some('N'),
            true,
            Conflict,
        ),
        (
            "G2 another path conflicts, added on both sides",
            ('N', "b", NONE),
            ('A', "b", &[("other.txt", "ours\n")]),
            ('B', "b", &[("other.txt", "theirs\n")]),
            Some('N'),
            true,
            Conflict,
        ),
        (
            "G3 the primary holds one of the closures",
            ('N', "b", NONE),
            ('A', "b", NONE),
            ('B', "b", NONE),
            Some('A'),
            true,
            Conflict,
        ),
        (
            "G3 the primary holds the other",
            ('N', "b", NONE),
            ('A', "b", NONE),
            ('B', "b", NONE),
            Some('B'),
            true,
            Conflict,
        ),
        (
            "G3 the primary holds a third",
            ('N', "b", NONE),
            ('A', "b", NONE),
            ('B', "b", NONE),
            Some('T'),
            true,
            Conflict,
        ),
        (
            "the Primary stage settles nothing",
            ('N', "b", NONE),
            ('A', "b", NONE),
            ('B', "b", NONE),
            Some('N'),
            false,
            Conflict,
        ),
        (
            "G4 the same closure on both sides",
            ('N', "b", NONE),
            ('A', "b", &[("ours.txt", "ours\n")]),
            ('A', "b", &[("theirs.txt", "theirs\n")]),
            Some('N'),
            true,
            Clean('A'),
        ),
        (
            "G5 ours withdrawn, theirs unchanged",
            ('A', "b", NONE),
            ('N', "b", NONE),
            ('A', "b", &[("theirs.txt", "theirs\n")]),
            Some('N'),
            true,
            Clean('N'),
        ),
        (
            "G6 ours re-recorded after a withdraw, theirs unchanged",
            ('A', "b", NONE),
            ('B', "b", NONE),
            ('A', "b", &[("theirs.txt", "theirs\n")]),
            Some('N'),
            true,
            Clean('B'),
        ),
        (
            "G7 ours withdrawn what the primary holds",
            ('A', "b", NONE),
            ('N', "b", NONE),
            ('A', "b", &[("theirs.txt", "theirs\n")]),
            Some('A'),
            true,
            Clean('A'),
        ),
        (
            "G7 the Primary stage",
            ('A', "b", NONE),
            ('N', "b", NONE),
            ('A', "b", &[("theirs.txt", "theirs\n")]),
            Some('A'),
            false,
            Clean('A'),
        ),
        (
            "G7 neither side holds one",
            ('N', "b", NONE),
            ('N', "b", &[("ours.txt", "ours\n")]),
            ('N', "b", &[("theirs.txt", "theirs\n")]),
            Some('A'),
            true,
            Clean('A'),
        ),
        (
            "G7 a provisional closure the primary's replaces",
            ('N', "b", NONE),
            ('B', "b", NONE),
            ('N', "b", &[("theirs.txt", "theirs\n")]),
            Some('A'),
            true,
            Clean('A'),
        ),
        (
            "G7 the result already holds the primary's",
            ('N', "b", NONE),
            ('A', "b", NONE),
            ('N', "b", &[("theirs.txt", "theirs\n")]),
            Some('A'),
            true,
            Clean('A'),
        ),
    ];
    let mut wrong = Vec::new();
    for (label, base, ours, theirs, primary, settles, expected) in rows {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let side = |(closure, body, files): &Side| {
            let mut all = vec![(TICKET_PATH, canonical_ticket(*closure, body))];
            all.extend(files.iter().map(|(path, text)| (*path, (*text).to_owned())));
            all
        };
        let base_oid = commit_files(&repo, &side(base), &[]);
        let local = commit_files(&repo, &side(ours), &[base_oid]);
        let incoming = commit_files(&repo, &side(theirs), &[base_oid]);
        let primary_oid = commit_files(
            &repo,
            &match primary {
                Some(closure) => vec![(TICKET_PATH, canonical_ticket(*closure, "primary"))],
                None => vec![("primary.txt", "primary\n".to_owned())],
            },
            &[],
        );
        let prepared = prepare::prepare_clean_merge(dir.path(), local, incoming).unwrap();
        let prepared = prepare::keep_primary_closure(
            dir.path(),
            Path::new(TICKET_PATH),
            local,
            incoming,
            primary_oid,
            *settles,
            prepared,
        )
        .unwrap();
        let outcome = match prepared {
            prepare::MergePreparation::Conflict => Conflict,
            prepare::MergePreparation::Clean(prepared) => {
                let tree = prepare::import_prepared_tree(&repo, &prepared).unwrap();
                let blob = repo
                    .find_tree(tree)
                    .unwrap()
                    .get_path(Path::new(TICKET_PATH))
                    .unwrap()
                    .id();
                let text =
                    String::from_utf8(repo.find_blob(blob).unwrap().content().to_vec()).unwrap();
                // The result's ticket is one of the fixtures, byte for byte.
                ['N', 'A', 'B', 'T']
                    .into_iter()
                    .find(|closure| text == canonical_ticket(*closure, "b"))
                    .map_or_else(|| panic!("{label}: {text}"), Clean)
            }
        };
        if outcome != *expected {
            wrong.push(format!("{label}: {outcome:?}, expected {expected:?}"));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}
