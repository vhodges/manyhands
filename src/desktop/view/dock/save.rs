//! Saving the layout (Part G; R-19, R-31): one debounced save, its write
//! off the UI thread, and the quit's write on it.

use std::time::Duration;

use gpui_kit::{App, Context, Task};

use super::{DockShell, accepted::centre_is_item_area};
use crate::desktop::{
    model::layout::{Layout, encode},
    services::layout::{Landed, LayoutStoreError},
    view::workspace::Workspace,
};

/// How long after the last change the layout is written (R-19).
pub(crate) const LAYOUT_SAVE_DELAY: Duration = Duration::from_millis(500);

/// The save in progress.
#[derive(Default)]
pub(super) struct SaveState {
    /// Waits out `LAYOUT_SAVE_DELAY`; a new request replaces it, which
    /// restarts the wait.
    timer: Option<Task<()>>,
    /// A change has been asked to be saved and no write has landed since.
    pending: bool,
    /// The newest generation a result was taken from.
    reported: u64,
}

impl DockShell {
    /// Asks for the layout to be saved (R-31): the one timer restarts, and
    /// when it fires the layout is written as it is then. With no store,
    /// saving is off.
    pub(in crate::desktop::view) fn request_save(&mut self, cx: &mut Context<Workspace>) {
        self.saves += 1;
        if self.store.is_none() {
            return;
        }
        self.save.pending = true;
        self.save.timer = Some(cx.spawn(async move |workspace, cx| {
            cx.background_executor().timer(LAYOUT_SAVE_DELAY).await;
            let _ = workspace.update(cx, |workspace, cx| workspace.write_layout(cx));
        }));
    }

    /// The bytes for `layout`: a fresh dump of the area, whose sizes are the
    /// latest, or the accepted state if a drop has not been undone yet.
    fn file_bytes(&self, layout: &Layout, cx: &App) -> Vec<u8> {
        let dump = self.area.read(cx).dump(cx);
        let area = if centre_is_item_area(&dump) {
            dump
        } else {
            self.accepted.clone()
        };
        encode(layout, &area)
    }

    /// The timer fired: takes the layout as it is and writes it off the UI
    /// thread, under the generation the store hands out now.
    pub(in crate::desktop::view) fn write(&mut self, layout: &Layout, cx: &mut Context<Workspace>) {
        self.save.timer = None;
        let Some(store) = self.store.clone() else {
            return;
        };
        let bytes = self.file_bytes(layout, cx);
        let (write, taken) = (store.begin(), self.saves);
        let generation = write.generation();
        cx.spawn(async move |workspace, cx| {
            let result = cx.background_executor().spawn(write.run(bytes)).await;
            let _ = workspace.update(cx, |workspace, cx| {
                workspace.layout_written(generation, taken, result, cx)
            });
        })
        .detach();
    }

    /// A write answered. A skipped one changes nothing; a write that landed
    /// removes the save notice, and one that failed shows it.
    pub(in crate::desktop::view) fn written(
        &mut self,
        generation: u64,
        taken: usize,
        result: Result<Landed, LayoutStoreError>,
        cx: &mut Context<Workspace>,
    ) {
        if generation < self.save.reported {
            return;
        }
        match result {
            Ok(Landed::Stale) => return,
            Ok(Landed::Written) => {
                // Nothing asked for since: the file is current.
                self.save.pending = self.saves != taken;
                self.notices.save = false;
            }
            Err(_) => self.notices.save = true,
        }
        self.save.reported = generation;
        cx.notify();
    }

    /// The quit's write: at once, on this thread, under the next
    /// generation, if a change has not been written (K-14). A write in
    /// flight then lands as stale.
    pub(in crate::desktop::view) fn write_for_quit(&mut self, layout: &Layout, cx: &App) {
        let Some(store) = self.store.clone() else {
            return;
        };
        if !self.save.pending {
            return;
        }
        let bytes = self.file_bytes(layout, cx);
        if store.write_now(&bytes).is_ok() {
            self.save.pending = false;
        }
    }
}
