//! `conflict show` and `conflict resolve` through the binary, without a
//! remote. A live conflict needs one, so the show-and-resolve journey, its
//! text golden and a stale observation are Task 12b's.
//!
//! The library's own tables (`tests/mutation_remote/`) pin what it answers.
//! These cases pin what the verbs add: their grammar, their input, the
//! operation lookups that need no remote, and their exit statuses.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::{Value, json};

use super::{
    index::Fixture,
    mutation::{self, golden},
};
use crate::support::{
    self, FailOnce,
    cli::CliEnv,
    items,
    mutation::{REQUEST_1, REQUEST_2, REQUEST_3, World},
    schema,
};

use manyhands::{
    repository::{AuthoringKind, FailurePoint, LocalContextSyncInput, Mutation, OperationId},
    results::ResultCode,
};

/// Well-formed in the library's sense (R-9): the prefix and 64 lower-case
/// hex digits. Neither names a live conflict.
const OBSERVATION: &str =
    "conflict-v1:0000000000000000000000000000000000000000000000000000000000000000";
const TOKEN: &str =
    "conflict-path-v1:1111111111111111111111111111111111111111111111111111111111111111";
const SOURCE: &str = "SOURCE-SENTINEL-77";

fn example() -> Value {
    serde_json::from_slice(include_bytes!(
        "../fixtures/cli_input_v1/conflict_resolve.json"
    ))
    .unwrap()
}

fn input_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("schemas/input/v1")
}

fn input(observation: &str, resolutions: Value) -> Vec<u8> {
    serde_json::to_vec(&json!({"observation": observation, "resolutions": resolutions})).unwrap()
}

fn well_formed() -> Vec<u8> {
    input(OBSERVATION, json!([{"token": TOKEN, "source": SOURCE}]))
}

fn operation() -> String {
    OperationId::new().to_string()
}

impl Fixture {
    fn show(&self, operation: &str, status: i32, code: &str) -> Value {
        let value = self.checked(
            "conflict",
            "show",
            &["--operation-id", operation],
            &[],
            status,
            code,
        );
        assert_eq!(value["command"], "conflict show");
        // A read has no request.
        assert!(value["request_id"].is_null(), "{value}");
        value
    }

    fn resolve(
        &self,
        operation: &str,
        request: &str,
        body: &[u8],
        status: i32,
        code: &str,
    ) -> Value {
        let value = self.checked(
            "conflict",
            "resolve",
            &[
                "--operation-id",
                operation,
                "--request-id",
                request,
                "--input",
                "-",
            ],
            body,
            status,
            code,
        );
        assert_eq!(value["command"], "conflict resolve");
        assert!(!value.to_string().contains(SOURCE), "{value}");
        value
    }

    fn without_record(&self, request: &str) {
        mutation::checked(
            &["request", "show", "--request-id", request, "--json"],
            &self.env,
            &[],
            2,
            "request_not_found",
            None,
        );
    }
}

/// A world over the CLI's data directory, with a git configuration that
/// leaves line endings alone.
fn environment(world: &World) -> CliEnv {
    let env = CliEnv::new(world.data.path().to_owned());
    fs::write(
        env.home.path().join(".gitconfig"),
        "[core]\n\tautocrlf = false\n",
    )
    .unwrap();
    env
}

fn args<'a>(verb: &'a str, root: &'a str, operation: &'a str, extra: &[&'a str]) -> Vec<&'a str> {
    let mut args = vec![
        "conflict",
        verb,
        "--repo",
        root,
        "--operation-id",
        operation,
    ];
    args.extend_from_slice(extra);
    args.push("--json");
    args
}

/// A local-context sync stopped in a conflict, made with the gate's library
/// mutation, because no verb starts one (decision 19). Returns its
/// operation.
fn local_sync(world: &World) -> String {
    let id = items::TICKET_A;
    let saved = world.execute(REQUEST_1, world.save(id, &world.token(id)));
    assert_eq!(saved.code, ResultCode::Ok);
    let path = items::ticket_path(id);
    items::write(
        &world.root,
        &path,
        &items::ticket_source(id, "Incoming conflict", ""),
    );
    items::commit(&world.fixture, &[&path], items::COMMITTED_AT);
    let observed = world
        .service
        .observe_local_context_sync(
            &world.resolved(),
            AuthoringKind::Ticket,
            &items::item_id(id),
        )
        .unwrap();
    let result = world.execute(
        REQUEST_2,
        Mutation::LocalContextSync(LocalContextSyncInput {
            root: world.root.clone(),
            kind: AuthoringKind::Ticket,
            id: items::item_id(id),
            observation: observed.observation,
        }),
    );
    assert_eq!(result.code, ResultCode::RecoveryRequired, "{result:?}");
    result.operation_id.expect("the sync's operation")
}

#[test]
fn the_input_schema_has_two_required_fields_and_is_closed() {
    let file = "conflict_resolve.schema.json";
    let directory = input_directory();
    schema::validate(&directory, file).unwrap();
    let ok = |value: &Value| schema::check(&directory, file, value).is_ok();

    // The example is validated by schema only: a live token cannot be
    // stored (design, Input).
    let example = example();
    assert!(ok(&example));
    let mut fields: Vec<_> = example.as_object().unwrap().keys().cloned().collect();
    fields.sort();
    assert_eq!(fields, ["observation", "resolutions"]);
    assert_eq!(example["resolutions"].as_array().unwrap().len(), 1);

    for field in ["observation", "resolutions"] {
        let mut value = example.clone();
        value.as_object_mut().unwrap().remove(field);
        assert!(!ok(&value), "{field} omitted");
        for wrong in [Value::Null, json!(1), json!(true)] {
            let mut value = example.clone();
            value[field] = wrong;
            assert!(!ok(&value), "{field} wrongly typed");
        }
    }
    // The list is of objects, each with exactly a token and a source.
    for wrong in [
        json!({}),
        json!("a"),
        json!(["a"]),
        json!([[]]),
        json!([null]),
        json!([{"token": TOKEN}]),
        json!([{"source": SOURCE}]),
        json!([{"token": 1, "source": SOURCE}]),
        json!([{"token": TOKEN, "source": null}]),
        json!([{"token": TOKEN, "source": SOURCE, "path": "a"}]),
    ] {
        let mut value = example.clone();
        value["resolutions"] = wrong.clone();
        assert!(!ok(&value), "{wrong}");
    }
    // A source may be empty, and a token is whatever string the CLI passes
    // on: the library judges its form.
    let mut empty = example.clone();
    empty["resolutions"] = json!([{"token": "", "source": ""}]);
    assert!(ok(&empty));
    let mut unknown = example;
    unknown["request_id"] = json!("not-an-input-field");
    assert!(!ok(&unknown));
}

#[test]
fn conflict_show_operation_not_found() {
    let f = Fixture::new();
    let before = support::repository_and_worktree_snapshot(&f.repo);
    let value = f.show(&operation(), 2, "operation_not_found");
    assert_eq!(value["outcome"], "error");
    assert!(value["data"].is_null());
    assert_eq!(
        value["scope"]["repository"],
        f.repo.root.canonicalize().unwrap().to_str().unwrap()
    );
    golden("conflict_show_operation_not_found", &value, &f.repo.root);

    // In text, the refusal is the usual one and nothing is printed to stdout.
    let human = mutation::run(
        &f.args("conflict", "show", &["--operation-id", &operation()]),
        &f.env,
        &[],
    );
    assert_eq!(human.status, 2);
    assert!(human.stdout.is_empty());
    let text = String::from_utf8(human.stderr).unwrap();
    assert!(text.starts_with("error: "), "{text}");
    assert!(text.contains("code: operation_not_found\n"), "{text}");
    // A read writes nothing, and the lease is not left held (R-12).
    assert_eq!(support::repository_and_worktree_snapshot(&f.repo), before);
    f.show(&operation(), 2, "operation_not_found");
}

#[test]
fn a_malformed_operation_id_is_invalid_id_for_both_verbs() {
    let f = Fixture::new();
    let value = f.checked(
        "conflict",
        "show",
        &["--operation-id", "not-a-ulid"],
        &[],
        2,
        "invalid_id",
    );
    assert!(value["request_id"].is_null());
    // The resolve names its request, as every mutation's flag refusal does.
    let value = f.resolve("not-a-ulid", REQUEST_1, &well_formed(), 2, "invalid_id");
    assert_eq!(value["request_id"], REQUEST_1);
    f.without_record(REQUEST_1);
}

#[test]
fn conflict_show_refuses_prepare_and_a_request_id_as_usage_errors() {
    let f = Fixture::new();
    let before = support::repository_and_worktree_snapshot(&f.repo);
    let operation = operation();
    for extra in [
        vec!["--prepare"],
        vec!["--request-id", REQUEST_1],
        vec!["--confirm", REQUEST_1],
        vec!["--input", "-"],
    ] {
        let mut flags = vec!["--operation-id", operation.as_str()];
        flags.extend(extra.iter().copied());
        let value = f.checked("conflict", "show", &flags, b"{}", 2, "usage_error");
        assert!(value["request_id"].is_null(), "{extra:?}");
    }
    assert_eq!(support::repository_and_worktree_snapshot(&f.repo), before);
}

#[test]
fn conflict_resolve_is_not_confirmable() {
    let f = Fixture::new();
    let before = support::repository_and_worktree_snapshot(&f.repo);
    let value = f.checked(
        "conflict",
        "resolve",
        &["--operation-id", &operation(), "--input", "-", "--prepare"],
        &well_formed(),
        2,
        "not_confirmable",
    );
    assert!(value["request_id"].is_null());
    assert_eq!(support::repository_and_worktree_snapshot(&f.repo), before);
}

#[test]
fn conflict_resolve_requires_its_input_and_its_operation() {
    let f = Fixture::new();
    // No `--input`, and no `--operation-id`: the grammar refuses both.
    let value = f.checked(
        "conflict",
        "resolve",
        &["--operation-id", &operation(), "--request-id", REQUEST_1],
        &[],
        2,
        "usage_error",
    );
    assert!(value["request_id"].is_null());
    let value = f.checked(
        "conflict",
        "resolve",
        &["--input", "-", "--request-id", REQUEST_1],
        &well_formed(),
        2,
        "usage_error",
    );
    assert!(value["request_id"].is_null());
}

#[test]
fn conflict_resolve_unknown_operation_is_operation_not_found_after_the_syntax_passes() {
    let f = Fixture::new();
    let before = support::repository_and_worktree_snapshot(&f.repo);
    let value = f.resolve(
        &operation(),
        REQUEST_1,
        &well_formed(),
        2,
        "operation_not_found",
    );
    assert_eq!(value["request_id"], REQUEST_1);
    assert_eq!(support::repository_and_worktree_snapshot(&f.repo), before);
}

#[test]
fn conflict_resolve_with_a_malformed_token_is_invalid_input() {
    let f = Fixture::new();
    let before = support::repository_and_worktree_snapshot(&f.repo);
    let operation = operation();
    let mut named = None;
    for (case, body) in [
        (
            "token without a prefix",
            input(OBSERVATION, json!([{"token": "a-token", "source": SOURCE}])),
        ),
        (
            "token with the observation's prefix",
            input(
                OBSERVATION,
                json!([{"token": OBSERVATION, "source": SOURCE}]),
            ),
        ),
        (
            "token with upper-case digits",
            input(
                OBSERVATION,
                json!([{"token": format!("conflict-path-v1:{}", "ABCDEF0123456789".repeat(4)), "source": SOURCE}]),
            ),
        ),
        (
            "empty token",
            input(OBSERVATION, json!([{"token": "", "source": SOURCE}])),
        ),
        (
            "malformed observation",
            input(
                "conflict-v1:abc",
                json!([{"token": TOKEN, "source": SOURCE}]),
            ),
        ),
        (
            "the same token twice",
            input(
                OBSERVATION,
                json!([
                    {"token": TOKEN, "source": SOURCE},
                    {"token": TOKEN, "source": "another"}
                ]),
            ),
        ),
    ] {
        let value = f.resolve(&operation, REQUEST_1, &body, 2, "invalid_input");
        assert_eq!(value["request_id"], REQUEST_1, "{case}");
        assert!(value["data"].is_null(), "{case}");
        assert_eq!(value["effects"]["write"], "not_requested", "{case}");
        assert_eq!(
            support::repository_and_worktree_snapshot(&f.repo),
            before,
            "{case}"
        );
        named.get_or_insert(value);
    }
    golden(
        "conflict_resolve_invalid_input",
        &named.unwrap(),
        &f.repo.root,
    );
}

#[test]
fn input_the_cli_refuses_is_invalid_input_with_the_request_named_and_no_record() {
    let f = Fixture::new();
    let before = support::repository_and_worktree_snapshot(&f.repo);
    let operation = operation();
    let resolutions = json!([{"token": TOKEN, "source": SOURCE}]);
    let mut cases: Vec<(&str, Vec<u8>)> = vec![
        ("not JSON", b"{".to_vec()),
        ("not an object", b"[]".to_vec()),
        (
            "missing observation",
            serde_json::to_vec(&json!({"resolutions": resolutions})).unwrap(),
        ),
        (
            "missing resolutions",
            serde_json::to_vec(&json!({"observation": OBSERVATION})).unwrap(),
        ),
        (
            "observation type",
            serde_json::to_vec(&json!({"observation": 1, "resolutions": resolutions})).unwrap(),
        ),
        ("empty list", input(OBSERVATION, json!([]))),
        ("not a list", input(OBSERVATION, json!({"token": TOKEN}))),
        ("a string element", input(OBSERVATION, json!([SOURCE]))),
        (
            "a null element",
            input(
                OBSERVATION,
                json!([{"token": TOKEN, "source": SOURCE}, null]),
            ),
        ),
        (
            "missing source",
            input(OBSERVATION, json!([{"token": TOKEN}])),
        ),
        (
            "missing token",
            input(OBSERVATION, json!([{"source": SOURCE}])),
        ),
        (
            "token type",
            input(OBSERVATION, json!([{"token": 1, "source": SOURCE}])),
        ),
        (
            "source type",
            input(OBSERVATION, json!([{"token": TOKEN, "source": null}])),
        ),
        (
            "unknown field in a resolution",
            input(
                OBSERVATION,
                json!([{"token": TOKEN, "source": SOURCE, "path": "a"}]),
            ),
        ),
        (
            "unknown field",
            serde_json::to_vec(&json!({
                "observation": OBSERVATION,
                "resolutions": resolutions,
                "request_id": REQUEST_2,
            }))
            .unwrap(),
        ),
    ];
    // A source that is itself JSON is a string like any other.
    cases.push((
        "duplicate key",
        format!(
            r#"{{"observation":"{OBSERVATION}","observation":"{OBSERVATION}","resolutions":[]}}"#
        )
        .into_bytes(),
    ));
    for (case, body) in cases {
        let value = f.resolve(&operation, REQUEST_3, &body, 2, "invalid_input");
        assert_eq!(value["request_id"], REQUEST_3, "{case}");
        assert!(value["data"].is_null(), "{case}");
        // The CLI refused it before it built a request.
        f.without_record(REQUEST_3);
        assert_eq!(
            support::repository_and_worktree_snapshot(&f.repo),
            before,
            "{case}"
        );
    }
}

#[test]
fn conflict_show_of_a_local_operation_that_is_no_synchronization_is_invalid_input() {
    // A save that committed and owes its index hand-off leaves a pending
    // local operation, which is not a synchronization.
    let mut world = World::with_service(|data| {
        FailOnce::at(FailurePoint::BeforeIndexTransactionCommit).open_service(data)
    });
    let token = world.token(items::TICKET_A);
    let owed = world.execute(REQUEST_1, world.save(items::TICKET_A, &token));
    assert_eq!(owed.code, ResultCode::DiscoveryPending);
    let operation = owed.operation_id.clone().unwrap();
    world.reopen();
    let env = environment(&world);
    let root = world.root.to_str().unwrap();
    let before = support::repository_and_worktree_snapshot(&world.fixture);

    let value = mutation::checked(
        &args("show", root, &operation, &[]),
        &env,
        &[],
        2,
        "invalid_input",
        None,
    );
    assert!(value["data"].is_null());
    golden("conflict_show_invalid_input", &value, &world.root);
    let value = mutation::checked(
        &args(
            "resolve",
            root,
            &operation,
            &["--request-id", REQUEST_2, "--input", "-"],
        ),
        &env,
        &well_formed(),
        2,
        "invalid_input",
        None,
    );
    assert_eq!(value["request_id"], REQUEST_2);
    // The operation is still pending, and nothing was written.
    assert_eq!(
        support::repository_and_worktree_snapshot(&world.fixture),
        before
    );
    assert_eq!(world.pending_journal_rows(), 1);
}

#[test]
fn conflict_show_and_resolve_of_a_local_context_syncs_operation_are_invalid_input() {
    let world = World::new();
    let operation = local_sync(&world);
    let sync_state = world.record(REQUEST_2).unwrap().state;
    let env = environment(&world);
    let root = world.root.to_str().unwrap();
    let before = support::repository_and_worktree_snapshot(&world.fixture);

    // Decision 19: it is never `operation_not_found`, and no source or
    // token of the local conflict is read.
    let shown = mutation::checked(
        &args("show", root, &operation, &[]),
        &env,
        &[],
        2,
        "invalid_input",
        None,
    );
    assert!(shown["data"].is_null());
    assert!(shown["request_id"].is_null());
    let human = mutation::run(
        &[
            "conflict",
            "show",
            "--repo",
            root,
            "--operation-id",
            &operation,
        ],
        &env,
        &[],
    );
    assert_eq!(human.status, 2);
    assert!(human.stdout.is_empty());
    assert!(
        String::from_utf8(human.stderr)
            .unwrap()
            .contains("code: invalid_input\n")
    );

    let resolved = mutation::checked(
        &args(
            "resolve",
            root,
            &operation,
            &["--request-id", REQUEST_3, "--input", "-"],
        ),
        &env,
        &well_formed(),
        2,
        "invalid_input",
        None,
    );
    assert_eq!(resolved["request_id"], REQUEST_3);
    assert!(resolved["data"].is_null());
    assert_eq!(resolved["effects"]["write"], "not_requested");
    assert!(!resolved.to_string().contains(SOURCE));
    // The refusal left no record of the request, the sync as it was, and
    // the repository untouched.
    assert!(world.record(REQUEST_3).is_none());
    assert_eq!(
        support::repository_and_worktree_snapshot(&world.fixture),
        before
    );
    assert_eq!(world.record(REQUEST_2).unwrap().state, sync_state);
}

#[test]
fn every_example_input_of_the_verb_is_an_object_the_cli_reads() {
    // The example is not run: its token is not a live one. It is read as
    // far as the library's syntax check, which it passes, and then meets an
    // operation no repository has.
    let f = Fixture::new();
    let body = serde_json::to_vec(&example()).unwrap();
    let value = f.resolve(&operation(), REQUEST_1, &body, 2, "operation_not_found");
    assert_eq!(value["request_id"], REQUEST_1);
    assert!(!value.to_string().contains("The resolved body."));
}

#[test]
fn conflict_resolve_refuses_an_option_it_does_not_declare() {
    let f = Fixture::new();
    let value = f.checked(
        "conflict",
        "resolve",
        &[
            "--operation-id",
            &operation(),
            "--request-id",
            REQUEST_1,
            "--input",
            "-",
            "--bogus",
        ],
        &well_formed(),
        2,
        "usage_error",
    );
    assert!(value["request_id"].is_null());
}
