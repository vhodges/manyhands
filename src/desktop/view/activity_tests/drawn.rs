//! P18, from a drawn button and from a row's menu (Wave 04 Cycle 03b):
//! Cancel from its button, and Dismiss and Resume from the menu, send what
//! the seam's choice of the same offer sends (`cases`). Abandon… from the
//! menu, which opens a dialog, is P20's.

use gpui_kit::TestAppContext;

use super::list_support::*;
use crate::desktop::{
    model::activity::{copy::DISMISS, rows::RowKey},
    services::{
        JobKind,
        requests::{Intent, Lane, RequestState},
    },
    view::{
        activity_support::{index_of, request, to_row},
        repositories::tests::support::choose,
    },
};

pub(super) async fn drawn(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let rows = rows(cx);
    let shell = &rows.shell;
    settle(shell, cx);

    // Dismiss, from the menu of a finished request's row.
    let RowKey::Request(plain) = rows.plain.clone() else {
        panic!("a request's row");
    };
    to_row(shell, &rows.plain, cx);
    shell.keys("shift-f10", cx);
    choose(shell, DISMISS, cx);
    assert!(matches!(
        request(shell, plain, cx).state,
        RequestState::Dismissed
    ));

    // Resume, from the menu of a pending row: one `operation resume`.
    let a = rows.pending[0].clone();
    to_row(shell, &a, cx);
    shell.keys("shift-f10", cx);
    choose(shell, "Resume", cx);
    assert_eq!(resumes(shell, OP_A, cx), 1);

    // Cancel, from the drawn button of a queued request, sent through the
    // worker behind a job that is held.
    let worker = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().worker().clone());
    let gate = shell.backend.hold_job(JobKind::IndexRefresh);
    let refresh = || Intent::IndexRefresh {
        root: rows.root.clone(),
    };
    let first = worker.submit(refresh(), Lane::User, None, None);
    gate.wait_reached();
    let queued = worker.submit(refresh(), Lane::User, None, None);
    shell.run(cx);
    let key = RowKey::Request(queued);
    to_row(shell, &key, cx);
    let at = index_of(shell, &key, cx);
    shell.click(("activity-button", at), cx);
    assert!(matches!(
        request(shell, queued, cx).state,
        RequestState::Removed
    ));
    gate.release();
    shell.until_ended(first, cx).await;
}
