//! Target states over real repositories made through the library, and the
//! drafts' survival of what the library does to its own data.
//!
//! Every test begins with `git_home::isolated`, so none reads or writes the
//! Git configuration of whoever runs it. The second actor is a child
//! process of this binary, running one test with a role in its environment.
//! No transport is used: there is no remote, and the credential provider
//! answers `Unavailable`.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use manyhands::{
    canonical::ItemId,
    drafts::{
        Change, CommentTarget, DraftBase, DraftClaim, DraftFields, DraftId, DraftStore,
        DraftTarget, Entry, Field, ItemKind, TargetState, classify, classify_comment,
    },
    repository::{
        CancellationToken, CloseTicketRequest, ConfirmationId, DocumentDraft,
        DocumentPromotionPreflight, DocumentSaveInput, ItemDto, Mutation, MutationCall, NoProgress,
        OperationId, PreviewDocumentPromotionRequest, PreviewTicketClosureRequest,
        PromoteDocumentRequest, REGISTRY_FILE, RelationshipWrite, RepoRemoveInput,
        RepositoryService, RequestId, TicketClosurePreflight, TicketDraft, TicketSaveInput,
        keys::{PassphraseResponse, SessionCredentialProvider, SessionCredentials, UnlockRequest},
    },
    results::{CheckpointEffect, Outcome, ResultCode},
};

use crate::repo::{
    self, TestRepository, git_home,
    items::{
        COMMENT_A, COMMENT_B, DOCUMENT_A, TICKET_A, commit, document_source, rebuild,
        refresh_completely, ticket_path, ticket_source, write, write_comment,
    },
};
use crate::support::{child, ran_one_test};

const DOCUMENT_PATH: &str = "docs/guide.md";
const ROLE: &str = "MANYHANDS_DRAFTS_TARGET_ROLE";
const DATA: &str = "MANYHANDS_DRAFTS_TARGET_DATA";
const ROOT: &str = "MANYHANDS_DRAFTS_TARGET_ROOT";
const OBSERVATION: &str = "MANYHANDS_DRAFTS_TARGET_OBSERVATION";

/// A provider with nothing to give: no remote asks it anything.
struct NoCredentials;

impl SessionCredentialProvider for NoCredentials {
    fn request_passphrase(&mut self, _request: &UnlockRequest) -> PassphraseResponse {
        PassphraseResponse::Unavailable
    }
}

fn item_id(id: &str) -> ItemId {
    id.parse().unwrap()
}

/// A registered repository holding the ticket `TICKET_A` and the document
/// `DOCUMENT_A`, committed and refreshed, with its own data directory.
struct World {
    // Fields drop in declaration order: the service before its data.
    service: RepositoryService,
    data: tempfile::TempDir,
    root: PathBuf,
    fixture: TestRepository,
}

impl World {
    fn new() -> Self {
        git_home::isolated();
        let fixture = repo::born_repository();
        let enabled = repo::enabled_repository(&fixture);
        let root = fixture.root.canonicalize().unwrap();
        let ticket = ticket_path(TICKET_A);
        write(&root, &ticket, &ticket_source(TICKET_A, "Ticket", ""));
        write(
            &root,
            DOCUMENT_PATH,
            &document_source(DOCUMENT_A, "Guide", ""),
        );
        commit(&fixture, &[&ticket, DOCUMENT_PATH], 1_700_000_000);
        refresh_completely(&enabled.service, &root);
        let repo::EnabledRepository {
            service,
            data_directory,
        } = enabled;
        Self {
            service,
            data: data_directory,
            root,
            fixture,
        }
    }

    fn store(&self) -> DraftStore {
        DraftStore::open_in(self.data.path()).expect("the store opens")
    }

    fn read(&self, id: &str) -> Result<ItemDto, ResultCode> {
        read_with(&self.service, &self.root, id)
    }

    fn item(&self, id: &str) -> ItemDto {
        match self.read(id) {
            Ok(item) => item,
            Err(code) => panic!("{id} must read: {code:?}"),
        }
    }

    fn target(&self, kind: ItemKind, id: &str) -> DraftTarget {
        DraftTarget {
            repository: self.root.to_str().unwrap().to_owned(),
            item_kind: kind,
            item_id: item_id(id),
            comment: None,
        }
    }

    /// A draft of `item` whose base is the item as read and whose text is
    /// `edit` of the item's fields.
    fn draft_of(
        &self,
        store: &DraftStore,
        kind: ItemKind,
        item: &ItemDto,
        edit: impl FnOnce(&mut DraftFields),
    ) -> DraftClaim {
        let base = DraftBase::of(item).expect("the item conforms");
        let mut current = base.fields.clone();
        edit(&mut current);
        store
            .create(
                self.target(kind, item.id.as_deref().unwrap()),
                Some(base),
                current,
            )
            .expect("the draft is created")
    }

    fn classify(&self, claim: &DraftClaim, id: &str) -> TargetState {
        classify(claim.draft(), self.read(id).as_ref().map_err(|code| *code))
    }

    fn execute(&self, confirmation: Option<ConfirmationId>, mutation: Mutation) -> Outcome {
        execute(&self.service, confirmation, mutation)
    }

    /// Saves `fields` over the ticket read at `observation`, as F2's
    /// `ticket save`.
    fn save_ticket(&self, fields: &DraftFields, observation: &str) {
        let outcome = self.execute(None, ticket_save(&self.root, fields, observation));
        assert_eq!(outcome, Outcome::Success);
    }

    /// `repo remove` through F2: a preview, then the confirmed request.
    fn remove_registration(&self) {
        let mutation = || {
            Mutation::RepoRemove(RepoRemoveInput {
                root: self.root.clone(),
            })
        };
        let prepared = self.service.prepare(&mutation());
        assert_eq!(prepared.code, ResultCode::Ok, "{}", prepared.message);
        let confirmation = prepared
            .data
            .and_then(|data| data.into_preview())
            .expect("a preview")
            .confirmation_id;
        let confirmation = ConfirmationId::parse(&confirmation).unwrap();
        assert_eq!(
            self.execute(Some(confirmation), mutation()),
            Outcome::Success
        );
    }

    /// Removes `path` from the primary worktree in a commit, then refreshes.
    fn remove_in_a_commit(&self, path: &str) {
        fs::remove_file(self.root.join(path)).unwrap();
        let repository = &self.fixture.repository;
        let mut index = repository.index().unwrap();
        index.read(true).unwrap();
        index.remove_path(Path::new(path)).unwrap();
        index.write().unwrap();
        let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
        let parent = repository.head().unwrap().peel_to_commit().unwrap();
        let signature = git2::Signature::new(
            "Manyhands Test",
            "manyhands-test@example.invalid",
            &git2::Time::new(1_700_000_100, 0),
        )
        .unwrap();
        repository
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                "remove",
                &tree,
                &[&parent],
            )
            .unwrap();
        refresh_completely(&self.service, &self.root);
    }
}

fn read_with(service: &RepositoryService, root: &Path, id: &str) -> Result<ItemDto, ResultCode> {
    let resolved = service.resolve_repository(root).map_err(|e| e.code())?;
    service
        .show_item(&resolved, &item_id(id))
        .map_err(|e| e.code())
}

fn execute(
    service: &RepositoryService,
    confirmation: Option<ConfirmationId>,
    mutation: Mutation,
) -> Outcome {
    let envelope = service.execute(
        MutationCall {
            request_id: RequestId::new(),
            confirmation,
            mutation,
        },
        &mut SessionCredentials::new(NoCredentials),
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(envelope.code, ResultCode::Ok, "{}", envelope.message);
    assert!(matches!(
        envelope.effects.checkpoint,
        CheckpointEffect::Committed | CheckpointEffect::Unchanged | CheckpointEffect::NotRequested
    ));
    envelope.outcome
}

fn ticket_save(root: &Path, fields: &DraftFields, observation: &str) -> Mutation {
    let DraftFields::Ticket {
        title,
        ticket_type,
        status,
        project,
        team,
        body,
        ..
    } = fields.clone()
    else {
        panic!("a ticket's fields")
    };
    Mutation::TicketSave(TicketSaveInput {
        root: root.to_owned(),
        id: item_id(TICKET_A),
        draft: TicketDraft {
            title,
            ticket_type,
            status,
            project,
            team,
            body,
        },
        // These tests never change a relationship.
        deps: RelationshipWrite::Unchanged,
        parent: RelationshipWrite::Unchanged,
        observation: observation.to_owned(),
    })
}

fn set_title(fields: &mut DraftFields, to: &str) {
    match fields {
        DraftFields::Document { title, .. } | DraftFields::Ticket { title, .. } => {
            *title = to.to_owned()
        }
        DraftFields::Comment { .. } => panic!("a comment has no title"),
    }
}

fn set_body(fields: &mut DraftFields, to: &str) {
    match fields {
        DraftFields::Document { body, .. }
        | DraftFields::Ticket { body, .. }
        | DraftFields::Comment { body } => *body = to.to_owned(),
    }
}

fn title_of(fields: &DraftFields) -> &str {
    match fields {
        DraftFields::Document { title, .. } | DraftFields::Ticket { title, .. } => title,
        DraftFields::Comment { .. } => panic!("a comment has no title"),
    }
}

fn change(comparison: &manyhands::drafts::Comparison, field: Field) -> Change {
    comparison
        .fields
        .iter()
        .find(|(f, _)| *f == field)
        .map(|(_, c)| *c)
        .unwrap_or_else(|| panic!("{field:?} is compared"))
}

/// Gives the ticket an item worktree, by a save that changes its body, and
/// reads it from there.
fn ticket_in_its_worktree(world: &World) -> ItemDto {
    let item = world.item(TICKET_A);
    let mut fields = DraftFields::of(&item).unwrap();
    set_body(&mut fields, "Worked on.\n");
    world.save_ticket(&fields, item.observation.as_deref().unwrap());
    let item = world.item(TICKET_A);
    assert!(
        item.context.branch.as_deref() == Some(&*format!("manyhands/ticket/{TICKET_A}")),
        "the ticket reads from its worktree"
    );
    item
}

#[test]
fn a_draft_is_current_straight_after_its_base_is_read() {
    let world = World::new();
    let store = world.store();
    let item = world.item(TICKET_A);
    let claim = world.draft_of(&store, ItemKind::Ticket, &item, |f| set_title(f, "Mine"));
    assert!(matches!(
        world.classify(&claim, TICKET_A),
        TargetState::Current
    ));
    let document = world.item(DOCUMENT_A);
    let claim = world.draft_of(&store, ItemKind::Document, &document, |f| {
        set_body(f, "Mine\n")
    });
    assert!(matches!(
        world.classify(&claim, DOCUMENT_A),
        TargetState::Current
    ));
}

#[test]
fn a_save_by_another_process_makes_the_draft_changed() {
    if std::env::var(ROLE).as_deref() == Ok("save-title") {
        // The second actor: another service over the same data directory.
        git_home::isolated();
        let data = PathBuf::from(std::env::var(DATA).unwrap());
        let root = PathBuf::from(std::env::var(ROOT).unwrap());
        let observation = std::env::var(OBSERVATION).unwrap();
        let service = RepositoryService::open_at(&data).unwrap();
        let item = read_with(&service, &root, TICKET_A).unwrap();
        let mut fields = DraftFields::of(&item).unwrap();
        set_title(&mut fields, "Their title");
        assert_eq!(
            execute(&service, None, ticket_save(&root, &fields, &observation)),
            Outcome::Success
        );
        return;
    }
    let world = World::new();
    let store = world.store();
    let item = ticket_in_its_worktree(&world);
    let claim = world.draft_of(&store, ItemKind::Ticket, &item, |f| {
        set_body(f, "My body.\n")
    });
    assert!(matches!(
        world.classify(&claim, TICKET_A),
        TargetState::Current
    ));

    let output = child(
        "targets::a_save_by_another_process_makes_the_draft_changed",
        &[
            (ROLE, "save-title"),
            (DATA, world.data.path().to_str().unwrap()),
            (ROOT, world.root.to_str().unwrap()),
            (OBSERVATION, item.observation.as_deref().unwrap()),
        ],
    )
    .output()
    .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "the second actor failed:\n{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    ran_one_test(&stdout);

    match world.classify(&claim, TICKET_A) {
        TargetState::Changed {
            theirs,
            comparison,
            context_changed,
        } => {
            assert!(!context_changed, "the save stayed in the same worktree");
            assert_eq!(title_of(&theirs), "Their title");
            assert_eq!(change(&comparison, Field::Title), Change::Theirs);
            assert_eq!(change(&comparison, Field::Body), Change::Mine);
            assert_eq!(change(&comparison, Field::Status), Change::None);
        }
        _ => panic!("another process's save makes the draft Changed"),
    }
}

#[test]
fn a_save_of_exactly_the_drafts_fields_is_already_saved() {
    let world = World::new();
    let store = world.store();
    let item = world.item(TICKET_A);
    let claim = world.draft_of(&store, ItemKind::Ticket, &item, |f| {
        set_title(f, "Saved title");
        set_body(f, "Saved body.\n");
    });
    world.save_ticket(&claim.draft().current, item.observation.as_deref().unwrap());
    assert!(matches!(
        world.classify(&claim, TICKET_A),
        TargetState::AlreadySaved
    ));
}

#[test]
fn a_local_closure_makes_the_ticket_closed_and_removes_its_worktree() {
    let world = World::new();
    let store = world.store();
    let item = ticket_in_its_worktree(&world);
    let worktree = PathBuf::from(&item.context.worktree);
    assert!(worktree.is_dir());
    let claim = world.draft_of(&store, ItemKind::Ticket, &item, |f| set_title(f, "Mine"));

    let preview = match world
        .service
        .preview_ticket_closure(PreviewTicketClosureRequest {
            root: world.root.clone(),
            item_id: item_id(TICKET_A),
            confirmed_identity: None,
        })
        .unwrap()
    {
        TicketClosurePreflight::Ready(preview) => preview,
        other => panic!("the closure is not ready: {other:?}"),
    };
    assert_eq!(preview.publication_remote(), None);
    world
        .service
        .close_ticket(
            CloseTicketRequest {
                root: world.root.clone(),
                operation_id: OperationId::new(),
                item_id: item_id(TICKET_A),
                expected: preview.expectation(),
                approval: None,
                confirmed_identity: None,
            },
            &mut SessionCredentials::new(NoCredentials),
        )
        .unwrap();
    assert!(!worktree.exists(), "the closure removed the item worktree");
    assert!(!manyhands::runtime::git_transport_initialized());

    assert!(matches!(
        world.classify(&claim, TICKET_A),
        TargetState::TicketClosed
    ));
}

#[test]
fn a_removed_item_worktree_is_a_changed_context() {
    let world = World::new();
    let store = world.store();
    // The document gets an item worktree by a save through F2.
    let document = world.item(DOCUMENT_A);
    let outcome = world.execute(
        None,
        Mutation::DocumentSave(DocumentSaveInput {
            root: world.root.clone(),
            id: item_id(DOCUMENT_A),
            draft: DocumentDraft {
                title: "Guide".to_owned(),
                body: "# Guide, worked on\n".to_owned(),
            },
            observation: document.observation.clone(),
        }),
    );
    assert_eq!(outcome, Outcome::Success);
    let item = world.item(DOCUMENT_A);
    let worktree = PathBuf::from(&item.context.worktree);
    assert_eq!(
        item.context.branch.as_deref(),
        Some(&*format!("manyhands/document/{DOCUMENT_A}"))
    );
    let claim = world.draft_of(&store, ItemKind::Document, &item, |f| {
        set_body(f, "My body.\n")
    });

    // Promotion integrates the worktree into the primary and removes it.
    let preview = match world
        .service
        .preview_document_promotion(PreviewDocumentPromotionRequest {
            root: world.root.clone(),
            item_id: item_id(DOCUMENT_A),
        })
        .unwrap()
    {
        DocumentPromotionPreflight::Ready(preview) => preview,
        other => panic!("the promotion is not ready: {other:?}"),
    };
    world
        .service
        .promote_document(
            PromoteDocumentRequest {
                root: world.root.clone(),
                operation_id: OperationId::new(),
                item_id: item_id(DOCUMENT_A),
                expected: preview.expectation(),
                approval: None,
                confirmed_identity: None,
            },
            &mut SessionCredentials::new(NoCredentials),
        )
        .unwrap();
    assert!(
        !worktree.exists(),
        "the promotion removed the item worktree"
    );
    assert_eq!(
        world.item(DOCUMENT_A).context.branch.as_deref(),
        Some("main")
    );

    match world.classify(&claim, DOCUMENT_A) {
        TargetState::Changed {
            comparison,
            context_changed,
            ..
        } => {
            assert!(context_changed, "the item reads from the primary now");
            assert_eq!(change(&comparison, Field::Title), Change::None);
            assert_eq!(change(&comparison, Field::Body), Change::Mine);
        }
        _ => panic!("a removed worktree is Changed"),
    }
}

#[test]
fn an_item_removed_in_a_commit_is_not_found() {
    let world = World::new();
    let store = world.store();
    let item = world.item(TICKET_A);
    let claim = world.draft_of(&store, ItemKind::Ticket, &item, |f| set_title(f, "Mine"));
    world.remove_in_a_commit(&ticket_path(TICKET_A));
    assert_eq!(world.read(TICKET_A).err(), Some(ResultCode::ItemNotFound));
    assert!(matches!(
        world.classify(&claim, TICKET_A),
        TargetState::ItemNotFound
    ));
}

#[test]
fn a_removed_registration_makes_the_repository_unavailable() {
    let world = World::new();
    let store = world.store();
    let item = world.item(TICKET_A);
    let claim = world.draft_of(&store, ItemKind::Ticket, &item, |f| set_title(f, "Mine"));
    world.remove_registration();
    match world.classify(&claim, TICKET_A) {
        TargetState::RepositoryUnavailable(code) => {
            assert_eq!(code, ResultCode::RepositoryNotRegistered)
        }
        _ => panic!("a removed registration is RepositoryUnavailable"),
    }
}

#[test]
fn a_repository_folder_renamed_away_is_unavailable() {
    let world = World::new();
    let store = world.store();
    let item = world.item(TICKET_A);
    let claim = world.draft_of(&store, ItemKind::Ticket, &item, |f| set_title(f, "Mine"));
    // The registration as the desktop holds it, resolved while the folder
    // was there.
    let resolved = world.service.resolve_repository(&world.root).unwrap();
    let away = tempfile::tempdir().unwrap();
    fs::rename(&world.root, away.path().join("moved")).unwrap();

    let read = world
        .service
        .show_item(&resolved, &item_id(TICKET_A))
        .map_err(|e| e.code());
    match classify(claim.draft(), read.as_ref().map_err(|c| *c)) {
        TargetState::RepositoryUnavailable(code) => {
            assert_eq!(code, ResultCode::RepositoryInaccessible)
        }
        _ => panic!("a repository renamed away is RepositoryUnavailable"),
    }

    // Resolving the root afresh answers `invalid_path`, which the approved
    // code mapping makes a failed read, not an unavailable repository.
    match world.classify(&claim, TICKET_A) {
        TargetState::ReadFailed(code) => assert_eq!(code, ResultCode::InvalidPath),
        _ => panic!("a fresh resolution of a missing root is ReadFailed by the mapping"),
    }
}

/// A draft of a reply on `TICKET_A` to `parent`.
fn reply_draft(world: &World, store: &DraftStore, parent: &str) -> DraftClaim {
    let mut target = world.target(ItemKind::Ticket, TICKET_A);
    target.comment = Some(CommentTarget {
        comment_id: item_id("01ARZ3NDEKTSV4RRFFQ69G5FE9"),
        parent_id: Some(item_id(parent)),
    });
    store
        .create(
            target,
            None,
            DraftFields::Comment {
                body: "A reply.\n".to_owned(),
            },
        )
        .expect("the draft is created")
}

fn classify_reply(world: &World, claim: &DraftClaim) -> TargetState {
    let item = world.read(TICKET_A);
    let resolved = world.service.resolve_repository(&world.root).unwrap();
    let comments = world
        .service
        .list_comments(&resolved, &item_id(TICKET_A))
        .map_err(|e| e.code());
    classify_comment(
        claim.draft(),
        item.as_ref().map_err(|c| *c),
        comments.as_ref().map_err(|c| *c),
    )
}

#[test]
fn a_reply_whose_parent_is_not_in_the_thread_has_no_parent() {
    let world = World::new();
    let store = world.store();
    write_comment(
        &world.root,
        TICKET_A,
        COMMENT_A,
        None,
        "2026-09-30T12:00:00Z",
        "",
    );
    refresh_completely(&world.service, &world.root);

    let to_present = reply_draft(&world, &store, COMMENT_A);
    assert!(matches!(
        classify_reply(&world, &to_present),
        TargetState::Current
    ));
    let to_absent = reply_draft(&world, &store, COMMENT_B);
    assert!(matches!(
        classify_reply(&world, &to_absent),
        TargetState::ParentNotFound
    ));
}

#[cfg(unix)]
#[test]
fn an_incomplete_comment_list_is_a_failed_read() {
    let world = World::new();
    let store = world.store();
    let outside = tempfile::tempdir().unwrap();
    let comments = world.root.join(format!(".manyhands/comments/{TICKET_A}"));
    fs::create_dir_all(comments.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(outside.path(), &comments).unwrap();
    refresh_completely(&world.service, &world.root);

    let claim = reply_draft(&world, &store, COMMENT_B);
    assert!(matches!(
        classify_reply(&world, &claim),
        TargetState::ReadFailed(_)
    ));
}

/// Every draft file's bytes, by ID, and the IDs `list` returns readable.
fn stored(world_data: &Path, store: &DraftStore) -> (BTreeMap<DraftId, Vec<u8>>, Vec<DraftId>) {
    let listed = store
        .list()
        .expect("the store lists")
        .into_iter()
        .map(|listing| match listing.entry {
            Entry::Draft(draft) => {
                assert!(draft.id == listing.id);
                listing.id
            }
            Entry::Unreadable(reason) => panic!("{} is unreadable: {reason:?}", listing.id),
        })
        .collect::<Vec<_>>();
    let bytes = listed
        .iter()
        .map(|id| {
            let file = fs::read(world_data.join("drafts").join(format!("{id}.json"))).unwrap();
            assert_eq!(store.raw_bytes(*id).unwrap(), file);
            (*id, file)
        })
        .collect();
    (bytes, listed)
}

#[test]
fn drafts_survive_registration_removal_rebuild_and_a_lost_index() {
    let mut world = World::new();
    let store = world.store();
    let ticket = world.item(TICKET_A);
    let document = world.item(DOCUMENT_A);
    world.draft_of(&store, ItemKind::Ticket, &ticket, |f| set_title(f, "Mine"));
    world.draft_of(&store, ItemKind::Document, &document, |f| {
        set_body(f, "Mine\n")
    });
    reply_draft(&world, &store, COMMENT_A);
    let before = stored(world.data.path(), &store);
    assert_eq!(before.1.len(), 3);
    let check = |world: &World, step: &str| {
        let after = stored(
            world.data.path(),
            &DraftStore::open_in(world.data.path()).unwrap(),
        );
        assert!(after == before, "{step} changed the drafts");
        assert!(stored(world.data.path(), &store) == before, "{step}");
    };

    rebuild(&world.service, &world.root);
    check(&world, "rebuild");

    world.remove_registration();
    check(&world, "registration removal");

    // The index lost, and the service opened again over the data. The old
    // service is closed first, as a new process would find it.
    let data = world.data.path().to_owned();
    let elsewhere = tempfile::tempdir().unwrap();
    drop(std::mem::replace(
        &mut world.service,
        RepositoryService::open_at(elsewhere.path()).unwrap(),
    ));
    for suffix in ["", "-wal", "-shm"] {
        let path = data.join(format!("{REGISTRY_FILE}{suffix}"));
        if path.exists() {
            fs::remove_file(path).unwrap();
        }
    }
    world.service = RepositoryService::open_at(&data).unwrap();
    check(&world, "a lost index");
}
