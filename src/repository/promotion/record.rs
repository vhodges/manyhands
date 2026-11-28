//! Parent storage only: no transport, Git effect, prompt or lease acquisition.
//! The table-agnostic parent logic is `lifecycle_record`'s, shared with the
//! ticket closure table; this module keeps the promotion schema, step graph,
//! invariants, insert and save.
#![allow(dead_code)] // The approved Task 6 runner composes these transaction-scoped seams.
use super::lifecycle::{LifecycleKind, RecordPath};
use super::lifecycle_record::{self as parent, Blocker, ParentRecord, categories};
use super::{PromotionBlocker, PromotionExpectation};
use crate::{
    canonical::{Integration, ItemId},
    repository::{
        OperationId, RepositoryError, RepositoryService,
        cleanup::RetirementEvidence,
        pull_request_links,
        remote::{reservation::RemoteReservation, state},
    },
};
use git2::Oid;
use rusqlite::{Connection, Transaction, params};

pub(in super::super) use super::lifecycle_record::{
    ChildExecution, ChildRole, ChildState, validate_poll_identity, validate_remote_identity,
};

const KIND: LifecycleKind = LifecycleKind::DocumentPromotion;
categories!(PromotionStep {
    Confirmed=>"confirmed", ContextSynchronized=>"context_synchronized", Integrating=>"integrating",
    Integrated=>"integrated", Published=>"published", PublicationPending=>"publication_pending",
    RemoteDeleting=>"remote_deleting", RemoteDeleted=>"remote_deleted", WorktreeRemoving=>"worktree_removing", WorktreeRemoved=>"worktree_removed",
    BranchDeleted=>"branch_deleted", Complete=>"complete", Abandoned=>"abandoned", Superseded=>"superseded"
});
impl PromotionStep {
    pub(in super::super) fn terminal(self) -> bool {
        matches!(self, Self::Complete | Self::Abandoned | Self::Superseded)
    }
    /// A cleanup record's graph (R-28), which `parent::cleanup_permits` holds
    /// for both tables.
    pub(in super::super) fn cleanup_permits(self, next: Self, remote: bool) -> bool {
        parent::cleanup_permits(self.into(), next.into(), remote)
    }
    /// The step graph of a record in `mode`. A pull-request promotion
    /// synchronizes its context and finishes: it never integrates, publishes
    /// a primary or cleans up, so it has no other step.
    fn permits(self, next: Self, remote: bool, mode: Integration) -> bool {
        use PromotionStep::*;
        if self.terminal() {
            return false;
        }
        if mode == Integration::PullRequest {
            return self == next
                || matches!(
                    (self, next),
                    (Confirmed, ContextSynchronized)
                        | (ContextSynchronized, Complete)
                        | (Confirmed, Abandoned)
                );
        }
        self == next
            || matches!(
                (self, next),
                (Confirmed, ContextSynchronized)
                    | (ContextSynchronized, Integrating)
                    | (Integrating, Integrated)
                    | (Integrated, Published | PublicationPending)
                    | (PublicationPending, Published)
                    | (Published, RemoteDeleted)
                    | (RemoteDeleted, WorktreeRemoving)
                    | (WorktreeRemoving, WorktreeRemoved)
                    | (WorktreeRemoved, BranchDeleted)
                    | (Confirmed | ContextSynchronized, Abandoned)
            )
            || (!remote && matches!((self, next), (PublicationPending, WorktreeRemoving)))
    }
}
categories!(IntegrationKind { Merged=>"merged", AlreadyContained=>"already_contained" });
categories!(PublicationCategory {
    Published=>"published", AlreadyCurrent=>"already_current", NoPublicationRemote=>"no_publication_remote",
    Conflict=>"conflict", Rejected=>"rejected", Transport=>"transport", Identity=>"identity", Busy=>"busy", Yielding=>"yielding", Cancelled=>"cancelled", RecoveryRequired=>"recovery_required"
});
categories!(CleanupCategory {
    AwaitingPublication=>"awaiting_publication", ContextChanged=>"context_changed", IgnoredFiles=>"ignored_files", RemoteContextChanged=>"remote_context_changed",
    RemoteDeletionTransport=>"remote_deletion_transport", RemoteDeletionRejected=>"remote_deletion_rejected",
    RemoteDeletionConfiguration=>"remote_deletion_configuration", RemoteDeletionKey=>"remote_deletion_key", RemoteDeletionUnlock=>"remote_deletion_unlock", RemoteDeletionHost=>"remote_deletion_host",
    RecoveryRequired=>"recovery_required", Cancelled=>"cancelled"
});
impl CleanupCategory {
    pub(in super::super) fn supersedable(self) -> bool {
        matches!(
            self,
            Self::ContextChanged | Self::IgnoredFiles | Self::RemoteContextChanged
        )
    }
}

// How a finished pull-request promotion answers, stored as its `answer_code`:
// the first three when its context child completed (`complete`), the rest when
// it ended (`abandoned`); `primary_holds` goes with both.
categories!(PromotionAnswer {
    Promoted=>"promoted", AlreadyPromoted=>"already_promoted", PrimaryHolds=>"primary_holds",
    RemoteBranchDeleted=>"remote_branch_deleted", HistoryUnresolved=>"history_unresolved", ContextGone=>"context_gone"
});
// What a promotion record does: `cleanup` is `context cleanup`'s record, which
// is never superseded and runs no integration, publication or child.
categories!(PromotionAction { Promote=>"promote", Cleanup=>"cleanup" });
// What a cleanup record found of the remote context branch when it deleted it,
// stored with the step that follows, and what its answer and replay are built
// from (R-42). `none` is no delete to make: no remote, or no `R`.
categories!(RemoteBranchOutcome { Deleted=>"deleted", Absent=>"absent", Unneeded=>"none" });
// Whether the document had a link row when the pull-request promotion was
// consented to.
categories!(PromotionSituation { First=>"first", Again=>"again" });

pub(in super::super) type Refusal = parent::Refusal<PromotionBlocker>;
const BLOCKERS: [PromotionBlocker; 17] = [
    PromotionBlocker::NotADocument,
    PromotionBlocker::ContextMissing,
    PromotionBlocker::ContextNonconforming,
    PromotionBlocker::ContextDirty,
    PromotionBlocker::ContextConflicted,
    PromotionBlocker::ConflictPending,
    PromotionBlocker::PrimaryMissing,
    PromotionBlocker::PrimaryNotCheckedOut,
    PromotionBlocker::PrimaryDirty,
    PromotionBlocker::PrimaryConflicted,
    PromotionBlocker::PrimaryAheadOfRemote,
    PromotionBlocker::ContextBehindRemote,
    PromotionBlocker::IntegrationConflict,
    PromotionBlocker::RemoteContextDeleted,
    PromotionBlocker::HistoryUnknown,
    PromotionBlocker::DocumentInvalidAfterIntegration,
    PromotionBlocker::HistoryUnresolved,
];
impl Blocker for PromotionBlocker {
    const ORDER: &'static [Self] = &BLOCKERS;
    const INTEGRATION_CONFLICT: Self = PromotionBlocker::IntegrationConflict;
}
#[derive(Clone, PartialEq, Eq)]
pub(in super::super) struct PromotionRecord {
    pub repository_id: i64,
    pub operation_id: OperationId,
    pub item_id: ItemId,
    pub expectation: PromotionExpectation,
    pub remote_name: Option<String>,
    pub step: PromotionStep,
    pub context_attempt: u32,
    pub context_sync_id: Option<OperationId>,
    pub primary_attempt: u32,
    pub primary_sync_id: Option<OperationId>,
    pub base_oid: Option<Oid>,
    pub context_oid: Option<Oid>,
    pub integration_oid: Option<Oid>,
    pub integration_kind: Option<IntegrationKind>,
    pub remote_context_oid: Option<Oid>,
    pub publication: Option<PublicationCategory>,
    pub cleanup: Option<CleanupCategory>,
    pub cancel_requested: bool,
    pub updated_at: i64,
    pub refusal: Option<Refusal>,
    /// The mode the record began in, fixed for its life. A pull-request
    /// record has no integration, publication of the primary or cleanup.
    pub integration_mode: Integration,
    /// How a finished pull-request record answers, stored with its last step.
    pub answer_code: Option<PromotionAnswer>,
    /// The context head the pull-request observation's digest bound at
    /// confirm. With the three that follow, set at begin and immutable; all
    /// four are null only on a refused record written at confirm, and on a
    /// direct record.
    pub consented_context_oid: Option<Oid>,
    pub situation: Option<PromotionSituation>,
    /// The configured primary branch at begin.
    pub primary_branch: Option<String>,
    /// `endpoint_digest_of` the publication remote at begin.
    pub endpoint_digest: Option<[u8; 32]>,
    /// The head the record's next attempt of its context child may start
    /// from, once the re-run table found the branch moved only by the
    /// record's own child (RR1). Null means the consented tip.
    pub accepted_context_oid: Option<Oid>,
    /// An attempt of the context child may have pushed (RR3). Once set, it
    /// stays set.
    pub context_pushed: bool,
    /// `promote` for every record but a `context cleanup`'s, which has its own
    /// path whatever mode it recorded.
    pub action: PromotionAction,
    /// A cleanup record's digest of the `unheld` list its consent bound; its
    /// `context_oid` is `L`, its `remote_context_oid` is `R`, and its
    /// `primary_branch` and `endpoint_digest` are what it began with. All
    /// immutable from insert.
    pub unheld_digest: Option<[u8; 32]>,
    /// A cleanup record's outcome of the remote delete, stored once, with the
    /// step after it.
    pub remote_branch: Option<RemoteBranchOutcome>,
}
impl PromotionRecord {
    /// A cleanup record has its own path. A pull-request record takes the
    /// branch path: it publishes its context and never integrates.
    pub(in super::super) fn path(&self) -> RecordPath {
        if self.action == PromotionAction::Cleanup {
            return RecordPath::Cleanup;
        }
        match self.integration_mode {
            Integration::Direct => RecordPath::Direct,
            Integration::PullRequest => RecordPath::Branch,
        }
    }
    /// A finished pull-request record has its answer stored, and answers
    /// from it.
    pub(in super::super) fn answered(&self) -> bool {
        self.answer_code.is_some()
    }
}
impl std::fmt::Debug for PromotionRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PromotionRecord")
            .field("operation_id", &self.operation_id)
            .field("item_id", &self.item_id)
            .field("step", &self.step)
            .field("context_attempt", &self.context_attempt)
            .field("primary_attempt", &self.primary_attempt)
            .field("integration_oid", &self.integration_oid)
            .field("publication", &self.publication)
            .field("cleanup", &self.cleanup)
            .field("refusal", &self.refusal)
            .field("integration_mode", &self.integration_mode)
            .field("answer_code", &self.answer_code)
            .field("situation", &self.situation)
            .field("context_pushed", &self.context_pushed)
            .field("action", &self.action)
            .field("remote_branch", &self.remote_branch)
            .finish()
    }
}
/// The table parents are kept in.
pub(in super::super) const TABLE: &str = "document_promotions";
/// The table as the `promotion_refusal_blockers_v2` rebuild left it, frozen: the
/// modes rebuild renames it away, and `V2_COLUMNS` are its columns.
const SCHEMA_V2:&str="CREATE TABLE document_promotions (
    repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    operation_id TEXT NOT NULL UNIQUE CHECK(length(operation_id)=26),
    item_id TEXT NOT NULL CHECK(length(item_id)=26),
    expectation BLOB NOT NULL CHECK(typeof(expectation)='blob' AND length(expectation)=32),
    remote_name TEXT,
    step TEXT NOT NULL CHECK(step IN ('confirmed','context_synchronized','integrating','integrated','published','publication_pending','remote_deleted','worktree_removing','worktree_removed','branch_deleted','complete','abandoned','superseded')),
    context_attempt INTEGER NOT NULL DEFAULT 0 CHECK(context_attempt BETWEEN 0 AND 4294967295),
    context_sync_id TEXT UNIQUE CHECK(length(context_sync_id)=26),
    primary_attempt INTEGER NOT NULL DEFAULT 0 CHECK(primary_attempt BETWEEN 0 AND 4294967295),
    primary_sync_id TEXT UNIQUE CHECK(length(primary_sync_id)=26),
    base_oid TEXT CHECK(length(base_oid)=40 AND base_oid NOT GLOB '*[^0-9a-f]*' AND base_oid!='0000000000000000000000000000000000000000'),
    context_oid TEXT CHECK(length(context_oid)=40 AND context_oid NOT GLOB '*[^0-9a-f]*' AND context_oid!='0000000000000000000000000000000000000000'),
    integration_oid TEXT CHECK(length(integration_oid)=40 AND integration_oid NOT GLOB '*[^0-9a-f]*' AND integration_oid!='0000000000000000000000000000000000000000'),
    integration_kind TEXT CHECK(integration_kind IN ('merged','already_contained')),
    remote_context_oid TEXT CHECK(length(remote_context_oid)=40 AND remote_context_oid NOT GLOB '*[^0-9a-f]*' AND remote_context_oid!='0000000000000000000000000000000000000000'),
    publication_category TEXT CHECK(publication_category IN ('published','already_current','no_publication_remote','conflict','rejected','transport','identity','busy','yielding','cancelled','recovery_required')),
    cleanup_category TEXT CHECK(cleanup_category IN ('awaiting_publication','context_changed','ignored_files','remote_context_changed','remote_deletion_transport','remote_deletion_rejected','remote_deletion_configuration','remote_deletion_key','remote_deletion_unlock','remote_deletion_host','recovery_required','cancelled')),
    cancel_requested INTEGER NOT NULL DEFAULT 0 CHECK(cancel_requested IN (0,1)),
    updated_at INTEGER NOT NULL CHECK(updated_at>=0),
    refusal_category TEXT CHECK(refusal_category IN ('blocked','external_change','reconfirmation_required')),
    refusal_blockers INTEGER CHECK(refusal_blockers BETWEEN 1 AND 131071),
    CHECK((context_attempt=0)=(context_sync_id IS NULL)),
    CHECK((primary_attempt=0)=(primary_sync_id IS NULL)),
    CHECK(context_sync_id IS NULL OR context_sync_id!=operation_id),
    CHECK(primary_sync_id IS NULL OR (primary_sync_id!=operation_id AND primary_sync_id IS NOT context_sync_id)),
    CHECK((step='abandoned')=(refusal_category IS NOT NULL)),
    CHECK((refusal_category IS 'blocked')=(refusal_blockers IS NOT NULL))
)";
/// The table as the `wave_06_promotion_modes_v1` rebuild left it, frozen: the
/// cleanup rebuild renames it away, and `V3_COLUMNS` are its columns.
const SCHEMA_V3:&str="CREATE TABLE document_promotions (
    repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    operation_id TEXT NOT NULL UNIQUE CHECK(length(operation_id)=26),
    item_id TEXT NOT NULL CHECK(length(item_id)=26),
    expectation BLOB NOT NULL CHECK(typeof(expectation)='blob' AND length(expectation)=32),
    remote_name TEXT,
    step TEXT NOT NULL CHECK(step IN ('confirmed','context_synchronized','integrating','integrated','published','publication_pending','remote_deleted','worktree_removing','worktree_removed','branch_deleted','complete','abandoned','superseded')),
    context_attempt INTEGER NOT NULL DEFAULT 0 CHECK(context_attempt BETWEEN 0 AND 4294967295),
    context_sync_id TEXT UNIQUE CHECK(length(context_sync_id)=26),
    primary_attempt INTEGER NOT NULL DEFAULT 0 CHECK(primary_attempt BETWEEN 0 AND 4294967295),
    primary_sync_id TEXT UNIQUE CHECK(length(primary_sync_id)=26),
    base_oid TEXT CHECK(length(base_oid)=40 AND base_oid NOT GLOB '*[^0-9a-f]*' AND base_oid!='0000000000000000000000000000000000000000'),
    context_oid TEXT CHECK(length(context_oid)=40 AND context_oid NOT GLOB '*[^0-9a-f]*' AND context_oid!='0000000000000000000000000000000000000000'),
    integration_oid TEXT CHECK(length(integration_oid)=40 AND integration_oid NOT GLOB '*[^0-9a-f]*' AND integration_oid!='0000000000000000000000000000000000000000'),
    integration_kind TEXT CHECK(integration_kind IN ('merged','already_contained')),
    remote_context_oid TEXT CHECK(length(remote_context_oid)=40 AND remote_context_oid NOT GLOB '*[^0-9a-f]*' AND remote_context_oid!='0000000000000000000000000000000000000000'),
    publication_category TEXT CHECK(publication_category IN ('published','already_current','no_publication_remote','conflict','rejected','transport','identity','busy','yielding','cancelled','recovery_required')),
    cleanup_category TEXT CHECK(cleanup_category IN ('awaiting_publication','context_changed','ignored_files','remote_context_changed','remote_deletion_transport','remote_deletion_rejected','remote_deletion_configuration','remote_deletion_key','remote_deletion_unlock','remote_deletion_host','recovery_required','cancelled')),
    cancel_requested INTEGER NOT NULL DEFAULT 0 CHECK(cancel_requested IN (0,1)),
    updated_at INTEGER NOT NULL CHECK(updated_at>=0),
    refusal_category TEXT CHECK(refusal_category IN ('blocked','external_change','reconfirmation_required')),
    refusal_blockers INTEGER CHECK(refusal_blockers BETWEEN 1 AND 131071),
    integration_mode TEXT NOT NULL DEFAULT 'direct' CHECK(integration_mode IN ('direct','pull_request')),
    answer_code TEXT CHECK(answer_code IN ('promoted','already_promoted','primary_holds','remote_branch_deleted','history_unresolved','context_gone')),
    consented_context_oid TEXT CHECK(length(consented_context_oid)=40 AND consented_context_oid NOT GLOB '*[^0-9a-f]*' AND consented_context_oid!='0000000000000000000000000000000000000000'),
    situation TEXT CHECK(situation IN ('first','again')),
    primary_branch TEXT CHECK(primary_branch IS NULL OR (typeof(primary_branch)='text' AND length(primary_branch) BETWEEN 1 AND 255)),
    endpoint_digest BLOB CHECK(endpoint_digest IS NULL OR (typeof(endpoint_digest)='blob' AND length(endpoint_digest)=32)),
    accepted_context_oid TEXT CHECK(length(accepted_context_oid)=40 AND accepted_context_oid NOT GLOB '*[^0-9a-f]*' AND accepted_context_oid!='0000000000000000000000000000000000000000'),
    context_pushed INTEGER NOT NULL DEFAULT 0 CHECK(context_pushed IN (0,1)),
    CHECK((context_attempt=0)=(context_sync_id IS NULL)),
    CHECK((primary_attempt=0)=(primary_sync_id IS NULL)),
    CHECK(context_sync_id IS NULL OR context_sync_id!=operation_id),
    CHECK(primary_sync_id IS NULL OR (primary_sync_id!=operation_id AND primary_sync_id IS NOT context_sync_id)),
    CHECK((step='abandoned')=(refusal_category IS NOT NULL)),
    CHECK((refusal_category IS 'blocked')=(refusal_blockers IS NOT NULL)),
    CHECK((consented_context_oid IS NULL)=(situation IS NULL) AND (consented_context_oid IS NULL)=(primary_branch IS NULL) AND (consented_context_oid IS NULL)=(endpoint_digest IS NULL)),
    CHECK(integration_mode='pull_request' OR (consented_context_oid IS NULL AND answer_code IS NULL)),
    CHECK(integration_mode='direct' OR step='abandoned' OR consented_context_oid IS NOT NULL),
    CHECK(integration_mode='direct' OR (base_oid IS NULL AND integration_oid IS NULL AND integration_kind IS NULL AND cleanup_category IS NULL AND primary_attempt=0 AND step NOT IN ('integrating','integrated','published','publication_pending','remote_deleted','worktree_removing','worktree_removed','branch_deleted','superseded'))),
    CHECK(answer_code IS NULL OR step IN ('complete','abandoned')),
    CHECK(answer_code IS NULL OR step='complete' OR refusal_category='reconfirmation_required'),
    CHECK(answer_code IS NULL OR consented_context_oid IS NOT NULL),
    CHECK(step!='complete' OR integration_mode='direct' OR (answer_code IS NOT NULL AND publication_category IS NOT NULL AND publication_category IN ('published','already_current'))),
    CHECK(consented_context_oid IS NOT NULL OR (accepted_context_oid IS NULL AND context_pushed=0))
)";
const SCHEMA:&str="CREATE TABLE document_promotions (
    repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    operation_id TEXT NOT NULL UNIQUE CHECK(length(operation_id)=26),
    item_id TEXT NOT NULL CHECK(length(item_id)=26),
    expectation BLOB NOT NULL CHECK(typeof(expectation)='blob' AND length(expectation)=32),
    remote_name TEXT,
    step TEXT NOT NULL CHECK(step IN ('confirmed','context_synchronized','integrating','integrated','published','publication_pending','remote_deleting','remote_deleted','worktree_removing','worktree_removed','branch_deleted','complete','abandoned','superseded')),
    context_attempt INTEGER NOT NULL DEFAULT 0 CHECK(context_attempt BETWEEN 0 AND 4294967295),
    context_sync_id TEXT UNIQUE CHECK(length(context_sync_id)=26),
    primary_attempt INTEGER NOT NULL DEFAULT 0 CHECK(primary_attempt BETWEEN 0 AND 4294967295),
    primary_sync_id TEXT UNIQUE CHECK(length(primary_sync_id)=26),
    base_oid TEXT CHECK(length(base_oid)=40 AND base_oid NOT GLOB '*[^0-9a-f]*' AND base_oid!='0000000000000000000000000000000000000000'),
    context_oid TEXT CHECK(length(context_oid)=40 AND context_oid NOT GLOB '*[^0-9a-f]*' AND context_oid!='0000000000000000000000000000000000000000'),
    integration_oid TEXT CHECK(length(integration_oid)=40 AND integration_oid NOT GLOB '*[^0-9a-f]*' AND integration_oid!='0000000000000000000000000000000000000000'),
    integration_kind TEXT CHECK(integration_kind IN ('merged','already_contained')),
    remote_context_oid TEXT CHECK(length(remote_context_oid)=40 AND remote_context_oid NOT GLOB '*[^0-9a-f]*' AND remote_context_oid!='0000000000000000000000000000000000000000'),
    publication_category TEXT CHECK(publication_category IN ('published','already_current','no_publication_remote','conflict','rejected','transport','identity','busy','yielding','cancelled','recovery_required')),
    cleanup_category TEXT CHECK(cleanup_category IN ('awaiting_publication','context_changed','ignored_files','remote_context_changed','remote_deletion_transport','remote_deletion_rejected','remote_deletion_configuration','remote_deletion_key','remote_deletion_unlock','remote_deletion_host','recovery_required','cancelled')),
    cancel_requested INTEGER NOT NULL DEFAULT 0 CHECK(cancel_requested IN (0,1)),
    updated_at INTEGER NOT NULL CHECK(updated_at>=0),
    refusal_category TEXT CHECK(refusal_category IN ('blocked','external_change','reconfirmation_required')),
    refusal_blockers INTEGER CHECK(refusal_blockers BETWEEN 1 AND 131071),
    integration_mode TEXT NOT NULL DEFAULT 'direct' CHECK(integration_mode IN ('direct','pull_request')),
    answer_code TEXT CHECK(answer_code IN ('promoted','already_promoted','primary_holds','remote_branch_deleted','history_unresolved','context_gone')),
    consented_context_oid TEXT CHECK(length(consented_context_oid)=40 AND consented_context_oid NOT GLOB '*[^0-9a-f]*' AND consented_context_oid!='0000000000000000000000000000000000000000'),
    situation TEXT CHECK(situation IN ('first','again')),
    primary_branch TEXT CHECK(primary_branch IS NULL OR (typeof(primary_branch)='text' AND length(primary_branch) BETWEEN 1 AND 255)),
    endpoint_digest BLOB CHECK(endpoint_digest IS NULL OR (typeof(endpoint_digest)='blob' AND length(endpoint_digest)=32)),
    accepted_context_oid TEXT CHECK(length(accepted_context_oid)=40 AND accepted_context_oid NOT GLOB '*[^0-9a-f]*' AND accepted_context_oid!='0000000000000000000000000000000000000000'),
    context_pushed INTEGER NOT NULL DEFAULT 0 CHECK(context_pushed IN (0,1)),
    action TEXT NOT NULL DEFAULT 'promote' CHECK(action IN ('promote','cleanup')),
    unheld_digest BLOB CHECK(unheld_digest IS NULL OR (typeof(unheld_digest)='blob' AND length(unheld_digest)=32)),
    remote_branch TEXT CHECK(remote_branch IS NULL OR remote_branch IN ('deleted','absent','none')),
    CHECK((context_attempt=0)=(context_sync_id IS NULL)),
    CHECK((primary_attempt=0)=(primary_sync_id IS NULL)),
    CHECK(context_sync_id IS NULL OR context_sync_id!=operation_id),
    CHECK(primary_sync_id IS NULL OR (primary_sync_id!=operation_id AND primary_sync_id IS NOT context_sync_id)),
    CHECK((step='abandoned')=(refusal_category IS NOT NULL)),
    CHECK((refusal_category IS 'blocked')=(refusal_blockers IS NOT NULL)),
    CHECK(action='cleanup' OR ((consented_context_oid IS NULL)=(situation IS NULL) AND (consented_context_oid IS NULL)=(primary_branch IS NULL) AND (consented_context_oid IS NULL)=(endpoint_digest IS NULL))),
    CHECK(integration_mode='pull_request' OR (consented_context_oid IS NULL AND answer_code IS NULL)),
    CHECK(integration_mode='direct' OR action='cleanup' OR step='abandoned' OR consented_context_oid IS NOT NULL),
    CHECK(integration_mode='direct' OR action='cleanup' OR (base_oid IS NULL AND integration_oid IS NULL AND integration_kind IS NULL AND cleanup_category IS NULL AND primary_attempt=0 AND step NOT IN ('integrating','integrated','published','publication_pending','remote_deleted','worktree_removing','worktree_removed','branch_deleted','superseded'))),
    CHECK(answer_code IS NULL OR step IN ('complete','abandoned')),
    CHECK(answer_code IS NULL OR step='complete' OR refusal_category='reconfirmation_required'),
    CHECK(answer_code IS NULL OR consented_context_oid IS NOT NULL),
    CHECK(step!='complete' OR integration_mode='direct' OR action='cleanup' OR (answer_code IS NOT NULL AND publication_category IS NOT NULL AND publication_category IN ('published','already_current'))),
    CHECK(consented_context_oid IS NOT NULL OR (accepted_context_oid IS NULL AND context_pushed=0)),
    CHECK(action!='cleanup' OR (base_oid IS NULL AND integration_oid IS NULL AND integration_kind IS NULL AND publication_category IS NULL AND answer_code IS NULL AND consented_context_oid IS NULL AND situation IS NULL AND accepted_context_oid IS NULL AND context_pushed=0 AND context_attempt=0 AND primary_attempt=0 AND context_oid IS NOT NULL AND unheld_digest IS NOT NULL AND primary_branch IS NOT NULL AND (endpoint_digest IS NULL)=(remote_name IS NULL) AND step NOT IN ('context_synchronized','integrating','integrated','published','publication_pending','superseded'))),
    CHECK(action='cleanup' OR (unheld_digest IS NULL AND remote_branch IS NULL AND step!='remote_deleting')),
    CHECK(action!='cleanup' OR (remote_branch IS NOT NULL)=(step IN ('remote_deleted','worktree_removing','worktree_removed','branch_deleted','complete'))),
    CHECK(remote_branch IS NULL OR (remote_branch='none')=(remote_context_oid IS NULL)),
    CHECK(step!='remote_deleting' OR remote_context_oid IS NOT NULL),
    CHECK(action!='cleanup' OR refusal_category IS NULL OR refusal_category='external_change')
)";
const INDEX: &str = "CREATE UNIQUE INDEX document_promotions_unfinished ON document_promotions(repository_id,item_id) WHERE step NOT IN ('complete','abandoned','superseded')";
const BLOCKERS_MARKER: &str = "promotion_refusal_blockers_v2";
const MODES_MARKER: &str = "wave_06_promotion_modes_v1";
const CLEANUP_MARKER: &str = "wave_06_promotion_cleanup_v1";
/// The 21 columns `SCHEMA_V2` has, in table order: what a table that
/// predates the mode carries into the rebuild.
const V2_COLUMNS: &str = "repository_id,operation_id,item_id,expectation,remote_name,step,context_attempt,context_sync_id,primary_attempt,primary_sync_id,base_oid,context_oid,integration_oid,integration_kind,remote_context_oid,publication_category,cleanup_category,cancel_requested,updated_at,refusal_category,refusal_blockers";
/// The 29 columns `SCHEMA_V3` has, in table order: what a table that predates
/// the cleanup action carries into the rebuild.
const V3_COLUMNS: &str = "repository_id,operation_id,item_id,expectation,remote_name,step,context_attempt,context_sync_id,primary_attempt,primary_sync_id,base_oid,context_oid,integration_oid,integration_kind,remote_context_oid,publication_category,cleanup_category,cancel_requested,updated_at,refusal_category,refusal_blockers,integration_mode,answer_code,consented_context_oid,situation,primary_branch,endpoint_digest,accepted_context_oid,context_pushed";
pub(in super::super) fn migrate(tx: &Transaction<'_>) -> Result<(), RepositoryError> {
    let mut created = false;
    let marked:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM registry_migrations WHERE name='cycle_09_document_promotions')",[],|r|r.get(0)).map_err(|_|state::recovery_required())?;
    if !marked {
        let partial:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name IN ('document_promotions','document_promotions_unfinished'))",[],|r|r.get(0)).map_err(|_|state::recovery_required())?;
        if partial {
            return Err(state::recovery_required());
        }
        tx.execute_batch(SCHEMA)
            .and_then(|()| tx.execute_batch(INDEX))
            .map_err(|_| state::recovery_required())?;
        tx.execute(
            "INSERT INTO registry_migrations(name) VALUES('cycle_09_document_promotions')",
            [],
        )
        .map_err(|_| state::recovery_required())?;
        created = true;
    }
    let widened: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM registry_migrations WHERE name=?1)",
            [BLOCKERS_MARKER],
            |r| r.get(0),
        )
        .map_err(|_| state::recovery_required())?;
    if !widened {
        // The refusal mask CHECK gained a bit; rebuild a table that predates
        // it. The index goes first so the rename cannot carry its name along.
        if !created {
            tx.execute_batch(&format!("DROP INDEX document_promotions_unfinished; ALTER TABLE document_promotions RENAME TO document_promotions_before_blockers; {SCHEMA_V2}; {INDEX}; INSERT INTO document_promotions SELECT * FROM document_promotions_before_blockers; DROP TABLE document_promotions_before_blockers;")).map_err(|_| state::recovery_required())?;
        }
        tx.execute(
            "INSERT INTO registry_migrations(name) VALUES(?1)",
            [BLOCKERS_MARKER],
        )
        .map_err(|_| state::recovery_required())?;
    }
    let moded: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM registry_migrations WHERE name=?1)",
            [MODES_MARKER],
            |r| r.get(0),
        )
        .map_err(|_| state::recovery_required())?;
    if !moded {
        // The mode and the pull-request columns: rebuild a table that
        // predates them. Its rows read `direct`, the column's default, with
        // the rest null; the index goes first, and the rows are copied in
        // rowid order, the order the records were made in.
        if !created {
            tx.execute_batch(&format!("DROP INDEX document_promotions_unfinished; ALTER TABLE document_promotions RENAME TO document_promotions_before_modes; {SCHEMA_V3}; {INDEX}; INSERT INTO document_promotions({V2_COLUMNS}) SELECT {V2_COLUMNS} FROM document_promotions_before_modes ORDER BY rowid; DROP TABLE document_promotions_before_modes;")).map_err(|_| state::recovery_required())?;
        }
        tx.execute(
            "INSERT INTO registry_migrations(name) VALUES(?1)",
            [MODES_MARKER],
        )
        .map_err(|_| state::recovery_required())?;
    }
    let cleaned: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM registry_migrations WHERE name=?1)",
            [CLEANUP_MARKER],
            |r| r.get(0),
        )
        .map_err(|_| state::recovery_required())?;
    if !cleaned {
        // The cleanup action, step and columns: rebuild a table that predates
        // them. Its rows read `promote`, the column's default, with the rest
        // null; the index goes first, and the rows are copied in rowid order.
        if !created {
            tx.execute_batch(&format!("DROP INDEX document_promotions_unfinished; ALTER TABLE document_promotions RENAME TO document_promotions_before_cleanup; {SCHEMA}; {INDEX}; INSERT INTO document_promotions({V3_COLUMNS}) SELECT {V3_COLUMNS} FROM document_promotions_before_cleanup ORDER BY rowid; DROP TABLE document_promotions_before_cleanup;")).map_err(|_| state::recovery_required())?;
        }
        tx.execute(
            "INSERT INTO registry_migrations(name) VALUES(?1)",
            [CLEANUP_MARKER],
        )
        .map_err(|_| state::recovery_required())?;
    }
    validate_schema(tx)
}
fn validate_schema(db: &Connection) -> Result<(), RepositoryError> {
    state::validate_schema_object(db, "TABLE", "document_promotions", SCHEMA)?;
    state::validate_schema_object(db, "INDEX", "document_promotions_unfinished", INDEX)
}
fn decode(row: &rusqlite::Row<'_>) -> Result<PromotionRecord, RepositoryError> {
    let row = parent::Row(row);
    let r = PromotionRecord {
        repository_id: row.number("repository_id")?,
        operation_id: row.operation("operation_id")?,
        item_id: row.item()?,
        expectation: PromotionExpectation::from_bytes(row.expectation()?),
        remote_name: row.opt("remote_name")?,
        step: PromotionStep::parse(&row.text("step")?)?,
        context_attempt: row.attempt("context_attempt")?,
        context_sync_id: row.sync_id("context_sync_id")?,
        primary_attempt: row.attempt("primary_attempt")?,
        primary_sync_id: row.sync_id("primary_sync_id")?,
        base_oid: row.oid("base_oid")?,
        context_oid: row.oid("context_oid")?,
        integration_oid: row.oid("integration_oid")?,
        integration_kind: row.category("integration_kind", IntegrationKind::parse)?,
        remote_context_oid: row.oid("remote_context_oid")?,
        publication: row.category("publication_category", PublicationCategory::parse)?,
        cleanup: row.category("cleanup_category", CleanupCategory::parse)?,
        cancel_requested: row.flag("cancel_requested")?,
        updated_at: row.number("updated_at")?,
        refusal: row.refusal()?,
        integration_mode: parse_mode(&row.text("integration_mode")?)?,
        answer_code: row.category("answer_code", PromotionAnswer::parse)?,
        consented_context_oid: row.oid("consented_context_oid")?,
        situation: row.category("situation", PromotionSituation::parse)?,
        primary_branch: row.opt("primary_branch")?,
        endpoint_digest: row
            .0
            .get::<_, Option<Vec<u8>>>("endpoint_digest")
            .map_err(|_| state::recovery_required())?
            .map(|digest| digest.try_into().map_err(|_| state::recovery_required()))
            .transpose()?,
        accepted_context_oid: row.oid("accepted_context_oid")?,
        context_pushed: row.flag("context_pushed")?,
        action: PromotionAction::parse(&row.text("action")?)?,
        unheld_digest: row
            .0
            .get::<_, Option<Vec<u8>>>("unheld_digest")
            .map_err(|_| state::recovery_required())?
            .map(|digest| digest.try_into().map_err(|_| state::recovery_required()))
            .transpose()?,
        remote_branch: row.category("remote_branch", RemoteBranchOutcome::parse)?,
    };
    validate(&r)?;
    Ok(r)
}
fn parse_mode(text: &str) -> Result<Integration, RepositoryError> {
    match text {
        "direct" => Ok(Integration::Direct),
        "pull_request" => Ok(Integration::PullRequest),
        _ => Err(state::recovery_required()),
    }
}
fn validate(r: &PromotionRecord) -> Result<(), RepositoryError> {
    use PromotionStep::*;
    if r.repository_id <= 0
        || time::OffsetDateTime::from_unix_timestamp(r.updated_at).is_err()
        || r.updated_at < 0
        || (r.context_attempt == 0) != r.context_sync_id.is_none()
        || (r.primary_attempt == 0) != r.primary_sync_id.is_none()
        || r.context_sync_id == Some(r.operation_id)
        || r.primary_sync_id == Some(r.operation_id)
        || r.context_sync_id.is_some() && r.context_sync_id == r.primary_sync_id
        || (r.step == Abandoned) != r.refusal.is_some()
        || [
            r.base_oid,
            r.context_oid,
            r.integration_oid,
            r.remote_context_oid,
            r.consented_context_oid,
            r.accepted_context_oid,
        ]
        .into_iter()
        .flatten()
        .any(|o| o.is_zero())
        || matches!(
            r.step,
            ContextSynchronized
                | Integrating
                | Integrated
                | Published
                | PublicationPending
                | RemoteDeleted
                | WorktreeRemoving
                | WorktreeRemoved
                | BranchDeleted
                | Complete
        ) && r.context_oid.is_none()
        || r.remote_name.is_none() && (r.remote_context_oid.is_some() || r.step == RemoteDeleted)
    {
        return Err(state::recovery_required());
    }
    match r.path() {
        RecordPath::Direct => validate_direct(r)?,
        RecordPath::Branch => validate_pull_request(r)?,
        RecordPath::Cleanup => validate_cleanup(r)?,
    }
    if let Some(name) = &r.remote_name {
        crate::repository::RemoteRefPlan::from_configuration(name, "main")
            .map_err(|_| state::recovery_required())?;
    }
    if let Some(refusal) = &r.refusal {
        refusal.encoded()?;
    }
    Ok(())
}
/// A cleanup record (R-28, the table's CHECKs read again): none of a
/// promotion's effects, the begin-time facts set and immutable, and the
/// outcome of the remote delete stored from `remote_deleted` on, `none`
/// exactly when there was no `R`.
fn validate_cleanup(r: &PromotionRecord) -> Result<(), RepositoryError> {
    use PromotionStep::*;
    if r.base_oid.is_some()
        || r.integration_oid.is_some()
        || r.integration_kind.is_some()
        || r.publication.is_some()
        || r.answer_code.is_some()
        || r.consented_context_oid.is_some()
        || r.situation.is_some()
        || r.accepted_context_oid.is_some()
        || r.context_pushed
        || r.context_attempt != 0
        || r.primary_attempt != 0
        || r.context_oid.is_none()
        || r.unheld_digest.is_none()
        || r.endpoint_digest.is_some() != r.remote_name.is_some()
        || matches!(
            r.step,
            ContextSynchronized
                | Integrating
                | Integrated
                | Published
                | PublicationPending
                | Superseded
        )
        || (parent::CleanupFacts {
            primary_branch: r.primary_branch.as_deref(),
            remote_name: r.remote_name.as_deref(),
            remote_branch: r.remote_branch,
            remote_context_oid: r.remote_context_oid,
            step: r.step.into(),
            foreign_refusal: r
                .refusal
                .as_ref()
                .is_some_and(|refusal| *refusal != Refusal::ExternalChange),
            stopped: r.cleanup.is_some(),
        })
        .broken()
    {
        return Err(state::recovery_required());
    }
    Ok(())
}
/// A direct record: the integration rules as they were, and none of a
/// pull-request record's columns.
fn validate_direct(r: &PromotionRecord) -> Result<(), RepositoryError> {
    use PromotionStep::*;
    if r.answer_code.is_some()
        || r.consented_context_oid.is_some()
        || r.situation.is_some()
        || r.primary_branch.is_some()
        || r.endpoint_digest.is_some()
        || r.accepted_context_oid.is_some()
        || r.context_pushed
        || matches!(
            r.step,
            Integrating
                | Integrated
                | Published
                | PublicationPending
                | RemoteDeleted
                | WorktreeRemoving
                | WorktreeRemoved
                | BranchDeleted
                | Complete
        ) && (r.base_oid.is_none() || r.integration_kind.is_none())
        || matches!(
            r.step,
            Integrated
                | Published
                | PublicationPending
                | RemoteDeleted
                | WorktreeRemoving
                | WorktreeRemoved
                | BranchDeleted
                | Complete
        ) && r.integration_oid.is_none()
        || matches!(
            r.step,
            Published
                | RemoteDeleted
                | WorktreeRemoving
                | WorktreeRemoved
                | BranchDeleted
                | Complete
        ) && r.remote_name.is_some()
            && !matches!(
                r.publication,
                Some(PublicationCategory::Published | PublicationCategory::AlreadyCurrent)
            )
        || r.step == PublicationPending
            && (r.publication.is_none()
                || matches!(
                    r.publication,
                    Some(PublicationCategory::Published | PublicationCategory::AlreadyCurrent)
                ))
    {
        return Err(state::recovery_required());
    }
    Ok(())
}
/// A pull-request record: it synchronizes its context and finishes with its
/// answer, and never integrates, publishes the primary or cleans up. Its four
/// begin-time columns are set from `confirmed` on, or all null on a record
/// refused at confirm.
fn validate_pull_request(r: &PromotionRecord) -> Result<(), RepositoryError> {
    use PromotionStep::*;
    let begun = r.consented_context_oid.is_some();
    if r.base_oid.is_some()
        || r.integration_oid.is_some()
        || r.integration_kind.is_some()
        || r.cleanup.is_some()
        || r.primary_sync_id.is_some()
        || matches!(
            r.step,
            Integrating
                | Integrated
                | Published
                | PublicationPending
                | RemoteDeleted
                | WorktreeRemoving
                | WorktreeRemoved
                | BranchDeleted
                | Superseded
        )
        // The four begin-time columns come together, and every unfinished or
        // finished record has them; only a refused one may have none.
        || [
            r.situation.is_some(),
            r.primary_branch.is_some(),
            r.endpoint_digest.is_some(),
        ]
        .into_iter()
        .any(|set| set != begun)
        || !begun && r.step != Abandoned
        // The re-run table's two columns belong to a record that began.
        || !begun && (r.accepted_context_oid.is_some() || r.context_pushed)
        // An answer is given by a record that began.
        || !begun && r.answer_code.is_some()
        // A record that began has the publication remote it began with.
        || r.step != Abandoned && r.remote_name.is_none()
        || r.primary_branch.as_deref().is_some_and(|branch| {
            branch.is_empty()
                || branch.len() > 255
                || r.remote_name.as_deref().is_none_or(|remote| {
                    crate::repository::RemoteRefPlan::from_configuration(remote, branch).is_err()
                })
        })
    {
        return Err(state::recovery_required());
    }
    let finished_with =
        |answers: &[PromotionAnswer]| r.answer_code.is_some_and(|a| answers.contains(&a));
    let ok = match r.step {
        Complete => {
            matches!(
                r.publication,
                Some(PublicationCategory::Published | PublicationCategory::AlreadyCurrent)
            ) && finished_with(&[
                PromotionAnswer::Promoted,
                PromotionAnswer::AlreadyPromoted,
                PromotionAnswer::PrimaryHolds,
            ])
        }
        // A refused record has its refusal and no answer; one that ended after
        // its child has an answer, with the refusal `reconfirmation_required`.
        Abandoned => {
            r.publication.is_none()
                && (r.answer_code.is_none()
                    || finished_with(&[
                        PromotionAnswer::PrimaryHolds,
                        PromotionAnswer::RemoteBranchDeleted,
                        PromotionAnswer::HistoryUnresolved,
                        PromotionAnswer::ContextGone,
                    ]) && r.refusal == Some(Refusal::ReconfirmationRequired))
        }
        _ => r.answer_code.is_none() && r.publication.is_none(),
    };
    if !ok {
        return Err(state::recovery_required());
    }
    Ok(())
}
pub(in super::super) fn read_record(
    db: &Connection,
    id: i64,
    op: OperationId,
) -> Result<Option<PromotionRecord>, RepositoryError> {
    parent::read_record(db, id, op)
}
pub(in super::super) fn records(
    db: &Connection,
    id: i64,
) -> Result<Vec<PromotionRecord>, RepositoryError> {
    parent::records(db, id)
}
/// Only a direct record's cleanup stop may be superseded: a pull-request
/// record has none, and a cleanup record never is (R-33).
fn supersedable(old: &PromotionRecord) -> bool {
    old.path() == RecordPath::Direct && old.cleanup.is_some_and(CleanupCategory::supersedable)
}
/// What a request answers when it names another verb's record: the one its
/// ID and digest found is not the one it continues.
pub(in super::super) fn action_mismatch() -> RepositoryError {
    parent::mismatch(KIND)
}
pub(in super::super) type ReplacementGuard = parent::ReplacementGuard<PromotionRecord>;
pub(in super::super) fn replacement_guard(
    tx: &Transaction<'_>,
    id: i64,
    item: &ItemId,
) -> Result<ReplacementGuard, RepositoryError> {
    parent::replacement_guard(tx, id, item)
}
/// Caller has validated new consent before this atomic supersession/insert.
pub(in super::super) fn begin_record(
    tx: &Transaction<'_>,
    id: i64,
    op: OperationId,
    item: &ItemId,
    expectation: PromotionExpectation,
    remote: Option<&str>,
) -> Result<PromotionRecord, RepositoryError> {
    if let Some(existing) = parent::existing_record(tx, id, op, item, expectation)? {
        return Ok(existing);
    }
    if let Some(old) = parent::unfinished_record::<PromotionRecord>(tx, id, item)? {
        if !supersedable(&old) {
            return Err(parent::busy(KIND));
        }
        parent::supersede(tx, id, &old)?;
    }
    insert_record(
        tx,
        id,
        op,
        item,
        expectation,
        remote,
        None,
        Integration::Direct,
        None,
    )
}
/// A pull-request record's begin-time columns, as the confirm's observation
/// gives them: the consented tip, the situation consented to, the configured
/// primary branch and the digest of the publication remote's endpoints. All
/// four are set at begin and immutable.
pub(in super::super) struct PullRequestBegin {
    pub consented: Oid,
    pub situation: PromotionSituation,
    pub primary_branch: String,
    pub endpoint_digest: [u8; 32],
}
/// The begin of a pull-request promotion: the operation's own record when it
/// has one, whatever mode it began in, else a new `confirmed` record with the
/// mode and the four columns. A pull-request record supersedes nothing: any
/// other unfinished record of the document holds it off.
pub(in super::super) fn begin_pull_request_record(
    tx: &Transaction<'_>,
    id: i64,
    op: OperationId,
    item: &ItemId,
    expectation: PromotionExpectation,
    remote: Option<&str>,
    begin: &PullRequestBegin,
) -> Result<PromotionRecord, RepositoryError> {
    if let Some(existing) = parent::existing_record(tx, id, op, item, expectation)? {
        return Ok(existing);
    }
    if parent::unfinished_record::<PromotionRecord>(tx, id, item)?.is_some() {
        return Err(parent::busy(KIND));
    }
    // A pull-request promotion publishes to its remote.
    let remote = remote.ok_or_else(state::recovery_required)?;
    insert_record(
        tx,
        id,
        op,
        item,
        expectation,
        Some(remote),
        None,
        Integration::PullRequest,
        Some(begin),
    )
}
/// A refused new request is terminal from insertion. It can coexist with the
/// original unfinished parent without superseding/fencing that parent's owner.
pub(in super::super) fn refuse_record(
    tx: &Transaction<'_>,
    id: i64,
    op: OperationId,
    item: &ItemId,
    expectation: PromotionExpectation,
    remote: Option<&str>,
    refusal: &Refusal,
) -> Result<PromotionRecord, RepositoryError> {
    refuse_record_on(
        tx,
        id,
        op,
        item,
        expectation,
        remote,
        refusal,
        Integration::Direct,
    )
}
/// `refuse_record` in the mode the refused confirm was made in: a refused
/// pull-request record has none of the four begin-time columns.
#[allow(clippy::too_many_arguments)] // `refuse_record`'s, with the mode.
pub(in super::super) fn refuse_record_on(
    tx: &Transaction<'_>,
    id: i64,
    op: OperationId,
    item: &ItemId,
    expectation: PromotionExpectation,
    remote: Option<&str>,
    refusal: &Refusal,
    mode: Integration,
) -> Result<PromotionRecord, RepositoryError> {
    if let Some(existing) = parent::existing_record(tx, id, op, item, expectation)? {
        return Ok(existing);
    }
    if parent::unfinished_record::<PromotionRecord>(tx, id, item)?
        .is_some_and(|old| !supersedable(&old))
    {
        return Err(parent::busy(KIND));
    }
    insert_record(
        tx,
        id,
        op,
        item,
        expectation,
        remote,
        Some(refusal),
        mode,
        None,
    )
}
#[allow(clippy::too_many_arguments)] // One INSERT of every column a begin or a refusal writes.
fn insert_record(
    tx: &Transaction<'_>,
    id: i64,
    op: OperationId,
    item: &ItemId,
    expectation: PromotionExpectation,
    remote: Option<&str>,
    refusal: Option<&Refusal>,
    mode: Integration,
    begin: Option<&PullRequestBegin>,
) -> Result<PromotionRecord, RepositoryError> {
    if let Some(remote) = remote {
        crate::repository::RemoteRefPlan::from_configuration(remote, "main")
            .map_err(|_| state::recovery_required())?;
    }
    let (category, blockers) = Refusal::encoded_columns(refusal)?;
    let step = if refusal.is_some() {
        "abandoned"
    } else {
        "confirmed"
    };
    tx.execute("INSERT INTO document_promotions(repository_id,operation_id,item_id,expectation,remote_name,step,updated_at,refusal_category,refusal_blockers,integration_mode,consented_context_oid,situation,primary_branch,endpoint_digest) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",params![id,op.to_string(),item.to_string(),expectation.to_bytes().as_slice(),remote,step,parent::now(),category,blockers,mode.as_str(),begin.map(|b|b.consented.to_string()),begin.map(|b|b.situation.name()),begin.map(|b|b.primary_branch.as_str()),begin.map(|b|b.endpoint_digest.as_slice())]).map_err(|_|state::recovery_required())?;
    read_record(tx, id, op)?.ok_or_else(state::recovery_required)
}
pub(in super::super) fn save_record(
    tx: &Transaction<'_>,
    id: i64,
    r: &PromotionRecord,
) -> Result<(), RepositoryError> {
    let old = read_record(tx, id, r.operation_id)?.ok_or_else(state::recovery_required)?;
    if old.repository_id != r.repository_id
        || old.item_id != r.item_id
        || old.expectation != r.expectation
        || old.remote_name != r.remote_name
        || old.context_attempt != r.context_attempt
        || old.context_sync_id != r.context_sync_id
        || old.primary_attempt != r.primary_attempt
        || old.primary_sync_id != r.primary_sync_id
    {
        return Err(parent::mismatch(KIND));
    }
    let permitted = match old.path() {
        RecordPath::Cleanup => old.step.cleanup_permits(r.step, r.remote_name.is_some()),
        RecordPath::Direct | RecordPath::Branch => {
            old.step
                .permits(r.step, r.remote_name.is_some(), old.integration_mode)
        }
    };
    if !permitted
        || old.action != r.action
        || old.unheld_digest != r.unheld_digest
        || !remote_branch_kept_or_written(&old, r)
        || old.path() == RecordPath::Cleanup && old.remote_context_oid != r.remote_context_oid
        || old.integration_mode != r.integration_mode
        || old.answer_code != r.answer_code
        || old.consented_context_oid != r.consented_context_oid
        || old.situation != r.situation
        || old.primary_branch != r.primary_branch
        || old.endpoint_digest != r.endpoint_digest
        || r.cancel_requested != old.cancel_requested
        || old.base_oid.is_some() && old.base_oid != r.base_oid
        || old.context_oid.is_some() && old.context_oid != r.context_oid
        || old.integration_oid.is_some() && old.integration_oid != r.integration_oid
        || old.integration_kind.is_some() && old.integration_kind != r.integration_kind
        || old.remote_context_oid.is_some() && old.remote_context_oid != r.remote_context_oid
    {
        return Err(state::recovery_required());
    }
    validate(r)?;
    let (refusal, mask) = Refusal::encoded_columns(r.refusal.as_ref())?;
    tx.execute("UPDATE document_promotions SET step=?2,base_oid=?3,context_oid=?4,integration_oid=?5,integration_kind=?6,remote_context_oid=?7,publication_category=?8,cleanup_category=?9,refusal_category=?10,refusal_blockers=?11,updated_at=max(updated_at,?12),remote_branch=?13 WHERE operation_id=?1",params![r.operation_id.to_string(),r.step.name(),r.base_oid.map(|o|o.to_string()),r.context_oid.map(|o|o.to_string()),r.integration_oid.map(|o|o.to_string()),r.integration_kind.map(IntegrationKind::name),r.remote_context_oid.map(|o|o.to_string()),r.publication.map(PublicationCategory::name),r.cleanup.map(CleanupCategory::name),refusal,mask,parent::now(),r.remote_branch.map(RemoteBranchOutcome::name)]).map_err(|_|state::recovery_required())?;
    Ok(())
}
/// Whether a save keeps the stored outcome of the remote delete or writes it
/// as `parent::remote_branch_kept_or_written` allows.
fn remote_branch_kept_or_written(old: &PromotionRecord, new: &PromotionRecord) -> bool {
    parent::remote_branch_kept_or_written(
        (old.remote_branch, new.remote_branch),
        (old.step.into(), new.step.into()),
        old.action == PromotionAction::Cleanup,
        new.remote_name.is_none(),
    )
}

/// What a finished pull-request promotion tells the link record, which the
/// answer pairs with: a first promote gives its link, a promote again renews
/// the row's `given_at` alone, and a document the primary holds touches
/// nothing.
pub(in super::super) enum LinkWrite {
    Untouched,
    /// A finish whose context branch is gone (RR8): the answer is given and
    /// no link row is written or renewed. The record keeps no URL (R-30).
    Withheld,
    Renew {
        given_at: i64,
    },
    Give {
        link: crate::forge::PullRequestLink,
        given_at: i64,
    },
}
/// What a pull-request promotion whose context child completed stores: its
/// answer, the child's category and the link record's write.
pub(in super::super) struct BranchFinish {
    pub answer: PromotionAnswer,
    pub category: PublicationCategory,
    pub link: LinkWrite,
}
/// Whether the context child holds the record: its row is live and holds no
/// authority, or it is retained.
fn child_holds(tx: &Transaction<'_>, r: &PromotionRecord) -> Result<bool, RepositoryError> {
    use crate::repository::RemoteOperationPhase as Phase;
    Ok(match child_state(tx, r, ChildRole::Context)? {
        ChildState::Absent | ChildState::Complete(_) => false,
        ChildState::Retained(_) => true,
        ChildState::Unowned(row) => !matches!(
            row.phase,
            Phase::Completed | Phase::Interrupted | Phase::Cancelled | Phase::Failed
        ),
    })
}
/// A pull-request promotion's completion after its context child, in the
/// caller's transaction: the answer and the child's category stored, and the
/// link record written as the answer says. Only a record at
/// `context_synchronized` completes, with an answer that goes with its link
/// write.
pub(in super::super) fn complete_branch_record(
    tx: &Transaction<'_>,
    id: i64,
    op: OperationId,
    finish: &BranchFinish,
) -> Result<(), RepositoryError> {
    let r = read_record(tx, id, op)?.ok_or_else(state::recovery_required)?;
    let paired = matches!(
        (finish.answer, &finish.link),
        (
            PromotionAnswer::Promoted,
            LinkWrite::Give { .. } | LinkWrite::Withheld
        ) | (
            PromotionAnswer::AlreadyPromoted,
            LinkWrite::Renew { .. } | LinkWrite::Withheld
        ) | (PromotionAnswer::PrimaryHolds, LinkWrite::Untouched)
    );
    if r.path() != RecordPath::Branch
        || !r
            .step
            .permits(PromotionStep::Complete, true, r.integration_mode)
        || !paired
        || !matches!(
            finish.category,
            PublicationCategory::Published | PublicationCategory::AlreadyCurrent
        )
        || !matches!(
            child_state(tx, &r, ChildRole::Context)?,
            ChildState::Complete(_)
        )
    {
        return Err(state::recovery_required());
    }
    tx.execute("UPDATE document_promotions SET step='complete',answer_code=?2,publication_category=?3,cancel_requested=0,updated_at=max(updated_at,?4) WHERE operation_id=?1",params![op.to_string(),finish.answer.name(),finish.category.name(),parent::now()]).map_err(|_|state::recovery_required())?;
    let item = r.item_id.to_string();
    match &finish.link {
        LinkWrite::Untouched | LinkWrite::Withheld => {}
        LinkWrite::Renew { given_at } => {
            pull_request_links::touch_link(tx, id, "document", &item, *given_at)
                .map_err(|_| state::recovery_required())?;
        }
        LinkWrite::Give { link, given_at } => {
            pull_request_links::upsert_link(tx, id, "document", &item, link, *given_at)
                .map_err(|_| state::recovery_required())?;
        }
    }
    read_record(tx, id, op)?.ok_or_else(state::recovery_required)?;
    Ok(())
}
/// A pull-request promotion's end after a context child that ended, in the
/// caller's transaction: the refusal `reconfirmation_required` and the answer,
/// from `confirmed` only. A deleted remote branch also ends the document's
/// link; no other answer touches it.
pub(in super::super) fn abandon_branch_record(
    tx: &Transaction<'_>,
    id: i64,
    op: OperationId,
    answer: PromotionAnswer,
) -> Result<(), RepositoryError> {
    let r = read_record(tx, id, op)?.ok_or_else(state::recovery_required)?;
    if r.path() != RecordPath::Branch
        || !r
            .step
            .permits(PromotionStep::Abandoned, true, r.integration_mode)
        || !matches!(
            answer,
            PromotionAnswer::PrimaryHolds
                | PromotionAnswer::RemoteBranchDeleted
                | PromotionAnswer::HistoryUnresolved
                | PromotionAnswer::ContextGone
        )
        || child_holds(tx, &r)?
    {
        return Err(state::recovery_required());
    }
    let (category, blockers) = Refusal::encoded_columns(Some(&Refusal::ReconfirmationRequired))?;
    tx.execute("UPDATE document_promotions SET step='abandoned',refusal_category=?2,refusal_blockers=?3,answer_code=?4,updated_at=max(updated_at,?5) WHERE operation_id=?1",params![op.to_string(),category,blockers,answer.name(),parent::now()]).map_err(|_|state::recovery_required())?;
    if answer == PromotionAnswer::RemoteBranchDeleted {
        pull_request_links::delete_link(tx, id, "document", &r.item_id.to_string())
            .map_err(|_| state::recovery_required())?;
    }
    read_record(tx, id, op)?.ok_or_else(state::recovery_required)?;
    Ok(())
}

/// The pull-request promotion record that holds the document `item`'s
/// context until it finishes (Cycle 04 decision 13, the design's table F):
/// an unfinished pull-request record, at any step. Retirement skips or
/// refuses a document it holds, as it does a ticket the closure's fence holds.
/// A direct record is not a hold here, and neither is a refused or a finished
/// one.
pub(in super::super) fn pull_request_promotion_holder(
    db: &Connection,
    id: i64,
    item: &ItemId,
) -> Result<Option<OperationId>, RepositoryError> {
    use rusqlite::OptionalExtension;
    if !crate::repository::recovery::has_table(db, TABLE).map_err(|_| state::recovery_required())? {
        return Ok(None);
    }
    let held: Option<String> = db.query_row("SELECT operation_id FROM document_promotions WHERE repository_id=?1 AND item_id=?2 AND integration_mode='pull_request' AND step NOT IN ('complete','abandoned','superseded') LIMIT 1",params![id,item.to_string()],|r|r.get(0)).optional().map_err(|_|state::recovery_required())?;
    held.map(|operation| OperationId::parse(&operation).map_err(|_| state::recovery_required()))
        .transpose()
}

/// Moves a pull-request record's accepted context head from `from` to `to`
/// (RR1), in the caller's transaction: only an unfinished pull-request record
/// at `confirmed` none of whose attempts may have pushed, and only from the
/// head it holds (its consented tip until the first move).
pub(in super::super) fn accept_context_head(
    tx: &Transaction<'_>,
    id: i64,
    op: OperationId,
    from: Oid,
    to: Oid,
) -> Result<(), RepositoryError> {
    let changed = tx.execute("UPDATE document_promotions SET accepted_context_oid=?3,updated_at=max(updated_at,?4) WHERE repository_id=?5 AND operation_id=?1 AND integration_mode='pull_request' AND step='confirmed' AND context_pushed=0 AND coalesce(accepted_context_oid,consented_context_oid)=?2",params![op.to_string(),from.to_string(),to.to_string(),parent::now(),id]).map_err(|_|state::recovery_required())?;
    if changed != 1 {
        return Err(state::recovery_required());
    }
    read_record(tx, id, op)?.ok_or_else(state::recovery_required)?;
    Ok(())
}

/// Records that an attempt of a pull-request record's context child may have
/// pushed (RR3): from then on the re-run table checks no tip, remote or
/// endpoint. Setting it again changes nothing.
pub(in super::super) fn note_context_pushed(
    tx: &Transaction<'_>,
    id: i64,
    op: OperationId,
) -> Result<(), RepositoryError> {
    let changed = tx.execute("UPDATE document_promotions SET context_pushed=1,updated_at=max(updated_at,?2) WHERE repository_id=?3 AND operation_id=?1 AND integration_mode='pull_request' AND step IN ('confirmed','context_synchronized') AND consented_context_oid IS NOT NULL",params![op.to_string(),parent::now(),id]).map_err(|_|state::recovery_required())?;
    if changed != 1 {
        return Err(state::recovery_required());
    }
    read_record(tx, id, op)?.ok_or_else(state::recovery_required)?;
    Ok(())
}

impl ParentRecord for PromotionRecord {
    const KIND: LifecycleKind = KIND;
    fn supersedable(&self) -> bool {
        supersedable(self)
    }
    fn decode(row: &rusqlite::Row<'_>) -> Result<Self, RepositoryError> {
        decode(row)
    }
    fn validate_schema(db: &Connection) -> Result<(), RepositoryError> {
        validate_schema(db)
    }
    fn context_attempt(&self) -> u32 {
        self.context_attempt
    }
    fn primary_attempt(&self) -> u32 {
        self.primary_attempt
    }
    fn expectation_bytes(&self) -> [u8; 32] {
        self.expectation.to_bytes()
    }
}
pub(in super::super) fn child_execution(
    r: &PromotionRecord,
    role: ChildRole,
) -> Result<ChildExecution, RepositoryError> {
    parent::child_execution(r, role)
}
pub(in super::super) fn child_state(
    db: &Connection,
    r: &PromotionRecord,
    role: ChildRole,
) -> Result<ChildState, RepositoryError> {
    parent::child_state(db, r, role)
}
pub(in super::super) fn next_attempt(
    tx: &Transaction<'_>,
    id: i64,
    op: OperationId,
    role: ChildRole,
) -> Result<PromotionRecord, RepositoryError> {
    parent::next_attempt(tx, id, op, role)
}
/// The sole back-transition: a fresh integration preparation proved primary
/// advancement before `integrating` intent. A completed context child may be
/// replaced here; ordinary retries may not replace authoritative completion.
pub(in super::super) fn reset_context_after_primary_advance(
    tx: &Transaction<'_>,
    id: i64,
    op: OperationId,
) -> Result<PromotionRecord, RepositoryError> {
    parent::reset_context_after_primary_advance(tx, id, op, |r: &PromotionRecord| {
        r.step == PromotionStep::ContextSynchronized
    })
}
pub(in super::super) fn validate_local_identity(
    db: &Connection,
    root: &str,
    action: &str,
    op: &str,
    target: &str,
) -> Result<(), RepositoryError> {
    parent::validate_local_identity(KIND, db, root, action, op, target)
}
pub(in super::super) fn audit(db: &Connection) -> Result<(), RepositoryError> {
    parent::audit::<PromotionRecord>(db)
}
pub(in super::super) fn complete_record(
    tx: &Transaction<'_>,
    service: &RepositoryService,
    id: i64,
    op: OperationId,
    evidence: &RetirementEvidence<'_>,
    owner: Option<&RemoteReservation>,
) -> Result<(), RepositoryError> {
    parent::complete_record::<PromotionRecord>(tx, service, id, op, evidence, owner)
}
/// Inspect actual committed parent state after an uncertain commit result.
pub(in super::super) fn reconcile_completion(
    db: &Transaction<'_>,
    id: i64,
    op: OperationId,
) -> Result<bool, RepositoryError> {
    parent::reconcile_completion::<PromotionRecord>(db, id, op)
}
pub(in super::super) fn request_cancellation(
    tx: &Transaction<'_>,
    id: i64,
    op: OperationId,
) -> Result<bool, RepositoryError> {
    parent::request_cancellation::<PromotionRecord>(tx, id, op)
}
pub(in super::super) fn clear_cancellation(
    tx: &Transaction<'_>,
    id: i64,
    op: OperationId,
) -> Result<(), RepositoryError> {
    parent::clear_cancellation::<PromotionRecord>(tx, id, op)
}

#[cfg(test)]
#[path = "record_tests.rs"]
mod tests;
