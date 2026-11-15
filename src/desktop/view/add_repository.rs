//! The Add repository forms (Wave 04 Cycle 05a design B4): Add an existing
//! repository and Create a repository, an entity of their own. Until the
//! Add repository sheet holds them, Settings › Repositories draws them
//! where they were, after Sharing: they reveal a focused control in
//! Settings' scroll, and go on with a submit only while Settings is open
//! (its `OpenFlag`).
//!
//! **Add existing** takes the folder of a Git repository that is not yet
//! a Manyhands repository, its primary branch and an optional short code
//! prefix. The folder is checked with `inspect_repository` (`checks`) when
//! Enter is pressed in its field, when focus leaves the field changed, and
//! when Browse chooses it: the check shows whether it is registered, its
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
//! not a repository included, by its typed code; the forms refuse only a
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
//! so a control that takes focus below the fold is brought into view
//! (`render`). Text is fixed strings, the library's own values, and
//! `ResultCode::message()` with its code.

use std::{collections::HashMap, path::PathBuf};

use gpui_kit::{
    AnyWindowHandle, App, AppContext, Context, Entity, FocusHandle, Focusable, PathPromptOptions,
    Subscription, Task, Window,
    component::input::{InputEvent, InputState},
};

mod checks;
mod render;

use self::checks::Inspection;
use crate::desktop::{
    model::stamp::Generations,
    services::{Services, requests::Intent},
    view::{confirm::confirm, scroll::ContentScroll, settings::OpenFlag},
};

pub(crate) const ADD_HEADING: &str = "Add an existing repository";
pub(crate) const CREATE_HEADING: &str = "Create a repository";
pub(crate) const FOLDER: &str = "Folder";
pub(crate) const CREATE_FOLDER: &str = "Folder (empty or not yet made)";
pub(crate) const BROWSE: &str = "Browse";
pub(crate) const PRIMARY_BRANCH: &str = "Primary branch";
pub(crate) const PREFIX: &str = "Short code prefix (optional)";
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

pub(crate) struct AddRepositorySheet {
    services: Services,
    /// The scroll the forms are drawn in, which each control's wrapper
    /// reveals it in: Settings' body region, while Settings draws them.
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
    /// check lands, the submit goes on (ruling R-T12b), while Settings is
    /// open (R-48).
    submit_pending: bool,
    /// Whether Settings is open: a check that lands after it closed opens
    /// no confirmation and moves no focus (design A4).
    open: OpenFlag,
    /// The window the forms are in, for going on with a submit once a read
    /// lands.
    window: AnyWindowHandle,
    _inputs: Vec<Subscription>,
}

/// The forms' inputs and the focus their wrappers track, by ID, as
/// Settings' Repositories page gives them to Cycle 04's tests (Ruling 9;
/// Task 4 removes it).
#[cfg(all(test, feature = "desktop-test-support"))]
#[derive(Clone)]
pub(crate) struct Controls {
    pub(crate) inputs: HashMap<&'static str, Entity<InputState>>,
    pub(crate) focus: HashMap<&'static str, FocusHandle>,
}

impl AddRepositorySheet {
    /// `scroll` is the region the forms are drawn in; `open` says whether
    /// what draws them is open.
    pub(crate) fn new(
        services: Services,
        scroll: ContentScroll,
        open: OpenFlag,
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
            open,
            window: window.window_handle(),
            _inputs: subscriptions,
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

    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn controls(&self) -> Controls {
        Controls {
            inputs: self.inputs.clone(),
            focus: self.focus.clone(),
        }
    }

    /// The Add folder field's focus, where the Add and Inspect routes land
    /// (Wave 04 Cycle 04 design Part D; R-24): always drawn.
    pub(crate) fn add_path_focus(&self, cx: &App) -> FocusHandle {
        self.inputs[Form::Add.path()].read(cx).focus_handle(cx)
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

    /// Settings closed: an Add waiting for its folder's check does not go on
    /// when the check lands (R-48).
    pub(crate) fn forget_pending_submit(&mut self) {
        self.submit_pending = false;
    }

    /// The check Add waited for landed (ruling R-T12b). If Git has no
    /// identity for the folder, its fields appear with focus in the first,
    /// and nothing is confirmed yet; otherwise the submit goes on. A check
    /// whose text changed meanwhile was dropped, and the wait with it.
    fn resume_submit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // A check that lands after Settings closed goes no further (A4).
        if !std::mem::take(&mut self.submit_pending) || !self.open.is_open(window, cx) {
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

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "add_repository_tests.rs"]
mod tests;
