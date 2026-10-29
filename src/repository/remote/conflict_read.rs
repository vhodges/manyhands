//! `conflict show`: the read of a synchronization's conflict, and the
//! string tokens a later process presents to resolve it (R-9).
//!
//! It is a read (R-12), and writes nothing. The registry is read in a
//! read-only session. Git is read through the library's inspection and
//! side reads, which take the repository's lease while they read it, so a
//! held lease answers `busy`; an outside repair is judged under the same
//! lease, by the rule a restart applies to it (fact 35). Nothing is
//! retired, recorded or reserved.
//!
//! It sits in `remote` because the library's observation and path tokens
//! keep their fields to this module (fact 31).

// `ReadError` carries its whole scope by value, as the result contract has it.
#![allow(clippy::result_large_err)]

use std::path::Path;

use rusqlite::{Connection, OptionalExtension};
use serde::{Serialize, Serializer};

use super::{
    RemoteOperationAction, RemoteOperationPhase, SynchronizationTarget,
    merge::{
        ConflictEligibility, ConflictObservation, ConflictPathToken, IntegrationStage,
        RedactedConflictBytes,
    },
    state::{
        self, IntegrationStepEvidence, IntegrationStepPhase, MergeMetadataEvidence,
        StoredRemoteOperation,
    },
    sync::{self, SynchronizationConflictInspection, SynchronizationError},
};
use crate::{
    repository::{
        AuthoringKind, ConfigurationInspection, OperationId, ReadError, RepositoryOperation,
        RepositoryService, ResolvedRepository,
        integration::{
            artifacts::MergeMetadataDirectory,
            validate::{self, ConflictEntryKind, ConflictStructure},
        },
        local_sync, mutation, read_configuration, repository_lease,
    },
    results::{RecoveryAction, RecoveryActionKind, ResultCode, contract_enum},
};

contract_enum!(
    /// Where a synchronization's conflict stands, as the conflict-states
    /// table of the design names it (R-13).
    ConflictState {
        ConflictPending => "conflict_pending",
        ResolutionPrepared => "resolution_prepared",
        Resolved => "resolved",
        Repaired => "repaired",
        RepairRefused => "repair_refused",
        Cancelled => "cancelled",
        Complete => "complete",
    }
);

contract_enum!(
    /// What a conflict needs next, as the read sees it. Wider than a
    /// mutation's `ConflictNextAction`, because a read also sees resolved,
    /// repaired, cancelled and published states. Where the two describe a
    /// live conflict they agree: `resolve` exactly when the set is not
    /// empty and every path is eligible canonical content.
    ConflictReadNextAction {
        Resolve => "resolve",
        RetryResolution => "retry_resolution",
        ExternalRepair => "external_repair",
        Resume => "resume",
        Synchronize => "synchronize",
        None => "none",
    }
);

contract_enum!(
    /// What a conflicted path holds (decision 12).
    ConflictPathKind {
        Document => "document",
        Ticket => "ticket",
        Comment => "comment",
        Noncanonical => "noncanonical",
    }
);

contract_enum!(
    /// The shape of a conflicted path's three sides (fact 30, R-40). Only
    /// `regular` canonical content can be resolved in Manyhands.
    ConflictPathStructure {
        Regular => "regular",
        Binary => "binary",
        Executable => "executable",
        Symlink => "symlink",
        Rename => "rename",
        Delete => "delete",
        IdentityChanged => "identity_changed",
        IncompatibleClosures => "incompatible_closures",
    }
);

contract_enum!(
    /// The branch a synchronization synchronizes: primary, or an item's
    /// editing branch.
    ConflictTarget {
        Primary => "primary",
        Document => "document",
        Ticket => "ticket",
    }
);

contract_enum!(
    /// The merge of a synchronization a conflict is in: the item's own
    /// remote branch into its editing branch, or primary into it.
    ConflictStage {
        Context => "context",
        Primary => "primary",
    }
);

contract_enum!(
    /// Whether a live conflict's set can be resolved in Manyhands. It is one
    /// value for the whole set (decision 12).
    ConflictReadEligibility {
        EligibleCanonical => "eligible_canonical",
        ExternalResolutionRequired => "external_resolution_required",
    }
);

/// A synchronization's conflict, as `conflict show` reports it. A
/// requested read: its sides are content, which no diagnostic holds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ConflictDto {
    /// The synchronization's ID, whichever ID was asked for (R-5).
    pub operation_id: String,
    /// The comment's operation, when the synchronization publishes a
    /// comment.
    pub comment_operation_id: Option<String>,
    pub target: ConflictTarget,
    pub item_id: Option<String>,
    pub stage: ConflictStage,
    pub state: ConflictState,
    pub next_action: ConflictReadNextAction,
    /// The request whose resolution attempt is unfinished. Null until the
    /// attempt's request is recorded (Task 4a).
    pub request_id: Option<String>,
    /// The set's eligibility, while the conflict is live.
    pub eligibility: Option<ConflictReadEligibility>,
    pub local_parent: String,
    pub incoming_parent: String,
    /// `conflict-v1:` and a digest of the live conflict's observation, or
    /// null when no conflict is live (R-9).
    pub observation: Option<String>,
    /// True when a resolution attempt has written paths: the sides are the
    /// recorded conflict's, and the worktree may already hold results
    /// (R-39).
    pub historical: bool,
    /// The live conflict's paths, ordered by raw path bytes.
    pub paths: Vec<ConflictPathDto>,
}

/// One path of a live conflict.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ConflictPathDto {
    /// The path, escaped by R-11's rule. It is never a writable path.
    pub display_path: String,
    /// `conflict-path-v1:` and a digest that binds the path to the
    /// observation (R-9).
    pub token: String,
    pub kind: ConflictPathKind,
    pub structure: ConflictPathStructure,
    /// Whether the library read the sides as text (fact 32, R-10).
    pub content_available: bool,
    pub base: Option<String>,
    pub local: Option<String>,
    pub incoming: Option<String>,
}

impl RepositoryService {
    /// The conflict of the synchronization `operation_id` names: the
    /// synchronization's own ID, or the operation of a comment it
    /// publishes (R-5).
    ///
    /// A local-context sync's ID is `invalid_input` (decision 19), as is an
    /// operation that is no synchronization, or a synchronization that met
    /// no conflict. An ID no operation of the repository has is
    /// `operation_not_found`.
    pub fn show_conflict(
        &self,
        repo: &ResolvedRepository,
        operation_id: OperationId,
    ) -> Result<ConflictDto, ReadError> {
        conflict(self, repo, operation_id).map_err(|error| placed(repo, error))
    }
}

/// The observation's string form (R-9): `conflict-v1:` and a BLAKE3 digest
/// of every field of the observation, each domain-separated and
/// length-delimited.
pub(crate) fn observation_text(observation: &ConflictObservation) -> String {
    format!("conflict-v1:{}", observation_digest(observation).to_hex())
}

/// Each path of the inspection, in its order, with its string token
/// (R-9): `conflict-path-v1:` and a BLAKE3 digest of the observation's
/// digest, the path's position in `paths`, the path's bytes, and the three
/// sides' object IDs and modes. Re-derived from the live conflict on every
/// call, and never stored.
pub(crate) fn path_tokens(
    inspection: &SynchronizationConflictInspection,
) -> Vec<(String, ConflictPathToken)> {
    let observation = observation_digest(&inspection.observation);
    inspection
        .paths
        .iter()
        .enumerate()
        .map(|(position, path)| {
            (
                path_token_text(observation.as_bytes(), position, &path.token),
                path.token.clone(),
            )
        })
        .collect()
}

/// A path for display (R-11): each byte outside printable ASCII as `\xNN`,
/// and a backslash doubled. The gate's `display_path` applies the same
/// rule, in a file C4 does not edit.
pub(crate) fn display_path(path: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut display = String::new();
    for &byte in path {
        match byte {
            b'\\' => display.push_str("\\\\"),
            b' '..=b'~' => display.push(char::from(byte)),
            _ => {
                let _ = write!(display, "\\x{byte:02x}");
            }
        }
    }
    display
}

/// One field of a token's digest: its length, then its bytes.
fn field(hash: &mut blake3::Hasher, bytes: &[u8]) {
    hash.update(&(bytes.len() as u64).to_be_bytes());
    hash.update(bytes);
}

fn observation_digest(observation: &ConflictObservation) -> blake3::Hash {
    let mut hash = blake3::Hasher::new();
    hash.update(b"manyhands-conflict-observation-v1");
    field(&mut hash, observation.operation_id.to_string().as_bytes());
    field(&mut hash, &observation.window_number.to_be_bytes());
    field(&mut hash, &[observation.ordinal]);
    field(&mut hash, &observation.fingerprint);
    field(&mut hash, observation.head.as_bytes());
    field(&mut hash, &observation.configuration);
    field(&mut hash, observation.root.as_os_str().as_encoded_bytes());
    hash.finalize()
}

fn path_token_text(observation: &[u8; 32], position: usize, token: &ConflictPathToken) -> String {
    let mut hash = blake3::Hasher::new();
    hash.update(b"manyhands-conflict-path-v1");
    field(&mut hash, observation);
    field(&mut hash, &(position as u64).to_be_bytes());
    field(&mut hash, &token.path);
    for (oid, mode) in [
        (token.base, token.base_mode),
        (token.local, token.local_mode),
        (token.incoming, token.incoming_mode),
    ] {
        field(&mut hash, oid.as_ref().map_or(&[][..], git2::Oid::as_bytes));
        field(
            &mut hash,
            &mode.map_or_else(Vec::new, |mode| mode.to_be_bytes().to_vec()),
        );
    }
    format!("conflict-path-v1:{}", hash.finalize().to_hex())
}

/// What the registry says of the ID a read was asked for.
enum Lookup {
    /// One of the gate's local-context syncs (decision 19).
    LocalSync,
    /// No remote operation of the repository.
    Missing,
    /// A remote operation that is no synchronization.
    NotSynchronization,
    /// A synchronization that recorded no conflict.
    NoConflict,
    Found(Box<Recorded>),
}

/// A synchronization that met a conflict, as the registry records it.
struct Recorded {
    operation_id: OperationId,
    comment_operation_id: Option<OperationId>,
    phase: RemoteOperationPhase,
    target: SynchronizationTarget,
    /// The step of the newest pass whose conflict is not settled, if any.
    pending: Option<IntegrationStepPhase>,
    /// The step whose conflict this is: the pending one, or else the
    /// newest that met a conflict.
    step: IntegrationStepEvidence,
    /// Whether a resolution attempt of the library's is recorded for it.
    resolution_attempted: bool,
    /// The merge metadata its merge left, as it recorded it.
    merge_metadata: Option<MergeMetadataEvidence>,
}

fn conflict(
    service: &RepositoryService,
    repo: &ResolvedRepository,
    operation_id: OperationId,
) -> Result<ConflictDto, ReadError> {
    let found = service.read_session(RepositoryOperation::Read, |db| {
        lookup(db, repo, operation_id)
    })?;
    let recorded = match found {
        Lookup::Found(recorded) => recorded,
        Lookup::Missing => {
            // Any other operation of the repository is no synchronization.
            return Err(match service.show_operation(repo, operation_id) {
                Ok(_) => ReadError::new(ResultCode::InvalidInput),
                Err(error) => error,
            });
        }
        Lookup::LocalSync | Lookup::NotSynchronization | Lookup::NoConflict => {
            return Err(ReadError::new(ResultCode::InvalidInput));
        }
    };
    let answer = |state, next_action| settled(&recorded, state, next_action);
    match recorded.pending {
        Some(IntegrationStepPhase::ConflictPending) => {
            match service.inspect_synchronization_recovery(repo.root(), recorded.operation_id) {
                Ok(inspection) => live(service, repo, &recorded, &inspection, false),
                // A held lease or an unreadable registry is no answer about
                // the conflict.
                Err(error @ (SynchronizationError::Repository(_) | SynchronizationError::Busy)) => {
                    Err(read_error(error))
                }
                // Git no longer holds the recorded conflict: it was repaired
                // outside, or it was changed some other way.
                Err(_) => Ok(if repair_accepted(repo.root(), &recorded)? {
                    answer(ConflictState::Repaired, ConflictReadNextAction::Resume)
                } else {
                    answer(
                        ConflictState::RepairRefused,
                        ConflictReadNextAction::ExternalRepair,
                    )
                }),
            }
        }
        // An attempt has written paths, and the index's stages are intact
        // (fact 86): the conflict is still live.
        Some(IntegrationStepPhase::ResolutionPrepared) => {
            let inspection = service
                .inspect_synchronization_recovery(repo.root(), recorded.operation_id)
                .map_err(read_error)?;
            live(service, repo, &recorded, &inspection, true)
        }
        // The resolution commit is prepared: `operation resume` finishes it
        // from durable state (decision 18).
        Some(_) => Ok(answer(
            ConflictState::ResolutionPrepared,
            ConflictReadNextAction::Resume,
        )),
        None => match recorded.phase {
            RemoteOperationPhase::Completed => Ok(answer(
                ConflictState::Complete,
                ConflictReadNextAction::None,
            )),
            // Final: a new synchronization publishes what it left (fact 56).
            RemoteOperationPhase::Cancelled => Ok(answer(
                ConflictState::Cancelled,
                ConflictReadNextAction::Synchronize,
            )),
            _ if recorded.step.phase == IntegrationStepPhase::Applied => Ok(answer(
                if recorded.resolution_attempted {
                    ConflictState::Resolved
                } else {
                    ConflictState::Repaired
                },
                ConflictReadNextAction::Resume,
            )),
            _ => Err(ReadError::new(ResultCode::RecoveryRequired)),
        },
    }
}

/// Looks `operation_id` up: among the gate's local syncs first, so that one
/// is never `operation_not_found` (R-37); then as a comment's operation,
/// for its synchronization (R-5); then among the repository's remote
/// operations.
fn lookup(
    db: &Connection,
    repo: &ResolvedRepository,
    operation_id: OperationId,
) -> Result<Lookup, ReadError> {
    if local_sync::record::owned_identity(db, &operation_id.to_string())? {
        return Ok(Lookup::LocalSync);
    }
    let (comment_operation_id, synchronization) =
        match mutation::comment_synchronization(db, repo.root(), operation_id) {
            Some((comment, synchronization)) => (Some(comment), synchronization),
            None => (None, operation_id),
        };
    let Some(record) = state::read_operation(db, repo.registration_id(), synchronization)? else {
        return Ok(Lookup::Missing);
    };
    let target = match record.target.action() {
        RemoteOperationAction::SynchronizePrimary => SynchronizationTarget::Primary,
        RemoteOperationAction::SynchronizeContext => {
            let (kind, item_id) = record.target.item().ok_or_else(state::recovery_required)?;
            SynchronizationTarget::Context {
                kind,
                item_id: item_id.clone(),
            }
        }
        _ => return Ok(Lookup::NotSynchronization),
    };
    // As the inspection requires: at most one step of the newest pass
    // holds an unsettled conflict.
    let window = state::latest_integration_window(db, &record)?;
    let mut pending = None;
    for ordinal in 0..=1 {
        let Some(step) = state::integration_step_in_window(db, record.id, window.number, ordinal)?
        else {
            continue;
        };
        if matches!(
            step.phase,
            IntegrationStepPhase::ConflictPending
                | IntegrationStepPhase::ResolutionPrepared
                | IntegrationStepPhase::CommitPrepared
        ) {
            if pending.is_some() {
                return Err(state::recovery_required().into());
            }
            pending = Some(step);
        }
    }
    let pending_phase = pending.as_ref().map(|step| step.phase);
    let step = match pending {
        Some(step) => step,
        None => match last_conflict(db, &record)? {
            Some(step) => step,
            None => return Ok(Lookup::NoConflict),
        },
    };
    let resolution_attempted = state::integration_has_resolution_attempt(db, &record, &step)?;
    let merge_metadata =
        state::integration_merge_metadata(db, &record, step.window_number, step.intent.ordinal)?;
    Ok(Lookup::Found(Box::new(Recorded {
        operation_id: synchronization,
        comment_operation_id,
        phase: record.phase,
        target,
        pending: pending_phase,
        step,
        resolution_attempted,
        merge_metadata,
    })))
}

/// The newest step of the operation's journal that met a conflict. A
/// conflict's digest stays on its step once it is resolved or repaired.
fn last_conflict(
    db: &Connection,
    record: &StoredRemoteOperation,
) -> Result<Option<IntegrationStepEvidence>, ReadError> {
    let found: Option<(i64, i64)> = db
        .query_row(
            "SELECT window_number, ordinal FROM remote_integration_steps
              WHERE operation_record_id = ?1 AND conflict_digest IS NOT NULL
              ORDER BY window_number DESC, ordinal DESC LIMIT 1",
            [record.id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let Some((window, ordinal)) = found else {
        return Ok(None);
    };
    let (Ok(window), Ok(ordinal)) = (u32::try_from(window), u8::try_from(ordinal)) else {
        return Err(state::recovery_required().into());
    };
    Ok(state::integration_step_in_window(
        db, record.id, window, ordinal,
    )?)
}

/// The read of a conflict no live inspection is made of: no observation,
/// no eligibility and no path.
fn settled(
    recorded: &Recorded,
    state: ConflictState,
    next_action: ConflictReadNextAction,
) -> ConflictDto {
    let (target, item_id) = match &recorded.target {
        SynchronizationTarget::Primary => (ConflictTarget::Primary, None),
        SynchronizationTarget::Context { kind, item_id } => (
            match kind {
                AuthoringKind::Document => ConflictTarget::Document,
                AuthoringKind::Ticket => ConflictTarget::Ticket,
            },
            Some(item_id.to_string()),
        ),
    };
    ConflictDto {
        operation_id: recorded.operation_id.to_string(),
        comment_operation_id: recorded.comment_operation_id.map(|id| id.to_string()),
        target,
        item_id,
        stage: match recorded.step.intent.stage {
            IntegrationStage::Context => ConflictStage::Context,
            IntegrationStage::Primary => ConflictStage::Primary,
        },
        state,
        next_action,
        request_id: None,
        eligibility: None,
        local_parent: recorded.step.intent.local_oid.to_string(),
        incoming_parent: recorded.step.intent.incoming_oid.to_string(),
        observation: None,
        historical: false,
        paths: Vec::new(),
    }
}

/// The read of a live conflict: the inspection's observation and tokens,
/// each path's kind and structure, and its sides where the library reads
/// them as text. `prepared` says a resolution attempt has written paths.
fn live(
    service: &RepositoryService,
    repo: &ResolvedRepository,
    recorded: &Recorded,
    inspection: &SynchronizationConflictInspection,
    prepared: bool,
) -> Result<ConflictDto, ReadError> {
    // Blobs are immutable: the index entries the inspection issued its
    // tokens for are classified from the object database.
    let git = git2::Repository::open_ext(
        repo.root(),
        git2::RepositoryOpenFlags::NO_SEARCH,
        &[] as &[&std::ffi::OsStr],
    )
    .map_err(|_| ReadError::new(ResultCode::RepositoryInaccessible))?;
    let mut paths = Vec::with_capacity(inspection.paths.len());
    for (token_text, token) in path_tokens(inspection) {
        let candidate = validate::classify_conflict_entry(&git, &index_conflict(&token));
        let sides = match service.read_synchronization_conflict(&token) {
            Ok(sides) => text_sides([sides.base, sides.local, sides.incoming]),
            // Not text the library reads: binary, not UTF-8, or not a
            // regular file at a plain path.
            Err(SynchronizationError::ExternalResolutionRequired { .. }) => None,
            Err(error) => return Err(read_error(error)),
        };
        let content_available = sides.is_some();
        let [base, local, incoming] = sides.unwrap_or_default();
        paths.push((
            token.path.clone(),
            ConflictPathDto {
                display_path: display_path(&token.path),
                token: token_text,
                kind: path_kind(candidate.kind),
                structure: path_structure(candidate.structure),
                content_available,
                base,
                local,
                incoming,
            },
        ));
    }
    paths.sort_by(|left, right| left.0.cmp(&right.0));
    // `conflict_next`'s rule, so the read and a mutation agree.
    let eligible = !inspection.paths.is_empty()
        && inspection
            .paths
            .iter()
            .all(|path| path.eligibility == ConflictEligibility::EligibleCanonical);
    let (state, next_action) = if prepared {
        (
            ConflictState::ResolutionPrepared,
            ConflictReadNextAction::RetryResolution,
        )
    } else if eligible {
        (
            ConflictState::ConflictPending,
            ConflictReadNextAction::Resolve,
        )
    } else {
        (
            ConflictState::ConflictPending,
            ConflictReadNextAction::ExternalRepair,
        )
    };
    Ok(ConflictDto {
        eligibility: Some(if eligible {
            ConflictReadEligibility::EligibleCanonical
        } else {
            ConflictReadEligibility::ExternalResolutionRequired
        }),
        observation: Some(observation_text(&inspection.observation)),
        historical: prepared && !paths.is_empty(),
        paths: paths.into_iter().map(|(_, path)| path).collect(),
        ..settled(recorded, state, next_action)
    })
}

/// The three sides the library read, as text. The library returns only
/// UTF-8 sides (fact 32); anything else is no content.
fn text_sides(sides: [Option<RedactedConflictBytes>; 3]) -> Option<[Option<String>; 3]> {
    let mut text: [Option<String>; 3] = Default::default();
    for (slot, side) in text.iter_mut().zip(sides) {
        if let Some(side) = side {
            *slot = Some(String::from_utf8(side.bytes().to_vec()).ok()?);
        }
    }
    Some(text)
}

/// The index conflict the token was issued for. Every stage of one index
/// conflict has the token's path; classification reads only the path,
/// each stage's mode and its blob.
fn index_conflict(token: &ConflictPathToken) -> git2::IndexConflict {
    let entry = |id: Option<git2::Oid>, mode: Option<u32>| {
        id.zip(mode).map(|(id, mode)| git2::IndexEntry {
            ctime: git2::IndexTime::new(0, 0),
            mtime: git2::IndexTime::new(0, 0),
            dev: 0,
            ino: 0,
            mode,
            uid: 0,
            gid: 0,
            file_size: 0,
            id,
            flags: 0,
            flags_extended: 0,
            path: token.path.clone(),
        })
    };
    git2::IndexConflict {
        ancestor: entry(token.base, token.base_mode),
        our: entry(token.local, token.local_mode),
        their: entry(token.incoming, token.incoming_mode),
    }
}

fn path_kind(kind: ConflictEntryKind) -> ConflictPathKind {
    match kind {
        ConflictEntryKind::Document => ConflictPathKind::Document,
        ConflictEntryKind::Ticket => ConflictPathKind::Ticket,
        ConflictEntryKind::Comment => ConflictPathKind::Comment,
        ConflictEntryKind::Noncanonical => ConflictPathKind::Noncanonical,
    }
}

fn path_structure(structure: ConflictStructure) -> ConflictPathStructure {
    match structure {
        ConflictStructure::RegularUtf8SamePath => ConflictPathStructure::Regular,
        ConflictStructure::Binary => ConflictPathStructure::Binary,
        ConflictStructure::Executable => ConflictPathStructure::Executable,
        ConflictStructure::Symlink => ConflictPathStructure::Symlink,
        ConflictStructure::Rename => ConflictPathStructure::Rename,
        ConflictStructure::Delete => ConflictPathStructure::Delete,
        ConflictStructure::IdentityChanged => ConflictPathStructure::IdentityChanged,
        ConflictStructure::IncompatibleClosures => ConflictPathStructure::IncompatibleClosures,
    }
}

const MERGE_MEMBERS: [&str; 3] = ["MERGE_HEAD", "MERGE_MSG", "MERGE_MODE"];

/// State of another Git command that a clean target holds none of, as
/// `require_clean_target_with` lists it beside the merge members.
const OTHER_STATE: [&str; 4] = [
    "REBASE_HEAD",
    "CHERRY_PICK_HEAD",
    "rebase-apply",
    "rebase-merge",
];

/// State that makes merge metadata left in the target foreign, as
/// `foreign_resolution_metadata_present` lists it.
const FOREIGN_METADATA: [&str; 7] = [
    "REBASE_HEAD",
    "CHERRY_PICK_HEAD",
    "REVERT_HEAD",
    "BISECT_LOG",
    "rebase-merge",
    "rebase-apply",
    "sequencer",
];

/// Whether the target holds an outside repair a restart would take up
/// (fact 35): HEAD is a commit with exactly the parents `[local,
/// incoming]`; the target is clean but for this operation's own recorded
/// merge metadata; and `validates_external_integration` accepts it. Read
/// under the repository's lease. Nothing is written, retired or recorded.
fn repair_accepted(root: &Path, recorded: &Recorded) -> Result<bool, ReadError> {
    let ConfigurationInspection::Valid(config) = read_configuration(root)? else {
        return Ok(false);
    };
    let Ok(repository) = sync::materialized_target(root, &config.primary_branch, &recorded.target)
    else {
        return Ok(false);
    };
    let _lease = repository_lease(&repository, root, RepositoryOperation::RepositorySnapshot)?;
    Ok(repaired(&repository, recorded).unwrap_or(false))
}

fn repaired(repository: &git2::Repository, recorded: &Recorded) -> Option<bool> {
    let (local, incoming) = (
        recorded.step.intent.local_oid,
        recorded.step.intent.incoming_oid,
    );
    let head = repository.head().ok()?.peel_to_commit().ok()?;
    if head.parent_count() != 2
        || [head.parent_id(0).ok(), head.parent_id(1).ok()] != [Some(local), Some(incoming)]
        || !clean(repository, &head, recorded.merge_metadata.as_ref())?
    {
        return Some(false);
    }
    validate::validates_external_integration(repository, local, incoming, head.id()).ok()
}

fn present(path: &Path) -> bool {
    !matches!(
        std::fs::symlink_metadata(path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound
    )
}

/// The cleanliness a restart requires of a repaired target: no state of
/// another Git command, no conflict, no change or untracked file, and an
/// index that is HEAD's tree. Merge metadata may remain in the target's
/// own Git directory only when every member is what the operation's own
/// merge recorded and its retirement has not been recorded.
fn clean(
    repository: &git2::Repository,
    head: &git2::Commit<'_>,
    recorded: Option<&MergeMetadataEvidence>,
) -> Option<bool> {
    let gitdir = repository.path();
    let common = repository.commondir();
    let own = MERGE_MEMBERS
        .iter()
        .any(|member| present(&gitdir.join(member)));
    if own {
        let Some(recorded) = recorded else {
            return Some(false);
        };
        if recorded.phase == "retired"
            || FOREIGN_METADATA
                .iter()
                .any(|name| present(&gitdir.join(name)) || present(&common.join(name)))
        {
            return Some(false);
        }
        let directory = MergeMetadataDirectory::open(repository).ok()?;
        for (member, expected) in MERGE_MEMBERS.iter().zip(recorded.digests) {
            if let Some(actual) = directory.digest(member).ok()?
                && Some(actual) != expected
            {
                return Some(false);
            }
        }
    }
    let foreign = MERGE_MEMBERS.iter().chain(&OTHER_STATE).any(|name| {
        [gitdir, common].into_iter().any(|root| {
            !(own && root == gitdir && MERGE_MEMBERS.contains(name)) && present(&root.join(name))
        })
    });
    let state = repository.state();
    let index = repository.index().ok()?;
    if foreign
        || !(state == git2::RepositoryState::Clean
            || (own && state == git2::RepositoryState::Merge))
        || index.has_conflicts()
    {
        return Some(false);
    }
    let mut options = git2::StatusOptions::new();
    options.include_untracked(true).recurse_untracked_dirs(true);
    if !repository.statuses(Some(&mut options)).ok()?.is_empty() {
        return Some(false);
    }
    let tree = head.tree().ok()?;
    // What `require_clean_target_with` proves by writing the index's tree,
    // read without a write.
    let staged = repository
        .diff_tree_to_index(Some(&tree), Some(&index), None)
        .ok()?;
    Some(staged.deltas().next().is_none())
}

/// What a failure of the library's conflict reads means to a read. None of
/// these reaches the transport.
fn read_error(error: SynchronizationError) -> ReadError {
    match error {
        SynchronizationError::Repository(error) => error.into(),
        SynchronizationError::Busy => ReadError::new(ResultCode::Busy),
        SynchronizationError::ExternalChange
        | SynchronizationError::TargetNotMaterialized
        | SynchronizationError::PrimaryMissing
        | SynchronizationError::WorktreeNotClean { .. }
        | SynchronizationError::WorktreeConflicted { .. } => {
            ReadError::new(ResultCode::ExternalChange)
        }
        SynchronizationError::RecoveryRequired
        | SynchronizationError::ConflictPending { .. }
        | SynchronizationError::ExternalResolutionRequired { .. } => {
            ReadError::new(ResultCode::RecoveryRequired)
        }
        SynchronizationError::Transport(_)
        | SynchronizationError::PollYielding
        | SynchronizationError::Interrupted
        | SynchronizationError::RemoteContextDeleted
        | SynchronizationError::HistoryUnknown
        | SynchronizationError::IdentityRequired { .. }
        | SynchronizationError::MergeRequired { .. }
        | SynchronizationError::PushRejected => ReadError::new(ResultCode::InternalError),
    }
}

/// Places a failure about the repository, as every read of a resolved
/// repository does: the scope names it, and a rebuild is told its root.
fn placed(repo: &ResolvedRepository, mut error: ReadError) -> ReadError {
    let root = repo.scope().repository;
    if error.code() == ResultCode::IndexUnavailable {
        error.recovery = vec![RecoveryAction::new(
            RecoveryActionKind::IndexRebuild,
            root.clone().map(|root| ("root", root.into())),
        )];
    }
    if error.scope.repository.is_none() {
        error.scope.repository = root;
    }
    error
}
