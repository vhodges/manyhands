//! C4's conflict and republish journeys through the binary, between two
//! clones of one SSH remote (`CliPair`, R-24). Each case is one fresh pair,
//! under the harness's watchdog.
//!
//! The library's tables (`tests/mutation_remote/conflict_read.rs`,
//! `resolution.rs`, `republish.rs`) pin what the library answers; these
//! cases pin the same rows as a person meets them: the exit, the envelope,
//! human mode's `progress:`, `conflict:`, `next:` and `note:` lines, what
//! the server and each clone hold, a replay of a finished request with no
//! second effect, and `request_mismatch` for a request ID sent again with
//! another target.
//!
//! Each clone diverges as a person would: a ticket save through the binary,
//! or a commit of their own on primary made with Git. Clone B, as a peer
//! with Git of its own, deletes the item's branch on the server. An outside
//! repair is the person's own Git work in the worktree.
//!
//! Every run goes through `scan_output`, and a resolution's source holds a
//! line no output may hold: only `conflict show --json` prints content, and
//! only the conflict's sides.

use super::*;
use cli_pair::{CONTEXT, CliClone, CliPair, ITEM, OTHER_ITEM, PRIMARY};
use collaboration::{
    RESERVATION_NOTE, connections, human, item_sync, next_lines, oid, progress, quoted, repo_sync,
    request_show, resume, resume_args, save, text,
};
use ssh_remote::{FixtureBoundary, FixtureError, ReceiveUpdate, fixed};

pub const CASES: &[ssh_harness::Case] = &[
    (
        "c4_conflicts_canonical_shown_and_resolved",
        c4_conflicts_canonical_shown_and_resolved,
    ),
    (
        "c4_conflicts_stale_resolution",
        c4_conflicts_stale_resolution,
    ),
    ("c4_conflicts_outside_repair", c4_conflicts_outside_repair),
    (
        "c4_conflicts_resolve_meets_a_deleted_branch",
        c4_conflicts_resolve_meets_a_deleted_branch,
    ),
    #[cfg(unix)]
    (
        "c4_conflicts_hand_edit_after_written_paths",
        unix::c4_conflicts_hand_edit_after_written_paths,
    ),
    (
        "c4_conflicts_republish_busy_then_confirmed",
        c4_conflicts_republish_busy_then_confirmed,
    ),
    (
        "c4_conflicts_republish_confirmed",
        c4_conflicts_republish_confirmed,
    ),
    (
        "c4_conflicts_republish_local_moved",
        c4_conflicts_republish_local_moved,
    ),
    (
        "c4_conflicts_republish_transport_stop",
        c4_conflicts_republish_transport_stop,
    ),
    #[cfg(unix)]
    (
        "c4_conflicts_republish_ctrl_c_at_consent",
        unix::c4_conflicts_republish_ctrl_c_at_consent,
    ),
];

const R1: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR1";
const R2: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR2";
const R3: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR3";
const R4: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR4";
const R5: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR5";
const R6: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR6";
/// The synchronization that stops on the conflict, or is refused for the
/// deleted branch: the request that holds the reservation.
const SYNC: &str = "01ARZ3NDEKTSV4RRFFQ69G5FY2";
/// A line of every resolution's source, which no output may hold.
const PRIVATE_SOURCE: &str = "cli-pair-private-resolution-source-sentinel";
/// A well-formed token that no conflict issued.
const NO_SUCH_TOKEN: &str =
    "conflict-path-v1:0000000000000000000000000000000000000000000000000000000000000000";
/// A file that is no canonical content.
const CODE: &str = "src/conflict.rs";
const CONFLICT_SCHEMA: &str = "conflict.schema.json";
const RESOLUTION_SCHEMA: &str = "conflict_resolution_mutation.schema.json";
const SYNCHRONIZATION_CONFLICT_SCHEMA: &str = "synchronization_conflict_mutation.schema.json";
const PREVIEW_SCHEMA: &str = "item_republish_preview.schema.json";
const DELETED: &str = "error: The published branch was deleted on the remote.\n\
                       code: remote_branch_deleted\n";
/// Task 10's note on a `busy` republish (`human_collaboration.rs`).
const BUSY_NOTE: &str = "note: if an earlier item sync of this item was refused, its request still holds the reservation: run request cancel with that request's ID, then send that request once more, before retrying this request.\n";

/// A ticket's canonical source, titled `title`, with the body line `body`.
fn source(id: &str, title: &str, body: &str) -> String {
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {id}\ntitle: {title}\n\
         type: task\nstatus: open\n---\n{body}\n"
    )
}

/// The source a ticket of `CliPair`'s seed is retitled to by a person:
/// `title`, with the body that goes with it.
fn retitled(id: &str, title: &str) -> String {
    source(id, title, &format!("Body of {title}."))
}

fn ticket_path(id: &str) -> String {
    format!(".manyhands/tickets/{id}/ticket.md")
}

/// A person's own commit of `files` on the clone's primary, made with Git,
/// which leaves the worktree clean. Returns the commit.
fn commit_on_primary(clone: &CliClone, files: &[(&str, &str)]) -> Result<git2::Oid, FixtureError> {
    let repo = clone.repo()?;
    let parent = fixed(repo.find_commit(fixed(repo.refname_to_id(PRIMARY))?))?;
    let new = commit_object(&repo, &fixed(parent.tree())?, &[parent.id()], files)?;
    fixed(repo.checkout_tree(
        &fixed(repo.find_object(new, None))?,
        Some(git2::build::CheckoutBuilder::new().safe()),
    ))?;
    fixed(repo.reference(PRIMARY, new, true, "a person's own commit"))?;
    Ok(new)
}

/// A commit of `files` over `tree`, whose parents are `parents`. No
/// reference moves.
fn commit_object(
    repo: &git2::Repository,
    tree: &git2::Tree<'_>,
    parents: &[git2::Oid],
    files: &[(&str, &str)],
) -> Result<git2::Oid, FixtureError> {
    let mut index = fixed(git2::Index::new())?;
    fixed(index.read_tree(tree))?;
    for (path, bytes) in files {
        fixed(index.add(&git2::IndexEntry {
            ctime: git2::IndexTime::new(0, 0),
            mtime: git2::IndexTime::new(0, 0),
            dev: 0,
            ino: 0,
            mode: 0o100644,
            uid: 0,
            gid: 0,
            file_size: 0,
            id: fixed(repo.blob(bytes.as_bytes()))?,
            flags: 0,
            flags_extended: 0,
            path: path.as_bytes().to_vec(),
        }))?;
    }
    let tree = fixed(repo.find_tree(fixed(index.write_tree_to(repo))?))?;
    let parents = parents
        .iter()
        .map(|parent| fixed(repo.find_commit(*parent)))
        .collect::<Result<Vec<_>, _>>()?;
    let parents: Vec<&git2::Commit<'_>> = parents.iter().collect();
    let signature = fixed(git2::Signature::now("A person", "person@example.invalid"))?;
    fixed(repo.commit(
        None,
        &signature,
        &signature,
        "a person's own commit",
        &tree,
        &parents,
    ))
}

/// The text `path` holds in the commit `commit` of `repo`.
fn blob_text(
    repo: &git2::Repository,
    commit: git2::Oid,
    path: &str,
) -> Result<String, FixtureError> {
    let tree = fixed(fixed(repo.find_commit(commit))?.tree())?;
    let entry = fixed(tree.get_path(std::path::Path::new(path)))?;
    let blob = fixed(repo.find_blob(entry.id()))?;
    fixed(String::from_utf8(blob.content().to_vec()))
}

/// The bytes of `path` in the clone's worktree.
fn worktree_bytes(clone: &CliClone, path: &str) -> Result<Vec<u8>, FixtureError> {
    fixed(std::fs::read(clone.root.join(path)))
}

/// A conflict on primary in both tickets' canonical sources. A retitles
/// them `Alpha` and publishes with `repo sync`; B retitles them `Beta`, and
/// its commit is left for B's own `repo sync` to meet the conflict. Returns
/// B's local commit and A's published one.
pub(super) fn tickets_diverge(p: &CliPair) -> Result<(git2::Oid, git2::Oid), FixtureError> {
    let incoming = retitle_both(&p.a, "Alpha")?;
    let published = repo_sync(&p.a, R1, "ok", 0);
    assert_eq!(published["effects"]["publication"], "published");
    assert_eq!(p.served(PRIMARY)?, Some(incoming));
    let local = retitle_both(&p.b, "Beta")?;
    Ok((local, incoming))
}

/// A person's own commit on the clone's primary that retitles both
/// tickets `title`.
fn retitle_both(clone: &CliClone, title: &str) -> Result<git2::Oid, FixtureError> {
    let sources = [ITEM, OTHER_ITEM].map(|id| (ticket_path(id), retitled(id, title)));
    commit_on_primary(
        clone,
        &sources
            .iter()
            .map(|(path, text)| (path.as_str(), text.as_str()))
            .collect::<Vec<_>>(),
    )
}

/// B's `repo sync` under `SYNC`, in JSON, which stops on the conflict:
/// `recovery_required` with the next step in its data (Cycle, item 4).
/// Returns the synchronization.
pub(super) fn stopped_on_primary(p: &CliPair, next: &str) -> String {
    let stopped = repo_sync(&p.b, SYNC, "recovery_required", 3);
    stopped_with(&stopped, next)
}

/// The synchronization a stop on a conflict names, and what the stop says.
fn stopped_with(stopped: &Value, next: &str) -> String {
    assert_eq!(stopped["outcome"], "blocked");
    assert_eq!(stopped["effects"]["integration"], "pending");
    assert_eq!(stopped["effects"]["publication"], "pending");
    assert!(stopped["effects"]["commit_oid"].is_null());
    schema::check_published(SYNCHRONIZATION_CONFLICT_SCHEMA, &stopped["data"]).unwrap();
    let operation = stopped["data"]["conflict"]["operation_id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(stopped["operation_id"], operation.as_str());
    assert_eq!(stopped["data"]["conflict"]["next_action"], next);
    assert_eq!(stopped["recovery"][0]["action"], "request.retry");
    assert_eq!(stopped["recovery"][0]["arguments"]["request_id"], SYNC);
    assert_eq!(stopped["recovery"][1]["action"], "repo.inspect");
    operation
}

fn show_args<'a>(clone: &'a CliClone, operation: &'a str) -> Vec<&'a str> {
    vec![
        "conflict",
        "show",
        "--repo",
        clone.root(),
        "--operation-id",
        operation,
    ]
}

/// `conflict show` in JSON, its data checked against the published schema.
pub(super) fn show(clone: &CliClone, operation: &str) -> Value {
    let value = checked(
        &clone.env,
        &show_args(clone, operation),
        &[],
        "ok",
        0,
        Some(CONFLICT_SCHEMA),
    );
    assert_eq!(value["command"], "conflict show");
    // A read has no request.
    assert!(value["request_id"].is_null());
    assert_eq!(value["data"]["operation_id"], operation);
    value
}

/// Asserts the state and next step `conflict show` reads.
fn assert_reads(shown: &Value, state: &str, next: &str) {
    assert_eq!(shown["data"]["state"], state, "{shown}");
    assert_eq!(shown["data"]["next_action"], next, "{shown}");
}

/// Asserts that `conflict show` reads a conflict no live inspection is made
/// of: no paths and no observation.
fn assert_settled(shown: &Value, state: &str, next: &str) {
    assert_reads(shown, state, next);
    assert!(shown["data"]["paths"].as_array().unwrap().is_empty());
    assert!(shown["data"]["observation"].is_null());
    assert!(shown["data"]["eligibility"].is_null());
}

pub(super) fn resolve_args<'a>(
    clone: &'a CliClone,
    operation: &'a str,
    request: &'a str,
) -> Vec<&'a str> {
    vec![
        "conflict",
        "resolve",
        "--repo",
        clone.root(),
        "--operation-id",
        operation,
        "--request-id",
        request,
        "--input",
        "-",
    ]
}

pub(super) fn resolve_input(observation: &str, resolutions: &[(String, String)]) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "observation": observation,
        "resolutions": resolutions
            .iter()
            .map(|(token, source)| serde_json::json!({"token": token, "source": source}))
            .collect::<Vec<_>>(),
    }))
    .unwrap()
}

/// One `conflict resolve` in JSON, its data checked against its schema.
fn resolve(
    clone: &CliClone,
    operation: &str,
    request: &str,
    input: &[u8],
    code: &str,
    status: i32,
) -> Value {
    let value = clone.json(
        &resolve_args(clone, operation, request),
        input,
        code,
        status,
    );
    assert_eq!(value["command"], "conflict resolve");
    assert_eq!(value["request_id"], request);
    // The command holds no operation of its own (R-32, R-35).
    assert!(value["operation_id"].is_null());
    if !value["data"].is_null() {
        schema::check_published(RESOLUTION_SCHEMA, &value["data"]).unwrap();
    }
    value
}

/// The ticket a conflicted path of `conflict show` is the source of.
fn ticket_of(path: &Value) -> &'static str {
    if path["display_path"].as_str().unwrap().contains(OTHER_ITEM) {
        OTHER_ITEM
    } else {
        ITEM
    }
}

/// The resolution of a path of `ticket` titled `title`, with the private
/// line as its body.
fn resolved_source(ticket: &str, title: &str) -> String {
    source(ticket, title, PRIVATE_SOURCE)
}

/// A resolution of every path `shown` gives, with the token it gave: each
/// ticket's source titled `title`.
pub(super) fn resolutions(shown: &Value, title: &str) -> Vec<(String, String)> {
    shown["data"]["paths"]
        .as_array()
        .unwrap()
        .iter()
        .map(|path| {
            (
                path["token"].as_str().unwrap().to_owned(),
                resolved_source(ticket_of(path), title),
            )
        })
        .collect()
}

pub(super) fn observation(shown: &Value) -> &str {
    shown["data"]["observation"].as_str().unwrap()
}

/// Makes `PRIVATE_SOURCE` a probe of every later run's output.
pub(super) fn private_source() {
    OUTPUT_PROBES.with(|probes| probes.borrow_mut().push(PRIVATE_SOURCE.to_owned()));
}

/// A request that was refused before it was accepted keeps no record.
pub(super) fn no_record(clone: &CliClone, request: &str) {
    checked(
        &clone.env,
        &["request", "show", "--request-id", request],
        &[],
        "request_not_found",
        2,
        None,
    );
}

/// Asserts that every effect but those `named` is `not_requested`.
pub(super) fn assert_effects(value: &Value, named: &[(&str, &str)]) {
    for key in [
        "write",
        "checkpoint",
        "discovery",
        "publication",
        "integration",
        "cleanup",
    ] {
        let expected = named
            .iter()
            .find(|(name, _)| *name == key)
            .map_or("not_requested", |(_, effect)| effect);
        assert_eq!(value["effects"][key], expected, "{key}: {value}");
    }
}

/// A CLI golden of one clone's result. The clone's root and the value at
/// each pointer of `varying`, each a value one run makes, are replaced.
fn golden_of(name: &str, value: &Value, clone: &CliClone, varying: &[(&str, &str)]) {
    let mut replacements = vec![(
        clone
            .root
            .canonicalize()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned(),
        "<repository>",
    )];
    for (pointer, label) in varying {
        let found = value.pointer(pointer).and_then(Value::as_str).unwrap();
        if !replacements.iter().any(|(known, _)| known == found) {
            replacements.push((found.to_owned(), label));
        }
    }
    cli::assert_cli_golden(
        name,
        value,
        &replacements
            .iter()
            .map(|(found, label)| (found.as_str(), *label))
            .collect::<Vec<_>>(),
    );
}

/// A clean worktree with no merge in progress.
fn assert_clean(clone: &CliClone) -> Result<(), FixtureError> {
    let repo = clone.repo()?;
    assert_eq!(repo.state(), git2::RepositoryState::Clean);
    let mut options = git2::StatusOptions::new();
    options.include_untracked(true).include_ignored(false);
    assert!(fixed(repo.statuses(Some(&mut options)))?.is_empty());
    Ok(())
}

/// A canonical conflict on primary with two paths, the two clones each
/// retitling both tickets: shown in JSON (golden) and in text (golden,
/// the owner's check of its labels), then resolved with the tokens it
/// showed. One resolution commit, its parents ordered, is published; the
/// `_ok` golden; a replay; `request_mismatch` for another operation under
/// the same request ID; a second, different resolution refused (Cycle,
/// Stale resolutions); and the other clone brought to the resolution.
fn c4_conflicts_canonical_shown_and_resolved() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let (a, b) = (&p.a, &p.b);
    let (local, incoming) = tickets_diverge(&p)?;
    let pushes = remote_world::accepted(&p.server);
    let operation = stopped_on_primary(&p, "resolve");
    assert_eq!(b.tip(PRIMARY)?, Some(local));
    assert_eq!(p.served(PRIMARY)?, Some(incoming));

    // What `conflict show` reads: every path, with its sides.
    let shown = show(b, &operation);
    let data = &shown["data"];
    for (key, expected) in [
        ("target", "primary"),
        ("stage", "primary"),
        ("state", "conflict_pending"),
        ("next_action", "resolve"),
        ("eligibility", "eligible_canonical"),
    ] {
        assert_eq!(data[key], expected, "{key}");
    }
    for key in ["comment_operation_id", "item_id", "request_id"] {
        assert!(data[key].is_null(), "{key}");
    }
    assert_eq!(data["historical"], false);
    assert_eq!(oid(&data["local_parent"]), local);
    assert_eq!(oid(&data["incoming_parent"]), incoming);
    assert!(observation(&shown).starts_with("conflict-v1:"));
    let paths = data["paths"].as_array().unwrap();
    assert_eq!(paths.len(), 2);
    // In raw byte order, each with its kind, structure and three sides.
    for (path, (id, base)) in paths.iter().zip([(ITEM, "Shared"), (OTHER_ITEM, "Other")]) {
        assert_eq!(path["display_path"], ticket_path(id));
        assert_eq!(path["kind"], "ticket");
        assert_eq!(path["structure"], "regular");
        assert_eq!(path["content_available"], true);
        assert!(
            path["token"]
                .as_str()
                .unwrap()
                .starts_with("conflict-path-v1:")
        );
        assert_eq!(path["base"], retitled(id, base));
        assert_eq!(path["local"], retitled(id, "Beta"));
        assert_eq!(path["incoming"], retitled(id, "Alpha"));
    }
    golden_of(
        "conflict_show_ok",
        &shown,
        b,
        &[
            ("/data/operation_id", "<operation-id>"),
            ("/data/observation", "<observation>"),
            ("/data/local_parent", "<local-parent>"),
            ("/data/incoming_parent", "<incoming-parent>"),
            ("/data/paths/0/token", "<token-1>"),
            ("/data/paths/1/token", "<token-2>"),
        ],
    );
    // In text: the state, the next step and each path, and no side.
    let out = human(b, &show_args(b, &operation), &[]);
    assert_eq!(out.status, 0);
    assert!(out.stderr.is_empty());
    let stdout = text(&out.stdout);
    assert!(!stdout.contains("Body of"), "{stdout}");
    assert!(!stdout.contains("title:"), "{stdout}");
    let placeholders = [
        (operation.as_str(), "<operation-id>"),
        (observation(&shown), "<observation>"),
        (paths[0]["token"].as_str().unwrap(), "<token-1>"),
        (paths[1]["token"].as_str().unwrap(), "<token-2>"),
    ];
    cli::assert_cli_text_golden("conflict_show_text", stdout, &placeholders);

    // Resolved with the tokens it showed.
    private_source();
    let input = resolve_input(observation(&shown), &resolutions(&shown, "Resolved"));
    let resolved = resolve(b, &operation, R2, &input, "ok", 0);
    assert_eq!(resolved["outcome"], "success");
    assert_effects(
        &resolved,
        &[
            ("write", "written"),
            ("checkpoint", "committed"),
            ("discovery", "current"),
            ("publication", "published"),
            ("integration", "complete"),
        ],
    );
    assert!(resolved["recovery"].as_array().unwrap().is_empty());
    let commit = b.tip(PRIMARY)?.ok_or(FixtureError)?;
    assert_eq!(oid(&resolved["effects"]["commit_oid"]), commit);
    let data = &resolved["data"];
    assert_eq!(data["type"], "conflict_resolution");
    assert_eq!(data["operation_id"], operation.as_str());
    assert_eq!(oid(&data["commit_oid"]), commit);
    assert_eq!(data["publication"], "published");
    assert!(data["conflict"].is_null());
    // One merge commit with ordered parents, holding the resolutions,
    // published by one push: no rebase, no forced push.
    let repo = b.repo()?;
    let merge = fixed(repo.find_commit(commit))?;
    assert_eq!(merge.parent_ids().collect::<Vec<_>>(), [local, incoming]);
    assert_eq!(merge.summary(), Some("Resolve synchronization primary"));
    for id in [ITEM, OTHER_ITEM] {
        assert_eq!(
            blob_text(&repo, commit, &ticket_path(id))?,
            resolved_source(id, "Resolved")
        );
    }
    assert_clean(b)?;
    assert_eq!(p.served(PRIMARY)?, Some(commit));
    assert_eq!(remote_world::accepted(&p.server), pushes + 1);
    assert_eq!(request_show(b, R2)["state"], "finished");
    golden_of(
        "conflict_resolve_ok",
        &resolved,
        b,
        &[
            ("/request_id", "<request-id>"),
            ("/data/operation_id", "<operation-id>"),
            ("/effects/commit_oid", "<commit-oid>"),
        ],
    );
    assert_settled(&show(b, &operation), "complete", "none");

    // The finished request again: its stored result, and nothing runs.
    let opened = connections(&p);
    assert_eq!(resolve(b, &operation, R2, &input, "ok", 0), resolved);
    // The same request ID for another operation is another request.
    let other = repository::OperationId::new().to_string();
    let mismatch = resolve(b, &other, R2, &input, "request_mismatch", 2);
    assert!(mismatch["data"].is_null());
    assert_effects(&mismatch, &[]);
    // A second, different resolution after the first is refused (fact 34).
    let again = resolve(
        b,
        &operation,
        R3,
        &resolve_input(observation(&shown), &resolutions(&shown, "Another")),
        "recovery_required",
        3,
    );
    assert_effects(&again, &[]);
    assert!(again["data"].is_null());
    assert_eq!(again["recovery"][0]["action"], "repo.inspect");
    assert_eq!(again["recovery"].as_array().unwrap().len(), 1);
    no_record(b, R3);
    assert_eq!(connections(&p), opened);
    assert_eq!(b.tip(PRIMARY)?, Some(commit));
    assert_eq!(remote_world::accepted(&p.server), pushes + 1);
    assert_clean(b)?;

    // A, behind, fast-forwards to the resolution, and pushes nothing.
    let behind = repo_sync(a, R4, "already_applied", 0);
    assert_eq!(behind["effects"]["publication"], "current");
    assert_eq!(a.tip(PRIMARY)?, Some(commit));
    assert_eq!(remote_world::accepted(&p.server), pushes + 1);
    Ok(())
}

/// The conflict met in human mode, and its `next:` line followed. Then the
/// Cycle's stale resolutions, each refused before any write: an
/// observation taken before the configuration changed (`external_change`),
/// an incomplete set (`invalid_input`) and a well-formed token of no path
/// of the conflict (`external_change`, R-33). Then the resolution, in human
/// mode with its progress.
fn c4_conflicts_stale_resolution() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let b = &p.b;
    let (local, incoming) = tickets_diverge(&p)?;
    let out = human(
        b,
        &["repo", "sync", "--repo", b.root(), "--request-id", SYNC],
        &[],
    );
    assert_eq!(out.status, 3);
    assert!(out.stdout.is_empty());
    let stderr = text(&out.stderr);
    let operation = stderr
        .lines()
        .find_map(|line| line.strip_prefix("operation: "))
        .unwrap()
        .to_owned();
    assert_eq!(
        stderr,
        format!(
            "{}error: Recovery is required before this can continue.\n\
             code: recovery_required\n\
             request: {SYNC}\n\
             operation: {operation}\n\
             effects: publication=pending integration=pending\n\
             conflict: operation {operation} stopped on a conflict; next action: resolve\n\
             next: manyhands-cli conflict show --repo {root} --operation-id {operation}\n\
             next: request.retry request_id={SYNC}\n\
             next: repo.inspect root={raw}\n",
            progress(&["accepted", "connecting", "fetching", "observing"]),
            root = quoted(b),
            raw = b.root.canonicalize().unwrap().to_str().unwrap(),
        )
    );
    // The `next:` line, run as printed, with `--json`.
    let shown = show(b, &operation);
    assert_reads(&shown, "conflict_pending", "resolve");
    let all = resolutions(&shown, "Resolved");
    let paths: Vec<String> = [ITEM, OTHER_ITEM].map(ticket_path).to_vec();
    let marked: Vec<Vec<u8>> = paths
        .iter()
        .map(|path| worktree_bytes(b, path))
        .collect::<Result<_, _>>()?;
    let unchanged = |shown: &Value| -> Result<(), FixtureError> {
        assert_eq!(b.tip(PRIMARY)?, Some(local));
        for (path, bytes) in paths.iter().zip(&marked) {
            assert_eq!(&worktree_bytes(b, path)?, bytes, "{path}");
        }
        // No attempt began: no request owns the conflict.
        let now = show(b, &operation);
        assert_reads(&now, "conflict_pending", "resolve");
        assert!(now["data"]["request_id"].is_null());
        assert_eq!(now["data"]["historical"], false);
        assert_eq!(now["data"]["observation"], shown["data"]["observation"]);
        Ok(())
    };
    private_source();
    let opened = connections(&p);

    // The configuration changes after `conflict show`: the observation it
    // gave is stale.
    let config = b.root.join(".manyhands/config.toml");
    let configuration = fixed(std::fs::read(&config))?;
    let mut changed = configuration.clone();
    changed.extend_from_slice(b"# changed after conflict show\n");
    fixed(std::fs::write(&config, &changed))?;
    let stale = resolve(
        b,
        &operation,
        R2,
        &resolve_input(observation(&shown), &all),
        "external_change",
        3,
    );
    assert_eq!(stale["outcome"], "blocked");
    assert_effects(&stale, &[]);
    assert!(stale["data"].is_null());
    assert!(stale["recovery"].as_array().unwrap().is_empty());
    no_record(b, R2);
    let fresh = show(b, &operation);
    assert_ne!(fresh["data"]["observation"], shown["data"]["observation"]);
    // The configuration as it was: the conflict reads as it did.
    fixed(std::fs::write(&config, &configuration))?;
    unchanged(&shown)?;

    // An incomplete set: one resolution of the conflict's two paths.
    let incomplete = resolve(
        b,
        &operation,
        R3,
        &resolve_input(observation(&shown), &all[..1]),
        "invalid_input",
        2,
    );
    assert_effects(&incomplete, &[]);
    no_record(b, R3);
    unchanged(&shown)?;
    // A well-formed token of no path of this conflict, in a path's place.
    let mut unknown = all.clone();
    unknown[1].0 = NO_SUCH_TOKEN.to_owned();
    let foreign = resolve(
        b,
        &operation,
        R4,
        &resolve_input(observation(&shown), &unknown),
        "external_change",
        3,
    );
    assert_effects(&foreign, &[]);
    no_record(b, R4);
    unchanged(&shown)?;
    assert_eq!(connections(&p), opened);

    // The resolution, in human mode: C2's lines, its data and its progress.
    let pushes = remote_world::accepted(&p.server);
    let out = human(
        b,
        &resolve_args(b, &operation, R5),
        &resolve_input(observation(&shown), &all),
    );
    assert_eq!(out.status, 0);
    let commit = b.tip(PRIMARY)?.ok_or(FixtureError)?;
    assert_eq!(
        text(&out.stdout),
        format!(
            "The request completed.\n\
             request: {R5}\n\
             effects: write=written checkpoint=committed discovery=current publication=published integration=complete commit={commit}\n\
             Commit oid: {commit}\n\
             Conflict: none\n\
             Operation id: {operation}\n\
             Publication: published\n\
             Type: conflict_resolution\n"
        )
    );
    assert_eq!(
        text(&out.stderr),
        progress(&[
            "accepted",
            "writing",
            "fetching",
            "observing",
            "pushing",
            "verifying",
            "indexing"
        ])
    );
    let parents: Vec<_> = fixed(b.repo()?.find_commit(commit))?.parent_ids().collect();
    assert_eq!(parents, [local, incoming]);
    assert_eq!(p.served(PRIMARY)?, Some(commit));
    assert_eq!(remote_world::accepted(&p.server), pushes + 1);
    Ok(())
}

/// The person's own tooling installs a whole committed merge: the commit,
/// the branch, and a worktree and index that match it. `cleanup` removes
/// the merge metadata, as `git commit` does.
fn install(repo: &git2::Repository, commit: git2::Oid, cleanup: bool) -> Result<(), FixtureError> {
    fixed(repo.checkout_tree(
        &fixed(repo.find_object(commit, None))?,
        Some(git2::build::CheckoutBuilder::new().force()),
    ))?;
    fixed(repo.reference(PRIMARY, commit, true, "an outside repair"))?;
    let mut index = fixed(repo.index())?;
    fixed(index.read_tree(&fixed(fixed(repo.find_commit(commit))?.tree())?))?;
    fixed(index.write())?;
    if cleanup {
        fixed(repo.cleanup_state())?;
    }
    Ok(())
}

/// The Cycle's outside repair of a noncanonical conflict: a code file both
/// clones edited. It is reported for outside repair as a whole, and
/// `conflict resolve` writes nothing. Removing the markers alone, staging
/// without a commit, and three commits with wrong parents (one parent,
/// reversed, a foreign second parent) are not accepted: `conflict show`
/// reads each, and `operation resume` publishes none. Then the person's own
/// two-parent merge, which `conflict show` reads as repaired and
/// `operation resume` publishes, in human mode with its progress.
fn c4_conflicts_outside_repair() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let (a, b) = (&p.a, &p.b);
    // Both clones hold the file: A adds it, and B fast-forwards to it.
    let base = commit_on_primary(a, &[(CODE, "fn shared() {}\n")])?;
    repo_sync(a, R1, "ok", 0);
    repo_sync(b, R2, "already_applied", 0);
    assert_eq!(b.tip(PRIMARY)?, Some(base));
    let incoming = commit_on_primary(a, &[(CODE, "fn alpha() {}\n")])?;
    repo_sync(a, R3, "ok", 0);
    let local = commit_on_primary(b, &[(CODE, "fn beta() {}\n")])?;
    let out = human(
        b,
        &["repo", "sync", "--repo", b.root(), "--request-id", SYNC],
        &[],
    );
    assert_eq!(out.status, 3);
    let stderr = text(&out.stderr);
    let operation = stderr
        .lines()
        .find_map(|line| line.strip_prefix("operation: "))
        .unwrap()
        .to_owned();
    // Outside-repair guidance: the next step, and where to look at it.
    assert!(
        stderr.contains(&format!(
            "conflict: operation {operation} stopped on a conflict; next action: external_repair\n\
             next: manyhands-cli conflict show --repo {} --operation-id {operation}\n",
            quoted(b)
        )),
        "{stderr}"
    );
    let shown = show(b, &operation);
    let data = &shown["data"];
    assert_reads(&shown, "conflict_pending", "external_repair");
    assert_eq!(data["eligibility"], "external_resolution_required");
    assert!(data["observation"].is_string());
    let paths = data["paths"].as_array().unwrap();
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0]["display_path"], CODE);
    assert_eq!(paths[0]["kind"], "noncanonical");
    // The gate's classification (fact 30) gives a regular text file that is
    // no canonical content `binary`, as the library's own read table has it
    // (`conflict_read.rs`, `plain.sh`).
    assert_eq!(paths[0]["structure"], "binary");
    // No item's text: no side (R-10).
    assert_eq!(paths[0]["content_available"], false);
    for side in ["base", "local", "incoming"] {
        assert!(paths[0][side].is_null(), "{side}");
    }
    let out = human(b, &show_args(b, &operation), &[]);
    assert_eq!(out.status, 0);
    assert!(
        text(&out.stdout).contains("next action: external_repair\n"),
        "{}",
        text(&out.stdout)
    );

    let repo = b.repo()?;
    let file = b.root.join(CODE);
    let index_file = repo.path().join("index");
    let marked = fixed(std::fs::read(&file))?;
    let conflicted = fixed(std::fs::read(&index_file))?;
    let opened = connections(&p);
    let updates = p.server.receive_updates();
    // `conflict resolve` writes nothing of an external-only set.
    let refused = resolve(
        b,
        &operation,
        R4,
        &resolve_input(
            observation(&shown),
            &[(
                paths[0]["token"].as_str().unwrap().to_owned(),
                "fn resolved() {}\n".to_owned(),
            )],
        ),
        "recovery_required",
        3,
    );
    assert_effects(&refused, &[]);
    assert_eq!(
        refused["data"],
        serde_json::json!({
            "type": "conflict_resolution",
            "operation_id": operation,
            "commit_oid": null,
            "publication": "not_requested",
            "conflict": {"operation_id": operation, "next_action": "external_repair"},
        })
    );
    assert_eq!(refused["recovery"].as_array().unwrap().len(), 1);
    assert_eq!(refused["recovery"][0]["action"], "repo.inspect");
    no_record(b, R4);
    assert_eq!(fixed(std::fs::read(&file))?, marked);
    assert_eq!(fixed(std::fs::read(&index_file))?, conflicted);

    // `operation resume` of a repair that is not accepted: refused, with no
    // transport and nothing pushed.
    let resumes = std::cell::Cell::new(0u8);
    let not_resumed = |pending: bool| {
        let n = resumes.get();
        resumes.set(n + 1);
        let request = format!("01ARZ3NDEKTSV4RRFFQ69G5FW{n}");
        let resumed = resume(b, &operation, &request, "recovery_required", 3);
        assert_eq!(resumed["outcome"], "blocked");
        assert_eq!(resumed["recovery"].as_array().unwrap().len(), 1);
        assert_eq!(resumed["recovery"][0]["action"], "repo.inspect");
        if pending {
            // The conflict is still pending (design, Resume).
            assert_effects(
                &resumed,
                &[("publication", "pending"), ("integration", "pending")],
            );
            schema::check_published(SYNCHRONIZATION_CONFLICT_SCHEMA, &resumed["data"]).unwrap();
            assert_eq!(
                resumed["data"]["conflict"]["next_action"],
                "external_repair"
            );
        } else {
            assert_effects(&resumed, &[("publication", "pending")]);
            assert!(resumed["data"].is_null());
        }
        no_record(b, &request);
    };
    // The markers removed in the worktree only: still pending.
    fixed(std::fs::write(&file, b"fn repaired() {}\n"))?;
    assert_reads(&show(b, &operation), "conflict_pending", "external_repair");
    not_resumed(true);
    // The path staged, with no commit: refused.
    {
        let mut index = fixed(repo.index())?;
        fixed(index.add_path(std::path::Path::new(CODE)))?;
        fixed(index.write())?;
    }
    assert_settled(&show(b, &operation), "repair_refused", "external_repair");
    not_resumed(false);
    fixed(std::fs::write(&index_file, &conflicted))?;
    fixed(std::fs::write(&file, &marked))?;
    assert_reads(&show(b, &operation), "conflict_pending", "external_repair");
    // Whole, clean commits that are not the recorded merge: refused.
    let repair = [(CODE, "fn repaired() {}\n")];
    let tree = fixed(fixed(repo.find_commit(local))?.tree())?;
    for parents in [vec![local], vec![incoming, local], vec![local, base]] {
        let wrong = commit_object(&repo, &tree, &parents, &repair)?;
        install(&repo, wrong, false)?;
        assert_settled(&show(b, &operation), "repair_refused", "external_repair");
        not_resumed(false);
        assert_eq!(b.tip(PRIMARY)?, Some(wrong));
        // The person undoes it: branch, index and worktree as they were.
        fixed(repo.reference(PRIMARY, local, true, "the wrong repair undone"))?;
        fixed(std::fs::write(&index_file, &conflicted))?;
        fixed(std::fs::write(&file, &marked))?;
        assert_reads(&show(b, &operation), "conflict_pending", "external_repair");
    }
    assert_eq!(connections(&p), opened);
    assert_eq!(p.server.receive_updates(), updates);
    assert_eq!(p.served(PRIMARY)?, Some(incoming));

    // The person's own two-parent merge, committed in the worktree.
    let repaired = commit_object(&repo, &tree, &[local, incoming], &repair)?;
    install(&repo, repaired, true)?;
    assert_settled(&show(b, &operation), "repaired", "resume");
    let out = human(b, &resume_args(b, &operation, R5), &[]);
    assert_eq!(out.status, 0);
    assert_eq!(
        text(&out.stdout),
        format!(
            "The request completed.\n\
             request: {R5}\n\
             effects: discovery=current publication=published integration=complete commit={repaired}\n"
        )
    );
    assert_eq!(
        text(&out.stderr),
        progress(&[
            "accepted",
            "observing",
            "fetching",
            "observing",
            "pushing",
            "verifying",
            "indexing"
        ])
    );
    remote_world::one_update(&p.server, updates.len(), PRIMARY, incoming, repaired);
    assert_eq!(b.tip(PRIMARY)?, Some(repaired));
    assert_clean(b)?;
    assert_settled(&show(b, &operation), "complete", "none");
    // A, behind, fast-forwards to the repair.
    repo_sync(a, R6, "already_applied", 0);
    assert_eq!(a.tip(PRIMARY)?, Some(repaired));
    Ok(())
}

/// The peer, clone B, deletes the item's branch on the server with its own
/// Git, as another person would.
fn peer_deletes(p: &CliPair) -> Result<(), FixtureError> {
    let old = p.served(CONTEXT)?.ok_or(FixtureError)?;
    let before = p.server.receive_updates().len();
    let repo = p.b.repo()?;
    let mut remote = fixed(repo.remote_anonymous(&p.server.url()))?;
    let mut options = git2::PushOptions::new();
    options.remote_callbacks(cli_pair::callbacks(&p.server)?);
    fixed(remote.push(&[&format!(":{CONTEXT}")], Some(&mut options)))?;
    p.server.fixture_push_transport_returned()?;
    p.server.wait_for_receive_update(
        before,
        &ReceiveUpdate {
            reference: CONTEXT.into(),
            old_oid: old,
            new_oid: git2::Oid::zero(),
            accepted: true,
        },
    )?;
    assert_eq!(p.served(CONTEXT)?, None);
    Ok(())
}

/// Carry 2 (Task 9's concern 1, M9): an item conflict between the clones,
/// then the item's branch deleted on the server while the conflict is
/// pending, then the conflict resolved. The continuation meets the
/// deletion: `remote_branch_deleted` after the checkpoint, exit 4. Human
/// output names the request that holds the synchronization, B's `item
/// sync`, in the cancel line and the "send again" line, and the republish
/// of the item the conflict read names (the resolve's scope names none).
/// The way out they name then frees the reservation, and a real republish
/// publishes the resolution.
fn c4_conflicts_resolve_meets_a_deleted_branch() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let (a, b) = (&p.a, &p.b);
    save(a, R1, "Saved in clone A.\n");
    let published = item_sync(a, ITEM, R2, "ok", 0);
    assert_eq!(published["effects"]["publication"], "published");
    let incoming = a.tip(CONTEXT)?.ok_or(FixtureError)?;
    save(b, R3, "Saved in clone B.\n");
    let local = b.tip(CONTEXT)?.ok_or(FixtureError)?;
    let stopped = item_sync(b, ITEM, SYNC, "recovery_required", 3);
    let operation = stopped_with(&stopped, "resolve");
    let shown = show(b, &operation);
    let data = &shown["data"];
    for (key, expected) in [
        ("target", "ticket"),
        ("item_id", ITEM),
        ("stage", "context"),
        ("state", "conflict_pending"),
        ("next_action", "resolve"),
    ] {
        assert_eq!(data[key], expected, "{key}");
    }
    assert_eq!(oid(&data["local_parent"]), local);
    assert_eq!(oid(&data["incoming_parent"]), incoming);
    assert_eq!(data["paths"].as_array().unwrap().len(), 1);
    assert_eq!(data["paths"][0]["display_path"], ticket_path(ITEM));
    peer_deletes(&p)?;
    let pushes = remote_world::accepted(&p.server);

    private_source();
    let input = resolve_input(observation(&shown), &resolutions(&shown, "Shared"));
    let out = human(b, &resolve_args(b, &operation, R4), &input);
    assert_eq!(out.status, 4);
    assert!(out.stdout.is_empty());
    // The run made the resolution commit: B's context tip.
    let commit = b.tip(CONTEXT)?.ok_or(FixtureError)?;
    let parents: Vec<_> = fixed(b.repo()?.find_commit(commit))?.parent_ids().collect();
    assert_eq!(parents, [local, incoming]);
    let stderr = text(&out.stderr);
    assert!(
        stderr.starts_with(&format!(
            "{}{DELETED}request: {R4}\n\
             effects: write=written checkpoint=committed discovery=pending publication=pending integration=complete commit={commit}\n",
            progress(&["accepted", "writing", "fetching", "observing"])
        )),
        "{stderr}"
    );
    assert_eq!(
        next_lines(stderr),
        [
            format!("next: manyhands-cli request cancel --request-id {SYNC}"),
            format!("next: send request {SYNC} again with its original command and input"),
            format!(
                "next: manyhands-cli item republish --repo {} --id {ITEM} --prepare",
                quoted(b)
            ),
        ]
    );
    assert!(
        stderr.ends_with(&RESERVATION_NOTE.replacen(
            "the refused request",
            &format!("request {SYNC}"),
            1
        )),
        "{stderr}"
    );
    // The read human output used: it names the item.
    let read = show(b, &operation);
    assert_settled(&read, "resolved", "resume");
    assert_eq!(read["data"]["item_id"], ITEM);
    // JSON: the library's envelope, the cancellation of the holder offered.
    let refused = resolve(b, &operation, R4, &input, "remote_branch_deleted", 4);
    assert_eq!(refused["outcome"], "partial");
    assert_effects(
        &refused,
        &[
            ("write", "written"),
            ("checkpoint", "committed"),
            ("discovery", "pending"),
            ("publication", "pending"),
            ("integration", "complete"),
        ],
    );
    assert_eq!(oid(&refused["effects"]["commit_oid"]), commit);
    assert_eq!(refused["data"]["publication"], "pending");
    assert_eq!(
        refused["recovery"],
        serde_json::json!([{"action": "request.cancel", "operation_id": null, "arguments": {"request_id": SYNC}}])
    );
    // B's synchronization holds the reservation, and nothing was pushed.
    assert_eq!(request_show(b, SYNC)["state"], "accepted");
    assert_eq!(p.served(CONTEXT)?, None);
    assert_eq!(remote_world::accepted(&p.server), pushes);
    assert_eq!(b.tip(CONTEXT)?, Some(commit));

    // The way out the lines name, followed.
    let asked = checked(
        &b.env,
        &["request", "cancel", "--request-id", SYNC],
        &[],
        "ok",
        0,
        None,
    );
    assert_eq!(asked["data"]["requested"], true);
    let ended = item_sync(b, ITEM, SYNC, "cancelled", 130);
    assert_eq!(ended["outcome"], "cancelled");
    assert_eq!(request_show(b, SYNC)["state"], "finished");
    let preview = prepare(b);
    assert_eq!(oid(&preview["data"]["details"]["local_oid"]), commit);
    // B never published the branch; its fetch saw A's publication.
    assert_eq!(
        oid(&preview["data"]["details"]["last_published_oid"]),
        incoming
    );
    let republished = confirm(b, R5, &preview, "ok", 0);
    assert_eq!(republished["outcome"], "success");
    assert_eq!(oid(&republished["effects"]["commit_oid"]), commit);
    assert_eq!(p.served(CONTEXT)?, Some(commit));
    assert_eq!(remote_world::accepted(&p.server), pushes + 1);
    assert_eq!(
        p.server.receive_updates().last(),
        Some(&ReceiveUpdate {
            reference: CONTEXT.into(),
            old_oid: git2::Oid::zero(),
            new_oid: commit,
            accepted: true,
        })
    );
    // A, behind, fast-forwards to the resolution.
    item_sync(a, ITEM, R6, "already_applied", 0);
    assert_eq!(a.tip(CONTEXT)?, Some(commit));
    Ok(())
}

/// A's editing context, made by a ticket save and published by `item
/// sync`, then deleted on the server by the peer. Returns A's tip, the
/// commit that was published.
pub(super) fn published_then_deleted(p: &CliPair) -> Result<git2::Oid, FixtureError> {
    let a = &p.a;
    save(a, R1, "Saved in clone A.\n");
    let published = item_sync(a, ITEM, R2, "ok", 0);
    assert_eq!(published["effects"]["publication"], "published");
    let tip = a.tip(CONTEXT)?.ok_or(FixtureError)?;
    assert_eq!(p.served(CONTEXT)?, Some(tip));
    peer_deletes(p)?;
    Ok(tip)
}

fn sync_args(clone: &CliClone) -> Vec<&str> {
    vec![
        "item",
        "sync",
        "--repo",
        clone.root(),
        "--id",
        ITEM,
        "--request-id",
        SYNC,
    ]
}

/// A's `item sync` under `SYNC`, in JSON, refused for the deleted branch:
/// it does not recreate it, and holds the reservation.
pub(super) fn sync_refused(p: &CliPair) -> Result<(), FixtureError> {
    let refused = item_sync(&p.a, ITEM, SYNC, "remote_branch_deleted", 3);
    assert_eq!(refused["outcome"], "blocked");
    assert_effects(&refused, &[("publication", "pending")]);
    assert!(refused["data"].is_null());
    assert_eq!(
        refused["recovery"],
        serde_json::json!([{"action": "request.cancel", "operation_id": null, "arguments": {"request_id": SYNC}}])
    );
    assert_eq!(p.served(CONTEXT)?, None);
    assert_eq!(request_show(&p.a, SYNC)["state"], "accepted");
    Ok(())
}

/// The refused `item sync` under `SYNC` cancelled and sent once more, which
/// ends it and lets the reservation go.
pub(super) fn release(clone: &CliClone) {
    let asked = checked(
        &clone.env,
        &["request", "cancel", "--request-id", SYNC],
        &[],
        "ok",
        0,
        None,
    );
    assert_eq!(asked["data"]["requested"], true);
    let ended = item_sync(clone, ITEM, SYNC, "cancelled", 130);
    assert_eq!(ended["outcome"], "cancelled");
    assert_eq!(request_show(clone, SYNC)["state"], "finished");
}

fn republish_args<'a>(clone: &'a CliClone, item: &'a str) -> Vec<&'a str> {
    vec!["item", "republish", "--repo", clone.root(), "--id", item]
}

/// `item republish --prepare` of the item, in JSON: a preview.
pub(super) fn prepare(clone: &CliClone) -> Value {
    let mut args = republish_args(clone, ITEM);
    args.push("--prepare");
    let value = checked(&clone.env, &args, &[], "ok", 0, Some(PREVIEW_SCHEMA));
    assert_eq!(value["command"], "item republish");
    // A preview is of no request and no operation.
    assert!(value["request_id"].is_null());
    assert!(value["operation_id"].is_null());
    assert_eq!(
        value["data"]["effects"],
        serde_json::json!([{"effect": "branch_republished"}])
    );
    let details = &value["data"]["details"];
    assert_eq!(details["kind"], "ticket");
    assert_eq!(details["item_id"], ITEM);
    assert_eq!(
        details["branch"],
        CONTEXT.strip_prefix("refs/heads/").unwrap()
    );
    assert_eq!(details["remote"], "origin");
    value
}

pub(super) fn confirm_args<'a>(
    clone: &'a CliClone,
    item: &'a str,
    request: &'a str,
    confirmation: &'a str,
) -> Vec<&'a str> {
    let mut args = republish_args(clone, item);
    args.extend(["--request-id", request, "--confirm", confirmation]);
    args
}

pub(super) fn confirmation(preview: &Value) -> &str {
    preview["data"]["confirmation_id"].as_str().unwrap()
}

/// `item republish --confirm` of the item, in JSON, with `preview`'s
/// confirmation.
pub(super) fn confirm(
    clone: &CliClone,
    request: &str,
    preview: &Value,
    code: &str,
    status: i32,
) -> Value {
    let value = clone.json(
        &confirm_args(clone, ITEM, request, confirmation(preview)),
        &[],
        code,
        status,
    );
    assert_eq!(value["command"], "item republish");
    assert_eq!(value["request_id"], request);
    value
}

/// Asserts that a republish published exactly `tip`, creating the branch on
/// the server with one push, and integrated nothing.
pub(super) fn assert_republished(
    p: &CliPair,
    value: &Value,
    tip: git2::Oid,
    updates: usize,
    primary: Option<git2::Oid>,
) -> Result<(), FixtureError> {
    assert_eq!(value["code"], "ok");
    assert_eq!(value["outcome"], "success");
    assert_effects(
        value,
        &[("discovery", "current"), ("publication", "published")],
    );
    assert_eq!(oid(&value["effects"]["commit_oid"]), tip);
    assert!(value["data"].is_null());
    assert!(value["recovery"].as_array().unwrap().is_empty());
    remote_world::one_update(&p.server, updates, CONTEXT, git2::Oid::zero(), tip);
    assert_eq!(p.served(CONTEXT)?, Some(tip));
    assert_eq!(p.a.tip(CONTEXT)?, Some(tip));
    assert_eq!(p.a.tip(PRIMARY)?, primary);
    Ok(())
}

/// Carry 1, the republish table's row 13: the refused `item sync` still
/// holds its reservation. In human mode `item sync`'s refusal says how to
/// free it; the republish preview reserves nothing; the confirmed republish
/// is `busy`, exit 5, after C2's bounded retry, with Task 10's note before
/// the retry line. Once the `item sync` is cancelled and sent once more,
/// the same republish request publishes, with its progress; its replay;
/// and `request_mismatch` for the same request ID with another `--id`.
fn c4_conflicts_republish_busy_then_confirmed() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let a = &p.a;
    let tip = published_then_deleted(&p)?;
    let primary = a.tip(PRIMARY)?;

    // `item sync`'s refusal, in human mode.
    let out = human(a, &sync_args(a), &[]);
    assert_eq!(out.status, 3);
    assert!(out.stdout.is_empty());
    let stderr = text(&out.stderr);
    assert!(
        stderr.starts_with(&format!(
            "{}{DELETED}request: {SYNC}\n",
            progress(&["accepted", "connecting", "fetching", "observing"])
        )),
        "{stderr}"
    );
    assert!(
        stderr.contains("effects: publication=pending\n"),
        "{stderr}"
    );
    assert_eq!(
        next_lines(stderr),
        [
            format!("next: manyhands-cli request cancel --request-id {SYNC}"),
            format!(
                "next: manyhands-cli item sync --repo {} --id {ITEM} --request-id {SYNC}",
                quoted(a)
            ),
            format!(
                "next: manyhands-cli item republish --repo {} --id {ITEM} --prepare",
                quoted(a)
            ),
        ]
    );
    assert!(stderr.ends_with(RESERVATION_NOTE), "{stderr}");
    // It does not recreate the branch, and holds the reservation.
    assert_eq!(p.served(CONTEXT)?, None);
    assert_eq!(request_show(a, SYNC)["state"], "accepted");

    let opened = connections(&p);
    let preview = prepare(a);
    assert_eq!(oid(&preview["data"]["details"]["local_oid"]), tip);
    assert_eq!(oid(&preview["data"]["details"]["last_published_oid"]), tip);
    // A preview reads what is stored: it connects to nothing.
    assert_eq!(connections(&p), opened);

    // The confirmed republish meets the reservation.
    let pushes = remote_world::accepted(&p.server);
    let confirmed = confirm_args(a, ITEM, R3, confirmation(&preview));
    let out = human(a, &confirmed, &[]);
    assert_eq!(out.status, 5);
    assert!(out.stdout.is_empty());
    let stderr = text(&out.stderr);
    // The busy result names the operation of its last attempt.
    let operation = stderr
        .lines()
        .find_map(|line| line.strip_prefix("operation: "))
        .unwrap();
    // C2's bounded retry: three re-runs, each after a `retrying` line; then
    // C2's refusal, Task 10's note, and the retry line.
    let attempt = progress(&["accepted", "connecting"]);
    assert_eq!(
        stderr,
        format!(
            "{attempt}progress: retrying\n{attempt}progress: retrying\n{attempt}progress: retrying\n{attempt}\
             error: The repository is busy; try again.\n\
             code: busy\n\
             request: {R3}\n\
             operation: {operation}\n\
             {BUSY_NOTE}\
             next: request.retry request_id={R3}\n"
        )
    );
    no_record(a, R3);
    assert_eq!(p.served(CONTEXT)?, None);
    assert_eq!(remote_world::accepted(&p.server), pushes);
    assert_eq!(request_show(a, SYNC)["state"], "accepted");

    // The way out the note names, then the same republish request.
    release(a);
    let updates = p.server.receive_updates().len();
    let out = human(a, &confirmed, &[]);
    assert_eq!(out.status, 0);
    let stdout = text(&out.stdout);
    assert!(
        stdout.starts_with(&format!(
            "The request completed.\nrequest: {R3}\noperation: "
        )),
        "{stdout}"
    );
    assert!(
        stdout.ends_with(&format!(
            "effects: discovery=current publication=published commit={tip}\n"
        )),
        "{stdout}"
    );
    assert_eq!(
        text(&out.stderr),
        progress(&[
            "accepted",
            "connecting",
            "fetching",
            "observing",
            "pushing",
            "verifying",
            "indexing"
        ])
    );
    let replay = confirm(a, R3, &preview, "ok", 0);
    assert_republished(&p, &replay, tip, updates, primary)?;
    assert_eq!(request_show(a, R3)["state"], "finished");

    // The finished request again: its stored result, and nothing runs.
    let opened = connections(&p);
    assert_eq!(confirm(a, R3, &preview, "ok", 0), replay);
    // The same request ID for another item is another request.
    let mismatch = a.json(
        &confirm_args(a, OTHER_ITEM, R3, confirmation(&preview)),
        &[],
        "request_mismatch",
        2,
    );
    assert!(mismatch["data"].is_null());
    assert_effects(&mismatch, &[]);
    assert_eq!(
        a.tip(&format!("refs/heads/manyhands/ticket/{OTHER_ITEM}"))?,
        None
    );
    assert_eq!(connections(&p), opened);
    assert_eq!(p.server.receive_updates().len(), updates + 1);
    Ok(())
}

/// After the peer deletes the branch: `item sync`'s refusal in JSON; the
/// `item_republish_preview` golden; confirm, and the `item_republish_ok`
/// golden: exactly the shown commit pushed, once, integrating nothing; then
/// `item sync`, primary unchanged, finds the remote current (row 17).
fn c4_conflicts_republish_confirmed() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let a = &p.a;
    let tip = published_then_deleted(&p)?;
    let primary = a.tip(PRIMARY)?;
    sync_refused(&p)?;
    release(a);
    let opened = connections(&p);
    let preview = prepare(a);
    assert_eq!(connections(&p), opened);
    let details = &preview["data"]["details"];
    assert_eq!(oid(&details["local_oid"]), tip);
    assert_eq!(oid(&details["last_published_oid"]), tip);
    golden_of(
        "item_republish_preview",
        &preview,
        a,
        &[
            ("/data/confirmation_id", "<confirmation-id>"),
            ("/data/expires_at", "<expires-at>"),
            ("/data/details/local_oid", "<commit-oid>"),
        ],
    );
    let updates = p.server.receive_updates().len();
    let republished = confirm(a, R3, &preview, "ok", 0);
    assert_republished(&p, &republished, tip, updates, primary)?;
    assert_eq!(republished["scope"]["item_id"], ITEM);
    golden_of(
        "item_republish_ok",
        &republished,
        a,
        &[
            ("/request_id", "<request-id>"),
            ("/operation_id", "<operation-id>"),
            ("/effects/commit_oid", "<commit-oid>"),
        ],
    );
    let pushes = remote_world::accepted(&p.server);
    let after = item_sync(a, ITEM, R4, "already_applied", 0);
    assert_eq!(after["effects"]["publication"], "current");
    assert_eq!(remote_world::accepted(&p.server), pushes);
    assert_eq!(a.tip(CONTEXT)?, Some(tip));
    Ok(())
}

/// A preview, then the local branch moves by a person's own ticket save:
/// the confirmation is refused, `external_change`, with nothing pushed and
/// a new preview offered. The new preview shows the new commit, which its
/// confirmation pushes.
fn c4_conflicts_republish_local_moved() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let a = &p.a;
    let tip = published_then_deleted(&p)?;
    let primary = a.tip(PRIMARY)?;
    sync_refused(&p)?;
    release(a);
    let stale = prepare(a);
    save(a, R5, "Saved again in clone A.\n");
    let moved = a.tip(CONTEXT)?.ok_or(FixtureError)?;
    assert_ne!(moved, tip);
    let opened = connections(&p);
    let pushes = remote_world::accepted(&p.server);
    let refused = confirm(a, R3, &stale, "external_change", 3);
    assert_eq!(refused["outcome"], "blocked");
    assert_effects(&refused, &[]);
    assert!(refused["data"].is_null());
    assert_eq!(
        refused["recovery"],
        serde_json::json!([{"action": "request.prepare", "operation_id": null, "arguments": {}}])
    );
    no_record(a, R3);
    assert_eq!(connections(&p), opened);
    assert_eq!(remote_world::accepted(&p.server), pushes);
    assert_eq!(p.served(CONTEXT)?, None);

    let fresh = prepare(a);
    assert_eq!(oid(&fresh["data"]["details"]["local_oid"]), moved);
    assert_eq!(oid(&fresh["data"]["details"]["last_published_oid"]), tip);
    let updates = p.server.receive_updates().len();
    let republished = confirm(a, R6, &fresh, "ok", 0);
    assert_republished(&p, &republished, moved, updates, primary)?;
    Ok(())
}

/// A transport stop after the request is accepted: the server drops the
/// connection at its advertisement. `transport_unavailable`, exit 5, with
/// nothing pushed and the record accepted; the same request, sent again,
/// completes it in human mode, under the same operation.
fn c4_conflicts_republish_transport_stop() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let a = &p.a;
    let tip = published_then_deleted(&p)?;
    let primary = a.tip(PRIMARY)?;
    sync_refused(&p)?;
    release(a);
    let preview = prepare(a);
    let pushes = remote_world::accepted(&p.server);
    p.server.disconnect_at(FixtureBoundary::Advertisement);
    let stopped = confirm(a, R3, &preview, "transport_unavailable", 5);
    p.server.clear_fault();
    assert_eq!(stopped["outcome"], "error");
    assert_effects(&stopped, &[("publication", "pending")]);
    assert_eq!(
        stopped["recovery"],
        serde_json::json!([{"action": "request.retry", "operation_id": null, "arguments": {"request_id": R3}}])
    );
    let operation = stopped["operation_id"].as_str().unwrap().to_owned();
    assert_eq!(request_show(a, R3)["state"], "accepted");
    assert_eq!(remote_world::accepted(&p.server), pushes);
    assert_eq!(p.served(CONTEXT)?, None);

    let updates = p.server.receive_updates().len();
    let out = human(a, &confirm_args(a, ITEM, R3, confirmation(&preview)), &[]);
    assert_eq!(out.status, 0);
    assert_eq!(
        text(&out.stdout),
        format!(
            "The request completed.\n\
             request: {R3}\n\
             operation: {operation}\n\
             effects: discovery=current publication=published commit={tip}\n"
        )
    );
    // Taken up where it stopped: it was accepted already.
    assert_eq!(
        text(&out.stderr),
        progress(&[
            "connecting",
            "fetching",
            "observing",
            "pushing",
            "verifying",
            "indexing"
        ])
    );
    let replay = confirm(a, R3, &preview, "ok", 0);
    assert_eq!(replay["operation_id"], operation.as_str());
    assert_republished(&p, &replay, tip, updates, primary)?;
    assert_eq!(request_show(a, R3)["state"], "finished");
    Ok(())
}

#[cfg(unix)]
mod unix {
    use super::*;
    use support::pty::PtyCli;

    /// Task 9's note on `original_request_required` of `conflict resolve`.
    const EDITED_NOTE: &str = "note: the attempt's written paths were edited by hand. Restore each file the request wrote to the request's resolution; a file it has not written yet may keep its conflicted content. Then send that request again, or a new conflict resolve may supersede it.\n";
    /// Task 9's note on `external_change` of `conflict resolve` with written
    /// paths.
    const HAND_EDIT_NOTE: &str = "note: a resolved file edited by hand must be restored to the request's resolution before the request is sent again.\n";

    /// Puts a directory's mode back when dropped, however the case ends
    /// short of the process being killed, so the tree stays deletable.
    struct ModeRestored {
        directory: std::path::PathBuf,
        mode: std::fs::Permissions,
    }

    impl Drop for ModeRestored {
        fn drop(&mut self) {
            let _ = std::fs::set_permissions(&self.directory, self.mode.clone());
        }
    }

    fn interactive(clone: &CliClone) -> PtyCli {
        PtyCli::spawn(
            &republish_args(clone, ITEM)
                .iter()
                .map(OsStr::new)
                .collect::<Vec<_>>(),
            &clone.env,
            &[],
        )
    }

    /// The request a run at a terminal allocated, from its first line.
    fn allocated(stderr: &str) -> String {
        stderr
            .lines()
            .next()
            .and_then(|line| line.strip_prefix("request: "))
            .unwrap()
            .to_owned()
    }

    /// Review Focus 4 over SSH: a Ctrl-C at `item republish`'s interactive
    /// consent prompt is the prompt's own cancellation, exit 130, before
    /// execution: no progress, no record, nothing reserved and nothing
    /// pushed, and the terminal as it was. Consent given at the terminal
    /// then republishes.
    pub fn c4_conflicts_republish_ctrl_c_at_consent() -> Result<(), FixtureError> {
        let p = CliPair::new()?;
        let a = &p.a;
        let tip = published_then_deleted(&p)?;
        let primary = a.tip(PRIMARY)?;
        sync_refused(&p)?;
        release(a);
        let opened = connections(&p);
        let updates = p.server.receive_updates().len();
        let mut pty = interactive(a);
        pty.expect("Proceed? [y/N] ");
        pty.send(&[3]);
        let (before, after) = pty.lflag_before_and_after();
        assert_eq!(after, before);
        let (out, transcript) = pty.finish();
        scan_output(&out);
        assert_eq!(out.status, 130);
        assert!(out.stdout.is_empty());
        let stderr = text(&out.stderr);
        let request = allocated(stderr);
        assert_eq!(
            stderr,
            format!(
                "request: {request}\nerror: The request was cancelled.\ncode: cancelled\nrequest: {request}\n"
            )
        );
        // The preview was shown at the terminal, and nothing ran. The
        // terminal ends each line with a carriage return too.
        let shown: Vec<&str> = transcript.lines().map(str::trim_end).collect();
        for line in [
            "Preview of item republish".to_owned(),
            format!("Local oid: {tip}"),
            "Will push the deleted branch again at the local commit.".to_owned(),
        ] {
            assert!(shown.contains(&line.as_str()), "{transcript}");
        }
        no_record(a, &request);
        assert_eq!(connections(&p), opened);
        assert_eq!(p.served(CONTEXT)?, None);
        assert_eq!(p.server.receive_updates().len(), updates);

        // Nothing is reserved: consent at the terminal republishes.
        let mut pty = interactive(a);
        pty.expect("Proceed? [y/N] ");
        pty.send(b"yes\n");
        let (out, _) = pty.finish();
        scan_output(&out);
        assert_eq!(out.status, 0, "{}", text(&out.stderr));
        let request = allocated(text(&out.stderr));
        assert!(
            text(&out.stdout).starts_with(&format!("The request completed.\nrequest: {request}\n")),
            "{}",
            text(&out.stdout)
        );
        remote_world::one_update(&p.server, updates, CONTEXT, git2::Oid::zero(), tip);
        assert_eq!(a.tip(PRIMARY)?, primary);
        assert_eq!(request_show(a, &request)["state"], "finished");
        Ok(())
    }

    /// Carry 3, Part A ruling I1, through the binary. A resolution stops
    /// after writing its first path: the second path's directory refuses the
    /// write, as a read-only directory does. That is the person's file
    /// system, not a test hook; it answers `internal_error` after the write,
    /// and `conflict show` reads the attempt as `resolution_prepared`, its
    /// request named. The written path is then edited by hand:
    /// - a new request is `original_request_required`, naming the first
    ///   (design, interrupted resolution row 8), with Task 9's note;
    /// - the first request is `external_change`, with the restore note;
    /// - once the path holds the request's resolution again, the first
    ///   request finishes it, and publishes.
    pub fn c4_conflicts_hand_edit_after_written_paths() -> Result<(), FixtureError> {
        use std::os::unix::fs::PermissionsExt;
        let p = CliPair::new()?;
        let b = &p.b;
        let (local, incoming) = tickets_diverge(&p)?;
        let operation = stopped_on_primary(&p, "resolve");
        let shown = show(b, &operation);
        let input = resolve_input(observation(&shown), &resolutions(&shown, "Resolved"));
        let (written, refusing) = (ticket_path(ITEM), ticket_path(OTHER_ITEM));
        let directory = b.root.join(&refusing).parent().unwrap().to_owned();
        let marked = worktree_bytes(b, &refusing)?;
        let pushes = remote_world::accepted(&p.server);
        private_source();

        let mode = fixed(std::fs::metadata(&directory))?.permissions();
        let original = mode.mode();
        fixed(std::fs::set_permissions(
            &directory,
            std::fs::Permissions::from_mode(0o555),
        ))?;
        let restored = ModeRestored {
            directory: directory.clone(),
            mode,
        };
        // The staging needs a user the permission binds.
        assert!(std::fs::write(directory.join("probe"), b"").is_err());
        let out = human(b, &resolve_args(b, &operation, R2), &input);
        drop(restored);
        assert_eq!(
            fixed(std::fs::metadata(&directory))?.permissions().mode(),
            original
        );
        assert_eq!(out.status, 4);
        assert!(out.stdout.is_empty());
        assert_eq!(
            text(&out.stderr),
            format!(
                "{}error: An internal error occurred.\ncode: internal_error\nrequest: {R2}\neffects: write=written\n",
                progress(&["accepted", "writing"])
            )
        );
        // The first path holds the request's resolution; the second, its
        // conflict.
        assert_eq!(
            worktree_bytes(b, &written)?,
            resolved_source(ITEM, "Resolved").into_bytes()
        );
        assert_eq!(worktree_bytes(b, &refusing)?, marked);
        let read = show(b, &operation);
        assert_reads(&read, "resolution_prepared", "retry_resolution");
        assert_eq!(read["data"]["request_id"], R2);
        assert_eq!(read["data"]["historical"], true);
        assert_eq!(read["data"]["observation"], shown["data"]["observation"]);
        assert_eq!(b.tip(PRIMARY)?, Some(local));

        // The written path edited by hand.
        let edited = source(ITEM, "Edited by hand", "Edited by hand.").into_bytes();
        fixed(std::fs::write(b.root.join(&written), &edited))?;
        // A new request may not supersede the attempt.
        let out = human(
            b,
            &resolve_args(b, &operation, R3),
            &resolve_input(observation(&shown), &resolutions(&shown, "Superseding")),
        );
        assert_eq!(out.status, 3);
        assert!(out.stdout.is_empty());
        assert_eq!(
            text(&out.stderr),
            format!(
                "{}error: The original request must be submitted again to finish the operation.\n\
                 code: original_request_required\n\
                 request: {R3}\n\
                 next: send request {R2} again with its original command and input\n\
                 {EDITED_NOTE}",
                progress(&["accepted", "writing"])
            )
        );
        no_record(b, R3);
        // The first request may not write over the edit.
        let out = human(b, &resolve_args(b, &operation, R2), &input);
        assert_eq!(out.status, 3);
        assert!(out.stdout.is_empty());
        assert_eq!(
            text(&out.stderr),
            format!(
                "{}error: What the request was based on has changed.\n\
                 code: external_change\n\
                 request: {R2}\n\
                 {HAND_EDIT_NOTE}",
                progress(&["accepted", "writing"])
            )
        );
        assert_eq!(worktree_bytes(b, &written)?, edited);
        assert_eq!(worktree_bytes(b, &refusing)?, marked);
        assert_eq!(b.tip(PRIMARY)?, Some(local));
        assert_eq!(remote_world::accepted(&p.server), pushes);

        // Restored to the request's resolution: the request goes on.
        fixed(std::fs::write(
            b.root.join(&written),
            resolved_source(ITEM, "Resolved"),
        ))?;
        let resolved = resolve(b, &operation, R2, &input, "ok", 0);
        assert_eq!(resolved["data"]["publication"], "published");
        let commit = b.tip(PRIMARY)?.ok_or(FixtureError)?;
        assert_eq!(oid(&resolved["effects"]["commit_oid"]), commit);
        let repo = b.repo()?;
        let parents: Vec<_> = fixed(repo.find_commit(commit))?.parent_ids().collect();
        assert_eq!(parents, [local, incoming]);
        for id in [ITEM, OTHER_ITEM] {
            assert_eq!(
                blob_text(&repo, commit, &ticket_path(id))?,
                resolved_source(id, "Resolved")
            );
        }
        assert_eq!(p.served(PRIMARY)?, Some(commit));
        assert_eq!(remote_world::accepted(&p.server), pushes + 1);
        assert_clean(b)?;
        Ok(())
    }
}
