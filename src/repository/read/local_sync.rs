//! Content-free local recovery views and explicit immutable side reads.
use crate::repository::{
    OperationId, ReadError, RepositoryService, ResolvedRepository, local_sync,
};
use serde::{Deserialize, Serialize};

macro_rules! values {
    ($name:ident {$($variant:ident => $text:literal),+ $(,)?}) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
        pub enum $name { $(#[serde(rename=$text)] $variant),+ }
        impl $name {
            pub const ALL: [Self; [$(Self::$variant),+].len()] = [$(Self::$variant),+];
            pub const fn as_str(self) -> &'static str { match self { $(Self::$variant => $text),+ } }
        }
    };
}
values!(LocalContextSyncState {
    Prepared=>"prepared", Applying=>"applying", ConflictPending=>"conflict_pending",
    ResolutionPrepared=>"resolution_prepared", CommitPrepared=>"commit_prepared",
    Integrated=>"integrated", IndexPending=>"index_pending", Complete=>"complete",
    RecoveryRequired=>"recovery_required", Cancelled=>"cancelled", Abandoned=>"abandoned", Refused=>"refused"
});
values!(LocalContextSyncReason {
    None=>"none", CanonicalConflict=>"canonical_conflict", UnsupportedConflict=>"unsupported_conflict",
    ImmutableClosureDisagreement=>"immutable_closure_disagreement", OriginalResolutionRequired=>"original_resolution_required",
    ForeignChange=>"foreign_change", AmbiguousBackend=>"ambiguous_backend", UnrelatedHistories=>"unrelated_histories",
    IndexPending=>"index_pending", IdentityRequired=>"identity_required", Cancelled=>"cancelled",
    Abandoned=>"abandoned", AuthorityMissing=>"authority_missing"
});
values!(LocalContextSyncNextAction {
    None=>"none", Resolve=>"resolve", RetryResolution=>"retry_resolution", Resume=>"resume",
    ExternalRepair=>"external_repair", SetIdentity=>"set_identity"
});
values!(LocalContextSyncEligibility {
    EligibleCanonical=>"eligible_canonical", ExternalResolutionRequired=>"external_resolution_required"
});
values!(LocalContextSyncDataType { LocalContextSync=>"local_context_sync" });

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalContextConflictEntryDto {
    pub display_path: String,
    pub token: String,
    pub content_available: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LocalContextSyncInspectionDto {
    pub operation_id: String,
    pub kind: String,
    pub item_id: String,
    pub context_ref: String,
    pub primary_ref: String,
    pub context_parent: String,
    pub primary_parent: String,
    pub context_head: Option<String>,
    pub state: LocalContextSyncState,
    pub reason: LocalContextSyncReason,
    pub next_action: LocalContextSyncNextAction,
    pub observation: Option<String>,
    pub eligibility: Option<LocalContextSyncEligibility>,
    pub paths: Vec<LocalContextConflictEntryDto>,
    pub can_start_resolution: bool,
    pub starting_request_id: String,
    pub cancellation_request_id: String,
    pub active_resolution_request_id: Option<String>,
    pub required_request_id: Option<String>,
    pub result_oid: Option<String>,
    pub cancel_requested: bool,
    pub cancel_acknowledged: bool,
}
// Source is intentionally available only on this explicit read, with no Debug.
#[derive(Serialize)]
pub struct LocalContextConflictContentDto {
    pub operation_id: String,
    pub token: String,
    pub base: Option<String>,
    pub local: Option<String>,
    pub incoming: Option<String>,
    pub historical: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalContextSyncMutationDto {
    #[serde(rename = "type")]
    pub data_type: LocalContextSyncDataType,
    pub item_id: String,
    pub kind: String,
    pub operation_id: Option<String>,
    pub resolution_request_id: Option<String>,
    pub required_request_id: Option<String>,
    pub result_oid: Option<String>,
    pub state: LocalContextSyncState,
    pub reason: LocalContextSyncReason,
    pub next_action: LocalContextSyncNextAction,
    pub eligibility: Option<LocalContextSyncEligibility>,
    pub cancel_acknowledged: bool,
}
impl LocalContextSyncMutationDto {
    pub(crate) fn from_stored(value: &serde_json::Value) -> Option<Self> {
        let result: Self = serde_json::from_value(value.clone()).ok()?;
        if !matches!(result.kind.as_str(), "document" | "ticket")
            || serde_json::to_value(&result).ok().as_ref() != Some(value)
        {
            return None;
        }
        Some(result)
    }
}
impl RepositoryService {
    pub fn inspect_local_context_sync(
        &self,
        repo: &ResolvedRepository,
        operation_id: OperationId,
    ) -> Result<LocalContextSyncInspectionDto, ReadError> {
        local_sync::inspection(self, repo, operation_id)
    }
    pub fn read_local_context_conflict(
        &self,
        repo: &ResolvedRepository,
        operation_id: OperationId,
        token: &str,
    ) -> Result<LocalContextConflictContentDto, ReadError> {
        local_sync::content(self, repo, operation_id, token)
    }
}
