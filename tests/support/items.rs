//! Fixtures for the item reads: sources with chosen IDs, commits with
//! chosen times, and an index refreshed over them.

use std::{
    fs,
    path::{Path, PathBuf},
};

use git2::{Signature, Time};
use manyhands::{
    canonical::ItemId,
    repository::{
        AuthoringKind, AuthoringTarget, ContextIntent, DocumentDraft, EnableRepositoryOutcome,
        ExpectedPathObservation, FailurePoint, OperationId, RebuildRepositoryRequest,
        RefreshOutcome, RefreshRepositoryRequest, RepositoryService, SaveDocumentRequest,
        SaveOutcome,
    },
};
use rusqlite::Connection;

use super::{EnabledRepository, FailOnce, TestRepository};

pub const DOCUMENT_A: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD0";
pub const DOCUMENT_B: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD1";
pub const DOCUMENT_C: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD2";
pub const TICKET_A: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC0";
pub const TICKET_B: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC1";
pub const TICKET_C: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC2";

/// When the contract fixture's first commit of items was made.
pub const COMMITTED_AT: i64 = 1_700_000_000;

pub fn item_id(id: &str) -> ItemId {
    id.parse().unwrap()
}

pub fn write(root: &Path, path: &str, source: &str) -> PathBuf {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, source).unwrap();
    path
}

pub fn ticket_path(id: &str) -> String {
    format!(".manyhands/tickets/{id}/ticket.md")
}

/// A ticket's source. `extra` is front matter lines, each ending in a
/// newline, placed after the required fields.
pub fn ticket_source(id: &str, title: &str, extra: &str) -> String {
    ticket_source_with(id, title, "task", "open", extra)
}

pub fn ticket_source_with(
    id: &str,
    title: &str,
    ticket_type: &str,
    status: &str,
    extra: &str,
) -> String {
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {id}\ntitle: {title}\n\
         type: {ticket_type}\nstatus: {status}\n{extra}---\nBody of {title}.\n"
    )
}

pub fn document_source(id: &str, title: &str, extra: &str) -> String {
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: {id}\ntitle: {title}\n\
         {extra}---\n# {title}\n"
    )
}

/// Front matter that closes a ticket.
pub const CLOSURE: &str =
    "closed_at: 2026-09-30T12:34:56Z\nclosed_by: Ada Lovelace <ada@example.invalid>\n";

/// Commits `paths` as they are on disk, at `seconds`, so that each one's
/// content-change time is that commit's.
pub fn commit(fixture: &TestRepository, paths: &[&str], seconds: i64) {
    let mut index = fixture.repository.index().unwrap();
    index.read(true).unwrap();
    // Enabling commits the configuration without refreshing this index.
    index.add_path(Path::new(".manyhands/config.toml")).unwrap();
    for path in paths {
        index.add_path(Path::new(path)).unwrap();
    }
    index.write().unwrap();
    let tree = fixture
        .repository
        .find_tree(index.write_tree().unwrap())
        .unwrap();
    let parent = fixture.repository.head().unwrap().peel_to_commit().unwrap();
    let signature = Signature::new(
        "Manyhands Test",
        "manyhands-test@example.invalid",
        &Time::new(seconds, 0),
    )
    .unwrap();
    fixture
        .repository
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            "fixture",
            &tree,
            &[&parent],
        )
        .unwrap();
}

pub fn refresh(service: &RepositoryService, root: &Path) -> RefreshOutcome {
    refresh_as(service, root, super::operation_id())
}

/// A refresh that did not complete is retried under its own operation ID.
pub fn refresh_as(
    service: &RepositoryService,
    root: &Path,
    operation_id: OperationId,
) -> RefreshOutcome {
    service
        .refresh_repository(RefreshRepositoryRequest {
            root: root.to_owned(),
            operation_id,
        })
        .unwrap()
}

pub fn refresh_completely(service: &RepositoryService, root: &Path) {
    assert!(matches!(
        refresh(service, root),
        RefreshOutcome::Refreshed { .. }
    ));
}

pub fn rebuild(service: &RepositoryService, root: &Path) {
    rebuild_as(service, root, super::operation_id());
}

pub fn rebuild_as(service: &RepositoryService, root: &Path, operation_id: OperationId) {
    service
        .rebuild_repository(RebuildRepositoryRequest {
            root: root.to_owned(),
            operation_id,
        })
        .unwrap();
}

/// A writable connection to the index, for arranging what no public call
/// can and for asserting what is stored.
pub fn index(data_directory: &Path) -> Connection {
    Connection::open(data_directory.join("manyhands.sqlite3")).unwrap()
}

/// Creates the item worktree for a new document and saves it there.
pub fn create_document_context(service: &RepositoryService, root: &Path, id: &str, path: &str) {
    // Enabling commits the configuration without refreshing the Git index,
    // and a context is not prepared while that path looks changed.
    let repository = git2::Repository::open(root).unwrap();
    let mut index = repository.index().unwrap();
    index.read(true).unwrap();
    index.add_path(Path::new(".manyhands/config.toml")).unwrap();
    index.write().unwrap();
    let outcome = service
        .save_document(SaveDocumentRequest {
            target: AuthoringTarget {
                root: root.to_owned(),
                kind: AuthoringKind::Document,
                item_id: item_id(id),
                intent: ContextIntent::Create,
                operation_id: super::new_operation_id(),
            },
            source_path: None,
            destination_path: PathBuf::from(path),
            draft: DocumentDraft {
                title: format!("Draft of {path}"),
                body: String::new(),
            },
            expected_source: None,
            expected_destination: ExpectedPathObservation::Missing,
        })
        .unwrap();
    assert!(!matches!(outcome, SaveOutcome::IdentityRequired { .. }));
}

pub fn context_worktree(root: &Path, id: &str) -> PathBuf {
    root.join(".manyhands/worktrees")
        .join(id)
        .canonicalize()
        .unwrap()
}

/// A registration whose first refresh, the one enabling starts, did not
/// complete. `operation_id` is the pending operation a retry resumes.
pub struct NeverRefreshed {
    pub fixture: TestRepository,
    pub data: tempfile::TempDir,
    pub service: RepositoryService,
    pub operation_id: OperationId,
}

pub fn never_refreshed_repository() -> NeverRefreshed {
    let fixture = super::born_repository();
    let data = tempfile::tempdir().unwrap();
    let operation_id = super::operation_id();
    let failing =
        FailOnce::at(FailurePoint::BeforeIndexTransactionCommit).open_service(data.path());
    let outcome = failing
        .enable(super::enable_request_with_operation_id(
            &fixture.root,
            operation_id,
        ))
        .unwrap();
    assert!(
        matches!(outcome, EnableRepositoryOutcome::IndexPending(_)),
        "{outcome:?}"
    );
    drop(failing);
    let service = RepositoryService::open_at(data.path()).unwrap();
    NeverRefreshed {
        fixture,
        data,
        service,
        operation_id,
    }
}

/// A service over an index that cannot be read. The repository was
/// registered in it, so it can still be named.
pub fn degraded_service(enabled: EnabledRepository) -> (tempfile::TempDir, RepositoryService) {
    let EnabledRepository {
        service,
        data_directory,
    } = enabled;
    drop(service);
    for entry in fs::read_dir(data_directory.path()).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_str().unwrap().to_owned();
        if name.starts_with("manyhands.sqlite3") && !name.ends_with(".lock") {
            fs::remove_file(path).unwrap();
        }
    }
    fs::write(
        data_directory.path().join("manyhands.sqlite3"),
        b"not sqlite",
    )
    .unwrap();
    let service = RepositoryService::open_at(data_directory.path()).unwrap();
    (data_directory, service)
}

/// The repository the contract fixtures are read from: two documents and
/// two tickets, one of them closed, committed at fixed times and indexed.
pub fn contract_repository() -> (TestRepository, EnabledRepository) {
    let fixture = super::born_repository();
    let enabled = super::enabled_repository(&fixture);
    let root = &fixture.root;
    write(
        root,
        "docs/guide.md",
        &document_source(DOCUMENT_A, "Guide", "audience: everyone\n"),
    );
    write(
        root,
        "docs/notes/plan.md",
        &document_source(DOCUMENT_B, "Plan", ""),
    );
    commit(
        &fixture,
        &["docs/guide.md", "docs/notes/plan.md"],
        COMMITTED_AT,
    );
    let open = ticket_path(TICKET_A);
    write(
        root,
        &open,
        &ticket_source(
            TICKET_A,
            "Open ticket",
            "project: manyhands\nteam: core\npriority: 2\nlabels: [one, two]\n",
        ),
    );
    commit(&fixture, &[&open], COMMITTED_AT + 100);
    let closed = ticket_path(TICKET_B);
    write(
        root,
        &closed,
        &ticket_source_with(TICKET_B, "Closed ticket", "bug", "done", CLOSURE),
    );
    commit(&fixture, &[&closed], COMMITTED_AT + 200);
    refresh_completely(&enabled.service, root);
    (fixture, enabled)
}

pub const COMMENT_A: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE0";
pub const COMMENT_B: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE1";
pub const COMMENT_C: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE2";
pub const COMMENT_D: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE3";
pub const COMMENT_E: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE4";

pub fn comment_path(item: &str, id: &str) -> String {
    format!(".manyhands/comments/{item}/{id}.md")
}

/// A comment's source. `created_at` is an RFC 3339 time; `extra` is front
/// matter lines, each ending in a newline, placed after the required
/// fields.
pub fn comment_source(
    item: &str,
    id: &str,
    parent: Option<&str>,
    created_at: &str,
    extra: &str,
) -> String {
    let parent = parent
        .map(|parent| format!("parent_id: {parent}\n"))
        .unwrap_or_default();
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: comment\nid: {id}\nitem_id: {item}\n\
         {parent}created_at: {created_at}\n{extra}---\nBody of {id}.\n"
    )
}

/// Writes a comment on `item` under `root`, which is a context's worktree.
pub fn write_comment(
    root: &Path,
    item: &str,
    id: &str,
    parent: Option<&str>,
    created_at: &str,
    extra: &str,
) -> PathBuf {
    write(
        root,
        &comment_path(item, id),
        &comment_source(item, id, parent, created_at, extra),
    )
}
