//! The operation area: this session's requests and each repository's
//! pending operations.
//!
//! **This session** lists every desktop request, newest first, with its
//! command, scope and state, and while it runs its latest stage. A
//! success or no-op is one line. A partial, blocked, failed or cancelled
//! request stays expanded, with its message, code, effects and recovery
//! actions, until it is dismissed. A queued request, or a running one the
//! library has not accepted, offers Cancel; an accepted one reads
//! "Finishing", except that an accepted command that stops at safe points,
//! in a repository with a publication remote, offers Stop at a safe point
//! (R-1, R-15), which reads Stopping… once pressed; a resume stops at safe
//! points only when it takes up a remote synchronization listed pending,
//! so any other resume reads Finishing. A running share or post reads its
//! stage in words ("Sharing · …", "Posting · …"), and a finished share
//! says its outcome in fixed words and offers the controls that outcome
//! names in place of the envelope's recovery actions (`outcomes`): a
//! stopped one says whether part of its work finished and offers Share, a
//! new request. Stop trying to share gives a request up (decision 8); one
//! whose record had already finished says "Already finished" above the
//! stored answer it replayed, and the answer of a Give up never offers it
//! again (P14). A draft store that did not open keeps an entry with its
//! fixed reason at the top, with no Dismiss; so does each session whose
//! recovery copy could not be written, named by its item's title as read,
//! with Retry, until a write lands. The launch's "Recovered drafts" entry
//! (`recovered`) follows the store's, with Review and Dismiss.
//!
//! **Pending in repositories** lists the operations each listed
//! repository reports with work outstanding: command, scope, operation ID
//! and what it waits for. A local operation, a remote item or repository
//! synchronization, or a local-context synchronization (D5 B6, E-5),
//! waiting to be resumed offers Resume (decision 11), and so does a
//! promotion's or closure's parent (D5 decision 10, `resume`), whose
//! resume opens the lifecycle dialog. The library lists a
//! comment's publication with nothing to resume, and its `submit_comment`
//! row only while its local work waits, so no publication is offered
//! Resume. A pending local operation offers Abandon, which is confirmed.
//!
//! An entry whose answer stopped on a conflict, and a pending row whose
//! conflict waits to be resolved, offer Review changes, which opens the
//! resolve conflict sheet on it (D5 B5, `review`).
//!
//! Every recovery action an envelope offers is a control: the ones D1
//! offers run, the rest are text, unavailable. So is the Retry of a context
//! request once a later one of its item was sent (R-78), which says why.
//! Text is fixed, the envelope's message or a code's own.

use std::path::Path;

use gpui_kit::{
    AnyElement, App, ClickEvent, Context, ElementId, FocusHandle, InteractiveElement, IntoElement,
    ParentElement, StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{
        ActiveTheme, Disableable, WindowExt,
        button::{Button, ButtonVariants},
        h_flex, v_flex,
    },
    div,
};
use manyhands::{
    drafts::DraftId,
    results::{Effects, Outcome, RecoveryAction},
};

mod outcomes;
mod recovered;
pub(crate) mod republish;
mod resume;
mod review;

use outcomes::{ALREADY_FINISHED, already_finished};
pub(crate) use outcomes::{share_result, share_words};

use super::{lifecycle, workspace::Workspace};
use crate::desktop::{
    model::{
        list_state::ListState,
        operations::{
            NOT_AVAILABLE_YET, NOT_SENT_AGAIN, Progressing, ResumeOffer, StopOffer,
            abandon_offered, draft_flush_failed, drafts_unavailable, progress_words, resume_offer,
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
/// An accepted share's Stop (R-1).
pub(crate) const STOP: &str = "Stop at a safe point";
/// Stop, once pressed.
pub(crate) const STOPPING: &str = "Stopping…";
pub(crate) const DISMISS: &str = "Dismiss";
pub(crate) const DISMISSED: &str = "Dismissed";
pub(crate) const RESUME: &str = "Resume";
pub(crate) const ABANDON: &str = "Abandon";
pub(crate) const APPLICATION: &str = "Application";
pub(crate) const NO_NEXT_ACTION: &str = "Nothing to do";
pub(crate) const NO_OPERATION_ID: &str = "No operation ID";
/// A lifecycle's entry, once failed or stopped: its dialog again (D5, A3).
pub(crate) const OPEN_LIFECYCLE: &str = "Open\u{2026}";

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
pub(crate) fn scope_name(root: Option<&Path>) -> String {
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

/// The element ids of a failing session's entry.
pub(crate) fn draft_flush_id(id: DraftId, part: &str) -> String {
    format!("draft-flush-{id}-{part}")
}

type Text = gpui_kit::base::ObservedElement<gpui_kit::Stateful<gpui_kit::Div>>;

fn text(id: impl Into<ElementId>, value: impl Into<String>) -> Text {
    let value = value.into();
    div()
        .id(id)
        .test_support()
        .aria_label(value.clone())
        .child(value)
}

/// A Kit button in a wrapper a test reads by `id`. With `focus`, the
/// wrapper tracks it, so a dialog the button opens can give focus back
/// there, and a test can see the button hold it. Enter or Space on the
/// wrapper, where that focus lands, presses the button too (A21), and the
/// wrapper shows a ring while it holds focus, as Abandon's does; a
/// pointer's click is the button's.
fn button(
    id: impl Into<ElementId>,
    label: &'static str,
    focus: Option<&FocusHandle>,
    cx: &App,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let id = id.into();
    let on_click = std::rc::Rc::new(on_click);
    let mut wrapper = div().id(id.clone()).test_support().aria_label(label);
    if let Some(focus) = focus {
        let pressed = on_click.clone();
        let ring = cx.theme().ring;
        wrapper = wrapper
            .track_focus(focus)
            .rounded_sm()
            .focus(move |style| style.border_2().border_color(ring))
            .on_click(move |event, window, cx| {
                if event.is_keyboard() {
                    pressed(event, window, cx);
                }
            });
    }
    wrapper
        .child(
            Button::new(ElementId::NamedChild(
                std::sync::Arc::new(id),
                "button".into(),
            ))
            .label(label)
            .on_click(move |event, window, cx| on_click(event, window, cx)),
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
    /// The publication remote navigation lists for the repository of
    /// `request`, if it lists one.
    fn publication_remote_of(&self, request: &DesktopRequest) -> Option<String> {
        let root = request.intent.root()?.to_str()?;
        self.navigation()
            .entry(root)?
            .summary
            .configuration
            .publication_remote
            .clone()
    }

    /// The primary branch navigation lists for the repository of
    /// `request`, if it lists one (R-35).
    fn primary_of(&self, request: &DesktopRequest) -> Option<String> {
        let root = request.intent.root()?.to_str()?;
        self.navigation()
            .entry(root)?
            .summary
            .configuration
            .primary_branch
            .clone()
    }

    /// A request's state in words: a running share or post by its stage,
    /// naming the publication remote `remote`, and a running lifecycle by
    /// its latest stage (D5, A3), naming the primary branch `primary`; any
    /// other as `state_label`.
    fn entry_state(
        request: &DesktopRequest,
        remote: Option<&str>,
        primary: Option<&str>,
    ) -> String {
        match (&request.state, Progressing::of(&request.intent)) {
            (RequestState::Running { stage, .. }, Some(kind)) => {
                progress_words(kind, *stage, remote)
            }
            (RequestState::Running { stage, .. }, None) => {
                lifecycle::progress_words(&request.intent, *stage, primary)
                    .unwrap_or_else(|| state_label(&request.state))
            }
            (state, _) => state_label(state),
        }
    }

    /// Open… on a lifecycle's entry: its dialog again, on that request
    /// (D5, A3). Focus goes back to Open… when it closes.
    fn open_lifecycle(
        &mut self,
        id: DesktopRequestId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if window.has_active_dialog(cx) {
            return;
        }
        let Some(request) = self.services().operations().read(cx).request(id).cloned() else {
            return;
        };
        // What navigation lists of its repository names its steps.
        let known = request
            .intent
            .root()
            .and_then(|root| root.to_str())
            .map(|root| self.known(root))
            .unwrap_or_default();
        let origin = self.control_focus_for(request_id(id, "open"), cx);
        let services = self.services().clone();
        lifecycle::reopen(&request, known, origin, &services, window, cx);
    }

    /// One request of this session.
    fn session_entry(
        &mut self,
        request: &DesktopRequest,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let id = request.id;
        let muted = cx.theme().muted_foreground;
        let remote = self.publication_remote_of(request);
        let primary = self.primary_of(request);
        // A lifecycle names its item by its short ID: never its title.
        let item = lifecycle::item_of(&request.intent);
        let line = h_flex()
            .gap_2()
            .child(
                text(
                    request_id(id, "command"),
                    request.intent.label_in(primary.as_deref()),
                )
                .text_sm(),
            )
            .children(item.map(|item| {
                text(request_id(id, "item"), item)
                    .text_xs()
                    .text_color(muted)
            }))
            .child(
                text(request_id(id, "scope"), scope_name(request.intent.root()))
                    .text_xs()
                    .text_color(muted),
            )
            .child(
                text(
                    request_id(id, "state"),
                    Self::entry_state(request, remote.as_deref(), primary.as_deref()),
                )
                .text_xs(),
            );
        // Show it, on a share answered `busy`, gives the entry of the
        // request that may hold the repository the focus.
        let shown = self.control_focus_for(request_id(id, "entry"), cx);
        let mut entry = v_flex()
            .id(request_id(id, "entry"))
            .test_support()
            .track_focus(&shown)
            .gap_1()
            .child(line);
        // A Give up that found the record already finished says so above
        // the stored answer it replayed.
        if matches!(request.state, RequestState::Finished(..))
            && already_finished(self.services(), id)
        {
            entry = entry.child(text(request_id(id, "gave-up"), ALREADY_FINISHED).text_sm());
        }
        let mut controls = h_flex().gap_2().flex_wrap();
        match &request.state {
            RequestState::Queued
            | RequestState::Running {
                accepted: false, ..
            } => {
                let focus = self.control_focus_for(request_id(id, "cancel"), cx);
                controls = controls.child(button(
                    request_id(id, "cancel"),
                    CANCEL,
                    Some(&focus),
                    cx,
                    cx.listener(move |this, _: &ClickEvent, _, cx| this.cancel_request(id, cx)),
                ));
            }
            RequestState::Running { accepted: true, .. } => {
                let offer = self
                    .services()
                    .operations()
                    .read(cx)
                    .stop_offer(id, remote.is_some());
                controls = controls.child(self.accepted_control(id, offer, cx));
            }
            RequestState::Finished(Outcome::Success | Outcome::Noop, _, _, _, _) => {
                // A share says its outcome on its one line.
                if let Some(result) = self.finished_share(id, cx)
                    && let Some(words) =
                        share_words(self.services(), id, &result, primary.as_deref(), cx)
                {
                    entry = entry.child(text(request_id(id, "outcome"), words).text_sm());
                }
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
                // A lifecycle's dialog shows its outcome and way out.
                if lifecycle::item_of(&request.intent).is_some() {
                    let focus = self.control_focus_for(request_id(id, "open"), cx);
                    controls = controls.child(button(
                        request_id(id, "open"),
                        OPEN_LIFECYCLE,
                        Some(&focus),
                        cx,
                        cx.listener(move |this, _: &ClickEvent, window, cx| {
                            this.open_lifecycle(id, window, cx)
                        }),
                    ));
                }
                // A conflict it stopped on: its sheet (D5 B5).
                controls = controls.children(self.review_control(id, cx));
                // A share's outcome offers its own controls in place of
                // the envelope's recovery actions.
                if let Some(result) = self.finished_share(id, cx) {
                    if let Some(words) =
                        share_words(self.services(), id, &result, primary.as_deref(), cx)
                    {
                        entry = entry.child(text(request_id(id, "outcome"), words).text_sm());
                    }
                    for offer in outcomes::shown_offers(self.services(), id, &result, cx) {
                        controls = controls.child(self.share_offer(id, offer, cx));
                    }
                } else {
                    for (n, action) in recovery.iter().enumerate() {
                        // Decision 26: no re-send of an unsaved post here.
                        if self
                            .services()
                            .operations()
                            .read(cx)
                            .hides_recovery(id, action)
                        {
                            continue;
                        }
                        controls = controls.child(self.recovery(id, n, action, cx));
                    }
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

    /// What an accepted request shows: Stop at a safe point, Stopping…
    /// once pressed, or Finishing, disabled, when it cannot be stopped.
    fn accepted_control(
        &mut self,
        id: DesktopRequestId,
        offer: StopOffer,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match offer {
            StopOffer::Offered => {
                let focus = self.control_focus_for(request_id(id, "stop"), cx);
                button(
                    request_id(id, "stop"),
                    STOP,
                    Some(&focus),
                    cx,
                    cx.listener(move |this, _: &ClickEvent, _, cx| this.stop_request(id, cx)),
                )
            }
            StopOffer::Stopping => div()
                .id(request_id(id, "stop"))
                .test_support()
                .aria_label(STOPPING)
                .child(
                    Button::new(ElementId::Name(request_id(id, "stopping").into()))
                        .label(STOPPING)
                        .disabled(true),
                )
                .into_any_element(),
            StopOffer::NotOffered => div()
                .child(
                    Button::new(ElementId::Name(request_id(id, "finishing").into()))
                        .label(FINISHING)
                        .disabled(true),
                )
                .into_any_element(),
        }
    }

    fn dismiss(&mut self, id: DesktopRequestId, cx: &mut Context<Self>) -> AnyElement {
        let focus = self.control_focus_for(request_id(id, "dismiss"), cx);
        button(
            request_id(id, "dismiss"),
            DISMISS,
            Some(&focus),
            cx,
            cx.listener(move |this, _: &ClickEvent, _, cx| this.dismiss_request(id, cx)),
        )
    }

    /// A recovery action's control: it runs when it names something this
    /// session can act on, and is text, unavailable, when not.
    fn recovery(
        &mut self,
        id: DesktopRequestId,
        n: usize,
        action: &RecoveryAction,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let operations = self.services().operations().read(cx);
        let label = operations.recovery_words(id, action);
        let runnable = operations.recovery_step(id, action).is_some();
        let why = if operations.retry_not_sent_again(id, action) {
            NOT_SENT_AGAIN
        } else {
            NOT_AVAILABLE_YET
        };
        if runnable {
            let focus = self.control_focus_for(recovery_id(id, n), cx);
            button(
                recovery_id(id, n),
                label,
                Some(&focus),
                cx,
                cx.listener(move |this, _: &ClickEvent, window, cx| {
                    this.run_recovery(id, n, window, cx)
                }),
            )
            .into_any_element()
        } else {
            unavailable(recovery_id(id, n), label, why, cx).into_any_element()
        }
    }

    /// A failing draft's Retry. The drafts-not-saved dialog's Cancel gives
    /// the focus to its wrapper, which then draws a ring and is pressed by
    /// Enter or Space; Tab reaches the Kit button inside, which draws its
    /// own. A pointer's click is the button's only.
    fn draft_flush_retry(
        &self,
        id: DraftId,
        focus: &FocusHandle,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let retry = draft_flush_id(id, "retry");
        let ring = cx.theme().ring;
        div()
            .id(ElementId::from(retry.clone()))
            .test_support()
            .aria_label(super::sidebar::RETRY)
            .relative()
            .rounded_sm()
            .track_focus(focus)
            .on_click(cx.listener(move |this, event: &ClickEvent, _, cx| {
                if event.is_keyboard() {
                    this.retry_draft_flush(id, cx)
                }
            }))
            .child(
                Button::new(ElementId::NamedChild(
                    std::sync::Arc::new(ElementId::from(retry.clone())),
                    "button".into(),
                ))
                .label(super::sidebar::RETRY)
                .on_click(
                    cx.listener(move |this, _: &ClickEvent, _, cx| this.retry_draft_flush(id, cx)),
                ),
            )
            .children(focus.is_focused(window).then(|| {
                div()
                    .id(format!("{retry}-focus-ring"))
                    .test_support()
                    .absolute()
                    .inset_0()
                    .rounded_sm()
                    .border_2()
                    .border_color(ring)
            }))
            .into_any_element()
    }

    /// The "This session" section.
    pub(super) fn session_section(
        &mut self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> gpui_kit::Stateful<gpui_kit::Div> {
        let requests: Vec<_> = self
            .services()
            .operations()
            .read(cx)
            .requests()
            .iter()
            .filter(|request| !matches!(request.state, RequestState::Dismissed))
            .map(|request| request.id)
            .collect();
        let unavailable = self.services().drafts().read(cx).unavailable();
        let recovered = self.recovered_entry(cx);
        let mut section = v_flex()
            .id("session-section")
            .gap_1()
            .child(div().text_sm().child(THIS_SESSION))
            // Persistent: the store stays closed for the session, so the
            // entry offers no Dismiss.
            .children(
                unavailable.map(|reason| text("drafts-unavailable", drafts_unavailable(reason))),
            )
            .children(recovered);
        let failures = self.services().drafts().read(cx).flush_failures();
        for (id, title, kind) in failures {
            // Until a later flush succeeds: no Dismiss.
            let retry = draft_flush_id(id, "retry");
            let focus = self.control_focus_for(retry.clone(), cx);
            section = section.child(
                h_flex()
                    .gap_2()
                    .child(text(
                        draft_flush_id(id, "entry"),
                        draft_flush_failed(&title, kind),
                    ))
                    .child(self.draft_flush_retry(id, &focus, window, cx)),
            );
        }
        if requests.is_empty() {
            section = section.child(
                text("session-empty", NO_REQUESTS)
                    .text_xs()
                    .text_color(cx.theme().muted_foreground),
            );
        }
        for id in requests {
            let entry = self.services().operations().read(cx).request(id).cloned();
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
            .services()
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
                                            None,
                                            cx,
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
                        let focus = self.control_focus_for(
                            format!(
                                "resume:{root}:{}",
                                operation_id.as_deref().unwrap_or_default()
                            ),
                            cx,
                        );
                        let (root, operation_id) = (root.clone(), operation_id.clone());
                        controls = controls.child(button(
                            ("pending-resume", index),
                            RESUME,
                            Some(&focus),
                            cx,
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
                // A promotion's or closure's parent (decision 10).
                controls = controls.children(self.lifecycle_resume(&root, &operation, index, cx));
                controls = controls.children(self.pending_review(
                    &root,
                    operation_id.as_deref(),
                    index,
                    cx,
                ));
                if abandon_offered(&operation)
                    && let Some(operation_id) = operation_id.clone()
                {
                    let focus = self.abandon_focus_for(&root, &operation_id, cx);
                    let focused = focus.is_focused(window);
                    let target = (root.clone(), operation_id);
                    let origin = focus.clone();
                    let (pressed, pressed_origin) = (target.clone(), origin.clone());
                    controls = controls.child(
                        div()
                            .id(("pending-abandon", index))
                            .test_support()
                            .aria_label(ABANDON)
                            .track_focus(&focus)
                            .rounded_sm()
                            .when_focused(focused, cx)
                            // The focus the confirmation gives back on Escape is the
                            // wrapper's: Enter or Space presses Abandon there too. A
                            // pointer's click is the button's.
                            .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                                if event.is_keyboard() {
                                    this.abandon(
                                        pressed.0.clone(),
                                        &pressed.1,
                                        pressed_origin.clone(),
                                        window,
                                        cx,
                                    )
                                }
                            }))
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
