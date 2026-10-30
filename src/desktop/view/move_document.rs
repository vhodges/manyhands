//! Move… (design, Part E: Move; R-23; Cycle contract 11): a document moved
//! to another folder or file name, keeping its ID, from its details
//! sheet's Folder row, which offers it only while R-23 holds
//! (`item::sheet::document`).
//!
//! - **The fields.** A Folder and a File name, the document's own until
//!   changed. The destination is checked as they are typed, as a new
//!   document's path is (`model::create::document_path`, with its words),
//!   and must not be where the document is; then `observe_path`, in the
//!   document's own context when its read names an active one and otherwise
//!   on primary, must find nothing there. Each change checks again under a
//!   new stamp, the last check's task dropped (`PathCheck`).
//! - **Move**, on the user's action only (contract 11), in the design's
//!   order: R-23 is asked of the tab again; the document is read again by
//!   its ID, for its observation; the destination is observed again; then
//!   `document move` is sent with both observations. A document the read
//!   finds at the destination already has moved: an earlier answer was
//!   lost.
//! - **Answers.** `ok` and `already_applied` close the dialog (`Moved`),
//!   and the tab, which reads its item again by its ID once any move of its
//!   repository finishes (R-B2), shows the new path. A file at the
//!   destination, found here or by the library (`occupied_path`), is said
//!   under the File name. `external_change` reads the document again by its
//!   ID: found at the destination, the move is done (the pre-flight
//!   ruling for known defect `01M4PB80T7`); found where it was read with a
//!   file now at the destination, which makes the library answer so, it is
//!   that collision (fix 1, M2); otherwise its source changed, the dialog
//!   says so, and Move is not offered again in it. Anything else is the
//!   library's message and code. Every refusal leaves the dialog open, and
//!   a change of the fields clears the last refusal of the destination they
//!   gave. Each attempt of its request is followed, a retry's included.
//!   An answer whose checkpoint committed has moved the document, whatever
//!   hand-off it still owes, which the operation area offers. The steps are
//!   `steps`'s.
//! - **Keyboard.** Focus starts in the Folder; Enter in a field moves, when
//!   Move is offered; Escape and Cancel close the dialog, but not while a
//!   move runs, when Cancel is disabled as Move is (Part E review, M5);
//!   focus goes back where it was.
//!
//! The paths are the user's text, so nothing here derives `Debug`; the
//! request carries them to the library and nothing else does.

use std::path::{Path, PathBuf};

use gpui_kit::{
    App, AppContext, Context, Entity, EventEmitter, FocusHandle, InteractiveElement, IntoElement,
    ParentElement, Render, Styled, Subscription, Task, TestSupportExt, WeakEntity, Window,
    component::{
        ActiveTheme, WindowExt, h_flex,
        input::{InputEvent, InputState},
        v_flex,
    },
};
use manyhands::{
    canonical::ItemId,
    repository::{ItemContextKind, ItemDto, PathObservationDto},
    results::ResultCode,
};

use super::{
    create::{
        CANCEL, FILE_NAME, FOLDER, PathCheck, button, field, focus_the_note, held_button,
        keep_focus_in,
    },
    item::{ItemPane, text},
};
use crate::desktop::{
    model::{
        create::{document_path, path_held},
        stamp::{Generations, RepoKey, Stamp, ViewScope},
    },
    services::{ReadKind, ReadResult, Services, requests::DesktopRequestId},
};

mod steps;

/// The sheet's control, on the Folder row.
pub(crate) const MOVE: &str = "Move\u{2026}";
pub(crate) const MOVE_HEADING: &str = "Move document";
pub(crate) const MOVE_ACTION: &str = "Move";
/// R-23's words (pre-flight ruling, Task 24): a session, a draft or a save.
pub(crate) const MOVE_WHILE_VIEWING: &str =
    "Move is available while viewing, with no draft or save in progress.";
/// R-23's words: a release of the item's context is pending.
pub(crate) const MOVE_WAITS: &str = "Move waits while a private copy is removed.";
/// A stale source (pre-flight ruling, Task 24).
pub(crate) const MOVE_STALE: &str =
    "This document changed since it was read. Close this and try again.";
/// The destination is where the document is (not ruled).
pub(crate) const MOVE_SAME_PLACE: &str = "The document is already at this path.";

/// The document a dialog moves, as its tab read it.
pub(crate) struct Source {
    pub(crate) root: RepoKey,
    pub(crate) id: ItemId,
    pub(crate) path: String,
    /// The worktree its destination is observed in: its own context's, when
    /// the read names an active one; otherwise primary.
    pub(crate) context: Option<PathBuf>,
}

/// The context `item`'s destination is observed in (`Source::context`).
pub(crate) fn context_of(item: &ItemDto) -> Option<PathBuf> {
    (item.context.kind == ItemContextKind::Active).then(|| PathBuf::from(&item.context.worktree))
}

/// The dialog's document is moved: its opener closes it (`finish`).
pub(crate) struct Moved;

impl EventEmitter<Moved> for MoveDialog {}

/// What a read of the document again goes on to.
#[derive(Clone, Copy)]
enum Then {
    /// Move was pressed: the destination is observed again, then the move
    /// sent.
    Send,
    /// The library answered `external_change`: moved, or stale.
    Judge,
}

/// The request Move sent, the attempt whose answer was taken, and where it
/// moves the document.
struct Sent {
    request: DesktopRequestId,
    taken: u32,
    destination: String,
}

/// Why the last Move moved nothing.
enum Failure {
    /// R-23 no longer holds, in its words.
    Blocked(&'static str),
    /// A file is at a destination the fields no longer give.
    Held(String),
    /// The document changed since the dialog read it: Move is not offered
    /// again in this dialog, as its words say (fix 1, M4).
    Stale,
    /// The library's message and code.
    Library { message: String, code: ResultCode },
}

impl Failure {
    /// A read's answer, which has its code's message.
    fn of(code: ResultCode) -> Self {
        Self::Library {
            message: code.message().to_owned(),
            code,
        }
    }

    /// What it says, and the library's code with its message.
    fn words(&self) -> (String, Option<&'static str>) {
        match self {
            Self::Blocked(words) => ((*words).to_owned(), None),
            Self::Held(path) => (path_held(path), None),
            Self::Stale => (MOVE_STALE.to_owned(), None),
            Self::Library { message, code } => (message.clone(), Some(code.as_str())),
        }
    }

    /// Whether a change of the fields clears it: it refused the destination
    /// they gave (fix 1, M4).
    fn of_destination(&self) -> bool {
        matches!(self, Self::Held(_) | Self::Library { .. })
    }
}

pub(crate) struct MoveDialog {
    source: Source,
    services: Services,
    /// The tab it was opened from: R-23 is asked of it again on Move.
    pane: WeakEntity<ItemPane>,
    folder: Entity<InputState>,
    file_name: Entity<InputState>,
    check: PathCheck,
    checks: Generations,
    observe: Option<Task<()>>,
    /// Move's reads, each under a stamp of its own.
    steps: Generations,
    step: Option<Task<()>>,
    /// Move runs, from its press until its answer.
    running: bool,
    sent: Option<Sent>,
    /// The document is moved: nothing more is offered, and Escape and Cancel
    /// close the dialog.
    moved: bool,
    failure: Option<Failure>,
    /// Around the dialog's own controls: it closes itself only while focus
    /// is here, so never a dialog opened above it.
    focus: FocusHandle,
    /// Move's wrapper, where a keyed press of it parks focus (`held_button`).
    primary: FocusHandle,
    _events: Vec<Subscription>,
    _operations: Subscription,
}

/// Opens Move… for `source`, read in the tab `pane`, with focus in its
/// Folder. Nothing opens while quitting. Gives the dialog, whose `Moved`
/// the caller follows.
pub(crate) fn open(
    source: Source,
    pane: WeakEntity<ItemPane>,
    services: &Services,
    window: &mut Window,
    cx: &mut App,
) -> Option<Entity<MoveDialog>> {
    if services.worker().quitting() || services.operations().read(cx).saving_drafts() {
        return None;
    }
    let dialog = cx.new(|cx| MoveDialog::new(source, pane, services.clone(), window, cx));
    let built = dialog.clone();
    window.open_dialog(cx, move |base, _, _| {
        let moving = built.clone();
        let cancelling = built.clone();
        base.title(MOVE_HEADING)
            .close_button(false)
            .overlay_closable(false)
            .child(built.clone())
            // Enter that no field or button took: Move, when offered. The
            // dialog closes once the document is moved, so never here.
            .on_ok(move |_, _, cx| {
                moving.update(cx, |dialog, cx| dialog.move_document(cx));
                false
            })
            // Escape: closed with nothing moved, but not while Move runs.
            .on_cancel(move |_, _, cx| !cancelling.read(cx).running)
    });
    let folder = dialog.read(cx).folder.clone();
    folder.update(cx, |input, cx| input.focus(window, cx));
    #[cfg(test)]
    cx.set_global(OpenedMove(dialog.downgrade()));
    Some(dialog)
}

/// The dialog last opened, for a test to type into and read.
#[cfg(test)]
pub(crate) struct OpenedMove(pub(crate) gpui_kit::WeakEntity<MoveDialog>);

#[cfg(test)]
impl gpui_kit::Global for OpenedMove {}

impl MoveDialog {
    fn new(
        source: Source,
        pane: WeakEntity<ItemPane>,
        services: Services,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let (folder, file_name) = source.path.rsplit_once('/').unwrap_or(("", &source.path));
        let input = |value: String, window: &mut Window, cx: &mut Context<Self>| {
            cx.new(|cx| InputState::new(window, cx).default_value(value))
        };
        let folder = input(folder.to_owned(), window, cx);
        let file_name = input(file_name.to_owned(), window, cx);
        let events = [folder.clone(), file_name.clone()]
            .into_iter()
            .map(|input| {
                cx.subscribe_in(&input, window, |dialog, _, event, _, cx| match event {
                    InputEvent::Change => dialog.changed(cx),
                    InputEvent::PressEnter { .. } => dialog.move_document(cx),
                    InputEvent::Focus | InputEvent::Blur => {}
                })
            })
            .collect();
        let operations = cx.observe(services.operations(), |dialog, _, cx| dialog.followed(cx));
        let mut dialog = Self {
            source,
            services,
            pane,
            folder,
            file_name,
            check: PathCheck::Checking,
            checks: Generations::default(),
            observe: None,
            steps: Generations::default(),
            step: None,
            running: false,
            sent: None,
            moved: false,
            failure: None,
            focus: cx.focus_handle(),
            primary: cx.focus_handle(),
            _events: events,
            _operations: operations,
        };
        dialog.check_path(cx);
        dialog
    }

    #[cfg(test)]
    pub(crate) fn folder_input(&self) -> Entity<InputState> {
        self.folder.clone()
    }

    #[cfg(test)]
    pub(crate) fn file_input(&self) -> Entity<InputState> {
        self.file_name.clone()
    }

    /// Whether Move can move now: the destination checked free, no move
    /// running or done, and the document not found changed since it was
    /// read.
    pub(crate) fn offered(&self) -> bool {
        !self.running
            && !self.moved
            && !matches!(self.failure, Some(Failure::Stale))
            && matches!(self.check, PathCheck::Free(_))
    }

    /// Why Move is not offered, while it says: a stale document, else the
    /// destination's check.
    fn reason(&self) -> Option<String> {
        match self.failure {
            Some(Failure::Stale) => Some(MOVE_STALE.to_owned()),
            _ => self.check.note(),
        }
    }

    /// The destination the fields give, or why it is not one.
    fn destination(&self, cx: &App) -> Result<String, &'static str> {
        let folder = self.folder.read(cx).value();
        let file_name = self.file_name.read(cx).value();
        let path = document_path(&folder, &file_name)?;
        if path == self.source.path {
            return Err(MOVE_SAME_PLACE);
        }
        Ok(path)
    }

    /// A field changed: the last refusal of the destination it gave goes,
    /// and the destination is checked again.
    fn changed(&mut self, cx: &mut Context<Self>) {
        if self.failure.as_ref().is_some_and(Failure::of_destination) {
            self.failure = None;
        }
        self.check_path(cx);
        cx.notify();
    }

    /// Checks the destination as the fields hold it now, under a new stamp:
    /// here first, then with `observe_path` in the document's context.
    #[allow(clippy::result_large_err)] // The library's read error, as every read gives it.
    fn check_path(&mut self, cx: &mut Context<Self>) {
        let stamp = self.checks.next(ViewScope::Create);
        self.observe = None;
        let path = match self.destination(cx) {
            Ok(path) => path,
            Err(words) => {
                self.check = PathCheck::Refused(words);
                return;
            }
        };
        self.check = PathCheck::Checking;
        self.observe = Some(self.observe_at(
            stamp,
            path.clone(),
            cx,
            move |dialog: &mut Self, stamp, result, cx| {
                if dialog.checks.is_current(&stamp) {
                    dialog.check = PathCheck::observed(path, result);
                    cx.notify();
                }
            },
        ));
    }

    /// `observe_path` of `path` in the document's context, under `stamp`.
    #[allow(clippy::result_large_err)] // The library's read error, as every read gives it.
    fn observe_at(
        &self,
        stamp: Stamp,
        path: String,
        cx: &mut Context<Self>,
        apply: impl FnOnce(&mut Self, Stamp, ReadResult<PathObservationDto>, &mut Context<Self>)
        + 'static,
    ) -> Task<()> {
        let root = PathBuf::from(&self.source.root);
        let context = self.source.context.clone();
        self.services.read(
            ReadKind::ObservePath,
            stamp,
            move |service| {
                let repo = service.resolve_repository(&root)?;
                service.observe_path(&repo, context.as_deref(), Path::new(&path))
            },
            cx,
            apply,
        )
    }

    /// Its document is moved (`Moved`): the dialog closes itself, while it is
    /// the dialog focus is in. The Kit closes only the topmost dialog.
    pub(crate) fn finish(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.focus.contains_focused(window, cx) {
            window.close_dialog(cx);
        }
        cx.notify();
    }

    /// Cancel: closed with nothing moved, but not while Move runs.
    fn cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.running {
            window.close_dialog(cx);
        }
    }
}

/// The dialog: the Folder and the File name, with the destination's note,
/// what the last Move said, and Cancel and Move.
impl Render for MoveDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let noted = self.check.note().is_some().then_some(&self.file_name);
        focus_the_note(&self.primary, noted, window, cx);
        let (danger, muted) = (cx.theme().danger, cx.theme().muted_foreground);
        let failure = self.failure.as_ref().map(|failure| {
            let (words, code) = failure.words();
            v_flex()
                .gap_1()
                .child(text("move-failure", words).text_sm().text_color(danger))
                .children(
                    code.map(|code| text("move-failure-code", code).text_xs().text_color(muted)),
                )
        });
        let entity = cx.entity().downgrade();
        let cancel = button("move-cancel", CANCEL, false, !self.running, None, {
            let entity = entity.clone();
            move |_, window, cx| {
                let _ = entity.update(cx, |dialog, cx| dialog.cancel(window, cx));
            }
        });
        let move_button = held_button(
            "move-ok",
            MOVE_ACTION,
            self.offered(),
            self.reason(),
            (&self.primary, window, cx),
            move |_, window, cx| {
                let _ = entity.update(cx, |dialog, cx| {
                    let fields = [&dialog.folder, &dialog.file_name];
                    keep_focus_in(&dialog.focus, &dialog.primary, &fields, window, cx);
                    dialog.move_document(cx)
                });
            },
        );
        v_flex()
            .id("move-dialog")
            .test_support()
            .track_focus(&self.focus)
            .gap_3()
            .child(field("move-folder", FOLDER, &self.folder, None, cx))
            .child(field(
                "move-file",
                FILE_NAME,
                &self.file_name,
                self.check.note(),
                cx,
            ))
            .children(failure)
            .child(
                h_flex()
                    .gap_2()
                    .justify_end()
                    .child(cancel)
                    .child(move_button),
            )
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "move_document_tests.rs"]
mod tests;
