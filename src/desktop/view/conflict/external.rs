//! External guidance (D5 design, B4): a set that goes outside Manyhands
//! (R-6, R-26) shows one card per path, "This file needs fixing outside
//! Manyhands", with its reason; then Open folder, which opens the item's
//! work folder, and Check again, which reads the conflict again. Resume is
//! offered only when that read's `next_action` says so (the sheet's
//! `resumable`); a repair the library refuses as it stands keeps the
//! guidance. There is no Save resolution and no winning closure.

use gpui_kit::{
    AnyElement, ClickEvent, Context, IntoElement, ParentElement, Styled,
    component::{ActiveTheme, Disableable, Sizable, button::Button, h_flex, v_flex},
    px,
};

use super::{ResolveSheet, render::text};
use crate::desktop::{
    model::conflict::{ExternalPath, ExternalReason},
    view::{
        item::{CHECK_AGAIN, open_folder},
        lifecycle::OPEN_FOLDER,
    },
};

/// B4: an external set's card, one per path.
pub(crate) const OUTSIDE: &str = "This file needs fixing outside Manyhands";
/// B4: each path's reason, by `ExternalReason`.
pub(crate) const CODE: &str = "It is code or another file that Manyhands does not edit.";
pub(crate) const BINARY: &str = "It holds binary content.";
pub(crate) const EXECUTABLE: &str = "It is an executable file.";
pub(crate) const SYMLINK: &str = "It is a symbolic link.";
pub(crate) const RENAMED_OR_DELETED: &str = "It was renamed or deleted on one side.";
pub(crate) const CLOSURES: &str = "Both copies record a different closure";
pub(crate) const UNREADABLE: &str = "Its copies can't be read as an item.";
pub(crate) const UNSUPPORTED: &str = "Manyhands can't resolve this conflict itself.";
pub(crate) const WAITS: &str = "It could be chosen here, but another file of this conflict \
     can't, so it waits for that repair.";
/// B4: how an outside repair goes on.
pub(crate) const OUTSIDE_LEAD: &str = "Fix these files in the folder, commit the merge, then \
     check again.";
/// A read after Check again: the library refuses the repair as it stands.
pub(crate) const REPAIR_NOT_DONE: &str = "The repair outside Manyhands isn't finished yet. \
     Finish it in the folder, then check again.";
/// A read after Check again: the library takes the repair up with Resume.
pub(crate) const REPAIRED: &str =
    "The repair is in place, and the synchronization goes on with Resume.";

/// What a path's card says of why it goes outside (B4).
fn reason_words(reason: ExternalReason) -> &'static str {
    match reason {
        ExternalReason::Code => CODE,
        ExternalReason::Binary => BINARY,
        ExternalReason::Executable => EXECUTABLE,
        ExternalReason::Symlink => SYMLINK,
        ExternalReason::RenamedOrDeleted => RENAMED_OR_DELETED,
        ExternalReason::Closures => CLOSURES,
        ExternalReason::Unreadable => UNREADABLE,
        ExternalReason::Unsupported => UNSUPPORTED,
        ExternalReason::Waits => WAITS,
    }
}

impl ResolveSheet {
    /// An external set's cards, one per path, and the way on.
    pub(super) fn external_cards(&self, paths: &[ExternalPath], cx: &Context<Self>) -> AnyElement {
        let border = cx.theme().border;
        let muted = cx.theme().muted_foreground;
        let cards = paths.iter().enumerate().map(|(index, path)| {
            v_flex()
                .gap_1()
                .p_2()
                .border_1()
                .border_color(border)
                .rounded(px(6.))
                .child(
                    text(
                        ("conflict-external", index),
                        format!("{}: {OUTSIDE}", path.display_path),
                    )
                    .text_sm(),
                )
                .child(
                    text(
                        ("conflict-external-reason", index),
                        reason_words(path.reason),
                    )
                    .text_xs()
                    .text_color(muted),
                )
        });
        v_flex()
            .gap_2()
            .children(cards)
            .child(
                text("conflict-outside-lead", OUTSIDE_LEAD)
                    .text_sm()
                    .text_color(muted),
            )
            .child(self.outside_actions(cx))
            .into_any_element()
    }

    /// A read after Check again whose way on is still a repair outside,
    /// with no conflict live: the repair is not one the library takes up.
    pub(super) fn repair_not_done(&self, cx: &Context<Self>) -> AnyElement {
        v_flex()
            .gap_2()
            .child(text("conflict-repair-not-done", REPAIR_NOT_DONE).text_sm())
            .child(self.outside_actions(cx))
            .into_any_element()
    }

    /// Open folder and Check again.
    fn outside_actions(&self, cx: &Context<Self>) -> impl IntoElement + use<> {
        let folder = self.folder.clone();
        h_flex()
            .gap_2()
            .child(
                Button::new("conflict-open-folder")
                    .label(OPEN_FOLDER)
                    .small()
                    .on_click(move |_: &ClickEvent, _, cx| open_folder(&folder, cx)),
            )
            .child(
                Button::new("conflict-check-again")
                    .label(CHECK_AGAIN)
                    .small()
                    .disabled(self.reading)
                    .on_click(self.on(cx, |sheet, _, cx| sheet.read_conflict(cx))),
            )
    }
}
