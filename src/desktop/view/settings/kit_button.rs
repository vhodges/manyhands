//! A Kit button whose focus is a handle its page owns (Wave 04 Cycle 04,
//! the final review of Part D's landings), in Settings and in the Add
//! repository sheet (Wave 04 Cycle 05a B12).
//!
//! The Kit's `Button` takes no focus handle from its caller. It keeps its
//! own as element state keyed by its ID, tracks it only while enabled, and
//! draws its focus ring only while that handle is focused
//! (`gpui-component-0.6.6/src/button/button.rs:586-590`, `:773`, `:835`).
//! GPUI clicks on Enter or Space only the focused element that has the
//! click. So focus a page put on the button's wrapper, a route's or a
//! landing's, showed no ring and Enter did nothing.
//!
//! `wrapped_kit` puts the page's handle in that state, keyed as the button
//! keys it, in the scope GPUI draws the button in, just before the button
//! draws. The handle the page focuses is then the button's own: its ring is
//! drawn and Enter presses it. While the button is disabled it tracks no
//! focus; its wrapper tracks the same handle, so focus put on it stays
//! there (R-53) and is on the button again, with no move, once it is
//! enabled.
//!
//! A handle's tab-stop flag is shared by all its clones
//! (`gpui-pre-0.3.6/src/window.rs:558-569`, `:584-590`, `:640-643`), and
//! the enabled button sets it (`gpui-base-0.6.6/src/button.rs:235-241`). So
//! the wrapper's handle is set back to no tab stop as the page draws it,
//! before the button draws: only the button's own place is a Tab stop, so
//! Shift+Tab moves back past it, and Tab passes it by while it is disabled.

use std::any::type_name;

use gpui_kit::{
    App, ElementId, FocusHandle, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::button::Button, div,
};

use super::{Wrapped, wrapped};
use crate::desktop::view::scroll::ContentScroll;

/// A Kit button drawn with a focus handle its page owns.
#[derive(IntoElement)]
struct KitButton {
    id: ElementId,
    focus: FocusHandle,
    button: Button,
}

/// `button`, whose element ID is `button_id`, in the wrapper `id` named
/// `label` (`wrapped`), with `focus` as its own focus. The wrapper tracks
/// `focus` too, as no tab stop.
pub(crate) fn wrapped_kit(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    focus: &FocusHandle,
    scroll: &ContentScroll,
    button_id: impl Into<ElementId>,
    button: Button,
) -> Wrapped {
    let held = focus.clone().tab_stop(false);
    let button = KitButton {
        id: button_id.into(),
        focus: focus.clone(),
        button,
    };
    wrapped(id, label, &held, scroll, button)
}

/// `wrapped_kit` for a button outside every scrolled region, as a sheet's
/// footer is (Wave 04 Cycle 05a B10, B12): the same handle and wrapper, and
/// no scroll marker, which would scroll a body the button is not in.
pub(crate) fn footer_kit(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    focus: &FocusHandle,
    button_id: impl Into<ElementId>,
    button: Button,
) -> Wrapped {
    let held = focus.clone().tab_stop(false);
    let button = KitButton {
        id: button_id.into(),
        focus: focus.clone(),
        button,
    };
    div()
        .id(id)
        .test_support()
        .aria_label(label)
        .relative()
        .track_focus(&held)
        .child(button)
}

impl RenderOnce for KitButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self { id, focus, button } = self;
        // GPUI draws a component in a scope named for its type, where the
        // Kit's button reads its focus, keyed by its ID.
        window.with_id(ElementId::from(type_name::<Button>()), |window| {
            let state = window.use_keyed_state(id, cx, |_, _| focus.clone());
            if *state.read(cx) != focus {
                state.update(cx, |held, _| *held = focus);
            }
        });
        button
    }
}
