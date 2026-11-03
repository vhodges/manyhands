use super::*;
use remote_world::{Provider, World};
use repository::{keys::*, transport::*};
use ssh_remote::{FixtureError, fixed};
pub const WRONG: &str = "wrong-CLI-unlock-sentinel-ÆØ";
pub const CASES: &[ssh_harness::Case] = &[
    (
        "json_approve_with_a_protected_key_is_unlock_required",
        json_approve_with_a_protected_key_is_unlock_required,
    ),
    (
        "json_observe_of_a_pinned_host_with_a_protected_key_is_unlock_required",
        json_observe_of_a_pinned_host_with_a_protected_key_is_unlock_required,
    ),
    (
        "observe_of_an_unpinned_host_needs_no_unlock",
        observe_of_an_unpinned_host_needs_no_unlock,
    ),
    #[cfg(unix)]
    (
        "the_right_passphrase_unlocks_approve_and_observe_of_a_pinned_host",
        the_right_passphrase_unlocks_approve_and_observe_of_a_pinned_host,
    ),
    #[cfg(unix)]
    (
        "a_wrong_passphrase_is_today_transport_unavailable_with_the_hint",
        a_wrong_passphrase_is_today_transport_unavailable_with_the_hint,
    ),
    #[cfg(unix)]
    (
        "observe_of_a_pinned_host_with_a_wrong_passphrase_shows_the_hint",
        observe_of_a_pinned_host_with_a_wrong_passphrase_shows_the_hint,
    ),
    #[cfg(unix)]
    (
        "a_rejected_key_after_the_right_passphrase_is_unlock_failed",
        a_rejected_key_after_the_right_passphrase_is_unlock_failed,
    ),
    #[cfg(unix)]
    (
        "ctrl_c_at_the_unlock_prompt_of_approve_is_unlock_required",
        ctrl_c_at_the_unlock_prompt_of_approve_is_unlock_required,
    ),
    #[cfg(unix)]
    (
        "ctrl_c_at_the_unlock_prompt_of_observe_is_unlock_required",
        ctrl_c_at_the_unlock_prompt_of_observe_is_unlock_required,
    ),
    #[cfg(unix)]
    (
        "no_passphrase_reaches_the_transcript_the_data_or_the_home",
        no_passphrase_reaches_the_transcript_the_data_or_the_home,
    ),
];
fn world() -> Result<(World, CliEnv), FixtureError> {
    let w = World::new()?;
    private_fixture(&w.server);
    let env = CliEnv::new(w.data());
    // World registers and selects acceptance-selected. Inspect, don't import a duplicate.
    let keys = checked(
        &env,
        &["key", "list"],
        &[],
        "ok",
        0,
        Some("key_list.schema.json"),
    );
    assert_eq!(keys["data"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(keys["data"]["items"][0]["label"], "acceptance-selected");
    assert_eq!(keys["data"]["items"][0]["selected"], true);
    Ok((w, env))
}
fn approve(w: &World) -> Result<(), FixtureError> {
    fixed(w.service.verify_ssh_transport(
        VerifySshTransportRequest {
            root: w.root.clone(),
            direction: SshDirection::Fetch,
            approval: Some(HostApproval {
                authority: SshAuthority {
                    host: "127.0.0.1".into(),
                    port: w.server.address().port(),
                },
                expected: None,
                presented: w.server.host_identity(),
            }),
        },
        &mut SessionCredentials::new(Provider),
    ))?;
    Ok(())
}
fn json_approve_with_a_protected_key_is_unlock_required() -> Result<(), FixtureError> {
    let (w, env) = world()?;
    let input = host_input(&w.server);
    let preview = prepare(&env, &w.root, "approve", &input);
    confirm(
        &env,
        &w.root,
        "approve",
        &input,
        &preview,
        "unlock_required",
        3,
    );
    assert_eq!(pins(&env), serde_json::json!([]));
    Ok(())
}
fn json_observe_of_a_pinned_host_with_a_protected_key_is_unlock_required()
-> Result<(), FixtureError> {
    let (w, env) = world()?;
    approve(&w)?;
    let before = pins(&env);
    host_call(
        &env,
        &w.root,
        "observe",
        &Value::Null,
        &[],
        "unlock_required",
        3,
    );
    assert_eq!(pins(&env), before);
    Ok(())
}
fn observe_of_an_unpinned_host_needs_no_unlock() -> Result<(), FixtureError> {
    let (w, env) = world()?;
    let value = host_call(&env, &w.root, "observe", &Value::Null, &[], "ok", 0);
    assert!(value["data"]["pinned_fingerprint"].is_null());
    assert_eq!(pins(&env), serde_json::json!([]));
    Ok(())
}

#[cfg(unix)]
fn prompted(
    w: &World,
    env: &CliEnv,
    verb: &str,
    answer: Option<&str>,
    status: i32,
    code: Option<&str>,
) -> String {
    use support::pty::PtyCli;
    let input = host_input(&w.server);
    let request = repository::RequestId::new().to_string();
    let mut args = vec!["host", verb, "--repo", w.root.to_str().unwrap()];
    let preview;
    if verb == "approve" {
        preview = prepare(env, &w.root, verb, &input);
        args.extend([
            "--input",
            "-",
            "--request-id",
            &request,
            "--confirm",
            preview["data"]["confirmation_id"].as_str().unwrap(),
        ]);
    }
    let mut pty = PtyCli::spawn(
        &args.iter().map(OsStr::new).collect::<Vec<_>>(),
        env,
        &serde_json::to_vec(&input).unwrap(),
    );
    pty.expect("Passphrase for key");
    match answer {
        Some(pass) => pty.send(format!("{pass}\n").as_bytes()),
        None => pty.send(&[3]),
    }
    let (out, transcript) = pty.finish();
    scan_output(&out);
    for bytes in [
        out.stdout.as_slice(),
        out.stderr.as_slice(),
        transcript.as_bytes(),
    ] {
        absent(bytes, remote_world::PASSWORD);
        absent(bytes, WRONG);
    }
    scan(&env.data);
    scan(env.home.path());
    assert_eq!(out.status, status);
    assert_eq!(transcript.matches("Passphrase for key").count(), 1);
    let err = std::str::from_utf8(&out.stderr).unwrap();
    assert!(!err.contains("progress: retrying"));
    if let Some(code) = code {
        assert!(out.stdout.is_empty());
        assert!(err.contains(&format!("code: {code}\n")));
    } else {
        assert!(!out.stdout.is_empty());
        assert!(!err.contains("error:"));
    }
    assert_eq!(
        err.contains("hint: if the passphrase was wrong, run the command again"),
        code == Some("transport_unavailable")
    );
    request
}
#[cfg(unix)]
fn scan(path: &Path) {
    for entry in std::fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            scan(&entry.path());
        } else if entry.file_type().unwrap().is_file() {
            let bytes = std::fs::read(entry.path()).unwrap();
            absent(&bytes, remote_world::PASSWORD);
            absent(&bytes, WRONG);
        }
    }
}
#[cfg(unix)]
fn the_right_passphrase_unlocks_approve_and_observe_of_a_pinned_host() -> Result<(), FixtureError> {
    let (w, env) = world()?;
    prompted(&w, &env, "approve", Some(remote_world::PASSWORD), 0, None);
    assert_eq!(
        pins(&env)[0]["pinned_fingerprint"],
        w.server.host_identity().sha256
    );
    prompted(&w, &env, "observe", Some(remote_world::PASSWORD), 0, None);
    Ok(())
}
#[cfg(unix)]
fn a_wrong_passphrase_is_today_transport_unavailable_with_the_hint() -> Result<(), FixtureError> {
    let (w, env) = world()?;
    prompted(
        &w,
        &env,
        "approve",
        Some(WRONG),
        5,
        Some("transport_unavailable"),
    );
    assert_eq!(pins(&env), serde_json::json!([]));
    Ok(())
}
#[cfg(unix)]
fn observe_of_a_pinned_host_with_a_wrong_passphrase_shows_the_hint() -> Result<(), FixtureError> {
    let (w, env) = world()?;
    approve(&w)?;
    let before = pins(&env);
    prompted(
        &w,
        &env,
        "observe",
        Some(WRONG),
        5,
        Some("transport_unavailable"),
    );
    assert_eq!(pins(&env), before);
    Ok(())
}
#[cfg(unix)]
fn a_rejected_key_after_the_right_passphrase_is_unlock_failed() -> Result<(), FixtureError> {
    let (w, env) = world()?;
    w.server.reject_client();
    prompted(
        &w,
        &env,
        "approve",
        Some(remote_world::PASSWORD),
        3,
        Some("unlock_failed"),
    );
    assert_eq!(pins(&env), serde_json::json!([]));
    Ok(())
}
#[cfg(unix)]
fn ctrl_c_at_the_unlock_prompt_of_approve_is_unlock_required() -> Result<(), FixtureError> {
    let (w, env) = world()?;
    let request = prompted(&w, &env, "approve", None, 3, Some("unlock_required"));
    assert_eq!(pins(&env), serde_json::json!([]));
    checked(
        &env,
        &["request", "show", "--request-id", &request],
        &[],
        "request_not_found",
        2,
        None,
    );
    Ok(())
}
#[cfg(unix)]
fn ctrl_c_at_the_unlock_prompt_of_observe_is_unlock_required() -> Result<(), FixtureError> {
    let (w, env) = world()?;
    approve(&w)?;
    let before = pins(&env);
    prompted(&w, &env, "observe", None, 3, Some("unlock_required"));
    assert_eq!(pins(&env), before);
    Ok(())
}
#[cfg(unix)]
fn no_passphrase_reaches_the_transcript_the_data_or_the_home() -> Result<(), FixtureError> {
    // Both successful invocations scan streams, transcript, data and home in both encodings.
    the_right_passphrase_unlocks_approve_and_observe_of_a_pinned_host()
}
