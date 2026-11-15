//! The Add repository sheet (Wave 04 Cycle 05a design B4): Add an existing
//! repository and Create a repository, an entity the workspace keeps and
//! draws in a sheet of its own (`workspace::add_sheet`, B1 to B3). The
//! sheet has its own scroll, which reveals a focused control, and its own
//! open flag: a check that lands after the sheet closed opens no
//! confirmation and moves no focus (C4 R-47's rule; G12). Its footer holds
//! Cancel and the two submits (B10).
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
//! opens the confirmation dialog, whose preview creates nothing. The sheet
//! follows it (`confirm_followed`) only to tell the workspace which request
//! and attempt it queued (`AddEvent::Confirmed`; B10, R-28); the workspace
//! keeps the add until it ends, and closes the sheet when it succeeds. While
//! an add of this open is pending, Add repository and Create repository read
//! "Adding…" and are disabled (R-27). The confirmation gives focus back to
//! the submit that opened it, also while it reads "Adding…" (Ruling 18). The
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
//! Every body control sits in a focus-tracking wrapper with a scroll
//! marker, so a control that takes focus below the fold is brought into
//! view (`render`); the footer's Kit buttons are drawn with their own
//! handles (`wrapped_kit`, B12). Text is fixed strings, the library's own
//! values, and `ResultCode::message()` with its code.

use std::{collections::HashMap, path::PathBuf, rc::Rc};

use gpui_kit::{
    AnyWindowHandle, App, AppContext, Context, Entity, EventEmitter, FocusHandle, Focusable,
    Subscription, Task, Window,
    component::input::{InputEvent, InputState},
};

mod browse;
mod checks;
mod render;

use self::checks::Inspection;
use crate::desktop::{
    model::stamp::Generations,
    services::{
        Services,
        requests::{DesktopRequestId, Intent},
    },
    view::{
        confirm::{Followed, confirm_followed},
        scroll::ContentScroll,
        settings::OpenFlag,
    },
};

/// The sheet's header (Part F; B5, R-23).
pub(crate) const ADD_TITLE: &str = "Add a repository";
pub(crate) const ADD_DESCRIPTION: &str = "Bring a folder you already work in, or start a new one. Manyhands keeps its tickets and documents in the repository.";
/// The footer's Cancel (Part F; B10).
pub(crate) const CANCEL: &str = "Cancel";
/// Add repository and Create repository while an add of this open is
/// pending (B10; R-27).
pub(crate) const ADDING: &str = "Adding\u{2026}";
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

    /// The submit's wrapper; its own Kit button is `(submit, 0)`.
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

/// The footer's Cancel: its wrapper, and its own Kit button `(CANCEL_ID, 0)`.
const CANCEL_ID: &str = "add-cancel";

/// What the sheet asks of the workspace that keeps it (B3, B10).
pub(crate) enum AddEvent {
    /// The confirmation queued the add `request` at `attempt` (R-28): the
    /// workspace records it for this open, or replaces its entry.
    Confirmed {
        request: DesktopRequestId,
        attempt: u32,
    },
    /// The footer's Cancel: the sheet closes, its text kept (`Kept`).
    Cancel,
}

impl EventEmitter<AddEvent> for AddRepositorySheet {}

pub(crate) struct AddRepositorySheet {
    services: Services,
    /// The sheet body's own scroll, which each control's wrapper reveals it
    /// in (R-8).
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
    /// check lands, the submit goes on (ruling R-T12b), while the sheet is
    /// open (R-48).
    submit_pending: bool,
    /// Whether the sheet is open, its own flag, which the workspace sets
    /// and clears (B4): a check that lands after it closed opens no
    /// confirmation and moves no focus (G12).
    open: OpenFlag,
    /// An add of this open is pending: the submits read "Adding…" and are
    /// disabled (R-27). The workspace says so.
    adding: bool,
    /// The confirmation opened last, followed for the request it queues;
    /// the sheet holds the subscription only, never the dialog (B10).
    _followed: Option<Subscription>,
    /// The window the forms are in, for going on with a submit once a read
    /// lands.
    window: AnyWindowHandle,
    _inputs: Vec<Subscription>,
}

impl AddRepositorySheet {
    /// `scroll` is the sheet body's own region, `open` the sheet's own flag
    /// (Ruling 14).
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
        focus.insert(CANCEL_ID, cx.focus_handle());
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
            adding: false,
            _followed: None,
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

    /// Whether the sheet's open flag is set, read as the flag alone
    /// (Wave 04 Cycle 05a P7).
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn open_flag(&self) -> bool {
        self.open.get()
    }

    /// Whether an Add waits for its folder's check (P7, P8).
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn submit_pending(&self) -> bool {
        self.submit_pending
    }

    /// The vertical offset of the scroll the sheet's body is drawn in (P23).
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn scroll_offset(&self) -> gpui_kit::Pixels {
        self.scroll.handle().offset().y
    }

    /// The Add folder field's focus, where every way in lands (G1): always
    /// drawn.
    pub(crate) fn add_path_focus(&self, cx: &App) -> FocusHandle {
        self.inputs[Form::Add.path()].read(cx).focus_handle(cx)
    }

    /// An add of this open is pending, or no longer (R-27).
    pub(crate) fn set_adding(&mut self, adding: bool, cx: &mut Context<Self>) {
        if self.adding != adding {
            self.adding = adding;
            cx.notify();
        }
    }

    /// A successful add of this open (B3 step 2; decision 16): both forms'
    /// typed text, the check and what each form said are cleared.
    /// Create's branch starts again from its default.
    pub(crate) fn clear(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        for (id, input) in &self.inputs {
            let text = if *id == Form::Create.branch() {
                DEFAULT_BRANCH
            } else {
                ""
            };
            input.update(cx, |input, cx| input.set_value(text, window, cx));
        }
        self.forget_inspection(cx);
        self.pending_branch = None;
        self.messages.clear();
        cx.notify();
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

    /// The sheet closed: an Add waiting for its folder's check does not go
    /// on when the check lands (R-48; B3 step 2).
    pub(crate) fn forget_pending_submit(&mut self) {
        self.submit_pending = false;
    }

    /// Submit: the form's intent, in the confirmation dialog, which gives
    /// focus back to the form's submit when it closes (G13), also while its
    /// add is pending and the button disabled (Ruling 18). Nothing while an
    /// add of this open is pending (R-27).
    fn submit(&mut self, form: Form, window: &mut Window, cx: &mut Context<Self>) {
        if self.adding {
            return;
        }
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
        let origin = self.focus[form.submit()].clone();
        let dialog = confirm_followed(
            intent,
            Rc::new(move |_| origin.clone()),
            &self.services,
            window,
            cx,
        );
        if let Some(dialog) = dialog {
            self._followed = Some(cx.subscribe(&dialog, |_, _, followed: &Followed, cx| {
                if let Followed::Confirmed {
                    request, attempt, ..
                } = followed
                {
                    cx.emit(AddEvent::Confirmed {
                        request: *request,
                        attempt: *attempt,
                    });
                }
            }));
        }
        cx.notify();
    }

    /// The footer's Cancel.
    fn cancel(&mut self, cx: &mut Context<Self>) {
        cx.emit(AddEvent::Cancel);
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "add_repository_tests.rs"]
mod tests;
