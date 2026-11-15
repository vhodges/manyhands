//! A Kit button whose focus is a handle its page owns (Wave 04 Cycle 04,
//! the final review of Part D's landings).
//!
//! The Kit's `Button` takes no focus handle from its caller. It keeps its
//! own as element state keyed by its ID, tracks it only while enabled, and
//! draws its focus ring only while that handle is focused
//! (`gpui-component-0.6.6/src/button/button.rs:586-590`, `:773`, `:835`).
//! GPUI clicks on Enter or Space only the focused element that has the
//! click. So focus a page put on the button's wrapper, a route's or a
//! landing's, showed no ring and Enter did nothing.
//!
//! `kit_button` puts the page's handle in that state, keyed as the button
//! keys it, in the scope GPUI draws the button in, just before the button
//! draws. The handle the page focuses is then the button's own: its ring is
//! drawn and Enter presses it. While the button is disabled it tracks no
//! focus; its wrapper tracks the same handle, so focus put on it stays
//! there (R-53) and is on the button again, with no move, once it is
//! enabled.

use std::any::type_name;

use gpui_kit::{
    App, ElementId, FocusHandle, IntoElement, RenderOnce, Window, component::button::Button,
};

/// A Kit button drawn with a focus handle its page owns.
#[derive(IntoElement)]
pub(crate) struct KitButton {
    id: ElementId,
    focus: FocusHandle,
    button: Button,
}

/// `button`, whose element ID is `id`, drawn with `focus` as its own focus.
pub(crate) fn kit_button(
    id: impl Into<ElementId>,
    focus: &FocusHandle,
    button: Button,
) -> KitButton {
    KitButton {
        id: id.into(),
        focus: focus.clone(),
        button,
    }
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
