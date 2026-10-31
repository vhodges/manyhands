//! Contained reads and limited decodes (design, Images; R-34, R-39), over
//! temporary trees: what `contained` refuses, that a FIFO or a directory
//! is never opened and a swapped file is never read, the size cap, the
//! magic bytes, the first frame of a GIF, the side limit and the per-tab
//! budget.

use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
};

use image::{
    AnimationDecoder, DynamicImage, Frame, ImageFormat, Rgb, RgbImage, Rgba, RgbaImage,
    codecs::gif::{GifDecoder, GifEncoder},
};

use super::{
    DecodeBudget, IMAGE_SIZE_CAP, ImageKind, NotDecoded, TAB_BUDGET, WALK_BOUND, checked,
    contained, decode, read_checked, read_contained, sniff, walk_docs, walk_docs_within,
};

/// A temporary directory holding `repo`, the tree, and `outside`, a
/// folder beside it. Its path is the system's, not canonical: `contained`
/// canonicalizes, and on Windows a canonical path is a verbatim one, in
/// which `join` would resolve the `..` a test writes.
struct Trees {
    _dir: tempfile::TempDir,
    outer: PathBuf,
    root: PathBuf,
}

fn trees() -> Trees {
    let dir = tempfile::tempdir().unwrap();
    let outer = dir.path().to_owned();
    let root = outer.join("repo");
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::create_dir_all(outer.join("outside")).unwrap();
    Trees {
        _dir: dir,
        outer,
        root,
    }
}

/// What `contained` gives for a file at `path`: its canonical path.
fn canonical(path: &Path) -> Option<PathBuf> {
    Some(path.canonicalize().unwrap())
}

/// Writes `bytes` at `path`, making its folders.
fn write(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

/// `image` encoded as `format`.
fn encoded(image: DynamicImage, format: ImageFormat) -> Vec<u8> {
    let mut bytes = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut bytes), format)
        .unwrap();
    bytes
}

/// A `width` by `height` PNG of one colour.
fn png(width: u32, height: u32) -> Vec<u8> {
    let image = RgbImage::from_pixel(width, height, Rgb([10, 20, 30]));
    encoded(DynamicImage::ImageRgb8(image), ImageFormat::Png)
}

/// A GIF of one 4 by 3 frame per colour, in order.
fn gif(colours: &[[u8; 4]]) -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = GifEncoder::new(&mut bytes);
        let frames = colours
            .iter()
            .map(|colour| Frame::new(RgbaImage::from_pixel(4, 3, Rgba(*colour))));
        encoder.encode_frames(frames).unwrap();
    }
    bytes
}

const RED: [u8; 4] = [255, 0, 0, 255];
const GREEN: [u8; 4] = [0, 255, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];

const SVG: &[u8] = b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"8\" height=\"8\"/>";
const XML_SVG: &[u8] =
    b"<?xml version=\"1.0\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"8\" height=\"8\"/>";

#[cfg(unix)]
fn link(target: &Path, at: &Path) {
    fs::create_dir_all(at.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(target, at).unwrap();
}

/// R-34 checks 1 to 3: a path through `..`, a symbolic link that leaves
/// the tree, even one that comes back, `.git`, and another item's
/// worktree below the tree are refused; a file, and a link that stays
/// inside, are not.
#[test]
fn dot_dot_symlink_escape_git_and_other_worktrees_are_refused() {
    let Trees { _dir, outer, root } = trees();
    let inside = root.join("docs/inside.png");
    write(&inside, &png(2, 2));
    write(&outer.join("outside/x.png"), &png(2, 2));
    assert_eq!(
        contained(&root, &inside),
        canonical(&inside),
        "a file in the tree"
    );

    for through in [
        "../outside/x.png",
        "docs/../../outside/x.png",
        // Even a `..` that stays inside: nothing is reached through one.
        "docs/../docs/inside.png",
    ] {
        assert_eq!(contained(&root, &root.join(through)), None, "{through}");
    }
    assert_eq!(
        contained(&root, &outer.join("outside/x.png")),
        None,
        "a path that is not below the tree"
    );

    #[cfg(unix)]
    {
        link(
            &outer.join("outside/x.png"),
            &root.join("docs/file-out.png"),
        );
        link(&outer.join("outside"), &root.join("docs/folder-out"));
        // Out to the folder that holds the tree, and back in.
        link(&outer, &root.join("docs/up"));
        write(&root.join(".git/x.png"), &png(2, 2));
        link(&root.join(".git/x.png"), &root.join("docs/into-git.png"));
        for through in [
            "docs/file-out.png",
            "docs/folder-out/x.png",
            "docs/up/repo/docs/inside.png",
            "docs/into-git.png",
        ] {
            assert_eq!(contained(&root, &root.join(through)), None, "{through}");
        }
        link(Path::new("inside.png"), &root.join("docs/alias.png"));
        assert_eq!(
            contained(&root, &root.join("docs/alias.png")),
            canonical(&inside),
            "a relative link that stays inside is followed"
        );
    }

    for refused in [
        ".git/x.png",
        "docs/.git/x.png",
        ".manyhands/x.png",
        ".manyhands/worktrees/01ARZ3NDEKTSV4RRFFQ69G5FA0/docs/pic.png",
    ] {
        write(&root.join(refused), &png(2, 2));
        assert_eq!(contained(&root, &root.join(refused)), None, "{refused}");
        assert_eq!(
            read_contained(&root, &root.join(refused)),
            None,
            "{refused}"
        );
    }
}

/// R-34 check 2 ignores ASCII case, and the names Windows reads as `.git`.
/// A name that only starts like one is a name.
#[test]
fn a_case_variant_of_git_is_refused() {
    let Trees { _dir, root, .. } = trees();
    for refused in [
        ".GIT/x.png",
        ".Git/x.png",
        "docs/.gIt/x.png",
        ".MANYHANDS/x.png",
        "docs/.ManyHands/worktrees/x/a.png",
        ".git./x.png",
        ".git /x.png",
    ] {
        write(&root.join(refused), &png(2, 2));
        assert_eq!(contained(&root, &root.join(refused)), None, "{refused}");
    }
    for allowed in [".github/x.png", "docs/git/x.png", "docs/.gitkeep.png"] {
        let path = root.join(allowed);
        write(&path, &png(2, 2));
        assert_eq!(contained(&root, &path), canonical(&path), "{allowed}");
    }
}

/// The check is relative to the tree a file is found in: below an item's
/// worktree, its own files are inside; from primary, they are another
/// worktree's.
#[test]
fn an_image_inside_a_context_worktree_is_allowed() {
    let Trees { _dir, root, .. } = trees();
    let context = root.join(".manyhands/worktrees/01ARZ3NDEKTSV4RRFFQ69G5FA0");
    let picture = context.join("docs/pic.png");
    let bytes = png(3, 2);
    write(&picture, &bytes);

    assert_eq!(contained(&context, &picture), canonical(&picture));
    assert_eq!(read_contained(&context, &picture), Some(bytes));
    assert_eq!(contained(&root, &picture), None, "from primary");

    #[cfg(unix)]
    {
        // A link from the context to primary's file leaves its tree.
        write(&root.join("docs/main.png"), &png(2, 2));
        link(&root.join("docs/main.png"), &context.join("docs/main.png"));
        assert_eq!(contained(&context, &context.join("docs/main.png")), None);
    }
}

/// R-34 check 4: only a regular file is opened. A FIFO with no writer
/// would block an open; a directory is no image.
#[test]
fn a_fifo_or_directory_is_never_opened() {
    let Trees { _dir, root, .. } = trees();
    let folder = root.join("docs/folder.png");
    fs::create_dir_all(&folder).unwrap();
    assert!(contained(&root, &folder).is_some(), "it is inside");
    assert!(checked(&folder).is_none());
    assert_eq!(read_contained(&root, &folder), None);

    #[cfg(unix)]
    {
        use std::{ffi::CString, os::unix::ffi::OsStrExt};

        let fifo = root.join("docs/fifo.png");
        let name = CString::new(fifo.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o644) }, 0);
        assert!(contained(&root, &fifo).is_some(), "it is inside");
        assert!(checked(&fifo).is_none());
        assert_eq!(read_contained(&root, &fifo), None);
    }
}

/// R-34 check 5: the file opened must be the one checked. Another file
/// moved into its place is refused, and so is a link put there, even to
/// the very file that was checked: the open does not follow it.
#[test]
fn a_handle_that_is_not_the_checked_file_is_refused() {
    let Trees { _dir, root, .. } = trees();
    let (first, other) = (root.join("docs/a.png"), root.join("docs/b.png"));
    write(&first, &png(2, 2));
    write(&other, &png(5, 5));
    let file = checked(&contained(&root, &first).unwrap()).unwrap();
    assert_eq!(read_checked(&file), Some(png(2, 2)), "the checked file");

    fs::rename(&other, &first).unwrap();
    assert_eq!(read_checked(&file), None, "another file in its place");

    #[cfg(unix)]
    {
        let (place, kept) = (root.join("docs/c.png"), root.join("docs/kept.png"));
        write(&place, &png(3, 3));
        let file = checked(&contained(&root, &place).unwrap()).unwrap();
        fs::hard_link(&place, &kept).unwrap();
        fs::remove_file(&place).unwrap();
        link(&kept, &place);
        assert_eq!(read_checked(&file), None, "a link to the same file");
    }
}

/// Magic bytes decide, never the name: SVG, with or without its XML
/// declaration, is no image.
#[test]
fn svg_named_png_is_not_an_image() {
    assert_eq!(sniff(SVG), None);
    assert_eq!(sniff(XML_SVG), None);
    let Trees { _dir, root, .. } = trees();
    let named = root.join("docs/drawing.png");
    write(&named, XML_SVG);
    let bytes = read_contained(&root, &named).expect("the file itself is readable");
    let mut budget = DecodeBudget::new();
    assert!(matches!(
        decode(&bytes, &mut budget),
        Err(NotDecoded::Refused)
    ));
    assert_eq!(budget.left(), TAB_BUDGET, "nothing was spent");
}

/// The read is capped at 20 MiB: a larger file is refused, and so is one
/// that grew past the cap after it was checked.
#[test]
fn oversized_files_are_not_read() {
    assert_eq!(IMAGE_SIZE_CAP, 20 * 1024 * 1024);
    let Trees { _dir, root, .. } = trees();
    let (at_cap, over) = (root.join("docs/at-cap.png"), root.join("docs/over.png"));
    fs::File::create(&at_cap)
        .unwrap()
        .set_len(IMAGE_SIZE_CAP)
        .unwrap();
    fs::File::create(&over)
        .unwrap()
        .set_len(IMAGE_SIZE_CAP + 1)
        .unwrap();
    assert_eq!(read_contained(&root, &over), None, "one byte over");
    assert_eq!(
        read_contained(&root, &at_cap).map(|bytes| bytes.len() as u64),
        Some(IMAGE_SIZE_CAP),
        "at the cap"
    );

    let grows = root.join("docs/grows.png");
    write(&grows, &png(2, 2));
    let file = checked(&contained(&root, &grows).unwrap()).unwrap();
    fs::OpenOptions::new()
        .write(true)
        .open(&grows)
        .unwrap()
        .set_len(IMAGE_SIZE_CAP + 1)
        .unwrap();
    assert_eq!(read_checked(&file), None, "it grew after the check");
}

/// PNG, JPEG, GIF, WebP and BMP, by their magic bytes, each decoded.
#[test]
fn each_supported_kind_is_sniffed() {
    let image = || DynamicImage::ImageRgb8(RgbImage::from_pixel(3, 2, Rgb([10, 20, 30])));
    let mut budget = DecodeBudget::new();
    for (kind, format) in [
        (ImageKind::Png, ImageFormat::Png),
        (ImageKind::Jpeg, ImageFormat::Jpeg),
        (ImageKind::Gif, ImageFormat::Gif),
        (ImageKind::Webp, ImageFormat::WebP),
        (ImageKind::Bmp, ImageFormat::Bmp),
    ] {
        let bytes = encoded(image(), format);
        assert_eq!(sniff(&bytes), Some(kind), "{format:?}");
        let Ok(frame) = decode(&bytes, &mut budget) else {
            panic!("{format:?} did not decode");
        };
        assert_eq!(frame.buffer().dimensions(), (3, 2), "{format:?}");
    }
    for (head, kind) in [
        (&b"\x89PNG"[..], ImageKind::Png),
        (b"\xFF\xD8\xFF", ImageKind::Jpeg),
        (b"GIF89a", ImageKind::Gif),
        (b"RIFF\x10\0\0\0WEBPVP8L", ImageKind::Webp),
        (b"BM", ImageKind::Bmp),
    ] {
        assert_eq!(sniff(head), Some(kind));
    }
    for head in [
        &b""[..],
        b"\x89PN",
        b"\xFF\xD8",
        b"GIF7",
        b"RIFF\x10\0\0\0WAVE",
        b"RIFFWEBP",
        b"B",
        b"# A heading",
    ] {
        assert_eq!(sniff(head), None, "{head:?}");
    }
}

/// The frame is handed over in BGRA, as `RenderImage` takes it.
#[test]
fn a_decoded_image_is_bgra() {
    let mut budget = DecodeBudget::new();
    let Ok(frame) = decode(&png(2, 1), &mut budget) else {
        panic!("the PNG did not decode");
    };
    assert_eq!(frame.buffer().get_pixel(0, 0).0, [30, 20, 10, 255]);
}

/// R-39: a GIF gives its first frame only. Cut inside its second frame,
/// it still does, though reading every frame fails.
#[test]
fn a_many_frame_gif_decodes_only_its_first_frame() {
    let many = gif(&[RED, GREEN, BLUE]);
    let mut budget = DecodeBudget::new();
    let Ok(frame) = decode(&many, &mut budget) else {
        panic!("the GIF did not decode");
    };
    assert_eq!(frame.buffer().dimensions(), (4, 3));
    let bgra_red = [0, 0, 255, 255];
    assert!(frame.buffer().pixels().all(|pixel| pixel.0 == bgra_red));
    assert_eq!(budget.left(), TAB_BUDGET - 4 * 3 * 4, "one frame spent");

    // The one-frame GIF is the many-frame one up to its trailer.
    let one = gif(&[RED]);
    let first = &one[..one.len() - 1];
    assert!(many.starts_with(first), "the encoder's frames differ");
    let cut = &many[..first.len() + 12];
    let every =
        GifDecoder::new(Cursor::new(cut)).and_then(|gif| gif.into_frames().collect_frames());
    assert!(every.is_err(), "the cut leaves the second frame whole");
    let Ok(frame) = decode(cut, &mut budget) else {
        panic!("the cut GIF's first frame did not decode");
    };
    assert!(frame.buffer().pixels().all(|pixel| pixel.0 == bgra_red));
}

/// R-39: either side over 4096 pixels is refused, and nothing is spent.
#[test]
fn an_image_over_4096_pixels_is_refused() {
    let mut budget = DecodeBudget::new();
    for (width, height) in [(4097, 1), (1, 4097)] {
        assert!(
            matches!(
                decode(&png(width, height), &mut budget),
                Err(NotDecoded::Refused)
            ),
            "{width} by {height}"
        );
    }
    assert_eq!(budget.left(), TAB_BUDGET);
    assert!(
        decode(&png(4096, 1), &mut budget).is_ok(),
        "4096 is allowed"
    );
    assert!(
        decode(&png(1, 4096), &mut budget).is_ok(),
        "4096 is allowed"
    );
}

/// R-39: a tab decodes at most 256 MiB. A decode that would go past what
/// is left is refused before it starts, as a broken file's header shows,
/// and spends nothing.
#[test]
fn the_per_tab_budget_stops_further_decodes() {
    assert_eq!(TAB_BUDGET, 256 * 1024 * 1024);
    assert_eq!(DecodeBudget::new().left(), TAB_BUDGET);

    let mut budget = DecodeBudget::with_left(10 * 10 * 4 + 5 * 5 * 4);
    assert!(decode(&png(10, 10), &mut budget).is_ok());
    assert_eq!(budget.left(), 5 * 5 * 4);
    assert!(matches!(
        decode(&png(6, 6), &mut budget),
        Err(NotDecoded::OverBudget)
    ));
    let broken = png(6, 6);
    let broken = &broken[..broken.len() / 2];
    assert!(
        matches!(decode(broken, &mut budget), Err(NotDecoded::OverBudget)),
        "refused before decoding"
    );
    assert_eq!(budget.left(), 5 * 5 * 4, "a refusal spends nothing");
    assert!(
        decode(&png(5, 5), &mut budget).is_ok(),
        "exactly what is left"
    );
    assert_eq!(budget.left(), 0);
    assert!(matches!(
        decode(&png(1, 1), &mut budget),
        Err(NotDecoded::OverBudget)
    ));
}

/// R-33: the walk lists the regular files below `docs/`, by their paths
/// in the tree, and never follows a link or enters `.git` or
/// `.manyhands`.
#[test]
fn the_walk_lists_docs_without_following_links() {
    let Trees { _dir, outer, root } = trees();
    for file in [
        "docs/a.png",
        "docs/sub/b.png",
        "docs/.git/c.png",
        "docs/.MANYHANDS/d.png",
        "e.png",
    ] {
        write(&root.join(file), &png(1, 1));
    }
    write(&outer.join("outside/f.png"), &png(1, 1));
    #[cfg(unix)]
    {
        link(&outer.join("outside"), &root.join("docs/out"));
        link(&root.join("docs/a.png"), &root.join("docs/alias.png"));
    }
    let mut listed = walk_docs(&root);
    listed.sort();
    assert_eq!(listed, ["docs/a.png", "docs/sub/b.png"]);
}

/// Bits packed for a deflate stream: data elements from their low bit,
/// Huffman codes from their high bit.
#[derive(Default)]
struct Bits {
    bytes: Vec<u8>,
    count: u64,
}

impl Bits {
    fn bit(&mut self, on: bool) {
        if self.count.is_multiple_of(8) {
            self.bytes.push(0);
        }
        if on {
            *self.bytes.last_mut().unwrap() |= 1 << (self.count % 8);
        }
        self.count += 1;
    }

    fn value(&mut self, value: u32, width: u32) {
        (0..width).for_each(|at| self.bit(value >> at & 1 == 1));
    }

    fn code(&mut self, code: u32, width: u32) {
        (0..width)
            .rev()
            .for_each(|at| self.bit(code >> at & 1 == 1));
    }
}

fn crc32(parts: &[&[u8]]) -> u32 {
    let mut crc = !0u32;
    for byte in parts.iter().flat_map(|part| part.iter()) {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn chunk(png: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    png.extend((data.len() as u32).to_be_bytes());
    png.extend(kind);
    png.extend(data);
    png.extend(crc32(&[kind, data]).to_be_bytes());
}

/// A `width` by `height` PNG of zeros with `channels` samples of `depth`
/// bits a pixel, of PNG colour type `colour`, written by hand: one
/// fixed-Huffman deflate block of a literal zero and 258-byte copies of
/// it, and the Adler-32 of zeros, which is their count. A large one takes
/// milliseconds, where the encoder takes seconds in a debug build.
fn zeros_png(width: u32, height: u32, depth: u8, colour: u8, channels: u32) -> Vec<u8> {
    // Each row is its filter byte, 0 for none, then its samples.
    let row = 1 + u64::from(width) * u64::from(channels) * u64::from(depth) / 8;
    let raw = row * u64::from(height);
    let mut bits = Bits::default();
    bits.value(1, 1); // The last block.
    bits.value(1, 2); // Fixed Huffman codes.
    bits.code(0x30, 8); // The literal 0.
    let mut left = raw - 1;
    while left >= 258 {
        bits.code(0xC5, 8); // Length 258, symbol 285.
        bits.code(0, 5); // Distance 1.
        left -= 258;
    }
    (0..left).for_each(|_| bits.code(0x30, 8));
    bits.code(0, 7); // The end of the block, symbol 256.
    let mut zlib = vec![0x78, 0x01];
    zlib.extend(&bits.bytes);
    zlib.extend((((raw % 65_521) << 16) as u32 | 1).to_be_bytes());

    let mut header = Vec::new();
    header.extend(width.to_be_bytes());
    header.extend(height.to_be_bytes());
    header.extend([depth, colour, 0, 0, 0]);
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    chunk(&mut png, b"IHDR", &header);
    chunk(&mut png, b"IDAT", &zlib);
    chunk(&mut png, b"IEND", &[]);
    png
}

/// R-39's allocation limit: an image within 4096 pixels a side whose
/// decode needs more than 96 MiB is refused. A 4096 by 4096 RGBA PNG of
/// 16-bit samples is 128 MiB decoded. The same hand-made PNG, small,
/// decodes, so the large one is refused for its size.
#[test]
fn a_decode_over_96_mib_is_refused() {
    let mut budget = DecodeBudget::new();
    let small = zeros_png(3, 2, 16, 6, 4);
    let Ok(frame) = decode(&small, &mut budget) else {
        panic!("the hand-made PNG did not decode");
    };
    assert_eq!(frame.buffer().dimensions(), (3, 2));

    let large = zeros_png(4096, 4096, 16, 6, 4);
    assert!(matches!(
        decode(&large, &mut budget),
        Err(NotDecoded::Refused)
    ));
    assert_eq!(budget.left(), TAB_BUDGET - 3 * 2 * 4, "nothing more spent");
}

/// R-33's bound counts every entry the walk visits. With a bound of 3 it
/// reads the three entries of `docs/` and stops before entering its
/// folder; with 4 it reads the folder's file too.
#[test]
fn the_walk_stops_at_its_bound() {
    assert_eq!(WALK_BOUND, 10_000);
    let Trees { _dir, root, .. } = trees();
    for file in ["docs/a.png", "docs/b.png", "docs/sub/c.png"] {
        write(&root.join(file), &png(1, 1));
    }
    let walked = |bound| {
        let mut listed = walk_docs_within(&root, bound);
        listed.sort();
        listed
    };
    assert_eq!(walked(3), ["docs/a.png", "docs/b.png"]);
    assert_eq!(walked(4), ["docs/a.png", "docs/b.png", "docs/sub/c.png"]);
    let mut whole = walk_docs(&root);
    whole.sort();
    assert_eq!(whole, walked(WALK_BOUND), "the walk is the bounded one");
}
