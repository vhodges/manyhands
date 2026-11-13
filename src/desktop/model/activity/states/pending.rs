//! S9 and S10: the operations each listed repository reports pending, and
//! a list that could not be read. The worker's own running work is no
//! pending row (N21): `pending_shown` leaves it out.

use manyhands::{
    canonical::ItemId,
    repository::{AuthoringKind, OperationAction, OperationDto},
};

use super::{StateInputs, item_label, name_of, primary_of};
use crate::desktop::model::activity::copy::REVIEW_CHANGES;
use crate::desktop::model::{
    activity::{
        copy::{
            ABANDON, NO_OPERATION, RESUME, action_words, next_action_sentence, pending_list_busy,
            pending_list_failed, pending_title,
        },
        lifecycle_lines::STILL_RUNNING,
        rows::{ActivityAction, Derived, Group, ItemRef, Kind, Offer, RowKey},
    },
    list_state::ListState,
    list_view::RETRY,
    operations::{LifecycleRow, ResumeOffer, abandon_offered, resume_offer_among},
    stamp::RepoKey,
};

/// S9: a row for each pending operation of each repository whose list is
/// read, in each list's order.
pub(super) fn rows(inputs: &StateInputs<'_>) -> Vec<Derived> {
    let own = inputs.operations.automatic_polls();
    let entries = inputs.operations.entered_operations();
    let mut rows = Vec::new();
    for (root, state) in inputs.operations.pending_shown() {
        let ListState::Ready { rows: list, .. } = &state else {
            continue;
        };
        for (at, operation) in list.items.iter().enumerate() {
            let resume = resume_offer_among(operation, own, &entries);
            rows.push(row(inputs, &root, at, operation, resume));
        }
    }
    rows
}

fn row(
    inputs: &StateInputs<'_>,
    root: &RepoKey,
    at: usize,
    operation: &OperationDto,
    resume: ResumeOffer,
) -> Derived {
    let id = operation.operation_id.clone();
    let repo = name_of(inputs, root);
    let words = action_words(operation.action, &primary_of(inputs, root));
    let item = pending_item(inputs, root, operation);
    let title = pending_title(&words, item.as_ref().map(|(_, name)| name.as_str()), &repo);
    let lifecycle = LifecycleRow::of(operation);
    let still_running = lifecycle.as_ref().is_some_and(|row| {
        row.item
            .parse::<ItemId>()
            .is_ok_and(|item| (inputs.lifecycle_running)(root, &item))
    });

    // Resume, then Review changes, then Abandon, as offered.
    let mut detail = detail_of(operation);
    let mut offers: Vec<Offer> = Vec::new();
    if let Some(row) = &lifecycle {
        let label = if still_running { STILL_RUNNING } else { RESUME };
        offers.push(Offer::new(
            ActivityAction::ResumeLifecycle {
                root: root.clone(),
                kind: row.kind,
                item: row.item.clone(),
                operation: row.operation.clone(),
            },
            label,
            !still_running,
        ));
    }
    if let Some(operation_id) = &id {
        let resume_action = ActivityAction::Resume {
            root: root.clone(),
            operation: operation_id.clone(),
        };
        match resume {
            ResumeOffer::Offered => offers.push(Offer::new(resume_action, RESUME, true)),
            ResumeOffer::Unavailable(why) => {
                offers.push(Offer::new(resume_action, RESUME, false));
                detail.push(why.to_owned());
            }
            ResumeOffer::NotOffered => {}
        }
        if (inputs.conflict_waits)(root, operation_id) {
            offers.push(Offer::new(
                ActivityAction::ReviewPending {
                    root: root.clone(),
                    operation: operation_id.clone(),
                },
                REVIEW_CHANGES,
                true,
            ));
        }
        if abandon_offered(operation) {
            offers.push(Offer::new(
                ActivityAction::Abandon {
                    root: root.clone(),
                    operation: operation_id.clone(),
                },
                ABANDON,
                true,
            ));
        }
    }
    // The button is the first offer that can be pressed and is not Abandon,
    // else the first that is not Abandon: a disabled Resume, or "Still
    // running…". The menu then holds it first.
    let risky = |offer: &Offer| matches!(offer.action, ActivityAction::Abandon { .. });
    let button = offers
        .iter()
        .position(|offer| offer.enabled && !risky(offer))
        .or_else(|| offers.iter().position(|offer| !risky(offer)))
        .map(|at| offers.remove(at));
    let menu: Vec<Offer> = button.iter().cloned().chain(offers).collect();

    let next = operation.next_action;
    let kind = if next.is_some() {
        Kind::Failure
    } else {
        Kind::Info
    };
    let key = RowKey::Pending {
        root: root.clone(),
        operation: id.clone().unwrap_or_else(|| format!("#{at}")),
    };
    Derived {
        description: Some(next_action_sentence(next).to_owned()),
        repo: Some(repo),
        button,
        menu,
        detail,
        item: item.map(|(item, _)| item),
        group: Group {
            // N12 counts nothing without a next action, and N25 nothing
            // while its lifecycle runs.
            candidate: next.is_some() && !still_running,
            running_operation: id.clone().filter(|_| still_running),
            operation: id,
            roots: vec![root.clone()],
            ..Group::default()
        },
        ..Derived::new(key, kind, title)
    }
}

/// The item a pending row is about, when its title is known, with the item
/// as a row names it: the kind comes from the action, else from whichever
/// kind the title is found under.
fn pending_item(
    inputs: &StateInputs<'_>,
    root: &RepoKey,
    operation: &OperationDto,
) -> Option<(ItemRef, String)> {
    let id: ItemId = operation.item_id.as_deref()?.parse().ok()?;
    let kinds: &[AuthoringKind] = match authoring_of(operation.action) {
        Some(AuthoringKind::Ticket) => &[AuthoringKind::Ticket],
        Some(AuthoringKind::Document) => &[AuthoringKind::Document],
        None => &[AuthoringKind::Ticket, AuthoringKind::Document],
    };
    kinds.iter().find_map(|&kind| {
        (inputs.title)(root, kind, &id)?;
        Some((
            ItemRef {
                root: root.clone(),
                kind,
                id: id.clone(),
            },
            item_label(inputs, root, kind, &id),
        ))
    })
}

/// A pending row's raw tooltip lines: its action, state, completed step,
/// operation, next action and time (B5).
fn detail_of(operation: &OperationDto) -> Vec<String> {
    let mut lines = vec![
        format!("Action: {}", operation.action.as_str()),
        format!("State: {}", operation.state),
    ];
    lines.extend(
        operation
            .completed_step
            .as_ref()
            .map(|step| format!("Completed step: {step}")),
    );
    lines.push(match &operation.operation_id {
        Some(id) => format!("Operation {id}"),
        None => NO_OPERATION.to_owned(),
    });
    lines.push(format!(
        "Next action: {}",
        operation.next_action.map_or("none", |next| next.as_str())
    ));
    lines.extend(
        operation
            .updated_at
            .as_ref()
            .map(|updated| format!("Updated {updated}")),
    );
    lines
}

/// S10: a row for a pending list that was Failed or Busy; Loading and Ready
/// make none here. Retry reads it again. Only a Failed list counts (N13).
pub(super) fn list_rows(inputs: &StateInputs<'_>) -> Vec<Derived> {
    inputs
        .operations
        .pending_shown()
        .into_iter()
        .filter_map(|(root, state)| {
            let repo = name_of(inputs, &root);
            let (kind, title, counted) = match state {
                ListState::Failed(_) => (Kind::Failure, pending_list_failed(&repo), true),
                ListState::Busy => (Kind::Info, pending_list_busy(&repo), false),
                ListState::Loading | ListState::Ready { .. } => return None,
            };
            let button = Offer::new(ActivityAction::RetryPendingList(root.clone()), RETRY, true);
            Some(Derived {
                repo: Some(repo),
                menu: vec![button.clone()],
                button: Some(button),
                group: Group {
                    candidate: counted,
                    roots: vec![root.clone()],
                    ..Group::default()
                },
                ..Derived::new(RowKey::PendingList(root), kind, title)
            })
        })
        .collect()
}

/// The kind of the item an action is about, when the action says.
fn authoring_of(action: OperationAction) -> Option<AuthoringKind> {
    match action {
        OperationAction::Close | OperationAction::SaveTicket => Some(AuthoringKind::Ticket),
        OperationAction::Promote | OperationAction::SaveDocument => Some(AuthoringKind::Document),
        _ => None,
    }
}
