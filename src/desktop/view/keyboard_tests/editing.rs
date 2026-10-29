//! D3's editing by keyboard alone (Cycle "Keyboard", contract 13): Edit by
//! `e` on the page and by `secondary-shift-e`, the mode key, every toolbar
//! control reached by the arrows along it and used by Enter or Space, Tab
//! indenting in the editor, Escape to the header's Done, F6 out of the
//! editor, `secondary-s` and Done; Continue editing after Done; the
//! stale-edit review, its controls and its body editor's Escape; the
//! details sheet's controls while editing and both ticket pickers; and
//! Create, Move…, New folder… and Repair…, each opened, filled and acted
//! on by key, Escape closing it with focus back on its opener.
//!
//! Every step is a keystroke, as in `frame`. The per-feature tests pin
//! each piece (`edit_tests`, `toolbar_tests`, `review_tests`,
//! `relations_edit_tests`, `create_tests`, `move_document_tests`,
//! `new_folder_tests`, `repair_tests`); these prove each is reached and
//! used end to end, by keyboard alone. Each waits on the worker, so each
//! allows parking.

use gpui_kit::{ElementId, Entity, Focusable, TestAppContext, test::TestWindowExt};
use manyhands::results::ResultCode;

use super::{
    Walk,
    frame::{has_sheet, sheet_stops},
};
use crate::desktop::{
    model::{
        edit::{CONTINUE_EDITING, Phase, fields::Field},
        editor_text::load,
    },
    services::requests::Intent,
    test_support::fixtures::{Fixture, ticket_path, ticket_source},
    view::{
        editor::{host::EditorHost, toolbar::controls::Control},
        harness::{Shell, is_save},
        workspace::Region,
    },
};

const EDITED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX0";
const STARTED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX1";

/// The edited ticket's body: "two" on its first line is what the inline
/// controls format, and its list's second item is what Tab indents.
const BODY: &str = "One two three\nFour five\n\n- one\n- two\n";
const ADDRESS: &str = "https://example.test";

/// A ticket's source with `body` in place of the fixture's own.
fn ticket_with_body(id: &str, title: &str, body: &str) -> String {
    ticket_source(id, title, "").replace(&format!("Body of {title}.\n"), body)
}

/// A window over `a` holding the tickets `ids`, each titled by its place
/// and with `BODY`.
fn editable(ids: &[&str], cx: &mut TestAppContext) -> Walk {
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    for (ix, id) in ids.iter().enumerate() {
        let title = format!("Ticket {ix}");
        let source = ticket_with_body(id, &title, BODY);
        fixture.commit_file(&a, &ticket_path(id), source.as_bytes());
    }
    fixture.refresh(&a);
    Walk::over(Shell::open(fixture, cx), a)
}

impl Walk {
    /// The shown tab's phase.
    fn phase(&self, cx: &mut TestAppContext) -> Phase {
        self.pane(cx).read_with(cx, |pane, _| pane.edit_phase())
    }

    /// `host`'s editor holds focus, inside the content region.
    fn in_editor(&self, host: &Entity<EditorHost>, cx: &mut TestAppContext) {
        let editor = host.read_with(cx, |host, _| host.editor().clone());
        assert!(
            self.editor_focused(&editor, cx),
            "the edit editor has focus"
        );
        self.on("item-page", Region::Content, cx);
    }

    /// Escape, from an editor: focus moves to the header's Done.
    fn escape_to_done(&self, cx: &mut TestAppContext) {
        self.step("escape", "item-done", Region::Content, cx);
    }

    /// `secondary-s`: it sends one save; waits until that has ended, and
    /// gives its code.
    async fn save_by_key(&self, cx: &mut TestAppContext) -> Option<ResultCode> {
        let saves = |cx: &mut TestAppContext| {
            self.shell
                .requests(cx, |request| is_save(&request.intent))
                .len()
        };
        let before = saves(cx);
        self.keys("secondary-s", cx);
        assert_eq!(saves(cx), before + 1, "secondary-s sent one save");
        self.ended(is_save, cx).await.map(|(_, code)| code)
    }
}

fn text(host: &Entity<EditorHost>, cx: &mut TestAppContext) -> String {
    host.read_with(cx, |host, cx| host.text(cx))
}

fn rich(host: &Entity<EditorHost>, cx: &mut TestAppContext) -> bool {
    host.read_with(cx, |host, _| host.is_rich())
}

/// Presses Right along the toolbar from where focus is until `control`
/// holds focus, then uses it by `key`, Enter or Space.
fn use_control(walk: &Walk, control: Control, key: &str, cx: &mut TestAppContext) {
    walk.tab_to("right", control.id(), cx);
    walk.on(control.id(), Region::Content, cx);
    walk.shell.press(key, cx);
    walk.shell.frame(cx, |_, _| ());
}

/// The toolbar's Undo, from where focus is on it: Home, Right to Undo,
/// Enter. Focus stays on Undo.
fn undo_by_toolbar(walk: &Walk, cx: &mut TestAppContext) {
    walk.keys("home", cx);
    walk.on(Control::Rich.id(), Region::Content, cx);
    use_control(walk, Control::Undo, "enter", cx);
    walk.on(Control::Undo.id(), Region::Content, cx);
}

/// Edit, by `secondary-shift-e` from the header and by `e` from the page;
/// the mode key; Tab indenting in the editor and Shift-Tab outdenting;
/// Escape to Done; F6 out of the editor; `secondary-s`; and Done, by Enter.
/// The toolbar is `every_toolbar_control_works_by_keyboard_alone`'s.
#[gpui_kit::test]
async fn edit_the_mode_key_save_and_done_by_keyboard_alone(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let walk = editable(&[STARTED, EDITED], cx);
    walk.select_a_repository(cx);

    // `secondary-shift-e` from the header, then Done by Enter: clean, so
    // the tab views again.
    walk.find(STARTED, cx);
    walk.keys("secondary-3", cx);
    walk.on(("tab", 0usize), Region::Content, cx);
    walk.tab_to("tab", "item-comments", cx);
    walk.on("item-comments", Region::Content, cx);
    walk.keys("secondary-shift-e", cx);
    let host = walk.shell.until_editing(cx).await;
    walk.in_editor(&host, cx);
    walk.escape_to_done(cx);
    walk.enter(cx);
    walk.shell
        .until_drawn(cx, "Done ended the session", |shell, cx| {
            !shell.present("item-done", cx)
        })
        .await;
    assert_eq!(walk.phase(cx), Phase::Viewing);
    walk.region(Region::Content, cx);

    // `e` from the page's read view.
    let host = edit_by_e(&walk, EDITED, cx).await;

    // The mode key, from the editor.
    assert!(rich(&host, cx));
    walk.keys("secondary-shift-m", cx);
    assert!(!rich(&host, cx), "Markdown");
    walk.keys("secondary-shift-m", cx);
    assert!(rich(&host, cx), "Rich text");
    walk.in_editor(&host, cx);

    // Tab indents the list's second item, Shift-Tab outdents it, and
    // focus stays in the editor.
    walk.keys("down down down down", cx);
    walk.keys("tab", cx);
    walk.in_editor(&host, cx);
    assert!(
        text(&host, cx).ends_with("- one\n    - two\n"),
        "Tab indented"
    );
    walk.keys("shift-tab", cx);
    walk.in_editor(&host, cx);
    assert!(text(&host, cx) == load(BODY), "Shift-Tab outdented");

    // Escape to Done, and Tab back into the editor through the toolbar.
    walk.escape_to_done(cx);
    walk.tab_to("tab", "item-page", cx);
    walk.in_editor(&host, cx);
    walk.type_text("x", cx);

    // F6 leaves the editor; the region keys come back to the tab.
    walk.keys("f6", cx);
    walk.region(Region::Operations, cx);
    walk.keys("secondary-3", cx);
    walk.region(Region::Content, cx);
    walk.tab_to("tab", "item-page", cx);
    walk.in_editor(&host, cx);

    // `secondary-s` saves, from the editor.
    assert_eq!(walk.save_by_key(cx).await, Some(ResultCode::Ok));
    walk.in_editor(&host, cx);

    // Escape to Done, and Done by Enter.
    walk.escape_to_done(cx);
    walk.enter(cx);
    walk.shell
        .until_drawn(cx, "Done ended the session", |shell, cx| {
            !shell.present("item-done", cx)
        })
        .await;
    assert_eq!(walk.phase(cx), Phase::Viewing);
    walk.region(Region::Content, cx);
}

/// `e` on the read view of the ticket `id`, found by Find and reached by
/// Tab: the edit editor opens, focused, and the letter is not typed.
async fn edit_by_e(walk: &Walk, id: &str, cx: &mut TestAppContext) -> Entity<EditorHost> {
    walk.find(id, cx);
    walk.keys("secondary-3", cx);
    walk.tab_to("tab", "item-page", cx);
    walk.keys("e", cx);
    let host = walk.shell.until_editing(cx).await;
    walk.in_editor(&host, cx);
    assert!(text(&host, cx) == load(BODY), "`e` was not typed");
    host
}

/// Every toolbar control, reached from Done by Tab and the arrows and
/// used by Enter or Space, with "two" selected in the editor; the Style
/// menu and the link popover closed by Escape with focus back on their
/// control; then Tab out of the toolbar into the editor.
#[gpui_kit::test]
async fn every_toolbar_control_works_by_keyboard_alone(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let walk = editable(&[EDITED], cx);
    walk.select_a_repository(cx);
    let host = edit_by_e(&walk, EDITED, cx).await;

    // Select "two", then Escape to Done and Tab into the toolbar.
    walk.keys(
        "home right right right right shift-right shift-right shift-right",
        cx,
    );
    walk.escape_to_done(cx);
    walk.tab_to("tab", Control::Rich.id(), cx);
    walk.on(Control::Rich.id(), Region::Content, cx);

    // The mode switch, by Enter and Space.
    use_control(&walk, Control::Markdown, "enter", cx);
    assert!(!rich(&host, cx), "Markdown by Enter");
    walk.step("left", Control::Rich.id(), Region::Content, cx);
    walk.shell.press("space", cx);
    assert!(rich(&host, cx), "Rich text by Space");

    // The inline marks: on, then off again, the other key each time.
    for (control, marked) in [
        (Control::Bold, "**two**"),
        (Control::Italic, "*two*"),
        (Control::Strike, "~~two~~"),
        (Control::Code, "`two`"),
    ] {
        use_control(&walk, control, "enter", cx);
        assert!(text(&host, cx).contains(marked), "{control:?} by Enter");
        walk.on(control.id(), Region::Content, cx);
        walk.shell.press("space", cx);
        assert!(text(&host, cx) == load(BODY), "{control:?} again by Space");
    }

    // Link: its popover, Escape closing it with focus back on Link and
    // nothing inserted; then an address typed and Enter. Undo and Redo.
    use_control(&walk, Control::Link, "enter", cx);
    assert!(walk.shell.present("toolbar-link-popover", cx));
    walk.keys("escape", cx);
    assert!(!walk.shell.present("toolbar-link-popover", cx), "Escape");
    walk.on(Control::Link.id(), Region::Content, cx);
    assert!(text(&host, cx) == load(BODY), "nothing inserted");
    walk.shell.press("enter", cx);
    walk.type_text(ADDRESS, cx);
    walk.shell.press("enter", cx);
    let linked = text(&host, cx);
    assert!(linked.contains(&format!("[two]({ADDRESS})")), "Link");
    walk.on(Control::Link.id(), Region::Content, cx);
    undo_by_toolbar(&walk, cx);
    assert!(text(&host, cx) == load(BODY), "Undo");
    use_control(&walk, Control::Redo, "space", cx);
    assert!(text(&host, cx) == linked, "Redo");
    walk.step("left", Control::Undo.id(), Region::Content, cx);
    walk.shell.press("enter", cx);
    assert!(text(&host, cx) == load(BODY));

    // Style: its menu, Escape closing it with focus back on Style; then
    // Heading 2 from it by the arrows and Enter.
    use_control(&walk, Control::Style, "enter", cx);
    assert!(walk.shell.present("toolbar-style-menu", cx));
    walk.keys("escape", cx);
    assert!(!walk.shell.present("toolbar-style-menu", cx), "Escape");
    walk.on(Control::Style.id(), Region::Content, cx);
    walk.shell.press("enter", cx);
    assert!(walk.shell.present("toolbar-style-menu", cx));
    walk.keys("down down", cx);
    walk.shell.press("enter", cx);
    assert!(text(&host, cx).starts_with("## One"), "Style");
    walk.on(Control::Style.id(), Region::Content, cx);
    undo_by_toolbar(&walk, cx);

    // The lists and the blocks, each undone by the toolbar's Undo.
    for (control, made) in [
        (Control::Bullets, "- One"),
        (Control::Numbered, "1. One"),
        (Control::Checklist, "- [ ] One"),
        (Control::Quote, "> One"),
        (Control::CodeBlock, "```\n"),
    ] {
        use_control(&walk, control, "enter", cx);
        assert!(text(&host, cx).contains(made), "{control:?}");
        walk.on(control.id(), Region::Content, cx);
        undo_by_toolbar(&walk, cx);
        assert!(text(&host, cx) == load(BODY), "{control:?} undone");
    }

    // Table, then each table control, the caret in the new table.
    use_control(&walk, Control::Table, "enter", cx);
    assert!(text(&host, cx).contains("| Column 1 | Column 2 |"), "Table");
    for control in [
        Control::RowBelow,
        Control::ColumnRight,
        Control::DeleteRow,
        Control::DeleteColumn,
        Control::AlignLeft,
        Control::AlignCentre,
        Control::AlignRight,
    ] {
        let before = text(&host, cx);
        use_control(&walk, control, "enter", cx);
        assert!(text(&host, cx) != before, "{control:?}");
        walk.on(control.id(), Region::Content, cx);
    }
    walk.keys("end", cx);
    walk.on(Control::AlignRight.id(), Region::Content, cx);

    // Tab leaves the toolbar for the editor.
    walk.keys("tab", cx);
    walk.in_editor(&host, cx);
}

/// Escape to Done and Done by Enter, with unsaved text: the session is
/// kept, the tab views, and Edit reads Continue editing.
async fn done_keeping(walk: &Walk, cx: &mut TestAppContext) {
    walk.escape_to_done(cx);
    walk.enter(cx);
    walk.shell
        .until_drawn(cx, "Done kept the session", |shell, cx| {
            !shell.present("item-done", cx)
        })
        .await;
    assert_eq!(walk.phase(cx), Phase::Viewing);
    assert_eq!(
        walk.shell.label("item-edit", cx).as_deref(),
        Some(CONTINUE_EDITING)
    );
    walk.region(Region::Content, cx);
}

/// After Done with unsaved text: the header's Continue editing, by
/// Shift-Tab and Enter; the kept banner's, by Tab and Enter; and `e` on
/// the page each show the kept editor again, focused, with its text.
#[gpui_kit::test]
async fn continue_editing_after_done_by_keyboard_alone(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let walk = editable(&[EDITED], cx);
    walk.select_a_repository(cx);
    let host = edit_by_e(&walk, EDITED, cx).await;
    walk.type_text("x", cx);
    let typed = text(&host, cx);
    assert!(typed != load(BODY), "typed");
    let resumed = |walk: &Walk, cx: &mut TestAppContext| {
        assert_eq!(walk.phase(cx), Phase::Editing);
        let again = walk.pane(cx).read_with(cx, |pane, _| pane.edit_host());
        assert_eq!(
            again.map(|again| again.entity_id()),
            Some(host.entity_id()),
            "the kept editor"
        );
        assert!(text(&host, cx) == typed, "with its text");
        walk.in_editor(&host, cx);
    };

    done_keeping(&walk, cx).await;
    walk.tab_to("shift-tab", "item-edit", cx);
    walk.on("item-edit", Region::Content, cx);
    walk.enter(cx);
    walk.shell.until_editing(cx).await;
    resumed(&walk, cx);

    done_keeping(&walk, cx).await;
    walk.tab_to("tab", "item-kept-continue", cx);
    walk.on("item-kept-continue", Region::Content, cx);
    walk.enter(cx);
    walk.shell.until_editing(cx).await;
    resumed(&walk, cx);

    done_keeping(&walk, cx).await;
    walk.tab_to("tab", "item-page", cx);
    walk.keys("e", cx);
    walk.shell.until_editing(cx).await;
    resumed(&walk, cx);
}

const REVIEWED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX2";
const OLD_PARENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX3";
const NEW_PARENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX4";
const DEPENDENCY: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX5";

/// A window over `a`: Reviewed, whose parent is Old parent, with `BODY`;
/// New parent; and Dependency, each with a short code.
fn related(cx: &mut TestAppContext) -> Walk {
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    fixture.commit_ticket(&a, OLD_PARENT, "Old parent", "slug: kb-p0001\n");
    fixture.commit_ticket(&a, NEW_PARENT, "New parent", "slug: kb-p0002\n");
    fixture.commit_ticket(&a, DEPENDENCY, "Dependency", "slug: kb-d3p00\n");
    let source = ticket_source(
        REVIEWED,
        "Reviewed",
        &format!("slug: kb-r3v00\nparent: {OLD_PARENT}\n"),
    )
    .replace("Body of Reviewed.\n", BODY);
    fixture.commit_file(&a, &ticket_path(REVIEWED), source.as_bytes());
    fixture.refresh(&a);
    Walk::over(Shell::open(fixture, cx), a)
}

/// Whether the picker is open, its filter focused.
fn picker_open(walk: &Walk, cx: &mut TestAppContext) -> bool {
    walk.shell.frame(cx, |_, _| ());
    let Some(filter) = walk.pane(cx).read_with(cx, |pane, _| pane.picker_filter()) else {
        return false;
    };
    let handle = filter.read_with(cx, |input, cx| input.focus_handle(cx));
    walk.shell.present("rel-picker", cx)
        && walk
            .shell
            .frame(cx, move |window, _| handle.is_focused(window))
}

/// The picker that `opener` opens, by Enter: it opens with its filter
/// focused; Escape closes it, the sheet stays, and focus is back on
/// `opener`. Opened again, `filter` narrows it to one offer and Enter
/// takes it, closing it with focus back on `opener`.
fn pick_by_keyboard(walk: &Walk, opener: &'static str, filter: &str, cx: &mut TestAppContext) {
    walk.tab_to("tab", opener, cx);
    walk.on(opener, Region::Content, cx);
    walk.enter(cx);
    assert!(picker_open(walk, cx), "{opener}: Enter opened the picker");
    walk.type_text("zz", cx);
    walk.keys("escape", cx);
    assert!(!walk.shell.present("rel-picker", cx), "{opener}: Escape");
    assert!(has_sheet(walk, cx), "{opener}: the sheet stays");
    walk.on(opener, Region::Content, cx);

    walk.enter(cx);
    assert!(picker_open(walk, cx), "{opener}: opened again");
    walk.type_text(filter, cx);
    assert_eq!(
        walk.shell.rows("rel-pick", cx).len(),
        1,
        "{opener}: {filter}"
    );
    walk.keys("enter", cx);
    walk.shell.frame(cx, |_, _| ());
    assert!(!walk.shell.present("rel-picker", cx), "{opener}: taken");
    walk.on(opener, Region::Content, cx);
}

/// While editing, the details sheet by Tab and Enter from All details:
/// Tab walks its edit controls, typing edits the Title and Project; both
/// pickers, Waiting on's Add… and Parent's Change…, are opened by Enter,
/// closed by Escape and used by typing and Enter, focus back on their
/// opener each time; Escape closes the sheet, focus back on All details.
/// Then a save over their change, by `secondary-s`, opens the review:
/// Tab reaches its controls and its body editor, whose Escape focuses
/// Done; the fields after that editor, which keeps Tab, are reached from
/// below by Shift-Tab; Keep mine and Use these choices by Enter end it,
/// and `secondary-s` saves.
///
/// Pinned here: a review control that takes focus is scrolled into view.
/// Its Kit buttons' focus is private, so each line of them is revealed
/// through a wrapper (`edit::review_view`).
#[gpui_kit::test]
async fn the_sheet_pickers_and_review_work_by_keyboard_alone(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let walk = related(cx);
    walk.select_a_repository(cx);
    let host = edit_by_e(&walk, REVIEWED, cx).await;
    walk.type_text("x", cx);
    let mine = text(&host, cx);

    // The sheet, from All details: it opens on the Title's control; Tab
    // walks the Type, Status and Project controls; typing edits two.
    walk.escape_to_done(cx);
    walk.tab_to("tab", "chip-all", cx);
    walk.enter(cx);
    assert!(has_sheet(&walk, cx));
    let control = |field: Field, walk: &Walk, cx: &mut TestAppContext| {
        let input = walk
            .pane(cx)
            .read_with(cx, |pane, _| pane.sheet_input(field))
            .unwrap_or_else(|| panic!("the {field:?} control"));
        let focus = input.read_with(cx, |input, cx| input.focus_handle(cx));
        walk.on_handle(&focus, Region::Content, cx);
        input
    };
    let title = control(Field::Title, &walk, cx);
    walk.keys("end", cx);
    walk.type_text(" mine", cx);
    assert_eq!(
        title.read_with(cx, |input, _| input.value().to_string()),
        "Reviewed mine"
    );
    for field in [Field::Type, Field::Status] {
        walk.keys("tab", cx);
        control(field, &walk, cx);
    }
    walk.keys("tab", cx);
    let project = control(Field::Project, &walk, cx);
    walk.type_text("p", cx);
    assert_eq!(
        project.read_with(cx, |input, _| input.value().to_string()),
        "p"
    );
    walk.keys("tab", cx);
    control(Field::Team, &walk, cx);

    // Both pickers.
    pick_by_keyboard(&walk, "rel-parent-change", "New", cx);
    assert_eq!(
        walk.shell.label("rel-parent-id", cx).as_deref(),
        Some(NEW_PARENT)
    );
    pick_by_keyboard(&walk, "rel-deps-add", "kb-d3p00", cx);
    assert_eq!(walk.shell.rows("rel-dep-id", cx), [DEPENDENCY]);

    // Every stop of the editing sheet, round to where Tab began, is named
    // and on screen; Parent's Clear and the dependency's Remove among them.
    let stops = sheet_stops(&walk, cx);
    for id in [
        ElementId::from("rel-parent-change"),
        "rel-parent-clear".into(),
        ("rel-dep-remove", 0usize).into(),
        "rel-deps-add".into(),
        "sheet-done".into(),
        "close".into(),
    ] {
        assert!(stops.contains(&id), "Tab reached {id:?}: {stops:?}");
    }

    // Clear and Remove, each by Enter: they take effect, and focus stays
    // in the sheet.
    walk.tab_to("tab", "rel-parent-clear", cx);
    walk.enter(cx);
    assert!(
        walk.shell.present("rel-parent-none", cx),
        "Clear cleared it"
    );
    assert!(!walk.shell.present("rel-parent-clear", cx));
    walk.tab_to("tab", ("rel-dep-remove", 0usize), cx);
    walk.enter(cx);
    assert!(
        walk.shell.rows("rel-dep-id", cx).is_empty(),
        "Remove removed it"
    );
    assert!(walk.shell.present("rel-deps-none", cx));
    assert!(has_sheet(&walk, cx));
    assert!(
        walk.focused_in("sheet-content", cx).is_some(),
        "focus stays"
    );
    walk.region(Region::Content, cx);

    // Escape closes the sheet; focus is back on All details.
    walk.keys("escape", cx);
    assert!(!has_sheet(&walk, cx));
    walk.on("chip-all", Region::Content, cx);

    // Their change, then `secondary-s`: the review.
    walk.shell.fixture.save_ticket(&walk.a, REVIEWED, "Theirs");
    assert_eq!(walk.save_by_key(cx).await, Some(ResultCode::ExternalChange));
    walk.shell
        .until_drawn(cx, "the review opened", |shell, cx| {
            shell.present("review-use", cx)
        })
        .await;
    assert_eq!(walk.phase(cx), Phase::Review);

    // The review's controls by Tab, the body's first: Cancel, then the
    // body's Take theirs and Keep mine; Use these choices waits for the
    // Title's choice.
    let row = |name: &str, walk: &Walk, cx: &mut TestAppContext| {
        walk.shell
            .rows("review-field", cx)
            .iter()
            .position(|field| field == name)
            .unwrap_or_else(|| panic!("the review lists {name}"))
    };
    let (body, title) = (row("Body", &walk, cx), row("Title", &walk, cx));
    walk.step("tab", "review-cancel", Region::Content, cx);
    walk.step("tab", ("review-take-theirs", body), Region::Content, cx);
    walk.step("tab", ("review-keep-mine", body), Region::Content, cx);

    // Then the body's result, in its own editor, which keeps Tab; its
    // Escape focuses Done and cancels nothing.
    walk.keys("tab", cx);
    let result = walk
        .pane(cx)
        .read_with(cx, |pane, _| pane.review_host())
        .expect("the body's result");
    walk.in_editor(&result, cx);
    walk.type_text("y", cx);
    let edited = text(&result, cx);
    assert!(edited != mine, "the result is edited");
    walk.escape_to_done(cx);
    assert_eq!(walk.phase(cx), Phase::Review, "nothing is cancelled");

    // Take theirs on the body, by Enter, asks first, the result being
    // edited: Escape closes it, focus is back on Take theirs, and the edit
    // is kept.
    walk.tab_to("tab", ("review-take-theirs", body), cx);
    walk.enter(cx);
    walk.dialog_open("review-replace-dialog", cx);
    walk.keys("escape", cx);
    assert!(!walk.shell.present("review-replace-dialog", cx), "Escape");
    walk.on(("review-take-theirs", body), Region::Content, cx);
    assert!(text(&result, cx) == edited, "the edit is kept");

    // The fields after the body's editor are reached from below: the
    // discussion's key, Tab to Back to top, then Shift-Tab. Keep mine for
    // the Title, by Enter.
    walk.keys("secondary-shift-c", cx);
    walk.on("discussion-heading", Region::Content, cx);
    walk.step("tab", "back-to-top", Region::Content, cx);
    walk.tab_to("shift-tab", ("review-keep-mine", title), cx);
    walk.on(("review-keep-mine", title), Region::Content, cx);
    walk.enter(cx);

    // Use these choices, by Tab from Done and Enter; `secondary-s` saves.
    walk.keys("secondary-3", cx);
    walk.tab_to("tab", "item-done", cx);
    walk.tab_to("tab", "review-use", cx);
    walk.on("review-use", Region::Content, cx);
    walk.enter(cx);
    walk.shell
        .until_drawn(cx, "the review ended", |shell, cx| {
            !shell.present("item-review", cx)
        })
        .await;
    assert_eq!(walk.phase(cx), Phase::Editing);
    assert!(text(&host, cx) == edited, "the result, edited, is used");
    walk.tab_to("tab", "item-page", cx);
    walk.in_editor(&host, cx);
    assert_eq!(walk.save_by_key(cx).await, Some(ResultCode::Ok));
}

const LISTED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX6";
const PLAN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX7";

/// A window over `a`: the document Plan at the top of `docs`, the folder
/// `docs/archive`, and the ticket Listed.
fn listed(cx: &mut TestAppContext) -> Walk {
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    fixture.create_document(&a, PLAN, "docs/plan.md", "Plan");
    fixture.create_folder(&a, "docs/archive");
    fixture.create_ticket(&a, LISTED, "Listed", "open");
    fixture.refresh(&a);
    Walk::over(Shell::open(fixture, cx), a)
}

/// The element the window's focus is on.
fn focused_handle(walk: &Walk, cx: &mut TestAppContext) -> Option<gpui_kit::FocusHandle> {
    walk.shell.frame(cx, |window, cx| window.focused(cx))
}

/// The create dialog is open, holding focus, its Title focused.
fn create_dialog_open(walk: &Walk, cx: &mut TestAppContext) {
    walk.dialog_open("create-dialog", cx);
    assert!(walk.focus_inside("create-title", cx), "focus in the Title");
}

/// Create by keyboard alone: `secondary-n` in the document tree opens New
/// document…, and Escape gives focus back to the tree; New document…, by
/// Shift-Tab and Enter, is filled by typing and Tab and created by Enter
/// on Create, and `secondary-s` creates the document. The same for New
/// ticket…, from the ticket list and its button.
///
/// Pinned here: Create, pressed from its own button by key, leaves focus
/// in the dialog as the button disables, so the dialog closes itself once
/// the item is made (`create::form::keep_focus_in`).
#[gpui_kit::test]
async fn create_by_keyboard_alone(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let walk = listed(cx);
    walk.select_a_repository(cx);

    for (pane, kind) in [("pane-documents", "document"), ("pane-tickets", "ticket")] {
        let button: &'static str = if kind == "document" {
            "new-document-button"
        } else {
            "new-ticket-button"
        };
        // `secondary-n` from the list; Escape gives focus back to it.
        walk.keys("secondary-2", cx);
        walk.choose(pane, cx);
        walk.keys("secondary-2", cx);
        walk.region(Region::Navigation, cx);
        let list = focused_handle(&walk, cx);
        walk.keys("secondary-n", cx);
        create_dialog_open(&walk, cx);
        assert_eq!(walk.shell.present("create-type", cx), kind == "ticket");
        walk.keys("escape", cx);
        assert!(!walk.shell.present("create-dialog", cx), "{kind}: Escape");
        assert_eq!(focused_handle(&walk, cx), list, "{kind}: back to the list");
        walk.region(Region::Navigation, cx);

        // Its button, by Tab and Enter; Escape gives focus back to it.
        walk.tab_to("shift-tab", button, cx);
        walk.on(button, Region::Navigation, cx);
        walk.enter(cx);
        create_dialog_open(&walk, cx);
        walk.keys("escape", cx);
        assert!(!walk.shell.present("create-dialog", cx));
        walk.on(button, Region::Navigation, cx);

        // Filled and created by key.
        walk.enter(cx);
        create_dialog_open(&walk, cx);
        walk.type_text(&format!("Made {kind}"), cx);
        if kind == "document" {
            walk.tab_into("tab", "create-file", cx);
            walk.keys("secondary-a", cx);
            walk.type_text("made.md", cx);
        }
        walk.shell
            .until_drawn(cx, "Create offered", |shell, cx| {
                !shell.present("create-ok-note", cx)
            })
            .await;
        walk.tab_into("tab", "create-ok", cx);
        walk.enter(cx);
        let host = walk.shell.until_editing(cx).await;
        walk.shell
            .until_drawn(cx, "the create dialog closing", |shell, cx| {
                !shell.present("create-dialog", cx)
            })
            .await;
        walk.in_editor(&host, cx);
        walk.type_text("Made.", cx);
        walk.keys("secondary-s", cx);
        let made = walk
            .ended(
                |intent| {
                    matches!(
                        intent,
                        Intent::DocumentCreate { .. } | Intent::TicketCreate { .. }
                    )
                },
                cx,
            )
            .await;
        assert_eq!(made.map(|(_, code)| code), Some(ResultCode::Ok), "{kind}");
        walk.in_editor(&host, cx);
    }
}

/// Move… and New folder… by keyboard alone. Move…, by Tab and Enter in the
/// document's sheet, opens with focus in its Folder; Escape closes it and
/// gives focus back to Move…; opened again, a folder typed and Move, by Tab
/// and Enter, moves the document, the tab follows, and focus is back on
/// Move…. New folder…, by Shift-Tab and Enter from the tree, opens with
/// focus in its name; Escape gives focus back to it; opened again, a name
/// typed and Create, by Tab and Enter, makes the folder, which the tree
/// lists, and focus is back on New folder….
///
/// Pinned here: Move, and New folder's Create, pressed from their own
/// button by key, leave focus in the dialog as the button disables, so the
/// dialog closes itself once done (`create::form::keep_focus_in`).
#[gpui_kit::test]
async fn move_and_new_folder_by_keyboard_alone(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let walk = listed(cx);
    walk.select_a_repository(cx);
    walk.open_document("Plan", cx);
    walk.keys("secondary-3", cx);
    walk.tab_to("tab", "chip-all", cx);
    walk.enter(cx);
    assert!(has_sheet(&walk, cx));
    walk.tab_to("tab", "sheet-move-button", cx);
    walk.on("sheet-move-button", Region::Content, cx);

    // Escape.
    walk.enter(cx);
    walk.dialog_open("move-dialog", cx);
    assert!(walk.focus_inside("move-folder", cx), "focus in the Folder");
    walk.keys("escape", cx);
    assert!(!walk.shell.present("move-dialog", cx), "Escape closed it");
    assert!(has_sheet(&walk, cx), "the sheet stays");
    walk.on("sheet-move-button", Region::Content, cx);

    // Moved, by key.
    walk.enter(cx);
    walk.dialog_open("move-dialog", cx);
    walk.keys("secondary-a", cx);
    walk.type_text("docs/archive", cx);
    walk.shell
        .until_drawn(cx, "Move offered", |shell, cx| {
            !shell.present("move-ok-note", cx)
        })
        .await;
    walk.tab_into("tab", "move-ok", cx);
    walk.enter(cx);
    walk.shell
        .until_drawn(cx, "the move dialog closing", |shell, cx| {
            !shell.present("move-dialog", cx)
        })
        .await;
    assert_eq!(
        walk.ended(|intent| matches!(intent, Intent::DocumentMove { .. }), cx)
            .await
            .map(|(_, code)| code),
        Some(ResultCode::Ok)
    );
    walk.shell
        .until_drawn(cx, "the tab following", |shell, cx| {
            shell.label("item-folder", cx).as_deref() == Some("docs/archive")
        })
        .await;
    assert!(has_sheet(&walk, cx), "the sheet stays");
    walk.on("sheet-move-button", Region::Content, cx);
    walk.keys("escape", cx);
    assert!(!has_sheet(&walk, cx));
    walk.on("chip-all", Region::Content, cx);

    // New folder…: Escape.
    walk.keys("secondary-2", cx);
    walk.region(Region::Navigation, cx);
    walk.tab_to("shift-tab", "new-folder-button", cx);
    walk.on("new-folder-button", Region::Navigation, cx);
    walk.enter(cx);
    walk.dialog_open("new-folder-dialog", cx);
    assert!(
        walk.focus_inside("new-folder-name", cx),
        "focus in the name"
    );
    walk.keys("escape", cx);
    assert!(
        !walk.shell.present("new-folder-dialog", cx),
        "Escape closed it"
    );
    walk.on("new-folder-button", Region::Navigation, cx);

    // Made, by key.
    walk.enter(cx);
    walk.dialog_open("new-folder-dialog", cx);
    walk.type_text("notes", cx);
    walk.tab_into("tab", "new-folder-ok", cx);
    walk.enter(cx);
    walk.shell
        .until_drawn(cx, "the new folder dialog closing", |shell, cx| {
            !shell.present("new-folder-dialog", cx)
        })
        .await;
    assert_eq!(
        walk.ended(|intent| matches!(intent, Intent::FolderCreate { .. }), cx)
            .await
            .map(|(_, code)| code),
        Some(ResultCode::Ok)
    );
    walk.shell
        .until_drawn(cx, "the tree listing it", |shell, cx| {
            shell.rows("doc-node", cx).contains(&"notes".to_owned())
        })
        .await;
    walk.on("new-folder-button", Region::Navigation, cx);
}

const ADOPTED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX8";
const READABLE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX9";
const MARKER: &str = "docs/marker.md";
const NOTES: &str = "docs/notes.md";

/// A window over `a`: a marker-only document, which needs fixing; a file
/// under `docs/` with no front matter, unmanaged; and a conforming
/// document.
fn repairable(cx: &mut TestAppContext) -> Walk {
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    fixture.commit_nonconforming_document(&a, MARKER);
    fixture.commit_file(&a, NOTES, b"Intro.\n\n## Notes\n\nWritten by hand.\n");
    fixture.create_document(&a, READABLE, "docs/readable.md", "Readable");
    fixture.refresh(&a);
    Walk::over(Shell::open(fixture, cx), a)
}

/// The repair view's text area holds focus.
fn in_repair_text(walk: &Walk, cx: &mut TestAppContext) {
    assert!(walk.focus_inside("repair-source", cx), "in the text area");
    walk.region(Region::Content, cx);
}

/// Preview, focused, by Enter: the one confirmation opens on the repair's
/// preview.
async fn preview_by_enter(walk: &Walk, cx: &mut TestAppContext) {
    walk.on("repair-preview-button", Region::Content, cx);
    walk.enter(cx);
    walk.shell
        .until_drawn(cx, "the preview", |shell, cx| {
            shell.present("confirm-ok", cx) && shell.present("confirm-expires", cx)
        })
        .await;
    walk.dialog_open("confirm-dialog", cx);
}

/// Repair… and Adopt… by keyboard alone. The marker-only document, opened
/// from the tree: Repair… in "Needs fixing", by Tab and Enter, opens the
/// view with focus in its text area, where its front matter is typed;
/// Tab leaves it for Preview and Shift-Tab comes back; Preview by Enter
/// opens the one confirmation, Escape gives focus back to the text area,
/// Tab reaches Preview again, and Enter in the confirmation confirms: the
/// tab follows the repaired document. The unmanaged file: Adopt…, by Tab
/// and Enter, opens the view prefilled; typed in, closing the tab asks
/// "Discard your repair?", whose Enter is Cancel and whose Escape is too,
/// focus back in the text; Cancel by Tab and Enter asks the same, and its
/// Discard, by Tab and Enter, closes the view, focus in the file it shows
/// again, its opener gone with it. A conforming document's Inspect tab,
/// from its sheet's link by Tab and Enter, offers no Repair….
#[gpui_kit::test]
async fn repair_and_adopt_by_keyboard_alone(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let walk = repairable(cx);
    walk.select_a_repository(cx);

    // Repair…, from "Needs fixing".
    walk.open_document(MARKER, cx);
    walk.keys("secondary-3", cx);
    walk.tab_to("tab", "item-repair-button", cx);
    walk.on("item-repair-button", Region::Content, cx);
    walk.enter(cx);
    in_repair_text(&walk, cx);
    walk.keys("down down", cx);
    walk.type_text(
        &format!("manyhands_kind: document\nid: {ADOPTED}\ntitle: Adopted\n"),
        cx,
    );
    walk.step("tab", "repair-preview-button", Region::Content, cx);
    walk.keys("shift-tab", cx);
    in_repair_text(&walk, cx);
    walk.step("tab", "repair-preview-button", Region::Content, cx);

    // The confirmation: Escape back to the text (Part E review, M2), Tab
    // to Preview again, then Enter confirms.
    preview_by_enter(&walk, cx).await;
    walk.keys("escape", cx);
    assert!(
        !walk.shell.present("confirm-dialog", cx),
        "Escape closed it"
    );
    in_repair_text(&walk, cx);
    walk.step("tab", "repair-preview-button", Region::Content, cx);
    preview_by_enter(&walk, cx).await;
    walk.keys("enter", cx);
    assert_eq!(
        walk.ended(|intent| matches!(intent, Intent::DocumentRepair { .. }), cx)
            .await
            .map(|(_, code)| code),
        Some(ResultCode::Ok)
    );
    walk.shell
        .until_drawn(
            cx,
            "the tab following the repaired document",
            |shell, cx| shell.label("item-title", cx).as_deref() == Some("Adopted"),
        )
        .await;
    assert!(!walk.shell.present("repair-source", cx));
    walk.region(Region::Content, cx);

    // Adopt…, and the close prompt.
    walk.open_document(NOTES, cx);
    walk.keys("secondary-3", cx);
    let shown = walk.shown_tab(cx);
    walk.tab_to("tab", "item-repair-button", cx);
    walk.on("item-repair-button", Region::Content, cx);
    walk.enter(cx);
    walk.shell
        .until_drawn(cx, "the adoption's view", |shell, cx| {
            shell.present("repair-source", cx)
        })
        .await;
    in_repair_text(&walk, cx);
    let prefilled = walk
        .pane(cx)
        .read_with(cx, |pane, _| pane.repair_state())
        .expect("the repair view")
        .read_with(cx, |state, _| state.value().to_string());
    assert!(
        prefilled.contains("manyhands_kind: document") && prefilled.ends_with("Written by hand.\n"),
        "front matter prefilled over the file"
    );
    walk.type_text("x", cx);
    for key in ["enter", "escape"] {
        walk.keys("secondary-w", cx);
        walk.dialog_open("repair-discard-dialog", cx);
        walk.keys(key, cx);
        assert!(!walk.shell.present("repair-discard-dialog", cx), "{key}");
        assert_eq!(walk.shown_tab(cx), shown, "{key} keeps the tab");
        in_repair_text(&walk, cx);
    }
    walk.tab_into("tab", "repair-cancel", cx);
    walk.enter(cx);
    walk.dialog_open("repair-discard-dialog", cx);
    walk.tab_into("tab", "repair-discard-discard", cx);
    walk.enter(cx);
    assert!(!walk.shell.present("repair-discard-dialog", cx));
    assert!(!walk.shell.present("repair-source", cx), "the view closed");
    assert!(walk.shell.present("item-source", cx), "the file is shown");
    assert!(
        walk.focus_inside("item-source", cx),
        "focus in the file shown"
    );
    walk.region(Region::Content, cx);

    // A conforming file's Inspect tab offers no Repair….
    walk.open_document("Readable", cx);
    walk.keys("secondary-3", cx);
    walk.tab_to("tab", "chip-all", cx);
    walk.enter(cx);
    walk.tab_to("tab", "sheet-inspect", cx);
    walk.enter(cx);
    walk.shell
        .until_drawn(cx, "the Inspect tab", |shell, cx| {
            shell.present("item-source", cx)
        })
        .await;
    assert!(!walk.shell.present("item-needs-fixing", cx));
    assert!(!walk.shell.present("item-repair-button", cx), "no Repair…");
}

/// A window over `a` holding `EDITED`, whose draft store did not open: its
/// folder is a link, as `edit_done_tests` makes it. The link's target goes
/// with the last value.
#[cfg(unix)]
fn without_a_store(cx: &mut TestAppContext) -> (Walk, tempfile::TempDir) {
    let fixture = Fixture::new();
    let elsewhere = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(elsewhere.path(), fixture.data_dir().join("drafts")).unwrap();
    let a = fixture.create_repository("a");
    let source = ticket_with_body(EDITED, "Ticket 0", BODY);
    fixture.commit_file(&a, &ticket_path(EDITED), source.as_bytes());
    fixture.refresh(&a);
    (Walk::over(Shell::open(fixture, cx), a), elsewhere)
}

/// With crash recovery off, Edit by Tab and Enter asks first: Escape
/// closes the dialog, nothing prepared, focus back on Edit; Enter on Edit
/// again, then Enter, its default Continue, goes on into the editor.
#[cfg(unix)]
#[gpui_kit::test]
async fn crash_recovery_off_asks_first_by_keyboard_alone(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (walk, _elsewhere) = without_a_store(cx);
    walk.select_a_repository(cx);
    walk.find(EDITED, cx);
    walk.keys("secondary-3", cx);
    walk.tab_to("tab", "item-edit", cx);
    walk.on("item-edit", Region::Content, cx);
    let prepares = |walk: &Walk, cx: &mut TestAppContext| {
        walk.shell
            .requests(cx, |request| {
                matches!(request.intent, Intent::ContextPrepare { .. })
            })
            .len()
    };

    walk.enter(cx);
    walk.dialog_open("recovery-off-dialog", cx);
    walk.keys("escape", cx);
    assert!(!walk.shell.present("recovery-off-dialog", cx), "Escape");
    walk.on("item-edit", Region::Content, cx);
    assert_eq!(prepares(&walk, cx), 0, "nothing is prepared");
    assert_eq!(walk.phase(cx), Phase::Viewing);

    walk.enter(cx);
    walk.dialog_open("recovery-off-dialog", cx);
    walk.enter(cx);
    let host = walk.shell.until_editing(cx).await;
    assert!(!walk.shell.present("recovery-off-dialog", cx));
    assert_eq!(prepares(&walk, cx), 1, "Continue prepared once");
    walk.in_editor(&host, cx);
}

/// Pinned (fix round 1, M4): a keyed Create that is refused, crash
/// recovery being off, leaves the dialog open saying why, with focus on
/// Create, ringed, where Enter presses it again.
#[cfg(unix)]
#[gpui_kit::test]
async fn a_refused_create_by_key_leaves_focus_on_create(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (walk, _elsewhere) = without_a_store(cx);
    walk.select_a_repository(cx);
    walk.keys("secondary-2", cx);
    walk.choose("pane-tickets", cx);
    walk.keys("secondary-2", cx);
    walk.keys("secondary-n", cx);
    create_dialog_open(&walk, cx);
    walk.type_text("No store", cx);
    walk.tab_into("tab", "create-ok", cx);
    let ids = |walk: &Walk| {
        walk.shell
            .backend
            .reads_seen(crate::desktop::services::ReadKind::NewId)
    };
    for press in 1..=2 {
        walk.enter(cx);
        walk.shell
            .until_drawn(cx, "the refusal", |shell, _| {
                shell
                    .backend
                    .reads_seen(crate::desktop::services::ReadKind::NewId)
                    == press
            })
            .await;
        walk.shell
            .until_drawn(cx, "the refusal's words", |shell, cx| {
                shell.present("create-failure", cx)
            })
            .await;
        walk.dialog_open("create-dialog", cx);
        assert_eq!(
            walk.focused_in("create-dialog", cx),
            Some((ElementId::from("create-ok"), true)),
            "press {press}: focus is on Create"
        );
        assert!(
            walk.shell.frame(cx, |window, _| {
                window
                    .try_find("create-ok-focus-ring")
                    .is_some_and(|ring| ring.visible())
            }),
            "press {press}: Create is ringed"
        );
    }
    assert_eq!(ids(&walk), 2, "Enter pressed Create again");
}

/// Pinned (fix round 1, M4): a keyed Move whose destination another commits
/// after it was checked free is refused under the File name, the dialog
/// stays, and focus goes to the File name.
#[gpui_kit::test]
async fn a_refused_move_by_key_puts_focus_in_the_field_with_the_note(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    let source = format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: {PLAN}\ntitle: Plan\n---\n\
         # Plan\n"
    );
    fixture.commit_file(&a, "docs/plan.md", source.as_bytes());
    fixture.create_folder(&a, "docs/archive");
    fixture.refresh(&a);
    let walk = Walk::over(Shell::open(fixture, cx), a);
    walk.select_a_repository(cx);
    walk.open_document("Plan", cx);
    walk.keys("secondary-3", cx);
    walk.tab_to("tab", "chip-all", cx);
    walk.enter(cx);
    walk.tab_to("tab", "sheet-move-button", cx);
    walk.enter(cx);
    walk.dialog_open("move-dialog", cx);
    walk.keys("secondary-a", cx);
    walk.type_text("docs/archive", cx);
    walk.shell
        .until_drawn(cx, "Move offered", |shell, cx| {
            !shell.present("move-ok-note", cx)
        })
        .await;
    walk.shell
        .fixture
        .commit_file(&walk.a, "docs/archive/plan.md", b"Taken.\n");
    walk.tab_into("tab", "move-ok", cx);
    walk.enter(cx);
    walk.shell
        .until_drawn(cx, "the collision's words", |shell, cx| {
            shell.label("move-file-note", cx).is_some()
        })
        .await;
    walk.dialog_open("move-dialog", cx);
    assert!(
        walk.focus_inside("move-file", cx),
        "focus is in the File name"
    );
    let (_, labeled) = walk
        .focused_in("move-dialog", cx)
        .expect("focus is in the dialog");
    assert!(labeled, "on a named control");
}
