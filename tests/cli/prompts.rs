use super::mutation::{Fixture, REQUEST};
use crate::support::pty::PtyCli;
use std::ffi::OsStr;
fn spawn(f: &Fixture, extra: &[&str], stdin: &[u8]) -> PtyCli {
    PtyCli::spawn(
        &f.args("repo", "identity-set", extra)
            .iter()
            .map(OsStr::new)
            .collect::<Vec<_>>(),
        &f.env,
        stdin,
    )
}
#[test]
fn consent_yes_sets_the_identity() {
    let f = Fixture::new();
    let mut p = spawn(&f, &[], &[]);
    p.expect("Proceed? [y/N] ");
    p.send(b"yes\n");
    let (out, transcript) = p.finish();
    assert_eq!(out.status, 0);
    assert!(transcript.contains("Preview of repo identity-set"));
    assert!(!transcript.contains("Apply with"));
    let stderr = String::from_utf8(out.stderr).unwrap();
    let request = stderr
        .lines()
        .find_map(|s| s.strip_prefix("request: "))
        .unwrap();
    assert!(manyhands::repository::RequestId::parse(request).is_ok());
    assert!(String::from_utf8(out.stdout).unwrap().contains(request));
    assert_eq!(
        git2::Config::open(&f.repo.repository.path().join("config"))
            .unwrap()
            .get_string("user.name")
            .unwrap(),
        "New Person"
    );
}
#[test]
fn consent_no_is_cancelled_exit_130_and_changes_nothing() {
    let f = Fixture::new();
    let before = f.config();
    let mut p = spawn(&f, &[], &[]);
    p.expect("Proceed? [y/N] ");
    p.send(b"n\n");
    let (out, _) = p.finish();
    assert_eq!(out.status, 130);
    assert!(out.stdout.is_empty());
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .contains("code: cancelled")
    );
    assert_eq!(f.config(), before);
}
#[test]
fn ctrl_c_at_consent_is_cancelled_and_restores_the_terminal() {
    let f = Fixture::new();
    let before = f.config();
    let mut p = spawn(&f, &[], &[]);
    p.expect("Proceed? [y/N] ");
    p.send(&[3]);
    let (before_mode, after_mode) = p.lflag_before_and_after();
    assert_eq!(after_mode, before_mode);
    let (out, _) = p.finish();
    assert_eq!(out.status, 130);
    assert_eq!(f.config(), before);
}
#[test]
fn an_existing_request_id_skips_consent() {
    let f = Fixture::new();
    let preview = f.prepare();
    let result = f.execute(preview["data"]["confirmation_id"].as_str().unwrap());
    let before = f.config();
    let p = spawn(&f, &["--request-id", REQUEST], &[]);
    let (out, transcript) = p.finish();
    assert_eq!(out.status, 0);
    assert!(!transcript.contains("Proceed?"));
    assert!(transcript.is_empty());
    assert_eq!(f.config(), before);
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains(result["request_id"].as_str().unwrap()));
}
#[test]
fn stdin_y_with_n_on_the_terminal_is_cancelled() {
    let f = Fixture::new();
    let before = f.config();
    let mut p = spawn(&f, &[], b"y\n");
    p.expect("Proceed? [y/N] ");
    p.send(b"n\n");
    let (out, _) = p.finish();
    assert_eq!(out.status, 130);
    assert_eq!(f.config(), before);
}

fn spawn_repository(f: &super::repository::Fixture, verb: &str, extra: &[&str]) -> PtyCli {
    PtyCli::spawn(
        &f.args(verb, extra)
            .iter()
            .map(OsStr::new)
            .collect::<Vec<_>>(),
        &f.env,
        &[],
    )
}

fn missing_identity_fixture(verb: &str) -> super::repository::Fixture {
    let mut f = super::repository::Fixture::new(verb);
    if verb == "enable" {
        f.remove_local_identity();
    }
    f.set_input(br#"{"primary_branch":"main"}"#);
    f
}

fn ask_identity_and_verify(verb: &str) {
    let mut f = missing_identity_fixture(verb);
    let global = f.global();
    let mut p = spawn_repository(&f, verb, &[]);
    p.expect("Name for commits in this repository: ");
    p.send(b"Prompted Person\n");
    p.expect("Email: ");
    p.send(b"prompted@example.invalid\n");
    p.expect("Proceed? [y/N] ");
    p.send(b"yes\n");
    let (out, transcript) = p.finish();
    assert_eq!(out.status, 0, "{}", String::from_utf8_lossy(&out.stderr));
    assert!(transcript.contains(&format!("Preview of repo {verb}")));
    assert!(transcript.contains("Identity name: Prompted Person"));
    assert!(transcript.contains("Identity email: prompted@example.invalid"));
    assert!(transcript.contains("Identity source: supplied"));
    assert!(transcript.contains("Identity written locally: true"));
    let stderr = String::from_utf8(out.stderr).unwrap();
    let request = stderr
        .lines()
        .find_map(|line| line.strip_prefix("request: "))
        .unwrap();
    assert!(manyhands::repository::RequestId::parse(request).is_ok());
    assert!(
        stderr.contains(
            "every retry, including at a terminal, must give it as \"identity\" in --input"
        )
    );
    assert!(!stderr.contains("Prompted Person") && !stderr.contains("prompted@example.invalid"));
    assert_eq!(
        f.local().get_string("user.name").unwrap(),
        "Prompted Person"
    );
    assert_eq!(
        f.local().get_string("user.email").unwrap(),
        "prompted@example.invalid"
    );
    assert_eq!(f.global(), global);

    // Owner ruling: an existing-record retry never reconstructs prompted input.
    let before = crate::support::repository_and_worktree_snapshot_at(&f.root);
    let p = spawn_repository(&f, verb, &["--request-id", request]);
    let (retry, transcript) = p.finish();
    assert_eq!(retry.status, 2);
    assert!(retry.stdout.is_empty());
    assert!(
        String::from_utf8(retry.stderr)
            .unwrap()
            .contains("code: request_mismatch")
    );
    assert!(transcript.is_empty());
    f.set_input(br#"{"primary_branch":"main","identity":{"name":"Prompted Person","email":"prompted@example.invalid"}}"#);
    let p = spawn_repository(&f, verb, &["--request-id", request]);
    let (replay, transcript) = p.finish();
    assert_eq!(replay.status, 0);
    assert_eq!(replay.stdout, out.stdout);
    assert!(transcript.is_empty());
    assert_eq!(
        crate::support::repository_and_worktree_snapshot_at(&f.root),
        before
    );
    assert_eq!(f.global(), global);
}

#[test]
fn create_asks_for_an_identity_and_previews_it() {
    super::repository::require_no_system_identity();
    ask_identity_and_verify("create");
}

#[test]
fn enable_asks_for_an_identity_and_previews_it() {
    super::repository::require_no_system_identity();
    ask_identity_and_verify("enable");
}

#[test]
fn ctrl_c_at_the_identity_prompt_is_cancelled_and_creates_nothing() {
    super::repository::require_no_system_identity();
    let f = missing_identity_fixture("create");
    let global = f.global();
    let mut p = spawn_repository(&f, "create", &[]);
    p.expect("Name for commits in this repository: ");
    p.send(&[3]);
    let (out, transcript) = p.finish();
    assert_eq!(out.status, 130);
    assert!(out.stdout.is_empty());
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .contains("code: cancelled")
    );
    assert!(!transcript.contains("Proceed?"));
    assert!(!f.root.exists());
    assert_eq!(f.global(), global);
}

#[test]
fn three_empty_names_are_invalid_input() {
    super::repository::require_no_system_identity();
    let f = missing_identity_fixture("create");
    let global = f.global();
    let mut p = spawn_repository(&f, "create", &[]);
    for _ in 0..3 {
        p.expect("Name for commits in this repository: ");
        p.send(b"\n");
        p.expect("Email: ");
        p.send(b"prompted@example.invalid\n");
    }
    let (out, transcript) = p.finish();
    assert_eq!(out.status, 2);
    assert!(out.stdout.is_empty());
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .contains("code: invalid_input")
    );
    assert_eq!(
        transcript
            .matches("Name for commits in this repository: ")
            .count(),
        3
    );
    assert!(!transcript.contains("Proceed?"));
    assert!(!f.root.exists());
    assert_eq!(f.global(), global);
}
