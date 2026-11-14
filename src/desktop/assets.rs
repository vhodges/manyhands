//! The desktop asset source: the item view's Lucide icons and the app icon,
//! asked first, and the Kit's default `Assets` as the fallback (facts 69
//! and 80).
//!
//! The Kit's `Assets::load` errors on an unknown path, so the added icons
//! must be asked before it. The SVGs are the Kit's own bundled Lucide
//! catalog (`gpui-kit-assets`); nothing is vendored in this repository except
//! the app icon, a small PNG made from `packaging/icons/manyhands-1024.png`.

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
        Activity,
        Repeat,
        Bug,
        Sparkles,
        ListChecks,
        FilePlus,
        FolderPlus,
        ChevronsDownUp,
        ArrowUpDown,
        ListFilter,
        FileExclamationPoint,
        Lock,
        RefreshCw,
        WifiOff,
    ]
);

/// Where the title bar asks for the app icon (R-16).
pub(crate) const APP_ICON_PATH: &str = "brand/manyhands.png";

const APP_ICON: &[u8] = include_bytes!("brand/manyhands.png");

/// The item view's icons and the app icon first, then the Kit's default
/// icons.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct DesktopAssets;

/// The path of the icon `name`, as the Kit's `Icon::path` takes it.
pub(crate) fn icon_path(name: &str) -> String {
    format!("icons/{name}.svg")
}

impl AssetSource for DesktopAssets {
    fn load(&self, path: &str) -> gpui_kit::Result<Option<Cow<'static, [u8]>>> {
        if path == APP_ICON_PATH {
            return Ok(Some(Cow::Borrowed(APP_ICON)));
        }
        if let Some(icon) = ItemIcons.load(path)? {
            return Ok(Some(icon));
        }
        Assets.load(path)
    }

    fn list(&self, path: &str) -> gpui_kit::Result<Vec<SharedString>> {
        let mut paths = ItemIcons.list(path)?;
        if APP_ICON_PATH.starts_with(path) {
            paths.push(APP_ICON_PATH.into());
        }
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
