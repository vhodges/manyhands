//! The key list (Wave 04 Cycle 04 design C2.1; decision 2): bordered, a
//! row per key, each with a radio that puts it in use, its label, origin
//! tag and In use tag, its fingerprint, today's source-state line, Copy
//! public key, the trash and, on generated keys, Delete key file…; then,
//! while a key is in use, Stop using this key. Each control sends what
//! today's did (contract 6).
//!
//! **The radio** is a plain control drawn as a radio, since the Kit's
//! cannot take focus from a route and has no click when checked (fact 39;
//! R-38): a radio button announced checked when its key is in use, a tab
//! stop, named `use_key(label)`. Click, Space or Enter sends `KeySelect`;
//! on the key in use, nothing.
//!
//! **Focus by key ID** (Part G item 4): a row's controls are keyed by their
//! key's ID, never its row. When a key's removal or its file's deletion
//! ends in success, focus on its row goes to the next key's radio, else the
//! previous one's, else Make a new key, so a list whose order changes on
//! the read that follows keeps focus on the key it went to. Only while
//! Settings is open and no dialog is (R-47; A6 F17).

use gpui_kit::{
    ClickEvent, Context, FocusHandle, InteractiveElement, IntoElement, ParentElement, Role,
    StatefulInteractiveElement, Styled, Window,
    accesskit::Toggled,
    component::{
        ActiveTheme, Icon, WindowExt,
        button::{Button, ButtonVariants},
        h_flex, v_flex,
    },
    div,
    prelude::FluentBuilder,
    px,
};
use manyhands::repository::{KeyDto, KeyOwnership, KeyPrivateSourceState, SharedKeyId};

use super::super::{Wrapped, wrapped};
use super::render::{heading, text};
use super::{
    CLEAR, CLEAR_ID, COPY, DELETE, KEYS_HEADING, KEYS_INTRO, KIND_GENERATED, KIND_IMPORTED,
    KeysPage, MAKE_ID, NO_FINGERPRINT, NO_KEYS, READING_KEYS, REMOVE_TOOLTIP, SELECTED,
    SELECTION_NONE, SOURCE_MISSING, SOURCE_UNAVAILABLE, UNREGISTER, selection_line, use_key,
};
use crate::desktop::{
    assets::icon_path,
    model::{list_state::LABEL_BUSY, read_state::Read},
};

const RADIO: &str = "key-radio";
const COPY_ID: &str = "key-copy";
const REMOVE_ID: &str = "key-remove";
pub(super) const DELETE_ID: &str = "key-delete-file";
/// A row's controls, each keyed by its key's ID.
const ROW_CONTROLS: [&str; 4] = [RADIO, COPY_ID, REMOVE_ID, DELETE_ID];
/// The radio's size, and its dot's.
const RADIO_SIZE: f32 = 16.;
const DOT_SIZE: f32 = 8.;
/// The trash icon's size.
const TRASH_SIZE: f32 = 14.;
/// The key list's corner radius (`05`).
const LIST_RADIUS: f32 = 8.;

/// The focus key of `key`'s `control`.
fn row_key(control: &str, key: &str) -> String {
    format!("{control}-{key}")
}

/// The element ID of the Kit button inside `key`'s `control`. A Kit button
/// keeps its focus as state keyed by its ID, so it is keyed by the key's ID
/// too: a row that goes never hands its button's focus to the key after it.
fn button_id(control: &str, key: &str) -> gpui_kit::ElementId {
    gpui_kit::ElementId::Name(format!("{control}-button-{key}").into())
}

impl KeysPage {
    /// The focus of `key`'s `control`, made the first time it is asked for.
    /// The radio is a tab stop of its own; the others are the wrappers of a
    /// Kit button, whose own focus is the tab stop.
    pub(super) fn row_focus(
        &mut self,
        control: &str,
        key: &str,
        cx: &mut Context<Self>,
    ) -> FocusHandle {
        self.focus
            .entry(row_key(control, key))
            .or_insert_with(|| cx.focus_handle().tab_stop(control == RADIO))
            .clone()
    }

    /// The radio of the row `ix`, `key`'s.
    fn radio(
        &mut self,
        ix: usize,
        key: &KeyDto,
        id: SharedKeyId,
        cx: &mut Context<Self>,
    ) -> Wrapped {
        let focus = self.row_focus(RADIO, &key.id, cx);
        let theme = cx.theme();
        let (border, ring, fill) = (theme.border, theme.ring, theme.primary);
        let dot = key
            .selected
            .then(|| div().size(px(DOT_SIZE)).rounded_full().bg(fill));
        wrapped(
            (RADIO, ix),
            use_key(&key.label),
            &focus,
            &self.scroll,
            div().children(dot),
        )
        .role(Role::RadioButton)
        .aria_toggled(if key.selected {
            Toggled::True
        } else {
            Toggled::False
        })
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .size(px(RADIO_SIZE))
        .rounded_full()
        .border_1()
        .border_color(border)
        .focus(|style| style.border_2().border_color(ring))
        .on_click(
            cx.listener(move |this, _: &ClickEvent, window, cx| this.submit_select(id, window, cx)),
        )
    }

    /// The row `ix`'s Kit button `control`, `key`'s, named `label`, in its
    /// wrapper.
    fn row_button(
        &mut self,
        control: &'static str,
        ix: usize,
        key: &str,
        label: &'static str,
        button: Button,
        cx: &mut Context<Self>,
    ) -> Wrapped {
        let focus = self.row_focus(control, key, cx);
        wrapped((control, ix), label, &focus, &self.scroll, button)
    }

    fn key_row(
        &mut self,
        ix: usize,
        key: &KeyDto,
        last: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();
        let (muted, border, mono) = (
            theme.muted_foreground,
            theme.border,
            theme.mono_font_family.clone(),
        );
        let kind = match key.ownership {
            KeyOwnership::Generated => KIND_GENERATED,
            KeyOwnership::Imported => KIND_IMPORTED,
        };
        let source = match key.private_source_state {
            KeyPrivateSourceState::Available => None,
            KeyPrivateSourceState::Missing => Some(SOURCE_MISSING),
            KeyPrivateSourceState::Unavailable => Some(SOURCE_UNAVAILABLE),
        };
        let mut radio = None;
        let mut actions = h_flex().gap_2().flex_wrap();
        if let Ok(id) = SharedKeyId::parse(&key.id) {
            radio = Some(self.radio(ix, key, id, cx));
            if key.public_key_path.is_some() {
                let button = Button::new(button_id(COPY_ID, &key.id))
                    .label(COPY)
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.copy(id, window, cx)
                    }));
                actions = actions.child(self.row_button(COPY_ID, ix, &key.id, COPY, button, cx));
            }
            let trash = Button::new(button_id(REMOVE_ID, &key.id))
                .ghost()
                .icon(
                    Icon::default()
                        .path(icon_path("trash"))
                        .w(px(TRASH_SIZE))
                        .h(px(TRASH_SIZE)),
                )
                .accessibility_label(UNREGISTER)
                .tooltip(REMOVE_TOOLTIP)
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    this.submit_unregister(id, window, cx)
                }));
            actions = actions.child(self.row_button(REMOVE_ID, ix, &key.id, UNREGISTER, trash, cx));
            if key.ownership == KeyOwnership::Generated {
                let button = Button::new(button_id(DELETE_ID, &key.id))
                    .danger()
                    .label(DELETE)
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.submit_delete(id, window, cx)
                    }));
                actions =
                    actions.child(self.row_button(DELETE_ID, ix, &key.id, DELETE, button, cx));
            }
        }
        let tags = h_flex()
            .gap_2()
            .flex_wrap()
            .child(
                div()
                    .font_weight(gpui_kit::FontWeight::BOLD)
                    .child(text(("key-label", ix), key.label.clone())),
            )
            .child(div().text_color(muted).child(text(("key-kind", ix), kind)))
            .children(key.selected.then(|| text(("key-selected", ix), SELECTED)));
        h_flex()
            .items_start()
            .gap_3()
            .p_3()
            .when(!last, |row| row.border_b_1().border_color(border))
            .children(radio)
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap_0p5()
                    .child(tags)
                    .child(
                        div().font_family(mono).child(text(
                            ("key-fingerprint", ix),
                            key.fingerprint
                                .clone()
                                .unwrap_or_else(|| NO_FINGERPRINT.to_owned()),
                        )),
                    )
                    .children(source.map(|source| text(("key-source", ix), source)))
                    .child(actions),
            )
    }

    /// The heading, `05`'s intro, which key is in use, the key list and,
    /// while a key is in use, Stop using this key.
    pub(super) fn key_rows(&mut self, cx: &mut Context<Self>) -> gpui_kit::Div {
        let theme = cx.theme();
        let (muted, border) = (theme.muted_foreground, theme.border);
        let body = v_flex()
            .gap_2()
            .child(heading("keys-heading", KEYS_HEADING))
            .child(
                div()
                    .text_color(muted)
                    .child(text("keys-intro", KEYS_INTRO)),
            );
        let keys = match &self.keys {
            Read::None => return body,
            Read::Loading => return body.child(text("keys-state", READING_KEYS)),
            Read::Busy => return body.child(text("keys-state", LABEL_BUSY)),
            Read::Failed(code) => {
                return body
                    .child(text("keys-state", code.message()))
                    .child(div().text_xs().text_color(muted).child(code.as_str()));
            }
            Read::Ready(keys) => keys.clone(),
        };
        let in_use = keys.iter().find(|key| key.selected);
        let selection = match in_use {
            Some(key) => selection_line(&key.label),
            None => SELECTION_NONE.to_owned(),
        };
        let mut body = body.child(text("keys-selection", selection));
        if keys.is_empty() {
            return body.child(text("keys-state", NO_KEYS));
        }
        let mut list = v_flex()
            .id("keys-list")
            .w_full()
            .border_1()
            .border_color(border)
            .rounded(px(LIST_RADIUS));
        for (ix, key) in keys.iter().enumerate() {
            list = list.child(self.key_row(ix, key, ix + 1 == keys.len(), cx));
        }
        body = body.child(list);
        if in_use.is_some() {
            let stop = self.button(
                CLEAR_ID,
                CLEAR_ID.to_owned(),
                CLEAR,
                |this, window, cx| this.submit_clear(window, cx),
                cx,
            );
            body = body.child(div().child(stop));
        }
        body
    }

    /// Where the key list lands (R-22): the radio of the key in use, else
    /// the first key's, else Make a new key, each drawn once the keys are
    /// read; `None` while they are still read, when none of it is.
    fn list_target(&mut self, cx: &mut Context<Self>) -> Option<FocusHandle> {
        let keys = match &self.keys {
            Read::None | Read::Loading => return None,
            Read::Ready(keys) => keys.clone(),
            Read::Busy | Read::Failed(_) => Vec::new(),
        };
        // Only a key whose ID parses has a radio (`key_row`).
        let drawn: Vec<&KeyDto> = keys
            .iter()
            .filter(|key| SharedKeyId::parse(&key.id).is_ok())
            .collect();
        let key = drawn
            .iter()
            .find(|key| key.selected)
            .or_else(|| drawn.first());
        Some(match key {
            Some(key) => self.row_focus(RADIO, &key.id, cx),
            None => self.focus[MAKE_ID].clone(),
        })
    }

    /// A key-list route (Part D; R-22): the control it lands on, drawn in
    /// the next frame, else `None` while the keys are still read, for the
    /// caller to focus the page's `entry`. The page reads its keys again as
    /// it is shown; when that read lands, focus moves once to what it
    /// gives, if what the route focused still holds focus (`land_retarget`).
    pub(crate) fn list_focus(
        &mut self,
        entry: FocusHandle,
        cx: &mut Context<Self>,
    ) -> Option<FocusHandle> {
        let target = self.list_target(cx);
        self.retarget = Some(target.clone().unwrap_or(entry));
        target
    }

    /// The keys' read landed: a key-list route's one retarget (R-22), only
    /// while Settings is open (design A4) and focus is where the route put
    /// it.
    pub(super) fn land_retarget(&mut self, cx: &mut Context<Self>) {
        let Some(held) = self.retarget.take() else {
            return;
        };
        let Some(target) = self.list_target(cx).filter(|target| *target != held) else {
            return;
        };
        let open = self.open.clone();
        let _ = self.window.update(cx, |_, window, cx| {
            if open.is_open(window, cx) && held.is_focused(window) {
                window.focus(&target, cx);
            }
        });
    }

    /// A key-list route's retarget is no longer wanted: Settings closed, or
    /// left Sharing & keys.
    pub(crate) fn forget_retarget(&mut self) {
        self.retarget = None;
    }

    /// A key removed, or its file deleted, while its row is still listed:
    /// focus on the row, where the confirmation closed onto, goes to the
    /// next key's radio by key ID, else the previous key's, else Make a new
    /// key. Focus elsewhere stays. Only while Settings is open and no
    /// dialog is (R-47; A6 F17).
    pub(super) fn refocus_removed(
        &mut self,
        removed: SharedKeyId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if window.has_active_dialog(cx) || !self.open.is_open(window, cx) {
            return;
        }
        let Read::Ready(keys) = &self.keys else {
            return;
        };
        let removed = removed.to_string();
        let Some(at) = keys.iter().position(|key| key.id == removed) else {
            return;
        };
        let on_row = ROW_CONTROLS
            .iter()
            .filter_map(|control| self.focus.get(&row_key(control, &removed)))
            .any(|handle| handle.contains_focused(window, cx));
        if !on_row {
            return;
        }
        let next = keys
            .get(at + 1)
            .or_else(|| at.checked_sub(1).and_then(|before| keys.get(before)))
            .map(|key| key.id.clone());
        let target = match next {
            Some(key) => self.row_focus(RADIO, &key, cx),
            None => self.focus[MAKE_ID].clone(),
        };
        window.focus(&target, cx);
    }
}
