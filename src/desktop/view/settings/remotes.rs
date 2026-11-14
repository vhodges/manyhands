//! The Remotes settings page (design, Settings / Remotes): one
//! repository's remotes, as `list_remotes_redacted` lists them, with
//! Add, Select for publication, Select none and Remove.
//!
//! The page works on the repository Settings chose (Manage on the
//! Repositories page, or a route), and is drawn on that page under Sharing
//! (Wave 04 Cycle 04 design C1): it lists no repositories of its own
//! (R-15). Each remote shows its name, its fetch and push locations as the
//! library redacts them, whether publication can use it and whether it is
//! the one selected, each in words. A raw location is never shown: the
//! only locations drawn are the library's redacted ones.
//!
//! **Add** takes a name and a location and submits `Intent::RemoteAdd` as
//! a user job. Adding writes the remote to Git's configuration and
//! contacts nothing. Once submitted, the form lets go of both fields, so
//! the location typed, with any credential it holds, stays in no field;
//! the request keeps it only to make each attempt (a Retry must ask the
//! same thing), and the operation area shows the command and the
//! repository, never the location.
//!
//! **Select for publication** and **Select none** submit
//! `Intent::RemoteSelect`. A remote that is not eligible is listed, said
//! to be so, and may still be selected: the library answers with its
//! typed refusal, shown here and in the operation area.
//!
//! **Remove** opens the confirmation dialog for `Intent::RemoteRemove`. A
//! remote the binding refuses to remove (`01M4PB7WZN9WNAQ3KXZ19J28KM`: a
//! sibling such as `origin.bak` beside `origin`) is answered at its
//! preview: the dialog shows the refusal's code and message, with no
//! Confirm, and both remotes stay.
//!
//! The remotes are read again whenever the page is shown (R-32) and
//! whenever a remote request of the chosen repository finishes, whoever
//! started it. The page's own latest add or selection is answered under
//! the form, by its outcome, the envelope's message and its code. Every
//! handler does nothing while a dialog is open. Text is fixed strings, the
//! library's own values, and the envelope's message with its code.

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use gpui_kit::{
    AppContext, ClickEvent, Context, Entity, FocusHandle, InteractiveElement, IntoElement,
    ParentElement, Render, StatefulInteractiveElement, Styled, Subscription, Task, TestSupportExt,
    Window,
    component::{
        ActiveTheme, WindowExt,
        button::{Button, ButtonVariants},
        h_flex,
        input::{Input, InputEvent, InputState},
        v_flex,
    },
    div,
};
use manyhands::{repository::RemoteDto, results::Outcome};

use super::{Wrapped, field_column, wrapped};
use crate::desktop::model::read_state::Read;
use crate::desktop::{
    model::{
        list_state::LABEL_BUSY,
        stamp::{Generations, SettingsPage, ViewScope},
    },
    services::{
        ReadKind, Services,
        requests::{DesktopRequestId, Intent, Lane, RequestState},
    },
    view::{confirm::confirm, operations::outcome_label, scroll::ContentScroll},
};

pub(crate) const REPOSITORY_HEADING: &str = "Repository";
/// The section the Repositories page draws the page under (design C1).
pub(crate) const SHARING_HEADING: &str = "Sharing";
pub(crate) const ADD_HEADING: &str = "Add a remote";
pub(crate) const NO_REPOSITORY: &str = "Choose a repository to see its remotes.";
pub(crate) const READING_REMOTES: &str = "Reading the remotes";
pub(crate) const NO_REMOTES: &str = "This repository has no remotes.";
pub(crate) const FETCHES_FROM: &str = "Fetches from";
pub(crate) const PUSHES_TO: &str = "Pushes to";
pub(crate) const ELIGIBLE: &str = "Eligible for publication";
pub(crate) const NOT_ELIGIBLE: &str =
    "Not eligible for publication: publishing needs a remote reached over SSH";
pub(crate) const SELECTED: &str = "Selected for publication";
pub(crate) const SELECTION_NONE: &str = "No remote is selected for publication.";
pub(crate) const SELECT: &str = "Select for publication";
pub(crate) const SELECT_NONE: &str = "Publish to no remote";
pub(crate) const REMOVE: &str = "Remove remote";
pub(crate) const NAME: &str = "Name";
pub(crate) const URL: &str = "Location (URL)";
pub(crate) const ADD: &str = "Add remote";
pub(crate) const ADD_NOTE: &str =
    "Adding a remote writes it to the repository's Git configuration. Nothing is contacted.";
pub(crate) const NEED_NAME: &str = "Enter the remote's name.";
pub(crate) const NEED_URL: &str = "Enter the remote's location.";

const NAME_FIELD: &str = "remote-add-name";
const URL_FIELD: &str = "remote-add-url";
const ADD_SUBMIT: &str = "remote-add-submit";
const SELECT_NONE_ID: &str = "remotes-select-none";

/// A remote's location line: what it is, and the library's redacted
/// location.
pub(crate) fn location_line(name: &str, location: &str) -> String {
    format!("{name}: {location}")
}

/// The line that names the publication remote.
pub(crate) fn selection_line(name: &str) -> String {
    format!("Publication remote: {name}")
}

/// The line that names the repository the page works on.
pub(crate) fn chosen_line(root: &str) -> String {
    format!("{REPOSITORY_HEADING}: {root}")
}

/// A repository's remotes, with the root they were read from: a row acts
/// on that root only.
#[derive(Clone)]
struct Listed {
    root: PathBuf,
    items: Vec<RemoteDto>,
}

/// The requests whose finishing can change a repository's remotes.
fn remote_command(intent: &Intent) -> bool {
    matches!(
        intent,
        Intent::RemoteAdd { .. } | Intent::RemoteRemove { .. } | Intent::RemoteSelect { .. }
    )
}

const SCOPE: ViewScope = ViewScope::Settings(SettingsPage::Remotes);

pub(crate) struct RemotesPage {
    services: Services,
    scroll: ContentScroll,
    /// The repository the page works on: the one Settings chose.
    root: Option<PathBuf>,
    /// The remotes last read, with the root they were read from.
    remotes: Read<Listed>,
    remote_generations: Generations,
    remote_read: Option<Task<()>>,
    inputs: HashMap<&'static str, Entity<InputState>>,
    /// The focus each control's wrapper tracks, by its ID; a row's are
    /// made when it is first drawn.
    focus: HashMap<String, FocusHandle>,
    /// What the Add form said when it asked nothing.
    message: Option<&'static str>,
    /// The page's latest add or selection, answered under the form.
    submitted: Option<DesktopRequestId>,
    /// The finished requests already answered, by attempt.
    seen: HashSet<(DesktopRequestId, u32)>,
    _inputs: Vec<Subscription>,
    _operations: Subscription,
}

impl RemotesPage {
    /// The page on `root`, which it reads when it is shown (`opened`).
    pub(crate) fn new(
        services: Services,
        scroll: ContentScroll,
        root: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut inputs = HashMap::new();
        let mut subscriptions = Vec::new();
        for id in [NAME_FIELD, URL_FIELD] {
            let input = cx.new(|cx| InputState::new(window, cx));
            subscriptions.push(cx.subscribe_in(
                &input,
                window,
                |this, _, event: &InputEvent, window, cx| {
                    if let InputEvent::PressEnter { .. } = event {
                        this.submit_add(window, cx)
                    }
                },
            ));
            inputs.insert(id, input);
        }
        let focus = [NAME_FIELD, URL_FIELD, ADD_SUBMIT, SELECT_NONE_ID]
            .into_iter()
            .map(|id| (id.to_owned(), cx.focus_handle()))
            .collect();
        let operations = cx.observe(services.operations(), |this, _, cx| {
            this.operations_changed(cx)
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
        Self {
            services,
            scroll,
            root,
            remotes: Read::Loading,
            remote_generations: Generations::default(),
            remote_read: None,
            inputs,
            focus,
            message: None,
            submitted: None,
            seen,
            _inputs: subscriptions,
            _operations: operations,
        }
    }

    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn input(&self, id: &str) -> Entity<InputState> {
        self.inputs[id].clone()
    }

    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn control_focus(&self, id: &str) -> FocusHandle {
        self.focus[id].clone()
    }

    /// The names of the remotes shown.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn remote_names(&self) -> Vec<String> {
        match &self.remotes {
            Read::Ready(listed) => listed
                .items
                .iter()
                .map(|remote| remote.name.clone())
                .collect(),
            _ => Vec::new(),
        }
    }

    /// The remote shown selected for publication.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn selected(&self) -> Option<&str> {
        match &self.remotes {
            Read::Ready(listed) => listed
                .items
                .iter()
                .find(|remote| remote.selected_for_publication)
                .map(|remote| remote.name.as_str()),
            _ => None,
        }
    }

    /// The page is shown: it reads the remotes it shows again (R-32).
    pub(crate) fn opened(&mut self, cx: &mut Context<Self>) {
        self.load_remotes(cx);
    }

    /// Works on `root`, which Settings chose (design C1), from now on, and
    /// reads its remotes; `None`, on none. Another repository's remotes are
    /// never left on show, not even while this one's are read: their rows
    /// would seem to be its own.
    pub(crate) fn open_on(&mut self, root: Option<PathBuf>, cx: &mut Context<Self>) {
        if root == self.root {
            return;
        }
        self.remotes = Read::Loading;
        self.remote_read = None;
        self.root = root;
        self.submitted = None;
        self.load_remotes(cx);
    }

    fn focus(&mut self, id: String, cx: &mut Context<Self>) -> FocusHandle {
        self.focus
            .entry(id)
            .or_insert_with(|| cx.focus_handle())
            .clone()
    }

    fn text(&self, id: &str, cx: &Context<Self>) -> String {
        self.inputs[id].read(cx).value().trim().to_owned()
    }

    /// Reads the remotes of the repository worked on.
    #[allow(clippy::result_large_err)] // `ReadError` carries its scope by value.
    fn load_remotes(&mut self, cx: &mut Context<Self>) {
        let stamp = self.remote_generations.next(SCOPE);
        let Some(root) = self.root.clone() else {
            self.remote_read = None;
            self.remotes = Read::Loading;
            cx.notify();
            return;
        };
        // The rows shown stay while the same root is read again; any
        // other root's go.
        if !matches!(&self.remotes, Read::Ready(listed) if listed.root == root) {
            self.remotes = Read::Loading;
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
                this.remotes = Read::from_result(result, |list| Listed {
                    root,
                    items: list.items,
                });
                cx.notify();
            },
        ));
        cx.notify();
    }

    /// Add: `remote add` of the form's name and location, as a user job.
    /// The form then lets both go.
    fn submit_add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_dialog(cx) {
            return;
        }
        let Some(root) = self.root.clone() else {
            return;
        };
        let name = self.text(NAME_FIELD, cx);
        let url = self.text(URL_FIELD, cx);
        let why = if name.is_empty() {
            Some(NEED_NAME)
        } else if url.is_empty() {
            Some(NEED_URL)
        } else {
            None
        };
        if let Some(why) = why {
            self.message = Some(why);
            cx.notify();
            return;
        }
        self.message = None;
        for id in [NAME_FIELD, URL_FIELD] {
            self.inputs[id].update(cx, |input, cx| input.set_value("", window, cx));
        }
        let intent = Intent::RemoteAdd { root, name, url };
        debug_assert!(!intent.confirmed());
        self.submit(intent, cx);
    }

    /// Whether `root`, the repository a row was read from, is still the
    /// one the page works on: a row left from another never acts.
    fn current(&self, root: &Path) -> bool {
        self.root.as_deref() == Some(root)
    }

    /// Select for publication, or Select none with no `name`, of `root`,
    /// the repository the row was read from, as a user job.
    fn submit_select(
        &mut self,
        root: PathBuf,
        name: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if window.has_active_dialog(cx) || !self.current(&root) {
            return;
        }
        let intent = Intent::RemoteSelect { root, name };
        debug_assert!(!intent.confirmed());
        self.submit(intent, cx);
    }

    fn submit(&mut self, intent: Intent, cx: &mut Context<Self>) {
        self.submitted = Some(
            self.services
                .worker()
                .submit(intent, Lane::User, None, None),
        );
        cx.notify();
    }

    /// Remove: `remote remove` of `name` in `root`, the repository the row
    /// was read from, in the confirmation dialog. Focus goes back to what
    /// held it when the dialog closes.
    fn submit_remove(
        &mut self,
        root: PathBuf,
        name: String,
        ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if window.has_active_dialog(cx) || !self.current(&root) {
            return;
        }
        let intent = Intent::RemoteRemove { root, name };
        debug_assert!(intent.confirmed());
        let origin = match window.focused(cx) {
            Some(focused) => focused,
            None => self.focus(format!("remote-remove-{ix}"), cx),
        };
        confirm(intent, origin, &self.services, window, cx);
    }

    /// A request finished: a remote request of the repository worked on
    /// reads its remotes again.
    fn operations_changed(&mut self, cx: &mut Context<Self>) {
        let mut remotes = false;
        let operations = self.services.operations().clone();
        for request in operations.read(cx).requests() {
            if !matches!(request.state, RequestState::Finished(..)) {
                continue;
            }
            let ours = remote_command(&request.intent)
                && self.root.is_some()
                && request.intent.root() == self.root.as_deref();
            if !ours || !self.seen.insert((request.id, request.attempt)) {
                continue;
            }
            remotes = true;
        }
        if remotes {
            self.load_remotes(cx);
        }
        // The page's own request may have moved on.
        cx.notify();
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

impl RemotesPage {
    /// A button in its wrapper `id`, whose focus is kept under `key`.
    fn button(
        &mut self,
        id: impl Into<gpui_kit::ElementId>,
        key: String,
        label: &'static str,
        on_click: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> Wrapped {
        let focus = self.focus(key.clone(), cx);
        let button = Button::new(gpui_kit::ElementId::Name(format!("{key}-button").into()))
            .label(label)
            .on_click(
                cx.listener(move |this, _: &ClickEvent, window, cx| on_click(this, window, cx)),
            );
        wrapped(id, label, &focus, &self.scroll, button)
    }

    fn field(&self, id: &'static str, label: &'static str, cx: &Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        field_column()
            .gap_0p5()
            .child(div().text_xs().text_color(muted).child(label))
            .child(
                wrapped(
                    id,
                    label,
                    &self.focus[id],
                    &self.scroll,
                    Input::new(&self.inputs[id]).aria_label(label),
                )
                .w_full(),
            )
    }

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
            Read::None | Read::Ready(_) => None,
        }
    }

    /// One remote: its name, locations, eligibility and selection in
    /// words, with Select and Remove.
    fn remote_row(
        &mut self,
        ix: usize,
        root: &Path,
        remote: &RemoteDto,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let name = remote.name.clone();
        let target = root.to_owned();
        let select = self.button(
            ("remote-select", ix),
            format!("remote-select-{ix}"),
            SELECT,
            move |this, window, cx| {
                this.submit_select(target.clone(), Some(name.clone()), window, cx)
            },
            cx,
        );
        let name = remote.name.clone();
        let target = root.to_owned();
        let remove = {
            let id = format!("remote-remove-{ix}");
            let focus = self.focus(id.clone(), cx);
            let button = Button::new(("remote-remove-button", ix))
                .danger()
                .label(REMOVE)
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    this.submit_remove(target.clone(), name.clone(), ix, window, cx)
                }));
            wrapped(("remote-remove", ix), REMOVE, &focus, &self.scroll, button)
        };
        let eligibility = if remote.publication_eligible {
            ELIGIBLE
        } else {
            NOT_ELIGIBLE
        };
        v_flex()
            .gap_0p5()
            .child(
                div()
                    .font_weight(gpui_kit::FontWeight::BOLD)
                    .child(text(("remote-name", ix), remote.name.clone())),
            )
            .child(text(
                ("remote-fetch", ix),
                location_line(FETCHES_FROM, &remote.fetch_location),
            ))
            .child(text(
                ("remote-push", ix),
                location_line(PUSHES_TO, &remote.push_location),
            ))
            .child(
                div()
                    .text_color(muted)
                    .child(text(("remote-eligibility", ix), eligibility)),
            )
            .children(
                remote
                    .selected_for_publication
                    .then(|| text(("remote-selected", ix), SELECTED)),
            )
            .child(h_flex().gap_2().flex_wrap().child(select).child(remove))
    }

    /// The repository's remotes and its publication selection.
    fn remote_rows(&mut self, cx: &mut Context<Self>) -> gpui_kit::Div {
        let body = v_flex()
            .gap_2()
            .child(heading("remotes-heading", SHARING_HEADING));
        let Some(root) = self.root.clone() else {
            return body.child(text("remotes-state", NO_REPOSITORY));
        };
        let body = body.child(text("remotes-root", chosen_line(&root.to_string_lossy())));
        if let Some(state) = Self::state_line("remotes-state", &self.remotes, READING_REMOTES, cx) {
            return body.child(state);
        }
        let Read::Ready(listed) = &self.remotes else {
            return body;
        };
        // Rows read from another repository are never shown under this one.
        if listed.root != root {
            return body.child(text("remotes-state", READING_REMOTES));
        }
        let Listed {
            root: read_from,
            items: remotes,
        } = listed.clone();
        let selection = match remotes
            .iter()
            .find(|remote| remote.selected_for_publication)
        {
            Some(remote) => selection_line(&remote.name),
            None => SELECTION_NONE.to_owned(),
        };
        let mut body = body.child(text("remotes-selection", selection));
        if remotes.is_empty() {
            body = body.child(text("remotes-state", NO_REMOTES));
        }
        for (ix, remote) in remotes.iter().enumerate() {
            body = body.child(self.remote_row(ix, &read_from, remote, cx));
        }
        let none = self.button(
            SELECT_NONE_ID,
            SELECT_NONE_ID.to_owned(),
            SELECT_NONE,
            move |this, window, cx| this.submit_select(read_from.clone(), None, window, cx),
            cx,
        );
        body.child(none)
    }

    /// The answer to the page's latest add or selection, once it has one.
    fn result(&self, cx: &Context<Self>) -> Option<impl IntoElement + use<>> {
        let id = self.submitted?;
        let operations = self.services.operations().read(cx);
        let RequestState::Finished(outcome, code, _, _, message) = &operations.request(id)?.state
        else {
            return None;
        };
        let muted = cx.theme().muted_foreground;
        let line = if *outcome == Outcome::Success {
            outcome_label(*outcome).to_owned()
        } else {
            format!("{}: {message}", outcome_label(*outcome))
        };
        Some(
            v_flex()
                .gap_0p5()
                .child(text("remotes-result", line))
                .child(
                    div()
                        .text_xs()
                        .text_color(muted)
                        .child(text("remotes-result-code", code.as_str())),
                ),
        )
    }

    fn add_form(&mut self, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
        self.root.as_ref()?;
        let muted = cx.theme().muted_foreground;
        let submit = {
            let button = Button::new((ADD_SUBMIT, 0usize))
                .primary()
                .label(ADD)
                .on_click(
                    cx.listener(|this, _: &ClickEvent, window, cx| this.submit_add(window, cx)),
                );
            wrapped(
                ADD_SUBMIT,
                ADD,
                &self.focus[ADD_SUBMIT],
                &self.scroll,
                button,
            )
        };
        Some(
            v_flex()
                .gap_2()
                .child(heading("remote-add-heading", ADD_HEADING))
                .child(self.field(NAME_FIELD, NAME, cx))
                .child(self.field(URL_FIELD, URL, cx))
                .child(div().text_xs().text_color(muted).child(ADD_NOTE))
                .children(self.message.map(|why| text("remote-add-message", why)))
                .child(submit)
                .children(self.result(cx)),
        )
    }
}

impl Render for RemotesPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let remotes = self.remote_rows(cx);
        let add = self.add_form(cx);
        v_flex()
            .id("settings-remotes")
            .test_support()
            .gap_4()
            .child(remotes)
            .children(add)
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "remotes_tests.rs"]
mod tests;
