//! `poll once`, the explicit poll, and `poll configure`, `poll pause` and
//! `poll resume`: one field of a registration's polling policy, written in the
//! request's own transaction (R-7, decision 15).
//!
//! **`poll once`** is one explicit fetch, clean fast-forward, materialization
//! and index pass. It skips eligibility, so a pause refuses nothing, and a pause
//! during a running poll applies to the next automatic one. The authoritative
//! report is classified first and index pending second, since `IndexPending`
//! wraps any completion: see `PollOnceBinding::reported`.
//!
//! **A stop after the reservation is final, and a new request polls (R-18).** The
//! domain finishes the row of a transport stop as `failed`, which the journal
//! reads as pending, so the answer's standing is `Final` and not the row's to
//! say. Only an interrupted pass is owed: the same request continues it with
//! `restart`. So does a poll that was killed, whose row was left as it was.
//!
//! **No accepted state.** The record is inserted and finished with the write, so
//! a request is absent or finished. A committed request replays. One that never
//! committed can't be told from one never sent, so sending it again is a new
//! write: last commit wins, deliberately, and its answer says what it did.
//!
//! **Nothing written** means the policy row: a finished request's record is
//! stored even when the policy was already so. Nothing needs Git, so a pending
//! local synchronization refuses nothing. A pause during a running poll applies
//! to the next automatic one. No verb sets `enabled`, and `poll resume` doesn't
//! lift the recovery suspension (decision 5): its data names one that is set.

use std::path::PathBuf;

use rusqlite::Transaction;
use time::OffsetDateTime;

use crate::results::{DiscoveryEffect, Effects, ResultCode, Scope, timestamp_string};

use super::{
    super::{
        super::{
            AuthoringKind, ConfigurationInspection, ItemDtoKind, OperationFamily, PollCompletion,
            PollDisposition, PollPublicationRemoteRequest, PollTargetReport, PollingInterval,
            ReadError, RemoteOperationPhase, RemoteOutcomeCategory, RemotePollError,
            RemotePollInvocation, RemotePollOutcome, RemotePollReport, RemotePollResult,
            RepositoryOperation, SynchronizationError, read_configuration_for,
            recovery::{JournalRow, PendingOperation},
            remote::state::{PollingChange, PollingUpdate, update_remote_polling},
        },
        command::Command,
        dto::{
            MutationDataDto, PollMutationCompletion, PollMutationDto, PollPolicyMutationDto,
            PollTargetDisposition, PollTargetDto,
        },
        evidence::Position,
        identity::{FieldValue, IntentDigestBuilder, ScopeKey},
        outcome,
    },
    Accepted, Answer, Binding, CallContext, Confirmed, EndedWithoutCheckpoint, Entered,
    OneTransaction, Place, Prepared, Ran, Standing, refused, resolve_root, sync,
};

/// The input of `poll configure`: the interval between automatic polls.
pub struct PollConfigureInput {
    pub root: PathBuf,
    pub interval_seconds: u64,
}

/// The input of `poll pause`: the repository.
pub struct PollPauseInput {
    pub root: PathBuf,
}

/// The input of `poll resume`: the repository.
pub struct PollResumeInput {
    pub root: PathBuf,
}

enum Verb {
    /// The interval as the caller gave it: `prepare` is what refuses one out
    /// of range, and the request's identity holds it either way.
    Configure(u64),
    Pause,
    Resume,
}

pub(crate) struct PollPolicyBinding {
    root: PathBuf,
    verb: Verb,
}

impl From<PollConfigureInput> for PollPolicyBinding {
    fn from(input: PollConfigureInput) -> Self {
        let PollConfigureInput {
            root,
            interval_seconds,
        } = input;
        Self {
            root,
            verb: Verb::Configure(interval_seconds),
        }
    }
}

impl From<PollPauseInput> for PollPolicyBinding {
    fn from(input: PollPauseInput) -> Self {
        let PollPauseInput { root } = input;
        Self {
            root,
            verb: Verb::Pause,
        }
    }
}

impl From<PollResumeInput> for PollPolicyBinding {
    fn from(input: PollResumeInput) -> Self {
        let PollResumeInput { root } = input;
        Self {
            root,
            verb: Verb::Resume,
        }
    }
}

impl PollPolicyBinding {
    /// What the request changes. `None` for an interval out of range.
    fn change(&self) -> Option<PollingChange> {
        match self.verb {
            Verb::Configure(seconds) => PollingInterval::from_seconds(seconds)
                .ok()
                .map(PollingChange::Interval),
            Verb::Pause => Some(PollingChange::Pause),
            Verb::Resume => Some(PollingChange::Resume),
        }
    }
}

/// The policy as the request's transaction left it.
fn data(update: &PollingUpdate) -> Result<PollPolicyMutationDto, ReadError> {
    let polling = &update.polling;
    Ok(PollPolicyMutationDto {
        enabled: polling.enabled(),
        paused: polling.paused(),
        interval_seconds: polling.interval().as_secs(),
        backoff_seconds: polling.automatic_backoff().map(|delay| delay.as_secs()),
        recovery_suspended: polling.recovery_suspended(),
        next_eligible_at: update
            .next_eligible_at
            .map(|time| {
                timestamp_string(time).ok_or_else(|| ReadError::new(ResultCode::InternalError))
            })
            .transpose()?,
    })
}

impl OneTransaction for PollPolicyBinding {
    fn write(&self, transaction: &Transaction<'_>, scope: &ScopeKey) -> Result<Answer, ReadError> {
        let change = self
            .change()
            .ok_or_else(|| ReadError::new(ResultCode::InvalidInput))?;
        // A repository error keeps a mutation's code for it: the conversion to
        // a `ReadError` is a read's, and calls most of them internal.
        let update = update_remote_polling(transaction, scope.as_str(), change)
            .map_err(|error| ReadError::new(outcome::repository_error_code(error.kind)))?;
        let code = if update.changed {
            ResultCode::Ok
        } else {
            ResultCode::AlreadyApplied
        };
        Ok(Answer::stopped(code).with_data(MutationDataDto::PollPolicy(data(&update)?)))
    }
}

impl Binding for PollPolicyBinding {
    fn command(&self) -> Command {
        match self.verb {
            Verb::Configure(_) => Command::PollConfigure,
            Verb::Pause => Command::PollPause,
            Verb::Resume => Command::PollResume,
        }
    }

    /// There is no row of any kind. No call of this build leaves a record
    /// accepted; `reenter` releases one that some other build did.
    fn ended_without_checkpoint(&self) -> EndedWithoutCheckpoint {
        EndedWithoutCheckpoint::SaysNothing
    }

    fn place(&self) -> Place<'_> {
        Place::Repository(&self.root)
    }

    /// The repository as a whole: the command names nothing within it.
    fn target(&self) -> String {
        String::new()
    }

    fn fields(&self, builder: IntentDigestBuilder) -> IntentDigestBuilder {
        match self.verb {
            Verb::Configure(seconds) => {
                builder.field("interval_seconds", FieldValue::Text(&seconds.to_string()))
            }
            Verb::Pause | Verb::Resume => builder,
        }
    }

    fn scope(&self, repository: Option<String>, _position: Option<&Position>) -> Scope {
        Scope {
            repository,
            ..Scope::default()
        }
    }

    /// An interval out of range and an unregistered or unenabled repository are
    /// refused here, with nothing recorded. Whether the policy is already so is
    /// read in the transaction.
    fn prepare(&mut self, call: &CallContext<'_>) -> Result<Prepared, Answer> {
        if self.change().is_none() {
            return Err(Answer::stopped(ResultCode::InvalidInput));
        }
        resolve_root(call.service, &self.root)?;
        Ok(Prepared {
            family: None,
            // Nothing is committed, so there is no branch to bound.
            position: Position {
                base_ref: None,
                base_oid: None,
            },
            expected: None,
            observation: None,
        })
    }

    fn one_transaction(&self) -> Option<&dyn OneTransaction> {
        Some(self)
    }

    /// Not reached by this build's own records: see `one_transaction`. For an
    /// accepted record it did not write, no policy is written; the record is
    /// released, and the next call runs as new.
    fn run(&self, _call: &CallContext<'_>, _accepted: &Accepted) -> Ran {
        Ran {
            answer: Answer::stopped(ResultCode::InternalError),
            standing: Standing::NotRun {
                journal: JournalRow::Absent,
            },
        }
    }

    fn reenter(
        &mut self,
        _call: &CallContext<'_>,
        _accepted: &Accepted,
        journal: JournalRow,
    ) -> Result<Entered, Ran> {
        Err(Ran {
            answer: Answer::stopped(ResultCode::InternalError),
            standing: Standing::NotRun { journal },
        })
    }

    fn as_confirmed(&mut self) -> Option<&mut dyn Confirmed> {
        None
    }

    fn rerun(&self, call: &CallContext<'_>, accepted: &Accepted, _entered: &Entered) -> Ran {
        self.run(call, accepted)
    }
}

/// The input of `poll once`: the repository.
pub struct PollOnceInput {
    pub root: PathBuf,
}

pub(crate) struct PollOnceBinding {
    root: PathBuf,
}

impl From<PollOnceInput> for PollOnceBinding {
    fn from(input: PollOnceInput) -> Self {
        let PollOnceInput { root } = input;
        Self { root }
    }
}

fn dto_disposition(disposition: PollDisposition) -> PollTargetDisposition {
    match disposition {
        PollDisposition::Current => PollTargetDisposition::Current,
        PollDisposition::FastForwarded => PollTargetDisposition::FastForwarded,
        PollDisposition::LocalAhead => PollTargetDisposition::LocalAhead,
        PollDisposition::Diverged => PollTargetDisposition::Diverged,
        PollDisposition::Dirty => PollTargetDisposition::Dirty,
        PollDisposition::Conflicted => PollTargetDisposition::Conflicted,
        PollDisposition::Inaccessible => PollTargetDisposition::Inaccessible,
        PollDisposition::Materialized => PollTargetDisposition::Materialized,
        PollDisposition::Nonconforming => PollTargetDisposition::Nonconforming,
        PollDisposition::BlockedByLocalState => PollTargetDisposition::BlockedByLocalState,
        PollDisposition::MaterializationIncomplete => {
            PollTargetDisposition::MaterializationIncomplete
        }
        PollDisposition::Malformed => PollTargetDisposition::Malformed,
        PollDisposition::RemotelyDeleted => PollTargetDisposition::RemotelyDeleted,
        PollDisposition::HistoryUnknown => PollTargetDisposition::HistoryUnknown,
        PollDisposition::NotReached => PollTargetDisposition::NotReached,
    }
}

fn dto_target(target: &PollTargetReport) -> PollTargetDto {
    PollTargetDto {
        target: target
            .target()
            .map(|reference| reference.remote_ref().to_owned()),
        kind: target.kind().map(|kind| match kind {
            AuthoringKind::Document => ItemDtoKind::Document,
            AuthoringKind::Ticket => ItemDtoKind::Ticket,
        }),
        item_id: target.item_id().map(ToString::to_string),
        disposition: dto_disposition(target.disposition()),
        before_oid: target.before_oid().map(|oid| oid.to_string()),
        after_oid: target.after_oid().map(|oid| oid.to_string()),
    }
}

/// The report as the result's data. `None` for a time that cannot be written.
fn poll_data(report: &RemotePollReport) -> Option<PollMutationDto> {
    let time = |at: OffsetDateTime| timestamp_string(at);
    Some(PollMutationDto {
        operation_id: report.operation_id().to_string(),
        completion: match report.completion() {
            PollCompletion::Completed => PollMutationCompletion::Completed,
            PollCompletion::Interrupted => PollMutationCompletion::Interrupted,
            PollCompletion::Cancelled => PollMutationCompletion::Cancelled,
        },
        completed_at: time(report.completed_at())?,
        next_eligible_at: match report.next_eligible_at() {
            Some(at) => Some(time(at)?),
            None => None,
        },
        targets: report.targets().iter().map(dto_target).collect(),
    })
}

/// Whether a target of the report was moved or made: an interrupted pass that
/// did so is partial.
fn moved(target: &PollTargetReport) -> bool {
    matches!(
        target.disposition(),
        PollDisposition::FastForwarded | PollDisposition::Materialized
    ) || target.before_oid() != target.after_oid()
}

/// The code of a stored category, which a later call reads where the first
/// call's error named the stop by its own code (design, Polling Verbs).
fn category_code(category: RemoteOutcomeCategory) -> ResultCode {
    match category {
        RemoteOutcomeCategory::Completed => ResultCode::Ok,
        RemoteOutcomeCategory::ConfigurationRequired => ResultCode::PublicationRemoteRequired,
        RemoteOutcomeCategory::SelectedKeyUnavailable => ResultCode::NoSelectedKey,
        RemoteOutcomeCategory::UnlockRequired => ResultCode::UnlockRequired,
        RemoteOutcomeCategory::HostApprovalRequired => ResultCode::HostApprovalRequired,
        RemoteOutcomeCategory::TransportUnavailable => ResultCode::TransportUnavailable,
        RemoteOutcomeCategory::ProtocolRejected => ResultCode::RemoteUnavailable,
        RemoteOutcomeCategory::Cancelled => ResultCode::Cancelled,
        RemoteOutcomeCategory::RepositoryUnavailable => ResultCode::RepositoryInaccessible,
    }
}

impl PollOnceBinding {
    /// The one domain call, under the recorded operation ID. `restart` is set
    /// by a call that enters the request again: it continues a pass that was
    /// interrupted or killed, and changes nothing for a row that has ended,
    /// whose stored result the domain returns, or for no row.
    fn call(&self, call: &CallContext<'_>, accepted: &Accepted, restart: bool) -> Ran {
        let request = PollPublicationRemoteRequest {
            root: accepted.root(),
            operation_id: accepted.operation_id,
            invocation: RemotePollInvocation::Explicit,
            // A host is approved by its own command, never in passing.
            approval: None,
            restart,
        };
        match call
            .session
            .poll_once(call.service, request, accepted.request_id)
        {
            None => Self::by_the_row(call, accepted, Answer::stopped(ResultCode::InternalError)),
            Some(Ok(result)) => Self::reported(call, accepted, result),
            Some(Err(error)) => Self::stopped(call, accepted, error),
        }
    }

    /// An answer whose record the operation's row settles: with no row nothing
    /// is in flight, and a row that cannot be read leaves the record.
    fn by_the_row(call: &CallContext<'_>, accepted: &Accepted, answer: Answer) -> Ran {
        Ran {
            answer,
            standing: Standing::Stopped {
                journal: accepted.journal_row(call.service),
            },
        }
    }

    /// The domain's report: the authoritative result first, and whether the
    /// index is behind second.
    fn reported(call: &CallContext<'_>, accepted: &Accepted, result: RemotePollResult) -> Ran {
        let (outcome, pending) = match result {
            RemotePollResult::Complete(outcome) => (outcome, false),
            RemotePollResult::IndexPending(pending) => (pending.authoritative, true),
        };
        let report = match outcome {
            RemotePollOutcome::Polled(report) => report,
            // An explicit poll is never asked whether it is eligible.
            RemotePollOutcome::NotEligible { .. } => {
                let answer = Answer::stopped(ResultCode::InternalError);
                return Self::by_the_row(call, accepted, answer);
            }
        };
        let Some(data) = poll_data(&report) else {
            let answer = Answer::stopped(ResultCode::InternalError);
            return Self::by_the_row(call, accepted, answer);
        };
        let discovery = |otherwise| {
            if pending {
                DiscoveryEffect::Pending
            } else {
                otherwise
            }
        };
        let (code, discovery, local_change, standing) = match report.completion() {
            PollCompletion::Cancelled => {
                // A failure to clear it leaves a flag on a record that is
                // finished below, where nothing reads it.
                let _ = call.service.take_cancellation(accepted.request_id);
                (
                    ResultCode::Cancelled,
                    discovery(DiscoveryEffect::NotRequested),
                    false,
                    Standing::Final,
                )
            }
            PollCompletion::Interrupted => (
                ResultCode::RecoveryRequired,
                discovery(DiscoveryEffect::NotRequested),
                report.targets().iter().any(moved),
                Standing::Owed,
            ),
            PollCompletion::Completed => match report.outcome() {
                None => {
                    let answer = Answer::stopped(ResultCode::InternalError);
                    return Self::by_the_row(call, accepted, answer);
                }
                Some(RemoteOutcomeCategory::Completed) if pending => (
                    ResultCode::DiscoveryPending,
                    DiscoveryEffect::Pending,
                    false,
                    Standing::Final,
                ),
                Some(RemoteOutcomeCategory::Completed) => (
                    ResultCode::Ok,
                    DiscoveryEffect::Current,
                    false,
                    Standing::Final,
                ),
                Some(category) => (
                    category_code(category),
                    discovery(DiscoveryEffect::NotRequested),
                    false,
                    Standing::Final,
                ),
            },
        };
        Ran {
            answer: Answer {
                effects: Effects {
                    discovery,
                    ..Effects::not_requested()
                },
                local_change,
                ..Answer::stopped(code).with_data(MutationDataDto::Poll(data))
            },
            standing,
        }
    }

    /// The domain returned an error.
    fn stopped(call: &CallContext<'_>, accepted: &Accepted, error: RemotePollError) -> Ran {
        let plain = |code| Self::by_the_row(call, accepted, Answer::stopped(code));
        match error {
            RemotePollError::Transport(error) => {
                let code = outcome::ssh_transport_error_code(&error.kind);
                let journal = accepted.journal_row(call.service);
                let error = SynchronizationError::Transport(error);
                let answer = Answer {
                    data: sync::host_data(&error),
                    ..Answer::stopped(code)
                };
                match journal {
                    // The live plan could not be made, and nothing reserved.
                    Some(JournalRow::Absent) | None => Ran {
                        answer,
                        standing: Standing::Stopped { journal },
                    },
                    // The domain ended the row, and the same request would
                    // only read it. The journal reads `failed` as pending, and
                    // the record is finished all the same.
                    Some(JournalRow::Pending(_) | JournalRow::Final { .. }) => Ran {
                        answer,
                        standing: Standing::Final,
                    },
                }
            }
            RemotePollError::Repository(error) => Self::by_the_row(call, accepted, refused(error)),
            RemotePollError::Busy => plain(ResultCode::Busy),
            RemotePollError::PollYielding => plain(ResultCode::PollYielding),
            RemotePollError::ConflictPending => plain(ResultCode::RecoveryRequired),
            RemotePollError::Interrupted => Ran {
                answer: Answer::stopped(ResultCode::RecoveryRequired),
                standing: Standing::Owed,
            },
        }
    }
}

impl Binding for PollOnceBinding {
    fn command(&self) -> Command {
        Command::PollOnce
    }

    /// The row of a poll records phases and no checkpoint step.
    fn ended_without_checkpoint(&self) -> EndedWithoutCheckpoint {
        EndedWithoutCheckpoint::SaysNothing
    }

    fn place(&self) -> Place<'_> {
        Place::Repository(&self.root)
    }

    /// The repository as a whole: the command names nothing within it.
    fn target(&self) -> String {
        String::new()
    }

    fn fields(&self, builder: IntentDigestBuilder) -> IntentDigestBuilder {
        builder
    }

    fn scope(&self, repository: Option<String>, position: Option<&Position>) -> Scope {
        Scope {
            branch: position.and_then(|position| position.branch().map(str::to_owned)),
            repository,
            ..Scope::default()
        }
    }

    /// A repository with no publication remote is refused here, with nothing
    /// written. Any other plan failure is the domain's, which writes its stored
    /// state (known limit). The request is placed on primary.
    fn prepare(&mut self, call: &CallContext<'_>) -> Result<Prepared, Answer> {
        let repository = resolve_root(call.service, &self.root)?;
        let root = repository.root();
        let configuration =
            match read_configuration_for(root, RepositoryOperation::RepositorySnapshot)
                .map_err(refused)?
            {
                ConfigurationInspection::Valid(configuration) => configuration,
                ConfigurationInspection::Missing | ConfigurationInspection::Invalid(_) => {
                    return Err(Answer::stopped(ResultCode::InvalidConfiguration));
                }
            };
        if configuration.publication_remote.is_none() {
            return Err(Answer::stopped(ResultCode::PublicationRemoteRequired));
        }
        let primary = format!("refs/heads/{}", configuration.primary_branch);
        Ok(Prepared {
            family: Some(OperationFamily::Remote),
            position: Position::of(root, &primary)
                .map_err(|_| Answer::stopped(ResultCode::RepositoryInaccessible))?,
            expected: None,
            observation: None,
        })
    }

    fn run(&self, call: &CallContext<'_>, accepted: &Accepted) -> Ran {
        self.call(call, accepted, false)
    }

    /// Every row is the domain's to read, which answers a row that ended from
    /// what it stored. A row fenced by a change of configuration isn't: a
    /// restart refuses it for good, so its answer is final, the one `request
    /// cancel` finishes it with. A failed row has ended, and its stored category
    /// stands whatever was changed since.
    fn reenter(
        &mut self,
        call: &CallContext<'_>,
        accepted: &Accepted,
        journal: JournalRow,
    ) -> Result<Entered, Ran> {
        let ended = matches!(
            journal,
            JournalRow::Pending(PendingOperation::Remote {
                phase: RemoteOperationPhase::Failed
            })
        );
        if !ended && let Some(answer) = sync::fenced_answer(call.service, accepted, Some(&journal))
        {
            return Err(Ran {
                answer,
                standing: Standing::Final,
            });
        }
        Ok(Entered::Unfinished)
    }

    fn as_confirmed(&mut self) -> Option<&mut dyn Confirmed> {
        None
    }

    fn rerun(&self, call: &CallContext<'_>, accepted: &Accepted, _entered: &Entered) -> Ran {
        self.call(call, accepted, true)
    }
}
