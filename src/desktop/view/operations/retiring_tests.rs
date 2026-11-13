//! The retirement rows of the operation area (D6 Task 11b, design E3, W-6,
//! decision 13): what the attempt's entry says of each retirement, and the
//! request row of one that stayed, at the default window. The rows wrap
//! inside Activity, and the action of a row is where a click reaches it.
//! Retire… and Resume are `retire_tests`'s.

use gpui_kit::{TestAppContext, px, size, test::TestWindowExt};
use manyhands::{
    canonical::ItemId,
    repository::{ContextRetirementState as State, OperationId, RetirementRemoval as Removal},
    results::ResultCode as Code,
};

use super::request_id;
use crate::desktop::{
    model::polling::{
        Answer, AttemptTarget, Clock, Disposition, PollAttempt, TargetKind,
        entry::{retired_text, retiring_text, unfinished_text},
        retire::Verdict,
    },
    services::{
        JobKind,
        requests::{Lane, RequestState},
    },
    test_support::retire::answer,
    view::settings::polling_support::Rig,
};

const TABS: usize = 80;

fn short(id: &ItemId) -> String {
    let id = id.to_string();
    id[id.len() - 8..].to_owned()
}

/// The element of repository `a`'s entry.
fn entry_id(rig: &Rig, part: &str) -> String {
    format!("attempt-{}-{part}", rig.key(0))
}

/// W-6, decision 13: the entry's retirement rows are inside the operation
/// area, wrap there and say what E3 says; the request row of a retirement
/// that stayed is reached by Tab where the area scrolls, and its Dismiss is
/// where a click reaches it at the default window, and dismisses it.
#[gpui_kit::test]
async fn retire_rows_hit(cx: &mut TestAppContext) {
    // E-37: polling is off. With it on, the scheduler reads `a`'s stored
    // policy, finds it due and sends a real poll whose answer replaces the
    // attempt injected below, thirty rows and all, whenever it arrives: the
    // rows and the Dismiss below the fold were only there if the answer was
    // late, and Tab brought nothing in view.
    let rig = Rig::new(&[("a", true)], cx);
    rig.schedule(0, cx);
    rig.shell
        .visual(cx)
        .simulate_resize(size(px(1280.), px(840.)));

    // An attempt that reported thirty closed tickets; the loop retired
    // twenty-eight, one did not finish, one is being retired.
    let ids: Vec<ItemId> = (0..30).map(|_| ItemId::generate()).collect();
    let now = rig.clock.now();
    let attempt = PollAttempt {
        root: rig.key(0),
        operation: OperationId::new(),
        answer: Answer::Completed,
        completed_at: Some(now.utc),
        next_eligible_at: Some(now.utc + time::Duration::seconds(300)),
        targets: ids
            .iter()
            .map(|id| AttemptTarget {
                item_id: Some(id.clone()),
                kind: Some(TargetKind::Ticket),
                disposition: Disposition::Closed,
                main: false,
            })
            .collect(),
        index_pending: false,
    };
    let owed = Verdict::Owed {
        attention: None,
        catching_up: true,
    };
    let root = rig.key(0);
    rig.services.scheduler().update(cx, |scheduler, cx| {
        let schedule = scheduler.schedule_mut();
        schedule.after(&now, &attempt);
        for id in &ids {
            schedule.retire_started(&root, TargetKind::Ticket, id);
        }
        for id in &ids[..28] {
            schedule.retire_settled(&root, TargetKind::Ticket, id, &Verdict::Retired);
        }
        schedule.retire_settled(&root, TargetKind::Ticket, &ids[28], &owed);
        cx.notify();
    });

    // The request of a retirement that was refused stays, with Dismiss.
    let stays = ItemId::generate();
    rig.shell.backend.answer_job(
        JobKind::ContextRetire,
        answer(State::Refused, Code::WorktreeNotClean, Removal::None),
    );
    let request = rig.services.scheduler().update(cx, |scheduler, _| {
        scheduler
            .retire(&root, TargetKind::Ticket, &stays, Lane::Background)
            .expect("not quitting")
    });
    rig.shell.until_ended(request, cx).await;
    rig.shell.run(cx);

    // What E3 says, item by item, in the order the retirements began: the
    // first rows are retired, the 29th did not finish, the 30th is still
    // being retired.
    for (i, expected) in [
        (0usize, retired_text(&short(&ids[0]))),
        (27usize, retired_text(&short(&ids[27]))),
        (28usize, unfinished_text(&short(&ids[28]), true)),
        (29usize, retiring_text(&short(&ids[29]))),
    ] {
        let id = entry_id(&rig, &format!("retirement-{i}"));
        assert_eq!(
            rig.shell.label(id, cx).as_deref(),
            Some(expected.as_str()),
            "row {i}"
        );
    }
    let area = rig
        .shell
        .frame(cx, |window, _| window.find("operation-area").bounds());
    for i in [0usize, 27, 28, 29] {
        let row = rig.shell.frame(cx, |window, _| {
            window
                .find(entry_id(&rig, &format!("retirement-{i}")))
                .bounds()
        });
        assert!(
            row.left() >= area.left() && row.right() <= area.right(),
            "row {i} wraps inside the area: {row:?} in {area:?}"
        );
    }

    // The request row's Dismiss is below the fold until Tab brings it in.
    let dismiss = request_id(request, "dismiss");
    assert!(rig.shell.tab_reaches(dismiss.clone(), TABS, cx));
    rig.shell.click(dismiss, cx);
    let state = rig.shell.operations(cx).read_with(cx, |operations, _| {
        operations.request(request).unwrap().state.clone()
    });
    assert!(matches!(state, RequestState::Dismissed), "dismissed");
}
