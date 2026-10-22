use super::mutation::{Fixture, REMOTE, REQUEST, checked, run};
use crate::support::cli;

fn key_human_result(
    f: &super::keys::Fixture,
    verb: &str,
    flags: &[&str],
    input: &[u8],
    noop: bool,
) {
    let mut args = vec!["key", verb];
    args.extend_from_slice(flags);
    let out = run(&args, &f.env, input);
    assert_eq!(out.status, 0, "{}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8(out.stdout).unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr
            .lines()
            .all(|line| line.starts_with("progress: ") || line.starts_with("note: "))
    );
    assert_eq!(
        stdout.starts_with("Already applied; nothing changed."),
        noop
    );
    args.push("--json");
    let replay = checked(
        &args,
        &f.env,
        input,
        0,
        if noop { "already_applied" } else { "ok" },
        Some("key_mutation.schema.json"),
    );
    for pointer in ["/request_id", "/operation_id", "/data/id"] {
        if let Some(id) = replay.pointer(pointer).and_then(serde_json::Value::as_str) {
            assert!(stdout.contains(id));
        }
    }
    for (field, label) in [
        ("label", "Label"),
        ("fingerprint", "Fingerprint"),
        ("deletion", "Deletion"),
        ("generation", "Generation"),
    ] {
        assert_line(
            &stdout,
            label,
            replay["data"][field].as_str().unwrap_or("none"),
        );
    }
    for sentinel in crate::support::credentials::SECRET_SENTINELS {
        assert!(!stdout.contains(sentinel) && !stderr.contains(sentinel));
    }
}

#[test]
fn c2_keys_human_results_and_selection_noops_retain_public_ids() {
    use super::keys::{Fixture as KeyFixture, IMPORT, key_id};
    use super::remotes::{LAST, NEXT};
    for (verb, noop) in [
        ("import", false),
        ("import", true),
        ("select", false),
        ("clear", false),
        ("remove", false),
        ("select", true),
        ("clear", true),
    ] {
        let f = KeyFixture::new();
        let imported = (verb != "import" || noop).then(|| f.import());
        let id = imported.as_ref().map(key_id).unwrap_or(REQUEST);
        if (verb == "select" && noop) || (verb == "clear" && !noop) {
            f.call(
                "select",
                &["--key-id", id, "--request-id", NEXT],
                &[],
                0,
                "ok",
            );
        }
        let mut flags = vec!["--request-id", LAST];
        if verb == "import" {
            flags.extend(["--input", "-"]);
        }
        if ["select", "remove"].contains(&verb) {
            flags.extend(["--key-id", id]);
        }
        key_human_result(&f, verb, &flags, IMPORT, noop);
    }
}

#[cfg(unix)]
#[test]
fn key_delete_human_preview_and_result() {
    let f = super::keys::Fixture::new();
    let key = super::keys::generate(&f.env);
    let id = key.id.to_string();
    let out = run(
        &["key", "delete", "--key-id", &id, "--prepare"],
        &f.env,
        &[],
    );
    assert_eq!(out.status, 0);
    assert!(out.stderr.is_empty());
    let text = String::from_utf8(out.stdout).unwrap();
    for expected in [
        "Preview of key delete",
        "Will delete the private key file.",
        "Will delete the public key file.",
        "Will unregister the key.",
        "Confirmation: ",
        "--confirm",
        "--request-id",
        &id,
        key.private_key_path.to_str().unwrap(),
        key.public_key_path.as_ref().unwrap().to_str().unwrap(),
    ] {
        assert!(text.contains(expected));
    }
    let preview = f.call("delete", &["--key-id", &id, "--prepare"], &[], 0, "ok");
    key_human_result(
        &f,
        "delete",
        &[
            "--key-id",
            &id,
            "--request-id",
            REQUEST,
            "--confirm",
            preview["data"]["confirmation_id"].as_str().unwrap(),
        ],
        &[],
        false,
    );
    assert!(!key.private_key_path.exists());
    assert!(!key.public_key_path.unwrap().exists());
}

#[test]
fn key_human_refusals_use_stderr_only_and_preserve_imported_files() {
    let f = super::keys::Fixture::new();
    let imported = f.import();
    let id = super::keys::key_id(&imported);
    let before = crate::support::credentials::key_file_states(&f.files());
    for (verb, flags, exit, code) in [
        (
            "delete",
            vec!["--key-id", id, "--prepare"],
            2,
            "key_not_deletable",
        ),
        (
            "select",
            vec!["--key-id", REQUEST, "--request-id", super::remotes::NEXT],
            2,
            "key_not_found",
        ),
    ] {
        let mut args = vec!["key", verb];
        args.extend(flags);
        let out = run(&args, &f.env, &[]);
        assert_eq!(out.status, exit);
        assert!(out.stdout.is_empty());
        let text = String::from_utf8(out.stderr).unwrap();
        assert_line(&text, "code", code);
        assert!(text.contains("error: "));
    }
    assert_eq!(
        crate::support::credentials::key_file_states(&f.files()),
        before
    );
}

#[test]
fn c2_remote_human_results_and_noops_retain_public_ids() {
    use super::remotes::{self, LAST, SELECT};
    for (verb, noop, clear) in [
        ("select", false, false),
        ("select", true, false),
        ("select", false, true),
        ("select", true, true),
        ("remove", false, false),
    ] {
        let f = Fixture::new();
        remotes::add(&f);
        if (noop && !clear) || (!noop && clear) {
            remotes::select(&f);
        }
        let input = if clear {
            br#"{"name":null}"#.as_slice()
        } else {
            SELECT
        };
        let preview =
            (verb == "remove").then(|| remotes::call(&f, verb, input, &["--prepare"], 0, "ok"));
        std::fs::write(&f.input, input).unwrap();
        let mut flags = vec!["--request-id", LAST];
        if let Some(preview) = &preview {
            flags.extend([
                "--confirm",
                preview["data"]["confirmation_id"].as_str().unwrap(),
            ]);
        }
        let output = run(&f.args("remote", verb, &flags), &f.env, &[]);
        assert_eq!(output.status, 0);
        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(
            stderr
                .lines()
                .all(|line| line.starts_with("progress: ") || line.starts_with("note: "))
        );
        assert_eq!(
            stdout.starts_with("Already applied; nothing changed."),
            noop
        );
        assert!(!stdout.contains("sentinel") && !stderr.contains("sentinel"));
        flags.push("--json");
        let replay = checked(
            &f.args("remote", verb, &flags),
            &f.env,
            &[],
            0,
            if noop { "already_applied" } else { "ok" },
            Some("remote_mutation.schema.json"),
        );
        for pointer in ["/request_id", "/operation_id", "/effects/commit_oid"] {
            if let Some(id) = replay.pointer(pointer).and_then(serde_json::Value::as_str) {
                assert!(stdout.contains(id));
            }
        }
        assert!(stdout.contains(if clear { "Name: none" } else { "Name: origin" }));
    }
}

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
