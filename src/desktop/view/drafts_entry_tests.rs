//! The ways into the Drafts page and what the rest of the window says of
//! drafts: the launch's "Recovered drafts" entry with Review and Dismiss,
//! a launch that restores nothing, the tab banner's Review in Drafts, and
//! an operation entry that names a draft's item, never the draft.
//!
//! Draft types derive no `Debug`; tests read them through `match` and
//! compare text.

use gpui_kit::{ElementId, TestAppContext, test::TestWindowExt};
use manyhands::{
    drafts::{DraftErrorKind, Held},
    repository::ItemDtoKind,
};

use super::{
    row_id,
    tests::{ONE, TWO, drafts_shown, open, registry, ticket, ticket_fields, with_ticket},
};
use crate::desktop::{
    model::{
        drafts::IDLE,
        operations::{draft_flush_failed, recovered_drafts},
        tabs::TabKey,
    },
    test_support::{
        drafts::{held, store_draft},
        fixtures::Fixture,
    },
    view::{harness::Shell, operations::draft_flush_id},
};

/// The launch's listing found two drafts: the operation area says so,
/// Review opens the page, and Dismiss hides the entry for the session.
/// Nothing was restored: no tab opened and no draft is claimed.
#[gpui_kit::test]
fn the_launch_offers_its_recovered_drafts_and_restores_nothing(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let a = with_ticket(&fixture, "a", ONE, "One");
    fixture.commit_ticket(&a, TWO, "Two", "");
    fixture.refresh(&a);
    let first = store_draft(&fixture, &a, ItemDtoKind::Ticket, ONE, "Mine");
    let second = store_draft(&fixture, &a, ItemDtoKind::Ticket, TWO, "Mine too");
    let shell = Shell::open(fixture, cx);

    assert_eq!(recovered_drafts(2), "Recovered drafts: 2");
    assert_eq!(
        shell.label("recovered-drafts", cx),
        Some(recovered_drafts(2))
    );
    assert!(shell.present("recovered-drafts-review", cx));
    let tabs = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.tab_keys());
    assert!(tabs.is_empty());
    let drafts = registry(&shell, cx);
    for id in [first, second] {
        assert!(drafts.read_with(cx, |registry, _| registry.session(id).is_none()));
        assert!(matches!(
            held(&shell.fixture.draft_store(), id),
            Some(Held::No)
        ));
    }

    shell.click("recovered-drafts-review", cx);
    assert!(drafts_shown(&shell, cx));
    assert!(shell.present(row_id(first, "row"), cx));

    shell.click("recovered-drafts-dismiss", cx);
    assert!(!shell.present("recovered-drafts", cx));
    drafts.update(cx, |registry, cx| {
        registry.reload_listing(cx);
    });
    shell.run(cx);
    assert!(!shell.present("recovered-drafts", cx), "for the session");
}

/// The tab banner's Review in Drafts opens the page with that draft's
/// row focused, not the first row's.
#[gpui_kit::test]
fn review_in_drafts_opens_the_page_on_the_drafts_row(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let a = with_ticket(&fixture, "a", ONE, "One");
    fixture.commit_ticket(&a, TWO, "Two", "");
    fixture.refresh(&a);
    let id = store_draft(&fixture, &a, ItemDtoKind::Ticket, ONE, "Mine");
    // Saved later, so listed first.
    std::thread::sleep(std::time::Duration::from_millis(5));
    let other = store_draft(&fixture, &a, ItemDtoKind::Ticket, TWO, "Mine too");
    let shell = Shell::open(fixture, cx);
    open(&shell, ticket(&a, ONE), cx);

    shell.click("draft-review-button", cx);
    assert!(drafts_shown(&shell, cx));
    assert!(shell.present(row_id(other, "row"), cx));
    assert!(shell.frame(cx, |window, _| {
        window.find(row_id(id, "row")).focused() == Some(true)
    }));
}

/// An operation entry names a draft's item by its title as read, or by
/// its ID when no read gave one, never by the draft's own title.
#[cfg(unix)]
#[gpui_kit::test]
async fn an_operation_entry_names_the_item_never_the_draft(cx: &mut TestAppContext) {
    use std::os::unix::fs::PermissionsExt;

    const SENTINEL: &str = "SENTINEL-d2c4";
    let fixture = Fixture::new();
    let a = with_ticket(&fixture, "a", ONE, "One");
    store_draft(&fixture, &a, ItemDtoKind::Ticket, ONE, "Mine");
    let shell = Shell::open(fixture, cx);
    open(&shell, ticket(&a, ONE), cx);
    shell.click("draft-restore-button", cx);
    let drafts = registry(&shell, cx);
    let tab = TabKey::for_target(&ticket(&a, ONE));
    let restored = drafts
        .read_with(cx, |registry, _| registry.attached(&tab))
        .expect("the draft is restored");
    // Begun with no read of its item: named by its ID.
    let unread = shell.begin_draft(cx).await;
    for id in [restored, unread] {
        drafts.update(cx, |registry, cx| {
            registry.edit(id, ticket_fields(SENTINEL, SENTINEL), cx)
        });
    }

    let folder = shell.fixture.data_dir().join("drafts");
    std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o500)).unwrap();
    cx.executor().advance_clock(IDLE);
    shell.run(cx);
    let entries = [
        (restored, draft_flush_failed("One", DraftErrorKind::Access)),
        (
            unread,
            draft_flush_failed("01ARZ3NDEKTSV4RRFFQ69G5FG0", DraftErrorKind::Access),
        ),
    ];
    for (id, expected) in entries {
        assert_eq!(shell.label(draft_flush_id(id, "entry"), cx), Some(expected));
    }
    // The draft's own text shows only in its own section of its tab.
    let section = ElementId::from("operation-area");
    let labels: Vec<String> = shell.frame(cx, |window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .iter()
            .filter(|snapshot| snapshot.path().contains(&section))
            .filter_map(|snapshot| snapshot.label().map(str::to_owned))
            .collect()
    });
    assert!(
        labels
            .iter()
            .any(|label| label.starts_with("Recovery copy of"))
    );
    assert!(!labels.iter().any(|label| label.contains(SENTINEL)));
    std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o700)).unwrap();
}
