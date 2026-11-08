//! Nothing lifecycle without consent (A17; Cycle contract 1), over real
//! local-only repositories: navigation, Save, Done, closing a tab or the
//! window, and quit never start a close, an approve and merge, a resume, a
//! republish, a synchronization or a resolution, after a restart too. Each
//! test asserts that every request made is one of those navigation, Save
//! and Done send (`consented`, an allowlist: the ticket's save, its working
//! copy prepared for Edit, the index refreshed at launch), so a `TicketClose`,
//! `DocumentPromote`, `OperationResume`, `ItemRepublish`, `ItemSync`,
//! `RepoSync`, `LocalContextSync`, `LocalContextResolve` or
//! `ConflictResolve` fails it; and, on the repositories, that no lifecycle
//! commit appears: main is unchanged, the items' context branches gain only
//! what a Save commits, the ticket is not closed, the library's rows stay
//! as they were and a kept resolution set stays kept.

use std::path::Path;

use gpui_kit::{Entity, TestAppContext};
use manyhands::{
    repository::{
        CancellationToken, ClosureState, FailurePoint, ItemDtoKind, LocalConflictResolutionInput,
        LocalContextResolveInput, Mutation, MutationCall, NoProgress, OperationAction,
        RepositoryService, RequestId, RequestState as LibraryState, keys::SessionCredentials,
    },
    results::ResultCode,
};

use super::{
    lock_tests::step,
    tests::{
        DOCUMENT, TICKET, closes, code, commits, edit, open, saved_ticket, shown, strand_close,
        ticket, until_present,
    },
};
use crate::desktop::{
    model::{navigation::OpenTarget, operations::Operations},
    services::{
        conflicts::ConflictSource,
        requests::{Intent, Lane, PathResolution, RequestState},
        resolutions::KeptSet,
    },
    test_support::fixtures::{Fixture, NoPassphrase, item_id},
    view::{
        harness::{Shell, is_save, key},
        item::LIFECYCLE_ACTION,
        workspace::QuitPhase,
    },
};

/// Whether `intent` is one that these tests' navigation, Save and Done
/// send, as they were found to: the ticket's save, its working copy
/// prepared for Edit, and the index refreshed at launch.
fn consented(intent: &Intent) -> bool {
    matches!(
        intent,
        Intent::TicketSave { .. } | Intent::ContextPrepare { .. } | Intent::IndexRefresh { .. }
    )
}

/// The requests `operations` ever held that are not `consented`, by label.
fn unconsented(operations: &Entity<Operations>, cx: &mut TestAppContext) -> Vec<&'static str> {
    operations.read_with(cx, |operations, _| {
        operations
            .requests()
            .iter()
            .filter(|request| !consented(&request.intent))
            .map(|request| request.intent.label())
            .collect()
    })
}

/// Every request `operations` ever held, by label.
fn sent(operations: &Entity<Operations>, cx: &mut TestAppContext) -> Vec<&'static str> {
    operations.read_with(cx, |operations, _| {
        operations
            .requests()
            .iter()
            .map(|request| request.intent.label())
            .collect()
    })
}

/// Waits until no request is queued or running, such as the launch's
/// index refresh, so quit has no job to wait for.
async fn until_idle(shell: &Shell, cx: &mut TestAppContext) {
    shell
        .until(cx, |operations| {
            operations.requests().iter().all(|request| {
                !matches!(
                    request.state,
                    RequestState::Queued | RequestState::Running { .. }
                )
            })
        })
        .await;
}

/// The lifecycle rows the library lists for `root`: closes and promotions.
fn lifecycle_rows(fixture: &Fixture, root: &Path) -> usize {
    let service = fixture.service();
    let repo = service.resolve_repository(root).unwrap();
    service
        .list_operations(&repo)
        .unwrap()
        .items
        .into_iter()
        .filter(|row| {
            matches!(
                row.action,
                OperationAction::Close | OperationAction::Promote
            )
        })
        .count()
}

fn document(root: &Path) -> OpenTarget {
    OpenTarget::Item {
        repo: key(root),
        kind: ItemDtoKind::Document,
        id: item_id(DOCUMENT),
    }
}

/// Commits on main and on the ticket's and the document's context
/// branches.
fn counts(shell: &Shell, root: &Path) -> (usize, usize, usize) {
    let branch = |id: &str| shown(shell, root, id).context.branch.expect("its own");
    (
        commits(root, "main"),
        commits(root, &branch(TICKET)),
        commits(root, &branch(DOCUMENT)),
    )
}

/// One repository with a ticket and a document, each with a working copy
/// of its own, in an open window.
fn two_items(cx: &mut TestAppContext) -> (Shell, std::path::PathBuf) {
    cx.executor().allow_parking();
    super::tests::shell(cx, |fixture, root| {
        saved_ticket(fixture, root, "Consent", "");
        fixture.create_document(root, DOCUMENT, "docs/plan.md", "Plan");
    })
}

/// A17, real: opening the ticket and the document, selecting the
/// repository, moving focus about, Edit, a Save, Done, closing both tabs,
/// quit and closing the window send no lifecycle request; main and the
/// document's branch are unchanged, the ticket's branch gains the Save's
/// one commit, the ticket stays open and no lifecycle row is listed.
#[gpui_kit::test]
async fn a17_navigation_save_done_close_and_quit_start_no_lifecycle(cx: &mut TestAppContext) {
    let (shell, root) = two_items(cx);
    let (main, ticket_branch, document_branch) = counts(&shell, &root);
    let operations = shell.operations(cx);

    // Navigation.
    open(&shell, ticket(&root), cx);
    open(&shell, document(&root), cx);
    shell.select(&root, cx);
    open(&shell, ticket(&root), cx);
    shell.keys("tab tab tab escape", cx);
    assert!(shell.present(LIFECYCLE_ACTION, cx), "the action is offered");

    // Edit, Save, then Done.
    let host = edit(&shell, cx).await;
    shell.type_at_end(&host, " more", cx);
    shell.click("item-save", cx);
    let save = shell.requests(cx, |request| is_save(&request.intent))[0];
    shell.until_ended(save, cx).await;
    assert_eq!(code(&shell, save, cx), Some(ResultCode::Ok));
    shell.click("item-done", cx);
    assert_eq!(
        unconsented(&operations, cx),
        Vec::<&str>::new(),
        "sent: {:?}",
        sent(&operations, cx)
    );

    // Closing both tabs.
    shell.keys("secondary-w", cx);
    open(&shell, document(&root), cx);
    shell.keys("secondary-w", cx);
    let tabs = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.tab_keys().len());
    assert_eq!(tabs, 0, "both tabs closed");

    // Quit, then the window's close.
    shell.keys("secondary-q", cx);
    let phase = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.quit_phase());
    assert_eq!(phase, QuitPhase::Quit);
    assert!(shell.visual(cx).simulate_close(), "the window may close");
    shell.run(cx);

    assert_eq!(
        unconsented(&operations, cx),
        Vec::<&str>::new(),
        "sent: {:?}",
        sent(&operations, cx)
    );
    assert_eq!(
        counts(&shell, &root),
        (main, ticket_branch + 1, document_branch),
        "only the Save's commit"
    );
    let closure = shown(&shell, &root, TICKET)
        .closure
        .expect("a ticket's closure");
    assert_ne!(closure.state, ClosureState::Closed, "the ticket stays open");
    assert_eq!(lifecycle_rows(&shell.fixture, &root), 0);
}

/// A17, real: the lifecycle dialog at its confirm stage runs nothing until
/// Confirm. Escape cancels it; opened again and left at Confirm, the
/// window's close request and quit send no lifecycle request, and nothing
/// is committed or listed.
#[gpui_kit::test]
async fn a17_a_dialog_left_at_confirm_starts_nothing(cx: &mut TestAppContext) {
    let (shell, root) = two_items(cx);
    let before = counts(&shell, &root);
    let operations = shell.operations(cx);

    open(&shell, ticket(&root), cx);
    shell.click(LIFECYCLE_ACTION, cx);
    until_present(&shell, "lifecycle-confirm", cx).await;
    shell.keys("escape", cx);
    assert!(!shell.present("lifecycle-dialog", cx));
    assert!(closes(&shell, cx).is_empty());

    open(&shell, document(&root), cx);
    shell.click(LIFECYCLE_ACTION, cx);
    until_present(&shell, "lifecycle-confirm", cx).await;
    assert!(shell.visual(cx).simulate_close(), "the window may close");
    shell.run(cx);
    shell.keys("secondary-q", cx);
    shell.run(cx);

    assert_eq!(
        unconsented(&operations, cx),
        Vec::<&str>::new(),
        "sent: {:?}",
        sent(&operations, cx)
    );
    assert_eq!(counts(&shell, &root), before, "nothing committed");
    assert_eq!(lifecycle_rows(&shell.fixture, &root), 0);
}

/// A17, real: a close another process stranded at `closing` (its request
/// accepted, its row offering Resume) is not resumed by a launch that
/// refreshes, by opening the ticket and moving about, by closing its tab or
/// by quitting: no request is made, its row stays at `closing`, its
/// request stays accepted, and main is unchanged.
#[gpui_kit::test]
async fn a17_a_stranded_close_is_not_resumed_without_consent(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    saved_ticket(&fixture, &root, "Stranded", "");
    fixture.refresh(&root);
    let holder = strand_close(&fixture, &root);
    let main = commits(&root, "main");
    let shell = Shell::launch(fixture, |_| {}, cx);
    let operations = shell.operations(cx);
    let wanted = root.clone();
    shell
        .until(cx, move |operations| {
            operations.requests().iter().any(|request| {
                request.lane == Lane::Background
                    && matches!(&request.intent, Intent::IndexRefresh { root } if *root == wanted)
                    && matches!(request.state, RequestState::Finished(..))
            })
        })
        .await;
    assert_eq!(step(&shell, &root).as_deref(), Some("closing"));

    open(&shell, ticket(&root), cx);
    shell
        .until_drawn(cx, "the close's row", |shell, cx| {
            shell
                .rows("pending-command", cx)
                .contains(&OperationAction::Close.as_str().to_owned())
        })
        .await;
    shell.select(&root, cx);
    shell.keys("tab tab tab escape", cx);
    shell.keys("secondary-w", cx);
    until_idle(&shell, cx).await;
    shell.keys("secondary-q", cx);
    assert!(shell.visual(cx).simulate_close(), "the window may close");
    shell.run(cx);

    assert_eq!(
        unconsented(&operations, cx),
        Vec::<&str>::new(),
        "sent: {:?}",
        sent(&operations, cx)
    );
    assert_eq!(step(&shell, &root).as_deref(), Some("closing"));
    let recorded = shell
        .fixture
        .service()
        .show_request(holder)
        .data
        .expect("the request is recorded")
        .state;
    assert_eq!(recorded, LibraryState::Accepted);
    assert_eq!(commits(&root, "main"), main, "nothing merged");
}

/// A17, real, after a restart: a local synchronization of the ticket
/// stopped on its conflict (`Fixture::conflict_locally`), with a
/// resolution kept for it (`ResolutionStore::keep`) that a session which
/// ended had sent and the library broke after writing its path
/// (`FailurePoint::LocalSyncAfterResolutionPath`, the recipe of
/// `conflict/answer_tests.rs`
/// `reopening_after_a_partial_local_resolution_offers_retry_not_new`). A
/// fresh shell that refreshes at launch, opening the ticket and moving
/// about, closing its tab, quit and closing the window send nothing
/// outside the allowlist: no synchronization, resolution or resume. The
/// synchronization's row still offers Resume, the library still lists it,
/// and the kept set is still stored.
#[gpui_kit::test]
async fn a17_a_conflict_waiting_after_a_restart_is_not_resumed_or_resolved(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let operation = fixture.conflict_locally(&root, TICKET, "Mine", "Theirs");
    let (inspection, contents) = fixture.read_local_conflict(&root, operation);
    let set = KeptSet {
        request_id: RequestId::new(),
        root: root.clone(),
        operation_id: operation,
        item: item_id(TICKET),
        source: ConflictSource::Local,
        observation: inspection.observation.clone().unwrap(),
        resolutions: vec![PathResolution {
            token: contents[0].token.clone(),
            source: contents[0].local.clone().unwrap(),
        }],
    };
    fixture.resolution_store().keep(&set).unwrap();
    let failing = RepositoryService::open_at_with_failure_point_for_testing(
        fixture.data_dir(),
        FailurePoint::LocalSyncAfterResolutionPath,
    )
    .unwrap();
    let stopped = failing.execute(
        MutationCall {
            request_id: set.request_id,
            confirmation: None,
            mutation: Mutation::LocalContextResolve(LocalContextResolveInput {
                root: root.clone(),
                operation_id: operation,
                observation: set.observation.clone(),
                resolutions: vec![LocalConflictResolutionInput {
                    token: set.resolutions[0].token.clone(),
                    source: set.resolutions[0].source.clone(),
                }],
            }),
        },
        &mut SessionCredentials::new(NoPassphrase),
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_ne!(stopped.code, ResultCode::Ok);
    drop(failing);
    let main = commits(&root, "main");

    let shell = Shell::launch(fixture, |_| {}, cx);
    let operations = shell.operations(cx);
    let sync = OperationAction::ContextSyncLocal.as_str().to_owned();
    let row = |shell: &Shell, cx: &mut TestAppContext| {
        shell
            .rows("pending-command", cx)
            .iter()
            .position(|listed| *listed == sync)
    };
    shell
        .until_drawn(cx, "the synchronization's Resume", |shell, cx| {
            row(shell, cx).is_some_and(|at| shell.present(("pending-resume", at), cx))
        })
        .await;

    open(&shell, ticket(&root), cx);
    shell.select(&root, cx);
    shell.keys("tab tab tab escape", cx);
    shell.run(cx);
    let at = row(&shell, cx).expect("still listed");
    assert!(shell.present(("pending-resume", at), cx), "still offered");
    shell.keys("secondary-w", cx);
    until_idle(&shell, cx).await;
    shell.keys("secondary-q", cx);
    assert!(shell.visual(cx).simulate_close(), "the window may close");
    shell.run(cx);

    assert_eq!(
        unconsented(&operations, cx),
        Vec::<&str>::new(),
        "sent: {:?}",
        sent(&operations, cx)
    );
    let service = shell.fixture.service();
    let repo = service.resolve_repository(&root).unwrap();
    assert!(
        service
            .list_operations(&repo)
            .unwrap()
            .items
            .iter()
            .any(|row| row.action == OperationAction::ContextSyncLocal),
        "the synchronization is still pending"
    );
    let kept = shell
        .fixture
        .resolution_store()
        .kept(set.request_id)
        .unwrap();
    assert!(kept.is_some(), "the kept set is still stored");
    assert_eq!(commits(&root, "main"), main, "nothing merged");
}
