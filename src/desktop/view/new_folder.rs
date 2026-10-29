//! New folder (design, Part E: New folder; R-24; fact 16; Cycle contract
//! 11): a folder below `docs/`, made in the primary worktree with no
//! commit, from the Documents pane's New folder… (E-10).
//!
//! - **The parent** is the folder selected in the tree, else `docs`, and
//!   the dialog shows the full path it makes as the name is typed. It says
//!   that a new folder stays on this computer until a document is saved in
//!   it.
//! - **The name** is refused here when it is empty, holds `/` or `\`, is
//!   `.` or `..`, or starts with `.`; anything else the library refuses is
//!   shown with its message and code.
//! - **Create**, on the user's action only (contract 11), sends `folder
//!   create`. Its `ok` closes the dialog (`Made`), as does
//!   `discovery_pending`, a folder made whose index refresh is owed (fix
//!   1, M1); the tree reads its folders again, as any finished request of
//!   its repository makes it: folder creation refreshes the index itself,
//!   so no refresh is sent here (R-24). `already_applied`, a retry's
//!   answer, reads "Folder exists", and the dialog stays; so does every
//!   other answer. Each attempt of its request is followed, a retry's
//!   included.
//! - **Keyboard.** Focus starts in the Folder name; Enter creates, when
//!   Create is offered; Escape and Cancel close the dialog, but not while a
//!   create runs, when Cancel is disabled as Create is (Part E review, M5);
//!   focus goes back where it was.
//!
//! The name is the user's text, so nothing here derives `Debug`; the
//! request carries it to the library and nothing else does.

use std::path::PathBuf;

use gpui_kit::{
    App, AppContext, Context, Entity, EventEmitter, FocusHandle, InteractiveElement, IntoElement,
    ParentElement, Render, Styled, Subscription, TestSupportExt, Window,
    component::{
        ActiveTheme, WindowExt, h_flex,
        input::{InputEvent, InputState},
        v_flex,
    },
};
use manyhands::results::ResultCode;

use super::{
    create::{CANCEL, CREATE, button, field, focus_the_note, held_button, keep_focus_in},
    item::text,
    workspace::Workspace,
};
use crate::desktop::{
    model::{create::DEFAULT_FOLDER, stamp::RepoKey},
    services::{
        Services,
        requests::{DesktopRequestId, Intent, Lane, RequestState},
    },
};

pub(crate) const NEW_FOLDER_HEADING: &str = "New folder";
/// The ruled words (pre-flight ruling, Task 24).
pub(crate) const FOLDER_NAME: &str = "Folder name";
pub(crate) const FOLDER_STAYS_LOCAL: &str =
    "A new folder stays on this computer until a document is saved in it.";
/// A retry's `already_applied` (the design's words).
pub(crate) const FOLDER_EXISTS: &str = "Folder exists";
pub(crate) const FOLDER_NAME_INVALID: &str = "This is not a valid folder name.";

/// Why `name` is not a folder's, as this dialog refuses it.
fn name_refused(name: &str) -> Option<&'static str> {
    let refused = name.is_empty() || name.contains(['/', '\\']) || name.starts_with('.');
    refused.then_some(FOLDER_NAME_INVALID)
}

/// The dialog's folder is made: its opener closes it (`finish`).
pub(crate) struct Made;

impl EventEmitter<Made> for NewFolderDialog {}

/// What the last Create was answered, while it made nothing new.
enum Answer {
    /// `already_applied`: the folder is there.
    Exists,
    /// The library's message and code.
    Refused { message: String, code: ResultCode },
}

pub(crate) struct NewFolderDialog {
    root: RepoKey,
    /// The folder the new one is made in.
    parent: String,
    services: Services,
    name: Entity<InputState>,
    /// The request Create sent, and the attempt whose answer was taken.
    sent: Option<(DesktopRequestId, u32)>,
    /// Create runs, from its press until its answer.
    running: bool,
    /// The folder is made: nothing more is offered.
    made: bool,
    answer: Option<Answer>,
    /// Around the dialog's own controls: it closes itself only while focus
    /// is here.
    focus: FocusHandle,
    /// Create's wrapper, where a keyed press of it parks focus (`held_button`).
    primary: FocusHandle,
    _events: Subscription,
    _operations: Subscription,
}

/// Opens New folder on the repository `root`, under `parent`, with focus
/// in its Folder name. Nothing opens while quitting.
fn open(
    root: RepoKey,
    parent: String,
    services: &Services,
    window: &mut Window,
    cx: &mut App,
) -> Option<Entity<NewFolderDialog>> {
    if services.worker().quitting() || services.operations().read(cx).saving_drafts() {
        return None;
    }
    let dialog = cx.new(|cx| NewFolderDialog::new(root, parent, services.clone(), window, cx));
    let built = dialog.clone();
    window.open_dialog(cx, move |base, _, _| {
        let creating = built.clone();
        let cancelling = built.clone();
        base.title(NEW_FOLDER_HEADING)
            .close_button(false)
            .overlay_closable(false)
            .child(built.clone())
            // Enter that no field or button took: Create, when offered.
            .on_ok(move |_, _, cx| {
                creating.update(cx, |dialog, cx| dialog.create(cx));
                false
            })
            // Escape: closed, but not while Create runs.
            .on_cancel(move |_, _, cx| !cancelling.read(cx).running)
    });
    let name = dialog.read(cx).name.clone();
    name.update(cx, |input, cx| input.focus(window, cx));
    #[cfg(test)]
    cx.set_global(OpenedFolder(dialog.downgrade()));
    Some(dialog)
}

/// The dialog last opened, for a test to type into and read.
#[cfg(test)]
pub(crate) struct OpenedFolder(pub(crate) gpui_kit::WeakEntity<NewFolderDialog>);

#[cfg(test)]
impl gpui_kit::Global for OpenedFolder {}

impl Workspace {
    /// New folder…, on the repository the Documents pane shows, under the
    /// folder selected in its tree, else `docs`. Nothing opens with no
    /// repository shown, nor in front of a dialog or the sheet.
    pub(in crate::desktop::view) fn new_folder(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if window.has_active_dialog(cx) || window.has_active_sheet(cx) {
            return;
        }
        let Some(root) = self.documents().repo().cloned() else {
            return;
        };
        let parent = self
            .documents_pane()
            .selected_folder(cx)
            .unwrap_or_else(|| DEFAULT_FOLDER.to_owned());
        let Some(dialog) = open(root, parent, self.services(), window, cx) else {
            return;
        };
        cx.subscribe_in(&dialog, window, |_, dialog, _: &Made, window, cx| {
            dialog.update(cx, |dialog, cx| dialog.finish(window, cx));
        })
        .detach();
    }
}

impl NewFolderDialog {
    fn new(
        root: RepoKey,
        parent: String,
        services: Services,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let name = cx.new(|cx| InputState::new(window, cx));
        let events = cx.subscribe_in(&name, window, |dialog, _, event, _, cx| match event {
            InputEvent::Change => {
                // A new name: what the last one was answered is not its.
                dialog.answer = None;
                cx.notify();
            }
            InputEvent::PressEnter { .. } => dialog.create(cx),
            InputEvent::Focus | InputEvent::Blur => {}
        });
        let operations = cx.observe(services.operations(), |dialog, _, cx| dialog.followed(cx));
        Self {
            root,
            parent,
            services,
            name,
            sent: None,
            running: false,
            made: false,
            answer: None,
            focus: cx.focus_handle(),
            primary: cx.focus_handle(),
            _events: events,
            _operations: operations,
        }
    }

    #[cfg(test)]
    pub(crate) fn name_input(&self) -> Entity<InputState> {
        self.name.clone()
    }

    /// The full path the dialog makes, as the name is typed.
    fn path(&self, cx: &App) -> String {
        format!("{}/{}", self.parent, self.name.read(cx).value())
    }

    /// Why Create is not offered, in words: the name refused here, or one
    /// the library has just found there.
    fn reason(&self, cx: &App) -> Option<&'static str> {
        name_refused(&self.name.read(cx).value())
            .or_else(|| matches!(self.answer, Some(Answer::Exists)).then_some(FOLDER_EXISTS))
    }

    /// Whether Create can create now.
    pub(crate) fn offered(&self, cx: &App) -> bool {
        !self.running && !self.made && self.reason(cx).is_none()
    }

    /// Create: `folder create` on the user lane, while offered.
    fn create(&mut self, cx: &mut Context<Self>) {
        if !self.offered(cx) {
            return;
        }
        let intent = Intent::FolderCreate {
            root: PathBuf::from(&self.root),
            path: self.path(cx),
        };
        let request = self
            .services
            .worker()
            .submit(intent, Lane::User, None, None);
        self.sent = Some((request, 0));
        self.running = true;
        self.answer = None;
        cx.notify();
    }

    /// The request model changed: an attempt of the sent request that this
    /// dialog has not taken runs, or was answered.
    fn followed(&mut self, cx: &mut Context<Self>) {
        let Some((request, taken)) = self.sent else {
            return;
        };
        let (attempt, state) = {
            let operations = self.services.operations().read(cx);
            let Some(sent) = operations.request(request) else {
                return;
            };
            if sent.attempt <= taken {
                return;
            }
            (sent.attempt, sent.state.clone())
        };
        match state {
            RequestState::Queued | RequestState::Running { .. } => {
                self.running = true;
                cx.notify();
                return;
            }
            // A folder made, or found, whose index refresh is owed is made: the
            // index alone owes work (fix 1, M1).
            RequestState::Finished(_, ResultCode::Ok | ResultCode::DiscoveryPending, ..) => {
                self.running = false;
                self.made = true;
                cx.emit(Made);
            }
            RequestState::Finished(_, ResultCode::AlreadyApplied, ..) => {
                self.running = false;
                self.answer = Some(Answer::Exists);
            }
            RequestState::Finished(_, code, _, _, message) => {
                self.running = false;
                self.answer = Some(Answer::Refused { message, code });
            }
            RequestState::Removed | RequestState::Dismissed | RequestState::NotSent(_) => {
                self.running = false;
            }
        }
        self.sent = Some((request, attempt));
        cx.notify();
    }

    /// Its folder is made (`Made`): the dialog closes itself, while it is
    /// the dialog focus is in. The Kit closes only the topmost dialog.
    pub(crate) fn finish(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.focus.contains_focused(window, cx) {
            window.close_dialog(cx);
        }
        cx.notify();
    }

    /// Cancel: closed, but not while Create runs.
    fn cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.running {
            window.close_dialog(cx);
        }
    }
}

/// The dialog: what a new folder is, the Folder name with its note, the
/// full path, what the last Create was answered, and Cancel and Create.
impl Render for NewFolderDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (danger, muted) = (cx.theme().danger, cx.theme().muted_foreground);
        let note = name_refused(&self.name.read(cx).value()).map(str::to_owned);
        focus_the_note(
            &self.primary,
            note.is_some().then_some(&self.name),
            window,
            cx,
        );
        let answer = self.answer.as_ref().map(|answer| {
            let (words, code) = match answer {
                Answer::Exists => (FOLDER_EXISTS.to_owned(), None),
                Answer::Refused { message, code } => (message.clone(), Some(code.as_str())),
            };
            let color = if code.is_some() { danger } else { muted };
            v_flex()
                .gap_1()
                .child(text("new-folder-answer", words).text_sm().text_color(color))
                .children(code.map(|code| {
                    text("new-folder-answer-code", code)
                        .text_xs()
                        .text_color(muted)
                }))
        });
        let entity = cx.entity().downgrade();
        let cancel = button("new-folder-cancel", CANCEL, false, !self.running, None, {
            let entity = entity.clone();
            move |_, window, cx| {
                let _ = entity.update(cx, |dialog, cx| dialog.cancel(window, cx));
            }
        });
        let create = held_button(
            "new-folder-ok",
            CREATE,
            self.offered(cx),
            self.reason(cx).map(str::to_owned),
            (&self.primary, window, cx),
            move |_, window, cx| {
                let _ = entity.update(cx, |dialog, cx| {
                    keep_focus_in(&dialog.focus, &dialog.primary, &[&dialog.name], window, cx);
                    dialog.create(cx)
                });
            },
        );
        v_flex()
            .id("new-folder-dialog")
            .test_support()
            .track_focus(&self.focus)
            .gap_3()
            .child(
                text("new-folder-explain", FOLDER_STAYS_LOCAL)
                    .text_sm()
                    .text_color(muted),
            )
            .child(field("new-folder-name", FOLDER_NAME, &self.name, note, cx))
            .child(
                text("new-folder-path", self.path(cx))
                    .text_xs()
                    .text_color(muted),
            )
            .children(answer)
            .child(h_flex().gap_2().justify_end().child(cancel).child(create))
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "new_folder_tests.rs"]
mod tests;
