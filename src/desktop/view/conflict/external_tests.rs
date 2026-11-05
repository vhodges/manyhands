//! B4's external guidance (Task 12): a set that goes outside shows a card
//! per path, "This file needs fixing outside Manyhands", with its reason,
//! and Open folder and Check again; it has no Save resolution and no
//! winner. Check again reads the conflict again and offers Resume only
//! when the library's `next_action` says so.
//!
//! The remote reads are canned (decision 9; `answer_next_conflict_read`,
//! as `conflict_tests.rs` explains): the SSH harness does not reach desktop
//! tests. Each canned read is built from library types, in the shape the
//! library test its doc names reads, and nothing is asserted of the
//! repository. R-26's real local case is `handoff_tests.rs`'s.

use std::path::{Path, PathBuf};

use gpui_kit::TestAppContext;
use manyhands::repository::{
    ConflictDto, ConflictPathDto, ConflictPathKind, ConflictPathStructure, ConflictReadEligibility,
    ConflictReadNextAction, ConflictState, OperationId,
};

use super::{
    SheetSource,
    external::{CLOSURES, CODE, OUTSIDE, REPAIR_NOT_DONE, REPAIRED, WAITS},
    tests::{TICKET, canned, canned_path, open_sheet_in, remote_shell, until_present},
};
use crate::desktop::{
    services::requests::Intent,
    test_support::fixtures::{ticket_path, ticket_source},
    view::item::OpenedFolders,
};

/// A live remote conflict that must be repaired outside, of the shape
/// `tests/mutation_remote/conflict_read.rs`
/// `c4_conflict_read_noncanonical_or_structural` reads: the ticket's
/// regular text beside files that are no item's, a binary, a regular text
/// file the classifier reads as binary (K-4), and an executable one, with
/// no sides for those.
fn noncanonical(operation: OperationId) -> ConflictDto {
    let source = |title: &str| ticket_source(TICKET, title, "");
    let other = |path: &str, structure, n: char| ConflictPathDto {
        display_path: path.to_owned(),
        token: format!("conflict-path-v1:{}", n.to_string().repeat(64)),
        kind: ConflictPathKind::Noncanonical,
        structure,
        content_available: false,
        base: None,
        local: None,
        incoming: None,
    };
    let ticket = ConflictPathDto {
        base: Some(source("Base")),
        local: Some(source("Local")),
        incoming: Some(source("Incoming")),
        ..canned_path(TICKET, '1')
    };
    ConflictDto {
        next_action: ConflictReadNextAction::ExternalRepair,
        eligibility: Some(ConflictReadEligibility::ExternalResolutionRequired),
        ..canned(
            operation,
            vec![
                ticket,
                other("blob.bin", ConflictPathStructure::Binary, '2'),
                other("plain.sh", ConflictPathStructure::Binary, '3'),
                other("tool.sh", ConflictPathStructure::Executable, '4'),
            ],
        )
    }
}

/// The read of `operation` once no conflict is live, in the state and way
/// on given, as `tests/mutation_remote/conflict_read.rs`'s `settled`
/// builds it: no observation, eligibility or path.
fn settled(
    operation: OperationId,
    state: ConflictState,
    next_action: ConflictReadNextAction,
) -> ConflictDto {
    ConflictDto {
        state,
        next_action,
        eligibility: None,
        observation: None,
        ..canned(operation, Vec::new())
    }
}

/// The folder a test sheet names as the item's work folder.
fn folder(root: &Path) -> PathBuf {
    root.join(".manyhands/worktrees/work")
}

/// A12, canned (`c4_conflict_read_noncanonical_or_structural`): every path
/// has its card with the outside words and its reason: the ticket, which
/// could be chosen here, waits for the others (R-6). There is no Save
/// resolution, Retry, choice, progress or Result, and Ctrl+Enter sends
/// nothing. Open folder opens the item's work folder.
#[gpui_kit::test]
async fn an_external_set_has_no_save(cx: &mut TestAppContext) {
    let (shell, root) = remote_shell(cx);
    let operation = OperationId::new();
    shell
        .backend
        .answer_next_conflict_read(noncanonical(operation));
    open_sheet_in(
        &shell,
        SheetSource::Share,
        &root,
        operation,
        Some(folder(&root)),
        cx,
    );
    until_present(&shell, ("conflict-external", 0usize), cx).await;

    let paths = [
        ticket_path(TICKET),
        "blob.bin".into(),
        "plain.sh".into(),
        "tool.sh".into(),
    ];
    for (index, path) in paths.iter().enumerate() {
        assert_eq!(
            shell.label(("conflict-external", index), cx),
            Some(format!("{path}: {OUTSIDE}"))
        );
    }
    assert_eq!(
        shell.rows("conflict-external-reason", cx),
        [WAITS, CODE, CODE, CODE]
    );
    for absent in [
        "conflict-save",
        "conflict-retry",
        "conflict-resume",
        "conflict-progress",
    ] {
        assert!(!shell.present(absent, cx), "{absent}");
    }
    assert!(!shell.present(("conflict-keep-mine", 0usize), cx));
    assert!(!shell.present(("conflict-result", 0usize), cx));
    assert!(shell.present("conflict-check-again", cx));

    shell.keys("ctrl-enter", cx);
    assert!(
        shell
            .requests(cx, |request| matches!(
                request.intent,
                Intent::ConflictResolve { .. } | Intent::LocalContextResolve { .. }
            ))
            .is_empty(),
        "nothing is saved"
    );

    shell.click("conflict-open-folder", cx);
    let opened = cx.update(|cx| {
        cx.try_global::<OpenedFolders>()
            .map(|opened| opened.0.clone())
            .unwrap_or_default()
    });
    assert_eq!(opened, [folder(&root)]);
}

/// B4, canned: Check again reads the conflict again. A repair the library
/// refuses (`c4_conflict_read_repair_refused`: `repair_refused`,
/// `external_repair`) keeps the guidance, with no Resume; a repair it
/// accepts (`c4_conflict_read_repaired`: `repaired`, `resume`) offers
/// Resume, which resumes the synchronization, and still no Save.
#[gpui_kit::test]
async fn check_again_offers_resume_only_when_the_library_does(cx: &mut TestAppContext) {
    let (shell, root) = remote_shell(cx);
    let operation = OperationId::new();
    shell
        .backend
        .answer_next_conflict_read(noncanonical(operation));
    open_sheet_in(&shell, SheetSource::Share, &root, operation, None, cx);
    until_present(&shell, "conflict-check-again", cx).await;
    assert!(!shell.present("conflict-resume", cx));

    shell.backend.answer_next_conflict_read(settled(
        operation,
        ConflictState::RepairRefused,
        ConflictReadNextAction::ExternalRepair,
    ));
    shell.click("conflict-check-again", cx);
    until_present(&shell, "conflict-repair-not-done", cx).await;
    assert_eq!(
        shell.label("conflict-repair-not-done", cx).as_deref(),
        Some(REPAIR_NOT_DONE)
    );
    assert!(!shell.present("conflict-resume", cx), "refused: no Resume");
    assert!(!shell.present("conflict-save", cx));
    assert!(shell.present("conflict-check-again", cx));
    // With no work folder named, Open folder opens the repository's.
    shell.click("conflict-open-folder", cx);
    let opened = cx.update(|cx| {
        cx.try_global::<OpenedFolders>()
            .map(|opened| opened.0.clone())
            .unwrap_or_default()
    });
    assert_eq!(opened, std::slice::from_ref(&root));

    shell.backend.answer_next_conflict_read(settled(
        operation,
        ConflictState::Repaired,
        ConflictReadNextAction::Resume,
    ));
    shell.click("conflict-check-again", cx);
    until_present(&shell, "conflict-resume", cx).await;
    assert_eq!(shell.label("conflict-state", cx).as_deref(), Some(REPAIRED));
    assert!(!shell.present("conflict-save", cx));
    assert!(!shell.present("conflict-check-again", cx));

    shell.click("conflict-resume", cx);
    let resumes = shell.requests(cx, |request| {
        matches!(
            request.intent,
            Intent::OperationResume { operation_id, .. } if operation_id == operation
        )
    });
    assert_eq!(resumes.len(), 1, "Resume resumes the synchronization");
}

/// P10, A12, canned (`c4_conflict_read_incompatible_closures`): two
/// different closures of the ticket are given with their sides, yet the
/// set goes outside whole. The card says so, and nothing offers a winner:
/// no choice, no Result, no Save resolution.
#[gpui_kit::test]
async fn incompatible_closures_offer_no_winner(cx: &mut TestAppContext) {
    let (shell, root) = remote_shell(cx);
    let operation = OperationId::new();
    let closed = |at: &str| {
        ticket_source(
            TICKET,
            "Base",
            &format!("closed_at: {at}\nclosed_by: Fixture <fixture@example.invalid>\n"),
        )
    };
    let path = ConflictPathDto {
        structure: ConflictPathStructure::IncompatibleClosures,
        base: Some(ticket_source(TICKET, "Base", "")),
        local: Some(closed("2026-10-25T12:00:00Z")),
        incoming: Some(closed("2026-10-26T12:00:00Z")),
        ..canned_path(TICKET, '1')
    };
    shell.backend.answer_next_conflict_read(ConflictDto {
        next_action: ConflictReadNextAction::ExternalRepair,
        eligibility: Some(ConflictReadEligibility::ExternalResolutionRequired),
        ..canned(operation, vec![path])
    });
    open_sheet_in(&shell, SheetSource::Close(None), &root, operation, None, cx);
    until_present(&shell, ("conflict-external", 0usize), cx).await;

    assert_eq!(
        shell.label(("conflict-external", 0usize), cx),
        Some(format!("{}: {OUTSIDE}", ticket_path(TICKET)))
    );
    assert_eq!(shell.rows("conflict-external-reason", cx), [CLOSURES]);
    assert!(!shell.present(("conflict-keep-mine", 0usize), cx));
    assert!(!shell.present(("conflict-keep-theirs", 0usize), cx));
    assert!(!shell.present(("conflict-body-mine", 0usize), cx));
    assert!(!shell.present(("conflict-result", 0usize), cx));
    assert!(!shell.present("conflict-save", cx));
    assert!(!shell.present("conflict-retry", cx));
    assert!(shell.present("conflict-open-folder", cx));
    assert!(shell.present("conflict-check-again", cx));
}
