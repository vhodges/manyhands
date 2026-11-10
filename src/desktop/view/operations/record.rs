//! The in-flight record's problem, once, in Activity (D6 design B3, R-31):
//! the record of unfinished checks could not be read, so it was set aside,
//! or could not be saved. Each is an entry with Dismiss, which hides it for
//! the session. It names no repository and no item.

use gpui_kit::{
    AnyElement, ClickEvent, Context, IntoElement, ParentElement, Styled, component::h_flex,
};

use super::{DISMISS, button, text};
use crate::desktop::{model::operations::record_note_words, view::workspace::Workspace};

impl Workspace {
    pub(super) fn record_entries(&mut self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let notes = self
            .services()
            .operations()
            .read(cx)
            .record_notes()
            .to_vec();
        let mut entries = Vec::with_capacity(notes.len());
        for (ix, report) in notes.into_iter().enumerate() {
            let focus = self.control_focus_for(format!("record-note-dismiss-{ix}"), cx);
            // One row that wraps: Dismiss stays inside the area (W-6), as the
            // attention entries' action does.
            entries.push(
                h_flex()
                    .gap_2()
                    .flex_wrap()
                    .items_center()
                    .child(text(("record-note", ix), record_note_words(report)))
                    .child(button(
                        ("record-note-dismiss", ix),
                        DISMISS,
                        Some(&focus),
                        cx,
                        cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.services().operations().update(cx, |operations, cx| {
                                operations.dismiss_record_note(report);
                                cx.notify();
                            })
                        }),
                    ))
                    .into_any_element(),
            );
        }
        entries
    }
}
