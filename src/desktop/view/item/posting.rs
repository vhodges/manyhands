//! A comment's posting as the thread shows it (design, Lane D, Posting):
//! the pending row of each post being sent, from its text, until its
//! outcome; a saved comment's "Saved on this computer" note, or its "not
//! shared yet" chip with the controls its outcome offers. The chip lives
//! in `Submissions` for the session, so it survives reloads and a closed
//! tab. What a composer posts and how its outcome applies is
//! `composer::place::post`'s.
//!
//! The tab hears `Submissions` change, and waits here on the requests its
//! checks queue (an index refresh before a listing, P4, P5). A post of the
//! item whose checkpoint committed reloads the tab (fact 28); one that
//! left discovery `pending` queues a background index refresh first, the
//! header saying "Updating the list…" until it ends, and its end reloads
//! the tab (design, Posting). Nothing here derives `Debug`: a pending row
//! holds the comment's text.

use std::{collections::HashSet, path::PathBuf};

use futures::channel::oneshot;
use gpui_kit::{
    AnyElement, ClickEvent, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Subscription, TestSupportExt, Window,
    component::{Sizable, button::Button, h_flex},
    div, px,
};
use manyhands::{
    canonical::ItemId,
    results::{CheckpointEffect, DiscoveryEffect, Outcome},
};

use super::{ItemPane, ItemRoute, pill::UPDATING_LIST, text, tokens::item_colors};
use crate::desktop::{
    model::{
        navigation::OpenTarget,
        operations::Operations,
        publication::{CommentOffer, CommentOutcome},
    },
    services::{
        Services,
        requests::{DesktopRequest, DesktopRequestId, Intent, Lane, RequestState},
        submissions::SubmissionState,
        worker::RecordState,
    },
};

/// A saved comment that is not shared (design, Posting).
pub(crate) const NOT_SHARED: &str = "Saved on this computer \u{b7} not shared yet";
/// A comment saved with no publication remote.
pub(crate) const SAVED_HERE: &str = "Saved on this computer";

/// The tab's posting: it hears `Submissions`, and the requests its checks
/// wait on.
pub(super) struct Posting {
    /// Each request a check waits on, and who hears whether it succeeded.
    waiters: Vec<(DesktopRequestId, oneshot::Sender<bool>)>,
    /// The posts' attempts whose answers this tab has met, by request and
    /// attempt: each that left discovery pending refreshes the index once.
    met: HashSet<(DesktopRequestId, u32)>,
    /// The index refresh a post's pending discovery queued, while it runs.
    updating: Option<DesktopRequestId>,
    _submissions: Subscription,
}

impl Posting {
    pub(super) fn new(services: &Services, cx: &mut Context<ItemPane>) -> Self {
        // Answers that landed before the tab opened are not this tab's to
        // follow: its own first read comes after them.
        let met = services
            .operations()
            .read(cx)
            .requests()
            .iter()
            .filter(|request| matches!(request.intent, Intent::CommentAdd { .. }))
            .map(|request| (request.id, request.attempt))
            .collect();
        Self {
            waiters: Vec::new(),
            met,
            updating: None,
            _submissions: cx.observe(services.submissions(), |_, _, cx| cx.notify()),
        }
    }
}

/// Whether `request` posts a comment on `target`'s item and its answer
/// left the comment committed with discovery pending.
fn left_discovery_pending(request: &DesktopRequest, target: &OpenTarget) -> bool {
    let OpenTarget::Item { repo, id, .. } = target else {
        return false;
    };
    let Intent::CommentAdd { root, id: item, .. } = &request.intent else {
        return false;
    };
    root.to_str() == Some(repo.as_str())
        && item == id
        && matches!(&request.state, RequestState::Finished(_, _, effects, ..)
            if effects.checkpoint == CheckpointEffect::Committed
                && effects.discovery == DiscoveryEffect::Pending)
}

impl ItemPane {
    /// The repository's root as `Submissions` keys it.
    pub(super) fn post_root(&self) -> PathBuf {
        PathBuf::from(self.repo())
    }

    /// Answers once the request `id` has ended: whether it succeeded.
    pub(super) fn wait_for(&mut self, id: DesktopRequestId) -> oneshot::Receiver<bool> {
        let (sender, receiver) = oneshot::channel();
        self.posting.waiters.push((id, sender));
        receiver
    }

    /// The requests a check waits on that have ended are answered; a post
    /// of the item that left discovery pending refreshes the index, and
    /// that refresh's end is followed (`index_updated`).
    pub(super) fn posts_progressed(&mut self, cx: &mut Context<Self>) {
        self.waiters_progressed(cx);
        let operations = self.services.operations().read(cx);
        let pending = self.newly_pending(operations);
        if pending && self.posting.updating.is_none() {
            let root = self.post_root();
            let id = self.services.worker().submit(
                Intent::IndexRefresh { root },
                Lane::Background,
                None,
                None,
            );
            self.posting.updating = Some(id);
            cx.notify();
        }
        self.index_updated(cx);
    }

    /// Whether a post of the item answered since the tab last looked, its
    /// checkpoint committed and its discovery pending. Each attempt's
    /// answer is met once.
    fn newly_pending(&mut self, operations: &Operations) -> bool {
        let target = self.model.target();
        let mut pending = false;
        for request in operations.requests() {
            if !matches!(request.state, RequestState::Finished(..))
                || !matches!(request.intent, Intent::CommentAdd { .. })
                || !self.posting.met.insert((request.id, request.attempt))
            {
                continue;
            }
            pending |= left_discovery_pending(request, target);
        }
        pending
    }

    /// The post's index refresh has ended: the header stops saying so. A
    /// finished one reloads every tab of the repository (`changes_items`);
    /// one removed or not sent reloads this tab here.
    fn index_updated(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.posting.updating else {
            return;
        };
        let state = self
            .services
            .operations()
            .read(cx)
            .request(id)
            .map(|request| request.state.clone());
        match state {
            None | Some(RequestState::Queued | RequestState::Running { .. }) => {}
            Some(RequestState::Finished(..)) => {
                self.posting.updating = None;
                cx.notify();
            }
            Some(_) => {
                self.posting.updating = None;
                self.reload(cx);
            }
        }
    }

    /// "Updating the list…" while a post's index refresh runs.
    pub(super) fn updating_list(&self) -> Option<String> {
        self.posting.updating.map(|_| UPDATING_LIST.to_owned())
    }

    /// The requests a check waits on that have ended are answered.
    fn waiters_progressed(&mut self, cx: &mut Context<Self>) {
        let operations = self.services.operations().read(cx);
        let mut waiting = Vec::new();
        for (id, sender) in self.posting.waiters.drain(..) {
            match operations.request(id).map(|request| &request.state) {
                None | Some(RequestState::Queued | RequestState::Running { .. }) => {
                    waiting.push((id, sender));
                }
                Some(RequestState::Finished(outcome, ..)) => {
                    let _ = sender.send(matches!(outcome, Outcome::Success | Outcome::Noop));
                }
                Some(_) => {
                    let _ = sender.send(false);
                }
            }
        }
        self.posting.waiters = waiting;
    }

    /// The pending rows of the new comments being sent, after the threads
    /// and before the composers.
    pub(super) fn posting_rows(
        &self,
        _window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let rows = self.pending_posts(None, cx);
        if rows.is_empty() {
            return None;
        }
        Some(
            gpui_kit::component::v_flex()
                .w_full()
                .gap(px(8.))
                .children(
                    rows.into_iter()
                        .map(|(id, body)| pending_row(&id, body, cx)),
                )
                .into_any_element(),
        )
    }

    /// What a listed comment shows of its post: "Saved on this computer",
    /// or the chip and its controls; nothing for a comment this session
    /// did not post, or one shared.
    pub(super) fn comment_marks(&self, id: &str, cx: &mut Context<Self>) -> Option<AnyElement> {
        let comment: ItemId = id.parse().ok()?;
        let submissions = self.services.submissions().read(cx);
        let post = submissions.get(&self.post_root(), &comment)?;
        let SubmissionState::Finished { result, .. } = &post.state else {
            return None;
        };
        // A share of the item published it since: the chip is cleared for
        // the session (decision 14).
        if post.shared_since() && result.outcome != CommentOutcome::SavedHere {
            return None;
        }
        let colors = item_colors(cx);
        match &result.outcome {
            CommentOutcome::SavedHere => Some(
                text(format!("comment-saved-here-{comment}"), SAVED_HERE)
                    .mt(px(4.))
                    .text_size(px(12.))
                    .text_color(colors.page_muted)
                    .into_any_element(),
            ),
            CommentOutcome::Shared | CommentOutcome::Mismatched { .. } => None,
            outcome if outcome.is_saved() => {
                let offers = result.offers();
                let mut chip = h_flex()
                    .id(format!("comment-chip-{comment}"))
                    .test_support()
                    .aria_label(NOT_SHARED)
                    .flex_wrap()
                    .items_center()
                    .gap(px(6.))
                    .mt(px(4.))
                    .child(
                        div()
                            .px(px(8.))
                            .py(px(2.))
                            .rounded_full()
                            .border_1()
                            .border_color(colors.page_amber)
                            .text_size(px(12.))
                            .text_color(colors.page_amber_text)
                            .child(NOT_SHARED),
                    );
                for offer in offers {
                    let Some((part, label)) = chip_control(offer) else {
                        continue;
                    };
                    let comment = comment.clone();
                    chip = chip.child(
                        div()
                            .id(format!("comment-chip-{comment}-{part}"))
                            .test_support()
                            .aria_label(label)
                            .child(
                                Button::new(format!("comment-chip-{comment}-{part}-button"))
                                    .label(label)
                                    .xsmall()
                                    .outline()
                                    .on_click(cx.listener(
                                        move |pane, _: &ClickEvent, window, cx| {
                                            pane.chip_pressed(&comment, offer, window, cx)
                                        },
                                    )),
                            ),
                    );
                }
                Some(chip.into_any_element())
            }
            _ => None,
        }
    }

    /// A chip's control on the post of `comment`: the same request
    /// (Retry sharing, Unlock… after clearing the declined key, R-3), Give
    /// up, a new share, or a route.
    fn chip_pressed(
        &mut self,
        comment: &ItemId,
        offer: CommentOffer,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let root = self.post_root();
        let submissions = self.services.submissions().clone();
        let worker = self.services.worker().clone();
        let Some((request, unlock_finished)) =
            submissions.read(cx).get(&root, comment).map(|post| {
                let finished_unlock = matches!(
                    &post.state,
                    SubmissionState::Finished { result, .. }
                        if result.outcome == CommentOutcome::SavedNeedsUnlock
                            && result.attempt.record == RecordState::Finished
                );
                (post.request, finished_unlock)
            })
        else {
            return;
        };
        match offer {
            CommentOffer::RetrySharing => {
                submissions.update(cx, |posts, _| posts.retry(&root, comment));
            }
            CommentOffer::Unlock => {
                worker.clear_declined_for(request);
                submissions.update(cx, |posts, _| posts.retry(&root, comment));
            }
            CommentOffer::GiveUp => {
                submissions.update(cx, |posts, _| posts.give_up(&root, comment));
            }
            CommentOffer::Share => {
                // A finished locked share clears its declined key first
                // (R-17).
                if unlock_finished {
                    worker.clear_declined_for(request);
                }
                self.share_item(window, cx);
            }
            CommentOffer::OpenSharingAndKeys => cx.emit(ItemRoute::SharingAndKeys),
            CommentOffer::CheckHost => cx.emit(ItemRoute::CheckHost {
                root,
                authority: None,
            }),
            CommentOffer::SetIdentity => cx.emit(ItemRoute::SetIdentity(root)),
            CommentOffer::CheckAgain => self.recheck_saved(comment.clone(), cx),
            CommentOffer::Retry
            | CommentOffer::EditText
            | CommentOffer::Discard
            | CommentOffer::CopyText => {}
        }
        cx.notify();
    }
}

/// A chip control's part of its element ID, and its words.
fn chip_control(offer: CommentOffer) -> Option<(&'static str, &'static str)> {
    Some(match offer {
        CommentOffer::RetrySharing => ("retry-sharing", "Retry sharing"),
        CommentOffer::Unlock => ("unlock", "Unlock\u{2026}"),
        CommentOffer::GiveUp => ("give-up", "Stop trying to share"),
        CommentOffer::Share => ("share", "Share"),
        CommentOffer::OpenSharingAndKeys => ("open-keys", "Open Sharing & keys"),
        CommentOffer::CheckHost => ("check-host", "Check host"),
        CommentOffer::SetIdentity => ("set-identity", "Set identity"),
        CommentOffer::CheckAgain => ("check-again", "Check again"),
        CommentOffer::Retry
        | CommentOffer::EditText
        | CommentOffer::Discard
        | CommentOffer::CopyText => return None,
    })
}

/// A post being sent, drawn from its text at 60% opacity, keyed by its
/// comment ID.
pub(crate) fn pending_row(id: &ItemId, body: String, cx: &gpui_kit::App) -> AnyElement {
    let colors = item_colors(cx);
    div()
        .id(format!("pending-comment-{id}"))
        .test_support()
        .w_full()
        .opacity(0.6)
        .px(px(14.))
        .py(px(12.))
        .bg(colors.page)
        .border_1()
        .border_color(colors.page_border)
        .rounded(px(8.))
        .text_size(px(14.))
        .child(body)
        .into_any_element()
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "posting_tests.rs"]
mod tests;
