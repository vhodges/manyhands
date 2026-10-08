//! The data shapes of JSON v1 reads.
//!
//! These are the only read types that serialize. A DTO is built from a
//! domain value field by field; no domain type serializes. Every field is
//! always present, and an absent value is `null`.

use serde::{Serialize, Serializer, ser::SerializeStruct};

use crate::results::{ProblemCode, contract_enum};

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
}

impl ProblemDto {
    pub fn guidance(&self) -> &'static str {
        self.code.guidance()
    }
}

impl Serialize for ProblemDto {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut problem = serializer.serialize_struct("ProblemDto", 3)?;
        problem.serialize_field("code", &self.code)?;
        problem.serialize_field("path", &self.path)?;
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
    pub slug: Option<String>,
    pub parent: Option<String>,
    pub deps: Vec<DependencyDto>,
    pub readiness: Option<ReadinessDto>,
    /// Front matter keys Manyhands does not define, in key order at every
    /// depth. A value JSON cannot express is null and is reported in
    /// `problems` as `metadata_not_representable`.
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
