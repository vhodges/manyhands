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

const OTHER_REQUEST: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX2";

/// A refusal of `command` for `request`, with a `request.retry` action for
/// each request in `named`.
fn refusal(
    command: &str,
    code: ResultCode,
    request: &str,
    named: &[&str],
) -> manyhands::results::Envelope<MutationDataDto> {
    use manyhands::results::{Envelope, RecoveryAction, RecoveryActionKind, Scope};
    let mut envelope = Envelope::<MutationDataDto>::failure(
        command,
        Scope::default(),
        code,
        named
            .iter()
            .map(|named| {
                RecoveryAction::new(
                    RecoveryActionKind::RequestRetry,
                    [("request_id", (*named).into())],
                )
            })
            .collect(),
    );
    envelope.request_id = Some(request.into());
    envelope
}

fn seen_of(seen: &Seen) -> (bool, bool, Option<&str>) {
    (seen.supersedable, seen.written, seen.item.as_deref())
}

#[test]
fn only_a_resume_s_original_request_and_a_resolve_s_stale_or_deleted_result_read_the_conflict() {
    use ResultCode::*;
    for (command, code, named, wanted) in [
        (
            "operation resume",
            OriginalRequestRequired,
            vec![REQUEST],
            true,
        ),
        // An original request that names no request has nothing to compare.
        ("operation resume", OriginalRequestRequired, vec![], false),
        ("operation resume", ExternalChange, vec![REQUEST], false),
        ("conflict resolve", ExternalChange, vec![], true),
        ("conflict resolve", RemoteBranchDeleted, vec![], true),
        // A resolve answers this only when the attempt's written paths
        // changed, so the note needs no read (controller ruling 2).
        (
            "conflict resolve",
            OriginalRequestRequired,
            vec![REQUEST],
            false,
        ),
        ("conflict resolve", RecoveryRequired, vec![], false),
        ("conflict resolve", InvalidInput, vec![], false),
        ("conflict resolve", Busy, vec![], false),
        ("comment add", ExternalChange, vec![], false),
        ("comment add", RemoteBranchDeleted, vec![], false),
        ("item sync", RemoteBranchDeleted, vec![], false),
        ("item sync", ExternalChange, vec![], false),
    ] {
        let envelope = refusal(command, code, OTHER_REQUEST, &named);
        assert_eq!(wants_conflict(&envelope), wanted, "{command} {code:?}");
    }
}

#[test]
fn a_resolve_refused_on_a_conflict_with_written_paths_is_marked_written() {
    let stale = refusal("conflict resolve", ResultCode::ExternalChange, REQUEST, &[]);
    // `historical`: an attempt has written paths (R-39).
    assert_eq!(
        seen_of(&observed(&stale, true, Some(REQUEST), None)),
        (false, true, None)
    );
    assert_eq!(
        seen_of(&observed(&stale, false, None, None)),
        (false, false, None)
    );
    // Another request's attempt with written paths marks it too: the
    // conflict reads as historical whoever's the attempt is.
    assert_eq!(
        seen_of(&observed(&stale, true, Some(OTHER_REQUEST), None)),
        (false, true, None)
    );
    // An item belongs to a deleted branch's result only.
    assert_eq!(
        seen_of(&observed(&stale, true, None, Some(ITEM.into()))),
        (false, true, None)
    );
}

#[test]
fn a_resolve_that_met_a_deleted_branch_takes_the_conflict_s_item() {
    let deleted = refusal(
        "conflict resolve",
        ResultCode::RemoteBranchDeleted,
        OTHER_REQUEST,
        &[],
    );
    assert_eq!(
        seen_of(&observed(&deleted, true, Some(REQUEST), Some(ITEM.into()))),
        (false, false, Some(ITEM))
    );
    // A conflict that names no item (a primary synchronization) adds none.
    assert_eq!(
        seen_of(&observed(&deleted, false, None, None)),
        (false, false, None)
    );
}

#[test]
fn a_resume_refused_for_the_request_with_written_paths_is_marked_supersedable() {
    let resume = refusal(
        "operation resume",
        ResultCode::OriginalRequestRequired,
        OTHER_REQUEST,
        &[REQUEST],
    );
    assert_eq!(
        seen_of(&observed(&resume, true, Some(REQUEST), None)),
        (true, false, None)
    );
    assert_eq!(
        seen_of(&observed(&resume, false, Some(REQUEST), None)),
        (false, false, None)
    );
    assert_eq!(
        seen_of(&observed(&resume, true, Some(OTHER_REQUEST), None)),
        (false, false, None)
    );
}
