//! Process startup contract for the built-in Git SSH backend.

use std::{fmt, sync::OnceLock};

static INITIALIZATION: OnceLock<Result<(), TransportInitializationError>> = OnceLock::new();

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransportInitializationError;

impl fmt::Display for TransportInitializationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Git transport initialization failed")
    }
}

impl std::error::Error for TransportInitializationError {}

/// Set a 10-second TCP connect budget per address and a 30-second budget per
/// blocking SSH call. DNS and the total transfer duration are not bounded.
///
/// # Safety
/// Call before spawning any thread or starting concurrent/native Git activity.
/// The host must ensure no later code mutates these libgit2 global settings.
/// Idempotence does not make a late first invocation safe.
pub unsafe fn initialize_git_transport_before_threads() -> Result<(), TransportInitializationError>
{
    *INITIALIZATION.get_or_init(|| {
        // SAFETY: the caller owns the process-wide pre-thread startup contract.
        unsafe {
            git2::opts::set_server_connect_timeout_in_milliseconds(10_000)
                .and_then(|()| git2::opts::set_server_timeout_in_milliseconds(30_000))
                .map_err(|_| TransportInitializationError)
        }
    })
}

/// Operations must fail closed when the host has not completed startup.
pub fn git_transport_initialized() -> bool {
    matches!(INITIALIZATION.get(), Some(Ok(())))
}

#[cfg(test)]
mod tests {
    #[test]
    fn local_git_does_not_silently_initialize_network_settings() {
        assert!(!super::git_transport_initialized());
        let directory = tempfile::tempdir().unwrap();
        let _repository = git2::Repository::init(directory.path()).unwrap();
        assert!(!super::git_transport_initialized());
    }
}
