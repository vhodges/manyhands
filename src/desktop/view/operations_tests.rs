//! The operation area and index maintenance over real repositories: the
//! launch refreshes and their order, failed refreshes and rebuilds that
//! stay until dismissed, a refresh held on the worker thread while the
//! user goes on, a lease another process holds, and pending operations
//! with Resume and Abandon.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

use gpui_kit::{TestAppContext, test::TestWindowExt};
use manyhands::{
    repository::{ItemDtoKind, LeaseKind, OperationNextAction, RepositoryService},
    results::{Outcome, RecoveryActionKind, ResultCode},
};

use super::{DISMISS, NOTHING_PENDING, outcome_label, recovery_id, request_id};
use crate::desktop::{
    model::{navigation::OpenTarget, operations::Operations},
    services::{
        JobKind, ReadKind,
        requests::{DesktopRequestId, Intent, Lane, RequestState},
    },
    test_support::{
        delaying::JobGate,
        fixtures::{Fixture, abandon_through, item_id, ticket_path, ticket_source},
        git_home,
    },
    view::{
        harness::{Shell, WAIT, key},
        workspace::Region,
    },
};

const FIRST: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG0";
const SECOND: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG1";
const DOC_A: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG2";
const DOC_B: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG3";
const THIRD: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG4";

fn finished(operations: &Operations, id: DesktopRequestId) -> Option<(Outcome, ResultCode)> {
    match &operations.request(id)?.state {
        RequestState::Finished(outcome, code, ..) => Some((*outcome, *code)),
        _ => None,
    }
}

fn outcome(shell: &Shell, id: DesktopRequestId, cx: &mut TestAppContext) -> (Outcome, ResultCode) {
    let operations = shell.operations(cx);
    operations
        .read_with(cx, |operations, _| finished(operations, id))
        .expect("the request finished")
}

fn state_of(shell: &Shell, id: DesktopRequestId, cx: &mut TestAppContext) -> &'static str {
    let operations = shell.operations(cx);
    operations.read_with(cx, |operations, _| {
        match operations.request(id).map(|request| &request.state) {
            Some(RequestState::Queued) => "queued",
            Some(RequestState::Running { .. }) => "running",
            Some(RequestState::Finished(..)) => "finished",
            Some(_) => "other",
            None => "absent",
        }
    })
}

/// The roots of the session's requests on `lane`, oldest first.
fn roots_on(
    shell: &Shell,
    lane: Lane,
    cx: &mut TestAppContext,
) -> Vec<(DesktopRequestId, PathBuf)> {
    let operations = shell.operations(cx);
    operations.read_with(cx, |operations, _| {
        let mut roots: Vec<_> = operations
            .requests()
            .iter()
            .filter(|request| request.lane == lane)
            .filter_map(|request| match &request.intent {
                Intent::IndexRefresh { root } => Some((request.id, root.clone())),
                _ => None,
            })
            .collect();
        roots.reverse();
        roots
    })
}

/// The ticket rows' labels of the selected repository.
fn tickets(shell: &Shell, cx: &mut TestAppContext) -> Vec<String> {
    shell.rows("ticket-row", cx)
}

/// At launch every listed repository is refreshed in the background, in
/// list order. A Refresh the user asks for meanwhile waits only for the
/// job already running: it runs before the second background refresh.
#[gpui_kit::test]
async fn launch_refreshes_each_repository_in_the_background_and_a_manual_refresh_runs_first(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    let b = fixture.create_repository("b");
    // Each job is held before its library call, so the reads a finished
    // job starts are done before the next job runs.
    let mut gates: Option<[JobGate; 3]> = None;
    let shell = Shell::launch(
        fixture,
        |backend| {
            gates = Some(std::array::from_fn(|_| {
                backend.hold_job(JobKind::IndexRefresh)
            }))
        },
        cx,
    );
    let [first, second, third] = gates.unwrap();
    first.wait_reached();
    shell.run(cx);

    let background = roots_on(&shell, Lane::Background, cx);
    assert_eq!(
        background
            .iter()
            .map(|(_, root)| root.clone())
            .collect::<Vec<_>>(),
        [a.clone(), b.clone()]
    );
    assert_eq!(state_of(&shell, background[0].0, cx), "running");
    assert_eq!(state_of(&shell, background[1].0, cx), "queued");

    shell.select(&b, cx);
    shell.keys("secondary-r", cx);
    let user = roots_on(&shell, Lane::User, cx);
    assert_eq!(
        user.iter()
            .map(|(_, root)| root.clone())
            .collect::<Vec<_>>(),
        std::slice::from_ref(&b)
    );

    // The user's refresh runs next, before the second background one.
    first.release();
    second.wait_reached();
    shell.run(cx);
    assert_eq!(state_of(&shell, background[0].0, cx), "finished");
    assert_eq!(state_of(&shell, user[0].0, cx), "running");
    assert_eq!(state_of(&shell, background[1].0, cx), "queued");

    second.release();
    third.wait_reached();
    shell.run(cx);
    assert_eq!(state_of(&shell, user[0].0, cx), "finished");
    assert_eq!(state_of(&shell, background[1].0, cx), "running");

    third.release();
    let ids = [background[0].0, user[0].0, background[1].0];
    shell
        .until(cx, move |operations| {
            ids.iter().all(|id| finished(operations, *id).is_some())
        })
        .await;
    for id in ids {
        assert_eq!(
            outcome(&shell, id, cx),
            (Outcome::Success, ResultCode::Ok),
            "{id:?}"
        );
        // A success is one line: no message or recovery is kept open.
        assert_eq!(
            shell.label(request_id(id, "state"), cx).as_deref(),
            Some(outcome_label(Outcome::Success))
        );
        assert!(!shell.present(request_id(id, "message"), cx));
    }
    assert_eq!(shell.backend.jobs_seen().len(), 3);
}

/// Reads stay bounded as the launch refreshes finish. Each repository's
/// pending operations are read once at the first load and at most once
/// more for each finished request that changed it, never again for every
/// listed repository whenever the list is read; the list is read once at
/// launch and at most once per batch of worker events.
#[gpui_kit::test]
async fn launch_refreshes_read_pending_operations_a_bounded_number_of_times(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    for name in ["a", "b", "c"] {
        fixture.create_repository(name);
    }
    let shell = Shell::launch(fixture, |_| {}, cx);
    let background = shell.requests(cx, |request| request.lane == Lane::Background);
    assert_eq!(background.len(), 3);
    shell
        .until(cx, move |operations| {
            background
                .iter()
                .all(|id| finished(operations, *id).is_some())
        })
        .await;

    let pending = shell.backend.reads_seen(ReadKind::Operations);
    let lists = shell.backend.reads_seen(ReadKind::Repositories);
    // Three at the first load, and one per refresh that changed its root.
    assert!((3..=6).contains(&pending), "{pending} pending reads");
    assert!((1..=4).contains(&lists), "{lists} list reads");
}

/// The single repository `a`, refreshed, then a ticket committed the
/// index does not hold yet; the window open, `a` selected and its tickets
/// shown.
fn with_an_unindexed_ticket(cx: &mut TestAppContext) -> (Shell, PathBuf) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.refresh(&root);
    fixture.commit_file(
        &root,
        &ticket_path(FIRST),
        ticket_source(FIRST, "Unindexed", "").as_bytes(),
    );
    let shell = Shell::open(fixture, cx);
    shell.select(&root, cx);
    shell.click("pane-tickets", cx);
    shell.click("filter-all", cx);
    (shell, root)
}

/// Puts a service that fails before its index transaction commits in
/// place of the desktop's: what the library tests open for a failed
/// refresh or rebuild.
fn failing(shell: &Shell) {
    shell.backend.replace_service(
        RepositoryService::open_at_with_failure_point_for_testing(
            shell.fixture.data_dir(),
            manyhands::repository::FailurePoint::BeforeIndexTransactionCommit,
        )
        .unwrap(),
    );
}

/// Opens the service again, as the library tests do before a retry.
fn reopened(shell: &Shell) {
    shell
        .backend
        .replace_service(RepositoryService::open_at(shell.fixture.data_dir()).unwrap());
}

/// The recovery control of `id` for `kind`.
fn recovery_of(
    shell: &Shell,
    id: DesktopRequestId,
    kind: RecoveryActionKind,
    cx: &mut TestAppContext,
) -> String {
    let operations = shell.operations(cx);
    let n = operations.read_with(cx, |operations, _| {
        match &operations.request(id).unwrap().state {
            RequestState::Finished(_, _, _, recovery, _) => recovery
                .iter()
                .position(|action| action.action == kind)
                .expect("the recovery action is offered"),
            _ => panic!("not finished"),
        }
    });
    recovery_id(id, n)
}

/// A refresh and a rebuild that fail stay in the operation area with
/// their code, message and recovery, through later requests, until they
/// are dismissed. The list shows the stale state it then reads. Retry
/// submits the same request again, which succeeds.
#[gpui_kit::test]
async fn a_failed_refresh_and_rebuild_stay_until_dismissed_and_retry_succeeds(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let (shell, root) = with_an_unindexed_ticket(cx);
    assert!(tickets(&shell, cx).is_empty());

    failing(&shell);
    shell.keys("secondary-r", cx);
    let refresh = shell.requests(cx, |request| {
        matches!(request.intent, Intent::IndexRefresh { .. })
    });
    let refresh = refresh[0];
    shell.until_ended(refresh, cx).await;
    let (failed, code) = outcome(&shell, refresh, cx);
    assert_eq!(failed, Outcome::Error);
    assert_eq!(
        shell.label(request_id(refresh, "state"), cx).as_deref(),
        Some(outcome_label(Outcome::Error))
    );
    assert_eq!(
        shell.label(request_id(refresh, "code"), cx).as_deref(),
        Some(code.as_str())
    );
    assert_eq!(
        shell.label(request_id(refresh, "message"), cx).as_deref(),
        Some(code.message())
    );
    // The list is read again and is still stale: the ticket is not there.
    assert!(tickets(&shell, cx).is_empty());
    // The failed refresh left its operation pending in the repository.
    assert_eq!(
        shell.label(("pending-command", 0usize), cx).as_deref(),
        Some("refresh")
    );

    reopened(&shell);
    let retry = recovery_of(&shell, refresh, RecoveryActionKind::RequestRetry, cx);
    shell.click(retry, cx);
    shell.until_ended(refresh, cx).await;
    assert_eq!(
        outcome(&shell, refresh, cx),
        (Outcome::Success, ResultCode::Ok)
    );
    // The same request, retried: one refresh in the session.
    assert_eq!(
        shell
            .requests(cx, |request| matches!(
                request.intent,
                Intent::IndexRefresh { .. }
            ))
            .len(),
        1
    );
    assert_eq!(tickets(&shell, cx), ["Unindexed"]);
    assert_eq!(
        shell.label("pending-empty", cx).as_deref(),
        Some(NOTHING_PENDING)
    );

    failing(&shell);
    shell.keys("secondary-shift-r", cx);
    let rebuild = shell.requests(cx, |request| {
        matches!(request.intent, Intent::IndexRebuild { .. })
    })[0];
    shell.until_ended(rebuild, cx).await;
    let (failed, code) = outcome(&shell, rebuild, cx);
    assert_eq!(failed, Outcome::Error);
    assert_eq!(
        shell.label(request_id(rebuild, "code"), cx).as_deref(),
        Some(code.as_str())
    );
    assert_eq!(
        shell.label(("pending-command", 0usize), cx).as_deref(),
        Some("rebuild")
    );

    // Another request finishes; the failure stays shown.
    reopened(&shell);
    let other = shell.workspace.update(cx, |workspace, _| {
        workspace.services().worker().submit(
            Intent::IndexRefresh { root: root.clone() },
            Lane::User,
            None,
            None,
        )
    });
    shell.until_ended(other, cx).await;
    assert!(shell.present(request_id(rebuild, "message"), cx));

    let retry = recovery_of(&shell, rebuild, RecoveryActionKind::RequestRetry, cx);
    shell.click(retry, cx);
    shell.until_ended(rebuild, cx).await;
    assert_eq!(
        outcome(&shell, rebuild, cx),
        (Outcome::Success, ResultCode::Ok)
    );
    assert_eq!(tickets(&shell, cx), ["Unindexed"]);

    // The other refresh failed meanwhile, refused while the rebuild was
    // pending: it stays until dismissed.
    let (outcome_of_other, _) = outcome(&shell, other, cx);
    assert_ne!(outcome_of_other, Outcome::Success);
    assert!(shell.present(request_id(other, "message"), cx));
    assert_eq!(
        shell.label(request_id(other, "dismiss"), cx).as_deref(),
        Some(DISMISS)
    );
    shell.click(request_id(other, "dismiss"), cx);
    assert!(!shell.present(request_id(other, "entry"), cx));
    assert!(shell.present(request_id(rebuild, "entry"), cx));
}

/// A refresh held on the worker thread, as it persists, holds nothing
/// else: the user switches repository, opens two items and switches
/// between their tabs meanwhile. Once
/// it is released, the list shows what it indexed.
#[gpui_kit::test]
async fn a_delayed_refresh_leaves_the_user_going_on_and_the_list_updates_on_release(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    let b = fixture.create_repository("b");
    fixture.commit_ticket(&a, FIRST, "In a", "");
    fixture.commit_ticket(&a, THIRD, "Also in a", "");
    fixture.refresh(&a);
    fixture.refresh(&b);
    fixture.commit_ticket(&b, SECOND, "New in b", "");
    let shell = Shell::open(fixture, cx);
    shell.click("pane-tickets", cx);
    shell.select(&b, cx);
    shell.click("filter-all", cx);
    assert!(tickets(&shell, cx).is_empty());

    let (reached, at_hook) = mpsc::channel();
    let (release, released) = mpsc::channel::<()>();
    shell
        .fixture
        .service()
        .set_refresh_persist_hook_for_testing(move || {
            reached.send(()).unwrap();
            let _ = released.recv();
        });
    shell.keys("secondary-r", cx);
    let refresh = shell.requests(cx, |request| request.lane == Lane::User)[0];
    at_hook
        .recv_timeout(WAIT)
        .expect("the refresh reached its hook");

    shell.select(&a, cx);
    let mut listed = tickets(&shell, cx);
    listed.sort();
    assert_eq!(listed, ["Also in a", "In a"]);
    for id in [FIRST, THIRD] {
        shell.workspace.update(cx, |workspace, cx| {
            workspace.open_item(
                OpenTarget::Item {
                    repo: key(&a),
                    kind: ItemDtoKind::Ticket,
                    id: item_id(id),
                },
                cx,
            )
        });
        shell.run(cx);
    }
    assert_eq!(shell.label("item-title", cx).as_deref(), Some("Also in a"));
    // Tabs switch while the refresh is held.
    shell.keys("ctrl-tab", cx);
    assert_eq!(shell.label("item-title", cx).as_deref(), Some("In a"));
    shell.keys("ctrl-shift-tab", cx);
    assert_eq!(shell.label("item-title", cx).as_deref(), Some("Also in a"));
    shell.select(&b, cx);
    assert!(tickets(&shell, cx).is_empty());
    assert_eq!(state_of(&shell, refresh, cx), "running");

    release.send(()).unwrap();
    shell.until_ended(refresh, cx).await;
    assert_eq!(
        outcome(&shell, refresh, cx),
        (Outcome::Success, ResultCode::Ok)
    );
    assert_eq!(tickets(&shell, cx), ["New in b"]);
}

const LEASE_ROOT: &str = "MANYHANDS_DESKTOP_LEASE_ROOT";
const LEASE_DATA: &str = "MANYHANDS_DESKTOP_LEASE_DATA";
const LEASE_READY: &str = "MANYHANDS_DESKTOP_LEASE_READY";
const LEASE_RELEASE: &str = "MANYHANDS_DESKTOP_LEASE_RELEASE";

/// The second actor of `a_lease_held_by_another_process_is_the_refreshs_typed_answer`,
/// run as a process of its own: it holds the repository's lease until
/// its parent says to let go. Outside that, it does nothing.
#[test]
fn lease_holder_child() {
    let Some(root) = std::env::var_os(LEASE_ROOT) else {
        return;
    };
    git_home::isolated();
    let data = std::env::var_os(LEASE_DATA).unwrap();
    let ready = std::env::var_os(LEASE_READY).unwrap();
    let release = PathBuf::from(std::env::var_os(LEASE_RELEASE).unwrap());
    let held = RepositoryService::hold_lease_for_testing(
        Path::new(&root),
        Path::new(&data),
        LeaseKind::Repository,
    )
    .unwrap();
    fs::write(ready, b"held").unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    while !release.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    drop(held);
}

/// The child that holds a repository's lease; it lets go when dropped.
struct LeaseHolder {
    child: Child,
    release: PathBuf,
    _signals: tempfile::TempDir,
}

impl LeaseHolder {
    fn start(root: &Path, data: &Path) -> Self {
        let signals = tempfile::tempdir().unwrap();
        let ready = signals.path().join("ready");
        let release = signals.path().join("release");
        let child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "desktop::view::operations::tests::lease_holder_child",
                "--nocapture",
            ])
            .env(LEASE_ROOT, root)
            .env(LEASE_DATA, data)
            .env(LEASE_READY, &ready)
            .env(LEASE_RELEASE, &release)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let holder = Self {
            child,
            release,
            _signals: signals,
        };
        let deadline = Instant::now() + WAIT;
        while !ready.exists() {
            assert!(
                Instant::now() < deadline,
                "the child did not take the lease"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        holder
    }
}

impl Drop for LeaseHolder {
    fn drop(&mut self) {
        let _ = fs::write(&self.release, b"go");
        let _ = self.child.wait();
    }
}

/// Track rule 4: another process holds the repository's lease while the
/// desktop's refresh waits to run. The refresh's typed answer is shown in
/// the operation area, and nothing says it succeeded.
#[gpui_kit::test]
async fn a_lease_held_by_another_process_is_the_refreshs_typed_answer(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let shell = Shell::open(fixture, cx);
    shell.select(&root, cx);

    let held = shell.backend.hold_job(JobKind::IndexRefresh);
    shell.keys("secondary-r", cx);
    let refresh = shell.requests(cx, |request| request.lane == Lane::User)[0];
    held.wait_reached();
    let holder = LeaseHolder::start(&root, shell.fixture.data_dir());
    held.release();
    shell.until_ended(refresh, cx).await;
    drop(holder);

    let (answered, code) = outcome(&shell, refresh, cx);
    assert!(
        !matches!(answered, Outcome::Success | Outcome::Noop),
        "{}",
        code.as_str()
    );
    assert_eq!(code, ResultCode::Busy, "{}", code.as_str());
    assert_eq!(
        shell.label(request_id(refresh, "state"), cx).as_deref(),
        Some(outcome_label(answered))
    );
    assert_eq!(
        shell.label(request_id(refresh, "code"), cx).as_deref(),
        Some(code.as_str())
    );
    assert_eq!(
        shell.label(request_id(refresh, "message"), cx).as_deref(),
        Some(code.message())
    );
    // Nothing in the area says the refresh succeeded: it stays
    // expanded, with its message.
    assert!(shell.present(request_id(refresh, "message"), cx));
    assert_ne!(
        shell.label(request_id(refresh, "state"), cx).as_deref(),
        Some(outcome_label(Outcome::Success))
    );
}

/// The pending local operation of a ticket save whose index hand-off
/// failed, in `a`, with the window open: its operation ID.
fn stranded(cx: &mut TestAppContext) -> (Shell, PathBuf, String) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.create_ticket(&root, FIRST, "Stranded", "open");
    let operation = fixture.strand_ticket_save(&root, FIRST, "Edited");
    let shell = Shell::open(fixture, cx);
    (shell, root, operation)
}

/// What the library lists as pending in `root`.
fn library_pending(shell: &Shell, root: &Path) -> Vec<String> {
    let service = shell.fixture.service();
    let repo = service.resolve_repository(root).unwrap();
    service
        .list_operations(&repo)
        .unwrap()
        .items
        .into_iter()
        .filter(|operation| operation.next_action.is_some())
        .filter_map(|operation| operation.operation_id)
        .collect()
}

/// A pending local operation is listed with its command, operation ID
/// and what it waits for. It offers Resume, since it is local and waits
/// to be resumed, and Abandon, which opens the confirmation dialog;
/// Confirm abandons it and the row is gone once read again.
#[gpui_kit::test]
async fn a_pending_local_operation_offers_resume_and_abandon_confirms_it(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root, operation) = stranded(cx);
    assert_eq!(
        library_pending(&shell, &root),
        std::slice::from_ref(&operation)
    );
    assert_eq!(
        shell.label(("pending-command", 0usize), cx).as_deref(),
        Some("save_ticket")
    );
    assert_eq!(
        shell.label(("pending-operation", 0usize), cx),
        Some(operation.clone())
    );
    assert_eq!(
        shell.label(("pending-next", 0usize), cx).as_deref(),
        Some(OperationNextAction::Resume.as_str())
    );
    assert_eq!(
        shell.label(("pending-scope", 0usize), cx).as_deref(),
        Some("a")
    );
    assert!(shell.present(("pending-resume", 0usize), cx));
    assert!(!shell.present(("pending-resume-unavailable", 0usize), cx));

    shell.click(("pending-abandon", 0usize), cx);
    assert_eq!(
        shell.label("confirm-command-name", cx).as_deref(),
        Some("operation abandon")
    );
    assert!(!shell.rows("confirm-effect", cx).is_empty());
    assert!(
        shell
            .rows("confirm-detail", cx)
            .contains(&format!("Operation: {operation}"))
    );
    assert!(
        shell
            .label("confirm-expires", cx)
            .is_some_and(|text| text.starts_with("Expires "))
    );
    // Nothing is submitted by the preview.
    assert!(shell.requests(cx, |_| true).is_empty());

    shell.click("confirm-ok", cx);
    let abandon = shell.requests(cx, |request| {
        matches!(request.intent, Intent::OperationAbandon { .. })
    });
    assert_eq!(abandon.len(), 1);
    shell.until_ended(abandon[0], cx).await;
    assert_eq!(
        outcome(&shell, abandon[0], cx),
        (Outcome::Success, ResultCode::Ok)
    );
    assert!(!shell.present("confirm-dialog", cx));
    assert!(library_pending(&shell, &root).is_empty());
    assert!(!shell.present(("pending-row", 0usize), cx));
    assert_eq!(
        shell.label("pending-empty", cx).as_deref(),
        Some(NOTHING_PENDING)
    );
    // The dialog gave focus back to Abandon, whose row is now gone: focus
    // is on the operation area itself, not nowhere.
    shell.run(cx);
    assert!(shell.present(Region::Operations.ring_id(), cx));
}

/// Resume runs `operation resume` as a user job; the operation finishes
/// and its row is gone once read again.
#[gpui_kit::test]
async fn resume_finishes_a_pending_local_operation(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root, _) = stranded(cx);
    shell.click(("pending-resume", 0usize), cx);
    let resume = shell.requests(cx, |request| {
        matches!(request.intent, Intent::OperationResume { .. }) && request.lane == Lane::User
    });
    assert_eq!(resume.len(), 1);
    shell.until_ended(resume[0], cx).await;
    assert_eq!(
        outcome(&shell, resume[0], cx),
        (Outcome::Success, ResultCode::Ok)
    );
    assert!(library_pending(&shell, &root).is_empty());
    assert!(!shell.present(("pending-row", 0usize), cx));
}

/// Track rule 4: another service abandons the row the desktop shows.
/// The desktop's own Abandon is then answered by the library, typed, with
/// nothing to confirm, and the row is gone once read again.
#[gpui_kit::test]
fn an_abandon_by_a_second_actor_is_answered_and_the_row_goes(cx: &mut TestAppContext) {
    let (shell, root, operation) = stranded(cx);
    let other = RepositoryService::open_at(shell.fixture.data_dir()).unwrap();
    abandon_through(&other, &root, &operation);
    assert!(shell.present(("pending-row", 0usize), cx));

    shell.click(("pending-abandon", 0usize), cx);
    let code = shell.label("confirm-code", cx).expect("a typed answer");
    assert_ne!(code, ResultCode::Ok.as_str());
    let answered = ResultCode::ALL
        .into_iter()
        .find(|known| known.as_str() == code)
        .unwrap();
    assert_eq!(
        shell.label("confirm-message", cx).as_deref(),
        Some(answered.message())
    );
    assert!(!shell.present("confirm-ok", cx));
    assert!(!shell.present(("pending-row", 0usize), cx));
    assert!(shell.requests(cx, |_| true).is_empty());

    shell.keys("escape", cx);
    assert!(!shell.present("confirm-dialog", cx));
}

/// A refresh's reload keeps the selected row of the document tree.
#[gpui_kit::test]
async fn a_refresh_keeps_the_selected_document(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.create_document(&root, DOC_A, "docs/a.md", "Alpha");
    fixture.create_document(&root, DOC_B, "docs/b.md", "Beta");
    let shell = Shell::open(fixture, cx);
    shell.select(&root, cx);
    shell.keys("secondary-2", cx);
    shell.keys("down down", cx);
    let selected: Vec<bool> = (0..2)
        .map(|ix| shell.selected(("doc-node", ix as usize), cx))
        .collect();
    assert!(selected.contains(&true));

    shell.keys("secondary-r", cx);
    let refresh = shell.requests(cx, |request| request.lane == Lane::User)[0];
    shell.until_ended(refresh, cx).await;
    assert_eq!(
        outcome(&shell, refresh, cx),
        (Outcome::Success, ResultCode::Ok)
    );
    let after: Vec<bool> = (0..2)
        .map(|ix| shell.selected(("doc-node", ix as usize), cx))
        .collect();
    assert_eq!(after, selected);
}

/// Whether the control wrapped as `id` holds focus.
fn control_focused(shell: &Shell, id: &str, cx: &mut TestAppContext) -> bool {
    let id = id.to_owned();
    shell.frame(cx, |window, _| {
        window
            .try_find(gpui_kit::ElementId::Name(id.into()))
            .and_then(|control| control.focused())
            == Some(true)
    })
}

/// From the operation area, Tab until the control `id` holds focus.
fn tab_to(shell: &Shell, id: &str, cx: &mut TestAppContext) {
    shell.keys("secondary-4", cx);
    let mut presses = 0;
    while !control_focused(shell, id, cx) {
        presses += 1;
        assert!(presses < 30, "Tab never reached {id}");
        shell.keys("tab", cx);
    }
}

/// The operation area holds focus on a drawn control, and rings it.
fn operations_keep_focus(shell: &Shell, cx: &mut TestAppContext) {
    assert!(shell.frame(cx, |window, cx| window.focused(cx).is_some()));
    assert!(shell.frame(cx, |window, _| {
        window.find(Region::Operations.id()).focused() == Some(true)
    }));
    assert!(shell.present(Region::Operations.ring_id(), cx));
}

/// Dismiss and Retry, pressed from the keyboard, take their control away
/// with the request's state: focus falls back to the operation area,
/// never nowhere.
#[gpui_kit::test]
async fn dismiss_and_retry_from_the_keyboard_leave_focus_in_the_operation_area(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let (shell, _) = with_an_unindexed_ticket(cx);
    failing(&shell);
    shell.keys("secondary-r", cx);
    let refresh = shell.requests(cx, |request| {
        matches!(request.intent, Intent::IndexRefresh { .. })
    })[0];
    shell.until_ended(refresh, cx).await;

    let retry = recovery_of(&shell, refresh, RecoveryActionKind::RequestRetry, cx);
    tab_to(&shell, &retry, cx);
    let held = shell.backend.hold_job(JobKind::IndexRefresh);
    shell.press("enter", cx);
    held.wait_reached();
    assert!(
        !shell.present(retry.clone(), cx),
        "Retry went with the attempt"
    );
    operations_keep_focus(&shell, cx);
    held.release();
    shell.until_ended(refresh, cx).await;

    let dismiss = request_id(refresh, "dismiss");
    tab_to(&shell, &dismiss, cx);
    shell.press("enter", cx);
    assert!(!shell.present(request_id(refresh, "entry"), cx));
    operations_keep_focus(&shell, cx);
}

/// A recovery action that opens the confirmation dialog, here Abandon of
/// the operation a failed refresh left pending, gives focus back to that
/// action's control when the dialog closes, not to the region.
#[gpui_kit::test]
async fn a_recovery_dialog_gives_focus_back_to_the_action_that_opened_it(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, _) = with_an_unindexed_ticket(cx);
    failing(&shell);
    shell.keys("secondary-r", cx);
    let refresh = shell.requests(cx, |request| {
        matches!(request.intent, Intent::IndexRefresh { .. })
    })[0];
    shell.until_ended(refresh, cx).await;
    reopened(&shell);

    let abandon = recovery_of(&shell, refresh, RecoveryActionKind::OperationAbandon, cx);
    tab_to(&shell, &abandon, cx);
    shell.press("enter", cx);
    assert!(shell.present("confirm-dialog", cx));
    shell.keys("escape", cx);
    assert!(!shell.present("confirm-dialog", cx));
    assert!(
        control_focused(&shell, &abandon, cx),
        "focus is back on the recovery action"
    );
    assert!(shell.present(Region::Operations.ring_id(), cx));
}
