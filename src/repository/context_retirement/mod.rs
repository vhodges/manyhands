//! Tree-only completion and preservation evidence, shared by poll and retirement.
mod execution;
pub(super) mod handoff;
mod historical;
mod instance;
pub(super) mod pass;
pub(super) mod preserve;
pub(super) mod proof;
pub(super) mod record;
mod reentry;
pub(super) mod request_scope;
mod result;
mod run;
pub(super) mod source;
pub use execution::{ContextRetirementOutcome, RetireContextRequest};
pub use preserve::RetirementAbandonMetadata;
pub use reentry::ReenterContextRetirementRequest;
pub use result::{ContextRetirementReport, RetirementDisposition, RetirementRemoval};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod store_tests;

#[cfg(test)]
mod execution_tests;
