//! The data shapes of JSON v1 reads.
//!
//! These are the only read types that serialize. A DTO is built from a
//! domain value field by field; no domain type serializes. Every field is
//! always present, and an absent value is `null`.

use serde::{Serialize, Serializer, ser::SerializeStruct};

use crate::results::{OperationFailureCode, ProblemCode, ResultCode, contract_enum};

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
    /// `relationship_not_a_ticket`, the lowest ID of the cycle for
    /// `dependency_cycle`, and the parent for `parent_cycle`. `None` for
    /// every other problem. It is always a well-formed item ID and never
    /// text from the file.
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

/// One pinned host key. `fingerprint` has the form a key's `fingerprint` has:
/// `SHA256:` and the digest of the host's public key.
/// `reapproval_required` is the application-wide marker left when the pin
/// registry was lost, so it is the same for every pin of one read.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HostPinDto {
    pub host: String,
    pub port: u16,
    pub algorithm: String,
    pub fingerprint: String,
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

contract_enum!(
    /// Whether a ticket can be started. A closed ticket is neither ready
    /// nor blocked.
    ReadinessState {
        Ready => "ready",
        Blocked => "blocked",
        Closed => "closed",
    }
);

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
/// follow every ticket, ordered by path. The relationship queries return
/// tickets only, and the critical path in its own order.
///
/// A list opens no file, so while a refresh is under way it may name an
/// item worktree as the context of an item whose file there is gone, where
/// a complete read of that item returns the primary copy.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ItemListDto {
    pub items: Vec<ItemDto>,
    /// False when the last refresh could not read, or stopped part of the
    /// way through, a directory that holds items of the kind listed: `docs`
    /// or a directory under it for documents; `.manyhands/tickets` or
    /// `.manyhands` for tickets and for every relationship query; and for
    /// all of them `.manyhands/worktrees`, where item worktrees are. Items
    /// may then be missing, and `index.state` can still be `current`: a
    /// refresh stops at the same place again.
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
    /// Whether a ticket can be started, decided when it is read from what
    /// its dependencies are in the index then. In a complete read the
    /// ticket's own closure and dependencies are its file's, so this can
    /// differ from the list's until the index is refreshed. Null for a
    /// document and for a nonconforming entry.
    pub readiness: Option<ReadinessDto>,
    /// Front matter keys Manyhands does not define, in key order at every
    /// depth. A ticket's `slug`, `parent` and `deps` are fields above and
    /// are not here. A value JSON cannot express, and a list or mapping
    /// nested more than 64 deep, is null and is reported in `problems` as
    /// `metadata_not_representable`.
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

/// `closed` for a ticket with lifecycle closure metadata, whatever its
/// `status` says and whatever it depends on. An open ticket is `ready` when
/// every dependency is a closed ticket, and `blocked` otherwise: a closed
/// dependency never blocks, whatever it depends on in turn. `reasons` is
/// empty unless it is `blocked`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ReadinessDto {
    pub state: ReadinessState,
    pub reasons: Vec<ReadinessReasonDto>,
}

/// One cause of a ticket being blocked. `ids` is the dependency for
/// `open_dependency` and `unresolved_dependency`. For `dependency_cycle`
/// it is the open tickets that wait for each other, this one among them,
/// in ID order.
///
/// A ticket has one reason for each dependency that is an open ticket and
/// each that no context holds a ticket for, in its file's order, and then
/// one for the cycle of open tickets it is on, if it is on one.
///
/// Only open tickets block, so the cycle a reason names can be part of a
/// larger one the cycles read lists, which counts closed tickets too. A
/// ticket on a cycle that blocks nothing has no such reason; its
/// `problems` report the cycle either way.
///
/// A reason names at most sixteen tickets of a cycle, the lowest IDs, and
/// `complete` is false when the cycle has more.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ReadinessReasonDto {
    pub code: ReadinessReasonCode,
    pub ids: Vec<String>,
    pub complete: bool,
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

/// An item's comment threads as one flat list: the root comments in
/// `created_at` and then ID order, each followed at once by its replies in
/// the same order and each of those by its own, to any depth; then a
/// nonconforming entry, in path order, for each file among the item's
/// comments that is not a comment of it.
///
/// Nothing is nested, however long a chain of replies is: the threads are
/// rebuilt from `depth` and `parent_id`. An entry's `parent_id` is null
/// exactly when its `depth` is 0, and is otherwise the `id` of the nearest
/// earlier entry whose `depth` is one less.
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
/// or `body` and a `depth` of 0; its `path` and `problems` say what is
/// wrong and where. Everything else here was read from the comment's file,
/// but for `depth`, which is where the comment is in its thread. The
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
    /// How many comments are above this one in its thread: 0 for a root
    /// comment and for a nonconforming entry, and one more than its
    /// parent's for a reply.
    pub depth: u32,
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
    /// Null when it is not known when the next attempt is due. Nothing
    /// stores that so far, so no read fills it.
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
    /// What an operation belongs to: the `owner` of an operation. A
    /// key-material operation belongs to the application and is listed
    /// with every repository. This is not the envelope's `scope`, which
    /// says what a result is about.
    OperationOwner {
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
    pub owner: OperationOwner,
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
    ///
    /// A local operation that is `completed` with the step `rejected` was
    /// refused and changed nothing. Asking for it again under the same ID
    /// begins it anew.
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

contract_enum!(
    /// Which way a dependency tree is followed from its ticket: `down` to
    /// the tickets it depends on, `up` to the tickets that depend on it.
    DependencyDirection {
        Down => "down",
        Up => "up",
        Both => "both",
    }
);

contract_enum!(
    /// The ticket field whose links form a cycle.
    CycleKind {
        Deps => "deps",
        Parent => "parent",
    }
);

contract_enum!(
    /// Why closing tickets can never bring an open ticket's turn.
    UnplannableReasonCode {
        UnresolvedDependency => "unresolved_dependency",
        UnplannableDependency => "unplannable_dependency",
        DependencyCycle => "dependency_cycle",
    }
);

/// The trees of what one ticket depends on and of what depends on it.
///
/// Each tree is written out line by line, depth first, with every ticket's
/// neighbors in ID order. A line belongs under the nearest line before it
/// whose `depth` is one less, and a line of depth 1 under `ticket`. The
/// tree of a direction that was not asked for is empty.
///
/// A tree is not nested in the JSON so that its depth, which is as great
/// as the longest chain of dependencies, is never the depth of a document.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DependencyTreeDto {
    /// The ticket both trees start at, at depth 0. `truncated` when
    /// `depth` is 0 and it has edges in a direction that was asked for.
    pub ticket: DependencyTreeNodeDto,
    pub direction: DependencyDirection,
    /// The limit that was asked for, and null when there was none.
    pub depth: Option<u32>,
    pub dependencies: Vec<DependencyTreeNodeDto>,
    pub dependents: Vec<DependencyTreeNodeDto>,
    /// False when the last refresh could not read every ticket's
    /// directory, as for a list of tickets: tickets may be missing from
    /// what this was made from.
    pub complete: bool,
    pub index: IndexStateDto,
}

/// One line of a dependency tree.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DependencyTreeNodeDto {
    pub id: String,
    /// `unresolved` for a dependency no context holds a ticket for.
    pub state: DependencyState,
    /// The ticket's short code and title, for showing beside its ID. Both
    /// null for an unresolved dependency.
    pub slug: Option<String>,
    pub title: Option<String>,
    /// Steps from the ticket the tree starts at.
    pub depth: u32,
    /// The ID has another line in this tree, and its own edges are shown
    /// there: an ID is expanded once, where it is nearest the ticket. That
    /// line can come later than this one.
    pub repeated: bool,
    /// The ticket has edges that the depth limit kept out.
    pub truncated: bool,
}

/// Dependency cycles and then parent cycles, each kind ordered by its
/// cycles' lowest IDs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CycleListDto {
    pub items: Vec<CycleDto>,
    /// False when the last refresh could not read every ticket's
    /// directory, as for a list of tickets: tickets may be missing from
    /// what this was made from.
    pub complete: bool,
    pub index: IndexStateDto,
}

/// The tickets that can each reach itself through the others by `kind`'s
/// links. `ids` ascend; they are a set, not a path.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CycleDto {
    pub kind: CycleKind,
    pub ids: Vec<String>,
}

/// Every open ticket the filter matches, once: in a batch, or unplannable.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PlanDto {
    /// In batch order. A batch the filter left empty is not listed.
    pub batches: Vec<PlanBatchDto>,
    /// In ID order.
    pub unplannable: Vec<UnplannableTicketDto>,
    /// False when the last refresh could not read every ticket's
    /// directory, as for a list of tickets: tickets may be missing from
    /// what this was made from.
    pub complete: bool,
    pub index: IndexStateDto,
}

/// Tickets that can be worked at once, in ID order. `batch` counts from 1
/// in the plan of every ticket, whatever the filter: each ticket here has
/// all its open dependencies in batches with a lower number.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PlanBatchDto {
    pub batch: u64,
    pub items: Vec<ItemDto>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct UnplannableTicketDto {
    pub ticket: ItemDto,
    /// Never empty.
    pub reasons: Vec<UnplannableReasonDto>,
}

/// `ids` is the dependency for `unresolved_dependency` and for
/// `unplannable_dependency`, which is an open ticket that is itself
/// unplannable, and for `dependency_cycle` the open tickets that wait for
/// each other, named as a readiness reason names them: at most sixteen,
/// with `complete` false when the cycle has more.
///
/// A ticket has one reason for each such dependency, in its file's order,
/// and then one for the cycle of open tickets it is on. A dependency on its
/// own cycle is covered by the cycle and has no reason of its own.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct UnplannableReasonDto {
    pub code: UnplannableReasonCode,
    pub ids: Vec<String>,
    pub complete: bool,
}

/// What a proposed `deps` and `parent` come to for one ticket, and whether
/// a save that wrote them would be rejected.
///
/// The check is a read, and a read that could answer succeeds, so this is
/// the data of an `ok` envelope whether or not the proposal is acceptable.
/// A null `rejection` says a save may write these relationships. One that
/// is not null says the read succeeded and its answer is that this
/// proposal would be rejected with that code: the check itself did not
/// fail. The ticket bindings read it before anything else is done and turn
/// it into a rejected mutation that carries the code.
///
/// The first three fields describe the proposal and are filled either way.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RelationshipCheckDto {
    /// The proposed dependencies as a save writes them: in ID order, each
    /// once.
    pub deps: Vec<String>,
    pub parent: Option<String>,
    /// The proposed dependencies and parent that nothing the index holds
    /// has the ID of, in ID order, each once. They are accepted: a later
    /// fetch or merge may bring the ticket. The ticket's own ID is never
    /// one of them.
    pub unresolved: Vec<String>,
    pub rejection: Option<RelationshipRejectionDto>,
}

/// Why a save of the proposed relationships would be rejected.
///
/// `code` is one of `CODES`. For `invalid_relationship`, `ids` holds every
/// proposed ID that the index holds as a document or a comment. For
/// `relationship_cycle` it holds the tickets of the cycle the proposal
/// would put the ticket on, the ticket included and closed tickets
/// counted: a set, not a path, and the ticket alone when it names itself.
/// Either way `ids` is in ID order, each once.
///
/// One rejection is reported: `invalid_relationship` before any cycle, and
/// a dependency cycle before a parent cycle.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RelationshipRejectionDto {
    pub code: ResultCode,
    pub ids: Vec<String>,
}

impl RelationshipRejectionDto {
    /// Every code a rejection can carry.
    pub const CODES: [ResultCode; 2] = [
        ResultCode::RelationshipCycle,
        ResultCode::InvalidRelationship,
    ];
}
