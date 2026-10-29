//! D3's item frame by keyboard alone (Cycle "Keyboard", contract 13): the
//! tab strip, the header's actions and chips and the page in the design's
//! Tab order, both ways, for a ticket and a document; the details sheet
//! opened by Enter, walked, trapping focus, and closed by Escape, Done and
//! its close button, focus back on its opener each time; the discussion
//! reached by its key and by Comments, walked comment by comment and left;
//! and the read view focused by Tab, selected, copied as Markdown, showing
//! the address of the link under its caret, and left by Tab and Shift-Tab.
//!
//! Every step is a keystroke. The walk starts from the window's first
//! region, as D1's does, and after each step asserts which element holds
//! focus, that it is on screen, and that its region alone draws its ring.
//! The per-feature tests (`header_tests`, `sheet_tests`,
//! `discussion_tests`, `page_tests`) pin each piece; these prove they are
//! reached end to end, by keyboard alone.

use gpui_kit::{ElementId, Entity, Focusable, TestAppContext, test::TestWindowExt};
use zorite_editor::EditorState;

use super::Walk;
use crate::desktop::{
    model::editor_text::load,
    test_support::fixtures::Fixture,
    view::{harness::Shell, item::ItemPane, workspace::Region},
};

const MAIN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FW0";
const PARENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FW1";
const BLOCKER: &str = "01ARZ3NDEKTSV4RRFFQ69G5FW2";
const SECOND: &str = "01ARZ3NDEKTSV4RRFFQ69G5FW3";
const DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FW4";
const FIRST_COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FW5";
const REPLY: &str = "01ARZ3NDEKTSV4RRFFQ69G5FW6";
const LAST_COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FW7";
const SLUG: &str = "kb-m41n0";
const PARENT_SLUG: &str = "kb-p4r3n";

/// The read view's body: a line, then a link on a line of its own.
const READING: &str = "A first line to read.\n\nSee [the guide](guide.md) here.\n";

/// The document's source with `body`, its status, author and date strings.
fn document_source(body: &str) -> String {
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: {DOCUMENT}\n\
         title: Reading\nstatus: draft\nauthor: Ada Lovelace\ndate: 2026-10-05\n---\n{body}"
    )
}

/// A window over `a`: Main, with a type, status, project, team, a parent
/// with a short code and an open dependency, so every ticket chip shows,
/// and three comments, the second a reply; a second ticket; and a document
/// whose body holds a link, with every document chip.
fn chipped(cx: &mut TestAppContext) -> Walk {
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    fixture.commit_ticket(&a, PARENT, "Parent", &format!("slug: {PARENT_SLUG}\n"));
    fixture.create_ticket(&a, BLOCKER, "Blocker", "open");
    fixture.commit_ticket(
        &a,
        MAIN,
        "Main",
        &format!("slug: {SLUG}\nproject: p\nteam: t\nparent: {PARENT}\ndeps: [{BLOCKER}]\n"),
    );
    fixture.commit_ticket(&a, SECOND, "Second", "");
    for (ix, (id, parent)) in [
        (FIRST_COMMENT, None),
        (REPLY, Some(FIRST_COMMENT)),
        (LAST_COMMENT, None),
    ]
    .into_iter()
    .enumerate()
    {
        fixture.commit_comment(
            &a,
            MAIN,
            id,
            parent,
            &format!("2026-09-30T12:00:0{ix}Z"),
            "created_by: Ada Lovelace <ada@example.invalid>\n",
        );
    }
    fixture.commit_file(&a, "docs/reading.md", document_source(READING).as_bytes());
    fixture.refresh(&a);
    Walk::over(Shell::open(fixture, cx), a)
}

impl Walk {
    /// The shown item tab.
    pub(super) fn pane(&self, cx: &mut TestAppContext) -> Entity<ItemPane> {
        self.shell
            .workspace
            .read_with(cx, |workspace, _| workspace.item_pane())
            .expect("an item tab is shown")
    }

    /// Whether `editor` itself holds focus, in a frame drawn now.
    pub(super) fn editor_focused(
        &self,
        editor: &Entity<EditorState>,
        cx: &mut TestAppContext,
    ) -> bool {
        let handle = editor.read_with(cx, |editor, cx| editor.focus_handle(cx));
        self.shell
            .frame(cx, move |window, _| handle.is_focused(window))
    }

    /// The shown tab's read editor, in a frame drawn now.
    fn read_editor(&self, cx: &mut TestAppContext) -> Entity<EditorState> {
        self.shell.frame(cx, |_, _| ());
        let pane = self.pane(cx);
        pane.read_with(cx, |pane, cx| pane.read_editor(cx))
            .expect("the read view is shown")
    }

    /// Presses `key` once; then the element `id` holds focus, on screen,
    /// inside `region`, whose ring alone is drawn.
    pub(super) fn step(
        &self,
        key: &str,
        id: impl Into<ElementId>,
        region: Region,
        cx: &mut TestAppContext,
    ) {
        let id = id.into();
        self.keys(key, cx);
        assert!(self.holds_focus(&id, cx), "{key} reached {id:?}");
        self.on(id, region, cx);
    }

    /// Whether focus is on the element `id` or inside it, as the Kit's own
    /// inputs and buttons inside a labelled wrapper take it.
    pub(super) fn focus_inside(&self, id: impl Into<ElementId>, cx: &mut TestAppContext) -> bool {
        let id = id.into();
        self.shell.frame(cx, |window, _| {
            window.try_find(id).is_some_and(|wrapper| {
                gpui_kit::base::test_support::has_observed_focus(window, wrapper.path())
            })
        })
    }

    /// Presses `key`, Tab or Shift-Tab, until focus is inside the element
    /// `id`.
    pub(super) fn tab_into(&self, key: &str, id: impl Into<ElementId>, cx: &mut TestAppContext) {
        let id = id.into();
        let mut presses = 0;
        while !self.focus_inside(id.clone(), cx) {
            presses += 1;
            assert!(presses <= super::TAB_LIMIT, "{key} never reached {id:?}");
            self.keys(key, cx);
        }
    }

    /// The innermost element holding focus inside the element `within`:
    /// its ID and whether it is labeled.
    fn focused_in(
        &self,
        within: &'static str,
        cx: &mut TestAppContext,
    ) -> Option<(ElementId, bool)> {
        let within = ElementId::from(within);
        self.shell.frame(cx, |window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .into_iter()
                .filter(|snapshot| {
                    snapshot.path().contains(&within)
                        && gpui_kit::base::test_support::has_observed_focus(window, snapshot.path())
                })
                .max_by_key(|snapshot| snapshot.path().len())
                .and_then(|snapshot| {
                    let id = snapshot.path().last().cloned()?;
                    Some((id, snapshot.label().is_some()))
                })
        })
    }

    /// `secondary-2`, then the arrows reach the document tree's row `label`,
    /// a document's title or a file's path, and Enter opens it into a tab.
    /// Focus stays in the tree.
    pub(super) fn open_document(&self, label: &str, cx: &mut TestAppContext) {
        self.keys("secondary-2", cx);
        self.region(Region::Navigation, cx);
        let nodes = self.shell.rows("doc-node", cx);
        let at = nodes
            .iter()
            .position(|node| node == label)
            .unwrap_or_else(|| panic!("{label} is listed in {nodes:?}"));
        let mut presses = 0;
        while !self.shell.selected(("doc-node", at), cx) {
            presses += 1;
            assert!(presses <= nodes.len() + 1, "the arrows never reached it");
            self.keys("down", cx);
        }
        self.enter(cx);
        self.region(Region::Navigation, cx);
    }
}

/// The tab strip, then the header's Comments and Edit, then every chip,
/// then the page, by Tab; and all the way back by Shift-Tab, into the
/// strip's other tab. The header's unavailable actions take no focus. A
/// document's chips run the same way.
#[gpui_kit::test]
fn the_strip_header_chips_and_page_run_in_tab_order(cx: &mut TestAppContext) {
    let walk = chipped(cx);
    walk.region(Region::Repositories, cx);
    walk.select_a_repository(cx);
    walk.find(SECOND, cx);
    walk.find(MAIN, cx);
    assert_eq!(walk.tab_count(cx), 2);
    walk.keys("secondary-3", cx);
    walk.on(("tab", 1usize), Region::Content, cx);

    let forward: Vec<ElementId> = vec![
        ("tab-close-button", 1usize).into(),
        "item-comments".into(),
        "item-edit".into(),
        "chip-type".into(),
        "chip-status".into(),
        "chip-project".into(),
        "chip-team".into(),
        "chip-parent".into(),
        "chip-waiting-on".into(),
        "chip-all".into(),
        "item-page".into(),
    ];
    for id in &forward {
        walk.step("tab", id.clone(), Region::Content, cx);
    }
    let read = walk.read_editor(cx);
    assert!(walk.editor_focused(&read, cx), "the page's read view");
    walk.step("tab", "back-to-top", Region::Content, cx);

    // Back: the page, each chip, Edit, Comments, then the strip, both tabs.
    let mut back: Vec<ElementId> = forward.iter().rev().cloned().collect();
    back.push(("tab", 1usize).into());
    back.push(("tab-close-button", 0usize).into());
    back.push(("tab", 0usize).into());
    for id in back {
        walk.step("shift-tab", id, Region::Content, cx);
    }

    // A document: Status, Author, Date, Updated and All details.
    walk.open_document("Reading", cx);
    walk.keys("secondary-3", cx);
    let shown = walk.shown_tab(cx);
    walk.on(("tab", shown), Region::Content, cx);
    walk.tab_to("tab", "item-edit", cx);
    for id in [
        "chip-status",
        "chip-author",
        "chip-date",
        "chip-updated",
        "chip-all",
        "item-page",
    ] {
        walk.step("tab", id, Region::Content, cx);
    }
    for id in [
        "chip-all",
        "chip-updated",
        "chip-date",
        "chip-author",
        "chip-status",
        "item-edit",
    ] {
        walk.step("shift-tab", id, Region::Content, cx);
    }
}

/// The sheet's stops, from the one focused onward by Tab, until Tab comes
/// back to it: the trap. Each is labeled, but the Kit's own close button.
fn sheet_stops(walk: &Walk, first: &'static str, cx: &mut TestAppContext) -> Vec<ElementId> {
    let first = ElementId::from(first);
    let mut stops = vec![first.clone()];
    loop {
        walk.keys("tab", cx);
        assert!(has_sheet(walk, cx));
        let (id, labeled) = walk
            .focused_in("sheet-content", cx)
            .expect("focus stays in the sheet");
        if id == first {
            return stops;
        }
        assert!(
            labeled || id == ElementId::from("close"),
            "{id:?} is labeled"
        );
        walk.on(id.clone(), Region::Content, cx);
        stops.push(id);
        assert!(stops.len() <= super::TAB_LIMIT, "Tab never came back");
    }
}

pub(super) fn has_sheet(walk: &Walk, cx: &mut TestAppContext) -> bool {
    walk.shell.frame(cx, |window, cx| {
        gpui_kit::component::WindowExt::has_active_sheet(window, cx)
    })
}

/// All details by Tab and Enter opens the sheet on its Title; Tab walks
/// every row and control in it and comes back round, Shift-Tab too: it
/// traps focus. Escape, Done by Enter and the close button by Enter each
/// close it, and focus is back on All details, which Enter opens it from
/// again. Opened from Status by Enter, Escape gives focus back to Status.
///
/// Pinned here: every stop the sheet draws is named, the Updated row's
/// included, but for the Kit's own close button.
#[gpui_kit::test]
fn the_details_sheet_opens_by_enter_traps_focus_and_gives_it_back(cx: &mut TestAppContext) {
    let walk = chipped(cx);
    walk.select_a_repository(cx);
    walk.find(MAIN, cx);
    walk.keys("secondary-3", cx);
    walk.tab_to("tab", "chip-all", cx);
    walk.on("chip-all", Region::Content, cx);
    walk.enter(cx);
    assert!(has_sheet(&walk, cx), "Enter opened it");
    walk.on("sheet-title", Region::Content, cx);

    let stops = sheet_stops(&walk, "sheet-title", cx);
    for id in [
        ElementId::from("sheet-type"),
        "sheet-status".into(),
        "sheet-project".into(),
        "sheet-team".into(),
        "sheet-parent".into(),
        ("rel-parent", 0usize).into(),
        "sheet-waiting-on".into(),
        ("rel-dependency", 0usize).into(),
        "copy-slug-button".into(),
        "copy-id-button".into(),
        "sheet-changed".into(),
        "sheet-inspect".into(),
        "sheet-done".into(),
        "close".into(),
    ] {
        assert!(stops.contains(&id), "Tab reached {id:?}: {stops:?}");
    }
    walk.step("shift-tab", "close", Region::Content, cx);
    walk.step("tab", "sheet-title", Region::Content, cx);

    // Escape.
    walk.keys("escape", cx);
    assert!(!has_sheet(&walk, cx), "Escape closed it");
    walk.on("chip-all", Region::Content, cx);

    // Done, by Tab and Enter.
    walk.enter(cx);
    assert!(has_sheet(&walk, cx), "Enter opened it again");
    walk.tab_to("tab", "sheet-done", cx);
    walk.on("sheet-done", Region::Content, cx);
    walk.enter(cx);
    assert!(!has_sheet(&walk, cx), "Done closed it");
    walk.on("chip-all", Region::Content, cx);

    // The close button, by Shift-Tab and Enter.
    walk.enter(cx);
    walk.step("shift-tab", "close", Region::Content, cx);
    walk.enter(cx);
    assert!(!has_sheet(&walk, cx), "the close button closed it");
    walk.on("chip-all", Region::Content, cx);

    // From another chip.
    walk.tab_to("shift-tab", "chip-status", cx);
    walk.enter(cx);
    assert!(has_sheet(&walk, cx));
    walk.on("sheet-status", Region::Content, cx);
    walk.keys("escape", cx);
    assert!(!has_sheet(&walk, cx));
    walk.on("chip-status", Region::Content, cx);
}

/// The read editor of the comment `id`, in a frame drawn now.
fn comment_editor(walk: &Walk, id: &str, cx: &mut TestAppContext) -> Entity<EditorState> {
    walk.shell.frame(cx, |_, _| ());
    let view = walk
        .pane(cx)
        .read_with(cx, |pane, _| pane.comment_view(id))
        .unwrap_or_else(|| panic!("{id} has a read editor"));
    view.read_with(cx, |view, _| view.editor().clone())
}

/// `secondary-shift-c` from the strip focuses the discussion's heading;
/// Tab reaches Back to top and then each comment, Shift-Tab goes back, and
/// F6 leaves for the next region. Comments, by Tab and Enter, jumps there
/// too; Back to top, by Enter, focuses the page.
#[gpui_kit::test]
fn the_discussion_is_reached_by_its_key_walked_and_left(cx: &mut TestAppContext) {
    let walk = chipped(cx);
    walk.select_a_repository(cx);
    walk.find(MAIN, cx);
    walk.keys("secondary-3", cx);
    walk.on(("tab", 0usize), Region::Content, cx);
    walk.keys("secondary-shift-c", cx);
    walk.on("discussion-heading", Region::Content, cx);

    walk.step("tab", "back-to-top", Region::Content, cx);
    for id in [FIRST_COMMENT, REPLY, LAST_COMMENT] {
        walk.keys("tab", cx);
        let editor = comment_editor(&walk, id, cx);
        assert!(walk.editor_focused(&editor, cx), "Tab reached {id}");
        walk.region(Region::Content, cx);
    }
    walk.keys("shift-tab", cx);
    let reply = comment_editor(&walk, REPLY, cx);
    assert!(walk.editor_focused(&reply, cx), "Shift-Tab went back");
    walk.region(Region::Content, cx);

    // Focus leaves the discussion by F6, and comes back by `secondary-3`.
    walk.keys("f6", cx);
    walk.region(Region::Operations, cx);
    walk.keys("secondary-3", cx);
    walk.on(("tab", 0usize), Region::Content, cx);

    // Comments, by Tab and Enter.
    walk.tab_to("tab", "item-comments", cx);
    walk.on("item-comments", Region::Content, cx);
    walk.enter(cx);
    walk.on("discussion-heading", Region::Content, cx);

    // Back to top, by Enter: the page takes focus, and Tab goes on into
    // its read view.
    walk.step("tab", "back-to-top", Region::Content, cx);
    walk.enter(cx);
    walk.on("item-page", Region::Content, cx);
    let read = walk.read_editor(cx);
    assert!(!walk.editor_focused(&read, cx), "the page itself");
    walk.keys("tab", cx);
    assert!(walk.editor_focused(&read, cx), "then its read view");
    walk.on("item-page", Region::Content, cx);
}

/// Whether the read editor draws the link's address: the address's first
/// and last offsets lie apart on screen. While the link's markers are
/// hidden, both sit at the one column after its text.
fn address_shown(walk: &Walk, editor: &Entity<EditorState>, cx: &mut TestAppContext) -> bool {
    let start = READING.find("guide.md").expect("the address");
    let end = start + "guide.md".len();
    walk.shell.frame(cx, |_, _| ());
    editor.read_with(cx, |editor, _| {
        match (
            editor.bounds_for_offset(start),
            editor.bounds_for_offset(end),
        ) {
            (Some(start), Some(end)) => end.left() > start.left(),
            _ => false,
        }
    })
}

/// The selected text, as Copy puts it on the clipboard.
fn copied(walk: &Walk, cx: &mut TestAppContext) -> Option<String> {
    walk.keys("secondary-c", cx);
    cx.read_from_clipboard().and_then(|item| item.text())
}

/// The read view, focused by Tab from All details: Shift and the arrows
/// select, Copy copies what is selected, Select All and Copy copy the
/// whole body as Markdown; the caret moved onto the link shows its
/// address; Tab moves on to Back to top and Shift-Tab back, and on to All
/// details. Nothing is indented or typed.
#[gpui_kit::test]
fn the_read_view_selects_copies_and_lets_tab_move_on(cx: &mut TestAppContext) {
    let walk = chipped(cx);
    walk.select_a_repository(cx);
    walk.open_document("Reading", cx);
    walk.keys("secondary-3", cx);
    walk.tab_to("tab", "chip-all", cx);
    walk.step("tab", "item-page", Region::Content, cx);
    let read = walk.read_editor(cx);
    assert!(walk.editor_focused(&read, cx), "Tab focused the read view");
    let shown = read.read_with(cx, |editor, _| editor.text().to_owned());
    assert!(shown == load(READING));
    assert_eq!(read.read_with(cx, |editor, _| editor.cursor()), 0);

    // Select by Shift and the arrows, then Copy.
    walk.keys(
        "shift-right shift-right shift-right shift-right shift-right",
        cx,
    );
    assert_eq!(copied(&walk, cx).as_deref(), Some("A fir"));
    walk.keys("secondary-a", cx);
    assert!(copied(&walk, cx).as_deref() == Some(load(READING).as_str()));

    // The caret onto the link: its address is drawn.
    walk.keys("up up up up right", cx);
    assert!(!address_shown(&walk, &read, cx), "hidden off the link");
    walk.keys("down down right right right right right right", cx);
    assert!(
        read.read_with(cx, |editor, _| editor.cursor()) > READING.find('[').unwrap(),
        "the caret is on the link"
    );
    assert!(address_shown(&walk, &read, cx), "the link's address shows");
    assert!(walk.editor_focused(&read, cx));

    // Tab and Shift-Tab move focus on.
    walk.step("tab", "back-to-top", Region::Content, cx);
    walk.keys("shift-tab", cx);
    assert!(walk.editor_focused(&read, cx), "Shift-Tab came back");
    walk.on("item-page", Region::Content, cx);
    walk.step("shift-tab", "chip-all", Region::Content, cx);
    assert!(read.read_with(cx, |editor, _| editor.text().to_owned()) == shown);
    assert!(
        std::fs::read_to_string(walk.a.join("docs/reading.md")).unwrap()
            == document_source(READING),
        "nothing was written"
    );
}
