//! P14's rows over an add's later attempts and its ends (Wave 04 Cycle 05a
//! design C2; B10; R-25, R-28; K-10, K-14): Prepare again then a success,
//! a Partial then Retry from its Activity row, and an add whose discovery
//! is still pending each send one selection; a failed selection leaves the
//! repository registered with its own Activity row; an add that succeeds
//! once the app is finishing sends nothing. The helpers are `share`'s.

use gpui_kit::TestAppContext;
use manyhands::{
    repository::{FailurePoint, RepositoryService},
    results::{Outcome, RecoveryActionKind, ResultCode},
};

use super::{
    share::{choose, ended, expect, origin_shell, press_add, selections},
    support::*,
};
use crate::desktop::{
    model::activity::rows::{ActivityAction, RowKey},
    services::{
        JobKind,
        requests::{DesktopRequestId, RequestState},
    },
    view::{
        activity_support::{act, label_of, refresh, row as log_row},
        confirm::PREPARE_AGAIN,
        harness::{Shell, key, wait_until},
        workspace::QuitPhase,
    },
};

pub(super) async fn rows(table: &mut Table, cx: &mut TestAppContext) {
    prepare_again(table, cx).await;
    partial_then_retry(table, cx).await;
    discovery_pending(table, cx).await;
    failed_selection(table, cx).await;
    finishing(table, cx).await;
}

/// The outcome and code the request `id` finished with, if it has.
fn finished(
    shell: &Shell,
    id: DesktopRequestId,
    cx: &mut TestAppContext,
) -> Option<(Outcome, ResultCode)> {
    shell.operations(cx).read_with(cx, |operations, _| {
        match operations.request(id).map(|request| &request.state) {
            Some(RequestState::Finished(outcome, code, ..)) => Some((*outcome, *code)),
            _ => None,
        }
    })
}

/// Prepare again, then a success: one selection. The preview goes stale as
/// a commit lands on the folder's checked-out branch.
async fn prepare_again(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, plain) = origin_shell(cx);
    press_plus(&shell, cx);
    let row = "P14 Prepare again, then a success";
    if !opened(table, &shell, row, cx) || !checked_folder(&shell, &plain, cx) {
        return;
    }
    choose(&shell, Some("origin"), cx);
    if !press_add(&shell, cx) {
        table.fact(&format!("{row} (its preview)"), false, "the add's preview");
        return;
    }
    shell.fixture.commit_file(&plain, "late.txt", b"late\n");
    let Some(id) = confirm_and_end(&shell, cx).await else {
        table.fact(&format!("{row} (its request)"), false, "the add's request");
        return;
    };
    settle(&shell, cx);
    let offered = shell.label("confirm-prepare-again", cx);
    table.fact(
        &format!("{row} (Prepare again offered)"),
        offered.as_deref() == Some(PREPARE_AGAIN),
        &format!("Prepare again (it is {offered:?})"),
    );
    expect(table, &shell, &format!("{row} (none yet)"), &plain, &[], cx);
    if offered.is_none() {
        return;
    }
    shell.click("confirm-prepare-again", cx);
    if !shell.present("confirm-ok", cx) {
        table.fact(&format!("{row} (a new preview)"), false, "the new preview");
        return;
    }
    shell.click("confirm-ok", cx);
    shell
        .until(cx, move |operations| {
            operations.request(id).is_some_and(|request| {
                request.attempt == 2 && matches!(request.state, RequestState::Finished(..))
            })
        })
        .await;
    settle(&shell, cx);
    expect(table, &shell, row, &plain, &[Some("origin")], cx);
    ended(&shell, &[&plain], cx).await;
}

/// A Partial, then Retry from its Activity row: nothing is sent for the
/// Partial, one selection after the Retry's success.
async fn partial_then_retry(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, plain) = origin_shell(cx);
    press_plus(&shell, cx);
    let row = "P14 a Partial, then Retry";
    if !opened(table, &shell, row, cx) || !checked_folder(&shell, &plain, cx) {
        return;
    }
    choose(&shell, Some("origin"), cx);
    if !press_add(&shell, cx) {
        table.fact(&format!("{row} (its preview)"), false, "the add's preview");
        return;
    }
    shell.backend.replace_service(
        RepositoryService::open_at_with_failure_point_for_testing(
            shell.fixture.data_dir(),
            FailurePoint::AfterInitializationCommit,
        )
        .unwrap(),
    );
    let Some(id) = confirm_and_end(&shell, cx).await else {
        table.fact(&format!("{row} (its request)"), false, "the add's request");
        return;
    };
    settle(&shell, cx);
    let retry = shell.operations(cx).read_with(cx, |operations, _| {
        let request = operations.request(id)?;
        let RequestState::Finished(outcome, code, _, recovery, _) = &request.state else {
            return None;
        };
        let retry = recovery
            .iter()
            .position(|action| action.action == RecoveryActionKind::RequestRetry);
        (*outcome == Outcome::Partial && *code != ResultCode::DiscoveryPending)
            .then_some(retry)
            .flatten()
    });
    table.fact(
        &format!("{row} (a Partial offering Retry)"),
        retry.is_some(),
        "a Partial, not a pending discovery, with Retry",
    );
    expect(table, &shell, &format!("{row} (none yet)"), &plain, &[], cx);
    let Some(retry) = retry else {
        return;
    };
    shell
        .backend
        .replace_service(RepositoryService::open_at(shell.fixture.data_dir()).unwrap());
    let entry = RowKey::Request(id);
    refresh(&shell, cx);
    let label = label_of(&shell, &entry, &ActivityAction::Recovery(id, retry), cx);
    act(&shell, &entry, &label, cx);
    shell
        .until(cx, move |operations| {
            operations.request(id).is_some_and(|request| {
                request.attempt == 2 && matches!(request.state, RequestState::Finished(..))
            })
        })
        .await;
    settle(&shell, cx);
    expect(table, &shell, row, &plain, &[Some("origin")], cx);
    ended(&shell, &[&plain], cx).await;
}

/// An add whose discovery is still pending (`Finished(Partial,
/// DiscoveryPending)`) has added the repository: one selection.
async fn discovery_pending(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, plain) = origin_shell(cx);
    press_plus(&shell, cx);
    let row = "P14 an add whose discovery is pending";
    if !opened(table, &shell, row, cx) || !checked_folder(&shell, &plain, cx) {
        return;
    }
    choose(&shell, Some("origin"), cx);
    if !press_add(&shell, cx) {
        table.fact(&format!("{row} (its preview)"), false, "the add's preview");
        return;
    }
    shell.backend.replace_service(
        RepositoryService::open_at_with_failure_point_for_testing(
            shell.fixture.data_dir(),
            FailurePoint::BeforeIndexTransactionCommit,
        )
        .unwrap(),
    );
    // The selection waits until the service in use has no failure point.
    let held = shell.backend.hold_job(JobKind::RemoteSelect);
    let Some(id) = confirm_and_end(&shell, cx).await else {
        table.fact(&format!("{row} (its request)"), false, "the add's request");
        return;
    };
    settle(&shell, cx);
    let answer = finished(&shell, id, cx);
    table.fact(
        &format!("{row} (its answer)"),
        answer == Some((Outcome::Partial, ResultCode::DiscoveryPending)),
        &format!("partial, discovery pending (it is {answer:?})"),
    );
    expect(table, &shell, row, &plain, &[Some("origin")], cx);
    shell
        .backend
        .replace_service(RepositoryService::open_at(shell.fixture.data_dir()).unwrap());
    held.release();
    ended(&shell, &[], cx).await;
}

/// K-14, decision 3: a selection that fails (the remote went away after
/// the check) leaves the repository registered, and its Activity row says
/// so with its code.
async fn failed_selection(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, plain) = origin_shell(cx);
    press_plus(&shell, cx);
    let row = "P14 a failed selection";
    if !opened(table, &shell, row, cx) || !checked_folder(&shell, &plain, cx) {
        return;
    }
    choose(&shell, Some("origin"), cx);
    git2::Repository::open(&plain)
        .unwrap()
        .remote_delete("origin")
        .unwrap();
    if !press_add(&shell, cx) {
        table.fact(&format!("{row} (its preview)"), false, "the add's preview");
        return;
    }
    confirm_and_end(&shell, cx).await;
    settle(&shell, cx);
    expect(
        table,
        &shell,
        &format!("{row} (sent)"),
        &plain,
        &[Some("origin")],
        cx,
    );
    let Some(id) = selections(&shell, cx).first().copied() else {
        return;
    };
    shell.until_ended(id, cx).await;
    let code = finished(&shell, id, cx)
        .filter(|(outcome, _)| *outcome != Outcome::Success)
        .map(|(_, code)| code);
    let registered: Vec<String> = shell
        .fixture
        .service()
        .list_repositories()
        .unwrap()
        .items
        .into_iter()
        .map(|summary| summary.root)
        .collect();
    table.fact(
        &format!("{row} (failed, the repository registered)"),
        code.is_some() && registered.contains(&key(&plain)),
        &format!("a failure, plain registered (code {code:?}, registered {registered:?})"),
    );
    refresh(&shell, cx);
    let detail = log_row(&shell, &RowKey::Request(id), cx).map(|entry| entry.row.detail);
    let says = code.is_some_and(|code| {
        detail.as_ref().is_some_and(|detail| {
            detail
                .iter()
                .any(|line| line.starts_with(&format!("{}: ", code.as_str())))
        })
    });
    table.fact(
        &format!("{row} (its Activity row)"),
        says,
        &format!("the selection's row with its code (detail {detail:?})"),
    );
    ended(&shell, &[&plain], cx).await;
}

/// K-10: the add succeeds once quitting has begun: nothing is sent. The
/// add is held once its library call has returned, so quitting, which
/// cancels a call still running, waits for its success.
async fn finishing(table: &mut Table, cx: &mut TestAppContext) {
    let (shell, plain) = origin_shell(cx);
    press_plus(&shell, cx);
    let row = "P14 the add succeeds while the app is finishing";
    if !opened(table, &shell, row, cx) || !checked_folder(&shell, &plain, cx) {
        return;
    }
    choose(&shell, Some("origin"), cx);
    if !press_add(&shell, cx) {
        table.fact(&format!("{row} (its preview)"), false, "the add's preview");
        return;
    }
    let held = shell.backend.hold_job_return(JobKind::RepoEnable);
    shell.click("confirm-ok", cx);
    held.wait_reached();
    shell.keys("escape", cx);
    let id = enables(&shell, cx)[0];
    shell.keys("secondary-q", cx);
    let phase = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.quit_phase());
    table.fact(
        &format!("{row} (quitting waits for the add)"),
        phase == QuitPhase::Finishing(id),
        &format!("quitting waits (it is {phase:?})"),
    );
    held.release();
    wait_until(cx, &shell.workspace, |workspace, _| {
        workspace.quit_phase() == QuitPhase::Quit
    })
    .await;
    shell.run(cx);
    let answer = finished(&shell, id, cx);
    table.fact(
        &format!("{row} (added)"),
        answer == Some((Outcome::Success, ResultCode::Ok)),
        &format!("the add succeeded (it answered {answer:?})"),
    );
    expect(table, &shell, row, &plain, &[], cx);
}
