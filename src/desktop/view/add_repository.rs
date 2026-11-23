//! The Add repository sheet (Wave 04 Cycle 05a design B4 to B10): Add an
//! existing folder and Create a new repository, an entity the workspace
//! keeps and draws in a sheet of its own (`workspace::add_sheet`, B1 to
//! B3). The sheet has its own scroll, which reveals a focused control, and
//! its own open flag: a check that lands after the sheet closed opens no
//! confirmation and moves no focus (C4 R-47's rule; G12).
//!
//! **The mode** (`mode`, B5) shows one form at a time; each keeps its typed
//! text while the other is shown, and every way in shows the existing
//! folder. The footer holds Cancel and the shown mode's submit (B10).
//!
//! **Add an existing folder** (B6) takes the folder of a Git repository,
//! its main branch and an optional short code prefix. The folder is checked
//! with `inspect_repository` (`checks`) when Enter is pressed in its field,
//! when focus leaves the field changed, when Browse… chooses it, and by a
//! check's own Retry or Check again. What the check found is drawn as rows,
//! each with at most one action. The main branch is the one checked out
//! (`branch`, B7), and Add repository is enabled only when the add can go
//! ahead (B10): pressed before the folder's text was checked, it checks
//! first and goes on only if the landed check enables it (R-T12b; G22).
//! Only when the check reports that Git finds no identity for it are name
//! and email asked for, and written to its own configuration; a folder
//! already set up keeps its own prefix, so none is asked (R-31). How it
//! shares is chosen among the folder's remotes (`share`, Part C), and the
//! workspace sends the choice once the add succeeds.
//!
//! **Create a new repository** (B8, `create`) takes a Name and a Location,
//! which `create-target` shows joined as typed and the sheet does not
//! check, its main branch, an optional identity and an optional prefix.
//! Create repository is enabled when Name, Location and Main branch are
//! filled (B10); the form draws no message about what is missing.
//!
//! Each submit builds `Intent::RepoEnable` or `Intent::RepoCreate` and
//! opens the confirmation dialog, whose preview creates nothing (`submit`).
//! The sheet follows it only to tell the workspace which request and
//! attempt it queued (`AddEvent::Confirmed`; B10, R-28); the workspace
//! keeps the add until it ends, and closes the sheet when it succeeds.
//! While an add of this open is pending, Add repository and Create
//! repository read "Adding…" and are disabled (R-27). Once a request
//! succeeds, the workspace reads the repository list again and the worker
//! refreshes the new repository in the background once (decision 4).
//!
//! Browse… asks the platform for one directory (`browse`), and the note
//! under it says why it is waiting or unavailable (Part G).
//!
//! Every body control sits in a focus-tracking wrapper with a scroll
//! marker, so a control that takes focus below the fold is brought into
//! view (`render`); a Kit button that focus is put on is drawn with its own
//! handle (`wrapped_kit`, B12). Text is fixed strings, the library's own
//! values, and `ResultCode::message()` with its code.

use std::{collections::HashMap, path::PathBuf};

use gpui_kit::{
    AnyWindowHandle, App, AppContext, Context, Entity, EventEmitter, FocusHandle, Focusable,
    Subscription, Task, Window,
    component::{
        input::{InputEvent, InputState},
        select::{SelectEvent, SelectState},
    },
};

mod branch;
mod browse;
mod checks;
mod copy;
mod create;
mod mode;
mod render;
mod rows;
mod share;
mod submit;

pub(crate) use copy::*;

use self::{
    branch::{Branch, Branches},
    checks::Inspection,
    mode::{Mode, SEGMENTS},
    share::{Share, Shares},
};
use crate::desktop::{
    model::stamp::Generations,
    services::{Services, requests::DesktopRequestId},
    view::{scroll::ContentScroll, settings::OpenFlag, workspace::AddRoute},
};

/// The branch a new repository's form starts with.
const DEFAULT_BRANCH: &str = "main";

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Form {
    Add,
    Create,
}

/// Create's Name: the new folder's name, which Location is joined with
/// (B8).
const FOLDER_NAME: &str = "create-folder-name";

impl Form {
    /// The form's folder: Add's folder, or Create's Location.
    fn path(self) -> &'static str {
        match self {
            Self::Add => "add-path",
            Self::Create => "create-location",
        }
    }

    /// Create's branch field; Add's main branch is a select (B7).
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

    /// The note under Browse… (B6 item 1).
    fn browse_note(self) -> &'static str {
        match self {
            Self::Add => "add-browse-note",
            Self::Create => "create-browse-note",
        }
    }

    /// The submit's wrapper; its own Kit button is `(submit, 0)`.
    fn submit(self) -> &'static str {
        match self {
            Self::Add => "add-submit",
            Self::Create => "create-submit",
        }
    }

    /// The form's text fields: Add's branch is not one, Create's Name is.
    fn fields(self) -> Vec<&'static str> {
        let fields = [self.path(), self.prefix(), self.name(), self.email()];
        match self {
            Self::Add => fields.to_vec(),
            Self::Create => [&[FOLDER_NAME], &fields[..1], &[self.branch()], &fields[1..]].concat(),
        }
    }
}

const FORMS: [Form; 2] = [Form::Add, Form::Create];

/// The footer's Cancel: its wrapper, and its own Kit button `(CANCEL_ID, 0)`.
const CANCEL_ID: &str = "add-cancel";

/// What the sheet asks of the workspace that keeps it (B3, B10).
pub(crate) enum AddEvent {
    /// The confirmation queued the add `request` at `attempt` (R-28): the
    /// workspace records it for this open, or replaces its entry, with the
    /// share it sends once the add succeeds (C2).
    Confirmed {
        request: DesktopRequestId,
        attempt: u32,
        share: Option<Option<String>>,
    },
    /// The footer's Cancel: the sheet closes, its text kept (`Kept`).
    Cancel,
    /// Open its settings, for the registered `root` (B6; decision 18).
    OpenSettings(PathBuf),
}

impl EventEmitter<AddEvent> for AddRepositorySheet {}

pub(crate) struct AddRepositorySheet {
    services: Services,
    /// The sheet body's own scroll, which each control's wrapper reveals it
    /// in (R-8).
    scroll: ContentScroll,
    /// The form shown (B5).
    mode: Mode,
    inputs: HashMap<&'static str, Entity<InputState>>,
    /// The focus each control's wrapper tracks, by its ID.
    focus: HashMap<&'static str, FocusHandle>,
    inspection: Inspection,
    generations: Generations,
    inspect_read: Option<Task<()>>,
    /// The main branch select (B7), and the items it was given last.
    branch: Entity<SelectState<Branches>>,
    branches: Vec<Branch>,
    /// The check changed since the selects' items were made: they are made
    /// again at the next frame, which has the window a select needs.
    choices_stale: bool,
    /// The Share changes through select (Part C), the items it was given
    /// last, and the choice: a remote's name, or `None` for Keep on this
    /// computer.
    share: Entity<SelectState<Shares>>,
    shares: Vec<Share>,
    share_choice: Option<String>,
    /// Browse is waiting for the platform's answer.
    browsing: Option<Task<()>>,
    /// Why Browse is unavailable, once the platform failed.
    browse_unavailable: Option<&'static str>,
    /// The form whose last Browse… chose a path that is not text.
    browse_not_text: Option<Form>,
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
        for id in [Form::Add.branch(), share::SHARE_ID, CANCEL_ID]
            .into_iter()
            .chain(rows::ACTION_IDS)
        {
            focus.insert(id, cx.focus_handle());
        }
        for (_, id, ..) in SEGMENTS {
            focus.insert(id, cx.focus_handle().tab_stop(true));
        }
        let branch = cx.new(|cx| SelectState::new(Branches::default(), None, window, cx));
        subscriptions.push(cx.subscribe_in(
            &branch,
            window,
            |this, _, event: &SelectEvent<Branches>, window, cx| {
                this.branch_confirmed(event, window, cx)
            },
        ));
        let (share, confirmed) = share::share_select(window, cx);
        subscriptions.push(confirmed);
        Self {
            services,
            scroll,
            mode: Mode::Existing,
            inputs,
            focus,
            inspection: Inspection::None,
            generations: Generations::default(),
            inspect_read: None,
            branch,
            branches: Vec::new(),
            // The first frame gives the selects their items: Keep on this
            // computer before any check (C1).
            choices_stale: true,
            share,
            shares: Vec::new(),
            share_choice: None,
            browsing: None,
            browse_unavailable: None,
            browse_not_text: None,
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

    /// The Add folder field's focus, where every way in lands (G1): drawn
    /// in the existing-folder mode, which every way in shows.
    pub(crate) fn add_path_focus(&self, cx: &App) -> FocusHandle {
        self.inputs[Form::Add.path()].read(cx).focus_handle(cx)
    }

    /// A way in (B2 steps 5 and 6): the existing-folder mode, the new
    /// mode's text kept; an `Inspect` fills and checks the folder.
    pub(crate) fn route(&mut self, route: AddRoute, window: &mut Window, cx: &mut Context<Self>) {
        self.set_mode(Mode::Existing, cx);
        if let AddRoute::Inspect(root) = route {
            self.inspect_root(root, window, cx);
        }
    }

    /// An add of this open is pending, or no longer (R-27).
    pub(crate) fn set_adding(&mut self, adding: bool, cx: &mut Context<Self>) {
        if self.adding != adding {
            self.adding = adding;
            cx.notify();
        }
    }

    /// A successful add of this open (B3 step 2; decision 16): both forms'
    /// typed text, the check and the choices are cleared. Create's branch
    /// starts again from its default.
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
        self.browse_not_text = None;
        cx.notify();
    }

    fn text(&self, id: &str, cx: &App) -> String {
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
                if self.inspection.path() != Some(text.as_str()) {
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

    /// The footer's Cancel.
    fn cancel(&mut self, cx: &mut Context<Self>) {
        cx.emit(AddEvent::Cancel);
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "add_repository_tests.rs"]
mod tests;
