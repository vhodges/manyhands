//! The data shapes of the mutation boundary's JSON v1 results.
//!
//! As with the read DTOs, these are the only types here that serialize.
//! Every field is always present, and an absent value is `null`.

use serde::{Serialize, Serializer};

use super::command::Command;
use crate::{
    repository::{IdentityDto, ItemDtoKind, OperationAction, OperationFamily},
    results::{Effects, Outcome, PublicationEffect, ResultCode, Scope, contract_enum},
};

contract_enum!(
    /// Where a request stands. `accepted` is a request that was begun and
    /// whose end is not recorded; `finished` is one whose result is stored
    /// and is returned to every retry.
    RequestState {
        Accepted => "accepted",
        Finished => "finished",
    }
);

/// A domain operation a request began, and the journal that holds it:
/// `operation show` reads it there.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RequestOperationDto {
    pub family: OperationFamily,
    pub operation_id: String,
}

/// What a finished request returned, as its envelope carried it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RequestResultDto {
    pub outcome: Outcome,
    pub code: ResultCode,
    /// The code's fixed message.
    pub message: String,
    pub effects: Effects,
    /// The envelope's `data`, whose shape is the command's own. Null when
    /// the envelope carried none.
    pub data: Option<serde_json::Value>,
}

/// What the records hold for one request ID. Nothing is observed again:
/// this is not what Git, the file system or a journal holds now.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RequestDto {
    pub request_id: String,
    pub state: RequestState,
    /// The command the request was made with, as its envelope names it.
    pub command: String,
    pub accepted_at: String,
    /// Null until the request is finished.
    pub finished_at: Option<String>,
    /// The request's operations, in the order it runs them.
    pub operations: Vec<RequestOperationDto>,
    /// Set exactly when `state` is `finished`.
    pub result: Option<RequestResultDto>,
}

/// What `request cancel` reports of the request it asked to stop.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CancelDto {
    pub request_id: String,
    /// Where the request stood when it was asked.
    pub state: RequestState,
    /// Whether the request was asked to stop: it is accepted, and its
    /// command has a safe point to stop at. False for a request that is
    /// finished and for one of a command that cannot be stopped once it
    /// is accepted, neither of which is changed in any way.
    pub requested: bool,
    /// Whether the request has stopped for a cancellation: its result
    /// is `cancelled`, or its operation has ended cancelled and the
    /// request's next call returns that. A request that was asked and
    /// has not reached a safe point since is `requested` and not
    /// `acknowledged`.
    pub acknowledged: bool,
}

contract_enum!(
    /// One thing a confirmed command will do, as its preview lists it.
    ///
    /// `head_moved` is HEAD made to name the primary branch, in a
    /// repository whose HEAD names another branch that has no commit.
    /// `worktree_exclusion_written` is the repository's own exclude file,
    /// `.git/info/exclude`, made to ignore the directory Manyhands keeps
    /// its worktrees in.
    ///
    /// `remote_removed` is the remote taken out of the repository's own
    /// Git configuration. With it Git removes the remote's tracking
    /// references, `remote_tracking_refs_removed`, and the upstream of
    /// every local branch that follows it, `branch_upstreams_removed`:
    /// each is listed when the repository has any.
    ///
    /// `registration_removed` is the repository taken out of the
    /// repository list. With it go what Manyhands cached of it:
    /// `index_dropped`, the index of its contexts, items, comments and
    /// problems and of its configuration; `operation_records_dropped`,
    /// the journal of its local operations; and `remote_state_dropped`,
    /// its polling policy, what polling observed of its remote and the
    /// records of its synchronizations. None of the four touches a file
    /// under the repository's root or anything Git holds.
    ///
    /// `private_key_file_deleted` and `public_key_file_deleted` are the
    /// two files of a key Manyhands generated, deleted from the user's
    /// key store, each named in the preview's details.
    /// `key_unregistered` is the key taken out of the key list, with the
    /// record that Manyhands made its files.
    ///
    /// `operation_closed` is a pending local operation closed as
    /// abandoned, so that it no longer refuses other operations on its
    /// repository. With it goes the record of the unfinished request
    /// that began the operation, `request_released`, listed when there
    /// is one: that request's ID is then free. Neither touches a file
    /// under the repository's root or anything Git holds.
    ///
    /// `host_pinned` is a host that has no pin given one: the key the
    /// request names, once the host is seen to present it.
    /// `host_pin_replaced` is a host's pin changed to the key the
    /// request names, likewise. Neither publishes or fetches anything,
    /// and neither moves a reference.
    ///
    /// `context_created`, `id_adopted`, `source_written` and
    /// `checkpoint_committed` are a repair's. `context_created` is the
    /// editing context of the repaired item made from the primary branch:
    /// its branch and its worktree, listed when the item has none.
    /// `id_adopted` is an ID assigned to a file that holds no valid one:
    /// the ID the corrected source carries, which the preview's details
    /// name. `source_written` is the corrected source written over the
    /// nonconforming file in that context, and `checkpoint_committed`
    /// the commit of it there. None of the four writes the primary
    /// worktree or moves the primary branch.
    ///
    /// `branch_republished` is an item's editing branch, which the remote
    /// deleted, pushed to it again at exactly the local commit the
    /// preview's details name. Nothing is integrated: no local branch
    /// moves, and the next synchronization of the item integrates
    /// primary.
    ///
    /// `context_synchronized` to `branch_deleted` are a promotion's and a
    /// closure's, in order, each listed when the preview plans it:
    /// `context_synchronized` is the editing branch synchronized with the
    /// remote first; `closure_recorded` the closure fields written and
    /// committed there; `primary_integrated` the branch merged into primary;
    /// `primary_published` primary pushed; `remote_branch_deleted`,
    /// `worktree_removed` and `branch_deleted` the context retired.
    PreviewEffect {
        DirectoryCreated => "directory_created",
        RepositoryInitialized => "repository_initialized",
        IdentityWritten => "identity_written",
        HeadMoved => "head_moved",
        WorktreeExclusionWritten => "worktree_exclusion_written",
        ConfigurationWritten => "configuration_written",
        InitializationCommitted => "initialization_committed",
        RepositoryRegistered => "repository_registered",
        RemoteRemoved => "remote_removed",
        RemoteTrackingRefsRemoved => "remote_tracking_refs_removed",
        BranchUpstreamsRemoved => "branch_upstreams_removed",
        RegistrationRemoved => "registration_removed",
        IndexDropped => "index_dropped",
        OperationRecordsDropped => "operation_records_dropped",
        RemoteStateDropped => "remote_state_dropped",
        PrivateKeyFileDeleted => "private_key_file_deleted",
        PublicKeyFileDeleted => "public_key_file_deleted",
        KeyUnregistered => "key_unregistered",
        OperationClosed => "operation_closed",
        RequestReleased => "request_released",
        HostPinned => "host_pinned",
        HostPinReplaced => "host_pin_replaced",
        ContextCreated => "context_created",
        IdAdopted => "id_adopted",
        SourceWritten => "source_written",
        CheckpointCommitted => "checkpoint_committed",
        BranchRepublished => "branch_republished",
        ContextSynchronized => "context_synchronized",
        ClosureRecorded => "closure_recorded",
        PrimaryIntegrated => "primary_integrated",
        PrimaryPublished => "primary_published",
        RemoteBranchDeleted => "remote_branch_deleted",
        WorktreeRemoved => "worktree_removed",
        BranchDeleted => "branch_deleted",
    }
);

contract_enum!(
    /// Where the identity a preview shows comes from: the request itself,
    /// or the level of Git configuration that holds it.
    PreviewIdentitySource {
        Supplied => "supplied",
        Repository => "repository",
        Global => "global",
        Xdg => "xdg",
        System => "system",
        ProgramData => "program_data",
        Application => "application",
    }
);

/// One entry of a preview's `effects`. The entries are in the order the
/// command makes them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PreviewEffectDto {
    pub effect: PreviewEffect,
}

/// The identity that will author the initialization commit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PreviewIdentityDto {
    pub name: String,
    pub email: String,
    pub source: PreviewIdentitySource,
    /// Whether the command writes this identity to the repository's own
    /// Git configuration. It never writes any other configuration.
    pub written_locally: bool,
}

/// What a preview of `repo create` or `repo enable` shows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RepositoryPreviewDto {
    /// The repository's root, as the request's records name it.
    pub root: String,
    pub primary_branch: String,
    pub identity: PreviewIdentityDto,
    /// The short code prefix the configuration will hold. Null when the
    /// request supplies none: the configuration then holds no prefix.
    pub slug_prefix: Option<String>,
}

/// A preview's `details`: the command's own fields.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum PreviewDetailsDto {
    Repository(RepositoryPreviewDto),
    RemoteRemoval(RemoteRemovalPreviewDto),
    RepositoryRemoval(RepositoryRemovalPreviewDto),
    KeyDeletion(KeyDeletionPreviewDto),
    OperationAbandon(OperationAbandonPreviewDto),
    Identity(IdentityPreviewDto),
    Host(HostPreviewDto),
    Repair(RepairPreviewDto),
    Republish(ItemRepublishPreviewDto),
    Lifecycle(Box<LifecyclePreviewDto>),
}

contract_enum!(
    /// How a preview plans to reach primary.
    LifecyclePlannedIntegration {
        MergeCommit => "merge_commit",
        AlreadyContained => "already_contained",
    }
);

contract_enum!(
    /// What a preview plans for primary's publication: a push, or none for
    /// a repository with no publication remote.
    LifecyclePlannedPublication {
        Publish => "publish",
        PendingNoRemote => "pending_no_remote",
    }
);

contract_enum!(
    /// How a path differs between primary and the item's context.
    LifecyclePathChangeKind {
        Added => "added",
        Modified => "modified",
        Deleted => "deleted",
    }
);

contract_enum!(
    /// How a closure reaches the ticket: this operation writes both closure
    /// fields, or the context already holds them.
    LifecycleRecording {
        Record => "record",
        Existing => "existing",
    }
);

/// One path of a preview's whole-branch change set. It names the path and
/// holds nothing the file holds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LifecyclePathChangeDto {
    /// Relative to the repository, with forward slashes.
    pub path: String,
    pub change: LifecyclePathChangeKind,
}

/// What a preview plans to retire of the item's context.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct LifecyclePlannedCleanupDto {
    pub remote_branch: bool,
    pub worktree: bool,
    pub local_branch: bool,
}

/// What a closure records. `closed_by` is personal data, and appears only in
/// a preview, canonical content, the index and Git (Cycle 10): a request's
/// record never holds it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LifecyclePlannedClosureDto {
    pub recording: LifecycleRecording,
    pub closed_by: String,
}

/// What a preview of `document promote` or `ticket close` shows. It holds
/// names, commits and paths, and no title, body or content.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LifecyclePreviewDto {
    /// The repository's root, as the request's records name it.
    pub root: String,
    pub kind: ItemDtoKind,
    pub item_id: String,
    /// The ticket's short code, when the index holds one.
    pub slug: Option<String>,
    /// The item's editing branch, as its short name.
    pub branch: String,
    pub primary_branch: String,
    /// The publication remote, by its name. Null for a repository with none.
    pub remote: Option<String>,
    pub context_oid: String,
    pub primary_oid: String,
    pub changes: Vec<LifecyclePathChangeDto>,
    pub integration: LifecyclePlannedIntegration,
    pub publication: LifecyclePlannedPublication,
    pub cleanup: LifecyclePlannedCleanupDto,
    /// Null for a promotion, which records no closure.
    pub closure: Option<LifecyclePlannedClosureDto>,
}

contract_enum!(
    /// What stands in the way of a promotion or a closure: the domain's
    /// blockers of both, by one vocabulary.
    LifecycleBlocker {
        NotADocument => "not_a_document",
        NotATicket => "not_a_ticket",
        ContextMissing => "context_missing",
        ContextNonconforming => "context_nonconforming",
        ContextDirty => "context_dirty",
        ContextConflicted => "context_conflicted",
        ConflictPending => "conflict_pending",
        LocalOperationPending => "local_operation_pending",
        CommentPublicationPending => "comment_publication_pending",
        PrimaryMissing => "primary_missing",
        PrimaryNotCheckedOut => "primary_not_checked_out",
        PrimaryDirty => "primary_dirty",
        PrimaryConflicted => "primary_conflicted",
        PrimaryAheadOfRemote => "primary_ahead_of_remote",
        ContextBehindRemote => "context_behind_remote",
        IntegrationConflict => "integration_conflict",
        RemoteContextDeleted => "remote_context_deleted",
        HistoryUnknown => "history_unknown",
        AlreadyClosed => "already_closed",
        PrimaryReopened => "primary_reopened",
        DocumentInvalidAfterIntegration => "document_invalid_after_integration",
        TicketInvalidAfterClosure => "ticket_invalid_after_closure",
        IdentityRequired => "identity_required",
    }
);

/// What a `prepare` of `document promote` or `ticket close` reports when it
/// is refused for its blockers: every one of them, in the domain's order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LifecycleBlockedDto {
    pub blockers: Vec<LifecycleBlocker>,
}

/// What a preview of `item republish` shows: the item's editing branch,
/// which the remote deleted, the remote it is pushed to, the commit last
/// published there and the local commit that will be pushed, exactly
/// (decision 11). It holds names and commits and no content.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ItemRepublishPreviewDto {
    /// The repository's root, as the request's records name it.
    pub root: String,
    pub kind: ItemDtoKind,
    pub item_id: String,
    /// The branch, as its short name.
    pub branch: String,
    /// The publication remote, by its name.
    pub remote: String,
    /// The commit last published on the branch, as the record of its
    /// deletion holds it. Null when that record holds none.
    pub last_published_oid: Option<String>,
    /// The local commit the branch will be published at: its tip.
    pub local_oid: String,
}

/// What a preview of `document repair` or `ticket repair` shows: the
/// file that will be corrected, and the identity the corrected source
/// gives it. It holds a path and an ID and nothing the file or the
/// source holds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RepairPreviewDto {
    /// The repository's root, as the request's records name it.
    pub root: String,
    /// The file, relative to the repository, with forward slashes.
    pub path: String,
    /// The ID the corrected source carries: the item's identity once
    /// the repair is made, and what names its editing context.
    pub id: String,
    /// Whether the ID is being assigned: the file holds no valid ID,
    /// and the source's becomes the item's. False for a file that holds
    /// this ID already, which the repair keeps.
    pub adoption: bool,
    /// Whether the command writes the primary worktree or moves the
    /// primary branch. It never does: this is false. The file stays on
    /// primary as it is until the item's context is merged.
    pub primary_changed: bool,
}

/// What a preview of `host approve` or `host replace` shows: the host,
/// what the publication remote uses it for, the key that will be pinned
/// once the host is seen to present it, and the pin there is now. A
/// fingerprint is `SHA256:` and the digest of the host's public key, as
/// a pin's is.
///
/// One vocabulary names a host's keys in every shape the boundary
/// publishes. `presented_algorithm` and `presented_fingerprint` are the
/// key the host presented when it was observed, and nothing else.
/// `pinned_algorithm` and `pinned_fingerprint` are the pin stored for
/// the host now. `algorithm` and `fingerprint` are the key a
/// `host approve` or `host replace` request names: its input, and the
/// echo of that input in its preview and its result. So what a
/// synchronization or an observation gives as `presented_*` is what an
/// approval is given as `algorithm` and `fingerprint`, and its
/// `pinned_fingerprint` is what a replacement is given as the old
/// fingerprint.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HostPreviewDto {
    /// The repository's root, as the request's records name it.
    pub root: String,
    /// The host, in lower case.
    pub host: String,
    pub port: u16,
    /// Whether the publication remote is fetched from this host.
    pub fetch: bool,
    /// Whether the publication remote is pushed to this host.
    pub push: bool,
    /// The algorithm and the fingerprint of the key that will be pinned.
    pub algorithm: String,
    pub fingerprint: String,
    /// The pin the host has now, which the command replaces. Null for a
    /// host that has none.
    pub pinned_algorithm: Option<String>,
    pub pinned_fingerprint: Option<String>,
    /// The application-wide marker left when the pin registry was lost.
    /// The command leaves it as it is.
    pub reapproval_required: bool,
}

/// What `host observe` reports of a host a repository's publication
/// remote names, once it has connected to it: the key the host
/// presented, and the pin the host has. Nothing is pinned by it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HostObservationDto {
    /// The host, in lower case.
    pub host: String,
    pub port: u16,
    /// Whether the publication remote is fetched from this host.
    pub fetch: bool,
    /// Whether the publication remote is pushed to this host.
    pub push: bool,
    /// The algorithm and the fingerprint of the key the host presented:
    /// what `host approve` and `host replace` are given as `algorithm`
    /// and `fingerprint`.
    pub presented_algorithm: String,
    pub presented_fingerprint: String,
    /// The pin the host has. Null for a host that has none. A host that
    /// presented its pin is trusted; one that presented another key has
    /// changed, and `pinned_fingerprint` is what `host replace` is given
    /// as the old fingerprint.
    pub pinned_algorithm: Option<String>,
    pub pinned_fingerprint: Option<String>,
    /// The application-wide marker left when the pin registry was lost.
    pub reapproval_required: bool,
}

/// What a repository's own Git configuration holds of an identity. A
/// value held at any other level of configuration is not here.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LocalIdentityDto {
    pub name: Option<String>,
    pub email: Option<String>,
    /// `manyhands.initials`, as it is written: it need not be initials a
    /// short code can be composed with.
    pub initials: Option<String>,
}

/// What a preview of `repo identity-set` shows: what the repository's
/// own Git configuration will hold, what it holds now, and the identity
/// a commit would carry now. No other configuration is written.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct IdentityPreviewDto {
    /// The repository's root, as the request's records name it.
    pub root: String,
    /// The name and the email that will be written.
    pub name: String,
    pub email: String,
    /// The initials the repository's own configuration will hold: the
    /// ones the request sets, in lower case, or the ones it holds now
    /// when the request says nothing of them. Null when it will hold
    /// none: the request clears them, or says nothing and none are set.
    pub initials: Option<String>,
    /// What the repository's own configuration holds now.
    pub current: LocalIdentityDto,
    /// The identity a commit would carry now, with the level of
    /// configuration it comes from, and the initials a short code would
    /// be composed with. The command changes it only through the
    /// repository's own configuration.
    pub effective: IdentityDto,
}

/// What a preview of `operation abandon` shows: the pending operation
/// that will be closed, and what it has left on disk, which the command
/// leaves as it is. It holds names and paths and no content.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OperationAbandonPreviewDto {
    /// The repository's root, as the operation's journal names it.
    pub root: String,
    /// The operation that will be closed.
    pub operation_id: String,
    /// What the operation does, as `operation show` names it. Null for
    /// an action this version does not know.
    pub action: Option<OperationAction>,
    /// The operation's state, as `operation show` gives it.
    pub state: String,
    /// The last step the operation recorded as done. Null when it
    /// recorded none.
    pub step: Option<String>,
    /// The item the operation writes. Null for an operation that writes
    /// no item.
    pub item_id: Option<String>,
    /// The worktree the operation writes in, relative to `root` with
    /// forward slashes: an item's own, or `.` for the primary worktree.
    /// Null for an operation that writes no file of a worktree, and for
    /// an item whose worktree is not there.
    pub worktree: Option<String>,
    /// The paths of the operation's target that differ from the head of
    /// their branch, relative to `root` with forward slashes. The
    /// command leaves every one of them as it is.
    pub paths: Vec<String>,
    /// Whether `paths` is all of them. False when there are more than
    /// are listed, or the worktree could not be read.
    pub paths_complete: bool,
    /// Whether the operation recorded that it passed the point where its
    /// work is committed. The command leaves that commit where it is.
    /// For a comment whose step is `comment_checkpoint_intent` the
    /// commit may not exist; `paths` holds the comment's file when it
    /// does not.
    pub committed: bool,
    /// The unfinished request that began the operation, whose record
    /// the command deletes. Null when no such request is recorded.
    pub request_id: Option<String>,
    /// Whether the command changes or removes any file under the root.
    /// It never does: this is false.
    pub files_changed: bool,
    /// Whether the command changes anything Git holds: a commit, a
    /// branch, a worktree, the index or the configuration. It never
    /// does: this is false.
    pub git_state_changed: bool,
}

/// What a preview of `key delete` shows: the key, and the files that
/// will be deleted. It holds what the key list gives of the key, and no
/// key material.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct KeyDeletionPreviewDto {
    pub id: String,
    pub label: String,
    /// The fingerprint the key's registration stores.
    pub fingerprint: Option<String>,
    /// The private key file that will be deleted.
    pub private_key_path: String,
    /// The public key file that will be deleted.
    pub public_key_path: Option<String>,
}

/// What a preview of `repo remove` shows: the registration that will be
/// removed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RepositoryRemovalPreviewDto {
    /// The registered root, as the repository list gives it.
    pub root: String,
    /// When the repository was registered, as the repository list gives
    /// it. Null when the stored time cannot be written as one.
    pub enabled_at: Option<String>,
    /// Whether the command changes or removes any file under the root,
    /// the directory Manyhands keeps its worktrees in among them. It
    /// never does: this is false.
    pub files_changed: bool,
    /// Whether the command changes anything Git holds: a commit, a
    /// branch, a worktree, the index or the configuration. It never
    /// does: this is false.
    pub git_state_changed: bool,
}

/// What a preview of `remote remove` shows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RemoteRemovalPreviewDto {
    /// The repository's root, as the request's records name it.
    pub root: String,
    /// The remote that will be removed.
    pub name: String,
    /// Where the remote fetches from and pushes to, redacted as the
    /// remote list redacts them: neither is a location to connect to.
    /// Null for a remote whose location that list does not give.
    pub fetch_location: Option<String>,
    pub push_location: Option<String>,
    /// Whether the remote is the one the repository publishes to. A
    /// preview is given only of a remote that is not, so this is false:
    /// the publication remote is cleared before it can be removed.
    pub publication_affected: bool,
    /// Whether polling observes this remote: the repository's polling
    /// state names it, and has nothing to observe once it is removed.
    pub polling_affected: bool,
}

/// What `prepare` returns for a command that needs confirmation: what
/// the command will do, and the confirmation an `execute` of the same
/// command, repository, target and input presents to do it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PreviewDto {
    pub confirmation_id: String,
    /// When the confirmation stops being accepted by a new request.
    pub expires_at: String,
    /// The command the confirmation is for, as an envelope names it.
    pub command: String,
    pub scope: Scope,
    pub effects: Vec<PreviewEffectDto>,
    pub details: PreviewDetailsDto,
}

/// What a `prepare` of `host approve` reports when it is refused as
/// `host_replacement_required`: the host has a pin, and a pin is changed
/// by `host replace` and by nothing else. With the `host.replace` action
/// the result offers, it is what that request is formed from:
/// `pinned_fingerprint` is its old fingerprint, and `algorithm` and
/// `fingerprint` are the key the refused request named.
///
/// A preview connects to nothing, so no key the host presents is known
/// here, and none is reported.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HostReplacementDto {
    /// The host, in lower case.
    pub host: String,
    pub port: u16,
    /// The algorithm and the fingerprint of the key the request names.
    pub algorithm: String,
    pub fingerprint: String,
    /// The pin stored for the host now.
    pub pinned_algorithm: String,
    pub pinned_fingerprint: String,
}

impl HostReplacementDto {
    /// The host as a recovery action names it: `host:port`, with an
    /// address that holds a colon in brackets.
    pub(crate) fn authority(&self) -> String {
        authority_text(&self.host, self.port)
    }
}

/// A host as a recovery action and a record's target name it:
/// `host:port`, with an address that holds a colon in brackets.
pub(crate) fn authority_text(host: &str, port: u16) -> String {
    if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

/// The `data` of a `prepare` envelope: the preview, or, for the two
/// refusals that report any, what the refusal is about. A `prepare`
/// that gives neither carries no data: every other rejection, and a
/// no-op for a command whose end state already holds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum PrepareDataDto {
    Preview(Box<PreviewDto>),
    /// Of `host approve`, refused as `host_replacement_required`.
    HostReplacement(HostReplacementDto),
    /// Of `document promote` and `ticket close`, refused for their blockers.
    LifecycleBlocked(LifecycleBlockedDto),
}

impl PrepareDataDto {
    /// The preview, when the data is one.
    pub fn preview(&self) -> Option<&PreviewDto> {
        match self {
            Self::Preview(preview) => Some(preview),
            Self::HostReplacement(_) | Self::LifecycleBlocked(_) => None,
        }
    }

    /// The preview, when the data is one.
    pub fn into_preview(self) -> Option<PreviewDto> {
        match self {
            Self::Preview(preview) => Some(*preview),
            Self::HostReplacement(_) | Self::LifecycleBlocked(_) => None,
        }
    }
}

/// The `data` of a mutation's envelope: the shape of the command's family.
/// It may accompany any outcome and never holds a body or a source.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum MutationDataDto {
    LocalContextSync(crate::repository::LocalContextSyncMutationDto),
    Document(DocumentMutationDto),
    Ticket(TicketMutationDto),
    Remote(RemoteMutationDto),
    Key(KeyMutationDto),
    Operation(OperationMutationDto),
    Synchronization(SynchronizationMutationDto),
    SynchronizationConflict(SynchronizationConflictMutationDto),
    Comment(CommentMutationDto),
    ConflictResolution(ConflictResolutionMutationDto),
    Identity(IdentityMutationDto),
    Host(HostMutationDto),
    Repair(RepairMutationDto),
    Context(ContextMutationDto),
    Lifecycle(Box<LifecycleMutationDto>),
}

contract_enum!(
    /// The tag of `LifecycleMutationDto`: one value, so that the data of a
    /// promotion or a closure is told apart by it alone.
    LifecycleMutationType {
        Lifecycle => "lifecycle",
    }
);

contract_enum!(
    /// How an operation reached primary: its merge commit, or a context
    /// primary already contained.
    LifecycleIntegrationKind {
        Merged => "merged",
        AlreadyContained => "already_contained",
    }
);

contract_enum!(
    /// How far primary's publication got.
    LifecyclePublicationState {
        Published => "published",
        AlreadyCurrent => "already_current",
        Pending => "pending",
    }
);

contract_enum!(
    /// Why primary's publication is pending: the repository has no
    /// publication remote; its synchronization stopped for this category;
    /// or it was cancelled.
    LifecyclePublicationPendingReason {
        NoPublicationRemote => "no_publication_remote",
        Conflict => "conflict",
        Rejected => "rejected",
        Transport => "transport",
        Identity => "identity",
        Busy => "busy",
        Yielding => "yielding",
        RecoveryRequired => "recovery_required",
        Cancelled => "cancelled",
    }
);

contract_enum!(
    /// How far retiring the context got.
    LifecycleCleanupState {
        Complete => "complete",
        Pending => "pending",
    }
);

contract_enum!(
    /// Why retiring the context is pending. It holds a fixed category and
    /// no text a server sent.
    LifecycleCleanupPendingReason {
        AwaitingPublication => "awaiting_publication",
        ContextChanged => "context_changed",
        IgnoredFiles => "ignored_files",
        RemoteContextChanged => "remote_context_changed",
        RemoteDeletionTransport => "remote_deletion_transport",
        RemoteDeletionRejected => "remote_deletion_rejected",
        RemoteDeletionConfiguration => "remote_deletion_configuration",
        RemoteDeletionKey => "remote_deletion_key",
        RemoteDeletionUnlock => "remote_deletion_unlock",
        RemoteDeletionHost => "remote_deletion_host",
        RecoveryRequired => "recovery_required",
        Cancelled => "cancelled",
    }
);

/// What `document promote` and `ticket close` report. An operation that
/// reached primary reports how far it got; one refused for its blockers
/// names them, and the rest is null. It holds IDs, commits and fixed
/// categories: no `closed_by`, title or body.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LifecycleMutationDto {
    pub r#type: LifecycleMutationType,
    pub kind: ItemDtoKind,
    pub item_id: String,
    /// How the closure reached the ticket. Null for a promotion, and for a
    /// closure that did not reach primary.
    pub recording: Option<LifecycleRecording>,
    /// The commit that recorded the closure, when this operation wrote it.
    pub checkpoint_oid: Option<String>,
    pub integration: Option<LifecycleIntegrationKind>,
    pub integration_oid: Option<String>,
    pub publication: Option<LifecyclePublicationState>,
    /// The commit primary was published at, or found current at.
    pub publication_oid: Option<String>,
    /// Set exactly when the publication is pending.
    pub publication_pending: Option<LifecyclePublicationPendingReason>,
    pub cleanup: Option<LifecycleCleanupState>,
    /// Set exactly when the cleanup is pending.
    pub cleanup_pending: Option<LifecycleCleanupPendingReason>,
    /// What refused the operation, in the domain's order. Empty otherwise.
    pub blockers: Vec<LifecycleBlocker>,
}

impl LifecycleMutationDto {
    /// The data as a finished record stored it.
    pub(crate) fn from_stored(stored: &serde_json::Value) -> Option<Self> {
        let text = |key: &str| -> Option<Option<String>> {
            match stored.get(key)? {
                serde_json::Value::Null => Some(None),
                value => Some(Some(value.as_str()?.to_owned())),
            }
        };
        // A closed vocabulary: what is not of it is not of this shape.
        fn known<T: Copy>(all: &[T], name: fn(T) -> &'static str, text: &str) -> Option<T> {
            all.iter().copied().find(|known| name(*known) == text)
        }
        // A nullable field of a closed vocabulary. The outer `None` is data
        // that is not of this shape.
        fn optional_known<T: Copy>(
            value: Option<String>,
            all: &[T],
            name: fn(T) -> &'static str,
        ) -> Option<Option<T>> {
            match value {
                Some(value) => Some(Some(known(all, name, &value)?)),
                None => Some(None),
            }
        }
        Some(Self {
            r#type: known(
                &LifecycleMutationType::ALL,
                LifecycleMutationType::as_str,
                &text("type")??,
            )?,
            kind: known(&ItemDtoKind::ALL, ItemDtoKind::as_str, &text("kind")??)?,
            item_id: text("item_id")??,
            recording: optional_known(
                text("recording")?,
                &LifecycleRecording::ALL,
                LifecycleRecording::as_str,
            )?,
            checkpoint_oid: text("checkpoint_oid")?,
            integration: optional_known(
                text("integration")?,
                &LifecycleIntegrationKind::ALL,
                LifecycleIntegrationKind::as_str,
            )?,
            integration_oid: text("integration_oid")?,
            publication: optional_known(
                text("publication")?,
                &LifecyclePublicationState::ALL,
                LifecyclePublicationState::as_str,
            )?,
            publication_oid: text("publication_oid")?,
            publication_pending: optional_known(
                text("publication_pending")?,
                &LifecyclePublicationPendingReason::ALL,
                LifecyclePublicationPendingReason::as_str,
            )?,
            cleanup: optional_known(
                text("cleanup")?,
                &LifecycleCleanupState::ALL,
                LifecycleCleanupState::as_str,
            )?,
            cleanup_pending: optional_known(
                text("cleanup_pending")?,
                &LifecycleCleanupPendingReason::ALL,
                LifecycleCleanupPendingReason::as_str,
            )?,
            blockers: stored
                .get("blockers")?
                .as_array()?
                .iter()
                .map(|value| {
                    known(
                        &LifecycleBlocker::ALL,
                        LifecycleBlocker::as_str,
                        value.as_str()?,
                    )
                })
                .collect::<Option<_>>()?,
        })
    }
}

contract_enum!(
    /// Why a comment's publication is pending: the repository has no
    /// publication remote; the comment's own index hand-off, or the
    /// proof of its checkpoint, is to be done first; or its
    /// synchronization stopped, as the result's code says.
    CommentPendingReason {
        NoPublicationRemote => "no_publication_remote",
        RecoveryRequired => "recovery_required",
        Synchronization => "synchronization",
    }
);

contract_enum!(
    /// The tag of `CommentMutationDto`: one value, so that the data of a
    /// comment is told apart by it alone.
    CommentMutationType {
        Comment => "comment",
    }
);

/// What `comment add` reports of the comment it checkpointed: the
/// comment, as its front matter names it, the synchronization that
/// publishes it, and how far that got. It holds no body.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CommentMutationDto {
    pub r#type: CommentMutationType,
    /// The item the comment is filed under.
    pub item_id: String,
    pub comment_id: String,
    /// Null for a root comment.
    pub parent_id: Option<String>,
    /// The comment's `created_by`, as its checkpoint holds it: the author
    /// the request gave, or the text of the identity that checkpointed
    /// it. Null when it cannot be read.
    pub created_by: Option<String>,
    /// The synchronization that publishes the comment: the ID it is
    /// inspected, resolved and resumed by.
    pub synchronization_id: String,
    /// As the result's effects say, and never `not_requested`.
    pub publication: PublicationEffect,
    /// Set exactly when the publication is pending.
    pub pending_reason: Option<CommentPendingReason>,
    /// The next step of the conflict the synchronization stopped on, when
    /// it stopped on one.
    pub conflict: Option<ConflictNextDto>,
}

impl CommentMutationDto {
    /// The data as a finished record stored it.
    pub(crate) fn from_stored(stored: &serde_json::Value) -> Option<Self> {
        let text = |key: &str| -> Option<Option<String>> {
            match stored.get(key)? {
                serde_json::Value::Null => Some(None),
                value => Some(Some(value.as_str()?.to_owned())),
            }
        };
        let tag = stored.get("type")?.as_str()?;
        let publication = stored.get("publication")?.as_str()?;
        Some(Self {
            r#type: CommentMutationType::ALL
                .into_iter()
                .find(|known| known.as_str() == tag)?,
            item_id: text("item_id")??,
            comment_id: text("comment_id")??,
            parent_id: text("parent_id")?,
            created_by: text("created_by")?,
            synchronization_id: text("synchronization_id")??,
            publication: PublicationEffect::ALL
                .into_iter()
                .find(|known| known.as_str() == publication)?,
            pending_reason: match text("pending_reason")? {
                Some(reason) => Some(
                    CommentPendingReason::ALL
                        .into_iter()
                        .find(|known| known.as_str() == reason)?,
                ),
                None => None,
            },
            conflict: match stored.get("conflict")? {
                serde_json::Value::Null => None,
                conflict => Some(ConflictNextDto::from_stored(conflict)?),
            },
        })
    }
}

contract_enum!(
    /// The tag of `ConflictResolutionMutationDto`: one value, so that the
    /// data of a resolution is told apart by it alone.
    ConflictResolutionMutationType {
        ConflictResolution => "conflict_resolution",
    }
);

/// What `conflict resolve` reports: the synchronization whose conflict it
/// resolved, the resolution commit, how far the publication that continued
/// it got, and the next step of a new conflict that publication met. It
/// holds no source and names no path.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ConflictResolutionMutationDto {
    pub r#type: ConflictResolutionMutationType,
    /// The synchronization, whichever ID the request named (R-5).
    pub operation_id: String,
    /// The resolution commit. Null when the request made none.
    pub commit_oid: Option<String>,
    /// As the result's effects say.
    pub publication: PublicationEffect,
    /// The next step of the conflict the publication met, or of the
    /// conflict the request could not resolve in Manyhands. Null otherwise.
    pub conflict: Option<ConflictNextDto>,
}

impl ConflictResolutionMutationDto {
    /// The data as a finished record stored it.
    pub(crate) fn from_stored(stored: &serde_json::Value) -> Option<Self> {
        let tag = stored.get("type")?.as_str()?;
        let publication = stored.get("publication")?.as_str()?;
        Some(Self {
            r#type: ConflictResolutionMutationType::ALL
                .into_iter()
                .find(|known| known.as_str() == tag)?,
            operation_id: stored.get("operation_id")?.as_str()?.to_owned(),
            commit_oid: match stored.get("commit_oid")? {
                serde_json::Value::Null => None,
                commit => Some(commit.as_str()?.to_owned()),
            },
            publication: PublicationEffect::ALL
                .into_iter()
                .find(|known| known.as_str() == publication)?,
            conflict: match stored.get("conflict")? {
                serde_json::Value::Null => None,
                conflict => Some(ConflictNextDto::from_stored(conflict)?),
            },
        })
    }
}

/// What `document repair` and `ticket repair` report about the item the
/// corrected source makes. It never holds the source or anything the
/// nonconforming file held.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RepairMutationDto {
    /// The ID the corrected source carries.
    pub id: String,
    /// The file the request corrects, relative to the item's context,
    /// with forward slashes: the path it has on primary too.
    pub path: String,
    /// Whether the ID was assigned to a file that held no valid one.
    /// Null when the request stopped before the file was read.
    pub adopted: Option<bool>,
}

impl RepairMutationDto {
    /// The data as a finished record stored it.
    pub(crate) fn from_stored(stored: &serde_json::Value) -> Option<Self> {
        Some(Self {
            id: stored.get("id")?.as_str()?.to_owned(),
            path: stored.get("path")?.as_str()?.to_owned(),
            adopted: match stored.get("adopted")? {
                serde_json::Value::Null => None,
                adopted => Some(adopted.as_bool()?),
            },
        })
    }
}

/// What `context prepare` reports: the item and the work context (the
/// branch and the worktree) it is edited in. It never holds a body or a
/// source.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ContextMutationDto {
    /// The item the context is for.
    pub id: String,
    /// `ticket` or `document`.
    pub kind: String,
    /// The context's branch, as a branch is named: `manyhands/<kind>/<id>`.
    pub branch: String,
    /// The context's worktree, as an absolute path.
    pub worktree: String,
    /// Whether this request made the context, rather than finding it.
    pub created: bool,
}

impl ContextMutationDto {
    /// The data as a finished record stored it.
    pub(crate) fn from_stored(stored: &serde_json::Value) -> Option<Self> {
        let text = |key: &str| stored.get(key)?.as_str().map(str::to_owned);
        Some(Self {
            id: text("id")?,
            kind: text("kind")?,
            branch: text("branch")?,
            worktree: text("worktree")?,
            created: stored.get("created")?.as_bool()?,
        })
    }
}

/// What `host approve` and `host replace` report: the host and the key
/// the request names. A pin that is stored is shown by no effect, so
/// this and the outcome say what the request did: `success` for a pin
/// that was stored, `noop` for a host already pinned to the key. A
/// request stopped as `host_mismatch` reports the key the host
/// presented instead of it, and no pin has changed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HostMutationDto {
    /// The host, in lower case.
    pub host: String,
    pub port: u16,
    /// The algorithm and the fingerprint of the key the request names:
    /// `SHA256:` and the digest of the public key.
    pub algorithm: String,
    pub fingerprint: String,
    /// The key the host presented, when it is not the one the request
    /// names. Null otherwise.
    pub presented_algorithm: Option<String>,
    pub presented_fingerprint: Option<String>,
}

impl HostMutationDto {
    /// The data as a finished record stored it.
    pub(crate) fn from_stored(stored: &serde_json::Value) -> Option<Self> {
        let text = |key: &str| stored.get(key)?.as_str().map(str::to_owned);
        let optional = |key: &str| -> Option<Option<String>> {
            match stored.get(key)? {
                serde_json::Value::Null => Some(None),
                value => Some(Some(value.as_str()?.to_owned())),
            }
        };
        Some(Self {
            host: text("host")?,
            port: u16::try_from(stored.get("port")?.as_u64()?).ok()?,
            algorithm: text("algorithm")?,
            fingerprint: text("fingerprint")?,
            presented_algorithm: optional("presented_algorithm")?,
            presented_fingerprint: optional("presented_fingerprint")?,
        })
    }
}

/// What `repo identity-set` reports: the identity the repository's own
/// Git configuration holds once the request is done, whether the request
/// wrote it or found it there. Every effect of the command is
/// `not_requested`: this, with the outcome, is what says what it did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct IdentityMutationDto {
    pub name: Option<String>,
    pub email: Option<String>,
    /// `manyhands.initials`. Null when the configuration holds none.
    pub initials: Option<String>,
}

impl IdentityMutationDto {
    /// The data as a finished record stored it.
    pub(crate) fn from_stored(stored: &serde_json::Value) -> Option<Self> {
        let text = |key: &str| -> Option<Option<String>> {
            match stored.get(key)? {
                serde_json::Value::Null => Some(None),
                value => Some(Some(value.as_str()?.to_owned())),
            }
        };
        Some(Self {
            name: text("name")?,
            email: text("email")?,
            initials: text("initials")?,
        })
    }
}

/// What a synchronization that stopped for its host reports: the host
/// it reached and the key that host presented, which is what a host
/// approval is given. One that stopped on a conflict reports
/// `SynchronizationConflictMutationDto` instead; every other result of one
/// reports no data, and what it published is in its effects.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SynchronizationMutationDto {
    /// The host, in lower case.
    pub host: String,
    pub port: u16,
    /// The algorithm of the key the host presented.
    pub presented_algorithm: String,
    /// The fingerprint of the key the host presented: `SHA256:` and
    /// the digest of the public key.
    pub presented_fingerprint: String,
    /// The pin stored for the host, when the host presented another key
    /// than its pin: `pinned_fingerprint` is what `host replace` is
    /// given as the old fingerprint. Null for a host that has no pin.
    pub pinned_algorithm: Option<String>,
    pub pinned_fingerprint: Option<String>,
}

impl SynchronizationMutationDto {
    /// The host as a recovery action names it: `host:port`, with an
    /// address that holds a colon in brackets.
    pub(crate) fn authority(&self) -> String {
        authority_text(&self.host, self.port)
    }

    /// The data as a finished record stored it.
    pub(crate) fn from_stored(stored: &serde_json::Value) -> Option<Self> {
        let text = |key: &str| stored.get(key)?.as_str().map(str::to_owned);
        let optional = |key: &str| -> Option<Option<String>> {
            match stored.get(key)? {
                serde_json::Value::Null => Some(None),
                value => Some(Some(value.as_str()?.to_owned())),
            }
        };
        Some(Self {
            host: text("host")?,
            port: u16::try_from(stored.get("port")?.as_u64()?).ok()?,
            presented_algorithm: text("presented_algorithm")?,
            presented_fingerprint: text("presented_fingerprint")?,
            pinned_algorithm: optional("pinned_algorithm")?,
            pinned_fingerprint: optional("pinned_fingerprint")?,
        })
    }
}

contract_enum!(
    /// What a conflict that a synchronization stopped on needs next:
    /// `resolve` when every conflicted path is canonical content of one
    /// item, which can be resolved in Manyhands, and `external_repair`
    /// when any is not, so the merge is finished in Git outside it. The
    /// set is one or the other as a whole.
    ConflictNextAction {
        Resolve => "resolve",
        ExternalRepair => "external_repair",
    }
);

/// The next step of a synchronization that stopped on a conflict: the
/// synchronization's operation, whose merge is left in its target
/// worktree, and what that merge needs. It names no path and holds no
/// content.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ConflictNextDto {
    /// The synchronization the conflict is of.
    pub operation_id: String,
    pub next_action: ConflictNextAction,
}

impl ConflictNextDto {
    /// The data as a finished record stored it.
    pub(crate) fn from_stored(stored: &serde_json::Value) -> Option<Self> {
        let next_action = stored.get("next_action")?.as_str()?;
        Some(Self {
            operation_id: stored.get("operation_id")?.as_str()?.to_owned(),
            next_action: ConflictNextAction::ALL
                .into_iter()
                .find(|known| known.as_str() == next_action)?,
        })
    }
}

contract_enum!(
    /// The tag of `SynchronizationConflictMutationDto`: one value, so
    /// that the data of a synchronization is told apart by it alone.
    SynchronizationConflictType {
        SynchronizationConflict => "synchronization_conflict",
    }
);

/// What `item sync` and `repo sync` report when they stopped on a
/// conflict in a merge of their own, and an `operation resume` that took
/// such a synchronization up: the conflict's next step. The merge is left
/// in the target worktree, the branch stays where the merge began, and
/// the result is `recovery_required` with the integration pending.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SynchronizationConflictMutationDto {
    pub r#type: SynchronizationConflictType,
    pub conflict: ConflictNextDto,
}

impl SynchronizationConflictMutationDto {
    /// The data as a finished record stored it.
    pub(crate) fn from_stored(stored: &serde_json::Value) -> Option<Self> {
        let tag = stored.get("type")?.as_str()?;
        Some(Self {
            r#type: SynchronizationConflictType::ALL
                .into_iter()
                .find(|known| known.as_str() == tag)?,
            conflict: ConflictNextDto::from_stored(stored.get("conflict")?)?,
        })
    }
}

/// What `operation abandon` reports of the operation it closed: where
/// the operation stood, and what it left on disk for its user to deal
/// with in Git. Every effect of an abandon is `not_requested`: this,
/// with the outcome, is what says what it did. It holds names and paths
/// and no content.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OperationMutationDto {
    /// The operation that was closed.
    pub operation_id: String,
    /// What the operation did, as `operation show` names it. Null for
    /// an action this version does not know.
    pub action: Option<OperationAction>,
    /// The state the operation was in when it was closed.
    pub state: String,
    /// The last step the operation had recorded as done. Null when it
    /// had recorded none.
    pub step: Option<String>,
    /// The item the operation wrote. Null for an operation that writes
    /// no item.
    pub item_id: Option<String>,
    /// The worktree the operation wrote in, relative to the repository's
    /// root with forward slashes: an item's own, or `.` for the primary
    /// worktree. Null for an operation that writes no file of a
    /// worktree, and for an item whose worktree is not there.
    pub worktree: Option<String>,
    /// The paths of the operation's target that differ from the head of
    /// their branch, relative to the repository's root with forward
    /// slashes. Each is left exactly as it was.
    pub paths: Vec<String>,
    /// Whether `paths` is all of them. False when there are more than
    /// are listed, or the worktree could not be read.
    pub paths_complete: bool,
    /// Whether the operation had recorded that it passed the point where
    /// its work is committed. That commit is kept: an abandon undoes
    /// nothing. For a comment whose step was `comment_checkpoint_intent`
    /// the commit may not exist; `paths` holds the comment's file when
    /// it does not. The released request, sent again, is a new request,
    /// and is answered by what the repository then shows.
    pub committed: bool,
    /// The request whose record was deleted with the operation: its ID
    /// is free, and its next use is a new request. Null when no
    /// unfinished request held the operation.
    pub released_request_id: Option<String>,
}

impl OperationMutationDto {
    /// The data as a finished record stored it.
    pub(crate) fn from_stored(stored: &serde_json::Value) -> Option<Self> {
        let text = |key: &str| -> Option<Option<String>> {
            match stored.get(key)? {
                serde_json::Value::Null => Some(None),
                value => Some(Some(value.as_str()?.to_owned())),
            }
        };
        Some(Self {
            operation_id: text("operation_id")??,
            action: match text("action")? {
                Some(action) => Some(
                    OperationAction::ALL
                        .into_iter()
                        .find(|known| known.as_str() == action)?,
                ),
                None => None,
            },
            state: text("state")??,
            step: text("step")?,
            item_id: text("item_id")?,
            worktree: text("worktree")?,
            paths: stored
                .get("paths")?
                .as_array()?
                .iter()
                .map(|value| value.as_str().map(str::to_owned))
                .collect::<Option<_>>()?,
            paths_complete: stored.get("paths_complete")?.as_bool()?,
            // A record stored before an abandon said so holds none.
            committed: stored
                .get("committed")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
            released_request_id: text("released_request_id")?,
        })
    }
}

contract_enum!(
    /// Where a key deletion that has not ended stands, as
    /// `operation show` names the state of its operation: `prepared`
    /// before the private key file is recorded as removed,
    /// `private_removed` once it is, and `files_removed` once the public
    /// key file is too and the key is still registered.
    KeyDeletionState {
        Prepared => "prepared",
        PrivateRemoved => "private_removed",
        FilesRemoved => "files_removed",
    }
);

contract_enum!(
    /// Where a key generation that has not registered its key stands, as
    /// `operation show` names the state of its operation.
    KeyGenerationState {
        Reserved => "reserved",
        PrivateWritten => "private_written",
        PairWritten => "pair_written",
        Completed => "completed",
        RetainedForInspection => "retained_for_inspection",
    }
);

/// What a key command reports about its key. Every effect of a key
/// command is `not_requested`: this, with the outcome, is what says what
/// it did. It holds no path, no key material and no passphrase.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct KeyMutationDto {
    /// The key the request is about: the one it generated, registered,
    /// selected, removed or deleted, or was refused for, and for a `key import` of
    /// a source that is already registered, the key that holds it. A
    /// `key generate` names its key from the moment it reserves the
    /// key's ID, which is before the key is registered. Null for
    /// `key clear`, which names none, and for a `key generate` or a
    /// `key import` that got no key.
    pub id: Option<String>,
    /// The key's label: as its registration holds it, when the request
    /// read the registration, and for a `key generate` the label the
    /// request gives its key.
    pub label: Option<String>,
    /// The fingerprint the key's registration stores, when the request
    /// read the registration and it stores one.
    pub fingerprint: Option<String>,
    /// Set only by a `key generate` that has not registered its key:
    /// where its generation stands.
    pub generation: Option<KeyGenerationState>,
    /// Set only by a `key delete` that began and has not ended: where
    /// its deletion stands. The same request completes it.
    pub deletion: Option<KeyDeletionState>,
}

impl KeyMutationDto {
    /// The data as a finished record stored it.
    pub(crate) fn from_stored(stored: &serde_json::Value) -> Option<Self> {
        let text = |key: &str| -> Option<Option<String>> {
            match stored.get(key)? {
                serde_json::Value::Null => Some(None),
                value => Some(Some(value.as_str()?.to_owned())),
            }
        };
        Some(Self {
            id: text("id")?,
            label: text("label")?,
            fingerprint: text("fingerprint")?,
            generation: match text("generation")? {
                Some(state) => Some(
                    KeyGenerationState::ALL
                        .into_iter()
                        .find(|known| known.as_str() == state)?,
                ),
                None => None,
            },
            // A record stored before a deletion had a state holds none.
            deletion: match stored.get("deletion") {
                None | Some(serde_json::Value::Null) => None,
                Some(state) => Some(
                    KeyDeletionState::ALL
                        .into_iter()
                        .find(|known| Some(known.as_str()) == state.as_str())?,
                ),
            },
        })
    }
}

/// What `remote add`, `remote select` and `remote remove` report about
/// the remote. The effects of a `remote add` and of a `remote remove`
/// are all `not_requested`: this, with the outcome, is what says what
/// it did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RemoteMutationDto {
    /// The remote the request names. Null for a `remote select` that
    /// clears the selection.
    pub name: Option<String>,
    /// The location a `remote add` gave the remote, redacted: it is not
    /// a location to connect to. Null for `remote select` and
    /// `remote remove`, which name a remote and give no location.
    pub location: Option<String>,
}

impl RemoteMutationDto {
    /// The data as a finished record stored it.
    pub(crate) fn from_stored(stored: &serde_json::Value) -> Option<Self> {
        let text = |key: &str| -> Option<Option<String>> {
            match stored.get(key)? {
                serde_json::Value::Null => Some(None),
                value => Some(Some(value.as_str()?.to_owned())),
            }
        };
        Some(Self {
            name: text("name")?,
            location: text("location")?,
        })
    }
}

/// What `document create`, `document save` and `document move` report
/// about the document.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DocumentMutationDto {
    pub id: String,
    /// The file the request writes, relative to the document's context,
    /// with forward slashes: for a move, the destination. Null when the
    /// request stopped before the document's file was found.
    pub path: Option<String>,
}

impl DocumentMutationDto {
    /// The data as a finished record stored it.
    pub(crate) fn from_stored(stored: &serde_json::Value) -> Option<Self> {
        Some(Self {
            id: stored.get("id")?.as_str()?.to_owned(),
            path: match stored.get("path")? {
                serde_json::Value::Null => None,
                path => Some(path.as_str()?.to_owned()),
            },
        })
    }
}

/// What `ticket create`, `ticket save` and `ticket slug-assign` report
/// about the ticket.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TicketMutationDto {
    pub id: String,
    /// The ticket's file, relative to its context, with forward slashes.
    pub path: String,
    /// The short code the ticket's file holds once the request has
    /// written or found it. Null until then, and for a ticket without one.
    pub slug: Option<String>,
    /// The dependencies and parent this request sets that nothing the
    /// index holds has the ID of, in ID order, each once. They are
    /// accepted.
    pub unresolved: Vec<String>,
    /// Empty unless the request was rejected for its relationships. Then,
    /// in ID order: for `relationship_cycle` the tickets of the cycle, the
    /// ticket itself included, and for `invalid_relationship` the IDs that
    /// are a document's or a comment's.
    pub rejected: Vec<String>,
}

impl TicketMutationDto {
    /// The data as a finished record stored it.
    pub(crate) fn from_stored(stored: &serde_json::Value) -> Option<Self> {
        let strings = |key: &str| -> Option<Vec<String>> {
            stored
                .get(key)?
                .as_array()?
                .iter()
                .map(|value| value.as_str().map(str::to_owned))
                .collect()
        };
        Some(Self {
            id: stored.get("id")?.as_str()?.to_owned(),
            path: stored.get("path")?.as_str()?.to_owned(),
            slug: match stored.get("slug")? {
                serde_json::Value::Null => None,
                slug => Some(slug.as_str()?.to_owned()),
            },
            unresolved: strings("unresolved")?,
            rejected: strings("rejected")?,
        })
    }
}

impl MutationDataDto {
    /// The host the data is about, as a recovery action names it. Only
    /// the data of a synchronization that stopped for its host names
    /// one.
    pub(crate) fn authority(&self) -> Option<String> {
        match self {
            Self::LocalContextSync(_) => None,
            Self::Synchronization(data) => Some(data.authority()),
            Self::SynchronizationConflict(_)
            // A comment's data names no host: its synchronization's is
            // not kept.
            | Self::Comment(_)
            // Nor does a resolution's.
            | Self::ConflictResolution(_)
            | Self::Document(_)
            | Self::Ticket(_)
            | Self::Remote(_)
            | Self::Key(_)
            | Self::Operation(_)
            | Self::Identity(_)
            // A host command is itself the approval: its results offer
            // none.
            | Self::Host(_)
            | Self::Repair(_)
            | Self::Context(_)
            // A lifecycle's category carries no authority (design, Answers).
            | Self::Lifecycle(_) => None,
        }
    }

    /// The data a finished record of `command` stored, in the shape of
    /// that command's family. `None` for data that is not of that shape.
    pub(crate) fn from_stored(command: Command, stored: &serde_json::Value) -> Option<Self> {
        match command {
            Command::LocalContextSync | Command::LocalContextResolve => {
                crate::repository::LocalContextSyncMutationDto::from_stored(stored)
                    .map(Self::LocalContextSync)
            }
            Command::OperationResume
                if stored.get("type").and_then(|t| t.as_str()) == Some("local_context_sync") =>
            {
                crate::repository::LocalContextSyncMutationDto::from_stored(stored)
                    .map(Self::LocalContextSync)
            }
            // A resume that took a comment's publication up gives the
            // comment's data (R-20), told apart by its own tag.
            Command::OperationResume
                if stored.get("type").and_then(|t| t.as_str())
                    == Some(CommentMutationType::Comment.as_str()) =>
            {
                CommentMutationDto::from_stored(stored).map(Self::Comment)
            }
            Command::DocumentCreate | Command::DocumentSave | Command::DocumentMove => {
                DocumentMutationDto::from_stored(stored).map(Self::Document)
            }
            Command::TicketCreate | Command::TicketSave | Command::TicketSlugAssign => {
                TicketMutationDto::from_stored(stored).map(Self::Ticket)
            }
            Command::RemoteAdd | Command::RemoteSelect | Command::RemoteRemove => {
                RemoteMutationDto::from_stored(stored).map(Self::Remote)
            }
            Command::KeyGenerate
            | Command::KeyImport
            | Command::KeySelect
            | Command::KeyClear
            | Command::KeyRemove
            | Command::KeyDelete => KeyMutationDto::from_stored(stored).map(Self::Key),
            Command::OperationAbandon => {
                OperationMutationDto::from_stored(stored).map(Self::Operation)
            }
            Command::RepoIdentitySet => {
                IdentityMutationDto::from_stored(stored).map(Self::Identity)
            }
            Command::HostApprove | Command::HostReplace => {
                HostMutationDto::from_stored(stored).map(Self::Host)
            }
            Command::DocumentRepair | Command::TicketRepair => {
                RepairMutationDto::from_stored(stored).map(Self::Repair)
            }
            Command::CommentAdd => CommentMutationDto::from_stored(stored).map(Self::Comment),
            Command::ConflictResolve => {
                ConflictResolutionMutationDto::from_stored(stored).map(Self::ConflictResolution)
            }
            // A synchronization that stopped on a conflict of its own
            // merge, or a resume that took one up, is told apart by its
            // tag.
            Command::ItemSync | Command::RepoSync | Command::OperationResume
                if stored.get("type").and_then(|t| t.as_str())
                    == Some(SynchronizationConflictType::SynchronizationConflict.as_str()) =>
            {
                SynchronizationConflictMutationDto::from_stored(stored)
                    .map(Self::SynchronizationConflict)
            }
            Command::ContextPrepare => ContextMutationDto::from_stored(stored).map(Self::Context),
            Command::DocumentPromote | Command::TicketClose => {
                LifecycleMutationDto::from_stored(stored)
                    .map(|data| Self::Lifecycle(Box::new(data)))
            }
            // A resume reports data only of a synchronization it took
            // up: the host that synchronization stopped for, or the
            // conflict above. A republish reports only the host it
            // stopped for: it merges nothing.
            Command::ItemSync
            | Command::RepoSync
            | Command::OperationResume
            | Command::ItemRepublish => {
                SynchronizationMutationDto::from_stored(stored).map(Self::Synchronization)
            }
            // These report no data, so a record of one stores none.
            Command::IndexRefresh
            | Command::IndexRebuild
            | Command::RepoCreate
            | Command::RepoEnable
            | Command::RepoRemove
            | Command::FolderCreate
            | Command::ContextRelease => None,
        }
    }
}
