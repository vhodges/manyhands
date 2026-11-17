//! The remote history repair, through the binary and one SSH remote.
//!
//! This is the feature's single end-to-end SSH case (AGENTS.md, "SSH-harness
//! case, by exception"). It needs SSH because the transport is SSH-only (a
//! `file://` remote is refused), and `remote history` and `item sync` cannot
//! run without one. What it pins is that the pieces connect: a key selected
//! after the remote was set no longer costs the history of a new ticket, and
//! a repository that did lose its history is listed, refused truthfully,
//! repaired by the owner's confirmation and then publishes. Every edge case
//! of the decisions behind it (the relation table, stale confirmations, the
//! flag) is a library unit test.
//!
//! The loss itself is made in the registry, between two runs of the binary,
//! as `remote::resolve_tests::Fixture::lose_history` makes it: a change of
//! the remote's identity is a unit-tested fence of the library, and needs a
//! second reachable endpoint that this fixture does not have.

use super::*;
use cli_pair::{CliClone, CliPair};
use collaboration::{human, item_sync, next_lines, text};
use ssh_remote::{FixtureError, fixed};

pub const CASES: &[ssh_harness::Case] = &[(
    "remote_history_repair_journey",
    remote_history_repair_journey,
)];

const LOCAL: &str = "01ARZ3NDEKTSV4RRFFQ69G5FB1";
const LOST: &str = "01ARZ3NDEKTSV4RRFFQ69G5FB2";
const AFTER: &str = "01ARZ3NDEKTSV4RRFFQ69G5FB3";

fn branch(item: &str) -> String {
    format!("refs/heads/manyhands/ticket/{item}")
}

fn request() -> String {
    repository::RequestId::new().to_string()
}

/// A ticket made in the clone: its first write makes the editing context.
fn create(clone: &CliClone, item: &str) {
    let input = serde_json::json!({
        "item_id": item,
        "title": format!("Ticket {item}"),
        "type": "task",
        "status": "open",
        "body": "Made locally.\n",
    });
    clone.json(
        &[
            "ticket",
            "create",
            "--repo",
            clone.root(),
            "--request-id",
            &request(),
            "--input",
            "-",
        ],
        &fixed(serde_json::to_vec(&input)).unwrap(),
        "ok",
        0,
    );
}

/// The registry as a repository that lost its cache holds it: no context
/// rows, no fetch, and the flag set.
fn lose_history(clone: &CliClone) {
    let db = rusqlite::Connection::open(clone.env.data.join(repository::REGISTRY_FILE)).unwrap();
    db.execute_batch(
        "DELETE FROM remote_context_states; DELETE FROM remote_ref_observations; \
         DELETE FROM remote_observation_batches; UPDATE remote_polling_state SET history_unknown=1;",
    )
    .unwrap();
}

/// The owner's choice for the lost ticket, with the flags of the step.
fn resolve_args<'a>(clone: &'a CliClone, extra: &[&'a str]) -> Vec<&'a str> {
    let mut args = vec![
        "item",
        "resolve-history",
        "--repo",
        clone.root(),
        "--id",
        LOST,
        "--as",
        "publish",
    ];
    args.extend_from_slice(extra);
    args
}

fn remote_history_repair_journey() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let a = &p.a;

    // The original bug: a key selected again after the remote was set.
    let keys = a.json(&["key", "list"], &[], "ok", 0);
    let key = keys["data"]["items"][0]["id"].as_str().unwrap().to_owned();
    a.json(&["key", "clear", "--request-id", &request()], &[], "ok", 0);
    a.json(
        &[
            "key",
            "select",
            "--key-id",
            &key,
            "--request-id",
            &request(),
        ],
        &[],
        "ok",
        0,
    );
    create(a, LOCAL);
    item_sync(a, LOCAL, &request(), "ok", 0);
    assert_eq!(p.served(&branch(LOCAL))?, a.tip(&branch(LOCAL))?);
    assert!(p.served(&branch(LOCAL))?.is_some());

    // A repository that lost its history: an unpublished ticket of it.
    create(a, LOST);
    lose_history(a);
    let listed = human(a, &["remote", "history", "--repo", a.root()], &[]);
    assert_eq!(listed.status, 0, "{}", text(&listed.stderr));
    let root = collaboration::quoted(a);
    assert!(
        text(&listed.stdout).contains(&format!(
            "{LOST}  ticket  manyhands/ticket/{LOST}  absent  local-only  "
        )),
        "{}",
        text(&listed.stdout)
    );
    assert!(text(&listed.stdout).contains("unresolved  choices=publish (release: desktop only)\n"));
    assert!(text(&listed.stdout).contains("Cache loss: yes\n"));
    assert_eq!(
        next_lines(text(&listed.stdout)),
        [format!(
            "next: manyhands-cli item resolve-history --repo {root} --id {LOST} --as publish --prepare"
        )]
    );

    // The refusal is final: it names the repair and holds no reservation.
    let refused = item_sync(a, LOST, &request(), "recovery_required", 3);
    assert!(
        refused["recovery"]
            .as_array()
            .unwrap()
            .iter()
            .any(|action| action["action"] == "remote.history")
    );
    assert_eq!(p.served(&branch(LOST))?, None);
    let human_refusal = human(
        a,
        &[
            "item",
            "sync",
            "--repo",
            a.root(),
            "--id",
            LOST,
            "--request-id",
            &request(),
        ],
        &[],
    );
    assert_eq!(human_refusal.status, 3);
    assert!(
        next_lines(text(&human_refusal.stderr))
            .contains(&format!("next: manyhands-cli remote history --repo {root}").as_str()),
        "{}",
        text(&human_refusal.stderr)
    );
    // Another context is not held up by it: the refusal held no reservation.
    item_sync(a, LOCAL, &request(), "already_applied", 0);

    // The owner's choice: prepare, then confirm.
    let preview = a.json(&resolve_args(a, &["--prepare"]), &[], "ok", 0);
    assert_eq!(preview["data"]["details"]["item_id"], LOST);
    let request_id = request();
    let confirmation = preview["data"]["confirmation_id"].as_str().unwrap();
    let resolved = a.json(
        &resolve_args(a, &["--request-id", &request_id, "--confirm", confirmation]),
        &[],
        "ok",
        0,
    );
    assert_eq!(resolved["data"]["cache_loss_cleared"], true);
    assert_eq!(p.served(&branch(LOST))?, None);

    // Then it publishes as any context does.
    item_sync(a, LOST, &request(), "ok", 0);
    assert!(p.served(&branch(LOST))?.is_some());
    assert_eq!(p.served(&branch(LOST))?, a.tip(&branch(LOST))?);

    // A context made after the loss syncs with no repair.
    create(a, AFTER);
    item_sync(a, AFTER, &request(), "ok", 0);
    assert!(p.served(&branch(AFTER))?.is_some());
    let settled = human(a, &["remote", "history", "--repo", a.root()], &[]);
    assert_eq!(settled.status, 0, "{}", text(&settled.stderr));
    assert_eq!(text(&settled.stdout), "Contexts: none\nCache loss: no\n");
    Ok(())
}
