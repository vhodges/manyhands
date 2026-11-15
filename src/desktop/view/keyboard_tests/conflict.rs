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
use manyhands::results::{Outcome, ResultCode};

use super::{Walk, drafts::kit_button};
use crate::desktop::{
    model::operations::CONFLICT_BANNER,
    services::{
        JobKind,
        requests::{Intent, RequestState},
    },
    test_support::fixtures::{Fixture, ticket_path},
    view::{
        activity_support::{list_focused, refresh, row as activity_row},
        conflict::{KEEPING_MINE, KEEPING_THEIRS},
        harness::Shell,
        workspace::Region,
    },
};

const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FT0";
/// The synchronization the canned share answer names.
const SYNCHRONIZATION: &str = "01M4CC0VN1KTQFSVR99378TDHV";
/// How long a real resolution may take under the whole suite, as
/// `view/conflict_tests.rs`'s `RESOLVING`.
const RESOLVING: Duration = Duration::from_secs(90);

mod canned;
mod sheet;

use canned::{applied, conflict_read, share_conflict};

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

    /// Waits until the open sheet has drawn `id`.
    async fn sheet_drawn(&self, id: &'static str, cx: &mut TestAppContext) {
        self.until_drawn(id, cx).await;
        assert!(self.shell.present("conflict-sheet", cx));
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
/// restart. `secondary-4` and the arrows reach its pending row, and its
/// menu's Review changes (`shift-f10`, the arrows and Enter) opens the sheet;
/// Escape closes it with nothing sent and gives focus back to the list, the
/// row still highlighted, and the menu opens it again. Save resolution is disabled until
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

    let review = walk.to_pending_review("context_sync_local", cx).await;
    walk.until_drawn("conflict-save", cx).await;
    walk.keys("escape", cx);
    assert!(
        !walk.shell.present("conflict-sheet", cx),
        "Escape closes it"
    );
    walk.region(Region::Operations, cx);
    assert!(list_focused(&walk.shell, cx), "focus is back on the list");
    assert_eq!(
        crate::desktop::view::activity_support::highlighted(&walk.shell, cx),
        Some(review.clone()),
        "its row is still highlighted"
    );
    walk.choose_review(cx);

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
    walk.shell
        .until_drawn(cx, "the pending row goes", |shell, cx| {
            refresh(shell, cx);
            activity_row(shell, &review, cx).is_none()
        })
        .await;

    let context = walk.shell.fixture.context_worktree(&a, TICKET);
    let file = std::fs::read_to_string(context.join(ticket_path(TICKET))).unwrap();
    assert!(file.contains("title: Theirs"), "theirs is kept: {file}");
}

/// A window over `a`, whose ticket has a work context and whose `origin`
/// is selected for publication. Found by Find, the ticket's Share is
/// reached by Tab after Edit and pressed by Enter; the share runs and is
/// answered as `share_conflict`. The banner says the work is safe, and
/// Tab reaches its Review changes, which then holds focus.
async fn shared_banner(cx: &mut TestAppContext) -> Walk {
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
    walk.on(review, Region::Content, cx);
    walk
}

/// A1, A21, canned (decision 9): a share stopped on a conflict, by
/// keyboard (`shared_banner`). Enter on Review changes opens the sheet,
/// whose first read fails, as the library answers for an operation the
/// repository does not hold: Tab and Enter on Read again read it again,
/// as `conflict_read`. Read again goes once the read lands, and focus
/// stays in the sheet. Tab and Enter keep mine, then Save resolution,
/// answered as `applied`: the sheet closes, the banner clears, and focus
/// is back in the content region, though the Review changes it came from
/// has gone.
#[gpui_kit::test]
async fn resolve_a_shared_conflict_by_keyboard(cx: &mut TestAppContext) {
    let walk = shared_banner(cx).await;
    walk.enter(cx);
    walk.sheet_drawn("conflict-read-again", cx).await;
    walk.tab_in_sheet("conflict-read-again".into(), cx);
    walk.shell
        .backend
        .answer_next_conflict_read(conflict_read());
    walk.enter(cx);
    walk.sheet_drawn("conflict-save", cx).await;
    assert!(!walk.shell.present("conflict-read-again", cx));
    walk.in_dialog("conflict-sheet", cx);

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
