use super::{
    mutation::{REQUEST, checked, run},
    remotes::{LAST, NEXT},
};
use crate::support::{
    cli::{self, CliEnv},
    credentials, schema,
};
use serde_json::Value;
use std::{fs, path::PathBuf};

pub const IMPORT: &[u8] = include_bytes!("../fixtures/cli_input_v1/key_import.json");
pub const GENERATE: &[u8] = include_bytes!("../fixtures/cli_input_v1/key_generate.json");
pub const PROTECTED: &[u8] = br#"{"label":"Protected key","protection":"passphrase"}"#;

pub fn no_generated_key_or_request(env: &CliEnv, request: &str) {
    assert!(!env.home.path().join(".ssh/manyhands").exists());
    checked(
        &["request", "show", "--json", "--request-id", request],
        env,
        &[],
        2,
        "request_not_found",
        None,
    );
}

#[test]
fn a_protected_generation_without_a_terminal_is_unlock_required() {
    let f = Fixture::new();
    let value = f.call(
        "generate",
        &["--input", "-", "--request-id", REQUEST],
        PROTECTED,
        3,
        "unlock_required",
    );
    assert_eq!(value["request_id"], REQUEST);
    assert!(value["operation_id"].is_null() && value["data"].is_null());
    golden("key_generate_unlock_required", &value, &f.env);
    no_generated_key_or_request(&f.env, REQUEST);
    let out = run(
        &["key", "generate", "--input", "-", "--request-id", REQUEST],
        &f.env,
        PROTECTED,
    );
    assert_eq!(out.status, 3);
    assert!(out.stdout.is_empty());
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .contains("A passphrase can be entered only at a terminal")
    );
}

#[test]
fn prepare_of_key_generate_is_not_confirmable_and_asks_nothing() {
    let f = Fixture::new();
    for input in [GENERATE, PROTECTED] {
        let value = f.call(
            "generate",
            &["--input", "-", "--prepare"],
            input,
            2,
            "not_confirmable",
        );
        assert!(value["request_id"].is_null());
    }
    no_generated_key_or_request(&f.env, REQUEST);
}

#[cfg(unix)]
#[test]
fn generate_an_unprotected_key() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::new();
    let value = f.call(
        "generate",
        &["--input", "-", "--request-id", REQUEST],
        GENERATE,
        0,
        "ok",
    );
    golden("key_generate", &value, &f.env);
    let directory = f.env.home.path().join(".ssh/manyhands");
    assert_eq!(
        fs::metadata(&directory).unwrap().permissions().mode() & 0o777,
        0o700
    );
    let private = directory.join(key_id(&value));
    let public = directory.join(format!("{}.pub", key_id(&value)));
    // The existing store contract is 0700 directories, 0600 private files,
    // and 0644 public companions (keys/storage/unix.rs).
    for (path, mode) in [(&private, 0o600), (&public, 0o644)] {
        assert_eq!(
            fs::metadata(path).unwrap().permissions().mode() & 0o777,
            mode
        );
    }
    let key = ssh_key::PrivateKey::from_openssh(fs::read(&private).unwrap()).unwrap();
    assert!(!key.is_encrypted());
    assert_eq!(
        ssh_key::PublicKey::from_openssh(&fs::read_to_string(public).unwrap()).unwrap(),
        *key.public_key()
    );
    let before = credentials::key_file_states(&[private]);
    assert_eq!(
        f.call(
            "generate",
            &["--input", "-", "--request-id", REQUEST],
            GENERATE,
            0,
            "ok"
        ),
        value
    );
    assert_eq!(
        credentials::key_file_states(&[directory.join(key_id(&value))]),
        before
    );
}

#[test]
fn generate_checks_ids_then_closed_input_and_refuses_repository_selection() {
    let f = Fixture::new();
    for (request, confirm, id) in [
        ("bad", "bad", Value::Null),
        (REQUEST, "bad", Value::String(REQUEST.into())),
    ] {
        let value = f.call(
            "generate",
            &[
                "--input",
                "missing",
                "--request-id",
                request,
                "--confirm",
                confirm,
            ],
            &[],
            2,
            "invalid_id",
        );
        assert_eq!(value["request_id"], id);
    }
    for input in [
        br#"{}"#.as_slice(),
        br#"{"label":null,"protection":"none"}"#,
        br#"{"label":"x","protection":null}"#,
        br#"{"label":"x","protection":"none","passphrase":"not-allowed"}"#,
        br#"{"label":"x","label":"y","protection":"none"}"#,
    ] {
        f.call(
            "generate",
            &["--input", "-", "--request-id", REQUEST],
            input,
            2,
            "invalid_input",
        );
    }
    f.call("generate", &["--prepare"], &[], 2, "usage_error");
    f.call(
        "generate",
        &["--input", "-", "--prepare", "--repo", "."],
        GENERATE,
        2,
        "usage_error",
    );
    no_generated_key_or_request(&f.env, REQUEST);
}
pub struct Fixture {
    pub env: CliEnv,
    _data: tempfile::TempDir,
}
impl Fixture {
    pub fn new() -> Self {
        let data = tempfile::tempdir().unwrap();
        let env = CliEnv::new(data.path().to_owned());
        // Inert registry fixtures, never authentication credentials.
        fs::write(
            env.home.path().join("imported"),
            credentials::private_fixture_bytes(),
        )
        .unwrap();
        fs::write(
            env.home.path().join("imported.pub"),
            credentials::public_fixture_text(),
        )
        .unwrap();
        Self { env, _data: data }
    }
    pub fn files(&self) -> Vec<PathBuf> {
        ["imported", "imported.pub"]
            .map(|p| self.env.home.path().join(p))
            .to_vec()
    }
    pub fn import(&self) -> Value {
        self.call(
            "import",
            &["--input", "-", "--request-id", REQUEST],
            IMPORT,
            0,
            "ok",
        )
    }
    pub fn call(&self, verb: &str, flags: &[&str], input: &[u8], exit: i32, code: &str) -> Value {
        call(&self.env, verb, flags, input, exit, code)
    }
}
pub fn call(
    env: &CliEnv,
    verb: &str,
    flags: &[&str],
    input: &[u8],
    exit: i32,
    code: &str,
) -> Value {
    let mut args = vec!["key", verb, "--json"];
    args.extend_from_slice(flags);
    let value = checked(&args, env, input, exit, code, None);
    if !value["data"].is_null() && !["list", "show"].contains(&verb) {
        schema::check_published(
            if flags.contains(&"--prepare") && code == "ok" {
                "key_deletion_preview.schema.json"
            } else {
                "key_mutation.schema.json"
            },
            &value["data"],
        )
        .unwrap();
    }
    value
}
pub fn key_id(value: &Value) -> &str {
    value["data"]["id"].as_str().unwrap()
}
fn golden(name: &str, value: &Value, env: &CliEnv) {
    let home = fs::canonicalize(env.home.path()).unwrap();
    let mut replacements = vec![(home.to_str().unwrap(), "<home>")];
    for (pointer, label) in [
        ("/request_id", "<request-id>"),
        ("/operation_id", "<operation-id>"),
        ("/data/id", "<key-id>"),
        ("/data/details/id", "<key-id>"),
        ("/data/fingerprint", "<fingerprint>"),
        ("/data/details/fingerprint", "<fingerprint>"),
        ("/data/confirmation_id", "<confirmation-id>"),
        ("/data/expires_at", "<expires-at>"),
        (
            "/data/details/private_key_path",
            "<home>/.ssh/manyhands/<key-id>",
        ),
        (
            "/data/details/public_key_path",
            "<home>/.ssh/manyhands/<key-id>.pub",
        ),
    ] {
        if let Some(s) = value.pointer(pointer).and_then(Value::as_str)
            && s != credentials::PUBLIC_FIXTURE_FINGERPRINT
        {
            replacements.push((s, label));
        }
    }
    cli::assert_cli_golden(name, value, &replacements);
}

#[test]
fn import_a_key_by_relative_paths() {
    let f = Fixture::new();
    let imported = f.import();
    golden("key_import", &imported, &f.env);
    assert_eq!(imported["data"]["label"], "Imported key");
    assert_eq!(
        imported["data"]["fingerprint"],
        credentials::PUBLIC_FIXTURE_FINGERPRINT
    );
    let list = f.call("list", &[], &[], 0, "ok");
    schema::check_published("key_list.schema.json", &list["data"]).unwrap();
    let item = &list["data"]["items"][0];
    assert_eq!(item["id"], key_id(&imported));
    for (field, path) in ["private_key_path", "public_key_path"]
        .into_iter()
        .zip(f.files())
    {
        let stored = PathBuf::from(item[field].as_str().unwrap());
        assert!(stored.is_absolute());
        // Compare resolved files, not platform spelling (Windows verbatim
        // prefixes and macOS's temporary-directory aliases can differ).
        assert_eq!(
            fs::canonicalize(stored).unwrap(),
            fs::canonicalize(path).unwrap()
        );
    }
}
#[test]
fn select_clear_and_remove() {
    let f = Fixture::new();
    let before = credentials::key_file_states(&f.files());
    let imported = f.import();
    let id = key_id(&imported);
    for (verb, request) in [
        ("select", NEXT),
        ("clear", LAST),
        ("remove", "01M4CC0VMTTNKVYS6XWQG4CK1R"),
    ] {
        let mut flags = vec!["--request-id", request];
        if verb != "clear" {
            flags.extend(["--key-id", id]);
        }
        let result = f.call(verb, &flags, &[], 0, "ok");
        golden(&format!("key_{verb}"), &result, &f.env);
    }
    assert_eq!(
        f.call("list", &[], &[], 0, "ok")["data"]["items"],
        serde_json::json!([])
    );
    assert_eq!(credentials::key_file_states(&f.files()), before);
}
#[test]
fn importing_a_missing_source_succeeds_and_preserves_its_missing_state() {
    let f = Fixture::new();
    let path = f.env.home.path().join("absent-key-source");
    assert!(!path.exists());
    let imported = f.call(
        "import",
        &["--input", "-", "--request-id", REQUEST],
        br#"{"label":"Missing key","private_key_path":"absent-key-source"}"#,
        0,
        "ok",
    );
    let listed = f.call("list", &[], &[], 0, "ok");
    schema::check_published("key_list.schema.json", &listed["data"]).unwrap();
    let key = &listed["data"]["items"][0];
    assert_eq!(key["id"], key_id(&imported));
    assert_eq!(key["private_source_state"], "missing");
    let stored = PathBuf::from(key["private_key_path"].as_str().unwrap());
    assert!(stored.is_absolute());
    assert_eq!(stored.file_name(), path.file_name());
    assert_eq!(
        fs::canonicalize(stored.parent().unwrap()).unwrap(),
        fs::canonicalize(f.env.home.path()).unwrap()
    );
    assert!(key["public_key_path"].is_null());
    assert!(!path.exists());
}
#[test]
fn selecting_the_selected_key_and_clearing_no_selection_are_no_ops() {
    let f = Fixture::new();
    let imported = f.import();
    for (request, code) in [(NEXT, "ok"), (LAST, "already_applied")] {
        f.call(
            "select",
            &["--key-id", key_id(&imported), "--request-id", request],
            &[],
            0,
            code,
        );
    }
    for (request, code) in [
        ("01M4CC0VMTTNKVYS6XWQG4CK1R", "ok"),
        ("01M4CC0VMTTNKVYS6XWQG4CK1S", "already_applied"),
    ] {
        f.call("clear", &["--request-id", request], &[], 0, code);
    }
}
#[test]
fn importing_an_already_registered_source_is_a_noop() {
    let f = Fixture::new();
    let first = f.import();
    let before = credentials::key_file_states(&f.files());
    let repeated = f.call(
        "import",
        &["--input", "-", "--request-id", NEXT],
        IMPORT,
        0,
        "already_applied",
    );
    assert_eq!(repeated["data"], first["data"]);
    assert_eq!(
        f.call("list", &[], &[], 0, "ok")["data"]["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(credentials::key_file_states(&f.files()), before);
}

#[test]
fn registry_noops_preserve_key_files_in_both_modes() {
    for verb in ["import", "select", "clear"] {
        for json in [false, true] {
            let f = Fixture::new();
            let imported = f.import();
            let id = key_id(&imported);
            if verb == "select" {
                f.call(
                    "select",
                    &["--key-id", id, "--request-id", NEXT],
                    &[],
                    0,
                    "ok",
                );
            }
            let before = credentials::key_file_states(&f.files());
            let listed = f.call("list", &[], &[], 0, "ok")["data"].clone();
            let mut args = vec!["key", verb, "--request-id", LAST];
            if verb == "import" {
                args.extend(["--input", "-"]);
            }
            if verb == "select" {
                args.extend(["--key-id", id]);
            }
            super::human::assert_noop_in_mode(&args, &f.env, IMPORT, json);
            assert!(
                credentials::key_file_states(&f.files()) == before,
                "key files changed"
            );
            assert_eq!(f.call("list", &[], &[], 0, "ok")["data"], listed);
        }
    }
}

#[test]
fn accepted_key_removal_with_lost_result_is_a_noop_in_both_modes() {
    use manyhands::{
        repository::{
            FailurePoint, KeyRemoveInput, Mutation, RepositoryService, RequestId, RequestState,
            SharedKeyId,
        },
        results::ResultCode,
    };
    for json in [false, true] {
        let f = Fixture::new();
        let imported = f.import();
        let id = key_id(&imported);
        let before = credentials::key_file_states(&f.files());
        {
            let service = RepositoryService::open_at_with_failure_point_for_testing(
                &f.env.data,
                FailurePoint::BeforeRequestSettlement,
            )
            .unwrap();
            let lost = crate::support::mutation::execute_with(
                &service,
                NEXT,
                Mutation::KeyRemove(KeyRemoveInput {
                    id: SharedKeyId::parse(id).unwrap(),
                }),
            );
            assert_eq!(lost.code, ResultCode::InternalError);
            assert_eq!(
                service
                    .request_record(RequestId::parse(NEXT).unwrap())
                    .unwrap()
                    .unwrap()
                    .state,
                RequestState::Accepted
            );
        }
        assert!(
            f.call("list", &[], &[], 0, "ok")["data"]["items"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        let result = super::human::assert_noop_in_mode(
            &["key", "remove", "--key-id", id, "--request-id", NEXT],
            &f.env,
            &[],
            json,
        );
        assert_eq!(result["data"]["id"], id);
        assert!(
            credentials::key_file_states(&f.files()) == before,
            "imported files changed"
        );
        assert!(
            f.call("list", &[], &[], 0, "ok")["data"]["items"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        let record = checked(
            &["request", "show", "--json", "--request-id", NEXT],
            &f.env,
            &[],
            0,
            "ok",
            Some("request.schema.json"),
        );
        assert_eq!(record["data"]["state"], "finished");
        assert_eq!(record["data"]["result"]["code"], "already_applied");
        assert_eq!(record["data"]["result"]["outcome"], "noop");
    }
}

#[cfg(unix)]
#[test]
fn completed_key_deletion_with_lost_result_is_a_noop_in_both_modes() {
    use manyhands::{
        repository::{
            FailurePoint, KeyDeleteInput, Mutation, RepositoryService, RequestId, RequestState,
            keys::KeyStore,
        },
        results::ResultCode,
    };
    for json in [false, true] {
        let f = Fixture::new();
        let key = generate(&f.env);
        let other = generate(&f.env);
        let build = || {
            Mutation::KeyDelete(KeyDeleteInput {
                store: KeyStore::for_home(f.env.home.path()).unwrap(),
                id: key.id,
            })
        };
        {
            let service = RepositoryService::open_at_with_failure_point_for_testing(
                &f.env.data,
                FailurePoint::BeforeRequestSettlement,
            )
            .unwrap();
            let preview = crate::support::confirmation::prepare_with(&service, &build());
            let lost = crate::support::mutation::execute_confirmed(
                &service,
                REQUEST,
                &preview.data.unwrap().confirmation_id,
                build(),
            );
            assert_eq!(lost.code, ResultCode::InternalError);
            assert_eq!(
                service
                    .request_record(RequestId::parse(REQUEST).unwrap())
                    .unwrap()
                    .unwrap()
                    .state,
                RequestState::Accepted
            );
        }
        let private = &key.private_key_path;
        let public = key.public_key_path.as_ref().unwrap();
        assert!(!private.exists() && !public.exists());
        // Files subsequently placed at the old names are not the deleted key.
        fs::write(private, b"unrelated private-path replacement\n").unwrap();
        fs::write(public, b"unrelated public-path replacement\n").unwrap();
        let mut files = f.files();
        files.extend([
            private.clone(),
            public.clone(),
            other.private_key_path,
            other.public_key_path.unwrap(),
        ]);
        let before = credentials::key_file_states(&files);
        let listed = f.call("list", &[], &[], 0, "ok")["data"].clone();
        assert!(
            !listed["items"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["id"] == key.id.to_string())
        );
        let result = super::human::assert_noop_in_mode(
            &[
                "key",
                "delete",
                "--key-id",
                &key.id.to_string(),
                "--request-id",
                REQUEST,
            ],
            &f.env,
            &[],
            json,
        );
        assert_eq!(result["data"]["id"], key.id.to_string());
        assert!(
            credentials::key_file_states(&files) == before,
            "key/replacement files changed"
        );
        assert_eq!(f.call("list", &[], &[], 0, "ok")["data"], listed);
        let record = checked(
            &["request", "show", "--json", "--request-id", REQUEST],
            &f.env,
            &[],
            0,
            "ok",
            Some("request.schema.json"),
        );
        assert_eq!(record["data"]["state"], "finished");
        assert_eq!(record["data"]["result"]["code"], "already_applied");
        assert_eq!(record["data"]["result"]["outcome"], "noop");
    }
}

#[test]
fn removing_the_selected_key_is_selected_key_in_use() {
    let f = Fixture::new();
    let imported = f.import();
    let before = credentials::key_file_states(&f.files());
    f.call(
        "select",
        &["--key-id", key_id(&imported), "--request-id", NEXT],
        &[],
        0,
        "ok",
    );
    f.call(
        "remove",
        &["--key-id", key_id(&imported), "--request-id", LAST],
        &[],
        3,
        "selected_key_in_use",
    );
    assert_eq!(credentials::key_file_states(&f.files()), before);
}
#[test]
fn deleting_an_imported_key_is_key_not_deletable_and_its_files_are_unchanged() {
    let f = Fixture::new();
    let imported = f.import();
    let before = credentials::key_file_states(&f.files());
    f.call(
        "delete",
        &["--key-id", key_id(&imported), "--prepare"],
        &[],
        2,
        "key_not_deletable",
    );
    assert_eq!(credentials::key_file_states(&f.files()), before);
}

#[cfg(unix)]
pub fn generate(env: &CliEnv) -> manyhands::repository::SharedKeyRegistration {
    use manyhands::repository::{
        OperationId, RepositoryService,
        keys::{GenerateSharedKeyOutcome, GenerateSharedKeyRequest, KeyProtection, KeyStore},
    };
    let service = RepositoryService::open_at(&env.data).unwrap();
    let store = KeyStore::for_home(env.home.path()).unwrap();
    match service
        .generate_shared_key(
            &store,
            GenerateSharedKeyRequest {
                operation_id: OperationId::new(),
                label: "Generated key".into(),
                protection: KeyProtection::Unencrypted,
            },
        )
        .unwrap()
    {
        GenerateSharedKeyOutcome::Created(key) => key,
        _ => panic!("fixture generation did not complete"),
    }
}
#[cfg(unix)]
#[test]
fn delete_a_generated_key() {
    let f = Fixture::new();
    let key = generate(&f.env);
    let id = key.id.to_string();
    f.call(
        "delete",
        &["--key-id", &id, "--request-id", NEXT],
        &[],
        3,
        "confirmation_required",
    );
    let preview = f.call("delete", &["--key-id", &id, "--prepare"], &[], 0, "ok");
    assert_eq!(
        preview["data"]["details"]["private_key_path"],
        key.private_key_path.to_str().unwrap()
    );
    assert_eq!(
        preview["data"]["details"]["public_key_path"],
        key.public_key_path.as_ref().unwrap().to_str().unwrap()
    );
    assert_eq!(
        preview["data"]["details"]["fingerprint"],
        key.public_key_fingerprint.as_deref().unwrap()
    );
    golden("key_delete_prepare", &preview, &f.env);
    assert!(key.private_key_path.exists());
    let result = f.call(
        "delete",
        &[
            "--key-id",
            &id,
            "--request-id",
            NEXT,
            "--confirm",
            preview["data"]["confirmation_id"].as_str().unwrap(),
        ],
        &[],
        0,
        "ok",
    );
    golden("key_delete", &result, &f.env);
    assert!(!key.private_key_path.exists());
    assert!(!key.public_key_path.unwrap().exists());
}
#[cfg(unix)]
#[test]
fn deleting_the_selected_generated_key_is_selected_key_in_use() {
    let f = Fixture::new();
    let key = generate(&f.env);
    let id = key.id.to_string();
    let files = [key.private_key_path, key.public_key_path.unwrap()];
    let before = credentials::key_file_states(&files);
    f.call(
        "select",
        &["--key-id", &id, "--request-id", REQUEST],
        &[],
        0,
        "ok",
    );
    f.call(
        "delete",
        &["--key-id", &id, "--prepare"],
        &[],
        3,
        "selected_key_in_use",
    );
    assert_eq!(credentials::key_file_states(&files), before);
}
#[cfg(unix)]
#[test]
fn an_interrupted_deletion_is_completed_by_the_same_request() {
    use manyhands::repository::{
        FailurePoint, KeyDeleteInput, Mutation, RepositoryService, RequestId, RequestState,
        keys::KeyStore,
    };
    let f = Fixture::new();
    let key = generate(&f.env);
    let other = generate(&f.env);
    let other_files = [other.private_key_path, other.public_key_path.unwrap()];
    let before = credentials::key_file_states(&other_files);
    let build = || {
        Mutation::KeyDelete(KeyDeleteInput {
            store: KeyStore::for_home(f.env.home.path()).unwrap(),
            id: key.id,
        })
    };
    {
        let service = RepositoryService::open_at_with_failure_point_for_testing(
            &f.env.data,
            FailurePoint::DeletionAfterPrivatePhase,
        )
        .unwrap();
        let preview = crate::support::confirmation::prepare_with(&service, &build());
        let confirmation = preview.data.unwrap().confirmation_id;
        let stopped =
            crate::support::mutation::execute_confirmed(&service, REQUEST, &confirmation, build());
        assert_eq!(stopped.outcome, manyhands::results::Outcome::Partial);
        assert_eq!(
            service
                .request_record(RequestId::parse(REQUEST).unwrap())
                .unwrap()
                .unwrap()
                .state,
            RequestState::Accepted
        );
    }
    assert!(!key.private_key_path.exists());
    assert!(key.public_key_path.as_ref().unwrap().exists());
    let id = key.id.to_string();
    let result = f.call(
        "delete",
        &["--key-id", &id, "--request-id", REQUEST],
        &[],
        0,
        "ok",
    );
    assert!(!key.public_key_path.unwrap().exists());
    let record = checked(
        &["request", "show", "--json", "--request-id", REQUEST],
        &f.env,
        &[],
        0,
        "ok",
        Some("request.schema.json"),
    );
    assert_eq!(record["data"]["state"], "finished");
    assert_eq!(
        f.call(
            "delete",
            &["--key-id", &id, "--request-id", REQUEST],
            &[],
            0,
            "ok"
        ),
        result
    );
    assert_eq!(credentials::key_file_states(&other_files), before);
}
#[test]
fn a_key_id_that_is_a_short_code_or_lowercase_is_invalid_id() {
    let f = Fixture::new();
    for verb in ["select", "remove", "delete"] {
        for id in ["abc123", "01m4cc0vmttnkvys6xwqg4ck1n"] {
            let v = f.call(
                verb,
                &["--key-id", id, "--request-id", REQUEST],
                &[],
                2,
                "invalid_id",
            );
            assert_eq!(v["request_id"], REQUEST);
            assert!(v["operation_id"].is_null() && v["data"].is_null());
        }
    }
}
#[test]
fn import_shapes_and_key_grammar_are_closed() {
    let f = Fixture::new();
    for input in [
        br#"{}"#.as_slice(),
        br#"{"label":null,"private_key_path":"imported"}"#,
        br#"{"label":"sentinel","private_key_path":"imported","public_key_path":null}"#,
        br#"{"label":"sentinel","private_key_path":1}"#,
        br#"{"label":"sentinel","label":"duplicate","private_key_path":"imported"}"#,
        br#"{"label":"sentinel","private_key_path":"imported","passphrase":"sentinel"}"#,
    ] {
        let v = f.call(
            "import",
            &["--input", "-", "--request-id", REQUEST],
            input,
            2,
            "invalid_input",
        );
        assert_eq!(v["request_id"], REQUEST);
        let out = run(
            &["key", "import", "--input", "-", "--request-id", REQUEST],
            &f.env,
            input,
        );
        assert_eq!(out.status, 2);
        assert!(!String::from_utf8(out.stderr).unwrap().contains("sentinel"));
    }
    for verb in ["import", "select", "clear", "remove", "delete"] {
        let mut flags = vec!["--prepare"];
        if verb == "import" {
            flags.extend(["--input", "-"]);
        }
        if ["select", "remove", "delete"].contains(&verb) {
            flags.extend(["--key-id", REQUEST]);
        }
        let mut repo = flags.clone();
        repo.extend(["--repo", "."]);
        let v = f.call(verb, &repo, IMPORT, 2, "usage_error");
        assert!(v["request_id"].is_null());
        if verb != "import" {
            flags.extend(["--input", "-"]);
            f.call(verb, &flags, IMPORT, 2, "usage_error");
        }
    }
    for verb in ["import", "select", "remove", "delete"] {
        f.call(verb, &["--prepare"], &[], 2, "usage_error");
    }
    f.call(
        "clear",
        &["--key-id", REQUEST, "--prepare"],
        &[],
        2,
        "usage_error",
    );
}
#[test]
fn ids_are_checked_in_order_before_import_input() {
    let f = Fixture::new();
    for (request, confirm, expected) in [
        ("bad", "bad", Value::Null),
        (REQUEST, "bad", Value::String(REQUEST.into())),
    ] {
        let v = f.call(
            "import",
            &[
                "--input",
                "missing-input",
                "--request-id",
                request,
                "--confirm",
                confirm,
            ],
            &[],
            2,
            "invalid_id",
        );
        assert_eq!(v["request_id"], expected);
    }
}
#[test]
fn confirmed_flags_match_key_prepare() {
    super::mutation::key_verbs_match_prepare();
}
#[test]
fn a_label_with_control_characters_is_escaped_in_human_output() {
    let f = Fixture::new();
    f.call(
        "import",
        &["--input", "-", "--request-id", REQUEST],
        br#"{"label":"k\u001b[2J","private_key_path":"imported"}"#,
        0,
        "ok",
    );
    let out = run(&["key", "list"], &f.env, &[]);
    assert_eq!(out.status, 0);
    assert!(out.stderr.is_empty());
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(!text.contains('\x1b'));
    assert!(text.contains("k\\u{1b}[2J"));
}
