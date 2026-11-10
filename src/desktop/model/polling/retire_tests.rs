//! A retirement's answer (D6 design E3, A20): one table, record state, then
//! the answer's state, then its code.

use manyhands::{
    canonical::ItemId,
    repository::{ContextRetirementState as State, RetirementRemoval as Removal, SharedKeyId},
    results::ResultCode as Code,
};

use super::{Authority, Need, RefusedSet, Verdict, classify, reason_of, recorded_key};
use crate::desktop::{
    model::polling::Reason,
    services::worker::RecordState,
    test_support::retire::{ITEM, OPERATION, answer, no_data},
};

/// The shape of a verdict, apart from the words it carries.
use Verdict::*;

struct Row {
    what: &'static str,
    record: RecordState,
    /// `None`: the library gave no retirement data.
    state: Option<State>,
    code: Code,
    removal: Removal,
    verdict: Verdict,
    /// Dismissed from Activity: attention or the next attempt covers it.
    dismissed: bool,
    /// The loop submits the next item (RT-7), rather than stopping (RT-8).
    goes_on: bool,
    /// The repository's tabs read their items again (A20).
    changed: bool,
    /// Joined the session's refused set after one such answer.
    refused: bool,
}

fn row(
    what: &'static str,
    record: RecordState,
    state: Option<State>,
    code: Code,
    verdict: Verdict,
) -> Row {
    Row {
        what,
        record,
        state,
        code,
        removal: Removal::None,
        dismissed: matches!(verdict, Retired | Attention(_) | Unreachable | Retry { .. }),
        goes_on: goes_on(what),
        changed: matches!(verdict, Retired),
        refused: matches!(verdict, Refused),
        verdict,
    }
}

/// RT-7 and RT-8, written out row by row and not derived from the verdict:
/// the loop goes on after a retirement, a refusal or preservation with a
/// refusal code, a retry code other than a transport one, and a request that
/// failed with no data; it stops on a pending or Accepted answer, an
/// attention code, and a transport or remote-unavailable code.
fn goes_on(what: &str) -> bool {
    match what {
        "retired"
        | "already absent"
        | "refused, index"
        | "refused, external change"
        | "refused, internal"
        | "refused, cancelled"
        | "refused, recovery required"
        | "refused, recovery required after a removal"
        | "refused, other"
        | "refused, input"
        | "preserved"
        | "preserved, a retry code"
        | "preserved after a removal"
        | "no data, finished"
        | "no data, a retry code" => true,
        "pending, the index still catching up"
        | "pending, cleanup"
        | "pending, cancelled"
        | "pending, locked key"
        | "pending, key rejected"
        | "pending, host"
        | "pending, bad remote"
        | "pending, a worktree removed"
        | "left accepted, no publication remote"
        | "left accepted, busy"
        | "left accepted, poll yielding"
        | "left accepted, no data"
        | "left accepted, a retry code"
        | "left accepted, locked key"
        | "left accepted, host"
        | "left accepted, final answer"
        | "record unknown"
        | "refused, locked"
        | "refused, unlock failed"
        | "refused, no key"
        | "refused, key missing"
        | "refused, key rejected"
        | "refused, host unapproved"
        | "refused, host replaced"
        | "refused, bad remote"
        | "refused, no publication remote"
        | "refused, transport"
        | "refused, remote" => false,
        other => panic!("{other}: no RT-7/RT-8 value written for this row"),
    }
}

fn owed(attention: Option<Need>, catching_up: bool) -> Verdict {
    Owed {
        attention,
        catching_up,
    }
}

fn rows() -> Vec<Row> {
    use RecordState::{Accepted, Finished, Unknown};
    let refused = |what, code, verdict| row(what, Finished, Some(State::Refused), code, verdict);
    let mut rows = vec![
        // Retired: the copy is gone.
        row("retired", Finished, Some(State::Retired), Code::Ok, Retired),
        row(
            "already absent",
            Finished,
            Some(State::AlreadyAbsent),
            Code::AlreadyApplied,
            Retired,
        ),
        // Pending, Owed: the pending row offers Resume and Abandon.
        row(
            "pending, the index still catching up",
            Finished,
            Some(State::Pending),
            Code::DiscoveryPending,
            owed(None, true),
        ),
        row(
            "pending, cleanup",
            Finished,
            Some(State::Pending),
            Code::RecoveryRequired,
            owed(None, false),
        ),
        row(
            "pending, cancelled",
            Finished,
            Some(State::Pending),
            Code::Cancelled,
            owed(None, false),
        ),
        // A pending answer with an unlock, key or host code raises that
        // attention too.
        row(
            "pending, locked key",
            Finished,
            Some(State::Pending),
            Code::UnlockRequired,
            owed(Some(Need::KeyLocked), false),
        ),
        row(
            "pending, key rejected",
            Finished,
            Some(State::Pending),
            Code::KeyRejected,
            owed(Some(Need::KeyProblem), false),
        ),
        row(
            "pending, host",
            Finished,
            Some(State::Pending),
            Code::HostReplacementRequired,
            owed(Some(Need::Host), false),
        ),
        // A configuration code is the item's answer only as a refusal (E3
        // row 6); an Owed answer raises no attention for it.
        row(
            "pending, bad remote",
            Finished,
            Some(State::Pending),
            Code::InvalidRemote,
            owed(None, false),
        ),
        row(
            "left accepted, no publication remote",
            Accepted,
            Some(State::Refused),
            Code::PublicationRemoteRequired,
            owed(None, false),
        ),
        // A request left Accepted, whatever its answer, is pending and never
        // a retry; the entry stays.
        row(
            "left accepted, busy",
            Accepted,
            Some(State::Refused),
            Code::Busy,
            owed(None, false),
        ),
        row(
            "left accepted, poll yielding",
            Accepted,
            Some(State::Refused),
            Code::PollYielding,
            owed(None, false),
        ),
        row(
            "left accepted, no data",
            Accepted,
            None,
            Code::Busy,
            owed(None, false),
        ),
        row(
            "left accepted, a retry code",
            Accepted,
            Some(State::Refused),
            Code::TransportUnavailable,
            owed(None, false),
        ),
        row(
            "left accepted, locked key",
            Accepted,
            Some(State::Refused),
            Code::UnlockRequired,
            owed(Some(Need::KeyLocked), false),
        ),
        row(
            "left accepted, host",
            Accepted,
            Some(State::Refused),
            Code::HostApprovalRequired,
            owed(Some(Need::Host), false),
        ),
        row(
            "left accepted, final answer",
            Accepted,
            Some(State::Retired),
            Code::Ok,
            owed(None, false),
        ),
        // Where the record stands is not known: the entry is kept, so it is
        // pending too.
        row(
            "record unknown",
            Unknown,
            Some(State::Refused),
            Code::WorktreeNotClean,
            owed(None, false),
        ),
        // Refused with an attention code: the person acts; the loop stops;
        // the next completed attempt tries again; not the refused set.
        refused(
            "refused, locked",
            Code::UnlockRequired,
            Attention(Need::KeyLocked),
        ),
        refused(
            "refused, unlock failed",
            Code::UnlockFailed,
            Attention(Need::KeyLocked),
        ),
        refused(
            "refused, no key",
            Code::NoSelectedKey,
            Attention(Need::NoKey),
        ),
        refused(
            "refused, key missing",
            Code::KeyUnavailable,
            Attention(Need::KeyProblem),
        ),
        refused(
            "refused, key rejected",
            Code::KeyRejected,
            Attention(Need::KeyProblem),
        ),
        refused(
            "refused, host unapproved",
            Code::HostApprovalRequired,
            Attention(Need::Host),
        ),
        refused(
            "refused, host replaced",
            Code::HostReplacementRequired,
            Attention(Need::Host),
        ),
        refused(
            "refused, bad remote",
            Code::InvalidRemote,
            Attention(Need::Configuration),
        ),
        refused(
            "refused, no publication remote",
            Code::PublicationRemoteRequired,
            Attention(Need::Configuration),
        ),
        // Nothing per item, and the loop stops (RT-8).
        refused(
            "refused, transport",
            Code::TransportUnavailable,
            Unreachable,
        ),
        refused("refused, remote", Code::RemoteUnavailable, Unreachable),
        // Nothing per item; the loop goes on; the next attempt tries again.
        refused(
            "refused, index",
            Code::IndexUnavailable,
            Retry {
                recovery_required: false,
            },
        ),
        refused(
            "refused, external change",
            Code::ExternalChange,
            Retry {
                recovery_required: false,
            },
        ),
        refused(
            "refused, internal",
            Code::InternalError,
            Retry {
                recovery_required: false,
            },
        ),
        refused(
            "refused, cancelled",
            Code::Cancelled,
            Retry {
                recovery_required: false,
            },
        ),
        refused(
            "refused, recovery required",
            Code::RecoveryRequired,
            Retry {
                recovery_required: true,
            },
        ),
        // Any other refusal, and every preserved one, is the item's answer:
        // the refused set, the row stays, Retire… is offered.
        refused("refused, other", Code::WorktreeNotClean, Refused),
        refused("refused, input", Code::InvalidInput, Refused),
        row(
            "preserved",
            Finished,
            Some(State::Preserved),
            Code::ExternalChange,
            Refused,
        ),
        row(
            "preserved, a retry code",
            Finished,
            Some(State::Preserved),
            Code::RecoveryRequired,
            Refused,
        ),
        // No retirement data on a finished request: an ordinary failure,
        // shown as one, tried again at the next attempt.
        row(
            "no data, finished",
            Finished,
            None,
            Code::InvalidInput,
            Failed,
        ),
        row(
            "no data, a retry code",
            Finished,
            None,
            Code::TransportUnavailable,
            Failed,
        ),
    ];
    // A removal marks a local change, whatever the answer; and
    // `recovery_required` with one removed is no plain retry (E3).
    let mut removed = row(
        "pending, a worktree removed",
        Finished,
        Some(State::Pending),
        Code::RecoveryRequired,
        owed(None, false),
    );
    removed.removal = Removal::Observed;
    removed.changed = true;
    rows.push(removed);
    let mut removed = row(
        "refused, recovery required after a removal",
        Finished,
        Some(State::Refused),
        Code::RecoveryRequired,
        Refused,
    );
    removed.removal = Removal::Possible;
    removed.changed = true;
    rows.push(removed);
    let mut removed = row(
        "preserved after a removal",
        Finished,
        Some(State::Preserved),
        Code::ExternalChange,
        Refused,
    );
    removed.removal = Removal::Observed;
    removed.changed = true;
    rows.push(removed);
    rows
}

/// A20: the record state first, then the answer's state, then its code.
#[test]
fn retire_answer_table() {
    let root = "/repo".to_owned();
    let item: ItemId = ITEM.parse().unwrap();
    for row in rows() {
        let envelope = match row.state {
            Some(state) => answer(state, row.code, row.removal),
            None => no_data(row.code),
        };
        let seen = classify(row.record, &envelope);
        assert_eq!(seen.verdict, row.verdict, "{}", row.what);
        assert_eq!(
            seen.verdict.dismissed(),
            row.dismissed,
            "{}: dismissed",
            row.what
        );
        assert_eq!(
            seen.verdict.goes_on(),
            row.goes_on,
            "{}: the loop",
            row.what
        );
        assert_eq!(seen.changed, row.changed, "{}: changed", row.what);
        // The journal row's operation is what a request and its pending row
        // are grouped by.
        let operation = row.state.map(|_| OPERATION.to_owned());
        assert_eq!(seen.operation, operation, "{}: operation", row.what);
        let mut set = RefusedSet::default();
        set.answered(&root, &item, &seen.verdict);
        assert_eq!(
            set.contains(&root, &item),
            row.refused,
            "{}: refused",
            row.what
        );
    }
    attention_is_keyed();
    recovery_refusals_are_counted();
}

/// Attention is keyed by the key the background provider refused, or the
/// one a user-lane request asked for, else the repository's selected key
/// (E3); with none known, the repository is asked to choose one. A host
/// answer carries no host, so the scheduler's is used, and with none the
/// repository is the entry.
fn attention_is_keyed() {
    let root = "/repo".to_owned();
    let key = SharedKeyId::new();
    let host = Authority {
        host: "git.example.invalid".to_owned(),
        port: 22,
    };
    let cases = [
        (Need::KeyLocked, Some(key), None, Reason::KeyLocked(key)),
        (
            Need::KeyLocked,
            None,
            None,
            Reason::NoKeySelected(root.clone()),
        ),
        (Need::KeyProblem, Some(key), None, Reason::KeyProblem(key)),
        (
            Need::KeyProblem,
            None,
            None,
            Reason::NoKeySelected(root.clone()),
        ),
        // No selected key has no key to name, whatever was recorded.
        (
            Need::NoKey,
            Some(key),
            None,
            Reason::NoKeySelected(root.clone()),
        ),
        (
            Need::Host,
            None,
            Some(host.clone()),
            Reason::HostUnapproved(host),
        ),
        (
            Need::Host,
            None,
            None,
            Reason::HostUnidentified(root.clone()),
        ),
        (
            Need::Configuration,
            Some(key),
            None,
            Reason::ConfigurationRequired(root.clone()),
        ),
    ];
    for (need, key, host, reason) in cases {
        assert_eq!(reason_of(need, &root, key, host), reason, "{need:?}");
    }
    // Which key: the one refused, else the one asked; never a guess among
    // several, which the selected key then settles.
    let other = SharedKeyId::new();
    assert_eq!(recorded_key(&[key], &[]), Some(key));
    assert_eq!(recorded_key(&[], &[key]), Some(key));
    assert_eq!(recorded_key(&[key], &[other]), Some(key));
    assert_eq!(recorded_key(&[], &[]), None);
    assert_eq!(recorded_key(&[key, other], &[key]), None);
    assert_eq!(recorded_key(&[], &[key, other]), None);
}

/// A `recovery_required` refusal is retried; the third in one session, for
/// one item, joins the refused set (E3, R-21). Each item counts alone, and
/// Retire… clears the item (A19).
fn recovery_refusals_are_counted() {
    let root = "/repo".to_owned();
    let other_root = "/other".to_owned();
    let item: ItemId = ITEM.parse().unwrap();
    let other_item: ItemId = "01ARZ3NDEKTSV4RRFFQ69G5FG1".parse().unwrap();
    let retry = Verdict::Retry {
        recovery_required: true,
    };
    let plain = Verdict::Retry {
        recovery_required: false,
    };
    let mut set = RefusedSet::default();
    for _ in 0..2 {
        set.answered(&root, &item, &retry);
        // Another retry code is not counted.
        set.answered(&root, &item, &plain);
    }
    set.answered(&other_root, &item, &retry);
    set.answered(&root, &other_item, &retry);
    assert!(!set.contains(&root, &item), "two are retried");
    set.answered(&root, &item, &retry);
    assert!(set.contains(&root, &item), "the third joins");
    assert!(!set.contains(&other_root, &item));
    assert!(!set.contains(&root, &other_item));
    set.forget(&root, &item);
    assert!(!set.contains(&root, &item), "Retire… removes it");
    set.answered(&root, &item, &retry);
    assert!(!set.contains(&root, &item), "and its count starts over");
}
