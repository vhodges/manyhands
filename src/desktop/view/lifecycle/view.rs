//! The lifecycle dialog drawn (D5 design, A2): each stage's body and its
//! buttons; its header is `stages.rs`'s, in the Kit's title slot. Text and
//! buttons carry their words as labels, for the tests to read.

use gpui_kit::{
    AnyElement, App, ClickEvent, Context, ElementId, InteractiveElement, IntoElement,
    ParentElement, Render, StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{
        ActiveTheme, WindowExt,
        button::{Button, ButtonVariants},
        dialog::Confirm,
        h_flex, v_flex,
    },
    div,
};
use manyhands::results::ResultCode;

use super::{
    CANCEL, CLOSE, HELD_LEAD, LifecycleDialog, PREPARE_AGAIN, PREPARING, Phase, RETRY, SAVING_FIRST,
};
use crate::desktop::view::frame;

/// A piece of text a test reads by `id`.
pub(super) type Text = gpui_kit::base::ObservedElement<gpui_kit::Stateful<gpui_kit::Div>>;

pub(super) fn text(id: impl Into<ElementId>, value: impl Into<String>) -> Text {
    let value = value.into();
    div()
        .id(id)
        .test_support()
        .aria_label(value.clone())
        .child(value)
}

/// A button a test reads by `id`. Enter on it presses it: its wrapper takes
/// the dialog's Enter before the dialog, whose `on_ok` would confirm.
pub(super) fn button(
    id: &'static str,
    label: impl Into<String>,
    primary: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let label = label.into();
    let on_click = std::rc::Rc::new(on_click);
    let on_enter = on_click.clone();
    let button = Button::new((id, 0usize))
        .label(label.clone())
        .on_click(move |event, window, cx| on_click(event, window, cx));
    let button = if primary { button.primary() } else { button };
    div()
        .id(id)
        .test_support()
        .aria_label(label)
        .on_action(move |_: &Confirm, window, cx| on_enter(&ClickEvent::default(), window, cx))
        .child(button)
        .into_any_element()
}

impl LifecycleDialog {
    fn cancel(&self, label: &'static str) -> AnyElement {
        let origin = self.origin.clone();
        button("lifecycle-cancel", label, false, move |_, window, cx| {
            window.close_dialog(cx);
            window.focus(&origin, cx);
        })
    }

    pub(super) fn on<F>(
        &self,
        cx: &Context<Self>,
        f: F,
    ) -> impl Fn(&ClickEvent, &mut Window, &mut App) + use<F>
    where
        F: Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    {
        let entity = cx.entity().downgrade();
        move |_, window, cx| {
            let _ = entity.update(cx, |dialog, cx| {
                // The pressed control may go with its stage (Confirm,
                // Retry, Unlock…) before a frame is drawn: focus moves to
                // the dialog first, so a passphrase prompt the worker opens
                // meanwhile gives focus back here, not to a gone button
                // (A21). `Workspace::focus_lost` covers controls that go
                // by themselves. A dialog the press opens takes focus after.
                window.focus(&dialog.focus, cx);
                f(dialog, window, cx)
            });
        }
    }

    fn answer(code: ResultCode, message: &str, cx: &App) -> impl IntoElement + use<> {
        v_flex()
            .gap_1()
            .child(text("lifecycle-message", message.to_owned()).text_sm())
            .child(
                text("lifecycle-code", code.as_str())
                    .text_xs()
                    .text_color(cx.theme().muted_foreground),
            )
    }
}

impl Render for LifecycleDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut body = v_flex()
            .id("lifecycle-dialog")
            .test_support()
            .track_focus(&self.focus)
            .gap_3();
        let mut buttons = h_flex().gap_2();
        match &self.phase {
            Phase::Saving => {
                body = body.child(text("lifecycle-state", SAVING_FIRST).text_sm());
                buttons = buttons.child(self.cancel(CANCEL));
            }
            Phase::Preparing => {
                body = body.child(text("lifecycle-state", PREPARING).text_sm());
                buttons = buttons.child(self.cancel(CANCEL));
            }
            Phase::Confirm { preview, .. } => {
                body = body.child(self.confirm_body(preview, cx));
                let confirm = self.on(cx, |dialog, _, cx| dialog.confirm(cx));
                buttons = buttons
                    .child(button(
                        "lifecycle-confirm",
                        self.intent.label(),
                        true,
                        confirm,
                    ))
                    .child(self.cancel(CANCEL));
            }
            Phase::Blocked {
                code,
                blockers,
                way_out,
                shared,
            } => {
                let (rows, actions) = self.blocked_body(*code, blockers, way_out, *shared, cx);
                body = body.child(rows);
                buttons = buttons.children(actions);
                // A closed ticket has nothing to prepare again for (P14).
                if *code != ResultCode::TicketClosed {
                    let again = self.on(cx, |dialog, _, cx| dialog.prepare(cx));
                    buttons = buttons.child(button(
                        "lifecycle-prepare-again",
                        PREPARE_AGAIN,
                        true,
                        again,
                    ));
                }
                buttons = buttons.child(self.cancel(CLOSE));
            }
            Phase::Held { way_out } => {
                body = body.child(text("lifecycle-lead", HELD_LEAD).text_sm());
                buttons = buttons
                    .children(self.held_action(way_out, cx))
                    .child(self.cancel(CLOSE));
            }
            Phase::Stale { code, message } => {
                body = body.child(Self::stale_body(*code, message, cx));
                let again = self.on(cx, |dialog, _, cx| dialog.prepare(cx));
                buttons = buttons
                    .child(button(
                        "lifecycle-prepare-again",
                        PREPARE_AGAIN,
                        true,
                        again,
                    ))
                    .child(self.cancel(CANCEL));
            }
            Phase::Answered { code, message } => {
                body = body.child(Self::answer(*code, message, cx));
                if *code == ResultCode::Busy {
                    let retry = self.on(cx, |dialog, _, cx| dialog.prepare(cx));
                    buttons = buttons.child(button("lifecycle-retry", RETRY, true, retry));
                }
                buttons = buttons.child(self.cancel(CLOSE));
            }
            Phase::Repaired => {
                let again = self.on(cx, |dialog, _, cx| dialog.prepare(cx));
                buttons = buttons
                    .child(button(
                        "lifecycle-prepare-again",
                        PREPARE_AGAIN,
                        true,
                        again,
                    ))
                    .child(self.cancel(CLOSE));
            }
            Phase::Message(why) => {
                body = body.child(text("lifecycle-message", *why).text_sm());
                buttons = buttons.child(self.cancel(CLOSE));
            }
            Phase::Submitted(id) => {
                let (shown, actions) = self.submitted_body(*id, cx);
                body = body.child(shown);
                buttons = buttons.children(actions).child(self.cancel(CLOSE));
            }
        }
        frame::body(body.children(self.repair_line(cx)).child(buttons))
    }
}
