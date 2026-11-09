//! The window's title (Part D, R-14).

use crate::desktop::model::navigation::RepositoryEntry;

/// The product's name, which the title is when no repository is selected.
const PRODUCT: &str = "Manyhands";

/// `Manyhands — {name}` for the selected repository, else
/// `Manyhands`. The name is the root's directory name, or the root itself
/// when it has none (`RepositoryEntry::name`).
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 6.
pub(crate) fn window_title(selected: Option<&RepositoryEntry>) -> String {
    match selected {
        Some(repository) => format!("{PRODUCT} — {}", repository.name()),
        None => PRODUCT.to_owned(),
    }
}

#[cfg(test)]
#[path = "title_tests.rs"]
mod tests;
