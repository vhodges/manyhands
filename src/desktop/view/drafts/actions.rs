//! What can be done with a draft both from its row on the Drafts page and
//! from the restored draft section of its tab: Export…, Export raw… and
//! Discard…. The page's own use of them is `row_actions`.
//!
//! Export asks the platform's save dialog where, with a name from the
//! draft's title, then writes the draft's Markdown, or an unreadable
//! file's bytes as they are, through a temporary file in the chosen
//! folder and a rename, and syncs the folder: a failed export never
//! truncates a file already there (ruling R11). A failure reads a fixed
//! sentence, with no path and no text from the system. Discard asks first
//! in a dialog that names the draft, its own surface. Its default is
//! Cancel: Enter, Escape and Cancel discard nothing, and the focus goes
//! back to the Discard that opened it; only Confirm discards.
//!
//! Neither dialog opens while quitting waits for drafts or a job, with
//! Quit now in view, nor over another dialog.

use std::{
    future::Future,
    io::Write,
    path::{Path, PathBuf},
    rc::Rc,
};

use gpui_kit::{
    App, ClickEvent, FocusHandle, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Task, TestSupportExt, Window,
    component::{
        WindowExt,
        button::{Button, ButtonVariants},
        dialog::Confirm as DialogConfirm,
        h_flex, v_flex,
    },
    div,
};
use manyhands::drafts::{Draft, DraftError, DraftErrorKind, DraftFields, DraftId};

use crate::desktop::{
    model::drafts::{DraftRow, RowState},
    services::Services,
    view::confirm::CANCEL,
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
/// rename, so a failure leaves a file already at `path` as it was, then
/// syncs the folder so the rename lasts, as the store does. On
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
    // The file is written and in place: a folder that cannot be synced
    // fails nothing.
    #[cfg(unix)]
    let _ = std::fs::File::open(folder).and_then(|folder| folder.sync_all());
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

/// What the Discard dialog calls a draft, as its row shows it: its own
/// title, else its kind ("Ticket", "Comment on <item title>"); `None` for
/// one that cannot be read. The page and the tab name drafts alike.
pub(crate) fn draft_name(row: &DraftRow) -> Option<String> {
    match row.state {
        RowState::Unreadable(_) => None,
        _ if !row.title.is_empty() => Some(row.title.clone()),
        _ => Some(row.kind_label.clone()),
    }
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
/// now. Only Confirm, by click (or Enter on the focused button), calls
/// `confirm`; the default is Cancel, so Enter, Escape and Cancel
/// discard nothing (as Keep draft and quit's Cancel are the defaults of
/// D2's other dialogs). Either way the focus goes back to `origin`.
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
        let close = origin.clone();
        base.title(DISCARD_TITLE)
            .close_button(false)
            .overlay_closable(false)
            .child(discard_body(&message, &confirm))
            // Enter: Cancel, the default; the Kit closes the dialog.
            .on_ok(|_, _, _| true)
            // Escape: close with nothing discarded.
            .on_cancel(|_, _, _| true)
            .on_close(move |_, window, cx| window.focus(&close, cx))
    });
}

/// A dialog button. Enter on it presses it: its wrapper takes the dialog's
/// Enter (`Confirm`, dispatched on key down) before the dialog, whose
/// default it would otherwise give.
fn dialog_button(
    id: &'static str,
    label: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let on_click = Rc::new(on_click);
    let on_enter = on_click.clone();
    let mut button = Button::new(format!("{id}-button")).label(label);
    if id == "discard-draft-confirm" {
        button = button.danger();
    }
    div()
        .id(id)
        .test_support()
        .aria_label(label)
        .on_action(move |_: &DialogConfirm, window, cx| {
            on_enter(&ClickEvent::default(), window, cx)
        })
        .child(button.on_click(move |event, window, cx| on_click(event, window, cx)))
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
                // The safe choice is the first Tab stop: one stray Tab and
                // Enter keeps the draft.
                .child(dialog_button(
                    "discard-draft-cancel",
                    CANCEL,
                    |_, window, cx| window.close_dialog(cx),
                ))
                .child(dialog_button(
                    "discard-draft-confirm",
                    DISCARD_CONFIRM,
                    move |_, window, cx| {
                        window.close_dialog(cx);
                        confirm(window, cx);
                    },
                )),
        )
}
