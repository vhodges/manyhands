//! `item republish`: the design's republish table (Task 5), a
//! `c4_republish_` case per row, three for the row of a transport, unlock or
//! host stop, and the command's conformance row.
//!
//! Each world publishes the item's editing branch through `item sync`; then
//! another deletes that branch on the remote, and `item sync` records the
//! deletion. A fetch records it when an earlier synchronization fetched the
//! published branch; otherwise only the push evidence of the publication
//! does. `item sync` is refused, and holds the reservation until its request
//! is cancelled and sent once more (`01M4R19PC0T6ZPKM39X9DF26HX`), which every
//! world but the busy row's does.
//!
//! The row for `operation resume` of a republish stopped after its push is
//! here; the resume table's own row for an operation an `item republish`
//! request holds is `resume::c4_resume_republish`. The Ctrl-C at the CLI's
//! consent prompt is Task 12b's: here a call whose token is cancelled before
//! it is accepted stands for it.

use std::{
    cell::{Cell, OnceCell},
    path::PathBuf,
    rc::Rc,
};

use rusqlite::OptionalExtension;

use super::*;

pub(crate) const CASES: &[ssh_harness::Case] = &[
    ("c4_republish_preview_fetched", c4_republish_preview_fetched),
    ("c4_republish_preview_pushed", c4_republish_preview_pushed),
    (
        "c4_republish_preview_ambiguous",
        c4_republish_preview_ambiguous,
    ),
    ("c4_republish_preview_unknown", c4_republish_preview_unknown),
    (
        "c4_republish_recreated_before_the_push",
        c4_republish_recreated_before_the_push,
    ),
    (
        "c4_republish_resumed_after_the_push",
        c4_republish_resumed_after_the_push,
    ),
    ("c4_republish_preview_noop", c4_republish_preview_noop),
    ("c4_republish_ticket_closed", c4_republish_ticket_closed),
    ("c4_republish_confirmed", c4_republish_confirmed),
    ("c4_republish_primary_moved", c4_republish_primary_moved),
    (
        "c4_republish_ancestor_of_primary",
        c4_republish_ancestor_of_primary,
    ),
    ("c4_republish_local_moved", c4_republish_local_moved),
    ("c4_republish_reappeared", c4_republish_reappeared),
    ("c4_republish_busy", c4_republish_busy),
    (
        "c4_republish_local_sync_unfinished",
        c4_republish_local_sync_unfinished,
    ),
    ("c4_republish_transport_stop", c4_republish_transport_stop),
    ("c4_republish_unlock_stop", c4_republish_unlock_stop),
    ("c4_republish_host_stop", c4_republish_host_stop),
    ("c4_republish_replayed", c4_republish_replayed),
    ("c4_republish_item_sync_after", c4_republish_item_sync_after),
    ("c4_republish_cancelled", c4_republish_cancelled),
    (
        "c4_republish_cancelled_before_execution",
        c4_republish_cancelled_before_execution,
    ),
    ("c4_republish_conformance", c4_republish_conformance),
];

/// The `item sync` that first publishes the item's editing branch.
const PUBLISH: &str = "01ARZ3NDEKTSV4RRFFQ69G5FP1";
/// The `item sync` that fetches the published branch, which records it.
const OBSERVE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FP2";
/// The `item sync` refused for the deletion: `remote_branch_deleted`.
const REFUSED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FP3";
/// `item republish`.
pub(super) const REPUBLISH: &str = "01ARZ3NDEKTSV4RRFFQ69G5FP4";
/// Another `item republish` request.
const ANOTHER: &str = "01ARZ3NDEKTSV4RRFFQ69G5FP5";
/// An `item sync` after the republish.
const AFTER: &str = "01ARZ3NDEKTSV4RRFFQ69G5FP6";
/// `operation resume`.
const RESUMING: &str = "01ARZ3NDEKTSV4RRFFQ69G5FP7";

/// The schema of a republish preview's data.
const PREVIEW_SCHEMA: &str = "item_republish_preview.schema.json";

/// The state a fetch records of a published branch it found: there, and
/// not yet materialized by a poll. The remote snapshot reads it as
/// `observed` for an item that has a local context.
const PRESENT: &str = "unmaterialized";

/// The item's branch, as the preview and the scope name it.
fn branch() -> &'static str {
    CONTEXT.strip_prefix("refs/heads/").expect("a branch")
}

fn republish_of(w: &World, id: &str) -> Mutation {
    Mutation::ItemRepublish(ItemRepublishInput {
        root: w.root.clone(),
        kind: AuthoringKind::Ticket,
        id: id.parse().unwrap(),
    })
}

/// `item republish` of the world's item.
pub(super) fn republish(w: &World) -> Mutation {
    republish_of(w, ITEM)
}

/// The item's editing worktree.
fn worktree(w: &World) -> PathBuf {
    w.root.join(".manyhands/worktrees").join(ITEM)
}

/// The commit the remote holds on the item's branch, when it has one.
fn served(w: &World) -> Result<Option<git2::Oid>, FixtureError> {
    match w.bare()?.refname_to_id(CONTEXT) {
        Ok(oid) => Ok(Some(oid)),
        Err(error) if error.code() == git2::ErrorCode::NotFound => Ok(None),
        Err(_) => Err(FixtureError),
    }
}

/// The local tip of the item's branch.
fn local(w: &World) -> Result<git2::Oid, FixtureError> {
    fixed(w.repo()?.refname_to_id(CONTEXT))
}

/// What a fetch recorded of the item's branch: its state, when a fetch
/// ever saw it published.
fn context_state(w: &World) -> Result<Option<String>, FixtureError> {
    fixed(
        w.db()?
            .query_row(
                "SELECT state FROM remote_context_states WHERE kind = 'ticket' AND item_id = ?1",
                [ITEM],
                |row| row.get(0),
            )
            .optional(),
    )
}

/// How many confirmations there are.
fn confirmations(w: &World) -> Result<i64, FixtureError> {
    fixed(
        w.db()?
            .query_row("SELECT COUNT(*) FROM confirmation_records", [], |row| {
                row.get(0)
            }),
    )
}

/// The configuration generation the remote journal is at.
fn generation(w: &World) -> Result<i64, FixtureError> {
    fixed(w.db()?.query_row(
        "SELECT configuration_generation FROM remote_polling_state",
        [],
        |row| row.get(0),
    ))
}

/// The phase and the checkpoint the remote journal holds for `operation`.
fn standing(w: &World, operation: &str) -> Result<(String, Option<String>), FixtureError> {
    fixed(w.db()?.query_row(
        "SELECT phase, sync_checkpoint FROM remote_operation_records WHERE operation_ulid = ?1",
        [operation],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ))
}

/// The state of the request `id`'s record, when there is one.
fn state_of(w: &World, id: &str) -> Option<RequestState> {
    record(w, id).map(|record| record.state)
}

/// How the deletion of the item's branch is recorded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Recorded {
    /// By a fetch: an earlier synchronization fetched the published branch,
    /// and the refused one found it gone, which marks it
    /// `remotely_deleted`.
    ByFetch,
    /// Only by the push evidence of the publication: no fetch ever saw the
    /// published branch, and the refused synchronization learned of its
    /// absence at the push endpoint.
    ByPush,
}

/// Cancels the refused `item sync` and sends it once more, which ends it
/// cancelled and lets its reservation go.
fn release(w: &World) -> Result<(), FixtureError> {
    let asked = w.service.cancel_request(request(REFUSED));
    assert!(asked.data.ok_or(FixtureError)?.requested);
    let ended = execute(w, REFUSED, item_sync(w));
    assert_eq!(ended.code, ResultCode::Cancelled);
    assert!(!reserved(w));
    Ok(())
}

/// A world whose item's editing branch was published at the commit it
/// returns, then deleted on the remote, with the deletion recorded as
/// `recorded` says. `before` runs before the deletion. The refused `item
/// sync` holds the reservation unless `released`.
pub(super) fn deleted_with(
    recorded: Recorded,
    released: bool,
    before: impl FnOnce(&World, git2::Oid) -> Result<(), FixtureError>,
) -> Result<(World, git2::Oid), FixtureError> {
    let mut w = World::new()?;
    approve(&w)?;
    w.context()?;
    let tip = local(&w)?;
    let published = execute(&w, PUBLISH, item_sync(&w));
    assert_eq!(published.outcome, Outcome::Success);
    assert_eq!(served(&w)?, Some(tip));
    if recorded == Recorded::ByFetch {
        let current = execute(&w, OBSERVE, item_sync(&w));
        assert_eq!(current.code, ResultCode::AlreadyApplied);
        assert_eq!(context_state(&w)?.as_deref(), Some(PRESENT));
    }
    before(&w, tip)?;
    fixed(fixed(w.bare()?.find_reference(CONTEXT))?.delete())?;
    let refused = execute(&w, REFUSED, item_sync(&w));
    assert_eq!(refused.code, ResultCode::RemoteBranchDeleted);
    assert!(reserved(&w));
    assert_eq!(
        context_state(&w)?.as_deref(),
        match recorded {
            Recorded::ByFetch => Some("remotely_deleted"),
            Recorded::ByPush => None,
        }
    );
    if released {
        release(&w)?;
    }
    Ok((w, tip))
}

/// `deleted_with`, with nothing done before the deletion.
pub(super) fn deleted(
    recorded: Recorded,
    released: bool,
) -> Result<(World, git2::Oid), FixtureError> {
    deleted_with(recorded, released, |_, _| Ok(()))
}

/// `prepare` of `mutation`, scanned for the world's private material and
/// checked against the published schemas.
fn prepared_of(w: &World, mutation: &Mutation) -> Envelope<PrepareDataDto> {
    let envelope = w.service.prepare(mutation);
    let rendered = serde_json::to_vec(&envelope).expect("an envelope serializes");
    ssh_privacy::clean(&rendered, &w.probes).expect("redacted envelope");
    ssh_privacy::clean(format!("{envelope:?}").as_bytes(), &w.probes).expect("redacted envelope");
    w.privacy().expect("redacted durable state");
    let value = serde_json::to_value(&envelope).expect("an envelope serializes");
    schema::check_published("envelope.schema.json", &value).expect("the envelope schema");
    if !value["data"].is_null() {
        schema::check_published(PREVIEW_SCHEMA, &value["data"]).expect("the preview schema");
    }
    assert_eq!(envelope.command, "item republish");
    // A preview is of no request and no operation.
    assert_eq!(envelope.request_id, None);
    assert_eq!(envelope.operation_id, None);
    envelope
}

/// `prepare` of the world's republish.
pub(super) fn prepared(w: &World) -> Envelope<PrepareDataDto> {
    prepared_of(w, &republish(w))
}

/// The preview a `prepare` gave.
fn preview_of(envelope: &Envelope<PrepareDataDto>) -> Result<PreviewDto, FixtureError> {
    envelope
        .data
        .clone()
        .and_then(PrepareDataDto::into_preview)
        .ok_or(FixtureError)
}

/// The details a preview shows.
fn details_of(preview: &PreviewDto) -> Result<&ItemRepublishPreviewDto, FixtureError> {
    match &preview.details {
        PreviewDetailsDto::Republish(details) => Ok(details),
        _ => Err(FixtureError),
    }
}

/// The confirmation of a preview of the world's republish, and what the
/// preview shows.
pub(super) fn previewed(w: &World) -> Result<(String, ItemRepublishPreviewDto), FixtureError> {
    let envelope = prepared(w);
    assert_eq!(envelope.code, ResultCode::Ok, "{}", envelope.message);
    let preview = preview_of(&envelope)?;
    Ok((
        preview.confirmation_id.clone(),
        details_of(&preview)?.clone(),
    ))
}

/// Asserts what a preview of the world's republish shows: the branch, the
/// remote, the commit last published there and the local commit that is to
/// be pushed.
fn assert_shows(
    preview: &PreviewDto,
    local: git2::Oid,
    last_published: Option<git2::Oid>,
) -> Result<(), FixtureError> {
    assert_eq!(preview.command, "item republish");
    assert_eq!(
        preview.effects,
        [PreviewEffectDto {
            effect: PreviewEffect::BranchRepublished
        }]
    );
    assert_eq!(preview.scope.item_id.as_deref(), Some(ITEM));
    let details = details_of(preview)?;
    assert_eq!(Some(&details.root), preview.scope.repository.as_ref());
    assert_eq!(details.kind, ItemDtoKind::Ticket);
    assert_eq!(details.item_id, ITEM);
    assert_eq!(details.branch, branch());
    assert_eq!(details.remote, "origin");
    assert_eq!(
        details.last_published_oid,
        last_published.map(|oid| oid.to_string())
    );
    assert_eq!(details.local_oid, local.to_string());
    Ok(())
}

/// Checks a `prepare` envelope against its golden: what differs from run
/// to run is replaced, as `assert_golden` replaces it for `execute`.
fn assert_prepare_golden(
    w: &World,
    name: &str,
    data_schema: Option<&str>,
    envelope: &Envelope<PrepareDataDto>,
) {
    let mut replaced = Vec::new();
    if let Ok(preview) = preview_of(envelope) {
        replaced.push((preview.confirmation_id.clone(), "<confirmation>"));
        replaced.push((preview.expires_at.clone(), "<expires-at>"));
        if let Ok(details) = details_of(&preview) {
            replaced.push((details.local_oid.clone(), "<commit>"));
            if let Some(published) = &details.last_published_oid {
                replaced.push((published.clone(), "<commit>"));
            }
        }
    }
    let replaced: Vec<(&str, &str)> = replaced
        .iter()
        .map(|(found, placeholder)| (found.as_str(), *placeholder))
        .collect();
    let root = w.root.canonicalize().unwrap();
    mutation_golden::assert_contract_at(
        name,
        root.to_str().unwrap(),
        data_schema,
        &replaced,
        &serde_json::to_value(envelope).unwrap(),
    );
}

/// A `prepare` that gives no preview: the no-op, which says nothing is to
/// be republished as things are recorded, with no confirmation, no data and
/// no connection. What to do instead, `item sync` first, is the CLI's to
/// say; no recovery action names a synchronization.
fn assert_noop_preview(w: &World) -> Result<Envelope<PrepareDataDto>, FixtureError> {
    let before = confirmations(w)?;
    let opened = connections(w);
    let envelope = prepared(w);
    assert_eq!(
        envelope.code,
        ResultCode::AlreadyApplied,
        "{}",
        envelope.message
    );
    assert_eq!(envelope.outcome, Outcome::Noop);
    assert!(envelope.data.is_none());
    assert!(envelope.recovery.is_empty());
    assert_eq!(confirmations(w)?, before);
    assert_eq!(connections(w), opened);
    assert!(!reserved(w));
    Ok(envelope)
}

/// One call of `execute` of the world's republish, presenting
/// `confirmation`, scanned and checked as `call` checks a synchronization.
pub(super) fn confirmed_as(
    w: &World,
    id: &str,
    confirmation: Option<&str>,
    provider: impl SessionCredentialProvider,
    progress: &mut dyn ProgressSink,
    cancel: &CancellationToken,
) -> Envelope<MutationDataDto> {
    let envelope = w.service.execute(
        MutationCall {
            request_id: request(id),
            confirmation: confirmation.map(|id| ConfirmationId::parse(id).unwrap()),
            mutation: republish(w),
        },
        &mut SessionCredentials::new(provider),
        progress,
        cancel,
    );
    let rendered = serde_json::to_vec(&envelope).expect("an envelope serializes");
    ssh_privacy::clean(&rendered, &w.probes).expect("redacted envelope");
    ssh_privacy::clean(format!("{envelope:?}").as_bytes(), &w.probes).expect("redacted envelope");
    w.privacy().expect("redacted durable state");
    let value = serde_json::to_value(&envelope).expect("an envelope serializes");
    schema::check_published("envelope.schema.json", &value).expect("the envelope schema");
    // A republish carries data only for a host it stopped for, as a
    // synchronization does.
    if !value["data"].is_null() {
        schema::check_published(SYNCHRONIZATION_SCHEMA, &value["data"])
            .expect("the synchronization data schema");
    }
    assert_eq!(envelope.command, "item republish");
    envelope
}

/// `confirmed_as`, with the world's passphrase, no sink and no
/// cancellation.
pub(super) fn confirmed(w: &World, id: &str, confirmation: &str) -> Envelope<MutationDataDto> {
    confirmed_as(
        w,
        id,
        Some(confirmation),
        Provider,
        &mut NoProgress,
        &CancellationToken::new(),
    )
}

/// Asserts that a republish published the item's branch at `tip` and did
/// nothing else: the remote's branch is `tip`, created by one update of
/// this call, and the local branches are as they were, with no merge.
fn assert_republished(
    w: &World,
    envelope: &Envelope<MutationDataDto>,
    tip: git2::Oid,
    updates: usize,
    primary: git2::Oid,
) -> Result<(), FixtureError> {
    assert_eq!(envelope.code, ResultCode::Ok, "{}", envelope.message);
    assert_eq!(envelope.outcome, Outcome::Success);
    assert_eq!(envelope.effects, published(tip));
    assert!(envelope.data.is_none());
    assert!(envelope.recovery.is_empty());
    assert_eq!(envelope.scope.item_id.as_deref(), Some(ITEM));
    assert_eq!(envelope.scope.branch.as_deref(), Some(branch()));
    // Exactly the local commit, created once.
    one_update(&w.server, updates, CONTEXT, git2::Oid::zero(), tip);
    assert_eq!(served(w)?, Some(tip));
    // Nothing is integrated: both local branches are where they were.
    let repo = w.repo()?;
    assert_eq!(local(w)?, tip);
    assert_eq!(fixed(repo.refname_to_id("refs/heads/main"))?, primary);
    assert_eq!(merge_commits(&repo)?, 0);
    assert_eq!(merge_rows(w)?, 0);
    // The publication is recorded, and the repository is free.
    let operation = envelope.operation_id.clone().ok_or(FixtureError)?;
    assert_eq!(standing(w, &operation)?.0, "completed");
    assert!(!reserved(w));
    assert_eq!(state_of(w, REPUBLISH), Some(RequestState::Finished));
    Ok(())
}

/// The local primary branch's tip.
fn primary(w: &World) -> Result<git2::Oid, FixtureError> {
    fixed(w.repo()?.refname_to_id("refs/heads/main"))
}

/// Republishes the world's deleted branch under `REPUBLISH`, and checks
/// that it published `tip` and nothing else.
fn republished(w: &World, tip: git2::Oid) -> Result<Envelope<MutationDataDto>, FixtureError> {
    let (confirmation, _) = previewed(w)?;
    let updates = w.server.receive_updates().len();
    let main = primary(w)?;
    let envelope = confirmed(w, REPUBLISH, &confirmation);
    assert_republished(w, &envelope, tip, updates, main)?;
    Ok(envelope)
}

// Previews.

/// Row 1: the deletion recorded by a fetch.
fn c4_republish_preview_fetched() -> Result<(), FixtureError> {
    let (w, tip) = deleted(Recorded::ByFetch, true)?;
    let opened = connections(&w);
    let pushes = accepted(&w.server);
    let envelope = prepared(&w);
    assert_eq!(envelope.code, ResultCode::Ok, "{}", envelope.message);
    assert_eq!(envelope.outcome, Outcome::Success);
    assert_shows(&preview_of(&envelope)?, tip, Some(tip))?;
    assert_prepare_golden(
        &w,
        "item_republish_preview",
        Some(PREVIEW_SCHEMA),
        &envelope,
    );
    // A preview reads what is stored: it connects to nothing, reserves
    // nothing and pushes nothing.
    assert_eq!(connections(&w), opened);
    assert_eq!(accepted(&w.server), pushes);
    assert!(!reserved(&w));
    assert_eq!(served(&w)?, None);
    Ok(())
}

/// Row 2: the deletion shown only by push evidence classified `Deleted`.
fn c4_republish_preview_pushed() -> Result<(), FixtureError> {
    let (w, tip) = deleted(Recorded::ByPush, true)?;
    let opened = connections(&w);
    let envelope = prepared(&w);
    assert_eq!(envelope.code, ResultCode::Ok, "{}", envelope.message);
    assert_shows(&preview_of(&envelope)?, tip, Some(tip))?;
    assert_eq!(connections(&w), opened);
    assert!(!reserved(&w));
    Ok(())
}

/// Row 3, `Ambiguous`: the item's only synchronization was cancelled at the
/// safe point before its push. Its push was intended and never confirmed.
fn c4_republish_preview_ambiguous() -> Result<(), FixtureError> {
    let mut w = World::new()?;
    approve(&w)?;
    w.context()?;
    let token = CancellationToken::new();
    let mut sink = CancelAt {
        stage: ProgressStage::Pushing,
        token: token.clone(),
        seen: Vec::new(),
    };
    let cancelled = call(
        &w,
        &w.service,
        PUBLISH,
        item_sync(&w),
        Provider,
        &mut sink,
        &token,
    );
    assert_eq!(cancelled.code, ResultCode::Cancelled);
    let operation = cancelled.operation_id.clone().ok_or(FixtureError)?;
    assert_eq!(
        standing(&w, &operation)?,
        ("cancelled".to_owned(), Some("push_prepared".to_owned()))
    );
    assert_eq!(served(&w)?, None);
    assert_noop_preview(&w)?;
    Ok(())
}

/// Row 3, `Unknown`: the item was published under a configuration the
/// repository has since left. A key selected again raises the generation at
/// the next synchronization, which is of primary.
fn c4_republish_preview_unknown() -> Result<(), FixtureError> {
    let mut w = World::new()?;
    approve(&w)?;
    w.context()?;
    let published = execute(&w, PUBLISH, item_sync(&w));
    assert_eq!(published.outcome, Outcome::Success);
    fixed(fixed(w.bare()?.find_reference(CONTEXT))?.delete())?;
    let generation_before = generation(&w)?;
    select_the_key_again(&w)?;
    let current = execute(&w, OBSERVE, repo_sync(&w));
    assert_eq!(
        current.code,
        ResultCode::AlreadyApplied,
        "{}",
        current.message
    );
    assert!(generation(&w)? > generation_before);
    assert_noop_preview(&w)?;
    Ok(())
}

/// Row 6: no deletion evidence. The item's branch was never published; it
/// is there, as a fetch recorded it; or its deletion is not yet observed.
fn c4_republish_preview_noop() -> Result<(), FixtureError> {
    let mut w = World::new()?;
    approve(&w)?;
    w.context()?;
    // Never published.
    let never = assert_noop_preview(&w)?;
    assert_prepare_golden(&w, "item_republish_preview_noop", None, &never);
    // Published, and fetched since: it is there.
    assert_eq!(
        execute(&w, PUBLISH, item_sync(&w)).outcome,
        Outcome::Success
    );
    assert_eq!(
        execute(&w, OBSERVE, item_sync(&w)).code,
        ResultCode::AlreadyApplied
    );
    assert_eq!(context_state(&w)?.as_deref(), Some(PRESENT));
    assert_noop_preview(&w)?;
    // Deleted on the remote, and no synchronization has looked since.
    fixed(fixed(w.bare()?.find_reference(CONTEXT))?.delete())?;
    assert_eq!(context_state(&w)?.as_deref(), Some(PRESENT));
    assert_noop_preview(&w)?;
    Ok(())
}

/// Row 7: a lifecycle-closed ticket, at `prepare` and at execution with the
/// confirmation of a preview made before it was closed.
fn c4_republish_ticket_closed() -> Result<(), FixtureError> {
    let (w, _) = deleted(Recorded::ByFetch, true)?;
    let (confirmation, _) = previewed(&w)?;
    // Closure fields, committed on the item's branch.
    let path = format!(".manyhands/tickets/{ITEM}/ticket.md");
    let source = fixed(std::fs::read_to_string(worktree(&w).join(&path)))?;
    let closed = source.replacen(
        "\n---\n",
        "\nclosed_at: 2026-10-02T12:34:56Z\nclosed_by: Fixture <fixture@example.invalid>\n---\n",
        1,
    );
    assert_ne!(closed, source);
    commit_in(&worktree(&w), CONTEXT, &path, closed.as_bytes())?;

    let refused = prepared(&w);
    assert_eq!(
        refused.code,
        ResultCode::TicketClosed,
        "{}",
        refused.message
    );
    assert_eq!(refused.outcome, Outcome::Blocked);
    assert!(refused.data.is_none());
    assert_prepare_golden(&w, "item_republish_ticket_closed", None, &refused);

    let pushes = accepted(&w.server);
    let executed = confirmed(&w, REPUBLISH, &confirmation);
    assert_eq!(executed.code, ResultCode::TicketClosed);
    assert_eq!(executed.outcome, Outcome::Blocked);
    assert!(record(&w, REPUBLISH).is_none());
    assert_eq!(accepted(&w.server), pushes);
    assert_eq!(served(&w)?, None);
    assert!(!reserved(&w));
    Ok(())
}

// Confirmation.

/// Row 4: the branch recreated elsewhere between the fetch and the push.
/// The push cannot be guarded as a create (fact 77): the remote refuses it.
fn c4_republish_recreated_before_the_push() -> Result<(), FixtureError> {
    let (w, tip) = deleted(Recorded::ByFetch, true)?;
    let (confirmation, _) = previewed(&w)?;
    let pushes = accepted(&w.server);
    let remote = w.server.repository_path().to_owned();
    let main = fixed(w.bare()?.refname_to_id("refs/heads/main"))?;
    let recreated = Rc::new(Cell::new(None));
    let hook = {
        let recreated = recreated.clone();
        during_the_push(move || {
            let bare = git2::Repository::open_bare(&remote).expect("the remote");
            let elsewhere =
                commit(&bare, main, "recreated-elsewhere", b"elsewhere").expect("a commit");
            bare.reference(CONTEXT, elsewhere, true, "recreated elsewhere")
                .expect("the branch");
            recreated.set(Some(elsewhere));
        })
    };
    let refused = confirmed(&w, REPUBLISH, &confirmation);
    drop(hook);
    assert_eq!(
        refused.code,
        ResultCode::PushRejected,
        "{}",
        refused.message
    );
    assert_eq!(refused.outcome, Outcome::Blocked);
    // Nothing of ours was pushed: the remote holds what was made there.
    let elsewhere = recreated.get().ok_or(FixtureError)?;
    assert_eq!(served(&w)?, Some(elsewhere));
    assert_eq!(accepted(&w.server), pushes);
    assert_eq!(local(&w)?, tip);
    // The operation is over, the repository free, and the consent spent.
    assert!(!reserved(&w));
    assert_eq!(state_of(&w, REPUBLISH), Some(RequestState::Finished));
    Ok(())
}

/// Row 5: the republish stopped after its push returned, with the result
/// lost; then `operation resume` of its operation, which only the republish
/// request goes on with (fact 78). The request then verifies its push.
fn c4_republish_resumed_after_the_push() -> Result<(), FixtureError> {
    let (w, tip) = deleted(Recorded::ByFetch, true)?;
    let (confirmation, _) = previewed(&w)?;
    let pushes = accepted(&w.server);
    w.server.disconnect_at(FixtureBoundary::AfterReceivePack);
    let lost = confirmed(&w, REPUBLISH, &confirmation);
    assert!(w.server.receive_status_withheld());
    assert_eq!(
        lost.code,
        ResultCode::TransportUnavailable,
        "{}",
        lost.message
    );
    w.server.clear_fault();
    assert_eq!(accepted(&w.server), pushes + 1);
    assert_eq!(served(&w)?, Some(tip));
    assert_eq!(state_of(&w, REPUBLISH), Some(RequestState::Accepted));
    assert!(reserved(&w));
    let operation = lost.operation_id.clone().ok_or(FixtureError)?;

    let opened = connections(&w);
    let resumed = execute(&w, RESUMING, resume(&w, &operation));
    assert_eq!(resumed.command, "operation resume");
    assert_eq!(
        resumed.code,
        ResultCode::OriginalRequestRequired,
        "{}",
        resumed.message
    );
    assert_eq!(resumed.outcome, Outcome::Blocked);
    assert_eq!(actions(&resumed), [RecoveryActionKind::RequestRetry]);
    assert_eq!(
        resumed.recovery[0]
            .arguments
            .get("request_id")
            .and_then(|id| id.as_str()),
        Some(REPUBLISH)
    );
    assert!(resumed.data.is_none());
    assert!(record(&w, RESUMING).is_none());
    // Nothing merged or pushed, and nothing asked of the remote.
    assert_eq!(connections(&w), opened);
    assert_eq!(accepted(&w.server), pushes + 1);
    assert_eq!(merge_commits(&w.repo()?)?, 0);
    assert_eq!(merge_rows(&w)?, 0);
    assert_eq!(local(&w)?, tip);
    assert_eq!(state_of(&w, REPUBLISH), Some(RequestState::Accepted));

    // The republish request, sent again, finds its push and pushes nothing.
    let finished = confirmed(&w, REPUBLISH, &confirmation);
    assert_eq!(finished.outcome, Outcome::Success, "{}", finished.message);
    assert_eq!(finished.effects, published(tip));
    assert_eq!(finished.operation_id, lost.operation_id);
    assert_eq!(accepted(&w.server), pushes + 1);
    assert_eq!(merge_rows(&w)?, 0);
    assert!(!reserved(&w));
    Ok(())
}

/// Row 8: nothing changed since the preview.
fn c4_republish_confirmed() -> Result<(), FixtureError> {
    use ProgressStage as S;
    let (w, tip) = deleted(Recorded::ByFetch, true)?;
    let (confirmation, _) = previewed(&w)?;
    let updates = w.server.receive_updates().len();
    let main = primary(&w)?;
    let mut events = Events::default();
    let envelope = confirmed_as(
        &w,
        REPUBLISH,
        Some(&confirmation),
        Provider,
        &mut events,
        &CancellationToken::new(),
    );
    assert_republished(&w, &envelope, tip, updates, main)?;
    assert_golden(&w, "item_republish", None, &envelope);
    // Its events are its own, at the safe points of a push-only path: it
    // never updates a local branch.
    let seen = stages(&events.0);
    assert_eq!(seen[..3], [S::Accepted, S::Connecting, S::Fetching]);
    assert!(seen.contains(&S::Pushing), "{seen:?}");
    assert!(!seen.contains(&S::UpdatingLocal), "{seen:?}");
    for event in &events.0 {
        assert_eq!(event.request_id, REPUBLISH);
        assert_eq!(event.command, "item republish");
        assert_eq!(event.operation_id, envelope.operation_id);
        let rendered = serde_json::to_value(event).expect("an event serializes");
        schema::check_published("progress_event.schema.json", &rendered)
            .expect("the progress event schema");
    }
    Ok(())
}

/// Row 9: remote primary moved since the item's branch last integrated it.
/// Exactly the local commit is pushed; primary is not merged.
fn c4_republish_primary_moved() -> Result<(), FixtureError> {
    let moved = Rc::new(Cell::new(None));
    let (w, tip) = {
        let moved = moved.clone();
        deleted_with(Recorded::ByFetch, true, move |w, _| {
            let advanced = advance(&w.peer, "refs/heads/main", "peer-primary")?;
            push_peer(&w.peer, &w.server, "refs/heads/main")?;
            moved.set(Some(advanced));
            Ok(())
        })?
    };
    let advanced = moved.get().ok_or(FixtureError)?;
    assert_eq!(fixed(w.bare()?.refname_to_id("refs/heads/main"))?, advanced);
    republished(&w, tip)?;
    assert!(!fixed(w.bare()?.graph_descendant_of(tip, advanced))?);
    Ok(())
}

/// Row 10: the item's branch is an ancestor of remote primary. Exactly the
/// local commit is pushed, not primary's tip.
fn c4_republish_ancestor_of_primary() -> Result<(), FixtureError> {
    let built = Rc::new(Cell::new(None));
    let (w, tip) = {
        let built = built.clone();
        deleted_with(Recorded::ByFetch, true, move |w, tip| {
            let bare = w.bare()?;
            let on_the_item = commit(&bare, tip, "built-on-the-item", b"built")?;
            fixed(bare.reference("refs/heads/main", on_the_item, true, "fixture"))?;
            built.set(Some(on_the_item));
            Ok(())
        })?
    };
    let on_the_item = built.get().ok_or(FixtureError)?;
    assert!(fixed(w.bare()?.graph_descendant_of(on_the_item, tip))?);
    republished(&w, tip)?;
    assert_ne!(served(&w)?, Some(on_the_item));
    Ok(())
}

/// Row 11: the local branch moved after the preview. The request is refused
/// before it is accepted, and a new preview is offered.
fn c4_republish_local_moved() -> Result<(), FixtureError> {
    let (w, tip) = deleted(Recorded::ByFetch, true)?;
    let (confirmation, _) = previewed(&w)?;
    let moved = commit_in(&worktree(&w), CONTEXT, "notes.md", b"moved\n")?;
    assert_ne!(moved, tip);
    let opened = connections(&w);
    let pushes = accepted(&w.server);
    let refused = confirmed(&w, REPUBLISH, &confirmation);
    assert_eq!(
        refused.code,
        ResultCode::ExternalChange,
        "{}",
        refused.message
    );
    assert_eq!(refused.outcome, Outcome::Blocked);
    assert_eq!(actions(&refused), [RecoveryActionKind::RequestPrepare]);
    assert!(record(&w, REPUBLISH).is_none());
    assert_eq!(connections(&w), opened);
    assert_eq!(accepted(&w.server), pushes);
    assert_eq!(served(&w)?, None);
    assert!(!reserved(&w));
    Ok(())
}

/// Row 12: the branch reappeared on the remote after the preview. The
/// fetch finds it, nothing is pushed, and the request is finished: a new
/// preview is offered, and shows nothing to republish.
fn c4_republish_reappeared() -> Result<(), FixtureError> {
    let (w, tip) = deleted(Recorded::ByFetch, true)?;
    let (confirmation, _) = previewed(&w)?;
    let bare = w.bare()?;
    let again = commit(&bare, tip, "pushed-again", b"again")?;
    fixed(bare.reference(CONTEXT, again, true, "reappeared"))?;
    let pushes = accepted(&w.server);
    let refused = confirmed(&w, REPUBLISH, &confirmation);
    assert_eq!(
        refused.code,
        ResultCode::ExternalChange,
        "{}",
        refused.message
    );
    assert_eq!(refused.outcome, Outcome::Blocked);
    assert_eq!(actions(&refused), [RecoveryActionKind::RequestPrepare]);
    assert_golden(&w, "item_republish_external_change", None, &refused);
    assert_eq!(accepted(&w.server), pushes);
    assert_eq!(served(&w)?, Some(again));
    assert_eq!(local(&w)?, tip);
    // The operation is over and the request finished.
    let operation = refused.operation_id.clone().ok_or(FixtureError)?;
    assert_ne!(standing(&w, &operation)?.0, "push_prepared");
    assert!(!reserved(&w));
    assert_eq!(state_of(&w, REPUBLISH), Some(RequestState::Finished));
    assert_eq!(confirmed(&w, REPUBLISH, &confirmation), refused);
    // What the fetch found is what a new preview reads.
    assert_eq!(context_state(&w)?.as_deref(), Some(PRESENT));
    assert_noop_preview(&w)?;
    Ok(())
}

/// Row 13: the refused `item sync` still holds its reservation. The
/// republish is `busy` and leaves nothing; once that request is cancelled
/// and sent once more, the same republish request goes on, as C2's bounded
/// retry sends it.
fn c4_republish_busy() -> Result<(), FixtureError> {
    let (w, tip) = deleted(Recorded::ByFetch, false)?;
    let (confirmation, _) = previewed(&w)?;
    let pushes = accepted(&w.server);
    let busy = confirmed(&w, REPUBLISH, &confirmation);
    assert_eq!(busy.code, ResultCode::Busy, "{}", busy.message);
    assert_eq!(busy.outcome, Outcome::Error);
    assert_eq!(actions(&busy), [RecoveryActionKind::RequestRetry]);
    assert!(record(&w, REPUBLISH).is_none());
    assert_eq!(accepted(&w.server), pushes);
    assert_eq!(served(&w)?, None);
    // The reservation is the refused synchronization's.
    assert!(reserved(&w));
    assert_eq!(state_of(&w, REFUSED), Some(RequestState::Accepted));

    release(&w)?;
    let updates = w.server.receive_updates().len();
    let main = primary(&w)?;
    let retried = confirmed(&w, REPUBLISH, &confirmation);
    assert_republished(&w, &retried, tip, updates, main)?;
    Ok(())
}

/// Row 14: a local-context synchronization in the repository is unfinished
/// (fact 80, R-38): the reservation is refused before anything is pushed.
fn c4_republish_local_sync_unfinished() -> Result<(), FixtureError> {
    let (w, _) = deleted(Recorded::ByFetch, true)?;
    let (confirmation, _) = previewed(&w)?;
    leave_local_sync_conflict(&w);
    let pushes = accepted(&w.server);
    let refused = confirmed(&w, REPUBLISH, &confirmation);
    assert_eq!(
        refused.code,
        ResultCode::RecoveryRequired,
        "{}",
        refused.message
    );
    assert_eq!(refused.outcome, Outcome::Blocked);
    assert_eq!(actions(&refused), [RecoveryActionKind::RepoInspect]);
    assert!(record(&w, REPUBLISH).is_none());
    assert_eq!(accepted(&w.server), pushes);
    assert_eq!(served(&w)?, None);
    assert!(!reserved(&w));
    Ok(())
}

/// Row 15: a stop the same request continues from, once its cause is dealt
/// with. `stop` makes the first call, which stops as `code` with nothing
/// pushed; `deal` deals with the cause.
fn continued_after(
    code: ResultCode,
    stop: impl FnOnce(&World, &str) -> Envelope<MutationDataDto>,
    deal: impl FnOnce(&World) -> Result<(), FixtureError>,
) -> Result<Envelope<MutationDataDto>, FixtureError> {
    let (w, tip) = deleted(Recorded::ByFetch, true)?;
    let (confirmation, _) = previewed(&w)?;
    let pushes = accepted(&w.server);
    let stopped = stop(&w, &confirmation);
    assert_eq!(stopped.code, code, "{}", stopped.message);
    assert_eq!(accepted(&w.server), pushes);
    assert_eq!(served(&w)?, None);
    // The same request continues: its record and its reservation stay.
    assert_eq!(state_of(&w, REPUBLISH), Some(RequestState::Accepted));
    assert!(reserved(&w));
    assert_eq!(
        actions(&stopped).last(),
        Some(&RecoveryActionKind::RequestRetry)
    );
    deal(&w)?;
    let updates = w.server.receive_updates().len();
    let main = primary(&w)?;
    let continued = confirmed(&w, REPUBLISH, &confirmation);
    assert_eq!(continued.operation_id, stopped.operation_id);
    assert_republished(&w, &continued, tip, updates, main)?;
    Ok(stopped)
}

/// Row 15, a transport stop: the server drops the connection at its
/// advertisement.
fn c4_republish_transport_stop() -> Result<(), FixtureError> {
    let stopped = continued_after(
        ResultCode::TransportUnavailable,
        |w, confirmation| {
            w.server.disconnect_at(FixtureBoundary::Advertisement);
            confirmed(w, REPUBLISH, confirmation)
        },
        |w| {
            w.server.clear_fault();
            Ok(())
        },
    )?;
    assert_eq!(stopped.outcome, Outcome::Error);
    Ok(())
}

/// Row 15, an unlock stop: a caller with no passphrase to give.
fn c4_republish_unlock_stop() -> Result<(), FixtureError> {
    let stopped = continued_after(
        ResultCode::UnlockRequired,
        |w, confirmation| {
            confirmed_as(
                w,
                REPUBLISH,
                Some(confirmation),
                NoPassphrase,
                &mut NoProgress,
                &CancellationToken::new(),
            )
        },
        |_| Ok(()),
    )?;
    assert_eq!(stopped.outcome, Outcome::Blocked);
    assert!(stopped.data.is_none());
    Ok(())
}

/// Row 15, a host stop: the server presents another key than its pin. The
/// result names the host, as a synchronization's does, and the replacement
/// that deals with it.
fn c4_republish_host_stop() -> Result<(), FixtureError> {
    let stopped = continued_after(
        ResultCode::HostReplacementRequired,
        |w, confirmation| {
            w.server.rotate_host_key().expect("a new host key");
            confirmed(w, REPUBLISH, confirmation)
        },
        |w| {
            fixed(w.service.approve_host_pin_for_testing(
                &SshAuthority {
                    host: "127.0.0.1".into(),
                    port: w.server.address().port(),
                },
                &w.server.host_identity(),
            ))
        },
    )?;
    assert_eq!(stopped.outcome, Outcome::Blocked);
    assert!(matches!(
        stopped.data,
        Some(MutationDataDto::Synchronization(_))
    ));
    assert_eq!(
        actions(&stopped),
        [
            RecoveryActionKind::HostReplace,
            RecoveryActionKind::RequestRetry
        ]
    );
    Ok(())
}

/// Row 16: a confirmed request is replayed from its record, whatever it
/// presents, with no connection and no second push.
fn c4_republish_replayed() -> Result<(), FixtureError> {
    let (w, tip) = deleted(Recorded::ByFetch, true)?;
    let (confirmation, _) = previewed(&w)?;
    let first = confirmed(&w, REPUBLISH, &confirmation);
    assert_eq!(first.effects, published(tip), "{}", first.message);
    let opened = connections(&w);
    let pushes = accepted(&w.server);
    assert_eq!(confirmed(&w, REPUBLISH, &confirmation), first);
    let unconsented = confirmed_as(
        &w,
        REPUBLISH,
        None,
        Provider,
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(unconsented, first);
    assert_eq!(connections(&w), opened);
    assert_eq!(accepted(&w.server), pushes);
    Ok(())
}

/// Row 17: `item sync` after the republish, with primary unchanged, finds
/// the remote current.
fn c4_republish_item_sync_after() -> Result<(), FixtureError> {
    let (w, tip) = deleted(Recorded::ByFetch, true)?;
    republished(&w, tip)?;
    let pushes = accepted(&w.server);
    let after = execute(&w, AFTER, item_sync(&w));
    assert_eq!(after.code, ResultCode::AlreadyApplied, "{}", after.message);
    assert_eq!(after.effects.publication, PublicationEffect::Current);
    assert_eq!(accepted(&w.server), pushes);
    assert_eq!(merge_rows(&w)?, 0);
    assert_eq!(local(&w)?, tip);
    Ok(())
}

/// Row 18: a Ctrl-C during execution, from the call's own token at the
/// first safe point. The operation is final (fact 56): a new prepare is
/// needed, since the request accepted its confirmation.
fn c4_republish_cancelled() -> Result<(), FixtureError> {
    let (w, tip) = deleted(Recorded::ByFetch, true)?;
    let (confirmation, _) = previewed(&w)?;
    let pushes = accepted(&w.server);
    let token = CancellationToken::new();
    let mut sink = CancelAt {
        stage: ProgressStage::Fetching,
        token: token.clone(),
        seen: Vec::new(),
    };
    let cancelled = confirmed_as(
        &w,
        REPUBLISH,
        Some(&confirmation),
        Provider,
        &mut sink,
        &token,
    );
    assert_eq!(
        cancelled.code,
        ResultCode::Cancelled,
        "{}",
        cancelled.message
    );
    assert_eq!(cancelled.outcome, Outcome::Cancelled);
    assert!(cancelled.recovery.is_empty());
    assert_eq!(accepted(&w.server), pushes);
    assert_eq!(served(&w)?, None);
    assert!(!reserved(&w));
    assert_eq!(state_of(&w, REPUBLISH), Some(RequestState::Finished));
    assert_eq!(confirmed(&w, REPUBLISH, &confirmation), cancelled);
    // Its confirmation is spent.
    let used = confirmed(&w, ANOTHER, &confirmation);
    assert_eq!(used.code, ResultCode::ConfirmationUsed);
    assert!(record(&w, ANOTHER).is_none());
    // A new preview is what goes on.
    let (fresh, _) = previewed(&w)?;
    let updates = w.server.receive_updates().len();
    let republished = confirmed(&w, ANOTHER, &fresh);
    assert_eq!(
        republished.outcome,
        Outcome::Success,
        "{}",
        republished.message
    );
    one_update(&w.server, updates, CONTEXT, git2::Oid::zero(), tip);
    Ok(())
}

/// Row 19, as the library has it: a cancellation before the request is
/// accepted (the CLI's consent prompt is Task 12b's). Nothing is recorded or
/// reserved, and the confirmation is still there to use.
fn c4_republish_cancelled_before_execution() -> Result<(), FixtureError> {
    let (w, tip) = deleted(Recorded::ByFetch, true)?;
    let (confirmation, _) = previewed(&w)?;
    let opened = connections(&w);
    let token = CancellationToken::new();
    token.cancel();
    let cancelled = confirmed_as(
        &w,
        REPUBLISH,
        Some(&confirmation),
        Provider,
        &mut NoProgress,
        &token,
    );
    assert_eq!(cancelled.code, ResultCode::Cancelled);
    assert_eq!(cancelled.outcome, Outcome::Cancelled);
    assert_eq!(cancelled.operation_id, None);
    assert!(record(&w, REPUBLISH).is_none());
    assert!(!reserved(&w));
    assert_eq!(connections(&w), opened);
    let updates = w.server.receive_updates().len();
    let main = primary(&w)?;
    let republished = confirmed(&w, REPUBLISH, &confirmation);
    assert_republished(&w, &republished, tip, updates, main)?;
    Ok(())
}

// The conformance row.

/// A world whose item's branch was deleted, with `rule` driven through
/// republish requests. `Ask::Same` and `Ask::Changed` present the
/// confirmation of one preview; the request that is refused presents a
/// confirmation no preview gave, and the one with the wrong consent presents
/// none. The preview is made stale by another commit on the item's branch.
pub(super) fn conforming(rule: &mut dyn FnMut(&conformance::Subject<'_>)) -> Result<(), String> {
    use conformance::{Ask, NO_SUCH_CONFIRMATION, Recorded as Held, Subject};
    let (w, _) = deleted(Recorded::ByPush, true)
        .map_err(|_| "a world whose item's branch was deleted".to_owned())?;
    let confirmation = OnceCell::new();
    let preview = || -> String {
        confirmation
            .get_or_init(|| previewed(&w).expect("a preview").0)
            .clone()
    };
    rule(&Subject {
        outdate: Some(&|| {
            preview();
            commit_in(&worktree(&w), CONTEXT, "outdated.md", b"outdated\n")
                .expect("another commit on the item's branch");
        }),
        call: &|id, ask| {
            let presented = match ask {
                Ask::Same | Ask::Changed => Some(preview()),
                Ask::Refused => Some(NO_SUCH_CONFIRMATION.to_owned()),
                Ask::Unconsented => None,
            };
            let mutation = match ask {
                // Another item: a changed input under a finished request's
                // ID.
                Ask::Changed => republish_of(&w, OTHER_ITEM),
                Ask::Same | Ask::Refused | Ask::Unconsented => republish(&w),
            };
            let envelope = presenting(&w, id, presented.as_deref(), mutation);
            serde_json::to_vec(&envelope).expect("an envelope serializes")
        },
        record: &|id| {
            record(&w, id).map(|record| Held {
                finished: record.state == RequestState::Finished,
                image: format!("{record:?}"),
            })
        },
        request_rows: &|| {
            w.db()
                .expect("the database")
                .query_row("SELECT COUNT(*) FROM request_records", [], |row| row.get(0))
                .expect("the request records")
        },
    });
    Ok(())
}

/// The row of `item republish`. The rows of the other commands of this
/// target are driven by their own cases.
fn c4_republish_conformance() -> Result<(), FixtureError> {
    conformance_row("item republish")
}
