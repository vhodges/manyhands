use manyhands::{
    repository::{
        Accessibility, ConfigurationDto, ConfigurationState, IndexState, IndexStateDto,
        RepositoryListDto, RepositorySummaryDto,
    },
    results::ResultCode,
};

use super::{ListState, Navigation, Pane, Resolution, UNAVAILABLE_CODES};
use crate::desktop::{
    services::{ReadFailure, ReadResult},
    test_support::fixtures::Fixture,
};

fn summary(root: &str, accessibility: Accessibility) -> RepositorySummaryDto {
    RepositorySummaryDto {
        root: root.to_owned(),
        enabled_at: None,
        accessibility,
        configuration: ConfigurationDto {
            state: ConfigurationState::Valid,
            primary_branch: Some("main".to_owned()),
            publication_remote: None,
            problems: Vec::new(),
        },
        index: IndexStateDto {
            state: IndexState::NeverRefreshed,
            refreshed_at: None,
        },
        problem_count: 0,
    }
}

fn list(roots: &[&str]) -> ReadResult<RepositoryListDto> {
    ReadResult::Ok(RepositoryListDto {
        items: roots
            .iter()
            .map(|root| summary(root, Accessibility::Accessible))
            .collect(),
        complete: true,
    })
}

fn failure(code: ResultCode) -> ReadResult<manyhands::repository::ResolvedRepository> {
    ReadResult::Failed(ReadFailure {
        code,
        recovery: Vec::new(),
    })
}

/// A navigation whose list holds `roots`.
fn loaded(roots: &[&str]) -> Navigation {
    let mut navigation = Navigation::default();
    let stamp = navigation.load_repositories();
    assert!(navigation.apply_repositories(stamp, list(roots)));
    navigation
}

fn resolution<'a>(navigation: &'a Navigation, key: &str) -> &'a Resolution {
    &navigation.entry(key).expect("a listed entry").resolution
}

#[test]
fn a_new_navigation_is_loading_with_nothing_selected_on_documents() {
    let navigation = Navigation::default();
    assert!(matches!(navigation.repositories(), ListState::Loading));
    assert_eq!(navigation.selected(), None);
    assert_eq!(navigation.pane(), Pane::Documents);
}

#[test]
fn the_list_holds_each_summary_unresolved_in_the_read_order() {
    let navigation = loaded(&["/r/a", "/r/b"]);
    let ListState::Ready { rows, complete, .. } = navigation.repositories() else {
        panic!("a ready list");
    };
    assert!(complete);
    let roots: Vec<&str> = rows.iter().map(|entry| entry.key().as_str()).collect();
    assert_eq!(roots, ["/r/a", "/r/b"]);
    assert!(
        rows.iter()
            .all(|entry| matches!(entry.resolution, Resolution::Unknown))
    );
    assert_eq!(rows[0].name(), "a");
}

#[test]
fn an_old_list_read_is_dropped() {
    let mut navigation = Navigation::default();
    let old = navigation.load_repositories();
    let new = navigation.load_repositories();
    assert!(!navigation.apply_repositories(old, list(&["/r/old"])));
    assert!(matches!(navigation.repositories(), ListState::Loading));
    assert!(navigation.apply_repositories(new, list(&["/r/new"])));
    assert!(navigation.entry("/r/new").is_some());
}

#[test]
fn a_busy_or_failed_list_read_is_kept_as_such() {
    let mut navigation = Navigation::default();
    let stamp = navigation.load_repositories();
    assert!(navigation.apply_repositories(stamp, ReadResult::Busy));
    assert!(matches!(navigation.repositories(), ListState::Busy));
    let stamp = navigation.load_repositories();
    let failed = ReadResult::Failed(ReadFailure {
        code: ResultCode::IndexUnavailable,
        recovery: Vec::new(),
    });
    assert!(navigation.apply_repositories(stamp, failed));
    assert!(matches!(
        navigation.repositories(),
        ListState::Failed(ReadFailure {
            code: ResultCode::IndexUnavailable,
            ..
        })
    ));
}

/// Unavailable is decided by resolution, never by the stored
/// accessibility (defect 01M4EHGE2K): an entry stored as inaccessible is
/// still Unknown until it is resolved.
#[test]
fn stored_accessibility_does_not_make_an_entry_unavailable() {
    let mut navigation = Navigation::default();
    let stamp = navigation.load_repositories();
    let read = ReadResult::Ok(RepositoryListDto {
        items: vec![summary("/r/a", Accessibility::Inaccessible)],
        complete: true,
    });
    assert!(navigation.apply_repositories(stamp, read));
    assert!(matches!(
        resolution(&navigation, "/r/a"),
        Resolution::Unknown
    ));
}

#[test]
fn each_resolution_failure_that_means_unavailable_makes_the_entry_unavailable() {
    assert_eq!(
        UNAVAILABLE_CODES,
        [
            ResultCode::InvalidPath,
            ResultCode::NotRepository,
            ResultCode::RepositoryInaccessible,
            ResultCode::RepositoryNotRegistered,
        ]
    );
    for code in UNAVAILABLE_CODES {
        let mut navigation = loaded(&["/r/a"]);
        let stamp = navigation.select("/r/a".to_owned());
        assert!(navigation.apply_resolution(stamp, failure(code)));
        assert!(
            matches!(resolution(&navigation, "/r/a"), Resolution::Unavailable(shown) if *shown == code),
            "{code:?}"
        );
    }
}

#[test]
fn another_resolution_failure_is_a_failure_not_unavailable() {
    let mut navigation = loaded(&["/r/a"]);
    let stamp = navigation.select("/r/a".to_owned());
    assert!(navigation.apply_resolution(stamp, failure(ResultCode::NotRepositoryRoot)));
    assert!(matches!(
        resolution(&navigation, "/r/a"),
        Resolution::Failed(ReadFailure {
            code: ResultCode::NotRepositoryRoot,
            ..
        })
    ));

    let stamp = navigation.retry_resolution("/r/a".to_owned());
    assert!(navigation.apply_resolution(stamp, ReadResult::Busy));
    assert!(matches!(resolution(&navigation, "/r/a"), Resolution::Busy));
}

#[test]
fn selecting_and_retrying_start_a_new_unknown_resolution() {
    let mut navigation = loaded(&["/r/a"]);
    let stamp = navigation.select("/r/a".to_owned());
    assert_eq!(navigation.selected(), Some(&"/r/a".to_owned()));
    assert!(navigation.apply_resolution(stamp, failure(ResultCode::InvalidPath)));

    let retry = navigation.retry_resolution("/r/a".to_owned());
    assert!(matches!(
        resolution(&navigation, "/r/a"),
        Resolution::Unknown
    ));
    assert!(navigation.apply_resolution(retry, failure(ResultCode::NotRepository)));
    assert!(matches!(
        resolution(&navigation, "/r/a"),
        Resolution::Unavailable(ResultCode::NotRepository)
    ));
}

/// Review Focus 1: A's resolution completes after B is selected; it is
/// dropped, B stays selected and A is left to resolve when selected again.
#[test]
fn a_resolution_that_completes_after_another_selection_is_dropped() {
    let mut navigation = loaded(&["/r/a", "/r/b"]);
    let a = navigation.select("/r/a".to_owned());
    let b = navigation.select("/r/b".to_owned());
    assert!(navigation.apply_resolution(b, failure(ResultCode::InvalidPath)));
    assert!(!navigation.apply_resolution(a, failure(ResultCode::NotRepository)));
    assert_eq!(navigation.selected(), Some(&"/r/b".to_owned()));
    assert!(matches!(
        resolution(&navigation, "/r/a"),
        Resolution::Unknown
    ));
    assert!(matches!(
        resolution(&navigation, "/r/b"),
        Resolution::Unavailable(ResultCode::InvalidPath)
    ));
}

#[test]
fn a_real_resolution_is_kept_as_resolved() {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let key = root.to_str().unwrap().to_owned();
    let mut navigation = loaded(&[&key]);
    let stamp = navigation.select(key.clone());
    let resolved = fixture.service().resolve_repository(&root).unwrap();
    assert!(navigation.apply_resolution(stamp, ReadResult::Ok(resolved.clone())));
    assert!(matches!(
        resolution(&navigation, &key),
        Resolution::Resolved(kept) if *kept == resolved
    ));
}

/// A new list keeps what each still-listed entry resolved to. A selection
/// that is no longer listed stays selected, out of the list, so its
/// resolution can still say why; selecting another lets it go.
#[test]
fn a_new_list_keeps_resolutions_and_a_vanished_selection_until_another_is_selected() {
    let mut navigation = loaded(&["/r/a", "/r/b"]);
    let stamp = navigation.select("/r/a".to_owned());
    assert!(navigation.apply_resolution(stamp, failure(ResultCode::InvalidPath)));

    let stamp = navigation.load_repositories();
    assert!(navigation.apply_repositories(stamp, list(&["/r/a", "/r/c"])));
    assert!(matches!(
        resolution(&navigation, "/r/a"),
        Resolution::Unavailable(ResultCode::InvalidPath)
    ));
    assert_eq!(navigation.selected(), Some(&"/r/a".to_owned()));

    let stamp = navigation.load_repositories();
    assert!(navigation.apply_repositories(stamp, list(&["/r/c"])));
    assert_eq!(navigation.selected(), Some(&"/r/a".to_owned()));
    let rows = navigation.repositories().rows().unwrap();
    assert!(rows.iter().all(|entry| entry.key() != "/r/a"), "not listed");
    assert!(matches!(
        navigation.selected_entry().map(|entry| &entry.resolution),
        Some(Resolution::Unavailable(ResultCode::InvalidPath))
    ));
    // A failed list read keeps the selection the same way.
    let stamp = navigation.load_repositories();
    assert!(navigation.apply_repositories(stamp, ReadResult::Busy));
    assert!(navigation.selected_entry().is_some());

    let stamp = navigation.load_repositories();
    assert!(navigation.apply_repositories(stamp, list(&["/r/c"])));
    navigation.select("/r/c".to_owned());
    assert!(navigation.entry("/r/a").is_none());
    assert_eq!(navigation.selected(), Some(&"/r/c".to_owned()));
}

/// A selection with no entry, listed or kept, is cleared by the next list.
#[test]
fn a_selection_never_listed_is_cleared_by_the_next_list() {
    let mut navigation = loaded(&["/r/a"]);
    navigation.select("/r/x".to_owned());
    let stamp = navigation.load_repositories();
    assert!(navigation.apply_repositories(stamp, list(&["/r/a"])));
    assert_eq!(navigation.selected(), None);
}

#[test]
fn the_pane_can_be_switched() {
    let mut navigation = Navigation::default();
    navigation.set_pane(Pane::Tickets);
    assert_eq!(navigation.pane(), Pane::Tickets);
}
