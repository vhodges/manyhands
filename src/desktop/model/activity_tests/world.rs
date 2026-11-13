//! The fixture of the state rows' tests (S2-S14): what the Workspace would
//! hand `state_rows`, and the row a case must make.

use std::collections::BTreeMap;

use manyhands::{
    canonical::ItemId,
    repository::{
        AuthoringKind, OperationAction, OperationDto, OperationFamily, OperationId,
        OperationListDto, OperationNextAction, OperationOwner,
    },
};
use time::OffsetDateTime;

use super::super::support::{ITEM, PRIMARY, ROOT, TITLE};
use crate::desktop::{
    model::{
        activity::{
            rows::{ActivityAction, Derived, Group, ItemRef, Kind, Offer, RowKey},
            states::{DraftsInput, RepositoryInput, StateInputs, state_rows},
        },
        operations::Operations,
        polling::{
            Answer, AttemptTarget, Clock, Disposition, KeyId, Policy, PollAttempt, PollSchedule,
            TargetKind, TestClock, Unshared,
        },
        repository_state::{AttentionCause, RepoState, RowAction, StateKind},
        stamp::RepoKey,
    },
    services::ReadResult,
};

pub(in super::super) const OTHER: &str = "/fixture/other";
/// A second item, a document, and the short codes the two go by.
pub(in super::super) const DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD0";
pub(in super::super) const SHORT: &str = "Q69G5FG0";
pub(in super::super) const DOCUMENT_SHORT: &str = "Q69G5FD0";

/// An operation's ID, as pending rows and requests name it.
pub(in super::super) const OPERATION: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";

/// What the Workspace hands the state rows, with nothing to say yet: one
/// repository with a state that makes no row.
pub(in super::super) struct World {
    pub operations: Operations,
    pub schedule: PollSchedule,
    pub repositories: Vec<RepositoryInput>,
    pub drafts: DraftsInput,
    /// `(root, operation)` pairs whose pending conflict waits.
    pub waits: Vec<(RepoKey, String)>,
    pub unlockable: bool,
    /// `(root, item)` pairs whose lifecycle runs.
    pub running: Vec<(RepoKey, ItemId)>,
    pub titled: bool,
    pub clock: TestClock,
}

/// A state with no label, as a case sets it.
pub(in super::super) fn state(
    kind: StateKind,
    action: Option<RowAction>,
    sentence: &[&str],
    cause: Option<AttentionCause>,
) -> RepoState {
    RepoState {
        kind,
        label: None,
        icon: None,
        tone: None,
        action,
        sentence: sentence.iter().map(|part| (*part).to_owned()).collect(),
        cause,
    }
}

pub(in super::super) fn repository(root: &str, name: &str, state: RepoState) -> RepositoryInput {
    RepositoryInput {
        root: root.to_owned(),
        name: name.to_owned(),
        state,
        remote: None,
        checking: false,
        primary: None,
        unshared: Unshared::default(),
        sharing: BTreeMap::new(),
        retiring: Default::default(),
        retire_offers: Vec::new(),
    }
}

impl World {
    pub fn new() -> Self {
        Self {
            operations: Operations::default(),
            schedule: PollSchedule::default(),
            repositories: vec![repository(
                ROOT,
                "demo",
                state(StateKind::UpToDate, None, &[], None),
            )],
            drafts: DraftsInput {
                flush_failures: Vec::new(),
                unavailable: None,
                recovered: None,
            },
            waits: Vec::new(),
            unlockable: true,
            running: Vec::new(),
            titled: true,
            clock: TestClock::new(),
        }
    }

    /// The state rows the Workspace would derive.
    pub fn rows(&self) -> Vec<Derived> {
        let title =
            |_: &RepoKey, _: AuthoringKind, _: &ItemId| self.titled.then(|| TITLE.to_owned());
        let primary = |_: &RepoKey| Some(PRIMARY.to_owned());
        let waits = |root: &RepoKey, operation: &str| {
            self.waits
                .iter()
                .any(|(held, wait)| held == root && wait == operation)
        };
        let unlock = |_: &KeyId| self.unlockable;
        let running = |root: &RepoKey, item: &ItemId| {
            self.running
                .iter()
                .any(|(held, running)| held == root && running == item)
        };
        state_rows(&StateInputs {
            operations: &self.operations,
            schedule: &self.schedule,
            repositories: &self.repositories,
            drafts: &self.drafts,
            conflict_waits: &waits,
            title: &title,
            primary: &primary,
            unlock_target: &unlock,
            lifecycle_running: &running,
        })
    }

    pub fn utc(&self) -> OffsetDateTime {
        self.clock.utc()
    }

    /// Time passes: the next check is applied later.
    pub fn tick(&self) {
        self.clock.advance(std::time::Duration::from_secs(61));
    }

    /// The repository `root` is scheduled, enabled and unpaused.
    pub fn schedule_root(&mut self, root: &str) {
        self.schedule.add(root.to_owned(), policy(), None);
    }

    /// An automatic poll's attempt, applied now.
    pub fn automatic(&mut self, root: &str, answer: Answer, seen: &[Disposition]) {
        let attempt = self.attempt(root, answer, seen);
        self.schedule
            .after(&self.clock.now(), &attempt)
            .expect("a scheduled repository");
    }

    /// A Check now's attempt, applied now.
    pub fn check_now(&mut self, root: &str, answer: Answer, seen: &[Disposition]) {
        let attempt = self.attempt(root, answer, seen);
        self.schedule
            .checked_now(&self.clock.now(), &attempt)
            .expect("a scheduled repository");
    }

    fn attempt(&self, root: &str, answer: Answer, seen: &[Disposition]) -> PollAttempt {
        PollAttempt {
            root: root.to_owned(),
            operation: OperationId::new(),
            completed_at: (answer == Answer::Completed).then(|| self.clock.utc()),
            answer,
            next_eligible_at: None,
            targets: seen.iter().map(|&seen| target(seen)).collect(),
            index_pending: false,
        }
    }

    /// The repository `root`'s pending operations, read: `rows` listed.
    pub fn pending(&mut self, root: &str, rows: Vec<OperationDto>) {
        let stamp = self.operations.load_pending(root.to_owned());
        let list = OperationListDto {
            items: rows,
            complete: true,
        };
        assert!(self.operations.apply_pending(stamp, ReadResult::Ok(list)));
    }
}

/// Enabled, unpaused, no deadline.
pub(in super::super) fn policy() -> Policy {
    Policy {
        enabled: true,
        paused: false,
        interval: std::time::Duration::from_secs(300),
        backoff: None,
        recovery_suspended: false,
        next_eligible_at: None,
        active_operation: None,
    }
}

/// The item `ITEM`, as a target the poll reported with `disposition`.
pub(in super::super) fn target(disposition: Disposition) -> AttemptTarget {
    AttemptTarget {
        item_id: Some(ITEM.parse().unwrap()),
        kind: Some(TargetKind::Ticket),
        disposition,
        main: false,
    }
}

/// A pending operation of `action`, with an ID, waiting to be resumed.
pub(in super::super) fn operation(
    action: OperationAction,
    family: OperationFamily,
) -> OperationDto {
    OperationDto {
        operation_id: Some(OPERATION.to_owned()),
        family,
        owner: OperationOwner::Repository,
        action,
        state: "prepared".to_owned(),
        completed_step: None,
        next_action: Some(OperationNextAction::Resume),
        item_id: None,
        key_id: None,
        worktree: None,
        updated_at: None,
        failure_code: None,
    }
}

/// What one state row must be: its key, kind and words first, then the
/// rest as the case says.
pub(in super::super) struct Want {
    key: RowKey,
    kind: Kind,
    title: String,
    description: Option<String>,
    repo: Option<String>,
    button: Option<Offer>,
    menu: Vec<Offer>,
    detail: Vec<String>,
    item: Option<ItemRef>,
    clearable: bool,
    at: Option<OffsetDateTime>,
    group: Group,
}

pub(in super::super) fn want(key: RowKey, kind: Kind, title: &str) -> Want {
    Want {
        key,
        kind,
        title: title.to_owned(),
        description: None,
        repo: None,
        button: None,
        menu: Vec::new(),
        detail: Vec::new(),
        item: None,
        clearable: false,
        at: None,
        group: Group::default(),
    }
}

pub(in super::super) fn offer(action: ActivityAction, label: &str, enabled: bool) -> Offer {
    Offer {
        action,
        label: label.to_owned(),
        enabled,
    }
}

impl Want {
    pub fn desc(mut self, description: &str) -> Self {
        self.description = Some(description.to_owned());
        self
    }

    pub fn repo(mut self, repo: &str) -> Self {
        self.repo = Some(repo.to_owned());
        self
    }

    /// The button, which is also the menu's first offer.
    pub fn button(mut self, action: ActivityAction, label: &str, enabled: bool) -> Self {
        let button = offer(action, label, enabled);
        self.menu.insert(0, button.clone());
        self.button = Some(button);
        self
    }

    /// An offer after the button.
    pub fn then(mut self, action: ActivityAction, label: &str, enabled: bool) -> Self {
        self.menu.push(offer(action, label, enabled));
        self
    }

    pub fn detail(mut self, lines: &[&str]) -> Self {
        self.detail = lines.iter().map(|line| (*line).to_owned()).collect();
        self
    }

    pub fn item(self, kind: AuthoringKind) -> Self {
        self.item_of(ITEM.parse().unwrap(), kind)
    }

    pub fn item_of(mut self, id: ItemId, kind: AuthoringKind) -> Self {
        self.item = Some(ItemRef {
            root: ROOT.to_owned(),
            kind,
            id,
        });
        self
    }

    pub fn clearable(mut self) -> Self {
        self.clearable = true;
        self
    }

    pub fn at(mut self, at: OffsetDateTime) -> Self {
        self.at = Some(at);
        self
    }

    /// What the log's attention pass reads of the row.
    pub fn group(mut self, group: Group) -> Self {
        self.group = group;
        self
    }

    /// The row is exactly this.
    pub fn holds(&self, row: &Derived, case: &str) {
        let said = (&row.key, row.kind, &row.title, &row.description, &row.repo);
        let wanted = (
            &self.key,
            self.kind,
            &self.title,
            &self.description,
            &self.repo,
        );
        assert_eq!(said, wanted, "{case}: key, kind, words and tag");
        assert_eq!(row.button, self.button, "{case}: button");
        assert_eq!(row.menu, self.menu, "{case}: menu");
        assert_eq!(row.detail, self.detail, "{case}: detail");
        assert_eq!(row.item, self.item, "{case}: item");
        assert_eq!(row.clearable, self.clearable, "{case}: clearable");
        assert_eq!(row.at, self.at, "{case}: at");
        assert_eq!(row.group, self.group, "{case}: group");
        assert!(
            !row.attention,
            "{case}: a source leaves attention to the log"
        );
    }
}

/// The rows `rows` has, by key, are exactly `wanted`, in order.
pub(in super::super) fn assert_rows(rows: &[Derived], wanted: &[Want], case: &str) {
    let keys: Vec<&RowKey> = rows.iter().map(|row| &row.key).collect();
    let wanted_keys: Vec<&RowKey> = wanted.iter().map(|want| &want.key).collect();
    assert_eq!(keys, wanted_keys, "{case}: the rows made");
    for (row, want) in rows.iter().zip(wanted) {
        want.holds(row, case);
    }
}

/// The group of a candidate row over `roots`.
pub(in super::super) fn counted(roots: &[&str]) -> Group {
    Group {
        candidate: true,
        roots: roots.iter().map(|root| (*root).to_owned()).collect(),
        ..Group::default()
    }
}

/// The group of a row about `roots` that counts for nothing.
pub(in super::super) fn quiet(roots: &[&str]) -> Group {
    Group {
        roots: roots.iter().map(|root| (*root).to_owned()).collect(),
        ..Group::default()
    }
}

/// A key's ID, for the holds and states that name one.
pub(in super::super) fn key_id() -> manyhands::repository::SharedKeyId {
    manyhands::repository::SharedKeyId::parse("01ARZ3NDEKTSV4RRFFQ69G5FB1").unwrap()
}
