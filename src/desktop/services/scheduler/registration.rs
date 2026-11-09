//! What the scheduler reads of a repository, off the UI thread: its polling
//! policy and whether it has anything to poll (D6 design B6).
//!
//! These are the scheduler's own calls, never part of an automatic poll's
//! (B4). A repository is polled only when its remotes show an SSH
//! publication remote (U-2): a poll without a remote writes state (Fact 31).

use std::time::Duration;

use manyhands::repository::{OperationId, PollingStatusDto, RemoteListDto};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::desktop::{
    model::polling::{Policy, Root},
    services::{Backend, inflight::ActiveRow, poll},
};

/// What a repository's reads came to.
pub(super) enum Read {
    /// It has an SSH publication remote: poll it by this policy.
    Polled {
        policy: Policy,
        last_completed: Option<OffsetDateTime>,
    },
    /// It has none: it is never polled ("No shared copy, nothing to check").
    NotPolled,
    /// A read failed. Nothing is learned, so nothing changes; the next
    /// window activation or clock event reads again.
    Failed,
}

/// Whether the remotes show one selected for publication that SSH reaches
/// (U-2): `publication_eligible` is true only for an `ssh://` or scp-style
/// endpoint that parses for fetch and push.
pub(super) fn ssh_publication(remotes: &RemoteListDto) -> bool {
    remotes
        .items
        .iter()
        .any(|remote| remote.selected_for_publication && remote.publication_eligible)
}

/// `resolve` then `polling_status` of `root`: the policy the library
/// stores, read fresh.
pub(super) fn status_of(backend: &dyn Backend, root: &Root) -> Option<PollingStatusDto> {
    let repo = backend.resolve(std::path::Path::new(root)).ok()?;
    backend.polling_status(&repo).ok()
}

/// Reads `root`: its remotes, then its policy.
pub(super) fn read(backend: &dyn Backend, root: &Root) -> Read {
    let Ok(repo) = backend.resolve(std::path::Path::new(root)) else {
        return Read::Failed;
    };
    let Ok(remotes) = backend.publication_remotes(&repo) else {
        return Read::Failed;
    };
    if !ssh_publication(&remotes) {
        return Read::NotPolled;
    }
    let Ok(status) = backend.polling_status(&repo) else {
        return Read::Failed;
    };
    Read::Polled {
        policy: policy_of(&status),
        // When the remote's branches were last observed completely: the
        // last completion, for fairness (R-7). Restored at launch.
        last_completed: parsed(status.latest_observed_at.as_deref()),
    }
}

/// The model's copy of `status`.
pub(super) fn policy_of(status: &PollingStatusDto) -> Policy {
    Policy {
        enabled: status.enabled,
        paused: status.paused,
        interval: Duration::from_secs(status.interval_seconds),
        backoff: status.backoff_seconds.map(Duration::from_secs),
        recovery_suspended: status.recovery_suspended,
        next_eligible_at: poll::stored_deadline(status),
        active_operation: status
            .active_operation_id
            .as_deref()
            .and_then(|id| OperationId::parse(id).ok()),
    }
}

/// What `polling_status.active_operation_id` named, read after an answer
/// (`scheduler::apply`'s `ActiveRow`): nothing known when the read failed.
pub(super) fn active_of(status: Option<&PollingStatusDto>) -> ActiveRow {
    match status {
        None => ActiveRow::Unread,
        Some(status) => match status.active_operation_id.as_deref() {
            None => ActiveRow::Nothing,
            Some(id) => OperationId::parse(id).map_or(ActiveRow::Unread, ActiveRow::Operation),
        },
    }
}

fn parsed(stored: Option<&str>) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(stored?, &Rfc3339).ok()
}
