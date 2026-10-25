use std::collections::HashSet;

use manyhands::{
    repository::{IndexState, IndexStateDto},
    results::ResultCode,
};

use super::ListState;
use crate::desktop::services::ReadFailure;

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
