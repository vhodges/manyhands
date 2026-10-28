//! Each item tab's local images (design, Images; decision 9), shared by
//! all its editors: the read view, every comment and the edit editor.
//!
//! - **Provider.** Each editor's block image provider reads the tab's
//!   `ImageCache`. A hit gives the decoded image; a miss records the `src`
//!   and gives none, so the line shows its source.
//! - **After the frame.** Each editor's host asks for a start as its frame
//!   is drawn (`after_frame`); the start runs once the frame is, as a
//!   deferred effect (as Task 7's per-frame hooks run), and starts what the
//!   frame recorded.
//! - **Source.** A `src` is parsed as a link target (`links::parse_link`).
//!   An external `src`, another scheme, a title, and a path the Parse rules
//!   refuse are never looked up. A path resolves from the item's folder.
//! - **Resolution.** Exact: in the item's own work context first, then on
//!   primary. The first tree that holds anything at the path decides: its
//!   file is read under containment (R-34), or refused. Nearest name, when
//!   no tree does: the nearest file of that name in a walk of `docs/`
//!   (R-33), the context's first, then the fewest folder steps, then path
//!   order. The walk is kept by the tab.
//! - **Loads** run one at a time per tab on the background executor, held
//!   at the backend's `ReadKind::Image` gate as reads are, and decode
//!   within the tab's budget (R-39). A load that lands notifies the tab's
//!   editors and starts the next. Misses and refusals stay cached, so the
//!   per-frame provider never repeats them.
//! - **Clear.** The tab's reload, or a new place to resolve from, clears it
//!   all. A load from before lands on nothing.
//! - **Atlas.** A painted image stays in each window's sprite atlas until
//!   it is dropped from there (`App::drop_image`); dropping the `Arc`
//!   frees nothing on the GPU. So the images the tab lets go of, at a
//!   clear, at a new place to resolve from, and when the tab is released
//!   (`close`), are dropped from every window once the update ends. A load
//!   that lands on nothing was never painted.
//!
//! A `src` is body text: nothing here derives `Debug`, and nothing logs or
//! reports a `src`, a path or a failure.

use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    fs, mem,
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
};

use gpui_kit::{App, Entity, RenderImage, WeakEntity};
use manyhands::repository::{ItemContextKind, ItemDto};
use zorite_editor::EditorState;

use super::{Backend, ReadKind, Services};
use crate::desktop::model::{
    links::{CandidatePath, LinkTarget, file_name, folder_of, link_path, nearest, parse_link},
    resources::{DecodeBudget, decode, decoded_size, read_contained, walk_docs},
};

/// One `src`'s image in a tab.
pub(crate) enum ImageEntry {
    /// Asked for, not yet loaded: the line shows its source.
    Pending,
    /// Decoded, with its width and height in pixels, which count against
    /// the tab's budget.
    Loaded(Arc<RenderImage>, f32, f32),
    /// Never loaded, or refused: the line shows its source.
    Failed,
}

/// What a tab holds of its images, by `src` as the editor gives it.
pub(crate) struct ImageCache {
    backend: Arc<dyn Backend>,
    /// Where the tab's srcs resolve from, once its item was read.
    source: Option<ImageSource>,
    entries: HashMap<String, ImageEntry>,
    /// The misses since the last start, in the order they were met.
    asked: Vec<String>,
    /// Local srcs waiting for their turn, with the path each names.
    queue: VecDeque<(String, String)>,
    /// The nearest-name walk of each tree, once a load needed it.
    walks: Option<Arc<Walks>>,
    /// A load is out; the next waits for it to land.
    running: bool,
    /// A start is due once this frame is drawn.
    scheduled: bool,
    /// Moves on at each clear: a load from before lands on nothing.
    generation: u64,
    /// The tab is released: nothing more is asked for or loaded.
    closed: bool,
    /// The tab's editors, notified when a load lands.
    editors: Vec<WeakEntity<EditorState>>,
}

/// The tab's `ImageCache`, which every editor of the tab reads.
#[derive(Clone)]
pub(crate) struct TabImages(Rc<RefCell<ImageCache>>);

/// A tree an image path is looked up in: an item's work context, or the
/// repository's root.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ImageTree {
    pub root: PathBuf,
    pub in_context: bool,
}

/// Where a tab's srcs resolve: its trees, the item's own context first,
/// and the item's folder there.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ImageSource {
    pub trees: Vec<ImageTree>,
    pub dir: String,
}

impl ImageSource {
    /// Where `item`, read in the repository whose root is `repo`, finds
    /// its images: in its own work context when it was read from one, then
    /// on primary, from its folder.
    pub(crate) fn of(item: &ItemDto, repo: &str) -> Self {
        let own = (item.context.kind == ItemContextKind::Active).then(|| ImageTree {
            root: PathBuf::from(&item.context.worktree),
            in_context: true,
        });
        let primary = ImageTree {
            root: PathBuf::from(repo),
            in_context: false,
        };
        Self {
            trees: own.into_iter().chain([primary]).collect(),
            dir: folder_of(&item.path).to_owned(),
        }
    }
}

/// What a test reads of one `src`'s entry.
#[cfg(test)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Seen {
    Pending,
    Loaded(f32, f32),
    Failed,
}

impl TabImages {
    pub(crate) fn new(services: &Services) -> Self {
        Self(Rc::new(RefCell::new(ImageCache {
            backend: services.backend.clone(),
            source: None,
            entries: HashMap::new(),
            asked: Vec::new(),
            queue: VecDeque::new(),
            walks: None,
            running: false,
            scheduled: false,
            generation: 0,
            closed: false,
            editors: Vec::new(),
        })))
    }

    /// Where the tab's srcs resolve from now, as its last read found the
    /// item. A different place clears what the tab holds, which resolved
    /// from the old one, its images dropped from the atlas; the editors
    /// ask again as they are drawn.
    pub(crate) fn locate(&self, source: ImageSource, cx: &mut App) {
        let dropped = {
            let mut cache = self.0.borrow_mut();
            if cache.source.as_ref() == Some(&source) {
                return;
            }
            match cache.source.replace(source) {
                Some(_) => cache.reset(),
                None => Vec::new(),
            }
        };
        release(dropped, cx);
    }

    /// Gives `editor` the tab's images: its provider reads them, and it is
    /// notified when a load lands.
    pub(crate) fn attach(&self, editor: &Entity<EditorState>, cx: &mut App) {
        let images = self.clone();
        editor.update(cx, |editor, _| {
            editor.set_block_image_provider(move |src| images.provide(src));
        });
        let mut cache = self.0.borrow_mut();
        cache.editors.retain(|editor| editor.upgrade().is_some());
        cache.editors.push(editor.downgrade());
    }

    /// As an editor's frame is drawn: the misses it records are started
    /// once the frame is, by one deferred start for all the tab's editors.
    pub(crate) fn after_frame(&self, cx: &mut App) {
        if mem::replace(&mut self.0.borrow_mut().scheduled, true) {
            return;
        }
        let images = self.clone();
        cx.defer(move |cx| images.start(cx));
    }

    /// Drops everything the tab holds, its images from the atlas too, and
    /// has its editors drawn again, so each image is read again as it is
    /// now.
    pub(crate) fn clear(&self, cx: &mut App) {
        let (dropped, editors) = {
            let mut cache = self.0.borrow_mut();
            (cache.reset(), cache.editors.clone())
        };
        release(dropped, cx);
        notify(&editors, cx);
    }

    /// The tab is released: what it holds is dropped, its images from the
    /// atlas too, and nothing more is asked for or loaded, should an
    /// editor of it outlive it. A load still out lands on nothing.
    pub(crate) fn close(&self, cx: &mut App) {
        let dropped = {
            let mut cache = self.0.borrow_mut();
            cache.closed = true;
            cache.editors.clear();
            cache.reset()
        };
        release(dropped, cx);
    }

    /// The provider: the image `src` names, once loaded. A miss is
    /// recorded for the next start.
    fn provide(&self, src: &str) -> Option<Arc<RenderImage>> {
        let mut cache = self.0.try_borrow_mut().ok()?;
        if cache.closed {
            return None;
        }
        match cache.entries.get(src) {
            Some(ImageEntry::Loaded(image, ..)) => Some(image.clone()),
            Some(ImageEntry::Pending | ImageEntry::Failed) => None,
            None => {
                cache.entries.insert(src.to_owned(), ImageEntry::Pending);
                cache.asked.push(src.to_owned());
                None
            }
        }
    }

    /// Queues what was asked for, failing at once what is never looked up,
    /// and starts the next load unless one is out.
    fn start(&self, cx: &mut App) {
        let (src, load, generation, gate) = {
            let mut cache = self.0.borrow_mut();
            let cache = &mut *cache;
            cache.scheduled = false;
            if cache.closed {
                return;
            }
            let Some(source) = &cache.source else {
                return;
            };
            for src in mem::take(&mut cache.asked) {
                match local_path(&src, &source.dir) {
                    Some(path) => cache.queue.push_back((src, path)),
                    None => {
                        cache.entries.insert(src, ImageEntry::Failed);
                    }
                }
            }
            if cache.running {
                return;
            }
            let Some((src, path)) = cache.queue.pop_front() else {
                return;
            };
            cache.running = true;
            let load = Load {
                path,
                source: source.clone(),
                walks: cache.walks.clone(),
                budget: cache.budget(),
            };
            // Taken now, in the order loads start; awaited in the load.
            let gate = cache.backend.read_gate(ReadKind::Image);
            (src, load, cache.generation, gate)
        };
        let running = cx.background_executor().spawn(async move {
            if let Some(gate) = gate {
                // A dropped sender releases the load as a send does.
                let _ = gate.await;
            }
            load.run()
        });
        let images = Rc::downgrade(&self.0);
        cx.spawn(async move |cx| {
            let landed = running.await;
            if let Some(images) = images.upgrade() {
                cx.update(|cx| TabImages(images).land(generation, src, landed, cx));
            }
        })
        .detach();
    }

    /// A load landed: kept unless the tab was cleared since, the editors
    /// notified, and the next one started.
    fn land(&self, generation: u64, src: String, landed: Landed, cx: &mut App) {
        let editors = {
            let mut cache = self.0.borrow_mut();
            cache.running = false;
            if generation != cache.generation {
                Vec::new()
            } else {
                cache.entries.insert(src, landed.entry);
                if landed.walks.is_some() {
                    cache.walks = landed.walks;
                }
                cache.editors.retain(|editor| editor.upgrade().is_some());
                cache.editors.clone()
            }
        };
        notify(&editors, cx);
        self.start(cx);
    }

    /// Holds a stand-in for a decoded `width` by `height` image at `src`,
    /// so a test can spend most of the tab's budget without decoding it.
    #[cfg(test)]
    pub(crate) fn hold_stand_in(&self, src: &str, width: f32, height: f32) {
        let image = image::Frame::new(image::RgbaImage::new(1, 1));
        let image = Arc::new(RenderImage::new(vec![image]));
        self.0
            .borrow_mut()
            .entries
            .insert(src.to_owned(), ImageEntry::Loaded(image, width, height));
    }

    /// The decoded image `src` names, once loaded.
    #[cfg(test)]
    pub(crate) fn image(&self, src: &str) -> Option<Arc<RenderImage>> {
        match self.0.borrow().entries.get(src)? {
            ImageEntry::Loaded(image, ..) => Some(image.clone()),
            ImageEntry::Pending | ImageEntry::Failed => None,
        }
    }

    #[cfg(test)]
    pub(crate) fn seen(&self, src: &str) -> Option<Seen> {
        self.0.borrow().entries.get(src).map(|entry| match entry {
            ImageEntry::Pending => Seen::Pending,
            ImageEntry::Loaded(_, width, height) => Seen::Loaded(*width, *height),
            ImageEntry::Failed => Seen::Failed,
        })
    }
}

impl ImageCache {
    /// What the tab may still decode: `TAB_BUDGET` less the images it
    /// holds. Loads run one at a time, so the next one sees what the last
    /// one spent.
    fn budget(&self) -> DecodeBudget {
        let held = self
            .entries
            .values()
            .map(|entry| match entry {
                ImageEntry::Loaded(_, width, height) => decoded_size(*width as u32, *height as u32),
                ImageEntry::Pending | ImageEntry::Failed => 0,
            })
            .sum();
        DecodeBudget::holding(held)
    }

    /// Everything the tab held is gone, so its budget is whole again; the
    /// decoded images it held are given back, to be dropped from the
    /// atlas. A load still out keeps the next waiting until it lands.
    fn reset(&mut self) -> Vec<Arc<RenderImage>> {
        let held = self
            .entries
            .drain()
            .filter_map(|(_, entry)| match entry {
                ImageEntry::Loaded(image, ..) => Some(image),
                ImageEntry::Pending | ImageEntry::Failed => None,
            })
            .collect();
        self.asked.clear();
        self.queue.clear();
        self.walks = None;
        self.generation += 1;
        held
    }
}

fn notify(editors: &[WeakEntity<EditorState>], cx: &mut App) {
    for editor in editors {
        // A closed editor needs nothing drawn.
        let _ = editor.update(cx, |_, cx| cx.notify());
    }
}

/// Drops `images` from every window's sprite atlas once the update ends.
/// A window being drawn is out of the app's list of windows while it is,
/// so a drop made then would miss it; deferred, `App::drop_image` reaches
/// them all.
fn release(images: Vec<Arc<RenderImage>>, cx: &mut App) {
    if images.is_empty() {
        return;
    }
    cx.defer(move |cx| {
        for image in images {
            cx.drop_image(image, None);
        }
    });
}

/// The repository path a local `src` names, from the item's folder `dir`.
/// None for an external `src`, another scheme, a title, or a path the
/// Parse rules refuse: none of them is looked up.
fn local_path(src: &str, dir: &str) -> Option<String> {
    match parse_link(src) {
        LinkTarget::Path { path, from_root } => link_path(&path, from_root, dir),
        _ => None,
    }
}

/// The files each tree's walk of `docs/` found, in the order of the
/// source's trees.
struct Walks(Vec<Vec<String>>);

/// One load, as it runs on the background executor, with what the tab
/// may still decode. It holds a path the body names, so it derives
/// nothing.
struct Load {
    path: String,
    source: ImageSource,
    walks: Option<Arc<Walks>>,
    budget: DecodeBudget,
}

/// What a load gives back: the entry, and the walk when it made one.
struct Landed {
    entry: ImageEntry,
    walks: Option<Arc<Walks>>,
}

impl Load {
    fn run(self) -> Landed {
        let Self {
            path,
            source,
            walks,
            mut budget,
        } = self;
        let (bytes, walks) = match exact(&source, &path) {
            Some(bytes) => (bytes, None),
            None => {
                let walks = walks.unwrap_or_else(|| {
                    Arc::new(Walks(
                        source
                            .trees
                            .iter()
                            .map(|tree| walk_docs(&tree.root))
                            .collect(),
                    ))
                });
                (nearest_name(&source, &walks, &path), Some(walks))
            }
        };
        let entry = match bytes.map(|bytes| decode(&bytes, &mut budget)) {
            Some(Ok(frame)) => {
                let (width, height) = frame.buffer().dimensions();
                let image = Arc::new(RenderImage::new(vec![frame]));
                ImageEntry::Loaded(image, width as f32, height as f32)
            }
            Some(Err(_)) | None => ImageEntry::Failed,
        };
        Landed { entry, walks }
    }
}

/// Exact: what the first tree holding anything at `path` gives, its
/// file's bytes or `None` when containment refuses it; `None` at the top
/// when no tree holds anything there.
fn exact(source: &ImageSource, path: &str) -> Option<Option<Vec<u8>>> {
    source.trees.iter().find_map(|tree| {
        let candidate = below(&tree.root, path);
        fs::symlink_metadata(&candidate)
            .is_ok()
            .then(|| read_contained(&tree.root, &candidate))
    })
}

/// Nearest name: the bytes of the walked file named as `path` is that is
/// nearest the item's folder, the item's own context's first.
fn nearest_name(source: &ImageSource, walks: &Walks, path: &str) -> Option<Vec<u8>> {
    let name = file_name(path);
    let mut candidates = Vec::new();
    let mut roots = Vec::new();
    for (tree, files) in source.trees.iter().zip(&walks.0) {
        for file in files.iter().filter(|file| file_name(file) == name) {
            candidates.push(CandidatePath {
                path: file.clone(),
                in_context: tree.in_context,
            });
            roots.push(&tree.root);
        }
    }
    let found = nearest(&source.dir, &candidates)?;
    let root = roots[candidates.iter().position(|candidate| candidate == found)?];
    read_contained(root, &below(root, &found.path))
}

/// `path`, written with forward slashes, below `root`, with the
/// platform's separators.
fn below(root: &Path, path: &str) -> PathBuf {
    path.split('/')
        .fold(root.to_owned(), |at, step| at.join(step))
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "images_tests.rs"]
mod tests;
