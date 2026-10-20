//! The content region: what an activated row opened.

use gpui_kit::{AppContext, Context, Entity};

use super::{item::ItemPane, workspace::Workspace};
use crate::desktop::model::{
    navigation::OpenTarget,
    stamp::{TabId, ViewScope},
};

/// The tab the single content item is stamped as, until tabs open.
const CONTENT_TAB: TabId = TabId(0);

impl Workspace {
    /// The item view in the content region.
    #[cfg(test)]
    pub(crate) fn item_pane(&self) -> Option<Entity<ItemPane>> {
        self.item.clone()
    }

    /// What the last activated row opened.
    #[cfg(test)]
    pub(crate) fn opened(&self, cx: &gpui_kit::App) -> Option<OpenTarget> {
        self.item
            .as_ref()
            .map(|item| item.read(cx).target().clone())
    }

    /// Opens `target` in the content region, in place of what was open,
    /// and reads it.
    pub(crate) fn open_item(&mut self, target: OpenTarget, cx: &mut Context<Self>) {
        let services = self.services.clone();
        self.item =
            Some(cx.new(|cx| ItemPane::new(target, ViewScope::Tab(CONTENT_TAB), services, cx)));
        cx.notify();
    }

    /// The content shown: the open item.
    pub(super) fn content_item(&self) -> Option<Entity<ItemPane>> {
        self.item.clone()
    }
}
