//! A main window over a fixture, for the views' tests: frames, clicks,
//! keys and the labels a test reads.

use std::{
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

use gpui_kit::{
    App, ElementId, Entity, Focusable, KeyDownEvent, KeyUpEvent, Keystroke, TestAppContext,
    VisualTestContext, Window, WindowHandle, component::Root, px, size, test::TestWindowExt,
};
use manyhands::{
    drafts::{DraftBase, DraftFields, DraftId, DraftTarget, ItemKind},
    repository::ItemDtoKind,
};

use super::{editor::host::EditorHost, workspace::Workspace};
use crate::desktop::{
    keys,
    model::{drafts::create, navigation::OpenTarget, operations::Operations, tabs::TabKey},
    open_test_window_over,
    services::requests::{DesktopRequest, DesktopRequestId, Intent},
    test_support::{delaying::DelayingBackend, fixtures::Fixture},
};

/// How long a test waits on the worker: long enough on a loaded machine,
/// inside the scheduler's fifteen seconds of parking.
pub(crate) const WAIT: Duration = Duration::from_secs(10);

/// How often a wait looks again while the worker runs.
const POLL: Duration = Duration::from_millis(10);

/// Waits, with the worker running, until `done` holds of `entity`, then
/// lets the reads that started finish. Panics after `WAIT` of real time.
/// Every test that waits on the worker waits here, never with
/// `VisualTestContext::condition`, whose timer gives up after three
/// seconds of parked time. The test allows parking.
pub(crate) async fn wait_until<T: 'static>(
    cx: &TestAppContext,
    entity: &Entity<T>,
    done: impl Fn(&T, &App) -> bool,
) {
    let deadline = Instant::now() + WAIT;
    loop {
        cx.run_until_parked();
        if entity.read_with(cx, |value, cx| done(value, cx)) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "the awaited state did not arrive in {WAIT:?}"
        );
        cx.executor().timer(POLL).await;
    }
}

pub(crate) struct Shell {
    pub window: WindowHandle<Root>,
    pub workspace: Entity<Workspace>,
    pub backend: Arc<DelayingBackend>,
    pub fixture: Fixture,
}

impl Shell {
    /// Opens the main window over `fixture`, whose repositories are
    /// already made, and lets its startup reads finish. Nothing is
    /// refreshed at launch, so a test that does not wait for the worker
    /// need not.
    pub(crate) fn open(fixture: Fixture, cx: &mut TestAppContext) -> Self {
        Self::start(fixture, false, |_| {}, cx)
    }

    /// Opens the main window as the application does, refreshing every
    /// repository at launch. `before` runs on the backend before the
    /// window opens, to hold what the test needs held. The test waits for
    /// the worker, so it allows parking.
    pub(crate) fn launch(
        fixture: Fixture,
        before: impl FnOnce(&DelayingBackend),
        cx: &mut TestAppContext,
    ) -> Self {
        Self::start(fixture, true, before, cx)
    }

    fn start(
        fixture: Fixture,
        launch_refresh: bool,
        before: impl FnOnce(&DelayingBackend),
        cx: &mut TestAppContext,
    ) -> Self {
        cx.update(gpui_kit::init);
        cx.update(keys::bind);
        cx.update(super::editor::keys::bind);
        // Dialogs open without their entrance animation, so a frame finds
        // them where they rest.
        cx.update(|cx| cx.set_reduce_motion(true));
        let backend = fixture.delaying_backend();
        before(&backend);
        let window = cx.update(|cx| open_test_window_over(backend.clone(), launch_refresh, cx));
        let workspace = window
            .update(cx, |root, _, _| {
                root.view().clone().downcast::<Workspace>().unwrap()
            })
            .unwrap();
        let shell = Self {
            window,
            workspace,
            backend,
            fixture,
        };
        // Tall enough that an item, its later actions and a few expanded
        // entries of the operation area are all on screen, where a click
        // reaches them.
        shell.visual(cx).simulate_resize(size(px(1440.), px(1400.)));
        shell.run(cx);
        shell
    }

    pub(crate) fn visual(&self, cx: &TestAppContext) -> VisualTestContext {
        VisualTestContext::from_window(self.window.into(), cx)
    }

    /// Lets every started read finish.
    pub(crate) fn run(&self, cx: &TestAppContext) {
        self.visual(cx).run_until_parked();
    }

    /// Draws a frame and runs `f` over the window.
    pub(crate) fn frame<R>(
        &self,
        cx: &mut TestAppContext,
        f: impl FnOnce(&mut Window, &mut App) -> R,
    ) -> R {
        let mut visual = self.visual(cx);
        visual.update(|window, cx| {
            window.render_frame(cx);
            f(window, cx)
        })
    }

    /// Clicks the element `id` and lets the reads it starts finish.
    pub(crate) fn click(&self, id: impl Into<ElementId>, cx: &mut TestAppContext) {
        let id = id.into();
        self.frame(cx, |window, cx| window.click(id, cx));
        self.run(cx);
    }

    pub(crate) fn keys(&self, keystrokes: &str, cx: &mut TestAppContext) {
        let mut visual = self.visual(cx);
        visual.simulate_keystrokes(keystrokes);
        visual.run_until_parked();
    }

    /// Presses and releases `key`, as a keyboard does. `keys` only presses:
    /// GPUI clicks a focused element on Enter or Space when the key is
    /// released.
    pub(crate) fn press(&self, key: &str, cx: &mut TestAppContext) {
        let keystroke = Keystroke::parse(key).unwrap();
        let mut visual = self.visual(cx);
        visual.simulate_event(KeyDownEvent {
            keystroke: keystroke.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        visual.simulate_event(KeyUpEvent { keystroke });
        visual.run_until_parked();
    }

    pub(crate) fn label(
        &self,
        id: impl Into<ElementId>,
        cx: &mut TestAppContext,
    ) -> Option<String> {
        let id = id.into();
        self.frame(cx, |window, _| {
            window
                .try_find(id)
                .and_then(|element| element.label().map(str::to_owned))
        })
    }

    /// Whether the element `id` is a button that does not say it takes
    /// input. The Kit's button reports no disabled flag (its snapshot's
    /// `disabled()` is `None` either way), so a test pairs this with a
    /// click that changes nothing.
    pub(crate) fn inert_button(&self, id: impl Into<ElementId>, cx: &mut TestAppContext) -> bool {
        let id = id.into();
        self.frame(cx, |window, _| {
            window.try_find(id).is_some_and(|button| {
                button.role() == Some(gpui_kit::Role::Button) && button.disabled() != Some(false)
            })
        })
    }

    /// Whether the element `id` is drawn.
    pub(crate) fn present(&self, id: impl Into<ElementId>, cx: &mut TestAppContext) -> bool {
        let id = id.into();
        self.frame(cx, |window, _| window.try_find(id).is_some())
    }

    pub(crate) fn selected(&self, id: impl Into<ElementId>, cx: &mut TestAppContext) -> bool {
        let id = id.into();
        self.frame(cx, |window, _| {
            window.try_find(id).and_then(|element| element.selected()) == Some(true)
        })
    }

    /// The labels of `name`'s rows, `(name, 0)` onwards, until one is
    /// absent.
    pub(crate) fn rows(&self, name: &'static str, cx: &mut TestAppContext) -> Vec<String> {
        (0..)
            .map_while(|index| self.label((name, index as usize), cx))
            .collect()
    }

    /// Selects the repository at `root` and lets its reads finish.
    pub(crate) fn select(&self, root: &Path, cx: &mut TestAppContext) {
        self.workspace.update(cx, |workspace, cx| {
            workspace.select_repository(key(root), cx)
        });
        self.run(cx);
    }

    /// The window's request model.
    pub(crate) fn operations(&self, cx: &mut TestAppContext) -> Entity<Operations> {
        self.workspace
            .read_with(cx, |workspace, _| workspace.services().operations().clone())
    }

    /// The requests of this session that `pick` picks, newest first.
    pub(crate) fn requests(
        &self,
        cx: &mut TestAppContext,
        pick: impl Fn(&DesktopRequest) -> bool,
    ) -> Vec<DesktopRequestId> {
        let operations = self.operations(cx);
        operations.read_with(cx, |operations, _| {
            operations
                .requests()
                .iter()
                .filter(|request| pick(request))
                .map(|request| request.id)
                .collect()
        })
    }

    /// Waits, with the worker running, until `done` holds of the request
    /// model, then lets the reads that started finish. The test allows
    /// parking.
    pub(crate) async fn until(
        &self,
        cx: &mut TestAppContext,
        done: impl Fn(&Operations) -> bool + 'static,
    ) {
        let operations = self.operations(cx);
        wait_until(cx, &operations, move |operations, _| done(operations)).await;
    }

    /// Draws frames, with the worker and the reads running, until `done`
    /// holds of the window, then draws one more. Panics after `WAIT` of
    /// real time, saying that `what` did not happen. The test allows
    /// parking.
    pub(crate) async fn until_drawn(
        &self,
        cx: &mut TestAppContext,
        what: &str,
        done: impl Fn(&Shell, &mut TestAppContext) -> bool,
    ) {
        let deadline = Instant::now() + WAIT;
        loop {
            self.frame(cx, |_, _| ());
            self.run(cx);
            if done(self, cx) {
                self.frame(cx, |_, _| ());
                return;
            }
            assert!(Instant::now() < deadline, "{what} did not happen");
            cx.executor().timer(POLL).await;
        }
    }

    /// Waits until the request `id` has ended: finished, removed or not
    /// sent.
    pub(crate) async fn until_ended(&self, id: DesktopRequestId, cx: &mut TestAppContext) {
        use crate::desktop::services::requests::RequestState;
        self.until(cx, move |operations| {
            operations.request(id).is_some_and(|request| {
                !matches!(
                    request.state,
                    RequestState::Queued | RequestState::Running { .. }
                )
            })
        })
        .await;
    }
}

/// A ticket draft's fields with `body`; no repository holds its item.
fn draft_fields(body: &str) -> DraftFields {
    DraftFields::Ticket {
        title: "Draft title".to_owned(),
        ticket_type: "task".to_owned(),
        status: "open".to_owned(),
        project: None,
        team: None,
        deps: Vec::new(),
        parent: None,
        body: body.to_owned(),
    }
}

impl Shell {
    /// Begins a session with "First body", stored at revision 1.
    pub(crate) async fn begin_draft(&self, cx: &mut TestAppContext) -> DraftId {
        let drafts = self
            .workspace
            .read_with(cx, |workspace, _| workspace.services().drafts().clone());
        let target = DraftTarget {
            repository: "/a/repository".to_owned(),
            item_kind: ItemKind::Ticket,
            item_id: "01ARZ3NDEKTSV4RRFFQ69G5FG0".parse().unwrap(),
            comment: None,
        };
        let begun = drafts.update(cx, |registry, cx| {
            registry.begin(target, None, draft_fields("First body"), cx)
        });
        self.run(cx);
        match begun.await {
            Ok(id) => id,
            Err(kind) => panic!("the draft must begin: {kind:?}"),
        }
    }

    /// A session whose newest text, "Newer body", is in memory only: its
    /// write is not yet due.
    pub(crate) async fn unsaved_draft(&self, cx: &mut TestAppContext) -> DraftId {
        let id = self.begin_draft(cx).await;
        let drafts = self
            .workspace
            .read_with(cx, |workspace, _| workspace.services().drafts().clone());
        drafts.update(cx, |registry, cx| {
            registry.edit(id, draft_fields("Newer body"), cx)
        });
        self.run(cx);
        id
    }
}

/// A new ticket's draft fields: `title`, the default type and status, and
/// an empty body, as New ticket… makes them.
pub(crate) fn new_ticket_fields(title: &str) -> DraftFields {
    create::ticket_fields(title, create::DEFAULT_TYPE)
}

/// The new item's draft of the ticket `id` of `root` with `fields`: its
/// target, and its base, with neither observation nor source (R-20), at the
/// ticket's path in the primary context, as New ticket… makes them. The
/// fixture's primary branch is `main`.
pub(crate) fn new_ticket_draft(
    root: &Path,
    id: &str,
    fields: &DraftFields,
) -> (DraftTarget, DraftBase) {
    let id = id.parse().unwrap();
    let path = create::ticket_path(&id);
    create::new_item_draft(
        &key(root),
        Some("main".to_owned()),
        ItemKind::Ticket,
        id,
        path,
        fields,
    )
}

impl Shell {
    /// Begins a new item's draft of the ticket `id` of `root`, which no
    /// repository holds yet, with `begin_new`: titled `title`, with an empty
    /// body. Its session is attached to the item's tab, which shows it
    /// restored once it opens.
    pub(crate) async fn begin_new_ticket(
        &self,
        root: &Path,
        id: &str,
        title: &str,
        cx: &mut TestAppContext,
    ) -> DraftId {
        let drafts = self
            .workspace
            .read_with(cx, |workspace, _| workspace.services().drafts().clone());
        let fields = new_ticket_fields(title);
        let (target, base) = new_ticket_draft(root, id, &fields);
        let begun = drafts.update(cx, |registry, cx| {
            registry.begin_new(target, base, fields, cx)
        });
        self.run(cx);
        let draft = match begun.await {
            Ok(draft) => draft,
            Err(kind) => panic!("the new item's draft must begin: {kind:?}"),
        };
        let tab = TabKey::Item {
            repo: key(root),
            kind: ItemDtoKind::Ticket,
            id: id.to_owned(),
        };
        drafts.update(cx, |registry, cx| registry.attach(tab, draft, cx));
        self.run(cx);
        draft
    }

    /// A real not-created tab: a new item's draft of the ticket `id` of
    /// `root`, begun with `begin_new`, in the item's tab, continued with
    /// Continue editing, which prepares with the Create intent; once the
    /// edit editor opens, its draft and its host. The test allows parking.
    pub(crate) async fn edit_new_ticket(
        &self,
        root: &Path,
        id: &str,
        title: &str,
        cx: &mut TestAppContext,
    ) -> (DraftId, Entity<EditorHost>) {
        let draft = self.begin_new_ticket(root, id, title, cx).await;
        let target = OpenTarget::Item {
            repo: key(root),
            kind: ItemDtoKind::Ticket,
            id: id.parse().unwrap(),
        };
        self.workspace
            .update(cx, |workspace, cx| workspace.open_item(target, cx));
        self.run(cx);
        self.click("draft-continue-button", cx);
        let deadline = Instant::now() + WAIT;
        loop {
            self.frame(cx, |_, _| ());
            self.run(cx);
            let pane = self
                .workspace
                .read_with(cx, |workspace, _| workspace.item_pane())
                .expect("an item tab is shown");
            if let Some(host) = pane.read_with(cx, |pane, _| pane.edit_host()) {
                self.frame(cx, |_, _| ());
                return (draft, host);
            }
            assert!(
                Instant::now() < deadline,
                "the new item's edit editor did not open"
            );
            cx.executor().timer(POLL).await;
        }
    }
}

impl Shell {
    /// The shown item tab's edit host, if its editor is open.
    fn edit_host(&self, cx: &mut TestAppContext) -> Option<Entity<EditorHost>> {
        let pane = self
            .workspace
            .read_with(cx, |workspace, _| workspace.item_pane())?;
        pane.read_with(cx, |pane, _| pane.edit_host())
    }

    /// The shown item tab's edit host, once its editor has opened: frames
    /// are drawn while the worker and the reads run. The test allows
    /// parking.
    pub(crate) async fn until_editing(&self, cx: &mut TestAppContext) -> Entity<EditorHost> {
        self.until_drawn(cx, "the edit editor opened", |shell, cx| {
            shell.edit_host(cx).is_some()
        })
        .await;
        self.edit_host(cx).expect("the edit editor is open")
    }

    /// Types `typed` at the end of `host`'s text, through the window: the
    /// caret set there, the editor focused and a frame drawn first.
    pub(crate) fn type_at_end(
        &self,
        host: &Entity<EditorHost>,
        typed: &str,
        cx: &mut TestAppContext,
    ) {
        let editor = host.read_with(cx, |host, _| host.editor().clone());
        let end = editor.read_with(cx, |editor, _| editor.text().len());
        editor.update(cx, |editor, cx| editor.set_cursor(end, cx));
        let handle = editor.read_with(cx, |editor, cx| editor.focus_handle(cx));
        self.frame(cx, |window, cx| window.focus(&handle, cx));
        self.frame(cx, |_, _| ());
        self.visual(cx).simulate_input(typed);
        self.run(cx);
    }
}

/// Whether `intent` saves an item.
pub(crate) fn is_save(intent: &Intent) -> bool {
    matches!(
        intent,
        Intent::TicketSave { .. } | Intent::DocumentSave { .. }
    )
}

/// A root as the repository list gives it.
pub(crate) fn key(root: &Path) -> String {
    root.to_str().unwrap().to_owned()
}

#[cfg(test)]
#[path = "harness_tests.rs"]
mod tests;
