//! The Needs attention popover (Wave 04 Cycle 03b, Part E2): a filtered view
//! of the Activity log's rows, the ones that need the person, opened from the
//! footer's Needs attention item.
//!
//! The popover keeps its rows the way Activity's list does, in an
//! `ActivityPane` it owns: the rows applied are the ones drawn, the highlight
//! follows its `RowKey` (the cursor), and when the highlighted row changes
//! while its button holds focus, focus moves to the popover's list (C2, E2).
//! A click or Enter builds its choice from the drawn row, with its key and
//! revision (R-55), so one made against a row the log has since changed runs
//! nothing (T8). The three focus handles (the list, the highlighted row's
//! button and the link) belong to the Workspace for its whole life, as
//! `attention_focus`: a dialog or menu opened from the popover remembers the
//! list as the place focus returns to, and `Workspace::focus_lost` sends a
//! focus lost on any of them to the footer (R-50, U-8).
//!
//! It is drawn deferred and anchored at the footer item's top right, its
//! bottom edge 6 px above the footer (R-38), 440 px wide, scrolling when the
//! window is too short.

use gpui_kit::{
    Anchor, AnyElement, App, Bounds, Context, ElementId, FocusHandle, FontWeight,
    InteractiveElement, IntoElement, ParentElement, Pixels, Point, Role,
    StatefulInteractiveElement, Styled, TestSupportExt, WeakEntity, Window, anchored,
    component::{ActiveTheme, tooltip::Tooltip},
    deferred, div, point,
    prelude::FluentBuilder,
    px, uniform_list,
};
use std::rc::Rc;

use super::{
    activity::{
        ActivityPane,
        row::{button, choice_of, kind_icon, tooltip},
    },
    footer::FOOTER_HEIGHT,
    rows::icon,
    shell::{TITLE_BAR_HEIGHT, tokens::chrome_colors},
    workspace::Workspace,
};
use crate::desktop::{
    keys::ATTENTION_POPOVER_CONTEXT,
    model::activity::{
        copy::{NEEDS_ATTENTION_SAFE, NEEDS_ATTENTION_TITLE, SHOW_EVERYTHING},
        rows::ActivityRow,
    },
};

/// The popover's width (R-38).
const WIDTH: f32 = 440.;
/// Between the popover's bottom edge and the footer's top edge (R-38).
const GAP: f32 = 6.;
/// One row: the title above its description.
const ROW_HEIGHT: f32 = 46.;
/// What the header, the link and the panel's edges take.
const CHROME_HEIGHT: f32 = 118.;
const TEXT_SIZE: f32 = 13.;
const SMALL_SIZE: f32 = 12.;
const ICON_SIZE: f32 = 14.;
const TAG_WIDTH: f32 = 120.;
/// The highlighted row's focus ring.
const MARK_WIDTH: f32 = 2.;

/// The popover's three tab stops, which the Workspace keeps across opens.
#[derive(Clone)]
pub(crate) struct AttentionHandles {
    /// The list: it holds the highlight, and is `attention_focus`.
    pub(crate) list: FocusHandle,
    /// The highlighted row's enabled button.
    pub(crate) button: FocusHandle,
    /// "Show everything in Activity".
    pub(crate) link: FocusHandle,
}

impl AttentionHandles {
    pub(crate) fn new(cx: &mut Context<Workspace>) -> Self {
        Self {
            list: cx.focus_handle().tab_stop(true),
            button: cx.focus_handle().tab_stop(true),
            link: cx.focus_handle().tab_stop(true),
        }
    }

    /// Whether `handle` is one of the three.
    pub(crate) fn owns(&self, handle: &FocusHandle) -> bool {
        [&self.list, &self.button, &self.link].contains(&handle)
    }
}

/// An open popover. Its focus handles are the Workspace's
/// (`AttentionHandles`), and the place focus goes back to is the footer
/// item's, which the Workspace holds: neither is kept here.
pub(crate) struct AttentionPopover {
    /// The rows applied (the drawn ones), the highlight by key (the
    /// cursor) and the scroll: Activity's own list state, over the
    /// attention rows.
    pub(crate) pane: ActivityPane,
}

impl AttentionPopover {
    pub(crate) fn new(handles: &AttentionHandles) -> Self {
        Self {
            pane: ActivityPane::new(handles.list.clone(), handles.button.clone()),
        }
    }

    /// The popover, drawn above everything but tooltips, anchored at the
    /// footer item's top right with `Anchor::BottomRight`: its bottom edge 6
    /// px above the footer's top edge, kept inside the window (R-38).
    pub(crate) fn layer(
        &self,
        item: Bounds<Pixels>,
        handles: &AttentionHandles,
        workspace: WeakEntity<Workspace>,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let colors = chrome_colors(cx);
        let size = window.bounds().size;
        let at: Point<Pixels> = point(item.right(), size.height - px(FOOTER_HEIGHT + GAP));
        let room = (size.height.as_f32() - TITLE_BAR_HEIGHT - FOOTER_HEIGHT - GAP - CHROME_HEIGHT)
            .max(ROW_HEIGHT);
        let rows = self.pane.rows_applied();
        let listed = rows.len();
        let list_height = (listed as f32 * ROW_HEIGHT).min(room);
        let highlight = self.pane.highlighted_key().cloned();
        let list = handles.list.clone();
        let focused = handles.list.is_focused(window);
        let body = {
            let (rows, highlight) = (rows.clone(), highlight.clone());
            let (workspace, handles) = (workspace.clone(), handles.clone());
            uniform_list("needs-attention-rows", listed, move |range, _, cx| {
                range
                    .filter_map(|ix| {
                        let row = rows.get(ix)?;
                        Some(row_element(
                            ix,
                            row,
                            highlight.as_ref() == Some(&row.row.key),
                            focused,
                            &handles,
                            workspace.clone(),
                            cx,
                        ))
                    })
                    .collect::<Vec<_>>()
            })
            .track_scroll(self.pane.scroll())
            .size_full()
        };
        let link = {
            let workspace = workspace.clone();
            let ring = cx.theme().ring;
            div()
                .id("needs-attention-show-all")
                .test_support()
                .role(Role::Button)
                .aria_label(SHOW_EVERYTHING)
                .track_focus(&handles.link)
                .focus(move |style| style.border_2().border_color(ring))
                .rounded_sm()
                .px_1()
                .text_size(px(TEXT_SIZE))
                .text_color(colors.teal)
                .underline()
                .cursor_pointer()
                .child(SHOW_EVERYTHING)
                .on_click(move |_, window, cx| {
                    let _ = workspace.update(cx, |workspace, cx| {
                        workspace.show_everything_in_activity(window, cx)
                    });
                })
        };
        let outside = workspace.clone();
        let panel = div()
            .id("needs-attention-popover")
            .test_support()
            .key_context(ATTENTION_POPOVER_CONTEXT)
            .role(Role::Dialog)
            .aria_label(NEEDS_ATTENTION_TITLE)
            .occlude()
            .w(px(WIDTH))
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .bg(colors.panel)
            .border_1()
            .border_color(colors.border_strong)
            .rounded_md()
            .shadow_lg()
            .text_color(colors.text)
            // A press outside closes it, and still reaches what is under it
            // (R-11). The footer item's own press is the item's: its click
            // closes the popover once (fact 47).
            .on_mouse_down_out(move |event, window, cx| {
                let position = event.position;
                let _ = outside.update(cx, |workspace, cx| {
                    workspace.attention_press_outside(position, window, cx)
                });
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(px(TEXT_SIZE + 1.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(NEEDS_ATTENTION_TITLE),
                    )
                    .child(
                        div()
                            .text_size(px(SMALL_SIZE))
                            .text_color(colors.muted)
                            .child(NEEDS_ATTENTION_SAFE),
                    ),
            )
            .child(
                div()
                    .id("needs-attention-list")
                    .test_support()
                    .track_focus(&list)
                    .relative()
                    .w_full()
                    .h(px(list_height))
                    .child(body),
            )
            .child(link);
        deferred(
            anchored()
                .position(at)
                .anchor(Anchor::BottomRight)
                .snap_to_window_with_margin(px(8.))
                .child(panel),
        )
        .with_priority(gpui_kit::base::POPUP_PRIORITY)
        .into_any_element()
    }
}

/// One popover row: the kind's icon, the title above its description, the
/// repository tag and the button. Its tooltip is the row's (B5).
fn row_element(
    ix: usize,
    row: &ActivityRow,
    highlighted: bool,
    list_focused: bool,
    handles: &AttentionHandles,
    workspace: WeakEntity<Workspace>,
    cx: &mut App,
) -> AnyElement {
    let colors = chrome_colors(cx);
    let (name, color) = kind_icon(row, &colors);
    let lines = tooltip(row);
    let tooltip_text: gpui_kit::SharedString = lines.join("\n").into();
    let key = row.row.key.clone();
    let pressed = workspace.clone();
    let button = row.row.button.as_ref().map(|offer| {
        let choice = choice_of(row, offer);
        let press = Rc::new(move |window: &mut Window, cx: &mut App| {
            let choice = choice.clone();
            let _ = pressed.update(cx, |workspace, cx| {
                workspace.press_attention(choice, window, cx)
            });
        });
        button(
            ElementId::from(("needs-attention-button", ix)),
            offer,
            highlighted,
            handles.button.clone(),
            press,
            cx,
        )
    });
    let tag = row.row.repo.clone().map(|repo| {
        div()
            .max_w(px(TAG_WIDTH))
            .flex_none()
            .overflow_hidden()
            .text_ellipsis()
            .whitespace_nowrap()
            .text_size(px(SMALL_SIZE - 0.5))
            .text_color(colors.secondary)
            .child(repo)
    });
    div()
        .id(("needs-attention-row", ix))
        .test_support()
        .aria_label(row.row.title.clone())
        .aria_description(lines.join(" "))
        // A press on a row highlights it and focuses the list; the button
        // inside, whose click comes later, runs on its own.
        .on_mouse_down(gpui_kit::MouseButton::Left, move |_, window, cx| {
            let key = key.clone();
            let _ = workspace.update(cx, |workspace, cx| {
                workspace.highlight_attention(key, window, cx)
            });
        })
        .relative()
        .flex()
        .flex_none()
        .w_full()
        .h(px(ROW_HEIGHT))
        .px_2()
        .gap_2()
        .items_center()
        .tooltip(move |window, cx| Tooltip::new(tooltip_text.clone()).build(window, cx))
        .when(highlighted, |this| this.bg(colors.selection))
        .when(!highlighted, |this| {
            this.hover(move |style| style.bg(colors.hover))
        })
        .child(icon(name, ICON_SIZE, color))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(
                    div()
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .text_size(px(TEXT_SIZE))
                        .child(row.row.title.clone()),
                )
                .children(row.row.description.clone().map(|description| {
                    div()
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .text_size(px(SMALL_SIZE))
                        .text_color(colors.muted)
                        .child(description)
                })),
        )
        .children(tag)
        .children(button)
        .when(highlighted && list_focused, |this| {
            this.child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .border(px(MARK_WIDTH))
                    .border_color(colors.teal),
            )
        })
        .into_any_element()
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "needs_attention_tests.rs"]
mod tests;
