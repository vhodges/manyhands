//! D2's actions by keyboard alone: the Drafts page by `secondary-shift-d`,
//! Tab to each row action, Inspect and Escape, Export through the
//! (simulated) save dialog, Discard with Escape, Enter (Cancel, the
//! default) and Confirm reached by Tab; Restore, then Compare and each
//! close-tab choice in the item's tab; quitting with a draft's write
//! held, Quit now focused; and quitting when a draft cannot be saved,
//! Cancel by Enter and Quit anyway by Tab.
//!
//! Defects these found are pinned here: a focus given back to a Kit
//! button's wrapper (Escape from Inspect or Discard's dialog) must draw
//! its own ring and be pressed by Enter again; Enter on a dialog's focused
//! button must press that button, not the dialog's default; a dialog's
//! first Tab stop is its safe choice; and the tab's Export… and Discard…
//! scroll into view below a long comparison.
//!
//! As in D1's walk, each step asserts which element holds focus and that
//! it is on screen: in a region, that region alone draws its ring; in a
//! dialog, no region holds focus.

use gpui_kit::{ElementId, TestAppContext, test::TestWindowExt};
use manyhands::{
    drafts::{DraftId, Held},
    repository::ItemDtoKind,
};

use super::Walk;
use crate::desktop::{
    model::tabs::TabKey,
    services::ReadKind,
    test_support::{
        drafts::{held, store_draft},
        fixtures::{Fixture, ticket_path, ticket_source},
    },
    view::{
        drafts::{
            ACTIONS,
            actions::{DISCARD_CONFIRM, EXPORTED},
            row_id,
        },
        harness::Shell,
        quit::{QUIT_NOW, QuitPhase, SAVING_DRAFTS},
        workspace::{REGIONS, Region},
    },
};

const ONE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FP0";
const TWO: &str = "01ARZ3NDEKTSV4RRFFQ69G5FP1";

/// Whether the store still keeps `id`, by its file.
fn kept(walk: &Walk, id: DraftId) -> bool {
    walk.shell
        .fixture
        .data_dir()
        .join("drafts")
        .join(format!("{id}.json"))
        .exists()
}

fn drafts_shown(walk: &Walk, cx: &mut TestAppContext) -> bool {
    walk.shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.drafts_shown(cx))
}

fn attached(walk: &Walk, cx: &mut TestAppContext) -> Option<DraftId> {
    let tab = TabKey::for_target(&walk.ticket(ONE));
    walk.shell.workspace.read_with(cx, |workspace, cx| {
        workspace.services().drafts().read(cx).attached(&tab)
    })
}

impl Walk {
    /// The element `id`, in a dialog, holds focus and is on screen; no
    /// region holds it: the dialog traps it.
    fn in_dialog(&self, id: impl Into<ElementId>, cx: &mut TestAppContext) {
        let id = id.into();
        self.shell.frame(cx, |window, _| {
            for region in REGIONS {
                assert_ne!(
                    window.find(region.id()).focused(),
                    Some(true),
                    "the dialog holds focus, not {region:?}"
                );
            }
            let element = window.find(id.clone());
            assert_eq!(element.focused(), Some(true), "{id:?} holds focus");
            assert!(element.visible(), "{id:?} is on screen");
        });
    }

    /// A dialog `dialog` is open and holds focus: no region does.
    fn dialog_open(&self, dialog: &'static str, cx: &mut TestAppContext) {
        assert!(self.shell.present(dialog, cx), "{dialog} is open");
        self.shell.frame(cx, |window, _| {
            for region in REGIONS {
                assert_ne!(
                    window.find(region.id()).focused(),
                    Some(true),
                    "{dialog} holds focus, not {region:?}"
                );
            }
        });
    }

    /// The wrapper ring `ring` is drawn and on screen: the focus Escape
    /// gave back to a wrapper is visible.
    fn ringed(&self, ring: impl Into<ElementId>, cx: &mut TestAppContext) {
        let ring = ring.into();
        self.shell.frame(cx, |window, _| {
            let found = window.try_find(ring.clone());
            assert!(
                found.is_some_and(|ring| ring.visible()),
                "{ring:?} is drawn"
            );
        });
    }

    /// Tab, within an open dialog, until `id` holds focus.
    fn tab_in_dialog(&self, id: &'static str, cx: &mut TestAppContext) {
        self.tab_to("tab", id, cx);
        self.in_dialog(id, cx);
    }
}

/// A repository `a` whose tickets One and Two each have a stored draft;
/// One was retitled since, so its draft reads as changed.
fn two_drafts(cx: &mut TestAppContext) -> (Walk, DraftId, DraftId) {
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    // One's body is long: its comparison (the item's body and the
    // draft's base) pushes the tab's Export… and Discard… off screen.
    let long: String = (0..120).map(|n| format!("Line {n} of One.\n")).collect();
    let source = ticket_source(ONE, "One", "").replace("Body of One.\n", &long);
    fixture.commit_file(&a, &ticket_path(ONE), source.as_bytes());
    fixture.commit_ticket(&a, TWO, "Two", "");
    fixture.refresh(&a);
    let one = store_draft(&fixture, &a, ItemDtoKind::Ticket, ONE, "Mine");
    let two = store_draft(&fixture, &a, ItemDtoKind::Ticket, TWO, "Also mine");
    fixture.save_ticket(&a, ONE, "Theirs");
    (Walk::over(Shell::open(fixture, cx), a), one, two)
}

/// The Drafts page by keyboard alone: `secondary-shift-d` opens it on a
/// row; Tab reaches every row action; Inspect and Escape; Export… through
/// the save dialog; Discard… with Escape, then Enter (Cancel, the
/// default), then Confirm reached by Tab and pressed by Enter.
#[gpui_kit::test]
fn the_drafts_page_works_by_keyboard_alone(cx: &mut TestAppContext) {
    let (walk, one, two) = two_drafts(cx);
    walk.keys("secondary-shift-d", cx);
    assert!(drafts_shown(&walk, cx));
    walk.region(Region::Content, cx);
    let first = [one, two]
        .into_iter()
        .find(|id| walk.holds_focus(&row_id(*id, "row").into(), cx))
        .expect("a row holds focus");
    walk.on(row_id(first, "row"), Region::Content, cx);

    // Tab reaches each action of each row.
    for id in [one, two] {
        let offered: Vec<&str> = ACTIONS
            .iter()
            .map(|(part, _)| *part)
            .filter(|part| walk.shell.present(row_id(id, part), cx))
            .collect();
        assert_eq!(offered, ["restore", "inspect", "export", "discard"]);
        for part in offered {
            walk.tab_to("tab", row_id(id, part), cx);
            walk.on(row_id(id, part), Region::Content, cx);
        }
    }

    // Inspect, then Escape: the panel closes, focus back on Inspect.
    walk.tab_to("shift-tab", row_id(one, "inspect"), cx);
    walk.enter(cx);
    assert!(walk.shell.present("drafts-inspect", cx));
    walk.on("drafts-inspect", Region::Content, cx);
    assert!(walk.shell.present("comparison", cx));
    walk.keys("escape", cx);
    assert!(!walk.shell.present("drafts-inspect", cx));
    let inspect_ring = row_id(one, "inspect-focus-ring");
    walk.on(row_id(one, "inspect"), Region::Content, cx);
    walk.ringed(inspect_ring.clone(), cx);
    // The focus given back is pressed by Enter again.
    walk.enter(cx);
    walk.on("drafts-inspect", Region::Content, cx);
    walk.keys("escape", cx);
    walk.on(row_id(one, "inspect"), Region::Content, cx);
    walk.ringed(inspect_ring, cx);

    // Export…: the save dialog asks where; the file is written.
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("One.md");
    walk.tab_to("tab", row_id(one, "export"), cx);
    walk.on(row_id(one, "export"), Region::Content, cx);
    walk.enter(cx);
    assert!(cx.did_prompt_for_new_path());
    let chosen = path.clone();
    cx.simulate_new_path_selection(move |_| Some(chosen));
    walk.shell.run(cx);
    assert!(std::fs::read_to_string(&path).unwrap().ends_with("Mine"));
    assert_eq!(
        walk.shell.label(row_id(one, "notice"), cx).as_deref(),
        Some(EXPORTED)
    );
    walk.on(row_id(one, "export"), Region::Content, cx);

    // Discard…, Escape: nothing is discarded, focus back on Discard.
    let discard = row_id(two, "discard");
    walk.tab_to("tab", discard.clone(), cx);
    walk.enter(cx);
    walk.dialog_open("discard-draft-dialog", cx);
    walk.keys("escape", cx);
    assert!(!walk.shell.present("discard-draft-dialog", cx));
    assert!(kept(&walk, two));
    let discard_ring = row_id(two, "discard-focus-ring");
    walk.on(discard.clone(), Region::Content, cx);
    walk.ringed(discard_ring.clone(), cx);

    // Discard…, Enter: the default is Cancel; nothing is discarded.
    walk.enter(cx);
    walk.dialog_open("discard-draft-dialog", cx);
    walk.enter(cx);
    assert!(!walk.shell.present("discard-draft-dialog", cx));
    assert!(kept(&walk, two));
    walk.on(discard.clone(), Region::Content, cx);
    walk.ringed(discard_ring.clone(), cx);

    // Discard…, one Tab, Enter: the first Tab stop is Cancel, the safe
    // choice; nothing is discarded.
    walk.enter(cx);
    walk.dialog_open("discard-draft-dialog", cx);
    walk.keys("tab", cx);
    walk.in_dialog("discard-draft-cancel-button", cx);
    walk.enter(cx);
    assert!(!walk.shell.present("discard-draft-dialog", cx));
    assert!(kept(&walk, two));
    walk.on(discard.clone(), Region::Content, cx);
    walk.ringed(discard_ring, cx);

    // Discard…, Tab to Confirm, Enter: the draft and its row go, and the
    // focus goes to the other row.
    walk.enter(cx);
    walk.dialog_open("discard-draft-dialog", cx);
    assert_eq!(
        walk.shell.label("discard-draft-confirm", cx).as_deref(),
        Some(DISCARD_CONFIRM)
    );
    walk.tab_in_dialog("discard-draft-confirm-button", cx);
    walk.enter(cx);
    assert!(!walk.shell.present("discard-draft-dialog", cx));
    assert!(!kept(&walk, two));
    assert!(kept(&walk, one));
    assert!(!walk.shell.present(row_id(two, "row"), cx));
    walk.on(row_id(one, "row"), Region::Content, cx);
}

/// Restore by keyboard opens the item's tab with the draft attached;
/// there, Compare opens the comparison; closing the tab asks, and Escape,
/// Cancel and Keep draft (Enter, the default) each do what they say;
/// restored again from the banner, Discard deletes the draft.
#[gpui_kit::test]
fn restore_compare_and_close_work_by_keyboard_alone(cx: &mut TestAppContext) {
    let (walk, one, _) = two_drafts(cx);
    walk.select_a_repository(cx);
    walk.keys("secondary-shift-d", cx);
    walk.tab_to("tab", row_id(one, "restore"), cx);
    walk.on(row_id(one, "restore"), Region::Content, cx);
    walk.enter(cx);
    assert!(!drafts_shown(&walk, cx), "the page is left");
    assert_eq!(walk.opened(cx), Some(walk.ticket(ONE)));
    walk.on(("tab", 0usize), Region::Content, cx);
    assert_eq!(attached(&walk, cx), Some(one));
    assert!(walk.shell.present("draft-section", cx));

    // Compare: the comparison opens under it.
    assert!(!walk.shell.present("comparison", cx));
    walk.tab_to("tab", "draft-compare-button", cx);
    walk.on("draft-compare-button", Region::Content, cx);
    walk.enter(cx);
    assert!(walk.shell.present("comparison", cx));
    walk.on("draft-compare-button", Region::Content, cx);

    // The tab's Export… is reached below the comparison, on screen.
    walk.tab_to("tab", "draft-export-button", cx);
    walk.on("draft-export-button", Region::Content, cx);

    // The tab's Discard…: Escape, focus back on Discard…, which Enter
    // presses again; Enter in the dialog is Cancel.
    walk.tab_to("tab", "draft-discard", cx);
    walk.on("draft-discard", Region::Content, cx);
    walk.enter(cx);
    walk.dialog_open("discard-draft-dialog", cx);
    walk.keys("escape", cx);
    assert!(!walk.shell.present("discard-draft-dialog", cx));
    walk.on("draft-discard", Region::Content, cx);
    walk.ringed("draft-discard-focus-ring", cx);
    walk.enter(cx);
    walk.dialog_open("discard-draft-dialog", cx);
    walk.enter(cx);
    assert!(!walk.shell.present("discard-draft-dialog", cx));
    assert_eq!(attached(&walk, cx), Some(one));
    walk.on("draft-discard", Region::Content, cx);
    walk.ringed("draft-discard-focus-ring", cx);

    // Close, Escape: the tab and its draft stay, focus back on the tab.
    walk.keys("secondary-3", cx);
    walk.on(("tab", 0usize), Region::Content, cx);
    walk.keys("secondary-w", cx);
    walk.dialog_open("close-draft-dialog", cx);
    walk.keys("escape", cx);
    assert!(!walk.shell.present("close-draft-dialog", cx));
    assert_eq!(walk.tab_count(cx), 1);
    assert_eq!(attached(&walk, cx), Some(one));
    walk.on(("tab", 0usize), Region::Content, cx);

    // Close, Tab to Cancel, Enter: the same.
    walk.keys("secondary-w", cx);
    walk.dialog_open("close-draft-dialog", cx);
    walk.tab_in_dialog("close-draft-cancel-button", cx);
    walk.enter(cx);
    assert!(!walk.shell.present("close-draft-dialog", cx));
    assert_eq!(walk.tab_count(cx), 1);
    assert_eq!(attached(&walk, cx), Some(one));
    walk.on(("tab", 0usize), Region::Content, cx);

    // Close, one Tab, Enter: the first Tab stop is Keep draft, the safe
    // choice (Enter alone is Keep draft too:
    // `tabs_tests::keep_draft_releases_the_claim_and_the_draft_is_listed_again`).
    // The tab closes and the draft is let go of, still stored.
    walk.keys("secondary-w", cx);
    walk.dialog_open("close-draft-dialog", cx);
    walk.keys("tab", cx);
    walk.in_dialog("close-draft-keep-button", cx);
    walk.enter(cx);
    walk.shell.run(cx);
    assert!(!walk.shell.present("close-draft-dialog", cx));
    assert_eq!(walk.tab_count(cx), 0);
    assert!(matches!(
        held(&walk.shell.fixture.draft_store(), one),
        Some(Held::No)
    ));
    walk.region(Region::Content, cx);

    // Opened again, from the ticket list: the banner's Restore by Tab and
    // Enter; then Close, Tab to Discard, Enter deletes the draft.
    walk.find(ONE, cx);
    assert_eq!(walk.opened(cx), Some(walk.ticket(ONE)));
    walk.keys("secondary-3", cx);
    walk.on(("tab", 0usize), Region::Content, cx);
    walk.tab_to("tab", "draft-restore-button", cx);
    walk.on("draft-restore-button", Region::Content, cx);
    walk.enter(cx);
    assert_eq!(attached(&walk, cx), Some(one));
    walk.keys("secondary-3", cx);
    walk.keys("secondary-w", cx);
    walk.dialog_open("close-draft-dialog", cx);
    walk.tab_in_dialog("close-draft-discard-button", cx);
    walk.enter(cx);
    walk.shell.run(cx);
    assert!(!walk.shell.present("close-draft-dialog", cx));
    assert_eq!(walk.tab_count(cx), 0);
    assert!(held(&walk.shell.fixture.draft_store(), one).is_none());
    walk.region(Region::Content, cx);
}

/// `secondary-q` with a draft's newest text unsaved and its write held:
/// "Saving drafts for recovery", with Quit now holding focus and drawing
/// its ring; Enter on it quits at once.
#[gpui_kit::test]
async fn quit_with_a_held_write_by_keyboard(cx: &mut TestAppContext) {
    let shell = Shell::open(Fixture::new(), cx);
    // No editor yet: the session's unsaved text is made as D3's editor
    // will make it, through the registry.
    shell.unsaved_draft(cx).await;
    let gate = shell.backend.hold_read(ReadKind::DraftWrite);
    let a = shell.fixture.data_dir().to_owned();
    let walk = Walk::over(shell, a);

    walk.keys("secondary-q", cx);
    let phase = |cx: &mut TestAppContext| {
        walk.shell
            .workspace
            .read_with(cx, |workspace, _| workspace.quit_phase())
    };
    assert_eq!(phase(cx), QuitPhase::SavingDrafts);
    assert_eq!(
        walk.shell.label("quit-finishing", cx).as_deref(),
        Some(SAVING_DRAFTS)
    );
    assert_eq!(walk.shell.label("quit-now", cx).as_deref(), Some(QUIT_NOW));
    walk.shell.frame(cx, |window, _| {
        let quit_now = window.find("quit-now");
        assert_eq!(quit_now.focused(), Some(true), "Quit now holds focus");
        assert!(quit_now.visible(), "Quit now is on screen");
        assert!(window.find("quit-now-focus-ring").visible());
    });
    walk.enter(cx);
    assert_eq!(phase(cx), QuitPhase::Quit);
    drop(gate);
}

/// `secondary-q` when a draft cannot be saved: the dialog holds focus;
/// Enter is Cancel, which focuses the draft's Retry in the operation area;
/// Quit anyway, reached by Tab and pressed by Enter, quits.
#[gpui_kit::test]
async fn a_draft_not_saved_on_quit_by_keyboard(cx: &mut TestAppContext) {
    use manyhands::drafts::{DraftErrorKind, FailureAction, FailurePoint};

    use crate::desktop::view::operations::draft_flush_id;

    let shell = Shell::open(Fixture::new(), cx);
    let id = shell.unsaved_draft(cx).await;
    // Clones share the failure point: the window's store fails too.
    let _failing = shell.fixture.draft_store().with_failure_point_for_testing(
        FailurePoint::CreateTemp,
        FailureAction::Fail(DraftErrorKind::NoSpace),
    );
    let a = shell.fixture.data_dir().to_owned();
    let walk = Walk::over(shell, a);
    let phase = |cx: &mut TestAppContext| {
        walk.shell
            .workspace
            .read_with(cx, |workspace, _| workspace.quit_phase())
    };

    walk.keys("secondary-q", cx);
    walk.dialog_open("drafts-not-saved-dialog", cx);
    walk.enter(cx);
    assert!(!walk.shell.present("drafts-not-saved-dialog", cx));
    assert_eq!(phase(cx), QuitPhase::Running);
    walk.on(draft_flush_id(id, "retry"), Region::Operations, cx);

    // One Tab, Enter: the first Tab stop is Cancel; it does not quit.
    walk.keys("secondary-q", cx);
    walk.dialog_open("drafts-not-saved-dialog", cx);
    walk.keys("tab", cx);
    walk.in_dialog("drafts-not-saved-cancel-button", cx);
    walk.enter(cx);
    assert!(!walk.shell.present("drafts-not-saved-dialog", cx));
    assert_eq!(phase(cx), QuitPhase::Running);
    walk.on(draft_flush_id(id, "retry"), Region::Operations, cx);

    walk.keys("secondary-q", cx);
    walk.dialog_open("drafts-not-saved-dialog", cx);
    walk.tab_in_dialog("drafts-not-saved-quit-button", cx);
    walk.enter(cx);
    assert_eq!(phase(cx), QuitPhase::Quit);
}
