//! Durable local-primary synchronization authority, separate from transport.
// ReadError intentionally carries the shared result scope by value.
#![allow(clippy::result_large_err)]

mod observe;
pub(super) mod record;
pub use observe::LocalContextSyncObservationDto;

#[cfg(test)]
mod admission_tests;
#[cfg(test)]
mod observe_tests;
#[cfg(test)]
mod record_tests;
#[cfg(test)]
mod review_tests;
