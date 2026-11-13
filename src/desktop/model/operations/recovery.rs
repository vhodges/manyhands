//! What running a recovery action does, and the control and words each
//! recovery action shows as.

use std::path::{Path, PathBuf};

use manyhands::{
    repository::{OperationId, transport::SshAuthority},
    results::{CheckpointEffect, RecoveryAction, RecoveryActionKind},
};

use super::Operations;
use crate::desktop::{
    model::list_state::ListState,
    services::requests::{DesktopRequestId, Intent, Refusals, RequestState},
};

/// What running a recovery action does.
pub(crate) enum RecoveryStep {
    /// Submit this command as a user job.
    Submit(Intent),
    /// Open the confirmation dialog for this command.
    Confirm(Intent),
    /// Submit the request again, under its own request ID.
    Retry(DesktopRequestId),
    /// Open the confirmation dialog for a fresh preview of the request.
    Prepare(DesktopRequestId),
    Cancel(DesktopRequestId),
    /// Open the Repositories page with this root checked.
    Inspect(PathBuf),
    /// Open the identity form of this root.
    IdentitySet(PathBuf),
    /// Open the Hosts page on this root and check this host: what it
    /// presents decides whether Approve or Replace is offered there
    /// (`host.approve`, `host.replace`).
    CheckHost {
        root: PathBuf,
        authority: SshAuthority,
    },
}

/// The host a recovery action names: `host:port`, with an address that
/// holds a colon in brackets, as the library writes it. `None` for text
/// in any other form.
pub(crate) fn parse_authority(text: &str) -> Option<SshAuthority> {
    let (host, port) = text.rsplit_once(':')?;
    let host = match host.strip_prefix('[') {
        Some(bracketed) => bracketed.strip_suffix(']')?,
        None if host.contains(':') => return None,
        None => host,
    };
    if host.is_empty() {
        return None;
    }
    Some(SshAuthority {
        host: host.to_owned(),
        port: port.parse().ok()?,
    })
}

impl Operations {
    /// The request this session holds by the request ID `action` names;
    /// `id` itself when it names none.
    fn named(&self, id: DesktopRequestId, action: &RecoveryAction) -> Option<DesktopRequestId> {
        match action.arguments.get("request_id") {
            None => Some(id),
            Some(named) => {
                let named = named.as_str()?;
                self.requests
                    .iter()
                    .find(|request| request.request_id.to_string() == named)
                    .map(|request| request.id)
            }
        }
    }

    /// R-78: whether the request `id` is never sent again, a later context
    /// request of its item having been sent (`Intent::kept_from_again_by`):
    /// a release once the item was prepared again, a prepare once its
    /// context was released, unless the library refused that release,
    /// touching nothing (owner, 2026-10-28). "Later" is the order the
    /// worker queued attempts in, as their `Queued` events came. The worker
    /// refuses it too.
    pub(crate) fn not_sent_again(&self, id: DesktopRequestId) -> bool {
        let (Some(request), Some(at)) = (self.request(id), self.sent.get(&id)) else {
            return false;
        };
        self.requests.iter().any(|later| {
            let refused = self
                .answers
                .get(&later.id)
                .is_some_and(Refusals::all_refused);
            self.sent.get(&later.id).is_some_and(|sent| sent > at)
                && request.intent.kept_from_again_by(&later.intent, refused)
        })
    }

    /// Whether the request `id`'s last answer named a pending operation of
    /// its own, offered for Abandon, and the operation area lists it for
    /// the request's repository (Task 17 fix 2).
    pub(crate) fn pending_row_listed(&self, id: DesktopRequestId) -> bool {
        let Some(request) = self.request(id) else {
            return false;
        };
        let RequestState::Finished(_, _, _, recovery, _) = &request.state else {
            return false;
        };
        let Some(operation) = recovery
            .iter()
            .find(|action| action.action == RecoveryActionKind::OperationAbandon)
            .and_then(|action| action.operation_id.as_deref())
        else {
            return false;
        };
        request
            .intent
            .root()
            .and_then(Path::to_str)
            .and_then(|root| self.pending.get(root))
            .and_then(ListState::rows)
            .is_some_and(|list| {
                list.items
                    .iter()
                    .any(|row| row.operation_id.as_deref() == Some(operation))
            })
    }

    /// For a comment's post, whether its last answer saved it; `None` for
    /// any other request, or a post that has not answered.
    pub(crate) fn post_saved(&self, id: DesktopRequestId) -> Option<bool> {
        let request = self.request(id)?;
        if !matches!(request.intent, Intent::CommentAdd { .. }) {
            return None;
        }
        match &request.state {
            RequestState::Finished(_, _, effects, ..) => {
                Some(effects.checkpoint == CheckpointEffect::Committed)
            }
            _ => None,
        }
    }

    /// Whether the recovery action `action` of the request `id` could
    /// complete a comment post that is not saved: Retry, Resume or Prepare
    /// again. The operation area never offers one; the post's composer is
    /// where it is sent again (decision 26). Abandon stays. The library
    /// does attach Resume to a pending operation's answer
    /// (`mutation/outcome.rs`, `DiscoveryPending` and the resumable
    /// operations it lists), so this is no mere defence.
    pub(crate) fn hides_recovery(&self, id: DesktopRequestId, action: &RecoveryAction) -> bool {
        matches!(
            recovery_control(action.action),
            RecoveryControl::Retry | RecoveryControl::Resume | RecoveryControl::Prepare
        ) && self.post_saved(id) == Some(false)
    }

    /// A recovery action's words on the request `id`: a saved comment's
    /// Retry only shares it, so it reads Retry sharing (decision 26).
    pub(crate) fn recovery_words(
        &self,
        id: DesktopRequestId,
        action: &RecoveryAction,
    ) -> &'static str {
        if recovery_control(action.action) == RecoveryControl::Retry
            && self.post_saved(id) == Some(true)
        {
            RETRY_SHARING
        } else {
            recovery_label(action.action)
        }
    }

    /// Whether `action`, offered on the request `id`, is a Retry that R-78
    /// forbids: the operation area says so.
    pub(crate) fn retry_not_sent_again(
        &self,
        id: DesktopRequestId,
        action: &RecoveryAction,
    ) -> bool {
        recovery_control(action.action) == RecoveryControl::Retry
            && self
                .named(id, action)
                .is_some_and(|named| self.not_sent_again(named))
    }

    /// What the recovery action `action`, offered on the request `id`,
    /// runs; `None` when D1 does not offer it, it names nothing this
    /// session can act on, or it is a Retry that R-78 forbids.
    pub(crate) fn recovery_step(
        &self,
        id: DesktopRequestId,
        action: &RecoveryAction,
    ) -> Option<RecoveryStep> {
        let request = self.request(id)?;
        let root = || -> Option<PathBuf> {
            match action.arguments.get("root") {
                Some(root) => root.as_str().map(PathBuf::from),
                None => request.intent.root().map(Path::to_owned),
            }
        };
        let operation = || {
            action
                .operation_id
                .as_deref()
                .and_then(|id| OperationId::parse(id).ok())
        };
        // A request this session holds, by the request ID the action
        // names; this one when it names none.
        let named = || self.named(id, action);
        // Decision 26: nothing here completes an unsaved post.
        if self.hides_recovery(id, action) {
            return None;
        }
        Some(match recovery_control(action.action) {
            RecoveryControl::IndexRefresh => {
                RecoveryStep::Submit(Intent::IndexRefresh { root: root()? })
            }
            RecoveryControl::IndexRebuild => {
                RecoveryStep::Submit(Intent::IndexRebuild { root: root()? })
            }
            RecoveryControl::Retry => {
                let named = named()?;
                if self.not_sent_again(named) {
                    return None;
                }
                RecoveryStep::Retry(named)
            }
            RecoveryControl::Prepare => {
                if !request.intent.confirmed() {
                    return None;
                }
                RecoveryStep::Prepare(id)
            }
            RecoveryControl::Cancel => RecoveryStep::Cancel(named()?),
            RecoveryControl::Resume => RecoveryStep::Submit(Intent::OperationResume {
                root: root()?,
                operation_id: operation()?,
            }),
            RecoveryControl::Abandon => RecoveryStep::Confirm(Intent::OperationAbandon {
                root: root()?,
                operation_id: operation()?,
            }),
            RecoveryControl::Inspect => RecoveryStep::Inspect(root()?),
            RecoveryControl::IdentitySet => RecoveryStep::IdentitySet(root()?),
            RecoveryControl::CheckHost => RecoveryStep::CheckHost {
                root: root()?,
                authority: parse_authority(action.arguments.get("authority")?.as_str()?)?,
            },
        })
    }
}

/// The control a recovery action shows as.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum RecoveryControl {
    IndexRefresh,
    IndexRebuild,
    Retry,
    Prepare,
    Cancel,
    Resume,
    Abandon,
    /// Open the Repositories page with the root checked.
    Inspect,
    /// Open the identity form of the root.
    IdentitySet,
    /// Open the Hosts page on the root and check the host named.
    CheckHost,
}

pub(crate) fn recovery_control(kind: RecoveryActionKind) -> RecoveryControl {
    match kind {
        RecoveryActionKind::IndexRefresh => RecoveryControl::IndexRefresh,
        RecoveryActionKind::IndexRebuild => RecoveryControl::IndexRebuild,
        RecoveryActionKind::RequestRetry => RecoveryControl::Retry,
        RecoveryActionKind::RequestPrepare => RecoveryControl::Prepare,
        RecoveryActionKind::RequestCancel => RecoveryControl::Cancel,
        RecoveryActionKind::OperationResume => RecoveryControl::Resume,
        RecoveryActionKind::OperationAbandon => RecoveryControl::Abandon,
        // `remote.history` has no button: repair runs in `manyhands-cli
        // remote history` until the repair sheet (vh-ez4y9). Inspect opens
        // the repository's details; the guidance text names the command.
        RecoveryActionKind::RepoInspect | RecoveryActionKind::RemoteHistory => {
            RecoveryControl::Inspect
        }
        RecoveryActionKind::RepoIdentitySet => RecoveryControl::IdentitySet,
        // The action names the host but not the key to pin: the Hosts
        // page checks the host, and offers Approve or Replace for the key
        // it presents.
        RecoveryActionKind::HostApprove | RecoveryActionKind::HostReplace => {
            RecoveryControl::CheckHost
        }
    }
}

/// A saved comment's Retry, which only shares it (decision 26).
pub(crate) const RETRY_SHARING: &str = "Retry sharing";

/// A recovery action's control, in fixed text.
pub(crate) fn recovery_label(kind: RecoveryActionKind) -> &'static str {
    match kind {
        RecoveryActionKind::IndexRefresh => "Refresh index",
        RecoveryActionKind::IndexRebuild => "Rebuild index",
        RecoveryActionKind::RequestRetry => "Retry",
        RecoveryActionKind::RequestPrepare => "Prepare again",
        RecoveryActionKind::RequestCancel => "Cancel",
        RecoveryActionKind::OperationResume => "Resume",
        RecoveryActionKind::OperationAbandon => "Abandon",
        RecoveryActionKind::RepoIdentitySet => "Set identity",
        RecoveryActionKind::HostApprove => "Approve host",
        RecoveryActionKind::HostReplace => "Replace host key",
        RecoveryActionKind::RepoInspect | RecoveryActionKind::RemoteHistory => "Inspect repository",
    }
}
