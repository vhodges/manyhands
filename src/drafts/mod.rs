//! Recoverable drafts: a headless store of unsaved edits, kept outside every
//! repository so a crash or a failed save never loses typed text.
//!
//! This module holds the version 1 file format. Types that hold draft text
//! derive no `Debug`, so text cannot reach a log or an error by accident.

mod schema;
#[cfg(test)]
mod schema_tests;

pub use schema::{
    BaseContext, CommentTarget, ContextKind, Draft, DraftBase, DraftFields, DraftId, DraftTarget,
    ItemKind, UnreadableReason,
};

/// The file codec, for the crate's integration tests and tooling only. The
/// crate uses `schema::encode` and `schema::decode` directly.
#[doc(hidden)]
pub mod codec {
    pub use super::schema::{decode, encode};
}
