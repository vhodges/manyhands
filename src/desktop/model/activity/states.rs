//! S2-S14: the rows of everything that is not a request (Wave 04 Cycle 03b,
//! design A2, B4): the checks that brought something in, the checking
//! holds, each repository's state, the unshared list's problems and items,
//! the Retire… offers and events, the pending operations, and the drafts and
//! record notes. A pure function of the inputs the Workspace builds at each
//! refresh; the model reads nothing else. A source leaves `attention` to the
//! log.

mod drafts;
mod items;
mod pending;
mod repositories;
mod retired;

use std::path::Path;

use manyhands::{canonical::ItemId, repository::AuthoringKind};

pub(crate) use self::{
    drafts::DraftsInput,
    repositories::{RepositoryInput, ShareState},
};
use super::{
    copy::{
        DISMISS, UNLOCK_UNAVAILABLE, brought_description, brought_title, checked_title, hold_words,
        item_name, scope_name,
    },
    rows::{ActivityAction, Derived, Kind, Offer, RowKey},
};
use crate::desktop::model::polling::KeyId;
use crate::desktop::model::{
    operations::Operations,
    polling::{
        PollSchedule, Reason, TargetKind,
        attention::{Action, action_of, headline, holding_back, locked_key},
    },
    repository_state::MAIN_NAME,
    stamp::RepoKey,
};

/// The state half of `Inputs`, which `state_rows` reads: the models, the
/// repositories' facts, and the lookups the Workspace supplies.
#[allow(dead_code)] // Used from Task 5
pub(crate) struct StateInputs<'a> {
    pub operations: &'a Operations,
    pub schedule: &'a PollSchedule,
    pub repositories: &'a [RepositoryInput],
    pub drafts: &'a DraftsInput,
    /// A pending conflict waits on the operation of the repository: its
    /// pending row offers Review changes.
    pub conflict_waits: &'a dyn Fn(&RepoKey, &str) -> bool,
    /// An item's title as read, never a draft's own (B1).
    pub title: &'a dyn Fn(&RepoKey, AuthoringKind, &ItemId) -> Option<String>,
    /// A repository's main branch, when known.
    pub primary: &'a dyn Fn(&RepoKey) -> Option<String>,
    /// An Unlock… has a key to ask for.
    pub unlock_target: &'a dyn Fn(&KeyId) -> bool,
    /// The item's close or promotion is queued or running elsewhere.
    pub lifecycle_running: &'a dyn Fn(&RepoKey, &ItemId) -> bool,
}

/// The rows of S2 to S14, in S order and, within a source, the source's own.
#[allow(dead_code)] // Used from Task 5
pub(crate) fn state_rows(inputs: &StateInputs<'_>) -> Vec<Derived> {
    let mut rows = brought_rows(inputs);
    rows.extend(hold_rows(inputs));
    rows.extend(repositories::rows(inputs));
    rows.extend(items::problem_rows(inputs));
    rows.extend(items::unshared_rows(inputs));
    rows.extend(items::retire_offer_rows(inputs));
    rows.extend(retired::rows(inputs));
    rows.extend(pending::rows(inputs));
    rows.extend(pending::list_rows(inputs));
    rows.extend(drafts::rows(inputs));
    rows
}

/// A hold's key (R-3): the variant's name and its key ID, authority or root.
/// `Reason` has no order, so the row is keyed by this string.
pub(crate) fn hold_key(reason: &Reason) -> String {
    match reason {
        Reason::KeyLocked(key) => format!("KeyLocked:{key}"),
        Reason::KeyProblem(key) => format!("KeyProblem:{key}"),
        Reason::NoKeySelected(root) => format!("NoKeySelected:{root}"),
        Reason::HostUnapproved(authority) => {
            format!("HostUnapproved:{}:{}", authority.host, authority.port)
        }
        Reason::HostUnidentified(root) => format!("HostUnidentified:{root}"),
        Reason::ConfigurationRequired(root) => format!("ConfigurationRequired:{root}"),
        Reason::RecoveryHeld(root) => format!("RecoveryHeld:{root}"),
    }
}

/// A repository's name: the listed one's, else its folder's.
fn name_of(inputs: &StateInputs<'_>, root: &RepoKey) -> String {
    inputs
        .repositories
        .iter()
        .find(|repository| &repository.root == root)
        .map_or_else(
            || scope_name(Some(Path::new(root))),
            |repository| repository.name.clone(),
        )
}

/// A repository's main branch as a row names it (B1): the branch when
/// known, else "main".
fn primary_of(inputs: &StateInputs<'_>, root: &RepoKey) -> String {
    (inputs.primary)(root).unwrap_or_else(|| MAIN_NAME.to_owned())
}

/// An item as a row names it (B1): its title as read, else its short code.
fn item_label(
    inputs: &StateInputs<'_>,
    root: &RepoKey,
    kind: AuthoringKind,
    id: &ItemId,
) -> String {
    item_name((inputs.title)(root, kind, id).as_deref(), kind, id)
}

fn authoring(kind: TargetKind) -> AuthoringKind {
    match kind {
        TargetKind::Ticket => AuthoringKind::Ticket,
        TargetKind::Document => AuthoringKind::Document,
    }
}

/// The menu of a row with `button`: its offer first, then `rest` in order.
fn menu_of(button: Option<&Offer>, rest: Vec<Offer>) -> Vec<Offer> {
    button.cloned().into_iter().chain(rest).collect()
}

fn dismiss(key: &RowKey) -> Offer {
    Offer::new(ActivityAction::Dismiss(key.clone()), DISMISS, true)
}

/// S2: each check that brought something in, from the schedule's own list.
fn brought_rows(inputs: &StateInputs<'_>) -> Vec<Derived> {
    inputs
        .schedule
        .brought()
        .iter()
        .map(|brought| {
            let repo = name_of(inputs, &brought.root);
            let key = RowKey::Brought {
                root: brought.root.clone(),
                seq: brought.seq,
            };
            // A Check now's request row already says "Checked {repo} for
            // updates", so its brought-in row says what came in (A5).
            let (title, description) = if brought.automatic {
                (
                    checked_title(&repo),
                    Some(brought_description(brought.count)),
                )
            } else {
                (brought_title(brought.count, &repo), None)
            };
            Derived {
                description,
                repo: Some(repo),
                menu: vec![dismiss(&key)],
                clearable: true,
                at: Some(brought.at),
                ..Derived::new(key, Kind::Sharing, title)
            }
        })
        .collect()
}

/// S3: each checking hold, one for each key, host or repository the
/// scheduler holds back. Unlock… is disabled, and says why, when no key
/// target remains.
fn hold_rows(inputs: &StateInputs<'_>) -> Vec<Derived> {
    inputs
        .schedule
        .attention()
        .into_iter()
        .filter_map(|entry| {
            let first = entry.roots.first()?.clone();
            let names: Vec<String> = entry
                .roots
                .iter()
                .map(|root| name_of(inputs, root))
                .collect();
            let (title, description) = hold_words(&entry.reason, &names.join(", "));
            let action = action_of(&entry.reason);
            let reachable = locked_key(&entry.reason)
                .filter(|_| action == Action::Unlock)
                .is_none_or(|key| (inputs.unlock_target)(&key));
            let mut detail = vec![headline(&entry.reason).to_owned(), holding_back(&names)];
            if !reachable {
                detail.push(UNLOCK_UNAVAILABLE.to_owned());
            }
            let button = Offer::new(
                ActivityAction::Attend(action, entry.reason.clone(), first),
                action.label(),
                reachable,
            );
            let kind = match entry.reason {
                Reason::KeyLocked(_) => Kind::Locked,
                _ => Kind::Failure,
            };
            let tag = match names.len() {
                1 => names[0].clone(),
                more => format!("{} +{}", names[0], more - 1),
            };
            Some(Derived {
                description: Some(description),
                repo: Some(tag),
                menu: menu_of(Some(&button), Vec::new()),
                button: Some(button),
                detail,
                group: super::rows::Group {
                    candidate: true,
                    roots: entry.roots.clone(),
                    ..Default::default()
                },
                ..Derived::new(RowKey::Hold(hold_key(&entry.reason)), kind, title)
            })
        })
        .collect()
}
