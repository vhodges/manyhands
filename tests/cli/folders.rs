use std::fs;

use serde_json::{Value, json};

use super::{
    index::{Fixture, REQUEST_2, assert_stored_result},
    mutation::{self, REQUEST, golden},
};
use crate::support::{self, cli};

fn create(f: &Fixture, path: &str, request: &str, status: i32, code: &str) -> Value {
    f.checked(
        "folder",
        "create",
        &["--request-id", request, "--input", "-"],
        &serde_json::to_vec(&json!({"path":path})).unwrap(),
        status,
        code,
    )
}

#[test]
fn folder_create_makes_an_empty_folder_and_lists_it() {
    let f = Fixture::new();
    let before = support::repository_and_worktree_snapshot(&f.repo);
    let refs = f.refs("refs/");
    let value = f.checked(
        "folder",
        "create",
        &["--request-id", REQUEST, "--input", "-"],
        include_bytes!("../fixtures/cli_input_v1/folder_create.json"),
        0,
        "ok",
    );
    assert!(value["data"].is_null());
    assert_eq!(value["effects"]["discovery"], "current");
    assert_eq!(value["effects"]["checkpoint"], "not_requested");
    assert_eq!(
        fs::read_dir(f.repo.root.join("docs/new-folder"))
            .unwrap()
            .count(),
        0
    );
    let listed = f.checked("folder", "list", &[], &[], 0, "ok");
    assert!(
        listed["data"]["items"]
            .as_array()
            .unwrap()
            .contains(&json!({"path":"docs/new-folder", "empty":true}))
    );
    assert_eq!(support::repository_and_worktree_snapshot(&f.repo), before);
    assert_eq!(f.refs("refs/"), refs);
    golden("folder_create_ok", &value, &f.repo.root);
}

#[test]
fn folder_create_of_an_existing_folder_is_a_noop() {
    let f = Fixture::new();
    assert!(f.repo.root.join("docs/notes").is_dir());
    let before = support::repository_and_worktree_snapshot(&f.repo);
    let value = create(&f, "docs/notes", REQUEST_2, 0, "already_applied");
    assert_eq!(value["outcome"], "noop");
    assert_eq!(support::repository_and_worktree_snapshot(&f.repo), before);
}

#[test]
fn folder_create_refuses_an_occupied_path_and_a_path_outside_docs() {
    let f = Fixture::new();
    let before = support::repository_and_worktree_snapshot(&f.repo);
    for (path, code) in [
        ("docs/guide.md", "occupied_path"),
        ("outside-docs", "invalid_path"),
    ] {
        let value = create(&f, path, REQUEST, 2, code);
        assert_eq!(value["request_id"], REQUEST);
        assert_eq!(support::repository_and_worktree_snapshot(&f.repo), before);
        if code == "occupied_path" {
            golden("folder_create_occupied_path", &value, &f.repo.root);
        }
    }
    assert!(!f.repo.root.join("outside-docs").exists());
}

#[test]
fn folder_create_replays_a_finished_request() {
    let f = Fixture::new();
    let first = create(&f, "docs/new-folder", REQUEST, 0, "ok");
    let record = f.request(REQUEST);
    assert_stored_result(&record, &first);
    // External removal makes a repeated create observable: a finished replay
    // must return the record, not recreate the directory.
    fs::remove_dir(f.repo.root.join("docs/new-folder")).unwrap();
    let status = f.checked("index", "status", &[], &[], 0, "ok");
    let before = support::repository_and_worktree_snapshot(&f.repo);
    assert_eq!(create(&f, "docs/new-folder", REQUEST, 0, "ok"), first);
    assert_eq!(f.request(REQUEST), record);
    assert_eq!(f.checked("index", "status", &[], &[], 0, "ok"), status);
    assert_eq!(support::repository_and_worktree_snapshot(&f.repo), before);
    assert!(!f.repo.root.join("docs/new-folder").exists());
}

#[test]
fn folder_create_has_human_output() {
    let f = Fixture::new();
    let output = mutation::run(
        &f.args(
            "folder",
            "create",
            &["--request-id", REQUEST, "--input", "-"],
        ),
        &f.env,
        br#"{"path":"docs/new-folder"}"#,
    );
    assert_eq!(
        output.status,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let replay = create(&f, "docs/new-folder", REQUEST, 0, "ok");
    cli::assert_cli_text_golden(
        "folder_create_text",
        std::str::from_utf8(&output.stdout).unwrap(),
        &[
            (REQUEST, "<request-id>"),
            (replay["operation_id"].as_str().unwrap(), "<operation-id>"),
            (
                f.repo.root.canonicalize().unwrap().to_str().unwrap(),
                "<repository>",
            ),
        ],
    );
}

#[test]
fn folder_create_refuses_bad_input_without_echoing_values() {
    let f = Fixture::new();
    let before = support::repository_and_worktree_snapshot(&f.repo);
    for input in [
        br#"{}"#.as_slice(),
        br#"{"path":null}"#,
        br#"{"path":7}"#,
        br#"{"path":"docs/SECRET-VALUE","extra":"SECRET-VALUE"}"#,
        br#"{"path":"docs/SECRET-VALUE","path":"docs/other"}"#,
        b"SECRET-VALUE",
    ] {
        let output = mutation::run(
            &f.args(
                "folder",
                "create",
                &["--request-id", REQUEST, "--input", "-", "--json"],
            ),
            &f.env,
            input,
        );
        let value = cli::json(&output);
        assert_eq!(value["code"], "invalid_input");
        assert_eq!(output.status, 2);
        assert_eq!(value["request_id"], REQUEST);
        assert!(
            !String::from_utf8(output.stdout)
                .unwrap()
                .contains("SECRET-VALUE")
        );
        assert_eq!(support::repository_and_worktree_snapshot(&f.repo), before);
    }
}
