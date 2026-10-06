use std::{fmt, str::FromStr};

use git2::Oid;

use super::{RemotePublicationEvidence, SynchronizationTarget};
use crate::{canonical::ItemId, repository::AuthoringKind};

const HEADS_PREFIX: &str = "refs/heads/";
const DOCUMENT_PREFIX: &str = "refs/heads/manyhands/document/";
const TICKET_PREFIX: &str = "refs/heads/manyhands/ticket/";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteRefTarget {
    remote_ref: String,
    tracking_ref: String,
}

impl RemoteRefTarget {
    pub fn remote_ref(&self) -> &str {
        &self.remote_ref
    }

    pub fn tracking_ref(&self) -> &str {
        &self.tracking_ref
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteRefPlan {
    remote_name: String,
    primary_branch: String,
    primary: RemoteRefTarget,
    fetch_refspecs: [String; 3],
}

impl RemoteRefPlan {
    pub fn from_configuration(
        remote_name: &str,
        primary_branch: &str,
    ) -> Result<Self, RemoteRefPlanError> {
        if !is_valid_git_short_name(remote_name) {
            return Err(RemoteRefPlanError::InvalidRemoteName);
        }
        if !is_valid_git_short_name(primary_branch)
            || primary_branch == "HEAD"
            || is_reserved_context_family(primary_branch)
        {
            return Err(RemoteRefPlanError::InvalidPrimaryBranch);
        }

        let primary = RemoteRefTarget {
            remote_ref: format!("{HEADS_PREFIX}{primary_branch}"),
            tracking_ref: format!("refs/remotes/{remote_name}/{primary_branch}"),
        };
        let fetch_refspecs = [
            format!("+{}:{}", primary.remote_ref, primary.tracking_ref),
            format!(
                "+refs/heads/manyhands/document/*:refs/remotes/{remote_name}/manyhands/document/*"
            ),
            format!("+refs/heads/manyhands/ticket/*:refs/remotes/{remote_name}/manyhands/ticket/*"),
        ];

        Ok(Self {
            remote_name: remote_name.to_owned(),
            primary_branch: primary_branch.to_owned(),
            primary,
            fetch_refspecs,
        })
    }

    pub fn remote_name(&self) -> &str {
        &self.remote_name
    }

    pub fn primary_branch(&self) -> &str {
        &self.primary_branch
    }

    pub fn primary(&self) -> &RemoteRefTarget {
        &self.primary
    }

    pub fn fetch_refspecs(&self) -> [&str; 3] {
        self.fetch_refspecs.each_ref().map(String::as_str)
    }

    pub fn primary_fetch_refspec(&self) -> String {
        format!("+{}:{}", self.primary.remote_ref, self.primary.tracking_ref)
    }

    pub fn context_fetch_refspec(&self, kind: AuthoringKind, item_id: &ItemId) -> [String; 2] {
        let context = self.context(kind, item_id);
        [
            self.primary_fetch_refspec(),
            format!("+{}:{}", context.remote_ref, context.tracking_ref),
        ]
    }

    pub fn primary_push_refspec(&self) -> String {
        format!("{}:{}", self.primary.remote_ref, self.primary.remote_ref)
    }

    pub fn context_push_refspec(&self, kind: AuthoringKind, item_id: &ItemId) -> String {
        let context = self.context(kind, item_id);
        format!("{}:{}", context.remote_ref, context.remote_ref)
    }

    pub fn context(&self, kind: AuthoringKind, item_id: &ItemId) -> RemoteRefTarget {
        let kind = kind_segment(kind);
        RemoteRefTarget {
            remote_ref: format!("refs/heads/manyhands/{kind}/{item_id}"),
            tracking_ref: format!(
                "refs/remotes/{}/manyhands/{kind}/{item_id}",
                self.remote_name
            ),
        }
    }

    pub fn classify_advertised_ref(&self, remote_ref: &str) -> Option<RemoteRefClassification> {
        if remote_ref == self.primary.remote_ref {
            return Some(RemoteRefClassification::Primary);
        }
        classify_context(remote_ref, DOCUMENT_PREFIX, AuthoringKind::Document)
            .or_else(|| classify_context(remote_ref, TICKET_PREFIX, AuthoringKind::Ticket))
    }

    pub fn tracking_ref_for(&self, remote_ref: &str) -> Option<String> {
        self.target_for_advertised_ref(remote_ref)
            .map(|target| target.tracking_ref)
    }

    pub(crate) fn target_for_advertised_ref(&self, remote_ref: &str) -> Option<RemoteRefTarget> {
        match self.classify_advertised_ref(remote_ref)? {
            RemoteRefClassification::Primary => Some(self.primary.clone()),
            RemoteRefClassification::RecognizedContext { kind, item_id } => {
                Some(self.context(kind, &item_id))
            }
            RemoteRefClassification::MalformedContext => {
                let branch = remote_ref.strip_prefix(HEADS_PREFIX)?;
                is_valid_git_short_name(branch).then(|| RemoteRefTarget {
                    remote_ref: remote_ref.to_owned(),
                    tracking_ref: format!("refs/remotes/{}/{branch}", self.remote_name),
                })
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RemoteRefClassification {
    Primary,
    RecognizedContext {
        kind: AuthoringKind,
        item_id: ItemId,
    },
    MalformedContext,
}

impl RemoteRefClassification {
    /// Ref-name recognition alone never proves that the referenced tree is a
    /// conforming Manyhands canonical tree.
    pub fn establishes_canonical_tree_validity(&self) -> bool {
        false
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoteRefPlanError {
    InvalidRemoteName,
    InvalidPrimaryBranch,
}

impl fmt::Display for RemoteRefPlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidRemoteName => "publication remote name is invalid",
            Self::InvalidPrimaryBranch => "primary branch name is invalid",
        })
    }
}

impl std::error::Error for RemoteRefPlanError {}

fn classify_context(
    remote_ref: &str,
    prefix: &str,
    kind: AuthoringKind,
) -> Option<RemoteRefClassification> {
    let remainder = remote_ref.strip_prefix(prefix)?;
    Some(match ItemId::from_str(remainder) {
        Ok(item_id) => RemoteRefClassification::RecognizedContext { kind, item_id },
        Err(_) => RemoteRefClassification::MalformedContext,
    })
}

fn kind_segment(kind: AuthoringKind) -> &'static str {
    match kind {
        AuthoringKind::Document => "document",
        AuthoringKind::Ticket => "ticket",
    }
}

fn is_reserved_context_family(branch: &str) -> bool {
    ["manyhands/document", "manyhands/ticket"]
        .into_iter()
        .any(|family| branch == family || branch.starts_with(&format!("{family}/")))
}

fn is_valid_git_short_name(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with(['-', '.'])
        && !value.ends_with(['.', '/'])
        && value != "@"
        && !value.contains("..")
        && !value.contains("@{")
        && !value
            .chars()
            .any(|character| character.is_control() || character.is_whitespace())
        && !value.contains(['~', '^', ':', '?', '*', '[', '\\'])
        && value.split('/').all(|component| {
            !component.is_empty() && !component.starts_with('.') && !component.ends_with(".lock")
        })
}

// Kept internal: orchestration consumes the pure plan, never a public caller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct CleanIntegrationPlan {
    pub final_oid: Oid,
    pub local_update: bool,
    /// Fetch-side planning only; publication still requires independent
    /// Push-direction observation and verification.
    pub push_needed: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum CleanIntegrationError<E> {
    PrimaryMissing,
    RemoteContextDeleted,
    HistoryUnknown,
    MergeRequired,
    Ancestry(E),
}

/// Tracking OIDs must describe the complete current Fetch advertisement, not
/// stale tracking refs for an absent remote branch. `is_ancestor(a, b)` asks
/// whether a is an ancestor of b; equal OIDs need no query. No mutation occurs.
pub(super) fn plan_clean_integration<E>(
    target: &SynchronizationTarget,
    local_oid: Oid,
    primary_tracking_oid: Option<Oid>,
    context_tracking_oid: Option<Oid>,
    publication_evidence: RemotePublicationEvidence,
    mut is_ancestor: impl FnMut(Oid, Oid) -> Result<bool, E>,
) -> Result<CleanIntegrationPlan, CleanIntegrationError<E>> {
    let primary = primary_tracking_oid.ok_or(CleanIntegrationError::PrimaryMissing)?;
    let (final_oid, remote_oid) = match target {
        SynchronizationTarget::Primary => (
            clean_descendant(local_oid, primary, &mut is_ancestor)?,
            Some(primary),
        ),
        SynchronizationTarget::Context { .. } => {
            let candidate = match context_tracking_oid {
                Some(remote) => clean_descendant(local_oid, remote, &mut is_ancestor)?,
                None => match publication_evidence {
                    RemotePublicationEvidence::NeverPublished => local_oid,
                    RemotePublicationEvidence::ObservedPublished => {
                        return Err(CleanIntegrationError::RemoteContextDeleted);
                    }
                    RemotePublicationEvidence::HistoryUnknown => {
                        return Err(CleanIntegrationError::HistoryUnknown);
                    }
                },
            };
            // Only return a plan after both context and primary relations pass.
            (
                clean_descendant(candidate, primary, &mut is_ancestor)?,
                context_tracking_oid,
            )
        }
    };
    Ok(CleanIntegrationPlan {
        final_oid,
        local_update: final_oid != local_oid,
        push_needed: remote_oid != Some(final_oid),
    })
}

fn clean_descendant<E>(
    local: Oid,
    remote: Oid,
    is_ancestor: &mut impl FnMut(Oid, Oid) -> Result<bool, E>,
) -> Result<Oid, CleanIntegrationError<E>> {
    if local == remote {
        return Ok(local);
    }
    if is_ancestor(local, remote).map_err(CleanIntegrationError::Ancestry)? {
        return Ok(remote);
    }
    if is_ancestor(remote, local).map_err(CleanIntegrationError::Ancestry)? {
        return Ok(local);
    }
    Err(CleanIntegrationError::MergeRequired)
}

#[cfg(test)]
#[path = "refs_tests.rs"]
mod tests;
