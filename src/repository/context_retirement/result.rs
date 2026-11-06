use serde::{Deserialize, Serialize};

use super::super::{RepositoryError, remote::state::recovery_required};
use crate::results::ResultCode;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetirementDisposition {
    AlreadyAbsent,
    Retired,
    Refused,
    Preserved,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetirementRemoval {
    None,
    Possible,
    Observed,
}
impl RetirementRemoval {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Possible => "possible",
            Self::Observed => "observed",
        }
    }
}

/// A closed metadata-only report. No source bytes, paths or endpoint/key data.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextRetirementReport {
    version: u8,
    #[serde(with = "code")]
    pub code: ResultCode,
    pub outcome: RetirementDisposition,
    pub worktree_removal: RetirementRemoval,
    pub branch_removal: RetirementRemoval,
    pub materialization_settled: bool,
}

impl ContextRetirementReport {
    pub(super) fn preserved(
        worktree_removal: RetirementRemoval,
        branch_removal: RetirementRemoval,
        materialization_settled: bool,
    ) -> Self {
        Self {
            version: 1,
            code: ResultCode::ExternalChange,
            outcome: RetirementDisposition::Preserved,
            worktree_removal,
            branch_removal,
            materialization_settled,
        }
    }
    pub(super) fn refused(code: ResultCode) -> Self {
        Self {
            version: 1,
            code,
            outcome: RetirementDisposition::Refused,
            worktree_removal: RetirementRemoval::None,
            branch_removal: RetirementRemoval::None,
            materialization_settled: false,
        }
    }
    pub(super) fn absent() -> Self {
        Self {
            version: 1,
            code: ResultCode::AlreadyApplied,
            outcome: RetirementDisposition::AlreadyAbsent,
            worktree_removal: RetirementRemoval::None,
            branch_removal: RetirementRemoval::None,
            materialization_settled: true,
        }
    }

    pub(super) fn retired(
        worktree_removal: RetirementRemoval,
        branch_removal: RetirementRemoval,
    ) -> Self {
        Self {
            version: 1,
            code: ResultCode::Ok,
            outcome: RetirementDisposition::Retired,
            worktree_removal,
            branch_removal,
            materialization_settled: true,
        }
    }

    pub(super) fn encode(&self) -> Result<String, RepositoryError> {
        serde_json::to_string(self).map_err(|_| recovery_required())
    }

    pub(super) fn decode(text: &str) -> Result<Self, RepositoryError> {
        let report: Self = serde_json::from_str(text).map_err(|_| recovery_required())?;
        if report.version != 1 {
            return Err(recovery_required());
        }
        Ok(report)
    }
}

mod code {
    use super::ResultCode;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(code: &ResultCode, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(code.as_str())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<ResultCode, D::Error> {
        let code = String::deserialize(deserializer)?;
        ResultCode::ALL
            .into_iter()
            .find(|value| value.as_str() == code)
            .ok_or_else(|| serde::de::Error::custom("unknown retirement result code"))
    }
}
