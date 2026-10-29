//! Repair (design, Part E: Repair; R-25; the Cycle's contract 11 and its
//! Known Limits): a file under `docs/` or `.manyhands/tickets/` corrected in
//! a plain text area over the whole file, and replaced through the
//! library's `document repair` or `ticket repair` once the one
//! confirmation dialog has previewed it.
//!
//! - **Entry.** "Needs fixing" offers Repair…; an unmanaged document's
//!   banner, a file under `docs/` with no front matter, offers Adopt…, a
//!   repair with adoption; an Inspect tab of a file that needs no fixing
//!   offers Repair… on its page. Each is offered while the tab views a file
//!   it read on primary, the copy the library repairs and whose token it
//!   takes (`DocumentRepairInput::observation`), with no edit session. A
//!   file that is not UTF-8 says it cannot be repaired here instead. A new
//!   item's stand-in is never read, so it is never repaired.
//! - **Editing** (R-25). The text area holds the whole file exactly as the
//!   tab read it, and that read's observation is the one sent with it. The
//!   text never passes through the markdown editor, its normalization or the
//!   draft store: it is not crash-protected (Known Limits). Tab and
//!   Shift-Tab leave it, as D1's source view's do.
//! - **Preview** prepares the repair in `view::confirm`, which shows Root,
//!   Path and ID and, for an adoption, "A new ID will be assigned". A stale
//!   preview, an ambiguous identity and every other refusal are the
//!   dialog's to explain, with Prepare again where the library offers it.
//! - **The answer.** Every attempt of the confirmed request is followed, a
//!   retry's included: the library answers a request sent again with what it
//!   stored. A committed checkpoint, `ok` or `already_applied` is the repair
//!   made: the view closes, and the tab follows the item by the ID the
//!   preview named (`FollowItem`), reading it again there. Any other answer
//!   is said under the text, the library's message and code, and the text is
//!   kept. While a request runs, the text and Preview wait.
//! - **Leaving.** Cancel, closing the tab, or quitting with text that
//!   differs from the file as read asks first; unedited text goes unasked.
//!
//! The text is the file's, so nothing here derives `Debug`, and it reaches
//! no operation entry, notification, dialog message or error: only the
//! request carries it to the library.

mod prompt;
mod view;

use std::path::PathBuf;

use gpui_kit::{
    App, AppContext, Context, Entity, EventEmitter, Focusable, Subscription, Window,
    component::{
        WindowExt,
        input::{InputEvent, TextareaState},
    },
};
use manyhands::{
    canonical::ItemId,
    repository::{ItemContextKind, ItemDto, ItemDtoKind, PreviewDetailsDto},
    results::{CheckpointEffect, ProblemCode, ResultCode},
};

use super::{CloseThisTab, ItemPane};
use crate::desktop::{
    model::{edit::Phase, item::CONTEXT_PROBLEMS, navigation::OpenTarget},
    services::requests::{DesktopRequestId, Intent, RequestState},
    view::confirm::{self, Confirmed},
};
use prompt::Choice;

/// A file with no text to repair (pre-flight ruling, Task 25).
pub(crate) const REPAIR_NOT_UTF8: &str =
    "This file is not UTF-8 text, so it cannot be repaired here.";
/// An unmanaged document's banner (the handoff's words,
/// `docs/design/04-item-area.md`).
pub(crate) const UNMANAGED: &str = "This file isn't a managed document yet. Adopting it adds a \
                                    title and an ID at the top; the rest stays exactly as \
                                    written.";
pub(crate) const PREVIEW: &str = "Preview";

/// A repair is made: the tab follows its item, by its ID, and reads it
/// there.
pub(crate) struct FollowItem(pub(crate) OpenTarget);

impl EventEmitter<FollowItem> for ItemPane {}

/// The kind of item the file at `path` is repaired as: a document under
/// `docs/`, a ticket's file under `.manyhands/tickets/`.
fn repair_kind(path: &str) -> Option<ItemDtoKind> {
    if path.starts_with("docs/") {
        Some(ItemDtoKind::Document)
    } else if path.starts_with(".manyhands/tickets/") {
        Some(ItemDtoKind::Ticket)
    } else {
        None
    }
}

/// Whether `item`, a nonconforming entry, is an unmanaged document: a file
/// under `docs/` whose only problem is that it has no front matter.
pub(super) fn unmanaged(item: &ItemDto) -> bool {
    let mut problems = item
        .problems
        .iter()
        .filter(|problem| !CONTEXT_PROBLEMS.contains(&problem.code));
    repair_kind(&item.path) == Some(ItemDtoKind::Document)
        && problems
            .next()
            .is_some_and(|problem| problem.code == ProblemCode::MissingFrontMatter)
        && problems.next().is_none()
}

/// What a tab offers for the file it read.
enum Offer {
    Repair,
    /// The file is not UTF-8: it says it cannot be repaired here.
    NotUtf8,
}

/// The request a confirmation sent, the attempt whose answer was taken, and
/// the ID its preview named.
struct Sent {
    request: DesktopRequestId,
    taken: u32,
    id: Option<ItemId>,
}

/// The repair view. It holds the file's text, so it derives nothing.
pub(super) struct Repairing {
    root: PathBuf,
    path: String,
    kind: ItemDtoKind,
    /// The token of the read the text came from, on primary.
    observation: String,
    /// The file as read: text equal to it is unedited.
    read: String,
    text: Entity<TextareaState>,
    sent: Option<Sent>,
    /// A repair is with the worker: the text and Preview wait.
    running: bool,
    /// The last answer that made no repair: the library's message and code.
    failure: Option<(String, ResultCode)>,
    /// The last confirmation opened, whose Confirm says what it sent.
    _confirming: Option<Subscription>,
    _changed: Subscription,
}

/// The text a repair request carries.
fn sent_source(intent: &Intent) -> Option<&str> {
    match intent {
        Intent::DocumentRepair { source, .. } | Intent::TicketRepair { source, .. } => Some(source),
        _ => None,
    }
}

impl ItemPane {
    /// The repair view's text area, while it is open.
    #[cfg(test)]
    pub(crate) fn repair_state(&self) -> Option<Entity<TextareaState>> {
        Some(self.repair.as_ref()?.text.clone())
    }

    /// Whether the repair view holds text that differs from the file as
    /// read: no draft keeps it, so leaving asks first.
    pub(in crate::desktop::view) fn repair_edited(&self, cx: &App) -> bool {
        self.repair
            .as_ref()
            .is_some_and(|repair| repair.text.read(cx).value().as_ref() != repair.read)
    }

    /// What `item`, this tab's read, offers: its repair, for a file under
    /// `docs/` or `.manyhands/tickets/` read on primary as UTF-8 text with
    /// its token, while the tab views it with no edit session and no repair
    /// open; or, for a file that is not UTF-8, why not.
    fn repair_offer(&self, item: &ItemDto) -> Option<Offer> {
        let viewing =
            self.edit.phase() == Phase::Viewing && !self.has_session() && self.repair.is_none();
        if !viewing
            || item.context.kind != ItemContextKind::Primary
            || repair_kind(&item.path).is_none()
        {
            return None;
        }
        match (&item.source, &item.observation) {
            (Some(_), Some(_)) => Some(Offer::Repair),
            (None, _) => Some(Offer::NotUtf8),
            (Some(_), None) => None,
        }
    }

    /// Opens the repair view on the file as this tab read it, with focus in
    /// its text.
    fn open_repair(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_dialog(cx) {
            return;
        }
        let Some(item) = self.model.item().rows() else {
            return;
        };
        if !matches!(self.repair_offer(item), Some(Offer::Repair)) {
            return;
        }
        let (Some(source), Some(observation), Some(kind)) = (
            item.source.clone(),
            item.observation.clone(),
            repair_kind(&item.path),
        ) else {
            return;
        };
        let path = item.path.clone();
        let value = source.clone();
        let text = cx.new(|cx| TextareaState::new(window, cx).default_value(value));
        // The tab's unsaved mark follows the text.
        let changed = cx.subscribe(&text, |_, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });
        text.update(cx, |state, cx| state.focus(window, cx));
        self.repair = Some(Repairing {
            root: PathBuf::from(self.repo()),
            path,
            kind,
            observation,
            read: source,
            text,
            sent: None,
            running: false,
            failure: None,
            _confirming: None,
            _changed: changed,
        });
        cx.notify();
    }

    /// Preview: the repair of the text as it is now, prepared in the
    /// confirmation dialog, whose Confirm sends it. The dialog gives focus
    /// back to what held it.
    fn preview_repair(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_dialog(cx) {
            return;
        }
        let Some(repair) = &self.repair else {
            return;
        };
        if repair.running {
            return;
        }
        let source = repair.text.read(cx).value().to_string();
        let (root, path) = (repair.root.clone(), PathBuf::from(&repair.path));
        let observation = repair.observation.clone();
        let intent = match repair.kind {
            ItemDtoKind::Document => Intent::DocumentRepair {
                root,
                path,
                observation,
                source,
            },
            ItemDtoKind::Ticket => Intent::TicketRepair {
                root,
                path,
                observation,
                source,
            },
        };
        let origin = window
            .focused(cx)
            .unwrap_or_else(|| repair.text.read(cx).focus_handle(cx));
        let Some(dialog) = confirm::confirm_followed(intent, origin, &self.services, window, cx)
        else {
            return;
        };
        let confirmed = cx.subscribe(&dialog, |pane, _, event: &Confirmed, cx| {
            pane.repair_confirmed(event, cx)
        });
        if let Some(repair) = &mut self.repair {
            repair._confirming = Some(confirmed);
        }
    }

    /// The confirmation sent the repair, or queued it again: its answers
    /// are followed from here.
    fn repair_confirmed(&mut self, event: &Confirmed, cx: &mut Context<Self>) {
        let Some(repair) = &mut self.repair else {
            return;
        };
        let id = match &event.details {
            PreviewDetailsDto::Repair(preview) => preview.id.parse().ok(),
            _ => None,
        };
        match &mut repair.sent {
            Some(sent) if sent.request == event.request => sent.id = id.or(sent.id.take()),
            sent => {
                *sent = Some(Sent {
                    request: event.request,
                    taken: 0,
                    id,
                })
            }
        }
        repair.running = true;
        repair.failure = None;
        cx.notify();
    }

    /// The request model changed: an attempt of the repair's request that
    /// the view has not taken runs, or was answered.
    pub(super) fn repair_progressed(&mut self, cx: &mut Context<Self>) {
        let Some(sent) = self.repair.as_ref().and_then(|repair| repair.sent.as_ref()) else {
            return;
        };
        let (request, taken) = (sent.request, sent.taken);
        let Some((attempt, state, carried)) = self
            .services
            .operations()
            .read(cx)
            .request(request)
            .filter(|listed| listed.attempt > taken)
            .map(|listed| {
                let carried = sent_source(&listed.intent).map(str::to_owned);
                (listed.attempt, listed.state.clone(), carried)
            })
        else {
            return;
        };
        let Some(repair) = &mut self.repair else {
            return;
        };
        match state {
            // A retry of it: the repair runs again.
            RequestState::Queued | RequestState::Running { .. } => repair.running = true,
            RequestState::Finished(_, code, effects, _, message) => {
                if let Some(sent) = &mut repair.sent {
                    sent.taken = attempt;
                }
                let made = effects.checkpoint == CheckpointEffect::Committed
                    || matches!(code, ResultCode::Ok | ResultCode::AlreadyApplied);
                if made {
                    return self.repaired(carried, cx);
                }
                repair.running = false;
                repair.failure = Some((message, code));
            }
            RequestState::Removed | RequestState::Dismissed | RequestState::NotSent(_) => {
                if let Some(sent) = &mut repair.sent {
                    sent.taken = attempt;
                }
                repair.running = false;
            }
        }
        cx.notify();
    }

    /// The repair is made with `carried`, the text its request sent. That
    /// text is the item now: the view closes, and the tab follows the item
    /// by the ID the preview named. Text changed since it was sent is not
    /// what was repaired, and stays.
    fn repaired(&mut self, carried: Option<String>, cx: &mut Context<Self>) {
        let Some(repair) = &mut self.repair else {
            return;
        };
        repair.running = false;
        repair.failure = None;
        if carried.as_deref() != Some(repair.text.read(cx).value().as_ref()) {
            cx.notify();
            return;
        }
        let id = repair.sent.as_ref().and_then(|sent| sent.id.clone());
        let kind = repair.kind;
        self.repair = None;
        if let Some(id) = id {
            let repo = self.repo().to_owned();
            cx.emit(FollowItem(OpenTarget::Item { repo, kind, id }));
        }
        cx.notify();
    }

    /// The view's Cancel: unedited text closes the view; edited, it asks
    /// first.
    fn cancel_repair(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_dialog(cx) || self.repair.as_ref().is_none_or(|repair| repair.running)
        {
            return;
        }
        if self.repair_edited(cx) {
            self.ask_discard_repair(false, window, cx);
        } else {
            self.close_repair(window, cx);
        }
    }

    /// Asks before edited repair text is lost, which no draft keeps:
    /// Discard drops it, and closes the tab when `closing`; Cancel, the
    /// default, keeps both.
    pub(in crate::desktop::view) fn ask_discard_repair(
        &mut self,
        closing: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pane = cx.entity().downgrade();
        prompt::ask(window, cx, move |choice, window, cx| {
            if choice != Choice::Discard {
                return;
            }
            let _ = pane.update(cx, |pane, cx| {
                if closing {
                    pane.repair = None;
                    cx.emit(CloseThisTab);
                } else {
                    pane.close_repair(window, cx);
                }
            });
        });
    }

    /// The view closes, and the file's own text takes the focus.
    fn close_repair(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.repair = None;
        if let Some((_, state)) = &self.source {
            state.update(cx, |state, cx| state.focus(window, cx));
        }
        cx.notify();
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "repair_tests.rs"]
mod tests;
