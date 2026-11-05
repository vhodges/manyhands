//! Handoff 2g's elements the sheet draws (Task 11b review): the meta line,
//! the description, line tints with their gutters, the Result note and the
//! footer's Git line; and R-26's outside placeholder for a work context
//! changed outside the app. Real local conflicts; no line counts (R-3).

use gpui_kit::TestAppContext;

use super::{
    DESCRIPTION_LOCAL, RESULT_NOTE, SheetSource,
    tests::{SHORT, TICKET, local_shell, open_sheet, until_present},
};
use crate::desktop::test_support::fixtures::ticket_path;

/// The meta line names the kind, the short code and the repository; the
/// description says what to do; each of Yours' and Theirs' lines carries
/// its gutter against Before (`~` changed, `+` added); the Result has its
/// note; the footer gives the Git detail: the paths and both commits.
#[gpui_kit::test]
async fn the_sheet_draws_the_handoffs_elements(cx: &mut TestAppContext) {
    let (shell, root, operation) = local_shell(cx, |fixture, root| {
        fixture.conflict_locally_with_body(
            root,
            TICKET,
            "Mine",
            Some("Mine body.\nAdded.\n"),
            "Theirs",
        )
    });
    let (inspection, _) = shell.fixture.read_local_conflict(&root, operation);
    open_sheet(&shell, SheetSource::Local, &root, operation, cx);
    until_present(&shell, "conflict-save", cx).await;

    assert_eq!(shell.label("conflict-kind", cx).as_deref(), Some("ticket"));
    assert_eq!(shell.label("conflict-short", cx).as_deref(), Some(SHORT));
    assert_eq!(shell.label("conflict-repository", cx).as_deref(), Some("a"));
    assert_eq!(
        shell.label("conflict-description", cx).as_deref(),
        Some(DESCRIPTION_LOCAL)
    );
    assert_eq!(
        shell.rows("conflict-line", cx),
        [
            "~ Mine",
            "~ Theirs",
            "~ Mine body.",
            "+ Added.",
            "~ Body of Theirs.",
        ]
    );
    assert_eq!(
        shell.label(("conflict-result-note", 0usize), cx).as_deref(),
        Some(RESULT_NOTE)
    );
    let short = |oid: &str| oid.chars().take(8).collect::<String>();
    assert_eq!(
        shell.label("conflict-git", cx),
        Some(format!(
            "Git: conflict in {} \u{b7} ours {} \u{b7} theirs {}",
            ticket_path(TICKET),
            short(&inspection.context_parent),
            short(&inspection.primary_parent),
        ))
    );
}

/// R-26, real local: an untracked file in the work context makes the
/// library's way on `external_repair`, so the sheet shows the outside
/// placeholder and offers no Save resolution.
#[gpui_kit::test]
async fn an_external_repair_shows_the_outside_placeholder(cx: &mut TestAppContext) {
    let (shell, root, operation) = local_shell(cx, |fixture, root| {
        let operation = fixture.conflict_locally(root, TICKET, "Mine", "Theirs");
        let worktree = fixture.context_worktree(root, TICKET);
        std::fs::write(worktree.join("untracked.txt"), b"x").unwrap();
        operation
    });
    open_sheet(&shell, SheetSource::Local, &root, operation, cx);
    until_present(&shell, ("conflict-external", 0usize), cx).await;
    assert!(
        shell
            .label(("conflict-external", 0usize), cx)
            .unwrap()
            .starts_with(&ticket_path(TICKET))
    );
    assert!(!shell.present("conflict-save", cx));
    assert!(!shell.present("conflict-retry", cx));
}
