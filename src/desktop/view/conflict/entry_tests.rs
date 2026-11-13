//! The resolve conflict sheet's entry points (D5 design, B5, B6; decision
//! 16): Review changes on the conflict banner, on the operation area's
//! entry of a conflicted share, post, local synchronization or lifecycle,
//! and on a pending row whose conflict waits to be resolved, after a
//! restart too; Resolve conflict… in the lifecycle dialog (L1); and Resume
//! on a local synchronization (E-5). Repair locally… is
//! `conflict/repair_tests.rs`, which shares these helpers.
//!
//! Local cases are real: a local synchronization's conflict
//! (`Fixture::conflict_locally`). A remote answer
//! and a remote conflict are canned (decision 9), each built from library
//! types and naming the library test that produces the real one; nothing is
//! asserted of the repository in a canned test.

use std::path::{Path, PathBuf};

use gpui_kit::{Entity, TestAppContext};
use manyhands::{
    repository::{
        AuthoringKind, ConflictNextAction, ConflictNextDto, ItemDtoKind, LifecycleBlocker,
        LifecycleMutationDto, LifecycleMutationType, MutationDataDto, OperationAction,
        OperationDto, OperationFamily, OperationId, OperationNextAction, OperationOwner, RequestId,
        SynchronizationConflictMutationDto, SynchronizationConflictType,
    },
    results::{
        Effects, Envelope, Outcome, PublicationEffect, RecoveryAction, RecoveryActionKind,
        ResultCode, Scope,
    },
};

use super::{
    REVIEW_CHANGES, ResolveSheet, SheetSource,
    answer_tests::applied,
    tests::{
        TICKET, canned, canned_path, local_shell, local_shell_on, open_sheet, opened, until_present,
    },
};
use crate::desktop::{
    model::{
        activity::rows::{ActivityAction, RowKey},
        navigation::OpenTarget,
        operations::{CONFLICT_BANNER, ResumeOffer, resume_offer},
    },
    services::{
        JobKind,
        requests::{DesktopRequest, DesktopRequestId, Intent, Lane, RequestState},
        worker::WorkerEvent,
    },
    test_support::fixtures::{Fixture, item_id},
    view::{
        activity_support::{act, offered, pending_key, refresh, row, until_offered},
        harness::{Shell, key},
        item::OpenedFolders,
        lifecycle::RESOLVE_CONFLICT,
        operations::{DISMISS, OPEN_LIFECYCLE, RESUME},
    },
};

/// The synchronization a canned conflict answer names.
const SYNCHRONIZATION: &str = "01M4CC0VN1KTQFSVR99378TDHV";
/// Fabricated requests stand far above the worker's own numbering.
const FABRICATED: u64 = 4_000_000;

/// A ticket committed on main and saved once, so it has a work context of
/// its own, in repository `a`, in an open window with its tab open.
pub(super) fn ticket_shell(cx: &mut TestAppContext) -> (Shell, PathBuf) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "Untitled", "");
    fixture.refresh(&root);
    fixture.save_ticket(&root, TICKET, "Mine");
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    shell.workspace.update(cx, |workspace, cx| {
        workspace.open_item(
            OpenTarget::Item {
                repo: key(&root),
                kind: ItemDtoKind::Ticket,
                id: item_id(TICKET),
            },
            cx,
        )
    });
    shell.run(cx);
    (shell, root)
}

/// The request `n` of `intent`, fabricated, queued, started and finished
/// with `envelope`.
pub(crate) fn fabricate(
    shell: &Shell,
    n: u64,
    intent: Intent,
    envelope: Envelope<MutationDataDto>,
    cx: &mut TestAppContext,
) -> DesktopRequestId {
    let id = DesktopRequestId(FABRICATED + n);
    let request = DesktopRequest {
        id,
        request_id: RequestId::new(),
        intent,
        confirmation: None,
        lane: Lane::User,
        attempt: 1,
        state: RequestState::Queued,
    };
    shell.operations(cx).update(cx, |operations, cx| {
        operations.apply(WorkerEvent::Queued(request));
        operations.apply(WorkerEvent::Started(id));
        operations.apply(WorkerEvent::Finished(id, Box::new(envelope)));
        cx.notify();
    });
    shell.run(cx);
    id
}

/// A share stopped on a conflict of `SYNCHRONIZATION`, as
/// `tests/mutation_remote/merging.rs` `c4_merging_canonical_conflict`
/// answers.
pub(crate) fn share_conflict() -> Envelope<MutationDataDto> {
    Envelope::classified_mutation(
        "item sync",
        Scope::default(),
        false,
        ResultCode::RecoveryRequired,
        Effects::not_requested(),
        false,
    )
    .with_data(MutationDataDto::SynchronizationConflict(
        SynchronizationConflictMutationDto {
            r#type: SynchronizationConflictType::SynchronizationConflict,
            conflict: ConflictNextDto {
                operation_id: SYNCHRONIZATION.to_owned(),
                next_action: ConflictNextAction::Resolve,
            },
        },
    ))
}

/// A close stopped on its context child's conflict, named by the close's
/// operation `parent`, with the same request to send again, as
/// `tests/mutation_remote/c5_child_conflict.rs` `shown_by_either_id`
/// answers (its `ticket_close` way out is `way_out_tests.rs`'s
/// `ResolveChildConflict` then `SendAgain`).
fn child_conflict(root: &Path, parent: &str, request: RequestId) -> Envelope<MutationDataDto> {
    let mut envelope = Envelope::<MutationDataDto>::classified_mutation(
        "ticket close",
        Scope {
            repository: Some(key(root)),
            item_id: Some(TICKET.to_owned()),
            ..Scope::default()
        },
        false,
        ResultCode::RecoveryRequired,
        Effects::not_requested(),
        false,
    );
    envelope.outcome = Outcome::Partial;
    envelope.recovery = vec![RecoveryAction::new(
        RecoveryActionKind::RequestRetry,
        [("request_id", request.to_string().into())],
    )];
    envelope.request_id = Some(request.to_string());
    envelope.operation_id = Some(parent.to_owned());
    envelope.data = Some(MutationDataDto::Lifecycle(Box::new(LifecycleMutationDto {
        r#type: LifecycleMutationType::Lifecycle,
        kind: ItemDtoKind::Ticket,
        item_id: TICKET.to_owned(),
        recording: None,
        checkpoint_oid: None,
        integration: None,
        integration_oid: None,
        publication: None,
        publication_oid: None,
        publication_pending: None,
        cleanup: None,
        cleanup_pending: None,
        blockers: vec![
            LifecycleBlocker::ContextConflicted,
            LifecycleBlocker::ConflictPending,
        ],
        made_ready: None,
        made_ready_state: None,
    })));
    envelope
}

/// What the sheet `sheet` was opened on: its source, operation and folder.
pub(super) fn opened_on(
    sheet: &Entity<ResolveSheet>,
    cx: &mut TestAppContext,
) -> (SheetSource, OperationId, PathBuf) {
    sheet.read_with(cx, |sheet, _| {
        (sheet.source(), sheet.operation(), sheet.folder().to_owned())
    })
}

/// The pending row of `root` whose command is `command`, once listed: its
/// key, for the seam.
pub(super) async fn row_of(
    shell: &Shell,
    root: &Path,
    command: &str,
    cx: &mut TestAppContext,
) -> RowKey {
    let root = key(root);
    shell
        .until_drawn(cx, "the pending row", |shell, cx| {
            shell.operations(cx).read_with(cx, |operations, _| {
                operations
                    .pending_shown()
                    .into_iter()
                    .filter(|(listed, _)| *listed == root)
                    .filter_map(|(_, state)| state.rows().map(|list| list.items.clone()))
                    .flatten()
                    .any(|operation| operation.action.as_str() == command)
            })
        })
        .await;
    pending_key(shell, &root, command, cx)
}

/// B5, E-7, canned (decision 9): a share of the item stopped on a conflict
/// raises the conflict banner, which says the work is safe and offers
/// Review changes; so does the share's operation area entry. Pressed, it
/// opens the sheet on the share's synchronization, naming the item's work
/// context as its folder. Share main's conflict, which names no item, is
/// not offered. The answer is `share_conflict`'s; the conflict read is
/// `c4_conflict_read_canonical`'s shape (`canned`).
#[gpui_kit::test]
async fn the_banner_offers_review_changes(cx: &mut TestAppContext) {
    let (shell, root) = ticket_shell(cx);
    let operation = OperationId::parse(SYNCHRONIZATION).unwrap();
    let share = fabricate(
        &shell,
        1,
        Intent::ItemSync {
            root: root.clone(),
            kind: AuthoringKind::Ticket,
            id: item_id(TICKET),
        },
        share_conflict(),
        cx,
    );
    let entry = RowKey::Request(share);
    until_present(&shell, "item-conflict", cx).await;
    assert_eq!(
        shell.label("item-conflict", cx).as_deref(),
        Some(CONFLICT_BANNER)
    );
    assert_eq!(
        shell.label("item-review-changes", cx).as_deref(),
        Some(REVIEW_CHANGES)
    );
    until_offered(&shell, &entry, REVIEW_CHANGES, cx).await;
    assert_eq!(
        offered(&shell, &entry, REVIEW_CHANGES, cx).map(|offer| offer.action),
        Some(ActivityAction::Review(share))
    );

    shell
        .backend
        .answer_next_conflict_read(canned(operation, vec![canned_path(TICKET, '1')]));
    shell.click("item-review-changes", cx);
    until_present(&shell, "conflict-save", cx).await;
    let sheet = opened(cx).expect("the sheet is open");
    let worktree = shell.fixture.context_worktree(&root, TICKET);
    assert_eq!(
        opened_on(&sheet, cx),
        (SheetSource::Share, operation, worktree.clone())
    );
    assert_eq!(shell.label("conflict-repository", cx).as_deref(), Some("a"));

    shell.keys("escape", cx);
    assert!(!shell.present("conflict-sheet", cx));
    shell
        .backend
        .answer_next_conflict_read(canned(operation, vec![canned_path(TICKET, '1')]));
    act(&shell, &entry, REVIEW_CHANGES, cx);
    until_present(&shell, "conflict-save", cx).await;
    let sheet = opened(cx).expect("the sheet is open again");
    assert_eq!(
        opened_on(&sheet, cx),
        (SheetSource::Share, operation, worktree)
    );
    shell.keys("escape", cx);

    // Share main's conflict names no item, which the sheet needs: its
    // entry offers no Review changes.
    let main = fabricate(
        &shell,
        3,
        Intent::RepoSync { root: root.clone() },
        share_conflict(),
        cx,
    );
    let entry = RowKey::Request(main);
    refresh(&shell, cx);
    assert!(row(&shell, &entry, cx).is_some(), "the entry is listed");
    assert!(offered(&shell, &entry, REVIEW_CHANGES, cx).is_none());
}

/// B5, real: after a restart this session holds nothing of the local
/// synchronization that stopped on a conflict, and its pending row, whose
/// conflict waits to be resolved, offers Review changes. Pressed, it opens
/// the sheet on that synchronization, its folder the item's work context.
/// The repository's primary branch is `trunk`: the sheet's note names it,
/// as navigation lists it (R-35).
#[gpui_kit::test]
async fn a_pending_resolve_row_offers_review_changes_after_restart(cx: &mut TestAppContext) {
    let (shell, root, operation) = local_shell_on(cx, "trunk", |fixture, root| {
        fixture.conflict_locally(root, TICKET, "Mine", "Theirs")
    });
    let pending = row_of(&shell, &root, "context_sync_local", cx).await;
    until_offered(&shell, &pending, REVIEW_CHANGES, cx).await;
    assert!(
        matches!(
            offered(&shell, &pending, REVIEW_CHANGES, cx).map(|offer| offer.action),
            Some(ActivityAction::ReviewPending { .. })
        ),
        "the pending row's own Review changes"
    );

    act(&shell, &pending, REVIEW_CHANGES, cx);
    until_present(&shell, "conflict-save", cx).await;
    let sheet = opened(cx).expect("the sheet is open");
    assert_eq!(
        opened_on(&sheet, cx),
        (
            SheetSource::Local,
            operation,
            shell.fixture.context_worktree(&root, TICKET)
        )
    );
    assert_eq!(
        shell.label("conflict-note", cx).as_deref(),
        Some("Saving finishes bringing trunk into this item.")
    );
}

/// E-5, B6: a local synchronization waiting to be resumed offers Resume:
/// a local-context or primary synchronization with no remote, as the
/// library lists it (built here: no local recipe leaves one pending), and,
/// real, a stopped `context sync-local`, whose Resume sends `operation
/// resume` of it.
#[gpui_kit::test]
async fn a_local_synchronization_offers_resume(cx: &mut TestAppContext) {
    for action in [
        OperationAction::SynchronizeContext,
        OperationAction::SynchronizePrimary,
    ] {
        let row = OperationDto {
            operation_id: Some("01ARZ3NDEKTSV4RRFFQ69G5FY1".to_owned()),
            family: OperationFamily::Local,
            owner: OperationOwner::Repository,
            action,
            state: "created".to_owned(),
            completed_step: None,
            next_action: Some(OperationNextAction::Resume),
            item_id: None,
            key_id: None,
            worktree: None,
            updated_at: None,
            failure_code: None,
        };
        assert_eq!(resume_offer(&row), ResumeOffer::Offered, "{action:?}");
    }

    let (shell, root, operation) = local_shell(cx, |fixture, root| {
        fixture.conflict_locally(root, TICKET, "Mine", "Theirs")
    });
    let pending = row_of(&shell, &root, "context_sync_local", cx).await;
    refresh(&shell, cx);
    // Offered and enabled, not the unavailable Resume.
    assert!(
        offered(&shell, &pending, RESUME, cx).is_some_and(
            |offer| offer.enabled && matches!(offer.action, ActivityAction::Resume { .. })
        ),
        "Resume is offered"
    );
    act(&shell, &pending, RESUME, cx);
    let resumed = shell.requests(cx, move |request| {
        matches!(request.intent, Intent::OperationResume { operation_id, .. }
            if operation_id == operation)
    });
    assert_eq!(resumed.len(), 1);
    shell.until_ended(resumed[0], cx).await;
}

/// B5, P8, canned (decision 9): a close stopped on its context child's
/// conflict (`child_conflict`) offers Review changes on its entry, and its
/// dialog offers Resolve conflict… beside its guidance. Pressed, the sheet
/// opens on the operation the way out names, as the close's child, this
/// session holding the close; its folder is the item's work context, read
/// since the answer names none. The conflict read is `canned`'s shape.
#[gpui_kit::test]
async fn resolve_conflict_opens_the_childs_sheet(cx: &mut TestAppContext) {
    let (shell, root) = ticket_shell(cx);
    let parent = "01ARZ3NDEKTSV4RRFFQ69G5FY1";
    let close = fabricate(
        &shell,
        2,
        Intent::TicketClose {
            root: root.clone(),
            id: item_id(TICKET),
        },
        child_conflict(&root, parent, RequestId::new()),
        cx,
    );
    let entry = RowKey::Request(close);
    until_offered(&shell, &entry, REVIEW_CHANGES, cx).await;
    assert_eq!(
        offered(&shell, &entry, REVIEW_CHANGES, cx).map(|offer| offer.action),
        Some(ActivityAction::Review(close))
    );
    act(&shell, &entry, OPEN_LIFECYCLE, cx);
    until_present(&shell, "lifecycle-resolve-conflict", cx).await;
    assert_eq!(
        shell.label("lifecycle-resolve-conflict", cx).as_deref(),
        Some(RESOLVE_CONFLICT)
    );
    assert!(!shell.rows("lifecycle-way-out", cx).is_empty(), "guidance");

    let operation = OperationId::parse(parent).unwrap();
    shell
        .backend
        .answer_next_conflict_read(canned(operation, vec![canned_path(TICKET, '1')]));
    shell.click("lifecycle-resolve-conflict", cx);
    until_present(&shell, "conflict-save", cx).await;
    assert!(!shell.present("lifecycle-dialog", cx));
    let worktree = shell.fixture.context_worktree(&root, TICKET);
    let wanted = worktree.clone();
    shell
        .until_drawn(cx, "the folder read", move |_, cx| {
            opened(cx).is_some_and(|sheet| {
                sheet.read_with(cx, |sheet, _| sheet.folder() == wanted.as_path())
            })
        })
        .await;
    let sheet = opened(cx).expect("the sheet is open");
    let (source, shown, folder) = opened_on(&sheet, cx);
    assert_eq!(source, SheetSource::Close(Some(close)));
    assert_eq!(shown, operation);
    assert_eq!(folder, worktree);
}

/// Task 13: a local conflict's Open folder opens the item's work context,
/// not the repository's, when the sheet is opened with no folder named: it
/// reads the item's. Real: an untracked file in the work context sends the
/// set outside (R-26).
#[gpui_kit::test]
async fn a_local_conflicts_open_folder_opens_its_work_context(cx: &mut TestAppContext) {
    let (shell, root, operation) = local_shell(cx, |fixture, root| {
        let operation = fixture.conflict_locally(root, TICKET, "Mine", "Theirs");
        let worktree = fixture.context_worktree(root, TICKET);
        std::fs::write(worktree.join("untracked.txt"), b"x").unwrap();
        operation
    });
    let worktree = shell.fixture.context_worktree(&root, TICKET);
    open_sheet(&shell, SheetSource::Local, &root, operation, cx);
    until_present(&shell, "conflict-open-folder", cx).await;
    // M5: the folder read has landed.
    let wanted = worktree.clone();
    shell
        .until_drawn(cx, "the folder read", move |_, cx| {
            opened(cx).is_some_and(|sheet| {
                sheet.read_with(cx, |sheet, _| sheet.folder() == wanted.as_path())
            })
        })
        .await;
    shell.click("conflict-open-folder", cx);
    let opened = cx.update(|cx| {
        cx.try_global::<OpenedFolders>()
            .map(|opened| opened.0.clone())
            .unwrap_or_default()
    });
    assert_eq!(opened, [worktree]);
}

/// I-1, B3 ("applied, share or local: Banner clears"), canned (decision
/// 9): a share stopped on a conflict (`share_conflict`) is resolved
/// through the banner's Review changes; once the resolution applies
/// (`answer_tests::applied`, as `tests/mutation_remote/resolution.rs`
/// `c4_resolution_publishes` answers), the banner, its Review changes and
/// the share's entry's Review changes are gone. A dismissed entry offers
/// its conflict no more either.
#[gpui_kit::test]
async fn an_applied_resolution_clears_the_banner_and_its_review(cx: &mut TestAppContext) {
    let (shell, root) = ticket_shell(cx);
    let operation = OperationId::parse(SYNCHRONIZATION).unwrap();
    let sync = || Intent::ItemSync {
        root: root.clone(),
        kind: AuthoringKind::Ticket,
        id: item_id(TICKET),
    };
    let share = fabricate(&shell, 4, sync(), share_conflict(), cx);
    let entry = RowKey::Request(share);
    until_present(&shell, "item-review-changes", cx).await;
    until_offered(&shell, &entry, REVIEW_CHANGES, cx).await;
    shell
        .backend
        .answer_next_conflict_read(canned(operation, vec![canned_path(TICKET, '1')]));
    shell.click("item-review-changes", cx);
    until_present(&shell, "conflict-save", cx).await;
    shell.click(("conflict-keep-mine", 0usize), cx);
    shell
        .backend
        .hold_job_return_answering(JobKind::ConflictResolve, applied())
        .release();
    shell.click("conflict-save", cx);
    let resolve = shell.requests(cx, |request| {
        matches!(request.intent, Intent::ConflictResolve { .. })
    });
    shell.until_ended(resolve[0], cx).await;
    shell
        .until_drawn(cx, "the banner's clearing", |shell, cx| {
            !shell.present("item-conflict", cx)
        })
        .await;
    assert!(!shell.present("item-review-changes", cx));
    refresh(&shell, cx);
    assert!(row(&shell, &entry, cx).is_some(), "the entry stays listed");
    assert!(offered(&shell, &entry, REVIEW_CHANGES, cx).is_none());

    // Dismissed, an entry offers its conflict no more.
    let again = fabricate(&shell, 5, sync(), share_conflict(), cx);
    let entry = RowKey::Request(again);
    until_offered(&shell, &entry, REVIEW_CHANGES, cx).await;
    until_present(&shell, "item-review-changes", cx).await;
    act(&shell, &entry, DISMISS, cx);
    shell
        .until_drawn(cx, "the banner's Review changes gone", |shell, cx| {
            !shell.present("item-review-changes", cx)
        })
        .await;
    let operations = shell.operations(cx);
    assert!(operations.read_with(cx, |operations, _| operations.review(again).is_none()));
}

/// Re-review 2, B4 then P15, canned (decision 9): a share stopped on a
/// conflict (`share_conflict`) that was repaired outside and finished by
/// Resume of its synchronization, answered ok with no new conflict as
/// `tests/mutation_remote/conflict_read.rs` `c4_conflict_read_repaired`'s
/// restart publishes, offers its conflict no more: the banner, its Review
/// changes and the entry's are gone. A resume of another operation leaves
/// them.
#[gpui_kit::test]
async fn a_resume_that_settles_the_conflict_clears_its_review(cx: &mut TestAppContext) {
    let (shell, root) = ticket_shell(cx);
    let share = fabricate(
        &shell,
        6,
        Intent::ItemSync {
            root: root.clone(),
            kind: AuthoringKind::Ticket,
            id: item_id(TICKET),
        },
        share_conflict(),
        cx,
    );
    until_present(&shell, "item-review-changes", cx).await;
    let resume = |operation: &str| Intent::OperationResume {
        root: root.clone(),
        operation_id: OperationId::parse(operation).unwrap(),
    };
    let published = || {
        Envelope::classified_mutation(
            "operation resume",
            Scope::default(),
            false,
            ResultCode::Ok,
            Effects {
                publication: PublicationEffect::Published,
                ..Effects::not_requested()
            },
            false,
        )
    };
    fabricate(
        &shell,
        7,
        resume("01ARZ3NDEKTSV4RRFFQ69G5FY9"),
        published(),
        cx,
    );
    assert!(
        shell.present("item-review-changes", cx),
        "another operation"
    );
    let entry = RowKey::Request(share);
    refresh(&shell, cx);
    assert!(
        offered(&shell, &entry, REVIEW_CHANGES, cx).is_some_and(|offer| offer.enabled),
        "another operation"
    );

    fabricate(&shell, 8, resume(SYNCHRONIZATION), published(), cx);
    shell
        .until_drawn(cx, "the banner's clearing", |shell, cx| {
            !shell.present("item-conflict", cx)
        })
        .await;
    assert!(!shell.present("item-review-changes", cx));
    refresh(&shell, cx);
    assert!(row(&shell, &entry, cx).is_some(), "the entry stays listed");
    assert!(offered(&shell, &entry, REVIEW_CHANGES, cx).is_none());
}
