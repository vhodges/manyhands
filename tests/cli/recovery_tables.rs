//! The CLI contract RFC's recovery tables (decision 4; R-10; the C5 design's
//! Recovery Tables). This parses the tables of its "Recovery tables" section,
//! one per family, and holds each row to its golden: the golden exists, is an
//! answer of the row's verb, and holds the row's code, its next action and its
//! exit.
//!
//! For C5's verbs every recoverable code of the design's outcome tables has a
//! row, or is under "Codes with no golden" with its reason. An earlier verb's
//! row holds a code of its Cycle's audit union, and every recoverable code of
//! that union has a row for one of the Cycle's verbs or a reason.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use manyhands::results::{FailureClass, RecoveryActionKind, ResultCode};
use serde_json::Value;

const RFC: &str = include_str!("../../docs/RFC/cli-contract.md");
const AUDIT: &str = include_str!("../../docs/research/wave-03-api-audit.md");

const ROW_HEADER: &str = "| Verb | Code | Next | Exit | Golden |";
const REASON_HEADER: &str = "| Scope | Code | Reason |";
const NO_GOLDEN: &str = "Codes with no golden";

const C2: &[&str] = &[
    "repo create",
    "repo enable",
    "repo remove",
    "repo identity-set",
    "remote add",
    "remote remove",
    "remote select",
    "key generate",
    "key import",
    "key select",
    "key clear",
    "key remove",
    "key delete",
    "host approve",
    "host replace",
    "host observe",
    "request show",
    "request cancel",
];
const C3: &[&str] = &[
    "folder create",
    "document create",
    "document save",
    "document move",
    "document repair",
    "ticket create",
    "ticket save",
    "ticket repair",
    "ticket slug-assign",
    "index refresh",
    "index rebuild",
    "path observe",
    "ticket check",
];
const C4: &[&str] = &[
    "comment add",
    "item sync",
    "repo sync",
    "item republish",
    "conflict show",
    "conflict resolve",
    "operation resume",
    "operation abandon",
];
const LIFECYCLE: &[&str] = &["document promote", "ticket close"];
const LOCAL_SYNC: &[&str] = &[
    "context observe-local",
    "context show-local",
    "context sync-local",
    "context resolve-local",
];
const POLLING: &[&str] = &["poll once", "poll configure", "poll pause", "poll resume"];

/// The six families, in the RFC's order, and their verbs.
const FAMILIES: &[(&str, &[&str])] = &[
    ("Authoring", C3),
    ("Repository and credentials", C2),
    ("Collaboration", C4),
    ("Lifecycle", LIFECYCLE),
    ("Local sync", LOCAL_SYNC),
    ("Polling", POLLING),
];

/// The recoverable codes of the C5 design's outcome tables, by verb: the
/// lifecycle's Prepare, Answers, Identity, Cache loss and Host and unlock; the
/// `poll once` table and the policy verbs'; and the local-sync required
/// outcomes.
const C5_REQUIRED: &[(&[&str], &[&str])] = &[
    (
        LIFECYCLE,
        &[
            "recovery_required",
            "identity_required",
            "push_rejected",
            "discovery_pending",
            "cancelled",
            "external_change",
            "busy",
            "poll_yielding",
            "confirmation_not_found",
            "confirmation_expired",
            "remote_branch_deleted",
            "host_approval_required",
            "host_replacement_required",
            "unlock_required",
            "transport_unavailable",
        ],
    ),
    (&["ticket close"], &["ticket_closed"]),
    (
        &["poll once"],
        &[
            "publication_remote_required",
            "cancelled",
            "recovery_required",
            "no_selected_key",
            "key_rejected",
            "unlock_required",
            "host_approval_required",
            "transport_unavailable",
            "remote_unavailable",
            "repository_inaccessible",
            "busy",
            "poll_yielding",
            "discovery_pending",
        ],
    ),
    (
        &["poll configure", "poll pause", "poll resume"],
        &["repository_not_registered", "index_unavailable"],
    ),
    (
        &["context sync-local"],
        &[
            "discovery_pending",
            "external_change",
            "worktree_not_clean",
            "identity_required",
            "recovery_required",
            "busy",
            "cancelled",
        ],
    ),
    (
        &["context resolve-local"],
        &["recovery_required", "external_change", "cancelled"],
    ),
];

#[derive(Debug)]
struct Row {
    family: String,
    verbs: Vec<String>,
    code: String,
    next: String,
    exit: i32,
    golden: String,
}

#[derive(Debug)]
struct Reason {
    /// `C2`, `C3` or `C4`, or the verbs of a C5 row.
    scope: Vec<String>,
    code: String,
    reason: String,
}

fn backticked(cell: &str) -> Vec<String> {
    cell.split('`')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}

fn one(cell: &str) -> String {
    let values = backticked(cell);
    assert_eq!(values.len(), 1, "{cell:?}");
    values.into_iter().next().unwrap()
}

/// The RFC's section and its subsections, by heading.
fn subsections() -> Vec<(String, String)> {
    let section = RFC
        .split("\n## Recovery tables\n")
        .nth(1)
        .expect("the RFC has a Recovery tables section")
        .split("\n## ")
        .next()
        .unwrap();
    section
        .split("\n### ")
        .skip(1)
        .map(|part| {
            let (heading, body) = part.split_once('\n').unwrap();
            (heading.trim().to_owned(), body.to_owned())
        })
        .collect()
}

fn table_rows<'a>(body: &'a str, header: &str) -> Vec<Vec<&'a str>> {
    let table = body
        .split(header)
        .nth(1)
        .unwrap_or_else(|| panic!("no table {header:?}"))
        .split("\n\n")
        .next()
        .unwrap();
    let rows: Vec<Vec<&str>> = table
        .lines()
        .filter(|line| line.starts_with("| ") && !line.starts_with("| ---"))
        .map(|line| {
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            cells[1..cells.len() - 1].to_vec()
        })
        .collect();
    assert!(!rows.is_empty(), "an empty table {header:?}");
    rows
}

fn parsed() -> (Vec<Row>, Vec<Reason>) {
    let sections = subsections();
    let headings: Vec<&str> = sections
        .iter()
        .map(|(heading, _)| heading.as_str())
        .collect();
    let mut expected: Vec<&str> = FAMILIES.iter().map(|(family, _)| *family).collect();
    expected.push(NO_GOLDEN);
    assert_eq!(headings, expected);
    let mut rows = Vec::new();
    let mut reasons = Vec::new();
    for (heading, body) in &sections {
        if heading == NO_GOLDEN {
            for cells in table_rows(body, REASON_HEADER) {
                assert_eq!(cells.len(), 3, "{cells:?}");
                let scope = if cells[0].starts_with('`') {
                    backticked(cells[0])
                } else {
                    vec![cells[0].to_owned()]
                };
                reasons.push(Reason {
                    scope,
                    code: one(cells[1]),
                    reason: cells[2].to_owned(),
                });
            }
            continue;
        }
        for cells in table_rows(body, ROW_HEADER) {
            assert_eq!(cells.len(), 5, "{cells:?}");
            let verbs = backticked(cells[0]);
            assert!(!verbs.is_empty(), "{cells:?}");
            rows.push(Row {
                family: heading.clone(),
                verbs,
                code: one(cells[1]),
                next: one(cells[2]),
                exit: cells[3].parse().unwrap_or_else(|_| panic!("{cells:?}")),
                golden: one(cells[4]),
            });
        }
    }
    (rows, reasons)
}

fn result_code(code: &str) -> ResultCode {
    ResultCode::ALL
        .into_iter()
        .find(|known| known.as_str() == code)
        .unwrap_or_else(|| panic!("no result code {code}"))
}

/// The exit of an answer whose code is `code`, by the code's own class.
fn class_exit(code: &str) -> i32 {
    match result_code(code).failure_class() {
        None => 0,
        Some(FailureClass::Input) => 2,
        Some(FailureClass::Blocked) => 3,
        Some(FailureClass::Incomplete) => 4,
        Some(FailureClass::Transient) => 5,
        Some(FailureClass::Internal) => 1,
        Some(FailureClass::Cancelled) => 130,
    }
}

/// A code with a way on: blocked, transient, incomplete or cancelled.
fn recoverable(code: &str) -> bool {
    matches!(
        result_code(code).failure_class(),
        Some(
            FailureClass::Blocked
                | FailureClass::Transient
                | FailureClass::Incomplete
                | FailureClass::Cancelled
        )
    )
}

fn snake(verb: &str) -> String {
    verb.replace([' ', '-'], "_")
}

fn is_action(next: &str) -> bool {
    RecoveryActionKind::ALL
        .into_iter()
        .any(|action| action.as_str() == next)
}

/// Checks the row against its golden, and gives the golden's exit.
fn check_golden(row: &Row) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(&row.golden);
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{row:?}: no golden {}: {error}", path.display()));
    let command_line = row.next.strip_prefix("manyhands-cli ");
    assert!(
        row.next == "none" || is_action(&row.next) || command_line.is_some(),
        "{row:?}: next is an action, a manyhands-cli line or none"
    );
    if row.golden.ends_with(".json") {
        assert!(
            row.golden.starts_with("cli_v1/") || row.golden.starts_with("mutation_v1/"),
            "{row:?}"
        );
        let value: Value = serde_json::from_str(&text).unwrap();
        let command = value["command"].as_str().unwrap();
        assert!(
            row.verbs.iter().any(|verb| verb == command),
            "{row:?}: the golden is {command}'s"
        );
        assert_eq!(value["code"], row.code.as_str(), "{row:?}");
        let actions: Vec<&str> = value["recovery"]
            .as_array()
            .unwrap()
            .iter()
            .map(|action| action["action"].as_str().unwrap())
            .collect();
        match row.next.as_str() {
            "none" => assert!(actions.is_empty(), "{row:?}: offers {actions:?}"),
            next if is_action(next) => {
                assert!(actions.contains(&next), "{row:?}: offers {actions:?}")
            }
            _ => panic!("{row:?}: a manyhands-cli line is a text golden's"),
        }
        let exit = match value["outcome"].as_str().unwrap() {
            "cancelled" => 130,
            "partial" => 4,
            _ => class_exit(&row.code),
        };
        assert_eq!(row.exit, exit, "{row:?}");
    } else {
        assert!(row.golden.ends_with(".txt"), "{row:?}");
        assert!(row.golden.starts_with("cli_v1/"), "{row:?}");
        let name = row.golden.trim_start_matches("cli_v1/");
        assert!(
            row.verbs.iter().any(|verb| name.starts_with(&snake(verb))),
            "{row:?}: the text golden is not one of its verbs'"
        );
        let code = text
            .lines()
            .find_map(|line| line.strip_prefix("code: "))
            .unwrap_or_else(|| panic!("{row:?}: no code line"));
        assert_eq!(code, row.code, "{row:?}");
        let nexts: Vec<&str> = text
            .lines()
            .filter_map(|line| line.strip_prefix("next: "))
            .collect();
        match row.next.as_str() {
            "none" => assert!(nexts.is_empty(), "{row:?}: prints {nexts:?}"),
            next => assert!(
                nexts
                    .iter()
                    .any(|line| *line == next || line.starts_with(&format!("{next} "))),
                "{row:?}: prints {nexts:?}"
            ),
        }
        let exit = text
            .lines()
            .find_map(|line| line.strip_prefix("--- exit "))
            .map(|exit| exit.parse::<i32>().unwrap());
        match exit {
            Some(exit) => assert_eq!(row.exit, exit, "{row:?}"),
            None => assert!(
                row.exit == class_exit(&row.code) || row.exit == 4,
                "{row:?}"
            ),
        }
    }
    assert!(
        recoverable(&row.code) || row.exit == 4,
        "{row:?}: not a recoverable code"
    );
}

#[test]
fn every_row_names_a_golden_that_holds_its_code_next_action_and_exit() {
    let (rows, _) = parsed();
    let families: BTreeMap<&str, &[&str]> = FAMILIES.iter().copied().collect();
    for row in &rows {
        for verb in &row.verbs {
            assert!(
                families[row.family.as_str()].contains(&verb.as_str()),
                "{row:?}: {verb} is not of the family"
            );
        }
        check_golden(row);
    }
    for (family, _) in FAMILIES {
        assert!(
            rows.iter().any(|row| row.family == *family),
            "{family} has no row"
        );
    }
}

/// C5 follow-up: the ordinary item is the sole continuation owner. The broad
/// governing tables are amended in Task 7; pin these delivered binary fixtures
/// here without inventing a parent resume/abandon or an aggregate local journal.
#[test]
fn retirement_recovery_is_item_owned_and_stopped_pass_is_only_a_summary() {
    let row = Row {
        family: "Retirement".into(),
        verbs: vec!["context retire".into()],
        code: "discovery_pending".into(),
        next: "operation.resume".into(),
        exit: 4,
        golden: "cli_v1/context_retire_index_pending.json".into(),
    };
    check_golden(&row);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/cli_v1");
    let stopped = fs::read_to_string(root.join("poll_once_retirement_stopped_text.txt")).unwrap();
    assert!(stopped.contains("as-of-pass"));
    assert!(stopped.contains("never resumes the loop"));
    assert!(!stopped.contains("operation.resume"));
    assert!(!stopped.contains("operation.abandon"));
    assert!(stopped.contains("--- exit 3"));
}

/// The codes the audit's `#### <cycle> exit-coverage union` block holds.
fn union(cycle: &str) -> BTreeSet<&'static str> {
    AUDIT
        .split(&format!("#### {cycle} exit-coverage union"))
        .nth(1)
        .unwrap()
        .split("```text\n")
        .nth(1)
        .unwrap()
        .split("```")
        .next()
        .unwrap()
        .split_whitespace()
        .collect()
}

/// Each C5 verb's codes, expanded from the audit's C5 result-set table with the
/// C4 refresh's groups.
fn c5_codes() -> BTreeMap<&'static str, BTreeSet<&'static str>> {
    let c4 = AUDIT
        .split("### Wave 03 CLI Cycle C4 refresh")
        .nth(1)
        .unwrap();
    let definitions = c4
        .split("The groups are:")
        .nth(1)
        .unwrap()
        .split("#### ")
        .next()
        .unwrap();
    let mut groups: BTreeMap<char, BTreeSet<&'static str>> = BTreeMap::new();
    for part in definitions
        .split("**")
        .skip(1)
        .step_by(2)
        .filter(|part| part.len() == 1 && part.chars().all(|c| c.is_ascii_uppercase()))
    {
        let name = part.chars().next().unwrap();
        let rest = definitions.split(&format!("**{part}** = ")).nth(1).unwrap();
        let mut codes = BTreeSet::new();
        let first = rest.chars().next().unwrap();
        if first.is_ascii_uppercase() {
            codes.extend(groups[&first].iter().copied());
        }
        codes.extend(rest.split('`').nth(1).unwrap().split_whitespace());
        groups.insert(name, codes);
    }
    let table = AUDIT
        .split("#### C5 bindings and result-set key")
        .nth(1)
        .unwrap()
        .split("| Verb | Codes | Count | Design anchor |")
        .nth(1)
        .unwrap()
        .split("\n\n")
        .next()
        .unwrap();
    let mut verbs = BTreeMap::new();
    for row in table.lines().filter(|line| line.starts_with("| `")) {
        let cells: Vec<&str> = row.split('|').map(str::trim).collect();
        let mut codes = BTreeSet::new();
        for part in cells[2].split(" + ") {
            match part.trim_matches('`') {
                group if group.len() == 1 => {
                    codes.extend(groups[&group.chars().next().unwrap()].iter().copied())
                }
                explicit => codes.extend(explicit.split_whitespace()),
            }
        }
        verbs.insert(cells[1].trim_matches('`'), codes);
    }
    assert_eq!(verbs.len(), 10, "{verbs:?}");
    verbs
}

/// The codes a verb's Cycle can answer with: its audit union or row.
fn codes_of(verb: &str) -> BTreeSet<&'static str> {
    for (cycle, verbs) in [("C2", C2), ("C3", C3), ("C4", C4)] {
        if verbs.contains(&verb) {
            return union(cycle);
        }
    }
    c5_codes()
        .remove(verb)
        .unwrap_or_else(|| panic!("{verb}: no audit row"))
}

#[test]
fn every_row_s_code_is_one_its_verbs_answer_with_by_the_audit() {
    let (rows, reasons) = parsed();
    for row in &rows {
        for verb in &row.verbs {
            assert!(
                codes_of(verb).contains(row.code.as_str()),
                "{row:?}: the audit gives {verb} no {}",
                row.code
            );
        }
    }
    for reason in &reasons {
        assert!(recoverable(&reason.code), "{reason:?}");
        assert!(reason.reason.len() > 20, "{reason:?}");
        for scope in &reason.scope {
            let codes = match scope.as_str() {
                "C2" | "C3" | "C4" => union(scope),
                verb => codes_of(verb),
            };
            assert!(codes.contains(reason.code.as_str()), "{reason:?}");
        }
    }
}

/// A code has a row for a verb when a row names the verb, or one of the verbs
/// that share its binding.
fn covered(rows: &[Row], verb: &str, code: &str) -> bool {
    rows.iter()
        .any(|row| row.code == code && row.verbs.iter().any(|known| known == verb))
}

fn reasoned(reasons: &[Reason], scope: &str, code: &str) -> bool {
    reasons
        .iter()
        .any(|reason| reason.code == code && reason.scope.iter().any(|known| known == scope))
}

#[test]
fn every_recoverable_code_of_c5_s_outcome_tables_has_a_row() {
    let (rows, reasons) = parsed();
    let mut missing = Vec::new();
    for (verbs, codes) in C5_REQUIRED {
        for verb in *verbs {
            for code in *codes {
                assert!(recoverable(code), "{code}");
                let row = covered(&rows, verb, code);
                let reason = reasoned(&reasons, verb, code);
                assert!(!(row && reason), "{verb} {code}: a row and a reason");
                if !row && !reason {
                    missing.push(format!("{verb} {code}"));
                }
            }
        }
    }
    assert!(missing.is_empty(), "no row: {missing:?}");
    // A C5 reason is for a code no golden can hold.
    let c5_reasons: Vec<_> = reasons
        .iter()
        .filter(|reason| !["C2", "C3", "C4"].contains(&reason.scope[0].as_str()))
        .collect();
    assert!(c5_reasons.len() <= 2, "{c5_reasons:?}");
}

#[test]
fn every_recoverable_code_of_an_earlier_cycle_s_union_has_a_row_or_a_reason() {
    let (rows, reasons) = parsed();
    for (cycle, verbs) in [("C2", C2), ("C3", C3), ("C4", C4)] {
        let mut missing = Vec::new();
        for code in union(cycle).into_iter().filter(|code| recoverable(code)) {
            let row = verbs.iter().any(|verb| covered(&rows, verb, code));
            let reason = reasoned(&reasons, cycle, code);
            assert!(!(row && reason), "{cycle} {code}: a row and a reason");
            if !row && !reason {
                missing.push(code);
            }
        }
        assert!(
            missing.is_empty(),
            "{cycle}: no row or reason for {missing:?}"
        );
    }
}
