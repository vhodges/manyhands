use crate::support::{
    self,
    cli::{self, CliEnv, CliRun},
    schema,
};
use serde_json::Value;
use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

pub const REQUEST: &str = "01M4CC0VMTTNKVYS6XWQG4CK1N";
pub const IDENTITY: &[u8] = include_bytes!("../fixtures/cli_input_v1/repo_identity_set.json");
pub const REMOTE: &[u8] = include_bytes!("../fixtures/cli_input_v1/remote_add.json");
pub struct Fixture {
    pub repo: support::TestRepository,
    pub env: CliEnv,
    pub input: PathBuf,
    pub data: tempfile::TempDir,
}
impl Fixture {
    pub fn new() -> Self {
        let repo = support::born_repository();
        let enabled = support::enabled_repository(&repo);
        let env = CliEnv::new(enabled.data_directory.path().to_owned());
        let input = env.home.path().join("input.json");
        fs::write(&input, IDENTITY).unwrap();
        fs::write(
            env.home.path().join(".gitconfig"),
            "[user]\nname = Global Unchanged\nemail = global@example.invalid\n",
        )
        .unwrap();
        Self {
            repo,
            env,
            input,
            data: enabled.data_directory,
        }
    }
    pub fn args<'a>(
        &'a self,
        resource: &'a str,
        verb: &'a str,
        extra: &'a [&'a str],
    ) -> Vec<&'a str> {
        let mut args = vec![
            resource,
            verb,
            "--repo",
            self.repo.root.to_str().unwrap(),
            "--input",
            self.input.to_str().unwrap(),
        ];
        args.extend_from_slice(extra);
        args
    }
    pub fn prepare(&self) -> Value {
        checked(
            &self.args("repo", "identity-set", &["--json", "--prepare"]),
            &self.env,
            &[],
            0,
            "ok",
            Some("identity_preview.schema.json"),
        )
    }
    pub fn execute(&self, confirmation: &str) -> Value {
        checked(
            &self.args(
                "repo",
                "identity-set",
                &["--json", "--request-id", REQUEST, "--confirm", confirmation],
            ),
            &self.env,
            &[],
            0,
            "ok",
            Some("identity_mutation.schema.json"),
        )
    }
    pub fn config(&self) -> Vec<u8> {
        fs::read(self.repo.repository.path().join("config")).unwrap()
    }
}
pub fn run(args: &[&str], env: &CliEnv, stdin: &[u8]) -> CliRun {
    cli::run_cli_with_stdin(&args.iter().map(OsStr::new).collect::<Vec<_>>(), env, stdin)
}
pub fn checked(
    args: &[&str],
    env: &CliEnv,
    stdin: &[u8],
    status: i32,
    code: &str,
    data_schema: Option<&str>,
) -> Value {
    let out = run(args, env, stdin);
    let v = cli::json(&out);
    assert_eq!(out.status, status, "{v}");
    assert_eq!(v["code"], code, "{v}");
    schema::check_published("envelope.schema.json", &v).unwrap();
    if let Some(name) = data_schema {
        schema::check_published(name, &v["data"]).unwrap();
    }
    v
}
pub fn golden(name: &str, v: &Value, root: &Path) {
    let mut replacements = vec![(
        fs::canonicalize(root).unwrap().to_str().unwrap().to_owned(),
        "<repository>",
    )];
    for (pointer, label) in [
        ("/request_id", "<request-id>"),
        ("/operation_id", "<operation-id>"),
        ("/data/request_id", "<request-id>"),
        ("/data/confirmation_id", "<confirmation-id>"),
        ("/data/expires_at", "<expires-at>"),
        ("/data/accepted_at", "<timestamp>"),
        ("/data/finished_at", "<timestamp>"),
        ("/data/operations/0/operation_id", "<operation-id>"),
    ] {
        if let Some(s) = v.pointer(pointer).and_then(Value::as_str)
            && !replacements.iter().any(|(value, _)| value == s)
        {
            replacements.push((s.into(), label));
        }
    }
    cli::assert_cli_golden(
        name,
        v,
        &replacements
            .iter()
            .map(|(v, k)| (v.as_str(), *k))
            .collect::<Vec<_>>(),
    );
}
#[test]
fn a_child_has_no_controlling_terminal() {
    let f = Fixture::new();
    let out = run(&f.args("repo", "identity-set", &[]), &f.env, &[]);
    assert_eq!(out.status, 2);
    assert!(out.stdout.is_empty());
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .contains("code: usage_error")
    );
}
#[test]
fn json_needs_a_request_id_and_prepare_refuses_request_or_confirm() {
    let f = Fixture::new();
    for extra in [
        vec!["--json"],
        vec!["--json", "--prepare", "--request-id", REQUEST],
        vec!["--json", "--prepare", "--confirm", REQUEST],
    ] {
        checked(
            &f.args("repo", "identity-set", &extra),
            &f.env,
            &[],
            2,
            "usage_error",
            None,
        );
    }
}
#[test]
fn a_mode_refusal_keeps_request_id_null_and_c2_usage_never_echoes_values() {
    let f = Fixture::new();
    let v = checked(
        &f.args(
            "repo",
            "identity-set",
            &["--json", "--prepare", "--request-id", REQUEST],
        ),
        &f.env,
        &[],
        2,
        "usage_error",
        None,
    );
    assert!(v["request_id"].is_null());
    let out = run(
        &f.args("repo", "identity-set", &["--bogus=private-sentinel"]),
        &f.env,
        &[],
    );
    assert_eq!(out.status, 2);
    assert!(
        !String::from_utf8(out.stderr)
            .unwrap()
            .contains("private-sentinel")
    );
}
#[test]
fn prepare_then_confirm_sets_the_identity_locally() {
    let f = Fixture::new();
    let before = f.config();
    let global = fs::read(f.env.home.path().join(".gitconfig")).unwrap();
    let preview = f.prepare();
    assert_eq!(f.config(), before);
    golden("repo_identity_set_prepare", &preview, &f.repo.root);
    let result = f.execute(preview["data"]["confirmation_id"].as_str().unwrap());
    golden("repo_identity_set", &result, &f.repo.root);
    let config = git2::Config::open(&f.repo.repository.path().join("config")).unwrap();
    assert_eq!(config.get_string("user.name").unwrap(), "New Person");
    assert_eq!(
        config.get_string("user.email").unwrap(),
        "new@example.invalid"
    );
    assert_eq!(config.get_string("manyhands.initials").unwrap(), "np");
    assert_eq!(
        fs::read(f.env.home.path().join(".gitconfig")).unwrap(),
        global
    );
}
#[test]
fn without_confirm_and_waiting_stdin_are_never_consent() {
    let f = Fixture::new();
    let before = f.config();
    for stdin in [&b""[..], &b"y\n"[..]] {
        checked(
            &f.args("repo", "identity-set", &["--json", "--request-id", REQUEST]),
            &f.env,
            stdin,
            3,
            "confirmation_required",
            None,
        );
        let out = run(
            &f.args("repo", "identity-set", &["--request-id", REQUEST]),
            &f.env,
            stdin,
        );
        assert_eq!(out.status, 3);
        assert!(out.stdout.is_empty());
        assert!(
            String::from_utf8(out.stderr)
                .unwrap()
                .contains("confirmation_required")
        );
        assert_eq!(f.config(), before);
    }
}
#[test]
fn replay_and_request_mismatch() {
    let f = Fixture::new();
    let preview = f.prepare();
    let first = f.execute(preview["data"]["confirmation_id"].as_str().unwrap());
    let before = f.config();
    let replay = checked(
        &f.args("repo", "identity-set", &["--json", "--request-id", REQUEST]),
        &f.env,
        &[],
        0,
        "ok",
        Some("identity_mutation.schema.json"),
    );
    assert_eq!(replay, first);
    assert_eq!(f.config(), before);
    fs::write(
        &f.input,
        br#"{"name":"Other","email":"other@example.invalid"}"#,
    )
    .unwrap();
    checked(
        &f.args("repo", "identity-set", &["--json", "--request-id", REQUEST]),
        &f.env,
        &[],
        2,
        "request_mismatch",
        None,
    );
    assert_eq!(f.config(), before);
}
#[test]
fn another_confirmation_or_other_input_is_confirmation_mismatch() {
    let f = Fixture::new();
    let before = f.config();
    let preview = f.prepare();
    let id = preview["data"]["confirmation_id"].as_str().unwrap();
    fs::write(
        &f.input,
        br#"{"name":"Other","email":"other@example.invalid"}"#,
    )
    .unwrap();
    let other = f.prepare();
    checked(
        &f.args(
            "repo",
            "identity-set",
            &["--json", "--request-id", REQUEST, "--confirm", id],
        ),
        &f.env,
        &[],
        2,
        "confirmation_mismatch",
        None,
    );
    fs::write(&f.input, IDENTITY).unwrap();
    checked(
        &f.args(
            "repo",
            "identity-set",
            &[
                "--json",
                "--request-id",
                REQUEST,
                "--confirm",
                other["data"]["confirmation_id"].as_str().unwrap(),
            ],
        ),
        &f.env,
        &[],
        2,
        "confirmation_mismatch",
        None,
    );
    assert_eq!(f.config(), before);
}
#[test]
fn remote_add_from_stdin() {
    let f = Fixture::new();
    let result = checked(
        &[
            "remote",
            "add",
            "--repo",
            f.repo.root.to_str().unwrap(),
            "--input",
            "-",
            "--request-id",
            REQUEST,
            "--json",
        ],
        &f.env,
        REMOTE,
        0,
        "ok",
        Some("remote_mutation.schema.json"),
    );
    golden("remote_add", &result, &f.repo.root);
    assert!(f.repo.repository.find_remote("origin").is_ok());
}
#[test]
fn initials_missing_null_and_set() {
    for (suffix, expected) in [
        ("", Some("old")),
        (",\"initials\":null", None),
        (",\"initials\":\"NP\"", Some("np")),
    ] {
        let f = Fixture::new();
        let mut config = git2::Config::open(&f.repo.repository.path().join("config")).unwrap();
        config.set_str("manyhands.initials", "old").unwrap();
        drop(config);
        fs::write(
            &f.input,
            format!("{{\"name\":\"New Person\",\"email\":\"new@example.invalid\"{suffix}}}"),
        )
        .unwrap();
        let p = f.prepare();
        let result = f.execute(p["data"]["confirmation_id"].as_str().unwrap());
        assert_eq!(result["data"]["initials"], serde_json::json!(expected));
    }
}
#[test]
fn input_refusals_through_the_binary_and_ids_are_ordered() {
    let f = Fixture::new();
    for input in [
        br#"{"name":"sentinel","email":"sentinel","unknown":"sentinel"}"#.as_slice(),
        br#"{"name":"sentinel","email":"sentinel","passphrase":"sentinel"}"#,
        br#"{"name":"sentinel","name":"sentinel","email":"sentinel"}"#,
        br#"{"name":null,"email":"sentinel"}"#,
    ] {
        fs::write(&f.input, input).unwrap();
        for json in [true, false] {
            let mut extra = vec!["--request-id", REQUEST];
            if json {
                extra.push("--json");
            }
            let out = run(&f.args("repo", "identity-set", &extra), &f.env, &[]);
            assert_eq!(out.status, 2);
            assert!(!String::from_utf8_lossy(&out.stdout).contains("sentinel"));
            assert!(!String::from_utf8_lossy(&out.stderr).contains("sentinel"));
            if json {
                let v = cli::json(&out);
                assert_eq!(v["code"], "invalid_input");
                assert_eq!(v["request_id"], REQUEST);
            }
        }
    }
    for (request, confirmation, expected_id) in
        [("bad", "bad", None), (REQUEST, "bad", Some(REQUEST))]
    {
        let v = checked(
            &f.args(
                "repo",
                "identity-set",
                &["--json", "--request-id", request, "--confirm", confirmation],
            ),
            &f.env,
            &[],
            2,
            "invalid_id",
            None,
        );
        assert_eq!(v["request_id"], serde_json::json!(expected_id));
    }
}
#[test]
fn every_mutation_verbs_confirmed_flag_matches_prepare() {
    host_verbs_match_prepare();
    key_verbs_match_prepare();
    let remote = Fixture::new();
    super::remotes::add(&remote);
    for (verb, code, exit) in [("remove", "ok", 0), ("select", "not_confirmable", 2)] {
        let value = super::remotes::call(
            &remote,
            verb,
            super::remotes::SELECT,
            &["--prepare"],
            exit,
            code,
        );
        if verb == "remove" {
            schema::check_published("remote_removal_preview.schema.json", &value["data"]).unwrap();
        }
    }
    let f = Fixture::new();
    for (resource, verb, input, confirmed) in [
        ("repo", "identity-set", IDENTITY, true),
        ("remote", "add", REMOTE, false),
    ] {
        fs::write(&f.input, input).unwrap();
        let v = checked(
            &f.args(resource, verb, &["--json", "--prepare"]),
            &f.env,
            &[],
            if confirmed { 0 } else { 2 },
            if confirmed { "ok" } else { "not_confirmable" },
            if confirmed {
                Some("identity_preview.schema.json")
            } else {
                None
            },
        );
        assert_eq!(v["data"].is_object(), confirmed);
    }
    for verb in ["create", "enable", "remove"] {
        let f = super::repository::Fixture::new(if verb == "remove" { "enable" } else { verb });
        if verb == "remove" {
            f.onboard("enable");
        }
        let preview = f.prepare(verb);
        assert_eq!(preview["data"]["command"], format!("repo {verb}"));
        assert!(preview["data"]["confirmation_id"].is_string());
    }
}
pub fn host_verbs_match_prepare() {
    let f = Fixture::new();
    let root = f.repo.root.to_str().unwrap();
    checked(
        &[
            "remote",
            "add",
            "--repo",
            root,
            "--input",
            "-",
            "--request-id",
            REQUEST,
            "--json",
        ],
        &f.env,
        br#"{"name":"origin","url":"ssh://git@example.invalid:2222/repo.git"}"#,
        0,
        "ok",
        None,
    );
    super::remotes::select(&f);
    for verb in ["approve", "replace"] {
        let input = std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("tests/fixtures/cli_input_v1/host_{verb}.json")),
        )
        .unwrap();
        if verb == "replace" {
            let value: Value = serde_json::from_slice(&input).unwrap();
            let service = manyhands::repository::RepositoryService::open_at(&f.env.data).unwrap();
            support::credentials::pin_host(
                &service,
                "example.invalid",
                2222,
                "ssh-ed25519",
                value["old_fingerprint"].as_str().unwrap(),
            );
        }
        let args = [
            "host",
            verb,
            "--repo",
            root,
            "--input",
            "-",
            "--prepare",
            "--json",
        ];
        let preview = checked(
            &args,
            &f.env,
            &input,
            0,
            "ok",
            Some("host_preview.schema.json"),
        );
        assert_eq!(preview["data"]["command"], format!("host {verb}"));
        assert!(preview["data"]["confirmation_id"].is_string());
        let out = run(&args[..args.len() - 1], &f.env, &input);
        assert_eq!(out.status, 0);
        assert!(out.stderr.is_empty());
        assert!(
            String::from_utf8(out.stdout)
                .unwrap()
                .contains(&format!("Preview of host {verb}"))
        );
    }
}

pub fn key_verbs_match_prepare() {
    let f = super::keys::Fixture::new();
    f.call(
        "generate",
        &["--input", "-", "--prepare"],
        super::keys::GENERATE,
        2,
        "not_confirmable",
    );
    let imported = f.import();
    for verb in ["import", "select", "clear", "remove"] {
        let mut flags = vec!["--prepare"];
        if verb == "import" {
            flags.extend(["--input", "-"]);
        }
        if ["select", "remove"].contains(&verb) {
            flags.extend(["--key-id", super::keys::key_id(&imported)]);
        }
        let value = f.call(verb, &flags, super::keys::IMPORT, 2, "not_confirmable");
        assert!(value["data"].is_null());
    }
    // Current-user Windows store ignores HOME; only this actual key-file row is skipped.
    #[cfg(unix)]
    {
        let key = super::keys::generate(&f.env);
        let value = f.call(
            "delete",
            &["--key-id", &key.id.to_string(), "--prepare"],
            &[],
            0,
            "ok",
        );
        assert_eq!(value["data"]["command"], "key delete");
        assert!(value["data"]["confirmation_id"].is_string());
    }
}
#[test]
fn human_progress_lines_and_empty_json_stderr() {
    for json in [false, true] {
        let f = Fixture::new();
        let p = f.prepare();
        let mut extra = vec![
            "--request-id",
            REQUEST,
            "--confirm",
            p["data"]["confirmation_id"].as_str().unwrap(),
        ];
        if json {
            extra.push("--json");
        }
        let out = run(&f.args("repo", "identity-set", &extra), &f.env, &[]);
        assert_eq!(out.status, 0);
        if json {
            cli::json(&out);
        } else {
            let stderr = String::from_utf8(out.stderr).unwrap();
            assert!(stderr.contains("progress: accepted\n"));
            assert!(stderr.contains("progress: writing\n"));
        }
    }
}
