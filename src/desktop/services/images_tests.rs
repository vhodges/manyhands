//! A tab's images (design, Images; decision 9), through a read editor
//! shown on its own over temporary trees: a local PNG loads on the
//! background executor after the frame that asked for it and notifies the
//! editor; an external or refused `src` is never looked up; a missing path
//! goes to the nearest image of the same name below `docs/`, from a walk
//! the tab keeps; a miss or a refusal is never looked up again; a new
//! place to resolve from drops the old images from the atlas; and the
//! tab's reload clears what it holds.

use std::{
    cell::{Cell, RefCell},
    fs,
    io::Cursor,
    path::PathBuf,
    rc::Rc,
    sync::Arc,
};

use gpui_kit::{AppContext, Entity, RenderImage, TestAppContext, component::Root};
use image::{DynamicImage, ImageFormat, Rgb, RgbImage};
use manyhands::repository::ItemDtoKind;
use zorite_editor::EditorState;

use super::{ImageSource, ImageTree, Seen, TabImages};
use crate::desktop::{
    model::navigation::OpenTarget,
    services::ReadKind,
    test_support::fixtures::{Fixture, item_id},
    view::{
        editor::read_only::ReadOnly,
        harness::{Shell, key},
        item::ItemPane,
    },
};

const DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FM0";

/// A `width` by `height` PNG.
fn png(width: u32, height: u32) -> Vec<u8> {
    let image = RgbImage::from_pixel(width, height, Rgb([10, 20, 30]));
    let mut bytes = Vec::new();
    DynamicImage::ImageRgb8(image)
        .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
        .unwrap();
    bytes
}

/// A canonical temporary folder to make trees in.
struct Folder {
    _dir: tempfile::TempDir,
    path: PathBuf,
}

impl Folder {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().canonicalize().unwrap();
        Self { _dir: dir, path }
    }

    /// Writes `bytes` at `path` below the tree `tree`.
    fn write(&self, tree: &str, path: &str, bytes: &[u8]) {
        let at = self.path.join(tree).join(path);
        fs::create_dir_all(at.parent().unwrap()).unwrap();
        fs::write(at, bytes).unwrap();
    }

    fn tree(&self, tree: &str, in_context: bool) -> ImageTree {
        let root = self.path.join(tree);
        fs::create_dir_all(&root).unwrap();
        ImageTree { root, in_context }
    }
}

/// A read editor over `body`, on its own in the shell's window, showing
/// the images of a tab whose srcs resolve from `dir` in `trees`. Its
/// first frame is drawn.
struct Reading {
    shell: Shell,
    images: TabImages,
    editor: Entity<EditorState>,
}

impl Reading {
    fn new(
        body: &str,
        trees: Vec<ImageTree>,
        dir: &str,
        hold: bool,
        cx: &mut TestAppContext,
    ) -> (Self, Option<futures::channel::oneshot::Sender<()>>) {
        let shell = Shell::open(Fixture::new(), cx);
        let held = hold.then(|| shell.backend.hold_read(ReadKind::Image));
        let services = shell
            .workspace
            .read_with(cx, |workspace, _| workspace.services().clone());
        let images = TabImages::new(&services);
        let source = ImageSource {
            trees,
            dir: dir.to_owned(),
        };
        cx.update(|cx| images.locate(source, cx));
        let made = Rc::new(RefCell::new(None));
        shell
            .window
            .update(cx, {
                let (made, images, body) = (made.clone(), images.clone(), body.to_owned());
                move |_, window, cx| {
                    window.replace_root(cx, |window, cx| {
                        let read = cx.new(|cx| ReadOnly::new(&body, window, cx));
                        read.update(cx, |read, cx| read.show_images(&images, cx));
                        *made.borrow_mut() = Some(read.clone());
                        Root::new(read, window, cx)
                    });
                }
            })
            .unwrap();
        let read: Entity<ReadOnly> = made.borrow_mut().take().unwrap();
        let editor = read.read_with(cx, |read, _| read.editor().clone());
        (
            Self {
                shell,
                images,
                editor,
            },
            held,
        )
    }

    /// Whether the editor's last paint drew `row` as an image: far taller
    /// than the text row `text`.
    fn drawn_as_image(&self, row: usize, text: usize, cx: &mut TestAppContext) -> bool {
        drawn_as_image(&self.editor, row, text, cx)
    }

    fn loads(&self) -> usize {
        self.shell.backend.reads_seen(ReadKind::Image)
    }
}

/// Whether `editor`'s last paint drew `row` as an image: more than three
/// times as tall as the text row `text`. The editor draws a block image
/// its height over the window's scale factor tall, so an image a test
/// looks for is a tall one.
fn drawn_as_image(
    editor: &Entity<EditorState>,
    row: usize,
    text: usize,
    cx: &mut TestAppContext,
) -> bool {
    let rows = editor.read_with(cx, |editor, _| editor.row_layout());
    let (_, image) = rows[row];
    let (_, line) = rows[text];
    image > line * 3.
}

/// The frame that meets the `src` draws its source and starts nothing
/// itself: the load waits for the background executor. Once it lands,
/// the editor is notified and draws the image.
#[gpui_kit::test]
fn a_local_png_loads_off_the_ui_thread_and_notifies(cx: &mut TestAppContext) {
    let folder = Folder::new();
    folder.write("repo", "docs/pic.png", &png(30, 300));
    let tree = folder.tree("repo", false);
    let (reading, hold) = Reading::new(
        "![a picture](pic.png)\n\nAfter.\n",
        vec![tree],
        "docs",
        true,
        cx,
    );
    assert_eq!(reading.images.seen("pic.png"), Some(Seen::Pending));
    assert_eq!(reading.loads(), 1, "the frame's miss started one load");
    assert!(!reading.drawn_as_image(0, 2, cx), "its source shows");
    reading.shell.run(cx);
    assert_eq!(
        reading.images.seen("pic.png"),
        Some(Seen::Pending),
        "held at its gate, the load has not run"
    );

    let notified = Rc::new(Cell::new(0));
    cx.update({
        let notified = notified.clone();
        |cx| {
            cx.observe(&reading.editor, move |_, _| {
                notified.set(notified.get() + 1)
            })
            .detach()
        }
    });
    drop(hold);
    reading.shell.run(cx);
    assert_eq!(
        reading.images.seen("pic.png"),
        Some(Seen::Loaded(30., 300.))
    );
    assert!(notified.get() > 0, "the editor was not notified");
    assert!(reading.drawn_as_image(0, 2, cx), "the image is drawn");
    assert_eq!(reading.loads(), 1, "a hit loads nothing more");
}

/// An external `src`, another scheme, or a path the Parse rules refuse is
/// never looked up, though a local image of the same name is there: the
/// editor shows its source. A local `src` beside them loads.
#[gpui_kit::test]
fn an_https_src_is_never_loaded(cx: &mut TestAppContext) {
    let folder = Folder::new();
    folder.write("repo", "docs/pic.png", &png(30, 300));
    let remote = [
        "https://example.com/pic.png",
        "http://example.com/pic.png",
        "<HTTPS://example.com/pic.png> \"Title\"",
        "file:///docs/pic.png",
        "data:image/png;base64,iVBORw0KGgo=",
        "//example.com/pic.png",
        "../../pic.png",
        "C:/docs/pic.png",
        "sub\\pic.png",
    ];
    let mut body: String = remote
        .iter()
        .map(|src| format!("![a picture]({src})\n\n"))
        .collect();
    body.push_str("![a picture](pic.png)\n\nAfter.\n");
    let (reading, _) = Reading::new(&body, vec![folder.tree("repo", false)], "docs", false, cx);
    reading.shell.run(cx);
    let local = remote.len() * 2;
    for (at, src) in remote.iter().enumerate() {
        assert_eq!(reading.images.seen(src), Some(Seen::Failed), "{src}");
        assert!(
            !reading.drawn_as_image(at * 2, local + 2, cx),
            "{src} is drawn"
        );
    }
    assert_eq!(reading.loads(), 1, "only the local src was looked up");
    assert_eq!(
        reading.images.seen("pic.png"),
        Some(Seen::Loaded(30., 300.))
    );
    assert!(reading.drawn_as_image(local, local + 2, cx));
}

/// A path that is not there goes to the nearest image of its file name
/// below `docs/`: the item's own context first, then the fewest folder
/// steps. `.git` and `.manyhands` are skipped. The walk is the tab's: a
/// file added later is found only once the tab's images are cleared.
#[gpui_kit::test]
fn nearest_name_finds_an_image_in_docs(cx: &mut TestAppContext) {
    let folder = Folder::new();
    folder.write("repo", "docs/img/pic.png", &png(10, 300));
    folder.write("repo", "docs/far/deep/er/pic.png", &png(20, 300));
    folder.write("repo", "docs/guides/.manyhands/pic.png", &png(7, 300));
    folder.write("repo", "docs/guides/.git/pic.png", &png(8, 300));
    let body = "![a picture](missing/pic.png)\n\n![again](gone/pic.png)\n\nAfter.\n";
    let (reading, first) = Reading::new(
        body,
        vec![folder.tree("repo", false)],
        "docs/guides",
        true,
        cx,
    );
    let second = reading.shell.backend.hold_read(ReadKind::Image);
    drop(first);
    reading.shell.run(cx);
    assert_eq!(
        reading.images.seen("missing/pic.png"),
        Some(Seen::Loaded(10., 300.)),
        "two steps away, not four, and never the skipped folders"
    );
    assert!(reading.drawn_as_image(0, 4, cx));
    assert_eq!(reading.images.seen("gone/pic.png"), Some(Seen::Pending));

    // The second load looks in the walk the first made.
    folder.write("repo", "docs/guides/sub/pic.png", &png(40, 40));
    drop(second);
    reading.shell.run(cx);
    assert_eq!(
        reading.images.seen("gone/pic.png"),
        Some(Seen::Loaded(10., 300.)),
        "the tab's walk, from before the nearer file"
    );
    assert_eq!(reading.loads(), 2);

    cx.update(|cx| reading.images.clear(cx));
    reading.shell.run(cx);
    for src in ["missing/pic.png", "gone/pic.png"] {
        assert_eq!(
            reading.images.seen(src),
            Some(Seen::Loaded(40., 40.)),
            "after a clear, a new walk sees the nearer file"
        );
    }

    // The item's own context first, though its copy is farther.
    let (in_context, _) = Reading::new(
        "![a picture](missing/pic.png)\n\nAfter.\n",
        {
            folder.write("context", "docs/x/y/z/pic.png", &png(30, 5));
            vec![folder.tree("context", true), folder.tree("repo", false)]
        },
        "docs/guides",
        false,
        cx,
    );
    in_context.shell.run(cx);
    assert_eq!(
        in_context.images.seen("missing/pic.png"),
        Some(Seen::Loaded(30., 5.))
    );
}

/// A load still running when the tab's images are cleared lands on
/// nothing: the `src` is asked for again and loads the file as it is now.
#[gpui_kit::test]
fn a_load_from_before_a_clear_is_dropped(cx: &mut TestAppContext) {
    let folder = Folder::new();
    folder.write("repo", "docs/pic.png", &png(30, 90));
    let (reading, first) = Reading::new(
        "![a picture](pic.png)\n\nAfter.\n",
        vec![folder.tree("repo", false)],
        "docs",
        true,
        cx,
    );
    assert_eq!(reading.loads(), 1);
    folder.write("repo", "docs/pic.png", &png(50, 50));
    cx.update(|cx| reading.images.clear(cx));
    reading.shell.run(cx);
    assert_eq!(reading.images.seen("pic.png"), Some(Seen::Pending));
    assert_eq!(reading.loads(), 1, "one load at a time: the next waits");

    let second = reading.shell.backend.hold_read(ReadKind::Image);
    drop(first);
    reading.shell.run(cx);
    assert_eq!(
        reading.images.seen("pic.png"),
        Some(Seen::Pending),
        "the first load landed on nothing"
    );
    assert_eq!(reading.loads(), 2, "asked again after the clear");
    drop(second);
    reading.shell.run(cx);
    assert_eq!(reading.images.seen("pic.png"), Some(Seen::Loaded(50., 50.)));
}

/// R-39 in the tab: what it holds counts against its 256 MiB, as each
/// load starts. With a 3000-pixel image and a stand-in for all but 8192
/// pixels' worth held, a 9000-pixel image shows its source; once the tab
/// is cleared, it loads.
#[gpui_kit::test]
fn the_tabs_images_count_against_its_budget(cx: &mut TestAppContext) {
    let folder = Folder::new();
    folder.write("repo", "docs/small.png", &png(10, 300));
    folder.write("repo", "docs/large.png", &png(30, 300));
    let (reading, first) = Reading::new(
        "![small](small.png)\n\n![large](large.png)\n\nAfter.\n",
        vec![folder.tree("repo", false)],
        "docs",
        true,
        cx,
    );
    reading.images.hold_stand_in("held", 8192., 8191.);
    drop(first);
    reading.shell.run(cx);
    assert_eq!(
        reading.images.seen("small.png"),
        Some(Seen::Loaded(10., 300.))
    );
    assert!(reading.drawn_as_image(0, 4, cx));
    assert_eq!(reading.images.seen("large.png"), Some(Seen::Failed));
    assert!(!reading.drawn_as_image(2, 4, cx), "over the budget: source");

    cx.update(|cx| reading.images.clear(cx));
    reading.shell.run(cx);
    assert_eq!(
        reading.images.seen("large.png"),
        Some(Seen::Loaded(30., 300.))
    );
    assert_eq!(
        reading.images.seen("small.png"),
        Some(Seen::Loaded(10., 300.))
    );
}

/// Misses and refusals stay cached: a local `src` missing everywhere,
/// and one whose file containment refuses, are each looked up once, and
/// never again over later frames.
#[gpui_kit::test]
fn a_failed_src_is_never_looked_up_again(cx: &mut TestAppContext) {
    let folder = Folder::new();
    folder.write("repo", ".git/pic.png", &png(30, 300));
    let body = "![missing](nowhere.png)\n\n![refused](/.git/pic.png)\n\nAfter.\n";
    let (reading, first) = Reading::new(body, vec![folder.tree("repo", false)], "docs", true, cx);
    // The two loads go on, and a third, were there one, waits: a repeat
    // then shows as a count, and cannot loop, each landing drawing a frame
    // that would ask again.
    let second = reading.shell.backend.hold_read(ReadKind::Image);
    let _third = reading.shell.backend.hold_read(ReadKind::Image);
    drop((first, second));
    reading.shell.run(cx);
    for src in ["nowhere.png", "/.git/pic.png"] {
        assert_eq!(reading.images.seen(src), Some(Seen::Failed), "{src}");
    }
    assert_eq!(reading.loads(), 2, "each looked up once");

    for _ in 0..3 {
        reading.shell.frame(cx, |_, _| ());
        reading.shell.run(cx);
    }
    assert_eq!(reading.loads(), 2, "the later frames looked up nothing");
    for src in ["nowhere.png", "/.git/pic.png"] {
        assert_eq!(reading.images.seen(src), Some(Seen::Failed), "{src}");
    }
}

/// A new place to resolve from drops the images found in the old one,
/// from the window's atlas too, and reads them in the new place.
#[gpui_kit::test]
fn a_new_place_to_resolve_from_drops_the_old_images(cx: &mut TestAppContext) {
    let folder = Folder::new();
    folder.write("repo", "docs/pic.png", &png(30, 300));
    folder.write("context", "docs/pic.png", &png(20, 300));
    let (reading, _) = Reading::new(
        "![a picture](pic.png)\n\nAfter.\n",
        vec![folder.tree("repo", false)],
        "docs",
        false,
        cx,
    );
    reading.shell.run(cx);
    let old = reading.images.image("pic.png").expect("the image loaded");
    let drawn = |image: &Arc<RenderImage>, cx: &mut TestAppContext| {
        reading
            .shell
            .frame(cx, |window, _| window.has_image_atlas_entry(image))
    };
    assert!(drawn(&old, cx), "the drawn image is in the atlas");

    let moved = ImageSource {
        trees: vec![folder.tree("context", true), folder.tree("repo", false)],
        dir: "docs".to_owned(),
    };
    cx.update(|cx| reading.images.locate(moved, cx));
    reading.shell.run(cx);
    assert!(!drawn(&old, cx), "the old image's entry is gone");
    reading.shell.run(cx);
    assert_eq!(
        reading.images.seen("pic.png"),
        Some(Seen::Loaded(20., 300.))
    );
}

/// The tab's reload clears its images: a file changed since is loaded
/// again, as it is now.
#[gpui_kit::test]
fn the_cache_clears_on_reload(cx: &mut TestAppContext) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let source = format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: {DOCUMENT}\ntitle: Pictures\n---\n![a picture](pic.png)\n\nAfter.\n"
    );
    fixture.commit_file(&root, "docs/a.md", source.as_bytes());
    fixture.commit_file(&root, "docs/pic.png", &png(40, 200));
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    let target = OpenTarget::Item {
        repo: key(&root),
        kind: ItemDtoKind::Document,
        id: item_id(DOCUMENT),
    };
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
    let pane = pane(&shell, cx);
    let images = pane.read_with(cx, |pane, _| pane.images().clone());
    assert_eq!(images.seen("pic.png"), Some(Seen::Loaded(40., 200.)));

    fs::write(root.join("docs/pic.png"), png(200, 40)).unwrap();
    shell.frame(cx, |_, _| ());
    shell.run(cx);
    assert_eq!(
        images.seen("pic.png"),
        Some(Seen::Loaded(40., 200.)),
        "kept until the tab reloads"
    );
    pane.update(cx, |pane, cx| {
        pane.reload(cx);
    });
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    shell.run(cx);
    assert_eq!(images.seen("pic.png"), Some(Seen::Loaded(200., 40.)));
    assert_eq!(shell.backend.reads_seen(ReadKind::Image), 2);
}

fn pane(shell: &Shell, cx: &mut TestAppContext) -> Entity<ItemPane> {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.item_pane())
        .expect("an item tab is shown")
}
