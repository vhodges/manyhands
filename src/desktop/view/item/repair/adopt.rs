//! Adopt… (Task 25 fix 1, I1): an unmanaged document, a file under
//! `docs/` with no front matter, opened in the repair view with the
//! canonical document front matter its banner promises, then the file
//! exactly as read, byte for byte. The front matter is the library's
//! (`canonical::serialize_item`): its title is the file's first ATX
//! heading's text, else its file stem, and its ID is made as a new item's
//! is (`new_item_id`).
//!
//! The ID is made once per read of the file: it is kept with the
//! observation it was made for, and a later Adopt… of the same read reuses
//! it. So a file whose repair was made, its answer lost, is never adopted
//! a second time as another item. A new observation makes a new ID. The
//! view opens at the frame after the ID lands, which gives the window; an
//! ID for a read the tab no longer shows opens nothing, and a pane gone
//! takes no answer.

use std::path::Path;

use gpui_kit::{Context, Task, Window};
use manyhands::{
    canonical::{self, CanonicalItem, Document, ItemId},
    repository::{NewIdDto, ReadError},
    results::ResultCode,
};

use super::{Offer, unmanaged};
use crate::desktop::{
    model::stamp::{Generations, Stamp, ViewScope},
    services::{ReadKind, ReadResult},
    view::item::ItemPane,
};

/// Adopt…'s ID: the one made, with the observation of the read it was made
/// for; the read making one, while it runs; whether one has landed for the
/// next frame to open the view on; and the code of a read that failed,
/// shown beside Adopt… until it is pressed again (fix 2, N1).
#[derive(Default)]
pub(in crate::desktop::view::item) struct Adoption {
    made: Option<(String, ItemId)>,
    making: Option<Task<()>>,
    ids: Generations,
    landed: bool,
    pub(super) failure: Option<ResultCode>,
}

/// The title an adopted file takes: its first ATX heading's text, outside
/// fenced code, with its closing hashes dropped; else its file stem.
pub(super) fn adopt_title(path: &str, source: &str) -> String {
    let mut fence: Option<&str> = None;
    for line in source.lines() {
        let indent = line.len() - line.trim_start_matches(' ').len();
        let line = &line[indent..];
        if let Some(open) = fence {
            if line.starts_with(open) {
                fence = None;
            }
            continue;
        }
        if indent > 3 {
            continue;
        }
        if let Some(open) = ["```", "~~~"]
            .into_iter()
            .find(|open| line.starts_with(open))
        {
            fence = Some(open);
            continue;
        }
        let hashes = line.len() - line.trim_start_matches('#').len();
        let rest = &line[hashes..];
        if !(1..=6).contains(&hashes) || !(rest.is_empty() || rest.starts_with([' ', '\t'])) {
            continue;
        }
        let text = rest.trim();
        let open = text.trim_end_matches('#');
        let text = if open.is_empty() || open.ends_with([' ', '\t']) {
            open.trim_end()
        } else {
            text
        };
        if !text.is_empty() {
            return text.to_owned();
        }
    }
    file_stem(path)
}

/// The file stem of `path`, the title a file with no heading takes.
fn file_stem(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_owned())
}

impl ItemPane {
    /// Adopt…: the view opens on the adoption of the file as this tab read
    /// it, with the ID made for this read, or once one is made.
    pub(super) fn open_adopt(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        use gpui_kit::component::WindowExt;
        if window.has_active_dialog(cx) {
            return;
        }
        let Some(item) = self.model.item().rows() else {
            return;
        };
        if !matches!(self.repair_offer(item), Some(Offer::Repair)) || !unmanaged(item) {
            return;
        }
        let Some(observation) = item.observation.clone() else {
            return;
        };
        self.adoption.failure = None;
        cx.notify();
        match &self.adoption.made {
            Some((seen, id)) if *seen == observation => {
                let id = id.clone();
                self.open_adopted(&id, false, window, cx);
            }
            _ if self.adoption.making.is_some() => {}
            _ => self.make_id(observation, cx),
        }
    }

    /// Makes an ID for the read `observation`.
    #[allow(clippy::result_large_err)] // The library's read error, as every read gives it.
    fn make_id(&mut self, observation: String, cx: &mut Context<Self>) {
        let stamp = self.adoption.ids.next(ViewScope::Create);
        self.adoption.making = Some(self.services.read(
            ReadKind::NewId,
            stamp,
            |service| Ok::<_, ReadError>(service.new_item_id()),
            cx,
            move |pane: &mut Self, stamp, result, cx| pane.id_made(stamp, observation, result, cx),
        ));
    }

    /// An ID landed for the read `observation`: it is kept for that read,
    /// and the next frame opens the view on it. A read that failed opens
    /// nothing, and says why beside Adopt…, in the library's message and
    /// code, as a create's does (`view::create`); Adopt… asks again.
    fn id_made(
        &mut self,
        stamp: Stamp,
        observation: String,
        result: ReadResult<NewIdDto>,
        cx: &mut Context<Self>,
    ) {
        if !self.adoption.ids.is_current(&stamp) {
            return;
        }
        self.adoption.making = None;
        let id = match result {
            ReadResult::Ok(made) => made
                .id
                .parse::<ItemId>()
                .map_err(|_| ResultCode::InternalError),
            ReadResult::Busy => Err(ResultCode::Busy),
            ReadResult::Failed(failure) => Err(failure.code),
        };
        let id = match id {
            Ok(id) => id,
            Err(code) => {
                self.adoption.failure = Some(code);
                cx.notify();
                return;
            }
        };
        self.adoption.made = Some((observation, id));
        self.adoption.landed = true;
        cx.notify();
    }

    /// Answers the Adopt… read now running with `result`, as if it had
    /// landed: the library's `new_item_id` cannot fail, so a test gives a
    /// failed answer here. The read running is dropped when it lands.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn answer_adoption_for_testing(
        &mut self,
        result: ReadResult<NewIdDto>,
        cx: &mut Context<Self>,
    ) {
        let observation = self
            .model
            .item()
            .rows()
            .and_then(|item| item.observation.clone())
            .expect("the tab has read its file");
        let stamp = self.adoption.ids.next(ViewScope::Create);
        self.id_made(stamp, observation, result, cx);
    }

    /// The frame after an ID landed opens the view on it, while the tab
    /// still shows the read it was made for.
    pub(in crate::desktop::view::item) fn sync_adoption(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !std::mem::take(&mut self.adoption.landed) {
            return;
        }
        let Some((seen, id)) = self.adoption.made.clone() else {
            return;
        };
        let shown = self
            .model
            .item()
            .rows()
            .and_then(|item| item.observation.as_deref());
        if shown == Some(seen.as_str()) {
            self.open_adopted(&id, true, window, cx);
        }
    }

    /// The view opens on the canonical document front matter with `id` and
    /// the file's title, then the file exactly as read: its unedited text. A
    /// heading the library cannot write as a title gives way to the file
    /// stem (fix 2, N2); the file alone is the last resort, which keeps this
    /// total.
    fn open_adopted(
        &mut self,
        id: &ItemId,
        landed: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(item) = self.model.item().rows() else {
            return;
        };
        let Some(source) = item.source.clone() else {
            return;
        };
        let adopted = |title: String| {
            canonical::serialize_item(&CanonicalItem::Document(Document {
                id: id.clone(),
                title,
                body: source.clone(),
                unknown: serde_yaml::Mapping::new(),
            }))
        };
        let opened = adopted(adopt_title(&item.path, &source))
            .or_else(|_| adopted(file_stem(&item.path)))
            .unwrap_or_else(|_| source.clone());
        self.open_view(opened, landed, window, cx);
    }
}
