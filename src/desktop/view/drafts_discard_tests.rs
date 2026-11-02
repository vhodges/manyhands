//! The Drafts page's Discard and its focus, over real repositories and a
//! real store:
//! - A discard the store refuses (a drafts folder that cannot be written)
//!   keeps the file and the row, and says so in a fixed sentence.
//! - Enter, the dialog's default, keeps the draft.
//! - A discarded row gives the focus to the next row, else the previous.
//! - An Inspect panel whose row goes leaves the focus on a live control.
//! - Neither dialog opens while the worker quits, or over another dialog.
//! - A discard landing behind another dialog leaves the focus there.
//!
//! Draft types derive no `Debug`; tests read them through `match` and
//! compare text.

use gpui_kit::{Entity, TestAppContext, test::TestWindowExt};
use manyhands::{drafts::DraftId, repository::ItemDtoKind};

use super::{
    DraftsPage, row_id,
    tests::{ONE, TWO, registry, with_ticket},
};
use crate::desktop::{
    test_support::{
        drafts::{Actor, second_actor, store_draft},
        fixtures::Fixture,
    },
    view::{harness::Shell, item::NOT_DISCARDED},
};

const THREE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD2";

fn kept(shell: &Shell, id: DraftId) -> bool {
    shell
        .fixture
        .data_dir()
        .join("drafts")
        .join(format!("{id}.json"))
        .exists()
}

fn focused(shell: &Shell, id: impl Into<gpui_kit::ElementId>, cx: &mut TestAppContext) -> bool {
    let id = id.into();
    shell.frame(cx, |window, _| window.find(id).focused() == Some(true))
}

fn page(shell: &Shell, cx: &mut TestAppContext) -> Entity<DraftsPage> {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.drafts_page().clone())
}

/// The page's rows, in the order shown.
fn shown(shell: &Shell, cx: &mut TestAppContext) -> Vec<DraftId> {
    page(shell, cx).read_with(cx, |page, _| {
        page.slots.iter().map(|slot| slot.row.id).collect()
    })
}

/// Repository `a` with tickets One, Two and Three, a draft of each, and
/// the page shown.
fn three_drafts(cx: &mut TestAppContext) -> Shell {
    let fixture = Fixture::new();
    let a = with_ticket(&fixture, "a", ONE, "One");
    fixture.commit_ticket(&a, TWO, "Two", "");
    fixture.commit_ticket(&a, THREE, "Three", "");
    fixture.refresh(&a);
    for id in [ONE, TWO, THREE] {
        store_draft(&fixture, &a, ItemDtoKind::Ticket, id, "Mine");
    }
    let shell = Shell::open(fixture, cx);
    shell.keys("secondary-shift-d", cx);
    shell
}

fn discard(shell: &Shell, id: DraftId, cx: &mut TestAppContext) {
    shell.click(row_id(id, "discard"), cx);
    shell.click("discard-draft-confirm", cx);
}

/// A discard the store refuses keeps the draft's file and its row, and
/// the row says why in the store's fixed sentence.
#[cfg(unix)]
#[gpui_kit::test]
fn a_failed_discard_keeps_the_draft_and_says_so(cx: &mut TestAppContext) {
    use crate::desktop::test_support::drafts::{FolderMode, superuser};

    if superuser() {
        return;
    }
    let fixture = Fixture::new();
    let a = with_ticket(&fixture, "a", ONE, "One");
    let id = store_draft(&fixture, &a, ItemDtoKind::Ticket, ONE, "Mine");
    fixture.draft_store();
    let broken = DraftId::generate();
    let drafts = fixture.data_dir().join("drafts");
    std::fs::write(drafts.join(format!("{broken}.json")), b"not a draft").unwrap();
    // The lock of a draft never claimed is made by its first claim.
    std::fs::write(drafts.join(format!("{broken}.lock")), b"").unwrap();
    let shell = Shell::open(fixture, cx);
    shell.keys("secondary-shift-d", cx);
    let _read_only = FolderMode::set(&drafts, 0o500);

    for id in [id, broken] {
        discard(&shell, id, cx);
        assert!(kept(&shell, id));
        assert!(shell.present(row_id(id, "row"), cx));
        assert_eq!(
            shell.label(row_id(id, "notice"), cx),
            Some(format!(
                "{NOT_DISCARDED}: Access to the drafts folder was denied."
            ))
        );
    }
}

/// Enter in the Discard dialog takes its default, Cancel: the draft is
/// kept, and the focus goes back to the row's Discard.
#[gpui_kit::test]
fn enter_in_the_discard_dialog_keeps_the_draft(cx: &mut TestAppContext) {
    let shell = three_drafts(cx);
    let id = shown(&shell, cx)[0];

    shell.click(row_id(id, "discard"), cx);
    assert!(shell.present("discard-draft-dialog", cx));
    shell.press("enter", cx);
    assert!(!shell.present("discard-draft-dialog", cx));
    assert!(kept(&shell, id));
    assert!(shell.present(row_id(id, "row"), cx));
    assert!(focused(&shell, row_id(id, "discard"), cx));
}

/// A discarded row gives the focus to the next row, and the last row to
/// the one before it.
#[gpui_kit::test]
fn a_discarded_row_focuses_its_neighbour(cx: &mut TestAppContext) {
    let shell = three_drafts(cx);
    let rows = shown(&shell, cx);

    discard(&shell, rows[2], cx);
    assert!(!kept(&shell, rows[2]));
    assert!(focused(&shell, row_id(rows[1], "row"), cx));

    discard(&shell, rows[0], cx);
    assert!(!kept(&shell, rows[0]));
    assert!(focused(&shell, row_id(rows[1], "row"), cx));
}

/// An Inspect panel whose row leaves the listing (discarded by another
/// process) takes the focus with it to the page's first control.
#[gpui_kit::test]
fn inspect_closed_with_its_row_leaves_the_focus_on_a_live_control(cx: &mut TestAppContext) {
    let shell = three_drafts(cx);
    let rows = shown(&shell, cx);
    shell.click(row_id(rows[0], "inspect"), cx);
    assert!(focused(&shell, "drafts-inspect", cx));

    second_actor(&shell.fixture, Actor::Discard(rows[0]));
    registry(&shell, cx).update(cx, |registry, cx| {
        registry.reload_listing(cx);
    });
    shell.run(cx);
    assert!(!shell.present("drafts-inspect", cx));
    let page = page(&shell, cx);
    let orphaned = shell.frame(cx, |window, cx| {
        page.read(cx).inspect_focus.is_focused(window)
    });
    assert!(!orphaned, "the closed panel keeps no focus");
    assert!(focused(&shell, row_id(rows[1], "row"), cx));
}

/// While the worker quits, neither Discard's dialog nor Export's save
/// dialog opens.
#[gpui_kit::test]
fn no_dialog_opens_while_the_worker_quits(cx: &mut TestAppContext) {
    let shell = three_drafts(cx);
    let id = shown(&shell, cx)[0];
    shell.workspace.read_with(cx, |workspace, _| {
        let _ = workspace.services().worker().quit();
    });

    shell.click(row_id(id, "discard"), cx);
    assert!(!shell.present("discard-draft-dialog", cx));
    shell.click(row_id(id, "export"), cx);
    assert!(!cx.did_prompt_for_new_path());
    assert!(kept(&shell, id));
}

/// Over an open dialog, Export asks nothing and Discard opens no second
/// dialog.
#[gpui_kit::test]
fn no_dialog_opens_over_another(cx: &mut TestAppContext) {
    let shell = three_drafts(cx);
    let rows = shown(&shell, cx);
    shell.click(row_id(rows[0], "discard"), cx);
    assert!(shell.present("discard-draft-dialog", cx));

    let page = page(&shell, cx);
    shell.visual(cx).update(|window, cx| {
        page.update(cx, |page, cx| {
            page.export(rows[1], window, cx);
            page.discard(rows[1], window, cx);
        })
    });
    shell.run(cx);
    assert!(!cx.did_prompt_for_new_path());
    // The dialog open is still the first row's: Escape closes it, and no
    // other is behind it.
    shell.keys("escape", cx);
    assert!(!shell.present("discard-draft-dialog", cx));
    assert!(focused(&shell, row_id(rows[0], "discard"), cx));
}

/// A discard that lands while another dialog is open leaves the focus in
/// that dialog: the page's refocus waits, and once the dialog closes it
/// goes to the discarded row's neighbour.
#[gpui_kit::test]
fn a_discard_landing_behind_a_dialog_leaves_the_focus_there(cx: &mut TestAppContext) {
    use gpui_kit::component::WindowExt;

    use crate::desktop::services::ReadKind;

    let shell = three_drafts(cx);
    let rows = shown(&shell, cx);
    shell.click(row_id(rows[1], "discard"), cx);
    let gate = shell.backend.hold_read(ReadKind::DraftWrite);
    shell.click("discard-draft-confirm", cx);
    assert!(!shell.present("discard-draft-dialog", cx));
    assert!(kept(&shell, rows[1]), "the discard is held");

    shell
        .visual(cx)
        .update(|window, cx| window.open_dialog(cx, |dialog, _, _| dialog.title("Probe")));
    shell.run(cx);
    let in_dialog = shell.frame(cx, |window, cx| window.focused(cx));
    assert!(in_dialog.is_some());

    drop(gate);
    shell.run(cx);
    assert!(!kept(&shell, rows[1]));
    assert!(!shown(&shell, cx).contains(&rows[1]), "the row went");
    assert_eq!(
        shell.frame(cx, |window, cx| window.focused(cx)),
        in_dialog,
        "the focus stays in the dialog"
    );
    assert!(!focused(&shell, row_id(rows[2], "row"), cx));

    shell
        .visual(cx)
        .update(|window, cx| window.close_dialog(cx));
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    assert!(focused(&shell, row_id(rows[2], "row"), cx));
}
