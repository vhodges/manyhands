//! The stale-edit review, over real repositories (design, Stale-edit
//! review; decision 4; R-36, R-38, R-44, R-59, R-68, R-79). The review's
//! Required outcomes come first, one test per row of the design's table,
//! then the session-outcome row for Task 19 (Done or Save during the
//! Review: disabled), then the ways the review opens, its defaults, its
//! body editor, its privacy, and Task 19's decisions on the clean reloads
//! (Task 16's N2, Task 18's M3) and the close dialog (Task 18's M4).
//!
//! The helpers are `edit_tests`'s. Each test sets the caret, focuses the
//! editor and draws a frame before typing, and types through the window;
//! each waits on the worker and allows parking. Draft fields and items
//! would print text through `Debug`, so they are compared with `assert!`.
//! The words are literals: the tests fix them.

use gpui_kit::{ElementId, component::WindowExt};
use manyhands::drafts::DraftBase;

use super::*;
use crate::desktop::test_support::drafts::{Actor, second_actor};

const REVIEWING: &str = "Reviewing changes";
const CHANGED_WORDS: &str = "Someone changed this file outside Manyhands. Your edits are kept.";
const CANCEL_TO_REVIEW: &str = "Someone changed this file outside Manyhands. Your edits are kept. \
     Cancel to review the changes.";
const DONE_IN_REVIEW: &str = "Use these choices or Cancel first";
const TAKING_THEIRS: &str = "Taking theirs";
const KEEPING_MINE: &str = "Keeping mine";
const CHOOSE_ONE: &str = "Choose one";
const REPLACE_YOUR_EDITS: &str = "Replace your edits?";
const THEIR_BODY: &str = "Their body.\n";
/// The ticket's body as the fixture commits it, titled "Mine".
const TICKET_BODY: &str = "Body of Mine.\n";
/// The document's body as the fixture creates it, titled "One".
const DOCUMENT_BODY: &str = "# One\n";
/// Fact 81's first shape: a second load of what the first one gives would
/// change it again.
const FACT_81: &str = "wer $$\na $$b$$ c\n$$ wer\n";
const SENTINEL: &str = "SENTINEL-D3-REVIEW-41b9";
/// The review's intro, true whoever changed the item (M5).
const INTRO: &str = "This item changed since your edits began. Choose what to keep for \
     each field. Nothing is saved until you use your choices and save.";

/// A repository `a` holding the ticket `TICKET`, "Mine", and the document
/// `DOCUMENT`, "One", in an open window.
fn with_items(cx: &mut TestAppContext) -> (Shell, PathBuf) {
    shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Mine", "");
        fixture.create_document(root, DOCUMENT, "docs/one.md", "One");
    })
}

fn document(root: &Path) -> OpenTarget {
    OpenTarget::Item {
        repo: key(root),
        kind: ItemDtoKind::Document,
        id: item_id(DOCUMENT),
    }
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

fn edit_text(host: &Entity<EditorHost>, cx: &mut TestAppContext) -> String {
    host.read_with(cx, |host, cx| host.text(cx))
}

fn phase(shell: &Shell, cx: &mut TestAppContext) -> Phase {
    pane(shell, cx).read_with(cx, |pane, _| pane.edit_phase())
}

/// The base the shown tab's edit session holds.
fn session_base(shell: &Shell, cx: &mut TestAppContext) -> DraftBase {
    pane(shell, cx).read_with(cx, |pane, _| {
        pane.edit_session().expect("a session").base().clone()
    })
}

/// The draft the tab of `target` holds.
fn draft_of(shell: &Shell, target: &OpenTarget, cx: &mut TestAppContext) -> Option<DraftId> {
    let tab = TabKey::for_target(target);
    registry(shell, cx).read_with(cx, |registry, _| registry.attached(&tab))
}

/// The body of D2's draft `id` in memory.
fn drafted_body(shell: &Shell, id: DraftId, cx: &mut TestAppContext) -> String {
    registry(shell, cx).read_with(cx, |registry, _| {
        body_of(registry.session(id).expect("a live draft").current()).to_owned()
    })
}

/// The bytes of the draft `id`'s file.
fn draft_bytes(shell: &Shell, id: DraftId) -> Vec<u8> {
    let path = shell
        .fixture
        .data_dir()
        .join("drafts")
        .join(format!("{id}.json"));
    std::fs::read(path).expect("the draft's file")
}

/// Lets the draft's newest revision be written, if anything may write it:
/// its idle time passes.
fn idle(shell: &Shell, cx: &mut TestAppContext) {
    cx.executor().advance_clock(IDLE);
    shell.run(cx);
    shell.frame(cx, |_, _| ());
}

/// Reads the shown tab again, as a refresh does, and lets the read land.
fn read_again(shell: &Shell, cx: &mut TestAppContext) {
    pane(shell, cx).update(cx, |pane, cx| {
        pane.reload(cx);
    });
    shell.run(cx);
    shell.frame(cx, |_, _| ());
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

/// Presses Save and waits until the save it sent has ended and settled.
async fn save(shell: &Shell, cx: &mut TestAppContext) -> DesktopRequestId {
    let before = saves(shell, cx).len();
    shell.click("item-save", cx);
    let sent = saves(shell, cx);
    assert_eq!(sent.len(), before + 1, "Save sent one save");
    ended(shell, sent[0], cx).await;
    sent[0]
}

/// Presses Save over a base someone else has moved: the save answers
/// `external_change`, and the review opens on it.
async fn stale_save(shell: &Shell, cx: &mut TestAppContext) -> DesktopRequestId {
    let request = save(shell, cx).await;
    assert_eq!(code(shell, request, cx), Some(ResultCode::ExternalChange));
    until_review(shell, cx).await;
    assert_eq!(phase(shell, cx), Phase::Review, "the review opens");
    request
}

/// The review's row of the field named `name`.
fn row(shell: &Shell, name: &str, cx: &mut TestAppContext) -> usize {
    shell
        .rows("review-field", cx)
        .iter()
        .position(|field| field == name)
        .unwrap_or_else(|| panic!("the review lists {name}"))
}

/// What the review says of the field `name`'s choice.
fn state(shell: &Shell, name: &str, cx: &mut TestAppContext) -> Option<String> {
    let ix = row(shell, name, cx);
    shell.label(("review-state", ix), cx)
}

/// The field `name`'s value on `side`: `review-before`, `review-theirs` or
/// `review-mine`.
fn value(shell: &Shell, side: &'static str, name: &str, cx: &mut TestAppContext) -> String {
    let ix = row(shell, name, cx);
    shell.label((side, ix), cx).expect("the value is shown")
}

fn take_theirs(shell: &Shell, name: &str, cx: &mut TestAppContext) {
    let ix = row(shell, name, cx);
    shell.click(("review-take-theirs", ix), cx);
}

fn keep_mine(shell: &Shell, name: &str, cx: &mut TestAppContext) {
    let ix = row(shell, name, cx);
    shell.click(("review-keep-mine", ix), cx);
}

/// The review's body editor's host, once the body has a result.
fn review_host(shell: &Shell, cx: &mut TestAppContext) -> Option<Entity<EditorHost>> {
    pane(shell, cx).read_with(cx, |pane, _| pane.review_host())
}

/// The body's result, as the review's body editor holds it.
fn result_text(shell: &Shell, cx: &mut TestAppContext) -> String {
    let host = review_host(shell, cx).expect("the body's result is shown");
    edit_text(&host, cx)
}

/// Types `typed` at the end of the review's body editor.
fn type_in_result(shell: &Shell, typed: &str, cx: &mut TestAppContext) {
    let host = review_host(shell, cx).expect("the body's result is shown");
    type_at_end(shell, &host, typed, cx);
}

/// The details sheet's change of the title, which Task 20 draws: the
/// session's fields besides the body, retitled.
fn retitle(shell: &Shell, title: &str, cx: &mut TestAppContext) {
    pane(shell, cx).update(cx, |pane, cx| {
        let mut fields = pane
            .edit_session()
            .expect("a session")
            .base()
            .fields
            .clone();
        match &mut fields {
            DraftFields::Document { title: old, .. } | DraftFields::Ticket { title: old, .. } => {
                *old = title.to_owned()
            }
            DraftFields::Comment { .. } => unreachable!(),
        }
        pane.change_fields(fields, cx);
    });
    shell.run(cx);
}

/// The second actor saves the document with `title` and `body`.
fn their_document(shell: &Shell, root: &Path, title: &str, body: &str) {
    second_actor(
        &shell.fixture,
        Actor::SaveDocumentAs {
            root,
            id: DOCUMENT,
            title,
            body,
        },
    );
}

/// The second actor saves the ticket with `title` and `status`.
fn their_ticket(shell: &Shell, root: &Path, title: &str, status: &str) {
    second_actor(
        &shell.fixture,
        Actor::SaveTicket {
            root,
            id: TICKET,
            title,
            status,
        },
    );
}

/// Opens the document, edits it and types "x" at the end of its body; the
/// draft is written. Gives the host.
async fn edited_document(
    shell: &Shell,
    root: &Path,
    cx: &mut TestAppContext,
) -> Entity<EditorHost> {
    open(shell, document(root), cx);
    let host = edit(shell, cx).await;
    type_at_end(shell, &host, "x", cx);
    idle(shell, cx);
    host
}

/// Opens the ticket, edits it and types "x" at the end of its body; the
/// draft is written. Gives the host.
async fn edited_ticket(shell: &Shell, root: &Path, cx: &mut TestAppContext) -> Entity<EditorHost> {
    open(shell, ticket(root, TICKET), cx);
    let host = edit(shell, cx).await;
    type_at_end(shell, &host, "x", cx);
    idle(shell, cx);
    host
}

/// Every label the window draws outside a draft's own section, where its
/// text belongs (R-38).
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

// The review's Required outcomes (design, Stale-edit review), one test per
// row, written first.

/// Row 1: Open, Cancel. The draft file is byte-identical, and the editor
/// holds mine; the session's base is unchanged.
#[gpui_kit::test]
async fn cancel_leaves_draft_and_base_unchanged(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = with_items(cx);
    let host = edited_document(&shell, &root, cx).await;
    let mine = edit_text(&host, cx);
    let id = draft_of(&shell, &document(&root), cx).expect("a draft");
    let bytes = draft_bytes(&shell, id);
    let base = session_base(&shell, cx);
    their_document(&shell, &root, "One", THEIR_BODY);

    stale_save(&shell, cx).await;
    assert!(shell.present("item-review", cx), "the review is shown");
    shell.click("review-cancel", cx);
    assert_eq!(phase(&shell, cx), Phase::Editing);
    assert!(!shell.present("item-review", cx));
    assert!(shell.present("item-edit-view", cx), "the edit editor again");
    assert!(edit_text(&host, cx) == mine, "the editor holds mine");
    idle(&shell, cx);
    assert!(
        draft_bytes(&shell, id) == bytes,
        "the draft file is byte-identical"
    );
    assert!(drafted_body(&shell, id, cx) == mine);
    assert!(session_base(&shell, cx) == base, "the base is unchanged");

    // The feed resumes: what is typed now is drafted and written.
    type_at_end(&shell, &host, "w", cx);
    idle(&shell, cx);
    assert!(drafted_body(&shell, id, cx) == format!("{mine}w"));
    assert!(draft_bytes(&shell, id) != bytes, "and written");
}

/// Row 2: Take theirs on the body, then Cancel. The draft file is
/// byte-identical, and the editor holds mine.
#[gpui_kit::test]
async fn cancel_after_taking_theirs_on_the_body_leaves_the_draft_file_byte_identical(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let (shell, root) = with_items(cx);
    let host = edited_document(&shell, &root, cx).await;
    let mine = edit_text(&host, cx);
    let id = draft_of(&shell, &document(&root), cx).expect("a draft");
    let bytes = draft_bytes(&shell, id);
    their_document(&shell, &root, "One", THEIR_BODY);

    stale_save(&shell, cx).await;
    take_theirs(&shell, "Body", cx);
    assert_eq!(state(&shell, "Body", cx).as_deref(), Some(TAKING_THEIRS));
    assert!(result_text(&shell, cx) == THEIR_BODY);
    idle(&shell, cx);
    assert!(draft_bytes(&shell, id) == bytes, "nothing is written");

    shell.click("review-cancel", cx);
    assert!(edit_text(&host, cx) == mine, "the editor holds mine");
    idle(&shell, cx);
    assert!(
        draft_bytes(&shell, id) == bytes,
        "the draft file is byte-identical"
    );
}

/// Row 3: Edit the body during the review, then Cancel. The draft file is
/// byte-identical, and the editor holds mine.
#[gpui_kit::test]
async fn cancel_after_editing_the_body_restores_mine(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = with_items(cx);
    let host = edited_document(&shell, &root, cx).await;
    let mine = edit_text(&host, cx);
    let id = draft_of(&shell, &document(&root), cx).expect("a draft");
    let bytes = draft_bytes(&shell, id);
    their_document(&shell, &root, "One", THEIR_BODY);

    stale_save(&shell, cx).await;
    take_theirs(&shell, "Body", cx);
    type_in_result(&shell, "z", cx);
    assert!(result_text(&shell, cx) == format!("{THEIR_BODY}z"));
    assert!(edit_text(&host, cx) == mine, "the edit editor is untouched");
    idle(&shell, cx);
    assert!(draft_bytes(&shell, id) == bytes, "nothing is written");
    assert!(drafted_body(&shell, id, cx) == mine, "nor fed to the draft");

    shell.click("review-cancel", cx);
    assert!(edit_text(&host, cx) == mine, "the editor holds mine");
    idle(&shell, cx);
    assert!(
        draft_bytes(&shell, id) == bytes,
        "the draft file is byte-identical"
    );
    assert!(drafted_body(&shell, id, cx) == mine);
}

/// Row 4: Edit the body, then switch its choice. It asks first, and No
/// keeps the edits; Replace takes the other side. Switching with no edit
/// asks nothing.
#[gpui_kit::test]
async fn switching_the_body_choice_after_an_edit_asks_first(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = with_items(cx);
    let host = edited_document(&shell, &root, cx).await;
    let mine = edit_text(&host, cx);
    their_document(&shell, &root, "One", THEIR_BODY);
    stale_save(&shell, cx).await;

    take_theirs(&shell, "Body", cx);
    keep_mine(&shell, "Body", cx);
    assert!(
        !shell.present("review-replace-dialog", cx),
        "no edit: no ask"
    );
    assert!(result_text(&shell, cx) == mine);

    take_theirs(&shell, "Body", cx);
    type_in_result(&shell, "z", cx);
    keep_mine(&shell, "Body", cx);
    assert!(shell.present("review-replace-dialog", cx), "it asks first");
    assert_eq!(
        shell.label("review-replace-message", cx).as_deref(),
        Some(REPLACE_YOUR_EDITS)
    );
    shell.click("review-replace-no-button", cx);
    assert!(!shell.present("review-replace-dialog", cx));
    assert!(
        result_text(&shell, cx) == format!("{THEIR_BODY}z"),
        "No keeps the edits"
    );
    assert_eq!(state(&shell, "Body", cx).as_deref(), Some(TAKING_THEIRS));

    keep_mine(&shell, "Body", cx);
    shell.click("review-replace-yes-button", cx);
    assert!(result_text(&shell, cx) == mine, "Replace takes mine");
    assert_eq!(state(&shell, "Body", cx).as_deref(), Some(KEEPING_MINE));
}

/// Row 5: the body changed only by them, the title only by me; Use, Save.
/// The saved file has their body and my title.
#[gpui_kit::test]
async fn a_body_changed_only_by_them_defaults_to_theirs(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = with_items(cx);
    open(&shell, document(&root), cx);
    let host = edit(&shell, cx).await;
    retitle(&shell, "My title", cx);
    their_document(&shell, &root, "One", THEIR_BODY);

    stale_save(&shell, cx).await;
    assert_eq!(value(&shell, "review-before", "Title", cx), "One");
    assert_eq!(value(&shell, "review-theirs", "Title", cx), "One");
    assert_eq!(value(&shell, "review-mine", "Title", cx), "My title");
    assert_eq!(state(&shell, "Title", cx).as_deref(), Some(KEEPING_MINE));
    assert!(value(&shell, "review-before", "Body", cx) == DOCUMENT_BODY);
    assert!(value(&shell, "review-theirs", "Body", cx) == THEIR_BODY);
    assert!(value(&shell, "review-mine", "Body", cx) == DOCUMENT_BODY);
    assert_eq!(state(&shell, "Body", cx).as_deref(), Some(TAKING_THEIRS));
    assert!(result_text(&shell, cx) == THEIR_BODY);

    shell.click("review-use", cx);
    assert_eq!(phase(&shell, cx), Phase::Editing);
    assert!(edit_text(&host, cx) == THEIR_BODY, "the result is in");
    let request = save(&shell, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));
    let item = shown(&shell, &root, DOCUMENT);
    assert_eq!(item.title.as_deref(), Some("My title"), "my title");
    assert!(item.body.as_deref() == Some(THEIR_BODY), "their body");
}

/// Row 6: both changed the body; Take theirs, edit it, Use, Save. The
/// saved body is the edited text.
#[gpui_kit::test]
async fn an_edited_body_result_is_what_save_writes(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = with_items(cx);
    let host = edited_document(&shell, &root, cx).await;
    their_document(&shell, &root, "One", THEIR_BODY);
    stale_save(&shell, cx).await;

    take_theirs(&shell, "Body", cx);
    type_in_result(&shell, "z", cx);
    let edited = format!("{THEIR_BODY}z");
    shell.click("review-use", cx);
    assert!(edit_text(&host, cx) == edited);
    let request = save(&shell, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));
    let item = shown(&shell, &root, DOCUMENT);
    assert!(
        item.body.as_deref() == Some(edited.as_str()),
        "the edited text"
    );
}

/// Row 7: both changed the title, and no choice is made: Use is disabled.
/// The fields changed by one side default to that side's: their status,
/// my body. Once the title is chosen, Use saves the choices.
#[gpui_kit::test]
async fn one_sided_changes_default_and_both_changed_fields_must_be_chosen(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = with_items(cx);
    let host = edited_ticket(&shell, &root, cx).await;
    retitle(&shell, "My title", cx);
    idle(&shell, cx);
    let mine = edit_text(&host, cx);
    their_ticket(&shell, &root, "Their title", "in progress");

    stale_save(&shell, cx).await;
    assert_eq!(state(&shell, "Title", cx).as_deref(), Some(CHOOSE_ONE));
    assert_eq!(state(&shell, "Status", cx).as_deref(), Some(TAKING_THEIRS));
    assert_eq!(state(&shell, "Body", cx).as_deref(), Some(KEEPING_MINE));
    assert!(shell.inert_button("review-use", cx), "Use is disabled");
    shell.click("review-use", cx);
    assert_eq!(phase(&shell, cx), Phase::Review, "Use did nothing");
    assert!(shell.present("review-use-note", cx), "it says why");

    keep_mine(&shell, "Title", cx);
    assert_eq!(state(&shell, "Title", cx).as_deref(), Some(KEEPING_MINE));
    assert!(!shell.present("review-use-note", cx));
    shell.click("review-use", cx);
    assert_eq!(phase(&shell, cx), Phase::Editing);
    let request = save(&shell, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));
    let item = shown(&shell, &root, TICKET);
    assert_eq!(item.title.as_deref(), Some("My title"));
    assert_eq!(item.status.as_deref(), Some("in progress"));
    assert!(item.body.as_deref() == Some(mine.as_str()));
}

/// Row 8: another change by them after Use these choices. Save answers
/// `external_change` and the review opens again, on their newer body.
#[gpui_kit::test]
async fn another_change_by_them_after_use_reopens_the_review_on_save(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = with_items(cx);
    let host = edited_document(&shell, &root, cx).await;
    let mine = edit_text(&host, cx);
    their_document(&shell, &root, "One", "First of theirs.\n");
    stale_save(&shell, cx).await;
    keep_mine(&shell, "Body", cx);
    shell.click("review-use", cx);
    assert_eq!(phase(&shell, cx), Phase::Editing);

    their_document(&shell, &root, "One", "Second of theirs.\n");
    stale_save(&shell, cx).await;
    assert!(value(&shell, "review-theirs", "Body", cx) == "Second of theirs.\n");
    assert!(value(&shell, "review-mine", "Body", cx) == mine);
    let item = shown(&shell, &root, DOCUMENT);
    assert!(
        item.body.as_deref() == Some("Second of theirs.\n"),
        "nothing is written"
    );
}

/// Row 9 (R-36): an unverified base, from D2's rule after a save another
/// actor followed. The review opens before any save, with the base's
/// fields as base. Nothing is written to the draft while it is open.
#[gpui_kit::test]
async fn an_unverified_base_opens_the_review_before_any_save(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = with_items(cx);
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    type_at_end(&shell, &host, "x", cx);
    let read = shell.backend.hold_read(ReadKind::DraftItem);
    shell.click("item-save", cx);
    let request = saves(&shell, cx)[0];
    shell.until_ended(request, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));
    shell
        .fixture
        .save_ticket(&root, TICKET, "Someone else's title");
    read.send(()).unwrap();
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    assert!(session_base(&shell, cx).observation.is_none(), "unverified");

    type_at_end(&shell, &host, "y", cx);
    idle(&shell, cx);
    let id = draft_of(&shell, &ticket(&root, TICKET), cx).expect("a draft began");
    let bytes = draft_bytes(&shell, id);
    type_at_end(&shell, &host, "z", cx);
    let sent = saves(&shell, cx).len();
    shell.click("item-save", cx);
    assert_eq!(saves(&shell, cx).len(), sent, "nothing is sent");
    assert_eq!(phase(&shell, cx), Phase::Review, "the review opens");
    until_review(&shell, cx).await;

    let saved = format!("{TICKET_BODY}x");
    assert_eq!(value(&shell, "review-before", "Title", cx), "Mine");
    assert_eq!(
        value(&shell, "review-theirs", "Title", cx),
        "Someone else's title"
    );
    assert_eq!(value(&shell, "review-mine", "Title", cx), "Mine");
    assert_eq!(state(&shell, "Title", cx).as_deref(), Some(TAKING_THEIRS));
    assert!(value(&shell, "review-before", "Body", cx) == saved);
    assert!(value(&shell, "review-theirs", "Body", cx) == saved);
    assert!(value(&shell, "review-mine", "Body", cx) == format!("{saved}yz"));
    assert_eq!(state(&shell, "Body", cx).as_deref(), Some(KEEPING_MINE));
    idle(&shell, cx);
    assert!(
        draft_bytes(&shell, id) == bytes,
        "the typed z waits: nothing is written while the review is open"
    );

    shell.click("review-use", cx);
    let request = save(&shell, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));
    let item = shown(&shell, &root, TICKET);
    assert_eq!(item.title.as_deref(), Some("Someone else's title"));
    assert!(item.body.as_deref() == Some(format!("{saved}yz").as_str()));
}

// The session-outcome row for Task 19 (R-76).

/// Done or Save during the Review: disabled. Done says why; Save, its key
/// and Done send and ask nothing.
#[gpui_kit::test]
async fn done_and_save_are_disabled_during_the_review(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = with_items(cx);
    edited_ticket(&shell, &root, cx).await;
    shell.fixture.save_ticket(&root, TICKET, "Theirs");
    stale_save(&shell, cx).await;
    let sent = saves(&shell, cx).len();

    assert_eq!(
        shell.label("item-done-note", cx).as_deref(),
        Some(DONE_IN_REVIEW)
    );
    shell.click("item-done", cx);
    assert!(!shell.present("done-draft-dialog", cx), "Done asks nothing");
    assert_eq!(phase(&shell, cx), Phase::Review, "Done does nothing");
    assert!(shell.inert_button("item-save", cx), "Save is disabled");
    shell.click("item-save", cx);
    shell.keys("secondary-s", cx);
    assert_eq!(saves(&shell, cx).len(), sent, "nothing is sent");
    assert_eq!(phase(&shell, cx), Phase::Review);
    assert!(shell.present("item-review", cx));
}

// How the review opens, and what it holds.

/// A save the library refuses as `external_change` opens the review in
/// the page's place, the toolbar hidden, under the change banner, which
/// offers no Compare… while the review is open. The draft file is
/// unchanged, and nothing is written to the item.
#[gpui_kit::test]
async fn a_stale_save_opens_the_review_and_leaves_the_draft_file_unchanged(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let (shell, root) = with_items(cx);
    edited_ticket(&shell, &root, cx).await;
    let id = draft_of(&shell, &ticket(&root, TICKET), cx).expect("a draft");
    let bytes = draft_bytes(&shell, id);
    shell.fixture.save_ticket(&root, TICKET, "Theirs");

    stale_save(&shell, cx).await;
    assert_eq!(pill(&shell, cx).as_deref(), Some(REVIEWING));
    assert_eq!(shell.label("review-intro", cx).as_deref(), Some(INTRO));
    assert!(shell.present("item-review", cx));
    assert!(!shell.present("item-toolbar", cx), "the toolbar is hidden");
    assert!(!shell.present("item-edit-view", cx), "the page's place");
    assert_eq!(
        shell.label("item-save-changed", cx).as_deref(),
        Some(CHANGED_WORDS)
    );
    assert!(!shell.present("item-save-compare", cx));
    assert_eq!(value(&shell, "review-theirs", "Title", cx), "Theirs");
    idle(&shell, cx);
    assert!(
        draft_bytes(&shell, id) == bytes,
        "the draft file is unchanged"
    );
    let item = shown(&shell, &root, TICKET);
    assert!(
        item.body.as_deref() == Some(TICKET_BODY),
        "nothing is written"
    );
}

/// R-68: a refresh that finds the item changed under a dirty draft shows
/// the change banner, with Compare…; nothing opens by itself. Compare…
/// opens the review on the item as read again.
#[gpui_kit::test]
async fn a_refresh_shows_the_change_banner_and_compare_opens_the_review(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = with_items(cx);
    let host = edited_ticket(&shell, &root, cx).await;
    let mine = edit_text(&host, cx);
    shell.fixture.save_ticket(&root, TICKET, "Theirs");

    read_again(&shell, cx);
    assert_eq!(phase(&shell, cx), Phase::Editing, "nothing opens by itself");
    assert!(!shell.present("item-review", cx));
    assert!(edit_text(&host, cx) == mine, "the text is untouched");
    assert_eq!(
        shell.label("item-save-changed", cx).as_deref(),
        Some(CHANGED_WORDS)
    );
    assert_eq!(
        shell.label("item-save-compare", cx).as_deref(),
        Some("Compare\u{2026}")
    );

    shell.click("item-save-compare", cx);
    assert_eq!(phase(&shell, cx), Phase::Review);
    until_review(&shell, cx).await;
    assert_eq!(value(&shell, "review-theirs", "Title", cx), "Theirs");
    assert_eq!(state(&shell, "Title", cx).as_deref(), Some(TAKING_THEIRS));
    assert!(value(&shell, "review-mine", "Body", cx) == mine);
}

/// R-68 for a session kept after Done: the kept change banner's Compare…
/// continues the session into the review.
#[gpui_kit::test]
async fn compare_on_the_kept_change_banner_opens_the_review(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = with_items(cx);
    let host = edited_ticket(&shell, &root, cx).await;
    let mine = edit_text(&host, cx);
    shell.click("item-done", cx);
    shell.run(cx);
    let kept = pane(&shell, cx).read_with(cx, |pane, _| {
        pane.edit_session().map(|session| session.kept())
    });
    assert_eq!(kept, Some(true), "the session is kept");
    shell.fixture.save_ticket(&root, TICKET, "Theirs");
    read_again(&shell, cx);

    shell.click("item-kept-compare", cx);
    assert_eq!(phase(&shell, cx), Phase::Review);
    until_review(&shell, cx).await;
    assert!(shell.present("item-review", cx));
    assert_eq!(value(&shell, "review-theirs", "Title", cx), "Theirs");
    assert!(value(&shell, "review-mine", "Body", cx) == mine);
}

/// Their title and my body: Take theirs for the title, Keep mine for the
/// body, Use, Save. The saved file has exactly that.
#[gpui_kit::test]
async fn taking_theirs_for_the_title_and_mine_for_the_body_saves_exactly_that(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let (shell, root) = with_items(cx);
    let host = edited_document(&shell, &root, cx).await;
    retitle(&shell, "My title", cx);
    let mine = edit_text(&host, cx);
    their_document(&shell, &root, "Their title", THEIR_BODY);
    stale_save(&shell, cx).await;
    assert_eq!(state(&shell, "Title", cx).as_deref(), Some(CHOOSE_ONE));
    assert_eq!(state(&shell, "Body", cx).as_deref(), Some(CHOOSE_ONE));

    take_theirs(&shell, "Title", cx);
    keep_mine(&shell, "Body", cx);
    shell.click("review-use", cx);
    let request = save(&shell, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));
    let item = shown(&shell, &root, DOCUMENT);
    assert_eq!(item.title.as_deref(), Some("Their title"));
    assert!(item.body.as_deref() == Some(mine.as_str()));
}

/// A ticket the second actor retitled and moved on, while I changed its
/// body: their title and status, and my body, are reviewed and saved.
#[gpui_kit::test]
async fn a_ticket_change_by_the_second_actor_is_reviewed(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = with_items(cx);
    let host = edited_ticket(&shell, &root, cx).await;
    let mine = edit_text(&host, cx);
    their_ticket(&shell, &root, "Their title", "in progress");

    stale_save(&shell, cx).await;
    assert_eq!(value(&shell, "review-before", "Status", cx), "open");
    assert_eq!(value(&shell, "review-theirs", "Status", cx), "in progress");
    assert_eq!(value(&shell, "review-mine", "Status", cx), "open");
    assert_eq!(state(&shell, "Status", cx).as_deref(), Some(TAKING_THEIRS));
    assert_eq!(state(&shell, "Title", cx).as_deref(), Some(TAKING_THEIRS));
    assert_eq!(state(&shell, "Body", cx).as_deref(), Some(KEEPING_MINE));
    shell.click("review-use", cx);
    let request = save(&shell, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));
    let item = shown(&shell, &root, TICKET);
    assert_eq!(item.title.as_deref(), Some("Their title"));
    assert_eq!(item.status.as_deref(), Some("in progress"));
    assert!(item.body.as_deref() == Some(mine.as_str()));
}

/// A second change by the other actor while the review is open, two
/// different bodies: Use goes on the first, so the next Save answers
/// `external_change`, and the review opens again on the second.
#[gpui_kit::test]
async fn a_second_change_during_review_reopens_it_on_save(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = with_items(cx);
    let host = edited_document(&shell, &root, cx).await;
    let mine = edit_text(&host, cx);
    their_document(&shell, &root, "One", "First of theirs.\n");
    stale_save(&shell, cx).await;
    assert!(value(&shell, "review-theirs", "Body", cx) == "First of theirs.\n");

    their_document(&shell, &root, "One", "Second of theirs.\n");
    keep_mine(&shell, "Body", cx);
    shell.click("review-use", cx);
    assert!(edit_text(&host, cx) == mine);
    stale_save(&shell, cx).await;
    assert!(value(&shell, "review-theirs", "Body", cx) == "Second of theirs.\n");
    assert!(value(&shell, "review-before", "Body", cx) == "First of theirs.\n");
    let item = shown(&shell, &root, DOCUMENT);
    assert!(item.body.as_deref() == Some("Second of theirs.\n"));
}

/// A body changed by both has no result until it is chosen, and Use is
/// disabled until then; Keep mine makes mine the result, and Use puts it
/// in the edit editor.
#[gpui_kit::test]
async fn a_body_changed_by_both_must_be_chosen_before_use(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = with_items(cx);
    let host = edited_document(&shell, &root, cx).await;
    let mine = edit_text(&host, cx);
    their_document(&shell, &root, "One", THEIR_BODY);
    stale_save(&shell, cx).await;

    assert_eq!(state(&shell, "Body", cx).as_deref(), Some(CHOOSE_ONE));
    assert!(review_host(&shell, cx).is_none(), "no result until chosen");
    assert!(shell.present("review-result-note", cx));
    assert!(shell.inert_button("review-use", cx));
    shell.click("review-use", cx);
    assert_eq!(phase(&shell, cx), Phase::Review, "Use did nothing");

    keep_mine(&shell, "Body", cx);
    assert!(result_text(&shell, cx) == mine);
    assert!(!shell.present("review-use-note", cx));
    shell.click("review-use", cx);
    assert_eq!(phase(&shell, cx), Phase::Editing, "Use is enabled");
    assert!(edit_text(&host, cx) == mine);
}

/// R-59: Escape in the review's body editor moves focus to the header's
/// Done. It cancels nothing.
#[gpui_kit::test]
async fn escape_in_the_reviews_body_editor_focuses_done(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = with_items(cx);
    edited_ticket(&shell, &root, cx).await;
    shell.fixture.save_ticket(&root, TICKET, "Theirs");
    stale_save(&shell, cx).await;
    let host = review_host(&shell, cx).expect("my body is the result");
    let editor = host_editor(&host, cx);
    let end = text(&editor, cx).len();
    focus_at(&shell, &editor, end, cx);

    shell.keys("escape", cx);
    assert!(holds_focus(&shell, "item-done", cx), "Done has the focus");
    assert_eq!(phase(&shell, cx), Phase::Review, "nothing is cancelled");
    assert!(shell.present("item-review", cx));
}

/// R-38: the review's draft text stays inside a `draft-section`. With a
/// sentinel in my body, nothing the window draws outside the section, and
/// no operation entry, holds it.
#[gpui_kit::test]
async fn the_d2_privacy_sweep_still_holds_with_the_review_open(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = with_items(cx);
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    type_at_end(&shell, &host, SENTINEL, cx);
    idle(&shell, cx);
    shell.fixture.save_ticket(&root, TICKET, "Theirs");
    stale_save(&shell, cx).await;

    assert!(
        value(&shell, "review-mine", "Body", cx).contains(SENTINEL),
        "the review shows my body"
    );
    assert!(result_text(&shell, cx).contains(SENTINEL));
    for drawn in drawn_outside_the_section(&shell, cx) {
        assert!(!drawn.contains(SENTINEL), "drawn outside the section");
    }
    let entries = shell.operations(cx).read_with(cx, |operations, _| {
        operations
            .requests()
            .iter()
            .flat_map(|request| {
                let mut texts = vec![request.intent.label().to_owned()];
                if let RequestState::Finished(.., message) = &request.state {
                    texts.push(message.clone());
                }
                texts
            })
            .collect::<Vec<_>>()
    });
    for entry in entries {
        assert!(!entry.contains(SENTINEL), "in an operation entry");
    }
    let notifications = shell.frame(cx, |window, cx| window.notifications(cx).len());
    assert_eq!(notifications, 0);
}

/// R-79: a result of fact 81's shape goes into the edit editor exactly,
/// and reads back byte for byte: in the review's editor, the edit editor,
/// D2's draft and the saved file. A second load would change it.
#[gpui_kit::test]
async fn using_the_choices_puts_the_result_in_exactly(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        let source = ticket_with_body(TICKET, "Mine", "", FACT_81);
        fixture.commit_file(root, &ticket_path(TICKET), source.as_bytes());
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    type_at_end(&shell, &host, "x", cx);
    let mine = edit_text(&host, cx);
    assert!(load(&mine) != mine, "a second load would change it");
    idle(&shell, cx);
    shell.fixture.save_ticket(&root, TICKET, "Theirs");
    stale_save(&shell, cx).await;

    assert_eq!(state(&shell, "Body", cx).as_deref(), Some(KEEPING_MINE));
    assert!(
        result_text(&shell, cx) == mine,
        "the review's editor: exactly"
    );
    shell.click("review-use", cx);
    assert!(edit_text(&host, cx) == mine, "the edit editor: exactly");
    let id = draft_of(&shell, &ticket(&root, TICKET), cx).expect("the draft");
    assert!(drafted_body(&shell, id, cx) == mine, "D2's draft: exactly");
    let request = save(&shell, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));
    let item = shown(&shell, &root, TICKET);
    assert!(item.body.as_deref() == Some(mine.as_str()), "byte for byte");
    // The tab's read after the save is taken without loading it again.
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    assert!(edit_text(&host, cx) == mine, "the read after it: exactly");
    assert_eq!(pill(&shell, cx).as_deref(), Some(EDITING_SAVED), "clean");
}

// Task 19's decisions (carried obligations 2 and 3).

/// Task 16's N2, on the read after a save: another actor's body, written
/// between our commit and the read, is reloaded into the clean editor. Its
/// base takes that body and stays unverified: the session reads clean, and
/// the next Save opens the review, which compares my typing with their
/// body alone, no false conflict.
#[gpui_kit::test]
async fn a_body_reloaded_after_a_save_reads_clean_and_the_next_save_reviews(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let (shell, root) = with_items(cx);
    open(&shell, document(&root), cx);
    let host = edit(&shell, cx).await;
    type_at_end(&shell, &host, "x", cx);
    let read = shell.backend.hold_read(ReadKind::DraftItem);
    shell.click("item-save", cx);
    let request = saves(&shell, cx)[0];
    shell.until_ended(request, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));
    their_document(&shell, &root, "One", THEIR_BODY);
    read.send(()).unwrap();
    shell.run(cx);
    shell.frame(cx, |_, _| ());

    assert!(edit_text(&host, cx) == THEIR_BODY, "their body is reloaded");
    assert_eq!(pill(&shell, cx).as_deref(), Some(EDITING_SAVED), "clean");
    let base = session_base(&shell, cx);
    assert!(base.observation.is_none(), "the base stays unverified");
    assert!(body_of(&base.fields) == THEIR_BODY, "with their body");
    assert_eq!(
        shell.label("item-save", cx).as_deref(),
        Some("Review before saving")
    );

    type_at_end(&shell, &host, "y", cx);
    let sent = saves(&shell, cx).len();
    shell.click("item-save", cx);
    assert_eq!(saves(&shell, cx).len(), sent, "nothing is sent");
    until_review(&shell, cx).await;
    assert_eq!(state(&shell, "Body", cx).as_deref(), Some(KEEPING_MINE));
    shell.click("review-use", cx);
    let request = save(&shell, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));
    let item = shown(&shell, &root, DOCUMENT);
    assert!(item.body.as_deref() == Some(format!("{THEIR_BODY}y").as_str()));
}

/// Task 18's M3, on a late read: a clean session on an unverified base
/// takes another actor's newer body the same way. It reads clean, Save
/// reads "Review before saving", and the review defaults their title and
/// my typing.
#[gpui_kit::test]
async fn a_late_body_on_an_unverified_base_reads_clean_and_the_next_save_reviews(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let (shell, root) = with_items(cx);
    open(&shell, document(&root), cx);
    let host = edit(&shell, cx).await;
    type_at_end(&shell, &host, "x", cx);
    let saved = edit_text(&host, cx);
    let read = shell.backend.hold_read(ReadKind::DraftItem);
    shell.click("item-save", cx);
    let request = saves(&shell, cx)[0];
    shell.until_ended(request, cx).await;
    their_document(&shell, &root, "Their title", &saved);
    read.send(()).unwrap();
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    assert!(session_base(&shell, cx).observation.is_none(), "unverified");
    assert_eq!(pill(&shell, cx).as_deref(), Some(EDITING_SAVED));

    their_document(&shell, &root, "Their title", THEIR_BODY);
    read_again(&shell, cx);
    assert!(
        edit_text(&host, cx) == THEIR_BODY,
        "the late body is loaded"
    );
    assert_eq!(pill(&shell, cx).as_deref(), Some(EDITING_SAVED), "clean");
    assert!(session_base(&shell, cx).observation.is_none());
    assert_eq!(
        shell.label("item-save", cx).as_deref(),
        Some("Review before saving")
    );

    type_at_end(&shell, &host, "y", cx);
    shell.click("item-save", cx);
    until_review(&shell, cx).await;
    assert_eq!(state(&shell, "Title", cx).as_deref(), Some(TAKING_THEIRS));
    assert_eq!(state(&shell, "Body", cx).as_deref(), Some(KEEPING_MINE));
    shell.click("review-use", cx);
    let request = save(&shell, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));
    let item = shown(&shell, &root, DOCUMENT);
    assert_eq!(item.title.as_deref(), Some("Their title"));
    assert!(item.body.as_deref() == Some(format!("{THEIR_BODY}y").as_str()));
}

/// Task 18's M4: the close dialog's Save leads to the review. The dialog
/// stays open, with the change words and how to reach the review; Cancel
/// shows it, and the tab stays.
#[gpui_kit::test]
async fn the_close_dialogs_save_into_the_review_says_cancel_to_review(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = with_items(cx);
    edited_ticket(&shell, &root, cx).await;
    shell.fixture.save_ticket(&root, TICKET, "Theirs");

    shell.click(("tab-close", 0usize), cx);
    assert!(shell.present("close-draft-dialog", cx));
    shell.click("close-draft-save-button", cx);
    let request = saves(&shell, cx)[0];
    ended(&shell, request, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::ExternalChange));
    assert_eq!(phase(&shell, cx), Phase::Review);
    assert!(shell.present("close-draft-dialog", cx), "the dialog stays");
    assert_eq!(
        shell.label("close-draft-save-state", cx).as_deref(),
        Some(CANCEL_TO_REVIEW)
    );

    shell.click("close-draft-cancel-button", cx);
    assert!(!shell.present("close-draft-dialog", cx));
    until_review(&shell, cx).await;
    assert!(shell.present("item-review", cx), "the review is in view");
    assert!(shell.present("item-view", cx), "the tab stays");
}

// Task 19 fix round 1.

/// Opens the ticket's change banner: a dirty session, another actor's
/// retitle, and a refresh that classifies the draft `Changed`.
async fn change_banner(shell: &Shell, root: &Path, cx: &mut TestAppContext) -> Entity<EditorHost> {
    let host = edited_ticket(shell, root, cx).await;
    shell.fixture.save_ticket(root, TICKET, "Theirs");
    read_again(shell, cx);
    assert!(
        shell.present("item-save-compare", cx),
        "Compare… is offered"
    );
    host
}

/// I1: the review's read is superseded by a newer read of the tab before
/// it lands (the workspace's reload after a request, a list's Retry). The
/// review takes the newer read and builds; while it reads, Cancel is
/// offered.
#[gpui_kit::test]
async fn a_reload_while_the_review_reads_does_not_strand_it(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = with_items(cx);
    change_banner(&shell, &root, cx).await;

    let first = shell.backend.hold_read(ReadKind::Item);
    shell.click("item-save-compare", cx);
    assert_eq!(phase(&shell, cx), Phase::Review);
    assert!(shell.present("review-status", cx), "it reads");
    assert!(shell.present("review-cancel", cx), "Cancel while it reads");
    read_again(&shell, cx);
    until_review(&shell, cx).await;
    assert_eq!(value(&shell, "review-theirs", "Title", cx), "Theirs");
    drop(first);
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    assert!(shell.present("review-use", cx), "the review stays built");
}

/// I1: Cancel while the review reads goes back to editing, mine kept, and
/// the feed resumes; the read landing after it opens nothing.
#[gpui_kit::test]
async fn cancel_while_the_review_reads_goes_back_to_editing(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = with_items(cx);
    let host = change_banner(&shell, &root, cx).await;
    let mine = edit_text(&host, cx);
    let id = draft_of(&shell, &ticket(&root, TICKET), cx).expect("a draft");

    let held = shell.backend.hold_read(ReadKind::Item);
    shell.click("item-save-compare", cx);
    shell.click("review-cancel", cx);
    assert_eq!(phase(&shell, cx), Phase::Editing);
    assert!(edit_text(&host, cx) == mine, "mine stays");
    drop(held);
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    assert!(
        !shell.present("item-review", cx),
        "the late read opens nothing"
    );
    assert_eq!(phase(&shell, cx), Phase::Editing);

    type_at_end(&shell, &host, "w", cx);
    idle(&shell, cx);
    assert!(drafted_body(&shell, id, cx) == format!("{mine}w"));
    let store = shell.fixture.draft_store();
    assert!(
        stored_body(&store, id).as_deref() == Some(format!("{mine}w").as_str()),
        "the feed resumed"
    );
}

/// I2 and M1: a pending checkpoint keeps its save for Retry. Its file is
/// written but not committed, so the tab's next read has a new token, and
/// with text typed since, D2 would classify its draft `Changed`: our own
/// write, not someone else's. (Another actor cannot commit meanwhile: the
/// library answers `recovery_required`.) While the save is kept, no change
/// banner shows, Compare… is not offered, and the review does not open.
#[gpui_kit::test]
async fn no_review_opens_over_a_kept_pending_checkpoint(cx: &mut TestAppContext) {
    use manyhands::repository::FailurePoint;

    cx.executor().allow_parking();
    let (shell, root) = with_items(cx);
    let host = edited_ticket(&shell, &root, cx).await;
    shell.backend.replace_service(
        RepositoryService::open_at_with_failure_point_for_testing(
            shell.fixture.data_dir(),
            FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence,
        )
        .unwrap(),
    );
    save(&shell, cx).await;
    assert!(shell.present("item-save-retry", cx), "the save is kept");
    shell
        .backend
        .replace_service(RepositoryService::open_at(shell.fixture.data_dir()).unwrap());
    type_at_end(&shell, &host, "y", cx);
    idle(&shell, cx);
    read_again(&shell, cx);

    assert!(!shell.present("item-save-compare", cx), "no Compare…");
    assert!(!shell.present("item-save-changed", cx), "no change banner");
    pane(&shell, cx).update(cx, |pane, cx| pane.open_review(cx));
    shell.run(cx);
    assert_eq!(phase(&shell, cx), Phase::Editing, "no review opens");
    assert!(!shell.present("item-review", cx));
    assert!(shell.present("item-save-retry", cx), "Retry stays");
}

/// M2: the close dialog opened while the review is open, not through the
/// dialog's own Save, says how to reach the review.
#[gpui_kit::test]
async fn closing_the_tab_during_the_review_says_cancel_to_review(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = with_items(cx);
    edited_ticket(&shell, &root, cx).await;
    shell.fixture.save_ticket(&root, TICKET, "Theirs");
    stale_save(&shell, cx).await;

    shell.click(("tab-close", 0usize), cx);
    assert!(shell.present("close-draft-dialog", cx));
    assert_eq!(
        shell.label("close-draft-save-state", cx).as_deref(),
        Some(CANCEL_TO_REVIEW)
    );
    shell.click("close-draft-cancel-button", cx);
    assert!(!shell.present("close-draft-dialog", cx));
    assert!(shell.present("item-review", cx), "the review is in view");
}

/// M3: choices that change nothing (theirs taken everywhere, no edit)
/// leave a clean session and no draft: closing the tab asks nothing, and
/// the draft is gone.
#[gpui_kit::test]
async fn choices_that_change_nothing_leave_no_draft_and_close_at_once(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = with_items(cx);
    let host = edited_document(&shell, &root, cx).await;
    let id = draft_of(&shell, &document(&root), cx).expect("a draft");
    their_document(&shell, &root, "One", THEIR_BODY);
    stale_save(&shell, cx).await;
    take_theirs(&shell, "Body", cx);
    shell.click("review-use", cx);
    idle(&shell, cx);
    assert!(edit_text(&host, cx) == THEIR_BODY);
    assert_eq!(pill(&shell, cx).as_deref(), Some(EDITING_SAVED), "clean");

    shell.click(("tab-close", 0usize), cx);
    assert!(!shell.present("close-draft-dialog", cx), "it asks nothing");
    assert!(!shell.present("item-view", cx), "the tab closed");
    shell.run(cx);
    let store = shell.fixture.draft_store();
    assert!(held(&store, id).is_none(), "the draft is discarded");
}

/// M4, R-18: Use these choices restarts undo: undo does not go back past
/// the result.
#[gpui_kit::test]
async fn undo_after_using_the_choices_does_not_go_back_past_the_result(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = with_items(cx);
    let host = edited_document(&shell, &root, cx).await;
    their_document(&shell, &root, "One", THEIR_BODY);
    stale_save(&shell, cx).await;
    take_theirs(&shell, "Body", cx);
    type_in_result(&shell, "z", cx);
    shell.click("review-use", cx);
    let result = format!("{THEIR_BODY}z");
    assert!(edit_text(&host, cx) == result);

    let editor = host_editor(&host, cx);
    let end = text(&editor, cx).len();
    focus_at(&shell, &editor, end, cx);
    shell.keys("secondary-z", cx);
    assert!(
        edit_text(&host, cx) == result,
        "undo restarted at the result"
    );
}

/// M4: Use these choices writes the draft's file, its fields and its new
/// base, not only D2's memory.
#[gpui_kit::test]
async fn using_the_choices_writes_the_draft_file(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = with_items(cx);
    edited_document(&shell, &root, cx).await;
    let id = draft_of(&shell, &document(&root), cx).expect("a draft");
    let bytes = draft_bytes(&shell, id);
    their_document(&shell, &root, "One", THEIR_BODY);
    stale_save(&shell, cx).await;
    take_theirs(&shell, "Body", cx);
    type_in_result(&shell, "z", cx);
    shell.click("review-use", cx);
    idle(&shell, cx);

    assert!(draft_bytes(&shell, id) != bytes, "the file is written");
    let store = shell.fixture.draft_store();
    let result = format!("{THEIR_BODY}z");
    assert!(stored_body(&store, id).as_deref() == Some(result.as_str()));
    let stored = store
        .list()
        .unwrap()
        .into_iter()
        .find(|listing| listing.id == id)
        .and_then(|listing| match listing.entry {
            manyhands::drafts::Entry::Draft(draft) => draft.base,
            manyhands::drafts::Entry::Unreadable(_) => None,
        })
        .expect("the stored draft's base");
    assert_eq!(
        stored.observation,
        shown(&shell, &root, DOCUMENT).observation,
        "its base is the current version"
    );
}
