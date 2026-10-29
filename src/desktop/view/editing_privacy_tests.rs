//! D3's privacy sweep (Cycle contract 14, design "Privacy"; Task 26). It
//! extends D2's (`drafts::privacy_tests`) across D3's flows. Three
//! sentinels, each unique, are planted in bodies: a body text, a link
//! target and an image path. The flows are driven with the library's real
//! answers: viewing, with the comment editors in the discussion; the
//! details sheet, viewing and editing, a field edited; Edit; a save; a save
//! the library refuses as `invalid_input` (a NUL in Project, which the
//! sheet lets through and the library's canonical check refuses); a stale
//! save, which opens the review; a release; a create; a move; and a
//! repair. After each step, every surface that may show text is read back
//! from the window's own models and frame:
//!
//! - every operation entry, its command's label and its envelope message;
//! - every notification (the desktop shows none);
//! - every label and value the window draws outside the places the design
//!   puts that text (`ALLOWED`), each dialog's included, and each dialog's
//!   own labels again as its message;
//! - the `Debug` and `Display` of every answer the desktop received: each
//!   finished request's outcome, code, effects, recovery and message, and
//!   each draft error kind the registry lists.
//!
//! The desktop logs nothing, so there is no log to read. The sentinels'
//! appearances inside `ALLOWED` are collected too, so a test can show the
//! scan sees drawn text.

use std::path::Path;

use gpui_kit::{ElementId, Entity, Focusable, TestAppContext, component::WindowExt};
use manyhands::{drafts::DraftError, repository::ItemDtoKind, results::ResultCode};

use crate::desktop::{
    model::{edit::fields::Field, navigation::OpenTarget},
    services::requests::{DesktopRequestId, Intent, RequestState},
    test_support::fixtures::{Fixture, item_id, ticket_path, ticket_source},
    view::{
        editor::host::EditorHost,
        harness::{Shell, key},
        item::ItemPane,
    },
};

const BODY: &str = "SENTINEL-D3-BODY-4c1e";
const LINK: &str = "SENTINEL-D3-LINK-8a2f";
const IMAGE: &str = "SENTINEL-D3-IMAGE-b7d3.png";
const SENTINELS: [&str; 3] = [BODY, LINK, IMAGE];

/// Where the design puts a body's text, so a sentinel may show there
/// (Task 26 notes, obligation 5): the read view, the edit editor's and the
/// review's draft section (R-38), a comment's read editor, the repair
/// text area, the toolbar's link field, an Inspect tab's whole file
/// (R-64), and the nearest-match banner's path.
const ALLOWED: [&str; 7] = [
    "item-read",
    "draft-section",
    "comment-body",
    "repair-source",
    "toolbar-link-popover",
    "item-source",
    "item-link",
];

const EDITED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FY0";
const RELEASED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FY1";
const COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FY2";
const MOVED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FY3";
const REPAIRED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FY4";
const MARKER: &str = "docs/marker.md";

/// A body holding the three sentinels: its text, a link and an image.
fn sentinel_body() -> String {
    format!("{BODY} is the body.\n\nSee [the notes]({LINK}).\n\n![]({IMAGE})\n")
}

/// A ticket's source with the sentinel body.
fn ticket_with_sentinels(id: &str, title: &str) -> String {
    ticket_source(id, title, "").replace(&format!("Body of {title}.\n"), &sentinel_body())
}

/// A comment on `item` whose body holds the sentinels.
fn comment_with_sentinels(item: &str, id: &str) -> String {
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: comment\nid: {id}\nitem_id: {item}\n\
         created_at: 2026-09-30T12:00:00Z\ncreated_by: Ada Lovelace <ada@example.invalid>\n\
         ---\n{}",
        sentinel_body()
    )
}

fn ticket(root: &Path, id: &str) -> OpenTarget {
    OpenTarget::Item {
        repo: key(root),
        kind: ItemDtoKind::Ticket,
        id: item_id(id),
    }
}

fn open(shell: &Shell, target: OpenTarget, cx: &mut TestAppContext) {
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
}

fn pane(shell: &Shell, cx: &mut TestAppContext) -> Entity<ItemPane> {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.item_pane())
        .expect("an item tab is shown")
}

/// The name of an element ID, its index aside.
fn name(id: &ElementId) -> Option<&str> {
    match id {
        ElementId::Name(name) | ElementId::NamedInteger(name, _) => Some(name),
        _ => None,
    }
}

/// The first place in `ALLOWED` that `path` is inside, if any.
fn allowed(path: &[ElementId]) -> Option<&'static str> {
    path.iter()
        .filter_map(name)
        .find_map(|name| ALLOWED.iter().copied().find(|allowed| *allowed == name))
}

/// Everything the sweep read, by where it was read, and where inside
/// `ALLOWED` a sentinel showed.
#[derive(Default)]
struct Seen {
    read: Vec<(&'static str, String)>,
    allowed: Vec<(&'static str, String)>,
    answers: Vec<String>,
}

impl Seen {
    fn add(&mut self, surface: &'static str, texts: impl IntoIterator<Item = String>) {
        self.read
            .extend(texts.into_iter().map(|text| (surface, text)));
    }

    /// The window's surfaces as they stand now: the operation entries, and
    /// every label and value drawn, outside `ALLOWED` and inside it; no
    /// notification.
    fn window(&mut self, shell: &Shell, cx: &mut TestAppContext) {
        self.add("operation entry", operation_entries(shell, cx));
        let drawn: Vec<(Option<&'static str>, String)> = shell.frame(cx, |window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .iter()
                .flat_map(|snapshot| {
                    let at = allowed(snapshot.path());
                    [snapshot.label(), snapshot.value()]
                        .into_iter()
                        .flatten()
                        .map(move |text| (at, text.to_owned()))
                        .collect::<Vec<_>>()
                })
                .collect()
        });
        for (at, text) in drawn {
            match at {
                Some(at) => {
                    if SENTINELS.iter().any(|sentinel| text.contains(sentinel)) {
                        self.allowed.push((at, text));
                    }
                }
                None => self.read.push(("drawn", text)),
            }
        }
        let notifications = shell.frame(cx, |window, cx| window.notifications(cx).len());
        assert_eq!(notifications, 0, "the desktop shows no notification");
    }

    /// The labels and values drawn inside the open dialog `dialog`: its
    /// message.
    fn dialog(&mut self, shell: &Shell, dialog: &'static str, cx: &mut TestAppContext) {
        let within = ElementId::from(dialog);
        let texts: Vec<String> = shell.frame(cx, |window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .iter()
                .filter(|snapshot| snapshot.path().contains(&within))
                .flat_map(|snapshot| {
                    [snapshot.label(), snapshot.value()]
                        .into_iter()
                        .flatten()
                        .map(str::to_owned)
                        .collect::<Vec<_>>()
                })
                .collect()
        });
        assert!(!texts.is_empty(), "{dialog} is drawn");
        self.add("dialog message", texts);
        self.window(shell, cx);
    }

    /// The `Debug` and `Display` of every answer the desktop received so
    /// far, and of each draft error the registry lists.
    fn answers(&mut self, shell: &Shell, cx: &mut TestAppContext) {
        let answers: Vec<String> = shell.operations(cx).read_with(cx, |operations, _| {
            operations
                .requests()
                .iter()
                .filter_map(|request| match &request.state {
                    RequestState::Finished(outcome, code, effects, recovery, message) => {
                        Some(format!(
                            "{outcome:?} {code:?} {} {effects:?} {recovery:?} {message:?} \
                             {message}",
                            code.as_str()
                        ))
                    }
                    RequestState::NotSent(why) => Some(format!("{why:?}")),
                    _ => None,
                })
                .collect()
        });
        self.answers.extend(answers);
        let drafts = shell
            .workspace
            .read_with(cx, |workspace, _| workspace.services().drafts().clone());
        let failures = drafts.read_with(cx, |registry, _| registry.flush_failures());
        for (_, title, kind) in failures {
            let error = DraftError { kind };
            self.add("flush failure title", [title]);
            self.answers.push(format!("{error:?} {error}"));
        }
    }

    fn surfaces(&self, surface: &str) -> usize {
        self.read.iter().filter(|(at, _)| *at == surface).count()
    }

    /// No surface read holds a sentinel, nor any answer's rendering.
    fn holds_none(&self) {
        for (surface, text) in &self.read {
            for sentinel in SENTINELS {
                assert!(!text.contains(sentinel), "{surface}: {text}");
            }
        }
        assert!(!self.answers.is_empty(), "answers were read");
        for answer in &self.answers {
            for sentinel in SENTINELS {
                assert!(!answer.contains(sentinel), "an answer: {answer}");
            }
        }
    }
}

/// Every operation entry: its command's label and, once finished, the
/// envelope message the operation area shows.
fn operation_entries(shell: &Shell, cx: &mut TestAppContext) -> Vec<String> {
    shell.operations(cx).read_with(cx, |operations, _| {
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
            .collect()
    })
}

/// Waits until the newest request that `pick` picks has ended, and gives
/// its code and envelope message.
async fn ended(
    shell: &Shell,
    pick: impl Fn(&Intent) -> bool,
    cx: &mut TestAppContext,
) -> (ResultCode, String) {
    let id: DesktopRequestId = *shell
        .requests(cx, |request| pick(&request.intent))
        .first()
        .expect("the request was made");
    shell.until_ended(id, cx).await;
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    shell.operations(cx).read_with(cx, |operations, _| {
        match &operations.request(id).expect("kept").state {
            RequestState::Finished(_, code, .., message) => (*code, message.clone()),
            _ => panic!("the request finished"),
        }
    })
}

/// The shown tab's edit host, once its editor opened.
async fn until_editing(shell: &Shell, cx: &mut TestAppContext) -> Entity<EditorHost> {
    shell
        .until_drawn(cx, "the edit editor opened", |shell, cx| {
            pane(shell, cx).read_with(cx, |pane, _| pane.edit_host().is_some())
        })
        .await;
    pane(shell, cx)
        .read_with(cx, |pane, _| pane.edit_host())
        .expect("editing")
}

/// Types `typed` at the end of the edit editor's text, through the window.
fn type_at_end(shell: &Shell, host: &Entity<EditorHost>, typed: &str, cx: &mut TestAppContext) {
    let editor = host.read_with(cx, |host, _| host.editor().clone());
    let end = editor.read_with(cx, |editor, _| editor.text().len());
    editor.update(cx, |editor, cx| editor.set_cursor(end, cx));
    let handle = editor.read_with(cx, |editor, cx| editor.focus_handle(cx));
    shell.frame(cx, |window, cx| window.focus(&handle, cx));
    shell.frame(cx, |_, _| ());
    shell.visual(cx).simulate_input(typed);
    shell.run(cx);
}

/// Opens the sheet from All details, focuses the control of `field`, and
/// types `typed` there, through the window.
fn type_in_sheet(shell: &Shell, field: Field, typed: &str, cx: &mut TestAppContext) {
    shell.click("chip-all", cx);
    assert!(shell.frame(cx, |window, cx| window.has_active_sheet(cx)));
    let input = pane(shell, cx)
        .read_with(cx, |pane, _| pane.sheet_input(field))
        .expect("the sheet's control");
    shell.frame(cx, |window, cx| {
        input.update(cx, |input, cx| input.focus(window, cx))
    });
    shell.frame(cx, |_, _| ());
    shell.keys("end", cx);
    shell.visual(cx).simulate_input(typed);
    shell.run(cx);
}

fn is_save(intent: &Intent) -> bool {
    matches!(
        intent,
        Intent::TicketSave { .. } | Intent::DocumentSave { .. }
    )
}

/// Viewing, the comment editors, the details sheet viewing and editing, a
/// save, an `invalid_input` refusal, a stale save into the review, and a
/// release: no sentinel reaches an entry, a notification, a dialog, a
/// drawn label or an answer.
#[gpui_kit::test]
async fn sentinels_reach_no_surface_through_editing_saving_and_release(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    for (id, title) in [(EDITED, "Private"), (RELEASED, "Released")] {
        let source = ticket_with_sentinels(id, title);
        fixture.commit_file(&a, &ticket_path(id), source.as_bytes());
    }
    fixture.commit_file(
        &a,
        &format!(".manyhands/comments/{EDITED}/{COMMENT}.md"),
        comment_with_sentinels(EDITED, COMMENT).as_bytes(),
    );
    fixture.refresh(&a);
    let shell = Shell::open(fixture, cx);
    let mut seen = Seen::default();

    // Viewing: the read view and the comment's read editor hold them.
    open(&shell, ticket(&a, EDITED), cx);
    shell.frame(cx, |_, _| ());
    let (read, comment) = pane(&shell, cx).read_with(cx, |pane, cx| {
        let read = pane.read_editor(cx).expect("the read view");
        let comment = pane.comment_view(COMMENT).expect("the comment's editor");
        (read, comment.read(cx).editor().clone())
    });
    for editor in [read, comment] {
        let text = editor.read_with(cx, |editor, _| editor.text().to_owned());
        assert!(SENTINELS.iter().all(|sentinel| text.contains(sentinel)));
    }
    seen.window(&shell, cx);

    // The details sheet while viewing.
    shell.click("chip-all", cx);
    seen.window(&shell, cx);
    shell.keys("escape", cx);

    // Edit, a typed change, and the sheet while editing, its Title edited.
    shell.click("item-edit", cx);
    let host = until_editing(&shell, cx).await;
    type_at_end(&shell, &host, " more", cx);
    seen.window(&shell, cx);
    type_in_sheet(&shell, Field::Title, " edited", cx);
    seen.window(&shell, cx);
    shell.keys("escape", cx);

    // A save.
    shell.click("item-save", cx);
    let (code, message) = ended(&shell, is_save, cx).await;
    assert_eq!(code, ResultCode::Ok);
    seen.add("envelope message", [message]);
    seen.window(&shell, cx);

    // A save the library refuses as `invalid_input`: a NUL in Project.
    type_at_end(&shell, &host, " again", cx);
    type_in_sheet(&shell, Field::Project, "\u{0}", cx);
    shell.keys("escape", cx);
    shell.click("item-save", cx);
    let (code, message) = ended(&shell, is_save, cx).await;
    assert_eq!(code, ResultCode::InvalidInput);
    assert!(!message.is_empty(), "the refusal says why");
    seen.add("envelope message", [message]);
    seen.window(&shell, cx);
    type_in_sheet(&shell, Field::Project, "", cx);
    shell.keys("secondary-a backspace escape", cx);

    // A stale save: the item was saved since; the review opens.
    shell.fixture.save_ticket(&a, EDITED, "Theirs");
    type_at_end(&shell, &host, " stale", cx);
    shell.click("item-save", cx);
    let (code, message) = ended(&shell, is_save, cx).await;
    assert_eq!(code, ResultCode::ExternalChange);
    seen.add("envelope message", [message]);
    shell
        .until_drawn(cx, "the review opened", |shell, cx| {
            shell.present("review-use", cx)
        })
        .await;
    seen.window(&shell, cx);
    assert!(
        seen.allowed
            .iter()
            .any(|(at, text)| *at == "draft-section" && text.contains(BODY)),
        "the scan sees drawn text: the review shows the body"
    );
    shell.click("review-cancel", cx);
    seen.window(&shell, cx);

    // A release: Edit, then Done with nothing changed.
    open(&shell, ticket(&a, RELEASED), cx);
    shell.click("item-edit", cx);
    until_editing(&shell, cx).await;
    shell.click("item-done", cx);
    let (code, message) = ended(
        &shell,
        |intent| matches!(intent, Intent::ContextRelease { .. }),
        cx,
    )
    .await;
    assert_eq!(code, ResultCode::Ok);
    seen.add("envelope message", [message]);
    seen.window(&shell, cx);

    seen.answers(&shell, cx);
    for surface in ["operation entry", "envelope message", "drawn"] {
        assert!(seen.surfaces(surface) > 0, "{surface} was read");
    }
    seen.holds_none();
}

/// A nonconforming document, under `docs/`, its front matter only the
/// marker, with the sentinel body.
fn marker_with_sentinels() -> String {
    format!("---\nmanyhands_managed: true\n---\n{}", sentinel_body())
}

/// A managed document's source at `id`, with the sentinel body.
fn document_with_sentinels(id: &str, title: &str) -> String {
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: {id}\ntitle: {title}\n---\n{}",
        sentinel_body()
    )
}

/// A create, a move and a repair, each of a body holding the sentinels:
/// none reaches an entry, a notification, a dialog, a drawn label or an
/// answer.
#[gpui_kit::test]
async fn sentinels_reach_no_surface_through_create_move_and_repair(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let fixture = Fixture::new();
    let a = fixture.create_repository("a");
    fixture.create_folder(&a, "docs/archive");
    fixture.commit_file(
        &a,
        "docs/moved.md",
        document_with_sentinels(MOVED, "Moved").as_bytes(),
    );
    fixture.commit_file(&a, MARKER, marker_with_sentinels().as_bytes());
    fixture.refresh(&a);
    let shell = Shell::open(fixture, cx);
    shell.select(&a, cx);
    let mut seen = Seen::default();

    // Create: New document…, its body typed in Markdown, then Save.
    shell.click("new-document-button", cx);
    shell.visual(cx).simulate_input("Created");
    shell.run(cx);
    seen.dialog(&shell, "create-dialog", cx);
    shell.keys("tab tab secondary-a", cx);
    shell.visual(cx).simulate_input("created.md");
    shell.run(cx);
    shell
        .until_drawn(cx, "Create offered", |shell, cx| {
            !shell.present("create-ok-note", cx)
        })
        .await;
    shell.keys("enter", cx);
    let host = until_editing(&shell, cx).await;
    host.update(cx, |host, cx| host.set_rich(false, cx));
    type_at_end(&shell, &host, &sentinel_body(), cx);
    seen.window(&shell, cx);
    shell.click("item-save", cx);
    let (code, message) = ended(
        &shell,
        |intent| matches!(intent, Intent::DocumentCreate { .. }),
        cx,
    )
    .await;
    assert_eq!(code, ResultCode::Ok);
    seen.add("envelope message", [message]);
    seen.window(&shell, cx);

    // Move: from the sheet, into `docs/archive`.
    open(
        &shell,
        OpenTarget::Item {
            repo: key(&a),
            kind: ItemDtoKind::Document,
            id: item_id(MOVED),
        },
        cx,
    );
    shell.click("chip-all", cx);
    shell.click("sheet-move-button", cx);
    shell.keys("secondary-a", cx);
    shell.visual(cx).simulate_input("docs/archive");
    shell.run(cx);
    shell
        .until_drawn(cx, "Move offered", |shell, cx| {
            !shell.present("move-ok-note", cx)
        })
        .await;
    seen.dialog(&shell, "move-dialog", cx);
    shell.keys("enter", cx);
    let (code, message) = ended(
        &shell,
        |intent| matches!(intent, Intent::DocumentMove { .. }),
        cx,
    )
    .await;
    assert_eq!(code, ResultCode::Ok);
    seen.add("envelope message", [message]);
    seen.window(&shell, cx);
    shell.keys("escape", cx);

    // Repair: the marker's front matter completed, the preview, confirmed.
    open(
        &shell,
        OpenTarget::Inspect {
            repo: key(&a),
            worktree: key(&a),
            path: MARKER.to_owned(),
        },
        cx,
    );
    seen.window(&shell, cx);
    shell.click("item-repair-button", cx);
    shell.keys("down down", cx);
    shell.visual(cx).simulate_input(&format!(
        "manyhands_kind: document\nid: {REPAIRED}\ntitle: Repaired\n"
    ));
    shell.run(cx);
    seen.window(&shell, cx);
    shell.click("repair-preview-button", cx);
    shell
        .until_drawn(cx, "the preview", |shell, cx| {
            shell.present("confirm-ok", cx) && shell.present("confirm-expires", cx)
        })
        .await;
    seen.dialog(&shell, "confirm-dialog", cx);
    shell.keys("enter", cx);
    let (code, message) = ended(
        &shell,
        |intent| matches!(intent, Intent::DocumentRepair { .. }),
        cx,
    )
    .await;
    assert_eq!(code, ResultCode::Ok);
    seen.add("envelope message", [message]);
    shell
        .until_drawn(
            cx,
            "the tab following the repaired document",
            |shell, cx| shell.label("item-title", cx).as_deref() == Some("Repaired"),
        )
        .await;
    seen.window(&shell, cx);

    seen.answers(&shell, cx);
    for surface in [
        "operation entry",
        "envelope message",
        "drawn",
        "dialog message",
    ] {
        assert!(seen.surfaces(surface) > 0, "{surface} was read");
    }
    seen.holds_none();
    for place in ["repair-source", "item-source"] {
        assert!(
            seen.allowed
                .iter()
                .any(|(at, text)| *at == place && text.contains(BODY)),
            "the scan sees drawn text: {place} shows the file"
        );
    }
}
