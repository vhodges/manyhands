//! `answer_unanswered_polls`: an opt-in stand-in for every automatic poll
//! no `answer_poll` answer is queued for, so a journey whose scheduler polls
//! a varying number of times never reaches the library's transport. Without
//! it a poll with nothing queued makes the library's own call, as before.

use std::path::Path;

use super::PollingBackend;
use crate::desktop::services::poll_convert::Library;

/// The stand-in answer, and the root of each poll that got it.
pub(super) struct Unanswered {
    answer: Box<dyn Fn() -> Library + Send>,
    roots: Vec<String>,
}

impl PollingBackend {
    /// From now on, a poll with no answer queued gets `answer()` in place
    /// of the library's call, and its root is counted (`unanswered_roots`).
    pub(crate) fn answer_unanswered_polls(&self, answer: impl Fn() -> Library + Send + 'static) {
        *self.unanswered.lock().unwrap() = Some(Unanswered {
            answer: Box::new(answer),
            roots: Vec::new(),
        });
    }

    /// The root of every poll the stand-in answered, in order.
    pub(crate) fn unanswered_roots(&self) -> Vec<String> {
        self.unanswered
            .lock()
            .unwrap()
            .as_ref()
            .map(|unanswered| unanswered.roots.clone())
            .unwrap_or_default()
    }

    /// In `poll_automatic`, when nothing is queued: queues the stand-in for
    /// `after_poll` and counts `root`. False when the mode is off.
    pub(super) fn stand_in(&self, root: &Path) -> bool {
        let mut unanswered = self.unanswered.lock().unwrap();
        let Some(unanswered) = unanswered.as_mut() else {
            return false;
        };
        unanswered.roots.push(root.to_string_lossy().into_owned());
        self.canned.lock().unwrap().push_back((unanswered.answer)());
        true
    }
}
