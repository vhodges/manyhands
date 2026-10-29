//! `item republish`: an item's editing branch, which the remote deleted,
//! is pushed to it again at exactly the local commit its owner was shown
//! and consented to (decisions 4, 7 and 11, R-18 and R-19). It needs
//! confirmation, since ordinary synchronization never recreates a deleted
//! branch, and confirmation is fixed per command (fact 12).
//!
//! **The preview** reads what is stored of the deletion, and nothing else:
//! a `prepare` cannot contact the remote (fact 55). Only a deletion that a
//! fetch recorded, or that the push evidence of the branch's publication
//! shows, gives a preview; anything else is the no-op, with nothing to
//! consent to. Its observation binds the branch, the remote, the stored
//! deletion, the commit last published there and the local commit, so any
//! change to them before the request is accepted is `external_change`.
//!
//! **The domain call** is the push-only path, through `Session::republish`,
//! made by a synchronization's binding (`sync::republished`): it reports
//! the stages of its safe points, reads a cancellation from the request's
//! flag and from the call's token, and answers with a synchronization's
//! codes and effects. Its operation is a synchronization of the context in
//! the remote journal (fact 78), held as a synchronization's is: a stop of
//! the transport, a key or a host leaves it, and the same request goes on
//! with `restart`, which asks the remote what it holds before it pushes
//! again. A change since the preview, or a push the remote refused, ends it
//! with nothing of it on the remote, and the request is finished: a new
//! preview is what goes on.
//!
//! **What it never does** is integrate: no local branch moves and nothing
//! is merged. The next `item sync` integrates primary. A lifecycle-closed
//! ticket is refused (decision 17).

use std::path::PathBuf;

use crate::{
    canonical::ItemId,
    results::{ResultCode, Scope},
};

use super::{
    super::{
        super::{
            AuthoringKind, ConfigurationInspection, ItemDtoKind, OperationFamily,
            RepositoryOperation, RepositoryService, SynchronizationError, SynchronizationTarget,
            authoring_kind_segment, read_configuration_for,
            recovery::{FinalKind, JournalRow},
            remote::sync::{DeletionEvidence, RepublishConsent, RepublishEvidence},
            ticket_guard::require_ticket_open,
        },
        command::Command,
        confirm::{self, Row, Seen},
        context::context_names,
        dto::{ItemRepublishPreviewDto, PreviewDetailsDto, PreviewEffect},
        evidence::Position,
        identity::{IntentDigestBuilder, ScopeKey},
        observe::{Observation, ObservationDigest},
        progress::Subject,
    },
    Accepted, Answer, Binding, CallContext, Confirmed, EndedWithoutCheckpoint, Entered, Place,
    Prepared, Previewed, Ran, Standing, item_scope, refused, resolve_root, sync,
};

/// The input of `item republish`: the item whose editing branch the remote
/// deleted.
#[derive(Clone)]
pub struct ItemRepublishInput {
    pub root: PathBuf,
    pub kind: AuthoringKind,
    pub id: ItemId,
}

pub(crate) struct ItemRepublishBinding {
    root: PathBuf,
    kind: AuthoringKind,
    id: ItemId,
    /// What the request's owner consented to, for the domain call: as
    /// this call observed it before the request was accepted, or as the
    /// request was accepted with, for a call that enters it again.
    consent: Option<RepublishConsent>,
}

impl From<ItemRepublishInput> for ItemRepublishBinding {
    fn from(input: ItemRepublishInput) -> Self {
        // Every field is named: one added to the input does not compile
        // until it is taken into the binding or left out here.
        let ItemRepublishInput { root, kind, id } = input;
        Self {
            root,
            kind,
            id,
            consent: None,
        }
    }
}

/// What a republish observes, with no lease, no reservation and no
/// connection.
struct Observed {
    /// The publication remote, by its name.
    remote: String,
    /// The item's editing branch, as its full reference name.
    branch_ref: String,
    /// The branch's local tip.
    local: git2::Oid,
    evidence: RepublishEvidence,
}

impl Observed {
    /// What a request accepted on this observation consents to.
    fn consent(&self) -> RepublishConsent {
        RepublishConsent {
            branch: self.branch_ref.clone(),
            last_published: self.evidence.last_published,
            local: self.local,
        }
    }

    /// The digest of what was observed: what a confirmation stores, and
    /// what a later observation is compared with. It binds every field the
    /// preview shows, and the stored deletion.
    fn digest(&self) -> ObservationDigest {
        let command = Command::ItemRepublish.name();
        let deletion = match self.evidence.deletion {
            DeletionEvidence::Fetched => "fetched",
            DeletionEvidence::Pushed => "pushed",
            DeletionEvidence::Ambiguous => "ambiguous",
            DeletionEvidence::Unknown => "unknown",
            DeletionEvidence::None => "none",
        };
        let observation = Observation::of(command)
            .text("branch", &self.branch_ref)
            .text("remote", &self.remote)
            .text("deletion", deletion);
        let observation = match self.evidence.last_published {
            Some(published) => observation.text("last_published", &published.to_string()),
            None => observation.absent("last_published"),
        };
        let observation = observation.text("local", &self.local.to_string());
        ObservationDigest::of_state(command, observation.finish())
    }
}

/// The answer to a republish refused before anything was pushed, for a
/// change since its preview or a push the remote refused, when its
/// operation has ended, or never began: the request is finished, since its
/// consent is spent. `None` for every other answer, which a
/// synchronization's binding has made.
fn spent(synchronized: &sync::Synchronized, ran: &Ran) -> Option<Ran> {
    let code = match synchronized {
        Some(Err(SynchronizationError::ExternalChange)) => ResultCode::ExternalChange,
        Some(Err(SynchronizationError::PushRejected)) => ResultCode::PushRejected,
        _ => return None,
    };
    match &ran.standing {
        Standing::Stopped {
            journal:
                Some(
                    JournalRow::Absent
                    | JournalRow::Final {
                        kind: FinalKind::Cancelled,
                        owes_work: _,
                        checkpointed: _,
                    },
                ),
        } => Some(Ran {
            answer: Answer::stopped(code),
            standing: Standing::Final,
        }),
        _ => None,
    }
}

impl ItemRepublishBinding {
    fn target_of(&self) -> SynchronizationTarget {
        SynchronizationTarget::Context {
            kind: self.kind,
            item_id: self.id.clone(),
        }
    }

    /// Reads what the request is about: the repository, its publication
    /// remote, the item's editing branch and its tip, and what is stored of
    /// the branch's deletion. A repository with no publication remote has
    /// nowhere to publish to, a lifecycle-closed ticket is not republished
    /// (decision 17), and an item with no editing branch has nothing to.
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
        let Some(remote) = configuration.publication_remote.clone() else {
            return Err(Answer::stopped(ResultCode::PublicationRemoteRequired));
        };
        let git = git2::Repository::open(&root)
            .map_err(|_| Answer::stopped(ResultCode::RepositoryInaccessible))?;
        if self.kind == AuthoringKind::Ticket {
            require_ticket_open(&git, &configuration.primary_branch, &self.id).map_err(refused)?;
        }
        let branch_ref = context_names(&root, &self.kind, &self.id.to_string()).branch_ref;
        let local = match git.refname_to_id(&branch_ref) {
            Ok(local) => local,
            Err(error) if error.code() == git2::ErrorCode::NotFound => {
                return Err(Answer::stopped(ResultCode::ItemNotFound));
            }
            Err(_) => return Err(Answer::stopped(ResultCode::RepositoryInaccessible)),
        };
        let evidence = service
            .republish_evidence(&root, self.kind, &self.id)
            .map_err(refused)?;
        Ok(Observed {
            remote,
            branch_ref,
            local,
            evidence,
        })
    }

    /// The consent a request whose operation has begun was accepted with:
    /// its branch and the local commit, as its recorded position holds
    /// them. The commit last published is `None`: the domain compares it
    /// with what is stored only before it begins an operation, and judges
    /// one that has begun by that operation's own evidence.
    fn accepted_consent(accepted: &Accepted) -> Option<RepublishConsent> {
        Some(RepublishConsent {
            branch: accepted.position.base_ref.clone()?,
            last_published: None,
            local: accepted.position.base_oid?,
        })
    }

    /// The one domain call, with the consent this call holds. `restart`
    /// is set by a call that enters the request again.
    fn call(&self, call: &CallContext<'_>, accepted: &Accepted, restart: bool) -> Ran {
        // No consent was read: nothing is called on that, and the row
        // says what is in flight.
        let Some(consent) = self.consent.clone() else {
            return Ran {
                answer: Answer::stopped(ResultCode::InternalError),
                standing: Standing::Stopped {
                    journal: accepted.journal_row(call.service),
                },
            };
        };
        let (synchronized, ran) = sync::republished(
            call,
            accepted,
            self.target_of(),
            consent,
            Subject::of(self, accepted),
            restart,
        );
        let ran = sync::kept_for_its_result(ran);
        spent(&synchronized, &ran).unwrap_or(ran)
    }
}

impl Binding for ItemRepublishBinding {
    fn command(&self) -> Command {
        Command::ItemRepublish
    }

    /// The row of a republish is in the remote journal, which records
    /// phases and no checkpoint step, as a synchronization's row is.
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
            // What the digest is begun with: the scope and the target. The
            // request holds nothing else.
            root: _,
            kind: _,
            id: _,
            // What a call observed, and no part of the request.
            consent: _,
        } = self;
        builder
    }

    fn scope(&self, repository: Option<String>, position: Option<&Position>) -> Scope {
        item_scope(&self.kind, &self.id.to_string(), repository, position)
    }

    /// Observes, and gives the digest the boundary compares with the
    /// confirmation's: no check is made here of what the preview covers.
    /// The request is placed on the item's branch, at the commit observed.
    fn prepare(&mut self, call: &CallContext<'_>) -> Result<Prepared, Answer> {
        let observed = self.observe(call.service)?;
        self.consent = Some(observed.consent());
        Ok(Prepared {
            family: Some(OperationFamily::Remote),
            position: Position {
                base_ref: Some(observed.branch_ref.clone()),
                base_oid: Some(observed.local),
            },
            expected: None,
            observation: Some(observed.digest().to_stored()),
        })
    }

    fn run(&self, call: &CallContext<'_>, accepted: &Accepted) -> Ran {
        self.call(call, accepted, false)
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
        self.call(call, accepted, true)
    }
}

impl Confirmed for ItemRepublishBinding {
    /// A preview is given only of a deletion that is recorded. Anything
    /// else is the no-op: the branch was never published, a fetch last
    /// found it there, or what its push evidence shows is ambiguous or of
    /// another configuration. `item sync` is what says more.
    fn preview(&mut self, call: &CallContext<'_>) -> Result<Previewed, Answer> {
        let observed = self.observe(call.service)?;
        if !observed.evidence.deleted() {
            return Err(Answer::stopped(ResultCode::AlreadyApplied));
        }
        let root = ScopeKey::for_repository(&self.root)
            .ok_or_else(|| Answer::stopped(ResultCode::InvalidPath))?;
        Ok(Previewed {
            observation: observed.digest().to_stored(),
            effects: vec![PreviewEffect::BranchRepublished],
            details: PreviewDetailsDto::Republish(ItemRepublishPreviewDto {
                root: root.as_str().to_owned(),
                kind: match self.kind {
                    AuthoringKind::Document => ItemDtoKind::Document,
                    AuthoringKind::Ticket => ItemDtoKind::Ticket,
                },
                item_id: self.id.to_string(),
                branch: observed
                    .branch_ref
                    .strip_prefix("refs/heads/")
                    .unwrap_or(&observed.branch_ref)
                    .to_owned(),
                remote: observed.remote.clone(),
                last_published_oid: observed
                    .evidence
                    .last_published
                    .map(|published| published.to_string()),
                local_oid: observed.local.to_string(),
            }),
        })
    }

    /// A fenced operation's answer is final, as a synchronization's is.
    /// One that has begun is the domain's to take up, with nothing
    /// compared: its own evidence is what it is judged by. A request whose
    /// operation never began observes again.
    fn row(&mut self, call: &CallContext<'_>, accepted: &Accepted, journal: &JournalRow) -> Row {
        if let Some(answer) = sync::fenced_answer(call.service, accepted, Some(journal)) {
            return Row::answers(Ran {
                answer,
                standing: Standing::Final,
            });
        }
        if *journal == JournalRow::Absent {
            return Row::Unrecorded;
        }
        self.consent = Self::accepted_consent(accepted);
        Row::Recorded
    }

    /// Observes as the preview observed. What cannot be observed, or is
    /// refused now, leaves nothing in flight: the operation never began.
    fn observe_again(
        &mut self,
        call: &CallContext<'_>,
        _accepted: &Accepted,
        journal: &JournalRow,
    ) -> Result<Seen, Ran> {
        let observed = self.observe(call.service).map_err(|answer| Ran {
            answer,
            standing: Standing::NotRun {
                journal: journal.clone(),
            },
        })?;
        self.consent = Some(observed.consent());
        Ok(Seen::State {
            found: observed.digest(),
            own: None,
        })
    }
}
