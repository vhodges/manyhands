//! The item view's icons load from the desktop asset source.

use gpui_kit::AssetSource;

use super::{DesktopAssets, icon_path};

/// The item view's Lucide icons (design R-52).
const ICONS: [&str; 36] = [
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
