//! What Repair draws (`super`): the tab's Repair… or Adopt…, or why its
//! file cannot be repaired here; an unmanaged document's banner; and the
//! repair view, a plain text area over the whole file in the page's place
//! of its text, with what the last answer said, Preview and Cancel.

use gpui_kit::{
    AnyElement, ClickEvent, Context, Focusable, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, TestSupportExt,
    component::{
        ActiveTheme, Disableable, Sizable,
        button::{Button, ButtonVariants},
        h_flex,
        input::{IndentInline, OutdentInline, Textarea},
        v_flex,
    },
    div, px,
};
use manyhands::repository::ItemDto;

use super::{Offer, PREVIEW, REPAIR_NOT_UTF8, UNMANAGED};
use crate::desktop::{
    model::item::ADOPT,
    view::{
        confirm::CANCEL,
        item::{ItemPane, SOURCE, text},
    },
};

/// The repair view's text area's height.
const TEXT_HEIGHT: f32 = 420.;

impl ItemPane {
    /// `label`, Repair… or Adopt…, for `item`, or why the file cannot be
    /// repaired here; nothing where neither applies.
    pub(in crate::desktop::view::item) fn repair_entry(
        &self,
        item: &ItemDto,
        label: &'static str,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        Some(match self.repair_offer(item)? {
            Offer::Repair => div()
                .id("item-repair")
                .test_support()
                .aria_label(label)
                .flex_none()
                .child(
                    Button::new("item-repair-button")
                        .label(label)
                        .small()
                        .on_click(cx.listener(|pane, _: &ClickEvent, window, cx| {
                            pane.open_repair(window, cx)
                        })),
                )
                .into_any_element(),
            Offer::NotUtf8 => text("item-repair-not-utf8", REPAIR_NOT_UTF8)
                .text_color(cx.theme().muted_foreground)
                .into_any_element(),
        })
    }

    /// An unmanaged document's banner: the handoff's words, and Adopt….
    pub(in crate::desktop::view::item) fn unmanaged_banner(
        &self,
        item: &ItemDto,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        h_flex()
            .flex_wrap()
            .gap_3()
            .items_start()
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .child(text("item-unmanaged", UNMANAGED)),
            )
            .children(self.repair_entry(item, ADOPT, cx))
            .into_any_element()
    }

    /// The repair view, in the page's place of the file's text: the text
    /// area, which Tab and Shift-Tab leave as D1's source view's do, what
    /// the last answer said, then Preview and Cancel. While a repair runs,
    /// the text is read-only and both wait.
    pub(in crate::desktop::view::item) fn repair_view(
        &self,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let repair = self.repair.as_ref()?;
        let theme = cx.theme();
        let (danger, muted, mono) = (
            theme.danger,
            theme.muted_foreground,
            theme.mono_font_family.clone(),
        );
        let running = repair.running;
        let failure = repair.failure.as_ref().map(|(message, code)| {
            v_flex()
                .gap_1()
                .child(
                    text("repair-failure", message.clone())
                        .text_sm()
                        .text_color(danger),
                )
                .child(
                    text("repair-failure-code", code.as_str())
                        .text_xs()
                        .text_color(muted),
                )
        });
        let button =
            |id: &'static str, label: &'static str| div().id(id).test_support().aria_label(label);
        let preview = Button::new("repair-preview-button")
            .label(PREVIEW)
            .primary()
            .disabled(running)
            .on_click(
                cx.listener(|pane, _: &ClickEvent, window, cx| pane.preview_repair(window, cx)),
            );
        let cancel = Button::new("repair-cancel-button")
            .label(CANCEL)
            .disabled(running)
            .on_click(
                cx.listener(|pane, _: &ClickEvent, window, cx| pane.cancel_repair(window, cx)),
            );
        let focus = repair.text.read(cx).focus_handle(cx);
        Some(
            v_flex()
                .id("repair-view")
                .test_support()
                .gap_3()
                .child(
                    div()
                        .id("repair-source")
                        .test_support()
                        .capture_action(|_: &IndentInline, window, cx| {
                            window.focus_next(cx);
                            cx.stop_propagation();
                        })
                        .capture_action(|_: &OutdentInline, window, cx| {
                            window.focus_prev(cx);
                            cx.stop_propagation();
                        })
                        .child(
                            Textarea::new(&repair.text)
                                .readonly(running)
                                .aria_label(SOURCE)
                                .font_family(mono)
                                .text_sm()
                                .h(px(TEXT_HEIGHT)),
                        )
                        .relative()
                        .child(self.scroll.marker(&focus)),
                )
                .children(failure)
                .child(
                    h_flex()
                        .gap_2()
                        .child(button("repair-preview", PREVIEW).child(preview))
                        .child(button("repair-cancel", CANCEL).child(cancel)),
                )
                .into_any_element(),
        )
    }
}
