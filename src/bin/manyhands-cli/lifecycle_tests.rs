//! The human lines of `document promote`, `ticket close` and an `operation
//! resume` that continued one, on envelopes the binary tests can't reach
//! without a remote (`tests/cli/lifecycle.rs` runs the rest). The text is the
//! pin: `context observe-local` and its three siblings arrive in Task 10, and
//! these rows hold what the refused merge says of them until then.

use crate::{human_lifecycle::output, render::test_support::stderr, render::test_support::stdout};
use manyhands::{
    repository::{
        ItemDtoKind, LifecycleBlocker, LifecycleCleanupState, LifecycleIntegrationKind,
        LifecycleMadeReadyDto, LifecycleMadeReadyState, LifecycleMutationDto,
        LifecycleMutationType, LifecyclePublicationPendingReason, LifecyclePublicationState,
        LifecycleRecording, MutationDataDto,
    },
    results::{Effects, Envelope, RecoveryAction, RecoveryActionKind, ResultCode, Scope},
};

const ROOT: &str = "/repo";
const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
const REQUEST: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX1";
const OPERATION: &str = "01ARZ3NDEKTSV4RRFFQ69G5FY1";
const DEPENDENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD1";
const OID: &str = "0123456789012345678901234567890123456789";

fn data(kind: ItemDtoKind) -> LifecycleMutationDto {
    let closes = kind == ItemDtoKind::Ticket;
    LifecycleMutationDto {
        r#type: LifecycleMutationType::Lifecycle,
        kind,
        item_id: ITEM.into(),
        recording: closes.then_some(LifecycleRecording::Record),
        checkpoint_oid: closes.then(|| OID.into()),
        integration: Some(LifecycleIntegrationKind::Merged),
        integration_oid: Some(OID.into()),
        publication: Some(LifecyclePublicationState::Published),
        publication_oid: Some(OID.into()),
        publication_pending: None,
        cleanup: Some(LifecycleCleanupState::Complete),
        cleanup_pending: None,
        blockers: Vec::new(),
        made_ready: closes.then(Vec::new),
        made_ready_state: closes.then_some(LifecycleMadeReadyState::Current),
    }
}

fn scope(item: bool) -> Scope {
    Scope {
        repository: Some(ROOT.into()),
        item_id: item.then(|| ITEM.into()),
        ..Scope::default()
    }
}

fn answer(
    command: &str,
    code: ResultCode,
    data: Option<LifecycleMutationDto>,
    recovery: Vec<RecoveryAction>,
) -> Envelope<MutationDataDto> {
    let mut envelope = Envelope::<MutationDataDto>::classified_mutation(
        command,
        scope(command != "operation resume"),
        false,
        code,
        Effects::not_requested(),
        false,
    );
    envelope.recovery = recovery;
    envelope.request_id = Some(REQUEST.into());
    envelope.operation_id = Some(OPERATION.into());
    envelope.data = data.map(|data| MutationDataDto::Lifecycle(Box::new(data)));
    envelope
}

fn retry() -> RecoveryAction {
    RecoveryAction::new(
        RecoveryActionKind::RequestRetry,
        [("request_id", REQUEST.into())],
    )
}

fn cancel() -> RecoveryAction {
    RecoveryAction::new(
        RecoveryActionKind::RequestCancel,
        [("request_id", REQUEST.into())],
    )
}

fn inspect() -> RecoveryAction {
    RecoveryAction::new(RecoveryActionKind::RepoInspect, [("root", ROOT.into())])
}

fn next_lines(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|line| line.starts_with("next: "))
        .collect()
}

#[test]
fn a_published_closure_with_nothing_made_ready_says_so() {
    let out = output(
        &answer(
            "ticket close",
            ResultCode::Ok,
            Some(data(ItemDtoKind::Ticket)),
            vec![],
        ),
        false,
    );
    assert_eq!(out.status, 0);
    assert_eq!(
        stdout(&out),
        format!(
            "The request completed.\nrequest: {REQUEST}\noperation: {OPERATION}\n\
             Blockers: none\nCheckpoint oid: {OID}\nCleanup: complete\nCleanup pending: none\n\
             Integration: merged\nIntegration oid: {OID}\nItem id: {ITEM}\nKind: ticket\n\
             Made ready state: current\nPublication: published\nPublication oid: {OID}\n\
             Publication pending: none\nRecording: record\nmade ready: none\n"
        )
    );
    assert_eq!(stderr(&out), "");
}

#[test]
fn a_promotion_reports_no_made_ready_at_all() {
    let out = output(
        &answer(
            "document promote",
            ResultCode::Ok,
            Some(data(ItemDtoKind::Document)),
            vec![],
        ),
        false,
    );
    assert_eq!(out.status, 0);
    assert!(
        !stdout(&out).to_lowercase().contains("made ready"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn made_ready_lines_carry_the_slug_when_there_is_one() {
    let mut closed = data(ItemDtoKind::Ticket);
    closed.made_ready = Some(vec![
        LifecycleMadeReadyDto {
            id: DEPENDENT.into(),
            slug: Some("mh-vh-k9x2b".into()),
        },
        LifecycleMadeReadyDto {
            id: ITEM.into(),
            slug: None,
        },
    ]);
    let out = output(
        &answer("ticket close", ResultCode::Ok, Some(closed), vec![]),
        false,
    );
    assert!(
        stdout(&out).ends_with(&format!(
            "made ready: {DEPENDENT} mh-vh-k9x2b\nmade ready: {ITEM}\n"
        )),
        "{}",
        stdout(&out)
    );
    assert!(!stdout(&out).contains("Made ready id"), "{}", stdout(&out));
}

#[test]
fn a_state_with_no_set_points_to_the_live_view() {
    for state in [
        LifecycleMadeReadyState::IndexPending,
        LifecycleMadeReadyState::IndexUnavailableFinal,
    ] {
        let mut closed = data(ItemDtoKind::Ticket);
        closed.made_ready_state = Some(state);
        let out = output(
            &answer("ticket close", ResultCode::Ok, Some(closed), vec![]),
            false,
        );
        assert!(!stdout(&out).contains("made ready: "), "{}", stdout(&out));
        assert!(
            stdout(&out).contains(&format!("Made ready state: {}\n", state.as_str())),
            "{}",
            stdout(&out)
        );
        assert_eq!(
            stderr(&out),
            "note: this answer holds no made-ready set; ticket ready is the live view.\n"
        );
    }
    for state in [
        LifecycleMadeReadyState::NotComputed,
        LifecycleMadeReadyState::NotApplicable,
    ] {
        let mut closed = data(ItemDtoKind::Ticket);
        closed.made_ready_state = Some(state);
        let out = output(
            &answer("ticket close", ResultCode::Ok, Some(closed), vec![]),
            false,
        );
        assert!(!stdout(&out).contains("made ready: "), "{}", stdout(&out));
        assert_eq!(stderr(&out), "");
    }
}

#[test]
fn a_pending_publication_with_no_remote_names_the_later_sync() {
    let mut promoted = data(ItemDtoKind::Document);
    promoted.publication = Some(LifecyclePublicationState::Pending);
    promoted.publication_oid = None;
    promoted.publication_pending = Some(LifecyclePublicationPendingReason::NoPublicationRemote);
    let out = output(
        &answer("document promote", ResultCode::Ok, Some(promoted), vec![]),
        false,
    );
    assert_eq!(out.status, 0);
    assert_eq!(
        stderr(&out),
        "note: primary is not published, because this repository has no publication remote; once one is selected, repo sync publishes it.\n"
    );
}

/// The refused merge's lines. The four verbs arrive in Task 10, and Task 10's
/// grammar has to answer to this text.
#[test]
fn a_refused_checkpoint_merge_names_the_four_context_verbs_then_its_actions() {
    let mut refused = data(ItemDtoKind::Ticket);
    refused.blockers = vec![LifecycleBlocker::IntegrationConflict];
    let out = output(
        &answer(
            "ticket close",
            ResultCode::RecoveryRequired,
            Some(refused),
            vec![retry(), inspect()],
        ),
        false,
    );
    assert!(out.stdout.is_empty());
    assert_eq!(
        next_lines(stderr(&out)),
        [
            format!("next: manyhands-cli context observe-local --repo '{ROOT}' --id {ITEM}"),
            format!(
                "next: manyhands-cli context sync-local --repo '{ROOT}' --id {ITEM} --input - --request-id <new-request-id>"
            ),
            format!(
                "next: manyhands-cli context show-local --repo '{ROOT}' --operation-id <sync-operation-id>"
            ),
            format!(
                "next: manyhands-cli context resolve-local --repo '{ROOT}' --operation-id <sync-operation-id> --input - --request-id <new-request-id>"
            ),
            format!("next: request.retry request_id={REQUEST}"),
            format!("next: repo.inspect root={ROOT}"),
        ]
    );
    assert!(
        stderr(&out).contains("Blockers: integration_conflict\n"),
        "{}",
        stderr(&out)
    );
    assert!(
        stderr(&out).contains(
            "note: primary refused this closure's checkpoint: bring primary into the ticket's context with the context commands, then send this request again; the closure's fields are kept.\n"
        ),
        "{}",
        stderr(&out)
    );
}

#[test]
fn other_blockers_are_named_with_no_repair_lines() {
    let mut blocked = data(ItemDtoKind::Document);
    blocked.blockers = vec![
        LifecycleBlocker::PrimaryDirty,
        LifecycleBlocker::ContextDirty,
    ];
    let out = output(
        &answer(
            "document promote",
            ResultCode::RecoveryRequired,
            Some(blocked),
            vec![inspect()],
        ),
        false,
    );
    let text = stderr(&out);
    assert!(text.contains("Blockers: primary_dirty\n"), "{text}");
    assert!(text.contains("Blockers: context_dirty\n"), "{text}");
    assert!(!text.contains("context observe-local"), "{text}");
}

/// Decision 18's route: cancel, the same request (`cancelled`), republish, the
/// same request again (`ok`).
#[test]
fn a_child_s_deleted_branch_prints_the_route_out() {
    for (command, kind) in [
        ("ticket close", ItemDtoKind::Ticket),
        ("document promote", ItemDtoKind::Document),
    ] {
        let again =
            format!("manyhands-cli {command} --repo '{ROOT}' --id {ITEM} --request-id {REQUEST}");
        let out = output(
            &answer(
                command,
                ResultCode::RemoteBranchDeleted,
                Some(data(kind)),
                vec![cancel(), inspect()],
            ),
            false,
        );
        assert_eq!(
            next_lines(stderr(&out)),
            [
                format!("next: manyhands-cli request cancel --request-id {REQUEST}"),
                format!("next: {again}"),
                format!("next: manyhands-cli item republish --repo '{ROOT}' --id {ITEM} --prepare"),
                format!("next: {again}"),
                format!("next: repo.inspect root={ROOT}"),
            ],
            "{command}"
        );
        assert!(
            stderr(&out).contains(
                "note: the refused request's context synchronization keeps its reservation until the request is cancelled; sending the request again then answers cancelled and releases it. After the republish, sending it once more finishes the work.\n"
            ),
            "{}",
            stderr(&out)
        );
    }
}

/// Decision 19's route: nothing to cancel, since the resume asked already.
#[test]
fn a_resume_s_deleted_branch_prints_republish_then_resume() {
    let operation = RecoveryAction {
        action: RecoveryActionKind::OperationResume,
        operation_id: Some(OPERATION.into()),
        arguments: serde_json::Map::new(),
    };
    let out = output(
        &answer(
            "operation resume",
            ResultCode::RemoteBranchDeleted,
            Some(data(ItemDtoKind::Ticket)),
            vec![operation, inspect()],
        ),
        false,
    );
    assert_eq!(
        next_lines(stderr(&out)),
        [
            format!("next: manyhands-cli item republish --repo '{ROOT}' --id {ITEM} --prepare"),
            format!(
                "next: manyhands-cli operation resume --repo '{ROOT}' --operation-id {OPERATION}"
            ),
            format!("next: repo.inspect root={ROOT}"),
        ]
    );
    assert!(
        stderr(&out).contains(
            "note: this resume released the operation's context synchronization if nothing it holds could be lost. If repo inspect still shows its reservation, it was kept, and the republish answers busy.\n"
        ),
        "{}",
        stderr(&out)
    );
}

#[test]
fn a_result_with_no_lifecycle_data_is_c2_s() {
    let out = output(
        &answer(
            "ticket close",
            ResultCode::ConfirmationRequired,
            None,
            vec![],
        ),
        false,
    );
    assert_eq!(out.status, 3);
    assert!(out.stdout.is_empty());
    assert!(
        stderr(&out).contains("code: confirmation_required\n"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn a_wrong_passphrase_keeps_c2_s_hint() {
    let out = output(
        &answer(
            "ticket close",
            ResultCode::TransportUnavailable,
            Some(data(ItemDtoKind::Ticket)),
            vec![retry()],
        ),
        true,
    );
    assert!(
        stderr(&out).contains("hint: if the passphrase was wrong, run the command again\n"),
        "{}",
        stderr(&out)
    );
}
