//! Recoverable drafts: a headless store of unsaved edits, kept outside every
//! repository so a crash or a failed save never loses typed text.
//!
//! This module holds the version 1 file format, the drafts folder's owner
//! protection, the store of claims and atomic revisions, the pure
//! comparison of a draft with its item and its restore policy, and the
//! Markdown export. Nothing here calls a repository mutation. Types that hold
//! draft text derive no `Debug`, so text cannot reach a log or an error by
//! accident.

mod compare;
#[cfg(test)]
mod compare_tests;
mod export;
#[cfg(test)]
mod export_tests;
#[cfg(test)]
mod folder_tests;
mod protect;
#[cfg(test)]
mod protect_tests;
mod schema;
#[cfg(test)]
mod schema_tests;
mod store;
#[cfg(test)]
mod store_tests;

pub use compare::{
    Change, Comparison, Field, RefusalReason, Restore, TargetState, classify, classify_comment,
    compare, is_new_item_base, restore_policy,
};
pub use export::markdown;
pub use protect::{DraftError, DraftErrorKind};
pub use schema::{
    BaseContext, CommentTarget, ContextKind, Draft, DraftBase, DraftFields, DraftId, DraftTarget,
    ItemKind, UnreadableReason,
};
#[doc(hidden)]
pub use store::{CRASH_EXIT_CODE, FailureAction, FailurePoint};
pub use store::{DraftClaim, DraftStore, Entry, Held, Listing, Retired, Written};

/// The file codec, for the crate's integration tests and tooling only. The
/// crate uses `schema::encode` and `schema::decode` directly.
#[doc(hidden)]
pub mod codec {
    pub use super::schema::{decode, encode};
}

/// A protected folder in the application data directory, by name.
///
/// [`folder::Folder::open_named`] holds `<data>/<name>` under the same
/// protection as the drafts folder: the data directory must be ours and not
/// writable by others; the folder is made owner-only (`0700` on Unix), never
/// followed through a link, and held open so that every file operation is
/// relative to it. Files are made owner-only (`0600`). On Windows the folder
/// must be no reparse point and be owned by this user, operations are path
/// based, and files inherit the folder's ACL (decision 3). Errors are
/// [`DraftError`]s, which carry no path and no file contents, and no type
/// here derives `Debug`.
pub mod folder {
    use super::{DraftError, DraftErrorKind, UnreadableReason, protect};
    use std::{
        fs::File,
        io::{Read, Write},
        path::Path,
    };

    /// A protected folder in the data directory, held open.
    pub struct Folder(protect::Folder);

    impl Folder {
        /// `<data>/drafts`, the drafts store's folder, made or checked and
        /// held open.
        pub fn open(data: &Path) -> Result<Self, DraftError> {
            protect::Folder::open(data).map(Self)
        }

        /// `<data>/<name>`, made or checked and held open. `name` must be a
        /// single normal path component: an empty name, `.`, `..`, a
        /// separator or a NUL (and on Windows a `:`) is refused with
        /// [`DraftErrorKind::Io`].
        pub fn open_named(data: &Path, name: &str) -> Result<Self, DraftError> {
            protect::Folder::open_named(data, name).map(Self)
        }

        /// A new owner-only file in the held folder, made exclusively and
        /// never through a link. `name` is checked as [`Folder::open_named`]
        /// checks a folder name, here and in every method below.
        pub fn create_new(&mut self, name: &str) -> Result<File, DraftError> {
            self.0.create_new(name)
        }

        /// Replaces the file `name` with `bytes` atomically: a new
        /// owner-only temporary file is written in full and synced, then
        /// renamed over `name`, and the folder synced. On a failure before
        /// the rename the temporary file is removed and any previous file
        /// is left as it was.
        pub fn replace(&mut self, name: &str, bytes: &[u8]) -> Result<(), DraftError> {
            let temporary = temporary_name(name)?;
            let mut file = self.0.create_new(&temporary)?;
            let written = file
                .write_all(bytes)
                .and_then(|()| file.sync_all())
                .map_err(DraftError::from);
            drop(file);
            if let Err(error) = written.and_then(|()| self.0.rename(&temporary, name)) {
                let _ = self.0.remove(&temporary);
                return Err(error);
            }
            self.0.sync()
        }

        /// The whole contents of the file `name`: a regular file, never a
        /// link, and on Unix ours ([`DraftErrorKind::ProtectionUnavailable`]
        /// otherwise). [`DraftErrorKind::NotFound`] when there is none.
        pub fn read(&self, name: &str) -> Result<Vec<u8>, DraftError> {
            let name = protect::entry_name(name)?;
            let mut file = self
                .0
                .open_file(name)
                .map_err(|reason| {
                    DraftError::new(match reason {
                        UnreadableReason::NotAFile | UnreadableReason::NotOwned => {
                            DraftErrorKind::ProtectionUnavailable
                        }
                        UnreadableReason::Access => DraftErrorKind::Access,
                        UnreadableReason::Damaged | UnreadableReason::NewerFormat { .. } => {
                            DraftErrorKind::Io
                        }
                    })
                })?
                .ok_or_else(|| DraftError::new(DraftErrorKind::NotFound))?;
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes)?;
            Ok(bytes)
        }

        /// The names of the files in the folder, sorted, leaving out
        /// temporary files (`.<name>.<16 hex>.tmp`), lock files (`*.lock`)
        /// and names that are not UTF-8.
        pub fn list(&self) -> Result<Vec<String>, DraftError> {
            let mut names: Vec<String> = self
                .0
                .names()?
                .into_iter()
                .filter(|name| !is_temporary(name) && !name.ends_with(".lock"))
                .collect();
            names.sort();
            Ok(names)
        }

        /// Removes the file `name`. [`DraftErrorKind::NotFound`] when there
        /// is none.
        pub fn remove(&mut self, name: &str) -> Result<(), DraftError> {
            self.0.remove(name)?;
            self.0.sync()
        }
    }

    /// `.<name>.<16 random hex>.tmp`, for a checked `name`.
    fn temporary_name(name: &str) -> Result<String, DraftError> {
        let name = protect::entry_name(name)?;
        let random = ulid::Ulid::new().random() as u64;
        Ok(format!(".{name}.{random:016x}.tmp"))
    }

    /// Whether `name` is a temporary file of [`Folder::replace`].
    fn is_temporary(name: &str) -> bool {
        name.strip_prefix('.')
            .and_then(|rest| rest.strip_suffix(".tmp"))
            .and_then(|rest| rest.rsplit_once('.'))
            .is_some_and(|(_, random)| {
                random.len() == 16 && random.bytes().all(|b| b.is_ascii_hexdigit())
            })
    }
}
