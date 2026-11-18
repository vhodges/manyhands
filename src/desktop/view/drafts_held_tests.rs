//! Drafts page rows of drafts someone holds: one open in another window
//! reads so and offers Inspect and Export only, never Restore or Discard,
//! whether its lock is held through a handle of this process or by
//! another Manyhands process; one held here with no tab to show offers
//! no Show; and an unreadable one held elsewhere offers no Discard….

use gpui_kit::TestAppContext;
use manyhands::repository::ItemDtoKind;

use super::{
    OPEN_ELSEWHERE, RESTORED_HERE,
    tests::{ONE, actions, registry, state, with_ticket},
};
use crate::desktop::{
    test_support::{
        drafts::{hold_elsewhere, store_draft},
        fixtures::Fixture,
    },
    view::harness::Shell,
};

/// The draft's lock is held through a handle of its own, as another
/// Manyhands process holds it: the store lists it as held elsewhere.
#[gpui_kit::test]
fn a_draft_open_in_another_window_offers_inspect_and_export_only(cx: &mut TestAppContext) {
    use fs4::fs_std::FileExt;

    let fixture = Fixture::new();
    let a = with_ticket(&fixture, "a", ONE, "One");
    let id = store_draft(&fixture, &a, ItemDtoKind::Ticket, ONE, "Mine");
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(fixture.data_dir().join("drafts").join(format!("{id}.lock")))
        .unwrap();
    assert!(FileExt::try_lock_exclusive(&lock).unwrap());
    let shell = Shell::open(fixture, cx);

    shell.keys("secondary-shift-d", cx);
    assert_eq!(state(&shell, id, cx).as_deref(), Some(OPEN_ELSEWHERE));
    assert_eq!(actions(&shell, id, cx), ["inspect", "export"]);
    FileExt::unlock(&lock).unwrap();
}

/// Another Manyhands process holds the draft's claim, as the library's
/// claims test does: the row reads so, offers Inspect and Export only,
/// and both work.
#[gpui_kit::test]
fn a_draft_claimed_by_another_process_offers_inspect_and_export_only(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let a = with_ticket(&fixture, "a", ONE, "One");
    let id = store_draft(&fixture, &a, ItemDtoKind::Ticket, ONE, "Mine");
    let holder = hold_elsewhere(&fixture, id);
    let shell = Shell::open(fixture, cx);

    shell.keys("secondary-shift-d", cx);
    assert_eq!(state(&shell, id, cx).as_deref(), Some(OPEN_ELSEWHERE));
    assert_eq!(actions(&shell, id, cx), ["inspect", "export"]);
    shell.click(super::row_id(id, "inspect"), cx);
    assert_eq!(
        shell.label("drafts-inspect-body", cx).as_deref(),
        Some("Mine")
    );
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("One.md");
    shell.click(super::row_id(id, "export"), cx);
    let chosen = path.clone();
    cx.simulate_new_path_selection(move |_| Some(chosen));
    shell.run(cx);
    assert!(std::fs::read_to_string(&path).unwrap().ends_with("Mine"));
    drop(holder);
}

/// A session held here with no tab (one the editor began) reads as
/// restored here but offers no Show: there is no tab to show.
#[gpui_kit::test]
async fn a_draft_held_here_without_a_tab_offers_no_show(cx: &mut TestAppContext) {
    let shell = Shell::open(Fixture::new(), cx);
    let id = shell.begin_draft(cx).await;
    registry(&shell, cx).update(cx, |registry, cx| {
        registry.reload_listing(cx);
    });
    shell.run(cx);

    shell.keys("secondary-shift-d", cx);
    assert_eq!(state(&shell, id, cx).as_deref(), Some(RESTORED_HERE));
    assert_eq!(actions(&shell, id, cx), ["inspect", "export", "discard"]);
}

/// An unreadable draft whose lock another Manyhands process holds offers
/// Export raw… only: no Discard…, as for a readable draft open elsewhere.
/// (`hold_elsewhere` claims through the store, which refuses an unreadable
/// draft, so the lock is held through a handle of its own, as the first
/// test here does.)
#[gpui_kit::test]
fn an_unreadable_draft_held_elsewhere_offers_no_discard(cx: &mut TestAppContext) {
    use fs4::fs_std::FileExt;

    let fixture = Fixture::new();
    fixture.draft_store();
    let id = manyhands::drafts::DraftId::generate();
    let drafts = fixture.data_dir().join("drafts");
    std::fs::write(drafts.join(format!("{id}.json")), b"not a draft").unwrap();
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(drafts.join(format!("{id}.lock")))
        .unwrap();
    assert!(FileExt::try_lock_exclusive(&lock).unwrap());
    let shell = Shell::open(fixture, cx);

    shell.keys("secondary-shift-d", cx);
    assert!(shell.present(super::row_id(id, "row"), cx));
    assert_eq!(actions(&shell, id, cx), ["export-raw"]);
    FileExt::unlock(&lock).unwrap();
}
