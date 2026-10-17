use crate::{
    ssh_harness::signals,
    ssh_remote::{FixtureError, SshRemoteFixture, fixed},
};

fn current_mask() -> Result<libc::sigset_t, FixtureError> {
    // SAFETY: valid output storage; a null input mask only queries this thread.
    unsafe {
        let mut mask = std::mem::zeroed();
        if libc::pthread_sigmask(libc::SIG_SETMASK, std::ptr::null(), &mut mask) != 0 {
            return Err(FixtureError);
        }
        Ok(mask)
    }
}

pub(crate) fn child_signal_blocked() -> Result<bool, FixtureError> {
    let mask = current_mask()?;
    // SAFETY: initialized signal set and valid signal number.
    Ok(unsafe { libc::sigismember(&mask, libc::SIGCHLD) } == 1)
}

fn set_mask(mask: &libc::sigset_t) -> Result<(), FixtureError> {
    // SAFETY: initialized signal set, changing only the current thread.
    if unsafe { libc::pthread_sigmask(libc::SIG_SETMASK, mask, std::ptr::null_mut()) } != 0 {
        return Err(FixtureError);
    }
    Ok(())
}

fn same_mask(expected: &libc::sigset_t) -> Result<(), FixtureError> {
    let actual = current_mask()?;
    // Linux/macOS signal numbers fit this range; compare membership, not padding.
    for signal in 1..=128 {
        // SAFETY: initialized sets; sigismember reports invalid numbers as -1.
        assert_eq!(unsafe { libc::sigismember(expected, signal) }, unsafe {
            libc::sigismember(&actual, signal)
        });
    }
    Ok(())
}

struct RestoreMask(libc::sigset_t);
impl Drop for RestoreMask {
    fn drop(&mut self) {
        let _ = set_mask(&self.0);
    }
}

pub(crate) fn mask_restored() -> Result<(), FixtureError> {
    assert!(
        child_signal_blocked()?,
        "test client must isolate fixture child signals"
    );
    let original = current_mask()?;
    let restore = RestoreMask(original);
    let mut prior = current_mask()?;
    // SAFETY: initialized set and valid signal numbers.
    unsafe {
        assert_eq!(libc::sigdelset(&mut prior, libc::SIGCHLD), 0);
        assert_eq!(libc::sigaddset(&mut prior, libc::SIGUSR1), 0);
    }
    set_mask(&prior)?;
    signals::run_case(|| {
        assert!(child_signal_blocked()?);
        Ok(())
    })?;
    same_mask(&prior)?;
    assert!(signals::run_case(|| Err(FixtureError)).is_err());
    same_mask(&prior)?;
    assert!(
        std::panic::catch_unwind(|| {
            signals::run_case(|| std::panic::resume_unwind(Box::new(())))
        })
        .is_err()
    );
    same_mask(&prior)?;
    // Also preserve a caller which already blocked SIGCHLD.
    set_mask(&restore.0)?;
    signals::run_case(|| Ok(()))?;
    same_mask(&restore.0)
}

pub(crate) fn fixture_routing() -> Result<(), FixtureError> {
    assert!(
        child_signal_blocked()?,
        "test client must isolate fixture child signals"
    );
    let mut first = SshRemoteFixture::start()?;
    let mut second = SshRemoteFixture::start()?;
    assert_eq!(first.worker_child_signal_masks()?, [false, false]);
    assert_eq!(second.worker_child_signal_masks()?, [false, false]);
    fixed(crate::advertise(&first, first.client_key_path(), None))?;
    first.shutdown()?;
    assert_eq!(first.active_helpers(), 0);
    assert_eq!(first.completed_helpers(), 1);
    assert!(
        child_signal_blocked()?,
        "first fixture shutdown must not unmask the client"
    );
    fixed(crate::advertise(&second, second.client_key_path(), None))?;
    second.shutdown()?;
    assert_eq!(second.active_helpers(), 0);
    assert_eq!(second.completed_helpers(), 1);
    assert!(
        child_signal_blocked()?,
        "all fixture cleanup precedes mask restoration"
    );
    Ok(())
}

pub(crate) fn poll_protected() -> Result<(), FixtureError> {
    use std::{
        io::Write,
        os::{fd::AsRawFd, unix::net::UnixStream},
        time::Duration,
    };
    extern "C" fn child_notice(_: libc::c_int) {}
    struct RestoreAction(libc::sigaction);
    impl Drop for RestoreAction {
        fn drop(&mut self) {
            // SAFETY: restore the initialized action saved in this isolated case.
            unsafe {
                libc::sigaction(libc::SIGCHLD, &self.0, std::ptr::null_mut());
            }
        }
    }
    // This isolated case creates no Tokio runtime or process waiter, so it never
    // replaces Tokio's handler. Match SA_RESTART, which does not restart poll.
    // SAFETY: valid signal/action storage and a handler with the C signal ABI.
    let _action = unsafe {
        let mut action: libc::sigaction = std::mem::zeroed();
        let mut previous = std::mem::zeroed();
        action.sa_sigaction = child_notice as *const () as usize;
        action.sa_flags = libc::SA_RESTART;
        assert_eq!(libc::sigemptyset(&mut action.sa_mask), 0);
        assert_eq!(libc::sigaction(libc::SIGCHLD, &action, &mut previous), 0);
        RestoreAction(previous)
    };
    let (read, mut write) = fixed(UnixStream::pair())?;
    // pthread_t is a pointer on macOS; carry only its value to the sender thread.
    // SAFETY: pthread_self has no preconditions.
    let client = unsafe { libc::pthread_self() } as usize;
    let sender = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        // SAFETY: the client thread remains alive until this sender is joined.
        assert_eq!(
            unsafe { libc::pthread_kill(client as libc::pthread_t, libc::SIGCHLD) },
            0
        );
        // Controlled injection timing, not a fixture settling delay: give an
        // unprotected poll a chance to report EINTR before making data ready.
        std::thread::sleep(Duration::from_millis(50));
        fixed(write.write_all(&[1]))
    });
    let mut descriptor = libc::pollfd {
        fd: read.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    // SAFETY: a live descriptor and one initialized pollfd; the wait is bounded.
    let ready = unsafe { libc::poll(&mut descriptor, 1, 2_000) };
    fixed(sender.join())??;
    // Consume our targeted pending signal before restoring the prior handler.
    // Process-directed fixture signals instead go to eligible runtime workers.
    // SAFETY: valid output storage and initialized sets; only consume SIGCHLD
    // after confirming it is pending on this thread/process.
    unsafe {
        let mut pending = std::mem::zeroed();
        assert_eq!(libc::sigpending(&mut pending), 0);
        if libc::sigismember(&pending, libc::SIGCHLD) == 1 {
            let mut child = std::mem::zeroed();
            assert_eq!(libc::sigemptyset(&mut child), 0);
            assert_eq!(libc::sigaddset(&mut child, libc::SIGCHLD), 0);
            let mut received = 0;
            assert_eq!(libc::sigwait(&child, &mut received), 0);
            assert_eq!(received, libc::SIGCHLD);
        }
    }
    assert_eq!(
        ready, 1,
        "fixture child signal must not interrupt client poll"
    );
    assert_ne!(descriptor.revents & libc::POLLIN, 0);
    Ok(())
}
