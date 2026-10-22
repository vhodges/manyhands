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
