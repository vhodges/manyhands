#[cfg(feature = "desktop-test-support")]
use gpui_kit::TestAppContext;
use manyhands::repository::{
    Accessibility, ConfigurationDto, ConfigurationState, IndexState, IndexStateDto,
    RepositorySummaryDto,
};

use super::window_title;
use crate::desktop::model::navigation::{RepositoryEntry, Resolution};
#[cfg(feature = "desktop-test-support")]
use crate::desktop::{
    services::requests::Intent,
    test_support::fixtures::Fixture,
    view::harness::{Shell, key, wait_until},
};

fn entry(root: &str) -> RepositoryEntry {
    RepositoryEntry {
        summary: RepositorySummaryDto {
            root: root.to_owned(),
            enabled_at: None,
            accessibility: Accessibility::Accessible,
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
        },
        resolution: Resolution::Unknown,
    }
}

/// P22 (R-14): the title names the selected repository by its root's
/// directory name, or by the root itself when it has none (fact 29), and
/// is just the product's name when none is selected. The dash is U+2014
/// with a space either side.
#[test]
fn title_names_the_selected_repository() {
    let nameless = entry("/");
    assert_eq!(nameless.name(), "/");
    for (selected, expected) in [
        (Some(entry("/work/alpha")), "Manyhands \u{2014} alpha"),
        (Some(entry("/work/alpha/")), "Manyhands \u{2014} alpha"),
        (Some(nameless), "Manyhands \u{2014} /"),
        (None, "Manyhands"),
    ] {
        assert_eq!(window_title(selected.as_ref()), expected);
    }
}

/// P22 (R-14, fact 18): the window's own title follows the selection, as
/// the window reads it: set once the shell has drawn after each change,
/// and back to the product's name when the selected repository is removed.
#[cfg(feature = "desktop-test-support")]
#[gpui_kit::test]
async fn the_window_title_follows_the_selected_repository(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    let b = fixture.create_repository("b");
    let shell = Shell::open(fixture, cx);
    let title = |cx: &mut TestAppContext| {
        shell.frame(cx, |_, _| ());
        shell.visual(cx).window_title()
    };
    assert_eq!(title(cx).as_deref(), Some("Manyhands"));

    shell.select(&a, cx);
    assert_eq!(title(cx).as_deref(), Some("Manyhands \u{2014} a"));
    shell.select(&b, cx);
    assert_eq!(title(cx).as_deref(), Some("Manyhands \u{2014} b"));

    // Removing the selected registration from its settings page lets the
    // selection go.
    shell.keys("secondary-,", cx);
    let rows = shell.rows("managed-row", cx);
    let ix = rows.iter().position(|row| *row == key(&b)).unwrap();
    shell.click(("managed-choose", ix), cx);
    shell.click("remove-submit", cx);
    shell.click("confirm-ok", cx);
    let removal = shell.requests(cx, |request| {
        matches!(request.intent, Intent::RepoRemove { .. })
    })[0];
    shell.until_ended(removal, cx).await;
    wait_until(cx, &shell.workspace, |workspace, _| {
        workspace.navigation().selected().is_none()
    })
    .await;
    assert_eq!(title(cx).as_deref(), Some("Manyhands"));
}
