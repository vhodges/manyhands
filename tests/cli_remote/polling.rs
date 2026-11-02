//! `poll once` through the binary, over SSH. Each case is one fresh world
//! under the harness's watchdog. The library's frozen table
//! (`tests/mutation_remote/c5_poll.rs`) pins every code; these pin what a person
//! meets, and what Git holds afterwards: a poll never pushes, checkpoints, merges
//! or cleans up, and `Sides` compares both ends by their refs to show it.
//! Ctrl-C and a killed poll are Task 13a's.

use std::{collections::BTreeMap, path::PathBuf};

use super::*;
use cli_pair::{CONTEXT, CliClone, CliPair, ITEM, OTHER_ITEM, PRIMARY};
use collaboration::save;
use ssh_remote::{FixtureError, fixed};

pub const CASES: &[ssh_harness::Case] = &[
    (
        "c5_polling_fast_forward_and_materialize_once",
        c5_polling_fast_forward_and_materialize_once,
    ),
    (
        "c5_polling_a_pause_does_not_refuse",
        c5_polling_a_pause_does_not_refuse,
    ),
    (
        "c5_polling_dirty_context_preserved",
        c5_polling_dirty_context_preserved,
    ),
    (
        "c5_polling_divergent_context_preserved",
        c5_polling_divergent_context_preserved,
    ),
    (
        "c5_polling_malformed_ref_names_nothing",
        c5_polling_malformed_ref_names_nothing,
    ),
    (
        "c5_polling_deleted_context_preserved",
        c5_polling_deleted_context_preserved,
    ),
    (
        "c5_polling_remote_context_of_a_closed_ticket",
        c5_polling_remote_context_of_a_closed_ticket,
    ),
    (
        "c5_polling_item_sync_of_a_closed_ticket_s_old_copy",
        c5_polling_item_sync_of_a_closed_ticket_s_old_copy,
    ),
    (
        "c5_polling_item_sync_of_a_copy_the_remote_still_holds",
        c5_polling_item_sync_of_a_copy_the_remote_still_holds,
    ),
    (
        "c5_polling_host_stop_is_final_then_a_new_request",
        c5_polling_host_stop_is_final_then_a_new_request,
    ),
    #[cfg(unix)]
    (
        "c5_polling_unlock_stop_is_final_then_a_new_request",
        c5_polling_unlock_stop_is_final_then_a_new_request,
    ),
    ("c5_polling_no_remote", c5_polling_no_remote),
    ("c5_polling_usage_failure", c5_polling_usage_failure),
    (
        "c5_polling_replay_and_mismatch",
        c5_polling_replay_and_mismatch,
    ),
    (
        "c5_polling_a_second_process_holds_the_lease",
        c5_polling_a_second_process_holds_the_lease,
    ),
    (
        "c5_polling_a_second_poll_overlaps",
        c5_polling_a_second_poll_overlaps,
    ),
    (
        "c5_polling_promote_then_close_needs_no_poll",
        c5_polling_promote_then_close_needs_no_poll,
    ),
    (
        "c5_polling_a_missing_tracking_ref_names_the_poll",
        c5_polling_a_missing_tracking_ref_names_the_poll,
    ),
];

const R1: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR1";
const R2: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR2";
const R3: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR3";
const R4: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR4";
const R5: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR5";
const R6: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR6";
const R9: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR9";
const SCHEMA: &str = "poll_mutation.schema.json";
const MALFORMED: &str = "c5-polling-malformed-ref-sentinel";
const INCOMING: &str = "incoming.txt";

fn once<'a>(root: &'a str, request: &'a str) -> Vec<&'a str> {
    vec!["poll", "once", "--repo", root, "--request-id", request]
}

/// One JSON-mode run: one envelope on stdout, nothing on stderr, and the data
/// checked when it is a poll's.
fn json_once(env: &CliEnv, root: &str, request: &str, code: &str, status: i32) -> Value {
    let mut args = once(root, request);
    args.push("--json");
    let out = run(env, &args, &[]);
    assert!(
        out.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let value = cli::json(&out);
    assert_eq!(value["code"], code, "{value}");
    assert_eq!(out.status, status, "{value}");
    assert_eq!(value["command"], "poll once");
    schema::check_published("envelope.schema.json", &value).unwrap();
    if value["data"]["completion"].is_string() {
        schema::check_published(SCHEMA, &value["data"]).unwrap();
    }
    value
}

fn polled(clone: &CliClone, request: &str) -> Value {
    json_once(&clone.env, clone.root(), request, "ok", 0)
}

fn text_once(env: &CliEnv, root: &str, request: &str) -> CliRun {
    run(env, &once(root, request), &[])
}

fn stdout(run: &CliRun) -> &str {
    std::str::from_utf8(&run.stdout).unwrap()
}

fn stderr(run: &CliRun) -> &str {
    std::str::from_utf8(&run.stderr).unwrap()
}

fn target<'a>(value: &'a Value, reference: &str) -> &'a Value {
    value["data"]["targets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|target| target["target"] == reference)
        .unwrap_or_else(|| panic!("no target {reference}: {value}"))
}

fn disposition<'a>(value: &'a Value, reference: &str) -> &'a str {
    target(value, reference)["disposition"].as_str().unwrap()
}

/// `value` with what a run makes up replaced, whichever of it the envelope
/// holds: the targets' commits, the times and the IDs. A target's other fields,
/// its order and its disposition stay.
fn scrubbed_json(value: &Value) -> Value {
    fn scrub(value: &mut Value, key: &str, label: &str) {
        if value.get(key).is_some_and(|found| !found.is_null()) {
            value[key] = label.into();
        }
    }
    let mut value = value.clone();
    scrub(&mut value["data"], "operation_id", "<operation-id>");
    scrub(&mut value["data"], "completed_at", "<time>");
    scrub(&mut value["data"], "next_eligible_at", "<time>");
    if let Some(targets) = value["data"]["targets"].as_array_mut() {
        for target in targets {
            scrub(target, "before_oid", "<oid>");
            scrub(target, "after_oid", "<oid>");
        }
    }
    value
}

fn json_golden(name: &str, value: &Value, root: &Path, port: u16) {
    let value = scrubbed_json(value);
    golden(name, &value, root, port);
}

/// Human output with what a run makes up replaced: commits, the IDs no case
/// fixes, the times, the root and the server's port.
fn text_golden(name: &str, out: &CliRun, root: &Path, port: u16) {
    let text = format!(
        "{}--- stderr\n{}--- exit {}\n",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
        out.status
    );
    let text = text
        .replace(
            root.canonicalize().unwrap().to_str().unwrap(),
            "<repository>",
        )
        .replace(&format!("127.0.0.1:{port}"), "127.0.0.1:0")
        .replace(&format!("Port: {port}\n"), "Port: 0\n");
    let text: String = text
        .lines()
        .map(|line| match line.split_once(" fingerprint: ") {
            Some((label, value)) if value != "none" => {
                format!("{label} fingerprint: <fingerprint>\n")
            }
            _ => format!("{line}\n"),
        })
        .collect();
    let mut scrubbed = String::new();
    let mut token = String::new();
    let flush = |token: &mut String, scrubbed: &mut String| {
        scrubbed.push_str(&scrub_token(token));
        token.clear();
    };
    for character in text.chars() {
        if character.is_alphanumeric() || matches!(character, '-' | ':' | '.' | '+') {
            token.push(character);
        } else {
            flush(&mut token, &mut scrubbed);
            scrubbed.push(character);
        }
    }
    flush(&mut token, &mut scrubbed);
    cli::assert_cli_text_golden(name, &scrubbed, &[]);
}

fn scrub_token(token: &str) -> String {
    let ulid = token.len() == 26
        && token
            .chars()
            .all(|c| c.is_ascii_digit() || c.is_ascii_uppercase());
    if token.len() == 40 && token.chars().all(|c| c.is_ascii_hexdigit()) {
        "<oid>".into()
    } else if ulid && ![R1, R2, R3, R4, R5, R6, R9, ITEM, OTHER_ITEM].contains(&token) {
        "<id>".into()
    } else if token.len() >= 20 && token.contains('T') && token.contains(':') {
        "<time>".into()
    } else {
        token.to_owned()
    }
}

/// A peer's commit on the remote's primary, with a file no clone holds.
fn advance_primary(p: &CliPair, name: &str) -> Result<git2::Oid, FixtureError> {
    let bare = p.bare()?;
    let parent = fixed(bare.find_commit(fixed(bare.refname_to_id(PRIMARY))?))?;
    let mut builder = fixed(bare.treebuilder(Some(&fixed(parent.tree())?)))?;
    fixed(builder.insert(name, fixed(bare.blob(b"incoming\n"))?, 0o100644))?;
    let tree = fixed(bare.find_tree(fixed(builder.write())?))?;
    let signature = fixed(git2::Signature::now("Peer", "peer@example.invalid"))?;
    fixed(bare.commit(
        Some(PRIMARY),
        &signature,
        &signature,
        "A peer's commit",
        &tree,
        &[&parent],
    ))
}

fn sync_item(clone: &CliClone, request: &str) -> Value {
    clone.json(
        &[
            "item",
            "sync",
            "--repo",
            clone.root(),
            "--id",
            ITEM,
            "--request-id",
            request,
        ],
        &[],
        "ok",
        0,
    )
}

/// `clone`'s save of `ITEM`, published on the remote.
fn published(clone: &CliClone, save_request: &str, sync_request: &str, body: &str) -> Value {
    save(clone, save_request, body);
    sync_item(clone, sync_request)
}

fn refs(repository: &git2::Repository) -> Result<BTreeMap<String, git2::Oid>, FixtureError> {
    let mut found = BTreeMap::new();
    for reference in fixed(repository.references())? {
        let reference = fixed(reference)?;
        if let (Some(name), Some(oid)) = (reference.name(), reference.target()) {
            found.insert(name.to_owned(), oid);
        }
    }
    Ok(found)
}

/// What a poll may not change: every ref on the server, and the clone's own
/// branches and worktree list. Tracking refs are a fetch's to move.
#[derive(Debug, PartialEq, Eq)]
struct Sides {
    served: BTreeMap<String, git2::Oid>,
    branches: BTreeMap<String, git2::Oid>,
    worktrees: Vec<String>,
    pushes: usize,
}

fn sides(p: &CliPair, clone: &CliClone) -> Result<Sides, FixtureError> {
    let repository = clone.repo()?;
    let branches = refs(&repository)?
        .into_iter()
        .filter(|(name, _)| name.starts_with("refs/heads/"))
        .collect();
    let mut worktrees: Vec<String> = fixed(repository.worktrees())?
        .iter()
        .flatten()
        .map(str::to_owned)
        .collect();
    worktrees.sort();
    Ok(Sides {
        served: refs(&p.bare()?)?,
        branches,
        worktrees,
        pushes: remote_world::accepted(&p.server),
    })
}

/// No push, checkpoint, merge or cleanup: `before` and `after` agree on every
/// ref the server holds, on the clone's branches and worktrees, and on the
/// pushes the server accepted. The clone's tracking refs aren't compared: a
/// fetch moves them.
fn nothing_but_a_fetch(before: &Sides, after: &Sides) {
    assert_eq!(before, after);
}

fn unchanged_file(clone: &CliClone) -> PathBuf {
    clone
        .context()
        .join(".manyhands/tickets")
        .join(ITEM)
        .join("ticket.md")
}

/// A clean primary fast-forwards and a new remote context materializes, once:
/// the next poll finds them current, with one worktree.
fn c5_polling_fast_forward_and_materialize_once() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let port = p.server.address().port();
    published(&p.b, R1, R2, "Saved in clone B.\n");
    let incoming = advance_primary(&p, INCOMING)?;
    let context = p.served(CONTEXT)?.ok_or(FixtureError)?;
    let pushes = remote_world::accepted(&p.server);
    let (a, a_primary) = (&p.a, p.a.tip(PRIMARY)?);
    assert_eq!(a.tip(CONTEXT)?, None);

    let value = polled(a, R1);
    assert_eq!(value["outcome"], "success");
    assert!(value["recovery"].as_array().unwrap().is_empty());
    assert_eq!(disposition(&value, PRIMARY), "fast_forwarded");
    assert_eq!(disposition(&value, CONTEXT), "materialized");
    assert_eq!(target(&value, CONTEXT)["item_id"], ITEM);
    assert_eq!(target(&value, CONTEXT)["kind"], "ticket");
    assert_eq!(
        target(&value, PRIMARY)["after_oid"],
        incoming.to_string().as_str()
    );
    assert_ne!(a_primary, Some(incoming));
    json_golden(
        "poll_once_fast_forward_and_materialize",
        &value,
        &a.root,
        port,
    );
    assert_eq!(a.tip(PRIMARY)?, Some(incoming));
    assert_eq!(a.tip(CONTEXT)?, Some(context));
    assert!(a.context().join(".manyhands").is_dir());
    assert!(a.root.join(INCOMING).is_file());
    assert_eq!(remote_world::accepted(&p.server), pushes, "no push");

    // The same state again, as a new request: nothing to do, and still one.
    let before = sides(&p, a)?;
    let again = polled(a, R2);
    assert_eq!(disposition(&again, PRIMARY), "current");
    assert_eq!(disposition(&again, CONTEXT), "current");
    nothing_but_a_fetch(&before, &sides(&p, a)?);
    assert_eq!(before.worktrees, [ITEM.to_owned()]);

    // Human mode, in the other clone: a line per target, each with its guidance.
    let b = &p.b;
    let out = text_once(&b.env, b.root(), R3);
    assert_eq!(out.status, 0, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.starts_with(&format!("The request completed.\nrequest: {R3}\n")),
        "{text}"
    );
    assert!(text.contains(&format!("target: {PRIMARY} ")), "{text}");
    assert!(text.contains("disposition=fast_forwarded"), "{text}");
    assert!(
        text.contains("  Clean local state was fast-forwarded.\n"),
        "{text}"
    );
    assert!(text.contains("  Local state is current.\n"), "{text}");
    assert_eq!(stderr(&out), "progress: accepted\nprogress: connecting\n");
    text_golden("poll_once_fast_forward_text", &out, &b.root, port);
    Ok(())
}

/// Cycle 08: an explicit poll runs while paused, and the pause stands.
fn c5_polling_a_pause_does_not_refuse() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let a = &p.a;
    let paused = a.json(
        &["poll", "pause", "--repo", a.root(), "--request-id", R1],
        &[],
        "ok",
        0,
    );
    assert_eq!(paused["data"]["paused"], true);
    let incoming = advance_primary(&p, INCOMING)?;
    let value = polled(a, R2);
    assert_eq!(disposition(&value, PRIMARY), "fast_forwarded");
    assert_eq!(a.tip(PRIMARY)?, Some(incoming));
    let status = a.json(&["poll", "status", "--repo", a.root()], &[], "ok", 0);
    assert_eq!(status["data"]["paused"], true);
    Ok(())
}

/// A dirty context stays as it was, with its unsaved bytes, and it is data.
fn c5_polling_dirty_context_preserved() -> Result<(), FixtureError> {
    const UNSAVED: &[u8] = b"Unsaved in clone A.\n";
    let p = CliPair::new()?;
    let port = p.server.address().port();
    let (a, b) = (&p.a, &p.b);
    published(a, R1, R2, "Saved in clone A.\n");
    polled(b, R1);
    published(b, R2, R3, "Saved in clone B.\n");
    let file = unchanged_file(a);
    fixed(std::fs::write(&file, UNSAVED))?;
    let (tip, primary) = (a.tip(CONTEXT)?, a.tip(PRIMARY)?);
    let before = sides(&p, a)?;

    let value = polled(a, R3);
    assert_eq!(disposition(&value, CONTEXT), "dirty");
    json_golden("poll_once_dirty", &value, &a.root, port);
    nothing_but_a_fetch(&before, &sides(&p, a)?);
    assert_eq!((a.tip(CONTEXT)?, a.tip(PRIMARY)?), (tip, primary));
    assert_eq!(fixed(std::fs::read(&file))?, UNSAVED);

    let out = text_once(&a.env, a.root(), R4);
    assert_eq!(out.status, 0, "exit 0, with the target in its data");
    assert!(
        stdout(&out).contains("  Save or otherwise handle local changes before polling again.\n")
    );
    text_golden("poll_once_dirty_text", &out, &a.root, port);
    assert_eq!(fixed(std::fs::read(&file))?, UNSAVED);
    Ok(())
}

/// Local work on a context the remote has moved past: neither moves.
fn c5_polling_divergent_context_preserved() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let port = p.server.address().port();
    let (a, b) = (&p.a, &p.b);
    published(a, R1, R2, "Saved in clone A.\n");
    polled(b, R1);
    published(b, R2, R3, "Saved in clone B.\n");
    save(a, R3, "Saved again in clone A.\n");
    let (local, served) = (a.tip(CONTEXT)?, p.served(CONTEXT)?);
    assert_ne!(local, served);
    let before = sides(&p, a)?;

    let value = polled(a, R4);
    assert_eq!(disposition(&value, CONTEXT), "diverged");
    json_golden("poll_once_diverged", &value, &a.root, port);
    nothing_but_a_fetch(&before, &sides(&p, a)?);
    assert_eq!((a.tip(CONTEXT)?, p.served(CONTEXT)?), (local, served));
    Ok(())
}

/// Review Focus 7: a malformed remote ref is a target with nullable fields, and
/// its name is in no output of either mode. Every run scans for it.
fn c5_polling_malformed_ref_names_nothing() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let port = p.server.address().port();
    OUTPUT_PROBES.with(|probes| probes.borrow_mut().push(MALFORMED.to_owned()));
    let bare = p.bare()?;
    let name = format!("refs/heads/manyhands/ticket/{MALFORMED}");
    let head = fixed(bare.refname_to_id(PRIMARY))?;
    fixed(bare.reference(&name, head, false, "malformed"))?;
    let a = &p.a;
    let before = sides(&p, a)?;

    let value = polled(a, R1);
    let malformed: Vec<_> = value["data"]["targets"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|target| target["disposition"] == "malformed")
        .collect();
    assert_eq!(malformed.len(), 1);
    for key in ["target", "kind", "item_id", "before_oid", "after_oid"] {
        assert!(malformed[0][key].is_null(), "{key}: {value}");
    }
    json_golden("poll_once_malformed", &value, &a.root, port);
    nothing_but_a_fetch(&before, &sides(&p, a)?);

    let out = text_once(&a.env, a.root(), R2);
    assert_eq!(out.status, 0);
    assert!(
        stdout(&out).contains("disposition=malformed"),
        "{}",
        stdout(&out)
    );
    assert!(stdout(&out).contains("  Repair the remote context's branch name.\n"));
    text_golden("poll_once_malformed_text", &out, &a.root, port);
    assert_eq!(p.served(&name)?, Some(head), "the remote's ref stays");
    Ok(())
}

/// A context deleted on the remote stays here: its branch, its worktree and
/// its commits.
fn c5_polling_deleted_context_preserved() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let port = p.server.address().port();
    let a = &p.a;
    published(a, R1, R2, "Saved in clone A.\n");
    polled(a, R3);
    fixed(fixed(p.bare()?.find_reference(CONTEXT))?.delete())?;
    let tip = a.tip(CONTEXT)?;
    let before = sides(&p, a)?;

    let value = polled(a, R4);
    assert_eq!(disposition(&value, CONTEXT), "remotely_deleted");
    json_golden("poll_once_remotely_deleted", &value, &a.root, port);
    nothing_but_a_fetch(&before, &sides(&p, a)?);
    assert_eq!(a.tip(CONTEXT)?, tip);
    assert!(a.context().is_dir());
    assert_eq!(p.served(CONTEXT)?, None, "no republish");
    Ok(())
}

/// A commit on the remote's primary that writes `path` as `bytes`.
fn commit_on_primary(p: &CliPair, path: &str, bytes: &[u8]) -> Result<git2::Oid, FixtureError> {
    let bare = p.bare()?;
    let parent = fixed(bare.find_commit(fixed(bare.refname_to_id(PRIMARY))?))?;
    let mut index = fixed(git2::Index::new())?;
    fixed(index.read_tree(&fixed(parent.tree())?))?;
    fixed(index.add(&git2::IndexEntry {
        ctime: git2::IndexTime::new(0, 0),
        mtime: git2::IndexTime::new(0, 0),
        dev: 0,
        ino: 0,
        mode: 0o100644,
        uid: 0,
        gid: 0,
        file_size: 0,
        id: fixed(bare.blob(bytes))?,
        flags: 0,
        flags_extended: 0,
        path: path.as_bytes().to_vec(),
    }))?;
    let tree = fixed(bare.find_tree(fixed(index.write_tree_to(&bare))?))?;
    let signature = fixed(git2::Signature::now("Peer", "peer@example.invalid"))?;
    fixed(bare.commit(
        Some(PRIMARY),
        &signature,
        &signature,
        "A peer's commit",
        &tree,
        &[&parent],
    ))
}

/// `OTHER_ITEM` depends on `ITEM`, on primary, in both clones.
fn a_dependent(p: &CliPair) -> Result<(), FixtureError> {
    let source = format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {OTHER_ITEM}\ntitle: Other\n\
         type: task\nstatus: open\ndeps: [{ITEM}]\n---\nBody of Other.\n"
    );
    commit_on_primary(
        p,
        &format!(".manyhands/tickets/{OTHER_ITEM}/ticket.md"),
        source.as_bytes(),
    )?;
    for clone in [&p.a, &p.b] {
        let value = polled(clone, R9);
        assert_eq!(disposition(&value, PRIMARY), "fast_forwarded");
    }
    Ok(())
}

/// `ITEM` closed by clone A through the binary.
fn close_in_a(p: &CliPair, request: &str) -> Result<(), FixtureError> {
    let a = &p.a;
    let args = ["ticket", "close", "--repo", a.root(), "--id", ITEM];
    let mut prepare = args.to_vec();
    prepare.push("--prepare");
    let preview = a.json(&prepare, &[], "ok", 0);
    let mut confirm = args.to_vec();
    confirm.extend([
        "--request-id",
        request,
        "--confirm",
        preview["data"]["confirmation_id"].as_str().unwrap(),
    ]);
    a.json(&confirm, &[], "ok", 0);
    assert_eq!(p.served(CONTEXT)?, None, "the closure deleted it");
    assert_eq!(a.tip(CONTEXT)?, None);
    Ok(())
}

/// `ITEM`'s source at `reference` in `clone` holds its closure.
fn holds_the_closure(clone: &CliClone, reference: &str) -> Result<(), FixtureError> {
    let repo = clone.repo()?;
    let tip = fixed(repo.find_commit(fixed(repo.refname_to_id(reference))?))?;
    let entry = fixed(
        fixed(tip.tree())?.get_path(Path::new(&format!(".manyhands/tickets/{ITEM}/ticket.md"))),
    )?;
    let blob = fixed(repo.find_blob(entry.id()))?;
    assert!(String::from_utf8_lossy(blob.content()).contains("\nclosed_at: "));
    Ok(())
}

/// What each read of `clone` says of `ITEM`: closed, and its dependent ready.
fn reads_closed(clone: &CliClone) {
    let shown = clone.json(
        &["ticket", "show", "--repo", clone.root(), "--id", ITEM],
        &[],
        "ok",
        0,
    );
    assert_eq!(shown["data"]["closure"]["state"], "closed", "{shown}");
    let ready = clone.json(&["ticket", "ready", "--repo", clone.root()], &[], "ok", 0);
    let ids: Vec<_> = ready["data"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["id"].as_str().unwrap().to_owned())
        .collect();
    assert!(ids.contains(&OTHER_ITEM.to_owned()), "{ready}");
    assert!(!ids.contains(&ITEM.to_owned()), "{ready}");
}

/// Decision 28: a ticket closed through the binary, whose branch then
/// reappears on the remote at its pre-closure tip, as a refused deletion or a
/// stale peer leaves it. Primary holds that tip, so the latest copy is
/// primary's: the poll reports the branch current, in the closing clone and in
/// the other, and makes no branch or worktree. Every read agrees the ticket is
/// closed, a save is refused, and nothing is pushed.
fn c5_polling_remote_context_of_a_closed_ticket() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let port = p.server.address().port();
    let (a, b) = (&p.a, &p.b);
    a_dependent(&p)?;
    published(a, R1, R2, "Saved in clone A.\n");
    let stale = a.tip(CONTEXT)?.ok_or(FixtureError)?;
    close_in_a(&p, R3)?;
    let closed = p.served(PRIMARY)?;
    fixed(p.bare()?.reference(CONTEXT, stale, false, "a stale peer"))?;
    let pushes = remote_world::accepted(&p.server);

    let value = polled(a, R4);
    assert_eq!(disposition(&value, CONTEXT), "current");
    json_golden("poll_once_closed_ticket", &value, &a.root, port);
    let in_b = polled(b, R1);
    assert_eq!(disposition(&in_b, CONTEXT), "current");
    for clone in [a, b] {
        assert_eq!(clone.tip(CONTEXT)?, None, "no branch");
        assert!(!clone.context().exists(), "no worktree");
        assert_eq!(clone.tip(PRIMARY)?, closed);
        reads_closed(clone);
    }
    let out = text_once(&b.env, b.root(), R5);
    assert_eq!(out.status, 0);
    assert!(stdout(&out).contains(&format!(
        "target: {CONTEXT} kind=ticket item={ITEM} disposition=current"
    )));
    assert!(stdout(&out).contains("  Local state is current.\n"));

    let shown = b.json(
        &["ticket", "show", "--repo", b.root(), "--id", ITEM],
        &[],
        "ok",
        0,
    );
    let input = serde_json::json!({
        "observation": shown["data"]["observation"],
        "body": "Saved after the closure.\n",
    });
    let mut args = vec!["ticket", "save", "--repo", b.root(), "--id", ITEM];
    args.extend(["--request-id", R6, "--input", "-", "--json"]);
    let out = run(&b.env, &args, &fixed(serde_json::to_vec(&input))?);
    let saved = cli::json(&out);
    assert_eq!(saved["code"], "ticket_closed", "{saved}");
    assert_eq!(out.status, 3);
    assert_eq!(saved["effects"]["write"], "not_requested");
    for clone in [a, b] {
        assert_eq!(clone.tip(CONTEXT)?, None);
        reads_closed(clone);
    }
    assert_eq!(remote_world::accepted(&p.server), pushes, "nothing pushed");
    assert_eq!(p.served(CONTEXT)?, Some(stale));
    assert_eq!(p.served(PRIMARY)?, closed);
    Ok(())
}

/// S2: a clone that materialized the context before another clone closed the
/// ticket keeps its old copy, which decision 28 doesn't repair. What `item
/// sync` of it does, pinned.
fn c5_polling_item_sync_of_a_closed_ticket_s_old_copy() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let (a, b) = (&p.a, &p.b);
    published(a, R1, R2, "Saved in clone A.\n");
    let value = polled(b, R1);
    assert_eq!(disposition(&value, CONTEXT), "materialized");
    let old = b.tip(CONTEXT)?.ok_or(FixtureError)?;
    close_in_a(&p, R3)?;
    let closed = p.served(PRIMARY)?;
    let pushes = remote_world::accepted(&p.server);

    let args = [
        "item",
        "sync",
        "--repo",
        b.root(),
        "--id",
        ITEM,
        "--request-id",
        R4,
        "--json",
    ];
    let out = run(&b.env, &args, &[]);
    let synced = cli::json(&out);
    // The closure's deletion is what B's fetch sees: the branch was published
    // and is gone, so the old copy is not pushed back.
    assert_eq!(synced["code"], "remote_branch_deleted", "{synced}");
    assert_eq!(out.status, 3);
    assert_eq!(synced["effects"]["publication"], "pending");
    assert_eq!(synced["recovery"][0]["action"], "request.cancel");
    assert_eq!(p.served(CONTEXT)?, None, "not published");
    assert_eq!(remote_world::accepted(&p.server), pushes, "nothing pushed");
    assert_eq!(p.served(PRIMARY)?, closed);
    assert_eq!(b.tip(CONTEXT)?, Some(old), "and kept");
    assert!(b.context().is_dir());
    Ok(())
}

/// S2, as a re-materialized copy stands: B holds the context, and A's close had
/// its remote deletion refused, so the remote still holds the branch at B's tip.
/// What `item sync` in B does, pinned.
fn c5_polling_item_sync_of_a_copy_the_remote_still_holds() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let (a, b) = (&p.a, &p.b);
    published(a, R1, R2, "Saved in clone A.\n");
    let value = polled(b, R1);
    assert_eq!(disposition(&value, CONTEXT), "materialized");
    let old = b.tip(CONTEXT)?.ok_or(FixtureError)?;
    p.server.reject_context_deletion(true)?;
    // The fixture's native refusal applies to the receiver's current branch.
    fixed(p.bare()?.set_head(CONTEXT))?;
    let args = ["ticket", "close", "--repo", a.root(), "--id", ITEM];
    let mut prepare = args.to_vec();
    prepare.push("--prepare");
    let preview = a.json(&prepare, &[], "ok", 0);
    let mut confirm = args.to_vec();
    confirm.extend([
        "--request-id",
        R3,
        "--confirm",
        preview["data"]["confirmation_id"].as_str().unwrap(),
    ]);
    let refused = a.json(&confirm, &[], "push_rejected", 4);
    assert_eq!(refused["effects"]["cleanup"], "pending");
    fixed(p.bare()?.set_head(PRIMARY))?;
    let served = p.served(CONTEXT)?;
    let closed = p.served(PRIMARY)?;
    let pushes = remote_world::accepted(&p.server);

    let args = [
        "item",
        "sync",
        "--repo",
        b.root(),
        "--id",
        ITEM,
        "--request-id",
        R4,
        "--json",
    ];
    let out = run(&b.env, &args, &[]);
    let synced = cli::json(&out);
    // Not the old copy: the sync takes B's context up to primary, which holds
    // the closure, and publishes that, in one push. The branch stays.
    assert_eq!(synced["code"], "ok", "{synced}");
    assert_eq!(out.status, 0);
    assert_eq!(synced["effects"]["publication"], "published");
    assert_eq!(served, Some(old), "the remote held B's copy");
    assert_eq!(b.tip(CONTEXT)?, closed);
    assert_eq!(p.served(CONTEXT)?, closed);
    assert_eq!(remote_world::accepted(&p.server), pushes + 1);
    assert_eq!(p.served(PRIMARY)?, closed, "primary keeps the closure");
    holds_the_closure(b, CONTEXT)?;
    Ok(())
}

/// A world the CLI drives, with `World`'s selected key.
fn world_env(world: &remote_world::World) -> CliEnv {
    private_fixture(&world.server);
    let env = CliEnv::new(world.data());
    fixed(std::fs::write(
        env.home.path().join(".gitconfig"),
        "[core]\n\tautocrlf = false\n",
    ))
    .unwrap();
    env
}

fn to_serve(world: &remote_world::World) -> Result<git2::Oid, FixtureError> {
    remote_world::advance(&world.peer, PRIMARY, "peer.txt")?;
    remote_world::push_peer(&world.peer, &world.server, PRIMARY)?;
    fixed(world.peer.refname_to_id(PRIMARY))
}

/// The first stop names the host and offers its approval. It is final: the same
/// request answers it again with nothing to offer, and only a new request polls
/// once the host is approved.
fn c5_polling_host_stop_is_final_then_a_new_request() -> Result<(), FixtureError> {
    let w = remote_world::World::plain_key()?;
    let env = world_env(&w);
    let port = w.server.address().port();
    let root = w.root.to_str().unwrap();
    let incoming = to_serve(&w)?;

    let first = json_once(&env, root, R1, "host_approval_required", 3);
    assert_eq!(first["recovery"][0]["action"], "host.approve");
    golden("poll_once_host_stop", &first, &w.root, port);

    let out = text_once(&env, root, R2);
    assert_eq!(out.status, 3);
    let said = stderr(&out);
    assert!(said.contains("code: host_approval_required\n"), "{said}");
    assert!(said.contains("next: host.approve "), "{said}");
    assert!(said.contains("a new request"), "{said}");
    assert!(!said.contains("request.retry"), "{said}");
    text_golden("poll_once_host_stop_text", &out, &w.root, port);

    let again = json_once(&env, root, R1, "host_approval_required", 3);
    assert_eq!(again["data"], first["data"]);
    assert!(again["recovery"].as_array().unwrap().is_empty(), "{again}");

    let host = host_input(&w.server);
    let preview = prepare(&env, &w.root, "approve", &host);
    confirm(&env, &w.root, "approve", &host, &preview, "ok", 0);
    let after = json_once(&env, root, R1, "host_approval_required", 3);
    assert!(
        after["recovery"].as_array().unwrap().is_empty(),
        "still final"
    );
    // The human replay of the stopped request says so too.
    let out = text_once(&env, root, R2);
    assert_eq!(out.status, 3);
    assert!(
        stderr(&out).contains("polling again takes a new request."),
        "{}",
        stderr(&out)
    );
    assert!(
        !stderr(&out).contains("next: host.approve"),
        "{}",
        stderr(&out)
    );
    assert_ne!(fixed(w.repo()?.refname_to_id(PRIMARY))?, incoming);

    let value = json_once(&env, root, R3, "ok", 0);
    assert_eq!(disposition(&value, PRIMARY), "fast_forwarded");
    assert_eq!(fixed(w.repo()?.refname_to_id(PRIMARY))?, incoming);
    Ok(())
}

/// With no terminal a protected key stops the poll, finally, and offers nothing.
/// At a terminal, a new request asks for the passphrase and polls.
#[cfg(unix)]
fn c5_polling_unlock_stop_is_final_then_a_new_request() -> Result<(), FixtureError> {
    use repository::{keys::*, transport::*};
    use support::pty::PtyCli;
    let w = remote_world::World::new()?;
    let env = world_env(&w);
    let port = w.server.address().port();
    let root = w.root.to_str().unwrap();
    fixed(w.service.verify_ssh_transport(
        VerifySshTransportRequest {
            root: w.root.clone(),
            direction: SshDirection::Fetch,
            approval: Some(HostApproval {
                authority: SshAuthority {
                    host: "127.0.0.1".into(),
                    port,
                },
                expected: None,
                presented: w.server.host_identity(),
            }),
        },
        &mut SessionCredentials::new(remote_world::Provider),
    ))?;
    let incoming = to_serve(&w)?;

    let first = json_once(&env, root, R1, "unlock_required", 3);
    assert!(first["recovery"].as_array().unwrap().is_empty(), "{first}");
    golden("poll_once_unlock_stop", &first, &w.root, port);
    let out = text_once(&env, root, R2);
    assert_eq!(out.status, 3);
    let said = stderr(&out);
    assert!(said.contains("code: unlock_required\n"), "{said}");
    assert!(said.contains("a new request"), "{said}");
    assert!(said.contains("at a terminal"), "{said}");
    text_golden("poll_once_unlock_stop_text", &out, &w.root, port);
    let replay = json_once(&env, root, R1, "unlock_required", 3);
    assert_eq!(replay, first);
    assert_ne!(fixed(w.repo()?.refname_to_id(PRIMARY))?, incoming);

    let args = ["poll", "once", "--repo", root];
    let mut pty = PtyCli::spawn(&args.iter().map(OsStr::new).collect::<Vec<_>>(), &env, b"");
    pty.expect("Passphrase for key");
    pty.send(format!("{}\n", remote_world::PASSWORD).as_bytes());
    let (out, transcript) = pty.finish();
    scan_output(&out);
    absent(transcript.as_bytes(), remote_world::PASSWORD);
    assert_eq!(out.status, 0, "{}", stderr(&out));
    assert!(stdout(&out).contains("disposition=fast_forwarded"));
    assert_eq!(fixed(w.repo()?.refname_to_id(PRIMARY))?, incoming);

    let still = json_once(&env, root, R1, "unlock_required", 3);
    assert_eq!(still, first, "the first request's answer stands");
    Ok(())
}

/// No publication remote: refused with nothing written, no record and no
/// connection.
fn c5_polling_no_remote() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let port = p.server.address().port();
    let a = &p.a;
    let source = a.root.join(".manyhands/config.toml");
    let text = fixed(std::fs::read_to_string(&source))?;
    fixed(std::fs::write(
        &source,
        text.replace("publication_remote = \"origin\"\n", ""),
    ))?;
    let opened = collaboration::connections(&p);
    let tracking = refs(&a.repo()?)?;

    let value = json_once(&a.env, a.root(), R1, "publication_remote_required", 3);
    assert!(value["data"].is_null());
    golden("poll_once_no_remote", &value, &a.root, port);
    let shown = run(
        &a.env,
        &["request", "show", "--request-id", R1, "--json"],
        &[],
    );
    assert_eq!(cli::json(&shown)["code"], "request_not_found");

    let out = text_once(&a.env, a.root(), R1);
    assert_eq!(out.status, 3);
    assert!(stderr(&out).contains("a new request"), "{}", stderr(&out));
    text_golden("poll_once_no_remote_text", &out, &a.root, port);
    assert_eq!(collaboration::connections(&p), opened);
    assert_eq!(refs(&a.repo()?)?, tracking);
    Ok(())
}

/// What the command line refuses: no poll, no connection and no record, in
/// both modes.
fn c5_polling_usage_failure() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let port = p.server.address().port();
    let a = &p.a;
    let opened = collaboration::connections(&p);
    let root = a.root();
    let rows: [(&[&str], &str, i32); 5] = [
        (&["poll", "once", "--repo", root], "usage_error", 2),
        (&["poll", "once", "--request-id", R1], "usage_error", 2),
        (
            &[
                "poll",
                "once",
                "--repo",
                root,
                "--request-id",
                R1,
                "--input",
                "-",
            ],
            "usage_error",
            2,
        ),
        (
            &["poll", "once", "--repo", root, "--request-id", "bad"],
            "invalid_id",
            2,
        ),
        (
            &["poll", "once", "--repo", root, "--prepare"],
            "not_confirmable",
            2,
        ),
    ];
    for (args, code, status) in rows {
        let human = run(&a.env, args, &[]);
        assert_eq!(human.status, status, "{args:?}");
        assert!(human.stdout.is_empty());
        assert!(
            stderr(&human).contains(&format!("code: {code}\n")),
            "{}",
            stderr(&human)
        );
        let mut args = args.to_vec();
        args.push("--json");
        let out = run(&a.env, &args, &[]);
        assert!(out.stderr.is_empty());
        let value = cli::json(&out);
        assert_eq!(value["code"], code, "{args:?}");
        assert_eq!(out.status, status);
        assert!(value["data"].is_null());
    }
    let value = checked(
        &a.env,
        &["poll", "once", "--repo", root],
        &[],
        "usage_error",
        2,
        None,
    );
    golden("poll_once_usage", &value, &a.root, port);
    assert_eq!(collaboration::connections(&p), opened);
    Ok(())
}

/// A finished request replays with no connection and no second effect, and its
/// ID with another command is `request_mismatch`.
fn c5_polling_replay_and_mismatch() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let port = p.server.address().port();
    let a = &p.a;
    advance_primary(&p, INCOMING)?;
    let first = polled(a, R1);
    assert_eq!(disposition(&first, PRIMARY), "fast_forwarded");

    let opened = collaboration::connections(&p);
    let moved = advance_primary(&p, "later.txt")?;
    let replay = polled(a, R1);
    assert_eq!(replay, first, "what it reported then");
    assert_eq!(collaboration::connections(&p), opened);
    assert_ne!(a.tip(PRIMARY)?, Some(moved));

    let args = ["poll", "pause", "--repo", a.root(), "--request-id", R1];
    let mismatch = a.json(&args, &[], "request_mismatch", 2);
    assert!(mismatch["data"].is_null());
    golden("poll_once_request_mismatch", &mismatch, &a.root, port);
    let status = a.json(&["poll", "status", "--repo", a.root()], &[], "ok", 0);
    assert_eq!(status["data"]["paused"], false);

    let args = ["poll", "pause", "--repo", a.root(), "--request-id", R2];
    a.json(&args, &[], "ok", 0);
    let other = json_once(&a.env, a.root(), R2, "request_mismatch", 2);
    assert!(other["data"].is_null());
    let out = text_once(&a.env, a.root(), R2);
    assert_eq!(out.status, 2);
    text_golden("poll_once_request_mismatch_text", &out, &a.root, port);
    assert_ne!(a.tip(PRIMARY)?, Some(moved));
    assert_eq!(collaboration::connections(&p), opened);
    Ok(())
}

/// The repository's lease, held outside the CLI's process. The answer is `busy`
/// after C2's bounded retries, the request stays open and the human lines say
/// how to find what holds it. The same request then polls.
fn c5_polling_a_second_process_holds_the_lease() -> Result<(), FixtureError> {
    use fs4::fs_std::FileExt;
    let p = CliPair::new()?;
    let port = p.server.address().port();
    let a = &p.a;
    let incoming = advance_primary(&p, INCOMING)?;
    let before = a.tip(PRIMARY)?;
    let lease = fixed(
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(a.repo()?.commondir().join("manyhands-operation.lock")),
    )?;
    assert!(fixed(FileExt::try_lock_exclusive(&lease))?);

    let value = json_once(&a.env, a.root(), R1, "busy", 5);
    assert_eq!(value["recovery"][0]["action"], "request.retry");
    golden("poll_once_busy", &value, &a.root, port);
    assert_eq!(a.tip(PRIMARY)?, before);

    let out = text_once(&a.env, a.root(), R1);
    assert_eq!(out.status, 5);
    let said = stderr(&out);
    assert!(said.contains("code: busy\n"), "{said}");
    let list = said
        .find("next: manyhands-cli operation list ")
        .expect(said);
    let resume = said
        .find("next: manyhands-cli operation resume ")
        .expect(said);
    assert!(list < resume, "{said}");
    text_golden("poll_once_busy_text", &out, &a.root, port);

    fixed(FileExt::unlock(&lease))?;
    let polled_now = polled(a, R1);
    assert_eq!(disposition(&polled_now, PRIMARY), "fast_forwarded");
    assert_eq!(polled_now["operation_id"], value["operation_id"]);
    assert_eq!(a.tip(PRIMARY)?, Some(incoming));
    Ok(())
}

/// A real second process, while the first is inside its transfer (decision 24):
/// it answers `busy` and the first goes on. The same request then polls.
/// A child process that is killed and reaped if the case stops before it is
/// collected.
struct Reaped(Option<std::process::Child>);

impl Drop for Reaped {
    fn drop(&mut self) {
        if let Some(child) = self.0.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn c5_polling_a_second_poll_overlaps() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let a = &p.a;
    let incoming = advance_primary(&p, INCOMING)?;
    let hold = p.server.hold_transfer_after(1)?;
    let mut args = once(a.root(), R1);
    args.push("--json");
    let argv: Vec<_> = args.iter().map(OsStr::new).collect();
    let mut running = Reaped(Some(fixed(
        cli::cli_command(&argv, &a.env)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn(),
    )?));
    // The helper's wait is fixed at 10 s, and takes no other.
    hold.wait_until_held()?;

    let second = run(&a.env, &[once(a.root(), R2), vec!["--json"]].concat(), &[]);
    let second = cli::json(&second);
    assert_eq!(second["code"], "busy", "{second}");
    assert_eq!(second["recovery"][0]["action"], "request.retry");
    hold.release()?;
    let first = cli::collect_cli(running.0.take().ok_or(FixtureError)?);
    scan_output(&first);
    assert_eq!(
        first.status,
        0,
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert_eq!(cli::json(&first)["code"], "ok");
    assert_eq!(a.tip(PRIMARY)?, Some(incoming));

    let value = polled(a, R2);
    assert_eq!(disposition(&value, PRIMARY), "current");
    Ok(())
}

/// A document of clone A's own, made by `document create`, local only.
const DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FDN";

/// Decision 29 through the binary: a published promotion, then a close that
/// prepares with no poll between them, and a re-prepare of the closed ticket
/// that reads it as closed.
fn c5_polling_promote_then_close_needs_no_poll() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let a = &p.a;
    let created = serde_json::json!({
        "item_id": DOCUMENT,
        "path": "docs/new.md",
        "title": "New document",
        "body": "A new document body.\n",
    });
    a.json(
        &[
            "document",
            "create",
            "--repo",
            a.root(),
            "--request-id",
            R1,
            "--input",
            "-",
        ],
        &fixed(serde_json::to_vec(&created))?,
        "ok",
        0,
    );
    save(a, R2, "Saved in clone A.\n");
    let confirmed = |resource: &str, verb: &str, id: &str, request: &str| {
        let args = [resource, verb, "--repo", a.root(), "--id", id];
        let mut prepare = args.to_vec();
        prepare.push("--prepare");
        let preview = a.json(&prepare, &[], "ok", 0);
        let mut confirm = args.to_vec();
        confirm.extend([
            "--request-id",
            request,
            "--confirm",
            preview["data"]["confirmation_id"].as_str().unwrap(),
        ]);
        a.json(&confirm, &[], "ok", 0)
    };
    let promoted = confirmed("document", "promote", DOCUMENT, R3);
    assert_eq!(
        promoted["effects"]["publication"], "published",
        "{promoted}"
    );
    let closed = confirmed("ticket", "close", ITEM, R4);
    assert_eq!(closed["effects"]["publication"], "published", "{closed}");
    assert_eq!(p.served(CONTEXT)?, None, "the closure deleted it");
    let again = a.json(
        &[
            "ticket",
            "close",
            "--repo",
            a.root(),
            "--id",
            ITEM,
            "--prepare",
        ],
        &[],
        "ticket_closed",
        3,
    );
    assert_eq!(
        again["data"]["blockers"],
        serde_json::json!(["already_closed"])
    );
    Ok(())
}

/// Spec review item 2: primary's tracking ref gone, as before a first fetch.
/// The blocked prepare names the poll that reads it, and once that has run
/// the same prepare is ready.
fn c5_polling_a_missing_tracking_ref_names_the_poll() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let port = p.server.address().port();
    let a = &p.a;
    save(a, R1, "Saved in clone A.\n");
    fixed(fixed(a.repo()?.find_reference("refs/remotes/origin/main"))?.delete())?;
    let args = [
        "ticket",
        "close",
        "--repo",
        a.root(),
        "--id",
        ITEM,
        "--prepare",
    ];
    let blocked = a.json(&args, &[], "recovery_required", 3);
    assert_eq!(
        blocked["data"]["blockers"],
        serde_json::json!(["primary_missing"])
    );
    let out = run(&a.env, &args, &[]);
    assert_eq!(out.status, 3);
    let text = stderr(&out);
    let poll = format!(
        "next: manyhands-cli poll once --repo '{}' --request-id <new-request-id>\n",
        a.root.canonicalize().unwrap().display()
    );
    assert!(text.contains(&poll), "{text}");
    assert!(text.contains("Then prepare again.\n"), "{text}");
    text_golden(
        "ticket_close_prepare_primary_missing_text",
        &out,
        &a.root,
        port,
    );
    polled(a, R2);
    a.json(&args, &[], "ok", 0);
    Ok(())
}
