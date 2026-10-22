//! Recoverable drafts: the file format, the store and its protections.
#[cfg(unix)]
#[path = "drafts/access.rs"]
mod access;
#[path = "drafts/format.rs"]
mod format;
