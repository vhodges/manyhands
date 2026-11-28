//! What the scheduler reads of a repository, off the UI thread: its polling
//! policy and whether it has anything to poll (D6 design B6).
//!
//! These are the scheduler's own calls, never part of an automatic poll's
//! (B4). A repository is polled only when its remotes show an SSH
//! publication remote (U-2): a poll without a remote writes state (Fact 31).
//! A repository whose first-time setup is published on its remote is polled
//! through that remote too, by the same policy, and `poll` takes the setup
//! path for it (Wave 06 Cycle 02, Part J).

use std::time::Duration;

use manyhands::repository::{OperationId, PollingOutcome, PollingStatusDto, RemoteListDto};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::desktop::{
    model::polling::{Authority, Policy, Root},
    services::{Backend, inflight::ActiveRow, poll, setup_poll::PendingSetup},
};

/// The publication remote a repository is checked against: its name and its
/// redacted push location, from the remotes read the scheduler already makes
/// (never a URL to connect to).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SharedCopy {
    pub name: String,
    pub location: String,
}

/// What a repository's reads came to.
pub(super) enum Read {
    /// It has an SSH publication remote: poll it by this policy.
    Polled {
        policy: Policy,
        last_completed: Option<OffsetDateTime>,
        /// The host the publication remote is reached at, when its location
        /// says (a host intent reaches the repositories on that host).
        authority: Option<Authority>,
        /// How the library's latest check ended, stored before this process
        /// ran (it stores no time for it).
        outcome: Option<PollingOutcome>,
        /// The remote the checks go to.
        shared_copy: Option<SharedCopy>,
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

/// The selected publication remote's name and redacted push location.
pub(super) fn shared_copy_of(remotes: &RemoteListDto) -> Option<SharedCopy> {
    let remote = remotes
        .items
        .iter()
        .find(|remote| remote.selected_for_publication && remote.publication_eligible)?;
    Some(SharedCopy {
        name: remote.name.clone(),
        location: remote.push_location.clone(),
    })
}

/// The host and port of the selected publication remote's redacted fetch
/// location: `ssh://[user@]host[:port]/path`, or the scp-like
/// `[user@]host:path` (port 22). `None` when it cannot be told.
pub(super) fn authority_of(remotes: &RemoteListDto) -> Option<Authority> {
    let remote = remotes
        .items
        .iter()
        .find(|remote| remote.selected_for_publication && remote.publication_eligible)?;
    let location = remote.fetch_location.as_str();
    let (scheme, rest) = match location.split_once("://") {
        Some((scheme, rest)) if scheme.eq_ignore_ascii_case("ssh") => (true, rest),
        Some(_) => return None,
        None => (false, location),
    };
    let rest = if scheme {
        rest.split('/').next()?
    } else {
        rest
    };
    let rest = rest.rsplit_once('@').map_or(rest, |(_, host)| host);
    let (host, port) = if let Some(bracketed) = rest.strip_prefix('[') {
        let (host, after) = bracketed.split_once(']')?;
        let port = match after.strip_prefix(':') {
            Some(port) if scheme => Some(port),
            _ => None,
        };
        (host, port)
    } else if scheme {
        match rest.split_once(':') {
            Some((host, port)) => (host, Some(port)),
            None => (rest, None),
        }
    } else {
        (rest.split_once(':')?.0, None)
    };
    if host.is_empty() {
        return None;
    }
    let port = match port {
        Some(port) => port.parse().ok()?,
        None => 22,
    };
    Some(Authority {
        host: host.to_owned(),
        port,
    })
}

/// `resolve` then `polling_status` of `root`: the policy the library
/// stores, read fresh.
pub(super) fn status_of(backend: &dyn Backend, root: &Root) -> Option<PollingStatusDto> {
    let repo = backend.resolve(std::path::Path::new(root)).ok()?;
    backend.polling_status(&repo).ok()
}

/// The remotes as the scheduler reads them for a repository whose setup is
/// polled through its own remote: a first-time setup, and a change in a
/// repository that has no SSH publication remote today's poll can check (none
/// at all, or one that is not SSH). The setup's remote stands where the
/// selected publication remote would (Part J). Any other repository's remotes
/// are as listed.
fn polled_through(remotes: RemoteListDto, setup: Option<&PendingSetup>) -> RemoteListDto {
    let Some(setup) = setup.filter(|setup| !setup.polls_current_remote()) else {
        return remotes;
    };
    RemoteListDto {
        items: remotes
            .items
            .into_iter()
            .map(|mut remote| {
                remote.selected_for_publication = remote.name == setup.remote;
                remote
            })
            .collect(),
        ..remotes
    }
}

/// Reads `root`: its setup, then its remotes, then its policy.
///
/// A repository with a first-time setup published on its remote is polled
/// through that remote by policy, as one with an SSH publication remote is
/// (Part J); a change on another remote is polled by the current remote's
/// policy, and `poll` adds the setup's poll after it. A repository with no
/// setup reads as it always has.
pub(super) fn read(backend: &dyn Backend, root: &Root) -> Read {
    let Ok(repo) = backend.resolve(std::path::Path::new(root)) else {
        return Read::Failed;
    };
    let setup = backend.pending_setup(std::path::Path::new(root));
    let Ok(remotes) = backend.publication_remotes(&repo) else {
        return Read::Failed;
    };
    let remotes = polled_through(remotes, setup.as_ref());
    if !ssh_publication(&remotes) {
        return Read::NotPolled;
    }
    let Ok(status) = backend.polling_status(&repo) else {
        return Read::Failed;
    };
    Read::Polled {
        authority: authority_of(&remotes),
        shared_copy: shared_copy_of(&remotes),
        outcome: status.latest_outcome,
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

#[cfg(test)]
mod tests {
    use manyhands::repository::RemoteDto;

    use super::*;

    fn list(location: &str) -> RemoteListDto {
        RemoteListDto {
            items: vec![RemoteDto {
                name: "origin".to_owned(),
                fetch_location: location.to_owned(),
                push_location: location.to_owned(),
                publication_eligible: true,
                selected_for_publication: true,
            }],
            complete: true,
        }
    }

    /// The host a redacted SSH location names, in the forms the library
    /// writes.
    #[test]
    fn authority_of_locations() {
        let at = |host: &str, port| {
            Some(Authority {
                host: host.to_owned(),
                port,
            })
        };
        for (location, expected) in [
            ("ssh://git@example.com/team/repo.git", at("example.com", 22)),
            (
                "ssh://git@example.com:2222/team/repo.git",
                at("example.com", 2222),
            ),
            ("SSH://git@[::1]:2222/repo.git", at("::1", 2222)),
            ("ssh://example.com/repo.git", at("example.com", 22)),
            ("git@example.com:team/repo.git", at("example.com", 22)),
            ("example.com:repo.git", at("example.com", 22)),
            ("[::1]:repo.git", at("::1", 22)),
            ("https://example.com/repo.git", None),
            ("ssh://git@:22/repo.git", None),
            ("ssh://git@example.com:notaport/repo.git", None),
        ] {
            assert_eq!(authority_of(&list(location)), expected, "{location}");
        }
        // Only the remote selected for publication says.
        let mut other = list("ssh://git@example.com/repo.git");
        other.items[0].selected_for_publication = false;
        assert_eq!(authority_of(&other), None);
    }
}
