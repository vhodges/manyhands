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
    compare, restore_policy,
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

/// The protected drafts folder, for the crate's integration tests and
/// tooling only. The crate uses `protect::Folder` directly; this wrapper
/// exposes just enough of it to test the folder's protection.
#[doc(hidden)]
pub mod folder {
    use super::{DraftError, protect};
    use std::{fs::File, path::Path};

    /// `<data>/drafts`, created or checked and held open.
    pub struct Folder(protect::Folder);

    impl Folder {
        pub fn open(data: &Path) -> Result<Self, DraftError> {
            protect::Folder::open(data).map(Self)
        }

        /// A new owner-only file in the held folder.
        pub fn create_new(&mut self, name: &str) -> Result<File, DraftError> {
            self.0.create_new(name)
        }
    }
}
