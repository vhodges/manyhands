//! Tree-only completion and preservation evidence, shared by poll and retirement.
mod execution;
mod instance;
pub(super) mod proof;
pub(super) mod record;
mod reentry;
pub(super) mod request_scope;
mod result;
mod run;
pub(super) mod source;
pub use execution::{ContextRetirementOutcome, RetireContextRequest};
pub use reentry::ReenterContextRetirementRequest;
pub use result::{ContextRetirementReport, RetirementDisposition, RetirementRemoval};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod store_tests;

#[cfg(test)]
mod execution_tests;
