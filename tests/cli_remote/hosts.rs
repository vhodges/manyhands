use super::*;
use ssh_remote::{FixtureError, SshRemoteFixture};

macro_rules! cases {
    ($($case:ident),+ $(,)?) => { pub const CASES: &[ssh_harness::Case] = &[$((stringify!($case), $case)),+]; };
}
cases!(
    observe_an_unpinned_host,
    observe_names_only_the_remotes_authorities,
    observe_without_a_publication_remote_is_publication_remote_required,
    observe_with_a_missing_selected_source_is_key_unavailable,
    observe_a_pinned_host_without_a_selected_key_is_no_selected_key,
    approve_by_prepare_and_confirm,
    approving_the_pinned_key_is_a_no_op,
    approve_a_fingerprint_the_host_does_not_present_is_host_mismatch,
    approve_over_another_pin_is_host_replacement_required,
    replace_after_the_host_key_rotates,
    replace_with_a_wrong_old_fingerprint_is_refused_at_prepare,
    replace_naming_a_key_the_host_does_not_present_is_host_mismatch,
    observe_prints_the_next_commands_input_line,
    human_host_verbs_and_noops,
    host_noop_prepares_preserve_state_in_both_modes,
    a_server_accepting_without_the_selected_key_is_key_rejected,
    host_grammar_and_input_shapes,
    ipv6_observation_input_round_trips_the_c1_parser
);

fn ipv6_observation_input_round_trips_the_c1_parser() -> Result<(), FixtureError> {
    let mut dto = repository::HostObservationDto {
        host: "2001:db8::1".into(),
        port: 2222,
        fetch: true,
        push: false,
        presented_algorithm: "ssh-ed25519".into(),
        presented_fingerprint: other_fingerprint(),
        pinned_algorithm: None,
        pinned_fingerprint: None,
        reapproval_required: true,
    };
    for replacement in [false, true] {
        if replacement {
            dto.pinned_algorithm = Some("ssh-ed25519".into());
            dto.pinned_fingerprint = Some(support::credentials::OTHER_FINGERPRINT.into());
        }
        let output = human_admin::observation(&dto);
        assert!(output.stdout.contains("Reapproval required: true"));
        let line = output
            .stdout
            .lines()
            .find(|line| line.starts_with('{'))
            .unwrap();
        let input: Value = serde_json::from_str(line).unwrap();
        assert_eq!(input["authority"], "[2001:db8::1]:2222");
        assert_eq!(
            parse::authority(input["authority"].as_str().unwrap()).unwrap(),
            (dto.host.clone(), dto.port)
        );
        assert_eq!(input.get("old_fingerprint").is_some(), replacement);
        if replacement {
            assert_eq!(
                input["old_fingerprint"],
                support::credentials::OTHER_FINGERPRINT
            );
        }
    }
    Ok(())
}

pub struct Plain {
    pub server: SshRemoteFixture,
    pub env: CliEnv,
    pub root: std::path::PathBuf,
    _data: tempfile::TempDir,
}
impl Plain {
    pub fn new() -> Result<Self, FixtureError> {
        let server = SshRemoteFixture::start()?;
        private_fixture(&server);
        let data = tempfile::tempdir().unwrap();
        let env = CliEnv::new(data.path().join("data"));
        let root = env.home.path().join("repo");
        let create = include_bytes!("../fixtures/cli_input_v1/repo_create.json");
        let preview = checked(
            &env,
            &[
                "repo",
                "create",
                "--repo",
                root.to_str().unwrap(),
                "--input",
                "-",
                "--prepare",
            ],
            create,
            "ok",
            0,
            None,
        );
        checked(
            &env,
            &[
                "repo",
                "create",
                "--repo",
                root.to_str().unwrap(),
                "--input",
                "-",
                "--request-id",
                &repository::RequestId::new().to_string(),
                "--confirm",
                preview["data"]["confirmation_id"].as_str().unwrap(),
            ],
            create,
            "ok",
            0,
            None,
        );
        let f = Self {
            server,
            env,
            root,
            _data: data,
        };
        f.mutate(
            "remote",
            "add",
            serde_json::json!({"name":"origin","url":f.server.url()}),
            &[],
        );
        f.mutate(
            "remote",
            "select",
            serde_json::json!({"name":"origin"}),
            &[],
        );
        let imported = f.mutate("key", "import", serde_json::json!({"label":"CLI SSH fixture","private_key_path":f.server.client_key_path()}), &[]);
        checked(
            &f.env,
            &[
                "key",
                "select",
                "--key-id",
                imported["data"]["id"].as_str().unwrap(),
                "--request-id",
                &repository::RequestId::new().to_string(),
            ],
            &[],
            "ok",
            0,
            None,
        );
        Ok(f)
    }
    fn mutate(&self, resource: &str, verb: &str, input: Value, extra: &[&str]) -> Value {
        let request = repository::RequestId::new().to_string();
        let mut args = vec![resource, verb, "--input", "-", "--request-id", &request];
        if resource == "remote" {
            args.extend(["--repo", self.root.to_str().unwrap()]);
        }
        args.extend_from_slice(extra);
        checked(
            &self.env,
            &args,
            &serde_json::to_vec(&input).unwrap(),
            "ok",
            0,
            None,
        )
    }
    pub fn approve(&self) -> Value {
        let input = host_input(&self.server);
        let preview = prepare(&self.env, &self.root, "approve", &input);
        confirm(&self.env, &self.root, "approve", &input, &preview, "ok", 0)
    }
    fn observe(&self) -> Value {
        host_call(&self.env, &self.root, "observe", &Value::Null, &[], "ok", 0)
    }
}

fn observe_an_unpinned_host() -> Result<(), FixtureError> {
    let f = Plain::new()?;
    let observed = f.observe();
    assert_eq!(
        observed["data"]["presented_fingerprint"],
        f.server.host_identity().sha256
    );
    assert!(observed["data"]["pinned_fingerprint"].is_null());
    assert_eq!(pins(&f.env), serde_json::json!([]));
    // Relative selectors are joined to the invocation directory before F2.
    let relative = checked(
        &f.env,
        &["host", "observe", "--repo", "repo"],
        &[],
        "ok",
        0,
        Some("host_observation.schema.json"),
    );
    assert_eq!(relative, observed);
    golden(
        "host_observe",
        &observed,
        &f.root,
        f.server.address().port(),
    );
    Ok(())
}
fn observe_names_only_the_remotes_authorities() -> Result<(), FixtureError> {
    let f = Plain::new()?;
    let sockets = f.server.accepted_sockets();
    host_call(
        &f.env,
        &f.root,
        "observe",
        &Value::Null,
        &["--authority", "other.invalid:2222"],
        "invalid_input",
        2,
    );
    assert_eq!(f.server.accepted_sockets(), sockets);
    Ok(())
}
fn observe_without_a_publication_remote_is_publication_remote_required() -> Result<(), FixtureError>
{
    let f = Plain::new()?;
    f.mutate("remote", "select", serde_json::json!({"name":null}), &[]);
    host_call(
        &f.env,
        &f.root,
        "observe",
        &Value::Null,
        &[],
        "publication_remote_required",
        3,
    );
    Ok(())
}
fn observe_with_a_missing_selected_source_is_key_unavailable() -> Result<(), FixtureError> {
    let f = Plain::new()?;
    let imported = f.mutate("key","import",serde_json::json!({"label":"Missing source","private_key_path":f.env.home.path().join("absent-private-source")}),&[]);
    checked(
        &f.env,
        &[
            "key",
            "select",
            "--key-id",
            imported["data"]["id"].as_str().unwrap(),
            "--request-id",
            &repository::RequestId::new().to_string(),
        ],
        &[],
        "ok",
        0,
        None,
    );
    let sockets = f.server.accepted_sockets();
    host_call(
        &f.env,
        &f.root,
        "observe",
        &Value::Null,
        &[],
        "key_unavailable",
        3,
    );
    assert_eq!(f.server.accepted_sockets(), sockets);
    assert_eq!(pins(&f.env), serde_json::json!([]));
    Ok(())
}
fn observe_a_pinned_host_without_a_selected_key_is_no_selected_key() -> Result<(), FixtureError> {
    let f = Plain::new()?;
    f.approve();
    checked(
        &f.env,
        &[
            "key",
            "clear",
            "--request-id",
            &repository::RequestId::new().to_string(),
        ],
        &[],
        "ok",
        0,
        None,
    );
    let before = pins(&f.env);
    host_call(
        &f.env,
        &f.root,
        "observe",
        &Value::Null,
        &[],
        "no_selected_key",
        3,
    );
    assert_eq!(pins(&f.env), before);
    Ok(())
}
fn references(root: &Path) -> Vec<(String, Option<git2::Oid>, Option<String>)> {
    let repo = git2::Repository::open(root).unwrap();
    let mut refs: Vec<_> = repo
        .references()
        .unwrap()
        .map(|r| {
            let r = r.unwrap();
            (
                r.name().unwrap().to_owned(),
                r.target(),
                r.symbolic_target().map(str::to_owned),
            )
        })
        .collect();
    refs.sort();
    refs
}
fn approve_by_prepare_and_confirm() -> Result<(), FixtureError> {
    let f = Plain::new()?;
    let before = references(&f.root);
    let input = host_input(&f.server);
    let sockets = f.server.accepted_sockets();
    let preview = prepare(&f.env, &f.root, "approve", &input);
    assert_eq!(f.server.accepted_sockets(), sockets);
    assert_eq!(pins(&f.env), serde_json::json!([]));
    golden(
        "host_approve_prepare",
        &preview,
        &f.root,
        f.server.address().port(),
    );
    let applied = confirm(&f.env, &f.root, "approve", &input, &preview, "ok", 0);
    golden("host_approve", &applied, &f.root, f.server.address().port());
    assert_eq!(pins(&f.env)[0]["pinned_fingerprint"], input["fingerprint"]);
    assert!(f.server.receive_updates().is_empty());
    assert_eq!(references(&f.root), before);
    Ok(())
}
fn approving_the_pinned_key_is_a_no_op() -> Result<(), FixtureError> {
    let f = Plain::new()?;
    f.approve();
    let sockets = f.server.accepted_sockets();
    host_call(
        &f.env,
        &f.root,
        "approve",
        &host_input(&f.server),
        &["--prepare"],
        "already_applied",
        0,
    );
    assert_eq!(f.server.accepted_sockets(), sockets);
    Ok(())
}
fn other_fingerprint() -> String {
    "SHA256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".into()
}
fn human_mismatch(f: &Plain, verb: &str, input: &Value) {
    let preview = prepare(&f.env, &f.root, verb, input);
    let request = repository::RequestId::new().to_string();
    let out = run(
        &f.env,
        &[
            "host",
            verb,
            "--repo",
            f.root.to_str().unwrap(),
            "--input",
            "-",
            "--request-id",
            &request,
            "--confirm",
            preview["data"]["confirmation_id"].as_str().unwrap(),
        ],
        &serde_json::to_vec(input).unwrap(),
    );
    assert_eq!(out.status, 3);
    assert!(out.stdout.is_empty());
    let stderr = std::str::from_utf8(&out.stderr).unwrap();
    for expected in [
        "code: host_mismatch",
        "Presented algorithm: ssh-ed25519",
        &f.server.host_identity().sha256,
        &request,
    ] {
        assert!(stderr.contains(expected));
    }
}
fn approve_a_fingerprint_the_host_does_not_present_is_host_mismatch() -> Result<(), FixtureError> {
    let f = Plain::new()?;
    let mut input = host_input(&f.server);
    input["fingerprint"] = other_fingerprint().into();
    let preview = prepare(&f.env, &f.root, "approve", &input);
    let result = confirm(
        &f.env,
        &f.root,
        "approve",
        &input,
        &preview,
        "host_mismatch",
        3,
    );
    assert_eq!(
        result["data"]["presented_fingerprint"],
        f.server.host_identity().sha256
    );
    golden(
        "host_approve_host_mismatch",
        &result,
        &f.root,
        f.server.address().port(),
    );
    human_mismatch(&f, "approve", &input);
    assert_eq!(pins(&f.env), serde_json::json!([]));
    Ok(())
}
fn approve_over_another_pin_is_host_replacement_required() -> Result<(), FixtureError> {
    let f = Plain::new()?;
    f.approve();
    f.server.rotate_host_key()?;
    let before = pins(&f.env);
    let sockets = f.server.accepted_sockets();
    let result = host_call(
        &f.env,
        &f.root,
        "approve",
        &host_input(&f.server),
        &["--prepare"],
        "host_replacement_required",
        3,
    );
    golden(
        "host_approve_host_replacement_required",
        &result,
        &f.root,
        f.server.address().port(),
    );
    let human = run(
        &f.env,
        &[
            "host",
            "approve",
            "--repo",
            f.root.to_str().unwrap(),
            "--input",
            "-",
            "--prepare",
        ],
        &serde_json::to_vec(&host_input(&f.server)).unwrap(),
    );
    assert_eq!(human.status, 3);
    assert!(human.stdout.is_empty());
    let stderr = std::str::from_utf8(&human.stderr).unwrap();
    for expected in [
        "code: host_replacement_required",
        "next: host.replace",
        before[0]["pinned_fingerprint"].as_str().unwrap(),
        &f.server.host_identity().sha256,
    ] {
        assert!(stderr.contains(expected));
    }
    assert_eq!(pins(&f.env), before);
    assert_eq!(f.server.accepted_sockets(), sockets);
    Ok(())
}
fn replace_after_the_host_key_rotates() -> Result<(), FixtureError> {
    let f = Plain::new()?;
    f.approve();
    let old = f.server.host_identity().sha256;
    f.server.rotate_host_key()?;
    let mut input = host_input(&f.server);
    input["old_fingerprint"] = old.into();
    let before = references(&f.root);
    let sockets = f.server.accepted_sockets();
    let preview = prepare(&f.env, &f.root, "replace", &input);
    assert_eq!(f.server.accepted_sockets(), sockets);
    golden(
        "host_replace_prepare",
        &preview,
        &f.root,
        f.server.address().port(),
    );
    let result = confirm(&f.env, &f.root, "replace", &input, &preview, "ok", 0);
    golden("host_replace", &result, &f.root, f.server.address().port());
    host_call(
        &f.env,
        &f.root,
        "replace",
        &input,
        &["--prepare"],
        "already_applied",
        0,
    );
    assert_eq!(pins(&f.env)[0]["pinned_fingerprint"], input["fingerprint"]);
    assert_eq!(references(&f.root), before);
    assert!(f.server.receive_updates().is_empty());
    Ok(())
}
fn replace_with_a_wrong_old_fingerprint_is_refused_at_prepare() -> Result<(), FixtureError> {
    let f = Plain::new()?;
    f.approve();
    let before = pins(&f.env);
    f.server.rotate_host_key()?;
    let mut input = host_input(&f.server);
    input["old_fingerprint"] = other_fingerprint().into();
    let sockets = f.server.accepted_sockets();
    host_call(
        &f.env,
        &f.root,
        "replace",
        &input,
        &["--prepare"],
        "invalid_input",
        2,
    );
    assert_eq!(pins(&f.env), before);
    assert_eq!(f.server.accepted_sockets(), sockets);
    Ok(())
}
fn replace_naming_a_key_the_host_does_not_present_is_host_mismatch() -> Result<(), FixtureError> {
    let f = Plain::new()?;
    f.approve();
    let before = pins(&f.env);
    let mut input = host_input(&f.server);
    input["old_fingerprint"] = input["fingerprint"].clone();
    input["fingerprint"] = other_fingerprint().into();
    let preview = prepare(&f.env, &f.root, "replace", &input);
    let result = confirm(
        &f.env,
        &f.root,
        "replace",
        &input,
        &preview,
        "host_mismatch",
        3,
    );
    assert_eq!(
        result["data"]["presented_fingerprint"],
        f.server.host_identity().sha256
    );
    assert_eq!(
        result["data"]["presented_algorithm"],
        f.server.host_identity().algorithm
    );
    human_mismatch(&f, "replace", &input);
    assert_eq!(pins(&f.env), before);
    Ok(())
}
fn observe_prints_the_next_commands_input_line() -> Result<(), FixtureError> {
    let f = Plain::new()?;
    let out = run(
        &f.env,
        &["host", "observe", "--repo", f.root.to_str().unwrap()],
        &[],
    );
    assert_eq!(out.status, 0);
    assert_eq!(out.stderr, b"progress: connecting\n");
    let text = std::str::from_utf8(&out.stdout).unwrap();
    for expected in [
        "Host: 127.0.0.1",
        "Port:",
        "Fetch: true",
        "Push: true",
        "Presented fingerprint:",
        "Pinned fingerprint: none",
        "Reapproval required: false",
        "host approve",
    ] {
        assert!(text.contains(expected));
    }
    let line = text.lines().find(|line| line.starts_with('{')).unwrap();
    let input: Value = serde_json::from_str(line).unwrap();
    assert_eq!(input, host_input(&f.server));
    checked(
        &f.env,
        &[
            "host",
            "approve",
            "--repo",
            f.root.to_str().unwrap(),
            "--input",
            "-",
            "--prepare",
        ],
        line.as_bytes(),
        "ok",
        0,
        Some("host_preview.schema.json"),
    );
    let text = text
        .replace(
            &format!("Port: {}\n", f.server.address().port()),
            "Port: 0\n",
        )
        .replace(
            &format!("127.0.0.1:{}", f.server.address().port()),
            "127.0.0.1:0",
        );
    cli::assert_cli_text_golden(
        "host_observe_text",
        &text,
        &[(&f.server.host_identity().sha256, "<presented>")],
    );
    Ok(())
}
fn human_host_verbs_and_noops() -> Result<(), FixtureError> {
    let f = Plain::new()?;
    for verb in ["approve", "replace"] {
        let mut input = host_input(&f.server);
        if verb == "replace" {
            let old = input["fingerprint"].clone();
            f.server.rotate_host_key()?;
            input = host_input(&f.server);
            input["old_fingerprint"] = old;
            let observed = run(
                &f.env,
                &["host", "observe", "--repo", f.root.to_str().unwrap()],
                &[],
            );
            assert_eq!(observed.status, 0);
            let text = std::str::from_utf8(&observed.stdout).unwrap();
            assert!(text.contains("host replace"));
            assert_eq!(
                serde_json::from_str::<Value>(text.lines().find(|l| l.starts_with('{')).unwrap())
                    .unwrap(),
                input
            );
        }
        let bytes = serde_json::to_vec(&input).unwrap();
        let preview = prepare(&f.env, &f.root, verb, &input);
        let request = repository::RequestId::new().to_string();
        let args = [
            "host",
            verb,
            "--repo",
            f.root.to_str().unwrap(),
            "--input",
            "-",
            "--request-id",
            &request,
            "--confirm",
            preview["data"]["confirmation_id"].as_str().unwrap(),
        ];
        let out = run(&f.env, &args, &bytes);
        assert_eq!(out.status, 0);
        let text = std::str::from_utf8(&out.stdout).unwrap();
        assert!(text.contains(&request));
        assert!(text.contains(input["fingerprint"].as_str().unwrap()));
        let record = checked(
            &f.env,
            &["request", "show", "--request-id", &request],
            &[],
            "ok",
            0,
            None,
        );
        assert_eq!(record["data"]["result"]["code"], "ok");
        if let Some(id) = record["data"]["result"]["operation_id"].as_str() {
            assert!(text.contains(id));
        }
        assert!(
            std::str::from_utf8(&out.stderr)
                .unwrap()
                .lines()
                .all(|l| l.starts_with("progress: "))
        );
        let noop = run(
            &f.env,
            &[
                "host",
                verb,
                "--repo",
                f.root.to_str().unwrap(),
                "--input",
                "-",
                "--prepare",
            ],
            &bytes,
        );
        assert_eq!(noop.status, 0);
        assert!(
            noop.stdout
                .starts_with(b"Already applied; nothing changed.")
        );
        assert!(noop.stderr.is_empty());
    }
    let observed = run(
        &f.env,
        &["host", "observe", "--repo", f.root.to_str().unwrap()],
        &[],
    );
    assert_eq!(observed.status, 0);
    assert!(
        !std::str::from_utf8(&observed.stdout)
            .unwrap()
            .lines()
            .any(|l| l.starts_with('{'))
    );
    Ok(())
}

fn host_noop_prepares_preserve_state_in_both_modes() -> Result<(), FixtureError> {
    let f = Plain::new()?;
    f.approve();
    let before = support::repository_and_worktree_snapshot_at(&f.root);
    let pin_before = pins(&f.env);
    let files = [f.server.client_key_path().to_owned()];
    let files_before = support::credentials::key_file_states(&files);
    let sockets = f.server.accepted_sockets();
    for verb in ["approve", "replace"] {
        let mut input = host_input(&f.server);
        if verb == "replace" {
            input["old_fingerprint"] = input["fingerprint"].clone();
        }
        for json in [false, true] {
            let mut args = vec![
                "host",
                verb,
                "--repo",
                f.root.to_str().unwrap(),
                "--input",
                "-",
                "--prepare",
            ];
            if json {
                args.push("--json");
            }
            let out = run(&f.env, &args, &serde_json::to_vec(&input).unwrap());
            assert_eq!(out.status, 0);
            assert!(out.stderr.is_empty());
            if json {
                let value = cli::json(&out);
                schema::check_published("envelope.schema.json", &value).unwrap();
                assert_eq!(value["outcome"], "noop");
                assert_eq!(value["code"], "already_applied");
                assert!(value["data"].is_null());
            } else {
                assert!(
                    out.stdout
                        .starts_with(b"Already applied; nothing changed.\n")
                );
            }
            assert_eq!(pins(&f.env), pin_before);
            assert_eq!(
                support::repository_and_worktree_snapshot_at(&f.root),
                before
            );
            assert!(
                support::credentials::key_file_states(&files) == files_before,
                "key file changed"
            );
            assert_eq!(f.server.accepted_sockets(), sockets);
        }
    }
    Ok(())
}

fn a_server_accepting_without_the_selected_key_is_key_rejected() -> Result<(), FixtureError> {
    let f = Plain::new()?;
    f.server.accept_anonymous();
    let input = host_input(&f.server);
    let preview = prepare(&f.env, &f.root, "approve", &input);
    confirm(
        &f.env,
        &f.root,
        "approve",
        &input,
        &preview,
        "key_rejected",
        3,
    );
    assert_eq!(pins(&f.env), serde_json::json!([]));
    Ok(())
}
fn host_grammar_and_input_shapes() -> Result<(), FixtureError> {
    let f = Plain::new()?;
    for verb in ["observe", "approve", "replace"] {
        let spec = catalog::find("host", verb).unwrap();
        assert_eq!(spec.availability, catalog::Availability::Available("C2"));
        assert_eq!(spec.repo, Some(catalog::RepoRule::Required));
        assert_eq!(
            spec.kind,
            if verb == "observe" {
                catalog::Kind::UnlockingRead
            } else {
                catalog::Kind::Mutation {
                    confirmed: true,
                    input: catalog::InputRule::Required,
                }
            }
        );
        checked(&f.env, &["host", verb], &[], "usage_error", 2, None);
    }
    checked(
        &f.env,
        &[
            "host",
            "observe",
            "--repo",
            f.root.to_str().unwrap(),
            "--authority",
            "host:0",
        ],
        &[],
        "usage_error",
        2,
        None,
    );
    for option in ["--prepare", "--request-id", "--confirm", "--input"] {
        let mut args = vec![
            "host",
            "observe",
            "--repo",
            f.root.to_str().unwrap(),
            option,
        ];
        if option != "--prepare" {
            args.push("bad");
        }
        checked(&f.env, &args, &[], "usage_error", 2, None);
    }
    for verb in ["approve", "replace"] {
        let mut input = host_input(&f.server);
        if verb == "replace" {
            input["old_fingerprint"] = other_fingerprint().into();
        }
        for field in input.as_object().unwrap().keys() {
            for value in [Value::Null, serde_json::json!(1)] {
                let mut bad = input.clone();
                bad[field] = value;
                host_call(
                    &f.env,
                    &f.root,
                    verb,
                    &bad,
                    &["--prepare"],
                    "invalid_input",
                    2,
                );
            }
            let mut bad = input.clone();
            bad.as_object_mut().unwrap().remove(field);
            host_call(
                &f.env,
                &f.root,
                verb,
                &bad,
                &["--prepare"],
                "invalid_input",
                2,
            );
        }
        for authority in ["host:0", "[bad]", "private-input-sentinel@host"] {
            let mut bad = input.clone();
            bad["authority"] = authority.into();
            let out = run(
                &f.env,
                &[
                    "host",
                    verb,
                    "--repo",
                    f.root.to_str().unwrap(),
                    "--input",
                    "-",
                    "--prepare",
                ],
                &serde_json::to_vec(&bad).unwrap(),
            );
            assert_eq!(out.status, 2);
            assert!(!String::from_utf8_lossy(&out.stderr).contains(authority));
        }
        let mut bad = input.clone();
        bad["passphrase"] = "private-input-sentinel".into();
        host_call(
            &f.env,
            &f.root,
            verb,
            &bad,
            &["--prepare"],
            "invalid_input",
            2,
        );
    }
    Ok(())
}
