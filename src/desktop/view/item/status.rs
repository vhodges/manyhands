//! Loading, typed failures and retained-state refresh guidance.

use gpui_kit::{
    ClickEvent, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, TestSupportExt,
    component::{ActiveTheme, button::Button, v_flex},
    div,
};
use manyhands::results::ResultCode;

use super::{ItemPane, NOT_REFRESHED, REFRESH, REFRESHING, text};
use crate::desktop::{
    model::{
        list_state::{ListState, Rows},
        navigation::NotReloaded,
    },
    view::sidebar::RETRY,
};

impl ItemPane {
    pub(in crate::desktop::view) fn status<T: Rows>(
        &self,
        name: &'static str,
        state: &ListState<T>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<T> {
        let muted = cx.theme().muted_foreground;
        let label = match state {
            ListState::Ready { index, .. }
                if index.as_ref().is_none_or(|index| {
                    index.state == manyhands::repository::IndexState::Current
                }) =>
            {
                None
            }
            _ => Some(
                text(format!("{name}-state"), state.label())
                    .text_xs()
                    .text_color(muted),
            ),
        };
        let failure = match state {
            ListState::Failed(failure) => Some(failure.code),
            _ => None,
        };
        let failed = failure.map(|code| {
            v_flex()
                .gap_1()
                .child(text(format!("{name}-failure"), code.message()).text_sm())
                .child(
                    text(format!("{name}-failure-code"), code.as_str())
                        .text_xs()
                        .text_color(muted),
                )
        });
        let action = match (state, failure) {
            (_, Some(ResultCode::ItemNotFound)) => Some(match self.refreshing {
                Some(_) => div()
                    .id(format!("{name}-refreshing"))
                    .test_support()
                    .aria_label(REFRESHING)
                    .text_sm()
                    .child(REFRESHING),
                None => div().id(format!("{name}-refresh")).test_support().child(
                    Button::new(format!("{name}-refresh-button"))
                        .label(REFRESH)
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.refresh(cx))),
                ),
            }),
            (ListState::Busy | ListState::Failed(_), _) => Some(
                div().id(format!("{name}-retry")).test_support().child(
                    Button::new(format!("{name}-retry-button"))
                        .label(RETRY)
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                            this.reload(cx);
                        })),
                ),
            ),
            _ => None,
        };
        v_flex()
            .gap_1()
            .children(label)
            .children(failed)
            .children(action)
    }

    pub(super) fn not_refreshed(&self, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
        let code = match self.model.not_refreshed()? {
            NotReloaded::Busy => ResultCode::Busy,
            NotReloaded::Failed(failure) => failure.code,
        };
        let muted = cx.theme().muted_foreground;
        Some(
            v_flex()
                .id("item-not-refreshed")
                .test_support()
                .aria_label(NOT_REFRESHED)
                .gap_1()
                .p_2()
                .border_1()
                .border_color(cx.theme().border)
                .child(div().text_sm().child(NOT_REFRESHED))
                .child(text("item-not-refreshed-failure", code.message()).text_sm())
                .child(
                    text("item-not-refreshed-code", code.as_str())
                        .text_xs()
                        .text_color(muted),
                )
                .child(
                    div().id("item-not-refreshed-retry").test_support().child(
                        Button::new("item-not-refreshed-retry-button")
                            .label(RETRY)
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                this.reload(cx);
                            })),
                    ),
                ),
        )
    }
}
