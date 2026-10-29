//! `comment add` through the boundary: the design's "Required outcomes:
//! comment submission" table, one `c4_comment_` case per row, and the
//! cases of Review Focus 1 and 5.
//!
//! Every call goes through `RepositoryService::execute`, and every
//! envelope and store is scanned for the world's private material, as
//! `call` scans them. The comment's body is one of the world's probes, so
//! no envelope, request record or journal row may hold it.
//!
//! A row the library answers alike for a person and for a front end is
//! asserted here. What only a front end decides is not: an exit status,
//! C2's bounded retry of a `busy` or a partial result, the human output,
//! and what a Ctrl-C is mapped to. An exit status is asserted as the
//! outcome and failure class it is made from. A Ctrl-C reaches the
//! library as the call's cancellation token, and the rows that name one
//! are asserted through it.

use std::path::Path;

use results::{CheckpointEffect, FailureClass, IntegrationEffect, WriteEffect};

use super::*;

pub(crate) const CASES: &[ssh_harness::Case] = &[
    ("c4_comment_publishes", c4_comment_publishes),
    (
        "c4_comment_no_publication_remote",
        c4_comment_no_publication_remote,
    ),
    ("c4_comment_push_refused", c4_comment_push_refused),
    (
        "c4_comment_retried_after_the_refusal",
        c4_comment_retried_after_the_refusal,
    ),
    (
        "c4_comment_canonical_conflict",
        c4_comment_canonical_conflict,
    ),
    (
        "c4_comment_another_item_s_conflict",
        c4_comment_another_item_s_conflict,
    ),
    (
        "c4_comment_this_item_s_conflict",
        c4_comment_this_item_s_conflict,
    ),
    (
        "c4_comment_unfinished_local_sync",
        c4_comment_unfinished_local_sync,
    ),
    ("c4_comment_ticket_closed", c4_comment_ticket_closed),
    ("c4_comment_identity_required", c4_comment_identity_required),
    (
        "c4_comment_identity_required_with_no_context",
        c4_comment_identity_required_with_no_context,
    ),
    (
        "c4_comment_identity_required_with_an_author",
        c4_comment_identity_required_with_an_author,
    ),
    (
        "c4_comment_checkpoint_lands_before_the_retry",
        c4_comment_checkpoint_lands_before_the_retry,
    ),
    (
        "c4_comment_identity_required_after_a_write",
        c4_comment_identity_required_after_a_write,
    ),
    (
        "c4_comment_remote_context_deleted",
        c4_comment_remote_context_deleted,
    ),
    ("c4_comment_malformed_author", c4_comment_malformed_author),
    ("c4_comment_unknown_parent", c4_comment_unknown_parent),
    ("c4_comment_self_parent", c4_comment_self_parent),
    (
        "c4_comment_comment_id_occupied",
        c4_comment_comment_id_occupied,
    ),
    (
        "c4_comment_cancelled_before_the_push",
        c4_comment_cancelled_before_the_push,
    ),
    (
        "c4_comment_cancelled_after_the_push_returned",
        c4_comment_cancelled_after_the_push_returned,
    ),
    (
        "c4_comment_unsettled_push_then_cancelled",
        c4_comment_unsettled_push_then_cancelled,
    ),
    (
        "c4_comment_remote_added_later",
        c4_comment_remote_added_later,
    ),
    ("c4_comment_lost_output", c4_comment_lost_output),
    (
        "c4_comment_merges_a_divergent_context",
        c4_comment_merges_a_divergent_context,
    ),
    ("c4_comment_body_round_trips", c4_comment_body_round_trips),
    ("c4_comment_request_mismatch", c4_comment_request_mismatch),
];

/// The comment a case adds to the world's item.
const COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC1";
/// A comment no fixture holds.
const NO_SUCH_COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC2";
/// Comments a case adds besides `COMMENT`.
const SECOND_COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC3";
const THIRD_COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC4";
/// A lifecycle-closed ticket a case commits on primary.
const CLOSED_TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD1";
/// A ticket a case commits on primary, with no editing context.
const NEW_TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD2";
/// The request a case publishes the item's editing branch with first.
const SETUP: &str = "01ARZ3NDEKTSV4RRFFQ69G5FY0";
const REQUEST_5: &str = "01ARZ3NDEKTSV4RRFFQ69G5FY5";
/// The comment's body: a probe, which no store or envelope may hold.
const COMMENT_BODY: &str = "comment-add-private-body-sentinel\n";
/// An author a request gives.
const AUTHOR: &str = "Comment Author <comment-author@example.invalid>";
/// The author the world's identity writes.
const WORLD_AUTHOR: &str = "Fixture <fixture@example.invalid>";
const COMMENT_SCHEMA: &str = "comment_mutation.schema.json";

/// A world with a trusted server and the item's editing context. Its
/// branch is not published yet. The comment's body is one of its probes.
fn world() -> Result<(World, ItemContext), FixtureError> {
    let mut w = World::new()?;
    approve(&w)?;
    let context = w.context()?;
    w.probes.push(COMMENT_BODY.trim_end().as_bytes().to_vec());
    w.privacy()?;
    Ok((w, context))
}

/// `world`, with the item's editing branch published.
fn published_world() -> Result<(World, ItemContext), FixtureError> {
    let (w, context) = world()?;
    assert_eq!(execute(&w, SETUP, item_sync(&w)).code, ResultCode::Ok);
    Ok((w, context))
}

/// `comment add` of `comment` on `item`.
fn comment_on(
    w: &World,
    item: &str,
    comment: &str,
    parent: Option<&str>,
    body: &str,
    created_by: Option<&str>,
) -> Mutation {
    Mutation::CommentAdd(CommentAddInput {
        root: w.root.clone(),
        kind: AuthoringKind::Ticket,
        id: item.parse().unwrap(),
        comment_id: comment.parse().unwrap(),
        parent_id: parent.map(|parent| parent.parse().unwrap()),
        body: body.to_owned(),
        created_by: created_by.map(str::to_owned),
    })
}

/// `comment add` of `COMMENT` on the world's item.
fn add(w: &World) -> Mutation {
    comment_on(w, ITEM, COMMENT, None, COMMENT_BODY, None)
}

/// The envelope, with its data checked against the published schema.
fn checked(envelope: Envelope<MutationDataDto>) -> Envelope<MutationDataDto> {
    let value = serde_json::to_value(&envelope).expect("an envelope serializes");
    if !value["data"].is_null() {
        schema::check_published(COMMENT_SCHEMA, &value["data"]).expect("the comment data schema");
    }
    envelope
}

/// One call of `comment add` with the world's passphrase, no sink and no
/// cancellation, scanned as `call` scans.
fn added(w: &World, id: &str, mutation: Mutation) -> Envelope<MutationDataDto> {
    checked(execute(w, id, mutation))
}

fn data(envelope: &Envelope<MutationDataDto>) -> Result<&CommentMutationDto, FixtureError> {
    match envelope.data.as_ref() {
        Some(MutationDataDto::Comment(data)) => Ok(data),
        _ => Err(FixtureError),
    }
}

/// The comment's file, relative to its item's editing worktree.
fn comment_path(item: &str, comment: &str) -> String {
    format!(".manyhands/comments/{item}/{comment}.md")
}

/// The one commit on the item's editing branch that checkpoints
/// `comment`.
fn checkpoint_of(w: &World, reference: &str, comment: &str) -> Result<git2::Oid, FixtureError> {
    match comment_domain::checkpoints(w, reference, &fixed(comment.parse())?)?.as_slice() {
        [checkpoint] => Ok(*checkpoint),
        _ => Err(FixtureError),
    }
}

/// What a request that wrote and checkpointed its comment reports, with
/// the publication `publication`.
fn saved(checkpoint: git2::Oid, publication: PublicationEffect) -> Effects {
    Effects {
        write: WriteEffect::Written,
        checkpoint: CheckpointEffect::Committed,
        discovery: DiscoveryEffect::Current,
        publication,
        commit_oid: Some(checkpoint.to_string()),
        ..Effects::not_requested()
    }
}

/// Whether the tip of `reference` holds `path`, in `repo`.
fn holds(repo: &git2::Repository, reference: &str, path: &str) -> Result<bool, FixtureError> {
    let Ok(tip) = repo.refname_to_id(reference) else {
        return Ok(false);
    };
    Ok(fixed(fixed(repo.find_commit(tip))?.tree())?
        .get_path(Path::new(path))
        .is_ok())
}

/// Whether the server's copy of the item's editing branch holds the
/// comment.
fn served(w: &World, comment: &str) -> Result<bool, FixtureError> {
    holds(&w.bare()?, CONTEXT, &comment_path(ITEM, comment))
}

/// How many publication bindings the registry holds.
fn binding_rows(w: &World) -> Result<i64, FixtureError> {
    fixed(w.db()?.query_row(
        "SELECT COUNT(*) FROM comment_publication_bindings",
        [],
        |row| row.get(0),
    ))
}

fn operation_of(envelope: &Envelope<MutationDataDto>) -> Result<OperationId, FixtureError> {
    OperationId::parse(envelope.operation_id.as_deref().ok_or(FixtureError)?)
        .map_err(|_| FixtureError)
}

fn state(w: &World, id: &str) -> Option<RequestState> {
    record(w, id).map(|record| record.state)
}

/// The comment's front matter `created_by`, as its file in the item's
/// editing worktree holds it.
fn written_author(context: &ItemContext, comment: &str) -> Result<Option<String>, FixtureError> {
    let path = comment_path(ITEM, comment);
    let source = fixed(std::fs::read_to_string(context.worktree.join(&path)))?;
    match fixed(canonical::parse_item(Path::new(&path), &source))? {
        canonical::CanonicalItem::Comment(comment) => Ok(comment.created_by),
        _ => Err(FixtureError),
    }
}

/// Checks `envelope` against its golden fixture, with the child
/// synchronization's ID given a placeholder of its own.
fn assert_comment_golden(w: &World, name: &str, envelope: &Envelope<MutationDataDto>) {
    let value = serde_json::to_value(envelope).unwrap();
    let synchronization = value
        .pointer("/data/synchronization_id")
        .and_then(|found| found.as_str())
        .map(str::to_owned);
    let replaced: Vec<(&str, &str)> = synchronization
        .as_deref()
        .map(|found| (found, "<synchronization>"))
        .into_iter()
        .collect();
    let root = w.root.canonicalize().unwrap();
    let data_schema = (!value["data"].is_null()).then_some(COMMENT_SCHEMA);
    mutation_golden::assert_contract_at(
        name,
        root.to_str().unwrap(),
        data_schema,
        &replaced,
        &value,
    );
}

/// Has the server refuse every push to the item's editing branch by its
/// own receive configuration, as it refuses pushes to primary: the branch
/// is made the one the server's repository has checked out.
fn refuse_pushes(w: &World) -> Result<(), FixtureError> {
    fixed(w.bare()?.set_head(CONTEXT))?;
    w.server.reject_primary_updates(true)
}

/// Has the server accept pushes to the item's editing branch again.
fn accept_pushes(w: &World) -> Result<(), FixtureError> {
    w.server.reject_primary_updates(false)?;
    fixed(w.bare()?.set_head("refs/heads/main"))
}

/// Gives the item's ticket another title on its editing branch, and the
/// peer another one again on the server's copy of the branch: a
/// conflict in canonical content, with all three sides.
fn diverge_on_the_ticket(
    w: &World,
    worktree: &Path,
    item: &str,
    reference: &str,
) -> Result<(), FixtureError> {
    let path = merging::ticket_path(item);
    commit_in(
        worktree,
        reference,
        &path,
        merging::ticket_source(item, "Local").as_bytes(),
    )?;
    merging::fetch_into_peer(w, reference)?;
    let parent = fixed(w.peer.refname_to_id(reference))?;
    let incoming = commit_path(
        &w.peer,
        parent,
        &path,
        merging::ticket_source(item, "Incoming").as_bytes(),
    )?;
    fixed(w.peer.reference(reference, incoming, true, "fixture"))?;
    push_peer(&w.peer, &w.server, reference)
}

const CONFIG: &str = ".manyhands/config.toml";
const REMOTE_LINE: &str = "publication_remote = \"origin\"\n";

/// Removes the publication remote from the repository's configuration,
/// in a commit on primary, which is left clean.
fn remove_the_remote(w: &World) -> Result<(), FixtureError> {
    let source = fixed(std::fs::read_to_string(w.root.join(CONFIG)))?;
    commit_in(
        &w.root,
        "refs/heads/main",
        CONFIG,
        source.replace(REMOTE_LINE, "").as_bytes(),
    )?;
    Ok(())
}

/// Gives the repository its publication remote again, as above.
fn restore_the_remote(w: &World) -> Result<(), FixtureError> {
    let mut source = fixed(std::fs::read_to_string(w.root.join(CONFIG)))?;
    source.push_str(REMOTE_LINE);
    commit_in(&w.root, "refs/heads/main", CONFIG, source.as_bytes())?;
    Ok(())
}

/// Asserts that `envelope` stopped having done nothing: no effect, no
/// data, no record, no binding and no comment file.
fn assert_nothing(
    w: &World,
    context: &ItemContext,
    id: &str,
    envelope: &Envelope<MutationDataDto>,
) -> Result<(), FixtureError> {
    assert_eq!(envelope.effects, Effects::not_requested());
    assert!(envelope.data.is_none());
    assert!(record(w, id).is_none());
    assert_eq!(binding_rows(w)?, 0);
    assert!(!context.worktree.join(comment_path(ITEM, COMMENT)).exists());
    Ok(())
}

fn c4_comment_publishes() -> Result<(), FixtureError> {
    let (w, context) = world()?;
    let pushes = accepted(&w.server);
    let published = added(&w, REQUEST_1, add(&w));
    assert_eq!(published.command, "comment add");
    assert_eq!(published.code, ResultCode::Ok);
    assert_eq!(published.outcome, Outcome::Success);
    let checkpoint = checkpoint_of(&w, CONTEXT, COMMENT)?;
    assert_eq!(
        published.effects,
        saved(checkpoint, PublicationEffect::Published)
    );
    assert!(published.recovery.is_empty());
    let data = data(&published)?;
    assert_eq!(data.r#type, CommentMutationType::Comment);
    assert_eq!(data.item_id, ITEM);
    assert_eq!(data.comment_id, COMMENT);
    assert_eq!(data.parent_id, None);
    assert_eq!(data.created_by.as_deref(), Some(WORLD_AUTHOR));
    assert_eq!(data.publication, PublicationEffect::Published);
    assert_eq!(data.pending_reason, None);
    assert_eq!(data.conflict, None);
    // The child synchronization is the comment's own, and not its
    // operation.
    let child = OperationId::parse(&data.synchronization_id).map_err(|_| FixtureError)?;
    let operation = operation_of(&published)?;
    assert_ne!(child, operation);
    assert_eq!(
        comment_domain::bindings(&w, operation)?,
        [child.to_string()]
    );
    assert_eq!(phase(&w, child)?, "completed");
    // The server holds the comment, as the branch does.
    assert!(served(&w, COMMENT)?);
    assert_eq!(
        fixed(w.bare()?.refname_to_id(CONTEXT))?,
        fixed(w.repo()?.refname_to_id(CONTEXT))?
    );
    assert_eq!(accepted(&w.server), pushes + 1);
    assert_eq!(
        written_author(&context, COMMENT)?.as_deref(),
        Some(WORLD_AUTHOR)
    );
    assert_eq!(state(&w, REQUEST_1), Some(RequestState::Finished));
    assert!(!reserved(&w));
    assert_comment_golden(&w, "comment_add", &published);
    Ok(())
}

fn c4_comment_no_publication_remote() -> Result<(), FixtureError> {
    let (w, _) = world()?;
    remove_the_remote(&w)?;
    let opened = connections(&w);
    let saved_only = added(&w, REQUEST_1, add(&w));
    assert_eq!(saved_only.code, ResultCode::Ok);
    assert_eq!(saved_only.outcome, Outcome::Success);
    let checkpoint = checkpoint_of(&w, CONTEXT, COMMENT)?;
    assert_eq!(
        saved_only.effects,
        saved(checkpoint, PublicationEffect::Pending)
    );
    let data = data(&saved_only)?;
    assert_eq!(data.publication, PublicationEffect::Pending);
    assert_eq!(
        data.pending_reason,
        Some(CommentPendingReason::NoPublicationRemote)
    );
    assert_eq!(data.conflict, None);
    assert!(saved_only.recovery.is_empty());
    assert_eq!(state(&w, REQUEST_1), Some(RequestState::Finished));
    assert_eq!(connections(&w), opened);
    assert!(w.bare()?.find_reference(CONTEXT).is_err());
    Ok(())
}

/// The comment is checkpointed and its publication refused by the
/// server, which the first case of the pair below asserts.
fn refused_by_the_server() -> Result<(World, ItemContext, Envelope<MutationDataDto>), FixtureError>
{
    let (w, context) = published_world()?;
    refuse_pushes(&w)?;
    let refused = added(&w, REQUEST_1, add(&w));
    Ok((w, context, refused))
}

fn c4_comment_push_refused() -> Result<(), FixtureError> {
    let (w, _, refused) = refused_by_the_server()?;
    assert_eq!(refused.code, ResultCode::PushRejected);
    assert_eq!(refused.outcome, Outcome::Partial);
    let checkpoint = checkpoint_of(&w, CONTEXT, COMMENT)?;
    assert_eq!(
        refused.effects,
        saved(checkpoint, PublicationEffect::Pending)
    );
    let data = data(&refused)?;
    assert_eq!(
        data.pending_reason,
        Some(CommentPendingReason::Synchronization)
    );
    assert_eq!(data.conflict, None);
    // The same request is offered first: the record stays accepted.
    assert_eq!(
        actions(&refused).first(),
        Some(&RecoveryActionKind::RequestRetry)
    );
    assert_eq!(state(&w, REQUEST_1), Some(RequestState::Accepted));
    assert!(!served(&w, COMMENT)?);
    assert_comment_golden(&w, "comment_add_push_rejected", &refused);
    Ok(())
}

fn c4_comment_retried_after_the_refusal() -> Result<(), FixtureError> {
    let (w, context, refused) = refused_by_the_server()?;
    assert_eq!(refused.code, ResultCode::PushRejected);
    let path = context.worktree.join(comment_path(ITEM, COMMENT));
    let written = fixed(std::fs::read(&path))?;
    accept_pushes(&w)?;
    let pushes = accepted(&w.server);
    let published = added(&w, REQUEST_1, add(&w));
    assert_eq!(published.code, ResultCode::Ok);
    assert_eq!(published.outcome, Outcome::Success);
    assert_eq!(published.operation_id, refused.operation_id);
    // One comment file, one checkpoint, its creation time unchanged.
    let checkpoint = checkpoint_of(&w, CONTEXT, COMMENT)?;
    assert_eq!(
        published.effects,
        saved(checkpoint, PublicationEffect::Published)
    );
    assert_eq!(data(&published)?.publication, PublicationEffect::Published);
    assert_eq!(
        data(&published)?.synchronization_id,
        data(&refused)?.synchronization_id
    );
    assert_eq!(fixed(std::fs::read(&path))?, written);
    let comments = fixed(std::fs::read_dir(
        context.worktree.join(format!(".manyhands/comments/{ITEM}")),
    ))?
    .count();
    assert_eq!(comments, 1);
    assert!(served(&w, COMMENT)?);
    assert_eq!(accepted(&w.server), pushes + 1);
    assert_eq!(state(&w, REQUEST_1), Some(RequestState::Finished));
    Ok(())
}

fn c4_comment_canonical_conflict() -> Result<(), FixtureError> {
    let (w, context) = published_world()?;
    diverge_on_the_ticket(&w, &context.worktree, ITEM, CONTEXT)?;
    let pushes = accepted(&w.server);
    let stopped = added(&w, REQUEST_1, add(&w));
    assert_eq!(stopped.code, ResultCode::RecoveryRequired);
    assert_eq!(stopped.outcome, Outcome::Partial);
    let checkpoint = checkpoint_of(&w, CONTEXT, COMMENT)?;
    assert_eq!(
        stopped.effects,
        Effects {
            integration: IntegrationEffect::Pending,
            ..saved(checkpoint, PublicationEffect::Pending)
        }
    );
    let data = data(&stopped)?;
    assert_eq!(
        data.conflict,
        Some(ConflictNextDto {
            operation_id: data.synchronization_id.clone(),
            next_action: ConflictNextAction::Resolve,
        })
    );
    assert_eq!(
        data.pending_reason,
        Some(CommentPendingReason::Synchronization)
    );
    assert_eq!(
        actions(&stopped),
        [
            RecoveryActionKind::RequestRetry,
            RecoveryActionKind::RepoInspect
        ]
    );
    // The merge is left in the item's worktree, the reservation is
    // released, and the record stays for the same request.
    let worktree = fixed(git2::Repository::open(&context.worktree))?;
    assert!(worktree.path().join("MERGE_HEAD").exists());
    assert!(fixed(worktree.index())?.has_conflicts());
    assert!(!reserved(&w));
    assert_eq!(state(&w, REQUEST_1), Some(RequestState::Accepted));
    assert_eq!(accepted(&w.server), pushes);
    // C2's retry re-runs it offline: the same answer, with no transport
    // (fact 26).
    let opened = connections(&w);
    let again = added(&w, REQUEST_1, add(&w));
    assert_eq!(again, stopped);
    assert_eq!(connections(&w), opened);
    assert_eq!(checkpoint_of(&w, CONTEXT, COMMENT)?, checkpoint);
    Ok(())
}

fn c4_comment_another_item_s_conflict() -> Result<(), FixtureError> {
    let (w, _) = published_world()?;
    // Another item's merge stops on a conflict, and is left pending.
    assert_eq!(
        execute(&w, REQUEST_4, create_other(&w)).code,
        ResultCode::Ok
    );
    let other = || {
        Mutation::ItemSync(ItemSyncInput {
            root: w.root.clone(),
            kind: AuthoringKind::Ticket,
            id: OTHER_ITEM.parse().unwrap(),
        })
    };
    assert_eq!(execute(&w, REQUEST_5, other()).code, ResultCode::Ok);
    let other_reference = format!("refs/heads/manyhands/ticket/{OTHER_ITEM}");
    let other_worktree = w.root.join(".manyhands/worktrees").join(OTHER_ITEM);
    diverge_on_the_ticket(&w, &other_worktree, OTHER_ITEM, &other_reference)?;
    assert_eq!(
        execute(&w, REQUEST_3, other()).code,
        ResultCode::RecoveryRequired
    );
    let pushes = accepted(&w.server);
    let busy = added(&w, REQUEST_1, add(&w));
    assert_eq!(busy.code, ResultCode::Busy);
    assert_eq!(busy.outcome, Outcome::Partial);
    let checkpoint = checkpoint_of(&w, CONTEXT, COMMENT)?;
    assert_eq!(busy.effects, saved(checkpoint, PublicationEffect::Pending));
    let data = data(&busy)?;
    assert_eq!(
        data.pending_reason,
        Some(CommentPendingReason::Synchronization)
    );
    assert_eq!(data.conflict, None);
    assert_eq!(actions(&busy), [RecoveryActionKind::RequestRetry]);
    assert_eq!(state(&w, REQUEST_1), Some(RequestState::Accepted));
    assert_eq!(accepted(&w.server), pushes);
    assert!(!served(&w, COMMENT)?);
    Ok(())
}

fn c4_comment_this_item_s_conflict() -> Result<(), FixtureError> {
    let (w, context) = published_world()?;
    diverge_on_the_ticket(&w, &context.worktree, ITEM, CONTEXT)?;
    assert_eq!(
        execute(&w, REQUEST_2, item_sync(&w)).code,
        ResultCode::RecoveryRequired
    );
    let rows = merge_rows(&w)?;
    let refused = added(&w, REQUEST_1, add(&w));
    assert_eq!(refused.code, ResultCode::RecoveryRequired);
    assert_eq!(refused.outcome, Outcome::Blocked);
    assert_eq!(actions(&refused), [RecoveryActionKind::RepoInspect]);
    assert_nothing(&w, &context, REQUEST_1, &refused)?;
    assert_eq!(merge_rows(&w)?, rows);
    Ok(())
}

fn c4_comment_unfinished_local_sync() -> Result<(), FixtureError> {
    let (w, context) = world()?;
    leave_local_sync_conflict(&w);
    let opened = connections(&w);
    let refused = added(&w, REQUEST_1, add(&w));
    assert_eq!(refused.code, ResultCode::RecoveryRequired);
    assert_eq!(refused.outcome, Outcome::Blocked);
    assert_eq!(actions(&refused), [RecoveryActionKind::RepoInspect]);
    assert_nothing(&w, &context, REQUEST_1, &refused)?;
    assert!(!reserved(&w));
    assert_eq!(connections(&w), opened);
    Ok(())
}

/// A ticket whose file on primary is lifecycle-closed.
fn closed_ticket_source(id: &str) -> String {
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {id}\ntitle: Closed\n\
         type: task\nstatus: done\nclosed_at: 2026-10-25T12:00:00Z\nclosed_by: {WORLD_AUTHOR}\n\
         ---\nBody of Closed.\n"
    )
}

fn c4_comment_ticket_closed() -> Result<(), FixtureError> {
    let (w, _) = world()?;
    commit_in(
        &w.root,
        "refs/heads/main",
        &merging::ticket_path(CLOSED_TICKET),
        closed_ticket_source(CLOSED_TICKET).as_bytes(),
    )?;
    let refused = added(
        &w,
        REQUEST_1,
        comment_on(&w, CLOSED_TICKET, COMMENT, None, COMMENT_BODY, None),
    );
    assert_eq!(refused.code, ResultCode::TicketClosed);
    assert_eq!(refused.outcome, Outcome::Blocked);
    assert_eq!(refused.effects, Effects::not_requested());
    assert!(refused.data.is_none());
    assert!(refused.recovery.is_empty());
    assert!(record(&w, REQUEST_1).is_none());
    // No context or binding is left.
    assert_eq!(binding_rows(&w)?, 0);
    assert!(
        !w.root
            .join(".manyhands/worktrees")
            .join(CLOSED_TICKET)
            .exists()
    );
    assert!(
        w.repo()?
            .find_reference(&format!("refs/heads/manyhands/ticket/{CLOSED_TICKET}"))
            .is_err()
    );
    assert_comment_golden(&w, "comment_add_ticket_closed", &refused);
    Ok(())
}

/// Asserts that `envelope` stopped for an identity having written
/// nothing: the binding released and the row rejected, the record kept.
fn assert_identity_required_with_nothing_written(
    w: &World,
    envelope: &Envelope<MutationDataDto>,
) -> Result<(), FixtureError> {
    assert_eq!(envelope.code, ResultCode::IdentityRequired);
    assert_eq!(envelope.outcome, Outcome::Blocked);
    assert_eq!(envelope.effects, Effects::not_requested());
    assert!(envelope.data.is_none());
    assert_eq!(
        actions(envelope),
        [
            RecoveryActionKind::RequestRetry,
            RecoveryActionKind::RepoIdentitySet
        ]
    );
    let operation = operation_of(envelope)?;
    assert!(comment_domain::bindings(w, operation)?.is_empty());
    assert_eq!(
        comment_domain::local_row(w, operation)?,
        ("completed".to_owned(), Some("rejected".to_owned()))
    );
    assert_eq!(state(w, REQUEST_1), Some(RequestState::Accepted));
    Ok(())
}

fn c4_comment_identity_required() -> Result<(), FixtureError> {
    let (w, context) = world()?;
    comment_domain::remove_the_identity(&w)?;
    let stopped = added(&w, REQUEST_1, add(&w));
    assert_identity_required_with_nothing_written(&w, &stopped)?;
    assert!(!context.worktree.join(comment_path(ITEM, COMMENT)).exists());
    // After `repo identity-set` the same request writes the comment once.
    assert_eq!(
        merging::set_the_identity(&w, REQUEST_2)?.code,
        ResultCode::Ok
    );
    let published = added(&w, REQUEST_1, add(&w));
    assert_eq!(published.code, ResultCode::Ok);
    assert_eq!(published.operation_id, stopped.operation_id);
    let checkpoint = checkpoint_of(&w, CONTEXT, COMMENT)?;
    assert_eq!(
        published.effects,
        saved(checkpoint, PublicationEffect::Published)
    );
    let author = format!("{} <{}>", merging::MERGE_NAME, merging::MERGE_EMAIL);
    assert_eq!(written_author(&context, COMMENT)?, Some(author.clone()));
    assert_eq!(data(&published)?.created_by, Some(author));
    assert_eq!(binding_rows(&w)?, 1);
    Ok(())
}

/// The item has no editing context yet, so the request makes one before
/// it finds no identity: its row records the context's steps, and the
/// binding is released all the same.
fn c4_comment_identity_required_with_no_context() -> Result<(), FixtureError> {
    let (w, _) = world()?;
    commit_in(
        &w.root,
        "refs/heads/main",
        &merging::ticket_path(NEW_TICKET),
        merging::ticket_source(NEW_TICKET, "New").as_bytes(),
    )?;
    let new_context = format!("refs/heads/manyhands/ticket/{NEW_TICKET}");
    assert!(w.repo()?.find_reference(&new_context).is_err());
    comment_domain::remove_the_identity(&w)?;
    let add_to_new = || comment_on(&w, NEW_TICKET, COMMENT, None, COMMENT_BODY, None);
    let stopped = added(&w, REQUEST_1, add_to_new());
    assert_identity_required_with_nothing_written(&w, &stopped)?;
    let worktree = w.root.join(".manyhands/worktrees").join(NEW_TICKET);
    assert!(!worktree.join(comment_path(NEW_TICKET, COMMENT)).exists());
    assert_eq!(
        merging::set_the_identity(&w, REQUEST_2)?.code,
        ResultCode::Ok
    );
    let published = added(&w, REQUEST_1, add_to_new());
    assert_eq!(published.code, ResultCode::Ok);
    assert_eq!(published.operation_id, stopped.operation_id);
    // Bound afresh, and saved once.
    let operation = operation_of(&published)?;
    assert_eq!(comment_domain::bindings(&w, operation)?.len(), 1);
    checkpoint_of(&w, &new_context, COMMENT)?;
    assert!(holds(
        &w.bare()?,
        &new_context,
        &comment_path(NEW_TICKET, COMMENT)
    )?);
    Ok(())
}

fn c4_comment_identity_required_with_an_author() -> Result<(), FixtureError> {
    let (w, context) = world()?;
    comment_domain::remove_the_identity(&w)?;
    let authored = || comment_on(&w, ITEM, COMMENT, None, COMMENT_BODY, Some(AUTHOR));
    let stopped = added(&w, REQUEST_1, authored());
    // A given author is no commit identity: one is needed either way.
    assert_identity_required_with_nothing_written(&w, &stopped)?;
    assert!(!context.worktree.join(comment_path(ITEM, COMMENT)).exists());
    comment_domain::restore_the_identity(&w)?;
    let published = added(&w, REQUEST_1, authored());
    assert_eq!(published.code, ResultCode::Ok);
    // The author is written verbatim, and the identity commits.
    assert_eq!(written_author(&context, COMMENT)?.as_deref(), Some(AUTHOR));
    assert_eq!(data(&published)?.created_by.as_deref(), Some(AUTHOR));
    let checkpoint = checkpoint_of(&w, CONTEXT, COMMENT)?;
    assert_eq!(
        fixed(w.repo()?.find_commit(checkpoint))?.author().name(),
        Some("Fixture")
    );
    Ok(())
}

fn c4_comment_checkpoint_lands_before_the_retry() -> Result<(), FixtureError> {
    let (w, context) = world()?;
    comment_domain::remove_the_identity(&w)?;
    let stopped = added(&w, REQUEST_1, add(&w));
    assert_identity_required_with_nothing_written(&w, &stopped)?;
    // A checkpoint lands in the context meanwhile.
    let landed = commit_in(&context.worktree, CONTEXT, "landed", b"landed")?;
    comment_domain::restore_the_identity(&w)?;
    let published = added(&w, REQUEST_1, add(&w));
    assert_eq!(published.code, ResultCode::Ok);
    assert_eq!(published.outcome, Outcome::Success);
    // Written once, on the new tip.
    let checkpoint = checkpoint_of(&w, CONTEXT, COMMENT)?;
    assert_eq!(
        fixed(fixed(w.repo()?.find_commit(checkpoint))?.parent_id(0))?,
        landed
    );
    assert_eq!(
        published.effects,
        saved(checkpoint, PublicationEffect::Published)
    );
    assert!(served(&w, COMMENT)?);
    Ok(())
}

fn c4_comment_identity_required_after_a_write() -> Result<(), FixtureError> {
    let (w, context) = world()?;
    // An earlier attempt writes the file and stops before its checkpoint.
    let stopping = fixed(RepositoryService::open_at_with_failure_point_for_testing(
        &w.data(),
        FailurePoint::BeforeCheckpointCommit,
    ))?;
    let first = checked(call(
        &w,
        &stopping,
        REQUEST_1,
        add(&w),
        Provider,
        &mut NoProgress,
        &CancellationToken::new(),
    ));
    drop(stopping);
    assert_eq!(first.code, ResultCode::InternalError);
    assert_eq!(state(&w, REQUEST_1), Some(RequestState::Accepted));
    let path = context.worktree.join(comment_path(ITEM, COMMENT));
    let written = fixed(std::fs::read(&path))?;
    let operation = operation_of(&first)?;
    let bound = comment_domain::bindings(&w, operation)?;
    assert_eq!(bound.len(), 1);

    comment_domain::remove_the_identity(&w)?;
    let stopped = added(&w, REQUEST_1, add(&w));
    assert_eq!(stopped.code, ResultCode::IdentityRequired);
    // The file is written: partial.
    assert_eq!(stopped.outcome, Outcome::Partial);
    assert_eq!(stopped.effects.write, WriteEffect::Written);
    assert_eq!(stopped.effects.checkpoint, CheckpointEffect::Pending);
    assert_eq!(stopped.effects.commit_oid, None);
    assert!(stopped.data.is_none());
    assert_eq!(
        actions(&stopped),
        [
            RecoveryActionKind::RequestRetry,
            RecoveryActionKind::RepoIdentitySet
        ]
    );
    // The binding and the row are kept, and so is the file.
    assert_eq!(comment_domain::bindings(&w, operation)?, bound);
    assert_ne!(
        comment_domain::local_row(&w, operation)?.1.as_deref(),
        Some("rejected")
    );
    assert_eq!(fixed(std::fs::read(&path))?, written);
    assert_eq!(state(&w, REQUEST_1), Some(RequestState::Accepted));

    // After `repo identity-set` the same request checkpoints the written
    // file once.
    assert_eq!(
        merging::set_the_identity(&w, REQUEST_2)?.code,
        ResultCode::Ok
    );
    let published = added(&w, REQUEST_1, add(&w));
    assert_eq!(published.code, ResultCode::Ok);
    let checkpoint = checkpoint_of(&w, CONTEXT, COMMENT)?;
    assert_eq!(
        published.effects,
        saved(checkpoint, PublicationEffect::Published)
    );
    assert_eq!(fixed(std::fs::read(&path))?, written);
    assert_eq!(comment_domain::bindings(&w, operation)?, bound);
    Ok(())
}

fn c4_comment_remote_context_deleted() -> Result<(), FixtureError> {
    let (w, _) = published_world()?;
    fixed(fixed(w.bare()?.find_reference(CONTEXT))?.delete())?;
    let pushes = accepted(&w.server);
    let stopped = added(&w, REQUEST_1, add(&w));
    assert_eq!(stopped.code, ResultCode::RemoteBranchDeleted);
    assert_eq!(stopped.outcome, Outcome::Partial);
    let checkpoint = checkpoint_of(&w, CONTEXT, COMMENT)?;
    assert_eq!(
        stopped.effects,
        saved(checkpoint, PublicationEffect::Pending)
    );
    assert_eq!(
        data(&stopped)?.pending_reason,
        Some(CommentPendingReason::Synchronization)
    );
    // The comment's synchronization holds the reservation, as `item
    // sync`'s does on this code, and the record stays accepted: the same
    // request cannot get past it, and what is offered is its cancellation
    // (controller ruling, Task 2b concern 1).
    assert_eq!(state(&w, REQUEST_1), Some(RequestState::Accepted));
    assert!(reserved(&w));
    assert_eq!(actions(&stopped), [RecoveryActionKind::RequestCancel]);
    assert_eq!(stopped.recovery[0].operation_id, None);
    assert_eq!(stopped.recovery[0].arguments.len(), 1);
    assert_eq!(
        stopped.recovery[0].arguments["request_id"].as_str(),
        Some(REQUEST_1)
    );
    // Nothing is recreated, and a new synchronization is refused while
    // the reservation is held.
    assert!(w.bare()?.find_reference(CONTEXT).is_err());
    assert_eq!(accepted(&w.server), pushes);
    assert_eq!(execute(&w, REQUEST_2, item_sync(&w)).code, ResultCode::Busy);

    // `request cancel`, then the same request, ends it.
    let asked = w
        .service
        .cancel_request(request(REQUEST_1))
        .data
        .ok_or(FixtureError)?;
    assert!(asked.requested);
    let ended = added(&w, REQUEST_1, add(&w));
    assert_eq!(ended.code, ResultCode::Cancelled);
    assert_eq!(ended.outcome, Outcome::Cancelled);
    assert_eq!(ended.operation_id, stopped.operation_id);
    assert_eq!(ended.effects, saved(checkpoint, PublicationEffect::Pending));
    assert!(ended.recovery.is_empty());
    assert!(!reserved(&w));
    assert_eq!(state(&w, REQUEST_1), Some(RequestState::Finished));
    assert_eq!(added(&w, REQUEST_1, add(&w)), ended);
    // The repository is free: a new synchronization reaches the remote,
    // where the branch is still deleted.
    let synchronized = execute(&w, REQUEST_3, item_sync(&w));
    assert_ne!(synchronized.code, ResultCode::Busy);
    assert_eq!(synchronized.code, ResultCode::RemoteBranchDeleted);
    assert!(w.bare()?.find_reference(CONTEXT).is_err());
    assert_eq!(accepted(&w.server), pushes);
    Ok(())
}

fn c4_comment_malformed_author() -> Result<(), FixtureError> {
    let (w, context) = world()?;
    let opened = connections(&w);
    let long = format!("{} <a@example.invalid>", "n".repeat(240));
    for author in [
        "No Brackets",
        "Name <>",
        "<a@example.invalid>",
        " Name <a@example.invalid>",
        "Name <a-at-example.invalid>",
        "Na<me <a@example.invalid>",
        "Na>me <a@example.invalid>",
        "Name <a@example.invalid> trailing",
        "Name  <a@example.invalid>",
        "Name\t<a@example.invalid>",
        "Na\u{7}me <a@example.invalid>",
        "Name <a@example.invalid >",
        long.as_str(),
    ] {
        let refused = added(
            &w,
            REQUEST_1,
            comment_on(&w, ITEM, COMMENT, None, COMMENT_BODY, Some(author)),
        );
        assert_eq!(refused.code, ResultCode::InvalidInput, "{author:?}");
        assert_eq!(refused.outcome, Outcome::Error);
        assert_eq!(refused.failure_class(), Some(FailureClass::Input));
        assert_eq!(refused.operation_id, None);
        assert!(refused.recovery.is_empty());
        assert_nothing(&w, &context, REQUEST_1, &refused)?;
    }
    assert!(long.len() > 256);
    assert_eq!(connections(&w), opened);
    Ok(())
}

fn c4_comment_unknown_parent() -> Result<(), FixtureError> {
    let (w, context) = world()?;
    let refused = added(
        &w,
        REQUEST_1,
        comment_on(&w, ITEM, COMMENT, Some(NO_SUCH_COMMENT), COMMENT_BODY, None),
    );
    assert_eq!(refused.code, ResultCode::ItemNotFound);
    assert_eq!(refused.outcome, Outcome::Error);
    assert_eq!(refused.failure_class(), Some(FailureClass::Input));
    assert!(refused.recovery.is_empty());
    assert_nothing(&w, &context, REQUEST_1, &refused)?;
    assert_comment_golden(&w, "comment_add_item_not_found", &refused);
    Ok(())
}

/// Review Focus 5: a comment that names itself as its parent.
fn c4_comment_self_parent() -> Result<(), FixtureError> {
    let (w, context) = world()?;
    let refused = added(
        &w,
        REQUEST_1,
        comment_on(&w, ITEM, COMMENT, Some(COMMENT), COMMENT_BODY, None),
    );
    assert_eq!(refused.code, ResultCode::ItemNotFound);
    assert_eq!(refused.outcome, Outcome::Error);
    assert_eq!(refused.failure_class(), Some(FailureClass::Input));
    assert!(refused.recovery.is_empty());
    assert_nothing(&w, &context, REQUEST_1, &refused)?;
    Ok(())
}

/// The domain refuses an occupied comment ID after it bound the comment.
/// Having written nothing, the refusal releases the binding with its
/// row's rejection (controller ruling, Task 2b concern 2). It refuses the
/// caller's input, so the record is finished with that result and offers
/// nothing: the same request, sent again, is answered from it (review fix
/// round 1).
fn c4_comment_comment_id_occupied() -> Result<(), FixtureError> {
    let (w, context) = world()?;
    // The ticket's own ID, which its file holds.
    let occupied = || comment_on(&w, ITEM, ITEM, None, COMMENT_BODY, None);
    let first = added(&w, REQUEST_1, occupied());
    let operation = operation_of(&first)?;
    let opened = connections(&w);
    let again = added(&w, REQUEST_1, occupied());
    assert_eq!(again, first);
    assert_eq!(connections(&w), opened);
    for refused in [&first, &again] {
        assert_eq!(refused.code, ResultCode::OccupiedPath);
        assert_eq!(refused.outcome, Outcome::Error);
        assert_eq!(refused.failure_class(), Some(FailureClass::Input));
        assert_eq!(refused.effects, Effects::not_requested());
        assert!(refused.data.is_none());
        assert!(refused.recovery.is_empty());
        assert_eq!(operation_of(refused)?, operation);
    }
    assert_eq!(state(&w, REQUEST_1), Some(RequestState::Finished));
    assert_eq!(binding_rows(&w)?, 0);
    assert_eq!(
        comment_domain::local_row(&w, operation)?,
        ("completed".to_owned(), Some("rejected".to_owned()))
    );
    assert!(!context.worktree.join(comment_path(ITEM, ITEM)).exists());
    Ok(())
}

/// One call of `comment add` whose sink cancels its token when it is
/// told of `stage`: what a Ctrl-C at that point does.
fn cancelled_at(
    w: &World,
    id: &str,
    stage: ProgressStage,
) -> (Envelope<MutationDataDto>, Vec<ProgressStage>) {
    let token = CancellationToken::new();
    let mut sink = CancelAt {
        stage,
        token: token.clone(),
        seen: Vec::new(),
    };
    let envelope = checked(call(w, &w.service, id, add(w), Provider, &mut sink, &token));
    assert!(token.is_cancelled());
    (envelope, sink.seen)
}

fn c4_comment_cancelled_before_the_push() -> Result<(), FixtureError> {
    use ProgressStage as S;
    let (w, _) = published_world()?;
    let pushes = accepted(&w.server);
    let (cancelled, seen) = cancelled_at(&w, REQUEST_1, S::Pushing);
    assert_eq!(cancelled.code, ResultCode::Cancelled);
    assert_eq!(cancelled.outcome, Outcome::Cancelled);
    assert_eq!(
        seen,
        [
            S::Accepted,
            S::Writing,
            S::Fetching,
            S::Observing,
            S::Pushing,
            S::Stopping
        ]
    );
    let checkpoint = checkpoint_of(&w, CONTEXT, COMMENT)?;
    assert_eq!(
        cancelled.effects,
        saved(checkpoint, PublicationEffect::Pending)
    );
    assert_eq!(
        data(&cancelled)?.pending_reason,
        Some(CommentPendingReason::Synchronization)
    );
    assert!(cancelled.recovery.is_empty());
    assert_eq!(state(&w, REQUEST_1), Some(RequestState::Finished));
    assert!(!reserved(&w));
    assert!(!served(&w, COMMENT)?);
    assert_eq!(accepted(&w.server), pushes);
    // The operation is final (fact 56): a new `item sync` publishes the
    // comment.
    let child =
        OperationId::parse(&data(&cancelled)?.synchronization_id).map_err(|_| FixtureError)?;
    assert_eq!(phase(&w, child)?, "cancelled");
    assert_eq!(execute(&w, REQUEST_1, add(&w)), cancelled);
    let synchronized = execute(&w, REQUEST_2, item_sync(&w));
    assert_eq!(synchronized.code, ResultCode::Ok);
    assert!(served(&w, COMMENT)?);
    Ok(())
}

fn c4_comment_cancelled_after_the_push_returned() -> Result<(), FixtureError> {
    let (w, _) = published_world()?;
    let pushes = accepted(&w.server);
    let (cancelled, _) = cancelled_at(&w, REQUEST_1, ProgressStage::Verifying);
    assert_eq!(cancelled.code, ResultCode::Cancelled);
    assert_eq!(cancelled.outcome, Outcome::Cancelled);
    // The publication as the stored push says (fact 74).
    let checkpoint = checkpoint_of(&w, CONTEXT, COMMENT)?;
    assert_eq!(
        cancelled.effects,
        Effects {
            discovery: DiscoveryEffect::Pending,
            ..saved(checkpoint, PublicationEffect::Published)
        }
    );
    let data = data(&cancelled)?;
    assert_eq!(data.publication, PublicationEffect::Published);
    assert_eq!(data.pending_reason, None);
    assert_eq!(state(&w, REQUEST_1), Some(RequestState::Finished));
    assert!(served(&w, COMMENT)?);
    assert_eq!(accepted(&w.server), pushes + 1);
    // A new `item sync` verifies it, and pushes nothing.
    assert_current(&execute(&w, REQUEST_2, item_sync(&w)));
    assert_eq!(accepted(&w.server), pushes + 1);
    Ok(())
}

fn c4_comment_unsettled_push_then_cancelled() -> Result<(), FixtureError> {
    let (w, _) = published_world()?;
    // The server takes the push, and the call never learns of it.
    w.server.disconnect_at(FixtureBoundary::AfterReceivePack);
    let lost = added(&w, REQUEST_1, add(&w));
    assert_eq!(lost.code, ResultCode::TransportUnavailable);
    assert_eq!(lost.outcome, Outcome::Partial);
    assert_eq!(state(&w, REQUEST_1), Some(RequestState::Accepted));
    // From here on the server cannot be reached at all.
    w.server.disconnect_at(FixtureBoundary::Handshake);
    let asked = w
        .service
        .cancel_request(request(REQUEST_1))
        .data
        .ok_or(FixtureError)?;
    assert!(asked.requested);
    // The later call begins with that unsettled push, and is cancelled:
    // the publication is not confirmed.
    let ended = added(&w, REQUEST_1, add(&w));
    assert_eq!(ended.code, ResultCode::RecoveryRequired);
    assert_eq!(ended.outcome, Outcome::Partial);
    let checkpoint = checkpoint_of(&w, CONTEXT, COMMENT)?;
    assert_eq!(ended.effects, saved(checkpoint, PublicationEffect::Pending));
    assert_eq!(ended.operation_id, lost.operation_id);
    assert_eq!(actions(&ended), [RecoveryActionKind::RepoInspect]);
    assert!(!reserved(&w));
    assert_eq!(state(&w, REQUEST_1), Some(RequestState::Finished));
    // A new `item sync` verifies and completes it.
    w.server.clear_fault();
    let verified = execute(&w, REQUEST_2, item_sync(&w));
    assert!(
        matches!(verified.code, ResultCode::Ok | ResultCode::AlreadyApplied),
        "{:?}",
        verified.code
    );
    assert!(served(&w, COMMENT)?);
    Ok(())
}

fn c4_comment_remote_added_later() -> Result<(), FixtureError> {
    let (w, _) = world()?;
    remove_the_remote(&w)?;
    let saved_only = added(&w, REQUEST_1, add(&w));
    assert_eq!(saved_only.code, ResultCode::Ok);
    assert_eq!(
        data(&saved_only)?.pending_reason,
        Some(CommentPendingReason::NoPublicationRemote)
    );
    // No child synchronization ran.
    let child: i64 = fixed(w.db()?.query_row(
        "SELECT COUNT(*) FROM remote_operation_records WHERE operation_ulid = ?1",
        [data(&saved_only)?.synchronization_id.as_str()],
        |row| row.get(0),
    ))?;
    assert_eq!(child, 0);
    // One is added later: a new `item sync` publishes the comment.
    restore_the_remote(&w)?;
    let synchronized = execute(&w, REQUEST_2, item_sync(&w));
    assert_eq!(synchronized.code, ResultCode::Ok);
    assert!(served(&w, COMMENT)?);
    // The finished request still says what it did.
    assert_eq!(execute(&w, REQUEST_1, add(&w)), saved_only);
    Ok(())
}

fn c4_comment_lost_output() -> Result<(), FixtureError> {
    let (w, _) = world()?;
    let first = added(&w, REQUEST_1, add(&w));
    assert_eq!(first.code, ResultCode::Ok);
    // The output is lost; `request show` has the result.
    let shown = w
        .service
        .show_request(request(REQUEST_1))
        .data
        .ok_or(FixtureError)?;
    assert_eq!(shown.state, RequestState::Finished);
    assert_eq!(shown.result.ok_or(FixtureError)?.code, ResultCode::Ok);
    // A replay writes and pushes nothing.
    let opened = connections(&w);
    let pushes = accepted(&w.server);
    let checkpoint = checkpoint_of(&w, CONTEXT, COMMENT)?;
    let again = added(&w, REQUEST_1, add(&w));
    assert_eq!(again, first);
    assert_eq!(connections(&w), opened);
    assert_eq!(accepted(&w.server), pushes);
    assert_eq!(checkpoint_of(&w, CONTEXT, COMMENT)?, checkpoint);
    Ok(())
}

/// The binding's synchronizer runs the child with `MERGING`: a context
/// the remote has moved on merges, rather than being refused.
fn c4_comment_merges_a_divergent_context() -> Result<(), FixtureError> {
    let (w, _) = published_world()?;
    merging::fetch_into_peer(&w, CONTEXT)?;
    let parent = fixed(w.peer.refname_to_id(CONTEXT))?;
    let incoming = commit_path(&w.peer, parent, "remote-divergent", b"remote-divergent")?;
    fixed(w.peer.reference(CONTEXT, incoming, true, "fixture"))?;
    push_peer(&w.peer, &w.server, CONTEXT)?;
    let published = added(&w, REQUEST_1, add(&w));
    assert_eq!(published.code, ResultCode::Ok);
    let checkpoint = checkpoint_of(&w, CONTEXT, COMMENT)?;
    // One merge commit of the comment's checkpoint and the incoming
    // history, published.
    let repo = w.repo()?;
    let merge = fixed(repo.refname_to_id(CONTEXT))?;
    let commit = fixed(repo.find_commit(merge))?;
    assert_eq!(
        commit.parent_ids().collect::<Vec<_>>(),
        [checkpoint, incoming]
    );
    assert_eq!(
        commit.summary(),
        Some(format!("Merge remote context {ITEM}").as_str())
    );
    assert_eq!(fixed(w.bare()?.refname_to_id(CONTEXT))?, merge);
    assert_eq!(
        published.effects,
        Effects {
            integration: IntegrationEffect::Complete,
            ..saved(checkpoint, PublicationEffect::Published)
        }
    );
    assert!(served(&w, COMMENT)?);
    Ok(())
}

/// Review Focus 1: a body with CRLF line endings, a character outside
/// the Basic Multilingual Plane, a line `---` and no final newline.
fn c4_comment_body_round_trips() -> Result<(), FixtureError> {
    let (w, context) = world()?;
    let body = "first line\r\nsecond \u{1F980} line\r\n---\r\nno final newline";
    let published = added(
        &w,
        REQUEST_1,
        comment_on(&w, ITEM, COMMENT, None, body, None),
    );
    assert_eq!(published.code, ResultCode::Ok);
    // Written into the comment's body byte for byte.
    let path = comment_path(ITEM, COMMENT);
    let source = fixed(std::fs::read_to_string(context.worktree.join(&path)))?;
    let canonical::CanonicalItem::Comment(written) =
        fixed(canonical::parse_item(Path::new(&path), &source))?
    else {
        return Err(FixtureError);
    };
    assert_eq!(written.body.as_bytes(), body.as_bytes());
    // `comment list` returns it unchanged.
    let repository = fixed(w.service.resolve_repository(&w.root))?;
    let listed = fixed(w.service.list_comments(&repository, &fixed(ITEM.parse())?))?;
    let comment = listed
        .items
        .iter()
        .find(|comment| comment.id.as_deref() == Some(COMMENT))
        .ok_or(FixtureError)?;
    assert_eq!(
        comment.body.as_deref().map(str::as_bytes),
        Some(body.as_bytes())
    );
    Ok(())
}

/// The same request ID sent again with another body: `request_mismatch`,
/// partial when the first request checkpointed and its record is
/// accepted, and an input error otherwise.
fn c4_comment_request_mismatch() -> Result<(), FixtureError> {
    let another =
        |w: &World, comment: &str| comment_on(w, ITEM, comment, None, "another body\n", None);
    // Checkpointed, its publication refused: its record is accepted.
    let (w, _, refused) = refused_by_the_server()?;
    assert_eq!(refused.code, ResultCode::PushRejected);
    let checkpoint = checkpoint_of(&w, CONTEXT, COMMENT)?;
    let image = format!("{:?}", record(&w, REQUEST_1).ok_or(FixtureError)?);
    let mismatched = added(&w, REQUEST_1, another(&w, COMMENT));
    assert_eq!(mismatched.code, ResultCode::RequestMismatch);
    assert_eq!(mismatched.outcome, Outcome::Partial);
    assert_eq!(mismatched.effects.checkpoint, CheckpointEffect::Committed);
    assert_eq!(mismatched.effects.commit_oid, Some(checkpoint.to_string()));
    assert!(mismatched.data.is_none());
    assert_eq!(
        format!("{:?}", record(&w, REQUEST_1).ok_or(FixtureError)?),
        image
    );

    // Finished: an input error.
    let (w, _) = world()?;
    assert_eq!(added(&w, REQUEST_1, add(&w)).code, ResultCode::Ok);
    let mismatched = added(&w, REQUEST_1, another(&w, COMMENT));
    assert_eq!(mismatched.code, ResultCode::RequestMismatch);
    assert_eq!(mismatched.outcome, Outcome::Error);
    assert_eq!(mismatched.failure_class(), Some(FailureClass::Input));
    assert_eq!(mismatched.effects, Effects::not_requested());

    // Accepted with nothing written: an input error.
    comment_domain::remove_the_identity(&w)?;
    let stopped = added(
        &w,
        REQUEST_3,
        comment_on(&w, ITEM, SECOND_COMMENT, None, COMMENT_BODY, None),
    );
    assert_eq!(stopped.code, ResultCode::IdentityRequired);
    assert_eq!(state(&w, REQUEST_3), Some(RequestState::Accepted));
    let mismatched = added(&w, REQUEST_3, another(&w, SECOND_COMMENT));
    assert_eq!(mismatched.code, ResultCode::RequestMismatch);
    assert_eq!(mismatched.outcome, Outcome::Error);
    assert_eq!(mismatched.effects, Effects::not_requested());
    // Another comment ID is another input too.
    let mismatched = added(
        &w,
        REQUEST_3,
        comment_on(&w, ITEM, THIRD_COMMENT, None, COMMENT_BODY, None),
    );
    assert_eq!(mismatched.code, ResultCode::RequestMismatch);
    Ok(())
}
