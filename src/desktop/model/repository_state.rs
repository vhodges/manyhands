//! One repository's state, tooltip and menu (Cycle 03a, Part B and D): pure
//! functions of the facts the Workspace already holds, so the row, the
//! tooltip, the accessible description and the menu's first line all say
//! the same.
//!
//! The first row of Part B's table that holds wins. Nothing here reads a
//! clock or does I/O: `now` comes from the facts, and the titles of the
//! items not shared yet come through a closure. Whether a stored outcome
//! counts is `status::stored_applies` alone (R-32); the words of a check's
//! outcome are D6's, from `status::row`, so Settings and the panel agree.
//!
//! No text here carries a key, a passphrase or a URL that is not the
//! library's redacted one (Part I).

mod menu;
mod words;

use manyhands::{
    canonical::ItemId,
    repository::{ConfigurationState, PollingOutcome},
};
use time::OffsetDateTime;

#[allow(unused_imports)] // Used from Task 3
pub(crate) use self::menu::{
    CHECK_NOW, MenuChoice, MenuItem, OPEN_FOLDER, REMOVE_FROM_MANYHANDS, RETRY_NOW, SET_UP_SHARING,
    SETTINGS, menu_items, more_items,
};
#[allow(unused_imports)] // Used from Task 3
pub(crate) use self::words::*;
use super::{
    list_view::RETRY,
    navigation::{RepositoryEntry, Resolution},
    polling::{
        Answer, KeyId, Phase, Reason, RepoSchedule, TransportFailure, Why,
        attention::{self, Action, action_of},
        status::{self, Shape, Unplaced, ago},
    },
};
use crate::desktop::services::scheduler::{Holding, SharedCopy};

/// What a repository's state is made from: what the Workspace already has
/// for it.
pub(crate) struct RepoFacts<'a> {
    /// The registration and what resolving it gave.
    pub entry: &'a RepositoryEntry,
    /// The chosen repository the list no longer gives (decision 22).
    pub kept: bool,
    pub schedule: Option<&'a RepoSchedule>,
    pub unplaced: Option<Unplaced>,
    pub holding: Option<Holding>,
    /// How the library's latest check ended, before this process ran. It
    /// speaks only where `status::stored_applies` says.
    pub stored: Option<PollingOutcome>,
    pub shared_copy: Option<&'a SharedCopy>,
    /// This process's own Check now is queued or running.
    pub checking_now: bool,
    /// A share of main is queued or running.
    pub sharing_main: bool,
    /// The scheduler is on (`Services::with_polling`).
    pub polling: bool,
    /// An Unlock… has a key to ask for (fact 46).
    pub unlock_target: bool,
    /// The title of an item not shared yet, when it is known.
    pub titles: &'a dyn Fn(&ItemId) -> Option<String>,
    pub now: OffsetDateTime,
}

impl RepoFacts<'_> {
    /// D6's facts for this repository, whose words and shape the state
    /// reads.
    fn polling_facts<'f>(&'f self, name: &'f str) -> status::Facts<'f> {
        status::Facts {
            name,
            schedule: self.schedule,
            unplaced: self.unplaced,
            holding: self.holding,
            checking_now: self.checking_now,
            stored: self.stored,
            now: self.now,
        }
    }

    /// D6's row for this repository: its status, its detail and its shape.
    pub(crate) fn row(&self) -> status::Row {
        status::row(&self.polling_facts(&self.entry.name()))
    }

    /// Checking: this process's Check now, or a poll queued or running.
    pub(crate) fn checking(&self) -> bool {
        self.checking_now || self.schedule.is_some_and(RepoSchedule::in_flight)
    }
}

/// Part B's rows 1 to 13, in order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum StateKind {
    CantOpen,
    BusyOpening,
    NeedsFixing,
    Checking,
    Locked,
    NeedsAttention,
    CantReach,
    ToShare(usize),
    Paused,
    UpToDate,
    ThisComputer,
    NotSsh,
    /// Row 13: reading, not checked yet, checking soon, waiting for an
    /// unfinished operation, checking off or polling off.
    Unknown,
}

/// The tone a state's label and icon are drawn in. Never the only signal:
/// a state with a tone has a label and an icon.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Tone {
    Failure,
    Warning,
    Working,
    Muted,
    Success,
    Secondary,
}

/// What a state offers.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum RowAction {
    /// `Navigation::retry_resolution`.
    Retry,
    /// `Scheduler::reread`: the scheduler could not read the repository.
    Reread,
    /// A locked key's Unlock….
    Unlock(KeyId),
    /// D6's action for a reason (`action_of`).
    Attend(Action, Reason),
    CheckNow,
    RetryNow,
    /// Settings for this repository.
    Settings,
    /// Settings › Remotes for this repository.
    SetUpSharing,
}

/// A repository's row: the label and icon at the right, and what it says.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct RepoState {
    pub kind: StateKind,
    pub label: Option<String>,
    pub icon: Option<&'static str>,
    pub tone: Option<Tone>,
    pub action: Option<RowAction>,
    /// The state's sentence, in parts: the first of the tooltip, the
    /// description and the menu. Empty when there is nothing to say.
    pub sentence: Vec<String>,
}

impl RepoState {
    fn labelled(
        kind: StateKind,
        (label, icon, tone): (String, &'static str, Tone),
        action: Option<RowAction>,
        sentence: Vec<String>,
    ) -> Self {
        Self {
            kind,
            label: Some(label),
            icon: Some(icon),
            tone: Some(tone),
            action,
            sentence,
        }
    }

    /// A state with no label, icon or tone (rows 2, 12 and 13).
    fn plain(kind: StateKind, action: Option<RowAction>, sentence: Vec<String>) -> Self {
        Self {
            kind,
            label: None,
            icon: None,
            tone: None,
            action,
            sentence,
        }
    }
}

/// How a state shows: its label, icon and tone.
fn look(label: &str, icon: &'static str, tone: Tone) -> (String, &'static str, Tone) {
    (label.to_owned(), icon, tone)
}

/// The words for a state's action.
pub(crate) fn action_label(action: &RowAction) -> &'static str {
    match action {
        RowAction::Retry | RowAction::Reread => RETRY,
        RowAction::Unlock(_) => attention::UNLOCK,
        RowAction::Attend(action, _) => action.label(),
        RowAction::CheckNow => CHECK_NOW,
        RowAction::RetryNow => RETRY_NOW,
        RowAction::Settings => SETTINGS,
        RowAction::SetUpSharing => SET_UP_SHARING,
    }
}

/// The one state of a repository: the first row of Part B that holds.
pub(crate) fn repository_state(facts: &RepoFacts<'_>) -> RepoState {
    let name = facts.entry.name();
    cant_open(facts, &name)
        .or_else(|| needs_fixing(facts))
        .or_else(|| checking(facts, &name))
        .or_else(|| locked(facts))
        .or_else(|| needs_attention(facts, &name))
        .or_else(|| rest(facts))
        .unwrap_or_else(|| unknown(facts))
}

/// Rows 1 and 2, from the registration alone.
fn cant_open(facts: &RepoFacts<'_>, repo: &str) -> Option<RepoState> {
    let sentence = |first: &str| vec![first.to_owned(), UNCHANGED.to_owned()];
    let sentence = match &facts.entry.resolution {
        _ if facts.kept => vec![kept_sentence(repo)],
        Resolution::Unavailable(code) => sentence(code.message()),
        Resolution::Failed(failure) => sentence(failure.code.message()),
        Resolution::Busy => {
            return Some(RepoState::plain(
                StateKind::BusyOpening,
                Some(RowAction::Retry),
                vec![busy_sentence(repo)],
            ));
        }
        Resolution::Unknown | Resolution::Resolved(_) => return None,
    };
    Some(RepoState::labelled(
        StateKind::CantOpen,
        look(CANT_OPEN, "circle-alert", Tone::Failure),
        Some(RowAction::Retry),
        sentence,
    ))
}

/// Row 3.
fn needs_fixing(facts: &RepoFacts<'_>) -> Option<RepoState> {
    let configuration = &facts.entry.summary.configuration;
    if configuration.state == ConfigurationState::Valid {
        return None;
    }
    let mut sentence = vec![NEEDS_FIXING_SENTENCE.to_owned()];
    sentence.extend(
        configuration
            .problems
            .iter()
            .map(|problem| problem.guidance().to_owned()),
    );
    Some(RepoState::labelled(
        StateKind::NeedsFixing,
        look(NEEDS_FIXING, "triangle-alert", Tone::Warning),
        Some(RowAction::Settings),
        sentence,
    ))
}

/// Row 4: before every row that waits on the person, so an Unlock…, Retry
/// now or Check now shows that it is working (decision 8).
fn checking(facts: &RepoFacts<'_>, repo: &str) -> Option<RepoState> {
    facts.checking().then(|| {
        RepoState::labelled(
            StateKind::Checking,
            look(CHECKING, "loader-circle", Tone::Working),
            None,
            vec![checking_sentence(repo)],
        )
    })
}

/// Row 5: a live locked key, or a stored `UnlockRequired`.
fn locked(facts: &RepoFacts<'_>) -> Option<RepoState> {
    let action = match facts.schedule.map(|schedule| &schedule.phase) {
        Some(Phase::Attention(Reason::KeyLocked(key))) => RowAction::Unlock(*key),
        _ if stored_outcome(facts) == Some(PollingOutcome::UnlockRequired) => RowAction::CheckNow,
        _ => return None,
    };
    Some(RepoState::labelled(
        StateKind::Locked,
        look(LOCKED, "lock", Tone::Warning),
        Some(action),
        vec![LOCKED_SENTENCE.to_owned()],
    ))
}

/// The stored outcome, when `status::stored_applies` lets it count.
fn stored_outcome(facts: &RepoFacts<'_>) -> Option<PollingOutcome> {
    status::stored_applies(&facts.polling_facts(&facts.entry.name()))
}

/// D6's words for the row, and its detail when present.
fn row_sentence(facts: &RepoFacts<'_>) -> Vec<String> {
    let row = facts.row();
    let mut sentence = vec![row.status];
    sentence.extend(row.detail);
    sentence
}

/// The transport failure the last attempt ended in, while it still speaks:
/// checking is on and not paused, as for a stored outcome. A failed check
/// outlives pausing and switching off, and the state then reads what pausing
/// or switching off says (owner ruling, 2026-11-12).
fn last_answer<'a>(facts: &RepoFacts<'a>) -> Option<&'a TransportFailure> {
    let schedule = facts.schedule?;
    if !schedule.policy.enabled || schedule.policy.paused {
        return None;
    }
    match schedule.last.as_ref().map(|last| &last.answer) {
        Some(Answer::Transport(failure)) => Some(failure),
        _ => None,
    }
}

/// Row 6.
fn needs_attention(facts: &RepoFacts<'_>, repo: &str) -> Option<RepoState> {
    let (action, sentence) = attention_cause(facts, repo)?;
    Some(RepoState::labelled(
        StateKind::NeedsAttention,
        look(NEEDS_ATTENTION, "circle-alert", Tone::Failure),
        action,
        sentence,
    ))
}

/// What holds row 6, the first cause that does: its action and its words.
/// A pending conflict and a check's problems have no action (Activity has
/// them).
fn attention_cause(facts: &RepoFacts<'_>, repo: &str) -> Option<(Option<RowAction>, Vec<String>)> {
    use PollingOutcome::*;
    let schedule = facts.schedule;
    let attend = |reason: Reason| Some(RowAction::Attend(action_of(&reason), reason));
    if let Some(Phase::Attention(reason)) = schedule.map(|schedule| &schedule.phase) {
        return Some((attend(reason.clone()), row_sentence(facts)));
    }
    if facts.row().shape == Shape::Held {
        // Held by the policy, with no reason in the phase: Resume checking
        // releases the hold for this repository.
        let held = Reason::RecoveryHeld(facts.entry.key().clone());
        return Some((attend(held), row_sentence(facts)));
    }
    if schedule.is_some_and(|schedule| schedule.conflict_pending) {
        return Some((None, row_sentence(facts)));
    }
    let failed_wait = matches!(
        schedule.map(|schedule| &schedule.phase),
        Some(Phase::Waiting {
            why: Why::Failed,
            ..
        })
    );
    if failed_wait {
        return Some((Some(RowAction::RetryNow), row_sentence(facts)));
    }
    if facts.unplaced == Some(Unplaced::Unreadable) {
        return Some((Some(RowAction::Reread), vec![unreadable_sentence(repo)]));
    }
    if matches!(
        last_answer(facts),
        Some(TransportFailure::Unavailable | TransportFailure::ConfigurationRequired)
    ) {
        return Some((Some(RowAction::RetryNow), row_sentence(facts)));
    }
    let stored = match stored_outcome(facts) {
        Some(SelectedKeyUnavailable | HostApprovalRequired) => Some(RowAction::CheckNow),
        Some(ConfigurationRequired | RepositoryUnavailable) => Some(RowAction::RetryNow),
        _ => None,
    };
    if stored.is_some() {
        return Some((stored, row_sentence(facts)));
    }
    let problems = schedule?.unshared.problems.len();
    (problems > 0).then(|| (None, vec![problems_sentence(problems)]))
}

/// Rows 7 to 12.
fn rest(facts: &RepoFacts<'_>) -> Option<RepoState> {
    // A repository waiting for an unfinished operation shows no label and
    // D6's sentence, as Settings does (R-34).
    let unfinished = matches!(
        facts.schedule.map(|schedule| &schedule.phase),
        Some(Phase::Waiting {
            why: Why::Unfinished,
            ..
        })
    );
    if let Some(schedule) = facts.schedule {
        if unfinished {
            return None;
        }
        return scheduled(facts, schedule);
    }
    if !facts.polling || facts.unplaced.is_some() {
        return None;
    }
    // Polling on, no schedule, not unplaced: no SSH shared copy.
    match &facts.entry.summary.configuration.publication_remote {
        None => Some(RepoState::labelled(
            StateKind::ThisComputer,
            look(THIS_COMPUTER, "hard-drive", Tone::Secondary),
            Some(RowAction::SetUpSharing),
            vec![THIS_COMPUTER_SENTENCE.to_owned()],
        )),
        Some(remote) => Some(RepoState::plain(
            StateKind::NotSsh,
            Some(RowAction::SetUpSharing),
            vec![not_ssh_sentence(remote)],
        )),
    }
}

/// Rows 7 to 10, for a repository with a schedule.
fn scheduled(facts: &RepoFacts<'_>, schedule: &RepoSchedule) -> Option<RepoState> {
    let unshared = &schedule.unshared;
    let unreachable = matches!(last_answer(facts), Some(TransportFailure::Unreachable))
        || matches!(
            stored_outcome(facts),
            Some(PollingOutcome::TransportUnavailable | PollingOutcome::ProtocolRejected)
        );
    if unreachable {
        return Some(RepoState::labelled(
            StateKind::CantReach,
            look(CANT_REACH, "circle-alert", Tone::Failure),
            Some(RowAction::RetryNow),
            vec![facts.row().status],
        ));
    }
    if !unshared.items.is_empty() {
        return Some(RepoState::labelled(
            StateKind::ToShare(unshared.items.len()),
            look(
                &to_share(unshared.items.len()),
                "cloud-upload",
                Tone::Warning,
            ),
            None,
            to_share_sentence(facts, &unshared.items),
        ));
    }
    if schedule.policy.paused {
        return Some(RepoState::labelled(
            StateKind::Paused,
            look(PAUSED, "pause", Tone::Muted),
            Some(RowAction::CheckNow),
            vec![PAUSED_SENTENCE.to_owned()],
        ));
    }
    let as_of = unshared.as_of?;
    Some(RepoState::labelled(
        StateKind::UpToDate,
        look(UP_TO_DATE, "circle-check", Tone::Success),
        None,
        vec![up_to_date_sentence(&ago(as_of, facts.now))],
    ))
}

/// Row 13: no label, and D6's sentence, or nothing while polling is off.
fn unknown(facts: &RepoFacts<'_>) -> RepoState {
    let sentence = if facts.polling {
        vec![facts.row().status]
    } else {
        Vec::new()
    };
    RepoState::plain(StateKind::Unknown, None, sentence)
}

/// The tooltip, and with " " the row's accessible description: the state's
/// sentence, the folder, the main branch when known and the shared copy
/// when there is one. Nothing else (P7).
pub(crate) fn repository_tooltip(facts: &RepoFacts<'_>, state: &RepoState) -> Vec<String> {
    let mut lines = Vec::new();
    let sentence = sentence_line(state);
    if !sentence.is_empty() {
        lines.push(sentence);
    }
    lines.push(folder_line(&facts.entry.summary.root));
    lines.extend(
        facts
            .entry
            .summary
            .configuration
            .primary_branch
            .as_deref()
            .map(main_branch_line),
    );
    lines.extend(facts.shared_copy.map(shared_copy_line));
    lines
}

#[cfg(test)]
#[path = "repository_state_tests.rs"]
mod tests;
