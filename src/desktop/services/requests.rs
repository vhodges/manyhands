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
        AuthoringKind, ConfirmationId, ContextPrepareIntent, InitialsChange, OperationId,
        ProgressStage, RelationshipWrite, RequestId, SharedKeyId, transport::SshAuthority,
    },
    results::{Effects, FailureClass, Outcome, RecoveryAction, RecoveryActionKind, ResultCode},
};

use manyhands::canonical::ItemId;

mod build;

pub(crate) use build::NotBuilt;

use super::JobKind;

/// Which queue a job waits in. The worker always takes the next user job
/// before any background job.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Lane {
    User,
    Background,
}

/// One command the desktop can ask the library for, with the values it
/// was asked with. One variant per command D1 offers, and the two saves
/// of a draft.
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
    /// A draft's revision saved as the document, from the observation
    /// of the draft's base. It holds the body.
    DocumentSave {
        root: PathBuf,
        id: ItemId,
        title: String,
        body: String,
        observation: Option<String>,
    },
    /// A draft's revision saved as the ticket. `deps` and `parent` are
    /// left as the file has them when the draft did not change them
    /// (ruling R13).
    TicketSave {
        root: PathBuf,
        id: ItemId,
        title: String,
        ticket_type: String,
        status: String,
        project: Option<String>,
        team: Option<String>,
        body: String,
        deps: RelationshipWrite<Vec<String>>,
        parent: RelationshipWrite<String>,
        observation: String,
    },
    /// Prepares the work context an edit or a new item is written in. No
    /// token: neither context command records a checkpoint, so the desktop
    /// never sends one again across the other (R-78, `undone_by`).
    ContextPrepare {
        root: PathBuf,
        kind: AuthoringKind,
        id: ItemId,
        intent: ContextPrepareIntent,
    },
    /// Releases the work context of an item. No token: neither context
    /// command records a checkpoint, so the desktop never sends one again
    /// across the other (R-78, `undone_by`).
    ContextRelease {
        root: PathBuf,
        kind: AuthoringKind,
        id: ItemId,
    },
    /// A new document at `path`. It holds the body. No token: the ID is
    /// fixed when the draft is written; the same request is answered from
    /// its record, and a new one finds the item `already_applied`, an
    /// earlier attempt's write not committed (`external_change`), or the
    /// path occupied (`occupied_path`).
    DocumentCreate {
        root: PathBuf,
        id: ItemId,
        path: PathBuf,
        title: String,
        body: String,
    },
    /// A new ticket. It holds the body. No token: the ID is fixed when the
    /// draft is written; the same request is answered from its record, and
    /// a new one finds the item `already_applied`, an earlier attempt's
    /// write not committed (`external_change`), or the path occupied
    /// (`occupied_path`).
    TicketCreate {
        root: PathBuf,
        id: ItemId,
        title: String,
        ticket_type: String,
        status: String,
        project: Option<String>,
        team: Option<String>,
        body: String,
        deps: Vec<String>,
        parent: Option<String>,
    },
    /// A document moved to another path.
    DocumentMove {
        root: PathBuf,
        id: ItemId,
        observation: Option<String>,
        destination_path: PathBuf,
        destination_observation: String,
    },
    /// A folder below `docs/`, by its path with forward slashes.
    FolderCreate {
        root: PathBuf,
        path: String,
    },
    /// A nonconforming document file, given its corrected source.
    DocumentRepair {
        root: PathBuf,
        path: PathBuf,
        observation: String,
        source: String,
    },
    /// A nonconforming ticket file, given its corrected source.
    TicketRepair {
        root: PathBuf,
        path: PathBuf,
        observation: String,
        source: String,
    },
}

impl Intent {
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
            | Self::OperationAbandon { .. }
            | Self::DocumentRepair { .. }
            | Self::TicketRepair { .. } => true,
            Self::RemoteAdd { .. }
            | Self::RemoteSelect { .. }
            | Self::IndexRefresh { .. }
            | Self::IndexRebuild { .. }
            | Self::KeyGenerate { .. }
            | Self::KeyImport { .. }
            | Self::KeySelect { .. }
            | Self::KeyClear
            | Self::KeyRemove { .. }
            | Self::OperationResume { .. }
            | Self::DocumentSave { .. }
            | Self::TicketSave { .. }
            | Self::ContextPrepare { .. }
            | Self::ContextRelease { .. }
            | Self::DocumentCreate { .. }
            | Self::TicketCreate { .. }
            | Self::DocumentMove { .. }
            | Self::FolderCreate { .. } => false,
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
            Self::DocumentSave { .. } => "Save document",
            Self::TicketSave { .. } => "Save ticket",
            Self::ContextPrepare { .. } => "Prepare to edit",
            Self::ContextRelease { .. } => "Release work context",
            Self::DocumentCreate { .. } => "Create document",
            Self::TicketCreate { .. } => "Create ticket",
            Self::DocumentMove { .. } => "Move document",
            Self::FolderCreate { .. } => "Create folder",
            Self::DocumentRepair { .. } => "Repair document",
            Self::TicketRepair { .. } => "Repair ticket",
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
            | Self::OperationAbandon { root, .. }
            | Self::DocumentSave { root, .. }
            | Self::TicketSave { root, .. }
            | Self::ContextPrepare { root, .. }
            | Self::ContextRelease { root, .. }
            | Self::DocumentCreate { root, .. }
            | Self::TicketCreate { root, .. }
            | Self::DocumentMove { root, .. }
            | Self::FolderCreate { root, .. }
            | Self::DocumentRepair { root, .. }
            | Self::TicketRepair { root, .. } => Some(root),
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
            | Self::RepoEnable { .. }
            | Self::DocumentSave { .. }
            | Self::TicketSave { .. }
            | Self::ContextPrepare { .. }
            | Self::ContextRelease { .. }
            | Self::DocumentCreate { .. }
            | Self::TicketCreate { .. }
            | Self::DocumentMove { .. }
            | Self::FolderCreate { .. }
            | Self::DocumentRepair { .. }
            | Self::TicketRepair { .. } => true,
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

    /// R-78 (owner, 2026-10-26): whether sending this request again, once
    /// `later` was sent after it, would undo `later`. Neither context
    /// command records a checkpoint, so a request sent again runs as new: a
    /// release after a later prepare of its item would release that newer
    /// context, and a prepare after a later release would make the context
    /// again. Such a request is never sent again.
    pub(crate) fn undone_by(&self, later: &Intent) -> bool {
        match (self, later) {
            (
                Self::ContextRelease { root, kind, id },
                Self::ContextPrepare {
                    root: later_root,
                    kind: later_kind,
                    id: later_id,
                    ..
                },
            )
            | (
                Self::ContextPrepare { root, kind, id, .. },
                Self::ContextRelease {
                    root: later_root,
                    kind: later_kind,
                    id: later_id,
                },
            ) => root == later_root && kind == later_kind && id == later_id,
            _ => false,
        }
    }

    /// Whether `later`, sent after this request, keeps it from being sent
    /// again (R-78, `undone_by`), `refused` saying whether `later` was
    /// answered with a refusal (`refused_release`). The owner narrowed
    /// R-78 on 2026-10-28 (Task 17 review, M3): a release the library
    /// refused, touching nothing, does not count against the earlier
    /// prepare of its item, which may be sent again under its request ID
    /// (R-77). A release still queued or running, one whose answer is
    /// unknown, and one that answered `ok` still do. A release is never
    /// sent again after a later prepare, whatever that prepare answered.
    pub(crate) fn kept_from_again_by(&self, later: &Intent, refused: bool) -> bool {
        self.undone_by(later) && !(refused && matches!(later, Self::ContextRelease { .. }))
    }

    /// Whether a finished request was refused, touching nothing (see
    /// `Refusals` for a request's attempts together): the
    /// library answered with a code that refuses (class `Blocked` or
    /// `Input`), with no durable effect (the outcome is not `partial`) and
    /// no record left for the same request to go on from (no
    /// `request.retry`). An interrupted request (`internal_error`), a busy
    /// one and one cancelled are not refusals: what they did is not known
    /// or not final.
    pub(crate) fn refused_answer(
        outcome: Outcome,
        code: ResultCode,
        recovery: &[RecoveryAction],
    ) -> bool {
        matches!(
            code.failure_class(),
            Some(FailureClass::Blocked | FailureClass::Input)
        ) && matches!(outcome, Outcome::Blocked | Outcome::Error)
            && !recovery
                .iter()
                .any(|action| action.action == RecoveryActionKind::RequestRetry)
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
            Self::DocumentSave { .. } => JobKind::DocumentSave,
            Self::TicketSave { .. } => JobKind::TicketSave,
            Self::ContextPrepare { .. } => JobKind::ContextPrepare,
            Self::ContextRelease { .. } => JobKind::ContextRelease,
            Self::DocumentCreate { .. } => JobKind::DocumentCreate,
            Self::TicketCreate { .. } => JobKind::TicketCreate,
            Self::DocumentMove { .. } => JobKind::DocumentMove,
            Self::FolderCreate { .. } => JobKind::FolderCreate,
            Self::DocumentRepair { .. } => JobKind::DocumentRepair,
            Self::TicketRepair { .. } => JobKind::TicketRepair,
        }
    }
}

/// What a request's attempts were answered, for R-78's exemption (owner,
/// 2026-10-28): a release frees the earlier prepare of its item only when
/// every one of its attempts was refused, touching nothing
/// (`Intent::refused_answer`). An attempt answered otherwise (unknown,
/// partial, `ok`) makes it block for good (Task 17 re-review, N1), and so
/// does one still queued or running, until it is refused. The worker and
/// the request model each keep one per request.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub(crate) struct Refusals {
    /// The last attempt finished, and was refused.
    last: bool,
    /// An attempt finished and was not refused.
    unrefused: bool,
}

impl Refusals {
    /// An attempt was queued: it is not answered yet.
    pub(crate) fn queued(&mut self) {
        self.last = false;
    }

    /// The attempt finished, `refused` or not. One not refused is never
    /// forgotten.
    pub(crate) fn finished(&mut self, refused: bool) {
        self.last = refused;
        self.unrefused |= !refused;
    }

    /// Whether every attempt so far was refused, touching nothing.
    pub(crate) fn all_refused(&self) -> bool {
        self.last && !self.unrefused
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
