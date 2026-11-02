//! `poll configure`, `poll pause` and `poll resume` through the binary: the
//! matrix's cells for these verbs, on one local repository, since none of them
//! touches a remote. Each writes one field in its request's own transaction,
//! so a request is either absent or finished. The library's table
//! (`tests/mutation_remote/c5_poll.rs`) pins the transaction. `poll once` is in
//! `tests/cli_remote/polling.rs`.
//!
//! No cell for Ctrl-C or `request cancel`: these verbs have no safe point.

use std::{
    ffi::OsStr,
    fs,
    process::{Command, Stdio},
};

use manyhands::repository::LeaseKind;
use serde_json::{Value, json};

use super::{c4_support::environment, mutation};
use crate::support::{
    self,
    cli::{self, CliEnv, CliRun},
    mutation::{REQUEST_1, REQUEST_2, REQUEST_3, World},
};

const SCHEMA: &str = "poll_policy_mutation.schema.json";
const WRITING: &str = "progress: writing\n";
const SUSPENDED_NOTE: &str = "note: automatic polls are suspended for recovery, and poll resume does not lift that; poll once still runs.\n";

struct Case {
    world: World,
    env: CliEnv,
}

impl Case {
    fn new() -> Self {
        let world = World::new();
        let env = environment(world.data.path());
        Self { world, env }
    }

    fn root(&self) -> &str {
        self.world.root.to_str().unwrap()
    }

    fn args<'a>(&'a self, verb: &'a str, extra: &[&'a str]) -> Vec<&'a str> {
        let mut args = vec!["poll", verb, "--repo", self.root()];
        if verb == "configure" {
            args.extend(["--input", "-"]);
        }
        args.extend_from_slice(extra);
        args
    }

    /// One JSON-mode run: one envelope on stdout and nothing on stderr.
    fn json(
        &self,
        verb: &str,
        extra: &[&str],
        input: &[u8],
        status: i32,
        code: &str,
        schema: Option<&str>,
    ) -> Value {
        let mut args = self.args(verb, extra);
        args.push("--json");
        let out = mutation::run(&args, &self.env, input);
        assert!(out.stderr.is_empty(), "{:?}", out.stderr);
        let value = cli::json(&out);
        assert_eq!(value["code"], code, "{value}");
        assert_eq!(out.status, status, "{value}");
        assert_eq!(value["command"], format!("poll {verb}"));
        crate::support::schema::check_published("envelope.schema.json", &value).unwrap();
        if let Some(schema) = schema {
            crate::support::schema::check_published(schema, &value["data"]).unwrap();
        }
        value
    }

    fn written(&self, verb: &str, request: &str, input: &[u8], code: &str) -> Value {
        self.json(
            verb,
            &["--request-id", request],
            input,
            0,
            code,
            Some(SCHEMA),
        )
    }

    fn text(&self, verb: &str, extra: &[&str], input: &[u8]) -> CliRun {
        mutation::run(&self.args(verb, extra), &self.env, input)
    }

    fn sql(&self, statement: &str) {
        let path = self
            .world
            .data
            .path()
            .join(manyhands::repository::REGISTRY_FILE);
        rusqlite::Connection::open(path)
            .unwrap()
            .execute_batch(statement)
            .unwrap();
    }

    fn status(&self) -> Value {
        let args = ["poll", "status", "--repo", self.root(), "--json"];
        mutation::checked(&args, &self.env, &[], 0, "ok", None)["data"].clone()
    }

    /// The policy, without what a poll or the clock moves.
    fn policy(&self) -> (Value, Value, Value, Value, Value) {
        let status = self.status();
        (
            status["paused"].clone(),
            status["interval_seconds"].clone(),
            status["backoff_seconds"].clone(),
            status["recovery_suspended"].clone(),
            status["next_eligible_at"].clone(),
        )
    }

    fn request_absent(&self, request: &str) {
        let args = ["request", "show", "--request-id", request, "--json"];
        mutation::checked(&args, &self.env, &[], 2, "request_not_found", None);
    }

    fn text_golden(&self, name: &str, run: &CliRun) {
        let text = format!(
            "{}--- stderr\n{}--- exit {}\n",
            String::from_utf8_lossy(&run.stdout),
            String::from_utf8_lossy(&run.stderr),
            run.status
        );
        let text = text.replace(self.root(), "<repository>");
        cli::assert_cli_text_golden(name, &text, &[]);
    }
}

fn stdout(run: &CliRun) -> &str {
    std::str::from_utf8(&run.stdout).unwrap()
}

fn stderr(run: &CliRun) -> &str {
    std::str::from_utf8(&run.stderr).unwrap()
}

fn interval(seconds: u64) -> Vec<u8> {
    serde_json::to_vec(&json!({"interval_seconds": seconds})).unwrap()
}

#[test]
fn pause_answers_writes_and_keeps_the_rest() {
    let case = Case::new();
    let before = case.policy();
    let value = case.written("pause", REQUEST_1, &[], "ok");
    assert_eq!(value["data"]["paused"], true);
    assert_eq!(value["outcome"], "success");
    assert!(value["recovery"].as_array().unwrap().is_empty());
    mutation::golden("poll_pause", &value, &case.world.root);
    let after = case.policy();
    assert_eq!(after.0, true);
    assert_eq!(
        (&after.1, &after.2, &after.3),
        (&before.1, &before.2, &before.3)
    );
}

#[test]
fn pause_in_text_says_so_and_reports_writing_only() {
    let case = Case::new();
    let run = case.text("pause", &["--request-id", REQUEST_1], &[]);
    assert_eq!(run.status, 0);
    assert_eq!(stderr(&run), WRITING, "one stage, and no `accepted`");
    assert!(stdout(&run).starts_with("The request completed.\n"));
    assert!(stdout(&run).contains("Paused: true\n"));
    case.text_golden("poll_pause_text", &run);
}

#[test]
fn pause_already_so_is_a_noop_in_both_modes() {
    let case = Case::new();
    case.written("pause", REQUEST_1, &[], "ok");
    let value = case.written("pause", REQUEST_2, &[], "already_applied");
    assert_eq!(value["outcome"], "noop");
    mutation::golden("poll_pause_noop", &value, &case.world.root);
    let run = case.text("pause", &["--request-id", REQUEST_3], &[]);
    assert_eq!(run.status, 0);
    assert!(stdout(&run).starts_with("Already applied; nothing changed.\n"));
    case.text_golden("poll_pause_noop_text", &run);
}

#[test]
fn resume_clears_the_pause_and_the_deadline_and_leaves_the_backoff() {
    let case = Case::new();
    case.written("pause", REQUEST_1, &[], "ok");
    case.sql("UPDATE remote_polling_state SET next_eligible_at=4102444800, automatic_backoff_seconds=120");
    let value = case.written("resume", REQUEST_2, &[], "ok");
    assert_eq!(value["data"]["paused"], false);
    assert_eq!(value["data"]["next_eligible_at"], Value::Null);
    assert_eq!(value["data"]["backoff_seconds"], 120);
    assert_eq!(value["data"]["recovery_suspended"], false);
    mutation::golden("poll_resume", &value, &case.world.root);
    let after = case.policy();
    assert_eq!(after.0, false);
    assert_eq!(after.2, 120);
    assert_eq!(after.4, Value::Null);
}

#[test]
fn resume_not_paused_writes_nothing_and_keeps_a_future_deadline() {
    let case = Case::new();
    case.sql("UPDATE remote_polling_state SET next_eligible_at=4102444800");
    let before = case.policy();
    let value = case.written("resume", REQUEST_1, &[], "already_applied");
    assert_eq!(value["outcome"], "noop");
    assert_eq!(case.policy(), before);
    mutation::golden("poll_resume_noop", &value, &case.world.root);
    let run = case.text("resume", &["--request-id", REQUEST_2], &[]);
    assert_eq!(run.status, 0);
    assert_eq!(stderr(&run), WRITING, "and no note");
    case.text_golden("poll_resume_noop_text", &run);
}

/// Decision 5: the suspension stays, and the answer names it.
#[test]
fn resume_names_a_suspension_it_does_not_lift() {
    let case = Case::new();
    case.written("pause", REQUEST_1, &[], "ok");
    case.sql("UPDATE remote_polling_state SET recovery_suspended=1");
    let run = case.text("resume", &["--request-id", REQUEST_2], &[]);
    assert_eq!(run.status, 0);
    assert!(stdout(&run).contains("Recovery suspended: true\n"));
    assert_eq!(stderr(&run), format!("{WRITING}{SUSPENDED_NOTE}"));
    case.text_golden("poll_resume_suspended_text", &run);
    assert_eq!(case.status()["recovery_suspended"], true, "not lifted");
    assert_eq!(case.status()["paused"], false);

    // The JSON answer says so in its data, and holds no note.
    let replay = case.json(
        "resume",
        &["--request-id", REQUEST_2],
        &[],
        0,
        "ok",
        Some(SCHEMA),
    );
    assert_eq!(replay["data"]["recovery_suspended"], true);
    mutation::golden("poll_resume_suspended", &replay, &case.world.root);
}

#[test]
fn a_suspension_is_named_on_a_noop_and_by_the_other_verbs() {
    let case = Case::new();
    case.sql("UPDATE remote_polling_state SET recovery_suspended=1");
    for (verb, request) in [("resume", REQUEST_1), ("pause", REQUEST_2)] {
        let run = case.text(verb, &["--request-id", request], &[]);
        assert_eq!(run.status, 0);
        assert_eq!(stderr(&run), format!("{WRITING}{SUSPENDED_NOTE}"), "{verb}");
    }
    let run = case.text("configure", &["--request-id", REQUEST_3], &interval(600));
    assert_eq!(stderr(&run), format!("{WRITING}{SUSPENDED_NOTE}"));
}

#[test]
fn configure_sets_the_interval_and_leaves_the_due_time() {
    let case = Case::new();
    case.sql("UPDATE remote_polling_state SET next_eligible_at=4102444800");
    let before = case.policy();
    let value = case.written("configure", REQUEST_1, &interval(600), "ok");
    assert_eq!(value["data"]["interval_seconds"], 600);
    mutation::golden("poll_configure", &value, &case.world.root);
    let after = case.policy();
    assert_eq!(after.1, 600);
    assert_eq!(after.4, before.4, "the stored due time stays");
    assert_eq!(after.0, before.0);
}

#[test]
fn configure_in_text_and_the_same_value() {
    let case = Case::new();
    let run = case.text("configure", &["--request-id", REQUEST_1], &interval(600));
    assert_eq!(run.status, 0);
    assert_eq!(stderr(&run), WRITING);
    assert!(stdout(&run).contains("Interval seconds: 600\n"));
    case.text_golden("poll_configure_text", &run);
    let value = case.written("configure", REQUEST_2, &interval(600), "already_applied");
    assert_eq!(value["outcome"], "noop");
    mutation::golden("poll_configure_noop", &value, &case.world.root);
}

#[test]
fn the_interval_s_bounds_hold_and_nothing_is_written_beyond_them() {
    let case = Case::new();
    for (request, seconds) in [(REQUEST_1, 60), (REQUEST_2, 3600)] {
        let value = case.written("configure", request, &interval(seconds), "ok");
        assert_eq!(value["data"]["interval_seconds"], seconds);
    }
    let before = case.policy();
    for seconds in [59, 3601, 0] {
        let value = case.json(
            "configure",
            &["--request-id", REQUEST_3],
            &interval(seconds),
            2,
            "invalid_input",
            None,
        );
        assert!(value["data"].is_null());
        if seconds == 3601 {
            mutation::golden("poll_configure_out_of_range", &value, &case.world.root);
        }
        assert_eq!(case.policy(), before, "{seconds}");
        case.request_absent(REQUEST_3);
        // Human mode names the range.
        let run = case.text(
            "configure",
            &["--request-id", REQUEST_3],
            &interval(seconds),
        );
        assert_eq!(run.status, 2);
        assert!(
            stderr(&run).contains("Input field interval_seconds must be from 60 to 3600.\n"),
            "{}",
            stderr(&run)
        );
        if seconds == 3601 {
            case.text_golden("poll_configure_out_of_range_text", &run);
        }
        case.request_absent(REQUEST_3);
    }
}

#[test]
fn input_failures_answer_invalid_input_and_write_nothing() {
    let case = Case::new();
    let before = case.policy();
    for (input, detail) in [
        (
            &br#"{}"#[..],
            "Required input field is missing: interval_seconds.",
        ),
        (
            br#"{"interval_seconds":"600"}"#,
            "Input field interval_seconds must be a whole number.",
        ),
        (
            br#"{"interval_seconds":600.5}"#,
            "Input field interval_seconds must be a whole number.",
        ),
        (
            br#"{"interval_seconds":-60}"#,
            "Input field interval_seconds must be a whole number.",
        ),
        (
            br#"{"interval_seconds":600.0}"#,
            "Input field interval_seconds must be a whole number.",
        ),
        (
            br#"{"interval_seconds":null}"#,
            "Input field interval_seconds must be a whole number.",
        ),
        (
            br#"{"interval_seconds":99999999999999999999999}"#,
            "Input field interval_seconds must be a whole number.",
        ),
        (
            br#"{"interval_seconds":600,"enabled":false}"#,
            "Unknown input field: enabled.",
        ),
        (
            br#"{"interval_seconds":600,"interval_seconds":700}"#,
            "Duplicate input field: interval_seconds.",
        ),
        (b"", "The input is empty."),
    ] {
        let value = case.json(
            "configure",
            &["--request-id", REQUEST_1],
            input,
            2,
            "invalid_input",
            None,
        );
        assert!(value["data"].is_null());
        let run = case.text("configure", &["--request-id", REQUEST_1], input);
        assert_eq!(run.status, 2);
        assert!(run.stdout.is_empty());
        assert!(stderr(&run).contains(detail), "{}", stderr(&run));
    }
    assert_eq!(case.policy(), before);
    case.request_absent(REQUEST_1);
    let run = case.text(
        "configure",
        &["--request-id", REQUEST_1],
        br#"{"interval_seconds":"600"}"#,
    );
    case.text_golden("poll_configure_invalid_text", &run);
}

#[test]
fn usage_failures_reach_nothing_in_both_modes() {
    let case = Case::new();
    let before = case.policy();
    for (verb, extra, input, code) in [
        ("pause", &[][..], &[][..], "usage_error"),
        ("resume", &[][..], &[][..], "usage_error"),
        ("configure", &[][..], &interval(600)[..], "usage_error"),
        ("pause", &["--request-id", "bad"][..], &[][..], "invalid_id"),
        (
            "pause",
            &["--request-id", REQUEST_1, "--input", "-"][..],
            &[][..],
            "usage_error",
        ),
        (
            "resume",
            &["--request-id", REQUEST_1, "--id", "X"][..],
            &[][..],
            "usage_error",
        ),
        ("pause", &["--prepare"][..], &[][..], "not_confirmable"),
        (
            "pause",
            &["--request-id", REQUEST_1, "--confirm", REQUEST_2][..],
            &[][..],
            "not_confirmable",
        ),
        (
            "configure",
            &["--prepare"][..],
            &interval(600)[..],
            "not_confirmable",
        ),
    ] {
        let value = case.json(verb, extra, input, 2, code, None);
        assert!(value["data"].is_null(), "{value}");
        let run = case.text(verb, extra, input);
        assert_eq!(run.status, 2);
        assert!(run.stdout.is_empty());
        assert!(
            stderr(&run).contains(&format!("code: {code}\n")),
            "{}",
            stderr(&run)
        );
    }
    let value = case.json("pause", &[], &[], 2, "usage_error", None);
    mutation::golden("poll_pause_usage", &value, &case.world.root);
    assert_eq!(case.policy(), before);
    // `configure` with no `--input` at all.
    let args = [
        "poll",
        "configure",
        "--repo",
        case.root(),
        "--request-id",
        REQUEST_1,
    ];
    let run = mutation::run(&args, &case.env, &[]);
    assert_eq!(run.status, 2);
    assert!(
        stderr(&run).contains("--input is required."),
        "{}",
        stderr(&run)
    );
}

/// A committed request replays what it reported then, and a later pause stands.
#[test]
fn a_replay_is_the_stored_answer_and_writes_nothing() {
    let case = Case::new();
    case.written("pause", REQUEST_1, &[], "ok");
    let resumed = case.written("resume", REQUEST_2, &[], "ok");
    let configured = case.written("configure", REQUEST_3, &interval(900), "ok");
    assert_eq!(configured["data"]["paused"], false);
    let now = case.policy();
    assert_eq!(case.written("resume", REQUEST_2, &[], "ok"), resumed);
    assert_eq!(
        case.written("configure", REQUEST_3, &interval(900), "ok"),
        configured
    );
    let replayed = case.written("pause", REQUEST_1, &[], "ok");
    assert_eq!(replayed["data"]["paused"], true, "as it was then");
    assert_eq!(replayed["data"]["interval_seconds"], 300, "as it was then");
    assert_eq!(case.policy(), now);
}

#[test]
fn a_request_id_sent_with_another_intent_is_a_mismatch() {
    let case = Case::new();
    case.written("configure", REQUEST_1, &interval(600), "ok");
    let now = case.policy();
    for (verb, input) in [
        ("configure", &interval(900)[..]),
        ("pause", &[][..]),
        ("resume", &[][..]),
    ] {
        let value = case.json(
            verb,
            &["--request-id", REQUEST_1],
            input,
            2,
            "request_mismatch",
            None,
        );
        assert!(value["data"].is_null());
        if verb == "pause" {
            mutation::golden("poll_pause_request_mismatch", &value, &case.world.root);
        }
    }
    let run = case.text("pause", &["--request-id", REQUEST_1], &[]);
    assert_eq!(run.status, 2);
    case.text_golden("poll_pause_request_mismatch_text", &run);
    assert_eq!(case.policy(), now);
}

/// A request refused before its transaction was never sent, as far as anything
/// can tell, so the same ID is a new write.
#[test]
fn a_request_that_never_committed_runs_when_it_is_sent_again() {
    let case = Case::new();
    let args = ["--request-id", REQUEST_1];
    case.json("configure", &args, &interval(59), 2, "invalid_input", None);
    let value = case.written("configure", REQUEST_1, &interval(600), "ok");
    assert_eq!(value["data"]["interval_seconds"], 600);

    // Held in a second process through every bounded retry, and then free.
    let holder = support::hold_lease_in_child_for_test(
        &case.world.root,
        case.world.data.path(),
        LeaseKind::CacheWrite,
        "index_lease_child",
    );
    let refused = case.json("pause", &["--request-id", REQUEST_2], &[], 5, "busy", None);
    assert!(refused["data"].is_null());
    holder.release();
    case.request_absent(REQUEST_2);
    assert_eq!(case.status()["paused"], false);
    let value = case.written("pause", REQUEST_2, &[], "ok");
    assert_eq!(value["data"]["paused"], true);
}

/// A reply that never arrived: the write stands, the process says it couldn't
/// answer, and the same request gives the answer with no second write.
#[test]
fn lost_stdout_then_the_same_request_is_the_answer() {
    let case = Case::new();
    let args = case.args("pause", &["--request-id", REQUEST_1, "--json"]);
    let argv: Vec<_> = args.iter().map(OsStr::new).collect();
    let mut command: Command = cli::cli_command(&argv, &case.env);
    // A pipe whose read end is closed before the process starts, so no write
    // to stdout can ever succeed.
    let (reader, writer) = std::io::pipe().unwrap();
    drop(reader);
    let status = command
        .stdin(Stdio::null())
        .stdout(writer)
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert!(!status.success(), "nothing could be written to stdout");
    assert_eq!(case.status()["paused"], true, "the write stands");
    case.sql("UPDATE remote_polling_state SET interval_seconds=900");
    let value = case.written("pause", REQUEST_1, &[], "ok");
    assert_eq!(value["data"]["interval_seconds"], 300, "the stored answer");
    assert_eq!(case.status()["interval_seconds"], 900, "no second write");
}

#[test]
fn a_repository_the_policy_cannot_be_written_for_is_refused() {
    let case = Case::new();
    let plain = support::born_repository();
    let root = plain.root.to_str().unwrap();
    for verb in ["pause", "resume"] {
        let args = [
            "poll",
            verb,
            "--repo",
            root,
            "--request-id",
            REQUEST_1,
            "--json",
        ];
        let value = mutation::checked(&args, &case.env, &[], 2, "repository_not_enabled", None);
        assert!(value["data"].is_null());
    }
    let missing = case.env.home.path().join("missing");
    let args = [
        "poll",
        "pause",
        "--repo",
        missing.to_str().unwrap(),
        "--request-id",
        REQUEST_1,
        "--json",
    ];
    mutation::checked(&args, &case.env, &[], 2, "invalid_path", None);
    assert!(!fs::exists(&missing).unwrap());
}

#[test]
fn a_relative_repository_is_taken_from_the_invocation_directory() {
    let case = Case::new();
    let env = environment(case.world.data.path()).in_dir(case.world.root.clone());
    let args = [
        "poll",
        "pause",
        "--repo",
        ".",
        "--request-id",
        REQUEST_1,
        "--json",
    ];
    let value = mutation::checked(&args, &env, &[], 0, "ok", Some(SCHEMA));
    assert_eq!(value["data"]["paused"], true);
}

/// The published example reaches the verb, and sets what it says.
pub fn the_example_input_is_accepted(example: &Value) {
    let case = Case::new();
    let input = serde_json::to_vec(example).unwrap();
    let value = case.written("configure", REQUEST_1, &input, "ok");
    assert_eq!(
        value["data"]["interval_seconds"],
        example["interval_seconds"]
    );
}
