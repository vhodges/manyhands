//! C4's comment and synchronization journeys through the binary, between
//! two clones of one SSH remote (`CliPair`, R-24). Each case is one fresh
//! pair, under the harness's watchdog.
//!
//! The library's tables (`tests/mutation_remote/comment.rs`, `merging.rs`,
//! `resume.rs`) pin what the library answers; these cases pin the same rows
//! as a person meets them: the exit, the envelope, human mode's `progress:`
//! lines and `next:` lines, what the server and each clone hold, a replay
//! of a finished request with no second effect, and `request_mismatch` for
//! a request ID sent again with another target. Every run goes through
//! `scan_output`, and the comment bodies a case never lists are probes no
//! output may hold.

use super::*;
use cli_pair::{CONTEXT, CliClone, CliPair, ITEM, OTHER_ITEM, PRIMARY};
use ssh_remote::{FixtureBoundary, FixtureError, fixed};

pub const CASES: &[ssh_harness::Case] = &[
    (
        "c4_collaboration_comment_publishes",
        c4_collaboration_comment_publishes,
    ),
    (
        "c4_collaboration_comment_refused_then_saved_once",
        c4_collaboration_comment_refused_then_saved_once,
    ),
    (
        "c4_collaboration_replies_from_both_clones",
        c4_collaboration_replies_from_both_clones,
    ),
    ("c4_collaboration_item_sync", c4_collaboration_item_sync),
    ("c4_collaboration_repo_sync", c4_collaboration_repo_sync),
    (
        "c4_collaboration_resume_publishes_a_stopped_comment",
        c4_collaboration_resume_publishes_a_stopped_comment,
    ),
    (
        "c4_collaboration_comment_meets_a_deleted_branch",
        c4_collaboration_comment_meets_a_deleted_branch,
    ),
    (
        "c4_collaboration_resume_meets_a_deleted_branch",
        c4_collaboration_resume_meets_a_deleted_branch,
    ),
];

const R1: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR1";
const R2: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR2";
const R3: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR3";
const R4: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR4";
const R5: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR5";
const R6: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR6";
const C1: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC1";
const C2: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC2";
const C3: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC3";
const C4: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC4";
/// A comment body no output may hold: a case that uses it never lists it.
const PRIVATE_BODY: &str = "cli-pair-private-comment-body-sentinel";
const AUTHOR_A: &str = "Clone A <clone-a@example.invalid>";
const AUTHOR_B: &str = "Clone B <clone-b@example.invalid>";
const COMMENT_SCHEMA: &str = "comment_mutation.schema.json";
const PUBLICATION_NOTE: &str = "note: publishing the comment also publishes this item's other checkpointed work; unsaved drafts are not included.\n";
const RESERVATION_NOTE: &str = "note: the refused request keeps its reservation until it is cancelled; sending it again then ends it and releases the reservation.\n";
const RESUME_DELETED_NOTE: &str = "note: the request that started this publication, its comment add or item sync, still holds the reservation: run request cancel with that request's ID, then send that request once more, then run item republish.\n";

/// Makes `PRIVATE_BODY` a probe of every later run's output.
fn private_body() {
    OUTPUT_PROBES.with(|probes| probes.borrow_mut().push(PRIVATE_BODY.to_owned()));
}

fn comment_input(comment: &str, parent: Option<&str>, body: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "comment_id": comment,
        "parent_id": parent,
        "body": format!("{body}\n"),
    }))
    .unwrap()
}

fn comment_args<'a>(clone: &'a CliClone, item: &'a str, request: &'a str) -> Vec<&'a str> {
    vec![
        "comment",
        "add",
        "--repo",
        clone.root(),
        "--id",
        item,
        "--request-id",
        request,
        "--input",
        "-",
    ]
}

/// One `comment add` of `comment` on `item`, in JSON mode, its data checked
/// against the published schema.
#[allow(clippy::too_many_arguments)]
fn add_on(
    clone: &CliClone,
    item: &str,
    request: &str,
    comment: &str,
    parent: Option<&str>,
    body: &str,
    code: &str,
    status: i32,
) -> Value {
    let value = clone.json(
        &comment_args(clone, item, request),
        &comment_input(comment, parent, body),
        code,
        status,
    );
    assert_eq!(value["command"], "comment add");
    assert_eq!(value["request_id"], request);
    if !value["data"].is_null() {
        schema::check_published(COMMENT_SCHEMA, &value["data"]).unwrap();
    }
    value
}

/// `add_on` of `ITEM`.
fn add(
    clone: &CliClone,
    request: &str,
    comment: &str,
    parent: Option<&str>,
    body: &str,
    code: &str,
    status: i32,
) -> Value {
    add_on(clone, ITEM, request, comment, parent, body, code, status)
}

fn item_sync(clone: &CliClone, item: &str, request: &str, code: &str, status: i32) -> Value {
    clone.json(
        &[
            "item",
            "sync",
            "--repo",
            clone.root(),
            "--id",
            item,
            "--request-id",
            request,
        ],
        &[],
        code,
        status,
    )
}

fn repo_sync(clone: &CliClone, request: &str, code: &str, status: i32) -> Value {
    clone.json(
        &[
            "repo",
            "sync",
            "--repo",
            clone.root(),
            "--request-id",
            request,
        ],
        &[],
        code,
        status,
    )
}

fn resume_args<'a>(clone: &'a CliClone, operation: &'a str, request: &'a str) -> Vec<&'a str> {
    vec![
        "operation",
        "resume",
        "--repo",
        clone.root(),
        "--operation-id",
        operation,
        "--request-id",
        request,
    ]
}

fn resume(clone: &CliClone, operation: &str, request: &str, code: &str, status: i32) -> Value {
    clone.json(&resume_args(clone, operation, request), &[], code, status)
}

fn request_show(clone: &CliClone, request: &str) -> Value {
    checked(
        &clone.env,
        &["request", "show", "--request-id", request],
        &[],
        "ok",
        0,
        Some("request.schema.json"),
    )["data"]
        .clone()
}

/// One human-mode run, with nothing on stdin.
fn human(clone: &CliClone, args: &[&str], input: &[u8]) -> CliRun {
    run(&clone.env, args, input)
}

fn text(bytes: &[u8]) -> &str {
    std::str::from_utf8(bytes).unwrap()
}

fn next_lines(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|line| line.starts_with("next: "))
        .collect()
}

/// `progress:` lines, one per stage, as human mode writes them on stderr.
fn progress(stages: &[&str]) -> String {
    stages
        .iter()
        .map(|stage| format!("progress: {stage}\n"))
        .collect()
}

/// How many times the server has authenticated a client.
fn connections(pair: &CliPair) -> usize {
    pair.server.accepted_keys().len()
}

/// A root as `next:` lines quote it.
fn quoted(clone: &CliClone) -> String {
    format!("'{}'", clone.root.canonicalize().unwrap().to_str().unwrap())
}

fn comment_path(comment: &str) -> String {
    format!(".manyhands/comments/{ITEM}/{comment}.md")
}

/// Whether the commit `oid` of `repo` holds `path`.
fn holds(repo: &git2::Repository, oid: git2::Oid, path: &str) -> Result<bool, FixtureError> {
    Ok(fixed(fixed(repo.find_commit(oid))?.tree())?
        .get_path(std::path::Path::new(path))
        .is_ok())
}

/// The commits in the history of `tip` that add `path`: those that hold it
/// while none of their parents does.
fn checkpoints(
    repo: &git2::Repository,
    tip: git2::Oid,
    path: &str,
) -> Result<Vec<git2::Oid>, FixtureError> {
    let mut walk = fixed(repo.revwalk())?;
    fixed(walk.push(tip))?;
    let mut found = Vec::new();
    for oid in walk {
        let oid = fixed(oid)?;
        let commit = fixed(repo.find_commit(oid))?;
        if holds(repo, oid, path)? && !commit.parent_ids().any(|p| holds(repo, p, path).unwrap()) {
            found.push(oid);
        }
    }
    Ok(found)
}

fn oid(value: &Value) -> git2::Oid {
    git2::Oid::from_str(value.as_str().unwrap()).unwrap()
}

/// A CLI golden of one clone's result. The clone's root and every ID or
/// commit a run makes are replaced, whichever of them the envelope holds.
fn c4_golden(name: &str, value: &Value, clone: &CliClone) {
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
    for (pointer, label) in [
        ("/request_id", "<request-id>"),
        ("/operation_id", "<operation-id>"),
        ("/data/synchronization_id", "<synchronization-id>"),
        ("/effects/commit_oid", "<commit-oid>"),
    ] {
        if let Some(found) = value.pointer(pointer).and_then(Value::as_str)
            && !replacements.iter().any(|(known, _)| known == found)
        {
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

/// What a request that wrote and checkpointed its comment reports, with
/// the publication `publication`.
fn assert_saved(value: &Value, publication: &str) {
    for (key, expected) in [
        ("write", "written"),
        ("checkpoint", "committed"),
        ("discovery", "current"),
        ("publication", publication),
        ("cleanup", "not_requested"),
    ] {
        assert_eq!(value["effects"][key], expected, "{key}");
    }
    assert_eq!(value["data"]["publication"], publication);
}

/// Comment publish success, and a replay of it; `request_mismatch` for its
/// request ID sent again with another item; human mode's progress.
fn c4_collaboration_comment_publishes() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let a = &p.a;
    private_body();
    let pushes = remote_world::accepted(&p.server);
    // A's first write makes its editing context, and publishes it.
    let published = add(a, R1, C1, None, PRIVATE_BODY, "ok", 0);
    assert_eq!(published["outcome"], "success");
    assert_saved(&published, "published");
    assert_eq!(published["effects"]["integration"], "not_requested");
    let tip = a.tip(CONTEXT)?.ok_or(FixtureError)?;
    assert_eq!(oid(&published["effects"]["commit_oid"]), tip);
    assert!(published["recovery"].as_array().unwrap().is_empty());
    let data = &published["data"];
    assert_eq!(data["type"], "comment");
    assert_eq!(data["item_id"], ITEM);
    assert_eq!(data["comment_id"], C1);
    assert!(data["parent_id"].is_null());
    assert_eq!(data["created_by"], AUTHOR_A);
    assert!(data["pending_reason"].is_null());
    assert!(data["conflict"].is_null());
    // The server holds the comment, as the branch does.
    assert_eq!(p.served(CONTEXT)?, Some(tip));
    assert!(holds(&p.bare()?, tip, &comment_path(C1))?);
    assert_eq!(remote_world::accepted(&p.server), pushes + 1);

    // The finished request again: its stored result, with no connection and
    // no second effect.
    let opened = connections(&p);
    let replay = add(a, R1, C1, None, PRIVATE_BODY, "ok", 0);
    assert_eq!(replay, published);
    assert_eq!(connections(&p), opened);
    assert_eq!(remote_world::accepted(&p.server), pushes + 1);
    assert_eq!(a.tip(CONTEXT)?, Some(tip));
    // The same request ID with another item is another request.
    let mismatch = add_on(
        a,
        OTHER_ITEM,
        R1,
        C1,
        None,
        PRIVATE_BODY,
        "request_mismatch",
        2,
    );
    assert!(mismatch["data"].is_null());
    assert_eq!(
        a.tip(&format!("refs/heads/manyhands/ticket/{OTHER_ITEM}"))?,
        None
    );
    assert_eq!(connections(&p), opened);

    // Human mode: C2's lines, the progress on stderr, then the note.
    let out = human(
        a,
        &comment_args(a, ITEM, R2),
        &comment_input(C2, None, PRIVATE_BODY),
    );
    assert_eq!(out.status, 0);
    let stdout = text(&out.stdout);
    assert!(
        stdout.starts_with(&format!("The request completed.\nrequest: {R2}\n")),
        "{stdout}"
    );
    assert!(stdout.contains("Publication: published\n"), "{stdout}");
    assert_eq!(
        text(&out.stderr),
        format!(
            "{}{PUBLICATION_NOTE}",
            progress(&[
                "accepted",
                "writing",
                "fetching",
                "observing",
                "pushing",
                "verifying",
                "indexing"
            ])
        )
    );
    let tip = a.tip(CONTEXT)?.ok_or(FixtureError)?;
    assert_eq!(p.served(CONTEXT)?, Some(tip));
    assert!(holds(&p.bare()?, tip, &comment_path(C2))?);
    Ok(())
}

/// Comment publication refused by the server, after C2's retry re-ran it
/// three more times (fact 66), then the same request with the server
/// accepting: one comment file, one checkpoint, its bytes unchanged.
fn c4_collaboration_comment_refused_then_saved_once() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let a = &p.a;
    private_body();
    add(a, R1, C1, None, PRIVATE_BODY, "ok", 0);
    let served = p.served(CONTEXT)?.ok_or(FixtureError)?;
    // The server refuses updates of the branch it has checked out.
    fixed(p.bare()?.set_head(CONTEXT))?;
    p.server.reject_primary_updates(true)?;
    let updates = p.server.receive_updates().len();
    let refused = add(a, R2, C2, None, PRIVATE_BODY, "push_rejected", 4);
    assert_eq!(refused["outcome"], "partial");
    assert_saved(&refused, "pending");
    let checkpoint = a.tip(CONTEXT)?.ok_or(FixtureError)?;
    assert_eq!(oid(&refused["effects"]["commit_oid"]), checkpoint);
    assert_eq!(refused["data"]["pending_reason"], "synchronization");
    assert!(refused["data"]["conflict"].is_null());
    // The same request is offered first: its record stays accepted.
    assert_eq!(
        refused["recovery"][0],
        serde_json::json!({"action": "request.retry", "operation_id": null, "arguments": {"request_id": R2}})
    );
    assert_eq!(request_show(a, R2)["state"], "accepted");
    // Four refused pushes of the checkpoint, and the branch as it was.
    let attempts = &p.server.receive_updates()[updates..];
    assert_eq!(attempts.len(), 4, "{attempts:?}");
    for attempt in attempts {
        assert_eq!(
            *attempt,
            ssh_remote::ReceiveUpdate {
                reference: CONTEXT.into(),
                old_oid: served,
                new_oid: checkpoint,
                accepted: false,
            }
        );
    }
    assert_eq!(p.served(CONTEXT)?, Some(served));
    let file = a.context().join(comment_path(C2));
    let written = fixed(std::fs::read(&file))?;

    p.server.reject_primary_updates(false)?;
    fixed(p.bare()?.set_head(PRIMARY))?;
    let pushes = remote_world::accepted(&p.server);
    let saved = add(a, R2, C2, None, PRIVATE_BODY, "ok", 0);
    assert_eq!(saved["outcome"], "success");
    assert_saved(&saved, "published");
    assert_eq!(saved["operation_id"], refused["operation_id"]);
    assert_eq!(
        saved["data"]["synchronization_id"],
        refused["data"]["synchronization_id"]
    );
    assert_eq!(oid(&saved["effects"]["commit_oid"]), checkpoint);
    // One comment file, one checkpoint, its creation time unchanged.
    assert_eq!(fixed(std::fs::read(&file))?, written);
    let mut files: Vec<_> = fixed(std::fs::read_dir(
        a.context().join(format!(".manyhands/comments/{ITEM}")),
    ))?
    .map(|entry| entry.unwrap().file_name().into_string().unwrap())
    .collect();
    files.sort();
    assert_eq!(files, [format!("{C1}.md"), format!("{C2}.md")]);
    assert_eq!(
        checkpoints(&a.repo()?, checkpoint, &comment_path(C2))?,
        [checkpoint]
    );
    assert_eq!(a.tip(CONTEXT)?, Some(checkpoint));
    assert_eq!(p.served(CONTEXT)?, Some(checkpoint));
    assert_eq!(remote_world::accepted(&p.server), pushes + 1);
    assert_eq!(request_show(a, R2)["state"], "finished");
    Ok(())
}

/// One comment of a clone's list: its ID, parent, depth and author.
type Listed = (String, Option<String>, u64, String);

fn listed(clone: &CliClone) -> Vec<Listed> {
    let list = checked(
        &clone.env,
        &["comment", "list", "--repo", clone.root(), "--id", ITEM],
        &[],
        "ok",
        0,
        Some("comment_list.schema.json"),
    );
    list["data"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|comment| {
            (
                comment["id"].as_str().unwrap().to_owned(),
                comment["parent_id"].as_str().map(str::to_owned),
                comment["depth"].as_u64().unwrap(),
                comment["author"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

/// Replies from both clones, each published by its `comment add` and
/// brought to the other by `item sync`: both clones list the same threads
/// in the same order.
fn c4_collaboration_replies_from_both_clones() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let (a, b) = (&p.a, &p.b);
    add(a, R1, C1, None, "first root", "ok", 0);
    // B's first write makes its own context, and its publication merges A's.
    let merged = add(b, R2, C2, None, "second root", "ok", 0);
    assert_saved(&merged, "published");
    assert_eq!(merged["effects"]["integration"], "complete");
    let reply = add(b, R3, C3, Some(C1), "reply to the first", "ok", 0);
    assert_eq!(reply["data"]["parent_id"], C1);
    assert_eq!(reply["data"]["created_by"], AUTHOR_B);
    // A is behind: its branch fast-forwards, and nothing is pushed.
    let pushes = remote_world::accepted(&p.server);
    item_sync(a, ITEM, R4, "already_applied", 0);
    assert_eq!(a.tip(CONTEXT)?, p.served(CONTEXT)?);
    assert_eq!(remote_world::accepted(&p.server), pushes);
    add(a, R5, C4, Some(C3), "reply to the reply", "ok", 0);
    item_sync(b, ITEM, R6, "already_applied", 0);
    let tip = p.served(CONTEXT)?;
    assert_eq!(a.tip(CONTEXT)?, tip);
    assert_eq!(b.tip(CONTEXT)?, tip);
    let expected: Vec<Listed> = vec![
        (C1.into(), None, 0, AUTHOR_A.into()),
        (C3.into(), Some(C1.into()), 1, AUTHOR_B.into()),
        (C4.into(), Some(C3.into()), 2, AUTHOR_A.into()),
        (C2.into(), None, 0, AUTHOR_B.into()),
    ];
    assert_eq!(listed(a), expected);
    assert_eq!(listed(b), expected);
    Ok(())
}

/// B's first write: a save of the ticket's body, which makes B's editing
/// context and publishes nothing.
fn save_the_ticket(b: &CliClone, request: &str) -> Value {
    let shown = b.json(
        &["ticket", "show", "--repo", b.root(), "--id", ITEM],
        &[],
        "ok",
        0,
    );
    let input = serde_json::json!({
        "observation": shown["data"]["observation"],
        "body": "Saved in clone B.\n",
    });
    let saved = b.json(
        &[
            "ticket",
            "save",
            "--repo",
            b.root(),
            "--id",
            ITEM,
            "--request-id",
            request,
            "--input",
            "-",
        ],
        &serde_json::to_vec(&input).unwrap(),
        "ok",
        0,
    );
    assert_eq!(saved["effects"]["checkpoint"], "committed");
    assert_eq!(saved["effects"]["publication"], "not_requested");
    saved
}

/// `item sync`'s clean divergence, its golden, a replay and
/// `request_mismatch`; then the fast-forward, in human mode, and already
/// current.
fn c4_collaboration_item_sync() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let (a, b) = (&p.a, &p.b);
    add(a, R1, C1, None, "first root", "ok", 0);
    let incoming = p.served(CONTEXT)?.ok_or(FixtureError)?;
    save_the_ticket(b, R2);
    let local = b.tip(CONTEXT)?.ok_or(FixtureError)?;
    assert_ne!(local, incoming);

    // Clean divergence: one merge commit, published.
    let pushes = remote_world::accepted(&p.server);
    let merged = item_sync(b, ITEM, R3, "ok", 0);
    assert_eq!(merged["outcome"], "success");
    let merge = b.tip(CONTEXT)?.ok_or(FixtureError)?;
    for (key, expected) in [
        ("write", "not_requested"),
        ("checkpoint", "not_requested"),
        ("discovery", "current"),
        ("publication", "published"),
        ("integration", "complete"),
        ("cleanup", "not_requested"),
    ] {
        assert_eq!(merged["effects"][key], expected, "{key}");
    }
    assert_eq!(oid(&merged["effects"]["commit_oid"]), merge);
    assert!(merged["data"].is_null());
    assert!(merged["recovery"].as_array().unwrap().is_empty());
    let repo = b.repo()?;
    let commit = fixed(repo.find_commit(merge))?;
    assert_eq!(commit.parent_ids().collect::<Vec<_>>(), [local, incoming]);
    assert_eq!(
        commit.summary(),
        Some(format!("Merge remote context {ITEM}").as_str())
    );
    // No rebase: the server keeps its earlier tip as an ancestor.
    assert_eq!(p.served(CONTEXT)?, Some(merge));
    assert!(fixed(p.bare()?.graph_descendant_of(merge, incoming))?);
    assert_eq!(remote_world::accepted(&p.server), pushes + 1);
    c4_golden("item_sync_ok", &merged, b);

    // The finished request again: its stored result, and nothing runs.
    let opened = connections(&p);
    assert_eq!(item_sync(b, ITEM, R3, "ok", 0), merged);
    // The same request ID for another item is another request.
    let mismatch = item_sync(b, OTHER_ITEM, R3, "request_mismatch", 2);
    assert!(mismatch["data"].is_null());
    // The refused request made no context of the other item.
    assert_eq!(
        b.tip(&format!("refs/heads/manyhands/ticket/{OTHER_ITEM}"))?,
        None
    );
    assert_eq!(connections(&p), opened);
    assert_eq!(remote_world::accepted(&p.server), pushes + 1);
    assert_eq!(b.tip(CONTEXT)?, Some(merge));

    // A is behind: in human mode, its branch fast-forwards, and nothing is
    // pushed (fact 63).
    let out = human(
        a,
        &[
            "item",
            "sync",
            "--repo",
            a.root(),
            "--id",
            ITEM,
            "--request-id",
            R4,
        ],
        &[],
    );
    assert_eq!(out.status, 0);
    let stdout = text(&out.stdout);
    assert!(
        stdout.starts_with(&format!(
            "Already applied; nothing changed.\nrequest: {R4}\n"
        )),
        "{stdout}"
    );
    assert!(
        stdout.contains("effects: discovery=current publication=current\n"),
        "{stdout}"
    );
    assert_eq!(
        text(&out.stderr),
        progress(&[
            "accepted",
            "connecting",
            "fetching",
            "observing",
            "updating_local",
            "observing",
            "indexing"
        ])
    );
    assert_eq!(a.tip(CONTEXT)?, Some(merge));
    assert_eq!(remote_world::accepted(&p.server), pushes + 1);

    // Already current.
    let current = item_sync(a, ITEM, R5, "already_applied", 0);
    assert_eq!(current["outcome"], "noop");
    assert_eq!(current["effects"]["publication"], "current");
    assert_eq!(current["effects"]["integration"], "not_requested");
    assert!(current["effects"]["commit_oid"].is_null());
    assert_eq!(a.tip(CONTEXT)?, Some(merge));
    assert_eq!(p.served(CONTEXT)?, Some(merge));
    assert_eq!(remote_world::accepted(&p.server), pushes + 1);
    Ok(())
}

/// `repo sync`: local ahead in human mode, then the other clone's clean
/// divergence, its golden, a replay and `request_mismatch`; then the
/// fast-forward and already current. Each clone's commit on primary is
/// history Git builds, as a person's own commit would be.
fn c4_collaboration_repo_sync() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let (a, b) = (&p.a, &p.b);
    let ahead = remote_world::advance(&a.repo()?, PRIMARY, "clone-a-file")?;
    let local = remote_world::advance(&b.repo()?, PRIMARY, "clone-b-file")?;
    let pushes = remote_world::accepted(&p.server);

    // A is ahead: in human mode, its commit is published.
    let out = human(
        a,
        &["repo", "sync", "--repo", a.root(), "--request-id", R1],
        &[],
    );
    assert_eq!(out.status, 0);
    let stdout = text(&out.stdout);
    assert!(
        stdout.starts_with(&format!("The request completed.\nrequest: {R1}\n")),
        "{stdout}"
    );
    assert!(
        stdout.contains(&format!(
            "effects: discovery=current publication=published commit={ahead}\n"
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
    assert_eq!(p.served(PRIMARY)?, Some(ahead));
    assert_eq!(remote_world::accepted(&p.server), pushes + 1);

    // B diverged: one merge commit, published.
    let merged = repo_sync(b, R2, "ok", 0);
    assert_eq!(merged["outcome"], "success");
    let merge = b.tip(PRIMARY)?.ok_or(FixtureError)?;
    assert_eq!(merged["effects"]["publication"], "published");
    assert_eq!(merged["effects"]["integration"], "complete");
    assert_eq!(oid(&merged["effects"]["commit_oid"]), merge);
    assert!(merged["data"].is_null());
    let repo = b.repo()?;
    let commit = fixed(repo.find_commit(merge))?;
    assert_eq!(commit.parent_ids().collect::<Vec<_>>(), [local, ahead]);
    assert_eq!(commit.summary(), Some("Merge remote primary"));
    assert_eq!(p.served(PRIMARY)?, Some(merge));
    assert!(fixed(p.bare()?.graph_descendant_of(merge, ahead))?);
    assert_eq!(remote_world::accepted(&p.server), pushes + 2);
    c4_golden("repo_sync_ok", &merged, b);

    // The finished request again: its stored result, and nothing runs.
    let opened = connections(&p);
    assert_eq!(repo_sync(b, R2, "ok", 0), merged);
    assert_eq!(b.tip(PRIMARY)?, Some(merge));
    // The same request ID for the item's branch is another request.
    let mismatch = item_sync(b, ITEM, R2, "request_mismatch", 2);
    assert!(mismatch["data"].is_null());
    // B has no editing context, and the refused request made none.
    assert_eq!(b.tip(CONTEXT)?, None);
    assert_eq!(connections(&p), opened);
    assert_eq!(remote_world::accepted(&p.server), pushes + 2);

    // A is behind: its primary fast-forwards, and nothing is pushed.
    let behind = repo_sync(a, R3, "already_applied", 0);
    assert_eq!(behind["effects"]["publication"], "current");
    assert_eq!(a.tip(PRIMARY)?, Some(merge));
    // Already current.
    let current = repo_sync(a, R4, "already_applied", 0);
    assert_eq!(current["outcome"], "noop");
    assert_eq!(current["effects"]["publication"], "current");
    assert_eq!(a.tip(PRIMARY)?, Some(merge));
    assert_eq!(p.served(PRIMARY)?, Some(merge));
    assert_eq!(remote_world::accepted(&p.server), pushes + 2);
    Ok(())
}

/// A comment whose publication stopped before it reached the server, for
/// the server went away at its advertisement, published by `operation
/// resume` of the comment's operation (resume row 1): in human mode with
/// its progress, then replayed in JSON, golden and all, and refused for
/// another operation under the same request ID.
fn c4_collaboration_resume_publishes_a_stopped_comment() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let a = &p.a;
    private_body();
    add(a, R1, C1, None, PRIVATE_BODY, "ok", 0);
    let served = p.served(CONTEXT)?;
    p.server.disconnect_at(FixtureBoundary::Advertisement);
    let stopped = add(a, R2, C2, None, PRIVATE_BODY, "transport_unavailable", 4);
    p.server.clear_fault();
    assert_eq!(stopped["outcome"], "partial");
    assert_saved(&stopped, "pending");
    assert_eq!(stopped["recovery"][0]["action"], "request.retry");
    assert_eq!(p.served(CONTEXT)?, served);
    let checkpoint = a.tip(CONTEXT)?.ok_or(FixtureError)?;
    let comment = stopped["operation_id"].as_str().unwrap().to_owned();
    let pushes = remote_world::accepted(&p.server);

    let out = human(a, &resume_args(a, &comment, R3), &[]);
    assert_eq!(out.status, 0);
    let stdout = text(&out.stdout);
    assert!(
        stdout.starts_with(&format!(
            "The request completed.\nrequest: {R3}\n\
             effects: discovery=current publication=published commit={checkpoint}\n"
        )),
        "{stdout}"
    );
    assert!(stdout.contains("Publication: published\n"), "{stdout}");
    assert_eq!(
        text(&out.stderr),
        format!(
            "{}{PUBLICATION_NOTE}",
            progress(&[
                "accepted",
                "observing",
                "fetching",
                "observing",
                "pushing",
                "verifying",
                "indexing"
            ])
        )
    );
    // It published the comment once, and wrote nothing.
    assert_eq!(p.served(CONTEXT)?, Some(checkpoint));
    assert!(holds(&p.bare()?, checkpoint, &comment_path(C2))?);
    assert_eq!(a.tip(CONTEXT)?, Some(checkpoint));
    assert_eq!(remote_world::accepted(&p.server), pushes + 1);

    // The finished request again, in JSON: its stored result, data and all,
    // with no connection.
    let opened = connections(&p);
    let resumed = resume(a, &comment, R3, "ok", 0);
    assert_eq!(connections(&p), opened);
    assert_eq!(a.tip(CONTEXT)?, Some(checkpoint));
    assert!(resumed["operation_id"].is_null());
    assert!(resumed["recovery"].as_array().unwrap().is_empty());
    for (key, expected) in [
        ("write", "not_requested"),
        ("checkpoint", "not_requested"),
        ("discovery", "current"),
        ("publication", "published"),
        ("integration", "not_requested"),
    ] {
        assert_eq!(resumed["effects"][key], expected, "{key}");
    }
    assert_eq!(oid(&resumed["effects"]["commit_oid"]), checkpoint);
    schema::check_published(COMMENT_SCHEMA, &resumed["data"]).unwrap();
    assert_eq!(resumed["data"]["comment_id"], C2);
    assert_eq!(resumed["data"]["publication"], "published");
    assert!(resumed["data"]["pending_reason"].is_null());
    assert_eq!(
        resumed["data"]["synchronization_id"],
        stopped["data"]["synchronization_id"]
    );
    c4_golden("operation_resume_comment_ok", &resumed, a);
    // The same request ID for another operation is another request.
    let other = repository::OperationId::new().to_string();
    resume(a, &other, R3, "request_mismatch", 2);
    assert_eq!(a.tip(CONTEXT)?, Some(checkpoint));
    assert_eq!(connections(&p), opened);
    // The comment's own request finds it published, and pushes nothing.
    let again = add(a, R2, C2, None, PRIVATE_BODY, "ok", 0);
    assert_eq!(again["data"]["publication"], "published");
    assert_eq!(remote_world::accepted(&p.server), pushes + 1);
    Ok(())
}

/// A, whose context is published, then deleted on the server, and a
/// comment whose publication meets the deletion: the request that holds
/// the reservation, and its comment's operation.
fn deleted(p: &CliPair) -> Result<(), FixtureError> {
    add(&p.a, R1, C1, None, PRIVATE_BODY, "ok", 0);
    fixed(fixed(p.bare()?.find_reference(CONTEXT))?.delete())?;
    Ok(())
}

/// The way out the human output names, followed: `request cancel` of the
/// comment's request, the same request again, which ends it cancelled with
/// its checkpoint kept, then `item republish`, prepared and confirmed. The
/// preview takes no reservation, so only the confirmed republish, which
/// reserves and pushes, proves the reservation free: it publishes the
/// checkpoint with one push, and is not refused as `busy`.
fn free_the_reservation(p: &CliPair, checkpoint: git2::Oid) -> Result<(), FixtureError> {
    let a = &p.a;
    let pushes = remote_world::accepted(&p.server);
    let asked = checked(
        &a.env,
        &["request", "cancel", "--request-id", R2],
        &[],
        "ok",
        0,
        None,
    );
    assert_eq!(asked["data"]["requested"], true);
    let ended = add(a, R2, C2, None, PRIVATE_BODY, "cancelled", 130);
    assert_eq!(ended["outcome"], "cancelled");
    assert_saved(&ended, "pending");
    assert_eq!(oid(&ended["effects"]["commit_oid"]), checkpoint);
    assert!(ended["recovery"].as_array().unwrap().is_empty());
    assert_eq!(request_show(a, R2)["state"], "finished");
    // The checkpoint stays committed, and nothing is recreated.
    assert_eq!(a.tip(CONTEXT)?, Some(checkpoint));
    assert!(holds(&a.repo()?, checkpoint, &comment_path(C2))?);
    assert_eq!(p.served(CONTEXT)?, None);
    assert_eq!(remote_world::accepted(&p.server), pushes);
    let preview = checked(
        &a.env,
        &[
            "item",
            "republish",
            "--repo",
            a.root(),
            "--id",
            ITEM,
            "--prepare",
        ],
        &[],
        "ok",
        0,
        Some("item_republish_preview.schema.json"),
    );
    assert_eq!(
        preview["data"]["effects"],
        serde_json::json!([{"effect": "branch_republished"}])
    );
    assert_eq!(oid(&preview["data"]["details"]["local_oid"]), checkpoint);
    let republished = checked(
        &a.env,
        &[
            "item",
            "republish",
            "--repo",
            a.root(),
            "--id",
            ITEM,
            "--request-id",
            R6,
            "--confirm",
            preview["data"]["confirmation_id"].as_str().unwrap(),
        ],
        &[],
        "ok",
        0,
        None,
    );
    assert_eq!(republished["outcome"], "success");
    assert_eq!(republished["request_id"], R6);
    assert_eq!(p.served(CONTEXT)?, Some(checkpoint));
    assert_eq!(remote_world::accepted(&p.server), pushes + 1);
    assert_eq!(
        p.server.receive_updates().last(),
        Some(&ssh_remote::ReceiveUpdate {
            reference: CONTEXT.into(),
            old_oid: git2::Oid::zero(),
            new_oid: checkpoint,
            accepted: true,
        })
    );
    assert_eq!(a.tip(CONTEXT)?, Some(checkpoint));
    Ok(())
}

/// Carry 1 (Task 2b's ruling): a comment whose publication meets a deleted
/// branch prints three `next:` lines in order: cancel its request, send it
/// again, prepare the republish. Followed, they free the reservation.
fn c4_collaboration_comment_meets_a_deleted_branch() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let a = &p.a;
    private_body();
    deleted(&p)?;
    let pushes = remote_world::accepted(&p.server);
    let out = human(
        a,
        &comment_args(a, ITEM, R2),
        &comment_input(C2, None, PRIVATE_BODY),
    );
    assert_eq!(out.status, 4);
    assert!(out.stdout.is_empty());
    // The run made the checkpoint: its OID is A's context tip.
    let checkpoint = a.tip(CONTEXT)?.ok_or(FixtureError)?;
    let stderr = text(&out.stderr);
    assert!(
        stderr.starts_with(&format!(
            "{}error: The published branch was deleted on the remote.\n\
             code: remote_branch_deleted\n\
             request: {R2}\n",
            progress(&["accepted", "writing", "fetching", "observing"])
        )),
        "{stderr}"
    );
    assert!(
        stderr.contains(&format!(
            "effects: write=written checkpoint=committed discovery=current publication=pending commit={checkpoint}\n"
        )),
        "{stderr}"
    );
    assert_eq!(
        next_lines(stderr),
        [
            format!("next: manyhands-cli request cancel --request-id {R2}"),
            format!("next: send request {R2} again with its original command and input"),
            format!(
                "next: manyhands-cli item republish --repo {} --id {ITEM} --prepare",
                quoted(a)
            ),
        ]
    );
    assert!(
        stderr.ends_with(&format!("{PUBLICATION_NOTE}{RESERVATION_NOTE}")),
        "{stderr}"
    );
    // The record stays accepted: the request holds the reservation.
    assert_eq!(request_show(a, R2)["state"], "accepted");
    assert_eq!(p.served(CONTEXT)?, None);
    assert_eq!(remote_world::accepted(&p.server), pushes);
    assert_eq!(a.tip(CONTEXT)?, Some(checkpoint));
    free_the_reservation(&p, checkpoint)
}

/// Carry 2: `operation resume` of that comment's publication. A resume
/// stops at no safe point, so the library offers no `request.cancel` and
/// keeps no record; human output says which request holds the reservation
/// and prints the republish preview of the comment's item (controller
/// ruling, Task 12a). JSON is the library's envelope alone. The way out it
/// names then works.
fn c4_collaboration_resume_meets_a_deleted_branch() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let a = &p.a;
    private_body();
    deleted(&p)?;
    let stopped = add(a, R2, C2, None, PRIVATE_BODY, "remote_branch_deleted", 4);
    let comment = stopped["operation_id"].as_str().unwrap().to_owned();
    let checkpoint = a.tip(CONTEXT)?.ok_or(FixtureError)?;
    let pushes = remote_world::accepted(&p.server);

    let out = human(a, &resume_args(a, &comment, R3), &[]);
    assert_eq!(out.status, 3);
    assert!(out.stdout.is_empty());
    let stderr = text(&out.stderr);
    assert!(
        stderr.starts_with(&format!(
            "{}error: The published branch was deleted on the remote.\n\
             code: remote_branch_deleted\n\
             request: {R3}\n\
             effects: publication=pending\n",
            progress(&["accepted", "observing", "fetching", "observing"])
        )),
        "{stderr}"
    );
    assert_eq!(
        next_lines(stderr),
        [format!(
            "next: manyhands-cli item republish --repo {} --id {ITEM} --prepare",
            quoted(a)
        )]
    );
    assert!(
        stderr.ends_with(&format!("{PUBLICATION_NOTE}{RESUME_DELETED_NOTE}")),
        "{stderr}"
    );
    // A resume that stopped here keeps no record.
    checked(
        &a.env,
        &["request", "show", "--request-id", R3],
        &[],
        "request_not_found",
        2,
        None,
    );

    // JSON: the envelope alone, with no recovery action.
    let refused = resume(a, &comment, R4, "remote_branch_deleted", 3);
    assert_eq!(refused["outcome"], "blocked");
    assert!(refused["recovery"].as_array().unwrap().is_empty());
    assert!(refused["operation_id"].is_null());
    assert_eq!(refused["effects"]["publication"], "pending");
    assert_eq!(refused["effects"]["write"], "not_requested");
    schema::check_published(COMMENT_SCHEMA, &refused["data"]).unwrap();
    assert_eq!(refused["data"]["publication"], "pending");
    assert_eq!(refused["data"]["pending_reason"], "synchronization");
    // The comment's request still holds the reservation.
    assert_eq!(request_show(a, R2)["state"], "accepted");
    assert_eq!(a.tip(CONTEXT)?, Some(checkpoint));
    assert_eq!(p.served(CONTEXT)?, None);
    assert_eq!(remote_world::accepted(&p.server), pushes);
    free_the_reservation(&p, checkpoint)
}
