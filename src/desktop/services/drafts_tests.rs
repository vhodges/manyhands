//! The draft store in the desktop: the launch listing, a stale listing
//! dropped, a store that cannot open, and the stored draft of an item.
//!
//! Draft types derive no `Debug`, so the listing is read through `match`.

use std::sync::Arc;

use gpui_kit::TestAppContext;
use manyhands::{
    drafts::{DraftFields, DraftId, DraftStore, DraftTarget, Entry, ItemKind, Listing},
    repository::ItemDtoKind,
};

use crate::desktop::{
    model::list_state::ListState,
    services::{ReadKind, Services},
    test_support::{delaying::DelayingBackend, fixtures::Fixture},
    view::harness::key,
};

const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG0";

fn ticket_target(repository: &str) -> DraftTarget {
    DraftTarget {
        repository: repository.to_owned(),
        item_kind: ItemKind::Ticket,
        item_id: TICKET.parse().unwrap(),
        comment: None,
    }
}

fn ticket_fields() -> DraftFields {
    DraftFields::Ticket {
        title: "Draft title".to_owned(),
        ticket_type: "task".to_owned(),
        status: "open".to_owned(),
        project: None,
        team: None,
        deps: Vec::new(),
        parent: None,
        body: "Draft body".to_owned(),
    }
}

/// Stores a draft of `target` and lets go of its claim, as a process that
/// ended would.
fn store_draft(store: &DraftStore, target: DraftTarget) -> DraftId {
    match store.create(target, None, ticket_fields()) {
        Ok(claim) => claim.draft().id,
        Err(error) => panic!("the draft must be stored: {error}"),
    }
}

fn start(fixture: &Fixture, cx: &mut TestAppContext) -> (Arc<DelayingBackend>, Services) {
    let backend = fixture.delaying_backend();
    let services = cx.update(|cx| Services::start(backend.clone(), cx));
    (backend, services)
}

/// The IDs of the registry's listing; `None` while it is not ready.
fn listed(services: &Services, cx: &mut TestAppContext) -> Option<Vec<DraftId>> {
    services
        .drafts()
        .read_with(cx, |registry, _| match registry.listing() {
            ListState::Ready { rows, .. } => Some(rows.iter().map(|listing| listing.id).collect()),
            _ => None,
        })
}

#[gpui_kit::test]
fn launch_lists_the_store_once_and_the_registry_holds_the_listing(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let id = store_draft(&fixture.draft_store(), ticket_target("/a/repository"));

    let (backend, services) = start(&fixture, cx);
    cx.run_until_parked();

    assert_eq!(backend.reads_seen(ReadKind::Drafts), 1);
    assert_eq!(listed(&services, cx), Some(vec![id]));
    services.drafts().read_with(cx, |registry, _| {
        assert!(registry.unavailable().is_none());
        let rows = registry.listing().rows().unwrap();
        assert!(matches!(
            rows.as_slice(),
            [Listing { entry: Entry::Draft(draft), .. }] if draft.id == id
        ));
    });
}

/// A listing held at its gate, then a newer one that lands first: the
/// held one, released after a draft was stored, never replaces it.
#[gpui_kit::test]
fn a_held_listing_never_lands_after_a_newer_one(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let (backend, services) = start(&fixture, cx);
    cx.run_until_parked();

    let release = backend.hold_read(ReadKind::Drafts);
    let first = services
        .drafts()
        .update(cx, |registry, cx| registry.reload_listing(&services, cx));
    services
        .drafts()
        .update(cx, |registry, cx| registry.reload_listing(&services, cx));
    cx.run_until_parked();
    assert_eq!(listed(&services, cx), Some(Vec::new()));

    store_draft(&fixture.draft_store(), ticket_target("/a/repository"));
    // The held read went with its task; a send to it may find it gone.
    let _ = release.send(());
    cx.run_until_parked();
    assert_eq!(listed(&services, cx), Some(Vec::new()));
    assert_eq!(backend.reads_seen(ReadKind::Drafts), 3);

    // Its answer, had its call finished, is dropped by its stamp.
    let late = fixture.draft_store().list();
    let applied = services
        .drafts()
        .update(cx, |registry, _| registry.apply_listing(first, late));
    assert!(!applied);
    assert_eq!(listed(&services, cx), Some(Vec::new()));
}

#[gpui_kit::test]
fn stored_for_finds_an_unclaimed_ticket_draft_by_repository_kind_and_id(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.create_ticket(&root, TICKET, "Item title", "open");
    let repo = key(&root);
    let id = store_draft(&fixture.draft_store(), ticket_target(&repo));
    let (_backend, services) = start(&fixture, cx);
    cx.run_until_parked();

    services.drafts().read_with(cx, |registry, _| {
        assert_eq!(
            registry.stored_for(&repo, ItemDtoKind::Ticket, TICKET),
            Some(id)
        );
        assert_eq!(
            registry.stored_for(&repo, ItemDtoKind::Document, TICKET),
            None
        );
        assert_eq!(
            registry.stored_for(&repo, ItemDtoKind::Ticket, "01ARZ3NDEKTSV4RRFFQ69G5FG1"),
            None
        );
        assert_eq!(
            registry.stored_for(&"/another".to_owned(), ItemDtoKind::Ticket, TICKET),
            None
        );
    });

    // Claimed by this process, the draft is no longer one to offer.
    let claim = fixture.draft_store().claim(id);
    assert!(claim.is_ok());
    services
        .drafts()
        .update(cx, |registry, cx| registry.reload_listing(&services, cx));
    cx.run_until_parked();
    services.drafts().read_with(cx, |registry, _| {
        assert_eq!(
            registry.stored_for(&repo, ItemDtoKind::Ticket, TICKET),
            None
        );
    });
    drop(claim);
}

/// The listing is not loaded yet: nothing is stored as far as a tab can
/// tell.
#[gpui_kit::test]
fn stored_for_answers_none_before_the_listing_lands(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    store_draft(&fixture.draft_store(), ticket_target("/a/repository"));
    let backend = fixture.delaying_backend();
    let _release = backend.hold_read(ReadKind::Drafts);
    let services = cx.update(|cx| Services::start(backend.clone(), cx));
    cx.run_until_parked();

    services.drafts().read_with(cx, |registry, _| {
        assert!(matches!(registry.listing(), ListState::Loading));
        assert_eq!(
            registry.stored_for(&"/a/repository".to_owned(), ItemDtoKind::Ticket, TICKET),
            None
        );
    });
}

/// `drafts/` is a link: the store cannot be protected. The window still
/// opens; the operation area keeps an entry with the fixed reason and no
/// Dismiss, and no draft is offered.
#[cfg(unix)]
#[gpui_kit::test]
fn a_store_that_cannot_be_protected_still_opens_the_window(cx: &mut TestAppContext) {
    use crate::desktop::{model::operations::DRAFTS_UNAVAILABLE, view::harness::Shell};

    let fixture = Fixture::new();
    let elsewhere = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(elsewhere.path(), fixture.data_dir().join("drafts")).unwrap();
    let shell = Shell::open(fixture, cx);

    let expected = format!("{DRAFTS_UNAVAILABLE}: The drafts folder could not be protected.");
    assert_eq!(shell.label("drafts-unavailable", cx), Some(expected));
    assert!(!shell.present("drafts-unavailable-dismiss", cx));
    assert!(shell.requests(cx, |_| true).is_empty());
    assert_eq!(shell.backend.reads_seen(ReadKind::Drafts), 0);

    let drafts = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().drafts().clone());
    drafts.read_with(cx, |registry, _| {
        assert!(registry.unavailable().is_some());
        assert_eq!(
            registry.stored_for(&"/a/repository".to_owned(), ItemDtoKind::Ticket, TICKET),
            None
        );
    });
}
