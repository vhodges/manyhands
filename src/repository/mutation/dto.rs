//! The data shapes of the mutation boundary's JSON v1 results.
//!
//! As with the read DTOs, these are the only types here that serialize.
//! Every field is always present, and an absent value is `null`.

use serde::{Serialize, Serializer};

use super::command::Command;
use crate::{
    repository::OperationFamily,
    results::{Effects, Outcome, ResultCode, Scope, contract_enum},
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

/// The `data` of a mutation's envelope: the shape of the command's family.
/// It may accompany any outcome and never holds a body or a source.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum MutationDataDto {
    Document(DocumentMutationDto),
    Ticket(TicketMutationDto),
    Remote(RemoteMutationDto),
    Key(KeyMutationDto),
}

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
    /// selected or removed, or was refused for, and for a `key import` of
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
    /// The data a finished record of `command` stored, in the shape of
    /// that command's family. `None` for data that is not of that shape.
    pub(crate) fn from_stored(command: Command, stored: &serde_json::Value) -> Option<Self> {
        match command {
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
            | Command::KeyRemove => KeyMutationDto::from_stored(stored).map(Self::Key),
            // These report no data, so a record of one stores none.
            Command::IndexRefresh
            | Command::IndexRebuild
            | Command::RepoCreate
            | Command::RepoEnable
            | Command::RepoRemove => None,
        }
    }
}
