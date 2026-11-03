//! A comment's posting (design, Lane D) over real repositories and a real
//! draft store: each composer's text is a comment draft of the item, begun
//! on the first keystroke once the item's comment drafts are listed, and an
//! item's comment drafts reopen in their threads when its tab opens and
//! while it is open.

use gpui_kit::{TestAppContext, px, size};
use manyhands::{
    drafts::{CommentTarget, DraftFields, DraftId, DraftTarget, Held, ItemKind},
    repository::ItemDtoKind,
};

use super::super::discussion_tests::{
    A, ADA, BO, C, TICKET, commit_comment, composer_target, composer_text, flushed, new_composer,
    open_new_composer, press_reply, reply_composer, shell, stored_comments, type_into,
};
use crate::desktop::{
    model::navigation::OpenTarget,
    services::ReadKind,
    test_support::fixtures::{Fixture, item_id},
    view::harness::{Shell, key},
};

const KEPT_NEW: &str = "01ARZ3NDEKTSV4RRFFQ69G5FN0";
const KEPT_REPLY: &str = "01ARZ3NDEKTSV4RRFFQ69G5FN1";
const LATER_REPLY: &str = "01ARZ3NDEKTSV4RRFFQ69G5FN2";

fn target(comment: &str, parent: Option<&str>) -> CommentTarget {
    CommentTarget {
        comment_id: item_id(comment),
        parent_id: parent.map(item_id),
    }
}

/// Stores a comment draft of `TICKET` in `repo` and lets go of its claim,
/// as a process that ended would.
fn store_comment(fixture: &Fixture, repo: &str, comment: CommentTarget, body: &str) -> DraftId {
    let target = DraftTarget {
        repository: repo.to_owned(),
        item_kind: ItemKind::Ticket,
        item_id: item_id(TICKET),
        comment: Some(comment),
    };
    let fields = DraftFields::Comment {
        body: body.to_owned(),
    };
    match fixture.draft_store().create(target, None, fields) {
        Ok(claim) => claim.draft().id,
        Err(error) => panic!("the draft must be stored: {error}"),
    }
}

/// The item's comment drafts reopen in their threads as its tab opens: the
/// new comment's in its composer, a reply's in its thread's, under their
/// comment IDs, held here. One stored while the tab is open is adopted
/// too, and kept in its thread until Reply there shows it.
#[gpui_kit::test]
fn an_items_comment_drafts_reopen_in_their_threads(cx: &mut TestAppContext) {
    let (shell, root) = shell(1400., cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Discussed", "");
        commit_comment(fixture, root, A, None, 1, Some(ADA), "First.\n");
        commit_comment(fixture, root, C, None, 2, Some(BO), "Another.\n");
        let repo = key(root);
        store_comment(fixture, &repo, target(KEPT_NEW, None), "Kept new.");
        store_comment(fixture, &repo, target(KEPT_REPLY, Some(A)), "Kept reply.");
    });

    let composer = new_composer(&shell, cx);
    assert_eq!(composer_text(&composer, cx), "Kept new.");
    assert_eq!(
        composer_target(&shell, &composer, cx),
        Some(target(KEPT_NEW, None))
    );
    let (thread, reply) = reply_composer(&shell, cx).expect("the reply reopens");
    assert_eq!(thread, A);
    assert_eq!(composer_text(&reply, cx), "Kept reply.");
    assert_eq!(
        composer_target(&shell, &reply, cx),
        Some(target(KEPT_REPLY, Some(A)))
    );
    assert_eq!(
        shell.label("composer-replying-to", cx).as_deref(),
        Some("Replying to Ada Lovelace")
    );
    let stored = stored_comments(&shell.fixture);
    assert_eq!(stored.len(), 2);
    assert!(
        stored.iter().all(|(_, _, held)| *held == Held::Here),
        "both are held here"
    );

    // Stored while the tab is open, as the Drafts page's Open finds it.
    store_comment(
        &shell.fixture,
        &key(&root),
        target(LATER_REPLY, Some(C)),
        "Kept later.",
    );
    let drafts = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().drafts().clone());
    drafts.update(cx, |registry, cx| {
        registry.reload_listing(cx);
    });
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    shell.run(cx);
    let (thread, _) = reply_composer(&shell, cx).unwrap();
    assert_eq!(thread, A, "the open reply stays where it is");
    press_reply(&shell, 1, cx);
    let (thread, later) = reply_composer(&shell, cx).unwrap();
    assert_eq!(thread, C);
    assert_eq!(composer_text(&later, cx), "Kept later.");
    assert_eq!(
        composer_target(&shell, &later, cx),
        Some(target(LATER_REPLY, Some(C)))
    );
    assert!(
        stored_comments(&shell.fixture)
            .iter()
            .all(|(_, _, held)| *held == Held::Here)
    );
}

/// No comment ID is allocated nor draft begun while the item's comment
/// drafts are not yet listed: text typed then stays only in the composer.
/// The first keystroke after the listing begins the draft with the whole
/// text.
#[gpui_kit::test]
async fn a_draft_begins_on_the_first_keystroke_after_the_listing(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "Discussed", "");
    fixture.refresh(&root);
    let mut listing = None;
    let shell = Shell::launch(
        fixture,
        |backend| listing = Some(backend.hold_read(ReadKind::Drafts)),
        cx,
    );
    shell.visual(cx).simulate_resize(size(px(1440.), px(1400.)));
    let target_item = OpenTarget::Item {
        repo: key(&root),
        kind: ItemDtoKind::Ticket,
        id: item_id(TICKET),
    };
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target_item, cx));
    shell
        .until_drawn(cx, "the discussion", |shell, cx| {
            shell.present("new-comment-composer", cx)
        })
        .await;
    open_new_composer(&shell, cx);
    let composer = new_composer(&shell, cx);
    type_into(&shell, &composer, "Early", cx);
    assert_eq!(composer_target(&shell, &composer, cx), None, "no ID yet");
    assert!(stored_comments(&shell.fixture).is_empty(), "no draft yet");

    listing.take().unwrap().send(()).unwrap();
    let drafts = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().drafts().clone());
    shell
        .until_drawn(cx, "the draft listing", |_, cx| {
            drafts.read_with(cx, |registry, _| registry.listings_landed() > 0)
        })
        .await;
    assert_eq!(composer_target(&shell, &composer, cx), None);
    assert!(
        stored_comments(&shell.fixture).is_empty(),
        "the listing begins none"
    );

    type_into(&shell, &composer, " and more", cx);
    let target = composer_target(&shell, &composer, cx).expect("an ID on the keystroke");
    assert_eq!(target.parent_id, None);
    flushed(&shell, cx);
    let stored = stored_comments(&shell.fixture);
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].0, target);
    assert_eq!(stored[0].1, "Early and more");
}

/// A draft whose adoption lands after the user began writing in its
/// thread is held, not shown, and is shown once that thread's composer is
/// free again: here, its text deleted and cancelled.
#[gpui_kit::test]
fn a_draft_adopted_into_a_busy_thread_is_shown_once_it_is_free(cx: &mut TestAppContext) {
    let (shell, root) = shell(1400., cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Discussed", "");
        commit_comment(fixture, root, A, None, 1, Some(ADA), "First.\n");
    });
    open_new_composer(&shell, cx);
    let composer = new_composer(&shell, cx);

    // A draft stored elsewhere is listed; its claim waits at the gate.
    store_comment(
        &shell.fixture,
        &key(&root),
        target(KEPT_NEW, None),
        "Kept new.",
    );
    let drafts = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().drafts().clone());
    let listing = shell.backend.hold_read(ReadKind::Drafts);
    let claim = shell.backend.hold_read(ReadKind::Drafts);
    drafts.update(cx, |registry, cx| {
        registry.reload_listing(cx);
    });
    shell.run(cx);
    listing.send(()).unwrap();
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    shell.run(cx);
    assert_eq!(new_composer(&shell, cx), composer, "the claim is held");

    // The user writes meanwhile; then the claim lands.
    type_into(&shell, &composer, "Mine.", cx);
    claim.send(()).unwrap();
    flushed(&shell, cx);
    shell.frame(cx, |_, _| ());
    shell.run(cx);
    let shown = new_composer(&shell, cx);
    assert_eq!(composer_text(&shown, cx), "Mine.", "the user's text stays");
    assert!(
        drafts.read_with(cx, |registry, _| registry
            .comment_drafts(&crate::desktop::services::drafts::comments::ItemKey {
                repo: key(&root),
                kind: ItemDtoKind::Ticket,
                id: item_id(TICKET),
            })
            .iter()
            .all(|draft| draft.live)),
        "the adopted draft is held here"
    );

    // Freed: the text deleted and cancelled, the held draft is shown.
    shell.keys("secondary-a backspace escape", cx);
    flushed(&shell, cx);
    shell.frame(cx, |_, _| ());
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    let adopted = new_composer(&shell, cx);
    assert_eq!(composer_text(&adopted, cx), "Kept new.");
    assert_eq!(
        composer_target(&shell, &adopted, cx),
        Some(target(KEPT_NEW, None))
    );
}
