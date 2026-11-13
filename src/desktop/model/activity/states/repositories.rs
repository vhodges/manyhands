//! S4: each listed repository's state (Cycle 03a's `repository_state`) as a
//! row, by its kind, its cause and, for Locked, its action (R-40). The
//! rows that need the person are attention rows; "This computer" and the
//! non-SSH remote are information.

use std::collections::{BTreeMap, BTreeSet};

use manyhands::canonical::ItemId;

use super::{StateInputs, menu_of};
use crate::desktop::model::{
    activity::{
        copy::{
            CANT_REACH_DESCRIPTION, LOCKED_DESCRIPTION, cant_open_title, cant_reach_title,
            description_of, locked_title, needs_attention_title, needs_fixing_title,
            no_shared_copy_title, repository_detail, title_of,
        },
        rows::{ActivityAction, Derived, Group, Kind, Offer, RowKey, Target},
    },
    polling::{Unshared, entry::RetireOffer},
    repository_state::{
        AttentionCause, RepoState, RowAction, StateKind, action_label, sentence_line,
    },
    stamp::RepoKey,
};

/// What the Workspace knows of one listed repository (A2): its state, with
/// the cause Needs attention holds for, and what the rows of S4 to S7 read.
pub(crate) struct RepositoryInput {
    pub root: RepoKey,
    pub name: String,
    pub state: RepoState,
    /// The shared copy's name only (B4, R-48).
    pub remote: Option<String>,
    /// A check of it is queued or running: the log keeps its row (L3).
    #[allow(dead_code)] // Used from Task 5
    pub checking: bool,
    /// The main branch, when known.
    pub primary: Option<String>,
    /// Its items not shared yet, and what needs a look.
    pub unshared: Unshared,
    /// Whether each unshared item's Share… can be pressed (S6).
    pub sharing: BTreeMap<Target, ShareState>,
    /// The items Retire… is offered for that are retiring now (S7).
    pub retiring: BTreeSet<ItemId>,
    /// `Scheduler::retire_offers`, which leaves out the items a running
    /// retirement loop will still take up (S7).
    pub retire_offers: Vec<RetireOffer>,
}

/// Whether an unshared item's share can be started now (S6): today's attempt
/// entry's three states.
#[allow(dead_code)] // Used from Task 5
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum ShareState {
    Ready,
    /// Its share is queued or running.
    Sharing,
    /// Its lifecycle holds it; today's lock words.
    Locked(String),
}

/// The rows of S4: a row for each state that needs the person, and for the
/// two that say no sharing is set up.
pub(super) fn rows(inputs: &StateInputs<'_>) -> Vec<Derived> {
    inputs.repositories.iter().filter_map(row).collect()
}

fn row(repository: &RepositoryInput) -> Option<Derived> {
    let state = &repository.state;
    let name = repository.name.as_str();
    // Kind, title, description and whether the row counts, by state.
    let (kind, title, description, counted) = match (state.kind, state.action.as_ref()) {
        (StateKind::CantOpen, _) => (
            Kind::Failure,
            cant_open_title(name),
            description_of(state),
            true,
        ),
        (StateKind::NeedsFixing, _) => (
            Kind::Failure,
            needs_fixing_title(name),
            description_of(state),
            true,
        ),
        (StateKind::CantReach, _) => (
            Kind::Failure,
            cant_reach_title(name),
            Some(CANT_REACH_DESCRIPTION.to_owned()),
            true,
        ),
        // A live locked key is its hold's (S3); a stored outcome is this row.
        (StateKind::Locked, Some(RowAction::CheckNow)) => (
            Kind::Locked,
            locked_title(name),
            Some(LOCKED_DESCRIPTION.to_owned()),
            true,
        ),
        (StateKind::NeedsAttention, _) => match state.cause {
            Some(
                AttentionCause::Held
                | AttentionCause::WaitingFailed
                | AttentionCause::Unreadable
                | AttentionCause::Transport
                | AttentionCause::Stored,
            ) => (
                Kind::Failure,
                needs_attention_title(name),
                description_of(state),
                true,
            ),
            // The phase's hold (S3), the conflict's own row and the problems
            // (S5) say these.
            _ => return None,
        },
        (StateKind::ThisComputer, _) => (Kind::Info, no_shared_copy_title(name), None, false),
        (StateKind::NotSsh, _) => (Kind::Info, title_of(&sentence_line(state)), None, false),
        // Checking… keeps its row through the log (L3); the rest need
        // nothing, or their facts are elsewhere (R-14).
        _ => return None,
    };
    let button = state.action.as_ref().map(|action| {
        Offer::new(
            ActivityAction::Repository(repository.root.clone(), action.clone()),
            action_label(action),
            true,
        )
    });
    Some(Derived {
        description,
        repo: Some(repository.name.clone()),
        menu: menu_of(button.as_ref(), Vec::new()),
        button,
        detail: repository_detail(repository.primary.as_deref(), repository.remote.as_deref()),
        group: Group {
            candidate: counted,
            roots: vec![repository.root.clone()],
            ..Group::default()
        },
        ..Derived::new(RowKey::Repository(repository.root.clone()), kind, title)
    })
}
