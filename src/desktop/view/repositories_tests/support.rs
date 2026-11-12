//! What the Repositories panel's window tests share: item IDs, poll and
//! job answers, readings of the rows, and the repository menu read and
//! chosen by label.

use std::path::Path;

use gpui_kit::{
    ElementId, FocusHandle, InputEvent, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers,
    MouseButton, MouseDownEvent, MouseUpEvent, Pixels, Point, TestAppContext, test::TestWindowExt,
};
use manyhands::{
    canonical::ItemId,
    repository::{MutationDataDto, OperationId, PollCompletion, PollIneligibility},
    results::{CheckpointEffect, Effects, Envelope, Outcome, PublicationEffect, ResultCode, Scope},
};

use super::super::{RepositoryRow, rows};
use crate::desktop::{
    model::polling::{
        Answer, AttemptTarget, Clock as _, Disposition, PollAttempt, TargetKind, TestClock,
    },
    model::stamp::RepoKey,
    services::{
        ReadKind, Services,
        poll_convert::{Library, ReportParts},
        requests::{DesktopRequestId, Intent, Lane},
    },
    view::{harness::Shell, settings::Shown as Page},
};

pub(super) const FIRST: &str = "01ARZ3NDEKTSV4RRFFQ69G5RA1";
pub(super) const SECOND: &str = "01ARZ3NDEKTSV4RRFFQ69G5RA2";
pub(super) const THIRD: &str = "01ARZ3NDEKTSV4RRFFQ69G5RA3";

pub(super) fn id(text: &str) -> ItemId {
    text.parse().unwrap()
}

pub(super) fn child(name: &'static str, part: &'static str) -> ElementId {
    (ElementId::from(name), part).into()
}

/// A poll the library says is not due for an hour: it records no attempt
/// and changes no state a row shows, only when the next poll is due. The
/// stand-in for every poll nothing is queued for.
pub(super) fn not_due(clock: &TestClock) -> impl Fn() -> Library + Send + 'static {
    let next = clock.utc() + time::Duration::hours(1);
    move || Library::NotEligible {
        reason: PollIneligibility::NotDue,
        next_eligible_at: Some(next),
        index_pending: false,
    }
}

/// A poll that completed now and reported no target: it sets no list.
pub(super) fn completed(clock: &TestClock) -> Library {
    let now = clock.utc();
    Library::Polled {
        parts: ReportParts {
            completion: PollCompletion::Completed,
            completed_at: now,
            next_eligible_at: Some(now + time::Duration::seconds(300)),
            targets: Vec::new(),
            outcome: None,
        },
        index_pending: false,
    }
}

pub(super) fn item(text: &str) -> AttemptTarget {
    AttemptTarget {
        item_id: Some(id(text)),
        kind: Some(TargetKind::Ticket),
        disposition: Disposition::LocalAhead,
        main: false,
    }
}

/// Main, always reported (fact 70): here shared.
pub(super) fn main_current() -> AttemptTarget {
    AttemptTarget {
        item_id: None,
        kind: None,
        disposition: Disposition::Current,
        main: true,
    }
}

/// A finished share that published, or a save that made a checkpoint and
/// did not publish, of item `id`.
pub(super) fn finished(command: &str, item: &str, published: bool) -> Envelope<MutationDataDto> {
    let effects = if published {
        Effects {
            publication: PublicationEffect::Published,
            ..Effects::not_requested()
        }
    } else {
        Effects {
            checkpoint: CheckpointEffect::Committed,
            ..Effects::not_requested()
        }
    };
    Envelope::classified_mutation(
        command,
        Scope {
            item_id: Some(item.to_owned()),
            ..Scope::default()
        },
        false,
        ResultCode::Ok,
        effects,
        false,
    )
}

/// The index of the row named `name`.
pub(super) fn index(shell: &Shell, name: &str, cx: &mut TestAppContext) -> usize {
    let names = shell.rows("repo-row", cx);
    names
        .iter()
        .position(|shown| shown == name)
        .unwrap_or_else(|| panic!("no row {name} in {names:?}"))
}

/// What the row named `name` says at its right, if anything.
pub(super) fn state(shell: &Shell, name: &str, cx: &mut TestAppContext) -> Option<String> {
    let at = index(shell, name, cx);
    shell.label(("repo-state", at), cx)
}

pub(super) fn reads(shell: &Shell, kind: ReadKind) -> usize {
    shell.backend.reads_seen(kind)
}

/// Whether the pane holds the rows the model gives, without a frame drawn
/// by the test (P16).
pub(super) fn pane_follows(shell: &Shell, cx: &mut TestAppContext) -> bool {
    shell.workspace.read_with(cx, |workspace, cx| {
        workspace.repositories_pane().rows(cx) == rows(workspace, cx)
    })
}

/// Submits `intent` on the user lane and waits for it to end.
pub(super) async fn finish(shell: &Shell, intent: Intent, cx: &mut TestAppContext) {
    let request = shell.workspace.read_with(cx, |workspace, _| {
        workspace
            .services()
            .worker()
            .submit(intent, Lane::User, None, None)
    });
    shell.until_ended(request, cx).await;
    shell.run(cx);
}

/// A completed check of `root` that reported `targets`, as the schedule's
/// own transition takes it; the scheduler notifies, as it does when a poll
/// is answered.
pub(super) fn checked(
    services: &Services,
    clock: &TestClock,
    root: &str,
    targets: Vec<AttemptTarget>,
    cx: &mut TestAppContext,
) {
    let now = clock.now();
    let attempt = PollAttempt {
        root: root.to_owned(),
        operation: OperationId::new(),
        answer: Answer::Completed,
        completed_at: Some(now.utc),
        next_eligible_at: Some(now.utc + time::Duration::seconds(300)),
        targets,
        index_pending: false,
    };
    services.scheduler().update(cx, |scheduler, cx| {
        scheduler.schedule_mut().after(&now, &attempt);
        cx.notify();
    });
}

/// The scheduler notifies with nothing new to say.
pub(super) fn notify(services: &Services, cx: &mut TestAppContext) {
    services.scheduler().update(cx, |_, cx| cx.notify());
}

/// A Check now's answer, in place of the library's: nothing reaches a
/// transport.
pub(super) fn polled() -> Envelope<MutationDataDto> {
    Envelope::mutation(
        "poll once",
        Scope::default(),
        Outcome::Success,
        ResultCode::Ok,
    )
}

/// A share of main's answer, in place of the library's.
pub(super) fn synced() -> Envelope<MutationDataDto> {
    Envelope::mutation(
        "repo sync",
        Scope::default(),
        Outcome::Success,
        ResultCode::Ok,
    )
}

/// The row named `name`, as the model gives it.
pub(super) fn row_of(shell: &Shell, name: &str, cx: &mut TestAppContext) -> RepositoryRow {
    shell.workspace.read_with(cx, |workspace, cx| {
        rows(workspace, cx)
            .into_iter()
            .find(|row| row.name == name)
            .unwrap_or_else(|| panic!("no row {name}"))
    })
}

/// The labels `menu_items` gives the row named `name`, in order.
pub(super) fn items_of(shell: &Shell, name: &str, cx: &mut TestAppContext) -> Vec<String> {
    let row = row_of(shell, name, cx);
    row.menu.iter().map(|item| item.label.clone()).collect()
}

/// The repository whose menu is open.
pub(in crate::desktop::view) fn open_root(
    shell: &Shell,
    cx: &mut TestAppContext,
) -> Option<RepoKey> {
    shell.frame(cx, |_, _| ());
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.repository_menu_root())
}

/// The open menu's items, in order, by label; a separator has none.
pub(in crate::desktop::view) fn labels(shell: &Shell, cx: &mut TestAppContext) -> Vec<String> {
    shell.frame(cx, |window, _| {
        let menu = window.within("popup-menu");
        (0usize..16)
            .filter_map(|ix| menu.try_find(ix))
            .filter_map(|item| item.label().map(str::to_owned))
            .collect()
    })
}

/// Where the open menu's item `label` is.
fn item_at(shell: &Shell, label: &str, cx: &mut TestAppContext) -> Point<Pixels> {
    shell
        .frame(cx, |window, _| {
            let menu = window.within("popup-menu");
            (0usize..16)
                .filter_map(|ix| menu.try_find(ix))
                .find(|item| item.label() == Some(label))
                .map(|item| item.bounds().center())
        })
        .unwrap_or_else(|| panic!("no item {label}"))
}

/// Clicks the open menu's item `label`, its mouse events one at a time, so
/// the choice reaches the Workspace before the next frame, as in the
/// application (`Shell::click_settled`).
pub(in crate::desktop::view) fn choose(shell: &Shell, label: &str, cx: &mut TestAppContext) {
    let at = item_at(shell, label, cx);
    let mut visual = shell.visual(cx);
    visual.simulate_click(at, Modifiers::default());
    visual.run_until_parked();
}

/// Clicks the open menu's item `label` and says whether `handle` holds
/// focus in that same update, before its effects run and the next draw.
pub(in crate::desktop::view) fn choose_and_hold(
    shell: &Shell,
    label: &str,
    handle: &FocusHandle,
    cx: &mut TestAppContext,
) -> bool {
    let at = item_at(shell, label, cx);
    let held = shell.frame(cx, |window, cx| {
        let (button, modifiers) = (MouseButton::Left, Modifiers::default());
        let down = MouseDownEvent {
            button,
            position: at,
            modifiers,
            click_count: 1,
            first_mouse: false,
        };
        window.dispatch_event(down.to_platform_input(), cx);
        let up = MouseUpEvent {
            button,
            position: at,
            modifiers,
            click_count: 1,
        };
        window.dispatch_event(up.to_platform_input(), cx);
        handle.is_focused(window)
    });
    shell.run(cx);
    held
}

/// Presses `key` and says whether `handle` holds focus in that same
/// update, before its effects run and the next draw.
pub(in crate::desktop::view) fn press_and_hold(
    shell: &Shell,
    key: &str,
    handle: &FocusHandle,
    cx: &mut TestAppContext,
) -> bool {
    let held = shell.frame(cx, |window, cx| {
        window.dispatch_keystroke(Keystroke::parse(key).unwrap(), cx);
        handle.is_focused(window)
    });
    shell.run(cx);
    held
}

/// Presses and releases `key`, as a keyboard does (GPUI clicks a focused
/// button when Enter is released), and says whether `handle` holds focus in
/// that same update, before its effects run and the next draw.
pub(in crate::desktop::view) fn press_released_and_hold(
    shell: &Shell,
    key: &str,
    handle: &FocusHandle,
    cx: &mut TestAppContext,
) -> bool {
    let held = shell.frame(cx, |window, cx| {
        let keystroke = Keystroke::parse(key).unwrap();
        let down = KeyDownEvent {
            keystroke: keystroke.clone(),
            is_held: false,
            prefer_character_input: false,
        };
        window.dispatch_event(down.to_platform_input(), cx);
        window.dispatch_event(KeyUpEvent { keystroke }.to_platform_input(), cx);
        handle.is_focused(window)
    });
    shell.run(cx);
    held
}

/// Closes the open menu, if one is, by Esc.
pub(in crate::desktop::view) fn close_menu(shell: &Shell, cx: &mut TestAppContext) {
    if open_root(shell, cx).is_some() {
        shell.keys("escape", cx);
    }
    assert_eq!(open_root(shell, cx), None);
}

/// A right click on the row named `name`.
pub(super) fn right_click(shell: &Shell, name: &str, cx: &mut TestAppContext) {
    let at = index(shell, name, cx);
    shell.frame(cx, |window, cx| window.right_click(("repo-row", at), cx));
    shell.run(cx);
}

/// The Check nows of `root` sent on the user lane, newest first.
pub(super) fn checks(shell: &Shell, root: &Path, cx: &mut TestAppContext) -> Vec<DesktopRequestId> {
    shell.requests(cx, |request| {
        request.lane == Lane::User
            && matches!(&request.intent, Intent::PollOnce { root: asked } if asked == root)
    })
}

pub(super) fn all_reads(shell: &Shell) -> usize {
    shell.backend.reads_in_order().len()
}

pub(in crate::desktop::view) fn focused(
    shell: &Shell,
    handle: &FocusHandle,
    cx: &mut TestAppContext,
) -> bool {
    shell.frame(cx, |window, _| handle.is_focused(window))
}

pub(super) fn page(shell: &Shell, cx: &mut TestAppContext) -> Page {
    shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.settings_shown(cx))
}
