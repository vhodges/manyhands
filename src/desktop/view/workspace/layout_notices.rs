//! The layout file's two notices (Part G; R-21): the restore notice and the
//! save notice, above the operation area in Activity. Each has a Dismiss
//! button and lasts for the session; neither counts in Needs attention.

use gpui_kit::{
    AnyElement, App, ClickEvent, Context, ElementId, FocusHandle, InteractiveElement, IntoElement,
    ParentElement, StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{ActiveTheme, button::Button, h_flex},
    div,
};

use super::Workspace;
use crate::desktop::view::operations::DISMISS;

const RESTORE_NOTICE: &str =
    "Your panel layout couldn't be restored, so the default layout is back.";
const SAVE_NOTICE: &str = "Your panel layout couldn't be saved.";

impl Workspace {
    /// The notices that show, in order.
    pub(super) fn layout_notices(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let notices = self.dock.notices();
        let mut shown = Vec::new();
        if notices.restore {
            shown.push(notice(
                "layout-restore",
                RESTORE_NOTICE,
                &notices.restore_focus,
                window,
                cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.dock.notices_mut().restore = false;
                    cx.notify();
                }),
                cx,
            ));
        }
        if notices.save {
            shown.push(notice(
                "layout-save",
                SAVE_NOTICE,
                &notices.save_focus,
                window,
                cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.dock.notices_mut().save = false;
                    cx.notify();
                }),
                cx,
            ));
        }
        shown
    }
}

/// One notice: its text and Dismiss. The wrapper around Dismiss holds the
/// focus and presses the button on Enter or Space; the Kit button inside
/// only draws it and takes a pointer's click.
fn notice(
    id: &'static str,
    text: &'static str,
    focus: &FocusHandle,
    window: &Window,
    dismiss: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    let dismiss = std::rc::Rc::new(dismiss);
    let pressed = dismiss.clone();
    let button_id = ElementId::Name(format!("{id}-dismiss").into());
    let ring = focus.is_focused(window).then(|| {
        div()
            .id(ElementId::Name(format!("{id}-dismiss-ring").into()))
            .test_support()
            .absolute()
            .inset_0()
            .rounded_sm()
            .border_2()
            .border_color(cx.theme().ring)
    });
    h_flex()
        .gap_2()
        .items_center()
        .child(
            div()
                .id(ElementId::Name(format!("{id}-notice").into()))
                .test_support()
                .aria_label(text)
                .flex_1()
                .min_w_0()
                .child(text),
        )
        .child(
            div()
                .id(button_id.clone())
                .test_support()
                .aria_label(DISMISS)
                .track_focus(focus)
                .flex_none()
                .relative()
                .rounded_sm()
                .on_click(move |event, window, cx| {
                    if event.is_keyboard() {
                        pressed(event, window, cx);
                    }
                })
                .child(
                    Button::new(ElementId::NamedChild(
                        std::sync::Arc::new(button_id),
                        "button".into(),
                    ))
                    .label(DISMISS)
                    .tab_stop(false)
                    .on_click(move |event, window, cx| dismiss(event, window, cx)),
                )
                .children(ring),
        )
        .into_any_element()
}
