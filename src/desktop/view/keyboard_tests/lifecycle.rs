//! D5's completion journeys by keyboard alone (D5 design, A2, A6, A21):
//! Close ticket… and Approve and merge…, each over a real local-only
//! repository. The item is found and opened from the keyboard, its
//! header's lifecycle action is reached by Tab, and the dialog is driven by
//! Tab, Enter and Escape only: no click and no call into the workspace.
//! After each step the walk asserts where focus is, that it is on screen,
//! and, in the dialog, that no region holds it.
//!
//! Each waits on the worker, so it allows parking.

use std::path::Path;

use futures::channel::oneshot;
use gpui_kit::{ElementId, FocusHandle, Keystroke, TestAppContext};
use manyhands::{
    repository::{
        FailurePoint, ItemContextKind, ItemDto, ItemDtoKind, ProgressStage, RepositoryService,
        RequestId, SharedKeyId, keys::UnlockReason,
    },
    results::{Outcome, ResultCode},
};

use super::Walk;
use crate::desktop::{
    model::{
        item::{APPROVE_AND_MERGE, CLOSE_TICKET},
        navigation::OpenTarget,
    },
    services::{
        JobKind,
        prompts::{PromptReply, PromptRequest},
        requests::{DesktopRequestId, Intent, RequestState},
    },
    test_support::fixtures::{Fixture, item_id},
    view::{
        harness::{Shell, key},
        item::LIFECYCLE_ACTION,
        lifecycle::{
            DONE, ENDING_DONE, ENDING_STOPPED, KEEP_LOCAL, NOT_NOW, SHARING_PENDING, TIDIED_UP,
            merged,
        },
        passphrase::open_dialog,
        workspace::Region,
    },
};

const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR0";
const DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR1";

/// The Kit button of the lifecycle dialog's control `id`, which takes the
/// focus.
fn kit(id: &'static str) -> ElementId {
    (id, 0usize).into()
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

/// Whether `path` is in the tree of `root`'s main.
fn on_main(root: &Path, path: &str) -> bool {
    let repository = git2::Repository::open(root).unwrap();
    let tree = repository
        .revparse_single("refs/heads/main")
        .unwrap()
        .peel_to_tree()
        .unwrap();
    tree.get_path(Path::new(path)).is_ok()
}

/// A window over the one repository `a` that `build` fills, refreshed.
fn walk(cx: &mut TestAppContext, build: impl FnOnce(&Fixture, &Path)) -> Walk {
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    build(&fixture, &a);
    fixture.refresh(&a);
    Walk::over(Shell::open(fixture, cx), a)
}

impl Walk {
    /// The item `id` of `a`, as the library reads it now.
    fn shown(&self, id: &str) -> ItemDto {
        let service = self.shell.fixture.service();
        let repo = service.resolve_repository(&self.a).unwrap();
        service.show_item(&repo, &item_id(id)).unwrap()
    }

    /// The lifecycle requests of this session, newest first.
    fn lifecycles(&self, cx: &mut TestAppContext) -> Vec<DesktopRequestId> {
        self.shell.requests(cx, |request| {
            matches!(
                request.intent,
                Intent::TicketClose { .. } | Intent::DocumentPromote { .. }
            )
        })
    }

    fn outcome(
        &self,
        id: DesktopRequestId,
        cx: &mut TestAppContext,
    ) -> Option<(Outcome, ResultCode)> {
        let operations = self.shell.operations(cx);
        operations.read_with(cx, |operations, _| {
            match &operations.request(id).unwrap().state {
                RequestState::Finished(outcome, code, ..) => Some((*outcome, *code)),
                _ => None,
            }
        })
    }

    fn request_id(&self, id: DesktopRequestId, cx: &mut TestAppContext) -> RequestId {
        let operations = self.shell.operations(cx);
        operations.read_with(cx, |operations, _| {
            operations.request(id).unwrap().request_id
        })
    }

    /// The shown tab's lifecycle action's focus stop, where the dialog
    /// gives focus back.
    fn action_focus(&self, cx: &mut TestAppContext) -> FocusHandle {
        let pane = self
            .shell
            .workspace
            .read_with(cx, |workspace, _| workspace.item_pane())
            .expect("an item tab is shown");
        pane.read_with(cx, |pane, _| pane.lifecycle_focus())
    }

    /// From the open dialog, Tab to its control `id`, whose Kit button
    /// then holds focus on screen while no region does
    /// (`keyboard_tests/drafts.rs`'s `in_dialog`).
    fn tab_to_control(&self, id: &'static str, cx: &mut TestAppContext) {
        self.tab_to("tab", kit(id), cx);
        self.in_dialog(kit(id), cx);
    }

    /// Opens the item `id` by Find, then `secondary-3` and Tab reach its
    /// header's lifecycle action, which reads `label`; shift-Tab is Share
    /// before it, and Tab comes back.
    fn reach_the_action(&self, id: &str, label: &str, cx: &mut TestAppContext) {
        self.select_a_repository(cx);
        self.reach_the_action_in(id, label, 0, cx);
    }

    /// `reach_the_action` with the repository already chosen, the item
    /// opening as the strip's tab `tab`.
    fn reach_the_action_in(&self, id: &str, label: &str, tab: usize, cx: &mut TestAppContext) {
        self.find(id, cx);
        self.keys("secondary-3", cx);
        self.on(("tab", tab), Region::Content, cx);
        self.tab_to("tab", LIFECYCLE_ACTION, cx);
        self.on(LIFECYCLE_ACTION, Region::Content, cx);
        assert_eq!(
            self.shell.label(LIFECYCLE_ACTION, cx).as_deref(),
            Some(label)
        );
        self.keys("shift-tab", cx);
        self.on("item-share", Region::Content, cx);
        self.keys("tab", cx);
        self.on(LIFECYCLE_ACTION, Region::Content, cx);
    }

    /// Enter on the focused action opens the dialog, which prepares; once
    /// its summary is shown, Tab reaches Confirm and Cancel inside it.
    async fn open_the_dialog(&self, cx: &mut TestAppContext) {
        self.enter(cx);
        self.until_drawn(kit("lifecycle-confirm"), cx).await;
        self.tab_to_control("lifecycle-confirm", cx);
        self.tab_to_control("lifecycle-cancel", cx);
        self.tab_to_control("lifecycle-confirm", cx);
    }

    /// Escape closes the dialog: nothing is sent, and focus is back on the
    /// action's stop, from which Enter opens it again.
    fn escape_to_the_action(&self, focus: &FocusHandle, cx: &mut TestAppContext) {
        self.keys("escape", cx);
        assert!(!self.shell.present("lifecycle-dialog", cx));
        self.on_handle(focus, Region::Content, cx);
    }

    /// Waits until the dialog says how the run ended, and gives its words.
    async fn ending(&self, cx: &mut TestAppContext) -> Option<String> {
        self.until_drawn("lifecycle-ending", cx).await;
        self.shell.label("lifecycle-ending", cx)
    }
}

/// A2, A6, A21, real local-only: Close ticket… by keyboard. Found by Find,
/// its action reached by Tab after Share; Enter opens the dialog and Tab
/// moves between Confirm and Cancel; Escape cancels with nothing sent and
/// gives focus back, and Enter opens it again. Tab and Enter on Confirm
/// send the close; held at `integrating`, Tab and Enter on Stop at a safe
/// point stop it before the merge; Tab and Enter on Retry send the same
/// request again, which finishes: Done, local-only. Enter closes the
/// dialog. The ticket is closed and merged into main, and its working copy
/// is tidied up.
#[gpui_kit::test]
async fn close_a_ticket_by_keyboard(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let walk = walk(cx, |fixture, a| {
        fixture.commit_ticket(a, TICKET, "Untitled", "");
        fixture.refresh(a);
        fixture.save_ticket(a, TICKET, "Closing");
    });
    let worktree = walk.shown(TICKET).context.worktree;
    let main = commits(&walk.a, "main");
    walk.reach_the_action(TICKET, CLOSE_TICKET, cx);
    let focus = walk.action_focus(cx);

    walk.open_the_dialog(cx).await;
    assert_eq!(
        walk.shell
            .rows("lifecycle-step", cx)
            .get(1)
            .map(String::as_str),
        Some(KEEP_LOCAL),
        "local-only"
    );
    walk.escape_to_the_action(&focus, cx);
    assert!(walk.lifecycles(cx).is_empty(), "Escape sends nothing");
    walk.open_the_dialog(cx).await;

    let gate = walk
        .shell
        .backend
        .hold_at_stage(JobKind::TicketClose, ProgressStage::Integrating);
    walk.tab_to_control("lifecycle-confirm", cx);
    walk.enter(cx);
    let sent = walk.lifecycles(cx);
    assert_eq!(sent.len(), 1, "Confirm sent one close");
    walk.in_dialog("lifecycle-dialog", cx);
    gate.wait_reached();
    walk.until_drawn(kit("lifecycle-stop"), cx).await;
    walk.tab_to_control("lifecycle-stop", cx);
    walk.enter(cx);
    assert!(walk.shell.present("lifecycle-stopping", cx), "Stop pressed");
    walk.in_dialog("lifecycle-dialog", cx);
    gate.release();
    walk.shell.until_ended(sent[0], cx).await;
    assert_eq!(
        walk.outcome(sent[0], cx),
        Some((Outcome::Cancelled, ResultCode::Cancelled))
    );
    assert_eq!(walk.ending(cx).await.as_deref(), Some(ENDING_STOPPED));
    assert_eq!(commits(&walk.a, "main"), main, "stopped before the merge");

    walk.tab_to_control("lifecycle-retry", cx);
    walk.enter(cx);
    walk.in_dialog("lifecycle-dialog", cx);
    let retried = walk.lifecycles(cx);
    assert_eq!(retried.len(), 2, "Retry sent the close again");
    assert_eq!(
        walk.request_id(retried[0], cx),
        walk.request_id(sent[0], cx),
        "under the same request ID"
    );
    walk.shell.until_ended(retried[0], cx).await;
    assert_eq!(
        walk.outcome(retried[0], cx),
        Some((Outcome::Success, ResultCode::Ok))
    );
    walk.shell
        .until_drawn(cx, "Done", |shell, cx| {
            shell.label("lifecycle-ending", cx).as_deref() == Some(ENDING_DONE)
        })
        .await;
    let outcomes = walk.shell.rows("lifecycle-outcome", cx);
    for line in [merged("main").as_str(), SHARING_PENDING, TIDIED_UP] {
        assert!(
            outcomes.iter().any(|shown| shown == line),
            "{line}: {outcomes:?}"
        );
    }

    walk.enter(cx);
    assert!(
        !walk.shell.present("lifecycle-dialog", cx),
        "Enter closes Done"
    );
    walk.region(Region::Content, cx);

    let closed = walk.shown(TICKET);
    assert!(
        closed
            .closure
            .and_then(|closure| closure.closed_at)
            .is_some(),
        "the ticket is closed"
    );
    assert_eq!(closed.context.kind, ItemContextKind::Primary);
    assert!(commits(&walk.a, "main") > main, "merged into main");
    assert!(
        !Path::new(&worktree).exists(),
        "its working copy is tidied up"
    );
}

/// A2, A6, A21, real local-only: Approve and merge… by keyboard. Found by
/// Find, its action reached by Tab after Share; Enter opens the dialog,
/// and Enter, the dialog's default, confirms. Held at `integrating`,
/// Escape hides the dialog and gives focus back to the action while the
/// merge goes on; it finishes, the document is on main, and focus is
/// still in the content region (review M-6).
#[gpui_kit::test]
async fn approve_and_merge_a_document_by_keyboard(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let walk = walk(cx, |fixture, a| {
        fixture.create_document(a, DOCUMENT, "docs/plan.md", "Plan");
    });
    assert!(!on_main(&walk.a, "docs/plan.md"), "not yet on main");
    walk.reach_the_action(DOCUMENT, APPROVE_AND_MERGE, cx);
    let focus = walk.action_focus(cx);
    assert_eq!(
        walk.opened(cx),
        Some(OpenTarget::Item {
            repo: key(&walk.a),
            kind: ItemDtoKind::Document,
            id: item_id(DOCUMENT),
        })
    );

    walk.open_the_dialog(cx).await;
    let gate = walk
        .shell
        .backend
        .hold_at_stage(JobKind::DocumentPromote, ProgressStage::Integrating);
    walk.enter(cx);
    let sent = walk.lifecycles(cx);
    assert_eq!(sent.len(), 1, "Enter confirmed once");
    gate.wait_reached();
    walk.escape_to_the_action(&focus, cx);
    gate.release();
    walk.shell.until_ended(sent[0], cx).await;
    assert_eq!(
        walk.outcome(sent[0], cx),
        Some((Outcome::Success, ResultCode::Ok))
    );
    assert_eq!(walk.lifecycles(cx), sent, "sent once");
    assert!(on_main(&walk.a, "docs/plan.md"), "merged into main");
    walk.shell.run(cx);
    walk.region(Region::Content, cx);
}

/// Review I-1 (A21), real local-only: Stop at a safe point holds focus,
/// reached by Tab, when the close finishes by itself. Stop goes with the
/// running stage, and focus stays in the dialog, so Enter closes Done and
/// focus is back in the content region.
#[gpui_kit::test]
async fn focus_stays_in_the_dialog_when_stop_goes_by_itself(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let walk = walk(cx, |fixture, a| {
        fixture.commit_ticket(a, TICKET, "Untitled", "");
        fixture.refresh(a);
        fixture.save_ticket(a, TICKET, "Finishing");
    });
    walk.reach_the_action(TICKET, CLOSE_TICKET, cx);
    walk.open_the_dialog(cx).await;
    let gate = walk
        .shell
        .backend
        .hold_at_stage(JobKind::TicketClose, ProgressStage::Integrating);
    walk.enter(cx);
    let sent = walk.lifecycles(cx);
    gate.wait_reached();
    walk.until_drawn(kit("lifecycle-stop"), cx).await;
    walk.tab_to_control("lifecycle-stop", cx);
    gate.release();
    walk.shell.until_ended(sent[0], cx).await;
    walk.shell
        .until_drawn(cx, "Done", |shell, cx| {
            shell.label("lifecycle-ending", cx).as_deref() == Some(ENDING_DONE)
        })
        .await;
    assert!(!walk.shell.present("lifecycle-stop", cx), "Stop went");
    walk.in_dialog("lifecycle-dialog", cx);
    walk.enter(cx);
    assert!(
        !walk.shell.present("lifecycle-dialog", cx),
        "Enter closes Done"
    );
    walk.region(Region::Content, cx);
}

/// Re-review I-1 (A21), real local-only: Enter on Confirm sends the close
/// and Confirm goes with its stage, and in the same turn, before a frame is
/// drawn, a passphrase prompt opens, as the workspace opens one when the
/// worker asks for a key (`passphrase::open_dialog`). Escape cancels the
/// prompt, and focus is back in the lifecycle dialog, not on the gone
/// Confirm: Escape then closes the dialog, and focus is back on the
/// action.
#[gpui_kit::test]
async fn a_prompt_over_a_pressed_confirm_gives_focus_back_to_the_dialog(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let walk = walk(cx, |fixture, a| {
        fixture.commit_ticket(a, TICKET, "Untitled", "");
        fixture.refresh(a);
        fixture.save_ticket(a, TICKET, "Prompted");
    });
    walk.reach_the_action(TICKET, CLOSE_TICKET, cx);
    let focus = walk.action_focus(cx);
    walk.open_the_dialog(cx).await;
    let held = walk.shell.backend.hold_job(JobKind::TicketClose);
    walk.tab_to_control("lifecycle-confirm", cx);

    let (reply, mut answer) = oneshot::channel();
    let prompt = PromptRequest {
        key_id: SharedKeyId::new(),
        label: "primary".to_owned(),
        reason: UnlockReason::ProtectedKey,
        reply,
    };
    let services = walk
        .shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().clone());
    walk.shell.visual(cx).update(|window, cx| {
        window.dispatch_keystroke(Keystroke::parse("enter").unwrap(), cx);
        open_dialog(prompt, &services, window, cx);
    });
    walk.shell.run(cx);
    assert_eq!(
        walk.lifecycles(cx).len(),
        1,
        "Enter on Confirm sent the close"
    );
    assert!(walk.shell.present("passphrase-dialog", cx));

    walk.keys("escape", cx);
    assert!(matches!(
        answer.try_recv(),
        Ok(Some(PromptReply::Cancelled))
    ));
    assert!(!walk.shell.present("passphrase-dialog", cx));
    walk.in_dialog("lifecycle-dialog", cx);
    walk.escape_to_the_action(&focus, cx);
    held.release();
    let sent = walk.lifecycles(cx);
    walk.shell.until_ended(sent[0], cx).await;
}

/// P13 (Wave exit), real local-only: a close and an approval that fail,
/// retried by keyboard to Done. The library's failure point
/// `BeforeRequestDomainCall` fails each first run with `internal_error`
/// once it is accepted, and the dialog offers Retry after Not now (A7). The
/// normal service is put back without moving the branch; Tab and Enter on
/// Retry send the same request again, which finishes. Tab reaches Done,
/// and Enter on it closes the dialog; neither item offers its action any
/// more, the closed ticket no Close ticket… and the merged document, now
/// on main, no Approve and merge…, so focus is in the content region, on
/// its tab.
#[gpui_kit::test]
async fn a_failed_run_is_retried_by_keyboard(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let walk = walk(cx, |fixture, a| {
        fixture.commit_ticket(a, TICKET, "Untitled", "");
        fixture.refresh(a);
        fixture.save_ticket(a, TICKET, "Failing");
        fixture.create_document(a, DOCUMENT, "docs/plan.md", "Plan");
    });
    let commands = [(TICKET, CLOSE_TICKET), (DOCUMENT, APPROVE_AND_MERGE)];
    for (tab, (id, label)) in commands.into_iter().enumerate() {
        walk.shell.backend.replace_service(
            RepositoryService::open_at_with_failure_point_for_testing(
                walk.shell.fixture.data_dir(),
                FailurePoint::BeforeRequestDomainCall,
            )
            .unwrap(),
        );
        if tab == 0 {
            walk.reach_the_action(id, label, cx);
        } else {
            walk.reach_the_action_in(id, label, tab, cx);
        }
        walk.open_the_dialog(cx).await;
        walk.tab_to_control("lifecycle-confirm", cx);
        walk.enter(cx);
        let sent = walk.lifecycles(cx)[0];
        walk.shell.until_ended(sent, cx).await;
        assert_eq!(
            walk.outcome(sent, cx).map(|(_, code)| code),
            Some(ResultCode::InternalError),
            "{label}: the failure point"
        );
        walk.until_drawn(kit("lifecycle-retry"), cx).await;
        walk.tab_to_control("lifecycle-cancel", cx);
        assert_eq!(
            walk.shell.label("lifecycle-cancel", cx).as_deref(),
            Some(NOT_NOW),
            "{label}"
        );

        walk.shell
            .backend
            .replace_service(RepositoryService::open_at(walk.shell.fixture.data_dir()).unwrap());
        walk.tab_to_control("lifecycle-retry", cx);
        walk.enter(cx);
        walk.in_dialog("lifecycle-dialog", cx);
        let retried = walk.lifecycles(cx)[0];
        assert_ne!(retried, sent, "{label}: Retry sent it again");
        assert_eq!(
            walk.request_id(retried, cx),
            walk.request_id(sent, cx),
            "{label}: under the same request ID"
        );
        walk.shell.until_ended(retried, cx).await;
        assert_eq!(
            walk.outcome(retried, cx),
            Some((Outcome::Success, ResultCode::Ok)),
            "{label}"
        );
        assert_eq!(walk.ending(cx).await.as_deref(), Some(ENDING_DONE));

        walk.tab_to_control("lifecycle-cancel", cx);
        assert_eq!(
            walk.shell.label("lifecycle-cancel", cx).as_deref(),
            Some(DONE),
            "{label}"
        );
        walk.enter(cx);
        assert!(
            !walk.shell.present("lifecycle-dialog", cx),
            "{label}: Enter on Done closes it"
        );
        // Ruling O-6: a closed ticket offers no Close ticket… any
        // more, nor a merged document, now on main, Approve and
        // merge…, so the origin is not drawn. Done gives focus back to
        // it, and D5's `focus_lost` fallback, unchanged (A8), keeps it
        // in the content region, on the item's tab.
        assert!(!walk.shell.present(LIFECYCLE_ACTION, cx), "{label}");
        walk.on(("tab", tab), Region::Content, cx);
    }
}
