//! Release on leave, over real repositories (design, Done and leaving:
//! Release on leave; decision 18; R-37, R-40, R-75, R-78): the design's
//! session-outcome rows for Task 17 (R-76), written first; the leaves that
//! release a context this run made, and the ones that keep a context; a tab
//! closed while it prepares; Discard while a save is in flight; a stranded
//! context released from the tab; and R-78's two rules on sending a context
//! request again.
//!
//! The helpers are `edit_tests`'s. Each test sets the caret, focuses the
//! editor and draws a frame before typing, and types through the window;
//! each waits on the worker and allows parking. Draft fields and items
//! would print text through `Debug`, so they are compared with `assert!`.
//! The words are literals: the tests fix them.

use git2::Repository;
use manyhands::{
    drafts::{DraftError, DraftErrorKind},
    repository::{
        AuthoringKind, CancellationToken, ContextPrepareInput, ContextPrepareIntent, FailurePoint,
        Mutation, MutationCall, NoProgress, OperationAction, RequestId, keys::SessionCredentials,
    },
    results::RecoveryActionKind,
};

use super::*;
#[cfg(unix)]
use crate::desktop::test_support::drafts::{FolderMode, superuser};
use crate::desktop::{
    model::list_state::ListState,
    services::requests::Lane,
    test_support::fixtures::{NoPassphrase, abandon_through},
    view::operations::recovery_id,
};

const VIEWING_PRIVATE: &str = "Viewing your private copy";
const STRANDED: &str = "Release stranded work context";
const NOT_SENT_AGAIN: &str = "Retry (Not sent again after a later Edit or release)";

/// The `context release` requests of this session for the ticket `id`,
/// newest first.
fn releases(shell: &Shell, id: &str, cx: &mut TestAppContext) -> Vec<DesktopRequestId> {
    shell.requests(cx, |request| {
        matches!(&request.intent, Intent::ContextRelease { id: of, .. } if of.to_string() == id)
    })
}

/// Every `context release` request of this session.
fn all_releases(shell: &Shell, cx: &mut TestAppContext) -> usize {
    shell
        .requests(cx, |request| {
            matches!(request.intent, Intent::ContextRelease { .. })
        })
        .len()
}

/// Waits until a `context release` of the ticket `id` has been sent and
/// has ended, with the foreground and the worker running; gives it.
async fn released(shell: &Shell, id: &str, cx: &mut TestAppContext) -> DesktopRequestId {
    let deadline = Instant::now() + WAIT;
    loop {
        shell.run(cx);
        if let Some(&request) = releases(shell, id, cx).first() {
            ended(shell, request, cx).await;
            return request;
        }
        assert!(Instant::now() < deadline, "no context release was sent");
        cx.executor().timer(Duration::from_millis(10)).await;
    }
}

/// Whether the ticket `id`'s work context is there: its worktree, then its
/// branch.
fn context_of(root: &Path, id: &str) -> (bool, bool) {
    let worktree = root.join(".manyhands/worktrees").join(id).exists();
    let repository = Repository::open(root).unwrap();
    let branch = repository
        .find_reference(&format!("refs/heads/manyhands/ticket/{id}"))
        .is_ok();
    (worktree, branch)
}

/// Whether release on leave counts the ticket `id`'s context as made here,
/// with nothing saved into it, and not released since.
fn made_here(shell: &Shell, root: &Path, id: &str, cx: &mut TestAppContext) -> bool {
    let item = context_key(root, id);
    release_service(shell, cx).read_with(cx, |releases, cx| releases.made_here(&item, cx))
}

/// The release `request` answered `ok`: the ticket `id`'s worktree and
/// branch are gone, and the context is not released again.
fn assert_released(
    shell: &Shell,
    root: &Path,
    id: &str,
    request: DesktopRequestId,
    cx: &mut TestAppContext,
) {
    assert_eq!(code(shell, request, cx), Some(ResultCode::Ok), "released");
    assert_eq!(context_of(root, id), (false, false), "the context is gone");
    assert!(!made_here(shell, root, id, cx), "not released again");
}

/// The ticket `id`'s context is kept: its worktree and branch are there,
/// and no release of it was sent.
fn assert_kept(shell: &Shell, root: &Path, id: &str, cx: &mut TestAppContext) {
    shell.run(cx);
    assert!(releases(shell, id, cx).is_empty(), "nothing is released");
    assert_eq!(context_of(root, id), (true, true), "the context is kept");
}

/// Presses Done, and lets what it starts finish.
fn done(shell: &Shell, cx: &mut TestAppContext) {
    shell.click("item-done", cx);
    shell.run(cx);
}

/// Whether the shown tab's edit session lives, and whether it is kept.
fn kept(shell: &Shell, cx: &mut TestAppContext) -> Option<bool> {
    pane(shell, cx).read_with(cx, |pane, _| {
        pane.edit_session().map(|session| session.kept())
    })
}

fn tab_count(shell: &Shell, cx: &mut TestAppContext) -> usize {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.tab_keys())
        .len()
}

/// The Drafts page's Discard… of the draft `id`, confirmed.
fn discard_on_the_drafts_page(shell: &Shell, id: DraftId, cx: &mut TestAppContext) {
    shell.keys("secondary-shift-d", cx);
    shell.run(cx);
    shell.click(row_id(id, "discard"), cx);
    assert!(shell.present("discard-draft-dialog", cx), "it asks first");
    shell.click("discard-draft-confirm", cx);
    shell.run(cx);
}

/// The ticket `id` of `root`'s context key.
fn context_key(root: &Path, id: &str) -> ContextKey {
    ContextKey::new(&key(root), ItemDtoKind::Ticket, id)
}

/// The window's release service.
fn release_service(
    shell: &Shell,
    cx: &mut TestAppContext,
) -> Entity<crate::desktop::services::release::Releases> {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().releases().clone())
}

/// The finished request `id`'s attempt.
fn attempt(shell: &Shell, id: DesktopRequestId, cx: &mut TestAppContext) -> u32 {
    shell
        .operations(cx)
        .read_with(cx, |operations, _| operations.request(id).unwrap().attempt)
}

/// The control the operation area shows for the request `id`'s Retry.
fn retry_control(shell: &Shell, id: DesktopRequestId, cx: &mut TestAppContext) -> String {
    let operations = shell.operations(cx);
    let n = operations.read_with(cx, |operations, _| {
        match &operations.request(id).unwrap().state {
            RequestState::Finished(_, _, _, recovery, _) => recovery
                .iter()
                .position(|action| action.action == RecoveryActionKind::RequestRetry)
                .expect("Retry is offered"),
            _ => panic!("not finished"),
        }
    });
    recovery_id(id, n)
}

fn failing_at(shell: &Shell, point: FailurePoint) {
    shell.backend.replace_service(
        RepositoryService::open_at_with_failure_point_for_testing(shell.fixture.data_dir(), point)
            .unwrap(),
    );
}

fn working(shell: &Shell) {
    shell
        .backend
        .replace_service(RepositoryService::open_at(shell.fixture.data_dir()).unwrap());
}

// The design's session-outcome rows for Task 17 (R-76), written first.

/// Row: the Drafts page discards a kept draft whose context is in
/// `CreatedContexts`. That context is released.
#[gpui_kit::test]
async fn a_drafts_page_discard_of_a_kept_draft_releases_a_context_this_run_made(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Kept here", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    assert!(created(&shell, &root, TICKET, cx), "made by this run");
    type_at_end(&shell, &host, "x", cx);
    let id = attached(&shell, &root, TICKET, cx).expect("the draft began");
    done(&shell, cx);
    assert_eq!(kept(&shell, cx), Some(true), "the session is kept");
    assert!(
        releases(&shell, TICKET, cx).is_empty(),
        "kept, not released"
    );

    discard_on_the_drafts_page(&shell, id, cx);
    let request = released(&shell, TICKET, cx).await;
    assert_released(&shell, &root, TICKET, request, cx);
    assert!(
        held(&shell.fixture.draft_store(), id).is_none(),
        "discarded"
    );
    assert_eq!(releases(&shell, TICKET, cx), [request], "one release");
}

/// Row: the Drafts page discards a never-created item's draft that a tab
/// holds. The tab closes, and a context in `CreatedContexts` is released.
/// (The session is marked as a new item's, as in Task 16's tests.)
#[gpui_kit::test]
async fn a_drafts_page_discard_of_a_held_never_created_draft_closes_the_tab_and_releases(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Never created", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    pane(&shell, cx).update(cx, |pane, _| pane.edit_session_mut().unwrap().as_new_item());
    type_at_end(&shell, &host, "x", cx);
    let id = attached(&shell, &root, TICKET, cx).expect("the draft began");
    assert!(created(&shell, &root, TICKET, cx));

    discard_on_the_drafts_page(&shell, id, cx);
    assert_eq!(tab_count(&shell, cx), 0, "the tab closes");
    let request = released(&shell, TICKET, cx).await;
    assert_released(&shell, &root, TICKET, request, cx);
}

/// Row: Keep draft on close after an Edit that created the context; later
/// in the same run, the Drafts page discards that draft. The context is
/// released then, not before.
#[gpui_kit::test]
async fn keep_draft_then_a_later_drafts_page_discard_releases_the_context(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Kept for later", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    type_at_end(&shell, &host, "x", cx);
    let id = attached(&shell, &root, TICKET, cx).expect("the draft began");
    close_keeping(&shell, id, cx);
    assert_eq!(tab_count(&shell, cx), 0, "Keep draft closes the tab");
    assert_kept(&shell, &root, TICKET, cx);
    assert!(created(&shell, &root, TICKET, cx), "still made by this run");

    discard_on_the_drafts_page(&shell, id, cx);
    let request = released(&shell, TICKET, cx).await;
    assert_released(&shell, &root, TICKET, request, cx);
    assert!(
        held(&shell.fixture.draft_store(), id).is_none(),
        "discarded"
    );
}

/// Row: a discard of a draft whose context this process did not create.
/// Nothing is released: the context was made before this run (a save
/// provisions it), so Edit found it.
#[gpui_kit::test]
async fn discarding_a_draft_whose_context_this_run_did_not_make_releases_nothing(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Made before", "");
    });
    shell.fixture.save_ticket(&root, TICKET, "Saved before");
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    assert!(!created(&shell, &root, TICKET, cx), "found, not made");
    type_at_end(&shell, &host, "x", cx);
    let id = attached(&shell, &root, TICKET, cx).expect("the draft began");
    done(&shell, cx);
    assert_eq!(kept(&shell, cx), Some(true));

    discard_on_the_drafts_page(&shell, id, cx);
    assert!(
        held(&shell.fixture.draft_store(), id).is_none(),
        "discarded"
    );
    assert_kept(&shell, &root, TICKET, cx);
    assert_eq!(all_releases(&shell, cx), 0);
}

// The other tests (R-37, R-40, R-75, R-78).

/// Done after an Edit that changed nothing releases the context the Edit
/// made. While the release runs, the item reads as having one pending (for
/// Move, Task 24); once it is done the tab reads the shared copy again.
#[gpui_kit::test]
async fn done_after_an_unused_edit_releases_the_context(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Unused", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    edit(&shell, cx).await;
    assert_eq!(
        context_of(&root, TICKET),
        (true, true),
        "the context is made"
    );

    let held = shell.backend.hold_job(JobKind::ContextRelease);
    done(&shell, cx);
    held.wait_reached();
    assert_eq!(phase(&shell, cx), Phase::Viewing);
    assert_eq!(pill(&shell, cx).as_deref(), Some(VIEWING_PRIVATE));
    let releases_service = release_service(&shell, cx);
    let item = context_key(&root, TICKET);
    assert!(
        releases_service.read_with(cx, |releases, _| releases.pending(&item)),
        "a release is running"
    );
    held.release();
    let request = released(&shell, TICKET, cx).await;
    assert_released(&shell, &root, TICKET, request, cx);
    assert!(
        !releases_service.read_with(cx, |releases, _| releases.pending(&item)),
        "no release is pending"
    );
    let shown = pane(&shell, cx);
    wait_until(cx, &shown, |pane, _| {
        pane.model
            .item()
            .rows()
            .is_some_and(|item| item.context.kind != manyhands::repository::ItemContextKind::Active)
    })
    .await;
    assert_eq!(pill(&shell, cx).as_deref(), Some(VIEWING_SHARED));
}

/// Decision 18 across releases: a context this run makes again after its
/// release is released again; one another process makes after the release
/// is found by the next Edit (`created: false`), and kept, though this run
/// once made a context of the item.
#[gpui_kit::test]
async fn a_context_made_again_is_released_again_and_one_made_elsewhere_is_kept(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Made twice", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    edit(&shell, cx).await;
    done(&shell, cx);
    let first = released(&shell, TICKET, cx).await;
    assert_released(&shell, &root, TICKET, first, cx);

    edit(&shell, cx).await;
    assert!(made_here(&shell, &root, TICKET, cx), "made again");
    done(&shell, cx);
    let deadline = Instant::now() + WAIT;
    while releases(&shell, TICKET, cx).len() < 2 {
        assert!(Instant::now() < deadline, "no second release was sent");
        shell.run(cx);
        cx.executor().timer(Duration::from_millis(10)).await;
    }
    let second = releases(&shell, TICKET, cx)[0];
    ended(&shell, second, cx).await;
    assert_released(&shell, &root, TICKET, second, cx);

    // Another process makes the context.
    let made = shell.fixture.service().execute(
        MutationCall {
            request_id: RequestId::new(),
            confirmation: None,
            mutation: Mutation::ContextPrepare(ContextPrepareInput {
                root: root.clone(),
                kind: AuthoringKind::Ticket,
                id: item_id(TICKET),
                intent: ContextPrepareIntent::Edit,
            }),
        },
        &mut SessionCredentials::new(NoPassphrase),
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(made.code, ResultCode::Ok, "{}", made.message);
    edit(&shell, cx).await;
    assert_eq!(
        code(&shell, prepares(&shell, cx)[0], cx),
        Some(ResultCode::Ok)
    );
    assert!(!made_here(&shell, &root, TICKET, cx), "found, not made");
    done(&shell, cx);
    shell.run(cx);
    assert_eq!(releases(&shell, TICKET, cx), [second, first], "kept");
    assert_eq!(context_of(&root, TICKET), (true, true));
}

/// Discard after an Edit that saved nothing releases its context: from the
/// kept session's banner, and from the close dialog, which closes the tab.
#[gpui_kit::test]
async fn discard_after_an_unused_edit_releases_the_context(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Discarded kept", "");
        fixture.commit_ticket(root, OTHER, "Discarded closing", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    type_at_end(&shell, &host, "x", cx);
    done(&shell, cx);
    assert_eq!(kept(&shell, cx), Some(true));
    shell.click("item-kept-discard", cx);
    assert!(shell.present("discard-draft-dialog", cx), "it asks first");
    shell.click("discard-draft-confirm", cx);
    shell.run(cx);
    assert_eq!(kept(&shell, cx), None, "the session ends");
    let request = released(&shell, TICKET, cx).await;
    assert_released(&shell, &root, TICKET, request, cx);

    open(&shell, ticket(&root, OTHER), cx);
    let host = edit(&shell, cx).await;
    type_at_end(&shell, &host, "y", cx);
    attached(&shell, &root, OTHER, cx).expect("the draft began");
    shell.keys("secondary-w", cx);
    assert!(shell.present("close-draft-dialog", cx), "it asks first");
    shell.click("close-draft-discard-button", cx);
    shell.run(cx);
    assert_eq!(tab_count(&shell, cx), 1, "Discard closes the tab");
    let request = released(&shell, OTHER, cx).await;
    assert_released(&shell, &root, OTHER, request, cx);
}

/// Closing a clean tab after an Edit that changed nothing closes it at once
/// and releases the context.
#[gpui_kit::test]
async fn closing_a_clean_tab_after_an_unused_edit_releases_the_context(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Closed clean", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    edit(&shell, cx).await;
    shell.keys("secondary-w", cx);
    assert!(!shell.present("close-draft-dialog", cx), "nothing to ask");
    assert_eq!(tab_count(&shell, cx), 0, "it closes at once");
    let request = released(&shell, TICKET, cx).await;
    assert_released(&shell, &root, TICKET, request, cx);
}

/// A tab closed while Loading, its prepare answered and its read not yet
/// landed, releases the context this run made, as a clean tab does.
#[gpui_kit::test]
async fn closing_the_tab_during_loading_releases_a_context_this_run_made(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Closed loading", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let reads = [
        shell.backend.hold_read(ReadKind::Item),
        shell.backend.hold_read(ReadKind::Item),
    ];
    shell.click("item-edit", cx);
    let request = prepares(&shell, cx)[0];
    ended(&shell, request, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));
    assert_eq!(phase(&shell, cx), Phase::Loading, "the read is held");
    assert!(created(&shell, &root, TICKET, cx));

    shell.keys("secondary-w", cx);
    assert_eq!(tab_count(&shell, cx), 0, "it closes at once");
    let release = released(&shell, TICKET, cx).await;
    assert_released(&shell, &root, TICKET, release, cx);
    drop(reads);
}

/// A session whose save committed keeps its context: the context holds
/// work now, so Done and closing release nothing.
#[gpui_kit::test]
async fn a_session_that_saved_keeps_its_context(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Saved here", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    type_at_end(&shell, &host, "x", cx);
    shell.click("item-save", cx);
    let request = shell.requests(cx, |request| {
        matches!(request.intent, Intent::TicketSave { .. })
    })[0];
    ended(&shell, request, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));
    assert!(!created(&shell, &root, TICKET, cx), "it holds work now");

    done(&shell, cx);
    assert_eq!(kept(&shell, cx), None, "the session ends");
    assert_kept(&shell, &root, TICKET, cx);
    shell.keys("secondary-w", cx);
    assert_eq!(tab_count(&shell, cx), 0);
    assert_kept(&shell, &root, TICKET, cx);
}

/// Done after an Edit of a context made before this run releases nothing,
/// and neither does closing the tab (decision 18).
#[gpui_kit::test]
async fn done_after_an_edit_on_an_existing_context_releases_nothing(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Existing", "");
    });
    shell.fixture.save_ticket(&root, TICKET, "Saved before");
    open(&shell, ticket(&root, TICKET), cx);
    edit(&shell, cx).await;
    assert!(!created(&shell, &root, TICKET, cx), "found, not made");
    done(&shell, cx);
    assert_eq!(kept(&shell, cx), None, "the session ends");
    assert_kept(&shell, &root, TICKET, cx);
    shell.keys("secondary-w", cx);
    assert_eq!(tab_count(&shell, cx), 0);
    assert_kept(&shell, &root, TICKET, cx);
}

/// A draft of the item prevents release: a kept session's, through Done
/// and Keep draft on close; and one stored by another session while this
/// one edits, which the store lists when Done ends the session.
#[gpui_kit::test]
async fn a_kept_draft_prevents_release(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Kept draft", "");
        fixture.commit_ticket(root, OTHER, "Draft elsewhere", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    type_at_end(&shell, &host, "x", cx);
    let id = attached(&shell, &root, TICKET, cx).expect("the draft began");
    done(&shell, cx);
    assert_eq!(kept(&shell, cx), Some(true));
    assert_kept(&shell, &root, TICKET, cx);
    close_keeping(&shell, id, cx);
    assert_eq!(tab_count(&shell, cx), 0);
    assert_kept(&shell, &root, TICKET, cx);
    assert!(created(&shell, &root, TICKET, cx), "still made by this run");

    open(&shell, ticket(&root, OTHER), cx);
    edit(&shell, cx).await;
    let stored = store_draft(&shell.fixture, &root, ItemDtoKind::Ticket, OTHER, "Theirs");
    done(&shell, cx);
    assert_eq!(kept(&shell, cx), None, "the clean session ends");
    assert_kept(&shell, &root, OTHER, cx);
    assert!(created(&shell, &root, OTHER, cx), "still made by this run");
    assert!(stored_body(&shell.fixture.draft_store(), stored).is_some());
}

/// The draft store must answer before anything is released: with crash
/// recovery off, and with a listing that fails, Done after an unused Edit
/// releases nothing, and the context still counts as made here.
#[gpui_kit::test]
async fn an_unavailable_draft_listing_prevents_release(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    // The drafts folder is a file: the store does not open.
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "No store", "");
    fixture.refresh(&root);
    std::fs::write(fixture.data_dir().join("drafts"), b"not a folder").unwrap();
    let shell = Shell::open(fixture, cx);
    open(&shell, ticket(&root, TICKET), cx);
    shell.click("item-edit", cx);
    shell.click("recovery-off-continue", cx);
    until_editing(&shell, cx).await;
    done(&shell, cx);
    assert_eq!(kept(&shell, cx), None, "the session ends");
    assert_kept(&shell, &root, TICKET, cx);
    assert!(created(&shell, &root, TICKET, cx), "still made by this run");

    // A listing that fails.
    let (shell, root) = super::shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Listing fails", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    edit(&shell, cx).await;
    let gates = [
        shell.backend.hold_read(ReadKind::Drafts),
        shell.backend.hold_read(ReadKind::Drafts),
    ];
    done(&shell, cx);
    assert_eq!(kept(&shell, cx), None, "the session ends");
    registry(&shell, cx).update(cx, |registry, cx| {
        let stamp = registry.reload_listing(cx);
        let failed = Err(DraftError {
            kind: DraftErrorKind::Access,
        });
        assert!(registry.apply_listing(stamp, failed));
        cx.notify();
    });
    shell.run(cx);
    assert!(registry(&shell, cx).read_with(cx, |registry, _| matches!(
        registry.listing(),
        ListState::Failed(_)
    )));
    drop(gates);
    assert_kept(&shell, &root, TICKET, cx);
    assert!(created(&shell, &root, TICKET, cx), "still made by this run");
}

/// R-37: a tab closed while its Edit prepares opens nothing, and the
/// context is released when the answer is `ok` with `created: true`. An
/// answer that found the context (`created: false`) and one that answered
/// `discovery_pending` release nothing (Known Limits).
#[gpui_kit::test]
async fn closing_the_tab_while_preparing_releases_the_context_it_made(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Made, then left", "");
        fixture.commit_ticket(root, OTHER, "Pending, then left", "");
        fixture.commit_ticket(root, THIRD, "Found, then left", "");
    });
    shell.fixture.save_ticket(&root, THIRD, "Saved before");

    // Made by this prepare: released once it answers.
    open(&shell, ticket(&root, TICKET), cx);
    let held = shell.backend.hold_job(JobKind::ContextPrepare);
    shell.click("item-edit", cx);
    held.wait_reached();
    shell.keys("secondary-w", cx);
    assert_eq!(tab_count(&shell, cx), 0, "the tab closed");
    held.release();
    let request = prepares(&shell, cx)[0];
    ended(&shell, request, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));
    let release = released(&shell, TICKET, cx).await;
    assert_released(&shell, &root, TICKET, release, cx);
    assert_eq!(tab_count(&shell, cx), 0, "nothing opened");

    // Found there: kept.
    open(&shell, ticket(&root, THIRD), cx);
    let held = shell.backend.hold_job(JobKind::ContextPrepare);
    shell.click("item-edit", cx);
    held.wait_reached();
    shell.keys("secondary-w", cx);
    held.release();
    let request = prepares(&shell, cx)[0];
    ended(&shell, request, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));
    assert_kept(&shell, &root, THIRD, cx);

    // `discovery_pending`: kept.
    failing_at(&shell, FailurePoint::BeforeIndexTransactionCommit);
    open(&shell, ticket(&root, OTHER), cx);
    let held = shell.backend.hold_job(JobKind::ContextPrepare);
    shell.click("item-edit", cx);
    held.wait_reached();
    shell.keys("secondary-w", cx);
    held.release();
    let request = prepares(&shell, cx)[0];
    ended(&shell, request, cx).await;
    assert_eq!(
        code(&shell, request, cx),
        Some(ResultCode::DiscoveryPending)
    );
    assert_kept(&shell, &root, OTHER, cx);
    assert_eq!(all_releases(&shell, cx), 1, "only the context made");
}

/// While a save is in flight, no Discard reaches the draft: the close
/// dialog's Discard waits, and the Drafts page's opens nothing. Nothing is
/// released meanwhile, and once the save commits the context holds work.
#[gpui_kit::test]
async fn discard_is_disabled_while_a_save_is_in_flight(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Saving", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    type_at_end(&shell, &host, "x", cx);
    let id = attached(&shell, &root, TICKET, cx).expect("the draft began");
    let held_save = shell.backend.hold_job(JobKind::TicketSave);
    shell.click("item-save", cx);
    held_save.wait_reached();

    shell.keys("secondary-w", cx);
    assert!(shell.present("close-draft-dialog", cx), "it asks first");
    shell.click("close-draft-discard-button", cx);
    assert!(shell.present("close-draft-dialog", cx), "Discard waits");
    assert_eq!(tab_count(&shell, cx), 1, "the tab stays");
    shell.click("close-draft-cancel-button", cx);
    shell.keys("secondary-shift-d", cx);
    shell.run(cx);
    shell.click(row_id(id, "discard"), cx);
    assert!(
        !shell.present("discard-draft-dialog", cx),
        "it opens nothing"
    );
    assert!(all_releases(&shell, cx) == 0, "nothing is released");

    held_save.release();
    let request = shell.requests(cx, |request| {
        matches!(request.intent, Intent::TicketSave { .. })
    })[0];
    ended(&shell, request, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));
    assert!(!created(&shell, &root, TICKET, cx), "it holds work now");
    assert_kept(&shell, &root, TICKET, cx);
}

/// Condition 3: a save of the item in flight stops a release, even when
/// the store lists no draft of the item. No Discard or close reaches a
/// leave while a save runs (above), so the leave is asked of the release
/// service directly; the draft never began, its folder read-only, and the
/// save is sent from the editor's text.
#[cfg(unix)]
#[gpui_kit::test]
async fn a_save_in_flight_prevents_release(cx: &mut TestAppContext) {
    if superuser() {
        return;
    }
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Saved unstored", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let _read_only = FolderMode::set(&shell.fixture.data_dir().join("drafts"), 0o500);
    type_at_end(&shell, &host, "x", cx);
    assert!(
        attached(&shell, &root, TICKET, cx).is_none(),
        "nothing stored"
    );
    let held_save = shell.backend.hold_job(JobKind::TicketSave);
    shell.click("item-save", cx);
    held_save.wait_reached();

    let item = context_key(&root, TICKET);
    release_service(&shell, cx).update(cx, |releases, cx| releases.leave(item, cx));
    shell.run(cx);
    assert!(
        releases(&shell, TICKET, cx).is_empty(),
        "a save is in flight"
    );
    assert!(created(&shell, &root, TICKET, cx));

    held_save.release();
    let request = shell.requests(cx, |request| {
        matches!(request.intent, Intent::TicketSave { .. })
    })[0];
    ended(&shell, request, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));
    assert!(!created(&shell, &root, TICKET, cx), "it holds work now");
    assert_kept(&shell, &root, TICKET, cx);
}

/// R-78: a release is never sent across a later prepare of its item. Done
/// after an unused Edit waits for the store's listing; an Edit sent
/// meanwhile stops the release, and the new session edits in the context.
#[gpui_kit::test]
async fn an_edit_while_the_store_is_listed_stops_the_release(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Edited again", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    edit(&shell, cx).await;
    let listing = shell.backend.hold_read(ReadKind::Drafts);
    done(&shell, cx);
    assert_eq!(kept(&shell, cx), None, "the session ends");
    shell.click("item-edit", cx);
    let later = prepares(&shell, cx)[0];
    ended(&shell, later, cx).await;
    assert_eq!(code(&shell, later, cx), Some(ResultCode::Ok));

    drop(listing);
    until_editing(&shell, cx).await;
    assert_kept(&shell, &root, TICKET, cx);
    assert!(created(&shell, &root, TICKET, cx), "still made by this run");
    assert_eq!(phase(&shell, cx), Phase::Editing);
}

/// R-40: an Edit refused with `recovery_required` while the read shows no
/// context offers "Release stranded work context". The design's row: a
/// prepare interrupted before its worktree, its row abandoned, and primary
/// moved on, leaves a branch that refuses Edit; the release removes it, and
/// Edit then answers `ok`.
#[gpui_kit::test]
async fn a_stranded_context_can_be_released_from_the_tab(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Stranded", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    failing_at(&shell, FailurePoint::BeforeWorktreeCreation);
    shell.click("item-edit", cx);
    let lost = prepares(&shell, cx)[0];
    ended(&shell, lost, cx).await;
    assert_eq!(code(&shell, lost, cx), Some(ResultCode::InternalError));
    assert_eq!(context_of(&root, TICKET), (false, true), "a branch only");
    assert!(
        !shell.present("item-release-stranded", cx),
        "not offered yet"
    );

    // The row is abandoned, and primary moves on.
    let service = shell.fixture.service();
    let repo = service.resolve_repository(&root).unwrap();
    let operation = service
        .list_operations(&repo)
        .unwrap()
        .items
        .into_iter()
        .find(|operation| operation.action == OperationAction::PrepareContext)
        .and_then(|operation| operation.operation_id)
        .expect("the interrupted prepare is pending");
    abandon_through(service, &root, &operation);
    shell.fixture.commit_ticket(&root, OTHER, "Moved on", "");
    working(&shell);

    shell.click("item-edit", cx);
    let refused = prepares(&shell, cx)[0];
    ended(&shell, refused, cx).await;
    assert_eq!(
        code(&shell, refused, cx),
        Some(ResultCode::RecoveryRequired)
    );
    assert_eq!(phase(&shell, cx), Phase::Viewing);
    assert_eq!(
        shell.label("item-release-stranded", cx).as_deref(),
        Some(STRANDED)
    );

    shell.click("item-release-stranded", cx);
    let release = released(&shell, TICKET, cx).await;
    assert_eq!(code(&shell, release, cx), Some(ResultCode::Ok));
    assert_eq!(
        context_of(&root, TICKET),
        (false, false),
        "the branch is gone"
    );
    assert!(!shell.present("item-release-stranded", cx));

    let host = edit(&shell, cx).await;
    let again = prepares(&shell, cx)[0];
    assert_eq!(code(&shell, again, cx), Some(ResultCode::Ok));
    assert!(created(&shell, &root, TICKET, cx));
    drop(host);
}

/// R-78 (owner, 2026-10-26): a release of an item is never retried, under
/// its request ID, once a later `context prepare` of the item was sent.
/// The release is interrupted between its worktree and its branch, its
/// record left accepted, so the operation area offers Retry. The item then
/// reads as no tab can edit (its worktree is gone), so the later prepare is
/// sent through the worker, as an Edit sends one. Then the operation area
/// shows Retry unavailable, and neither it nor the worker sends the release
/// again.
#[gpui_kit::test]
async fn a_release_is_not_retried_once_the_item_is_prepared_again(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Released, then edited", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    edit(&shell, cx).await;
    failing_at(&shell, FailurePoint::BeforeContextBranchRelease);
    done(&shell, cx);
    let release = released(&shell, TICKET, cx).await;
    assert_eq!(code(&shell, release, cx), Some(ResultCode::InternalError));
    assert_eq!(
        context_of(&root, TICKET),
        (false, true),
        "the branch is left"
    );
    let control = retry_control(&shell, release, cx);
    assert_eq!(shell.label(control.clone(), cx).as_deref(), Some("Retry"));
    working(&shell);

    shell.workspace.read_with(cx, |workspace, _| {
        workspace.services().worker().submit(
            Intent::ContextPrepare {
                root: root.clone(),
                kind: AuthoringKind::Ticket,
                id: item_id(TICKET),
                intent: ContextPrepareIntent::Edit,
            },
            Lane::User,
            None,
            None,
        )
    });
    shell.run(cx);
    let later = prepares(&shell, cx)[0];
    ended(&shell, later, cx).await;
    assert_eq!(
        shell.label(control.clone(), cx).as_deref(),
        Some(NOT_SENT_AGAIN)
    );
    shell.click(control, cx);
    shell.run(cx);
    assert_eq!(attempt(&shell, release, cx), 1, "not sent again");
    let sent = shell.workspace.read_with(cx, |workspace, _| {
        workspace.services().worker().retry(release, None)
    });
    assert!(!sent, "the worker refuses it");
    shell.run(cx);
    assert_eq!(attempt(&shell, release, cx), 1, "not sent again");
    assert_eq!(releases(&shell, TICKET, cx), [release]);
}

/// R-78: once a release of the item was sent, its earlier prepare is never
/// sent again. The prepare answered `discovery_pending`, and the tab waits
/// for Resume (R-77); then a release of the item is sent (through the
/// release service's own entry, as the tab's stranded release sends one;
/// the prepare's pending row refuses it). Resume sends nothing and the tab
/// is viewing again; the operation area's Retry of the prepare is
/// unavailable, and the worker refuses it. A new Edit is a new request,
/// under a new request ID (which that pending row refuses in turn, until
/// it is abandoned).
#[gpui_kit::test]
async fn a_prepare_is_not_sent_again_once_its_context_is_released(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Prepared, then released", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    failing_at(&shell, FailurePoint::BeforeIndexTransactionCommit);
    shell.click("item-edit", cx);
    let pending = prepares(&shell, cx)[0];
    ended(&shell, pending, cx).await;
    assert_eq!(
        code(&shell, pending, cx),
        Some(ResultCode::DiscoveryPending)
    );
    assert!(
        shell.present("item-prepare-resume", cx),
        "Resume is offered"
    );
    let control = retry_control(&shell, pending, cx);
    assert_eq!(shell.label(control.clone(), cx).as_deref(), Some("Retry"));
    working(&shell);

    let item = context_key(&root, TICKET);
    release_service(&shell, cx).update(cx, |releases, _| releases.stranded(item));
    let release = released(&shell, TICKET, cx).await;
    assert_eq!(
        code(&shell, release, cx),
        Some(ResultCode::RecoveryRequired)
    );

    shell.click("item-prepare-resume", cx);
    shell.run(cx);
    assert_eq!(prepares(&shell, cx), [pending], "nothing new");
    assert_eq!(attempt(&shell, pending, cx), 1, "not sent again");
    assert_eq!(phase(&shell, cx), Phase::Viewing, "viewing again");
    assert_eq!(
        shell.label(control.clone(), cx).as_deref(),
        Some(NOT_SENT_AGAIN)
    );
    shell.click(control, cx);
    shell.run(cx);
    assert_eq!(attempt(&shell, pending, cx), 1, "not sent again");
    let sent = shell.workspace.read_with(cx, |workspace, _| {
        workspace.services().worker().retry(pending, None)
    });
    assert!(!sent, "the worker refuses it");
    shell.run(cx);
    assert_eq!(attempt(&shell, pending, cx), 1, "not sent again");

    shell.click("item-edit", cx);
    let again = prepares(&shell, cx);
    assert_eq!(again.len(), 2, "a new request");
    ended(&shell, again[0], cx).await;
    let ids = shell.operations(cx).read_with(cx, |operations, _| {
        (
            operations.request(again[0]).unwrap().request_id,
            operations.request(pending).unwrap().request_id,
        )
    });
    assert_ne!(ids.0, ids.1, "under a new request ID");
    assert_eq!(
        attempt(&shell, pending, cx),
        1,
        "the first is not sent again"
    );
}

fn phase(shell: &Shell, cx: &mut TestAppContext) -> Phase {
    pane(shell, cx).read_with(cx, |pane, _| pane.edit_phase())
}
