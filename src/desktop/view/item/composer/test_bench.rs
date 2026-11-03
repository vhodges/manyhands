//! The composer tests' bench: a composer in the test shell between two
//! tab stops, as the discussion holds it, and what a test does to it and
//! reads of it.

use std::{cell::RefCell, rc::Rc};

use gpui_kit::{
    AppContext, Context, ElementId, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, ParentElement, Render, Styled, TestAppContext, Window, component::Root, div, px,
    test::TestWindowExt,
};
use manyhands::drafts::ItemKind;
use zorite_editor::EditorState;

use super::{Composer, ComposerEvent, ComposerKind, toolbar::MiniControl};
use crate::desktop::{
    model::drafts::RecoveryState, test_support::fixtures::Fixture, view::harness::Shell,
};

/// The name a reply answers.
pub(super) const REPLY_TO: &str = "Jo Smith";
/// The user, whose avatar a reply shows.
const ME: &str = "Vince Hodges";

/// The composer between two tab stops, as the discussion holds it.
pub(super) struct Stage {
    pub(super) before: FocusHandle,
    composer: Entity<Composer>,
    after: FocusHandle,
}

impl Render for Stage {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .text_size(px(15.))
            .child(
                div()
                    .id("stage-before")
                    .track_focus(&self.before)
                    .h(px(24.))
                    .child("Before"),
            )
            .child(div().w(px(664.)).child(self.composer.clone()))
            .child(
                div()
                    .id("stage-after")
                    .track_focus(&self.after)
                    .h(px(24.))
                    .child("After"),
            )
    }
}

/// What a composer is made with.
#[derive(Clone)]
pub(super) struct Setup {
    pub(super) reply: bool,
    pub(super) item: ItemKind,
    pub(super) remote: bool,
    pub(super) restored: String,
}

impl Setup {
    pub(super) fn new() -> Self {
        Self {
            reply: false,
            item: ItemKind::Ticket,
            remote: false,
            restored: String::new(),
        }
    }

    pub(super) fn reply(mut self) -> Self {
        self.reply = true;
        self
    }

    pub(super) fn kind(&self) -> ComposerKind {
        if self.reply {
            ComposerKind::Reply {
                to: REPLY_TO.to_owned(),
                me: Some(ME.to_owned()),
            }
        } else {
            ComposerKind::New
        }
    }

    pub(super) fn build(&self, window: &mut Window, cx: &mut gpui_kit::App) -> Entity<Composer> {
        let setup = self.clone();
        cx.new(|cx| {
            Composer::new(
                setup.kind(),
                setup.item,
                setup.remote,
                &setup.restored,
                window,
                cx,
            )
        })
    }
}

/// A composer in the test shell, and the texts of the changes it emitted.
pub(super) struct Bench {
    pub(super) shell: Shell,
    pub(super) stage: Entity<Stage>,
    seen: Rc<RefCell<Vec<String>>>,
}

impl Bench {
    pub(super) fn open(setup: Setup, cx: &mut TestAppContext) -> Self {
        let shell = Shell::open(Fixture::new(), cx);
        let made = Rc::new(RefCell::new(None));
        shell
            .window
            .update(cx, {
                let made = made.clone();
                move |_, window, cx| {
                    window.replace_root(cx, |window, cx| {
                        let stage = cx.new(|cx| Stage {
                            before: cx.focus_handle().tab_stop(true),
                            composer: setup.build(window, cx),
                            after: cx.focus_handle().tab_stop(true),
                        });
                        *made.borrow_mut() = Some(stage.clone());
                        Root::new(stage, window, cx)
                    });
                }
            })
            .unwrap();
        let stage = made.borrow_mut().take().unwrap();
        shell.run(cx);
        let bench = Self {
            shell,
            stage,
            seen: Rc::default(),
        };
        bench.watch(cx);
        bench.frame(cx);
        bench
    }

    /// A new composer made with `setup`, in the last one's place.
    pub(super) fn reset(&self, setup: Setup, cx: &mut TestAppContext) {
        let stage = self.stage.clone();
        self.shell.frame(cx, move |window, cx| {
            stage.update(cx, |stage, cx| {
                stage.composer = setup.build(window, cx);
                cx.notify();
            })
        });
        self.watch(cx);
        self.frame(cx);
    }

    /// Records what the current composer emits, from now on.
    pub(super) fn watch(&self, cx: &mut TestAppContext) {
        self.seen.borrow_mut().clear();
        let composer = self.composer(cx);
        let seen = self.seen.clone();
        cx.update(|cx| {
            cx.subscribe(&composer, move |_, event: &ComposerEvent, _| {
                let ComposerEvent::Changed(text) = event;
                seen.borrow_mut().push(text.clone())
            })
            .detach()
        });
    }

    pub(super) fn frame(&self, cx: &mut TestAppContext) {
        self.shell.frame(cx, |_, _| ());
        self.shell.frame(cx, |_, _| ());
    }

    pub(super) fn composer(&self, cx: &mut TestAppContext) -> Entity<Composer> {
        self.stage.read_with(cx, |stage, _| stage.composer.clone())
    }

    pub(super) fn editor(&self, cx: &mut TestAppContext) -> Entity<EditorState> {
        let composer = self.composer(cx);
        composer.read_with(cx, |composer, _| composer.editor().clone())
    }

    pub(super) fn text(&self, cx: &mut TestAppContext) -> String {
        let composer = self.composer(cx);
        composer.read_with(cx, |composer, cx| composer.text(cx))
    }

    pub(super) fn rich(&self, cx: &mut TestAppContext) -> bool {
        let composer = self.composer(cx);
        composer.read_with(cx, |composer, _| composer.is_rich())
    }

    /// Puts the caret at `cursor`, focuses the editor and draws, then
    /// forgets what the composer emitted so far.
    pub(super) fn place(&self, cursor: usize, cx: &mut TestAppContext) {
        let editor = self.editor(cx);
        editor.update(cx, |editor, cx| editor.set_cursor(cursor, cx));
        let handle = editor.read_with(cx, |editor, cx| editor.focus_handle(cx));
        self.shell
            .frame(cx, move |window, cx| window.focus(&handle, cx));
        self.frame(cx);
        self.seen.borrow_mut().clear();
    }

    /// Selects `start..end` of the line from the keyboard.
    pub(super) fn select(&self, start: usize, end: usize, cx: &mut TestAppContext) {
        self.place(start, cx);
        let keys = vec!["shift-right"; end - start].join(" ");
        self.keys(&keys, cx);
    }

    pub(super) fn keys(&self, keystrokes: &str, cx: &mut TestAppContext) {
        self.shell.visual(cx).simulate_keystrokes(keystrokes);
        self.frame(cx);
    }

    pub(super) fn input(&self, text: &str, cx: &mut TestAppContext) {
        self.shell.visual(cx).simulate_input(text);
        self.frame(cx);
    }

    pub(super) fn press(&self, key: &str, cx: &mut TestAppContext) {
        self.shell.press(key, cx);
        self.frame(cx);
    }

    pub(super) fn click(&self, id: impl Into<ElementId>, cx: &mut TestAppContext) {
        self.shell.click(id, cx);
        self.frame(cx);
    }

    pub(super) fn label(
        &self,
        id: impl Into<ElementId>,
        cx: &mut TestAppContext,
    ) -> Option<String> {
        self.shell.label(id, cx)
    }

    pub(super) fn present(&self, id: impl Into<ElementId>, cx: &mut TestAppContext) -> bool {
        self.shell.present(id, cx)
    }

    pub(super) fn checked(
        &self,
        id: impl Into<ElementId>,
        cx: &mut TestAppContext,
    ) -> Option<bool> {
        let id = id.into();
        self.shell.frame(cx, move |window, _| {
            window.try_find(id).and_then(|found| found.checked())
        })
    }

    pub(super) fn height(&self, id: impl Into<ElementId>, cx: &mut TestAppContext) -> f32 {
        let id = id.into();
        self.shell.frame(cx, move |window, _| {
            f32::from(window.try_find(id).expect("drawn").bounds().size.height)
        })
    }

    pub(super) fn is_focused(&self, handle: &FocusHandle, cx: &mut TestAppContext) -> bool {
        let handle = handle.clone();
        self.shell
            .frame(cx, move |window, _| handle.is_focused(window))
    }

    pub(super) fn editor_focused(&self, cx: &mut TestAppContext) -> bool {
        let editor = self.editor(cx);
        let handle = editor.read_with(cx, |editor, cx| editor.focus_handle(cx));
        self.is_focused(&handle, cx)
    }

    pub(super) fn control(&self, control: MiniControl, cx: &mut TestAppContext) -> FocusHandle {
        let composer = self.composer(cx);
        composer.read_with(cx, |composer, _| composer.control_handle(control))
    }

    pub(super) fn focused(&self, control: MiniControl, cx: &mut TestAppContext) -> bool {
        let handle = self.control(control, cx);
        self.is_focused(&handle, cx)
    }

    pub(super) fn focus_control(&self, control: MiniControl, cx: &mut TestAppContext) {
        let handle = self.control(control, cx);
        self.shell
            .frame(cx, move |window, cx| window.focus(&handle, cx));
        self.frame(cx);
    }

    pub(super) fn link_field_focused(&self, cx: &mut TestAppContext) -> bool {
        let composer = self.composer(cx);
        let Some(input) = composer.read_with(cx, |composer, _| composer.link_input()) else {
            return false;
        };
        let handle = input.read_with(cx, |input, cx| input.focus_handle(cx));
        self.is_focused(&handle, cx)
    }

    pub(super) fn set_recovery(&self, state: Option<RecoveryState>, cx: &mut TestAppContext) {
        let composer = self.composer(cx);
        composer.update(cx, |composer, cx| composer.set_recovery(state, cx));
        self.frame(cx);
    }

    pub(super) fn set_remote(&self, remote: bool, cx: &mut TestAppContext) {
        let composer = self.composer(cx);
        composer.update(cx, |composer, cx| composer.set_remote(remote, cx));
        self.frame(cx);
    }

    pub(super) fn changes(&self) -> usize {
        self.seen.borrow().len()
    }

    /// The text the last change carried.
    pub(super) fn changed_to(&self) -> Option<String> {
        self.seen.borrow().last().cloned()
    }
}
