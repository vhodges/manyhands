//! `item resolve-history`: a context whose publication history is unknown
//! is recorded as never published, so that the next `item sync` may publish
//! it by the ordinary path (Decision 4). It needs confirmation, since it
//! lifts the refusal that guards a branch the remote may already have held,
//! and confirmation is fixed per command (fact 12).
//!
//! **The preview** reads what is stored of the context, and nothing else: a
//! `prepare` cannot contact the remote. `remote history` is the read that
//! fetches. Its observation binds the command and the choice, the context's
//! branch and head, whether its tracking ref exists, the OID the **current**
//! batch advertises for it (or its absence), the configuration generation and
//! the fetch evidence. A newer fetch that changed any of them before the
//! request is accepted is `external_change`, and a fresh prepare is named.
//!
//! - `publish`, with the context unknown and absent from the batch, is the
//!   preview.
//! - `release` is never previewed: the way out names `context release`
//!   (Decision 2).
//! - A context already resolved, or one the remote has, is the no-op; the
//!   latter names `item sync`.
//! - A context published and then seen gone is refused, naming `item
//!   republish`: this verb never stands in for a republish's consent.
//!
//! **No operation and no row.** The domain call is one registry transaction
//! (`RepositoryService::resolve_history`) that re-reads what the digest
//! covers and writes the context's row, or writes nothing. What says the
//! work is done is the registry itself, as for `repo identity-set`: a
//! request whose result was lost finds the context resolved and is answered
//! `already_applied`; any other change is `external_change`.

use std::path::PathBuf;

use crate::{
    canonical::ItemId,
    results::{ResultCode, Scope},
};

use super::{
    super::{
        super::{
            AuthoringKind, ConfigurationInspection, ItemDtoKind, RemotePublicationEvidence,
            RepositoryErrorKind, RepositoryOperation, RepositoryService, authoring_kind_segment,
            read_configuration_for,
            recovery::JournalRow,
            remote::resolve::{HistoryObservation, HistoryResolution, MarkerRetirement},
            ticket_guard::require_ticket_open,
        },
        command::Command,
        confirm::{self, Row, Seen},
        context::context_names,
        dto::{
            HistoryMarkerState, ItemResolveHistoryMutationDto, ItemResolveHistoryPreviewDto,
            MutationDataDto, PreviewDetailsDto, PreviewEffect,
        },
        evidence::Position,
        identity::{FieldValue, IntentDigestBuilder, ScopeKey},
        observe::{Observation, ObservationDigest},
    },
    Accepted, Answer, Binding, CallContext, Confirmed, EndedWithoutCheckpoint, Entered, Place,
    Prepared, Previewed, Ran, Standing, item_scope, refused, resolve_root,
};

/// What the owner decides for a context whose history is unknown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResolveHistoryChoice {
    /// Record it as never published; `item sync` then publishes it.
    Publish,
    /// Give up the local copy. Never previewed: `context release` does it.
    Release,
}

impl ResolveHistoryChoice {
    const fn name(self) -> &'static str {
        match self {
            Self::Publish => "publish",
            Self::Release => "release",
        }
    }
}

/// The input of `item resolve-history`: the item whose context is unresolved,
/// and the owner's choice.
#[derive(Clone)]
pub struct ItemResolveHistoryInput {
    pub root: PathBuf,
    pub kind: AuthoringKind,
    pub id: ItemId,
    pub choice: ResolveHistoryChoice,
}

pub(crate) struct ItemResolveHistoryBinding {
    root: PathBuf,
    kind: AuthoringKind,
    id: ItemId,
    choice: ResolveHistoryChoice,
    /// What this call saw: the domain call writes only while it still holds.
    /// Set by every observation that leads to one.
    seen: Option<HistoryObservation>,
}

impl From<ItemResolveHistoryInput> for ItemResolveHistoryBinding {
    fn from(input: ItemResolveHistoryInput) -> Self {
        // Every field is named: one added to the input does not compile
        // until it is taken into the binding or left out here.
        let ItemResolveHistoryInput {
            root,
            kind,
            id,
            choice,
        } = input;
        Self {
            root,
            kind,
            id,
            choice,
            seen: None,
        }
    }
}

/// No row is ever read for the command: it has no operation.
const NO_ROW: JournalRow = JournalRow::Absent;

/// What a resolution observes, with no lease, no reservation and no
/// connection.
struct Observed {
    /// The repository's canonical root.
    root: String,
    observation: HistoryObservation,
}

fn evidence_name(evidence: RemotePublicationEvidence) -> &'static str {
    match evidence {
        RemotePublicationEvidence::NeverPublished => "never_published",
        RemotePublicationEvidence::ObservedPublished => "observed_published",
        RemotePublicationEvidence::HistoryUnknown => "history_unknown",
    }
}

fn flag(value: bool) -> &'static str {
    if value { "1" } else { "0" }
}

/// `release` is no choice of this verb: the way out is `context release`
/// (`resolve_history_way_out`).
fn release_refusal() -> Answer {
    Answer::stopped(ResultCode::InvalidInput)
}

impl Observed {
    /// The digest of what was observed: what a confirmation stores, and what
    /// a later observation is compared with. It binds every field the preview
    /// shows, and the choice.
    fn digest(&self, choice: ResolveHistoryChoice) -> ObservationDigest {
        let command = Command::ItemResolveHistory.name();
        let HistoryObservation {
            branch,
            local_head,
            tracking_ref_exists,
            advertised,
            generation,
            evidence,
            remotely_deleted,
            cache_loss,
        } = &self.observation;
        let optional = |observation: Observation, name: &str, oid: &Option<git2::Oid>| match oid {
            Some(oid) => observation.text(name, &oid.to_string()),
            None => observation.absent(name),
        };
        let observation = Observation::of(command)
            .text("choice", choice.name())
            .text("branch", branch);
        let observation = optional(observation, "local", local_head);
        let observation = observation.text("tracking_ref", flag(*tracking_ref_exists));
        let observation = optional(observation, "advertised", advertised)
            .text("generation", &generation.to_string())
            .text("evidence", evidence_name(*evidence))
            .text("remotely_deleted", flag(*remotely_deleted))
            .text("cache_loss", flag(*cache_loss));
        ObservationDigest::of_state(command, observation.finish())
    }
}

fn data(resolution: HistoryResolution) -> MutationDataDto {
    let HistoryResolution {
        cache_loss_cleared,
        marker,
    } = resolution;
    MutationDataDto::ResolveHistory(ItemResolveHistoryMutationDto {
        cache_loss_cleared,
        marker: match marker {
            MarkerRetirement::NotApplicable => HistoryMarkerState::NotApplicable,
            MarkerRetirement::Removed => HistoryMarkerState::Removed,
            MarkerRetirement::Absent => HistoryMarkerState::Absent,
            MarkerRetirement::Failed => HistoryMarkerState::Failed,
        },
    })
}

impl ItemResolveHistoryBinding {
    /// Reads what the request is about: the repository, its publication
    /// remote, the item's editing branch and what is stored of its history.
    /// A repository with no publication remote has nothing to resolve, a
    /// lifecycle-closed ticket is not resolved (decision 17), and an item
    /// with no editing branch has no context.
    fn observe(&self, service: &RepositoryService) -> Result<Observed, Answer> {
        let repository = resolve_root(service, &self.root)?;
        let root = repository.root().to_owned();
        let configuration =
            match read_configuration_for(&root, RepositoryOperation::RepositorySnapshot)
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
        let git = git2::Repository::open(&root)
            .map_err(|_| Answer::stopped(ResultCode::RepositoryInaccessible))?;
        if self.kind == AuthoringKind::Ticket {
            require_ticket_open(&git, &configuration.primary_branch, &self.id).map_err(refused)?;
        }
        let branch_ref = context_names(&root, &self.kind, &self.id.to_string()).branch_ref;
        match git.refname_to_id(&branch_ref) {
            Ok(_) => {}
            Err(error) if error.code() == git2::ErrorCode::NotFound => {
                return Err(Answer::stopped(ResultCode::ItemNotFound));
            }
            Err(_) => return Err(Answer::stopped(ResultCode::RepositoryInaccessible)),
        }
        let observation = service
            .history_observation(&root, self.kind, &self.id)
            .map_err(refused)?;
        Ok(Observed {
            root: ScopeKey::for_repository(&root).map_or_else(
                || root.to_string_lossy().into_owned(),
                |key| key.as_str().to_owned(),
            ),
            observation,
        })
    }

    /// The one domain call, and what it came to.
    fn call(&self, service: &RepositoryService, accepted: &Accepted) -> Ran {
        let Some(seen) = &self.seen else {
            return Ran {
                answer: Answer::stopped(ResultCode::InternalError),
                standing: Standing::Stopped {
                    journal: Some(NO_ROW),
                },
            };
        };
        match service.resolve_history(&accepted.root(), self.kind, &self.id, seen) {
            Ok(resolution) => Ran {
                answer: Answer::stopped(ResultCode::Ok).with_data(data(resolution)),
                standing: Standing::Final,
            },
            // Changed between this call's observation and its transaction:
            // what is there is not what was shown, as when the observation
            // itself finds it so.
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

impl Binding for ItemResolveHistoryBinding {
    fn command(&self) -> Command {
        Command::ItemResolveHistory
    }

    /// The command has no operation, so no row of one is ever read.
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
        let Self {
            // What the digest is begun with: the scope and the target.
            root: _,
            kind: _,
            id: _,
            choice,
            // What a call observed, and no part of the request.
            seen: _,
        } = self;
        builder.field("choice", FieldValue::Text(choice.name()))
    }

    fn scope(&self, repository: Option<String>, position: Option<&Position>) -> Scope {
        item_scope(&self.kind, &self.id.to_string(), repository, position)
    }

    /// Observes, and gives the digest the boundary compares with the
    /// confirmation's: no check is made here of what the preview covers.
    /// `release` is refused, since it has no preview to have consented to.
    fn prepare(&mut self, call: &CallContext<'_>) -> Result<Prepared, Answer> {
        let observed = self.observe(call.service)?;
        if self.choice == ResolveHistoryChoice::Release {
            return Err(release_refusal());
        }
        let observation = observed.digest(self.choice).to_stored();
        self.seen = Some(observed.observation);
        Ok(Prepared {
            // No operation ID and no row.
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

impl Confirmed for ItemResolveHistoryBinding {
    fn preview(&mut self, call: &CallContext<'_>) -> Result<Previewed, Answer> {
        let observed = self.observe(call.service)?;
        if self.choice == ResolveHistoryChoice::Release {
            return Err(release_refusal());
        }
        let found = &observed.observation;
        // Published, then seen gone: only `item republish` recreates it.
        if found.remotely_deleted {
            return Err(Answer::stopped(ResultCode::RemoteBranchDeleted));
        }
        // The remote has the branch, or the history is no longer unknown:
        // nothing is left to resolve.
        if found.advertised.is_some() || !found.resolvable() {
            return Err(Answer::stopped(ResultCode::AlreadyApplied));
        }
        Ok(Previewed {
            observation: observed.digest(self.choice).to_stored(),
            effects: vec![PreviewEffect::HistoryResolved],
            details: PreviewDetailsDto::ResolveHistory(ItemResolveHistoryPreviewDto {
                root: observed.root.clone(),
                kind: match self.kind {
                    AuthoringKind::Document => ItemDtoKind::Document,
                    AuthoringKind::Ticket => ItemDtoKind::Ticket,
                },
                item_id: self.id.to_string(),
                branch: found.branch.clone(),
                local_oid: found.local_head.map(|oid| oid.to_string()),
                tracking_ref_exists: found.tracking_ref_exists,
                cache_loss: found.cache_loss,
            }),
        })
    }

    /// No row is read for a request with no operation. Nothing is run or
    /// settled on one that is.
    fn row(&mut self, _call: &CallContext<'_>, _accepted: &Accepted, journal: &JournalRow) -> Row {
        if *journal == NO_ROW {
            Row::Unrecorded
        } else {
            Row::Impossible
        }
    }

    /// Observes as the preview observed. What cannot be observed leaves the
    /// record in place. A context that is resolved and absent is the end
    /// state, by this request's work or another's; anything else that moved
    /// is compared with the consent, and refused as `external_change`.
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
        let found = &observed.observation;
        if found.evidence == RemotePublicationEvidence::NeverPublished
            && found.advertised.is_none()
            && !found.remotely_deleted
        {
            return Ok(Seen::EndState(Answer::stopped(ResultCode::AlreadyApplied)));
        }
        let digest = observed.digest(self.choice);
        self.seen = Some(observed.observation);
        Ok(Seen::State {
            found: digest,
            own: None,
        })
    }
}

#[cfg(test)]
#[path = "resolve_history_tests.rs"]
mod tests;
