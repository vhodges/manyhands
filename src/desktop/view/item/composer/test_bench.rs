//! The composer tests' bench: a composer in the test shell between two
//! tab stops, as the discussion holds it, and what a test does to it and
//! reads of it.

use std::{cell::RefCell, rc::Rc};

use gpui_kit::{
    AppContext, Context, ElementId, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, Modifiers, MouseButton, ParentElement, Pixels, Point, Render, Styled,
    TestAppContext, Window, component::Root, div, px, test::TestWindowExt,
};
use manyhands::drafts::ItemKind;
use zorite_editor::EditorState;

use super::{
    Composer, ComposerEvent, ComposerKind, ComposerState, FailedAction, toolbar::MiniControl,
};
use crate::desktop::{
    keys::{NextRegion, PreviousRegion},
    model::drafts::RecoveryState,
    test_support::fixtures::Fixture,
    view::harness::Shell,
};

/// The name a reply answers.
pub(super) const REPLY_TO: &str = "Jo Smith";
/// The user, whose avatar a reply shows.
const ME: &str = "Vince Hodges";

/// The composer between two tab stops, as the discussion holds it. F6
/// and Shift-F6 move to the stop after it and the one before, as the
/// workspace moves between regions.
pub(super) struct Stage {
    pub(super) before: FocusHandle,
    composer: Entity<Composer>,
    pub(super) after: FocusHandle,
}

impl Render for Stage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .on_action(
                cx.listener(|stage, _: &NextRegion, window, cx| window.focus(&stage.after, cx)),
            )
            .on_action(
                cx.listener(|stage, _: &PreviousRegion, window, cx| {
                    window.focus(&stage.before, cx)
                }),
            )
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
            .children(Root::render_dialog_layer(window, cx))
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

/// An event the composer emitted, but for a change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Seen {
    Submit,
    Cancel,
    Discard,
    Failed(FailedAction),
}

/// A composer in the test shell, the texts of the changes it emitted and
/// its other events.
pub(super) struct Bench {
    pub(super) shell: Shell,
    pub(super) stage: Entity<Stage>,
    seen: Rc<RefCell<Vec<String>>>,
    events: Rc<RefCell<Vec<Seen>>>,
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
            events: Rc::default(),
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
        self.events.borrow_mut().clear();
        let composer = self.composer(cx);
        let (seen, events) = (self.seen.clone(), self.events.clone());
        cx.update(|cx| {
            cx.subscribe(&composer, move |_, event: &ComposerEvent, _| {
                let other = match event {
                    ComposerEvent::Changed(text) => {
                        seen.borrow_mut().push(text.clone());
                        return;
                    }
                    ComposerEvent::Submit => Seen::Submit,
                    ComposerEvent::Cancel => Seen::Cancel,
                    ComposerEvent::Discard => Seen::Discard,
                    ComposerEvent::Failed(action) => Seen::Failed(*action),
                };
                events.borrow_mut().push(other);
            })
            .detach()
        });
    }

    /// The events other than changes, since the last `watch` or `forget`.
    pub(super) fn events(&self) -> Vec<Seen> {
        self.events.borrow().clone()
    }

    pub(super) fn forget(&self) {
        self.events.borrow_mut().clear();
    }

    pub(super) fn set_state(&self, state: ComposerState, cx: &mut TestAppContext) {
        let composer = self.composer(cx);
        let mut state = Some(state);
        self.shell.frame(cx, move |window, cx| {
            composer.update(cx, |composer, cx| {
                composer.set_state(state.take().unwrap(), window, cx)
            })
        });
        self.frame(cx);
    }

    /// Dispatches `action` from the editor, as its key would.
    pub(super) fn dispatch(&self, action: &dyn gpui_kit::Action, cx: &mut TestAppContext) {
        let editor = self.editor(cx);
        let action = action.boxed_clone();
        self.shell.frame(cx, move |window, cx| {
            let handle = editor.read(cx).focus_handle(cx);
            handle.dispatch_action(action.as_ref(), window, cx);
        });
        self.frame(cx);
    }

    pub(super) fn read_only(&self, cx: &mut TestAppContext) -> bool {
        let composer = self.composer(cx);
        composer.read_with(cx, |composer, _| composer.is_read_only())
    }

    pub(super) fn cursor(&self, cx: &mut TestAppContext) -> usize {
        let editor = self.editor(cx);
        editor.read_with(cx, |editor, _| editor.cursor())
    }

    /// The centre of the box the editor lays `offset` out in.
    pub(super) fn point(&self, offset: usize, cx: &mut TestAppContext) -> Point<Pixels> {
        let editor = self.editor(cx);
        self.shell
            .frame(cx, move |_, cx| editor.read(cx).bounds_for_offset(offset))
            .unwrap_or_else(|| panic!("offset {offset} is not laid out"))
            .center()
    }

    /// Presses and releases the left button at `position`, then draws.
    pub(super) fn click_at(&self, position: Point<Pixels>, cx: &mut TestAppContext) {
        self.shell.frame(cx, |_, _| ());
        let mut visual = self.shell.visual(cx);
        visual.simulate_mouse_move(position, None, Modifiers::none());
        visual.simulate_mouse_down(position, MouseButton::Left, Modifiers::none());
        visual.simulate_mouse_up(position, MouseButton::Left, Modifiers::none());
        self.frame(cx);
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

    /// Whether the element `id` has focus.
    pub(super) fn focused_id(&self, id: impl Into<ElementId>, cx: &mut TestAppContext) -> bool {
        let id = id.into();
        self.shell.frame(cx, move |window, _| {
            window.try_find(id).and_then(|found| found.focused()) == Some(true)
        })
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
