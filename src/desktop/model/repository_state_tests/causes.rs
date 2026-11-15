//! The cause of Needs attention (Wave 04 Cycle 03b design A7; R-5): set by
//! `attention_cause` for the state's row 6 and for no other, one variant
//! for each cause of fact 23, in its order. Cycle 03a's tables compare the
//! state's other fields, so this is what pins the new one.

use super::*;

/// The cause each Needs attention case of the tables has.
fn cause_of(name: &str) -> AttentionCause {
    use AttentionCause::*;
    match name {
        "key problem" => Phase(Reason::KeyProblem(key())),
        "no key selected" => Phase(Reason::NoKeySelected(ROOT.to_owned())),
        "host unapproved" => Phase(Reason::HostUnapproved(authority())),
        "configuration required" => Phase(Reason::ConfigurationRequired(ROOT.to_owned())),
        "held by the phase" => Phase(Reason::RecoveryHeld(ROOT.to_owned())),
        "held by the policy" | "a held repository reads needs attention, not can't reach" => Held,
        "pending conflict" | "a stored outcome with a pending conflict does not apply" => Conflict,
        "failed wait" => WaitingFailed,
        "unreadable" => Unreadable,
        "last answer: transport unavailable" | "last answer: configuration required" => Transport,
        "stored: selected key unavailable"
        | "stored: host approval required"
        | "stored: configuration required"
        | "stored: repository unavailable" => Stored,
        "problems" | "to share with problems" => Problems,
        other => panic!("{other}: no cause written for this Needs attention case"),
    }
}

#[test]
fn each_attention_cause_is_set() {
    let mut causes = Vec::new();
    for case in states::alone().into_iter().chain(pairs::pairs()) {
        let mut scene = Scene::new();
        (case.set)(&mut scene);
        let state = scene.state();
        if state.kind == StateKind::NeedsAttention {
            assert_eq!(state.cause, Some(cause_of(case.name)), "{}", case.name);
            causes.push(state.cause);
        } else {
            // Locked is decided before `attention_cause` runs, so it has none.
            assert_eq!(state.cause, None, "{}", case.name);
        }
    }
    // Every cause of fact 23 is reached by some case.
    for wanted in [
        AttentionCause::Held,
        AttentionCause::Conflict,
        AttentionCause::WaitingFailed,
        AttentionCause::Unreadable,
        AttentionCause::Transport,
        AttentionCause::Stored,
        AttentionCause::Problems,
    ] {
        assert!(causes.contains(&Some(wanted.clone())), "{wanted:?}");
    }
    assert!(
        causes
            .iter()
            .any(|cause| matches!(cause, Some(AttentionCause::Phase(_)))),
        "a reason in the phase"
    );
}
