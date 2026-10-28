//! The one confirmation dialog every confirmed command uses.
//!
//! `confirm` prepares the command as a read and shows its preview: the
//! command, its scope, each effect, the command's own details and when
//! the confirmation expires, in local time. A `host approve` refused as
//! `host_replacement_required` shows both pins instead. Confirm, by
//! click or Enter, submits the request once with the preview's
//! confirmation and is then unavailable; a second press does nothing.
//! Escape or Cancel closes the dialog with nothing asked of the worker,
//! and focus goes back where the dialog was opened from. A prepare still
//! busy after its retries is shown as busy with Retry, which prepares
//! again under the same request, and Close.
//!
//! If the request is answered with a stale confirmation, or the state the
//! preview showed has changed and the library offers a new preview, the
//! dialog shows that answer and Prepare again, which takes a fresh
//! preview under the same desktop request and request ID. Any other
//! answer closes the dialog: the operation area holds it. Text is the
//! envelope's message, the code and fixed strings.

use gpui_kit::{
    AnyElement, App, AppContext, ClickEvent, Context, ElementId, FocusHandle, InteractiveElement,
    IntoElement, ParentElement, Render, StatefulInteractiveElement, Styled, Subscription, Task,
    TestSupportExt, Window,
    component::{
        ActiveTheme, Disableable, WindowExt,
        button::{Button, ButtonVariants},
        dialog::Confirm,
        h_flex, v_flex,
    },
    div,
};
use manyhands::{
    repository::{
        ConfirmationId, HostReplacementDto, PrepareDataDto, PreviewDetailsDto, PreviewDto,
    },
    results::{Envelope, RecoveryAction, RecoveryActionKind, ResultCode},
};
use time::{OffsetDateTime, UtcOffset, format_description::well_known::Rfc3339};

use crate::desktop::{
    model::stamp::{Generations, Stamp, ViewScope},
    services::{
        KEY_STORE_UNAVAILABLE, Services,
        requests::{DesktopRequestId, Intent, Lane, NotBuilt, RequestState},
    },
    view::quit::unkept_tabs,
};

pub(crate) const CONFIRM: &str = "Confirm";
pub(crate) const CANCEL: &str = "Cancel";
pub(crate) const CLOSE: &str = "Close";
pub(crate) const PREPARE_AGAIN: &str = "Prepare again";
pub(crate) const RETRY: &str = "Retry";
pub(crate) const PREPARING: &str = "Preparing the preview";
pub(crate) const SUBMITTED: &str = "Submitted";
pub(crate) const NOT_QUEUED: &str = "The request cannot be submitted again now: it is still queued or running, or Manyhands is quitting.";
pub(crate) const SECRET_REQUIRED: &str = "The passphrase is needed again.";
pub(crate) const REQUESTED_KEY: &str = "Requested key";
pub(crate) const PINNED_KEY: &str = "Pinned key";
/// A repository removal's preview, which says it changes no file and
/// nothing Git holds, says so in words too (design, Repositories).
pub(crate) const FILES_REMAIN: &str = "Local files remain: only the registration is removed. Nothing in the folder or in Git is changed.";
/// A removal keeps the drafts of the repository's items (design, Tabs).
pub(crate) const DRAFTS_REMAIN: &str = "Drafts of this repository's items stay in Drafts.";

/// A removal closes the repository's tabs, `count` of which hold unsaved
/// text that no draft holds (Part C fix 1, I1): `None` when none do. The
/// words name no item and no text.
pub(crate) fn removing_closes(count: usize) -> Option<String> {
    let them = match count {
        0 => return None,
        1 => "it",
        _ => "them",
    };
    Some(format!("{}. Removing closes {them}.", unkept_tabs(count)))
}

/// The answers that mean the preview no longer stands: the dialog offers
/// Prepare again.
pub(crate) const STALE_CODES: [ResultCode; 5] = [
    ResultCode::ConfirmationExpired,
    ResultCode::ConfirmationUsed,
    ResultCode::ConfirmationMismatch,
    ResultCode::ConfirmationNotFound,
    ResultCode::ExternalChange,
];

/// Whether an execute answered `code` with `recovery` is shown with
/// Prepare again. An external change is, only when the library offers
/// `request.prepare`: on a record left accepted it offers
/// `repo.inspect`, and a new preview would re-enter that record. The
/// answer then closes the dialog, and the operation area shows its
/// message and recovery. A spent or mismatched confirmation comes with
/// no action, the library asking for a new preview (design,
/// Confirmation Dialog).
pub(crate) fn offers_prepare_again(code: ResultCode, recovery: &[RecoveryAction]) -> bool {
    match code {
        ResultCode::ExternalChange => recovery
            .iter()
            .any(|action| action.action == RecoveryActionKind::RequestPrepare),
        code => STALE_CODES.contains(&code),
    }
}

/// Opens the confirmation dialog for `intent`: prepares it, shows the
/// preview and, on Confirm, submits it once as a user job. Closing it
/// focuses `origin`.
pub(crate) fn confirm(
    intent: Intent,
    origin: FocusHandle,
    services: &Services,
    window: &mut Window,
    cx: &mut App,
) {
    open(intent, None, origin, services, window, cx);
}

/// Opens the dialog for the request `id` again, whose answer asked for a
/// new preview: a fresh preview under the same request.
pub(crate) fn prepare_again(
    id: DesktopRequestId,
    origin: FocusHandle,
    services: &Services,
    window: &mut Window,
    cx: &mut App,
) {
    let Some(intent) = services
        .operations()
        .read(cx)
        .request(id)
        .map(|request| request.intent.clone())
    else {
        return;
    };
    open(intent, Some(id), origin, services, window, cx);
}

fn open(
    intent: Intent,
    request: Option<DesktopRequestId>,
    origin: FocusHandle,
    services: &Services,
    window: &mut Window,
    cx: &mut App,
) {
    // Quitting waits for drafts or a job with Quit now in view: nothing
    // opens in front of it, and once the worker quits no request could be
    // sent.
    if services.worker().quitting() || services.operations().read(cx).saving_drafts() {
        return;
    }
    let title = intent.label();
    let dialog = cx.new(|cx| {
        ConfirmDialog::new(
            intent,
            request,
            origin.clone(),
            services.clone(),
            window,
            cx,
        )
    });
    window.open_dialog(cx, move |base, _, _| {
        let confirming = dialog.clone();
        let origin = origin.clone();
        base.title(title)
            .close_button(false)
            .overlay_closable(false)
            .child(dialog.clone())
            // Enter: the dialog closes itself once the request is
            // answered, so the Kit never closes it here.
            .on_ok(move |_, _, cx| {
                confirming.update(cx, |dialog, cx| dialog.confirm(cx));
                false
            })
            // Escape: close with nothing submitted.
            .on_cancel(|_, _, _| true)
            .on_close(move |_, window, cx| window.focus(&origin, cx))
    });
}

/// Where the dialog stands.
enum Phase {
    Preparing,
    /// The preview is shown and Confirm is available.
    Ready,
    /// Confirm was pressed: the request is with the worker.
    Submitted,
    /// The preview is no longer good: Prepare again.
    Stale {
        code: ResultCode,
        message: String,
    },
    /// `prepare` gave no preview: a refusal, or nothing left to do.
    Answered {
        code: ResultCode,
        message: String,
        replacement: Option<HostReplacementDto>,
    },
    /// Nothing was asked of the library: the command could not be made,
    /// or the request could not be queued again.
    Message(&'static str),
}

pub(crate) struct ConfirmDialog {
    intent: Intent,
    services: Services,
    origin: FocusHandle,
    /// The desktop request this dialog submitted, or prepares again.
    request: Option<DesktopRequestId>,
    /// The attempt of `request` this dialog submitted: an answer to an
    /// earlier one is not this dialog's to show.
    attempt: u32,
    preview: Option<(Box<PreviewDto>, ConfirmationId)>,
    phase: Phase,
    generations: Generations,
    prepare_read: Option<Task<()>>,
    _operations: Subscription,
}

impl ConfirmDialog {
    fn new(
        intent: Intent,
        request: Option<DesktopRequestId>,
        origin: FocusHandle,
        services: Services,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let operations = cx.observe_in(services.operations(), window, |this, _, window, cx| {
            this.operations_changed(window, cx)
        });
        let mut dialog = Self {
            intent,
            services,
            origin,
            request,
            attempt: 0,
            preview: None,
            phase: Phase::Preparing,
            generations: Generations::default(),
            prepare_read: None,
            _operations: operations,
        };
        dialog.prepare(cx);
        dialog
    }

    /// Takes a fresh preview of the intent.
    fn prepare(&mut self, cx: &mut Context<Self>) {
        self.preview = None;
        self.phase = Phase::Preparing;
        let services = self.services.clone();
        let mutation = match self.intent.build(None, || services.key_store()) {
            Ok(mutation) => mutation,
            Err(NotBuilt::KeyStoreUnavailable) => {
                self.phase = Phase::Message(KEY_STORE_UNAVAILABLE);
                cx.notify();
                return;
            }
            Err(NotBuilt::MissingSecret) => {
                self.phase = Phase::Message(SECRET_REQUIRED);
                cx.notify();
                return;
            }
        };
        let stamp = self.generations.next(ViewScope::Confirmation);
        self.prepare_read = Some(
            self.services
                .prepare(stamp, mutation, cx, Self::apply_prepare),
        );
        cx.notify();
    }

    fn apply_prepare(
        &mut self,
        stamp: Stamp,
        envelope: Envelope<PrepareDataDto>,
        cx: &mut Context<Self>,
    ) {
        if !self.generations.is_current(&stamp) {
            return;
        }
        let Envelope {
            code,
            message,
            data,
            ..
        } = envelope;
        match data {
            Some(PrepareDataDto::Preview(preview)) if code == ResultCode::Ok => {
                match ConfirmationId::parse(&preview.confirmation_id) {
                    Ok(confirmation) => {
                        self.preview = Some((preview, confirmation));
                        self.phase = Phase::Ready;
                    }
                    Err(_) => {
                        self.phase = Phase::Answered {
                            code: ResultCode::InternalError,
                            message: ResultCode::InternalError.message().to_owned(),
                            replacement: None,
                        };
                    }
                }
            }
            data => {
                let replacement = match data {
                    Some(PrepareDataDto::HostReplacement(replacement)) => Some(replacement),
                    _ => None,
                };
                self.phase = Phase::Answered {
                    code,
                    message,
                    replacement,
                };
                // What the row offered may be gone: the views that show
                // the repository read it again. A busy answer says nothing
                // changed: Retry prepares again.
                if code != ResultCode::Busy
                    && let Some(root) = self.intent.root().and_then(|root| root.to_str())
                {
                    let root = root.to_owned();
                    let items = self.intent.changes_items();
                    // A removal whose registration is already gone counts as
                    // removed: its tabs close and its selection goes (ruling
                    // R-T13a).
                    let removed = code == ResultCode::AlreadyApplied
                        && matches!(self.intent, Intent::RepoRemove { .. });
                    self.services.operations().update(cx, |operations, cx| {
                        if removed {
                            operations.mark_removed(root);
                        } else {
                            operations.mark_changed(root, items);
                        }
                        cx.notify();
                    });
                }
            }
        }
        cx.notify();
    }

    /// Confirm: submits the request once with the preview's confirmation,
    /// or, after Prepare again, queues the same request with the new one.
    /// Anything but a shown preview does nothing.
    pub(crate) fn confirm(&mut self, cx: &mut Context<Self>) {
        if !matches!(self.phase, Phase::Ready) {
            return;
        }
        let Some((_, confirmation)) = &self.preview else {
            return;
        };
        let confirmation = *confirmation;
        let worker = self.services.worker();
        match self.request {
            None => {
                self.request =
                    Some(worker.submit(self.intent.clone(), Lane::User, Some(confirmation), None));
                self.attempt = 1;
            }
            Some(id) => match worker.reconfirm(id, confirmation) {
                Some(attempt) => self.attempt = attempt,
                None => {
                    self.phase = Phase::Message(NOT_QUEUED);
                    cx.notify();
                    return;
                }
            },
        }
        // Part C fix 2, N1: once it lands, a removal closes only the tabs
        // whose unkept text the line counted as it was confirmed.
        if let Some(root) = self.removal_root() {
            let root = root.to_owned();
            self.services
                .edit_tabs()
                .update(cx, |tabs, cx| tabs.confirmed(&root, cx));
        }
        self.phase = Phase::Submitted;
        cx.notify();
    }

    /// Prepare again: a fresh preview under the same request.
    fn prepare_again(&mut self, cx: &mut Context<Self>) {
        if matches!(self.phase, Phase::Stale { .. }) {
            self.prepare(cx);
        }
    }

    /// Retry on a prepare that stayed busy: the same preview is asked for
    /// again, under the same request.
    fn retry(&mut self, cx: &mut Context<Self>) {
        if matches!(
            self.phase,
            Phase::Answered {
                code: ResultCode::Busy,
                ..
            }
        ) {
            self.prepare(cx);
        }
    }

    /// The request this dialog submitted was answered: a stale preview
    /// shows Prepare again; anything else closes the dialog.
    fn operations_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !matches!(self.phase, Phase::Submitted) {
            return;
        }
        let Some(id) = self.request else {
            return;
        };
        let operations = self.services.operations().read(cx);
        let Some(request) = operations.request(id) else {
            return;
        };
        // The model still holds an earlier attempt until this one's
        // `Queued` event arrives through the worker's channel.
        if request.attempt != self.attempt {
            return;
        }
        match &request.state {
            RequestState::Queued | RequestState::Running { .. } => {}
            RequestState::Finished(_, code, _, recovery, message)
                if offers_prepare_again(*code, recovery) =>
            {
                self.phase = Phase::Stale {
                    code: *code,
                    message: message.clone(),
                };
                cx.notify();
            }
            _ => self.close(window, cx),
        }
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.close_dialog(cx);
        window.focus(&self.origin, cx);
    }
}

/// A piece of text a test reads by `id`.
type Text = gpui_kit::base::ObservedElement<gpui_kit::Stateful<gpui_kit::Div>>;

fn text(id: impl Into<ElementId>, value: impl Into<String>) -> Text {
    let value = value.into();
    div()
        .id(id)
        .test_support()
        .aria_label(value.clone())
        .child(value)
}

/// The expiry, an RFC 3339 time, in local time; as given when it cannot
/// be read.
pub(crate) fn local_time(at: &str) -> String {
    let Ok(at) = OffsetDateTime::parse(at, &Rfc3339) else {
        return at.to_owned();
    };
    let (at, zone) = match local_offset(at.unix_timestamp()) {
        Some(offset) => (at.to_offset(offset), ""),
        None => (at.to_offset(UtcOffset::UTC), " UTC"),
    };
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}{zone}",
        at.year(),
        u8::from(at.month()),
        at.day(),
        at.hour(),
        at.minute(),
        at.second()
    )
}

/// This system's offset from UTC at `timestamp`.
#[cfg(unix)]
pub(in crate::desktop::view) fn local_offset(timestamp: i64) -> Option<UtcOffset> {
    #[allow(clippy::useless_conversion)] // `time_t` is narrower on some targets.
    let time: libc::time_t = timestamp.try_into().ok()?;
    let mut local = std::mem::MaybeUninit::<libc::tm>::zeroed();
    // SAFETY: `localtime_r` is the reentrant form; it reads `time` and
    // writes the broken-down local time into `local`, which outlives the
    // call, and returns null on failure.
    let filled = unsafe { libc::localtime_r(&time, local.as_mut_ptr()) };
    if filled.is_null() {
        return None;
    }
    // SAFETY: `localtime_r` succeeded, so it initialized `local`.
    let local = unsafe { local.assume_init() };
    UtcOffset::from_whole_seconds(i32::try_from(local.tm_gmtoff).ok()?).ok()
}

/// Shown in UTC, and said so, where no local offset is read.
#[cfg(not(unix))]
pub(in crate::desktop::view) fn local_offset(_timestamp: i64) -> Option<UtcOffset> {
    None
}

fn flag(value: bool) -> String {
    if value { "yes" } else { "no" }.to_owned()
}

/// A preview's command-specific details as named values, in fixed names
/// and the library's own values.
pub(crate) fn detail_fields(details: &PreviewDetailsDto) -> Vec<(&'static str, String)> {
    let mut fields = Vec::new();
    let mut put = |name: &'static str, value: Option<String>| {
        if let Some(value) = value {
            fields.push((name, value));
        }
    };
    match details {
        PreviewDetailsDto::Repository(repository) => {
            put("Root", Some(repository.root.clone()));
            put("Primary branch", Some(repository.primary_branch.clone()));
            put("Name", Some(repository.identity.name.clone()));
            put("Email", Some(repository.identity.email.clone()));
            put(
                "Identity from",
                Some(repository.identity.source.as_str().to_owned()),
            );
            put(
                "Written to the repository",
                Some(flag(repository.identity.written_locally)),
            );
            put("Short code prefix", repository.slug_prefix.clone());
        }
        PreviewDetailsDto::RemoteRemoval(remote) => {
            put("Root", Some(remote.root.clone()));
            put("Remote", Some(remote.name.clone()));
            put("Fetches from", remote.fetch_location.clone());
            put("Pushes to", remote.push_location.clone());
            put("Polling affected", Some(flag(remote.polling_affected)));
        }
        PreviewDetailsDto::RepositoryRemoval(removal) => {
            put("Root", Some(removal.root.clone()));
            put("Registered", removal.enabled_at.clone());
            put("Files changed", Some(flag(removal.files_changed)));
            put("Git changed", Some(flag(removal.git_state_changed)));
        }
        PreviewDetailsDto::KeyDeletion(key) => {
            put("Key", Some(key.id.clone()));
            put("Label", Some(key.label.clone()));
            put("Fingerprint", key.fingerprint.clone());
            put("Private key file", Some(key.private_key_path.clone()));
            put("Public key file", key.public_key_path.clone());
        }
        PreviewDetailsDto::OperationAbandon(operation) => {
            put("Root", Some(operation.root.clone()));
            put("Operation", Some(operation.operation_id.clone()));
            put(
                "Action",
                operation.action.map(|action| action.as_str().to_owned()),
            );
            put("State", Some(operation.state.clone()));
            put("Last step", operation.step.clone());
            put("Item", operation.item_id.clone());
            put("Worktree", operation.worktree.clone());
            for path in &operation.paths {
                put("Left as it is", Some(path.clone()));
            }
            put("Committed", Some(flag(operation.committed)));
            put("Files changed", Some(flag(operation.files_changed)));
            put("Git changed", Some(flag(operation.git_state_changed)));
        }
        PreviewDetailsDto::Identity(identity) => {
            put("Root", Some(identity.root.clone()));
            put("Name", Some(identity.name.clone()));
            put("Email", Some(identity.email.clone()));
            put("Initials", identity.initials.clone());
        }
        PreviewDetailsDto::Host(host) => {
            put("Root", Some(host.root.clone()));
            put("Host", Some(format!("{}:{}", host.host, host.port)));
            put(
                REQUESTED_KEY,
                Some(format!("{} {}", host.algorithm, host.fingerprint)),
            );
            put(
                PINNED_KEY,
                match (&host.pinned_algorithm, &host.pinned_fingerprint) {
                    (Some(algorithm), Some(fingerprint)) => {
                        Some(format!("{algorithm} {fingerprint}"))
                    }
                    _ => None,
                },
            );
        }
        PreviewDetailsDto::Repair(repair) => {
            put("Root", Some(repair.root.clone()));
            put("Path", Some(repair.path.clone()));
            put("ID", Some(repair.id.clone()));
        }
    }
    fields
}

/// A preview's scope as named values.
fn scope_fields(preview: &PreviewDto) -> Vec<(&'static str, String)> {
    let scope = &preview.scope;
    [
        ("Repository", scope.repository.clone()),
        ("Item", scope.item_id.clone()),
        ("Branch", scope.branch.clone()),
        ("Worktree", scope.worktree.clone()),
        ("Remote", scope.remote.clone()),
    ]
    .into_iter()
    .filter_map(|(name, value)| value.map(|value| (name, value)))
    .collect()
}

fn named(id: impl Into<ElementId>, name: &str, value: &str) -> Text {
    text(id, format!("{name}: {value}")).text_sm()
}

impl ConfirmDialog {
    fn button(
        id: &'static str,
        label: &'static str,
        primary: bool,
        disabled: bool,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> AnyElement {
        let on_click = std::rc::Rc::new(on_click);
        let on_enter = on_click.clone();
        let button = Button::new((id, 0usize))
            .label(label)
            .disabled(disabled)
            .on_click(move |event, window, cx| on_click(event, window, cx));
        let button = if primary { button.primary() } else { button };
        // Enter on a focused button presses that button: its wrapper takes
        // the dialog's Enter (`Confirm`, dispatched on key down) before the
        // dialog, whose `on_ok` would confirm. A disabled button cannot take
        // the focus. With no button focused, Enter is the dialog's.
        let wrapper = div().id(id).test_support().aria_label(label);
        match disabled {
            true => wrapper,
            false => wrapper.on_action(move |_: &Confirm, window, cx| {
                on_enter(&ClickEvent::default(), window, cx)
            }),
        }
        .child(button)
        .into_any_element()
    }

    /// A repository removal's tabs whose edit session holds unsaved text
    /// that no draft holds, read as the dialog is drawn (Part C fix 1, I1):
    /// the confirmation says they close.
    fn unkept_tabs(&self, cx: &mut Context<Self>) -> usize {
        let Some(root) = self.removal_root() else {
            return 0;
        };
        self.services
            .edit_tabs()
            .update(cx, |tabs, cx| tabs.unkept(root, cx).len())
    }

    /// The repository a removal's confirmation removes.
    fn removal_root(&self) -> Option<&str> {
        let Intent::RepoRemove { root } = &self.intent else {
            return None;
        };
        root.to_str()
    }

    fn preview_body(&self, unkept: usize, cx: &App) -> Option<impl IntoElement + use<>> {
        let (preview, _) = self.preview.as_ref()?;
        let muted = cx.theme().muted_foreground;
        let note = match &preview.details {
            PreviewDetailsDto::RepositoryRemoval(removal)
                if !removal.files_changed && !removal.git_state_changed =>
            {
                Some(text("confirm-note", FILES_REMAIN).text_sm())
            }
            _ => None,
        };
        let removal = matches!(preview.details, PreviewDetailsDto::RepositoryRemoval(_));
        let drafts = removal.then(|| text("confirm-drafts-note", DRAFTS_REMAIN).text_sm());
        let unkept = removal
            .then(|| removing_closes(unkept))
            .flatten()
            .map(|words| text("confirm-unkept-note", words).text_sm());
        Some(
            v_flex()
                .gap_1()
                .child(text("confirm-command-name", preview.command.clone()).text_sm())
                .children(
                    scope_fields(preview)
                        .into_iter()
                        .enumerate()
                        .map(|(ix, (name, value))| named(("confirm-scope", ix), name, &value)),
                )
                .child(div().text_xs().text_color(muted).child("What it does"))
                .children(preview.effects.iter().enumerate().map(|(ix, effect)| {
                    text(("confirm-effect", ix), effect.effect.as_str()).text_sm()
                }))
                .children(
                    detail_fields(&preview.details)
                        .into_iter()
                        .enumerate()
                        .map(|(ix, (name, value))| named(("confirm-detail", ix), name, &value)),
                )
                .children(note)
                .children(drafts)
                .children(unkept)
                .child(
                    text(
                        "confirm-expires",
                        format!("Expires {}", local_time(&preview.expires_at)),
                    )
                    .text_xs()
                    .text_color(muted),
                ),
        )
    }

    fn answer(code: ResultCode, message: &str, cx: &App) -> impl IntoElement + use<> {
        v_flex()
            .gap_1()
            .child(text("confirm-message", message.to_owned()).text_sm())
            .child(
                text("confirm-code", code.as_str())
                    .text_xs()
                    .text_color(cx.theme().muted_foreground),
            )
    }
}

impl Render for ConfirmDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity().downgrade();
        let cancel = {
            let origin = self.origin.clone();
            move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                window.close_dialog(cx);
                window.focus(&origin, cx);
            }
        };
        let confirm = {
            let entity = entity.clone();
            move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                let _ = entity.update(cx, |dialog, cx| dialog.confirm(cx));
            }
        };
        let again = {
            let entity = entity.clone();
            move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                let _ = entity.update(cx, |dialog, cx| dialog.prepare_again(cx));
            }
        };
        let retry = move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
            let _ = entity.update(cx, |dialog, cx| dialog.retry(cx));
        };
        let mut body = v_flex()
            .id("confirm-dialog")
            .test_support()
            .gap_2()
            .child(text("confirm-command", self.intent.label()).text_sm());
        let mut buttons = h_flex().gap_2();
        match &self.phase {
            Phase::Preparing => {
                body = body.child(text("confirm-state", PREPARING).text_sm());
                buttons = buttons
                    .child(Self::button(
                        "confirm-ok",
                        CONFIRM,
                        true,
                        true,
                        |_, _, _| {},
                    ))
                    .child(Self::button("confirm-cancel", CANCEL, false, false, cancel));
            }
            Phase::Ready | Phase::Submitted => {
                let submitted = matches!(self.phase, Phase::Submitted);
                let unkept = self.unkept_tabs(cx);
                body = body.children(self.preview_body(unkept, cx));
                if submitted {
                    body = body.child(text("confirm-state", SUBMITTED).text_sm());
                }
                buttons = buttons
                    .child(Self::button(
                        "confirm-ok",
                        CONFIRM,
                        true,
                        submitted,
                        confirm,
                    ))
                    .child(Self::button("confirm-cancel", CANCEL, false, false, cancel));
            }
            Phase::Stale { code, message } => {
                body = body.child(Self::answer(*code, message, cx));
                buttons = buttons
                    .child(Self::button(
                        "confirm-prepare-again",
                        PREPARE_AGAIN,
                        true,
                        false,
                        again,
                    ))
                    .child(Self::button("confirm-cancel", CANCEL, false, false, cancel));
            }
            Phase::Answered {
                code,
                message,
                replacement,
            } => {
                body = body.child(Self::answer(*code, message, cx));
                if let Some(replacement) = replacement {
                    body = body
                        .child(named(
                            "confirm-host",
                            "Host",
                            &format!("{}:{}", replacement.host, replacement.port),
                        ))
                        .child(named(
                            "confirm-requested-key",
                            REQUESTED_KEY,
                            &format!("{} {}", replacement.algorithm, replacement.fingerprint),
                        ))
                        .child(named(
                            "confirm-pinned-key",
                            PINNED_KEY,
                            &format!(
                                "{} {}",
                                replacement.pinned_algorithm, replacement.pinned_fingerprint
                            ),
                        ));
                }
                // Still busy after every retry: Retry prepares again.
                if *code == ResultCode::Busy {
                    buttons =
                        buttons.child(Self::button("confirm-retry", RETRY, true, false, retry));
                }
                buttons =
                    buttons.child(Self::button("confirm-cancel", CLOSE, false, false, cancel));
            }
            Phase::Message(why) => {
                body = body.child(text("confirm-message", *why).text_sm());
                buttons =
                    buttons.child(Self::button("confirm-cancel", CLOSE, false, false, cancel));
            }
        }
        body.child(buttons)
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "confirm_tests.rs"]
mod tests;
