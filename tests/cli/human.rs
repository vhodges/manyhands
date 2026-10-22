use super::mutation::{Fixture, REMOTE, REQUEST, run};
use crate::support::cli;
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
