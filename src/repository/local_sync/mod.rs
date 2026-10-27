//! Durable local-primary synchronization authority, separate from transport.
// ReadError intentionally carries the shared result scope by value.
#![allow(clippy::result_large_err)]

mod conflict;
mod conflict_record;
mod control;
mod file_image;
mod image_proof;
mod observe;
pub(super) mod record;
mod resolve;
pub use observe::LocalContextSyncObservationDto;

mod recover;
mod request;
mod run;
pub(super) use request::{Fence, content, data, execute_resolution, execute_start, inspection};
pub(super) fn valid_observation(value: &str) -> bool {
    record::valid_observation(value)
}
pub(super) fn valid_token(value: &str) -> bool {
    conflict::valid_token(value)
}
pub(super) use control::abandon_bound;
pub(super) fn observation_checkpoint(service: &super::RepositoryService) {
    let hook = service
        .observation_hook
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take();
    if let Some(hook) = hook {
        hook();
    }
}
pub(super) fn bound_to_request(
    service: &super::RepositoryService,
    root: &std::path::Path,
    id: super::OperationId,
) -> Result<bool, super::ReadError> {
    service.read_session(super::RepositoryOperation::Read,|db|Ok(db.query_row("SELECT EXISTS(SELECT 1 FROM local_context_sync_request_control c JOIN local_context_sync_records p ON p.operation_ulid=c.parent_operation WHERE c.role='start' AND c.parent_operation=?1 AND c.scope_key=?2)",rusqlite::params![id.to_string(),root.to_str()],|r|r.get(0))?))
}
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
