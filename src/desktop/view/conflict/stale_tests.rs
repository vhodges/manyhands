//! P11: a stale observation refreshes the comparison (B3's table). Shares
//! `conflict_tests.rs`'s helpers.

use gpui_kit::TestAppContext;
use manyhands::{
    repository::OperationId,
    results::{Effects, Envelope, ResultCode, Scope},
};

use super::{
    CHANGED, CHOOSE_ONE, KEEPING_THEIRS, SheetSource,
    tests::{
        OTHER, TICKET, canned, canned_path, code, open_sheet, remote_shell, resolves, same_body,
        until_long, until_present,
    },
};
use crate::desktop::{services::JobKind, test_support::fixtures::ticket_source};

/// P11, canned (decision 9; Task 11b review, R-26): a side changes while
/// the sheet is open, so its observation is stale. No real local trigger
/// leaves the work context tidy: any change to it is now R-26's
/// `external_repair`, and a new commit on main does not change a recorded
/// local conflict's observation. The answer is the one
/// `tests/mutation_remote/resolution.rs` `c4_resolution_another_observation`
/// gives (`external_change`, nothing written, no recovery); the conflicts
/// are the shape `c4_conflict_read_canonical` reads, the second with a new
/// observation and theirs changed on its second path. The sheet says so,
/// reads again, and keeps the choice of the part that did not change.
#[gpui_kit::test]
async fn a_stale_observation_refreshes_the_comparison(cx: &mut TestAppContext) {
    let (shell, root) = remote_shell(cx);
    let operation = OperationId::new();
    let paths = vec![canned_path(TICKET, '1'), canned_path(OTHER, '2')];
    shell
        .backend
        .answer_next_conflict_read(canned(operation, paths.clone()));
    let mut changed = canned(operation, paths);
    changed.observation = Some(format!("conflict-v1:{}", "9".repeat(64)));
    changed.paths[1].incoming = Some(same_body(&ticket_source(OTHER, "Moved", ""), "Moved"));
    shell.backend.answer_next_conflict_read(changed);
    let sheet = open_sheet(&shell, SheetSource::Share, &root, operation, cx);
    until_present(&shell, "conflict-save", cx).await;
    let first = sheet.read_with(cx, |sheet, _| sheet.observation());
    shell.click(("conflict-keep-theirs", 0usize), cx);
    shell.click(("conflict-keep-mine", 1usize), cx);

    shell
        .backend
        .hold_job_return_answering(
            JobKind::ConflictResolve,
            Envelope::classified_mutation(
                "conflict resolve",
                Scope::default(),
                false,
                ResultCode::ExternalChange,
                Effects::not_requested(),
                false,
            ),
        )
        .release();
    shell.click("conflict-save", cx);
    let sent = resolves(&shell, cx);
    assert_eq!(sent.len(), 1);
    until_present(&shell, "conflict-notice", cx).await;
    assert_eq!(code(&shell, sent[0], cx), Some(ResultCode::ExternalChange));
    assert_eq!(shell.label("conflict-notice", cx).as_deref(), Some(CHANGED));
    until_long(&shell, "the read again", cx, |_, cx| {
        sheet.read_with(cx, |sheet, _| sheet.observation()) != first
    })
    .await;
    assert_eq!(
        sheet.read_with(cx, |sheet, _| sheet.observation()),
        Some(format!("conflict-v1:{}", "9".repeat(64)))
    );
    assert_eq!(
        shell.label("conflict-progress", cx).as_deref(),
        Some("1 of 2 chosen"),
        "only the unchanged part keeps its choice"
    );
    assert_eq!(
        shell.label(("conflict-choice", 0usize), cx).as_deref(),
        Some(KEEPING_THEIRS)
    );
    assert_eq!(
        shell.label(("conflict-choice", 1usize), cx).as_deref(),
        Some(CHOOSE_ONE)
    );
}
