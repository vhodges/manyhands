use super::*;
use manyhands::results::{
    CheckpointEffect, CleanupEffect, DiscoveryEffect, Effects, IntegrationEffect, Outcome,
    PublicationEffect, RecoveryAction, Scope, WriteEffect,
};
use serde_json::Value;

/// A mutation golden of `tests/fixtures/mutation_v1`, as the envelope it
/// pins. Only the data shapes these tests render are read.
fn golden(name: &str) -> Envelope<MutationDataDto> {
    let path = format!(
        "{}/tests/fixtures/mutation_v1/{name}.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let value: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let text = |pointer: &str| {
        value
            .pointer(pointer)
            .and_then(Value::as_str)
            .map(str::to_owned)
    };
    fn pick<T: Copy, const N: usize>(all: [T; N], name: &str, text: fn(T) -> &'static str) -> T {
        all.into_iter()
            .find(|known| text(*known) == name)
            .unwrap_or_else(|| panic!("unknown value {name}"))
    }
    let effect = |key: &str| text(&format!("/effects/{key}")).unwrap();
    Envelope {
        schema_version: 1,
        command: text("/command").unwrap(),
        request_id: text("/request_id"),
        operation_id: text("/operation_id"),
        outcome: pick(Outcome::ALL, &text("/outcome").unwrap(), Outcome::as_str),
        code: pick(ResultCode::ALL, &text("/code").unwrap(), ResultCode::as_str),
        message: text("/message").unwrap(),
        scope: Scope {
            repository: text("/scope/repository"),
            item_id: text("/scope/item_id"),
            branch: text("/scope/branch"),
            worktree: text("/scope/worktree"),
            remote: text("/scope/remote"),
        },
        effects: Effects {
            write: pick(WriteEffect::ALL, &effect("write"), WriteEffect::as_str),
            checkpoint: pick(
                CheckpointEffect::ALL,
                &effect("checkpoint"),
                CheckpointEffect::as_str,
            ),
            discovery: pick(
                DiscoveryEffect::ALL,
                &effect("discovery"),
                DiscoveryEffect::as_str,
            ),
            publication: pick(
                PublicationEffect::ALL,
                &effect("publication"),
                PublicationEffect::as_str,
            ),
            integration: pick(
                IntegrationEffect::ALL,
                &effect("integration"),
                IntegrationEffect::as_str,
            ),
            cleanup: pick(
                CleanupEffect::ALL,
                &effect("cleanup"),
                CleanupEffect::as_str,
            ),
            commit_oid: text("/effects/commit_oid"),
        },
        data: data(&value["data"]),
        recovery: value["recovery"]
            .as_array()
            .unwrap()
            .iter()
            .map(|action| RecoveryAction {
                action: pick(
                    RecoveryActionKind::ALL,
                    action["action"].as_str().unwrap(),
                    RecoveryActionKind::as_str,
                ),
                operation_id: action["operation_id"].as_str().map(str::to_owned),
                arguments: action["arguments"].as_object().unwrap().clone(),
            })
            .collect(),
    }
}

fn data(value: &Value) -> Option<MutationDataDto> {
    match value["type"].as_str() {
        _ if value.is_null() => None,
        Some("local_context_sync") => Some(MutationDataDto::LocalContextSync(
            serde_json::from_value(value.clone()).unwrap(),
        )),
        Some("comment") => Some(MutationDataDto::Comment(comment(value))),
        Some("synchronization_conflict") => Some(MutationDataDto::SynchronizationConflict(
            SynchronizationConflictMutationDto {
                r#type: SynchronizationConflictType::SynchronizationConflict,
                conflict: next_step(&value["conflict"]),
            },
        )),
        other => panic!("no reader for data {other:?}"),
    }
}

/// The data of a `comment add` golden. `CommentMutationDto` is built by the
/// library only, so it has no reader of its own.
fn comment(value: &Value) -> CommentMutationDto {
    let text = |key: &str| value[key].as_str().map(str::to_owned);
    CommentMutationDto {
        r#type: CommentMutationType::Comment,
        item_id: text("item_id").unwrap(),
        comment_id: text("comment_id").unwrap(),
        parent_id: text("parent_id"),
        created_by: text("created_by"),
        synchronization_id: text("synchronization_id").unwrap(),
        publication: PublicationEffect::ALL
            .into_iter()
            .find(|known| Some(known.as_str()) == value["publication"].as_str())
            .unwrap(),
        pending_reason: value["pending_reason"].as_str().map(|name| {
            CommentPendingReason::ALL
                .into_iter()
                .find(|known| known.as_str() == name)
                .unwrap()
        }),
        conflict: (!value["conflict"].is_null()).then(|| next_step(&value["conflict"])),
    }
}

fn next_step(value: &Value) -> ConflictNextDto {
    ConflictNextDto {
        operation_id: value["operation_id"].as_str().unwrap().to_owned(),
        next_action: match value["next_action"].as_str().unwrap() {
            "resolve" => ConflictNextAction::Resolve,
            "external_repair" => ConflictNextAction::ExternalRepair,
            other => panic!("unknown next action {other}"),
        },
    }
}

fn stdout(output: &Output) -> &str {
    std::str::from_utf8(&output.stdout).unwrap()
}

fn stderr(output: &Output) -> &str {
    std::str::from_utf8(&output.stderr).unwrap()
}

/// No byte of `text` is a control other than a line's end.
fn no_raw_control(text: &str) {
    assert!(
        !text.chars().any(|c| c.is_control() && c != '\n'),
        "{text:?}"
    );
}

const R1: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX1";
const R2: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX2";

#[test]
fn a_local_sync_s_conflict_shows_its_state_reason_and_next_action() {
    let output = output(&golden("local_sync_partial"), false, &Seen::default());
    assert_eq!(output.status, 4);
    assert!(output.stdout.is_empty());
    assert_eq!(
        stderr(&output),
        "error: Recovery is required before this can continue.\n\
         code: recovery_required\n\
         request: 01ARZ3NDEKTSV4RRFFQ69G5FX2\n\
         operation: <operation>\n\
         effects: write=written checkpoint=pending discovery=pending integration=pending\n\
         state: conflict_pending\n\
         reason: canonical_conflict\n\
         next action: resolve\n\
         next: repo.inspect root=<root>\n"
    );
}

#[test]
fn a_local_sync_s_identity_stop_names_its_state_and_c2_s_identity_action() {
    let output = output(
        &golden("local_sync_identity_required"),
        false,
        &Seen::default(),
    );
    assert_eq!(output.status, 3);
    let text = stderr(&output);
    assert!(
        text.ends_with(
            "state: refused\nreason: identity_required\nnext action: set_identity\nnext: repo.identity_set root=<root>\n"
        ),
        "{text}"
    );
    // The gate's fields are not also listed one by one.
    assert!(!text.contains("Cancel acknowledged"), "{text}");
}

#[test]
fn a_local_sync_that_needed_nothing_is_a_success_with_its_state() {
    let output = output(&golden("local_sync_noop"), false, &Seen::default());
    assert_eq!(output.status, 0);
    assert!(output.stderr.is_empty());
    assert_eq!(
        stdout(&output),
        "Already applied; nothing changed.\n\
         request: 01ARZ3NDEKTSV4RRFFQ69G5FX2\n\
         operation: <operation>\n\
         effects: write=unchanged checkpoint=unchanged discovery=current\n\
         state: complete\n\
         reason: none\n\
         next action: none\n"
    );
}

#[test]
fn a_local_sync_refused_with_no_data_is_c2_s_refusal() {
    let envelope = golden("local_sync_refused");
    assert_eq!(
        output(&envelope, false, &Seen::default()).stderr,
        crate::render::mutation_output(false, &envelope, false).stderr
    );
}

#[test]
fn a_local_sync_that_needs_its_original_request_names_the_required_request() {
    let mut envelope = golden("local_sync_partial");
    envelope.command = "operation resume".into();
    envelope.code = ResultCode::OriginalRequestRequired;
    envelope.outcome = Outcome::Blocked;
    envelope.message = ResultCode::OriginalRequestRequired.message().into();
    envelope.recovery = vec![RecoveryAction::new(
        RecoveryActionKind::RequestRetry,
        [("request_id", R1.into())],
    )];
    let Some(MutationDataDto::LocalContextSync(data)) = envelope.data.as_mut() else {
        panic!("local sync data")
    };
    data.state = manyhands::repository::LocalContextSyncState::ResolutionPrepared;
    data.reason = manyhands::repository::LocalContextSyncReason::OriginalResolutionRequired;
    data.next_action = manyhands::repository::LocalContextSyncNextAction::RetryResolution;
    data.required_request_id = Some(R1.into());
    data.resolution_request_id = Some(R1.into());
    let output = output(&envelope, false, &Seen::default());
    assert_eq!(output.status, 3);
    let text = stderr(&output);
    assert!(
        text.ends_with(
            "state: resolution_prepared\n\
             reason: original_resolution_required\n\
             next action: retry_resolution\n\
             next: send request 01ARZ3NDEKTSV4RRFFQ69G5FX1 again with its original command and input\n"
        ),
        "{text}"
    );
    // The request is named once, as a request, and no local verb is named.
    assert_eq!(text.matches("next:").count(), 1, "{text}");
    assert!(!text.contains("request.retry"), "{text}");
    assert!(!text.contains("supersede"), "{text}");
}

#[test]
fn a_deleted_remote_branch_names_cancel_the_same_request_and_republish() {
    let mut envelope = golden("item_sync_remote_branch_deleted");
    envelope.scope.repository = Some("/work/repo".into());
    let output = output(&envelope, false, &Seen::default());
    assert_eq!(output.status, 3);
    assert_eq!(
        stderr(&output),
        "error: The published branch was deleted on the remote.\n\
         code: remote_branch_deleted\n\
         request: 01ARZ3NDEKTSV4RRFFQ69G5FX1\n\
         operation: <operation>\n\
         effects: publication=pending\n\
         next: manyhands-cli request cancel --request-id 01ARZ3NDEKTSV4RRFFQ69G5FX1\n\
         next: manyhands-cli item sync --repo '/work/repo' --id 01ARZ3NDEKTSV4RRFFQ69G5FAW --request-id 01ARZ3NDEKTSV4RRFFQ69G5FX1\n\
         next: manyhands-cli item republish --repo '/work/repo' --id 01ARZ3NDEKTSV4RRFFQ69G5FAW --prepare\n\
         note: the refused request keeps its reservation until it is cancelled; sending it again then ends it and releases the reservation.\n"
    );
}

#[test]
fn another_request_s_deleted_branch_is_sent_again_with_its_own_command() {
    // As conflict resolve meets it: the request holding the reservation is
    // another one, whose command this result cannot name.
    let lines = branch_deleted(Some("/r"), Some("01ARZ3NDEKTSV4RRFFQ69G5FAW"), R1, None);
    assert_eq!(
        lines,
        [
            "manyhands-cli request cancel --request-id 01ARZ3NDEKTSV4RRFFQ69G5FX1",
            "send request 01ARZ3NDEKTSV4RRFFQ69G5FX1 again with its original command and input",
            "manyhands-cli item republish --repo '/r' --id 01ARZ3NDEKTSV4RRFFQ69G5FAW --prepare",
        ]
    );
    assert_eq!(branch_deleted(None, None, R1, None).len(), 2);
}

#[test]
fn a_conflict_prints_its_conflict_and_the_command_that_shows_it() {
    let mut envelope = golden("repo_sync_recovery_required");
    envelope.scope.repository = Some("/work/repo".into());
    let output = output(&envelope, false, &Seen::default());
    assert_eq!(output.status, 3);
    assert_eq!(
        stderr(&output),
        "error: Recovery is required before this can continue.\n\
         code: recovery_required\n\
         request: 01ARZ3NDEKTSV4RRFFQ69G5FX1\n\
         operation: <operation>\n\
         effects: publication=pending integration=pending\n\
         conflict: operation <operation> stopped on a conflict; next action: resolve\n\
         next: manyhands-cli conflict show --repo '/work/repo' --operation-id <operation>\n\
         next: request.retry request_id=01ARZ3NDEKTSV4RRFFQ69G5FX1\n\
         next: repo.inspect root=<root>\n"
    );
}

#[test]
fn a_conflict_to_repair_outside_says_so() {
    let mut envelope = golden("repo_sync_recovery_required");
    let Some(MutationDataDto::SynchronizationConflict(data)) = envelope.data.as_mut() else {
        panic!("conflict data")
    };
    data.conflict.next_action = ConflictNextAction::ExternalRepair;
    let text = stderr(&output(&envelope, false, &Seen::default())).to_owned();
    assert!(
        text.contains("stopped on a conflict; next action: external_repair\n"),
        "{text}"
    );
    assert!(text.contains("next: manyhands-cli conflict show --repo '<root>'"));
}

#[test]
fn a_resume_of_a_conflict_stopped_synchronization_prints_the_same_conflict_lines() {
    let sync = golden("repo_sync_recovery_required");
    let mut resume = sync.clone();
    resume.command = "operation resume".into();
    resume.request_id = Some(R2.into());
    resume.recovery.clear();
    let text = stderr(&output(&resume, false, &Seen::default())).to_owned();
    let conflict = |text: &str| -> Vec<String> {
        text.lines()
            .filter(|line| line.starts_with("conflict: ") || line.contains("conflict show"))
            .map(str::to_owned)
            .collect()
    };
    assert_eq!(conflict(&text).len(), 2, "{text}");
    assert_eq!(
        conflict(&text),
        conflict(stderr(&output(&sync, false, &Seen::default())))
    );
}

#[test]
fn an_original_request_is_named_and_a_written_resolution_may_be_superseded() {
    let envelope = golden("operation_resume_original_request_required");
    let plain = output(&envelope, false, &Seen::default());
    assert_eq!(plain.status, 3);
    assert_eq!(
        stderr(&plain),
        "error: The original request must be submitted again to finish the operation.\n\
         code: original_request_required\n\
         request: 01ARZ3NDEKTSV4RRFFQ69G5FX2\n\
         next: send request 01ARZ3NDEKTSV4RRFFQ69G5FX1 again with its original command and input\n"
    );
    let resolution = output(
        &envelope,
        false,
        &Seen {
            supersedable: true,
            ..Seen::default()
        },
    );
    assert_eq!(
        stderr(&resolution),
        format!(
            "{}note: a new conflict resolve may supersede its resolution while the paths it wrote are unchanged.\n",
            stderr(&plain)
        )
    );
}

#[test]
fn roots_in_next_lines_are_quoted_and_escaped_and_no_control_byte_is_printed() {
    assert_eq!(quoted("/plain root"), "'/plain root'");
    assert_eq!(quoted("/it's"), r"'/it'\''s'");
    let hostile = "/a\nnext: forged\r\u{1b}[31m'b";
    assert_eq!(quoted(hostile), r"'/a\nnext: forged\r\u{1b}[31m'\''b'");
    let mut envelope = golden("item_sync_remote_branch_deleted");
    envelope.scope.repository = Some(hostile.into());
    let mut conflict = golden("repo_sync_recovery_required");
    conflict.scope.repository = Some(hostile.into());
    for envelope in [envelope, conflict] {
        let text = stderr(&output(&envelope, false, &Seen::default())).to_owned();
        no_raw_control(&text);
        assert!(!text.contains("\nnext: forged"), "{text}");
    }
}

#[test]
fn a_result_without_c4_lines_is_c2_s_output() {
    for name in [
        "operation_resume",
        "operation_resume_noop",
        "operation_resume_not_found",
        "repo_sync_publication_remote_required",
        "item_sync_publication_remote_required",
    ] {
        let envelope = golden(name);
        let ours = output(&envelope, false, &Seen::default());
        let c2 = crate::render::mutation_output(false, &envelope, false);
        assert_eq!(
            (ours.stdout, ours.stderr, ours.status),
            (c2.stdout, c2.stderr, c2.status),
            "{name}"
        );
    }
}

const PUBLICATION_NOTE: &str = "note: publishing the comment also publishes this item's other checkpointed work; unsaved drafts are not included.\n";

#[test]
fn a_comment_s_result_carries_the_publication_note_and_c2_s_lines() {
    for (name, status) in [("comment_add", 0), ("comment_add_push_rejected", 4)] {
        let envelope = golden(name);
        let output = output(&envelope, false, &Seen::default());
        let c2 = crate::render::mutation_output(false, &envelope, false);
        assert_eq!(output.status, status, "{name}");
        assert_eq!(output.stdout, c2.stdout, "{name}");
        // The note follows whatever C2 prints on stderr, here the refusal.
        assert_eq!(
            stderr(&output),
            format!("{}{PUBLICATION_NOTE}", stderr(&c2)),
            "{name}"
        );
    }
}

#[test]
fn a_comment_refused_before_it_was_checkpointed_has_no_publication_note() {
    // No comment data: nothing was checkpointed, so nothing is published.
    let envelope = golden("comment_add_ticket_closed");
    assert!(envelope.data.is_none());
    let output = output(&envelope, false, &Seen::default());
    let c2 = crate::render::mutation_output(false, &envelope, false);
    assert_eq!(
        (output.stdout, output.stderr, output.status),
        (c2.stdout, c2.stderr, c2.status)
    );
}

#[test]
fn a_comment_s_conflict_prints_its_conflict_lines_and_the_note() {
    let mut envelope = golden("comment_add_push_rejected");
    envelope.scope.repository = Some("/work/repo".into());
    envelope.code = ResultCode::RecoveryRequired;
    envelope.message = ResultCode::RecoveryRequired.message().into();
    let Some(MutationDataDto::Comment(data)) = envelope.data.as_mut() else {
        panic!("comment data")
    };
    data.conflict = Some(ConflictNextDto {
        operation_id: "<operation>".into(),
        next_action: ConflictNextAction::Resolve,
    });
    let text = stderr(&output(&envelope, false, &Seen::default())).to_owned();
    assert!(
        text.contains(
            "conflict: operation <operation> stopped on a conflict; next action: resolve\n\
             next: manyhands-cli conflict show --repo '/work/repo' --operation-id <operation>\n"
        ),
        "{text}"
    );
    assert!(text.ends_with(PUBLICATION_NOTE), "{text}");
}

/// Task 2b's ruling: a comment whose child synchronization meets a deleted
/// branch keeps its record accepted and offers the request's cancellation.
/// Only a remote stages it, so Task 12a covers it end to end.
#[test]
fn a_comment_s_deleted_remote_branch_names_cancel_the_request_sent_again_and_republish() {
    let mut envelope = golden("comment_add_push_rejected");
    envelope.scope.repository = Some("/work/repo".into());
    envelope.code = ResultCode::RemoteBranchDeleted;
    envelope.message = ResultCode::RemoteBranchDeleted.message().into();
    envelope.recovery = vec![RecoveryAction::new(
        RecoveryActionKind::RequestCancel,
        [("request_id", R1.into())],
    )];
    let output = output(&envelope, false, &Seen::default());
    assert_eq!(output.status, 4);
    assert!(output.stdout.is_empty());
    let text = stderr(&output);
    assert!(
        text.starts_with(
            "error: The published branch was deleted on the remote.\n\
             code: remote_branch_deleted\n\
             request: 01ARZ3NDEKTSV4RRFFQ69G5FX1\n"
        ),
        "{text}"
    );
    // The same three lines as `item sync`'s, in the same order. The input
    // is the caller's, so the second names the request and not a command.
    let next: Vec<_> = text.lines().filter(|l| l.starts_with("next: ")).collect();
    assert_eq!(
        next,
        [
            "next: manyhands-cli request cancel --request-id 01ARZ3NDEKTSV4RRFFQ69G5FX1",
            "next: send request 01ARZ3NDEKTSV4RRFFQ69G5FX1 again with its original command and input",
            "next: manyhands-cli item republish --repo '/work/repo' --id 01ARZ3NDEKTSV4RRFFQ69G5FAW --prepare",
        ],
        "{text}"
    );
    // C2's line for the cancellation is replaced, not repeated.
    assert!(!text.contains("request.cancel"), "{text}");
    assert!(
        text.ends_with(&format!("{PUBLICATION_NOTE}note: {RESERVATION_NOTE}\n")),
        "{text}"
    );
    // The body of the comment is in none of it: the data holds none.
    no_raw_control(text);
}

// `conflict resolve`, and `conflict show`.

const R3: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX3";
const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
const OPERATION: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAZ";
const OBSERVATION: &str =
    "conflict-v1:0000000000000000000000000000000000000000000000000000000000000000";
const TOKEN: &str =
    "conflict-path-v1:1111111111111111111111111111111111111111111111111111111111111111";
const HAND_EDIT_NOTE_LINE: &str = "note: a resolved file edited by hand must be restored to the request's resolution before the request is sent again.\n";

fn seen(f: impl FnOnce(&mut Seen)) -> Seen {
    let mut seen = Seen::default();
    f(&mut seen);
    seen
}

fn same_as_c2(envelope: &Envelope<MutationDataDto>, seen: &Seen) {
    let ours = output(envelope, false, seen);
    let c2 = crate::render::mutation_output(false, envelope, false);
    assert_eq!(
        (ours.stdout, ours.stderr, ours.status),
        (c2.stdout, c2.stderr, c2.status)
    );
}

/// Controller ruling 1: the escape of a hand-edited resolution is told by
/// the human output only. The JSON message stays the code's own.
#[test]
fn a_resolution_refused_after_a_hand_edit_says_to_restore_the_file() {
    let envelope = golden("conflict_resolve_external_change");
    // No written paths: C2's refusal, exactly.
    same_as_c2(&envelope, &Seen::default());

    let c2 = crate::render::mutation_output(false, &envelope, false);
    let written = output(&envelope, false, &seen(|seen| seen.written = true));
    assert_eq!(written.status, 3);
    assert_eq!(written.stdout, c2.stdout);
    assert_eq!(
        stderr(&written),
        format!("{}{HAND_EDIT_NOTE_LINE}", stderr(&c2))
    );
    no_raw_control(stderr(&written));

    // JSON mode is not touched: one envelope, the code's own message.
    let json = crate::render::mutation_output(true, &envelope, false);
    assert!(json.stderr.is_empty());
    let json: Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(json["message"], ResultCode::ExternalChange.message());
    assert!(!json.to_string().contains("restored"));
}

#[test]
fn the_hand_edit_note_follows_an_external_change_only() {
    // Any other code is C2's, even when the conflict read says written.
    let written = seen(|seen| seen.written = true);
    for name in [
        "conflict_resolve_invalid_input",
        "conflict_resolve_original_request_required",
    ] {
        let envelope = golden(name);
        let text = stderr(&output(&envelope, false, &written)).to_owned();
        assert!(!text.contains(HAND_EDIT_NOTE_LINE), "{name}: {text}");
    }
    same_as_c2(&golden("conflict_resolve_invalid_input"), &written);
    // And the same external change of another command, which reads no
    // conflict, never carries it.
    let mut envelope = golden("conflict_resolve_external_change");
    envelope.command = "item sync".into();
    assert!(!stderr(&output(&envelope, false, &Seen::default())).contains("edited by hand"));
}

/// Controller ruling 2: `conflict resolve` answers
/// `original_request_required` only when an unfinished attempt's written
/// paths changed, so Task 7's supersede note would mislead.
#[test]
fn a_resolve_that_needs_its_original_request_says_the_written_paths_were_edited() {
    let envelope = golden("conflict_resolve_original_request_required");
    let expected = "error: The original request must be submitted again to finish the operation.\n\
         code: original_request_required\n\
         request: 01ARZ3NDEKTSV4RRFFQ69G5FR2\n\
         next: send request 01ARZ3NDEKTSV4RRFFQ69G5FR1 again with its original command and input\n\
         note: the attempt's written paths were edited by hand. Restore each file the request wrote to the request's resolution; a file it has not written yet may keep its conflicted content. Then send that request again, or a new conflict resolve may supersede it.\n";
    let plain = output(&envelope, false, &Seen::default());
    assert_eq!(plain.status, 3);
    assert!(plain.stdout.is_empty());
    assert_eq!(stderr(&plain), expected);
    // Task 7's note is an `operation resume`'s. A resolve never carries it,
    // whatever was read.
    let read = output(&envelope, false, &seen(|seen| seen.supersedable = true));
    assert_eq!(stderr(&read), expected);
    assert!(!expected.contains(SUPERSEDE_NOTE));
    // The `request.retry` action is rendered once, as a request.
    assert!(!expected.contains("request.retry"));
}

#[test]
fn a_resumes_original_request_keeps_task_7_s_supersede_note() {
    let envelope = golden("operation_resume_original_request_required");
    let text = stderr(&output(&envelope, false, &Seen::default())).to_owned();
    assert!(!text.contains("note:"), "{text}");
    let text = stderr(&output(
        &envelope,
        false,
        &seen(|seen| seen.supersedable = true),
    ))
    .to_owned();
    assert!(
        text.ends_with(&format!("note: {SUPERSEDE_NOTE}\n")),
        "{text}"
    );
    assert!(!text.contains("were edited"), "{text}");
}

/// A `conflict resolve` that met the item's branch deleted on the remote:
/// the envelope is the resolve's own, and its `request.cancel` names the
/// request that holds the synchronization (Part A checkpoint ruling M9).
fn deleted_branch_of_a_resolve() -> Envelope<MutationDataDto> {
    let mut envelope = Envelope::<MutationDataDto>::failure(
        "conflict resolve",
        Scope {
            repository: Some("/work/repo".into()),
            ..Scope::default()
        },
        ResultCode::RemoteBranchDeleted,
        vec![RecoveryAction::new(
            RecoveryActionKind::RequestCancel,
            [("request_id", R1.into())],
        )],
    );
    envelope.request_id = Some(R3.into());
    envelope.effects.write = WriteEffect::Written;
    envelope.effects.checkpoint = CheckpointEffect::Committed;
    envelope.effects.commit_oid = Some("0123456789abcdef0123456789abcdef01234567".into());
    envelope
}

fn next_lines(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|line| line.starts_with("next: "))
        .collect()
}

/// Controller ruling 3: the three lines name the holder's request, taken
/// from the action, and never the resolve's own.
#[test]
fn a_resolve_that_met_a_deleted_branch_names_the_holder_s_request_in_three_lines() {
    let envelope = deleted_branch_of_a_resolve();
    let output = output(
        &envelope,
        false,
        &seen(|seen| seen.item = Some(ITEM.into())),
    );
    assert_eq!(
        output.status,
        crate::render::exit_status(envelope.failure_class())
    );
    assert!(output.stdout.is_empty());
    let text = stderr(&output);
    assert!(
        text.starts_with(&format!(
            "error: The published branch was deleted on the remote.\n\
             code: remote_branch_deleted\n\
             request: {R3}\n"
        )),
        "{text}"
    );
    assert_eq!(
        next_lines(text),
        [
            format!("next: manyhands-cli request cancel --request-id {R1}"),
            format!("next: send request {R1} again with its original command and input"),
            format!("next: manyhands-cli item republish --repo '/work/repo' --id {ITEM} --prepare"),
        ]
    );
    // The resolve's own request is named once, as the request refused, and
    // in none of the lines that tell what to do.
    assert_eq!(text.matches(R3).count(), 1, "{text}");
    // C2's line for the cancellation is replaced, not repeated.
    assert!(!text.contains("request.cancel"), "{text}");
    // The note names the request that holds the reservation, not "the
    // refused request", which holds none.
    assert!(
        text.ends_with(&format!(
            "note: request {R1} keeps its reservation until it is cancelled; sending it again then ends it and releases the reservation.\n"
        )),
        "{text}"
    );
    assert!(!text.contains(RESERVATION_NOTE), "{text}");
    no_raw_control(text);
}

#[test]
fn a_deleted_branch_whose_item_is_unknown_still_names_cancel_and_the_holder_s_request() {
    // The conflict read failed: two lines, as for a result with no item.
    let envelope = deleted_branch_of_a_resolve();
    let output = output(&envelope, false, &Seen::default());
    assert_eq!(
        next_lines(stderr(&output)),
        [
            format!("next: manyhands-cli request cancel --request-id {R1}"),
            format!("next: send request {R1} again with its original command and input"),
        ]
    );
}

#[test]
fn a_deleted_branch_of_the_refused_request_s_own_reservation_keeps_its_note() {
    // As `item sync` and `comment add` meet it: the refused request is the
    // holder.
    let mut envelope = deleted_branch_of_a_resolve();
    envelope.request_id = Some(R1.into());
    let text = stderr(&output(&envelope, false, &Seen::default())).to_owned();
    assert!(
        text.ends_with(&format!("note: {RESERVATION_NOTE}\n")),
        "{text}"
    );
}

#[test]
fn a_resolve_the_library_sends_to_outside_repair_prints_the_conflict_lines() {
    // `recovery_required` with `next_action: external_repair` (design,
    // Conflict Resolution, step 4): the data carries the next step.
    let mut envelope = golden("conflict_resolve_invalid_input");
    envelope.scope.repository = Some("/work/repo".into());
    envelope.code = ResultCode::RecoveryRequired;
    envelope.message = ResultCode::RecoveryRequired.message().into();
    envelope.data = Some(MutationDataDto::ConflictResolution(
        ConflictResolutionMutationDto {
            r#type: ConflictResolutionMutationType::ConflictResolution,
            operation_id: OPERATION.into(),
            commit_oid: None,
            publication: PublicationEffect::NotRequested,
            conflict: Some(ConflictNextDto {
                operation_id: OPERATION.into(),
                next_action: ConflictNextAction::ExternalRepair,
            }),
        },
    ));
    let text = stderr(&output(&envelope, false, &Seen::default())).to_owned();
    assert!(
        text.contains(&format!(
            "conflict: operation {OPERATION} stopped on a conflict; next action: external_repair\n\
             next: manyhands-cli conflict show --repo '/work/repo' --operation-id {OPERATION}\n"
        )),
        "{text}"
    );
}

fn path(display: &str) -> ConflictPathDto {
    ConflictPathDto {
        display_path: display.into(),
        token: TOKEN.into(),
        kind: ConflictPathKind::Ticket,
        structure: ConflictPathStructure::Regular,
        content_available: true,
        base: Some("BASE-SIDE-SENTINEL".into()),
        local: Some("LOCAL-SIDE-SENTINEL\u{1b}[2J".into()),
        incoming: Some("INCOMING-SIDE-SENTINEL".into()),
    }
}

fn conflict(paths: Vec<ConflictPathDto>) -> ConflictDto {
    ConflictDto {
        operation_id: OPERATION.into(),
        comment_operation_id: None,
        target: ConflictTarget::Ticket,
        item_id: Some(ITEM.into()),
        stage: ConflictStage::Context,
        state: ConflictState::ConflictPending,
        next_action: ConflictReadNextAction::Resolve,
        request_id: None,
        eligibility: Some(ConflictReadEligibility::EligibleCanonical),
        local_parent: "a".repeat(40),
        incoming_parent: "b".repeat(40),
        observation: Some(OBSERVATION.into()),
        historical: false,
        paths,
    }
}

fn shown(conflict: &ConflictDto) -> Output {
    let envelope = Envelope::read_success(
        "conflict show",
        Scope {
            repository: Some("/work/repo".into()),
            ..Scope::default()
        },
        conflict.clone(),
    );
    crate::render::envelope_output(false, &envelope, conflict_show)
}

const SHOWN: &str = "operation: 01ARZ3NDEKTSV4RRFFQ69G5FAZ\n\
     target: ticket\n\
     item: 01ARZ3NDEKTSV4RRFFQ69G5FAW\n\
     stage: context\n\
     state: conflict_pending\n\
     next action: resolve\n\
     observation: conflict-v1:0000000000000000000000000000000000000000000000000000000000000000\n";

#[test]
fn conflict_show_prints_the_state_the_next_step_and_each_path() {
    let output = shown(&conflict(vec![
        path(".manyhands/tickets/a.md"),
        ConflictPathDto {
            kind: ConflictPathKind::Noncanonical,
            structure: ConflictPathStructure::Binary,
            ..path("assets/logo.png")
        },
    ]));
    assert_eq!(output.status, 0);
    assert!(output.stderr.is_empty(), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        format!(
            "{SHOWN}\
             path: kind=ticket structure=regular token={TOKEN} display=.manyhands/tickets/a.md\n\
             path: kind=noncanonical structure=binary token={TOKEN} display=assets/logo.png\n"
        )
    );
}

#[test]
fn conflict_show_names_an_attempt_a_comment_and_a_conflict_with_no_live_paths() {
    let mut data = conflict(Vec::new());
    data.comment_operation_id = Some("01ARZ3NDEKTSV4RRFFQ69G5FB0".into());
    data.target = ConflictTarget::Primary;
    data.item_id = None;
    data.stage = ConflictStage::Primary;
    data.state = ConflictState::ResolutionPrepared;
    data.next_action = ConflictReadNextAction::RetryResolution;
    data.request_id = Some(R1.into());
    data.historical = true;
    let output = shown(&data);
    assert_eq!(
        stdout(&output),
        format!(
            "operation: {OPERATION}\n\
             comment operation: 01ARZ3NDEKTSV4RRFFQ69G5FB0\n\
             target: primary\n\
             stage: primary\n\
             state: resolution_prepared\n\
             next action: retry_resolution\n\
             request: {R1}\n\
             observation: {OBSERVATION}\n\
             historical: yes\n"
        )
    );
    // A conflict no longer live has no observation and no path.
    data.observation = None;
    data.state = ConflictState::Complete;
    data.next_action = ConflictReadNextAction::None;
    data.request_id = None;
    data.historical = false;
    let text = stdout(&shown(&data)).to_owned();
    assert!(text.contains("observation: -\n"), "{text}");
    assert!(!text.contains("path:"), "{text}");
}

/// Controller ruling 4, and the design's Human Output: a side is printed
/// only with `--json`, so a terminal is never repainted by content.
#[test]
fn conflict_show_prints_no_side_in_text_and_the_sides_in_json() {
    let data = conflict(vec![path(".manyhands/tickets/a.md")]);
    let output = shown(&data);
    let text = format!("{}{}", stdout(&output), stderr(&output));
    for side in ["BASE-SIDE", "LOCAL-SIDE", "INCOMING-SIDE"] {
        assert!(!text.contains(side), "{text}");
    }
    no_raw_control(&text);

    let envelope = Envelope::read_success("conflict show", Scope::default(), data);
    let json = crate::render::envelope_output(true, &envelope, conflict_show);
    let json: Value = serde_json::from_slice(&json.stdout).unwrap();
    let path = &json["data"]["paths"][0];
    assert_eq!(path["base"], "BASE-SIDE-SENTINEL");
    assert_eq!(path["local"], "LOCAL-SIDE-SENTINEL\u{1b}[2J");
    assert_eq!(path["incoming"], "INCOMING-SIDE-SENTINEL");
}

/// Review Focus 2: the library has escaped the display path (R-11), so a
/// path holding an escape sequence and a newline arrives as text. It is
/// printed once, not escaped again.
#[test]
fn a_display_path_the_library_escaped_is_printed_as_it_is() {
    // The bytes `d/a<ESC>[31m<LF>b`, as `display_path` renders them.
    let display = r"d/a\x1b[31m\x0ab";
    let output = shown(&conflict(vec![path(display)]));
    let text = stdout(&output);
    no_raw_control(text);
    assert!(
        text.ends_with(&format!(
            "path: kind=ticket structure=regular token={TOKEN} display=d/a\\x1b[31m\\x0ab\n"
        )),
        "{text}"
    );
    // Not doubled: the backslash of `\x1b` is one character.
    assert!(!text.contains(r"\\x1b"), "{text}");
    assert!(!text.contains(r"\\x0a"), "{text}");
    assert_eq!(text.lines().count(), SHOWN.lines().count() + 1);

    // A library that let a raw control through is not repainted either.
    let raw = shown(&conflict(vec![path("d/a\u{1b}[31m\nnext: forged\nb")]));
    let text = stdout(&raw);
    no_raw_control(text);
    assert_eq!(text.lines().count(), SHOWN.lines().count() + 1, "{text}");
    assert!(!text.contains("\nnext: forged"), "{text}");
    assert!(text.contains(r"d/a\u{1b}[31m\nnext: forged\nb"), "{text}");
}

#[test]
fn a_path_that_looks_like_a_field_cannot_forge_one() {
    // The path is last on its line, after the fields it could imitate.
    let display = "x display=y token=conflict-path-v1:forged structure=regular";
    let text = stdout(&shown(&conflict(vec![path(display)]))).to_owned();
    let line = text.lines().last().unwrap();
    assert!(
        line.starts_with(&format!(
            "path: kind=ticket structure=regular token={TOKEN} display=x display=y"
        )),
        "{line}"
    );
}

/// The design's note, once, whatever the number of such paths.
#[test]
fn an_incompatible_closures_path_says_to_repair_it_outside_manyhands() {
    const CLOSURES: &str =
        "note: two clones closed this ticket differently; repair it outside Manyhands.\n";
    let closed = |display: &str| ConflictPathDto {
        structure: ConflictPathStructure::IncompatibleClosures,
        ..path(display)
    };
    let mut data = conflict(vec![closed(".manyhands/tickets/a.md")]);
    data.next_action = ConflictReadNextAction::ExternalRepair;
    data.eligibility = Some(ConflictReadEligibility::ExternalResolutionRequired);
    let output = shown(&data);
    assert_eq!(stderr(&output), CLOSURES);
    assert!(
        stdout(&output).ends_with(&format!(
            "path: kind=ticket structure=incompatible_closures token={TOKEN} display=.manyhands/tickets/a.md\n"
        ))
    );
    data.paths.push(closed(".manyhands/tickets/b.md"));
    assert_eq!(stderr(&shown(&data)), CLOSURES);
    // Another structure is no such note.
    let plain = shown(&conflict(vec![
        path("a"),
        ConflictPathDto {
            structure: ConflictPathStructure::Delete,
            ..path("b")
        },
    ]));
    assert!(plain.stderr.is_empty());
}
