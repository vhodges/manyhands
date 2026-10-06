//! Native scratch controls and bounded status bookkeeping, not a draft store.
use super::{
    adapter,
    catalog::Document,
    evidence::{CaptureContext, CaptureRun, hash},
    session::Draft,
};
use gpui_kit::component::button::*;
use gpui_kit::*;
use std::{collections::BTreeMap, io, process::Command, time::SystemTime};
use zorite_editor::EditorState;

actions!(
    editor_spike,
    [ToggleMode, NextDocument, FocusEditor, Capture]
);

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("ctrl-alt-m", ToggleMode, Some("EditorSpike")),
        KeyBinding::new("ctrl-alt-n", NextDocument, Some("EditorSpike")),
        KeyBinding::new("ctrl-alt-f", FocusEditor, Some("EditorSpike")),
        KeyBinding::new("ctrl-alt-s", Capture, Some("EditorSpike")),
    ]);
}

pub struct RunningProvenance {
    head: String,
    lock_sha256: String,
}

pub fn running_provenance() -> io::Result<RunningProvenance> {
    fn output(command: &mut Command) -> io::Result<String> {
        let output = command.output()?;
        if !output.status.success() {
            return Err(io::Error::other("provenance command failed"));
        }
        String::from_utf8(output.stdout).map_err(io::Error::other)
    }
    let head = output(Command::new("git").args(["rev-parse", "HEAD"]))?;
    let lock = output(Command::new("sha256sum").arg("Cargo.lock"))?;
    let head = head.trim().to_owned();
    let lock_sha256 = lock.split_whitespace().next().unwrap_or("").to_owned();
    if head.len() != 40 || lock_sha256.len() != 64 {
        return Err(io::Error::other("invalid provenance output"));
    }
    Ok(RunningProvenance { head, lock_sha256 })
}

struct Scratch {
    id: &'static str,
    label: &'static str,
    draft: Draft,
    editor: Entity<EditorState>,
    rich: bool,
    scroll: ScrollHandle,
    counts: BTreeMap<&'static str, u64>,
}

#[derive(Clone, Copy)]
enum Control {
    Mode,
    Focus,
    Undo,
    Redo,
    Bold,
    Italic,
    Code,
    Row,
    Column,
    DeleteRow,
    DeleteColumn,
    Capture,
    Next,
}

pub struct Host {
    docs: Vec<Scratch>,
    active: usize,
    // Drop these with Host; callbacks hold weak Host references via Context.
    _subscriptions: Vec<Subscription>,
    provenance: RunningProvenance,
    message: String,
}

impl Host {
    pub fn new(
        documents: Vec<Document>,
        provenance: RunningProvenance,
        capture_initial: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut docs = Vec::new();
        let mut subscriptions = Vec::new();
        for document in documents {
            let mut draft = Draft::new(document.original);
            let generation = draft.generation();
            let id = document.id;
            let editor = adapter::create(draft.original().body(), window, cx);
            let initial = adapter::readback(&editor, generation, cx);
            draft
                .observe(generation, initial.body)
                .expect("initial generation");
            eprintln!(
                "scratch id={id} original_bytes={} original_blake3={} header_bytes={} status={:?}",
                draft.original().full().len(),
                hash(draft.original().full()),
                draft.original().header().len(),
                draft.status()
            );
            subscriptions.push(cx.observe(&editor, move |host, entity, cx| {
                host.sample(id, generation, &entity, cx);
            }));
            subscriptions.push(cx.subscribe(&editor, move |host, entity, event, cx| {
                let Some(doc) = host.docs.iter_mut().find(|d| d.id == id) else {
                    return;
                };
                if !adapter::accepts(&doc.draft, doc.id, id, generation)
                    || doc.editor.entity_id() != entity.entity_id()
                {
                    return;
                }
                let category = adapter::event_category(event);
                let count = doc.counts.entry(category).or_default();
                *count = count.saturating_add(1);
                if category.starts_with("denied-") || category.starts_with("unsupported-") {
                    host.message = format!("{id}: {category} (no opener/provider)");
                    eprintln!("scratch id={id} event={category} count={count}");
                }
                host.sample(id, generation, &entity, cx);
            }));
            docs.push(Scratch {
                id,
                label: document.label,
                draft,
                editor,
                rich: true,
                scroll: ScrollHandle::new(),
                counts: BTreeMap::from([("initial-actual-readback", 1)]),
            });
        }
        let mut host = Self {
            docs,
            active: 0,
            _subscriptions: subscriptions,
            provenance,
            message: "Scratch only • originals never saved • no resource providers".into(),
        };
        if capture_initial {
            host.capture(cx);
        }
        host
    }

    fn sample(
        &mut self,
        id: &str,
        generation: u64,
        entity: &Entity<EditorState>,
        cx: &mut Context<Self>,
    ) {
        let Some(doc) = self.docs.iter_mut().find(|d| d.id == id) else {
            return;
        };
        if !adapter::accepts(&doc.draft, doc.id, id, generation)
            || entity.entity_id() != doc.editor.entity_id()
        {
            return;
        }
        let readback = adapter::readback(entity, generation, cx);
        let changed = doc.draft.current() != Some(readback.body.as_str());
        if doc.draft.observe(generation, readback.body).is_err() {
            return;
        }
        if changed {
            let count = doc.counts.entry("observed-byte-change").or_default();
            *count = count.saturating_add(1);
            eprintln!(
                "scratch id={id} observed_changes={count} status={:?}",
                doc.draft.status()
            );
        }
        cx.notify();
    }

    fn capture(&mut self, cx: &mut Context<Self>) {
        // A new run for every explicit capture; no default automatic snapshots.
        let result = (|| -> io::Result<_> {
            let nonce = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map_err(io::Error::other)?
                .as_nanos();
            let run = CaptureRun::create(&format!("native-{}-{nonce}", std::process::id()))?;
            for doc in &mut self.docs {
                let readback = adapter::readback(&doc.editor, doc.draft.generation(), cx);
                doc.draft
                    .observe(readback.generation, readback.body.clone())
                    .map_err(|e| io::Error::other(format!("{e:?}")))?;
                let mut actions = vec![
                    format!("catalog={}", doc.label),
                    format!("presentation={}", if doc.rich { "rich" } else { "source" }),
                    "actual EditorState::text readback; protected header is host-recombined, not editor round-trip".into(),
                    "runtime checkout HEAD/lock; not a build attestation; native input/render/performance not automatically verified".into(),
                    "providers absent; URL/OS opener absent; inherited OS/font/display/clipboard not sandboxed".into(),
                ];
                actions.extend(
                    doc.counts
                        .iter()
                        .map(|(category, count)| format!("{category}={count}")),
                );
                run.capture(
                    doc.id,
                    &doc.draft,
                    Some(&readback),
                    &CaptureContext {
                        tested_head: self.provenance.head.clone(),
                        lock_sha256: self.provenance.lock_sha256.clone(),
                        action_status: actions,
                    },
                )?;
            }
            Ok(run.path().display().to_string())
        })();
        self.message = match result {
            Ok(path) => {
                eprintln!("scratch capture docs={} path={path}", self.docs.len());
                format!("Captured {} actual readbacks: {path}", self.docs.len())
            }
            Err(error) => {
                eprintln!(
                    "scratch capture failed kind={:?}; partial evidence retained",
                    error.kind()
                );
                format!(
                    "Capture failed ({:?}); partial evidence retained",
                    error.kind()
                )
            }
        };
        cx.notify();
    }

    fn control(&mut self, control: Control, window: &mut Window, cx: &mut Context<Self>) {
        if matches!(control, Control::Capture) {
            self.capture(cx);
            return;
        }
        if matches!(control, Control::Next) {
            self.select((self.active + 1) % self.docs.len(), window, cx);
            return;
        }
        let doc = &mut self.docs[self.active];
        adapter::focus(&doc.editor, window, cx);
        let category = match control {
            Control::Mode => {
                doc.rich = !doc.rich;
                adapter::presentation(&doc.editor, doc.rich, cx);
                "mode-toggle-same-entity"
            }
            Control::Focus => "focus-request",
            Control::Undo | Control::Redo | Control::Bold | Control::Italic | Control::Code => {
                let action: Box<dyn Action> = match control {
                    Control::Undo => Box::new(zorite_editor::Undo),
                    Control::Redo => Box::new(zorite_editor::Redo),
                    Control::Bold => Box::new(zorite_editor::Bold),
                    Control::Italic => Box::new(zorite_editor::Italic),
                    _ => Box::new(zorite_editor::Code),
                };
                window.dispatch_action(action, cx);
                match control {
                    Control::Undo => "undo-dispatch-request",
                    Control::Redo => "redo-dispatch-request",
                    Control::Bold => "bold-dispatch-request",
                    Control::Italic => "italic-dispatch-request",
                    _ => "code-dispatch-request",
                }
            }
            Control::Row | Control::Column | Control::DeleteRow | Control::DeleteColumn => {
                doc.editor.update(cx, |editor, cx| match control {
                    Control::Row => editor.insert_table_row(true, cx),
                    Control::Column => editor.insert_table_column(true, cx),
                    Control::DeleteRow => editor.delete_table_row(cx),
                    _ => editor.delete_table_column(cx),
                });
                "table-operation-invoked-may-noop-outside-table"
            }
            Control::Capture | Control::Next => unreachable!(),
        };
        let count = doc.counts.entry(category).or_default();
        *count = count.saturating_add(1);
        self.message = format!(
            "{}: {category}; capture/notifications read actual text",
            doc.id
        );
        // dispatch_action is deferred. Notifications catch undo/format even
        // though Changed is not emitted. Deferred sample also handles no-ops.
        let id = doc.id;
        let generation = doc.draft.generation();
        let editor = doc.editor.clone();
        cx.defer_in(window, move |host, _, cx| {
            host.sample(id, generation, &editor, cx)
        });
        cx.notify();
    }

    fn select(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index >= self.docs.len() {
            return;
        }
        self.active = index;
        let doc = &self.docs[index];
        let id = doc.id;
        let generation = doc.draft.generation();
        let editor = doc.editor.clone();
        adapter::focus(&editor, window, cx);
        self.sample(id, generation, &editor, cx);
        // No reload, replacement, reset or discarded history on selection.
    }

    fn button(
        &self,
        id: &'static str,
        label: &'static str,
        control: Control,
        cx: &Context<Self>,
    ) -> Button {
        Button::new(id)
            .label(label)
            .tab_index(0)
            .on_click(cx.listener(move |host, _, window, cx| {
                host.control(control, window, cx);
            }))
    }
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let doc = &self.docs[self.active];
        let status = doc.draft.status();
        let status = format!(
            "{} • load-normalized={} • later-edits={} • conservative-dirty={} • body={} B • protected-header={} B",
            if doc.rich { "Rich" } else { "Source" },
            status.is_some_and(|s| s.changed_on_load),
            status.is_some_and(|s| s.user_edits),
            status.is_none_or(|s| s.dirty),
            doc.draft.current().map_or(0, str::len),
            doc.draft.original().header().len(),
        );
        div().key_context("EditorSpike").size_full().flex().flex_col()
            .bg(rgb(0x161c26)).text_color(rgb(0xe6edf3)).p_3().gap_2()
            .on_action(cx.listener(|host, _: &ToggleMode, window, cx| host.control(Control::Mode, window, cx)))
            .on_action(cx.listener(|host, _: &NextDocument, window, cx| host.control(Control::Next, window, cx)))
            .on_action(cx.listener(|host, _: &FocusEditor, window, cx| host.control(Control::Focus, window, cx)))
            .on_action(cx.listener(|host, _: &Capture, window, cx| host.control(Control::Capture, window, cx)))
            .child("Zorite 0.10.0 • repo-document scratch demo • byte changes tolerated ONLY for spike")
            .child(div().flex().gap_2().flex_wrap()
                .child(self.button("mode", "Rich / Source", Control::Mode, cx))
                .child(self.button("focus", "Focus editor", Control::Focus, cx))
                .child(self.button("undo", "Undo", Control::Undo, cx))
                .child(self.button("redo", "Redo", Control::Redo, cx))
                .child(self.button("bold", "Bold", Control::Bold, cx))
                .child(self.button("italic", "Italic", Control::Italic, cx))
                .child(self.button("code", "Code", Control::Code, cx))
                .child(self.button("row", "Row +", Control::Row, cx))
                .child(self.button("column", "Col +", Control::Column, cx))
                .child(self.button("delete-row", "Row −", Control::DeleteRow, cx))
                .child(self.button("delete-column", "Col −", Control::DeleteColumn, cx))
                .child(self.button("capture", "Capture all drafts", Control::Capture, cx)))
            .child("Ctrl-Alt: N next doc • M mode • F focus • S capture. Editor: Ctrl-Z / Ctrl-Shift-Z, Ctrl-B/I/E. Table controls require caret in table.")
            .child(status)
            .child(div().flex().flex_1().min_h_0().gap_3()
                .child(div().id("catalog").w(px(245.)).flex_shrink_0().overflow_y_scroll().flex().flex_col().gap_2()
                    .children(self.docs.iter().enumerate().map(|(index, doc)| {
                        Button::new(("doc", index)).label(format!("{}{}", if index == self.active { "▶ " } else { "" }, doc.label))
                            .tab_index(0).on_click(cx.listener(move |host, _, window, cx| host.select(index, window, cx)))
                    })))
                .child(div().id(("body-scroll", self.active)).flex_1().min_w_0().overflow_y_scroll()
                    .track_scroll(&doc.scroll).p_4().text_size(px(16.)).child(doc.editor.clone())))
            .child(self.message.clone())
            .child("Images/chips/embeds/mermaid/math/property editors/highlighting unavailable (providers absent). No URL opener. Clipboard/display/fonts are inherited OS access, NOT sandboxed.")
    }
}
