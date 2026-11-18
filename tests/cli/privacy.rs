use super::mutation::{REQUEST, run};
use crate::support::cli::{CliEnv, CliRun};
use crate::support::{credentials, operations};
use std::{collections::BTreeSet, fs};

type OutputCase<'a> = (&'a str, &'a str, Vec<&'a str>, &'a [u8], i32);
const PASSWORD: &str = "CLI-PASSWORD-SENTINEL-7a82";
const QUERY: &str = "CLI-QUERY-SENTINEL-2b43";

fn scan_output(out: &CliRun) {
    for bytes in [&out.stdout, &out.stderr] {
        for sentinel in [PASSWORD, QUERY, operations::KEY_MATERIAL_SENTINEL]
            .into_iter()
            .chain(credentials::SECRET_SENTINELS)
        {
            absent(bytes, sentinel);
        }
    }
}

struct ExecutionPrivacy<'a> {
    env: &'a CliEnv,
    json: bool,
    covered: BTreeSet<String>,
}

impl ExecutionPrivacy<'_> {
    fn call(
        &self,
        args: &[&str],
        input: &[u8],
        json: bool,
        code: &str,
        status: i32,
    ) -> serde_json::Value {
        let mut flags = args.to_vec();
        if json {
            flags.push("--json");
        }
        let out = run(&flags, self.env, input);
        scan_output(&out);
        assert_eq!(out.status, status, "{} {}", args[0], args[1]);
        if json {
            let value = crate::support::cli::json(&out);
            assert_eq!(value["code"], code);
            crate::support::schema::check_published("envelope.schema.json", &value).unwrap();
            return value;
        }
        if status != 0 {
            assert!(out.stdout.is_empty());
            assert!(
                std::str::from_utf8(&out.stderr)
                    .unwrap()
                    .contains(&format!("code: {code}"))
            );
            return serde_json::Value::Null;
        }
        assert!(!out.stdout.is_empty());
        if args[0] != "request" && !args.contains(&"--prepare") {
            // The human mutation really executed on this mode's fresh fixture.
            // Read its stored code/data, not a second mutation or a replay used
            // as a substitute for executing once in each output mode.
            let request = args
                .windows(2)
                .find(|pair| pair[0] == "--request-id")
                .unwrap()[1];
            let stored = self.call(
                &["request", "show", "--request-id", request],
                &[],
                true,
                "ok",
                0,
            );
            assert_eq!(stored["data"]["state"], "finished");
            assert_eq!(stored["data"]["result"]["code"], code);
            stored["data"]["result"].clone()
        } else {
            // Obtain the confirmation or read DTO after also checking the
            // human preview/read output. Both invocations are scanned.
            self.call(args, input, true, code, status)
        }
    }

    fn success(&mut self, args: &[&str], input: &[u8]) -> serde_json::Value {
        let result = self.call(args, input, self.json, "ok", 0);
        assert!(!args.contains(&"--prepare"));
        self.covered.insert(format!("{} {}", args[0], args[1]));
        result
    }

    fn execute(
        &mut self,
        args: &[&str],
        input: &[u8],
        confirmed: bool,
    ) -> (serde_json::Value, String) {
        let confirmation = confirmed.then(|| {
            let mut prepare = args.to_vec();
            prepare.push("--prepare");
            let preview = self.call(&prepare, input, self.json, "ok", 0);
            preview["data"]["confirmation_id"]
                .as_str()
                .unwrap()
                .to_owned()
        });
        let request = manyhands::repository::RequestId::new().to_string();
        let mut execute = args.to_vec();
        execute.extend(["--request-id", &request]);
        if let Some(confirmation) = &confirmation {
            execute.extend(["--confirm", confirmation]);
        }
        (self.success(&execute, input), request)
    }
}

fn execution_outputs(json: bool) -> BTreeSet<String> {
    // Each mode gets independent repositories, registry, home and requests:
    // JSON success cannot be merely replay of the human-mode mutation.
    let f = super::mutation::Fixture::new();
    fs::write(
        f.env.home.path().join("imported"),
        credentials::private_fixture_bytes(),
    )
    .unwrap();
    fs::write(
        f.env.home.path().join("imported.pub"),
        credentials::public_fixture_text(),
    )
    .unwrap();
    operations::insert_key_material(
        &f.env.data,
        operations::OPERATION_A,
        "generate",
        "retained-for-inspection",
        Some("source-missing"),
    );
    let url = format!("https://user:{PASSWORD}@example.invalid/repo.git?token={QUERY}");
    f.repo.repository.remote("origin", &url).unwrap();
    f.repo
        .repository
        .remote("publish", "ssh://git@example.invalid/repo.git")
        .unwrap();
    let remote = serde_json::to_vec(&serde_json::json!({"name":"other", "url":url})).unwrap();
    let root = f.repo.root.to_str().unwrap();
    let mut p = ExecutionPrivacy {
        env: &f.env,
        json,
        covered: BTreeSet::new(),
    };

    let create = f.env.home.path().join("created-repo");
    p.execute(
        &[
            "repo",
            "create",
            "--repo",
            create.to_str().unwrap(),
            "--input",
            "-",
        ],
        include_bytes!("../fixtures/cli_input_v1/repo_create.json"),
        true,
    );
    assert!(create.join(".manyhands/config.toml").is_file());
    let enable = crate::support::born_repository();
    enable.repository.remote("origin", &url).unwrap();
    p.execute(
        &[
            "repo",
            "enable",
            "--repo",
            enable.root.to_str().unwrap(),
            "--input",
            "-",
        ],
        include_bytes!("../fixtures/cli_input_v1/repo_enable.json"),
        true,
    );
    assert!(enable.root.join(".manyhands/config.toml").is_file());
    p.execute(
        &["repo", "identity-set", "--repo", root, "--input", "-"],
        super::mutation::IDENTITY,
        true,
    );
    p.execute(
        &["remote", "add", "--repo", root, "--input", "-"],
        &remote,
        false,
    );
    p.execute(
        &["remote", "select", "--repo", root, "--input", "-"],
        br#"{"name":"publish"}"#,
        false,
    );
    p.execute(
        &["remote", "remove", "--repo", root, "--input", "-"],
        br#"{"name":"origin"}"#,
        true,
    );
    assert!(
        git2::Repository::open(&f.repo.root)
            .unwrap()
            .find_remote("origin")
            .is_err()
    );

    let (imported, request) = p.execute(
        &["key", "import", "--input", "-"],
        super::keys::IMPORT,
        false,
    );
    let key = imported["data"]["id"].as_str().unwrap();
    let source_before = credentials::key_file_states(&[
        f.env.home.path().join("imported"),
        f.env.home.path().join("imported.pub"),
    ]);
    p.execute(&["key", "select", "--key-id", key], &[], false);
    p.execute(&["key", "clear"], &[], false);
    p.execute(&["key", "remove", "--key-id", key], &[], false);
    assert_eq!(
        credentials::key_file_states(&[
            f.env.home.path().join("imported"),
            f.env.home.path().join("imported.pub")
        ]),
        source_before
    );
    let listed = p.call(&["key", "list"], &[], true, "ok", 0);
    assert_eq!(listed["data"]["items"], serde_json::json!([]));

    #[cfg(unix)]
    {
        let (generated, _) = p.execute(
            &["key", "generate", "--input", "-"],
            super::keys::GENERATE,
            false,
        );
        let key = generated["data"]["id"].as_str().unwrap();
        let directory = f.env.home.path().join(".ssh/manyhands");
        let private = directory.join(key);
        let public = directory.join(format!("{key}.pub"));
        assert!(private.is_file() && public.is_file());
        p.execute(&["key", "delete", "--key-id", key], &[], true);
        assert!(!private.exists() && !public.exists());
    }
    p.success(&["request", "show", "--request-id", &request], &[]);
    p.success(&["request", "cancel", "--request-id", &request], &[]);
    p.execute(&["repo", "remove", "--repo", root], &[], true);
    p.covered
}

fn absent(bytes: &[u8], sentinel: &str) {
    for needle in [
        sentinel.as_bytes().to_vec(),
        sentinel.encode_utf16().flat_map(u16::to_le_bytes).collect(),
    ] {
        assert!(
            !bytes.windows(needle.len()).any(|part| part == needle),
            "privacy sentinel leaked"
        );
    }
}

#[cfg(unix)]
pub fn scan_passphrase(env: &CliEnv, out: &CliRun, transcript: &str) {
    for bytes in [&out.stdout[..], &out.stderr[..], transcript.as_bytes()] {
        absent(bytes, super::prompts::PASSPHRASE);
    }
    let mut pending = vec![env.data.clone(), env.home.path().to_owned()];
    while let Some(path) = pending.pop() {
        if path.is_dir() {
            pending.extend(fs::read_dir(path).unwrap().map(|e| e.unwrap().path()));
        } else if path.is_file() {
            absent(&fs::read(path).unwrap(), super::prompts::PASSPHRASE);
        }
    }
}

#[cfg(unix)]
#[test]
fn no_passphrase_reaches_output_data_home_or_the_transcript() {
    let f = super::keys::Fixture::new();
    let (out, transcript) = super::prompts::protected_generation(&f.env);
    scan_passphrase(&f.env, &out, &transcript);
}

#[test]
fn no_url_credential_or_key_material_sentinel_reaches_any_output() {
    let f = super::mutation::Fixture::new();
    // Same inert source and URL sentinels as C1. Source files intentionally
    // contain key material; only the PTY passphrase is scanned in all files.
    fs::write(
        f.env.home.path().join("imported"),
        credentials::private_fixture_bytes(),
    )
    .unwrap();
    fs::write(
        f.env.home.path().join("imported.pub"),
        credentials::public_fixture_text(),
    )
    .unwrap();
    let imported = super::keys::call(
        &f.env,
        "import",
        &["--input", "-", "--request-id", REQUEST],
        super::keys::IMPORT,
        0,
        "ok",
    );
    let key = super::keys::key_id(&imported);
    operations::insert_key_material(
        &f.env.data,
        operations::OPERATION_A,
        "generate",
        "retained-for-inspection",
        Some("source-missing"),
    );
    let url = format!("https://user:{PASSWORD}@example.invalid/repo.git?token={QUERY}");
    f.repo.repository.remote("origin", &url).unwrap();
    let enable = crate::support::born_repository();
    enable.repository.remote("origin", &url).unwrap();
    let remote = serde_json::to_vec(&serde_json::json!({"name":"other", "url":url})).unwrap();
    let root = f.repo.root.to_str().unwrap();
    let create = f.env.home.path().join("new-repo");
    let rows: Vec<OutputCase<'_>> = vec![
        (
            "repo",
            "create",
            vec![
                "--repo",
                create.to_str().unwrap(),
                "--input",
                "-",
                "--prepare",
            ],
            include_bytes!("../fixtures/cli_input_v1/repo_create.json"),
            0,
        ),
        (
            "repo",
            "enable",
            vec![
                "--repo",
                enable.root.to_str().unwrap(),
                "--input",
                "-",
                "--prepare",
            ],
            include_bytes!("../fixtures/cli_input_v1/repo_enable.json"),
            0,
        ),
        ("repo", "remove", vec!["--repo", root, "--prepare"], b"", 0),
        (
            "repo",
            "identity-set",
            vec!["--repo", root, "--input", "-", "--prepare"],
            super::mutation::IDENTITY,
            0,
        ),
        (
            "remote",
            "add",
            vec![
                "--repo",
                root,
                "--input",
                "-",
                "--request-id",
                super::remotes::NEXT,
            ],
            &remote,
            0,
        ),
        (
            "remote",
            "remove",
            vec!["--repo", root, "--input", "-", "--prepare"],
            br#"{"name":"origin"}"#,
            0,
        ),
        (
            "remote",
            "select",
            vec!["--repo", root, "--input", "-", "--prepare"],
            br#"{"name":null}"#,
            2,
        ),
        (
            "key",
            "generate",
            vec!["--input", "-", "--request-id", REQUEST],
            super::keys::PROTECTED,
            3,
        ),
        (
            "key",
            "import",
            vec!["--input", "-", "--request-id", REQUEST],
            super::keys::IMPORT,
            0,
        ),
        ("key", "select", vec!["--key-id", key, "--prepare"], b"", 2),
        ("key", "clear", vec!["--prepare"], b"", 2),
        ("key", "remove", vec!["--key-id", key, "--prepare"], b"", 2),
        ("key", "delete", vec!["--key-id", key, "--prepare"], b"", 2),
        ("request", "show", vec!["--request-id", REQUEST], b"", 0),
        ("request", "cancel", vec!["--request-id", REQUEST], b"", 0),
    ];
    let help = crate::support::cli::json(&run(&["--help", "--json"], &f.env, &[]));
    let available: BTreeSet<_> = help["data"]["commands"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["available"] == true && c["delivered_by"] == "C2")
        .map(|c| c["command"].as_str().unwrap().to_owned())
        .collect();
    let covered: BTreeSet<_> = rows.iter().map(|(r, v, ..)| format!("{r} {v}")).collect();
    // SSH execution cannot live in this libtest target. These named cases use
    // the CLI remote helper's UTF-8/UTF-16LE output scan, including successful
    // execution in each mode; previews/refusals are not execution evidence.
    let remote = [
        (
            "host observe",
            "observe_an_unpinned_host",
            "observe_prints_the_next_commands_input_line",
        ),
        (
            "host approve",
            "approve_by_prepare_and_confirm",
            "human_host_verbs_and_noops",
        ),
        (
            "host replace",
            "replace_after_the_host_key_rotates",
            "human_host_verbs_and_noops",
        ),
    ];
    let source = include_str!("../cli_remote/hosts.rs");
    let remote_covered: BTreeSet<_> = remote
        .iter()
        .map(|(verb, json, human)| {
            for case in [json, human] {
                assert!(source.contains(&format!("fn {case}(")));
            }
            (*verb).to_owned()
        })
        .collect();
    assert!(covered.is_disjoint(&remote_covered));
    assert_eq!(
        covered
            .union(&remote_covered)
            .cloned()
            .collect::<BTreeSet<_>>(),
        available
    );
    assert_eq!(available.len(), 18);
    assert_eq!(covered.len(), rows.len());
    let mut expected_executions = covered.clone();
    if !cfg!(unix) {
        expected_executions.remove("key generate");
        expected_executions.remove("key delete");
    }
    for json in [false, true] {
        assert_eq!(
            execution_outputs(json),
            expected_executions,
            "refusals/previews do not prove execution-output privacy"
        );
    }
    for (resource, verb, flags, input, status) in rows {
        for json in [false, true] {
            let mut args = vec![resource, verb];
            args.extend_from_slice(&flags);
            if json {
                args.push("--json");
            }
            let out = run(&args, &f.env, input);
            scan_output(&out);
            assert_eq!(out.status, status, "{resource} {verb}");
            let expected_code = match (resource, verb, status) {
                ("key", "generate", _) => "unlock_required",
                ("key", "delete", _) => "key_not_deletable",
                (_, _, 2) => "not_confirmable",
                _ => "ok",
            };
            if json {
                assert_eq!(crate::support::cli::json(&out)["code"], expected_code);
            } else if status != 0 {
                assert!(out.stdout.is_empty());
                assert!(
                    std::str::from_utf8(&out.stderr)
                        .unwrap()
                        .contains(&format!("code: {expected_code}"))
                );
            }
        }
    }
}
