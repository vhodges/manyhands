//! Desktop requests: what the user asked for, as form values, and what
//! became of it.
//!
//! An `Intent` holds the non-secret values a command was asked with, so
//! every attempt builds a fresh `Mutation` from it and the library's
//! input types need not be `Clone`. A protected key generation holds only
//! that it is protected; the passphrase is given to each attempt and
//! never kept. A request keeps one `RequestId` for every attempt, so a
//! retry is the same request to the library.
//!
//! Nothing here derives `Debug`: an intent holds what the user typed.

use std::path::{Path, PathBuf};

use manyhands::{
    repository::{
        CommitIdentity, ConfirmationId, HostApproveInput, HostReplaceInput, IndexRebuildInput,
        IndexRefreshInput, InitialsChange, KeyClearInput, KeyDeleteInput, KeyGenerateInput,
        KeyImportInput, KeyRemoveInput, KeySelectInput, Mutation, OperationAbandonInput,
        OperationId, OperationResumeInput, ProgressStage, RemoteAddInput, RemoteRemoveInput,
        RemoteSelectInput, RepoCreateInput, RepoEnableInput, RepoIdentitySetInput, RepoRemoveInput,
        RequestId, SharedKeyId,
        keys::{KeyProtection, KeyStore, SecretPassphrase},
        transport::SshAuthority,
    },
    results::{Effects, Outcome, RecoveryAction, ResultCode},
};

use super::{JobKind, KeyStoreUnavailable};

/// Which queue a job waits in. The worker always takes the next user job
/// before any background job.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Lane {
    User,
    Background,
}

/// One command the desktop can ask the library for, with the values it
/// was asked with. One variant per command D1 offers.
#[derive(Clone)]
pub(crate) enum Intent {
    RepoCreate {
        root: PathBuf,
        primary_branch: String,
        /// Name and email.
        identity: Option<(String, String)>,
        slug_prefix: Option<String>,
    },
    RepoEnable {
        root: PathBuf,
        primary_branch: String,
        identity: Option<(String, String)>,
        slug_prefix: Option<String>,
    },
    RepoRemove {
        root: PathBuf,
    },
    RepoIdentitySet {
        root: PathBuf,
        name: String,
        email: String,
        initials: InitialsChange,
    },
    RemoteAdd {
        root: PathBuf,
        name: String,
        url: String,
    },
    RemoteRemove {
        root: PathBuf,
        name: String,
    },
    RemoteSelect {
        root: PathBuf,
        name: Option<String>,
    },
    IndexRefresh {
        root: PathBuf,
    },
    IndexRebuild {
        root: PathBuf,
    },
    /// Whether the key is protected, never the passphrase.
    KeyGenerate {
        label: String,
        protected: bool,
    },
    KeyImport {
        label: String,
        private_key_path: PathBuf,
        public_key_path: Option<PathBuf>,
    },
    KeySelect {
        id: SharedKeyId,
    },
    KeyClear,
    KeyRemove {
        id: SharedKeyId,
    },
    KeyDelete {
        id: SharedKeyId,
    },
    HostApprove {
        root: PathBuf,
        authority: SshAuthority,
        algorithm: String,
        fingerprint: String,
    },
    HostReplace {
        root: PathBuf,
        authority: SshAuthority,
        algorithm: String,
        fingerprint: String,
        old_fingerprint: String,
    },
    OperationResume {
        root: PathBuf,
        operation_id: OperationId,
    },
    OperationAbandon {
        root: PathBuf,
        operation_id: OperationId,
    },
}

/// Why an intent could not be made into a command. Nothing was asked of
/// the library.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum NotBuilt {
    /// A protected key generation was given no passphrase: the user is to
    /// enter it again.
    MissingSecret,
    /// A key command's store could not be found.
    KeyStoreUnavailable,
}

impl From<KeyStoreUnavailable> for NotBuilt {
    fn from(_: KeyStoreUnavailable) -> Self {
        Self::KeyStoreUnavailable
    }
}

fn identity(identity: &Option<(String, String)>) -> Option<CommitIdentity> {
    identity.as_ref().map(|(name, email)| CommitIdentity {
        name: name.clone(),
        email: email.clone(),
    })
}

impl Intent {
    /// The library command for one attempt. `secret` is the passphrase of
    /// a protected key generation and is dropped by every other intent.
    /// `store` is asked for only by the commands that need the key store.
    pub(crate) fn build(
        &self,
        secret: Option<SecretPassphrase>,
        store: impl FnOnce() -> Result<KeyStore, KeyStoreUnavailable>,
    ) -> Result<Mutation, NotBuilt> {
        Ok(match self {
            Self::RepoCreate {
                root,
                primary_branch,
                identity: id,
                slug_prefix,
            } => Mutation::RepoCreate(RepoCreateInput {
                root: root.clone(),
                primary_branch: primary_branch.clone(),
                identity: identity(id),
                slug_prefix: slug_prefix.clone(),
            }),
            Self::RepoEnable {
                root,
                primary_branch,
                identity: id,
                slug_prefix,
            } => Mutation::RepoEnable(RepoEnableInput {
                root: root.clone(),
                primary_branch: primary_branch.clone(),
                identity: identity(id),
                slug_prefix: slug_prefix.clone(),
            }),
            Self::RepoRemove { root } => {
                Mutation::RepoRemove(RepoRemoveInput { root: root.clone() })
            }
            Self::RepoIdentitySet {
                root,
                name,
                email,
                initials,
            } => Mutation::RepoIdentitySet(RepoIdentitySetInput {
                root: root.clone(),
                name: name.clone(),
                email: email.clone(),
                initials: initials.clone(),
            }),
            Self::RemoteAdd { root, name, url } => Mutation::RemoteAdd(RemoteAddInput {
                root: root.clone(),
                name: name.clone(),
                url: url.clone(),
            }),
            Self::RemoteRemove { root, name } => Mutation::RemoteRemove(RemoteRemoveInput {
                root: root.clone(),
                name: name.clone(),
            }),
            Self::RemoteSelect { root, name } => Mutation::RemoteSelect(RemoteSelectInput {
                root: root.clone(),
                name: name.clone(),
            }),
            Self::IndexRefresh { root } => {
                Mutation::IndexRefresh(IndexRefreshInput { root: root.clone() })
            }
            Self::IndexRebuild { root } => {
                Mutation::IndexRebuild(IndexRebuildInput { root: root.clone() })
            }
            Self::KeyGenerate { label, protected } => {
                let protection = if *protected {
                    KeyProtection::Passphrase(secret.ok_or(NotBuilt::MissingSecret)?)
                } else {
                    KeyProtection::Unencrypted
                };
                Mutation::KeyGenerate(KeyGenerateInput {
                    store: store()?,
                    label: label.clone(),
                    protection,
                })
            }
            Self::KeyImport {
                label,
                private_key_path,
                public_key_path,
            } => Mutation::KeyImport(KeyImportInput {
                label: label.clone(),
                private_key_path: private_key_path.clone(),
                public_key_path: public_key_path.clone(),
            }),
            Self::KeySelect { id } => Mutation::KeySelect(KeySelectInput { id: *id }),
            Self::KeyClear => Mutation::KeyClear(KeyClearInput {}),
            Self::KeyRemove { id } => Mutation::KeyRemove(KeyRemoveInput { id: *id }),
            Self::KeyDelete { id } => Mutation::KeyDelete(KeyDeleteInput {
                store: store()?,
                id: *id,
            }),
            Self::HostApprove {
                root,
                authority,
                algorithm,
                fingerprint,
            } => Mutation::HostApprove(HostApproveInput {
                root: root.clone(),
                authority: authority.clone(),
                algorithm: algorithm.clone(),
                fingerprint: fingerprint.clone(),
            }),
            Self::HostReplace {
                root,
                authority,
                algorithm,
                fingerprint,
                old_fingerprint,
            } => Mutation::HostReplace(HostReplaceInput {
                root: root.clone(),
                authority: authority.clone(),
                algorithm: algorithm.clone(),
                fingerprint: fingerprint.clone(),
                old_fingerprint: old_fingerprint.clone(),
            }),
            Self::OperationResume { root, operation_id } => {
                Mutation::OperationResume(OperationResumeInput {
                    root: root.clone(),
                    operation_id: *operation_id,
                })
            }
            Self::OperationAbandon { root, operation_id } => {
                Mutation::OperationAbandon(OperationAbandonInput {
                    root: root.clone(),
                    operation_id: *operation_id,
                })
            }
        })
    }

    /// Whether the command needs confirmation: `prepare` previews it, and
    /// `execute` runs it only with the confirmation the preview gave. The
    /// library's own list; a test holds the two together.
    pub(crate) fn confirmed(&self) -> bool {
        match self {
            Self::RepoCreate { .. }
            | Self::RepoEnable { .. }
            | Self::RepoRemove { .. }
            | Self::RepoIdentitySet { .. }
            | Self::RemoteRemove { .. }
            | Self::KeyDelete { .. }
            | Self::HostApprove { .. }
            | Self::HostReplace { .. }
            | Self::OperationAbandon { .. } => true,
            Self::RemoteAdd { .. }
            | Self::RemoteSelect { .. }
            | Self::IndexRefresh { .. }
            | Self::IndexRebuild { .. }
            | Self::KeyGenerate { .. }
            | Self::KeyImport { .. }
            | Self::KeySelect { .. }
            | Self::KeyClear
            | Self::KeyRemove { .. }
            | Self::OperationResume { .. } => false,
        }
    }

    /// What the operation area calls the command. Fixed text: no value
    /// the user typed.
    pub(crate) fn label(&self) -> &'static str {
        match self {
            Self::RepoCreate { .. } => "Create repository",
            Self::RepoEnable { .. } => "Add repository",
            Self::RepoRemove { .. } => "Remove repository",
            Self::RepoIdentitySet { .. } => "Set identity",
            Self::RemoteAdd { .. } => "Add remote",
            Self::RemoteRemove { .. } => "Remove remote",
            Self::RemoteSelect { .. } => "Select remote",
            Self::IndexRefresh { .. } => "Refresh index",
            Self::IndexRebuild { .. } => "Rebuild index",
            Self::KeyGenerate { .. } => "Generate key",
            Self::KeyImport { .. } => "Import key",
            Self::KeySelect { .. } => "Select key",
            Self::KeyClear => "Clear key selection",
            Self::KeyRemove { .. } => "Unregister key",
            Self::KeyDelete { .. } => "Delete key",
            Self::HostApprove { .. } => "Approve host",
            Self::HostReplace { .. } => "Replace host key",
            Self::OperationResume { .. } => "Resume operation",
            Self::OperationAbandon { .. } => "Abandon operation",
        }
    }

    /// The repository the command is about, when it is about one.
    pub(crate) fn root(&self) -> Option<&Path> {
        match self {
            Self::RepoCreate { root, .. }
            | Self::RepoEnable { root, .. }
            | Self::RepoRemove { root }
            | Self::RepoIdentitySet { root, .. }
            | Self::RemoteAdd { root, .. }
            | Self::RemoteRemove { root, .. }
            | Self::RemoteSelect { root, .. }
            | Self::IndexRefresh { root }
            | Self::IndexRebuild { root }
            | Self::HostApprove { root, .. }
            | Self::HostReplace { root, .. }
            | Self::OperationResume { root, .. }
            | Self::OperationAbandon { root, .. } => Some(root),
            Self::KeyGenerate { .. }
            | Self::KeyImport { .. }
            | Self::KeySelect { .. }
            | Self::KeyClear
            | Self::KeyRemove { .. }
            | Self::KeyDelete { .. } => None,
        }
    }

    /// The root a create or enable registers: once it succeeds, the worker
    /// refreshes it in the background (decision 4).
    pub(crate) fn onboarded_root(&self) -> Option<PathBuf> {
        match self {
            Self::RepoCreate { root, .. } | Self::RepoEnable { root, .. } => Some(root.clone()),
            _ => None,
        }
    }

    /// Whether the command can change the items of its repository, so
    /// that its open tabs read their items again once it has finished
    /// (ruling R-B2). Repository removal closes the tabs instead; the
    /// identity, remote, key and host commands change no item.
    pub(crate) fn changes_items(&self) -> bool {
        match self {
            Self::IndexRefresh { .. }
            | Self::IndexRebuild { .. }
            | Self::OperationResume { .. }
            | Self::OperationAbandon { .. }
            | Self::RepoCreate { .. }
            | Self::RepoEnable { .. } => true,
            Self::RepoRemove { .. }
            | Self::RepoIdentitySet { .. }
            | Self::RemoteAdd { .. }
            | Self::RemoteRemove { .. }
            | Self::RemoteSelect { .. }
            | Self::KeyGenerate { .. }
            | Self::KeyImport { .. }
            | Self::KeySelect { .. }
            | Self::KeyClear
            | Self::KeyRemove { .. }
            | Self::KeyDelete { .. }
            | Self::HostApprove { .. }
            | Self::HostReplace { .. } => false,
        }
    }

    /// The kind the backend's seam knows the job by.
    pub(crate) fn job_kind(&self) -> JobKind {
        match self {
            Self::RepoCreate { .. } => JobKind::RepoCreate,
            Self::RepoEnable { .. } => JobKind::RepoEnable,
            Self::RepoRemove { .. } => JobKind::RepoRemove,
            Self::RepoIdentitySet { .. } => JobKind::RepoIdentitySet,
            Self::RemoteAdd { .. } => JobKind::RemoteAdd,
            Self::RemoteRemove { .. } => JobKind::RemoteRemove,
            Self::RemoteSelect { .. } => JobKind::RemoteSelect,
            Self::IndexRefresh { .. } => JobKind::IndexRefresh,
            Self::IndexRebuild { .. } => JobKind::IndexRebuild,
            Self::KeyGenerate { .. } => JobKind::KeyGenerate,
            Self::KeyImport { .. } => JobKind::KeyImport,
            Self::KeySelect { .. } => JobKind::KeySelect,
            Self::KeyClear => JobKind::KeyClear,
            Self::KeyRemove { .. } => JobKind::KeyRemove,
            Self::KeyDelete { .. } => JobKind::KeyDelete,
            Self::HostApprove { .. } => JobKind::HostApprove,
            Self::HostReplace { .. } => JobKind::HostReplace,
            Self::OperationResume { .. } => JobKind::OperationResume,
            Self::OperationAbandon { .. } => JobKind::OperationAbandon,
        }
    }
}

/// A desktop request, by the number the worker gave it in this session.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub(crate) struct DesktopRequestId(pub(crate) u64);

/// Why a request was not sent to the library. Nothing was asked of it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum NotSent {
    /// A protected key generation came without its passphrase; the user
    /// is to enter it again and retry.
    SecretRequired,
    /// The key store could not be found.
    KeyStoreUnavailable,
}

impl From<NotBuilt> for NotSent {
    fn from(not_built: NotBuilt) -> Self {
        match not_built {
            NotBuilt::MissingSecret => Self::SecretRequired,
            NotBuilt::KeyStoreUnavailable => Self::KeyStoreUnavailable,
        }
    }
}

/// What the operation area lists: one request of this session.
#[derive(Clone)]
pub(crate) struct DesktopRequest {
    pub id: DesktopRequestId,
    /// The library's replay identity, kept for every attempt.
    pub request_id: RequestId,
    pub intent: Intent,
    /// The confirmation and lane this attempt was queued with. No view
    /// shows them; tests read them to check what an attempt carried.
    #[cfg_attr(not(test), allow(dead_code))]
    pub confirmation: Option<ConfirmationId>,
    #[cfg_attr(not(test), allow(dead_code))]
    pub lane: Lane,
    /// Which submission of the request this is: 1, then one more for each
    /// retry or Prepare again. A `busy` background job taken again is
    /// the same attempt.
    pub attempt: u32,
    pub state: RequestState,
}

/// Where a request stands.
#[derive(Clone)]
pub(crate) enum RequestState {
    Queued,
    Running {
        /// The last stage its progress named, if any yet.
        stage: Option<ProgressStage>,
        /// Whether the library has accepted it: from then on a local
        /// command cannot be cancelled.
        accepted: bool,
    },
    /// The envelope's outcome, code, effects, recovery and message: what
    /// is shown, and nothing that could hold input.
    Finished(Outcome, ResultCode, Effects, Vec<RecoveryAction>, String),
    /// It was removed from the queue before it ran: cancelled while it
    /// waited, or emptied from the queue on quit. Nothing was asked of the
    /// library.
    Removed,
    /// The user dismissed it from the operation area.
    Dismissed,
    /// It was never sent to the library.
    NotSent(NotSent),
}

#[cfg(test)]
#[path = "requests_tests.rs"]
mod tests;
