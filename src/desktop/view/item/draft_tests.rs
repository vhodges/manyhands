//! Drafts in an item's tab, over real repositories and a real store: the
//! banner for a stored draft, Restore and what it refuses, the restored
//! draft's section, its recovery and target states, and a refresh after
//! another actor saved the item.
//!
//! Draft types derive no `Debug`; tests read them through `match` and
//! compare text.

use std::path::{Path, PathBuf};

use gpui_kit::{Entity, TestAppContext};
use manyhands::{
    drafts::{DraftFields, DraftId, Held},
    repository::ItemDtoKind,
};

use super::{ANOTHER_OPEN, DRAFT_KEPT, NO_LONGER_KEPT, NOT_YET_SAVED, RESTORE, SAVED_FOR_RECOVERY};
use crate::desktop::{
    model::{drafts::IDLE, navigation::OpenTarget, tabs::TabKey},
    services::{ReadKind, drafts::DraftRegistry},
    test_support::{
        drafts::{Actor, SOMEONE_ELSES_TITLE, held, second_actor, store_draft, stored_body},
        fixtures::{Fixture, item_id},
        snapshot::RepositorySnapshot,
    },
    view::{
        draft_comparison::{CHANGED, CURRENT, ITEM_NOW, YOUR_DRAFT},
        harness::{Shell, key},
    },
};

const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FK0";
const DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FK1";

/// A repository `a` holding the ticket One, indexed.
fn with_ticket(fixture: &Fixture) -> PathBuf {
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "One", "");
    fixture.refresh(&root);
    root
}

fn ticket(root: &Path) -> OpenTarget {
    OpenTarget::Item {
        repo: key(root),
        kind: ItemDtoKind::Ticket,
        id: item_id(TICKET),
    }
}

fn open(shell: &Shell, target: OpenTarget, cx: &mut TestAppContext) {
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
}

fn registry(shell: &Shell, cx: &mut TestAppContext) -> Entity<DraftRegistry> {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().drafts().clone())
}

/// The session attached to `target`'s tab.
fn attached(shell: &Shell, target: &OpenTarget, cx: &mut TestAppContext) -> Option<DraftId> {
    let tab = TabKey::for_target(target);
    registry(shell, cx).read_with(cx, |registry, _| registry.attached(&tab))
}

fn restore(shell: &Shell, cx: &mut TestAppContext) {
    shell.click("draft-restore-button", cx);
    shell.run(cx);
}

fn label(shell: &Shell, id: &'static str, cx: &mut TestAppContext) -> Option<String> {
    shell.label(id, cx)
}

/// A stored draft shows the banner with Restore; opening the tab restores
/// nothing, and the store still lists the draft unclaimed.
#[gpui_kit::test]
fn a_stored_draft_shows_the_banner_and_opening_the_tab_restores_nothing(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = with_ticket(&fixture);
    let id = store_draft(&fixture, &root, ItemDtoKind::Ticket, TICKET, "Draft body");
    let shell = Shell::open(fixture, cx);

    open(&shell, ticket(&root), cx);
    assert_eq!(
        label(&shell, "draft-banner", cx).as_deref(),
        Some(DRAFT_KEPT)
    );
    assert_eq!(label(&shell, "draft-restore", cx).as_deref(), Some(RESTORE));
    assert!(!shell.present("draft-section", cx));
    assert!(attached(&shell, &ticket(&root), cx).is_none());
    assert!(matches!(
        held(&shell.fixture.draft_store(), id),
        Some(Held::No)
    ));
}

/// A draft this process holds is not offered again.
#[gpui_kit::test]
fn a_claimed_draft_shows_no_banner(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = with_ticket(&fixture);
    let id = store_draft(&fixture, &root, ItemDtoKind::Ticket, TICKET, "Draft body");
    let _claim = fixture.draft_store().claim(id).ok().unwrap();
    let shell = Shell::open(fixture, cx);

    open(&shell, ticket(&root), cx);
    assert!(shell.present("item-title", cx), "the item is read");
    assert!(!shell.present("draft-banner", cx));
    assert!(!shell.present("draft-restore", cx));
}

/// Restore attaches the draft: its section shows its fields, that it is
/// saved for recovery and that the item is unchanged, with Continue
/// editing and Save (E-8). The repository's files and refs are as they
/// were.
#[gpui_kit::test]
fn restore_attaches_the_draft_and_writes_nothing_to_the_repository(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = with_ticket(&fixture);
    let id = store_draft(&fixture, &root, ItemDtoKind::Ticket, TICKET, "Draft body");
    let shell = Shell::open(fixture, cx);
    open(&shell, ticket(&root), cx);
    let before = RepositorySnapshot::take(&root);

    restore(&shell, cx);
    assert_eq!(attached(&shell, &ticket(&root), cx), Some(id));
    assert!(shell.present("draft-section", cx));
    assert_eq!(label(&shell, "draft-title", cx).as_deref(), Some("One"));
    assert_eq!(
        label(&shell, "draft-body", cx).as_deref(),
        Some("Draft body")
    );
    let fields = shell.rows("draft-field", cx);
    assert!(fields.contains(&"Status: open".to_owned()), "{fields:?}");
    let recovery = label(&shell, "draft-recovery", cx).unwrap();
    assert!(recovery.starts_with(SAVED_FOR_RECOVERY), "{recovery}");
    assert_eq!(label(&shell, "draft-target", cx).as_deref(), Some(CURRENT));
    assert!(!shell.present("draft-compare", cx), "nothing to compare");
    assert_eq!(
        label(&shell, "draft-continue", cx).as_deref(),
        Some("Continue editing")
    );
    assert_eq!(label(&shell, "draft-save", cx).as_deref(), Some("Save"));
    assert!(!shell.present("draft-banner", cx), "the draft is restored");

    // Neither is pressed: the text is as it was, the store still holds the
    // draft, claimed here, and nothing is written.
    assert_eq!(
        label(&shell, "draft-body", cx).as_deref(),
        Some("Draft body")
    );
    assert!(matches!(
        held(&shell.fixture.draft_store(), id),
        Some(Held::Here)
    ));
    assert_eq!(RepositorySnapshot::take(&root), before);
}

/// The recovery state follows the session: a write held at its gate
/// reads as not yet saved until it lands.
#[gpui_kit::test]
fn a_held_write_reads_not_yet_saved_until_it_lands(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = with_ticket(&fixture);
    let id = store_draft(&fixture, &root, ItemDtoKind::Ticket, TICKET, "Draft body");
    let shell = Shell::open(fixture, cx);
    open(&shell, ticket(&root), cx);
    restore(&shell, cx);

    let gate = shell.backend.hold_read(ReadKind::DraftWrite);
    let drafts = registry(&shell, cx);
    let edited = drafts.read_with(cx, |registry, _| {
        let mut fields = registry.session(id).unwrap().current().clone();
        if let DraftFields::Ticket { body, .. } = &mut fields {
            *body = "Newer body".to_owned();
        }
        fields
    });
    drafts.update(cx, |registry, cx| registry.edit(id, edited, cx));
    cx.executor().advance_clock(IDLE);
    shell.run(cx);
    assert_eq!(
        label(&shell, "draft-recovery", cx).as_deref(),
        Some(NOT_YET_SAVED)
    );
    assert_eq!(
        stored_body(&shell.fixture.draft_store(), id).as_deref(),
        Some("Draft body")
    );

    drop(gate);
    shell.run(cx);
    let recovery = label(&shell, "draft-recovery", cx).unwrap();
    assert!(recovery.starts_with(SAVED_FOR_RECOVERY), "{recovery}");
    assert_eq!(
        stored_body(&shell.fixture.draft_store(), id).as_deref(),
        Some("Newer body")
    );
    assert_eq!(
        label(&shell, "draft-body", cx).as_deref(),
        Some("Newer body")
    );
}

/// One session of an item at a time: with one draft restored, Restore of
/// a second draft of the same item is refused and claims nothing.
#[gpui_kit::test]
fn a_second_draft_of_the_same_item_is_refused(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = with_ticket(&fixture);
    let first = store_draft(&fixture, &root, ItemDtoKind::Ticket, TICKET, "First");
    let second = store_draft(&fixture, &root, ItemDtoKind::Ticket, TICKET, "Second");
    let shell = Shell::open(fixture, cx);
    open(&shell, ticket(&root), cx);

    restore(&shell, cx);
    let restored = attached(&shell, &ticket(&root), cx).unwrap();
    let other = if restored == first { second } else { first };
    assert_eq!(
        label(&shell, "draft-banner", cx).as_deref(),
        Some(DRAFT_KEPT),
        "the other draft is still offered"
    );

    restore(&shell, cx);
    assert_eq!(
        label(&shell, "draft-refused", cx).as_deref(),
        Some(ANOTHER_OPEN)
    );
    assert_eq!(attached(&shell, &ticket(&root), cx), Some(restored));
    let store = shell.fixture.draft_store();
    assert!(matches!(held(&store, restored), Some(Held::Here)));
    assert!(matches!(held(&store, other), Some(Held::No)));
}

/// A draft discarded by another process since the listing reads as no
/// longer kept, and the store is listed again: the banner goes.
#[gpui_kit::test]
fn a_draft_discarded_elsewhere_reads_no_longer_kept_and_the_listing_reloads(
    cx: &mut TestAppContext,
) {
    let fixture = Fixture::new();
    let root = with_ticket(&fixture);
    let id = store_draft(&fixture, &root, ItemDtoKind::Ticket, TICKET, "Draft body");
    let shell = Shell::open(fixture, cx);
    open(&shell, ticket(&root), cx);
    assert!(shell.present("draft-banner", cx));

    second_actor(&shell.fixture, Actor::Discard(id));
    let listings = shell.backend.reads_seen(ReadKind::Drafts);
    restore(&shell, cx);
    assert_eq!(
        label(&shell, "draft-refused", cx).as_deref(),
        Some(NO_LONGER_KEPT)
    );
    assert!(shell.backend.reads_seen(ReadKind::Drafts) > listings + 1);
    assert!(!shell.present("draft-banner", cx));
    assert!(!shell.present("draft-section", cx));
    assert!(attached(&shell, &ticket(&root), cx).is_none());
}

/// Another actor saves the item; after Refresh the section's text is as
/// it was and the item reads as changed. Compare names the field the
/// item changed as the item's, and the one the draft changed as the
/// draft's.
#[gpui_kit::test]
async fn a_refresh_after_another_save_keeps_the_text_and_reads_changed(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.create_document(&root, DOCUMENT, "docs/a.md", "Item title");
    fixture.refresh(&root);
    let id = store_draft(
        &fixture,
        &root,
        ItemDtoKind::Document,
        DOCUMENT,
        "Draft body",
    );
    let shell = Shell::open(fixture, cx);
    shell.select(&root, cx);
    let document = OpenTarget::Item {
        repo: key(&root),
        kind: ItemDtoKind::Document,
        id: item_id(DOCUMENT),
    };
    open(&shell, document.clone(), cx);
    restore(&shell, cx);
    assert_eq!(label(&shell, "draft-target", cx).as_deref(), Some(CURRENT));

    second_actor(
        &shell.fixture,
        Actor::SaveDocument {
            root: &root,
            id: DOCUMENT,
        },
    );
    shell.keys("secondary-r", cx);
    let refresh = shell.requests(cx, |_| true)[0];
    shell.until_ended(refresh, cx).await;
    shell.run(cx);

    assert_eq!(
        shell.label("item-title", cx).as_deref(),
        Some(SOMEONE_ELSES_TITLE)
    );
    assert_eq!(
        label(&shell, "draft-title", cx).as_deref(),
        Some("Item title")
    );
    assert_eq!(
        label(&shell, "draft-body", cx).as_deref(),
        Some("Draft body")
    );
    assert_eq!(label(&shell, "draft-target", cx).as_deref(), Some(CHANGED));
    assert!(!shell.present(("comparison-field", 0usize), cx));

    shell.click("draft-compare-button", cx);
    let fields = shell.rows("comparison-field", cx);
    let changes = shell.rows("comparison-change", cx);
    assert_eq!(fields, ["Title", "Body"]);
    assert_eq!(changes, [ITEM_NOW, YOUR_DRAFT]);
    assert_eq!(
        stored_body(&shell.fixture.draft_store(), id).as_deref(),
        Some("Draft body")
    );
    assert_eq!(attached(&shell, &document, cx), Some(id));
}

/// The tab closes while its Restore's claim is held: when the claim lands
/// nothing shows the session, so it is let go of and the draft is offered
/// again, unclaimed.
#[gpui_kit::test]
fn a_restore_that_lands_after_its_tab_closed_lets_the_draft_go(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = with_ticket(&fixture);
    let id = store_draft(&fixture, &root, ItemDtoKind::Ticket, TICKET, "Draft body");
    let shell = Shell::open(fixture, cx);
    open(&shell, ticket(&root), cx);

    let claim = shell.backend.hold_read(ReadKind::Drafts);
    restore(&shell, cx);
    shell.click(("tab-close", 0usize), cx);
    assert!(!shell.present("item-view", cx), "the tab closed at once");
    drop(claim);
    shell.run(cx);

    let drafts = registry(&shell, cx);
    assert!(drafts.read_with(cx, |registry, _| registry.session(id).is_none()));
    assert!(attached(&shell, &ticket(&root), cx).is_none());
    assert!(matches!(
        held(&shell.fixture.draft_store(), id),
        Some(Held::No)
    ));
    open(&shell, ticket(&root), cx);
    assert!(
        shell.present("draft-banner", cx),
        "the draft is offered again"
    );
}
