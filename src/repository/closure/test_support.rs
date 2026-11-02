//! Ticket closure test fixtures shared by the observation and checkpoint
//! tests: an enabled repository with one ticket context, and Git helpers
//! that change only what they name.
use crate::{
    canonical::{self, CanonicalItem, ItemId, Ticket},
    repository::{
        AddRemoteRequest, AuthoringKind, AuthoringTarget, ContextIntent, EnableRepositoryRequest,
        ExpectedPathObservation, OperationId, REGISTRY_FILE, RepositoryService, SaveTicketRequest,
        SetPublicationRemoteRequest, TicketDraft,
    },
};
use git2::Oid;
use std::{fs, path::Path};
use time::OffsetDateTime;

pub(super) const ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";

pub(super) fn ticket() -> String {
    format!(".manyhands/tickets/{ID}/ticket.md")
}
pub(super) fn id() -> ItemId {
    ID.parse().unwrap()
}
pub(super) fn at(seconds: i64) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(seconds).unwrap()
}
pub(super) struct Fixture {
    pub(super) root: tempfile::TempDir,
    pub(super) data: tempfile::TempDir,
    pub(super) service: RepositoryService,
    pub(super) repo: git2::Repository,
    pub(super) context: git2::Repository,
    pub(super) base: Oid,
}

pub(super) fn commit(repo: &git2::Repository) -> Oid {
    let mut index = repo.index().unwrap();
    index
        .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.update_all(["*"], None).unwrap();
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let sig = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        "fixture",
        &tree,
        &parent.iter().collect::<Vec<_>>(),
    )
    .unwrap()
}

/// A path with its new bytes and mode, or `None` to remove it.
pub(super) type Edit<'a> = (&'a str, Option<(&'a [u8], u32)>);

/// A commit with `edits` (bytes and mode, or removal) that changes no
/// checked-out file, index or ref.
pub(super) fn tree_commit(repo: &git2::Repository, parent: Oid, edits: &[Edit<'_>]) -> Oid {
    let mut index = git2::Index::new().unwrap();
    index
        .read_tree(&repo.find_commit(parent).unwrap().tree().unwrap())
        .unwrap();
    for (path, edit) in edits {
        if let Some((bytes, mode)) = edit {
            let entry = git2::IndexEntry {
                ctime: git2::IndexTime::new(0, 0),
                mtime: git2::IndexTime::new(0, 0),
                dev: 0,
                ino: 0,
                mode: *mode,
                uid: 0,
                gid: 0,
                file_size: bytes.len() as u32,
                id: repo.blob(bytes).unwrap(),
                flags: 0,
                flags_extended: 0,
                path: path.as_bytes().to_vec(),
            };
            index.add(&entry).unwrap();
        } else {
            index.remove_path(Path::new(path)).unwrap();
        }
    }
    let tree = repo.find_tree(index.write_tree_to(repo).unwrap()).unwrap();
    let sig = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
    repo.commit(
        None,
        &sig,
        &sig,
        "fixture detached",
        &tree,
        &[&repo.find_commit(parent).unwrap()],
    )
    .unwrap()
}

pub(super) fn write(repo: &git2::Repository, path: &str, bytes: impl AsRef<[u8]>) {
    let path = repo.workdir().unwrap().join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}
pub(super) fn read(repo: &git2::Repository, path: &str) -> Vec<u8> {
    fs::read(repo.workdir().unwrap().join(path)).unwrap()
}
pub(super) fn checkout(repo: &git2::Repository, oid: Oid) {
    repo.reset(
        &repo.find_object(oid, None).unwrap(),
        git2::ResetType::Hard,
        None,
    )
    .unwrap();
}
pub(super) fn tree_bytes(repo: &git2::Repository, commit: Oid, path: &str) -> Vec<u8> {
    let tree = repo.find_commit(commit).unwrap().tree().unwrap();
    let entry = tree.get_path(Path::new(path)).unwrap();
    repo.find_blob(entry.id()).unwrap().content().to_vec()
}
impl Fixture {
    pub(super) fn new(remote: bool) -> Self {
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(root.path()).unwrap();
        let mut config = repo.config().unwrap();
        config.set_bool("core.autocrlf", false).unwrap();
        config.set_str("user.name", "Fixture").unwrap();
        config
            .set_str("user.email", "fixture@example.invalid")
            .unwrap();
        repo.set_head("refs/heads/main").unwrap();
        write(&repo, "ordinary.txt", "original\n");
        commit(&repo);
        let service = RepositoryService::open_at(data.path()).unwrap();
        service
            .enable(EnableRepositoryRequest {
                root: root.path().into(),
                primary_branch: "main".into(),
                identity: None,
                operation_id: OperationId::new(),
            })
            .unwrap();
        commit(&repo);
        if remote {
            service
                .add_remote(AddRemoteRequest {
                    root: root.path().into(),
                    name: "origin".into(),
                    url: "ssh://fixture@127.0.0.1:1/repository".into(),
                    operation_id: OperationId::new(),
                })
                .unwrap();
            service
                .set_publication_remote(SetPublicationRemoteRequest {
                    root: root.path().into(),
                    name: Some("origin".into()),
                    operation_id: OperationId::new(),
                })
                .unwrap();
        }
        service
            .save_ticket(SaveTicketRequest {
                target: AuthoringTarget {
                    root: root.path().into(),
                    kind: AuthoringKind::Ticket,
                    item_id: id(),
                    intent: ContextIntent::Create,
                    operation_id: OperationId::new(),
                },
                draft: TicketDraft {
                    title: "Ticket".into(),
                    ticket_type: "task".into(),
                    status: "open".into(),
                    project: None,
                    team: None,
                    body: "original body\n".into(),
                },
                expected_path: ExpectedPathObservation::Missing,
            })
            .unwrap();
        let context =
            git2::Repository::open(root.path().join(".manyhands/worktrees").join(ID)).unwrap();
        // Put the original ticket on the primary so both sides can change it.
        checkout(&repo, context.head().unwrap().target().unwrap());
        let base = repo.head().unwrap().target().unwrap();
        let source = String::from_utf8(read(&context, &ticket()))
            .unwrap()
            .replace("original body", "context body");
        write(&context, &ticket(), source);
        commit(&context);
        let f = Self {
            root,
            data,
            service,
            repo,
            context,
            base,
        };
        if remote {
            f.db()
                .execute("UPDATE remote_polling_state SET history_unknown=0", [])
                .unwrap();
            f.tracking("main", base);
        }
        f
    }
    pub(super) fn db(&self) -> rusqlite::Connection {
        rusqlite::Connection::open(self.data.path().join(REGISTRY_FILE)).unwrap()
    }
    pub(super) fn tracking(&self, branch: &str, oid: Oid) {
        self.repo
            .reference(
                &format!("refs/remotes/origin/{branch}"),
                oid,
                true,
                "fixture",
            )
            .unwrap();
    }
}

pub(super) fn hand_written(line_end: &str) -> String {
    [
        "---",
        "manyhands_managed: true",
        "manyhands_kind: \"ticket\"",
        &format!("id: \"{ID}\""),
        "wave: 2",
        "cycle: '10'",
        "title: 'A quoted: title'",
        "type: task",
        "status: \"open\"",
        "nested:",
        "  key: [1, two]",
        "---",
        "",
        "Body line one",
        "Body line two",
        "",
    ]
    .join(line_end)
}

pub(super) fn parse_ticket(bytes: &[u8]) -> Ticket {
    match canonical::parse_item(Path::new(&ticket()), std::str::from_utf8(bytes).unwrap()).unwrap()
    {
        CanonicalItem::Ticket(t) => t,
        other => panic!("not a ticket: {other:?}"),
    }
}

/// This repository's own ticket shape: `manyhands_*` keys first, quoted
/// scalars, the unknown `wave`/`cycle` keys and a Markdown body.
pub(super) fn repository_ticket(line_end: &str) -> String {
    [
        "---",
        "manyhands_managed: true",
        "manyhands_kind: ticket",
        &format!("id: \"{ID}\""),
        "title: \"Wave 02 Cycle 10: Confirmed Ticket Closure\"",
        "type: \"cycle\"",
        "status: \"open\"",
        "project: \"manyhands\"",
        "team: \"core\"",
        "wave: \"02\"",
        "cycle: \"10\"",
        "---",
        "",
        "Close a ticket with its required closure metadata.",
        "",
        "## Planning",
        "",
        "- [x] Create the implementation plan.",
        "",
    ]
    .join(line_end)
}
