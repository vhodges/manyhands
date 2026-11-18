//! A nonconforming file's Git details: where the file was read from, in
//! the details sheet, read only. The header's Git details button opens it
//! (`OpenDetails`, on All details); the page holds none of it. A file with
//! context problems has no provenance to show, so it has no button.

use gpui_kit::{
    AnyElement, ClickEvent, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, TestSupportExt,
    component::{ActiveTheme, Sizable, button::Button},
    div,
};
use manyhands::repository::ItemDto;

use super::super::{ChipField, ItemPane, chips::OpenDetails};
use crate::desktop::model::item::context_problems;

pub(crate) const GIT_DETAILS: &str = "Git details";
pub(crate) const GIT_DESCRIPTION: &str = "Where this file was read from. Read only.";
/// The header's button, and the wrapper that tracks its focus.
pub(crate) const GIT_BUTTON: &str = "item-git-details";
const GIT_FOCUS: &str = "item-git-details-focus";

impl ItemPane {
    /// The header's Git details button, for a nonconforming file with
    /// provenance to show.
    pub(in crate::desktop::view::item) fn git_details_button(
        &self,
        item: Option<&ItemDto>,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let item = item.filter(|item| self.nonconforming_file(item))?;
        if !context_problems(item).is_empty() {
            return None;
        }
        // An item read by path has no chips, so All details' focus is free
        // for the button, and the sheet gives focus back to it.
        let focus = self.chip_focus[ChipField::All.index()].clone();
        let ring = cx.theme().ring;
        let wrapper = div()
            .id(GIT_FOCUS)
            .flex_none()
            .rounded_sm()
            .track_focus(&focus)
            .focus(move |style| style.border_2().border_color(ring))
            .on_click(cx.listener(|_, event: &ClickEvent, _, cx| {
                if event.is_keyboard() {
                    cx.emit(OpenDetails(ChipField::All));
                }
            }))
            .child(Button::new(GIT_BUTTON).label(GIT_DETAILS).small().on_click(
                cx.listener(|_, _: &ClickEvent, _, cx| cx.emit(OpenDetails(ChipField::All))),
            ));
        Some(wrapper.into_any_element())
    }

    /// The sheet's rows for a nonconforming file: its provenance, in a
    /// tab stop that takes the focus the sheet opens with.
    pub(super) fn git_rows(&self, item: &ItemDto, cx: &mut Context<Self>) -> Vec<AnyElement> {
        vec![
            div()
                .id("git-details")
                .test_support()
                .track_focus(&self.sheet_focus[ChipField::All.index()])
                .child(self.provenance(item, cx))
                .into_any_element(),
        ]
    }
}
