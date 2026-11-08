//! `remote resolve-history`: clears the repository-wide cache-loss flag
//! when it is set and no context reads unknown (Decision 7), and deletes
//! the marker file after the commit. It needs confirmation.
//!
//! **The preview** reads what is stored: the flag, the contexts that still
//! read unknown, the generation and whether the marker file is there. A
//! flag already clear is the no-op; a context still unknown refuses with
//! `recovery_required`, whose way out is `remote history` and `item
//! resolve-history` (`clear_cache_loss_way_out`). As for `item
//! resolve-history`, the command has no operation and no row: the registry
//! is what says the work is done.

use std::path::PathBuf;

use crate::results::{ResultCode, Scope};

use super::{
    super::{
        super::{
            ConfigurationInspection, RepositoryErrorKind, RepositoryOperation, RepositoryService,
            read_configuration_for, recovery::JournalRow, remote::resolve::CacheLossObservation,
        },
        command::Command,
        confirm::{self, Row, Seen},
        dto::{PreviewDetailsDto, PreviewEffect, RemoteResolveHistoryPreviewDto},
        evidence::Position,
        identity::{IntentDigestBuilder, ScopeKey},
        observe::{Observation, ObservationDigest},
    },
    Accepted, Answer, Binding, CallContext, Confirmed, EndedWithoutCheckpoint, Entered, Place,
    Prepared, Previewed, Ran, Standing, refused,
    resolve_history::data_of,
    resolve_root,
};

/// The input of `remote resolve-history`: the repository.
#[derive(Clone)]
pub struct RemoteResolveHistoryInput {
    pub root: PathBuf,
}

pub(crate) struct RemoteResolveHistoryBinding {
    root: PathBuf,
    /// What this call saw: the domain call writes only while it still holds.
    seen: Option<CacheLossObservation>,
}

impl From<RemoteResolveHistoryInput> for RemoteResolveHistoryBinding {
    fn from(input: RemoteResolveHistoryInput) -> Self {
        let RemoteResolveHistoryInput { root } = input;
        Self { root, seen: None }
    }
}

/// No row is ever read for the command: it has no operation.
const NO_ROW: JournalRow = JournalRow::Absent;

struct Observed {
    root: String,
    observation: CacheLossObservation,
}

impl Observed {
    fn digest(&self) -> ObservationDigest {
        let command = Command::RemoteResolveHistory.name();
        let CacheLossObservation {
            cache_loss,
            generation,
            unresolved,
            marker_present,
        } = &self.observation;
        let flag = |value: bool| if value { "1" } else { "0" };
        let unresolved: Vec<String> = unresolved
            .iter()
            .map(|(kind, id)| format!("{}/{id}", crate::repository::authoring_kind_segment(kind)))
            .collect();
        let observation = Observation::of(command)
            .text("cache_loss", flag(*cache_loss))
            .text("generation", &generation.to_string())
            .text("unresolved", &unresolved.join(","))
            .text("marker", flag(*marker_present));
        ObservationDigest::of_state(command, observation.finish())
    }
}

impl RemoteResolveHistoryBinding {
    fn observe(&self, service: &RepositoryService) -> Result<Observed, Answer> {
        let repository = resolve_root(service, &self.root)?;
        let root = repository.root().to_owned();
        match read_configuration_for(&root, RepositoryOperation::RepositorySnapshot)
            .map_err(refused)?
        {
            ConfigurationInspection::Valid(_) => {}
            ConfigurationInspection::Missing | ConfigurationInspection::Invalid(_) => {
                return Err(Answer::stopped(ResultCode::InvalidConfiguration));
            }
        }
        let observation = service.cache_loss_observation(&root).map_err(refused)?;
        Ok(Observed {
            root: ScopeKey::for_repository(&root).map_or_else(
                || root.to_string_lossy().into_owned(),
                |key| key.as_str().to_owned(),
            ),
            observation,
        })
    }

    fn call(&self, service: &RepositoryService, accepted: &Accepted) -> Ran {
        let Some(seen) = &self.seen else {
            return Ran {
                answer: Answer::stopped(ResultCode::InternalError),
                standing: Standing::Stopped {
                    journal: Some(NO_ROW),
                },
            };
        };
        match service.clear_cache_loss(&accepted.root(), seen) {
            Ok(resolution) => Ran {
                answer: Answer::stopped(ResultCode::Ok).with_data(data_of(resolution)),
                standing: Standing::Final,
            },
            Err(error) if error.kind == RepositoryErrorKind::ExternalChange => Ran {
                answer: refused(error),
                standing: Standing::NotRun { journal: NO_ROW },
            },
            Err(error) => Ran {
                answer: refused(error),
                standing: if accepted.attempt > 1 {
                    Standing::Unconfirmed
                } else {
                    Standing::Stopped {
                        journal: Some(NO_ROW),
                    }
                },
            },
        }
    }
}

impl Binding for RemoteResolveHistoryBinding {
    fn command(&self) -> Command {
        Command::RemoteResolveHistory
    }

    fn ended_without_checkpoint(&self) -> EndedWithoutCheckpoint {
        EndedWithoutCheckpoint::SaysNothing
    }

    fn place(&self) -> Place<'_> {
        Place::Repository(&self.root)
    }

    /// The repository as a whole.
    fn target(&self) -> String {
        String::new()
    }

    fn fields(&self, builder: IntentDigestBuilder) -> IntentDigestBuilder {
        let Self { root: _, seen: _ } = self;
        builder
    }

    fn scope(&self, repository: Option<String>, _position: Option<&Position>) -> Scope {
        Scope {
            repository,
            ..Scope::default()
        }
    }

    fn prepare(&mut self, call: &CallContext<'_>) -> Result<Prepared, Answer> {
        let observed = self.observe(call.service)?;
        let observation = observed.digest().to_stored();
        self.seen = Some(observed.observation);
        Ok(Prepared {
            family: None,
            position: Position {
                base_ref: None,
                base_oid: None,
            },
            expected: None,
            observation: Some(observation),
        })
    }

    fn run(&self, call: &CallContext<'_>, accepted: &Accepted) -> Ran {
        self.call(call.service, accepted)
    }

    fn reenter(
        &mut self,
        call: &CallContext<'_>,
        accepted: &Accepted,
        journal: JournalRow,
    ) -> Result<Entered, Ran> {
        confirm::reenter(self, call, accepted, journal)
    }

    fn as_confirmed(&mut self) -> Option<&mut dyn Confirmed> {
        Some(self)
    }

    fn rerun(&self, call: &CallContext<'_>, accepted: &Accepted, _entered: &Entered) -> Ran {
        self.call(call.service, accepted)
    }
}

impl Confirmed for RemoteResolveHistoryBinding {
    fn preview(&mut self, call: &CallContext<'_>) -> Result<Previewed, Answer> {
        let observed = self.observe(call.service)?;
        let found = &observed.observation;
        if !found.cache_loss {
            return Err(Answer::stopped(ResultCode::AlreadyApplied));
        }
        if !found.unresolved.is_empty() {
            return Err(Answer::stopped(ResultCode::RecoveryRequired));
        }
        Ok(Previewed {
            observation: observed.digest().to_stored(),
            effects: vec![PreviewEffect::HistoryFlagCleared],
            details: PreviewDetailsDto::ResolveCacheLoss(RemoteResolveHistoryPreviewDto {
                root: observed.root.clone(),
                marker_present: found.marker_present,
            }),
        })
    }

    fn row(&mut self, _call: &CallContext<'_>, _accepted: &Accepted, journal: &JournalRow) -> Row {
        if *journal == NO_ROW {
            Row::Unrecorded
        } else {
            Row::Impossible
        }
    }

    /// A flag already clear is the end state, by this request's work or
    /// another's; anything else that moved is `external_change`.
    fn observe_again(
        &mut self,
        call: &CallContext<'_>,
        _accepted: &Accepted,
        _journal: &JournalRow,
    ) -> Result<Seen, Ran> {
        let observed = self.observe(call.service).map_err(|answer| Ran {
            answer,
            standing: Standing::Stopped { journal: None },
        })?;
        if !observed.observation.cache_loss {
            return Ok(Seen::EndState(Answer::stopped(ResultCode::AlreadyApplied)));
        }
        let digest = observed.digest();
        self.seen = Some(observed.observation);
        Ok(Seen::State {
            found: digest,
            own: None,
        })
    }
}

#[cfg(test)]
#[path = "remote_resolve_history_tests.rs"]
mod tests;
