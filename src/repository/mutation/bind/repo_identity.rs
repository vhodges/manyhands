//! `repo identity-set`: a repository's own Git configuration is given a
//! name and an email, and optionally initials. It needs confirmation.
//!
//! The command works on any repository that has a working directory,
//! whether Manyhands was enabled on it or not: an identity is what
//! enabling needs first. So its root is not resolved as a registered
//! repository's is, and a repository that was never enabled is not
//! refused for that.
//!
//! **What its preview covers.** The name, the email and the initials the
//! repository's own configuration holds. The identity Git finds at any
//! other level is shown and is no part of it: the command neither reads
//! it to act nor changes it.
//!
//! **No operation and no row.** `set_local_identity` holds the repository
//! lease, writes at most three keys and puts the configuration file back
//! if one of them fails. Nothing records that it began. What says a
//! request's work is done is the configuration itself:
//!
//! - it holds what the request asks: the request's end state holds, by
//!   its own work or another's, and a retry is a no-op. A request whose
//!   result was lost after it wrote is answered so, and never as
//!   `external_change`;
//! - it holds what was previewed: nothing of the request has been done,
//!   and the domain is called;
//! - it holds anything else: what is there is not what was shown, and
//!   the answer is `external_change`. That is also the answer to a
//!   process killed between two keys, whose own partial write is then
//!   what is there: a new preview shows it, and a new request completes
//!   it.
//!
//! The comparison with the preview is what keeps a late retry from
//! writing its identity over a later one. It is the consent's own
//! observation digest, which the request's record holds through its
//! confirmation, so the record needs no `expected_digest` beside it.
//! What it cannot tell apart is a configuration that was changed and
//! put back as it was previewed: a retry then writes, as the request
//! was consented to.

use std::path::PathBuf;

use git2::Repository;

use crate::{
    repository::mutation::confirm::{self, Row, Seen},
    results::{ResultCode, Scope},
};

use super::{
    super::{
        super::{
            InitialsChange, LocalIdentity, RepositoryErrorKind, RepositoryOperation,
            RepositoryService, SetLocalIdentityOutcome, SetLocalIdentityRequest,
            bridges::identity::{ValidIdentity, identity_repository},
            read::identity_dto,
            recovery::JournalRow,
        },
        command::Command,
        dto::{
            IdentityMutationDto, IdentityPreviewDto, LocalIdentityDto, MutationDataDto,
            PreviewDetailsDto, PreviewEffect,
        },
        evidence::Position,
        identity::{FieldValue, IntentDigestBuilder},
        observe::{Observation, ObservationDigest},
    },
    Accepted, Answer, Binding, CallContext, Confirmed, EndedWithoutCheckpoint, Entered, Place,
    Prepared, Previewed, Ran, Standing, refused,
};

/// The input of `repo identity-set`: the root of a repository that has a
/// working directory, and the identity its own configuration is to hold.
#[derive(Clone)]
pub struct RepoIdentitySetInput {
    pub root: PathBuf,
    pub name: String,
    pub email: String,
    /// What becomes of the repository's initials: left as they are, when
    /// the request says nothing of them; cleared; or set.
    pub initials: InitialsChange,
}

/// `repo identity-set`. Every effect of it is `not_requested`: its data,
/// with the outcome, is what says what it did.
pub(crate) struct RepoIdentityBinding {
    root: PathBuf,
    name: String,
    email: String,
    initials: InitialsChange,
    /// What this call saw the repository's own configuration hold: the
    /// domain call writes only while it still holds that. Set by every
    /// observation that leads to one.
    seen: Option<LocalIdentity>,
}

impl From<RepoIdentitySetInput> for RepoIdentityBinding {
    fn from(input: RepoIdentitySetInput) -> Self {
        // Every field is named: one added to the input does not compile
        // until it is taken into the binding or left out here.
        let RepoIdentitySetInput {
            root,
            name,
            email,
            initials,
        } = input;
        Self {
            root,
            name,
            email,
            initials,
            seen: None,
        }
    }
}

/// No row is ever read for the command: it has no operation.
const NO_ROW: JournalRow = JournalRow::Absent;

/// What a request found.
struct Observed {
    repository: Repository,
    /// The repository's canonical root.
    root: PathBuf,
    /// The request's identity, as it would be written.
    asked: ValidIdentity,
    /// What the repository's own configuration holds.
    current: LocalIdentity,
}

fn data(local: LocalIdentity) -> MutationDataDto {
    let LocalIdentity {
        name,
        email,
        initials,
    } = local;
    MutationDataDto::Identity(IdentityMutationDto {
        name,
        email,
        initials,
    })
}

impl RepoIdentityBinding {
    /// Checks the input, and reads what a preview covers. Nothing is
    /// written and no lease is taken. An `Err` is an input that cannot
    /// be written, a root that is no repository's own, a configuration
    /// that says a key of the identity twice or in an included file, or
    /// a read that failed.
    fn observe(&self) -> Result<Observed, Answer> {
        let asked = ValidIdentity::of(&self.name, &self.email, &self.initials).map_err(refused)?;
        let (repository, root) = identity_repository(&self.root).map_err(refused)?;
        // A key that is there twice, or in an included file, is refused
        // here as `recovery_required`: no preview is given of what Git
        // would then refuse to write.
        let current = LocalIdentity::own(&repository, &root).map_err(refused)?;
        Ok(Observed {
            repository,
            root,
            asked,
            current,
        })
    }

    /// The digest of what was observed: what a confirmation stores, and
    /// what a later observation is compared with.
    fn digest(current: &LocalIdentity) -> ObservationDigest {
        let LocalIdentity {
            name,
            email,
            initials,
        } = current;
        let command = Command::RepoIdentitySet.name();
        let mut observation = Observation::of(command);
        for (key, value) in [("name", name), ("email", email), ("initials", initials)] {
            observation = match value {
                Some(value) => observation.text(key, value),
                None => observation.absent(key),
            };
        }
        ObservationDigest::of_state(command, observation.finish())
    }

    /// The one domain call, and what it came to.
    ///
    /// A call that fails has written nothing, or has put the file back.
    /// The call that accepted the request is then refused and its record
    /// released. A later call of the request keeps the record: it is the
    /// request's, and the same request is what goes on.
    fn call(&self, service: &RepositoryService, accepted: &Accepted) -> Ran {
        let result = service.set_local_identity(SetLocalIdentityRequest {
            root: accepted.root(),
            name: self.name.clone(),
            email: self.email.clone(),
            initials: self.initials.clone(),
            expected: self.seen.clone(),
        });
        match result {
            Ok(SetLocalIdentityOutcome::Changed(local)) => Ran {
                answer: Answer::stopped(ResultCode::Ok).with_data(data(local)),
                standing: Standing::Final,
            },
            Ok(SetLocalIdentityOutcome::Unchanged(local)) => Ran {
                answer: Answer::stopped(ResultCode::AlreadyApplied).with_data(data(local)),
                standing: Standing::Final,
            },
            // Changed between this call's observation and its lease:
            // what is there is not what was shown, as when the
            // observation itself finds it so.
            Err(error) if error.kind == RepositoryErrorKind::ExternalChange => Ran {
                answer: refused(error),
                standing: Standing::NotRun { journal: NO_ROW },
            },
            Err(error) => {
                // The file could not be put back: some of the identity
                // is written and no effect shows it.
                let local_change = error.kind == RepositoryErrorKind::RollbackIncomplete;
                Ran {
                    answer: Answer {
                        local_change,
                        ..refused(error)
                    },
                    standing: if accepted.attempt > 1 {
                        Standing::Unconfirmed
                    } else {
                        Standing::Stopped {
                            journal: Some(NO_ROW),
                        }
                    },
                }
            }
        }
    }
}

impl Binding for RepoIdentityBinding {
    fn command(&self) -> Command {
        Command::RepoIdentitySet
    }

    /// The command has no operation, so no row of one is ever read.
    fn ended_without_checkpoint(&self) -> EndedWithoutCheckpoint {
        EndedWithoutCheckpoint::SaysNothing
    }

    /// The repository the identity is set on, by its root on disk: it
    /// need not be registered.
    fn place(&self) -> Place<'_> {
        Place::Repository(&self.root)
    }

    /// The repository as a whole: the command names nothing within it.
    fn target(&self) -> String {
        String::new()
    }

    fn fields(&self, builder: IntentDigestBuilder) -> IntentDigestBuilder {
        let Self {
            // What the digest is begun with.
            root: _,
            name,
            email,
            initials,
            // What a call observed, and no part of the request.
            seen: _,
        } = self;
        builder
            .field("name", FieldValue::Text(name))
            .field("email", FieldValue::Text(email))
            .field(
                "initials",
                match initials {
                    InitialsChange::Unchanged => FieldValue::Absent,
                    InitialsChange::Clear => FieldValue::Null,
                    InitialsChange::Set(initials) => FieldValue::Text(initials),
                },
            )
    }

    fn scope(&self, repository: Option<String>, _position: Option<&Position>) -> Scope {
        Scope {
            repository,
            ..Scope::default()
        }
    }

    /// Checks the input and observes. No check is made of what the
    /// preview covers: the boundary compares the digest with the
    /// confirmation's, and refuses a request whose preview is stale as
    /// `external_change`.
    fn prepare(&mut self, _call: &CallContext<'_>) -> Result<Prepared, Answer> {
        let observed = self.observe()?;
        let observation = Self::digest(&observed.current).to_stored();
        self.seen = Some(observed.current);
        Ok(Prepared {
            // No operation ID and no row.
            family: None,
            // Nothing is committed, so there is no branch to bound.
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

    /// The configuration is asked, since there is no row to ask; see the
    /// module's description for the three things it can hold.
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

impl Confirmed for RepoIdentityBinding {
    /// No row is read for a request with no operation. Nothing is run
    /// or settled on one that is.
    fn row(&mut self, _call: &CallContext<'_>, _accepted: &Accepted, journal: &JournalRow) -> Row {
        if *journal == NO_ROW {
            Row::Unrecorded
        } else {
            Row::Impossible
        }
    }

    fn observe_again(
        &mut self,
        _call: &CallContext<'_>,
        _accepted: &Accepted,
        _journal: &JournalRow,
    ) -> Result<Seen, Ran> {
        let observed = match self.observe() {
            Ok(observed) => observed,
            // Nothing is known of the configuration: the record is kept.
            Err(answer) => {
                return Err(Ran {
                    answer,
                    standing: Standing::Stopped { journal: None },
                });
            }
        };
        if observed.asked.held_by(&observed.current) {
            // By this request's work, whose result was lost, or by
            // another's: the configuration does not say which, and
            // either way nothing is left to write.
            return Ok(Seen::EndState(
                Answer::stopped(ResultCode::AlreadyApplied).with_data(data(observed.current)),
            ));
        }
        let found = Self::digest(&observed.current);
        // What the domain call is told the configuration was seen as.
        // It is made only when that is what was shown.
        self.seen = Some(observed.current);
        Ok(Seen::State { found, own: None })
    }

    fn preview(&mut self, _call: &CallContext<'_>) -> Result<Previewed, Answer> {
        let Observed {
            repository,
            root,
            asked,
            current,
        } = self.observe()?;
        if asked.held_by(&current) {
            // The command's end state holds.
            return Err(Answer::stopped(ResultCode::AlreadyApplied));
        }
        let effective_config = repository
            .config()
            .map_err(|_| Answer::stopped(ResultCode::RepositoryInaccessible))?;
        let effective = identity_dto(
            &repository,
            &root,
            &effective_config,
            RepositoryOperation::SetLocalIdentity,
        )
        .map_err(|error| Answer::of_read(&error))?;
        let LocalIdentity {
            name,
            email,
            initials,
        } = asked.written_over(&current);
        let observation = Self::digest(&current).to_stored();
        Ok(Previewed {
            observation,
            effects: vec![PreviewEffect::IdentityWritten],
            details: PreviewDetailsDto::Identity(IdentityPreviewDto {
                root: root.to_string_lossy().into_owned(),
                // A valid request names both.
                name: name.unwrap_or_default(),
                email: email.unwrap_or_default(),
                initials,
                current: LocalIdentityDto {
                    name: current.name,
                    email: current.email,
                    initials: current.initials,
                },
                effective,
            }),
        })
    }
}
