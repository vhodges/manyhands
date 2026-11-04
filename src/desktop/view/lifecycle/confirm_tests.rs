//! The lifecycle dialog's confirm stage (D5 design, A2): the step list, the
//! whole branch's changed paths with their tags and the code note, the
//! local-only wording, `closed_by` from the preview, a refused consent that
//! offers Prepare again (P2, P3), a ticket whose status text is `closed`
//! (P13) and the keys (A21). Every test runs over a real local-only
//! repository, except one that renders a remote preview through a canned
//! prepare (decision 9) and says so.

use std::path::{Path, PathBuf};

use gpui_kit::TestAppContext;
use manyhands::{
    repository::{
        Clock, LifecyclePlannedPublication, Mutation, PreviewDetailsDto, RepositoryService,
        TicketCloseInput,
    },
    results::ResultCode,
};

use super::tests::{
    TICKET, closes, code, commits, dialog, open, pane, request, saved_ticket, shell, shown, ticket,
    until_present,
};
use crate::desktop::{
    model::item::CLOSE_TICKET,
    test_support::fixtures::{identity, item_id, ticket_path, ticket_source},
    view::{harness::Shell, item::LIFECYCLE_ACTION},
};

const COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5KF0";
const STALE: &str = "Something changed since you confirmed.";
const CODE_NOTE: &str =
    "This working copy includes code as well as the ticket. All of it is merged.";

/// The ticket's own working copy, where its branch is checked out.
fn worktree(shell: &Shell, root: &Path) -> PathBuf {
    PathBuf::from(shown(shell, root, TICKET).context.worktree)
}

/// The confirm stage's changed paths: (path, tag, change), in order.
fn paths(shell: &Shell, cx: &mut TestAppContext) -> Vec<(String, String, String)> {
    let paths = shell.rows("lifecycle-path", cx);
    let tags = shell.rows("lifecycle-path-tag", cx);
    let changes = shell.rows("lifecycle-path-change", cx);
    assert_eq!(paths.len(), tags.len(), "a tag for every path");
    assert_eq!(paths.len(), changes.len(), "a change for every path");
    paths
        .into_iter()
        .zip(tags)
        .zip(changes)
        .map(|((path, tag), change)| (path, tag, change))
        .collect()
}

fn row(path: &str, tag: &str, change: &str) -> (String, String, String) {
    (path.to_owned(), tag.to_owned(), change.to_owned())
}

/// Opens the ticket's tab and the dialog, and waits for its preview.
async fn confirming(shell: &Shell, root: &Path, cx: &mut TestAppContext) {
    open(shell, ticket(root), cx);
    shell.click(LIFECYCLE_ACTION, cx);
    until_present(shell, "lifecycle-confirm", cx).await;
}

/// A19, real: the summary lists every path the ticket's branch changes,
/// the ticket's own file, a comment, a Markdown file under `docs/` and code,
/// each tagged by its path alone (R-2) with its change, and no line counts
/// (R-3).
#[gpui_kit::test]
async fn the_summary_lists_every_changed_path_with_its_tag(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| saved_ticket(fixture, root, "Paths", ""));
    let context = worktree(&shell, &root);
    let comment = format!(".manyhands/comments/{TICKET}/{COMMENT}.md");
    let source = format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: comment\nid: {COMMENT}\n\
         item_id: {TICKET}\ncreated_at: 2026-11-04T12:00:00Z\n\
         created_by: Ada Example <ada@example.invalid>\n---\nNoted.\n"
    );
    shell
        .fixture
        .commit_file(&context, &comment, source.as_bytes());
    shell
        .fixture
        .commit_file(&context, "docs/guide.md", b"Plain notes.\n");
    shell
        .fixture
        .commit_file(&context, "src/main.rs", b"fn main() {}\n");

    confirming(&shell, &root, cx).await;
    assert_eq!(
        paths(&shell, cx),
        [
            row(&comment, "comment", "added"),
            row(&ticket_path(TICKET), "ticket", "modified"),
            row("docs/guide.md", "document", "added"),
            row("src/main.rs", "code", "added"),
        ]
    );
    assert_eq!(
        shell.label("lifecycle-paths-heading", cx).as_deref(),
        Some("Files that will be merged into main (4)")
    );
    for path in shell.rows("lifecycle-path", cx) {
        assert!(!path.contains('+'), "no line counts: {path}");
    }
    assert!(closes(&shell, cx).is_empty(), "nothing runs before Confirm");
}

/// Real: a branch that changes only the ticket has no code note; once code
/// is committed on it, the summary adds "This working copy includes code as
/// well as the ticket. All of it is merged."
#[gpui_kit::test]
async fn code_on_the_branch_adds_the_code_note(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| saved_ticket(fixture, root, "Code", ""));
    confirming(&shell, &root, cx).await;
    assert_eq!(
        paths(&shell, cx),
        [row(&ticket_path(TICKET), "ticket", "modified")]
    );
    assert!(!shell.present("lifecycle-code-note", cx));
    shell.keys("escape", cx);

    let context = worktree(&shell, &root);
    shell
        .fixture
        .commit_file(&context, "tools/build.sh", b"#!/bin/sh\n");
    shell.click(LIFECYCLE_ACTION, cx);
    until_present(&shell, "lifecycle-confirm", cx).await;
    assert_eq!(
        shell.label("lifecycle-code-note", cx).as_deref(),
        Some(CODE_NOTE)
    );
    assert!(
        paths(&shell, cx).contains(&row("tools/build.sh", "code", "added")),
        "{:?}",
        paths(&shell, cx)
    );
}

/// Real (R-8, GRFC 305): a closure's steps run Save, Keep on this computer,
/// Mark as closed, Merge into main, Tidy up: the closure is recorded before
/// the merge.
#[gpui_kit::test]
async fn closure_steps_mark_closed_before_merge(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| saved_ticket(fixture, root, "Steps", ""));
    confirming(&shell, &root, cx).await;
    let steps = shell.rows("lifecycle-step", cx);
    assert_eq!(
        steps,
        [
            "Save your latest edits",
            "Keep on this computer (sharing pending)",
            "Mark as closed",
            "Merge into main",
            "Tidy up",
        ]
    );
    let marked = steps.iter().position(|step| step == "Mark as closed");
    let merged = steps.iter().position(|step| step == "Merge into main");
    assert!(marked < merged, "{steps:?}");
    assert_eq!(
        shell.rows("lifecycle-step-note", cx).len(),
        steps.len(),
        "a note for every step"
    );
}

/// Real: a repository with no remote keeps the work on this computer. Step
/// 2 says so, there is no step that shares main, the tidy-up names nothing
/// remote and the Git line pushes nothing.
#[gpui_kit::test]
async fn local_only_reads_keep_on_this_computer(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| saved_ticket(fixture, root, "Local", ""));
    confirming(&shell, &root, cx).await;
    let steps = shell.rows("lifecycle-step", cx);
    assert_eq!(steps[1], "Keep on this computer (sharing pending)");
    assert!(
        !steps.iter().any(|step| step.starts_with("Share")),
        "{steps:?}"
    );
    let notes = shell.rows("lifecycle-step-note", cx);
    let tidy = notes.last().expect("the tidy-up's note");
    assert_eq!(tidy, "Remove the working copy and the local branch.");
    let git = shell.label("lifecycle-git", cx).expect("the Git line");
    assert!(git.starts_with("Git: merge "), "{git}");
    assert!(!git.contains("push"), "{git}");
}

/// Canned (decision 9): a repository with a publication remote cannot be
/// reached here. The real preview is the library's
/// (`tests/mutation_remote/c5_lifecycle.rs` `prepare_ready`, the
/// `c5_lifecycle_prepare_ready` cases: remote `origin`, publication
/// `publish`). The canned answer is this repository's real preview with
/// that remote and publication, and a remote branch to delete. Step 2
/// shares with origin, main is shared after the merge, and the tidy-up and
/// Git line name the remote. Nothing is asserted of the repository.
#[gpui_kit::test]
async fn a_remote_preview_shares_with_origin(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        saved_ticket(fixture, root, "Remote", "")
    });
    let mut prepared = shell
        .fixture
        .service()
        .prepare(&Mutation::TicketClose(TicketCloseInput {
            root: root.clone(),
            id: item_id(TICKET),
        }));
    let Some(manyhands::repository::PrepareDataDto::Preview(preview)) = prepared.data.as_mut()
    else {
        panic!("the close is ready");
    };
    let PreviewDetailsDto::Lifecycle(details) = &mut preview.details else {
        panic!("a lifecycle preview");
    };
    details.remote = Some("origin".to_owned());
    details.publication = LifecyclePlannedPublication::Publish;
    details.cleanup.remote_branch = true;
    shell.backend.answer_next_prepare("ticket close", prepared);

    confirming(&shell, &root, cx).await;
    assert_eq!(
        shell.rows("lifecycle-step", cx),
        [
            "Save your latest edits",
            "Share with origin",
            "Mark as closed",
            "Merge into main",
            "Share main with origin",
            "Tidy up",
        ]
    );
    let notes = shell.rows("lifecycle-step-note", cx);
    assert_eq!(
        notes.last().map(String::as_str),
        Some("Remove the working copy, the local branch and the branch on origin.")
    );
    let git = shell.label("lifecycle-git", cx).expect("the Git line");
    assert!(git.contains("push origin"), "{git}");
}

/// Real: the Mark as closed step names who closes it, as the preview gives
/// `closed_by`: the repository's identity.
#[gpui_kit::test]
async fn closure_shows_closed_by_from_the_preview(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| saved_ticket(fixture, root, "Who", ""));
    confirming(&shell, &root, cx).await;
    let steps = shell.rows("lifecycle-step", cx);
    let at = steps
        .iter()
        .position(|step| step == "Mark as closed")
        .expect("a closure step");
    let who = identity();
    assert_eq!(
        shell.rows("lifecycle-step-note", cx)[at],
        format!("Closed by {} <{}>", who.name, who.email)
    );
}

/// P2, A4, real: a path added on the ticket's branch after the preview
/// changes what was confirmed. The library refuses the consent
/// (`external_change`, before anything is accepted): the dialog says
/// something changed and offers Prepare again, and nothing runs. Prepare
/// again shows the new path; Confirm then sends the same request again, and
/// the close finishes.
#[gpui_kit::test]
async fn a_new_path_after_prepare_offers_prepare_again(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| saved_ticket(fixture, root, "Stale", ""));
    confirming(&shell, &root, cx).await;
    assert_eq!(paths(&shell, cx).len(), 1);
    let main = commits(&root, "main");
    let context = worktree(&shell, &root);
    shell
        .fixture
        .commit_file(&context, "src/late.rs", b"// late\n");

    shell.click("lifecycle-confirm", cx);
    let sent = closes(&shell, cx);
    assert_eq!(sent.len(), 1);
    shell.until_ended(sent[0], cx).await;
    assert_eq!(code(&shell, sent[0], cx), Some(ResultCode::ExternalChange));
    until_present(&shell, "lifecycle-stale", cx).await;
    assert_eq!(shell.label("lifecycle-stale", cx).as_deref(), Some(STALE));
    assert_eq!(
        shell.label("lifecycle-code", cx).as_deref(),
        Some(ResultCode::ExternalChange.as_str())
    );
    assert!(!shell.present("lifecycle-confirm", cx));
    assert_eq!(commits(&root, "main"), main, "a refused consent never runs");

    shell.click("lifecycle-prepare-again", cx);
    until_present(&shell, "lifecycle-confirm", cx).await;
    assert!(
        paths(&shell, cx).contains(&row("src/late.rs", "code", "added")),
        "the new summary: {:?}",
        paths(&shell, cx)
    );
    assert_eq!(
        shell.label("lifecycle-code-note", cx).as_deref(),
        Some(CODE_NOTE)
    );
    let fresh = dialog(cx)
        .read_with(cx, |dialog, _| dialog.confirmation())
        .expect("a new confirmation");

    shell.click("lifecycle-confirm", cx);
    assert_eq!(closes(&shell, cx), sent, "the same request");
    shell
        .until(cx, {
            let id = sent[0];
            move |operations| operations.request(id).is_some_and(|r| r.attempt == 2)
        })
        .await;
    shell.until_ended(sent[0], cx).await;
    let again = request(&shell, sent[0], cx);
    assert_eq!(again.confirmation, Some(fresh), "with the new confirmation");
    assert_eq!(code(&shell, sent[0], cx), Some(ResultCode::Ok));
    assert!(commits(&root, "main") > main, "merged once confirmed again");
}

/// A clock a test moves on, which the library times its records by.
struct MovedClock(std::sync::Mutex<time::OffsetDateTime>);

impl Clock for MovedClock {
    fn now(&self) -> time::OffsetDateTime {
        *self.0.lock().unwrap()
    }
}

/// P3, real: a consent older than ten minutes is refused
/// `confirmation_expired`. The dialog says something changed and offers
/// Prepare again, and nothing runs; a fresh preview then closes the ticket.
#[gpui_kit::test]
async fn expired_consent_offers_prepare_again(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| saved_ticket(fixture, root, "Late", ""));
    let clock = std::sync::Arc::new(MovedClock(std::sync::Mutex::new(
        time::OffsetDateTime::now_utc(),
    )));
    shell.backend.replace_service(
        RepositoryService::open_at(shell.fixture.data_dir())
            .unwrap()
            .with_clock_for_testing(clock.clone()),
    );
    confirming(&shell, &root, cx).await;
    let main = commits(&root, "main");
    *clock.0.lock().unwrap() += time::Duration::minutes(11);

    shell.click("lifecycle-confirm", cx);
    let sent = closes(&shell, cx);
    shell.until_ended(sent[0], cx).await;
    assert_eq!(
        code(&shell, sent[0], cx),
        Some(ResultCode::ConfirmationExpired)
    );
    until_present(&shell, "lifecycle-stale", cx).await;
    assert_eq!(shell.label("lifecycle-stale", cx).as_deref(), Some(STALE));
    assert_eq!(
        shell.label("lifecycle-code", cx).as_deref(),
        Some(ResultCode::ConfirmationExpired.as_str())
    );
    assert_eq!(commits(&root, "main"), main, "nothing ran");

    shell.click("lifecycle-prepare-again", cx);
    until_present(&shell, "lifecycle-confirm", cx).await;
    shell.click("lifecycle-confirm", cx);
    shell
        .until(cx, {
            let id = sent[0];
            move |operations| operations.request(id).is_some_and(|r| r.attempt == 2)
        })
        .await;
    shell.until_ended(sent[0], cx).await;
    assert_eq!(code(&shell, sent[0], cx), Some(ResultCode::Ok));
    assert_eq!(closes(&shell, cx), sent, "one request");
}

/// P13, real: a ticket whose status text is `closed` is not closed by its
/// lifecycle. It offers Close ticket…, its preview records the closure, and
/// the close runs; the ticket then holds its lifecycle closure, and its
/// status text stays as it was.
#[gpui_kit::test]
async fn status_text_closed_still_closes(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        let source =
            ticket_source(TICKET, "Untitled", "").replace("status: open", "status: closed");
        fixture.commit_file(root, &ticket_path(TICKET), source.as_bytes());
        fixture.refresh(root);
        fixture.save_ticket(root, TICKET, "Said closed");
    });
    assert_eq!(
        shown(&shell, &root, TICKET).status.as_deref(),
        Some("closed")
    );
    open(&shell, ticket(&root), cx);
    assert_eq!(
        shell.label(LIFECYCLE_ACTION, cx).as_deref(),
        Some(CLOSE_TICKET)
    );
    shell.click(LIFECYCLE_ACTION, cx);
    until_present(&shell, "lifecycle-confirm", cx).await;
    assert!(
        shell
            .rows("lifecycle-step", cx)
            .contains(&"Mark as closed".to_owned())
    );

    shell.click("lifecycle-confirm", cx);
    let sent = closes(&shell, cx);
    shell.until_ended(sent[0], cx).await;
    assert_eq!(code(&shell, sent[0], cx), Some(ResultCode::Ok));
    let closed = shown(&shell, &root, TICKET);
    let closure = closed.closure.expect("a ticket's closure");
    assert!(closure.closed_at.is_some(), "lifecycle closed");
    assert_eq!(closed.status.as_deref(), Some("closed"), "status apart");
}

/// A21: from the keyboard, Enter on the header's action opens the dialog;
/// Escape cancels with nothing sent and gives focus back to the action's
/// focus stop, from which one Tab is the action; Enter confirms, sending the
/// close once with its confirmation; Escape then closes, and focus goes back
/// again.
#[gpui_kit::test]
async fn enter_confirms_esc_cancels_and_focus_returns(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| saved_ticket(fixture, root, "Keys", ""));
    open(&shell, ticket(&root), cx);
    let focus = pane(&shell, cx).read_with(cx, |pane, _| pane.lifecycle_focus());
    let returned = |shell: &Shell, cx: &mut TestAppContext| {
        shell.frame(cx, |window, _| focus.is_focused(window))
    };

    shell.click(LIFECYCLE_ACTION, cx);
    until_present(&shell, "lifecycle-confirm", cx).await;
    shell.keys("escape", cx);
    assert!(!shell.present("lifecycle-dialog", cx));
    assert!(closes(&shell, cx).is_empty(), "Escape sends nothing");
    assert!(returned(&shell, cx), "focus is back at the action");
    assert!(
        shell.tab_reaches(LIFECYCLE_ACTION, 1, cx),
        "one Tab is the action"
    );

    shell.press("enter", cx);
    until_present(&shell, "lifecycle-confirm", cx).await;
    let held = dialog(cx)
        .read_with(cx, |dialog, _| dialog.confirmation())
        .expect("a preview");
    shell.keys("enter", cx);
    let sent = closes(&shell, cx);
    assert_eq!(sent.len(), 1, "Enter confirms once");
    assert_eq!(request(&shell, sent[0], cx).confirmation, Some(held));
    shell.keys("escape", cx);
    assert!(!shell.present("lifecycle-dialog", cx));
    assert!(returned(&shell, cx), "focus is back at the action");
    shell.until_ended(sent[0], cx).await;
    assert_eq!(closes(&shell, cx), sent, "sent once");
}
