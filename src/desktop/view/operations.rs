//! The operation area: this session's requests and each repository's
//! pending operations.
//!
//! **This session** lists every desktop request, newest first, with its
//! command, scope and state, and while it runs its latest stage. A
//! success or no-op is one line. A partial, blocked, failed or cancelled
//! request stays expanded, with its message, code, effects and recovery
//! actions, until it is dismissed. A queued request, or a running one the
//! library has not accepted, offers Cancel; an accepted one reads
//! "Finishing".
//!
//! **Pending in repositories** lists the operations each listed
//! repository reports with work outstanding: command, scope, operation ID
//! and what it waits for. A local operation waiting to be resumed offers
//! Resume; a synchronization shows Resume unavailable. A pending local
//! operation offers Abandon, which is confirmed.
//!
//! Every recovery action an envelope offers is a control: the ones D1
//! offers run, the rest are text, unavailable. Text is fixed, the
//! envelope's message or a code's own.

use std::path::Path;

use gpui_kit::{
    AnyElement, App, ClickEvent, Context, ElementId, InteractiveElement, IntoElement,
    ParentElement, StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{
        ActiveTheme, Disableable,
        button::{Button, ButtonVariants},
        h_flex, v_flex,
    },
    div,
};
use manyhands::results::{Effects, Outcome, RecoveryAction};

use super::workspace::Workspace;
use crate::desktop::{
    model::{
        list_state::ListState,
        operations::{
            NOT_AVAILABLE_YET, ResumeOffer, abandon_offered, recovery_control, recovery_label,
            resume_offer,
        },
        stamp::RepoKey,
    },
    services::{
        KEY_STORE_UNAVAILABLE,
        requests::{DesktopRequest, DesktopRequestId, NotSent, RequestState},
    },
};

pub(crate) const THIS_SESSION: &str = "This session";
pub(crate) const PENDING: &str = "Pending in repositories";
pub(crate) const NOTHING_PENDING: &str = "No pending operations";
pub(crate) const NO_REQUESTS: &str = "No requests yet";
pub(crate) const QUEUED: &str = "Queued";
pub(crate) const RUNNING: &str = "Running";
pub(crate) const FINISHING: &str = "Finishing";
pub(crate) const CANCELLED: &str = "Cancelled";
pub(crate) const NOT_SENT: &str = "Not sent";
pub(crate) const CANCEL: &str = "Cancel";
pub(crate) const DISMISS: &str = "Dismiss";
pub(crate) const DISMISSED: &str = "Dismissed";
pub(crate) const RESUME: &str = "Resume";
pub(crate) const ABANDON: &str = "Abandon";
pub(crate) const APPLICATION: &str = "Application";
pub(crate) const NO_NEXT_ACTION: &str = "Nothing to do";
pub(crate) const NO_OPERATION_ID: &str = "No operation ID";

/// An outcome in fixed text.
pub(crate) fn outcome_label(outcome: Outcome) -> &'static str {
    match outcome {
        Outcome::Success => "Done",
        Outcome::Noop => "No change",
        Outcome::Partial => "Partly done",
        Outcome::Blocked => "Blocked",
        Outcome::Cancelled => "Cancelled",
        Outcome::Error => "Failed",
    }
}

/// A request's state in fixed text.
pub(crate) fn state_label(state: &RequestState) -> String {
    match state {
        RequestState::Queued => QUEUED.to_owned(),
        RequestState::Running { accepted: true, .. } => FINISHING.to_owned(),
        RequestState::Running { stage: None, .. } => RUNNING.to_owned(),
        RequestState::Running {
            stage: Some(stage), ..
        } => format!("{RUNNING}: {}", stage.as_str()),
        RequestState::Finished(outcome, ..) => outcome_label(*outcome).to_owned(),
        RequestState::Removed => CANCELLED.to_owned(),
        RequestState::Dismissed => DISMISSED.to_owned(),
        RequestState::NotSent(NotSent::SecretRequired) => {
            format!("{NOT_SENT}: the passphrase is needed again")
        }
        RequestState::NotSent(NotSent::KeyStoreUnavailable) => {
            format!("{NOT_SENT}: {KEY_STORE_UNAVAILABLE}")
        }
    }
}

/// What a request or operation is about: its repository's directory
/// name, or the application for a key command.
fn scope_name(root: Option<&Path>) -> String {
    match root {
        Some(root) => root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| root.to_string_lossy().into_owned()),
        None => APPLICATION.to_owned(),
    }
}

/// The effects a request reports, each one it was asked for, in the
/// contract's names.
pub(crate) fn effects_text(effects: &Effects) -> String {
    let named = [
        ("write", effects.write.as_str()),
        ("checkpoint", effects.checkpoint.as_str()),
        ("discovery", effects.discovery.as_str()),
        ("publication", effects.publication.as_str()),
        ("integration", effects.integration.as_str()),
        ("cleanup", effects.cleanup.as_str()),
    ];
    let shown: Vec<String> = named
        .into_iter()
        .filter(|(_, value)| *value != "not_requested")
        .map(|(name, value)| format!("{name}: {value}"))
        .collect();
    if shown.is_empty() {
        "No effects".to_owned()
    } else {
        shown.join(", ")
    }
}

/// The element ids of a request's parts: stable across reorders, since
/// they carry the request's number.
pub(crate) fn request_id(id: DesktopRequestId, part: &str) -> String {
    format!("request-{}-{part}", id.0)
}

pub(crate) fn recovery_id(id: DesktopRequestId, n: usize) -> String {
    format!("request-{}-recovery-{n}", id.0)
}

type Text = gpui_kit::base::test_support::Observed<gpui_kit::Stateful<gpui_kit::Div>>;

fn text(id: impl Into<ElementId>, value: impl Into<String>) -> Text {
    let value = value.into();
    div()
        .id(id)
        .test_support()
        .aria_label(value.clone())
        .child(value)
}

fn button(
    id: impl Into<ElementId>,
    label: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let id = id.into();
    div()
        .id(id.clone())
        .test_support()
        .aria_label(label)
        .child(
            Button::new(ElementId::NamedChild(
                std::sync::Arc::new(id),
                "button".into(),
            ))
            .label(label)
            .on_click(on_click),
        )
        .into_any_element()
}

/// A control D1 does not offer: disabled, and saying so in text.
fn unavailable(id: impl Into<ElementId>, label: &str, why: &str, cx: &App) -> Text {
    let shown = format!("{label} ({why})");
    text(id, shown)
        .text_xs()
        .text_color(cx.theme().muted_foreground)
}

impl Workspace {
    /// One request of this session.
    fn session_entry(
        &self,
        request: &DesktopRequest,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let id = request.id;
        let muted = cx.theme().muted_foreground;
        let line = h_flex()
            .gap_2()
            .child(text(request_id(id, "command"), request.intent.label()).text_sm())
            .child(
                text(request_id(id, "scope"), scope_name(request.intent.root()))
                    .text_xs()
                    .text_color(muted),
            )
            .child(text(request_id(id, "state"), state_label(&request.state)).text_xs());
        let mut entry = v_flex()
            .id(request_id(id, "entry"))
            .test_support()
            .gap_1()
            .child(line);
        let mut controls = h_flex().gap_2().flex_wrap();
        match &request.state {
            RequestState::Queued
            | RequestState::Running {
                accepted: false, ..
            } => {
                controls = controls.child(button(
                    request_id(id, "cancel"),
                    CANCEL,
                    cx.listener(move |this, _: &ClickEvent, _, cx| this.cancel_request(id, cx)),
                ));
            }
            RequestState::Running { accepted: true, .. } => {
                controls = controls.child(
                    div().child(
                        Button::new(ElementId::Name(request_id(id, "finishing").into()))
                            .label(FINISHING)
                            .disabled(true),
                    ),
                );
            }
            RequestState::Finished(Outcome::Success | Outcome::Noop, _, _, _, _) => {
                controls = controls.child(self.dismiss(id, cx));
            }
            RequestState::Finished(_, code, effects, recovery, message) => {
                entry = entry.child(
                    h_flex()
                        .gap_2()
                        .flex_wrap()
                        .child(text(request_id(id, "message"), message.clone()).text_sm())
                        .child(
                            text(request_id(id, "code"), code.as_str())
                                .text_xs()
                                .text_color(muted),
                        )
                        .child(
                            text(request_id(id, "effects"), effects_text(effects))
                                .text_xs()
                                .text_color(muted),
                        ),
                );
                for (n, action) in recovery.iter().enumerate() {
                    controls = controls.child(self.recovery(id, n, action, cx));
                }
                controls = controls.child(self.dismiss(id, cx));
            }
            RequestState::Removed | RequestState::NotSent(_) => {
                controls = controls.child(self.dismiss(id, cx));
            }
            RequestState::Dismissed => {}
        }
        entry.child(controls)
    }

    fn dismiss(&self, id: DesktopRequestId, cx: &mut Context<Self>) -> AnyElement {
        button(
            request_id(id, "dismiss"),
            DISMISS,
            cx.listener(move |this, _: &ClickEvent, _, cx| this.dismiss_request(id, cx)),
        )
    }

    /// A recovery action's control: it runs when D1 offers it, and is
    /// text, unavailable, when not.
    fn recovery(
        &self,
        id: DesktopRequestId,
        n: usize,
        action: &RecoveryAction,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let label = recovery_label(action.action);
        let runnable = recovery_control(action.action)
            != crate::desktop::model::operations::RecoveryControl::Unavailable
            && self
                .services
                .operations()
                .read(cx)
                .recovery_step(id, action)
                .is_some();
        if runnable {
            button(
                recovery_id(id, n),
                label,
                cx.listener(move |this, _: &ClickEvent, window, cx| {
                    this.run_recovery(id, n, window, cx)
                }),
            )
            .into_any_element()
        } else {
            unavailable(recovery_id(id, n), label, NOT_AVAILABLE_YET, cx).into_any_element()
        }
    }

    /// The "This session" section.
    pub(super) fn session_section(
        &self,
        cx: &mut Context<Self>,
    ) -> gpui_kit::Stateful<gpui_kit::Div> {
        let requests: Vec<_> = self
            .services
            .operations()
            .read(cx)
            .requests()
            .iter()
            .filter(|request| !matches!(request.state, RequestState::Dismissed))
            .map(|request| request.id)
            .collect();
        let mut section = v_flex()
            .id("session-section")
            .gap_1()
            .child(div().text_sm().child(THIS_SESSION));
        if requests.is_empty() {
            section = section.child(
                text("session-empty", NO_REQUESTS)
                    .text_xs()
                    .text_color(cx.theme().muted_foreground),
            );
        }
        for id in requests {
            let entry = self.services.operations().read(cx).request(id).cloned();
            if let Some(request) = entry {
                section = section.child(self.session_entry(&request, cx));
            }
        }
        section
    }

    /// The "Pending in repositories" section.
    pub(super) fn pending_section(
        &mut self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> gpui_kit::Stateful<gpui_kit::Div> {
        let muted = cx.theme().muted_foreground;
        let pending: Vec<(RepoKey, ListState<manyhands::repository::OperationListDto>)> = self
            .services
            .operations()
            .read(cx)
            .pending()
            .iter()
            .map(|(root, state)| (root.clone(), state.clone()))
            .collect();
        let mut section = v_flex()
            .id("pending-section")
            .gap_1()
            .child(div().text_sm().child(PENDING));
        let mut n = 0usize;
        let mut any = false;
        for (root, state) in pending {
            let name = scope_name(Some(Path::new(&root)));
            let rows = match &state {
                ListState::Ready { rows, .. } => rows.items.clone(),
                _ => {
                    any = true;
                    let reload = root.clone();
                    section = section.child(
                        h_flex()
                            .gap_2()
                            .child(text(("pending-repository", n), name.clone()).text_xs())
                            .child(
                                text(("pending-repository-state", n), state.label())
                                    .text_xs()
                                    .text_color(muted),
                            )
                            .children(
                                matches!(state, ListState::Busy | ListState::Failed(_)).then(
                                    || {
                                        button(
                                            ("pending-repository-retry", n),
                                            super::sidebar::RETRY,
                                            cx.listener(move |this, _: &ClickEvent, _, cx| {
                                                this.load_pending(reload.clone(), cx)
                                            }),
                                        )
                                    },
                                ),
                            ),
                    );
                    continue;
                }
            };
            for operation in rows {
                any = true;
                let index = n;
                n += 1;
                let operation_id = operation.operation_id.clone();
                let mut row = v_flex()
                    .id(("pending-row", index))
                    .test_support()
                    .gap_1()
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                text(("pending-command", index), operation.action.as_str())
                                    .text_sm(),
                            )
                            .child(
                                text(("pending-scope", index), name.clone())
                                    .text_xs()
                                    .text_color(muted),
                            )
                            .child(
                                text(("pending-state", index), operation.state.clone())
                                    .text_xs()
                                    .text_color(muted),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                text(
                                    ("pending-operation", index),
                                    operation_id
                                        .clone()
                                        .unwrap_or_else(|| NO_OPERATION_ID.to_owned()),
                                )
                                .text_xs(),
                            )
                            .child(
                                text(
                                    ("pending-next", index),
                                    operation
                                        .next_action
                                        .map(|next| next.as_str())
                                        .unwrap_or(NO_NEXT_ACTION),
                                )
                                .text_xs(),
                            ),
                    );
                let mut controls = h_flex().gap_2();
                match resume_offer(&operation) {
                    ResumeOffer::Offered => {
                        let (root, operation_id) = (root.clone(), operation_id.clone());
                        controls = controls.child(button(
                            ("pending-resume", index),
                            RESUME,
                            cx.listener(move |this, _: &ClickEvent, _, cx| {
                                if let Some(operation_id) = &operation_id {
                                    this.resume(root.clone(), operation_id, cx);
                                }
                            }),
                        ));
                    }
                    ResumeOffer::Unavailable(why) => {
                        controls = controls.child(
                            div().child(
                                Button::new(("pending-resume-button", index))
                                    .label(RESUME)
                                    .disabled(true),
                            ),
                        );
                        controls = controls.child(
                            text(("pending-resume-unavailable", index), why)
                                .text_xs()
                                .text_color(muted),
                        );
                    }
                    ResumeOffer::NotOffered => {}
                }
                if abandon_offered(&operation)
                    && let Some(operation_id) = operation_id.clone()
                {
                    let focus = self.abandon_focus_for(&root, &operation_id, cx);
                    let focused = focus.is_focused(window);
                    let target = (root.clone(), operation_id);
                    let origin = focus.clone();
                    controls = controls.child(
                        div()
                            .id(("pending-abandon", index))
                            .test_support()
                            .aria_label(ABANDON)
                            .track_focus(&focus)
                            .rounded_sm()
                            .when_focused(focused, cx)
                            .child(
                                Button::new(("pending-abandon-button", index))
                                    .danger()
                                    .label(ABANDON)
                                    .on_click(cx.listener(
                                        move |this, _: &ClickEvent, window, cx| {
                                            this.abandon(
                                                target.0.clone(),
                                                &target.1,
                                                origin.clone(),
                                                window,
                                                cx,
                                            )
                                        },
                                    )),
                            ),
                    );
                }
                row = row.child(controls);
                section = section.child(row);
            }
        }
        if !any {
            section = section.child(
                text("pending-empty", NOTHING_PENDING)
                    .text_xs()
                    .text_color(muted),
            );
        }
        section
    }
}

/// A focus style: a ring while focused, so focus is not shown by color
/// of the content alone.
trait FocusRing {
    fn when_focused(self, focused: bool, cx: &App) -> Self;
}

impl<E: Styled> FocusRing for E {
    fn when_focused(self, focused: bool, cx: &App) -> Self {
        if focused {
            self.border_2().border_color(cx.theme().ring)
        } else {
            self
        }
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "operations_tests.rs"]
mod tests;
