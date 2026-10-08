//! The comment threads of one item.
//!
//! The index says which copy of the item is the effective one and which
//! comment files that copy has. Everything a comment says is read from its
//! file as it is now, through the same guarded reader as the item itself,
//! and validated and ordered by the canonical rules a refresh applies.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use rusqlite::Connection;

use super::{
    CommentDto, CommentListDto, ReadError, ResolvedRepository,
    items::{
        EffectiveCopy, ItemFileRead, behind, effective_copy, invalid_stored_data,
        is_plain_relative, item_not_found, metadata_problems, problem, read_item_file,
        stored_index_state, stored_items, stored_problems,
    },
};
use crate::{
    canonical::{self, ItemId},
    repository::{RepositoryOperation, RepositoryService, discovery::UnknownMetadata},
    results::{ProblemCode, timestamp_string},
};

/// The front matter key that records who created a comment. The canonical
/// parser does not define it yet, so it arrives among the unknown keys.
const CREATED_BY: &str = "created_by";

/// Where an item's comment files are, as a path prefix.
fn comment_directory(item: &str) -> String {
    format!(".manyhands/comments/{item}/")
}

/// Whether `path` is written as a file directly inside `directory`, which
/// ends with a slash. Nothing below a subdirectory, and nothing reached by
/// stepping out, is.
fn is_file_in(directory: &str, path: &str) -> bool {
    is_plain_relative(path)
        && path
            .strip_prefix(directory)
            .is_some_and(|name| !name.contains('/'))
}

/// What the index stored for one comment: enough to tell whether the file
/// still says the same.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct IndexedComment {
    path: String,
    id: String,
    parent_id: Option<String>,
    created_at: i64,
}

/// The comments stored under one item row, in no particular order.
fn stored_comments(
    connection: &Connection,
    item_row: i64,
) -> Result<BTreeSet<IndexedComment>, ReadError> {
    let mut statement = connection.prepare(
        "SELECT canonical_path, comment_id, parent_comment_id, created_at
           FROM discovered_comments
          WHERE item_id = ?1",
    )?;
    let comments = statement
        .query_map([item_row], |row| {
            Ok(IndexedComment {
                path: row.get(0)?,
                id: row.get(1)?,
                parent_id: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(comments)
}

/// Takes `created_by` out of a comment's unknown keys. The author is its
/// value when that is what `closed_by` may be, a non-empty string with no
/// NUL; `Err` is any other value, which names nobody.
fn take_author(unknown: &mut serde_yaml::Mapping) -> Result<Option<String>, ()> {
    match unknown.remove(CREATED_BY) {
        None => Ok(None),
        Some(serde_yaml::Value::String(author)) if !author.is_empty() && !author.contains('\0') => {
            Ok(Some(author))
        }
        Some(_) => Err(()),
    }
}

/// A comment and the replies beneath it, in the order they are given.
fn comment_dto(thread: canonical::CommentThread, directory: &str) -> CommentDto {
    let mut comment = thread.comment;
    // Validation accepted the comment, so its file is named for its ID.
    let path = format!("{directory}{}.md", comment.id);
    let mut problems = Vec::new();
    let author = take_author(&mut comment.unknown).unwrap_or_else(|()| {
        problems.push(problem(ProblemCode::InvalidField, &path));
        None
    });
    let unknown = UnknownMetadata::from_yaml(&comment.unknown);
    problems.extend(metadata_problems(&unknown, &path));
    CommentDto {
        id: Some(comment.id.to_string()),
        item_id: comment.item_id.to_string(),
        parent_id: comment.parent_id.as_ref().map(ToString::to_string),
        author,
        created_at: timestamp_string(comment.created_at),
        body: Some(comment.body),
        path,
        unknown_metadata: unknown.values,
        problems,
        replies: thread
            .replies
            .into_iter()
            .map(|reply| comment_dto(reply, directory))
            .collect(),
    }
}

/// A file among an item's comments that is not a comment of it.
fn nonconforming_entry(item: &str, path: &str, codes: &[ProblemCode]) -> CommentDto {
    CommentDto {
        id: None,
        item_id: item.to_owned(),
        parent_id: None,
        author: None,
        created_at: None,
        body: None,
        path: path.to_owned(),
        unknown_metadata: serde_json::Map::new(),
        problems: codes.iter().map(|code| problem(*code, path)).collect(),
        replies: Vec::new(),
    }
}

/// What the index would hold for these threads, were it refreshed now.
fn collect_indexed(
    threads: &[canonical::CommentThread],
    directory: &str,
    comments: &mut BTreeSet<IndexedComment>,
) {
    for thread in threads {
        let comment = &thread.comment;
        comments.insert(IndexedComment {
            path: format!("{directory}{}.md", comment.id),
            id: comment.id.to_string(),
            parent_id: comment.parent_id.as_ref().map(ToString::to_string),
            created_at: comment.created_at.unix_timestamp(),
        });
        collect_indexed(&thread.replies, directory, comments);
    }
}

fn add_code(entries: &mut BTreeMap<String, Vec<ProblemCode>>, path: &str, code: ProblemCode) {
    let codes = entries.entry(path.to_owned()).or_default();
    if !codes.contains(&code) {
        codes.push(code);
    }
}

impl RepositoryService {
    /// The comment threads of the item with this ID, read from the item's
    /// effective copy: the worktree created to edit it when there is one,
    /// otherwise the primary copy. `context` says which.
    ///
    /// Root comments are in `created_at` and then ID order, and so are the
    /// replies beneath each comment, to any depth. `author` is a comment's
    /// `created_by` value and null when it has none; Git history is never
    /// read.
    ///
    /// The index says which files to read: those it holds as the item's
    /// comments and those among them it holds a problem for. Each is read
    /// as it is now, and together with the item's own file they are
    /// validated as a refresh would validate them. A file that is not a
    /// comment of the item by those rules, that is not valid UTF-8, or
    /// that is no longer a regular file follows the threads as a
    /// nonconforming entry, in path order, with a null ID, its path and
    /// the reason. A file that is gone is left out.
    ///
    /// `index.state` is `stale` when the comments are no longer what the
    /// index stored. A comment file added since the last refresh is not
    /// read, and nothing reports it, until the next one.
    ///
    /// An item the index does not hold, whose file is gone, or whose file
    /// now holds another item is `item_not_found`, as it is for
    /// `show_item`. A file that cannot be opened is
    /// `repository_inaccessible`: the list is not returned without it.
    pub fn list_comments(
        &self,
        repo: &ResolvedRepository,
        item: &ItemId,
    ) -> Result<CommentListDto, ReadError> {
        let id = item.to_string();
        self.read_session(RepositoryOperation::Read, |connection| {
            let (index, refreshed_at) = stored_index_state(connection, repo)?;
            let stored = stored_items(connection, repo)?;
            let EffectiveCopy {
                row,
                file: item_file,
                index,
            } = effective_copy(repo, &stored, &id, index)?;
            let worktree = &row.context.worktree;
            let directory = comment_directory(&id);

            // A row that names a file anywhere but in the item's own
            // comment directory is not read from.
            let indexed = stored_comments(connection, row.row_id)?;
            if indexed
                .iter()
                .any(|comment| !is_file_in(&directory, &comment.path))
            {
                return Err(invalid_stored_data());
            }
            // The files the index knows of there, comments or not.
            let mut paths: BTreeSet<String> =
                indexed.iter().map(|comment| comment.path.clone()).collect();
            paths.extend(
                stored_problems(connection, repo)?
                    .into_iter()
                    .filter(|problem| {
                        &problem.context.worktree == worktree
                            && is_file_in(&directory, &problem.path)
                    })
                    .map(|problem| problem.path),
            );

            let mut is_behind = false;
            let mut entries: BTreeMap<String, Vec<ProblemCode>> = BTreeMap::new();
            let mut sources = Vec::new();
            // An item file that is not text is not an item, and then no
            // comment has one.
            if let Ok(source) = String::from_utf8(item_file.bytes) {
                // Another item is now where this one was.
                if canonical::parse_item(Path::new(&row.path), &source)
                    .is_ok_and(|parsed| canonical_id(&parsed) != item)
                {
                    return Err(item_not_found(repo, true));
                }
                sources.push((PathBuf::from(&row.path), source));
            }
            for path in &paths {
                match read_item_file(repo, worktree, path)? {
                    ItemFileRead::Found(file) => {
                        is_behind |= file.newer_than(refreshed_at);
                        match String::from_utf8(file.bytes) {
                            Ok(source) => sources.push((PathBuf::from(path), source)),
                            Err(_) => add_code(&mut entries, path, ProblemCode::SourceUnreadable),
                        }
                    }
                    ItemFileRead::Missing => is_behind = true,
                    ItemFileRead::NotAFile => {
                        add_code(&mut entries, path, ProblemCode::SourceUnreadable);
                    }
                }
            }

            let validation = canonical::validate_context(sources);
            for found in &validation.problems {
                // The item's own problems are the item read's to report. A
                // problem's message can repeat the file's text; only its
                // code is kept.
                if let Some(path) = found.path.to_str().filter(|path| paths.contains(*path)) {
                    add_code(&mut entries, path, ProblemCode::from(&found.code));
                }
            }
            let threads = canonical::ordered_comment_threads(&validation);
            let mut current = BTreeSet::new();
            collect_indexed(&threads, &directory, &mut current);
            is_behind |= current != indexed;

            let items: Vec<CommentDto> = threads
                .into_iter()
                .map(|thread| comment_dto(thread, &directory))
                .chain(
                    entries
                        .iter()
                        .map(|(path, codes)| nonconforming_entry(&id, path, codes)),
                )
                .collect();
            Ok(CommentListDto {
                items,
                complete: true,
                context: row.context.dto(),
                index: if is_behind { behind(&index) } else { index },
            })
        })
        .map_err(|error| {
            let mut error = repo.failure(error);
            error.scope.item_id = Some(id.clone());
            error
        })
    }
}

fn canonical_id(item: &canonical::CanonicalItem) -> &ItemId {
    match item {
        canonical::CanonicalItem::Document(document) => &document.id,
        canonical::CanonicalItem::Ticket(ticket) => &ticket.id,
        canonical::CanonicalItem::Comment(comment) => &comment.id,
    }
}

#[cfg(test)]
#[path = "comments_tests.rs"]
mod tests;
