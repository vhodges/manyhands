// A read closure returns the library's `ReadError`, which carries its whole
// scope by value, as the library's own read module allows.
#![allow(clippy::result_large_err)]

use manyhands::{
    repository::{
        CommentListDto, DependencyTreeDto, HostObservationDto, ItemDto, ItemListDto, Mutation,
        MutationCall, MutationDataDto, OperationListDto, PrepareDataDto, ProgressEvent, ReadError,
        RepositoryListDto, RepositoryService,
    },
    results::Envelope,
};

/// Compiles only for a type that may cross to a background thread.
fn send<T: Send + 'static>() {}

/// Every value a read, a prepare or a job carries between the UI thread
/// and the threads that call the library. A failure here stops the work:
/// it is a library change.
#[test]
fn what_crosses_threads_is_send() {
    send::<RepositoryService>();
    send::<Mutation>();
    send::<MutationCall>();
    send::<Envelope<MutationDataDto>>();
    send::<Envelope<PrepareDataDto>>();
    send::<Envelope<HostObservationDto>>();
    send::<RepositoryListDto>();
    send::<ItemListDto>();
    send::<ItemDto>();
    send::<CommentListDto>();
    send::<DependencyTreeDto>();
    send::<OperationListDto>();
    send::<ReadError>();
    send::<ProgressEvent>();
}

/// The backend is shared by reference between threads.
#[test]
fn the_service_is_shared_between_threads() {
    fn sync<T: Sync>() {}
    sync::<RepositoryService>();
}

#[cfg(feature = "desktop-test-support")]
mod view {
    use std::{
        path::{Path, PathBuf},
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };

    use gpui_kit::{AppContext, Context, Entity, Task, TestAppContext};
    use manyhands::{
        repository::{
            LeaseHolderForTesting, LeaseKind, Mutation, PrepareDataDto, RepoCreateInput,
            RepositoryService,
        },
        results::ResultCode,
    };

    use crate::desktop::{
        model::stamp::{Generations, SettingsPage, Stamp, ViewScope},
        services::{ReadFailure, ReadKind, ReadResult, Services},
        test_support::{
            delaying::DelayingBackend,
            fixtures::{Fixture, identity},
        },
    };

    /// A view that asks for a preview and records the envelope's code.
    #[derive(Default)]
    struct PrepareProbe {
        generations: Generations,
        prepared: Vec<(Stamp, ResultCode, bool)>,
        task: Option<Task<()>>,
    }

    #[gpui_kit::test]
    fn a_prepare_delivers_its_envelope_after_its_gate(cx: &mut TestAppContext) {
        let (_fixture, backend, services, _root) = setup(cx);
        let parent = tempfile::tempdir().unwrap();
        let root = parent.path().canonicalize().unwrap().join("new-repository");
        let probe = cx.new(|_| PrepareProbe::default());

        let release = backend.hold_read(ReadKind::Prepare);
        probe.update(cx, |probe, cx| {
            let stamp = probe
                .generations
                .next(ViewScope::Settings(SettingsPage::Repositories));
            let mutation = Mutation::RepoCreate(RepoCreateInput {
                root,
                primary_branch: "main".to_owned(),
                identity: Some(identity()),
                slug_prefix: None,
            });
            probe.task =
                Some(
                    services.prepare(stamp, mutation, cx, |probe, stamp, envelope, _| {
                        let preview = matches!(envelope.data, Some(PrepareDataDto::Preview(_)));
                        probe.prepared.push((stamp, envelope.code, preview));
                    }),
                );
        });
        cx.run_until_parked();
        assert!(probe.read_with(cx, |probe, _| probe.prepared.is_empty()));

        release.send(()).unwrap();
        cx.run_until_parked();
        probe.read_with(cx, |probe, _| {
            assert_eq!(probe.prepared.len(), 1);
            let (stamp, code, preview) = &probe.prepared[0];
            assert_eq!(stamp.generation, 1);
            assert_eq!(*code, ResultCode::Ok);
            assert!(preview);
        });
    }

    /// A view that asks for the repository list and records what it is
    /// given, and what it drops as stale.
    #[derive(Default)]
    struct Probe {
        generations: Generations,
        applied: Vec<(Stamp, ReadResult<Vec<String>>)>,
        dropped: Vec<Stamp>,
        tasks: Vec<Task<()>>,
    }

    impl Probe {
        fn list(&mut self, services: &Services, calls: Arc<AtomicUsize>, cx: &mut Context<Self>) {
            let stamp = self.generations.next(ViewScope::Repositories);
            let task = services.read(
                ReadKind::Repositories,
                stamp,
                move |service| {
                    calls.fetch_add(1, Ordering::SeqCst);
                    service
                        .list_repositories()
                        .map(|list| list.items.into_iter().map(|item| item.root).collect())
                },
                cx,
                |probe, stamp, result, cx| {
                    if !probe.generations.is_current(&stamp) {
                        probe.dropped.push(stamp);
                        return;
                    }
                    probe.applied.push((stamp, result));
                    cx.notify();
                },
            );
            self.tasks.push(task);
        }
    }

    /// A fixture with one repository, and services over its delaying
    /// backend.
    fn setup(cx: &mut TestAppContext) -> (Fixture, Arc<DelayingBackend>, Services, PathBuf) {
        let fixture = Fixture::new();
        let root = fixture.create_repository("a");
        let backend = fixture.delaying_backend();
        let services = cx.update(|cx| Services::start(backend.clone(), cx));
        (fixture, backend, services, root)
    }

    fn roots(probe: &Entity<Probe>, cx: &mut TestAppContext) -> Vec<String> {
        probe.read_with(cx, |probe, _| match &probe.applied[..] {
            [(_, ReadResult::Ok(roots))] => roots.clone(),
            other => panic!("one Ok expected, got {other:?}"),
        })
    }

    #[gpui_kit::test]
    fn a_read_delivers_the_repository_list(cx: &mut TestAppContext) {
        let (_fixture, _backend, services, root) = setup(cx);
        let probe = cx.new(|_| Probe::default());
        let calls = Arc::new(AtomicUsize::new(0));

        probe.update(cx, |probe, cx| probe.list(&services, calls.clone(), cx));
        cx.run_until_parked();

        assert_eq!(roots(&probe, cx), [root.to_string_lossy().into_owned()]);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    /// A view that resolves a path and records the result's shape.
    #[derive(Default)]
    struct ResolveProbe {
        failed: Vec<ReadFailure>,
        task: Option<Task<()>>,
    }

    #[gpui_kit::test]
    fn a_read_that_fails_delivers_its_code_and_nothing_else(cx: &mut TestAppContext) {
        let (_fixture, _backend, services, _root) = setup(cx);
        let parent = tempfile::tempdir().unwrap();
        // A Git repository that was never registered.
        git2::Repository::init(parent.path()).unwrap();
        let path = parent.path().to_owned();
        let calls = Arc::new(AtomicUsize::new(0));
        let counted = calls.clone();
        let probe = cx.new(|_| ResolveProbe::default());

        probe.update(cx, |probe, cx| {
            let stamp = Generations::default().next(ViewScope::Repositories);
            probe.task = Some(services.read(
                ReadKind::Resolve,
                stamp,
                move |service| {
                    counted.fetch_add(1, Ordering::SeqCst);
                    service.resolve_repository(&path).map(|_| ())
                },
                cx,
                |probe, _, result, _| match result {
                    ReadResult::Failed(failure) => probe.failed.push(failure),
                    other => panic!("a failure expected, got {other:?}"),
                },
            ));
        });
        cx.run_until_parked();

        probe.read_with(cx, |probe, _| {
            assert_eq!(probe.failed.len(), 1);
            assert_eq!(probe.failed[0].code, ResultCode::RepositoryNotRegistered);
        });
        // A failure that is not busy is not retried.
        cx.executor().advance_clock(Duration::from_secs(1));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    fn hold_cache_write(root: &Path, data: &Path) -> LeaseHolderForTesting {
        RepositoryService::hold_lease_for_testing(root, data, LeaseKind::CacheWrite).unwrap()
    }

    #[gpui_kit::test]
    fn a_read_that_stays_busy_delivers_busy_after_three_retries(cx: &mut TestAppContext) {
        let (fixture, _backend, services, root) = setup(cx);
        let _held = hold_cache_write(&root, fixture.data_dir());
        let probe = cx.new(|_| Probe::default());
        let calls = Arc::new(AtomicUsize::new(0));

        probe.update(cx, |probe, cx| probe.list(&services, calls.clone(), cx));
        cx.run_until_parked();
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        cx.executor().advance_clock(Duration::from_millis(100));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        cx.executor().advance_clock(Duration::from_millis(200));
        assert_eq!(calls.load(Ordering::SeqCst), 3);
        // The last retry waits its full 400 ms.
        cx.executor().advance_clock(Duration::from_millis(399));
        assert_eq!(calls.load(Ordering::SeqCst), 3);
        assert!(probe.read_with(cx, |probe, _| probe.applied.is_empty()));

        cx.executor().advance_clock(Duration::from_millis(1));
        cx.run_until_parked();
        assert_eq!(calls.load(Ordering::SeqCst), 4);
        probe.read_with(cx, |probe, _| {
            assert!(
                matches!(probe.applied[..], [(_, ReadResult::Busy)]),
                "{:?}",
                probe.applied
            );
        });

        // Nothing is retried after busy is delivered.
        cx.executor().advance_clock(Duration::from_secs(5));
        assert_eq!(calls.load(Ordering::SeqCst), 4);
    }

    #[gpui_kit::test]
    fn a_read_released_before_its_second_retry_delivers_ok(cx: &mut TestAppContext) {
        let (fixture, _backend, services, root) = setup(cx);
        let held = hold_cache_write(&root, fixture.data_dir());
        let probe = cx.new(|_| Probe::default());
        let calls = Arc::new(AtomicUsize::new(0));

        probe.update(cx, |probe, cx| probe.list(&services, calls.clone(), cx));
        cx.run_until_parked();
        cx.executor().advance_clock(Duration::from_millis(100));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert!(probe.read_with(cx, |probe, _| probe.applied.is_empty()));

        drop(held);
        cx.executor().advance_clock(Duration::from_millis(200));
        cx.run_until_parked();

        assert_eq!(calls.load(Ordering::SeqCst), 3);
        assert_eq!(roots(&probe, cx), [root.to_string_lossy().into_owned()]);
    }

    #[gpui_kit::test]
    fn a_read_that_completes_after_a_newer_one_is_dropped(cx: &mut TestAppContext) {
        let (_fixture, backend, services, root) = setup(cx);
        let probe = cx.new(|_| Probe::default());
        let calls = Arc::new(AtomicUsize::new(0));

        let release_first = backend.hold_read(ReadKind::Repositories);
        probe.update(cx, |probe, cx| probe.list(&services, calls.clone(), cx));
        cx.run_until_parked();
        // Held at its gate, before the library call.
        assert_eq!(calls.load(Ordering::SeqCst), 0);

        probe.update(cx, |probe, cx| probe.list(&services, calls.clone(), cx));
        cx.run_until_parked();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        let second = probe.read_with(cx, |probe, _| probe.applied[0].0.clone());
        assert_eq!(second.generation, 2);

        release_first.send(()).unwrap();
        cx.run_until_parked();

        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(roots(&probe, cx), [root.to_string_lossy().into_owned()]);
        probe.read_with(cx, |probe, _| {
            assert_eq!(probe.applied[0].0, second);
            assert_eq!(probe.dropped.len(), 1);
            assert_eq!(probe.dropped[0].generation, 1);
            assert_eq!(probe.dropped[0].scope, ViewScope::Repositories);
        });
    }
}
