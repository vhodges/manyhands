//! The types of Activity's one store of rows (Wave 04 Cycle 03b, design
//! A1). A source derives a `Derived`; the log (Task 5) gives it a time and a
//! revision as an `ActivityRow` and settles its attention. No field is unset
//! at any point: a source that does not name a `Group` field leaves it
//! `None`, empty or false.
//!
//! No GPUI: a view draws these.

use manyhands::{
    canonical::ItemId,
    drafts::DraftId,
    repository::{AuthoringKind, ItemDtoKind},
    results::Outcome,
};
use time::OffsetDateTime;

use crate::desktop::{
    model::{
        polling::{Reason, TargetKind, attention::Action},
        publication::SyncOffer,
        repository_state::RowAction,
        stamp::RepoKey,
    },
    services::{JobKind, inflight::Report, requests::DesktopRequestId},
};

/// What a row is about; its identity across refreshes.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) enum RowKey {
    Request(DesktopRequestId),
    Brought {
        root: RepoKey,
        seq: u64,
    },
    /// `hold_key(&Reason)` (R-3).
    Hold(String),
    Repository(RepoKey),
    Problem {
        root: RepoKey,
        target: Target,
    },
    Unshared {
        root: RepoKey,
        target: Target,
    },
    RetireOffer {
        root: RepoKey,
        item: ItemId,
    },
    /// S8, a Retired event: `RetireEvent.seq` (A2, R-51).
    Retired(u64),
    /// S8, an information event: the latest per item (R-59).
    RetireNote {
        root: RepoKey,
        item: ItemId,
    },
    /// An operation ID, or "#{n}" when it has none (R-4).
    Pending {
        root: RepoKey,
        operation: String,
    },
    PendingList(RepoKey),
    Flush(DraftId),
    DraftStore,
    Recovered,
    Note(Report),
}

/// A target of the unshared list. An item whose ID did not parse also has
/// `item_id: None` (`D/model/polling/attempt.rs:134-140`), so it is
/// `Unnamed(n)`, its position among that list's targets with no ID.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) enum Target {
    Main,
    Item(ItemId),
    Unnamed(u32),
}

/// The item a row is about.
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct ItemRef {
    pub root: RepoKey,
    pub kind: AuthoringKind,
    pub id: ItemId,
}

/// What the row's icon colour says (`03`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Kind {
    Info,
    Success,
    Sharing,
    ToShare,
    Failure,
    Locked,
    Working,
}

/// One control a row offers: its action, its label and whether it can be
/// pressed now.
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Offer {
    pub action: ActivityAction,
    pub label: String,
    pub enabled: bool,
}

impl Offer {
    pub(crate) fn new(action: ActivityAction, label: &str, enabled: bool) -> Self {
        Self {
            action,
            label: label.to_owned(),
            enabled,
        }
    }
}

/// What the log's attention pass (A3) reads of a row, set by its source.
#[derive(Clone, Default, PartialEq, Debug)]
pub(crate) struct Group {
    /// The source's own "would count" (A2's Attention column).
    pub candidate: bool,
    /// S1: `operation_of(request)`, else an OperationResume's or
    /// OperationAbandon's operation ID; S9: the row's operation ID.
    pub operation: Option<String>,
    /// S3: every root the hold covers; S4, S5, S9: its root; S1: the
    /// intent's root.
    pub roots: Vec<RepoKey>,
    /// S1: a PollOnce request.
    pub check: bool,
    /// S1: `RequestFacts.check_applied` (N4, R-58).
    pub check_applied: bool,
    /// S1: `RequestFacts.superseded` (N22 clause 4).
    pub superseded: bool,
    /// S1: its `Intent` subject (A3; R-52, R-57); S8 Retired: `Intent {
    /// Retire, root, item, None }`.
    pub subject: Option<Subject>,
    /// S1: its attempt's queue order, `Operations::sent` (N2, N22, R-56);
    /// S8: `RetireEvent.sent`.
    pub sent: Option<u64>,
    /// S1: its finished outcome (N22); S8 Retired: Success.
    pub outcome: Option<Outcome>,
    /// S1: the operation it runs, or its operation when an automatic poll
    /// runs it (N21).
    pub running_operation: Option<String>,
    /// S1: `RequestFacts.followed` (N23).
    pub followed: bool,
    /// S1: `RequestFacts.follow_void` (N23 undone).
    pub follow_void: bool,
    /// S1: `RequestFacts.resend_waits` (A1's `clearable`).
    pub resend_waits: bool,
}

/// A request's subject (A3; R-52, R-57).
#[derive(Clone, PartialEq, Debug)]
pub(crate) enum Subject {
    /// `Group.operation` as a subject: N22 clause 1's extra match.
    #[allow(dead_code)] // Used from Task 5
    Operation(String),
    Intent {
        family: Family,
        root: Option<RepoKey>,
        item: Option<ItemId>,
        target: Option<String>,
    },
}

/// An intent with its retries and resends, and a share with any later share
/// of the same item.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Family {
    ItemShare,
    MainShare,
    Check,
    Retire,
    Save,
    Own(JobKind),
}

/// A row as derived: everything but its time, unless the source has one.
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Derived {
    pub key: RowKey,
    pub kind: Kind,
    pub title: String,
    pub description: Option<String>,
    /// The repository tag: `{repo}`, or `{repo} +{n}` for a hold over more
    /// than one repository.
    pub repo: Option<String>,
    /// At most one offer (decisions 8 and 24).
    pub button: Option<Offer>,
    /// Every offer, the button's first, then today's control order, then
    /// Dismiss when the row can be dismissed.
    pub menu: Vec<Offer>,
    /// The tooltip's lines after the full message and the time (B5).
    pub detail: Vec<String>,
    /// Set by the log (A3); a source leaves it false.
    pub attention: bool,
    pub item: Option<ItemRef>,
    /// "Clear finished" may take the row (A1, R-47).
    pub clearable: bool,
    /// S2 only: `Brought.at` (L1, T1).
    pub at: Option<OffsetDateTime>,
    pub group: Group,
}

impl Derived {
    /// A row with its key, kind and title, and nothing else: no description,
    /// tag, offer, detail or time, and a quiet group.
    pub(crate) fn new(key: RowKey, kind: Kind, title: String) -> Self {
        Self {
            key,
            kind,
            title,
            description: None,
            repo: None,
            button: None,
            menu: Vec::new(),
            detail: Vec::new(),
            attention: false,
            item: None,
            clearable: false,
            at: None,
            group: Group::default(),
        }
    }
}

/// A row of the log: a `Derived`, the time the app first learned of it, and
/// its revision, from a log-wide counter (A4).
#[allow(dead_code)] // Used from Task 5
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct ActivityRow {
    pub at: OffsetDateTime,
    pub revision: u64,
    pub row: Derived,
}

/// What a row's offer does. Each runs an existing handler (Part F4).
#[derive(Clone, PartialEq, Debug)]
pub(crate) enum ActivityAction {
    Cancel(DesktopRequestId),
    Stop(DesktopRequestId),
    /// The lifecycle dialog.
    Open(DesktopRequestId),
    /// The conflict sheet.
    Review(DesktopRequestId),
    Offer(DesktopRequestId, SyncOffer),
    /// `run_recovery(id, n)`.
    Recovery(DesktopRequestId, usize),
    /// S1, S2 and S8 only.
    Dismiss(RowKey),
    /// S13.
    DismissRecovered,
    /// S14.
    DismissNote(Report),
    /// A hold's action, on its first root.
    Attend(Action, Reason, RepoKey),
    /// Cycle 03a's row action.
    Repository(RepoKey, RowAction),
    OpenItem(ItemRef),
    Share {
        root: RepoKey,
        item: Option<(AuthoringKind, ItemId)>,
    },
    Retire {
        root: RepoKey,
        kind: TargetKind,
        item: ItemId,
    },
    Resume {
        root: RepoKey,
        operation: String,
    },
    /// `LifecycleRow`'s fields (S9).
    ResumeLifecycle {
        root: RepoKey,
        kind: ItemDtoKind,
        item: String,
        operation: String,
    },
    ReviewPending {
        root: RepoKey,
        operation: String,
    },
    Abandon {
        root: RepoKey,
        operation: String,
    },
    RetryPendingList(RepoKey),
    RetryFlush(DraftId),
    ReviewDrafts,
}
