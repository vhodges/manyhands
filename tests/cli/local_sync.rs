//! `context observe-local`, `context show-local` and `context sync-local`
//! through the binary, a row of the design's required outcomes per test. No row
//! needs a server: nothing here reaches a remote, and
//! `nothing_here_contacts_a_remote_or_moves_primary` shows it.
//!
//! Two rows stage their stop in this process, since the failure seam is the
//! library's and no CLI child can have it (as `tests/cli/authoring_exits.rs`
//! records of the same seam): `discovery_pending`, and a closure past
//! `Closing`. The binary then gives the answer that follows.

use std::{ffi::OsStr, fs, net::TcpListener, path::Path, process::Command};

use manyhands::{
    repository::{
        AuthoringKind, FailurePoint, LeaseKind, LocalConflictResolutionInput,
        LocalContextResolveInput, LocalContextSyncInput, Mutation, OperationId, TicketCloseInput,
    },
    results::{FailureClass, ResultCode},
};
use serde_json::{Value, json};

use super::c4_support::environment;
use crate::support::{
    self, FailOnce,
    cli::{self, CliEnv, CliRun, stderr, stdout},
    items,
    mutation::{
        DOCUMENT, DOCUMENT_PATH, REQUEST_1, REQUEST_2, REQUEST_3, TICKET_CLOSED, World,
        execute_confirmed,
    },
    operations,
};

const SAVE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX8";
const TICKET: &str = items::TICKET_A;
const UNKNOWN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FZZ";
const UNKNOWN_OPERATION: &str = "01ARZ3NDEKTSV4RRFFQ69G5FZY";
const REMOTE_OPERATION: &str = "01ARZ3NDEKTSV4RRFFQ69G5FZX";
const OBSERVATION_SCHEMA: &str = "local_context_sync_observation.schema.json";
const INSPECTION_SCHEMA: &str = "local_context_sync_inspection.schema.json";
const CONTENT_SCHEMA: &str = "local_context_conflict_content.schema.json";
const MUTATION_SCHEMA: &str = "local_context_sync_mutation.schema.json";
/// Well-formed in the library's sense, and naming nothing that exists.
const OTHER_OBSERVATION: &str =
    "local-sync-v1:0000000000000000000000000000000000000000000000000000000000000000";
const INCOMING_TITLE: &str = "Incoming conflict sentinel";
const REQUEST_4: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX5";
/// Planted in a submitted source: the index may hold it, as the resolved
/// item's title, and nothing else in application data or any output may.
const SOURCE_SENTINEL: &str = "SENTINEL-SOURCE-4b7e";

struct Case {
    world: World,
    env: CliEnv,
}

impl Case {
    fn over(world: World) -> Self {
        let env = environment(world.data.path());
        Self { world, env }
    }

    fn new() -> Self {
        Self::over(World::new())
    }

    /// The ticket with an editing context primary does not hold.
    fn saved() -> Self {
        let case = Self::new();
        case.save();
        case
    }

    fn save(&self) {
        let token = self.world.token(TICKET);
        let answer = self.world.execute(SAVE, self.world.save(TICKET, &token));
        assert_eq!(answer.code, ResultCode::Ok, "{:?}", answer.data);
    }

    /// A context and a primary that each changed the ticket's file.
    fn conflicted(extra: &str) -> (Self, String) {
        let case = Self::saved();
        case.primary_moved_the_ticket(extra);
        let observation = case.observation();
        let value = case.sync_json(REQUEST_2, &observation, 4, "recovery_required");
        let operation = value["operation_id"].as_str().unwrap().to_owned();
        (case, operation)
    }

    fn root(&self) -> &str {
        self.world.root.to_str().unwrap()
    }

    fn run(&self, args: &[&str], input: &[u8]) -> CliRun {
        cli::run_cli_with_stdin(
            &args.iter().map(OsStr::new).collect::<Vec<_>>(),
            &self.env,
            input,
        )
    }

    /// One JSON-mode run: one envelope on stdout and nothing on stderr.
    fn json(
        &self,
        args: &[&str],
        input: &[u8],
        status: i32,
        code: &str,
        schema: Option<&str>,
    ) -> Value {
        let mut args = args.to_vec();
        args.push("--json");
        let out = self.run(&args, input);
        assert!(
            out.stderr.is_empty(),
            "{args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let value = cli::json(&out);
        assert_eq!(value["code"], code, "{value}");
        assert_eq!(out.status, status, "{value}");
        crate::support::schema::check_published("envelope.schema.json", &value).unwrap();
        if let Some(schema) = schema {
            crate::support::schema::check_published(schema, &value["data"]).unwrap();
        }
        value
    }

    fn observe_args<'a>(&'a self, id: &'a str) -> Vec<&'a str> {
        vec![
            "context",
            "observe-local",
            "--repo",
            self.root(),
            "--id",
            id,
        ]
    }

    fn observe(&self, id: &str, status: i32, code: &str) -> Value {
        let schema = (status == 0).then_some(OBSERVATION_SCHEMA);
        self.json(&self.observe_args(id), &[], status, code, schema)
    }

    fn observation(&self) -> String {
        self.observe(TICKET, 0, "ok")["data"]["observation"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    fn sync_args<'a>(&'a self, request: &'a str) -> Vec<&'a str> {
        vec![
            "context",
            "sync-local",
            "--repo",
            self.root(),
            "--id",
            TICKET,
            "--input",
            "-",
            "--request-id",
            request,
        ]
    }

    fn sync_json(&self, request: &str, observation: &str, status: i32, code: &str) -> Value {
        let before = self.primary_refs();
        let input = input(observation);
        let value = self.json(&self.sync_args(request), &input, status, code, None);
        assert_eq!(value["command"], "context sync-local");
        if !value["data"].is_null() {
            crate::support::schema::check_published(MUTATION_SCHEMA, &value["data"]).unwrap();
        }
        assert_eq!(self.primary_refs(), before, "primary or a remote ref moved");
        value
    }

    fn sync_text(&self, request: &str, observation: &str) -> CliRun {
        self.run(&self.sync_args(request), &input(observation))
    }

    fn show_args<'a>(&'a self, operation: &'a str, token: Option<&'a str>) -> Vec<&'a str> {
        let mut args = vec![
            "context",
            "show-local",
            "--repo",
            self.root(),
            "--operation-id",
            operation,
        ];
        if let Some(token) = token {
            args.extend(["--token", token]);
        }
        args
    }

    fn show(&self, operation: &str, status: i32, code: &str) -> Value {
        let schema = (status == 0).then_some(INSPECTION_SCHEMA);
        self.json(&self.show_args(operation, None), &[], status, code, schema)
    }

    fn git(&self, directory: &Path, args: &[&str]) -> String {
        let mut command = Command::new("git");
        cli::cli_environment(&mut command, &self.env);
        let output = command
            .arg("-C")
            .arg(directory)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    }

    fn refs(&self) -> String {
        self.git(&self.world.root, &["for-each-ref", "refs/"])
    }

    /// Primary and every remote-tracking ref: what no verb here may move.
    fn primary_refs(&self) -> String {
        self.git(
            &self.world.root,
            &["for-each-ref", "refs/heads/main", "refs/remotes/"],
        )
    }

    fn context(&self) -> std::path::PathBuf {
        self.world.worktree(TICKET)
    }

    fn primary_moved_the_ticket(&self, extra: &str) {
        let path = items::ticket_path(TICKET);
        items::write(
            &self.world.root,
            &path,
            &items::ticket_source(TICKET, INCOMING_TITLE, extra),
        );
        items::commit(&self.world.fixture, &[&path], items::COMMITTED_AT + 60);
    }

    /// A commit on primary touching no file of the context's.
    fn primary_moved_elsewhere(&self) {
        let path = "elsewhere.txt";
        fs::write(self.world.root.join(path), "elsewhere\n").unwrap();
        items::commit(&self.world.fixture, &[path], items::COMMITTED_AT + 60);
    }

    fn forget_identity(&self) {
        let repository = git2::Repository::open(&self.world.root).unwrap();
        let config = repository.config().unwrap();
        let mut local = config.open_level(git2::ConfigLevel::Local).unwrap();
        local.remove("user.name").unwrap();
        local.remove("user.email").unwrap();
    }

    fn request_absent(&self, request: &str) {
        let args = ["request", "show", "--request-id", request];
        self.json(&args, &[], 2, "request_not_found", None);
    }

    fn text_golden(&self, name: &str, run: &CliRun) {
        cli::text_golden_scrubbed(
            name,
            run,
            &[(self.root(), "<repository>")],
            &['-', ':', '_'],
            scrubbed_word,
        );
    }

    fn golden(&self, name: &str, value: &Value) {
        let mut replacements = vec![(self.root().to_owned(), "<repository>")];
        for (pointer, label) in [
            ("/operation_id", "<operation-id>"),
            ("/data/operation_id", "<operation-id>"),
            ("/data/observation", "<observation>"),
            ("/data/context_head", "<context-head>"),
            ("/data/primary_head", "<primary-head>"),
            ("/data/context_parent", "<context-parent>"),
            ("/data/primary_parent", "<primary-parent>"),
            ("/data/result_oid", "<result-oid>"),
            ("/data/token", "<token>"),
            ("/data/paths/0/token", "<token>"),
            ("/effects/commit_oid", "<commit-oid>"),
        ] {
            if let Some(text) = value.pointer(pointer).and_then(Value::as_str)
                && !replacements.iter().any(|(known, _)| known == text)
            {
                replacements.push((text.into(), label));
            }
        }
        cli::assert_cli_golden(
            name,
            value,
            &replacements
                .iter()
                .map(|(value, label)| (value.as_str(), *label))
                .collect::<Vec<_>>(),
        );
    }
}

fn input(observation: &str) -> Vec<u8> {
    json!({ "observation": observation })
        .to_string()
        .into_bytes()
}

/// What a run makes up: commits, observations, tokens, the IDs no fixture fixes.
fn scrubbed_word(word: &str) -> String {
    let fixed = [
        TICKET,
        DOCUMENT,
        UNKNOWN,
        UNKNOWN_OPERATION,
        REMOTE_OPERATION,
        REQUEST_1,
        REQUEST_2,
        REQUEST_3,
        REQUEST_4,
    ];
    if word.starts_with("local-sync-v1:") {
        "<observation>".into()
    } else if word.starts_with("local-conflict-v1:") {
        "<token>".into()
    } else if word.len() == 40 && word.chars().all(|c| c.is_ascii_hexdigit()) {
        "<oid>".into()
    } else if word.len() == 26
        && word
            .chars()
            .all(|c| c.is_ascii_digit() || c.is_ascii_uppercase())
        && !fixed.contains(&word)
    {
        "<id>".into()
    } else {
        word.to_owned()
    }
}

fn tampered(token: &str) -> String {
    let (head, last) = token.split_at(token.len() - 1);
    format!("{head}{}", if last == "0" { "1" } else { "0" })
}

// ---------------------------------------------------------------- the reads

#[test]
fn an_observation_reads_the_context_and_writes_nothing() {
    let case = Case::saved();
    let before = case.refs();
    let value = case.observe(TICKET, 0, "ok");
    assert_eq!(value["command"], "context observe-local");
    assert_eq!(value["data"]["kind"], "ticket");
    assert_eq!(value["data"]["item_id"], TICKET);
    case.golden("context_observe_local", &value);
    let run = case.run(&case.observe_args(TICKET), &[]);
    assert_eq!(run.status, 0);
    assert!(
        stdout(&run).contains(value["data"]["observation"].as_str().unwrap()),
        "{}",
        stdout(&run)
    );
    case.text_golden("context_observe_local_text", &run);
    assert_eq!(case.refs(), before);
}

#[test]
fn an_observation_of_a_document_finds_its_kind_from_the_item() {
    let case = Case::new();
    items::create_document_context(
        &case.world.service,
        &case.world.root,
        UNKNOWN,
        "docs/new.md",
    );
    let value = case.observe(UNKNOWN, 0, "ok");
    assert_eq!(value["data"]["kind"], "document");
}

#[test]
fn an_unknown_item_is_item_not_found_for_both_verbs() {
    let case = Case::saved();
    let value = case.observe(UNKNOWN, 2, "item_not_found");
    assert_eq!(value["recovery"], json!([]));
    case.golden("context_observe_local_unknown", &value);
    let run = case.run(&case.observe_args(UNKNOWN), &[]);
    assert_eq!(run.status, 2);
    assert!(!stderr(&run).contains("save"), "{}", stderr(&run));
    case.text_golden("context_observe_local_unknown_text", &run);

    let mut args = case.sync_args(REQUEST_1);
    args[5] = UNKNOWN;
    let value = case.json(&args, &input(OTHER_OBSERVATION), 2, "item_not_found", None);
    assert_eq!(value["request_id"], REQUEST_1);
    case.request_absent(REQUEST_1);
}

#[test]
fn a_malformed_id_is_invalid_id() {
    let case = Case::new();
    case.observe("not-a-ulid", 2, "invalid_id");
    case.show("not-a-ulid", 2, "invalid_id");
    let mut args = case.sync_args(REQUEST_1);
    args[5] = "not-a-ulid";
    case.json(&args, &input(OTHER_OBSERVATION), 2, "invalid_id", None);
}

#[test]
fn an_item_with_no_context_is_refused_and_none_is_made() {
    let case = Case::new();
    let before = case.refs();
    for (id, save) in [(TICKET, "ticket save"), (DOCUMENT, "document save")] {
        let value = case.observe(id, 2, "item_not_found");
        assert_eq!(value["recovery"], json!([]), "{value}");
        let run = case.run(&case.observe_args(id), &[]);
        assert_eq!(run.status, 2);
        assert!(stderr(&run).contains(save), "{}", stderr(&run));
        assert!(
            stderr(&run)
                .starts_with("error: This item has no editing context.\ncode: item_not_found\n"),
            "{}",
            stderr(&run)
        );
        if id == TICKET {
            case.golden("context_observe_local_no_context", &value);
            case.text_golden("context_observe_local_no_context_text", &run);
        }
    }
    assert_eq!(case.refs(), before);
    assert!(!case.world.worktree(TICKET).exists());
}

#[test]
fn an_unknown_operation_is_operation_not_found() {
    let case = Case::new();
    let value = case.show(UNKNOWN_OPERATION, 2, "operation_not_found");
    case.golden("context_show_local_unknown", &value);
    let run = case.run(&case.show_args(UNKNOWN_OPERATION, None), &[]);
    assert_eq!(run.status, 2);
    case.text_golden("context_show_local_unknown_text", &run);
}

#[test]
fn a_remote_operation_is_invalid_input_pointing_to_conflict_show() {
    let case = Case::new();
    operations::configure_remote(case.world.data.path());
    operations::insert_remote_synchronization(
        case.world.data.path(),
        REMOTE_OPERATION,
        Some(("ticket", TICKET)),
        "interrupted",
        Some("before_fetch"),
        None,
    );
    let value = case.show(REMOTE_OPERATION, 2, "invalid_input");
    assert_eq!(value["data"], Value::Null);
    case.golden("context_show_local_remote_operation", &value);
    let run = case.run(&case.show_args(REMOTE_OPERATION, None), &[]);
    assert_eq!(run.status, 2);
    assert!(stderr(&run).contains("conflict show"), "{}", stderr(&run));
    case.text_golden("context_show_local_remote_operation_text", &run);
}

#[test]
fn a_remote_poll_is_invalid_input_and_names_no_conflict_show() {
    let case = Case::new();
    operations::configure_remote(case.world.data.path());
    operations::insert_remote_poll(
        case.world.data.path(),
        REMOTE_OPERATION,
        "reserved",
        None,
        None,
    );
    case.show(REMOTE_OPERATION, 2, "invalid_input");
    let run = case.run(&case.show_args(REMOTE_OPERATION, None), &[]);
    assert_eq!(run.status, 2);
    assert!(!stderr(&run).contains("conflict show"), "{}", stderr(&run));
}

#[test]
fn another_local_operation_is_invalid_input_and_names_no_conflict_show() {
    let case = Case::new();
    let token = case.world.token(TICKET);
    let saved = case.world.execute(SAVE, case.world.save(TICKET, &token));
    let operation = saved.operation_id.unwrap();
    case.show(&operation, 2, "invalid_input");
    let run = case.run(&case.show_args(&operation, None), &[]);
    assert!(!stderr(&run).contains("conflict show"), "{}", stderr(&run));
}

#[test]
fn an_inspection_lists_a_conflicts_tokens_and_no_side() {
    let (case, operation) = Case::conflicted("");
    let before = case.refs();
    let value = case.show(&operation, 0, "ok");
    assert_eq!(value["data"]["operation_id"], *operation);
    assert_eq!(value["data"]["state"], "conflict_pending");
    let text = value.to_string();
    assert!(!text.contains(INCOMING_TITLE), "{text}");
    case.golden("context_show_local", &value);
    let run = case.run(&case.show_args(&operation, None), &[]);
    assert_eq!(run.status, 0, "{}", stderr(&run));
    let shown = stdout(&run);
    assert!(
        shown.contains(value["data"]["paths"][0]["token"].as_str().unwrap()),
        "{shown}"
    );
    assert!(!shown.contains(INCOMING_TITLE), "{shown}");
    case.text_golden("context_show_local_text", &run);
    assert_eq!(case.refs(), before);
}

fn token_of(case: &Case, operation: &str) -> String {
    case.show(operation, 0, "ok")["data"]["paths"][0]["token"]
        .as_str()
        .unwrap()
        .to_owned()
}

#[test]
fn a_token_with_json_reads_the_sides() {
    let (case, operation) = Case::conflicted("");
    let token = token_of(&case, &operation);
    let args = case.show_args(&operation, Some(&token));
    let value = case.json(&args, &[], 0, "ok", Some(CONTENT_SCHEMA));
    assert!(
        value["data"]["incoming"]
            .as_str()
            .unwrap()
            .contains(INCOMING_TITLE),
        "{value}"
    );
    assert_eq!(value["data"]["historical"], false);
    for side in ["base", "local", "incoming"] {
        assert!(value["data"][side].is_string(), "{side}: {value}");
    }
}

#[test]
fn a_token_in_human_mode_is_a_usage_error_pointing_to_json() {
    let (case, operation) = Case::conflicted("");
    let token = token_of(&case, &operation);
    let run = case.run(&case.show_args(&operation, Some(&token)), &[]);
    assert_eq!(run.status, 2);
    assert!(stdout(&run).is_empty());
    assert!(
        stderr(&run).contains("code: usage_error"),
        "{}",
        stderr(&run)
    );
    assert!(stderr(&run).contains("--json"), "{}", stderr(&run));
    assert!(!stderr(&run).contains(INCOMING_TITLE), "{}", stderr(&run));
    case.text_golden("context_show_local_token_human", &run);
}

/// Review Focus 5: nothing is read for a token that doesn't name a path, and a
/// valid one on the same operation shows what would have been read.
#[test]
fn a_tampered_foreign_or_stale_token_is_invalid_input_with_nothing_read() {
    let (case, operation) = Case::conflicted("");
    let token = token_of(&case, &operation);
    let (other, other_operation) = Case::conflicted("");
    let foreign = token_of(&other, &other_operation);
    assert_ne!(token, foreign);
    let good = case.json(
        &case.show_args(&operation, Some(&token)),
        &[],
        0,
        "ok",
        Some(CONTENT_SCHEMA),
    );
    assert!(good.to_string().contains(INCOMING_TITLE));
    for bad in [
        tampered(&token).as_str(),
        foreign.as_str(),
        "local-conflict-v1:short",
        "../../etc/passwd",
    ] {
        let args = case.show_args(&operation, Some(bad));
        let value = case.json(&args, &[], 2, "invalid_input", None);
        assert_eq!(value["data"], Value::Null, "{bad}");
        assert!(!value.to_string().contains(INCOMING_TITLE), "{bad}");
    }
    let tamper = tampered(&token);
    let refused = case.json(
        &case.show_args(&operation, Some(&tamper)),
        &[],
        2,
        "invalid_input",
        None,
    );
    case.golden("context_show_local_token_tampered", &refused);

    // Stale: the operation has finished, and its path is no longer in conflict.
    let view = case.show(&operation, 0, "ok");
    let resolved = case.world.execute(
        REQUEST_3,
        Mutation::LocalContextResolve(LocalContextResolveInput {
            root: case.world.root.clone(),
            operation_id: OperationId::parse(&operation).unwrap(),
            observation: view["data"]["observation"].as_str().unwrap().to_owned(),
            resolutions: vec![LocalConflictResolutionInput {
                token: token.clone(),
                source: items::ticket_source(TICKET, "Resolved", ""),
            }],
        }),
    );
    assert_eq!(resolved.code, ResultCode::Ok, "{:?}", resolved.data);
    let args = case.show_args(&operation, Some(&token));
    let value = case.json(&args, &[], 2, "invalid_input", None);
    assert_eq!(value["data"], Value::Null);
}

// ------------------------------------------------------------ `sync-local`

#[test]
fn a_contained_primary_is_already_applied_and_equal_too() {
    let case = Case::saved();
    let observation = case.observation();
    let value = case.sync_json(REQUEST_1, &observation, 0, "already_applied");
    assert_eq!(value["data"]["state"], "complete");
    case.golden("context_sync_local_noop", &value);
    let run = case.sync_text(REQUEST_2, &case.observation());
    assert_eq!(run.status, 0, "{}", stderr(&run));
    case.text_golden("context_sync_local_noop_text", &run);

    // Equal: primary at the context's tip.
    let tip = World::branch(TICKET);
    case.git(&case.world.root, &["merge", "--ff-only", "--quiet", &tip]);
    let observation = case.observation();
    case.sync_json(REQUEST_3, &observation, 0, "already_applied");
}

#[test]
fn a_fast_forward_is_ok() {
    let case = Case::saved();
    let tip = World::branch(TICKET);
    case.git(&case.world.root, &["merge", "--ff-only", "--quiet", &tip]);
    case.primary_moved_elsewhere();
    let primary = case.world.primary_head();
    let observation = case.observation();
    let value = case.sync_json(REQUEST_1, &observation, 0, "ok");
    assert_eq!(value["data"]["state"], "complete");
    assert_eq!(case.world.branch_tip(TICKET), Some(primary));
    case.golden("context_sync_local_fast_forward", &value);
}

#[test]
fn a_clean_merge_is_ok_and_its_replay_answers_the_same() {
    let case = Case::saved();
    case.primary_moved_elsewhere();
    let observation = case.observation();
    let value = case.sync_json(REQUEST_1, &observation, 0, "ok");
    assert_eq!(value["data"]["state"], "complete");
    assert!(case.context().join("elsewhere.txt").exists());
    case.golden("context_sync_local_merge", &value);
    let tip = case.world.branch_tip(TICKET);

    // Replay: the stored answer, though the observation has gone stale.
    let again = case.sync_json(REQUEST_1, &observation, 0, "ok");
    assert_eq!(again, value);
    assert_eq!(case.world.branch_tip(TICKET), tip);
}

#[test]
fn a_clean_merge_in_text_says_what_it_did() {
    let case = Case::saved();
    case.primary_moved_elsewhere();
    let run = case.sync_text(REQUEST_1, &case.observation());
    assert_eq!(run.status, 0, "{}", stderr(&run));
    assert!(stdout(&run).contains("State: complete"), "{}", stdout(&run));
    case.text_golden("context_sync_local_merge_text", &run);
}

#[test]
fn another_observation_under_a_request_is_a_mismatch() {
    let case = Case::saved();
    case.primary_moved_elsewhere();
    let observation = case.observation();
    case.sync_json(REQUEST_1, &observation, 0, "ok");
    let tip = case.world.branch_tip(TICKET);
    let value = case.sync_json(REQUEST_1, OTHER_OBSERVATION, 2, "request_mismatch");
    case.golden("context_sync_local_request_mismatch", &value);
    let run = case.sync_text(REQUEST_1, OTHER_OBSERVATION);
    assert_eq!(run.status, 2);
    case.text_golden("context_sync_local_request_mismatch_text", &run);
    assert_eq!(case.world.branch_tip(TICKET), tip);
}

/// Staged in this process: see the file's comment. The library's answer is what
/// the binary prints, class and all, and the binary's same request finishes it.
#[test]
fn discovery_pending_is_finished_by_the_same_request() {
    let case = Case::over(World::with_service(|data| {
        FailOnce::at(FailurePoint::LocalSyncBeforeIndex).open_service(data)
    }));
    case.save();
    case.primary_moved_elsewhere();
    let observation = case.observation();
    let answer = case.world.execute(
        REQUEST_1,
        Mutation::LocalContextSync(LocalContextSyncInput {
            root: case.world.root.clone(),
            kind: AuthoringKind::Ticket,
            id: items::item_id(TICKET),
            observation: observation.clone(),
        }),
    );
    assert_eq!(
        answer.code,
        ResultCode::DiscoveryPending,
        "{:?}",
        answer.data
    );
    assert_eq!(answer.failure_class(), Some(FailureClass::Incomplete));
    case.golden(
        "context_sync_local_discovery_pending",
        &serde_json::to_value(&answer).unwrap(),
    );
    let tip = case.world.branch_tip(TICKET);
    let value = case.sync_json(REQUEST_1, &observation, 0, "ok");
    assert_eq!(
        value["operation_id"].as_str(),
        answer.operation_id.as_deref()
    );
    assert_eq!(case.world.branch_tip(TICKET), tip);
}

#[test]
fn a_malformed_observation_is_invalid_input_and_points_to_observe() {
    let case = Case::saved();
    let value = case.sync_json(REQUEST_1, "malformed", 2, "invalid_input");
    assert!(value["data"].is_null(), "{value}");
    case.golden("context_sync_local_malformed", &value);
    let run = case.sync_text(REQUEST_2, "malformed");
    assert_eq!(run.status, 2);
    assert!(
        stderr(&run).contains("context observe-local"),
        "{}",
        stderr(&run)
    );
    case.text_golden("context_sync_local_malformed_text", &run);
}

#[test]
fn a_stale_observation_is_external_change_and_points_to_observe() {
    let case = Case::saved();
    let observation = case.observation();
    case.primary_moved_elsewhere();
    let tip = case.world.branch_tip(TICKET);
    let value = case.sync_json(REQUEST_1, &observation, 3, "external_change");
    case.golden("context_sync_local_stale", &value);
    let run = case.sync_text(REQUEST_2, &observation);
    assert_eq!(run.status, 3);
    assert!(
        stderr(&run).contains("context observe-local"),
        "{}",
        stderr(&run)
    );
    // Observing again is the way on: no repair outside is claimed, and no
    // operation is named when the refusal keeps none.
    assert!(
        !stderr(&run).contains("external_repair"),
        "{}",
        stderr(&run)
    );
    assert!(!stderr(&run).contains("operation:"), "{}", stderr(&run));
    assert!(!stderr(&run).contains(": none\n"), "{}", stderr(&run));
    case.text_golden("context_sync_local_stale_text", &run);
    assert_eq!(case.world.branch_tip(TICKET), tip);
}

#[test]
fn a_context_on_the_wrong_branch_is_external_change() {
    let case = Case::saved();
    let observation = case.observation();
    case.git(&case.context(), &["switch", "--quiet", "-c", "elsewhere"]);
    case.observe(TICKET, 3, "external_change");
    case.sync_json(REQUEST_1, &observation, 3, "external_change");
}

#[test]
fn a_dirty_context_is_worktree_not_clean() {
    let case = Case::saved();
    let observation = case.observation();
    fs::write(case.context().join("scratch.txt"), "dirty\n").unwrap();
    case.observe(TICKET, 3, "worktree_not_clean");
    let value = case.sync_json(REQUEST_1, &observation, 3, "worktree_not_clean");
    case.golden("context_sync_local_dirty", &value);
    let run = case.sync_text(REQUEST_2, &observation);
    assert_eq!(run.status, 3);
    assert!(
        stderr(&run).contains("context observe-local"),
        "{}",
        stderr(&run)
    );
    case.text_golden("context_sync_local_dirty_text", &run);
}

#[test]
fn no_identity_on_a_merge_is_identity_required_then_the_same_request() {
    let case = Case::saved();
    case.primary_moved_elsewhere();
    let observation = case.observation();
    case.forget_identity();
    let value = case.sync_json(REQUEST_1, &observation, 3, "identity_required");
    let actions: Vec<_> = value["recovery"]
        .as_array()
        .unwrap()
        .iter()
        .map(|action| action["action"].as_str().unwrap())
        .collect();
    assert_eq!(actions[0], "repo.identity_set", "{value}");
    case.golden("context_sync_local_identity_required", &value);
    let run = case.sync_text(REQUEST_1, &observation);
    assert_eq!(run.status, 3);
    assert!(stderr(&run).contains("same request"), "{}", stderr(&run));
    case.text_golden("context_sync_local_identity_required_text", &run);
    case.world.set_local_config("user.name", "Restored");
    case.world
        .set_local_config("user.email", "restored@example.invalid");
    case.sync_json(REQUEST_1, &observation, 0, "ok");
}

fn assert_refused_without_effect(case: &Case, observation: &str, golden: &str) {
    let tip = case.world.branch_tip(TICKET);
    let value = case.sync_json(REQUEST_1, observation, 3, "recovery_required");
    case.golden(golden, &value);
    let run = case.sync_text(REQUEST_2, observation);
    assert_eq!(run.status, 3);
    assert!(stderr(&run).contains("repair outside"), "{}", stderr(&run));
    case.text_golden(&format!("{golden}_text"), &run);
    assert_eq!(case.world.branch_tip(TICKET), tip);
    assert!(
        value["operation_id"].is_null() || value["data"]["state"] == "refused",
        "{value}"
    );
    // No owner: the context goes on being observed, and nothing is listed.
    let list = ["operation", "list", "--repo", case.root()];
    let listed = case.json(&list, &[], 0, "ok", None);
    assert_eq!(listed["data"]["items"], json!([]), "{listed}");
    assert_eq!(case.observation(), observation);
}

#[test]
fn unrelated_histories_are_refused_with_no_owner_or_effect() {
    let case = Case::saved();
    let repository = git2::Repository::open(&case.world.root).unwrap();
    let tip = repository
        .find_commit(case.world.branch_tip(TICKET).unwrap())
        .unwrap();
    let signature = git2::Signature::now("Test", "test@example.invalid").unwrap();
    let orphan = repository
        .commit(
            None,
            &signature,
            &signature,
            "orphan",
            &tip.tree().unwrap(),
            &[],
        )
        .unwrap();
    repository
        .reference(
            &format!("refs/heads/{}", World::branch(TICKET)),
            orphan,
            true,
            "orphan",
        )
        .unwrap();
    let observation = case.observation();
    assert_refused_without_effect(&case, &observation, "context_sync_local_unrelated");
}

#[test]
fn incompatible_closures_are_refused_with_no_owner_or_effect() {
    let case = Case::saved();
    let tip = World::branch(TICKET);
    case.git(&case.world.root, &["merge", "--ff-only", "--quiet", &tip]);
    let path = items::ticket_path(TICKET);
    for (directory, who) in [(case.context(), "One"), (case.world.root.clone(), "Two")] {
        let file = directory.join(&path);
        let source = fs::read_to_string(&file).unwrap().replacen(
            "---\n",
            &format!("---\nclosed_at: 2026-10-25T12:00:00Z\nclosed_by: {who}\n"),
            1,
        );
        fs::write(file, source).unwrap();
        case.git(&directory, &["add", "--", &path]);
        case.git(&directory, &["commit", "--quiet", "-m", "closed"]);
    }
    let observation = case.observation();
    assert_refused_without_effect(&case, &observation, "context_sync_local_closures");
}

#[cfg(unix)]
#[test]
fn an_unsupported_entry_is_refused_with_no_owner_or_effect() {
    let case = Case::saved();
    std::os::unix::fs::symlink("elsewhere", case.world.root.join("a-link")).unwrap();
    items::commit(&case.world.fixture, &["a-link"], items::COMMITTED_AT + 60);
    let observation = case.observation();
    assert_refused_without_effect(&case, &observation, "context_sync_local_unsupported");
}

#[test]
fn an_eligible_conflict_stops_with_its_operation_and_the_way_on() {
    let case = Case::saved();
    case.primary_moved_the_ticket("");
    let observation = case.observation();
    let value = case.sync_json(REQUEST_1, &observation, 4, "recovery_required");
    let operation = value["operation_id"].as_str().unwrap();
    assert_eq!(value["data"]["operation_id"], operation);
    assert_eq!(value["data"]["state"], "conflict_pending");
    assert_eq!(value["data"]["next_action"], "resolve");
    assert_eq!(value["data"]["eligibility"], "eligible_canonical");
    case.golden("context_sync_local_conflict", &value);
    // The same request gets the same stop in text, naming both verbs.
    let run = case.sync_text(REQUEST_1, &observation);
    assert_eq!(run.status, 4);
    let lines = stderr(&run);
    assert!(
        lines.contains(&format!(
            "context show-local --repo '{}' --operation-id {operation}",
            case.root()
        )),
        "{lines}"
    );
    assert!(lines.contains("context resolve-local"), "{lines}");
    assert!(!lines.contains(INCOMING_TITLE), "{lines}");
    case.text_golden("context_sync_local_conflict_text", &run);
}

#[test]
fn a_mixed_conflict_is_external_repair_only() {
    let (case, operation) = Case::conflicted("invalid yaml line\n");
    let view = case.show(&operation, 0, "ok");
    assert_eq!(view["data"]["next_action"], "external_repair");
    assert_eq!(view["data"]["eligibility"], "external_resolution_required");
}

#[test]
fn a_mixed_conflict_stops_with_no_resolve_line() {
    let case = Case::saved();
    case.primary_moved_the_ticket("invalid yaml line\n");
    let observation = case.observation();
    let value = case.sync_json(REQUEST_1, &observation, 4, "recovery_required");
    assert_eq!(value["data"]["next_action"], "external_repair");
    case.golden("context_sync_local_external_only", &value);
    let run = case.sync_text(REQUEST_1, &observation);
    assert_eq!(run.status, 4);
    assert!(
        !stderr(&run).contains("context resolve-local"),
        "{}",
        stderr(&run)
    );
    case.text_golden("context_sync_local_external_only_text", &run);
}

#[test]
fn another_owner_of_the_lease_is_busy() {
    let case = Case::saved();
    case.primary_moved_elsewhere();
    let observation = case.observation();
    let holder = support::hold_lease_in_child_for_test(
        &case.world.root,
        case.world.data.path(),
        LeaseKind::CacheWrite,
        "index_lease_child",
    );
    let value = case.sync_json(REQUEST_1, &observation, 5, "busy");
    holder.release();
    case.golden("context_sync_local_busy", &value);
    case.sync_json(REQUEST_1, &observation, 0, "ok");
}

/// Staged in this process: see the file's comment.
#[test]
fn a_closure_past_closing_is_recovery_required() {
    let case = Case::over(World::with_service(|data| {
        FailOnce::at(FailurePoint::AfterIntegrationRef).open_service(data)
    }));
    case.save();
    let close = || {
        Mutation::TicketClose(TicketCloseInput {
            root: case.world.root.clone(),
            id: items::item_id(TICKET),
        })
    };
    let preview = case.world.service.prepare(&close());
    let confirmation = preview
        .data
        .as_ref()
        .and_then(|data| data.preview())
        .unwrap()
        .confirmation_id
        .clone();
    let stopped = execute_confirmed(&case.world.service, REQUEST_3, &confirmation, close());
    assert_ne!(stopped.code, ResultCode::Ok, "{:?}", stopped.data);
    let observation = case.observation();
    let tip = case.world.branch_tip(TICKET);
    let value = case.sync_json(REQUEST_1, &observation, 3, "recovery_required");
    case.golden("context_sync_local_closure_past_closing", &value);
    assert_eq!(case.world.branch_tip(TICKET), tip);
}

#[test]
fn input_failures_are_invalid_input_with_nothing_done() {
    let case = Case::saved();
    let tip = case.world.branch_tip(TICKET);
    for (index, bad) in [
        &b"{}"[..],
        br#"{"observation":7}"#,
        br#"{"observation":"x","extra":1}"#,
        b"not json",
        b"",
    ]
    .iter()
    .enumerate()
    {
        let args = case.sync_args(REQUEST_1);
        let value = case.json(&args, bad, 2, "invalid_input", None);
        if index == 0 {
            case.golden("context_sync_local_input_failure", &value);
            let run = case.run(&args, bad);
            case.text_golden("context_sync_local_input_failure_text", &run);
        }
    }
    case.request_absent(REQUEST_1);
    assert_eq!(case.world.branch_tip(TICKET), tip);
}

#[test]
fn a_verb_without_its_options_is_a_usage_error() {
    let case = Case::saved();
    for args in [
        &["context", "observe-local", "--repo", case.root()][..],
        &["context", "show-local", "--repo", case.root()],
        &[
            "context",
            "sync-local",
            "--repo",
            case.root(),
            "--id",
            TICKET,
            "--request-id",
            REQUEST_1,
        ],
        &[
            "context",
            "sync-local",
            "--repo",
            case.root(),
            "--id",
            TICKET,
            "--input",
            "-",
        ],
    ] {
        case.json(args, b"{}", 2, "usage_error", None);
    }
}

#[test]
fn a_prepare_is_not_confirmable() {
    let case = Case::saved();
    let args = [
        "context",
        "sync-local",
        "--repo",
        case.root(),
        "--id",
        TICKET,
        "--input",
        "-",
        "--prepare",
    ];
    let value = case.json(&args, &input(OTHER_OBSERVATION), 2, "not_confirmable", None);
    assert!(value["data"].is_null(), "{value}");
}

/// `human_lifecycle::merge_refused`'s first three commands, in its argument
/// order with its placeholders filled: each parses and reaches its verb. The
/// walk from a refused closure to its finish is
/// `the_refused_merges_route_finishes_the_close`.
#[test]
fn the_refused_merges_first_three_commands_parse_and_reach_their_verbs() {
    let (case, operation) = Case::conflicted("");
    let root = case.root();
    let observe = ["context", "observe-local", "--repo", root, "--id", TICKET];
    // A context in conflict has no observation to give, and says so.
    let run = case.run(&observe, &[]);
    assert_eq!(run.status, 3, "{}", stderr(&run));
    assert!(
        stderr(&run).contains("code: worktree_conflicted"),
        "{}",
        stderr(&run)
    );
    let show = [
        "context",
        "show-local",
        "--repo",
        root,
        "--operation-id",
        &operation,
    ];
    let run = case.run(&show, &[]);
    assert_eq!(run.status, 0, "{}", stderr(&run));

    let case = Case::saved();
    case.primary_moved_elsewhere();
    let root = case.root();
    let observe = [
        "context",
        "observe-local",
        "--repo",
        root,
        "--id",
        TICKET,
        "--json",
    ];
    let observed = cli::json(&case.run(&observe, &[]));
    // Observe's observation is sync-local's input, as it is printed.
    let body = json!({ "observation": observed["data"]["observation"] }).to_string();
    let sync = [
        "context",
        "sync-local",
        "--repo",
        root,
        "--id",
        TICKET,
        "--input",
        "-",
        "--request-id",
        REQUEST_1,
    ];
    let run = case.run(&sync, body.as_bytes());
    assert_eq!(run.status, 0, "{}", stderr(&run));
}

/// A publication remote is configured, and something listens where it says.
#[test]
fn nothing_here_contacts_a_remote_or_moves_primary() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();
    let case = Case::saved();
    git2::Repository::open(&case.world.root)
        .unwrap()
        .remote(
            "origin",
            &format!("ssh://git@127.0.0.1:{port}/repository.git"),
        )
        .unwrap();
    case.world
        .commit_configuration("publication_remote = \"origin\"\n");
    items::refresh_completely(&case.world.service, &case.world.root);
    let primary = case.primary_refs();
    let observation = case.observation();
    let value = case.sync_json(REQUEST_1, &observation, 0, "ok");
    assert_eq!(value["effects"]["publication"], "not_requested", "{value}");
    case.show(value["operation_id"].as_str().unwrap(), 0, "ok");
    assert_eq!(case.primary_refs(), primary);
    // A conflict's sides, read through the same configured remote.
    case.primary_moved_the_ticket("");
    let primary = case.primary_refs();
    let observation = case.observation();
    let value = case.sync_json(REQUEST_2, &observation, 4, "recovery_required");
    let operation = value["operation_id"].as_str().unwrap().to_owned();
    let token = token_of(&case, &operation);
    let args = case.show_args(&operation, Some(&token));
    case.json(&args, &[], 0, "ok", Some(CONTENT_SCHEMA));
    assert_eq!(case.primary_refs(), primary);
    assert_eq!(
        listener.accept().err().map(|error| error.kind()),
        Some(std::io::ErrorKind::WouldBlock),
        "a verb connected"
    );
}

/// The verb's own closed input: what its example says, the binary takes.
pub fn the_example_input_is_accepted(example: &Value) {
    let case = Case::saved();
    let args = case.sync_args(REQUEST_1);
    let value = case.json(
        &args,
        example.to_string().as_bytes(),
        3,
        "external_change",
        None,
    );
    assert_eq!(value["code"], "external_change");
}

#[test]
fn a_closed_ticket_is_read_like_any_item() {
    let case = Case::new();
    case.observe(TICKET_CLOSED, 2, "item_not_found");
    // A save of a closed ticket is refused, so none is offered.
    let run = case.run(&case.observe_args(TICKET_CLOSED), &[]);
    assert_eq!(run.status, 2);
    assert!(!stderr(&run).contains("save"), "{}", stderr(&run));
}

/// The context has lost the item: no save makes it again, so none is offered.
#[test]
fn a_context_that_lost_its_item_gets_no_save_hint() {
    let case = Case::saved();
    let path = items::ticket_path(TICKET);
    case.git(&case.context(), &["rm", "--quiet", "--", &path]);
    case.git(&case.context(), &["commit", "--quiet", "-m", "removed"]);
    let before = case.refs();
    case.observe(TICKET, 2, "item_not_found");
    let run = case.run(&case.observe_args(TICKET), &[]);
    assert_eq!(run.status, 2);
    assert!(!stderr(&run).contains("save"), "{}", stderr(&run));
    assert_eq!(case.refs(), before);
}

#[test]
fn sync_local_on_an_item_with_no_context_creates_none() {
    let case = Case::new();
    let before = case.refs();
    let value = case.sync_json(REQUEST_1, OTHER_OBSERVATION, 2, "item_not_found");
    assert_eq!(value["request_id"], REQUEST_1);
    let run = case.sync_text(REQUEST_1, OTHER_OBSERVATION);
    assert_eq!(run.status, 2);
    case.request_absent(REQUEST_1);
    assert_eq!(case.refs(), before);
    assert!(!case.world.worktree(TICKET).exists());
}

#[test]
fn an_empty_observation_is_the_librarys_invalid_input() {
    let case = Case::saved();
    let tip = case.world.branch_tip(TICKET);
    let value = case.sync_json(REQUEST_1, "", 2, "invalid_input");
    assert!(value["data"].is_null(), "{value}");
    assert_eq!(case.world.branch_tip(TICKET), tip);
}

/// A listed token whose sides aren't text answers as the library does.
#[test]
fn a_listed_unreadable_token_is_the_librarys_answer() {
    let case = Case::saved();
    let path = "blob.bin";
    let context = case.context();
    fs::write(context.join(path), [0xff, 0xfe, 0x01]).unwrap();
    case.git(&context, &["add", "--", path]);
    case.git(&context, &["commit", "--quiet", "-m", "local blob"]);
    fs::write(case.world.root.join(path), [0xff, 0x00, 0x02]).unwrap();
    items::commit(&case.world.fixture, &[path], items::COMMITTED_AT + 60);
    let observation = case.observation();
    let value = case.sync_json(REQUEST_1, &observation, 4, "recovery_required");
    let operation = value["operation_id"].as_str().unwrap().to_owned();
    let view = case.show(&operation, 0, "ok");
    let entry = view["data"]["paths"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["content_available"] == false)
        .unwrap_or_else(|| panic!("no unreadable path: {view}"))
        .clone();
    let token = entry["token"].as_str().unwrap();
    let library = case
        .world
        .service
        .read_local_context_conflict(
            &case.world.resolved(),
            OperationId::parse(&operation).unwrap(),
            token,
        )
        .err()
        .map(|error| error.code().as_str())
        .unwrap_or("ok");
    assert_eq!(library, "external_change");
    let value = case.json(
        &case.show_args(&operation, Some(token)),
        &[],
        3,
        library,
        None,
    );
    assert_eq!(value["data"], Value::Null);
}

/// A state that lists no paths while it goes on: the library's to judge, so
/// the binary gives whatever its read gives.
#[test]
fn a_token_where_the_inspection_lists_no_paths_is_the_librarys_answer() {
    let (case, operation) = Case::conflicted("");
    let token = token_of(&case, &operation);
    items::index(case.world.data.path())
        .execute(
            "UPDATE local_context_sync_records SET state='recovery_required' WHERE operation_ulid=?1",
            [&operation],
        )
        .unwrap();
    let view = case.world.service.inspect_local_context_sync(
        &case.world.resolved(),
        OperationId::parse(&operation).unwrap(),
    );
    let Ok(view) = view else {
        panic!(
            "the seeded state's inspection fails: {:?}",
            view.err().map(|e| e.code())
        );
    };
    assert!(view.paths.is_empty());
    let library = case.world.service.read_local_context_conflict(
        &case.world.resolved(),
        OperationId::parse(&operation).unwrap(),
        &token,
    );
    let args = case.show_args(&operation, Some(&token));
    match library {
        Ok(_) => {
            let value = case.json(&args, &[], 0, "ok", Some(CONTENT_SCHEMA));
            assert!(value.to_string().contains(INCOMING_TITLE));
        }
        Err(error) => {
            let code = error.code();
            assert_ne!(
                code,
                ResultCode::InvalidInput,
                "the library refuses as the binary would"
            );
            let mut json_args = args.clone();
            json_args.push("--json");
            let out = case.run(&json_args, &[]);
            assert_eq!(cli::json(&out)["code"], code.as_str());
        }
    }
}

#[test]
fn the_verbs_help_is_available_and_is_its_own_golden() {
    let temp = tempfile::tempdir().unwrap();
    let env = CliEnv::new(temp.path().join("absent-data"));
    for (verb, golden) in [
        ("observe-local", "help_context_observe_local"),
        ("show-local", "help_context_show_local"),
        ("sync-local", "help_context_sync_local"),
    ] {
        let args = ["context", verb, "--help", "--json"];
        let output = cli::run_cli(&args.map(OsStr::new), &env);
        assert_eq!(output.status, 0);
        let value = cli::json(&output);
        crate::support::schema::check_published("cli_help.schema.json", &value["data"]).unwrap();
        cli::assert_cli_golden(golden, &value, &[]);
    }
    assert!(!env.data.exists());
}

// --------------------------------------------------------- `resolve-local`

impl Case {
    /// A conflict on the ticket and on the guide: two paths.
    fn two_paths_conflicted() -> (Self, String) {
        let case = Self::saved();
        let context = case.context();
        items::write(
            &context,
            DOCUMENT_PATH,
            &items::document_source(DOCUMENT, "Local guide", ""),
        );
        case.git(&context, &["add", "--", DOCUMENT_PATH]);
        case.git(&context, &["commit", "--quiet", "-m", "local guide"]);
        let path = items::ticket_path(TICKET);
        items::write(
            &case.world.root,
            &path,
            &items::ticket_source(TICKET, INCOMING_TITLE, ""),
        );
        items::write(
            &case.world.root,
            DOCUMENT_PATH,
            &items::document_source(DOCUMENT, "Incoming guide", ""),
        );
        items::commit(
            &case.world.fixture,
            &[&path, DOCUMENT_PATH],
            items::COMMITTED_AT + 60,
        );
        let observation = case.observation();
        let value = case.sync_json(REQUEST_2, &observation, 4, "recovery_required");
        let operation = value["operation_id"].as_str().unwrap().to_owned();
        (case, operation)
    }

    fn resolve_args<'a>(&'a self, operation: &'a str, request: &'a str) -> Vec<&'a str> {
        vec![
            "context",
            "resolve-local",
            "--repo",
            self.root(),
            "--operation-id",
            operation,
            "--input",
            "-",
            "--request-id",
            request,
        ]
    }

    fn resolve_json(
        &self,
        operation: &str,
        request: &str,
        input: &Value,
        status: i32,
        code: &str,
    ) -> Value {
        let before = self.primary_refs();
        let args = self.resolve_args(operation, request);
        let value = self.json(&args, input.to_string().as_bytes(), status, code, None);
        assert_eq!(value["command"], "context resolve-local");
        if !value["data"].is_null() {
            crate::support::schema::check_published(MUTATION_SCHEMA, &value["data"]).unwrap();
        }
        assert!(!value.to_string().contains(SOURCE_SENTINEL), "{value}");
        assert_eq!(self.primary_refs(), before, "primary or a remote ref moved");
        value
    }

    fn resolve_text(&self, operation: &str, request: &str, input: &Value) -> CliRun {
        let run = self.run(
            &self.resolve_args(operation, request),
            input.to_string().as_bytes(),
        );
        assert!(!stdout(&run).contains(SOURCE_SENTINEL));
        assert!(!stderr(&run).contains(SOURCE_SENTINEL));
        run
    }

    /// The live observation, and a source for each path: the ticket's carries
    /// the sentinel, and the guide's is its local side.
    fn resolution(&self, operation: &str) -> Value {
        let view = self.show(operation, 0, "ok");
        let resolutions: Vec<Value> = view["data"]["paths"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| {
                let token = entry["token"].as_str().unwrap();
                let source = if entry["display_path"] == DOCUMENT_PATH {
                    items::document_source(DOCUMENT, "Resolved guide", "")
                } else {
                    sentinel_ticket()
                };
                json!({ "token": token, "source": source })
            })
            .collect();
        json!({ "observation": view["data"]["observation"], "resolutions": resolutions })
    }

    fn state(&self, operation: &str) -> Value {
        self.show(operation, 0, "ok")["data"]["state"].clone()
    }

    fn context_files(&self) -> (String, String) {
        let context = self.context();
        (
            fs::read_to_string(context.join(items::ticket_path(TICKET))).unwrap(),
            fs::read_to_string(context.join(DOCUMENT_PATH)).unwrap(),
        )
    }
}

/// The ticket as a resolution submits it: the sentinel is in its body only,
/// which no table of the index keeps.
fn sentinel_ticket() -> String {
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {TICKET}\ntitle: Resolved\n\
         type: task\nstatus: open\n---\nThe resolved body, {SOURCE_SENTINEL}.\n"
    )
}

/// Every file of the data directory holding the sentinel, read as raw bytes:
/// the store, its WAL and anything beside them.
fn files_holding(data: &Path, sentinel: &str) -> Vec<std::path::PathBuf> {
    fn walk(directory: &Path, sentinel: &[u8], found: &mut Vec<std::path::PathBuf>) {
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, sentinel, found);
            } else if let Ok(bytes) = fs::read(&path)
                && bytes
                    .windows(sentinel.len())
                    .any(|window| window == sentinel)
            {
                found.push(path);
            }
        }
    }
    let mut found = Vec::new();
    walk(data, sentinel.as_bytes(), &mut found);
    found
}

fn assert_data_excludes_the_source(case: &Case) {
    let data = case.world.data.path();
    assert_eq!(
        cells_beyond_the_index(data, SOURCE_SENTINEL),
        Vec::<String>::new()
    );
    assert_eq!(
        files_holding(data, SOURCE_SENTINEL),
        Vec::<std::path::PathBuf>::new()
    );
}

/// Every cell of application data holding a sentinel, but the index's items.
fn cells_beyond_the_index(data: &Path, sentinel: &str) -> Vec<String> {
    let connection = items::index(data);
    let tables: Vec<String> = connection
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert!(
        tables
            .iter()
            .any(|table| table == "local_context_sync_resolutions")
    );
    let mut found = Vec::new();
    for table in tables.iter().filter(|table| *table != "discovered_items") {
        let mut statement = connection
            .prepare(&format!("SELECT * FROM \"{table}\""))
            .unwrap();
        let columns = statement.column_count();
        let mut rows = statement.query([]).unwrap();
        while let Some(row) = rows.next().unwrap() {
            for index in 0..columns {
                let bytes = match row.get_ref(index).unwrap() {
                    rusqlite::types::ValueRef::Text(bytes)
                    | rusqlite::types::ValueRef::Blob(bytes) => bytes.to_vec(),
                    _ => continue,
                };
                if String::from_utf8_lossy(&bytes).contains(sentinel) {
                    found.push(table.clone());
                }
            }
        }
    }
    found
}

#[test]
fn a_complete_valid_set_is_ok_and_keeps_its_source_out_of_application_data() {
    let (case, operation) = Case::two_paths_conflicted();
    let input = case.resolution(&operation);
    let value = case.resolve_json(&operation, REQUEST_3, &input, 0, "ok");
    assert_eq!(value["data"]["state"], "complete", "{value}");
    assert_eq!(value["data"]["operation_id"], *operation);
    case.golden("context_resolve_local", &value);
    let (ticket, guide) = case.context_files();
    assert!(ticket.contains(SOURCE_SENTINEL), "{ticket}");
    assert!(guide.contains("Resolved guide"), "{guide}");
    assert_eq!(case.state(&operation), "complete");
    assert_data_excludes_the_source(&case);

    // Replay: the stored answer, and nothing more is written.
    let tip = case.world.branch_tip(TICKET);
    let again = case.resolve_json(&operation, REQUEST_3, &input, 0, "ok");
    assert_eq!(again, value);
    assert_eq!(case.world.branch_tip(TICKET), tip);
    let run = case.resolve_text(&operation, REQUEST_3, &input);
    assert_eq!(run.status, 0, "{}", stderr(&run));
    case.text_golden("context_resolve_local_text", &run);

    // Mismatch: another source under the same request.
    let mut other = input.clone();
    other["resolutions"][0]["source"] = json!(items::ticket_source(TICKET, "Another", ""));
    let value = case.resolve_json(&operation, REQUEST_3, &other, 2, "request_mismatch");
    case.golden("context_resolve_local_request_mismatch", &value);
    assert_eq!(case.world.branch_tip(TICKET), tip);
    assert_data_excludes_the_source(&case);
}

#[test]
fn an_incomplete_duplicate_unknown_or_invalid_set_is_invalid_input_before_any_write() {
    let (case, operation) = Case::two_paths_conflicted();
    let input = case.resolution(&operation);
    let files = case.context_files();
    let tip = case.world.branch_tip(TICKET);
    let first = input["resolutions"][0].clone();
    let mut incomplete = input.clone();
    incomplete["resolutions"] = json!([first]);
    let mut duplicate = input.clone();
    duplicate["resolutions"] = json!([first.clone(), first.clone()]);
    let mut invalid = input.clone();
    invalid["resolutions"][0]["source"] =
        json!(format!("not a canonical item {SOURCE_SENTINEL}\n"));
    let mut empty = input.clone();
    empty["resolutions"] = json!([]);
    for (index, (name, bad)) in [
        ("incomplete", &incomplete),
        ("duplicate", &duplicate),
        ("invalid", &invalid),
        ("empty", &empty),
    ]
    .into_iter()
    .enumerate()
    {
        let request = [REQUEST_3, REQUEST_4][index % 2];
        let value = case.resolve_json(&operation, request, bad, 2, "invalid_input");
        assert_eq!(case.context_files(), files, "{name}");
        assert_eq!(case.world.branch_tip(TICKET), tip, "{name}");
        assert_eq!(case.state(&operation), "conflict_pending", "{name}");
        if name == "incomplete" {
            case.golden("context_resolve_local_incomplete", &value);
            let run = case.resolve_text(&operation, REQUEST_3, bad);
            assert_eq!(run.status, 2);
            assert!(
                stderr(&run).contains(&format!(
                    "next: manyhands-cli context show-local --repo '{}' --operation-id {operation}\n",
                    case.root()
                )),
                "{}",
                stderr(&run)
            );
            case.text_golden("context_resolve_local_incomplete_text", &run);
        }
    }
    assert_data_excludes_the_source(&case);
    // The complete set still resolves.
    case.resolve_json(&operation, REQUEST_1, &input, 0, "ok");
}

/// An unknown token, well formed, is `invalid_input` before any write: the
/// verb's check, as `show-local`'s, since `conflict::read` refuses it too.
#[test]
fn an_unknown_token_is_invalid_input_before_any_write() {
    let (case, operation) = Case::two_paths_conflicted();
    let input = case.resolution(&operation);
    let files = case.context_files();
    let mut unknown = input.clone();
    let token = input["resolutions"][0]["token"].as_str().unwrap();
    unknown["resolutions"][0]["token"] = json!(tampered(token));
    let value = case.resolve_json(&operation, REQUEST_3, &unknown, 2, "invalid_input");
    assert!(value["data"].is_null(), "{value}");
    case.golden("context_resolve_local_unknown_token", &value);
    case.request_absent(REQUEST_3);
    let run = case.resolve_text(&operation, REQUEST_3, &unknown);
    assert_eq!(run.status, 2);
    assert!(
        stderr(&run).contains("context show-local"),
        "{}",
        stderr(&run)
    );
    case.text_golden("context_resolve_local_unknown_token_text", &run);
    assert_eq!(case.context_files(), files);
    assert_eq!(case.state(&operation), "conflict_pending");
    assert_data_excludes_the_source(&case);
    // The same request with the live tokens then resolves.
    case.resolve_json(&operation, REQUEST_3, &input, 0, "ok");
}

#[test]
fn a_stale_observation_is_external_change_and_points_to_show_local() {
    let (case, operation) = Case::two_paths_conflicted();
    let input = case.resolution(&operation);
    // The guide is edited in the context after the inspection.
    let guide = case.context().join(DOCUMENT_PATH);
    let mut text = fs::read_to_string(&guide).unwrap();
    text.push_str("edited after the inspection\n");
    fs::write(&guide, &text).unwrap();
    let value = case.resolve_json(&operation, REQUEST_3, &input, 3, "external_change");
    assert_eq!(fs::read_to_string(&guide).unwrap(), text);
    case.golden("context_resolve_local_stale", &value);
    let run = case.resolve_text(&operation, REQUEST_4, &input);
    assert_eq!(run.status, 3);
    assert!(
        stderr(&run).contains("context show-local"),
        "{}",
        stderr(&run)
    );
    case.text_golden("context_resolve_local_stale_text", &run);
}

/// A resolution stopped after it wrote a path, in this process (see the file's
/// comment), and its operation, observation and sources.
fn partially_resolved() -> (Case, String, Value) {
    let case = Case::over(World::with_service(|data| {
        FailOnce::at(FailurePoint::LocalSyncAfterResolutionPath).open_service(data)
    }));
    case.save();
    case.primary_moved_the_ticket("");
    let observation = case.observation();
    let value = case.sync_json(REQUEST_2, &observation, 4, "recovery_required");
    let operation = value["operation_id"].as_str().unwrap().to_owned();
    let view = case.show(&operation, 0, "ok");
    let token = view["data"]["paths"][0]["token"]
        .as_str()
        .unwrap()
        .to_owned();
    let source = sentinel_ticket();
    let input = json!({
        "observation": view["data"]["observation"],
        "resolutions": [{ "token": token, "source": source }],
    });
    let stopped = case.world.execute(
        REQUEST_3,
        Mutation::LocalContextResolve(LocalContextResolveInput {
            root: case.world.root.clone(),
            operation_id: OperationId::parse(&operation).unwrap(),
            observation: input["observation"].as_str().unwrap().to_owned(),
            resolutions: vec![LocalConflictResolutionInput { token, source }],
        }),
    );
    assert_ne!(stopped.code, ResultCode::Ok, "{:?}", stopped.data);
    (case, operation, input)
}

/// The inspection and a resume name the original request, and that request,
/// with the same sources, finishes.
#[test]
fn a_partial_accepted_resolution_is_finished_by_the_same_request_and_source() {
    let (case, operation, input) = partially_resolved();
    let view = case.show(&operation, 0, "ok");
    assert_eq!(
        view["data"]["reason"], "original_resolution_required",
        "{view}"
    );
    assert_eq!(view["data"]["required_request_id"], REQUEST_3, "{view}");
    let resume = [
        "operation",
        "resume",
        "--repo",
        case.root(),
        "--operation-id",
        &operation,
        "--request-id",
        REQUEST_4,
    ];
    // A resume's own answer, `blocked`; the table has no row for it.
    let resumed = case.json(&resume, &[], 3, "original_request_required", None);
    assert_eq!(
        resumed["data"]["required_request_id"], REQUEST_3,
        "{resumed}"
    );
    assert_eq!(
        resumed["recovery"][0]["action"], "request.retry",
        "{resumed}"
    );
    assert_eq!(
        resumed["recovery"][0]["arguments"]["request_id"], REQUEST_3,
        "{resumed}"
    );
    case.golden(
        "context_operation_resume_original_request_required",
        &resumed,
    );

    let done = case.resolve_json(&operation, REQUEST_3, &input, 0, "ok");
    assert_eq!(done["data"]["state"], "complete", "{done}");
    assert_data_excludes_the_source(&case);
}

/// A new `resolve-local` request after a partial accepted resolution is
/// `external_change`, exit 3, as the gate has it: the original request is the
/// one to resend. Human output names it, from what `show-local` reads.
#[test]
fn a_new_request_after_a_partial_resolution_is_external_change_naming_the_original() {
    let (case, operation, input) = partially_resolved();
    let view = case.show(&operation, 0, "ok");
    let mut fresh = input.clone();
    fresh["observation"] = view["data"]["observation"].clone();
    // With the observation it sent, or one read again: the same refusal.
    for body in [&input, &fresh] {
        let refused = case.resolve_json(&operation, REQUEST_4, body, 3, "external_change");
        assert_eq!(refused["effects"]["write"], "not_requested", "{refused}");
        case.request_absent(REQUEST_4);
    }
    let refused = case.resolve_json(&operation, REQUEST_4, &input, 3, "external_change");
    case.golden("context_resolve_local_after_partial", &refused);
    let run = case.resolve_text(&operation, REQUEST_4, &input);
    assert_eq!(run.status, 3);
    assert!(
        stderr(&run).contains(&format!("next: request.retry request_id={REQUEST_3}\n")),
        "{}",
        stderr(&run)
    );
    case.text_golden("context_resolve_local_after_partial_text", &run);
    assert_data_excludes_the_source(&case);
    let done = case.resolve_json(&operation, REQUEST_3, &input, 0, "ok");
    assert_eq!(done["data"]["state"], "complete", "{done}");
}

/// The gate's guarded abandonment: refused while the conflict's files remain.
#[test]
fn operation_abandon_of_a_pending_conflict_is_refused_and_keeps_its_files() {
    let (case, operation) = Case::conflicted("");
    let ticket = fs::read_to_string(case.context().join(items::ticket_path(TICKET))).unwrap();
    let prepare = [
        "operation",
        "abandon",
        "--repo",
        case.root(),
        "--operation-id",
        &operation,
        "--prepare",
        "--json",
    ];
    let prepared = cli::json(&case.run(&prepare, &[]));
    let answer = match prepared["data"]["confirmation_id"].as_str() {
        Some(confirmation) => {
            let confirm = [
                "operation",
                "abandon",
                "--repo",
                case.root(),
                "--operation-id",
                &operation,
                "--request-id",
                REQUEST_3,
                "--confirm",
                confirmation,
                "--json",
            ];
            cli::json(&case.run(&confirm, &[]))
        }
        None => prepared,
    };
    assert_eq!(answer["code"], "recovery_required", "{answer}");
    case.golden("context_operation_abandon_refused", &answer);
    assert_eq!(case.state(&operation), "conflict_pending");
    assert_eq!(
        fs::read_to_string(case.context().join(items::ticket_path(TICKET))).unwrap(),
        ticket
    );
}

#[test]
fn resolve_input_failures_are_invalid_input_with_nothing_done() {
    let (case, operation) = Case::conflicted("");
    let tip = case.world.branch_tip(TICKET);
    for (index, bad) in [
        json!({}),
        json!({"observation": "x"}),
        json!({"observation": "x", "resolutions": [{"token": "t"}]}),
        json!({"observation": "x", "resolutions": [{"token": "t", "source": "s", "path": "p"}]}),
        json!({"observation": "x", "resolutions": [], "extra": 1}),
        json!({"observation": 7, "resolutions": []}),
    ]
    .iter()
    .enumerate()
    {
        let value = case.json(
            &case.resolve_args(&operation, REQUEST_3),
            bad.to_string().as_bytes(),
            2,
            "invalid_input",
            None,
        );
        if index == 0 {
            case.golden("context_resolve_local_input_failure", &value);
        }
    }
    case.request_absent(REQUEST_3);
    assert_eq!(case.world.branch_tip(TICKET), tip);
    assert_eq!(case.state(&operation), "conflict_pending");
}

#[test]
fn resolve_local_has_its_help_golden() {
    let temp = tempfile::tempdir().unwrap();
    let env = CliEnv::new(temp.path().join("absent-data"));
    let args = ["context", "resolve-local", "--help", "--json"];
    let output = cli::run_cli(&args.map(OsStr::new), &env);
    assert_eq!(output.status, 0);
    let value = cli::json(&output);
    crate::support::schema::check_published("cli_help.schema.json", &value["data"]).unwrap();
    cli::assert_cli_golden("help_context_resolve_local", &value, &[]);
}

/// The resolve verb's closed input: what its example says, the binary takes.
/// Its token names no path of the live conflict, so the verb's token check
/// refuses it, and not the input's reading.
pub fn the_resolve_example_input_is_accepted(example: &Value) {
    let (case, operation) = Case::conflicted("");
    let args = case.resolve_args(&operation, REQUEST_3);
    case.json(
        &args,
        example.to_string().as_bytes(),
        2,
        "invalid_input",
        None,
    );
    // The verb's check refused it, and nothing else: its line is the only one
    // past C2's.
    let run = case.run(&args, example.to_string().as_bytes());
    assert_eq!(run.status, 2);
    assert_eq!(
        stderr(&run),
        format!(
            "error: The request input is not valid.\ncode: invalid_input\nrequest: {REQUEST_3}\nnext: manyhands-cli context show-local --repo '{}' --operation-id {operation}\n",
            case.root()
        )
    );
}

/// Splits a `next:` line as a POSIX shell would, for the quoting `quoted` uses.
fn words(line: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quoted = false;
    let mut any = false;
    let mut chars = line.chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            '\'' => {
                quoted = !quoted;
                any = true;
            }
            '\\' if !quoted => {
                if let Some(next) = chars.next() {
                    word.push(next);
                }
            }
            ' ' if !quoted => {
                if any || !word.is_empty() {
                    words.push(std::mem::take(&mut word));
                }
                any = false;
            }
            _ => word.push(character),
        }
    }
    if any || !word.is_empty() {
        words.push(word);
    }
    words
}

/// What a human result names after `<label>: `.
fn value_after<'a>(text: &'a str, label: &str) -> &'a str {
    text.lines()
        .find_map(|line| line.strip_prefix(&format!("{label}: ")))
        .unwrap_or_else(|| panic!("no {label}: {text}"))
}

/// The route `human_lifecycle.rs` prints for a refused closure checkpoint merge, run from its
/// own lines with only the placeholders filled, ends with the same close request
/// finishing with one merge and the closure's fields kept.
#[test]
fn the_refused_merges_route_finishes_the_close() {
    let case = Case::over(World::with_service(|data| {
        FailOnce::at(FailurePoint::AfterClosureCheckpoint).open_service(data)
    }));
    case.save();
    let close = || {
        Mutation::TicketClose(TicketCloseInput {
            root: case.world.root.clone(),
            id: items::item_id(TICKET),
        })
    };
    let preview = case.world.service.prepare(&close());
    let confirmation = preview
        .data
        .as_ref()
        .and_then(|data| data.preview())
        .unwrap()
        .confirmation_id
        .clone();
    let stopped = execute_confirmed(&case.world.service, REQUEST_1, &confirmation, close());
    assert_ne!(stopped.code, ResultCode::Ok, "{:?}", stopped.data);
    let path = items::ticket_path(TICKET);
    let checkpoint = fs::read_to_string(case.context().join(&path)).unwrap();
    let closure: Vec<&str> = checkpoint
        .lines()
        .filter(|line| line.starts_with("closed_at:") || line.starts_with("closed_by:"))
        .collect();
    assert_eq!(closure.len(), 2, "{checkpoint}");
    items::write(
        &case.world.root,
        &path,
        &items::ticket_source(TICKET, "Moved on primary", ""),
    );
    items::commit(&case.world.fixture, &[&path], items::COMMITTED_AT + 60);
    let merges = || {
        case.git(&case.world.root, &["rev-list", "--merges", "main"])
            .lines()
            .count()
    };
    assert_eq!(merges(), 0);

    // The refused merge, and its lines.
    let close_args = [
        "ticket",
        "close",
        "--repo",
        case.root(),
        "--id",
        TICKET,
        "--request-id",
        REQUEST_1,
    ];
    let refused = case.run(&close_args, &[]);
    assert_eq!(refused.status, 4, "{}", stderr(&refused));
    let lines: Vec<Vec<String>> = stderr(&refused)
        .lines()
        .filter_map(|line| line.strip_prefix("next: manyhands-cli context "))
        .map(|line| {
            let mut words = words(line);
            words.insert(0, "context".into());
            words
        })
        .collect();
    assert_eq!(lines.len(), 4, "{}", stderr(&refused));
    let fill = |words: &[String], values: &[(&str, &str)]| -> Vec<String> {
        words
            .iter()
            .map(|word| {
                values
                    .iter()
                    .find(|(placeholder, _)| word == placeholder)
                    .map_or_else(|| word.clone(), |(_, value)| (*value).to_owned())
            })
            .collect()
    };
    let run = |words: Vec<String>, input: &[u8]| {
        let args: Vec<&str> = words.iter().map(String::as_str).collect();
        case.run(&args, input)
    };

    // 1. Observe.
    let observed = run(lines[0].clone(), &[]);
    assert_eq!(observed.status, 0, "{}", stderr(&observed));
    let observation = value_after(stdout(&observed), "Observation").to_owned();
    // 2. Synchronize, with the observation as the input: a conflict.
    let synced = run(
        fill(&lines[1], &[("<new-request-id>", REQUEST_2)]),
        &input(&observation),
    );
    assert_eq!(synced.status, 4, "{}", stderr(&synced));
    let operation = value_after(stderr(&synced), "Operation id").to_owned();
    // 3. Inspect it.
    let shown = run(fill(&lines[2], &[("<sync-operation-id>", &operation)]), &[]);
    assert_eq!(shown.status, 0, "{}", stderr(&shown));
    let resolution_observation = value_after(stdout(&shown), "Observation").to_owned();
    let token = stdout(&shown)
        .lines()
        .find_map(|line| {
            line.split_whitespace()
                .find_map(|word| word.strip_prefix("token="))
        })
        .unwrap()
        .to_owned();
    // The sides, through --json: the local side is the closure's checkpoint.
    let sides = case.json(
        &case.show_args(&operation, Some(&token)),
        &[],
        0,
        "ok",
        Some(CONTENT_SCHEMA),
    );
    let local = sides["data"]["local"].as_str().unwrap().to_owned();
    for line in &closure {
        assert!(local.contains(line), "{local}");
    }
    // 4. Resolve, keeping the closure.
    let body = json!({
        "observation": resolution_observation,
        "resolutions": [{ "token": token, "source": local }],
    });
    let resolved = run(
        fill(
            &lines[3],
            &[
                ("<sync-operation-id>", &operation),
                ("<new-request-id>", REQUEST_3),
            ],
        ),
        body.to_string().as_bytes(),
    );
    assert_eq!(resolved.status, 0, "{}", stderr(&resolved));

    // The same close request finishes. Primary gains the resolution's merge,
    // which checkpoints `[context, primary]` in the context, and the closure's
    // one integration.
    let mut json_args = close_args.to_vec();
    json_args.push("--json");
    let done = cli::json(&case.run(&json_args, &[]));
    assert_eq!(done["code"], "ok", "{done}");
    assert_eq!(done["data"]["integration"], "merged", "{done}");
    let subjects = case.git(
        &case.world.root,
        &["log", "--merges", "--format=%s", "main"],
    );
    assert_eq!(
        subjects.lines().collect::<Vec<_>>(),
        [
            format!("Integrate closed ticket {TICKET}").as_str(),
            "Resolve local context conflicts",
        ],
        "{subjects}"
    );
    assert_eq!(merges(), 2);
    let on_primary = case.git(&case.world.root, &["show", &format!("main:{path}")]);
    for line in &closure {
        assert!(on_primary.contains(line), "{on_primary}");
    }
}

// ------------------------------------------- `resolve-local`, its refusals

/// A conflict two paths resolved, by `REQUEST_3`.
fn resolved() -> (Case, String, Value) {
    let (case, operation) = Case::two_paths_conflicted();
    let input = case.resolution(&operation);
    case.resolve_json(&operation, REQUEST_3, &input, 0, "ok");
    (case, operation, input)
}

/// A finished operation lists no paths, so a new request goes to the library,
/// which answers a finished resolution as `external_change`.
#[test]
fn a_new_request_after_complete_is_external_change() {
    let (case, operation, input) = resolved();
    let tip = case.world.branch_tip(TICKET);
    let value = case.resolve_json(&operation, REQUEST_4, &input, 3, "external_change");
    assert_eq!(value["data"]["state"], "complete", "{value}");
    assert_eq!(case.world.branch_tip(TICKET), tip);
}

/// ... and an abandoned one as `cancelled`. The abandonment is artifact-free:
/// a resolution stopped once integrated, before its index hand-off.
#[test]
fn a_new_request_after_abandoned_is_cancelled() {
    let case = Case::over(World::with_service(|data| {
        FailOnce::at(FailurePoint::LocalSyncBeforeIndex).open_service(data)
    }));
    case.save();
    case.primary_moved_the_ticket("");
    let observation = case.observation();
    let value = case.sync_json(REQUEST_2, &observation, 4, "recovery_required");
    let operation = value["operation_id"].as_str().unwrap().to_owned();
    let view = case.show(&operation, 0, "ok");
    let token = view["data"]["paths"][0]["token"]
        .as_str()
        .unwrap()
        .to_owned();
    let source = sentinel_ticket();
    let input = json!({
        "observation": view["data"]["observation"],
        "resolutions": [{ "token": token, "source": source }],
    });
    let stopped = case.world.execute(
        REQUEST_3,
        Mutation::LocalContextResolve(LocalContextResolveInput {
            root: case.world.root.clone(),
            operation_id: OperationId::parse(&operation).unwrap(),
            observation: input["observation"].as_str().unwrap().to_owned(),
            resolutions: vec![LocalConflictResolutionInput { token, source }],
        }),
    );
    assert_eq!(
        stopped.code,
        ResultCode::DiscoveryPending,
        "{:?}",
        stopped.data
    );
    let prepare = [
        "operation",
        "abandon",
        "--repo",
        case.root(),
        "--operation-id",
        &operation,
        "--prepare",
    ];
    let prepared = case.json(&prepare, &[], 0, "ok", None);
    let confirmation = prepared["data"]["confirmation_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let confirm = [
        "operation",
        "abandon",
        "--repo",
        case.root(),
        "--operation-id",
        &operation,
        "--request-id",
        "01ARZ3NDEKTSV4RRFFQ69G5FX7",
        "--confirm",
        &confirmation,
    ];
    case.json(&confirm, &[], 0, "ok", None);
    assert_eq!(case.state(&operation), "abandoned");
    let value = case.resolve_json(&operation, REQUEST_4, &input, 130, "cancelled");
    assert_eq!(value["data"]["state"], "abandoned", "{value}");
}

/// A request the library holds a record of is its own to judge, whatever the
/// token: the finished resolution's request, and the synchronization's.
#[test]
fn a_recorded_request_with_a_tampered_token_is_a_mismatch() {
    let (case, operation, input) = resolved();
    let mut other = input.clone();
    let token = input["resolutions"][0]["token"].as_str().unwrap();
    other["resolutions"][0]["token"] = json!(tampered(token));
    case.resolve_json(&operation, REQUEST_3, &other, 2, "request_mismatch");
    // The synchronization's request is the library's mismatch too; its class
    // is `partial`, from that request's own effects, so it exits 4.
    let value = case.resolve_json(&operation, REQUEST_2, &other, 4, "request_mismatch");
    assert_eq!(value["outcome"], "partial", "{value}");
}

#[test]
fn a_resolution_prepare_is_not_confirmable() {
    let (case, operation) = Case::two_paths_conflicted();
    let input = case.resolution(&operation);
    let args = [
        "context",
        "resolve-local",
        "--repo",
        case.root(),
        "--operation-id",
        &operation,
        "--input",
        "-",
        "--prepare",
    ];
    let value = case.json(
        &args,
        input.to_string().as_bytes(),
        2,
        "not_confirmable",
        None,
    );
    assert!(value["data"].is_null(), "{value}");
    assert_eq!(case.state(&operation), "conflict_pending");
}

/// An external-only conflict has no resolution here: refused before anything is
/// written, with the repair outside, and no line back to `show-local`.
#[test]
fn an_external_only_conflict_is_refused_with_the_repair_outside() {
    let (case, operation) = Case::conflicted("invalid yaml line\n");
    let view = case.show(&operation, 0, "ok");
    assert_eq!(view["data"]["can_start_resolution"], false);
    let ticket = fs::read_to_string(case.context().join(items::ticket_path(TICKET))).unwrap();
    let token = view["data"]["paths"][0]["token"].as_str().unwrap();
    let input = json!({
        "observation": view["data"]["observation"],
        "resolutions": [{ "token": token, "source": sentinel_ticket() }],
    });
    let value = case.resolve_json(&operation, REQUEST_3, &input, 3, "recovery_required");
    assert!(value["data"].is_null(), "{value}");
    case.golden("context_resolve_local_external_only", &value);
    case.request_absent(REQUEST_3);
    let run = case.resolve_text(&operation, REQUEST_3, &input);
    assert_eq!(run.status, 3);
    assert!(stderr(&run).contains("repair outside"), "{}", stderr(&run));
    assert!(!stderr(&run).contains("show-local"), "{}", stderr(&run));
    case.text_golden("context_resolve_local_external_only_text", &run);
    assert_eq!(
        fs::read_to_string(case.context().join(items::ticket_path(TICKET))).unwrap(),
        ticket
    );
    assert_eq!(case.state(&operation), "conflict_pending");
    assert_data_excludes_the_source(&case);
}
