//! Shared support for the tests of repair and adoption: a repository
//! that holds, committed on primary and indexed, one file for each kind
//! of thing a repair meets.

use std::{
    fs,
    path::{Path, PathBuf},
};

use manyhands::{
    repository::{
        DocumentRepairInput, ExpectedPathObservation, FailurePoint, Mutation, MutationDataDto,
        PreviewDto, RepairMutationDto, RepositoryService, TicketRepairInput,
    },
    results::Envelope,
};

use super::{
    confirmation, items,
    mutation::{self, World},
};

/// What the nonconforming files and the corrected sources hold. A
/// repair reads both whole, and neither may reach anything it gives or
/// keeps.
pub const FILE_SENTINEL: &str = "SENTINEL-nonconforming-3b7e";
pub const SOURCE_SENTINEL: &str = "SENTINEL-corrected-a41c";
pub const SENTINELS: [&str; 2] = [FILE_SENTINEL, SOURCE_SENTINEL];

/// A document with no ID: only the marker.
pub const MARKER_PATH: &str = "docs/notes/marker.md";
/// The ID a repair gives it: the caller's, from `id new`.
pub const ADOPTED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FM0";
/// A document with a valid ID and no title.
pub const UNTITLED_PATH: &str = "docs/untitled.md";
pub const UNTITLED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FM1";
/// Two conforming documents that hold one ID.
pub const DUPLICATE_PATHS: [&str; 2] = ["docs/twin-a.md", "docs/twin-b.md"];
pub const DUPLICATE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FM2";
/// A file under `docs/` that says it is a ticket.
pub const MISPLACED_PATH: &str = "docs/misplaced.md";
pub const MISPLACED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FM3";
/// A managed file outside every canonical path.
pub const OUTSIDE_PATH: &str = "notes/managed.md";
/// A ticket with no ID, in a directory that is a ULID.
pub const TICKET_MARKER: &str = "01ARZ3NDEKTSV4RRFFQ69G5FM4";
/// A ticket with no ID, in a directory that is none.
pub const TICKET_UNNAMED_PATH: &str = ".manyhands/tickets/not-a-ulid/ticket.md";
/// A lifecycle-closed ticket with a broken field: it has no type.
pub const TICKET_CLOSED_BROKEN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FM5";
/// An open ticket with the same broken field.
pub const TICKET_OPEN_BROKEN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FM6";
/// A ticket whose front matter cannot be read at all.
pub const TICKET_UNREADABLE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FM7";
/// An ID nothing holds.
pub const UNHELD: &str = "01ARZ3NDEKTSV4RRFFQ69G5FM9";

pub fn marker_only() -> String {
    format!("---\nmanyhands_managed: true\n---\nNotes {FILE_SENTINEL}.\n")
}

/// A ticket with every field but its type.
pub fn untyped_ticket(id: &str, status: &str, extra: &str) -> String {
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {id}\ntitle: Broken\n\
         status: {status}\n{extra}---\nBody {FILE_SENTINEL}.\n"
    )
}

/// The corrected source of a document.
pub fn document(id: &str) -> String {
    format!(
        "{}{SOURCE_SENTINEL}\n",
        items::document_source(id, "Repaired", "")
    )
}

/// The corrected source of a ticket. `extra` is front matter lines.
pub fn ticket(id: &str, extra: &str) -> String {
    format!(
        "{}{SOURCE_SENTINEL}\n",
        items::ticket_source(id, "Repaired", extra)
    )
}

/// `World`, with a file committed on primary and indexed for each kind
/// of thing a repair meets. Every envelope it gives is scanned for the
/// sentinels, and its stores are when it is dropped.
pub struct Shelf {
    pub world: World,
}

impl std::ops::Deref for Shelf {
    type Target = World;

    fn deref(&self) -> &World {
        &self.world
    }
}

impl std::ops::DerefMut for Shelf {
    fn deref_mut(&mut self) -> &mut World {
        &mut self.world
    }
}

fn scanned<T: serde::Serialize>(envelope: Envelope<T>) -> Envelope<T> {
    let text = serde_json::to_string(&envelope).unwrap();
    for sentinel in SENTINELS {
        assert!(!text.contains(sentinel), "the envelope holds {sentinel}");
    }
    envelope
}

impl Shelf {
    pub fn new() -> Self {
        Self::with_service(|data| RepositoryService::open_at(data).unwrap())
    }

    /// A shelf whose service fails once at `point`. The fixture is made
    /// by another service, so that the failure is the test's to meet.
    pub fn failing_once_at(point: FailurePoint) -> Self {
        let mut shelf = Self::new();
        shelf.world.service = super::FailOnce::at(point).open_service(shelf.world.data.path());
        shelf
    }

    pub fn with_service(open: impl FnOnce(&Path) -> RepositoryService) -> Self {
        let world = World::with_service(open);
        let files = [
            (MARKER_PATH.to_owned(), marker_only()),
            (
                UNTITLED_PATH.to_owned(),
                format!(
                    "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: {UNTITLED}\n\
                     ---\nText {FILE_SENTINEL}.\n"
                ),
            ),
            (
                DUPLICATE_PATHS[0].to_owned(),
                items::document_source(DUPLICATE, "Twin", ""),
            ),
            (
                DUPLICATE_PATHS[1].to_owned(),
                items::document_source(DUPLICATE, "Twin", ""),
            ),
            (
                MISPLACED_PATH.to_owned(),
                items::ticket_source(MISPLACED, "Misplaced", ""),
            ),
            (OUTSIDE_PATH.to_owned(), marker_only()),
            (items::ticket_path(TICKET_MARKER), marker_only()),
            (TICKET_UNNAMED_PATH.to_owned(), marker_only()),
            (
                items::ticket_path(TICKET_CLOSED_BROKEN),
                untyped_ticket(TICKET_CLOSED_BROKEN, "done", items::CLOSURE),
            ),
            (
                items::ticket_path(TICKET_OPEN_BROKEN),
                untyped_ticket(TICKET_OPEN_BROKEN, "open", ""),
            ),
            (
                items::ticket_path(TICKET_UNREADABLE),
                format!("---\n: : :\n\t- [\n---\nBody {FILE_SENTINEL}.\n"),
            ),
        ];
        for (path, source) in &files {
            items::write(&world.root, path, source);
        }
        let paths: Vec<&str> = files.iter().map(|(path, _)| path.as_str()).collect();
        items::commit(&world.fixture, &paths, items::COMMITTED_AT + 1);
        items::refresh_completely(&world.service, &world.root);
        Self { world }
    }

    /// A file of the primary worktree, as it is on disk.
    pub fn primary(&self, path: &str) -> Vec<u8> {
        fs::read(self.root.join(path)).unwrap()
    }

    /// The file at `path` in the editing context of `id`, if it is there.
    pub fn context(&self, id: &str, path: &str) -> Option<Vec<u8>> {
        fs::read(self.worktree(id).join(path)).ok()
    }

    /// What `repair_item` is to expect of the file at `path` on primary.
    pub fn expected(&self, path: &str) -> ExpectedPathObservation {
        ExpectedPathObservation::from_bytes(&self.primary(path))
    }

    /// The token `show_path` gives for the file at `path` on primary.
    pub fn path_token(&self, path: &str) -> String {
        self.service
            .show_path(&self.resolved(), None, Path::new(path))
            .unwrap()
            .observation
            .unwrap()
    }

    /// A repair of the document at `path` with `source`, presenting the
    /// token the file has now.
    pub fn document_repair(&self, path: &str, source: &str) -> Mutation {
        self.document_repair_with(path, &self.path_token(path), source)
    }

    pub fn document_repair_with(&self, path: &str, observation: &str, source: &str) -> Mutation {
        Mutation::DocumentRepair(DocumentRepairInput {
            root: self.root.clone(),
            path: PathBuf::from(path),
            observation: observation.to_owned(),
            source: source.to_owned(),
        })
    }

    /// A repair of the ticket in the directory `directory` with `source`,
    /// presenting the token the file has now.
    pub fn ticket_repair(&self, path: &str, source: &str) -> Mutation {
        Mutation::TicketRepair(TicketRepairInput {
            root: self.root.clone(),
            path: PathBuf::from(path),
            observation: self.path_token(path),
            source: source.to_owned(),
        })
    }

    /// `prepare`, with its envelope checked against the published
    /// contract and scanned.
    pub fn prepare(&self, mutation: &Mutation) -> Envelope<PreviewDto> {
        scanned(confirmation::prepare_with(&self.service, mutation))
    }

    /// A preview that is given, and the confirmation it gives.
    pub fn preview(&self, mutation: &Mutation) -> (String, PreviewDto) {
        let envelope = self.prepare(mutation);
        let preview = envelope
            .data
            .unwrap_or_else(|| panic!("no preview: {:?}", envelope.code));
        (preview.confirmation_id.clone(), preview)
    }

    /// One request that presents `confirmation`.
    pub fn confirmed(
        &self,
        request_id: &str,
        confirmation: &str,
        mutation: Mutation,
    ) -> Envelope<MutationDataDto> {
        scanned(mutation::execute_confirmed(
            &self.service,
            request_id,
            confirmation,
            mutation,
        ))
    }

    /// A preview of `mutation` and one request that presents its
    /// confirmation.
    pub fn repair(
        &self,
        request_id: &str,
        mutation: impl Fn() -> Mutation,
    ) -> Envelope<MutationDataDto> {
        let (confirmation, _) = self.preview(&mutation());
        self.confirmed(request_id, &confirmation, mutation())
    }

    /// The editing contexts there are: the directories under the one
    /// Manyhands keeps its worktrees in.
    pub fn contexts(&self) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(self.root.join(".manyhands/worktrees"))
            .map(|entries| {
                entries
                    .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        names
    }

    /// The local branches that are an item's.
    pub fn item_branches(&self) -> Vec<String> {
        let repository = git2::Repository::open(&self.root).unwrap();
        let mut names: Vec<String> = repository
            .branches(Some(git2::BranchType::Local))
            .unwrap()
            .map(|branch| branch.unwrap().0.name().unwrap().unwrap().to_owned())
            .filter(|name| name.starts_with("manyhands/"))
            .collect();
        names.sort();
        names
    }

    pub fn count(&self, query: &str) -> i64 {
        items::index(self.data.path())
            .query_row(query, [], |row| row.get(0))
            .unwrap()
    }

    pub fn journal_rows(&self) -> i64 {
        self.count("SELECT COUNT(*) FROM operation_records")
    }

    pub fn confirmation_rows(&self) -> i64 {
        self.count("SELECT COUNT(*) FROM confirmation_records")
    }

    /// The message of the commit at the tip of the item's branch.
    pub fn tip_subject(&self, id: &str) -> String {
        let repository = git2::Repository::open(&self.root).unwrap();
        repository
            .find_commit(self.branch_tip(id).unwrap())
            .unwrap()
            .message()
            .unwrap()
            .to_owned()
    }
}

impl Drop for Shelf {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            // The request tables and every journal.
            mutation::assert_request_stores_exclude(self.data.path(), &SENTINELS);
        }
    }
}

/// What a repair's envelope reports of its item.
pub fn repair_data(envelope: &Envelope<MutationDataDto>) -> &RepairMutationDto {
    match envelope.data.as_ref() {
        Some(MutationDataDto::Repair(data)) => data,
        Some(
            MutationDataDto::Document(_)
            | MutationDataDto::Ticket(_)
            | MutationDataDto::Remote(_)
            | MutationDataDto::Key(_)
            | MutationDataDto::Operation(_)
            | MutationDataDto::Synchronization(_)
            | MutationDataDto::Identity(_)
            | MutationDataDto::Host(_),
        )
        | None => panic!("the envelope carries no repair data"),
    }
}
