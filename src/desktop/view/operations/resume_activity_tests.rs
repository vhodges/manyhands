//! A stranded lifecycle's row in Activity's log (Wave 04 Cycle 03b, S9, N25;
//! ruling T4-3): while a resume of the row runs, the item's lifecycle runs
//! elsewhere, so the row reads "Still running…", disabled, and is not
//! counted. Real, over the stranded close of `resume_tests`.

use gpui_kit::TestAppContext;

use super::tests::{close_row, stranded};
use crate::desktop::{
    model::activity::rows::{ActivityRow, RowKey},
    services::JobKind,
    view::{
        harness::{Shell, key},
        lifecycle::STILL_RUNNING,
        operations::RESUME,
    },
};

/// The close's row in the log, refreshed now.
fn pending(shell: &Shell, root: &std::path::Path, cx: &mut TestAppContext) -> ActivityRow {
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.refresh_activity_now(cx));
    let root = key(root);
    shell.workspace.read_with(cx, |workspace, _| {
        workspace
            .activity()
            .rows()
            .iter()
            .find(|row| matches!(&row.row.key, RowKey::Pending { root: held, .. } if *held == root))
            .cloned()
            .expect("the close's pending row")
    })
}

/// T4-3: the stranded close's row offers Resume and counts; with a resume of
/// it in flight (the worker holds it before its first progress, so the row is
/// still listed), the row reads "Still running…", disabled, and counts no
/// more.
#[gpui_kit::test]
async fn a_running_resume_makes_the_stranded_row_still_running(cx: &mut TestAppContext) {
    let (shell, root, _) = stranded(cx);
    let row = close_row(&shell, cx).await;
    let idle = pending(&shell, &root, cx);
    let button = idle.row.button.clone().expect("a button");
    assert_eq!((button.label.as_str(), button.enabled), (RESUME, true));
    assert!(idle.row.attention, "an idle stranded row counts");

    let held = shell.backend.hold_job(JobKind::OperationResume);
    shell.click(("pending-resume", row), cx);
    held.wait_reached();
    let running = pending(&shell, &root, cx);
    let button = running.row.button.clone().expect("a button");
    assert_eq!(
        (button.label.as_str(), button.enabled),
        (STILL_RUNNING, false)
    );
    assert!(
        !running.row.attention,
        "a lifecycle that runs counts nothing"
    );
    held.release();
}
