//! Scrolling the content region by keyboard (design, Layout And
//! Keyboard): PageUp and PageDown move it by most of its height, Home and
//! End to its top and bottom, and a control that takes focus outside the
//! visible part is scrolled into view.
//!
//! Settings, the Drafts page and the empty pane scroll the content
//! region's own column, as in D1. An item tab scrolls its own region below
//! its fixed tab strip and header (R-47): each item pane holds a
//! `ContentScroll` of its own, made by `region`, which shares the focus
//! the frame saw with the column's, so a control is revealed once, by the
//! region that draws it.
//!
//! GPUI scrolls a `div` by the wheel only, and nothing in GPUI 0.3.6 or the
//! Kit brings a focused element into view: `ScrollHandle::scroll_to_item`
//! knows only the scrolled element's direct children. So each control the
//! region can focus carries a `marker`, an empty canvas laid over it. When
//! the window's focus has moved since the last frame and the marker's
//! control is the one focused, the marker compares its bounds with the
//! region's and moves the region's offset the least that shows the
//! control, or its top when it is taller than the region. The next frame
//! draws the region at that offset. Focus that has not moved is never
//! revealed again, so scrolling away from a focused control is left alone.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use gpui_kit::{
    Bounds, FocusHandle, IntoElement, Pixels, ScrollHandle, Styled, WeakFocusHandle, Window,
    canvas, point, px,
};

/// How much of the visible height a page moves, so a line of the last
/// page stays in view.
const PAGE: f32 = 0.9;

/// A scrolled region's state, shared by the view that draws the region and
/// the views drawn inside it. Cloning shares it. The workspace's own is
/// the content region's column; `region` makes one for a region inside.
#[derive(Clone, Default)]
pub(crate) struct ContentScroll {
    handle: ScrollHandle,
    reveal: Rc<RefCell<Reveal>>,
    /// A press landed in the region since it was last laid out
    /// (`note_press`).
    pressed: Rc<Cell<bool>>,
}

#[derive(Default)]
struct Reveal {
    /// The focus the last frame saw.
    focused: Option<WeakFocusHandle>,
    /// Focus moved and is not yet brought into view.
    pending: bool,
}

impl ContentScroll {
    /// A region of its own inside the content area, such as an item's:
    /// a handle of its own, and the focus this one sees, so focus that
    /// moved is revealed once, by the region that draws its control.
    pub(crate) fn region(&self) -> Self {
        Self {
            handle: ScrollHandle::new(),
            reveal: self.reveal.clone(),
            pressed: Rc::default(),
        }
    }

    /// The handle the region's scrolled element tracks.
    pub(crate) fn handle(&self) -> &ScrollHandle {
        &self.handle
    }

    /// The region's visible height in the last frame.
    pub(crate) fn height(&self) -> Pixels {
        self.handle.bounds().size.height
    }

    /// An empty canvas to draw inside the region, for a child sized from
    /// the region's height in the last frame, `used`: when the region is
    /// laid out at another height, as on a window's first frame or a
    /// resize, it asks for the frame that sizes the child again.
    pub(crate) fn follow_height(&self, used: Pixels) -> impl IntoElement + use<> {
        let handle = self.handle.clone();
        canvas(
            move |_, window, cx| {
                if handle.bounds().size.height != used {
                    redraw(window, cx);
                }
            },
            |_, _, _, _| {},
        )
        .absolute()
        .size_0()
    }

    /// A press in the region, from its capture phase. Focus the press
    /// gives an editor reveals no caret (`editor::CaretInView`): the press
    /// is where the reader is looking. The press lasts until the region is
    /// next laid out (`press_ends`), in the frame this asks for, so it
    /// never outlives a press that changes nothing.
    pub(crate) fn note_press(&self, window: &mut Window) {
        self.pressed.set(true);
        window.refresh();
    }

    /// Whether a press landed in the region since it was last laid out.
    /// Views drawn inside the region read it as they render.
    pub(crate) fn pressed(&self) -> bool {
        self.pressed.get()
    }

    /// An empty canvas to draw with the region: once the views inside it
    /// have rendered and it is laid out, the last press is over.
    pub(crate) fn press_ends(&self) -> impl IntoElement + use<> {
        let pressed = self.pressed.clone();
        canvas(move |_, _, _| pressed.set(false), |_, _, _, _| {})
            .absolute()
            .size_0()
    }

    /// The window's focus as this frame begins: when it moved, the control
    /// that now holds it is brought into view as it is drawn.
    pub(crate) fn note_focus(&self, focused: Option<&FocusHandle>) {
        let mut reveal = self.reveal.borrow_mut();
        let moved = match (&reveal.focused, focused) {
            (Some(last), Some(now)) => last != now,
            (None, None) => false,
            _ => true,
        };
        if moved {
            reveal.focused = focused.map(FocusHandle::downgrade);
            reveal.pending = focused.is_some();
        }
    }

    /// Moves the region by `pages` of its visible height: down when
    /// positive. Gives whether it moved.
    pub(crate) fn page(&self, pages: f32) -> bool {
        let height = self.handle.bounds().size.height;
        let offset = self.handle.offset();
        self.scroll_to(offset.y - height * (pages * PAGE))
    }

    /// Moves the region to its top.
    pub(crate) fn to_top(&self) -> bool {
        self.scroll_to(px(0.))
    }

    /// Moves the region to its bottom.
    pub(crate) fn to_bottom(&self) -> bool {
        self.scroll_to(-self.handle.max_offset().y)
    }

    /// Sets the vertical offset, within the region's range. Gives whether
    /// it changed.
    fn scroll_to(&self, y: Pixels) -> bool {
        let max = self.handle.max_offset().y;
        let y = y.min(px(0.)).max(-max);
        let offset = self.handle.offset();
        if offset.y == y {
            return false;
        }
        self.handle.set_offset(point(offset.x, y));
        true
    }

    /// An empty canvas to lay over the control that `focus` focuses, so
    /// the control is scrolled into view when focus moves to it. The
    /// control must be `relative()`: the marker fills it.
    pub(crate) fn marker(&self, focus: &FocusHandle) -> impl IntoElement + use<> {
        self.marker_for(focus, false)
    }

    /// An empty canvas to lay over a wrapper that tracks `focus` around a
    /// control whose own focus is private, as a Kit button's is: the
    /// wrapper is scrolled into view when focus moves to anything inside
    /// it. The wrapper must be `relative()`.
    pub(crate) fn marker_within(&self, focus: &FocusHandle) -> impl IntoElement + use<> {
        self.marker_for(focus, true)
    }

    fn marker_for(&self, focus: &FocusHandle, within: bool) -> impl IntoElement + use<> {
        let this = self.clone();
        let focus = focus.clone();
        canvas(
            move |bounds, window, cx| this.prepainted(&focus, within, bounds, window, cx),
            |_, _, _, _| {},
        )
        .absolute()
        .inset_0()
    }

    /// The marker of `focus` was laid out at `bounds`, in the window, the
    /// region's offset applied. With `within`, focus anywhere inside the
    /// element that tracks `focus` counts.
    fn prepainted(
        &self,
        focus: &FocusHandle,
        within: bool,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut gpui_kit::App,
    ) {
        {
            let mut reveal = self.reveal.borrow_mut();
            let focused = if within {
                focus.contains_focused(window, cx)
            } else {
                focus.is_focused(window)
            };
            if !reveal.pending || !focused {
                return;
            }
            reveal.pending = false;
        }
        let viewport = self.handle.bounds();
        let offset = self.handle.offset();
        let y = if bounds.size.height >= viewport.size.height || bounds.top() < viewport.top() {
            offset.y + (viewport.top() - bounds.top())
        } else if bounds.bottom() > viewport.bottom() {
            offset.y - (bounds.bottom() - viewport.bottom())
        } else {
            return;
        };
        if self.scroll_to(y) {
            // The frame that draws the new offset.
            redraw(window, cx);
        }
    }
}

/// A window being drawn takes no refresh: asks for another frame once
/// this one is done.
fn redraw(window: &Window, cx: &mut gpui_kit::App) {
    let window = window.window_handle();
    cx.defer(move |cx| {
        let _ = window.update(cx, |_, window, _| window.refresh());
    });
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "scroll_tests.rs"]
mod tests;
