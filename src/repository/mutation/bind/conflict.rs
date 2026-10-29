//! `conflict resolve`: a remote synchronization's conflict, resolved in
//! Manyhands from the sources its caller gives, and the synchronization's
//! publication continued under the same request (R-14, R-15, R-35).
//!
//! The caller presents what `conflict show` gave it: the conflict's
//! observation and one string token per path, each with the source that
//! is to be the path's result. The library re-derives both from the live
//! conflict (R-9), and nothing the caller presents is a path.
//!
//! **No operation of its own (R-32, R-35).** The synchronization is
//! another request's, and is this request's target, as a resume's is. So
//! the request's record holds no operation, and settling deletes it on any
//! stop (fact 67). The durable state is the resolution attempt, which the
//! remote journal records against the synchronization, and which
//! `remote_resolution_owners` associates with this request by its ID and
//! the digest of the input it presented. A retry of the same request
//! finds its attempt there and finishes it, before any live token is
//! checked (decision 13): with the presented sources while the attempt
//! has only written paths, and from durable state alone once its commit
//! is prepared. A different input under the same request ID is
//! `request_mismatch`.
//!
//! **Another request's attempt (decision 18).** A new request never
//! begins an attempt beside an unfinished one (fact 71). It supersedes an
//! attempt that has only written paths while every path it wrote still
//! holds exactly what it wrote; with one changed it is
//! `original_request_required`, naming the attempt's request. An attempt
//! whose commit is prepared is finished by `operation resume`, which the
//! answer offers.
//!
//! **The continuation (R-15).** Once the resolution is checkpointed, the
//! request publishes it as the synchronization's own request would: a
//! comment's synchronization through the comment's retry, and any other
//! through a restart with the merging settings. Its `Watch` is this
//! request's, whose flag nothing sets (fact 11) and which clears it, so the
//! call's token stops it. It also reads the flag of the request that holds
//! the operation: the synchronization's own request, or the `comment add`
//! request whose record holds the comment's operation (fact 79). So
//! `request cancel` of that request stops it at the next safe point, once
//! the push is intended as before; that request's own next call settles its
//! record. A cancellation seen before the continuation starts stops after
//! the checkpoint, with the synchronization left resolved for `operation
//! resume`.
//!
//! The input types derive no `Debug`, `Display` or `Serialize`: they hold
//! sources. No source reaches a result, a record or a diagnostic.

use std::path::{Path, PathBuf};

use crate::results::{
    CheckpointEffect, DiscoveryEffect, Effects, IntegrationEffect, PublicationEffect,
    RecoveryAction, RecoveryActionKind, ResultCode, Scope, WriteEffect,
};

use super::{
    super::{
        super::{
            CommentPublicationPendingReason, CommentPublicationState, CommentSubmissionOutcome,
            ConflictEligibility, OperationFamily, OperationId, ReadError, RedactedConflictBytes,
            RepositoryOperation, RepositoryService, ResolveSynchronizationOutcome,
            ResolveSynchronizationRequest, ResolvedRepository, RetryCommentPublicationRequest,
            SynchronizationError, SynchronizationTarget, local_sync,
            recovery::{FinalKind, JournalRow},
            remote::{
                conflict_read,
                state::{self, ResolutionAttemptPhase, ResolutionAttemptView, ResolutionView},
            },
        },
        command::Command,
        dto::{
            ConflictNextAction, ConflictNextDto, ConflictResolutionMutationDto,
            ConflictResolutionMutationType, MutationDataDto,
        },
        evidence::Position,
        identity::{FieldValue, IntentDigestBuilder, RequestId},
        outcome,
        progress::Subject,
        records,
    },
    Accepted, Answer, Binding, CallContext, Confirmed, EndedWithoutCheckpoint, Entered, Place,
    Prepared, Ran, Standing,
    comment::comment_synchronization,
    refused, resolve_root, sync,
};

/// The input of `conflict resolve`: the synchronization whose conflict is
/// resolved, by its own ID or the ID of a comment it publishes (R-5); the
/// observation `conflict show` gave of the conflict; and a resolution of
/// every path, each by the token `conflict show` gave it.
pub struct ConflictResolveInput {
    pub root: PathBuf,
    pub operation_id: OperationId,
    pub observation: String,
    pub resolutions: Vec<ConflictResolutionInput>,
}

/// One path's resolution: the token `conflict show` gave the path, and the
/// source that is to be its result.
pub struct ConflictResolutionInput {
    pub token: String,
    pub source: String,
}

pub(crate) struct ConflictResolveBinding {
    root: PathBuf,
    operation_id: OperationId,
    observation: String,
    resolutions: Vec<ConflictResolutionInput>,
    /// The synchronization `operation_id` names, by its own ID or a
    /// comment's, once `settle_target` has looked it up: what the request's
    /// identity is of.
    synchronization: Option<OperationId>,
    /// This call entered the request again: an earlier call of it started
    /// and its end was not recorded.
    entered_again: bool,
}

impl From<ConflictResolveInput> for ConflictResolveBinding {
    fn from(input: ConflictResolveInput) -> Self {
        // Every field is named: one added to the input does not compile
        // until it is taken into the binding or left out here.
        let ConflictResolveInput {
            root,
            operation_id,
            observation,
            resolutions,
        } = input;
        Self {
            root,
            operation_id,
            observation,
            resolutions,
            synchronization: None,
            entered_again: false,
        }
    }
}

/// The prefix of an observation's string form, as `conflict show` gives it.
const OBSERVATION: &str = "conflict-v1:";
/// The prefix of a path token's string form.
const TOKEN: &str = "conflict-path-v1:";

/// Whether `text` is `prefix` and a BLAKE3 digest in lower-case hex: the
/// form R-9 gives an observation and a token.
fn well_formed(text: &str, prefix: &str) -> bool {
    text.strip_prefix(prefix).is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

/// One field of a digest: its length, then its bytes.
fn field(hash: &mut blake3::Hasher, bytes: &[u8]) {
    hash.update(&(bytes.len() as u64).to_be_bytes());
    hash.update(bytes);
}

/// The resolution attempt of the request `request` for the conflict at
/// `window` and `ordinal` of the synchronization `synchronization` (R-14):
/// a ULID made from a domain-separated BLAKE3 digest of the four, so that
/// a request ID used again never meets another conflict's attempt. The
/// test target that plants a row under it derives it here too.
pub(crate) fn attempt_for(
    request: RequestId,
    synchronization: OperationId,
    window: u32,
    ordinal: u8,
) -> OperationId {
    let mut hash = blake3::Hasher::new();
    hash.update(b"manyhands-conflict-resolve-attempt-v1");
    field(&mut hash, request.to_string().as_bytes());
    field(&mut hash, synchronization.to_string().as_bytes());
    field(&mut hash, &window.to_be_bytes());
    field(&mut hash, &[ordinal]);
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&hash.finalize().as_bytes()[..16]);
    OperationId::parse(&ulid::Ulid::from_bytes(bytes).to_string())
        .expect("a ULID's own text is canonical")
}

/// What a request with no operation leaves of its record when it stops:
/// nothing is in flight under it, and the record goes (fact 67).
fn gone() -> Standing {
    Standing::Stopped {
        journal: Some(JournalRow::Absent),
    }
}

/// The effects of a resolution checkpointed by this request: the resolved
/// paths written and committed, the integration complete with the
/// resolution commit, and nothing published yet.
fn resolved(commit: git2::Oid) -> Effects {
    Effects {
        write: WriteEffect::Written,
        checkpoint: CheckpointEffect::Committed,
        integration: IntegrationEffect::Complete,
        publication: PublicationEffect::Pending,
        discovery: DiscoveryEffect::Pending,
        commit_oid: Some(commit.to_string()),
        ..Effects::not_requested()
    }
}

/// The data of a resolution's result.
fn data(
    synchronization: OperationId,
    commit: Option<git2::Oid>,
    publication: PublicationEffect,
    conflict: Option<ConflictNextDto>,
) -> MutationDataDto {
    MutationDataDto::ConflictResolution(ConflictResolutionMutationDto {
        r#type: ConflictResolutionMutationType::ConflictResolution,
        operation_id: synchronization.to_string(),
        commit_oid: commit.map(|commit| commit.to_string()),
        publication,
        conflict,
    })
}

/// A conflict that cannot be resolved in Manyhands, as a whole (decision
/// 12, R-40): `recovery_required`, and the next step is an outside repair.
/// Nothing is written.
fn external_repair(synchronization: OperationId) -> Answer {
    Answer::stopped(ResultCode::RecoveryRequired).with_data(data(
        synchronization,
        None,
        PublicationEffect::NotRequested,
        Some(ConflictNextDto {
            operation_id: synchronization.to_string(),
            next_action: ConflictNextAction::ExternalRepair,
        }),
    ))
}

/// Another request's attempt has only written paths, and one of them has
/// changed since: only that request goes on with it (decision 18).
fn original_request(attempt: &ResolutionAttemptView) -> Answer {
    Answer {
        recovery: attempt
            .owner
            .iter()
            .map(|owner| outcome::original_request(owner.request_id.clone()))
            .collect(),
        ..Answer::stopped(ResultCode::OriginalRequestRequired)
    }
}

/// Another request's attempt has its commit prepared: `operation resume`
/// finishes it from durable state (decision 18), and the repository is
/// what is looked at otherwise. Nothing is written.
fn resume_first(root: &Path, synchronization: OperationId) -> Answer {
    let mut recovery = vec![RecoveryAction::for_operation(
        RecoveryActionKind::OperationResume,
        synchronization.to_string(),
        [],
    )];
    recovery.extend(outcome::inspect_the_root_it_lies_in(root));
    Answer {
        recovery,
        ..Answer::stopped(ResultCode::RecoveryRequired)
    }
}

/// The answer to a failure of the library's conflict reads or writes. Only
/// the error's kind is read: its text can hold a path.
fn failed(error: &SynchronizationError) -> Answer {
    Answer::stopped(outcome::synchronization_error_code(error))
}

/// What a request's own attempt had come to, as one read shows it: whether
/// a later read shows that the call moved it on.
type AttemptMark = Option<(ResolutionAttemptPhase, Option<git2::Oid>)>;

/// Where the resolution of the call came from, for its continuation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Checkpointed {
    /// This call made or finished the resolution commit.
    Now,
    /// An earlier call of the same request did, and nothing of it was left
    /// to finish.
    Before,
}

/// What a call works with once the request is accepted.
struct Resolving<'a> {
    call: &'a CallContext<'a>,
    accepted: &'a Accepted,
    repository: ResolvedRepository,
    synchronization: OperationId,
    target: SynchronizationTarget,
}

impl Resolving<'_> {
    fn service(&self) -> &RepositoryService {
        self.call.service
    }

    fn root(&self) -> &Path {
        self.repository.root()
    }

    /// The synchronization's resolution, as one read shows it.
    fn view(&self) -> Result<ResolutionView, Answer> {
        let id = self.repository.registration_id();
        let synchronization = self.synchronization;
        self.service()
            .read_session(RepositoryOperation::Read, |db| {
                Ok(state::resolution_view(db, id, synchronization)?)
            })
            .map_err(|error| Answer::of_read(&error))?
            .ok_or_else(|| Answer::stopped(ResultCode::OperationNotFound))
    }

    /// The mark of `attempt` as one read shows it; `None` for one with no
    /// row, and when it cannot be read.
    fn mark(&self, attempt: OperationId) -> AttemptMark {
        let view = self.view().ok()?;
        view.attempts
            .iter()
            .find(|found| found.attempt == attempt)
            .map(|found| (found.phase, found.candidate))
    }

    /// The effects of a call that stopped while it worked on `attempt`,
    /// which stood at `before` when the call began: what the call moved it
    /// to, and nothing when it moved nothing.
    fn effects_of(&self, attempt: OperationId, before: AttemptMark) -> Effects {
        let after = self.mark(attempt);
        if after == before {
            return Effects::not_requested();
        }
        match after {
            Some((ResolutionAttemptPhase::Applied, Some(commit))) => resolved(commit),
            Some((ResolutionAttemptPhase::CandidatePrepared, Some(commit)))
                if self.branch_holds(commit) =>
            {
                resolved(commit)
            }
            Some((
                ResolutionAttemptPhase::PathsApplying | ResolutionAttemptPhase::CandidatePrepared,
                _,
            )) => Effects {
                write: WriteEffect::Written,
                ..Effects::not_requested()
            },
            Some(_) | None => Effects::not_requested(),
        }
    }

    /// Whether the branch the synchronization synchronizes is at `commit`.
    fn branch_holds(&self, commit: git2::Oid) -> bool {
        let Some(branch) = self.accepted.position.base_ref.as_deref() else {
            return false;
        };
        git2::Repository::open(self.root())
            .and_then(|repository| repository.refname_to_id(branch))
            .is_ok_and(|tip| tip == commit)
    }

    /// Records that `attempt` is this request's, made with the input whose
    /// digest is `digest` (R-35).
    fn own(&self, attempt: OperationId, digest: [u8; 32]) -> Result<(), Answer> {
        let request = self.accepted.request_id.to_string();
        state::with_transaction(self.service(), self.root(), |tx, _| {
            state::record_resolution_owner(tx, attempt, &request, digest)
        })
        .map_err(refused)
    }

    /// Lets go of the record that `attempt` is this request's, when the
    /// domain refused the request before it recorded the attempt: the same
    /// request may then present another input. An attempt the domain did
    /// record is kept as the request's durable state. A failure to let go
    /// is the answer.
    fn disown(&self, attempt: OperationId) -> Result<(), Answer> {
        state::with_transaction(self.service(), self.root(), |tx, _| {
            // `false`: the attempt is recorded, and the record of its owner
            // stays with it.
            state::release_resolution_owner(tx, attempt).map(|_released| ())
        })
        .map_err(refused)
    }

    /// The request that holds the synchronization, whose `request cancel`
    /// is to reach the continuation (design, step 6): the synchronization's
    /// own request; or, for the synchronization of the comment `comment`,
    /// the `comment add` request, whose record holds the comment's
    /// operation and not its synchronization (fact 79). `None` when no
    /// unfinished record holds it.
    fn holder(&self, comment: Option<OperationId>) -> Result<Option<RequestId>, Answer> {
        let (family, operation_id) = match comment {
            Some(comment) => (OperationFamily::Local, comment),
            None => (OperationFamily::Remote, self.synchronization),
        };
        let scope = self.accepted.scope.as_str();
        let holder = self
            .service()
            .read_session(RepositoryOperation::Read, |db| {
                Ok(records::operation_holder(db, family, scope, operation_id)?)
            })
            .map_err(|error| Answer::of_read(&error))?;
        holder
            .map(|holder| {
                RequestId::parse(&holder).map_err(|_| Answer::stopped(ResultCode::InternalError))
            })
            .transpose()
    }
}

impl ConflictResolveBinding {
    /// Step 1, syntax: an observation and tokens in the form `conflict
    /// show` gives them, at least one resolution, and no token twice.
    fn well_formed(&self) -> bool {
        let mut tokens = std::collections::BTreeSet::new();
        well_formed(&self.observation, OBSERVATION)
            && !self.resolutions.is_empty()
            && self.resolutions.iter().all(|resolution| {
                well_formed(&resolution.token, TOKEN) && tokens.insert(&resolution.token)
            })
    }

    /// The resolutions in token order, as the digests take them.
    fn ordered(&self) -> Vec<&ConflictResolutionInput> {
        let mut ordered: Vec<_> = self.resolutions.iter().collect();
        ordered.sort_by(|left, right| left.token.cmp(&right.token));
        ordered
    }

    /// The digest of the input the request presents, for the
    /// synchronization `synchronization` (R-35): its ID, the observation,
    /// and every token and source in token order.
    fn input_digest(&self, synchronization: OperationId) -> [u8; 32] {
        let mut hash = blake3::Hasher::new();
        hash.update(b"manyhands-conflict-resolve-input-v1");
        field(&mut hash, synchronization.to_string().as_bytes());
        field(&mut hash, self.observation.as_bytes());
        for resolution in self.ordered() {
            field(&mut hash, resolution.token.as_bytes());
            field(&mut hash, resolution.source.as_bytes());
        }
        *hash.finalize().as_bytes()
    }

    /// The synchronization the request names, by its own ID or a
    /// comment's (R-5), and what it synchronizes. A local-context sync's ID
    /// is `invalid_input` (decision 19, R-37), found before the remote
    /// lookup; so is an operation that is no synchronization. An ID no
    /// operation of the repository has is `operation_not_found`.
    fn synchronization(
        &self,
        service: &RepositoryService,
        repository: &ResolvedRepository,
    ) -> Result<(OperationId, SynchronizationTarget), Answer> {
        enum Found {
            LocalSync,
            Missing,
            NotSynchronization,
            Synchronization(OperationId, SynchronizationTarget),
        }
        let root = repository.root().to_owned();
        let operation_id = self.operation_id;
        let found = service
            .read_session(RepositoryOperation::Read, |db| {
                if local_sync::record::owned_identity(db, &operation_id.to_string())? {
                    return Ok(Found::LocalSync);
                }
                let synchronization = comment_synchronization(db, &root, operation_id)?
                    .map_or(operation_id, |(_, synchronization)| synchronization);
                // A root that is not UTF-8 cannot have been stored.
                let Some(root) = root.to_str() else {
                    return Ok(Found::Missing);
                };
                Ok(
                    match state::lookup_synchronization_target(db, root, synchronization)? {
                        None => Found::Missing,
                        Some(None) => Found::NotSynchronization,
                        Some(Some(target)) => Found::Synchronization(synchronization, target),
                    },
                )
            })
            .map_err(|error| Answer::of_read(&error))?;
        match found {
            Found::Synchronization(synchronization, target) => Ok((synchronization, target)),
            Found::LocalSync | Found::NotSynchronization => {
                Err(Answer::stopped(ResultCode::InvalidInput))
            }
            // Any other operation of the repository is no synchronization.
            Found::Missing => Err(match service.show_operation(repository, operation_id) {
                Ok(_) => Answer::stopped(ResultCode::InvalidInput),
                Err(error) => Answer::of_read(&error),
            }),
        }
    }

    /// The attempt this request is recorded as the owner of for the
    /// synchronization's conflicts, with the digest of the input it
    /// presented (step 2): the one whose ID is derived from this request
    /// and a step of the synchronization that met a conflict.
    fn own_attempt(
        &self,
        resolving: &Resolving<'_>,
        view: &ResolutionView,
    ) -> Result<Option<(OperationId, [u8; 32])>, Answer> {
        let request = resolving.accepted.request_id;
        let owned = resolving
            .service()
            .read_session(RepositoryOperation::Read, |db| {
                Ok(state::resolution_owned_by(db, &request.to_string())?)
            })
            .map_err(|error| Answer::of_read(&error))?;
        Ok(view.steps.iter().find_map(|step| {
            let derived = attempt_for(
                request,
                resolving.synchronization,
                step.window_number,
                step.ordinal,
            );
            owned
                .iter()
                .find(|(attempt, _)| *attempt == derived)
                .copied()
        }))
    }

    /// The one call of the request: steps 2 to 7 of the design's
    /// Conflict Resolution section. Whatever stops it leaves no record.
    fn call(&self, call: &CallContext<'_>, accepted: &Accepted) -> Ran {
        match self.resolve(call, accepted) {
            Ok(ran) => ran,
            Err(answer) => Ran {
                answer,
                standing: gone(),
            },
        }
    }

    fn resolve(&self, call: &CallContext<'_>, accepted: &Accepted) -> Result<Ran, Answer> {
        let repository = resolve_root(call.service, &accepted.root())?;
        let (synchronization, target) = self.synchronization(call.service, &repository)?;
        let resolving = Resolving {
            call,
            accepted,
            repository,
            synchronization,
            target,
        };
        let view = resolving.view()?;
        let digest = self.input_digest(synchronization);
        // Step 2: replay first. This request's own attempt is reconciled
        // before any live token is checked, by the digest stored with it.
        if let Some((attempt, recorded)) = self.own_attempt(&resolving, &view)? {
            if recorded != digest {
                return Err(Answer::stopped(ResultCode::RequestMismatch));
            }
            let Some(own) = view.attempts.iter().find(|found| found.attempt == attempt) else {
                // Recorded as this request's, and refused or stopped before
                // the domain recorded it: nothing was written.
                return self.fresh(&resolving, &view, Some(attempt), digest);
            };
            let before = Some((own.phase, own.candidate));
            if own.released {
                let commit = own
                    .candidate
                    .ok_or_else(|| Answer::stopped(ResultCode::RecoveryRequired))?;
                return Ok(self.continued(&resolving, commit, Checkpointed::Before));
            }
            return match own.phase {
                ResolutionAttemptPhase::Prepared => {
                    self.fresh(&resolving, &view, Some(attempt), digest)
                }
                // Its commit is prepared, or applied with its artifacts
                // held: finished from durable state alone (fact 34).
                ResolutionAttemptPhase::CandidatePrepared | ResolutionAttemptPhase::Applied => {
                    let finished = resolving.service().finish_resolution_attempt(
                        resolving.root(),
                        synchronization,
                        attempt,
                    );
                    match finished {
                        Ok(Some(outcome)) => {
                            self.reported(&resolving, attempt, before, Ok(outcome))
                        }
                        Ok(None) => Err(Answer::stopped(ResultCode::RecoveryRequired)),
                        Err(error) => self.reported(&resolving, attempt, before, Err(error)),
                    }
                }
                // Its paths are written and no candidate exists: the
                // presented sources and the live inspection finish it.
                ResolutionAttemptPhase::PathsApplying => {
                    let sources: Vec<RedactedConflictBytes> = self
                        .resolutions
                        .iter()
                        .map(|resolution| {
                            RedactedConflictBytes::from_bytes(resolution.source.as_bytes().to_vec())
                        })
                        .collect();
                    let again = resolving.service().resolution_request_again(
                        resolving.root(),
                        synchronization,
                        attempt,
                        &sources,
                    );
                    match again {
                        Ok(Some(request)) => {
                            let outcome =
                                resolving.service().resolve_synchronization(request.alone());
                            self.reported(&resolving, attempt, before, outcome)
                        }
                        Ok(None) => Err(Answer::stopped(ResultCode::RecoveryRequired)),
                        Err(error) => Err(failed(&error)),
                    }
                }
                ResolutionAttemptPhase::RecoveryRequired => {
                    Err(Answer::stopped(ResultCode::RecoveryRequired))
                }
            };
        }
        self.fresh(&resolving, &view, None, digest)
    }

    /// Steps 3 to 5 for a request with no attempt the domain recorded:
    /// another request's unfinished attempt, validation of the presented
    /// input against the live conflict, and the resolution. `attempt` is
    /// this request's when it is recorded as the owner of one.
    fn fresh(
        &self,
        resolving: &Resolving<'_>,
        view: &ResolutionView,
        attempt: Option<OperationId>,
        digest: [u8; 32],
    ) -> Result<Ran, Answer> {
        let service = resolving.service();
        let root = resolving.root();
        let synchronization = resolving.synchronization;
        // Step 3: another request's unfinished attempt (decision 18).
        let mut superseded = None;
        if let Some(other) = view
            .unfinished()
            .filter(|other| Some(other.attempt) != attempt)
        {
            if !view.has_written_paths_only(other) {
                return Err(resume_first(root, synchronization));
            }
            if !service
                .resolution_paths_unchanged(root, synchronization, other.attempt)
                .map_err(|error| failed(&error))?
            {
                return Err(original_request(other));
            }
            superseded = Some(other.clone());
        }
        // Step 4: validation, in the design's order. Nothing is written in
        // any of these cases.
        let Some(live) = view.live_step() else {
            // 4.1: a conflict already resolved, or repaired outside (fact
            // 34). A synchronization that met no conflict has none to
            // resolve.
            return Err(Answer::stopped(if view.steps.is_empty() {
                ResultCode::InvalidInput
            } else {
                ResultCode::RecoveryRequired
            }));
        };
        let inspection = service
            .inspect_synchronization_recovery(root, synchronization)
            .map_err(|error| failed(&error))?;
        // 4.2
        if conflict_read::observation_text(&inspection.observation) != self.observation {
            return Err(Answer::stopped(ResultCode::ExternalChange));
        }
        // 4.3: an external-only set, incompatible closures among it (R-40).
        if inspection.paths.is_empty()
            || inspection
                .paths
                .iter()
                .any(|path| path.eligibility != ConflictEligibility::EligibleCanonical)
        {
            return Err(external_repair(synchronization));
        }
        // 4.4: count before membership, as the gate orders them (fact 53).
        if self.resolutions.len() != inspection.paths.len() {
            return Err(Answer::stopped(ResultCode::InvalidInput));
        }
        // 4.5: a well-formed token that is not in this conflict (R-33).
        let tokens = conflict_read::path_tokens(&inspection);
        let mut resolutions = Vec::with_capacity(self.resolutions.len());
        for resolution in &self.resolutions {
            let Some((_, token)) = tokens.iter().find(|(text, _)| *text == resolution.token) else {
                return Err(Answer::stopped(ResultCode::ExternalChange));
            };
            resolutions.push((
                token.clone(),
                RedactedConflictBytes::from_bytes(resolution.source.as_bytes().to_vec()),
            ));
        }
        // Step 5: the association first, so that a retry of the request
        // finds its attempt however the call ends; then the supersede; then
        // the resolution.
        let attempt = attempt.unwrap_or_else(|| {
            attempt_for(
                resolving.accepted.request_id,
                synchronization,
                live.window_number,
                live.ordinal,
            )
        });
        // R-14: an ID that is already another row's is refused before the
        // request is recorded as its owner, as the library refuses it when
        // it records the attempt. Nothing is written.
        if !view.attempts.iter().any(|found| found.attempt == attempt) {
            state::with_transaction(service, root, |tx, _| state::validate_child_id(tx, attempt))
                .map_err(|error| failed(&error.into()))?;
        }
        resolving.own(attempt, digest)?;
        if let Some(other) = superseded {
            let refusal =
                match service.supersede_resolution_attempt(root, synchronization, other.attempt) {
                    Ok(true) => None,
                    Ok(false) => Some(original_request(&other)),
                    Err(error) => Some(failed(&error)),
                };
            if let Some(answer) = refusal {
                // The domain never recorded this request's attempt: the
                // same request may present another input.
                resolving.disown(attempt)?;
                return Err(answer);
            }
        }
        let before = resolving.mark(attempt);
        let outcome = service.resolve_synchronization(
            ResolveSynchronizationRequest::new(
                root.to_owned(),
                synchronization,
                attempt,
                inspection.observation,
                resolutions,
                None,
            )
            // Fact 71 again, under the conflict's reservation.
            .alone(),
        );
        self.reported(resolving, attempt, before, outcome)
    }

    /// Step 7: what the library's resolution of `attempt` came to, and the
    /// continuation once it is checkpointed. A refusal the domain made
    /// before it recorded the attempt lets go of the request's ownership of
    /// it.
    fn reported(
        &self,
        resolving: &Resolving<'_>,
        attempt: OperationId,
        before: AttemptMark,
        outcome: Result<ResolveSynchronizationOutcome, SynchronizationError>,
    ) -> Result<Ran, Answer> {
        let synchronization = resolving.synchronization;
        let refusal = |code| {
            resolving.disown(attempt)?;
            Err(Answer::stopped(code))
        };
        match outcome {
            Ok(ResolveSynchronizationOutcome::LocalCheckpointComplete { commit_oid }) => {
                Ok(self.continued(resolving, commit_oid, Checkpointed::Now))
            }
            Ok(ResolveSynchronizationOutcome::StaleObservation) => {
                refusal(ResultCode::ExternalChange)
            }
            Ok(ResolveSynchronizationOutcome::ValidationFailed) => {
                refusal(ResultCode::InvalidInput)
            }
            Ok(ResolveSynchronizationOutcome::IdentityRequired) => {
                refusal(ResultCode::IdentityRequired)
            }
            Ok(ResolveSynchronizationOutcome::RecoveryRequired) => Err(Answer {
                effects: resolving.effects_of(attempt, before),
                ..Answer::stopped(ResultCode::RecoveryRequired)
            }),
            // Before any write (fact 33).
            Err(SynchronizationError::ExternalResolutionRequired { .. }) => {
                resolving.disown(attempt)?;
                Err(external_repair(synchronization))
            }
            // An unfinished local-context sync among them (fact 80, R-38):
            // the attempt, and the request's ownership of it, are kept for
            // the same request.
            Err(error) => Err(Answer {
                effects: resolving.effects_of(attempt, before),
                ..failed(&error)
            }),
        }
    }

    /// Step 6: the resolution commit `commit` is checkpointed; the call's
    /// token is looked at, and the synchronization's publication is
    /// continued under this request (R-15).
    fn continued(&self, resolving: &Resolving<'_>, commit: git2::Oid, how: Checkpointed) -> Ran {
        let call = resolving.call;
        let synchronization = resolving.synchronization;
        // A cancellation seen before the continuation starts: the
        // synchronization is left resolved, for `operation resume`.
        if call.session.cancelled() {
            return Ran {
                answer: Answer {
                    effects: resolved(commit),
                    ..Answer::stopped(ResultCode::Cancelled)
                }
                .with_data(data(
                    synchronization,
                    Some(commit),
                    PublicationEffect::Pending,
                    None,
                )),
                standing: Standing::Final,
            };
        }
        // Published by another request since this one's earlier call
        // checkpointed it: nothing is written or pushed. A call that
        // entered the request again may itself have published it, and asks
        // the synchronization what it came to.
        if how == Checkpointed::Before && !self.entered_again {
            let journal = call.service.journal_row(
                OperationFamily::Remote,
                &resolving.accepted.scope,
                synchronization,
            );
            if let Ok(JournalRow::Final {
                kind: FinalKind::Completed,
                owes_work: false,
                checkpointed: _,
            }) = journal
            {
                return Ran {
                    answer: Answer {
                        effects: Effects {
                            publication: PublicationEffect::Current,
                            commit_oid: Some(commit.to_string()),
                            ..Effects::not_requested()
                        },
                        ..Answer::stopped(ResultCode::AlreadyApplied)
                    }
                    .with_data(data(
                        synchronization,
                        Some(commit),
                        PublicationEffect::Current,
                        None,
                    )),
                    standing: Standing::Final,
                };
            }
        }
        // Whether it publishes a comment, and which request holds it. A read
        // that fails stops the call with the read's answer, the resolution
        // checkpointed: nothing is continued on a guess.
        let found = call
            .service
            .read_session(RepositoryOperation::Read, |db| {
                comment_synchronization(db, resolving.root(), synchronization)
            })
            .map_err(|error| Answer::of_read(&error))
            .and_then(|bound| {
                let comment = bound.map(|(comment, _)| comment);
                Ok((comment, resolving.holder(comment)?))
            });
        let (comment, holder) = match found {
            Ok(found) => found,
            Err(answer) => {
                let ran = Ran {
                    answer,
                    standing: gone(),
                };
                return published(synchronization, commit, ran);
            }
        };
        let ran = match comment {
            Some(comment) => self.comment_continued(resolving, comment, holder),
            None => sync::continued(
                call,
                &resolving.accepted.taking_up(synchronization),
                resolving.target.clone(),
                Subject::of(self, resolving.accepted),
                Some(resolving.accepted.request_id),
                holder,
            ),
        };
        published(synchronization, commit, ran)
    }

    /// The continuation of a comment's synchronization: the comment's
    /// synchronizer-driven retry with `restart` (R-15), which proves the
    /// comment's checkpoint before it publishes, and never writes a
    /// comment. `holder` is the `comment add` request, whose flag stops it.
    /// What its synchronization's binding made of the call is the answer;
    /// with no call made, what the retry said.
    fn comment_continued(
        &self,
        resolving: &Resolving<'_>,
        comment: OperationId,
        holder: Option<RequestId>,
    ) -> Ran {
        let call = resolving.call;
        let accepted = resolving.accepted;
        let mut child = None;
        let retried = call.service.retry_comment_publication_with(
            RetryCommentPublicationRequest {
                root: resolving.root().to_owned(),
                operation_id: comment,
                approval: None,
                confirmed_identity: None,
                restart: true,
            },
            &mut |request| {
                let synchronization = accepted.taking_up(request.operation_id);
                let (result, ran) = sync::of_a_comment(
                    call,
                    &synchronization,
                    request,
                    Subject::of(self, accepted),
                    Some(accepted.request_id),
                    holder,
                );
                child = Some(ran);
                result
            },
        );
        if let Some(ran) = child {
            return ran;
        }
        let code = match retried {
            Ok(CommentSubmissionOutcome::Saved {
                publication:
                    CommentPublicationState::Pending {
                        reason: CommentPublicationPendingReason::Synchronization(error),
                    },
                ..
            }) => outcome::synchronization_error_code(&error),
            // The comment's own hand-off, or the proof of its checkpoint,
            // comes first; or nothing was published by a call this request
            // did not make.
            Ok(_) => ResultCode::RecoveryRequired,
            Err(error) => outcome::repository_error_code(error.kind),
        };
        Ran {
            answer: Answer::stopped(code),
            standing: gone(),
        }
    }
}

/// The phases of the resolution attempts of the synchronization
/// `synchronization` of the repository `repository_id`, read by
/// `state::resolution_attempt_phase`. A caller that acts only on an attempt
/// in some phase asks this first, and reads the resolution's view only when
/// one is there: `state::resolution_view` reads the synchronization's whole
/// row.
#[allow(clippy::result_large_err)] // `ReadError` carries its scope by value.
pub(super) fn attempt_phases(
    db: &rusqlite::Connection,
    repository_id: i64,
    synchronization: OperationId,
) -> Result<Vec<ResolutionAttemptPhase>, ReadError> {
    let mut statement = db.prepare(
        "SELECT attempt.phase
           FROM remote_resolution_attempts attempt
           JOIN remote_operation_records operation
             ON operation.id = attempt.operation_record_id
          WHERE operation.repository_id = ?1 AND operation.operation_ulid = ?2",
    )?;
    let stored = statement
        .query_map(
            rusqlite::params![repository_id, synchronization.to_string()],
            |row| row.get::<_, String>(0),
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(stored
        .iter()
        .map(|phase| state::resolution_attempt_phase(phase))
        .collect::<Result<Vec<_>, _>>()?)
}

/// Finishes, from durable state alone, the resolution attempt of the
/// synchronization `synchronization` whose commit is prepared, or applied
/// with its native artifacts held, so that `operation resume` takes the
/// synchronization up with the attempt finished (decision 18): a restart
/// refuses to touch such an attempt's artifacts itself. Nothing is done
/// when no attempt is in that state.
pub(crate) fn finish_prepared_resolution(
    service: &RepositoryService,
    root: &Path,
    synchronization: OperationId,
) -> Result<(), Answer> {
    let repository = resolve_root(service, root)?;
    let id = repository.registration_id();
    let view = service
        .read_session(RepositoryOperation::Read, |db| {
            if !attempt_phases(db, id, synchronization)?
                .iter()
                .any(|phase| {
                    matches!(
                        phase,
                        ResolutionAttemptPhase::CandidatePrepared | ResolutionAttemptPhase::Applied
                    )
                })
            {
                return Ok(None);
            }
            Ok(state::resolution_view(db, id, synchronization)?)
        })
        .map_err(|error| Answer::of_read(&error))?;
    let Some(attempt) = view.as_ref().and_then(ResolutionView::unfinished) else {
        return Ok(());
    };
    if !matches!(
        attempt.phase,
        ResolutionAttemptPhase::CandidatePrepared | ResolutionAttemptPhase::Applied
    ) {
        return Ok(());
    }
    match service.finish_resolution_attempt(repository.root(), synchronization, attempt.attempt) {
        Ok(Some(ResolveSynchronizationOutcome::LocalCheckpointComplete { .. })) => Ok(()),
        Ok(Some(_) | None) => Err(Answer::stopped(ResultCode::RecoveryRequired)),
        Err(error) => Err(failed(&error)),
    }
}

/// The answer of a request whose resolution `commit` is checkpointed, once
/// its continuation `ran`: the continuation's code, the resolution's
/// effects with what was published or merged, and the next step of a new
/// conflict the publication met.
fn published(synchronization: OperationId, commit: git2::Oid, ran: Ran) -> Ran {
    let Ran {
        answer,
        standing: _,
    } = ran;
    let mut effects = resolved(commit);
    if answer.effects.publication != PublicationEffect::NotRequested {
        effects.publication = answer.effects.publication;
    }
    if answer.effects.discovery != DiscoveryEffect::NotRequested {
        effects.discovery = answer.effects.discovery;
    }
    // A merge the publication began and could not finish.
    if answer.effects.integration == IntegrationEffect::Pending {
        effects.integration = IntegrationEffect::Pending;
    }
    let conflict = match answer.data {
        Some(MutationDataDto::SynchronizationConflict(data)) => Some(data.conflict),
        _ => None,
    };
    // The request did its work: a remote that held the commit already is
    // no reason to call it a no-op.
    let code = match answer.code {
        ResultCode::AlreadyApplied => ResultCode::Ok,
        code => code,
    };
    let (standing, recovery) = match code {
        ResultCode::Ok | ResultCode::Cancelled => (Standing::Final, Vec::new()),
        // The publication's hand-off is owed, which a resume does.
        ResultCode::DiscoveryPending => (
            Standing::Final,
            vec![RecoveryAction::for_operation(
                RecoveryActionKind::OperationResume,
                synchronization.to_string(),
                [],
            )],
        ),
        _ => (gone(), Vec::new()),
    };
    let publication = effects.publication;
    Ran {
        answer: Answer {
            effects,
            recovery,
            // What the continuation said of a change of its own to the
            // local branch; the data's commit is still the resolution's.
            local_change: answer.local_change,
            ..Answer::stopped(code)
        }
        .with_data(data(synchronization, Some(commit), publication, conflict)),
        standing,
    }
}

impl Binding for ConflictResolveBinding {
    fn command(&self) -> Command {
        Command::ConflictResolve
    }

    /// The request has no operation, so no row of its own is ever read.
    fn ended_without_checkpoint(&self) -> EndedWithoutCheckpoint {
        EndedWithoutCheckpoint::SaysNothing
    }

    fn place(&self) -> Place<'_> {
        Place::Repository(&self.root)
    }

    /// Looks up the synchronization the request names, by its own ID or a
    /// comment's (R-5), so that its identity is the same whichever ID
    /// names it. A repository that cannot be resolved has nothing to look
    /// up, and is refused when the request is prepared.
    fn settle_target(&mut self, service: &RepositoryService) -> Result<(), Answer> {
        let Ok(repository) = resolve_root(service, &self.root) else {
            return Ok(());
        };
        let operation_id = self.operation_id;
        let bound = service
            .read_session(RepositoryOperation::Read, |db| {
                comment_synchronization(db, repository.root(), operation_id)
            })
            .map_err(|error| Answer::of_read(&error))?;
        self.synchronization =
            Some(bound.map_or(operation_id, |(_, synchronization)| synchronization));
        Ok(())
    }

    /// The synchronization the request resolves, by its own ID even when
    /// the request names it by a comment's: the digest covers the resolved
    /// synchronization's ID (design, "Conflict Resolution"). The ID as
    /// named until it is settled.
    fn target(&self) -> String {
        self.synchronization
            .unwrap_or(self.operation_id)
            .to_string()
    }

    /// The observation, then every token and its source in token order;
    /// the target holds the synchronization the request resolves. A source
    /// is fed here and stored nowhere.
    fn fields(&self, builder: IntentDigestBuilder) -> IntentDigestBuilder {
        let Self {
            // What the digest is begun with: the scope and the target.
            root: _,
            operation_id: _,
            synchronization: _,
            observation,
            resolutions: _,
            // What a call found, and no part of the request.
            entered_again: _,
        } = self;
        let mut builder = builder.field("observation", FieldValue::Text(observation));
        for resolution in self.ordered() {
            builder = builder
                .field("token", FieldValue::Text(&resolution.token))
                .field("source", FieldValue::Bytes(resolution.source.as_bytes()));
        }
        builder
    }

    fn scope(&self, repository: Option<String>, _position: Option<&Position>) -> Scope {
        Scope {
            repository,
            ..Scope::default()
        }
    }

    /// Step 1, then what refuses the request before anything is recorded
    /// of it: a repository that is not registered, and an operation that
    /// is no remote synchronization of it. The request is placed on the
    /// branch the synchronization synchronizes.
    fn prepare(&mut self, call: &CallContext<'_>) -> Result<Prepared, Answer> {
        if !self.well_formed() {
            return Err(Answer::stopped(ResultCode::InvalidInput));
        }
        let repository = resolve_root(call.service, &self.root)?;
        let (_, target) = self.synchronization(call.service, &repository)?;
        Ok(Prepared {
            // The synchronization is another request's: this one has none.
            family: None,
            position: sync::resumed_position(call.service, repository.root(), target)?,
            expected: None,
            observation: None,
        })
    }

    fn run(&self, call: &CallContext<'_>, accepted: &Accepted) -> Ran {
        self.call(call, accepted)
    }

    /// No row is read for a request with no operation, and nothing is run
    /// or settled on one that is. Otherwise the request is taken up as a
    /// first call would take it up: its attempt is what says how far it
    /// got.
    fn reenter(
        &mut self,
        _call: &CallContext<'_>,
        _accepted: &Accepted,
        journal: JournalRow,
    ) -> Result<Entered, Ran> {
        if journal != JournalRow::Absent {
            return Err(Ran {
                answer: Answer::stopped(ResultCode::InternalError),
                standing: Standing::NotRun { journal },
            });
        }
        self.entered_again = true;
        Ok(Entered::Unfinished)
    }

    fn as_confirmed(&mut self) -> Option<&mut dyn Confirmed> {
        None
    }

    fn rerun(&self, call: &CallContext<'_>, accepted: &Accepted, _entered: &Entered) -> Ran {
        self.call(call, accepted)
    }
}
