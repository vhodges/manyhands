//! The log of Activity's rows (Wave 04 Cycle 03b, design A4): the time each
//! row was first seen, the rows the log keeps while a check or a read runs
//! (L3, L3b), the log-wide revisions (R-46), what was dismissed, and, last,
//! which rows count (A3, `attention.rs`). A pure model with an injected clock:
//! the Workspace derives the rows and hands them over at each refresh.

mod attention;

use std::collections::{BTreeMap, BTreeSet};

use time::OffsetDateTime;

use super::rows::{ActivityAction, ActivityRow, Derived, RowKey};
use crate::desktop::model::{list_state::ListState, operations::Operations, stamp::RepoKey};

/// What a kept row's button reads while its repository checks (L3).
pub(crate) const CHECKING: &str = "Checking\u{2026}";

/// Owned by the Workspace: the time each row was first seen, what was
/// dismissed (with the attempt of a retirement note, T18), the latches of N22
/// and N23 (`settled`, `absorbed`, per attempt; R-56), the log-wide revision
/// counter and the current rows.
#[derive(Default)]
pub(crate) struct ActivityLog {
    seen: BTreeMap<RowKey, OffsetDateTime>,
    /// A dismissed row's key, and `Group.sent` of the row at the time, which
    /// tells a retirement note replaced by a later event (T18).
    dismissed: BTreeMap<RowKey, Option<u64>>,
    /// A request key mapped to the attempt at which N22 or N23 settled it.
    settled: BTreeMap<RowKey, u64>,
    /// For each failed request, the rows that absorbed it at a refresh of its
    /// current attempt (N1, N4, N18, N19).
    absorbed: BTreeMap<RowKey, BTreeSet<RowKey>>,
    /// For each failed Check now, whether a row L3 keeps covered its root
    /// when it was first evaluated this attempt: only such a failure is
    /// absorbed by a kept row (N19; C9).
    first_kept: BTreeMap<RowKey, bool>,
    revisions: u64,
    rows: Vec<ActivityRow>,
    /// The rows this refresh kept (L3, L3b): N19's covering row is one.
    kept: BTreeSet<RowKey>,
}

/// The repositories L3b's trigger names: a listed repository whose pending
/// list is Busy, or has none at all because a failed read with an unavailable
/// code removed it. Its pending rows are kept (R-54). Built over the listed
/// repositories only, so a repository that leaves the list leaves the set.
pub(crate) fn pending_held(operations: &Operations, listed: &[RepoKey]) -> BTreeSet<RepoKey> {
    listed
        .iter()
        .filter(|root| match operations.pending().get(*root) {
            None | Some(ListState::Busy) => true,
            Some(ListState::Loading | ListState::Failed(_) | ListState::Ready { .. }) => false,
        })
        .cloned()
        .collect()
}

/// Two offers say the same thing to a person who chose one.
fn same_offers(left: &Derived, right: &Derived) -> bool {
    left.button == right.button && left.menu == right.menu
}

/// A row kept while its repository checks: the button reads "Checking…" and
/// every offer is disabled, Dismiss excepted (L3).
fn checking_row(row: &Derived) -> Derived {
    let mut kept = row.clone();
    if let Some(button) = kept.button.as_mut() {
        button.label = CHECKING.to_owned();
        button.enabled = false;
    }
    for offer in &mut kept.menu {
        if matches!(offer.action, ActivityAction::Dismiss(_)) {
            continue;
        }
        if row
            .button
            .as_ref()
            .is_some_and(|button| button.action == offer.action)
        {
            offer.label = CHECKING.to_owned();
        }
        offer.enabled = false;
    }
    kept
}

/// A pending row kept while its list is unread: every offer disabled (L3b).
fn unread_row(row: &Derived) -> Derived {
    let mut kept = row.clone();
    for offer in kept.button.iter_mut().chain(kept.menu.iter_mut()) {
        offer.enabled = false;
    }
    kept
}

impl ActivityLog {
    /// Applies L1 to L7 to the derived rows, then assigns attention (A3) and
    /// settles `clearable` over the final rows. True when a row changed.
    pub(crate) fn refresh(
        &mut self,
        derived: Vec<Derived>,
        checking: &BTreeSet<RepoKey>,
        pending_held: &BTreeSet<RepoKey>,
        now: OffsetDateTime,
    ) -> bool {
        let before = std::mem::take(&mut self.rows);
        let prior: BTreeMap<&RowKey, &ActivityRow> =
            before.iter().map(|row| (&row.row.key, row)).collect();
        let mut rows: Vec<ActivityRow> = Vec::new();
        let mut derived_keys: BTreeSet<RowKey> = BTreeSet::new();
        for row in derived {
            derived_keys.insert(row.key.clone());
            // L6: dismissed, unless a retirement note was replaced (T18).
            if let Some(then) = self.dismissed.get(&row.key) {
                let replaced =
                    matches!(row.key, RowKey::RetireNote { .. }) && *then != row.group.sent;
                if !replaced {
                    continue;
                }
                self.dismissed.remove(&row.key);
            }
            let before = prior.get(&row.key).copied();
            // L1: first seen now, unless the source says when; a retirement
            // note replaced by a later event is new again.
            let replaced = matches!(row.key, RowKey::RetireNote { .. })
                && before.is_some_and(|row_before| row_before.row.group.sent != row.group.sent);
            let at = match self.seen.get(&row.key) {
                Some(at) if !replaced => *at,
                _ => row.at.unwrap_or(now),
            };
            let revision = self.revision(before, &row);
            if before.is_some_and(|row_before| row_before.row.group.sent != row.group.sent) {
                // A retry or resend under the same ID is a new attempt.
                self.absorbed.remove(&row.key);
                self.first_kept.remove(&row.key);
            }
            rows.push(ActivityRow { at, revision, row });
        }
        // L3 and L3b: rows no longer derived that the log keeps.
        self.kept.clear();
        let mut kept_rows: Vec<(usize, ActivityRow)> = Vec::new();
        for (index, old) in before.iter().enumerate() {
            let key = &old.row.key;
            if derived_keys.contains(key) {
                continue;
            }
            let kept = match key {
                RowKey::Repository(root) if checking.contains(root) => Some(checking_row(&old.row)),
                RowKey::Pending { root, .. } if pending_held.contains(root) => {
                    Some(unread_row(&old.row))
                }
                _ => None,
            };
            let Some(row) = kept else { continue };
            let revision = if same_offers(&row, &old.row) {
                old.revision
            } else {
                self.revisions += 1;
                self.revisions
            };
            self.kept.insert(key.clone());
            kept_rows.push((
                index,
                ActivityRow {
                    at: old.at,
                    revision,
                    row,
                },
            ));
        }
        // A kept row goes back where it was, so equal times keep their order
        // (L7).
        for (index, row) in kept_rows {
            rows.insert(index.min(rows.len()), row);
        }
        // L7: newest first; equal times keep derivation order.
        rows.sort_by_key(|row| std::cmp::Reverse(row.at));
        // L2: a key that is no longer a row is forgotten, with what was
        // latched for it.
        self.seen = rows
            .iter()
            .map(|row| (row.row.key.clone(), row.at))
            .collect();
        let present: BTreeSet<&RowKey> = rows.iter().map(|row| &row.row.key).collect();
        self.settled.retain(|key, _| present.contains(key));
        self.absorbed.retain(|key, _| present.contains(key));
        self.first_kept.retain(|key, _| present.contains(key));
        for row in &rows {
            // A new attempt: what settled the last one does not settle it.
            if self
                .settled
                .get(&row.row.key)
                .is_some_and(|attempt| Some(*attempt) != row.row.group.sent)
            {
                self.settled.remove(&row.row.key);
            }
        }
        self.rows = rows;
        self.assign_attention();
        self.rows != before
    }

    /// A row's revision: the log-wide counter's next value when it is new, or
    /// when its kind, offers, attempt or outcome changed; its old one
    /// otherwise, so stage words and times never change it (R-46).
    fn revision(&mut self, before: Option<&ActivityRow>, row: &Derived) -> u64 {
        match before {
            Some(old)
                if old.row.kind == row.kind
                    && same_offers(&old.row, row)
                    && old.row.group.sent == row.group.sent
                    && old.row.group.outcome == row.group.outcome =>
            {
                old.revision
            }
            _ => {
                self.revisions += 1;
                self.revisions
            }
        }
    }

    /// The rows, newest first.
    pub(crate) fn rows(&self) -> &[ActivityRow] {
        &self.rows
    }

    /// The shown rows that count (A6).
    pub(crate) fn attention_count(&self) -> usize {
        self.rows.iter().filter(|row| row.row.attention).count()
    }

    /// S2's and S8's Dismiss (L6): the row is not shown again, and its time is
    /// forgotten. A request goes through `dismiss_request` instead.
    pub(crate) fn dismiss(&mut self, key: &RowKey) {
        let sent = self
            .rows
            .iter()
            .find(|row| &row.row.key == key)
            .and_then(|row| row.row.group.sent);
        self.dismissed.insert(key.clone(), sent);
        self.seen.remove(key);
    }

    /// The rows "Clear finished" takes (L8).
    pub(crate) fn clearable(&self) -> Vec<RowKey> {
        self.rows
            .iter()
            .filter(|row| row.row.clearable)
            .map(|row| row.row.key.clone())
            .collect()
    }

    /// Whether the row `key` is shown and still at `revision`: what Enter
    /// on a row with no button checks before it opens the row's item (T8).
    pub(crate) fn is_current(&self, key: &RowKey, revision: u64) -> bool {
        self.rows
            .iter()
            .any(|row| &row.row.key == key && row.revision == revision)
    }

    /// Whether a choice made against the row `key` at `revision` may still
    /// run: the row's revision is the same, and it still has that offer,
    /// enabled (T11, T12).
    pub(crate) fn still_offered(
        &self,
        key: &RowKey,
        revision: u64,
        action: &ActivityAction,
        label: &str,
    ) -> bool {
        self.rows.iter().any(|row| {
            &row.row.key == key
                && row.revision == revision
                && row
                    .row
                    .menu
                    .iter()
                    .any(|offer| offer.enabled && &offer.action == action && offer.label == label)
        })
    }
}
