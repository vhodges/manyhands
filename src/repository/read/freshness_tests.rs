//! How a read decides that a file is newer than what the index holds of it:
//! against the time its own context was indexed, not the repository's last
//! refresh (the fast-writes plan, P-2). The index is written by SQL and the
//! files get the modification times a case names, so nothing waits.

use std::{path::PathBuf, time::Duration};

use rusqlite::params;

use super::*;

const SECOND: i64 = 1_000_000_000;
const ITEM_ONE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FB0";
const ITEM_TWO: &str = "01ARZ3NDEKTSV4RRFFQ69G5FB1";
const COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE0";

/// A registered repository whose index holds one document in a context of
/// its own for each item a test adds, with the file on disk.
struct IndexedRepository {
    _root: tempfile::TempDir,
    _data: tempfile::TempDir,
    root: PathBuf,
    service: RepositoryService,
    db: Connection,
    repo: ResolvedRepository,
}

/// An item's context row and its file.
struct ItemContext {
    id: &'static str,
    row: i64,
    file: PathBuf,
}

impl IndexedRepository {
    /// The repository was last refreshed in whole at `refreshed_at`, in
    /// seconds.
    fn new(refreshed_at: i64) -> Self {
        let root = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        let root_path = root.path().canonicalize().unwrap();
        git2::Repository::init(&root_path).unwrap();
        let service = RepositoryService::open_at(data.path()).unwrap();
        let db = Connection::open(data.path().join(crate::repository::REGISTRY_FILE)).unwrap();
        db.execute(
            "INSERT INTO repositories
                (root_path, enabled_at, accessibility, refresh_required, refreshed_at)
             VALUES (?1, 7, 'accessible', 0, ?2)",
            params![root_path.to_str().unwrap(), refreshed_at],
        )
        .unwrap();
        let repo = service.resolve_repository(&root_path).unwrap();
        Self {
            _root: root,
            _data: data,
            root: root_path,
            service,
            db,
            repo,
        }
    }

    /// The context of the worktree made to edit the document `id`, indexed
    /// at `indexed_at` (nanoseconds; none for a row an older build wrote),
    /// and the document in it, last modified at `modified` seconds.
    fn item_context(
        &self,
        id: &'static str,
        indexed_at: Option<i64>,
        dirty: bool,
        modified: u64,
    ) -> ItemContext {
        let worktree = self.root.join(".manyhands/worktrees").join(id);
        let path = format!("docs/{}.md", id.to_lowercase());
        let file = worktree.join(&path);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(
            &file,
            format!(
                "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: {id}\ntitle: T\n---\n# T\n"
            ),
        )
        .unwrap();
        self.db
            .execute(
                "INSERT INTO contexts
                    (repository_id, kind, branch, worktree_path, item_id, indexed_at, dirty)
                 VALUES (?1, 'active', ?2, ?3, ?4, ?5, ?6)",
                params![
                    self.repo.registration_id(),
                    format!("manyhands/document/{id}"),
                    worktree.to_str().unwrap(),
                    id,
                    indexed_at,
                    dirty
                ],
            )
            .unwrap();
        let row = self.db.last_insert_rowid();
        self.db
            .execute(
                "INSERT INTO discovered_items
                    (context_id, item_id, kind, canonical_path, title, activity_at, activity_source)
                 VALUES (?1, ?2, 'document', ?3, 'T', 5, 'git')",
                params![row, id, path],
            )
            .unwrap();
        let item = ItemContext { id, row, file };
        modify(&item, modified);
        item
    }

    /// A comment on the item that the index stored as one whose ID another
    /// comment of the context has too, a finding it takes the whole context
    /// to make, last modified at `modified` seconds.
    fn duplicate_comment(&self, item: &ItemContext, modified: u64) {
        let worktree = item.file.parent().and_then(Path::parent).unwrap();
        let path = format!(".manyhands/comments/{}/{COMMENT}.md", item.id);
        let file = worktree.join(&path);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(
            &file,
            format!(
                "---\nmanyhands_managed: true\nmanyhands_kind: comment\nid: {COMMENT}\n\
                 item_id: {}\ncreated_at: 2026-01-01T00:00:00Z\n---\nBody.\n",
                item.id
            ),
        )
        .unwrap();
        set_modified(&file, modified);
        self.db
            .execute(
                "INSERT INTO problems (repository_id, context_id, path, code, guidance, observed_at)
                 VALUES (?1, ?2, ?3, 'duplicate-id', '', 5)",
                params![self.repo.registration_id(), item.row, path],
            )
            .unwrap();
    }

    /// The index state each way of reading the item's file gives: the item,
    /// the file by its path in the item's context, and the item's comments.
    fn states(&self, item: &ItemContext) -> [IndexState; 3] {
        let id: ItemId = item.id.parse().unwrap();
        let worktree = item.file.parent().and_then(Path::parent).unwrap();
        let path = item.file.strip_prefix(worktree).unwrap();
        [
            self.service.show_item(&self.repo, &id).unwrap().index.state,
            self.service
                .show_path(&self.repo, Some(worktree), path)
                .unwrap()
                .index
                .state,
            self.service
                .list_comments(&self.repo, &id)
                .unwrap()
                .index
                .state,
        ]
    }
}

/// Sets the item file's modification time, in seconds since the epoch.
fn modify(item: &ItemContext, seconds: u64) {
    set_modified(&item.file, seconds);
}

fn set_modified(file: &Path, seconds: u64) {
    fs::File::options()
        .write(true)
        .open(file)
        .unwrap()
        .set_modified(UNIX_EPOCH + Duration::from_secs(seconds))
        .unwrap();
}

#[test]
fn a_write_to_one_item_leaves_other_items_reads_fresh() {
    // The last refresh of the whole repository was at second 1000. Two items
    // have been written in their own worktrees since, and each write
    // indexed its item's context alone, so the repository's own time did
    // not move.
    let repository = IndexedRepository::new(1_000);
    let one = repository.item_context(ITEM_ONE, Some(2_000 * SECOND), false, 1_900);
    let two = repository.item_context(ITEM_TWO, Some(3_000 * SECOND), false, 2_900);

    assert_eq!(repository.states(&one), [IndexState::Current; 3]);
    assert_eq!(repository.states(&two), [IndexState::Current; 3]);

    // One of them is written and indexed again: the other's reads do not
    // change.
    modify(&one, 3_900);
    repository
        .db
        .execute(
            "UPDATE contexts SET indexed_at = ?2 WHERE id = ?1",
            params![one.row, 4_000 * SECOND],
        )
        .unwrap();

    assert_eq!(repository.states(&one), [IndexState::Current; 3]);
    assert_eq!(repository.states(&two), [IndexState::Current; 3]);
}

#[test]
fn an_outside_edit_after_indexing_reads_stale_from_its_context() {
    // (case, when the repository was last refreshed, when the item's context
    // was indexed, whether it is dirty, when the file was last modified,
    // what a read says). Seconds, but for the nanoseconds of an indexing.
    let cases = [
        (
            "edited a second after its context was indexed, the repository refreshed since",
            9_000,
            Some(1_000 * SECOND + 500_000_000),
            false,
            1_001,
            IndexState::Stale,
        ),
        (
            "edited before its context was indexed, the repository refreshed before that",
            500,
            Some(1_000 * SECOND),
            false,
            900,
            IndexState::Current,
        ),
        (
            "edited in the second its context was indexed, which a whole second hides",
            500,
            Some(1_000 * SECOND + 900_000_000),
            false,
            1_000,
            IndexState::Current,
        ),
        (
            "a context an older build indexed: the repository's time stands, edited after it",
            1_000,
            None,
            false,
            1_001,
            IndexState::Stale,
        ),
        (
            "a context an older build indexed: the repository's time stands, edited before it",
            1_000,
            None,
            false,
            1_000,
            IndexState::Current,
        ),
        (
            "a dirty context is behind whatever its file holds",
            9_000,
            Some(5_000 * SECOND),
            true,
            100,
            IndexState::Stale,
        ),
    ];
    for (case, refreshed_at, indexed_at, dirty, modified, state) in cases {
        let repository = IndexedRepository::new(refreshed_at);
        let item = repository.item_context(ITEM_ONE, indexed_at, dirty, modified);

        assert_eq!(repository.states(&item), [state; 3], "{case}");
    }
}

#[test]
fn a_stored_finding_of_the_whole_context_stands_only_for_a_file_its_context_vouches_for() {
    // (case, when the repository was last refreshed, when the item's context
    // was indexed, whether it is dirty, when the comment file was last
    // modified, whether the finding stands).
    let cases = [
        (
            "written after the repository's refresh, and no later than its context's indexing",
            1_000,
            Some(2_000 * SECOND),
            false,
            1_900,
            true,
        ),
        (
            "edited after its context was indexed",
            9_000,
            Some(1_000 * SECOND),
            false,
            1_001,
            false,
        ),
        (
            "in a dirty context, whatever the file's time",
            9_000,
            Some(5_000 * SECOND),
            true,
            100,
            false,
        ),
        (
            "a context an older build indexed: the repository's time stands",
            1_000,
            None,
            false,
            900,
            true,
        ),
    ];
    for (case, refreshed_at, indexed_at, dirty, modified, stands) in cases {
        let repository = IndexedRepository::new(refreshed_at);
        let item = repository.item_context(ITEM_ONE, indexed_at, dirty, 100);
        repository.duplicate_comment(&item, modified);

        let listed = repository
            .service
            .list_comments(&repository.repo, &ITEM_ONE.parse().unwrap())
            .unwrap();

        // Kept as the nonconforming entry the index stored; otherwise it is
        // checked like any file, and is a comment.
        let ids: Vec<_> = listed
            .items
            .iter()
            .map(|entry| entry.id.as_deref())
            .collect();
        assert_eq!(ids, [(!stands).then_some(COMMENT)], "{case}");
    }
}
