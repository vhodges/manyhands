//! Kept resolution sets (D5 B3; decision 19; R-12; P24), over the
//! fixture's real data directory and real local conflicts.

use std::{
    fmt::Debug,
    fs,
    marker::PhantomData,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use gpui_kit::TestAppContext;
use manyhands::{
    repository::{
        CancellationToken, FailurePoint, FolderCreateInput, Mutation, MutationCall, NoProgress,
        OperationId, RepositoryService, RequestId, RequestState as LibraryState,
        keys::SessionCredentials,
    },
    results::ResultCode,
};

use super::{KeptSet, RESOLUTIONS, ResolutionStore, ResolutionStoreError};
use crate::desktop::{
    services::{
        JobKind, Services,
        conflicts::ConflictSource,
        requests::{DesktopRequestId, Intent, PathResolution, RequestState},
    },
    test_support::{
        delaying::DelayingBackend,
        fixtures::{Fixture, NoPassphrase, item_id},
    },
};

pub(super) const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG0";
const SENTINEL: &str = "SENTINEL-D5-RESOLUTION-3b9d";

pub(super) fn start(
    fixture: &Fixture,
    cx: &mut TestAppContext,
) -> (Arc<DelayingBackend>, Services) {
    cx.executor().allow_parking();
    let backend = fixture.delaying_backend();
    let services = cx.update(|cx| Services::start(backend.clone(), cx).without_launch_refresh());
    cx.run_until_parked();
    (backend, services)
}

/// A set of `source` for `operation` of the ticket `TICKET`, whose one
/// path's result is `text`.
pub(super) fn a_set(
    root: &Path,
    operation: OperationId,
    source: ConflictSource,
    text: &str,
) -> KeptSet {
    KeptSet {
        request_id: RequestId::new(),
        root: root.to_owned(),
        operation_id: operation,
        item: item_id(TICKET),
        source,
        observation: format!("observation of {text}"),
        resolutions: vec![PathResolution {
            token: "token-1".to_owned(),
            source: text.to_owned(),
        }],
    }
}

/// A real local conflict of `TICKET` in `root`, and the set that keeps
/// mine for its one path, with the inspection's observation (R-11).
pub(super) fn a_local_resolution(fixture: &Fixture, root: &Path) -> KeptSet {
    let operation = fixture.conflict_locally(root, TICKET, "Mine", "Theirs");
    let (inspection, contents) = fixture.read_local_conflict(root, operation);
    KeptSet {
        request_id: RequestId::new(),
        root: root.to_owned(),
        operation_id: operation,
        item: item_id(TICKET),
        source: ConflictSource::Local,
        observation: inspection.observation.expect("a conflict is observed"),
        resolutions: contents
            .iter()
            .map(|content| PathResolution {
                token: content.token.clone(),
                source: content.local.clone().expect("mine is given"),
            })
            .collect(),
    }
}

/// Every field of `set`, as text, for comparing two sets.
pub(super) fn fields(set: &KeptSet) -> Vec<String> {
    let mut fields = vec![
        set.request_id.to_string(),
        set.root.display().to_string(),
        set.operation_id.to_string(),
        set.item.to_string(),
        format!("{:?}", set.source),
        set.observation.clone(),
    ];
    for path in &set.resolutions {
        fields.push(path.token.clone());
        fields.push(path.source.clone());
    }
    fields
}

pub(super) fn kept(store: &ResolutionStore, request: RequestId) -> Option<Vec<String>> {
    store.kept(request).unwrap().as_ref().map(fields)
}

/// How long a real resolution may take. Alone it takes half a second;
/// under the whole desktop suite its writing and indexing have taken
/// over twenty, past the harness's `WAIT`.
pub(super) const RESOLVING: Duration = Duration::from_secs(90);

/// Waits until `done` holds, letting the foreground and the background
/// run between looks.
pub(super) async fn until(
    cx: &mut TestAppContext,
    what: &str,
    done: impl Fn(&TestAppContext) -> bool,
) {
    let deadline = Instant::now() + RESOLVING;
    loop {
        cx.run_until_parked();
        if done(cx) {
            return;
        }
        assert!(Instant::now() < deadline, "{what} did not happen");
        cx.executor().timer(Duration::from_millis(10)).await;
    }
}

/// Waits until the request `id` has its answer, then lets what follows
/// it run.
pub(super) async fn finished(services: &Services, id: DesktopRequestId, cx: &mut TestAppContext) {
    until(cx, "the request's answer", |cx| {
        services.operations().read_with(cx, |operations, _| {
            operations
                .request(id)
                .is_some_and(|request| matches!(request.state, RequestState::Finished(..)))
        })
    })
    .await;
}

/// Runs a real `folder create` under `request` to its end on `service`.
fn run_folder_create(service: &RepositoryService, root: &Path, request: RequestId) -> ResultCode {
    service
        .execute(
            MutationCall {
                request_id: request,
                confirmation: None,
                mutation: Mutation::FolderCreate(FolderCreateInput {
                    root: root.to_owned(),
                    path: format!("docs/{request}"),
                }),
            },
            &mut SessionCredentials::new(NoPassphrase),
            &mut NoProgress,
            &CancellationToken::new(),
        )
        .code
}

/// Decision 19: the set is written under its request ID before the
/// request reaches the library, and the request carries that ID. A store
/// that did not open sends nothing.
#[gpui_kit::test]
async fn a_set_is_written_before_submission(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let (backend, services) = start(&fixture, cx);
    let set = a_local_resolution(&fixture, &root);
    let request = set.request_id;
    let expected = fields(&set);

    let held = backend.hold_job(JobKind::LocalContextResolve);
    let submitted = cx.update(|cx| services.submit_resolution(set, cx));
    cx.run_until_parked();
    let id = match submitted.await {
        Ok(id) => id,
        Err(error) => panic!("the set must be kept: {error}"),
    };
    held.wait_reached();
    assert_eq!(
        kept(&fixture.resolution_store(), request),
        Some(expected),
        "kept before the library call"
    );
    let (carried, local) = services.operations().read_with(cx, |operations, _| {
        let sent = operations.request(id).unwrap();
        (
            sent.request_id,
            matches!(sent.intent, Intent::LocalContextResolve { .. }),
        )
    });
    assert_eq!(carried, request, "the request carries the set's ID");
    assert!(local, "a local set is sent as a local resolution");
    held.release();
    finished(&services, id, cx).await;

    // No store: nothing is kept, so nothing is sent.
    let unkept = Arc::new(DelayingBackend::new(
        Arc::new(RepositoryService::open_at(fixture.data_dir()).unwrap()),
        fixture.home().to_owned(),
        Ok(fixture.draft_store()),
    ));
    let without = cx.update(|cx| Services::start(unkept.clone(), cx).without_launch_refresh());
    let set = a_set(&root, OperationId::new(), ConflictSource::Remote, "x");
    let refused = cx.update(|cx| without.submit_resolution(set, cx));
    cx.run_until_parked();
    assert!(matches!(
        refused.await,
        Err(ResolutionStoreError::Unavailable)
    ));
    cx.run_until_parked();
    assert!(unkept.jobs_seen().is_empty(), "nothing was sent");
}

/// P24: a set outlives its store: a store opened again over the same data
/// directory, as after a restart, reads it back whole, from an owner-only
/// file in `<data_dir>/resolutions`.
#[gpui_kit::test]
fn a_set_is_read_back_after_reopening_the_store(_cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = PathBuf::from("/nowhere/a");
    let set = a_set(&root, OperationId::new(), ConflictSource::Local, "Kept");
    let request = set.request_id;
    {
        let store = ResolutionStore::open_in(fixture.data_dir()).unwrap();
        store.keep(&set).unwrap();
    }
    let reopened = ResolutionStore::open_in(fixture.data_dir()).unwrap();
    assert_eq!(kept(&reopened, request), Some(fields(&set)));
    assert_eq!(reopened.requests().unwrap(), vec![request]);

    let file = fixture
        .data_dir()
        .join(RESOLUTIONS)
        .join(format!("{request}.json"));
    assert!(file.is_file(), "beside drafts, by request ID");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let folder = fs::metadata(fixture.data_dir().join(RESOLUTIONS)).unwrap();
        assert_eq!(folder.permissions().mode() & 0o777, 0o700);
        let kept = fs::metadata(&file).unwrap();
        assert_eq!(kept.permissions().mode() & 0o777, 0o600);
    }
}

/// R-12: a set is deleted once its request has finished, whatever its
/// answer: applied, and refused.
#[gpui_kit::test]
async fn a_set_is_deleted_when_its_request_finishes(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let (_backend, services) = start(&fixture, cx);
    let store = fixture.resolution_store();

    let applied = a_local_resolution(&fixture, &root);
    let request = applied.request_id;
    let submitted = cx.update(|cx| services.submit_resolution(applied, cx));
    cx.run_until_parked();
    let id = submitted.await.expect("kept");
    finished(&services, id, cx).await;
    let code = services.operations().read_with(cx, |operations, _| {
        match &operations.request(id).unwrap().state {
            RequestState::Finished(_, code, ..) => *code,
            _ => unreachable!(),
        }
    });
    assert_eq!(code, ResultCode::Ok, "the resolution applied");
    until(cx, "the applied set's deletion", |_| {
        store.kept(request).unwrap().is_none()
    })
    .await;

    // A second conflict's set, refused for a stale observation.
    let operation = fixture.conflict_locally(&root, TICKET, "Mine again", "Theirs again");
    let mut refused = a_set(&root, operation, ConflictSource::Local, "x");
    refused.observation = "stale".to_owned();
    let request = refused.request_id;
    let submitted = cx.update(|cx| services.submit_resolution(refused, cx));
    cx.run_until_parked();
    let id = submitted.await.expect("kept");
    finished(&services, id, cx).await;
    until(cx, "the refused set's deletion", |_| {
        store.kept(request).unwrap().is_none()
    })
    .await;
    assert!(store.requests().unwrap().is_empty());
}

/// Decision 19: two requests for the same operation keep a set each;
/// keeping the same set again changes nothing, a different set is never
/// kept over it (M-2), and forgetting one leaves the other.
#[gpui_kit::test]
fn sets_are_kept_per_request_id(_cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let store = fixture.resolution_store();
    let root = PathBuf::from("/nowhere/a");
    let operation = OperationId::new();
    let first = a_set(&root, operation, ConflictSource::Remote, "First");
    let mut second = a_set(&root, operation, ConflictSource::Remote, "Second");
    store.keep(&first).unwrap();
    store.keep(&second).unwrap();

    let mut both = vec![first.request_id, second.request_id];
    both.sort_by_key(|id| id.to_string());
    assert_eq!(store.requests().unwrap(), both);
    assert_eq!(kept(&store, first.request_id), Some(fields(&first)));
    assert_eq!(kept(&store, second.request_id), Some(fields(&second)));

    // M-2: the same set again is kept as it is; a different one under the
    // same ID is refused, and the kept one stands.
    store.keep(&second).unwrap();
    assert_eq!(kept(&store, second.request_id), Some(fields(&second)));
    let kept_second = fields(&second);
    second.resolutions[0].source = "Second, chosen again".to_owned();
    assert!(matches!(
        store.keep(&second),
        Err(ResolutionStoreError::Differs)
    ));
    assert_eq!(kept(&store, second.request_id), Some(kept_second.clone()));
    assert_eq!(kept(&store, first.request_id), Some(fields(&first)));

    store.forget(first.request_id).unwrap();
    assert_eq!(kept(&store, first.request_id), None);
    assert_eq!(kept(&store, second.request_id), Some(kept_second));
    store.forget(first.request_id).unwrap();
    assert_eq!(store.requests().unwrap(), vec![second.request_id]);
}

/// Privacy: a set derives no `Debug`, the store's errors name no file and
/// no text and use their own words, and a submitted set's text reaches
/// neither the operation area's label nor its message.
#[gpui_kit::test]
async fn a_set_has_no_debug_and_never_reaches_a_message(cx: &mut TestAppContext) {
    // Checked as the test compiles.
    const {
        assert!(!IsDebug::<KeptSet>::IS_DEBUG);
        assert!(!IsDebug::<PathResolution>::IS_DEBUG);
        assert!(IsDebug::<ResolutionStoreError>::IS_DEBUG, "the probe works");
    }

    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let (_backend, services) = start(&fixture, cx);
    let store = fixture.resolution_store();

    // A damaged kept file holding the sentinel reads as `Damaged`.
    let damaged = RequestId::new();
    fs::write(
        fixture
            .data_dir()
            .join(RESOLUTIONS)
            .join(format!("{damaged}.json")),
        SENTINEL,
    )
    .unwrap();
    assert!(matches!(
        store.kept(damaged),
        Err(ResolutionStoreError::Damaged)
    ));
    for error in [
        ResolutionStoreError::Unavailable,
        ResolutionStoreError::ProtectionUnavailable,
        ResolutionStoreError::Access,
        ResolutionStoreError::NoSpace,
        ResolutionStoreError::Damaged,
        ResolutionStoreError::Differs,
        ResolutionStoreError::Newer,
        ResolutionStoreError::Io,
    ] {
        let words = error.to_string();
        assert!(!words.contains(SENTINEL), "{words}");
        assert!(!words.contains("draft"), "{words}");
        assert!(words.contains("resolution"), "{words}");
    }

    // A set whose every text holds the sentinel, sent and refused.
    let operation = fixture.conflict_locally(&root, TICKET, "Mine", "Theirs");
    let mut set = a_set(&root, operation, ConflictSource::Local, SENTINEL);
    set.observation = format!("{SENTINEL} observation");
    let submitted = cx.update(|cx| services.submit_resolution(set, cx));
    cx.run_until_parked();
    let id = submitted.await.expect("kept");
    finished(&services, id, cx).await;
    let (label, message) = services.operations().read_with(cx, |operations, _| {
        let request = operations.request(id).unwrap();
        let message = match &request.state {
            RequestState::Finished(.., message) => message.clone(),
            _ => unreachable!(),
        };
        (request.intent.label().to_owned(), message)
    });
    assert!(!label.contains(SENTINEL), "{label}");
    assert!(!message.contains(SENTINEL), "{message}");
}

/// R-12's launch sweep: `show_request` of every kept set at start; a
/// finished request's set and a local one the library has no record of
/// are deleted, an accepted one's is kept for its Retry.
#[gpui_kit::test]
async fn the_launch_sweep_deletes_finished_and_unknown_sets(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let store = fixture.resolution_store();
    let nowhere = PathBuf::from("/nowhere/a");

    // A request that finished.
    let done = a_set(&nowhere, OperationId::new(), ConflictSource::Remote, "Done");
    assert_eq!(
        run_folder_create(fixture.service(), &root, done.request_id),
        ResultCode::Ok
    );
    // A request accepted and never settled: the library's failure point
    // after the change and before the record's settlement.
    let accepted = a_set(&nowhere, OperationId::new(), ConflictSource::Local, "Open");
    let failing = RepositoryService::open_at_with_failure_point_for_testing(
        fixture.data_dir(),
        FailurePoint::BeforeRequestSettlement,
    )
    .unwrap();
    run_folder_create(&failing, &root, accepted.request_id);
    drop(failing);
    let shown = fixture.service().show_request(accepted.request_id);
    assert_eq!(
        shown.data.map(|request| request.state),
        Some(LibraryState::Accepted)
    );
    // A local request the library never saw. A remote one is kept while
    // its conflict names it (R-25; `resolutions_answers_tests`).
    let unknown = a_set(
        &nowhere,
        OperationId::new(),
        ConflictSource::Local,
        "Unknown",
    );
    for set in [&done, &accepted, &unknown] {
        store.keep(set).unwrap();
    }

    let backend = fixture.delaying_backend();
    let _services = cx.update(|cx| Services::start(backend, cx).without_launch_refresh());
    until(cx, "the sweep", |_| {
        store.requests().unwrap() == vec![accepted.request_id]
    })
    .await;
    assert_eq!(kept(&store, accepted.request_id), Some(fields(&accepted)));
}

/// B3: a set records its request ID, root, operation, item and whether it
/// is remote or local, and is sent as that kind's intent with its
/// observation and every path.
#[gpui_kit::test]
fn a_set_records_root_operation_item_and_kind(_cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let store = fixture.resolution_store();
    let root = fixture.data_dir().join("a repository");
    for source in [ConflictSource::Remote, ConflictSource::Local] {
        let mut set = a_set(&root, OperationId::new(), source, "Text\nwith lines\n");
        set.resolutions.push(PathResolution {
            token: "token-2".to_owned(),
            source: "Second\n".to_owned(),
        });
        store.keep(&set).unwrap();
        let back = store.kept(set.request_id).unwrap().expect("kept");
        assert_eq!(back.request_id, set.request_id);
        assert_eq!(back.root, root);
        assert_eq!(back.operation_id, set.operation_id);
        assert_eq!(back.item, item_id(TICKET));
        assert_eq!(back.source, source);
        assert_eq!(fields(&back), fields(&set));

        let (intent_root, operation, observation, paths) = match (source, back.intent()) {
            (
                ConflictSource::Remote,
                Intent::ConflictResolve {
                    root,
                    operation_id,
                    observation,
                    resolutions,
                },
            )
            | (
                ConflictSource::Local,
                Intent::LocalContextResolve {
                    root,
                    operation_id,
                    observation,
                    resolutions,
                },
            ) => (root, operation_id, observation, resolutions),
            _ => panic!("the set's kind picks its intent"),
        };
        assert_eq!(intent_root, root);
        assert_eq!(operation, set.operation_id);
        assert_eq!(observation, set.observation);
        let paths: Vec<_> = paths
            .iter()
            .map(|path| (path.token.clone(), path.source.clone()))
            .collect();
        assert_eq!(
            paths,
            vec![
                ("token-1".to_owned(), "Text\nwith lines\n".to_owned()),
                ("token-2".to_owned(), "Second\n".to_owned()),
            ]
        );
    }
}

/// Whether `T` implements `Debug`: the inherent constant applies only
/// when it does; otherwise the trait's default is found.
struct IsDebug<T: ?Sized>(PhantomData<T>);

trait NotDebug {
    const IS_DEBUG: bool = false;
}

impl<T: ?Sized> NotDebug for IsDebug<T> {}

impl<T: ?Sized + Debug> IsDebug<T> {
    const IS_DEBUG: bool = true;
}
