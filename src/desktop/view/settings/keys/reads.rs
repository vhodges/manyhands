//! Stamped key listing and public-key copy reads.

use gpui_kit::{ClipboardItem, Context, Window, component::WindowExt};
use manyhands::repository::{KeyDto, SharedKeyId};

use super::{Copied, KeysPage, SCOPE};
use crate::desktop::{
    model::read_state::Read,
    services::{ReadKind, ReadResult},
};

impl KeysPage {
    #[allow(clippy::result_large_err)] // ReadError carries its scope by value.
    pub(super) fn load(&mut self, cx: &mut Context<Self>) {
        let stamp = self.generations.next(SCOPE);
        if !matches!(self.keys, Read::Ready(_)) {
            self.keys = Read::Loading;
        }
        self.read = Some(self.services.read(
            ReadKind::Keys,
            stamp,
            |service| service.list_keys(),
            cx,
            |this: &mut Self, stamp, result, cx| {
                if !this.generations.is_current(&stamp) {
                    return;
                }
                this.keys = Read::from_result(result, |list| list.items);
                cx.notify();
            },
        ));
        cx.notify();
    }

    /// A row no longer listed acts on nothing.
    pub(super) fn listed(&self, id: SharedKeyId) -> Option<&KeyDto> {
        let Read::Ready(keys) = &self.keys else {
            return None;
        };
        let id = id.to_string();
        keys.iter().find(|key| key.id == id)
    }

    /// Only the library's public-key text reaches the clipboard.
    #[allow(clippy::result_large_err)]
    pub(crate) fn copy(&mut self, id: SharedKeyId, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_dialog(cx) || self.listed(id).is_none() {
            return;
        }
        let stamp = self.copy_generations.next(SCOPE);
        self.copied = None;
        self.copy_read = Some(self.services.read(
            ReadKind::Keys,
            stamp,
            move |service| service.public_key_text(id),
            cx,
            |this: &mut Self, stamp, result, cx| {
                if !this.copy_generations.is_current(&stamp) {
                    return;
                }
                this.copied = Some(match result {
                    ReadResult::Ok(public) => {
                        cx.write_to_clipboard(ClipboardItem::new_string(public.public_key));
                        Copied::Done
                    }
                    ReadResult::Busy => Copied::Busy,
                    ReadResult::Failed(failure) => Copied::Failed(failure.code),
                });
                cx.notify();
            },
        ));
        cx.notify();
    }
}
