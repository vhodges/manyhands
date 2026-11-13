//! Activity's log in the Workspace (design A4): the rows are derived from the
//! models at each refresh point, never from render, and handed to the log,
//! which gives them times, keeps what it must and says what counts. A burst
//! of notifications makes one refresh per effect cycle (R-43).

use std::collections::{BTreeMap, BTreeSet, HashMap};

use gpui_kit::{App, Context, Entity, EntityId};
use manyhands::{
    canonical::ItemId,
    drafts::ItemKind,
    repository::{AuthoringKind, ItemDtoKind},
};
use time::OffsetDateTime;

use super::Workspace;
use crate::desktop::{
    model::{
        activity::{
            Inputs, derive,
            log::{ActivityLog, pending_held},
            rows::{ItemRef, RowKey, Target},
            states::{DraftsInput, RepositoryInput, ShareState, target_of},
        },
        lifecycle_lock::LockKey,
        polling::{KeyId, TargetKind},
        repository_state::{RepoFacts, repository_state},
        stamp::RepoKey,
        tabs::TabKey,
    },
    services::{lifecycle_of, requests::Intent},
    view::{
        item::{ItemPane, locked_text},
        repositories::each_repository,
    },
};

/// What the Workspace keeps for Activity: the log, whether a refresh waits
/// for the end of the effect cycle, and the title each open item pane last
/// had, so a pane's other changes refresh nothing.
#[derive(Default)]
pub(in crate::desktop::view) struct ActivityState {
    log: ActivityLog,
    dirty: bool,
    titles: HashMap<EntityId, Option<String>>,
    #[cfg(test)]
    derivations: usize,
}

fn dto_kind(kind: AuthoringKind) -> ItemDtoKind {
    match kind {
        AuthoringKind::Ticket => ItemDtoKind::Ticket,
        AuthoringKind::Document => ItemDtoKind::Document,
    }
}

fn authoring(kind: ItemKind) -> AuthoringKind {
    match kind {
        ItemKind::Ticket => AuthoringKind::Ticket,
        ItemKind::Document => AuthoringKind::Document,
    }
}

impl Workspace {
    /// The log: the rows Activity shows, and what Needs attention counts.
    pub(crate) fn activity(&self) -> &ActivityLog {
        &self.activity.log
    }

    /// Marks the log dirty and defers one refresh to the end of the effect
    /// cycle, so a burst of notifications derives once (R-43).
    pub(in crate::desktop::view) fn refresh_activity(&mut self, cx: &mut Context<Self>) {
        if std::mem::replace(&mut self.activity.dirty, true) {
            return;
        }
        let this = cx.weak_entity();
        cx.defer(move |cx| {
            this.update(cx, |this, cx| this.refresh_activity_now(cx))
                .ok();
        });
    }

    /// S2's and S8's Dismiss (L6): the log forgets the row, and the next
    /// refresh does not show it again. A request goes through
    /// `dismiss_request`.
    pub(in crate::desktop::view) fn dismiss_activity_row(
        &mut self,
        key: &RowKey,
        cx: &mut Context<Self>,
    ) {
        self.activity.log.dismiss(key);
        self.refresh_activity(cx);
    }

    /// Derives the rows and refreshes the log now: the deferred refresh, and
    /// the test seam's synchronous one. The Workspace is notified only when
    /// a row changed.
    pub(in crate::desktop::view) fn refresh_activity_now(&mut self, cx: &mut Context<Self>) {
        self.activity.dirty = false;
        let (derived, checking, held, now) = self.activity_inputs(cx);
        #[cfg(test)]
        {
            self.activity.derivations += 1;
        }
        if self.activity.log.refresh(derived, &checking, &held, now) {
            cx.notify();
        }
    }

    /// How many times the rows were derived (T10).
    #[cfg(test)]
    pub(in crate::desktop::view) fn activity_derivations(&self) -> usize {
        self.activity.derivations
    }

    /// An open item pane changed: its title may have loaded.
    pub(in crate::desktop::view) fn pane_titled(
        &mut self,
        pane: &Entity<ItemPane>,
        cx: &mut Context<Self>,
    ) {
        let title = pane.read(cx).model.last_title().map(str::to_owned);
        if self.activity.titles.get(&pane.entity_id()) != Some(&title) {
            self.activity.titles.insert(pane.entity_id(), title);
            self.refresh_activity(cx);
        }
    }

    /// The rows now, with L3's checking set and L3b's pending-held set (both
    /// over the listed repositories only) and the clock's time.
    fn activity_inputs(
        &self,
        cx: &App,
    ) -> (
        Vec<crate::desktop::model::activity::rows::Derived>,
        BTreeSet<RepoKey>,
        BTreeSet<RepoKey>,
        OffsetDateTime,
    ) {
        let services = &self.services;
        let operations = services.operations().read(cx);
        let scheduler = services.scheduler().read(cx);
        let repositories: Vec<RepositoryInput> =
            each_repository(self, cx, |facts| self.repository_input(facts, cx));
        let listed: Vec<RepoKey> = self.listed().unwrap_or_default();
        let checking: BTreeSet<RepoKey> = repositories
            .iter()
            .filter(|repository| repository.checking && listed.contains(&repository.root))
            .map(|repository| repository.root.clone())
            .collect();
        let held = pending_held(operations, &listed);
        let drafts = {
            let registry = services.drafts().read(cx);
            DraftsInput {
                flush_failures: registry
                    .flush_failure_items()
                    .into_iter()
                    .map(|(draft, target, title, kind)| {
                        let item = ItemRef {
                            root: target.repository.clone(),
                            kind: authoring(target.item_kind),
                            id: target.item_id.clone(),
                        };
                        (draft, item, title, kind)
                    })
                    .collect(),
                unavailable: registry.unavailable(),
                recovered: registry.recovered(),
            }
        };
        // The pending operations of each item's close or promotion: a
        // resume in flight for one is the item's lifecycle running (S9; T4-3).
        let lifecycles: Vec<(RepoKey, String, String)> = operations
            .pending_shown()
            .into_iter()
            .filter_map(|(root, state)| Some((root, state.rows()?.items.clone())))
            .flat_map(|(root, items)| {
                items.into_iter().filter_map(move |operation| {
                    crate::desktop::model::operations::LifecycleRow::of(&operation)
                        .map(|row| (root.clone(), row.item, row.operation))
                })
            })
            .collect();
        let requests = |id| self.request_facts(id, cx);
        let waits =
            |root: &RepoKey, operation: &str| self.pending_conflict(root, operation).is_some();
        let title = |root: &RepoKey, kind: AuthoringKind, id: &ItemId| {
            self.activity_title(root, kind, id, cx)
        };
        let primary = |root: &RepoKey| {
            self.navigation()
                .entry(root)
                .and_then(|entry| entry.summary.configuration.primary_branch.clone())
        };
        let unlock = |key: &KeyId| scheduler.unlock_target(*key, operations).is_some();
        let running = |root: &RepoKey, item: &ItemId| {
            let key = LockKey::new(root, &item.to_string());
            let mut held = operations.locks().operations(&key);
            held.extend(
                lifecycles
                    .iter()
                    .filter(|(held_root, held_item, _)| held_root == root && *held_item == key.id)
                    .map(|(_, _, operation)| operation.clone()),
            );
            services
                .worker()
                .in_flight(|intent| lifecycle_of(intent, &key, &held))
        };
        let derived = derive(&Inputs {
            operations,
            schedule: scheduler.schedule(),
            repositories: &repositories,
            drafts,
            requests: &requests,
            conflict_waits: &waits,
            title: &title,
            primary: &primary,
            unlock_target: &unlock,
            lifecycle_running: &running,
        });
        (derived, checking, held, scheduler.utc())
    }

    /// An item's title as a row may name it (B1): the open pane's last read,
    /// else what a list read of the repository gave. Never a draft's own.
    fn activity_title(
        &self,
        root: &RepoKey,
        kind: AuthoringKind,
        id: &ItemId,
        cx: &App,
    ) -> Option<String> {
        let key = TabKey::Item {
            repo: root.clone(),
            kind: dto_kind(kind),
            id: id.to_string(),
        };
        self.tab_of(&key)
            .and_then(|tab| self.content.tabs.pane(tab))
            .and_then(|pane| pane.read(cx).model.last_title().map(str::to_owned))
            .or_else(|| self.share_title(root, id))
    }

    /// One listed repository as Activity reads it (A7): its state with its
    /// cause, its name, its shared copy's name, whether a check of it runs,
    /// and what S4 to S7 read of its unshared list.
    fn repository_input(&self, facts: &RepoFacts<'_>, cx: &App) -> RepositoryInput {
        let services = &self.services;
        let root = facts.entry.key().clone();
        let unshared = facts
            .schedule
            .map(|schedule| schedule.unshared.clone())
            .unwrap_or_default();
        let mut unnamed = 0;
        let sharing: BTreeMap<Target, ShareState> = unshared
            .items
            .iter()
            .map(|target| {
                let key = target_of(target, &mut unnamed);
                let state = match (&target.item_id, target.kind) {
                    _ if target.main => {
                        if self.sharing_main(&root) {
                            ShareState::Sharing
                        } else {
                            ShareState::Ready
                        }
                    }
                    (Some(id), Some(kind)) => {
                        let dto = match kind {
                            TargetKind::Ticket => ItemDtoKind::Ticket,
                            TargetKind::Document => ItemDtoKind::Document,
                        };
                        let lock = services.lock(&root, dto, &id.to_string(), cx);
                        if lock.blocks() {
                            ShareState::Locked(locked_text(lock, dto).to_owned())
                        } else if self.item_sharing(&root, kind, id) {
                            ShareState::Sharing
                        } else {
                            ShareState::Ready
                        }
                    }
                    _ => ShareState::Ready,
                };
                (key, state)
            })
            .collect();
        let retire_offers = services.scheduler().read(cx).retire_offers(&root);
        let retiring = retire_offers
            .iter()
            .filter(|offer| {
                let dto = match offer.kind {
                    TargetKind::Ticket => ItemDtoKind::Ticket,
                    TargetKind::Document => ItemDtoKind::Document,
                };
                services.retiring(&root, dto, &offer.item.to_string(), cx)
            })
            .map(|offer| offer.item.clone())
            .collect();
        RepositoryInput {
            name: facts.entry.name(),
            state: repository_state(facts),
            remote: facts.shared_copy.map(|copy| copy.name.clone()),
            // A repository the list dropped is not checked for its row (L3).
            checking: facts.checking() && !facts.kept,
            primary: facts.entry.summary.configuration.primary_branch.clone(),
            unshared,
            sharing,
            retiring,
            retire_offers,
            root,
        }
    }

    /// Whether a share of the item is queued, waiting out `busy` or running:
    /// the worker's own account, as the item header's Share reads it.
    fn item_sharing(&self, root: &str, kind: TargetKind, id: &ItemId) -> bool {
        let kind = match kind {
            TargetKind::Ticket => AuthoringKind::Ticket,
            TargetKind::Document => AuthoringKind::Document,
        };
        self.services.worker().in_flight(|intent| {
            matches!(
                intent,
                Intent::ItemSync { root: asked, kind: asked_kind, id: asked_id }
                    if asked.to_str() == Some(root) && *asked_kind == kind && asked_id == id
            )
        })
    }
}
