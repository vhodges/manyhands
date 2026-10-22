use super::mutation::{REQUEST, checked};
use crate::support::{
    self,
    cli::{self, CliEnv},
};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
};

/// libgit2 can still inherit its system search path in an isolated child.
/// Read only: never change the machine's configuration to satisfy a test.
pub fn require_no_system_identity() {
    match git2::Config::find_system() {
        Ok(path) => {
            let config = git2::Config::open(&path).expect("read system Git configuration");
            for key in ["user.name", "user.email"] {
                match config.get_entry(key) {
                    Ok(_) => panic!(
                        "this test needs a system Git configuration without user.name or user.email"
                    ),
                    Err(error) if error.code() == git2::ErrorCode::NotFound => {}
                    Err(error) => panic!("cannot probe system Git identity: {error}"),
                }
            }
        }
        Err(error) if error.code() == git2::ErrorCode::NotFound => {}
        Err(error) => panic!("cannot locate system Git configuration: {error}"),
    }
}

pub struct Fixture {
    pub env: CliEnv,
    pub root: PathBuf,
    pub input: PathBuf,
    _existing: Option<support::TestRepository>,
    _temp: tempfile::TempDir,
}

impl Fixture {
    pub fn new(verb: &str) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let existing =
            (verb == "enable").then(|| cli::born_repository_in(temp.path(), "repository"));
        let root = existing
            .as_ref()
            .map_or_else(|| temp.path().join("repository"), |r| r.root.clone());
        let env = CliEnv::new(temp.path().join("data"));
        fs::write(
            env.home.path().join(".gitconfig"),
            "[core]\n autocrlf = false\n",
        )
        .unwrap();
        let input = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("tests/fixtures/cli_input_v1/repo_{verb}.json"));
        Self {
            env,
            root,
            input,
            _existing: existing,
            _temp: temp,
        }
    }
    pub fn set_input(&mut self, bytes: &[u8]) {
        self.input = self.env.home.path().join("input.json");
        fs::write(&self.input, bytes).unwrap();
    }
    pub fn args<'a>(&'a self, verb: &'a str, extra: &[&'a str]) -> Vec<&'a str> {
        let mut args = vec!["repo", verb, "--repo", self.root.to_str().unwrap()];
        if verb != "remove" {
            args.extend(["--input", self.input.to_str().unwrap()]);
        }
        args.extend_from_slice(extra);
        args
    }
    pub fn prepare(&self, verb: &str) -> Value {
        checked(
            &self.args(verb, &["--json", "--prepare"]),
            &self.env,
            &[],
            0,
            "ok",
            Some(if verb == "remove" {
                "repository_removal_preview.schema.json"
            } else {
                "preview.schema.json"
            }),
        )
    }
    pub fn execute(&self, verb: &str, preview: &Value) -> Value {
        checked(
            &self.args(
                verb,
                &[
                    "--json",
                    "--request-id",
                    REQUEST,
                    "--confirm",
                    preview["data"]["confirmation_id"].as_str().unwrap(),
                ],
            ),
            &self.env,
            &[],
            0,
            "ok",
            None,
        )
    }
    pub fn onboard(&self, verb: &str) -> Value {
        self.execute(verb, &self.prepare(verb))
    }
    pub fn listed(&self) -> Value {
        checked(
            &["repo", "list", "--json"],
            &self.env,
            &[],
            0,
            "ok",
            Some("repository_list.schema.json"),
        )
    }
    pub fn global(&self) -> Vec<u8> {
        fs::read(self.env.home.path().join(".gitconfig")).unwrap()
    }
    pub fn local(&self) -> git2::Config {
        git2::Config::open(&self.root.join(".git/config")).unwrap()
    }
    pub fn remove_local_identity(&self) {
        let mut config = self.local();
        config.remove("user.name").unwrap();
        config.remove("user.email").unwrap();
    }
}

pub fn golden(name: &str, value: &Value) {
    let mut replacements = Vec::new();
    for (pointer, placeholder) in [
        ("/scope/repository", "<repository>"),
        ("/request_id", "<request-id>"),
        ("/operation_id", "<operation-id>"),
        ("/effects/commit_oid", "<commit-oid>"),
        ("/data/confirmation_id", "<confirmation-id>"),
        ("/data/expires_at", "<expires-at>"),
        ("/data/details/enabled_at", "<enabled-at>"),
    ] {
        if let Some(text) = value.pointer(pointer).and_then(Value::as_str) {
            replacements.push((text, placeholder));
        }
    }
    cli::assert_cli_golden(name, value, &replacements);
}

fn assert_onboarded(f: &Fixture) {
    let root = fs::canonicalize(&f.root).unwrap();
    assert_eq!(
        f.listed()["data"]["items"][0]["root"],
        root.to_str().unwrap()
    );
    let repo = git2::Repository::open(&f.root).unwrap();
    let entry = repo
        .index()
        .unwrap()
        .get_path(Path::new(".manyhands/config.toml"), 0)
        .unwrap();
    let committed = repo
        .head()
        .unwrap()
        .peel_to_tree()
        .unwrap()
        .get_path(Path::new(".manyhands/config.toml"))
        .unwrap();
    assert_eq!(entry.id, committed.id());
    assert_eq!(entry.mode, committed.filemode() as u32);
    assert_eq!(
        repo.status_file(Path::new(".manyhands/config.toml"))
            .unwrap(),
        git2::Status::CURRENT
    );
}

#[test]
fn create_in_an_empty_directory() {
    let f = Fixture::new("create");
    fs::create_dir(&f.root).unwrap();
    let preview = f.prepare("create");
    golden("repo_create_empty_prepare", &preview);
    golden("repo_create", &f.execute("create", &preview));
    assert_onboarded(&f);
}
#[test]
fn create_in_an_absent_directory() {
    let f = Fixture::new("create");
    let preview = f.prepare("create");
    assert!(!f.root.exists());
    golden("repo_create_prepare", &preview);
    golden("repo_create", &f.execute("create", &preview));
    assert_onboarded(&f);
}
#[test]
fn create_in_an_occupied_directory_is_occupied_path() {
    let f = Fixture::new("create");
    fs::create_dir(&f.root).unwrap();
    fs::write(f.root.join("keep"), b"unchanged").unwrap();
    checked(
        &f.args("create", &["--json", "--prepare"]),
        &f.env,
        &[],
        2,
        "occupied_path",
        None,
    );
    assert_eq!(fs::read(f.root.join("keep")).unwrap(), b"unchanged");
    assert!(!f.root.join(".git").exists());
}
#[test]
fn enable_an_existing_repository() {
    let f = Fixture::new("enable");
    let before = support::repository_and_worktree_snapshot_at(&f.root);
    let preview = f.prepare("enable");
    assert_eq!(
        support::repository_and_worktree_snapshot_at(&f.root),
        before
    );
    golden("repo_enable_prepare", &preview);
    golden("repo_enable", &f.execute("enable", &preview));
    assert_onboarded(&f);
}
#[test]
fn enable_an_enabled_but_unregistered_repository_registers_it() {
    let mut f = Fixture::new("enable");
    f.onboard("enable");
    let before = support::repository_and_worktree_snapshot_at(&f.root);
    f.env = CliEnv::new(f.env.data.with_file_name("second-data"));
    assert!(f.listed()["data"]["items"].as_array().unwrap().is_empty());
    let preview = f.prepare("enable");
    assert_eq!(
        preview["data"]["effects"],
        serde_json::json!([
            {"effect": "worktree_exclusion_written"},
            {"effect": "repository_registered"}
        ])
    );
    assert_eq!(
        preview["data"]["details"]["identity"]["written_locally"],
        false
    );
    f.execute("enable", &preview);
    assert_onboarded(&f);
    assert_eq!(
        support::repository_and_worktree_snapshot_at(&f.root),
        before
    );
}
#[test]
fn remote_select_follows_create_and_enable_without_worktree_not_clean() {
    for verb in ["create", "enable"] {
        let f = Fixture::new(verb);
        f.onboard(verb);
        assert_onboarded(&f);
        checked(
            &[
                "remote",
                "add",
                "--repo",
                f.root.to_str().unwrap(),
                "--input",
                "-",
                "--request-id",
                "01M4CC0VMTTNKVYS6XWQG4CK1P",
                "--json",
            ],
            &f.env,
            super::mutation::REMOTE,
            0,
            "ok",
            Some("remote_mutation.schema.json"),
        );
        checked(
            &[
                "remote",
                "select",
                "--repo",
                f.root.to_str().unwrap(),
                "--input",
                "-",
                "--request-id",
                "01M4CC0VMTTNKVYS6XWQG4CK1Q",
                "--json",
            ],
            &f.env,
            super::remotes::SELECT,
            0,
            "ok",
            Some("remote_mutation.schema.json"),
        );
    }
}
#[test]
fn the_prefix_is_previewed_and_written() {
    for verb in ["create", "enable"] {
        let f = Fixture::new(verb);
        let preview = f.prepare(verb);
        assert_eq!(preview["data"]["details"]["slug_prefix"], "mh");
        f.execute(verb, &preview);
        let config: toml::Value =
            toml::from_str(&fs::read_to_string(f.root.join(".manyhands/config.toml")).unwrap())
                .unwrap();
        assert_eq!(config["ticket_slug_prefix"].as_str(), Some("mh"));
    }
}
#[test]
fn identity_required_then_resubmitted_with_identity() {
    require_no_system_identity();
    for verb in ["create", "enable"] {
        let mut f = Fixture::new(verb);
        if verb == "enable" {
            f.remove_local_identity();
        }
        let example = f.input.clone();
        f.set_input(br#"{"primary_branch":"main"}"#);
        let global = f.global();
        let refusal = checked(
            &f.args(verb, &["--json", "--prepare"]),
            &f.env,
            &[],
            3,
            "identity_required",
            None,
        );
        golden(&format!("repo_{verb}_identity_required"), &refusal);
        f.input = example;
        f.onboard(verb);
        assert_eq!(
            f.local().get_string("user.name").unwrap(),
            "Repository Person"
        );
        assert_eq!(
            f.local().get_string("user.email").unwrap(),
            "repository@example.invalid"
        );
        assert_eq!(f.global(), global);
    }
}
#[test]
fn a_global_identity_is_previewed_with_its_source_and_enable_writes_none() {
    let mut f = Fixture::new("enable");
    f.remove_local_identity();
    fs::write(
        f.env.home.path().join(".gitconfig"),
        "[user]\nname = Global Person\nemail = global@example.invalid\n",
    )
    .unwrap();
    f.set_input(br#"{"primary_branch":"main"}"#);
    let global = f.global();
    let local = fs::read(f.root.join(".git/config")).unwrap();
    let preview = f.prepare("enable");
    let identity = &preview["data"]["details"]["identity"];
    assert_eq!(identity["source"], "global");
    assert_eq!(identity["name"], "Global Person");
    assert_eq!(identity["email"], "global@example.invalid");
    assert_eq!(identity["written_locally"], false);
    f.execute("enable", &preview);
    assert!(f.local().get_entry("user.name").is_err());
    assert!(f.local().get_entry("user.email").is_err());
    assert_eq!(fs::read(f.root.join(".git/config")).unwrap(), local);
    assert_eq!(f.global(), global);
}
#[test]
fn remove_an_accessible_registration() {
    let f = Fixture::new("enable");
    f.onboard("enable");
    let before = support::repository_and_worktree_snapshot_at(&f.root);
    let preview = f.prepare("remove");
    golden("repo_remove_prepare", &preview);
    // A different command is a different intent.
    let result = checked(
        &f.args(
            "remove",
            &[
                "--json",
                "--request-id",
                "01M4CC0VMTTNKVYS6XWQG4CK1P",
                "--confirm",
                preview["data"]["confirmation_id"].as_str().unwrap(),
            ],
        ),
        &f.env,
        &[],
        0,
        "ok",
        None,
    );
    golden("repo_remove", &result);
    assert!(f.listed()["data"]["items"].as_array().unwrap().is_empty());
    assert_eq!(
        support::repository_and_worktree_snapshot_at(&f.root),
        before
    );
}
#[test]
fn remove_the_exact_stored_root_of_a_deleted_directory() {
    let mut f = Fixture::new("enable");
    f.onboard("enable");
    f.root = PathBuf::from(f.listed()["data"]["items"][0]["root"].as_str().unwrap());
    fs::remove_dir_all(&f.root).unwrap();
    let preview = f.prepare("remove");
    checked(
        &f.args(
            "remove",
            &[
                "--json",
                "--request-id",
                "01M4CC0VMTTNKVYS6XWQG4CK1P",
                "--confirm",
                preview["data"]["confirmation_id"].as_str().unwrap(),
            ],
        ),
        &f.env,
        &[],
        0,
        "ok",
        None,
    );
    assert!(f.listed()["data"]["items"].as_array().unwrap().is_empty());
    assert!(!f.root.exists());
}

#[test]
fn a_deleted_root_spelled_with_a_trailing_separator_is_invalid_path() {
    let mut f = Fixture::new("enable");
    f.onboard("enable");
    let exact = PathBuf::from(f.listed()["data"]["items"][0]["root"].as_str().unwrap());
    fs::remove_dir_all(&exact).unwrap();
    f.root = PathBuf::from(format!(
        "{}{}",
        exact.to_str().unwrap(),
        std::path::MAIN_SEPARATOR
    ));
    checked(
        &f.args("remove", &["--json", "--prepare"]),
        &f.env,
        &[],
        2,
        "invalid_path",
        None,
    );
    assert_eq!(
        f.listed()["data"]["items"][0]["root"],
        exact.to_str().unwrap()
    );
    f.root = exact;
    let preview = f.prepare("remove");
    checked(
        &f.args(
            "remove",
            &[
                "--json",
                "--request-id",
                "01M4CC0VMTTNKVYS6XWQG4CK1P",
                "--confirm",
                preview["data"]["confirmation_id"].as_str().unwrap(),
            ],
        ),
        &f.env,
        &[],
        0,
        "ok",
        None,
    );
    assert!(f.listed()["data"]["items"].as_array().unwrap().is_empty());
}

#[test]
fn repository_input_shapes_are_closed_and_diagnostics_are_value_free() {
    for verb in ["create", "enable"] {
        let mut f = Fixture::new(verb);
        for input in [
            r#"{}"#,
            r#"{"primary_branch":null}"#,
            r#"{"primary_branch":"sentinel","identity":null}"#,
            r#"{"primary_branch":"sentinel","identity":"sentinel"}"#,
            r#"{"primary_branch":"sentinel","identity":{}}"#,
            r#"{"primary_branch":"sentinel","identity":{"name":"sentinel"}}"#,
            r#"{"primary_branch":"sentinel","identity":{"name":null,"email":"sentinel"}}"#,
            r#"{"primary_branch":"sentinel","identity":{"name":"sentinel","email":17}}"#,
            r#"{"primary_branch":"sentinel","identity":{"name":"sentinel","email":"sentinel","extra":"sentinel"}}"#,
            r#"{"primary_branch":"sentinel","identity":{"name":"sentinel","name":"sentinel","email":"sentinel"}}"#,
            r#"{"primary_branch":"sentinel","ticket_slug_prefix":null}"#,
            r#"{"primary_branch":"sentinel","ticket_slug_prefix":[]}"#,
            r#"{"primary_branch":"sentinel","passphrase":"sentinel"}"#,
            r#"{"primary_branch":"sentinel","slug_prefix":"sentinel"}"#,
        ] {
            f.set_input(input.as_bytes());
            for json in [false, true] {
                let extra = if json {
                    vec!["--json", "--request-id", REQUEST]
                } else {
                    vec!["--request-id", REQUEST]
                };
                let out = super::mutation::run(&f.args(verb, &extra), &f.env, &[]);
                assert_eq!(out.status, 2);
                assert!(!String::from_utf8_lossy(&out.stdout).contains("sentinel"));
                assert!(!String::from_utf8_lossy(&out.stderr).contains("sentinel"));
                if json {
                    let value = cli::json(&out);
                    assert_eq!(value["code"], "invalid_input");
                    assert_eq!(value["request_id"], REQUEST);
                    assert!(value["operation_id"].is_null() && value["data"].is_null());
                    assert!(
                        value["scope"]
                            .as_object()
                            .unwrap()
                            .values()
                            .all(Value::is_null)
                    );
                } else {
                    assert!(out.stdout.is_empty());
                }
            }
        }
    }
}

#[test]
fn repository_grammar_requires_selectors_and_refuses_remove_input() {
    let f = Fixture::new("create");
    for verb in ["create", "enable", "remove"] {
        checked(
            &["repo", verb, "--json", "--prepare"],
            &f.env,
            &[],
            2,
            "usage_error",
            None,
        );
        if verb == "remove" {
            checked(
                &[
                    "repo",
                    verb,
                    "--repo",
                    f.root.to_str().unwrap(),
                    "--input",
                    "-",
                    "--json",
                    "--prepare",
                ],
                &f.env,
                &[],
                2,
                "usage_error",
                None,
            );
        } else {
            checked(
                &[
                    "repo",
                    verb,
                    "--repo",
                    f.root.to_str().unwrap(),
                    "--json",
                    "--prepare",
                ],
                &f.env,
                &[],
                2,
                "usage_error",
                None,
            );
        }
        for extra in [
            vec!["--json"],
            vec!["--json", "--prepare", "--request-id", REQUEST],
            vec!["--json", "--prepare", "--confirm", REQUEST],
        ] {
            let value = checked(&f.args(verb, &extra), &f.env, &[], 2, "usage_error", None);
            assert!(value["request_id"].is_null());
        }
    }
    assert!(!f.root.exists());
    assert!(!f.env.data.exists());
}
