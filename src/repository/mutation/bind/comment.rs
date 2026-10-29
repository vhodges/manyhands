//! `comment add`: a new comment written into its item's editing context,
//! checkpointed there, and published by a synchronization of that
//! context's branch. It is one call of `submit_comment_with`.
//!
//! The comment's own work is an operation of the local journal, under
//! the request's operation ID: its write and its checkpoint. The
//! publication is a child synchronization, under an ID the comment's
//! binding names (fact 21), which no request record holds (fact 79).
//! That synchronization is called as `item sync` calls its own: with the
//! merging settings, and a `Watch` that reports its safe points as this
//! request's progress and stops it when this request is asked to stop
//! (R-3). Its outcome is answered as `item sync` answers it, and is this
//! request's: the code, what was published or merged, the conflict's
//! next step.
//!
//! **Standing (R-34).**
//!
//! - A comment published, or with no publication remote to publish to,
//!   is finished.
//! - A publication left pending is owed: the record stays accepted, and
//!   the same request publishes the comment without writing it again.
//!   The exception is a child that is final, such as one cancelled,
//!   which is finished. A code the same request never gets past, such as
//!   `remote_branch_deleted`, is owed as well: the child holds the
//!   reservation, as `item sync`'s does, and `request cancel` followed by
//!   the same request ends it, `cancelled`.
//! - `identity_required` keeps the record accepted, and so does a refusal
//!   the domain makes after it bound the comment. A binding that wrote
//!   nothing is released with its row's rejection, so the same request
//!   binds afresh; one that wrote the file is kept, and the same request
//!   checkpoints the file once.
//!
//! A comment's operation records no checkpoint step of the boundary's
//! kind, so a completed row says nothing of the request
//! (`EndedWithoutCheckpoint::SaysNothing`). Re-entry makes the domain
//! call with `restart` (fact 65): the submission proves the receipt from
//! Git and publishes, and never writes a comment it has checkpointed.

use std::path::{Path, PathBuf};

use rusqlite::OptionalExtension;

use crate::{
    canonical::{self, ItemId},
    results::{
        CheckpointEffect, DiscoveryEffect, Effects, PublicationEffect, ResultCode, Scope,
        WriteEffect,
    },
};

use super::{
    super::{
        super::{
            AuthoringKind, AuthoringTarget, CommentAuthor, CommentIndexingState,
            CommentPublicationPendingReason, CommentPublicationState, CommentReceipt,
            CommentSubmissionError, CommentSubmissionOutcome, ContextIntent,
            ExpectedPathObservation, OperationFamily, OperationId, PublishCommentRequest,
            RepositoryOperation, RepositoryService, SubmitCommentRequest, authoring_kind_segment,
            owned_file_bytes,
            recovery::{self, FinalKind, JournalRow},
        },
        command::Command,
        context::context_names,
        dto::{CommentMutationDto, CommentMutationType, CommentPendingReason, MutationDataDto},
        evidence::Position,
        identity::{FieldValue, IntentDigestBuilder},
        outcome,
        progress::Subject,
    },
    Accepted, Answer, Binding, CallContext, Confirmed, EndedWithoutCheckpoint, Entered, Place,
    Prepared, Ran, Standing, item_scope, prepared, resolve_root, sync,
};

/// The input of `comment add`: a new comment of the item `id`, of kind
/// `kind`. The caller supplies the comment's ID. `created_by`, when
/// given, is the comment's author in place of the Git identity's text
/// (decision 9, R-7). The request's digest covers every field, the body
/// by digest; nothing derives a rendering of it.
pub struct CommentAddInput {
    pub root: PathBuf,
    pub kind: AuthoringKind,
    pub id: ItemId,
    pub comment_id: ItemId,
    pub parent_id: Option<ItemId>,
    pub body: String,
    pub created_by: Option<String>,
}

pub(crate) struct CommentAddBinding {
    root: PathBuf,
    kind: AuthoringKind,
    id: ItemId,
    comment_id: ItemId,
    parent_id: Option<ItemId>,
    body: String,
    created_by: Option<String>,
}

impl From<CommentAddInput> for CommentAddBinding {
    fn from(input: CommentAddInput) -> Self {
        // Every field is named: one added to the input does not compile
        // until it is taken into the binding or left out here.
        let CommentAddInput {
            root,
            kind,
            id,
            comment_id,
            parent_id,
            body,
            created_by,
        } = input;
        Self {
            root,
            kind,
            id,
            comment_id,
            parent_id,
            body,
            created_by,
        }
    }
}

/// The comment and the synchronization that publishes it, for the
/// comment's operation or for its synchronization, whichever
/// `operation_id` is, in the repository at `root` (R-5). `None` when the
/// registry binds neither to a comment there, or cannot be read.
pub(crate) fn comment_synchronization(
    connection: &rusqlite::Connection,
    root: &Path,
    operation_id: OperationId,
) -> Option<(OperationId, OperationId)> {
    let root = root.to_str()?;
    if !recovery::has_table(connection, "comment_publication_bindings").ok()? {
        return None;
    }
    let (comment, synchronization): (String, String) = connection
        .query_row(
            "SELECT binding.operation_ulid, binding.synchronization_ulid
               FROM comment_publication_bindings binding
               JOIN operation_records operation ON operation.id = binding.local_record_id
              WHERE operation.root_path = ?1
                AND (binding.operation_ulid = ?2 OR binding.synchronization_ulid = ?2)",
            rusqlite::params![root, operation_id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .ok()??;
    Some((
        OperationId::parse(&comment).ok()?,
        OperationId::parse(&synchronization).ok()?,
    ))
}

/// The request `accepted`, as the binding of a synchronization is to see
/// it: with the comment's synchronization `operation_id` in place of the
/// comment's own operation, as a resume builds one for the operation it
/// takes up. Its position is the comment's, on the item's branch, which
/// is the branch the synchronization publishes.
fn synchronization_of(accepted: &Accepted, operation_id: OperationId) -> Accepted {
    Accepted {
        request_id: accepted.request_id,
        attempt: accepted.attempt,
        scope: accepted.scope.clone(),
        family: Some(OperationFamily::Remote),
        operation_id,
        position: accepted.position.clone(),
        expected: None,
    }
}

/// What the repository shows of the comment the request `accepted`
/// writes: committed, once its binding records the checkpoint; written,
/// when its file is in the item's worktree under a binding that holds it;
/// and nothing otherwise, a binding that was released among it.
fn comment_effects(service: &RepositoryService, accepted: &Accepted) -> Effects {
    let root = accepted.root();
    let Ok(Some(binding)) = service.read_comment_binding(&root, accepted.operation_id) else {
        return Effects::not_requested();
    };
    match binding.checkpoint_oid {
        Some(commit) => outcome::stopped_save_effects(Some(commit), true),
        None => {
            let item = binding.item_id.to_string();
            let worktree = context_names(&root, &binding.kind, &item).worktree;
            let path = format!(".manyhands/comments/{item}/{}.md", binding.comment_id);
            let written = owned_file_bytes(
                &worktree,
                Path::new(&path),
                RepositoryOperation::Read,
                &root,
            )
            .ok()
            .flatten()
            .is_some();
            outcome::stopped_save_effects(None, written)
        }
    }
}

/// What the repository shows of an unfinished `comment add`: the
/// comment's write and checkpoint, and, once it is checkpointed, what
/// its synchronization is known to have published and merged. This is
/// the answer to another request, and stores nothing.
pub(crate) fn recorded_effects(service: &RepositoryService, accepted: &Accepted) -> Effects {
    let mut effects = comment_effects(service, accepted);
    if effects.checkpoint != CheckpointEffect::Committed {
        return effects;
    }
    let bound = service
        .read_session(RepositoryOperation::Read, |connection| {
            Ok(comment_synchronization(
                connection,
                &accepted.root(),
                accepted.operation_id,
            ))
        })
        .ok()
        .flatten();
    // A synchronization that cannot be found is no claim of anything: the
    // publication is pending, which is what "not confirmed" is written as.
    effects.publication = PublicationEffect::Pending;
    if let Some((_, synchronization)) = bound {
        let published =
            sync::recorded_effects(service, &synchronization_of(accepted, synchronization));
        effects.publication = published.publication;
        effects.integration = published.integration;
    }
    effects
}

/// The comment's `created_by`, as its checkpoint holds it. `None` when
/// the checkpoint cannot be read, or names nobody.
fn checkpointed_author(receipt: &CommentReceipt) -> Option<String> {
    let repository = git2::Repository::open(&receipt.root).ok()?;
    let entry = repository
        .find_commit(receipt.checkpoint_oid)
        .ok()?
        .tree()
        .ok()?
        .get_path(&receipt.comment_path)
        .ok()?;
    let blob = repository.find_blob(entry.id()).ok()?;
    let source = std::str::from_utf8(blob.content()).ok()?;
    match canonical::parse_item(&receipt.comment_path, source).ok()? {
        canonical::CanonicalItem::Comment(comment) => comment.created_by,
        canonical::CanonicalItem::Document(_) | canonical::CanonicalItem::Ticket(_) => None,
    }
}

/// What the comment's synchronization came to, when the submission
/// called it.
struct Child {
    /// What the binding of a synchronization makes of the call.
    ran: Ran,
    /// The domain returned a result, and no error.
    succeeded: bool,
}

/// What becomes of the record of a request whose publication stopped,
/// given what the synchronization's binding says of its own row (R-34). A
/// row that was cancelled or retained is final. A row that completed
/// beside an error leaves the record for the same request to answer from
/// it. Any other stop is owed, whatever stopped it: the synchronization
/// holds the repository's reservation, and the same request is what takes
/// it up, or, for a code the same request never gets past, such as
/// `remote_branch_deleted`, what ends it after `request cancel`, as for
/// `item sync` (controller ruling, Task 2b concern 1).
fn standing_of(standing: Standing) -> Standing {
    match standing {
        Standing::Final => Standing::Final,
        Standing::Owed => Standing::Owed,
        Standing::Unconfirmed => Standing::Unconfirmed,
        Standing::Stopped {
            journal:
                Some(JournalRow::Final {
                    kind: FinalKind::Cancelled | FinalKind::RetainedForInspection,
                    owes_work: _,
                    checkpointed: _,
                }),
        } => Standing::Final,
        Standing::Stopped {
            journal:
                Some(JournalRow::Final {
                    kind: FinalKind::Completed,
                    owes_work: _,
                    checkpointed: _,
                }),
        } => Standing::Unconfirmed,
        Standing::Stopped {
            journal: Some(JournalRow::Absent | JournalRow::Pending(_)) | None,
        }
        | Standing::NotRun { journal: _ } => Standing::Owed,
    }
}

impl CommentAddBinding {
    /// The author the request gives, as it is written. A `created_by`
    /// that is not written as an author is the caller's input error.
    fn author(&self) -> Result<Option<CommentAuthor>, Answer> {
        self.created_by
            .as_deref()
            .map(CommentAuthor::parse)
            .transpose()
            .map_err(|_| Answer::stopped(ResultCode::InvalidInput))
    }

    /// The one domain call, under the recorded operation ID and
    /// expectation, and what it came to. `restart` is set by a call that
    /// enters the request again.
    fn submit(&self, call: &CallContext<'_>, accepted: &Accepted, restart: bool) -> Ran {
        let service = call.service;
        let author = match self.author() {
            Ok(author) => author,
            // The author was read before the request was accepted, and
            // the input is the record's: not reached. Nothing is run.
            Err(answer) => {
                return Ran {
                    answer,
                    standing: Standing::Stopped {
                        journal: accepted.journal_row(service),
                    },
                };
            }
        };
        let request = PublishCommentRequest {
            comment: SubmitCommentRequest {
                target: AuthoringTarget {
                    root: accepted.root(),
                    kind: self.kind,
                    item_id: self.id.clone(),
                    intent: ContextIntent::Edit,
                    operation_id: accepted.operation_id,
                },
                comment_id: self.comment_id.clone(),
                parent_id: self.parent_id.clone(),
                body: self.body.clone(),
                expected_destination: accepted.expectation(),
            },
            // A host is approved by its own command, and a merge commits
            // with the repository's own identity.
            approval: None,
            confirmed_identity: None,
        };
        let mut child = None;
        let submitted =
            service.submit_comment_with(request, author, restart, true, &mut |request| {
                let synchronization = synchronization_of(accepted, request.operation_id);
                let (result, ran) = sync::of_a_comment(
                    call,
                    &synchronization,
                    request,
                    Subject::of(self, accepted),
                    Some(accepted.request_id),
                );
                child = Some(Child {
                    ran,
                    succeeded: matches!(result, Some(Ok(_))),
                });
                result
            });
        match submitted {
            Ok(CommentSubmissionOutcome::Saved {
                receipt,
                publication,
                indexing,
                context: _,
                checkpoint: _,
            }) => saved(&receipt, publication, indexing, child),
            Ok(CommentSubmissionOutcome::IdentityRequired { context: _ }) => Ran {
                // Nothing when the binding was released; the file, when an
                // earlier attempt wrote it.
                answer: Answer {
                    effects: comment_effects(service, accepted),
                    ..Answer::stopped(ResultCode::IdentityRequired)
                },
                // Kept for the same request, which goes on once there is
                // an identity (R-34).
                standing: Standing::Owed,
            },
            Err(error) => self.refused(service, accepted, &error),
        }
    }

    /// The submission refused the request, or failed. Only the error's
    /// kind is read: its text can hold a path or a backend's message.
    ///
    /// - A refusal that released the binding it inserted, having written
    ///   nothing, keeps the record, as an identity stop does: the same
    ///   request binds afresh, and is answered as it is then (R-34;
    ///   controller ruling, Task 2b concern 2).
    /// - A request whose comment is checkpointed keeps its record, for the
    ///   same request to prove the checkpoint again.
    /// - Any other is the local row's to settle.
    fn refused(
        &self,
        service: &RepositoryService,
        accepted: &Accepted,
        error: &CommentSubmissionError,
    ) -> Ran {
        let effects = comment_effects(service, accepted);
        let standing = if error.released() || effects.checkpoint == CheckpointEffect::Committed {
            Standing::Owed
        } else {
            Standing::Stopped {
                journal: accepted.journal_row(service),
            }
        };
        Ran {
            answer: Answer {
                effects,
                ..Answer::stopped(outcome::repository_error_code(error.kind))
            },
            standing,
        }
    }
}

/// The answer to a comment that was checkpointed: the request's own write
/// and commit, however this call went, and what became of its
/// publication (R-6).
fn saved(
    receipt: &CommentReceipt,
    publication: CommentPublicationState,
    indexing: CommentIndexingState,
    child: Option<Child>,
) -> Ran {
    let mut effects = Effects {
        write: WriteEffect::Written,
        checkpoint: CheckpointEffect::Committed,
        discovery: if indexing.local_pending || indexing.remote_pending {
            DiscoveryEffect::Pending
        } else {
            DiscoveryEffect::Current
        },
        publication: PublicationEffect::Pending,
        commit_oid: Some(receipt.checkpoint_oid.to_string()),
        ..Effects::not_requested()
    };
    // What the synchronization merged, and whether what it published is
    // still to be handed to the index.
    if let Some(child) = &child {
        effects.integration = child.ran.answer.effects.integration;
        if child.ran.answer.effects.discovery == DiscoveryEffect::Pending {
            effects.discovery = DiscoveryEffect::Pending;
        }
    }
    // A publication that holds the comment: the request's end state, but
    // for the publication's index hand-off when that is owed, which the
    // same request does.
    let published = |effect| {
        if indexing.remote_pending {
            (effect, ResultCode::DiscoveryPending, Standing::Owed)
        } else {
            (effect, ResultCode::Ok, Standing::Final)
        }
    };
    let mut conflict = None;
    let (code, reason, standing) = match publication {
        CommentPublicationState::Published { oid: _ } => {
            let (effect, code, standing) = published(PublicationEffect::Published);
            effects.publication = effect;
            (code, None, standing)
        }
        CommentPublicationState::AlreadyCurrent { oid: _ } => {
            let (effect, code, standing) = published(PublicationEffect::Current);
            effects.publication = effect;
            (code, None, standing)
        }
        // Nowhere to publish to: the comment is saved, which is the
        // request's work, and a later `item sync` publishes it.
        CommentPublicationState::Pending {
            reason: CommentPublicationPendingReason::NoPublicationRemote,
        } => (
            ResultCode::Ok,
            Some(CommentPendingReason::NoPublicationRemote),
            Standing::Final,
        ),
        // The comment's own index hand-off, or the proof of its
        // checkpoint, comes first; the same request does both.
        CommentPublicationState::Pending {
            reason: CommentPublicationPendingReason::LocalRecoveryRequired,
        } => (
            ResultCode::RecoveryRequired,
            Some(CommentPendingReason::RecoveryRequired),
            Standing::Owed,
        ),
        CommentPublicationState::Pending {
            reason: CommentPublicationPendingReason::Synchronization(error),
        } => match child {
            // The synchronization stopped: its answer is this request's.
            Some(Child {
                ran,
                succeeded: false,
            }) => {
                let Ran { answer, standing } = ran;
                // A push the remote took before the stop is published.
                if matches!(
                    answer.effects.publication,
                    PublicationEffect::Published | PublicationEffect::Current
                ) {
                    effects.publication = answer.effects.publication;
                }
                if let Some(MutationDataDto::SynchronizationConflict(data)) = answer.data {
                    conflict = Some(data.conflict);
                }
                (
                    answer.code,
                    Some(CommentPendingReason::Synchronization),
                    standing_of(standing),
                )
            }
            // It published, and what it published does not hold the
            // comment: nothing the same request changes.
            Some(Child {
                ran: _,
                succeeded: true,
            }) => (
                ResultCode::RecoveryRequired,
                Some(CommentPendingReason::Synchronization),
                Standing::Final,
            ),
            // It was not called: the comment is not where its checkpoint
            // left it, or its synchronization cannot be read. The same
            // request looks again.
            None => (
                outcome::synchronization_error_code(&error),
                Some(CommentPendingReason::Synchronization),
                Standing::Owed,
            ),
        },
    };
    let pending_reason = reason.filter(|_| effects.publication == PublicationEffect::Pending);
    let data = CommentMutationDto {
        r#type: CommentMutationType::Comment,
        item_id: receipt.item_id.to_string(),
        comment_id: receipt.comment_id.to_string(),
        parent_id: receipt.parent_id.as_ref().map(ToString::to_string),
        created_by: checkpointed_author(receipt),
        synchronization_id: receipt.synchronization_id.to_string(),
        publication: effects.publication,
        pending_reason,
        conflict,
    };
    Ran {
        answer: Answer {
            effects,
            ..Answer::stopped(code)
        }
        .with_data(MutationDataDto::Comment(data)),
        standing,
    }
}

impl Binding for CommentAddBinding {
    fn command(&self) -> Command {
        Command::CommentAdd
    }

    /// The comment's operation completes its row whether it checkpointed
    /// or stopped for an identity with its file written, and its
    /// publication is another row's: a completed row says nothing of the
    /// request. The record is kept, and the domain is called again.
    fn ended_without_checkpoint(&self) -> EndedWithoutCheckpoint {
        EndedWithoutCheckpoint::SaysNothing
    }

    fn place(&self) -> Place<'_> {
        Place::Repository(&self.root)
    }

    /// The item, with its kind.
    fn target(&self) -> String {
        format!("{}/{}", authoring_kind_segment(&self.kind), self.id)
    }

    fn fields(&self, builder: IntentDigestBuilder) -> IntentDigestBuilder {
        // Every field is named. One that is added does not compile until
        // it is a field of the digest or is left out here.
        let Self {
            // The command, the repository and the target are what the
            // digest is begun with.
            root: _,
            kind: _,
            id: _,
            comment_id,
            parent_id,
            body,
            created_by,
        } = self;
        let comment_id = comment_id.to_string();
        let parent_id = parent_id.as_ref().map(ToString::to_string);
        builder
            .field("comment_id", FieldValue::Text(&comment_id))
            .field(
                "parent_id",
                match &parent_id {
                    Some(parent) => FieldValue::Text(parent),
                    None => FieldValue::Null,
                },
            )
            .field("body", FieldValue::Bytes(body.as_bytes()))
            .field(
                "created_by",
                match created_by {
                    Some(author) => FieldValue::Text(author),
                    None => FieldValue::Absent,
                },
            )
    }

    fn scope(&self, repository: Option<String>, position: Option<&Position>) -> Scope {
        item_scope(&self.kind, &self.id.to_string(), repository, position)
    }

    /// The author is read, and the repository resolved. Everything else
    /// is the domain's to check, under its lease: the item's context, a
    /// conflict pending in it, an unfinished local synchronization, a
    /// lifecycle-closed ticket, the parent, and the comment's ID. A fresh
    /// request expects the comment's path to be missing (fact 19).
    fn prepare(&mut self, call: &CallContext<'_>) -> Result<Prepared, Answer> {
        self.author()?;
        let repository = resolve_root(call.service, &self.root)?;
        prepared(
            repository.root(),
            &self.kind,
            &self.id.to_string(),
            ExpectedPathObservation::Missing,
        )
    }

    fn run(&self, call: &CallContext<'_>, accepted: &Accepted) -> Ran {
        self.submit(call, accepted, false)
    }

    /// Nothing is read here and nothing is answered: whatever the row
    /// holds, the submission is called with the full input and `restart`.
    /// It proves a checkpoint the request made from Git and publishes
    /// it, writing nothing; a request that wrote nothing it writes once.
    fn reenter(
        &mut self,
        _call: &CallContext<'_>,
        _accepted: &Accepted,
        _journal: JournalRow,
    ) -> Result<Entered, Ran> {
        Ok(Entered::Unfinished)
    }

    fn as_confirmed(&mut self) -> Option<&mut dyn Confirmed> {
        None
    }

    fn rerun(&self, call: &CallContext<'_>, accepted: &Accepted, _entered: &Entered) -> Ran {
        self.submit(call, accepted, true)
    }
}
