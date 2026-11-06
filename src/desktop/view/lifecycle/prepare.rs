//! The dialog up to its confirm stage (D5 design, A2): the save the tab
//! makes first, the prepare, and what a prepare that gave no preview says,
//! blocked or held, with its way out (L1).

use gpui_kit::{
    AnyElement, App, Context, IntoElement, ParentElement, Styled, Window,
    component::{ActiveTheme, v_flex},
};
use manyhands::{
    repository::{
        ConfirmationId, LifecycleBlocker, LifecycleWayOut, OperationId, PrepareDataDto, RequestId,
        lifecycle_prepare_way_out,
    },
    results::{Envelope, ResultCode},
};

use super::{
    CONTINUE, LifecycleDialog, NOT_SENT, OPEN_FOLDER, Phase, RESUME, STILL_RUNNING, button,
    share_main, sharing_main, text,
};
use crate::desktop::{
    model::{
        lifecycle_lock::LockKey,
        operations::primary_subject,
        stamp::{Stamp, ViewScope},
    },
    services::{
        KEY_STORE_UNAVAILABLE, lifecycle_of,
        requests::{Intent, Lane, NotBuilt},
    },
    view::{confirm::SECRET_REQUIRED, item::SaveFirst},
};

/// What a dirty primary branch is called, at prepare or in a stop at
/// execute (A2): `primary` names it (R-35).
pub(crate) fn primary_dirty(primary: &str) -> String {
    format!(
        "{} has unsaved changes outside Manyhands.",
        primary_subject(primary)
    )
}

/// A blocker in words; `primary` names the primary branch (R-35).
pub(crate) fn blocker_words(blocker: LifecycleBlocker, primary: &str) -> String {
    use LifecycleBlocker::{
        AlreadyClosed, CommentPublicationPending, ConflictPending, ContextBehindRemote,
        ContextConflicted, ContextDirty, ContextMissing, ContextNonconforming,
        DocumentInvalidAfterIntegration, HistoryUnknown, IdentityRequired, IntegrationConflict,
        LocalOperationPending, NotADocument, NotATicket, PrimaryAheadOfRemote, PrimaryConflicted,
        PrimaryDirty, PrimaryMissing, PrimaryNotCheckedOut, PrimaryReopened, RemoteContextDeleted,
        TicketInvalidAfterClosure,
    };
    let subject = primary_subject(primary);
    let words = match blocker {
        NotADocument => "This item is not a document.",
        NotATicket => "This item is not a ticket.",
        ContextMissing => "This item has no working copy of its own yet.",
        ContextNonconforming => "This item's working copy holds a file Manyhands cannot read.",
        ContextDirty => "This item's working copy has changes that are not saved.",
        ContextConflicted => "This item's working copy has a conflict to resolve.",
        ConflictPending => "A conflict on this item is waiting to be resolved.",
        LocalOperationPending => "Another change to this item has not finished.",
        CommentPublicationPending => "A comment on this item has not finished posting.",
        PrimaryMissing => return format!("{subject} is not known on this computer yet."),
        PrimaryNotCheckedOut => {
            return format!("{subject} is not checked out in the repository's folder.");
        }
        PrimaryDirty => return primary_dirty(primary),
        PrimaryConflicted => return format!("{subject} has a conflict to resolve."),
        PrimaryAheadOfRemote => return format!("{subject} has changes that are not shared yet."),
        ContextBehindRemote => "This item's working copy is behind its shared copy.",
        IntegrationConflict => return format!("This item's changes conflict with {primary}."),
        RemoteContextDeleted => "This item's shared copy was deleted.",
        HistoryUnknown => "Whether this item's shared copy still stands is not known.",
        AlreadyClosed => "This ticket is already closed.",
        PrimaryReopened => return format!("{subject} has reopened this ticket."),
        DocumentInvalidAfterIntegration => {
            return format!("The document would not be valid once merged into {primary}.");
        }
        TicketInvalidAfterClosure => "The ticket would not be valid once closed.",
        IdentityRequired => "Your name and email are not set for this repository.",
    };
    words.to_owned()
}

/// A way out that is guidance, not a button, in words; `None` for one the
/// dialog offers as a button. The repository's details are both: these
/// words, and Open repository details.
pub(super) fn guidance(
    way_out: &LifecycleWayOut,
    item: &'static str,
    primary: &str,
) -> Option<String> {
    match way_out {
        LifecycleWayOut::PollOnce => {
            Some("Check origin for others' changes, then prepare again.".to_owned())
        }
        LifecycleWayOut::SaveItem { .. } => Some(format!(
            "Edit and save this {item} once to give it a working copy, then prepare again."
        )),
        // Repair locally… is a button too (`repair.rs`).
        LifecycleWayOut::RepairLocally { .. } => Some(format!(
            "Bring {primary} into this {item}'s working copy, then prepare again."
        )),
        LifecycleWayOut::RepairOutside => Some(format!(
            "Fix the conflict with {primary} outside Manyhands, then prepare again."
        )),
        LifecycleWayOut::Inspect => {
            Some("The repository's details in Repositories say more.".to_owned())
        }
        LifecycleWayOut::SharePrimary
        | LifecycleWayOut::RetryRequest { .. }
        | LifecycleWayOut::ResumeOperation { .. }
        | LifecycleWayOut::SendAgain
        | LifecycleWayOut::CancelThenSendAgain { .. }
        | LifecycleWayOut::ResolveChildConflict { .. }
        | LifecycleWayOut::Republish { .. } => None,
    }
}

impl LifecycleDialog {
    /// The tab saving first moved on: once saved, the dialog prepares;
    /// any other answer closes it, and the tab shows that save's outcome
    /// (decision 13, P1).
    pub(super) fn saving_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !matches!(self.phase, Phase::Saving) {
            return;
        }
        let state = match self.saving.as_ref().and_then(|pane| pane.upgrade()) {
            Some(pane) => pane.read(cx).save_first_state(cx),
            None => SaveFirst::Failed,
        };
        match state {
            SaveFirst::Saving => {}
            SaveFirst::Saved => {
                self.saving = None;
                self.prepare(cx);
            }
            SaveFirst::Failed => {
                self.saving = None;
                self.close(window, cx);
            }
        }
    }

    /// Prepares the command afresh: nothing runs.
    pub(super) fn prepare(&mut self, cx: &mut Context<Self>) {
        self.set_phase(Phase::Preparing, cx);
        let services = self.services.clone();
        let mutation = match self.intent.build(None, || services.key_store()) {
            Ok(mutation) => mutation,
            Err(NotBuilt::KeyStoreUnavailable) => {
                self.set_phase(Phase::Message(KEY_STORE_UNAVAILABLE), cx);
                cx.notify();
                return;
            }
            Err(NotBuilt::MissingSecret) => {
                self.set_phase(Phase::Message(SECRET_REQUIRED), cx);
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
        let way_out = lifecycle_prepare_way_out(self.heading.kind, &envelope);
        let Envelope {
            code,
            message,
            data,
            ..
        } = envelope;
        let phase = match data {
            Some(PrepareDataDto::Preview(preview)) if code == ResultCode::Ok => {
                match ConfirmationId::parse(&preview.confirmation_id) {
                    Ok(confirmation) => Phase::Confirm {
                        preview,
                        confirmation,
                    },
                    Err(_) => Phase::Answered {
                        code: ResultCode::InternalError,
                        message: ResultCode::InternalError.message().to_owned(),
                    },
                }
            }
            Some(PrepareDataDto::LifecycleBlocked(blocked)) => Phase::Blocked {
                code,
                blockers: blocked.blockers,
                way_out,
                shared: false,
            },
            None if code == ResultCode::RecoveryRequired
                && matches!(
                    way_out.as_slice(),
                    [LifecycleWayOut::RetryRequest { .. }
                        | LifecycleWayOut::ResumeOperation { .. }]
                ) =>
            {
                Phase::Held {
                    way_out: way_out.into_iter().next().expect("matched one"),
                }
            }
            _ => Phase::Answered { code, message },
        };
        self.set_phase(phase, cx);
        // What the tab shows may be gone, a ticket already closed among it:
        // the views that show the repository read it again. Busy says
        // nothing changed.
        if !matches!(self.phase, Phase::Confirm { .. })
            && code != ResultCode::Busy
            && let Some(root) = self.root().and_then(|root| root.to_str())
        {
            let root = root.to_owned();
            self.services.operations().update(cx, |operations, cx| {
                operations.mark_changed(root, true);
                cx.notify();
            });
        }
        cx.notify();
    }

    /// Whether the item's lifecycle is queued or running in this process,
    /// or a resume of the operation that holds it: Continue and Resume are
    /// not offered meanwhile (P22).
    fn in_flight(&self) -> bool {
        let held = match &self.phase {
            Phase::Held {
                way_out: LifecycleWayOut::ResumeOperation { operation_id },
            } => Some(operation_id.as_str()),
            _ => None,
        };
        self.in_flight_with(held)
    }

    /// As `in_flight`, with a resume of `held`, the operation a way out
    /// names, counted too.
    pub(super) fn in_flight_with(&self, held: Option<&str>) -> bool {
        let (Some(root), Some(id)) = (self.root().and_then(|root| root.to_str()), self.item_id())
        else {
            return false;
        };
        let key = LockKey::new(root, &id.to_string());
        let held: Vec<String> = held.into_iter().map(str::to_owned).collect();
        self.services
            .worker()
            .in_flight(|intent| lifecycle_of(intent, &key, &held))
    }

    pub(super) fn item_id(&self) -> Option<&manyhands::canonical::ItemId> {
        match &self.intent {
            Intent::TicketClose { id, .. } | Intent::DocumentPromote { id, .. } => Some(id),
            _ => None,
        }
    }

    /// Continue: the request holding the item, sent again under its own
    /// request ID, with no confirmation (its accepted record needs none).
    fn continue_held(&mut self, request: &str, cx: &mut Context<Self>) {
        if self.in_flight() {
            return;
        }
        self.following_anew();
        let phase = match RequestId::parse(request) {
            Ok(request) => Phase::Submitted(self.services.worker().submit_with_request_id(
                self.intent.clone(),
                Lane::User,
                request,
            )),
            Err(_) => Phase::Message(NOT_SENT),
        };
        self.set_phase(phase, cx);
        cx.notify();
    }

    /// Resume: the operation holding the item, or the one an outcome's way
    /// out names, resumed.
    pub(super) fn resume_held(&mut self, operation: &str, cx: &mut Context<Self>) {
        if self.in_flight_with(Some(operation)) {
            return;
        }
        self.following_anew();
        let (Some(root), Ok(operation_id)) = (self.root(), OperationId::parse(operation)) else {
            self.set_phase(Phase::Message(NOT_SENT), cx);
            cx.notify();
            return;
        };
        let intent = Intent::OperationResume {
            root: root.to_path_buf(),
            operation_id,
        };
        let request = self
            .services
            .worker()
            .submit(intent, Lane::User, None, None);
        self.set_phase(Phase::Submitted(request), cx);
        cx.notify();
    }

    /// Share main, from a blocked prepare: `repo sync` of the repository.
    fn share_main(&mut self, cx: &mut Context<Self>) {
        let Some(root) = self.root().map(|root| root.to_path_buf()) else {
            return;
        };
        if let Phase::Blocked { shared, .. } = &mut self.phase {
            if *shared {
                return;
            }
            *shared = true;
            self.services
                .worker()
                .submit(Intent::RepoSync { root }, Lane::User, None, None);
            cx.notify();
        }
    }

    /// The held item's one way on, as a button; while the item's lifecycle
    /// is in flight, that it is still running.
    pub(super) fn held_action(
        &self,
        way_out: &LifecycleWayOut,
        cx: &Context<Self>,
    ) -> Option<AnyElement> {
        if self.in_flight() {
            return Some(
                text("lifecycle-still-running", STILL_RUNNING)
                    .text_sm()
                    .into_any_element(),
            );
        }
        match way_out {
            LifecycleWayOut::RetryRequest { request_id } => {
                let request = request_id.clone();
                let go = self.on(cx, move |dialog, _, cx| dialog.continue_held(&request, cx));
                Some(button("lifecycle-continue", CONTINUE, true, go))
            }
            LifecycleWayOut::ResumeOperation { operation_id } => {
                let operation = operation_id.clone();
                let go = self.on(cx, move |dialog, _, cx| dialog.resume_held(&operation, cx));
                Some(button("lifecycle-resume", RESUME, true, go))
            }
            _ => None,
        }
    }

    /// A blocked prepare's rows: each blocker in words, then each way out
    /// that is guidance, and its code; and its buttons: Repair locally…,
    /// Open folder for a dirty main and Share main when it is a way out.
    pub(super) fn blocked_body(
        &self,
        code: ResultCode,
        blockers: &[LifecycleBlocker],
        way_out: &[LifecycleWayOut],
        shared: bool,
        cx: &Context<Self>,
    ) -> (impl IntoElement + use<>, Vec<AnyElement>) {
        let item = match self.heading.kind {
            manyhands::repository::ItemDtoKind::Ticket => "ticket",
            manyhands::repository::ItemDtoKind::Document => "document",
        };
        let muted = cx.theme().muted_foreground;
        let primary = self.primary();
        let guidance: Vec<String> = way_out
            .iter()
            .filter_map(|step| guidance(step, item, &primary))
            .collect();
        let rows = v_flex()
            .gap_1()
            .child(text("lifecycle-lead", super::BLOCKED_LEAD).text_sm())
            .children(blockers.iter().enumerate().map(|(ix, blocker)| {
                text(("lifecycle-blocker", ix), blocker_words(*blocker, &primary)).text_sm()
            }))
            .children(guidance.into_iter().enumerate().map(|(ix, words)| {
                text(("lifecycle-way-out", ix), words)
                    .text_sm()
                    .text_color(muted)
            }))
            .children(
                shared.then(|| text("lifecycle-sharing-main", sharing_main(&primary)).text_sm()),
            )
            .child(
                text("lifecycle-code", code.as_str())
                    .text_xs()
                    .text_color(muted),
            );
        // Not while the item's lifecycle is in flight (P22), as for a run.
        let mut actions = self.repair_buttons(way_out, self.in_flight_with(None), cx);
        if blockers.contains(&LifecycleBlocker::PrimaryDirty)
            && let Some(root) = self.root().map(|root| root.to_path_buf())
        {
            actions.push(button(
                "lifecycle-open-folder",
                OPEN_FOLDER,
                false,
                move |_, _, cx: &mut App| cx.open_with_system(&root),
            ));
        }
        if way_out.contains(&LifecycleWayOut::SharePrimary) && !shared {
            let share = self.on(cx, |dialog, _, cx| dialog.share_main(cx));
            actions.push(button(
                "lifecycle-share-main",
                share_main(&primary),
                false,
                share,
            ));
        }
        if way_out.contains(&LifecycleWayOut::Inspect) {
            actions.push(self.inspect_button(cx));
        }
        (rows, actions)
    }
}
