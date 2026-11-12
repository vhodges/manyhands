//! Which list an unreadable file belongs to (Part I, P28).

use manyhands::{
    repository::{IndexProblemDto, IndexStatusDto, IndexStatusState},
    results::ProblemCode,
};

use super::{UnreadableFile, unreadable_rows};

const ROOT: &str = "/repo";
const ITEM: &str = "/repo/.manyhands/worktrees/X";

fn status(problems: Vec<IndexProblemDto>) -> IndexStatusDto {
    IndexStatusDto {
        state: IndexStatusState::Current,
        refreshed_at: None,
        context_count: None,
        item_count: None,
        problem_count: u64::try_from(problems.len()).ok(),
        problems,
        pending_operations: Vec::new(),
    }
}

fn problem(code: ProblemCode, path: Option<&str>, worktree: Option<&str>) -> IndexProblemDto {
    IndexProblemDto {
        code,
        path: path.map(str::to_owned),
        worktree: worktree.map(str::to_owned),
    }
}

fn file(path: &str, worktree: &str, primary: bool) -> UnreadableFile {
    UnreadableFile {
        path: path.to_owned(),
        worktree: worktree.to_owned(),
        primary,
    }
}

/// Where a problem's file goes.
enum Goes {
    Tickets(UnreadableFile),
    Documents(UnreadableFile),
    Neither,
}

/// Only an unreadable `.md` file, found by a relative path in a known
/// worktree, under `.manyhands/tickets/` or `docs/`, is listed; the rest are
/// Activity's or the incomplete alert's.
#[test]
fn unreadable_files_go_to_their_list() {
    let ticket = ".manyhands/tickets/X/ticket.md";
    let cases = [
        (
            "a ticket file",
            problem(ProblemCode::SourceUnreadable, Some(ticket), Some(ROOT)),
            Goes::Tickets(file(ticket, ROOT, true)),
        ),
        (
            "a document file",
            problem(
                ProblemCode::SourceUnreadable,
                Some("docs/a/b.md"),
                Some(ROOT),
            ),
            Goes::Documents(file("docs/a/b.md", ROOT, true)),
        ),
        (
            "the same document path in an item worktree is not primary",
            problem(
                ProblemCode::SourceUnreadable,
                Some("docs/a/b.md"),
                Some(ITEM),
            ),
            Goes::Documents(file("docs/a/b.md", ITEM, false)),
        ),
        (
            "a ticket file in an item worktree",
            problem(ProblemCode::SourceUnreadable, Some(ticket), Some(ITEM)),
            Goes::Tickets(file(ticket, ITEM, false)),
        ),
        (
            "a directory",
            problem(ProblemCode::SourceUnreadable, Some("docs/a"), Some(ROOT)),
            Goes::Neither,
        ),
        (
            "a ticket directory",
            problem(
                ProblemCode::SourceUnreadable,
                Some(".manyhands/tickets/X"),
                Some(ROOT),
            ),
            Goes::Neither,
        ),
        (
            "a file elsewhere",
            problem(ProblemCode::SourceUnreadable, Some("src/x.md"), Some(ROOT)),
            Goes::Neither,
        ),
        (
            "a folder that only starts with docs",
            problem(
                ProblemCode::SourceUnreadable,
                Some("docs-old/a.md"),
                Some(ROOT),
            ),
            Goes::Neither,
        ),
        (
            "a comment file",
            problem(
                ProblemCode::SourceUnreadable,
                Some(".manyhands/comments/X/C.md"),
                Some(ROOT),
            ),
            Goes::Neither,
        ),
        (
            "an absolute path",
            problem(
                ProblemCode::SourceUnreadable,
                Some("/repo/docs/a.md"),
                Some(ROOT),
            ),
            Goes::Neither,
        ),
        (
            "no path",
            problem(ProblemCode::SourceUnreadable, None, Some(ROOT)),
            Goes::Neither,
        ),
        (
            "no worktree",
            problem(ProblemCode::SourceUnreadable, Some("docs/a.md"), None),
            Goes::Neither,
        ),
        (
            "another code at a ticket path",
            problem(ProblemCode::MissingFrontMatter, Some(ticket), Some(ROOT)),
            Goes::Neither,
        ),
        (
            "another code at a document path",
            problem(
                ProblemCode::MissingFrontMatter,
                Some("docs/a.md"),
                Some(ROOT),
            ),
            Goes::Neither,
        ),
    ];
    for (name, problem, goes) in cases {
        let (tickets, documents) = match goes {
            Goes::Tickets(file) => (vec![file], Vec::new()),
            Goes::Documents(file) => (Vec::new(), vec![file]),
            Goes::Neither => (Vec::new(), Vec::new()),
        };
        assert_eq!(
            unreadable_rows(&status(vec![problem]), ROOT),
            (tickets, documents),
            "{name}"
        );
    }

    // Together: each list keeps the status's order, and the same file named
    // twice is one row, since the tree keys a row by worktree and path.
    let both = status(vec![
        problem(ProblemCode::SourceUnreadable, Some("docs/b.md"), Some(ROOT)),
        problem(ProblemCode::SourceUnreadable, Some(ticket), Some(ROOT)),
        problem(ProblemCode::SourceUnreadable, Some("docs/a.md"), Some(ROOT)),
        problem(ProblemCode::SourceUnreadable, Some("docs/a.md"), Some(ROOT)),
        problem(ProblemCode::SourceUnreadable, Some("docs/a.md"), Some(ITEM)),
    ]);
    assert_eq!(
        unreadable_rows(&both, ROOT),
        (
            vec![file(ticket, ROOT, true)],
            vec![
                file("docs/b.md", ROOT, true),
                file("docs/a.md", ROOT, true),
                file("docs/a.md", ITEM, false),
            ]
        )
    );
}
