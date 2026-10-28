//! The Drafts page: every draft the store keeps, with where it stands.
//!
//! The rows come from the registry's listing of the store. Each readable
//! row's item is then read under that row's own stamp (resolve, item and,
//! for a comment, its thread), so a late answer never lands on a newer
//! listing: a listing that lands rebuilds the rows and reads again. A row
//! shows its kind, the draft's own title, the repository, the time of its
//! last recovery save, and its state as text, never by color alone.
//!
//! The page owns whether it is shown and knows nothing of its host: the
//! content router in `workspace/content.rs` shows and hides it, and asks
//! it for its first control's focus. It does not assume it fills the
//! content region. Show asks the host to show a tab, as an event; it never
//! hides the page itself (a later dock may keep it in view). Restore, once
//! the draft is claimed, asks the host to open its item's tab, which the
//! host does through `open_item`, leaving the page. Inspect opens a panel
//! under its row; Restore, Export… and Discard… are `row_actions`, the
//! last two through the `actions` shared with the tab.

pub(crate) mod actions;
mod inspect;
mod row_actions;
mod rows;

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use gpui_kit::{
    Context, EventEmitter, FocusHandle, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, Subscription, Task, TestSupportExt, Window,
    component::v_flex, div,
};
use manyhands::{
    drafts::{Draft, DraftId, Entry, Listing, TargetState, classify, classify_comment},
    results::ResultCode,
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use super::{confirm::local_time, scroll::ContentScroll, sidebar::RETRY};
use crate::desktop::{
    model::{
        drafts::DraftRow,
        editor_text::normalized,
        navigation::OpenTarget,
        stamp::{Generations, Stamp, ViewScope},
        tabs::TabKey,
    },
    services::{ReadKind, ReadResult, Reader, Services},
};

pub(crate) const DRAFTS_HEADING: &str = "Drafts";
pub(crate) const NO_DRAFTS: &str = "No drafts are kept.";
pub(crate) const LOADING_DRAFTS: &str = "Loading drafts";
pub(crate) const CHECKING: &str = "Checking the item";
pub(crate) const OPEN_ELSEWHERE: &str = "Open in another Manyhands window";
pub(crate) const RESTORED_HERE: &str = "Restored in this window";
pub(crate) const CANNOT_READ: &str = "This draft cannot be read";
pub(crate) const COMMENT_LATER: &str = "The comment composer arrives in a later release.";
pub(crate) const SAVED_AT: &str = "Saved for recovery at";
pub(crate) const RESTORE: &str = "Restore";
pub(crate) const SHOW: &str = "Show";
pub(crate) const INSPECT: &str = "Inspect";
pub(crate) const DISCARD: &str = "Discard…";

pub(crate) use actions::{EXPORT, EXPORT_RAW};

/// A row's actions, by element part and label, in the order a row shows
/// them (an already saved draft shows Discard first).
pub(crate) const ACTIONS: [(&str, &str); 7] = [
    ("restore", RESTORE),
    ("show", SHOW),
    ("inspect", INSPECT),
    ("export", EXPORT),
    ("export-raw", EXPORT_RAW),
    ("discard", DISCARD),
    ("retry", RETRY),
];

/// The element ID of a part of the row of `id`: stable across listings.
pub(crate) fn row_id(id: DraftId, part: &str) -> String {
    format!("drafts-row-{id}-{part}")
}

/// What the page asks of its host.
pub(crate) enum DraftsEvent {
    /// Show and focus this tab: a draft restored here.
    Show(TabKey),
    /// Open and focus this item's tab: its draft was restored into it.
    Open(OpenTarget),
}

/// What the last action on a row came to, in words: fixed sentences, or
/// a refusal's.
struct Notice {
    text: String,
    /// A Restore refused as open elsewhere.
    restore: bool,
}

/// A discarded row, and the rows to focus once it has gone, in order.
struct Refocus {
    removed: DraftId,
    next: Vec<DraftId>,
}

/// One row, the draft it shows when readable, and its item's reads.
struct Slot {
    row: DraftRow,
    draft: Option<Draft>,
    /// Replacing it drops the previous reads; the stamp still guards.
    read: Option<Task<()>>,
}

/// A row's focus, and its actions' wrappers', in `ACTIONS` order. Kept by
/// draft across listings.
struct RowFocus {
    row: FocusHandle,
    actions: [FocusHandle; ACTIONS.len()],
}

pub(crate) struct DraftsPage {
    services: Services,
    scroll: ContentScroll,
    shown: bool,
    slots: Vec<Slot>,
    generations: Generations,
    focus: HashMap<DraftId, RowFocus>,
    /// The row to focus first: the draft the page was opened on.
    focus_row: Option<DraftId>,
    /// The drafts being restored: one Restore at a time each.
    restoring: HashSet<DraftId>,
    /// What the last action on a row came to, in words, by draft.
    notices: HashMap<DraftId, Notice>,
    /// Where the focus goes once a discarded row has gone.
    refocus: Option<Refocus>,
    /// The row the Inspect panel is open on, and the panel's focus.
    inspecting: Option<DraftId>,
    inspect_focus: FocusHandle,
    /// The registry's listing count the rows were built from.
    built: Option<u64>,
    _registry: Subscription,
}

impl EventEmitter<DraftsEvent> for DraftsPage {}

impl DraftsPage {
    pub(crate) fn new(services: Services, scroll: ContentScroll, cx: &mut Context<Self>) -> Self {
        let registry = cx.observe(services.drafts(), |page, _, cx| page.registry_changed(cx));
        Self {
            services,
            scroll,
            shown: false,
            slots: Vec::new(),
            generations: Generations::default(),
            focus: HashMap::new(),
            focus_row: None,
            restoring: HashSet::new(),
            notices: HashMap::new(),
            refocus: None,
            inspecting: None,
            inspect_focus: cx.focus_handle().tab_stop(true),
            built: None,
            _registry: registry,
        }
    }

    pub(crate) fn is_shown(&self) -> bool {
        self.shown
    }

    /// Shows the page, its rows built from the last listing and their
    /// items read again, `focus` the row to focus first.
    pub(crate) fn show(
        &mut self,
        focus: Option<DraftId>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.shown = true;
        self.focus_row = focus;
        self.rebuild(cx);
        cx.notify();
    }

    /// Hides the page; its reads are dropped.
    pub(crate) fn hide(&mut self, cx: &mut Context<Self>) {
        if self.shown {
            self.shown = false;
            self.slots.clear();
            self.notices.clear();
            self.refocus = None;
            self.inspecting = None;
            cx.notify();
        }
    }

    /// The page's first control: the row it was opened on, else the
    /// first row. `None` while hidden or with no rows.
    pub(crate) fn first_focus(&self) -> Option<FocusHandle> {
        if !self.shown {
            return None;
        }
        let listed = |id: &DraftId| self.slots.iter().any(|slot| slot.row.id == *id);
        let id = self
            .focus_row
            .filter(listed)
            .or_else(|| self.slots.first().map(|slot| slot.row.id))?;
        self.focus.get(&id).map(|focus| focus.row.clone())
    }

    /// A new listing landed while shown: the rows are built again.
    fn registry_changed(&mut self, cx: &mut Context<Self>) {
        if self.shown && Some(self.services.drafts().read(cx).listings_landed()) != self.built {
            self.rebuild(cx);
        }
        cx.notify();
    }

    /// The rows of the last listing, each readable one's item read under
    /// a new stamp, so no earlier read lands on them.
    fn rebuild(&mut self, cx: &mut Context<Self>) {
        let registry = self.services.drafts().read(cx);
        self.built = Some(registry.listings_landed());
        let listings: Vec<Listing> = registry.listing().rows().cloned().unwrap_or_default();
        self.slots = listings
            .into_iter()
            .map(|listing| match listing.entry {
                Entry::Draft(draft) => Slot {
                    row: DraftRow::readable(&draft, listing.held, saved_at(draft.flushed_at)),
                    draft: Some(draft),
                    read: None,
                },
                Entry::Unreadable(reason) => Slot {
                    row: DraftRow::unreadable(listing.id, reason, listing.held),
                    draft: None,
                    read: None,
                },
            })
            .collect();
        let ids: Vec<DraftId> = self.slots.iter().map(|slot| slot.row.id).collect();
        self.focus.retain(|id, _| ids.contains(id));
        self.notices.retain(|id, _| ids.contains(id));
        self.inspecting = self.inspecting.filter(|id| ids.contains(id));
        for id in ids {
            self.focus.entry(id).or_insert_with(|| RowFocus {
                row: cx.focus_handle().tab_stop(true),
                actions: std::array::from_fn(|_| cx.focus_handle()),
            });
            self.read_row(id, cx);
        }
    }

    /// Reads the item of row `id` under a new stamp; it is checked until
    /// the reads land.
    fn read_row(&mut self, id: DraftId, cx: &mut Context<Self>) {
        let Some(draft) = self.slot(id).and_then(|slot| slot.draft.clone()) else {
            return;
        };
        let stamp = self.generations.next(ViewScope::DraftRow(id));
        let read = self.services.read_job(
            stamp,
            move |reader| read_target(reader, draft),
            cx,
            move |page: &mut Self, stamp, (target, title), cx| {
                page.landed(id, &stamp, target, title, cx)
            },
        );
        if let Some(slot) = self.slots.iter_mut().find(|slot| slot.row.id == id) {
            slot.row.reading();
            slot.read = Some(read);
        }
    }

    fn landed(
        &mut self,
        id: DraftId,
        stamp: &Stamp,
        target: TargetState,
        title: Option<String>,
        cx: &mut Context<Self>,
    ) {
        if !self.generations.is_current(stamp) {
            return;
        }
        if let Some(Slot {
            row,
            draft: Some(draft),
            ..
        }) = self.slots.iter_mut().find(|slot| slot.row.id == id)
        {
            row.landed(draft, target, title.as_deref());
            cx.notify();
        }
    }

    fn slot(&self, id: DraftId) -> Option<&Slot> {
        self.slots.iter().find(|slot| slot.row.id == id)
    }

    /// Show: the host shows and focuses the tab the draft is restored in.
    fn show_tab_of(&mut self, id: DraftId, cx: &mut Context<Self>) {
        if let Some(tab) = self.services.drafts().read(cx).tab_of(id) {
            cx.emit(DraftsEvent::Show(tab));
        }
    }

    /// Retry: the row's item is read again.
    fn retry(&mut self, id: DraftId, cx: &mut Context<Self>) {
        self.read_row(id, cx);
        cx.notify();
    }
}

/// The local time of a recovery save.
fn saved_at(at: OffsetDateTime) -> String {
    at.format(&Rfc3339)
        .map(|at| local_time(&at))
        .unwrap_or_default()
}

/// A read's answer as the classifiers take it: busy is a read error too.
fn answer<T>(read: &ReadResult<T>) -> Result<&T, ResultCode> {
    match read {
        ReadResult::Ok(value) => Ok(value),
        ReadResult::Busy => Err(ResultCode::Busy),
        ReadResult::Failed(failure) => Err(failure.code),
    }
}

/// Where the draft's item stands, and its title as read. A comment draft
/// is classified by its item and its thread, never as an item's draft.
#[allow(clippy::result_large_err)]
async fn read_target(reader: Reader, draft: Draft) -> (TargetState, Option<String>) {
    let root = PathBuf::from(&draft.target.repository);
    let repo = reader
        .call(ReadKind::Resolve, |service| {
            service.resolve_repository(&root)
        })
        .await;
    let repo = match answer(&repo) {
        Ok(repo) => repo,
        Err(code) => return (classify(&draft, Err(code)), None),
    };
    let id = &draft.target.item_id;
    // Classified as the base holds the item, its body as the editor loads
    // it and its `deps` in ID order (R-11, R-72); a comment's draft is
    // checked against the same read.
    let item = match reader
        .call(ReadKind::Item, |service| service.show_item(repo, id))
        .await
    {
        ReadResult::Ok(item) => ReadResult::Ok(normalized(&item)),
        other => other,
    };
    let title = match &item {
        ReadResult::Ok(item) => item.title.clone(),
        _ => None,
    };
    let target = match (&draft.target.comment, answer(&item)) {
        (Some(_), Ok(_)) => {
            let comments = reader
                .call(ReadKind::Comments, |service| {
                    service.list_comments(repo, id)
                })
                .await;
            classify_comment(&draft, answer(&item), answer(&comments))
        }
        (Some(_), Err(code)) => classify_comment(&draft, Err(code), Err(code)),
        (None, item) => classify(&draft, item),
    };
    (target, title)
}

impl Render for DraftsPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.shown {
            return div().id("drafts-hidden").into_any_element();
        }
        self.apply_refocus(window, cx);
        let body = self.body(window, cx);
        v_flex()
            .id("drafts")
            .test_support()
            .gap_3()
            .child(
                div()
                    .id("drafts-heading")
                    .test_support()
                    .aria_label(DRAFTS_HEADING)
                    .text_lg()
                    .font_weight(gpui_kit::FontWeight::BOLD)
                    .child(DRAFTS_HEADING),
            )
            .children(body)
            .into_any_element()
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "drafts_tests.rs"]
mod tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "drafts_routing_tests.rs"]
mod routing_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "drafts_entry_tests.rs"]
mod entry_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "drafts_held_tests.rs"]
mod held_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "drafts_actions_tests.rs"]
mod actions_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "drafts_inspect_tests.rs"]
mod inspect_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "drafts_discard_tests.rs"]
mod discard_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "drafts_privacy_tests.rs"]
mod privacy_tests;
