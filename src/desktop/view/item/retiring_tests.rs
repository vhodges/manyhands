//! A composer while the item's local copy is being retired (D6 Task 11b,
//! decision 19): it takes no new text, so no comment draft is begun; once
//! the retirement is over it takes text as ever. The rest of the state is
//! `retire_tabs_read_only`'s.

use gpui_kit::TestAppContext;
use manyhands::repository::ItemDtoKind;

use super::composer::ComposerState;
use super::discussion_tests::{
    TICKET, composer_text, flushed, new_composer, open_new_composer, shell, stored_comments,
    type_into,
};
use crate::desktop::{model::tabs::TabKey, view::harness::key};

/// Typing into the composer of an item marked "Retiring…" changes nothing
/// and stores no draft; after the mark is cleared, typing is kept and a
/// draft is begun.
#[gpui_kit::test]
async fn a_composer_begins_no_draft_while_retiring(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(1400., cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Discussed", "");
    });
    open_new_composer(&shell, cx);
    let composer = new_composer(&shell, cx);
    let services = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().clone());
    let item = (key(&root), ItemDtoKind::Ticket, TICKET.to_owned());

    let mark = services
        .operations()
        .update(cx, |operations, _| operations.mark_retiring(item.clone()));
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    assert!(composer.read_with(cx, |composer, _| composer.is_read_only()));
    type_into(&shell, &composer, "typed while retiring", cx);
    flushed(&shell, cx);
    assert_eq!(composer_text(&composer, cx), "", "no new text");
    assert!(stored_comments(&shell.fixture).is_empty(), "no draft begun");

    services.operations().update(cx, |operations, _| {
        assert!(operations.unmark_retiring(mark));
    });
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    assert!(!composer.read_with(cx, |composer, _| composer.is_read_only()));
    type_into(&shell, &composer, "typed after", cx);
    flushed(&shell, cx);
    assert_eq!(composer_text(&composer, cx), "typed after");
    assert_eq!(stored_comments(&shell.fixture).len(), 1, "a draft is begun");
}

/// RT-3: a comment being written holds its item back from the retirement
/// loop, through the tab itself: a composer that is posting, and one that
/// holds text that is not blank. A quiet composer does not.
#[gpui_kit::test]
async fn a_composer_holds_its_item_back_from_the_loop(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(1400., cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Discussed", "");
    });
    open_new_composer(&shell, cx);
    let composer = new_composer(&shell, cx);
    let services = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().clone());
    let tab = TabKey::Item {
        repo: key(&root),
        kind: ItemDtoKind::Ticket,
        id: TICKET.to_owned(),
    };
    let busy = |cx: &mut TestAppContext| {
        services
            .edit_tabs()
            .read_with(cx, |tabs, cx| tabs.item_busy(&tab, cx))
    };
    let state = |state: ComposerState, cx: &mut TestAppContext| {
        let composer = composer.clone();
        shell.frame(cx, move |window, cx| {
            composer.update(cx, |composer, cx| composer.set_state(state, window, cx));
        });
    };
    assert!(!busy(cx), "a quiet composer");

    state(ComposerState::Posting, cx);
    assert!(busy(cx), "posting, with no text");
    state(ComposerState::Idle, cx);
    assert!(!busy(cx), "idle again");

    type_into(&shell, &composer, "Half a thought", cx);
    assert!(busy(cx), "text that is not blank");
}
