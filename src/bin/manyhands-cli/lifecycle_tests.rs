//! The human lines of `document promote`, `ticket close` and an `operation
//! resume` that continued one, on envelopes the binary tests can't reach
//! without a remote (`tests/cli/lifecycle.rs` runs the rest). The text is the
//! pin of what the refused merge says of `context observe-local` and its three
//! siblings, whose route `tests/cli/local_sync.rs`
//! `the_refused_merges_route_finishes_the_close` follows.

use crate::{
    human_lifecycle::{self, output},
    mutate::should_retry,
    render::test_support::{no_raw_control, stderr, stdout},
};
use manyhands::{
    repository::{
        ItemDtoKind, LifecycleBlockedDto, LifecycleBlocker, LifecycleCleanupState,
        LifecycleIntegrationKind, LifecycleMadeReadyDto, LifecycleMadeReadyState,
        LifecycleMutationDto, LifecycleMutationType, LifecyclePublicationPendingReason,
        LifecyclePublicationState, LifecycleRecording, MutationDataDto, PrepareDataDto, RequestId,
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

/// The verb's prepare, its kind the one the envelope's command acts on.
fn prepare(envelope: &Envelope<PrepareDataDto>) -> crate::render::Output {
    let kind = if envelope.command == "document promote" {
        ItemDtoKind::Document
    } else {
        ItemDtoKind::Ticket
    };
    human_lifecycle::prepare(kind, envelope)
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

/// The refused merge's lines, whose grammar the four context verbs answer to
/// (`tests/cli/local_sync.rs` `the_refused_merges_route_finishes_the_close`).
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
            "note: primary refused this closure's checkpoint. The commands above are the local repair: the <...> values are placeholders to fill in, context sync-local's --input - takes {\"observation\": \"...\"} with the observation the first line prints, and context show-local and context resolve-local apply only if context sync-local answers a conflict. Then send this request again; the closure's fields are kept.\n"
        ),
        "{}",
        stderr(&out)
    );
}

fn refused_merge(kind: ItemDtoKind, recording: Option<LifecycleRecording>) -> LifecycleMutationDto {
    let mut refused = data(kind);
    refused.blockers = vec![LifecycleBlocker::IntegrationConflict];
    refused.recording = recording;
    refused
}

fn resume_action() -> RecoveryAction {
    RecoveryAction::for_operation(RecoveryActionKind::OperationResume, OPERATION, [])
}

#[test]
fn a_refused_merge_under_a_resume_says_to_resume_it_again() {
    let out = output(
        &answer(
            "operation resume",
            ResultCode::RecoveryRequired,
            Some(refused_merge(
                ItemDtoKind::Ticket,
                Some(LifecycleRecording::Record),
            )),
            vec![resume_action(), inspect()],
        ),
        false,
    );
    let text = stderr(&out);
    let lines = next_lines(text);
    assert_eq!(lines.len(), 6, "{text}");
    assert!(lines[0].contains("context observe-local"), "{text}");
    assert_eq!(
        lines[4],
        format!("next: manyhands-cli operation resume --repo '{ROOT}' --operation-id {OPERATION}")
    );
    assert_eq!(lines[5], format!("next: repo.inspect root={ROOT}"));
    assert!(
        text.contains("Then resume it again (add --request-id unless you run it at a terminal); the closure's fields are kept.\n"),
        "{text}"
    );
    assert!(!text.contains("send this request"), "{text}");
}

/// Only a closure's own checkpoint, recorded by it, with a way to go on, has this
/// route: `sync-local` refuses an existing closure (`local_sync/record.rs`).
#[test]
fn other_forms_of_the_blocker_print_no_route_and_rely_on_their_actions() {
    for (command, dto, actions) in [
        (
            "document promote",
            refused_merge(ItemDtoKind::Document, None),
            vec![retry(), inspect()],
        ),
        (
            "ticket close",
            refused_merge(ItemDtoKind::Ticket, Some(LifecycleRecording::Record)),
            vec![inspect()],
        ),
        (
            "ticket close",
            refused_merge(ItemDtoKind::Ticket, Some(LifecycleRecording::Existing)),
            vec![retry(), inspect()],
        ),
        (
            "ticket close",
            refused_merge(ItemDtoKind::Ticket, None),
            vec![retry(), inspect()],
        ),
    ] {
        let count = actions.len();
        let out = output(
            &answer(command, ResultCode::RecoveryRequired, Some(dto), actions),
            false,
        );
        let text = stderr(&out);
        assert!(!text.contains("context observe-local"), "{text}");
        assert!(!text.contains("primary refused"), "{text}");
        assert_eq!(next_lines(text).len(), count, "{text}");
    }
}

fn blocked(
    command: &str,
    code: ResultCode,
    blockers: Vec<LifecycleBlocker>,
    recovery: Vec<RecoveryAction>,
) -> Envelope<PrepareDataDto> {
    let mut envelope = Envelope::<PrepareDataDto>::failure(command, scope(true), code, recovery);
    envelope.data = Some(PrepareDataDto::LifecycleBlocked(LifecycleBlockedDto {
        blockers,
    }));
    envelope
}

#[test]
fn a_blocked_prepare_on_an_integration_conflict_offers_the_repair_then_a_new_prepare() {
    for command in ["ticket close", "document promote"] {
        let out = prepare(&blocked(
            command,
            ResultCode::RecoveryRequired,
            vec![LifecycleBlocker::IntegrationConflict],
            vec![inspect()],
        ));
        assert_eq!(out.status, 3);
        assert!(out.stdout.is_empty());
        let text = stderr(&out);
        let lines = next_lines(text);
        assert_eq!(lines.len(), 5, "{text}");
        assert!(lines[0].contains("context observe-local"), "{text}");
        assert!(lines[3].contains("context resolve-local"), "{text}");
        assert_eq!(lines[4], format!("next: repo.inspect root={ROOT}"));
        assert!(text.contains("Blockers: integration_conflict\n"), "{text}");
        assert!(
            text.contains(
                "note: this context can't merge into primary. The commands above are the local repair: the <...> values are placeholders to fill in, context sync-local's --input - takes {\"observation\": \"...\"} with the observation the first line prints, and context show-local and context resolve-local apply only if context sync-local answers a conflict. Then prepare again.\n"
            ),
            "{text}"
        );
        // An add/add, a promotion's or a ticket's, is external-only (decision 30).
        assert!(
            text.contains(
                "note: If context sync-local answers external_resolution_required, repair it with Git in the context.\n"
            ),
            "{text}"
        );
    }
}

/// `sync-local` refuses a dirty or conflicted context, and a pending conflict or
/// local operation is another route's.
#[test]
fn a_blocked_prepare_the_repair_cannot_reach_prints_no_route() {
    for other in [
        LifecycleBlocker::ContextDirty,
        LifecycleBlocker::ContextConflicted,
        LifecycleBlocker::ConflictPending,
        LifecycleBlocker::LocalOperationPending,
    ] {
        let out = prepare(&blocked(
            "ticket close",
            ResultCode::RecoveryRequired,
            vec![LifecycleBlocker::IntegrationConflict, other],
            vec![inspect()],
        ));
        let text = stderr(&out);
        assert!(!text.contains("context observe-local"), "{text}");
        assert_eq!(next_lines(text).len(), 1, "{text}");
    }
}

/// Decision 35: a published context whose remote state is unknown (no
/// tracking ref, and no fetch that saw it gone) names the poll that reads it,
/// then a new prepare.
#[test]
fn an_unknown_remote_context_names_the_poll() {
    for command in ["ticket close", "document promote"] {
        let out = prepare(&blocked(
            command,
            ResultCode::RecoveryRequired,
            vec![LifecycleBlocker::HistoryUnknown],
            vec![inspect()],
        ));
        assert_eq!(out.status, 3);
        let text = stderr(&out);
        assert_eq!(
            next_lines(text),
            [
                format!(
                    "next: manyhands-cli poll once --repo '{ROOT}' --request-id <new-request-id>"
                ),
                format!("next: repo.inspect root={ROOT}"),
            ],
            "{text}"
        );
        assert!(
            text.contains("note: this repository hasn't read whether the remote still holds this item's branch, and a poll reads it; <new-request-id> is a placeholder to fill in. Then prepare again.\n"),
            "{text}"
        );
    }
}

/// Lost history is the owner's to resolve: the way out is the repair read, and
/// the poll that settles push-proof unknown is not named.
#[test]
fn unresolved_history_names_the_repair_read_and_not_the_poll() {
    for command in ["ticket close", "document promote"] {
        let out = prepare(&blocked(
            command,
            ResultCode::RecoveryRequired,
            vec![LifecycleBlocker::HistoryUnresolved],
            vec![inspect()],
        ));
        assert_eq!(out.status, 3);
        let text = stderr(&out);
        assert_eq!(
            next_lines(text),
            [
                format!("next: manyhands-cli remote history --repo '{ROOT}'"),
                format!("next: repo.inspect root={ROOT}"),
            ],
            "{text}"
        );
        assert!(!text.contains("poll once"), "{text}");
        assert!(text.contains("item resolve-history"), "{text}");
    }
}

#[test]
fn a_missing_context_names_the_save_verb_by_kind() {
    for (command, save) in [
        ("ticket close", "ticket save"),
        ("document promote", "document save"),
    ] {
        let out = prepare(&blocked(
            command,
            ResultCode::RecoveryRequired,
            vec![LifecycleBlocker::ContextMissing],
            vec![inspect()],
        ));
        let text = stderr(&out);
        assert_eq!(
            next_lines(text),
            [
                format!(
                    "next: manyhands-cli {save} --repo '{ROOT}' --id {ITEM} --input - --request-id <new-request-id>"
                ),
                format!("next: repo.inspect root={ROOT}"),
            ],
            "{text}"
        );
        assert!(
            text.contains(
                "note: this item has no editing context, and a save makes one; <new-request-id> is a placeholder to fill in, and the save's --input - takes {\"observation\": \"...\"}, with the observation ticket show or document show prints, and the fields it changes, such as \"body\". Then prepare again.\n"
            ),
            "{text}"
        );
    }
}

#[test]
fn a_prepare_blocked_otherwise_is_c2_s() {
    let envelope = blocked(
        "ticket close",
        ResultCode::IdentityRequired,
        vec![LifecycleBlocker::IdentityRequired],
        vec![],
    );
    let out = prepare(&envelope);
    let plain = crate::render::prepare_output(false, &envelope);
    assert_eq!(stderr(&out), stderr(&plain));
    assert_eq!(out.status, plain.status);
}

#[test]
fn a_lifecycle_answer_naming_blockers_is_not_retried_automatically() {
    let id = RequestId::parse(REQUEST).unwrap();
    let mut refused = answer(
        "ticket close",
        ResultCode::RecoveryRequired,
        Some(refused_merge(
            ItemDtoKind::Ticket,
            Some(LifecycleRecording::Record),
        )),
        vec![retry(), inspect()],
    );
    refused.outcome = manyhands::results::Outcome::Partial;
    assert!(!should_retry(&refused, id));
    // The same stop naming no blockers still repeats, as the design accepts.
    refused.data = Some(MutationDataDto::Lifecycle(Box::new(data(
        ItemDtoKind::Ticket,
    ))));
    assert!(should_retry(&refused, id));
}

#[test]
fn a_root_and_a_slug_with_controls_are_escaped() {
    const HOSTILE: &str = "/re\npo\x1b[2J";
    let mut closed = refused_merge(ItemDtoKind::Ticket, Some(LifecycleRecording::Record));
    closed.made_ready = Some(vec![LifecycleMadeReadyDto {
        id: DEPENDENT.into(),
        slug: Some("mh\n\x1b[2Jfake".into()),
    }]);
    let mut refused = answer(
        "ticket close",
        ResultCode::RecoveryRequired,
        Some(closed.clone()),
        vec![retry(), inspect()],
    );
    refused.scope.repository = Some(HOSTILE.into());
    let out = output(&refused, false);
    no_raw_control(stderr(&out));
    assert!(
        stderr(&out).contains("--repo '/re\\npo\\u{1b}[2J'"),
        "{}",
        stderr(&out)
    );
    closed.blockers.clear();
    let out = output(
        &answer("ticket close", ResultCode::Ok, Some(closed), vec![]),
        false,
    );
    no_raw_control(stdout(&out));
    let mut deleted = answer(
        "ticket close",
        ResultCode::RemoteBranchDeleted,
        Some(data(ItemDtoKind::Ticket)),
        vec![cancel(), inspect()],
    );
    deleted.scope.repository = Some(HOSTILE.into());
    no_raw_control(stderr(&output(&deleted, false)));
    let mut missing = blocked(
        "ticket close",
        ResultCode::RecoveryRequired,
        vec![LifecycleBlocker::ContextMissing],
        vec![],
    );
    missing.scope.repository = Some(HOSTILE.into());
    no_raw_control(stderr(&prepare(&missing)));
}

/// A context child's conflict (decision 34): the blockers a stop on it names.
fn child_conflict(kind: ItemDtoKind) -> LifecycleMutationDto {
    let mut stopped = data(kind);
    stopped.blockers = vec![
        LifecycleBlocker::ContextConflicted,
        LifecycleBlocker::ConflictPending,
        LifecycleBlocker::ContextBehindRemote,
    ];
    stopped
}

/// The conflict verbs, named by this operation's ID, which they take for its
/// context child (decision 34), then C2's lines for the same request. Both
/// verbs.
#[test]
fn a_child_s_conflict_names_the_conflict_verbs_then_the_same_request() {
    for (command, kind) in [
        ("ticket close", ItemDtoKind::Ticket),
        ("document promote", ItemDtoKind::Document),
    ] {
        let out = output(
            &answer(
                command,
                ResultCode::RecoveryRequired,
                Some(child_conflict(kind)),
                vec![retry(), inspect()],
            ),
            false,
        );
        let text = stderr(&out);
        assert_eq!(
            next_lines(text),
            [
                format!(
                    "next: manyhands-cli conflict show --repo '{ROOT}' --operation-id {OPERATION}"
                ),
                format!(
                    "next: manyhands-cli conflict resolve --repo '{ROOT}' --operation-id {OPERATION} --input - --request-id <new-request-id>"
                ),
                format!("next: request.retry request_id={REQUEST}"),
                format!("next: repo.inspect root={ROOT}"),
            ],
            "{text}"
        );
        assert!(
            text.contains(
                "note: this operation's context synchronization stopped on a conflict. conflict show reads it, and --json gives each path's sides and token; when its next action is resolve, conflict resolve with the observation and a source for every token resolves and publishes it, and <new-request-id> is a placeholder to fill in. When it is external_repair, repair it with Git in the context and commit the merge. Then send this request again. The request finishes only if the resolved context holds what was confirmed; otherwise it answers external_change, and a new prepare is needed. After key select or key clear, the resolution is refused and there is no way on (a known limit).\n"
            ),
            "{text}"
        );
        assert!(!text.contains("context observe-local"), "{text}");
    }
}

/// Under a resume, the resume closes the route.
#[test]
fn a_child_s_conflict_under_a_resume_says_to_resume_it_again() {
    let out = output(
        &answer(
            "operation resume",
            ResultCode::RecoveryRequired,
            Some(child_conflict(ItemDtoKind::Ticket)),
            vec![resume_action(), inspect()],
        ),
        false,
    );
    let text = stderr(&out);
    let lines = next_lines(text);
    assert_eq!(lines.len(), 4, "{text}");
    assert!(lines[0].contains("conflict show"), "{text}");
    assert!(lines[1].contains("conflict resolve"), "{text}");
    assert_eq!(
        lines[2],
        format!("next: manyhands-cli operation resume --repo '{ROOT}' --operation-id {OPERATION}")
    );
    assert_eq!(lines[3], format!("next: repo.inspect root={ROOT}"));
    assert!(
        text.contains("Then resume it again (add --request-id unless you run it at a terminal). The request finishes only if"),
        "{text}"
    );
    assert!(!text.contains("send this request"), "{text}");
}

/// `conflict_pending` alone is any conflict of the repository's, not this
/// operation's child's: no conflict lines, and C2's actions alone.
#[test]
fn a_repository_s_pending_conflict_alone_prints_no_conflict_lines() {
    let mut stopped = data(ItemDtoKind::Ticket);
    stopped.blockers = vec![LifecycleBlocker::ConflictPending];
    let out = output(
        &answer(
            "ticket close",
            ResultCode::RecoveryRequired,
            Some(stopped),
            vec![retry(), inspect()],
        ),
        false,
    );
    let text = stderr(&out);
    assert!(!text.contains("conflict show"), "{text}");
    assert!(!text.contains("conflict resolve"), "{text}");
    assert!(!text.contains("context synchronization stopped"), "{text}");
    assert_eq!(next_lines(text).len(), 2, "{text}");
}

/// The note goes with the conflict lines even when no action follows them.
#[test]
fn a_child_s_conflict_with_no_action_still_prints_its_note() {
    let out = output(
        &answer(
            "ticket close",
            ResultCode::RecoveryRequired,
            Some(child_conflict(ItemDtoKind::Ticket)),
            vec![inspect()],
        ),
        false,
    );
    let text = stderr(&out);
    assert_eq!(next_lines(text).len(), 3, "{text}");
    assert!(
        text.contains("note: this operation's context synchronization stopped on a conflict."),
        "{text}"
    );
    assert!(text.contains("a new prepare is needed"), "{text}");
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
    assert!(
        text.contains("Blockers: primary_dirty, context_dirty\n"),
        "{text}"
    );
    assert_eq!(text.matches("Blockers:").count(), 1, "{text}");
    assert!(!text.contains("context observe-local"), "{text}");
}

/// Primary's tracking ref missing, with or without a missing context, and
/// nothing else: the poll that reads it, then a new prepare. Primary ahead of
/// it: the poll, then `repo sync`, which publishes primary if it is still
/// ahead, then a new prepare.
#[test]
fn a_tracking_ref_behind_or_missing_names_the_poll() {
    use LifecycleBlocker::{ContextMissing, PrimaryAheadOfRemote, PrimaryMissing};
    for (blockers, sync) in [
        (vec![PrimaryAheadOfRemote], true),
        (vec![PrimaryMissing], false),
        (vec![ContextMissing, PrimaryMissing], false),
        (vec![ContextMissing, PrimaryAheadOfRemote], true),
    ] {
        let out = prepare(&blocked(
            "ticket close",
            ResultCode::RecoveryRequired,
            blockers.clone(),
            vec![inspect()],
        ));
        assert_eq!(out.status, 3);
        let names: Vec<&str> = blockers.iter().map(|blocker| blocker.as_str()).collect();
        let (lines, note) = if sync {
            (
                format!(
                    "next: manyhands-cli poll once --repo '{ROOT}' --request-id <new-request-id>\n\
                     next: manyhands-cli repo sync --repo '{ROOT}' --request-id <new-request-id>\n"
                ),
                "primary holds commits the remote's primary, as this repository last read it, lacks: a poll reads the remote again, and if primary is still ahead, repo sync publishes it; each <new-request-id> is a placeholder for a request of its own. Then prepare again.",
            )
        } else {
            (
                format!(
                    "next: manyhands-cli poll once --repo '{ROOT}' --request-id <new-request-id>\n"
                ),
                "this repository has no record of the remote's primary, and a poll reads it; <new-request-id> is a placeholder to fill in. Then prepare again.",
            )
        };
        assert_eq!(
            stderr(&out),
            format!(
                "error: Recovery is required before this can continue.\ncode: recovery_required\n\
                 Blockers: {}\n{lines}next: repo.inspect root={ROOT}\nnote: {note}\n",
                names.join(", ")
            )
        );
    }
    // With anything else in the way, the poll alone is no way out.
    let out = prepare(&blocked(
        "ticket close",
        ResultCode::RecoveryRequired,
        vec![PrimaryMissing, LifecycleBlocker::PrimaryDirty],
        vec![inspect()],
    ));
    assert!(!stderr(&out).contains("poll once"), "{}", stderr(&out));
}

/// Decision 33: a prepare an unfinished request holds names it as a command,
/// and one no request holds names the resume.
#[test]
fn a_held_prepare_names_the_request_or_the_resume() {
    let held = |recovery| {
        let mut envelope = Envelope::<PrepareDataDto>::failure(
            "ticket close",
            scope(true),
            ResultCode::RecoveryRequired,
            vec![recovery],
        );
        envelope.data = None;
        envelope
    };
    let out = prepare(&held(retry()));
    assert_eq!(out.status, 3);
    assert_eq!(
        stderr(&out),
        format!(
            "error: Recovery is required before this can continue.\ncode: recovery_required\n\
             next: manyhands-cli ticket close --repo '{ROOT}' --id {ITEM} --request-id {REQUEST}\n\
             note: an unfinished request holds this item; send it again to finish it, and prepare again once it has finished, or has stopped on a cleanup a new request can replace.\n"
        )
    );
    let resume = RecoveryAction::for_operation(RecoveryActionKind::OperationResume, OPERATION, []);
    let out = prepare(&held(resume));
    assert_eq!(
        next_lines(stderr(&out)),
        [format!(
            "next: manyhands-cli operation resume --repo '{ROOT}' --operation-id {OPERATION}"
        )]
    );
    assert!(
        stderr(&out).contains("resume it (add --request-id"),
        "{}",
        stderr(&out)
    );
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
            "note: this resume released the operation's context synchronization if nothing it holds could be lost. If repo inspect still shows its reservation, it was kept, and the republish answers busy. Add --request-id to the resume unless you run it at a terminal.\n"
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
