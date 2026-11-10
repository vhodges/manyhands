//! What the registry tells the retirement loop of an item's drafts (D6 Task
//! 11b, RT-4 to RT-6, R-36): yes, no, or not known, and a fresh listing.

use std::sync::Arc;

use gpui_kit::{AppContext as _, Entity, TestAppContext};
use manyhands::{
    canonical::ItemId,
    drafts::{
        CommentTarget, DraftError, DraftErrorKind, DraftFields, DraftId, DraftTarget, ItemKind,
    },
    repository::ItemDtoKind,
};

use super::{DraftRegistry, Fresh};
use crate::desktop::{
    model::polling::retire_loop::Held,
    services::{Backend, ReadKind, worker::WorkerHandle},
    test_support::{delaying::DelayingBackend, fixtures::Fixture},
};

const REPO: &str = "/a/repository";
const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FA1";
const OTHER: &str = "01ARZ3NDEKTSV4RRFFQ69G5FA2";
const COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC1";

fn id(text: &str) -> ItemId {
    text.parse().unwrap()
}

fn target(repo: &str, kind: ItemKind, item: &str, comment: Option<&str>) -> DraftTarget {
    DraftTarget {
        repository: repo.to_owned(),
        item_kind: kind,
        item_id: id(item),
        comment: comment.map(|comment| CommentTarget {
            comment_id: id(comment),
            parent_id: None,
        }),
    }
}

fn fields(target: &DraftTarget) -> DraftFields {
    match (&target.comment, target.item_kind) {
        (Some(_), _) => DraftFields::Comment {
            body: "Words".to_owned(),
        },
        (None, ItemKind::Ticket) => DraftFields::Ticket {
            title: "T".to_owned(),
            ticket_type: "task".to_owned(),
            status: "open".to_owned(),
            project: None,
            team: None,
            deps: Vec::new(),
            parent: None,
            body: "Words".to_owned(),
        },
        (None, ItemKind::Document) => DraftFields::Document {
            title: "D".to_owned(),
            body: "Words".to_owned(),
        },
    }
}

/// Stores a draft of `target` and lets go of its claim.
fn store(fixture: &Fixture, target: DraftTarget) -> DraftId {
    let current = fields(&target);
    match fixture.draft_store().create(target, None, current) {
        Ok(claim) => claim.draft().id,
        Err(error) => panic!("the draft must be stored: {error}"),
    }
}

fn registry(backend: Arc<DelayingBackend>, cx: &mut TestAppContext) -> Entity<DraftRegistry> {
    let backend: Arc<dyn Backend> = backend;
    let (worker, _events) = WorkerHandle::spawn(backend.clone());
    cx.new(|_| DraftRegistry::new(backend, worker))
}

/// Lists the store into `registry` and waits for it to land.
fn list(registry: &Entity<DraftRegistry>, cx: &mut TestAppContext) {
    registry.update(cx, |registry, cx| {
        registry.reload_listing(cx);
    });
    cx.run_until_parked();
}

fn held(
    registry: &Entity<DraftRegistry>,
    kind: ItemDtoKind,
    item: &str,
    cx: &mut TestAppContext,
) -> Held {
    registry.read_with(cx, |registry, _| {
        registry.held_by(&REPO.to_owned(), kind, &id(item))
    })
}

/// RT-4 and RT-5: a draft of the item or of a comment on it, whoever holds
/// it, counts; so does a listing that cannot be read. A draft of another
/// item, kind or repository does not, but an entry that cannot be read
/// leaves every item unknown.
#[gpui_kit::test]
fn drafts_held_by_table(cx: &mut TestAppContext) {
    use ItemDtoKind::{Document, Ticket};
    let fixture = Fixture::new();
    let registry = registry(fixture.delaying_backend(), cx);

    // No listing has landed: nothing is known.
    assert_eq!(
        held(&registry, Ticket, ITEM, cx),
        Held::Unknown,
        "not landed"
    );

    list(&registry, cx);
    assert_eq!(
        held(&registry, Ticket, ITEM, cx),
        Held::No,
        "an empty store"
    );

    // Drafts of other items, kinds and repositories hold this one's copy
    // back from nothing.
    store(&fixture, target(REPO, ItemKind::Ticket, OTHER, None));
    store(&fixture, target("/another", ItemKind::Ticket, ITEM, None));
    store(&fixture, target(REPO, ItemKind::Document, ITEM, None));
    list(&registry, cx);
    assert_eq!(
        held(&registry, Ticket, ITEM, cx),
        Held::No,
        "others' drafts"
    );
    assert_eq!(held(&registry, Document, ITEM, cx), Held::Yes, "its kind's");

    // The item's own draft, and a comment's draft, however held.
    let own = store(&fixture, target(REPO, ItemKind::Ticket, ITEM, None));
    list(&registry, cx);
    assert_eq!(held(&registry, Ticket, ITEM, cx), Held::Yes, "its draft");
    assert_eq!(held(&registry, Ticket, OTHER, cx), Held::Yes, "another's");
    fixture.draft_store().claim(own).unwrap().discard().unwrap();
    list(&registry, cx);
    assert_eq!(held(&registry, Ticket, ITEM, cx), Held::No, "discarded");
    store(
        &fixture,
        target(REPO, ItemKind::Ticket, ITEM, Some(COMMENT)),
    );
    list(&registry, cx);
    assert_eq!(
        held(&registry, Ticket, ITEM, cx),
        Held::Yes,
        "a comment's draft"
    );

    // An entry that cannot be read names no target: every item is unknown.
    let broken = DraftId::generate();
    let path = fixture
        .data_dir()
        .join("drafts")
        .join(format!("{broken}.json"));
    std::fs::write(&path, b"not a draft").unwrap();
    list(&registry, cx);
    assert_eq!(
        held(&registry, Ticket, ITEM, cx),
        Held::Yes,
        "its own draft first"
    );
    assert_eq!(
        held(&registry, Ticket, "01ARZ3NDEKTSV4RRFFQ69G5FA9", cx),
        Held::Unknown,
        "an unreadable entry"
    );
    std::fs::remove_file(&path).unwrap();
    list(&registry, cx);
    assert_eq!(
        held(&registry, Ticket, "01ARZ3NDEKTSV4RRFFQ69G5FA9", cx),
        Held::No,
        "readable again"
    );

    // A listing that failed is not a listing: unknown.
    registry.update(cx, |registry, cx| {
        let stamp = registry.reload_listing(cx);
        registry.apply_listing(
            stamp,
            Err(DraftError {
                kind: DraftErrorKind::Io,
            }),
        );
    });
    assert_eq!(
        held(&registry, Ticket, "01ARZ3NDEKTSV4RRFFQ69G5FA9", cx),
        Held::Unknown,
        "a failed listing"
    );
}

/// A session this process began is held before any listing shows its draft:
/// the cached listing is not reloaded when a session begins.
#[gpui_kit::test]
async fn a_live_session_is_held_before_it_is_listed(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let registry = registry(fixture.delaying_backend(), cx);
    list(&registry, cx);
    assert_eq!(
        held(&registry, ItemDtoKind::Ticket, ITEM, cx),
        Held::No,
        "listed empty"
    );
    let comment = target(REPO, ItemKind::Ticket, ITEM, Some(COMMENT));
    let current = fields(&comment);
    let begun = registry.update(cx, |registry, cx| {
        registry.begin(comment, None, current, cx)
    });
    begun.await.unwrap();
    assert_eq!(
        held(&registry, ItemDtoKind::Ticket, ITEM, cx),
        Held::Yes,
        "begun, not listed"
    );
}

/// A store that did not open is not known to be empty.
#[cfg(unix)]
#[gpui_kit::test]
fn an_unavailable_store_is_unknown(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let elsewhere = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(elsewhere.path(), fixture.data_dir().join("drafts")).unwrap();
    let registry = registry(fixture.delaying_backend(), cx);
    list(&registry, cx);
    assert!(registry.read_with(cx, |registry, _| registry.unavailable().is_some()));
    assert_eq!(
        held(&registry, ItemDtoKind::Ticket, ITEM, cx),
        Held::Unknown
    );
}

/// The fresh listing of RT-6 reads the store at the moment it is asked, off
/// the UI thread and behind the `Drafts` gate: a draft stored while the
/// listing is held shows once it lands, and a listing overtaken by a newer
/// one is not fresh.
#[gpui_kit::test]
async fn relist_reads_the_store_behind_its_gate(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let backend = fixture.delaying_backend();
    let registry = registry(backend.clone(), cx);
    list(&registry, cx);
    assert_eq!(held(&registry, ItemDtoKind::Ticket, ITEM, cx), Held::No);

    let release = backend.hold_read(ReadKind::Drafts);
    let task = registry.update(cx, |registry, cx| registry.relist(cx));
    cx.run_until_parked();
    // Stored between the first check and the fresh listing.
    store(&fixture, target(REPO, ItemKind::Ticket, ITEM, None));
    assert_eq!(
        held(&registry, ItemDtoKind::Ticket, ITEM, cx),
        Held::No,
        "the cached listing is old until the fresh one lands"
    );
    release.send(()).unwrap();
    assert_eq!(task.await, Fresh::Landed, "the fresh listing landed");
    assert_eq!(held(&registry, ItemDtoKind::Ticket, ITEM, cx), Held::Yes);

    // A newer listing asked meanwhile overtakes it.
    let release = backend.hold_read(ReadKind::Drafts);
    let overtaken = registry.update(cx, |registry, cx| registry.relist(cx));
    cx.run_until_parked();
    registry.update(cx, |registry, cx| {
        registry.reload_listing(cx);
    });
    cx.run_until_parked();
    let _ = release.send(());
    assert_eq!(
        overtaken.await,
        Fresh::Overtaken,
        "overtaken by a newer listing"
    );
}
