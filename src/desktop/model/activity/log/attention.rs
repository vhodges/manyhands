//! Which rows count (design A3): each thing counts once. `assign_attention`
//! runs last in `refresh`, over the final rows, after the keeping rules.
//!
//! A failed request (a candidate S1 row) goes through A3's precedence: N23
//! (followed), N22 (settled), N21 (running), N2 (grouped; the newest member
//! goes on), N4 (absorbed by a hold or repository row), N1 (absorbed by a
//! counted pending row), N24 and N3 (counts). N22 and N23 are latched per
//! attempt (`settled`); what absorbed a request is kept per attempt
//! (`absorbed`). Every other candidate row counts, unless the operation it is
//! about runs (N21; N25 stays quiet for its operation).

use std::{collections::BTreeSet, mem::take};

use manyhands::results::Outcome;

use super::ActivityLog;
use crate::desktop::model::activity::rows::{ActivityRow, Group, RowKey, Subject};

/// What N2 groups by: the operation when a request names one, else its
/// subject.
#[derive(PartialEq)]
enum Grouping<'a> {
    Operation(&'a str),
    Subject(Option<&'a Subject>),
}

/// A hold or repository row that covers a failed Check now's root (A3, N4;
/// S5's and S9's rows name roots too, and cover nothing).
fn covers(row: &ActivityRow, roots: &[String]) -> bool {
    matches!(row.row.key, RowKey::Hold(_) | RowKey::Repository(_))
        && row.row.group.candidate
        && roots.iter().any(|root| row.row.group.roots.contains(root))
}

/// What a row is about for N22's clause 1: its `Intent` subject and its
/// operation as a subject (R-57), either of which a later success may match.
fn subjects(group: &Group) -> Vec<Subject> {
    group
        .subject
        .iter()
        .cloned()
        .chain(group.operation.iter().cloned().map(Subject::Operation))
        .collect()
}

impl ActivityLog {
    /// Sets `attention` and the settled half of `clearable` on every row, and
    /// moves the two latches. Reads and writes `settled` and `absorbed`.
    pub(super) fn assign_attention(&mut self) {
        let mut rows = take(&mut self.rows);
        let running: BTreeSet<String> = rows
            .iter()
            .filter_map(|row| row.row.group.running_operation.clone())
            .collect();
        let mut counted: BTreeSet<RowKey> = BTreeSet::new();

        // Every row that is not a failed request counts by its own say, unless
        // its operation runs (N21).
        for row in &rows {
            let group = &row.row.group;
            let runs = group
                .operation
                .as_ref()
                .is_some_and(|operation| running.contains(operation));
            if group.candidate && !matches!(row.row.key, RowKey::Request(_)) && !runs {
                counted.insert(row.row.key.clone());
            }
        }

        // The failed requests that remain after N23, N22 and N21.
        let mut remaining: Vec<&ActivityRow> = Vec::new();
        for row in rows
            .iter()
            .filter(|row| matches!(row.row.key, RowKey::Request(_)) && row.row.group.candidate)
        {
            let (key, group) = (&row.row.key, &row.row.group);
            let attempt = group.sent.unwrap_or_default();
            if group.followed {
                if group.follow_void {
                    // The follow-up did nothing: the one latch that is undone.
                    self.settled.remove(key);
                } else {
                    self.settled.insert(key.clone(), attempt);
                    continue;
                }
            }
            if self.settled.get(key) == Some(&attempt) {
                continue;
            }
            if self.settles(row, &rows) {
                self.settled.insert(key.clone(), attempt);
                continue;
            }
            if group
                .operation
                .as_ref()
                .is_some_and(|operation| running.contains(operation))
            {
                continue;
            }
            // What N19 may absorb: whether a kept row covers it as it is
            // first evaluated (C9), set once for the attempt.
            if group.check {
                let kept = rows
                    .iter()
                    .any(|other| covers(other, &group.roots) && self.kept.contains(&other.row.key));
                self.first_kept.entry(key.clone()).or_insert(kept);
            }
            remaining.push(row);
        }

        // N2: grouped, and the newest member by queue order goes on.
        let mut groups: Vec<(Grouping<'_>, Vec<&ActivityRow>)> = Vec::new();
        for row in remaining {
            let group = &row.row.group;
            let by = match &group.operation {
                Some(operation) => Grouping::Operation(operation),
                None => Grouping::Subject(group.subject.as_ref()),
            };
            match groups.iter_mut().find(|(held, _)| *held == by) {
                Some((_, members)) => members.push(row),
                None => groups.push((by, vec![row])),
            }
        }
        for (_, members) in groups {
            let Some(newest) = members.iter().max_by_key(|row| row.row.group.sent) else {
                continue;
            };
            match self.absorbers(newest, &rows, &running) {
                Some(absorbers) => {
                    // The absorber is recorded for every member (C4).
                    for member in &members {
                        self.absorbed
                            .entry(member.row.key.clone())
                            .or_default()
                            .extend(absorbers.iter().cloned());
                    }
                }
                None => {
                    counted.insert(newest.row.key.clone());
                }
            }
        }

        for row in &mut rows {
            let key = &row.row.key;
            row.row.attention = counted.contains(key);
            let settled = matches!(key, RowKey::Request(_))
                && row.row.group.candidate
                && self.settled.contains_key(key)
                && !row.row.group.resend_waits;
            row.row.clearable = !row.row.attention && (row.row.clearable || settled);
        }
        self.rows = rows;
    }

    /// N22: whether one of its clauses holds for `failed` (the attempt it is
    /// on): a later success, its absorber left, a later completed check.
    fn settles(&self, failed: &ActivityRow, rows: &[ActivityRow]) -> bool {
        let (key, group) = (&failed.row.key, &failed.row.group);
        // Clause 1 (and 5, and 2, which it includes): a later row, by queue
        // order, that finished Success or Noop, with the same subject or the
        // same operation.
        let mine = subjects(group);
        let later = rows.iter().any(|other| {
            let theirs = &other.row.group;
            &other.row.key != key
                && theirs.sent > group.sent
                && matches!(theirs.outcome, Some(Outcome::Success | Outcome::Noop))
                && subjects(theirs)
                    .iter()
                    .any(|subject| mine.contains(subject))
        });
        // Clause 4: a later completed check of its repository.
        let checked = group.check && group.superseded;
        later || checked || self.absorber_left(failed, rows)
    }

    /// Clause 3: a row that absorbed it has left. A pending row's leaving is
    /// enough. A hold or repository row has left when it is gone or no longer
    /// covers the request's root (C3), and no candidate hold or repository
    /// row covers the root now either.
    fn absorber_left(&self, failed: &ActivityRow, rows: &[ActivityRow]) -> bool {
        let Some(absorbers) = self.absorbed.get(&failed.row.key) else {
            return false;
        };
        let roots = &failed.row.group.roots;
        let alive = |key: &RowKey| rows.iter().find(|row| &row.row.key == key);
        let pending_left = absorbers
            .iter()
            .any(|key| matches!(key, RowKey::Pending { .. }) && alive(key).is_none());
        let mut covering = absorbers
            .iter()
            .filter(|key| matches!(key, RowKey::Hold(_) | RowKey::Repository(_)))
            .peekable();
        let covering_left = covering.peek().is_some()
            && covering.all(|key| alive(key).is_none_or(|row| !covers(row, roots)))
            && !rows.iter().any(|row| covers(row, roots));
        pending_left || covering_left
    }

    /// The rows that absorb the newest member of a group, when one does: N4 for
    /// a Check now (its answer was applied, or the covering row is one L3
    /// kept, or the row already absorbed it this attempt, C2), then N1 for a
    /// pending row of its operation that counts. `None`: it counts (N24, N3,
    /// or N12's fallback).
    fn absorbers(
        &self,
        newest: &ActivityRow,
        rows: &[ActivityRow],
        running: &BTreeSet<String>,
    ) -> Option<Vec<RowKey>> {
        let (key, group) = (&newest.row.key, &newest.row.group);
        if group.check {
            let covering: Vec<RowKey> = rows
                .iter()
                .filter(|row| covers(row, &group.roots))
                .map(|row| row.row.key.clone())
                .collect();
            let held = self
                .absorbed
                .get(key)
                .is_some_and(|before| covering.iter().any(|row| before.contains(row)));
            let kept = self.first_kept.get(key) == Some(&true)
                && covering.iter().any(|row| self.kept.contains(row));
            if !covering.is_empty() && (group.check_applied || kept || held) {
                return Some(covering);
            }
        }
        let operation = group.operation.as_ref()?;
        let pending = rows.iter().find(|row| {
            matches!(row.row.key, RowKey::Pending { .. })
                && row.row.group.operation.as_ref() == Some(operation)
        })?;
        let counts = pending.row.group.candidate && !running.contains(operation);
        counts.then(|| vec![pending.row.key.clone()])
    }
}
