//! The versioned result contract shared by the Manyhands front ends.
//!
//! These types serialize; domain types do not. Field names, enumeration
//! strings and result codes are the published JSON v1 contract, so each
//! string is written out here instead of being derived from a Rust name.

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

/// A stable result code. `as_str` is the contract; the variant name is not.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ResultCode {
    Ok,
    InvalidPath,
    NotRepository,
    NotRepositoryRoot,
    BareRepository,
    RepositoryNotRegistered,
    RepositoryInaccessible,
    InvalidId,
    ItemNotFound,
    PathNotFound,
    KeyNotFound,
    PublicKeyUnavailable,
    AuthorityNotFound,
    OperationNotFound,
    IndexUnavailable,
    Busy,
    InternalError,
}

impl ResultCode {
    pub const ALL: [Self; 17] = [
        Self::Ok,
        Self::InvalidPath,
        Self::NotRepository,
        Self::NotRepositoryRoot,
        Self::BareRepository,
        Self::RepositoryNotRegistered,
        Self::RepositoryInaccessible,
        Self::InvalidId,
        Self::ItemNotFound,
        Self::PathNotFound,
        Self::KeyNotFound,
        Self::PublicKeyUnavailable,
        Self::AuthorityNotFound,
        Self::OperationNotFound,
        Self::IndexUnavailable,
        Self::Busy,
        Self::InternalError,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::InvalidPath => "invalid_path",
            Self::NotRepository => "not_repository",
            Self::NotRepositoryRoot => "not_repository_root",
            Self::BareRepository => "bare_repository",
            Self::RepositoryNotRegistered => "repository_not_registered",
            Self::RepositoryInaccessible => "repository_inaccessible",
            Self::InvalidId => "invalid_id",
            Self::ItemNotFound => "item_not_found",
            Self::PathNotFound => "path_not_found",
            Self::KeyNotFound => "key_not_found",
            Self::PublicKeyUnavailable => "public_key_unavailable",
            Self::AuthorityNotFound => "authority_not_found",
            Self::OperationNotFound => "operation_not_found",
            Self::IndexUnavailable => "index_unavailable",
            Self::Busy => "busy",
            Self::InternalError => "internal_error",
        }
    }

    /// The fixed English explanation. It never carries backend text.
    pub const fn message(self) -> &'static str {
        match self {
            Self::Ok => "The request completed.",
            Self::InvalidPath => "The path cannot be used as a target.",
            Self::NotRepository => "There is no Git repository at the path.",
            Self::NotRepositoryRoot => {
                "The path is inside a repository but is not its root or a linked worktree root."
            }
            Self::BareRepository => "The repository has no working tree.",
            Self::RepositoryNotRegistered => "The repository is not enabled in Manyhands.",
            Self::RepositoryInaccessible => "The registered repository root cannot be read.",
            Self::InvalidId => "The ID is not a canonical ULID.",
            Self::ItemNotFound => "No item with that ID was found.",
            Self::PathNotFound => "There is no canonical resource at that path.",
            Self::KeyNotFound => "No key registration has that ID.",
            Self::PublicKeyUnavailable => "The key registration has no readable public key.",
            Self::AuthorityNotFound => "No pin exists for that authority.",
            Self::OperationNotFound => "No operation has that ID.",
            Self::IndexUnavailable => "The index is degraded and must be rebuilt.",
            Self::Busy => "The repository is busy; try again.",
            Self::InternalError => "An internal error occurred.",
        }
    }

    /// `None` only for `ok`.
    pub const fn failure_class(self) -> Option<FailureClass> {
        match self {
            Self::Ok => None,
            Self::InvalidPath
            | Self::NotRepository
            | Self::NotRepositoryRoot
            | Self::BareRepository
            | Self::InvalidId
            | Self::ItemNotFound
            | Self::PathNotFound
            | Self::KeyNotFound
            | Self::AuthorityNotFound
            | Self::OperationNotFound => Some(FailureClass::Input),
            Self::RepositoryNotRegistered
            | Self::RepositoryInaccessible
            | Self::PublicKeyUnavailable
            | Self::IndexUnavailable => Some(FailureClass::Blocked),
            Self::Busy => Some(FailureClass::Transient),
            Self::InternalError => Some(FailureClass::Internal),
        }
    }

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

/// Removes what may be a credential from a remote URL.
///
/// A `scheme://` URL loses its password, query and fragment. Its user name
/// is kept only for `ssh`, where `git@` is not a secret. The scp-like form
/// and local paths are returned as written. Input that cannot be parsed
/// becomes `REDACTED` whole.
pub fn redact_url(url: &str) -> String {
    let redacted = match url.split_once("://") {
        Some((scheme, rest)) => redact_scheme_url(scheme, rest),
        None if has_scp_like_password(url) => None,
        None => Some(url.to_owned()),
    };
    redacted.unwrap_or_else(|| REDACTED.to_owned())
}

fn redact_scheme_url(scheme: &str, rest: &str) -> Option<String> {
    let mut scheme_bytes = scheme.bytes();
    if !scheme_bytes.next()?.is_ascii_alphabetic()
        || !scheme_bytes.all(|byte| byte.is_ascii_alphanumeric() || b"+-.".contains(&byte))
        || rest
            .chars()
            .any(|character| character.is_whitespace() || character.is_control())
    {
        return None;
    }

    let rest = rest.find(['?', '#']).map_or(rest, |end| &rest[..end]);
    let (authority, path) = rest.split_at(rest.find('/').unwrap_or(rest.len()));
    let (user_info, host_port) = match authority.rsplit_once('@') {
        Some((user_info, host_port)) => (Some(user_info), host_port),
        None => (None, authority),
    };
    let is_local_file =
        scheme.eq_ignore_ascii_case("file") && user_info.is_none() && host_port.is_empty();
    if !is_local_file && !is_host_port(host_port) {
        return None;
    }

    let user = if scheme.eq_ignore_ascii_case("ssh") {
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

/// Accepts `host`, `host:port` and `[address]:port` with a numeric port, so a
/// password cut short by an unescaped `/` is not mistaken for a host.
fn is_host_port(host_port: &str) -> bool {
    let (host, port) = if let Some(bracketed) = host_port.strip_prefix('[') {
        match bracketed.split_once(']') {
            Some((address, "")) => (address, None),
            Some((address, after)) => match after.strip_prefix(':') {
                Some(port) => (address, Some(port)),
                None => return false,
            },
            None => return false,
        }
    } else {
        match host_port.split_once(':') {
            Some((host, port)) => (host, Some(port)),
            None => (host_port, None),
        }
    };
    !host.is_empty() && port.is_none_or(|port| port.bytes().all(|byte| byte.is_ascii_digit()))
}

/// Recognizes `user:password@host:path`, which is not a valid scp-like
/// remote but is what a mistyped credential looks like.
fn has_scp_like_password(url: &str) -> bool {
    let Some((host, path)) = url.split_once(':') else {
        return false;
    };
    // A slash before the colon is a local path; one letter is a Windows drive.
    if host.len() <= 1 || host.contains(['/', '\\', '@']) {
        return false;
    }
    path.split('/')
        .next()
        .is_some_and(|first| first.contains('@'))
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
/// converted, or when it is not relative to the repository: absolute, or
/// reaching outside through `..`.
pub fn relative_path_string(path: &Path) -> Option<String> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_str()?),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(parts.join("/"))
}

/// An absolute path as the platform reports it, or `None` when it is not
/// valid UTF-8.
pub fn absolute_path_string(path: &Path) -> Option<String> {
    path.to_str().map(str::to_owned)
}

#[cfg(test)]
#[path = "results_tests.rs"]
mod tests;
