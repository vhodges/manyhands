//! The create dialogs (design, Create (decision 3); R-20, R-21, R-22, R-46):
//! New document… and New ticket…, opened on the selected repository from
//! the shown list's button, or by `secondary-n` in the navigation region.
//!
//! - **The fields.** Both take a Title. A ticket also takes a Type, `task`
//!   until changed; it is made with the status `open` and no initials,
//!   which are the repository's settings (R-21). A document also takes a
//!   Folder, `docs` until changed, and a File name, whose path is checked
//!   as it is typed (`document`, R-22). Create waits while a required field
//!   is empty, saying so in the details sheet's words (Task 20).
//! - **Create**, in R-20's order: a new ID (`new_item_id`), then the
//!   registry's one entry point, which writes the new item's draft and only
//!   then sends `context prepare` with the Create intent
//!   (`DraftRegistry::new_item`). Then the dialog says `Created`: it closes
//!   itself (`finish`; it waits under a dialog opened above it), and the
//!   new tab follows that prepare to the edit editor. With no draft
//!   store nothing is provisioned, and the dialog stays open, saying why
//!   (R-46). One create runs at a time, and Escape waits for it; Cancel
//!   is disabled meanwhile (Part E review, M5).
//! - **Keyboard.** Focus starts in the Title; Tab moves through the fields,
//!   Cancel and Create; Enter in a field creates, when Create is offered;
//!   Escape closes the dialog, and focus goes back where it was.
//!
//! `Workspace::new_item` is the way in; the drawing is `form`'s. The
//! fields hold the user's text, so nothing here derives `Debug`.

mod document;
mod form;

use gpui_kit::{
    App, AppContext, Context, Entity, EventEmitter, FocusHandle, ParentElement, Subscription, Task,
    Window,
    component::{
        WindowExt,
        input::{InputEvent, InputState},
    },
};
use manyhands::{
    canonical::ItemId,
    drafts::{DraftError, DraftErrorKind, DraftFields, ItemKind},
    repository::{ItemDtoKind, NewIdDto, ReadError},
    results::ResultCode,
};

use self::document::Place;
/// Move… and New folder are drawn and checked as these dialogs are (Task
/// 24).
pub(in crate::desktop::view) use self::{
    document::PathCheck,
    form::{button, field, focus_the_note, held_button, keep_focus_in},
};
use super::workspace::Workspace;
use crate::desktop::{
    model::{
        create::DEFAULT_FOLDER,
        drafts::create::{
            DEFAULT_TYPE, document_fields, new_item_draft, ticket_fields, ticket_path,
        },
        edit::fields::missing,
        navigation::{OpenTarget, Pane},
        stamp::{Generations, RepoKey, Stamp, ViewScope},
    },
    services::{
        ReadKind, ReadResult, Services,
        drafts::{NewItem, RECOVERY_NEEDED},
    },
};

pub(crate) const NEW_DOCUMENT: &str = "New document\u{2026}";
pub(crate) const NEW_TICKET: &str = "New ticket\u{2026}";
pub(crate) const DOCUMENT_HEADING: &str = "New document";
pub(crate) const TICKET_HEADING: &str = "New ticket";
pub(crate) const CREATE: &str = "Create";
pub(crate) const CANCEL: &str = "Cancel";
pub(crate) const TITLE: &str = "Title";
pub(crate) const TYPE: &str = "Type";
pub(crate) const FOLDER: &str = "Folder";
pub(crate) const FILE_NAME: &str = "File name";
/// The new item's draft could not be written, the store being open: it is
/// followed by why, in the store's words.
pub(crate) const DRAFT_NOT_WRITTEN: &str =
    "The new item's draft could not be written, so nothing was created.";

/// A dialog's new item: its draft is written and its Create prepare sent
/// (R-20). Its tab is to open, editing it.
pub(crate) struct Created {
    pub(crate) target: OpenTarget,
    pub(crate) item: NewItem,
}

impl EventEmitter<Created> for CreateDialog {}

/// The fields besides the Title.
enum Inputs {
    Ticket { ticket_type: Entity<InputState> },
    Document(Place),
}

pub(crate) struct CreateDialog {
    root: RepoKey,
    /// The primary branch, as the repository list gives it.
    branch: Option<String>,
    services: Services,
    title: Entity<InputState>,
    inputs: Inputs,
    /// The path checks' stamps: each change moves them on (`document`).
    checks: Generations,
    observe: Option<Task<()>>,
    /// Create runs, from its new ID until the dialog closes or says why
    /// nothing was made.
    creating: bool,
    /// Its item is made: Create offers nothing more, and Escape and Cancel
    /// close the dialog, which waits under one opened above it.
    created: bool,
    /// Around the dialog's own controls: it closes itself only while focus
    /// is here, so never a dialog opened above it (Task 23 fix 1).
    focus: FocusHandle,
    /// Create's wrapper, where a keyed press parks focus (`form::held_button`).
    primary: FocusHandle,
    ids: Generations,
    id_read: Option<Task<()>>,
    /// Why the last Create made nothing.
    failure: Option<String>,
    _events: Vec<Subscription>,
}

/// Opens New document… (`kind` Document) or New ticket… on the repository
/// `root`, whose primary branch the repository list gives as `branch`, with
/// focus in its Title. Nothing opens while quitting. Gives the dialog, whose
/// `Created` the caller follows.
fn open(
    kind: ItemDtoKind,
    root: RepoKey,
    branch: Option<String>,
    services: &Services,
    window: &mut Window,
    cx: &mut App,
) -> Option<Entity<CreateDialog>> {
    if services.worker().quitting() || services.operations().read(cx).saving_drafts() {
        return None;
    }
    let dialog = cx.new(|cx| CreateDialog::new(kind, root, branch, services.clone(), window, cx));
    let heading = match kind {
        ItemDtoKind::Document => DOCUMENT_HEADING,
        ItemDtoKind::Ticket => TICKET_HEADING,
    };
    let built = dialog.clone();
    window.open_dialog(cx, move |base, _, _| {
        let creating = built.clone();
        let cancelling = built.clone();
        base.title(heading)
            .close_button(false)
            .overlay_closable(false)
            .child(built.clone())
            // Enter that no field or button took: Create, when offered. The
            // dialog closes once the draft is written, so never here.
            .on_ok(move |_, window, cx| {
                creating.update(cx, |dialog, cx| dialog.create(window, cx));
                false
            })
            // Escape: closed with nothing made, but not while Create runs.
            .on_cancel(move |_, _, cx| !cancelling.read(cx).creating)
    });
    let title = dialog.read(cx).title.clone();
    title.update(cx, |input, cx| input.focus(window, cx));
    #[cfg(test)]
    cx.set_global(OpenedDialog(dialog.downgrade()));
    Some(dialog)
}

/// The dialog last opened, for a test to type into and read.
#[cfg(test)]
pub(crate) struct OpenedDialog(pub(crate) gpui_kit::WeakEntity<CreateDialog>);

#[cfg(test)]
impl gpui_kit::Global for OpenedDialog {}

impl Workspace {
    /// New document… or New ticket…, the shown list's, on the selected
    /// repository: from its button, or `secondary-n` in the navigation
    /// region. Nothing opens with no repository shown, nor in front of a
    /// dialog or the sheet. The dialog's new item opens in its tab.
    pub(in crate::desktop::view) fn new_item(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if window.has_active_dialog(cx) || window.has_active_sheet(cx) {
            return;
        }
        let (kind, root) = match self.navigation().pane() {
            Pane::Documents => (ItemDtoKind::Document, self.documents().repo()),
            Pane::Tickets => (ItemDtoKind::Ticket, self.tickets().repo()),
        };
        let Some(root) = root.cloned() else {
            return;
        };
        let branch = self
            .navigation()
            .entry(&root)
            .and_then(|entry| entry.summary.configuration.primary_branch.clone());
        let Some(dialog) = open(kind, root, branch, self.services(), window, cx) else {
            return;
        };
        cx.subscribe_in(
            &dialog,
            window,
            |workspace, dialog, created: &Created, window, cx| {
                dialog.update(cx, |dialog, cx| dialog.finish(window, cx));
                workspace.open_created(created.target.clone(), created.item, window, cx)
            },
        )
        .detach();
    }
}

impl CreateDialog {
    fn new(
        kind: ItemDtoKind,
        root: RepoKey,
        branch: Option<String>,
        services: Services,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let input = |value: &'static str, window: &mut Window, cx: &mut Context<Self>| {
            cx.new(|cx| InputState::new(window, cx).default_value(value))
        };
        let title = input("", window, cx);
        let inputs = match kind {
            ItemDtoKind::Ticket => Inputs::Ticket {
                ticket_type: input(DEFAULT_TYPE, window, cx),
            },
            ItemDtoKind::Document => Inputs::Document(Place {
                folder: input(DEFAULT_FOLDER, window, cx),
                file_name: input("", window, cx),
                check: PathCheck::Checking,
            }),
        };
        // Each input, and whether it is a path field.
        let mut watched = vec![(title.clone(), false)];
        match &inputs {
            Inputs::Ticket { ticket_type } => watched.push((ticket_type.clone(), false)),
            Inputs::Document(place) => watched.extend([
                (place.folder.clone(), true),
                (place.file_name.clone(), true),
            ]),
        }
        let events = watched
            .into_iter()
            .map(|(input, path)| {
                cx.subscribe_in(
                    &input,
                    window,
                    move |dialog, _, event, window, cx| match event {
                        InputEvent::Change => dialog.changed(path, cx),
                        InputEvent::PressEnter { .. } => dialog.create(window, cx),
                        InputEvent::Focus | InputEvent::Blur => {}
                    },
                )
            })
            .collect();
        let mut dialog = Self {
            root,
            branch,
            services,
            title,
            inputs,
            checks: Generations::default(),
            observe: None,
            creating: false,
            created: false,
            focus: cx.focus_handle(),
            primary: cx.focus_handle(),
            ids: Generations::default(),
            id_read: None,
            failure: None,
            _events: events,
        };
        dialog.check_path(cx);
        dialog
    }

    fn kind(&self) -> ItemDtoKind {
        match self.inputs {
            Inputs::Ticket { .. } => ItemDtoKind::Ticket,
            Inputs::Document(_) => ItemDtoKind::Document,
        }
    }

    /// The new item's fields, as typed.
    fn fields(&self, cx: &App) -> DraftFields {
        let title = self.title.read(cx).value();
        match &self.inputs {
            Inputs::Ticket { ticket_type } => ticket_fields(&title, &ticket_type.read(cx).value()),
            Inputs::Document(_) => document_fields(&title),
        }
    }

    /// Why Create is not offered, in words: a required field left empty,
    /// then what the path check says.
    fn reason(&self, cx: &App) -> Option<String> {
        missing(&self.fields(cx))
            .map(str::to_owned)
            .or_else(|| self.path_note())
    }

    /// Whether Create can create now: every required field filled, a
    /// document's path checked free, and no create running.
    fn offered(&self, cx: &App) -> bool {
        !self.creating
            && !self.created
            && missing(&self.fields(cx)).is_none()
            && self.place().is_some()
    }

    /// A field changed: a path field's change checks the path again, and so
    /// does any field's while the last check failed (Task 23 fix 1).
    fn changed(&mut self, path: bool, cx: &mut Context<Self>) {
        if path || self.path_failed() {
            self.check_path(cx);
        }
        cx.notify();
    }

    /// Create (R-20): a new ID first, while Create is offered, then the
    /// new item's draft (`id_taken`). One runs at a time.
    #[allow(clippy::result_large_err)] // The library's read error, as every read gives it.
    fn create(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.offered(cx) {
            return;
        }
        let (fields, place) = (self.fields(cx), self.place().flatten());
        self.creating = true;
        self.failure = None;
        let stamp = self.ids.next(ViewScope::Create);
        self.id_read = Some(self.services.read(
            ReadKind::NewId,
            stamp,
            |service| Ok::<_, ReadError>(service.new_item_id()),
            cx,
            move |dialog: &mut Self, stamp, result, cx| {
                dialog.id_taken(stamp, fields, place, result, cx)
            },
        ));
        cx.notify();
    }

    /// The new ID landed: the new item's draft begins over it.
    fn id_taken(
        &mut self,
        stamp: Stamp,
        fields: DraftFields,
        place: Option<String>,
        result: ReadResult<NewIdDto>,
        cx: &mut Context<Self>,
    ) {
        if !self.ids.is_current(&stamp) {
            return;
        }
        let code = match result {
            ReadResult::Ok(made) => match made.id.parse::<ItemId>() {
                Ok(id) => return self.begin(id, fields, place, cx),
                Err(_) => ResultCode::InternalError,
            },
            ReadResult::Busy => ResultCode::Busy,
            ReadResult::Failed(failure) => failure.code,
        };
        self.not_created(code.message().to_owned(), cx);
    }

    /// The registry writes the new item `id`'s draft, at `place` or, for a
    /// ticket, at its ID's path, in the primary context, then sends its
    /// Create prepare (`DraftRegistry::new_item`). That runs to its end
    /// whatever becomes of the dialog, so a draft written is always
    /// followed by its prepare.
    fn begin(
        &mut self,
        id: ItemId,
        fields: DraftFields,
        place: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let kind = self.kind();
        let item_kind = match kind {
            ItemDtoKind::Document => ItemKind::Document,
            ItemDtoKind::Ticket => ItemKind::Ticket,
        };
        let path = place.unwrap_or_else(|| ticket_path(&id));
        let (target, base) = new_item_draft(
            &self.root,
            self.branch.clone(),
            item_kind,
            id.clone(),
            path,
            &fields,
        );
        let open = OpenTarget::Item {
            repo: self.root.clone(),
            kind,
            id,
        };
        let begun = self.services.drafts().update(cx, |registry, cx| {
            registry.new_item(target, base, fields, cx)
        });
        cx.spawn(async move |dialog, cx| {
            let begun = begun.await;
            let _ = dialog.update(cx, |dialog, cx| dialog.begun(open, begun, cx));
        })
        .detach();
    }

    /// The new item's draft is written and its prepare sent: `Created`.
    /// Otherwise nothing is provisioned, and the dialog says why: crash
    /// recovery is off (R-46), or the draft could not be written.
    fn begun(
        &mut self,
        target: OpenTarget,
        begun: Result<NewItem, DraftErrorKind>,
        cx: &mut Context<Self>,
    ) {
        match begun {
            Ok(item) => cx.emit(Created { target, item }),
            Err(kind) => {
                let words = match self.services.drafts().read(cx).unavailable() {
                    Some(_) => RECOVERY_NEEDED.to_owned(),
                    None => format!("{DRAFT_NOT_WRITTEN} {}", DraftError { kind }),
                };
                self.not_created(words, cx);
            }
        }
    }

    fn not_created(&mut self, words: String, cx: &mut Context<Self>) {
        self.creating = false;
        self.failure = Some(words);
        cx.notify();
    }

    /// Its item is made (`Created`): the dialog closes itself, while it is
    /// the dialog focus is in. Under a dialog opened above it meanwhile, as
    /// a credential prompt may be, it stays, with nothing more to create,
    /// until Escape or Cancel closes it. The Kit closes only the topmost
    /// dialog.
    fn finish(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.creating = false;
        self.created = true;
        if self.focus.contains_focused(window, cx) {
            window.close_dialog(cx);
        }
        cx.notify();
    }

    /// Cancel: closed with nothing made, but not while Create runs.
    fn cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.creating {
            window.close_dialog(cx);
        }
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "create_tests.rs"]
mod tests;
