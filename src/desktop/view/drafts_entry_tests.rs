//! The ways into the Drafts page and what the rest of the window says of
//! drafts: the launch's "Recovered drafts" entry with Review and Dismiss,
//! a launch that restores nothing, the tab banner's Review in Drafts, and
//! an operation entry that names a draft's item, never the draft.
//!
//! Draft types derive no `Debug`; tests read them through `match` and
//! compare text.

use gpui_kit::{ElementId, TestAppContext, test::TestWindowExt};
use manyhands::{
    drafts::{DraftError, DraftErrorKind, Held},
    repository::{AuthoringKind, ItemDtoKind},
};

use super::{
    row_id,
    tests::{ONE, TWO, drafts_shown, open, registry, ticket, ticket_fields, with_ticket},
};
use crate::desktop::{
    model::{
        activity::{
            copy::{REVIEW, flush_title, item_name, recovered_title},
            rows::{ActivityAction, Kind, RowKey},
        },
        drafts::IDLE,
        list_view::RETRY,
        operations::recovered_drafts,
        tabs::TabKey,
    },
    test_support::{
        drafts::{held, store_draft},
        fixtures::Fixture,
    },
    view::{
        activity_support::{act, offered, refresh, row, rows},
        harness::Shell,
        operations::DISMISS,
    },
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
    let entry = RowKey::Recovered;
    refresh(&shell, cx);
    let recovered = row(&shell, &entry, cx).expect("the launch's entry is listed");
    assert_eq!(recovered.row.title, recovered_title(2));
    assert_eq!(
        offered(&shell, &entry, REVIEW, cx).map(|offer| (offer.action, offer.enabled)),
        Some((ActivityAction::ReviewDrafts, true))
    );
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

    act(&shell, &entry, REVIEW, cx);
    assert!(drafts_shown(&shell, cx));
    assert!(shell.present(row_id(first, "row"), cx));

    act(&shell, &entry, DISMISS, cx);
    refresh(&shell, cx);
    assert!(row(&shell, &entry, cx).is_none());
    drafts.update(cx, |registry, cx| {
        registry.reload_listing(cx);
    });
    shell.run(cx);
    refresh(&shell, cx);
    assert!(row(&shell, &entry, cx).is_none(), "for the session");
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

/// The drafts folder made read-only for as long as it lives; its mode is
/// put back on drop, so a failed assertion leaves the fixture removable.
#[cfg(unix)]
struct ReadOnly(std::path::PathBuf);

#[cfg(unix)]
impl ReadOnly {
    fn new(folder: std::path::PathBuf) -> Self {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o500)).unwrap();
        Self(folder)
    }
}

#[cfg(unix)]
impl Drop for ReadOnly {
    fn drop(&mut self) {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o700));
    }
}

/// An operation entry names a draft's item by its title as read when the
/// draft began, or by its ID when that read failed, never by the draft's
/// own title.
#[cfg(unix)]
#[gpui_kit::test]
async fn an_operation_entry_names_the_item_never_the_draft(cx: &mut TestAppContext) {
    use manyhands::{
        drafts::{DraftBase, DraftTarget, ItemKind},
        repository::{LeaseKind, RepositoryService},
    };

    use crate::desktop::test_support::fixtures::item_id;

    const SENTINEL: &str = "SENTINEL-d2c4";
    let fixture = Fixture::new();
    let a = with_ticket(&fixture, "a", ONE, "One");
    fixture.commit_ticket(&a, TWO, "Two", "");
    fixture.refresh(&a);
    store_draft(&fixture, &a, ItemDtoKind::Ticket, ONE, "Mine");
    let shell = Shell::open(fixture, cx);
    open(&shell, ticket(&a, ONE), cx);
    shell.click("draft-restore-button", cx);
    let drafts = registry(&shell, cx);
    let tab = TabKey::for_target(&ticket(&a, ONE));
    let restored = drafts
        .read_with(cx, |registry, _| registry.attached(&tab))
        .expect("the draft is restored");

    // A draft of Two begun when its read failed: the registry's cache is
    // held, so the read answers busy and the draft has no base.
    let base = {
        let _lease = RepositoryService::hold_lease_for_testing(
            &a,
            shell.fixture.data_dir(),
            LeaseKind::CacheWrite,
        )
        .unwrap();
        let service = shell.fixture.service();
        match service.resolve_repository(&a) {
            Ok(repo) => match service.show_item(&repo, &item_id(TWO)) {
                Ok(item) => DraftBase::of(&item),
                Err(_) => None,
            },
            Err(_) => None,
        }
    };
    assert!(base.is_none(), "the read must fail");
    let target = DraftTarget {
        repository: a.to_str().unwrap().to_owned(),
        item_kind: ItemKind::Ticket,
        item_id: item_id(TWO),
        comment: None,
    };
    let begun = drafts.update(cx, |registry, cx| {
        registry.begin(target, base, ticket_fields("Two", "Mine"), cx)
    });
    shell.run(cx);
    let unread = match begun.await {
        Ok(id) => id,
        Err(kind) => panic!("the draft must begin: {kind:?}"),
    };
    for id in [restored, unread] {
        drafts.update(cx, |registry, cx| {
            registry.edit(id, ticket_fields(SENTINEL, SENTINEL), cx)
        });
    }

    let _read_only = ReadOnly::new(shell.fixture.data_dir().join("drafts"));
    cx.executor().advance_clock(IDLE);
    shell.run(cx);
    // The title as read when the draft began, or the item's short code when
    // that read failed: the entry names the item, never the draft.
    let entries = [
        (
            restored,
            item_name(Some("One"), AuthoringKind::Ticket, &item_id(ONE)),
        ),
        (
            unread,
            item_name(None, AuthoringKind::Ticket, &item_id(TWO)),
        ),
    ];
    refresh(&shell, cx);
    for (id, name) in entries {
        let entry = row(&shell, &RowKey::Flush(id), cx).expect("the failing session's entry");
        assert_eq!(entry.row.kind, Kind::Failure);
        assert_eq!(entry.row.title, flush_title(&name));
        assert_eq!(
            entry.row.description,
            Some(
                DraftError {
                    kind: DraftErrorKind::Access
                }
                .to_string()
            )
        );
        assert!(offered(&shell, &RowKey::Flush(id), RETRY, cx).is_some_and(|offer| offer.enabled));
    }
    for entry in rows(&shell, cx) {
        let texts = [
            Some(entry.row.title.as_str()),
            entry.row.description.as_deref(),
        ]
        .into_iter()
        .flatten()
        .chain(entry.row.detail.iter().map(String::as_str));
        for text in texts {
            assert!(!text.contains(SENTINEL), "the draft's text is in {text:?}");
        }
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
            .any(|label| label.starts_with("The recovery copy of"))
    );
    assert!(!labels.iter().any(|label| label.contains(SENTINEL)));
}

/// A Review in Drafts that arrives while a dialog is open waits: the page
/// opens, on that row, once the dialog has closed.
#[gpui_kit::test]
fn review_in_drafts_waits_for_an_open_dialog(cx: &mut TestAppContext) {
    use gpui_kit::component::WindowExt;

    let fixture = Fixture::new();
    let a = with_ticket(&fixture, "a", ONE, "One");
    let id = store_draft(&fixture, &a, ItemDtoKind::Ticket, ONE, "Mine");
    let shell = Shell::open(fixture, cx);
    open(&shell, ticket(&a, ONE), cx);
    shell
        .visual(cx)
        .update(|window, cx| window.open_dialog(cx, |dialog, _, _| dialog.title("Probe")));
    shell.run(cx);

    // The banner cannot be pressed behind the dialog; its request is
    // made as the banner makes it.
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.review_in_drafts(id, cx));
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    assert!(!drafts_shown(&shell, cx), "not behind the dialog");

    shell
        .visual(cx)
        .update(|window, cx| window.close_dialog(cx));
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    assert!(drafts_shown(&shell, cx));
    assert!(shell.frame(cx, |window, _| {
        window.find(row_id(id, "row")).focused() == Some(true)
    }));
}
