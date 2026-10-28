//! What the tab shows of its saves (design, Save states; R-14): the
//! header's Save, the pill while editing, and the banners a save raises,
//! each in the words of `model::edit::save_state`.
//!
//! - **Save** (`save` icon, "Save a checkpoint on this computer (Ctrl S)")
//!   is enabled only while dirty, not saving and not in the review. On an
//!   unverified base it reads "Review before saving".
//! - **The pill** while editing: unsaved while dirty; saved on this
//!   computer when clean, not shared yet once a save has found a selected
//!   publication remote (fact 41).
//! - **The banners**: check the current version, with Review…, on an
//!   unverified base until the review opens; a hand-off owed, with Resume;
//!   a pending checkpoint, with Retry; the change banner while the review
//!   an `external_change` opened stays open; a refusal, with the library's
//!   message and code, and Retry when its recovery offers the request
//!   again.

use gpui_kit::{
    AnyElement, App, ClickEvent, Context, IntoElement, ParentElement, Styled,
    component::{ActiveTheme, Disableable, Icon, Sizable, button::Button, h_flex, v_flex},
};
use manyhands::results::RecoveryActionKind;

use super::Phase;
use crate::desktop::{
    assets::icon_path,
    model::edit::save_state::{
        REVIEW_BEFORE_SAVING, SAVE, SAVE_TOOLTIP, SaveState, before_save, save_banner, save_offer,
        save_state,
    },
    view::item::{
        ItemPane,
        banners::Banner,
        pill::{PillState, edit_state, save_pill},
        text,
    },
};

impl ItemPane {
    /// Whether Save can save now: editing, dirty, and no save in flight or
    /// waiting for its Retry.
    pub(in crate::desktop::view::item) fn save_offered(&self, cx: &App) -> bool {
        self.edit.phase() == Phase::Editing && self.edit.save.run.is_none() && self.edit_dirty(cx)
    }

    /// The header's Save while editing, before Done.
    pub(in crate::desktop::view::item) fn save_action(&self, cx: &mut Context<Self>) -> AnyElement {
        let label = match before_save(self.base_verified()) {
            SaveState::ReviewBeforeSaving => REVIEW_BEFORE_SAVING,
            _ => SAVE,
        };
        Button::new("item-save")
            .label(label)
            .small()
            .icon(Icon::default().path(icon_path("save")))
            .tooltip(SAVE_TOOLTIP)
            .disabled(!self.save_offered(cx))
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                this.save_requested(window, cx);
            }))
            .into_any_element()
    }

    /// The pill while a session is made ready or lives; `None` while
    /// viewing, when the item decides.
    pub(in crate::desktop::view::item) fn edit_pill(&self, cx: &App) -> Option<PillState> {
        let (phase, dirty) = (self.edit.phase(), self.edit_dirty(cx));
        if phase != Phase::Editing {
            return edit_state(phase, dirty);
        }
        let saved = self
            .edit
            .session
            .as_ref()
            .is_some_and(|session| session.saved());
        let state = if dirty {
            SaveState::Unsaved
        } else {
            SaveState::SavedLocally {
                publication_pending: saved && self.edit.save.remote,
            }
        };
        save_pill(state)
    }

    /// The banners the tab's saves raise.
    pub(in crate::desktop::view::item) fn save_banners(
        &self,
        cx: &mut Context<Self>,
    ) -> Vec<(Banner, AnyElement)> {
        let mut banners = Vec::new();
        if !self.editing() {
            return banners;
        }
        if self.edit.phase() != Phase::Review && !self.base_verified() {
            let state = before_save(false);
            let action = action("item-save-review", state, cx, |this, cx| {
                this.open_review(cx)
            });
            banners.push((
                Banner::ReviewBeforeSaving,
                row("item-save-unverified", state, action),
            ));
        }
        let Some(answer) = &self.edit.save.answer else {
            return banners;
        };
        let state = save_state(answer.code, &answer.effects, self.edit.save.remote);
        match state {
            SaveState::DiscoveryPending => {
                let resume = action("item-save-resume", state, cx, |this, cx| {
                    this.resume_save(cx)
                });
                banners.push((
                    Banner::DiscoveryPending,
                    row("item-save-pending", state, resume),
                ));
            }
            SaveState::CheckpointFailed => {
                let retry = action("item-save-retry", state, cx, |this, cx| this.retry_save(cx));
                banners.push((
                    Banner::CheckpointFailed,
                    row("item-save-failed", state, retry),
                ));
            }
            // The change banner stays while the review it opened is open.
            SaveState::Changed if self.edit.phase() == Phase::Review => {
                banners.push((Banner::Changed, row("item-save-changed", state, None)));
            }
            SaveState::Refused(code) => {
                let muted = cx.theme().muted_foreground;
                let again = answer
                    .recovery
                    .iter()
                    .any(|action| action.action == RecoveryActionKind::RequestRetry)
                    .then(|| {
                        Button::new("item-save-again")
                            .label(super::RETRY)
                            .small()
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                this.save_requested(window, cx);
                            }))
                    });
                banners.push((
                    Banner::Refused,
                    v_flex()
                        .gap_1()
                        .children(save_banner(state).map(|words| text("item-save-refused", words)))
                        .child(text("item-save-message", answer.message.clone()).text_color(muted))
                        .child(text("item-save-code", code.as_str()).text_color(muted))
                        .children(again)
                        .into_any_element(),
                ));
            }
            SaveState::Changed
            | SaveState::Unsaved
            | SaveState::SavedLocally { .. }
            | SaveState::ReviewBeforeSaving => {}
        }
        banners
    }
}

/// A banner's words for `state`, as the element `id`, and its action.
fn row(id: &'static str, state: SaveState, action: Option<Button>) -> AnyElement {
    h_flex()
        .flex_wrap()
        .gap_3()
        .items_center()
        .children(save_banner(state).map(|words| text(id, words)))
        .children(action)
        .into_any_element()
}

/// The action `state` offers, as the button `id`, doing `act`.
fn action(
    id: &'static str,
    state: SaveState,
    cx: &mut Context<ItemPane>,
    act: impl Fn(&mut ItemPane, &mut Context<ItemPane>) + 'static,
) -> Option<Button> {
    let label = save_offer(state)?;
    Some(
        Button::new(id)
            .label(label)
            .small()
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| act(this, cx))),
    )
}
