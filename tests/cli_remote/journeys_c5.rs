//! The CLI counterparts of the PRD's six end-to-end journeys (design,
//! "Journeys"; Task 12b), each through the binary over SSH, asserting what Git
//! holds and the canonical content, and with every output scanned.
//!
//! "Final save" differs by design: a CLI caller saves first (RFC, "CLI callers
//! save drafts first"), so each journey saves before it promotes or closes, and
//! neither verb takes input.
//!
//! A variant the binary already proves as a row of its own is named here and
//! not repeated:
//!
//! - background update: the dirty, divergent, malformed and remotely deleted
//!   contexts are `c5_polling_dirty_context_preserved`,
//!   `c5_polling_divergent_context_preserved`,
//!   `c5_polling_malformed_ref_names_nothing` and
//!   `c5_polling_deleted_context_preserved`;
//! - conflict recovery: C4's `c4_conflicts_*` rows hold the conflict's every
//!   answer, and this journey is the thin one that resolves and retries;
//! - close with retryable failure: the refused push and the refused deletion
//!   are `c5_lifecycle_refused_primary_push_then_same_request_close` and
//!   `c5_lifecycle_refused_deletion_then_same_request_close`.

use super::*;
use cli::stderr;
use cli_pair::{CONTEXT, CliClone, CliPair, ITEM, OTHER_ITEM, PRIMARY};
use collaboration::{add, item_sync, repo_sync, request_show, save};
use lifecycle::{
    DOCUMENT, DOCUMENT_BRANCH, DOCUMENT_PATH, LocalOnly, Verb, closure_at, closure_checkpoints,
    finished_once, merges, peer_commit, poll, refs, source_at, ticket_path, ticket_source,
};
use ssh_remote::{FixtureBoundary, FixtureError, SshRemoteFixture, fixed};

pub const CASES: &[ssh_harness::Case] = &[
    (
        "c5_journey_offline_edit_then_sync",
        c5_journey_offline_edit_then_sync,
    ),
    ("c5_journey_conflict_recovery", c5_journey_conflict_recovery),
    (
        "c5_journey_background_update_and_discovery",
        c5_journey_background_update_and_discovery,
    ),
    (
        "c5_journey_threaded_discussion",
        c5_journey_threaded_discussion,
    ),
    (
        "c5_journey_document_promotion",
        c5_journey_document_promotion,
    ),
    (
        "c5_journey_document_promotion_local_only_then_published",
        c5_journey_document_promotion_local_only_then_published,
    ),
    (
        "c5_journey_close_with_retryable_failure",
        c5_journey_close_with_retryable_failure,
    ),
    (
        "c5_journey_close_local_only_then_published",
        c5_journey_close_local_only_then_published,
    ),
];

const R1: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR1";
const R2: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR2";
const R3: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR3";
const R4: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR4";
const R5: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR5";
const R6: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR6";
const R7: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR7";
const C1: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC1";
const C2: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC2";
const C3: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC3";
const C4: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC4";
/// A ticket a peer adds on the remote's primary.
const NEW_TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FNT";
const AUTHOR_A: &str = "Clone A <clone-a@example.invalid>";
const AUTHOR_B: &str = "Clone B <clone-b@example.invalid>";

/// `path`'s text at `reference` of `repository`.
fn text_at(
    repository: &git2::Repository,
    reference: &str,
    path: &str,
) -> Result<Option<String>, FixtureError> {
    source_at(
        repository,
        fixed(repository.refname_to_id(reference))?,
        path,
    )
}

fn worktrees(clone: &CliClone) -> Result<Vec<String>, FixtureError> {
    let mut found: Vec<String> = fixed(clone.repo()?.worktrees())?
        .iter()
        .flatten()
        .map(str::to_owned)
        .collect();
    found.sort();
    Ok(found)
}

// ---------------------------------------------------- offline, then sync

/// Offline edit, checkpoint and later sync. With the remote unreachable a
/// save checkpoints locally and contacts nothing. `item sync` then can't reach
/// the remote, names the same request as the way on and keeps the work; once
/// the remote is reachable that request publishes the checkpoint, byte for
/// byte. The variant: the remote refusing the next checkpoint is a recovery
/// with the work kept, and the same request publishes it once it is taken.
fn c5_journey_offline_edit_then_sync() -> Result<(), FixtureError> {
    const BODY: &str = "Written while the remote was unreachable.\n";
    let p = CliPair::new()?;
    let a = &p.a;
    p.server.disconnect_at(FixtureBoundary::Handshake);
    let opened = p.server.accepted_sockets();
    let saved = save(a, R1, BODY);
    assert_eq!(saved["effects"]["discovery"], "current");
    assert_eq!(
        p.server.accepted_sockets(),
        opened,
        "a save connects to nothing"
    );
    let checkpoint = a.tip(CONTEXT)?.ok_or(FixtureError)?;
    let repository = a.repo()?;
    let source = text_at(&repository, CONTEXT, &ticket_path(ITEM))?.ok_or(FixtureError)?;
    assert!(source.ends_with(BODY), "{source}");
    assert_eq!(p.served(CONTEXT)?, None);

    // Still unreachable: actionable feedback, the checkpoint kept.
    let out = run(
        &a.env,
        &[
            "item",
            "sync",
            "--repo",
            a.root(),
            "--id",
            ITEM,
            "--request-id",
            R2,
        ],
        &[],
    );
    assert_ne!(out.status, 0);
    let said = stderr(&out);
    assert!(said.contains("code: transport_unavailable\n"), "{said}");
    assert!(
        said.contains(&format!("next: request.retry request_id={R2}")),
        "the same request goes on: {said}"
    );
    assert_eq!(a.tip(CONTEXT)?, Some(checkpoint));
    assert_eq!(p.served(CONTEXT)?, None);
    p.server.clear_fault();

    // Reachable again: the same request publishes the same checkpoint, byte
    // for byte.
    let published = item_sync(a, ITEM, R2, "ok", 0);
    assert_eq!(published["effects"]["publication"], "published");
    assert_eq!(request_show(a, R2)["state"], "finished");
    assert_eq!(a.tip(CONTEXT)?, Some(checkpoint), "no new commit");
    assert_eq!(p.served(CONTEXT)?, Some(checkpoint));
    assert_eq!(
        text_at(&p.bare()?, CONTEXT, &ticket_path(ITEM))?.as_deref(),
        Some(source.as_str())
    );

    // The variant: the remote refuses the next checkpoint. A recovery that
    // names the same request, with the work kept, and once the remote takes
    // it, the same request publishes it.
    save(a, R3, "Written again, for a remote that refuses it.\n");
    let next = a.tip(CONTEXT)?.ok_or(FixtureError)?;
    fixed(p.bare()?.set_head(CONTEXT))?;
    p.server.reject_primary_updates(true)?;
    let out = run(
        &a.env,
        &[
            "item",
            "sync",
            "--repo",
            a.root(),
            "--id",
            ITEM,
            "--request-id",
            R4,
            "--json",
        ],
        &[],
    );
    let refused = cli::json(&out);
    assert_eq!(refused["code"], "push_rejected", "{refused}");
    assert_ne!(out.status, 0);
    assert_eq!(refused["effects"]["publication"], "pending");
    assert_eq!(refused["recovery"][0]["action"], "request.retry");
    assert_eq!(refused["recovery"][0]["arguments"]["request_id"], R4);
    assert_eq!(a.tip(CONTEXT)?, Some(next), "the work is kept");
    assert_eq!(p.served(CONTEXT)?, Some(checkpoint));
    p.server.reject_primary_updates(false)?;
    fixed(p.bare()?.set_head(PRIMARY))?;
    let published = item_sync(a, ITEM, R4, "ok", 0);
    assert_eq!(published["effects"]["publication"], "published");
    assert_eq!(p.served(CONTEXT)?, Some(next));
    assert_eq!(a.tip(CONTEXT)?, Some(next));
    Ok(())
}

// --------------------------------------------------------------- conflict

/// Concurrent collaboration conflict recovery, on the item: both clones save
/// `ITEM`, A publishes, and B's `item sync` stops on the conflict, naming the
/// item, the repository and the editing context, with B's checkpoint kept. B
/// reads it with the guidance's `conflict show`, resolves it with the tokens
/// read, and the same sync request then finishes: the resolution, a merge of
/// both sides, is what the remote's branch holds. C4's `c4_conflicts_*` hold
/// every other answer, the primary conflicts among them.
fn c5_journey_conflict_recovery() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let (a, b) = (&p.a, &p.b);
    save(a, R1, "Saved in clone A.\n");
    item_sync(a, ITEM, R2, "ok", 0);
    let incoming = p.served(CONTEXT)?.ok_or(FixtureError)?;
    save(b, R3, "Saved in clone B.\n");
    let local = b.tip(CONTEXT)?.ok_or(FixtureError)?;

    let stopped = item_sync(b, ITEM, conflicts::SYNC, "recovery_required", 3);
    assert_eq!(stopped["outcome"], "blocked", "{stopped}");
    let scope = &stopped["scope"];
    assert_eq!(scope["item_id"], ITEM, "names the item");
    assert_eq!(
        scope["repository"],
        b.root.canonicalize().unwrap().to_str().unwrap(),
        "names the repository"
    );
    assert_eq!(
        scope["branch"],
        CONTEXT.trim_start_matches("refs/heads/"),
        "names the context"
    );
    assert!(
        scope["worktree"]
            .as_str()
            .is_some_and(|worktree| worktree.ends_with(&format!(".manyhands/worktrees/{ITEM}"))),
        "{stopped}"
    );
    let operation = stopped["data"]["conflict"]["operation_id"]
        .as_str()
        .ok_or(FixtureError)?
        .to_owned();
    assert_eq!(stopped["recovery"][0]["action"], "request.retry");
    assert_eq!(b.tip(CONTEXT)?, Some(local), "B's work is kept");
    assert_eq!(p.served(CONTEXT)?, Some(incoming));

    let shown = conflicts::show(b, &operation);
    assert_eq!(shown["data"]["item_id"], ITEM);
    assert_eq!(shown["data"]["stage"], "context");
    assert_eq!(shown["data"]["paths"].as_array().unwrap().len(), 1);
    conflicts::private_source();
    let input = conflicts::resolve_input(
        conflicts::observation(&shown),
        &conflicts::resolutions(&shown, "Resolved"),
    );
    b.json(&conflicts::resolve_args(b, &operation, R4), &input, "ok", 0);
    let commit = b.tip(CONTEXT)?.ok_or(FixtureError)?;
    let repository = b.repo()?;
    assert_eq!(
        fixed(repository.find_commit(commit))?
            .parent_ids()
            .collect::<Vec<_>>(),
        [local, incoming]
    );
    let text = text_at(&repository, CONTEXT, &ticket_path(ITEM))?.ok_or(FixtureError)?;
    assert!(text.contains("\ntitle: Resolved\n"), "{text}");

    // The sync, retried under its own request: complete, the resolution
    // published.
    let retried = item_sync(b, ITEM, conflicts::SYNC, "ok", 0);
    assert_eq!(retried["outcome"], "success", "{retried}");
    assert_eq!(retried["effects"]["publication"], "published", "{retried}");
    assert_eq!(b.tip(CONTEXT)?, Some(commit));
    assert_eq!(p.served(CONTEXT)?, Some(commit));
    Ok(())
}

// ------------------------------------------------------ background update

/// Background update and discovery, explicitly: B publishes its context of
/// `ITEM` with a comment, and a peer publishes primary with a new ticket. One
/// `poll once` in A fast-forwards A's primary, makes exactly one worktree for
/// the new context, and indexes its folder and comments. A second poll finds
/// both current, still with one worktree. The preserved variants are
/// `polling.rs`'s rows, named in the module's comment. The peer's commit is
/// written into the bare repository directly, as a collaborator's push would
/// leave it, with no clone of its own.
fn c5_journey_background_update_and_discovery() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let (a, b) = (&p.a, &p.b);
    add(b, R1, C1, None, "a comment from clone B", "ok", 0);
    let context = p.served(CONTEXT)?.ok_or(FixtureError)?;
    let new = ticket_source(
        NEW_TICKET,
        "Added by a peer",
        "",
        "Body of Added by a peer.\n",
    );
    let incoming = peer_commit(&p, &ticket_path(NEW_TICKET), new.as_bytes())?;
    let pushes = remote_world::accepted(&p.server);
    assert!(worktrees(a)?.is_empty());

    let polled = poll(a, R2);
    let disposition = |value: &Value, reference: &str| {
        value["data"]["targets"]
            .as_array()
            .unwrap()
            .iter()
            .find(|target| target["target"] == reference)
            .map(|target| target["disposition"].clone())
    };
    assert_eq!(disposition(&polled, PRIMARY), Some("fast_forwarded".into()));
    assert_eq!(disposition(&polled, CONTEXT), Some("materialized".into()));
    assert_eq!(a.tip(PRIMARY)?, Some(incoming));
    assert_eq!(a.tip(CONTEXT)?, Some(context));
    assert_eq!(worktrees(a)?, [ITEM.to_owned()]);
    assert_eq!(
        text_at(&a.repo()?, PRIMARY, &ticket_path(NEW_TICKET))?.as_deref(),
        Some(new.as_str())
    );
    let shown = a.json(
        &["ticket", "show", "--repo", a.root(), "--id", NEW_TICKET],
        &[],
        "ok",
        0,
    );
    assert_eq!(shown["data"]["title"], "Added by a peer", "{shown}");
    let comments = a.json(
        &["comment", "list", "--repo", a.root(), "--id", ITEM],
        &[],
        "ok",
        0,
    );
    let items = comments["data"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 1, "{comments}");
    assert_eq!(items[0]["id"], C1);
    assert_eq!(items[0]["author"], AUTHOR_B);

    let again = poll(a, R3);
    assert_eq!(disposition(&again, PRIMARY), Some("current".into()));
    assert_eq!(disposition(&again, CONTEXT), Some("current".into()));
    assert_eq!(worktrees(a)?, [ITEM.to_owned()], "exactly one worktree");
    assert_eq!(
        remote_world::accepted(&p.server),
        pushes,
        "a poll pushes nothing"
    );
    Ok(())
}

// ----------------------------------------------------- threaded discussion

type Listed = (String, Option<String>, u64, String);

fn listed(clone: &CliClone, repo: &str) -> Vec<Listed> {
    let list = checked(
        &clone.env,
        &["comment", "list", "--repo", repo, "--id", ITEM],
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

/// Durable threaded discussion: roots from both clones and two replies to
/// one root, each published by its `comment add`. A's index is rebuilt, and
/// the item is read again from its own worktree: every root in creation order,
/// each reply beneath its parent in creation order, as B reads them too, and
/// every comment file on the remote's branch.
fn c5_journey_threaded_discussion() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let (a, b) = (&p.a, &p.b);
    add(a, R1, C1, None, "first root", "ok", 0);
    add(b, R2, C2, None, "second root", "ok", 0);
    add(
        b,
        R3,
        C3,
        Some(C1),
        "first reply to the first root",
        "ok",
        0,
    );
    item_sync(a, ITEM, R4, "already_applied", 0);
    add(
        a,
        R5,
        C4,
        Some(C1),
        "second reply to the first root",
        "ok",
        0,
    );
    item_sync(b, ITEM, R6, "already_applied", 0);
    let tip = p.served(CONTEXT)?.ok_or(FixtureError)?;
    assert_eq!((a.tip(CONTEXT)?, b.tip(CONTEXT)?), (Some(tip), Some(tip)));
    for comment in [C1, C2, C3, C4] {
        assert!(collaboration::holds(
            &p.bare()?,
            tip,
            &collaboration::comment_path(comment)
        )?);
    }

    let rebuilt = a.json(
        &["index", "rebuild", "--repo", a.root(), "--request-id", R7],
        &[],
        "ok",
        0,
    );
    assert_eq!(rebuilt["command"], "index rebuild");
    let expected: Vec<Listed> = vec![
        (C1.into(), None, 0, AUTHOR_A.into()),
        (C3.into(), Some(C1.into()), 1, AUTHOR_B.into()),
        (C4.into(), Some(C1.into()), 1, AUTHOR_A.into()),
        (C2.into(), None, 0, AUTHOR_B.into()),
    ];
    // The worktree changes: the item read from its own editing worktree.
    let worktree = a.context();
    assert_eq!(listed(a, worktree.to_str().unwrap()), expected);
    assert_eq!(listed(a, a.root()), expected);
    assert_eq!(listed(b, b.root()), expected);
    Ok(())
}

// --------------------------------------------------------------- promotion

/// Confirmed document promotion: a document created and saved, its preview
/// naming every effect, its confirmation integrating it into primary and
/// publishing it, and cleanup. The other clone's poll brings it in, and a
/// fresh `document save` of it starts a new editing context from primary.
fn c5_journey_document_promotion() -> Result<(), FixtureError> {
    const BODY: &str = "The promoted body.\n";
    let p = CliPair::new()?;
    let (a, b) = (&p.a, &p.b);
    let verb = Verb::Promote;
    verb.begin(a, R1);
    let shown = a.json(
        &["document", "show", "--repo", a.root(), "--id", DOCUMENT],
        &[],
        "ok",
        0,
    );
    let input = serde_json::json!({"observation": shown["data"]["observation"], "body": BODY});
    a.json(
        &[
            "document",
            "save",
            "--repo",
            a.root(),
            "--id",
            DOCUMENT,
            "--request-id",
            R2,
            "--input",
            "-",
        ],
        &fixed(serde_json::to_vec(&input))?,
        "ok",
        0,
    );
    let saved = a.tip(DOCUMENT_BRANCH)?.ok_or(FixtureError)?;

    let preview = verb.prepare(a, "ok", 0);
    let details = &preview["data"]["details"];
    assert_eq!(details["kind"], "document");
    assert_eq!(
        details["branch"],
        "manyhands/document/01ARZ3NDEKTSV4RRFFQ69G5FDC"
    );
    assert_eq!(details["primary_branch"], "main");
    assert_eq!(details["remote"], "origin");
    assert_eq!(details["context_oid"], saved.to_string().as_str());
    assert_eq!(
        details["changes"],
        serde_json::json!([{"path": DOCUMENT_PATH, "change": "added"}])
    );
    assert_eq!(details["integration"], "merge_commit");
    assert_eq!(details["publication"], "publish");
    assert_eq!(
        details["cleanup"],
        serde_json::json!({"remote_branch": true, "worktree": true, "local_branch": true})
    );
    assert!(details["closure"].is_null());

    let done = verb.confirm(a, R3, &preview, "ok", 0);
    assert_eq!(done["effects"]["publication"], "published", "{done}");
    assert_eq!(done["data"]["integration"], "merged");
    finished_once(&p, verb, None)?;
    let promoted = text_at(&p.bare()?, PRIMARY, DOCUMENT_PATH)?.ok_or(FixtureError)?;
    assert!(promoted.ends_with(BODY), "{promoted}");
    assert!(fixed(a.repo()?.graph_descendant_of(
        a.tip(PRIMARY)?.ok_or(FixtureError)?,
        saved
    ))?);

    // The other clone reads it from primary.
    poll(b, R1);
    assert_eq!(b.tip(PRIMARY)?, p.served(PRIMARY)?);
    let read = b.json(
        &["document", "show", "--repo", b.root(), "--id", DOCUMENT],
        &[],
        "ok",
        0,
    );
    assert_eq!(read["data"]["id"], DOCUMENT, "{read}");
    assert_eq!(
        read["data"]["body"], BODY,
        "B reads the promoted body: {read}"
    );
    assert_eq!(
        text_at(&b.repo()?, PRIMARY, DOCUMENT_PATH)?.as_deref(),
        Some(promoted.as_str())
    );

    // A fresh save of the promoted document starts a new context.
    let shown = a.json(
        &["document", "show", "--repo", a.root(), "--id", DOCUMENT],
        &[],
        "ok",
        0,
    );
    let input = serde_json::json!({"observation": shown["data"]["observation"], "body": "Edited after the promotion.\n"});
    a.json(
        &[
            "document",
            "save",
            "--repo",
            a.root(),
            "--id",
            DOCUMENT,
            "--request-id",
            R4,
            "--input",
            "-",
        ],
        &fixed(serde_json::to_vec(&input))?,
        "ok",
        0,
    );
    let fresh = a.tip(DOCUMENT_BRANCH)?.ok_or(FixtureError)?;
    let repository = a.repo()?;
    assert_eq!(
        fixed(repository.find_commit(fresh))?
            .parent_ids()
            .collect::<Vec<_>>(),
        [a.tip(PRIMARY)?.ok_or(FixtureError)?],
        "the new context starts at primary"
    );
    assert!(verb.worktree(a).is_dir());
    Ok(())
}

// ------------------------------------------------- local only, then later

impl LocalOnly {
    /// Decision 10's order: a key, then the remote, then the host, through
    /// the binary. The key is selected before any publication remote exists,
    /// so nothing is fenced (`01M66HFV44` is not reached).
    fn publish_later(&self, server: &SshRemoteFixture) -> Result<Value, FixtureError> {
        let clone = &self.clone;
        let key = clone.json(
            &["key", "import", "--input", "-", "--request-id", R5],
            &fixed(serde_json::to_vec(&serde_json::json!({
                "label": "Local key",
                "private_key_path": server.client_key_path(),
            })))?,
            "ok",
            0,
        );
        clone.json(
            &[
                "key",
                "select",
                "--key-id",
                key["data"]["id"].as_str().ok_or(FixtureError)?,
                "--request-id",
                R6,
            ],
            &[],
            "ok",
            0,
        );
        clone.json(
            &[
                "remote",
                "select",
                "--repo",
                clone.root(),
                "--input",
                "-",
                "--request-id",
                R7,
            ],
            br#"{"name":"origin"}"#,
            "ok",
            0,
        );
        // The selection's commit leaves the worktree clean (`01M669FJR2`).
        let mut options = git2::StatusOptions::new();
        options.include_untracked(true);
        assert!(fixed(clone.repo()?.statuses(Some(&mut options)))?.is_empty());
        let host = host_input(server);
        let preview = prepare(&clone.env, &clone.root, "approve", &host);
        confirm(&clone.env, &clone.root, "approve", &host, &preview, "ok", 0);
        Ok(repo_sync(clone, R4, "ok", 0))
    }
}

/// Local-only, then later publication, for one verb: the verb exits 0 with
/// publication pending and touches nothing remote; after `key select` and
/// `remote select`, `repo sync` publishes the integrated commit as it stands,
/// with no fixture seeding.
fn local_only_then_published(verb: Verb, created: Value) -> Result<(), FixtureError> {
    let server = SshRemoteFixture::start()?;
    private_fixture(&server);
    let local = LocalOnly::new(&server)?;
    let clone = &local.clone;
    let [resource, _] = verb.words();
    clone.json(
        &[
            resource,
            "create",
            "--repo",
            clone.root(),
            "--request-id",
            R1,
            "--input",
            "-",
        ],
        &fixed(serde_json::to_vec(&created))?,
        "ok",
        0,
    );
    let served = refs(&fixed(git2::Repository::open_bare(
        server.repository_path(),
    ))?)?;
    let preview = verb.prepare(clone, "ok", 0);
    assert!(preview["data"]["details"]["remote"].is_null());
    assert_eq!(
        preview["data"]["details"]["publication"],
        "pending_no_remote"
    );
    let done = verb.confirm(clone, R2, &preview, "ok", 0);
    assert_eq!(done["effects"]["publication"], "pending", "{done}");
    assert_eq!(done["data"]["publication_pending"], "no_publication_remote");
    assert_eq!(done["effects"]["cleanup"], "complete");
    let repository = clone.repo()?;
    assert_eq!(merges(&repository)?, 1);
    assert_eq!(clone.tip(verb.branch())?, None);
    let integrated = clone.tip(PRIMARY)?.ok_or(FixtureError)?;
    let bare = fixed(git2::Repository::open_bare(server.repository_path()))?;
    assert_eq!(refs(&bare)?, served, "nothing remote moved");

    let published = local.publish_later(&server)?;
    assert_eq!(
        published["effects"]["publication"], "published",
        "{published}"
    );
    let primary = clone.tip(PRIMARY)?.ok_or(FixtureError)?;
    assert!(fixed(repository.graph_descendant_of(primary, integrated))? || primary == integrated);
    assert_eq!(
        fixed(bare.refname_to_id(PRIMARY))?,
        primary,
        "primary is published"
    );
    // The item's bytes on the remote's primary are local primary's.
    let here = text_at(&repository, PRIMARY, &verb.path())?.ok_or(FixtureError)?;
    assert_eq!(
        text_at(&bare, PRIMARY, &verb.path())?.as_deref(),
        Some(here.as_str())
    );
    if verb == Verb::Close {
        assert_eq!(closure_checkpoints(&repository)?, 1);
        assert_eq!(
            closure_at(&bare, PRIMARY)?,
            closure_at(&repository, PRIMARY)?
        );
        assert_eq!(closure_at(&bare, PRIMARY)?.len(), 2);
    }
    assert_eq!(fixed(bare.refname_to_id(verb.branch())).ok(), None);
    // The server shuts down when it drops, as every case's does.
    Ok(())
}

fn c5_journey_document_promotion_local_only_then_published() -> Result<(), FixtureError> {
    local_only_then_published(
        Verb::Promote,
        serde_json::json!({
            "item_id": DOCUMENT,
            "path": DOCUMENT_PATH,
            "title": "Local document",
            "body": "Promoted while local only.\n",
        }),
    )
}

fn c5_journey_close_local_only_then_published() -> Result<(), FixtureError> {
    local_only_then_published(
        Verb::Close,
        serde_json::json!({
            "item_id": ITEM,
            "title": "Local ticket",
            "type": "task",
            "status": "open",
            "body": "Closed while local only.\n",
        }),
    )
}

// ---------------------------------------------------------------- closure

/// Confirmed close with a retryable failure. A dependent of `ITEM` is on
/// primary. A dirty primary, a close prerequisite, refuses the prepare and is
/// named as its blocker, with the ticket's work as it was. Corrected, the
/// close is prepared and confirmed, and the injected failure comes after the
/// request is accepted: the server drops the context synchronization's
/// connection. The answer names that step (`transport_unavailable`, after
/// `progress: connecting`), offers the same request, and leaves the ticket's
/// context, branch and worktree, with no cleanup. The same request then
/// completes: published, cleaned up, one merge and one checkpoint, and
/// `ticket ready` lists what the report made ready.
fn c5_journey_close_with_retryable_failure() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let a = &p.a;
    let verb = Verb::Close;
    let dependent = ticket_source(
        OTHER_ITEM,
        "Other",
        &format!("deps: [{ITEM}]\n"),
        "Body of Other.\n",
    );
    peer_commit(&p, &ticket_path(OTHER_ITEM), dependent.as_bytes())?;
    poll(a, R1);
    verb.begin(a, R2);
    let (context, served) = (a.tip(CONTEXT)?, refs(&p.bare()?)?);
    let intact = || -> Result<(), FixtureError> {
        // Nothing is lost and nothing is cleaned up early.
        assert_eq!(a.tip(CONTEXT)?, context);
        assert!(verb.worktree(a).is_dir());
        assert_eq!(refs(&p.bare()?)?, served);
        assert_eq!(merges(&a.repo()?)?, 0);
        Ok(())
    };

    let dirty = a.root.join("dirty-primary.txt");
    fixed(std::fs::write(&dirty, b"dirty\n"))?;
    let blocked = verb.prepare(a, "recovery_required", 3);
    assert_eq!(
        blocked["data"]["blockers"],
        serde_json::json!(["primary_dirty"])
    );
    intact()?;
    fixed(std::fs::remove_file(&dirty))?;

    let preview = verb.prepare(a, "ok", 0);
    assert_eq!(
        preview["data"]["details"]["closure"],
        serde_json::json!({"recording": "record", "closed_by": AUTHOR_A})
    );
    p.server.disconnect_at(FixtureBoundary::Advertisement);
    let failed = verb.confirm(a, R3, &preview, "transport_unavailable", 5);
    assert_eq!(failed["effects"]["integration"], "not_requested");
    assert_eq!(failed["data"]["item_id"], ITEM);
    assert_eq!(failed["recovery"][0]["action"], "request.retry");
    assert_eq!(request_show(a, R3)["state"], "accepted");
    let mut again = vec!["ticket", "close", "--repo", a.root(), "--id", ITEM];
    again.extend(["--request-id", R3]);
    let out = run(&a.env, &again, &[]);
    assert_eq!(out.status, 5, "{}", stderr(&out));
    let said = stderr(&out);
    let step = said.find("progress: connecting\n").expect(said);
    let code = said.find("code: transport_unavailable\n").expect(said);
    assert!(step < code, "the failed step, then the code: {said}");
    assert!(
        said.contains(&format!("next: request.retry request_id={R3}")),
        "{said}"
    );
    intact()?;
    p.server.clear_fault();

    let done = verb.send(a, R3, "ok", 0);
    assert_eq!(done["effects"]["publication"], "published", "{done}");
    assert_eq!(done["data"]["made_ready_state"], "current");
    let made_ready: Vec<&str> = done["data"]["made_ready"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["id"].as_str().unwrap())
        .collect();
    assert_eq!(made_ready, [OTHER_ITEM]);
    finished_once(&p, verb, None)?;
    let closed = closure_at(&p.bare()?, PRIMARY)?;
    assert!(
        closed.contains(&format!("closed_by: {AUTHOR_A}")),
        "{closed:?}"
    );

    let ready = a.json(&["ticket", "ready", "--repo", a.root()], &[], "ok", 0);
    let ids: Vec<&str> = ready["data"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["id"].as_str().unwrap())
        .collect();
    for id in &made_ready {
        assert!(ids.contains(id), "{ready}");
    }
    assert!(!ids.contains(&ITEM), "{ready}");
    let shown = a.json(
        &["ticket", "show", "--repo", a.root(), "--id", ITEM],
        &[],
        "ok",
        0,
    );
    assert_eq!(shown["data"]["closure"]["state"], "closed", "{shown}");
    Ok(())
}
