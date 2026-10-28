//! Link resolution (design, Links; decision 10), off the UI thread through
//! the backend's reads, each retried on `busy` as every read is.
//!
//! A path link is resolved from the linking item's folder; a comment's,
//! from its item's. The linking item's own work context, when it was read
//! from one, is tried first, then primary:
//! - **Exact.** `observe_path` says whether a file is at the path, and
//!   refuses a path no item can be at as `invalid_path` (fact 4) without
//!   opening anything. That answer, absence, and an observe that stays
//!   busy or fails all go on: to primary, then to the nearest name. A file
//!   that is there is read with `show_path` in the same context. A managed
//!   item opens its tab. Any other file, one with no ID, opens its Inspect
//!   tab in that context, as the lists open it (controller, Task 11 fix
//!   round 1). `show_path`'s own `invalid_path`, for a symbolic link or
//!   something that is not a regular file, is not found.
//! - **Nearest name.** Among the managed documents `list_documents` gives
//!   with the same file name: the context's copies first, then the fewest
//!   folder steps, then path order. The tab says which path opened. An
//!   item's context holds the item alone, so its one copy there is the
//!   linking item's own.
//!
//! A title link matches document titles, ignoring case, nearest first. A
//! path refused as not found (`links::link_path`) is never looked up. The
//! repository's resolution, `show_path` and the listing stop the lookup as
//! not found when they give nothing.
//!
//! Nothing here is shown or kept but what the library returned: a read's
//! failure keeps its code alone, and the target is never logged.

use std::path::{Path, PathBuf};

use gpui_kit::{App, Task};
use manyhands::{
    repository::{ItemContextKind, ItemDto, ItemListDto, ResolvedRepository},
    results::ResultCode,
};

use super::{ReadKind, ReadResult, Reader, Services};
use crate::desktop::model::{
    links::{CandidatePath, LinkTarget, file_name, folder_of, link_path, nearest},
    navigation::ItemLocator,
};

/// Where a link resolved. It holds what the library returned, never the
/// link's own text.
#[derive(Clone, PartialEq, Eq)]
pub(crate) enum LinkResolution {
    /// The item at the exact path, by its ID, or the file there by its
    /// path in its context when it has none; or the document a title
    /// names.
    Item(ItemLocator),
    /// No item at the path: the nearest document of the same file name,
    /// at `path` in its context.
    Nearest {
        locator: ItemLocator,
        path: String,
    },
    /// Something is at the item path that Manyhands does not read: a
    /// symbolic link, or not a regular file.
    NotManaged,
    NotFound,
}

/// What one link asks the repository for.
enum Lookup {
    Path(String),
    Title(String),
}

/// Resolves `target`, a path or a title clicked in `from` or one of its
/// comments, in the repository whose root is `repo`. Any other target,
/// and a path refused as not found, resolve to `NotFound` at once,
/// without a read.
#[allow(clippy::result_large_err)] // The library's read error, as every read gives it.
pub(crate) fn resolve(
    target: LinkTarget,
    from: &ItemDto,
    repo: &str,
    services: &Services,
    cx: &App,
) -> Task<LinkResolution> {
    let from_dir = folder_of(&from.path).to_owned();
    let lookup = match target {
        LinkTarget::Path { path, from_root } => match link_path(&path, from_root, &from_dir) {
            Some(exact) => Lookup::Path(exact),
            None => return Task::ready(LinkResolution::NotFound),
        },
        LinkTarget::Title(title) => Lookup::Title(title),
        LinkTarget::External(_) | LinkTarget::Unsafe | LinkTarget::Ignored => {
            return Task::ready(LinkResolution::NotFound);
        }
    };
    let executor = cx.background_executor().clone();
    let linking = Linking {
        reader: Reader::new(services.backend.clone(), executor.clone()),
        own: (from.context.kind == ItemContextKind::Active).then(|| from.context.worktree.clone()),
        dir: from_dir,
    };
    let root = PathBuf::from(repo);
    executor.spawn(async move {
        let repo = match linking
            .reader
            .call(ReadKind::Resolve, |service| {
                service.resolve_repository(&root)
            })
            .await
        {
            ReadResult::Ok(repo) => repo,
            _ => return LinkResolution::NotFound,
        };
        match lookup {
            Lookup::Path(exact) => linking.path(&repo, &exact).await,
            Lookup::Title(title) => linking.title(&repo, &title).await,
        }
    })
}

/// The linking item, as a lookup needs it: the reads, its own context's
/// worktree when it was read from one, and its folder.
struct Linking {
    reader: Reader,
    own: Option<String>,
    dir: String,
}

impl Linking {
    /// Exact in the item's own context, then on primary, then the nearest
    /// managed document of the same file name.
    async fn path(&self, repo: &ResolvedRepository, exact: &str) -> LinkResolution {
        let contexts = self.own.iter().map(|own| Some(PathBuf::from(own)));
        for context in contexts.chain([None]) {
            if let Some(found) = self.exact(repo, context.as_deref(), exact).await {
                return found;
            }
        }
        let Some(documents) = self.documents(repo).await else {
            return LinkResolution::NotFound;
        };
        let name = file_name(exact);
        let named: Vec<&ItemDto> = documents
            .items
            .iter()
            .filter(|row| file_name(&row.path) == name)
            .collect();
        match self.nearest_of(&named) {
            Some(row) => LinkResolution::Nearest {
                locator: ItemLocator::for_item(row),
                path: row.path.clone(),
            },
            None => LinkResolution::NotFound,
        }
    }

    /// The document titled `title`, ignoring case, nearest first.
    async fn title(&self, repo: &ResolvedRepository, title: &str) -> LinkResolution {
        let Some(documents) = self.documents(repo).await else {
            return LinkResolution::NotFound;
        };
        let wanted = title.to_lowercase();
        let titled: Vec<&ItemDto> = documents
            .items
            .iter()
            .filter(|row| {
                row.title
                    .as_deref()
                    .is_some_and(|title| title.to_lowercase() == wanted)
            })
            .collect();
        match self.nearest_of(&titled) {
            Some(row) => LinkResolution::Item(ItemLocator::for_item(row)),
            None => LinkResolution::NotFound,
        }
    }

    /// What is at `path` in `context` (primary when `None`): `None` when
    /// nothing is, no item can be, or the observe stayed busy or failed,
    /// so the lookup goes on.
    #[allow(clippy::result_large_err)] // The library's read error, as every read gives it.
    async fn exact(
        &self,
        repo: &ResolvedRepository,
        context: Option<&Path>,
        path: &str,
    ) -> Option<LinkResolution> {
        let at = Path::new(path);
        let observed = self
            .reader
            .call(ReadKind::Observe, |service| {
                service.observe_path(repo, context, at)
            })
            .await;
        if !matches!(observed, ReadResult::Ok(observed) if observed.exists) {
            return None;
        }
        let shown = self
            .reader
            .call(ReadKind::Item, |service| {
                service.show_path(repo, context, at)
            })
            .await;
        match shown {
            // By its ID, or by its path in this context when it has none,
            // as the lists open a row.
            ReadResult::Ok(item) => Some(LinkResolution::Item(ItemLocator::for_item(&item))),
            // Gone since it was observed.
            ReadResult::Failed(failure) if failure.code == ResultCode::PathNotFound => None,
            ReadResult::Failed(failure) if failure.code == ResultCode::InvalidPath => {
                Some(LinkResolution::NotManaged)
            }
            ReadResult::Busy | ReadResult::Failed(_) => Some(LinkResolution::NotFound),
        }
    }

    /// The managed documents, as the index lists them.
    #[allow(clippy::result_large_err)] // The library's read error, as every read gives it.
    async fn documents(&self, repo: &ResolvedRepository) -> Option<ItemListDto> {
        match self
            .reader
            .call(ReadKind::Documents, |service| service.list_documents(repo))
            .await
        {
            ReadResult::Ok(list) => Some(list),
            _ => None,
        }
    }

    /// The managed one of `rows` nearest the linking folder.
    fn nearest_of<'a>(&self, rows: &[&'a ItemDto]) -> Option<&'a ItemDto> {
        let managed: Vec<&ItemDto> = rows
            .iter()
            .copied()
            .filter(|row| matches!(ItemLocator::for_item(row), ItemLocator::Identified { .. }))
            .collect();
        let candidates: Vec<CandidatePath> = managed
            .iter()
            .map(|row| CandidatePath {
                path: row.path.clone(),
                in_context: self.own.as_deref() == Some(row.context.worktree.as_str()),
            })
            .collect();
        let found = nearest(&self.dir, &candidates)?;
        let at = candidates
            .iter()
            .position(|candidate| std::ptr::eq(candidate, found))?;
        Some(managed[at])
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "links_tests.rs"]
mod tests;
