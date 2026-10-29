//! New-item drafts over real repositories and a real store (design, Create
//! (decision 3); R-20, R-20b, R-21, R-42, R-46, R-73; decision 18): the
//! draft is stored before its context is prepared, and nothing is
//! provisioned without a store; a create's completion clears the marker,
//! whatever the read after it says; an occupied path that holds the
//! draft's own item becomes a review; a create retried after a lost answer
//! creates one item; and, in a tab, a not-created draft is routed by its
//! classification, restored and continued with no item read, created by
//! Save, and released on discard only when this process made its context.
//!
//! Draft types derive no `Debug`; they are read through `match` and
//! `assert!`. Tests that wait on the worker allow parking. Each test sets
//! the caret, focuses the editor and draws a frame before typing, and types
//! through the window. The words are literals: the tests fix them.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use git2::Repository;
use gpui_kit::{Entity, Focusable, TestAppContext};
use manyhands::{
    drafts::{
        BaseContext, ContextKind, Draft, DraftBase, DraftError, DraftErrorKind, DraftFields,
        DraftId, DraftStore, DraftTarget, Entry, Held, ItemKind, TargetState, classify,
        is_new_item_base,
    },
    repository::{
        AuthoringKind, CancellationToken, ContextPrepareInput, ContextPrepareIntent, DocumentDraft,
        DocumentSaveInput, FailurePoint, ItemDto, ItemDtoKind, Mutation, MutationCall, NoProgress,
        RepositoryService, RequestId, keys::SessionCredentials,
    },
    results::{CheckpointEffect, Outcome, ResultCode},
};

use super::{NewItem, RECOVERY_NEEDED};
use crate::desktop::{
    model::{
        edit::Phase,
        editor_text::normalized,
        navigation::OpenTarget,
        review::{Choice, Review},
    },
    services::{
        JobKind, ReadKind, Services,
        created_contexts::ContextKey,
        drafts::DraftRegistry,
        requests::{DesktopRequestId, Intent, RequestState},
    },
    test_support::{
        delaying::DelayingBackend,
        drafts::held,
        fixtures::{Fixture, NoPassphrase, item_id},
    },
    view::{
        drafts::row_id,
        editor::host::EditorHost,
        harness::{Shell, WAIT, key, new_ticket_draft, new_ticket_fields, wait_until},
    },
};

const DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FH0";
const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FH1";
const DOCUMENT_PATH: &str = "docs/new.md";
const NOT_CREATED: &str = "The item is not created yet";

// ---- the registry ----

fn start(fixture: &Fixture, cx: &mut TestAppContext) -> (Arc<DelayingBackend>, Services) {
    cx.executor().allow_parking();
    let backend = fixture.delaying_backend();
    let services = cx.update(|cx| Services::start(backend.clone(), cx));
    cx.run_until_parked();
    (backend, services)
}

fn document_fields(title: &str, body: &str) -> DraftFields {
    DraftFields::Document {
        title: title.to_owned(),
        body: body.to_owned(),
    }
}

/// The new document `DOCUMENT` of `root` at `DOCUMENT_PATH`, titled
/// `title`: its target, its base (no observation and no source: R-20) and
/// its fields.
fn new_document(root: &Path, title: &str) -> (DraftTarget, DraftBase, DraftFields) {
    let fields = document_fields(title, "Body\n");
    let target = DraftTarget {
        repository: key(root),
        item_kind: ItemKind::Document,
        item_id: item_id(DOCUMENT),
        comment: None,
    };
    let base = DraftBase {
        observation: None,
        path: DOCUMENT_PATH.to_owned(),
        context: BaseContext {
            kind: ContextKind::Primary,
            branch: Some("main".to_owned()),
            worktree: key(root),
            head_oid: None,
        },
        source: None,
        fields: fields.clone(),
    };
    (target, base, fields)
}

/// R-20's entry point, through to its prepare's answer.
async fn new_item(
    services: &Services,
    (target, base, fields): (DraftTarget, DraftBase, DraftFields),
    cx: &mut TestAppContext,
) -> NewItem {
    let made = services.drafts().update(cx, |registry, cx| {
        registry.new_item(target, base, fields, cx)
    });
    cx.run_until_parked();
    let made = match made.await {
        Ok(made) => made,
        Err(kind) => panic!("the new item's draft must be written: {kind:?}"),
    };
    finished(services, made.prepare, 1, cx).await;
    assert_eq!(code(services, made.prepare, cx), ResultCode::Ok, "prepared");
    made
}

fn edit(drafts: &Entity<DraftRegistry>, id: DraftId, fields: DraftFields, cx: &mut TestAppContext) {
    drafts.update(cx, |registry, cx| registry.edit(id, fields, cx));
}

async fn create(
    drafts: &Entity<DraftRegistry>,
    id: DraftId,
    cx: &mut TestAppContext,
) -> Option<DesktopRequestId> {
    let sent = drafts.update(cx, |registry, cx| registry.create(id, cx));
    cx.run_until_parked();
    sent.await
}

/// Waits until the request has finished `attempt` times' worth, then lets
/// what follows its answer run.
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

fn answer(
    services: &Services,
    request: DesktopRequestId,
    cx: &TestAppContext,
) -> (Outcome, ResultCode, CheckpointEffect) {
    services.operations().read_with(cx, |operations, _| {
        match &operations.request(request).unwrap().state {
            RequestState::Finished(outcome, code, effects, _, _) => {
                (*outcome, *code, effects.checkpoint)
            }
            _ => panic!("the request must have finished"),
        }
    })
}

fn code(services: &Services, request: DesktopRequestId, cx: &TestAppContext) -> ResultCode {
    answer(services, request, cx).1
}

fn show(service: &RepositoryService, root: &Path, id: &str) -> ItemDto {
    let repo = service.resolve_repository(root).unwrap();
    service.show_item(&repo, &item_id(id)).unwrap()
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

/// Whether this process counts the item's context as made here.
fn made_here(
    services: &Services,
    root: &Path,
    kind: ItemDtoKind,
    id: &str,
    cx: &TestAppContext,
) -> bool {
    services.created_contexts().read_with(cx, |created, _| {
        created.contains(&ContextKey::new(&key(root), kind, id))
    })
}

/// Commits reachable from `branch` of `root`.
fn commits(root: &Path, branch: &str) -> usize {
    let repository = Repository::open(root).unwrap();
    let tip = repository
        .revparse_single(&format!("refs/heads/{branch}"))
        .unwrap()
        .peel_to_commit()
        .unwrap();
    let mut walk = repository.revwalk().unwrap();
    walk.push(tip.id()).unwrap();
    walk.count()
}

/// Another actor saves the document `DOCUMENT` of `root` retitled, from
/// the observation `show_item` gives it now.
fn retitled_by_another(fixture: &Fixture, root: &Path, title: &str) {
    let service = fixture.service();
    let shown = show(service, root, DOCUMENT);
    let saved = service.execute(
        MutationCall {
            request_id: RequestId::new(),
            confirmation: None,
            mutation: Mutation::DocumentSave(DocumentSaveInput {
                root: root.to_owned(),
                id: item_id(DOCUMENT),
                draft: DocumentDraft {
                    title: title.to_owned(),
                    body: shown.body.clone().unwrap_or_default(),
                },
                observation: shown.observation.clone(),
            }),
        },
        &mut SessionCredentials::new(NoPassphrase),
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(saved.code, ResultCode::Ok, "{}", saved.message);
}

/// R-20: the new item's draft is in the store before `context prepare`
/// is sent, and the prepare is the Create intent's, of this item.
#[gpui_kit::test]
async fn the_new_item_draft_is_stored_before_context_prepare(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let (backend, services) = start(&fixture, cx);
    let root = fixture.create_repository("a");
    let (target, base, fields) = new_document(&root, "New title");

    // The draft's create waits behind `Drafts`; the prepare's job is held
    // where it reaches the library.
    let store = backend.hold_read(ReadKind::Drafts);
    let prepare = backend.hold_job(JobKind::ContextPrepare);
    let made = services.drafts().update(cx, |registry, cx| {
        registry.new_item(target, base, fields, cx)
    });
    cx.run_until_parked();
    assert!(fixture.draft_store().list().unwrap().is_empty());
    let sent = services
        .operations()
        .read_with(cx, |operations, _| operations.requests().len());
    assert_eq!(sent, 0, "nothing is sent before the draft is stored");

    store.send(()).unwrap();
    cx.run_until_parked();
    let made = match made.await {
        Ok(made) => made,
        Err(kind) => panic!("the new item's draft must be written: {kind:?}"),
    };
    prepare.wait_reached();
    // As the prepare reaches the library, the store holds the draft.
    let Some((draft, held)) = stored(&fixture.draft_store(), made.draft) else {
        panic!("the draft is stored before the prepare runs");
    };
    assert_eq!(held, Held::Here);
    assert!(draft.base.as_ref().is_some_and(is_new_item_base));
    let intent = services.operations().read_with(cx, |operations, _| {
        operations
            .request(made.prepare)
            .map(|request| request.intent.clone())
    });
    assert!(matches!(
        intent,
        Some(Intent::ContextPrepare {
            kind: AuthoringKind::Document,
            intent: ContextPrepareIntent::Create,
            ref id,
            ..
        }) if id.to_string() == DOCUMENT
    ));

    prepare.release();
    finished(&services, made.prepare, 1, cx).await;
    assert_eq!(code(&services, made.prepare, cx), ResultCode::Ok);
    assert!(made_here(
        &services,
        &root,
        ItemDtoKind::Document,
        DOCUMENT,
        cx
    ));
}

/// R-46: with no draft store, `begin_new` fails, nothing is sent and no
/// context is made; the words say why.
#[gpui_kit::test]
async fn create_stops_before_provisioning_when_the_store_is_unavailable(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let service = Arc::new(RepositoryService::open_at(fixture.data_dir()).unwrap());
    let backend = Arc::new(DelayingBackend::new(
        service,
        fixture.home().to_owned(),
        Err(DraftError {
            kind: DraftErrorKind::Access,
        }),
    ));
    let services = cx.update(|cx| Services::start(backend.clone(), cx));
    cx.run_until_parked();

    let (target, base, fields) = new_document(&root, "New title");
    let made = services.drafts().update(cx, |registry, cx| {
        registry.new_item(target, base, fields, cx)
    });
    cx.run_until_parked();
    assert!(matches!(made.await, Err(DraftErrorKind::Access)));
    cx.run_until_parked();

    let sent = services
        .operations()
        .read_with(cx, |operations, _| operations.requests().len());
    assert_eq!(sent, 0, "nothing is sent");
    assert!(backend.jobs_seen().is_empty(), "nothing reached the worker");
    assert!(
        !root.join(".manyhands/worktrees").join(DOCUMENT).exists(),
        "no context is made"
    );
    assert_eq!(
        RECOVERY_NEEDED,
        "Crash recovery is off, so a new item cannot be created."
    );
}

/// R-20b: the create commits and the read after it gives the fields sent:
/// the base becomes that read, the marker clears, and the context this
/// process made holds work now.
#[gpui_kit::test]
async fn create_completion_clears_the_marker_when_the_read_matches(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let (backend, services) = start(&fixture, cx);
    let drafts = services.drafts().clone();
    let root = fixture.create_repository("a");
    let made = new_item(&services, new_document(&root, "New title"), cx).await;
    assert!(made_here(
        &services,
        &root,
        ItemDtoKind::Document,
        DOCUMENT,
        cx
    ));

    // Typed while the create runs: the session goes on, over the base the
    // create leaves.
    let held = backend.hold_job(JobKind::DocumentCreate);
    let request = create(&drafts, made.draft, cx)
        .await
        .expect("the create is sent");
    held.wait_reached();
    edit(
        &drafts,
        made.draft,
        document_fields("Newer title", "Body\n"),
        cx,
    );
    held.release();
    finished(&services, request, 1, cx).await;

    let (outcome, code, checkpoint) = answer(&services, request, cx);
    assert_eq!(
        (outcome, code, checkpoint),
        (
            Outcome::Success,
            ResultCode::Ok,
            CheckpointEffect::Committed
        )
    );
    let shown = show(fixture.service(), &root, DOCUMENT);
    assert_eq!(shown.title.as_deref(), Some("New title"));
    assert_eq!(shown.path, DOCUMENT_PATH);
    let Some((draft, _)) = stored(&fixture.draft_store(), made.draft) else {
        panic!("the newer text is kept");
    };
    assert_eq!(title_of(&draft.current), "Newer title");
    let base = draft.base.as_ref().expect("a base");
    assert!(!is_new_item_base(base), "the marker clears");
    assert_eq!(base.observation, shown.observation);
    assert!(base.fields == DraftFields::of(&shown).unwrap());
    assert!(
        !made_here(&services, &root, ItemDtoKind::Document, DOCUMENT, cx),
        "the create committed into the context"
    );
}

/// R-20b: the read after the create fails: the base becomes the fields
/// sent, with no observation and the source `""`, so the marker clears
/// all the same.
#[gpui_kit::test]
async fn create_completion_clears_the_marker_when_the_read_fails(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let (backend, services) = start(&fixture, cx);
    let drafts = services.drafts().clone();
    let root = fixture.create_repository("a");
    let made = new_item(&services, new_document(&root, "New title"), cx).await;

    let held = backend.hold_job(JobKind::DocumentCreate);
    let request = create(&drafts, made.draft, cx)
        .await
        .expect("the create is sent");
    held.wait_reached();
    edit(
        &drafts,
        made.draft,
        document_fields("Newer title", "Body\n"),
        cx,
    );
    let read = backend.hold_read(ReadKind::DraftItem);
    held.release();
    finished(&services, request, 1, cx).await;
    assert_eq!(code(&services, request, cx), ResultCode::Ok);

    // The read runs on a service that knows no repository: it fails.
    let elsewhere = tempfile::tempdir().unwrap();
    backend.replace_service(RepositoryService::open_at(elsewhere.path()).unwrap());
    read.send(()).unwrap();
    cx.run_until_parked();
    backend.replace_service(RepositoryService::open_at(fixture.data_dir()).unwrap());

    let Some((draft, _)) = stored(&fixture.draft_store(), made.draft) else {
        panic!("the newer text is kept");
    };
    assert_eq!(title_of(&draft.current), "Newer title");
    let base = draft.base.as_ref().expect("a base");
    assert_eq!(base.observation, None);
    assert_eq!(base.source.as_deref(), Some(""));
    assert!(!is_new_item_base(base), "the marker clears");
    assert_eq!(title_of(&base.fields), "New title", "the fields sent");
    let shown = show(fixture.service(), &root, DOCUMENT);
    assert!(matches!(
        classify(&draft, Ok(&shown)),
        TargetState::Unverified { .. }
    ));
}

/// R-20b: another actor retitles the item between the create and the
/// read: the base is the fields sent, unverified, and the review that the
/// next Save opens defaults to theirs for the title.
#[gpui_kit::test]
async fn a_retitle_between_create_and_read_reviews_with_theirs_as_default(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let (backend, services) = start(&fixture, cx);
    let drafts = services.drafts().clone();
    let root = fixture.create_repository("a");
    let made = new_item(&services, new_document(&root, "New title"), cx).await;

    let held = backend.hold_job(JobKind::DocumentCreate);
    let request = create(&drafts, made.draft, cx)
        .await
        .expect("the create is sent");
    held.wait_reached();
    // Typed in the body while the create runs.
    edit(
        &drafts,
        made.draft,
        document_fields("New title", "Body\nMore\n"),
        cx,
    );
    let read = backend.hold_read(ReadKind::DraftItem);
    held.release();
    finished(&services, request, 1, cx).await;

    retitled_by_another(&fixture, &root, "Someone else's title");
    read.send(()).unwrap();
    cx.run_until_parked();

    let Some((draft, _)) = stored(&fixture.draft_store(), made.draft) else {
        panic!("the newer text is kept");
    };
    let base = draft.base.clone().expect("a base");
    assert_eq!(base.observation, None);
    assert!(!is_new_item_base(&base), "the marker clears");
    assert_eq!(title_of(&base.fields), "New title", "the fields sent");
    // Unverified: Save opens the review instead, and creates nothing.
    assert!(
        drafts
            .update(cx, |registry, cx| registry.save(made.draft, cx))
            .await
            .is_none()
    );
    assert!(create(&drafts, made.draft, cx).await.is_none());
    let shown = show(fixture.service(), &root, DOCUMENT);
    assert!(matches!(
        classify(&draft, Ok(&normalized(&shown))),
        TargetState::Unverified { .. }
    ));
    let theirs = DraftBase::of(&shown).expect("a conforming item");
    let review = Review::new(base.fields.clone(), theirs, draft.current.clone());
    let title = review
        .rows()
        .iter()
        .position(|(field, _)| *field == manyhands::drafts::Field::Title)
        .expect("a title row");
    assert_eq!(
        review.choice(title),
        Some(Choice::Theirs),
        "theirs by default"
    );
    let body = review
        .rows()
        .iter()
        .position(|(field, _)| *field == manyhands::drafts::Field::Body)
        .expect("a body row");
    assert_eq!(review.choice(body), Some(Choice::Mine));
}

/// The worker's `retry`: a create whose answer is lost after it ran (the
/// library's `BeforeRequestSettlement` failure point) is sent again under
/// the same request. The item is created once, and the retried answer
/// completes the draft.
#[gpui_kit::test]
async fn a_create_retried_after_a_lost_answer_creates_one_item(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let (backend, services) = start(&fixture, cx);
    let drafts = services.drafts().clone();
    let root = fixture.create_repository("a");
    let made = new_item(&services, new_document(&root, "New title"), cx).await;

    backend.replace_service(
        RepositoryService::open_at_with_failure_point_for_testing(
            fixture.data_dir(),
            FailurePoint::BeforeRequestSettlement,
        )
        .unwrap(),
    );
    let request = create(&drafts, made.draft, cx)
        .await
        .expect("the create is sent");
    finished(&services, request, 1, cx).await;
    assert_eq!(
        code(&services, request, cx),
        ResultCode::InternalError,
        "lost"
    );
    let shown = show(fixture.service(), &root, DOCUMENT);
    assert_eq!(shown.title.as_deref(), Some("New title"), "it ran");
    let branch = shown.context.branch.clone().expect("the item's branch");
    let before = commits(&root, &branch);
    let Some((draft, _)) = stored(&fixture.draft_store(), made.draft) else {
        panic!("a lost answer keeps the draft");
    };
    assert!(draft.base.as_ref().is_some_and(is_new_item_base));

    backend.replace_service(RepositoryService::open_at(fixture.data_dir()).unwrap());
    services.worker().retry(request, None);
    finished(&services, request, 2, cx).await;

    let (outcome, code, _) = answer(&services, request, cx);
    assert!(
        matches!(outcome, Outcome::Success | Outcome::Noop),
        "answered"
    );
    assert!(matches!(code, ResultCode::Ok | ResultCode::AlreadyApplied));
    assert_eq!(commits(&root, &branch), before, "no second commit");
    let service = fixture.service();
    let repo = service.resolve_repository(&root).unwrap();
    let documents = service.list_documents(&repo).unwrap();
    let ours = documents
        .items
        .iter()
        .filter(|item| item.id.as_deref() == Some(DOCUMENT))
        .count();
    assert_eq!(ours, 1, "one item");
    assert!(
        stored(&fixture.draft_store(), made.draft).is_none(),
        "retired"
    );
    assert!(drafts.read_with(cx, |registry, _| registry.is_idle()));
}

/// A create of a new document whose answer is lost after it ran (the
/// library's `BeforeRequestSettlement`): the made item, its draft, the
/// lost request, and the commits on the item's branch then.
async fn lost_create(
    fixture: &Fixture,
    backend: &DelayingBackend,
    services: &Services,
    root: &Path,
    cx: &mut TestAppContext,
) -> (NewItem, DesktopRequestId, String, usize) {
    let drafts = services.drafts().clone();
    let made = new_item(services, new_document(root, "New title"), cx).await;
    backend.replace_service(
        RepositoryService::open_at_with_failure_point_for_testing(
            fixture.data_dir(),
            FailurePoint::BeforeRequestSettlement,
        )
        .unwrap(),
    );
    let request = create(&drafts, made.draft, cx)
        .await
        .expect("the create is sent");
    finished(services, request, 1, cx).await;
    assert_eq!(
        code(services, request, cx),
        ResultCode::InternalError,
        "lost"
    );
    backend.replace_service(RepositoryService::open_at(fixture.data_dir()).unwrap());
    let shown = show(fixture.service(), root, DOCUMENT);
    let branch = shown.context.branch.clone().expect("the item's branch");
    let before = commits(root, &branch);
    (made, request, branch, before)
}

/// What a user does after a lost create answer, which offers no Retry:
/// Save again. Its text unchanged, the new create is `already_applied`:
/// one item, no second commit, and the draft retires.
#[gpui_kit::test]
async fn a_create_sent_again_after_a_lost_answer_completes_the_draft(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let (backend, services) = start(&fixture, cx);
    let drafts = services.drafts().clone();
    let root = fixture.create_repository("a");
    let (made, lost, branch, before) = lost_create(&fixture, &backend, &services, &root, cx).await;

    let again = create(&drafts, made.draft, cx).await.expect("sent again");
    assert_ne!(again, lost, "a new request");
    finished(&services, again, 1, cx).await;

    let (outcome, code, _) = answer(&services, again, cx);
    assert_eq!((outcome, code), (Outcome::Noop, ResultCode::AlreadyApplied));
    assert_eq!(commits(&root, &branch), before, "no second commit");
    let service = fixture.service();
    let repo = service.resolve_repository(&root).unwrap();
    let ours = service
        .list_documents(&repo)
        .unwrap()
        .items
        .iter()
        .filter(|item| item.id.as_deref() == Some(DOCUMENT))
        .count();
    assert_eq!(ours, 1, "one item");
    assert!(
        stored(&fixture.draft_store(), made.draft).is_none(),
        "retired"
    );
    assert!(drafts.read_with(cx, |registry, _| registry.is_idle()));
}

/// After a lost create answer, newer text saved again meets its own item
/// at the path (`occupied_path`): no second item, and the draft goes on
/// as an ordinary unverified draft, which its tab reviews.
#[gpui_kit::test]
async fn a_changed_create_sent_again_after_a_lost_answer_becomes_unverified(
    cx: &mut TestAppContext,
) {
    let fixture = Fixture::new();
    let (backend, services) = start(&fixture, cx);
    let drafts = services.drafts().clone();
    let root = fixture.create_repository("a");
    let (made, _, branch, before) = lost_create(&fixture, &backend, &services, &root, cx).await;

    edit(
        &drafts,
        made.draft,
        document_fields("Newer title", "Body\n"),
        cx,
    );
    let again = create(&drafts, made.draft, cx).await.expect("sent again");
    finished(&services, again, 1, cx).await;

    assert_eq!(code(&services, again, cx), ResultCode::OccupiedPath);
    assert_eq!(commits(&root, &branch), before, "no second commit");
    let shown = show(fixture.service(), &root, DOCUMENT);
    assert_eq!(shown.title.as_deref(), Some("New title"));
    let base = drafts.read_with(cx, |registry, _| {
        registry
            .session(made.draft)
            .and_then(|session| session.base().cloned())
            .expect("the session goes on, with a base")
    });
    assert!(!is_new_item_base(&base), "an ordinary unverified draft");
    assert_eq!(base.observation, None);
    assert_eq!(base.source.as_deref(), Some(""));
    assert!(create(&drafts, made.draft, cx).await.is_none(), "no create");
}

// ---- the tab ----

/// One repository, `a`, built by `build` and refreshed, in an open window.
fn shell(cx: &mut TestAppContext, build: impl FnOnce(&Fixture, &Path)) -> (Shell, PathBuf) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    build(&fixture, &root);
    fixture.refresh(&root);
    (Shell::open(fixture, cx), root)
}

fn ticket(root: &Path) -> OpenTarget {
    OpenTarget::Item {
        repo: key(root),
        kind: ItemDtoKind::Ticket,
        id: item_id(TICKET),
    }
}

fn open(shell: &Shell, root: &Path, cx: &mut TestAppContext) {
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(ticket(root), cx));
    shell.run(cx);
}

/// Stores a new item's draft of the ticket `TICKET` of `root`, titled
/// `title`, with `body`, and lets go of it: it is listed unclaimed, as a
/// draft left by an earlier run is.
fn store_new_ticket(fixture: &Fixture, root: &Path, title: &str, body: &str) -> DraftId {
    let mut fields = new_ticket_fields(title);
    if let DraftFields::Ticket { body: old, .. } = &mut fields {
        *old = body.to_owned();
    }
    let (target, base) = new_ticket_draft(root, TICKET, &fields);
    match fixture.draft_store().create(target, Some(base), fields) {
        Ok(claim) => claim.draft().id,
        Err(error) => panic!("the draft must be stored: {error}"),
    }
}

/// An earlier run's `context prepare` with the Create intent of the
/// ticket `TICKET` of `root`, through the library.
fn prepare_through(fixture: &Fixture, root: &Path) {
    let prepared = fixture.service().execute(
        MutationCall {
            request_id: RequestId::new(),
            confirmation: None,
            mutation: Mutation::ContextPrepare(ContextPrepareInput {
                root: root.to_owned(),
                kind: AuthoringKind::Ticket,
                id: item_id(TICKET),
                intent: ContextPrepareIntent::Create,
            }),
        },
        &mut SessionCredentials::new(NoPassphrase),
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(prepared.code, ResultCode::Ok, "{}", prepared.message);
}

fn pane(shell: &Shell, cx: &mut TestAppContext) -> Entity<crate::desktop::view::item::ItemPane> {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.item_pane())
        .expect("an item tab is shown")
}

fn phase(shell: &Shell, cx: &mut TestAppContext) -> Phase {
    pane(shell, cx).read_with(cx, |pane, _| pane.edit_phase())
}

fn tab_count(shell: &Shell, cx: &mut TestAppContext) -> usize {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.tab_keys().len())
}

/// The intents of this session's `context prepare` requests, oldest first:
/// whether each was a Create's, and its request.
fn prepares(shell: &Shell, cx: &mut TestAppContext) -> Vec<(bool, DesktopRequestId)> {
    let operations = shell.operations(cx);
    operations.read_with(cx, |operations, _| {
        operations
            .requests()
            .iter()
            .rev()
            .filter_map(|request| match &request.intent {
                Intent::ContextPrepare { intent, .. } => {
                    Some((matches!(intent, ContextPrepareIntent::Create), request.id))
                }
                _ => None,
            })
            .collect()
    })
}

/// This session's new items' creates, newest first.
fn creates(shell: &Shell, cx: &mut TestAppContext) -> Vec<DesktopRequestId> {
    shell.requests(cx, |request| {
        matches!(
            request.intent,
            Intent::TicketCreate { .. } | Intent::DocumentCreate { .. }
        )
    })
}

/// This session's `context release` requests.
fn releases(shell: &Shell, cx: &mut TestAppContext) -> Vec<DesktopRequestId> {
    shell.requests(cx, |request| {
        matches!(request.intent, Intent::ContextRelease { .. })
    })
}

fn request_code(
    shell: &Shell,
    id: DesktopRequestId,
    cx: &mut TestAppContext,
) -> Option<ResultCode> {
    shell
        .operations(cx)
        .read_with(cx, |operations, _| match &operations.request(id)?.state {
            RequestState::Finished(_, code, ..) => Some(*code),
            _ => None,
        })
}

/// Whether the ticket's work context is there: its worktree, then its
/// branch.
fn context_of(root: &Path) -> (bool, bool) {
    let worktree = root.join(".manyhands/worktrees").join(TICKET).exists();
    let branch = Repository::open(root)
        .unwrap()
        .find_reference(&format!("refs/heads/manyhands/ticket/{TICKET}"))
        .is_ok();
    (worktree, branch)
}

/// Whether this process counts the ticket's context as made here.
fn created(shell: &Shell, root: &Path, cx: &mut TestAppContext) -> bool {
    let created = shell.workspace.read_with(cx, |workspace, _| {
        workspace.services().created_contexts().clone()
    });
    created.read_with(cx, |created, _| {
        created.contains(&ContextKey::new(&key(root), ItemDtoKind::Ticket, TICKET))
    })
}

/// Draws frames, with the worker and the reads running, until `done`.
async fn until(
    shell: &Shell,
    cx: &mut TestAppContext,
    what: &str,
    done: impl Fn(&Shell, &mut TestAppContext) -> bool,
) {
    let deadline = Instant::now() + WAIT;
    loop {
        shell.frame(cx, |_, _| ());
        shell.run(cx);
        if done(shell, cx) {
            shell.frame(cx, |_, _| ());
            return;
        }
        assert!(Instant::now() < deadline, "{what} did not happen");
        cx.executor().timer(Duration::from_millis(10)).await;
    }
}

/// The shown tab's edit host, once its editor has opened.
async fn until_editing(shell: &Shell, cx: &mut TestAppContext) -> Entity<EditorHost> {
    until(shell, cx, "the edit editor opening", |shell, cx| {
        pane(shell, cx).read_with(cx, |pane, _| pane.edit_host().is_some())
    })
    .await;
    pane(shell, cx)
        .read_with(cx, |pane, _| pane.edit_host())
        .unwrap()
}

fn edit_text(host: &Entity<EditorHost>, cx: &mut TestAppContext) -> String {
    host.read_with(cx, |host, cx| host.text(cx))
}

/// Types `typed` at the end of the edit editor's text: the caret set, the
/// editor focused and a frame drawn first.
fn type_at_end(shell: &Shell, host: &Entity<EditorHost>, typed: &str, cx: &mut TestAppContext) {
    let editor = host.read_with(cx, |host, _| host.editor().clone());
    let end = editor.read_with(cx, |editor, _| editor.text().len());
    editor.update(cx, |editor, cx| editor.set_cursor(end, cx));
    let handle = editor.read_with(cx, |editor, cx| editor.focus_handle(cx));
    shell.frame(cx, |window, cx| window.focus(&handle, cx));
    shell.frame(cx, |_, _| ());
    shell.visual(cx).simulate_input(typed);
    shell.run(cx);
}

/// The base of the draft `id`'s session, as the registry holds it: the
/// review, once open, holds its writes back (R-44).
fn session_base(shell: &Shell, id: DraftId, cx: &mut TestAppContext) -> DraftBase {
    let drafts = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().drafts().clone());
    drafts.read_with(cx, |registry, _| {
        registry
            .session(id)
            .and_then(|session| session.base().cloned())
            .expect("the draft's session, with a base")
    })
}

/// R-42: a not-created draft, stored by an earlier run, is offered in its
/// item's tab with no item read; the Drafts page offers its Restore, which
/// restores it into that tab with Continue editing and Save offered, and
/// prepares nothing yet.
#[gpui_kit::test]
async fn a_not_created_draft_restores_into_a_tab_with_no_item_read(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        store_new_ticket(fixture, root, "Left before", "Body\n");
    });
    let id = shell.fixture.draft_store().list().unwrap()[0].id;
    open(&shell, &root, cx);
    assert!(!shell.present("item-title", cx), "no item is read");
    assert!(shell.present("draft-banner", cx), "the draft is offered");
    assert!(shell.present("draft-restore", cx));

    shell.keys("secondary-shift-d", cx);
    until(&shell, cx, "the Drafts page's check", |shell, cx| {
        shell.label(row_id(id, "state"), cx).as_deref() == Some(NOT_CREATED)
    })
    .await;
    assert!(shell.present(row_id(id, "restore"), cx), "restorable");
    shell.click(row_id(id, "restore"), cx);
    until(&shell, cx, "the restore", |shell, cx| {
        shell.present("draft-section", cx)
    })
    .await;

    assert_eq!(
        held(&shell.fixture.draft_store(), id),
        Some(Held::Here),
        "claimed"
    );
    assert!(!shell.present("item-title", cx), "still no item read");
    assert_eq!(
        shell.label("item-edit", cx).as_deref(),
        Some("Continue editing"),
        "the header offers it, with no item read"
    );
    assert_eq!(
        shell.label("draft-continue", cx).as_deref(),
        Some("Continue editing")
    );
    assert!(!shell.present("draft-continue-note", cx), "offered");
    assert_eq!(shell.label("draft-save", cx).as_deref(), Some("Save"));
    assert!(!shell.present("draft-save-note", cx), "offered");
    assert!(prepares(&shell, cx).is_empty(), "nothing is prepared yet");
}

/// Restores the not-created draft stored for `root`'s ticket into its tab,
/// from the banner, and waits for D2's section.
async fn restore_in_tab(shell: &Shell, root: &Path, cx: &mut TestAppContext) {
    open(shell, root, cx);
    shell.click("draft-restore-button", cx);
    until(shell, cx, "the restore", |shell, cx| {
        shell.present("draft-section", cx)
    })
    .await;
}

/// The ticket `TICKET` of `root`, as read now, if it exists.
fn ticket_read(shell: &Shell, root: &Path) -> Option<ItemDto> {
    let service = shell.fixture.service();
    let repo = service.resolve_repository(root).unwrap();
    service.show_item(&repo, &item_id(TICKET)).ok()
}

/// D2's section's Save of a restored not-created draft (R-42): Continue
/// editing's steps with the Create intent, then its create. The item is
/// created, and the draft retires.
#[gpui_kit::test]
async fn save_of_a_restored_not_created_draft_creates_it(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        store_new_ticket(fixture, root, "Left before", "Draft body\n");
    });
    let id = shell.fixture.draft_store().list().unwrap()[0].id;
    restore_in_tab(&shell, &root, cx).await;

    shell.click("draft-save-button", cx);
    until(&shell, cx, "the create", |shell, cx| {
        creates(shell, cx)
            .first()
            .is_some_and(|request| request_code(shell, *request, cx).is_some())
    })
    .await;
    until(&shell, cx, "the draft retiring", |shell, _| {
        held(&shell.fixture.draft_store(), id).is_none()
    })
    .await;

    let sent = creates(&shell, cx);
    assert_eq!(sent.len(), 1, "one create");
    assert_eq!(request_code(&shell, sent[0], cx), Some(ResultCode::Ok));
    let prepared = prepares(&shell, cx);
    assert!(prepared.len() == 1 && prepared[0].0, "one Create prepare");
    let shown = ticket_read(&shell, &root).expect("the item is created");
    assert_eq!(shown.title.as_deref(), Some("Left before"));
    assert_eq!(shown.body.as_deref(), Some("Draft body\n"));
    let branch = shown.context.branch.clone().expect("the item's branch");
    let repository = Repository::open(&root).unwrap();
    let tip = repository
        .revparse_single(&format!("refs/heads/{branch}"))
        .unwrap()
        .peel_to_commit()
        .unwrap();
    assert!(
        tip.tree()
            .unwrap()
            .get_path(Path::new(&format!(".manyhands/tickets/{TICKET}/ticket.md")))
            .is_ok(),
        "the item's file is committed"
    );
}

/// The close dialog's Save of a restored not-created draft (R-42): the
/// dialog offers it, the item is created once, and the tab closes.
#[gpui_kit::test]
async fn the_close_dialogs_save_of_a_not_created_draft_creates_it_and_closes(
    cx: &mut TestAppContext,
) {
    let (shell, root) = shell(cx, |fixture, root| {
        store_new_ticket(fixture, root, "Left before", "Draft body\n");
    });
    let id = shell.fixture.draft_store().list().unwrap()[0].id;
    restore_in_tab(&shell, &root, cx).await;

    shell.click(("tab-close", 0usize), cx);
    assert!(shell.present("close-draft-dialog", cx), "it asks first");
    assert_eq!(shell.label("close-draft-save", cx).as_deref(), Some("Save"));
    assert!(!shell.present("close-draft-save-note", cx), "offered");
    shell.click("close-draft-save-button", cx);
    until(&shell, cx, "the tab closing", |shell, cx| {
        tab_count(shell, cx) == 0
    })
    .await;

    assert!(!shell.present("close-draft-dialog", cx), "closed with it");
    let sent = creates(&shell, cx);
    assert_eq!(sent.len(), 1, "created once");
    assert_eq!(request_code(&shell, sent[0], cx), Some(ResultCode::Ok));
    let shown = ticket_read(&shell, &root).expect("the item is created");
    assert_eq!(shown.title.as_deref(), Some("Left before"));
    assert!(held(&shell.fixture.draft_store(), id).is_none(), "retired");
}

/// R-42: Continue editing of a not-created draft prepares with the Create
/// intent, and the edit editor opens on the draft's text with no item
/// read; its Save creates, with no review first.
#[gpui_kit::test]
async fn continue_editing_a_not_created_draft_prepares_with_create(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        store_new_ticket(fixture, root, "Left before", "Draft body\n");
    });
    open(&shell, &root, cx);
    shell.click("draft-banner-continue-button", cx);
    let host = until_editing(&shell, cx).await;

    let sent = prepares(&shell, cx);
    assert_eq!(sent.len(), 1, "one prepare");
    assert!(sent[0].0, "with the Create intent");
    assert_eq!(request_code(&shell, sent[0].1, cx), Some(ResultCode::Ok));
    assert_eq!(edit_text(&host, cx), "Draft body\n", "the draft's text");
    assert_eq!(phase(&shell, cx), Phase::Editing);
    assert!(created(&shell, &root, cx), "the prepare made the context");
    assert_eq!(shell.label("item-save", cx).as_deref(), Some("Save"));
    assert!(
        !shell.present("item-save-unverified", cx),
        "no review first"
    );
}

/// R-42: a new item's draft whose item exists after all takes the Edit
/// intent, and the review opens; the draft goes on unverified, its marker
/// cleared.
#[gpui_kit::test]
async fn a_marked_draft_whose_item_exists_routes_to_edit_and_review(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.create_ticket(root, TICKET, "Created elsewhere", "open");
        store_new_ticket(fixture, root, "Mine", "Body\n");
    });
    let id = shell.fixture.draft_store().list().unwrap()[0].id;
    open(&shell, &root, cx);
    assert!(shell.present("item-title", cx), "the item is read");
    shell.click("draft-banner-continue-button", cx);
    until(&shell, cx, "the review", |shell, cx| {
        phase(shell, cx) == Phase::Review
    })
    .await;

    let sent = prepares(&shell, cx);
    assert_eq!(sent.len(), 1, "one prepare");
    assert!(!sent[0].0, "with the Edit intent");
    let base = session_base(&shell, id, cx);
    assert!(!is_new_item_base(&base), "an ordinary unverified draft");
    assert_eq!(base.observation, None);
    assert_eq!(base.source.as_deref(), Some(""));
}

/// R-42: the tab read the item missing, then the item appeared: Continue
/// editing's Create is answered `occupied_path`, and the draft takes the
/// Edit route and the review.
#[gpui_kit::test]
async fn continue_editing_a_not_created_draft_whose_item_appeared_routes_to_review(
    cx: &mut TestAppContext,
) {
    let (shell, root) = shell(cx, |fixture, root| {
        store_new_ticket(fixture, root, "Mine", "Body\n");
    });
    let id = shell.fixture.draft_store().list().unwrap()[0].id;
    open(&shell, &root, cx);
    assert!(!shell.present("item-title", cx), "read missing");
    // Created by another actor; the tab does not read it again.
    shell
        .fixture
        .create_ticket(&root, TICKET, "Created elsewhere", "open");
    shell.click("draft-banner-continue-button", cx);
    until(&shell, cx, "the review", |shell, cx| {
        phase(shell, cx) == Phase::Review
    })
    .await;

    let sent = prepares(&shell, cx);
    assert_eq!(sent.len(), 2, "Create, then Edit");
    assert!(sent[0].0 && !sent[1].0);
    assert_eq!(
        request_code(&shell, sent[0].1, cx),
        Some(ResultCode::OccupiedPath)
    );
    assert_eq!(request_code(&shell, sent[1].1, cx), Some(ResultCode::Ok));
    let base = session_base(&shell, id, cx);
    assert!(!is_new_item_base(&base), "an ordinary unverified draft");
    assert_eq!(base.observation, None);
}

/// Save's one gate (`save_blocked`) holds a new item's create too: a new
/// ticket with an empty Title is not created, from Save or `secondary-s`,
/// and D2's section says why before Continue editing.
#[gpui_kit::test]
async fn an_untitled_new_ticket_is_not_created(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        store_new_ticket(fixture, root, "", "Body\n");
    });
    open(&shell, &root, cx);
    shell.click("draft-restore-button", cx);
    until(&shell, cx, "the restore", |shell, cx| {
        shell.present("draft-section", cx)
    })
    .await;
    assert_eq!(
        shell.label("draft-save-note", cx).as_deref(),
        Some("A ticket needs a title before it can be saved.")
    );
    shell.click("draft-continue-button", cx);
    let host = until_editing(&shell, cx).await;
    type_at_end(&shell, &host, "x", cx);

    shell.click("item-save", cx);
    shell.keys("secondary-s", cx);
    assert!(creates(&shell, cx).is_empty(), "nothing is created");
    assert_eq!(phase(&shell, cx), Phase::Editing);
}

/// R-73: a titled new item with an empty body is dirty until created, so
/// its Save is offered, and creates it.
#[gpui_kit::test]
async fn a_titled_new_item_with_an_empty_body_can_be_saved(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |_, _| {});
    let (id, host) = shell.edit_new_ticket(&root, TICKET, "Titled", cx).await;
    assert_eq!(edit_text(&host, cx), "", "nothing typed");
    assert_eq!(shell.label("item-save", cx).as_deref(), Some("Save"));

    shell.click("item-save", cx);
    let sent = creates(&shell, cx);
    assert_eq!(sent.len(), 1, "Save sent one create");
    shell.until_ended(sent[0], cx).await;
    until(&shell, cx, "the draft retiring", |shell, _| {
        shell.fixture.draft_store().list().unwrap().is_empty()
    })
    .await;

    assert_eq!(request_code(&shell, sent[0], cx), Some(ResultCode::Ok));
    let shown = show(shell.fixture.service(), &root, TICKET);
    assert_eq!(shown.title.as_deref(), Some("Titled"));
    assert_eq!(shown.status.as_deref(), Some("open"));
    assert_eq!(shown.body.as_deref(), Some(""));
    assert!(held(&shell.fixture.draft_store(), id).is_none(), "retired");
    assert_eq!(phase(&shell, cx), Phase::Editing, "the session goes on");
    assert!(!created(&shell, &root, cx), "the create holds work now");
}

/// An occupied path that holds this draft's own item (an earlier create's
/// answer was lost): the draft becomes an ordinary unverified draft, and
/// the review opens.
#[gpui_kit::test]
async fn an_occupied_path_for_this_drafts_own_item_becomes_a_review(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |_, _| {});
    let (id, host) = shell.edit_new_ticket(&root, TICKET, "Mine", cx).await;
    type_at_end(&shell, &host, "x", cx);
    // The item, created at its path in its own context with other fields:
    // what an earlier create whose answer was lost left.
    shell
        .fixture
        .create_ticket(&root, TICKET, "Created before", "open");

    shell.click("item-save", cx);
    let sent = creates(&shell, cx);
    assert_eq!(sent.len(), 1, "Save sent one create");
    shell.until_ended(sent[0], cx).await;
    assert_eq!(
        request_code(&shell, sent[0], cx),
        Some(ResultCode::OccupiedPath)
    );
    until(&shell, cx, "the review", |shell, cx| {
        phase(shell, cx) == Phase::Review
    })
    .await;

    let base = session_base(&shell, id, cx);
    assert!(!is_new_item_base(&base), "an ordinary unverified draft");
    assert_eq!(base.observation, None);
    assert_eq!(base.source.as_deref(), Some(""));
    assert_eq!(title_of(&base.fields), "Mine");
    assert!(!shell.present("item-save-refused", cx), "the review's now");
}

/// The finished request `id`'s message, as the library gave it.
fn request_message(shell: &Shell, id: DesktopRequestId, cx: &mut TestAppContext) -> String {
    shell.operations(cx).read_with(cx, |operations, _| {
        match &operations.request(id).unwrap().state {
            RequestState::Finished(.., message) => message.clone(),
            _ => panic!("the request must have finished"),
        }
    })
}

/// Part D fix 1, M5: an `occupied_path` whose path cannot then be read,
/// here with this draft's own item there, names no other file: the tab
/// shows the library's refusal, its message and code, as for any other.
/// The draft is kept, still a new item's, and Save stays offered.
#[gpui_kit::test]
async fn an_occupied_path_that_cannot_be_read_names_no_other_file(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |_, _| {});
    let (id, host) = shell.edit_new_ticket(&root, TICKET, "Mine", cx).await;
    type_at_end(&shell, &host, "x", cx);
    // This draft's own item at its path: what a lost create answer left.
    shell
        .fixture
        .create_ticket(&root, TICKET, "Created before", "open");

    let read = shell.backend.hold_read(ReadKind::DraftItem);
    shell.click("item-save", cx);
    let sent = creates(&shell, cx);
    assert_eq!(sent.len(), 1, "Save sent one create");
    shell.until_ended(sent[0], cx).await;
    assert_eq!(
        request_code(&shell, sent[0], cx),
        Some(ResultCode::OccupiedPath)
    );
    // The path is read on a service that knows no repository: it fails.
    let elsewhere = tempfile::tempdir().unwrap();
    shell
        .backend
        .replace_service(RepositoryService::open_at(elsewhere.path()).unwrap());
    read.send(()).unwrap();
    until(&shell, cx, "the refusal", |shell, cx| {
        shell.present("item-save-code", cx)
    })
    .await;
    shell
        .backend
        .replace_service(RepositoryService::open_at(shell.fixture.data_dir()).unwrap());

    let words = shell.label("item-save-refused", cx);
    assert!(
        !words.is_some_and(|words| words.contains("Another file")),
        "no other file is named"
    );
    assert_eq!(
        shell.label("item-save-message", cx),
        Some(request_message(&shell, sent[0], cx))
    );
    assert_eq!(
        shell.label("item-save-code", cx).as_deref(),
        Some(ResultCode::OccupiedPath.as_str())
    );
    assert_eq!(phase(&shell, cx), Phase::Editing, "no review");
    assert!(
        is_new_item_base(&session_base(&shell, id, cx)),
        "the draft is kept, still a new item's"
    );
    assert_eq!(held(&shell.fixture.draft_store(), id), Some(Held::Here));
    // Save stays offered: it sends the create again.
    shell.click("item-save", cx);
    let again = creates(&shell, cx);
    assert_eq!(again.len(), 2, "Save stays offered");
    shell.until_ended(again[0], cx).await;
}

/// Part D fix 1, M1: a create whose answer was lost, sent again by Save
/// and answered `already_applied`, has its item's commit in the context:
/// the context leaves `CreatedContexts`, and Done, which ends the clean
/// session, sends no release, which the library would refuse.
#[gpui_kit::test]
async fn done_after_a_create_found_already_applied_releases_nothing(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |_, _| {});
    let (id, host) = shell.edit_new_ticket(&root, TICKET, "Lost", cx).await;
    assert!(created(&shell, &root, cx), "this session made the context");
    type_at_end(&shell, &host, "x", cx);

    // The create runs, and its answer is lost (the library's
    // `BeforeRequestSettlement`).
    shell.backend.replace_service(
        RepositoryService::open_at_with_failure_point_for_testing(
            shell.fixture.data_dir(),
            FailurePoint::BeforeRequestSettlement,
        )
        .unwrap(),
    );
    shell.click("item-save", cx);
    let lost = creates(&shell, cx);
    assert_eq!(lost.len(), 1, "Save sent one create");
    shell.until_ended(lost[0], cx).await;
    assert_eq!(
        request_code(&shell, lost[0], cx),
        Some(ResultCode::InternalError),
        "lost"
    );
    shell
        .backend
        .replace_service(RepositoryService::open_at(shell.fixture.data_dir()).unwrap());
    assert!(ticket_read(&shell, &root).is_some(), "it ran");

    // Save again, the text unchanged: the library finds it applied.
    assert_eq!(phase(&shell, cx), Phase::Editing);
    shell.click("item-save", cx);
    let sent = creates(&shell, cx);
    assert_eq!(sent.len(), 2, "a new request");
    shell.until_ended(sent[0], cx).await;
    assert_eq!(
        request_code(&shell, sent[0], cx),
        Some(ResultCode::AlreadyApplied)
    );
    until(&shell, cx, "the draft retiring", |shell, _| {
        held(&shell.fixture.draft_store(), id).is_none()
    })
    .await;
    assert!(
        !created(&shell, &root, cx),
        "the context holds the item's commit"
    );

    shell.click("item-done", cx);
    until(&shell, cx, "Done ending the session", |shell, cx| {
        pane(shell, cx).read_with(cx, |pane, _| pane.edit_host().is_none())
    })
    .await;
    shell.run(cx);
    assert!(releases(&shell, cx).is_empty(), "nothing is released");
    assert_eq!(context_of(&root), (true, true), "the context is kept");
}

/// Decision 18: Discard of a new item's draft, its context made by this
/// session's Create prepare, closes the tab and releases that context.
#[gpui_kit::test]
async fn discarding_a_new_item_draft_releases_the_context_this_session_made(
    cx: &mut TestAppContext,
) {
    let (shell, root) = shell(cx, |_, _| {});
    let (id, host) = shell.edit_new_ticket(&root, TICKET, "New", cx).await;
    assert!(created(&shell, &root, cx), "this session made the context");
    assert_eq!(context_of(&root), (true, true));
    type_at_end(&shell, &host, "x", cx);

    shell.click("item-done", cx);
    shell.run(cx);
    assert!(shell.present("done-draft-dialog", cx), "Done asks");
    shell.click("done-draft-discard-button", cx);
    until(&shell, cx, "a release", |shell, cx| {
        releases(shell, cx)
            .first()
            .is_some_and(|request| request_code(shell, *request, cx).is_some())
    })
    .await;

    assert_eq!(tab_count(&shell, cx), 0, "the tab closes");
    assert!(
        held(&shell.fixture.draft_store(), id).is_none(),
        "discarded"
    );
    let sent = releases(&shell, cx);
    assert_eq!(sent.len(), 1, "one release");
    assert_eq!(request_code(&shell, sent[0], cx), Some(ResultCode::Ok));
    assert_eq!(context_of(&root), (false, false), "the context is gone");
}

/// Decision 18: a not-created draft restored after a relaunch keeps its
/// context, which an earlier run made, when it is discarded.
#[gpui_kit::test]
async fn discarding_a_restored_new_item_draft_keeps_its_context(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        prepare_through(fixture, root);
        store_new_ticket(fixture, root, "Left before", "Body\n");
    });
    let id = shell.fixture.draft_store().list().unwrap()[0].id;
    open(&shell, &root, cx);
    shell.click("draft-banner-continue-button", cx);
    let host = until_editing(&shell, cx).await;
    let sent = prepares(&shell, cx);
    assert!(sent.len() == 1 && sent[0].0, "one Create prepare");
    assert!(!created(&shell, &root, cx), "found, not made");
    type_at_end(&shell, &host, "x", cx);

    shell.click("item-done", cx);
    shell.run(cx);
    shell.click("done-draft-discard-button", cx);
    until(&shell, cx, "the tab closing", |shell, cx| {
        tab_count(shell, cx) == 0
    })
    .await;
    shell.run(cx);

    assert!(
        held(&shell.fixture.draft_store(), id).is_none(),
        "discarded"
    );
    assert!(releases(&shell, cx).is_empty(), "nothing is released");
    assert_eq!(context_of(&root), (true, true), "the context is kept");
}
