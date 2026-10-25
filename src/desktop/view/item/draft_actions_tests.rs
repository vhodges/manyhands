//! The restored draft section's Export… and Discard…, the Drafts page's
//! own: Export… writes the session's newest text as Markdown; Discard…
//! asks first, naming the draft, Escape keeps it and gives the focus back,
//! and Confirm ends the session, removes the draft and the section goes,
//! the tab taking the focus. Also: a discard the store refuses, Enter as
//! Cancel, the quitting gate, and a draft named by its kind.
//!
//! Draft types derive no `Debug`; tests read them through `match` and
//! compare text.

use std::path::{Path, PathBuf};

use gpui_kit::{TestAppContext, test::TestWindowExt};
use manyhands::{
    drafts::{DraftBase, DraftFields, DraftId, DraftTarget, Held, ItemKind, markdown},
    repository::ItemDtoKind,
};

use crate::desktop::{
    model::{drafts::rows::TICKET as TICKET_KIND, navigation::OpenTarget, tabs::TabKey},
    services::drafts::DraftRegistry,
    test_support::{
        drafts::{held, store_draft},
        fixtures::{Fixture, item_id},
    },
    view::{
        drafts::actions::{EXPORTED, discard_message},
        harness::{Shell, key},
        item::NOT_DISCARDED,
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
    // The section and its Discard… are gone: the tab takes the focus.
    assert!(shell.frame(cx, |window, _| {
        window.find(("tab", 0usize)).focused() == Some(true)
    }));
}

fn drafts(shell: &Shell, cx: &mut TestAppContext) -> gpui_kit::Entity<DraftRegistry> {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().drafts().clone())
}

/// A discard the store refuses (a drafts folder that cannot be written)
/// has still ended the session and released the draft, kept on disk: the
/// section goes, the banner says why and offers the draft again.
#[cfg(unix)]
#[gpui_kit::test]
fn a_failed_discard_releases_the_draft_and_says_so(cx: &mut TestAppContext) {
    use crate::desktop::test_support::drafts::{FolderMode, superuser};

    if superuser() {
        return;
    }
    let (shell, _, id) = restored(cx);
    let folder = shell.fixture.data_dir().join("drafts");
    let read_only = FolderMode::set(&folder, 0o500);

    shell.click("draft-discard", cx);
    shell.click("discard-draft-confirm", cx);
    assert!(kept(&shell, id));
    assert_eq!(
        shell.label("draft-discard-failed", cx),
        Some(format!(
            "{NOT_DISCARDED}: Access to the drafts folder was denied."
        ))
    );
    assert!(!shell.present("draft-section", cx));
    assert!(drafts(&shell, cx).read_with(cx, |registry, _| registry.session(id).is_none()));
    assert!(shell.present("draft-banner", cx), "offered again");
    drop(read_only);
    assert!(matches!(
        held(&shell.fixture.draft_store(), id),
        Some(Held::No)
    ));
}

/// Enter in the Discard dialog takes its default, Cancel.
#[gpui_kit::test]
fn enter_in_the_sections_discard_keeps_the_draft(cx: &mut TestAppContext) {
    let (shell, _, id) = restored(cx);

    shell.click("draft-discard", cx);
    shell.press("enter", cx);
    assert!(!shell.present("discard-draft-dialog", cx));
    assert!(kept(&shell, id));
    assert!(shell.present("draft-section", cx));
}

/// While quitting saves drafts, neither the section's Discard… nor its
/// Export… opens a dialog.
#[gpui_kit::test]
fn the_sections_dialogs_wait_while_quitting_saves_drafts(cx: &mut TestAppContext) {
    let (shell, _, id) = restored(cx);
    shell
        .operations(cx)
        .update(cx, |operations, _| operations.set_saving_drafts(true));

    shell.click("draft-discard", cx);
    assert!(!shell.present("discard-draft-dialog", cx));
    shell.click("draft-export", cx);
    assert!(!cx.did_prompt_for_new_path());
    assert!(kept(&shell, id));
}

/// A draft with no title is named by its kind, as the Drafts page names
/// it.
#[gpui_kit::test]
fn a_draft_with_no_title_is_named_by_its_kind(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = with_ticket(&fixture);
    let service = fixture.service();
    let repo = service.resolve_repository(&root).unwrap();
    let item = service.show_item(&repo, &item_id(TICKET)).unwrap();
    let mut current = DraftFields::of(&item).unwrap();
    if let DraftFields::Ticket { title, .. } = &mut current {
        title.clear();
    }
    let target = DraftTarget {
        repository: key(&root),
        item_kind: ItemKind::Ticket,
        item_id: item_id(TICKET),
        comment: None,
    };
    let created = fixture
        .draft_store()
        .create(target, DraftBase::of(&item), current);
    assert!(created.is_ok());
    drop(created);
    let shell = Shell::open(fixture, cx);
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(ticket(&root), cx));
    shell.run(cx);
    shell.click("draft-restore-button", cx);

    shell.click("draft-discard", cx);
    assert_eq!(
        shell.label("discard-draft-message", cx),
        Some(discard_message(Some(TICKET_KIND)))
    );
}
