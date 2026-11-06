//! D5's conflict journey by keyboard alone (D5 design, A1, A21): the
//! resolve conflict sheet reached from where a conflict is offered, each
//! part chosen, and Save resolution pressed, by Tab, Enter and Escape only:
//! no click and no call into the workspace. After each step the walk
//! asserts where focus is and that it is on screen; in the sheet, that no
//! region holds it.
//!
//! The local conflict is real (`Fixture::conflict_locally`), and its
//! resolution runs, so the test waits up to `RESOLVING`. The remote one is
//! canned (decision 9): the SSH harness does not reach desktop tests. Its
//! share runs for real against an SSH remote whose transport is never
//! initialized, and its resolve against an operation the repository does
//! not hold; library-built answers stand in place of theirs, and nothing
//! is asserted of the repository.

use std::time::{Duration, Instant};

use gpui_kit::{ElementId, TestAppContext};
use manyhands::{
    repository::{
        ConflictDto, ConflictNextAction, ConflictNextDto, ConflictPathDto, ConflictPathKind,
        ConflictPathStructure, ConflictReadEligibility, ConflictReadNextAction, ConflictStage,
        ConflictState, ConflictTarget, MutationDataDto, SynchronizationConflictMutationDto,
        SynchronizationConflictType,
    },
    results::{
        CheckpointEffect, DiscoveryEffect, Effects, Envelope, IntegrationEffect, Outcome,
        PublicationEffect, ResultCode, Scope, WriteEffect,
    },
};

use super::{Walk, drafts::kit_button};
use crate::desktop::{
    model::operations::CONFLICT_BANNER,
    services::{
        JobKind,
        requests::{Intent, RequestState},
    },
    test_support::fixtures::{Fixture, ticket_path, ticket_source},
    view::{
        conflict::{KEEPING_MINE, KEEPING_THEIRS, REVIEW_CHANGES},
        harness::Shell,
        workspace::Region,
    },
};

const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FT0";
/// The synchronization the canned share answer names.
const SYNCHRONIZATION: &str = "01M4CC0VN1KTQFSVR99378TDHV";
/// A pending row's Review changes (`view/operations/review.rs`).
const PENDING_REVIEW: &str = "pending-review";
/// How long a real resolution may take under the whole suite, as
/// `view/conflict_tests.rs`'s `RESOLVING`.
const RESOLVING: Duration = Duration::from_secs(90);

/// A share stopped on a conflict of `SYNCHRONIZATION`, as
/// `tests/mutation_remote/merging.rs` `c4_merging_canonical_conflict`
/// answers.
fn share_conflict() -> Envelope<MutationDataDto> {
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

/// The ticket's source titled `title`, with the body every side shares.
fn side(title: &str) -> String {
    ticket_source(TICKET, title, "").replace(&format!("Body of {title}."), "Body.")
}

/// The share's conflict, live and to be resolved, whose one path is the
/// ticket with its title changed on both sides: the shape
/// `tests/mutation_remote/conflict_read.rs` `c4_conflict_read_canonical`
/// reads.
fn conflict_read() -> ConflictDto {
    ConflictDto {
        operation_id: SYNCHRONIZATION.to_owned(),
        comment_operation_id: None,
        target: ConflictTarget::Primary,
        item_id: None,
        stage: ConflictStage::Primary,
        state: ConflictState::ConflictPending,
        next_action: ConflictReadNextAction::Resolve,
        request_id: None,
        eligibility: Some(ConflictReadEligibility::EligibleCanonical),
        local_parent: "1".repeat(40),
        incoming_parent: "2".repeat(40),
        observation: Some(format!("conflict-v1:{}", "0".repeat(64))),
        historical: false,
        paths: vec![ConflictPathDto {
            display_path: ticket_path(TICKET),
            token: format!("conflict-path-v1:{}", "1".repeat(64)),
            kind: ConflictPathKind::Ticket,
            structure: ConflictPathStructure::Regular,
            content_available: true,
            base: Some(side("Base")),
            local: Some(side("Local")),
            incoming: Some(side("Incoming")),
        }],
    }
}

/// The resolution applied and published, as
/// `tests/mutation_remote/resolution.rs` `c4_resolution_publishes`
/// answers.
fn applied() -> Envelope<MutationDataDto> {
    Envelope::classified_mutation(
        "conflict resolve",
        Scope::default(),
        false,
        ResultCode::Ok,
        Effects {
            write: WriteEffect::Written,
            checkpoint: CheckpointEffect::Committed,
            integration: IntegrationEffect::Complete,
            publication: PublicationEffect::Published,
            discovery: DiscoveryEffect::Current,
            commit_oid: Some("3".repeat(40)),
            ..Effects::not_requested()
        },
        false,
    )
}

/// The Kit button of part `k`'s choice `id`.
fn choice(id: &'static str, k: usize) -> ElementId {
    (id, k).into()
}

impl Walk {
    /// Tab, in the open sheet, to `id`, which then holds focus on screen
    /// while no region does.
    fn tab_in_sheet(&self, id: ElementId, cx: &mut TestAppContext) {
        self.tab_to("tab", id.clone(), cx);
        self.in_dialog(id, cx);
    }

    /// Once the sheet has read the conflict, Save resolution is drawn
    /// disabled, and Tab passes it by. Tab reaches the title's choice
    /// `pick`, and Enter chooses it: the part says so.
    async fn choose_by_keyboard(&self, pick: &'static str, says: &str, cx: &mut TestAppContext) {
        self.until_drawn("conflict-save", cx).await;
        assert_eq!(
            self.shell.label("conflict-progress", cx).as_deref(),
            Some("0 of 1 chosen")
        );
        assert!(
            !self.shell.tab_reaches("conflict-save", 24, cx),
            "Save resolution is disabled"
        );
        self.tab_in_sheet(choice(pick, 0), cx);
        self.enter(cx);
        assert_eq!(
            self.shell.label("conflict-progress", cx).as_deref(),
            Some("1 of 1 chosen")
        );
        assert_eq!(
            self.shell.label(("conflict-choice", 0usize), cx).as_deref(),
            Some(says)
        );
    }

    /// Tab and Enter on Save resolution, now enabled: one resolve is sent.
    /// While it waits, the sheet keeps focus.
    fn save_by_keyboard(&self, cx: &mut TestAppContext) {
        self.tab_in_sheet("conflict-save".into(), cx);
        self.enter(cx);
        let sent = self.shell.requests(cx, |request| {
            matches!(
                request.intent,
                Intent::ConflictResolve { .. } | Intent::LocalContextResolve { .. }
            )
        });
        assert_eq!(sent.len(), 1, "Save resolution sent one resolve");
    }

    /// Waits, past the harness's wait, until the sheet has closed.
    async fn until_the_sheet_closes(&self, cx: &mut TestAppContext) {
        let deadline = Instant::now() + RESOLVING;
        while self.shell.present("conflict-sheet", cx) {
            assert!(Instant::now() < deadline, "the sheet never closed");
            cx.executor().timer(Duration::from_millis(10)).await;
            self.shell.run(cx);
        }
    }
}

/// A1, A21, real: a local synchronization stopped on a conflict, after a
/// restart. `secondary-4` and Tab reach its pending row's Review changes,
/// and Enter opens the sheet; Escape closes it with nothing sent and gives
/// focus back, and Enter opens it again. Save resolution is disabled until
/// the title is chosen: Tab and Enter keep theirs, then Tab and Enter on
/// Save resolution send the resolution, which applies. The sheet closes,
/// the row goes, and the work context holds theirs.
#[gpui_kit::test]
async fn resolve_a_local_conflict_by_keyboard(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    fixture.conflict_locally(&a, TICKET, "Mine", "Theirs");
    let walk = Walk::over(Shell::open(fixture, cx), a.clone());

    walk.keys("secondary-4", cx);
    walk.region(Region::Operations, cx);
    walk.until_drawn((PENDING_REVIEW, 0usize), cx).await;
    let review = ElementId::from((PENDING_REVIEW, 0usize));
    assert_eq!(
        walk.shell.label(review.clone(), cx).as_deref(),
        Some(REVIEW_CHANGES)
    );
    walk.tab_to("tab", review.clone(), cx);
    walk.on(review.clone(), Region::Operations, cx);

    walk.enter(cx);
    walk.until_drawn("conflict-save", cx).await;
    walk.keys("escape", cx);
    assert!(
        !walk.shell.present("conflict-sheet", cx),
        "Escape closes it"
    );
    walk.on(review.clone(), Region::Operations, cx);
    walk.enter(cx);

    walk.choose_by_keyboard("conflict-keep-theirs", KEEPING_THEIRS, cx)
        .await;
    let sent = walk.shell.requests(cx, |request| {
        matches!(request.intent, Intent::LocalContextResolve { .. })
    });
    assert!(sent.is_empty(), "nothing sent before Save");
    let gate = walk
        .shell
        .backend
        .hold_job_return(JobKind::LocalContextResolve);
    walk.save_by_keyboard(cx);
    walk.in_dialog("conflict-sheet", cx);
    gate.release();
    walk.until_the_sheet_closes(cx).await;
    let resolve = walk.shell.requests(cx, |request| {
        matches!(request.intent, Intent::LocalContextResolve { .. })
    })[0];
    walk.shell.until_ended(resolve, cx).await;
    let finished = walk.shell.operations(cx).read_with(cx, |operations, _| {
        operations
            .request(resolve)
            .map(|request| request.state.clone())
    });
    assert!(
        matches!(
            finished,
            Some(RequestState::Finished(Outcome::Success, ResultCode::Ok, ..))
        ),
        "the resolution applied"
    );
    walk.region(Region::Operations, cx);
    walk.until_drawn("pending-empty", cx).await;

    let context = walk.shell.fixture.context_worktree(&a, TICKET);
    let file = std::fs::read_to_string(context.join(ticket_path(TICKET))).unwrap();
    assert!(file.contains("title: Theirs"), "theirs is kept: {file}");
}

/// A1, A21, canned (decision 9): a share stopped on a conflict, by
/// keyboard. Found by Find, the ticket's Share is reached by Tab after
/// Edit and pressed by Enter; the share is answered as `share_conflict`.
/// The banner says the work is safe, and Tab reaches its Review changes;
/// Enter opens the sheet on the conflict as `conflict_read` reads it. Tab
/// and Enter keep mine, then Save resolution, answered as `applied`: the
/// sheet closes and the banner clears.
#[gpui_kit::test]
async fn resolve_a_shared_conflict_by_keyboard(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    fixture.commit_ticket(&a, TICKET, "Untitled", "");
    fixture.publish_over_ssh(&a);
    fixture.refresh(&a);
    fixture.save_ticket(&a, TICKET, "Mine");
    fixture.refresh(&a);
    let walk = Walk::over(Shell::open(fixture, cx), a);
    walk.select_a_repository(cx);
    walk.find(TICKET, cx);
    walk.keys("secondary-3", cx);
    walk.tab_to("tab", "item-edit", cx);
    walk.keys("tab", cx);
    walk.on("item-share", Region::Content, cx);

    walk.shell
        .backend
        .hold_job_return_answering(JobKind::SynchronizeItem, share_conflict())
        .release();
    walk.enter(cx);
    let share = walk.shell.requests(cx, |request| {
        matches!(request.intent, Intent::ItemSync { .. })
    })[0];
    walk.shell.until_ended(share, cx).await;
    walk.until_drawn("item-conflict", cx).await;
    assert_eq!(
        walk.shell.label("item-conflict", cx).as_deref(),
        Some(CONFLICT_BANNER)
    );
    let review = kit_button("item-review-changes".to_owned());
    walk.tab_to("tab", review.clone(), cx);
    walk.on(review.clone(), Region::Content, cx);

    walk.shell
        .backend
        .answer_next_conflict_read(conflict_read());
    walk.enter(cx);
    walk.choose_by_keyboard("conflict-keep-mine", KEEPING_MINE, cx)
        .await;
    let gate = walk
        .shell
        .backend
        .hold_job_return_answering(JobKind::ConflictResolve, applied());
    walk.save_by_keyboard(cx);
    gate.wait_reached();
    walk.in_dialog("conflict-sheet", cx);
    gate.release();
    walk.until_the_sheet_closes(cx).await;
    walk.shell
        .until_drawn(cx, "the banner's clearing", |shell, cx| {
            !shell.present("item-conflict", cx)
        })
        .await;
    assert!(!walk.shell.present("item-review-changes", cx));
    walk.region(Region::Content, cx);
}
