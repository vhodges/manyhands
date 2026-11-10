//! The attempt entry in the operation area (D6 design A4, R-20, R-23, W-6):
//! one entry per repository for its latest attempt, in "This session" after
//! what needs attention, replaced by the next attempt and absent when the
//! attempt has nothing to say. It names the repository, says what was
//! brought in, and lists each item the poll left alone with its text. An
//! unpublished item offers Share…, which sends the share the item header's
//! Share sends. A disposition itself is only text: nothing is sent until the
//! person presses an action. Each row wraps, so its action stays inside the
//! area at the default window, and carries a scroll marker so Tab brings it
//! into view (W-6, E-2).
//!
//! The ids of an entry's parts are keyed by its repository, not its place,
//! so a focused action stays on its own item when an earlier entry comes or
//! goes.
//!
//! The entry is where the retirement summary (Task 11a), the items Retire…
//! is offered for (`retire`, E4) and the failure of an attempt join:
//! `attempt_entry` in the model decides what is worth showing, and each
//! adds its rows after the listed ones.
//!
//! Text is the library's fixed words and an item's ID; no title, body or
//! comment is named.

use std::path::{Path, PathBuf};

use gpui_kit::{
    AnyElement, ClickEvent, Context, InteractiveElement, IntoElement, ParentElement, Styled,
    component::{ActiveTheme, h_flex, v_flex},
    div,
};
use manyhands::{
    canonical::ItemId,
    repository::{AuthoringKind, ItemDtoKind},
};

use super::{button, scope_name, text, unavailable};
use crate::desktop::{
    model::polling::{
        Root, TargetKind,
        entry::{AttemptEntry, EntryLine, LineAction, attempt_entry},
    },
    services::requests::{Intent, Lane},
    view::{item::locked_text, workspace::Workspace},
};

impl Workspace {
    /// The entries of the repositories whose latest attempt has something
    /// to say, in the order of their roots.
    pub(super) fn attempt_entries(&mut self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let services = self.services().clone();
        let entries: Vec<_> = services
            .scheduler()
            .read(cx)
            .schedule()
            .attempts()
            .filter_map(|(root, summary, retirements)| {
                // Retire… is not offered while a retirement of the item is
                // queued, running or pending, or marked (RT-1).
                let offers = services
                    .scheduler()
                    .read(cx)
                    .schedule()
                    .retire_offers(root)
                    .into_iter()
                    .filter(|offer| {
                        let kind = match offer.kind {
                            TargetKind::Ticket => ItemDtoKind::Ticket,
                            TargetKind::Document => ItemDtoKind::Document,
                        };
                        !services.retiring(root, kind, &offer.item.to_string(), cx)
                    })
                    .collect();
                Some((
                    root.clone(),
                    AttemptEntry::with_offers(attempt_entry(summary, retirements), offers)?,
                ))
            })
            .collect();
        let muted = cx.theme().muted_foreground;
        let mut shown = Vec::with_capacity(entries.len());
        for (root, entry) in entries {
            let mut rows = v_flex().id(format!("attempt-{root}")).gap_1().child(text(
                format!("attempt-{root}-repository"),
                scope_name(Some(Path::new(&root))),
            ));
            if let Some(brought_in) = entry.brought_in {
                rows = rows.child(
                    text(format!("attempt-{root}-brought-in"), brought_in)
                        .text_xs()
                        .text_color(muted),
                );
            }
            for (i, line) in entry.lines.iter().enumerate() {
                rows = rows.child(self.attempt_line(i, &root, line, cx));
            }
            // What came of the retirements since (E3): one row each, as
            // wide as the entry, so a long one wraps inside it (W-6).
            for (i, line) in entry.retirements.iter().enumerate() {
                rows = rows
                    .child(text(format!("attempt-{root}-retirement-{i}"), line.text()).w_full());
            }
            // The items the loop left alone, each with Retire… (E4).
            for offer in &entry.offers {
                rows = rows.child(self.retire_offer(&root, offer, cx));
            }
            shown.push(rows.into_any_element());
        }
        shown
    }

    /// One listed item: its ID, its text and its action, in one row that
    /// wraps.
    fn attempt_line(
        &mut self,
        i: usize,
        root: &Root,
        line: &EntryLine,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let mut row = h_flex().gap_2().flex_wrap().items_center();
        if let Some(id) = &line.item_id {
            row = row.child(
                text(format!("attempt-{root}-item-{i}"), id.to_string())
                    .text_xs()
                    .text_color(muted),
            );
        }
        row = row.child(text(format!("attempt-{root}-line-{i}"), line.text));
        if let (Some(LineAction::Share), Some(id), Some(kind)) =
            (line.action, line.item_id.clone(), line.kind)
        {
            row = row.child(self.share_action(i, root, kind, id, cx));
        }
        row.into_any_element()
    }

    /// Share…, or "Sharing…" while a share of the item is queued or running.
    fn share_action(
        &mut self,
        i: usize,
        root: &Root,
        kind: TargetKind,
        id: ItemId,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let intent = share_of(root, kind, &id);
        let element = format!("attempt-{root}-action-{i}");
        // While the item's close or merge is unfinished, Share… waits, as
        // the item header's Share does (D5 A2, R-23): no control, and the
        // lock's words say why.
        let dto = match kind {
            TargetKind::Ticket => ItemDtoKind::Ticket,
            TargetKind::Document => ItemDtoKind::Document,
        };
        let lock = self.services().lock(root, dto, &id.to_string(), cx);
        if lock.blocks() {
            return unavailable(
                element,
                LineAction::Share.label(),
                locked_text(lock, dto),
                cx,
            )
            .into_any_element();
        }
        if self.sharing(&intent) {
            return text(element, "Sharing\u{2026}")
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .into_any_element();
        }
        let focus = self.control_focus_for(element.clone(), cx);
        // The action scrolls into view when focus reaches it (E-2).
        let marker = self.operations_scroll().marker_within(&focus);
        let root = root.clone();
        let control = button(
            element.clone(),
            LineAction::Share.label(),
            Some(&focus),
            cx,
            cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.share_unpublished(share_of(&root, kind, &id), cx)
            }),
        );
        div()
            .id(format!("{element}-slot"))
            .relative()
            .child(control)
            .child(marker)
            .into_any_element()
    }

    /// Whether `share` is queued, waiting out `busy` or running: the
    /// worker's own account, as the item header's Share reads it.
    fn sharing(&self, share: &Intent) -> bool {
        let Intent::ItemSync { root, kind, id } = share else {
            return false;
        };
        self.services().worker().in_flight(|intent| {
            matches!(
                intent,
                Intent::ItemSync { root: asked, kind: asked_kind, id: asked_id }
                    if asked == root && asked_kind == kind && asked_id == id
            )
        })
    }

    /// Share… pressed on an unpublished item: sends the share the item's
    /// header sends (`item sync`), on the user lane, unless one is already
    /// queued or running. A poll runs only with a publication remote, so
    /// there is no remote to set up here.
    fn share_unpublished(&mut self, share: Intent, cx: &mut Context<Self>) {
        if self.sharing(&share) {
            return;
        }
        self.services()
            .worker()
            .submit(share, Lane::User, None, None);
        cx.notify();
    }
}

/// The share of the item `id` of the repository `root`.
fn share_of(root: &Root, kind: TargetKind, id: &ItemId) -> Intent {
    Intent::ItemSync {
        root: PathBuf::from(root),
        kind: match kind {
            TargetKind::Ticket => AuthoringKind::Ticket,
            TargetKind::Document => AuthoringKind::Document,
        },
        id: id.clone(),
    }
}
