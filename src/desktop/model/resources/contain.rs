//! R-34 checks 1 to 3 (design, Images, Containment), decided without
//! following any symbolic link (Part B review, Important 1): no lookup
//! reaches a path outside the tree, or a network path, before the checks
//! pass.
//!
//! The walk goes from the tree's root one step at a time, with
//! `symlink_metadata` only, on paths whose every folder has passed. Each
//! link on the way is decided from `read_link`, never followed:
//! - an absolute, rooted or prefixed target (`/…`, `C:…`, `\…`, `\\…`,
//!   `\\?\…`) is refused, even one inside the root (a Known Limit);
//! - a relative target's steps are walked from the link's folder, in
//!   order. A `..` goes back one step from where the walk is, after the
//!   links before it, as `realpath` takes it, with no lookup; a climb
//!   above the root is refused;
//! - a chain is decided link by link, up to `HOP_LIMIT` links, and past it
//!   the path is refused;
//! - past anything but a folder there is no path, `..` included, as the
//!   system finds it.
//!
//! Only once every link on the way has passed is a path made canonical,
//! and checks 1 and 2 run on that. That path is the place the walk
//! reached, the root and the names it found, none of them a link, never
//! the path as written: Windows takes a relative target's `..` by name,
//! from the link's folder, so it could resolve the written path through
//! a link the walk never decided (Part B re-review, N1). A reserved step,
//! `.git` or `.manyhands`, is refused wherever the walk meets it, once the
//! path is known to be there.
//!
//! What the walk finds tells the exact lookup whether the tree holds
//! anything at the path (ruling 3): a missing step of the written path is
//! nothing there; once its last step is there, a dangling link or a
//! refused target is something, refused.

use std::{
    collections::VecDeque,
    ffi::{OsStr, OsString},
    fs, io,
    path::{Component, Path, PathBuf},
};

/// The most links one lookup passes through, as on Linux.
const HOP_LIMIT: usize = 40;

/// The filesystem calls containment makes, so that a test can record the
/// paths they are given: the walk's `symlink_metadata` and `read_link`,
/// and `canonicalize` once the walk has passed.
trait Lookups {
    fn symlink_metadata(&self, path: &Path) -> io::Result<fs::Metadata>;
    fn read_link(&self, path: &Path) -> io::Result<PathBuf>;
    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf>;
}

/// `std::fs`.
struct Os;

impl Lookups for Os {
    fn symlink_metadata(&self, path: &Path) -> io::Result<fs::Metadata> {
        fs::symlink_metadata(path)
    }

    fn read_link(&self, path: &Path) -> io::Result<PathBuf> {
        fs::read_link(path)
    }

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        path.canonicalize()
    }
}

/// What a tree holds at a path. It holds a path the body names, so it
/// derives nothing.
pub(crate) enum Found {
    /// Nothing: a step of the written path is not there.
    Nothing,
    /// Something, which containment refuses.
    Refused,
    /// Something inside the tree, by its canonical path.
    Contained(PathBuf),
}

/// R-34 checks 1 to 3: the canonical path of `candidate`, written below
/// `root`, the tree it was looked up in, when each symbolic link on the
/// way stays inside the tree, no step is `.git` or `.manyhands`, as
/// walked or canonical, and the canonical path is inside the tree's
/// canonical root. A `..`, or a candidate that is not below the root as
/// written, is refused. Something must be there.
pub(crate) fn contained(root: &Path, candidate: &Path) -> Option<PathBuf> {
    match locate(root, candidate) {
        Found::Contained(canonical) => Some(canonical),
        Found::Nothing | Found::Refused => None,
    }
}

/// What the tree at `root` holds at `candidate`, written below it, under
/// checks 1 to 3.
pub(crate) fn locate(root: &Path, candidate: &Path) -> Found {
    locate_with(root, candidate, &Os)
}

fn locate_with(root: &Path, candidate: &Path, lookups: &impl Lookups) -> Found {
    let Some(written) = written_steps(root, candidate) else {
        return Found::Refused;
    };
    match walk(root, written, lookups) {
        Ok(reached) => {
            canonical_inside(root, &reached, lookups).map_or(Found::Refused, Found::Contained)
        }
        Err(found) => found,
    }
}

/// The steps of `candidate` below `root`, as written: none for a path not
/// below the root, with no step, or with a step that is not a name.
fn written_steps(root: &Path, candidate: &Path) -> Option<Vec<OsString>> {
    let steps: Vec<OsString> = candidate
        .strip_prefix(root)
        .ok()?
        .components()
        .map(|step| match step {
            Component::Normal(name) => Some(name.to_owned()),
            _ => None,
        })
        .collect::<Option<_>>()?;
    (!steps.is_empty()).then_some(steps)
}

/// A step the walk takes: a name to look up, or a `..` of a link's target.
enum Step {
    Name(OsString),
    Up,
}

/// Walks `written` from `root`, deciding each link on the way from its
/// target, and looking up only paths whose folders have passed. Gives
/// the place it reached, with no link on it.
fn walk(root: &Path, written: Vec<OsString>, lookups: &impl Lookups) -> Result<PathBuf, Found> {
    let count = written.len();
    // Each step to take, and whether it is the written path's last.
    let mut steps: VecDeque<(Step, bool)> = written
        .into_iter()
        .enumerate()
        .map(|(at, name)| (Step::Name(name), at + 1 == count))
        .collect();
    let mut at = root.to_owned();
    let mut depth = 0;
    let mut hops = 0;
    // The written path's last step is there: a miss from now on is a
    // refusal.
    let mut there = false;
    let mut reserved_step = false;
    while let Some((step, last)) = steps.pop_front() {
        let name = match step {
            Step::Up if depth == 0 => return Err(Found::Refused),
            Step::Up => {
                at.pop();
                depth -= 1;
                continue;
            }
            Step::Name(name) => name,
        };
        at.push(&name);
        depth += 1;
        let missing = |there| {
            if there {
                Found::Refused
            } else {
                Found::Nothing
            }
        };
        let Ok(metadata) = lookups.symlink_metadata(&at) else {
            return Err(missing(there));
        };
        there |= last;
        reserved_step |= reserved(&name);
        let kind = metadata.file_type();
        if !kind.is_symlink() {
            // Past anything but a folder there is no path, as the system
            // finds it, not even back up by `..`.
            if !kind.is_dir() && !steps.is_empty() {
                return Err(missing(there));
            }
            continue;
        }
        hops += 1;
        if hops > HOP_LIMIT {
            return Err(Found::Refused);
        }
        let target = lookups.read_link(&at).map_err(|_| Found::Refused)?;
        let target = relative_steps(&target).ok_or(Found::Refused)?;
        at.pop();
        depth -= 1;
        for step in target.into_iter().rev() {
            steps.push_front((step, false));
        }
    }
    if reserved_step {
        Err(Found::Refused)
    } else {
        Ok(at)
    }
}

/// A link's target as steps from the link's folder: none for an absolute,
/// rooted or prefixed target, or an empty one, which the system does not
/// resolve.
fn relative_steps(target: &Path) -> Option<Vec<Step>> {
    if target.as_os_str().is_empty() {
        return None;
    }
    target
        .components()
        .filter_map(|step| match step {
            Component::Normal(name) => Some(Some(Step::Name(name.to_owned()))),
            Component::ParentDir => Some(Some(Step::Up)),
            Component::CurDir => None,
            Component::RootDir | Component::Prefix(_) => Some(None),
        })
        .collect()
}

/// Checks 1 and 2 on the canonical path of `reached`, the place the walk
/// reached once every link on the way had passed.
fn canonical_inside(root: &Path, reached: &Path, lookups: &impl Lookups) -> Option<PathBuf> {
    let base = lookups.canonicalize(root).ok()?;
    let canonical = lookups.canonicalize(reached).ok()?;
    inside(&base, &canonical)?;
    Some(canonical)
}

/// `path` is `base` or below it, and no step below it is reserved.
fn inside(base: &Path, path: &Path) -> Option<()> {
    path.strip_prefix(base)
        .ok()?
        .components()
        .all(|step| matches!(step, Component::Normal(name) if !reserved(name)))
        .then_some(())
}

/// `.git` or `.manyhands`, ignoring ASCII case, and also with what Windows
/// ignores after a name: trailing dots and spaces, and a `:stream`.
pub(super) fn reserved(name: &OsStr) -> bool {
    let Some(name) = name.to_str() else {
        return false;
    };
    let name = name.split(':').next().unwrap_or_default();
    let name = name.trim_end_matches(['.', ' ']);
    name.eq_ignore_ascii_case(".git") || name.eq_ignore_ascii_case(".manyhands")
}

#[cfg(test)]
#[path = "contain_tests.rs"]
mod tests;
