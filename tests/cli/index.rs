use std::{collections::BTreeSet, fs, path::Path, process::Command};

use serde_json::Value;

use super::mutation::{self, REQUEST, golden};
use crate::support::{
    self,
    cli::{self, CliEnv},
    items,
};

pub(super) const REQUEST_2: &str = "01M4CC0VMTTNKVYS6XWQG4CK1P";

pub(super) struct Fixture {
    pub repo: support::TestRepository,
    pub env: CliEnv,
    pub data: tempfile::TempDir,
}

impl Fixture {
    pub fn new() -> Self {
        let (repo, enabled) = items::contract_repository();
        let env = CliEnv::new(enabled.data_directory.path().to_owned());
        fs::write(
            env.home.path().join(".gitconfig"),
            "[core]\n\tautocrlf = false\n",
        )
        .unwrap();
        // No service remains open when a child runs or cache files are replaced.
        drop(enabled.service);
        Self {
            repo,
            env,
            data: enabled.data_directory,
        }
    }

    pub fn args<'a>(
        &'a self,
        resource: &'a str,
        verb: &'a str,
        extra: &'a [&'a str],
    ) -> Vec<&'a str> {
        let mut args = vec![resource, verb, "--repo", self.repo.root.to_str().unwrap()];
        args.extend_from_slice(extra);
        args
    }

    pub fn checked(
        &self,
        resource: &str,
        verb: &str,
        extra: &[&str],
        input: &[u8],
        status: i32,
        code: &str,
    ) -> Value {
        let mut args = self.args(resource, verb, extra);
        args.push("--json");
        mutation::checked(&args, &self.env, input, status, code, None)
    }

    pub fn index(&self, verb: &str, request: &str) -> Value {
        let value = self.checked("index", verb, &["--request-id", request], &[], 0, "ok");
        assert_eq!(value["effects"]["discovery"], "current");
        assert!(value["data"].is_null());
        assert_eq!(value["request_id"], request);
        value
    }

    pub fn refs(&self, pattern: &str) -> Vec<u8> {
        let mut command = Command::new("git");
        cli::cli_environment(&mut command, &self.env);
        let output = command
            .arg("-C")
            .arg(&self.repo.root)
            .arg("for-each-ref")
            .arg(pattern)
            .output()
            .unwrap();
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        output.stdout
    }

    pub fn request(&self, request: &str) -> Value {
        mutation::checked(
            &["request", "show", "--request-id", request, "--json"],
            &self.env,
            &[],
            0,
            "ok",
            Some("request.schema.json"),
        )
    }
}

pub(super) fn assert_stored_result(record: &Value, envelope: &Value) {
    assert_eq!(record["data"]["state"], "finished");
    for key in ["outcome", "code", "message", "effects", "data"] {
        assert_eq!(record["data"]["result"][key], envelope[key]);
    }
}

#[test]
fn index_refresh_and_rebuild_succeed_and_change_nothing_canonical() {
    let f = Fixture::new();
    let before = support::repository_and_worktree_snapshot(&f.repo);
    let refs = f.refs("refs/");
    for (verb, request) in [("refresh", REQUEST), ("rebuild", REQUEST_2)] {
        let value = f.index(verb, request);
        golden(&format!("index_{verb}_ok"), &value, &f.repo.root);
        assert_eq!(support::repository_and_worktree_snapshot(&f.repo), before);
        assert_eq!(f.refs("refs/"), refs);
    }
}

#[test]
fn index_verbs_never_fetch() {
    let f = Fixture::new();
    let remote_dir = tempfile::tempdir().unwrap();
    let bare = git2::Repository::init_bare(remote_dir.path()).unwrap();
    let tree = bare
        .find_tree(bare.treebuilder(None).unwrap().write().unwrap())
        .unwrap();
    let signature = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
    bare.commit(
        Some("refs/heads/remote-only"),
        &signature,
        &signature,
        "remote only",
        &tree,
        &[],
    )
    .unwrap();
    f.repo
        .repository
        .remote("origin", remote_dir.path().to_str().unwrap())
        .unwrap();
    assert!(
        f.repo
            .repository
            .find_reference("refs/remotes/origin/remote-only")
            .is_err()
    );
    let refs = f.refs("refs/remotes/");
    let before = support::repository_and_worktree_snapshot(&f.repo);
    for (verb, request) in [("refresh", REQUEST), ("rebuild", REQUEST_2)] {
        f.index(verb, request);
        assert_eq!(f.refs("refs/remotes/"), refs);
        assert_eq!(support::repository_and_worktree_snapshot(&f.repo), before);
    }
}

#[test]
fn index_verbs_need_a_request_id_without_a_terminal() {
    let f = Fixture::new();
    for verb in ["refresh", "rebuild"] {
        let value = f.checked("index", verb, &[], &[], 2, "usage_error");
        assert!(value["request_id"].is_null());
        golden(&format!("index_{verb}_usage_error"), &value, &f.repo.root);
        let human = mutation::run(&f.args("index", verb, &[]), &f.env, &[]);
        assert_eq!(human.status, 2);
        assert!(human.stdout.is_empty());
        assert!(
            String::from_utf8(human.stderr)
                .unwrap()
                .contains("code: usage_error")
        );
    }
}

fn remove_sidecars(data: &Path) {
    for name in ["manyhands.sqlite3-wal", "manyhands.sqlite3-shm"] {
        match fs::remove_file(data.join(name)) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("remove SQLite sidecar: {error}"),
        }
    }
}

fn restored_lists(f: &Fixture) {
    for (resource, expected) in [
        ("document", [items::DOCUMENT_A, items::DOCUMENT_B]),
        ("ticket", [items::TICKET_A, items::TICKET_B]),
    ] {
        let value = f.checked(resource, "list", &[], &[], 0, "ok");
        assert_eq!(value["data"]["complete"], true);
        let actual: BTreeSet<_> = value["data"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["id"].as_str().unwrap())
            .collect();
        assert_eq!(actual, BTreeSet::from(expected));
    }
}

fn rebuild_after_cache_loss(corrupt: bool) {
    let f = Fixture::new();
    let before = support::repository_and_worktree_snapshot(&f.repo);
    let refs = f.refs("refs/");
    remove_sidecars(f.data.path());
    let database = f.data.path().join("manyhands.sqlite3");
    if corrupt {
        fs::write(database, [0xA5; 4096]).unwrap();
    } else {
        fs::remove_file(database).unwrap();
    }
    let code = if corrupt {
        "index_unavailable"
    } else {
        "repository_not_registered"
    };
    let failed = f.checked("document", "list", &[], &[], 3, code);
    if corrupt {
        assert!(
            failed["recovery"]
                .as_array()
                .unwrap()
                .iter()
                .any(|action| action["action"] == "index.rebuild")
        );
    } else {
        assert_eq!(failed["recovery"], serde_json::json!([]));
    }
    f.index("rebuild", REQUEST);
    restored_lists(&f);
    assert_eq!(support::repository_and_worktree_snapshot(&f.repo), before);
    assert_eq!(f.refs("refs/"), refs);
}

#[test]
fn index_rebuild_restores_a_deleted_database() {
    rebuild_after_cache_loss(false);
}

#[test]
fn index_rebuild_restores_a_database_replaced_by_other_bytes() {
    rebuild_after_cache_loss(true);
}

#[test]
fn index_verbs_replay_a_finished_request() {
    for verb in ["refresh", "rebuild"] {
        let f = Fixture::new();
        let first = f.index(verb, REQUEST);
        let record = f.request(REQUEST);
        assert_stored_result(&record, &first);
        // A second discovery pass would pick this up. This proves replay does
        // not repeat the effect, independent of the refresh clock's resolution.
        items::write(
            &f.repo.root,
            "docs/not-indexed.md",
            &items::document_source(items::DOCUMENT_C, "Not indexed", ""),
        );
        let status = f.checked("index", "status", &[], &[], 0, "ok");
        let before = support::repository_and_worktree_snapshot(&f.repo);
        assert_eq!(f.index(verb, REQUEST), first);
        assert_eq!(f.request(REQUEST), record);
        assert_eq!(f.checked("index", "status", &[], &[], 0, "ok"), status);
        restored_lists(&f);
        assert_eq!(support::repository_and_worktree_snapshot(&f.repo), before);
    }
}

#[test]
fn index_verbs_refuse_input_and_id() {
    let f = Fixture::new();
    for verb in ["refresh", "rebuild"] {
        for extra in [["--input", "-"], ["--id", items::DOCUMENT_A]] {
            let value = f.checked(
                "index",
                verb,
                &["--request-id", REQUEST, extra[0], extra[1]],
                &[],
                2,
                "usage_error",
            );
            assert!(value["request_id"].is_null());
        }
    }
}

#[test]
fn index_verbs_have_human_output() {
    let f = Fixture::new();
    for (verb, request) in [("refresh", REQUEST), ("rebuild", REQUEST_2)] {
        let output = mutation::run(
            &f.args("index", verb, &["--request-id", request]),
            &f.env,
            &[],
        );
        assert_eq!(
            output.status,
            0,
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let replay = f.index(verb, request);
        cli::assert_cli_text_golden(
            &format!("index_{verb}_text"),
            std::str::from_utf8(&output.stdout).unwrap(),
            &[
                (request, "<request-id>"),
                (replay["operation_id"].as_str().unwrap(), "<operation-id>"),
                (
                    f.repo.root.canonicalize().unwrap().to_str().unwrap(),
                    "<repository>",
                ),
            ],
        );
    }
}
