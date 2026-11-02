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
    )
    .lines;
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
        let lines = way_on(&stopped(ResultCode::RecoveryRequired, action), ROOT, ITEM).lines;
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
        let lines = way_on(&stopped(code, LocalContextSyncNextAction::None), ROOT, ITEM).lines;
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
    for (action, reason, says) in [
        (
            LocalContextSyncNextAction::None,
            LocalContextSyncReason::UnrelatedHistories,
            true,
        ),
        (
            LocalContextSyncNextAction::None,
            LocalContextSyncReason::ImmutableClosureDisagreement,
            true,
        ),
        (
            LocalContextSyncNextAction::None,
            LocalContextSyncReason::ForeignChange,
            false,
        ),
    ]
    .into_iter()
    .chain(
        [
            (LocalContextSyncReason::UnsupportedConflict, true),
            (LocalContextSyncReason::UnrelatedHistories, true),
            (LocalContextSyncReason::ImmutableClosureDisagreement, true),
            (LocalContextSyncReason::AuthorityMissing, false),
            (LocalContextSyncReason::AmbiguousBackend, false),
        ]
        .map(|(reason, says)| (LocalContextSyncNextAction::ExternalRepair, reason, says)),
    ) {
        if let Some(MutationDataDto::LocalContextSync(data)) = &mut envelope.data {
            data.reason = reason;
            data.next_action = action;
        }
        assert_eq!(
            way_on(&envelope, ROOT, ITEM).lines.contains(EXTERNAL_NOTE),
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
    )
    .lines;
    assert!(lines.contains("same request"), "{lines}");
}

#[test]
fn a_refused_resolution_goes_back_to_show_local() {
    for code in [ResultCode::InvalidInput, ResultCode::ExternalChange] {
        let lines = resolve_way_on(
            &stopped(code, LocalContextSyncNextAction::Resolve),
            ROOT,
            OPERATION,
            None,
        )
        .lines;
        assert_eq!(
            lines,
            format!(
                "next: manyhands-cli context show-local --repo {} --operation-id {OPERATION}\n",
                quoted(ROOT)
            ),
            "{code:?}"
        );
    }
    let lines = resolve_way_on(
        &stopped(
            ResultCode::OriginalRequestRequired,
            LocalContextSyncNextAction::RetryResolution,
        ),
        ROOT,
        OPERATION,
        None,
    )
    .lines;
    assert!(lines.contains("same input"), "{lines}");
}

const ORIGINAL: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX3";

#[test]
fn a_new_request_after_a_partial_resolution_names_the_original_one() {
    let lines = resolve_way_on(
        &stopped(
            ResultCode::ExternalChange,
            LocalContextSyncNextAction::ExternalRepair,
        ),
        ROOT,
        OPERATION,
        Some(ORIGINAL),
    )
    .lines;
    assert!(
        lines.starts_with(&format!("next: request.retry request_id={ORIGINAL}\n")),
        "{lines}"
    );
    assert!(lines.contains("same input"), "{lines}");
    assert!(!lines.contains("show-local"), "{lines}");
}

#[test]
fn the_original_request_is_named_once() {
    let mut envelope = stopped(
        ResultCode::OriginalRequestRequired,
        LocalContextSyncNextAction::RetryResolution,
    );
    envelope
        .recovery
        .push(manyhands::results::RecoveryAction::new(
            manyhands::results::RecoveryActionKind::RequestRetry,
            [("request_id", ORIGINAL.into())],
        ));
    let lines = resolve_way_on(&envelope, ROOT, OPERATION, Some(ORIGINAL)).lines;
    assert!(!lines.contains("next: request.retry"), "{lines}");
    assert!(lines.contains("same input"), "{lines}");
}

/// The data's `next action` is left out only where a line takes its place: one
/// to look again, or the original request to send.
#[test]
fn only_a_line_to_look_again_replaces_next_action() {
    let stale = stopped(
        ResultCode::ExternalChange,
        LocalContextSyncNextAction::RetryResolution,
    );
    assert!(resolve_way_on(&stale, ROOT, OPERATION, None).replaces_next_action);
    assert!(resolve_way_on(&stale, ROOT, OPERATION, Some(ORIGINAL)).replaces_next_action);
    let no_line = stopped(
        ResultCode::RecoveryRequired,
        LocalContextSyncNextAction::RetryResolution,
    );
    assert!(!resolve_way_on(&no_line, ROOT, OPERATION, None).replaces_next_action);
    assert!(way_on(&stale, ROOT, ITEM).replaces_next_action);
    let refused = stopped(
        ResultCode::RecoveryRequired,
        LocalContextSyncNextAction::ExternalRepair,
    );
    assert!(!way_on(&refused, ROOT, ITEM).replaces_next_action);
}
