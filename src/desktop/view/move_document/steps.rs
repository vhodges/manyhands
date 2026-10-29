//! Move's steps (design, Part E: Move; R-23; Cycle contract 11), in the
//! design's order: R-23 asked of the tab again, the document read again by
//! its ID, the destination observed again, then `document move`; and what
//! each attempt of that request is answered (`super`).
//!
//! Nothing here derives `Debug`: the paths are the user's text.

use std::path::PathBuf;

use gpui_kit::Context;
use manyhands::{
    repository::ItemDto,
    results::{CheckpointEffect, ResultCode},
};

use super::{Failure, MOVE_WHILE_VIEWING, MoveDialog, Moved, Sent, Then, context_of};
use crate::desktop::{
    model::stamp::ViewScope,
    services::{
        ReadKind, ReadResult,
        requests::{Intent, Lane, RequestState},
    },
    view::create::PathCheck,
};

impl MoveDialog {
    /// Move: R-23 asked of the tab again, then the document read again.
    pub(crate) fn move_document(&mut self, cx: &mut Context<Self>) {
        if !self.offered() {
            return;
        }
        let PathCheck::Free(destination) = &self.check else {
            return;
        };
        let destination = destination.clone();
        let blocked = match self.pane.upgrade() {
            Some(pane) => pane.read(cx).move_blocked(cx),
            None => Some(MOVE_WHILE_VIEWING),
        };
        self.failure = blocked.map(Failure::Blocked);
        if self.failure.is_none() {
            self.running = true;
            self.read_again(destination, Then::Send, cx);
        }
        cx.notify();
    }

    /// Reads the document again by its ID, then goes on to `then`.
    #[allow(clippy::result_large_err)] // The library's read error, as every read gives it.
    fn read_again(&mut self, destination: String, then: Then, cx: &mut Context<Self>) {
        let stamp = self.steps.next(ViewScope::Create);
        let root = PathBuf::from(&self.source.root);
        let id = self.source.id.clone();
        self.step = Some(self.services.read(
            ReadKind::Item,
            stamp,
            move |service| {
                let repo = service.resolve_repository(&root)?;
                service.show_item(&repo, &id)
            },
            cx,
            move |dialog: &mut Self, stamp, result, cx| {
                if dialog.steps.is_current(&stamp) {
                    dialog.read_landed(destination, then, result, cx);
                }
            },
        ));
    }

    /// The document as read again: at the destination it is moved; anywhere
    /// but where the dialog read it, it is stale. Where it was read, Move
    /// goes on to observe the destination and send; after `external_change`
    /// the destination is observed to tell a file put there from a changed
    /// source (fix 1, M2).
    fn read_landed(
        &mut self,
        destination: String,
        then: Then,
        result: ReadResult<ItemDto>,
        cx: &mut Context<Self>,
    ) {
        let item = match result {
            ReadResult::Ok(item) => item,
            ReadResult::Busy => return self.refused(Failure::of(ResultCode::Busy), cx),
            ReadResult::Failed(failure) => return self.refused(Failure::of(failure.code), cx),
        };
        match then {
            _ if item.path == destination => {
                // With no answer of this dialog's own, the views read the
                // repository again.
                let reload = matches!(then, Then::Send);
                self.done(reload, cx)
            }
            _ if item.path != self.source.path => self.refused(Failure::Stale, cx),
            Then::Send => {
                self.source.context = context_of(&item);
                self.observe_destination(destination, then, item.observation, cx);
            }
            Then::Judge => self.observe_destination(destination, then, None, cx),
        }
    }

    /// Observes the destination again: a file there refuses the move.
    /// Nothing there sends it, with the source's observation and this one;
    /// or, once the library answered `external_change`, says the source
    /// changed.
    fn observe_destination(
        &mut self,
        destination: String,
        then: Then,
        observation: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let stamp = self.steps.next(ViewScope::Create);
        self.step = Some(self.observe_at(
            stamp,
            destination.clone(),
            cx,
            move |dialog: &mut Self, stamp, result, cx| {
                if !dialog.steps.is_current(&stamp) {
                    return;
                }
                match result {
                    ReadResult::Ok(observed) if observed.exists => {
                        dialog.collision(destination, cx)
                    }
                    ReadResult::Ok(observed) => match then {
                        Then::Send => {
                            dialog.send(destination, observation, observed.observation, cx)
                        }
                        Then::Judge => dialog.refused(Failure::Stale, cx),
                    },
                    ReadResult::Busy => dialog.refused(Failure::of(ResultCode::Busy), cx),
                    ReadResult::Failed(failure) => dialog.refused(Failure::of(failure.code), cx),
                }
            },
        ));
    }

    /// Sends `document move` on the user lane.
    fn send(
        &mut self,
        destination: String,
        observation: Option<String>,
        destination_observation: String,
        cx: &mut Context<Self>,
    ) {
        let intent = Intent::DocumentMove {
            root: PathBuf::from(&self.source.root),
            id: self.source.id.clone(),
            observation,
            destination_path: PathBuf::from(&destination),
            destination_observation,
        };
        let request = self
            .services
            .worker()
            .submit(intent, Lane::User, None, None);
        self.sent = Some(Sent {
            request,
            taken: 0,
            destination,
        });
        cx.notify();
    }

    /// The request model changed: an attempt of the sent request that this
    /// dialog has not taken runs, or was answered.
    pub(super) fn followed(&mut self, cx: &mut Context<Self>) {
        let Some(sent) = &self.sent else {
            return;
        };
        let (attempt, state) = {
            let operations = self.services.operations().read(cx);
            let Some(request) = operations.request(sent.request) else {
                return;
            };
            if request.attempt <= sent.taken {
                return;
            }
            (request.attempt, request.state.clone())
        };
        let destination = sent.destination.clone();
        match state {
            RequestState::Queued | RequestState::Running { .. } => {
                // A retry of it: Move runs again.
                self.running = true;
                cx.notify();
                return;
            }
            RequestState::Finished(_, code, effects, _, message) => {
                let committed = effects.checkpoint == CheckpointEffect::Committed;
                self.answered(code, committed, message, destination, cx)
            }
            RequestState::Removed | RequestState::Dismissed | RequestState::NotSent(_) => {
                self.running = false;
                cx.notify();
            }
        }
        if let Some(sent) = &mut self.sent {
            sent.taken = attempt;
        }
    }

    /// An attempt's answer, `committed` when its checkpoint committed:
    /// the move is in the document's branch, whatever else is still owed
    /// (a hand-off the operation area offers).
    pub(super) fn answered(
        &mut self,
        code: ResultCode,
        committed: bool,
        message: String,
        destination: String,
        cx: &mut Context<Self>,
    ) {
        match code {
            _ if committed => self.done(false, cx),
            ResultCode::Ok | ResultCode::AlreadyApplied => self.done(false, cx),
            ResultCode::OccupiedPath => self.collision(destination, cx),
            ResultCode::ExternalChange => self.read_again(destination, Then::Judge, cx),
            code => self.refused(Failure::Library { message, code }, cx),
        }
    }

    /// A file is at `destination`: said under the File name while the
    /// fields still give it, and otherwise below them.
    fn collision(&mut self, destination: String, cx: &mut Context<Self>) {
        if matches!(&self.check, PathCheck::Free(path) if *path == destination) {
            self.checks.next(ViewScope::Create);
            self.observe = None;
            self.check = PathCheck::Held(destination);
            self.running = false;
            cx.notify();
        } else {
            self.refused(Failure::Held(destination), cx);
        }
    }

    fn refused(&mut self, failure: Failure, cx: &mut Context<Self>) {
        self.running = false;
        self.failure = Some(failure);
        cx.notify();
    }

    /// The document is moved. `reload`: no answer of this dialog's marked
    /// its repository changed, so it is marked now, and the tab and the
    /// tree read it again.
    fn done(&mut self, reload: bool, cx: &mut Context<Self>) {
        self.running = false;
        self.moved = true;
        self.failure = None;
        if reload {
            let root = self.source.root.clone();
            self.services.operations().update(cx, |operations, cx| {
                operations.mark_changed(root, true);
                cx.notify();
            });
        }
        cx.emit(Moved);
        cx.notify();
    }
}
