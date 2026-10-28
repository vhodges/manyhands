//! Edit, over real repositories (design, Part C, Phases, steps 1 to 4):
//! the private context made ready with progress in the pill, the keys that
//! start it and the places they never act, what is not offered, a stored
//! draft continued, a refusal that makes nothing, a hand-off resumed with
//! the same request, a tab closed while preparing, `CreatedContexts`, the
//! read editor's caret carried in, the read again under a new stamp, the
//! toolbar and focus, D2's draft begun and fed, an unavailable store said
//! first, a failed write shown, the status item, undo, normalized
//! classification and a large body.
//!
//! Each test sets the caret, focuses the editor and draws a frame before
//! typing, and types through the window. A test that waits on the worker
//! allows parking. Draft fields and items would print text through
//! `Debug`, so they are compared with `assert!`.

use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use gpui_kit::{Entity, Focusable, TestAppContext, test::TestWindowExt};
use manyhands::{
    drafts::{DraftFields, DraftId, Held},
    repository::{ItemDto, ItemDtoKind, RepositoryService},
    results::ResultCode,
};
use zorite_editor::EditorState;

use super::{
    ItemPane,
    chips::ChipField,
    edit::{CONTINUE, PREPARE_PENDING, PREPARE_REFUSED, RESUME, recovery_off_message},
    pill::{PillState, edit_state, pill_words},
};
use crate::desktop::{
    keys::ITEM_PAGE_CONTEXT,
    model::{
        drafts::IDLE,
        edit::{CONTINUE_EDITING, DRAFT_NOT_KEPT, DraftState, Phase, RECOVERY_OFF, base_for_edit},
        editor_text::load,
        item::LATER_NOTE,
        navigation::OpenTarget,
        tabs::TabKey,
    },
    services::{
        JobKind, ReadKind,
        created_contexts::ContextKey,
        requests::{DesktopRequestId, Intent, RequestState},
    },
    test_support::{
        drafts::{held, store_draft, stored_body},
        fixtures::{Fixture, item_id, ticket_path, ticket_source},
        snapshot::RepositorySnapshot,
    },
    view::{
        draft_comparison::CURRENT,
        drafts::row_id,
        editor::host::EditorHost,
        harness::{Shell, WAIT, key, wait_until},
    },
};

const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG1";
const OTHER: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG2";
const THIRD: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG3";
const DEP_A: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG4";
const DEP_B: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG5";
const DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG6";
/// CRLF line endings and a words-mixed `$$` pair: a body the editor's load
/// changes (N-1, N-2).
const RAW: &str = "First line\r\nWhat if words $$E=mc^2$$ more\r\nLast line\r\n";

const VIEWING_SHARED: &str = "Viewing the shared copy";
const GETTING_READY: &str = "Getting a private copy ready\u{2026}";
const EDITING_SAVED: &str = "Editing \u{b7} saved on this computer";
const EDITING_UNSAVED: &str = "Editing \u{b7} unsaved changes";

/// One repository, `a`, built by `build` and refreshed, in an open window.
fn shell(cx: &mut TestAppContext, build: impl FnOnce(&Fixture, &Path)) -> (Shell, PathBuf) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    build(&fixture, &root);
    fixture.refresh(&root);
    (Shell::open(fixture, cx), root)
}

fn open(shell: &Shell, target: OpenTarget, cx: &mut TestAppContext) {
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
}

fn ticket(root: &Path, id: &str) -> OpenTarget {
    OpenTarget::Item {
        repo: key(root),
        kind: ItemDtoKind::Ticket,
        id: item_id(id),
    }
}

/// A ticket's source with `body` in place of the fixture's own.
fn ticket_with_body(id: &str, title: &str, extra: &str, body: &str) -> String {
    ticket_source(id, title, extra).replace(&format!("Body of {title}.\n"), body)
}

fn shown(shell: &Shell, root: &Path, id: &str) -> ItemDto {
    let service = shell.fixture.service();
    let repo = service.resolve_repository(root).unwrap();
    service.show_item(&repo, &item_id(id)).unwrap()
}

fn pane(shell: &Shell, cx: &mut TestAppContext) -> Entity<ItemPane> {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.item_pane())
        .expect("an item tab is shown")
}

fn pill(shell: &Shell, cx: &mut TestAppContext) -> Option<String> {
    shell.label("item-pill", cx)
}

/// The `context prepare` requests of this session, newest first.
fn prepares(shell: &Shell, cx: &mut TestAppContext) -> Vec<DesktopRequestId> {
    shell.requests(cx, |request| {
        matches!(request.intent, Intent::ContextPrepare { .. })
    })
}

/// The shown tab's read editor, after a frame.
fn read_editor(shell: &Shell, cx: &mut TestAppContext) -> Entity<EditorState> {
    shell.frame(cx, |_, _| ());
    let pane = pane(shell, cx);
    pane.read_with(cx, |pane, cx| pane.read_editor(cx))
        .expect("the read view is shown")
}

fn text(editor: &Entity<EditorState>, cx: &mut TestAppContext) -> String {
    editor.read_with(cx, |editor, _| editor.text().to_owned())
}

fn cursor(editor: &Entity<EditorState>, cx: &mut TestAppContext) -> usize {
    editor.read_with(cx, |editor, _| editor.cursor())
}

/// Whether `editor` itself holds focus, after a frame.
fn focused(shell: &Shell, editor: &Entity<EditorState>, cx: &mut TestAppContext) -> bool {
    let handle = editor.read_with(cx, |editor, cx| editor.focus_handle(cx));
    shell.frame(cx, move |window, _| handle.is_focused(window))
}

/// Whether the element `id` holds focus.
fn holds_focus(shell: &Shell, id: &'static str, cx: &mut TestAppContext) -> bool {
    shell.frame(cx, |window, _| {
        window
            .try_find(id)
            .is_some_and(|element| element.focused() == Some(true))
    })
}

/// Whether the focused element is in the page's context, where `e` edits.
fn in_page_context(shell: &Shell, cx: &mut TestAppContext) -> bool {
    shell.frame(cx, |window, _| {
        window
            .context_stack()
            .iter()
            .any(|context| context.contains(ITEM_PAGE_CONTEXT))
    })
}

/// Puts the caret at `at`, focuses `editor` and draws a frame, as the
/// editor tests do before typing.
fn focus_at(shell: &Shell, editor: &Entity<EditorState>, at: usize, cx: &mut TestAppContext) {
    editor.update(cx, |editor, cx| editor.set_cursor(at, cx));
    let handle = editor.read_with(cx, |editor, cx| editor.focus_handle(cx));
    shell.frame(cx, |window, cx| window.focus(&handle, cx));
    shell.frame(cx, |_, _| ());
    assert!(focused(shell, editor, cx), "the editor took focus");
}

/// The shown tab's edit host, once its editor has opened: frames are drawn
/// while the worker and the reads run.
async fn until_editing(shell: &Shell, cx: &mut TestAppContext) -> Entity<EditorHost> {
    let deadline = Instant::now() + WAIT;
    loop {
        shell.frame(cx, |_, _| ());
        shell.run(cx);
        let pane = pane(shell, cx);
        if let Some(host) = pane.read_with(cx, |pane, _| pane.edit_host()) {
            shell.frame(cx, |_, _| ());
            return host;
        }
        assert!(Instant::now() < deadline, "the edit editor did not open");
        cx.executor().timer(Duration::from_millis(10)).await;
    }
}

/// Waits until the request `id` has ended, then draws a frame.
async fn ended(shell: &Shell, id: DesktopRequestId, cx: &mut TestAppContext) {
    shell.until_ended(id, cx).await;
    shell.run(cx);
    shell.frame(cx, |_, _| ());
}

/// The finished request `id`'s code.
fn code(shell: &Shell, id: DesktopRequestId, cx: &mut TestAppContext) -> Option<ResultCode> {
    shell
        .operations(cx)
        .read_with(cx, |operations, _| match &operations.request(id)?.state {
            RequestState::Finished(_, code, ..) => Some(*code),
            _ => None,
        })
}

/// Clicks Edit and waits for the editor.
async fn edit(shell: &Shell, cx: &mut TestAppContext) -> Entity<EditorHost> {
    shell.click("item-edit", cx);
    until_editing(shell, cx).await
}

fn host_editor(host: &Entity<EditorHost>, cx: &mut TestAppContext) -> Entity<EditorState> {
    host.read_with(cx, |host, _| host.editor().clone())
}

/// Types `typed` at the end of the edit editor's text.
fn type_at_end(shell: &Shell, host: &Entity<EditorHost>, typed: &str, cx: &mut TestAppContext) {
    let editor = host_editor(host, cx);
    let end = text(&editor, cx).len();
    focus_at(shell, &editor, end, cx);
    shell.visual(cx).simulate_input(typed);
    shell.run(cx);
}

fn registry(
    shell: &Shell,
    cx: &mut TestAppContext,
) -> Entity<crate::desktop::services::drafts::DraftRegistry> {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().drafts().clone())
}

/// The draft the tab of `root`'s ticket `id` holds.
fn attached(shell: &Shell, root: &Path, id: &str, cx: &mut TestAppContext) -> Option<DraftId> {
    let tab = TabKey::Item {
        repo: key(root),
        kind: ItemDtoKind::Ticket,
        id: id.to_owned(),
    };
    registry(shell, cx).read_with(cx, |registry, _| registry.attached(&tab))
}

/// Whether this process counts the ticket `id`'s context as made here.
fn created(shell: &Shell, root: &Path, id: &str, cx: &mut TestAppContext) -> bool {
    let created = shell.workspace.read_with(cx, |workspace, _| {
        workspace.services().created_contexts().clone()
    });
    created.read_with(cx, |created, _| {
        created.contains(&ContextKey::new(&key(root), ItemDtoKind::Ticket, id))
    })
}

/// The stored body of every draft in the store.
fn stored_bodies(shell: &Shell) -> Vec<(DraftId, String)> {
    let store = shell.fixture.draft_store();
    let ids: Vec<DraftId> = store
        .list()
        .unwrap()
        .into_iter()
        .map(|listing| listing.id)
        .collect();
    ids.into_iter()
        .filter_map(|id| Some((id, stored_body(&store, id)?)))
        .collect()
}

/// The ticket's body among `fields`.
fn body_of(fields: &DraftFields) -> &str {
    match fields {
        DraftFields::Document { body, .. }
        | DraftFields::Ticket { body, .. }
        | DraftFields::Comment { body } => body,
    }
}

/// The edit editor's toolbar's status, as D2 gave it.
fn status(shell: &Shell, cx: &mut TestAppContext) -> DraftState {
    let toolbar = pane(shell, cx)
        .read_with(cx, |pane, _| pane.edit_toolbar())
        .expect("the toolbar is shown");
    toolbar.read_with(cx, |toolbar, _| toolbar.status())
}

/// Edit runs `context prepare` on the user lane. While it runs, the pill
/// says the private copy is being made ready, Edit is disabled and the
/// page stays readable; then the editor opens, the pill says editing, and
/// the context is there.
#[gpui_kit::test]
async fn edit_prepares_the_context_with_progress_in_the_pill_before_the_editor_opens(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Edited", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    assert_eq!(pill(&shell, cx).as_deref(), Some(VIEWING_SHARED));
    assert!(shell.present("item-edit", cx), "Edit is offered");

    let held = shell.backend.hold_job(JobKind::ContextPrepare);
    shell.click("item-edit", cx);
    held.wait_reached();
    shell.run(cx);
    assert_eq!(pill(&shell, cx).as_deref(), Some(GETTING_READY));
    assert!(shell.inert_button("item-edit", cx), "Edit is disabled");
    shell.click("item-edit", cx);
    assert_eq!(prepares(&shell, cx).len(), 1, "one prepare");
    assert!(shell.present("item-read", cx), "the page stays readable");
    assert!(!shell.present("item-toolbar", cx));
    assert!(!shell.present("item-edit-view", cx));

    held.release();
    let host = until_editing(&shell, cx).await;
    assert_eq!(pill(&shell, cx).as_deref(), Some(EDITING_SAVED));
    assert!(shell.present("item-toolbar", cx));
    assert!(shell.present("item-edit-view", cx));
    assert!(
        !shell.present("item-read", cx),
        "the edit editor replaces it"
    );
    assert!(!shell.present("item-edit", cx), "Done takes Edit's place");
    assert!(shell.present("item-done", cx));
    let branch = format!("refs/heads/manyhands/ticket/{TICKET}");
    assert!(
        RepositorySnapshot::take(&root)
            .refs
            .iter()
            .any(|(name, _)| *name == branch),
        "the context's branch is made"
    );
    let body = shown(&shell, &root, TICKET).body.unwrap();
    assert!(host.read_with(cx, |host, cx| host.text(cx)) == load(&body));
    assert_eq!(
        code(&shell, prepares(&shell, cx)[0], cx),
        Some(ResultCode::Ok)
    );
}

/// `e` on the page, the read editor included, and `secondary-shift-e`
/// anywhere in the tab start Edit while viewing. Neither acts while
/// editing, where `e` is typed; nor in the link popover, the details
/// sheet, an Inspect tab's source or the Find field (controller ruling,
/// 2026-10-27). A dialog is the unavailable store's test's.
#[gpui_kit::test]
async fn e_and_secondary_shift_e_start_edit_only_while_viewing(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "First", "");
        fixture.commit_ticket(root, OTHER, "Second", "");
        fixture.commit_ticket(root, THIRD, "Third", "");
    });

    // `e` in the read editor: Edit, and the letter is not typed anywhere.
    open(&shell, ticket(&root, TICKET), cx);
    let read = read_editor(&shell, cx);
    let shown_text = text(&read, cx);
    focus_at(&shell, &read, 3, cx);
    assert!(
        in_page_context(&shell, cx),
        "the read editor is on the page"
    );
    shell.keys("e", cx);
    let host = until_editing(&shell, cx).await;
    assert_eq!(prepares(&shell, cx).len(), 1, "e started Edit");
    assert!(text(&read, cx) == shown_text, "the read view is unchanged");

    // While editing, `e` is typed and `secondary-shift-e` starts nothing.
    let before = host.read_with(cx, |host, cx| host.text(cx));
    type_at_end(&shell, &host, "e", cx);
    assert!(host.read_with(cx, |host, cx| host.text(cx)) == format!("{before}e"));
    assert!(!in_page_context(&shell, cx), "set only while viewing");
    shell.keys("secondary-shift-e", cx);
    assert_eq!(prepares(&shell, cx).len(), 1, "nothing while editing");

    // In the link popover's address, `e` is the address's.
    shell.keys("secondary-k", cx);
    assert!(shell.present("toolbar-link-popover", cx));
    let typed = host.read_with(cx, |host, cx| host.text(cx));
    shell.visual(cx).simulate_input("e");
    shell.run(cx);
    assert!(host.read_with(cx, |host, cx| host.text(cx)) == typed);
    assert_eq!(prepares(&shell, cx).len(), 1, "nothing in the popover");
    shell.keys("escape", cx);

    // `secondary-shift-e` from the header of a viewing tab.
    open(&shell, ticket(&root, OTHER), cx);
    shell.keys("secondary-3", cx);
    let mut presses = 0;
    while !holds_focus(&shell, "item-comments", cx) {
        presses += 1;
        assert!(presses < 5, "Tab reaches the header");
        shell.keys("tab", cx);
    }
    shell.keys("secondary-shift-e", cx);
    until_editing(&shell, cx).await;
    assert_eq!(
        prepares(&shell, cx).len(),
        2,
        "secondary-shift-e started Edit"
    );

    // The details sheet, an Inspect tab's source and the Find field.
    open(&shell, ticket(&root, THIRD), cx);
    shell.click(ChipField::All.id(), cx);
    shell.keys("e", cx);
    shell.keys("secondary-shift-e", cx);
    assert_eq!(prepares(&shell, cx).len(), 2, "nothing in the sheet");
    shell.keys("escape", cx);
    open(
        &shell,
        OpenTarget::Inspect {
            repo: key(&root),
            worktree: key(&root),
            path: ticket_path(THIRD),
        },
        cx,
    );
    let state = pane(&shell, cx)
        .read_with(cx, |pane, _| pane.source_state().cloned())
        .expect("the source is shown");
    let mut visual = shell.visual(cx);
    visual.update(|window, cx| state.update(cx, |state, cx| state.focus(window, cx)));
    visual.run_until_parked();
    assert!(visual.update(|window, cx| state.read(cx).focus_handle(cx).is_focused(window)));
    shell.keys("e", cx);
    shell.keys("secondary-shift-e", cx);
    assert!(
        !shell.present("item-edit", cx),
        "an Inspect tab offers no Edit"
    );
    assert_eq!(prepares(&shell, cx).len(), 2, "nothing in the source");
    shell.keys("secondary-p", cx);
    shell.visual(cx).simulate_input("e");
    shell.run(cx);
    assert_eq!(prepares(&shell, cx).len(), 2, "nothing in the Find field");
}

/// A closed ticket offers no Edit, and its keys start nothing.
#[gpui_kit::test]
fn edit_of_a_closed_ticket_is_not_offered(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_closed_ticket(root, TICKET, "Closed for good");
    });
    open(&shell, ticket(&root, TICKET), cx);
    assert_eq!(pill(&shell, cx).as_deref(), Some("Closed"));
    assert!(!shell.present("item-edit", cx));
    let read = read_editor(&shell, cx);
    focus_at(&shell, &read, 0, cx);
    shell.keys("e secondary-shift-e", cx);
    assert!(prepares(&shell, cx).is_empty());
    assert!(!shell.present("item-toolbar", cx));
}

/// R-35: while a draft of the item is stored, Edit reads Continue editing,
/// and no second draft begins: until Task 18's route it restores the
/// stored draft, as D2's banner does, and prepares nothing. Once the draft
/// is restored in the tab, Continue editing waits for Task 18.
#[gpui_kit::test]
fn edit_reads_continue_editing_while_a_draft_is_stored(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "Drafted", "");
    fixture.refresh(&root);
    let stored = store_draft(&fixture, &root, ItemDtoKind::Ticket, TICKET, "Draft body");
    let shell = Shell::open(fixture, cx);
    open(&shell, ticket(&root, TICKET), cx);

    assert_eq!(
        shell.label("item-edit", cx).as_deref(),
        Some(CONTINUE_EDITING)
    );
    assert!(
        !shell.present("item-edit-note", cx),
        "offered, it needs no note"
    );
    shell.click("item-edit", cx);
    shell.run(cx);
    assert_eq!(
        attached(&shell, &root, TICKET, cx),
        Some(stored),
        "restored"
    );
    assert!(shell.present("draft-section", cx), "restored in the tab");
    assert!(prepares(&shell, cx).is_empty(), "nothing is prepared");
    assert_eq!(
        shell.label("item-edit", cx).as_deref(),
        Some(CONTINUE_EDITING)
    );
    assert!(shell.inert_button("item-edit", cx), "Task 18 continues it");
    assert_eq!(
        shell.label("item-edit-note", cx).as_deref(),
        Some(LATER_NOTE),
        "said as the header's other disabled actions say it"
    );
    shell.click("item-edit", cx);
    shell.run(cx);
    assert!(
        prepares(&shell, cx).is_empty(),
        "pressed, it does nothing yet"
    );
    assert!(
        shell.label("draft-refused", cx).is_none(),
        "no second restore"
    );
}

/// Review Focus 3: Edit of a primary file edited by hand and not
/// committed is refused with `worktree_not_clean`, in a banner; no branch
/// or worktree is made, the editor never opens, and the tab stays viewing.
#[gpui_kit::test]
async fn edit_of_an_uncommitted_primary_file_is_refused_and_nothing_is_made(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Hand edited", "");
    });
    let path = root.join(ticket_path(TICKET));
    let edited = std::fs::read_to_string(&path).unwrap() + "Typed by hand.\n";
    std::fs::write(&path, edited).unwrap();
    open(&shell, ticket(&root, TICKET), cx);
    let before = RepositorySnapshot::take(&root);

    shell.click("item-edit", cx);
    let request = prepares(&shell, cx)[0];
    ended(&shell, request, cx).await;

    assert_eq!(
        code(&shell, request, cx),
        Some(ResultCode::WorktreeNotClean)
    );
    assert_eq!(
        shell.label("item-prepare-refused", cx).as_deref(),
        Some(PREPARE_REFUSED)
    );
    assert_eq!(
        shell.label("item-prepare-code", cx).as_deref(),
        Some(ResultCode::WorktreeNotClean.as_str())
    );
    assert!(shell.present("item-prepare-message", cx));
    assert_eq!(RepositorySnapshot::take(&root), before, "nothing is made");
    assert!(!shell.present("item-toolbar", cx));
    assert!(shell.present("item-read", cx));
    assert_eq!(pill(&shell, cx).as_deref(), Some(VIEWING_SHARED));
    let phase = pane(&shell, cx).read_with(cx, |pane, _| pane.edit_phase());
    assert_eq!(phase, Phase::Viewing);
    shell.click("item-edit", cx);
    let again = prepares(&shell, cx);
    assert_eq!(again.len(), 2, "Edit is offered again");
    ended(&shell, again[0], cx).await;
    assert_eq!(
        code(&shell, again[0], cx),
        Some(ResultCode::WorktreeNotClean)
    );
}

/// R-13, R-77 (owner, 2026-10-26): a prepare whose hand-off to the index
/// fails answers `discovery_pending`; the tab waits in Loading, with the
/// pill saying the copy is being made ready, Edit disabled and Resume in
/// the banner, which sends the same request again (controller ruling,
/// 2026-10-28). A read that lands meanwhile opens nothing. Its `ok, created: true` enters
/// `CreatedContexts`, and the editor opens.
#[gpui_kit::test]
async fn a_discovery_pending_context_waits_for_resume(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Pending", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
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

    assert_eq!(
        code(&shell, request, cx),
        Some(ResultCode::DiscoveryPending)
    );
    assert_eq!(
        shell.label("item-prepare-pending", cx).as_deref(),
        Some(PREPARE_PENDING)
    );
    assert_eq!(
        shell.label("item-prepare-resume", cx).as_deref(),
        Some(RESUME)
    );
    assert!(!shell.present("item-toolbar", cx), "editing waits");
    let phase = pane(&shell, cx).read_with(cx, |pane, _| pane.edit_phase());
    assert_eq!(phase, Phase::Loading, "it waits in Loading");
    assert_eq!(pill(&shell, cx).as_deref(), Some(GETTING_READY));
    assert!(shell.inert_button("item-edit", cx), "Resume, not Edit");
    shell.click("item-edit", cx);
    assert_eq!(prepares(&shell, cx), [request], "Edit sends nothing new");
    // The tab's reads land, the repository's reload among them; none opens
    // the editor before Resume.
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    assert!(!shell.present("item-toolbar", cx), "still waiting");
    let phase = pane(&shell, cx).read_with(cx, |pane, _| pane.edit_phase());
    assert_eq!(phase, Phase::Loading);
    assert!(!created(&shell, &root, TICKET, cx), "not before the retry");
    let first = shell.operations(cx).read_with(cx, |operations, _| {
        let request = operations.request(request).unwrap();
        (request.request_id, request.attempt)
    });

    shell
        .backend
        .replace_service(RepositoryService::open_at(shell.fixture.data_dir()).unwrap());
    shell.click("item-prepare-resume", cx);
    until_editing(&shell, cx).await;

    assert_eq!(prepares(&shell, cx), [request], "the same request");
    let again = shell.operations(cx).read_with(cx, |operations, _| {
        let request = operations.request(request).unwrap();
        (request.request_id, request.attempt)
    });
    assert_eq!(again.0, first.0, "under its own request ID");
    assert_eq!(again.1, first.1 + 1, "a second attempt");
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));
    assert!(created(&shell, &root, TICKET, cx));
    assert!(!shell.present("item-prepare-pending", cx));
}

/// A tab closed while its prepare runs opens nothing. The context the
/// answer made still enters `CreatedContexts`, for Task 17 to release.
#[gpui_kit::test]
async fn closing_the_tab_while_preparing_opens_no_editor(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Closed early", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let held = shell.backend.hold_job(JobKind::ContextPrepare);
    shell.click("item-edit", cx);
    held.wait_reached();
    shell.keys("secondary-w", cx);
    assert!(
        shell
            .workspace
            .read_with(cx, |workspace, _| workspace.item_pane())
            .is_none(),
        "the tab closed"
    );

    held.release();
    let request = prepares(&shell, cx)[0];
    ended(&shell, request, cx).await;
    assert_eq!(code(&shell, request, cx), Some(ResultCode::Ok));
    assert!(
        shell
            .workspace
            .read_with(cx, |workspace, _| workspace.tab_keys())
            .is_empty(),
        "nothing opened"
    );
    assert!(!shell.present("item-toolbar", cx));
    assert!(created(&shell, &root, TICKET, cx), "counted for release");
}

/// R-75: an Edit that made its item's context enters `CreatedContexts`;
/// one that found the context there already does not.
#[gpui_kit::test]
async fn a_created_answer_enters_created_contexts_and_a_reused_one_does_not(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Made here", "");
        fixture.commit_ticket(root, OTHER, "Made before", "");
    });
    // A save provisions the item's own context before this run.
    shell.fixture.save_ticket(&root, OTHER, "Saved before");
    open(&shell, ticket(&root, TICKET), cx);
    edit(&shell, cx).await;
    assert!(created(&shell, &root, TICKET, cx));

    open(&shell, ticket(&root, OTHER), cx);
    edit(&shell, cx).await;
    assert_eq!(
        code(&shell, prepares(&shell, cx)[0], cx),
        Some(ResultCode::Ok)
    );
    assert!(!created(&shell, &root, OTHER, cx), "reused, not made");
}

/// R-65: the read editor's caret carries into the edit editor when both
/// hold the same text. The body is one the load changes (CRLF and a
/// words-mixed `$$` pair), so the carry depends on comparing the texts as
/// loaded (R-79), never the raw body.
#[gpui_kit::test]
async fn edit_carries_the_read_cursor_into_the_editor(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        let source = ticket_with_body(TICKET, "Carried", "", RAW);
        fixture.commit_file(root, &ticket_path(TICKET), source.as_bytes());
    });
    open(&shell, ticket(&root, TICKET), cx);
    let read = read_editor(&shell, cx);
    assert!(
        text(&read, cx) == load(RAW),
        "the read editor holds it loaded"
    );
    assert!(load(RAW) != RAW, "the load changes the fixture");
    focus_at(&shell, &read, 5, cx);
    let host = edit(&shell, cx).await;
    let editor = host_editor(&host, cx);
    assert_eq!(cursor(&editor, cx), 5);
    assert!(text(&editor, cx) == load(RAW));
    assert!(text(&read, cx) == load(RAW));
}

/// R-70: Loading reads the item again, under a new stamp. When that read's
/// body is not what the read editor holds, the edit editor opens on the
/// new body with its caret at the start, and the read editor reloads.
#[gpui_kit::test]
async fn loading_reloads_the_read_editor_when_the_texts_differ(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Changing", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let read = read_editor(&shell, cx);
    let old = text(&read, cx);
    focus_at(&shell, &read, 5, cx);

    // Changed on primary since the tab read it; the tab has not read again.
    let newer = "A newer body, committed elsewhere.\n";
    shell.fixture.commit_file(
        &root,
        &ticket_path(TICKET),
        ticket_with_body(TICKET, "Changing", "", newer).as_bytes(),
    );
    shell.fixture.refresh(&root);
    assert!(text(&read, cx) == old, "the tab still shows the old body");

    let host = edit(&shell, cx).await;
    let editor = host_editor(&host, cx);
    assert!(text(&editor, cx) == load(newer), "the newer read's body");
    assert_eq!(cursor(&editor, cx), 0, "the caret starts at the start");
    shell.frame(cx, |_, _| ());
    assert!(text(&read, cx) == load(newer), "the read editor reloaded");
}

/// Edit shows the toolbar under the header, outside the item's scroll
/// region, with its hint, and moves focus into the edit editor.
#[gpui_kit::test]
async fn the_toolbar_appears_and_focus_moves_into_the_editor(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Focused", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    assert!(!shell.present("item-toolbar", cx));
    let host = edit(&shell, cx).await;
    assert!(shell.present("item-toolbar", cx));
    assert!(shell.present("toolbar-hint", cx));
    let (in_tab, in_region) = shell.frame(cx, |window, _| {
        let path = window.find("item-toolbar").path().to_vec();
        (
            path.contains(&"item-view".into()),
            path.contains(&"item-scroll".into()),
        )
    });
    assert!(in_tab && !in_region, "under the header, above the region");
    let editor = host_editor(&host, cx);
    assert!(focused(&shell, &editor, cx), "focus is in the editor");
}

/// The first change from the base begins D2's draft, on the base the read
/// gave (R-11), attached to the tab; later changes update the same draft.
/// Moving the caret begins nothing.
#[gpui_kit::test]
async fn the_first_real_edit_begins_a_draft_and_later_edits_update_it(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Drafted", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let editor = host_editor(&host, cx);
    focus_at(&shell, &editor, 0, cx);
    shell.keys("right right left", cx);
    assert!(stored_bodies(&shell).is_empty(), "moving begins nothing");
    assert_eq!(attached(&shell, &root, TICKET, cx), None);

    let baseline = text(&editor, cx);
    type_at_end(&shell, &host, "x", cx);
    let drafts = stored_bodies(&shell);
    assert_eq!(drafts.len(), 1, "one draft began");
    let (id, body) = &drafts[0];
    assert!(*body == format!("{baseline}x"));
    assert_eq!(attached(&shell, &root, TICKET, cx), Some(*id));
    let item = shown(&shell, &root, TICKET);
    let base = registry(&shell, cx).read_with(cx, |registry, _| {
        registry.session(*id).unwrap().base().cloned().unwrap()
    });
    assert!(
        base == base_for_edit(&item).unwrap(),
        "the read's base (R-11)"
    );

    type_at_end(&shell, &host, "y", cx);
    cx.executor().advance_clock(IDLE);
    shell.run(cx);
    let drafts = stored_bodies(&shell);
    assert_eq!(drafts.len(), 1, "the same draft");
    assert_eq!(drafts[0].0, *id);
    assert!(drafts[0].1 == format!("{baseline}xy"));
    assert_eq!(pill(&shell, cx).as_deref(), Some(EDITING_UNSAVED));
}

/// A draft store that did not open is said before editing: Edit opens the
/// dialog first, which a key behind it cannot reach; Cancel makes nothing,
/// Continue edits with crash recovery off, said in a banner and in the
/// toolbar, and no draft begins.
#[cfg(unix)]
#[gpui_kit::test]
async fn an_unavailable_store_is_said_before_editing(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let elsewhere = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(elsewhere.path(), fixture.data_dir().join("drafts")).unwrap();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "Unprotected", "");
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    open(&shell, ticket(&root, TICKET), cx);
    let reason = registry(&shell, cx)
        .read_with(cx, |registry, _| registry.unavailable())
        .expect("the store did not open");

    shell.click("item-edit", cx);
    assert!(shell.present("recovery-off-dialog", cx));
    assert_eq!(
        shell.label("recovery-off-message", cx),
        Some(recovery_off_message(reason))
    );
    assert!(prepares(&shell, cx).is_empty(), "said before editing");
    shell.keys("e secondary-shift-e", cx);
    assert!(prepares(&shell, cx).is_empty(), "nothing behind the dialog");
    shell.click("recovery-off-cancel", cx);
    assert!(!shell.present("recovery-off-dialog", cx));
    assert!(prepares(&shell, cx).is_empty(), "Cancel makes nothing");

    shell.click("item-edit", cx);
    assert_eq!(
        shell.label("recovery-off-continue", cx).as_deref(),
        Some(CONTINUE)
    );
    shell.click("recovery-off-continue", cx);
    let host = until_editing(&shell, cx).await;
    assert_eq!(
        shell.label("item-recovery-off", cx),
        Some(format!(
            "{RECOVERY_OFF}: {reason} Save to protect your changes."
        ))
    );
    assert_eq!(
        shell.label("toolbar-status", cx).as_deref(),
        Some(RECOVERY_OFF)
    );
    type_at_end(&shell, &host, "x", cx);
    assert_eq!(attached(&shell, &root, TICKET, cx), None, "no draft begins");
    assert_eq!(status(&shell, cx), DraftState::StoreUnavailable);
}

/// A write that fails while editing stays visible in the toolbar's status
/// item, and typing goes on; a later write that lands says so again.
#[cfg(unix)]
#[gpui_kit::test]
async fn a_failed_flush_while_editing_stays_visible(cx: &mut TestAppContext) {
    use crate::desktop::test_support::drafts::{FolderMode, superuser};

    if superuser() {
        return;
    }
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Failing", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    type_at_end(&shell, &host, "x", cx);
    assert!(matches!(status(&shell, cx), DraftState::Written(_)));
    let id = attached(&shell, &root, TICKET, cx).expect("the draft began");

    let folder = shell.fixture.data_dir().join("drafts");
    let read_only = FolderMode::set(&folder, 0o500);
    type_at_end(&shell, &host, "y", cx);
    cx.executor().advance_clock(IDLE);
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    assert_eq!(
        shell.label("toolbar-status", cx).as_deref(),
        Some(DRAFT_NOT_KEPT)
    );
    type_at_end(&shell, &host, "z", cx);
    let typed = host.read_with(cx, |host, cx| host.text(cx));
    assert!(typed.ends_with("xyz"), "typing goes on");
    assert_eq!(
        shell.label("toolbar-status", cx).as_deref(),
        Some(DRAFT_NOT_KEPT)
    );

    drop(read_only);
    type_at_end(&shell, &host, "w", cx);
    cx.executor().advance_clock(IDLE);
    shell.run(cx);
    assert!(matches!(status(&shell, cx), DraftState::Written(_)));
    let store = shell.fixture.draft_store();
    assert!(stored_body(&store, id).is_some_and(|body| body.ends_with("xyzw")));
    assert!(
        shell
            .label("toolbar-status", cx)
            .is_some_and(|words| words.starts_with("Draft kept \u{b7} "))
    );
}

/// The status item reads D2's state only: nothing before the first write
/// lands, the time of the last write that landed, kept while a newer one
/// waits, and the newer time once it lands. Text typed while the draft
/// begins is written with it.
#[gpui_kit::test]
async fn the_draft_indicator_reads_only_d2s_flush_state(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Indicated", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    assert_eq!(status(&shell, cx), DraftState::NotBegun);
    assert_eq!(shell.label("toolbar-status", cx), None, "nothing to say");

    let begin = shell.backend.hold_read(ReadKind::Drafts);
    type_at_end(&shell, &host, "x", cx);
    type_at_end(&shell, &host, "y", cx);
    assert_eq!(status(&shell, cx), DraftState::NotBegun, "not yet written");
    drop(begin);
    shell.run(cx);
    let id = attached(&shell, &root, TICKET, cx).expect("the draft began");
    let begun = registry(&shell, cx)
        .read_with(cx, |registry, _| registry.session(id).unwrap().flushed_at());
    assert_eq!(status(&shell, cx), DraftState::Written(begun));
    // The begin stored the text it was asked with; what was typed while it
    // began follows, on the timer.
    cx.executor().advance_clock(IDLE);
    shell.run(cx);
    assert_eq!(
        stored_body(&shell.fixture.draft_store(), id).map(|body| body.ends_with("xy")),
        Some(true),
        "what was typed while it began"
    );
    let first = registry(&shell, cx)
        .read_with(cx, |registry, _| registry.session(id).unwrap().flushed_at());
    assert_eq!(status(&shell, cx), DraftState::Written(first));

    let write = shell.backend.hold_read(ReadKind::DraftWrite);
    type_at_end(&shell, &host, "z", cx);
    cx.executor().advance_clock(IDLE);
    shell.run(cx);
    assert_eq!(
        status(&shell, cx),
        DraftState::Written(first),
        "kept, older"
    );
    drop(write);
    shell.run(cx);
    let second = registry(&shell, cx)
        .read_with(cx, |registry, _| registry.session(id).unwrap().flushed_at());
    assert!(second >= first);
    assert_eq!(status(&shell, cx), DraftState::Written(second));
    assert_eq!(
        stored_body(&shell.fixture.draft_store(), id).map(|body| body.ends_with("xyz")),
        Some(true),
        "the newest text"
    );
}

/// R-11, R-72: normalization is never a change. A draft undone to the text
/// as the editor loaded it holds the item's own fields, so the tab and the
/// Drafts page read it as already saved, though the file's bytes (CRLF
/// line endings, or `deps` out of ID order) differ from it.
#[gpui_kit::test]
async fn a_draft_undone_to_the_loaded_text_reads_already_saved(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        let source = ticket_with_body(TICKET, "Normalized", "", RAW);
        fixture.commit_file(root, &ticket_path(TICKET), source.as_bytes());
        fixture.commit_ticket(root, DEP_A, "First dependency", "");
        fixture.commit_ticket(root, DEP_B, "Second dependency", "");
        let deps = format!("deps:\n  - {DEP_B}\n  - {DEP_A}\n");
        fixture.commit_ticket(root, OTHER, "Unsorted", &deps);
    });
    for id in [TICKET, OTHER] {
        open(&shell, ticket(&root, id), cx);
        let host = edit(&shell, cx).await;
        type_at_end(&shell, &host, "x", cx);
        shell.keys("secondary-z", cx);
        cx.executor().advance_clock(IDLE);
        shell.run(cx);
        let draft = attached(&shell, &root, id, cx).expect("the draft began");
        let item = shown(&shell, &root, id);
        let state =
            registry(&shell, cx).read_with(cx, |registry, _| registry.classify(draft, Ok(&item)));
        assert!(
            matches!(state, Some(manyhands::drafts::TargetState::AlreadySaved)),
            "{id} in its tab"
        );
        close_keeping(&shell, draft, cx);
        assert_eq!(
            drafts_page_state(&shell, draft, cx).as_deref(),
            Some(crate::desktop::view::draft_comparison::ALREADY_SAVED),
            "{id} on the Drafts page"
        );
    }
}

/// R-12: undo emits no event of its own, yet reaches the draft: the text
/// undone to the base is written.
#[gpui_kit::test]
async fn undo_reaches_the_draft(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Undone", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let baseline = host.read_with(cx, |host, cx| host.text(cx));
    type_at_end(&shell, &host, "x", cx);
    let id = attached(&shell, &root, TICKET, cx).expect("the draft began");
    let store = shell.fixture.draft_store();
    assert!(stored_body(&store, id).is_some_and(|body| body == format!("{baseline}x")));

    shell.keys("secondary-z", cx);
    assert!(host.read_with(cx, |host, cx| host.text(cx)) == baseline);
    assert!(!host.read_with(cx, |host, _| host.is_dirty()));
    cx.executor().advance_clock(IDLE);
    shell.run(cx);
    assert!(stored_body(&store, id).is_some_and(|body| body == baseline));
    assert_eq!(pill(&shell, cx).as_deref(), Some(EDITING_SAVED));
}

/// Closes the shown tab, keeping its draft: it is let go of and listed.
fn close_keeping(shell: &Shell, id: DraftId, cx: &mut TestAppContext) {
    shell.keys("secondary-w", cx);
    shell.click("close-draft-keep", cx);
    shell.run(cx);
    assert!(matches!(
        held(&shell.fixture.draft_store(), id),
        Some(Held::No)
    ));
}

/// The Drafts page's state of the draft `id`, once its read lands.
fn drafts_page_state(shell: &Shell, id: DraftId, cx: &mut TestAppContext) -> Option<String> {
    shell.keys("secondary-shift-d", cx);
    shell.run(cx);
    shell.label(row_id(id, "state"), cx)
}

/// R-11: a draft begun on an item whose body the load normalizes reads
/// Current on the Drafts page: the base holds the body as loaded, and the
/// page classifies the item as the base holds it.
#[gpui_kit::test]
async fn normalization_alone_reads_current_on_the_drafts_page(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        let source = ticket_with_body(TICKET, "Normalized", "", RAW);
        fixture.commit_file(root, &ticket_path(TICKET), source.as_bytes());
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    assert!(host.read_with(cx, |host, cx| host.text(cx)) == load(RAW));
    type_at_end(&shell, &host, "x", cx);
    let id = attached(&shell, &root, TICKET, cx).expect("the draft began");
    let base = registry(&shell, cx).read_with(cx, |registry, _| {
        registry.session(id).unwrap().base().cloned().unwrap()
    });
    assert!(body_of(&base.fields) == load(RAW), "the loaded body");

    close_keeping(&shell, id, cx);
    assert_eq!(drafts_page_state(&shell, id, cx).as_deref(), Some(CURRENT));
}

/// R-72: a ticket whose file lists its `deps` out of ID order is drafted
/// with them in ID order, and its draft classifies as Current, in the tab
/// and on the Drafts page.
#[gpui_kit::test]
async fn unsorted_deps_classify_as_current(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, DEP_A, "First dependency", "");
        fixture.commit_ticket(root, DEP_B, "Second dependency", "");
        let deps = format!("deps:\n  - {DEP_B}\n  - {DEP_A}\n");
        fixture.commit_ticket(root, TICKET, "Unsorted", &deps);
    });
    let item = shown(&shell, &root, TICKET);
    let file_order: Vec<&str> = item.deps.iter().map(|dep| dep.id.as_str()).collect();
    assert_eq!(file_order, [DEP_B, DEP_A], "the file's order");
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    type_at_end(&shell, &host, "x", cx);
    // Read again, from the context Edit made, as the tab's next read is.
    let item = shown(&shell, &root, TICKET);
    let id = attached(&shell, &root, TICKET, cx).expect("the draft began");
    let (deps, state) = registry(&shell, cx).read_with(cx, |registry, _| {
        let session = registry.session(id).unwrap();
        let deps = match &session.base().unwrap().fields {
            DraftFields::Ticket { deps, .. } => deps.clone(),
            _ => Vec::new(),
        };
        (deps, registry.classify(id, Ok(&item)))
    });
    assert_eq!(deps, [DEP_A, DEP_B], "in ID order");
    assert!(matches!(
        state,
        Some(manyhands::drafts::TargetState::Current)
    ));

    close_keeping(&shell, id, cx);
    assert_eq!(drafts_page_state(&shell, id, cx).as_deref(), Some(CURRENT));
}

/// Review Focus 4, a smoke test: a 200 KiB body opens in the edit editor,
/// and a key typed into it lands. A test build takes seconds to draw a
/// frame of such a body, so this test forces only the one frame typing
/// needs: Edit is asked for directly, and the editor opens and takes focus
/// on the frames the test build draws as effects flush.
#[gpui_kit::test]
async fn a_large_body_opens_in_the_editor(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let body: String = (0..)
        .map(|line| format!("Line {line} of a large document, with words enough to fill it.\n"))
        .scan(0, |size, line| {
            (*size < 200 * 1024).then(|| {
                *size += line.len();
                line
            })
        })
        .collect();
    assert!(body.len() >= 200 * 1024);
    let (shell, root) = shell(cx, |fixture, root| {
        let source = format!(
            "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: {DOCUMENT}\ntitle: Large\n---\n{body}"
        );
        fixture.commit_file(root, "docs/large.md", source.as_bytes());
    });
    open(
        &shell,
        OpenTarget::Item {
            repo: key(&root),
            kind: ItemDtoKind::Document,
            id: item_id(DOCUMENT),
        },
        cx,
    );
    let pane = pane(&shell, cx);
    // A test build draws each dirty window as its effects flush, so the
    // editor opens and takes focus without a frame forced here.
    shell
        .visual(cx)
        .update(|window, cx| pane.update(cx, |pane, cx| pane.edit_requested(window, cx)));
    let Phase::Preparing(request) = pane.read_with(cx, |pane, _| pane.edit_phase()) else {
        panic!("Edit prepares the context");
    };
    shell.until_ended(request, cx).await;
    wait_until(cx, &pane, |pane, _| pane.edit_host().is_some()).await;
    let host = pane
        .read_with(cx, |pane, _| pane.edit_host())
        .expect("the editor opened");
    let editor = host_editor(&host, cx);
    assert!(text(&editor, cx) == load(&body));
    let end = load(&body).len();
    editor.update(cx, |editor, cx| editor.set_cursor(end, cx));
    // The one frame before typing, which also shows focus moved in.
    let handle = editor.read_with(cx, |editor, cx| editor.focus_handle(cx));
    assert!(
        shell.frame(cx, move |window, _| handle.is_focused(window)),
        "focus moved into the editor"
    );
    shell.visual(cx).simulate_input("x");
    shell.run(cx);
    let typed = text(&editor, cx);
    assert_eq!(typed.len(), end + 1);
    assert!(typed.ends_with('x'));
}

/// Escape in the editor focuses the header's Done (R-59), and the mode
/// key switches Rich text and Markdown from there too, the whole tab
/// being its context while editing.
#[gpui_kit::test]
async fn escape_focuses_done_and_the_mode_key_works_from_the_header(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Switched", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    let editor = host_editor(&host, cx);
    focus_at(&shell, &editor, 0, cx);
    shell.keys("escape", cx);
    assert!(holds_focus(&shell, "item-done", cx), "Escape focused Done");
    assert!(host.read_with(cx, |host, _| host.is_rich()));
    shell.keys("secondary-shift-m", cx);
    assert!(!host.read_with(cx, |host, _| host.is_rich()), "Markdown");
    shell.keys("secondary-shift-m", cx);
    assert!(host.read_with(cx, |host, _| host.is_rich()), "Rich text");
}

/// Decision 10: a link the edit editor reports goes where the tab's other
/// links go: a kind that is not opened says so in the link banner.
#[gpui_kit::test]
async fn a_link_in_the_edit_editor_is_followed_by_the_tab(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Linked", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    let host = edit(&shell, cx).await;
    host.update(cx, |_, cx| {
        cx.emit(crate::desktop::model::links::LinkTarget::Unsafe)
    });
    shell.run(cx);
    assert_eq!(
        shell.label("item-link", cx).as_deref(),
        Some(super::LINK_NOT_OPENED)
    );
}

/// The pill reads the phase while a session is made ready or lives, and
/// the item while viewing.
#[test]
fn the_pill_reads_the_phase() {
    let request = DesktopRequestId(7);
    assert_eq!(edit_state(Phase::Viewing, false), None);
    for phase in [Phase::Preparing(request), Phase::Loading] {
        assert_eq!(edit_state(phase, false), Some(PillState::Preparing));
    }
    assert_eq!(
        edit_state(Phase::Editing, true),
        Some(PillState::EditingUnsaved)
    );
    assert_eq!(
        edit_state(Phase::Editing, false),
        Some(PillState::EditingSaved { not_shared: false })
    );
    assert_eq!(edit_state(Phase::Saving, true), Some(PillState::Saving));
    assert_eq!(edit_state(Phase::Review, true), Some(PillState::Review));
    assert_eq!(pill_words(PillState::Preparing), GETTING_READY);
    assert_eq!(pill_words(PillState::EditingUnsaved), EDITING_UNSAVED);
    assert_eq!(
        pill_words(PillState::EditingSaved { not_shared: false }),
        EDITING_SAVED
    );
}

/// A new session is neither saved nor kept: Tasks 15 and 16 set them.
#[gpui_kit::test]
async fn a_new_session_is_neither_saved_nor_kept(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, TICKET, "Fresh", "");
    });
    open(&shell, ticket(&root, TICKET), cx);
    edit(&shell, cx).await;
    let (saved, kept) = pane(&shell, cx).read_with(cx, |pane, _| {
        let session = pane.edit_session().unwrap();
        (session.saved(), session.kept())
    });
    assert!(!saved && !kept);
}

/// Save, the session after a save, and the save states (Task 15).
#[path = "edit_save_tests.rs"]
mod save;

/// Done, the kept session and closing the tab (Task 16).
#[path = "edit_done_tests.rs"]
mod done;

/// Release on leave (Task 17).
#[path = "edit_release_tests.rs"]
mod release;
