//! Retire… on an item the loop left alone (D6 design E4, R-18, decisions 15
//! and 18, A19, A20, W-6).
//!
//! The attempt's entry lists each reported `Closed` or `Promoted` item that
//! was not retired, by its short code, with Retire…. Pressing it:
//!
//! 1. checks that no retirement of the item is queued, running or pending
//!    (RT-1), and ignores a second press while the first is in progress;
//! 2. stops, with nothing submitted, if a tab of the item holds text that no
//!    draft keeps: Show tab or Cancel, never "Retire anyway";
//! 3. lists the draft store again, and asks if the item has kept drafts (or
//!    they cannot be known): Retire or Cancel, the default;
//! 4. marks the item "Retiring…" in one UI update, which also ends a clean
//!    edit session and keeps a dirty one (the tabs read the lock), flushes
//!    the item's draft sessions, and stops if one cannot be written;
//! 5. submits `Intent::ContextRetire` on the user lane under a new request
//!    ID, which may prompt, and removes the item from the refused set.
//!
//! The tabs are read here, at the view, never from inside the scheduler's
//! update: a scheduler update that read a pane from inside that pane's
//! update would panic.

use gpui_kit::{
    AnyElement, ClickEvent, Context, InteractiveElement, IntoElement, ParentElement, Styled,
    Window,
    component::{WindowExt, h_flex},
    div,
};
use manyhands::{canonical::ItemId, drafts::DraftErrorKind, repository::ItemDtoKind};

use super::{button, text};
use crate::desktop::{
    model::{
        navigation::OpenTarget,
        polling::{
            Root, TargetKind,
            entry::{RETIRE, RetireOffer, short_code},
        },
        tabs::TabKey,
    },
    services::drafts::{Fresh, Kept},
    view::{
        retire_asks::{RetireChoice, ask_kept, ask_unkept},
        workspace::{Region, Workspace},
    },
};

fn dto_kind(kind: TargetKind) -> ItemDtoKind {
    match kind {
        TargetKind::Ticket => ItemDtoKind::Ticket,
        TargetKind::Document => ItemDtoKind::Document,
    }
}

/// Where a Retire… press stands while it is in progress.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::desktop::view) enum RetireStep {
    /// Listing the draft store, or flushing the item's drafts: nothing the
    /// person can answer.
    Working,
    /// A question is open. If no dialog is open any more, something closed
    /// it without an answer (quitting closes every dialog), and the press
    /// is over.
    Asking,
}

/// The item a Retire… is for, in the shape the tabs and the lock know it.
#[derive(Clone)]
pub(in crate::desktop::view) struct Subject {
    pub root: Root,
    pub kind: TargetKind,
    pub item: ItemId,
}

impl Subject {
    fn tab(&self) -> TabKey {
        TabKey::Item {
            repo: self.root.clone(),
            kind: dto_kind(self.kind),
            id: self.item.to_string(),
        }
    }
}

impl Workspace {
    /// The row of one item Retire… is offered for: its short code and
    /// words, and the action, in one row that wraps so the action stays
    /// inside the area at the default window (W-6). The ids are keyed by
    /// the repository and the item, not the place, so a focused action
    /// stays on its own item when an earlier one goes.
    pub(super) fn retire_offer(
        &mut self,
        root: &Root,
        offer: &RetireOffer,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let short = short_code(&offer.item);
        let element = format!("attempt-{root}-retire-{short}");
        let focus = self.control_focus_for(element.clone(), cx);
        // The action scrolls into view when focus reaches it (E-2).
        let marker = self.operations_scroll().marker_within(&focus);
        let subject = Subject {
            root: root.clone(),
            kind: offer.kind,
            item: offer.item.clone(),
        };
        let control = button(
            element.clone(),
            RETIRE,
            Some(&focus),
            cx,
            cx.listener(move |this, _: &ClickEvent, window, cx| {
                this.retire_chosen(subject.clone(), window, cx)
            }),
        );
        h_flex()
            .gap_2()
            .flex_wrap()
            .items_center()
            .child(text(format!("attempt-{root}-offer-{short}"), offer.text()))
            .child(
                div()
                    .id(format!("{element}-slot"))
                    .relative()
                    .child(control)
                    .child(marker),
            )
            .into_any_element()
    }

    /// Retire… was pressed (E4, steps 1 and 2).
    pub(in crate::desktop::view) fn retire_chosen(
        &mut self,
        subject: Subject,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key = (subject.root.clone(), subject.item.to_string());
        let dto = dto_kind(subject.kind);
        if self.services().worker().quitting()
            || self.services().retiring(&subject.root, dto, &key.1, cx)
        {
            return;
        }
        // A question that was closed without an answer leaves no press.
        if self.retire_asked.get(&key) == Some(&RetireStep::Asking) && !window.has_active_dialog(cx)
        {
            self.retire_asked.remove(&key);
        }
        if self.retire_asked.contains_key(&key) {
            return;
        }
        self.retire_asked.insert(key, RetireStep::Working);
        // Unkept text stops it: read at the view, across the windows' tabs.
        let unkept = self
            .services()
            .edit_tabs()
            .read(cx)
            .unkept_count(&subject.tab(), cx);
        if unkept > 0 {
            return self.retire_unkept(subject, unkept, window, cx);
        }
        // The draft store is listed again, off the UI thread (step 3).
        let listing = self
            .services()
            .drafts()
            .clone()
            .update(cx, |registry, cx| registry.relist(cx));
        cx.spawn_in(window, async move |this, cx| {
            let fresh = listing.await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.retire_listed(subject, fresh, window, cx)
            });
        })
        .detach();
    }

    /// The press is over: another can start.
    fn retire_done(&mut self, subject: &Subject) {
        self.retire_asked
            .remove(&(subject.root.clone(), subject.item.to_string()));
    }

    /// A question is about to open for the press.
    fn retire_asking(&mut self, subject: &Subject) {
        self.retire_asked.insert(
            (subject.root.clone(), subject.item.to_string()),
            RetireStep::Asking,
        );
    }

    /// Step 2: the dialog for text no draft keeps.
    fn retire_unkept(
        &mut self,
        subject: Subject,
        count: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.retire_asking(&subject);
        let workspace = cx.entity().downgrade();
        ask_unkept(count, window, cx, move |choice, window, cx| {
            let _ = workspace.update(cx, |workspace, cx| {
                workspace.retire_done(&subject);
                if choice == RetireChoice::ShowTab {
                    workspace.show_item_tab(&subject, window, cx)
                }
            });
        });
    }

    /// Shows the tab of the item, opening it if it is not open here.
    fn show_item_tab(&mut self, subject: &Subject, window: &mut Window, cx: &mut Context<Self>) {
        match self.tab_of(&subject.tab()) {
            Some(tab) => self.show_tab(tab, cx),
            None => self.open_item(
                OpenTarget::Item {
                    repo: subject.root.clone(),
                    kind: dto_kind(subject.kind),
                    id: subject.item.clone(),
                },
                cx,
            ),
        }
        self.focus_region(Region::Content, window, cx);
    }

    /// Step 3: the fresh listing is in. An item with kept drafts, or whose
    /// drafts cannot be known, asks first.
    fn retire_listed(
        &mut self,
        subject: Subject,
        fresh: Fresh,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let kept = self.services().drafts().read(cx).kept_of(
            &subject.root,
            dto_kind(subject.kind),
            &subject.item,
        );
        // A listing a newer one overtook is not known to be fresh.
        let kept = match (kept, fresh) {
            (Kept::Counted { item, comments }, Fresh::Landed) => Some(item + comments),
            _ => None,
        };
        if kept == Some(0) {
            return self.retire_commit(subject, window, cx);
        }
        self.retire_asking(&subject);
        let workspace = cx.entity().downgrade();
        let asked = subject.clone();
        ask_kept(subject.kind, kept, window, cx, move |choice, window, cx| {
            let _ = workspace.update(cx, |workspace, cx| match choice {
                RetireChoice::Retire => workspace.retire_commit(asked.clone(), window, cx),
                _ => workspace.retire_done(&asked),
            });
        });
    }

    /// Step 4: nothing holds the item back. RT-1 and the unkept text are
    /// checked once more, as a window or the person may have changed them
    /// meanwhile; the item is marked "Retiring…" in this update, and its
    /// draft sessions are flushed while it is read-only.
    fn retire_commit(&mut self, subject: Subject, window: &mut Window, cx: &mut Context<Self>) {
        let dto = dto_kind(subject.kind);
        let id = subject.item.to_string();
        if self.services().retiring(&subject.root, dto, &id, cx) {
            return self.retire_done(&subject);
        }
        self.retire_asked.insert(
            (subject.root.clone(), subject.item.to_string()),
            RetireStep::Working,
        );
        let unkept = self
            .services()
            .edit_tabs()
            .read(cx)
            .unkept_count(&subject.tab(), cx);
        if unkept > 0 {
            return self.retire_unkept(subject, unkept, window, cx);
        }
        let key = (subject.root.clone(), dto, id);
        let mark = self
            .services()
            .operations()
            .clone()
            .update(cx, |operations, cx| {
                let mark = operations.mark_retiring(key);
                cx.notify();
                mark
            });
        let flush = self.services().drafts().clone().update(cx, |registry, cx| {
            registry.flush_item(&subject.root, dto, &subject.item, cx)
        });
        cx.spawn_in(window, async move |this, cx| {
            let failed = flush.await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.retire_flushed(subject, mark, failed, window, cx)
            });
        })
        .detach();
    }

    /// Step 5: the item's drafts are written. If one could not be, the text
    /// is not kept, and nothing is submitted. Otherwise the retirement goes
    /// on the user lane, and the mark is cleared once the worker holds it.
    fn retire_flushed(
        &mut self,
        subject: Subject,
        mark: u64,
        failed: Vec<(manyhands::drafts::DraftId, DraftErrorKind)>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if failed.is_empty() {
            let scheduler = self.services().scheduler().clone();
            let (root, kind, item) = (subject.root.clone(), subject.kind, subject.item.clone());
            scheduler.update(cx, |scheduler, cx| {
                scheduler.retire_chosen(&root, kind, &item, cx)
            });
        }
        self.services()
            .operations()
            .clone()
            .update(cx, |operations, cx| {
                if operations.unmark_retiring(mark) {
                    cx.notify();
                }
            });
        if failed.is_empty() {
            self.retire_done(&subject);
        } else {
            let count = failed.len();
            self.retire_unkept(subject, count, window, cx);
        }
        cx.notify();
    }
}
