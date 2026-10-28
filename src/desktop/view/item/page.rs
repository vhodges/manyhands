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
//! focus, and when it takes focus. Its links go to the pane (decision 10;
//! `banners`). It draws the local images its body names from the tab's
//! images, which each read locates in the item's trees and folder
//! (decision 9). A press on the page that no control takes focuses the read
//! editor, its caret at the end when the press is below the text; with no
//! read view, focus goes into the content region.
//! The page's ring shows only after Back to top, never after a press.
//!
//! The region shows the Kit's vertical scrollbar on its own handle, thumb
//! in `page-border-strong` at rest, hovered and dragged, laid over the
//! region as the Kit's own scrollables lay it. The discussion's floating
//! button lies over the region too (`discussion`).

use std::{cell::Cell, rc::Rc};

use gpui_kit::{
    AnyElement, App, AppContext, Context, Entity, FocusHandle, InteractiveElement, IntoElement,
    MouseButton, MouseDownEvent, ParentElement, Pixels, StatefulInteractiveElement, Styled,
    Subscription, TestSupportExt, Window, canvas,
    component::{ActiveTheme, scroll::Scrollbar, v_flex},
    div, px,
};

use super::{ItemPane, tokens::item_colors};
use crate::desktop::{
    model::{links::LinkTarget, navigation::OpenTarget},
    services::images::{ImageSource, TabImages},
    view::{
        editor::{CaretInView, read_only::ReadOnly},
        scroll::ContentScroll,
    },
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
    /// The page's ring shows while it holds focus from Back to top, not
    /// from a press.
    ring: bool,
    /// The body's read editor, once a read gave a body.
    read: Option<Entity<ReadOnly>>,
    /// The body `read` last loaded, as the library read it.
    body: String,
    /// The read editor's caret reveal (R-67).
    reveal: Option<CaretInView>,
    /// The links clicked in the read editor go to the pane (decision 10).
    links: Option<Subscription>,
    /// The bottom of the read view in the last frame, in the window.
    read_bottom: Rc<Cell<Pixels>>,
}

impl Page {
    pub(super) fn new(cx: &mut App) -> Self {
        Self {
            focus: cx.focus_handle(),
            ring: false,
            read: None,
            body: String::new(),
            reveal: None,
            links: None,
            read_bottom: Rc::default(),
        }
    }

    #[cfg(test)]
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
    /// The read editor keeps its caret in view in `scroll`'s region, and
    /// draws its images from `images`.
    fn show(
        &mut self,
        body: Option<&str>,
        scroll: &ContentScroll,
        images: &TabImages,
        window: &mut Window,
        cx: &mut Context<ItemPane>,
    ) {
        let Some(body) = body else {
            self.read = None;
            self.reveal = None;
            self.links = None;
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
        read.update(cx, |read, cx| read.show_images(images, cx));
        let editor = read.read(cx).editor().clone();
        self.reveal = Some(CaretInView::new(&editor, scroll, window, cx));
        self.links = Some(cx.subscribe(&read, |pane, _, target: &LinkTarget, cx| {
            pane.follow_link(target, cx)
        }));
        self.read = Some(read);
        self.body = body.to_owned();
    }

    /// R-67: when the read editor has taken focus since the last frame,
    /// its caret is brought into view once this frame is drawn.
    fn note_focus(&mut self, window: &Window, cx: &mut Context<ItemPane>) {
        if let Some(reveal) = &mut self.reveal {
            reveal.note_focus(window, cx);
        }
    }

    /// Back to top: the page focused, with its ring.
    pub(super) fn focus_from_back_to_top(&mut self, window: &mut Window, cx: &mut App) {
        self.ring = true;
        window.focus(&self.focus, cx);
    }

    /// A left press on the page. A press never rings the page: the ring is
    /// Back to top's. One that no control took focuses the read editor,
    /// with its caret at the end of the body when the press is below the
    /// text, and where it was otherwise. With no read view, the press goes
    /// on, so focus goes into the content region, as D1's did.
    fn pressed(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut App) {
        self.ring = false;
        if window.default_prevented() {
            return;
        }
        let Some(read) = &self.read else {
            return;
        };
        window.prevent_default();
        let below = event.position.y >= self.read_bottom.get();
        let editor = read.read(cx).editor().clone();
        editor.update(cx, |editor, cx| {
            if below {
                editor.set_cursor(editor.text().len(), cx);
            }
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
        let ring = (self.ring && self.focus.is_focused(window)).then(|| {
            div()
                .id("item-page-ring")
                .test_support()
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
    /// An item tab's images, and its comments', resolve from where the
    /// last successful read found the item. Called as each read lands, so
    /// no frame builds a place only to compare it.
    pub(super) fn locate_images(&self, cx: &mut App) {
        if !matches!(self.model.target(), OpenTarget::Item { .. }) {
            return;
        }
        if let Some(item) = self.model.item().rows() {
            self.images.locate(ImageSource::of(item, self.repo()), cx);
        }
    }

    /// An item tab shows the last successful read's body in its read
    /// editor, kept during an in-flight or failed read. An Inspect tab
    /// keeps D1's source view (`source`).
    pub(super) fn sync_read(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !matches!(self.model.target(), OpenTarget::Item { .. }) {
            return;
        }
        let Some(item) = self.model.item().rows() else {
            return;
        };
        self.page
            .show(item.body.as_deref(), &self.scroll, &self.images, window, cx);
        self.page.note_focus(window, cx);
    }
}

/// The discussion's place among the scrolled children `region` lays out,
/// after the page, for `ScrollHandle::bounds_for_item` and
/// `scroll_to_top_of_item` (fact 80).
pub(super) const DISCUSSION_CHILD: usize = 1;

/// The item's scroll region, holding `page`, then `discussion`, then
/// `end`, the selected repository's details, with the Kit's scrollbar
/// over it, and then `over`, laid over the region after it.
pub(super) fn region<P: IntoElement, D: IntoElement>(
    scroll: &ContentScroll,
    page: P,
    discussion: Option<D>,
    end: Option<AnyElement>,
    over: Vec<AnyElement>,
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
        .children(over)
}
