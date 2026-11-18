//! P1's rows for the item area's dialogs: the repair discard, the composer
//! discard, Recovery off and Replace your edits (Wave 04 Cycle 05a design
//! A3, A4). Each is reached the way its own test file reaches it, by the
//! keys or the control that opens it in the window. Escape and Enter each
//! close it, and the row reads what each did to the text or the work the
//! dialog guards, and where focus is.

use gpui_kit::{Entity, TestAppContext, component::input::Position};
use manyhands::{repository::ItemDtoKind, results::ResultCode};

use super::{
    Table, dialog_itself,
    dialogs::{back_at, origin},
    focus_at,
};
use crate::desktop::{
    model::{drafts::IDLE, navigation::OpenTarget},
    services::requests::{DesktopRequestId, Intent, RequestState},
    test_support::{
        drafts::{Actor, second_actor},
        fixtures::{Fixture, item_id},
    },
    view::{
        editor::host::EditorHost,
        harness::{Shell, key},
        item::ItemPane,
    },
};

const MARKER: &str = "docs/marker.md";
const DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG6";
const COMMENTED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FK0";
const THEIR_BODY: &str = "Their body.\n";

pub(super) async fn item_dialog_keys(table: &mut Table, cx: &mut TestAppContext) {
    repair_discard_keys(table, cx);
    composer_discard_keys(table, cx);
    #[cfg(unix)]
    recovery_off_keys(table, cx).await;
    replace_keys(table, cx).await;
}

fn pane(shell: &Shell, cx: &mut TestAppContext) -> Entity<ItemPane> {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.item_pane())
        .expect("an item tab is shown")
}

fn open(shell: &Shell, target: OpenTarget, cx: &mut TestAppContext) {
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
}

/// Edited repair text, closed with its tab, asks "Discard your repair?":
/// Cancel is its default, so Enter and Escape keep the view and its text.
fn repair_discard_keys(table: &mut Table, cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_nonconforming_document(&root, MARKER);
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    open(
        &shell,
        OpenTarget::Inspect {
            repo: key(&root),
            worktree: key(&root),
            path: MARKER.to_owned(),
        },
        cx,
    );
    shell.click("item-repair-button", cx);
    assert!(shell.present("repair-source", cx), "the repair view opens");
    let state = pane(&shell, cx)
        .read_with(cx, |pane, _| pane.repair_state())
        .expect("the repair view is open");
    shell.frame(cx, |window, cx| {
        state.update(cx, |state, cx| {
            state.set_cursor_position(Position::new(0, 0), window, cx)
        })
    });
    shell.frame(cx, |_, _| ());
    shell.visual(cx).simulate_input("x");
    shell.run(cx);
    let typed = state.read_with(cx, |state, _| state.value().to_string());
    let origin = origin(&shell, cx);

    for key in ["escape", "enter"] {
        shell.keys("secondary-w", cx);
        assert!(shell.present("repair-discard-dialog", cx), "it asks");
        if key == "escape" {
            let at = focus_at(&shell, cx);
            table.fact(
                "Repair discard, on open",
                at == Some(dialog_itself()),
                &format!("focus on the dialog itself (it is on {at:?})"),
            );
        }
        shell.keys(key, cx);
        let keeps = state.read_with(cx, |state, _| state.value().to_string()) == typed;
        table.fact(
            &format!("Repair discard, {key} closes it"),
            !shell.present("repair-discard-dialog", cx),
            "the dialog closed",
        );
        table.fact(
            &format!("Repair discard, {key} keeps the tab and its text"),
            shell.present("repair-source", cx) && keeps,
            "the repair view open with its text",
        );
        table.fact(
            &format!("Repair discard, {key} gives focus back"),
            back_at(&shell, &origin, cx),
            "focus on the text it was asked from",
        );
    }
}

/// A comment's composer with text, left by Escape, asks "Discard this
/// comment?": Keep editing has focus as it opens, and Enter and Escape
/// each keep the text and focus the editor.
fn composer_discard_keys(table: &mut Table, cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, COMMENTED, "Main", "project: p\nteam: t\n");
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    shell.resize_keeping_content(1440., 1400., cx);
    open(
        &shell,
        OpenTarget::Item {
            repo: key(&root),
            kind: ItemDtoKind::Ticket,
            id: item_id(COMMENTED),
        },
        cx,
    );
    shell.click("item-comments", cx);
    shell.click("new-comment-start", cx);
    shell.frame(cx, |_, _| ());
    shell.run(cx);
    let editor = pane(&shell, cx)
        .read_with(cx, |pane, _| pane.new_composer())
        .expect("the new comment's composer")
        .read_with(cx, |composer, _| composer.editor().clone());
    let handle = editor.read_with(cx, |editor, cx| {
        gpui_kit::Focusable::focus_handle(editor, cx)
    });
    assert!(
        shell.frame(cx, |window, _| handle.is_focused(window)),
        "a click opens the composer with focus in its editor"
    );
    shell.visual(cx).simulate_input("Not this one.");
    shell.run(cx);

    for key in ["escape", "enter"] {
        shell.keys("escape", cx);
        assert!(shell.present("composer-discard-dialog", cx), "it asks");
        if key == "escape" {
            table.fact(
                "Composer discard, on open",
                shell.holds_focus("composer-discard-keep-stop", cx),
                "focus on Keep editing",
            );
        }
        shell.keys(key, cx);
        table.fact(
            &format!("Composer discard, {key} closes it"),
            !shell.present("composer-discard-dialog", cx),
            "the dialog closed",
        );
        table.fact(
            &format!("Composer discard, {key} keeps the text"),
            editor.read_with(cx, |editor, _| editor.text() == "Not this one."),
            "the text kept",
        );
        table.fact(
            &format!("Composer discard, {key} gives focus back"),
            back_at(&shell, &handle, cx),
            "focus in the editor",
        );
    }
}

/// An unavailable draft store is said before editing, in Recovery off:
/// Escape makes nothing and gives focus back; Enter is Continue, which goes
/// on to edit.
#[cfg(unix)]
async fn recovery_off_keys(table: &mut Table, cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let elsewhere = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(elsewhere.path(), fixture.data_dir().join("drafts")).unwrap();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, COMMENTED, "Unprotected", "");
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    open(
        &shell,
        OpenTarget::Item {
            repo: key(&root),
            kind: ItemDtoKind::Ticket,
            id: item_id(COMMENTED),
        },
        cx,
    );
    let origin = origin(&shell, cx);
    let prepares = |shell: &Shell, cx: &mut TestAppContext| {
        shell
            .requests(cx, |request| {
                matches!(request.intent, Intent::ContextPrepare { .. })
            })
            .len()
    };

    shell.click("item-edit", cx);
    assert!(shell.present("recovery-off-dialog", cx), "it asks");
    let at = focus_at(&shell, cx);
    table.fact(
        "Recovery off, on open",
        at == Some(dialog_itself()),
        &format!("focus on the dialog itself (it is on {at:?})"),
    );
    shell.keys("escape", cx);
    table.fact(
        "Recovery off, Escape closes it",
        !shell.present("recovery-off-dialog", cx),
        "the dialog closed",
    );
    table.fact(
        "Recovery off, Escape makes nothing",
        prepares(&shell, cx) == 0,
        "no prepare",
    );
    table.fact(
        "Recovery off, Escape gives focus back",
        back_at(&shell, &origin, cx),
        "focus where Edit was pressed",
    );

    shell.click("item-edit", cx);
    assert!(shell.present("recovery-off-dialog", cx), "it asks again");
    shell.keys("enter", cx);
    table.fact(
        "Recovery off, Enter closes it",
        !shell.present("recovery-off-dialog", cx),
        "the dialog closed",
    );
    shell.until_editing(cx).await;
    table.fact(
        "Recovery off, Enter goes on to edit",
        prepares(&shell, cx) == 1,
        "one prepare, and the editor open",
    );
}

/// The body's choice of a review, switched after an edit, asks "Replace
/// your edits?": Keep my edits is its default, so Enter and Escape each
/// close it with the result as it was typed.
async fn replace_keys(table: &mut Table, cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.create_document(&root, DOCUMENT, "docs/one.md", "One");
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    open(
        &shell,
        OpenTarget::Item {
            repo: key(&root),
            kind: ItemDtoKind::Document,
            id: item_id(DOCUMENT),
        },
        cx,
    );
    shell.click("item-edit", cx);
    let host = shell.until_editing(cx).await;
    shell.type_at_end(&host, "x", cx);
    cx.executor().advance_clock(IDLE);
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    second_actor(
        &shell.fixture,
        Actor::SaveDocumentAs {
            root: &root,
            id: DOCUMENT,
            title: "One",
            body: THEIR_BODY,
        },
    );
    shell.click("item-save", cx);
    let saves = shell.requests(cx, |request| {
        matches!(request.intent, Intent::DocumentSave { .. })
    });
    assert_eq!(saves.len(), 1, "Save sent one save");
    shell.until_ended(saves[0], cx).await;
    assert_eq!(code(&shell, saves[0], cx), Some(ResultCode::ExternalChange));
    shell
        .until_drawn(cx, "the review", |shell, cx| {
            shell.present("review-use", cx)
        })
        .await;

    let body = shell
        .rows("review-field", cx)
        .iter()
        .position(|field| field == "Body")
        .expect("the review lists the Body");
    shell.click(("review-take-theirs", body), cx);
    let result = pane(&shell, cx)
        .read_with(cx, |pane, _| pane.review_host())
        .expect("the body's result is shown");
    shell.type_at_end(&result, "z", cx);
    let typed = text(&result, cx);
    let origin = origin(&shell, cx);

    for key in ["escape", "enter"] {
        shell.click(("review-keep-mine", body), cx);
        assert!(shell.present("review-replace-dialog", cx), "it asks first");
        if key == "escape" {
            let at = focus_at(&shell, cx);
            table.fact(
                "Replace your edits, on open",
                at == Some(dialog_itself()),
                &format!("focus on the dialog itself (it is on {at:?})"),
            );
        }
        shell.keys(key, cx);
        table.fact(
            &format!("Replace your edits, {key} closes it"),
            !shell.present("review-replace-dialog", cx),
            "the dialog closed",
        );
        table.fact(
            &format!("Replace your edits, {key} keeps my edits"),
            text(&result, cx) == typed,
            "the result as typed",
        );
        table.fact(
            &format!("Replace your edits, {key} gives focus back"),
            back_at(&shell, &origin, cx),
            "focus where the choice was pressed",
        );
    }
}

fn text(host: &Entity<EditorHost>, cx: &mut TestAppContext) -> String {
    host.read_with(cx, |host, cx| host.text(cx))
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
