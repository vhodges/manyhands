//! The comment threads of one item.
//!
//! The index says which copy of the item is the effective one and which
//! files among that copy's comments there are. What a comment says is read
//! from its file as it is now, through the same guarded reader as the item
//! itself. The item's file and its comments are then validated and ordered
//! by the canonical rules; a problem that only the whole context shows is
//! taken from what the last refresh stored.

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
    results::{ProblemCode, ResultCode, timestamp_string},
};

/// The front matter key that records who created a comment. The canonical
/// parser does not define it yet, so it arrives among the unknown keys.
const CREATED_BY: &str = "created_by";

/// Where every item's comments are, relative to a context's worktree.
const COMMENTS_DIRECTORY: &str = ".manyhands/comments";

/// The problems a refresh finds by looking at every file of a context, so
/// that the files of one item are not enough to find them again: an ID
/// that another item's comment also has, and a parent that is a comment on
/// another item.
const CONTEXT_WIDE_PROBLEMS: [ProblemCode; 2] =
    [ProblemCode::DuplicateId, ProblemCode::CrossItemParent];

/// Where an item's comment files are, as a path prefix.
fn comment_directory(item: &str) -> String {
    format!("{COMMENTS_DIRECTORY}/{item}/")
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

/// Takes `created_by` out of a comment's unknown keys, where the parser
/// leaves a value it does not take for the comment's author. The author is
/// its value when that is what `closed_by` may be, a non-empty string with
/// no NUL; `Err` is any other value, which names nobody.
fn take_author(unknown: &mut serde_yaml::Mapping) -> Result<Option<String>, ()> {
    match unknown.remove(CREATED_BY) {
        None => Ok(None),
        Some(serde_yaml::Value::String(author)) if !author.is_empty() && !author.contains('\0') => {
            Ok(Some(author))
        }
        Some(_) => Err(()),
    }
}

/// A comment that is `depth` replies below a root comment.
fn comment_dto(mut comment: canonical::Comment, depth: u32, directory: &str) -> CommentDto {
    // Validation accepted the comment, so its file is named for its ID.
    let path = format!("{directory}{}.md", comment.id);
    let mut problems = Vec::new();
    let unparsed = take_author(&mut comment.unknown).unwrap_or_else(|()| {
        problems.push(problem(ProblemCode::InvalidField, &path));
        None
    });
    let author = comment.created_by.take().or(unparsed);
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
        depth,
    }
}

/// The threads as one list: each comment, then its replies in the order
/// they are given, each followed by its own. The canonical order puts a
/// reply only beneath the comment its file names, so a reply's `parent_id`
/// is the ID of the nearest earlier comment one level up. A chain of
/// replies is walked without recursion, however long it is.
fn flattened(threads: Vec<canonical::CommentThread>, directory: &str) -> Vec<CommentDto> {
    let mut comments = Vec::new();
    // The comments still to list at each depth, the deepest last.
    let mut pending = vec![threads.into_iter()];
    while let Some(level) = pending.last_mut() {
        let Some(thread) = level.next() else {
            pending.pop();
            continue;
        };
        let depth = u32::try_from(pending.len() - 1).unwrap_or(u32::MAX);
        comments.push(comment_dto(thread.comment, depth, directory));
        pending.push(thread.replies.into_iter());
    }
    comments
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
        depth: 0,
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

/// Records what validation found wrong with each file in `paths`. Only a
/// problem's code is kept: its message can repeat the file's text or the
/// parser's. A problem with any other file, the item's own included, is
/// not a comment's and is left to the item read.
fn add_validation_codes(
    entries: &mut BTreeMap<String, Vec<ProblemCode>>,
    problems: &[canonical::ValidationProblem],
    paths: &BTreeSet<String>,
) {
    for found in problems {
        if let Some(path) = found.path.to_str().filter(|path| paths.contains(*path)) {
            add_code(entries, path, ProblemCode::from(&found.code));
        }
    }
}

impl RepositoryService {
    /// The comment threads of the item with this ID, read from the item's
    /// effective copy: the worktree created to edit it when there is one,
    /// otherwise the primary copy. `context` says which.
    ///
    /// Root comments are in `created_at` and then ID order, and so are the
    /// replies to each comment, to any depth. The list is flat: a comment
    /// is followed at once by its replies, each of them by its own, and
    /// `depth` says how far below a root comment each one is. `author` is
    /// a comment's `created_by` value and null when it has none; Git
    /// history is never read.
    ///
    /// The index says which files there are: those it holds as the item's
    /// comments, and those directly inside the item's comment directory it
    /// holds a problem for. Nothing lists the directory, so a comment added
    /// since the last refresh is not returned until the indexer has seen
    /// it. That is how every read works, not something this one reports.
    ///
    /// Each of those files is read as it is now. What is checked again,
    /// over the item's own file and those files alone, is everything one
    /// item's files can show: front matter, fields, the path a comment is
    /// filed at, a parent that is no comment of the item, and a cycle of
    /// parents. What takes the whole context to find is not checked again
    /// and is taken from the problems the last refresh stored: an ID that
    /// another item's comment has too, and a parent that is a comment on
    /// another item. A file with such a stored problem that is no newer
    /// than that refresh keeps it; one that is newer is checked like any
    /// other, and the index is `stale`.
    ///
    /// A file that is not a comment of the item by either means, that is
    /// not valid UTF-8, or that is no longer a regular file follows the
    /// threads as a nonconforming entry, in path order, with a null ID,
    /// its path and the reason. A file that is gone is left out.
    ///
    /// `complete` is false when the last refresh could not read the item's
    /// comment directory, or the directory that holds every item's, or
    /// stopped part of the way through it: there may be comments it never
    /// saw. `index.state` is `stale` when the item's file or a comment's
    /// is newer than the last refresh, or the comments are no longer what
    /// the index stored.
    ///
    /// An item the index does not hold, whose file is gone, or whose file
    /// now holds another item is `item_not_found`, as it is for
    /// `show_item`. A comment the index holds whose file cannot be opened
    /// is `repository_inaccessible`: the list is not returned without it.
    /// A file the refresh itself could not open, and stored as such, is a
    /// nonconforming entry instead.
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
            // What the index stored about the files there that are not
            // comments, and whether it could look at all of them.
            let mut stored_codes: BTreeMap<String, Vec<ProblemCode>> = BTreeMap::new();
            let mut complete = true;
            for stored in stored_problems(connection, repo)? {
                if &stored.context.worktree != worktree {
                    continue;
                }
                if is_file_in(&directory, &stored.path) {
                    add_code(&mut stored_codes, &stored.path, stored.code);
                } else if stored.code == ProblemCode::SourceUnreadable
                    && (stored.path == COMMENTS_DIRECTORY
                        || stored.path == directory.trim_end_matches('/'))
                {
                    // The refresh could not read the directory, or stopped
                    // part of the way through it.
                    complete = false;
                }
            }
            // The files the index knows of there, comments or not.
            let paths: BTreeSet<String> = indexed
                .iter()
                .map(|comment| &comment.path)
                .chain(stored_codes.keys())
                .cloned()
                .collect();

            // Newer than the refresh, the item may no longer be what the
            // index stored its comments against.
            let mut is_behind = item_file.newer_than(refreshed_at);
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
                let stored_here = stored_codes.get(path).map_or(&[][..], Vec::as_slice);
                let unreadable_when_stored = stored_here.contains(&ProblemCode::SourceUnreadable);
                let read = match read_item_file(repo, worktree, path) {
                    Ok(read) => read,
                    // The refresh could not open this file either and
                    // stored that, so the index holds no comment here to
                    // leave out: the file is listed as what it was found
                    // to be. An indexed comment that cannot be opened
                    // fails the read.
                    Err(error)
                        if error.code() == ResultCode::RepositoryInaccessible
                            && unreadable_when_stored
                            && !indexed.iter().any(|comment| &comment.path == path) =>
                    {
                        add_code(&mut entries, path, ProblemCode::SourceUnreadable);
                        continue;
                    }
                    Err(error) => return Err(error),
                };
                match read {
                    ItemFileRead::Found(file) => {
                        let newer = file.newer_than(refreshed_at);
                        is_behind |= newer;
                        // A problem that takes the whole context to find
                        // cannot be found again from this item's files. As
                        // long as the file is the one the refresh saw, the
                        // stored finding stands and the file is kept out of
                        // validation, so that a reply to it has no parent.
                        if !newer
                            && stored_here
                                .iter()
                                .any(|code| CONTEXT_WIDE_PROBLEMS.contains(code))
                        {
                            for code in stored_here {
                                add_code(&mut entries, path, *code);
                            }
                            continue;
                        }
                        match String::from_utf8(file.bytes) {
                            Ok(source) => sources.push((PathBuf::from(path), source)),
                            Err(_) => add_code(&mut entries, path, ProblemCode::SourceUnreadable),
                        }
                    }
                    ItemFileRead::Missing => is_behind = true,
                    ItemFileRead::NotAFile => {
                        // Behind unless that is what the refresh found.
                        is_behind |= !unreadable_when_stored;
                        add_code(&mut entries, path, ProblemCode::SourceUnreadable);
                    }
                }
            }

            let validation = canonical::validate_context(sources);
            add_validation_codes(&mut entries, &validation.problems, &paths);
            let threads = canonical::ordered_comment_threads(&validation);
            let mut current = BTreeSet::new();
            collect_indexed(&threads, &directory, &mut current);
            is_behind |= current != indexed;

            let mut items = flattened(threads, &directory);
            items.extend(
                entries
                    .iter()
                    .map(|(path, codes)| nonconforming_entry(&id, path, codes)),
            );
            Ok(CommentListDto {
                items,
                complete,
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
