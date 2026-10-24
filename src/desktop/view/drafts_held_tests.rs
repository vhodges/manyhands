//! Drafts page rows of drafts someone holds: one open in another window
//! reads so and offers Inspect only (Export arrives with the page's
//! actions), never Restore or Discard; and one held here with no tab to
//! show offers no Show.

use gpui_kit::TestAppContext;
use manyhands::repository::ItemDtoKind;

use super::{
    OPEN_ELSEWHERE, RESTORED_HERE,
    tests::{ONE, actions, registry, state, with_ticket},
};
use crate::desktop::{
    test_support::{drafts::store_draft, fixtures::Fixture},
    view::harness::Shell,
};

/// The draft's lock is held through a handle of its own, as another
/// Manyhands process holds it: the store lists it as held elsewhere.
#[gpui_kit::test]
fn a_draft_open_in_another_window_offers_inspect_only(cx: &mut TestAppContext) {
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
    assert_eq!(actions(&shell, id, cx), ["inspect"]);
    FileExt::unlock(&lock).unwrap();
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
    assert_eq!(actions(&shell, id, cx), ["inspect", "discard"]);
}
