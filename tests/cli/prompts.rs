use super::mutation::{Fixture, REQUEST};
use crate::support::pty::PtyCli;
use std::ffi::OsStr;

pub const PASSPHRASE: &str = "pass-sentinel-ÆØ";
pub fn spawn_generate(env: &crate::support::cli::CliEnv, extra: &[&str], input: &[u8]) -> PtyCli {
    let mut args = vec!["key", "generate", "--input", "-"];
    args.extend_from_slice(extra);
    PtyCli::spawn(&args.iter().map(OsStr::new).collect::<Vec<_>>(), env, input)
}
pub fn answer_passphrase(p: &mut PtyCli) {
    p.expect("New passphrase for key \"Protected key\": ");
    p.send(PASSPHRASE.as_bytes());
    p.send(b"\n");
    p.expect("Repeat it: ");
    p.send(PASSPHRASE.as_bytes());
    p.send(b"\n");
}
fn allocated_request(stderr: &[u8]) -> String {
    let text = std::str::from_utf8(stderr).unwrap();
    let id = text
        .lines()
        .find_map(|line| line.strip_prefix("request: "))
        .unwrap();
    manyhands::repository::RequestId::parse(id).unwrap();
    id.into()
}
pub fn protected_generation(
    env: &crate::support::cli::CliEnv,
) -> (crate::support::cli::CliRun, String) {
    let mut p = spawn_generate(env, &[], super::keys::PROTECTED);
    // Read the flushed note before supplying either secret answer, not merely
    // from the final buffered result after execution has completed.
    let note = p.first_stderr_line();
    let early_id = note.strip_prefix("request: ").unwrap();
    manyhands::repository::RequestId::parse(early_id).unwrap();
    answer_passphrase(&mut p);
    let (out, transcript) = p.finish();
    assert_eq!(out.status, 0);
    assert_eq!(transcript.matches("New passphrase").count(), 1);
    assert_eq!(transcript.matches("Repeat it:").count(), 1);
    assert!(!transcript.contains(PASSPHRASE));
    let request = allocated_request(&out.stderr);
    assert_eq!(request, early_id);
    assert!(std::str::from_utf8(&out.stdout).unwrap().contains(&request));
    (out, transcript)
}

#[test]
fn a_protected_generation_asks_twice_masked() {
    let f = super::keys::Fixture::new();
    let (out, transcript) = protected_generation(&f.env);
    let listed = f.call("list", &[], &[], 0, "ok");
    let path = listed["data"]["items"][0]["private_key_path"]
        .as_str()
        .unwrap();
    let key = ssh_key::PrivateKey::from_openssh(std::fs::read(path).unwrap()).unwrap();
    assert!(key.is_encrypted());
    assert!(key.decrypt("wrong-answer").is_err());
    assert!(key.decrypt(PASSPHRASE).is_ok());
    super::privacy::scan_passphrase(&f.env, &out, &transcript);
}

#[test]
fn mismatched_passphrases_ask_again_and_three_failures_are_invalid_input() {
    let f = super::keys::Fixture::new();
    let mut p = spawn_generate(&f.env, &[], super::keys::PROTECTED);
    for _ in 0..3 {
        p.expect("New passphrase for key \"Protected key\": ");
        p.send(PASSPHRASE.as_bytes());
        p.send(b"\n");
        p.expect("Repeat it: ");
        p.send(b"different\n");
    }
    let (out, transcript) = p.finish();
    assert_eq!(out.status, 2);
    assert!(out.stdout.is_empty());
    assert_eq!(transcript.matches("Passphrases do not match.").count(), 3);
    let id = allocated_request(&out.stderr);
    let stderr = std::str::from_utf8(&out.stderr).unwrap();
    assert!(stderr.contains("code: invalid_input"));
    assert_eq!(stderr.matches(&format!("request: {id}")).count(), 2);
    super::keys::no_generated_key_or_request(&f.env, &id);
    super::privacy::scan_passphrase(&f.env, &out, &transcript);
}

#[test]
fn ctrl_c_at_the_passphrase_prompt_is_cancelled_and_writes_no_key() {
    let f = super::keys::Fixture::new();
    let mut p = spawn_generate(&f.env, &[], super::keys::PROTECTED);
    p.expect("New passphrase for key \"Protected key\": ");
    p.send(&[3]);
    let (before, after) = p.lflag_before_and_after();
    assert_eq!(before, after);
    let (out, transcript) = p.finish();
    assert_eq!(out.status, 130);
    assert!(out.stdout.is_empty());
    let id = allocated_request(&out.stderr);
    let stderr = std::str::from_utf8(&out.stderr).unwrap();
    assert!(stderr.contains("code: cancelled"));
    assert_eq!(stderr.matches(&format!("request: {id}")).count(), 2);
    super::keys::no_generated_key_or_request(&f.env, &id);
    super::privacy::scan_passphrase(&f.env, &out, &transcript);
}

#[test]
fn a_protected_generation_retried_without_a_terminal_is_unlock_required_and_request_show_has_its_result()
 {
    let f = super::keys::Fixture::new();
    let (out, _) = protected_generation(&f.env);
    let id = allocated_request(&out.stderr);
    f.call(
        "generate",
        &["--input", "-", "--request-id", &id],
        super::keys::PROTECTED,
        3,
        "unlock_required",
    );
    let stored = super::mutation::checked(
        &["request", "show", "--json", "--request-id", &id],
        &f.env,
        &[],
        0,
        "ok",
        Some("request.schema.json"),
    );
    assert_eq!(stored["data"]["state"], "finished");
    assert_eq!(stored["data"]["result"]["code"], "ok");
    assert_eq!(stored["data"]["result"]["outcome"], "success");
    crate::support::schema::check_published(
        "key_mutation.schema.json",
        &stored["data"]["result"]["data"],
    )
    .unwrap();
    assert!(
        std::str::from_utf8(&out.stdout)
            .unwrap()
            .contains(stored["data"]["result"]["data"]["id"].as_str().unwrap())
    );
    let mut p = spawn_generate(&f.env, &["--request-id", &id], super::keys::PROTECTED);
    answer_passphrase(&mut p);
    let (retry, transcript) = p.finish();
    assert_eq!(retry.status, 0);
    assert_eq!(retry.stdout, out.stdout);
    assert!(
        !std::str::from_utf8(&retry.stderr)
            .unwrap()
            .contains("retrying")
    );
    super::privacy::scan_passphrase(&f.env, &retry, &transcript);
}

#[test]
fn generate_prepare_on_a_terminal_never_prompts_or_allocates_a_request() {
    let f = super::keys::Fixture::new();
    for input in [super::keys::GENERATE, super::keys::PROTECTED] {
        let p = spawn_generate(&f.env, &["--prepare"], input);
        let (out, transcript) = p.finish();
        assert_eq!(out.status, 2);
        assert!(transcript.is_empty());
        let text = std::str::from_utf8(&out.stderr).unwrap();
        assert!(text.contains("code: not_confirmable"));
        assert!(!text.contains("request:"));
    }
    super::keys::no_generated_key_or_request(&f.env, REQUEST);
}

#[test]
fn interactive_input_refusals_retain_the_allocated_request_before_prompts() {
    let f = super::keys::Fixture::new();
    let p = spawn_generate(&f.env, &[], b"{}");
    let (out, transcript) = p.finish();
    assert_eq!(out.status, 2);
    assert!(transcript.is_empty());
    let id = allocated_request(&out.stderr);
    assert_eq!(
        std::str::from_utf8(&out.stderr)
            .unwrap()
            .matches(&format!("request: {id}"))
            .count(),
        2
    );
    super::keys::no_generated_key_or_request(&f.env, &id);
}
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
