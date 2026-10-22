//! Recoverable drafts: the file format, the store and its protections.
#[cfg(unix)]
#[path = "drafts/access.rs"]
mod access;
#[path = "drafts/claims.rs"]
mod claims;
#[path = "drafts/crash.rs"]
mod crash;
#[path = "drafts/format.rs"]
mod format;
#[path = "drafts/support.rs"]
mod support;
#[path = "drafts/write.rs"]
mod write;
