//! S11 to S14: the recovery copies not saved, the draft store that could
//! not open, the drafts recovered from an earlier session and the record
//! notes. None names a draft's own title or a path (decision 21): an item is
//! named by B1's rule from the title read when its draft began.

use manyhands::drafts::{DraftError, DraftErrorKind, DraftId};

use super::{StateInputs, menu_of, name_of};
use crate::desktop::{
    model::{
        activity::{
            copy::{
                DETAIL_FLUSH, DETAIL_NOTE, DETAIL_RECOVERED, DETAIL_STORE, REVIEW, flush_title,
                item_name, recovered_title, title_of,
            },
            rows::{ActivityAction, Derived, Group, ItemRef, Kind, Offer, RowKey},
        },
        list_view::RETRY,
        operations::record_note_words,
    },
    services::DraftStoreUnavailable,
};

/// The drafts' part of the inputs (A2): what `DraftRegistry` reports.
pub(crate) struct DraftsInput {
    /// A session whose recovery copy is not saved: its draft, its item, the
    /// item's title as read when the draft began (or its ID's text), and why.
    pub flush_failures: Vec<(DraftId, ItemRef, String, DraftErrorKind)>,
    pub unavailable: Option<DraftStoreUnavailable>,
    /// How many drafts the launch found that this process does not hold.
    pub recovered: Option<usize>,
}

pub(super) fn rows(inputs: &StateInputs<'_>) -> Vec<Derived> {
    let drafts = inputs.drafts;
    let mut rows: Vec<Derived> = drafts
        .flush_failures
        .iter()
        .map(|(draft, item, title, kind)| flush(inputs, *draft, item, title, *kind))
        .collect();
    rows.extend(drafts.unavailable.as_ref().map(|reason| Derived {
        description: Some(reason.to_string()),
        detail: vec![DETAIL_STORE.to_owned()],
        group: Group {
            candidate: true,
            ..Group::default()
        },
        ..Derived::new(
            RowKey::DraftStore,
            Kind::Failure,
            crate::desktop::model::operations::DRAFTS_UNAVAILABLE.to_owned(),
        )
    }));
    rows.extend(drafts.recovered.map(|count| {
        let review = Offer::new(ActivityAction::ReviewDrafts, REVIEW, true);
        Derived {
            menu: menu_of(
                Some(&review),
                vec![Offer::new(
                    ActivityAction::DismissRecovered,
                    crate::desktop::model::activity::copy::DISMISS,
                    true,
                )],
            ),
            button: Some(review),
            detail: vec![DETAIL_RECOVERED.to_owned()],
            ..Derived::new(RowKey::Recovered, Kind::Info, recovered_title(count))
        }
    }));
    rows.extend(inputs.operations.record_notes().iter().map(|report| {
        let key = RowKey::Note(*report);
        Derived {
            menu: vec![Offer::new(
                ActivityAction::DismissNote(*report),
                crate::desktop::model::activity::copy::DISMISS,
                true,
            )],
            detail: vec![DETAIL_NOTE.to_owned()],
            ..Derived::new(key, Kind::Info, title_of(record_note_words(*report)))
        }
    }));
    rows
}

/// S11: a recovery copy that was not saved. Its item is named by B1's rule
/// from the title read when the draft began; when that is the item's ID, by
/// its short code.
fn flush(
    inputs: &StateInputs<'_>,
    draft: DraftId,
    item: &ItemRef,
    title: &str,
    kind: DraftErrorKind,
) -> Derived {
    let known = (title != item.id.to_string()).then_some(title);
    let name = item_name(known, item.kind, &item.id);
    let retry = Offer::new(ActivityAction::RetryFlush(draft), RETRY, true);
    Derived {
        description: Some(DraftError { kind }.to_string()),
        repo: Some(name_of(inputs, &item.root)),
        menu: menu_of(Some(&retry), Vec::new()),
        button: Some(retry),
        detail: vec![DETAIL_FLUSH.to_owned()],
        item: Some(item.clone()),
        group: Group {
            candidate: true,
            roots: vec![item.root.clone()],
            ..Group::default()
        },
        ..Derived::new(RowKey::Flush(draft), Kind::Failure, flush_title(&name))
    }
}
