//! S8: what became of retirements whose answer dismissed their request
//! (R-51, R-59). Nothing is read from the schedule's retirement lines, which
//! a later attempt clears: `Operations::retire_events` keeps them for the
//! session.

use std::collections::BTreeMap;

use manyhands::{canonical::ItemId, results::Outcome};

use super::{StateInputs, dismiss, item_label, name_of};
use crate::desktop::model::{
    activity::{
        copy::{RETIRE_TRIED_AGAIN, retire_note_title, retired_title},
        rows::{Derived, Family, Group, Kind, RowKey, Subject},
    },
    operations::RetireEvent,
    polling::retire::Verdict,
    stamp::RepoKey,
};

/// A Retired event is a success row keyed by its sequence number, so a later
/// retirement of the same item is a new row; it counts for nothing but sees
/// as a later success (N22 clause 5). The others are information rows, one
/// for each item, the latest replacing the earlier (R-59): its request's
/// queue order tells a replaced note from the same one.
pub(super) fn rows(inputs: &StateInputs<'_>) -> Vec<Derived> {
    let events = inputs.operations.retire_events();
    // The last information event of each item, found once: an earlier one is
    // replaced by it.
    let mut last_note: BTreeMap<(&RepoKey, &ItemId), usize> = BTreeMap::new();
    for (at, event) in events.iter().enumerate() {
        if is_note(event) {
            last_note.insert((&event.root, &event.item), at);
        }
    }
    events
        .iter()
        .enumerate()
        .filter_map(|(at, event)| match event.verdict {
            Verdict::Retired => Some(retired(inputs, event)),
            Verdict::Attention(_) | Verdict::Unreachable | Verdict::Retry { .. } => {
                let replaced = last_note
                    .get(&(&event.root, &event.item))
                    .is_some_and(|last| *last > at);
                (!replaced).then(|| note(inputs, event))
            }
            _ => None,
        })
        .collect()
}

fn is_note(event: &RetireEvent) -> bool {
    matches!(
        event.verdict,
        Verdict::Attention(_) | Verdict::Unreachable | Verdict::Retry { .. }
    )
}

fn retired(inputs: &StateInputs<'_>, event: &RetireEvent) -> Derived {
    let key = RowKey::Retired(event.seq);
    let name = item_label(inputs, &event.root, event.kind, &event.item);
    Derived {
        repo: Some(name_of(inputs, &event.root)),
        menu: vec![dismiss(&key)],
        clearable: true,
        group: Group {
            subject: Some(Subject::Intent {
                family: Family::Retire,
                root: Some(event.root.clone()),
                item: Some(event.item.clone()),
                target: None,
            }),
            sent: Some(event.sent),
            outcome: Some(Outcome::Success),
            ..Group::default()
        },
        ..Derived::new(key, Kind::Success, retired_title(&name))
    }
}

fn note(inputs: &StateInputs<'_>, event: &RetireEvent) -> Derived {
    let key = RowKey::RetireNote {
        root: event.root.clone(),
        item: event.item.clone(),
    };
    let name = item_label(inputs, &event.root, event.kind, &event.item);
    Derived {
        description: Some(RETIRE_TRIED_AGAIN.to_owned()),
        repo: Some(name_of(inputs, &event.root)),
        menu: vec![dismiss(&key)],
        clearable: true,
        group: Group {
            sent: Some(event.sent),
            ..Group::default()
        },
        ..Derived::new(key, Kind::Info, retire_note_title(&name))
    }
}
