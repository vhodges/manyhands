//! The resolve conflict sheet's note, which follows where the conflict
//! stopped (decision 15), and a Save whose set cannot be kept (decision
//! 19). Shares `conflict_tests.rs`'s helpers.

use gpui_kit::TestAppContext;
use manyhands::repository::OperationId;

use super::{
    NOTE_CLOSE, NOTE_LOCAL, NOTE_MERGE, NOTE_SHARE, SheetSource,
    tests::{TICKET, local_shell, open_sheet, remote_shell, resolves, until_present},
};
use crate::desktop::services::{JobKind, resolutions::ResolutionStoreError};

/// Decision 15: the note under the title follows where the conflict
/// stopped.
#[gpui_kit::test]
async fn the_note_follows_the_source(cx: &mut TestAppContext) {
    let (shell, root) = remote_shell(cx);
    for (source, note) in [
        (SheetSource::Share, NOTE_SHARE),
        (SheetSource::Post, NOTE_SHARE),
        (SheetSource::Local, NOTE_LOCAL),
        (SheetSource::Close(None), NOTE_CLOSE),
        (SheetSource::Merge(None), NOTE_MERGE),
    ] {
        open_sheet(&shell, source, &root, OperationId::new(), cx);
        assert_eq!(shell.label("conflict-note", cx).as_deref(), Some(note));
        shell
            .visual(cx)
            .update(gpui_kit::component::WindowExt::close_dialog);
        shell.run(cx);
        assert!(!shell.present("conflict-sheet", cx));
    }
}

/// Decision 19: a set that cannot be kept is not sent. With the
/// resolutions folder unopenable (a file stands in its place), the sheet
/// says why as it opens (review M-4), never as a lost set (K-10), and Save
/// resolution sends nothing.
#[gpui_kit::test]
async fn a_set_that_cannot_be_kept_is_not_sent(cx: &mut TestAppContext) {
    let (shell, root, operation) = local_shell(cx, |fixture, root| {
        std::fs::write(fixture.data_dir().join("resolutions"), b"not a folder").unwrap();
        fixture.conflict_locally(root, TICKET, "Mine", "Theirs")
    });
    open_sheet(&shell, SheetSource::Local, &root, operation, cx);
    until_present(&shell, "conflict-save", cx).await;
    let opened = shell.label("conflict-error", cx);
    assert!(
        opened.is_some(),
        "the store's error shows as the sheet opens"
    );
    assert!(!shell.present("conflict-lost", cx));
    shell.click(("conflict-keep-mine", 0usize), cx);
    shell.click("conflict-save", cx);
    until_present(&shell, "conflict-error", cx).await;
    let words = shell.label("conflict-error", cx).unwrap();
    assert_eq!(Some(&words), opened.as_ref());
    assert!(
        [
            ResolutionStoreError::ProtectionUnavailable,
            ResolutionStoreError::Io,
            ResolutionStoreError::Access,
        ]
        .iter()
        .any(|error| error.to_string() == words),
        "{words}"
    );
    assert!(resolves(&shell, cx).is_empty(), "nothing is sent");
    assert!(
        !shell
            .backend
            .jobs_seen()
            .contains(&JobKind::LocalContextResolve)
    );
}
