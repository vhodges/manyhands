//! The page and the item's scroll region (R-47; design, Layout, Page and
//! The read view).
//!
//! The region sits below the tab strip and the header, which stay in
//! place, and holds the page, then the discussion, then the workspace's
//! details of the selected repository (owner, 2026-10-27). The page is white in
//! both themes, its column at most 760 px wide and centred, and it is at
//! least as tall as the region was in the last frame, so the white reaches
//! the bottom and the discussion starts below the fold.
//!
//! While viewing, an item tab's page holds the body in a read editor
//! (decision 17): a `ReadOnly` made at the first read with a body, kept
//! across reloads, and reloaded in place when the body changes (R-19). It
//! keeps its own caret in view (R-67): after each caret move while it has
//! focus, and when it takes focus. A press on the page below the text puts
//! the caret at the end; no press focuses the page itself, so the page's
//! ring shows only after Back to top.
//!
//! The region shows the Kit's vertical scrollbar on its own handle, thumb
//! in `page-border-strong` at rest, hovered and dragged, laid over the
//! region as the Kit's own scrollables lay it.

use std::{cell::Cell, rc::Rc};

use gpui_kit::{
    AnyElement, App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, MouseButton, MouseDownEvent, ParentElement, Pixels, StatefulInteractiveElement,
    Styled, Subscription, TestSupportExt, Window, canvas,
    component::{ActiveTheme, scroll::Scrollbar, v_flex},
    div, px,
};
use zorite_editor::EditorState;

use super::{ItemPane, tokens::item_colors};
use crate::desktop::view::{
    editor::{read_only::ReadOnly, reveal_caret},
    scroll::ContentScroll,
};

/// The page's column at its widest.
const COLUMN: f32 = 760.;
/// The column's padding: top, sides and bottom.
const PADDING: (f32, f32, f32) = (36., 48., 40.);
/// Body text on the page.
const BODY_SIZE: f32 = 15.;

/// The item's page.
pub(crate) struct Page {
    /// Back to top focuses the page. It is no tab stop of its own: Tab
    /// goes on to what the page holds.
    focus: FocusHandle,
    /// The body's read editor, once a read gave a body.
    read: Option<Entity<ReadOnly>>,
    /// The body `read` last loaded, as the library read it.
    body: String,
    /// The caret and the text's length `read` last showed, so that only a
    /// move reveals the caret.
    caret: (usize, usize),
    /// Whether the read editor had focus in the last frame.
    focused: bool,
    /// The bottom of the read view in the last frame, in the window.
    read_bottom: Rc<Cell<Pixels>>,
    _read: Vec<Subscription>,
}

impl Page {
    pub(super) fn new(cx: &mut App) -> Self {
        Self {
            focus: cx.focus_handle(),
            read: None,
            body: String::new(),
            caret: (0, 0),
            focused: false,
            read_bottom: Rc::default(),
            _read: Vec::new(),
        }
    }

    pub(super) fn focus_handle(&self) -> &FocusHandle {
        &self.focus
    }

    /// The read view, once a read gave a body.
    #[cfg(test)]
    pub(super) fn read(&self) -> Option<&Entity<ReadOnly>> {
        self.read.as_ref()
    }

    /// Shows `body`, the last successful read's, in the read editor. The
    /// first body makes it; a changed body is loaded into it in place,
    /// keeping its cursor, clamped (R-19); the same body leaves it
    /// untouched. A read with no body, a file that is not UTF-8, drops it.
    fn show(&mut self, body: Option<&str>, window: &mut Window, cx: &mut Context<ItemPane>) {
        let Some(body) = body else {
            self.read = None;
            self._read.clear();
            self.body.clear();
            return;
        };
        if let Some(read) = &self.read {
            if self.body != body {
                read.update(cx, |read, cx| read.reload(body, cx));
                self.body = body.to_owned();
            }
            return;
        }
        let read = cx.new(|cx| ReadOnly::new(body, window, cx));
        let editor = read.read(cx).editor().clone();
        self._read = vec![cx.observe_in(&editor, window, |pane, editor, window, cx| {
            pane.page.caret_moved(&editor, &pane.scroll, window, cx)
        })];
        self.caret = caret(editor.read(cx));
        self.read = Some(read);
        self.body = body.to_owned();
    }

    /// R-67: when the read editor has taken focus since the last frame,
    /// its caret is brought into view once this frame is drawn, from this
    /// frame's positions. A frame notes the focus, since GPUI sends focus
    /// events only to an active window.
    fn note_focus(&mut self, window: &Window, cx: &mut Context<ItemPane>) {
        let focused = self.read.as_ref().is_some_and(|read| {
            let editor = read.read(cx).editor();
            editor.read(cx).focus_handle(cx).is_focused(window)
        });
        if focused && !self.focused {
            cx.defer_in(window, |pane, _, cx| {
                if let Some(read) = &pane.page.read {
                    let editor = read.read(cx).editor().clone();
                    reveal_caret(&editor, &pane.scroll, cx);
                    cx.notify();
                }
            });
        }
        self.focused = focused;
    }

    /// R-67: a move of the caret, or a change of text, while the read
    /// editor has focus brings the caret into view. Other notifications,
    /// such as the pointer passing over it, move nothing.
    fn caret_moved(
        &mut self,
        editor: &Entity<EditorState>,
        scroll: &ContentScroll,
        window: &Window,
        cx: &mut App,
    ) {
        let now = caret(editor.read(cx));
        if now == self.caret {
            return;
        }
        self.caret = now;
        if editor.read(cx).focus_handle(cx).is_focused(window) {
            reveal_caret(editor, scroll, cx);
        }
    }

    /// A left press on the page that no control took. The page itself
    /// never takes focus from it. Below the text, it focuses the read
    /// editor with its caret at the end of the body.
    fn pressed(&self, event: &MouseDownEvent, window: &mut Window, cx: &mut App) {
        if window.default_prevented() {
            return;
        }
        window.prevent_default();
        let Some(read) = &self.read else {
            return;
        };
        if event.position.y < self.read_bottom.get() {
            return;
        }
        let editor = read.read(cx).editor().clone();
        editor.update(cx, |editor, cx| {
            editor.set_cursor(editor.text().len(), cx);
            editor.focus(window, cx);
        });
    }

    /// The read view, for the page's content: the read editor, noting
    /// where it ends.
    pub(super) fn read_view(&self) -> Option<AnyElement> {
        let read = self.read.clone()?;
        let bottom = self.read_bottom.clone();
        Some(
            div()
                .id("item-read")
                .test_support()
                .relative()
                .w_full()
                .child(read)
                .child(
                    canvas(
                        move |bounds, _, _| bottom.set(bounds.bottom()),
                        |_, _, _, _| {},
                    )
                    .absolute()
                    .inset_0(),
                )
                .into_any_element(),
        )
    }

    /// The page, holding `content`, in the region `scroll` scrolls.
    pub(super) fn render<C: IntoElement>(
        &self,
        content: C,
        scroll: &ContentScroll,
        window: &Window,
        cx: &Context<ItemPane>,
    ) -> impl IntoElement + use<C> {
        let colors = item_colors(cx);
        let height = scroll.height();
        let ring = self.focus.is_focused(window).then(|| {
            div()
                .absolute()
                .inset_0()
                .border_2()
                .border_color(cx.theme().ring)
        });
        let (top, sides, bottom) = PADDING;
        v_flex()
            .id("item-page")
            .test_support()
            .track_focus(&self.focus)
            .relative()
            .flex_none()
            .w_full()
            .min_h(height)
            .items_center()
            .bg(colors.page)
            .text_color(colors.page_text)
            .text_size(px(BODY_SIZE))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|pane, event: &MouseDownEvent, window, cx| {
                    pane.page.pressed(event, window, cx)
                }),
            )
            .child(
                div()
                    .id("item-page-column")
                    .test_support()
                    .w_full()
                    .max_w(px(COLUMN))
                    .pt(px(top))
                    .px(px(sides))
                    .pb(px(bottom))
                    .child(content),
            )
            .child(scroll.follow_height(height))
            .children(ring)
    }
}

impl ItemPane {
    /// An item tab shows the last successful read's body in its read
    /// editor, kept during an in-flight or failed read. An Inspect tab
    /// keeps D1's source view (`source`).
    pub(super) fn sync_read(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !matches!(
            self.model.target(),
            crate::desktop::model::navigation::OpenTarget::Item { .. }
        ) {
            return;
        }
        let Some(item) = self.model.item().rows() else {
            return;
        };
        self.page.show(item.body.as_deref(), window, cx);
        self.page.note_focus(window, cx);
    }
}

/// The caret and the text's length, which change when the caret moves or
/// the text changes.
fn caret(editor: &EditorState) -> (usize, usize) {
    (editor.cursor(), editor.text().len())
}

/// The item's scroll region, holding `page`, then `discussion`, then
/// `end`, the selected repository's details, with the Kit's scrollbar
/// over it.
pub(super) fn region<P: IntoElement, D: IntoElement>(
    scroll: &ContentScroll,
    page: P,
    discussion: Option<D>,
    end: Option<AnyElement>,
    cx: &App,
) -> impl IntoElement + use<P, D> {
    // One light thumb at rest, under the pointer and while dragged, so it
    // stays light against the page in either theme.
    let thumb = item_colors(cx).page_border_strong;
    let end = end.map(|end| {
        div()
            .id("item-region-end")
            .w_full()
            .pt_3()
            .pb_3()
            .child(end)
    });
    div()
        .id("item-region")
        .relative()
        .flex_1()
        .min_h_0()
        .overflow_hidden()
        .w_full()
        .child(
            v_flex()
                .id("item-scroll")
                .test_support()
                .size_full()
                .overflow_y_scroll()
                .track_scroll(scroll.handle())
                .child(page)
                .children(discussion)
                .children(end),
        )
        .child(
            div()
                .id("item-scrollbar")
                .test_support()
                .absolute()
                .inset_0()
                .child(
                    Scrollbar::vertical(scroll.handle())
                        .id("item-scrollbar-bar")
                        .viewport_from_layout()
                        .styles(|styles| {
                            styles
                                .thumb(|style| style.bg(thumb))
                                .thumb_hover(|style| style.bg(thumb))
                                .thumb_active(|style| style.bg(thumb))
                        }),
                ),
        )
}
