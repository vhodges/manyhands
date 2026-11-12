//! What the Repositories panel's window tests share: item IDs, poll and
//! job answers, and readings of the rows.

use gpui_kit::{ElementId, TestAppContext};
use manyhands::{
    canonical::ItemId,
    repository::{MutationDataDto, OperationId, PollCompletion},
    results::{CheckpointEffect, Effects, Envelope, PublicationEffect, ResultCode, Scope},
};

use super::super::rows;
use crate::desktop::{
    model::polling::{
        Answer, AttemptTarget, Clock as _, Disposition, PollAttempt, TargetKind, TestClock,
    },
    services::{
        ReadKind, Services,
        poll_convert::{Library, ReportParts},
        requests::{Intent, Lane},
    },
    view::harness::Shell,
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
