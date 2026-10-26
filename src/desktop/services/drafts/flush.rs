//! Writing a session's newest revision: on its schedule, at once (Retry,
//! a close, a quit, a save) and after a write in flight.

use futures::future::{FutureExt, Shared, join_all};
use gpui_kit::{Context, Task};
use manyhands::drafts::{DraftError, DraftErrorKind, DraftId, Written};

use super::{DraftRegistry, off_thread};
use crate::desktop::{model::drafts::RecoveryState, services::ReadKind};

/// What a flush does next.
enum Flush {
    Done(Result<(), DraftErrorKind>),
    /// Wait for this write; `started` when this flush started it.
    Wait {
        write: Shared<Task<()>>,
        started: bool,
    },
}

impl DraftRegistry {
    /// Writes the newest revision of `id` now, after any write in flight:
    /// Retry, a close and a quit. Answers once it is written, or why the
    /// write this flush made failed. A session whose draft is gone with
    /// its text unstored is stored again as a new draft first.
    ///
    /// `NotFound` with no live session (`current_id` is `None`) means only
    /// that the session has ended, retired or discarded, and holds no
    /// unsaved text: no text is left in memory with no way to store it.
    /// A live session's write can fail `NotFound` too (its drafts folder
    /// gone): that is a real failure, and the session stays failed with
    /// its text. An ID whose session goes on under a new draft reaches
    /// that session.
    pub(crate) fn flush_now(
        &mut self,
        id: DraftId,
        cx: &mut Context<Self>,
    ) -> Task<Result<(), DraftErrorKind>> {
        cx.spawn(async move |registry, cx| {
            let mut attempted = false;
            loop {
                let step = registry
                    .update(cx, |registry, cx| registry.flush_step(id, attempted, cx))
                    .map_err(|_| DraftErrorKind::NotFound)?;
                match step {
                    Flush::Done(result) => return result,
                    Flush::Wait { write, started } => {
                        attempted |= started;
                        write.await;
                    }
                }
            }
        })
    }

    /// Flushes every session now; answers the ones that failed, by the ID
    /// each goes on under. Only a session that ended meanwhile, `NotFound`
    /// with no live session, left nothing unsaved and is not a failure; a
    /// live session's `NotFound` is reported. If the registry is gone,
    /// every failure is reported.
    pub(crate) fn flush_all(
        &mut self,
        cx: &mut Context<Self>,
    ) -> Task<Vec<(DraftId, DraftErrorKind)>> {
        let ids: Vec<DraftId> = self.sessions.keys().copied().collect();
        let flushes: Vec<_> = ids
            .into_iter()
            .map(|id| {
                let flush = self.flush_now(id, cx);
                async move { (id, flush.await) }
            })
            .collect();
        cx.spawn(async move |registry, cx| {
            let failed: Vec<(DraftId, DraftErrorKind)> = join_all(flushes)
                .await
                .into_iter()
                .filter_map(|(id, result)| result.err().map(|kind| (id, kind)))
                .collect();
            registry
                .read_with(cx, |registry, _| {
                    failed
                        .iter()
                        .filter_map(|(id, kind)| match (registry.current_id(*id), kind) {
                            // The session ended: nothing unsaved is left.
                            (None, DraftErrorKind::NotFound) => None,
                            (current, kind) => Some((current.unwrap_or(*id), *kind)),
                        })
                        .collect()
                })
                .unwrap_or(failed)
        })
    }

    /// Whether a session's newest text is not yet stored, or a store call
    /// holds its claim: quitting flushes them all first.
    pub(crate) fn unsaved(&self) -> bool {
        self.sessions.values().any(|live| {
            live.session.busy() || !matches!(live.session.recovery_state(), RecoveryState::Saved(_))
        })
    }

    fn flush_step(&mut self, id: DraftId, attempted: bool, cx: &mut Context<Self>) -> Flush {
        let Some(id) = self.current_id(id) else {
            return Flush::Done(Err(DraftErrorKind::NotFound));
        };
        let Some(live) = self.sessions.get_mut(&id) else {
            return Flush::Done(Err(DraftErrorKind::NotFound));
        };
        if live.session.busy()
            && let Some(write) = live.write.clone()
        {
            return Flush::Wait {
                write,
                started: false,
            };
        }
        match live.session.recovery_state() {
            RecoveryState::Saved(_) => Flush::Done(Ok(())),
            RecoveryState::Failed(kind) if attempted => Flush::Done(Err(kind)),
            _ => {
                live.timer = None;
                match self.start_write(id, cx) {
                    Some(write) => Flush::Wait {
                        write,
                        started: true,
                    },
                    None => Flush::Done(Ok(())),
                }
            }
        }
    }

    /// Waits on a timer for the session's due time, then writes it.
    pub(super) fn schedule(&mut self, id: DraftId, cx: &mut Context<Self>) {
        let now = cx.background_executor().now();
        let Some(live) = self.sessions.get_mut(&id) else {
            return;
        };
        let Some(due) = live.session.flush_due() else {
            live.timer = None;
            return;
        };
        let wait = due.saturating_duration_since(now);
        live.timer = Some(cx.spawn(async move |registry, cx| {
            cx.background_executor().timer(wait).await;
            let _ = registry.update(cx, |registry, cx| registry.start_write(id, cx));
        }));
    }

    /// Takes the session's newest revision and writes it off the
    /// foreground. `None` when nothing is new or a write is in flight. A
    /// session whose draft is gone with its text unstored (a store again
    /// that failed) is stored again instead, as a new draft over its base.
    pub(super) fn start_write(
        &mut self,
        id: DraftId,
        cx: &mut Context<Self>,
    ) -> Option<Shared<Task<()>>> {
        let live = self.sessions.get_mut(&id)?;
        if live.session.unclaimed() && !live.session.busy() {
            let base = live.session.base().cloned();
            return self.store_again(id, base, cx);
        }
        let write = live.session.next_write()?;
        let work = off_thread(
            &self.backend,
            ReadKind::DraftWrite,
            move |_| write.write(),
            cx,
        );
        let task = cx
            .spawn(async move |registry, cx| {
                let result = work.await;
                let _ = registry.update(cx, |registry, cx| registry.write_landed(id, result, cx));
            })
            .shared();
        live.write = Some(task.clone());
        Some(task)
    }

    fn write_landed(
        &mut self,
        id: DraftId,
        result: Result<Written, DraftError>,
        cx: &mut Context<Self>,
    ) {
        let Some(live) = self.sessions.get_mut(&id) else {
            return;
        };
        live.session.write_landed(result);
        self.schedule(id, cx);
        cx.notify();
        self.release_if_pending(id, cx);
    }
}
