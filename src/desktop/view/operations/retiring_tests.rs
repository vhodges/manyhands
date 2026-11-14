//! The request row of a retirement that stayed (D6 Task 11b, design E3, W-6,
//! decision 13; Wave 04 Cycle 03b, R-51): a refused retirement's request
//! stays in Activity, and its Dismiss is in the row's menu. Retire… and
//! Resume are `retire_tests`'s. The attempt's retirement lines are no longer
//! drawn (R-51): what a retirement says is the request's row and, once it
//! was dismissed automatically, an S8 event; the model's tests hold them.

use gpui_kit::TestAppContext;
use manyhands::{
    canonical::ItemId,
    repository::{ContextRetirementState as State, RetirementRemoval as Removal},
    results::ResultCode as Code,
};

use crate::desktop::{
    model::{activity::copy::DISMISS, activity::rows::RowKey, polling::TargetKind},
    services::{
        JobKind,
        requests::{Lane, RequestState},
    },
    test_support::retire::answer,
    view::{
        activity_support::{choose_in_menu, offered, refresh, row},
        settings::polling_support::Rig,
    },
};

/// W-6, decision 13: the request row of a retirement that stayed is reached
/// by the arrows, and its Dismiss, chosen from its menu, dismisses it.
#[gpui_kit::test]
async fn retire_rows_hit(cx: &mut TestAppContext) {
    // E-37: polling is off. With it on, the scheduler reads `a`'s stored
    // policy, finds it due and sends a real poll whose answer replaces the
    // attempt a test reports whenever it arrives.
    let rig = Rig::new(&[("a", true)], cx);
    rig.schedule(0, cx);
    let root = rig.key(0);

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
    refresh(&rig.shell, cx);
    let at = RowKey::Request(request);
    let shown = row(&rig.shell, &at, cx).expect("the refused retirement's row stays");
    assert!(
        shown.row.menu.iter().any(|offer| offer.label == DISMISS),
        "Dismiss is in its menu: {:?}",
        shown.row.menu
    );
    assert!(offered(&rig.shell, &at, DISMISS, cx).is_some_and(|offer| offer.enabled));

    // The arrows reach the row, `shift-f10` opens its menu, and Dismiss
    // dismisses the request.
    choose_in_menu(&rig.shell, &at, DISMISS, cx);
    let state = rig.shell.operations(cx).read_with(cx, |operations, _| {
        operations.request(request).unwrap().state.clone()
    });
    assert!(matches!(state, RequestState::Dismissed), "dismissed");
}
