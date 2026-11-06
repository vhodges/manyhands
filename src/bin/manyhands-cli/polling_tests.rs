//! The human lines of the polling verbs, on built envelopes.

use super::*;
use crate::render::test_support::{no_raw_control, stderr, stdout};
use manyhands::{
    repository::{
        ItemDtoKind, PollMutationCompletion, PollPolicyMutationDto, PollTargetDisposition,
        PollTargetDto, RequestId,
    },
    results::{Effects, RecoveryAction, Scope},
};

const REQUEST: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX1";

fn policy_envelope(recovery_suspended: bool) -> Envelope<MutationDataDto> {
    let mut envelope = Envelope::classified_mutation(
        "poll resume",
        Scope::default(),
        false,
        ResultCode::Ok,
        Effects::not_requested(),
        false,
    );
    envelope.request_id = Some(RequestId::parse(REQUEST).unwrap().to_string());
    envelope.data = Some(MutationDataDto::PollPolicy(PollPolicyMutationDto {
        enabled: true,
        paused: false,
        interval_seconds: 300,
        backoff_seconds: None,
        recovery_suspended,
        next_eligible_at: None,
    }));
    envelope
}

#[test]
fn a_suspension_is_named_from_the_answer_s_own_data() {
    let output = policy_output(&policy_envelope(true), false);
    assert_eq!(
        stderr(&output),
        format!("note: {SUSPENDED_NOTE}\n"),
        "no read but the envelope built here"
    );
    assert!(stdout(&output).contains("Recovery suspended: true\n"));
}

#[test]
fn no_suspension_has_no_note() {
    let output = policy_output(&policy_envelope(false), false);
    assert!(stderr(&output).is_empty());
}

#[test]
fn json_mode_holds_one_envelope_and_no_note() {
    let output = crate::mutate::finish(
        true,
        crate::mutate::Driven::Executed {
            envelope: Box::new(policy_envelope(true)),
            supplied: false,
        },
        |_| unreachable!("no prepare"),
        policy_output,
    );
    assert!(stderr(&output).is_empty());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["data"]["recovery_suspended"], true);
}

const OID: &str = "0123456789012345678901234567890123456789";
const ROOT: &str = "/repo";

fn poll_envelope(code: ResultCode, targets: Vec<PollTargetDto>) -> Envelope<MutationDataDto> {
    let mut envelope = Envelope::classified_mutation(
        "poll once",
        Scope {
            repository: Some(ROOT.into()),
            ..Scope::default()
        },
        code == ResultCode::Cancelled,
        code,
        Effects::not_requested(),
        false,
    );
    envelope.request_id = Some(REQUEST.into());
    envelope.data = Some(MutationDataDto::Poll(PollMutationDto {
        operation_id: REQUEST.into(),
        completion: PollMutationCompletion::Completed,
        completed_at: "2026-11-01T00:00:00Z".into(),
        next_eligible_at: None,
        targets,
    }));
    envelope
}

fn malformed() -> PollTargetDto {
    PollTargetDto {
        target: None,
        kind: None,
        item_id: None,
        disposition: PollTargetDisposition::Malformed,
        before_oid: None,
        after_oid: None,
    }
}

fn retry() -> RecoveryAction {
    RecoveryAction::new(
        RecoveryActionKind::RequestRetry,
        [("request_id", serde_json::Value::from(REQUEST))],
    )
}

/// Review Focus 7: nullable fields print as `none`, with the domain's guidance.
#[test]
fn a_malformed_target_is_its_disposition_and_guidance_only() {
    let output = once_output(
        &poll_envelope(ResultCode::Ok, vec![malformed()]),
        false,
        ROOT,
    );
    assert_eq!(output.status, 0);
    assert_eq!(
        stdout(&output).lines().rev().take(2).collect::<Vec<_>>(),
        [
            "  Repair the remote context's branch name.",
            "target: none kind=none item=none disposition=malformed before=none after=none",
        ]
    );
    assert!(stderr(&output).is_empty());
}

#[test]
fn what_a_target_is_named_by_is_escaped() {
    let target = PollTargetDto {
        target: Some("refs/heads/a\u{1b}[31m\nb".into()),
        kind: Some(ItemDtoKind::Ticket),
        item_id: Some("item\t".into()),
        disposition: PollTargetDisposition::Dirty,
        before_oid: Some(OID.into()),
        after_oid: Some(OID.into()),
    };
    let output = once_output(&poll_envelope(ResultCode::Ok, vec![target]), false, ROOT);
    let text = stdout(&output);
    no_raw_control(text);
    assert!(text.contains(&format!(
        "target: refs/heads/a\\u{{1b}}[31m\\nb kind=ticket item=item\\t disposition=dirty before={OID} after={OID}\n"
    )), "{text}");
    assert!(text.contains("  Save or otherwise handle local changes before polling again.\n"));
}

#[test]
fn every_disposition_prints_the_domain_s_guidance() {
    for disposition in PollTargetDisposition::ALL {
        let target = PollTargetDto {
            disposition,
            ..malformed()
        };
        let output = once_output(&poll_envelope(ResultCode::Ok, vec![target]), false, ROOT);
        assert!(
            stdout(&output).contains(&format!("  {}\n", disposition.guidance())),
            "{disposition:?}"
        );
    }
}

#[test]
fn no_target_says_so() {
    let output = once_output(&poll_envelope(ResultCode::Ok, Vec::new()), false, ROOT);
    assert!(stdout(&output).contains("targets: none\n"));
}

/// Every stop the table calls final takes a new request, and doesn't offer this one.
#[test]
fn a_final_stop_says_a_new_request_and_never_this_one() {
    for code in [
        ResultCode::HostApprovalRequired,
        ResultCode::UnlockRequired,
        ResultCode::UnlockFailed,
        ResultCode::NoSelectedKey,
        ResultCode::KeyRejected,
        ResultCode::KeyUnavailable,
        ResultCode::TransportUnavailable,
        ResultCode::RemoteUnavailable,
        ResultCode::RepositoryInaccessible,
        ResultCode::PublicationRemoteRequired,
        ResultCode::InvalidRemote,
        ResultCode::ExternalChange,
        ResultCode::Cancelled,
        ResultCode::HostReplacementRequired,
        ResultCode::RecoveryRequired,
        ResultCode::IndexUnavailable,
        ResultCode::InternalError,
    ] {
        let mut envelope = poll_envelope(code, Vec::new());
        envelope.data = None;
        envelope.recovery.clear();
        let output = once_output(&envelope, false, ROOT);
        let said = stderr(&output);
        assert!(said.contains("a new request"), "{code:?}: {said}");
        assert!(!said.contains("same request"), "{code:?}: {said}");
        assert!(!said.contains("again\n"), "{code:?}: {said}");
    }
}

#[test]
fn the_unlock_and_host_stops_say_what_comes_first() {
    let mut envelope = poll_envelope(ResultCode::UnlockRequired, Vec::new());
    envelope.recovery.clear();
    let said = once_output(&envelope, false, ROOT);
    assert!(stderr(&said).contains("at a terminal"));
    for code in [
        ResultCode::HostApprovalRequired,
        ResultCode::HostReplacementRequired,
    ] {
        envelope.code = code;
        let said = once_output(&envelope, false, ROOT);
        assert!(
            stderr(&said).contains("once the host is approved or replaced"),
            "{code:?}"
        );
    }
}

/// A recovery the same request or an inspection gets past isn't final.
#[test]
fn a_recovery_with_a_way_on_has_no_final_note() {
    for kind in [
        RecoveryActionKind::RepoInspect,
        RecoveryActionKind::RequestRetry,
    ] {
        for code in [
            ResultCode::RecoveryRequired,
            ResultCode::IndexUnavailable,
            ResultCode::InternalError,
        ] {
            let mut envelope = poll_envelope(code, Vec::new());
            envelope.data = None;
            envelope.recovery = vec![match kind {
                RecoveryActionKind::RequestRetry => retry(),
                _ => RecoveryAction::new(kind, [("root", serde_json::Value::from(ROOT))]),
            }];
            let said = once_output(&envelope, false, ROOT);
            assert!(!stderr(&said).contains("new request"), "{code:?} {kind:?}");
        }
    }
}

/// A wrong passphrase: C2's "run the command again" would replay this request.
#[test]
fn a_wrong_passphrase_s_hint_is_a_new_request() {
    let mut envelope = poll_envelope(ResultCode::TransportUnavailable, Vec::new());
    envelope.recovery.clear();
    let said = once_output(&envelope, true, ROOT);
    assert!(stderr(&said).contains(PASSPHRASE_HINT), "{}", stderr(&said));
    assert!(!stderr(&said).contains("run the command again"));
    let said = once_output(&envelope, false, ROOT);
    assert!(!stderr(&said).contains("hint:"), "{}", stderr(&said));
}

/// A stop that offers the same request again isn't final, and says nothing of
/// a new one.
#[test]
fn a_stop_the_same_request_continues_has_no_final_note() {
    let mut envelope = poll_envelope(ResultCode::RecoveryRequired, Vec::new());
    envelope.recovery = vec![retry()];
    let output = once_output(&envelope, false, ROOT);
    assert!(
        !stderr(&output).contains("new request"),
        "{}",
        stderr(&output)
    );
    assert!(stderr(&output).contains("next: request.retry"));
}

#[test]
fn busy_hints_operation_list_and_then_operation_resume() {
    let mut envelope = poll_envelope(ResultCode::Busy, Vec::new());
    envelope.data = None;
    envelope.recovery = vec![retry()];
    let output = once_output(&envelope, false, ROOT);
    let said = stderr(&output);
    let retried = said.find("next: request.retry").unwrap();
    let list = said
        .find("next: manyhands-cli operation list --repo '/repo'\n")
        .unwrap();
    let resume = said
        .find("next: manyhands-cli operation resume --repo '/repo' --operation-id <operation-id>\n")
        .unwrap();
    assert!(retried < list && list < resume, "{said}");
    assert!(!said.contains("new request"), "{said}");
    assert!(said.contains(BUSY_NOTE));
    assert!(
        said.contains("If none is another request's, wait and send this request again (add --request-id unless you run it at a terminal)."),
        "{said}"
    );
    // The holder may be a lease outside any operation, which the list
    // doesn't show (Task 13b's walk).
    assert!(said.contains("it may be no operation"), "{said}");
    assert!(!said.contains("operation list names it"), "{said}");
}

#[test]
fn busy_with_no_scope_names_the_root_the_command_named() {
    let mut envelope = poll_envelope(ResultCode::Busy, Vec::new());
    envelope.scope = Scope::default();
    envelope.data = None;
    envelope.recovery = vec![retry()];
    let output = once_output(&envelope, false, "/named it's");
    assert!(
        stderr(&output).contains(r"--repo '/named it'\''s'"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_stop_with_a_report_prints_its_targets_on_stderr() {
    let mut envelope = poll_envelope(ResultCode::DiscoveryPending, vec![malformed()]);
    envelope.recovery.clear();
    let output = once_output(&envelope, false, ROOT);
    assert!(output.stdout.is_empty());
    assert!(stderr(&output).contains("disposition=malformed"));
    assert_eq!(output.status, 4);
}

#[test]
fn retirement_report_distinguishes_as_of_cut_from_current_child_actions() {
    use manyhands::repository::{
        ContextRetirementMutationDto, ContextRetirementMutationType, ContextRetirementState,
        PollRetirementCompletion, PollRetirementDebt, PollRetirementItemDto,
        PollRetirementItemState, PollRetirementMutationDto, PollRetirementMutationType,
        RetirementRemoval,
    };
    let mut e = poll_envelope(ResultCode::RecoveryRequired, vec![]);
    const PARENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FP0";
    const HISTORICAL_CHILD: &str = "01ARZ3NDEKTSV4RRFFQ69G5FK0";
    const CURRENT_OPERATION: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX0";
    const CURRENT_REQUEST: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR0";
    const SCOPE_ROOT: &str = "/scope's\troot\n";
    const ACTION_ROOT: &str = "/action's\troot\n";
    e.operation_id = Some(PARENT.into());
    e.scope.repository = Some(SCOPE_ROOT.into());
    e.recovery = vec![
        RecoveryAction::for_operation(RecoveryActionKind::OperationResume, CURRENT_OPERATION, []),
        RecoveryAction::for_operation(
            RecoveryActionKind::OperationAbandon,
            CURRENT_OPERATION,
            [("root", serde_json::Value::from(ACTION_ROOT))],
        ),
        RecoveryAction::new(
            RecoveryActionKind::RequestRetry,
            [("request_id", serde_json::Value::from(CURRENT_REQUEST))],
        ),
    ];
    let Some(MutationDataDto::Poll(poll)) = e.data.take() else {
        unreachable!()
    };
    let poll = PollMutationDto {
        operation_id: PARENT.into(),
        ..poll
    };
    let states = [
        PollRetirementItemState::Retired,
        PollRetirementItemState::Refused,
        PollRetirementItemState::Pending,
        PollRetirementItemState::NotReached,
    ];
    e.data = Some(MutationDataDto::PollRetirement(PollRetirementMutationDto {
        r#type: PollRetirementMutationType::PollRetirement,
        poll,
        completion: PollRetirementCompletion::Stopped,
        stop_code: Some(ResultCode::RecoveryRequired),
        retirements: states
            .into_iter()
            .enumerate()
            .map(|(i, state)| PollRetirementItemDto {
                kind: ItemDtoKind::Ticket,
                item_id: format!("item-{i}\n"),
                request_id: REQUEST.into(),
                operation_id: Some(HISTORICAL_CHILD.into()),
                state,
                code: Some(ResultCode::RecoveryRequired),
                debt: (state == PollRetirementItemState::Pending)
                    .then_some(PollRetirementDebt::Discovery),
                result: Some(ContextRetirementMutationDto {
                    r#type: ContextRetirementMutationType::ContextRetirement,
                    kind: ItemDtoKind::Ticket,
                    item_id: format!("item-{i}"),
                    operation_id: HISTORICAL_CHILD.into(),
                    state: ContextRetirementState::Pending,
                    code: ResultCode::DiscoveryPending,
                    worktree_removal: RetirementRemoval::Observed,
                    branch_removal: RetirementRemoval::Possible,
                    materialization_settled: false,
                }),
            })
            .collect(),
    }));
    let before = serde_json::to_value(&e).unwrap();
    assert!(
        !serde_json::to_string(&e.data)
            .unwrap()
            .contains(CURRENT_OPERATION),
        "the current recovery target is absent from the historical report"
    );
    let out = once_output_in_mode(&e, false, ROOT, true);
    assert!(out.stdout.is_empty());
    let text = stderr(&out);
    for state in ["retired", "refused", "pending", "not_reached"] {
        assert!(text.contains(&format!("state={state}")), "{text}");
    }
    assert!(text.contains("as-of-pass"), "{text}");
    assert!(text.contains("current child"), "{text}");
    assert!(
        text.contains(
            "worktree_removal=observed branch_removal=possible materialization_settled=false"
        ),
        "{text}"
    );
    assert!(text.contains("debt=discovery"), "{text}");
    assert!(text.contains("never resumes the loop"), "{text}");
    assert!(
        text.contains(&format!(
            "next: manyhands-cli operation resume --repo {} --operation-id '{}'\n",
            quoted(SCOPE_ROOT),
            CURRENT_OPERATION
        )),
        "{text}"
    );
    assert!(
        text.contains(&format!(
            "next: manyhands-cli operation abandon --repo {} --operation-id '{}' --prepare\n",
            quoted(ACTION_ROOT),
            CURRENT_OPERATION
        )),
        "{text}"
    );
    assert!(
        text.contains(&format!(
            "next: request.retry request_id={CURRENT_REQUEST}\n"
        )),
        "{text}"
    );
    for line in text.lines().filter(|line| line.starts_with("next:")) {
        assert!(
            !line.contains(PARENT) && !line.contains(HISTORICAL_CHILD),
            "{line}"
        );
    }
    assert_eq!(
        serde_json::to_value(&e).unwrap(),
        before,
        "human formatting never changes JSON"
    );
    no_raw_control(text);

    // An opted-in refusal may precede a tagged report. The explicit mode still
    // renders only the current action's target; the invocation root is not a
    // substitute for a missing record/action root.
    let mut no_report = e.clone();
    no_report.data = None;
    let out = once_output_in_mode(&no_report, false, "/not-the-record-root", true);
    let text = stderr(&out);
    assert!(
        text.contains(&format!(
            "next: manyhands-cli operation resume --repo {} --operation-id '{}'\n",
            quoted(SCOPE_ROOT),
            CURRENT_OPERATION
        )),
        "{text}"
    );
    assert!(!text.contains("/not-the-record-root"));

    let mut ordinary = poll_envelope(ResultCode::RecoveryRequired, vec![]);
    ordinary.recovery = e.recovery.clone();
    let ordinary = once_output(&ordinary, false, ROOT);
    let text = stderr(&ordinary);
    assert!(
        text.contains("next: operation.resume\n"),
        "ordinary output is unchanged: {text}"
    );
    assert!(text.contains(&format!(
        "next: operation.abandon root={}\n",
        field(ACTION_ROOT)
    )));
    assert!(!text.contains("next: manyhands-cli operation resume"));

    // Completed mixed passes are exit 0, not a claim that every item retired.
    // Empty, cancelled and index-pending reports retain their actual code/exit.
    use manyhands::results::Outcome;
    for (code, outcome, status, completion, empty) in [
        (
            ResultCode::Ok,
            Outcome::Success,
            0,
            PollRetirementCompletion::Completed,
            false,
        ),
        (
            ResultCode::Ok,
            Outcome::Success,
            0,
            PollRetirementCompletion::Completed,
            true,
        ),
        (
            ResultCode::DiscoveryPending,
            Outcome::Partial,
            4,
            PollRetirementCompletion::Stopped,
            false,
        ),
        (
            ResultCode::Cancelled,
            Outcome::Cancelled,
            130,
            PollRetirementCompletion::Stopped,
            false,
        ),
    ] {
        let mut row = e.clone();
        row.code = code;
        row.outcome = outcome;
        row.message = code.message().into();
        let Some(MutationDataDto::PollRetirement(data)) = &mut row.data else {
            unreachable!()
        };
        data.completion = completion;
        if empty {
            data.retirements.clear();
        } else if completion == PollRetirementCompletion::Completed {
            data.retirements.truncate(2);
        }
        let out = once_output_in_mode(&row, false, ROOT, true);
        assert_eq!(out.status, status);
        let text = format!("{}{}", stdout(&out), stderr(&out));
        assert!(text.contains("poll (original observation report):"));
        if empty {
            assert!(text.contains("retirements: none"));
        } else {
            assert!(text.contains("state=refused"));
        }
        assert!(!text.contains("all contexts retired"));
    }
}
