//! `SIGINT` through `sigaction`, with `SA_RESTART` so that an interrupted
//! system call is restarted rather than failed.

use super::Press;
use std::ptr::null_mut;

/// The disposition `SIGINT` had before the window installed its own.
pub(super) struct Saved(libc::sigaction);

/// Installs the window's handler. `None` when it could not be installed,
/// which leaves the disposition as it was.
pub(super) fn install() -> Option<Saved> {
    // SAFETY: zeroed sigaction storage is valid; the mask is initialized
    // by sigemptyset, and both pointers are to live, initialized storage.
    unsafe {
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = handler as extern "C" fn(libc::c_int) as libc::sighandler_t;
        action.sa_flags = libc::SA_RESTART;
        libc::sigemptyset(&mut action.sa_mask);
        let mut saved: libc::sigaction = std::mem::zeroed();
        (libc::sigaction(libc::SIGINT, &action, &mut saved) == 0).then_some(Saved(saved))
    }
}

/// Puts back the disposition `install` found.
pub(super) fn restore(saved: Saved) {
    // SAFETY: `saved` is the complete action sigaction reported.
    unsafe { libc::sigaction(libc::SIGINT, &saved.0, null_mut()) };
}

/// The first press cancels; a later one restores the default disposition
/// and raises `SIGINT` again. `SIGINT` is blocked while this runs, so the
/// raised signal is delivered when the handler returns, and the process
/// ends by it.
extern "C" fn handler(_: libc::c_int) {
    if super::press() == Press::Again {
        // SAFETY: sigaction and raise are async-signal-safe, and the action
        // is zeroed storage with the default disposition and an empty mask.
        unsafe {
            let mut default: libc::sigaction = std::mem::zeroed();
            default.sa_sigaction = libc::SIG_DFL;
            libc::sigemptyset(&mut default.sa_mask);
            libc::sigaction(libc::SIGINT, &default, null_mut());
            libc::raise(libc::SIGINT);
        }
    }
}
