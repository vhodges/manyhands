use super::*;
use crate::mutate::stops_on_ctrl_c;
use manyhands::canonical::ItemId;
use manyhands::repository::{
    CommentAddInput, ConflictResolveInput, IndexRefreshInput, ItemRepublishInput,
};

const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
const REQUEST: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX1";

fn item() -> ItemId {
    ITEM.parse().unwrap()
}

#[test]
fn the_window_covers_exactly_the_six_stoppable_verbs() {
    let root = PathBuf::from("/r");
    let operation_id = OperationId::new();
    let kind = AuthoringKind::Ticket;
    for (mutation, windowed) in [
        (
            Mutation::ItemSync(ItemSyncInput {
                root: root.clone(),
                kind,
                id: item(),
            }),
            true,
        ),
        (
            Mutation::RepoSync(RepoSyncInput { root: root.clone() }),
            true,
        ),
        (
            Mutation::ItemRepublish(ItemRepublishInput {
                root: root.clone(),
                kind,
                id: item(),
            }),
            true,
        ),
        (
            Mutation::CommentAdd(CommentAddInput {
                root: root.clone(),
                kind,
                id: item(),
                comment_id: item(),
                parent_id: None,
                body: String::new(),
                created_by: None,
            }),
            true,
        ),
        (
            Mutation::ConflictResolve(ConflictResolveInput {
                root: root.clone(),
                operation_id,
                observation: String::new(),
                resolutions: Vec::new(),
            }),
            true,
        ),
        (
            Mutation::OperationResume(OperationResumeInput {
                root: root.clone(),
                operation_id,
            }),
            true,
        ),
        (
            Mutation::OperationAbandon(OperationAbandonInput {
                root: root.clone(),
                operation_id,
            }),
            false,
        ),
        (
            Mutation::IndexRefresh(IndexRefreshInput { root: root.clone() }),
            false,
        ),
    ] {
        assert_eq!(
            stops_on_ctrl_c(&mutation),
            windowed,
            "{}",
            mutation.command()
        );
    }
}

#[test]
fn an_item_s_kind_is_its_authoring_kind() {
    assert_eq!(authoring_kind(ItemDtoKind::Ticket), AuthoringKind::Ticket);
    assert_eq!(
        authoring_kind(ItemDtoKind::Document),
        AuthoringKind::Document
    );
}

#[test]
fn only_the_named_request_s_resolution_with_written_paths_may_be_superseded() {
    assert!(supersedable(true, Some(REQUEST), REQUEST));
    // No paths written: the attempt is no written resolution.
    assert!(!supersedable(false, Some(REQUEST), REQUEST));
    // The unfinished attempt is another request's, or no request's.
    assert!(!supersedable(
        true,
        Some("01ARZ3NDEKTSV4RRFFQ69G5FX2"),
        REQUEST
    ));
    assert!(!supersedable(true, None, REQUEST));
}

#[test]
fn the_request_an_original_request_required_names_is_another_request_s() {
    use manyhands::results::{Envelope, RecoveryAction, RecoveryActionKind, Scope};
    let mut envelope = Envelope::<MutationDataDto>::failure(
        "operation resume",
        Scope::default(),
        ResultCode::OriginalRequestRequired,
        vec![RecoveryAction::new(
            RecoveryActionKind::RequestRetry,
            [("request_id", REQUEST.into())],
        )],
    );
    envelope.request_id = Some("01ARZ3NDEKTSV4RRFFQ69G5FX2".into());
    assert_eq!(original_request(&envelope), Some(REQUEST));
    // The envelope's own request is no original request.
    envelope.request_id = Some(REQUEST.into());
    assert_eq!(original_request(&envelope), None);
    envelope.request_id = Some("01ARZ3NDEKTSV4RRFFQ69G5FX2".into());
    envelope.code = ResultCode::RecoveryRequired;
    assert_eq!(original_request(&envelope), None);
}
