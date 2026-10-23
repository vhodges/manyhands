//! Real CLI host commands, one isolated SSH fixture process per case.
pub use manyhands::{repository, runtime};
#[path = "cli_remote/hosts.rs"]
mod hosts;
#[path = "cli_remote/journeys.rs"]
mod journeys;
// As in the other SSH targets, preserve the shared fixture's domain error type.
// Limit this allowance to that imported module, not the CLI cases or bin modules.
#[allow(clippy::result_large_err)]
#[path = "support/remote_world.rs"]
mod remote_world;
#[path = "support/ssh_harness.rs"]
mod ssh_harness;
#[path = "support/ssh_privacy.rs"]
mod ssh_privacy;
#[path = "support/ssh_remote.rs"]
mod ssh_remote;
mod support;
// Pure renderer/parser evidence runs in the same task target; no SSH fixture
// is introduced into the bin's libtest process, and IPv6 needs no live listener.
#[allow(dead_code, unused_imports)]
#[path = "../src/bin/manyhands-cli/catalog.rs"]
mod catalog;
#[allow(dead_code, unused_imports)]
#[path = "../src/bin/manyhands-cli/human_admin.rs"]
mod human_admin;
#[allow(dead_code, unused_imports)]
#[path = "../src/bin/manyhands-cli/parse.rs"]
mod parse;
#[allow(dead_code, unused_imports)]
#[path = "../src/bin/manyhands-cli/render.rs"]
mod render;
#[path = "cli_remote/unlock.rs"]
mod unlock;

use serde_json::Value;
use std::{ffi::OsStr, path::Path};
use support::{
    cli::{self, CliEnv, CliRun},
    schema,
};

thread_local! {
    // Test-only, in-memory probes. The CLI receives none of these through its
    // arguments, environment, input object or stdin.
    static OUTPUT_PROBES: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn private_fixture(server: &ssh_remote::SshRemoteFixture) {
    let private = std::fs::read_to_string(server.client_key_path()).unwrap();
    OUTPUT_PROBES.with(|probes| {
        let mut probes = probes.borrow_mut();
        probes.extend(
            private
                .lines()
                .filter(|line| line.len() > 40)
                .map(str::to_owned),
        );
    });
}

fn scan_output(out: &CliRun) {
    for bytes in [&out.stdout, &out.stderr] {
        OUTPUT_PROBES.with(|probes| {
            for sentinel in probes.borrow().iter() {
                absent(bytes, sentinel);
            }
        });
        for sentinel in support::credentials::SECRET_SENTINELS
            .iter()
            .copied()
            .chain([
                support::operations::KEY_MATERIAL_SENTINEL,
                remote_world::PASSWORD,
                unlock::WRONG,
            ])
        {
            absent(bytes, sentinel);
        }
    }
}

fn main() {
    // SAFETY: real main, before any fixture or runner thread exists.
    if unsafe { ssh_harness::initialize() }.is_err() {
        eprintln!("CLI SSH initialization failed");
        std::process::exit(1);
    }
    let cases: Vec<_> = hosts::CASES
        .iter()
        .chain(unlock::CASES)
        .chain(journeys::CASES)
        .copied()
        .collect();
    ssh_harness::run(&cases);
}

fn run(env: &CliEnv, args: &[&str], input: &[u8]) -> CliRun {
    let out = cli::run_cli_with_stdin(&args.iter().map(OsStr::new).collect::<Vec<_>>(), env, input);
    // Scan all helper-produced output, including preparation and companion reads.
    scan_output(&out);
    out
}

fn checked(
    env: &CliEnv,
    args: &[&str],
    input: &[u8],
    code: &str,
    status: i32,
    data: Option<&str>,
) -> Value {
    let mut args = args.to_vec();
    args.push("--json");
    let out = run(env, &args, input);
    let value = cli::json(&out);
    if value["code"] != code {
        eprintln!(
            "expected code {code}; received code {}",
            value["code"].as_str().unwrap()
        );
    }
    assert_eq!(value["code"], code);
    assert_eq!(out.status, status);
    schema::check_published("envelope.schema.json", &value).unwrap();
    if let Some(data) = data {
        schema::check_published(data, &value["data"]).unwrap();
    }
    value
}

fn absent(bytes: &[u8], sentinel: &str) {
    for needle in [
        sentinel.as_bytes().to_vec(),
        sentinel.encode_utf16().flat_map(u16::to_le_bytes).collect(),
    ] {
        assert!(
            !bytes.windows(needle.len()).any(|window| window == needle),
            "private sentinel leaked"
        );
    }
}

fn golden(name: &str, value: &Value, root: &Path, port: u16) {
    let mut value = value.clone();
    for pointer in ["/data/port", "/data/details/port"] {
        if let Some(found) = value.pointer_mut(pointer) {
            assert_eq!(*found, serde_json::json!(port));
            *found = serde_json::json!(0);
        }
    }
    if let Some(authority) = value.pointer_mut("/recovery/0/arguments/authority") {
        assert_eq!(*authority, serde_json::json!(format!("127.0.0.1:{port}")));
        *authority = "127.0.0.1:0".into();
    }
    let mut replacements = vec![(
        root.canonicalize().unwrap().to_str().unwrap().to_owned(),
        "<repository>",
    )];
    for (pointer, label) in [
        ("/request_id", "<request-id>"),
        ("/operation_id", "<operation-id>"),
        ("/data/confirmation_id", "<confirmation-id>"),
        ("/data/expires_at", "<expires-at>"),
        ("/data/fingerprint", "<fingerprint>"),
        ("/data/presented_fingerprint", "<presented>"),
        ("/data/pinned_fingerprint", "<pinned>"),
        ("/data/details/fingerprint", "<fingerprint>"),
        ("/data/details/pinned_fingerprint", "<pinned>"),
    ] {
        if let Some(s) = value.pointer(pointer).and_then(Value::as_str)
            && !replacements.iter().any(|(v, _)| v == s)
        {
            replacements.push((s.to_owned(), label));
        }
    }
    cli::assert_cli_golden(
        name,
        &value,
        &replacements
            .iter()
            .map(|(v, k)| (v.as_str(), *k))
            .collect::<Vec<_>>(),
    );
}

fn host_input(server: &ssh_remote::SshRemoteFixture) -> Value {
    let key = server.host_identity();
    serde_json::json!({"authority":format!("127.0.0.1:{}", server.address().port()),"algorithm":key.algorithm,"fingerprint":key.sha256})
}

fn host_call(
    env: &CliEnv,
    root: &Path,
    verb: &str,
    input: &Value,
    extra: &[&str],
    code: &str,
    status: i32,
) -> Value {
    let mut args = vec!["host", verb, "--repo", root.to_str().unwrap()];
    if verb != "observe" {
        args.extend(["--input", "-"]);
    }
    args.extend_from_slice(extra);
    let data = if code == "ok" {
        Some(if verb == "observe" {
            "host_observation.schema.json"
        } else if extra.contains(&"--prepare") {
            "host_preview.schema.json"
        } else {
            "host_mutation.schema.json"
        })
    } else if code == "host_mismatch" {
        Some("host_mutation.schema.json")
    } else if code == "host_replacement_required" {
        Some("host_replacement.schema.json")
    } else {
        None
    };
    checked(
        env,
        &args,
        &serde_json::to_vec(input).unwrap(),
        code,
        status,
        data,
    )
}

fn prepare(env: &CliEnv, root: &Path, verb: &str, input: &Value) -> Value {
    host_call(env, root, verb, input, &["--prepare"], "ok", 0)
}

fn confirm(
    env: &CliEnv,
    root: &Path,
    verb: &str,
    input: &Value,
    preview: &Value,
    code: &str,
    status: i32,
) -> Value {
    host_call(
        env,
        root,
        verb,
        input,
        &[
            "--request-id",
            &repository::RequestId::new().to_string(),
            "--confirm",
            preview["data"]["confirmation_id"].as_str().unwrap(),
        ],
        code,
        status,
    )
}

fn pins(env: &CliEnv) -> Value {
    checked(
        env,
        &["host", "list"],
        &[],
        "ok",
        0,
        Some("host_pin_list.schema.json"),
    )["data"]["items"]
        .clone()
}
