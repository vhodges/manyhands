use super::mutation::{REQUEST, run};
#[cfg(unix)]
use crate::support::cli::{CliEnv, CliRun};
use crate::support::{credentials, operations};
use std::{collections::BTreeSet, fs};

type OutputCase<'a> = (&'a str, &'a str, Vec<&'a str>, &'a [u8], i32);

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
    const PASSWORD: &str = "CLI-PASSWORD-SENTINEL-7a82";
    const QUERY: &str = "CLI-QUERY-SENTINEL-2b43";
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
    assert_eq!(covered, available);
    assert_eq!(covered.len(), rows.len());
    for (resource, verb, flags, input, status) in rows {
        for json in [false, true] {
            let mut args = vec![resource, verb];
            args.extend_from_slice(&flags);
            if json {
                args.push("--json");
            }
            let out = run(&args, &f.env, input);
            assert_eq!(out.status, status, "{resource} {verb}");
            for bytes in [&out.stdout, &out.stderr] {
                for sentinel in [PASSWORD, QUERY, operations::KEY_MATERIAL_SENTINEL]
                    .into_iter()
                    .chain(credentials::SECRET_SENTINELS)
                {
                    absent(bytes, sentinel);
                }
            }
        }
    }
}
