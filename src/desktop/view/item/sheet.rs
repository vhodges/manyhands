//! The details sheet while viewing (design, Details sheet (viewing);
//! `item_view/03-front-matter-sheet.md`): the Kit's `Sheet` on the right,
//! 440 px wide (fact 61, R-48), opened from a header chip or All details
//! with that chip's field focused (fact 80).
//!
//! It holds what D1 drew on the item's page: the named fields, each read
//! only in its own box at 75% (R-74); a ticket's relationships (`ticket`)
//! or a document's other keys (`document`); the other fields, kept as
//! written; the `facts`; the item's problems; and the Inspect link, which
//! opens the item's file in an Inspect tab (R-64). Escape, Done and the
//! close button close it, and Root gives focus back to the chip that
//! opened it. The workspace closes it when its tab goes away, and refuses
//! its own keys while it is open (R-60). Nothing in it writes.

mod document;
mod facts;
mod ticket;

use gpui_kit::{
    AnyElement, App, ClickEvent, Context, Entity, FocusHandle, FontWeight, InteractiveElement,
    IntoElement, ParentElement, Role, StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{
        ActiveTheme, Sizable, WindowExt,
        button::{Button, ButtonVariants},
        h_flex,
        scroll::Scrollbar,
        sheet::Sheet,
        v_flex,
    },
    div, px,
};
use manyhands::repository::{ItemContextKind, ItemDto, ItemDtoKind};

use super::{ChipField, ItemPane, text};
use crate::desktop::{
    model::{navigation::OpenTarget, relationships::NONE},
    view::relationships::OpenRelated,
};

pub(crate) const TICKET_DETAILS: &str = "Ticket details";
pub(crate) const DOCUMENT_DETAILS: &str = "Document details";
pub(crate) const VIEWING: &str = "Viewing the shared copy. Choose Edit to change these.";
pub(crate) const VIEWING_PRIVATE: &str = "Viewing your private copy. Choose Edit to change these.";
pub(crate) const READ_ONLY: &str = "Read only while viewing";
pub(crate) const DONE: &str = "Done";
pub(crate) const INSPECT: &str = "Inspect the raw front matter (YAML)…";
pub(crate) const OTHER_FIELDS: &str = "Other fields";
pub(crate) const KEPT: &str = "Kept exactly as written";
pub(crate) const FACTS: &str = "Facts";
pub(crate) const PROBLEMS: &str = "Problems";
pub(crate) const NOT_CLOSED: &str = "Not closed. Use Close ticket… to close it.";

/// The sheet's width (fact 61).
const WIDTH: f32 = 440.;
const TITLE_SIZE: f32 = 15.;
const LABEL_SIZE: f32 = 12.;
/// Between the body's rows.
const GAP: f32 = 14.;
/// The body's padding, vertical and horizontal.
const PADDING: (f32, f32) = (16., 18.);
const BOX_HEIGHT: f32 = 32.;
const BOX_RADIUS: f32 = 6.;
/// Fields are read only while viewing (R-74).
const READ_ONLY_OPACITY: f32 = 0.75;

/// One labelled row: its element ID, its label, and whether the design
/// marks it required.
#[derive(Clone, Copy)]
pub(super) struct Row {
    id: &'static str,
    label: &'static str,
    required: bool,
}

impl Row {
    const fn new(id: &'static str, label: &'static str) -> Self {
        Self {
            id,
            label,
            required: false,
        }
    }

    const fn required(id: &'static str, label: &'static str) -> Self {
        Self {
            id,
            label,
            required: true,
        }
    }

    /// Its name for assistive tech, which hears the red `*` as words.
    fn name(self) -> String {
        if self.required {
            format!("{}, required", self.label)
        } else {
            self.label.to_owned()
        }
    }
}

const TITLE: Row = Row::required("sheet-title", "Title");

/// The sheet's field focus, one per chip field, each a tab stop; All
/// details' is the Title's.
pub(super) fn sheet_focus(cx: &mut App) -> Vec<FocusHandle> {
    ChipField::FIELDS
        .iter()
        .map(|_| cx.focus_handle().tab_stop(true))
        .collect()
}

/// Opens `pane`'s details sheet with `field` focused. The chip takes focus
/// first, since Root gives focus back, on every close, to what held it as
/// the sheet opened; `open_sheet` then focuses a handle of Root's own, so
/// the field is focused after it (fact 80).
pub(crate) fn open_details(
    pane: &Entity<ItemPane>,
    field: ChipField,
    window: &mut Window,
    cx: &mut App,
) {
    let (chip, target) = {
        let pane = pane.read(cx);
        (
            pane.chip_focus[field.index()].clone(),
            pane.sheet_focus[field.index()].clone(),
        )
    };
    window.focus(&chip, cx);
    let pane = pane.downgrade();
    window.open_sheet(cx, move |sheet, window, cx| match pane.upgrade() {
        Some(pane) => pane.update(cx, |pane, cx| pane.sheet(sheet, window, cx)),
        None => sheet,
    });
    window.focus(&target, cx);
}

/// A field's value in its box, labelled by itself.
fn value(id: &'static str, value: Option<&String>) -> Option<AnyElement> {
    value.map(|value| text(id, value.clone()).into_any_element())
}

impl ItemPane {
    /// The sheet as this frame draws it, from the item as last read, so a
    /// read again updates it in place.
    fn sheet(&mut self, sheet: Sheet, window: &mut Window, cx: &mut Context<Self>) -> Sheet {
        let item = self.model.item().rows().cloned();
        let kind = self
            .shown_kind(item.as_ref())
            .unwrap_or(ItemDtoKind::Ticket);
        let heading = match kind {
            ItemDtoKind::Ticket => TICKET_DETAILS,
            ItemDtoKind::Document => DOCUMENT_DETAILS,
        };
        let rows = match (&item, kind) {
            (Some(item), ItemDtoKind::Ticket) => self.ticket_rows(item, window, cx),
            (Some(item), ItemDtoKind::Document) => self.document_rows(item, window, cx),
            (None, _) => Vec::new(),
        };
        let private = item
            .as_ref()
            .is_some_and(|item| item.context.kind == ItemContextKind::Active);
        let description = if private { VIEWING_PRIVATE } else { VIEWING };
        let muted = cx.theme().muted_foreground;
        let (vertical, horizontal) = PADDING;
        let body = v_flex()
            .gap(px(GAP))
            .py(px(vertical))
            .px(px(horizontal))
            .child(
                text("sheet-description", description)
                    .text_sm()
                    .text_color(muted),
            )
            .children(rows);
        let handle = self.sheet_scroll.handle().clone();
        let footer = h_flex()
            .w_full()
            .justify_between()
            .items_center()
            .child(text("sheet-note", READ_ONLY).text_xs().text_color(muted))
            .child(
                Button::new("sheet-done")
                    .label(DONE)
                    .small()
                    .on_click(|_, window, cx| window.close_sheet(cx)),
            );
        sheet
            .size(px(WIDTH))
            .title(
                text("sheet-heading", heading)
                    .text_size(px(TITLE_SIZE))
                    .font_weight(FontWeight::SEMIBOLD),
            )
            .footer(footer)
            // The Kit's body grows with what it holds and scrolls on a handle
            // of its own. Laid over it, out of its flow, the sheet scrolls on
            // its own handle instead, which reveals the focused control.
            .child(
                div()
                    .id("sheet-body")
                    .absolute()
                    .inset_0()
                    .child(
                        div()
                            .id("sheet-scroll")
                            .size_full()
                            .overflow_y_scroll()
                            .track_scroll(&handle)
                            .child(body),
                    )
                    .child(
                        div()
                            .absolute()
                            .inset_0()
                            .child(Scrollbar::vertical(&handle).id("sheet-scrollbar")),
                    ),
            )
    }

    /// A row: its label, with the design's red `*` when required, over its
    /// value, read only in the design's box. With `focus`, the row is a tab
    /// stop, ringed while focused and revealed in the sheet's scroll.
    fn row(
        &self,
        row: Row,
        value: Option<AnyElement>,
        focus: Option<&FocusHandle>,
        window: &Window,
        cx: &App,
    ) -> AnyElement {
        let theme = cx.theme();
        let focused = focus.is_some_and(|focus| focus.is_focused(window));
        let label = h_flex()
            .gap(px(2.))
            .text_size(px(LABEL_SIZE))
            .font_weight(FontWeight::MEDIUM)
            .child(row.label)
            .children(
                row.required
                    .then(|| div().text_color(theme.danger).child("*")),
            );
        let ring = focused.then(|| {
            div()
                .absolute()
                .top(px(-3.))
                .bottom(px(-3.))
                .left(px(-3.))
                .right(px(-3.))
                .rounded(px(BOX_RADIUS + 2.))
                .border_2()
                .border_color(theme.ring)
        });
        let content = v_flex()
            .justify_center()
            .gap_1()
            .min_h(px(BOX_HEIGHT))
            .px(px(10.))
            .py(px(6.))
            .rounded(px(BOX_RADIUS))
            .border_1()
            .border_color(if focused { theme.info } else { theme.input })
            .bg(theme.background)
            .text_sm()
            .opacity(READ_ONLY_OPACITY)
            .children(value);
        let field = div().relative().child(content).children(ring);
        let mut element = v_flex()
            .id(row.id)
            .test_support()
            .role(Role::Group)
            .aria_label(row.name())
            .relative()
            .gap(px(6.))
            .child(label)
            .child(field);
        if let Some(focus) = focus {
            element = element
                .track_focus(focus)
                .child(self.sheet_scroll.marker(focus));
        }
        element.into_any_element()
    }

    /// The Title row, which All details opens the sheet on.
    fn title_row(&self, item: &ItemDto, window: &Window, cx: &App) -> AnyElement {
        let focus = &self.sheet_focus[ChipField::All.index()];
        self.row(
            TITLE,
            value("sheet-title-value", item.title.as_ref()),
            Some(focus),
            window,
            cx,
        )
    }

    /// A section of the sheet: its heading, as its name, then `content`.
    fn section(
        id: &'static str,
        heading: &'static str,
        content: impl IntoElement,
    ) -> impl IntoElement {
        v_flex()
            .id(id)
            .test_support()
            .role(Role::Group)
            .aria_label(heading)
            .gap_1()
            .child(
                div()
                    .text_size(px(LABEL_SIZE))
                    .font_weight(FontWeight::MEDIUM)
                    .child(heading),
            )
            .child(content)
    }

    /// Other fields: each key but those in `skip`, with its value as the
    /// library gives it, in its order (fact 78), or None. Never editable
    /// (decision 14).
    fn other_fields(&self, item: &ItemDto, skip: &[&str], cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let any = item
            .unknown_metadata
            .keys()
            .any(|key| !skip.contains(&key.as_str()));
        let fields = if any {
            self.metadata(item, skip, cx).into_any_element()
        } else {
            text("sheet-other-none", NONE)
                .text_sm()
                .text_color(muted)
                .into_any_element()
        };
        Self::section(
            "sheet-other",
            OTHER_FIELDS,
            v_flex()
                .gap_1()
                .child(text("sheet-other-note", KEPT).text_xs().text_color(muted))
                .child(fields),
        )
        .into_any_element()
    }

    /// The item's own problems, as D1 listed them, when it has any.
    fn problems_section(&self, item: &ItemDto, cx: &mut Context<Self>) -> Option<AnyElement> {
        if self.own_problems(item).is_empty() {
            return None;
        }
        Some(Self::section("sheet-problems", PROBLEMS, self.problems(item, cx)).into_any_element())
    }

    /// "Inspect the raw front matter (YAML)…": the item's file, by its path
    /// in the worktree it was read from, in an Inspect tab, which shows the
    /// whole file (R-64). Opening it closes the sheet. Its wrapper holds
    /// the Kit button's focus, so the link is revealed when Tab reaches it.
    fn inspect_link(&self, item: &ItemDto, cx: &mut Context<Self>) -> AnyElement {
        let target = OpenTarget::Inspect {
            repo: self.repo().to_owned(),
            worktree: item.context.worktree.clone(),
            path: item.path.clone(),
        };
        div()
            .track_focus(&self.link_focus)
            .relative()
            .child(Button::new("sheet-inspect").link().label(INSPECT).on_click(
                cx.listener(move |_, _: &ClickEvent, _, cx| cx.emit(OpenRelated(target.clone()))),
            ))
            .child(self.sheet_scroll.marker_within(&self.link_focus))
            .into_any_element()
    }
}
