//! One image load, as it runs on the background executor: the exact
//! lookup in each tree, else the nearest name over the walks of `docs/`,
//! read under containment (R-34) and decoded within what the tab has left
//! (R-39). It holds a path the body names, so nothing here derives
//! `Debug`.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use gpui_kit::RenderImage;

use super::{ImageEntry, ImageSource};
use crate::desktop::model::{
    links::{CandidatePath, file_name, nearest},
    resources::{DecodeBudget, Found, decode, locate, read_canonical, read_contained, walk_docs},
};

/// The files each tree's walk of `docs/` found, in the order of the
/// source's trees.
pub(super) struct Walks(Vec<Vec<String>>);

/// One load, as it runs on the background executor, with what the tab
/// may still decode. It holds a path the body names, so it derives
/// nothing.
pub(super) struct Load {
    pub(super) path: String,
    pub(super) source: ImageSource,
    pub(super) walks: Option<Arc<Walks>>,
    pub(super) budget: DecodeBudget,
}

/// What a load gives back: the entry, and the walk when it made one.
pub(super) struct Landed {
    pub(super) entry: ImageEntry,
    pub(super) walks: Option<Arc<Walks>>,
}

impl Load {
    pub(super) fn run(self) -> Landed {
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
/// when no tree holds anything there. Whether a tree holds anything is
/// found by containment's own walk, which follows no link (`locate`).
fn exact(source: &ImageSource, path: &str) -> Option<Option<Vec<u8>>> {
    source
        .trees
        .iter()
        .find_map(|tree| match locate(&tree.root, &below(&tree.root, path)) {
            Found::Nothing => None,
            Found::Refused => Some(None),
            Found::Contained(canonical) => Some(read_canonical(&canonical)),
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
