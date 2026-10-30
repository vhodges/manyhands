//! `item sync`, `repo sync`, `operation resume` and `operation abandon`
//! through the binary, without a remote. What needs one is Task 12a's.

use std::{fs, path::Path};

use serde_json::Value;

use manyhands::{
    repository::{
        AuthoringKind, FailurePoint, OperationId, RemoteOperationAction, RemoteOperationTarget,
        RemoteRefPlan, RemoteReservationOutcome, RequestState,
    },
    results::ResultCode,
};

use super::{
    c4_support::{environment, golden},
    mutation::{checked, run},
};
use crate::support::{
    self, FailOnce,
    cli::{self, CliEnv, CliRun},
    items,
    mutation::{REQUEST_1, REQUEST_2, REQUEST_3, TITLE, World},
};

const OTHER_REQUEST: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX7";
const EDITED_TITLE: &str = "An uncommitted third-party title";

/// A save of the open ticket under `REQUEST_1` that committed and failed
/// its index hand-off: its operation is pending, owing that hand-off, and
/// its record is accepted. A plain service and a CLI environment are over
/// the same data.
struct Owing {
    world: World,
    env: CliEnv,
    operation: String,
    token: String,
}

impl Owing {
    fn new() -> Self {
        let mut world = World::with_service(|data| {
            FailOnce::at(FailurePoint::BeforeIndexTransactionCommit).open_service(data)
        });
        let token = world.token(items::TICKET_A);
        let owed = world.execute(REQUEST_1, world.save(items::TICKET_A, &token));
        assert_eq!(owed.code, ResultCode::DiscoveryPending);
        assert_eq!(world.pending_journal_rows(), 1);
        let operation = owed.operation_id.clone().unwrap();
        world.reopen();
        let env = environment(world.data.path());
        Self {
            world,
            env,
            operation,
            token,
        }
    }

    fn root(&self) -> &str {
        self.world.root.to_str().unwrap()
    }

    fn file(&self) -> std::path::PathBuf {
        self.world
            .worktree(items::TICKET_A)
            .join(items::ticket_path(items::TICKET_A))
    }

    /// C3's known limit: the save's file is edited by hand, so the save's
    /// own retry is refused and nothing else finishes it (R-30).
    fn stuck(&self) -> Vec<u8> {
        let committed = fs::read_to_string(self.file()).unwrap();
        assert_eq!(committed.matches(TITLE).count(), 1);
        fs::write(self.file(), committed.replace(TITLE, EDITED_TITLE)).unwrap();
        let retry = self
            .world
            .execute(REQUEST_1, self.world.save(items::TICKET_A, &self.token));
        assert_eq!(retry.code, ResultCode::ExternalChange);
        assert_eq!(self.world.pending_journal_rows(), 1);
        fs::read(self.file()).unwrap()
    }

    fn args<'a>(&'a self, verb: &'a str, operation: &'a str, extra: &[&'a str]) -> Vec<&'a str> {
        let mut args = vec![
            "operation",
            verb,
            "--repo",
            self.root(),
            "--operation-id",
            operation,
        ];
        args.extend_from_slice(extra);
        args
    }

    fn call(
        &self,
        verb: &str,
        operation: &str,
        extra: &[&str],
        status: i32,
        code: &str,
        data_schema: Option<&str>,
    ) -> Value {
        let mut args = self.args(verb, operation, extra);
        args.push("--json");
        checked(&args, &self.env, &[], status, code, data_schema)
    }

    fn prepare(&self) -> Value {
        let preview = self.call(
            "abandon",
            &self.operation,
            &["--prepare"],
            0,
            "ok",
            Some("operation_abandon_preview.schema.json"),
        );
        assert!(preview["request_id"].is_null());
        preview
    }

    fn confirm(&self, request: &str, preview: &Value, status: i32, code: &str) -> Value {
        let schema = (code == "ok").then_some("operation_mutation.schema.json");
        self.call(
            "abandon",
            &self.operation,
            &[
                "--request-id",
                request,
                "--confirm",
                preview["data"]["confirmation_id"].as_str().unwrap(),
            ],
            status,
            code,
            schema,
        )
    }
}

fn text(name: &str, bytes: &[u8], root: &Path, ids: &[(&str, &str)]) {
    let text = std::str::from_utf8(bytes).unwrap();
    let root = fs::canonicalize(root).unwrap();
    let mut placeholders = vec![(root.to_str().unwrap(), "<repository>")];
    placeholders.extend_from_slice(ids);
    cli::assert_cli_text_golden(name, text, &placeholders);
}

#[test]
fn the_verbs_that_take_no_input_refuse_input_as_a_usage_error() {
    let f = super::index::Fixture::new();
    let root = f.repo.root.to_str().unwrap();
    let operation = OperationId::new().to_string();
    for args in [
        vec!["item", "sync", "--repo", root, "--id", items::TICKET_A],
        vec!["repo", "sync", "--repo", root],
        vec![
            "operation",
            "resume",
            "--repo",
            root,
            "--operation-id",
            &operation,
        ],
        vec![
            "operation",
            "abandon",
            "--repo",
            root,
            "--operation-id",
            &operation,
        ],
    ] {
        let mut args = args.clone();
        args.extend(["--request-id", REQUEST_1, "--input", "-", "--json"]);
        let value = checked(&args, &f.env, b"{}", 2, "usage_error", None);
        assert!(value["request_id"].is_null(), "{value}");
    }
}

#[test]
fn repo_sync_without_a_publication_remote_is_refused_with_nothing_done() {
    let f = super::index::Fixture::new();
    let root = f.repo.root.to_str().unwrap();
    let before = support::repository_and_worktree_snapshot(&f.repo);
    let value = checked(
        &[
            "repo",
            "sync",
            "--repo",
            root,
            "--request-id",
            REQUEST_1,
            "--json",
        ],
        &f.env,
        &[],
        3,
        "publication_remote_required",
        None,
    );
    assert_eq!(value["request_id"], REQUEST_1);
    golden(
        "repo_sync_publication_remote_required",
        &value,
        &f.repo.root,
    );
    let human = run(
        &["repo", "sync", "--repo", root, "--request-id", REQUEST_2],
        &f.env,
        &[],
    );
    assert_eq!(human.status, 3);
    assert!(human.stdout.is_empty());
    text(
        "repo_sync_text",
        &human.stderr,
        &f.repo.root,
        &[(REQUEST_2, "<request-id>")],
    );
    assert_eq!(support::repository_and_worktree_snapshot(&f.repo), before);
}

#[test]
fn item_sync_reads_the_item_s_kind_and_refuses_an_unknown_item() {
    let f = super::index::Fixture::new();
    let root = f.repo.root.to_str().unwrap();
    let value = checked(
        &[
            "item",
            "sync",
            "--repo",
            root,
            "--id",
            items::TICKET_ABSENT,
            "--request-id",
            REQUEST_1,
            "--json",
        ],
        &f.env,
        &[],
        2,
        "item_not_found",
        None,
    );
    assert_eq!(value["request_id"], REQUEST_1);
    assert_eq!(value["scope"]["item_id"], items::TICKET_ABSENT);
    golden("item_sync_item_not_found", &value, &f.repo.root);
    // The read refused it: no request was recorded.
    checked(
        &["request", "show", "--request-id", REQUEST_1, "--json"],
        &f.env,
        &[],
        2,
        "request_not_found",
        None,
    );
    // An item that is there, a ticket or a document, reaches the mutation,
    // which needs a publication remote.
    for (id, request) in [(items::TICKET_A, REQUEST_2), (items::DOCUMENT_A, REQUEST_3)] {
        let value = checked(
            &[
                "item",
                "sync",
                "--repo",
                root,
                "--id",
                id,
                "--request-id",
                request,
                "--json",
            ],
            &f.env,
            &[],
            3,
            "publication_remote_required",
            None,
        );
        assert_eq!(value["command"], "item sync");
        assert_eq!(value["scope"]["item_id"], id);
    }
    let human = run(
        &[
            "item",
            "sync",
            "--repo",
            root,
            "--id",
            items::TICKET_A,
            "--request-id",
            OTHER_REQUEST,
        ],
        &f.env,
        &[],
    );
    assert_eq!(human.status, 3);
    assert!(human.stdout.is_empty());
    text(
        "item_sync_text",
        &human.stderr,
        &f.repo.root,
        &[(OTHER_REQUEST, "<request-id>")],
    );
    // A malformed item ID is a usage of the flag, with the request named.
    let value = checked(
        &[
            "item",
            "sync",
            "--repo",
            root,
            "--id",
            "not-a-ulid",
            "--request-id",
            REQUEST_1,
            "--json",
        ],
        &f.env,
        &[],
        2,
        "invalid_id",
        None,
    );
    assert_eq!(value["request_id"], REQUEST_1);
}

#[test]
fn a_resume_of_an_operation_no_journal_holds_is_operation_not_found() {
    let f = super::index::Fixture::new();
    let root = f.repo.root.to_str().unwrap();
    let operation = OperationId::new().to_string();
    let value = checked(
        &[
            "operation",
            "resume",
            "--repo",
            root,
            "--operation-id",
            &operation,
            "--request-id",
            REQUEST_1,
            "--json",
        ],
        &f.env,
        &[],
        2,
        "operation_not_found",
        None,
    );
    golden("operation_resume_operation_not_found", &value, &f.repo.root);
    let value = checked(
        &[
            "operation",
            "resume",
            "--repo",
            root,
            "--operation-id",
            "not-a-ulid",
            "--request-id",
            REQUEST_1,
            "--json",
        ],
        &f.env,
        &[],
        2,
        "invalid_id",
        None,
    );
    assert_eq!(value["request_id"], REQUEST_1);
}

#[test]
fn a_resume_runs_a_save_s_hand_off_is_replayed_and_refuses_another_operation() {
    let f = Owing::new();
    let commit = f.world.branch_tip(items::TICKET_A).unwrap();
    let resumed = f.call(
        "resume",
        &f.operation,
        &["--request-id", REQUEST_2],
        0,
        "ok",
        None,
    );
    assert_eq!(resumed["effects"]["discovery"], "current");
    assert!(resumed["operation_id"].is_null());
    golden("operation_resume_ok", &resumed, &f.world.root);
    assert_eq!(f.world.pending_journal_rows(), 0);
    assert_eq!(
        f.world.record(REQUEST_2).unwrap().state,
        RequestState::Finished
    );
    // The same request again: its stored result, and nothing runs.
    let replay = f.call(
        "resume",
        &f.operation,
        &["--request-id", REQUEST_2],
        0,
        "ok",
        None,
    );
    assert_eq!(replay, resumed);
    // The same request ID for another operation is another request.
    let other = OperationId::new().to_string();
    let mismatch = f.call(
        "resume",
        &other,
        &["--request-id", REQUEST_2],
        2,
        "request_mismatch",
        None,
    );
    assert_eq!(mismatch["request_id"], REQUEST_2);
    assert_eq!(f.world.branch_tip(items::TICKET_A), Some(commit));
}

#[test]
fn a_human_resume_prints_its_progress_on_stderr_and_json_prints_none() {
    let f = Owing::new();
    let human = run(
        &f.args("resume", &f.operation, &["--request-id", REQUEST_2]),
        &f.env,
        &[],
    );
    assert_eq!(human.status, 0);
    assert_eq!(
        std::str::from_utf8(&human.stderr).unwrap(),
        "progress: accepted\nprogress: observing\nprogress: indexing\n"
    );
    text(
        "operation_resume_text",
        &human.stdout,
        &f.world.root,
        &[(REQUEST_2, "<request-id>")],
    );
    let f = Owing::new();
    let mut args = f.args("resume", &f.operation, &["--request-id", REQUEST_2]);
    args.push("--json");
    let out = run(&args, &f.env, &[]);
    assert_eq!(out.status, 0);
    // One envelope, and nothing on stderr.
    cli::json(&out);
}

/// R-30: C3's interrupted save, whose file was then edited by hand, is
/// closed by `operation abandon`, and the file stays as it is.
#[test]
fn abandon_closes_c3_s_stuck_save_and_leaves_its_file_as_it_is() {
    let f = Owing::new();
    let edited = f.stuck();
    let commit = f.world.branch_tip(items::TICKET_A).unwrap();
    let before = support::repository_and_worktree_snapshot(&f.world.fixture);
    let preview = f.prepare();
    assert_eq!(preview["data"]["command"], "operation abandon");
    assert_eq!(preview["data"]["details"]["request_id"], REQUEST_1);
    golden("operation_abandon_preview", &preview, &f.world.root);
    assert_eq!(
        support::repository_and_worktree_snapshot(&f.world.fixture),
        before
    );
    assert_eq!(f.world.pending_journal_rows(), 1);

    let abandoned = f.confirm(REQUEST_2, &preview, 0, "ok");
    assert_eq!(abandoned["data"]["operation_id"], f.operation);
    assert_eq!(abandoned["data"]["released_request_id"], REQUEST_1);
    assert_eq!(abandoned["data"]["committed"], true);
    golden("operation_abandon_ok", &abandoned, &f.world.root);
    // Nothing on disk or in Git changed: the edit is still there.
    assert_eq!(fs::read(f.file()).unwrap(), edited);
    assert_eq!(f.world.branch_tip(items::TICKET_A), Some(commit));
    assert_eq!(
        support::repository_and_worktree_snapshot(&f.world.fixture),
        before
    );
    assert_eq!(f.world.pending_journal_rows(), 0);
    assert!(f.world.record(REQUEST_1).is_none(), "released");

    // A replay returns the stored result; another operation under the same
    // request ID is another request.
    let replay = f.confirm(REQUEST_2, &preview, 0, "ok");
    assert_eq!(replay, abandoned);
    let other = OperationId::new().to_string();
    let mismatch = f.call(
        "abandon",
        &other,
        &["--request-id", REQUEST_2],
        2,
        "request_mismatch",
        None,
    );
    assert_eq!(mismatch["request_id"], REQUEST_2);

    // The repository goes on.
    checked(
        &[
            "index",
            "refresh",
            "--repo",
            f.root(),
            "--request-id",
            REQUEST_3,
            "--json",
        ],
        &f.env,
        &[],
        0,
        "ok",
        None,
    );
}

#[test]
fn a_human_abandon_prints_c2_s_lines() {
    let f = Owing::new();
    let preview = f.prepare();
    let human = run(
        &f.args(
            "abandon",
            &f.operation,
            &[
                "--request-id",
                REQUEST_2,
                "--confirm",
                preview["data"]["confirmation_id"].as_str().unwrap(),
            ],
        ),
        &f.env,
        &[],
    );
    assert_eq!(human.status, 0);
    assert_eq!(
        std::str::from_utf8(&human.stderr).unwrap(),
        "progress: accepted\nprogress: writing\n"
    );
    text(
        "operation_abandon_text",
        &human.stdout,
        &f.world.root,
        &[
            (REQUEST_2, "<request-id>"),
            (&f.operation, "<operation-id>"),
        ],
    );
}

/// A confirmation presented after the operation's row moved is
/// `external_change`, and closes nothing.
#[test]
fn an_abandon_confirmed_after_the_row_moved_is_external_change() {
    let f = Owing::new();
    let preview = f.prepare();
    // The save's own request finishes the operation.
    let finished = f
        .world
        .execute(REQUEST_1, f.world.save(items::TICKET_A, &f.token));
    assert_eq!(finished.code, ResultCode::Ok);
    let before = support::repository_and_worktree_snapshot(&f.world.fixture);
    let refused = f.confirm(REQUEST_2, &preview, 3, "external_change");
    assert!(refused["data"].is_null());
    assert!(f.world.record(REQUEST_2).is_none());
    assert_eq!(
        f.world.record(REQUEST_1).unwrap().state,
        RequestState::Finished
    );
    assert_eq!(
        support::repository_and_worktree_snapshot(&f.world.fixture),
        before
    );
}

#[test]
fn an_abandon_of_a_remote_operation_is_refused_and_changes_nothing() {
    let world = World::new();
    let connection = items::index(world.data.path());
    connection
        .execute_batch("UPDATE remote_polling_state SET remote_name='origin',primary_branch='main'")
        .unwrap();
    let remote = OperationId::new();
    let target = RemoteOperationTarget::for_context(
        &RemoteRefPlan::from_configuration("origin", "main").unwrap(),
        RemoteOperationAction::SynchronizeContext,
        AuthoringKind::Ticket,
        items::item_id(items::TICKET_A),
    )
    .unwrap();
    assert!(matches!(
        world
            .service
            .reserve_remote_operation(&world.root, remote, &target)
            .unwrap(),
        RemoteReservationOutcome::Reserved(_)
    ));
    let count = |table: &str| -> i64 {
        connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap()
    };
    let before = (
        support::repository_and_worktree_snapshot(&world.fixture),
        count("remote_operation_records"),
        count("confirmation_records"),
        count("request_records"),
    );
    let env = CliEnv::new(world.data.path().to_owned());
    let root = world.root.to_str().unwrap();
    let remote = remote.to_string();
    let args = |extra: &[&'static str]| {
        let mut args = vec![
            "operation",
            "abandon",
            "--repo",
            root,
            "--operation-id",
            remote.as_str(),
        ];
        args.extend_from_slice(extra);
        args.push("--json");
        args
    };
    // Only `--prepare` reaches the library's refusal of an operation that
    // another journal holds.
    let refused = checked(&args(&["--prepare"]), &env, &[], 2, "invalid_input", None);
    assert!(refused["data"].is_null());
    // With no preview there is no confirmation to run it with. C2's
    // confirmation check answers this before anything reads the operation,
    // so it shows only that nothing is changed.
    checked(
        &args(&["--request-id", REQUEST_2]),
        &env,
        &[],
        3,
        "confirmation_required",
        None,
    );
    assert_eq!(
        before,
        (
            support::repository_and_worktree_snapshot(&world.fixture),
            count("remote_operation_records"),
            count("confirmation_records"),
            count("request_records"),
        )
    );
}

#[cfg(unix)]
mod terminal {
    use super::*;
    use crate::support::pty::PtyCli;
    use std::ffi::OsStr;

    fn spawn(f: &Owing) -> PtyCli {
        PtyCli::spawn(
            &f.args("abandon", &f.operation, &[])
                .iter()
                .map(OsStr::new)
                .collect::<Vec<_>>(),
            &f.env,
            &[],
        )
    }

    fn allocated(out: &CliRun) -> String {
        let stderr = std::str::from_utf8(&out.stderr).unwrap();
        stderr
            .lines()
            .find_map(|line| line.strip_prefix("request: "))
            .unwrap()
            .to_owned()
    }

    #[test]
    fn consent_at_a_terminal_abandons_the_operation() {
        let f = Owing::new();
        let mut p = spawn(&f);
        p.expect("Proceed? [y/N] ");
        p.send(b"yes\n");
        let (out, transcript) = p.finish();
        assert_eq!(out.status, 0, "{}", String::from_utf8_lossy(&out.stderr));
        assert!(transcript.contains("Preview of operation abandon"));
        let request = allocated(&out);
        assert!(
            std::str::from_utf8(&out.stdout)
                .unwrap()
                .contains(&format!("request: {request}"))
        );
        assert_eq!(f.world.pending_journal_rows(), 0);
        assert!(f.world.record(REQUEST_1).is_none(), "released");
    }

    /// Review Focus 4: a Ctrl-C at the consent prompt is the prompt's own
    /// cancellation. Nothing was executed, so nothing was reserved.
    #[test]
    fn ctrl_c_at_the_consent_prompt_is_cancelled_and_leaves_the_operation_pending() {
        let f = Owing::new();
        let before = support::repository_and_worktree_snapshot(&f.world.fixture);
        let mut p = spawn(&f);
        p.expect("Proceed? [y/N] ");
        p.send(&[3]);
        let (before_mode, after_mode) = p.lflag_before_and_after();
        assert_eq!(after_mode, before_mode);
        let (out, _) = p.finish();
        assert_eq!(out.status, 130);
        assert!(out.stdout.is_empty());
        let stderr = std::str::from_utf8(&out.stderr).unwrap();
        assert!(stderr.contains("code: cancelled\n"), "{stderr}");
        assert!(!stderr.contains("progress:"), "{stderr}");
        let request = allocated(&out);
        assert!(f.world.record(&request).is_none(), "no record of {request}");
        assert_eq!(f.world.pending_journal_rows(), 1);
        assert_eq!(
            f.world.record(REQUEST_1).unwrap().state,
            RequestState::Accepted
        );
        assert_eq!(
            support::repository_and_worktree_snapshot(&f.world.fixture),
            before
        );
        checked(
            &["request", "show", "--request-id", &request, "--json"],
            &f.env,
            &[],
            2,
            "request_not_found",
            None,
        );
    }
}
