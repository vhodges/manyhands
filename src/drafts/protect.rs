//! The drafts folder's owner protection (design decisions 3 and 5), and the
//! store's error type.
//!
//! On Unix the folder is held open and every file operation is relative to
//! that handle, so renaming or swapping `drafts/` cannot redirect a write.
//! On Windows operations are path based; the folder must be no reparse point
//! and be owned by the process's user or its default owner.

use std::{fmt, path::PathBuf};

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

/// Why a draft operation failed. It carries no path, no text and no
/// underlying error message.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DraftError {
    pub kind: DraftErrorKind,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DraftErrorKind {
    /// The platform gave no application data directory.
    Unavailable,
    /// A link, a reparse point, another owner or an other-writable parent.
    ProtectionUnavailable,
    /// Permission was denied.
    Access,
    /// The disk or the quota is full.
    NoSpace,
    /// The draft's lock is held by another process.
    OpenElsewhere,
    /// This store already holds the draft's claim.
    HeldHere,
    /// The draft is gone.
    NotFound,
    /// Anything else.
    Io,
}

impl DraftError {
    pub(crate) fn new(kind: DraftErrorKind) -> Self {
        Self { kind }
    }
}

impl fmt::Display for DraftError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.kind {
            DraftErrorKind::Unavailable => "No application data directory is available for drafts.",
            DraftErrorKind::ProtectionUnavailable => "The drafts folder could not be protected.",
            DraftErrorKind::Access => "Access to the drafts folder was denied.",
            DraftErrorKind::NoSpace => "There is no space left to keep the draft.",
            DraftErrorKind::OpenElsewhere => "The draft is open in another Manyhands process.",
            DraftErrorKind::HeldHere => "The draft is already open here.",
            DraftErrorKind::NotFound => "The draft no longer exists.",
            DraftErrorKind::Io => "The draft could not be read or written.",
        })
    }
}

impl std::error::Error for DraftError {}

impl From<std::io::Error> for DraftError {
    fn from(error: std::io::Error) -> Self {
        use std::io::ErrorKind;
        Self::new(match error.kind() {
            ErrorKind::PermissionDenied => DraftErrorKind::Access,
            ErrorKind::StorageFull | ErrorKind::QuotaExceeded => DraftErrorKind::NoSpace,
            ErrorKind::NotFound => DraftErrorKind::NotFound,
            _ => DraftErrorKind::Io,
        })
    }
}

/// The drafts folder, held open. On Unix every file operation is relative
/// to `handle` (decision 5); on Windows to `path`.
pub(crate) struct Folder {
    /// `<data>/drafts`.
    path: PathBuf,
    #[cfg(unix)]
    handle: std::fs::File,
}

impl Folder {
    /// The data directory `path` is in.
    fn data(&self) -> Result<&std::path::Path, DraftError> {
        self.path
            .parent()
            .ok_or_else(|| DraftError::new(DraftErrorKind::Io))
    }
}

/// What a directory's mode and owner allow.
#[cfg(unix)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Verdict {
    Ok,
    /// Ours, but with group or other bits to clear.
    Tighten,
    Refuse,
}

/// The data directory may be group-writable (the library makes it with the
/// user's umask) but must be ours and not writable by others.
#[cfg(unix)]
pub(crate) fn data_verdict(mode: u32, uid: u32, euid: u32) -> Verdict {
    if uid != euid || mode & 0o002 != 0 {
        Verdict::Refuse
    } else {
        Verdict::Ok
    }
}

/// `drafts/` must be our directory; any bit beyond the owner's is cleared.
#[cfg(unix)]
pub(crate) fn folder_verdict(mode: u32, uid: u32, euid: u32, is_dir: bool) -> Verdict {
    if uid != euid || !is_dir {
        Verdict::Refuse
    } else if mode & 0o7777 & !0o700 != 0 {
        Verdict::Tighten
    } else {
        Verdict::Ok
    }
}

/// The application data directory, the registry's own: the twin of
/// `default_data_directory` in `src/repository.rs`, with the same
/// `ProjectDirs` arguments. If either changes, change both.
// Used by the draft store (Task 3).
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn default_data_directory() -> Result<PathBuf, DraftError> {
    directories::ProjectDirs::from("com", "manyhands", "Manyhands")
        .map(|directories| directories.data_local_dir().to_owned())
        .ok_or_else(|| DraftError::new(DraftErrorKind::Unavailable))
}
