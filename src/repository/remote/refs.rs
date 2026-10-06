use std::{fmt, str::FromStr};

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
        if !is_valid_git_short_name(primary_branch) || primary_branch == "HEAD" {
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
        match self.classify_advertised_ref(remote_ref)? {
            RemoteRefClassification::Primary => Some(self.primary.tracking_ref.clone()),
            RemoteRefClassification::RecognizedContext { kind, item_id } => {
                Some(self.context(kind, &item_id).tracking_ref)
            }
            RemoteRefClassification::MalformedContext => {
                let branch = remote_ref.strip_prefix(HEADS_PREFIX)?;
                is_valid_git_short_name(branch)
                    .then(|| format!("refs/remotes/{}/{branch}", self.remote_name))
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
