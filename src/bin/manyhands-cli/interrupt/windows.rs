//! Ctrl-C through `SetConsoleCtrlHandler`. The system runs the routine on
//! a thread it creates; the CLI spawns none.

use super::Press;
use windows_sys::{
    Win32::System::Console::{CTRL_C_EVENT, SetConsoleCtrlHandler},
    core::BOOL,
};

/// The routine was added to the process's list of handlers.
pub(super) struct Saved;

/// Adds the window's routine. `None` when it could not be added.
pub(super) fn install() -> Option<Saved> {
    // SAFETY: the routine is a valid `extern "system"` function for the
    // whole process lifetime.
    (unsafe { SetConsoleCtrlHandler(Some(routine), 1) } != 0).then_some(Saved)
}

/// Removes the routine `install` added. A routine the system had already
/// dispatched can still run after this: it may latch a press and return
/// `TRUE`, with no token left to cancel. That press is harmless: the retry
/// loop stops on it, or the run's next window starts cancelled.
pub(super) fn restore(_: Saved) {
    // SAFETY: removes exactly the routine `install` added.
    unsafe { SetConsoleCtrlHandler(Some(routine), 0) };
}

/// The first `CTRL_C_EVENT` cancels and is handled; a later one is not, so
/// the next handler, the default, ends the process. Other events are left
/// to the default handler.
unsafe extern "system" fn routine(event: u32) -> BOOL {
    if event != CTRL_C_EVENT {
        return 0;
    }
    match super::press() {
        Press::First => 1,
        Press::Again => 0,
    }
}
