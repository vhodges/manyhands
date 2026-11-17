//! Task 14: the Drafts page's rows of comment drafts an item tab holds
//! (Task 12 review). Show reaches the tab holding the draft, beside any
//! body draft that tab holds; Discard there resets the composer that held
//! it, so its later text goes to a new draft rather than nowhere.

use gpui_kit::TestAppContext;
use manyhands::repository::ItemDtoKind;

use super::super::super::discussion_tests::{
    TICKET, composer_target, composer_text, flushed, new_composer, pane, stored_comments, type_into,
};
use super::support::{KEPT, open_ticket, posting_shell, store_comment, until_adopted};
use crate::desktop::{
    model::navigation::OpenTarget,
    test_support::fixtures::item_id,
    view::{
        drafts::row_id,
        harness::{Shell, key},
    },
};

/// A second ticket of the repository.
const OTHER: &str = "01ARZ3NDEKTSV4RRFFQ69G5FQ0";

fn drafts_shown(shell: &Shell, cx: &mut TestAppContext) -> bool {
    shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.drafts_shown(cx))
}

/// Show on a comment draft held by an item tab shows that tab, from
/// another tab, and leaves the Drafts page.
#[gpui_kit::test]
async fn show_reaches_the_tab_holding_a_comment_draft(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let mut draft = None;
    let (shell, root) = posting_shell(false, cx, |fixture, root| {
        fixture.commit_ticket(root, OTHER, "Other", "");
        draft = Some(store_comment(fixture, root, KEPT, None, "Held here."));
    });
    let draft = draft.unwrap();
    until_adopted(&shell, "Held here.", cx).await;

    let other = OpenTarget::Item {
        repo: key(&root),
        kind: ItemDtoKind::Ticket,
        id: item_id(OTHER),
    };
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(other.clone(), cx));
    shell.run(cx);
    let opened = shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.opened(cx));
    assert_eq!(opened, Some(other));

    shell.keys("secondary-shift-d", cx);
    assert!(drafts_shown(&shell, cx));
    assert!(
        shell.present(row_id(draft, "show"), cx),
        "a held comment draft offers Show"
    );
    shell.click(row_id(draft, "show"), cx);
    shell.run(cx);
    assert!(!drafts_shown(&shell, cx), "Show leaves the Drafts page");
    let opened = shell
        .workspace
        .read_with(cx, |workspace, cx| workspace.opened(cx));
    assert_eq!(
        opened,
        Some(OpenTarget::Item {
            repo: key(&root),
            kind: ItemDtoKind::Ticket,
            id: item_id(TICKET),
        }),
        "the tab holding the draft is shown"
    );
}

/// Discard of a held comment draft on the Drafts page resets the
/// composer holding it: empty, with no comment ID, and the next text
/// begins a new draft.
#[gpui_kit::test]
async fn discarding_a_held_comment_draft_resets_its_composer(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let mut draft = None;
    let (shell, root) = posting_shell(false, cx, |fixture, root| {
        draft = Some(store_comment(fixture, root, KEPT, None, "Held here."));
    });
    let draft = draft.unwrap();
    until_adopted(&shell, "Held here.", cx).await;
    let tab = pane(&shell, cx);

    shell.keys("secondary-shift-d", cx);
    assert!(shell.present(row_id(draft, "discard"), cx));
    shell.click(row_id(draft, "discard"), cx);
    shell.click("discard-draft-confirm", cx);
    shell
        .until_drawn(cx, "the draft discarded", |shell, _| {
            stored_comments(&shell.fixture).is_empty()
        })
        .await;

    open_ticket(&shell, &root, cx);
    assert_eq!(pane(&shell, cx), tab, "the same tab");
    let composer = new_composer(&shell, cx);
    assert_eq!(composer_text(&composer, cx), "", "the composer is reset");
    assert_eq!(
        composer_target(&shell, &composer, cx),
        None,
        "no comment ID"
    );

    super::support::press(&shell, "new-comment-start", cx);
    shell.frame(cx, |_, _| ());
    let composer = new_composer(&shell, cx);
    type_into(&shell, &composer, "Written again.", cx);
    flushed(&shell, cx);
    let target = composer_target(&shell, &composer, cx).expect("a new comment ID");
    assert_ne!(target.comment_id, item_id(KEPT));
    let stored = stored_comments(&shell.fixture);
    assert_eq!(stored.len(), 1, "a new draft");
    assert_eq!(stored[0].0, target);
    assert_eq!(stored[0].1, "Written again.");
}
