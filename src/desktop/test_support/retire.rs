//! Canned `context retire` answers (D6 decision 9): the library's own
//! shape, built the way its retirement tests build theirs, so the desktop's
//! reading of an answer is tested without a remote.

use manyhands::{
    repository::{
        ContextRetirementMutationDto, ContextRetirementMutationType, ContextRetirementState,
        ItemDtoKind, MutationDataDto, RetirementRemoval,
    },
    results::{Effects, Envelope, Outcome, ResultCode, Scope},
};

/// The item every canned answer is about.
pub(crate) const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG0";
/// The operation of the retirement's journal row.
pub(crate) const OPERATION: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";

/// A retirement's answer: `state` under `code`, having removed `removal` of
/// the worktree. The envelope's outcome is the library's own
/// (`Outcome::of_mutation`, U-10), so a refusal is blocked and a pending
/// retirement partial.
pub(crate) fn answer(
    state: ContextRetirementState,
    code: ResultCode,
    removal: RetirementRemoval,
) -> Envelope<MutationDataDto> {
    answer_of(OPERATION, state, code, removal)
}

/// [`answer`] for the operation `operation`.
pub(crate) fn answer_of(
    operation: &str,
    state: ContextRetirementState,
    code: ResultCode,
    removal: RetirementRemoval,
) -> Envelope<MutationDataDto> {
    let outcome = Outcome::of_mutation(
        false,
        code,
        &Effects::not_requested(),
        removal != RetirementRemoval::None,
    );
    Envelope::mutation("context retire", Scope::default(), outcome, code)
        .with_operation_id(operation)
        .with_data(MutationDataDto::ContextRetirement(
            ContextRetirementMutationDto {
                r#type: ContextRetirementMutationType::ContextRetirement,
                kind: ItemDtoKind::Ticket,
                item_id: ITEM.to_owned(),
                operation_id: operation.to_owned(),
                state,
                code,
                worktree_removal: removal,
                branch_removal: RetirementRemoval::None,
                materialization_settled: false,
            },
        ))
}

/// A request refused with `code` before the retirement had data to give:
/// the plain failure envelope of any command.
pub(crate) fn no_data(code: ResultCode) -> Envelope<MutationDataDto> {
    Envelope::failure("context retire", Scope::default(), code, vec![])
}

/// `operation resume` answered `original_request_required`, naming the
/// request that holds the operation (Fact 26): the one a pending
/// retirement's Resume gets.
pub(crate) fn original_request(
    named: manyhands::repository::RequestId,
) -> Envelope<MutationDataDto> {
    use manyhands::results::{RecoveryAction, RecoveryActionKind};
    Envelope::failure(
        "operation resume",
        Scope::default(),
        ResultCode::OriginalRequestRequired,
        vec![RecoveryAction::new(
            RecoveryActionKind::RequestRetry,
            [("request_id", named.to_string().into())],
        )],
    )
}

/// The pending row the library lists for an Owed retirement (Fact 26): a
/// local `retire_context` operation waiting to be resumed, at `step`.
pub(crate) fn pending_row(
    operation: &str,
    state: &str,
    step: Option<&str>,
) -> manyhands::repository::OperationDto {
    use manyhands::repository::{
        OperationAction, OperationDto, OperationFamily, OperationNextAction, OperationOwner,
    };
    OperationDto {
        operation_id: Some(operation.to_owned()),
        family: OperationFamily::Local,
        owner: OperationOwner::Repository,
        action: OperationAction::RetireContext,
        state: state.to_owned(),
        completed_step: step.map(str::to_owned),
        next_action: Some(OperationNextAction::Resume),
        item_id: None,
        key_id: None,
        worktree: None,
        updated_at: None,
        failure_code: None,
    }
}

/// The library's complete list of pending operations naming the retirement
/// row `operation`, as the scheduler's reconciliation reads it.
pub(crate) fn pending_list(operation: &str) -> manyhands::repository::OperationListDto {
    manyhands::repository::OperationListDto {
        items: vec![pending_row(operation, "created", None)],
        complete: true,
    }
}

/// A complete list that names no pending operation.
pub(crate) fn empty_list() -> manyhands::repository::OperationListDto {
    manyhands::repository::OperationListDto {
        items: Vec::new(),
        complete: true,
    }
}
