//! The lifecycle dialog drawn (D5 design, A2): its header, each stage's
//! body and its buttons. Text and buttons carry their words as labels, for
//! the tests to read.

use gpui_kit::{
    AnyElement, App, ClickEvent, Context, ElementId, FontWeight, InteractiveElement, IntoElement,
    ParentElement, Render, StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{
        ActiveTheme, Icon, WindowExt,
        button::{Button, ButtonVariants},
        dialog::Confirm,
        h_flex, v_flex,
    },
    div, px,
};
use manyhands::{
    repository::{ItemDtoKind, PreviewDetailsDto},
    results::ResultCode,
};

use super::{
    CANCEL, CLOSE, HELD_LEAD, LifecycleDialog, PREPARE_AGAIN, PREPARING, Phase, RETRY, SAVING_FIRST,
};
use crate::desktop::assets::icon_path;

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
    /// The kind's icon, the command, who it speaks of and the branch.
    fn header(&self, cx: &App) -> impl IntoElement + use<> {
        let muted = cx.theme().muted_foreground;
        let (icon, kind) = match self.heading.kind {
            ItemDtoKind::Ticket => ("ticket", "ticket"),
            ItemDtoKind::Document => ("file-text", "document"),
        };
        let branch = match &self.phase {
            Phase::Confirm { preview, .. } => match &preview.details {
                PreviewDetailsDto::Lifecycle(lifecycle) => Some(lifecycle.branch.clone()),
                _ => self.heading.branch.clone(),
            },
            // The preview the run was confirmed with names its branch.
            Phase::Submitted(_) => self
                .plan
                .as_ref()
                .map(|plan| plan.branch.clone())
                .or_else(|| self.heading.branch.clone()),
            _ => self.heading.branch.clone(),
        };
        h_flex()
            .gap_3()
            .items_center()
            .child(
                div()
                    .id("lifecycle-kind")
                    .test_support()
                    .aria_label(kind)
                    .flex_none()
                    .child(Icon::default().path(icon_path(icon)).w(px(20.)).h(px(20.))),
            )
            .child(
                v_flex()
                    .min_w_0()
                    .gap_1()
                    .child(
                        text("lifecycle-title", self.intent.label())
                            .font_weight(FontWeight::SEMIBOLD),
                    )
                    .child(
                        text("lifecycle-subtitle", self.heading.subtitle())
                            .text_sm()
                            .text_color(muted),
                    )
                    .children(branch.map(|branch| {
                        text("lifecycle-branch", branch).text_xs().text_color(muted)
                    })),
            )
    }

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
                // The pressed control may go with its stage (Confirm, Stop,
                // Retry): focus stays in the dialog, where Tab, Enter and
                // Escape still reach it (A21). A dialog the press opens
                // takes focus after.
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
            .gap_3()
            .child(self.header(cx));
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
        body.children(self.repair_line(cx)).child(buttons)
    }
}
