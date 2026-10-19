use std::path::Path;

use super::test_support::{fixtures::Fixture, git_home};

#[test]
fn a_created_repository_is_the_one_registration() {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");

    let listed = fixture.service().list_repositories().unwrap();
    assert_eq!(listed.items.len(), 1);
    assert_eq!(Path::new(&listed.items[0].root), root);
    assert_ne!(fixture.home(), fixture.data_dir());
}

#[test]
fn a_file_committed_by_the_fixture_is_refreshed_through_the_library() {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");

    fixture.commit_file(&root, "notes/plain.txt", b"not an item\n");
    fixture.refresh(&root);

    let repository = git2::Repository::open(&root).unwrap();
    let head = repository.head().unwrap().peel_to_commit().unwrap();
    assert_eq!(head.message(), Some("Add notes/plain.txt"));
}

#[test]
fn git_is_isolated_before_any_fixture_is_made() {
    let home = git_home::isolated();
    assert_eq!(home.files(), [None, None, None]);
}

#[cfg(feature = "desktop-test-support")]
mod view {
    use std::{cell::Cell, fs, rc::Rc};

    use futures::{StreamExt, channel::mpsc};
    use gpui_kit::{
        AppContext, Entity, TestAppContext, VisualTestContext, component::WindowExt,
        test::TestWindowExt,
    };
    use manyhands::{
        repository::{
            CancellationToken, IndexRefreshInput, Mutation, MutationCall, MutationDataDto,
            NoProgress, RepositoryService, RequestId, keys::SessionCredentials,
        },
        results::{Envelope, ResultCode},
    };

    use super::super::{
        Opener, StartupFailureView, start,
        test_support::fixtures::{Fixture, NoPassphrase},
        view::workspace::Workspace,
    };

    #[gpui_kit::test]
    fn root_is_the_first_entity_of_the_main_window(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let window = cx.update(super::super::open_test_window);
        // Root's entity must be created before the workspace's.
        let (root_id, workspace_id, is_workspace) = window
            .update(cx, |root, _, cx| {
                (
                    cx.entity_id(),
                    root.view().entity_id(),
                    root.view().clone().downcast::<Workspace>().is_ok(),
                )
            })
            .unwrap();
        assert!(is_workspace, "the main window's Root holds the workspace");
        assert!(root_id < workspace_id);
    }

    /// The test window reads through the fixture's own service, not a
    /// second one opened beside it.
    #[gpui_kit::test]
    fn the_test_window_reads_through_the_fixture_service(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let window = cx.update(super::super::open_test_window);
        let workspace = window
            .update(cx, |root, _, _| {
                root.view().clone().downcast::<Workspace>().unwrap()
            })
            .unwrap();
        cx.update(|cx| {
            let fixtures = cx.global::<super::super::TestFixtures>();
            let fixture = fixtures.0.last().unwrap();
            let services = workspace.read(cx).services();
            assert!(std::ptr::eq(
                services.backend().service(),
                fixture.service()
            ));
        });
    }

    #[gpui_kit::test]
    fn a_dialog_opened_on_the_main_window_is_rendered(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let window = cx.update(super::super::open_test_window);
        let mut visual = VisualTestContext::from_window(window.into(), cx);
        visual.run_until_parked();
        visual.update(|window, cx| window.render_frame(cx));
        assert_eq!(visual.debug_bounds("dialog-layer"), None);

        visual.update(|window, cx| window.open_dialog(cx, |dialog, _, _| dialog.title("Probe")));
        visual.run_until_parked();
        visual.update(|window, cx| window.render_frame(cx));

        assert!(visual.debug_bounds("dialog-layer").is_some());
        visual.update(|window, _| {
            assert!(window.try_find("dialog").is_some(), "a dialog element");
        });
    }

    /// An opener over `data`, counting its calls.
    fn counting_opener(data: std::path::PathBuf, calls: Rc<Cell<usize>>) -> Opener {
        Rc::new(move || {
            calls.set(calls.get() + 1);
            RepositoryService::open_at(&data)
        })
    }

    #[gpui_kit::test]
    fn a_data_directory_that_cannot_open_shows_the_startup_failure(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let parent = tempfile::tempdir().unwrap();
        let data = parent.path().join("data");
        // A regular file where the data directory would be.
        fs::write(&data, b"not a directory").unwrap();
        let calls = Rc::new(Cell::new(0));
        let opener = counting_opener(data.clone(), calls.clone());

        let window = cx.update(|cx| start(opener, cx));
        assert_eq!(calls.get(), 1);
        let (root_id, child_id, message) = window
            .update(cx, |root, _, cx| {
                let failure = root
                    .view()
                    .clone()
                    .downcast::<StartupFailureView>()
                    .expect("the startup failure view");
                (
                    cx.entity_id(),
                    failure.entity_id(),
                    failure.read(cx).message(),
                )
            })
            .unwrap();
        assert!(root_id < child_id);
        assert_eq!(message, super::super::STARTUP_FAILURE_DATA_DIRECTORY);

        let mut visual = VisualTestContext::from_window(window.into(), cx);
        visual.update(|window, cx| {
            window.render_frame(cx);
            assert!(window.try_find("startup-failure-message").is_some());
            assert!(window.try_find("startup-quit").is_some());
            // Still blocked: Retry opens again and fails again.
            window.click("startup-retry", cx);
        });
        visual.run_until_parked();
        assert_eq!(calls.get(), 2);
        let still_failed = window
            .update(cx, |root, _, _| {
                root.view().clone().downcast::<StartupFailureView>().is_ok()
            })
            .unwrap();
        assert!(still_failed);

        // Unblocked: Retry opens the service and shows the workspace,
        // under a Root created before it.
        fs::remove_file(&data).unwrap();
        let mut visual = VisualTestContext::from_window(window.into(), cx);
        visual.update(|window, cx| {
            window.render_frame(cx);
            window.click("startup-retry", cx);
        });
        visual.run_until_parked();
        assert_eq!(calls.get(), 3);
        let (root_id, workspace_id, is_workspace) = window
            .update(cx, |root, _, cx| {
                (
                    cx.entity_id(),
                    root.view().entity_id(),
                    root.view().clone().downcast::<Workspace>().is_ok(),
                )
            })
            .unwrap();
        assert!(is_workspace);
        assert!(root_id < workspace_id);
    }

    /// Two Retry presses before the window's root is replaced open the
    /// service, and start its worker, once.
    #[gpui_kit::test]
    fn a_second_retry_before_the_workspace_opens_does_nothing(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let parent = tempfile::tempdir().unwrap();
        let data = parent.path().join("data");
        fs::write(&data, b"not a directory").unwrap();
        let calls = Rc::new(Cell::new(0));
        let opener = counting_opener(data.clone(), calls.clone());
        let window = cx.update(|cx| start(opener, cx));
        assert_eq!(calls.get(), 1);

        fs::remove_file(&data).unwrap();
        let mut visual = VisualTestContext::from_window(window.into(), cx);
        visual.update(|window, cx| {
            window.render_frame(cx);
            window.click("startup-retry", cx);
            window.click("startup-retry", cx);
        });
        visual.run_until_parked();
        assert_eq!(calls.get(), 2);
        let is_workspace = window
            .update(cx, |root, _, _| {
                root.view().clone().downcast::<Workspace>().is_ok()
            })
            .unwrap();
        assert!(is_workspace);
    }

    enum WorkerEvent {
        Finished(Box<Envelope<MutationDataDto>>),
    }

    #[derive(Default)]
    struct Received {
        finished: Vec<ResultCode>,
    }

    /// The feasibility gap: a real refresh on a thread of its own, whose
    /// `Finished` event reaches a model through a foreground task.
    #[gpui_kit::test]
    async fn a_worker_refresh_reaches_a_model(cx: &mut TestAppContext) {
        cx.executor().allow_parking();
        let fixture = Fixture::new();
        let root = fixture.create_repository("a");
        let service = RepositoryService::open_at(fixture.data_dir()).unwrap();

        let (events, mut receiver) = mpsc::unbounded();
        let worker = std::thread::spawn(move || {
            let envelope = service.execute(
                MutationCall {
                    request_id: RequestId::new(),
                    confirmation: None,
                    mutation: Mutation::IndexRefresh(IndexRefreshInput { root }),
                },
                &mut SessionCredentials::new(NoPassphrase),
                &mut NoProgress,
                &CancellationToken::new(),
            );
            events
                .unbounded_send(WorkerEvent::Finished(Box::new(envelope)))
                .unwrap();
        });

        let model: Entity<Received> = cx.new(|_| Received::default());
        let drained = model.clone();
        let drain = cx.spawn(|mut cx| async move {
            while let Some(event) = receiver.next().await {
                match event {
                    WorkerEvent::Finished(envelope) => {
                        drained.update(&mut cx, |received, cx| {
                            received.finished.push(envelope.code);
                            cx.notify();
                        });
                    }
                }
            }
        });
        // The task ends when the worker drops its sender. Waiting parks the
        // test thread; the scheduler's 15 s hard limit bounds it.
        drain.await;
        worker.join().unwrap();

        let finished = model.read_with(cx, |received, _| received.finished.clone());
        assert_eq!(finished, [ResultCode::Ok]);
    }
}
