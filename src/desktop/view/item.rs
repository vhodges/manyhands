//! The read-only item view: state, construction and composition.
//!
//! `reads` owns the ordered per-tab read and refresh job, `status` its loading
//! and recovery controls, `details` the identity/provenance/metadata, `source`
//! the retained read-only textarea, `discussion` the comment thread, and
//! `draft` the banner of a stored draft and the section of one restored here.
//! Relationships remain in the sibling view. All children use the same entity,
//! stamped model and subscriptions; none writes the item.

mod details;
mod discussion;
mod draft;
mod reads;
mod source;
mod status;

use std::collections::HashMap;

use gpui_kit::{
    Context, Entity, FocusHandle, InteractiveElement, IntoElement, ParentElement, Render, Styled,
    Subscription, Task, TestSupportExt, Window,
    component::{input::TextareaState, v_flex},
};

use super::{relationships::RowKey, scroll::ContentScroll};
use crate::desktop::{
    model::{item::ItemView, names::TicketName, navigation::OpenTarget, stamp::ViewScope},
    services::{Services, requests::DesktopRequestId},
};
pub(super) use details::text;
pub(crate) use draft::EDITOR_LATER;
#[cfg(test)]
pub(crate) use draft::NOT_SAVED;

pub(crate) const NONCONFORMING_FILE: &str = "Nonconforming file";
pub(crate) const SOURCE_NOT_UTF8: &str =
    "This file is not valid UTF-8, so its text cannot be shown.";
pub(crate) const REFRESH: &str = "Refresh";
pub(crate) const SOURCE: &str = "Source";
pub(crate) const REFRESHING: &str = "Refreshing the index";
pub(crate) const RECOVERY: &str = "This item's context needs repair before it can be read from it.";
pub(crate) const COPY_ID: &str = "Copy ID";
pub(crate) const COPY_SLUG: &str = "Copy short code";
pub(crate) const NOT_REFRESHED: &str = "Could not refresh";
const COMMENT_INDENT: f32 = 16.;

pub(crate) struct ItemPane {
    pub(super) model: ItemView,
    services: Services,
    /// Replacing the one stamped read job drops the previous task.
    read: Option<Task<()>>,
    /// Relationship focus follows row identity across reorders.
    pub(super) row_focus: HashMap<RowKey, FocusHandle>,
    copy_focus: [FocusHandle; 2],
    refreshing: Option<DesktopRequestId>,
    /// Last source plus the same textarea entity, preserving selection/focus.
    source: Option<(String, Entity<TextareaState>)>,
    pub(super) scroll: ContentScroll,
    draft: draft::TabDraft,
    _operations: Subscription,
    /// A draft listed, restored or written redraws the banner and section.
    _drafts: Subscription,
}

impl ItemPane {
    pub(crate) fn new(
        target: OpenTarget,
        scope: ViewScope,
        services: Services,
        scroll: ContentScroll,
        cx: &mut Context<Self>,
    ) -> Self {
        let operations = cx.observe(services.operations(), |this, _, cx| {
            this.operations_changed(cx)
        });
        let drafts = cx.observe(services.drafts(), |_, _, cx| cx.notify());
        let mut pane = Self {
            model: ItemView::new(target, scope),
            services,
            read: None,
            row_focus: HashMap::new(),
            copy_focus: [cx.focus_handle(), cx.focus_handle()],
            refreshing: None,
            source: None,
            scroll,
            draft: draft::TabDraft::default(),
            _operations: operations,
            _drafts: drafts,
        };
        let stamp = pane.model.load();
        pane.read(stamp, cx);
        pane
    }

    #[cfg(test)]
    pub(crate) fn target(&self) -> &OpenTarget {
        self.model.target()
    }

    pub(super) fn repo(&self) -> &str {
        match self.model.target() {
            OpenTarget::Item { repo, .. } | OpenTarget::Inspect { repo, .. } => repo,
        }
    }

    /// Keep the last successful title after a failed refresh (ruling R-B5).
    pub(crate) fn tab_title(&self) -> String {
        let fallback = match self.model.target() {
            OpenTarget::Item { id, .. } => id.to_string(),
            OpenTarget::Inspect { path, .. } => path.clone(),
        };
        TicketName {
            slug: None,
            title: self.model.last_title(),
        }
        .title_or(&fallback)
        .to_owned()
    }
}

impl Render for ItemPane {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_source(window, cx);
        let item = self.model.item().rows().cloned();
        let identity = self.identity(item.as_ref(), cx);
        let status = self.status("item", self.model.item(), cx);
        let not_refreshed = self.not_refreshed(cx);
        let banner = self.draft_banner(item.as_ref(), cx);
        let section = self.draft_section(cx);
        let relationships = match &item {
            Some(_) => self.relationships_panel(window, cx),
            None => None,
        };
        if relationships.is_none() {
            self.row_focus.clear();
        }
        let body = item.as_ref().map(|item| {
            v_flex()
                .gap_3()
                .child(self.provenance(item, cx))
                .child(self.fields(item, cx))
                .children(relationships)
                .child(self.problems(item, cx))
                .child(self.metadata(item, cx))
                .child(self.source(item, cx))
        });
        let discussion = self.discussion(cx);
        v_flex()
            .id("item-view")
            .test_support()
            .gap_3()
            .child(identity)
            .child(status)
            .children(not_refreshed)
            .children(banner)
            .children(section)
            .children(body)
            .children(discussion)
            .child(self.later(cx))
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "item_tests.rs"]
mod tests;
