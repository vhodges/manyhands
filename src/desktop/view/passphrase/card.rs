//! Unlock key's key card (Wave 04 Cycle 05a design Part D item 2; R-16):
//! `key-round`, the key's label and, once the key list has it, its
//! fingerprint in mono.
//!
//! The dialog reads the key list once, as it opens, through
//! `Services::read` (which retries a busy read itself), stamped under
//! `ViewScope::Unlock`, and takes the entry whose ID is the prompt's key.
//! It keeps only that entry's fingerprint: the entry also carries the
//! key's file paths, which are never drawn. Until the list lands, or if
//! the read fails, finds no such key or the key records no fingerprint,
//! the card shows the label only.

use gpui_kit::{
    App, Context, FontWeight, InteractiveElement, IntoElement, ParentElement, Styled,
    TestSupportExt,
    assets::IconName,
    component::{ActiveTheme, Icon, h_flex, v_flex},
    div, px,
};
use manyhands::repository::KeyListDto;

use super::{PassphraseDialog, text};
use crate::desktop::{
    model::stamp::{Stamp, ViewScope},
    services::{ReadKind, ReadResult, Services},
    view::shell::tokens::chrome_colors,
};

impl PassphraseDialog {
    /// Reads the key list, for the prompt's key's fingerprint.
    #[allow(clippy::result_large_err)] // ReadError carries its scope by value.
    pub(super) fn read_key(&mut self, services: &Services, cx: &mut Context<Self>) {
        let stamp = self.generations.next(ViewScope::Unlock);
        self.read = Some(services.read(
            ReadKind::Keys,
            stamp,
            |service| service.list_keys(),
            cx,
            |this: &mut Self, stamp, result, cx| this.listed(stamp, result, cx),
        ));
    }

    /// The prompt was answered or withdrawn: a read still in flight is
    /// dropped, and lands nothing.
    pub(super) fn drop_read(&mut self) {
        self.read = None;
    }

    /// The key list landed: the prompt's key's fingerprint, if it has one,
    /// and nothing else of the list. A busy or failed read leaves the
    /// label alone on the card, and so does one for a prompt no longer
    /// held.
    fn listed(&mut self, stamp: Stamp, result: ReadResult<KeyListDto>, cx: &mut Context<Self>) {
        if !self.generations.is_current(&stamp) || self.prompt.is_none() {
            return;
        }
        let ReadResult::Ok(list) = result else {
            return;
        };
        self.fingerprint = list
            .items
            .into_iter()
            .find(|key| key.id == self.key_id)
            .and_then(|key| key.fingerprint);
        cx.notify();
    }

    /// The card: `key-round`, the label and, once known, the fingerprint.
    pub(super) fn card(&self, cx: &App) -> impl IntoElement + use<> {
        let colors = chrome_colors(cx);
        let mono = cx.theme().mono_font_family.clone();
        h_flex()
            .id("passphrase-key")
            .test_support()
            .items_center()
            .gap(px(10.))
            .px(px(12.))
            .py(px(10.))
            .rounded(px(6.))
            .border_1()
            .border_color(colors.border)
            .child(
                Icon::new(IconName::KeyRound)
                    .text_color(colors.secondary)
                    .w(px(16.))
                    .h(px(16.)),
            )
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap(px(2.))
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .child(text("passphrase-key-label", self.label.clone())),
                    )
                    .children(self.fingerprint.clone().map(|fingerprint| {
                        div()
                            .text_xs()
                            .text_color(colors.secondary)
                            .font_family(mono)
                            .child(text("passphrase-fingerprint", fingerprint))
                    })),
            )
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
impl PassphraseDialog {
    /// The fingerprint the card holds, read past the dialog's close.
    pub(crate) fn fingerprint(&self) -> Option<&str> {
        self.fingerprint.as_deref()
    }

    /// Lands `result` as the key list's answer (U-8: no test backend can
    /// fail a read). The read in flight goes stale, so it lands nothing.
    pub(crate) fn answer_key_read(
        &mut self,
        result: ReadResult<KeyListDto>,
        cx: &mut Context<Self>,
    ) {
        let stamp = self.generations.next(ViewScope::Unlock);
        self.listed(stamp, result, cx);
    }
}
