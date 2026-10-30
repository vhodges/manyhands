//! The Ctrl-C window (R-21, R-36). While a stoppable verb's `execute` runs,
//! a process-wide handler turns the first Ctrl-C into a cancellation of the
//! call's token, which the library honours at its next safe point, and lets
//! a second one end the process. Outside a window Ctrl-C keeps the
//! disposition the process had, which ends it at once.
//!
//! One latch per run: the press count is shared by every window of one CLI
//! run, so a press stays latched across C2's bounded retry, and the retry
//! loop stops once a press is latched. `mutate::run` resets it on entry.
//!
//! The handler only does atomic loads and stores, which are
//! async-signal-safe. The token it cancels lives in a slot that teardown
//! clears, after restoring the disposition and before the token is dropped;
//! teardown also waits for a handler already running on another thread.

use manyhands::repository::CancellationToken;
use std::{
    ptr::null_mut,
    sync::atomic::{AtomicPtr, AtomicUsize, Ordering::SeqCst},
};

#[cfg(unix)]
#[path = "interrupt/unix.rs"]
mod platform;
#[cfg(windows)]
#[path = "interrupt/windows.rs"]
mod platform;

/// The token of the open window, or null when no window is open.
static SLOT: AtomicPtr<CancellationToken> = AtomicPtr::new(null_mut());
/// The presses of this run, in every window (R-36).
static PRESSES: AtomicUsize = AtomicUsize::new(0);
/// Handlers between reading the slot and finishing with its token.
static ACTIVE: AtomicUsize = AtomicUsize::new(0);

/// What a press is to the handler that saw it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Press {
    /// The run's first: the window's token is cancelled.
    First,
    /// A later one: the process is to end as Ctrl-C ends it.
    Again,
}

/// Clears the run's latch. A run starts with no press.
pub(crate) fn reset() {
    PRESSES.store(0, SeqCst);
}

/// Whether a press has been latched in this run.
pub(crate) fn latched() -> bool {
    PRESSES.load(SeqCst) > 0
}

/// Counts a press and, for the run's first, cancels the open window's
/// token. Called by the platform's handler, so it only uses atomics.
fn press() -> Press {
    ACTIVE.fetch_add(1, SeqCst);
    let first = PRESSES.fetch_add(1, SeqCst) == 0;
    if first {
        let token = SLOT.load(SeqCst);
        if !token.is_null() {
            // SAFETY: the slot holds a live token until teardown clears it,
            // and teardown drops the token only once ACTIVE is back to zero,
            // which this handler holds above zero until it is done with it.
            unsafe { (*token).cancel() };
        }
    }
    ACTIVE.fetch_sub(1, SeqCst);
    if first { Press::First } else { Press::Again }
}

/// Runs `work` with Ctrl-C handled: the first press of the run cancels the
/// token `work` is given, and a second ends the process. A token begins
/// cancelled when the run has already latched a press. The disposition the
/// process had is restored when `work` returns or unwinds.
pub(crate) fn during<T>(work: impl FnOnce(&CancellationToken) -> T) -> T {
    let token = Box::new(CancellationToken::new());
    if latched() {
        token.cancel();
    }
    let window = Window::open(token);
    // SAFETY: the window owns the token until it is dropped below.
    work(unsafe { &*window.token })
}

/// An open window: its token is in the slot and the handler is installed.
/// Dropping it, on return or on unwind, tears the window down. Windows
/// never nest: one wraps one `execute`, and the CLI runs one at a time.
struct Window {
    token: *mut CancellationToken,
    saved: Option<platform::Saved>,
}

impl Window {
    fn open(token: Box<CancellationToken>) -> Self {
        let token = Box::into_raw(token);
        let opened = SLOT.compare_exchange(null_mut(), token, SeqCst, SeqCst);
        debug_assert!(opened.is_ok(), "a window is already open");
        Self {
            token,
            saved: platform::install(),
        }
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        if let Some(saved) = self.saved.take() {
            platform::restore(saved);
        }
        SLOT.store(null_mut(), SeqCst);
        // A handler that read the slot before it was cleared is done with
        // the token once ACTIVE is zero again.
        while ACTIVE.load(SeqCst) != 0 {
            std::hint::spin_loop();
        }
        // SAFETY: the pointer came from Box::into_raw in `open`, nothing
        // else frees it, and no handler can reach it any more.
        drop(unsafe { Box::from_raw(self.token) });
    }
}

#[cfg(test)]
#[path = "interrupt_tests.rs"]
mod tests;
