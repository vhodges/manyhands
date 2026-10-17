//! Signal routing belongs only to the in-process SSH fixture test host.
use crate::ssh_remote::FixtureError;

#[cfg(unix)]
pub(crate) fn child_signal_blocked() -> Result<bool, FixtureError> {
    // SAFETY: valid output storage; a null input mask only queries this thread.
    unsafe {
        let mut mask = std::mem::zeroed();
        if libc::pthread_sigmask(libc::SIG_SETMASK, std::ptr::null(), &mut mask) != 0 {
            return Err(FixtureError);
        }
        Ok(libc::sigismember(&mask, libc::SIGCHLD) == 1)
    }
}

#[cfg(unix)]
pub(crate) fn server_thread_start() -> Result<(), FixtureError> {
    // Workers inherit the client's blocked mask. Keep SIGCHLD eligible here so
    // Tokio receives child exits and helpers inherit an unblocked SIGCHLD mask.
    // This hook also runs for the runtime's blocking-pool threads.
    // SAFETY: initialized set and valid signal; modifies only the current worker.
    unsafe {
        let mut child = std::mem::zeroed();
        if libc::sigemptyset(&mut child) != 0
            || libc::sigaddset(&mut child, libc::SIGCHLD) != 0
            || libc::pthread_sigmask(libc::SIG_UNBLOCK, &child, std::ptr::null_mut()) != 0
        {
            return Err(FixtureError);
        }
    }
    Ok(())
}

pub(crate) fn run_case(
    case: impl FnOnce() -> Result<(), FixtureError>,
) -> Result<(), FixtureError> {
    // One guard covers the entire case, rather than individual fixtures. All
    // case-local fixtures are dropped before restoration, including on unwind.
    #[cfg(unix)]
    let mut mask = ClientMask::block()?;
    let outcome = case();
    #[cfg(unix)]
    mask.restore()?;
    outcome
}

#[cfg(unix)]
struct ClientMask {
    previous: libc::sigset_t,
    restored: bool,
    // A saved pthread mask must only be restored on the thread which saved it.
    _thread: std::marker::PhantomData<std::rc::Rc<()>>,
}

#[cfg(unix)]
impl ClientMask {
    fn block() -> Result<Self, FixtureError> {
        // SAFETY: initialized sets; pthread_sigmask modifies only this thread
        // and writes its complete previous mask into valid storage.
        unsafe {
            let mut child = std::mem::zeroed();
            let mut previous = std::mem::zeroed();
            if libc::sigemptyset(&mut child) != 0
                || libc::sigaddset(&mut child, libc::SIGCHLD) != 0
                || libc::pthread_sigmask(libc::SIG_BLOCK, &child, &mut previous) != 0
            {
                return Err(FixtureError);
            }
            Ok(Self {
                previous,
                restored: false,
                _thread: std::marker::PhantomData,
            })
        }
    }

    fn restore(&mut self) -> Result<(), FixtureError> {
        if !self.restored {
            // SAFETY: restore the initialized mask saved on this same thread.
            if unsafe {
                libc::pthread_sigmask(libc::SIG_SETMASK, &self.previous, std::ptr::null_mut())
            } != 0
            {
                return Err(FixtureError);
            }
            self.restored = true;
        }
        Ok(())
    }
}

#[cfg(unix)]
impl Drop for ClientMask {
    fn drop(&mut self) {
        // Unwinding must restore the mask before the runner exits the child.
        // The ordinary path checks restoration errors explicitly in run_case.
        let _ = self.restore();
    }
}
