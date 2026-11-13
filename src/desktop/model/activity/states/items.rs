//! S5 to S7: what the unshared list says of a repository's items (R-17), and
//! the Retire… offers. Problems need a look and count; reminders and offers
//! are information.

use manyhands::repository::AuthoringKind;

use super::{RepositoryInput, ShareState, StateInputs, authoring, item_label, menu_of, primary_of};
use crate::desktop::model::{
    activity::{
        copy::{
            OPEN, SHARE_MAIN, SHARING, UNNAMED_ITEM, disposition_sentence, item_of_unknown_kind,
            main_in, problem_detail, problem_title, retire_offer_title, unshared_detail,
            unshared_title, upper_first,
        },
        rows::{ActivityAction, Derived, Group, ItemRef, Kind, Offer, RowKey, Target},
    },
    polling::{
        AttemptTarget,
        entry::{RETIRE, SHARE, short_code},
    },
    stamp::RepoKey,
};

/// A target of the unshared list, as a row's key names it: main, its item,
/// or its position among those with no ID.
pub(crate) fn target_of(target: &AttemptTarget, unnamed: &mut u32) -> Target {
    match (&target.item_id, target.main) {
        (_, true) => Target::Main,
        (Some(id), false) => Target::Item(id.clone()),
        (None, false) => {
            let at = *unnamed;
            *unnamed += 1;
            Target::Unnamed(at)
        }
    }
}

/// What a target is called in a title: main as "{primary} in {repo}", an
/// item by B1's rule, an item with no kind by its short code, an item with no
/// ID as "an item". The first letter of a name is upper case at the start
/// of the title.
fn subject(
    inputs: &StateInputs<'_>,
    repository: &RepositoryInput,
    target: &AttemptTarget,
) -> String {
    if target.main {
        return main_in(&primary_of(inputs, &repository.root), &repository.name);
    }
    let name = match (&target.item_id, target.kind) {
        (Some(id), Some(kind)) => item_label(inputs, &repository.root, authoring(kind), id),
        (Some(id), None) => item_of_unknown_kind(&short_code(id)),
        (None, _) => UNNAMED_ITEM.to_owned(),
    };
    upper_first(&name)
}

/// The item a target is about, when it has an ID and a kind.
fn item_of(root: &RepoKey, target: &AttemptTarget) -> Option<ItemRef> {
    Some(ItemRef {
        root: root.clone(),
        kind: authoring(target.kind?),
        id: target.item_id.clone()?,
    })
}

/// S5: a problem row for each problem of the unshared list. Open takes the
/// person to an item that has an ID and a kind.
pub(super) fn problem_rows(inputs: &StateInputs<'_>) -> Vec<Derived> {
    let mut rows = Vec::new();
    for repository in inputs.repositories {
        let mut unnamed = 0;
        for target in &repository.unshared.problems {
            let key = RowKey::Problem {
                root: repository.root.clone(),
                target: target_of(target, &mut unnamed),
            };
            let item = item_of(&repository.root, target);
            let button = item
                .clone()
                .map(|item| Offer::new(ActivityAction::OpenItem(item), OPEN, true));
            rows.push(Derived {
                description: disposition_sentence(target.disposition).map(str::to_owned),
                repo: Some(repository.name.clone()),
                menu: menu_of(button.as_ref(), Vec::new()),
                button,
                detail: vec![problem_detail(target.disposition)],
                item,
                group: Group {
                    candidate: true,
                    roots: vec![repository.root.clone()],
                    ..Group::default()
                },
                ..Derived::new(
                    key,
                    Kind::Failure,
                    problem_title(&subject(inputs, repository, target)),
                )
            });
        }
    }
    rows
}

/// S6: a reminder for each item not shared yet, main among them. Share… is
/// disabled while the item's share runs, reading "Sharing…", or while its
/// lifecycle holds it, with today's lock words in the detail (never counted).
pub(super) fn unshared_rows(inputs: &StateInputs<'_>) -> Vec<Derived> {
    let mut rows = Vec::new();
    for repository in inputs.repositories {
        let mut unnamed = 0;
        for target in &repository.unshared.items {
            let at = target_of(target, &mut unnamed);
            let state = repository.sharing.get(&at).unwrap_or(&ShareState::Ready);
            let key = RowKey::Unshared {
                root: repository.root.clone(),
                target: at,
            };
            let item = item_of(&repository.root, target);
            let action = if target.main {
                Some((
                    ActivityAction::Share {
                        root: repository.root.clone(),
                        item: None,
                    },
                    SHARE_MAIN,
                ))
            } else {
                item.as_ref().map(|item| {
                    (
                        ActivityAction::Share {
                            root: repository.root.clone(),
                            item: Some((item.kind, item.id.clone())),
                        },
                        SHARE,
                    )
                })
            };
            let button = action.map(|(action, label)| match state {
                ShareState::Ready => Offer::new(action, label, true),
                ShareState::Sharing => Offer::new(action, SHARING, false),
                ShareState::Locked(_) => Offer::new(action, label, false),
            });
            let mut detail = vec![unshared_detail(target.disposition)];
            if let ShareState::Locked(words) = state {
                detail.push(words.clone());
            }
            rows.push(Derived {
                repo: Some(repository.name.clone()),
                menu: menu_of(button.as_ref(), Vec::new()),
                button,
                detail,
                item,
                group: Group {
                    roots: vec![repository.root.clone()],
                    ..Group::default()
                },
                ..Derived::new(
                    key,
                    Kind::ToShare,
                    unshared_title(&subject(inputs, repository, target)),
                )
            });
        }
    }
    rows
}

/// S7: a Retire… offer for each item whose copy is left: a closed ticket or
/// a promoted document. Disabled while the item is retiring.
pub(super) fn retire_offer_rows(inputs: &StateInputs<'_>) -> Vec<Derived> {
    let mut rows = Vec::new();
    for repository in inputs.repositories {
        for offer in &repository.retire_offers {
            let kind = authoring(offer.kind);
            let name = upper_first(&item_label(inputs, &repository.root, kind, &offer.item));
            let button = Offer::new(
                ActivityAction::Retire {
                    root: repository.root.clone(),
                    kind: offer.kind,
                    item: offer.item.clone(),
                },
                RETIRE,
                !repository.retiring.contains(&offer.item),
            );
            rows.push(Derived {
                repo: Some(repository.name.clone()),
                menu: menu_of(Some(&button), Vec::new()),
                button: Some(button),
                item: Some(ItemRef {
                    root: repository.root.clone(),
                    kind,
                    id: offer.item.clone(),
                }),
                group: Group {
                    roots: vec![repository.root.clone()],
                    ..Group::default()
                },
                ..Derived::new(
                    RowKey::RetireOffer {
                        root: repository.root.clone(),
                        item: offer.item.clone(),
                    },
                    Kind::Info,
                    retire_offer_title(&name, kind == AuthoringKind::Ticket),
                )
            });
        }
    }
    rows
}
