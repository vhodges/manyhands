use super::proof::{self, PreservationProof::*, ProofRefusal::*};
use crate::{canonical::ItemId, repository::AuthoringKind};
use std::{collections::BTreeMap, path::Path};

const ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
type Files = BTreeMap<Vec<u8>, (Vec<u8>, i32)>;

fn ticket(closed: bool) -> Vec<u8> {
    format!("---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {ID}\ntitle: Example\ntype: task\nstatus: open\ncustom: kept\n{}---\nBody\n", if closed { "closed_at: '2026-11-04T00:00:00Z'\nclosed_by: 'Owner <owner@example.com>'\n" } else { "" }).into_bytes()
}
fn path() -> Vec<u8> {
    format!(".manyhands/tickets/{ID}/ticket.md").into_bytes()
}
fn files(closed: bool) -> Files {
    BTreeMap::from([
        (path(), (ticket(closed), 0o100644)),
        (b"code".to_vec(), (b"base".to_vec(), 0o100644)),
    ])
}
fn commit(repo: &git2::Repository, files: &Files, parents: &[git2::Oid]) -> git2::Oid {
    let mut index = git2::Index::new().unwrap();
    for (path, (bytes, mode)) in files {
        let entry = git2::IndexEntry {
            ctime: git2::IndexTime::new(0, 0),
            mtime: git2::IndexTime::new(0, 0),
            dev: 0,
            ino: 0,
            mode: *mode as u32,
            uid: 0,
            gid: 0,
            file_size: bytes.len() as u32,
            id: repo.blob(bytes).unwrap(),
            flags: 0,
            flags_extended: 0,
            path: path.clone(),
        };
        index.add(&entry).unwrap();
    }
    let tree = repo.find_tree(index.write_tree_to(repo).unwrap()).unwrap();
    let parents = parents
        .iter()
        .map(|id| repo.find_commit(*id).unwrap())
        .collect::<Vec<_>>();
    let sig = git2::Signature::now("Proof", "proof@example.com").unwrap();
    repo.commit(
        None,
        &sig,
        &sig,
        "proof fixture",
        &tree,
        &parents.iter().collect::<Vec<_>>(),
    )
    .unwrap()
}
fn prove(
    repo: &git2::Repository,
    local: git2::Oid,
    primary: git2::Oid,
) -> Result<super::proof::PreservationProof, super::proof::ProofRefusal> {
    proof::preservation(
        repo,
        local,
        primary,
        AuthoringKind::Ticket,
        &ID.parse::<ItemId>().unwrap(),
    )
}

#[test]
fn retirement_proof_ancestor_requires_positive_completion() {
    for case in [
        "closed",
        "open",
        "status_only",
        "malformed",
        "missing",
        "nonregular",
        "duplicate",
        "wrong_id",
    ] {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let local = commit(&repo, &files(false), &[]);
        let mut primary = files(true);
        match case {
            "open" => primary = files(false),
            "status_only" => {
                primary.get_mut(&path()).unwrap().0 = String::from_utf8(ticket(false))
                    .unwrap()
                    .replace("status: open", "status: closed")
                    .into_bytes()
            }
            "malformed" => {
                primary.get_mut(&path()).unwrap().0 = String::from_utf8(ticket(true))
                    .unwrap()
                    .replace("closed_by:", "not_closed_by:")
                    .into_bytes()
            }
            "missing" => {
                primary.remove(&path());
            }
            "nonregular" => primary.get_mut(&path()).unwrap().1 = 0o120000,
            "wrong_id" => {
                primary.get_mut(&path()).unwrap().0 = String::from_utf8(ticket(true))
                    .unwrap()
                    .replace(ID, "01ARZ3NDEKTSV4RRFFQ69G5FAW")
                    .into_bytes();
            }
            "duplicate" => {
                primary.insert(b"docs/duplicate.md".to_vec(), (format!("---\nmanyhands_managed: true\nmanyhands_kind: document\nid: {ID}\ntitle: Duplicate\n---\n").into_bytes(),0o100644));
            }
            _ => {}
        }
        let primary = commit(&repo, &primary, &[local]);
        let expected = match case {
            "closed" => Ok(Ancestor),
            "open" | "status_only" => Err(TicketOpen),
            _ => Err(InvalidPrimaryItem),
        };
        assert_eq!(prove(&repo, local, primary), expected, "{case}");
        if case == "closed" {
            assert_eq!(prove(&repo, primary, primary), Ok(Ancestor));
        }
    }
}

#[test]
fn retirement_proof_folded_full_changed_entry_matrix() {
    for case in [
        "folded",
        "normalized",
        "status",
        "title",
        "type",
        "project",
        "team",
        "body",
        "unknown",
        "slug",
        "relations",
        "comment",
        "code",
        "deleted",
        "deletion_missing",
        "rename",
        "rename_missing",
        "mode",
        "ticket_exec_local",
        "ticket_exec_primary",
        "ticket_exec_equal",
        "different_closed",
        "inherited",
        "raw_path",
        "typechange",
    ] {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let mut base_files = files(false);
        // Make the ticket itself part of the base->local delta, so both
        // executable-bit directions exercise the semantic-exception gate.
        base_files.get_mut(&path()).unwrap().0 = String::from_utf8(ticket(false))
            .unwrap()
            .replace("Body", "Original body")
            .into_bytes();
        let base = commit(&repo, &base_files, &[]);
        let mut local_files = files(false);
        local_files.insert(b"extra".to_vec(), (b"carried".to_vec(), 0o100644));
        let mut primary_files = local_files.clone();
        primary_files.insert(path(), (ticket(true), 0o100644));
        match case {
            "normalized" => {
                local_files.get_mut(&path()).unwrap().0 = String::from_utf8(ticket(false))
                    .unwrap()
                    .replace("custom: kept", "custom: kept\nparent: null\ndeps: []")
                    .into_bytes();
            }
            "status" | "title" | "type" | "project" | "team" | "unknown" | "slug" | "relations" => {
                let text = String::from_utf8(ticket(false)).unwrap();
                let text = match case {
                    "status" => text.replace("status: open", "status: private"),
                    "title" => text.replace("title: Example", "title: Private"),
                    "type" => text.replace("type: task", "type: defect"),
                    "unknown" => text.replace("custom: kept", "custom: private"),
                    "relations" => text.replace(
                        "custom: kept",
                        "custom: kept\ndeps: [01ARZ3NDEKTSV4RRFFQ69G5FAW]",
                    ),
                    _ => text.replace("custom: kept", &format!("custom: kept\n{case}: private")),
                };
                local_files.get_mut(&path()).unwrap().0 = text.into_bytes();
            }
            "body" => local_files
                .get_mut(&path())
                .unwrap()
                .0
                .extend_from_slice(b"Private\n"),
            "comment" => {
                local_files.insert(
                    format!(".manyhands/comments/{ID}/01ARZ3NDEKTSV4RRFFQ69G5FAW.md").into_bytes(),
                    (b"private comment".to_vec(), 0o100644),
                );
            }
            "code" => {
                local_files.insert(b"code".to_vec(), (b"private".to_vec(), 0o100644));
            }
            "deleted" | "deletion_missing" => {
                local_files.remove(b"code".as_slice());
                if case == "deleted" {
                    primary_files.remove(b"code".as_slice());
                }
            }
            "rename" | "rename_missing" => {
                local_files.remove(b"code".as_slice());
                local_files.insert(b"renamed".to_vec(), (b"base".to_vec(), 0o100644));
                primary_files.insert(b"renamed".to_vec(), (b"base".to_vec(), 0o100644));
                if case == "rename" {
                    primary_files.remove(b"code".as_slice());
                }
            }
            "mode" => local_files.get_mut(b"extra".as_slice()).unwrap().1 = 0o100755,
            "ticket_exec_local" => local_files.get_mut(&path()).unwrap().1 = 0o100755,
            "ticket_exec_primary" => primary_files.get_mut(&path()).unwrap().1 = 0o100755,
            "ticket_exec_equal" => {
                local_files.get_mut(&path()).unwrap().1 = 0o100755;
                primary_files.get_mut(&path()).unwrap().1 = 0o100755;
            }
            "different_closed" => {
                local_files.get_mut(&path()).unwrap().0 = String::from_utf8(ticket(true))
                    .unwrap()
                    .replace("00:00:00Z", "00:00:01Z")
                    .into_bytes()
            }
            "inherited" => {
                primary_files.insert(b"code".to_vec(), (b"new primary".to_vec(), 0o100644));
            }
            "raw_path" => {
                local_files.insert(b"raw-\xff".to_vec(), (b"private".to_vec(), 0o100644));
            }
            "typechange" => {
                local_files.get_mut(b"code".as_slice()).unwrap().1 = 0o120000;
            }
            _ => {}
        }
        let local = commit(&repo, &local_files, &[base]);
        let primary = commit(&repo, &primary_files, &[base]);
        assert!(!repo.graph_descendant_of(primary, local).unwrap());
        let success = matches!(
            case,
            "folded" | "normalized" | "deleted" | "rename" | "inherited" | "ticket_exec_equal"
        );
        assert_eq!(
            prove(&repo, local, primary),
            if success {
                Ok(Folded)
            } else {
                Err(ContentMismatch)
            },
            "{case}"
        );
        assert!(
            !dir.path().join(Path::new(".manyhands")).exists(),
            "proof is tree-only"
        );
    }
}

#[test]
fn retirement_proof_document_presence_is_not_preservation() {
    for matching in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let base = commit(&repo, &Files::new(), &[]);
        let doc=format!("---\nmanyhands_managed: true\nmanyhands_kind: document\nid: {ID}\ntitle: Example\n---\nBody\n").into_bytes();
        let local_files = BTreeMap::from([
            (b"docs/example.md".to_vec(), (doc.clone(), 0o100644)),
            (b"code".to_vec(), (b"private".to_vec(), 0o100644)),
        ]);
        let mut primary_files = local_files.clone();
        primary_files.insert(
            b"primary-only".to_vec(),
            (b"newer primary".to_vec(), 0o100644),
        );
        if !matching {
            primary_files.remove(b"code".as_slice());
        }
        let local = commit(&repo, &local_files, &[base]);
        let primary = commit(&repo, &primary_files, &[base]);
        assert!(!repo.graph_descendant_of(primary, local).unwrap());
        assert_eq!(
            proof::preservation(
                &repo,
                local,
                primary,
                AuthoringKind::Document,
                &ID.parse().unwrap()
            ),
            if matching {
                Ok(Folded)
            } else {
                Err(ContentMismatch)
            }
        );
    }
}

#[test]
fn retirement_proof_history_refusals() {
    for case in [
        "missing_commit",
        "missing_blob",
        "unrelated",
        "shallow",
        "multiple_bases",
    ] {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let base = commit(&repo, &files(false), &[]);
        let mut changed = files(false);
        changed.insert(b"extra".to_vec(), (b"local".to_vec(), 0o100644));
        let mut local = commit(&repo, &changed, &[base]);
        let mut primary = commit(
            &repo,
            &files(true),
            if case == "unrelated" {
                &[]
            } else {
                std::slice::from_ref(&base)
            },
        );
        let expected = match case {
            "missing_commit" => {
                local = git2::Oid::from_bytes(&[42; 20]).unwrap();
                MissingObject
            }
            "missing_blob" => {
                let oid = repo.blob(&ticket(true)).unwrap().to_string();
                std::fs::remove_file(repo.path().join("objects").join(&oid[..2]).join(&oid[2..]))
                    .unwrap();
                MissingObject
            }
            "shallow" => {
                std::fs::write(repo.path().join("shallow"), format!("{base}\n")).unwrap();
                ShallowHistory
            }
            "multiple_bases" => {
                let other = commit(&repo, &files(true), &[base]);
                let a = local;
                local = commit(&repo, &changed, &[a, other]);
                primary = commit(&repo, &files(true), &[other, a]);
                MultipleMergeBases
            }
            _ => UnrelatedHistory,
        };
        assert_eq!(prove(&repo, local, primary), Err(expected), "{case}");
    }
}

#[test]
fn retirement_proof_completion_is_independent_of_unreadable_unrelated_content() {
    let dir = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();
    let local = commit(&repo, &files(false), &[]);
    let mut primary_files = files(true);
    primary_files.insert(
        b"unrelated".to_vec(),
        (b"missing unrelated object".to_vec(), 0o100644),
    );
    let primary = commit(&repo, &primary_files, &[local]);
    let oid = repo.blob(b"missing unrelated object").unwrap().to_string();
    std::fs::remove_file(repo.path().join("objects").join(&oid[..2]).join(&oid[2..])).unwrap();
    assert_eq!(
        proof::completion(&repo, primary, AuthoringKind::Ticket, &ID.parse().unwrap()),
        Ok(())
    );
    assert_eq!(prove(&repo, local, primary), Err(MissingObject));
}

#[test]
fn retirement_proof_empty_tree_entries_are_not_silently_omitted() {
    let dir = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();
    let base = commit(&repo, &files(false), &[]);
    let empty = repo.treebuilder(None).unwrap().write().unwrap();
    let parent = repo.find_commit(base).unwrap();
    let mut builder = repo.treebuilder(Some(&parent.tree().unwrap())).unwrap();
    builder.insert("empty-directory", empty, 0o040000).unwrap();
    let tree = repo.find_tree(builder.write().unwrap()).unwrap();
    let sig = git2::Signature::now("Proof", "proof@example.com").unwrap();
    let local = repo
        .commit(None, &sig, &sig, "empty tree change", &tree, &[&parent])
        .unwrap();
    let primary = commit(&repo, &files(true), &[base]);
    assert_eq!(prove(&repo, local, primary), Err(ContentMismatch));
}
