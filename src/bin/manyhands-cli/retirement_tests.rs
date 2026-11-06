use super::*;
use manyhands::repository::{
    ContextRetirementMutationType, ContextRetirementState, ItemDtoKind, RetirementRemoval,
};

#[test]
fn removal_flags_are_evidence_not_an_all_removed_claim() {
    let data = ContextRetirementMutationDto {
        r#type: ContextRetirementMutationType::ContextRetirement,
        kind: ItemDtoKind::Ticket,
        item_id: "item".into(),
        operation_id: "operation".into(),
        state: ContextRetirementState::Pending,
        code: manyhands::results::ResultCode::RecoveryRequired,
        worktree_removal: RetirementRemoval::Possible,
        branch_removal: RetirementRemoval::None,
        materialization_settled: false,
    };
    assert_eq!(
        removal(&data),
        "worktree_removal=possible branch_removal=none materialization_settled=false"
    );
}
