//! What can be done with a draft from its row on the Drafts page, and from
//! the restored draft section of its tab: Restore, Export…, Export raw…
//! and Discard….
//!
//! Export asks the platform's save dialog where, with a name from the
//! draft's title, then writes the draft's Markdown, or an unreadable
//! file's bytes as they are, through a temporary file in the chosen
//! folder and a rename: a failed export never truncates a file already
//! there (ruling R11). A failure reads a fixed sentence, with no path and
//! no text from the system. Discard asks first in a dialog that names the
//! draft, its own surface; Escape or Cancel discards nothing, and the
//! focus goes back to the Discard that opened it.
//!
//! Neither dialog opens while quitting waits for drafts or a job, with
//! Quit now in view, nor over another dialog. Restore leaves the page
//! through its host, which opens the item's tab (ruling R25); the page
//! never hides itself.

use std::{
    future::Future,
    io::Write,
    path::{Path, PathBuf},
    rc::Rc,
};

use gpui_kit::{
    App, ClickEvent, Context, FocusHandle, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Task, TestSupportExt, Window,
    component::{
        WindowExt,
        button::{Button, ButtonVariants},
        h_flex, v_flex,
    },
    div,
};
use manyhands::{
    canonical::ItemId,
    drafts::{Draft, DraftError, DraftErrorKind, DraftFields, DraftId, DraftTarget, ItemKind},
    repository::ItemDtoKind,
};

use super::{ACTIONS, DraftsEvent, DraftsPage, Slot};
use crate::desktop::{
    model::{
        navigation::{ItemLocator, OpenTarget},
        tabs::TabKey,
    },
    services::Services,
    view::{
        confirm::CANCEL,
        item::{NOT_DISCARDED, refused_label},
    },
};

pub(crate) const EXPORT: &str = "Export…";
pub(crate) const EXPORT_RAW: &str = "Export raw…";
pub(crate) const EXPORTED: &str = "Draft exported";
pub(crate) const NOT_EXPORTED: &str = "Draft not exported";
pub(crate) const DISCARD_TITLE: &str = "Discard a draft";
pub(crate) const DISCARD_CONFIRM: &str = "Discard";
/// A Restore met by another process's claim: perhaps only a passing probe
/// of its lock, so it may be tried again.
pub(crate) const OPEN_ELSEWHERE_RETRY: &str =
    "The draft was open in another Manyhands process just now. Try Restore again.";

/// What an export came to.
pub(crate) enum Exported {
    Written,
    /// No place was chosen.
    Cancelled,
    /// The draft's bytes could not be read, for this reason.
    NotRead(DraftErrorKind),
    /// The chosen file could not be written, for this reason.
    NotWritten(DraftErrorKind),
}

impl Exported {
    /// What the export came to, in words; `None` when cancelled.
    pub(crate) fn label(&self) -> Option<String> {
        match self {
            Self::Written => Some(EXPORTED.to_owned()),
            Self::Cancelled => None,
            Self::NotRead(kind) => Some(format!("{NOT_EXPORTED}: {}", DraftError { kind: *kind })),
            Self::NotWritten(kind) => Some(export_failed(*kind)),
        }
    }
}

/// Why the chosen file could not be written, as a fixed sentence.
pub(crate) fn export_failed(kind: DraftErrorKind) -> String {
    let reason = match kind {
        DraftErrorKind::Access => "that folder cannot be written",
        DraftErrorKind::NoSpace => "there is no space left there",
        DraftErrorKind::NotFound => "that folder no longer exists",
        _ => "the file could not be written",
    };
    format!("{NOT_EXPORTED}: {reason}.")
}

/// Writes `bytes` to `path` through a temporary file in its folder and a
/// rename, so a failure leaves a file already at `path` as it was. On
/// Unix the file is the owner's only; the user may widen it. Blocking
/// file I/O.
pub(crate) fn export_to(path: &Path, bytes: &[u8]) -> Result<(), DraftErrorKind> {
    let kind = |error: std::io::Error| DraftError::from(error).kind;
    let folder = match path.parent() {
        Some(folder) if !folder.as_os_str().is_empty() => folder,
        _ => Path::new("."),
    };
    let mut file = tempfile::Builder::new()
        .prefix(".manyhands-export-")
        .tempfile_in(folder)
        .map_err(kind)?;
    file.write_all(bytes).map_err(kind)?;
    file.as_file().sync_all().map_err(kind)?;
    file.persist(path).map_err(|failed| kind(failed.error))?;
    Ok(())
}

/// The name the save dialog suggests for `draft`: its title, kept to
/// letters, digits, spaces, dashes and underscores, as Markdown.
pub(crate) fn suggested_name(draft: &Draft) -> String {
    let title = match &draft.current {
        DraftFields::Document { title, .. } | DraftFields::Ticket { title, .. } => title.as_str(),
        DraftFields::Comment { .. } => "comment",
    };
    let name: String = title
        .chars()
        .map(|c| match c {
            c if c.is_alphanumeric() || matches!(c, ' ' | '-' | '_') => c,
            _ => '-',
        })
        .take(80)
        .collect();
    match name.trim() {
        "" => "draft.md".to_owned(),
        name => format!("{name}.md"),
    }
}

/// The name the save dialog suggests for an unreadable draft's bytes.
pub(crate) fn raw_name(id: DraftId) -> String {
    format!("draft-{id}.json")
}

/// Where the save dialog opens: the user's documents folder, else home.
fn export_folder() -> PathBuf {
    directories::UserDirs::new()
        .map(|dirs| dirs.document_dir().unwrap_or(dirs.home_dir()).to_owned())
        .unwrap_or_default()
}

/// Asks where to export, suggesting `name`, then writes `bytes` there off
/// the foreground.
pub(crate) fn export(
    name: String,
    bytes: impl Future<Output = Result<Vec<u8>, DraftErrorKind>> + 'static,
    cx: &mut App,
) -> Task<Exported> {
    let chosen = cx.prompt_for_new_path(&export_folder(), Some(&name));
    let background = cx.background_executor().clone();
    cx.spawn(async move |_| {
        let path = match chosen.await {
            Ok(Ok(Some(path))) => path,
            // The platform's dialog failed: nothing was written.
            Ok(Err(_)) => return Exported::NotWritten(DraftErrorKind::Io),
            Ok(Ok(None)) | Err(_) => return Exported::Cancelled,
        };
        let bytes = match bytes.await {
            Ok(bytes) => bytes,
            Err(kind) => return Exported::NotRead(kind),
        };
        match background
            .spawn(async move { export_to(&path, &bytes) })
            .await
        {
            Ok(()) => Exported::Written,
            Err(kind) => Exported::NotWritten(kind),
        }
    })
}

/// Whether a dialog may open now: not while quitting waits for drafts or
/// a job, with Quit now in view, and not over another dialog.
pub(crate) fn may_open(services: &Services, window: &mut Window, cx: &mut App) -> bool {
    !(services.worker().quitting()
        || services.operations().read(cx).saving_drafts()
        || window.has_active_dialog(cx))
}

/// The Discard dialog's message, naming the draft; `None` for a draft that
/// cannot be read.
pub(crate) fn discard_message(name: Option<&str>) -> String {
    match name {
        Some(name) => {
            format!("Discard the draft “{name}”? Its text is deleted and cannot be recovered.")
        }
        None => "Discard this draft, which cannot be read? Its file is deleted and cannot be \
                 recovered."
            .to_owned(),
    }
}

type Confirm = Rc<dyn Fn(&mut Window, &mut App)>;

/// Opens the Discard dialog naming `name`, unless a dialog may not open
/// now. Confirm, by click or Enter, calls `confirm`; Escape or Cancel
/// discards nothing. Either way the focus goes back to `origin`.
pub(crate) fn open_discard(
    name: Option<String>,
    origin: FocusHandle,
    services: &Services,
    window: &mut Window,
    cx: &mut App,
    confirm: impl Fn(&mut Window, &mut App) + 'static,
) {
    if !may_open(services, window, cx) {
        return;
    }
    let confirm: Confirm = Rc::new(confirm);
    let message = discard_message(name.as_deref());
    window.open_dialog(cx, move |base, _, _| {
        let (enter, close) = (confirm.clone(), origin.clone());
        base.title(DISCARD_TITLE)
            .close_button(false)
            .overlay_closable(false)
            .child(discard_body(&message, &confirm))
            // Enter: Discard; the Kit closes the dialog.
            .on_ok(move |_, window, cx| {
                enter(window, cx);
                true
            })
            // Escape: close with nothing discarded.
            .on_cancel(|_, _, _| true)
            .on_close(move |_, window, cx| window.focus(&close, cx))
    });
}

fn dialog_button(
    id: &'static str,
    label: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let mut button = Button::new(format!("{id}-button")).label(label);
    if id == "discard-draft-confirm" {
        button = button.danger();
    }
    div()
        .id(id)
        .test_support()
        .aria_label(label)
        .child(button.on_click(on_click))
}

fn discard_body(message: &str, confirm: &Confirm) -> impl IntoElement + use<> {
    let confirm = confirm.clone();
    v_flex()
        .id("discard-draft-dialog")
        .test_support()
        .gap_2()
        .child(
            div()
                .id("discard-draft-message")
                .test_support()
                .aria_label(message.to_owned())
                .text_sm()
                .child(message.to_owned()),
        )
        .child(
            h_flex()
                .gap_2()
                .child(dialog_button(
                    "discard-draft-confirm",
                    DISCARD_CONFIRM,
                    move |_, window, cx| {
                        window.close_dialog(cx);
                        confirm(window, cx);
                    },
                ))
                .child(dialog_button(
                    "discard-draft-cancel",
                    CANCEL,
                    |_, window, cx| window.close_dialog(cx),
                )),
        )
}

/// The desktop's item kind of a draft's.
fn dto_kind(kind: ItemKind) -> ItemDtoKind {
    match kind {
        ItemKind::Document => ItemDtoKind::Document,
        ItemKind::Ticket => ItemDtoKind::Ticket,
    }
}

/// The tab a draft of `target` is restored in, opened through its item
/// locator (ruling R25).
fn open_target(target: &DraftTarget) -> OpenTarget {
    let id: ItemId = target.item_id.clone();
    ItemLocator::Identified {
        kind: dto_kind(target.item_kind),
        id,
    }
    .in_repository(&target.repository)
}

/// The position of the action `part` in `ACTIONS`.
pub(super) fn action_ix(part: &str) -> usize {
    ACTIONS
        .iter()
        .position(|(name, _)| *name == part)
        .expect("a row action")
}

impl DraftsPage {
    /// Restore: claims the draft and attaches it to its item's tab, which
    /// the host then opens and focuses. A refusal is said on the row.
    pub(super) fn restore(&mut self, id: DraftId, cx: &mut Context<Self>) {
        let Some(draft) = self.slot(id).and_then(|slot| slot.draft.as_ref()) else {
            return;
        };
        let target = open_target(&draft.target);
        if !self.restoring.insert(id) {
            return;
        }
        let tab = TabKey::for_target(&target);
        self.notices.remove(&id);
        let drafts = self.services.drafts().clone();
        let restore = drafts.update(cx, |registry, cx| registry.restore(id, tab, cx));
        cx.spawn(async move |page, cx| {
            let restored = restore.await;
            let shown = page.update(cx, |page, cx| {
                page.restoring.remove(&id);
                match restored {
                    Ok(_) => cx.emit(DraftsEvent::Open(target)),
                    Err(refused) => {
                        page.notices.insert(id, refused_label(refused));
                    }
                }
                cx.notify();
            });
            // The page went while the claim was taken: nothing opens the
            // tab, so the draft is let go of and offered again.
            if let (Err(_), Ok(restored)) = (shown, restored) {
                drafts.update(cx, |registry, cx| {
                    registry.release_when_stored(restored, cx)
                });
            }
        })
        .detach();
    }

    /// Export… or Export raw…: a draft restored here exports its newest
    /// text; another readable one as listed; an unreadable one its bytes.
    pub(super) fn export(&mut self, id: DraftId, window: &mut Window, cx: &mut Context<Self>) {
        if !may_open(&self.services, window, cx) {
            return;
        }
        let drafts = self.services.drafts().clone();
        let (name, bytes) = match self.slot(id) {
            Some(Slot {
                draft: Some(draft), ..
            }) => {
                let draft = drafts
                    .read(cx)
                    .session(id)
                    .map(|session| session.draft())
                    .unwrap_or_else(|| draft.clone());
                let bytes = manyhands::drafts::markdown(&draft).into_bytes();
                (suggested_name(&draft), Task::ready(Ok(bytes)))
            }
            Some(_) => (
                raw_name(id),
                drafts.update(cx, |registry, cx| registry.raw_bytes(id, cx)),
            ),
            None => return,
        };
        let export = export(name, bytes, cx);
        cx.spawn(async move |page, cx| {
            let exported = export.await;
            let _ = page.update(cx, |page, cx| {
                if let Some(label) = exported.label() {
                    page.notices.insert(id, label);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Discard…: asks first, naming the draft by its title, a comment's
    /// by its kind.
    pub(super) fn discard(&mut self, id: DraftId, window: &mut Window, cx: &mut Context<Self>) {
        let (Some(slot), Some(focus)) = (self.slot(id), self.focus.get(&id)) else {
            return;
        };
        let readable = slot.draft.is_some();
        let name = match (readable, slot.row.title.is_empty()) {
            (false, _) => None,
            (true, false) => Some(slot.row.title.clone()),
            (true, true) => Some(slot.row.kind_label.clone()),
        };
        let origin = focus.actions[action_ix("discard")].clone();
        let page = cx.entity().downgrade();
        open_discard(name, origin, &self.services, window, cx, move |_, cx| {
            let _ = page.update(cx, |page, cx| page.discard_confirmed(id, readable, cx));
        });
    }

    /// Discards the draft `id`: through its session when restored here,
    /// else from the store. A failure is said on the row; the draft is
    /// then released, still stored, and listed again.
    fn discard_confirmed(&mut self, id: DraftId, readable: bool, cx: &mut Context<Self>) {
        let drafts = self.services.drafts().clone();
        let held_here = drafts.read(cx).session(id).is_some();
        let discard = drafts.update(cx, |registry, cx| match held_here {
            true => registry.discard(id, cx),
            false => registry.discard_stored(id, readable, cx),
        });
        self.notices.remove(&id);
        cx.spawn(async move |page, cx| {
            if let Err(kind) = discard.await {
                let _ = page.update(cx, |page, cx| {
                    let reason = DraftError { kind };
                    page.notices
                        .insert(id, format!("{NOT_DISCARDED}: {reason}"));
                    cx.notify();
                });
            }
        })
        .detach();
    }
}
