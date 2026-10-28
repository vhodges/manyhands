//! Local images (design, Images; decision 9): which file a tree holds may
//! be read (R-34), how it is read and decoded (R-39), and the walk the
//! nearest name looks in (R-33). Nothing here runs on the UI thread: the
//! tab's loads call it on the background executor (`services::images`).
//!
//! **Containment** (R-34). A file is read only when all of these hold:
//! 1. its canonical path is inside the canonical root of its tree;
//! 2. no step of its path below that root, as written or canonical, is
//!    `.git` or `.manyhands`, ignoring ASCII case and the trailing dots,
//!    spaces and `:stream` Windows reads as the same name;
//! 3. every symbolic link on the way resolves inside that root;
//! 4. `symlink_metadata` of the canonical path shows a regular file, whose
//!    identity is kept;
//! 5. it is opened without following a final symbolic link, and the
//!    handle has the identity kept.
//!
//! The identity is the device and inode on Unix. On Windows, stable `std`
//! gives no file ID (`MetadataExt::file_index` is unstable), so it is the
//! size, the creation and last-write times and the attributes, and the
//! handle must not be a reparse point. Elsewhere nothing is read. The
//! read stops one byte past `IMAGE_SIZE_CAP`, and more is refused.
//!
//! **Decode** (R-39). The format comes from the magic bytes alone, never
//! the name, so SVG is refused. `image` decodes it under `Limits` of 4096
//! pixels a side and 96 MiB, a GIF's first frame only, after the tab's
//! budget says the frame fits. The pixels are handed over in BGRA.

use std::{
    collections::VecDeque,
    ffi::OsStr,
    fs,
    io::{Cursor, Read},
    path::{Component, Path, PathBuf},
};

use image::{Frame, ImageFormat, ImageReader, Limits};

/// The most an image file may hold.
pub(crate) const IMAGE_SIZE_CAP: u64 = 20 * 1024 * 1024;
/// The most a decoded image may be, in pixels, either way.
const MAX_SIDE: u32 = 4096;
/// The most the decoder may allocate for one image: a 4096 by 4096 RGBA
/// image is 64 MiB.
const MAX_ALLOC: u64 = 96 * 1024 * 1024;
/// The most one tab holds in decoded images, in bytes.
pub(crate) const TAB_BUDGET: u64 = 256 * 1024 * 1024;
/// The most entries the nearest-name walk of one tree visits (R-33).
const WALK_BOUND: usize = 10_000;

/// A raster format the desktop reads.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ImageKind {
    Png,
    Jpeg,
    Gif,
    Webp,
    Bmp,
}

impl ImageKind {
    fn format(self) -> ImageFormat {
        match self {
            Self::Png => ImageFormat::Png,
            Self::Jpeg => ImageFormat::Jpeg,
            Self::Gif => ImageFormat::Gif,
            Self::Webp => ImageFormat::WebP,
            Self::Bmp => ImageFormat::Bmp,
        }
    }
}

/// The format `head`, a file's first bytes, starts with: PNG `\x89PNG`,
/// JPEG `\xFF\xD8\xFF`, GIF `GIF8`, WebP `RIFF…WEBP` or BMP `BM`. Anything
/// else, SVG included, is none.
pub(crate) fn sniff(head: &[u8]) -> Option<ImageKind> {
    if head.starts_with(b"\x89PNG") {
        Some(ImageKind::Png)
    } else if head.starts_with(b"\xFF\xD8\xFF") {
        Some(ImageKind::Jpeg)
    } else if head.starts_with(b"GIF8") {
        Some(ImageKind::Gif)
    } else if head.starts_with(b"RIFF") && head.get(8..12) == Some(b"WEBP") {
        Some(ImageKind::Webp)
    } else if head.starts_with(b"BM") {
        Some(ImageKind::Bmp)
    } else {
        None
    }
}

/// R-34 checks 1 to 3: the canonical path of `candidate`, written below
/// `root`, the tree it was looked up in, when it is inside the tree's
/// canonical root, no step below the root is `.git` or `.manyhands`, as
/// written or canonical, and each symbolic link on the way resolves
/// inside the root. A `..`, or a candidate that is not below the root as
/// written, is refused. Something must be there.
pub(crate) fn contained(root: &Path, candidate: &Path) -> Option<PathBuf> {
    let written = candidate.strip_prefix(root).ok()?;
    let steps: Vec<&OsStr> = written
        .components()
        .map(|step| match step {
            Component::Normal(name) if !reserved(name) => Some(name),
            _ => None,
        })
        .collect::<Option<_>>()?;
    if steps.is_empty() {
        return None;
    }
    let base = root.canonicalize().ok()?;
    let mut at = root.to_owned();
    for step in steps {
        at.push(step);
        let link = fs::symlink_metadata(&at).ok()?.file_type().is_symlink();
        if link {
            inside(&base, &at.canonicalize().ok()?)?;
        }
    }
    let canonical = candidate.canonicalize().ok()?;
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
fn reserved(name: &OsStr) -> bool {
    let Some(name) = name.to_str() else {
        return false;
    };
    let name = name.split(':').next().unwrap_or_default();
    let name = name.trim_end_matches(['.', ' ']);
    name.eq_ignore_ascii_case(".git") || name.eq_ignore_ascii_case(".manyhands")
}

/// What a file is, as R-34 checks 4 and 5 compare it.
#[cfg(unix)]
#[derive(Clone, Copy, PartialEq, Eq)]
struct Identity {
    device: u64,
    inode: u64,
}

#[cfg(unix)]
fn identity(metadata: &fs::Metadata) -> Option<Identity> {
    use std::os::unix::fs::MetadataExt;
    Some(Identity {
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}

/// Stable `std` gives no Windows file ID, so the strongest it allows: the
/// size, the creation and last-write times and the attributes, of a file
/// that is no reparse point.
#[cfg(windows)]
#[derive(Clone, Copy, PartialEq, Eq)]
struct Identity {
    size: u64,
    created: u64,
    written: u64,
    attributes: u32,
}

#[cfg(windows)]
fn identity(metadata: &fs::Metadata) -> Option<Identity> {
    use std::os::windows::fs::MetadataExt;
    /// `FILE_ATTRIBUTE_REPARSE_POINT`: a symbolic link or a junction.
    const REPARSE_POINT: u32 = 0x400;
    let attributes = metadata.file_attributes();
    ((attributes & REPARSE_POINT) == 0).then_some(Identity {
        size: metadata.file_size(),
        created: metadata.creation_time(),
        written: metadata.last_write_time(),
        attributes,
    })
}

/// No identity can be kept here, so nothing is read.
#[cfg(not(any(unix, windows)))]
#[derive(Clone, Copy, PartialEq, Eq)]
struct Identity;

#[cfg(not(any(unix, windows)))]
fn identity(_: &fs::Metadata) -> Option<Identity> {
    None
}

/// Opens `path` to read, not following a final symbolic link, and on
/// Unix not blocking on a FIFO put there since it was checked.
#[cfg(unix)]
fn open_no_follow(path: &Path) -> std::io::Result<fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_NOCTTY)
        .open(path)
}

#[cfg(windows)]
fn open_no_follow(path: &Path) -> std::io::Result<fs::File> {
    use std::os::windows::fs::OpenOptionsExt;
    /// `FILE_FLAG_OPEN_REPARSE_POINT`: the link itself, not its target.
    const OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    fs::OpenOptions::new()
        .read(true)
        .custom_flags(OPEN_REPARSE_POINT)
        .open(path)
}

#[cfg(not(any(unix, windows)))]
fn open_no_follow(_: &Path) -> std::io::Result<fs::File> {
    Err(std::io::ErrorKind::Unsupported.into())
}

/// A regular file, as R-34 check 4 found it.
pub(crate) struct Checked {
    path: PathBuf,
    identity: Identity,
}

/// R-34 check 4: `path`, a canonical path, is a regular file, by
/// `symlink_metadata`; its identity is kept. A FIFO, a device, a
/// directory or a link is never opened.
pub(crate) fn checked(path: &Path) -> Option<Checked> {
    let metadata = fs::symlink_metadata(path).ok()?;
    if !metadata.file_type().is_file() {
        return None;
    }
    Some(Checked {
        path: path.to_owned(),
        identity: identity(&metadata)?,
    })
}

/// R-34 check 5, and the cap: the bytes of `file`, opened without
/// following a final link, when the handle is the regular file that was
/// checked and holds at most `IMAGE_SIZE_CAP` bytes. The read stops one
/// byte past the cap, which shows a file that grew.
pub(crate) fn read_checked(file: &Checked) -> Option<Vec<u8>> {
    let opened = open_no_follow(&file.path).ok()?;
    let metadata = opened.metadata().ok()?;
    if !metadata.is_file() || identity(&metadata)? != file.identity {
        return None;
    }
    if metadata.len() > IMAGE_SIZE_CAP {
        return None;
    }
    let mut bytes = Vec::new();
    opened
        .take(IMAGE_SIZE_CAP + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    (bytes.len() as u64 <= IMAGE_SIZE_CAP).then_some(bytes)
}

/// All five checks and the cap: the bytes of `candidate`, written below
/// `root`, its tree.
pub(crate) fn read_contained(root: &Path, candidate: &Path) -> Option<Vec<u8>> {
    read_checked(&checked(&contained(root, candidate)?)?)
}

/// What a tab may still decode, in bytes of decoded image.
#[derive(Clone, Copy)]
pub(crate) struct DecodeBudget {
    left: u64,
}

impl DecodeBudget {
    /// What is left of `TAB_BUDGET` while a tab holds `held` bytes of
    /// decoded images.
    pub(crate) fn holding(held: u64) -> Self {
        Self {
            left: TAB_BUDGET.saturating_sub(held),
        }
    }

    /// A tab's whole budget.
    #[cfg(test)]
    pub(crate) fn new() -> Self {
        Self::holding(0)
    }

    #[cfg(test)]
    pub(crate) fn with_left(left: u64) -> Self {
        Self { left }
    }

    #[cfg(test)]
    pub(crate) fn left(&self) -> u64 {
        self.left
    }
}

/// The bytes a decoded `width` by `height` image takes, four a pixel.
pub(crate) fn decoded_size(width: u32, height: u32) -> u64 {
    u64::from(width) * u64::from(height) * 4
}

/// Why an image was not decoded. It holds nothing of the file.
pub(crate) enum NotDecoded {
    /// The decoded frame would not fit what the tab has left.
    OverBudget,
    /// Not a supported raster format, over a limit, or broken.
    Refused,
}

/// R-39: `bytes` decoded, as one BGRA frame, and its size spent from
/// `budget`. The format comes from the magic bytes. The header is read
/// first, under the limits, and a frame that would not fit what is left
/// is refused before anything is decoded. A GIF gives its first frame.
pub(crate) fn decode(bytes: &[u8], budget: &mut DecodeBudget) -> Result<Frame, NotDecoded> {
    let kind = sniff(bytes).ok_or(NotDecoded::Refused)?;
    let reader = || {
        let mut reader = ImageReader::with_format(Cursor::new(bytes), kind.format());
        reader.limits(limits());
        reader
    };
    let (width, height) = reader()
        .into_dimensions()
        .map_err(|_| NotDecoded::Refused)?;
    let size = decoded_size(width, height);
    if size > budget.left {
        return Err(NotDecoded::OverBudget);
    }
    let decoded = reader().decode().map_err(|_| NotDecoded::Refused)?;
    let mut pixels = decoded.into_rgba8();
    for pixel in pixels.pixels_mut() {
        pixel.0.swap(0, 2);
    }
    budget.left -= size;
    Ok(Frame::new(pixels))
}

/// `Limits` is non-exhaustive, so it is set by field.
fn limits() -> Limits {
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_SIDE);
    limits.max_image_height = Some(MAX_SIDE);
    limits.max_alloc = Some(MAX_ALLOC);
    limits
}

/// R-33: the regular files below `tree`'s `docs/`, by their paths in the
/// tree with forward slashes, breadth first and in name order. It follows
/// no symbolic link, enters no `.git` or `.manyhands`, and stops after
/// `WALK_BOUND` entries.
pub(crate) fn walk_docs(tree: &Path) -> Vec<String> {
    let mut files = Vec::new();
    let docs = tree.join("docs");
    let is_folder = fs::symlink_metadata(&docs).is_ok_and(|docs| docs.file_type().is_dir());
    if !is_folder {
        return files;
    }
    let mut seen = 0;
    let mut folders = VecDeque::from([(docs, "docs".to_owned())]);
    while let Some((folder, path)) = folders.pop_front() {
        let Ok(entries) = fs::read_dir(&folder) else {
            continue;
        };
        let mut entries: Vec<fs::DirEntry> = entries.filter_map(Result::ok).collect();
        entries.sort_by_key(fs::DirEntry::file_name);
        for entry in entries {
            seen += 1;
            if seen > WALK_BOUND {
                return files;
            }
            let name = entry.file_name();
            let (Some(text), Ok(kind)) = (name.to_str(), entry.file_type()) else {
                continue;
            };
            if reserved(&name) {
                continue;
            }
            let below = format!("{path}/{text}");
            if kind.is_dir() {
                folders.push_back((entry.path(), below));
            } else if kind.is_file() {
                files.push(below);
            }
        }
    }
    files
}

#[cfg(test)]
#[path = "resources_tests.rs"]
mod tests;
