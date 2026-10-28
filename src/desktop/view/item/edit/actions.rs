//! The header's part in editing: Edit or Continue editing while viewing,
//! Save and Done while editing, and the banners an Edit, its saves and a
//! kept session raise.

use gpui_kit::{
    AnyElement, ClickEvent, Context, InteractiveElement, IntoElement, ParentElement, Role,
    StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{
        ActiveTheme, Disableable, Icon, Sizable, button::Button, h_flex, tooltip::Tooltip, v_flex,
    },
    div, px,
};
use manyhands::repository::ItemDto;

use super::{
    PREPARE_PENDING, PREPARE_REFUSED, RESUME, RETRY, Stopped,
    prepare::{ABANDON_THEN_EDIT, NOT_PREPARED_AGAIN, PRESS_EDIT},
    release::RELEASE_STRANDED,
};
use crate::desktop::{
    assets::icon_path,
    model::{
        edit::{CONTINUE_EDITING, DONE, DONE_IN_REVIEW, Phase, RECOVERY_OFF, edit_offered},
        item::{EDIT, LATER_NOTE},
    },
    view::item::{ItemPane, banners::Banner, text},
};

impl ItemPane {
    /// The header's actions for editing: Edit, or Continue editing while a
    /// draft is kept or stored (R-35), where it is offered, disabled while
    /// it cannot start, with "Available in a later release" while a draft
    /// restored in the tab waits for Task 18; Save and Done while editing.
    pub(in crate::desktop::view::item) fn edit_action(
        &self,
        item: Option<&ItemDto>,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        if self.editing() {
            return vec![self.save_action(cx), self.done_action(window, cx)];
        }
        if !item.is_some_and(|item| edit_offered(self.model.target(), item)) {
            return Vec::new();
        }
        let label = if self.draft_kept(cx) {
            CONTINUE_EDITING
        } else {
            EDIT
        };
        let button = Button::new("item-edit")
            .label(label)
            .small()
            .icon(Icon::default().path(icon_path("pencil")))
            .disabled(!self.edit_available(cx))
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                this.edit_requested(window, cx);
            }));
        // A draft restored in the tab waits for Task 18's route: until then
        // Continue editing is disabled and says so, as the header's other
        // disabled actions do (design, Header block). Its label is not a
        // later action's. While preparing, the pill and the banners say why.
        let restored = self.edit.phase() == Phase::Viewing
            && !self.has_session()
            && self
                .services
                .drafts()
                .read(cx)
                .attached(&self.tab_key())
                .is_some();
        if !restored {
            return vec![button.into_any_element()];
        }
        vec![
            div()
                .id("item-edit-action")
                .test_support()
                .role(Role::Group)
                .aria_label(label)
                .aria_description(LATER_NOTE)
                .flex_none()
                .child(
                    div()
                        .id("item-edit-note")
                        .test_support()
                        .aria_label(LATER_NOTE)
                        .child(button.tooltip(LATER_NOTE)),
                )
                .into_any_element(),
        ]
    }

    /// The header's Done while editing: the focus Escape in the editor
    /// gives (R-59), pressed by a click, Enter or Space (`done`). During
    /// the review it is disabled, with "Use these choices or Cancel first"
    /// as its tooltip and accessible description.
    fn done_action(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let focused = self.edit.done.is_focused(window);
        let ring = if focused { theme.ring } else { theme.input };
        let review = self.edit.phase() == Phase::Review;
        let words = if review {
            theme.muted_foreground
        } else {
            theme.foreground
        };
        let done = h_flex()
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
            .text_color(words)
            .text_size(px(12.))
            .whitespace_nowrap()
            .child(Icon::default().path(icon_path("eye")).w(px(13.)).h(px(13.)))
            .child(DONE);
        if !review {
            return done
                .on_click(
                    cx.listener(|this, _: &ClickEvent, window, cx| this.done_requested(window, cx)),
                )
                .into_any_element();
        }
        div()
            .id("item-done-note")
            .test_support()
            .aria_label(DONE_IN_REVIEW)
            .flex_none()
            .tooltip(|window, cx| Tooltip::new(DONE_IN_REVIEW).build(window, cx))
            .child(done.aria_description(DONE_IN_REVIEW))
            .into_any_element()
    }

    /// The banners an Edit raises: crash recovery off while editing, a
    /// prepare's hand-off still owed, with Resume, a refusal, with
    /// "Release stranded work context" where R-40 offers it, and a prepare
    /// R-78 does not send again; then its saves' (`save_view`).
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
                            this.retry_prepare(request, cx)
                        }))
                });
                // R-40: the context left by an abandoned prepare blocks Edit.
                let stranded = self.stranded_offered(*code).then(|| {
                    Button::new("item-release-stranded")
                        .label(RELEASE_STRANDED)
                        .small()
                        .on_click(
                            cx.listener(|this, _: &ClickEvent, _, cx| this.release_stranded(cx)),
                        )
                });
                banners.push((
                    Banner::Refused,
                    v_flex()
                        .gap_1()
                        .child(text("item-prepare-refused", PREPARE_REFUSED))
                        .child(text("item-prepare-message", message.clone()).text_color(muted))
                        .child(text("item-prepare-code", code.as_str()).text_color(muted))
                        .children(retry)
                        .children(stranded)
                        .into_any_element(),
                ));
            }
            // M3: R-78 did not send the prepare again. The way on is the
            // operation area's Abandon when it lists the prepare's own row,
            // and otherwise Edit.
            Some(Stopped::NotSentAgain(request)) => {
                let listed = self
                    .services
                    .operations()
                    .read(cx)
                    .pending_row_listed(*request);
                let next = if listed {
                    ABANDON_THEN_EDIT
                } else {
                    PRESS_EDIT
                };
                let words = format!("{NOT_PREPARED_AGAIN} {next}");
                banners.push((
                    Banner::DiscoveryPending,
                    text("item-prepare-not-sent", words).into_any_element(),
                ));
            }
            None => {}
        }
        banners.extend(self.save_banners(cx));
        banners.extend(self.kept_banners(cx));
        banners
    }
}
