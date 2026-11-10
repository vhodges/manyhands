//! The attempt entry's content (D6 Task 9, A2, A4, R-20, R-23).

use manyhands::repository::{OperationId, PollDisposition};

use super::*;
use crate::desktop::model::polling::{Answer, AttemptTarget, PollAttempt};

fn attempt(answer: Answer, dispositions: &[Disposition]) -> AttemptSummary {
    let attempt = PollAttempt {
        root: "r".to_owned(),
        operation: OperationId::new(),
        answer,
        completed_at: None,
        next_eligible_at: None,
        targets: dispositions
            .iter()
            .map(|&disposition| AttemptTarget {
                item_id: Some(ItemId::generate()),
                kind: Some(TargetKind::Ticket),
                disposition,
            })
            .collect(),
        index_pending: false,
    };
    AttemptSummary::of(&attempt)
}

/// A2, R-20: each disposition the poll leaves alone is listed with its
/// text, `guidance()` but for `Unpublished`, which names Share… in place of
/// the CLI's `item sync` and offers it; no other line offers anything.
#[test]
fn dispositions_listed() {
    use Disposition::*;
    let table = [
        (Dirty, PollDisposition::Dirty.guidance()),
        (Diverged, PollDisposition::Diverged.guidance()),
        (Conflicted, PollDisposition::Conflicted.guidance()),
        (Inaccessible, PollDisposition::Inaccessible.guidance()),
        (
            BlockedByLocalState,
            PollDisposition::BlockedByLocalState.guidance(),
        ),
        (
            MaterializationIncomplete,
            PollDisposition::MaterializationIncomplete.guidance(),
        ),
        (Nonconforming, PollDisposition::Nonconforming.guidance()),
        (Malformed, PollDisposition::Malformed.guidance()),
        (RemotelyDeleted, PollDisposition::RemotelyDeleted.guidance()),
        (HistoryUnknown, PollDisposition::HistoryUnknown.guidance()),
        (Unpublished, UNPUBLISHED),
        (NotReached, PollDisposition::NotReached.guidance()),
    ];
    for (disposition, expected) in table {
        for answer in [Answer::Completed, Answer::Interrupted] {
            let summary = attempt(answer.clone(), &[disposition]);
            let entry = attempt_entry(&summary)
                .unwrap_or_else(|| panic!("{disposition:?} in {answer:?}: no entry"));
            assert_eq!(entry.brought_in, None, "{disposition:?}");
            assert_eq!(entry.lines.len(), 1, "{disposition:?}");
            let line = &entry.lines[0];
            assert_eq!(line.text, expected, "{disposition:?}");
            assert_eq!(
                line.action,
                (disposition == Unpublished).then_some(LineAction::Share),
                "{disposition:?}"
            );
            assert!(line.item_id.is_some() && line.kind == Some(TargetKind::Ticket));
        }
    }
    // The desktop's own words name no CLI command (R-20).
    assert!(!UNPUBLISHED.contains("item sync") && UNPUBLISHED.contains("Share\u{2026}"));
    assert_eq!(LineAction::Share.label(), SHARE);

    // One line per item, in the poll's order, and only for the ones left
    // alone.
    let summary = attempt(Answer::Completed, &[Dirty, Current, Unpublished, Dirty]);
    let entry = attempt_entry(&summary).expect("an entry");
    let texts: Vec<_> = entry.lines.iter().map(|line| line.text).collect();
    assert_eq!(
        texts,
        [
            PollDisposition::Dirty.guidance(),
            UNPUBLISHED,
            PollDisposition::Dirty.guidance()
        ]
    );
}

/// A4, R-23: an attempt that only found things current or ahead shows no
/// entry (the Settings row says so); with something else to say, those two
/// add no line, and what was brought in is one line of its own.
#[test]
fn a_quiet_attempt_has_no_entry() {
    use Disposition::*;
    for quiet in [&[][..], &[Current], &[LocalAhead, Current, LocalAhead]] {
        let summary = attempt(Answer::Completed, quiet);
        assert_eq!(attempt_entry(&summary), None, "{quiet:?}");
    }

    let summary = attempt(Answer::Completed, &[Current, LocalAhead, FastForwarded]);
    let entry = attempt_entry(&summary).expect("brought in is something to say");
    assert_eq!(entry.brought_in.as_deref(), Some("Brought in 1 change"));
    assert!(entry.lines.is_empty());

    let summary = attempt(
        Answer::Completed,
        &[Materialized, FastForwarded, LocalAhead, Dirty],
    );
    let entry = attempt_entry(&summary).expect("an entry");
    assert_eq!(entry.brought_in.as_deref(), Some("Brought in 2 changes"));
    assert_eq!(entry.lines.len(), 1, "LocalAhead adds no line");

    // An item without an ID or a kind cannot be shared.
    let mut summary = attempt(Answer::Completed, &[Unpublished]);
    summary.listed[0].item_id = None;
    let entry = attempt_entry(&summary).expect("an entry");
    assert_eq!(entry.lines[0].action, None);
}
