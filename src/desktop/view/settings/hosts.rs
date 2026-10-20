//! The Hosts settings page (design, Settings / Hosts): the host pins
//! `list_host_pins` lists, and Check host for the repository the page
//! works on, which then offers Approve or Replace for the key the host
//! presented.
//!
//! The page works on the repository selected in the main window when the
//! settings were last opened, or on the one a `host.approve` or
//! `host.replace` recovery names. Check host is offered only when that
//! repository's publication remote is reached over SSH, as
//! `list_remotes_redacted` says; otherwise the page says why in fixed text.
//!
//! **Check host** runs `observe_host` on the worker, under its session,
//! for the host the publication remote fetches from (a recovery names its
//! host instead). It connects, so it may ask for the selected key's
//! passphrase; it has no cancellation token, so while it runs the page
//! says "Connecting" and that Cancel is unavailable, in words, and Check
//! host does nothing more. It pins nothing. Focus goes to Check host when
//! it starts, so a prompt that comes while it runs gives focus back there.
//!
//! The answer shows the host, port, algorithm and fingerprint the host
//! presented. A host with no pin offers **Approve** (`host approve`); one
//! pinned to another key shows that pin and offers **Replace** (`host
//! replace`, with the pin as the old fingerprint); one that presented its
//! pin says so. Both open the confirmation dialog, whose preview the
//! library builds with no connection and which shows the old and new
//! fingerprints for a replacement. The approval writes a pin only after
//! the library connects again and the host presents that key.
//!
//! An answer acts only while it is the page's latest Check host and of
//! the repository the page works on: one of another repository is never
//! shown, and its controls ask nothing. The pins are read again when a
//! host request finishes, whoever started it, and when the page is shown
//! again. Every handler does nothing while a dialog is open. Text is fixed
//! strings, the library's own values, and the envelope's message with its
//! code.

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use gpui_kit::{
    ClickEvent, Context, FocusHandle, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, Subscription, Task, TestSupportExt, Window,
    component::{ActiveTheme, Disableable, WindowExt, button::Button, v_flex},
    div,
};
use manyhands::{
    repository::{HostObservationDto, HostPinDto, transport::SshAuthority},
    results::{Envelope, Outcome},
};

use super::{Read, wrapped};
use crate::desktop::{
    model::{
        list_state::LABEL_BUSY,
        stamp::{Generations, SettingsPage, ViewScope},
    },
    services::{
        ReadKind, Services,
        requests::{DesktopRequestId, Intent, RequestState},
    },
    view::{confirm::confirm, scroll::ContentScroll},
};

pub(crate) const PINS_HEADING: &str = "Host pins";
pub(crate) const CHECK_HEADING: &str = "Check host";
pub(crate) const READING_PINS: &str = "Reading the host pins";
pub(crate) const NO_PINS: &str = "No host is pinned yet.";
pub(crate) const REAPPROVAL: &str =
    "The host pin registry was lost: every host must be approved again.";
pub(crate) const NO_REPOSITORY: &str =
    "Choose a repository in the sidebar to check the host it publishes to.";
pub(crate) const READING_REMOTES: &str = "Reading the repository's remotes";
pub(crate) const NO_SSH_PUBLICATION: &str = "Check host is unavailable: this repository has no \
     publication remote reached over SSH. Choose one on the Remotes page.";
pub(crate) const CHECK: &str = "Check host";
pub(crate) const CHECK_NOTE: &str = "Check host connects to the host the publication remote \
     fetches from and shows the key it presents. It pins nothing.";
pub(crate) const CONNECTING: &str = "Connecting";
pub(crate) const CANCEL_UNAVAILABLE: &str = "Cancel is unavailable while connecting to a host.";
pub(crate) const NOT_PINNED: &str = "This host is not pinned.";
pub(crate) const MATCHES: &str = "The host presents the key it is pinned to.";
pub(crate) const DIFFERENT: &str = "The host presents another key than the one it is pinned to.";
pub(crate) const PINNED: &str = "Pinned key";
pub(crate) const APPROVE: &str = "Approve host";
pub(crate) const REPLACE: &str = "Replace host key";

const CHECK_ID: &str = "hosts-check";
const APPROVE_ID: &str = "hosts-approve";
const REPLACE_ID: &str = "hosts-replace";

/// A host as the library names it: `host:port`, with an address that holds
/// a colon in brackets.
pub(crate) fn authority_line(host: &str, port: u16) -> String {
    if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

/// A key as the confirmation dialog names it: its algorithm and its
/// fingerprint.
pub(crate) fn key_line(algorithm: &str, fingerprint: &str) -> String {
    format!("{algorithm} {fingerprint}")
}

/// The line that names the repository the page works on.
pub(crate) fn chosen_line(root: &str) -> String {
    format!("Repository: {root}")
}

fn named(name: &str, value: impl std::fmt::Display) -> String {
    format!("{name}: {value}")
}

/// What Approve or Replace asks the library to pin: the key a host
/// presented, for the repository whose remote names the host. `replaces`
/// is the fingerprint of the pin a replacement replaces.
pub(crate) struct Trust {
    pub root: PathBuf,
    pub authority: SshAuthority,
    pub algorithm: String,
    pub fingerprint: String,
    pub replaces: Option<String>,
}

impl Trust {
    fn intent(self) -> Intent {
        let Self {
            root,
            authority,
            algorithm,
            fingerprint,
            replaces,
        } = self;
        match replaces {
            None => Intent::HostApprove {
                root,
                authority,
                algorithm,
                fingerprint,
            },
            Some(old_fingerprint) => Intent::HostReplace {
                root,
                authority,
                algorithm,
                fingerprint,
                old_fingerprint,
            },
        }
    }
}

/// What the host presented, beside its pin.
enum Verdict<'a> {
    NotPinned,
    Matches,
    Different {
        algorithm: &'a str,
        fingerprint: &'a str,
    },
}

fn verdict(observed: &HostObservationDto) -> Verdict<'_> {
    match (&observed.pinned_algorithm, &observed.pinned_fingerprint) {
        (Some(algorithm), Some(fingerprint))
            if *algorithm == observed.presented_algorithm
                && *fingerprint == observed.presented_fingerprint =>
        {
            Verdict::Matches
        }
        (Some(algorithm), Some(fingerprint)) => Verdict::Different {
            algorithm,
            fingerprint,
        },
        // A fingerprint is what a replacement is given; a pin the library
        // reports only in part is approved again.
        _ => Verdict::NotPinned,
    }
}

/// The publication remote of a repository, as last read, with the root it
/// was read from.
struct Publication {
    root: PathBuf,
    /// Whether it is reached over SSH; `false` with no publication remote.
    ssh: bool,
}

/// The page's latest Check host.
struct Observing {
    id: DesktopRequestId,
    root: PathBuf,
    /// The library's answer, once the worker gave it.
    answer: Option<Envelope<HostObservationDto>>,
}

/// The requests whose finishing can change the pins.
fn host_command(intent: &Intent) -> bool {
    matches!(
        intent,
        Intent::HostApprove { .. } | Intent::HostReplace { .. }
    )
}

/// The requests whose finishing can change a repository's publication
/// remote.
fn remote_command(intent: &Intent) -> bool {
    matches!(
        intent,
        Intent::RemoteAdd { .. } | Intent::RemoteRemove { .. } | Intent::RemoteSelect { .. }
    )
}

const SCOPE: ViewScope = ViewScope::Settings(SettingsPage::Hosts);

pub(crate) struct HostsPage {
    services: Services,
    scroll: ContentScroll,
    /// The pins `list_host_pins` last gave, and whether the registry was
    /// lost.
    pins: Read<(Vec<HostPinDto>, bool)>,
    pin_generations: Generations,
    pin_read: Option<Task<()>>,
    /// The repository the page works on.
    root: Option<PathBuf>,
    publication: Read<Publication>,
    remote_generations: Generations,
    remote_read: Option<Task<()>>,
    observing: Option<Observing>,
    /// The focus each control's wrapper tracks, by its ID.
    focus: HashMap<&'static str, FocusHandle>,
    /// The finished requests already acted on, by attempt.
    seen: HashSet<(DesktopRequestId, u32)>,
    _operations: Subscription,
}

impl HostsPage {
    pub(crate) fn new(
        services: Services,
        scroll: ContentScroll,
        root: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus = [CHECK_ID, APPROVE_ID, REPLACE_ID]
            .into_iter()
            .map(|id| (id, cx.focus_handle()))
            .collect();
        let operations = cx.observe_in(services.operations(), window, |this, _, window, cx| {
            this.operations_changed(window, cx)
        });
        // What finished before the page was made is already in what it
        // reads now.
        let seen = services
            .operations()
            .read(cx)
            .requests()
            .iter()
            .filter(|request| matches!(request.state, RequestState::Finished(..)))
            .map(|request| (request.id, request.attempt))
            .collect();
        let mut page = Self {
            services,
            scroll,
            pins: Read::Loading,
            pin_generations: Generations::default(),
            pin_read: None,
            root: None,
            publication: Read::Loading,
            remote_generations: Generations::default(),
            remote_read: None,
            observing: None,
            focus,
            seen,
            _operations: operations,
        };
        page.load_pins(cx);
        page.choose(root, cx);
        page
    }

    /// Around Check host, whose own focus is the Kit's.
    #[cfg(test)]
    pub(crate) fn check_focus(&self) -> FocusHandle {
        self.focus[CHECK_ID].clone()
    }

    /// The focus the wrapper `id` tracks.
    #[cfg(test)]
    pub(crate) fn control_focus(&self, id: &str) -> FocusHandle {
        self.focus[id].clone()
    }

    /// The request of the page's latest Check host, answered or not.
    #[cfg(test)]
    pub(crate) fn observing_id(&self) -> Option<DesktopRequestId> {
        self.observing.as_ref().map(|observing| observing.id)
    }

    /// The code of the answer to the page's latest Check host, once it has
    /// one and it is of the repository the page works on.
    #[cfg(test)]
    pub(crate) fn observed_code(&self) -> Option<manyhands::results::ResultCode> {
        self.current()?.answer.as_ref().map(|answer| answer.code)
    }

    /// The page is shown again: it works on `selection` when there is one,
    /// and reads what it shows again.
    pub(crate) fn opened(&mut self, selection: Option<PathBuf>, cx: &mut Context<Self>) {
        self.load_pins(cx);
        if selection.is_some() && selection != self.root {
            self.choose(selection, cx);
        } else {
            self.load_publication(cx);
        }
    }

    /// Works on `root` from now on. Another repository's publication and
    /// Check host are never left on show.
    fn choose(&mut self, root: Option<PathBuf>, cx: &mut Context<Self>) {
        if root != self.root {
            self.publication = Read::Loading;
            self.remote_read = None;
            self.observing = None;
        }
        self.root = root;
        self.load_publication(cx);
    }

    #[allow(clippy::result_large_err)] // `ReadError` carries its scope by value.
    fn load_pins(&mut self, cx: &mut Context<Self>) {
        let stamp = self.pin_generations.next(SCOPE);
        if !matches!(self.pins, Read::Ready(_)) {
            self.pins = Read::Loading;
        }
        self.pin_read = Some(self.services.read(
            ReadKind::Hosts,
            stamp,
            |service| service.list_host_pins(),
            cx,
            |this: &mut Self, stamp, result, cx| {
                if !this.pin_generations.is_current(&stamp) {
                    return;
                }
                this.pins =
                    Read::from_result(result, |list| (list.items, list.reapproval_required));
                cx.notify();
            },
        ));
        cx.notify();
    }

    /// Reads which remote the repository publishes to, and whether it is
    /// reached over SSH.
    #[allow(clippy::result_large_err)] // `ReadError` carries its scope by value.
    fn load_publication(&mut self, cx: &mut Context<Self>) {
        let stamp = self.remote_generations.next(SCOPE);
        let Some(root) = self.root.clone() else {
            self.remote_read = None;
            self.publication = Read::Loading;
            cx.notify();
            return;
        };
        if !matches!(&self.publication, Read::Ready(publication) if publication.root == root) {
            self.publication = Read::Loading;
        }
        let read_root = root.clone();
        self.remote_read = Some(self.services.read(
            ReadKind::Remotes,
            stamp,
            move |service| {
                let repo = service.resolve_repository(&read_root)?;
                service.list_remotes_redacted(&repo)
            },
            cx,
            move |this: &mut Self, stamp, result, cx| {
                if !this.remote_generations.is_current(&stamp) {
                    return;
                }
                this.publication = Read::from_result(result, |list| Publication {
                    root,
                    ssh: list.items.iter().any(|remote| {
                        remote.selected_for_publication && remote.publication_eligible
                    }),
                });
                cx.notify();
            },
        ));
        cx.notify();
    }

    /// Whether the page's latest Check host still runs.
    fn connecting(&self) -> bool {
        self.observing
            .as_ref()
            .is_some_and(|observing| observing.answer.is_none())
    }

    /// The page's latest Check host, while it is of the repository the
    /// page works on.
    fn current(&self) -> Option<&Observing> {
        self.observing
            .as_ref()
            .filter(|observing| self.root.as_deref() == Some(observing.root.as_path()))
    }

    /// Whether Check host is offered: the publication remote of the
    /// repository worked on, as last read for it, is reached over SSH.
    fn check_offered(&self) -> bool {
        matches!(
            (&self.publication, &self.root),
            (Read::Ready(publication), Some(root)) if publication.root == *root && publication.ssh
        )
    }

    /// Check host: observes the host the publication remote fetches from.
    pub(crate) fn check(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_dialog(cx) || self.connecting() || !self.check_offered() {
            return;
        }
        let Some(root) = self.root.clone() else {
            return;
        };
        self.start(root, None, window, cx);
    }

    /// The `host.approve` / `host.replace` recovery: works on `root` and
    /// checks the host `authority`, whose answer offers Approve or Replace.
    pub(crate) fn check_host(
        &mut self,
        root: PathBuf,
        authority: SshAuthority,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if window.has_active_dialog(cx) {
            return;
        }
        self.choose(Some(root.clone()), cx);
        if self.connecting() {
            window.focus(&self.focus[CHECK_ID], cx);
            return;
        }
        self.start(root, Some(authority), window, cx);
    }

    /// Starts the observation on the worker, and puts focus on Check host:
    /// a prompt it raises gives focus back there.
    fn start(
        &mut self,
        root: PathBuf,
        authority: Option<SshAuthority>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let id = self.services.worker().observe_host(root.clone(), authority);
        self.observing = Some(Observing {
            id,
            root,
            answer: None,
        });
        window.focus(&self.focus[CHECK_ID], cx);
        cx.notify();
    }

    /// What Approve or Replace on the answer of Check host `id` asks: only
    /// while it is the page's latest, of the repository worked on, and an
    /// answer that offers it.
    fn offered(&self, id: DesktopRequestId) -> Option<Trust> {
        let observing = self.current().filter(|observing| observing.id == id)?;
        let answer = observing.answer.as_ref()?;
        if answer.outcome != Outcome::Success {
            return None;
        }
        let observed = answer.data.as_ref()?;
        let replaces = match verdict(observed) {
            Verdict::Matches => return None,
            Verdict::NotPinned => None,
            Verdict::Different { fingerprint, .. } => Some(fingerprint.to_owned()),
        };
        Some(Trust {
            root: observing.root.clone(),
            authority: SshAuthority {
                host: observed.host.clone(),
                port: observed.port,
            },
            algorithm: observed.presented_algorithm.clone(),
            fingerprint: observed.presented_fingerprint.clone(),
            replaces,
        })
    }

    /// Approve or Replace, on the answer of Check host `id`.
    fn submit_trust(&mut self, id: DesktopRequestId, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(trust) = self.offered(id) {
            self.confirm_trust(trust, window, cx);
        }
    }

    /// The confirmation dialog for `trust`. Focus goes back to what held
    /// it when the dialog closes, else to Check host.
    pub(crate) fn confirm_trust(
        &mut self,
        trust: Trust,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if window.has_active_dialog(cx) {
            return;
        }
        let intent = trust.intent();
        debug_assert!(intent.confirmed());
        // Closing gives focus back to the control the dialog was opened
        // from. A successful request lets go of the answer that drew
        // Approve or Replace; `operations_changed` then moves focus left
        // on either to Check host.
        let origin = window
            .focused(cx)
            .unwrap_or_else(|| self.focus[CHECK_ID].clone());
        confirm(intent, origin, &self.services, window, cx);
    }

    /// The worker answered, or a request finished: the page's Check host
    /// takes its answer; a host request reads the pins again, and one that
    /// pinned the host the answer shows lets that answer go, since what it
    /// offered is done; a remote request of the repository worked on reads
    /// its publication again.
    fn operations_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let operations = self.services.operations().clone();
        let operations = operations.read(cx);
        if let Some(observing) = &mut self.observing
            && observing.answer.is_none()
            && let Some(answer) = operations.observation(observing.id)
        {
            observing.answer = Some(answer.clone());
        }
        let (mut pins, mut publication, mut done) = (false, false, false);
        for request in operations.requests() {
            let RequestState::Finished(outcome, ..) = &request.state else {
                continue;
            };
            let host = host_command(&request.intent);
            let ours = remote_command(&request.intent)
                && self.root.is_some()
                && request.intent.root() == self.root.as_deref();
            if !(host || ours) || !self.seen.insert((request.id, request.attempt)) {
                continue;
            }
            pins |= host;
            publication |= ours;
            done |= host && *outcome == Outcome::Success && self.answered_for(&request.intent);
        }
        if done {
            self.observing = None;
            // The dialog that submitted the request closes onto its origin
            // in this same batch; once it has, focus left on Approve or
            // Replace, no longer drawn, goes to Check host.
            cx.defer_in(window, |this, window, cx| this.refocus_gone(window, cx));
        }
        if pins {
            self.load_pins(cx);
        }
        if publication {
            self.load_publication(cx);
        }
        cx.notify();
    }

    /// Focus on Approve or Replace, or on nothing, once the answer that
    /// drew them is gone: Check host takes it, the region's first control
    /// left in its place.
    fn refocus_gone(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.observing.is_some() || window.has_active_dialog(cx) {
            return;
        }
        let gone = window.focused(cx).is_none()
            || [APPROVE_ID, REPLACE_ID]
                .iter()
                .any(|id| self.focus[id].is_focused(window));
        if gone {
            window.focus(&self.focus[CHECK_ID], cx);
        }
    }

    /// Whether `intent` pinned the host the page's answer shows, for the
    /// repository it was observed for.
    fn answered_for(&self, intent: &Intent) -> bool {
        let Some(observing) = &self.observing else {
            return false;
        };
        let Some(observed) = observing
            .answer
            .as_ref()
            .and_then(|answer| answer.data.as_ref())
        else {
            return false;
        };
        let (Intent::HostApprove {
            root, authority, ..
        }
        | Intent::HostReplace {
            root, authority, ..
        }) = intent
        else {
            return false;
        };
        *root == observing.root
            && authority.host == observed.host
            && authority.port == observed.port
    }
}

/// A piece of text a test reads by `id`.
fn text(id: impl Into<gpui_kit::ElementId>, value: impl Into<String>) -> impl IntoElement {
    let value = value.into();
    div()
        .id(id)
        .test_support()
        .aria_label(value.clone())
        .text_sm()
        .child(value)
}

fn heading(id: &'static str, label: &'static str) -> impl IntoElement {
    div()
        .id(id)
        .test_support()
        .aria_label(label)
        .text_base()
        .font_weight(gpui_kit::FontWeight::BOLD)
        .child(label)
}

impl HostsPage {
    /// A read's state line, when it gives no rows.
    fn state_line<T>(
        id: &'static str,
        read: &Read<T>,
        loading: &'static str,
        cx: &Context<Self>,
    ) -> Option<gpui_kit::Div> {
        let muted = cx.theme().muted_foreground;
        let body = v_flex().gap_0p5();
        match read {
            Read::Loading => Some(body.child(text(id, loading))),
            Read::Busy => Some(body.child(text(id, LABEL_BUSY))),
            Read::Failed(code) => Some(
                body.child(text(id, code.message()))
                    .child(div().text_xs().text_color(muted).child(code.as_str())),
            ),
            Read::Ready(_) => None,
        }
    }

    /// The pins, each by its host and port, algorithm and fingerprint.
    fn pin_rows(&self, cx: &Context<Self>) -> gpui_kit::Div {
        let body = v_flex()
            .gap_2()
            .child(heading("hosts-pins-heading", PINS_HEADING));
        if let Some(state) = Self::state_line("hosts-pins-state", &self.pins, READING_PINS, cx) {
            return body.child(state);
        }
        let Read::Ready((pins, reapproval)) = &self.pins else {
            return body;
        };
        let mut body = body.children(reapproval.then(|| text("hosts-reapproval", REAPPROVAL)));
        if pins.is_empty() {
            return body.child(text("hosts-pins-state", NO_PINS));
        }
        for (ix, pin) in pins.iter().enumerate() {
            body = body.child(
                v_flex()
                    .gap_0p5()
                    .child(
                        div()
                            .font_weight(gpui_kit::FontWeight::BOLD)
                            .child(text(("host-pin", ix), authority_line(&pin.host, pin.port))),
                    )
                    .child(text(
                        ("host-pin-key", ix),
                        key_line(&pin.pinned_algorithm, &pin.pinned_fingerprint),
                    )),
            );
        }
        body
    }

    /// A button in its wrapper `id`.
    fn button(
        &self,
        id: &'static str,
        label: &'static str,
        disabled: bool,
        on_click: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let button = Button::new(gpui_kit::ElementId::Name(format!("{id}-button").into()))
            .label(label)
            .disabled(disabled)
            .on_click(
                cx.listener(move |this, _: &ClickEvent, window, cx| on_click(this, window, cx)),
            );
        wrapped(id, label, &self.focus[id], &self.scroll, button)
    }

    /// Check host for the repository worked on, and what it answered.
    fn check_section(&self, cx: &mut Context<Self>) -> gpui_kit::Div {
        let muted = cx.theme().muted_foreground;
        let body = v_flex()
            .gap_2()
            .child(heading("hosts-check-heading", CHECK_HEADING));
        let Some(root) = self.root.clone() else {
            return body.child(text("hosts-check-state", NO_REPOSITORY));
        };
        let mut body = body.child(text("hosts-root", chosen_line(&root.to_string_lossy())));
        let offered = self.check_offered();
        if let Some(state) =
            Self::state_line("hosts-check-state", &self.publication, READING_REMOTES, cx)
        {
            body = body.child(state);
        } else if !offered {
            // A read of another repository is never taken for this one's.
            let line = match &self.publication {
                Read::Ready(publication) if publication.root != root => READING_REMOTES,
                _ => NO_SSH_PUBLICATION,
            };
            body = body.child(text("hosts-check-state", line));
        }
        // Drawn, unavailable, whenever it cannot run, so focus put on it
        // (a recovery's check, a prompt closing) has a place to stay.
        let unavailable = !offered || self.connecting();
        body.child(self.button(
            CHECK_ID,
            CHECK,
            unavailable,
            |this, window, cx| this.check(window, cx),
            cx,
        ))
        .child(div().text_xs().text_color(muted).child(CHECK_NOTE))
        .children(self.observation(cx))
    }

    /// The answer to the page's latest Check host, or that it runs.
    fn observation(&self, cx: &mut Context<Self>) -> Option<gpui_kit::Div> {
        let muted = cx.theme().muted_foreground;
        let observing = self.current()?;
        let Some(answer) = &observing.answer else {
            // Said in words: the request connects and cannot be stopped.
            return Some(
                v_flex()
                    .gap_0p5()
                    .child(text("hosts-observation-state", CONNECTING))
                    .child(text("hosts-cancel-unavailable", CANCEL_UNAVAILABLE)),
            );
        };
        let body = v_flex().gap_0p5();
        let observed = match (&answer.outcome, &answer.data) {
            (Outcome::Success, Some(observed)) => observed,
            _ => {
                return Some(
                    body.child(text("hosts-observation-message", answer.message.clone()))
                        .child(
                            div()
                                .text_xs()
                                .text_color(muted)
                                .child(text("hosts-observation-code", answer.code.as_str())),
                        ),
                );
            }
        };
        let id = observing.id;
        let mut body = body
            .child(text("hosts-observed-host", named("Host", &observed.host)))
            .child(text("hosts-observed-port", named("Port", observed.port)))
            .child(text(
                "hosts-observed-algorithm",
                named("Algorithm", &observed.presented_algorithm),
            ))
            .child(text(
                "hosts-observed-fingerprint",
                named("Fingerprint", &observed.presented_fingerprint),
            ))
            .children(
                observed
                    .reapproval_required
                    .then(|| text("hosts-observed-reapproval", REAPPROVAL)),
            );
        body = match verdict(observed) {
            Verdict::Matches => body.child(text("hosts-observed-verdict", MATCHES)),
            Verdict::NotPinned => body
                .child(text("hosts-observed-verdict", NOT_PINNED))
                .child(self.button(
                    APPROVE_ID,
                    APPROVE,
                    false,
                    move |this, window, cx| this.submit_trust(id, window, cx),
                    cx,
                )),
            Verdict::Different {
                algorithm,
                fingerprint,
            } => body
                .child(text("hosts-observed-verdict", DIFFERENT))
                .child(text(
                    "hosts-observed-pinned",
                    named(PINNED, key_line(algorithm, fingerprint)),
                ))
                .child(self.button(
                    REPLACE_ID,
                    REPLACE,
                    false,
                    move |this, window, cx| this.submit_trust(id, window, cx),
                    cx,
                )),
        };
        Some(body)
    }
}

impl Render for HostsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let pins = self.pin_rows(cx);
        let check = self.check_section(cx);
        v_flex()
            .id("settings-hosts")
            .test_support()
            .gap_4()
            .child(pins)
            .child(check)
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "hosts_tests.rs"]
mod tests;
