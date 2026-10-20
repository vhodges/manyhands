//! The Find input, at the top of the navigation region, and what it
//! found (design, Discovery / Find).
//!
//! `secondary-p` focuses it, and Enter searches the selected repository.
//! Text that parses as a ULID opens its item in a tab through
//! `show_item`, a lifecycle-closed ticket as any other. Other text is a
//! short code: every ticket `find_tickets_by_slug` gives for it is listed
//! with its title, type, status, lifecycle, context and ID, and Open on
//! one opens it by its ID. No match says so. A short code is never a tab
//! key: a tab opens only from an item the library read by its ID.

use gpui_kit::{
    ClickEvent, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{
        ActiveTheme,
        button::{Button, ButtonVariants},
        h_flex,
        input::Input,
        v_flex,
    },
    div,
};
use manyhands::repository::ItemDto;

use super::{tickets::lifecycle_label, workspace::Workspace};
use crate::desktop::{
    model::{
        find::{FindQuery, FindResult, FindState},
        item::context_kind_label,
        list_state::LABEL_BUSY,
        tickets::TicketRow,
    },
    services::ReadKind,
};

pub(crate) const FIND_LABEL: &str = "Find by short code or ID";
pub(crate) const FIND_SEARCHING: &str = "Finding";
pub(crate) const FIND_OPENED: &str = "Opened in a tab";
pub(crate) const FIND_NONE: &str = "No match";
pub(crate) const FIND_NO_REPOSITORY: &str = "Select an available repository to find in";
pub(crate) const FIND_MATCHES: &str = "Tickets with this short code: choose one to open";
pub(crate) const FIND_OPEN: &str = "Open";

impl Workspace {
    /// The Find input, for tests.
    #[cfg(test)]
    pub(crate) fn find_input(&self) -> &gpui_kit::Entity<gpui_kit::component::input::InputState> {
        &self.find_input
    }

    /// `secondary-p`: focuses the Find input.
    pub(super) fn focus_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.find_input
            .update(cx, |input, cx| input.focus(window, cx));
        cx.notify();
    }

    /// Enter in the Find input: searches the selected repository for its
    /// text. Blank text searches nothing and clears what was found.
    #[allow(clippy::result_large_err)] // `ReadError` carries its scope by value.
    pub(super) fn submit_find(&mut self, cx: &mut Context<Self>) {
        let text = self.find_input.read(cx).value();
        let Some(query) = FindQuery::parse(&text) else {
            self.clear_find(cx);
            return;
        };
        let Some((key, resolved)) = self.resolved_selection() else {
            self.find.no_repository();
            self.find_read = None;
            cx.notify();
            return;
        };
        let stamp = self.find.start(key);
        self.find_read = Some(match query {
            FindQuery::Id(id) => self.services.read(
                ReadKind::Item,
                stamp,
                move |service| service.show_item(&resolved, &id),
                cx,
                |this: &mut Self, stamp, result, cx| {
                    if let Some(target) = this.find.apply_item(stamp, result) {
                        this.open_item(target, cx);
                    }
                    cx.notify();
                },
            ),
            FindQuery::Slug(slug) => self.services.read(
                ReadKind::FindSlug,
                stamp,
                move |service| service.find_tickets_by_slug(&resolved, &slug),
                cx,
                |this: &mut Self, stamp, result, cx| {
                    if this.find.apply_matches(stamp, result) {
                        cx.notify();
                    }
                },
            ),
        });
        cx.notify();
    }

    /// Nothing is searched: a search still running is dropped, and what
    /// the last one found goes.
    pub(super) fn clear_find(&mut self, cx: &mut Context<Self>) {
        self.find.clear();
        self.find_read = None;
        cx.notify();
    }

    /// Open on the match at `ix`: opens it by its ID.
    fn open_match(&mut self, ix: usize, cx: &mut Context<Self>) {
        let target = match self.find.state() {
            FindState::Done(FindResult::Matches(items)) => {
                items.get(ix).and_then(|item| self.find.target(item))
            }
            _ => None,
        };
        if let Some(target) = target {
            self.open_item(target, cx);
        }
    }

    /// The input, where it is in text, and the matches.
    pub(super) fn find_bar(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let muted = cx.theme().muted_foreground;
        let state = match self.find.state() {
            FindState::Idle => None,
            FindState::NoRepository => Some(FIND_NO_REPOSITORY),
            FindState::Searching => Some(FIND_SEARCHING),
            FindState::Done(FindResult::Opened) => Some(FIND_OPENED),
            FindState::Done(FindResult::None) => Some(FIND_NONE),
            FindState::Done(FindResult::Matches(_)) => Some(FIND_MATCHES),
            FindState::Busy => Some(LABEL_BUSY),
            FindState::Failed(failure) => Some(failure.code.message()),
        };
        let mut matches = Vec::new();
        if let FindState::Done(FindResult::Matches(items)) = self.find.state() {
            for (ix, item) in items.iter().enumerate() {
                matches.push(self.match_row(ix, item, cx));
            }
        }
        v_flex()
            .id("find")
            .gap_1()
            .child(
                div()
                    .id("find-input")
                    .test_support()
                    .aria_label(FIND_LABEL)
                    .child(Input::new(&self.find_input).aria_label(FIND_LABEL)),
            )
            .children(state.map(|text| {
                div()
                    .id("find-state")
                    .test_support()
                    .aria_label(text)
                    .text_xs()
                    .text_color(muted)
                    .child(text)
            }))
            .children(matches)
    }

    /// One match: its title, then its short code, type, status, lifecycle,
    /// context and ID, each in text, and Open.
    fn match_row(
        &self,
        ix: usize,
        item: &ItemDto,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let muted = cx.theme().muted_foreground;
        let row = TicketRow::from_item(item);
        let cell = |name: &'static str, text: String| {
            div()
                .id((name, ix))
                .test_support()
                .aria_label(text.clone())
                .child(text)
        };
        let label = item
            .title
            .clone()
            .or_else(|| item.id.clone())
            .unwrap_or_else(|| item.path.clone());
        let workspace = cx.entity().downgrade();
        v_flex()
            .id(("find-match", ix))
            .test_support()
            .aria_label(label.clone())
            .gap_0p5()
            .p_1()
            .border_1()
            .border_color(cx.theme().border)
            .rounded_sm()
            .child(
                h_flex().gap_2().child(div().text_sm().child(label)).child(
                    div().id(("find-match-open", ix)).test_support().child(
                        Button::new(("find-match-open-button", ix))
                            .ghost()
                            .label(FIND_OPEN)
                            .on_click(move |_: &ClickEvent, _, cx| {
                                let _ = workspace.update(cx, |this, cx| this.open_match(ix, cx));
                            }),
                    ),
                ),
            )
            .child(
                h_flex()
                    .flex_wrap()
                    .gap_2()
                    .text_xs()
                    .text_color(muted)
                    .children(row.slug.map(|slug| cell("find-match-slug", slug)))
                    .children(row.ticket_type.map(|text| cell("find-match-type", text)))
                    .children(row.status.map(|text| cell("find-match-status", text)))
                    .children(row.lifecycle.map(|lifecycle| {
                        cell(
                            "find-match-lifecycle",
                            lifecycle_label(lifecycle).to_owned(),
                        )
                    })),
            )
            .child(
                h_flex()
                    .flex_wrap()
                    .gap_2()
                    .text_xs()
                    .text_color(muted)
                    .child(cell(
                        "find-match-context",
                        context_kind_label(item.context.kind).to_owned(),
                    ))
                    .children(
                        item.context
                            .branch
                            .clone()
                            .map(|branch| cell("find-match-branch", branch)),
                    )
                    .child(cell("find-match-worktree", item.context.worktree.clone()))
                    .children(item.id.clone().map(|id| cell("find-match-id", id))),
            )
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "find_tests.rs"]
mod tests;
