//! A consent the library refuses, as the lifecycle dialog shows it (D5
//! design, A2, Stale): a path added after the preview (P2, A4) and a
//! confirmation that expired (P3). Nothing runs; Prepare again, by click or
//! Enter, shows the new summary, and Confirm then sends the same request
//! again. All over real local-only repositories.

use gpui_kit::TestAppContext;
use manyhands::{
    repository::{Clock, RepositoryService},
    results::ResultCode,
};

use super::{
    confirm_tests::{CODE_NOTE, STALE, confirming, paths, row, worktree},
    tests::{closes, code, commits, dialog, request, saved_ticket, shell, until_present},
};

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
    assert!(
        !shell.present("lifecycle-stale", cx),
        "the first attempt's answer is not shown again"
    );
    shell.until_ended(sent[0], cx).await;
    assert!(!shell.present("lifecycle-stale", cx), "the second ran");
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
    assert!(
        !shell.present("lifecycle-stale", cx),
        "the first attempt's answer is not shown again"
    );
    shell.until_ended(sent[0], cx).await;
    assert!(!shell.present("lifecycle-stale", cx), "the second ran");
    assert_eq!(code(&shell, sent[0], cx), Some(ResultCode::Ok));
    assert_eq!(closes(&shell, cx), sent, "one request");
}

/// Optional 3 (review): Enter on Stale prepares again. Real, as P2: a path
/// added after the preview refuses the consent; Enter then shows the new
/// summary, and nothing has run.
#[gpui_kit::test]
async fn enter_on_stale_prepares_again(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| saved_ticket(fixture, root, "Again", ""));
    confirming(&shell, &root, cx).await;
    let main = commits(&root, "main");
    let context = worktree(&shell, &root);
    shell
        .fixture
        .commit_file(&context, "src/again.rs", b"// again\n");
    shell.click("lifecycle-confirm", cx);
    let sent = closes(&shell, cx);
    shell.until_ended(sent[0], cx).await;
    until_present(&shell, "lifecycle-stale", cx).await;

    shell.keys("enter", cx);
    until_present(&shell, "lifecycle-confirm", cx).await;
    assert!(!shell.present("lifecycle-stale", cx));
    assert!(
        paths(&shell, cx).contains(&row("src/again.rs", "code", "added")),
        "{:?}",
        paths(&shell, cx)
    );
    assert_eq!(closes(&shell, cx), sent);
    assert_eq!(commits(&root, "main"), main, "nothing ran");
}
