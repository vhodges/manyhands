use super::{
    mutation::{Fixture, IDENTITY, REMOTE, REQUEST, checked, run},
    repository::{golden, require_no_system_identity},
};
use crate::support::{cli, schema};
use serde_json::Value;
use std::fs;

pub const SELECT: &[u8] = include_bytes!("../fixtures/cli_input_v1/remote_select.json");
pub const REMOVE: &[u8] = include_bytes!("../fixtures/cli_input_v1/remote_remove.json");
pub const NEXT: &str = "01M4CC0VMTTNKVYS6XWQG4CK1P";
pub const LAST: &str = "01M4CC0VMTTNKVYS6XWQG4CK1Q";

pub fn call(f: &Fixture, verb: &str, input: &[u8], extra: &[&str], exit: i32, code: &str) -> Value {
    fs::write(&f.input, input).unwrap();
    let mut flags = vec!["--json"];
    flags.extend_from_slice(extra);
    checked(
        &f.args("remote", verb, &flags),
        &f.env,
        &[],
        exit,
        code,
        None,
    )
}
pub fn add(f: &Fixture) {
    call(f, "add", REMOTE, &["--request-id", REQUEST], 0, "ok");
}
pub fn select(f: &Fixture) -> Value {
    call(f, "select", SELECT, &["--request-id", NEXT], 0, "ok")
}

#[test]
fn remote_noops_preserve_repository_in_both_modes() {
    for (verb, clear) in [
        ("add", false),
        ("remove", false),
        ("select", false),
        ("select", true),
    ] {
        for json in [false, true] {
            let f = Fixture::new();
            if verb == "add" || (verb == "select" && !clear) {
                add(&f);
            }
            if verb == "select" && !clear {
                select(&f);
            }
            let input = if clear {
                br#"{"name":null}"#.as_slice()
            } else if verb == "add" {
                REMOTE
            } else {
                SELECT
            };
            fs::write(&f.input, input).unwrap();
            let flags = if verb == "remove" {
                vec!["--prepare"]
            } else {
                vec!["--request-id", LAST]
            };
            // LAST is a new intent, not replay of the successful add/select.
            let before = crate::support::repository_and_worktree_snapshot_at(&f.repo.root);
            let global = fs::read(f.env.home.path().join(".gitconfig")).unwrap();
            super::human::assert_noop_in_mode(&f.args("remote", verb, &flags), &f.env, &[], json);
            assert_eq!(
                crate::support::repository_and_worktree_snapshot_at(&f.repo.root),
                before
            );
            assert_eq!(
                fs::read(f.env.home.path().join(".gitconfig")).unwrap(),
                global
            );
        }
    }
}

#[test]
fn select_then_clear() {
    let f = Fixture::new();
    add(&f);
    let selected = select(&f);
    schema::check_published("remote_mutation.schema.json", &selected["data"]).unwrap();
    golden("remote_select", &selected);
    let cleared = call(
        &f,
        "select",
        br#"{"name":null}"#,
        &["--request-id", LAST],
        0,
        "ok",
    );
    schema::check_published("remote_mutation.schema.json", &cleared["data"]).unwrap();
    golden("remote_select_clear", &cleared);
    let config: toml::Value =
        toml::from_str(&fs::read_to_string(f.repo.root.join(".manyhands/config.toml")).unwrap())
            .unwrap();
    assert!(config.get("publication_remote").is_none());
}

#[test]
fn remove_a_non_publication_remote() {
    let f = Fixture::new();
    add(&f);
    let preview = call(&f, "remove", REMOVE, &["--prepare"], 0, "ok");
    schema::check_published("remote_removal_preview.schema.json", &preview["data"]).unwrap();
    golden("remote_remove_prepare", &preview);
    assert!(f.repo.repository.find_remote("origin").is_ok());
    let removed = call(
        &f,
        "remove",
        REMOVE,
        &[
            "--request-id",
            NEXT,
            "--confirm",
            preview["data"]["confirmation_id"].as_str().unwrap(),
        ],
        0,
        "ok",
    );
    schema::check_published("remote_mutation.schema.json", &removed["data"]).unwrap();
    golden("remote_remove", &removed);
    assert!(f.repo.repository.find_remote("origin").is_err());
}

#[test]
fn removing_the_publication_remote_is_refused_at_prepare() {
    let f = Fixture::new();
    add(&f);
    select(&f);
    let before = f.config();
    call(
        &f,
        "remove",
        REMOVE,
        &["--prepare"],
        3,
        "publication_remote_in_use",
    );
    assert_eq!(f.config(), before);
}

#[test]
fn a_taken_name_and_an_invalid_url_are_refused() {
    let f = Fixture::new();
    add(&f);
    call(
        &f,
        "add",
        br#"{"name":"origin","url":"ssh://git@other.invalid/repo.git"}"#,
        &["--request-id", NEXT],
        2,
        "remote_name_conflict",
    );
    call(
        &f,
        "add",
        br#"{"name":"other","url":""}"#,
        &["--request-id", LAST],
        2,
        "invalid_remote",
    );
}

#[test]
fn a_non_ssh_remote_cannot_be_selected() {
    let f = Fixture::new();
    call(
        &f,
        "add",
        br#"{"name":"origin","url":"https://example.invalid/repo.git"}"#,
        &["--request-id", REQUEST],
        0,
        "ok",
    );
    call(
        &f,
        "select",
        SELECT,
        &["--request-id", NEXT],
        2,
        "invalid_remote",
    );
}

pub fn remove_identity(f: &Fixture) {
    require_no_system_identity();
    fs::write(
        f.env.home.path().join(".gitconfig"),
        "[core]\n autocrlf = false\n",
    )
    .unwrap();
    let mut config = git2::Config::open(&f.repo.repository.path().join("config")).unwrap();
    config.remove("user.name").unwrap();
    config.remove("user.email").unwrap();
}

#[test]
fn select_without_identity_recovers_through_identity_set() {
    let f = Fixture::new();
    add(&f);
    remove_identity(&f);
    let global = fs::read(f.env.home.path().join(".gitconfig")).unwrap();
    let failure = call(
        &f,
        "select",
        SELECT,
        &["--request-id", NEXT],
        3,
        "identity_required",
    );
    assert!(
        failure["recovery"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["action"] == "repo.identity_set")
    );
    fs::write(&f.input, SELECT).unwrap();
    let human = run(
        &f.args("remote", "select", &["--request-id", NEXT]),
        &f.env,
        &[],
    );
    assert_eq!(human.status, 3);
    assert!(human.stdout.is_empty());
    let text = String::from_utf8(human.stderr).unwrap();
    let operation = text
        .lines()
        .find_map(|line| line.strip_prefix("operation: "))
        .unwrap();
    let root = fs::canonicalize(&f.repo.root).unwrap();
    cli::assert_cli_text_golden(
        "remote_select_identity_required_text",
        &text,
        &[
            (root.to_str().unwrap(), "<repository>"),
            (NEXT, "<request-id>"),
            (operation, "<operation-id>"),
        ],
    );
    fs::write(&f.input, IDENTITY).unwrap();
    let preview = f.prepare();
    checked(
        &f.args(
            "repo",
            "identity-set",
            &[
                "--json",
                "--request-id",
                LAST,
                "--confirm",
                preview["data"]["confirmation_id"].as_str().unwrap(),
            ],
        ),
        &f.env,
        &[],
        0,
        "ok",
        Some("identity_mutation.schema.json"),
    );
    select(&f);
    assert_eq!(
        fs::read(f.env.home.path().join(".gitconfig")).unwrap(),
        global
    );
}

#[test]
fn credentials_in_remote_urls_never_reach_either_output_mode() {
    for json in [false, true] {
        let f = Fixture::new();
        // Synthetic URL redaction marker, never an authentication credential.
        let input = br#"{"name":"origin","url":"https://url-user-sentinel:url-password-sentinel@example.invalid/repo.git"}"#;
        for (verb, bytes, id) in [
            ("add", input.as_slice(), REQUEST),
            ("select", SELECT, NEXT),
            ("remove", REMOVE, LAST),
        ] {
            fs::write(&f.input, bytes).unwrap();
            let mut flags = if verb == "remove" {
                vec!["--prepare"]
            } else {
                vec!["--request-id", id]
            };
            if json {
                flags.push("--json");
            }
            let out = run(&f.args("remote", verb, &flags), &f.env, &[]);
            assert_eq!(out.status, if verb == "select" { 2 } else { 0 });
            for stream in [&out.stdout, &out.stderr] {
                let text = String::from_utf8_lossy(stream);
                assert!(!text.contains("url-user-sentinel"));
                assert!(!text.contains("url-password-sentinel"));
            }
            if json {
                cli::json(&out);
            }
            if verb == "remove" {
                let preview = call(&f, "remove", REMOVE, &["--prepare"], 0, "ok");
                let mut flags = vec![
                    "--request-id",
                    id,
                    "--confirm",
                    preview["data"]["confirmation_id"].as_str().unwrap(),
                ];
                if json {
                    flags.push("--json");
                }
                let out = run(&f.args("remote", "remove", &flags), &f.env, &[]);
                assert_eq!(out.status, 0);
                for stream in [&out.stdout, &out.stderr] {
                    let text = String::from_utf8_lossy(stream);
                    assert!(!text.contains("url-user-sentinel"));
                    assert!(!text.contains("url-password-sentinel"));
                }
                if json {
                    cli::json(&out);
                }
                assert!(f.repo.repository.find_remote("origin").is_err());
            }
        }
    }
}

#[test]
fn a_stopped_remote_add_is_completed_by_the_same_request() {
    let f = Fixture::new();
    super::requests::accepted(&f);
    let before = f.config();
    let result = call(&f, "add", REMOTE, &["--request-id", REQUEST], 0, "ok");
    assert_eq!(result["outcome"], "success");
    let request = checked(
        &["request", "show", "--request-id", REQUEST, "--json"],
        &f.env,
        &[],
        0,
        "ok",
        Some("request.schema.json"),
    );
    assert_eq!(request["data"]["state"], "finished");
    assert_eq!(f.config(), before);
}

#[test]
fn another_url_on_a_stopped_request_is_partial_request_mismatch() {
    let f = Fixture::new();
    super::requests::accepted(&f);
    let before = f.config();
    let result = call(
        &f,
        "add",
        br#"{"name":"origin","url":"ssh://git@other.invalid/repo.git"}"#,
        &["--request-id", REQUEST],
        4,
        "request_mismatch",
    );
    assert_eq!(result["outcome"], "partial");
    assert_eq!(f.config(), before);
}

#[test]
fn selecting_the_selected_remote_is_a_no_op() {
    let f = Fixture::new();
    add(&f);
    select(&f);
    let before = crate::support::repository_and_worktree_snapshot_at(&f.repo.root);
    call(
        &f,
        "select",
        SELECT,
        &["--request-id", LAST],
        0,
        "already_applied",
    );
    assert_eq!(
        crate::support::repository_and_worktree_snapshot_at(&f.repo.root),
        before
    );
}

#[test]
fn remote_shapes_and_grammar_are_checked_before_mutation() {
    let f = Fixture::new();
    for verb in ["remove", "select"] {
        for input in [
            br#"{}"#.as_slice(),
            br#"{"name":42}"#,
            br#"{"name":"sentinel","url":"sentinel"}"#,
            br#"{"name":"sentinel","name":"sentinel"}"#,
        ] {
            call(&f, verb, input, &["--prepare"], 2, "invalid_input");
        }
        checked(
            &["remote", verb, "--json", "--prepare"],
            &f.env,
            &[],
            2,
            "usage_error",
            None,
        );
        checked(
            &[
                "remote",
                verb,
                "--repo",
                f.repo.root.to_str().unwrap(),
                "--json",
                "--prepare",
            ],
            &f.env,
            &[],
            2,
            "usage_error",
            None,
        );
    }
    call(
        &f,
        "remove",
        br#"{"name":null}"#,
        &["--prepare"],
        2,
        "invalid_input",
    );
    call(
        &f,
        "select",
        br#"{"name":null}"#,
        &["--prepare"],
        2,
        "not_confirmable",
    );
}
