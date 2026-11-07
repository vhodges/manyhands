//! Comment drafts in the registry: begun under the caller's comment ID, one
//! per thread, flushed on the idle delay, adopted again after the process
//! that began them is gone, listed only once the store is, retired and
//! discarded, and never an item's stored draft. A retirement writes the
//! newest text first, and a stored draft retires the revision its claim
//! reads.
//!
//! Draft types derive no `Debug`, so stored drafts are read through
//! `match`.

use std::{sync::Arc, time::Duration};

use gpui_kit::{AppContext as _, Entity, TestAppContext};
use manyhands::{
    canonical::ItemId,
    drafts::{
        CommentTarget, Draft, DraftErrorKind, DraftFields, DraftId, DraftStore, DraftTarget, Entry,
        Held, ItemKind, Listing,
    },
    repository::ItemDtoKind,
};

use super::{AdoptedComment, CommentDraft, ItemKey};
use crate::desktop::{
    model::drafts::{IDLE, RecoveryState},
    services::{Backend, ReadKind, Services, drafts::DraftRegistry, worker::WorkerHandle},
    test_support::{delaying::DelayingBackend, fixtures::Fixture},
};

const REPOSITORY: &str = "/a/repository";
const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG0";
const COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC1";
const REPLY: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC2";

fn item() -> ItemKey {
    ItemKey {
        repo: REPOSITORY.to_owned(),
        kind: ItemDtoKind::Ticket,
        id: TICKET.parse().unwrap(),
    }
}

fn comment_id(text: &str) -> ItemId {
    text.parse().unwrap()
}

/// A new comment of the item's discussion, or a reply under `parent`.
fn target(comment: &str, parent: Option<&str>) -> CommentTarget {
    CommentTarget {
        comment_id: comment_id(comment),
        parent_id: parent.map(comment_id),
    }
}

fn body(text: &str) -> DraftFields {
    DraftFields::Comment {
        body: text.to_owned(),
    }
}

fn start(fixture: &Fixture, cx: &mut TestAppContext) -> (Arc<DelayingBackend>, Services) {
    let backend = fixture.delaying_backend();
    let services = cx.update(|cx| Services::start(backend.clone(), cx));
    (backend, services)
}

/// Stores a comment draft of the item and lets go of its claim, as a
/// process that ended would.
fn store_comment(store: &DraftStore, comment: CommentTarget, text: &str) -> DraftId {
    let target = DraftTarget {
        repository: REPOSITORY.to_owned(),
        item_kind: ItemKind::Ticket,
        item_id: TICKET.parse().unwrap(),
        comment: Some(comment),
    };
    match store.create(target, None, body(text)) {
        Ok(claim) => claim.draft().id,
        Err(error) => panic!("the draft must be stored: {error}"),
    }
}

/// Begins a comment draft through the registry and waits for it.
async fn begin(
    drafts: &Entity<DraftRegistry>,
    comment: CommentTarget,
    text: &str,
    cx: &mut TestAppContext,
) -> DraftId {
    let begun = drafts.update(cx, |registry, cx| {
        registry.begin_comment(item(), comment, text.to_owned(), cx)
    });
    cx.run_until_parked();
    match begun.await {
        Ok(id) => id,
        Err(kind) => panic!("the comment draft must begin: {kind:?}"),
    }
}

async fn adopt(
    drafts: &Entity<DraftRegistry>,
    id: DraftId,
    cx: &mut TestAppContext,
) -> AdoptedComment {
    let adopted = drafts.update(cx, |registry, cx| registry.adopt_comment(id, cx));
    cx.run_until_parked();
    match adopted.await {
        Ok(adopted) => adopted,
        Err(kind) => panic!("the comment draft must be adopted: {kind:?}"),
    }
}

/// The stored draft `id`: its revision, its comment target and its body,
/// and who holds it.
fn stored(store: &DraftStore, id: DraftId) -> Option<(u64, CommentTarget, String, Held)> {
    store
        .list()
        .unwrap()
        .into_iter()
        .find_map(|listing| match listing {
            Listing {
                id: listed,
                entry:
                    Entry::Draft(Draft {
                        revision,
                        target:
                            DraftTarget {
                                comment: Some(comment),
                                ..
                            },
                        current: DraftFields::Comment { body },
                        ..
                    }),
                held,
            } if listed == id => Some((revision, comment, body, held)),
            _ => None,
        })
}

/// Retires the comment draft `id` through the registry and waits for it.
async fn retire(
    drafts: &Entity<DraftRegistry>,
    id: DraftId,
    cx: &mut TestAppContext,
) -> Result<(), DraftErrorKind> {
    let retired = drafts.update(cx, |registry, cx| registry.retire_comment(id, cx));
    cx.run_until_parked();
    retired.await
}

fn comment_drafts(drafts: &Entity<DraftRegistry>, cx: &TestAppContext) -> Vec<CommentDraft> {
    drafts.read_with(cx, |registry, _| registry.comment_drafts(&item()))
}

#[gpui_kit::test]
async fn begin_keeps_the_callers_comment_id(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let (_backend, services) = start(&fixture, cx);
    cx.run_until_parked();
    let drafts = services.drafts().clone();

    let id = begin(&drafts, target(COMMENT, None), "First", cx).await;

    let (revision, comment, text, held) = stored(&fixture.draft_store(), id).expect("stored");
    assert_eq!(revision, 1);
    assert_eq!(comment, target(COMMENT, None));
    assert_eq!(text, "First");
    assert_eq!(held, Held::Here);
    assert_eq!(
        comment_drafts(&drafts, cx),
        vec![CommentDraft {
            id,
            target: target(COMMENT, None),
            live: true,
        }]
    );
}

/// A new comment and a reply of the same item are two drafts, each with
/// its own target and text.
#[gpui_kit::test]
async fn each_thread_keeps_its_own_draft(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let (_backend, services) = start(&fixture, cx);
    cx.run_until_parked();
    let drafts = services.drafts().clone();

    let new = begin(&drafts, target(COMMENT, None), "New comment", cx).await;
    let reply = begin(&drafts, target(REPLY, Some(COMMENT)), "A reply", cx).await;
    drafts.update(cx, |registry, cx| {
        registry.edit(reply, body("A reply, edited"), cx)
    });
    cx.executor().advance_clock(IDLE);
    cx.run_until_parked();

    let mut listed = comment_drafts(&drafts, cx);
    listed.sort_by_key(|draft| draft.id);
    let mut expected = vec![
        CommentDraft {
            id: new,
            target: target(COMMENT, None),
            live: true,
        },
        CommentDraft {
            id: reply,
            target: target(REPLY, Some(COMMENT)),
            live: true,
        },
    ];
    expected.sort_by_key(|draft| draft.id);
    assert_eq!(listed, expected);

    let adopted = adopt(&drafts, new, cx).await;
    assert_eq!(
        (adopted.id, adopted.target, adopted.body),
        (new, target(COMMENT, None), "New comment".to_owned())
    );
    let adopted = adopt(&drafts, reply, cx).await;
    assert_eq!(
        (adopted.id, adopted.target, adopted.body),
        (
            reply,
            target(REPLY, Some(COMMENT)),
            "A reply, edited".to_owned()
        )
    );
    let store = fixture.draft_store();
    assert_eq!(stored(&store, new).unwrap().2, "New comment");
    assert_eq!(stored(&store, reply).unwrap().2, "A reply, edited");

    // Another item's thread lists neither.
    drafts.read_with(cx, |registry, _| {
        let other = ItemKey {
            id: "01ARZ3NDEKTSV4RRFFQ69G5FG1".parse().unwrap(),
            ..item()
        };
        assert!(registry.comment_drafts(&other).is_empty());
        let document = ItemKey {
            kind: ItemDtoKind::Document,
            ..item()
        };
        assert!(registry.comment_drafts(&document).is_empty());
    });
}

/// `edit` and flushing are D2's: a comment's newer text is written once the
/// idle delay has passed, not before.
#[gpui_kit::test]
async fn an_edit_flushes_after_the_idle_delay(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let (backend, services) = start(&fixture, cx);
    cx.run_until_parked();
    let drafts = services.drafts().clone();
    let id = begin(&drafts, target(COMMENT, None), "First", cx).await;

    drafts.update(cx, |registry, cx| registry.edit(id, body("Newer"), cx));
    cx.executor().advance_clock(IDLE - Duration::from_millis(1));
    cx.run_until_parked();
    assert_eq!(backend.reads_seen(ReadKind::DraftWrite), 0);
    assert_eq!(stored(&fixture.draft_store(), id).unwrap().0, 1);
    assert_eq!(
        drafts.read_with(cx, |registry, _| registry.recovery_state(id)),
        Some(RecoveryState::Pending)
    );

    cx.executor().advance_clock(Duration::from_millis(1));
    cx.run_until_parked();
    assert_eq!(backend.reads_seen(ReadKind::DraftWrite), 1);
    let (revision, _, text, _) = stored(&fixture.draft_store(), id).unwrap();
    assert_eq!((revision, text.as_str()), (2, "Newer"));
    assert!(matches!(
        drafts.read_with(cx, |registry, _| registry.recovery_state(id)),
        Some(RecoveryState::Saved(_))
    ));
}

/// The process that began a comment draft is gone: its claim is let go of
/// and the draft stays stored. The next registry lists it, not live, and
/// adopts it with its comment ID, its parent and its newest text.
#[gpui_kit::test]
async fn a_dropped_registry_leaves_the_draft_to_adopt_with_its_id(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    // The process that goes: a registry of its own, which the test alone
    // holds, so dropping it ends it.
    let backend: Arc<dyn Backend> = fixture.delaying_backend();
    let (worker, _events) = WorkerHandle::spawn(backend.clone());
    let drafts = cx.new(|_| DraftRegistry::new(backend, worker));
    let id = begin(&drafts, target(REPLY, Some(COMMENT)), "First", cx).await;
    drafts.update(cx, |registry, cx| registry.edit(id, body("Newest"), cx));
    let flushed = drafts.update(cx, |registry, cx| registry.flush_now(id, cx));
    cx.run_until_parked();
    assert_eq!(flushed.await, Ok(()));

    drop(drafts);
    // An update flushes the app's effects, which releases the entity.
    cx.update(|_| {});
    cx.run_until_parked();
    let (_, _, _, held) = stored(&fixture.draft_store(), id).expect("still stored");
    assert_eq!(held, Held::No, "the claim went with the registry");

    let (_backend, services) = start(&fixture, cx);
    cx.run_until_parked();
    let drafts = services.drafts().clone();
    assert_eq!(
        comment_drafts(&drafts, cx),
        vec![CommentDraft {
            id,
            target: target(REPLY, Some(COMMENT)),
            live: false,
        }]
    );

    let adopted = adopt(&drafts, id, cx).await;
    assert_eq!(adopted.id, id);
    assert_eq!(adopted.target, target(REPLY, Some(COMMENT)));
    assert_eq!(adopted.body, "Newest");
    assert!(matches!(adopted.state, RecoveryState::Saved(_)));
    assert_eq!(
        comment_drafts(&drafts, cx),
        vec![CommentDraft {
            id,
            target: target(REPLY, Some(COMMENT)),
            live: true,
        }]
    );
    assert_eq!(stored(&fixture.draft_store(), id).unwrap().3, Held::Here);

    // Adopted, it goes on as any session does.
    drafts.update(cx, |registry, cx| {
        registry.edit(id, body("Newest, again"), cx)
    });
    cx.executor().advance_clock(IDLE);
    cx.run_until_parked();
    let (revision, _, text, _) = stored(&fixture.draft_store(), id).unwrap();
    assert_eq!((revision, text.as_str()), (3, "Newest, again"));

    // A second adoption answers the live session, not a second claim.
    let again = adopt(&drafts, id, cx).await;
    assert_eq!((again.id, again.body), (id, "Newest, again".to_owned()));
}

/// Until the first listing lands, no comment draft of the item is known,
/// stored or not; once it has, the stored one is listed.
#[gpui_kit::test]
fn comment_drafts_waits_for_the_first_listing(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let id = store_comment(&fixture.draft_store(), target(COMMENT, None), "Kept");
    let backend = fixture.delaying_backend();
    let release = backend.hold_read(ReadKind::Drafts);
    let services = cx.update(|cx| Services::start(backend.clone(), cx));
    cx.run_until_parked();
    let drafts = services.drafts().clone();

    drafts.read_with(cx, |registry, _| {
        assert_eq!(registry.listings_landed(), 0);
        assert!(registry.comment_drafts(&item()).is_empty());
    });

    release.send(()).unwrap();
    cx.run_until_parked();
    assert_eq!(
        comment_drafts(&drafts, cx),
        vec![CommentDraft {
            id,
            target: target(COMMENT, None),
            live: false,
        }]
    );
}

/// A comment saved by its post retires its draft: a live one ends and
/// leaves the store, and one only stored is claimed and retired.
#[gpui_kit::test]
async fn retiring_removes_the_draft(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let left = store_comment(&fixture.draft_store(), target(REPLY, Some(COMMENT)), "Left");
    let (_backend, services) = start(&fixture, cx);
    cx.run_until_parked();
    let drafts = services.drafts().clone();
    let id = begin(&drafts, target(COMMENT, None), "Posted", cx).await;

    assert_eq!(retire(&drafts, id, cx).await, Ok(()));
    assert!(stored(&fixture.draft_store(), id).is_none(), "retired");
    drafts.read_with(cx, |registry, _| {
        assert_eq!(registry.recovery_state(id), None, "the session ended");
        assert!(registry.current_id(id).is_none());
    });
    assert_eq!(
        comment_drafts(&drafts, cx),
        vec![CommentDraft {
            id: left,
            target: target(REPLY, Some(COMMENT)),
            live: false,
        }]
    );

    assert_eq!(retire(&drafts, left, cx).await, Ok(()));
    assert!(stored(&fixture.draft_store(), left).is_none(), "retired");
    assert!(comment_drafts(&drafts, cx).is_empty());
}

/// An edit still waiting for its write when the draft retires is written
/// first; the newest revision then retires, and nothing is left stored.
#[gpui_kit::test]
async fn retiring_with_an_edit_pending_retires_the_draft(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let (_backend, services) = start(&fixture, cx);
    cx.run_until_parked();
    let drafts = services.drafts().clone();
    let id = begin(&drafts, target(COMMENT, None), "Posted", cx).await;
    drafts.update(cx, |registry, cx| registry.edit(id, body("Newer"), cx));
    assert_eq!(
        drafts.read_with(cx, |registry, _| registry.recovery_state(id)),
        Some(RecoveryState::Pending)
    );

    assert_eq!(retire(&drafts, id, cx).await, Ok(()));
    assert!(stored(&fixture.draft_store(), id).is_none(), "retired");
    assert!(drafts.read_with(cx, |registry, _| registry.current_id(id).is_none()));
    assert!(comment_drafts(&drafts, cx).is_empty());
}

/// A write in flight when the draft retires lands first, and the edit that
/// came after it is written too; the newest revision then retires.
#[gpui_kit::test]
async fn retiring_with_a_write_in_flight_retires_the_draft(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let (backend, services) = start(&fixture, cx);
    cx.run_until_parked();
    let drafts = services.drafts().clone();
    let id = begin(&drafts, target(COMMENT, None), "Posted", cx).await;
    let release = backend.hold_read(ReadKind::DraftWrite);
    drafts.update(cx, |registry, cx| registry.edit(id, body("Held"), cx));
    cx.executor().advance_clock(IDLE);
    cx.run_until_parked();
    assert_eq!(backend.reads_seen(ReadKind::DraftWrite), 1, "in flight");
    drafts.update(cx, |registry, cx| registry.edit(id, body("After"), cx));

    let retired = drafts.update(cx, |registry, cx| registry.retire_comment(id, cx));
    cx.run_until_parked();
    assert!(
        drafts.read_with(cx, |registry, _| registry.current_id(id).is_some()),
        "the session lasts until its text is written"
    );
    assert!(stored(&fixture.draft_store(), id).is_some());

    release.send(()).unwrap();
    cx.run_until_parked();
    assert_eq!(retired.await, Ok(()));
    assert!(stored(&fixture.draft_store(), id).is_none(), "retired");
    assert!(drafts.read_with(cx, |registry, _| registry.current_id(id).is_none()));
}

/// A stored draft written again since the listing read it retires the
/// revision its claim reads, not the listing's: it is deleted, not kept.
#[gpui_kit::test]
async fn a_stored_draft_retires_at_the_revision_its_claim_reads(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let id = store_comment(&fixture.draft_store(), target(COMMENT, None), "First");
    let (_backend, services) = start(&fixture, cx);
    cx.run_until_parked();
    let drafts = services.drafts().clone();
    assert_eq!(
        drafts.read_with(cx, |registry, _| registry
            .listed(id)
            .map(|(draft, _)| draft.revision)),
        Some(1)
    );

    // Another process writes revision 2 and goes; the listing still reads 1.
    match fixture.draft_store().claim(id) {
        Ok(mut claim) => {
            if let Err(error) = claim.write(2, body("Second"), None) {
                panic!("the revision must be written: {error}");
            }
        }
        Err(error) => panic!("the draft must be claimed: {error}"),
    }
    assert_eq!(stored(&fixture.draft_store(), id).unwrap().0, 2);

    assert_eq!(retire(&drafts, id, cx).await, Ok(()));
    assert!(stored(&fixture.draft_store(), id).is_none(), "retired");
    assert!(comment_drafts(&drafts, cx).is_empty());
}

/// A draft another process holds is listed, not live, and adopting it
/// answers that it is open elsewhere.
#[gpui_kit::test]
async fn adopting_a_draft_held_elsewhere_answers_open_elsewhere(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let id = store_comment(&fixture.draft_store(), target(COMMENT, None), "Elsewhere");
    // Another process's store over the same folder holds its claim.
    let elsewhere = match DraftStore::open_in(fixture.data_dir()) {
        Ok(store) => store,
        Err(error) => panic!("the other store must open: {error}"),
    };
    let held = match elsewhere.claim(id) {
        Ok(claim) => claim,
        Err(error) => panic!("the draft must be claimed: {error}"),
    };
    let (_backend, services) = start(&fixture, cx);
    cx.run_until_parked();
    let drafts = services.drafts().clone();
    assert_eq!(
        comment_drafts(&drafts, cx),
        vec![CommentDraft {
            id,
            target: target(COMMENT, None),
            live: false,
        }]
    );

    let adopted = drafts.update(cx, |registry, cx| registry.adopt_comment(id, cx));
    cx.run_until_parked();
    assert!(matches!(adopted.await, Err(DraftErrorKind::OpenElsewhere)));
    assert!(drafts.read_with(cx, |registry, _| registry.current_id(id).is_none()));
    drop(held);
}

/// Two adoptions of one draft asked together: the second answers the
/// session the first made, not its own claim's refusal.
#[gpui_kit::test]
async fn two_adoptions_at_once_answer_one_session(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let id = store_comment(&fixture.draft_store(), target(COMMENT, None), "Once");
    let (_backend, services) = start(&fixture, cx);
    cx.run_until_parked();
    let drafts = services.drafts().clone();

    let (first, second) = drafts.update(cx, |registry, cx| {
        (
            registry.adopt_comment(id, cx),
            registry.adopt_comment(id, cx),
        )
    });
    cx.run_until_parked();
    let (first, second) = match (first.await, second.await) {
        (Ok(first), Ok(second)) => (first, second),
        _ => panic!("both adoptions must answer the session"),
    };
    assert_eq!((first.id, first.body.as_str()), (id, "Once"));
    assert_eq!((second.id, second.body.as_str()), (id, "Once"));
    assert_eq!(
        drafts.read_with(cx, |registry, _| registry.session_ids()),
        vec![id]
    );
}

/// The composer's Discard: a live comment draft ends and leaves the store,
/// and one only stored is discarded from the store.
#[gpui_kit::test]
async fn discarding_removes_the_draft(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let left = store_comment(&fixture.draft_store(), target(REPLY, Some(COMMENT)), "Left");
    let (_backend, services) = start(&fixture, cx);
    cx.run_until_parked();
    let drafts = services.drafts().clone();
    let id = begin(&drafts, target(COMMENT, None), "Unwanted", cx).await;
    drafts.update(cx, |registry, cx| {
        registry.edit(id, body("Unwanted, edited"), cx)
    });

    drafts.update(cx, |registry, cx| {
        registry.discard_comment(id, cx);
        registry.discard_comment(left, cx);
    });
    cx.run_until_parked();
    let store = fixture.draft_store();
    assert!(stored(&store, id).is_none(), "discarded");
    assert!(stored(&store, left).is_none(), "discarded");
    assert_eq!(
        drafts.read_with(cx, |registry, _| registry.recovery_state(id)),
        None
    );
    assert!(comment_drafts(&drafts, cx).is_empty());
}

/// `stored_for` offers an item's own draft, never a comment's of it.
#[gpui_kit::test]
fn stored_for_still_ignores_comment_drafts(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let id = store_comment(&fixture.draft_store(), target(COMMENT, None), "Kept");
    let (_backend, services) = start(&fixture, cx);
    cx.run_until_parked();

    services.drafts().read_with(cx, |registry, _| {
        assert_eq!(
            registry.stored_for(&REPOSITORY.to_owned(), ItemDtoKind::Ticket, TICKET),
            None
        );
        assert_eq!(
            registry
                .comment_drafts(&item())
                .iter()
                .map(|draft| draft.id)
                .collect::<Vec<_>>(),
            vec![id]
        );
    });
}
