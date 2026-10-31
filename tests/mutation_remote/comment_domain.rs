//! The library's comment submission as `comment add` will drive it: the
//! crate-private `submit_comment_with` and `retry_comment_publication_with`,
//! called directly, with a synchronizer the case supplies (R-3).
//!
//! - A parent that is not a comment of the item is refused before any
//!   binding row is inserted, on the binding's path (fact 19).
//! - `IdentityRequired` with nothing written releases the binding, and the
//!   same operation binds afresh; after a write, it keeps the binding
//!   (R-34).
//! - With `release_unwritten: false`, the order and the standing are
//!   today's.
//! - The synchronizer sees the child `synchronization_id` and `restart` as
//!   given.
//! - A given author is written verbatim, once; a malformed one is refused
//!   by its parse, before anything is called (R-7).
//!
//! The merging settings the binding's synchronizer passes are asserted
//! through the binding, in `comment`.

use std::path::{Path, PathBuf};

use super::*;

pub(crate) const CASES: &[ssh_harness::Case] = &[
    (
        "c4_comment_domain_parent_refused_before_binding",
        c4_comment_domain_parent_refused_before_binding,
    ),
    (
        "c4_comment_domain_identity_required_releases_an_unwritten_binding",
        c4_comment_domain_identity_required_releases_an_unwritten_binding,
    ),
    (
        "c4_comment_domain_identity_required_after_a_write_keeps_the_binding",
        c4_comment_domain_identity_required_after_a_write_keeps_the_binding,
    ),
    (
        "c4_comment_domain_without_release_the_public_path_is_unchanged",
        c4_comment_domain_without_release_the_public_path_is_unchanged,
    ),
    (
        "c4_comment_domain_the_synchronizer_sees_the_child_and_restart",
        c4_comment_domain_the_synchronizer_sees_the_child_and_restart,
    ),
    (
        "c4_comment_domain_a_given_author_is_written_verbatim",
        c4_comment_domain_a_given_author_is_written_verbatim,
    ),
    (
        "c4_comment_domain_a_malformed_author_is_refused_before_any_write",
        c4_comment_domain_a_malformed_author_is_refused_before_any_write,
    ),
];

/// A ticket the case commits on primary, with no editing context, so the
/// parent is read from the primary checkout.
const PRIMARY_ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FB1";
/// A comment of `PRIMARY_ITEM`, on primary.
const PRIMARY_COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FB2";
const COMMENT_BODY: &str = "A comment of the domain cases.\n";

type Submitted = Result<CommentSubmissionOutcome, CommentSubmissionError>;

/// A world whose item has its editing context.
fn world() -> Result<(World, ItemContext), FixtureError> {
    let mut w = World::new()?;
    let context = w.context()?;
    Ok((w, context))
}

/// A new comment of `item` under `operation`.
fn comment(
    w: &World,
    item: &str,
    operation: OperationId,
    comment_id: canonical::ItemId,
    parent_id: Option<canonical::ItemId>,
) -> Result<PublishCommentRequest, FixtureError> {
    Ok(PublishCommentRequest {
        comment: SubmitCommentRequest {
            target: AuthoringTarget {
                root: w.root.clone(),
                kind: AuthoringKind::Ticket,
                item_id: fixed(item.parse())?,
                intent: ContextIntent::Edit,
                operation_id: operation,
            },
            comment_id,
            parent_id,
            body: COMMENT_BODY.into(),
            expected_destination: ExpectedPathObservation::Missing,
        },
        approval: w.context_request().approval,
        confirmed_identity: None,
    })
}

/// A synchronizer that makes no domain call.
fn unsynchronized(_: SynchronizeRemoteRequest) -> Option<SynchronizationAnswer> {
    None
}

type SynchronizationAnswer = Result<SynchronizationResult, SynchronizationError>;

/// The binding's submission: no restart, a binding that wrote nothing
/// released.
fn bound(w: &World, request: PublishCommentRequest) -> Submitted {
    w.service
        .submit_comment_with(request, None, false, true, &mut unsynchronized)
}

fn saved(outcome: Submitted) -> Result<(CommentReceipt, CommentPublicationState), FixtureError> {
    match outcome {
        Ok(CommentSubmissionOutcome::Saved {
            receipt,
            publication,
            ..
        }) => Ok((*receipt, publication)),
        _ => Err(FixtureError),
    }
}

fn identity_required(outcome: &Submitted) -> bool {
    matches!(
        outcome,
        Ok(CommentSubmissionOutcome::IdentityRequired { .. })
    )
}

fn refused(outcome: Submitted) -> Option<RepositoryErrorKind> {
    outcome.err().map(|error| error.kind)
}

/// The child synchronization IDs bound to `operation`.
pub(super) fn bindings(w: &World, operation: OperationId) -> Result<Vec<String>, FixtureError> {
    let db = w.db()?;
    let mut statement = fixed(db.prepare(
        "SELECT synchronization_ulid FROM comment_publication_bindings WHERE operation_ulid=?1",
    ))?;
    let rows = fixed(statement.query_map([operation.to_string()], |row| row.get(0)))?;
    fixed(rows.collect::<Result<Vec<String>, _>>())
}

/// The local row of `operation`: its state and its last step.
pub(super) fn local_row(
    w: &World,
    operation: OperationId,
) -> Result<(String, Option<String>), FixtureError> {
    fixed(w.db()?.query_row(
        "SELECT state, completed_step FROM operation_records WHERE operation_ulid=?1",
        [operation.to_string()],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ))
}

fn comment_path(item: &str, comment_id: &canonical::ItemId) -> PathBuf {
    PathBuf::from(format!(".manyhands/comments/{item}/{comment_id}.md"))
}

/// The commits on `reference` whose subject checkpoints `comment_id`.
pub(super) fn checkpoints(
    w: &World,
    reference: &str,
    comment_id: &canonical::ItemId,
) -> Result<Vec<git2::Oid>, FixtureError> {
    let repo = w.repo()?;
    let mut walk = fixed(repo.revwalk())?;
    fixed(walk.push(fixed(repo.refname_to_id(reference))?))?;
    let subject = format!("Checkpoint comment {comment_id}");
    let mut found = Vec::new();
    for oid in walk {
        let oid = fixed(oid)?;
        if fixed(repo.find_commit(oid))?.summary() == Some(subject.as_str()) {
            found.push(oid);
        }
    }
    Ok(found)
}

fn parent_of(w: &World, oid: git2::Oid) -> Result<git2::Oid, FixtureError> {
    fixed(fixed(w.repo()?.find_commit(oid))?.parent_id(0))
}

/// Removes the identity the world gives its repository (fact 60 keeps
/// any other out).
pub(super) fn remove_the_identity(w: &World) -> Result<(), FixtureError> {
    let mut config = fixed(fixed(w.repo()?.config())?.open_level(git2::ConfigLevel::Local))?;
    fixed(config.remove("user.name"))?;
    fixed(config.remove("user.email"))?;
    Ok(())
}

/// Gives the repository the world's identity again.
pub(super) fn restore_the_identity(w: &World) -> Result<(), FixtureError> {
    let mut config = fixed(fixed(w.repo()?.config())?.open_level(git2::ConfigLevel::Local))?;
    fixed(config.set_str("user.name", "Fixture"))?;
    fixed(config.set_str("user.email", "fixture@example.invalid"))?;
    Ok(())
}

/// A ticket and one of its comments, committed on primary.
fn commit_primary_item(w: &World) -> Result<(), FixtureError> {
    commit_in(
        &w.root,
        "refs/heads/main",
        &merging::ticket_path(PRIMARY_ITEM),
        merging::ticket_source(PRIMARY_ITEM, "Primary").as_bytes(),
    )?;
    let source = format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: comment\nid: \"{PRIMARY_COMMENT}\"\n\
         item_id: \"{PRIMARY_ITEM}\"\ncreated_at: \"2026-01-01T00:00:00Z\"\n---\n\nParent\n"
    );
    commit_in(
        &w.root,
        "refs/heads/main",
        &format!(".manyhands/comments/{PRIMARY_ITEM}/{PRIMARY_COMMENT}.md"),
        source.as_bytes(),
    )?;
    Ok(())
}

fn c4_comment_domain_parent_refused_before_binding() -> Result<(), FixtureError> {
    let (w, _) = world()?;
    // In the item's editing worktree: no such comment, and the comment
    // itself.
    let unknown = OperationId::new();
    let request = comment(
        &w,
        ITEM,
        unknown,
        canonical::ItemId::generate(),
        Some(canonical::ItemId::generate()),
    )?;
    assert_eq!(
        refused(bound(&w, request)),
        Some(RepositoryErrorKind::MissingAuthoringTarget)
    );
    assert!(bindings(&w, unknown)?.is_empty());
    let itself = OperationId::new();
    let comment_id = canonical::ItemId::generate();
    let request = comment(&w, ITEM, itself, comment_id.clone(), Some(comment_id))?;
    assert_eq!(
        refused(bound(&w, request)),
        Some(RepositoryErrorKind::MissingAuthoringTarget)
    );
    assert!(bindings(&w, itself)?.is_empty());
    // A comment of the item is a parent.
    let first = canonical::ItemId::generate();
    saved(bound(
        &w,
        comment(&w, ITEM, OperationId::new(), first.clone(), None)?,
    ))?;
    let reply = OperationId::new();
    saved(bound(
        &w,
        comment(&w, ITEM, reply, canonical::ItemId::generate(), Some(first))?,
    ))?;
    assert_eq!(bindings(&w, reply)?.len(), 1);

    // With no editing context, the primary checkout is read, and nothing
    // is made of a context for a refused parent.
    commit_primary_item(&w)?;
    let worktree = w.root.join(".manyhands/worktrees").join(PRIMARY_ITEM);
    let unknown = OperationId::new();
    let request = comment(
        &w,
        PRIMARY_ITEM,
        unknown,
        canonical::ItemId::generate(),
        Some(canonical::ItemId::generate()),
    )?;
    assert_eq!(
        refused(bound(&w, request)),
        Some(RepositoryErrorKind::MissingAuthoringTarget)
    );
    assert!(bindings(&w, unknown)?.is_empty());
    assert!(!worktree.exists());
    let reply = OperationId::new();
    saved(bound(
        &w,
        comment(
            &w,
            PRIMARY_ITEM,
            reply,
            canonical::ItemId::generate(),
            Some(fixed(PRIMARY_COMMENT.parse())?),
        )?,
    ))?;
    assert_eq!(bindings(&w, reply)?.len(), 1);
    Ok(())
}

fn c4_comment_domain_identity_required_releases_an_unwritten_binding() -> Result<(), FixtureError> {
    let (w, context) = world()?;
    let operation = OperationId::new();
    let comment_id = canonical::ItemId::generate();
    let request = comment(&w, ITEM, operation, comment_id.clone(), None)?;
    remove_the_identity(&w)?;
    assert!(identity_required(&bound(&w, request.clone())));
    // One transaction deleted the binding and rejected the row.
    assert!(bindings(&w, operation)?.is_empty());
    assert_eq!(
        local_row(&w, operation)?,
        ("completed".to_owned(), Some("rejected".to_owned()))
    );
    let path = comment_path(ITEM, &comment_id);
    assert!(!context.worktree.join(&path).exists());

    // A checkpoint lands in the context meanwhile (fact 68).
    let landed = commit_in(&context.worktree, CONTEXT, "landed", b"landed")?;
    restore_the_identity(&w)?;
    let mut seen = Vec::new();
    let mut record = |request: SynchronizeRemoteRequest| -> Option<SynchronizationAnswer> {
        seen.push(request.operation_id);
        None
    };
    let (receipt, _) =
        saved(
            w.service
                .submit_comment_with(request, None, false, true, &mut record),
        )?;
    // Bound afresh, on the new tip, and written once.
    assert_eq!(
        bindings(&w, operation)?,
        [receipt.synchronization_id.to_string()]
    );
    assert_eq!(seen, [receipt.synchronization_id]);
    assert_eq!(
        checkpoints(&w, CONTEXT, &comment_id)?,
        [receipt.checkpoint_oid]
    );
    assert_eq!(parent_of(&w, receipt.checkpoint_oid)?, landed);
    let written = fixed(std::fs::read_to_string(context.worktree.join(&path)))?;
    assert!(written.contains("created_by: Fixture <fixture@example.invalid>"));
    Ok(())
}

fn c4_comment_domain_identity_required_after_a_write_keeps_the_binding() -> Result<(), FixtureError>
{
    let (w, context) = world()?;
    let operation = OperationId::new();
    let comment_id = canonical::ItemId::generate();
    let request = comment(&w, ITEM, operation, comment_id.clone(), None)?;
    // An earlier attempt writes the file and stops before its checkpoint.
    let stopping = fixed(RepositoryService::open_at_with_failure_point_for_testing(
        &w.data(),
        FailurePoint::BeforeCheckpointCommit,
    ))?;
    let stopped =
        stopping.submit_comment_with(request.clone(), None, false, true, &mut unsynchronized);
    drop(stopping);
    assert_eq!(refused(stopped), Some(RepositoryErrorKind::InjectedFailure));
    let path = context.worktree.join(comment_path(ITEM, &comment_id));
    let written = fixed(std::fs::read(&path))?;
    let bound_to = bindings(&w, operation)?;
    assert_eq!(bound_to.len(), 1);

    remove_the_identity(&w)?;
    assert!(identity_required(&bound(&w, request.clone())));
    // The binding and the row are kept, and so is the file.
    assert_eq!(bindings(&w, operation)?, bound_to);
    assert_ne!(local_row(&w, operation)?.1.as_deref(), Some("rejected"));
    assert_eq!(fixed(std::fs::read(&path))?, written);

    restore_the_identity(&w)?;
    let (receipt, _) = saved(bound(&w, request))?;
    // The written file is checkpointed once, as it was written.
    assert_eq!(bound_to, [receipt.synchronization_id.to_string()]);
    assert_eq!(
        checkpoints(&w, CONTEXT, &comment_id)?,
        [receipt.checkpoint_oid]
    );
    assert_eq!(fixed(std::fs::read(&path))?, written);
    Ok(())
}

fn c4_comment_domain_without_release_the_public_path_is_unchanged() -> Result<(), FixtureError> {
    let (w, context) = world()?;
    let public = |request: PublishCommentRequest| {
        w.service
            .submit_comment_with(request, None, false, false, &mut unsynchronized)
    };
    // A refused parent leaves its binding row, in today's order.
    let unknown = OperationId::new();
    let request = comment(
        &w,
        ITEM,
        unknown,
        canonical::ItemId::generate(),
        Some(canonical::ItemId::generate()),
    )?;
    assert_eq!(
        refused(public(request)),
        Some(RepositoryErrorKind::MissingAuthoringTarget)
    );
    assert_eq!(bindings(&w, unknown)?.len(), 1);

    // `IdentityRequired` keeps the binding and completes the row.
    let operation = OperationId::new();
    let request = comment(&w, ITEM, operation, canonical::ItemId::generate(), None)?;
    remove_the_identity(&w)?;
    assert!(identity_required(&public(request.clone())));
    let bound_to = bindings(&w, operation)?;
    assert_eq!(bound_to.len(), 1);
    let (state, step) = local_row(&w, operation)?;
    assert_eq!(state, "completed");
    assert_ne!(step.as_deref(), Some("rejected"));

    // A checkpoint that lands meanwhile then refuses the same operation.
    commit_in(&context.worktree, CONTEXT, "landed", b"landed")?;
    restore_the_identity(&w)?;
    assert_eq!(
        refused(public(request)),
        Some(RepositoryErrorKind::RecoveryRequired)
    );
    assert_eq!(bindings(&w, operation)?, bound_to);
    Ok(())
}

/// What a synchronizer was asked: the operation, `restart`, the target
/// and the root.
type Asked = (OperationId, bool, SynchronizationTarget, PathBuf);

fn asked(request: &SynchronizeRemoteRequest) -> Asked {
    (
        request.operation_id,
        request.restart,
        request.target.clone(),
        request.root.clone(),
    )
}

fn c4_comment_domain_the_synchronizer_sees_the_child_and_restart() -> Result<(), FixtureError> {
    let (w, _) = world()?;
    let operation = OperationId::new();
    let request = comment(&w, ITEM, operation, canonical::ItemId::generate(), None)?;
    let mut seen = Vec::new();
    let mut record = |request: SynchronizeRemoteRequest| -> Option<SynchronizationAnswer> {
        seen.push(asked(&request));
        None
    };
    // The first submission writes, then asks the synchronizer, which
    // makes no domain call: the publication is pending.
    let (receipt, publication) =
        saved(
            w.service
                .submit_comment_with(request.clone(), None, false, true, &mut record),
        )?;
    assert!(matches!(
        publication,
        CommentPublicationState::Pending {
            reason: CommentPublicationPendingReason::Synchronization(ref error),
        } if matches!(**error, SynchronizationError::RecoveryRequired)
    ));
    // Re-entry proves the receipt and asks again, with `restart` as given.
    saved(
        w.service
            .submit_comment_with(request, None, true, true, &mut record),
    )?;
    let child = receipt.synchronization_id;
    assert_eq!(bindings(&w, operation)?, [child.to_string()]);
    let target = SynchronizationTarget::Context {
        kind: AuthoringKind::Ticket,
        item_id: fixed(ITEM.parse())?,
    };
    let root = fixed(std::fs::canonicalize(&w.root))?;
    assert_eq!(
        seen,
        [
            (child, false, target.clone(), root.clone()),
            (child, true, target.clone(), root.clone()),
        ]
    );

    // The retry gives its own synchronizer the same child, and a real
    // synchronization through it publishes the comment.
    let mut seen = Vec::new();
    let mut publish = |request: SynchronizeRemoteRequest| -> Option<SynchronizationAnswer> {
        seen.push(asked(&request));
        Some(
            w.service
                .synchronize_remote(request, &mut SessionCredentials::new(Provider)),
        )
    };
    let (retried, publication) = saved(w.service.retry_comment_publication_with(
        RetryCommentPublicationRequest {
            root: w.root.clone(),
            operation_id: operation,
            approval: w.context_request().approval,
            confirmed_identity: None,
            restart: false,
        },
        &mut publish,
    ))?;
    assert_eq!(seen, [(child, false, target, root)]);
    assert_eq!(retried.synchronization_id, child);
    let served = fixed(w.bare()?.refname_to_id(CONTEXT))?;
    assert!(matches!(
        publication,
        CommentPublicationState::Published { oid } if oid == served
    ));
    assert!(served_contains(&w, &receipt.comment_path)?);
    Ok(())
}

/// The `created_by` of the comment at `path` in `worktree`, as its file
/// holds it.
fn author_of(worktree: &Path, path: &Path) -> Result<Option<String>, FixtureError> {
    let source = fixed(std::fs::read_to_string(worktree.join(path)))?;
    match fixed(canonical::parse_item(path, &source))? {
        canonical::CanonicalItem::Comment(comment) => Ok(comment.created_by),
        _ => Err(FixtureError),
    }
}

const AUTHOR: &str = "Comment Author <comment-author@example.invalid>";

fn c4_comment_domain_a_given_author_is_written_verbatim() -> Result<(), FixtureError> {
    let (w, context) = world()?;
    let operation = OperationId::new();
    let comment_id = canonical::ItemId::generate();
    let request = comment(&w, ITEM, operation, comment_id.clone(), None)?;
    let author = || CommentAuthor::parse(AUTHOR).map_err(|_| FixtureError);
    let (receipt, _) = saved(w.service.submit_comment_with(
        request.clone(),
        Some(author()?),
        false,
        true,
        &mut unsynchronized,
    ))?;
    let path = comment_path(ITEM, &comment_id);
    // The given text replaces the identity's, verbatim; the identity
    // still commits.
    assert_eq!(
        author_of(&context.worktree, &path)?.as_deref(),
        Some(AUTHOR)
    );
    let repo = w.repo()?;
    let commit = fixed(repo.find_commit(receipt.checkpoint_oid))?;
    assert_eq!(commit.author().name(), Some("Fixture"));
    assert_eq!(commit.author().email(), Some("fixture@example.invalid"));
    // It is written once: the same operation with another author rewrites
    // nothing.
    let written = fixed(std::fs::read(context.worktree.join(&path)))?;
    let other = CommentAuthor::parse("Another Author <another@example.invalid>")
        .map_err(|_| FixtureError)?;
    let (again, _) = saved(w.service.submit_comment_with(
        request,
        Some(other),
        true,
        true,
        &mut unsynchronized,
    ))?;
    assert_eq!(again.checkpoint_oid, receipt.checkpoint_oid);
    assert_eq!(fixed(std::fs::read(context.worktree.join(&path)))?, written);
    assert_eq!(
        checkpoints(&w, CONTEXT, &comment_id)?,
        [receipt.checkpoint_oid]
    );
    Ok(())
}

fn c4_comment_domain_a_malformed_author_is_refused_before_any_write() -> Result<(), FixtureError> {
    let (w, _) = world()?;
    let long = format!("{} <a@example.invalid>", "n".repeat(240));
    assert!(long.len() > 256);
    for malformed in [
        "",
        "No Brackets",
        "Name <>",
        "<a@example.invalid>",
        " Name <a@example.invalid>",
        "Name  <a@example.invalid>",
        "Name\t<a@example.invalid>",
        "Name <a-at-example.invalid>",
        "Name < a@example.invalid>",
        "Name <a@example.invalid >",
        "Na<me <a@example.invalid>",
        "Na>me <a@example.invalid>",
        "Name <a@<example.invalid>",
        "Name <a@example.invalid> trailing",
        "Name <a@example.invalid>>",
        "Na\u{7}me <a@example.invalid>",
        "Name <a@exam\nple.invalid>",
        long.as_str(),
    ] {
        assert!(
            CommentAuthor::parse(malformed).is_err(),
            "{malformed:?} is not an author"
        );
    }
    // At the limit, and with a name of more than one word, outside ASCII.
    let at_the_limit = format!("{} <a@example.invalid>", "n".repeat(236));
    assert_eq!(at_the_limit.len(), 256);
    for wellformed in [
        AUTHOR,
        "Zoë Ångström-Ñuñez <zoe@example.invalid>",
        at_the_limit.as_str(),
    ] {
        let author = CommentAuthor::parse(wellformed).map_err(|_| FixtureError)?;
        assert_eq!(author.as_str(), wellformed);
    }
    // Nothing is called with a malformed one, so nothing was written.
    let bound: i64 = fixed(w.db()?.query_row(
        "SELECT COUNT(*) FROM comment_publication_bindings",
        [],
        |row| row.get(0),
    ))?;
    assert_eq!(bound, 0);
    Ok(())
}

/// Whether the server's context branch holds `path`.
fn served_contains(w: &World, path: &Path) -> Result<bool, FixtureError> {
    let bare = w.bare()?;
    let tip = fixed(bare.find_commit(fixed(bare.refname_to_id(CONTEXT))?))?;
    Ok(fixed(tip.tree())?.get_path(path).is_ok())
}
