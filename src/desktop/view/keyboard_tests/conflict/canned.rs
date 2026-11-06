//! The canned remote answers of the keyboard conflict journeys (decision
//! 9): each built from library types, in the shape the library test its
//! doc names produces. The SSH harness does not reach desktop tests.

use manyhands::{
    repository::{
        ConflictDto, ConflictNextAction, ConflictNextDto, ConflictPathDto, ConflictPathKind,
        ConflictPathStructure, ConflictReadEligibility, ConflictReadNextAction,
        ConflictResolutionMutationDto, ConflictResolutionMutationType, ConflictStage,
        ConflictState, ConflictTarget, MutationDataDto, SynchronizationConflictMutationDto,
        SynchronizationConflictType,
    },
    results::{
        CheckpointEffect, DiscoveryEffect, Effects, Envelope, IntegrationEffect, PublicationEffect,
        ResultCode, Scope, WriteEffect,
    },
};

use super::{SYNCHRONIZATION, TICKET};
use crate::desktop::test_support::fixtures::{ticket_path, ticket_source};

/// The resolution commit the canned answer names.
const COMMIT: &str = "3333333333333333333333333333333333333333";

/// A share stopped on a conflict of `SYNCHRONIZATION`, as
/// `tests/mutation_remote/merging.rs` `c4_merging_canonical_conflict`
/// answers (`assert_conflict`): `recovery_required`, a merge begun and
/// nothing published (`merge_pending`), and the conflict of the result's
/// own operation, to resolve.
pub(super) fn share_conflict() -> Envelope<MutationDataDto> {
    Envelope::classified_mutation(
        "item sync",
        Scope::default(),
        false,
        ResultCode::RecoveryRequired,
        Effects {
            publication: PublicationEffect::Pending,
            integration: IntegrationEffect::Pending,
            ..Effects::not_requested()
        },
        false,
    )
    .with_operation_id(SYNCHRONIZATION)
    .with_data(MutationDataDto::SynchronizationConflict(
        SynchronizationConflictMutationDto {
            r#type: SynchronizationConflictType::SynchronizationConflict,
            conflict: ConflictNextDto {
                operation_id: SYNCHRONIZATION.to_owned(),
                next_action: ConflictNextAction::Resolve,
            },
        },
    ))
}

/// The ticket's source titled `title`, with the body every side shares.
fn side(title: &str) -> String {
    ticket_source(TICKET, title, "").replace(&format!("Body of {title}."), "Body.")
}

/// The share's conflict as read with `paths`, in `state`, with
/// `next_action` and `eligibility`.
fn read(
    state: ConflictState,
    next_action: ConflictReadNextAction,
    eligibility: Option<ConflictReadEligibility>,
    paths: Vec<ConflictPathDto>,
) -> ConflictDto {
    let live = !paths.is_empty();
    ConflictDto {
        operation_id: SYNCHRONIZATION.to_owned(),
        comment_operation_id: None,
        target: ConflictTarget::Primary,
        item_id: None,
        stage: ConflictStage::Primary,
        state,
        next_action,
        request_id: None,
        eligibility,
        local_parent: "1".repeat(40),
        incoming_parent: "2".repeat(40),
        observation: live.then(|| format!("conflict-v1:{}", "0".repeat(64))),
        historical: false,
        paths,
    }
}

/// The ticket's path, its title changed on both sides.
fn ticket() -> ConflictPathDto {
    ConflictPathDto {
        display_path: ticket_path(TICKET),
        token: format!("conflict-path-v1:{}", "1".repeat(64)),
        kind: ConflictPathKind::Ticket,
        structure: ConflictPathStructure::Regular,
        content_available: true,
        base: Some(side("Base")),
        local: Some(side("Local")),
        incoming: Some(side("Incoming")),
    }
}

/// The share's conflict, live and to be resolved, whose one path is the
/// ticket: the shape `tests/mutation_remote/conflict_read.rs`
/// `c4_conflict_read_canonical` reads.
pub(super) fn conflict_read() -> ConflictDto {
    read(
        ConflictState::ConflictPending,
        ConflictReadNextAction::Resolve,
        Some(ConflictReadEligibility::EligibleCanonical),
        vec![ticket()],
    )
}

/// The share's conflict, live, beside a binary file no item holds, so the
/// whole set is repaired outside: the shape
/// `tests/mutation_remote/conflict_read.rs`
/// `c4_conflict_read_noncanonical_or_structural` reads.
pub(super) fn external_read() -> ConflictDto {
    let binary = ConflictPathDto {
        display_path: "blob.bin".to_owned(),
        token: format!("conflict-path-v1:{}", "2".repeat(64)),
        kind: ConflictPathKind::Noncanonical,
        structure: ConflictPathStructure::Binary,
        content_available: false,
        base: None,
        local: None,
        incoming: None,
    };
    read(
        ConflictState::ConflictPending,
        ConflictReadNextAction::ExternalRepair,
        Some(ConflictReadEligibility::ExternalResolutionRequired),
        vec![ticket(), binary],
    )
}

/// The share's conflict once repaired outside, to resume, no conflict
/// live: as `tests/mutation_remote/conflict_read.rs`
/// `c4_conflict_read_repaired` reads it (`settled`).
pub(super) fn repaired_read() -> ConflictDto {
    read(
        ConflictState::Repaired,
        ConflictReadNextAction::Resume,
        None,
        Vec::new(),
    )
}

/// The resolution applied and published, as
/// `tests/mutation_remote/resolution.rs` `c4_resolution_publishes`
/// answers (`assert_published`): `published(commit)` effects and
/// `data_of(operation, commit, Published, None)`.
pub(super) fn applied() -> Envelope<MutationDataDto> {
    Envelope::classified_mutation(
        "conflict resolve",
        Scope::default(),
        false,
        ResultCode::Ok,
        Effects {
            write: WriteEffect::Written,
            checkpoint: CheckpointEffect::Committed,
            integration: IntegrationEffect::Complete,
            publication: PublicationEffect::Published,
            discovery: DiscoveryEffect::Current,
            commit_oid: Some(COMMIT.to_owned()),
            ..Effects::not_requested()
        },
        false,
    )
    .with_data(MutationDataDto::ConflictResolution(
        ConflictResolutionMutationDto {
            r#type: ConflictResolutionMutationType::ConflictResolution,
            operation_id: SYNCHRONIZATION.to_owned(),
            commit_oid: Some(COMMIT.to_owned()),
            publication: PublicationEffect::Published,
            conflict: None,
        },
    ))
}
