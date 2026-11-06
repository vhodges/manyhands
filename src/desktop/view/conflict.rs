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
    App, AppContext, Context, Entity, EntityId, FocusHandle, ParentElement, Subscription, Task,
    Window, component::WindowExt, px,
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
/// A local synchronization's, naming the primary branch `primary` (R-35).
pub(crate) fn note_local(primary: &str) -> String {
    format!("Saving finishes bringing {primary} into this item.")
}
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
/// A local synchronization's, naming the primary branch `primary`.
pub(crate) fn description_local(primary: &str) -> String {
    format!(
        "This item and {primary} both changed since they last met. Pick a version for each \
         part, or keep both."
    )
}
/// The Result's note (handoff 2g). What Saving does is the source note's
/// alone (decision 15; owner ruling R-34).
pub(crate) const RESULT_NOTE: &str = "You can still edit it in the item afterwards.";
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
    /// What Saving does (decision 15); `primary` names the primary branch.
    fn note(self, primary: &str) -> String {
        match self {
            Self::Share | Self::Post => NOTE_SHARE.to_owned(),
            Self::Local => note_local(primary),
            Self::Close(_) => NOTE_CLOSE.to_owned(),
            Self::Merge(_) => NOTE_MERGE.to_owned(),
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

/// Opens the sheet. `None` while quitting, or while drafts are being saved
/// for it: nothing opens in front of Quit now.
pub(crate) fn open(
    opening: Opening,
    services: &Services,
    window: &mut Window,
    cx: &mut App,
) -> Option<Entity<ResolveSheet>> {
    if services.worker().quitting() || services.operations().read(cx).saving_drafts() {
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
    /// The part's path and three sides, as `ConflictModel::keep_choices`
    /// matches them: a read that gives the same ones keeps this editor,
    /// with its focus and history. The path's token is not part of it, as
    /// it changes with every observation.
    part: (String, [String; 3]),
    result: Entity<EditorState>,
    /// The Result's tab stop, after the part's choices: Enter on it puts
    /// focus in the editor, and Escape there comes back to it (R-31).
    stop: FocusHandle,
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
    /// The sheet's own focus: a control that goes or is disabled while it
    /// holds focus (Save resolution while its request waits, Read again
    /// once its read lands) gives focus back here, the nearest focusable
    /// that held it (`Workspace::focus_lost`), so it stays in the sheet
    /// (A21).
    focus: FocusHandle,
    services: Services,
    generations: Generations,
    read: Option<Task<()>>,
    /// The lookup of a landed read's kept set (`kept_landed`).
    kept_lookup: Option<Task<()>>,
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
    /// The lifecycle children's resolutions this sheet registered: what
    /// the drain does once one is applied is kept for it until it closes.
    watching: Vec<RequestId>,
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
        let subscriptions = vec![
            cx.observe_in(services.operations(), window, |this, _, window, cx| {
                this.operations_changed(window, cx)
            }),
            // Closed, it takes nothing more from the drain (Task 16, fix 3).
            cx.on_release(|sheet: &mut Self, _| {
                for request in sheet.watching.drain(..) {
                    sheet.services.followups().unwatch(request);
                }
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
            kept_lookup: None,
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
            watching: Vec::new(),
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
    /// A model read waits for its kept set, looked up off the UI thread
    /// (`kept_landed`): the sheet reads on until then, so Save waits too.
    fn read_landed(
        &mut self,
        stamp: Stamp,
        result: ReadResult<ConflictModel>,
        cx: &mut Context<Self>,
    ) {
        if !self.generations.is_current(&stamp) {
            return;
        }
        match result {
            ReadResult::Ok(model) => {
                let named = model
                    .attempt()
                    .and_then(|attempt| RequestId::parse(attempt).ok());
                let found = self.services.find_kept(
                    named,
                    self.root.clone(),
                    self.operation_id,
                    self.except(cx),
                    cx,
                );
                self.kept_lookup = Some(cx.spawn(async move |sheet, cx| {
                    let kept = found.await;
                    let _ = sheet.update(cx, |sheet, cx| sheet.kept_landed(stamp, model, kept, cx));
                }));
                return;
            }
            ReadResult::Busy => self.not_read = Some(ResultCode::Busy),
            ReadResult::Failed(failure) => self.not_read = Some(failure.code),
        }
        self.reading = false;
        cx.notify();
    }

    /// A read's model, with the kept set Retry sends (`find_kept`). A new
    /// model keeps the earlier one's choices where its parts are unchanged.
    fn kept_landed(
        &mut self,
        stamp: Stamp,
        mut model: ConflictModel,
        kept: Option<KeptSet>,
        cx: &mut Context<Self>,
    ) {
        if !self.generations.is_current(&stamp) {
            return;
        }
        self.reading = false;
        if let Some(earlier) = &self.model {
            model.keep_choices(earlier);
        }
        self.kept = kept;
        // A Settled read keeps what the read before it said.
        match model.set() {
            ConflictSet::External(_) => self.outside = true,
            ConflictSet::Canonical(_) => self.outside = false,
            ConflictSet::Settled => self.outside |= model.external_repair(),
        }
        self.model = Some(model);
        self.not_read = None;
        self.editors_stale = true;
        cx.notify();
    }

    fn choose_field(&mut self, path: usize, field: usize, choice: Choice, cx: &mut Context<Self>) {
        // Out with Save resolution, the choices are as they were sent.
        if self.saving() {
            return;
        }
        if let Some(model) = &mut self.model
            && let Some(path) = model.paths_mut().get_mut(path)
        {
            path.choose_field(field, choice);
        }
        cx.notify();
    }

    /// Keep mine, theirs or both: the Result editor shows it.
    fn choose_body(&mut self, path: usize, choice: BodyChoice, cx: &mut Context<Self>) {
        if self.saving() {
            return;
        }
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

    /// The Result editor `editor` changed: by typing, or by undo or redo,
    /// which emit no event. Its text is then the Result. A change while
    /// Save resolution is out is put back: the request has the text sent.
    fn result_edited(&mut self, editor: EntityId, cx: &mut Context<Self>) {
        let Some((path, result)) = self
            .editors
            .iter()
            .find(|editors| editors.result.entity_id() == editor)
            .map(|editors| (editors.path, editors.result.clone()))
        else {
            return;
        };
        let saving = self.saving();
        let Some(conflict) = self
            .model
            .as_mut()
            .and_then(|model| model.paths_mut().get_mut(path))
        else {
            return;
        };
        let Some(body) = conflict.body() else {
            return;
        };
        // Any change of the editor notifies, the caret's and focus too.
        let text = result.read(cx).text();
        if text == body.result {
            return;
        }
        if saving {
            let sent = body.result.clone();
            set_exact(&result, &sent, cx);
            return;
        }
        conflict.edit_body(text.to_owned());
        cx.notify();
    }

    /// Makes the body editors of the model just read. A part whose path
    /// and sides are as they were keeps its editor, and with it focus,
    /// caret and history (R-31): the model took its Result from the
    /// earlier read, which the editor holds.
    fn make_editors(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !std::mem::take(&mut self.editors_stale) {
            return;
        }
        let mut earlier = std::mem::take(&mut self.editors);
        let Some(ConflictSet::Canonical(paths)) = self.model.as_ref().map(ConflictModel::set)
        else {
            return;
        };
        let bodies: Vec<_> = paths
            .iter()
            .enumerate()
            .filter_map(|(index, path)| {
                let body = path.body()?;
                let part = (
                    path.display_path().to_owned(),
                    [body.base.clone(), body.mine.clone(), body.theirs.clone()],
                );
                Some((index, part, body.result.clone()))
            })
            .collect();
        for (path, part, result) in bodies {
            if let Some(at) = earlier.iter().position(|editors| editors.part == part) {
                let mut kept = earlier.swap_remove(at);
                kept.path = path;
                if kept.result.read(cx).text() != result {
                    set_exact(&kept.result, &result, cx);
                }
                self.editors.push(kept);
                continue;
            }
            let editor = crate::desktop::view::editor::open_editor(&result, window, cx);
            set_exact(&editor, &result, cx);
            // Observed, not subscribed to `Changed`: undo and redo only
            // notify.
            let changed = cx.observe(&editor, |sheet, editor, cx| {
                sheet.result_edited(editor.entity_id(), cx)
            });
            self.editors.push(BodyEditors {
                path,
                part,
                result: editor,
                stop: cx.focus_handle().tab_stop(true),
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
pub(crate) mod tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "conflict/answer_tests.rs"]
pub(crate) mod answer_tests;

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
pub(crate) mod entry_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "conflict/repair_tests.rs"]
mod repair_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "conflict/lifecycle_resume_tests.rs"]
mod lifecycle_resume_tests;
