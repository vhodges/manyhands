//! The read-only item view: state, construction and composition.
//!
//! An item tab is a column (R-47): the tab strip, which the workspace draws,
//! then the header block (`header`), which stays in place, then the item's
//! own scroll region with the page and the discussion (`page`). The region
//! ends with the workspace's details of the selected repository, which D1
//! drew at the end of the content column (owner, 2026-10-27). While
//! viewing, the page holds the body in a read editor (decision 17).
//!
//! The header holds the breadcrumb, the title, the `chips`, the actions
//! with the state `pill`, and the `banners`, where D1's status lines now
//! show, and where a link clicked in the tab's editors is followed. A chip
//! opens the details `sheet`, which holds D1's identity details,
//! provenance, fields, metadata, problems and, from the sibling view,
//! relationships; a file read by its path has no sheet, so its page
//! keeps D1's `details`. `reads` owns the ordered per-tab read and refresh
//! job, `status` D1's loading and recovery controls, `source` the
//! read-only textarea of a file read by its path (an Inspect tab, which
//! the sheet's Inspect link opens: R-64), `discussion` the threads, with a
//! read editor per comment body, the jump to them and the floating button
//! over the region, and `draft` the banner of a stored draft and the
//! section of one restored here, with `draft_actions` its Export… and
//! Discard…. All children use the same entity, stamped model and
//! subscriptions; none writes the item. Every editor of the tab draws the
//! local images its text names from the tab's own `TabImages`
//! (`services::images`), which a reload clears.
//!
//! `edit` makes the tab edit (design, Part C): Edit prepares the item's
//! private work context, the edit editor and its toolbar replace the read
//! view, D2's draft follows the text, Save, or `secondary-s`, saves it,
//! and Done ends the session or keeps it while the tab views the saved
//! copy.

mod banners;
mod chips;
mod details;
mod discussion;
mod draft;
mod draft_actions;
mod edit;
mod header;
mod page;
mod pill;
mod reads;
mod sheet;
mod source;
mod status;
pub(crate) mod tokens;

use std::{collections::HashMap, rc::Rc};

use gpui_kit::{
    AnyElement, App, Context, Entity, FocusHandle, InteractiveElement, IntoElement, ParentElement,
    Render, Styled, Subscription, Task, TestSupportExt, Window,
    component::{input::TextareaState, v_flex},
};

use super::{relationships::RowKey, scroll::ContentScroll};
use crate::desktop::{
    keys::{Edit, ITEM_TAB_CONTEXT, Save, ToggleRich},
    model::{
        edit::Phase, item::ItemView, names::TicketName, navigation::OpenTarget, stamp::ViewScope,
    },
    services::{
        Services,
        drafts::{DraftSaved, SessionDiscarded},
        images::TabImages,
        requests::DesktopRequestId,
    },
};
#[cfg(test)]
pub(crate) use banners::{LINK_NOT_FOUND, LINK_NOT_OPENED, NEAREST_MATCH};
pub(crate) use chips::{ChipField, OpenDetails};
pub(super) use details::text;
#[cfg(test)]
pub(crate) use draft::NOT_SAVED;
pub(crate) use draft::{EDITOR_LATER, NOT_DISCARDED, ReviewInDrafts, refused_label};
pub(crate) use draft_actions::DraftDiscarded;
pub(crate) use edit::{CloseThisTab, Leaving};
pub(crate) use sheet::open_details;

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
    /// One per chip field, so focus can return to the chip that asked for
    /// the details sheet.
    chip_focus: Vec<FocusHandle>,
    /// One per chip field: the sheet's field each chip opens it on.
    sheet_focus: Vec<FocusHandle>,
    /// Around the sheet's Inspect link, whose own focus is the Kit's.
    link_focus: FocusHandle,
    /// The details sheet's own scroll, where its focused controls are
    /// revealed.
    pub(super) sheet_scroll: ContentScroll,
    refreshing: Option<DesktopRequestId>,
    /// An Inspect tab's last source plus the same textarea entity,
    /// preserving selection/focus.
    source: Option<(String, Entity<TextareaState>)>,
    /// The item's own scroll region, which the page keys move while this
    /// tab is shown, and where its controls are revealed.
    pub(super) scroll: ContentScroll,
    page: page::Page,
    /// The discussion's Back to top, a tab stop.
    back_to_top: FocusHandle,
    /// The comments' read editors, the discussion's heading and the
    /// header's Comments {n}.
    discussion: discussion::Discussion,
    /// Set by the workspace when it opens the tab.
    region_end: Option<RegionEnd>,
    draft: draft::TabDraft,
    /// What the last link clicked here came to, when it opened nothing
    /// or opened the nearest match; the next link clears it.
    link_note: Option<banners::LinkNote>,
    /// The last link's resolution; a later link drops it.
    link_read: Option<Task<()>>,
    /// The local images every editor of the tab draws from (decision 9),
    /// cleared on reload.
    images: TabImages,
    /// Edit: the phase, the session and what stopped a prepare.
    edit: edit::TabEdit,
    _operations: Subscription,
    /// A draft listed, restored or written redraws the banner and section.
    _drafts: Subscription,
    /// A save of this tab's session committed (R-69).
    _saved: Subscription,
    /// A discard elsewhere ended this tab's session (R-71).
    _discarded: Subscription,
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
        let drafts = cx.observe(services.drafts(), |pane, _, cx| {
            pane.sync_status(cx);
            pane.follow_registry(cx);
            pane.follow_done(cx);
            cx.notify()
        });
        let saved = cx.subscribe(services.drafts(), |pane, _, event: &DraftSaved, cx| {
            pane.draft_saved(event, cx)
        });
        let discarded = cx.subscribe(
            services.drafts(),
            |pane, _, event: &SessionDiscarded, cx| pane.session_discarded(event, cx),
        );
        let images = TabImages::new(&services);
        // The tab's images leave the windows' atlases with it.
        cx.on_release(|pane, cx| pane.images.close(cx)).detach();
        let edit = edit::TabEdit::new(scope.clone(), cx);
        let mut pane = Self {
            model: ItemView::new(target, scope),
            services,
            read: None,
            row_focus: HashMap::new(),
            copy_focus: [cx.focus_handle(), cx.focus_handle()],
            chip_focus: chips::chip_focus(cx),
            sheet_focus: sheet::sheet_focus(cx),
            link_focus: cx.focus_handle(),
            sheet_scroll: scroll.region(),
            refreshing: None,
            source: None,
            scroll: scroll.region(),
            page: page::Page::new(cx),
            back_to_top: cx.focus_handle().tab_stop(true),
            discussion: discussion::Discussion::new(cx),
            region_end: None,
            draft: draft::TabDraft::default(),
            link_note: None,
            link_read: None,
            images,
            edit,
            _operations: operations,
            _drafts: drafts,
            _saved: saved,
            _discarded: discarded,
        };
        pane.watch_editing(cx);
        let stamp = pane.model.load();
        pane.read(stamp, cx);
        pane
    }

    #[cfg(test)]
    pub(crate) fn target(&self) -> &OpenTarget {
        self.model.target()
    }

    /// The read view of an item tab, once a read gave a body.
    #[cfg(test)]
    pub(crate) fn read_view(&self) -> Option<Entity<super::editor::read_only::ReadOnly>> {
        self.page.read().cloned()
    }

    /// The read view's editor.
    #[cfg(test)]
    pub(crate) fn read_editor(&self, cx: &App) -> Option<Entity<zorite_editor::EditorState>> {
        Some(self.page.read()?.read(cx).editor().clone())
    }

    /// The read view of the comment `id`'s body.
    #[cfg(test)]
    pub(crate) fn comment_view(
        &self,
        id: &str,
    ) -> Option<Entity<super::editor::read_only::ReadOnly>> {
        self.discussion.body(id).cloned()
    }

    /// The tab's images.
    #[cfg(test)]
    pub(crate) fn images(&self) -> &TabImages {
        &self.images
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
        self.page.focus_from_back_to_top(window, cx);
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
        // Before the read editor takes a new read: Edit compares the two
        // (R-65).
        self.sync_edit(window, cx);
        self.sync_read(window, cx);
        self.sync_discussion(window, cx);
        self.sync_kept(cx);
        let item = self.model.item().rows().cloned();
        let (header, later) = self.header(item.as_ref(), window, cx);
        // An item tab's details are in its sheet. A file read by its path
        // has no sheet, so its page keeps D1's; a nonconforming file shows
        // its problems in "Needs fixing".
        let inspect = matches!(self.model.target(), OpenTarget::Inspect { .. });
        let identity = inspect.then(|| self.identity(item.as_ref(), cx));
        let banner = self.draft_banner(item.as_ref(), cx);
        // While a session lives, kept or not, its text is the edit editor's.
        let section = if self.has_session() {
            None
        } else {
            self.draft_section(window, cx)
        };
        let body = item.as_ref().map(|item| {
            let details = inspect.then(|| {
                v_flex()
                    .gap_3()
                    .child(self.provenance(item, cx))
                    .child(self.fields(item, cx))
                    .children((!banners::nonconforming(item)).then(|| self.problems(item, cx)))
                    .child(self.metadata(item, &[], cx))
            });
            // An item tab reads its body in the read view (decision 17); a
            // file read by its path keeps D1's source view (R-64).
            let text = if inspect {
                self.source(item, cx)
            } else if let Some(editing) = self.edit_view() {
                editing
            } else {
                self.read_body(item, cx)
            };
            v_flex().gap_3().children(details).child(text)
        });
        // D2's draft, on the page until Task 18 moves its banner.
        let content = v_flex()
            .gap_3()
            .children(identity)
            .children(banner)
            .children(section)
            .children(body);
        let viewing = !inspect && self.edit.phase() == Phase::Viewing;
        let page = self.page.render(content, &self.scroll, viewing, window, cx);
        let discussion = self.discussion_section(later, window, cx);
        let end = self
            .region_end
            .clone()
            .and_then(|end| end(&self.scroll, window, cx));
        let over = self.over_region(cx);
        // No zero minimum: the pane is never shorter than its header, so a
        // window too short for the header scrolls the content column.
        // The toolbar sits under the header, in the content region, so its
        // Home and End come before the region's.
        v_flex()
            .id("item-view")
            .test_support()
            .key_context(ITEM_TAB_CONTEXT)
            .on_action(cx.listener(|pane, _: &Edit, window, cx| {
                if !pane.edit_requested(window, cx) {
                    cx.propagate();
                }
            }))
            .on_action(cx.listener(|pane, _: &ToggleRich, _, cx| {
                if !pane.toggle_rich(cx) {
                    cx.propagate();
                }
            }))
            .on_action(cx.listener(|pane, _: &Save, window, cx| {
                if !pane.save_requested(window, cx) {
                    cx.propagate();
                }
            }))
            .flex_1()
            .w_full()
            .child(header)
            .children(self.toolbar_view())
            .child(page::region(&self.scroll, page, discussion, end, over, cx))
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "item_tests.rs"]
mod tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "item/frame_tests.rs"]
mod frame_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "item/header_tests.rs"]
mod header_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "item/sheet_tests.rs"]
mod sheet_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "item/page_tests.rs"]
mod page_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "item/discussion_tests.rs"]
mod discussion_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "item/images_tests.rs"]
mod images_tests;

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "item/edit_tests.rs"]
mod edit_tests;
