//! The desktop asset source: the item view's Lucide icons, asked first,
//! and the Kit's default `Assets` as the fallback (facts 69 and 80).
//!
//! The Kit's `Assets::load` errors on an unknown path, so the added icons
//! must be asked before it. The SVGs are the Kit's own bundled Lucide
//! catalog (`gpui-kit-assets`); nothing is vendored in this repository.

use std::borrow::Cow;

use gpui_kit::{
    AssetSource, SharedString,
    assets::{Assets, icon_assets},
};

icon_assets!(
    ItemIcons,
    [
        FolderGit2,
        Ticket,
        FileText,
        SlidersHorizontal,
        CircleCheck,
        Pencil,
        Eye,
        Save,
        CloudUpload,
        MessageSquare,
        Undo2,
        Redo2,
        Pilcrow,
        Bold,
        Italic,
        Strikethrough,
        Code,
        Link,
        List,
        ListOrdered,
        ListTodo,
        TextQuote,
        SquareCode,
        Table,
        ShieldCheck,
        CircleAlert,
        FileCode,
        Calendar,
        FolderKanban,
        Users,
        Close,
        Copy,
        CloudOff,
        ArrowUp,
        ArrowDown,
        Archive,
    ]
);

/// The item view's icons first, then the Kit's default icons.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct DesktopAssets;

/// The path of the icon `name`, as the Kit's `Icon::path` takes it.
pub(crate) fn icon_path(name: &str) -> String {
    format!("icons/{name}.svg")
}

impl AssetSource for DesktopAssets {
    fn load(&self, path: &str) -> gpui_kit::Result<Option<Cow<'static, [u8]>>> {
        if let Some(icon) = ItemIcons.load(path)? {
            return Ok(Some(icon));
        }
        Assets.load(path)
    }

    fn list(&self, path: &str) -> gpui_kit::Result<Vec<SharedString>> {
        let mut paths = ItemIcons.list(path)?;
        for kit in Assets.list(path)? {
            if !paths.contains(&kit) {
                paths.push(kit);
            }
        }
        Ok(paths)
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "assets_tests.rs"]
mod tests;
