//! `poll configure`, `poll pause` and `poll resume`: one field of a
//! registration's polling policy, written in the request's own transaction
//! (R-7, decision 15).
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

use crate::results::{ResultCode, Scope, timestamp_string};

use super::{
    super::{
        super::{
            PollingInterval, ReadError,
            recovery::JournalRow,
            remote::state::{PollingChange, PollingUpdate, update_remote_polling},
        },
        command::Command,
        dto::{MutationDataDto, PollPolicyMutationDto},
        evidence::Position,
        identity::{FieldValue, IntentDigestBuilder, ScopeKey},
    },
    Accepted, Answer, Binding, CallContext, Confirmed, EndedWithoutCheckpoint, Entered,
    OneTransaction, Place, Prepared, Ran, Standing, resolve_root,
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
        let update = update_remote_polling(transaction, scope.as_str(), change)?;
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

    /// There is no row of any kind: `reenter` is not reached.
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

    /// Not reached: see `one_transaction`. A record that says otherwise is no
    /// record of this build, and nothing is written on it.
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
