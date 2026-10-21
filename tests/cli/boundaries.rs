//! Cross-command privacy, read-only, process and platform boundaries.

use std::{
    collections::{BTreeSet, HashSet},
    ffi::OsStr,
    fs,
    io::Read,
    process::Child,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use manyhands::{
    repository::LeaseKind,
    results::{FailureClass, ResultCode},
};

use crate::support::{
    self,
    cli::{CliEnv, cli_command, json, run_cli},
    credentials, golden, items, operations, repository_and_worktree_snapshot,
};

const PASSWORD: &str = "CLI-PASSWORD-SENTINEL-7a82";
const QUERY: &str = "CLI-QUERY-SENTINEL-2b43";
const GUIDANCE: &str = "CLI-GUIDANCE-SENTINEL-9c61";

#[test]
fn every_c1_verb_redacts_secrets_and_preserves_repository_in_both_modes() {
    let (fixture, enabled) = items::contract_repository();
    let data = enabled.data_directory.path();
    let env = CliEnv::new(data.to_owned()).in_dir(fixture.root.clone());
    let keys = credentials::register_keys(&enabled.service);
    credentials::pin_host(
        &enabled.service,
        "git.example.invalid",
        22,
        "ssh-ed25519",
        credentials::OTHER_FINGERPRINT,
    );
    let url = format!("https://user:{PASSWORD}@example.invalid/repo.git?token={QUERY}");
    fixture.repository.remote("origin", &url).unwrap();
    fixture
        .repository
        .remote_set_pushurl("origin", Some(&url))
        .unwrap();
    items::write_comment(
        &fixture.root,
        items::TICKET_A,
        items::COMMENT_A,
        None,
        "2026-10-18T12:00:00Z",
        "",
    );
    items::write(
        &fixture.root,
        "docs/marker.md",
        "---\nmanyhands_managed: true\n---\n",
    );
    items::commit(
        &fixture,
        &[
            "docs/marker.md",
            &items::comment_path(items::TICKET_A, items::COMMENT_A),
        ],
        items::COMMITTED_AT + 300,
    );
    // Include a real linked worktree in every snapshot, not just the primary.
    items::create_document_context(
        &enabled.service,
        &fixture.root,
        items::DOCUMENT_C,
        "docs/draft.md",
    );
    items::refresh_completely(&enabled.service, &fixture.root);
    let index = items::index(data);
    assert_eq!(
        index
            .execute(
                "INSERT INTO problems (repository_id, code, guidance, observed_at)
                 SELECT id, 'retry-required', ?1, 0 FROM repositories",
                [GUIDANCE],
            )
            .unwrap(),
        1
    );
    assert!(
        index
            .execute("UPDATE problems SET guidance = ?1", [GUIDANCE])
            .unwrap()
            > 0
    );
    index
        .execute("UPDATE repositories SET refresh_required = 1", [])
        .unwrap();
    drop(index);
    operations::insert_key_material(
        data,
        operations::OPERATION_A,
        "generate",
        "retained-for-inspection",
        Some("source-missing"),
    );

    // Setup is complete before any snapshot or CLI read. The key fixture is
    // kept alive for the whole loop, including the public-key read.
    let root = fixture.root.as_os_str();
    let key = keys.imported.id.to_string();
    let mut calls: Vec<Vec<&OsStr>> = vec![
        vec!["repo".as_ref(), "list".as_ref()],
        vec!["key".as_ref(), "list".as_ref()],
        vec![
            "key".as_ref(),
            "show".as_ref(),
            "--key-id".as_ref(),
            key.as_ref(),
        ],
        vec![
            "key".as_ref(),
            "public".as_ref(),
            "--key-id".as_ref(),
            key.as_ref(),
        ],
        vec!["host".as_ref(), "list".as_ref()],
        vec![
            "host".as_ref(),
            "inspect".as_ref(),
            "--authority".as_ref(),
            "git.example.invalid".as_ref(),
        ],
        vec!["id".as_ref(), "new".as_ref()],
    ];
    for (resource, verb, selectors) in [
        ("repo", "inspect", vec![]),
        ("repo", "identity", vec![]),
        ("remote", "list", vec![]),
        ("document", "list", vec![]),
        ("document", "show", vec!["--id", items::DOCUMENT_A]),
        ("folder", "list", vec![]),
        ("ticket", "list", vec![]),
        ("ticket", "show", vec!["--id", items::TICKET_A]),
        ("ticket", "ready", vec![]),
        ("ticket", "blocked", vec![]),
        ("ticket", "deps", vec!["--id", items::TICKET_A]),
        ("ticket", "children", vec!["--id", items::TICKET_B]),
        ("ticket", "cycles", vec![]),
        ("ticket", "plan", vec![]),
        ("ticket", "critical-path", vec![]),
        ("ticket", "find", vec!["--slug", items::SHARED_SLUG]),
        ("comment", "list", vec!["--id", items::TICKET_A]),
        ("index", "status", vec![]),
        ("poll", "status", vec![]),
        ("operation", "list", vec![]),
        (
            "operation",
            "show",
            vec!["--operation-id", operations::OPERATION_A],
        ),
    ] {
        let mut args = vec![resource.as_ref(), verb.as_ref(), "--repo".as_ref(), root];
        args.extend(selectors.into_iter().map(OsStr::new));
        calls.push(args);
    }
    let before = repository_and_worktree_snapshot(&fixture);
    assert_eq!(before.linked_worktrees.len(), 1);
    let help = run_cli(&["--help".as_ref(), "--json".as_ref()], &env);
    assert_eq!(before, repository_and_worktree_snapshot(&fixture));
    assert_eq!(help.status, 0);
    let help = json(&help);
    let available: BTreeSet<_> = help["data"]["commands"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["available"] == true)
        .map(|entry| entry["command"].as_str().unwrap().to_owned())
        .collect();
    let covered: BTreeSet<_> = calls
        .iter()
        .map(|args| {
            format!(
                "{} {}",
                args[0].to_str().unwrap(),
                args[1].to_str().unwrap()
            )
        })
        .collect();
    assert_eq!(calls.len(), 28);
    assert_eq!(
        covered.len(),
        calls.len(),
        "duplicate invocation hides a missing verb"
    );
    assert_eq!(
        covered, available,
        "boundary coverage must match the actual CLI catalog"
    );

    let mut sentinels = vec![PASSWORD, QUERY, GUIDANCE, operations::KEY_MATERIAL_SENTINEL];
    sentinels.extend(credentials::SECRET_SENTINELS);
    for args in calls {
        for json_mode in [false, true] {
            let mut args = args.clone();
            if json_mode {
                args.push("--json".as_ref());
            }
            let before = repository_and_worktree_snapshot(&fixture);
            let run = run_cli(&args, &env);
            let after = repository_and_worktree_snapshot(&fixture);
            assert_eq!(before, after, "repository changed: {args:?}");
            assert_eq!(
                run.status,
                0,
                "{args:?}: {} {}",
                String::from_utf8_lossy(&run.stdout),
                String::from_utf8_lossy(&run.stderr)
            );
            for (stream, bytes) in [("stdout", &run.stdout), ("stderr", &run.stderr)] {
                let text = std::str::from_utf8(bytes).unwrap();
                for sentinel in &sentinels {
                    assert!(
                        !text.contains(sentinel),
                        "{args:?}: {stream} leaked {sentinel}"
                    );
                }
            }
            if json_mode {
                let value = json(&run);
                assert_eq!(value["code"], "ok", "{args:?}");
                assert_eq!(golden::find_sentinel(&value, &sentinels), None, "{args:?}");
            }
        }
    }
}

/// Reap even when an assertion or pipe read fails; Child's own Drop does not.
struct ReapedChild(Child);

impl Drop for ReapedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn broken_stdout_pipe_exits_one_without_panic_or_lingering_process() {
    let (fixture, enabled) = items::contract_repository();
    let source =
        items::document_source(items::DOCUMENT_A, "Large guide", "") + &"x".repeat(4 * 1024 * 1024);
    items::write(&fixture.root, "docs/guide.md", &source);
    items::commit(&fixture, &["docs/guide.md"], items::COMMITTED_AT + 300);
    items::refresh_completely(&enabled.service, &fixture.root);
    let env = CliEnv::new(enabled.data_directory.path().to_owned());
    let mut child = ReapedChild(
        cli_command(
            &[
                "document".as_ref(),
                "show".as_ref(),
                "--repo".as_ref(),
                fixture.root.as_os_str(),
                "--id".as_ref(),
                items::DOCUMENT_A.as_ref(),
            ],
            &env,
        )
        .spawn()
        .unwrap(),
    );
    let mut stdout = child.0.stdout.take().unwrap();
    let mut stderr = child.0.stderr.take().unwrap();
    let (prefix_tx, prefix_rx) = mpsc::channel();
    let prefix_reader = thread::spawn(move || {
        let mut prefix = [0; 16];
        let result = stdout.read_exact(&mut prefix);
        drop(stdout);
        let _ = prefix_tx.send(result);
    });
    let (stderr_tx, stderr_rx) = mpsc::channel();
    let stderr_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = stderr.read_to_end(&mut bytes).map(|_| bytes);
        let _ = stderr_tx.send(result);
    });
    // The prefix read is on a reader thread so even a child that never writes
    // cannot block the test past this deadline. A timeout kills and reaps it.
    let deadline = Instant::now() + Duration::from_secs(30);
    let status = loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            break status;
        }
        assert!(
            Instant::now() < deadline,
            "broken-pipe CLI exceeded 30 seconds"
        );
        thread::sleep(Duration::from_millis(5));
    };
    let eof_deadline = Instant::now() + Duration::from_secs(1);
    prefix_rx
        .recv_timeout(eof_deadline.saturating_duration_since(Instant::now()))
        .expect("stdout prefix reader did not finish")
        .unwrap();
    let stderr = stderr_rx
        .recv_timeout(eof_deadline.saturating_duration_since(Instant::now()))
        .expect("stderr did not reach EOF after CLI exit")
        .unwrap();
    prefix_reader.join().unwrap();
    stderr_reader.join().unwrap();
    assert_eq!(status.code(), Some(1));
    assert!(!String::from_utf8_lossy(&stderr).contains("panicked"));
    assert!(
        stderr.is_empty(),
        "output failure must not print another diagnostic"
    );
}

// R-3: Windows uses the known-folder API, which ignores LOCALAPPDATA; a
// platform-default invocation there would touch real application data.
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn platform_default_data_directory_is_created_only_under_the_temporary_home() {
    let unused = tempfile::tempdir().unwrap();
    let env = CliEnv::new(unused.path().join("override-not-used")).without_data_dir();
    // cli_command sets XDG_DATA_HOME to <home>/data on Linux and HOME to
    // <home> on macOS, in the child only. Do not mutate the test process env.
    #[cfg(target_os = "linux")]
    let expected = env.home.path().join("data/manyhands");
    #[cfg(target_os = "macos")]
    let expected = env
        .home
        .path()
        .join("Library/Application Support/com.manyhands.Manyhands");
    assert!(!expected.exists());
    let run = run_cli(&["repo".as_ref(), "list".as_ref(), "--json".as_ref()], &env);
    assert_eq!(run.status, 0);
    assert_eq!(json(&run)["code"], "ok");
    assert!(expected.is_dir(), "{}", expected.display());
    assert!(
        expected
            .join(manyhands::repository::REGISTRY_FILE)
            .is_file()
    );
    assert!(!env.data.exists());
}

#[test]
fn exit_class_table_runs_every_reachable_class_through_the_binary() {
    let (fixture, enabled) = items::contract_repository();
    let env = CliEnv::new(enabled.data_directory.path().to_owned());
    let file = env.home.path().join("regular-file-data-directory");
    fs::write(&file, b"not a directory").unwrap();
    let broken_env = CliEnv::new(file);
    let unregistered = support::born_repository();
    let root = fixture.root.to_str().unwrap();
    let rows = [
        (vec!["repo", "list"], ResultCode::Ok, 0),
        (vec!["unknown"], ResultCode::UsageError, 2),
        (vec!["ticket", "close"], ResultCode::CommandUnavailable, 2),
        (
            vec!["ticket", "show", "--repo", root, "--id", "bad"],
            ResultCode::InvalidId,
            2,
        ),
        (
            vec![
                "repo",
                "identity",
                "--repo",
                unregistered.root.to_str().unwrap(),
            ],
            ResultCode::RepositoryNotRegistered,
            3,
        ),
        (vec!["index", "status", "--repo", root], ResultCode::Busy, 5),
        (vec!["repo", "list"], ResultCode::InternalError, 1),
    ];
    let mut covered = HashSet::new();
    for (args, code, exit) in rows {
        // Hold a genuine exclusive lease in a second process only for Busy.
        let holder = (code == ResultCode::Busy).then(|| {
            support::hold_lease_in_child_for_test(
                &fixture.root,
                enabled.data_directory.path(),
                LeaseKind::CacheWrite,
                "index_lease_child",
            )
        });
        let env = if code == ResultCode::InternalError {
            &broken_env
        } else {
            &env
        };
        for json_mode in [false, true] {
            let mut args: Vec<_> = args.iter().map(OsStr::new).collect();
            if json_mode {
                args.push("--json".as_ref());
            }
            let run = run_cli(&args, env);
            assert_eq!(
                run.status,
                exit,
                "{args:?}: {} {}",
                String::from_utf8_lossy(&run.stdout),
                String::from_utf8_lossy(&run.stderr)
            );
            if json_mode {
                let value = json(&run);
                assert_eq!(value["code"], code.as_str(), "{args:?}");
                assert_eq!(value["message"], code.message());
            } else if code == ResultCode::Ok {
                assert!(!run.stdout.is_empty());
                assert!(run.stderr.is_empty());
            } else {
                assert!(run.stdout.is_empty());
                let stderr = std::str::from_utf8(&run.stderr).unwrap();
                assert!(stderr.contains(&format!("\ncode: {}\n", code.as_str())));
                assert!(!stderr.contains("panicked"));
            }
        }
        if let Some(holder) = holder {
            holder.release();
        }
        covered.insert(code.failure_class());
    }
    assert_eq!(
        covered,
        HashSet::from([
            None,
            Some(FailureClass::Input),
            Some(FailureClass::Blocked),
            Some(FailureClass::Transient),
            Some(FailureClass::Internal),
        ])
    );
}
