use manyhands::repository::{
    Accessibility, ConfigurationDto, ConfigurationState, IndexState, IndexStateDto,
    RepositorySummaryDto,
};

use super::window_title;
use crate::desktop::model::navigation::{RepositoryEntry, Resolution};

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
