//! The Checking for updates settings page (D6 design C1, C2, C4): one row
//! per repository with its status line, the interval, Check now (Retry now
//! after a failure) and a switch, as the model says (`model::polling::
//! status`).
//!
//! - **Check now** sends `poll once` on the user lane. A user job cancels a
//!   running automatic poll and runs first (B2), so the button is disabled
//!   only while this process's own Check now of the repository is queued or
//!   running (C1). A prompt for the key's passphrase comes from the person's
//!   own press, through the usual modal.
//! - **The interval** offers Every 1, 5, 10, 15, 30 and 60 minutes, all
//!   inside the library's 60 to 3600 s, and sends `poll configure`.
//! - **The switch** is on while unpaused: off sends `poll pause`, on sends
//!   `poll resume`. No verb sets `enabled`, so a repository whose
//!   `enabled` is off says "Checking is off" and has no control (K-5).
//! - A repository with no SSH publication remote says "No shared copy,
//!   nothing to check" and offers Set up sharing….
//!
//! The page never parses a verb's answer: the scheduler reads the policy
//! again when one finishes and the page follows the schedule. What the
//! scheduler does not hold yet is told apart from what it holds nothing for
//! (`Scheduler::unplaced`).
//!
//! A needs-attention row carries the action its reason names (Unlock…,
//! Choose a key…, Approve host…, Open remote settings…; Resume checking for
//! a repository held until recovery), as the operation area's entry does
//! (`attention`).
//!
//! Every control sits in a focus-tracking wrapper with a scroll marker, so
//! a control that takes focus below the fold is brought into view. Every
//! handler does nothing while a dialog is open. Text is fixed strings and
//! the repository's name; no item, comment or key is named.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use gpui_kit::{
    AnyElement, AppContext, ClickEvent, Context, Entity, EventEmitter, FocusHandle,
    InteractiveElement, IntoElement, ParentElement, Render, StatefulInteractiveElement, Styled,
    Subscription, Task, TestSupportExt, Window,
    component::{
        ActiveTheme, Disableable, IndexPath, WindowExt,
        button::Button,
        h_flex,
        select::{SearchableVec, Select, SelectEvent, SelectState},
        switch::Switch,
        v_flex,
    },
    div, px,
};

use super::{Wrapped, wrapped};
use crate::desktop::{
    model::{
        list_state::LABEL_BUSY,
        polling::status::{self, Controls, Facts, INTERVALS, Row, Shape, interval_words},
        read_state::Read,
        stamp::{Generations, RepoKey, SettingsPage, ViewScope},
    },
    services::{
        ReadKind, Services,
        requests::{Intent, Lane},
    },
    view::scroll::ContentScroll,
};

pub(crate) const POLLING_HEADING: &str = "Checking for updates";
pub(crate) const INTRO: &str = "Manyhands checks your shared copies for other people's changes and brings them in when it's safe. It never shares anything. It also removes this computer's copies of tickets closed and documents promoted elsewhere, unless you have unsaved or kept work in them.";
pub(crate) const READING_REPOSITORIES: &str = "Reading the registered repositories";
pub(crate) const NO_REPOSITORIES: &str = "No repository is registered yet.";
pub(crate) const CHECK_NOW: &str = "Check now";
pub(crate) const RETRY_NOW: &str = "Retry now";
pub(crate) const SET_UP_SHARING: &str = "Set up sharing\u{2026}";
pub(crate) const SHOW_IN_ACTIVITY: &str = "Show in Activity";
pub(crate) const AUTOMATIC: &str = "Check automatically";
pub(crate) const INTERVAL: &str = "How often";

const SCOPE: ViewScope = ViewScope::Settings(SettingsPage::Polling);

type Interval = SelectState<SearchableVec<&'static str>>;

/// What the page asks of the settings host.
pub(crate) enum PollingEvent {
    /// Set up sharing…: the Remotes page on this repository.
    SetUpSharing(PathBuf),
    /// Choose a key…, Approve host… or Open remote settings…: the page that
    /// fixes the cause (C3).
    Attend(Attend),
    /// The status points at a pending operation: show Activity.
    ShowActivity,
}

impl EventEmitter<PollingEvent> for PollingPage {}

#[path = "polling_attention.rs"]
mod attention;
pub(crate) use attention::Attend;

pub(crate) struct PollingPage {
    services: Services,
    scroll: ContentScroll,
    repositories: Read<Vec<RepoKey>>,
    generations: Generations,
    read: Option<Task<()>>,
    /// The focus each control's wrapper tracks, by its ID.
    focus: HashMap<(&'static str, usize), FocusHandle>,
    /// Each repository's interval select, made when its row first has one.
    intervals: HashMap<RepoKey, Entity<Interval>>,
    _picks: HashMap<RepoKey, Subscription>,
    _operations: Subscription,
    _scheduler: Subscription,
}

fn name_of(root: &str) -> String {
    Path::new(root)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.to_owned())
}

fn text(id: impl Into<gpui_kit::ElementId>, value: impl Into<String>) -> impl IntoElement {
    let value = value.into();
    div()
        .id(id)
        .test_support()
        .aria_label(value.clone())
        .text_sm()
        .child(value)
}

impl PollingPage {
    pub(crate) fn new(
        services: Services,
        scroll: ContentScroll,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let operations = cx.observe(services.operations(), |_, _, cx| cx.notify());
        let scheduler = cx.observe(services.scheduler(), |_, _, cx| cx.notify());
        let mut page = Self {
            services,
            scroll,
            repositories: Read::Loading,
            generations: Generations::default(),
            read: None,
            focus: HashMap::new(),
            intervals: HashMap::new(),
            _picks: HashMap::new(),
            _operations: operations,
            _scheduler: scheduler,
        };
        page.load(cx);
        page
    }

    /// The page is shown again: the registered repositories are read again.
    pub(crate) fn opened(&mut self, cx: &mut Context<Self>) {
        self.load(cx);
    }

    #[allow(clippy::result_large_err)] // `ReadError` carries its scope by value.
    fn load(&mut self, cx: &mut Context<Self>) {
        let stamp = self.generations.next(SCOPE);
        if !matches!(self.repositories, Read::Ready(_)) {
            self.repositories = Read::Loading;
        }
        self.read = Some(self.services.read(
            ReadKind::Repositories,
            stamp,
            |service| service.list_repositories(),
            cx,
            |this: &mut Self, stamp, result, cx| {
                if !this.generations.is_current(&stamp) {
                    return;
                }
                this.repositories = Read::from_result(result, |list| {
                    list.items.into_iter().map(|summary| summary.root).collect()
                });
                if let Read::Ready(roots) = &this.repositories {
                    this.intervals.retain(|root, _| roots.contains(root));
                    this._picks.retain(|root, _| roots.contains(root));
                }
                cx.notify();
            },
        ));
        cx.notify();
    }

    fn focus(&mut self, id: &'static str, ix: usize, cx: &mut Context<Self>) -> FocusHandle {
        self.focus
            .entry((id, ix))
            .or_insert_with(|| cx.focus_handle())
            .clone()
    }

    /// This process's own Check now of `root` is queued or running (C1).
    fn checking_now(&self, root: &Path) -> bool {
        self.services
            .worker()
            .in_flight(|intent| matches!(intent, Intent::PollOnce { root: asked } if asked == root))
    }

    /// A policy verb of `root` is queued or running.
    fn configuring(&self, root: &Path) -> bool {
        self.services.worker().in_flight(|intent| {
            matches!(
                intent,
                Intent::PollConfigure { root: asked, .. } if asked == root
            )
        })
    }

    fn send(&mut self, intent: Intent, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_dialog(cx) {
            return;
        }
        self.services
            .worker()
            .submit(intent, Lane::User, None, None);
        cx.notify();
    }

    fn check_now(&mut self, root: &Path, window: &mut Window, cx: &mut Context<Self>) {
        if !self.checking_now(root) {
            self.send(
                Intent::PollOnce {
                    root: root.to_owned(),
                },
                window,
                cx,
            );
        }
    }

    fn toggle(&mut self, root: PathBuf, on: bool, window: &mut Window, cx: &mut Context<Self>) {
        let intent = if on {
            Intent::PollResume { root }
        } else {
            Intent::PollPause { root }
        };
        self.send(intent, window, cx);
    }

    /// `root`'s interval select, made the first time, showing `seconds`:
    /// the policy as the schedule holds it, except while a verb of the
    /// repository is queued or running or its policy is being read again,
    /// when the person's pick stays.
    fn interval(
        &mut self,
        root: &RepoKey,
        seconds: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<Interval> {
        let words = INTERVALS
            .iter()
            .find(|(offered, _)| *offered == seconds)
            .map(|(_, words)| *words);
        let held = self.configuring(Path::new(root))
            || self.services.scheduler().read(cx).refreshing(root);
        let state = match self.intervals.get(root) {
            Some(state) => state.clone(),
            None => {
                let at = words
                    .and_then(|words| INTERVALS.iter().position(|(_, shown)| *shown == words))
                    .map(IndexPath::new);
                let items: Vec<&'static str> = INTERVALS.iter().map(|(_, words)| *words).collect();
                let state =
                    cx.new(|cx| SelectState::new(SearchableVec::new(items), at, window, cx));
                let target = PathBuf::from(root);
                let pick = cx.subscribe_in(
                    &state,
                    window,
                    move |this, _, event: &SelectEvent<SearchableVec<&'static str>>, window, cx| {
                        let SelectEvent::Confirm(Some(words)) = event else {
                            return;
                        };
                        if let Some((seconds, _)) =
                            INTERVALS.iter().find(|(_, offered)| offered == words)
                        {
                            let intent = Intent::PollConfigure {
                                root: target.clone(),
                                interval_seconds: *seconds,
                            };
                            this.send(intent, window, cx);
                        }
                    },
                );
                self._picks.insert(root.clone(), pick);
                self.intervals.insert(root.clone(), state.clone());
                state
            }
        };
        if !held && state.read(cx).selected_value().copied() != words {
            let at = words
                .and_then(|words| INTERVALS.iter().position(|(_, shown)| *shown == words))
                .map(IndexPath::new);
            state.update(cx, |state, cx| state.set_selected_index(at, window, cx));
        }
        state
    }

    fn button(
        &mut self,
        (id, inner): (&'static str, &'static str),
        ix: usize,
        label: &'static str,
        disabled: bool,
        on_click: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> Wrapped {
        let focus = self.focus(id, ix, cx);
        let button = Button::new((inner, ix))
            .label(label)
            .disabled(disabled)
            .on_click(
                cx.listener(move |this, _: &ClickEvent, window, cx| on_click(this, window, cx)),
            );
        wrapped((id, ix), label, &focus, &self.scroll, button)
    }

    fn controls(
        &mut self,
        ix: usize,
        root: &RepoKey,
        controls: Controls,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let select = self.interval(root, controls.interval_seconds, window, cx);
        let placeholder = interval_words(controls.interval_seconds);
        let focus = self.focus("polling-interval", ix, cx);
        let interval = wrapped(
            ("polling-interval", ix),
            INTERVAL,
            &focus,
            &self.scroll,
            Select::new(&select)
                .id(("polling-interval-select", ix))
                .accessibility_label(INTERVAL)
                .placeholder(placeholder)
                .w(px(200.)),
        );
        let target = PathBuf::from(root);
        let check = self.button(
            ("polling-check", "polling-check-button"),
            ix,
            if controls.retry { RETRY_NOW } else { CHECK_NOW },
            controls.checking,
            move |this, window, cx| this.check_now(&target, window, cx),
            cx,
        );
        let target = PathBuf::from(root);
        let focus = self.focus("polling-switch", ix, cx);
        let switch = wrapped(
            ("polling-switch", ix),
            AUTOMATIC,
            &focus,
            &self.scroll,
            Switch::new(("polling-switch-toggle", ix))
                .checked(!controls.paused)
                .label(AUTOMATIC)
                .on_change(cx.listener(move |this, on: &bool, window, cx| {
                    this.toggle(target.clone(), *on, window, cx)
                })),
        );
        h_flex()
            .gap_3()
            .items_center()
            .flex_wrap()
            .child(interval)
            .child(check)
            .child(switch)
            .into_any_element()
    }

    fn row(
        &mut self,
        ix: usize,
        root: &RepoKey,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let name = name_of(root);
        let checking = self.checking_now(Path::new(root));
        let row: Row = {
            let scheduler = self.services.scheduler().read(cx);
            status::row(&Facts {
                name: &name,
                schedule: scheduler.schedule().repo(root),
                unplaced: scheduler.unplaced(root),
                holding: scheduler.holding(root),
                checking_now: checking,
                now: scheduler.utc(),
            })
        };
        let mut body = v_flex()
            .gap_1()
            .child(
                div()
                    .font_weight(gpui_kit::FontWeight::BOLD)
                    .child(text(("polling-name", ix), name)),
            )
            .child(text(("polling-status", ix), row.status.clone()))
            .children(row.detail.map(|detail| {
                div()
                    .text_xs()
                    .text_color(muted)
                    .child(text(("polling-detail", ix), detail))
            }));
        // What the row waits for the person to do (C3).
        if let Some(reason) = self.held_for(root, &row.shape, cx) {
            body = body.child(self.attention_action(ix, root, &reason, cx));
        }
        match row.shape {
            Shape::Polling(controls) => {
                if row.pending {
                    body = body.child(self.button(
                        ("polling-pending", "polling-pending-button"),
                        ix,
                        SHOW_IN_ACTIVITY,
                        false,
                        |_, _, cx| cx.emit(PollingEvent::ShowActivity),
                        cx,
                    ));
                }
                body = body.child(self.controls(ix, root, controls, window, cx));
            }
            Shape::NoSharedCopy => {
                let target = PathBuf::from(root);
                body = body.child(self.button(
                    ("polling-setup", "polling-setup-button"),
                    ix,
                    SET_UP_SHARING,
                    false,
                    move |_, _, cx| cx.emit(PollingEvent::SetUpSharing(target.clone())),
                    cx,
                ));
            }
            Shape::Pending | Shape::Off | Shape::Held => {}
        }
        body.into_any_element()
    }
}

impl Render for PollingPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let page = v_flex()
            .id("settings-polling")
            .test_support()
            .gap_3()
            .child(
                div()
                    .id("polling-heading")
                    .test_support()
                    .aria_label(POLLING_HEADING)
                    .text_base()
                    .font_weight(gpui_kit::FontWeight::BOLD)
                    .child(POLLING_HEADING),
            )
            .child(div().text_color(muted).child(text("polling-intro", INTRO)));
        let roots: Vec<RepoKey> = match &self.repositories {
            Read::Ready(roots) => roots.clone(),
            Read::Failed(code) => {
                return page
                    .child(text("polling-state", code.message()))
                    .child(div().text_xs().text_color(muted).child(code.as_str()));
            }
            Read::Busy => return page.child(text("polling-state", LABEL_BUSY)),
            Read::None | Read::Loading => {
                return page.child(text("polling-state", READING_REPOSITORIES));
            }
        };
        if roots.is_empty() {
            return page.child(text("polling-state", NO_REPOSITORIES));
        }
        let mut rows = v_flex().gap_4();
        for (ix, root) in roots.iter().enumerate() {
            rows = rows.child(self.row(ix, root, window, cx));
        }
        page.child(rows)
    }
}
