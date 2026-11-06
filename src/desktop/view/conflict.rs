//! The resolve conflict sheet (D5 design, B3; handoff 2g): "Choose what to
//! keep", one card per part, and Save resolution once every part is chosen.
//! There is no automatic winner.
//!
//! - **Read** (B1). The conflict is read on the background executor,
//!   remote or local, stamped: a late read never replaces a newer one.
//! - **Save resolution** sends `ConflictResolve` or `LocalContextResolve`
//!   with the read's observation (R-11 for a local one) and every path's
//!   `{token, source}`. The set is kept first, per request ID
//!   (`Services::submit_resolution`; decision 19): one that cannot be kept
//!   is not sent, and the sheet says why. Ctrl+Enter saves; Enter in the
//!   Result editor is a newline.
//! - **Kept sets** (R-12, R-25). A kept set of the request the read names
//!   as the unfinished attempt's, or another of this conflict, is sent
//!   again under its own request ID: Retry, never a new request (P24). A
//!   Save that finds another window's set uses it instead, and says so when
//!   its choices differ. A local attempt whose set was lost cannot be
//!   finished here (K-10): its read says it is `historical`, and Save is
//!   not offered.
//! - **Answers** (B3's table). Applied closes the sheet; a lifecycle
//!   child's sends the lifecycle's request again when this session holds
//!   it and shows its run, or else resumes the lifecycle row the conflict
//!   was read by (R-10). `external_change` says "This changed while you
//!   were choosing." and reads again, keeping the choices of unchanged parts
//!   (P11). `original_request_required` offers Retry with that request's
//!   kept set. A stop after the checkpoint, or a read whose commit is
//!   prepared, offers Resume (P15). Any other answer is shown, and the
//!   conflict read again.
//!
//! - **Outside** (B4; `conflict/external.rs`). A set that goes outside
//!   (R-6, R-26) shows each path's card with its reason, Open folder and
//!   Check again, and no Save resolution and no winner. Check again reads
//!   again, and offers Resume only when the library's `next_action` does.
//!
//! A resolution is never given up (R-24): the sheet offers no Give up.
//! The sheet holds item text, and derives no `Debug`.

mod entry;
mod external;
mod render;
mod submit;

pub(crate) use entry::Entry;

use std::path::PathBuf;

use gpui_kit::{
    App, AppContext, Context, Entity, FocusHandle, ParentElement, Subscription, Task, Window,
    component::WindowExt, px,
};
use manyhands::{
    canonical::ItemId,
    repository::{OperationId, RequestId},
    results::ResultCode,
};
use zorite_editor::EditorState;

use crate::desktop::{
    model::{
        conflict::{BodyChoice, Choice, ConflictModel, ConflictSet},
        stamp::{Generations, Stamp, ViewScope},
    },
    services::{
        ReadResult, Services, conflicts::ConflictSource, requests::DesktopRequestId,
        resolutions::KeptSet,
    },
    view::{
        editor::set_exact,
        lifecycle::{Heading, Reopen},
    },
};

pub(crate) const SAVE_RESOLUTION: &str = "Save resolution";
/// The sheet's entry points (B5): the conflict banner's, the operation
/// area's and a pending row's.
pub(crate) const REVIEW_CHANGES: &str = "Review changes";
pub(crate) const RETRY: &str = "Retry";
pub(crate) const RESUME: &str = "Resume";
pub(crate) const CANCEL: &str = "Cancel";
pub(crate) const READ_AGAIN: &str = "Read again";
pub(crate) const READING: &str = "Reading the conflict\u{2026}";
pub(crate) const NOT_READ: &str = "The conflict could not be read.";
/// The note under the title, by where the conflict stopped (decision 15).
pub(crate) const NOTE_SHARE: &str = "Saving finishes the share that stopped.";
pub(crate) const NOTE_LOCAL: &str = "Saving finishes bringing main into this item.";
pub(crate) const NOTE_CLOSE: &str = "Saving lets the close continue.";
pub(crate) const NOTE_MERGE: &str = "Saving lets the merge continue.";
/// `external_change`: the comparison was read again (P11).
pub(crate) const CHANGED: &str = "This changed while you were choosing.";
/// A Save found another window's set of this conflict (R-12).
pub(crate) const EARLIER_USED: &str =
    "An earlier resolution is still being saved; it is used instead.";
/// A kept set's request could not be queued again (review M-c).
pub(crate) const NOT_SENT_AGAIN: &str = "The earlier resolution could not be sent again.";
/// A part's choice, or that it has none yet.
pub(crate) const CHOOSE_ONE: &str = "Choose one";
pub(crate) const KEEPING_MINE: &str = "Keeping yours";
pub(crate) const KEEPING_THEIRS: &str = "Keeping theirs";
pub(crate) const KEEPING_BOTH: &str = "Keeping both";
pub(crate) const KEEPING_EDITED: &str = "Keeping the result as edited";
/// A part only one side changed is taken from it (R-5).
pub(crate) const TAKEN_THEIRS: &str = "Only theirs changed it; theirs is kept.";
pub(crate) const TAKEN_MINE: &str = "Only yours changed it; yours is kept.";
/// The synchronization's resolution is saved; Resume finishes it (P15).
pub(crate) const RESUME_LEAD: &str =
    "The resolution is saved, and what is left of it runs again with Resume.";
/// Nothing is left to choose.
pub(crate) const SETTLED: &str = "Nothing is left to choose here.";
/// The description under the title (handoff 2g), remote and local.
pub(crate) const DESCRIPTION_REMOTE: &str = "You and someone else both changed this since it \
     was last shared. Pick a version for each part, or keep both.";
pub(crate) const DESCRIPTION_LOCAL: &str = "This item and main both changed since they last \
     met. Pick a version for each part, or keep both.";
/// The Result's note (handoff 2g).
pub(crate) const RESULT_NOTE: &str = "Saving keeps the result on this computer. You can \
     still edit it in the item afterwards, and Share sends it to everyone.";
/// K-10: a partial local resolution whose kept set is not here. A read is
/// also historical while another process's attempt is still running, so
/// the words fit that too.
pub(crate) const LOST: &str = "This resolution was partly saved, and the choices it was saved \
     with are not kept here. If it is still being resolved elsewhere, let that finish; \
     otherwise abandon the synchronization to start again.";

/// Where the conflict stopped: what the sheet says Saving does, and what
/// follows an applied resolution.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SheetSource {
    /// A share's synchronization.
    Share,
    /// A comment post's.
    Post,
    /// A local synchronization with main.
    Local,
    /// A close's child synchronization, with the close's request when this
    /// session holds it.
    Close(Option<DesktopRequestId>),
    /// A merge's, likewise.
    Merge(Option<DesktopRequestId>),
}

impl SheetSource {
    fn note(self) -> &'static str {
        match self {
            Self::Share | Self::Post => NOTE_SHARE,
            Self::Local => NOTE_LOCAL,
            Self::Close(_) => NOTE_CLOSE,
            Self::Merge(_) => NOTE_MERGE,
        }
    }

    fn conflict(self) -> ConflictSource {
        match self {
            Self::Local => ConflictSource::Local,
            Self::Share | Self::Post | Self::Close(_) | Self::Merge(_) => ConflictSource::Remote,
        }
    }
}

/// What opening the sheet takes.
pub(crate) struct Opening {
    pub source: SheetSource,
    pub root: PathBuf,
    /// The synchronization's operation: a lifecycle child's, or after a
    /// restart the lifecycle row's, which the conflict verbs accept (B1).
    pub operation_id: OperationId,
    pub item: ItemId,
    /// Who the sheet speaks of: the item's kind, short code, title and
    /// repository for its meta line, and the main branch and remote a
    /// reopened lifecycle dialog shows (review M-2).
    pub heading: Heading,
    /// The item's work folder, where a set that goes outside is repaired:
    /// Open folder opens it (B4). With none named, the sheet reads the
    /// item's (Task 13), and opens the repository's until that read lands
    /// or when it finds none.
    pub folder: Option<PathBuf>,
    /// Where focus goes back when the sheet closes.
    pub origin: FocusHandle,
}

/// Opens the sheet. `None` while quitting.
pub(crate) fn open(
    opening: Opening,
    services: &Services,
    window: &mut Window,
    cx: &mut App,
) -> Option<Entity<ResolveSheet>> {
    if services.worker().quitting() {
        return None;
    }
    let origin = opening.origin.clone();
    let sheet = cx.new(|cx| ResolveSheet::new(opening, services.clone(), window, cx));
    let opened = sheet.clone();
    window.open_dialog(cx, move |base, _, _| {
        let origin = origin.clone();
        base.w(px(920.))
            .close_button(false)
            .overlay_closable(false)
            .child(sheet.clone())
            // Enter is the Result editor's newline, or a focused button's.
            .on_ok(|_, _, _| false)
            .on_cancel(|_, _, _| true)
            .on_close(move |_, window, cx| window.focus(&origin, cx))
    });
    #[cfg(all(test, feature = "desktop-test-support"))]
    cx.set_global(OpenedSheet(opened.downgrade()));
    Some(opened)
}

/// The sheet last opened, for a test to read what it holds.
#[cfg(all(test, feature = "desktop-test-support"))]
pub(crate) struct OpenedSheet(pub(crate) gpui_kit::WeakEntity<ResolveSheet>);

#[cfg(all(test, feature = "desktop-test-support"))]
impl gpui_kit::Global for OpenedSheet {}

/// A canonical path's Result editor. Before, Yours and Theirs are drawn
/// as mono lines, each with its tint and gutter (handoff 2g).
struct BodyEditors {
    path: usize,
    result: Entity<EditorState>,
    _changed: Subscription,
}

pub(crate) struct ResolveSheet {
    source: SheetSource,
    root: PathBuf,
    operation_id: OperationId,
    item: ItemId,
    heading: Heading,
    /// The folder Open folder opens (B4).
    folder: PathBuf,
    /// The read of the item's work folder, when the opening named none.
    folder_read: Option<Task<()>>,
    /// The lifecycle dialog whose Repair locally… led here, opened again
    /// once a local resolution applies (B6, P9).
    reopen: Option<Reopen>,
    origin: FocusHandle,
    /// The sheet's own focus: Save resolution or Retry, pressed, is
    /// disabled while its request waits, and focus moves here first, so
    /// it stays in the sheet (A21).
    focus: FocusHandle,
    services: Services,
    generations: Generations,
    read: Option<Task<()>>,
    reading: bool,
    model: Option<ConflictModel>,
    /// The last read failed, with this code.
    not_read: Option<ResultCode>,
    /// A kept, unfinished set of this conflict: Retry sends it.
    kept: Option<KeptSet>,
    /// Requests this sheet saw answered: a Save does not take up their
    /// sets as another window's.
    spent: Vec<RequestId>,
    notice: Option<&'static str>,
    /// Why a set could not be kept, or a request built.
    error: Option<String>,
    /// The last answer that the sheet shows, by code and message.
    answer: Option<(ResultCode, String)>,
    /// Resume is offered: the resolution stopped after its checkpoint.
    stopped: bool,
    /// A read of this sheet sent the set outside: a later read that says
    /// Resume speaks of the repair (B4).
    outside: bool,
    /// A set is being kept, before its request is sent (review I-1).
    submitting: bool,
    /// The resolve request the sheet follows, and the first attempt of it
    /// whose answer it has not yet taken.
    following: Option<(DesktopRequestId, u32)>,
    /// The body editors, made as the next frame is drawn after a read.
    editors: Vec<BodyEditors>,
    editors_stale: bool,
    _subscriptions: Vec<Subscription>,
}

impl ResolveSheet {
    fn new(
        opening: Opening,
        services: Services,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let subscriptions =
            vec![
                cx.observe_in(services.operations(), window, |this, _, window, cx| {
                    this.operations_changed(window, cx)
                }),
            ];
        let named = opening.folder.is_some();
        let folder = opening.folder.unwrap_or_else(|| opening.root.clone());
        let mut sheet = Self {
            source: opening.source,
            root: opening.root,
            operation_id: opening.operation_id,
            item: opening.item,
            heading: opening.heading,
            folder,
            folder_read: None,
            reopen: None,
            origin: opening.origin,
            focus: cx.focus_handle(),
            services,
            generations: Generations::default(),
            read: None,
            reading: false,
            model: None,
            not_read: None,
            kept: None,
            spent: Vec::new(),
            notice: None,
            error: None,
            answer: None,
            stopped: false,
            outside: false,
            submitting: false,
            following: None,
            editors: Vec::new(),
            editors_stale: false,
            _subscriptions: subscriptions,
        };
        // M-4: a store that did not open says so at once; nothing can be
        // kept, so nothing is sent.
        if let Err(error) = sheet.services.resolutions() {
            sheet.error = Some(error.to_string());
        }
        if !named {
            sheet.read_folder(cx);
        }
        sheet.read_conflict(cx);
        sheet
    }

    /// Once a local resolution applies, the lifecycle dialog opens again on
    /// `reopen` (P9).
    pub(crate) fn reopen_after(&mut self, reopen: Reopen) {
        self.reopen = Some(reopen);
    }

    /// Reads the item's work folder, for Open folder (B4): an entry point
    /// that does not know it still opens the sheet on the item's work
    /// context, never the repository's while the item has one.
    fn read_folder(&mut self, cx: &mut Context<Self>) {
        let stamp = self.generations.next(ViewScope::ConflictFolder);
        self.folder_read = Some(self.services.read_item_folder(
            self.root.clone(),
            self.item.clone(),
            stamp,
            cx,
            |sheet, stamp, result, cx| {
                if !sheet.generations.is_current(&stamp) {
                    return;
                }
                if let ReadResult::Ok(Some(folder)) = result {
                    sheet.folder = folder;
                    cx.notify();
                }
            },
        ));
    }

    /// Reads the conflict, stamped (B1).
    fn read_conflict(&mut self, cx: &mut Context<Self>) {
        let stamp = self.generations.next(ViewScope::Conflict);
        self.reading = true;
        self.read = Some(self.services.read_conflict(
            self.source.conflict(),
            self.root.clone(),
            self.operation_id,
            stamp,
            cx,
            |sheet, stamp, result, cx| sheet.read_landed(stamp, result, cx),
        ));
        cx.notify();
    }

    /// A read's answer. A late one, asked before a newer read, is dropped.
    /// A new model keeps the earlier one's choices where its parts are
    /// unchanged.
    fn read_landed(
        &mut self,
        stamp: Stamp,
        result: ReadResult<ConflictModel>,
        cx: &mut Context<Self>,
    ) {
        if !self.generations.is_current(&stamp) {
            return;
        }
        self.reading = false;
        match result {
            ReadResult::Ok(mut model) => {
                if let Some(earlier) = &self.model {
                    model.keep_choices(earlier);
                }
                self.kept = self.kept_of(&model, cx);
                // A Settled read keeps what the read before it said.
                match model.set() {
                    ConflictSet::External(_) => self.outside = true,
                    ConflictSet::Canonical(_) => self.outside = false,
                    ConflictSet::Settled => self.outside |= model.external_repair(),
                }
                self.model = Some(model);
                self.not_read = None;
                self.editors_stale = true;
            }
            ReadResult::Busy => self.not_read = Some(ResultCode::Busy),
            ReadResult::Failed(failure) => self.not_read = Some(failure.code),
        }
        cx.notify();
    }

    fn choose_field(&mut self, path: usize, field: usize, choice: Choice, cx: &mut Context<Self>) {
        if let Some(model) = &mut self.model
            && let Some(path) = model.paths_mut().get_mut(path)
        {
            path.choose_field(field, choice);
        }
        cx.notify();
    }

    /// Keep mine, theirs or both: the Result editor shows it.
    fn choose_body(&mut self, path: usize, choice: BodyChoice, cx: &mut Context<Self>) {
        let Some(result) = self.model.as_mut().and_then(|model| {
            let conflict = model.paths_mut().get_mut(path)?;
            conflict.choose_body(choice);
            Some(conflict.body()?.result.clone())
        }) else {
            return;
        };
        if let Some(editors) = self.editors.iter().find(|editors| editors.path == path) {
            set_exact(&editors.result, &result, cx);
        }
        cx.notify();
    }

    /// The Result editor of `path` changed by the user's hand.
    fn result_edited(&mut self, path: usize, cx: &mut Context<Self>) {
        let Some(editors) = self.editors.iter().find(|editors| editors.path == path) else {
            return;
        };
        let text = editors.result.read(cx).text().to_owned();
        if let Some(conflict) = self
            .model
            .as_mut()
            .and_then(|model| model.paths_mut().get_mut(path))
        {
            conflict.edit_body(text);
        }
        cx.notify();
    }

    /// Makes the body editors of the model just read.
    fn make_editors(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !std::mem::take(&mut self.editors_stale) {
            return;
        }
        self.editors.clear();
        let Some(ConflictSet::Canonical(paths)) = self.model.as_ref().map(ConflictModel::set)
        else {
            return;
        };
        let bodies: Vec<_> = paths
            .iter()
            .enumerate()
            .filter_map(|(index, path)| Some((index, path.body()?.result.clone())))
            .collect();
        for (path, result) in bodies {
            let editor = crate::desktop::view::editor::open_editor(&result, window, cx);
            set_exact(&editor, &result, cx);
            let changed = cx.subscribe(&editor, move |sheet, _, event, cx| {
                if matches!(event, zorite_editor::EditorEvent::Changed) {
                    sheet.result_edited(path, cx);
                }
            });
            self.editors.push(BodyEditors {
                path,
                result: editor,
                _changed: changed,
            });
        }
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
impl ResolveSheet {
    /// The Result editor of the path `path`, once drawn.
    pub(crate) fn result_editor(&self, path: usize) -> Option<Entity<EditorState>> {
        self.editors
            .iter()
            .find(|editors| editors.path == path)
            .map(|editors| editors.result.clone())
    }

    pub(crate) fn source(&self) -> SheetSource {
        self.source
    }

    pub(crate) fn operation(&self) -> OperationId {
        self.operation_id
    }

    /// The folder Open folder opens.
    pub(crate) fn folder(&self) -> &std::path::Path {
        &self.folder
    }

    /// The observation of the conflict as last read.
    pub(crate) fn observation(&self) -> Option<String> {
        self.model
            .as_ref()
            .and_then(ConflictModel::observation)
            .map(str::to_owned)
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "conflict_tests.rs"]
mod tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "conflict/answer_tests.rs"]
mod answer_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "conflict/sheet_tests.rs"]
mod sheet_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "conflict/handoff_tests.rs"]
mod handoff_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "conflict/review_tests.rs"]
mod review_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "conflict/stale_tests.rs"]
mod stale_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "conflict/followup_tests.rs"]
mod followup_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "conflict/external_tests.rs"]
mod external_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "conflict/entry_tests.rs"]
mod entry_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "conflict/repair_tests.rs"]
mod repair_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "conflict/lifecycle_resume_tests.rs"]
mod lifecycle_resume_tests;
