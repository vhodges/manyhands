//! A document's details (design, Details sheet, Document): Title,
//! required; Status, Date and Author from its other keys, exactly as the
//! library gives them and never editable (fact 58, decision 14); its
//! Folder, with Move…; the remaining keys as Other fields; the Facts, the
//! Problems and the Inspect link.
//!
//! **Move…** (design, Part E: Move; R-23) is offered only while the tab
//! views the document as read, with no edit session, live or kept, no
//! draft of it kept or stored, no save of it in flight, and no release of
//! its context queued, running or waiting (`Releases::pending`). Otherwise
//! it is disabled, and says why. A new document's stand-in is never read,
//! so it is not moved before it is created and read. The dialog is
//! `view::move_document`'s.

use gpui_kit::{
    AnyElement, App, ClickEvent, Context, InteractiveElement, IntoElement, ParentElement, Role,
    StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{ActiveTheme, Disableable, Sizable, WindowExt, button::Button, h_flex, v_flex},
    div,
};
use manyhands::repository::{ItemDto, ItemDtoKind};

use super::{super::header::folder_and_file, Row};
use crate::desktop::{
    model::{
        edit::Phase,
        item::{NOT_REPRESENTABLE, metadata_value},
        navigation::OpenTarget,
        tabs::TabKey,
    },
    services::created_contexts::ContextKey,
    view::{
        item::{ChipField, ItemPane, text},
        move_document::{self, MOVE, MOVE_WAITS, MOVE_WHILE_VIEWING, Moved, Source, context_of},
    },
};

const STATUS: Row = Row::new("sheet-status", "Status");
const DATE: Row = Row::new("sheet-date", "Date");
const AUTHOR: Row = Row::new("sheet-author", "Author");
const FOLDER: Row = Row::new("sheet-folder", "Folder");

/// The other keys a document's sheet shows in rows of their own.
const NAMED: [&str; 3] = ["status", "date", "author"];

/// The value of the other key `key`, as the library gives it.
fn key_value(item: &ItemDto, id: &'static str, key: &str) -> Option<AnyElement> {
    let shown = metadata_value(item.unknown_metadata.get(key)?)
        .unwrap_or_else(|| NOT_REPRESENTABLE.to_owned());
    Some(text(id, shown).into_any_element())
}

impl ItemPane {
    pub(super) fn document_rows(
        &mut self,
        item: &ItemDto,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let mut rows = vec![self.title_row(item, window, cx)];
        for (row, id, key, field) in [
            (STATUS, "item-status", NAMED[0], ChipField::Status),
            (DATE, "item-date", NAMED[1], ChipField::Date),
            (AUTHOR, "item-author", NAMED[2], ChipField::Author),
        ] {
            let focus = self.sheet_focus[field.index()].clone();
            rows.push(self.row(row, key_value(item, id, key), Some(&focus), window, cx));
        }
        rows.push(self.folder_row(item, window, cx));
        rows.push(self.other_fields(item, &NAMED, cx));
        rows.push(self.facts(item, ItemDtoKind::Document, window, cx));
        rows.extend(self.file_rows(item, cx));
        rows
    }

    /// The Folder, with Move… beside it, and, while Move… is not offered,
    /// why.
    fn folder_row(&self, item: &ItemDto, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let (folder, _) = folder_and_file(&item.path);
        let folder = folder.map(|folder| text("item-folder", folder).into_any_element());
        let blocked = self.move_blocked(cx);
        let mut control = Button::new("sheet-move-button")
            .label(MOVE)
            .small()
            .disabled(blocked.is_some())
            .on_click(cx.listener(|pane, _: &ClickEvent, window, cx| pane.open_move(window, cx)));
        let mut wrapper = div()
            .id("sheet-move")
            .test_support()
            .role(Role::Group)
            .aria_label(MOVE);
        if let Some(note) = blocked {
            control = control.tooltip(note);
            wrapper = wrapper.aria_description(note);
        }
        let muted = cx.theme().muted_foreground;
        v_flex()
            .gap_1()
            .child(
                h_flex()
                    .gap_2()
                    .items_end()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(self.row(FOLDER, folder, None, window, cx)),
                    )
                    .child(wrapper.child(control)),
            )
            .children(blocked.map(|note| text("sheet-move-note", note).text_xs().text_color(muted)))
            .into_any_element()
    }

    /// R-23: why Move… is not offered now, in the ruled words, or `None`
    /// while it is. It needs the document as read: a new document's
    /// stand-in, shown before its first read, is never moved.
    pub(in crate::desktop::view) fn move_blocked(&self, cx: &App) -> Option<&'static str> {
        let (TabKey::Item { repo, kind, id }, Some(_)) = (self.tab_key(), self.model.item().rows())
        else {
            return Some(MOVE_WHILE_VIEWING);
        };
        let drafts = self.services.drafts().read(cx);
        let saving = drafts.saving_item(|target| {
            target.comment.is_none()
                && target.repository == repo
                && target.item_id.to_string() == id
        });
        // A stored draft is known only from a listing that is ready: while
        // it loads or has failed, Move waits, as a release does (Part E
        // review, M1). A store that did not open is never listed, and
        // keeps no draft: Move does not wait for it (fix round 2).
        let known = drafts.unavailable().is_some() || drafts.listing().rows().is_some();
        if !known
            || self.edit.phase() != Phase::Viewing
            || self.has_session()
            || self.draft.restoring()
            || self.draft_kept(cx)
            || saving
        {
            return Some(MOVE_WHILE_VIEWING);
        }
        let key = ContextKey::new(&repo, kind, &id);
        self.services
            .releases()
            .read(cx)
            .pending(&key)
            .then_some(MOVE_WAITS)
    }

    /// Move…: its dialog, over the sheet, while R-23 holds. Moved, the
    /// dialog closes; the tab reads its item again by its ID as any finished
    /// move of its repository makes it (R-B2).
    fn open_move(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_dialog(cx) || self.move_blocked(cx).is_some() {
            return;
        }
        let (OpenTarget::Item { repo, id, .. }, Some(item)) =
            (self.model.target(), self.model.item().rows())
        else {
            return;
        };
        let source = Source {
            root: repo.clone(),
            id: id.clone(),
            path: item.path.clone(),
            context: context_of(item),
        };
        let pane = cx.entity().downgrade();
        let Some(dialog) = move_document::open(source, pane, &self.services, window, cx) else {
            return;
        };
        cx.subscribe_in(&dialog, window, |_, dialog, _: &Moved, window, cx| {
            dialog.update(cx, |dialog, cx| dialog.finish(window, cx));
        })
        .detach();
    }
}
