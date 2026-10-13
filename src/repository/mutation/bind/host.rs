//! `host approve` and `host replace`: the host of a repository's
//! publication remote is pinned to a key, or its pin is changed to
//! another. Both need confirmation. And `host inspect`, the read that
//! shows what a host presents before either.
//!
//! A host is named by its authority: its name in lower case, and its
//! port. The host that can be approved is one the selected publication
//! remote names, for fetching or for pushing: those are the only hosts
//! anything connects to.
//!
//! **Nothing here writes a pin.** The one writer is the verification of
//! the transport, `verify_ssh_transport`: it connects to the host,
//! authenticates with the selected key, and pins the key the request
//! names only if that is the key the host presented, in the pin store's
//! own transaction. It fetches nothing, pushes nothing and moves no
//! reference. The binding gives it the exact approval: the host, the
//! pin it saw the host have, and the key to pin.
//!
//! **What a preview covers.** Which remote is the publication remote,
//! where it is fetched from and pushed to as far as that is this host,
//! and the pin the host has, or that it has none. `prepare` reads all of
//! it and opens no connection.
//!
//! **No operation and no row.** What says a request's work is done is
//! the pin store:
//!
//! - the host is pinned to the key the request names: the request's end
//!   state holds, by its own work or another's, and the request is a
//!   no-op. A request whose result was lost after the pin was stored is
//!   answered so, and never as `external_change`;
//! - the pin is what was previewed: nothing of the request has been
//!   done, and the host is connected to;
//! - anything else: what is there is not what was shown, and the answer
//!   is `external_change`, before any connection.
//!
//! The comparison with the preview is the consent's own observation
//! digest, which the request's record holds through its confirmation,
//! so the record needs no `expected_digest` beside it. A pin is stored
//! whole or not at all, so nothing of a request is ever there in part.
//! The verification compares once more, in the transaction that writes:
//! a pin that moved after this call observed it is refused there.
//!
//! What the comparison cannot tell apart is a pin that was changed and
//! put back as it was previewed: a retry then connects, and pins the
//! key only if the host still presents it.
//!
//! **Nothing private.** A fingerprint, a host and a port are what the
//! commands are given and what they return. No location of the remote,
//! no key path, no passphrase and nothing a server sent reaches a
//! result, a record or the observation digest's input beyond the
//! digest itself.

use std::path::{Path, PathBuf};

use crate::{
    repository::mutation::confirm::{self, Row, Seen},
    results::{Envelope, ResultCode, Scope},
};

use super::{
    super::{
        super::{
            ConfigurationInspection, RepositoryService,
            keys::{SessionCredentialProvider, SessionCredentials},
            read::stored_authority,
            read_configuration,
            recovery::JournalRow,
            transport::{
                HostApproval, HostKeyIdentity, SshAuthority, SshDirection, SshTransportError,
                SshTransportErrorKind, VerifySshTransportRequest,
                endpoint::configured_remote_endpoint, trust::valid_identity,
            },
        },
        command::Command,
        dto::{
            HostMutationDto, HostObservationDto, HostPreviewDto, MutationDataDto,
            PreviewDetailsDto, PreviewEffect,
        },
        evidence::Position,
        identity::{FieldValue, IntentDigestBuilder},
        observe::{Observation, ObservationDigest},
        outcome,
    },
    Accepted, Answer, Binding, CallContext, Confirmed, EndedWithoutCheckpoint, Entered, Place,
    Prepared, Previewed, Ran, Standing, refused, resolve_root,
};

/// The input of `host approve`: a repository, the host its publication
/// remote names, and the key the caller was shown the host present.
#[derive(Clone)]
pub struct HostApproveInput {
    pub root: PathBuf,
    pub authority: SshAuthority,
    /// The algorithm of the key, as `host inspect` and a
    /// synchronization that stopped for the host give it.
    pub algorithm: String,
    /// The key's fingerprint: `SHA256:` and the digest of the public key.
    pub fingerprint: String,
}

/// The input of `host replace`: as for `host approve`, and the
/// fingerprint the host is pinned to now.
#[derive(Clone)]
pub struct HostReplaceInput {
    pub root: PathBuf,
    pub authority: SshAuthority,
    pub algorithm: String,
    pub fingerprint: String,
    /// The fingerprint of the pin that is replaced.
    pub old_fingerprint: String,
}

/// The command `observe_host` answers.
const HOST_INSPECT: &str = "host inspect";

/// No row is ever read for the commands: they have no operation.
const NO_ROW: JournalRow = JournalRow::Absent;

/// The host as a record's target names it: `host:port`, with an address
/// that holds a colon in brackets.
fn authority_text(authority: &SshAuthority) -> String {
    if authority.host.contains(':') {
        format!("[{}]:{}", authority.host, authority.port)
    } else {
        format!("{}:{}", authority.host, authority.port)
    }
}

/// Whether `fingerprint` is one a pin can hold: `SHA256:` and a digest,
/// in the one way a fingerprint is written.
fn valid_fingerprint(fingerprint: &str) -> bool {
    valid_identity(&HostKeyIdentity {
        // Any algorithm a pin can hold: only the fingerprint is asked of.
        algorithm: "ssh-ed25519".to_owned(),
        sha256: fingerprint.to_owned(),
    })
}

/// What the publication remote of a repository uses a host for, and the
/// pin the host has.
struct Host {
    /// The repository's root, as it is registered.
    root: PathBuf,
    /// The host, as a pin stores it.
    authority: SshAuthority,
    /// The publication remote.
    remote: String,
    /// Where the remote is fetched from, when that is this host: the
    /// location as the transport connects to it. It goes into the
    /// observation digest and nowhere else.
    fetch: Option<String>,
    /// Where the remote is pushed to, when that is this host.
    push: Option<String>,
    pin: Option<HostKeyIdentity>,
    reapproval_required: bool,
    /// What neither location of the remote could be read for, when
    /// neither could: the remote is then no host's.
    unusable: Option<ResultCode>,
}

impl Host {
    /// Reads what the publication remote of the repository at `root`
    /// uses `authority` for, and the host's pin. `authority` is `None`
    /// for the host the remote is fetched from. No connection is
    /// opened and nothing is written.
    fn read(
        service: &RepositoryService,
        root: &Path,
        authority: Option<&SshAuthority>,
    ) -> Result<Self, Answer> {
        let authority = match authority {
            Some(authority) => Some(
                stored_authority(authority)
                    .ok_or_else(|| Answer::stopped(ResultCode::InvalidInput))?,
            ),
            None => None,
        };
        let resolved = resolve_root(service, root)?;
        let root = resolved.root().to_owned();
        let remote = match read_configuration(&root).map_err(refused)? {
            ConfigurationInspection::Valid(configuration) => configuration
                .publication_remote
                .ok_or_else(|| Answer::stopped(ResultCode::PublicationRemoteRequired))?,
            ConfigurationInspection::Missing | ConfigurationInspection::Invalid(_) => {
                return Err(Answer::stopped(ResultCode::InvalidConfiguration));
            }
        };
        let repository = git2::Repository::open(&root)
            .map_err(|_| Answer::stopped(ResultCode::RepositoryInaccessible))?;
        let fetch = configured_remote_endpoint(&repository, &remote, SshDirection::Fetch);
        let push = configured_remote_endpoint(&repository, &remote, SshDirection::Push);
        let unusable = match (&fetch, &push) {
            (Err(kind), Err(_)) => Some(outcome::ssh_transport_error_code(kind)),
            (Ok(_), _) | (_, Ok(_)) => None,
        };
        let authority = match authority {
            Some(authority) => authority,
            // The host the remote is fetched from.
            None => match &fetch {
                Ok(endpoint) => endpoint.authority.clone(),
                Err(kind) => {
                    return Err(Answer::stopped(outcome::ssh_transport_error_code(kind)));
                }
            },
        };
        let serves = |endpoint: Result<_, SshTransportErrorKind>| {
            endpoint
                .ok()
                .filter(
                    |endpoint: &super::super::super::transport::endpoint::SshEndpoint| {
                        endpoint.authority == authority
                    },
                )
                .map(|endpoint| endpoint.connection_url)
        };
        let (fetch, push) = (serves(fetch), serves(push));
        let trust = service
            .read_host_trust(&authority)
            .map_err(|kind| Answer::stopped(outcome::ssh_transport_error_code(&kind)))?;
        Ok(Self {
            root,
            authority,
            remote,
            fetch,
            push,
            pin: trust.pin,
            reapproval_required: trust.reapproval_required,
            unusable,
        })
    }

    /// The direction the host is connected to in: the one the remote
    /// uses it for, and fetching when it uses it for both. `Err` for a
    /// host the remote does not name.
    fn direction(&self) -> Result<SshDirection, Answer> {
        match (&self.fetch, &self.push) {
            (Some(_), _) => Ok(SshDirection::Fetch),
            (None, Some(_)) => Ok(SshDirection::Push),
            (None, None) => Err(Answer::stopped(
                self.unusable.unwrap_or(ResultCode::InvalidInput),
            )),
        }
    }

    /// The digest of what was read: what a confirmation of `command`
    /// stores, and what a later observation is compared with.
    fn digest(&self, command: Command) -> ObservationDigest {
        let Self {
            // The scope the confirmation is of.
            root: _,
            // The record's target.
            authority: _,
            remote,
            fetch,
            push,
            pin,
            // The marker is left as it is, and is no part of the pin.
            reapproval_required: _,
            // A remote with no location is seen in `fetch` and `push`.
            unusable: _,
        } = self;
        let name = command.name();
        let mut observation = Observation::of(name).text("remote", remote);
        for (key, value) in [
            ("fetch", fetch.as_deref()),
            ("push", push.as_deref()),
            (
                "pin_algorithm",
                pin.as_ref().map(|pin| pin.algorithm.as_str()),
            ),
            (
                "pin_fingerprint",
                pin.as_ref().map(|pin| pin.sha256.as_str()),
            ),
        ] {
            observation = match value {
                Some(value) => observation.text(key, value),
                None => observation.absent(key),
            };
        }
        ObservationDigest::of_state(name, observation.finish())
    }
}

/// `host approve` and `host replace`. Every effect of either is
/// `not_requested`: its data, with the outcome, is what says what it
/// did.
pub(crate) struct HostBinding {
    command: Command,
    root: PathBuf,
    authority: SshAuthority,
    algorithm: String,
    fingerprint: String,
    /// The fingerprint of the pin a `host replace` replaces. `None` for
    /// `host approve`, which is of a host that has no pin.
    old_fingerprint: Option<String>,
    /// What this call saw. The verification pins only while the host
    /// still has the pin that was seen. Set by every observation that
    /// leads to a connection.
    seen: Option<Sighting>,
}

/// What a call saw of a host before it connected to it.
#[derive(Clone)]
struct Sighting {
    /// The direction the host is connected to in.
    direction: SshDirection,
    /// The pin the host had.
    pin: Option<HostKeyIdentity>,
    /// Whether the reapproval marker was there.
    reapproval_required: bool,
}

impl Sighting {
    /// What is connected to for `host`. `None` for a host the remote
    /// does not name: it gives a digest no preview was given of, so no
    /// connection follows from it.
    fn of(host: &Host) -> Option<Self> {
        Some(Self {
            direction: host.direction().ok()?,
            pin: host.pin.clone(),
            reapproval_required: host.reapproval_required,
        })
    }
}

impl From<HostApproveInput> for HostBinding {
    fn from(input: HostApproveInput) -> Self {
        // Every field is named: one added to the input does not compile
        // until it is taken into the binding or left out here.
        let HostApproveInput {
            root,
            authority,
            algorithm,
            fingerprint,
        } = input;
        Self {
            command: Command::HostApprove,
            root,
            authority,
            algorithm,
            fingerprint,
            old_fingerprint: None,
            seen: None,
        }
    }
}

impl From<HostReplaceInput> for HostBinding {
    fn from(input: HostReplaceInput) -> Self {
        let HostReplaceInput {
            root,
            authority,
            algorithm,
            fingerprint,
            old_fingerprint,
        } = input;
        Self {
            command: Command::HostReplace,
            root,
            authority,
            algorithm,
            fingerprint,
            old_fingerprint: Some(old_fingerprint),
            seen: None,
        }
    }
}

impl HostBinding {
    /// The key the request names.
    fn approved(&self) -> HostKeyIdentity {
        HostKeyIdentity {
            algorithm: self.algorithm.clone(),
            sha256: self.fingerprint.clone(),
        }
    }

    /// Checks the input, and reads what a preview covers. An `Err` is an
    /// input that names no host or no key, a repository that has no
    /// publication remote, or a read that failed.
    fn observe(&self, service: &RepositoryService) -> Result<Host, Answer> {
        let fingerprints_are_valid = valid_identity(&self.approved())
            && self
                .old_fingerprint
                .as_deref()
                .is_none_or(valid_fingerprint);
        if !fingerprints_are_valid {
            return Err(Answer::stopped(ResultCode::InvalidInput));
        }
        Host::read(service, &self.root, Some(&self.authority))
    }

    /// Whether the host is pinned to the key the request names.
    fn held_by(&self, host: &Host) -> bool {
        host.pin.as_ref() == Some(&self.approved())
    }

    /// The request's data: the host, and the key the request names.
    /// `presented` is the key the host presented instead, when it
    /// presented another.
    fn data(
        &self,
        authority: &SshAuthority,
        presented: Option<&HostKeyIdentity>,
    ) -> MutationDataDto {
        MutationDataDto::Host(HostMutationDto {
            host: authority.host.clone(),
            port: authority.port,
            algorithm: self.algorithm.clone(),
            fingerprint: self.fingerprint.clone(),
            presented_algorithm: presented.map(|presented| presented.algorithm.clone()),
            presented_fingerprint: presented.map(|presented| presented.sha256.clone()),
        })
    }

    /// What a call that failed leaves. The call that accepted the
    /// request is refused and its record released. A later call of the
    /// request keeps the record: it is the request's, and the same
    /// request is what goes on. Should the pin have been stored all the
    /// same, the next call finds the request's end state and is a
    /// no-op.
    fn failed(accepted: &Accepted, code: ResultCode) -> Ran {
        Ran {
            answer: Answer::stopped(code),
            standing: if accepted.attempt > 1 {
                Standing::Unconfirmed
            } else {
                Standing::Stopped {
                    journal: Some(NO_ROW),
                }
            },
        }
    }

    /// The one domain call, and what it came to: the host is connected
    /// to in the direction the remote uses it for, and pinned to the
    /// key the request names if that is the key it presents. A call
    /// that did not end well is answered by `stored_all_the_same`.
    fn call(&self, call: &CallContext<'_>, accepted: &Accepted) -> Ran {
        let ran = self.attempt(call, accepted);
        match ran.standing {
            Standing::Final => ran,
            Standing::Owed
            | Standing::Stopped { journal: _ }
            | Standing::Unconfirmed
            | Standing::NotRun { journal: _ } => self.stored_all_the_same(call, accepted, ran),
        }
    }

    /// The answer to a call that failed, once the pin store is asked
    /// what the call left. The verification stores the pin and then
    /// still has work that can fail: it reads what the host advertises,
    /// and checks once more that the key, the remote and its location
    /// are what it began with. A call that failed there has pinned the
    /// host all the same, and no answer may say that nothing changed.
    ///
    /// Where the host is now pinned to the key the request names, the
    /// answer keeps the code of what stopped the call and says the rest:
    /// a durable change was made, so the outcome is `partial`; the data
    /// names the host and the key; the record is kept, and the same
    /// request, which then finds its end state and is a no-op, is what
    /// is offered. Otherwise `ran` is the answer as it is. A pin store
    /// that cannot be read says nothing, and `ran` stands.
    fn stored_all_the_same(&self, call: &CallContext<'_>, accepted: &Accepted, ran: Ran) -> Ran {
        let Some(authority) = stored_authority(&self.authority) else {
            return ran;
        };
        let stored = call
            .service
            .read_host_trust(&authority)
            .is_ok_and(|trust| trust.pin.as_ref() == Some(&self.approved()));
        if !stored {
            return ran;
        }
        Ran {
            answer: Answer {
                data: Some(self.data(&authority, None)),
                recovery: vec![outcome::same_request(accepted.request_id)],
                local_change: true,
                ..Answer::stopped(ran.answer.code)
            },
            standing: Standing::Unconfirmed,
        }
    }

    /// One connection to the host, and what it came to, before the pin
    /// store is asked what it left; see `call`.
    fn attempt(&self, call: &CallContext<'_>, accepted: &Accepted) -> Ran {
        // Not observed: nothing is known of the pin the approval is
        // for, and nothing is connected to on that.
        let Some(Sighting {
            direction,
            pin,
            reapproval_required,
        }) = self.seen.clone()
        else {
            return Ran {
                answer: Answer::stopped(ResultCode::InternalError),
                standing: Standing::Stopped { journal: None },
            };
        };
        let Some(authority) = stored_authority(&self.authority) else {
            return Self::failed(accepted, ResultCode::InvalidInput);
        };
        let approved = self.approved();
        let verified = call.session.verify_transport(
            call.service,
            VerifySshTransportRequest {
                root: accepted.root(),
                direction,
                approval: Some(HostApproval {
                    authority: authority.clone(),
                    expected: pin.clone(),
                    presented: approved.clone(),
                }),
            },
        );
        // What is there is not what this call observed: the remote
        // names another host by now, or the host's pin has moved. The
        // record is released, as when the observation itself finds it.
        let changed = || Ran {
            answer: Answer::stopped(ResultCode::ExternalChange),
            standing: Standing::NotRun { journal: NO_ROW },
        };
        let error = match verified {
            // The session has no credentials to connect with.
            None => return Self::failed(accepted, ResultCode::InternalError),
            Some(Ok(verified)) => {
                return if verified.authority == authority && verified.host_key == approved {
                    Ran {
                        answer: Answer::stopped(ResultCode::Ok)
                            .with_data(self.data(&authority, None)),
                        standing: Standing::Final,
                    }
                } else {
                    changed()
                };
            }
            Some(Err(error)) => error,
        };
        let SshTransportError {
            authority: reached,
            kind,
            // The rest says where the verification was: nothing of it
            // is reported.
            root: _,
            remote_name: _,
            direction: _,
            selected_key_id: _,
        } = error;
        // The host presented another key than the one the request
        // names. Nothing was pinned, and the same request never gets
        // past it.
        let mismatch = |presented: &HostKeyIdentity| Ran {
            answer: Answer::stopped(ResultCode::HostMismatch)
                .with_data(self.data(&authority, Some(presented))),
            standing: Standing::NotRun { journal: NO_ROW },
        };
        match &kind {
            SshTransportErrorKind::HostApprovalRequired { presented }
            | SshTransportErrorKind::HostReplacementRequired {
                expected: _,
                presented,
            } => {
                if reached.as_ref() != Some(&authority) || *presented == approved {
                    // Another host than the one approved, or the key
                    // approved under another pin than the one seen.
                    changed()
                } else {
                    mismatch(presented)
                }
            }
            // The verification let the host's key pass and then pinned
            // nothing. It lets a key pass that is the host's pin or the
            // key approved, and it pins the key approved unless the pin
            // or the marker moved. So where neither has moved, the host
            // presented its pin and not the key the request names: a
            // replacement by a key the host does not present.
            //
            // That is said only of the host the request names. A remote
            // that was given another location meanwhile had another host
            // connected to, and nothing is known of what this one
            // presents.
            //
            // The inference has two edges. A host that presents two
            // keys within one connection is refused with the same kind,
            // and is reported here as presenting its pin. And a pin
            // changed and put back between this call's two reads is
            // reported as the key presented, whatever was.
            SshTransportErrorKind::HostTrustChanged => {
                if reached.as_ref() != Some(&authority) {
                    changed()
                } else {
                    match call.service.read_host_trust(&authority) {
                        Ok(now)
                            if now.pin == pin && now.reapproval_required == reapproval_required =>
                        {
                            match &pin {
                                Some(pinned) if *pinned != approved => mismatch(pinned),
                                Some(_) | None => changed(),
                            }
                        }
                        Ok(_) | Err(_) => changed(),
                    }
                }
            }
            SshTransportErrorKind::EndpointChanged => changed(),
            SshTransportErrorKind::ConfigurationInvalid
            | SshTransportErrorKind::PublicationRemoteMissing
            | SshTransportErrorKind::UsernameRequired
            | SshTransportErrorKind::NoSelectedKey
            | SshTransportErrorKind::KeyMissing
            | SshTransportErrorKind::KeyUnreadable
            | SshTransportErrorKind::KeySourceChanged
            | SshTransportErrorKind::SelectionChanged
            | SshTransportErrorKind::KeyInvalidOrUnsupported
            | SshTransportErrorKind::KeyRejected
            | SshTransportErrorKind::UnlockCancelled
            | SshTransportErrorKind::ProviderUnavailable
            | SshTransportErrorKind::UnlockFailed
            | SshTransportErrorKind::HostVerificationUnavailable
            | SshTransportErrorKind::RegistryUnavailable
            | SshTransportErrorKind::RuntimeUninitialized
            | SshTransportErrorKind::TransportUnavailable
            | SshTransportErrorKind::RemoteUnavailable
            | SshTransportErrorKind::PushRejected
            | SshTransportErrorKind::ProtocolFailure => {
                Self::failed(accepted, outcome::ssh_transport_error_code(&kind))
            }
        }
    }
}

impl Binding for HostBinding {
    fn command(&self) -> Command {
        self.command
    }

    /// The commands have no operation, so no row of one is ever read.
    fn ended_without_checkpoint(&self) -> EndedWithoutCheckpoint {
        EndedWithoutCheckpoint::SaysNothing
    }

    fn place(&self) -> Place<'_> {
        Place::Repository(&self.root)
    }

    /// The host, as the request names it.
    fn target(&self) -> String {
        authority_text(&self.authority)
    }

    fn fields(&self, builder: IntentDigestBuilder) -> IntentDigestBuilder {
        let Self {
            // What the digest is begun with.
            command: _,
            root: _,
            // The record's target.
            authority: _,
            algorithm,
            fingerprint,
            old_fingerprint,
            // What a call observed, and no part of the request.
            seen: _,
        } = self;
        builder
            .field("algorithm", FieldValue::Text(algorithm))
            .field("fingerprint", FieldValue::Text(fingerprint))
            .field(
                "old_fingerprint",
                match old_fingerprint {
                    Some(old) => FieldValue::Text(old),
                    None => FieldValue::Absent,
                },
            )
    }

    fn scope(&self, repository: Option<String>, _position: Option<&Position>) -> Scope {
        Scope {
            repository,
            ..Scope::default()
        }
    }

    /// Checks the input and observes. A host already pinned to the key
    /// the request names is the request's end state, and a no-op: so a
    /// request whose first call stored the pin and failed is answered.
    /// No other check is made of what the preview covers: the boundary
    /// compares the digest with the confirmation's, and refuses a
    /// request whose preview is stale as `external_change`.
    fn prepare(&mut self, call: &CallContext<'_>) -> Result<Prepared, Answer> {
        let host = self.observe(call.service)?;
        if self.held_by(&host) {
            return Err(Answer::stopped(ResultCode::AlreadyApplied)
                .with_data(self.data(&host.authority, None)));
        }
        let observation = host.digest(self.command).to_stored();
        self.seen = Sighting::of(&host);
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
        self.call(call, accepted)
    }

    /// The pin store is asked, since there is no row to ask; see the
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
        self.call(call, accepted)
    }
}

impl Confirmed for HostBinding {
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
        call: &CallContext<'_>,
        _accepted: &Accepted,
        _journal: &JournalRow,
    ) -> Result<Seen, Ran> {
        let host = match self.observe(call.service) {
            Ok(host) => host,
            // Nothing is known of the pin: the record is kept.
            Err(answer) => {
                return Err(Ran {
                    answer,
                    standing: Standing::Stopped { journal: None },
                });
            }
        };
        if self.held_by(&host) {
            // By this request's work, whose result was lost, or by
            // another's: the pin store does not say which, and either
            // way nothing is left to pin.
            return Ok(Seen::EndState(
                Answer::stopped(ResultCode::AlreadyApplied)
                    .with_data(self.data(&host.authority, None)),
            ));
        }
        let found = host.digest(self.command);
        // What the verification is told the pin was seen as. It is
        // made only when that is what was shown.
        self.seen = Sighting::of(&host);
        Ok(Seen::State { found, own: None })
    }

    fn preview(&mut self, call: &CallContext<'_>) -> Result<Previewed, Answer> {
        let host = self.observe(call.service)?;
        // A host the publication remote does not name.
        host.direction()?;
        if self.held_by(&host) {
            // The command's end state holds.
            return Err(Answer::stopped(ResultCode::AlreadyApplied));
        }
        let effect = match (&self.old_fingerprint, &host.pin) {
            // A host with no pin is approved.
            (None, None) => PreviewEffect::HostPinned,
            // A pin is changed by its replacement and by nothing else.
            (None, Some(_)) => {
                return Err(Answer::stopped(ResultCode::HostReplacementRequired));
            }
            (Some(old), Some(pin)) if *old == pin.sha256 => PreviewEffect::HostPinReplaced,
            // The pin that is to be replaced is not the host's.
            (Some(_), Some(_) | None) => {
                return Err(Answer::stopped(ResultCode::InvalidInput));
            }
        };
        let observation = host.digest(self.command).to_stored();
        let Host {
            root,
            authority,
            remote: _,
            fetch,
            push,
            pin,
            reapproval_required,
            unusable: _,
        } = host;
        Ok(Previewed {
            observation,
            effects: vec![effect],
            details: PreviewDetailsDto::Host(HostPreviewDto {
                root: root.to_string_lossy().into_owned(),
                host: authority.host,
                port: authority.port,
                fetch: fetch.is_some(),
                push: push.is_some(),
                algorithm: self.algorithm.clone(),
                fingerprint: self.fingerprint.clone(),
                current_algorithm: pin.as_ref().map(|pin| pin.algorithm.clone()),
                current_fingerprint: pin.map(|pin| pin.sha256),
                reapproval_required,
            }),
        })
    }
}

impl RepositoryService {
    /// `host inspect`, with a connection: what the host `authority`
    /// presents now, and the pin it has. `authority` is a host the
    /// publication remote of the repository at `root` names, for
    /// fetching or for pushing; `None` is the host the remote is
    /// fetched from.
    ///
    /// This is a read over the network, with no request ID: it
    /// authenticates with the selected key, as every connection does,
    /// and pins nothing, whatever the host presents. A host that has no
    /// pin, and one that presents another key than its pin, are what it
    /// is for, and are reported as successes: the data says which.
    pub fn observe_host<P: SessionCredentialProvider>(
        &self,
        root: &Path,
        authority: Option<&SshAuthority>,
        credentials: &mut SessionCredentials<P>,
    ) -> Envelope<HostObservationDto> {
        let unresolved = || Scope {
            repository: crate::results::absolute_path_string(root),
            ..Scope::default()
        };
        let failure = |scope: Scope, code: ResultCode| {
            Envelope::failure(HOST_INSPECT, scope, code, Vec::new())
        };
        let host = match Host::read(self, root, authority) {
            Ok(host) => host,
            Err(answer) => return failure(unresolved(), answer.code),
        };
        let scope = Scope {
            repository: crate::results::absolute_path_string(&host.root),
            ..Scope::default()
        };
        let direction = match host.direction() {
            Ok(direction) => direction,
            Err(answer) => return failure(scope, answer.code),
        };
        let verified = self.verify_ssh_transport(
            VerifySshTransportRequest {
                root: host.root.clone(),
                direction,
                // Nothing is approved: a host that is not trusted is
                // refused, and what it presented comes back with that.
                approval: None,
            },
            credentials,
        );
        // The pin is read again where the host was trusted: it is the
        // pin the connection was checked against, unless it moved.
        let (reached, presented, pinned) = match verified {
            Ok(verified) => match self.read_host_trust(&verified.authority) {
                Ok(trust) => (Some(verified.authority), verified.host_key, trust.pin),
                Err(kind) => {
                    return failure(scope, outcome::ssh_transport_error_code(&kind));
                }
            },
            Err(error) => match error.kind {
                SshTransportErrorKind::HostApprovalRequired { presented } => {
                    (error.authority, presented, None)
                }
                SshTransportErrorKind::HostReplacementRequired {
                    expected,
                    presented,
                } => (error.authority, presented, Some(expected)),
                // Every other kind is a failure of the read, with its
                // code. Each is named: a kind added later that holds a
                // key the host presented is placed here, and is not
                // made a failure by default.
                kind @ (SshTransportErrorKind::ConfigurationInvalid
                | SshTransportErrorKind::PublicationRemoteMissing
                | SshTransportErrorKind::UsernameRequired
                | SshTransportErrorKind::NoSelectedKey
                | SshTransportErrorKind::KeyMissing
                | SshTransportErrorKind::KeyUnreadable
                | SshTransportErrorKind::KeySourceChanged
                | SshTransportErrorKind::SelectionChanged
                | SshTransportErrorKind::EndpointChanged
                | SshTransportErrorKind::KeyInvalidOrUnsupported
                | SshTransportErrorKind::KeyRejected
                | SshTransportErrorKind::UnlockCancelled
                | SshTransportErrorKind::ProviderUnavailable
                | SshTransportErrorKind::UnlockFailed
                | SshTransportErrorKind::HostTrustChanged
                | SshTransportErrorKind::HostVerificationUnavailable
                | SshTransportErrorKind::RegistryUnavailable
                | SshTransportErrorKind::RuntimeUninitialized
                | SshTransportErrorKind::TransportUnavailable
                | SshTransportErrorKind::RemoteUnavailable
                | SshTransportErrorKind::PushRejected
                | SshTransportErrorKind::ProtocolFailure) => {
                    return failure(scope, outcome::ssh_transport_error_code(&kind));
                }
            },
        };
        // The remote was given another location while it was read.
        if reached.as_ref() != Some(&host.authority) {
            return failure(scope, ResultCode::ExternalChange);
        }
        Envelope::read_success(
            HOST_INSPECT,
            scope,
            HostObservationDto {
                host: host.authority.host,
                port: host.authority.port,
                fetch: host.fetch.is_some(),
                push: host.push.is_some(),
                algorithm: presented.algorithm,
                fingerprint: presented.sha256,
                pinned_algorithm: pinned.as_ref().map(|pin| pin.algorithm.clone()),
                pinned_fingerprint: pinned.map(|pin| pin.sha256),
                reapproval_required: host.reapproval_required,
            },
        )
    }
}
