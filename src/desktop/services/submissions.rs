//! The comments this session posted, shared by every item tab, keyed by
//! repository and comment ID (design, Submissions).
//!
//! A post's library request ID is its comment ID (R-16), so the first post
//! after a relaunch is the same request. One comment posts once at a time:
//! a second Submit while it is posting is refused (P10). The services
//! event drain, not a tab, applies each answer, so an answer with no tab
//! open still lands (P9), and so does a retry or a Give up sent from the
//! operation area: its `Queued` marks the post posting again (`resent`).
//! Each answer is classified once, as it lands, under the attempt it
//! answered (`comment_outcome`).
//!
//! Nothing here derives `Debug`: an intent holds the comment's text.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use manyhands::{canonical::ItemId, drafts::DraftId, repository::RequestId};

use super::{
    requests::{DesktopRequestId, Intent, Lane},
    worker::{FinishedEnvelope, RecordState, WorkerHandle},
};
use crate::desktop::model::publication::{Attempt, CommentResult, comment_outcome};

/// One comment's post.
pub(crate) struct Submission {
    /// The worker request, for a retry. Its library request ID is the
    /// comment ID (R-16).
    pub request: DesktopRequestId,
    /// The comment's draft, retired by Lane D's drain hook.
    pub draft: Option<DraftId>,
    pub state: SubmissionState,
    /// Whether the attempt posting now re-sends a comment ID already
    /// sent: an adopted draft's first post after a relaunch, a submit of
    /// a comment this session already posted, a retry or a Give up (P4,
    /// P11, P15). Its answer is classified with it.
    resend: bool,
    /// The answer of the last attempt that finished, kept while a re-send
    /// of the same request (a retry, a Give up, a submit again) is
    /// posting: a re-send removed before it ran brings it back, so the
    /// post is never forgotten once an attempt answered (P2, P7, P15).
    previous: Option<SubmissionState>,
}

impl Submission {
    /// Marks the post posting again under the same request, a re-send,
    /// keeping the last finished answer. Marking it twice, as a retry of
    /// this registry and then its `Queued` do, is marking it once.
    fn resend(&mut self) {
        self.resend = true;
        let previous = std::mem::replace(&mut self.state, SubmissionState::Posting);
        if matches!(previous, SubmissionState::Finished { .. }) {
            self.previous = Some(previous);
        }
    }
}

/// Where a post stands.
pub(crate) enum SubmissionState {
    /// Queued or running: a second Submit is refused.
    Posting,
    /// Its last attempt's answer, and that answer classified under the
    /// attempt it answered: whether it re-sent the comment ID, where its
    /// record stood after it, and whether it was a Give up's.
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 13.
    Finished {
        envelope: FinishedEnvelope,
        result: CommentResult,
    },
}

/// What a finished attempt of a post leaves for the services drain: the
/// draft to retire once the comment is saved (P9, Lane D's hook).
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 13.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Posted {
    pub draft: Option<DraftId>,
    /// Whether its checkpoint committed: the comment is saved.
    pub saved: bool,
}

/// The session's posts.
pub(crate) struct Submissions {
    worker: WorkerHandle,
    posts: BTreeMap<(PathBuf, ItemId), Submission>,
}

impl Submissions {
    pub(crate) fn new(worker: WorkerHandle) -> Self {
        Self {
            worker,
            posts: BTreeMap::new(),
        }
    }

    /// The post of `comment_id` in `repository`, if this session made one.
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 13.
    pub(crate) fn get(&self, repository: &Path, comment_id: &ItemId) -> Option<&Submission> {
        self.posts.get(&(repository.to_owned(), comment_id.clone()))
    }

    /// Posts `intent`, a comment, under its comment ID as the request ID
    /// (R-16), with `draft` to retire on a committed checkpoint. `resend`
    /// when the comment ID may have been sent before, as an adopted
    /// draft's first post after a relaunch may (P4, P11); a comment this
    /// session already posted is a re-send whatever it says. `None`, and
    /// nothing sent, while that comment is posting (P10), or for any other
    /// intent.
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 13.
    pub(crate) fn submit(
        &mut self,
        intent: Intent,
        draft: Option<DraftId>,
        resend: bool,
    ) -> Option<DesktopRequestId> {
        let key = key_of(&intent)?;
        if self.posting(&key) {
            return None;
        }
        let request_id = RequestId::parse(&key.1.to_string()).ok()?;
        let request = self
            .worker
            .submit_with_request_id(intent, Lane::User, request_id);
        // The same comment ID is the same request (R-16): an answer it
        // already had is kept until this one lands.
        let earlier = self.posts.remove(&key);
        let resend = resend || earlier.is_some();
        let previous = earlier
            .map(|submission| submission.state)
            .filter(|state| matches!(state, SubmissionState::Finished { .. }));
        self.posts.insert(
            key,
            Submission {
                request,
                draft,
                state: SubmissionState::Posting,
                resend,
                previous,
            },
        );
        Some(request)
    }

    /// Sends the post of `comment_id` in `repository` again as the same
    /// request. False, and nothing sent, when this session has no such
    /// post, or the worker leaves the request alone (queued or running,
    /// its Give up among them, or quitting).
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 13.
    pub(crate) fn retry(&mut self, repository: &Path, comment_id: &ItemId) -> bool {
        let key = (repository.to_owned(), comment_id.clone());
        let Some(submission) = self.posts.get_mut(&key) else {
            return false;
        };
        if !self.worker.retry(submission.request, None) {
            return false;
        }
        submission.resend();
        true
    }

    /// Gives up sharing the post of `comment_id` in `repository` (decision
    /// 8): the worker's Give up of its request. The post is posting while
    /// it runs, so a Submit of the comment is refused (P10). False, and
    /// nothing sent, when this session has no such post or the worker
    /// refuses it (queued or running, or quitting).
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 13.
    pub(crate) fn give_up(&mut self, repository: &Path, comment_id: &ItemId) -> bool {
        let key = (repository.to_owned(), comment_id.clone());
        let Some(submission) = self.posts.get_mut(&key) else {
            return false;
        };
        if !self.worker.give_up(submission.request) {
            return false;
        }
        submission.resend();
        true
    }

    /// Forgets the post of `old_comment_id` in the repository `intent`
    /// posts to, and posts `intent` under `new_comment_id`, and so a new
    /// request ID, with `draft`, the draft re-targeted to the new ID
    /// (P13). Another repository's comment of the old ID is not touched.
    /// Only where R-12 allows: the caller decides. `None`, and nothing
    /// changed, when `intent` is not a comment with `new_comment_id`, or
    /// either comment is posting.
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 13.
    pub(crate) fn restart(
        &mut self,
        old_comment_id: &ItemId,
        new_comment_id: &ItemId,
        intent: Intent,
        draft: Option<DraftId>,
    ) -> Option<DesktopRequestId> {
        let key = key_of(&intent)?;
        if key.1 != *new_comment_id {
            return None;
        }
        let old = (key.0.clone(), old_comment_id.clone());
        if self.posting(&old) || self.posting(&key) {
            return None;
        }
        self.posts.remove(&old);
        self.submit(intent, draft, false)
    }

    /// The worker queued the request `id` again, a retry or a Give up sent
    /// from anywhere, the operation area among them: a post of this
    /// session is posting again, as a re-send, so a Submit of it is
    /// refused while it runs (P10). True when a post of this session was
    /// its request.
    pub(crate) fn resent(&mut self, id: DesktopRequestId) -> bool {
        let Some(submission) = self.by_request(id) else {
            return false;
        };
        submission.resend();
        true
    }

    /// Applies the answer of the worker request `id`, from the services
    /// drain: classified under its attempt, with the record state and the
    /// Give up the worker kept for it. What it leaves for the drain when a
    /// post of this session was its request.
    pub(crate) fn finished(
        &mut self,
        id: DesktopRequestId,
        envelope: FinishedEnvelope,
    ) -> Option<Posted> {
        let record = self.worker.record_state(id).unwrap_or(RecordState::Unknown);
        let after_give_up = self.worker.gave_up(id).is_some();
        let submission = self.by_request(id)?;
        let attempt = Attempt {
            resend: submission.resend,
            record,
            after_give_up,
        };
        let result = comment_outcome(&envelope, attempt);
        let saved = result.outcome.is_saved();
        submission.state = SubmissionState::Finished { envelope, result };
        submission.previous = None;
        Some(Posted {
            draft: submission.draft,
            saved,
        })
    }

    /// The worker request `id` was removed from the queue, or not sent:
    /// that attempt never ran. A post whose earlier attempt answered goes
    /// back to that answer; one that never answered is forgotten, and the
    /// comment may be posted again. True when a post of this session was
    /// its request.
    pub(crate) fn ended(&mut self, id: DesktopRequestId) -> bool {
        let Some((key, submission)) = self
            .posts
            .iter_mut()
            .find(|(_, submission)| submission.request == id)
        else {
            return false;
        };
        match submission.previous.take() {
            Some(previous) => submission.state = previous,
            None => {
                let key = key.clone();
                self.posts.remove(&key);
            }
        }
        true
    }

    /// Whether the post at `key` is posting: queued or running here, or a
    /// request of its comment queued or running in the worker, which a
    /// retry from the operation area is before its `Queued` reaches the
    /// drain (P10).
    fn posting(&self, key: &(PathBuf, ItemId)) -> bool {
        self.posts
            .get(key)
            .is_some_and(|submission| matches!(submission.state, SubmissionState::Posting))
            || self
                .worker
                .in_flight(|intent| key_of(intent).as_ref() == Some(key))
    }

    fn by_request(&mut self, id: DesktopRequestId) -> Option<&mut Submission> {
        self.posts
            .values_mut()
            .find(|submission| submission.request == id)
    }
}

/// The repository and comment ID a comment intent posts to.
fn key_of(intent: &Intent) -> Option<(PathBuf, ItemId)> {
    match intent {
        Intent::CommentAdd {
            root, comment_id, ..
        } => Some((root.clone(), comment_id.clone())),
        _ => None,
    }
}

#[cfg(test)]
#[path = "submissions_tests.rs"]
mod tests;
