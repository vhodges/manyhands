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
        Some("synchronization_conflict") => Some(MutationDataDto::SynchronizationConflict(
            SynchronizationConflictMutationDto {
                r#type: SynchronizationConflictType::SynchronizationConflict,
                conflict: next_step(&value["conflict"]),
            },
        )),
        other => panic!("no reader for data {other:?}"),
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
    let resolution = output(&envelope, false, &Seen { supersedable: true });
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
