//! The item view's icons load from the desktop asset source.

use gpui_kit::AssetSource;

use super::{APP_ICON_PATH, DesktopAssets, icon_path};

/// The item view's Lucide icons (design R-52).
const ICONS: [&str; 37] = [
    "folder-git-2",
    "ticket",
    "file-text",
    "sliders-horizontal",
    "circle-check",
    "pencil",
    "eye",
    "save",
    "cloud-upload",
    "message-square",
    "undo-2",
    "redo-2",
    "pilcrow",
    "bold",
    "italic",
    "strikethrough",
    "code",
    "link",
    "list",
    "list-ordered",
    "list-todo",
    "text-quote",
    "square-code",
    "table",
    "shield-check",
    "circle-alert",
    "file-code",
    "calendar",
    "folder-kanban",
    "users",
    "close",
    "copy",
    "cloud-off",
    "arrow-up",
    "arrow-down",
    "archive",
    "activity",
];

#[test]
fn every_item_view_icon_loads() {
    for name in ICONS {
        let svg = DesktopAssets
            .load(&icon_path(name))
            .unwrap_or_else(|_| panic!("{name} errored"))
            .unwrap_or_else(|| panic!("{name} is missing"));
        assert!(svg.starts_with(b"<svg"), "{name} is not an SVG");
    }
}

#[test]
fn the_kit_icons_still_load_and_an_unknown_path_is_not_an_icon() {
    assert!(
        DesktopAssets
            .load("icons/chevron-down.svg")
            .unwrap()
            .is_some()
    );
    assert!(DesktopAssets.load("icons/no-such-icon.svg").is_err());
}

#[test]
fn the_app_icon_loads_as_a_small_png() {
    let png = DesktopAssets.load(APP_ICON_PATH).unwrap().unwrap();
    assert!(png.starts_with(b"\x89PNG"), "the app icon is not a PNG");
    assert!(png.len() < 16 * 1024, "the app icon is not small");
}
