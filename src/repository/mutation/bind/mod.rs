//! Bindings: what connects one command to the boundary. A binding names
//! the command, its target and the fields of its identity, makes the
//! checks that need no lease, and makes the one domain call.
//!
//! The boundary does the rest for every binding alike: it looks the
//! request up, answers a finished one from its record, rejects a changed
//! one, accepts a new one and settles its record.

use std::path::{Path, PathBuf};

use crate::results::{Effects, RecoveryAction, ResultCode, Scope, absolute_path_string};

use super::{
    super::{
        AuthoringKind, ExpectedPathObservation, OperationFamily, OperationId, ReadError,
        RepositoryService, ResolvedRepository,
        recovery::{FinalKind, JournalRow},
    },
    command::Command,
    context::context_names,
    dto::{MutationDataDto, PreviewDetailsDto, PreviewEffect},
    evidence::{PathEvidence, Position},
    identity::{IntentDigestBuilder, RequestId, ScopeKey},
    records::{ConfirmationRecord, RequestOperation, RequestRecord},
};

pub(crate) mod document;
pub(crate) mod index;
pub(crate) mod key;
pub(crate) mod remote;
pub(crate) mod repo;
pub(crate) mod save;
pub(crate) mod ticket;

/// The scope of the results of a request that writes the item `id`, of
/// kind `kind`. The branch and the worktree are named once the request
/// is accepted, from its recorded position: before that neither may exist.
pub(crate) fn item_scope(
    kind: &AuthoringKind,
    id: &str,
    repository: Option<String>,
    position: Option<&Position>,
) -> Scope {
    let context = position.zip(repository.as_deref());
    Scope {
        item_id: Some(id.to_owned()),
        branch: context.and_then(|(position, _)| position.branch().map(str::to_owned)),
        worktree: context.and_then(|(_, root)| {
            absolute_path_string(&context_names(Path::new(root), kind, id).worktree)
        }),
        repository,
        remote: None,
    }
}

/// What a request that commits to the editing branch of the item `id` is
/// accepted with: its position on that branch, and what it expects of the
/// file its recorded expectation is about.
pub(crate) fn prepared(
    root: &Path,
    kind: &AuthoringKind,
    id: &str,
    expected: ExpectedPathObservation,
) -> Result<Prepared, Answer> {
    let position = Position::of(root, &context_names(root, kind, id).branch_ref)
        .map_err(|_| Answer::stopped(ResultCode::RepositoryInaccessible))?;
    Ok(Prepared {
        family: Some(OperationFamily::Local),
        position,
        expected: Some(expected),
        observation: None,
    })
}

/// The repository a mutation names by its root. A linked worktree
/// resolves to its owner for a read; here it would give the request a
/// scope that is not the repository's.
pub(crate) fn resolve_root(
    service: &RepositoryService,
    root: &Path,
) -> Result<ResolvedRepository, Answer> {
    let repository = service
        .resolve_repository(root)
        .map_err(|error| Answer::of_read(&error))?;
    if std::fs::canonicalize(root).ok().as_deref() != Some(repository.root()) {
        return Err(Answer::stopped(ResultCode::NotRepositoryRoot));
    }
    Ok(repository)
}

/// What one call of the boundary gives a binding to work with, in each of
/// `prepare`, `run`, `reenter` and `rerun`.
///
/// It is the one way anything of the call reaches a binding: what a
/// command needs of its call is a field here, and no binding is given a
/// parameter of its own. The sink a call's progress goes to, the token
/// that stops it and the session's credentials are the call's as well,
/// and each is added here with the first command that reads it.
pub(crate) struct CallContext<'a> {
    pub(crate) service: &'a RepositoryService,
    /// The consent the request runs with: the confirmation its first
    /// call presents, once the boundary has found it to be this
    /// request's to use, and on a later call the one its record holds.
    /// `None` for a request of a command that needs none, and in
    /// `prepare`, which is what makes one. The boundary reads the time
    /// a confirmation is judged by; a binding is given none.
    pub(crate) confirmation: Option<&'a ConfirmationRecord>,
}

/// Where a request works: what its scope key is made from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Place<'a> {
    /// One repository, by its root as the caller gave it.
    Repository(&'a Path),
    /// A repository the request is to make, by the root it will have.
    /// Nothing need be there yet: the scope key is made from the parent,
    /// which exists, and is the root's own once the root does.
    NewRepository(&'a Path),
    /// The application: a command that belongs to no repository, as the
    /// key commands do. Its record's scope is the literal `application`,
    /// and its results name no repository, item, branch or worktree.
    Application,
}

/// What a request is answered with, before the boundary classifies it: the
/// outcome is derived from the code and the effects, never chosen here.
pub(crate) struct Answer {
    pub(crate) code: ResultCode,
    pub(crate) effects: Effects,
    pub(crate) data: Option<MutationDataDto>,
    /// The recovery a read suggested for its own failure. The boundary
    /// adds the actions that follow from the code.
    pub(crate) recovery: Vec<RecoveryAction>,
    /// The request made a durable change that no effect shows: a remote,
    /// which is no canonical content and no commit. A request that
    /// stopped after one is `partial`, as after a durable effect.
    pub(crate) local_change: bool,
}

impl Answer {
    /// A request that stopped having done nothing.
    pub(crate) fn stopped(code: ResultCode) -> Self {
        Self {
            code,
            effects: Effects::not_requested(),
            data: None,
            recovery: Vec::new(),
            local_change: false,
        }
    }

    /// A check's read failed: the request is answered with the read's
    /// code and the recovery it suggests.
    pub(crate) fn of_read(error: &ReadError) -> Self {
        Self {
            recovery: error.recovery.clone(),
            ..Self::stopped(error.code())
        }
    }

    pub(crate) fn with_data(mut self, data: MutationDataDto) -> Self {
        self.data = Some(data);
        self
    }
}

/// What a binding's checks give the boundary to accept a request with.
pub(crate) struct Prepared {
    /// The journal the binding's domain operation is recorded in. `None`
    /// for a command whose domain call has no operation ID and no journal
    /// row: a single step that happened or did not, as a change to the
    /// key registry is.
    pub(crate) family: Option<OperationFamily>,
    pub(crate) position: Position,
    /// The digest of the canonical file's bytes as they were before the
    /// request, which the domain operation is given as its expectation.
    pub(crate) expected: Option<ExpectedPathObservation>,
    /// For a command that needs confirmation, the digest of the state
    /// the request found: the boundary accepts the request only when it
    /// is the digest its confirmation holds. `None` for every other
    /// command.
    pub(crate) observation: Option<String>,
}

/// What a preview shows of a command that needs confirmation, and the
/// digest of the state it observed.
pub(crate) struct Previewed {
    pub(crate) observation: String,
    pub(crate) effects: Vec<PreviewEffect>,
    pub(crate) details: PreviewDetailsDto,
}

/// A binding of a command that needs confirmation: what `prepare` asks
/// of it. The rest of what it owes is in `Binding`:
///
/// - Its `Binding::prepare` makes no check of the state the preview
///   covers. It observes that state and gives its digest in
///   `Prepared::observation`, and the boundary refuses a request whose
///   digest is not its confirmation's as `external_change`. A check
///   there would answer a stale preview with the check's own code.
/// - Its `Binding::reenter` first asks whether the command's end state
///   holds, and finishes the request when it does. Otherwise it observes
///   again and compares with `CallContext::confirmation`, unless the
///   request's operation has recorded a step: from then on the request's
///   own work has changed what would be observed.
pub(crate) trait Confirmed: Binding {
    /// Validates the input, observes, and makes the checks the command's
    /// domain operation would refuse it for. It changes nothing and takes
    /// no lease. An `Err` answers `prepare` with no confirmation: a
    /// rejection, or a no-op when the command's end state already holds.
    fn preview(&mut self, call: &CallContext<'_>) -> Result<Previewed, Answer>;
}

/// An accepted request, as the call that is running it holds it. A first
/// call makes one from what it inserted; a call that re-enters an accepted
/// record makes one from the record and the attempt it raised.
pub(crate) struct Accepted {
    pub(crate) request_id: RequestId,
    /// This call's attempt. Every change to the record is conditional on
    /// it still being the record's.
    pub(crate) attempt: u64,
    pub(crate) scope: ScopeKey,
    /// The journal of the request's operation, or `None` for a request
    /// that has no operation; see `Prepared::family`.
    pub(crate) family: Option<OperationFamily>,
    /// The ID the request's domain operation runs under. For a request
    /// with no operation it names nothing: no record and no result holds
    /// it, and `operation` is what both are given.
    pub(crate) operation_id: OperationId,
    pub(crate) position: Position,
    pub(crate) expected: Option<ExpectedPathObservation>,
}

impl Accepted {
    /// An accepted record, entered as `attempt`.
    ///
    /// `None` for a record that does not hold what its command's records
    /// hold: an operation for a command that has one, and none for a
    /// command that has none. No call of this build accepts such a
    /// record, and nothing is run, answered or settled from it: a request
    /// that lost its operation would otherwise be entered again under an
    /// operation ID that is not its own. A record of a command this build
    /// does not bind is read as one of a command that has an operation.
    pub(crate) fn of_record(record: &RequestRecord, attempt: u64) -> Option<Self> {
        let operation = record.operations.first();
        let has_operation = Command::parse(&record.command).is_none_or(Command::has_operation);
        if operation.is_some() != has_operation {
            return None;
        }
        Some(Self {
            request_id: record.request_id,
            attempt,
            scope: record.scope.clone(),
            family: operation.map(|operation| operation.family),
            operation_id: operation
                .map_or_else(OperationId::new, |operation| operation.operation_id),
            position: Position {
                base_ref: record.base_ref.clone(),
                base_oid: record.base_oid,
            },
            expected: record.expected_digest.clone(),
        })
    }

    /// The request's operation, as its record and its results name it:
    /// none for a request that has no operation.
    pub(crate) fn operation(&self) -> Option<RequestOperation> {
        self.family.map(|family| RequestOperation {
            family,
            operation_id: self.operation_id,
        })
    }

    /// The repository's root as it is registered: what the scope key
    /// holds.
    pub(crate) fn root(&self) -> PathBuf {
        PathBuf::from(self.scope.as_str())
    }

    /// What the domain operation is to expect at the request's path: the
    /// recorded expectation.
    pub(crate) fn expectation(&self) -> ExpectedPathObservation {
        self.expected
            .clone()
            .unwrap_or(ExpectedPathObservation::Missing)
    }

    /// Where the request's operation stands in its journal. `None` when
    /// the lookup itself failed.
    pub(crate) fn journal_row(&self, service: &RepositoryService) -> Option<JournalRow> {
        self.read_journal_row(service).ok()
    }

    /// Where the request's operation stands in its journal, or why that
    /// could not be read. A request with no operation has no row, and
    /// nothing is in flight under it: it reads as absent, and no journal
    /// is asked.
    pub(crate) fn read_journal_row(
        &self,
        service: &RepositoryService,
    ) -> Result<JournalRow, ReadError> {
        match self.family {
            Some(family) => service.journal_row(family, &self.scope, self.operation_id),
            None => Ok(JournalRow::Absent),
        }
    }
}

/// What is left of a request once its domain call has returned.
pub(crate) enum Standing {
    /// The request's end state holds: the result is stored.
    Final,
    /// The domain returned a result and says work remains that a retry
    /// continues: the record stays accepted.
    Owed,
    /// The request stopped. Whether anything is in flight is the journal
    /// row's to say: `journal` is the row as the binding read it after the
    /// call, or `None` when it could not be read.
    Stopped { journal: Option<JournalRow> },
    /// Git could not be read to say what the request committed: after a
    /// domain call that returned a result, or on re-entry before any call.
    /// Nothing is stored and nothing is deleted: a result finished now
    /// would be a no-op for a save that may have committed. The record
    /// stays accepted for a retry to settle.
    Unconfirmed,
    /// A re-entry stopped before its domain call, for a reason that says
    /// nothing of what an earlier attempt did. `journal` is the row as it
    /// was read: when no attempt had started, nothing is in flight and the
    /// record is deleted; otherwise the request's result is still owed and
    /// the record stays accepted, also when the row has completed.
    NotRun { journal: JournalRow },
}

/// A domain call's result and what it leaves.
pub(crate) struct Ran {
    pub(crate) answer: Answer,
    pub(crate) standing: Standing,
}

/// What re-entry reads before it makes its domain call: where the
/// request's operation stood in its journal, and what Git showed of the
/// request's path. The same two are what the commit rule needs after the
/// call, because the row's state after the call proves nothing: a repeat
/// resets a rejected row before the domain runs.
pub(crate) struct Before {
    pub(crate) journal: JournalRow,
    pub(crate) evidence: PathEvidence,
}

/// What a binding read on re-entry and found no answer in: it is handed
/// back for the domain call, whose result is judged by it. The boundary
/// reads none of it.
///
/// This is in place of the `Before` that `Binding::reenter` used to
/// return, which holds the evidence of a path. A command that commits to
/// no path has none to give.
pub(crate) enum Entered {
    /// An authoring write: the operation's row, and what Git showed of the
    /// request's path. A row without a checkpoint step is read the
    /// authoring way; see `Before::committing`.
    Authoring(Before),
    /// A command whose completed row has no checkpoint step whatever it
    /// did, and whose work its binding found not to be done, or leaves to
    /// its domain call to find: a key generation's call is itself the
    /// check of what an earlier attempt left. The call's result is judged
    /// by what the call returns and by the operation's row after it, so
    /// nothing read before it is kept.
    Unfinished,
}

impl Entered {
    /// What an authoring write read. A binding of one is never handed
    /// anything else: it is given back what its own `reenter` returned.
    /// Were it, nothing would be known of what the request committed, and
    /// nothing is run, finished or deleted on that.
    pub(crate) fn authoring(&self) -> Result<&Before, Ran> {
        match self {
            Self::Authoring(before) => Ok(before),
            Self::Unfinished => Err(Ran {
                answer: Answer::stopped(ResultCode::InternalError),
                standing: Standing::Unconfirmed,
            }),
        }
    }
}

/// What re-entry does about what it read before the domain call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Found {
    /// The request's work is done and nothing is in flight: its operation
    /// completed and its commit is in range. Nothing remains for the domain
    /// to complete, and calling it again could only be refused for what
    /// has happened to the file since. The request is not run again; its
    /// commit is reported, and a later change by someone else, committed
    /// or not, is left as it is.
    Done { commit: git2::Oid },
    /// A commit in range changed the path to something the request did
    /// not intend, and the request's work is not known to be done: the
    /// answer is `external_change` and the request is not run again.
    /// `own` is the commit an earlier attempt of the request made before
    /// that, when one had started and made one.
    Foreign { own: Option<git2::Oid> },
    /// Nothing stands in the way: the domain operation is called.
    Continue,
}

impl Before {
    /// Whether an earlier attempt of the request had started: its
    /// operation's journal row was pending, or final and not rejected. A
    /// rejected row reads as absent.
    pub(crate) fn started(&self) -> bool {
        match self.journal {
            JournalRow::Absent => false,
            JournalRow::Pending(_) | JournalRow::Final { .. } => true,
        }
    }

    /// Whether an earlier attempt of the request may have committed: its
    /// operation's journal row was pending, or was final having recorded a
    /// step of its work. A row that completed with none never reached a
    /// checkpoint, and no commit is that request's, whatever Git shows.
    ///
    /// This is the reading of an authoring save, whose operation records
    /// a checkpoint step on every row that did its work. It is wrong for
    /// a binding that answers `EndedWithoutCheckpoint::SaysNothing`: the
    /// completed rows of its operation never have the step, so this would
    /// say of work that was done that none of it is the request's. Such a
    /// binding must not use this, `found` or `reported`.
    pub(crate) fn committing(&self) -> bool {
        match self.journal {
            JournalRow::Absent => false,
            JournalRow::Pending(_) => true,
            JournalRow::Final { checkpointed, .. } => checkpointed,
        }
    }

    /// The request's own commit, as read before the domain call: a commit
    /// of the intended content is the request's own only when an attempt
    /// of the request may have committed.
    pub(crate) fn own(&self) -> Option<git2::Oid> {
        self.evidence.own().filter(|_| self.committing())
    }

    /// What re-entry does about what it read, for an authoring save.
    ///
    /// It rests on `committing`, and so on the authoring reading of a
    /// row without a checkpoint. For a binding that answers
    /// `EndedWithoutCheckpoint::SaysNothing` a completed row would never
    /// be `Done` here: it would be `Continue`, and the domain called a
    /// second time, or `Foreign`, and the request refused as
    /// `external_change`, for work that completed. Such a binding must
    /// not use this. It needs evidence of its own that its work is done,
    /// and answers from `Binding::reenter` with
    /// `Err(Ran { standing: Standing::Final, .. })` when it is.
    pub(crate) fn found(&self) -> Found {
        let own = self.own();
        match (&self.journal, own) {
            (
                JournalRow::Final {
                    kind: FinalKind::Completed,
                    owes_work: false,
                    checkpointed: _,
                },
                Some(commit),
            ) => Found::Done { commit },
            (
                JournalRow::Absent
                | JournalRow::Pending(_)
                | JournalRow::Final {
                    kind:
                        FinalKind::Completed | FinalKind::Cancelled | FinalKind::RetainedForInspection,
                    owes_work: _,
                    checkpointed: _,
                },
                own,
            ) => {
                if self.evidence.superseded(self.committing()) {
                    Found::Foreign { own }
                } else {
                    Found::Continue
                }
            }
        }
    }

    /// The commit a call reports, from the evidence read after its domain
    /// call.
    ///
    /// For an authoring save only: which commit can be the request's is
    /// decided by `committing`. A binding that answers
    /// `EndedWithoutCheckpoint::SaysNothing` must not use this.
    pub(crate) fn reported(&self, after: &PathEvidence) -> Option<git2::Oid> {
        after.reported(self.committing(), &self.evidence)
    }
}

/// What it says of a request that its operation's journal row is final,
/// completed, owes no work and never recorded a checkpoint step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EndedWithoutCheckpoint {
    /// The operation did nothing: it records a checkpoint step whenever it
    /// gets as far as its work, so a row without one stopped or was
    /// refused before it. The record is released and the call is run as a
    /// new request.
    DidNothing,
    /// Nothing: the operation's completed row has no checkpoint step
    /// whatever it did. The record is kept and the binding's `reenter`
    /// is called, as for any other row of an operation that has ended.
    ///
    /// The boundary does no more than that. What the request is answered
    /// with is the binding's to work out: it needs evidence of its own
    /// that its work is done, and when it is, it answers from `reenter`
    /// with `Err(Ran { standing: Standing::Final, .. })` and makes no
    /// domain call. `Before::committing`, `Before::found` and
    /// `Before::reported` are not that evidence: they read a row
    /// without a checkpoint as an authoring save's, and would have the
    /// domain called again or the request refused as `external_change`.
    SaysNothing,
}

pub(crate) trait Binding {
    /// The command. Its name is what the envelope gives.
    fn command(&self) -> Command;

    /// What a final row of this command's operation that never reached a
    /// checkpoint says of the request, when the request is entered again.
    ///
    /// There is no default, because neither answer is safe for a binding
    /// that has not thought about it. `DidNothing` is right only where
    /// the domain operation records a checkpoint step on every row that
    /// did its work, as an authoring save does: there, running the
    /// request as new is what settling would have done had the first
    /// answer not been lost. For an operation whose completed row never
    /// has such a step, an index refresh or a confirmed command among
    /// them, `DidNothing` would run completed work a second time after
    /// lost output, and send a confirmed command back through a
    /// confirmation it has already used. Those answer `SaysNothing`.
    fn ended_without_checkpoint(&self) -> EndedWithoutCheckpoint;

    /// Where the request works: a repository, by its root as the caller
    /// gave it, or the application.
    fn place(&self) -> Place<'_>;

    /// The request's target within its repository.
    fn target(&self) -> String;

    /// Adds the binding's typed inputs to the intent digest, in a fixed
    /// order, each under its name, with absent, null and empty kept
    /// distinct. A body is fed here and stored nowhere.
    fn fields(&self, builder: IntentDigestBuilder) -> IntentDigestBuilder;

    /// The scope of the request's results. `position` is the recorded
    /// position once the request is accepted; before that the result names
    /// no branch or worktree, since none may exist.
    fn scope(&self, repository: Option<String>, position: Option<&Position>) -> Scope;

    /// Validates the input and makes the checks that need no lease. An
    /// `Err` answers the request without accepting it: nothing is
    /// recorded and no domain call is made.
    fn prepare(&mut self, call: &CallContext<'_>) -> Result<Prepared, Answer>;

    /// Makes the domain call for an accepted request, under the recorded
    /// operation ID and expectation, and reports what the repository shows
    /// of it afterwards.
    fn run(&self, call: &CallContext<'_>, accepted: &Accepted) -> Ran;

    /// Re-entry, before the domain call: an earlier call of the request
    /// started and its end was not recorded. Sets what `prepare` worked
    /// out for the domain call, without the checks a retry skips: the
    /// caller's token described the file before the first attempt, whose own
    /// write is allowed to have changed it. Then reads the evidence the
    /// rules need. `journal` is the operation's row as the boundary read
    /// it before this call. It is never a row the boundary released the
    /// record for: see `ended_without_checkpoint`.
    ///
    /// An `Err` answers the request without a domain call, and its
    /// standing says what becomes of the record: the request can be
    /// finished here, when the evidence shows its work is done.
    ///
    /// `Before::found` decides this for an authoring save and for
    /// nothing else. A binding that answers
    /// `EndedWithoutCheckpoint::SaysNothing` is called here with a
    /// completed row that has no checkpoint step and must decide from
    /// evidence of its own whether its work is done; when it is, it
    /// returns `Err(Ran { standing: Standing::Final, .. })`.
    fn reenter(
        &mut self,
        call: &CallContext<'_>,
        accepted: &Accepted,
        journal: JournalRow,
    ) -> Result<Entered, Ran>;

    /// Re-entry, the domain call: made with the recorded operation ID and
    /// the recorded expectation, which is the only way the work is
    /// continued. `entered` is what this binding's `reenter` returned.
    /// For an authoring write the commit is taken from the evidence read
    /// before and after the call, never from what the domain names.
    fn rerun(&self, call: &CallContext<'_>, accepted: &Accepted, entered: &Entered) -> Ran;
}

/// What the repository shows of an unfinished request, without its input.
pub(crate) struct Recorded {
    pub(crate) effects: Effects,
    /// A durable change that no effect shows; see `Answer::local_change`.
    pub(crate) local_change: bool,
}

impl Recorded {
    fn nothing() -> Self {
        Self::of(Effects::not_requested())
    }

    fn of(effects: Effects) -> Self {
        Self {
            effects,
            local_change: false,
        }
    }
}

/// The effects the repository shows of the unfinished request `record`
/// holds. The original input is not needed, and is not available when the
/// request is being reused with another.
///
/// An earlier attempt ran when the operation's journal row is pending, or
/// final and not rejected: anything but absent. A completed row counts,
/// since a call can commit and complete its row and still fail to settle
/// its record. Then the effects are what Git and the item's context show.
/// With no row, no attempt reached the domain and there is no effect. A
/// row that could not be read is taken to be there: the effects still
/// have to be shown by the repository.
pub(crate) fn recorded_effects(service: &RepositoryService, record: &RequestRecord) -> Recorded {
    let Some(accepted) = Accepted::of_record(record, record.attempt) else {
        return Recorded::nothing();
    };
    let journal = accepted.journal_row(service);
    let ran = match journal {
        Some(JournalRow::Absent) => false,
        Some(JournalRow::Pending(_) | JournalRow::Final { .. }) | None => true,
    };
    if !ran {
        return Recorded::nothing();
    }
    match Command::parse(&record.command) {
        Some(Command::DocumentCreate | Command::DocumentSave | Command::DocumentMove) => {
            Recorded::of(document::recorded_effects(record, &accepted))
        }
        Some(Command::TicketCreate | Command::TicketSave | Command::TicketSlugAssign) => {
            Recorded::of(ticket::recorded_effects(record, &accepted))
        }
        // A remote is shown by no effect. Its row says whether it was
        // added with work remaining.
        Some(Command::RemoteAdd) => Recorded {
            effects: Effects::not_requested(),
            local_change: remote::added_and_unfinished(journal.as_ref()),
        },
        Some(Command::RemoteSelect) => Recorded::of(remote::recorded_effects(record, &accepted)),
        // A refresh or a rebuild in flight has made no durable effect.
        Some(Command::IndexRefresh | Command::IndexRebuild) => Recorded::nothing(),
        // A change to the key registry has no operation, so none is ever
        // found to have run: this is not reached, and nothing is claimed.
        Some(Command::KeyImport | Command::KeySelect | Command::KeyClear | Command::KeyRemove) => {
            Recorded::nothing()
        }
        // A key file is shown by no effect. The generation's row says
        // whether one was written with the generation not yet ended.
        Some(Command::KeyGenerate) => Recorded {
            effects: Effects::not_requested(),
            local_change: key::written_and_unfinished(journal.as_ref()),
        },
        Some(Command::RepoCreate | Command::RepoEnable) => {
            repo::recorded_effects(service, &accepted, journal.as_ref())
        }
        // A record of no bound command: nothing is known of its effects.
        None => Recorded::nothing(),
    }
}

#[cfg(test)]
#[path = "before_tests.rs"]
mod before_tests;
