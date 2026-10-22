//! C2's audited code inventory. Staged evidence is deliberately not proof:
//! Tasks 8–10 must replace each staged entry with binary/SSH rows.
use super::{
    mutation::{Fixture, IDENTITY, REMOTE, REQUEST, checked},
    remotes::{self, NEXT, SELECT},
};
use crate::support::{self, cli::CliEnv};
use manyhands::repository::LeaseKind;
use std::{
    collections::BTreeSet,
    fs,
    path::Path,
    time::{Duration, Instant},
};

// Exact Task 0 audit union: 49 F2 codes, followed by the three CLI additions.
const AUDIT_F2: &str = "already_applied bare_repository busy cancelled confirmation_expired confirmation_mismatch confirmation_not_found confirmation_required confirmation_used discovery_pending external_change host_mismatch host_replacement_required identity_required index_unavailable internal_error invalid_configuration invalid_input invalid_path invalid_remote key_not_deletable key_not_found key_rejected key_unavailable no_selected_key not_confirmable not_repository not_repository_root occupied_path ok publication_remote_in_use publication_remote_required push_rejected recovery_required registration_pending remote_name_conflict remote_unavailable repository_inaccessible repository_not_enabled repository_not_registered request_mismatch request_not_found selected_key_in_use transport_unavailable unlock_failed unlock_required worktree_conflicted worktree_not_clean wrong_branch";
const CLI_ONLY: &[&str] = &["usage_error", "command_unavailable", "invalid_id"];

// No host_approval_required: the audit explicitly excludes it from C2.
const REASONS: &[(&str, &str)] = &[
    (
        "confirmation_expired",
        "Needs ten minutes; F2 covers expiry with an injected clock, unavailable to the CLI child.",
    ),
    (
        "registration_pending",
        "Needs an injected registration failure in the CLI child; F2 failure seams are library-only.",
    ),
    (
        "discovery_pending",
        "Needs an injected discovery failure in the CLI child; F2 failure seams are library-only.",
    ),
    (
        "remote_unavailable",
        "No deterministic fixture produces this transport code, as F2 records.",
    ),
    (
        "push_rejected",
        "Defensive host error mapping; C2 host verification never pushes.",
    ),
];

const STAGED: &[(&str, u8, &str)] = &[
    #[cfg(not(unix))]
    (
        "key_unavailable",
        10,
        "Windows: missing-selected-source host observe; Task 8's generated-key deletion fixture is Unix-only",
    ),
    (
        "host_mismatch",
        10,
        "SSH approval/replacement presented fingerprint mismatch",
    ),
    (
        "host_replacement_required",
        10,
        "Approval prepare over another pin",
    ),
    (
        "publication_remote_required",
        10,
        "Observe without a publication remote",
    ),
    (
        "no_selected_key",
        10,
        "Observe pinned host without a selected key",
    ),
    (
        "key_rejected",
        10,
        "SSH server rejects an unprotected selected key",
    ),
    (
        "unlock_failed",
        10,
        "SSH server rejects the correctly unlocked key",
    ),
    (
        "transport_unavailable",
        10,
        "Wrong-passphrase SSH case, current library contract",
    ),
];

enum How {
    Here,
    // Task 10 installs real function references, then this variant is constructed.
    #[allow(dead_code)]
    Remote(&'static str),
    Prompt(&'static str),
}
struct Row {
    name: &'static str,
    args: &'static [&'static str],
    input: &'static [u8],
    code: &'static str,
    exit: i32,
    how: How,
}
const ADD: &[&str] = &[
    "remote",
    "add",
    "--repo",
    "{root}",
    "--input",
    "-",
    "--request-id",
    NEXT,
];
const SELECT_ARGS: &[&str] = &[
    "remote",
    "select",
    "--repo",
    "{root}",
    "--input",
    "-",
    "--request-id",
    NEXT,
];
const IDENTITY_ARGS: &[&str] = &[
    "repo",
    "identity-set",
    "--repo",
    "{root}",
    "--input",
    "-",
    "--request-id",
    NEXT,
    "--confirm",
    "{confirmation}",
];
const CREATE: &[&str] = &[
    "repo",
    "create",
    "--repo",
    "{root}",
    "--input",
    "-",
    "--prepare",
];
const REMOVE: &[&str] = &[
    "remote",
    "remove",
    "--repo",
    "{root}",
    "--input",
    "-",
    "--prepare",
];
const ONBOARD: &[u8] = include_bytes!("../fixtures/cli_input_v1/repo_create.json");
macro_rules! row {
    ($name:literal, $args:expr, $input:expr, $code:literal, $exit:literal) => {
        Row {
            name: $name,
            args: $args,
            input: $input,
            code: $code,
            exit: $exit,
            how: How::Here,
        }
    };
}
const ROWS: &[Row] = &[
    row!("protected_generation", &["key", "generate", "--input", "-", "--request-id", REQUEST], super::keys::PROTECTED, "unlock_required", 3),
    row!("imported_delete", &["key", "delete", "--key-id", "{key}", "--prepare"], b"", "key_not_deletable", 2),
    row!("selected_key_remove", &["key", "remove", "--key-id", "{key}", "--request-id", super::remotes::LAST], b"", "selected_key_in_use", 3),
    row!("unknown_key", &["key", "select", "--key-id", REQUEST, "--request-id", NEXT], b"", "key_not_found", 2),
    #[cfg(unix)]
    row!("unsafe_key_store_lock", &["key", "delete", "--key-id", "{key}", "--request-id", NEXT, "--confirm", "{confirmation}"], b"", "key_unavailable", 3),
    row!("success", ADD, REMOTE, "ok", 0),
    row!("selected_noop", SELECT_ARGS, SELECT, "already_applied", 0),
    row!("usage", &["remote", "add"], b"", "usage_error", 2),
    row!(
        "unavailable",
        &["ticket", "close"],
        b"",
        "command_unavailable",
        2
    ),
    row!("input", ADD, b"{}", "invalid_input", 2),
    row!("prefix", CREATE, br#"{"primary_branch":"main","identity":{"name":"Person","email":"person@example.invalid"},"ticket_slug_prefix":"invalid prefix!"}"#, "invalid_input", 2),
    row!("initials", &["repo", "identity-set", "--repo", "{root}", "--input", "-", "--prepare"], br#"{"name":"Person","email":"person@example.invalid","initials":"invalid initials!"}"#, "invalid_input", 2),
    row!(
        "id",
        &["request", "show", "--request-id", "bad"],
        b"",
        "invalid_id",
        2
    ),
    row!(
        "unconfirmed_prepare",
        &[
            "remote",
            "add",
            "--repo",
            "{root}",
            "--input",
            "-",
            "--prepare"
        ],
        REMOTE,
        "not_confirmable",
        2
    ),
    row!(
        "finished_mismatch",
        ADD,
        br#"{"name":"other","url":"ssh://git@example.invalid/other.git"}"#,
        "request_mismatch",
        2
    ),
    row!(
        "confirmation_mismatch",
        IDENTITY_ARGS,
        br#"{"name":"Other","email":"other@example.invalid"}"#,
        "confirmation_mismatch",
        2
    ),
    row!(
        "request_missing",
        &["request", "show", "--request-id", NEXT],
        b"",
        "request_not_found",
        2
    ),
    row!("occupied", CREATE, ONBOARD, "occupied_path", 2),
    row!("path", CREATE, ONBOARD, "invalid_path", 2),
    row!(
        "name_conflict",
        ADD,
        br#"{"name":"origin","url":"ssh://git@other.invalid/repo.git"}"#,
        "remote_name_conflict",
        2
    ),
    row!(
        "remote",
        ADD,
        br#"{"name":"origin","url":""}"#,
        "invalid_remote",
        2
    ),
    row!("not_repository", ADD, REMOTE, "not_repository", 2),
    row!("not_root", ADD, REMOTE, "not_repository_root", 2),
    row!("bare", ADD, REMOTE, "bare_repository", 2),
    row!("not_enabled", ADD, REMOTE, "repository_not_enabled", 2),
    row!(
        "confirmation_required",
        &[
            "repo",
            "identity-set",
            "--repo",
            "{root}",
            "--input",
            "-",
            "--request-id",
            NEXT
        ],
        IDENTITY,
        "confirmation_required",
        3
    ),
    row!(
        "confirmation_missing",
        IDENTITY_ARGS,
        IDENTITY,
        "confirmation_not_found",
        3
    ),
    row!(
        "confirmation_used",
        IDENTITY_ARGS,
        IDENTITY,
        "confirmation_used",
        3
    ),
    row!("stale", IDENTITY_ARGS, IDENTITY, "external_change", 3),
    row!("identity", SELECT_ARGS, SELECT, "identity_required", 3),
    row!(
        "publication",
        REMOVE,
        SELECT,
        "publication_remote_in_use",
        3
    ),
    row!("dirty", SELECT_ARGS, SELECT, "worktree_not_clean", 3),
    row!("branch", SELECT_ARGS, SELECT, "wrong_branch", 3),
    row!(
        "configuration",
        SELECT_ARGS,
        SELECT,
        "invalid_configuration",
        3
    ),
    row!("registration", ADD, REMOTE, "repository_not_registered", 3),
    row!(
        "inaccessible",
        &["repo", "remove", "--repo", "{root}", "--prepare"],
        b"",
        "repository_inaccessible",
        3
    ),
    row!("index", ADD, REMOTE, "index_unavailable", 3),
    row!("recovery", SELECT_ARGS, SELECT, "recovery_required", 3),
    row!(
        "conflicted",
        &[
            "repo",
            "enable",
            "--repo",
            "{root}",
            "--input",
            "-",
            "--prepare"
        ],
        ONBOARD,
        "worktree_conflicted",
        3
    ),
    row!(
        "partial_mismatch",
        ADD,
        br#"{"name":"origin","url":"ssh://git@other.invalid/repo.git"}"#,
        "request_mismatch",
        4
    ),
    row!("busy", ADD, REMOTE, "busy", 5),
    row!("internal", ADD, REMOTE, "internal_error", 1),
    Row {
        name: "cancelled",
        args: &[],
        input: b"",
        code: "cancelled",
        exit: 130,
        how: How::Prompt("ctrl_c_at_consent_is_cancelled_and_restores_the_terminal"),
    },
];

#[test]
fn every_run_here_row_reaches_its_code_and_exit_through_the_binary() {
    for row in ROWS.iter().filter(|row| matches!(row.how, How::Here)) {
        let f = Fixture::new();
        let mut root = f.repo.root.clone();
        let mut confirmation = REQUEST.to_owned();
        let mut alternate_env = None;
        let mut other_repo = None;
        let mut key = String::new();
        let mut key_files = Vec::new();
        let mut key_before = Vec::new();
        #[cfg(unix)]
        let mut key_registration = None;
        match row.name {
            #[cfg(unix)]
            "unsafe_key_store_lock" => {
                use manyhands::repository::{
                    RepositoryService,
                    keys::{KeyMaterialErrorKind, KeyStore},
                };
                let generated = super::keys::generate(&f.env);
                key = generated.id.to_string();
                key_registration = Some(super::keys::call(
                    &f.env,
                    "show",
                    &["--key-id", &key],
                    &[],
                    0,
                    "ok",
                ));
                key_files = vec![
                    generated.private_key_path,
                    generated.public_key_path.unwrap(),
                ];
                let preview = super::keys::call(
                    &f.env,
                    "delete",
                    &["--key-id", &key, "--prepare"],
                    &[],
                    0,
                    "ok",
                );
                confirmation = preview["data"]["confirmation_id"]
                    .as_str()
                    .unwrap()
                    .to_owned();
                let sentinel = f.env.home.path().join("lock-sentinel");
                fs::write(&sentinel, b"untouched lock target").unwrap();
                key_files.push(sentinel.clone());
                key_before = support::credentials::key_file_states(&key_files);
                let lock = f.env.home.path().join(".ssh/manyhands/.lock");
                fs::remove_file(&lock).unwrap();
                std::os::unix::fs::symlink(&sentinel, &lock).unwrap();
                // Prove the errno and public-domain cause, not just the final code.
                use std::os::unix::fs::OpenOptionsExt;
                let error = fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .custom_flags(libc::O_NOFOLLOW)
                    .open(&lock)
                    .unwrap_err();
                assert_eq!(error.raw_os_error(), Some(libc::ELOOP));
                let service = RepositoryService::open_at(&f.env.data).unwrap();
                let error = service
                    .review_generated_key_deletion(
                        &KeyStore::for_home(f.env.home.path()).unwrap(),
                        generated.id,
                    )
                    .unwrap_err();
                assert_eq!(error.kind, KeyMaterialErrorKind::UnsafePath);
            }
            "imported_delete" | "selected_key_remove" => {
                use crate::support::credentials;
                key_files = ["imported", "imported.pub"]
                    .map(|p| f.env.home.path().join(p))
                    .to_vec();
                fs::write(&key_files[0], credentials::private_fixture_bytes()).unwrap();
                fs::write(&key_files[1], credentials::public_fixture_text()).unwrap();
                key_before = credentials::key_file_states(&key_files);
                let imported = super::keys::call(
                    &f.env,
                    "import",
                    &["--input", "-", "--request-id", REQUEST],
                    super::keys::IMPORT,
                    0,
                    "ok",
                );
                key = super::keys::key_id(&imported).to_owned();
                if row.name == "selected_key_remove" {
                    super::keys::call(
                        &f.env,
                        "select",
                        &["--key-id", &key, "--request-id", NEXT],
                        &[],
                        0,
                        "ok",
                    );
                }
            }
            "selected_noop" | "publication" => {
                remotes::add(&f);
                remotes::select(&f);
            }
            "name_conflict" => remotes::add(&f),
            "finished_mismatch" => {
                remotes::call(&f, "add", REMOTE, &["--request-id", NEXT], 0, "ok");
            }
            "confirmation_mismatch" | "confirmation_used" | "stale" => {
                let p = f.prepare();
                confirmation = p["data"]["confirmation_id"].as_str().unwrap().to_owned();
                if row.name == "confirmation_used" {
                    f.execute(&confirmation);
                }
                if row.name == "stale" {
                    git2::Config::open(&f.repo.repository.path().join("config"))
                        .unwrap()
                        .set_str("user.name", "Changed externally")
                        .unwrap();
                }
            }
            "path" => {
                root = f.env.home.path().join("absent-parent/target");
            }
            "prefix" => root = f.env.home.path().join("new-repository"),
            "not_repository" => {
                root = f.env.home.path().to_owned();
            }
            "not_root" => {
                root = root.join("nested");
                fs::create_dir(&root).unwrap();
            }
            "bare" => {
                root = f.env.home.path().join("bare");
                other_repo = Some(git2::Repository::init_bare(&root).unwrap());
            }
            "not_enabled" => {
                root = f.env.home.path().join("plain");
                other_repo = Some(git2::Repository::init(&root).unwrap());
            }
            "identity" | "dirty" | "branch" | "configuration" => {
                remotes::add(&f);
                match row.name {
                    "identity" => remotes::remove_identity(&f),
                    "dirty" => {
                        let p = root.join(".manyhands/config.toml");
                        let mut bytes = fs::read(&p).unwrap();
                        bytes.extend_from_slice(b"\n# external edit\n");
                        fs::write(p, bytes).unwrap();
                    }
                    "configuration" => {
                        fs::write(root.join(".manyhands/config.toml"), "invalid = [").unwrap()
                    }
                    "branch" => {
                        let commit = f.repo.repository.head().unwrap().peel_to_commit().unwrap();
                        f.repo.repository.branch("other", &commit, false).unwrap();
                        f.repo.repository.set_head("refs/heads/other").unwrap();
                    }
                    _ => unreachable!(),
                }
            }
            "registration" => {
                alternate_env = Some(CliEnv::new(f.env.home.path().join("other-data")))
            }
            "inaccessible" => fs::write(root.join(".git/config"), "[invalid\n").unwrap(),
            "index" => {
                // Same persisted degraded-index fixture as C1's items::degraded_service.
                for entry in fs::read_dir(&f.env.data).unwrap() {
                    let path = entry.unwrap().path();
                    let name = path.file_name().unwrap().to_str().unwrap();
                    if name.starts_with("manyhands.sqlite3") && !name.ends_with(".lock") {
                        fs::remove_file(path).unwrap();
                    }
                }
                fs::write(f.env.data.join("manyhands.sqlite3"), b"not sqlite").unwrap();
            }
            "recovery" => {
                remotes::add(&f);
                let before = f.repo.repository.head().unwrap().target().unwrap();
                remotes::select(&f);
                f.repo
                    .repository
                    .reference(
                        "refs/heads/main",
                        before,
                        true,
                        "fixture rewinds committed request",
                    )
                    .unwrap();
            }
            "conflicted" => {
                let mut index = f.repo.repository.index().unwrap();
                index.read(true).unwrap();
                let mut entry = index
                    .get_path(Path::new(".manyhands/config.toml"), 0)
                    .unwrap();
                entry.flags = (entry.flags & !0x3000) | 0x1000;
                index.add(&entry).unwrap();
                index.write().unwrap();
                assert!(index.has_conflicts());
            }
            "partial_mismatch" => super::requests::accepted(&f),
            "internal" => {
                let path = f.env.home.path().join("file-data");
                fs::write(&path, b"file").unwrap();
                alternate_env = Some(CliEnv::new(path));
            }
            _ => {}
        }
        let _holder = (row.name == "busy").then(|| {
            support::hold_lease_in_child_for_test(
                &root,
                &f.env.data,
                LeaseKind::Repository,
                "index_lease_child",
            )
        });
        let mut args: Vec<_> = row
            .args
            .iter()
            .map(|arg| match *arg {
                "{root}" => root.to_str().unwrap(),
                "{confirmation}" => confirmation.as_str(),
                "{key}" => key.as_str(),
                // The stopped request belongs to REQUEST, not NEXT.
                NEXT if row.name == "partial_mismatch" => REQUEST,
                arg => arg,
            })
            .collect();
        // Selection's fixture used NEXT; a no-op must be a new intent.
        if row.name == "selected_noop" {
            *args.last_mut().unwrap() = remotes::LAST;
        }
        args.push("--json");
        let start = Instant::now();
        let result = checked(
            &args,
            alternate_env.as_ref().unwrap_or(&f.env),
            row.input,
            row.exit,
            row.code,
            None,
        );
        if row.name == "partial_mismatch" {
            assert_eq!(result["outcome"], "partial");
        }
        #[cfg(unix)]
        if row.name == "unsafe_key_store_lock" {
            assert_eq!(result["outcome"], "blocked");
            assert_eq!(result["request_id"], NEXT);
            assert!(result["operation_id"].is_null());
            for (field, effect) in result["effects"].as_object().unwrap() {
                if field == "commit_oid" {
                    assert!(effect.is_null());
                } else {
                    assert_eq!(effect, "not_requested");
                }
            }
            checked(
                &["request", "show", "--request-id", NEXT, "--json"],
                &f.env,
                &[],
                2,
                "request_not_found",
                None,
            );
            let registered = super::keys::call(&f.env, "show", &["--key-id", &key], &[], 0, "ok");
            assert_eq!(Some(&registered), key_registration.as_ref());
            assert_eq!(registered["data"]["id"], key);
            assert_eq!(registered["data"]["selected"], false);
            assert_eq!(
                fs::read_link(f.env.home.path().join(".ssh/manyhands/.lock")).unwrap(),
                key_files[2]
            );
        }
        if row.name == "busy" {
            assert!(
                start.elapsed() >= Duration::from_millis(700),
                "bounded retry delays must run"
            );
            assert!(
                start.elapsed() < Duration::from_secs(10),
                "lease holder deadline"
            );
        }
        drop(other_repo);
        assert_eq!(
            crate::support::credentials::key_file_states(&key_files),
            key_before
        );
    }
}

fn set<'a>(values: impl IntoIterator<Item = &'a str>) -> BTreeSet<&'a str> {
    values.into_iter().collect()
}

#[test]
fn the_table_and_reasons_account_for_every_code_of_the_audit() {
    let f2 = set(AUDIT_F2.split_whitespace());
    assert_eq!(f2.len(), 49);
    let expected = set(f2.iter().copied().chain(CLI_ONLY.iter().copied()));
    assert_eq!(expected.len(), 52);
    // Pin the literal to Task 0's actual published source, not just its count.
    let audit = include_str!("../../docs/research/wave-03-api-audit.md");
    let section = audit.split("#### C2 exit-coverage union").nth(1).unwrap();
    let published = section
        .split("```text\n")
        .nth(1)
        .unwrap()
        .split("```")
        .next()
        .unwrap();
    assert_eq!(f2, set(published.split_whitespace()));
    let covered = set(ROWS.iter().map(|r| r.code));
    let reasons = set(REASONS.iter().map(|(code, reason)| {
        assert!(!reason.is_empty());
        *code
    }));
    let staged = set(STAGED.iter().map(|(code, task, reason)| {
        assert!((8..=10).contains(task));
        assert!(!reason.is_empty());
        *code
    }));
    assert_eq!(reasons.len(), REASONS.len());
    assert_eq!(staged.len(), STAGED.len());
    assert!(
        covered.is_disjoint(&reasons)
            && covered.is_disjoint(&staged)
            && reasons.is_disjoint(&staged)
    );
    assert_eq!(
        set(covered.into_iter().chain(reasons).chain(staged)),
        expected
    );
    assert_eq!(set(ROWS.iter().map(|r| r.name)).len(), ROWS.len());
}

#[test]
fn every_cli_remote_reference_names_an_existing_case() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/cli_remote");
    for row in ROWS {
        if let How::Remote(case) = row.how {
            let function = format!("fn {case}(");
            assert!(
                fs::read_dir(&directory).unwrap().any(|entry| {
                    let path = entry.unwrap().path();
                    path.extension().is_some_and(|ext| ext == "rs")
                        && fs::read_to_string(path).unwrap().contains(&function)
                }),
                "missing cli_remote::{case}"
            );
        }
    }
}

#[test]
fn every_exit_class_is_covered() {
    let mut exits = set(ROWS
        .iter()
        .filter(|row| matches!(row.how, How::Here | How::Remote(_)))
        .map(|row| match row.exit {
            0 => "success",
            1 => "internal",
            2 => "input",
            3 => "blocked",
            4 => "incomplete",
            5 => "transient",
            _ => panic!("unexpected exit"),
        }));
    for row in ROWS {
        if let How::Prompt(case) = row.how {
            assert_eq!(row.exit, 130);
            assert!(include_str!("prompts.rs").contains(&format!("fn {case}(")));
            // Windows prompt execution is native/manual evidence, not a local pass.
            if cfg!(unix) {
                exits.insert("cancelled");
            }
        }
    }
    let mut expected = set([
        "success",
        "internal",
        "input",
        "blocked",
        "incomplete",
        "transient",
    ]);
    if cfg!(unix) {
        expected.insert("cancelled");
    }
    assert_eq!(exits, expected);
}
