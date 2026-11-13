//! A failed composer's row (design, Lane C, States): the owner's message
//! in red, then exactly the actions the owner offers (`FailedActions`).
//! The composer decides none of them. Retry, Set identity, Check again and
//! Post as a new comment (P5, P16; the owner allocates the new ID) go to
//! the owner as `Failed` events; Edit text goes back to `Idle` and
//! tells the owner (P6); Discard asks first (`discard`); Copy text puts
//! the text on the clipboard (P7, P12) and tells no one.

use gpui_kit::{
    ClickEvent, ClipboardItem, Context, Focusable, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{ActiveTheme, Sizable, button::Button, h_flex},
    div, px,
};

use super::{Composer, ComposerEvent, ComposerState};

/// The actions a failed composer offers; its owner sets them from the
/// outcome. It holds no text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct FailedActions {
    pub(crate) retry: bool,
    pub(crate) edit_text: bool,
    pub(crate) set_identity: bool,
    pub(crate) discard: bool,
    pub(crate) check_again: bool,
    pub(crate) copy_text: bool,
    pub(crate) post_as_new: bool,
}

/// An action a failed composer tells its owner of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FailedAction {
    Retry,
    /// The composer is `Idle` again, its text editable.
    EditText,
    SetIdentity,
    CheckAgain,
    /// Post the same text under a new comment ID, which the owner
    /// allocates (P5, P16).
    PostAsNew,
}

/// One control of the row.
#[derive(Clone, Copy)]
enum Control {
    Retry,
    EditText,
    SetIdentity,
    CheckAgain,
    PostAsNew,
    CopyText,
    Discard,
}

impl Control {
    fn id(self) -> &'static str {
        match self {
            Self::Retry => "composer-retry",
            Self::EditText => "composer-edit-text",
            Self::SetIdentity => "composer-set-identity",
            Self::CheckAgain => "composer-check-again",
            Self::PostAsNew => "composer-post-as-new",
            Self::CopyText => "composer-copy-text",
            Self::Discard => "composer-discard",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Retry => "Retry",
            Self::EditText => "Edit text",
            Self::SetIdentity => "Set identity",
            Self::CheckAgain => "Check again",
            Self::PostAsNew => "Post as a new comment",
            Self::CopyText => "Copy text",
            Self::Discard => "Discard",
        }
    }
}

impl FailedActions {
    /// The controls offered, in the order they are drawn.
    fn controls(self) -> impl Iterator<Item = Control> {
        [
            (self.retry, Control::Retry),
            (self.set_identity, Control::SetIdentity),
            (self.check_again, Control::CheckAgain),
            (self.post_as_new, Control::PostAsNew),
            (self.edit_text, Control::EditText),
            (self.copy_text, Control::CopyText),
            (self.discard, Control::Discard),
        ]
        .into_iter()
        .filter_map(|(offered, control)| offered.then_some(control))
    }
}

impl Composer {
    /// Does what `control` names.
    fn act(&mut self, control: Control, window: &mut Window, cx: &mut Context<Self>) {
        let event = match control {
            Control::Retry => FailedAction::Retry,
            Control::SetIdentity => FailedAction::SetIdentity,
            Control::CheckAgain => FailedAction::CheckAgain,
            Control::PostAsNew => FailedAction::PostAsNew,
            Control::EditText => {
                self.set_state(ComposerState::Idle, window, cx);
                let editor = self.editor.read(cx).focus_handle(cx);
                window.focus(&editor, cx);
                FailedAction::EditText
            }
            Control::CopyText => {
                cx.write_to_clipboard(ClipboardItem::new_string(self.text(cx)));
                return;
            }
            Control::Discard => {
                self.ask_discard(window, cx);
                return;
            }
        };
        cx.emit(ComposerEvent::Failed(event));
    }

    /// The failed row: the message in red and the actions offered; nothing
    /// in any other state.
    pub(super) fn failed_row(&self, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
        let ComposerState::Failed {
            message, actions, ..
        } = &self.state
        else {
            return None;
        };
        let mut row = h_flex()
            .w_full()
            .flex_wrap()
            .items_center()
            .gap(px(8.))
            .px(px(12.))
            .py(px(6.))
            .child(
                div()
                    .id("composer-failed")
                    .test_support()
                    .aria_label(message.clone())
                    .flex_1()
                    .min_w_0()
                    .text_size(px(12.))
                    .text_color(cx.theme().danger)
                    .child(message.clone()),
            );
        for control in actions.controls() {
            row = row.child(
                div()
                    .id(control.id())
                    .test_support()
                    .aria_label(control.label())
                    .flex_none()
                    .child(
                        Button::new(format!("{}-button", control.id()))
                            .label(control.label())
                            .small()
                            .outline()
                            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                                this.act(control, window, cx)
                            })),
                    ),
            );
        }
        Some(row)
    }
}
