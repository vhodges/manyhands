//! Files the library could not read (Part I): the index status read is the
//! only one that reports them, as `source_unreadable` problems. A file in
//! `.manyhands/tickets/` is the Tickets list's, one in `docs/` the
//! Documents tree's.

use manyhands::{repository::IndexStatusDto, results::ProblemCode};

use super::markers::GIT_PREFIX;
use crate::desktop::services::ReadResult;

/// Where a ticket's files are, relative to a worktree.
const TICKETS_DIR: &str = ".manyhands/tickets/";
/// Where a document's files are, relative to a worktree.
const DOCUMENTS_DIR: &str = "docs/";
/// The tooltip's first line: `Can't be read: {guidance}`.
pub(crate) const CANT_BE_READ_PREFIX: &str = "Can't be read: ";

/// A `.md` file the library could not read, and the working tree it is in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct UnreadableFile {
    /// Relative to its worktree.
    pub path: String,
    pub worktree: String,
    /// Whether the worktree is the repository's own, not an item's.
    pub primary: bool,
}

/// The unreadable files an index status names, as tickets and as
/// documents; `root` is the repository's root, the primary worktree.
///
/// Only a `source_unreadable` problem with a relative path and a worktree
/// that names a `.md` file under `.manyhands/tickets/` or `docs/` is
/// listed. A directory, a file elsewhere, an absolute path and any other
/// code are Activity's or the incomplete alert's. The same file named twice
/// is one, since a row is keyed by worktree and path.
pub(crate) fn unreadable_rows(
    status: &IndexStatusDto,
    root: &str,
) -> (Vec<UnreadableFile>, Vec<UnreadableFile>) {
    let (mut tickets, mut documents): (Vec<UnreadableFile>, Vec<UnreadableFile>) =
        (Vec::new(), Vec::new());
    for problem in &status.problems {
        let (Some(path), Some(worktree)) = (&problem.path, &problem.worktree) else {
            continue;
        };
        // An absolute path starts with neither folder, so it is skipped
        // below.
        if problem.code != ProblemCode::SourceUnreadable || !path.ends_with(".md") {
            continue;
        }
        let list = if path.starts_with(TICKETS_DIR) {
            &mut tickets
        } else if path.starts_with(DOCUMENTS_DIR) {
            &mut documents
        } else {
            continue;
        };
        if list
            .iter()
            .any(|file| file.path == *path && file.worktree == *worktree)
        {
            continue;
        }
        list.push(UnreadableFile {
            path: path.clone(),
            worktree: worktree.clone(),
            primary: worktree == root,
        });
    }
    (tickets, documents)
}

/// A status read as the two lists' reads: each gets the same answer.
pub(crate) fn split_status(
    result: ReadResult<IndexStatusDto>,
    root: &str,
) -> (
    ReadResult<Vec<UnreadableFile>>,
    ReadResult<Vec<UnreadableFile>>,
) {
    match result {
        ReadResult::Ok(status) => {
            let (tickets, documents) = unreadable_rows(&status, root);
            (ReadResult::Ok(tickets), ReadResult::Ok(documents))
        }
        ReadResult::Busy => (ReadResult::Busy, ReadResult::Busy),
        ReadResult::Failed(failure) => (
            ReadResult::Failed(failure.clone()),
            ReadResult::Failed(failure),
        ),
    }
}

/// A file's tooltip, line by line: why, where, and in which worktree when
/// it is not the primary one. The description is the lines joined.
pub(crate) fn unreadable_tooltip(file: &UnreadableFile) -> Vec<String> {
    let mut lines = vec![
        format!(
            "{CANT_BE_READ_PREFIX}{}",
            ProblemCode::SourceUnreadable.guidance()
        ),
        file.path.clone(),
    ];
    if !file.primary {
        lines.push(format!("{GIT_PREFIX}worktree {}", file.worktree));
    }
    lines
}

#[cfg(test)]
#[path = "unreadable_tests.rs"]
mod tests;
