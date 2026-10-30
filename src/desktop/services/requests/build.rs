//! How an intent becomes a library command, one attempt at a time.

use manyhands::repository::{
    CommitIdentity, ContextPrepareInput, ContextReleaseInput, DocumentCreateInput, DocumentDraft,
    DocumentMoveInput, DocumentRepairInput, DocumentSaveInput, FolderCreateInput, HostApproveInput,
    HostReplaceInput, IndexRebuildInput, IndexRefreshInput, KeyClearInput, KeyDeleteInput,
    KeyGenerateInput, KeyImportInput, KeyRemoveInput, KeySelectInput, Mutation,
    OperationAbandonInput, OperationResumeInput, RemoteAddInput, RemoteRemoveInput,
    RemoteSelectInput, RepoCreateInput, RepoEnableInput, RepoIdentitySetInput, RepoRemoveInput,
    TicketCreateInput, TicketDraft, TicketRepairInput, TicketSaveInput,
    keys::{KeyProtection, KeyStore, SecretPassphrase},
};

use super::Intent;
use crate::desktop::services::KeyStoreUnavailable;

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
            Self::DocumentSave {
                root,
                id,
                title,
                body,
                observation,
            } => Mutation::DocumentSave(DocumentSaveInput {
                root: root.clone(),
                id: id.clone(),
                draft: DocumentDraft {
                    title: title.clone(),
                    body: body.clone(),
                },
                observation: observation.clone(),
            }),
            Self::TicketSave {
                root,
                id,
                title,
                ticket_type,
                status,
                project,
                team,
                body,
                deps,
                parent,
                observation,
            } => Mutation::TicketSave(TicketSaveInput {
                root: root.clone(),
                id: id.clone(),
                draft: TicketDraft {
                    title: title.clone(),
                    ticket_type: ticket_type.clone(),
                    status: status.clone(),
                    project: project.clone(),
                    team: team.clone(),
                    body: body.clone(),
                },
                deps: deps.clone(),
                parent: parent.clone(),
                observation: observation.clone(),
            }),
            Self::ContextPrepare {
                root,
                kind,
                id,
                intent,
            } => Mutation::ContextPrepare(ContextPrepareInput {
                root: root.clone(),
                kind: *kind,
                id: id.clone(),
                intent: *intent,
            }),
            Self::ContextRelease { root, kind, id } => {
                Mutation::ContextRelease(ContextReleaseInput {
                    root: root.clone(),
                    kind: *kind,
                    id: id.clone(),
                })
            }
            Self::DocumentCreate {
                root,
                id,
                path,
                title,
                body,
            } => Mutation::DocumentCreate(DocumentCreateInput {
                root: root.clone(),
                id: id.clone(),
                path: path.clone(),
                draft: DocumentDraft {
                    title: title.clone(),
                    body: body.clone(),
                },
            }),
            Self::TicketCreate {
                root,
                id,
                title,
                ticket_type,
                status,
                project,
                team,
                body,
                deps,
                parent,
            } => Mutation::TicketCreate(TicketCreateInput {
                root: root.clone(),
                id: id.clone(),
                draft: TicketDraft {
                    title: title.clone(),
                    ticket_type: ticket_type.clone(),
                    status: status.clone(),
                    project: project.clone(),
                    team: team.clone(),
                    body: body.clone(),
                },
                deps: deps.clone(),
                parent: parent.clone(),
                initials: None,
            }),
            Self::DocumentMove {
                root,
                id,
                observation,
                destination_path,
                destination_observation,
            } => Mutation::DocumentMove(DocumentMoveInput {
                root: root.clone(),
                id: id.clone(),
                observation: observation.clone(),
                destination_path: destination_path.clone(),
                destination_observation: destination_observation.clone(),
            }),
            Self::FolderCreate { root, path } => Mutation::FolderCreate(FolderCreateInput {
                root: root.clone(),
                path: path.clone(),
            }),
            Self::DocumentRepair {
                root,
                path,
                observation,
                source,
            } => Mutation::DocumentRepair(DocumentRepairInput {
                root: root.clone(),
                path: path.clone(),
                observation: observation.clone(),
                source: source.clone(),
            }),
            Self::TicketRepair {
                root,
                path,
                observation,
                source,
            } => Mutation::TicketRepair(TicketRepairInput {
                root: root.clone(),
                path: path.clone(),
                observation: observation.clone(),
                source: source.clone(),
            }),
        })
    }
}
