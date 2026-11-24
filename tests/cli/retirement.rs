//! Shipping parser/driver/replay contract. Live authentication is covered only
//! by the single cli_remote retirement case, not by these local fixtures.
use crate::support::{
    cli::{self, CliEnv},
    items,
    mutation::{REQUEST_1, REQUEST_2, REQUEST_3, World},
    schema,
};
use std::ffi::OsStr;

fn run(env: &CliEnv, args: &[&str]) -> cli::CliRun {
    cli::run_cli(&args.iter().map(OsStr::new).collect::<Vec<_>>(), env)
}

#[test]
fn retirement_command_flag_and_stopped_report_contract() {
    let world = World::new();
    let env = CliEnv::new(world.data.path().into());
    let root = world.root.to_str().unwrap();
    let base = [
        "context",
        "retire",
        "--repo",
        root,
        "--kind",
        "ticket",
        "--id",
        items::TICKET_B,
    ];
    for (extra, code, exit) in [
        (vec!["--prepare"], "not_confirmable", 2),
        (
            vec!["--request-id", REQUEST_1, "--confirm", REQUEST_2],
            "not_confirmable",
            2,
        ),
        (
            vec!["--request-id", REQUEST_1, "--input", "-"],
            "usage_error",
            2,
        ),
        (vec!["--request-id", REQUEST_1, "--force"], "usage_error", 2),
        (vec![], "usage_error", 2),
    ] {
        let mut args = base.to_vec();
        args.extend(extra);
        args.push("--json");
        let out = run(&env, &args);
        let value = cli::json(&out);
        assert_eq!(value["code"], code, "{args:?}: {value}");
        assert_eq!(out.status, exit, "{value}");
        assert_eq!(value["command"], "context retire");
    }
    for (flag, value, code) in [
        ("--kind", "folder", "usage_error"),
        ("--id", "bad", "invalid_id"),
    ] {
        let mut args = base.to_vec();
        let at = args.iter().position(|arg| *arg == flag).unwrap();
        args[at + 1] = value;
        args.extend(["--request-id", REQUEST_1, "--json"]);
        let out = run(&env, &args);
        assert_eq!(cli::json(&out)["code"], code);
        assert_eq!(out.status, 2);
    }
    for at in [2, 4, 6] {
        let mut args = base.to_vec();
        args.drain(at..at + 2);
        args.extend(["--request-id", REQUEST_1, "--json"]);
        assert_eq!(cli::json(&run(&env, &args))["code"], "usage_error");
    }
    // Standard duplicate semantics: last value wins, just as other verbs do.
    let mut args = base.to_vec();
    args.extend(["--id", items::TICKET_A, "--request-id", REQUEST_1, "--json"]);
    let out = run(&env, &args);
    let value = cli::json(&out);
    assert_eq!(out.status, 0, "{value}");
    assert_eq!(value["data"]["item_id"], items::TICKET_A);
    assert_eq!(value["data"]["state"], "already_absent");
    schema::check_published("context_retirement_mutation.schema.json", &value["data"]).unwrap();

    // No publication remote is refused before acceptance, so cannot pin a
    // request digest. A configured disposable endpoint reaches the real driver
    // and stops locally on the absent key, without network or implicit unlock.
    git2::Repository::open(&world.root)
        .unwrap()
        .remote("origin", "ssh://fixture@127.0.0.1:1/fixture")
        .unwrap();
    let config = world.root.join(".manyhands/config.toml");
    std::fs::write(
        &config,
        std::fs::read_to_string(&config).unwrap() + "publication_remote = \"origin\"\n",
    )
    .unwrap();
    items::commit(
        &world.fixture,
        &[".manyhands/config.toml"],
        items::COMMITTED_AT,
    );
    let ordinary = run(
        &env,
        &[
            "poll",
            "once",
            "--repo",
            root,
            "--request-id",
            REQUEST_2,
            "--json",
        ],
    );
    let ordinary = cli::json(&ordinary);
    assert_eq!(ordinary["code"], "no_selected_key");
    let opted = run(
        &env,
        &[
            "poll",
            "once",
            "--repo",
            root,
            "--request-id",
            REQUEST_2,
            "--retire-completed",
            "--json",
        ],
    );
    assert_eq!(
        cli::json(&opted)["code"],
        "request_mismatch",
        "mode changes never reuse an ordinary digest"
    );
    let opted_args = [
        "poll",
        "once",
        "--repo",
        root,
        "--request-id",
        REQUEST_3,
        "--retire-completed",
    ];
    let mut args = opted_args.to_vec();
    args.push("--json");
    let opted = run(&env, &args);
    let stopped = cli::json(&opted);
    assert_eq!(stopped["data"]["type"], "poll_retirement");
    assert_eq!(stopped["data"]["completion"], "stopped");
    assert_eq!(stopped["code"], "no_selected_key");
    assert_eq!(opted.status, 3);
    schema::check_published("poll_retirement_mutation.schema.json", &stopped["data"]).unwrap();
    let text = run(&env, &opted_args);
    assert_eq!(text.status, 3);
    assert!(text.stdout.is_empty());
    assert!(cli::stderr(&text).contains("as-of-pass"));
    assert!(cli::stderr(&text).contains("never resumes the loop"));
    cli::text_golden_scrubbed(
        "poll_once_retirement_stopped_text",
        &text,
        &[(root, "<repository>"), (REQUEST_3, "<request-id>")],
        &['-', ':', '.', '+'],
        |s| {
            if s.len() == 26
                && s.chars()
                    .all(|c| c.is_ascii_digit() || c.is_ascii_uppercase())
            {
                "<id>".into()
            } else if s.len() >= 20 && s.contains('T') && s.contains(':') {
                "<time>".into()
            } else {
                s.into()
            }
        },
    );

    let help = cli::json(&run(&env, &["context", "retire", "--help", "--json"]));
    assert_eq!(help["data"]["commands"][0]["delivered_by"], "C5");
    assert!(
        !help["data"]["commands"][0]["options"]
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o["name"] == "--input")
    );
}

#[test]
fn retirement_replay_index_and_preserving_binary_contract() {
    use manyhands::repository::{AuthoringKind, ContextRetireInput, FailurePoint, Mutation};
    for preserve in [false, true] {
        let world = World::with_service(|data| {
            crate::support::FailOnce::at(FailurePoint::BeforeIndexTransactionCommit)
                .open_service(data)
        });
        let env = CliEnv::new(world.data.path().into());
        let root = world.root.to_str().unwrap();
        let item = crate::support::mutation::TICKET_NEW;
        let pending = world.execute(
            REQUEST_1,
            Mutation::ContextRetire(ContextRetireInput {
                root: world.root.clone(),
                kind: AuthoringKind::Ticket,
                id: items::item_id(item),
            }),
        );
        assert_eq!(
            pending.code,
            manyhands::results::ResultCode::DiscoveryPending
        );
        // The isolated persisted fixture is an actual ordinary boundary result,
        // not SQL-fabricated deletion authority; only index/preserving is owed.
        super::mutation::golden(
            "context_retire_index_pending",
            &serde_json::to_value(&pending).unwrap(),
            &world.root,
        );
        let operation = pending.operation_id.unwrap();
        let git = git2::Repository::open(&world.root).unwrap();
        let branch = format!("refs/heads/manyhands/ticket/{item}");
        let tip = git.head().unwrap().target().unwrap();
        git.reference(&branch, tip, false, "new lifetime after index failure")
            .unwrap();
        let path = world.root.join(".manyhands/worktrees").join(item);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let reference = git.find_reference(&branch).unwrap();
        git.worktree(
            item,
            &path,
            Some(git2::WorktreeAddOptions::new().reference(Some(&reference))),
        )
        .unwrap();
        let sentinel = path.join("private-untracked");
        std::fs::write(&sentinel, b"new lifetime must survive\0full body\n").unwrap();
        let before = std::fs::read(&sentinel).unwrap();
        if preserve {
            let base = [
                "operation",
                "abandon",
                "--repo",
                root,
                "--operation-id",
                &operation,
            ];
            let mut args = base.to_vec();
            args.extend(["--prepare", "--json"]);
            let preview = cli::json(&run(&env, &args));
            assert_eq!(preview["code"], "ok", "{preview}");
            assert_eq!(
                preview["data"]["details"]["retirement"]["request_id"],
                REQUEST_1
            );
            assert_eq!(
                preview["data"]["details"]["retirement"]["branch_retained"],
                true
            );
            let mut args = base.to_vec();
            args.extend([
                "--request-id",
                REQUEST_2,
                "--confirm",
                preview["data"]["confirmation_id"].as_str().unwrap(),
                "--json",
            ]);
            let out = run(&env, &args);
            assert_eq!(out.status, 0, "{}", cli::json(&out));
        } else {
            let out = run(
                &env,
                &[
                    "operation",
                    "resume",
                    "--repo",
                    root,
                    "--operation-id",
                    &operation,
                    "--request-id",
                    REQUEST_2,
                    "--json",
                ],
            );
            assert_eq!(out.status, 0, "{}", cli::json(&out));
        }
        let args = [
            "context",
            "retire",
            "--repo",
            root,
            "--kind",
            "ticket",
            "--id",
            item,
            "--request-id",
            REQUEST_1,
            "--json",
        ];
        let replay = run(&env, &args);
        let value = cli::json(&replay);
        assert_eq!(
            value["data"]["state"],
            if preserve {
                "preserved"
            } else {
                "already_absent"
            },
            "{value}"
        );
        assert_eq!(replay.status, if preserve { 3 } else { 0 });
        assert_eq!(std::fs::read(&sentinel).unwrap(), before);
        assert_eq!(git.refname_to_id(&branch).unwrap(), tip);
        assert!(git.find_worktree(item).is_ok());
        assert_eq!(world.pending_journal_rows(), 0);
        let status = cli::json(&run(&env, &["index", "status", "--repo", root, "--json"]));
        assert_eq!(status["code"], "ok");
        let text = run(&env, &args[..args.len() - 1]);
        let said = format!("{}{}", cli::stdout(&text), cli::stderr(&text));
        assert!(said.contains("state="), "{said}");
        assert!(
            said.contains("worktree_removal=none branch_removal=none"),
            "{said}"
        );
    }
}
