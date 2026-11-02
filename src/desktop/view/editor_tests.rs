//! A real editor in a headless window: typing and undo through the
//! window-level helpers (the feasibility test), the editor's keys, and its
//! load. The host and helpers here serve the guard's tests too.

use std::{cell::RefCell, rc::Rc};

use gpui_kit::{
    AppContext, Context, Entity, EventEmitter, FocusHandle, Focusable, InteractiveElement,
    IntoElement, KeyBinding, ParentElement, Render, Styled, TestAppContext, Window,
    component::Root, div, px,
};
use zorite_editor::{EditorEvent, EditorState};

use super::{open_editor, set_exact, step_over_blocks};
use crate::desktop::{
    model::editor_text::{load, tests as editor_text_tests},
    test_support::fixtures::Fixture,
    view::harness::Shell,
};

/// A view between two controls that Tab stops at, so focus has somewhere
/// to go before and after it.
pub(super) struct Host<V: 'static> {
    pub(super) before: FocusHandle,
    pub(super) view: Entity<V>,
    pub(super) after: FocusHandle,
}

impl<V: Render> Render for Host<V> {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .id("host-before")
                    .track_focus(&self.before)
                    .h(px(24.))
                    .child("Before"),
            )
            .child(self.view.clone())
            .child(
                div()
                    .id("host-after")
                    .track_focus(&self.after)
                    .h(px(24.))
                    .child("After"),
            )
    }
}

/// Opens the test shell, which binds the keys as startup does, and puts
/// `build`'s view in its window, under a new `Root`, in place of the
/// workspace.
pub(super) fn show<V: Render>(
    cx: &mut TestAppContext,
    build: impl FnOnce(&mut Window, &mut Context<V>) -> V,
) -> (Shell, Entity<Host<V>>) {
    let shell = Shell::open(Fixture::new(), cx);
    let made = Rc::new(RefCell::new(None));
    shell
        .window
        .update(cx, {
            let made = made.clone();
            move |_, window, cx| {
                window.replace_root(cx, |window, cx| {
                    let host = cx.new(|cx| Host {
                        before: cx.focus_handle().tab_stop(true),
                        view: cx.new(|cx| build(window, cx)),
                        after: cx.focus_handle().tab_stop(true),
                    });
                    *made.borrow_mut() = Some(host.clone());
                    Root::new(host, window, cx)
                });
            }
        })
        .unwrap();
    let host = made.borrow_mut().take().unwrap();
    shell.run(cx);
    (shell, host)
}

/// Every event `entity` emits from now on.
pub(super) fn events<T: EventEmitter<EditorEvent>>(
    entity: &Entity<T>,
    cx: &mut TestAppContext,
) -> Rc<RefCell<Vec<EditorEvent>>> {
    let seen = Rc::new(RefCell::new(Vec::new()));
    cx.update({
        let seen = seen.clone();
        |cx| {
            cx.subscribe(entity, move |_, event: &EditorEvent, _| {
                seen.borrow_mut().push(event.clone())
            })
            .detach()
        }
    });
    seen
}

/// Focuses `editor`, then draws a frame, so it takes input.
pub(super) fn focus_and_draw(shell: &Shell, editor: &Entity<EditorState>, cx: &mut TestAppContext) {
    let handle = editor.read_with(cx, |editor, cx| editor.focus_handle(cx));
    shell.frame(cx, |window, cx| window.focus(&handle, cx));
    shell.frame(cx, |_, _| ());
}

/// Stage 1 of the design's feasibility proof: the shell's keys reach a
/// real editor, typed text lands with one `Changed`, and Ctrl-Z undoes it.
#[gpui_kit::test]
fn a_real_editor_takes_typing_and_undo_in_a_headless_window(cx: &mut TestAppContext) {
    let (shell, host) = show(cx, |window, cx| {
        EditorState::new(window, cx).with_text("abc")
    });
    let editor = host.read_with(cx, |host, _| host.view.clone());
    let seen = events(&editor, cx);
    editor.update(cx, |editor, cx| editor.set_cursor(3, cx));
    focus_and_draw(&shell, &editor, cx);
    seen.borrow_mut().clear();

    shell.visual(cx).simulate_input("d");
    assert_eq!(
        editor.read_with(cx, |editor, _| editor.text().to_owned()),
        "abcd"
    );
    let changed = seen
        .borrow()
        .iter()
        .filter(|event| **event == EditorEvent::Changed)
        .count();
    assert_eq!(changed, 1);

    shell.visual(cx).simulate_keystrokes("ctrl-z");
    assert_eq!(
        editor.read_with(cx, |editor, _| editor.text().to_owned()),
        "abc"
    );
}

/// `editor_text::load` is the editor's own load: an editor opened over
/// each golden fixture holds exactly what `load` predicts, and so does one
/// over a body with both CRLF line endings and a words-mixed `$$` pair
/// (Review Focus 1).
#[gpui_kit::test]
fn the_editor_reads_back_exactly_what_load_predicts(cx: &mut TestAppContext) {
    let shell = Shell::open(Fixture::new(), cx);
    let bodies = editor_text_tests::every_fixture()
        .map(|name| (name, editor_text_tests::fixture(name)))
        .chain([(
            "CRLF and a mixed pair",
            "First\r\nWhat if words $$E=mc^2$$ more\r\n".to_owned(),
        )]);
    for (name, body) in bodies {
        let editor = shell.frame(cx, |window, cx| open_editor(&body, window, cx));
        let text = editor.read_with(cx, |editor, _| editor.text().to_owned());
        assert!(text == load(&body), "{name}");
    }
}

gpui_kit::actions!(editor_tests, [AroundEscape]);

/// A bare editor inside a key context that binds Escape.
struct Around {
    editor: Entity<EditorState>,
    escapes: usize,
}

impl Render for Around {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .key_context("AroundEditor")
            .on_action(cx.listener(|this, _: &AroundEscape, _, _| this.escapes += 1))
            .child(self.editor.clone())
    }
}

/// R-8 and R-59: underline, which writes HTML, has no key, and Escape is
/// not the editor's `Dismiss`, which never propagates, so it reaches the
/// binding around the editor.
#[gpui_kit::test]
fn underline_has_no_key_and_escape_reaches_the_bindings_around_the_editor(cx: &mut TestAppContext) {
    let (shell, host) = show(cx, |window, cx| Around {
        editor: cx.new(|cx| EditorState::new(window, cx).with_text("abc")),
        escapes: 0,
    });
    cx.update(|cx| {
        cx.bind_keys([KeyBinding::new(
            "escape",
            AroundEscape,
            Some("AroundEditor"),
        )])
    });
    let around = host.read_with(cx, |host, _| host.view.clone());
    let editor = around.read_with(cx, |around, _| around.editor.clone());
    editor.update(cx, |editor, cx| editor.set_cursor(3, cx));
    focus_and_draw(&shell, &editor, cx);

    shell.visual(cx).simulate_keystrokes("ctrl-a");
    // Each on its own: a second underline would unwrap the first.
    for underline in ["ctrl-u", "cmd-u"] {
        shell.visual(cx).simulate_keystrokes(underline);
        assert_eq!(
            editor.read_with(cx, |editor, _| editor.text().to_owned()),
            "abc",
            "{underline}"
        );
    }

    shell.visual(cx).simulate_keystrokes("escape");
    assert_eq!(around.read_with(cx, |around, _| around.escapes), 1);
}

/// R-79: `set_exact` puts a text the editor already loaded back in byte
/// for byte, though the pin would load fact 81's shapes differently a
/// second time.
#[gpui_kit::test]
fn set_exact_puts_a_loaded_text_in_exactly(cx: &mut TestAppContext) {
    let (shell, host) = show(cx, |window, cx| {
        EditorState::new(window, cx).with_text("abc")
    });
    let editor = host.read_with(cx, |host, _| host.view.clone());
    editor.update(cx, |editor, cx| editor.set_cursor(3, cx));
    focus_and_draw(&shell, &editor, cx);
    let text = |cx: &mut TestAppContext| editor.read_with(cx, |editor, _| editor.text().to_owned());

    // A text a second pass leaves alone goes in through `set_text` alone,
    // which restarts undo: the "d" typed before is not undone.
    shell.visual(cx).simulate_input("d");
    let plain = load(&editor_text_tests::fixture("n2_mixed_pair"));
    assert!(load(&plain) == plain);
    cx.update(|cx| set_exact(&editor, &plain, cx));
    assert!(text(cx) == plain);
    shell.frame(cx, |_, _| ());
    shell.visual(cx).simulate_keystrokes("ctrl-z");
    assert!(text(cx) == plain, "undo reached past set_exact");

    // Fact 81's shapes, loaded once, go in exactly. One Ctrl-Z then leads
    // to the editor's own second pass of them (Known Limits).
    for name in editor_text_tests::SECOND_PASS {
        let once = load(&editor_text_tests::fixture(name));
        cx.update(|cx| set_exact(&editor, &once, cx));
        assert!(text(cx) == once, "{name}");
        shell.frame(cx, |_, _| ());
        shell.visual(cx).simulate_keystrokes("ctrl-z");
        let second = load(&once);
        assert!(second != once, "{name}");
        assert!(text(cx) == second, "{name}: one undo");
    }
}

/// A block event whose range no longer fits the text, as an editable
/// host can see after an edit, moves nothing and does not panic.
#[gpui_kit::test]
fn a_stale_block_range_moves_nothing(cx: &mut TestAppContext) {
    let (_shell, host) = show(cx, |window, cx| {
        EditorState::new(window, cx).with_text("abc")
    });
    let editor = host.read_with(cx, |host, _| host.view.clone());
    editor.update(cx, |editor, cx| editor.set_cursor(1, cx));
    for (inline, at_end) in [(false, false), (false, true), (true, false), (true, true)] {
        let events = [
            EditorEvent::EditMath {
                range: 10..20,
                source: "x".into(),
                at_end,
                inline,
            },
            EditorEvent::EditProperties {
                range: 10..20,
                source: "k:: v".into(),
                at_end,
                row: None,
            },
        ];
        for event in events {
            assert!(cx.update(|cx| step_over_blocks(&editor, &event, cx)));
            assert_eq!(editor.read_with(cx, |editor, _| editor.cursor()), 1);
        }
    }
}
