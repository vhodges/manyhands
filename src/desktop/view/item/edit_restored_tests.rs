//! Restored drafts and late results, over real repositories (design,
//! Restored drafts and the close dialog; Late results; R-19, R-42, R-70,
//! R-79): Continue editing from the draft banner and Save from D2's
//! restored section take the restored-draft route; the base takes the
//! context's token only when the source matches, and otherwise the review
//! opens; a draft's text goes in exactly; and a late read never replaces a
//! dirty or drafted editor, while a clean one reloads only a newer body,
//! keeping its caret. The close dialog's Save is `tabs_tests`'s.
//!
//! The helpers are `edit_tests`'s. Each test sets the caret, focuses the
//! editor and draws a frame before typing, and types through the window;
//! each waits on the worker and allows parking. Draft fields and items
//! would print text through `Debug`, so they are compared with `assert!`.
//! The words are literals: the tests fix them.

use manyhands::{
    drafts::ContextKind,
    repository::{
        CancellationToken, Mutation, MutationCall, NoProgress, RelationshipWrite, RequestId,
        TicketDraft, TicketSaveInput, keys::SessionCredentials,
    },
};

use super::*;
use crate::desktop::{services::prompts::PromptReply, test_support::fixtures::NoPassphrase};

const REVIEWING: &str = "Reviewing changes";
/// Fact 81's first shape: a second load of what the first one gives would
/// change it again.
const FACT_81: &str = "wer $$\na $$b$$ c\n$$ wer\n";

/// A repository `a` holding the ticket `TICKET` titled `title`, with a
/// stored draft of it whose body is `body`, in an open window.
fn with_stored_draft(
    cx: &mut TestAppContext,
    title: &str,
    body: &str,
) -> (Shell, PathBuf, DraftId) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, title, "");
    fixture.refresh(&root);
    let stored = store_draft(&fixture, &root, ItemDtoKind::Ticket, TICKET, body);
    (Shell::open(fixture, cx), root, stored)
}

/// The save requests of this session, newest first.
fn saves(shell: &Shell, cx: &mut TestAppContext) -> Vec<DesktopRequestId> {
    shell.requests(cx, |request| {
        matches!(request.intent, Intent::TicketSave { .. })
    })
}

fn edit_text(host: &Entity<EditorHost>, cx: &mut TestAppContext) -> String {
    host.read_with(cx, |host, cx| host.text(cx))
}

/// The base the shown tab's edit session holds.
fn session_base(shell: &Shell, cx: &mut TestAppContext) -> manyhands::drafts::DraftBase {
    pane(shell, cx).read_with(cx, |pane, _| {
        pane.edit_session().expect("a session").base().clone()
    })
}

fn phase(shell: &Shell, cx: &mut TestAppContext) -> Phase {
    pane(shell, cx).read_with(cx, |pane, _| pane.edit_phase())
}

/// Presses `button` and waits until the save it leads to has been sent
/// and has ended, with frames drawn meanwhile: the route opens the editor
/// as a frame is drawn.
async fn saved_after(
    shell: &Shell,
    button: &'static str,
    cx: &mut TestAppContext,
) -> DesktopRequestId {
    let before = saves(shell, cx).len();
    shell.click(button, cx);
    let deadline = Instant::now() + WAIT;
    loop {
        shell.frame(cx, |_, _| ());
        shell.run(cx);
        let sent = saves(shell, cx);
        if sent.len() > before {
            ended(shell, sent[0], cx).await;
            return sent[0];
        }
        assert!(Instant::now() < deadline, "no save was sent");
        cx.executor().timer(Duration::from_millis(10)).await;
    }
}

/// Another actor saves the ticket `id` with `title` and `body`, over the
/// read it makes now.
fn theirs(shell: &Shell, root: &Path, id: &str, title: &str, body: &str) {
    let service = shell.fixture.service();
    let repo = service.resolve_repository(root).unwrap();
    let item = service.show_item(&repo, &item_id(id)).unwrap();
    let saved = service.execute(
        MutationCall {
            request_id: RequestId::new(),
            confirmation: None,
            mutation: Mutation::TicketSave(TicketSaveInput {
                root: root.to_path_buf(),
                id: item_id(id),
                draft: TicketDraft {
                    title: title.to_owned(),
                    ticket_type: item.ticket_type.clone().unwrap(),
                    status: item.status.clone().unwrap(),
                    project: item.project.clone(),
                    team: item.team.clone(),
                    body: body.to_owned(),
                },
                deps: RelationshipWrite::Unchanged,
                parent: RelationshipWrite::Unchanged,
                observation: item.observation.clone().unwrap(),
            }),
        },
        &mut SessionCredentials::new(NoPassphrase),
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(saved.code, ResultCode::Ok, "{}", saved.message);
}

/// Reads the shown tab again, as a refresh does, and lets the read land.
fn read_again(shell: &Shell, cx: &mut TestAppContext) {
    pane(shell, cx).update(cx, |pane, cx| {
        pane.reload(cx);
    });
    shell.run(cx);
    shell.frame(cx, |_, _| ());
}

/// Undoes in `host`'s editor, at most three times, until it holds
/// `before`: whether undo reaches it.
fn undo_reaches(
    shell: &Shell,
    host: &Entity<EditorHost>,
    before: &str,
    cx: &mut TestAppContext,
) -> bool {
    let editor = host_editor(host, cx);
    let end = text(&editor, cx).len();
    focus_at(shell, &editor, end, cx);
    for _ in 0..3 {
        shell.keys("secondary-z", cx);
        if text(&editor, cx) == before {
            return true;
        }
    }
    false
}

/// Continue editing from the draft banner: the stored draft is claimed into
/// the tab, the context is prepared, and the edit editor holds the draft's
/// text, dirty, over the draft's own base, with the read's token (R-42).
/// Save commits the draft's text, and the draft retires.
#[gpui_kit::test]
async fn continue_editing_from_the_banner_loads_the_drafts_text_dirty_on_its_own_base_and_saves(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let (shell, root, stored) = with_stored_draft(cx, "Continued", "Draft body");
    open(&shell, ticket(&root, TICKET), cx);
    assert_eq!(
        shell.label("draft-banner-continue", cx).as_deref(),
        Some("Continue editing")
    );
    let original = shown(&shell, &root, TICKET).body.unwrap();

    shell.click("draft-banner-continue-button", cx);
    let host = until_editing(&shell, cx).await;
    assert_eq!(prepares(&shell, cx).len(), 1, "the context was prepared");
    assert_eq!(attached(&shell, &root, TICKET, cx), Some(stored));
    assert!(edit_text(&host, cx) == "Draft body", "the draft's text");
    assert_eq!(pill(&shell, cx).as_deref(), Some(EDITING_UNSAVED));
    assert_eq!(phase(&shell, cx), Phase::Editing);
    let base = session_base(&shell, cx);
    assert!(
        body_of(&base.fields) == original,
        "the draft's own base, not its text"
    );
    assert_eq!(
        base.observation,
        shown(&shell, &root, TICKET).observation,
        "with the read's token"
    );

    let request = saved_after(&shell, "item-save", cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));
    assert!(shown(&shell, &root, TICKET).body.as_deref() == Some("Draft body"));
    assert_eq!(
        attached(&shell, &root, TICKET, cx),
        None,
        "the draft retired"
    );
    assert!(stored_bodies(&shell).is_empty(), "nothing is stored");
    assert_eq!(pill(&shell, cx).as_deref(), Some(EDITING_SAVED));
}

/// R-79: a draft's text is text an editor already loaded, so it goes into
/// the edit editor exactly, though a second load would change it (fact
/// 81). The caret starts at the start, the texts differing (R-65).
#[gpui_kit::test]
async fn continue_editing_puts_a_drafts_text_in_exactly(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let drafted = load(FACT_81);
    assert!(load(&drafted) != drafted, "a second load would change it");
    let (shell, root, stored) = with_stored_draft(cx, "Exact", &drafted);
    open(&shell, ticket(&root, TICKET), cx);

    shell.click("draft-banner-continue-button", cx);
    let host = until_editing(&shell, cx).await;
    assert!(edit_text(&host, cx) == drafted, "byte for byte");
    assert_eq!(cursor(&host_editor(&host, cx), cx), 0);
    shell.run(cx);
    assert!(
        stored_body(&shell.fixture.draft_store(), stored).as_deref() == Some(drafted.as_str()),
        "the stored draft is unchanged"
    );
}

/// R-42: a draft based on the primary copy. After Continue editing makes
/// the item's own context, only the branch changed the token: the source
/// and fields match, so the base takes the context's token and context,
/// in the session and in D2's draft, and Save commits with no review.
#[gpui_kit::test]
async fn continue_editing_a_draft_based_on_primary_takes_the_contexts_token_when_the_source_matches(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let (shell, root, stored) = with_stored_draft(cx, "Primary based", "Draft body");
    let primary = shown(&shell, &root, TICKET);
    assert_eq!(
        primary.context.kind,
        manyhands::repository::ItemContextKind::Primary
    );
    open(&shell, ticket(&root, TICKET), cx);

    shell.click("draft-banner-continue-button", cx);
    until_editing(&shell, cx).await;
    let read = shown(&shell, &root, TICKET);
    assert_eq!(
        read.context.kind,
        manyhands::repository::ItemContextKind::Active
    );
    assert!(read.source == primary.source, "the same source");
    assert!(read.observation != primary.observation, "a new token");
    let base = session_base(&shell, cx);
    assert_eq!(base.observation, read.observation, "the context's token");
    assert_eq!(base.context.kind, ContextKind::Active);
    assert_eq!(base.context.branch, read.context.branch);
    let drafted = registry(&shell, cx).read_with(cx, |registry, _| {
        registry
            .session(stored)
            .and_then(|session| session.base().cloned())
    });
    let drafted = drafted.expect("D2's draft has a base");
    assert_eq!(drafted.observation, read.observation, "in D2's draft too");
    assert_eq!(drafted.context.kind, ContextKind::Active);

    let request = saved_after(&shell, "item-save", cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok), "no review");
    assert_eq!(phase(&shell, cx), Phase::Editing);
}

/// R-42: the item was saved by another actor since the draft's base, so
/// the read's source differs: the review opens, and nothing is written.
#[gpui_kit::test]
async fn continue_editing_opens_the_review_when_the_source_differs(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root, stored) = with_stored_draft(cx, "Mine", "Draft body");
    let based = shown(&shell, &root, TICKET);
    shell.fixture.save_ticket(&root, TICKET, "Theirs");
    open(&shell, ticket(&root, TICKET), cx);

    shell.click("draft-banner-continue-button", cx);
    let host = until_editing(&shell, cx).await;
    shell.run(cx);
    assert_eq!(phase(&shell, cx), Phase::Review, "the review opens");
    assert_eq!(pill(&shell, cx).as_deref(), Some(REVIEWING));
    assert!(edit_text(&host, cx) == "Draft body", "the draft's text");
    assert!(saves(&shell, cx).is_empty(), "nothing is saved");
    let item = shown(&shell, &root, TICKET);
    assert_eq!(item.title.as_deref(), Some("Theirs"));
    assert!(item.body == based.body, "nothing is written");
    let drafted = registry(&shell, cx).read_with(cx, |registry, _| {
        registry
            .session(stored)
            .and_then(|session| session.base().cloned())
    });
    let drafted = drafted.expect("D2's draft has a base");
    assert_eq!(
        drafted.observation, based.observation,
        "the draft's base keeps its token"
    );
}

/// Save from D2's restored section runs Continue editing's steps first: the
/// context is prepared before the save is sent, and the save commits the
/// draft's text.
#[gpui_kit::test]
async fn save_from_a_restored_draft_prepares_the_context_first(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root, stored) = with_stored_draft(cx, "Saved from Drafts", "Draft body");
    open(&shell, ticket(&root, TICKET), cx);
    shell.click("draft-restore-button", cx);
    assert_eq!(attached(&shell, &root, TICKET, cx), Some(stored));
    assert!(prepares(&shell, cx).is_empty(), "Restore prepares nothing");

    let request = saved_after(&shell, "draft-save-button", cx).await;
    let prepared = prepares(&shell, cx);
    assert_eq!(prepared.len(), 1);
    assert!(prepared[0] < request, "prepared before the save was sent");
    assert_eq!(code(&shell, prepared[0], cx), Some(ResultCode::Ok));
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));
    assert!(shown(&shell, &root, TICKET).body.as_deref() == Some("Draft body"));
    assert_eq!(
        attached(&shell, &root, TICKET, cx),
        None,
        "the draft retired"
    );
}

/// Review Focus 2: typing during a save while a tab read lands. The save
/// is held; newer text is typed; the tab's read is held, then lands once
/// the save has committed. The newer text stays in the editor and in the
/// draft, and only the saved revision retires.
#[gpui_kit::test]
async fn typing_during_a_save_while_a_tab_read_lands_keeps_the_newer_text(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Focus two", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let original = edit_text(&host, cx);
    type_at_end(&shell, &host, "x", cx);

    let held = shell.backend.hold_job(JobKind::TicketSave);
    shell.click("item-save", cx);
    held.wait_reached();
    shell.run(cx);
    type_at_end(&shell, &host, "y", cx);
    let read = shell.backend.hold_read(ReadKind::Item);
    pane(&shell, cx).update(cx, |pane, cx| {
        pane.reload(cx);
    });
    shell.run(cx);
    held.release();
    let request = saves(&shell, cx)[0];
    ended(&shell, request, cx).await;
    read.send(()).ok();
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    cx.executor().advance_clock(IDLE);
    shell.run(cx);

    assert!(shown(&shell, &root, TICKET).body.as_deref() == Some(format!("{original}x").as_str()));
    assert!(
        edit_text(&host, cx) == format!("{original}xy"),
        "the newer text stays in the editor"
    );
    assert_eq!(pill(&shell, cx).as_deref(), Some(EDITING_UNSAVED));
    let id = attached(&shell, &root, TICKET, cx).expect("the draft is not retired");
    assert!(
        stored_body(&shell.fixture.draft_store(), id) == Some(format!("{original}xy")),
        "and in the draft"
    );
}

/// A late read never replaces a dirty editor, nor a clean one whose draft
/// is live: another actor's body, read again, leaves the text and its undo
/// history alone.
#[gpui_kit::test]
async fn a_late_read_does_not_replace_a_dirty_editor(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Late", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let original = edit_text(&host, cx);
    type_at_end(&shell, &host, "x", cx);
    assert!(attached(&shell, &root, TICKET, cx).is_some(), "drafted");

    theirs(&shell, &root, TICKET, "Late", "Their body.\n");
    read_again(&shell, cx);
    assert!(
        shown(&shell, &root, TICKET).body.as_deref() == Some("Their body.\n"),
        "the read holds theirs"
    );
    assert!(
        edit_text(&host, cx) == format!("{original}x"),
        "the dirty editor keeps its text"
    );

    // Undone back to the base: clean, but its draft is live.
    assert!(undo_reaches(&shell, &host, &original, cx), "undo is kept");
    assert_eq!(pill(&shell, cx).as_deref(), Some(EDITING_SAVED));
    assert!(
        attached(&shell, &root, TICKET, cx).is_some(),
        "still drafted"
    );
    theirs(&shell, &root, TICKET, "Late", "Their second body.\n");
    read_again(&shell, cx);
    assert!(
        edit_text(&host, cx) == original,
        "a drafted editor keeps its text"
    );
}

/// While a session is kept after Done, a refresh reloads the read editor
/// from another actor's body, and shows the change banner; the kept edit
/// editor keeps its text.
#[gpui_kit::test]
async fn a_refresh_with_a_kept_session_reloads_only_the_read_editor(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Kept", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let read_view = read_editor(&shell, cx);
    let host = edit(&shell, cx).await;
    let original = edit_text(&host, cx);
    type_at_end(&shell, &host, "x", cx);
    shell.click("item-done", cx);
    shell.run(cx);
    let kept = pane(&shell, cx).read_with(cx, |pane, _| {
        pane.edit_session().map(|session| session.kept())
    });
    assert_eq!(kept, Some(true), "the session is kept");

    theirs(&shell, &root, TICKET, "Kept", "Their body.\n");
    read_again(&shell, cx);
    assert!(
        text(&read_view, cx) == "Their body.\n",
        "the read editor reloads"
    );
    assert!(
        edit_text(&host, cx) == format!("{original}x"),
        "the kept editor keeps its text"
    );
    assert_eq!(
        shell.label("item-kept-changed", cx).as_deref(),
        Some("Someone changed this file outside Manyhands. Your edits are kept.")
    );
}

/// Edits the shown tab, types "x" at the end and saves it: the session is
/// clean, its draft retired. Gives the host and the text before the save.
async fn saved_clean(
    shell: &Shell,
    root: &Path,
    id: &str,
    cx: &mut TestAppContext,
) -> (Entity<EditorHost>, String) {
    let host = edit(shell, cx).await;
    let original = edit_text(&host, cx);
    type_at_end(shell, &host, "x", cx);
    let request = saved_after(shell, "item-save", cx).await;
    assert_eq!(code(shell, request, cx), Some(ResultCode::Ok));
    assert_eq!(attached(shell, root, id, cx), None, "the draft retired");
    assert_eq!(pill(shell, cx).as_deref(), Some(EDITING_SAVED));
    (host, original)
}

/// R-19, R-70: a clean edit editor (saved, its draft retired) read again.
/// A new token over an equal body is adopted without `set_text`, so undo
/// still reaches the text before the save; a newer body is loaded, the
/// caret kept, and the session stays clean on it.
#[gpui_kit::test]
async fn a_clean_editor_reloads_only_on_a_newer_body_and_keeps_its_cursor(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Clean", "");
        fixture.commit_ticket(root, OTHER, "Newer", "");
    });

    // Another actor retitles it: a new token over the same body.
    open(&shell, ticket(&root, TICKET), cx);
    let (host, original) = saved_clean(&shell, &root, TICKET, cx).await;
    let saved = format!("{original}x");
    theirs(&shell, &root, TICKET, "Retitled", &saved);
    read_again(&shell, cx);
    let item = shown(&shell, &root, TICKET);
    assert_eq!(item.title.as_deref(), Some("Retitled"));
    assert!(edit_text(&host, cx) == saved, "the text is unchanged");
    assert_eq!(
        session_base(&shell, cx).observation,
        item.observation,
        "the token is adopted"
    );
    assert_eq!(pill(&shell, cx).as_deref(), Some(EDITING_SAVED), "clean");
    assert!(
        undo_reaches(&shell, &host, &original, cx),
        "no set_text: undo is kept"
    );

    // Another actor writes a newer body.
    open(&shell, ticket(&root, OTHER), cx);
    let (host, _) = saved_clean(&shell, &root, OTHER, cx).await;
    let editor = host_editor(&host, cx);
    editor.update(cx, |editor, cx| editor.set_cursor(3, cx));
    theirs(&shell, &root, OTHER, "Newer", "Their newer body.\n");
    read_again(&shell, cx);
    let item = shown(&shell, &root, OTHER);
    assert!(edit_text(&host, cx) == "Their newer body.\n", "reloaded");
    assert_eq!(cursor(&editor, cx), 3, "the caret is kept");
    assert_eq!(session_base(&shell, cx).observation, item.observation);
    assert_eq!(
        pill(&shell, cx).as_deref(),
        Some(EDITING_SAVED),
        "clean on it"
    );
}

/// I1 (Task 18 fix 1): with crash recovery off, a save that newer typing
/// outlived leaves the editor's own flag apart from the session's. The
/// typing is deleted, so the session is clean on the saved copy, while the
/// editor's flag still reads dirty. Another actor then commits, and a tab
/// read lands: the editor takes their body, the session reads clean on
/// their token, and Save has nothing to send, so their change is never
/// overwritten without the review (contracts 7 and 8).
#[gpui_kit::test]
async fn a_late_read_reloads_a_clean_editor_whatever_its_own_flag_says(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    // The drafts folder is a file: the store does not open.
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "Recovery off", "");
    fixture.refresh(&root);
    std::fs::write(fixture.data_dir().join("drafts"), b"not a folder").unwrap();
    let shell = Shell::open(fixture, cx);
    open(&shell, ticket(&root, TICKET), cx);
    shell.click("item-edit", cx);
    shell.click("recovery-off-continue", cx);
    let host = until_editing(&shell, cx).await;
    let original = edit_text(&host, cx);
    type_at_end(&shell, &host, "a", cx);

    let held = shell.backend.hold_job(JobKind::TicketSave);
    shell.click("item-save", cx);
    held.wait_reached();
    shell.run(cx);
    type_at_end(&shell, &host, "b", cx);
    held.release();
    let request = saves(&shell, cx)[0];
    ended(&shell, request, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));
    let saved = format!("{original}a");
    assert!(shown(&shell, &root, TICKET).body.as_deref() == Some(saved.as_str()));

    // The typed "b" deleted: the session is clean on the saved copy, while
    // the editor's own baseline is still the text before the save.
    let editor = host_editor(&host, cx);
    let end = text(&editor, cx).len();
    focus_at(&shell, &editor, end, cx);
    shell.keys("backspace", cx);
    assert!(edit_text(&host, cx) == saved);
    assert_eq!(pill(&shell, cx).as_deref(), Some(EDITING_SAVED), "clean");
    assert!(
        host.read_with(cx, |host, _| host.is_dirty()),
        "the editor's own flag disagrees"
    );

    theirs(&shell, &root, TICKET, "Recovery off", "Their body.\n");
    read_again(&shell, cx);
    let item = shown(&shell, &root, TICKET);
    assert!(
        edit_text(&host, cx) == "Their body.\n",
        "the editor holds theirs"
    );
    assert_eq!(
        pill(&shell, cx).as_deref(),
        Some(EDITING_SAVED),
        "clean on it"
    );
    assert_eq!(session_base(&shell, cx).observation, item.observation);
    let sent = saves(&shell, cx).len();
    shell.click("item-save", cx);
    assert_eq!(saves(&shell, cx).len(), sent, "Save has nothing to send");
    assert!(
        shown(&shell, &root, TICKET).body.as_deref() == Some("Their body.\n"),
        "their change stands"
    );
}

/// M7 (Task 18 fix 1): `e` on the page, with a stored draft, takes the
/// restored-draft route, not D2's Restore: the draft is claimed into the
/// tab, then the context is prepared, and the editor opens on the draft's
/// text.
#[gpui_kit::test]
async fn e_with_a_stored_draft_takes_the_restored_draft_route(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root, stored) = with_stored_draft(cx, "Keyed", "Draft body");
    open(&shell, ticket(&root, TICKET), cx);
    let read = read_editor(&shell, cx);
    focus_at(&shell, &read, 3, cx);
    assert!(
        in_page_context(&shell, cx),
        "the read editor is on the page"
    );

    shell.keys("e", cx);
    let host = until_editing(&shell, cx).await;
    assert_eq!(attached(&shell, &root, TICKET, cx), Some(stored), "claimed");
    assert_eq!(prepares(&shell, cx).len(), 1, "then prepared");
    assert!(edit_text(&host, cx) == "Draft body", "the draft's text");
}

/// Part C fix 1, M3: a stored draft of the item appears while a clean
/// session edits, here another process's. D2's banner says it is kept,
/// with no Restore above the live session, and Restore, pressed anyway,
/// claims nothing: no second session is attached to the tab, and no claim
/// is left.
#[gpui_kit::test]
async fn restore_is_not_offered_above_a_live_edit_session(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Edited", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let text = edit_text(&host, cx);
    let stored = store_draft(
        &shell.fixture,
        &root,
        ItemDtoKind::Ticket,
        TICKET,
        "Elsewhere",
    );
    registry(&shell, cx).update(cx, |registry, cx| {
        registry.reload_listing(cx);
    });
    shell.run(cx);
    shell.frame(cx, |_, _| ());

    assert!(
        shell.present("draft-banner", cx),
        "the banner says it is kept"
    );
    assert!(
        !shell.present("draft-restore", cx),
        "no Restore over the editor"
    );
    pane(&shell, cx).update(cx, |pane, cx| pane.restore(stored, cx));
    shell.run(cx);
    assert!(
        registry(&shell, cx).read_with(cx, |registry, _| registry.session(stored).is_none()),
        "nothing is claimed"
    );
    assert_eq!(attached(&shell, &root, TICKET, cx), None, "nor attached");
    assert!(matches!(
        held(&shell.fixture.draft_store(), stored),
        Some(Held::No)
    ));
    assert_eq!(phase(&shell, cx), Phase::Editing);
    assert!(edit_text(&host, cx) == text, "the session goes on");
}

/// Part C fix 2, D (R-35): a draft of the item, here another process's,
/// is listed on the Drafts page while a tab of this window edits the item,
/// with no draft of its own. The page's Restore is disabled, says why, and
/// pressing it claims nothing: no second session is attached to the tab,
/// and the page stays.
#[gpui_kit::test]
async fn the_drafts_pages_restore_is_disabled_over_a_live_edit_session(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Edited", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let text = edit_text(&host, cx);
    assert_eq!(
        attached(&shell, &root, TICKET, cx),
        None,
        "no draft of its own"
    );
    let stored = store_draft(
        &shell.fixture,
        &root,
        ItemDtoKind::Ticket,
        TICKET,
        "Elsewhere",
    );
    registry(&shell, cx).update(cx, |registry, cx| {
        registry.reload_listing(cx);
    });
    shell.run(cx);

    shell.keys("secondary-shift-d", cx);
    shell.run(cx);
    assert!(
        shell.present(row_id(stored, "restore"), cx),
        "Restore is shown"
    );
    shell.click(row_id(stored, "restore"), cx);
    shell.run(cx);
    assert!(
        registry(&shell, cx).read_with(cx, |registry, _| registry.session(stored).is_none()),
        "nothing is claimed"
    );
    assert_eq!(
        shell.label(row_id(stored, "restore-note"), cx).as_deref(),
        Some("Open in its tab to continue editing")
    );
    assert_eq!(attached(&shell, &root, TICKET, cx), None, "nor attached");
    assert!(matches!(
        held(&shell.fixture.draft_store(), stored),
        Some(Held::No)
    ));
    assert!(
        shell
            .workspace
            .read_with(cx, |workspace, cx| workspace.drafts_shown(cx)),
        "the page stays"
    );
    assert_eq!(phase(&shell, cx), Phase::Editing);
    assert!(edit_text(&host, cx) == text, "the session goes on");
}

// Task 27 fix 1.

/// A1: the close dialog of a restored draft, its Save pressed by key, stays
/// open while the draft is continued and saved. The editor that opens
/// beneath it takes no focus: focus stays in the dialog, and Escape is the
/// dialog's Cancel. The save goes on, and the tab stays.
#[gpui_kit::test]
async fn the_editor_opening_under_the_close_dialog_leaves_focus_in_it(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root, stored) = with_stored_draft(cx, "Closing", "Draft body");
    open(&shell, ticket(&root, TICKET), cx);
    shell.click("draft-restore-button", cx);
    assert_eq!(attached(&shell, &root, TICKET, cx), Some(stored));
    let held = shell.backend.hold_job(JobKind::TicketSave);
    shell.click(("tab-close", 0usize), cx);
    assert!(shell.present("close-draft-dialog", cx));
    shell.keys("tab", cx);
    shell.keys("tab", cx);
    assert!(
        holds_focus(&shell, "close-draft-save-button", cx),
        "Tab reaches Save"
    );
    shell.keys("enter", cx);
    let host = until_editing(&shell, cx).await;
    shell
        .until_drawn(cx, "the save sent", |shell, cx| {
            !saves(shell, cx).is_empty()
        })
        .await;
    held.wait_reached();

    assert!(shell.present("close-draft-dialog", cx), "it stays open");
    assert!(
        !focused(&shell, &host_editor(&host, cx), cx),
        "the editor takes no focus"
    );
    assert!(
        holds_focus(&shell, "close-draft-dialog", cx),
        "focus stays in the dialog"
    );
    shell.keys("escape", cx);
    assert!(
        !shell.present("close-draft-dialog", cx),
        "Escape is its Cancel"
    );

    held.release();
    let request = saves(&shell, cx)[0];
    ended(&shell, request, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));
    assert_eq!(
        shell
            .workspace
            .read_with(cx, |workspace, _| workspace.tab_keys())
            .len(),
        1,
        "Cancel kept the tab"
    );
}

/// The ticket `TICKET` edited, "x" typed and its draft begun, with its close
/// dialog's Save held on the worker. Gives the hold.
async fn close_save_held(
    shell: &Shell,
    root: &Path,
    cx: &mut TestAppContext,
) -> crate::desktop::test_support::delaying::JobGate {
    open(shell, ticket(root, TICKET), cx);
    let host = edit(shell, cx).await;
    type_at_end(shell, &host, "x", cx);
    assert!(attached(shell, root, TICKET, cx).is_some(), "a draft");
    let held = shell.backend.hold_job(JobKind::TicketSave);
    shell.click(("tab-close", 0usize), cx);
    assert!(shell.present("close-draft-dialog", cx));
    shell.click("close-draft-save-button", cx);
    held.wait_reached();
    held
}

fn tabs(shell: &Shell, cx: &mut TestAppContext) -> usize {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.tab_keys())
        .len()
}

/// A2: a credential prompt opens above the close dialog while its Save
/// runs. The save settles: the tab closes, as was chosen, but the dialog
/// closes only itself, and only while focus is in it, so the prompt stays,
/// unanswered. The close dialog, left over a closed tab, offers Cancel
/// alone until Escape closes it.
#[gpui_kit::test]
async fn a_settled_close_save_closes_no_dialog_above_its_own(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Closing", "");
    });
    let held = close_save_held(&shell, &root, cx).await;
    let mut answer = shell.prompt(cx);
    assert!(shell.present("passphrase-dialog", cx));

    held.release();
    shell
        .until_drawn(cx, "the tab closing", |shell, cx| tabs(shell, cx) == 0)
        .await;
    assert!(shell.present("passphrase-dialog", cx), "the prompt stays");
    assert!(matches!(answer.try_recv(), Ok(None)), "unanswered");
    assert!(shell.prompt_focused(cx), "with the focus");
    assert!(shell.present("close-draft-dialog", cx), "under it");

    shell.keys("escape", cx);
    assert!(matches!(
        answer.try_recv(),
        Ok(Some(PromptReply::Cancelled))
    ));
    assert!(!shell.present("passphrase-dialog", cx));
    assert!(shell.present("close-draft-dialog", cx));
    assert!(!shell.present("close-draft-keep", cx), "no Keep draft");
    assert!(shell.inert_button("close-draft-discard-button", cx));
    assert!(shell.inert_button("close-draft-save-button", cx));
    shell.keys("escape", cx);
    assert!(!shell.present("close-draft-dialog", cx), "Escape closes it");
    assert!(shell.fixture.draft_store().list().unwrap().is_empty());
}

/// A2: the close dialog goes without a choice while its Save runs, as
/// quitting closes every dialog. The save that settles then closes
/// nothing: not the dialog in front, nor the tab.
#[gpui_kit::test]
async fn a_close_dialog_gone_unanswered_closes_nothing_when_its_save_settles(
    cx: &mut TestAppContext,
) {
    use gpui_kit::component::WindowExt;

    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Not closing", "");
    });
    let held = close_save_held(&shell, &root, cx).await;
    shell.frame(cx, |window, cx| window.close_all_dialogs(cx));
    assert!(!shell.present("close-draft-dialog", cx));
    let mut answer = shell.prompt(cx);

    held.release();
    let request = saves(&shell, cx)[0];
    ended(&shell, request, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));
    shell
        .until_drawn(cx, "the draft retiring", |shell, cx| {
            attached(shell, &root, TICKET, cx).is_none()
        })
        .await;
    assert_eq!(tabs(&shell, cx), 1, "the tab stays");
    assert!(shell.present("passphrase-dialog", cx), "the prompt stays");
    assert!(matches!(answer.try_recv(), Ok(None)), "unanswered");
}

/// B3: a Discard that waits for the draft still beginning holds the tab's
/// session until that draft is gone: the Drafts page's Restore of another
/// draft of the item, here another process's, is disabled meanwhile, says
/// why, and claims nothing.
#[gpui_kit::test]
async fn the_drafts_pages_restore_waits_while_a_discard_waits_for_its_draft(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Discarding", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let begin = shell.backend.hold_read(ReadKind::Drafts);
    type_at_end(&shell, &host, "x", cx);
    assert_eq!(attached(&shell, &root, TICKET, cx), None, "still beginning");
    shell.click(("tab-close", 0usize), cx);
    shell.click("close-draft-discard-button", cx);
    assert!(
        pane(&shell, cx).read_with(cx, |pane, _| pane.edit_session().is_none()),
        "the session ended"
    );
    assert_eq!(tabs(&shell, cx), 1, "the Discard waits");

    let stored = store_draft(
        &shell.fixture,
        &root,
        ItemDtoKind::Ticket,
        TICKET,
        "Elsewhere",
    );
    registry(&shell, cx).update(cx, |registry, cx| {
        registry.reload_listing(cx);
    });
    shell.run(cx);
    shell.keys("secondary-shift-d", cx);
    shell.run(cx);
    assert!(shell.present(row_id(stored, "restore"), cx));
    shell.click(row_id(stored, "restore"), cx);
    shell.run(cx);
    assert!(
        registry(&shell, cx).read_with(cx, |registry, _| registry.session(stored).is_none()),
        "nothing is claimed"
    );
    assert_eq!(
        shell.label(row_id(stored, "restore-note"), cx).as_deref(),
        Some("Open in its tab to continue editing")
    );
    assert!(matches!(
        held(&shell.fixture.draft_store(), stored),
        Some(Held::No)
    ));
    drop(begin);
    shell.run(cx);
}

// Task 27 fix 2.

/// A repository `a` holding the ticket `TICKET`, "Child", and `OTHER`,
/// "Parent", and, when `cyclic`, `OTHER` a child of `TICKET`; with a stored
/// draft of `TICKET` whose Parent is `OTHER`, which the item's is not.
fn with_a_parent_drafted(cx: &mut TestAppContext, cyclic: bool) -> (Shell, PathBuf, DraftId) {
    use manyhands::drafts::{DraftBase, DraftTarget, ItemKind};

    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "Child", "");
    let extra = if cyclic {
        format!("parent: {TICKET}\n")
    } else {
        String::new()
    };
    fixture.commit_ticket(&root, OTHER, "Parent", &extra);
    fixture.refresh(&root);
    let service = fixture.service();
    let repo = service.resolve_repository(&root).unwrap();
    let shown = service.show_item(&repo, &item_id(TICKET)).unwrap();
    let mut current = DraftFields::of(&shown).expect("a conforming ticket");
    match &mut current {
        DraftFields::Ticket { parent, .. } => *parent = Some(OTHER.to_owned()),
        _ => unreachable!(),
    }
    let target = DraftTarget {
        repository: key(&root),
        item_kind: ItemKind::Ticket,
        item_id: item_id(TICKET),
        comment: None,
    };
    let stored = match fixture
        .draft_store()
        .create(target, DraftBase::of(&shown), current)
    {
        Ok(claim) => claim.draft().id,
        Err(error) => panic!("the draft must be stored: {error}"),
    };
    (Shell::open(fixture, cx), root, stored)
}

/// The ticket `TICKET`'s parent, as read now.
fn parent_of(shell: &Shell, root: &Path) -> Option<String> {
    shown(shell, root, TICKET).parent.map(|parent| parent.id)
}

/// Draws frames until the edit editor opens, then until what its opening
/// deferred has run.
async fn opened(shell: &Shell, cx: &mut TestAppContext) {
    until_editing(shell, cx).await;
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    shell.frame(cx, |_, _| ());
}

/// N1: the restored section's Save of a ticket draft whose Parent differs
/// from the item goes through Continue editing's steps, and the editor's
/// opening checks the relationships. The Save waits for that check, not
/// stopping on it: while the check is held nothing is sent; once it lands
/// with no rejection, the one save is sent and commits.
#[gpui_kit::test]
async fn a_restored_save_waits_for_the_opens_relationship_check(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root, stored) = with_a_parent_drafted(cx, false);
    open(&shell, ticket(&root, TICKET), cx);
    shell.click("draft-restore-button", cx);
    assert_eq!(attached(&shell, &root, TICKET, cx), Some(stored));
    let held = shell.backend.hold_read(ReadKind::RelationshipCheck);
    shell.click("draft-save-button", cx);
    opened(&shell, cx).await;
    assert!(saves(&shell, cx).is_empty(), "the save waits for the check");

    held.send(()).expect("the check is still running");
    shell
        .until_drawn(cx, "the save sent", |shell, cx| {
            !saves(shell, cx).is_empty()
        })
        .await;
    let sent = saves(&shell, cx);
    assert_eq!(sent.len(), 1, "one save");
    ended(&shell, sent[0], cx).await;
    assert_eq!(code(&shell, sent[0], cx), Some(ResultCode::Ok));
    assert_eq!(parent_of(&shell, &root).as_deref(), Some(OTHER));
}

/// N1: the close dialog's Save of the same restored draft, with no
/// session. While the open's check is held, nothing is sent and the dialog
/// stays open; once it lands, the save commits and the tab closes, with
/// its dialog.
#[gpui_kit::test]
async fn the_close_dialogs_restored_save_waits_for_the_opens_check(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root, _) = with_a_parent_drafted(cx, false);
    open(&shell, ticket(&root, TICKET), cx);
    shell.click("draft-restore-button", cx);
    shell.click(("tab-close", 0usize), cx);
    assert!(shell.present("close-draft-dialog", cx));
    let held = shell.backend.hold_read(ReadKind::RelationshipCheck);
    shell.click("close-draft-save-button", cx);
    opened(&shell, cx).await;
    assert!(saves(&shell, cx).is_empty(), "the save waits for the check");
    assert!(shell.present("close-draft-dialog", cx), "the dialog stays");

    held.send(()).expect("the check is still running");
    shell
        .until_drawn(cx, "the tab closing", |shell, cx| tabs(shell, cx) == 0)
        .await;
    let sent = saves(&shell, cx);
    assert_eq!(sent.len(), 1, "one save");
    assert_eq!(code(&shell, sent[0], cx), Some(ResultCode::Ok));
    assert!(
        !shell.present("close-draft-dialog", cx),
        "the dialog closed"
    );
    assert_eq!(parent_of(&shell, &root).as_deref(), Some(OTHER));
}

/// N1: the same, with a Parent that makes a cycle. Once the check lands,
/// its rejection blocks the save: nothing is sent, the toolbar and the
/// close dialog say the cycle's words, and the dialog stays open.
#[gpui_kit::test]
async fn a_restored_save_whose_check_finds_a_cycle_is_not_sent(cx: &mut TestAppContext) {
    use crate::desktop::model::relations_edit::SAVE_CYCLE;

    cx.executor().allow_parking();
    let (shell, root, _) = with_a_parent_drafted(cx, true);
    open(&shell, ticket(&root, TICKET), cx);
    shell.click("draft-restore-button", cx);
    shell.click(("tab-close", 0usize), cx);
    let held = shell.backend.hold_read(ReadKind::RelationshipCheck);
    shell.click("close-draft-save-button", cx);
    opened(&shell, cx).await;
    assert!(saves(&shell, cx).is_empty());

    held.send(()).expect("the check is still running");
    shell
        .until_drawn(cx, "the cycle said", |shell, cx| {
            shell.label("close-draft-save-state", cx).as_deref() == Some(SAVE_CYCLE)
        })
        .await;
    shell.run(cx);
    assert!(saves(&shell, cx).is_empty(), "nothing is sent");
    assert_eq!(
        shell.label("toolbar-status", cx).as_deref(),
        Some(SAVE_CYCLE)
    );
    assert!(shell.present("close-draft-dialog", cx), "the dialog stays");
    assert_eq!(tabs(&shell, cx), 1);
    assert_eq!(parent_of(&shell, &root), None, "nothing is written");
}

/// N1: the close dialog's Cancel while its Save waits for the open's check
/// ends that Save, as it ends one still in Continue editing's steps (Task
/// 18 fix 1, M1): once the check lands, nothing is sent, and the tab stays,
/// editing.
#[gpui_kit::test]
async fn cancel_while_a_restored_save_waits_for_the_check_sends_nothing(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root, _) = with_a_parent_drafted(cx, false);
    open(&shell, ticket(&root, TICKET), cx);
    shell.click("draft-restore-button", cx);
    shell.click(("tab-close", 0usize), cx);
    let held = shell.backend.hold_read(ReadKind::RelationshipCheck);
    shell.click("close-draft-save-button", cx);
    opened(&shell, cx).await;
    shell.click("close-draft-cancel-button", cx);
    assert!(!shell.present("close-draft-dialog", cx));

    held.send(()).expect("the check is still running");
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    shell.run(cx);
    assert!(
        saves(&shell, cx).is_empty(),
        "the cancelled Save is not sent"
    );
    assert_eq!(tabs(&shell, cx), 1, "the tab stays");
    assert_eq!(phase(&shell, cx), Phase::Editing);
    assert_eq!(parent_of(&shell, &root), None, "nothing is written");
}

/// N2: a tab whose Keep draft is still writing closes once the write lands,
/// and the next frame would focus the content region. Meanwhile another
/// tab is shown with its details sheet open: the refocus is dropped, so
/// focus stays in the sheet, and its Escape gives focus back to its chip.
#[gpui_kit::test]
async fn a_background_tab_closing_takes_no_focus_from_the_sheet(cx: &mut TestAppContext) {
    use gpui_kit::component::WindowExt;

    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Kept", "");
        fixture.commit_ticket(root, OTHER, "Shown", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    type_at_end(&shell, &host, "x", cx);
    assert!(attached(&shell, &root, TICKET, cx).is_some(), "a draft");
    type_at_end(&shell, &host, "y", cx);
    let write = shell.backend.hold_read(ReadKind::DraftWrite);
    shell.click(("tab-close", 0usize), cx);
    shell.click("close-draft-keep-button", cx);
    assert!(!shell.present("close-draft-dialog", cx));
    assert_eq!(tabs(&shell, cx), 1, "Keep draft waits for its write");

    open(&shell, ticket(&root, OTHER), cx);
    shell.click(ChipField::All.id(), cx);
    assert!(shell.frame(cx, |window, cx| window.has_active_sheet(cx)));
    shell.frame(cx, |_, _| ());
    assert!(holds_focus(&shell, "sheet", cx), "the sheet has the focus");

    drop(write);
    shell
        .until_drawn(cx, "the kept tab closing", |shell, cx| {
            tabs(shell, cx) == 1
                && shell
                    .workspace
                    .read_with(cx, |workspace, _| workspace.tab_keys())
                    .iter()
                    .all(|tab| *tab != TabKey::for_target(&ticket(&root, TICKET)))
        })
        .await;
    shell.frame(cx, |_, _| ());
    assert!(shell.frame(cx, |window, cx| window.has_active_sheet(cx)));
    assert!(holds_focus(&shell, "sheet", cx), "focus stays in the sheet");

    shell.keys("escape", cx);
    assert!(!shell.frame(cx, |window, cx| window.has_active_sheet(cx)));
    assert!(
        holds_focus(&shell, ChipField::All.id(), cx),
        "back on its chip"
    );
}

// Task 27 fix 3.

/// Fix 3 (ruling): a tab closed in the background once its Keep draft's
/// write lands held no focus, so focus stays where the user is: here B's
/// read editor. A shown tab closed the same way gives the content region
/// the focus, as before.
#[gpui_kit::test]
async fn a_background_tab_closing_leaves_focus_where_it_is(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Kept", "");
        fixture.commit_ticket(root, OTHER, "Shown", "");
    });
    let a = TabKey::for_target(&ticket(&root, TICKET));
    let a_open = |shell: &Shell, cx: &mut TestAppContext| {
        shell
            .workspace
            .read_with(cx, |workspace, _| workspace.tab_keys())
            .contains(&a)
    };
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    type_at_end(&shell, &host, "x", cx);
    assert!(attached(&shell, &root, TICKET, cx).is_some(), "a draft");
    type_at_end(&shell, &host, "y", cx);
    let write = shell.backend.hold_read(ReadKind::DraftWrite);
    shell.click(("tab-close", 0usize), cx);
    shell.click("close-draft-keep-button", cx);
    assert!(a_open(&shell, cx), "Keep draft waits for its write");

    open(&shell, ticket(&root, OTHER), cx);
    let read = read_editor(&shell, cx);
    focus_at(&shell, &read, 0, cx);
    drop(write);
    shell
        .until_drawn(cx, "the kept tab closing", |shell, cx| !a_open(shell, cx))
        .await;
    shell.frame(cx, |_, _| ());
    assert!(focused(&shell, &read, cx), "focus stays in B's read editor");

    // The shown case: B, edited and kept the same way, closes while shown.
    let host = edit(&shell, cx).await;
    type_at_end(&shell, &host, "x", cx);
    assert!(attached(&shell, &root, OTHER, cx).is_some(), "a draft");
    type_at_end(&shell, &host, "y", cx);
    let write = shell.backend.hold_read(ReadKind::DraftWrite);
    shell.click(("tab-close", 0usize), cx);
    shell.click("close-draft-keep-button", cx);
    drop(write);
    shell
        .until_drawn(cx, "the shown tab closing", |shell, cx| {
            tabs(shell, cx) == 0
        })
        .await;
    shell.frame(cx, |_, _| ());
    assert!(
        holds_focus(&shell, "region-content", cx),
        "the content region takes the focus"
    );
}
