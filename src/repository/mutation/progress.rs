//! Progress and cancellation: what a call of `execute` is given besides
//! its request, and how that reaches a binding.
//!
//! A caller hands `execute` a sink for the call's progress and a token
//! that stops it. Both are the call's and no part of the request: the
//! same request sent again brings its own.

use std::{
    cell::RefCell,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

/// How a caller asks a running request to stop. A request that is
/// cancelled before it is accepted does nothing.
#[derive(Clone, Debug, Default)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// Where a request reports its progress. No command reports any yet; the
/// events are added with the commands that have stages to report.
pub trait ProgressSink {}

/// A sink for a caller that wants no progress.
pub struct NoProgress;

impl ProgressSink for NoProgress {}

/// What one call was given besides its request, as a binding reaches
/// it: where its progress goes and what stops it.
///
/// This is a trait and its one use is as a trait object. `execute` is
/// generic over what supplies a passphrase, and a binding is not: what
/// a call was given is behind this, so that no binding, and nothing
/// between `execute` and a binding, is generic with it.
pub(crate) trait Session {
    /// Whether the call's token has been cancelled.
    fn cancelled(&self) -> bool;
}

/// What one call of `execute` was given.
///
/// A binding is handed its call's context by shared reference, and a
/// sink is written to: the sink is behind a `RefCell`, which the one
/// thread a call runs on borrows for the length of one report.
pub(crate) struct CallSession<'s> {
    // Read by the commands that report stages, which are not bound yet.
    #[expect(dead_code)]
    progress: RefCell<&'s mut dyn ProgressSink>,
    cancel: &'s CancellationToken,
}

impl<'s> CallSession<'s> {
    pub(crate) fn new(progress: &'s mut dyn ProgressSink, cancel: &'s CancellationToken) -> Self {
        Self {
            progress: RefCell::new(progress),
            cancel,
        }
    }
}

impl Session for CallSession<'_> {
    fn cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }
}

/// What `prepare` gives a binding in place of a call's session: a
/// preview reports no progress and is not stopped, since it changes
/// nothing and is over at once.
pub(crate) struct NoSession;

impl Session for NoSession {
    fn cancelled(&self) -> bool {
        false
    }
}
