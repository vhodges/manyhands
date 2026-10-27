//! `context prepare`: the work context an item is edited in, a branch and
//! a worktree, provisioned for an existing item (`edit`) or for an item
//! that does not exist yet (`create`). It needs no confirmation: it cuts a
//! branch from primary's head and checks it out, and writes no content.
//!
//! **Prechecks** run on a request's first call only, so that a request
//! that is entered again always continues to the domain: a check that
//! has changed since cannot strand its pending row.
//!
//! - `edit`: the item is read, and one that is absent (or is of another
//!   kind) is `item_not_found`; any other read error is the read's. A
//!   closed ticket is `ticket_closed`. With no worktree yet, the item's
//!   file on primary must be committed, since the context is cut from
//!   the commit: otherwise `worktree_not_clean`.
//! - `create`: the item must not be there, in either kind: one that is is
//!   `occupied_path`, and any other read error is the read's.
//!
//! **What the domain's answer comes to.** A context that was made, or a
//! pending row of this request that is finished, is `created`. One that
//! was there already is not. A failed hand-off to the index leaves the
//! row pending at `worktree_observed`: a resume answers
//! `original_request_required`, so the request's own retry finishes the
//! hand-off, and the answer is `discovery_pending` offering that retry
//! (R-77). Neither a write nor a checkpoint is requested.
//!
//! **Re-entry.** A row that is absent or pending is continued by the
//! domain under the same operation ID, which finishes what an earlier
//! call left. A row that completed records no checkpoint step (the journal
//! keeps the first step it recorded, which is the branch or worktree
//! one), so the boundary runs the request as new: the context is found
//! there, and the answer is `ok` with `created` false.

use std::path::PathBuf;

use crate::{
    canonical::ItemId,
    results::{DiscoveryEffect, Effects, ResultCode, Scope, absolute_path_string},
};

use super::{
    super::{
        super::{
            AuthoringKind, AuthoringTarget, ContextIntent, ContextProvisionOutcome, ItemContext,
            ItemDtoKind, OperationFamily, RepositoryService, ResolvedRepository,
            authoring_kind_segment,
            recovery::{FinalKind, JournalRow, LocalStep, PendingOperation},
        },
        command::Command,
        context::context_names,
        dto::{ContextMutationDto, MutationDataDto},
        evidence::Position,
        identity::{FieldValue, IntentDigestBuilder},
    },
    Accepted, Answer, Binding, CallContext, Confirmed, EndedWithoutCheckpoint, Entered, Place,
    Prepared, Ran, Standing, item_scope, refused, resolve_root, save, ticket,
};

/// What the context is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextPrepareIntent {
    /// A new item: its ID is absent from primary.
    Create,
    /// An item that exists on primary.
    Edit,
}

impl ContextPrepareIntent {
    fn name(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Edit => "edit",
        }
    }

    fn domain(self) -> ContextIntent {
        match self {
            Self::Create => ContextIntent::Create,
            Self::Edit => ContextIntent::Edit,
        }
    }
}

/// The input of `context prepare`: the root of a registered repository,
/// the item's kind and ID, and what the context is for.
#[derive(Clone, Debug)]
pub struct ContextPrepareInput {
    pub root: PathBuf,
    pub kind: AuthoringKind,
    pub id: ItemId,
    pub intent: ContextPrepareIntent,
}

pub(crate) struct ContextPrepareBinding {
    root: PathBuf,
    kind: AuthoringKind,
    id: ItemId,
    intent: ContextPrepareIntent,
}

impl From<ContextPrepareInput> for ContextPrepareBinding {
    fn from(input: ContextPrepareInput) -> Self {
        // Every field is named: one added to the input does not compile
        // until it is taken into the binding or left out here.
        let ContextPrepareInput {
            root,
            kind,
            id,
            intent,
        } = input;
        Self {
            root,
            kind,
            id,
            intent,
        }
    }
}

/// Whether a request that is unfinished may have made part of its
/// context: its row is pending with a step of the context recorded. A
/// context is shown by no effect. A completed row has no step to say
/// whether it made the context or found it, and reads as none.
pub(crate) fn changed_and_unfinished(journal: Option<&JournalRow>) -> bool {
    match journal {
        Some(JournalRow::Pending(PendingOperation::Local { state: _, step })) => {
            step.is_some_and(|step| {
                step.is(LocalStep::BranchObserved)
                    || step.is(LocalStep::WorktreeObserved)
                    || step.is(LocalStep::AuthoritativeObserved)
            })
        }
        Some(
            JournalRow::Absent
            | JournalRow::Pending(
                PendingOperation::Remote { phase: _ } | PendingOperation::KeyMaterial { phase: _ },
            )
            | JournalRow::Final {
                kind: _,
                owes_work: _,
                checkpointed: _,
            },
        )
        | None => false,
    }
}

impl ContextPrepareBinding {
    fn id_text(&self) -> String {
        self.id.to_string()
    }

    fn dto(&self, branch: &str, worktree: String, created: bool) -> MutationDataDto {
        MutationDataDto::Context(ContextMutationDto {
            id: self.id_text(),
            kind: authoring_kind_segment(&self.kind).to_owned(),
            branch: branch.to_owned(),
            worktree,
            created,
        })
    }

    fn dto_of(&self, context: &ItemContext, created: bool) -> MutationDataDto {
        let worktree = absolute_path_string(&context.worktree)
            .unwrap_or_else(|| context.worktree.to_string_lossy().into_owned());
        self.dto(&context.branch, worktree, created)
    }

    fn ok(data: MutationDataDto) -> Answer {
        Answer {
            effects: Effects {
                discovery: DiscoveryEffect::Current,
                ..Effects::not_requested()
            },
            ..Answer::stopped(ResultCode::Ok)
        }
        .with_data(data)
    }

    /// The one domain call, under the recorded operation ID, and what
    /// it came to.
    fn call(&self, service: &RepositoryService, accepted: &Accepted) -> Ran {
        let target = AuthoringTarget {
            root: accepted.root(),
            kind: self.kind,
            item_id: self.id.clone(),
            intent: self.intent.domain(),
            operation_id: accepted.operation_id,
        };
        match service.prepare_context(target) {
            Ok(ContextProvisionOutcome::Created(context)) => Ran {
                answer: Self::ok(self.dto_of(&context, true)),
                standing: Standing::Final,
            },
            Ok(ContextProvisionOutcome::Reused(context)) => Ran {
                answer: Self::ok(self.dto_of(&context, false)),
                standing: Standing::Final,
            },
            // The context is there and the index has not been told: the
            // row stays pending, and the hand-off is owed.
            Ok(ContextProvisionOutcome::IndexPending { context }) => Ran {
                answer: Answer {
                    effects: Effects {
                        discovery: DiscoveryEffect::Pending,
                        ..Effects::not_requested()
                    },
                    ..Answer::stopped(ResultCode::DiscoveryPending)
                }
                .with_data(self.dto_of(&context, true)),
                standing: Standing::Owed,
            },
            // Only the kind of an error is read: its text can hold a
            // path.
            Err(error) => Ran {
                answer: refused(error),
                standing: Self::left(service, accepted),
            },
        }
    }

    /// What is left of a request that was refused: its row says.
    ///
    /// No row, on a call that entered a record another call accepted,
    /// does not say that nothing is in flight: that call may hold the
    /// lease and be making the context. The record is kept, and the same
    /// request is its way out.
    fn left(service: &RepositoryService, accepted: &Accepted) -> Standing {
        match accepted.journal_row(service) {
            Some(JournalRow::Absent) if accepted.attempt > 1 => Standing::Unconfirmed,
            journal @ (Some(
                JournalRow::Absent
                | JournalRow::Pending(_)
                | JournalRow::Final {
                    kind: _,
                    owes_work: _,
                    checkpointed: _,
                },
            )
            | None) => Standing::Stopped { journal },
        }
    }

    fn item_kind(&self) -> ItemDtoKind {
        match self.kind {
            AuthoringKind::Document => ItemDtoKind::Document,
            AuthoringKind::Ticket => ItemDtoKind::Ticket,
        }
    }

    /// The prechecks of an edit; see the module's description.
    fn check_edit(
        &self,
        service: &RepositoryService,
        repository: &ResolvedRepository,
    ) -> Result<(), Answer> {
        let item = service
            .show_item(repository, &self.id)
            .map_err(|error| Answer::of_read(&error))?;
        let (true, Some(source)) = (item.kind == self.item_kind(), item.source.as_deref()) else {
            return Err(Answer::stopped(ResultCode::ItemNotFound));
        };
        ticket::refuse_closed(&item)?;
        save::primary_is_committed(
            repository.root(),
            &self.kind,
            &self.id_text(),
            &item.path,
            source.as_bytes(),
        )
    }

    /// The prechecks of a create: the ID is held by no item.
    fn check_create(
        &self,
        service: &RepositoryService,
        repository: &ResolvedRepository,
    ) -> Result<(), Answer> {
        match service.show_item(repository, &self.id) {
            Ok(_) => Err(Answer::stopped(ResultCode::OccupiedPath)),
            Err(error) if error.code() == ResultCode::ItemNotFound => Ok(()),
            Err(error) => Err(Answer::of_read(&error)),
        }
    }
}

impl Binding for ContextPrepareBinding {
    fn command(&self) -> Command {
        Command::ContextPrepare
    }

    /// A completed row without a checkpoint step found the context there
    /// already and did nothing.
    fn ended_without_checkpoint(&self) -> EndedWithoutCheckpoint {
        EndedWithoutCheckpoint::DidNothing
    }

    fn place(&self) -> Place<'_> {
        Place::Repository(&self.root)
    }

    fn target(&self) -> String {
        self.id_text()
    }

    fn fields(&self, builder: IntentDigestBuilder) -> IntentDigestBuilder {
        let Self {
            // What the digest is begun with, with the target.
            root: _,
            id: _,
            kind,
            intent,
        } = self;
        builder
            .field("kind", FieldValue::Text(authoring_kind_segment(kind)))
            .field("intent", FieldValue::Text(intent.name()))
    }

    fn scope(&self, repository: Option<String>, position: Option<&Position>) -> Scope {
        item_scope(&self.kind, &self.id_text(), repository, position)
    }

    fn prepare(&mut self, call: &CallContext<'_>) -> Result<Prepared, Answer> {
        let repository = resolve_root(call.service, &self.root)?;
        match self.intent {
            ContextPrepareIntent::Edit => self.check_edit(call.service, &repository)?,
            ContextPrepareIntent::Create => self.check_create(call.service, &repository)?,
        }
        let names = context_names(repository.root(), &self.kind, &self.id_text());
        // The branch is recorded where it will be cut from.
        let position = Position::of(repository.root(), &names.branch_ref)
            .map_err(|_| Answer::stopped(ResultCode::RepositoryInaccessible))?;
        Ok(Prepared {
            observation: None,
            family: Some(OperationFamily::Local),
            position,
            expected: None,
        })
    }

    fn run(&self, call: &CallContext<'_>, accepted: &Accepted) -> Ran {
        self.call(call.service, accepted)
    }

    /// See the module's description for what each row says.
    fn reenter(
        &mut self,
        _call: &CallContext<'_>,
        _accepted: &Accepted,
        journal: JournalRow,
    ) -> Result<Entered, Ran> {
        match journal {
            JournalRow::Absent | JournalRow::Pending(_) => Ok(Entered::Unfinished),
            // A completed row with no checkpoint is run as new before it
            // gets here, and no other local operation ends in another
            // way or owes work once it has completed. Nothing is run or
            // settled on a row that says so.
            JournalRow::Final {
                kind: FinalKind::Completed | FinalKind::Cancelled | FinalKind::RetainedForInspection,
                owes_work: _,
                checkpointed: _,
            } => Err(Ran {
                answer: Answer::stopped(ResultCode::InternalError),
                standing: Standing::NotRun { journal },
            }),
        }
    }

    fn as_confirmed(&mut self) -> Option<&mut dyn Confirmed> {
        None
    }

    fn rerun(&self, call: &CallContext<'_>, accepted: &Accepted, _entered: &Entered) -> Ran {
        self.call(call.service, accepted)
    }
}
