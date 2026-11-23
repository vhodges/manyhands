use std::collections::HashSet;

use manyhands::{
    repository::{IndexState, IndexStateDto},
    results::ResultCode,
};

use super::{KeptList, ListState};
use crate::desktop::services::{ReadFailure, ReadResult};

fn index(state: IndexState) -> Option<IndexStateDto> {
    Some(IndexStateDto {
        state,
        refreshed_at: None,
    })
}

fn ready(rows: Vec<u8>, complete: bool, index: Option<IndexStateDto>) -> ListState<Vec<u8>> {
    ListState::Ready {
        rows,
        complete,
        index,
    }
}

fn failed(code: ResultCode) -> ListState<Vec<u8>> {
    ListState::Failed(ReadFailure {
        code,
        recovery: Vec::new(),
    })
}

/// Every state the list can be in, by the name a test gives it.
fn every_state() -> Vec<(&'static str, ListState<Vec<u8>>)> {
    vec![
        ("loading", ListState::Loading),
        ("ready", ready(vec![1], true, index(IndexState::Current))),
        ("empty", ready(Vec::new(), true, index(IndexState::Current))),
        (
            "never refreshed",
            ready(Vec::new(), true, index(IndexState::NeverRefreshed)),
        ),
        ("stale", ready(vec![1], true, index(IndexState::Stale))),
        (
            "incomplete",
            ready(vec![1], false, index(IndexState::Current)),
        ),
        ("busy", ListState::Busy),
        ("failed", failed(ResultCode::RepositoryInaccessible)),
        ("index unavailable", failed(ResultCode::IndexUnavailable)),
    ]
}

#[test]
fn every_state_has_its_own_label() {
    let states = every_state();
    let labels: HashSet<&'static str> = states.iter().map(|(_, state)| state.label()).collect();
    assert_eq!(labels.len(), states.len(), "{:?}", {
        states
            .iter()
            .map(|(name, state)| (*name, state.label()))
            .collect::<Vec<_>>()
    });
    assert!(labels.iter().all(|label| !label.is_empty()));
}

#[test]
fn an_empty_list_is_not_labeled_never_refreshed() {
    let empty = ready(Vec::new(), true, index(IndexState::Current));
    let never = ready(Vec::new(), true, index(IndexState::NeverRefreshed));
    assert_ne!(empty.label(), never.label());
    // Rows from a never-refreshed index are still never refreshed.
    let never_with_rows = ready(vec![1], true, index(IndexState::NeverRefreshed));
    assert_eq!(never.label(), never_with_rows.label());
}

#[test]
fn each_index_state_has_its_own_label() {
    let labels: HashSet<&'static str> = [
        IndexState::Current,
        IndexState::Stale,
        IndexState::NeverRefreshed,
    ]
    .into_iter()
    .map(|state| ready(vec![1], true, index(state)).label())
    .collect();
    assert_eq!(labels.len(), 3);
}

/// A list that is not of one repository's index, such as the repository
/// list, has no index state; it is labeled by its rows alone.
#[test]
fn a_list_with_no_index_is_labeled_by_its_rows() {
    assert_eq!(
        ready(vec![1], true, None).label(),
        ready(vec![1], true, index(IndexState::Current)).label()
    );
    assert_eq!(
        ready(Vec::new(), true, None).label(),
        ready(Vec::new(), true, index(IndexState::Current)).label()
    );
}

#[test]
fn only_a_ready_list_has_rows() {
    assert_eq!(ready(vec![7], true, None).rows(), Some(&vec![7]));
    for state in [
        ListState::Loading,
        ListState::Busy,
        failed(ResultCode::Busy),
    ] {
        assert!(state.rows().is_none());
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    Loading,
    Ready,
    Busy,
    Failed,
}

fn kind(state: &ListState<Vec<u8>>) -> Kind {
    match state {
        ListState::Loading => Kind::Loading,
        ListState::Ready { .. } => Kind::Ready,
        ListState::Busy => Kind::Busy,
        ListState::Failed(_) => Kind::Failed,
    }
}

#[derive(Clone, Copy, Debug)]
enum Op {
    /// `apply_state` of a ready state with these rows.
    State(u8),
    /// `apply_state` of an incomplete ready state with a stale index.
    Marked(u8),
    Reload,
    Clear,
    /// `apply` of a read that gave these rows.
    Read(u8),
    /// `apply` of a read that stayed busy.
    ReadBusy,
    /// `apply` of a read that failed.
    ReadFailed,
}

/// What a step leaves: the state's kind, `rows()`, `reloading()` and
/// `busy_over_kept()`.
type After = (Kind, Option<u8>, bool, bool, Shape);

/// A ready state's `complete` and whether its index is stale; `None` for
/// any other state. Kept rows must keep the shape of the read they came
/// from.
type Shape = Option<(bool, bool)>;

const PLAIN: Shape = Some((true, false));
const STALE: Shape = Some((false, true));
const NONE: Shape = None;

fn shape(state: &ListState<Vec<u8>>) -> Shape {
    match state {
        ListState::Ready {
            complete, index, ..
        } => Some((
            *complete,
            index
                .as_ref()
                .is_some_and(|index| matches!(index.state, IndexState::Stale)),
        )),
        _ => None,
    }
}

#[test]
fn kept_rows_follow_reloads() {
    use Kind::{Busy, Failed, Loading, Ready};
    use Op::{Clear, Marked, Read, ReadBusy, ReadFailed, Reload, State};
    let table: Vec<(&str, Vec<(Op, After)>)> = vec![
        (
            "a reload shows the kept rows until the read lands",
            vec![
                (State(1), (Ready, Some(1), false, false, PLAIN)),
                (Reload, (Loading, Some(1), true, false, NONE)),
                (Read(2), (Ready, Some(2), false, false, PLAIN)),
            ],
        ),
        (
            "a second reload keeps them",
            vec![
                (State(1), (Ready, Some(1), false, false, PLAIN)),
                (Reload, (Loading, Some(1), true, false, NONE)),
                (Reload, (Loading, Some(1), true, false, NONE)),
            ],
        ),
        (
            "busy over kept rows keeps them, and the next reload clears the flag",
            vec![
                (State(1), (Ready, Some(1), false, false, PLAIN)),
                (Reload, (Loading, Some(1), true, false, NONE)),
                (ReadBusy, (Ready, Some(1), false, true, PLAIN)),
                (Reload, (Loading, Some(1), true, false, NONE)),
                (Read(2), (Ready, Some(2), false, false, PLAIN)),
            ],
        ),
        (
            "busy with no reload over a ready list keeps its rows",
            vec![
                (State(1), (Ready, Some(1), false, false, PLAIN)),
                (ReadBusy, (Ready, Some(1), false, true, PLAIN)),
                (Read(2), (Ready, Some(2), false, false, PLAIN)),
            ],
        ),
        (
            "kept rows keep the shape of their read through busy and reload",
            vec![
                (Marked(1), (Ready, Some(1), false, false, STALE)),
                (Reload, (Loading, Some(1), true, false, NONE)),
                (ReadBusy, (Ready, Some(1), false, true, STALE)),
                (Reload, (Loading, Some(1), true, false, NONE)),
                (ReadBusy, (Ready, Some(1), false, true, STALE)),
                (Read(2), (Ready, Some(2), false, false, PLAIN)),
            ],
        ),
        (
            "a failure replaces the kept rows",
            vec![
                (State(1), (Ready, Some(1), false, false, PLAIN)),
                (Reload, (Loading, Some(1), true, false, NONE)),
                (ReadFailed, (Failed, None, false, false, NONE)),
            ],
        ),
        (
            "busy with nothing kept is busy",
            vec![(ReadBusy, (Busy, None, false, false, NONE))],
        ),
        (
            "a clear drops the kept rows",
            vec![
                (State(1), (Ready, Some(1), false, false, PLAIN)),
                (Clear, (Loading, None, false, false, NONE)),
            ],
        ),
        (
            "a clear after a reload drops them too",
            vec![
                (State(1), (Ready, Some(1), false, false, PLAIN)),
                (Reload, (Loading, Some(1), true, false, NONE)),
                (Clear, (Loading, None, false, false, NONE)),
            ],
        ),
        (
            "a ready state applied after a reload replaces the kept rows",
            vec![
                (State(1), (Ready, Some(1), false, false, PLAIN)),
                (Reload, (Loading, Some(1), true, false, NONE)),
                (State(2), (Ready, Some(2), false, false, PLAIN)),
            ],
        ),
    ];
    for (name, steps) in table {
        let mut list = KeptList::<Vec<u8>>::default();
        assert_eq!(
            (kind(list.state()), list.rows(), list.reloading()),
            (Loading, None, false),
            "{name}: starts empty and loading"
        );
        for (op, expected) in steps {
            match op {
                State(rows) => list.apply_state(ready(vec![rows], true, None)),
                Marked(rows) => {
                    list.apply_state(ready(vec![rows], false, index(IndexState::Stale)))
                }
                Reload => list.reload(),
                Clear => list.clear(),
                Read(rows) => {
                    list.apply(ReadResult::Ok(vec![rows]), |rows| ready(rows, true, None))
                }
                ReadBusy => list.apply(ReadResult::<Vec<u8>>::Busy, |rows| ready(rows, true, None)),
                ReadFailed => list.apply(
                    ReadResult::<Vec<u8>>::Failed(ReadFailure {
                        code: ResultCode::RepositoryInaccessible,
                        recovery: Vec::new(),
                    }),
                    |rows| ready(rows, true, None),
                ),
            }
            let rows = list.rows().map(|rows| rows[0]);
            assert_eq!(
                (
                    kind(list.state()),
                    rows,
                    list.reloading(),
                    list.busy_over_kept(),
                    shape(list.state())
                ),
                expected,
                "{name}: after {op:?}"
            );
        }
    }
}
