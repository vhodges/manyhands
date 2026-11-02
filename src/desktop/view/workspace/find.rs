//! Find actions and stamped results, owned by the workspace.

use super::Workspace;
use crate::desktop::{
    model::find::{FindQuery, FindResult, FindState},
    services::ReadKind,
};
use gpui_kit::{Context, Entity, FocusHandle, Window, component::input::InputState};

impl Workspace {
    pub(crate) fn find_input(&self) -> &Entity<InputState> {
        &self.find_input
    }
    pub(in crate::desktop::view) fn find_state(&self) -> &FindState {
        self.find.state()
    }
    pub(in crate::desktop::view) fn find_match_focus(&self, index: usize) -> Option<&FocusHandle> {
        self.find_focus.get(index)
    }

    pub(super) fn focus_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.find_input
            .update(cx, |input, cx| input.focus(window, cx));
        cx.notify();
    }

    #[allow(clippy::result_large_err)]
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
                        let count = match this.find.state() {
                            FindState::Done(FindResult::Matches(items)) => items.len(),
                            _ => 0,
                        };
                        while this.find_focus.len() < count {
                            this.find_focus.push(cx.focus_handle());
                        }
                        cx.notify();
                    }
                },
            ),
        });
        cx.notify();
    }

    pub(super) fn clear_find(&mut self, cx: &mut Context<Self>) {
        self.find.clear();
        self.find_read = None;
        cx.notify();
    }

    pub(in crate::desktop::view) fn open_match(&mut self, ix: usize, cx: &mut Context<Self>) {
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
}
