//! The Drafts page's row actions over real repositories and a real store:
//! Restore leaves the page through the host and attaches the draft to the
//! item's tab; Discard asks first, naming the draft, and removes only it,
//! Escape removing nothing and giving the focus back to the row's
//! Discard; an unreadable row's Discard removes its file; a Restore met
//! by a passing claim in another process may be tried again; no dialog
//! opens while quitting saves drafts; and each row button, clicked, does
//! what it says.
//!
//! Draft types derive no `Debug`; tests read them through `match` and
//! compare text.

use gpui_kit::{TestAppContext, test::TestWindowExt};
use manyhands::{
    drafts::{DraftId, Held},
    repository::ItemDtoKind,
};

use super::{
    actions::{DISCARD_CONFIRM, OPEN_ELSEWHERE_RETRY, discard_message},
    row_id,
    tests::{ONE, TWO, actions, drafts_shown, open, registry, ticket, with_ticket},
};
use crate::desktop::{
    model::tabs::TabKey,
    test_support::{
        drafts::{held, hold_elsewhere, store_draft},
        fixtures::Fixture,
    },
    view::harness::Shell,
};

/// Whether the store still keeps `id`, by its file.
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

/// Restore on a row claims the draft, leaves the page through the host and
/// shows the item's tab with the draft attached.
#[gpui_kit::test]
fn restore_from_the_page_shows_the_items_tab_with_the_draft_attached(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let a = with_ticket(&fixture, "a", ONE, "One");
    let id = store_draft(&fixture, &a, ItemDtoKind::Ticket, ONE, "Mine");
    let shell = Shell::open(fixture, cx);
    shell.keys("secondary-shift-d", cx);

    shell.click(row_id(id, "restore"), cx);
    assert!(!drafts_shown(&shell, cx), "the page is left");
    let opened = shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.opened(cx));
    assert_eq!(opened, Some(ticket(&a, ONE)));
    let tab = TabKey::for_target(&ticket(&a, ONE));
    let attached = registry(&shell, cx).read_with(cx, |registry, _| registry.attached(&tab));
    assert_eq!(attached, Some(id));
    assert!(shell.present("draft-section", cx));
    assert!(matches!(
        held(&shell.fixture.draft_store(), id),
        Some(Held::Here)
    ));
}

/// Discard asks first, naming the draft by its own title; Confirm removes
/// that draft only.
#[gpui_kit::test]
fn discard_names_the_draft_and_confirm_removes_only_it(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let a = with_ticket(&fixture, "a", ONE, "One");
    fixture.commit_ticket(&a, TWO, "Two", "");
    fixture.refresh(&a);
    let one = store_draft(&fixture, &a, ItemDtoKind::Ticket, ONE, "Mine");
    let two = store_draft(&fixture, &a, ItemDtoKind::Ticket, TWO, "Also mine");
    let shell = Shell::open(fixture, cx);
    shell.keys("secondary-shift-d", cx);

    shell.click(row_id(one, "discard"), cx);
    assert_eq!(
        shell.label("discard-draft-message", cx),
        Some(discard_message(Some("One")))
    );
    assert!(kept(&shell, one), "nothing is removed before Confirm");
    shell.click("discard-draft-confirm", cx);
    assert!(!shell.present("discard-draft-dialog", cx));
    assert!(!kept(&shell, one));
    assert!(kept(&shell, two));
    assert!(!shell.present(row_id(one, "row"), cx));
    assert!(shell.present(row_id(two, "row"), cx));
}

/// Escape closes the dialog with nothing removed, and the focus goes back
/// to the row's Discard.
#[gpui_kit::test]
fn escape_from_discard_removes_nothing_and_focuses_discard(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let a = with_ticket(&fixture, "a", ONE, "One");
    let id = store_draft(&fixture, &a, ItemDtoKind::Ticket, ONE, "Mine");
    let shell = Shell::open(fixture, cx);
    shell.keys("secondary-shift-d", cx);

    shell.click(row_id(id, "discard"), cx);
    assert!(shell.present("discard-draft-dialog", cx));
    assert_eq!(
        shell.label("discard-draft-confirm", cx).as_deref(),
        Some(DISCARD_CONFIRM)
    );
    shell.keys("escape", cx);
    assert!(!shell.present("discard-draft-dialog", cx));
    assert!(kept(&shell, id));
    assert!(shell.present(row_id(id, "row"), cx));
    assert!(focused(&shell, row_id(id, "discard"), cx));
}

/// An unreadable row's Discard removes its file.
#[gpui_kit::test]
fn an_unreadable_rows_discard_removes_its_file(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    fixture.draft_store();
    let id = DraftId::generate();
    let path = fixture.data_dir().join("drafts").join(format!("{id}.json"));
    std::fs::write(&path, b"not a draft").unwrap();
    let shell = Shell::open(fixture, cx);
    shell.keys("secondary-shift-d", cx);
    assert_eq!(actions(&shell, id, cx), ["export-raw", "discard"]);

    shell.click(row_id(id, "discard"), cx);
    assert_eq!(
        shell.label("discard-draft-message", cx),
        Some(discard_message(None))
    );
    shell.click("discard-draft-confirm", cx);
    assert!(!path.exists());
    assert!(!shell.present(row_id(id, "row"), cx));
}

/// A draft discarded from the page while restored here ends its session:
/// its tab's section goes, and the draft is gone.
#[gpui_kit::test]
fn discard_of_a_draft_restored_here_ends_its_session(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let a = with_ticket(&fixture, "a", ONE, "One");
    let id = store_draft(&fixture, &a, ItemDtoKind::Ticket, ONE, "Mine");
    let shell = Shell::open(fixture, cx);
    open(&shell, ticket(&a, ONE), cx);
    shell.click("draft-restore-button", cx);
    shell.keys("secondary-shift-d", cx);

    shell.click(row_id(id, "discard"), cx);
    shell.click("discard-draft-confirm", cx);
    assert!(!kept(&shell, id));
    let session = registry(&shell, cx).read_with(cx, |registry, _| registry.session(id).is_some());
    assert!(!session);
    open(&shell, ticket(&a, ONE), cx);
    assert!(!shell.present("draft-section", cx));
    assert!(!shell.present("draft-banner", cx));
}

/// Restore met by a claim another process holds just then says it may be
/// tried again, and still offers Restore; once let go, Restore succeeds.
#[gpui_kit::test]
fn restore_met_by_a_passing_claim_elsewhere_may_be_tried_again(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let a = with_ticket(&fixture, "a", ONE, "One");
    let id = store_draft(&fixture, &a, ItemDtoKind::Ticket, ONE, "Mine");
    let shell = Shell::open(fixture, cx);
    // Listed unclaimed at launch; claimed elsewhere only afterwards.
    let holder = hold_elsewhere(&shell.fixture, id);
    shell.keys("secondary-shift-d", cx);

    shell.click(row_id(id, "restore"), cx);
    assert!(drafts_shown(&shell, cx));
    assert_eq!(
        shell.label(row_id(id, "notice"), cx).as_deref(),
        Some(OPEN_ELSEWHERE_RETRY)
    );
    assert!(shell.present(row_id(id, "restore"), cx));
    drop(holder);
    shell.click(row_id(id, "restore"), cx);
    assert!(!drafts_shown(&shell, cx));
    let tab = TabKey::for_target(&ticket(&a, ONE));
    let attached = registry(&shell, cx).read_with(cx, |registry, _| registry.attached(&tab));
    assert_eq!(attached, Some(id));
}

/// Each row button, clicked, does what it says: Inspect opens the panel,
/// Export… asks where, Export raw… asks where, Discard… asks first and
/// Restore restores.
#[gpui_kit::test]
fn each_row_button_does_what_it_says(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let a = with_ticket(&fixture, "a", ONE, "One");
    let id = store_draft(&fixture, &a, ItemDtoKind::Ticket, ONE, "Mine");
    fixture.draft_store();
    let broken = DraftId::generate();
    let path = fixture
        .data_dir()
        .join("drafts")
        .join(format!("{broken}.json"));
    std::fs::write(&path, b"not a draft").unwrap();
    let shell = Shell::open(fixture, cx);
    shell.keys("secondary-shift-d", cx);
    assert_eq!(
        actions(&shell, id, cx),
        ["restore", "inspect", "export", "discard"]
    );

    shell.click(row_id(id, "inspect"), cx);
    assert!(shell.present("drafts-inspect", cx));
    assert_eq!(
        shell.label("drafts-inspect-body", cx).as_deref(),
        Some("Mine")
    );
    shell.click("drafts-inspect-close", cx);
    assert!(!shell.present("drafts-inspect", cx));

    for export in [row_id(id, "export"), row_id(broken, "export-raw")] {
        shell.click(export, cx);
        assert!(cx.did_prompt_for_new_path());
        cx.simulate_new_path_selection(|_| None);
        shell.run(cx);
    }

    shell.click(row_id(id, "discard"), cx);
    assert!(shell.present("discard-draft-dialog", cx));
    shell.keys("escape", cx);
    assert!(kept(&shell, id));

    shell.click(row_id(id, "restore"), cx);
    assert!(!drafts_shown(&shell, cx));
    assert!(shell.present("draft-section", cx));
}

/// While quitting saves drafts, with Quit now in view, neither Discard's
/// dialog nor Export's save dialog opens in front of it.
#[gpui_kit::test]
fn no_dialog_opens_while_quitting_saves_drafts(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let a = with_ticket(&fixture, "a", ONE, "One");
    let id = store_draft(&fixture, &a, ItemDtoKind::Ticket, ONE, "Mine");
    let shell = Shell::open(fixture, cx);
    shell.keys("secondary-shift-d", cx);
    shell
        .operations(cx)
        .update(cx, |operations, _| operations.set_saving_drafts(true));

    shell.click(row_id(id, "discard"), cx);
    assert!(!shell.present("discard-draft-dialog", cx));
    shell.click(row_id(id, "export"), cx);
    assert!(!cx.did_prompt_for_new_path());
    assert!(kept(&shell, id));
}
