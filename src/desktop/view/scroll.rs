//! Scrolling the content region by keyboard (design, Layout And
//! Keyboard): PageUp and PageDown move it by most of its height, Home and
//! End to its top and bottom, and a control that takes focus outside the
//! visible part is scrolled into view.
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

use std::{cell::RefCell, rc::Rc};

use gpui_kit::{
    Bounds, FocusHandle, IntoElement, Pixels, ScrollHandle, Styled, WeakFocusHandle, Window,
    canvas, point, px,
};

/// How much of the visible height a page moves, so a line of the last
/// page stays in view.
const PAGE: f32 = 0.9;

/// The content region's scroll state, shared by the workspace that draws
/// the region and the views drawn inside it. Cloning shares it.
#[derive(Clone, Default)]
pub(crate) struct ContentScroll {
    handle: ScrollHandle,
    reveal: Rc<RefCell<Reveal>>,
}

#[derive(Default)]
struct Reveal {
    /// The focus the last frame saw.
    focused: Option<WeakFocusHandle>,
    /// Focus moved and is not yet brought into view.
    pending: bool,
}

impl ContentScroll {
    /// The handle the region's scrolled element tracks.
    pub(crate) fn handle(&self) -> &ScrollHandle {
        &self.handle
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
        let this = self.clone();
        let focus = focus.clone();
        canvas(
            move |bounds, window, cx| this.prepainted(&focus, bounds, window, cx),
            |_, _, _, _| {},
        )
        .absolute()
        .inset_0()
    }

    /// The marker of `focus` was laid out at `bounds`, in the window, the
    /// region's offset applied.
    fn prepainted(
        &self,
        focus: &FocusHandle,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut gpui_kit::App,
    ) {
        {
            let mut reveal = self.reveal.borrow_mut();
            if !reveal.pending || !focus.is_focused(window) {
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
            // A window being drawn takes no refresh: ask for the frame that
            // draws the new offset once this one is done.
            let window = window.window_handle();
            cx.defer(move |cx| {
                let _ = window.update(cx, |_, window, _| window.refresh());
            });
        }
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "scroll_tests.rs"]
mod tests;
