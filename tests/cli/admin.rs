use std::{ffi::OsStr, fs, path::Path};

use manyhands::{
    canonical::ItemId,
    repository::{LeaseKind, RepositoryService},
    results::Scope,
};
use serde_json::Value;

use crate::support::{
    self,
    cli::{self, CliEnv, CliRun},
    credentials, items, operations, schema,
};

fn run(args: &[&str], env: &CliEnv) -> CliRun {
    cli::run_cli(&args.iter().map(OsStr::new).collect::<Vec<_>>(), env)
}

fn checked(
    args: &[&str],
    env: &CliEnv,
    status: i32,
    code: &str,
    data_schema: Option<&str>,
) -> Value {
    let output = run(args, env);
    let value = cli::json(&output);
    assert_eq!(output.status, status, "{args:?}: {value}");
    assert_eq!(value["code"], code, "{args:?}");
    schema::check_published("envelope.schema.json", &value).unwrap();
    if let Some(schema) = data_schema {
        schema::check_published(schema, &value["data"]).unwrap();
    } else {
        assert!(value["data"].is_null());
    }
    value
}

fn scope(root: &Path) -> Scope {
    Scope {
        repository: Some(fs::canonicalize(root).unwrap().to_str().unwrap().to_owned()),
        ..Scope::default()
    }
}

fn assert_scope(value: &Value, expected: Scope) {
    assert_eq!(value["scope"], serde_json::to_value(expected).unwrap());
}

// Only volatile fields are substituted, never whole DTOs or scopes.
fn golden(name: &str, value: &Value, root: Option<&Path>, extra: &[(String, &str)]) {
    let root = root.map(|root| fs::canonicalize(root).unwrap());
    let mut replacements: Vec<_> = extra
        .iter()
        .map(|(value, label)| (value.as_str(), *label))
        .collect();
    if let Some(root) = &root {
        replacements.push((root.to_str().unwrap(), "<repository>"));
    }
    cli::assert_cli_golden(name, value, &replacements);
}

fn repository_read(resource: &str, verb: &str, data_schema: &str, name: &str) {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    fixture
        .repository
        .remote("origin", "ssh://git@example.invalid/team/repo.git")
        .unwrap();
    operations::insert_local(
        enabled.data_directory.path(),
        Some(operations::OPERATION_A),
        "refresh",
        "failed",
        None,
    );
    let env = CliEnv::new(enabled.data_directory.path().to_owned());
    let mut args = vec![resource, verb, "--json"];
    if (resource, verb) != ("repo", "list") {
        args.extend(["--repo", fixture.root.to_str().unwrap()]);
    }
    if (resource, verb) == ("operation", "show") {
        args.extend(["--operation-id", operations::OPERATION_A]);
    }
    let value = checked(&args, &env, 0, "ok", Some(data_schema));
    assert_eq!(value["command"], format!("{resource} {verb}"));
    // F1 read_contract: repo list is global; inspection uses DTO.root;
    // identity, remotes and all status/operation reads use repo.scope().
    assert_scope(
        &value,
        if verb == "list" && resource == "repo" {
            Scope::default()
        } else {
            scope(&fixture.root)
        },
    );
    let mut times = Vec::new();
    for pointer in [
        "/data/items/0/enabled_at",
        "/data/items/0/index/refreshed_at",
        "/data/refreshed_at",
    ] {
        if let Some(time) = value.pointer(pointer).and_then(Value::as_str) {
            times.push((time.to_owned(), "<timestamp>"));
        }
    }
    golden(name, &value, Some(&fixture.root), &times);
    args.retain(|arg| *arg != "--json");
    let human = run(&args, &env);
    assert_eq!(human.status, 0);
    assert!(!human.stdout.is_empty(), "{resource} {verb}");
    assert!(human.stderr.is_empty());
}

macro_rules! repository_case {
    ($test:ident, $resource:literal, $verb:literal, $schema:literal) => {
        #[test]
        fn $test() {
            repository_read(
                $resource,
                $verb,
                $schema,
                concat!("cli_", stringify!($test)),
            );
        }
    };
}
repository_case!(repo_list, "repo", "list", "repository_list.schema.json");
repository_case!(
    repo_inspect,
    "repo",
    "inspect",
    "repository_inspection.schema.json"
);
repository_case!(repo_identity, "repo", "identity", "identity.schema.json");
repository_case!(remote_list, "remote", "list", "remote_list.schema.json");
repository_case!(index_status, "index", "status", "index_status.schema.json");
repository_case!(poll_status, "poll", "status", "polling_status.schema.json");
repository_case!(
    operation_list,
    "operation",
    "list",
    "operation_list.schema.json"
);
repository_case!(operation_show, "operation", "show", "operation.schema.json");

fn key_read(verb: &str, data_schema: &str, name: &str) {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let keys = credentials::register_keys(&service);
    let env = CliEnv::new(data.path().to_owned());
    let id = if verb == "show" {
        keys.generated.id
    } else {
        keys.imported.id
    }
    .to_string();
    let mut args = vec!["key", verb, "--json"];
    if verb != "list" {
        args.extend(["--key-id", &id]);
    }
    let value = checked(&args, &env, 0, "ok", Some(data_schema));
    assert_eq!(value["command"], format!("key {verb}"));
    assert_scope(&value, Scope::default());
    let replacements = vec![
        (keys.imported.id.to_string(), "<imported-key-id>"),
        (keys.generated.id.to_string(), "<generated-key-id>"),
        (keys.without_public.id.to_string(), "<third-key-id>"),
        (
            keys.generated.public_key_fingerprint.clone().unwrap(),
            "<generated-fingerprint>",
        ),
        (
            keys.generated.private_key_path.to_str().unwrap().to_owned(),
            "<generated-private-key>",
        ),
        (
            keys.generated
                .public_key_path
                .as_ref()
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned(),
            "<generated-public-key>",
        ),
        (
            keys.imported
                .private_key_path
                .parent()
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned(),
            "<key-files>",
        ),
    ];
    golden(name, &value, None, &replacements);
    args.retain(|arg| *arg != "--json");
    let human = run(&args, &env);
    assert_eq!(human.status, 0);
    assert!(human.stderr.is_empty());
    assert!(!human.stdout.is_empty());
    if verb == "public" {
        assert_eq!(
            human.stdout,
            format!("{}\n", value["data"]["public_key"].as_str().unwrap()).as_bytes()
        );
    }
    for sentinel in credentials::SECRET_SENTINELS {
        assert!(!value.to_string().contains(sentinel));
        assert!(!String::from_utf8_lossy(&human.stdout).contains(sentinel));
    }
}

#[test]
fn key_list() {
    key_read("list", "key_list.schema.json", "cli_key_list");
}
#[test]
fn key_show() {
    key_read("show", "key.schema.json", "cli_key_show");
}
#[test]
fn key_public() {
    key_read("public", "public_key.schema.json", "cli_key_public");
}

fn host_read(verb: &str, data_schema: &str, name: &str) {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    credentials::pin_host(
        &service,
        "::1",
        2222,
        "ssh-ed25519",
        credentials::PUBLIC_FIXTURE_FINGERPRINT,
    );
    credentials::pin_host(
        &service,
        "git.example.invalid",
        22,
        "ssh-rsa",
        credentials::OTHER_FINGERPRINT,
    );
    let env = CliEnv::new(data.path().to_owned());
    let mut args = vec!["host", verb, "--json"];
    if verb == "inspect" {
        args.extend(["--authority", "[::1]:2222"]);
    }
    let value = checked(&args, &env, 0, "ok", Some(data_schema));
    assert_eq!(value["command"], format!("host {verb}"));
    assert_scope(&value, Scope::default());
    golden(name, &value, None, &[]);
    args.retain(|arg| *arg != "--json");
    let human = run(&args, &env);
    assert_eq!(human.status, 0);
    assert!(human.stderr.is_empty());
    assert!(
        String::from_utf8(human.stdout)
            .unwrap()
            .contains("[::1]:2222")
    );
    if verb == "inspect" {
        let normalized = checked(
            &[
                "host",
                "inspect",
                "--authority",
                "GIT.EXAMPLE.INVALID",
                "--json",
            ],
            &env,
            0,
            "ok",
            Some(data_schema),
        );
        assert_eq!(normalized["data"]["host"], "git.example.invalid");
        assert_eq!(normalized["data"]["port"], 22);
    }
}
#[test]
fn host_list() {
    host_read("list", "host_pin_list.schema.json", "cli_host_list");
}
#[test]
fn host_inspect() {
    host_read("inspect", "host_pin.schema.json", "cli_host_inspect");
}

#[test]
fn id_new() {
    let data = tempfile::tempdir().unwrap();
    let env = CliEnv::new(data.path().to_owned());
    let value = checked(
        &["id", "new", "--json"],
        &env,
        0,
        "ok",
        Some("new_id.schema.json"),
    );
    assert_scope(&value, Scope::default());
    let id = value["data"]["id"].as_str().unwrap();
    assert!(id.parse::<ItemId>().is_ok());
    golden("cli_id_new", &value, None, &[(id.to_owned(), "<item-id>")]);
    let human = run(&["id", "new"], &env);
    assert_eq!(human.status, 0);
    assert!(human.stderr.is_empty());
    let text = String::from_utf8(human.stdout).unwrap();
    assert_eq!(text.len(), 27);
    assert!(text.ends_with('\n'));
    assert!(text.trim_end().parse::<ItemId>().is_ok());
}

#[test]
fn repository_failures_and_resolution_precedence() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let env = CliEnv::new(enabled.data_directory.path().to_owned());
    let empty = tempfile::tempdir().unwrap();
    let missing = empty.path().join("missing");
    let nested = fixture.root.join("docs");
    fs::create_dir_all(&nested).unwrap();
    let bare = support::bare_repository();
    let unregistered = support::born_repository();
    for (path, code, status) in [
        (missing.as_path(), "invalid_path", 2),
        (empty.path(), "not_repository", 2),
        (nested.as_path(), "not_repository_root", 2),
        (bare.root.as_path(), "bare_repository", 2),
        (unregistered.root.as_path(), "repository_not_registered", 3),
    ] {
        let value = checked(
            &["remote", "list", "--repo", path.to_str().unwrap(), "--json"],
            &env,
            status,
            code,
            None,
        );
        // F1 keeps the canonical bare/unregistered root in failure scope.
        let root = match code {
            "repository_not_registered" => &unregistered.root,
            "bare_repository" => &bare.root,
            _ => &fixture.root,
        };
        golden(&format!("cli_failure_{code}"), &value, Some(root), &[]);
    }
    let value = checked(
        &[
            "operation",
            "show",
            "--repo",
            unregistered.root.to_str().unwrap(),
            "--operation-id",
            "bad",
            "--json",
        ],
        &env,
        3,
        "repository_not_registered",
        None,
    );
    golden(
        "cli_failure_repository_before_id",
        &value,
        Some(&unregistered.root),
        &[],
    );
    let human = run(
        &["remote", "list", "--repo", nested.to_str().unwrap()],
        &env,
    );
    assert_eq!(human.status, 2);
    assert!(human.stdout.is_empty());
    let stderr = String::from_utf8(human.stderr).unwrap();
    assert!(stderr.starts_with("error: "));
    assert!(stderr.contains("\ncode: not_repository_root\n"));
    assert!(stderr.contains(&format!(
        "next: repo.inspect root={}\n",
        fs::canonicalize(&fixture.root).unwrap().display()
    )));
}

#[test]
fn selector_failures_preserve_global_and_repository_scopes() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let env = CliEnv::new(enabled.data_directory.path().to_owned());
    let lower = operations::OPERATION_A.to_ascii_lowercase();
    let root = fixture.root.to_str().unwrap();
    for (args, code, name, expected_scope) in [
        (
            vec!["key", "show", "--key-id", &lower, "--json"],
            "invalid_id",
            "cli_failure_invalid_key_id",
            Scope::default(),
        ),
        (
            vec![
                "operation",
                "show",
                "--repo",
                root,
                "--operation-id",
                &lower,
                "--json",
            ],
            "invalid_id",
            "cli_failure_invalid_operation_id",
            scope(&fixture.root),
        ),
        (
            vec![
                "key",
                "show",
                "--key-id",
                operations::OPERATION_ABSENT,
                "--json",
            ],
            "key_not_found",
            "cli_failure_key_not_found",
            Scope::default(),
        ),
        (
            vec!["host", "inspect", "--authority", "absent.invalid", "--json"],
            "authority_not_found",
            "cli_failure_authority_not_found",
            Scope::default(),
        ),
        (
            vec![
                "operation",
                "show",
                "--repo",
                root,
                "--operation-id",
                operations::OPERATION_ABSENT,
                "--json",
            ],
            "operation_not_found",
            "cli_failure_operation_not_found",
            scope(&fixture.root),
        ),
    ] {
        let value = checked(&args, &env, 2, code, None);
        assert_scope(&value, expected_scope);
        golden(name, &value, Some(&fixture.root), &[]);
    }
}

#[test]
fn public_key_unavailable_is_blocked() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let keys = credentials::register_keys(&service);
    let env = CliEnv::new(data.path().to_owned());
    let value = checked(
        &[
            "key",
            "public",
            "--key-id",
            &keys.without_public.id.to_string(),
            "--json",
        ],
        &env,
        3,
        "public_key_unavailable",
        None,
    );
    assert_scope(&value, Scope::default());
    golden("cli_failure_public_key_unavailable", &value, None, &[]);
}

#[test]
#[cfg(unix)]
fn unreadable_repository_configuration_is_inaccessible() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let env = CliEnv::new(enabled.data_directory.path().to_owned());
    let config = fixture.root.join(".manyhands/config.toml");
    let permissions = fs::metadata(&config).unwrap().permissions();
    fs::set_permissions(&config, fs::Permissions::from_mode(0o000)).unwrap();
    if fs::File::open(&config).is_ok() {
        // Root (or an equivalent capability) bypasses these permissions.
        fs::set_permissions(&config, permissions).unwrap();
        return;
    }
    let output = run(
        &[
            "remote",
            "list",
            "--repo",
            fixture.root.to_str().unwrap(),
            "--json",
        ],
        &env,
    );
    fs::set_permissions(&config, permissions).unwrap();
    let value = cli::json(&output);
    assert_eq!(output.status, 3, "{value}");
    assert_eq!(value["code"], "repository_inaccessible");
    schema::check_published("envelope.schema.json", &value).unwrap();
    golden(
        "cli_failure_repository_inaccessible",
        &value,
        Some(&fixture.root),
        &[],
    );
}

#[test]
fn degraded_index_fails_during_resolution_not_as_a_status_dto() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let (data, _service) = items::degraded_service(enabled);
    let env = CliEnv::new(data.path().to_owned());
    let value = checked(
        &[
            "index",
            "status",
            "--repo",
            fixture.root.to_str().unwrap(),
            "--json",
        ],
        &env,
        3,
        "index_unavailable",
        None,
    );
    golden(
        "cli_failure_index_unavailable",
        &value,
        Some(&fixture.root),
        &[],
    );
}

#[test]
fn exclusive_index_lease_is_transient() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let env = CliEnv::new(enabled.data_directory.path().to_owned());
    let holder = support::hold_lease_in_child_for_test(
        &fixture.root,
        enabled.data_directory.path(),
        LeaseKind::CacheWrite,
        "index_lease_child",
    );
    let output = run(
        &[
            "index",
            "status",
            "--repo",
            fixture.root.to_str().unwrap(),
            "--json",
        ],
        &env,
    );
    holder.release();
    // Service opening and resolution both lock: scope is deliberately unpinned.
    assert_eq!(output.status, 5);
    assert_eq!(cli::json(&output)["code"], "busy");
}

#[test]
fn inspect_works_without_registration() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let env = CliEnv::new(data.path().to_owned());
    let value = checked(
        &[
            "repo",
            "inspect",
            "--repo",
            fixture.root.to_str().unwrap(),
            "--json",
        ],
        &env,
        0,
        "ok",
        Some("repository_inspection.schema.json"),
    );
    assert_scope(&value, scope(&fixture.root));
    assert_eq!(value["data"]["registered"], false);
}

#[test]
fn repository_spellings_and_linked_worktree_select_the_owner() {
    let parent = tempfile::tempdir().unwrap();
    let special = parent.path().join("with space é");
    fs::create_dir(&special).unwrap();
    let fixture = cli::born_repository_in(&special, "repo");
    let enabled = support::enabled_repository(&fixture);
    fs::create_dir_all(fixture.root.join("docs")).unwrap();
    items::create_document_context(
        &enabled.service,
        &fixture.root,
        items::DOCUMENT_A,
        "docs/guide.md",
    );
    let linked = items::context_worktree(&fixture.root, items::DOCUMENT_A);
    for (path, cwd) in [
        (format!("{}/", fixture.root.display()), fixture.root.clone()),
        (".".to_owned(), fixture.root.clone()),
        ("..".to_owned(), fixture.root.join("docs")),
        (
            fixture.root.to_str().unwrap().to_owned(),
            parent.path().to_owned(),
        ),
        (
            linked.to_str().unwrap().to_owned(),
            parent.path().to_owned(),
        ),
    ] {
        let env = CliEnv::new(enabled.data_directory.path().to_owned()).in_dir(cwd);
        let value = checked(
            &["repo", "identity", "--repo", &path, "--json"],
            &env,
            0,
            "ok",
            Some("identity.schema.json"),
        );
        assert_scope(&value, scope(&fixture.root));
    }
}

#[test]
fn repository_list_has_one_line_per_registration_and_empty_lists_are_silent() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let second = support::born_repository();
    enabled
        .service
        .enable(support::enable_request_with_operation_id(
            &second.root,
            support::operation_id(),
        ))
        .unwrap();
    let env = CliEnv::new(enabled.data_directory.path().to_owned());
    let output = run(&["repo", "list"], &env);
    assert_eq!(output.status, 0);
    assert!(output.stderr.is_empty());
    let text = String::from_utf8(output.stdout).unwrap();
    assert_eq!(text.lines().count(), 2);
    for root in [&fixture.root, &second.root] {
        assert!(text.contains(fs::canonicalize(root).unwrap().to_str().unwrap()));
    }
    let empty = tempfile::tempdir().unwrap();
    let env = CliEnv::new(empty.path().to_owned());
    for args in [["repo", "list"], ["key", "list"], ["host", "list"]] {
        let output = run(&args, &env);
        assert_eq!(output.status, 0);
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());
    }
}
