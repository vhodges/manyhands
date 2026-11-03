//! C4's exit evidence (R-27). Every code of Task 0's audit union, with the
//! CLI's own, maps to a test in which a C4 verb reaches it through the
//! binary, or to the reason no fixture reaches it; the rows and the reasons
//! equal the union, by set equality. A row names a `#[test]` of this target
//! or a `c4_` SSH case of `cli_remote` (`tests/cli_remote/`), which runs
//! there with `-- c4_`.
//!
//! The tests below reach, without a server, the codes no other C4 test
//! reaches: root and authoring refusals, the repository's own states, the
//! refusals of a remote or key the transport cannot use before it connects,
//! a corrupt registry and an unopenable data directory.

use std::{collections::BTreeSet, fs, path::Path};

use manyhands::results::{FailureClass, ResultCode};
use serde_json::{Value, json};

use super::{c4_support::environment, mutation::checked};
use crate::support::{self, cli::CliEnv, items};

const AUDIT: &str = include_str!("../../docs/research/wave-03-api-audit.md");
/// The CLI's own codes, as C3 has them. `usage_error` and `invalid_id` are
/// the two the library's union does not hold.
const CLI_ONLY: &[&str] = &[
    "usage_error",
    "invalid_input",
    "invalid_id",
    "invalid_path",
    "cancelled",
    "internal_error",
];
const REQUEST: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR1";
const KEY_REQUEST: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR7";
const SELECT_REQUEST: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR8";
const COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FM0";

enum How {
    /// A `#[test]` of this target.
    Binary(&'static str),
    /// A `c4_` SSH case of `cli_remote`, registered under this name.
    Remote(&'static str),
    /// Why no fixture reaches the code through the binary, and what pins it.
    Reason(&'static str),
}

struct Row {
    code: &'static str,
    exit: i32,
    /// Whether the named test's result is partial: a durable effect came
    /// first, so it exits 4 whatever its code's own class.
    partial: bool,
    how: How,
}

const fn binary(code: &'static str, exit: i32, case: &'static str) -> Row {
    Row {
        code,
        exit,
        partial: false,
        how: How::Binary(case),
    }
}

const fn remote(code: &'static str, exit: i32, case: &'static str) -> Row {
    Row {
        code,
        exit,
        partial: false,
        how: How::Remote(case),
    }
}

/// A `c4_` SSH case whose result is partial: exit 4.
const fn partial(code: &'static str, case: &'static str) -> Row {
    Row {
        code,
        exit: 4,
        partial: true,
        how: How::Remote(case),
    }
}

/// Each code once, with the exit the named test asserts for it.
const ROWS: &[Row] = &[
    remote("ok", 0, "c4_collaboration_comment_publishes"),
    remote("already_applied", 0, "c4_collaboration_item_sync"),
    binary(
        "usage_error",
        2,
        "the_verbs_that_take_no_input_refuse_input_as_a_usage_error",
    ),
    binary(
        "invalid_id",
        2,
        "a_malformed_operation_id_is_invalid_id_for_both_verbs",
    ),
    binary(
        "invalid_input",
        2,
        "conflict_show_of_a_local_operation_that_is_no_synchronization_is_invalid_input",
    ),
    binary("invalid_path", 2, "c4_verbs_refuse_each_root"),
    binary("bare_repository", 2, "c4_verbs_refuse_each_root"),
    binary("not_repository", 2, "c4_verbs_refuse_each_root"),
    binary("not_repository_root", 2, "c4_verbs_refuse_each_root"),
    binary("repository_not_enabled", 2, "c4_verbs_refuse_each_root"),
    binary("repository_inaccessible", 3, "c4_verbs_refuse_each_root"),
    binary("repository_not_registered", 3, "c4_verbs_refuse_each_root"),
    binary(
        "item_not_found",
        2,
        "item_sync_reads_the_item_s_kind_and_refuses_an_unknown_item",
    ),
    binary(
        "operation_not_found",
        2,
        "a_resume_of_an_operation_no_journal_holds_is_operation_not_found",
    ),
    binary(
        "occupied_path",
        2,
        "a_comment_id_a_file_already_uses_is_occupied_path",
    ),
    binary(
        "not_confirmable",
        2,
        "prepare_of_each_unconfirmed_mutation_is_not_confirmable",
    ),
    remote("request_mismatch", 2, "c4_collaboration_item_sync"),
    remote(
        "confirmation_mismatch",
        2,
        "c4_interruption_request_cancel_of_item_republish",
    ),
    binary(
        "invalid_remote",
        2,
        "repo_sync_refuses_a_remote_or_key_it_cannot_use_before_connecting",
    ),
    binary(
        "confirmation_required",
        3,
        "an_abandon_of_a_remote_operation_is_refused_and_changes_nothing",
    ),
    binary(
        "confirmation_not_found",
        3,
        "an_unknown_confirmation_is_confirmation_not_found",
    ),
    remote(
        "confirmation_used",
        3,
        "c4_interruption_request_cancel_of_item_republish",
    ),
    remote("external_change", 3, "c4_conflicts_stale_resolution"),
    remote(
        "recovery_required",
        3,
        "c4_conflicts_canonical_shown_and_resolved",
    ),
    binary(
        "identity_required",
        3,
        "comment_add_without_an_identity_is_identity_required",
    ),
    binary(
        "invalid_configuration",
        3,
        "comment_add_refuses_what_authoring_refuses",
    ),
    binary(
        "worktree_not_clean",
        3,
        "comment_add_refuses_what_authoring_refuses",
    ),
    binary(
        "wrong_branch",
        3,
        "comment_add_refuses_what_authoring_refuses",
    ),
    binary(
        "worktree_conflicted",
        3,
        "repo_sync_of_a_conflicted_worktree_is_worktree_conflicted",
    ),
    binary(
        "ticket_closed",
        3,
        "a_comment_on_a_closed_ticket_is_ticket_closed_and_changes_nothing",
    ),
    binary(
        "publication_remote_required",
        3,
        "repo_sync_without_a_publication_remote_is_refused_with_nothing_done",
    ),
    binary(
        "no_selected_key",
        3,
        "repo_sync_refuses_a_remote_or_key_it_cannot_use_before_connecting",
    ),
    binary(
        "key_unavailable",
        3,
        "repo_sync_refuses_a_remote_or_key_it_cannot_use_before_connecting",
    ),
    remote(
        "host_approval_required",
        3,
        "c4_interruption_transport_refusals",
    ),
    remote(
        "host_replacement_required",
        3,
        "c4_interruption_transport_refusals",
    ),
    remote("unlock_required", 3, "c4_interruption_transport_refusals"),
    remote("key_rejected", 3, "c4_interruption_transport_refusals"),
    binary(
        "index_unavailable",
        3,
        "repo_sync_on_a_corrupt_registry_is_index_unavailable",
    ),
    // A comment checkpointed before its publication is refused, or meets a
    // deleted branch, or before its index hand-off fails: partial.
    partial(
        "push_rejected",
        "c4_collaboration_comment_refused_then_saved_once",
    ),
    partial(
        "remote_branch_deleted",
        "c4_collaboration_comment_meets_a_deleted_branch",
    ),
    partial(
        "discovery_pending",
        "c4_interruption_index_hand_off_refused",
    ),
    // Task 12b: a `busy` republish carries the `operation_id` of its last
    // attempt in its envelope, yet keeps no request record.
    remote("busy", 5, "c4_conflicts_republish_busy_then_confirmed"),
    remote(
        "poll_yielding",
        5,
        "c4_interruption_poll_holds_the_reservation",
    ),
    remote(
        "transport_unavailable",
        5,
        "c4_conflicts_republish_transport_stop",
    ),
    binary(
        "internal_error",
        1,
        "repo_sync_with_an_unopenable_data_directory_is_internal_error",
    ),
    remote(
        "cancelled",
        130,
        "c4_interruption_request_cancel_of_item_sync",
    ),
    #[cfg(unix)]
    remote("unlock_failed", 3, "c4_interruption_unlock_prompt"),
    #[cfg(unix)]
    remote(
        "original_request_required",
        3,
        "c4_conflicts_hand_edit_after_written_paths",
    ),
];

const fn reason(code: &'static str, exit: i32, why: &'static str) -> Row {
    Row {
        code,
        exit,
        partial: false,
        how: How::Reason(why),
    }
}

/// The codes no fixture reaches through the binary with a C4 verb.
const REASONS: &[Row] = &[
    reason(
        "remote_unavailable",
        5,
        "libgit2's NotFound is its only source (src/repository/transport/remote.rs), and no deterministic fixture produces it: the transport tests accept it or transport_unavailable for one fixture, as C2's table records.",
    ),
    reason(
        "confirmation_expired",
        3,
        "A republish or abandon confirmation expires after ten minutes (src/repository/mutation/confirm.rs); the clock is the library's, which an isolated CLI child cannot advance, as C2 and C3 record.",
    ),
    #[cfg(not(unix))]
    reason(
        "unlock_failed",
        3,
        "A passphrase is typed at a terminal: the Unix PTY case c4_interruption_unlock_prompt reaches it. Windows prompting is the owed native manual check.",
    ),
    #[cfg(not(unix))]
    reason(
        "original_request_required",
        3,
        "The binary stages a resolution stopped after its written paths with a read-only directory, in the Unix case c4_conflicts_hand_edit_after_written_paths; the library's interrupted-resolution table pins it on every platform (tests/mutation_remote/resolution.rs).",
    ),
];

fn set<'a>(values: impl IntoIterator<Item = &'a str>) -> BTreeSet<&'a str> {
    values.into_iter().collect()
}

/// The audit's published union, read from its fenced block.
fn union() -> BTreeSet<&'static str> {
    let section = AUDIT.split("#### C4 exit-coverage union").nth(1).unwrap();
    let block = section
        .split("```text\n")
        .nth(1)
        .unwrap()
        .split("```")
        .next()
        .unwrap();
    set(block.split_whitespace())
}

#[test]
fn the_c4_table_and_reasons_account_for_every_code_of_the_audit() {
    let library = union();
    assert_eq!(library.len(), 48);
    let expected = set(library.iter().copied().chain(CLI_ONLY.iter().copied()));
    assert_eq!(expected.len(), 50);
    let covered = set(ROWS.iter().map(|row| row.code));
    let reasons = set(REASONS.iter().map(|row| row.code));
    assert_eq!(covered.len(), ROWS.len(), "a code appears twice");
    assert_eq!(reasons.len(), REASONS.len(), "a reason appears twice");
    assert!(covered.is_disjoint(&reasons));
    assert_eq!(set(covered.into_iter().chain(reasons)), expected);
}

fn sources(directory: &str) -> Vec<String> {
    fs::read_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join(directory))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
        .map(|path| fs::read_to_string(path).unwrap())
        .collect()
}

#[test]
fn every_c4_reference_names_an_existing_case() {
    let local = sources("tests/cli");
    let ssh = sources("tests/cli_remote");
    for row in ROWS {
        match row.how {
            How::Binary(case) => {
                let test = format!("#[test]\nfn {case}(");
                assert!(
                    local.iter().any(|source| source.contains(&test)),
                    "missing CLI test {case}"
                );
            }
            How::Remote(case) => {
                assert!(case.starts_with("c4_"), "{case}");
                let registered = format!("\"{case}\"");
                let function = format!("fn {case}(");
                assert!(
                    ssh.iter().any(|source| source.contains(&registered))
                        && ssh.iter().any(|source| source.contains(&function)),
                    "missing cli_remote case {case}"
                );
            }
            How::Reason(_) => panic!("{}: a reason belongs in REASONS", row.code),
        }
    }
    for row in REASONS {
        let How::Reason(why) = row.how else {
            panic!("{}: a test belongs in ROWS", row.code)
        };
        assert!(!why.is_empty(), "{}", row.code);
    }
}

/// The exit of a result whose code is `code`, by the code's own class.
fn class_exit(code: &str) -> i32 {
    let code = ResultCode::ALL
        .into_iter()
        .find(|known| known.as_str() == code)
        .unwrap_or_else(|| panic!("no result code {code}"));
    match code.failure_class() {
        None => 0,
        Some(FailureClass::Input) => 2,
        Some(FailureClass::Blocked) => 3,
        Some(FailureClass::Incomplete) => 4,
        Some(FailureClass::Transient) => 5,
        Some(FailureClass::Internal) => 1,
        Some(FailureClass::Cancelled) => 130,
    }
}

#[test]
fn every_c4_row_exits_by_its_code_s_class_unless_partial() {
    // A partial result exits 4 whatever its code (`Envelope::failure_class`);
    // any other exits by its code's own class.
    for row in ROWS.iter().chain(REASONS) {
        let expected = if row.partial { 4 } else { class_exit(row.code) };
        assert_eq!(row.exit, expected, "{}", row.code);
    }
    assert!(REASONS.iter().all(|row| !row.partial));
}

#[test]
fn every_c4_exit_class_is_reached_by_a_c4_verb_through_the_binary() {
    let reached = ROWS.iter().map(|row| row.exit).collect::<BTreeSet<_>>();
    assert_eq!(reached, [0, 1, 2, 3, 4, 5, 130].into_iter().collect());
}

/// A contract repository, enabled, and a CLI environment over its data.
struct Fixture {
    repo: support::TestRepository,
    env: CliEnv,
    _data: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let (repo, enabled) = items::contract_repository();
        let env = environment(enabled.data_directory.path());
        // No service stays open while a child runs or a file is replaced.
        drop(enabled.service);
        Self {
            repo,
            env,
            _data: enabled.data_directory,
        }
    }

    /// The configuration names `origin`, at `url`, as its publication
    /// remote: a repository configured with Git, as a person would.
    fn with_remote(self, url: &str) -> Self {
        self.repo.repository.remote("origin", url).unwrap();
        let config = self.repo.root.join(".manyhands/config.toml");
        let mut source = fs::read_to_string(&config).unwrap();
        source.push_str("publication_remote = \"origin\"\n");
        fs::write(&config, source).unwrap();
        items::commit(
            &self.repo,
            &[".manyhands/config.toml"],
            items::COMMITTED_AT + 600,
        );
        self
    }

    fn snapshot(&self) -> impl PartialEq + std::fmt::Debug {
        (
            support::repository_and_worktree_snapshot(&self.repo),
            self.refs(),
        )
    }

    fn refs(&self) -> Vec<(String, Option<git2::Oid>)> {
        let mut refs: Vec<_> = self
            .repo
            .repository
            .references()
            .unwrap()
            .map(|reference| {
                let reference = reference.unwrap();
                (reference.name().unwrap().to_owned(), reference.target())
            })
            .collect();
        refs.sort();
        refs
    }
}

fn repo_sync(root: &Path) -> Vec<&str> {
    vec![
        "repo",
        "sync",
        "--repo",
        root.to_str().unwrap(),
        "--request-id",
        REQUEST,
        "--json",
    ]
}

fn comment_add(root: &Path) -> Vec<&str> {
    vec![
        "comment",
        "add",
        "--repo",
        root.to_str().unwrap(),
        "--id",
        items::TICKET_A,
        "--request-id",
        REQUEST,
        "--input",
        "-",
        "--json",
    ]
}

fn comment() -> Vec<u8> {
    serde_json::to_vec(&json!({"comment_id": COMMENT, "body": "A comment.\n"})).unwrap()
}

/// The actions a result offers, in order.
fn actions(value: &Value) -> Vec<&str> {
    value["recovery"]
        .as_array()
        .unwrap()
        .iter()
        .map(|action| action["action"].as_str().unwrap())
        .collect()
}

/// Each root a C4 verb is refused before it reads the repository:
/// `repo sync`, and `comment add` where its item read answers first. Nothing
/// changes.
#[test]
fn c4_verbs_refuse_each_root() {
    for (case, code, exit) in [
        ("bare", "bare_repository", 2),
        ("home", "not_repository", 2),
        ("nested", "not_repository_root", 2),
        ("unreadable", "repository_inaccessible", 3),
        ("plain", "repository_not_enabled", 2),
        ("absent", "invalid_path", 2),
        ("other data", "repository_not_registered", 3),
    ] {
        let f = Fixture::new();
        let mut env = None;
        let root = match case {
            "bare" => {
                let root = f.env.home.path().join("bare");
                git2::Repository::init_bare(&root).unwrap();
                root
            }
            "home" => f.env.home.path().to_owned(),
            "nested" => f.repo.root.join("docs"),
            "unreadable" => {
                fs::write(f.repo.repository.path().join("config"), "[invalid\n").unwrap();
                f.repo.root.clone()
            }
            "plain" => {
                let root = f.env.home.path().join("plain");
                git2::Repository::init(&root).unwrap();
                root
            }
            "absent" => f.env.home.path().join("absent-parent/target"),
            "other data" => {
                env = Some(CliEnv::new(f.env.home.path().join("other-data")));
                f.repo.root.clone()
            }
            _ => unreachable!(),
        };
        let env = env.as_ref().unwrap_or(&f.env);
        // Read through the handle opened before the root was broken: an
        // unreadable configuration cannot be opened again.
        let before = support::repository_git_file_bytes(&f.repo);
        let value = checked(&repo_sync(&root), env, &[], exit, code, None);
        assert_eq!(value["command"], "repo sync", "{case}");
        assert_eq!(value["request_id"], REQUEST, "{case}");
        // `comment add` reads its item first: an unenabled root is one the
        // registry does not hold.
        if case != "plain" {
            let value = checked(&comment_add(&root), env, &comment(), exit, code, None);
            assert_eq!(value["command"], "comment add", "{case}");
        }
        assert_eq!(
            support::repository_git_file_bytes(&f.repo),
            before,
            "{case}"
        );
    }
}

/// `comment add` is refused where authoring is: an unreadable
/// configuration, a worktree with an outside edit, a checkout of another
/// branch. Each is blocked, offers the same request again, and writes no
/// comment and no context.
#[test]
fn comment_add_refuses_what_authoring_refuses() {
    for (case, code) in [
        ("configuration", "invalid_configuration"),
        ("edited", "worktree_not_clean"),
        ("branch", "wrong_branch"),
    ] {
        let f = Fixture::new();
        let config = f.repo.root.join(".manyhands/config.toml");
        match case {
            "configuration" => fs::write(&config, "invalid = [").unwrap(),
            "edited" => {
                let mut bytes = fs::read(&config).unwrap();
                bytes.extend_from_slice(b"\n# external edit\n");
                fs::write(&config, bytes).unwrap();
            }
            "branch" => {
                let commit = f.repo.repository.head().unwrap().peel_to_commit().unwrap();
                f.repo.repository.branch("other", &commit, false).unwrap();
                f.repo.repository.set_head("refs/heads/other").unwrap();
            }
            _ => unreachable!(),
        }
        let before = f.snapshot();
        let value = checked(
            &comment_add(&f.repo.root),
            &f.env,
            &comment(),
            3,
            code,
            None,
        );
        assert_eq!(value["outcome"], "blocked", "{case}");
        assert_eq!(actions(&value)[0], "request.retry", "{case}");
        assert_eq!(f.snapshot(), before, "{case}");
    }
}

/// Comment table, "No identity and no `created_by`": blocked, with the
/// same request and `repo identity-set` offered, and nothing written: no
/// comment file and no commit. The item's editing context may be made, at
/// primary's tip, as C3's authoring makes it before the identity is needed.
#[test]
fn comment_add_without_an_identity_is_identity_required() {
    super::repository::require_no_system_identity();
    let f = Fixture::new();
    let mut config = git2::Config::open(&f.repo.repository.path().join("config")).unwrap();
    config.remove("user.name").unwrap();
    config.remove("user.email").unwrap();
    drop(config);
    let primary = f.repo.repository.head().unwrap().target().unwrap();
    let before = f.refs();
    let value = checked(
        &comment_add(&f.repo.root),
        &f.env,
        &comment(),
        3,
        "identity_required",
        None,
    );
    assert_eq!(value["outcome"], "blocked");
    assert_eq!(actions(&value), ["request.retry", "repo.identity_set"]);
    assert_eq!(value["effects"]["write"], "not_requested");
    assert!(value["effects"]["commit_oid"].is_null());
    for (name, target) in f.refs() {
        if !before.contains(&(name.clone(), target)) {
            assert_eq!(target, Some(primary), "{name}");
        }
    }
    let comment = format!(".manyhands/comments/{}/{COMMENT}.md", items::TICKET_A);
    assert!(!f.repo.root.join(&comment).exists());
    assert!(
        !items::context_worktree(&f.repo.root, items::TICKET_A)
            .join(&comment)
            .exists()
    );
}

/// Divergent-synchronization table, "Target worktree in an unfinished
/// outside merge": blocked, nothing done, before any connection.
#[test]
fn repo_sync_of_a_conflicted_worktree_is_worktree_conflicted() {
    let f = Fixture::new().with_remote("ssh://git@127.0.0.1:9/repository.git");
    let mut index = f.repo.repository.index().unwrap();
    index.read(true).unwrap();
    let mut entry = index
        .get_path(Path::new(".manyhands/config.toml"), 0)
        .unwrap();
    entry.flags = (entry.flags & !0x3000) | 0x1000;
    index.add(&entry).unwrap();
    index.write().unwrap();
    let before = f.snapshot();
    let value = checked(
        &repo_sync(&f.repo.root),
        &f.env,
        &[],
        3,
        "worktree_conflicted",
        None,
    );
    assert_eq!(value["outcome"], "blocked");
    assert_eq!(f.snapshot(), before);
}

/// A port nothing listens on, so a connection would be refused.
fn closed_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// Imports the key at `path`, relative to the CLI's home, and selects it.
fn select_a_key(env: &CliEnv, path: &str) {
    let input =
        serde_json::to_vec(&json!({"label": "Selected", "private_key_path": path})).unwrap();
    let imported = checked(
        &[
            "key",
            "import",
            "--input",
            "-",
            "--request-id",
            KEY_REQUEST,
            "--json",
        ],
        env,
        &input,
        0,
        "ok",
        None,
    );
    checked(
        &[
            "key",
            "select",
            "--key-id",
            imported["data"]["id"].as_str().unwrap(),
            "--request-id",
            SELECT_REQUEST,
            "--json",
        ],
        env,
        &[],
        0,
        "ok",
        None,
    );
}

/// `repo sync` refuses, before it connects, what the transport cannot use:
/// no selected key (blocked, 3); a selected key whose private source is
/// missing (blocked, 3); and, with a usable key, a remote whose address
/// names no user (`invalid_remote`, input, 2). Each offers the same request
/// again, and changes nothing. The remote's port has nothing listening.
#[test]
fn repo_sync_refuses_a_remote_or_key_it_cannot_use_before_connecting() {
    for (case, code, exit, outcome) in [
        ("no key", "no_selected_key", 3, "blocked"),
        ("missing source", "key_unavailable", 3, "blocked"),
        ("no user", "invalid_remote", 2, "error"),
    ] {
        let port = closed_port();
        let url = if case == "no user" {
            format!("ssh://127.0.0.1:{port}/repository.git")
        } else {
            format!("ssh://git@127.0.0.1:{port}/repository.git")
        };
        let f = Fixture::new().with_remote(&url);
        match case {
            "no key" => {}
            "missing source" => select_a_key(&f.env, "absent-private-source"),
            "no user" => {
                let home = f.env.home.path();
                fs::write(
                    home.join("imported"),
                    support::credentials::private_fixture_bytes(),
                )
                .unwrap();
                select_a_key(&f.env, "imported");
            }
            _ => unreachable!(),
        }
        let before = f.snapshot();
        let value = checked(&repo_sync(&f.repo.root), &f.env, &[], exit, code, None);
        assert_eq!(value["outcome"], outcome, "{case}");
        assert_eq!(actions(&value), ["request.retry"], "{case}");
        assert_eq!(f.snapshot(), before, "{case}");
    }
}

/// A registry that is no database: `repo sync` is blocked, offering an
/// index rebuild, and changes nothing.
#[test]
fn repo_sync_on_a_corrupt_registry_is_index_unavailable() {
    let f = Fixture::new();
    for name in ["manyhands.sqlite3-wal", "manyhands.sqlite3-shm"] {
        match fs::remove_file(f.env.data.join(name)) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("remove SQLite sidecar: {error}"),
        }
    }
    fs::write(f.env.data.join("manyhands.sqlite3"), [0xA5; 4096]).unwrap();
    let before = f.snapshot();
    let value = checked(
        &repo_sync(&f.repo.root),
        &f.env,
        &[],
        3,
        "index_unavailable",
        None,
    );
    assert_eq!(actions(&value), ["index.rebuild"]);
    assert_eq!(f.snapshot(), before);
}

/// Exit 1: a data directory that is a file. The envelope still goes to
/// stdout. The service is not opened, so this is an error before dispatch,
/// with a null request ID (Global Constraints, Runtime).
#[test]
fn repo_sync_with_an_unopenable_data_directory_is_internal_error() {
    let f = Fixture::new();
    let path = f.env.home.path().join("file-data");
    fs::write(&path, b"not a directory").unwrap();
    let before = f.snapshot();
    let value = checked(
        &repo_sync(&f.repo.root),
        &CliEnv::new(path),
        &[],
        1,
        "internal_error",
        None,
    );
    assert!(value["request_id"].is_null(), "{value}");
    assert_eq!(f.snapshot(), before);
}
