//! The header's part in editing: Edit or Continue editing while viewing,
//! Done while editing, and the banners an Edit raises.

use gpui_kit::{
    AnyElement, App, ClickEvent, Context, InteractiveElement, IntoElement, ParentElement, Role,
    StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{ActiveTheme, Disableable, Icon, Sizable, button::Button, h_flex, v_flex},
    px,
};
use manyhands::repository::ItemDto;

use super::{PREPARE_PENDING, PREPARE_REFUSED, RESUME, RETRY, Stopped};
use crate::desktop::{
    assets::icon_path,
    model::{
        edit::{CONTINUE_EDITING, DONE, Phase, RECOVERY_OFF, edit_offered},
        item::EDIT,
    },
    view::item::{ItemPane, banners::Banner, text},
};

impl ItemPane {
    /// The header's action for editing: Edit, or Continue editing while a
    /// draft is kept or stored (R-35), where it is offered, disabled while
    /// it cannot start; Done while editing.
    pub(in crate::desktop::view::item) fn edit_action(
        &self,
        item: Option<&ItemDto>,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if self.editing() {
            return Some(self.done_action(window, cx));
        }
        if !edit_offered(self.model.target(), item?) {
            return None;
        }
        let label = if self.draft_kept(cx) {
            CONTINUE_EDITING
        } else {
            EDIT
        };
        Some(
            Button::new("item-edit")
                .label(label)
                .small()
                .icon(Icon::default().path(icon_path("pencil")))
                .disabled(!self.edit_available(cx))
                .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                    this.edit_requested(window, cx);
                }))
                .into_any_element(),
        )
    }

    /// The header's Done while editing: the focus Escape in the editor
    /// gives (R-59). Its rules are Task 16's.
    fn done_action(&self, window: &Window, cx: &App) -> AnyElement {
        let theme = cx.theme();
        let focused = self.edit.done.is_focused(window);
        let ring = if focused { theme.ring } else { theme.input };
        h_flex()
            .id("item-done")
            .test_support()
            .role(Role::Button)
            .aria_label(DONE)
            .track_focus(&self.edit.done)
            .flex_none()
            .gap(px(6.))
            .items_center()
            .px(px(10.))
            .py(px(5.))
            .rounded(px(5.))
            .border_1()
            .border_color(ring)
            .bg(theme.secondary)
            .text_color(theme.foreground)
            .text_size(px(12.))
            .whitespace_nowrap()
            .child(Icon::default().path(icon_path("eye")).w(px(13.)).h(px(13.)))
            .child(DONE)
            .into_any_element()
    }

    /// The banners an Edit raises: crash recovery off while editing, a
    /// prepare's hand-off still owed, with Resume, and a refusal.
    pub(in crate::desktop::view::item) fn edit_banners(
        &self,
        cx: &mut Context<Self>,
    ) -> Vec<(Banner, AnyElement)> {
        let mut banners = Vec::new();
        if self.editing()
            && let Some(reason) = self.services.drafts().read(cx).unavailable()
        {
            let words = format!("{RECOVERY_OFF}: {reason} Save to protect your changes.");
            banners.push((
                Banner::RecoveryOff,
                text("item-recovery-off", words).into_any_element(),
            ));
        }
        match &self.edit.stopped {
            Some(Stopped::Pending(_)) => banners.push((
                Banner::DiscoveryPending,
                h_flex()
                    .flex_wrap()
                    .gap_3()
                    .items_center()
                    .child(text("item-prepare-pending", PREPARE_PENDING))
                    .child(
                        Button::new("item-prepare-resume")
                            .label(RESUME)
                            .small()
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.resume(cx))),
                    )
                    .into_any_element(),
            )),
            Some(Stopped::Refused {
                code,
                message,
                retry,
            }) => {
                let muted = cx.theme().muted_foreground;
                let retry = retry.map(|request| {
                    Button::new("item-prepare-retry")
                        .label(RETRY)
                        .small()
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.edit.stopped = None;
                            this.edit.phase = Phase::Preparing(request);
                            this.services.worker().retry(request, None);
                            cx.notify();
                        }))
                });
                banners.push((
                    Banner::Refused,
                    v_flex()
                        .gap_1()
                        .child(text("item-prepare-refused", PREPARE_REFUSED))
                        .child(text("item-prepare-message", message.clone()).text_color(muted))
                        .child(text("item-prepare-code", code.as_str()).text_color(muted))
                        .children(retry)
                        .into_any_element(),
                ));
            }
            None => {}
        }
        banners
    }
}
