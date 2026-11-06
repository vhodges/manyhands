//! D5's privacy sweep (A16) over an approve and merge, over a real
//! local-only repository: a document whose title, body and comment hold
//! sentinels is stopped at a safe point, refused at its tidy-up with its
//! banner, and finished. `privacy_tests.rs` holds the sentinels and the
//! sweep (`Seen`).

use std::path::{Path, PathBuf};

use gpui_kit::TestAppContext;
use manyhands::{
    repository::{
        CancellationToken, DocumentDraft, DocumentSaveInput, ItemDtoKind, Mutation, MutationCall,
        NoProgress, ProgressStage, RequestId, keys::SessionCredentials,
    },
    results::ResultCode,
};

use super::{
    outcome::{ENDING_FAILED, ENDING_STOPPED},
    outcome_tests::at_stage,
    privacy_tests::{
        BODY, Seen, TITLE, commit_sentinel_comment, shows_body_and_comment, until_long,
    },
    tests::{DOCUMENT, code, open, request, shell, shown, until_present},
};
use crate::desktop::{
    model::navigation::OpenTarget,
    services::{
        JobKind,
        requests::{DesktopRequestId, Intent},
    },
    test_support::fixtures::{Fixture, NoPassphrase, item_id},
    view::{
        harness::{Shell, key},
        item::LIFECYCLE_ACTION,
    },
};

/// A document committed on main, titled "Untitled", with the sentinel
/// comment, then saved through the library with `TITLE` and `BODY`: it has
/// a working copy of its own to approve and merge.
fn saved_document(fixture: &Fixture, root: &Path) {
    let source = format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: {DOCUMENT}\n\
         title: Untitled\n---\nUntitled.\n"
    );
    fixture.commit_file(root, "docs/plan.md", source.as_bytes());
    commit_sentinel_comment(fixture, root, DOCUMENT);
    fixture.refresh(root);
    let service = fixture.service();
    let repo = service.resolve_repository(root).unwrap();
    let shown = service.show_item(&repo, &item_id(DOCUMENT)).unwrap();
    let saved = service.execute(
        MutationCall {
            request_id: RequestId::new(),
            confirmation: None,
            mutation: Mutation::DocumentSave(DocumentSaveInput {
                root: root.to_owned(),
                id: item_id(DOCUMENT),
                draft: DocumentDraft {
                    title: TITLE.to_owned(),
                    body: format!("{BODY} is the body.\n"),
                },
                observation: shown.observation,
            }),
        },
        &mut SessionCredentials::new(NoPassphrase),
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(saved.code, ResultCode::Ok, "{}", saved.message);
}

fn promotes(shell: &Shell, cx: &mut TestAppContext) -> Vec<DesktopRequestId> {
    shell.requests(cx, |request| {
        matches!(request.intent, Intent::DocumentPromote { .. })
    })
}

/// A16, real: a document whose title, body and comment hold sentinels is
/// approved and merged. The run is confirmed, held running at
/// `integrating` and stopped there; Retry merges it, but a file left in its
/// working copy while it tidies up stops the tidy-up (`context_changed`,
/// the recipe of `lock_tests.rs`
/// `a_changed_context_cleanup_unlocks_with_check_again`), which raises the
/// "Merged, but not tidied up" banner; with the file gone, the banner's
/// Check again finishes it. No sentinel reaches an entry, an answer, the
/// operation area, the banner or the dialog's messages and outcome.
#[gpui_kit::test]
async fn a16_a_promotion_stopped_untidied_and_done_shows_no_item_text(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, saved_document);
    let context = PathBuf::from(shown(&shell, &root, DOCUMENT).context.worktree);
    let mut seen = Seen::default();

    // Viewing, then the confirm stage.
    open(
        &shell,
        OpenTarget::Item {
            repo: key(&root),
            kind: ItemDtoKind::Document,
            id: item_id(DOCUMENT),
        },
        cx,
    );
    shows_body_and_comment(&shell, cx);
    seen.window(&shell, cx);
    shell.click(LIFECYCLE_ACTION, cx);
    until_present(&shell, "lifecycle-confirm", cx).await;
    seen.window(&shell, cx);

    // Running, held at `integrating`, then stopped there.
    let gate = shell
        .backend
        .hold_at_stage(JobKind::DocumentPromote, ProgressStage::Integrating);
    shell.click("lifecycle-confirm", cx);
    gate.wait_reached();
    let first = promotes(&shell, cx)[0];
    at_stage(&shell, first, ProgressStage::Integrating, cx).await;
    until_present(&shell, "lifecycle-stop", cx).await;
    seen.window(&shell, cx);
    shell.click("lifecycle-stop", cx);
    seen.window(&shell, cx);
    gate.release();
    shell.until_ended(first, cx).await;
    assert_eq!(code(&shell, first, cx), Some(ResultCode::Cancelled));
    until_present(&shell, "lifecycle-ending", cx).await;
    assert_eq!(
        shell.label("lifecycle-ending", cx).as_deref(),
        Some(ENDING_STOPPED)
    );
    seen.window(&shell, cx);

    // Retry merges; the tidy-up is refused for a file left behind.
    let gate = shell
        .backend
        .hold_at_stage(JobKind::DocumentPromote, ProgressStage::CleaningUp);
    shell.click("lifecycle-retry", cx);
    gate.wait_reached();
    let left = context.join("untracked.txt");
    std::fs::write(&left, b"the owner's\n").unwrap();
    gate.release();
    let retried = promotes(&shell, cx)[0];
    shell.until_ended(retried, cx).await;
    assert_eq!(
        code(&shell, retried, cx),
        Some(ResultCode::RecoveryRequired)
    );
    until_present(&shell, "lifecycle-ending", cx).await;
    assert_eq!(
        shell.label("lifecycle-ending", cx).as_deref(),
        Some(ENDING_FAILED)
    );
    seen.window(&shell, cx);
    shell.keys("escape", cx);
    until_present(&shell, "item-not-tidied", cx).await;
    seen.window(&shell, cx);
    assert!(seen.read_at("banner", "Merged, but not tidied up"));

    // Check again, with the file gone, finishes it.
    std::fs::remove_file(&left).unwrap();
    shell.click("item-not-tidied-check-again", cx);
    until_long(&shell, "the tidy-up", cx, |shell, cx| {
        request(shell, retried, cx).attempt == 2 && code(shell, retried, cx).is_some()
    })
    .await;
    assert_eq!(code(&shell, retried, cx), Some(ResultCode::Ok));
    until_long(&shell, "the banner's clearing", cx, |shell, cx| {
        !shell.present("item-not-tidied", cx)
    })
    .await;
    seen.window(&shell, cx);

    seen.holds_none(&[
        "operation entry",
        "operation area",
        "banner",
        "dialog message",
        "drawn",
    ]);
    assert!(seen.read_at("dialog message", ENDING_STOPPED));
    for at in ["item-title", "lifecycle-subtitle"] {
        assert!(seen.allowed_at(at), "a sentinel is drawn in {at}");
    }
}
