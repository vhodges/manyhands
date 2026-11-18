//! Where a post stands, as the tab finds out (design, R-8, P4, P5, P11,
//! P12, P16; changes 1, 2, 7, 8; decisions 23-25).
//!
//! - **Looking** (`look`): an index refresh, awaited; then, once it
//!   succeeded, the item's comment listing; and, when asked, the request
//!   record of the comment's ID (`show_request`, read as an Operations
//!   read). Nothing here sends a post.
//! - **An adopted draft** shows what this session knows of its post (a
//!   finished, unsaved answer: decision 25), else its record decides: one
//!   exists, so it was submitted and waits read-only for Retry (P11); none,
//!   so its first post is checked first (P4); unknown, Retry or Check
//!   again.
//! - **Check again** ends in a final state once it finds a record, never
//!   in another bare "may already be saved" (change 2,
//!   `publication::record::check_again`).

use gpui_kit::{Context, EntityId, Task, Window};
use manyhands::{
    canonical::ItemId,
    drafts::DraftId,
    repository::{IndexState, ReadError, RequestId},
    results::ResultCode,
};

use super::{
    SlotDraft,
    post::{
        MAY_STILL_BE_SAVED, MAYBE_SAVED, MISMATCH, NOT_SAVED_FINAL, RECORD_UNKNOWN, RESEND,
        discard_note, failed_state, stored_body,
    },
};
use crate::desktop::{
    model::publication::CommentOutcome,
    model::{
        publication::record::{Checked, Listing, RecordRead, check_again, envelope_of_record},
        stamp::{Stamp, ViewScope},
    },
    services::{
        ReadKind, ReadResult,
        requests::{Intent, Lane},
        submissions::SubmissionState,
        worker::RecordState,
    },
    view::item::{
        ItemPane,
        composer::{ComposerState, FailedActions},
    },
};

/// What a look found.
pub(super) struct Found {
    pub(super) listing: Listing,
    /// The comment's body, when any listing shows it.
    pub(super) listed_body: Option<String>,
    pub(super) record: Option<RecordRead>,
}

/// `show_request`'s answer as a record's state.
fn record_read(
    shown: ReadResult<manyhands::results::Envelope<manyhands::repository::RequestDto>>,
) -> RecordRead {
    let ReadResult::Ok(shown) = shown else {
        return RecordRead::Unknown;
    };
    match shown.data {
        Some(request) => match (request.state, request.result) {
            (manyhands::repository::RequestState::Accepted, _) => RecordRead::Accepted,
            (manyhands::repository::RequestState::Finished, Some(result)) => {
                RecordRead::Finished(result)
            }
            (manyhands::repository::RequestState::Finished, None) => RecordRead::Unknown,
        },
        None if shown.code == ResultCode::RequestNotFound => RecordRead::None,
        None => RecordRead::Unknown,
    }
}

impl ItemPane {
    /// Looks where the comment `comment` stands: a refresh first, then the
    /// listing, once it succeeded, and the record when `record`.
    pub(super) fn look(
        &mut self,
        comment: ItemId,
        record: bool,
        cx: &mut Context<Self>,
    ) -> Task<Found> {
        let root = self.post_root();
        let refresh =
            self.services
                .worker()
                .submit(Intent::IndexRefresh { root }, Lane::User, None, None);
        let refreshed = self.wait_for(refresh);
        cx.spawn(async move |pane, cx| {
            let refreshed = refreshed.await.unwrap_or(false);
            let Ok(read) = pane.update(cx, |pane, cx| {
                pane.read_post(comment, refreshed, record, cx)
            }) else {
                return Found {
                    listing: Listing::Unproven,
                    listed_body: None,
                    record: record.then_some(RecordRead::Unknown),
                };
            };
            read.await.unwrap_or(Found {
                listing: Listing::Unproven,
                listed_body: None,
                record: record.then_some(RecordRead::Unknown),
            })
        })
    }

    /// Reads the item's comment listing, when `listing`, and the record
    /// of `comment`, when `record`.
    #[allow(clippy::result_large_err)]
    pub(super) fn read_post(
        &mut self,
        comment: ItemId,
        listing: bool,
        record: bool,
        cx: &mut Context<Self>,
    ) -> futures::channel::oneshot::Receiver<Found> {
        let (sender, receiver) = futures::channel::oneshot::channel();
        let root = self.post_root();
        let item = self.item_key().map(|item| item.id);
        let stamp = Stamp {
            scope: ViewScope::Drafts,
            generation: 0,
        };
        self.services
            .read_job(
                stamp,
                move |reader| async move {
                    let mut found = Found {
                        listing: Listing::Unproven,
                        listed_body: None,
                        record: None,
                    };
                    if listing && let Some(item) = item {
                        let repo = reader
                            .call(ReadKind::Resolve, |service| {
                                service.resolve_repository(&root)
                            })
                            .await;
                        if let ReadResult::Ok(repo) = repo
                            && let ReadResult::Ok(list) = reader
                                .call(ReadKind::Comments, |service| {
                                    service.list_comments(&repo, &item)
                                })
                                .await
                        {
                            let id = comment.to_string();
                            let shown = list
                                .items
                                .iter()
                                .find(|entry| entry.id.as_deref() == Some(&id));
                            found.listed_body = shown.and_then(|entry| entry.body.clone());
                            if list.complete && list.index.state == IndexState::Current {
                                found.listing = if shown.is_some() {
                                    Listing::Listed
                                } else {
                                    Listing::NotListed
                                };
                            }
                        }
                    }
                    if record {
                        let request = RequestId::parse(&comment.to_string());
                        found.record = Some(match request {
                            Ok(request) => record_read(
                                reader
                                    .call(ReadKind::Operations, move |service| {
                                        Ok::<_, ReadError>(service.show_request(request))
                                    })
                                    .await,
                            ),
                            Err(_) => RecordRead::Unknown,
                        });
                    }
                    found
                },
                cx,
                move |_, _, found, _| {
                    let _ = sender.send(found);
                },
            )
            .detach();
        receiver
    }

    /// An adopted draft of `comment` in `composer`: what this session knows
    /// of its post, else its record, decides what it offers.
    pub(super) fn adopted_post(
        &mut self,
        composer: EntityId,
        comment: ItemId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let root = self.post_root();
        let known = self
            .services
            .submissions()
            .read(cx)
            .get(&root, &comment)
            .and_then(|post| match &post.state {
                SubmissionState::Finished { envelope, result } => Some((
                    failed_state(envelope, result),
                    matches!(result.outcome, CommentOutcome::Mismatched { .. })
                        && result.attempt.record == RecordState::Finished,
                )),
                SubmissionState::Posting => None,
            });
        let Some((_, slot)) = self.composers.find(composer) else {
            return;
        };
        if let Some((state, mismatched)) = known {
            // Decision 25: a post that finished unsaved shows its state.
            slot.post.sent = true;
            self.set_composer(composer, state, window, cx);
            if mismatched {
                self.mismatch_record(composer, window, cx);
            }
            return;
        }
        slot.post.busy = true;
        self.set_composer(composer, ComposerState::Checking, window, cx);
        let read = self.read_post(comment, false, true, cx);
        cx.spawn_in(window, async move |pane, cx| {
            let record = read
                .await
                .ok()
                .and_then(|found| found.record)
                .unwrap_or(RecordRead::Unknown);
            let _ = pane.update_in(cx, |pane, window, cx| {
                let Some((_, slot)) = pane.composers.find(composer) else {
                    return;
                };
                slot.post.busy = false;
                let state = match record {
                    RecordRead::None => {
                        slot.post.unchecked = true;
                        ComposerState::Idle
                    }
                    RecordRead::Accepted | RecordRead::Finished(_) => {
                        slot.post.resend_pending = true;
                        failed(RESEND, retry_copy_discard(), None)
                    }
                    RecordRead::Unknown => {
                        slot.post.resend_pending = true;
                        failed(
                            RECORD_UNKNOWN,
                            FailedActions {
                                retry: true,
                                check_again: true,
                                ..Default::default()
                            },
                            None,
                        )
                    }
                };
                pane.set_composer(composer, state, window, cx);
            });
        })
        .detach();
    }

    /// Check again: a refresh, the listing and the record, and a final
    /// answer once a record is found (change 2).
    pub(super) fn check_again(
        &mut self,
        composer: EntityId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let root = self.post_root();
        // A new comment held for its old one checks the old one.
        let held = self
            .composers
            .find(composer)
            .and_then(|(_, slot)| slot.post.held.then(|| slot.post.from.clone()).flatten());
        if let Some(old) = held {
            self.check_overtaken(composer, old, window, cx);
            return;
        }
        if self.refused_in_flight(composer, window, cx) {
            return;
        }
        let drafts = self.services.drafts().clone();
        let Some((_, slot)) = self.composers.find(composer) else {
            return;
        };
        let Some(comment) = slot.target.as_ref().map(|target| target.comment_id.clone()) else {
            return;
        };
        if slot.post.busy {
            return;
        }
        slot.post.busy = true;
        let text = slot.composer.read(cx).text(cx);
        let draft = match slot.draft {
            SlotDraft::Begun(id) => Some(id),
            _ => None,
        };
        let last = self
            .services
            .submissions()
            .read(cx)
            .get(&root, &comment)
            .and_then(|post| match &post.state {
                SubmissionState::Finished { envelope, .. } => {
                    Some((envelope.code, discard_note(envelope, RecordState::Accepted)))
                }
                SubmissionState::Posting => None,
            });
        let (last_code, note) = last.unwrap_or((ResultCode::Ok, None));
        self.set_composer(composer, ComposerState::Checking, window, cx);
        let look = self.look(comment.clone(), true, cx);
        cx.spawn_in(window, async move |pane, cx| {
            let found = look.await;
            let record = found.record.unwrap_or(RecordRead::Unknown);
            let checked = check_again(&record, found.listing, last_code);
            // Decision 24 retires only a draft whose text is the listed
            // comment's, byte for byte (R-19).
            let same = match checked {
                Checked::Listed => {
                    let body = match draft {
                        Some(id) => stored_body(&drafts, id, cx).await.unwrap_or(text),
                        None => text,
                    };
                    found.listed_body.as_deref() == Some(body.as_str())
                }
                _ => false,
            };
            let _ = pane.update_in(cx, |pane, window, cx| {
                if let Some((_, slot)) = pane.composers.find(composer) {
                    slot.post.busy = false;
                }
                pane.checked(
                    composer,
                    comment,
                    (draft, same),
                    record,
                    checked,
                    note,
                    window,
                    cx,
                );
            });
        })
        .detach();
    }

    /// What Check again found, applied.
    #[allow(clippy::too_many_arguments)]
    fn checked(
        &mut self,
        composer: EntityId,
        comment: ItemId,
        (draft, same): (Option<DraftId>, bool),
        record: RecordRead,
        checked: Checked,
        note: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let root = self.post_root();
        match checked {
            Checked::Saved(result) => {
                if let RecordRead::Finished(stored) = &record {
                    let envelope = Box::new(envelope_of_record(stored));
                    self.services.submissions().update(cx, |posts, cx| {
                        posts.settle(&root, &comment, envelope, result, cx)
                    });
                }
                if let Some(draft) = draft {
                    self.retire_post_draft(comment, draft, cx);
                }
                self.clear_slot(composer, window, cx);
            }
            Checked::Listed if !same => {
                // Listed with other text: a mismatch, the text kept (R-19).
                let actions = FailedActions {
                    copy_text: true,
                    discard: true,
                    ..Default::default()
                };
                self.set_composer(composer, failed(MISMATCH, actions, None), window, cx);
            }
            Checked::Listed => {
                // Decision 24: shown in the thread, claiming nothing.
                self.services
                    .submissions()
                    .update(cx, |posts, cx| posts.forget(&root, &comment, cx));
                if let Some(draft) = draft {
                    self.services
                        .drafts()
                        .update(cx, |registry, cx| registry.retire_comment(draft, cx))
                        .detach();
                }
                self.clear_slot(composer, window, cx);
            }
            Checked::Final { retry, post_as_new } => {
                let actions = FailedActions {
                    retry,
                    post_as_new,
                    ..retry_copy_discard()
                };
                let note = match &record {
                    RecordRead::Finished(stored) => {
                        discard_note(&envelope_of_record(stored), RecordState::Finished)
                    }
                    RecordRead::Accepted => Some(MAY_STILL_BE_SAVED.to_owned()),
                    _ => note,
                };
                let state = failed(NOT_SAVED_FINAL, actions, note);
                self.set_composer(composer, state, window, cx);
            }
            Checked::Maybe { post_as_new } => {
                // R-19: never trapped behind Check again.
                let actions = FailedActions {
                    check_again: true,
                    post_as_new,
                    copy_text: true,
                    discard: true,
                    ..Default::default()
                };
                let note = Some(MAY_STILL_BE_SAVED.to_owned());
                self.set_composer(composer, failed(MAYBE_SAVED, actions, note), window, cx);
            }
        }
    }
}

/// Retry's companions on a post that is not saved: Copy text and Discard.
fn retry_copy_discard() -> FailedActions {
    FailedActions {
        retry: true,
        copy_text: true,
        discard: true,
        ..Default::default()
    }
}

fn failed(message: &str, actions: FailedActions, discard_note: Option<String>) -> ComposerState {
    ComposerState::Failed {
        message: message.to_owned(),
        actions,
        discard_note,
    }
}
