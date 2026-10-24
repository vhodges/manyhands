//! The Drafts page's Inspect and Export: Inspect on a changed row shows
//! the draft's fields and the comparison, each change in words, and
//! Escape closes it, focus back on Inspect; Export… writes the draft's
//! Markdown where the save dialog says, Export raw… an unreadable file's
//! bytes, and an export that cannot be written says so and leaves an
//! existing file as it was.
//!
//! Draft types derive no `Debug`; tests read them through `match` and
//! compare text.

use gpui_kit::{TestAppContext, test::TestWindowExt};
use manyhands::{
    drafts::{Change, DraftId, Entry, markdown},
    repository::ItemDtoKind,
};

use super::{
    actions::{EXPORTED, export_failed},
    row_id,
    tests::{ONE, with_ticket},
};
use crate::desktop::{
    test_support::{drafts::store_draft, fixtures::Fixture},
    view::{
        draft_comparison::{CHANGED, change_label},
        harness::Shell,
    },
};

/// The stored draft `id`'s Markdown, as the library renders it.
fn markdown_of(fixture: &Fixture, id: DraftId) -> String {
    let listed = fixture.draft_store().list().unwrap();
    let listing = listed.into_iter().find(|listing| listing.id == id).unwrap();
    match listing.entry {
        Entry::Draft(draft) => markdown(&draft),
        Entry::Unreadable(_) => panic!("the draft must be readable"),
    }
}

/// Inspect on a row whose item changed shows the draft's fields and the
/// comparison, each change as text; Escape closes it and the focus goes
/// back to Inspect.
#[gpui_kit::test]
fn inspect_shows_the_comparison_and_escape_returns_to_inspect(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let a = with_ticket(&fixture, "a", ONE, "One");
    let id = store_draft(&fixture, &a, ItemDtoKind::Ticket, ONE, "Mine");
    fixture.save_ticket(&a, ONE, "Theirs");
    let shell = Shell::open(fixture, cx);
    shell.keys("secondary-shift-d", cx);
    let state = shell.label(row_id(id, "state"), cx).unwrap_or_default();
    assert!(state.starts_with(CHANGED), "{state}");

    shell.click(row_id(id, "inspect"), cx);
    assert!(shell.present("drafts-inspect", cx));
    assert!(shell.frame(cx, |window, _| {
        window.find("drafts-inspect").focused() == Some(true)
    }));
    assert_eq!(
        shell.label("drafts-inspect-title", cx).as_deref(),
        Some("One")
    );
    assert_eq!(
        shell.label("drafts-inspect-body", cx).as_deref(),
        Some("Mine")
    );
    assert!(shell.present("comparison", cx));
    let fields = shell.rows("comparison-field", cx);
    let changes = shell.rows("comparison-change", cx);
    let mine = shell.rows("comparison-mine", cx);
    let theirs = shell.rows("comparison-theirs", cx);
    let base = shell.rows("comparison-base", cx);
    // The item was retitled; the draft changed its body.
    let title = fields.iter().position(|field| field == "Title").unwrap();
    assert_eq!(changes[title], change_label(Change::Theirs));
    assert_eq!(
        [&mine[title], &theirs[title], &base[title]],
        ["One", "Theirs", "One"]
    );
    let body = fields.iter().position(|field| field == "Body").unwrap();
    assert_eq!(changes[body], change_label(Change::Mine));
    assert_eq!(mine[body], "Mine");

    shell.keys("escape", cx);
    assert!(!shell.present("drafts-inspect", cx));
    assert!(shell.frame(cx, |window, _| {
        window.find(row_id(id, "inspect")).focused() == Some(true)
    }));
}

/// Export… asks where, and writes the draft's Markdown there.
#[gpui_kit::test]
fn export_writes_the_drafts_markdown(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let a = with_ticket(&fixture, "a", ONE, "One");
    let id = store_draft(&fixture, &a, ItemDtoKind::Ticket, ONE, "Mine");
    let expected = markdown_of(&fixture, id);
    let shell = Shell::open(fixture, cx);
    shell.keys("secondary-shift-d", cx);
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("One.md");

    shell.click(row_id(id, "export"), cx);
    let chosen = path.clone();
    cx.simulate_new_path_selection(move |_| Some(chosen));
    shell.run(cx);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), expected);
    assert_eq!(
        shell.label(row_id(id, "notice"), cx).as_deref(),
        Some(EXPORTED)
    );
    let names: Vec<_> = std::fs::read_dir(folder.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(names.len(), 1, "no temporary file is left: {names:?}");
}

/// Export raw… writes an unreadable draft's file as it is.
#[gpui_kit::test]
fn export_raw_writes_an_unreadable_drafts_bytes(cx: &mut TestAppContext) {
    const BYTES: &[u8] = b"not a draft \xff\x00";
    let fixture = Fixture::new();
    fixture.draft_store();
    let id = DraftId::generate();
    let stored = fixture.data_dir().join("drafts").join(format!("{id}.json"));
    std::fs::write(&stored, BYTES).unwrap();
    let shell = Shell::open(fixture, cx);
    shell.keys("secondary-shift-d", cx);
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("raw.json");

    shell.click(row_id(id, "export-raw"), cx);
    let chosen = path.clone();
    cx.simulate_new_path_selection(move |_| Some(chosen));
    shell.run(cx);
    assert_eq!(std::fs::read(&path).unwrap(), BYTES);
    assert_eq!(std::fs::read(&stored).unwrap(), BYTES, "the draft is kept");
}

/// An export over an existing file in a folder that cannot be written
/// says so in a fixed sentence and leaves that file's bytes as they were.
#[cfg(unix)]
#[gpui_kit::test]
fn a_failed_export_leaves_an_existing_file_unchanged(cx: &mut TestAppContext) {
    use std::os::unix::fs::PermissionsExt;

    // Permissions do not bind the superuser.
    if unsafe { libc::geteuid() } == 0 {
        return;
    }
    let fixture = Fixture::new();
    let a = with_ticket(&fixture, "a", ONE, "One");
    let id = store_draft(&fixture, &a, ItemDtoKind::Ticket, ONE, "Mine");
    let shell = Shell::open(fixture, cx);
    shell.keys("secondary-shift-d", cx);
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("One.md");
    std::fs::write(&path, b"kept as it was").unwrap();
    std::fs::set_permissions(folder.path(), std::fs::Permissions::from_mode(0o555)).unwrap();

    shell.click(row_id(id, "export"), cx);
    let chosen = path.clone();
    cx.simulate_new_path_selection(move |_| Some(chosen));
    shell.run(cx);
    let notice = shell.label(row_id(id, "notice"), cx);
    std::fs::set_permissions(folder.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(
        notice,
        Some(export_failed(manyhands::drafts::DraftErrorKind::Access))
    );
    assert_eq!(std::fs::read(&path).unwrap(), b"kept as it was");
}
