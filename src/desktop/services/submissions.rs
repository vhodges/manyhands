//! The comments this session posted, shared by every item tab, keyed by
//! repository and comment ID (design, Submissions).
//!
//! A post's library request ID is its comment ID (R-16), so the first post
//! after a relaunch is the same request. One comment posts once at a time:
//! a second Submit while it is posting is refused (P10). The services
//! event drain, not a tab, applies each answer, so an answer with no tab
//! open still lands (P9).
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

/// One comment's post.
pub(crate) struct Submission {
    /// The worker request, for a retry. Its library request ID is the
    /// comment ID (R-16).
    pub request: DesktopRequestId,
    /// The comment's draft, retired by Lane D's drain hook.
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 13.
    pub draft: Option<DraftId>,
    pub state: SubmissionState,
    /// The answer of the last attempt that finished, kept while a re-send
    /// of the same request (a retry, a Give up, a submit again) is
    /// posting: a re-send removed before it ran brings it back, so the
    /// post is never forgotten once an attempt answered (P2, P7, P15).
    previous: Option<SubmissionState>,
}

impl Submission {
    /// Marks the post posting again under the same request, keeping the
    /// last finished answer.
    fn resend(&mut self) {
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
    /// Its last attempt's answer, and where its record stood after it:
    /// what Task 3's `comment_outcome` reads.
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 13.
    Finished {
        envelope: FinishedEnvelope,
        record: RecordState,
    },
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
    /// (R-16), with `draft` to retire on a committed checkpoint. `None`,
    /// and nothing sent, while that comment is posting (P10), or for any
    /// other intent.
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 13.
    pub(crate) fn submit(
        &mut self,
        intent: Intent,
        draft: Option<DraftId>,
    ) -> Option<DesktopRequestId> {
        let key = key_of(&intent)?;
        if self
            .posts
            .get(&key)
            .is_some_and(|submission| matches!(submission.state, SubmissionState::Posting))
        {
            return None;
        }
        let request_id = RequestId::parse(&key.1.to_string()).ok()?;
        let request = self
            .worker
            .submit_with_request_id(intent, Lane::User, request_id);
        // The same comment ID is the same request (R-16): an answer it
        // already had is kept until this one lands.
        let previous = self
            .posts
            .remove(&key)
            .map(|submission| submission.state)
            .filter(|state| matches!(state, SubmissionState::Finished { .. }));
        self.posts.insert(
            key,
            Submission {
                request,
                draft,
                state: SubmissionState::Posting,
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

    /// Forgets the post of `old_comment_id` and posts `intent` under
    /// `new_comment_id`, and so a new request ID, with `draft`, the draft
    /// re-targeted to the new ID (P13). Only where R-12 allows: the caller
    /// decides. `None`, and nothing changed, when `intent` is not a comment
    /// with `new_comment_id`, or either comment is posting.
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
        let old = self
            .posts
            .iter()
            .find_map(|(key, _)| (key.1 == *old_comment_id).then(|| key.clone()));
        if old.iter().chain([&key]).any(|key| {
            self.posts
                .get(key)
                .is_some_and(|submission| matches!(submission.state, SubmissionState::Posting))
        }) {
            return None;
        }
        if let Some(old) = old {
            self.posts.remove(&old);
        }
        self.submit(intent, draft)
    }

    /// Applies the answer of the worker request `id`, from the services
    /// drain, with the record state the worker kept for it. True when a
    /// post of this session was its request.
    pub(crate) fn finished(&mut self, id: DesktopRequestId, envelope: FinishedEnvelope) -> bool {
        let record = self.worker.record_state(id).unwrap_or(RecordState::Unknown);
        let Some(submission) = self.by_request(id) else {
            return false;
        };
        submission.state = SubmissionState::Finished { envelope, record };
        submission.previous = None;
        true
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
