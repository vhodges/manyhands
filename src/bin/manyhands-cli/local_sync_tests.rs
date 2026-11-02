use super::*;
use manyhands::{
    repository::{
        LocalContextSyncDataType, LocalContextSyncEligibility, LocalContextSyncReason,
        LocalContextSyncState,
    },
    results::{Effects, Scope},
};

const ROOT: &str = "/tmp/it's";
const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
const OPERATION: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX1";

fn stopped(code: ResultCode, next_action: LocalContextSyncNextAction) -> Envelope<MutationDataDto> {
    let mut envelope = Envelope::classified_mutation(
        "context sync-local",
        Scope::default(),
        false,
        code,
        Effects::not_requested(),
        false,
    );
    envelope.data = Some(MutationDataDto::LocalContextSync(
        LocalContextSyncMutationDto {
            data_type: LocalContextSyncDataType::LocalContextSync,
            item_id: ITEM.into(),
            kind: "ticket".into(),
            operation_id: Some(OPERATION.into()),
            resolution_request_id: None,
            required_request_id: None,
            result_oid: None,
            state: LocalContextSyncState::ConflictPending,
            reason: LocalContextSyncReason::CanonicalConflict,
            next_action,
            eligibility: Some(LocalContextSyncEligibility::EligibleCanonical),
            cancel_acknowledged: false,
        },
    ));
    envelope
}

/// `human_lifecycle::merge_refused`'s last two strings, past the program's name.
#[test]
fn a_conflict_names_the_same_commands_a_refused_merge_prints() {
    let lines = way_on(
        &stopped(
            ResultCode::RecoveryRequired,
            LocalContextSyncNextAction::Resolve,
        ),
        ROOT,
        ITEM,
    );
    let root = quoted(ROOT);
    assert!(lines.contains(&format!(
        "next: manyhands-cli context show-local --repo {root} --operation-id {OPERATION}\n"
    )));
    assert!(lines.contains(&format!(
        "next: manyhands-cli context resolve-local --repo {root} --operation-id {OPERATION} --input - --request-id <new-request-id>\n"
    )));
}

#[test]
fn only_a_resolvable_conflict_offers_resolve_local() {
    for action in [
        LocalContextSyncNextAction::ExternalRepair,
        LocalContextSyncNextAction::None,
        LocalContextSyncNextAction::Resume,
        LocalContextSyncNextAction::SetIdentity,
        LocalContextSyncNextAction::RetryResolution,
    ] {
        let lines = way_on(&stopped(ResultCode::RecoveryRequired, action), ROOT, ITEM);
        assert!(!lines.contains("resolve-local"), "{action:?}: {lines}");
    }
}

#[test]
fn a_stale_or_dirty_stop_goes_back_to_observe() {
    for code in [
        ResultCode::InvalidInput,
        ResultCode::ExternalChange,
        ResultCode::WorktreeNotClean,
    ] {
        let lines = way_on(&stopped(code, LocalContextSyncNextAction::None), ROOT, ITEM);
        assert!(
            lines.starts_with(&format!(
                "next: manyhands-cli context observe-local --repo {} --id {ITEM}\n",
                quoted(ROOT)
            )),
            "{code:?}: {lines}"
        );
    }
}

#[test]
fn only_a_repair_outside_says_so() {
    let mut envelope = stopped(
        ResultCode::RecoveryRequired,
        LocalContextSyncNextAction::ExternalRepair,
    );
    for (reason, says) in [
        (LocalContextSyncReason::UnsupportedConflict, true),
        (LocalContextSyncReason::UnrelatedHistories, true),
        (LocalContextSyncReason::ImmutableClosureDisagreement, true),
        (LocalContextSyncReason::AuthorityMissing, false),
        (LocalContextSyncReason::AmbiguousBackend, false),
    ] {
        if let Some(MutationDataDto::LocalContextSync(data)) = &mut envelope.data {
            data.reason = reason;
        }
        assert_eq!(
            way_on(&envelope, ROOT, ITEM).contains(EXTERNAL_NOTE),
            says,
            "{reason:?}"
        );
    }
}

#[test]
fn identity_required_says_the_same_request_follows() {
    let lines = way_on(
        &stopped(
            ResultCode::IdentityRequired,
            LocalContextSyncNextAction::SetIdentity,
        ),
        ROOT,
        ITEM,
    );
    assert!(lines.contains("same request"), "{lines}");
}
