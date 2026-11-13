//! The facts one repository's state is made from, owned, so a table row
//! changes only what it is about. Nothing sleeps and nothing reads a clock:
//! `now` is a fixed instant.

use std::time::Instant;

use manyhands::{
    canonical::ItemId,
    repository::{
        Accessibility, ConfigurationDto, ConfigurationState, IndexState, IndexStateDto,
        PollingOutcome, ProblemDto, RepositorySummaryDto,
    },
    results::{ProblemCode, ResultCode},
};
use time::OffsetDateTime;

use crate::desktop::{
    model::{
        navigation::{RepositoryEntry, Resolution},
        polling::{
            Answer, AttemptSummary, AttemptTarget, Disposition, KeyId, Phase, Policy, PollAttempt,
            RepoSchedule, TargetKind, TransportFailure, Why, status::Unplaced,
        },
        repository_state::{RepoFacts, RepoState, repository_state},
    },
    services::{
        ReadFailure,
        scheduler::{Holding, SharedCopy},
    },
};

pub(super) const ROOT: &str = "/work/alpha";
/// The `n`th item's ID text, a valid ULID.
pub(super) fn ulid(n: u32) -> String {
    format!("01JD{n:022}")
}

pub(super) fn id(n: u32) -> ItemId {
    ulid(n).parse().expect("an item ID")
}

/// The one key the scene's locked rows name.
pub(super) fn key() -> KeyId {
    static KEY: std::sync::OnceLock<KeyId> = std::sync::OnceLock::new();
    *KEY.get_or_init(KeyId::new)
}

pub(super) fn problem(code: ProblemCode) -> ProblemDto {
    ProblemDto {
        code,
        path: None,
        target_id: None,
    }
}

pub(super) fn at(minutes_ago: i64) -> OffsetDateTime {
    now() - time::Duration::minutes(minutes_ago)
}

pub(super) fn now() -> OffsetDateTime {
    OffsetDateTime::UNIX_EPOCH + time::Duration::days(20_000)
}

/// A target of `disposition`: an item with a kind, or main.
pub(super) fn item(n: u32, disposition: Disposition) -> AttemptTarget {
    AttemptTarget {
        item_id: Some(id(n)),
        kind: Some(TargetKind::Ticket),
        disposition,
        main: false,
    }
}

pub(super) fn main_ahead() -> AttemptTarget {
    AttemptTarget {
        item_id: None,
        kind: None,
        disposition: Disposition::LocalAhead,
        main: true,
    }
}

/// An attempt that ended in `answer`, as the schedule keeps it.
pub(super) fn attempt(answer: Answer) -> AttemptSummary {
    AttemptSummary::of(&PollAttempt {
        root: ROOT.to_owned(),
        operation: manyhands::repository::OperationId::new(),
        completed_at: (answer == Answer::Completed).then(|| at(1)),
        answer,
        next_eligible_at: None,
        targets: Vec::new(),
        index_pending: false,
    })
}

pub(super) fn unreachable() -> Answer {
    Answer::Transport(TransportFailure::Unreachable)
}

pub(super) fn waiting(why: Why) -> Phase {
    Phase::Waiting {
        until: Instant::now(),
        why,
        op: None,
    }
}

pub(super) struct Scene {
    pub entry: RepositoryEntry,
    pub kept: bool,
    pub schedule: Option<RepoSchedule>,
    pub unplaced: Option<Unplaced>,
    pub holding: Option<Holding>,
    pub stored: Option<PollingOutcome>,
    pub shared_copy: Option<SharedCopy>,
    pub checking_now: bool,
    pub sharing_main: bool,
    pub polling: bool,
    pub unlock_target: bool,
}

impl Scene {
    /// A valid repository with an SSH publication remote, scheduled, never
    /// checked: Part B's row 13 without a label.
    pub fn new() -> Self {
        let policy = Policy {
            enabled: true,
            paused: false,
            interval: std::time::Duration::from_secs(300),
            backoff: None,
            recovery_suspended: false,
            next_eligible_at: None,
            active_operation: None,
        };
        Self {
            entry: RepositoryEntry {
                summary: RepositorySummaryDto {
                    root: ROOT.to_owned(),
                    enabled_at: None,
                    accessibility: Accessibility::Accessible,
                    configuration: ConfigurationDto {
                        state: ConfigurationState::Valid,
                        primary_branch: Some("main".to_owned()),
                        publication_remote: Some("origin".to_owned()),
                        problems: Vec::new(),
                    },
                    index: IndexStateDto {
                        state: IndexState::Current,
                        refreshed_at: None,
                    },
                    problem_count: 0,
                },
                resolution: Resolution::Unknown,
            },
            kept: false,
            schedule: Some(RepoSchedule::new(policy, None)),
            unplaced: None,
            holding: None,
            stored: None,
            shared_copy: None,
            checking_now: false,
            sharing_main: false,
            polling: true,
            unlock_target: true,
        }
    }

    pub fn schedule(&mut self) -> &mut RepoSchedule {
        self.schedule.as_mut().expect("a schedule")
    }

    /// No schedule: the repository has no SSH publication remote.
    pub fn unscheduled(&mut self, remote: Option<&str>) {
        self.schedule = None;
        self.entry.summary.configuration.publication_remote = remote.map(str::to_owned);
    }

    pub fn resolution(&mut self, resolution: Resolution) {
        self.entry.resolution = resolution;
    }

    pub fn unavailable(&mut self) {
        self.resolution(Resolution::Unavailable(ResultCode::RepositoryNotRegistered));
    }

    pub fn failed(&mut self) {
        self.resolution(Resolution::Failed(ReadFailure {
            code: ResultCode::RepositoryInaccessible,
            recovery: Vec::new(),
        }));
    }

    pub fn invalid(&mut self, state: ConfigurationState, problems: Vec<ProblemDto>) {
        self.entry.summary.configuration.state = state;
        self.entry.summary.configuration.problems = problems;
    }

    /// The check that last completed found these, as of `minutes` ago.
    pub fn checked(&mut self, items: Vec<AttemptTarget>, problems: Vec<AttemptTarget>) {
        let schedule = self.schedule();
        schedule.unshared.items = items;
        schedule.unshared.problems = problems;
        schedule.unshared.as_of = Some(at(7));
    }

    pub fn facts<'a>(&'a self, titles: &'a dyn Fn(&ItemId) -> Option<String>) -> RepoFacts<'a> {
        RepoFacts {
            entry: &self.entry,
            kept: self.kept,
            schedule: self.schedule.as_ref(),
            unplaced: self.unplaced,
            holding: self.holding,
            stored: self.stored,
            shared_copy: self.shared_copy.as_ref(),
            checking_now: self.checking_now,
            sharing_main: self.sharing_main,
            polling: self.polling,
            unlock_target: self.unlock_target,
            titles,
            now: now(),
        }
    }

    pub fn state(&self) -> RepoState {
        repository_state(&self.facts(&|_| None))
    }
}
