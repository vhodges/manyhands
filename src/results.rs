//! The versioned result contract shared by the Manyhands front ends.
//!
//! These types serialize; domain types do not. Field names, enumeration
//! strings and result codes are the published JSON v1 contract. Enumeration
//! strings and result codes are written out here instead of being derived
//! from a Rust name. Field names are derived from the struct fields and are
//! pinned by the exact-JSON tests.

use std::path::{Component, Path};

use serde::{Serialize, Serializer};
use time::{OffsetDateTime, UtcOffset, format_description::well_known::Rfc3339};

pub const SCHEMA_VERSION: u32 = 1;

/// What `redact_url` returns for input it cannot parse.
pub const REDACTED: &str = "[redacted]";

/// Defines a contract enumeration whose JSON form is its explicit `as_str`.
macro_rules! contract_enum {
    ($(#[$meta:meta])* $name:ident { $($variant:ident => $string:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum $name {
            $($variant),+
        }

        impl $name {
            pub const ALL: [Self; [$(Self::$variant),+].len()] = [$(Self::$variant),+];

            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $string),+
                }
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(self.as_str())
            }
        }
    };
}

pub(crate) use contract_enum;

contract_enum!(Outcome {
    Success => "success",
    Noop => "noop",
    Partial => "partial",
    Blocked => "blocked",
    Cancelled => "cancelled",
    Error => "error",
});

contract_enum!(WriteEffect {
    NotRequested => "not_requested",
    Unchanged => "unchanged",
    Written => "written",
});

contract_enum!(CheckpointEffect {
    NotRequested => "not_requested",
    Unchanged => "unchanged",
    Committed => "committed",
    Pending => "pending",
});

contract_enum!(DiscoveryEffect {
    NotRequested => "not_requested",
    Current => "current",
    Pending => "pending",
});

contract_enum!(PublicationEffect {
    NotRequested => "not_requested",
    Published => "published",
    Current => "current",
    Pending => "pending",
});

contract_enum!(IntegrationEffect {
    NotRequested => "not_requested",
    Complete => "complete",
    Pending => "pending",
});

contract_enum!(CleanupEffect {
    NotRequested => "not_requested",
    Complete => "complete",
    Pending => "pending",
});

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Envelope<T> {
    pub schema_version: u32,
    pub command: String,
    pub request_id: Option<String>,
    pub operation_id: Option<String>,
    pub outcome: Outcome,
    pub code: ResultCode,
    pub message: String,
    pub scope: Scope,
    pub effects: Effects,
    pub data: Option<T>,
    pub recovery: Vec<RecoveryAction>,
}

impl<T> Envelope<T> {
    /// A completed read. The caller names the command, because a read
    /// service does not know which verb or view asked.
    pub fn read_success(command: impl Into<String>, scope: Scope, data: T) -> Self {
        Self::read(
            command.into(),
            scope,
            ResultCode::Ok,
            Some(data),
            Vec::new(),
        )
    }

    /// A read that returned no data. The outcome and message come from
    /// `code`, which must not be `ResultCode::Ok`.
    pub fn failure(
        command: impl Into<String>,
        scope: Scope,
        code: ResultCode,
        recovery: Vec<RecoveryAction>,
    ) -> Self {
        debug_assert!(code != ResultCode::Ok, "a failure cannot carry the ok code");
        Self::read(command.into(), scope, code, None, recovery)
    }

    fn read(
        command: String,
        scope: Scope,
        code: ResultCode,
        data: Option<T>,
        recovery: Vec<RecoveryAction>,
    ) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            command,
            request_id: None,
            operation_id: None,
            outcome: code.outcome(),
            code,
            message: code.message().to_owned(),
            scope,
            effects: Effects::not_requested(),
            data,
            recovery,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Scope {
    pub repository: Option<String>,
    pub item_id: Option<String>,
    pub branch: Option<String>,
    pub worktree: Option<String>,
    pub remote: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Effects {
    pub write: WriteEffect,
    pub checkpoint: CheckpointEffect,
    pub discovery: DiscoveryEffect,
    pub publication: PublicationEffect,
    pub integration: IntegrationEffect,
    pub cleanup: CleanupEffect,
    pub commit_oid: Option<String>,
}

impl Effects {
    /// The only value a read produces.
    pub fn not_requested() -> Self {
        Self {
            write: WriteEffect::NotRequested,
            checkpoint: CheckpointEffect::NotRequested,
            discovery: DiscoveryEffect::NotRequested,
            publication: PublicationEffect::NotRequested,
            integration: IntegrationEffect::NotRequested,
            cleanup: CleanupEffect::NotRequested,
            commit_oid: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RecoveryAction {
    pub action: String,
    pub operation_id: Option<String>,
    pub arguments: serde_json::Map<String, serde_json::Value>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FailureClass {
    Input,
    Blocked,
    Incomplete,
    Transient,
    Internal,
    Cancelled,
}

/// Defines `ResultCode` from one list, so a code cannot be added without its
/// string, class and message, or be left out of `ALL`.
macro_rules! result_codes {
    ($($variant:ident => $string:literal, $class:expr, $message:literal;)+) => {
        /// A stable result code. `as_str` is the contract; the variant name is not.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum ResultCode {
            $($variant),+
        }

        impl ResultCode {
            pub const ALL: [Self; [$(Self::$variant),+].len()] = [$(Self::$variant),+];

            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $string),+
                }
            }

            /// The fixed English explanation. It never carries backend text.
            pub const fn message(self) -> &'static str {
                match self {
                    $(Self::$variant => $message),+
                }
            }

            /// `None` only for `ok`.
            pub const fn failure_class(self) -> Option<FailureClass> {
                match self {
                    $(Self::$variant => $class),+
                }
            }
        }
    };
}

result_codes! {
    Ok => "ok", None,
        "The request completed.";
    InvalidPath => "invalid_path", Some(FailureClass::Input),
        "That path cannot be used as a target.";
    NotRepository => "not_repository", Some(FailureClass::Input),
        "No Git repository exists at that path.";
    NotRepositoryRoot => "not_repository_root", Some(FailureClass::Input),
        "That path is inside a repository but is not its root or a linked worktree root.";
    BareRepository => "bare_repository", Some(FailureClass::Input),
        "The repository has no working tree.";
    RepositoryNotRegistered => "repository_not_registered", Some(FailureClass::Blocked),
        "The repository is not enabled in Manyhands.";
    RepositoryInaccessible => "repository_inaccessible", Some(FailureClass::Blocked),
        "The repository cannot be read.";
    InvalidId => "invalid_id", Some(FailureClass::Input),
        "That ID is not a canonical ULID.";
    ItemNotFound => "item_not_found", Some(FailureClass::Input),
        "No item has that ID.";
    PathNotFound => "path_not_found", Some(FailureClass::Input),
        "No canonical resource exists at that path.";
    KeyNotFound => "key_not_found", Some(FailureClass::Input),
        "No key registration has that ID.";
    PublicKeyUnavailable => "public_key_unavailable", Some(FailureClass::Blocked),
        "The key registration has no readable public key.";
    AuthorityNotFound => "authority_not_found", Some(FailureClass::Input),
        "No host pin exists for that authority.";
    OperationNotFound => "operation_not_found", Some(FailureClass::Input),
        "No operation has that ID.";
    IndexUnavailable => "index_unavailable", Some(FailureClass::Blocked),
        "The index is degraded and must be rebuilt.";
    Busy => "busy", Some(FailureClass::Transient),
        "The repository is busy; try again.";
    InternalError => "internal_error", Some(FailureClass::Internal),
        "An internal error occurred.";
}

impl ResultCode {
    /// The outcome an envelope carrying this code reports.
    pub const fn outcome(self) -> Outcome {
        match self.failure_class() {
            None => Outcome::Success,
            Some(FailureClass::Blocked) => Outcome::Blocked,
            Some(FailureClass::Input | FailureClass::Transient | FailureClass::Internal) => {
                Outcome::Error
            }
            Some(FailureClass::Incomplete) => Outcome::Partial,
            Some(FailureClass::Cancelled) => Outcome::Cancelled,
        }
    }
}

impl Serialize for ResultCode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

/// Defines `ProblemCode` from one list: its contract string, the code string
/// the index stores for it, if any, and its guidance. Adding a code is one
/// entry.
macro_rules! problem_codes {
    ($($variant:ident => $string:literal, $stored:expr, $guidance:literal;)+) => {
        /// A stable code for a problem found in observed content. `as_str`
        /// is the contract; the variant name is not.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum ProblemCode {
            $($variant),+
        }

        impl ProblemCode {
            pub const ALL: [Self; [$(Self::$variant),+].len()] = [$(Self::$variant),+];

            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $string),+
                }
            }

            /// The fixed English guidance. Text stored with a problem is
            /// never used in its place.
            pub const fn guidance(self) -> &'static str {
                match self {
                    $(Self::$variant => $guidance),+
                }
            }

            /// The code string the index stores for this problem, or `None`
            /// for a code that only a read produces.
            pub const fn stored(self) -> Option<&'static str> {
                match self {
                    $(Self::$variant => $stored),+
                }
            }
        }
    };
}

problem_codes! {
    InvalidPath => "invalid_path", Some("invalid-path"),
        "Move the file to a canonical Manyhands path.";
    MissingFrontMatter => "missing_front_matter", Some("missing-front-matter"),
        "Add YAML front matter to the file.";
    MalformedFrontMatter => "malformed_front_matter", Some("malformed-front-matter"),
        "Correct the YAML front matter.";
    MalformedConfiguration => "malformed_configuration", Some("malformed-configuration"),
        "Correct the Manyhands configuration file.";
    MissingField => "missing_field", Some("missing-field"),
        "Add the required front matter field.";
    InvalidField => "invalid_field", Some("invalid-field"),
        "Correct the invalid front matter field.";
    KindPathMismatch => "kind_path_mismatch", Some("kind-path-mismatch"),
        "Make the item kind agree with the directory that holds the file.";
    DuplicateId => "duplicate_id", Some("duplicate-id"),
        "Give each item its own ID.";
    MissingCommentItem => "missing_comment_item", Some("missing-comment-item"),
        "Restore the item the comment belongs to, or remove the comment.";
    MissingParent => "missing_parent", Some("missing-parent"),
        "Restore the parent comment, or remove the reply.";
    CrossItemParent => "cross_item_parent", Some("cross-item-parent"),
        "Make the reply name a parent comment on the same item.";
    CommentCycle => "comment_cycle", Some("comment-cycle"),
        "Break the cycle between the comments' parents.";
    SourceUnreadable => "source_unreadable", Some("source"),
        "Make the file readable, then refresh the index.";
    ContextProblem => "context_problem", Some("context"),
        "Repair the item's branch or worktree, then refresh the index.";
    BranchProblem => "branch_problem", Some("branch"),
        "Restore the primary branch, then refresh the index.";
    RetryRequired => "retry_required", Some("retry-required"),
        "The repository changed while it was being observed; refresh the index again.";
    RelationshipWrongType => "relationship_wrong_type", Some("relationship-wrong-type"),
        "Write deps as a list of ticket IDs and parent as one ticket ID.";
    RelationshipInvalidId => "relationship_invalid_id", Some("relationship-invalid-id"),
        "Use the full ID of a ticket in deps and parent.";
    RelationshipSelfReference => "relationship_self_reference", Some("relationship-self-reference"),
        "Remove the ticket's own ID from its deps and parent.";
    DuplicateDependency => "duplicate_dependency", Some("duplicate-dependency"),
        "List each dependency once.";
    InvalidSlug => "invalid_slug", Some("invalid-slug"),
        "Correct the short code, or remove it.";
    PathNotUtf8 => "path_not_utf8", None,
        "Rename the file so that its path is valid UTF-8.";
    MetadataNotRepresentable => "metadata_not_representable", None,
        "Rewrite the metadata so that it can be represented as JSON.";
    RelationshipNotATicket => "relationship_not_a_ticket", None,
        "Name a ticket in deps and parent, not a document or a comment.";
    DependencyCycle => "dependency_cycle", None,
        "Break the cycle between the tickets' dependencies.";
    ParentCycle => "parent_cycle", None,
        "Break the cycle between the tickets' parents.";
    UnknownProblem => "unknown_problem", None,
        "Refresh the index; if the problem remains, inspect the file.";
}

impl ProblemCode {
    /// Maps a code string stored in the index. A string this build does not
    /// recognize becomes `unknown_problem`.
    pub fn from_stored(stored: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|code| code.stored() == Some(stored))
            .unwrap_or(Self::UnknownProblem)
    }
}

impl Serialize for ProblemCode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

/// Defines `OperationFailureCode` from one list: its contract string and the
/// code string a key-material operation stores for it, if any.
macro_rules! operation_failure_codes {
    ($($variant:ident => $string:literal, $stored:expr;)+) => {
        /// A stable code for why a stored operation did not complete.
        /// `as_str` is the contract; the variant name is not.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum OperationFailureCode {
            $($variant),+
        }

        impl OperationFailureCode {
            pub const ALL: [Self; [$(Self::$variant),+].len()] = [$(Self::$variant),+];

            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $string),+
                }
            }

            /// The code string a key-material operation stores for this
            /// failure, or `None` for a code no such operation stores.
            pub const fn stored_key_material(self) -> Option<&'static str> {
                match self {
                    $(Self::$variant => $stored),+
                }
            }
        }
    };
}

operation_failure_codes! {
    RegistryUnavailable => "registry_unavailable", Some("registry-unavailable");
    SourceMissing => "source_missing", Some("source-missing");
    SourceChanged => "source_changed", Some("source-changed");
    OwnershipUnverified => "ownership_unverified", Some("ownership-unverified");
    InvalidGeneratedKey => "invalid_generated_key", Some("invalid-generated-key");
    ProtectionUnavailable => "protection_unavailable", Some("protection-unavailable");
    UnsafePath => "unsafe_path", Some("unsafe-path");
    StorageUnavailable => "storage_unavailable", Some("storage-unavailable");
    ConfigurationRequired => "configuration_required", None;
    SelectedKeyUnavailable => "selected_key_unavailable", None;
    UnlockRequired => "unlock_required", None;
    HostApprovalRequired => "host_approval_required", None;
    TransportUnavailable => "transport_unavailable", None;
    ProtocolRejected => "protocol_rejected", None;
    Cancelled => "cancelled", None;
    RepositoryUnavailable => "repository_unavailable", None;
    UnknownFailure => "unknown_failure", None;
}

impl OperationFailureCode {
    /// Maps a failure code stored with a key-material operation. A string
    /// this build does not recognize becomes `unknown_failure`.
    pub fn from_stored_key_material(stored: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|code| code.stored_key_material() == Some(stored))
            .unwrap_or(Self::UnknownFailure)
    }
}

impl Serialize for OperationFailureCode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

/// Removes what may be a credential from a remote URL.
///
/// A network `scheme://` URL loses its password, query and fragment. Its
/// user name is kept only for `ssh`, where `git@` is not a secret. The
/// scp-like form, `file:///` URLs and local paths are returned as written.
/// Input that cannot be parsed, or that may still hold a credential,
/// becomes `REDACTED` whole: a wrongly redacted location is acceptable and
/// a surviving secret is not. That includes any location without a
/// `scheme://` that holds whitespace or a control character, a local path
/// among them, and Git's remote-helper form `<transport>::<address>`, whose
/// address is handed to a program.
pub fn redact_url(url: &str) -> String {
    let redacted = match url.split_once("://") {
        Some((scheme, rest)) => redact_scheme_url(scheme, rest),
        None if has_schemeless_credential(url)
            || is_remote_helper_location(url)
            || has_whitespace_or_control(url) =>
        {
            None
        }
        None => Some(url.to_owned()),
    };
    redacted.unwrap_or_else(|| REDACTED.to_owned())
}

fn has_whitespace_or_control(text: &str) -> bool {
    text.chars()
        .any(|character| character.is_whitespace() || character.is_control())
}

/// Recognizes Git's remote-helper form, `<transport>::<address>`: a `::`
/// with no path separator before it. What follows is passed to the program
/// `git-remote-<transport>`, and for `ext` it is a command line.
fn is_remote_helper_location(url: &str) -> bool {
    url.split(['/', '\\'])
        .next()
        .is_some_and(|first| first.contains("::"))
}

fn redact_scheme_url(scheme: &str, rest: &str) -> Option<String> {
    let mut scheme_bytes = scheme.bytes();
    if !scheme_bytes.next()?.is_ascii_alphabetic()
        || !scheme_bytes.all(|byte| byte.is_ascii_alphanumeric() || b"+-.".contains(&byte))
    {
        return None;
    }
    // Without an authority a file URL is a local location, so `?` and `#`
    // are path text.
    if scheme.eq_ignore_ascii_case("file") && rest.starts_with('/') {
        return Some(format!("{scheme}://{rest}"));
    }
    if has_whitespace_or_control(rest) {
        return None;
    }

    let (authority, remainder) = rest.split_at(rest.find(['/', '?', '#']).unwrap_or(rest.len()));
    let (user_info, host_port) = match authority.rsplit_once('@') {
        Some((user_info, host_port)) => (Some(user_info), host_port),
        None => (None, authority),
    };
    // An `@` after the authority may mean a credential held an unescaped `/`,
    // `?` or `#`, so that what was parsed as the host is part of it. Only an
    // ssh URL that names its user is trusted to have an `@` in its path.
    let is_ssh = scheme.eq_ignore_ascii_case("ssh");
    if (remainder.contains('@') && !(is_ssh && user_info.is_some())) || !is_host_port(host_port) {
        return None;
    }
    let path = remainder
        .find(['?', '#'])
        .map_or(remainder, |end| &remainder[..end]);

    let user = if is_ssh {
        user_info.map(|user_info| {
            user_info
                .split_once(':')
                .map_or(user_info, |(user, _)| user)
        })
    } else {
        None
    };
    match user {
        Some("") => None,
        Some(user) => Some(format!("{scheme}://{user}@{host_port}{path}")),
        None => Some(format!("{scheme}://{host_port}{path}")),
    }
}

/// Accepts `host`, `host:port` and `[address]:port`, with a host of ASCII
/// letters, digits, `.`, `_` and `-` and a port of one or more digits, so that part of a
/// credential is not mistaken for a host.
fn is_host_port(host_port: &str) -> bool {
    let (host, port, is_address) = if let Some(bracketed) = host_port.strip_prefix('[') {
        match bracketed.split_once(']') {
            Some((address, "")) => (address, None, true),
            Some((address, after)) => match after.strip_prefix(':') {
                Some(port) => (address, Some(port), true),
                None => return false,
            },
            None => return false,
        }
    } else {
        match host_port.split_once(':') {
            Some((host, port)) => (host, Some(port), false),
            None => (host_port, None, false),
        }
    };
    let is_host_byte = |byte: u8| {
        if is_address {
            byte.is_ascii_hexdigit() || b":.".contains(&byte)
        } else {
            byte.is_ascii_alphanumeric() || b"._-".contains(&byte)
        }
    };
    !host.is_empty()
        && host.bytes().all(is_host_byte)
        && port
            .is_none_or(|port| !port.is_empty() && port.bytes().all(|byte| byte.is_ascii_digit()))
}

/// Recognizes a credential in a location that has no `scheme://`: a URL
/// that lost its scheme or a slash, such as `//user:password@host/path` or
/// `https:/user:password@host/path`, and `user:password@host:path`, which
/// is not a valid scp-like remote.
fn has_schemeless_credential(url: &str) -> bool {
    let after = if url.starts_with("//") {
        url
    } else {
        let Some((before, after)) = url.split_once(':') else {
            return false;
        };
        // A separator before the colon is a local path, and so is a drive
        // letter followed by one.
        let is_drive = before.len() == 1
            && before.bytes().all(|byte| byte.is_ascii_alphabetic())
            && after.starts_with(['/', '\\']);
        if is_drive || before.contains(['/', '\\']) {
            return false;
        }
        after
    };
    after
        .split(['/', '\\'])
        .find(|segment| !segment.is_empty())
        .is_some_and(|segment| segment.contains('@'))
}

/// RFC 3339 in UTC with second precision, or `None` for an instant RFC 3339
/// cannot express.
pub fn timestamp_string(value: OffsetDateTime) -> Option<String> {
    value
        .checked_to_offset(UtcOffset::UTC)?
        .replace_nanosecond(0)
        .ok()?
        .format(&Rfc3339)
        .ok()
}

pub fn object_id_string(oid: git2::Oid) -> String {
    oid.to_string()
}

/// A repository-relative path with forward slashes on every platform.
///
/// `None` when the path is not valid UTF-8, which is never lossily
/// converted, or when it does not name something inside the repository:
/// empty, absolute, or reaching outside through `..`.
pub fn relative_path_string(path: &Path) -> Option<String> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_str()?),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

/// An absolute path as the platform reports it, or `None` when it is not
/// valid UTF-8.
pub fn absolute_path_string(path: &Path) -> Option<String> {
    path.to_str().map(str::to_owned)
}

#[cfg(test)]
#[path = "results_tests.rs"]
mod tests;
