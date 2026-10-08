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

/// Provisional until the index-status work (Task 5), which fills
/// `refreshed_at` and reports `never_refreshed`.
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
