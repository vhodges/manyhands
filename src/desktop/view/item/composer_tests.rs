//! The comment composer (design, Lane C), drawn in the test shell between
//! two tab stops (`test_bench`): its compact editor and mode switch, the
//! mini toolbar, the labels and footer by remote and kind, the
//! placeholders, the draft indicator and a restored text's load. Each test
//! that types sets the cursor, focuses the editor and draws a frame first,
//! then types through the window (D3's editor-test rules).

use gpui_kit::TestAppContext;
use manyhands::drafts::{DraftErrorKind, ItemKind};
use time::OffsetDateTime;

use super::{
    test_bench::{Bench, REPLY_TO, Setup},
    toolbar::{CONTROLS, MiniControl},
};
use crate::desktop::model::{
    drafts::RecoveryState,
    editor_text::{load, tests as editor_text_tests},
};

/// One line; the formatting cases select "two".
const BODY: &str = "One two three";
/// "two" in `BODY`.
const TWO: (usize, usize) = (4, 7);
/// The address typed into the link field.
const ADDRESS: &str = "https://example.test";

/// What a formatting control makes of `BODY` with "two" selected.
fn formatted(control: MiniControl) -> &'static str {
    match control {
        MiniControl::Bold => "One **two** three",
        MiniControl::Italic => "One *two* three",
        MiniControl::Code => "One `two` three",
        MiniControl::Link => "One [two](https://example.test) three",
        MiniControl::Bullets => "- One two three",
        MiniControl::Rich | MiniControl::Markdown => panic!("not a formatting case"),
    }
}

/// The composer is a small editor of its own, with no page chrome: the
/// edit toolbar and its hint are not drawn, the text area is at least
/// 70 px for a new comment and 64 px for a reply and stays compact, and
/// the mini toolbar's Rich text / Markdown switch sets the mode without
/// touching the text. Tab from before the composer reaches its toolbar,
/// then its editor; typing there is a change.
#[gpui_kit::test]
fn the_composer_is_a_compact_editor_with_the_mode_switch(cx: &mut TestAppContext) {
    let bench = Bench::open(Setup::new(), cx);
    assert!(bench.present("composer", cx));
    assert!(!bench.present("toolbar", cx), "no edit toolbar");
    assert!(!bench.present("toolbar-hint", cx), "no edit hint");
    let new = bench.height("composer-editor", cx);
    assert!((70.0..120.0).contains(&new), "a new comment's area: {new}");
    assert_eq!(bench.checked(MiniControl::Rich.id(), cx), Some(true));
    assert_eq!(bench.checked(MiniControl::Markdown.id(), cx), Some(false));
    assert!(bench.rich(cx), "rich text from the start");

    // Tab enters at the toolbar, then the editor.
    let before = bench.stage.read_with(cx, |stage, _| stage.before.clone());
    bench
        .shell
        .frame(cx, move |window, cx| window.focus(&before, cx));
    bench.keys("tab", cx);
    assert!(bench.focused(MiniControl::Bold, cx), "the toolbar's stop");
    bench.keys("tab", cx);
    assert!(bench.editor_focused(cx), "then the editor");

    bench.place(0, cx);
    bench.input(BODY, cx);
    assert_eq!(bench.text(cx), BODY);
    assert!(bench.changes() >= 1, "typing is a change");
    assert!(
        bench.changed_to().as_deref() == Some(BODY),
        "the change carries the text"
    );

    // The switch: Markdown, then Rich text; the text and focus stay.
    bench.click(MiniControl::Markdown.id(), cx);
    assert!(!bench.rich(cx));
    assert_eq!(bench.checked(MiniControl::Markdown.id(), cx), Some(true));
    assert_eq!(bench.text(cx), BODY);
    assert!(
        bench.editor_focused(cx),
        "a click leaves focus in the editor"
    );
    bench.click(MiniControl::Rich.id(), cx);
    assert!(bench.rich(cx));
    assert_eq!(bench.text(cx), BODY);

    // A growing text grows the area.
    let short = bench.height("composer-editor", cx);
    bench.place(BODY.len(), cx);
    bench.input("\nline\nline\nline\nline\nline", cx);
    let tall = bench.height("composer-editor", cx);
    assert!(tall > short, "the area grew: {short} to {tall}");

    bench.reset(Setup::new().reply(), cx);
    let reply = bench.height("composer-editor", cx);
    assert!((64.0..70.0).contains(&reply), "a reply's area: {reply}");
}

/// Ctrl Shift M (the secondary key) switches the mode from the composer's
/// editor and from a toolbar control, and the text and focus stay.
#[gpui_kit::test]
fn ctrl_shift_m_switches_mode_in_a_composer(cx: &mut TestAppContext) {
    let bench = Bench::open(Setup::new(), cx);
    bench.place(0, cx);
    bench.input(BODY, cx);

    bench.keys("secondary-shift-m", cx);
    assert!(!bench.rich(cx), "the editor's key: Markdown");
    assert!(bench.editor_focused(cx));
    assert_eq!(bench.text(cx), BODY);
    bench.keys("secondary-shift-m", cx);
    assert!(bench.rich(cx), "and back");

    bench.focus_control(MiniControl::Italic, cx);
    bench.keys("secondary-shift-m", cx);
    assert!(!bench.rich(cx), "from a control");
    assert!(bench.focused(MiniControl::Italic, cx));
    assert_eq!(bench.text(cx), BODY);
}

/// Bold, Italic, Inline code, Link and Bulleted list format the
/// selection, by a click (focus stays in the editor) and by Enter on the
/// control; Link asks for its address. Left, Right, Home and End move
/// along the toolbar, which is one tab stop.
#[gpui_kit::test]
fn the_mini_toolbar_formats_the_selection_and_arrows_move_along_it(cx: &mut TestAppContext) {
    let setup = Setup {
        restored: BODY.to_owned(),
        ..Setup::new()
    };
    let bench = Bench::open(setup.clone(), cx);
    let formatting = [
        MiniControl::Bold,
        MiniControl::Italic,
        MiniControl::Code,
        MiniControl::Link,
        MiniControl::Bullets,
    ];
    for control in formatting {
        for keyboard in [false, true] {
            let case = format!("{control:?}, keyboard {keyboard}");
            bench.reset(setup.clone(), cx);
            bench.select(TWO.0, TWO.1, cx);
            if keyboard {
                bench.focus_control(control, cx);
                bench.press("enter", cx);
            } else {
                bench.click(control.id(), cx);
            }
            if control == MiniControl::Link {
                assert!(bench.link_field_focused(cx), "{case}: the address field");
                bench.input(ADDRESS, cx);
                bench.press("enter", cx);
                assert!(!bench.present("composer-link-field", cx), "{case}: closed");
            }
            assert_eq!(bench.text(cx), formatted(control), "{case}");
            if !keyboard && control != MiniControl::Link {
                assert!(bench.editor_focused(cx), "{case}: a click keeps focus");
            }
        }
    }

    // Escape closes the link field and gives focus back.
    bench.reset(setup.clone(), cx);
    bench.select(TWO.0, TWO.1, cx);
    bench.click(MiniControl::Link.id(), cx);
    bench.input("https://never", cx);
    bench.keys("escape", cx);
    assert!(!bench.present("composer-link-field", cx));
    assert!(bench.editor_focused(cx), "focus is back in the editor");
    assert_eq!(bench.text(cx), BODY);

    // The arrows, Home and End.
    bench.focus_control(MiniControl::Bold, cx);
    bench.keys("left", cx);
    assert!(
        bench.focused(MiniControl::Bold, cx),
        "Left stops at the start"
    );
    for next in CONTROLS.iter().skip(1) {
        bench.keys("right", cx);
        assert!(bench.focused(*next, cx), "Right to {next:?}");
    }
    bench.keys("right", cx);
    assert!(
        bench.focused(MiniControl::Markdown, cx),
        "Right stops at the end"
    );
    bench.keys("home", cx);
    assert!(bench.focused(MiniControl::Bold, cx), "Home");
    bench.keys("end", cx);
    assert!(bench.focused(MiniControl::Markdown, cx), "End");
    bench.keys("left", cx);
    assert!(bench.focused(MiniControl::Rich, cx), "Left");
    assert_eq!(bench.text(cx), BODY, "moving changed nothing");
}

/// The footer and buttons follow the remote, the item's kind and the
/// composer's kind (R-9), and a change of remote is drawn at once.
#[gpui_kit::test]
fn labels_and_footer_follow_the_remote_and_the_kind(cx: &mut TestAppContext) {
    let bench = Bench::open(Setup::new(), cx);
    for reply in [false, true] {
        for item in [ItemKind::Ticket, ItemKind::Document] {
            for remote in [true, false] {
                let case = format!("reply {reply}, {item:?}, remote {remote}");
                bench.reset(
                    Setup {
                        reply,
                        item,
                        remote,
                        restored: String::new(),
                    },
                    cx,
                );
                let noun = match item {
                    ItemKind::Ticket => "ticket",
                    ItemKind::Document => "document",
                };
                let footer = if remote {
                    format!(
                        "Posting also shares other saved work in this {noun}. \
                         Unsaved edits above are not included."
                    )
                } else {
                    "Saved on this computer. Unsaved edits above are not included.".to_owned()
                };
                assert_eq!(bench.label("composer-footer", cx), Some(footer), "{case}");
                let post = match (reply, remote) {
                    (false, true) => "Post and sync",
                    (false, false) => "Save comment locally",
                    (true, true) => "Reply and sync",
                    (true, false) => "Save reply locally",
                };
                assert_eq!(
                    bench.label("composer-post", cx).as_deref(),
                    Some(post),
                    "{case}"
                );
                if reply {
                    assert_eq!(
                        bench.label("composer-cancel", cx).as_deref(),
                        Some("Cancel"),
                        "{case}"
                    );
                    assert_eq!(
                        bench.label("composer-hint", cx).as_deref(),
                        Some("Ctrl Enter to post \u{b7} Esc to cancel"),
                        "{case}"
                    );
                    assert_eq!(
                        bench.label("composer-replying-to", cx),
                        Some(format!("Replying to {REPLY_TO}")),
                        "{case}"
                    );
                } else {
                    assert!(!bench.present("composer-cancel", cx), "{case}");
                    assert!(!bench.present("composer-hint", cx), "{case}");
                    assert!(!bench.present("composer-replying-to", cx), "{case}");
                }
            }
        }
    }

    // The remote can change while the composer is open.
    bench.reset(Setup::new(), cx);
    assert_eq!(
        bench.label("composer-post", cx).as_deref(),
        Some("Save comment locally")
    );
    bench.set_remote(true, cx);
    assert_eq!(
        bench.label("composer-post", cx).as_deref(),
        Some("Post and sync")
    );
}

/// A new comment and a reply ask differently; a reply shows the user's
/// avatar, a new comment none.
#[gpui_kit::test]
fn placeholders_differ_for_new_and_reply(cx: &mut TestAppContext) {
    let bench = Bench::open(Setup::new(), cx);
    let new = "Add a comment. Markdown works here.";
    assert_eq!(bench.label("composer-editor", cx).as_deref(), Some(new));
    let composer = bench.composer(cx);
    assert_eq!(composer.read_with(cx, |c, _| c.placeholder()), new);
    assert!(!bench.present("composer-avatar", cx));

    bench.reset(Setup::new().reply(), cx);
    let reply = "Write a reply. Markdown works here.";
    assert_eq!(bench.label("composer-editor", cx).as_deref(), Some(reply));
    let composer = bench.composer(cx);
    assert_eq!(composer.read_with(cx, |c, _| c.placeholder()), reply);
    assert_eq!(bench.label("composer-avatar", cx).as_deref(), Some("VH"));
}

/// The indicator says what its owner set: "Draft kept" for a written
/// draft, "Draft not kept. Post to protect your text." for a failed one,
/// and nothing while a write is pending or before any.
#[gpui_kit::test]
fn the_draft_indicator_shows_kept_and_not_kept(cx: &mut TestAppContext) {
    let bench = Bench::open(Setup::new(), cx);
    assert!(
        !bench.present("composer-draft", cx),
        "nothing before a draft"
    );

    bench.set_recovery(Some(RecoveryState::Saved(OffsetDateTime::now_utc())), cx);
    assert_eq!(
        bench.label("composer-draft", cx).as_deref(),
        Some("Draft kept")
    );

    bench.set_recovery(Some(RecoveryState::Pending), cx);
    assert!(
        !bench.present("composer-draft", cx),
        "no claim while pending"
    );

    bench.set_recovery(Some(RecoveryState::Failed(DraftErrorKind::NoSpace)), cx);
    assert_eq!(
        bench.label("composer-draft", cx).as_deref(),
        Some("Draft not kept. Post to protect your text.")
    );

    bench.set_recovery(None, cx);
    assert!(!bench.present("composer-draft", cx));
}

/// R-10: a restored text, which an editor loaded once, goes in exactly,
/// though a second load would rewrite its math fences, and its load is no
/// change. Typing afterwards is.
#[gpui_kit::test]
fn a_restored_text_with_math_fences_is_not_changed(cx: &mut TestAppContext) {
    let bench = Bench::open(Setup::new(), cx);
    let names = editor_text_tests::SECOND_PASS
        .into_iter()
        .chain(["n2_mixed_pair"]);
    for name in names {
        let once = load(&editor_text_tests::fixture(name));
        assert!(once.contains("$$"), "{name} has math fences");
        bench.reset(
            Setup {
                restored: once.clone(),
                ..Setup::new()
            },
            cx,
        );
        assert!(bench.text(cx) == once, "{name}: restored exactly");
        bench.frame(cx);
        assert_eq!(bench.changes(), 0, "{name}: the load is no change");
        bench.place(0, cx);
        bench.input("x", cx);
        assert!(bench.text(cx) == format!("x{once}"), "{name}: typed");
        assert!(bench.changes() >= 1, "{name}: typing is");
    }
}
