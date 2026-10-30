//! The details sheet's named controls, over real repositories (design,
//! Named controls in the details sheet; R-27, R-38, R-57, R-74; decision
//! 14): each control edits its field and marks the tab dirty as typing
//! does, and putting it back leaves the tab clean (contract 4); a chip
//! opens the sheet on its control; the suggestions are the loaded tickets'
//! values, and any typed value is kept; an empty required field says why
//! and blocks Save; what is never edited stays so; `closed` is ordinary
//! text; an emptied Project is saved as absent; `secondary-s` in the sheet
//! saves the tab, but not during the review; the controls are read only
//! while preparing, loading and in the review; and the sheet's body is a
//! `draft-section` while editing.
//!
//! The helpers are `edit_tests`'s. Each test opens the sheet, so the
//! control holds focus and its frame is drawn, before typing, and types
//! through the window; each waits on the worker and allows parking. Draft
//! fields would print text through `Debug`, so they are compared with
//! `assert!`.

use gpui_kit::{ElementId, component::WindowExt, component::input::InputState};
use manyhands::repository::ClosureState;

use super::*;
use crate::desktop::{
    model::edit::fields::{
        DOCUMENT_NEEDS_TITLE, Field, TICKET_NEEDS_STATUS, TICKET_NEEDS_TITLE, TICKET_NEEDS_TYPE,
        value,
    },
    test_support::drafts::{Actor, second_actor},
    view::{item::sheet::NOT_CLOSED, tickets::LIFECYCLE_OPEN},
};

const SENTINEL: &str = "SENTINEL-D3-SHEET-7d21";
const EDITING_TICKET: &str =
    "Changes here become part of your draft and are saved with the ticket.";
const EDITING_DOCUMENT: &str =
    "Changes here become part of your draft and are saved with the document.";
const SAVED_WITH_TICKET: &str = "Saved with the ticket (Ctrl S)";
const SAVED_WITH_DOCUMENT: &str = "Saved with the document (Ctrl S)";
const VIEWING: &str = "Viewing the shared copy. Choose Edit to change these.";
const READ_ONLY: &str = "Read only while viewing";

/// A ticket's source with its own type and status, and `extra` front
/// matter lines.
fn ticket_as(id: &str, title: &str, ticket_type: &str, status: &str, extra: &str) -> String {
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {id}\ntitle: {title}\n\
         type: {ticket_type}\nstatus: {status}\n{extra}---\nBody of {title}.\n"
    )
}

/// A managed document's source at `id`, with `extra` front matter lines.
fn document_source(id: &str, title: &str, extra: &str) -> String {
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: {id}\ntitle: {title}\n\
         {extra}---\nBody of {title}.\n"
    )
}

fn document(root: &Path) -> OpenTarget {
    OpenTarget::Item {
        repo: key(root),
        kind: ItemDtoKind::Document,
        id: item_id(DOCUMENT),
    }
}

/// The sheet's control of `field`, once the sheet has made it.
fn input(shell: &Shell, field: Field, cx: &mut TestAppContext) -> Option<Entity<InputState>> {
    pane(shell, cx).read_with(cx, |pane, _| pane.sheet_input(field))
}

/// What the control of `field` holds.
fn holds(shell: &Shell, field: Field, cx: &mut TestAppContext) -> String {
    let input = input(shell, field, cx).expect("the control is made");
    input.read_with(cx, |input, _| input.value().to_string())
}

/// Whether the control of `field` holds focus.
fn control_focused(shell: &Shell, field: Field, cx: &mut TestAppContext) -> bool {
    let Some(input) = input(shell, field, cx) else {
        return false;
    };
    let handle = input.read_with(cx, |input, cx| input.focus_handle(cx));
    shell.frame(cx, move |window, _| handle.is_focused(window))
}

/// Whether a details sheet is open in the window.
fn has_sheet(shell: &Shell, cx: &mut TestAppContext) -> bool {
    shell.frame(cx, |window, cx| window.has_active_sheet(cx))
}

/// Whether the element `id` is drawn inside `within`.
fn inside(
    shell: &Shell,
    id: impl Into<ElementId>,
    within: &'static str,
    cx: &mut TestAppContext,
) -> bool {
    let id = id.into();
    shell.frame(cx, |window, _| {
        window
            .try_find(id)
            .is_some_and(|element| element.path().contains(&ElementId::from(within)))
    })
}

/// Opens the sheet from `chip`'s click, then draws the frames that paint
/// its focused control.
fn open_sheet(shell: &Shell, chip: ChipField, cx: &mut TestAppContext) {
    shell.click(chip.id(), cx);
    assert!(has_sheet(shell, cx), "{chip:?} opens the sheet");
    shell.frame(cx, |_, _| ());
}

/// Types `typed` where focus is, through the window.
fn type_in(shell: &Shell, typed: &str, cx: &mut TestAppContext) {
    shell.visual(cx).simulate_input(typed);
    shell.run(cx);
    shell.frame(cx, |_, _| ());
}

/// Empties the focused control.
fn empty_it(shell: &Shell, cx: &mut TestAppContext) {
    shell.keys("secondary-a backspace", cx);
    shell.frame(cx, |_, _| ());
}

/// Opens the sheet on `chip`, puts `typed` in its control in place of what
/// it held, and closes the sheet.
fn set(shell: &Shell, chip: ChipField, typed: &str, cx: &mut TestAppContext) {
    open_sheet(shell, chip, cx);
    empty_it(shell, cx);
    if !typed.is_empty() {
        type_in(shell, typed, cx);
    }
    shell.keys("escape", cx);
}

/// The shown session's fields besides the body.
fn meta(shell: &Shell, cx: &mut TestAppContext) -> DraftFields {
    pane(shell, cx).read_with(cx, |pane, _| {
        pane.session_meta().cloned().expect("a session is shown")
    })
}

/// Whether the session is dirty against its saved copy.
fn dirty(shell: &Shell, cx: &mut TestAppContext) -> bool {
    pane(shell, cx).read_with(cx, |pane, cx| pane.edit_dirty(cx))
}

fn phase(shell: &Shell, cx: &mut TestAppContext) -> Phase {
    pane(shell, cx).read_with(cx, |pane, _| pane.edit_phase())
}

/// The save requests of this session, newest first.
fn saves(shell: &Shell, cx: &mut TestAppContext) -> Vec<DesktopRequestId> {
    shell.requests(cx, |request| {
        matches!(
            request.intent,
            Intent::TicketSave { .. } | Intent::DocumentSave { .. }
        )
    })
}

/// Presses the header's Save, then waits until the save it sent has
/// ended and what follows it landed.
async fn save(shell: &Shell, cx: &mut TestAppContext) -> DesktopRequestId {
    let before = saves(shell, cx).len();
    shell.click("item-save", cx);
    let sent = saves(shell, cx);
    assert_eq!(sent.len(), before + 1, "Save sent one save");
    ended(shell, sent[0], cx).await;
    sent[0]
}

/// The committed bytes of `root`'s ticket `id` at the tip of `branch`.
fn committed(root: &Path, branch: &str, id: &str) -> String {
    let repository = git2::Repository::open(root).unwrap();
    let tree = repository
        .revparse_single(&format!("refs/heads/{branch}"))
        .unwrap()
        .peel_to_tree()
        .unwrap();
    let entry = tree.get_path(Path::new(&ticket_path(id))).unwrap();
    let blob = repository.find_blob(entry.id()).unwrap();
    String::from_utf8(blob.content().to_vec()).unwrap()
}

/// Every label the window draws outside a draft's own section.
fn drawn_outside_the_section(shell: &Shell, cx: &mut TestAppContext) -> Vec<String> {
    let section = ElementId::from("draft-section");
    shell.frame(cx, |window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .iter()
            .filter(|snapshot| !snapshot.path().contains(&section))
            .filter_map(|snapshot| snapshot.label().map(str::to_owned))
            .collect()
    })
}

/// Draws frames, with the worker and the reads running, until the review
/// shows its choices.
async fn until_review(shell: &Shell, cx: &mut TestAppContext) {
    let deadline = Instant::now() + WAIT;
    loop {
        shell.frame(cx, |_, _| ());
        shell.run(cx);
        if shell.present("review-use", cx) {
            return;
        }
        assert!(Instant::now() < deadline, "the review did not open");
        cx.executor().timer(Duration::from_millis(10)).await;
    }
}

/// The ticket's title changed in the sheet, then someone else saves it,
/// and Save opens the review on their change.
async fn into_review(shell: &Shell, root: &Path, cx: &mut TestAppContext) {
    set(shell, ChipField::All, "My title", cx);
    second_actor(
        &shell.fixture,
        Actor::SaveTicket {
            root,
            id: TICKET,
            title: "Their title",
            status: "open",
        },
    );
    let request = save(shell, cx).await;
    assert_eq!(code(shell, request, cx), Some(ResultCode::ExternalChange));
    until_review(shell, cx).await;
    assert_eq!(phase(shell, cx), Phase::Review, "the review opens");
}

/// The review's value of the field `name` on `side`.
fn review_value(
    shell: &Shell,
    side: &'static str,
    name: &str,
    cx: &mut TestAppContext,
) -> Option<String> {
    let ix = shell
        .rows("review-field", cx)
        .iter()
        .position(|field| field == name)?;
    shell.label((side, ix), cx)
}

/// Each named control writes its field into the session, which marks the
/// tab dirty as typing does; put back, the tab is clean again (contract 4).
/// Changed together, the draft holds them and Save writes them. A document
/// names its Title alone. While editing, the sheet says its changes are
/// part of the draft.
#[gpui_kit::test]
async fn each_named_control_edits_its_field_and_marks_the_tab_dirty(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Named", "project: p\nteam: t\n");
        fixture.create_document(root, DOCUMENT, "docs/one.md", "One");
    });
    open(&shell, ticket(&root, TICKET), cx);
    edit(&shell, cx).await;
    let base = meta(&shell, cx);
    for (chip, field, before) in [
        (ChipField::All, Field::Title, "Named"),
        (ChipField::Type, Field::Type, "task"),
        (ChipField::Status, Field::Status, "open"),
        (ChipField::Project, Field::Project, "p"),
        (ChipField::Team, Field::Team, "t"),
    ] {
        open_sheet(&shell, chip, cx);
        assert_eq!(holds(&shell, field, cx), before, "{field:?}");
        type_in(&shell, "x", cx);
        let after = format!("{before}x");
        assert_eq!(holds(&shell, field, cx), after, "{field:?}");
        assert!(
            value(&meta(&shell, cx), field) == Some(after.as_str()),
            "{field:?} is in the session's fields"
        );
        assert!(dirty(&shell, cx), "{field:?} marks the session dirty");
        assert_eq!(
            shell.rows("tab", cx),
            ["Named, unsaved"],
            "{field:?}: the tab's unsaved mark"
        );
        assert_eq!(pill(&shell, cx).as_deref(), Some(EDITING_UNSAVED));
        // Put back, as typing back is: clean, and nothing to save.
        shell.keys("backspace", cx);
        assert!(meta(&shell, cx) == base, "{field:?} put back");
        assert!(!dirty(&shell, cx), "{field:?}: clean again");
        assert_eq!(
            pill(&shell, cx).as_deref(),
            Some(EDITING_SAVED),
            "{field:?}"
        );
        shell.keys("secondary-s", cx);
        assert!(
            saves(&shell, cx).is_empty(),
            "{field:?}: no change, no save"
        );
        shell.keys("escape", cx);
    }

    for (chip, typed) in [
        (ChipField::All, "Renamed"),
        (ChipField::Type, "bug"),
        (ChipField::Status, "doing"),
        (ChipField::Project, "alpha"),
        (ChipField::Team, "core"),
    ] {
        set(&shell, chip, typed, cx);
    }
    open_sheet(&shell, ChipField::All, cx);
    assert_eq!(
        shell.label("sheet-description", cx).as_deref(),
        Some(EDITING_TICKET)
    );
    assert_eq!(
        shell.label("sheet-note", cx).as_deref(),
        Some(SAVED_WITH_TICKET)
    );
    shell.keys("escape", cx);
    let id = attached(&shell, &root, TICKET, cx).expect("the draft began");
    let drafted = registry(&shell, cx).read_with(cx, |registry, _| {
        registry.session(id).unwrap().current().clone()
    });
    assert!(
        matches!(&drafted, DraftFields::Ticket { title, ticket_type, status, project, team, .. }
            if title == "Renamed" && ticket_type == "bug" && status == "doing"
                && project.as_deref() == Some("alpha") && team.as_deref() == Some("core")),
        "the draft holds each field"
    );
    let request = save(&shell, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));
    let item = shown(&shell, &root, TICKET);
    assert_eq!(item.title.as_deref(), Some("Renamed"));
    assert_eq!(item.ticket_type.as_deref(), Some("bug"));
    assert_eq!(item.status.as_deref(), Some("doing"));
    assert_eq!(item.project.as_deref(), Some("alpha"));
    assert_eq!(item.team.as_deref(), Some("core"));
    assert!(!dirty(&shell, cx), "saved");

    // A document's Title.
    open(&shell, document(&root), cx);
    edit(&shell, cx).await;
    open_sheet(&shell, ChipField::All, cx);
    assert!(control_focused(&shell, Field::Title, cx));
    assert!(input(&shell, Field::Type, cx).is_none(), "a title alone");
    assert_eq!(
        shell.label("sheet-description", cx).as_deref(),
        Some(EDITING_DOCUMENT)
    );
    assert_eq!(
        shell.label("sheet-note", cx).as_deref(),
        Some(SAVED_WITH_DOCUMENT)
    );
    type_in(&shell, "x", cx);
    assert!(value(&meta(&shell, cx), Field::Title) == Some("Onex"));
    assert!(dirty(&shell, cx));
    shell.keys("backspace", cx);
    assert!(!dirty(&shell, cx), "put back, clean");
}

/// A chip, by click or by keyboard, opens the sheet with its own control
/// focused, and Escape gives focus back to the chip; Parent and Waiting on,
/// which have no control yet, open on their rows. A document's Title is a
/// control; its Status, Author and Date are rows.
#[gpui_kit::test]
async fn a_chip_opens_the_sheet_focused_on_its_control(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, OTHER, "Parent", "");
        fixture.create_ticket(root, THIRD, "Blocker", "open");
        fixture.commit_ticket(
            root,
            TICKET,
            "Chips",
            &format!("project: p\nteam: t\nparent: {OTHER}\ndeps: [{THIRD}]\n"),
        );
        fixture.commit_file(
            root,
            "docs/kept.md",
            document_source(
                DOCUMENT,
                "Kept",
                "status: draft\nauthor: Ada\ndate: 2026-10-05\n",
            )
            .as_bytes(),
        );
    });
    open(&shell, ticket(&root, TICKET), cx);
    edit(&shell, cx).await;
    for (chip, field) in [
        (ChipField::Type, Field::Type),
        (ChipField::Status, Field::Status),
        (ChipField::Project, Field::Project),
        (ChipField::Team, Field::Team),
        (ChipField::All, Field::Title),
    ] {
        open_sheet(&shell, chip, cx);
        assert!(
            control_focused(&shell, field, cx),
            "{chip:?} focuses {field:?}"
        );
        type_in(&shell, "k", cx);
        assert!(
            holds(&shell, field, cx).ends_with('k'),
            "{field:?} takes it"
        );
        shell.keys("backspace", cx);
        shell.keys("escape", cx);
        assert!(!has_sheet(&shell, cx));
        assert!(holds_focus(&shell, chip.id(), cx), "back to {chip:?}");
    }
    for (chip, row) in [
        (ChipField::Parent, "sheet-parent"),
        (ChipField::WaitingOn, "sheet-waiting-on"),
    ] {
        open_sheet(&shell, chip, cx);
        assert!(holds_focus(&shell, row, cx), "{chip:?} focuses {row}");
        shell.keys("escape", cx);
    }
    assert!(!dirty(&shell, cx));

    // By keyboard: Tab to the Status chip, then Enter.
    shell.keys("secondary-3", cx);
    let mut presses = 0;
    while !holds_focus(&shell, ChipField::Status.id(), cx) {
        presses += 1;
        assert!(presses < 40, "Tab never reached the Status chip");
        shell.keys("tab", cx);
    }
    shell.press("enter", cx);
    shell.frame(cx, |_, _| ());
    assert!(has_sheet(&shell, cx));
    assert!(control_focused(&shell, Field::Status, cx));
    shell.keys("escape", cx);

    open(&shell, document(&root), cx);
    edit(&shell, cx).await;
    open_sheet(&shell, ChipField::All, cx);
    assert!(control_focused(&shell, Field::Title, cx));
    shell.keys("escape", cx);
    for (chip, row) in [
        (ChipField::Status, "sheet-status"),
        (ChipField::Author, "sheet-author"),
        (ChipField::Date, "sheet-date"),
    ] {
        open_sheet(&shell, chip, cx);
        assert!(holds_focus(&shell, row, cx), "{chip:?} focuses {row}");
        shell.keys("escape", cx);
    }
}

/// R-57: Type, Status, Project and Team suggest the distinct values on the
/// tickets the desktop loaded for the repository (the ticket list, open
/// tickets by default), never one on a ticket it did not load; typing
/// narrows them, a suggestion chosen is the field's value, and a value no
/// ticket has is kept and saved.
#[gpui_kit::test]
async fn suggestions_come_from_the_loaded_tickets_values(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    for (id, title, kind, status, extra) in [
        (
            TICKET,
            "Main",
            "task",
            "open",
            "project: alpha\nteam: core\n",
        ),
        (
            OTHER,
            "Other",
            "bug",
            "in progress",
            "project: beta\nteam: web\n",
        ),
        (THIRD, "Third", "feature", "review", "project: alpha\n"),
    ] {
        let source = ticket_as(id, title, kind, status, extra);
        fixture.commit_file(&root, &ticket_path(id), source.as_bytes());
    }
    // Closed: the ticket list, of open tickets, never loads it.
    let closed = ticket_as(
        DEP_A,
        "Gone",
        "chore",
        "done",
        "project: gamma\nteam: old\nclosed_at: 2026-09-30T12:34:56Z\n\
         closed_by: Ada Lovelace <ada@example.invalid>\n",
    );
    fixture.commit_file(&root, &ticket_path(DEP_A), closed.as_bytes());
    // Another repository's ticket.
    let elsewhere = fixture.create_repository("b");
    let source = ticket_as(DEP_B, "Elsewhere", "epic", "open", "project: delta\n");
    fixture.commit_file(&elsewhere, &ticket_path(DEP_B), source.as_bytes());
    fixture.refresh(&root);
    fixture.refresh(&elsewhere);
    let shell = Shell::open(fixture, cx);
    shell.select(&root, cx);
    open(&shell, ticket(&root, TICKET), cx);
    edit(&shell, cx).await;

    for (chip, offered) in [
        (ChipField::Type, vec!["bug", "feature"]),
        (ChipField::Status, vec!["in progress", "review"]),
        (ChipField::Project, vec!["beta"]),
        (ChipField::Team, vec!["web"]),
    ] {
        open_sheet(&shell, chip, cx);
        assert_eq!(
            shell.rows(suggestion_id(chip), cx),
            offered,
            "{chip:?} suggests the loaded values but its own"
        );
        shell.keys("escape", cx);
    }

    // Typing narrows them, ignoring case; one chosen is the value.
    open_sheet(&shell, ChipField::Type, cx);
    empty_it(&shell, cx);
    assert_eq!(
        shell.rows("sheet-type-suggestion", cx),
        ["bug", "feature", "task"],
        "emptied, every value"
    );
    type_in(&shell, "FE", cx);
    assert_eq!(shell.rows("sheet-type-suggestion", cx), ["feature"]);
    shell.click(("sheet-type-suggestion", 0usize), cx);
    shell.frame(cx, |_, _| ());
    assert_eq!(holds(&shell, Field::Type, cx), "feature");
    assert!(value(&meta(&shell, cx), Field::Type) == Some("feature"));
    assert!(
        control_focused(&shell, Field::Type, cx),
        "focus stays in it"
    );
    assert!(dirty(&shell, cx));
    // Undone in the control, it holds what was typed again.
    shell.keys("secondary-z", cx);
    assert_eq!(holds(&shell, Field::Type, cx), "FE", "undone");
    assert!(value(&meta(&shell, cx), Field::Type) == Some("FE"));

    // A value no loaded ticket has is kept, and saved.
    empty_it(&shell, cx);
    type_in(&shell, "spike", cx);
    assert!(shell.rows("sheet-type-suggestion", cx).is_empty());
    shell.keys("escape", cx);
    let request = save(&shell, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));
    assert_eq!(
        shown(&shell, &root, TICKET).ticket_type.as_deref(),
        Some("spike")
    );

    // Once the list holds another repository's tickets, this tab suggests
    // only the item's own values.
    shell.select(&elsewhere, cx);
    open(&shell, ticket(&root, TICKET), cx);
    open_sheet(&shell, ChipField::Type, cx);
    empty_it(&shell, cx);
    assert_eq!(shell.rows("sheet-type-suggestion", cx), ["spike"]);
    shell.keys("escape", cx);
    open_sheet(&shell, ChipField::Project, cx);
    empty_it(&shell, cx);
    assert_eq!(shell.rows("sheet-project-suggestion", cx), ["alpha"]);
}

/// The element ID of `chip`'s field's suggestions.
fn suggestion_id(chip: ChipField) -> &'static str {
    match chip {
        ChipField::Type => "sheet-type-suggestion",
        ChipField::Status => "sheet-status-suggestion",
        ChipField::Project => "sheet-project-suggestion",
        ChipField::Team => "sheet-team-suggestion",
        _ => unreachable!(),
    }
}

/// An empty Title, Type or Status shows its message under its control;
/// Save stays disabled, and the header's Save, `secondary-s` in the tab
/// and in the sheet, and the close dialog's Save send nothing, each saying
/// why where it says anything. Filled, Save saves. A document's empty
/// Title is said in a document's words.
#[gpui_kit::test]
async fn an_empty_required_field_shows_its_message_and_save_is_disabled(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Required", "");
        fixture.create_document(root, DOCUMENT, "docs/one.md", "One");
    });
    open(&shell, ticket(&root, TICKET), cx);
    edit(&shell, cx).await;
    for (chip, field, message, id, before) in [
        (
            ChipField::All,
            Field::Title,
            TICKET_NEEDS_TITLE,
            "sheet-title-required",
            "Required",
        ),
        (
            ChipField::Type,
            Field::Type,
            TICKET_NEEDS_TYPE,
            "sheet-type-required",
            "task",
        ),
        (
            ChipField::Status,
            Field::Status,
            TICKET_NEEDS_STATUS,
            "sheet-status-required",
            "open",
        ),
    ] {
        open_sheet(&shell, chip, cx);
        assert!(!shell.present(id, cx), "{field:?}: nothing to say yet");
        empty_it(&shell, cx);
        assert_eq!(shell.label(id, cx).as_deref(), Some(message));
        assert!(dirty(&shell, cx), "{field:?}");
        let before_saves = saves(&shell, cx).len();
        // `secondary-s` inside the sheet.
        shell.keys("secondary-s", cx);
        assert_eq!(
            saves(&shell, cx).len(),
            before_saves,
            "{field:?}: sheet key"
        );
        assert_eq!(
            shell.label("toolbar-status", cx).as_deref(),
            Some(message),
            "{field:?}: the sheet's key says why"
        );
        shell.keys("escape", cx);
        // The header's Save, and `secondary-s` in the tab.
        shell.click("item-save", cx);
        assert_eq!(saves(&shell, cx).len(), before_saves, "{field:?}: Save");
        shell.keys("secondary-s", cx);
        assert_eq!(saves(&shell, cx).len(), before_saves, "{field:?}: tab key");
        assert_eq!(
            shell.label("toolbar-status", cx).as_deref(),
            Some(message),
            "{field:?}: the tab's key says why"
        );
        // The close dialog's Save.
        shell.keys("secondary-w", cx);
        assert_eq!(
            shell.label("close-draft-save-state", cx).as_deref(),
            Some(message),
            "{field:?}: the close dialog says why"
        );
        shell.click("close-draft-save-button", cx);
        assert_eq!(saves(&shell, cx).len(), before_saves, "{field:?}: dialog");
        shell.click("close-draft-cancel-button", cx);
        // Put back, nothing is said.
        open_sheet(&shell, chip, cx);
        type_in(&shell, before, cx);
        assert!(!shell.present(id, cx), "{field:?}: filled");
        shell.keys("escape", cx);
        assert!(!dirty(&shell, cx), "{field:?}: put back");
    }

    // Filled with a new value, Save saves.
    set(&shell, ChipField::Type, "", cx);
    shell.click("item-save", cx);
    assert!(saves(&shell, cx).is_empty(), "still empty");
    set(&shell, ChipField::Type, "bug", cx);
    let request = save(&shell, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));
    assert_eq!(
        shown(&shell, &root, TICKET).ticket_type.as_deref(),
        Some("bug")
    );

    // A document's Title.
    open(&shell, document(&root), cx);
    edit(&shell, cx).await;
    open_sheet(&shell, ChipField::All, cx);
    empty_it(&shell, cx);
    assert_eq!(
        shell.label("sheet-title-required", cx).as_deref(),
        Some(DOCUMENT_NEEDS_TITLE)
    );
    shell.keys("secondary-s", cx);
    assert_eq!(saves(&shell, cx).len(), 1, "the document saves nothing");
    shell.keys("escape", cx);
    shell.click("item-save", cx);
    assert_eq!(saves(&shell, cx).len(), 1);
}

/// Decision 14: while editing, the ticket's ID, short code, closure and
/// other keys are shown as they were and none of them takes typing: every
/// stop the sheet's Tab reaches after the five controls is typed and
/// pasted into, and the session stays clean.
#[gpui_kit::test]
async fn id_short_code_closure_and_every_other_key_are_not_editable(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(
            root,
            TICKET,
            "Kept",
            "slug: ae-k9x2b\nproject: p\nteam: t\ncustom: kept\nzeta: last\n",
        );
    });
    open(&shell, ticket(&root, TICKET), cx);
    edit(&shell, cx).await;
    let base = meta(&shell, cx);
    open_sheet(&shell, ChipField::Team, cx);
    assert!(control_focused(&shell, Field::Team, cx));
    for field in [Field::Title, Field::Type, Field::Status, Field::Project] {
        assert!(input(&shell, field, cx).is_some(), "{field:?} is a control");
    }
    cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string("pasted".to_owned()));
    let mut stops = 0;
    loop {
        shell.keys("tab", cx);
        if control_focused(&shell, Field::Title, cx) {
            break;
        }
        stops += 1;
        assert!(stops < 40, "Tab never came back round");
        assert!(
            Field::of(&base)
                .iter()
                .all(|field| !control_focused(&shell, *field, cx)),
            "past the controls, none is reached"
        );
        type_in(&shell, "q", cx);
        shell.keys("secondary-v backspace delete", cx);
    }
    assert!(stops > 3, "the facts, other fields and link are reached");
    assert!(has_sheet(&shell, cx));
    assert!(meta(&shell, cx) == base, "nothing changed");
    assert!(!dirty(&shell, cx));
    for (id, shown) in [
        ("item-id", TICKET),
        ("item-slug", "ae-k9x2b"),
        ("sheet-closure", NOT_CLOSED),
        ("item-lifecycle", LIFECYCLE_OPEN),
    ] {
        assert_eq!(shell.label(id, cx).as_deref(), Some(shown), "{id}");
        assert!(inside(&shell, id, "sheet-facts", cx), "{id}");
    }
    assert_eq!(shell.rows("meta-key", cx), ["custom", "zeta"]);
    assert_eq!(shell.rows("meta-value", cx), ["kept", "last"]);
}

/// Decision 14: a document's Status, Date and Author, from its other keys,
/// are rows, never controls, while editing: each chip opens on its row,
/// and nothing typed or pasted there changes it or the session.
#[gpui_kit::test]
async fn a_documents_status_date_and_author_are_not_editable(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_file(
            root,
            "docs/kept.md",
            document_source(
                DOCUMENT,
                "Kept",
                "status: draft\nauthor: Ada Lovelace\ndate: 2026-10-05\n",
            )
            .as_bytes(),
        );
    });
    open(&shell, document(&root), cx);
    edit(&shell, cx).await;
    let base = meta(&shell, cx);
    cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string("pasted".to_owned()));
    for (chip, row, id, shown) in [
        (ChipField::Status, "sheet-status", "item-status", "draft"),
        (
            ChipField::Author,
            "sheet-author",
            "item-author",
            "Ada Lovelace",
        ),
        (ChipField::Date, "sheet-date", "item-date", "2026-10-05"),
    ] {
        open_sheet(&shell, chip, cx);
        assert!(holds_focus(&shell, row, cx), "{chip:?} opens on {row}");
        type_in(&shell, "q", cx);
        shell.keys("secondary-v backspace delete", cx);
        assert_eq!(shell.label(id, cx).as_deref(), Some(shown), "{id}");
        assert!(!control_focused(&shell, Field::Title, cx));
        shell.keys("escape", cx);
    }
    assert!(
        input(&shell, Field::Status, cx).is_none(),
        "no Status control"
    );
    assert!(meta(&shell, cx) == base, "nothing changed");
    assert!(!dirty(&shell, cx));
}

/// A status of `closed` typed in the sheet is ordinary text: the facts
/// still say the ticket is not closed, the save writes the text and no
/// closure, and the ticket stays open, with Edit offered.
#[gpui_kit::test]
async fn a_status_of_closed_saves_as_ordinary_text_and_the_ticket_stays_open(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Still open", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    edit(&shell, cx).await;
    open_sheet(&shell, ChipField::Status, cx);
    empty_it(&shell, cx);
    type_in(&shell, "closed", cx);
    assert_eq!(
        shell.label("sheet-closure", cx).as_deref(),
        Some(NOT_CLOSED)
    );
    assert_eq!(
        shell.label("item-lifecycle", cx).as_deref(),
        Some(LIFECYCLE_OPEN)
    );
    shell.keys("escape", cx);
    let request = save(&shell, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));

    let item = shown(&shell, &root, TICKET);
    assert_eq!(item.status.as_deref(), Some("closed"));
    let closure = item.closure.clone().expect("a ticket's closure");
    assert_eq!(closure.state, ClosureState::Open, "the ticket stays open");
    assert!(closure.closed_at.is_none() && closure.closed_by.is_none());
    let branch = item.context.branch.clone().expect("its own branch");
    let file = committed(&root, &branch, TICKET);
    assert!(file.contains("status: closed\n"), "the text is saved");
    assert!(!file.contains("closed_at") && !file.contains("closed_by"));

    open_sheet(&shell, ChipField::All, cx);
    assert_eq!(
        shell.label("sheet-closure", cx).as_deref(),
        Some(NOT_CLOSED)
    );
    assert_eq!(
        shell.label("item-lifecycle", cx).as_deref(),
        Some(LIFECYCLE_OPEN)
    );
    shell.keys("escape", cx);
    shell.click("item-done", cx);
    shell.run(cx);
    assert!(shell.present("item-edit", cx), "Edit is still offered");
}

/// R-27: an emptied Project and Team are absent in the session's fields
/// and the draft, and the save removes both keys: the committed file has
/// neither, and the item reads with none.
#[gpui_kit::test]
async fn an_emptied_project_is_saved_as_absent(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Absent", "project: p\nteam: t\n");
    });
    open(&shell, ticket(&root, TICKET), cx);
    edit(&shell, cx).await;
    set(&shell, ChipField::Project, "", cx);
    set(&shell, ChipField::Team, "", cx);
    assert!(
        matches!(
            meta(&shell, cx),
            DraftFields::Ticket {
                project: None,
                team: None,
                ..
            }
        ),
        "absent, not empty"
    );
    let id = attached(&shell, &root, TICKET, cx).expect("the draft began");
    let drafted = registry(&shell, cx).read_with(cx, |registry, _| {
        registry.session(id).unwrap().current().clone()
    });
    assert!(matches!(
        drafted,
        DraftFields::Ticket {
            project: None,
            team: None,
            ..
        }
    ));
    let request = save(&shell, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));

    let item = shown(&shell, &root, TICKET);
    assert_eq!(item.project, None);
    assert_eq!(item.team, None);
    assert!(!item.unknown_metadata.contains_key("project"));
    assert!(!item.unknown_metadata.contains_key("team"));
    let branch = item.context.branch.clone().expect("its own branch");
    let file = committed(&root, &branch, TICKET);
    assert!(file.starts_with("---\n") && file.contains("title: Absent\n"));
    assert!(!file.contains("project:"), "no project key");
    assert!(!file.contains("team:"), "no team key");
}

/// A save asked for while the first draft still begins is sent once it
/// lands, with the fields as they are then: a Title emptied meanwhile is
/// checked again, so nothing is sent, the toolbar says why, and the tab
/// stays dirty (fix round 1, A). The keys go in one by one, with nothing
/// run between them, so the begin lands only after the last.
#[gpui_kit::test]
async fn a_save_deferred_until_the_draft_begins_checks_the_required_fields_again(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Deferred", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    edit(&shell, cx).await;
    open_sheet(&shell, ChipField::All, cx);
    assert!(control_focused(&shell, Field::Title, cx));
    let window = shell.window.into();
    for key in ["x", "secondary-s", "secondary-a", "backspace"] {
        cx.dispatch_keystroke(window, gpui_kit::Keystroke::parse(key).unwrap());
    }
    assert!(
        attached(&shell, &root, TICKET, cx).is_none(),
        "still beginning"
    );
    assert!(matches!(phase(&shell, cx), Phase::Saving), "the save waits");
    shell.run(cx);
    shell.frame(cx, |_, _| ());

    assert!(
        attached(&shell, &root, TICKET, cx).is_some(),
        "the draft began"
    );
    assert!(saves(&shell, cx).is_empty(), "nothing is sent");
    assert_eq!(
        shell.label("toolbar-status", cx).as_deref(),
        Some(TICKET_NEEDS_TITLE)
    );
    assert_eq!(phase(&shell, cx), Phase::Editing, "the run ended");
    assert!(value(&meta(&shell, cx), Field::Title) == Some(""));
    assert!(dirty(&shell, cx), "still dirty");
}

/// R-74: `secondary-s` with focus in the sheet saves the tab, and the
/// sheet stays open.
#[gpui_kit::test]
async fn secondary_s_inside_the_sheet_saves_the_tab(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Saved", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    edit(&shell, cx).await;
    open_sheet(&shell, ChipField::All, cx);
    type_in(&shell, " here", cx);
    assert!(control_focused(&shell, Field::Title, cx));

    shell.keys("secondary-s", cx);
    let sent = saves(&shell, cx);
    assert_eq!(sent.len(), 1, "the sheet's key saves");
    ended(&shell, sent[0], cx).await;
    assert_eq!(code(&shell, sent[0], cx), Some(ResultCode::Ok));
    assert_eq!(
        shown(&shell, &root, TICKET).title.as_deref(),
        Some("Saved here")
    );
    assert!(has_sheet(&shell, cx), "the sheet stays open");
    assert!(!dirty(&shell, cx));
}

/// R-74: while the tab is Preparing or Loading, and in the review, the
/// sheet's fields are not controls: nothing typed or pasted changes them.
/// The review shows the control's value as mine, and the sheet shows mine,
/// read only. Once Editing, they are controls again.
#[gpui_kit::test]
async fn the_controls_are_read_only_while_preparing_loading_and_in_review(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Mine", "");
        fixture.commit_ticket(root, OTHER, "Pending", "");
    });
    cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string("pasted".to_owned()));

    // Preparing.
    open(&shell, ticket(&root, TICKET), cx);
    let held = shell.backend.hold_job(JobKind::ContextPrepare);
    shell.click("item-edit", cx);
    held.wait_reached();
    shell.run(cx);
    assert!(matches!(phase(&shell, cx), Phase::Preparing(_)));
    open_sheet(&shell, ChipField::All, cx);
    assert!(!shell.present("sheet-title-input", cx), "no control");
    assert!(holds_focus(&shell, "sheet-title", cx));
    type_in(&shell, "q", cx);
    shell.keys("secondary-v backspace", cx);
    assert_eq!(
        shell.label("sheet-title-value", cx).as_deref(),
        Some("Mine")
    );
    assert_eq!(
        shell.label("sheet-description", cx).as_deref(),
        Some(VIEWING)
    );
    assert_eq!(shell.label("sheet-note", cx).as_deref(), Some(READ_ONLY));
    shell.keys("escape", cx);
    held.release();
    until_editing(&shell, cx).await;
    open_sheet(&shell, ChipField::All, cx);
    assert!(shell.present("sheet-title-input", cx), "Editing: a control");
    shell.keys("escape", cx);

    // In the review: the sheet shows mine, read only.
    into_review(&shell, &root, cx).await;
    let mine = meta(&shell, cx);
    assert_eq!(
        review_value(&shell, "review-mine", "Title", cx).as_deref(),
        Some("My title"),
        "the review shows the control's value as mine"
    );
    open_sheet(&shell, ChipField::All, cx);
    assert!(!shell.present("sheet-title-input", cx), "no control");
    assert_eq!(
        shell.label("sheet-title-value", cx).as_deref(),
        Some("My title")
    );
    type_in(&shell, "q", cx);
    shell.keys("secondary-v backspace", cx);
    shell.keys("tab", cx);
    type_in(&shell, "q", cx);
    assert!(meta(&shell, cx) == mine, "nothing changed");
    assert_eq!(
        shell.label("sheet-title-value", cx).as_deref(),
        Some("My title")
    );
    shell.keys("escape", cx);

    // Their title taken and used: the control holds the session's value.
    let title = shell
        .rows("review-field", cx)
        .iter()
        .position(|field| field == "Title")
        .expect("the review lists the title");
    shell.click(("review-take-theirs", title), cx);
    shell.click("review-use", cx);
    assert_eq!(phase(&shell, cx), Phase::Editing);
    open_sheet(&shell, ChipField::All, cx);
    assert_eq!(holds(&shell, Field::Title, cx), "Their title");
    shell.keys("escape", cx);

    // Kept after Done, the sheet views the saved copy, read only.
    set(&shell, ChipField::All, "Kept title", cx);
    shell.click("item-done", cx);
    let kept = pane(&shell, cx).read_with(cx, |pane, _| {
        pane.edit_session().is_some_and(|session| session.kept())
    });
    assert!(kept, "the session is kept");
    open_sheet(&shell, ChipField::All, cx);
    assert!(!shell.present("sheet-title-input", cx), "kept: no control");
    assert_eq!(
        shell.label("sheet-title-value", cx).as_deref(),
        Some("Their title")
    );
    type_in(&shell, "q", cx);
    assert_eq!(
        shell.label("sheet-title-value", cx).as_deref(),
        Some("Their title")
    );
    assert_eq!(shell.label("sheet-note", cx).as_deref(), Some(READ_ONLY));
    shell.keys("escape", cx);

    // Loading: a prepare that answered `discovery_pending` waits there.
    open(&shell, ticket(&root, OTHER), cx);
    shell.backend.replace_service(
        RepositoryService::open_at_with_failure_point_for_testing(
            shell.fixture.data_dir(),
            manyhands::repository::FailurePoint::BeforeIndexTransactionCommit,
        )
        .unwrap(),
    );
    shell.click("item-edit", cx);
    let request = prepares(&shell, cx)[0];
    ended(&shell, request, cx).await;
    assert_eq!(phase(&shell, cx), Phase::Loading);
    open_sheet(&shell, ChipField::All, cx);
    assert!(!shell.present("sheet-title-input", cx), "no control");
    type_in(&shell, "q", cx);
    shell.keys("secondary-v backspace", cx);
    assert_eq!(
        shell.label("sheet-title-value", cx).as_deref(),
        Some("Pending")
    );
    shell.keys("escape", cx);
    shell
        .backend
        .replace_service(RepositoryService::open_at(shell.fixture.data_dir()).unwrap());
}

/// R-74: during the review, `secondary-s` in the sheet sends nothing; the
/// review stays open, and so does the sheet.
#[gpui_kit::test]
async fn secondary_s_inside_the_sheet_does_nothing_during_the_review(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Mine", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    edit(&shell, cx).await;
    into_review(&shell, &root, cx).await;
    let before = saves(&shell, cx).len();
    open_sheet(&shell, ChipField::All, cx);
    shell.keys("secondary-s", cx);
    shell.frame(cx, |_, _| ());
    assert_eq!(saves(&shell, cx).len(), before, "nothing is sent");
    assert_eq!(phase(&shell, cx), Phase::Review, "the review stays open");
    assert!(has_sheet(&shell, cx), "and the sheet");
}

/// R-38: while editing, the sheet's body is a `draft-section`: text typed
/// into its controls is drawn there and nowhere else in the window. While
/// viewing it is not one.
#[gpui_kit::test]
async fn the_sheets_body_is_a_draft_section_while_editing(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Private", "project: p\n");
    });
    open(&shell, ticket(&root, TICKET), cx);
    open_sheet(&shell, ChipField::All, cx);
    assert!(
        !inside(&shell, "sheet-title", "draft-section", cx),
        "viewing"
    );
    shell.keys("escape", cx);

    edit(&shell, cx).await;
    for chip in [ChipField::All, ChipField::Project] {
        open_sheet(&shell, chip, cx);
        type_in(&shell, SENTINEL, cx);
        shell.keys("escape", cx);
    }
    open_sheet(&shell, ChipField::All, cx);
    for row in ["sheet-title", "sheet-project", "sheet-facts"] {
        assert!(inside(&shell, row, "draft-section", cx), "{row}");
    }
    assert!(inside(&shell, "sheet-title-input", "draft-section", cx));
    let drawn = drawn_outside_the_section(&shell, cx);
    assert!(
        !drawn.iter().any(|text| text.contains(SENTINEL)),
        "typed text is drawn only in the section"
    );
}

/// Task 27 fix 1, B1: every save consults the one gate, the one sent as
/// the fields are, once D2 answers that it sent nothing, its session having
/// ended, included. Title is emptied while D2 holds the save: nothing is
/// sent, the toolbar says the sheet's own words, and the tab stays dirty.
#[gpui_kit::test]
async fn a_save_d2_sent_nothing_for_still_consults_the_gate(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Gated", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    type_at_end(&shell, &host, "x", cx);
    let id = attached(&shell, &root, TICKET, cx).expect("the draft began");
    let untitled =
        crate::desktop::model::edit::fields::with_value(&meta(&shell, cx), Field::Title, "");
    let (pane, registry) = (pane(&shell, cx), registry(&shell, cx));
    shell.frame(cx, |window, cx| {
        pane.update(cx, |pane, cx| {
            assert!(pane.save_requested(window, cx), "Save is asked of D2");
            pane.change_fields(untitled, cx);
        });
        registry.update(cx, |registry, cx| registry.end_session_for_testing(id, cx));
    });
    shell.run(cx);
    shell.frame(cx, |_, _| ());

    assert!(saves(&shell, cx).is_empty(), "nothing is sent");
    assert_eq!(
        shell.label("toolbar-status", cx).as_deref(),
        Some(TICKET_NEEDS_TITLE)
    );
    assert_eq!(phase(&shell, cx), Phase::Editing, "the run ended");
    assert!(dirty(&shell, cx), "still dirty");
}
