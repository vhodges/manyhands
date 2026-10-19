//! The shell: its regions and keys, the repository selector, and
//! quitting. The Quit action and the window's close request share one
//! handler, which waits for a running job.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use gpui_kit::{
    Entity, TestAppContext, VisualTestContext, Window, WindowHandle, component::Root,
    test::TestWindowExt,
};
use manyhands::{
    repository::{ConfigurationState, RepositoryService},
    results::ResultCode,
};

use super::{QuitPhase, REGIONS, Region, Workspace};
use crate::desktop::{
    keys::{self, Quit},
    model::{list_state::index_label, navigation::Resolution},
    open_test_window_with,
    services::{
        JobKind, ReadKind,
        requests::{DesktopRequestId, Intent, Lane, RequestState},
    },
    test_support::{
        delaying::DelayingBackend,
        fixtures::{Fixture, refresh_through, remove_registration},
        snapshot::RepositorySnapshot,
    },
    view::{harness::wait_until, sidebar::configuration_label},
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
    wait_until(&visual, &setup.workspace, |workspace, _| {
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
    // The worker must not outlive the test: wait for the job's end.
    let operations = setup.workspace.read_with(&visual, |workspace, _| {
        workspace.services().operations().clone()
    });
    wait_until(&visual, &operations, move |operations, _| {
        matches!(
            operations.request(id).map(|request| &request.state),
            Some(RequestState::Finished(..))
        )
    })
    .await;
}

/// A window over a fixture with two repositories, `a` and `b`, whose
/// startup reads have run.
struct Shell {
    window: WindowHandle<Root>,
    workspace: Entity<Workspace>,
    backend: Arc<DelayingBackend>,
    a: PathBuf,
    b: PathBuf,
    fixture: Fixture,
}

impl Shell {
    fn new(cx: &mut TestAppContext) -> Self {
        cx.update(gpui_kit::init);
        cx.update(keys::bind);
        let fixture = Fixture::new();
        let a = fixture.create_repository("a");
        let b = fixture.create_repository("b");
        let (window, backend) = cx.update(|cx| open_test_window_with(&fixture, cx));
        let workspace = window
            .update(cx, |root, _, _| {
                root.view().clone().downcast::<Workspace>().unwrap()
            })
            .unwrap();
        let shell = Self {
            window,
            workspace,
            backend,
            a,
            b,
            fixture,
        };
        shell.visual(cx).run_until_parked();
        shell
    }

    fn visual(&self, cx: &TestAppContext) -> VisualTestContext {
        VisualTestContext::from_window(self.window.into(), cx)
    }

    /// Draws a frame and runs `f` over the window.
    fn frame<R>(
        &self,
        cx: &mut TestAppContext,
        f: impl FnOnce(&mut Window, &mut gpui_kit::App) -> R,
    ) -> R {
        let mut visual = self.visual(cx);
        visual.update(|window, cx| {
            window.render_frame(cx);
            f(window, cx)
        })
    }

    /// Clicks the element `id` and lets the reads it starts finish.
    fn click(&self, id: impl Into<gpui_kit::ElementId>, cx: &mut TestAppContext) {
        let id = id.into();
        self.frame(cx, |window, cx| window.click(id, cx));
        self.visual(cx).run_until_parked();
    }

    fn keys(&self, keystrokes: &str, cx: &mut TestAppContext) {
        let mut visual = self.visual(cx);
        visual.simulate_keystrokes(keystrokes);
        visual.run_until_parked();
    }

    fn resolution(&self, root: &Path, cx: &mut TestAppContext) -> Resolution {
        let key = key(root);
        self.workspace.read_with(cx, |workspace, _| {
            workspace
                .navigation()
                .entry(&key)
                .expect("a listed repository")
                .resolution
                .clone()
        })
    }

    fn selected(&self, cx: &mut TestAppContext) -> Option<String> {
        self.workspace.read_with(cx, |workspace, _| {
            workspace.navigation().selected().cloned()
        })
    }

    fn label(&self, id: impl Into<gpui_kit::ElementId>, cx: &mut TestAppContext) -> Option<String> {
        let id = id.into();
        self.frame(cx, |window, _| {
            window
                .try_find(id)
                .and_then(|element| element.label().map(str::to_owned))
        })
    }

    fn present(&self, id: impl Into<gpui_kit::ElementId>, cx: &mut TestAppContext) -> bool {
        let id = id.into();
        self.frame(cx, |window, _| window.try_find(id).is_some())
    }
}

fn key(root: &Path) -> String {
    root.to_str().unwrap().to_owned()
}

/// Row `index` of the repository list, in the read's order.
fn row(index: usize) -> (&'static str, usize) {
    ("repo-row", index)
}

#[gpui_kit::test]
fn the_fixtures_repositories_are_listed_by_name(cx: &mut TestAppContext) {
    let shell = Shell::new(cx);
    let listed = shell.fixture.service().list_repositories().unwrap();
    let roots: Vec<String> = listed.items.iter().map(|item| item.root.clone()).collect();
    assert_eq!(roots, [key(&shell.a), key(&shell.b)]);
    assert_eq!(shell.label(row(0), cx).as_deref(), Some("a"));
    assert_eq!(shell.label(row(1), cx).as_deref(), Some("b"));
    assert!(!shell.present(row(2), cx), "no row the read did not give");
    // Each row carries its full root as secondary text.
    assert_eq!(shell.label(("repo-root", 0usize), cx), Some(key(&shell.a)));
}

#[gpui_kit::test]
fn selecting_a_repository_resolves_it_and_shows_its_configuration_and_index(
    cx: &mut TestAppContext,
) {
    let shell = Shell::new(cx);
    assert!(matches!(
        shell.resolution(&shell.a, cx),
        Resolution::Unknown
    ));
    shell.click(row(0), cx);

    assert_eq!(shell.selected(cx), Some(key(&shell.a)));
    let expected = shell
        .fixture
        .service()
        .resolve_repository(&shell.a)
        .unwrap();
    assert!(matches!(
        shell.resolution(&shell.a, cx),
        Resolution::Resolved(resolved) if resolved == expected
    ));
    let summary = shell
        .fixture
        .service()
        .list_repositories()
        .unwrap()
        .items
        .remove(0);
    assert_eq!(summary.configuration.state, ConfigurationState::Valid);
    assert_eq!(shell.label("selection-name", cx).as_deref(), Some("a"));
    assert_eq!(
        shell.label("selection-configuration", cx).as_deref(),
        Some(configuration_label(summary.configuration.state))
    );
    assert_eq!(
        shell.label("selection-index", cx).as_deref(),
        Some(index_label(&summary.index))
    );
    assert!(!shell.present("selection-unavailable", cx));
}

/// Review Focus 2: the selected repository's folder is renamed away and
/// back. Selection shows Unavailable with Retry and Remove; Retry resolves
/// once the folder is back.
#[gpui_kit::test]
fn a_renamed_away_repository_is_unavailable_until_retry_finds_it_again(cx: &mut TestAppContext) {
    let shell = Shell::new(cx);
    shell.click(row(0), cx);
    assert!(matches!(
        shell.resolution(&shell.a, cx),
        Resolution::Resolved(_)
    ));

    let away = shell.a.with_file_name("a-away");
    fs::rename(&shell.a, &away).unwrap();
    shell.click(row(0), cx);
    assert!(matches!(
        shell.resolution(&shell.a, cx),
        Resolution::Unavailable(ResultCode::InvalidPath)
    ));
    assert_eq!(
        shell.label("selection-unavailable", cx).as_deref(),
        Some(ResultCode::InvalidPath.message())
    );
    assert!(shell.present("selection-retry", cx));
    assert!(shell.present("selection-remove", cx));
    assert!(!shell.present("selection-configuration", cx));
    // The row says so in text, not by color alone.
    assert!(shell.present(("repo-unavailable", 0usize), cx));

    fs::rename(&away, &shell.a).unwrap();
    shell.click("selection-retry", cx);
    assert!(matches!(
        shell.resolution(&shell.a, cx),
        Resolution::Resolved(_)
    ));
    assert!(!shell.present("selection-unavailable", cx));
    assert!(!shell.present(("repo-unavailable", 0usize), cx));
    assert!(shell.present("selection-configuration", cx));
}

/// Track rule 4: a second service over the same data directory removes
/// the selected repository's registration. The desktop's next read shows
/// it unavailable as not registered, not what it showed before.
#[gpui_kit::test]
fn a_registration_removed_by_a_second_actor_shows_unavailable(cx: &mut TestAppContext) {
    let shell = Shell::new(cx);
    shell.click(row(0), cx);
    assert!(shell.present("selection-configuration", cx));

    let second = RepositoryService::open_at(shell.fixture.data_dir()).unwrap();
    remove_registration(&second, &shell.a);

    shell.click(row(0), cx);
    assert!(matches!(
        shell.resolution(&shell.a, cx),
        Resolution::Unavailable(ResultCode::RepositoryNotRegistered)
    ));
    assert_eq!(
        shell.label("selection-unavailable", cx).as_deref(),
        Some(ResultCode::RepositoryNotRegistered.message())
    );
    assert!(!shell.present("selection-configuration", cx));
    assert!(!shell.present("selection-index", cx));
}

/// Review Focus 1: B is selected while A's resolution is held; releasing
/// A leaves B selected and shown.
#[gpui_kit::test]
fn a_held_resolution_of_a_never_replaces_b(cx: &mut TestAppContext) {
    let shell = Shell::new(cx);
    // Queued after the startup reads, so it holds A's resolution.
    let held = shell.backend.hold_read(ReadKind::Resolve);
    shell.click(row(0), cx);
    assert!(matches!(
        shell.resolution(&shell.a, cx),
        Resolution::Unknown
    ));
    shell.click(row(1), cx);
    assert!(matches!(
        shell.resolution(&shell.b, cx),
        Resolution::Resolved(_)
    ));

    // A's read may already be dropped with its task; either way it is
    // let go.
    let _ = held.send(());
    shell.visual(cx).run_until_parked();
    assert_eq!(shell.selected(cx), Some(key(&shell.b)));
    assert!(matches!(
        shell.resolution(&shell.b, cx),
        Resolution::Resolved(_)
    ));
    assert!(matches!(
        shell.resolution(&shell.a, cx),
        Resolution::Unknown
    ));
    assert_eq!(shell.label("selection-name", cx).as_deref(), Some("b"));
}

/// The one focused region, which must also be the one drawing a focus
/// ring.
fn focused_region(shell: &Shell, cx: &mut TestAppContext) -> Region {
    shell.frame(cx, |window, _| {
        let focused: Vec<Region> = REGIONS
            .into_iter()
            .filter(|region| window.find(region.id()).focused() == Some(true))
            .collect();
        assert_eq!(focused.len(), 1, "exactly one focused region");
        for region in REGIONS {
            assert_eq!(
                window.try_find(region.ring_id()).is_some(),
                region == focused[0],
                "only the focused region draws its ring ({region:?})"
            );
        }
        let ring = window.find(focused[0].ring_id());
        assert!(ring.visible(), "the focus ring is visible");
        focused[0]
    })
}

#[gpui_kit::test]
fn f6_cycles_the_regions_and_secondary_digits_focus_each(cx: &mut TestAppContext) {
    let shell = Shell::new(cx);
    assert_eq!(focused_region(&shell, cx), Region::Repositories);

    for expected in [
        Region::Navigation,
        Region::Content,
        Region::Operations,
        Region::Repositories,
    ] {
        shell.keys("f6", cx);
        assert_eq!(focused_region(&shell, cx), expected);
    }
    for expected in [
        Region::Operations,
        Region::Content,
        Region::Navigation,
        Region::Repositories,
    ] {
        shell.keys("shift-f6", cx);
        assert_eq!(focused_region(&shell, cx), expected);
    }
    for (keystroke, expected) in [
        ("secondary-3", Region::Content),
        ("secondary-1", Region::Repositories),
        ("secondary-4", Region::Operations),
        ("secondary-2", Region::Navigation),
    ] {
        shell.keys(keystroke, cx);
        assert_eq!(focused_region(&shell, cx), expected, "{keystroke}");
    }
}

/// Selecting and browsing reads only: refs, worktrees, status and the
/// Git configuration of both repositories are unchanged.
#[gpui_kit::test]
fn selecting_and_browsing_leave_the_repositories_untouched(cx: &mut TestAppContext) {
    let shell = Shell::new(cx);
    let before = [
        RepositorySnapshot::take(&shell.a),
        RepositorySnapshot::take(&shell.b),
    ];

    shell.click(row(0), cx);
    shell.click(row(1), cx);
    shell.keys("f6 f6 f6 secondary-1", cx);
    shell.click(row(0), cx);
    shell.keys("secondary-3", cx);

    let after = [
        RepositorySnapshot::take(&shell.a),
        RepositorySnapshot::take(&shell.b),
    ];
    assert_eq!(before, after);
}

/// Track rule 4: a configuration the selected repository's index has not
/// seen is committed, and a second service refreshes the index. Selecting
/// it again, or Retry, re-reads the list, so the configuration shown is
/// the library's current one, not the one read at startup.
#[gpui_kit::test]
fn reselecting_or_retry_shows_a_configuration_refreshed_by_a_second_actor(cx: &mut TestAppContext) {
    let shell = Shell::new(cx);
    let second = RepositoryService::open_at(shell.fixture.data_dir()).unwrap();
    let valid = Some(configuration_label(ConfigurationState::Valid));
    let invalid = Some(configuration_label(ConfigurationState::Invalid));

    for (index, root) in [(0, &shell.a), (1, &shell.b)] {
        shell.click(row(index), cx);
        assert_eq!(shell.label("selection-configuration", cx).as_deref(), valid);

        shell
            .fixture
            .commit_file(root, ".manyhands/config.toml", b"format_version = 99\n");
        refresh_through(&second, root);
        let summary = second.list_repositories().unwrap().items.remove(index);
        assert_eq!(summary.configuration.state, ConfigurationState::Invalid);
        // Until the desktop reads again it still shows what it read.
        assert_eq!(shell.label("selection-configuration", cx).as_deref(), valid);

        if index == 0 {
            shell.click(row(0), cx);
        } else {
            shell.workspace.update(cx, |workspace, cx| {
                workspace.retry_resolution(key(root), cx)
            });
            shell.visual(cx).run_until_parked();
        }
        assert_eq!(shell.selected(cx), Some(key(root)));
        assert_eq!(
            shell.label("selection-configuration", cx).as_deref(),
            invalid
        );
        assert_eq!(
            shell.label("selection-index", cx).as_deref(),
            Some(index_label(&summary.index))
        );
    }
}
