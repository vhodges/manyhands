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
    let output = policy_output(false, &policy_envelope(true), false);
    assert_eq!(
        stderr(&output),
        format!("note: {SUSPENDED_NOTE}\n"),
        "no read but the envelope built here"
    );
    assert!(stdout(&output).contains("Recovery suspended: true\n"));
}

#[test]
fn no_suspension_has_no_note() {
    let output = policy_output(false, &policy_envelope(false), false);
    assert!(stderr(&output).is_empty());
}

#[test]
fn json_mode_holds_one_envelope_and_no_note() {
    let output = policy_output(true, &policy_envelope(true), false);
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
        said.contains("If it names only this request, wait and send this request again (add --request-id unless you run it at a terminal)."),
        "{said}"
    );
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
