//! What the stale-edit review draws, in the page's place (design,
//! Stale-edit review): Use these choices, disabled with its reason until
//! every both-changed field has a choice, and Cancel, at the top; then
//! every field, the body first, with its base, theirs and mine side by
//! side, who changed it, Take theirs and Keep mine and what is chosen;
//! and the body's result in its own editor once the body has a side.
//! While its read is out, it says so, with Cancel; a read that failed
//! offers Retry and Cancel.
//!
//! All of it is inside the tab's draft section, where draft text belongs
//! (R-38). Layout follows the page's tokens; it is checked on screen.

use gpui_kit::{
    AnyElement, ClickEvent, Context, FontWeight, InteractiveElement, IntoElement, ParentElement,
    Styled, TestSupportExt,
    component::{
        ActiveTheme, Disableable, Selectable, Sizable,
        button::{Button, ButtonVariants},
        h_flex, v_flex,
    },
    div, px,
};
use manyhands::drafts::{Change, Field};

use super::review::ReviewState;
use crate::desktop::{
    model::review::{
        BEFORE, CHOOSE_A_BODY, CHOOSE_EACH, CHOOSE_ONE, Choice, KEEP_MINE, KEEPING_MINE, MINE,
        NOT_READ, READING, RESULT, REVIEW_INTRO, REVIEW_TITLE, Review, TAKE_THEIRS, TAKING_THEIRS,
        THEIRS, USE_THESE_CHOICES,
    },
    view::{
        confirm::CANCEL,
        draft_comparison::{change_label, field_name, field_value},
        item::{ItemPane, edit::RETRY, text},
    },
};

impl ItemPane {
    /// The review, in the tab's draft section.
    pub(super) fn review_view(&self, review: &ReviewState, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let mut content = v_flex()
            .id("item-review")
            .test_support()
            .track_focus(&review.focus)
            .w_full()
            .gap_3()
            .child(text("review-title", REVIEW_TITLE).font_weight(FontWeight::SEMIBOLD))
            .child(text("review-intro", REVIEW_INTRO).text_color(muted));
        match &review.model {
            Some(model) => {
                // The actions first, and the body, with its result, before
                // the other fields: a long review is read from the top of
                // the scroll region, which keeps no footer in place.
                let mut rows: Vec<_> = model.rows().iter().copied().enumerate().collect();
                rows.sort_by_key(|(_, (field, _))| *field != Field::Body);
                content =
                    content
                        .child(self.review_actions(model, cx))
                        .children(rows.into_iter().map(|(ix, (field, change))| {
                            self.field_row(ix, field, change, model, review, cx)
                        }));
            }
            None if review.failed => {
                content = content.child(
                    h_flex()
                        .flex_wrap()
                        .gap_3()
                        .items_center()
                        .child(text("review-status", NOT_READ))
                        .child(Button::new("review-retry").label(RETRY).small().on_click(
                            cx.listener(|this, _: &ClickEvent, _, cx| this.review_retry(cx)),
                        ))
                        .child(cancel(cx)),
                );
            }
            // While it reads, Cancel goes back to editing (I1).
            None => {
                content = content.child(
                    h_flex()
                        .flex_wrap()
                        .gap_3()
                        .items_center()
                        .child(text("review-status", READING).text_color(muted))
                        .child(cancel(cx)),
                );
            }
        }
        div()
            .id("draft-section")
            .test_support()
            .w_full()
            .child(content)
            .into_any_element()
    }

    /// One field: its name, who changed it, what is chosen and the two
    /// choices, then its three values side by side; the body's result
    /// below the body's.
    fn field_row(
        &self,
        ix: usize,
        field: Field,
        change: Change,
        model: &Review,
        review: &ReviewState,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let (muted, border, warning) = (theme.muted_foreground, theme.border, theme.warning);
        let choice = model.choice(ix);
        let state = match choice {
            None => text(("review-state", ix), CHOOSE_ONE).text_color(warning),
            Some(Choice::Theirs) => text(("review-state", ix), TAKING_THEIRS).text_color(muted),
            Some(Choice::Mine) => text(("review-state", ix), KEEPING_MINE).text_color(muted),
        };
        let chooser =
            |id: &'static str, label: &'static str, side: Choice, cx: &mut Context<Self>| {
                Button::new((id, ix))
                    .label(label)
                    .small()
                    .selected(choice == Some(side))
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.review_choose(ix, side, window, cx)
                    }))
            };
        let column = |id: &'static str, name: &'static str, value: String| {
            v_flex()
                .flex_1()
                .min_w_0()
                .gap_1()
                .child(div().text_xs().text_color(muted).child(name))
                .child(text((id, ix), value).text_sm())
        };
        let body = field == Field::Body;
        let mut values = h_flex().w_full().gap_3().items_start();
        if body {
            values = values.font_family(cx.theme().mono_font_family.clone());
        }
        let values = values
            .child(column(
                "review-before",
                BEFORE,
                field_value(model.base(), field),
            ))
            .child(column(
                "review-theirs",
                THEIRS,
                field_value(&model.theirs().fields, field),
            ))
            .child(column(
                "review-mine",
                MINE,
                field_value(model.mine(), field),
            ));
        let result = body.then(|| result_view(review, cx));
        v_flex()
            .w_full()
            .gap_2()
            .p(px(10.))
            .rounded(px(6.))
            .border_1()
            .border_color(border)
            .child(
                h_flex()
                    .flex_wrap()
                    .gap_2()
                    .items_center()
                    .child(
                        text(("review-field", ix), field_name(field))
                            .font_weight(FontWeight::SEMIBOLD),
                    )
                    .child(
                        text(("review-change", ix), change_label(change))
                            .text_xs()
                            .text_color(muted),
                    )
                    .child(state.text_xs())
                    .child(div().flex_1())
                    .child(chooser(
                        "review-take-theirs",
                        TAKE_THEIRS,
                        Choice::Theirs,
                        cx,
                    ))
                    .child(chooser("review-keep-mine", KEEP_MINE, Choice::Mine, cx)),
            )
            .child(values)
            .children(result)
            .into_any_element()
    }

    /// Use these choices, disabled with its reason until every
    /// both-changed field has a choice, and Cancel.
    fn review_actions(&self, model: &Review, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let ready = model.ready();
        h_flex()
            .flex_wrap()
            .gap_3()
            .items_center()
            .child(
                Button::new("review-use")
                    .label(USE_THESE_CHOICES)
                    .primary()
                    .small()
                    .disabled(!ready)
                    .on_click(
                        cx.listener(|this, _: &ClickEvent, window, cx| this.review_use(window, cx)),
                    ),
            )
            .child(cancel(cx))
            .children((!ready).then(|| {
                text("review-use-note", CHOOSE_EACH)
                    .text_xs()
                    .text_color(muted)
            }))
            .into_any_element()
    }
}

/// Cancel: back to the edit editor, with mine.
fn cancel(cx: &mut Context<ItemPane>) -> Button {
    Button::new("review-cancel")
        .label(CANCEL)
        .small()
        .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.review_cancel(window, cx)))
}

/// The body's result: its editor, once the body has a side, or why there
/// is none yet.
fn result_view(review: &ReviewState, cx: &Context<ItemPane>) -> AnyElement {
    let muted = cx.theme().muted_foreground;
    let border = cx.theme().border;
    let editor = match &review.host {
        Some(host) => div()
            .id("review-result-editor")
            .test_support()
            .w_full()
            .p(px(8.))
            .rounded(px(4.))
            .border_1()
            .border_color(border)
            .child(host.clone())
            .into_any_element(),
        None => text("review-result-note", CHOOSE_A_BODY)
            .text_sm()
            .text_color(muted)
            .into_any_element(),
    };
    v_flex()
        .w_full()
        .gap_1()
        .child(text("review-result", RESULT).text_xs().text_color(muted))
        .child(editor)
        .into_any_element()
}
