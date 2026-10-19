//! The session's desktop requests, newest first, as the worker's events
//! left them.
//!
//! Task 7 adds the pending operations each repository reports.

use manyhands::repository::ProgressStage;

use crate::desktop::services::{
    requests::{DesktopRequest, DesktopRequestId, RequestState},
    worker::WorkerEvent,
};

#[derive(Default)]
pub(crate) struct Operations {
    requests: Vec<DesktopRequest>,
}

impl Operations {
    /// This session's requests, newest first.
    pub(crate) fn requests(&self) -> &[DesktopRequest] {
        &self.requests
    }

    pub(crate) fn request(&self, id: DesktopRequestId) -> Option<&DesktopRequest> {
        self.requests.iter().find(|request| request.id == id)
    }

    fn request_mut(&mut self, id: DesktopRequestId) -> Option<&mut DesktopRequest> {
        self.requests.iter_mut().find(|request| request.id == id)
    }

    /// Applies what the worker said. An event for a request this model
    /// never saw queued (a host observation) changes nothing here.
    pub(crate) fn apply(&mut self, event: WorkerEvent) {
        match event {
            WorkerEvent::Queued(request) => match self.request_mut(request.id) {
                // A retry: the same request, in its place.
                Some(existing) => *existing = request,
                None => self.requests.insert(0, request),
            },
            WorkerEvent::Started(id) => self.set(
                id,
                RequestState::Running {
                    stage: None,
                    accepted: false,
                },
            ),
            WorkerEvent::Progress(id, progress) => {
                if let Some(request) = self.request_mut(id)
                    && let RequestState::Running { stage, accepted } = &mut request.state
                {
                    *stage = Some(progress.stage);
                    *accepted |= progress.stage == ProgressStage::Accepted;
                }
            }
            WorkerEvent::Finished(id, envelope) => {
                let envelope = *envelope;
                self.set(
                    id,
                    RequestState::Finished(
                        envelope.outcome,
                        envelope.code,
                        envelope.effects,
                        envelope.recovery,
                        envelope.message,
                    ),
                );
            }
            WorkerEvent::Removed(id) => self.set(id, RequestState::Dismissed),
            WorkerEvent::NotSent(id, why) => self.set(id, RequestState::NotSent(why)),
            // The Hosts page reads observations (Task 16).
            WorkerEvent::HostObserved(..) => {}
            // No prompt is shown until Task 11: the dropped reply answers
            // `Unavailable`.
            WorkerEvent::Prompt(_) => {}
        }
    }

    /// Hides a request that is no longer queued or running.
    pub(crate) fn dismiss(&mut self, id: DesktopRequestId) {
        if let Some(request) = self.request_mut(id)
            && !matches!(
                request.state,
                RequestState::Queued | RequestState::Running { .. }
            )
        {
            request.state = RequestState::Dismissed;
        }
    }

    fn set(&mut self, id: DesktopRequestId, state: RequestState) {
        if let Some(request) = self.request_mut(id) {
            request.state = state;
        }
    }
}

#[cfg(test)]
#[path = "operations_tests.rs"]
mod tests;
