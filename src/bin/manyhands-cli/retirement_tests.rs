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
    use crate::render::test_support::{no_raw_control, stderr};
    use manyhands::results::{Effects, RecoveryAction, RecoveryActionKind, ResultCode, Scope};
    const CURRENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX0";
    const ROOT: &str = "/current's\troot\n";
    for with_data in [true, false] {
        let mut envelope = Envelope::classified_mutation(
            "context retire",
            Scope {
                repository: Some(ROOT.into()),
                ..Scope::default()
            },
            false,
            ResultCode::RecoveryRequired,
            Effects::not_requested(),
            false,
        );
        envelope.operation_id = Some("old-operation".into());
        if with_data {
            envelope.data = Some(MutationDataDto::ContextRetirement(data.clone()));
        }
        envelope.recovery = vec![
            RecoveryAction::for_operation(RecoveryActionKind::OperationResume, CURRENT, []),
            RecoveryAction::for_operation(
                RecoveryActionKind::OperationAbandon,
                CURRENT,
                [("root", serde_json::Value::from(ROOT))],
            ),
        ];
        let before = serde_json::to_value(&envelope).unwrap();
        let out = output(&envelope, false);
        let text = stderr(&out);
        assert!(
            text.contains(&format!(
                "next: manyhands-cli operation resume --repo {} --operation-id '{CURRENT}'\n",
                render::quoted(ROOT)
            )),
            "{text}"
        );
        assert!(text.contains(&format!("next: manyhands-cli operation abandon --repo {} --operation-id '{CURRENT}' --prepare\n", render::quoted(ROOT))), "{text}");
        assert_eq!(serde_json::to_value(&envelope).unwrap(), before);
        no_raw_control(text);

        envelope.scope.repository = None;
        envelope.recovery = vec![
            RecoveryAction::for_operation(RecoveryActionKind::OperationResume, CURRENT, []),
            RecoveryAction::for_operation(RecoveryActionKind::OperationAbandon, CURRENT, []),
        ];
        let out = output(&envelope, false);
        let text = stderr(&out);
        assert!(text.contains(&format!("next: operation.resume operation_id={CURRENT}\n")));
        assert!(text.contains(&format!("next: operation.abandon operation_id={CURRENT}\n")));
        assert!(
            !text.contains("--repo"),
            "missing root is never inferred from historical data"
        );
    }
}
