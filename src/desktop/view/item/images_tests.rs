//! Local images in an item tab's editors (design, Images; decision 9),
//! over real repositories: the read editor draws an image's source while
//! it loads and the image once it lands, an edit host given the tab's
//! images shares what the tab already loaded for a comment, and a reload
//! or the tab's close gives its images back to the window's atlas.

use std::{cell::RefCell, io::Cursor, path::Path, rc::Rc, sync::Arc};

use gpui_kit::{AppContext, Entity, RenderImage, TestAppContext, component::Root};
use image::{DynamicImage, ImageFormat, Rgb, RgbImage};
use manyhands::repository::ItemDtoKind;
use zorite_editor::EditorState;

use super::ItemPane;
use crate::desktop::{
    model::navigation::OpenTarget,
    services::{ReadKind, images::Seen},
    test_support::fixtures::{Fixture, item_id},
    view::{
        editor::host::EditorHost,
        harness::{Shell, key},
    },
};

const DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FP0";
const COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FP1";

/// The body's image is its third line, the text after it its fifth.
const PICTURED: &str = "Before the picture.\n\n![a picture](pic.png)\n\nAfter the picture.\n";
const IMAGE_ROW: usize = 2;
const TEXT_ROW: usize = 4;

/// A `width` by `height` PNG.
fn png(width: u32, height: u32) -> Vec<u8> {
    let image = RgbImage::from_pixel(width, height, Rgb([10, 20, 30]));
    let mut bytes = Vec::new();
    DynamicImage::ImageRgb8(image)
        .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
        .unwrap();
    bytes
}

/// Commits `DOCUMENT` at `docs/a.md` with `body`, and `docs/pic.png`.
fn commit_pictured(fixture: &Fixture, root: &Path, body: &str) {
    let source = format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: {DOCUMENT}\ntitle: Pictures\n---\n{body}"
    );
    fixture.commit_file(root, "docs/a.md", source.as_bytes());
    fixture.commit_file(root, "docs/pic.png", &png(40, 200));
}

/// One repository, `a`, built by `build` and refreshed, in an open
/// window, with `DOCUMENT` shown in its tab once its reads land. The
/// `hold`, when asked for, holds the tab's first image load.
fn opened(
    cx: &mut TestAppContext,
    hold: bool,
    build: impl FnOnce(&Fixture, &Path),
) -> (Shell, Option<futures::channel::oneshot::Sender<()>>) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    build(&fixture, &root);
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    let held = hold.then(|| shell.backend.hold_read(ReadKind::Image));
    let target = OpenTarget::Item {
        repo: key(&root),
        kind: ItemDtoKind::Document,
        id: item_id(DOCUMENT),
    };
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
    (shell, held)
}

fn pane(shell: &Shell, cx: &mut TestAppContext) -> Entity<ItemPane> {
    shell
        .workspace
        .read_with(cx, |workspace, _| workspace.item_pane())
        .expect("an item tab is shown")
}

/// Whether `editor`'s last paint drew `row` as an image: more than three
/// times as tall as the text row `text`.
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

/// While its load is held, the read editor shows the image's source; once
/// it lands, the image. The text is unchanged.
#[gpui_kit::test]
fn the_read_editor_shows_the_image_once_loaded(cx: &mut TestAppContext) {
    let (shell, hold) = opened(cx, true, |fixture, root| {
        commit_pictured(fixture, root, PICTURED)
    });
    shell.frame(cx, |_, _| ());
    let pane = pane(&shell, cx);
    let editor = pane
        .read_with(cx, |pane, cx| pane.read_editor(cx))
        .expect("the read view is shown");
    let images = pane.read_with(cx, |pane, _| pane.images().clone());
    assert_eq!(images.seen("pic.png"), Some(Seen::Pending));
    assert!(
        !drawn_as_image(&editor, IMAGE_ROW, TEXT_ROW, cx),
        "its source shows while it loads"
    );

    drop(hold);
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    assert_eq!(images.seen("pic.png"), Some(Seen::Loaded(40., 200.)));
    assert!(
        drawn_as_image(&editor, IMAGE_ROW, TEXT_ROW, cx),
        "the image shows once loaded"
    );
    let text = editor.read_with(cx, |editor, _| editor.text().to_owned());
    assert_eq!(text, PICTURED);
}

/// A comment's image loads through the tab. An edit host given the tab's
/// images draws that image at its first frame, with no second load, and
/// its own new `src` loads through the same tab.
#[gpui_kit::test]
fn an_edit_host_and_a_comment_share_the_tabs_cache(cx: &mut TestAppContext) {
    let (shell, _) = opened(cx, false, |fixture, root| {
        commit_pictured(fixture, root, "A body with no picture.\n");
        let comment = format!(
            "---\nmanyhands_managed: true\nmanyhands_kind: comment\nid: {COMMENT}\nitem_id: {DOCUMENT}\n\
             created_at: 2026-09-30T12:00:00Z\n---\n![a picture](pic.png)\n\nAfter the picture.\n"
        );
        fixture.commit_file(
            root,
            &format!(".manyhands/comments/{DOCUMENT}/{COMMENT}.md"),
            comment.as_bytes(),
        );
        fixture.commit_file(root, "docs/other.png", &png(60, 240));
    });
    shell.frame(cx, |_, _| ());
    shell.run(cx);
    let pane = pane(&shell, cx);
    let images = pane.read_with(cx, |pane, _| pane.images().clone());
    let comment = pane
        .read_with(cx, |pane, _| pane.comment_view(COMMENT))
        .expect("the comment has a read editor");
    let comment = comment.read_with(cx, |view, _| view.editor().clone());
    assert_eq!(images.seen("pic.png"), Some(Seen::Loaded(40., 200.)));
    assert!(drawn_as_image(&comment, 0, 2, cx), "the comment's image");
    assert_eq!(shell.backend.reads_seen(ReadKind::Image), 1);

    let made = Rc::new(RefCell::new(None));
    cx.add_window({
        let (made, images) = (made.clone(), images.clone());
        move |window, cx| {
            let host = cx.new(|cx| {
                EditorHost::new(
                    "![a picture](pic.png)\n\n![another](other.png)\n\nAfter.\n",
                    None,
                    window,
                    cx,
                )
            });
            host.update(cx, |host, cx| host.show_images(&images, cx));
            *made.borrow_mut() = Some(host.clone());
            Root::new(host, window, cx)
        }
    });
    let host: Entity<EditorHost> = made.borrow_mut().take().unwrap();
    let editor = host.read_with(cx, |host, _| host.editor().clone());
    assert!(
        drawn_as_image(&editor, 0, 4, cx),
        "the tab's image at the host's first frame"
    );
    assert_eq!(images.seen("other.png"), Some(Seen::Pending));
    assert_eq!(
        shell.backend.reads_seen(ReadKind::Image),
        2,
        "only the host's new src was loaded"
    );

    shell.run(cx);
    assert_eq!(images.seen("other.png"), Some(Seen::Loaded(60., 240.)));
    assert!(drawn_as_image(&editor, 2, 4, cx), "the host's own image");
    assert!(drawn_as_image(&comment, 0, 2, cx), "the comment's is kept");
}

/// Whether every frame of `image` is in the window's sprite atlas, after
/// a frame.
fn in_atlas(shell: &Shell, image: &Arc<RenderImage>, cx: &mut TestAppContext) -> bool {
    shell.frame(cx, |window, _| window.has_image_atlas_entry(image))
}

/// A painted image stays in the window's atlas until it is dropped from
/// it. A reload drops the tab's old images: the one drawn before has an
/// entry, and after the reload only the image read again, drawn in its
/// place, has one.
#[gpui_kit::test]
fn a_reload_drops_the_old_images_from_the_atlas(cx: &mut TestAppContext) {
    let (shell, _) = opened(cx, false, |fixture, root| {
        commit_pictured(fixture, root, PICTURED)
    });
    shell.frame(cx, |_, _| ());
    let pane = pane(&shell, cx);
    let images = pane.read_with(cx, |pane, _| pane.images().clone());
    let old = images.image("pic.png").expect("the image loaded");
    assert!(
        in_atlas(&shell, &old, cx),
        "the drawn image is in the atlas"
    );

    pane.update(cx, |pane, cx| {
        pane.reload(cx);
    });
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    shell.run(cx);
    let new = images.image("pic.png").expect("the image is read again");
    assert!(!Arc::ptr_eq(&old, &new), "a new image");
    assert!(!in_atlas(&shell, &old, cx), "the old image's entry is gone");
    assert!(in_atlas(&shell, &new, cx), "the new one is drawn");
}

/// Closing the tab drops its images, the body's and a comment's, from the
/// window's atlas.
#[gpui_kit::test]
fn closing_the_tab_drops_its_images_from_the_atlas(cx: &mut TestAppContext) {
    let (shell, _) = opened(cx, false, |fixture, root| {
        commit_pictured(fixture, root, PICTURED);
        let comment = format!(
            "---\nmanyhands_managed: true\nmanyhands_kind: comment\nid: {COMMENT}\nitem_id: {DOCUMENT}\n\
             created_at: 2026-09-30T12:00:00Z\n---\n![another](other.png)\n\nAfter the picture.\n"
        );
        fixture.commit_file(
            root,
            &format!(".manyhands/comments/{DOCUMENT}/{COMMENT}.md"),
            comment.as_bytes(),
        );
        fixture.commit_file(root, "docs/other.png", &png(60, 240));
    });
    shell.frame(cx, |_, _| ());
    shell.run(cx);
    let images = pane(&shell, cx).read_with(cx, |pane, _| pane.images().clone());
    let drawn = [
        images.image("pic.png").expect("the body's image loaded"),
        images
            .image("other.png")
            .expect("the comment's image loaded"),
    ];
    for image in &drawn {
        assert!(in_atlas(&shell, image, cx), "drawn before the close");
    }

    shell.keys("secondary-w", cx);
    let open = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.item_pane().is_some());
    assert!(!open, "the tab closed");
    shell.run(cx);
    for image in &drawn {
        assert!(!in_atlas(&shell, image, cx), "its entry is gone");
    }
}
