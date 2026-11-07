//! What a finished share says and who may hold its repository (design,
//! Lane B, Outcomes and routes; Part 0, Busy repositories).
//!
//! A share's outcome is read in fixed words, never the envelope's: the
//! views render `SyncOutcome` and never read envelope fields themselves.
//! A `busy` answer names no holder (fact 14), so the request model names
//! a possible one: a request of this session on the same repository whose
//! last attempt left its record accepted and was answered with a code the
//! library gives only after reserving the repository.

use std::path::Path;

use manyhands::{
    canonical::ItemId,
    repository::{AuthoringKind, transport::SshAuthority},
    results::ResultCode,
};

use super::{Operations, stopped_text};
use crate::desktop::{
    model::publication::{NOTHING_TO_SHARE, ShareTarget, SyncOutcome},
    services::{
        requests::{DesktopRequestId, Intent, RequestState},
        worker::RecordState,
    },
};

pub(crate) const SHARED: &str = "Shared";
pub(crate) const CURRENT: &str = "Same as the shared copy";
pub(crate) const NEEDS_ATTENTION: &str = "Needs attention";
pub(crate) const KEY_LOCKED: &str = "Your key is locked";
pub(crate) const CHOOSE_A_KEY: &str = "Choose a key to share with";
/// A host problem whose recovery names no host it can read (K-3).
pub(crate) const CHECK_THE_HOST: &str = "Check the host before sharing";
pub(crate) const NEEDS_IDENTITY: &str = "Set your identity before sharing";
/// `busy`, with no possible holder in this session (Busy repositories).
pub(crate) const ANOTHER_SHARE_HOLDS: &str = "Another share may be holding this repository";
pub(crate) const NOWHERE_TO_SHARE: &str = "This repository has nowhere to share to";
pub(crate) const BRANCH_DELETED: &str = "The shared copy of this item was deleted. \
     Republish it to share it again.";
/// What the texts call the repository's primary branch while its name is
/// not known: before navigation lists the repository, or when its
/// configuration names none (R-35).
pub(crate) const PRIMARY_UNKNOWN: &str = "the primary branch";

/// The primary branch's name, `primary` as the repository names it, or
/// `PRIMARY_UNKNOWN` (R-35).
pub(crate) fn primary_name(primary: Option<&str>) -> &str {
    primary.unwrap_or(PRIMARY_UNKNOWN)
}

/// The primary branch as a sentence's first words (R-35a): "The {primary}
/// branch", its name as it is, or "The primary branch" when not known.
pub(crate) fn primary_subject(primary: Option<&str>) -> String {
    match primary {
        Some(primary) => format!("The {primary} branch"),
        None => "The primary branch".to_owned(),
    }
}

/// A share of the primary branch `primary` whose shared copy was deleted:
/// it has nothing to republish, so no Republish… is promised (review M3).
pub(crate) fn main_branch_deleted(primary: &str) -> String {
    format!("The shared copy of {primary} was deleted.")
}
/// The item banner of a share or post stopped on a conflict (decision 13):
/// it offers Review changes (D5 B5, E-7).
pub(crate) const CONFLICT_BANNER: &str = "You and someone else both changed this since it \
     was last shared. Your work is safe.";

/// "Check {host} before sharing".
pub(crate) fn check_host_text(authority: Option<&SshAuthority>) -> String {
    match authority {
        Some(authority) => format!("Check {} before sharing", authority.host),
        None => CHECK_THE_HOST.to_owned(),
    }
}

/// `busy`, naming the request of this session that may hold the
/// repository by its label.
pub(crate) fn may_be_waiting_for(label: &str) -> String {
    format!("May be waiting for {label}")
}

/// What a share's outcome says (design, Outcomes and routes). `holder` is
/// the label of the request that may hold the repository, for `busy`.
/// `None` for a failure: the entry shows the library's message and code.
pub(crate) fn sync_text(outcome: &SyncOutcome, holder: Option<&str>) -> Option<String> {
    let words = match outcome {
        SyncOutcome::Published => SHARED,
        SyncOutcome::Current => CURRENT,
        SyncOutcome::Stopped { completed } => stopped_text(*completed),
        SyncOutcome::Conflict { .. } => NEEDS_ATTENTION,
        SyncOutcome::NeedsUnlock => KEY_LOCKED,
        SyncOutcome::NeedsKey => CHOOSE_A_KEY,
        SyncOutcome::NeedsHost { authority } => return Some(check_host_text(authority.as_ref())),
        SyncOutcome::NeedsIdentity => NEEDS_IDENTITY,
        SyncOutcome::Busy => {
            return Some(holder.map_or_else(|| ANOTHER_SHARE_HOLDS.to_owned(), may_be_waiting_for));
        }
        SyncOutcome::NoRemote => NOWHERE_TO_SHARE,
        SyncOutcome::BranchDeleted => BRANCH_DELETED,
        SyncOutcome::NothingToShare => NOTHING_TO_SHARE,
        SyncOutcome::Failed { .. } => return None,
    };
    Some(words.to_owned())
}

/// What a share's outcome says, as `sync_text` does, of the share `target`:
/// a deleted branch of the primary branch, named `primary` when known, has
/// its own words.
pub(crate) fn share_text(
    outcome: &SyncOutcome,
    target: ShareTarget,
    holder: Option<&str>,
    primary: Option<&str>,
) -> Option<String> {
    match (outcome, target) {
        (SyncOutcome::BranchDeleted, ShareTarget::Main) => {
            Some(main_branch_deleted(primary_name(primary)))
        }
        _ => sync_text(outcome, holder),
    }
}

/// The codes a request is answered only once it has reserved its
/// repository: a request that stopped with one, its record still
/// accepted, holds the reservation (fact 14, K-5).
const RESERVING_CODES: [ResultCode; 11] = [
    ResultCode::UnlockRequired,
    ResultCode::UnlockFailed,
    ResultCode::NoSelectedKey,
    ResultCode::KeyUnavailable,
    ResultCode::KeyRejected,
    ResultCode::HostApprovalRequired,
    ResultCode::HostReplacementRequired,
    ResultCode::TransportUnavailable,
    ResultCode::RemoteUnavailable,
    ResultCode::PushRejected,
    ResultCode::RemoteBranchDeleted,
];

impl Operations {
    /// The request of this session that may hold the repository the
    /// request `id` was answered `busy` for: the newest other request on
    /// the same root whose intent stops at safe points, whose last answer
    /// is a code given after reserving, and whose record `record` says is
    /// still accepted. `None` when there is none. A dismissed request is
    /// no longer listed, so it is not named.
    pub(crate) fn busy_holder(
        &self,
        id: DesktopRequestId,
        record: impl Fn(DesktopRequestId) -> Option<RecordState>,
    ) -> Option<DesktopRequestId> {
        let root = self.request(id)?.intent.root()?.to_owned();
        self.requests
            .iter()
            .filter(|request| request.id != id)
            .filter(|request| request.intent.root() == Some(root.as_path()))
            .filter(|request| request.intent.stops_at_safe_points())
            .filter(|request| {
                matches!(&request.state, RequestState::Finished(_, code, ..)
                    if RESERVING_CODES.contains(code))
            })
            .find(|request| record(request.id) == Some(RecordState::Accepted))
            .map(|request| request.id)
    }

    /// The newest share of the item `id` of `kind` in `root` (`item
    /// sync`) this session sent, whatever its state.
    pub(crate) fn latest_item_share(
        &self,
        root: &Path,
        kind: AuthoringKind,
        id: &ItemId,
    ) -> Option<DesktopRequestId> {
        self.requests
            .iter()
            .find(|request| {
                matches!(&request.intent, Intent::ItemSync { root: asked, kind: asked_kind, id: asked_id }
                    if asked == root && *asked_kind == kind && asked_id == id)
            })
            .map(|request| request.id)
    }
}

#[cfg(test)]
#[path = "outcomes_tests.rs"]
mod tests;
