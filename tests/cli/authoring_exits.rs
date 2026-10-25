//! C3 Part A evidence. Part B obligations remain explicit, not passing cases.
use std::{
    collections::BTreeSet,
    fs,
    path::Path,
    time::{Duration, Instant},
};

use manyhands::repository::LeaseKind;
use serde_json::{Value, json};

use super::{
    index::Fixture,
    mutation::{REQUEST, checked, run},
};
use crate::support::{self, cli, items};

const AUDIT: &str = include_str!("../../docs/research/wave-03-api-audit.md");
const CLI_ONLY: &[&str] = &[
    "usage_error",
    "invalid_input",
    "invalid_id",
    "invalid_path",
    "cancelled",
    "internal_error",
];
const PART_A: &[&str] = &[
    "index refresh",
    "index rebuild",
    "folder create",
    "document create",
    "document save",
    "document move",
];

enum How {
    Binary(&'static str),
    ActorThenBinary {
        case: &'static str,
        reason: &'static str,
    },
    Pending(&'static str),
    Source(&'static str),
}

struct Row {
    code: &'static str,
    exit: i32,
    how: How,
}

macro_rules! row {
    ($code:literal, $exit:literal, $case:literal) => {
        Row {
            code: $code,
            exit: $exit,
            how: How::Binary($case),
        }
    };
}

// These are references to real-binary assertions, not synthetic envelopes.
// Shared fixture tests may provide more than one code; each code appears once.
const ROWS: &[Row] = &[
    row!("ok", 0, "document_create_commits_one_checkpoint_offline"),
    row!(
        "already_applied",
        0,
        "a_save_that_omits_every_field_is_a_noop_for_a_library_written_document"
    ),
    row!(
        "usage_error",
        2,
        "index_verbs_need_a_request_id_without_a_terminal"
    ),
    row!(
        "invalid_input",
        2,
        "folder_create_refuses_bad_input_without_echoing_values"
    ),
    row!(
        "invalid_id",
        2,
        "document_inputs_are_closed_typed_and_private_and_bad_ids_keep_request_id"
    ),
    row!(
        "invalid_path",
        2,
        "a_move_to_an_escaping_destination_is_refused"
    ),
    row!("occupied_path", 2, "a_move_onto_a_file_is_occupied_path"),
    row!(
        "item_not_found",
        2,
        "a_document_save_of_a_ticket_id_is_item_not_found_with_its_scope"
    ),
    row!("not_confirmable", 2, "c3_confirmed_flags_match_prepare"),
    row!(
        "request_mismatch",
        2,
        "a_retry_after_another_actor_changed_an_omitted_field_is_request_mismatch"
    ),
    row!(
        "repository_not_enabled",
        2,
        "a_never_enabled_root_answers_by_whether_a_field_is_omitted"
    ),
    row!(
        "repository_not_registered",
        3,
        "a_lost_cache_answers_by_whether_a_field_is_omitted"
    ),
    row!(
        "index_unavailable",
        3,
        "index_refresh_on_a_corrupt_database_is_index_unavailable"
    ),
    row!(
        "external_change",
        3,
        "a_title_change_by_another_actor_makes_a_body_only_save_external_change"
    ),
    row!(
        "recovery_required",
        3,
        "interrupted_saves_follow_the_required_outcomes"
    ),
    row!("busy", 5, "busy_is_reached_with_the_lease_held"),
    row!(
        "internal_error",
        1,
        "an_unopenable_data_directory_is_internal_error"
    ),
    row!(
        "bare_repository",
        2,
        "document_create_root_refusals_reach_the_binary"
    ),
    row!(
        "not_repository",
        2,
        "document_create_root_refusals_reach_the_binary"
    ),
    row!(
        "not_repository_root",
        2,
        "document_create_root_refusals_reach_the_binary"
    ),
    row!(
        "repository_inaccessible",
        3,
        "document_create_root_refusals_reach_the_binary"
    ),
    row!(
        "invalid_configuration",
        3,
        "document_create_authoring_refusals_reach_the_binary"
    ),
    row!(
        "worktree_not_clean",
        3,
        "document_create_authoring_refusals_reach_the_binary"
    ),
    row!(
        "wrong_branch",
        3,
        "document_create_authoring_refusals_reach_the_binary"
    ),
    row!(
        "identity_required",
        3,
        "document_create_without_identity_reaches_the_binary"
    ),
];

const REASONS: &[Row] = &[
    Row {
        code: "discovery_pending",
        exit: 4,
        how: How::ActorThenBinary {
            case: "commit_success_refresh_failure_is_completed_by_the_same_request",
            reason: "Owner decision 6: BeforeIndexTransactionCommit is a library-only failure seam. The actor proves partial/Incomplete with a committed checkpoint; the CLI completes the same request. This is not a binary exit-4 claim (src/results.rs:273–288; tests/cli/documents.rs).",
        },
    },
    Row {
        code: "registration_pending",
        exit: 4,
        how: How::Source(
            "Folder creation's pre-directory stale mark needs an injected registration failure, unavailable in the CLI child: src/repository/bridges/folder.rs:504–520 and src/repository.rs:10780–10818. Ordinary saves instead report discovery_pending.",
        ),
    },
    Row {
        code: "cancelled",
        exit: 130,
        how: How::Pending(
            "Part B, Task 6: decline repair consent through the PTY; C3 local operations have no post-acceptance safe point (Cycle cancellation evidence; src/bin/manyhands-cli/mutate.rs).",
        ),
    },
    Row {
        code: "confirmation_required",
        exit: 3,
        how: How::Pending(
            "Part B, Task 6: repair execution without consent; src/repository/mutation/execute.rs:277–280.",
        ),
    },
    Row {
        code: "confirmation_not_found",
        exit: 3,
        how: How::Pending(
            "Part B, Task 6: repair consent missing or lost with the cache; src/repository/mutation/confirm.rs:399–410.",
        ),
    },
    Row {
        code: "confirmation_mismatch",
        exit: 2,
        how: How::Pending(
            "Part B, Task 6: repair confirmation for different input; src/repository/mutation/confirm.rs:393–428.",
        ),
    },
    Row {
        code: "confirmation_used",
        exit: 3,
        how: How::Pending(
            "Part B, Task 6: consumed repair confirmation under another request; src/repository/mutation/confirm.rs:393–428.",
        ),
    },
    Row {
        code: "confirmation_expired",
        exit: 3,
        how: How::Pending(
            "Part B, Task 6: repair consent expiry; src/repository/mutation/confirm.rs:393–428. The ten-minute clock seam is library-only, so final coverage needs a reason rather than a fabricated binary case.",
        ),
    },
    Row {
        code: "identity_ambiguous",
        exit: 3,
        how: How::Pending(
            "Part B, Task 6: repair adopts an ID already held by another item; src/repository/mutation/bind/repair.rs:467–548.",
        ),
    },
    Row {
        code: "not_repairable",
        exit: 2,
        how: How::Pending(
            "Part B, Task 6: repair of conforming/non-place source; src/repository/mutation/bind/repair.rs:193–198,446–464.",
        ),
    },
    Row {
        code: "path_not_found",
        exit: 2,
        how: How::Pending(
            "Part B, Task 6: missing repair source path; src/repository/mutation/bind/repair.rs:350–371. path observe of an absent path is successful observation, not this code.",
        ),
    },
    Row {
        code: "initials_required",
        exit: 3,
        how: How::Pending(
            "Part B, Task 5: ticket create cannot compose initials; src/repository/mutation/bind/ticket.rs:491–524,582–598.",
        ),
    },
    Row {
        code: "invalid_slug_configuration",
        exit: 3,
        how: How::Pending(
            "Part B, Task 5: ticket create/slug assignment configuration; src/repository/mutation/bind/ticket.rs:350–633,1240–1290.",
        ),
    },
    Row {
        code: "invalid_relationship",
        exit: 2,
        how: How::Pending(
            "Part B, Task 5: malformed ticket relationship input; src/repository/mutation/bind/ticket.rs:350–633,751–773.",
        ),
    },
    Row {
        code: "relationship_cycle",
        exit: 2,
        how: How::Pending(
            "Part B, Task 5: ticket create/save relationship rejection; src/repository/mutation/bind/ticket.rs:350–633,751–773. Part A ticket check returns ok with rejection data, not this top-level code.",
        ),
    },
    Row {
        code: "slug_already_assigned",
        exit: 2,
        how: How::Pending(
            "Part B, Task 5: different held ticket slug; src/repository/mutation/bind/ticket.rs:1240–1290.",
        ),
    },
    Row {
        code: "ticket_closed",
        exit: 3,
        how: How::Pending(
            "Part B, Task 5: save/slug assignment of a lifecycle-closed ticket; src/repository/mutation/bind/ticket.rs:601–633,1240–1290.",
        ),
    },
];

fn set<'a>(values: impl IntoIterator<Item = &'a str>) -> BTreeSet<&'a str> {
    values.into_iter().collect()
}

fn assert_case_exists(case: &str) {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/cli");
    let function = format!("#[test]\nfn {case}(");
    assert!(
        fs::read_dir(directory).unwrap().any(|entry| {
            let path = entry.unwrap().path();
            path.extension().is_some_and(|ext| ext == "rs")
                && fs::read_to_string(path).unwrap().contains(&function)
        }),
        "missing CLI test {case}"
    );
}

#[test]
fn the_c3_table_and_reasons_account_for_every_code_of_the_audit() {
    let section = AUDIT.split("#### C3 exit-coverage union").nth(1).unwrap();
    let published = section
        .split("```text\n")
        .nth(1)
        .unwrap()
        .split("```")
        .next()
        .unwrap();
    let library = set(published.split_whitespace());
    assert_eq!(library.len(), 40);
    let expected = set(library.into_iter().chain(CLI_ONLY.iter().copied()));
    assert_eq!(expected.len(), 42);
    let covered = set(ROWS.iter().map(|r| r.code));
    let reasons = set(REASONS.iter().map(|r| r.code));
    assert_eq!(covered.len(), ROWS.len());
    assert_eq!(reasons.len(), REASONS.len());
    assert!(covered.is_disjoint(&reasons));
    assert_eq!(set(covered.into_iter().chain(reasons)), expected);
}

#[test]
fn every_c3_reference_names_an_existing_case() {
    for row in ROWS {
        let How::Binary(case) = row.how else {
            panic!("a row must be binary evidence")
        };
        assert_case_exists(case);
    }
    for row in REASONS {
        match row.how {
            How::ActorThenBinary { case, reason } => {
                assert_case_exists(case);
                assert!(!reason.is_empty());
            }
            How::Pending(reason) => assert!(reason.starts_with("Part B, Task ")),
            How::Source(reason) => assert!(!reason.is_empty()),
            How::Binary(_) => panic!("binary evidence belongs in ROWS"),
        }
    }
}

#[test]
fn every_c3_exit_class_is_covered() {
    let binary: BTreeSet<_> = ROWS
        .iter()
        .filter(|row| matches!(row.how, How::Binary(_)))
        .map(|r| r.exit)
        .collect();
    for exit in [0, 2, 3, 5] {
        assert!(binary.contains(&exit), "missing binary exit {exit}");
    }
    assert!(ROWS.iter().chain(REASONS).any(|r| r.exit == 1));
    assert!(REASONS.iter().any(|r| r.exit == 4
        && matches!(
            r.how,
            How::ActorThenBinary {
                case: "commit_success_refresh_failure_is_completed_by_the_same_request",
                ..
            }
        )));
    assert!(REASONS.iter().any(|r| r.code == "cancelled"
        && r.exit == 130
        && matches!(r.how, How::Pending(reason) if reason.starts_with("Part B, Task 6"))));
    let all: BTreeSet<_> = ROWS.iter().chain(REASONS).map(|r| r.exit).collect();
    assert_eq!(all, [0, 1, 2, 3, 4, 5, 130].into_iter().collect());
}

fn show(f: &Fixture) -> Value {
    f.checked(
        "document",
        "show",
        &["--id", items::DOCUMENT_A],
        &[],
        0,
        "ok",
    )["data"]
        .clone()
}

#[test]
fn index_refresh_on_a_corrupt_database_is_index_unavailable() {
    let f = Fixture::new();
    let before = support::repository_and_worktree_snapshot(&f.repo);
    let refs = f.refs("refs/");
    // Fixture::new drops its service. Remove WAL/SHM before replacing the whole
    // database, preserving the approved corruption boundary (not partial damage).
    for name in ["manyhands.sqlite3-wal", "manyhands.sqlite3-shm"] {
        match fs::remove_file(f.data.path().join(name)) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("remove SQLite sidecar: {error}"),
        }
    }
    fs::write(f.data.path().join("manyhands.sqlite3"), [0xA5; 4096]).unwrap();
    let value = f.checked(
        "index",
        "refresh",
        &["--request-id", REQUEST],
        &[],
        3,
        "index_unavailable",
    );
    assert_eq!(value["command"], "index refresh");
    assert_eq!(value["request_id"], REQUEST);
    assert!(
        value["recovery"]
            .as_array()
            .unwrap()
            .iter()
            .any(|action| { action["action"] == "index.rebuild" })
    );
    assert_eq!(support::repository_and_worktree_snapshot(&f.repo), before);
    assert_eq!(f.refs("refs/"), refs);
}

#[test]
fn busy_is_reached_with_the_lease_held() {
    let f = Fixture::new();
    let item = show(&f);
    // Full draft avoids a fill-read refusal before mutate::run's retry loop.
    let input = serde_json::to_vec(&json!({
        "observation": item["observation"], "title": item["title"], "body": "new body\n",
    }))
    .unwrap();
    let before = support::repository_and_worktree_snapshot(&f.repo);
    let _holder = support::hold_lease_in_child_for_test(
        &f.repo.root,
        &f.env.data,
        LeaseKind::Repository,
        "index_lease_child",
    );
    let start = Instant::now();
    let value = f.checked(
        "document",
        "save",
        &[
            "--id",
            items::DOCUMENT_A,
            "--request-id",
            REQUEST,
            "--input",
            "-",
        ],
        &input,
        5,
        "busy",
    );
    assert!(
        start.elapsed() >= Duration::from_millis(700),
        "bounded retry delays must run"
    );
    assert!(
        start.elapsed() < Duration::from_secs(10),
        "lease holder deadline"
    );
    assert_eq!(value["request_id"], REQUEST);
    assert_eq!(support::repository_and_worktree_snapshot(&f.repo), before);
}

fn available_c3_mutations() -> Vec<String> {
    let data = tempfile::tempdir().unwrap();
    let env = cli::CliEnv::new(data.path().join("data"));
    let help = checked(&["--help", "--json"], &env, &[], 0, "ok", None);
    help["data"]["commands"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["available"] == true && c["delivered_by"] == "C3")
        .filter(|c| {
            c["options"]
                .as_array()
                .unwrap()
                .iter()
                .any(|o| o["name"] == "--request-id")
        })
        .map(|c| c["command"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn c3_confirmed_flags_match_prepare() {
    let commands = available_c3_mutations();
    assert_eq!(
        set(commands.iter().map(String::as_str)),
        set(PART_A.iter().copied())
    );
    for command in commands {
        let f = Fixture::new();
        let (resource, verb) = command.split_once(' ').unwrap();
        let mut extra = vec!["--prepare"];
        let input = match command.as_str() {
            "index refresh" | "index rebuild" => None,
            "folder create" => Some(json!({"path":"docs/new-folder"})),
            "document create" => Some(
                serde_json::from_slice::<Value>(include_bytes!(
                    "../fixtures/cli_input_v1/document_create.json"
                ))
                .unwrap(),
            ),
            "document save" => {
                extra.extend(["--id", items::DOCUMENT_A]);
                Some(json!({"observation":show(&f)["observation"],"body":"prepared body\n"}))
            }
            "document move" => {
                extra.extend(["--id", items::DOCUMENT_A]);
                let destination = f.checked(
                    "path",
                    "observe",
                    &["--path", "docs/moved.md"],
                    &[],
                    0,
                    "ok",
                );
                Some(
                    json!({"observation":show(&f)["observation"],"destination_path":"docs/moved.md","destination_observation":destination["data"]["observation"]}),
                )
            }
            _ => panic!("unaccounted mutation {command}"),
        };
        let bytes = input
            .map(|input| {
                extra.extend(["--input", "-"]);
                serde_json::to_vec(&input).unwrap()
            })
            .unwrap_or_default();
        let before = support::repository_and_worktree_snapshot(&f.repo);
        let value = f.checked(resource, verb, &extra, &bytes, 2, "not_confirmable");
        assert_eq!(value["command"], command);
        assert_eq!(support::repository_and_worktree_snapshot(&f.repo), before);
    }
}

const NOOPS: &[(&str, How)] = &[
    (
        "folder create",
        How::Binary("folder_create_of_an_existing_folder_is_a_noop"),
    ),
    (
        "document create",
        How::Binary("document_verbs_have_human_success_refusal_and_noop_output"),
    ),
    (
        "document save",
        How::Binary("a_save_that_omits_every_field_is_a_noop_for_a_library_written_document"),
    ),
    (
        "document move",
        How::Binary("a_move_to_its_own_path_is_a_noop"),
    ),
    (
        "index refresh",
        How::Source(
            "src/repository/mutation/bind/index.rs:80–88,133–159: each fresh request explicitly refreshes; successful completion and finished replay are ok, not already_applied/noop.",
        ),
    ),
    (
        "index rebuild",
        How::Source(
            "src/repository/mutation/bind/index.rs:80–88,148–159,263–270: each fresh request explicitly rebuilds; an unchanged canonical tree does not make index work a noop.",
        ),
    ),
];

#[test]
fn c3_noop_inventory_accounts_for_part_a() {
    let commands = available_c3_mutations();
    let available = set(commands.iter().map(String::as_str));
    assert_eq!(available, set(PART_A.iter().copied()));
    assert_eq!(available, set(NOOPS.iter().map(|(verb, _)| *verb)));
    assert_eq!(NOOPS.len(), 6);
    let capability = AUDIT
        .split("#### C3 no-op capability")
        .nth(1)
        .unwrap()
        .split("#### ")
        .next()
        .unwrap();
    let mut capable = 0;
    for (verb, how) in NOOPS {
        let prefix = format!("| `{verb}` | ");
        let row = capability
            .lines()
            .find_map(|line| line.strip_prefix(&prefix))
            .unwrap();
        match how {
            How::Binary(case) => {
                assert!(row.starts_with("Yes:"), "{verb}");
                assert_case_exists(case);
                capable += 1;
            }
            How::Source(reason) => {
                assert!(row.starts_with("No:"), "{verb}");
                assert!(!reason.is_empty());
            }
            _ => panic!("no-op inventory needs a case or incapability reason"),
        }
    }
    assert_eq!(capable, 4);
}

const CREATE: &[u8] = include_bytes!("../fixtures/cli_input_v1/document_create.json");

fn create_at(f: &Fixture, root: &Path, exit: i32, code: &str) {
    checked(
        &[
            "document",
            "create",
            "--repo",
            root.to_str().unwrap(),
            "--request-id",
            REQUEST,
            "--input",
            "-",
            "--json",
        ],
        &f.env,
        CREATE,
        exit,
        code,
        None,
    );
}

#[test]
fn document_create_root_refusals_reach_the_binary() {
    for (code, exit) in [
        ("bare_repository", 2),
        ("not_repository", 2),
        ("not_repository_root", 2),
        ("repository_inaccessible", 3),
    ] {
        let f = Fixture::new();
        let root = match code {
            "bare_repository" => {
                let root = f.env.home.path().join("bare");
                git2::Repository::init_bare(&root).unwrap();
                root
            }
            "not_repository" => f.env.home.path().to_owned(),
            "not_repository_root" => f.repo.root.join("docs"),
            "repository_inaccessible" => {
                fs::write(f.repo.repository.path().join("config"), "[invalid\n").unwrap();
                f.repo.root.clone()
            }
            _ => unreachable!(),
        };
        create_at(&f, &root, exit, code);
        assert!(!root.join("docs/new.md").exists());
    }
}

#[test]
fn document_create_authoring_refusals_reach_the_binary() {
    for code in [
        "invalid_configuration",
        "worktree_not_clean",
        "wrong_branch",
    ] {
        let f = Fixture::new();
        let config = f.repo.root.join(".manyhands/config.toml");
        match code {
            "invalid_configuration" => fs::write(config, "invalid = [").unwrap(),
            "worktree_not_clean" => {
                let mut bytes = fs::read(&config).unwrap();
                bytes.extend_from_slice(b"\n# external edit\n");
                fs::write(config, bytes).unwrap();
            }
            "wrong_branch" => {
                let commit = f.repo.repository.head().unwrap().peel_to_commit().unwrap();
                f.repo.repository.branch("other", &commit, false).unwrap();
                f.repo.repository.set_head("refs/heads/other").unwrap();
            }
            _ => unreachable!(),
        }
        let before = support::repository_and_worktree_snapshot(&f.repo);
        create_at(&f, &f.repo.root, 3, code);
        assert_eq!(support::repository_and_worktree_snapshot(&f.repo), before);
    }
}

#[test]
fn document_create_without_identity_reaches_the_binary() {
    super::repository::require_no_system_identity();
    let f = Fixture::new();
    let mut config = git2::Config::open(&f.repo.repository.path().join("config")).unwrap();
    config.remove("user.name").unwrap();
    config.remove("user.email").unwrap();
    drop(config);
    create_at(&f, &f.repo.root, 3, "identity_required");
    let branch = format!(
        "refs/heads/manyhands/document/{}",
        support::mutation::DOCUMENT_NEW
    );
    assert_eq!(
        f.repo.repository.find_reference(&branch).unwrap().target(),
        f.repo.repository.head().unwrap().target()
    );
}

#[test]
fn an_unopenable_data_directory_is_internal_error() {
    let f = Fixture::new();
    let path = f.env.home.path().join("file-data");
    fs::write(&path, b"not a directory").unwrap();
    let env = cli::CliEnv::new(path);
    let before = support::repository_and_worktree_snapshot(&f.repo);
    let out = run(
        &[
            "index",
            "refresh",
            "--repo",
            f.repo.root.to_str().unwrap(),
            "--request-id",
            REQUEST,
            "--json",
        ],
        &env,
        &[],
    );
    let value = cli::json(&out);
    assert_eq!(out.status, 1);
    assert_eq!(value["code"], "internal_error");
    support::schema::check_published("envelope.schema.json", &value).unwrap();
    assert_eq!(support::repository_and_worktree_snapshot(&f.repo), before);
}
