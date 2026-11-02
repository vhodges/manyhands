//! The human lines of the polling verbs, on built envelopes.

use super::*;
use crate::render::test_support::{stderr, stdout};
use manyhands::{
    repository::{PollPolicyMutationDto, RequestId},
    results::{Effects, ResultCode, Scope},
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
