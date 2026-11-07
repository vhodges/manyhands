//! The draft store in the desktop: the launch listing, a stale listing
//! dropped, a store that cannot open, the stored draft of an item, and
//! sessions flushed on their schedule, at once, and after a failure.
//!
//! Draft types derive no `Debug`, so the listing is read through `match`.

use std::{sync::Arc, time::Duration};

use gpui_kit::{Entity, TestAppContext};
use manyhands::{
    drafts::{
        BaseContext, ContextKind, Draft, DraftBase, DraftErrorKind, DraftFields, DraftId,
        DraftStore, DraftTarget, Entry, ItemKind, Listing,
    },
    repository::ItemDtoKind,
};

use super::DraftRegistry;
use crate::desktop::{
    model::{
        drafts::{IDLE, RecoveryState},
        list_state::ListState,
    },
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
        .update(cx, |registry, cx| registry.reload_listing(cx));
    services
        .drafts()
        .update(cx, |registry, cx| registry.reload_listing(cx));
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
        // Not an item ID: lowercase is not canonical, and text is not one.
        assert_eq!(
            registry.stored_for(&repo, ItemDtoKind::Ticket, &TICKET.to_lowercase()),
            None
        );
        assert_eq!(
            registry.stored_for(&repo, ItemDtoKind::Ticket, "not an id"),
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
        .update(cx, |registry, cx| registry.reload_listing(cx));
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
    use gpui_kit::ElementId;

    use crate::desktop::{
        model::operations::DRAFTS_UNAVAILABLE,
        view::{harness::Shell, operations::DISMISS},
    };

    let fixture = Fixture::new();
    let elsewhere = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(elsewhere.path(), fixture.data_dir().join("drafts")).unwrap();
    let shell = Shell::open(fixture, cx);

    let expected = format!("{DRAFTS_UNAVAILABLE}: The drafts folder could not be protected.");
    assert_eq!(shell.label("drafts-unavailable", cx), Some(expected));
    // Every control the "This session" section draws is labeled: the
    // entry is drawn there, and nothing there reads Dismiss.
    let section = ElementId::from("session-section");
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
            .any(|label| label.starts_with(DRAFTS_UNAVAILABLE))
    );
    assert!(!labels.iter().any(|label| label == DISMISS));
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

/// The ticket's fields with `body`.
fn ticket_body(body: &str) -> DraftFields {
    match ticket_fields() {
        DraftFields::Ticket {
            title,
            ticket_type,
            status,
            project,
            team,
            deps,
            parent,
            ..
        } => DraftFields::Ticket {
            title,
            ticket_type,
            status,
            project,
            team,
            deps,
            parent,
            body: body.to_owned(),
        },
        _ => unreachable!(),
    }
}

/// The item as read when the draft began, titled `title`.
fn ticket_base(title: &str) -> DraftBase {
    DraftBase {
        observation: Some("observation".to_owned()),
        path: format!(".manyhands/tickets/{TICKET}.md"),
        context: BaseContext {
            kind: ContextKind::Primary,
            branch: Some("main".to_owned()),
            worktree: "/a/repository".to_owned(),
            head_oid: None,
        },
        source: None,
        fields: match ticket_fields() {
            DraftFields::Ticket {
                ticket_type,
                status,
                project,
                team,
                deps,
                parent,
                ..
            } => DraftFields::Ticket {
                title: title.to_owned(),
                ticket_type,
                status,
                project,
                team,
                deps,
                parent,
                body: "Item body".to_owned(),
            },
            _ => unreachable!(),
        },
    }
}

/// Begins a draft through the registry and waits for it.
async fn begin(
    drafts: &Entity<DraftRegistry>,
    base: Option<DraftBase>,
    cx: &mut TestAppContext,
) -> DraftId {
    let begun = drafts.update(cx, |registry, cx| {
        registry.begin(ticket_target("/a/repository"), base, ticket_fields(), cx)
    });
    cx.run_until_parked();
    match begun.await {
        Ok(id) => id,
        Err(kind) => panic!("the draft must begin: {kind:?}"),
    }
}

/// The stored revision of `id` and its body.
fn stored(store: &DraftStore, id: DraftId) -> (u64, String) {
    let listing = store.list().unwrap();
    match listing.iter().find(|listing| listing.id == id) {
        Some(Listing {
            entry:
                Entry::Draft(Draft {
                    revision,
                    current: DraftFields::Ticket { body, .. },
                    ..
                }),
            ..
        }) => (*revision, body.clone()),
        _ => panic!("the draft must be stored"),
    }
}

fn state(drafts: &Entity<DraftRegistry>, id: DraftId, cx: &TestAppContext) -> RecoveryState {
    drafts
        .read_with(cx, |registry, _| registry.recovery_state(id))
        .expect("the session must be live")
}

/// A held write: the session reads `Pending` and the store keeps revision
/// 1; once released, it reads `Saved` and the store holds the newest
/// revision, written once.
#[gpui_kit::test]
async fn a_held_write_is_pending_until_it_lands_with_the_newest_revision(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let (backend, services) = start(&fixture, cx);
    cx.run_until_parked();
    let drafts = services.drafts().clone();
    let id = begin(&drafts, None, cx).await;
    assert!(matches!(state(&drafts, id, cx), RecoveryState::Saved(_)));

    let release = backend.hold_read(ReadKind::DraftWrite);
    drafts.update(cx, |registry, cx| {
        registry.edit(id, ticket_body("second"), cx);
        registry.edit(id, ticket_body("newest"), cx);
    });
    cx.executor().advance_clock(IDLE - Duration::from_millis(1));
    cx.run_until_parked();
    assert_eq!(backend.reads_seen(ReadKind::DraftWrite), 0);

    cx.executor().advance_clock(Duration::from_millis(1));
    cx.run_until_parked();
    assert_eq!(backend.reads_seen(ReadKind::DraftWrite), 1);
    assert_eq!(state(&drafts, id, cx), RecoveryState::Pending);
    assert_eq!(
        stored(&fixture.draft_store(), id),
        (1, "Draft body".to_owned())
    );

    release.send(()).unwrap();
    cx.run_until_parked();
    assert!(matches!(state(&drafts, id, cx), RecoveryState::Saved(_)));
    assert_eq!(stored(&fixture.draft_store(), id), (3, "newest".to_owned()));
    assert_eq!(backend.reads_seen(ReadKind::DraftWrite), 1);
}

/// `flush_now` writes at once, with no clock advanced, and waits for a
/// write in flight before writing what came after it.
#[gpui_kit::test]
async fn flush_now_writes_the_newest_revision_after_a_write_in_flight(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let (backend, services) = start(&fixture, cx);
    cx.run_until_parked();
    let drafts = services.drafts().clone();
    let id = begin(&drafts, None, cx).await;

    let release = backend.hold_read(ReadKind::DraftWrite);
    drafts.update(cx, |registry, cx| {
        registry.edit(id, ticket_body("held"), cx)
    });
    cx.executor().advance_clock(IDLE);
    cx.run_until_parked();
    drafts.update(cx, |registry, cx| {
        registry.edit(id, ticket_body("after"), cx)
    });
    let flushed = drafts.update(cx, |registry, cx| registry.flush_now(id, cx));
    cx.run_until_parked();
    assert_eq!(state(&drafts, id, cx), RecoveryState::Pending);

    release.send(()).unwrap();
    cx.run_until_parked();
    assert_eq!(flushed.await, Ok(()));
    assert_eq!(stored(&fixture.draft_store(), id), (3, "after".to_owned()));
    assert_eq!(backend.reads_seen(ReadKind::DraftWrite), 2);

    // Nothing new: a flush writes nothing.
    let again = drafts.update(cx, |registry, cx| registry.flush_now(id, cx));
    cx.run_until_parked();
    assert_eq!(again.await, Ok(()));
    assert_eq!(backend.reads_seen(ReadKind::DraftWrite), 2);
}

/// `flush_all` writes every session now and answers the ones that failed.
#[cfg(unix)]
#[gpui_kit::test]
async fn flush_all_answers_each_session_that_failed(cx: &mut TestAppContext) {
    use std::os::unix::fs::PermissionsExt;

    let fixture = Fixture::new();
    let (_backend, services) = start(&fixture, cx);
    cx.run_until_parked();
    let drafts = services.drafts().clone();
    let first = begin(&drafts, None, cx).await;
    let second = begin(&drafts, None, cx).await;
    drafts.update(cx, |registry, cx| {
        registry.edit(first, ticket_body("one"), cx);
        registry.edit(second, ticket_body("two"), cx);
    });

    let folder = fixture.data_dir().join("drafts");
    std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o500)).unwrap();
    let all = drafts.update(cx, |registry, cx| registry.flush_all(cx));
    cx.run_until_parked();
    let mut failed = all.await;
    std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o700)).unwrap();
    failed.sort_by_key(|(id, _)| *id);
    let mut expected = vec![
        (first, DraftErrorKind::Access),
        (second, DraftErrorKind::Access),
    ];
    expected.sort_by_key(|(id, _)| *id);
    assert_eq!(failed, expected);

    let all = drafts.update(cx, |registry, cx| registry.flush_all(cx));
    cx.run_until_parked();
    assert_eq!(all.await, Vec::new());
    assert_eq!(stored(&fixture.draft_store(), first).1, "one");
    assert_eq!(stored(&fixture.draft_store(), second).1, "two");
}

/// A store that did not open begins no session.
#[cfg(unix)]
#[gpui_kit::test]
async fn an_unavailable_store_begins_no_session(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let elsewhere = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(elsewhere.path(), fixture.data_dir().join("drafts")).unwrap();
    let (_backend, services) = start(&fixture, cx);
    cx.run_until_parked();
    let begun = services.drafts().update(cx, |registry, cx| {
        registry.begin(ticket_target("/a/repository"), None, ticket_fields(), cx)
    });
    assert_eq!(
        begun.await.err(),
        Some(DraftErrorKind::ProtectionUnavailable)
    );
}

/// `drafts/` made read-only: the operation area names the item by its
/// title as read, gives the store's fixed reason and offers Retry; with
/// the mode restored, Retry writes and the entry goes. Draft text, its
/// body and its own title, is in no entry.
#[cfg(unix)]
#[gpui_kit::test]
async fn a_failed_flush_shows_an_entry_with_retry_and_no_draft_text(cx: &mut TestAppContext) {
    use std::os::unix::fs::PermissionsExt;

    use crate::desktop::{
        model::operations::draft_flush_failed, view::harness::Shell,
        view::operations::draft_flush_id,
    };

    const SENTINEL: &str = "SENTINEL-9b1e";

    let shell = Shell::open(Fixture::new(), cx);
    let drafts = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().drafts().clone());
    let id = begin(&drafts, Some(ticket_base("Item title")), cx).await;
    let text = match ticket_body(&format!("{SENTINEL} body")) {
        DraftFields::Ticket {
            ticket_type,
            status,
            project,
            team,
            deps,
            parent,
            body,
            ..
        } => DraftFields::Ticket {
            title: format!("{SENTINEL} title"),
            ticket_type,
            status,
            project,
            team,
            deps,
            parent,
            body,
        },
        _ => unreachable!(),
    };
    drafts.update(cx, |registry, cx| registry.edit(id, text, cx));

    let folder = shell.fixture.data_dir().join("drafts");
    std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o500)).unwrap();
    cx.executor().advance_clock(IDLE);
    shell.run(cx);

    let expected = draft_flush_failed("Item title", DraftErrorKind::Access);
    assert_eq!(
        expected,
        "Recovery copy of Item title not saved: Access to the drafts folder was denied."
    );
    assert_eq!(shell.label(draft_flush_id(id, "entry"), cx), Some(expected));
    assert!(shell.present(draft_flush_id(id, "retry"), cx));
    let labels: Vec<String> = shell.frame(cx, |window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .iter()
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
    shell.click(draft_flush_id(id, "retry"), cx);
    assert!(!shell.present(draft_flush_id(id, "entry"), cx));
    assert!(matches!(state(&drafts, id, cx), RecoveryState::Saved(_)));
    assert_eq!(
        stored(&shell.fixture.draft_store(), id),
        (2, format!("{SENTINEL} body"))
    );
}

/// The folder sync fails after the rename: the new revision is stored, but
/// the write counts as failed and `flushed` stays. A retry rewrites that
/// same revision: it gets past the store's check of a lower revision (an
/// `Io` refusal) to the sync, and stores nothing newer. The library keeps
/// a failure point for the store's life, so this test cannot see the
/// retry succeed; the model's
/// `a_failed_write_can_be_taken_again_without_an_edit` pins the landing.
#[gpui_kit::test]
async fn a_write_stored_before_its_folder_sync_failed_is_failed_and_retried_at_its_revision(
    cx: &mut TestAppContext,
) {
    use manyhands::drafts::{FailureAction, FailurePoint};

    let fixture = Fixture::new();
    let (backend, services) = start(&fixture, cx);
    cx.run_until_parked();
    let drafts = services.drafts().clone();
    let id = begin(&drafts, None, cx).await;
    // Clones share the failure point: the desktop's store fails too.
    let _ = fixture.draft_store().with_failure_point_for_testing(
        FailurePoint::SyncFolder,
        FailureAction::Fail(DraftErrorKind::NoSpace),
    );

    drafts.update(cx, |registry, cx| {
        registry.edit(id, ticket_body("replaced"), cx)
    });
    cx.executor().advance_clock(IDLE);
    cx.run_until_parked();
    assert_eq!(
        state(&drafts, id, cx),
        RecoveryState::Failed(DraftErrorKind::NoSpace)
    );
    assert_eq!(
        stored(&fixture.draft_store(), id),
        (2, "replaced".to_owned())
    );

    let retried = drafts.update(cx, |registry, cx| registry.flush_now(id, cx));
    cx.run_until_parked();
    assert_eq!(retried.await, Err(DraftErrorKind::NoSpace));
    assert_eq!(backend.reads_seen(ReadKind::DraftWrite), 2);
    assert_eq!(
        stored(&fixture.draft_store(), id),
        (2, "replaced".to_owned())
    );
}

/// An edit tells the views only when the session's recovery state
/// changes: the first edit after a write, not every keystroke.
#[gpui_kit::test]
async fn edits_notify_only_when_the_recovery_state_changes(cx: &mut TestAppContext) {
    use std::{cell::Cell, rc::Rc};

    let fixture = Fixture::new();
    let (_backend, services) = start(&fixture, cx);
    cx.run_until_parked();
    let drafts = services.drafts().clone();
    let id = begin(&drafts, None, cx).await;
    let notified = Rc::new(Cell::new(0));
    let _observe = cx.update(|cx| {
        let notified = notified.clone();
        cx.observe(&drafts, move |_, _| notified.set(notified.get() + 1))
    });

    for body in ["a", "b", "c"] {
        drafts.update(cx, |registry, cx| registry.edit(id, ticket_body(body), cx));
    }
    cx.run_until_parked();
    // Saved to Pending, once.
    assert_eq!(notified.get(), 1);

    cx.executor().advance_clock(IDLE);
    cx.run_until_parked();
    // Pending to Saved, when the write lands.
    assert_eq!(notified.get(), 2);
    assert!(matches!(state(&drafts, id, cx), RecoveryState::Saved(_)));
}

/// "Recovered drafts" counts what the launch listing held and nothing
/// stored later: a draft kept this session is never counted.
#[gpui_kit::test]
fn recovered_counts_only_the_launch_listings_drafts(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    store_draft(&fixture.draft_store(), ticket_target("/a/repository"));
    let (_backend, services) = start(&fixture, cx);
    cx.run_until_parked();
    let recovered = |cx: &mut TestAppContext| {
        services
            .drafts()
            .read_with(cx, |registry, _| registry.recovered())
    };
    assert_eq!(recovered(cx), Some(1));

    store_draft(&fixture.draft_store(), ticket_target("/b/repository"));
    services
        .drafts()
        .update(cx, |registry, cx| registry.reload_listing(cx));
    cx.run_until_parked();
    assert_eq!(listed(&services, cx).map(|ids| ids.len()), Some(2));
    assert_eq!(recovered(cx), Some(1));
}

/// A launch listing that failed recovers nothing for the session, even
/// once a later listing finds drafts.
#[gpui_kit::test]
fn a_failed_launch_listing_recovers_nothing_for_the_session(cx: &mut TestAppContext) {
    use manyhands::drafts::DraftError;

    use crate::desktop::model::stamp::{Stamp, ViewScope};

    let fixture = Fixture::new();
    store_draft(&fixture.draft_store(), ticket_target("/a/repository"));
    let backend = fixture.delaying_backend();
    let gate = backend.hold_read(ReadKind::Drafts);
    let services = cx.update(|cx| Services::start(backend.clone(), cx));
    cx.run_until_parked();

    // The launch listing's own stamp, answered with a failure.
    let launch = Stamp {
        scope: ViewScope::Drafts,
        generation: 1,
    };
    services.drafts().update(cx, |registry, _| {
        let failed = Err(DraftError {
            kind: DraftErrorKind::Io,
        });
        assert!(registry.apply_listing(launch, failed));
        assert_eq!(registry.recovered(), None);
    });
    drop(gate);
    cx.run_until_parked();
    services
        .drafts()
        .update(cx, |registry, cx| registry.reload_listing(cx));
    cx.run_until_parked();
    assert_eq!(listed(&services, cx).map(|ids| ids.len()), Some(1));
    services.drafts().read_with(cx, |registry, _| {
        assert_eq!(registry.recovered(), None);
    });
}
