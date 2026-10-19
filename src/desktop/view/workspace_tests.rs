//! Quitting: the Quit action and the window's close request share one
//! handler, which waits for a running job.

use std::{path::PathBuf, sync::Arc};

use gpui_kit::{
    Entity, TestAppContext, VisualTestContext, WindowHandle, component::Root, test::TestWindowExt,
};

use super::{QuitPhase, Workspace};
use crate::desktop::{
    keys::{self, Quit},
    open_test_window_with,
    services::{
        JobKind,
        requests::{DesktopRequestId, Intent, Lane},
    },
    test_support::{delaying::DelayingBackend, fixtures::Fixture},
};

struct Setup {
    window: WindowHandle<Root>,
    workspace: Entity<Workspace>,
    backend: Arc<DelayingBackend>,
    root: PathBuf,
    // Kept for as long as the window reads through it.
    _fixture: Fixture,
}

fn setup(cx: &mut TestAppContext) -> Setup {
    cx.update(gpui_kit::init);
    cx.update(keys::bind);
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let (window, backend) = cx.update(|cx| open_test_window_with(&fixture, cx));
    let workspace = window
        .update(cx, |root, _, _| {
            root.view().clone().downcast::<Workspace>().unwrap()
        })
        .unwrap();
    Setup {
        window,
        workspace,
        backend,
        root,
        _fixture: fixture,
    }
}

fn phase(workspace: &Entity<Workspace>, cx: &mut TestAppContext) -> QuitPhase {
    workspace.read_with(cx, |workspace, _| workspace.quit_phase())
}

/// Submits a refresh through the window's own worker.
fn submit_refresh(setup: &Setup, cx: &mut TestAppContext) -> DesktopRequestId {
    setup.workspace.read_with(cx, |workspace, _| {
        workspace.services().worker().submit(
            Intent::IndexRefresh {
                root: setup.root.clone(),
            },
            Lane::User,
            None,
            None,
        )
    })
}

#[gpui_kit::test]
fn quit_with_no_job_quits(cx: &mut TestAppContext) {
    let setup = setup(cx);
    let mut visual = VisualTestContext::from_window(setup.window.into(), cx);
    visual.run_until_parked();
    assert_eq!(phase(&setup.workspace, &mut visual), QuitPhase::Running);

    visual.simulate_keystrokes("secondary-q");
    assert_eq!(phase(&setup.workspace, &mut visual), QuitPhase::Quit);
}

#[gpui_kit::test]
fn closing_the_window_with_no_job_is_allowed(cx: &mut TestAppContext) {
    let setup = setup(cx);
    let mut visual = VisualTestContext::from_window(setup.window.into(), cx);
    visual.run_until_parked();
    assert!(visual.simulate_close());
    assert_eq!(phase(&setup.workspace, &mut visual), QuitPhase::Quit);
}

/// Quit and close with a held job both refuse, show "Finishing" and
/// Quit now, and quit once the job returns.
#[gpui_kit::test]
async fn quitting_with_a_held_job_waits_for_it(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let setup = setup(cx);
    let held = setup.backend.hold_job(JobKind::IndexRefresh);
    let id = submit_refresh(&setup, cx);
    held.wait_reached();

    let mut visual = VisualTestContext::from_window(setup.window.into(), cx);
    visual.dispatch_action(Quit);
    visual.run_until_parked();
    assert_eq!(
        phase(&setup.workspace, &mut visual),
        QuitPhase::Finishing(id)
    );
    visual.update(|window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("quit-finishing").is_some());
        assert!(window.try_find("quit-now").is_some());
    });
    assert_eq!(
        setup
            .workspace
            .read_with(&visual, |workspace, cx| workspace.finishing_text(cx)),
        Some("Finishing Refresh index".to_owned())
    );
    // The close request goes through the same handler.
    assert!(!visual.simulate_close());
    assert_eq!(
        phase(&setup.workspace, &mut visual),
        QuitPhase::Finishing(id)
    );

    held.release();
    visual
        .condition(&setup.workspace, |workspace, _| {
            workspace.quit_phase() == QuitPhase::Quit
        })
        .await;
}

/// Quit now does not wait: the job is still held when the app quits.
#[gpui_kit::test]
async fn quit_now_quits_while_the_job_is_held(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let setup = setup(cx);
    let held = setup.backend.hold_job(JobKind::IndexRefresh);
    let id = submit_refresh(&setup, cx);
    held.wait_reached();

    let mut visual = VisualTestContext::from_window(setup.window.into(), cx);
    visual.dispatch_action(Quit);
    visual.run_until_parked();
    assert_eq!(
        phase(&setup.workspace, &mut visual),
        QuitPhase::Finishing(id)
    );
    visual.update(|window, cx| {
        window.render_frame(cx);
        window.click("quit-now", cx);
    });
    visual.run_until_parked();
    assert_eq!(phase(&setup.workspace, &mut visual), QuitPhase::Quit);
    held.release();
}
