//! Activity's rows (Wave 04 Cycle 03b, Part A): the types of one store of
//! rows, the request rows (S1), the state rows (S2-S14) and the words every
//! row says (Part B). The
//! model has no GPUI dependency and reads no view module.

use manyhands::{canonical::ItemId, repository::AuthoringKind};
use time::{OffsetDateTime, UtcOffset};

use self::{
    requests::{RequestFacts, RequestInputs, request_rows},
    rows::Derived,
    states::{DraftsInput, RepositoryInput, StateInputs, state_rows},
};
use super::{
    operations::Operations,
    polling::{KeyId, PollSchedule},
    stamp::RepoKey,
};
use crate::desktop::services::requests::DesktopRequestId;

pub(crate) mod copy;
mod intent_words;
pub(crate) mod lifecycle_lines;
pub(crate) mod log;
pub(crate) mod requests;
pub(crate) mod rows;
pub(crate) mod states;

/// What the Workspace hands the derivation at each refresh (design A2): the
/// model reads nothing else.
pub(crate) struct Inputs<'a> {
    pub operations: &'a Operations,
    pub schedule: &'a PollSchedule,
    pub repositories: &'a [RepositoryInput],
    pub drafts: DraftsInput,
    pub requests: &'a dyn Fn(DesktopRequestId) -> RequestFacts,
    pub conflict_waits: &'a dyn Fn(&RepoKey, &str) -> bool,
    pub title: &'a dyn Fn(&RepoKey, AuthoringKind, &ItemId) -> Option<String>,
    pub primary: &'a dyn Fn(&RepoKey) -> Option<String>,
    pub unlock_target: &'a dyn Fn(&KeyId) -> bool,
    pub lifecycle_running: &'a dyn Fn(&RepoKey, &ItemId) -> bool,
}

/// Every row there is, from the requests (S1) and the states (S2-S14): it
/// hides no row (R-51) and assigns no attention (the log does, A3).
pub(crate) fn derive(inputs: &Inputs<'_>) -> Vec<Derived> {
    let mut rows = request_rows(&RequestInputs {
        operations: inputs.operations,
        requests: inputs.requests,
        title: inputs.title,
        primary: inputs.primary,
    });
    rows.extend(state_rows(&StateInputs {
        operations: inputs.operations,
        schedule: inputs.schedule,
        repositories: inputs.repositories,
        drafts: &inputs.drafts,
        conflict_waits: inputs.conflict_waits,
        title: inputs.title,
        primary: inputs.primary,
        unlock_target: inputs.unlock_target,
        lifecycle_running: inputs.lifecycle_running,
    }));
    rows
}

/// A row's time and its tooltip's time line (B6): "HH:MM", and
/// "YYYY-MM-DD HH:MM" in the same zone. With no offset both are UTC and the
/// tooltip says so.
#[allow(dead_code)] // Used from Task 9
pub(crate) fn activity_time(at: OffsetDateTime, offset: Option<UtcOffset>) -> (String, String) {
    let local = at.to_offset(offset.unwrap_or(UtcOffset::UTC));
    let clock = format!("{:02}:{:02}", local.hour(), local.minute());
    let date = format!(
        "{:04}-{:02}-{:02}",
        local.year(),
        u8::from(local.month()),
        local.day()
    );
    let zone = if offset.is_none() { " UTC" } else { "" };
    (clock.clone(), format!("{date} {clock}{zone}"))
}

#[cfg(test)]
#[path = "activity_tests.rs"]
mod tests;
