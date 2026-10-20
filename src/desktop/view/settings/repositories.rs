//! The Repositories settings page (design, Settings / Repositories): Add
//! an existing repository and Create a repository.
//!
//! **Add existing** takes the folder of a Git repository that is not yet
//! a Manyhands repository, its primary branch and an optional short code
//! prefix. The folder is checked with `inspect_repository` when Enter is
//! pressed in its field, when focus leaves the field changed, and when
//! Browse chooses it: the check shows whether it is registered, its
//! configuration and the branch checked out, which fills an empty primary
//! branch. Only when the check reports that Git finds no identity for it
//! are name and email asked for, and written to its own configuration.
//! Add confirms only a folder whose current text was checked: pressed
//! before, it checks first, then shows the identity fields with focus in
//! the first when they are needed, or goes on to the preview (ruling
//! R-T12b).
//!
//! **Create** takes an empty or absent folder, its primary branch, an
//! optional identity and an optional prefix.
//!
//! Each submit builds `Intent::RepoEnable` or `Intent::RepoCreate` and
//! opens the confirmation dialog, whose preview creates nothing. The
//! library answers every refusal, a non-empty folder or a folder that is
//! not a repository included, by its typed code; the page refuses only a
//! form with no folder or no branch, before anything is asked. Once a
//! request succeeds, the workspace reads the repository list again and
//! the worker refreshes the new repository in the background once
//! (decision 4).
//!
//! Browse asks the platform for one directory, for both forms. A
//! dismissal leaves the field as it was; a platform failure disables
//! Browse, with why, and the field stays usable.
//!
//! Every control sits in a focus-tracking wrapper with a scroll marker,
//! so a control that takes focus below the fold is brought into view.
//! Text is fixed strings, the library's own values, and
//! `ResultCode::message()` with its code.

use std::{collections::HashMap, path::PathBuf};

use gpui_kit::{
    AnyWindowHandle, AppContext, ClickEvent, Context, Entity, FocusHandle, InteractiveElement,
    IntoElement, ParentElement, PathPromptOptions, Render, StatefulInteractiveElement, Styled,
    Subscription, Task, TestSupportExt, Window,
    component::{
        ActiveTheme, Disableable,
        button::{Button, ButtonVariants},
        h_flex,
        input::{Input, InputEvent, InputState},
        v_flex,
    },
    div, px,
};
use manyhands::{
    repository::{ConfigurationState, IdentityAvailability, RepositoryInspectionDto},
    results::ResultCode,
};

use super::wrapped;
use crate::desktop::{
    model::{
        list_state::LABEL_BUSY,
        stamp::{Generations, SettingsPage, ViewScope},
    },
    services::{ReadKind, ReadResult, Services, requests::Intent},
    view::{confirm::confirm, scroll::ContentScroll, sidebar::configuration_label},
};

pub(crate) const ADD_HEADING: &str = "Add an existing repository";
pub(crate) const CREATE_HEADING: &str = "Create a repository";
pub(crate) const FOLDER: &str = "Folder";
pub(crate) const CREATE_FOLDER: &str = "Folder (empty or not yet made)";
pub(crate) const BROWSE: &str = "Browse";
pub(crate) const PRIMARY_BRANCH: &str = "Primary branch";
pub(crate) const PREFIX: &str = "Short code prefix (optional)";
pub(crate) const NAME: &str = "Name";
pub(crate) const EMAIL: &str = "Email";
pub(crate) const CREATE_IDENTITY: &str =
    "The name and email its commits are to carry. Leave both empty to use the ones Git finds.";
pub(crate) const ADD: &str = "Add repository";
pub(crate) const CREATE: &str = "Create repository";
pub(crate) const CHECKING: &str = "Checking the folder";
pub(crate) const ALREADY_ADDED: &str = "Already added to Manyhands";
pub(crate) const NOT_ADDED: &str = "A Git repository, not yet added to Manyhands";
pub(crate) const CHECKED_OUT: &str = "Checked out";
pub(crate) const IDENTITY_REQUIRED: &str =
    "Git has no name and email for this repository. Enter the ones its commits are to carry.";
pub(crate) const NEED_FOLDER: &str = "Enter the folder's path.";
pub(crate) const NEED_BRANCH: &str = "Enter the primary branch.";
pub(crate) const BROWSE_PROMPT: &str = "Choose";
pub(crate) const BROWSE_UNAVAILABLE: &str =
    "The folder chooser could not be opened. Type the folder's path instead.";
pub(crate) const BROWSE_NOT_TEXT: &str =
    "The chosen folder's path cannot be used as text. Type the folder's path instead.";

/// The branch a new repository's form starts with.
const DEFAULT_BRANCH: &str = "main";

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Form {
    Add,
    Create,
}

impl Form {
    fn path(self) -> &'static str {
        match self {
            Self::Add => "add-path",
            Self::Create => "create-path",
        }
    }

    fn branch(self) -> &'static str {
        match self {
            Self::Add => "add-branch",
            Self::Create => "create-branch",
        }
    }

    fn prefix(self) -> &'static str {
        match self {
            Self::Add => "add-prefix",
            Self::Create => "create-prefix",
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Add => "add-name",
            Self::Create => "create-name",
        }
    }

    fn email(self) -> &'static str {
        match self {
            Self::Add => "add-email",
            Self::Create => "create-email",
        }
    }

    fn browse(self) -> &'static str {
        match self {
            Self::Add => "add-browse",
            Self::Create => "create-browse",
        }
    }

    fn submit(self) -> &'static str {
        match self {
            Self::Add => "add-submit",
            Self::Create => "create-submit",
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::Add => "add-message",
            Self::Create => "create-message",
        }
    }

    fn fields(self) -> [&'static str; 5] {
        [
            self.path(),
            self.branch(),
            self.prefix(),
            self.name(),
            self.email(),
        ]
    }
}

const FORMS: [Form; 2] = [Form::Add, Form::Create];

/// What checking the Add folder found.
enum Inspection {
    /// Not checked since its text last changed.
    None,
    Checking(String),
    Checked {
        /// The text that was checked.
        path: String,
        inspection: Box<RepositoryInspectionDto>,
    },
    Busy(String),
    Failed(ResultCode),
}

impl Inspection {
    /// The text this check is of, if it is of one.
    fn path(&self) -> Option<&str> {
        match self {
            Self::Checking(path) | Self::Busy(path) | Self::Checked { path, .. } => Some(path),
            Self::None | Self::Failed(_) => None,
        }
    }

    /// Whether the check of `path` found that Git has no identity for it.
    fn identity_required(&self, path: &str) -> bool {
        matches!(self, Self::Checked { path: checked, inspection }
            if checked == path && inspection.identity_state == IdentityAvailability::Required)
    }
}

pub(crate) struct RepositoriesPage {
    services: Services,
    scroll: ContentScroll,
    inputs: HashMap<&'static str, Entity<InputState>>,
    /// The focus each control's wrapper tracks, by its ID.
    focus: HashMap<&'static str, FocusHandle>,
    inspection: Inspection,
    /// The text the failed check was of, so leaving the field unchanged
    /// does not check it again.
    failed_path: Option<String>,
    generations: Generations,
    inspect_read: Option<Task<()>>,
    /// The checked-out branch a check found, for an empty primary branch:
    /// set at the next frame, which has the window a field needs.
    pending_branch: Option<String>,
    /// What each form said when it asked nothing.
    messages: HashMap<Form, &'static str>,
    /// Browse is waiting for the platform's answer.
    browsing: Option<Task<()>>,
    /// Why Browse is unavailable, once the platform failed.
    browse_unavailable: Option<&'static str>,
    /// Add was pressed before its folder's text was checked: once the
    /// check lands, the submit goes on (ruling R-T12b).
    submit_pending: bool,
    /// The window the page is in, for going on with a submit once a read
    /// lands.
    window: AnyWindowHandle,
    _inputs: Vec<Subscription>,
}

impl RepositoriesPage {
    pub(crate) fn new(
        services: Services,
        scroll: ContentScroll,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut inputs = HashMap::new();
        let mut subscriptions = Vec::new();
        for form in FORMS {
            for id in form.fields() {
                let input = cx.new(|cx| {
                    let input = InputState::new(window, cx);
                    if form == Form::Create && id == form.branch() {
                        input.default_value(DEFAULT_BRANCH)
                    } else {
                        input
                    }
                });
                subscriptions.push(cx.subscribe_in(
                    &input,
                    window,
                    move |this, _, event: &InputEvent, window, cx| {
                        this.input_event(form, id, event, window, cx)
                    },
                ));
                inputs.insert(id, input);
            }
        }
        let mut focus = HashMap::new();
        for form in FORMS {
            for id in form
                .fields()
                .into_iter()
                .chain([form.browse(), form.submit()])
            {
                focus.insert(id, cx.focus_handle());
            }
        }
        Self {
            services,
            scroll,
            inputs,
            focus,
            inspection: Inspection::None,
            failed_path: None,
            generations: Generations::default(),
            inspect_read: None,
            pending_branch: None,
            messages: HashMap::new(),
            browsing: None,
            browse_unavailable: None,
            submit_pending: false,
            window: window.window_handle(),
            _inputs: subscriptions,
        }
    }

    #[cfg(test)]
    pub(crate) fn input(&self, id: &str) -> Entity<InputState> {
        self.inputs[id].clone()
    }

    #[cfg(test)]
    pub(crate) fn control_focus(&self, id: &str) -> FocusHandle {
        self.focus[id].clone()
    }

    fn text(&self, id: &str, cx: &Context<Self>) -> String {
        self.inputs[id].read(cx).value().trim().to_owned()
    }

    /// Enter in a field checks the Add folder, or submits its form. The
    /// Add folder's text changing lets its check go; focus leaving it
    /// changed checks it.
    fn input_event(
        &mut self,
        form: Form,
        id: &'static str,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let add_path = id == Form::Add.path();
        match event {
            InputEvent::PressEnter { .. } if add_path => self.inspect(cx),
            InputEvent::PressEnter { .. } => self.submit(form, window, cx),
            InputEvent::Change if add_path => {
                let text = self.text(id, cx);
                if self.inspection.path() != Some(text.as_str()) {
                    self.forget_inspection(cx);
                }
            }
            InputEvent::Blur if add_path => {
                let text = self.text(id, cx);
                let checked = self.inspection.path() == Some(text.as_str())
                    || self.failed_path.as_deref() == Some(text.as_str());
                if !checked {
                    self.inspect(cx);
                }
            }
            _ => {}
        }
    }

    fn forget_inspection(&mut self, cx: &mut Context<Self>) {
        self.inspection = Inspection::None;
        self.submit_pending = false;
        self.failed_path = None;
        self.inspect_read = None;
        self.generations
            .next(ViewScope::Settings(SettingsPage::Repositories));
        cx.notify();
    }

    /// Checks the Add folder with `inspect_repository`. An empty field is
    /// not checked.
    #[allow(clippy::result_large_err)] // `ReadError` carries its scope by value.
    fn inspect(&mut self, cx: &mut Context<Self>) {
        let path = self.text(Form::Add.path(), cx);
        if path.is_empty() {
            self.forget_inspection(cx);
            return;
        }
        let stamp = self
            .generations
            .next(ViewScope::Settings(SettingsPage::Repositories));
        self.inspection = Inspection::Checking(path.clone());
        self.failed_path = None;
        let root = PathBuf::from(&path);
        self.inspect_read = Some(self.services.read(
            ReadKind::Inspect,
            stamp,
            move |service| service.inspect_repository(&root),
            cx,
            move |this: &mut Self, stamp, result, cx| {
                if !this.generations.is_current(&stamp) {
                    return;
                }
                this.inspection = match result {
                    ReadResult::Ok(inspection) => {
                        if this.text(Form::Add.branch(), cx).is_empty() {
                            this.pending_branch = inspection.head_branch.clone();
                        }
                        Inspection::Checked {
                            path,
                            inspection: Box::new(inspection),
                        }
                    }
                    ReadResult::Busy => Inspection::Busy(path),
                    ReadResult::Failed(failure) => {
                        this.failed_path = Some(path);
                        Inspection::Failed(failure.code)
                    }
                };
                if this.submit_pending {
                    let window = this.window;
                    cx.spawn(async move |this, cx| {
                        let _ = cx.update_window(window, |_, window, cx| {
                            let _ = this.update(cx, |this, cx| this.resume_submit(window, cx));
                        });
                    })
                    .detach();
                }
                cx.notify();
            },
        ));
        cx.notify();
    }

    /// The `repo.inspect` recovery: `root` in the Add folder, checked.
    pub(crate) fn inspect_root(
        &mut self,
        root: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(text) = root.to_str() else {
            return;
        };
        let text = text.to_owned();
        self.inputs[Form::Add.path()].update(cx, |input, cx| input.set_value(text, window, cx));
        self.inspect(cx);
    }

    /// Whether the Add folder's current text has been checked: what was
    /// found, or that the check failed.
    fn add_checked(&self, path: &str) -> bool {
        matches!(&self.inspection, Inspection::Checked { path: checked, .. } if checked == path)
            || self.failed_path.as_deref() == Some(path)
    }

    /// The check Add waited for landed (ruling R-T12b). If Git has no
    /// identity for the folder, its fields appear with focus in the first,
    /// and nothing is confirmed yet; otherwise the submit goes on. A check
    /// whose text changed meanwhile was dropped, and the wait with it.
    fn resume_submit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !std::mem::take(&mut self.submit_pending) {
            return;
        }
        if let Some(branch) = self.pending_branch.take() {
            self.inputs[Form::Add.branch()]
                .update(cx, |input, cx| input.set_value(branch, window, cx));
        }
        let path = self.text(Form::Add.path(), cx);
        if !self.add_checked(&path) {
            return;
        }
        if self.inspection.identity_required(&path) {
            self.inputs[Form::Add.name()].update(cx, |input, cx| input.focus(window, cx));
            cx.notify();
            return;
        }
        self.submit(Form::Add, window, cx);
    }

    /// Submit: the form's intent, in the confirmation dialog. Focus goes
    /// back to what held it when the dialog closes.
    fn submit(&mut self, form: Form, window: &mut Window, cx: &mut Context<Self>) {
        let path = self.text(form.path(), cx);
        // Add confirms only a folder whose current text was checked: the
        // check says whether it needs an identity (ruling R-T12b).
        if form == Form::Add && !path.is_empty() && !self.add_checked(&path) {
            self.submit_pending = true;
            self.messages.remove(&form);
            self.inspect(cx);
            return;
        }
        let branch = self.text(form.branch(), cx);
        let why = if path.is_empty() {
            Some(NEED_FOLDER)
        } else if branch.is_empty() {
            Some(NEED_BRANCH)
        } else {
            None
        };
        if let Some(why) = why {
            self.messages.insert(form, why);
            cx.notify();
            return;
        }
        self.messages.remove(&form);
        let asks_identity = match form {
            Form::Create => true,
            Form::Add => self.inspection.identity_required(&path),
        };
        let identity = asks_identity
            .then(|| (self.text(form.name(), cx), self.text(form.email(), cx)))
            .filter(|(name, email)| !(name.is_empty() && email.is_empty()));
        let prefix = Some(self.text(form.prefix(), cx)).filter(|prefix| !prefix.is_empty());
        let root = PathBuf::from(path);
        let intent = match form {
            Form::Add => Intent::RepoEnable {
                root,
                primary_branch: branch,
                identity,
                slug_prefix: prefix,
            },
            Form::Create => Intent::RepoCreate {
                root,
                primary_branch: branch,
                identity,
                slug_prefix: prefix,
            },
        };
        debug_assert!(intent.confirmed());
        let origin = window
            .focused(cx)
            .unwrap_or_else(|| self.focus[form.submit()].clone());
        confirm(intent, origin, &self.services, window, cx);
        cx.notify();
    }

    /// Browse: asks the platform for one directory.
    fn browse(&mut self, form: Form, window: &mut Window, cx: &mut Context<Self>) {
        if self.browsing.is_some() || self.browse_unavailable.is_some() {
            return;
        }
        let answer = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(BROWSE_PROMPT.into()),
        });
        self.browsing = Some(cx.spawn_in(window, async move |this, cx| {
            let answer = answer.await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.browsing = None;
                match answer {
                    Ok(Ok(Some(paths))) => this.chosen(form, paths, window, cx),
                    // Dismissed: the field stays as it was.
                    Ok(Ok(None)) => {}
                    Ok(Err(_)) | Err(_) => this.browse_unavailable = Some(BROWSE_UNAVAILABLE),
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    /// Browse chose `paths`: the first fills the form's folder, and the
    /// Add folder is checked.
    fn chosen(
        &mut self,
        form: Form,
        paths: Vec<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(path) = paths.into_iter().next() else {
            return;
        };
        let Some(text) = path.to_str() else {
            self.messages.insert(form, BROWSE_NOT_TEXT);
            return;
        };
        let text = text.to_owned();
        self.messages.remove(&form);
        self.inputs[form.path()].update(cx, |input, cx| input.set_value(text, window, cx));
        if form == Form::Add {
            self.inspect(cx);
        }
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

impl RepositoriesPage {
    /// A labeled field: its label, then its input in its wrapper.
    fn field(&self, id: &'static str, label: &'static str, cx: &Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        v_flex()
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
                .w(px(480.)),
            )
    }

    fn button(
        &self,
        id: &'static str,
        label: &'static str,
        primary: bool,
        disabled: bool,
        on_click: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let button = Button::new((id, 0usize))
            .label(label)
            .disabled(disabled)
            .on_click(
                cx.listener(move |this, _: &ClickEvent, window, cx| on_click(this, window, cx)),
            );
        let button = if primary { button.primary() } else { button };
        wrapped(id, label, &self.focus[id], &self.scroll, button)
    }

    /// The folder field with Browse beside it.
    fn folder(&self, form: Form, label: &'static str, cx: &mut Context<Self>) -> impl IntoElement {
        let disabled = self.browsing.is_some() || self.browse_unavailable.is_some();
        h_flex()
            .gap_2()
            .items_end()
            .child(self.field(form.path(), label, cx))
            .child(self.button(
                form.browse(),
                BROWSE,
                false,
                disabled,
                move |this, window, cx| this.browse(form, window, cx),
                cx,
            ))
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

    /// What the check of the Add folder found.
    fn inspection(&self, cx: &Context<Self>) -> Option<impl IntoElement + use<>> {
        let muted = cx.theme().muted_foreground;
        let body = v_flex().gap_1();
        let body = match &self.inspection {
            Inspection::None => return None,
            Inspection::Checking(_) => body.child(text("add-inspection", CHECKING)),
            Inspection::Busy(_) => body.child(text("add-inspection", LABEL_BUSY)),
            Inspection::Failed(code) => body.child(text("add-inspection", code.message())).child(
                div()
                    .id("add-inspection-code")
                    .test_support()
                    .aria_label(code.as_str())
                    .text_xs()
                    .text_color(muted)
                    .child(code.as_str()),
            ),
            Inspection::Checked { inspection, .. } => {
                let state = if inspection.registered {
                    ALREADY_ADDED
                } else {
                    NOT_ADDED
                };
                let configuration = &inspection.configuration;
                let body = body.child(text("add-inspection", state)).children(
                    inspection
                        .head_branch
                        .clone()
                        .map(|branch| text("add-head-branch", format!("{CHECKED_OUT}: {branch}"))),
                );
                let body = if inspection.registered
                    || configuration.state != ConfigurationState::Missing
                {
                    body.child(text(
                        "add-configuration",
                        configuration_label(configuration.state),
                    ))
                } else {
                    body
                };
                body.children(
                    configuration
                        .problems
                        .iter()
                        .enumerate()
                        .map(|(ix, problem)| text(("add-problem", ix), problem.code.guidance())),
                )
            }
        };
        Some(body)
    }

    fn message(&self, form: Form) -> Option<impl IntoElement + use<>> {
        self.messages
            .get(&form)
            .map(|why| text(form.message(), *why))
    }

    fn add_form(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let path = self.text(Form::Add.path(), cx);
        let identity = self.inspection.identity_required(&path).then(|| {
            v_flex()
                .gap_2()
                .child(text("add-identity-required", IDENTITY_REQUIRED))
                .child(self.field(Form::Add.name(), NAME, cx))
                .child(self.field(Form::Add.email(), EMAIL, cx))
        });
        v_flex()
            .gap_2()
            .child(Self::heading("add-heading", ADD_HEADING))
            .child(self.folder(Form::Add, FOLDER, cx))
            .children(self.inspection(cx))
            .child(self.field(Form::Add.branch(), PRIMARY_BRANCH, cx))
            .child(self.field(Form::Add.prefix(), PREFIX, cx))
            .children(identity)
            .children(self.message(Form::Add))
            .child(self.button(
                Form::Add.submit(),
                ADD,
                true,
                false,
                |this, window, cx| this.submit(Form::Add, window, cx),
                cx,
            ))
    }

    fn create_form(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let muted = cx.theme().muted_foreground;
        v_flex()
            .gap_2()
            .child(Self::heading("create-heading", CREATE_HEADING))
            .child(self.folder(Form::Create, CREATE_FOLDER, cx))
            .child(self.field(Form::Create.branch(), PRIMARY_BRANCH, cx))
            .child(div().text_xs().text_color(muted).child(CREATE_IDENTITY))
            .child(self.field(Form::Create.name(), NAME, cx))
            .child(self.field(Form::Create.email(), EMAIL, cx))
            .child(self.field(Form::Create.prefix(), PREFIX, cx))
            .children(self.message(Form::Create))
            .child(self.button(
                Form::Create.submit(),
                CREATE,
                true,
                false,
                |this, window, cx| this.submit(Form::Create, window, cx),
                cx,
            ))
    }
}

impl Render for RepositoriesPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(branch) = self.pending_branch.take() {
            self.inputs[Form::Add.branch()]
                .update(cx, |input, cx| input.set_value(branch, window, cx));
        }
        let unavailable = self
            .browse_unavailable
            .map(|why| text("browse-unavailable", why));
        v_flex()
            .id("settings-repositories")
            .test_support()
            .gap_4()
            .child(self.add_form(cx))
            .child(self.create_form(cx))
            .children(unavailable)
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "repositories_tests.rs"]
mod tests;
