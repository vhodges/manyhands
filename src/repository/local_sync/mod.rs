//! Durable local-primary synchronization authority, separate from transport.
// ReadError intentionally carries the shared result scope by value.
#![allow(clippy::result_large_err)]

#[cfg(test)]
mod conflict;
mod conflict_record;
#[cfg(test)]
mod control;
#[cfg(test)]
mod file_image;
mod image_proof;
mod observe;
pub(super) mod record;
#[cfg(test)]
mod resolve;
pub use observe::LocalContextSyncObservationDto;

// Controller staging decision: Task 7 MUST remove these gates when it adds
// production mutation/resume callers and their request-attempt authority.
#[cfg(test)]
mod recover;
#[cfg(test)]
mod run;
#[cfg(test)]
mod run_tests;

#[cfg(test)]
mod admission_tests;
#[cfg(test)]
mod conflict_tests;
#[cfg(test)]
mod control_tests;
#[cfg(test)]
mod observe_tests;
#[cfg(test)]
mod quality_tests;
#[cfg(test)]
mod record_tests;
#[cfg(test)]
mod review_tests;
#[cfg(test)]
mod spec_tests;
