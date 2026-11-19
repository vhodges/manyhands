//! The lifecycle dialog drawn (D5 design, A2): each stage's body, and its
//! footer, the body element's last child (Cycle 05b A2, A7; C5a R-19); its
//! header is `stages.rs`'s, in the Kit's title slot. The footer puts the
//! dismiss button first, then the stage's other buttons, then its primary
//! ones, each kept in the order the stage gives them; on Done, Done comes
//! last. Text and buttons carry their words as labels, for the tests to
//! read.

use gpui_kit::{
    AnyElement, App, ClickEvent, Context, ElementId, InteractiveElement, IntoElement,
    ParentElement, Render, StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{
        ActiveTheme, WindowExt,
        button::{Button, ButtonVariants},
        dialog::Confirm,
        v_flex,
    },
    div,
};
use manyhands::results::ResultCode;

use super::{
    HELD_LEAD, LifecycleDialog, PREPARE_AGAIN, PREPARING, Phase, RETRY, SAVING_FIRST, stages::Stage,
};
use crate::desktop::view::frame::{self, Footer};

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

/// A button a test reads by `id`, with whether it is primary, which the
/// footer orders by (A7). Enter on it presses it: its wrapper takes the
/// dialog's Enter before the dialog, whose `on_ok` would confirm.
pub(super) fn button(
    id: &'static str,
    label: impl Into<String>,
    primary: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> (bool, AnyElement) {
    let label = label.into();
    let on_click = std::rc::Rc::new(on_click);
    let on_enter = on_click.clone();
    let button = Button::new((id, 0usize))
        .label(label.clone())
        .on_click(move |event, window, cx| on_click(event, window, cx));
    let button = if primary { button.primary() } else { button };
    let element = div()
        .id(id)
        .test_support()
        .aria_label(label)
        .on_action(move |_: &Confirm, window, cx| on_enter(&ClickEvent::default(), window, cx))
        .child(button)
        .into_any_element();
    (primary, element)
}

impl LifecycleDialog {
    /// The dismiss button, labelled by the stage: it closes the dialog and
    /// focuses the origin, as Escape does.
    fn dismiss(&self, label: &'static str, primary: bool) -> AnyElement {
        let origin = self.origin.clone();
        let (_, dismiss) = button("lifecycle-cancel", label, primary, move |_, window, cx| {
            window.close_dialog(cx);
            window.focus(&origin, cx);
        });
        dismiss
    }

    /// The footer's buttons at `stage` (A7): the dismiss button first, then
    /// the non-primary `actions`, then the primary ones, each in the order
    /// given; on Done, Done comes last, and is primary.
    fn footer_buttons(&self, stage: Stage, actions: Vec<(bool, AnyElement)>) -> Vec<AnyElement> {
        let done = stage == Stage::Done;
        let dismiss = self
            .dismiss_label(stage)
            .map(|label| self.dismiss(label, done));
        let (first, last) = if done {
            (None, dismiss)
        } else {
            (dismiss, None)
        };
        let (primary, others): (Vec<_>, Vec<_>) =
            actions.into_iter().partition(|(primary, _)| *primary);
        first
            .into_iter()
            .chain(
                others
                    .into_iter()
                    .chain(primary)
                    .map(|(_, element)| element),
            )
            .chain(last)
            .collect()
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Repair locally…'s sheet, waiting for focus back inside (R-24).
        self.conflicted_waits(window, cx);
        let stage = self.stage(cx);
        let mut body = v_flex().gap_3();
        let mut actions = Vec::new();
        match &self.phase {
            Phase::Saving => {
                body = body.child(text("lifecycle-state", SAVING_FIRST).text_sm());
            }
            Phase::Preparing => {
                body = body.child(text("lifecycle-state", PREPARING).text_sm());
            }
            Phase::Confirm { preview, .. } => {
                body = body.child(self.confirm_body(preview, cx));
                let confirm = self.on(cx, |dialog, _, cx| dialog.confirm(cx));
                actions.push(button(
                    "lifecycle-confirm",
                    self.intent.label(),
                    true,
                    confirm,
                ));
            }
            Phase::Blocked {
                code,
                blockers,
                way_out,
                shared,
            } => {
                let (rows, way_out) = self.blocked_body(*code, blockers, way_out, *shared, cx);
                body = body.child(rows);
                actions = way_out;
                // A closed ticket has nothing to prepare again for (P14).
                if *code != ResultCode::TicketClosed {
                    let again = self.on(cx, |dialog, _, cx| dialog.prepare(cx));
                    actions.push(button(
                        "lifecycle-prepare-again",
                        PREPARE_AGAIN,
                        true,
                        again,
                    ));
                }
            }
            Phase::Held { way_out } => {
                body = body.child(text("lifecycle-lead", HELD_LEAD).text_sm());
                actions.extend(self.held_action(way_out, cx));
            }
            Phase::Stale { code, message } => {
                body = body.child(Self::stale_body(*code, message, cx));
                let again = self.on(cx, |dialog, _, cx| dialog.prepare(cx));
                actions.push(button(
                    "lifecycle-prepare-again",
                    PREPARE_AGAIN,
                    true,
                    again,
                ));
            }
            Phase::Answered { code, message } => {
                body = body.child(Self::answer(*code, message, cx));
                if *code == ResultCode::Busy {
                    let retry = self.on(cx, |dialog, _, cx| dialog.prepare(cx));
                    actions.push(button("lifecycle-retry", RETRY, true, retry));
                }
            }
            Phase::Repaired => {
                let again = self.on(cx, |dialog, _, cx| dialog.prepare(cx));
                actions.push(button(
                    "lifecycle-prepare-again",
                    PREPARE_AGAIN,
                    true,
                    again,
                ));
            }
            Phase::Message(why) => {
                body = body.child(text("lifecycle-message", *why).text_sm());
            }
            Phase::Submitted(id) => {
                let (shown, sent) = self.submitted_body(*id, cx);
                body = body.child(shown);
                actions = sent;
            }
        }
        let footer = Footer {
            hint: self.hint(stage),
            buttons: self.footer_buttons(stage, actions),
        };
        // The footer is the last child of the element that tracks the
        // dialog's focus, inside which the privacy sweeps and focus reads
        // find the buttons (C5a R-19). A stage with more buttons than fit
        // the 580 px dialog wraps them onto another line, still in order,
        // rather than drawing them past its edge, where a click misses.
        v_flex()
            .id("lifecycle-dialog")
            .test_support()
            .track_focus(&self.focus)
            .child(frame::body(body.children(self.repair_line(cx))))
            .child(frame::footer(footer, cx).flex_wrap().id("lifecycle-footer"))
    }
}
