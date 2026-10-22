use super::mutation::{Fixture, REMOTE, REQUEST, checked, run};
use crate::support::cli;

#[test]
fn c2_repository_human_results_split_streams_and_retain_public_ids() {
    for (verb, noop) in [
        ("create", false),
        ("enable", false),
        ("remove", false),
        ("enable", true),
    ] {
        let f = super::repository::Fixture::new(if verb == "create" { "create" } else { "enable" });
        if verb == "remove" || noop {
            f.onboard("enable");
        }
        let preview = if noop { None } else { Some(f.prepare(verb)) };
        let mut extra = if let Some(preview) = &preview {
            vec![
                "--request-id",
                "01M4CC0VMTTNKVYS6XWQG4CK1P",
                "--confirm",
                preview["data"]["confirmation_id"].as_str().unwrap(),
            ]
        } else {
            vec!["--prepare"]
        };
        let out = run(&f.args(verb, &extra), &f.env, &[]);
        assert_eq!(out.status, 0, "{}", String::from_utf8_lossy(&out.stderr));
        let stdout = String::from_utf8(out.stdout).unwrap();
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert!(
            stderr
                .lines()
                .all(|line| line.starts_with("progress: ") || line.starts_with("note: "))
        );
        assert!(!stdout.contains("sentinel") && !stderr.contains("sentinel"));
        if noop {
            assert!(stdout.starts_with("Already applied; nothing changed."));
        }
        extra.push("--json");
        let replay = run(&f.args(verb, &extra), &f.env, &[]);
        let json = cli::json(&replay);
        assert_eq!(replay.status, 0);
        assert_eq!(json["code"], if noop { "already_applied" } else { "ok" });
        for pointer in ["/request_id", "/operation_id", "/effects/commit_oid"] {
            if let Some(id) = json.pointer(pointer).and_then(serde_json::Value::as_str) {
                assert!(stdout.contains(id));
            }
        }
    }
}

#[test]
fn repo_create_human_prepare_preview() {
    let f = super::repository::Fixture::new("create");
    let output = run(&f.args("create", &["--prepare"]), &f.env, &[]);
    assert_eq!(output.status, 0);
    assert!(output.stderr.is_empty());
    assert!(!f.root.exists());
    let text = String::from_utf8(output.stdout).unwrap();
    let line = text
        .lines()
        .find_map(|line| line.strip_prefix("Confirmation: "))
        .unwrap();
    let (confirmation, expires) = line.split_once(" (expires ").unwrap();
    let expires = expires.strip_suffix(')').unwrap();
    // The library resolves the existing parent of an absent create target.
    let root = std::fs::canonicalize(f.root.parent().unwrap())
        .unwrap()
        .join(f.root.file_name().unwrap());
    cli::assert_cli_text_golden(
        "repo_create_prepare_text",
        &text,
        &[
            (root.to_str().unwrap(), "<repository>"),
            (confirmation, "<confirmation-id>"),
            (expires, "<expires-at>"),
        ],
    );
}

fn request_fixture(finished: bool) -> Fixture {
    let f = Fixture::new();
    if finished {
        std::fs::write(&f.input, REMOTE).unwrap();
        checked(
            &f.args("remote", "add", &["--json", "--request-id", REQUEST]),
            &f.env,
            &[],
            0,
            "ok",
            Some("remote_mutation.schema.json"),
        );
    } else {
        super::requests::accepted(&f);
    }
    f
}

fn assert_line(text: &str, label: &str, value: &str) {
    let expected = format!("{label}: {value}");
    assert!(
        text.lines().any(|line| line == expected),
        "missing {expected:?} in {text}"
    );
}

#[test]
fn request_show_human_reports_finished_and_accepted_record_details() {
    for finished in [false, true] {
        let f = request_fixture(finished);
        let before = f.config();
        let json = checked(
            &["request", "show", "--request-id", REQUEST, "--json"],
            &f.env,
            &[],
            0,
            "ok",
            Some("request.schema.json"),
        );
        let dto = &json["data"];
        assert_eq!(dto["state"], if finished { "finished" } else { "accepted" });
        let output = run(&["request", "show", "--request-id", REQUEST], &f.env, &[]);
        assert_eq!(output.status, 0);
        assert!(output.stderr.is_empty());
        let text = String::from_utf8(output.stdout).unwrap();
        assert_line(&text, "Request id", REQUEST);
        assert_line(&text, "State", dto["state"].as_str().unwrap());
        assert_line(&text, "Command", "remote add");
        assert_line(&text, "Accepted at", dto["accepted_at"].as_str().unwrap());
        assert_line(
            &text,
            "Finished at",
            dto["finished_at"].as_str().unwrap_or("none"),
        );
        let operations = dto["operations"].as_array().unwrap();
        assert!(!operations.is_empty());
        for operation in operations {
            assert_line(
                &text,
                "Operations family",
                operation["family"].as_str().unwrap(),
            );
            assert_line(
                &text,
                "Operations operation id",
                operation["operation_id"].as_str().unwrap(),
            );
        }
        if finished {
            assert!(!dto["finished_at"].is_null());
            for (key, label) in [
                ("outcome", "Result outcome"),
                ("code", "Result code"),
                ("message", "Result message"),
            ] {
                assert_line(&text, label, dto["result"][key].as_str().unwrap());
            }
            assert_line(&text, "Result data name", "origin");
            assert_line(
                &text,
                "Result data location",
                dto["result"]["data"]["location"].as_str().unwrap(),
            );
        } else {
            assert!(dto["result"].is_null());
            assert_line(&text, "Result", "none");
        }
        assert_eq!(f.config(), before);
    }
}

#[test]
fn request_cancel_human_reports_state_ids_and_local_no_stop_explanation() {
    for finished in [false, true] {
        let f = request_fixture(finished);
        let before = f.config();
        let output = run(&["request", "cancel", "--request-id", REQUEST], &f.env, &[]);
        assert_eq!(output.status, 0);
        assert!(output.stderr.is_empty());
        let text = String::from_utf8(output.stdout).unwrap();
        assert_line(&text, "Request id", REQUEST);
        assert_line(
            &text,
            "State",
            if finished { "finished" } else { "accepted" },
        );
        assert_line(&text, "Requested", "false");
        assert_line(&text, "Acknowledged", "false");
        assert_eq!(
            text.contains("This local command cannot be stopped after acceptance.\n"),
            !finished
        );
        let json = checked(
            &["request", "cancel", "--request-id", REQUEST, "--json"],
            &f.env,
            &[],
            0,
            "ok",
            Some("cancel.schema.json"),
        );
        assert_eq!(json["data"]["request_id"], REQUEST);
        assert_eq!(json["data"]["requested"], false);
        assert_eq!(json["data"]["acknowledged"], false);
        assert_eq!(f.config(), before);
    }
}

#[test]
fn request_human_failures_use_stderr_only() {
    let f = Fixture::new();
    for verb in ["show", "cancel"] {
        let output = run(&["request", verb, "--request-id", REQUEST], &f.env, &[]);
        assert_eq!(output.status, 2);
        assert!(output.stdout.is_empty());
        let text = String::from_utf8(output.stderr).unwrap();
        assert_line(&text, "code", "request_not_found");
        assert_line(
            &text,
            "error",
            manyhands::results::ResultCode::RequestNotFound.message(),
        );
    }
}

#[test]
fn c2_human_results_split_streams_and_retain_public_ids() {
    for (resource, verb, noop) in [
        ("repo", "identity-set", false),
        ("repo", "identity-set", true),
        ("remote", "add", false),
    ] {
        let f = Fixture::new();
        let confirmation = if resource == "repo" {
            let p = f.prepare();
            if noop {
                f.execute(p["data"]["confirmation_id"].as_str().unwrap());
            }
            Some(p["data"]["confirmation_id"].as_str().unwrap().to_owned())
        } else {
            std::fs::write(&f.input, REMOTE).unwrap();
            None
        };
        let id = if noop {
            "01M4CC0VMTTNKVYS6XWQG4CK1P"
        } else {
            REQUEST
        };
        let mut extra = if noop {
            vec!["--prepare"]
        } else {
            vec!["--request-id", id]
        };
        if !noop && let Some(c) = &confirmation {
            extra.extend(["--confirm", c]);
        }
        // An already-set identity is a no-op at prepare; no confirmation is
        // issued and no new request exists.
        let out = run(&f.args(resource, verb, &extra), &f.env, &[]);
        assert_eq!(out.status, 0, "{}", String::from_utf8_lossy(&out.stderr));
        let stdout = String::from_utf8(out.stdout).unwrap();
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert!(
            stderr
                .lines()
                .all(|line| line.starts_with("progress: ") || line.starts_with("note: "))
        );
        assert!(!stdout.contains("sentinel") && !stderr.contains("sentinel"));
        if noop {
            assert!(stdout.starts_with("Already applied; nothing changed."));
        }
        extra.push("--json");
        let replay = run(&f.args(resource, verb, &extra), &f.env, &[]);
        let v = cli::json(&replay);
        assert_eq!(replay.status, 0);
        for pointer in ["/request_id", "/operation_id"] {
            if let Some(id) = v.pointer(pointer).and_then(serde_json::Value::as_str) {
                assert!(stdout.contains(id));
            }
        }
        if resource == "remote" {
            let mut replacements = vec![(id, "<request-id>")];
            if let Some(op) = v["operation_id"].as_str() {
                replacements.push((op, "<operation-id>"));
            }
            cli::assert_cli_text_golden("remote_add_text", &stdout, &replacements);
        }
    }
}
