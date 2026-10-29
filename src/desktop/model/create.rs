//! The create dialogs' rules, as pure decisions (design, Create (decision
//! 3); R-21, R-22): where a new document's Folder and File name put it, and
//! the words a create says of its path and of a repository with no initials.
//!
//! - **The path** (R-22) is checked here first: it must be relative, with
//!   no empty, `.` or `..` part, under `docs`, and end in `.md`, as the
//!   library places a document. The dialog then asks `observe_path`
//!   whether a file is there.
//! - **At Save**, the library checks the path again: a file there keeps
//!   the draft, and the tab says so.
//!
//! The paths are the user's text, so nothing here derives `Debug`.

/// The folder a new document is made in until its dialog says otherwise
/// (decision 3).
pub(crate) const DEFAULT_FOLDER: &str = "docs";

/// The ruled words (pre-flight ruling, Task 23).
pub(crate) const PATH_NOT_UNDER_DOCS: &str = "A document's path must be under docs.";
pub(crate) const PATH_NOT_MARKDOWN: &str = "A document's file name must end in .md.";
pub(crate) const INITIALS_NEEDED: &str =
    "This repository needs initials before it can create tickets.";
pub(crate) const OPEN_SETTINGS: &str = "Open Settings";

/// The folder every document is under.
const DOCUMENTS: &str = "docs";

/// A document's file name ends so.
const MARKDOWN: &str = ".md";

/// What the dialog says of `path` when a file is already there.
pub(crate) fn path_held(path: &str) -> String {
    format!("Another file is already at {path}.")
}

/// What the tab says when a create found a file at `path`: the draft stays.
pub(crate) fn path_held_draft_kept(path: &str) -> String {
    format!("{} The draft is kept.", path_held(path))
}

/// The path `folder` and `file_name` give a new document, as the library
/// writes it, or why it cannot be one: a file name that does not end in
/// `.md`, then a path that is not a plain one under `docs`. A folder may end
/// with `/`.
pub(crate) fn document_path(folder: &str, file_name: &str) -> Result<String, &'static str> {
    if !file_name.ends_with(MARKDOWN) {
        return Err(PATH_NOT_MARKDOWN);
    }
    let folder = folder.strip_suffix('/').unwrap_or(folder);
    let path = format!("{folder}/{file_name}");
    let plain = !path.contains(['\\', '\0'])
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..");
    if !plain || path.split('/').next() != Some(DOCUMENTS) {
        return Err(PATH_NOT_UNDER_DOCS);
    }
    Ok(path)
}
