//! The restored draft section's Export… and Discard…, the Drafts page's
//! own: Export… writes the session's newest text as Markdown; Discard…
//! asks first, naming the draft, Escape keeps it and gives the focus back,
//! and Confirm ends the session, removes the draft and the section goes.
//!
//! Draft types derive no `Debug`; tests read them through `match` and
//! compare text.

use std::path::{Path, PathBuf};

use gpui_kit::{TestAppContext, test::TestWindowExt};
use manyhands::{
    drafts::{DraftId, markdown},
    repository::ItemDtoKind,
};

use crate::desktop::{
    model::{navigation::OpenTarget, tabs::TabKey},
    test_support::{
        drafts::store_draft,
        fixtures::{Fixture, item_id},
    },
    view::{
        drafts::actions::{EXPORTED, discard_message},
        harness::{Shell, key},
    },
};

const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FM0";

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

/// A tab of the ticket with its stored draft restored.
fn restored(cx: &mut TestAppContext) -> (Shell, PathBuf, DraftId) {
    let fixture = Fixture::new();
    let root = with_ticket(&fixture);
    let id = store_draft(&fixture, &root, ItemDtoKind::Ticket, TICKET, "Draft body");
    let shell = Shell::open(fixture, cx);
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(ticket(&root), cx));
    shell.run(cx);
    shell.click("draft-restore-button", cx);
    assert!(shell.present("draft-section", cx));
    (shell, root, id)
}

fn kept(shell: &Shell, id: DraftId) -> bool {
    shell
        .fixture
        .data_dir()
        .join("drafts")
        .join(format!("{id}.json"))
        .exists()
}

#[gpui_kit::test]
fn the_sections_export_writes_the_sessions_markdown(cx: &mut TestAppContext) {
    let (shell, root, _) = restored(cx);
    let drafts = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().drafts().clone());
    let tab = TabKey::for_target(&ticket(&root));
    let expected = drafts.read_with(cx, |registry, _| {
        let id = registry.attached(&tab).unwrap();
        markdown(&registry.session(id).unwrap().draft())
    });
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("One.md");

    shell.click("draft-export", cx);
    let chosen = path.clone();
    cx.simulate_new_path_selection(move |_| Some(chosen));
    shell.run(cx);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), expected);
    assert_eq!(
        shell.label("draft-export-result", cx).as_deref(),
        Some(EXPORTED)
    );
}

#[gpui_kit::test]
fn the_sections_discard_asks_first_and_escape_keeps_the_draft(cx: &mut TestAppContext) {
    let (shell, _, id) = restored(cx);

    shell.click("draft-discard", cx);
    assert_eq!(
        shell.label("discard-draft-message", cx),
        Some(discard_message(Some("One")))
    );
    shell.keys("escape", cx);
    assert!(!shell.present("discard-draft-dialog", cx));
    assert!(kept(&shell, id));
    assert!(shell.present("draft-section", cx));
    assert!(shell.frame(cx, |window, _| {
        window.find("draft-discard").focused() == Some(true)
    }));
}

/// Confirm ends the session: the draft is removed and the section goes.
#[gpui_kit::test]
fn the_sections_discard_ends_the_session_and_the_section_goes(cx: &mut TestAppContext) {
    let (shell, root, id) = restored(cx);

    shell.click("draft-discard", cx);
    shell.click("discard-draft-confirm", cx);
    assert!(!kept(&shell, id));
    assert!(!shell.present("draft-section", cx));
    assert!(!shell.present("draft-banner", cx));
    let tab = TabKey::for_target(&ticket(&root));
    let drafts = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().drafts().clone());
    assert!(drafts.read_with(cx, |registry, _| registry.attached(&tab).is_none()));
}
