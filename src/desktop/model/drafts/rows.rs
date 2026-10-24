//! The Drafts page's rows (ruling R24): one per stored draft, with its kind,
//! its own title, its repository, when it was last saved for recovery,
//! where it stands and what may be done with it.
//!
//! A readable row's target is `Read::Loading` until its reads land, then
//! `Ready`: read errors are classified into the target state, so a row is
//! never `None`, `Busy` or `Failed`. Who holds the draft comes first: one
//! open in another window offers Inspect only, and one restored in this
//! window Show, Inspect and Discard. Otherwise the restore policy decides
//! Restore. An unreadable draft offers Discard only. Export is offered on
//! every row, so it is not among the actions.
//!
//! A row holds the draft's title and, once read, its comparison, so it
//! derives no `Debug`. Outside its own row a draft's item is named by the
//! item's title as read, or its ID (ruling R25).

use manyhands::drafts::{
    Draft, DraftFields, DraftId, Held, ItemKind, RefusalReason, Restore, TargetState,
    UnreadableReason, restore_policy,
};

use crate::desktop::model::{names::TicketName, read_state::Read};

pub(crate) const DOCUMENT: &str = "Document";
pub(crate) const TICKET: &str = "Ticket";
pub(crate) const COMMENT_ON: &str = "Comment on";

/// One stored draft, as the Drafts page shows it.
pub(crate) struct DraftRow {
    pub(crate) id: DraftId,
    /// "Document", "Ticket", "Comment on <item title>".
    pub(crate) kind_label: String,
    /// The draft's own title, shown only in its row; empty for a comment
    /// or an unreadable draft.
    pub(crate) title: String,
    pub(crate) repository: String,
    /// The local time of the last recovery save; empty when unreadable.
    pub(crate) flushed_at: String,
    pub(crate) state: RowState,
    pub(crate) actions: RowActions,
}

// The approved row type (ruling R24) holds the target state by value.
#[allow(clippy::large_enum_variant)]
pub(crate) enum RowState {
    Readable {
        held: Held,
        target: Read<TargetState>,
    },
    Unreadable(UnreadableReason),
}

/// What a row offers besides Export.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub(crate) struct RowActions {
    /// The restore policy allows it.
    pub(crate) restore: bool,
    /// Restored in this window: focuses its tab.
    pub(crate) show: bool,
    /// Readable.
    pub(crate) inspect: bool,
    /// Not open in another window.
    pub(crate) discard: bool,
    /// The item could not be read: read it again.
    pub(crate) retry: bool,
}

/// What a row in `state` offers, `restore` being the restore policy's
/// answer once the target is known.
pub(crate) fn row_actions(state: &RowState, restore: Option<Restore>) -> RowActions {
    let (held, target) = match state {
        RowState::Unreadable(_) => {
            return RowActions {
                discard: true,
                ..RowActions::default()
            };
        }
        RowState::Readable { held, target } => (*held, target),
    };
    match held {
        Held::Elsewhere => RowActions {
            inspect: true,
            ..RowActions::default()
        },
        Held::Here => RowActions {
            show: true,
            inspect: true,
            discard: true,
            ..RowActions::default()
        },
        Held::No => {
            let Read::Ready(target) = target else {
                return RowActions {
                    inspect: true,
                    discard: true,
                    ..RowActions::default()
                };
            };
            RowActions {
                restore: matches!(restore, Some(Restore::Allowed | Restore::AllowedFlagged)),
                // Restored here after all: the tab is shown, not restored.
                show: matches!(restore, Some(Restore::Refused(RefusalReason::HeldHere))),
                inspect: true,
                discard: true,
                retry: matches!(target, TargetState::ReadFailed(_)),
            }
        }
    }
}

/// An unreadable draft's reason, in words.
pub(crate) fn unreadable_label(reason: UnreadableReason) -> &'static str {
    match reason {
        UnreadableReason::NewerFormat { .. } => "saved by a newer Manyhands",
        UnreadableReason::Damaged => "damaged",
        UnreadableReason::Access => "not accessible",
        UnreadableReason::NotOwned => "owned by another user",
        UnreadableReason::NotAFile => "not a file",
    }
}

/// A comment draft's kind: the item it comments on, by its title as read
/// or its ID.
fn comment_label(item: &str) -> String {
    format!("{COMMENT_ON} {item}")
}

impl DraftRow {
    /// A readable draft, its item not yet read. A comment's item is named
    /// by its ID until then.
    pub(crate) fn readable(draft: &Draft, held: Held, flushed_at: String) -> Self {
        let item = draft.target.item_id.to_string();
        let (kind_label, title) = match (&draft.target.comment, &draft.current) {
            (Some(_), _) | (None, DraftFields::Comment { .. }) => (comment_label(&item), ""),
            (None, DraftFields::Document { title, .. } | DraftFields::Ticket { title, .. }) => {
                let kind = match draft.target.item_kind {
                    ItemKind::Document => DOCUMENT,
                    ItemKind::Ticket => TICKET,
                };
                (kind.to_owned(), title.as_str())
            }
        };
        let state = RowState::Readable {
            held,
            target: Read::Loading,
        };
        let actions = row_actions(&state, None);
        Self {
            id: draft.id,
            kind_label,
            title: title.to_owned(),
            repository: draft.target.repository.clone(),
            flushed_at,
            state,
            actions,
        }
    }

    /// A draft file that could not be read. An unreadable draft's row. One another process holds offers no
    /// Discard…, as a readable one open elsewhere does not: the store's
    /// listing probes every draft's lock, readable or not.
    pub(crate) fn unreadable(id: DraftId, reason: UnreadableReason, held: Held) -> Self {
        let state = RowState::Unreadable(reason);
        let mut actions = row_actions(&state, None);
        actions.discard = held != Held::Elsewhere;
        Self {
            id,
            kind_label: String::new(),
            title: String::new(),
            repository: String::new(),
            flushed_at: String::new(),
            state,
            actions,
        }
    }

    /// The row's reads landed: its item stands as `target`, and is called
    /// `item_title` as read (`None` when it has none or the read failed).
    /// An unreadable row has nothing to land.
    pub(crate) fn landed(&mut self, draft: &Draft, target: TargetState, item_title: Option<&str>) {
        let RowState::Readable { held, .. } = self.state else {
            return;
        };
        if draft.target.comment.is_some() {
            let id = draft.target.item_id.to_string();
            let item = TicketName {
                slug: None,
                title: item_title,
            }
            .title_or(&id)
            .to_owned();
            self.kind_label = comment_label(&item);
        }
        let restore = restore_policy(draft, &target, held);
        self.state = RowState::Readable {
            held,
            target: Read::Ready(target),
        };
        self.actions = row_actions(&self.state, Some(restore));
    }

    /// The row's item is read again: it is checked anew.
    pub(crate) fn reading(&mut self) {
        let RowState::Readable { held, .. } = self.state else {
            return;
        };
        self.state = RowState::Readable {
            held,
            target: Read::Loading,
        };
        self.actions = row_actions(&self.state, None);
    }

    /// Where the row's item stands, once read.
    pub(crate) fn target(&self) -> Option<&TargetState> {
        match &self.state {
            RowState::Readable {
                target: Read::Ready(target),
                ..
            } => Some(target),
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "rows_tests.rs"]
mod tests;
