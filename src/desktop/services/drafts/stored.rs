//! Stored drafts this process holds no session of: their raw bytes, for an
//! unreadable draft's export, and their discard from the Drafts page.
//!
//! A readable draft is claimed and discarded, so one another process holds
//! is refused; an unreadable one is deleted under its lock, which is how
//! the store discards what it cannot read. Either way the store is listed
//! again once the call lands, whatever it answered. A draft this process
//! holds is discarded through its session (`discard`), never here.

use gpui_kit::{Context, Task};
use manyhands::drafts::{DraftErrorKind, DraftId};

use super::{DraftDiscardLanded, DraftRegistry, off_thread};
use crate::desktop::services::ReadKind;

impl DraftRegistry {
    /// The bytes of `id`'s file as they are: an unreadable draft's raw
    /// export. They are read behind the `Drafts` gate, as a listing is.
    pub(crate) fn raw_bytes(
        &mut self,
        id: DraftId,
        cx: &mut Context<Self>,
    ) -> Task<Result<Vec<u8>, DraftErrorKind>> {
        let read = off_thread(
            &self.backend,
            ReadKind::Drafts,
            move |store| store.raw_bytes(id),
            cx,
        );
        cx.background_executor()
            .spawn(async move { read.await.map_err(|error| error.kind) })
    }

    /// Discards the stored draft `id`, which this process holds no session
    /// of: claimed and discarded when `readable`, else deleted under its
    /// lock. Another process holding it refuses with `OpenElsewhere`. A
    /// failed discard of a claimed draft releases it, still stored. The
    /// store is listed again once it lands; a readable draft deleted is
    /// `DraftDiscardLanded`, by the target it was listed with.
    pub(crate) fn discard_stored(
        &mut self,
        id: DraftId,
        readable: bool,
        cx: &mut Context<Self>,
    ) -> Task<Result<(), DraftErrorKind>> {
        let target = self.listed(id).map(|(draft, _)| draft.target.clone());
        let discard = off_thread(
            &self.backend,
            ReadKind::DraftWrite,
            move |store| match readable {
                true => store.claim(id)?.discard(),
                false => store.discard_unreadable(id),
            },
            cx,
        );
        cx.spawn(async move |registry, cx| {
            let discarded = discard.await.map_err(|error| error.kind);
            // Gone or released, the draft is listed as it now is.
            let _ = registry.update(cx, |registry, cx| {
                registry.reload_listing(cx);
                if let (Ok(()), Some(target)) = (&discarded, target) {
                    cx.emit(DraftDiscardLanded(target));
                }
            });
            discarded
        })
    }
}
