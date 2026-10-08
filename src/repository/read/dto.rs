//! The data shapes of JSON v1 reads.
//!
//! These are the only read types that serialize. A DTO is built from a
//! domain value field by field; no domain type serializes. Every field is
//! always present, and an absent value is `null`.

use serde::{Serialize, Serializer, ser::SerializeStruct};

use crate::results::{OperationFailureCode, ProblemCode, contract_enum};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct NewIdDto {
    pub id: String,
}

/// A problem found in observed content.
///
/// `guidance` is written from the code's registry entry when the problem
/// serializes, so text stored with a problem can never take its place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProblemDto {
    pub code: ProblemCode,
    /// Repository-relative, or `None` when the problem has no path or its
    /// path cannot be written as one.
    pub path: Option<String>,
    /// The ID a problem with a ticket's `deps` or `parent` is about: the
    /// ticket's own for `relationship_self_reference`, the repeated one
    /// for `duplicate_dependency`, and the document's or comment's for
    /// `relationship_not_a_ticket`. `None` for every other problem. It is
    /// always a well-formed item ID and never text from the file.
    pub target_id: Option<String>,
}

impl ProblemDto {
    pub fn guidance(&self) -> &'static str {
        self.code.guidance()
    }
}

impl Serialize for ProblemDto {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut problem = serializer.serialize_struct("ProblemDto", 4)?;
        problem.serialize_field("code", &self.code)?;
        problem.serialize_field("path", &self.path)?;
        problem.serialize_field("target_id", &self.target_id)?;
        problem.serialize_field("guidance", self.guidance())?;
        problem.end()
    }
}

contract_enum!(
    /// Whether a registered root could be read when it was last observed.
    Accessibility {
        Accessible => "accessible",
        Inaccessible => "inaccessible",
    }
);

contract_enum!(ConfigurationState {
    Valid => "valid",
    Invalid => "invalid",
    Missing => "missing",
});

contract_enum!(
    /// How far the index is behind the repository it describes.
    IndexState {
        Current => "current",
        Stale => "stale",
        NeverRefreshed => "never_refreshed",
    }
);

contract_enum!(
    /// Whether a commit could be made with the identity now configured.
    IdentityAvailability {
        Available => "available",
        Required => "required",
    }
);

contract_enum!(
    /// The Git configuration level a complete identity was found at.
    IdentitySource {
        Repository => "repository",
        Global => "global",
        Xdg => "xdg",
        System => "system",
        ProgramData => "program_data",
        Application => "application",
        None => "none",
    }
);

/// The list shape of JSON v1: `items` and `complete`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RepositoryListDto {
    pub items: Vec<RepositorySummaryDto>,
    pub complete: bool,
}

/// One registration, as the index last stored it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RepositorySummaryDto {
    pub root: String,
    pub enabled_at: Option<String>,
    pub accessibility: Accessibility,
    pub configuration: ConfigurationDto,
    pub index: IndexStateDto,
    pub problem_count: u64,
}

/// `primary_branch` and `publication_remote` are set only when `state` is
/// `valid`; `problems` is empty unless it is `invalid`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ConfigurationDto {
    pub state: ConfigurationState,
    pub primary_branch: Option<String>,
    pub publication_remote: Option<String>,
    pub problems: Vec<ProblemDto>,
}

/// `never_refreshed` when nothing has been stored for the registration: no
/// refresh or rebuild has completed and the index holds no context for it.
/// Otherwise `stale` when a refresh is required, or when the index was
/// written before refresh times were recorded, and `current` when neither.
/// `refreshed_at` is the time the last completed refresh or rebuild began
/// observing, and null when none is recorded.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct IndexStateDto {
    pub state: IndexState,
    pub refreshed_at: Option<String>,
}

/// `selected_path` is the canonical path the caller gave; `root` is the
/// repository it selects, which differs for a linked worktree.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RepositoryInspectionDto {
    pub selected_path: String,
    pub root: String,
    pub registered: bool,
    pub head_branch: Option<String>,
    pub local_branches: Vec<String>,
    pub configuration: ConfigurationDto,
    pub identity_state: IdentityAvailability,
    pub remotes: Vec<RemoteDto>,
}

/// `name` and `email` are both set, or `source` is `none` and neither is.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct IdentityDto {
    pub name: Option<String>,
    pub email: Option<String>,
    pub source: IdentitySource,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RemoteListDto {
    pub items: Vec<RemoteDto>,
    pub complete: bool,
}

/// Both locations are redacted; neither is a URL to connect to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RemoteDto {
    pub name: String,
    pub fetch_location: String,
    pub push_location: String,
    pub publication_eligible: bool,
    pub selected_for_publication: bool,
}

contract_enum!(
    /// Who created a key's files: the user, or Manyhands.
    KeyOwnership {
        Imported => "imported",
        Generated => "generated",
    }
);

contract_enum!(
    /// What the private key file was when it was last observed. No read
    /// observes it again.
    KeyPrivateSourceState {
        Available => "available",
        Missing => "missing",
        Unavailable => "unavailable",
    }
);

contract_enum!(
    /// Whether the registration has a public key file, and whether a
    /// fingerprint could be read from it when it was registered.
    KeyPublicMetadataState {
        NotProvided => "not_provided",
        Available => "available",
        Unavailable => "unavailable",
    }
);

/// Registrations in the order they were registered.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct KeyListDto {
    pub items: Vec<KeyDto>,
    pub complete: bool,
}

/// One key registration, as the index stores it. Nothing here is read from
/// a key file: the states and the fingerprint are those recorded when the
/// key was registered or generated.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct KeyDto {
    pub id: String,
    pub label: String,
    pub ownership: KeyOwnership,
    pub selected: bool,
    /// Set exactly when `public_metadata_state` is `available`.
    pub fingerprint: Option<String>,
    pub private_source_state: KeyPrivateSourceState,
    pub public_metadata_state: KeyPublicMetadataState,
    pub private_key_path: String,
    pub public_key_path: Option<String>,
}

/// The public key now in a registration's public key file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PublicKeyDto {
    pub id: String,
    /// The file's one line of OpenSSH public key text, without its line
    /// ending.
    pub public_key: String,
    /// Computed from `public_key`, not taken from the registration.
    pub fingerprint: String,
    /// Whether `fingerprint` equals the registration's stored fingerprint.
    /// `false` when the registration stores none.
    pub matches_registration: bool,
}

/// Host pins ordered by host, then port.
///
/// `reapproval_required` is the application-wide marker left when the pin
/// registry was lost. It is reported here as well as on each pin because
/// that loss leaves no pins to report it on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HostPinListDto {
    pub items: Vec<HostPinDto>,
    pub complete: bool,
    pub reapproval_required: bool,
}

/// One pinned host key. `reapproval_required` is the application-wide
/// marker left when the pin registry was lost, so it is the same for every
/// pin of one read.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HostPinDto {
    pub host: String,
    pub port: u16,
    pub algorithm: String,
    pub sha256: String,
    pub reapproval_required: bool,
}

contract_enum!(
    /// The two kinds of item. A comment is not an item.
    ItemDtoKind {
        Document => "document",
        Ticket => "ticket",
    }
);

contract_enum!(
    /// Whether a ticket carries lifecycle closure metadata. Its `status`
    /// text does not decide this.
    ClosureState {
        Open => "open",
        Closed => "closed",
    }
);

contract_enum!(
    /// What a dependency's target is, among the tickets now present.
    DependencyState {
        Open => "open",
        Closed => "closed",
        Unresolved => "unresolved",
    }
);

contract_enum!(ReadinessState {
    Ready => "ready",
    Blocked => "blocked",
    Closed => "closed",
});

contract_enum!(
    /// Why an open ticket is blocked.
    ReadinessReasonCode {
        OpenDependency => "open_dependency",
        UnresolvedDependency => "unresolved_dependency",
        DependencyCycle => "dependency_cycle",
    }
);

contract_enum!(
    /// The working tree an item was read from: the repository root on its
    /// primary branch, the root when that could not be verified, or the
    /// worktree created to edit the item.
    ItemContextKind {
        Primary => "primary",
        Unverified => "unverified",
        Active => "active",
    }
);

contract_enum!(
    /// Where an item's content-change time comes from.
    ChangeSource {
        GitCommit => "git_commit",
        Uncommitted => "uncommitted",
    }
);

/// Documents ordered by path and then ID; tickets by content-change time,
/// latest first, and then ID. Nonconforming entries have no ID: among
/// documents they sort by path with the rest, and among tickets they
/// follow every ticket, ordered by path.
///
/// A list opens no file, so while a refresh is under way it may name an
/// item worktree as the context of an item whose file there is gone, where
/// a complete read of that item returns the primary copy.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ItemListDto {
    pub items: Vec<ItemDto>,
    pub complete: bool,
    pub index: IndexStateDto,
}

/// A document or a ticket, or a file where one should be.
///
/// A nonconforming entry has no `id` and no metadata; its `path`, `context`
/// and `problems` say what is wrong and where. In a list `body`, `source`
/// and `observation` are always null, and nothing here was read from a
/// file: every value is what the index stored.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ItemDto {
    pub id: Option<String>,
    pub kind: ItemDtoKind,
    /// Relative to `context.worktree`, with forward slashes.
    pub path: String,
    pub title: Option<String>,
    #[serde(rename = "type")]
    pub ticket_type: Option<String>,
    pub status: Option<String>,
    pub project: Option<String>,
    pub team: Option<String>,
    /// Null for a document and for a nonconforming entry.
    pub closure: Option<ClosureDto>,
    /// A ticket's short code, in lowercase. Null when it has none, and when
    /// its `slug` is not one, which `problems` reports. It is a label to
    /// search by and never identifies a ticket.
    pub slug: Option<String>,
    /// A ticket's parent, in the shape of a dependency: `unresolved` when
    /// no context holds a ticket with that ID. Null when it has none, and
    /// when its `parent` was ignored: `problems` says why.
    pub parent: Option<DependencyDto>,
    /// A ticket's dependencies in its file's order, each once. An entry
    /// that was ignored is not here, and `problems` says why.
    pub deps: Vec<DependencyDto>,
    /// Null: readiness is not computed yet.
    pub readiness: Option<ReadinessDto>,
    /// Front matter keys Manyhands does not define, in key order at every
    /// depth. A ticket's `slug`, `parent` and `deps` are fields above and
    /// are not here. A value JSON cannot express is null and is reported
    /// in `problems` as `metadata_not_representable`.
    pub unknown_metadata: serde_json::Map<String, serde_json::Value>,
    pub body: Option<String>,
    /// The file as it is on disk. Null when it is not valid UTF-8.
    pub source: Option<String>,
    /// Opaque. It changes when `context.branch`, `path` or the file's
    /// bytes change.
    pub observation: Option<String>,
    pub context: ItemContextDto,
    /// Null when the index holds no item at this path: for a nonconforming
    /// entry, and for a file read by path that the index does not list.
    pub changed_at: Option<String>,
    pub change_source: Option<ChangeSource>,
    pub problems: Vec<ProblemDto>,
    /// In a list, the list's index state. In a complete read it is `stale`
    /// when the index is otherwise current and the file is no longer what
    /// the index stored.
    pub index: IndexStateDto,
}

/// `closed_at` is set exactly when `state` is `closed`. `closed_by` is set
/// with it, except in a list from an index written before `closed_by` was
/// stored, where it is null until the next refresh.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ClosureDto {
    pub state: ClosureState,
    pub closed_at: Option<String>,
    pub closed_by: Option<String>,
}

/// A ticket another ticket depends on or has as its parent. `state` is
/// what `id` names in the index when it is read: an open ticket, a closed
/// one, or nothing in any context it has seen.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DependencyDto {
    pub id: String,
    pub state: DependencyState,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ReadinessDto {
    pub state: ReadinessState,
    pub reasons: Vec<ReadinessReasonDto>,
}

/// `ids` is the dependency for `open_dependency` and
/// `unresolved_dependency`, and the tickets of the cycle for
/// `dependency_cycle`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ReadinessReasonDto {
    pub code: ReadinessReasonCode,
    pub ids: Vec<String>,
}

/// As the index stored it at the last refresh. `branch` and `head_oid` are
/// null for a root with no branch checked out, or no commit, and for a
/// root the index holds no observation of.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ItemContextDto {
    pub kind: ItemContextKind,
    pub branch: Option<String>,
    pub worktree: String,
    pub head_oid: Option<String>,
}

/// An item's comment threads: the root comments in `created_at` and then ID
/// order, each with its replies in the same order beneath it, followed by a
/// nonconforming entry, in path order, for each file among the item's
/// comments that is not a comment of it.
///
/// `context` is the item's effective copy, which every comment was read
/// from. `index` is `stale` when the item's file or a comment's is newer
/// than the last refresh, or the comments are no longer what the index
/// stored.
///
/// The index says which files there are, so a comment added since the
/// last refresh is not listed until the indexer has seen it. `complete` is
/// about something else: it is false when the last refresh could not read
/// the directory that holds the comments, or stopped part of the way
/// through it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CommentListDto {
    pub items: Vec<CommentDto>,
    pub complete: bool,
    pub context: ItemContextDto,
    pub index: IndexStateDto,
}

/// A comment, or a file where one should be.
///
/// A nonconforming entry has no `id`, `parent_id`, `author`, `created_at`
/// or `body` and no replies; its `path` and `problems` say what is wrong
/// and where. Everything else here was read from the comment's file. The
/// reason is found by checking the file again, except `duplicate_id` and
/// `cross_item_parent`, which take the whole context to find and are what
/// the last refresh stored.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CommentDto {
    pub id: Option<String>,
    /// The item the comment is filed under.
    pub item_id: String,
    /// Null for a root comment.
    pub parent_id: Option<String>,
    /// The comment's `created_by` front matter value, and null when it has
    /// none. Nothing is derived from Git history.
    pub author: Option<String>,
    pub created_at: Option<String>,
    pub body: Option<String>,
    /// Relative to the list's `context.worktree`, with forward slashes.
    pub path: String,
    /// Front matter keys Manyhands does not define, in key order at every
    /// depth; `created_by` is never among them. A value JSON cannot express
    /// is null and is reported in `problems` as
    /// `metadata_not_representable`.
    pub unknown_metadata: serde_json::Map<String, serde_json::Value>,
    /// On a comment that has an `id`, only what does not stop it being one:
    /// `metadata_not_representable`, and `invalid_field` for a `created_by`
    /// that is not a non-empty string.
    pub problems: Vec<ProblemDto>,
    pub replies: Vec<CommentDto>,
}

contract_enum!(
    /// What the index status read found. `unavailable` is an index that
    /// cannot be read at all, which every other read reports as the
    /// `index_unavailable` failure.
    IndexStatusState {
        Current => "current",
        Stale => "stale",
        NeverRefreshed => "never_refreshed",
        Unavailable => "unavailable",
    }
);

/// One registration's index: how far behind it is, how much it holds and
/// what it could not make sense of.
///
/// `state` and `refreshed_at` are what every list reports as its `index`,
/// with one more state: an index that cannot be read is `unavailable`, and
/// then the counts are null, both lists are empty, and rebuilding the index
/// is the way out. Nothing here is found by looking at the repository: a
/// change the index has not been told of does not make it `stale`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct IndexStatusDto {
    pub state: IndexStatusState,
    pub refreshed_at: Option<String>,
    /// The working trees the index observed: the root, and each item
    /// worktree.
    pub context_count: Option<u64>,
    /// Documents and tickets, each counted once however many working trees
    /// hold a copy.
    pub item_count: Option<u64>,
    /// The length of `problems`.
    pub problem_count: Option<u64>,
    /// Every problem the last refresh stored, ordered by worktree, path and
    /// code. This is the only read that reports a problem with no path, or
    /// one about a branch, a working tree or an unreadable file.
    pub problems: Vec<IndexProblemDto>,
    /// The repository's local operations that have not completed, in the
    /// order they were stored and as the operation list has them. Each ends
    /// by bringing the index up to date, so until it does the index may not
    /// hold what it changed.
    pub pending_operations: Vec<OperationDto>,
}

/// A problem the index stored, and the working tree it was found in.
///
/// `guidance` is written from the code's registry entry when the problem
/// serializes, so text stored with a problem can never take its place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexProblemDto {
    pub code: ProblemCode,
    /// As the index stored it, and null when the problem has no path:
    /// relative to `worktree` when the refresh could make it so, which is
    /// when the problem is about something inside that working tree, and
    /// absolute otherwise.
    pub path: Option<String>,
    /// Null for a problem about the registration as a whole.
    pub worktree: Option<String>,
}

impl IndexProblemDto {
    pub fn guidance(&self) -> &'static str {
        self.code.guidance()
    }
}

impl Serialize for IndexProblemDto {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut problem = serializer.serialize_struct("IndexProblemDto", 4)?;
        problem.serialize_field("code", &self.code)?;
        problem.serialize_field("path", &self.path)?;
        problem.serialize_field("worktree", &self.worktree)?;
        problem.serialize_field("guidance", self.guidance())?;
        problem.end()
    }
}

contract_enum!(
    /// How the latest attempt to observe the publication remote ended.
    PollingOutcome {
        Completed => "completed",
        ConfigurationRequired => "configuration_required",
        SelectedKeyUnavailable => "selected_key_unavailable",
        UnlockRequired => "unlock_required",
        HostApprovalRequired => "host_approval_required",
        TransportUnavailable => "transport_unavailable",
        ProtocolRejected => "protocol_rejected",
        Cancelled => "cancelled",
        RepositoryUnavailable => "repository_unavailable",
    }
);

/// The stored polling policy of one registration and what polling last
/// observed. Nothing here says whether any process is polling now.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PollingStatusDto {
    pub enabled: bool,
    pub paused: bool,
    pub interval_seconds: u64,
    /// The delay automatic polling adds after failures, and null when it
    /// adds none.
    pub backoff_seconds: Option<u64>,
    /// Whether polling waits for the user after the index was recovered.
    pub recovery_suspended: bool,
    /// Null when none is stored.
    pub latest_outcome: Option<PollingOutcome>,
    /// When the remote's branches were last observed completely, which is
    /// not when `latest_outcome` was recorded: no time is stored for that.
    /// Null when they never were.
    pub latest_observed_at: Option<String>,
    /// The remote operation that holds the registration's reservation now,
    /// whatever its action, and null when none does.
    pub active_operation_id: Option<String>,
    /// Always null: nothing stores when the next attempt is due.
    pub next_eligible_at: Option<String>,
}

contract_enum!(
    /// Which of the three stores an operation is recorded in.
    OperationFamily {
        Local => "local",
        Remote => "remote",
        KeyMaterial => "key_material",
    }
);

contract_enum!(
    /// What an operation belongs to. A key-material operation belongs to
    /// the application and is listed with every repository.
    OperationScope {
        Repository => "repository",
        Application => "application",
    }
);

contract_enum!(
    /// What an operation does. The local store records the first twelve,
    /// the remote store the next five and the key-material store the last
    /// two; a local synchronization is reported under its remote name.
    OperationAction {
        CreateAndEnable => "create_and_enable",
        Enable => "enable",
        RemoveRegistration => "remove_registration",
        AddRemote => "add_remote",
        RemoveRemote => "remove_remote",
        SetPublicationRemote => "set_publication_remote",
        Refresh => "refresh",
        Rebuild => "rebuild",
        PrepareContext => "prepare_context",
        SaveDocument => "save_document",
        SaveTicket => "save_ticket",
        SubmitComment => "submit_comment",
        Poll => "poll",
        SynchronizeContext => "synchronize_context",
        SynchronizePrimary => "synchronize_primary",
        Promote => "promote",
        Close => "close",
        GenerateKey => "generate_key",
        DeleteKey => "delete_key",
    }
);

contract_enum!(
    /// What a listed operation waits for.
    ///
    /// `resume` is to ask for `action` again with the same operation ID;
    /// for a remote synchronization that stopped, as a restart. An
    /// operation with a null `operation_id` was recorded before operations
    /// had IDs and is always a refresh or a rebuild: asking for that action
    /// again under any ID takes it up. The other three are what
    /// key-material recovery offers.
    OperationNextAction {
        Resume => "resume",
        RetryGeneration => "retry_generation",
        ReviewDeletionAgain => "review_deletion_again",
        InspectRetainedFiles => "inspect_retained_files",
    }
);

/// The operations with work outstanding, ordered by operation ID, which is
/// the order they were started in; an operation recorded before operations
/// had IDs follows the rest, in the order it was stored.
///
/// These are:
///
/// - the local operations that have not completed;
/// - the remote operation that holds the reservation, in any phase but
///   `completed`, `interrupted`, `cancelled` and `failed`;
/// - the remote synchronizations that are `interrupted` or `failed`, which
///   can be resumed;
/// - the remote synchronizations that are `completed` while the index has
///   not caught up with them, and any remote operation with reconciliation
///   recorded as required;
/// - the key-material operations that have not completed or that failed.
///
/// A poll that ended is never listed, however it ended, so the list does
/// not grow with the number of polls. An operation that is not listed can
/// still be read by its ID.
///
/// One operation ID can be listed twice. A synchronization whose index
/// hand-off is pending is a remote operation, and once that hand-off has
/// begun it is also a local refresh under the same ID; resuming either is
/// resuming the other. The local one sorts first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OperationListDto {
    pub items: Vec<OperationDto>,
    pub complete: bool,
}

/// One stored operation, as its store last recorded it. Nothing is
/// observed again: this is not what Git or the file system holds now.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OperationDto {
    /// Null only for a local operation recorded before operations had IDs.
    pub operation_id: Option<String>,
    pub family: OperationFamily,
    pub scope: OperationScope,
    /// A synchronization with no remote to publish to is a local operation
    /// whose action is `synchronize_primary` or `synchronize_context`.
    pub action: OperationAction,
    /// The store's own name for where the operation stands, in
    /// lower_snake_case: a local operation's state, a remote operation's
    /// phase or a key-material operation's phase.
    ///
    /// `completed` is the last of them in all three, and it does not by
    /// itself mean nothing is left to do: a remote synchronization is
    /// `completed` once Git holds its outcome, while the index may still
    /// have to catch up, and a key generation can be `completed` and have
    /// lost its files since. Such an operation is still listed, and
    /// `next_action` says what it waits for.
    pub state: String,
    /// The last step a local or remote operation recorded as done, and
    /// null when it recorded none. Always null for a key-material
    /// operation, whose `state` is that step.
    pub completed_step: Option<String>,
    /// Null when nothing is left to do. Also null for a remote operation
    /// that holds the reservation, whose store does not say whether
    /// anything is still running it, and for one that cannot be resumed.
    pub next_action: Option<OperationNextAction>,
    /// The item a remote operation or a local synchronization is about.
    /// Null for other local operations, which do not record one.
    pub item_id: Option<String>,
    /// The key a key-material operation is about, as the key reads name
    /// it. Null for the other two families.
    pub key_id: Option<String>,
    /// The working tree a local operation last worked in, as an absolute
    /// path. Null for the other two families.
    pub worktree: Option<String>,
    /// When the store last changed the record. Null for a key-material
    /// operation, which stores no time.
    pub updated_at: Option<String>,
    /// Why a remote or key-material operation did not complete. Always
    /// null for a local operation, which stores no reason.
    pub failure_code: Option<OperationFailureCode>,
}
