//! A draft's save over real repositories and a real store: the save
//! flushes first; a completion retires only the revision it saved; newer
//! text, a write in flight and an edit made while the revision retires
//! keep their text and take the moved base; a second actor leaves the base
//! unverified; a stale save, a discarded draft, a second save, a replay
//! and a pending checkpoint change nothing they must not.
//!
//! Draft types derive no `Debug`; they are read through `match`. Tests
//! assert on text and base, never on revision numbers (ruling R16).

use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};

use gpui_kit::{Entity, TestAppContext};
use manyhands::{
    drafts::{
        Draft, DraftBase, DraftErrorKind, DraftFields, DraftId, DraftStore, DraftTarget, Entry,
        Held, ItemKind, TargetState, classify,
    },
    repository::{
        CancellationToken, DocumentDraft, DocumentSaveInput, FailurePoint, ItemDto, Mutation,
        MutationCall, NoProgress, RelationshipWrite, RepositoryService, RequestId, TicketDraft,
        TicketSaveInput, keys::SessionCredentials,
    },
    results::{CheckpointEffect, Outcome, ResultCode},
};

use crate::desktop::{
    model::drafts::RecoveryState,
    services::{
        JobKind, ReadKind, Services,
        drafts::DraftRegistry,
        requests::{DesktopRequestId, RequestState},
    },
    test_support::{
        delaying::DelayingBackend,
        fixtures::{Fixture, NoPassphrase, item_id},
        git_home,
    },
    view::harness::{key, wait_until},
};

const DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG0";
const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG1";
const DOCUMENT_PATH: &str = "docs/a.md";

fn start(fixture: &Fixture, cx: &mut TestAppContext) -> (Arc<DelayingBackend>, Services) {
    cx.executor().allow_parking();
    let backend = fixture.delaying_backend();
    let services = cx.update(|cx| Services::start(backend.clone(), cx));
    cx.run_until_parked();
    (backend, services)
}

fn show(service: &RepositoryService, root: &Path, id: &str) -> ItemDto {
    let repo = service.resolve_repository(root).unwrap();
    service.show_item(&repo, &item_id(id)).unwrap()
}

fn document_fields(title: &str, body: &str) -> DraftFields {
    DraftFields::Document {
        title: title.to_owned(),
        body: body.to_owned(),
    }
}

/// The item's fields with its title as `title`.
fn retitled(fields: &DraftFields, title: &str) -> DraftFields {
    let mut fields = fields.clone();
    match &mut fields {
        DraftFields::Document { title: old, .. } | DraftFields::Ticket { title: old, .. } => {
            *old = title.to_owned()
        }
        DraftFields::Comment { .. } => unreachable!(),
    }
    fields
}

/// Begins a draft of the item `id` of `root` as `show_item` reads it now,
/// through the registry.
async fn begin(
    fixture: &Fixture,
    services: &Services,
    root: &Path,
    kind: ItemKind,
    id: &str,
    cx: &mut TestAppContext,
) -> DraftId {
    let shown = show(fixture.service(), root, id);
    let target = DraftTarget {
        repository: key(root),
        item_kind: kind,
        item_id: item_id(id),
        comment: None,
    };
    let base = DraftBase::of(&shown);
    let current = DraftFields::of(&shown).unwrap();
    let begun = services
        .drafts()
        .update(cx, |registry, cx| registry.begin(target, base, current, cx));
    cx.run_until_parked();
    match begun.await {
        Ok(id) => id,
        Err(kind) => panic!("the draft must begin: {kind:?}"),
    }
}

/// A document in a new repository, and a draft of it.
async fn document_draft(
    fixture: &Fixture,
    services: &Services,
    cx: &mut TestAppContext,
) -> (PathBuf, DraftId) {
    let root = fixture.create_repository("a");
    fixture.create_document(&root, DOCUMENT, DOCUMENT_PATH, "Item title");
    let id = begin(fixture, services, &root, ItemKind::Document, DOCUMENT, cx).await;
    (root, id)
}

fn edit(drafts: &Entity<DraftRegistry>, id: DraftId, fields: DraftFields, cx: &mut TestAppContext) {
    drafts.update(cx, |registry, cx| registry.edit(id, fields, cx));
}

async fn save(
    drafts: &Entity<DraftRegistry>,
    id: DraftId,
    cx: &mut TestAppContext,
) -> Option<DesktopRequestId> {
    let saved = drafts.update(cx, |registry, cx| registry.save(id, cx));
    cx.run_until_parked();
    saved.await
}

/// Waits until the request has finished `attempt` times' worth, then lets
/// the completion's reads and writes run.
async fn finished(
    services: &Services,
    request: DesktopRequestId,
    attempt: u32,
    cx: &mut TestAppContext,
) {
    wait_until(cx, services.operations(), move |operations, _| {
        operations.request(request).is_some_and(|request| {
            request.attempt == attempt && matches!(request.state, RequestState::Finished(..))
        })
    })
    .await;
    cx.run_until_parked();
}

fn outcome(
    services: &Services,
    request: DesktopRequestId,
    cx: &TestAppContext,
) -> (Outcome, ResultCode, CheckpointEffect) {
    services.operations().read_with(cx, |operations, _| {
        match &operations.request(request).unwrap().state {
            RequestState::Finished(outcome, code, effects, _, _) => {
                (*outcome, *code, effects.checkpoint)
            }
            _ => panic!("the save must have finished"),
        }
    })
}

/// The stored draft `id`, if the store holds it.
fn stored(store: &DraftStore, id: DraftId) -> Option<(Draft, Held)> {
    store
        .list()
        .unwrap()
        .into_iter()
        .find(|listing| listing.id == id)
        .map(|listing| match listing.entry {
            Entry::Draft(draft) => (draft, listing.held),
            Entry::Unreadable(_) => panic!("the draft must be readable"),
        })
}

fn title_of(fields: &DraftFields) -> &str {
    match fields {
        DraftFields::Document { title, .. } | DraftFields::Ticket { title, .. } => title,
        DraftFields::Comment { .. } => unreachable!(),
    }
}

fn draft_files(fixture: &Fixture, id: DraftId) -> [PathBuf; 2] {
    let folder = fixture.data_dir().join("drafts");
    [
        folder.join(format!("{id}.json")),
        folder.join(format!("{id}.lock")),
    ]
}

fn session_live(drafts: &Entity<DraftRegistry>, id: DraftId, cx: &TestAppContext) -> bool {
    drafts.read_with(cx, |registry, _| registry.recovery_state(id).is_some())
}

/// Commits reachable from `branch` of `root`.
fn commits(root: &Path, branch: &str) -> usize {
    let repository = git2::Repository::open(root).unwrap();
    let tip = repository
        .revparse_single(&format!("refs/heads/{branch}"))
        .unwrap()
        .peel_to_commit()
        .unwrap();
    let mut walk = repository.revwalk().unwrap();
    walk.push(tip.id()).unwrap();
    walk.count()
}

fn branch_of(item: &ItemDto) -> String {
    item.context.branch.clone().expect("the item's branch")
}

/// The draft holds `title` over a base whose observation is the item's as
/// `show_item` gives it now.
fn holds_newer_text_on_the_moved_base(
    fixture: &Fixture,
    root: &Path,
    item: &str,
    id: DraftId,
    title: &str,
) {
    let shown = show(fixture.service(), root, item);
    let Some((draft, held)) = stored(&fixture.draft_store(), id) else {
        panic!("the draft must be kept");
    };
    assert_eq!(held, Held::Here);
    assert_eq!(title_of(&draft.current), title);
    let base = draft.base.expect("the draft keeps a base");
    assert!(base.observation.is_some());
    assert_eq!(base.observation, shown.observation);
    assert!(base.fields == DraftFields::of(&shown).unwrap());
}

/// Ruling R21: an edit saved at once, with no clock advanced, is written
/// before the save is sent; on completion the draft and its lock are gone
/// and the session ended.
#[gpui_kit::test]
async fn an_edit_saved_at_once_is_written_first_and_retires(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let (backend, services) = start(&fixture, cx);
    let drafts = services.drafts().clone();
    let (root, id) = document_draft(&fixture, &services, cx).await;
    let branch = branch_of(&show(fixture.service(), &root, DOCUMENT));
    let before = commits(&root, &branch);

    edit(
        &drafts,
        id,
        document_fields("Saved title", "Saved body"),
        cx,
    );
    let request = save(&drafts, id, cx).await.expect("the save is sent");
    assert_eq!(backend.reads_seen(ReadKind::DraftWrite), 1);
    finished(&services, request, 1, cx).await;

    assert_eq!(
        outcome(&services, request, cx).2,
        CheckpointEffect::Committed
    );
    let shown = show(fixture.service(), &root, DOCUMENT);
    assert_eq!(shown.title.as_deref(), Some("Saved title"));
    assert_eq!(commits(&root, &branch_of(&shown)), before + 1);
    for file in draft_files(&fixture, id) {
        assert!(!file.exists(), "{} is gone", file.display());
    }
    assert!(!session_live(&drafts, id, cx));
}

/// Review focus 1: newer text typed while the save runs is never retired.
/// The draft keeps it over the moved base, and a second save from that
/// base commits.
#[gpui_kit::test]
async fn newer_text_typed_during_a_document_save_is_kept_on_the_moved_base(
    cx: &mut TestAppContext,
) {
    let fixture = Fixture::new();
    let (backend, services) = start(&fixture, cx);
    let drafts = services.drafts().clone();
    let (root, id) = document_draft(&fixture, &services, cx).await;

    edit(&drafts, id, document_fields("Saved title", "Body"), cx);
    let held = backend.hold_job(JobKind::DocumentSave);
    let request = save(&drafts, id, cx).await.expect("the save is sent");
    held.wait_reached();
    edit(&drafts, id, document_fields("Newer title", "Body"), cx);
    held.release();
    finished(&services, request, 1, cx).await;

    assert_eq!(
        outcome(&services, request, cx).2,
        CheckpointEffect::Committed
    );
    assert!(session_live(&drafts, id, cx));
    holds_newer_text_on_the_moved_base(&fixture, &root, DOCUMENT, id, "Newer title");

    let again = save(&drafts, id, cx)
        .await
        .expect("the second save is sent");
    finished(&services, again, 1, cx).await;
    assert_eq!(outcome(&services, again, cx).2, CheckpointEffect::Committed);
    let shown = show(fixture.service(), &root, DOCUMENT);
    assert_eq!(shown.title.as_deref(), Some("Newer title"));
    assert!(stored(&fixture.draft_store(), id).is_none());
}

/// Ruling R21: a write of newer text still in flight when the save
/// completes holds the retirement back; once it lands, the draft holds
/// that text over the moved base.
#[gpui_kit::test]
async fn a_write_in_flight_at_completion_is_waited_for_and_kept(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let (backend, services) = start(&fixture, cx);
    let drafts = services.drafts().clone();
    let (root, id) = document_draft(&fixture, &services, cx).await;

    edit(&drafts, id, document_fields("Saved title", "Body"), cx);
    let held = backend.hold_job(JobKind::DocumentSave);
    let request = save(&drafts, id, cx).await.expect("the save is sent");
    held.wait_reached();
    let write = backend.hold_read(ReadKind::DraftWrite);
    edit(&drafts, id, document_fields("Newer title", "Body"), cx);
    let flush = drafts.update(cx, |registry, cx| registry.flush_now(id, cx));
    cx.run_until_parked();
    held.release();
    finished(&services, request, 1, cx).await;

    // The completion waits on the write: the store still holds what was
    // saved, and nothing retired it.
    let Some((draft, _)) = stored(&fixture.draft_store(), id) else {
        panic!("the draft must not retire under a write in flight");
    };
    assert_eq!(title_of(&draft.current), "Saved title");
    assert!(session_live(&drafts, id, cx));

    write.send(()).unwrap();
    cx.run_until_parked();
    assert_eq!(flush.await, Ok(()));
    cx.run_until_parked();
    holds_newer_text_on_the_moved_base(&fixture, &root, DOCUMENT, id, "Newer title");
    assert!(matches!(
        drafts.read_with(cx, |registry, _| registry.recovery_state(id)),
        Some(RecoveryState::Saved(_))
    ));
}

/// A ticket's save sends `deps` unchanged (ruling R13): an entry the
/// library ignored stays in the file through both saves.
#[gpui_kit::test]
async fn newer_text_typed_during_a_ticket_save_is_kept_and_an_ignored_dependency_survives(
    cx: &mut TestAppContext,
) {
    let fixture = Fixture::new();
    let (backend, services) = start(&fixture, cx);
    let drafts = services.drafts().clone();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "Item title", "deps:\n  - not-a-ulid\n");
    fixture.refresh(&root);
    let id = begin(&fixture, &services, &root, ItemKind::Ticket, TICKET, cx).await;
    let fields = DraftFields::of(&show(fixture.service(), &root, TICKET)).unwrap();

    edit(&drafts, id, retitled(&fields, "Saved title"), cx);
    let held = backend.hold_job(JobKind::TicketSave);
    let request = save(&drafts, id, cx).await.expect("the save is sent");
    held.wait_reached();
    edit(&drafts, id, retitled(&fields, "Newer title"), cx);
    held.release();
    finished(&services, request, 1, cx).await;
    assert_eq!(
        outcome(&services, request, cx).2,
        CheckpointEffect::Committed
    );
    holds_newer_text_on_the_moved_base(&fixture, &root, TICKET, id, "Newer title");

    let again = save(&drafts, id, cx)
        .await
        .expect("the second save is sent");
    finished(&services, again, 1, cx).await;
    assert_eq!(outcome(&services, again, cx).2, CheckpointEffect::Committed);
    let shown = show(fixture.service(), &root, TICKET);
    assert_eq!(shown.title.as_deref(), Some("Newer title"));
    let source = shown.source.expect("the ticket's source");
    assert!(source.contains("not-a-ulid"), "{source}");
    assert!(stored(&fixture.draft_store(), id).is_none());
}

const ACTOR_DATA: &str = "MANYHANDS_TEST_SAVE_ACTOR_DATA";
const ACTOR_ROOT: &str = "MANYHANDS_TEST_SAVE_ACTOR_ROOT";
const ACTOR_ITEM: &str = "MANYHANDS_TEST_SAVE_ACTOR_ITEM";

/// The second actor: in a process of its own, saves the item from the
/// observation `show_item` gives it, a document retitled and a ticket with
/// another status. Does nothing unless the parent named the item.
#[test]
fn second_actor_child() {
    let Some(data) = std::env::var_os(ACTOR_DATA) else {
        return;
    };
    git_home::isolated();
    let root = PathBuf::from(std::env::var_os(ACTOR_ROOT).unwrap());
    let id = std::env::var(ACTOR_ITEM).unwrap();
    let service = RepositoryService::open_at(Path::new(&data)).unwrap();
    let shown = show(&service, &root, &id);
    let mutation = match shown.ticket_type {
        None => Mutation::DocumentSave(DocumentSaveInput {
            root: root.clone(),
            id: item_id(&id),
            draft: DocumentDraft {
                title: "Someone else's title".to_owned(),
                body: shown.body.unwrap_or_default(),
            },
            observation: shown.observation,
        }),
        Some(ticket_type) => Mutation::TicketSave(TicketSaveInput {
            root: root.clone(),
            id: item_id(&id),
            draft: TicketDraft {
                title: shown.title.unwrap(),
                ticket_type,
                status: "in-progress".to_owned(),
                project: shown.project,
                team: shown.team,
                body: shown.body.unwrap_or_default(),
            },
            deps: RelationshipWrite::Unchanged,
            parent: RelationshipWrite::Unchanged,
            observation: shown.observation.unwrap(),
        }),
    };
    let saved = service.execute(
        MutationCall {
            request_id: RequestId::new(),
            confirmation: None,
            mutation,
        },
        &mut SessionCredentials::new(NoPassphrase),
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(saved.code, ResultCode::Ok, "{}", saved.message);
}

/// Runs `second_actor_child` in a child process of the test binary over
/// `item` of `root`, and checks it ran exactly that test.
fn second_actor_saves(fixture: &Fixture, root: &Path, item: &str) {
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "desktop::services::drafts::save::tests::second_actor_child",
            "--nocapture",
        ])
        .env(ACTOR_DATA, fixture.data_dir())
        .env(ACTOR_ROOT, root)
        .env(ACTOR_ITEM, item)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "the child failed:\n{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("1 passed"),
        "the child ran no test:\n{stdout}"
    );
}

/// Review focus 4: a second actor saves between the commit and the
/// registry's read: the base is the submitted fields, unverified, the
/// draft is `Unverified` against the item, and no save is sent from it.
#[gpui_kit::test]
async fn a_second_actor_between_commit_and_read_leaves_the_base_unverified(
    cx: &mut TestAppContext,
) {
    let fixture = Fixture::new();
    let (backend, services) = start(&fixture, cx);
    let drafts = services.drafts().clone();
    let root = fixture.create_repository("a");
    fixture.create_ticket(&root, TICKET, "Item title", "open");
    let id = begin(&fixture, &services, &root, ItemKind::Ticket, TICKET, cx).await;
    let fields = DraftFields::of(&show(fixture.service(), &root, TICKET)).unwrap();

    edit(&drafts, id, retitled(&fields, "Saved title"), cx);
    let held = backend.hold_job(JobKind::TicketSave);
    let request = save(&drafts, id, cx).await.expect("the save is sent");
    held.wait_reached();
    edit(&drafts, id, retitled(&fields, "Newer title"), cx);
    let read = backend.hold_read(ReadKind::DraftItem);
    held.release();
    finished(&services, request, 1, cx).await;
    assert_eq!(backend.reads_seen(ReadKind::DraftItem), 1);

    second_actor_saves(&fixture, &root, TICKET);
    read.send(()).unwrap();
    cx.run_until_parked();

    let Some((draft, _)) = stored(&fixture.draft_store(), id) else {
        panic!("the draft must be kept");
    };
    assert_eq!(title_of(&draft.current), "Newer title");
    let base = draft.base.as_ref().expect("the draft keeps a base");
    assert_eq!(base.observation, None);
    assert_eq!(title_of(&base.fields), "Saved title");
    let shown = show(fixture.service(), &root, TICKET);
    assert_eq!(shown.status.as_deref(), Some("in-progress"));
    assert!(matches!(
        classify(&draft, Ok(&shown)),
        TargetState::Unverified { .. }
    ));
    assert!(save(&drafts, id, cx).await.is_none());
}

/// A save from a base someone else has moved answers `external_change`,
/// and the draft file is byte for byte as it was.
#[gpui_kit::test]
async fn a_stale_save_leaves_the_draft_file_as_it_was(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let (_backend, services) = start(&fixture, cx);
    let drafts = services.drafts().clone();
    let (root, id) = document_draft(&fixture, &services, cx).await;
    edit(&drafts, id, document_fields("Mine", "Body"), cx);
    let flush = drafts.update(cx, |registry, cx| registry.flush_now(id, cx));
    cx.run_until_parked();
    assert_eq!(flush.await, Ok(()));
    let [file, _] = draft_files(&fixture, id);
    let before = std::fs::read(&file).unwrap();

    second_actor_saves(&fixture, &root, DOCUMENT);
    let request = save(&drafts, id, cx).await.expect("the save is sent");
    finished(&services, request, 1, cx).await;

    assert_eq!(
        outcome(&services, request, cx).1,
        ResultCode::ExternalChange
    );
    assert_eq!(std::fs::read(&file).unwrap(), before);
    assert!(session_live(&drafts, id, cx));
    // The save is over: another may be sent.
    assert!(save(&drafts, id, cx).await.is_some());
}

/// Review focus 1: a save that completes after its draft was discarded
/// changes nothing and does not panic.
#[gpui_kit::test]
async fn a_completion_after_discard_changes_nothing(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let (backend, services) = start(&fixture, cx);
    let drafts = services.drafts().clone();
    let (root, id) = document_draft(&fixture, &services, cx).await;
    edit(&drafts, id, document_fields("Saved title", "Body"), cx);
    let held = backend.hold_job(JobKind::DocumentSave);
    let request = save(&drafts, id, cx).await.expect("the save is sent");
    held.wait_reached();

    let discarded = drafts.update(cx, |registry, cx| registry.discard(id, cx));
    cx.run_until_parked();
    assert_eq!(discarded.await, Ok(()));
    assert!(stored(&fixture.draft_store(), id).is_none());
    held.release();
    finished(&services, request, 1, cx).await;

    assert!(!session_live(&drafts, id, cx));
    assert!(stored(&fixture.draft_store(), id).is_none());
    assert!(fixture.draft_store().list().unwrap().is_empty());
    let shown = show(fixture.service(), &root, DOCUMENT);
    assert_eq!(shown.title.as_deref(), Some("Saved title"));
}

/// Ruling R15: one save in flight per session.
#[gpui_kit::test]
async fn a_second_save_while_one_is_in_flight_is_not_sent(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let (backend, services) = start(&fixture, cx);
    let drafts = services.drafts().clone();
    let (root, id) = document_draft(&fixture, &services, cx).await;
    edit(&drafts, id, document_fields("Saved title", "Body"), cx);
    let held = backend.hold_job(JobKind::DocumentSave);
    let request = save(&drafts, id, cx).await.expect("the save is sent");
    held.wait_reached();

    edit(&drafts, id, document_fields("Newer title", "Body"), cx);
    assert!(save(&drafts, id, cx).await.is_none());
    held.release();
    finished(&services, request, 1, cx).await;

    // The newer text survives the first save, over its moved base, and the
    // session may save again.
    assert!(session_live(&drafts, id, cx));
    holds_newer_text_on_the_moved_base(&fixture, &root, DOCUMENT, id, "Newer title");
    assert!(save(&drafts, id, cx).await.is_some());
}

/// A Retry of a save that retired its revision is answered from F2's
/// record: the branch has one commit for it, and the registry changes
/// nothing.
#[gpui_kit::test]
async fn a_replayed_save_changes_nothing(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let (_backend, services) = start(&fixture, cx);
    let drafts = services.drafts().clone();
    let (root, id) = document_draft(&fixture, &services, cx).await;
    let branch = branch_of(&show(fixture.service(), &root, DOCUMENT));
    let before = commits(&root, &branch);
    edit(&drafts, id, document_fields("Saved title", "Body"), cx);
    let request = save(&drafts, id, cx).await.expect("the save is sent");
    finished(&services, request, 1, cx).await;
    assert!(!session_live(&drafts, id, cx));
    assert_eq!(commits(&root, &branch), before + 1);

    services.worker().retry(request, None);
    finished(&services, request, 2, cx).await;

    let (answered, _, _) = outcome(&services, request, cx);
    assert!(matches!(answered, Outcome::Success | Outcome::Noop));
    assert_eq!(commits(&root, &branch), before + 1);
    assert!(drafts.read_with(cx, |registry, _| registry.is_idle()));
    assert!(fixture.draft_store().list().unwrap().is_empty());
}

/// Ruling R22: a save whose checkpoint is pending keeps its draft and
/// blocks a second save; Retry under the same request commits once and
/// retires the revision. The recipe of
/// `tests/mutation_replay/documents.rs`.
#[gpui_kit::test]
async fn a_pending_save_keeps_its_draft_until_its_retry_commits(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let (backend, services) = start(&fixture, cx);
    let drafts = services.drafts().clone();
    let (root, id) = document_draft(&fixture, &services, cx).await;
    let branch = branch_of(&show(fixture.service(), &root, DOCUMENT));
    let before = commits(&root, &branch);
    edit(&drafts, id, document_fields("Saved title", "Body"), cx);

    backend.replace_service(
        RepositoryService::open_at_with_failure_point_for_testing(
            fixture.data_dir(),
            FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence,
        )
        .unwrap(),
    );
    let request = save(&drafts, id, cx).await.expect("the save is sent");
    finished(&services, request, 1, cx).await;
    let (answered, _, checkpoint) = outcome(&services, request, cx);
    assert_eq!(
        (answered, checkpoint),
        (Outcome::Partial, CheckpointEffect::Pending)
    );
    let Some((draft, _)) = stored(&fixture.draft_store(), id) else {
        panic!("a pending save keeps its draft");
    };
    assert_eq!(title_of(&draft.current), "Saved title");
    assert!(session_live(&drafts, id, cx));
    assert!(save(&drafts, id, cx).await.is_none());

    backend.replace_service(RepositoryService::open_at(fixture.data_dir()).unwrap());
    services.worker().retry(request, None);
    finished(&services, request, 2, cx).await;

    assert_eq!(
        outcome(&services, request, cx).2,
        CheckpointEffect::Committed
    );
    let shown = show(fixture.service(), &root, DOCUMENT);
    assert_eq!(shown.title.as_deref(), Some("Saved title"));
    assert_eq!(commits(&root, &branch_of(&shown)), before + 1);
    assert!(!session_live(&drafts, id, cx));
    assert!(stored(&fixture.draft_store(), id).is_none());
}

/// Ruling R22: dismissing a pending save's request ends that save: the
/// draft is kept, and another save may be sent.
#[gpui_kit::test]
async fn dismissing_a_pending_save_lets_another_be_sent(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let (backend, services) = start(&fixture, cx);
    let drafts = services.drafts().clone();
    let (_root, id) = document_draft(&fixture, &services, cx).await;
    edit(&drafts, id, document_fields("Saved title", "Body"), cx);
    backend.replace_service(
        RepositoryService::open_at_with_failure_point_for_testing(
            fixture.data_dir(),
            FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence,
        )
        .unwrap(),
    );
    let request = save(&drafts, id, cx).await.expect("the save is sent");
    finished(&services, request, 1, cx).await;
    assert!(save(&drafts, id, cx).await.is_none());

    drafts.update(cx, |registry, cx| registry.dismissed(request, cx));
    assert!(session_live(&drafts, id, cx));
    assert!(stored(&fixture.draft_store(), id).is_some());
    backend.replace_service(RepositoryService::open_at(fixture.data_dir()).unwrap());
    assert!(save(&drafts, id, cx).await.is_some());
}

/// An edit made while the saved revision retires is never lost: once the
/// revision is gone, the newer text is stored again, as a new draft over
/// the moved base, and the session goes on under it.
#[gpui_kit::test]
async fn an_edit_made_while_the_revision_retires_is_stored_again(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let (backend, services) = start(&fixture, cx);
    let drafts = services.drafts().clone();
    let (root, id) = document_draft(&fixture, &services, cx).await;
    edit(&drafts, id, document_fields("Saved title", "Body"), cx);
    let request = save(&drafts, id, cx).await.expect("the save is sent");
    let retire = backend.hold_read(ReadKind::DraftWrite);
    finished(&services, request, 1, cx).await;
    // The retirement waits at its gate with the claim still in its slot.
    assert!(stored(&fixture.draft_store(), id).is_some());

    edit(
        &drafts,
        id,
        document_fields("Typed while retiring", "Body"),
        cx,
    );
    retire.send(()).unwrap();
    cx.run_until_parked();

    assert!(stored(&fixture.draft_store(), id).is_none());
    let sessions = drafts.read_with(cx, |registry, _| registry.session_ids());
    let [again] = sessions.as_slice() else {
        panic!("one session goes on");
    };
    assert_ne!(*again, id);
    holds_newer_text_on_the_moved_base(&fixture, &root, DOCUMENT, *again, "Typed while retiring");
}

/// Ruling R22: abandoning the operation a pending save holds ends that
/// save: the draft is kept, and another save may be sent.
#[gpui_kit::test]
async fn abandoning_a_pending_saves_operation_lets_another_be_sent(cx: &mut TestAppContext) {
    use crate::desktop::services::{
        Backend,
        requests::{Intent, Lane},
    };
    use manyhands::repository::{
        ConfirmationId, OperationAbandonInput, OperationId, PrepareDataDto,
    };

    let fixture = Fixture::new();
    let (backend, services) = start(&fixture, cx);
    let drafts = services.drafts().clone();
    let (root, id) = document_draft(&fixture, &services, cx).await;
    edit(&drafts, id, document_fields("Saved title", "Body"), cx);
    backend.replace_service(
        RepositoryService::open_at_with_failure_point_for_testing(
            fixture.data_dir(),
            FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence,
        )
        .unwrap(),
    );
    let request = save(&drafts, id, cx).await.expect("the save is sent");
    finished(&services, request, 1, cx).await;
    let operation = drafts.read_with(cx, |registry, _| {
        registry.saves[&request]
            .operation
            .clone()
            .expect("a pending save names its operation")
    });
    backend.replace_service(RepositoryService::open_at(fixture.data_dir()).unwrap());

    let operation_id = OperationId::parse(&operation).unwrap();
    let prepared = backend
        .service()
        .prepare(&Mutation::OperationAbandon(OperationAbandonInput {
            root: root.clone(),
            operation_id,
        }));
    let Some(PrepareDataDto::Preview(preview)) = prepared.data else {
        panic!("operation abandon gave no preview: {}", prepared.message);
    };
    let confirmation = ConfirmationId::parse(&preview.confirmation_id).unwrap();
    let abandon = services.worker().submit(
        Intent::OperationAbandon { root, operation_id },
        Lane::User,
        Some(confirmation),
        None,
    );
    finished(&services, abandon, 1, cx).await;
    assert_eq!(outcome(&services, abandon, cx).1, ResultCode::Ok);

    assert!(session_live(&drafts, id, cx));
    assert!(stored(&fixture.draft_store(), id).is_some());
    assert!(drafts.read_with(cx, |registry, _| registry.saves.is_empty()));
    assert!(save(&drafts, id, cx).await.is_some());
}

/// Starts a save whose retirement waits at the `DraftWrite` gate, with an
/// edit made after it was decided. Gives the request, the held
/// retirement, and the `Drafts` gate holding the store again when `hold`.
async fn retiring_under_an_edit(
    backend: &DelayingBackend,
    services: &Services,
    id: DraftId,
    hold_store_again: bool,
    cx: &mut TestAppContext,
) -> (
    futures::channel::oneshot::Sender<()>,
    Option<futures::channel::oneshot::Sender<()>>,
) {
    let drafts = services.drafts().clone();
    edit(&drafts, id, document_fields("Saved title", "Body"), cx);
    let request = save(&drafts, id, cx).await.expect("the save is sent");
    let retire = backend.hold_read(ReadKind::DraftWrite);
    let store_again = hold_store_again.then(|| backend.hold_read(ReadKind::Drafts));
    finished(services, request, 1, cx).await;
    edit(
        &drafts,
        id,
        document_fields("Typed while retiring", "Body"),
        cx,
    );
    (retire, store_again)
}

/// Review fix 1: a store again that fails keeps the text with its own
/// error over the moved base, and the next flush stores it again; it is
/// never left only in memory.
#[cfg(unix)]
#[gpui_kit::test]
async fn a_store_again_that_failed_is_stored_by_the_next_flush(cx: &mut TestAppContext) {
    use std::os::unix::fs::PermissionsExt;

    let fixture = Fixture::new();
    let (backend, services) = start(&fixture, cx);
    let drafts = services.drafts().clone();
    let (root, id) = document_draft(&fixture, &services, cx).await;
    let (retire, store_again) = retiring_under_an_edit(&backend, &services, id, true, cx).await;
    retire.send(()).unwrap();
    cx.run_until_parked();

    let folder = fixture.data_dir().join("drafts");
    std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o500)).unwrap();
    store_again.unwrap().send(()).unwrap();
    cx.run_until_parked();
    assert_eq!(
        drafts.read_with(cx, |registry, _| registry.recovery_state(id)),
        Some(RecoveryState::Failed(DraftErrorKind::Access))
    );
    // While the store keeps failing, a flush says why: never `NotFound`.
    let all = drafts.update(cx, |registry, cx| registry.flush_all(cx));
    cx.run_until_parked();
    assert_eq!(all.await, vec![(id, DraftErrorKind::Access)]);
    std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o700)).unwrap();

    // Retry stores it again, as a new draft over the moved base.
    let retry = drafts.update(cx, |registry, cx| registry.flush_now(id, cx));
    cx.run_until_parked();
    assert_eq!(retry.await, Ok(()));
    let new = drafts
        .read_with(cx, |registry, _| registry.current_id(id))
        .expect("the session goes on");
    assert_ne!(new, id);
    holds_newer_text_on_the_moved_base(&fixture, &root, DOCUMENT, new, "Typed while retiring");
    let all = drafts.update(cx, |registry, cx| registry.flush_all(cx));
    cx.run_until_parked();
    assert_eq!(all.await, Vec::new());
}

/// Review fix 2: a flush of every session started while a revision
/// retires under an edit follows the session to its new draft, and an
/// edit by the old ID reaches it.
#[gpui_kit::test]
async fn the_old_id_reaches_a_session_stored_again(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let (backend, services) = start(&fixture, cx);
    let drafts = services.drafts().clone();
    let (root, id) = document_draft(&fixture, &services, cx).await;
    let (retire, _) = retiring_under_an_edit(&backend, &services, id, false, cx).await;
    let all = drafts.update(cx, |registry, cx| registry.flush_all(cx));
    cx.run_until_parked();
    retire.send(()).unwrap();
    cx.run_until_parked();
    assert_eq!(all.await, Vec::new());

    let new = drafts
        .read_with(cx, |registry, _| registry.current_id(id))
        .expect("the session goes on");
    assert_ne!(new, id);
    holds_newer_text_on_the_moved_base(&fixture, &root, DOCUMENT, new, "Typed while retiring");

    edit(&drafts, id, document_fields("After the move", "Body"), cx);
    let flush = drafts.update(cx, |registry, cx| registry.flush_now(id, cx));
    cx.run_until_parked();
    assert_eq!(flush.await, Ok(()));
    holds_newer_text_on_the_moved_base(&fixture, &root, DOCUMENT, new, "After the move");
    assert!(matches!(
        drafts.read_with(cx, |registry, _| registry.recovery_state(id)),
        Some(RecoveryState::Saved(_))
    ));
}

/// Review minor 1: a discard while the text is being stored again
/// discards the new draft too; nothing comes back.
#[gpui_kit::test]
async fn a_discard_during_a_store_again_discards_the_new_draft(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let (backend, services) = start(&fixture, cx);
    let drafts = services.drafts().clone();
    let (_root, id) = document_draft(&fixture, &services, cx).await;
    let (retire, store_again) = retiring_under_an_edit(&backend, &services, id, true, cx).await;
    retire.send(()).unwrap();
    cx.run_until_parked();

    let discarded = drafts.update(cx, |registry, cx| registry.discard(id, cx));
    cx.run_until_parked();
    store_again.unwrap().send(()).unwrap();
    cx.run_until_parked();
    assert_eq!(discarded.await, Ok(()));
    assert!(fixture.draft_store().list().unwrap().is_empty());
    assert!(drafts.read_with(cx, |registry, _| registry.is_idle()));
}

/// Review minor 2: dismissing a pending save from the operation area, as
/// the user does, ends that save.
#[gpui_kit::test]
async fn dismissing_a_pending_save_in_the_operation_area_lets_another_be_sent(
    cx: &mut TestAppContext,
) {
    use crate::desktop::view::{harness::Shell, operations::request_id};

    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.create_document(&root, DOCUMENT, DOCUMENT_PATH, "Item title");
    let shell = Shell::open(fixture, cx);
    let services = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().clone());
    let drafts = services.drafts().clone();
    let id = begin(
        &shell.fixture,
        &services,
        &root,
        ItemKind::Document,
        DOCUMENT,
        cx,
    )
    .await;
    edit(&drafts, id, document_fields("Saved title", "Body"), cx);
    shell.backend.replace_service(
        RepositoryService::open_at_with_failure_point_for_testing(
            shell.fixture.data_dir(),
            FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence,
        )
        .unwrap(),
    );
    let request = save(&drafts, id, cx).await.expect("the save is sent");
    finished(&services, request, 1, cx).await;
    assert!(save(&drafts, id, cx).await.is_none());

    shell.click(request_id(request, "dismiss"), cx);
    assert!(session_live(&drafts, id, cx));
    assert!(stored(&shell.fixture.draft_store(), id).is_some());
    shell
        .backend
        .replace_service(RepositoryService::open_at(shell.fixture.data_dir()).unwrap());
    assert!(save(&drafts, id, cx).await.is_some());
}
