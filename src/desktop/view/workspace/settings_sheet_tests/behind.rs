//! A landing applied while a view behind the open Settings draws (design A6
//! F16, "or any landing"; A3, "The sheet keeps focus"; fix round 1): Drafts'
//! Discard… that lands while Settings is open would focus the discarded
//! row's neighbour inside the Drafts page's own render, behind the sheet.
//!
//! Focus is read after GPUI's own draws only. Nothing between the landing
//! and the read calls `render_frame` (`Shell::frame`), which would give a
//! check that needs one more frame the frame it would otherwise never get;
//! the read takes the window's focus and the key contexts of the frame drawn
//! last, without drawing.

use gpui_kit::{TestAppContext, component::WindowExt};
use manyhands::repository::ItemDtoKind;

use super::{
    returns::settings_open,
    support::{MAIN, OTHER, drafts_shown, host},
};
use crate::desktop::{
    services::ReadKind,
    test_support::{drafts::store_draft, fixtures::Fixture},
    view::{drafts::row_id, harness::Shell},
};
use manyhands::drafts::DraftId;

/// Whether the draft `id`'s file is still in the store.
fn kept(shell: &Shell, id: DraftId) -> bool {
    shell
        .fixture
        .data_dir()
        .join("drafts")
        .join(format!("{id}.json"))
        .exists()
}

/// Two drafts on the Drafts page; Discard… of the first confirmed, its
/// write held; Settings opened, and a dialog over it. The write lands while
/// the dialog is open, so the page keeps its refocus for the first frame
/// after the dialog closes (`drafts/row_actions.rs`, `apply_refocus`). The
/// dialog closes in one update, whose flush draws that one frame, and
/// nothing is left to draw another.
pub(super) fn landing(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, MAIN, "Main", "");
    fixture.commit_ticket(&root, OTHER, "Other", "");
    fixture.refresh(&root);
    let main = store_draft(&fixture, &root, ItemDtoKind::Ticket, MAIN, "Mine");
    let other = store_draft(&fixture, &root, ItemDtoKind::Ticket, OTHER, "Mine");
    let shell = Shell::open(fixture, cx);
    shell.select(&root, cx);
    shell.keys("secondary-shift-d", cx);
    assert!(drafts_shown(&shell, cx));
    assert!(
        shell.present(row_id(other, "row"), cx),
        "both drafts are listed"
    );

    // Confirmed, the discard waits for its write; the page will focus the
    // neighbour once the row has gone.
    shell.click(row_id(main, "discard"), cx);
    let held = shell.backend.hold_read(ReadKind::DraftWrite);
    shell.click("discard-draft-confirm", cx);
    assert!(kept(&shell, main), "the discard is held");
    shell.keys("secondary-,", cx);
    assert!(settings_open(&shell, cx), "Settings opened over Drafts");
    let host = host(&shell, cx);
    let entry = host
        .read_with(cx, |host, _| host.first_focus())
        .expect("a page is shown");
    shell
        .visual(cx)
        .update(|window, cx| window.open_dialog(cx, |dialog, _, _| dialog.title("Probe")));
    shell.run(cx);
    drop(held);
    shell.run(cx);
    assert!(!kept(&shell, main), "the discard landed");

    // The dialog closes onto the entry; from here on, only GPUI's own draws.
    shell
        .visual(cx)
        .update(|window, cx| window.close_dialog(cx));
    shell.run(cx);
    let (on_entry, contexts, sheet) = shell.visual(cx).update(|window, cx| {
        let contexts: Vec<String> = window
            .context_stack()
            .iter()
            .map(|context| format!("{context:?}"))
            .collect();
        (
            entry.is_focused(window),
            contexts.join(" > "),
            window.has_active_sheet(cx),
        )
    });
    assert!(
        on_entry,
        "F16: focus is on the page's entry, not behind the sheet; it is in [{contexts}], \
         sheet {sheet}"
    );
}
