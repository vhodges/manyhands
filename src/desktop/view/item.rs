//! The read-only item view: state, construction and composition.
//!
//! An item tab is a column (R-47): the tab strip, which the workspace draws,
//! then a header placeholder holding D1's identity line, which stays in
//! place, then the item's own scroll region with the page and the
//! discussion (`page`). The region ends with the workspace's details of
//! the selected repository, which D1 drew at the end of the content
//! column (owner, 2026-10-27).
//!
//! `reads` owns the ordered per-tab read and refresh job, `status` its loading
//! and recovery controls, `details` the identity/provenance/metadata, `source`
//! the retained read-only textarea, `discussion` the comment thread, and
//! `draft` the banner of a stored draft and the section of one restored here,
//! with `draft_actions` its Export… and Discard….
//! Relationships remain in the sibling view. All children use the same entity,
//! stamped model and subscriptions; none writes the item.

mod details;
mod discussion;
mod draft;
mod draft_actions;
mod page;
mod reads;
mod source;
mod status;
pub(crate) mod tokens;

use std::{collections::HashMap, rc::Rc};

use gpui_kit::{
    AnyElement, App, Context, Entity, FocusHandle, InteractiveElement, IntoElement, ParentElement,
    Render, Styled, Subscription, Task, TestSupportExt, Window,
    component::{ActiveTheme, input::TextareaState, v_flex},
    div, px,
};

use super::{relationships::RowKey, scroll::ContentScroll};
use crate::desktop::{
    model::{item::ItemView, names::TicketName, navigation::OpenTarget, stamp::ViewScope},
    services::{Services, requests::DesktopRequestId},
};
pub(super) use details::text;
#[cfg(test)]
pub(crate) use draft::NOT_SAVED;
pub(crate) use draft::{EDITOR_LATER, NOT_DISCARDED, ReviewInDrafts, refused_label};
pub(crate) use draft_actions::DraftDiscarded;

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
/// The discussion header's way back to the page (`item_view/04-comments.md`).
pub(crate) const BACK_TO_TOP: &str = "Back to top";
const COMMENT_INDENT: f32 = 16.;
/// The header block's padding, vertical and horizontal
/// (`item_view/01-item-frame.md`).
const HEADER_PADDING: (f32, f32) = (14., 28.);

/// Builds what ends the item's scroll region, after the discussion: the
/// workspace's details of the selected repository, revealed by the region
/// it is given. It is built as the pane is drawn, so it is always the
/// frame's own.
pub(crate) type RegionEnd = Rc<dyn Fn(&ContentScroll, &mut Window, &mut App) -> Option<AnyElement>>;

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
    /// The item's own scroll region, which the page keys move while this
    /// tab is shown, and where its controls are revealed.
    pub(super) scroll: ContentScroll,
    page: page::Page,
    /// The discussion's Back to top, a tab stop.
    back_to_top: FocusHandle,
    /// Set by the workspace when it opens the tab.
    region_end: Option<RegionEnd>,
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
            scroll: scroll.region(),
            page: page::Page::new(cx),
            back_to_top: cx.focus_handle().tab_stop(true),
            region_end: None,
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

    /// The vertical offset of the item's scroll region.
    #[cfg(test)]
    pub(crate) fn region_offset(&self) -> gpui_kit::Pixels {
        self.scroll.handle().offset().y
    }

    /// The item's scroll region, which the page keys move while this tab
    /// is shown.
    pub(super) fn region(&self) -> &ContentScroll {
        &self.scroll
    }

    /// What ends the item's scroll region, after the discussion.
    pub(super) fn end_region_with(&mut self, end: RegionEnd) {
        self.region_end = Some(end);
    }

    /// Back to top: the region at its top, and the page focused.
    fn back_to_top(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.scroll.to_top();
        window.focus(self.page.focus_handle(), cx);
        cx.notify();
    }

    pub(super) fn repo(&self) -> &str {
        match self.model.target() {
            OpenTarget::Item { repo, .. } | OpenTarget::Inspect { repo, .. } => repo,
        }
    }

    /// The window's draft registry, for the tab strip.
    pub(super) fn drafts(&self) -> &Entity<crate::desktop::services::drafts::DraftRegistry> {
        self.services.drafts()
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
        let section = self.draft_section(window, cx);
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
        // D1's content, on the page until Tasks 5b, 6 and 7 move it.
        let content = v_flex()
            .gap_3()
            .child(status)
            .children(not_refreshed)
            .children(banner)
            .children(section)
            .children(body)
            .child(self.later(cx));
        let page = self.page.render(content, &self.scroll, window, cx);
        let discussion = self.discussion(window, cx);
        let end = self
            .region_end
            .clone()
            .and_then(|end| end(&self.scroll, window, cx));
        let (vertical, horizontal) = HEADER_PADDING;
        let header = div()
            .id("item-header")
            .test_support()
            .flex_none()
            .w_full()
            .py(px(vertical))
            .px(px(horizontal))
            .border_b_1()
            .border_color(cx.theme().border)
            .child(identity);
        // No zero minimum: the pane is never shorter than its header, so a
        // window too short for the header scrolls the content column.
        v_flex()
            .id("item-view")
            .test_support()
            .flex_1()
            .w_full()
            .child(header)
            .child(page::region(&self.scroll, page, discussion, end, cx))
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "item_tests.rs"]
mod tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "item/frame_tests.rs"]
mod frame_tests;
