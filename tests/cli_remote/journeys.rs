//! Onboarding through the binary only. git2 builds existing history, never
//! onboarding state. The server's own Git helpers are outside the CLI process.
use super::*;
use ssh_remote::{FixtureError, SshRemoteFixture};

pub const CASES: &[ssh_harness::Case] = &[
    ("onboard_an_empty_directory", onboard_an_empty_directory),
    (
        "onboard_an_existing_repository",
        onboard_an_existing_repository,
    ),
    #[cfg(unix)]
    ("onboard_with_a_generated_key", onboard_with_a_generated_key),
];

fn execute(env: &CliEnv, args: &[&str], input: &Value, confirmed: bool) -> Value {
    let bytes = serde_json::to_vec(input).unwrap();
    let preview = confirmed.then(|| {
        let mut args = args.to_vec();
        args.push("--prepare");
        let preview = checked(env, &args, &bytes, "ok", 0, None);
        assert!(preview["request_id"].is_null());
        preview
    });
    let request = repository::RequestId::new().to_string();
    let mut args = args.to_vec();
    args.extend(["--request-id", &request]);
    if let Some(preview) = &preview {
        args.extend([
            "--confirm",
            preview["data"]["confirmation_id"].as_str().unwrap(),
        ]);
    }
    // checked -> run_cli_with_stdin -> collect_cli checks one JSON envelope,
    // empty stderr and both output pipes reaching EOF within a second of exit.
    let result = checked(env, &args, &bytes, "ok", 0, None);
    assert_eq!(result["request_id"], request);
    result
}

enum KeySource {
    Imported,
    #[cfg(unix)]
    Generated,
}

fn onboard(existing: bool, key_source: KeySource) -> Result<(), FixtureError> {
    let server = SshRemoteFixture::start()?;
    private_fixture(&server);
    let temp = tempfile::tempdir().unwrap();
    let env = CliEnv::new(temp.path().join("data"));
    let existing = existing.then(|| cli::born_repository_in(temp.path(), "repository"));
    let root = existing
        .as_ref()
        .map_or_else(|| temp.path().join("repository"), |repo| repo.root.clone());
    let original_head = existing
        .as_ref()
        .map(|repo| repo.repository.head().unwrap().target().unwrap());
    if existing.is_none() {
        std::fs::create_dir(&root).unwrap();
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0);
    }
    let verb = if existing.is_some() {
        "enable"
    } else {
        "create"
    };
    execute(
        &env,
        &[
            "repo",
            verb,
            "--repo",
            root.to_str().unwrap(),
            "--input",
            "-",
        ],
        &serde_json::json!({
            "primary_branch":"main",
            "identity":{"name":"Journey Person","email":"journey@example.invalid"}
        }),
        true,
    );
    for (verb, input) in [
        (
            "add",
            serde_json::json!({"name":"origin","url":server.url()}),
        ),
        ("select", serde_json::json!({"name":"origin"})),
    ] {
        execute(
            &env,
            &[
                "remote",
                verb,
                "--repo",
                root.to_str().unwrap(),
                "--input",
                "-",
            ],
            &input,
            false,
        );
    }
    let key = match key_source {
        // Windows' key store ignores the isolated HOME: compile out CLI writes.
        #[cfg(unix)]
        KeySource::Generated => {
            let key = execute(
                &env,
                &["key", "generate", "--input", "-"],
                &serde_json::json!({"label":"Journey generated key","protection":"none"}),
                false,
            );
            let public = checked(
                &env,
                &[
                    "key",
                    "public",
                    "--key-id",
                    key["data"]["id"].as_str().unwrap(),
                ],
                &[],
                "ok",
                0,
                Some("public_key.schema.json"),
            );
            assert_eq!(public["data"]["matches_registration"], true);
            let parsed = russh::keys::PublicKey::from_openssh(
                public["data"]["public_key"].as_str().unwrap(),
            )
            .unwrap();
            server.allow_client_public_key(parsed);
            key
        }
        KeySource::Imported => execute(
            &env,
            &["key", "import", "--input", "-"],
            &serde_json::json!({"label":"Journey fixture key","private_key_path":server.client_key_path()}),
            false,
        ),
    };
    execute(
        &env,
        &[
            "key",
            "select",
            "--key-id",
            key["data"]["id"].as_str().unwrap(),
        ],
        &Value::Null,
        false,
    );
    let observed = host_call(&env, &root, "observe", &Value::Null, &[], "ok", 0);
    assert!(observed["data"]["pinned_fingerprint"].is_null());
    assert_eq!(
        observed["data"]["presented_fingerprint"],
        server.host_identity().sha256
    );
    // Approve precisely the observation, rather than substituting fixture state.
    let input = serde_json::json!({
        "authority":format!("{}:{}", observed["data"]["host"].as_str().unwrap(), observed["data"]["port"].as_u64().unwrap()),
        "algorithm":observed["data"]["presented_algorithm"],
        "fingerprint":observed["data"]["presented_fingerprint"]
    });
    let preview = prepare(&env, &root, "approve", &input);
    confirm(&env, &root, "approve", &input, &preview, "ok", 0);
    let before = server.authentication_counts();
    let accepted_before = server.accepted_keys().len();
    let final_observation = host_call(&env, &root, "observe", &Value::Null, &[], "ok", 0);
    let after = server.authentication_counts();
    assert!(after.0 > before.0, "final observation must authenticate");
    assert_eq!(after.1, before.1, "selected key must not be rejected");
    assert!(server.accepted_keys().len() > accepted_before);
    assert_eq!(
        final_observation["data"]["pinned_fingerprint"],
        observed["data"]["presented_fingerprint"]
    );
    assert_eq!(
        final_observation["data"]["pinned_algorithm"],
        observed["data"]["presented_algorithm"]
    );
    assert_eq!(
        final_observation["data"]["presented_fingerprint"],
        observed["data"]["presented_fingerprint"]
    );
    assert_eq!(final_observation["data"]["reapproval_required"], false);
    assert!(
        server
            .accepted_keys()
            .iter()
            .all(|key| *key == server.allowed_client_public_key())
    );
    assert!(server.receive_updates().is_empty());
    let listed = checked(
        &env,
        &["repo", "list"],
        &[],
        "ok",
        0,
        Some("repository_list.schema.json"),
    );
    assert!(
        listed["data"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|repo| repo["root"] == root.canonicalize().unwrap().to_str().unwrap())
    );
    if let Some(original) = original_head {
        let repo = git2::Repository::open(&root).unwrap();
        assert!(
            repo.graph_descendant_of(repo.head().unwrap().target().unwrap(), original)
                .unwrap()
        );
    }
    Ok(())
}

fn onboard_an_empty_directory() -> Result<(), FixtureError> {
    onboard(false, KeySource::Imported)
}
fn onboard_an_existing_repository() -> Result<(), FixtureError> {
    onboard(true, KeySource::Imported)
}
#[cfg(unix)]
fn onboard_with_a_generated_key() -> Result<(), FixtureError> {
    onboard(false, KeySource::Generated)
}
