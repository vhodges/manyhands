//! Drawing the Add repository sheet (Wave 04 Cycle 05a design A1, B5 to
//! B10): a Kit sheet 470 px wide, the frame's header, a body in a scroll of
//! its own laid over the Kit's, which takes the page keys (B12), and the
//! frame's footer with Cancel and the shown mode's submit (B10). The body
//! is the mode control, then the shown mode's form: Add an existing folder
//! (B6) or Create a repository.

use gpui_kit::{
    ClickEvent, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{
        ActiveTheme, Disableable, Icon,
        button::{Button, ButtonVariants},
        h_flex,
        input::Input,
        scroll::Scrollbar,
        sheet::Sheet,
        v_flex,
    },
    div, px,
};

use super::{
    ADD, ADD_DESCRIPTION, ADD_TITLE, ADDING, AddRepositorySheet, BROWSE, CANCEL, CANCEL_ID, CREATE,
    FOLDER, Form, IDENTITY_HEADING, IDENTITY_REQUIRED, PREFIX, REMOVE_NOTE, mode::Mode,
};
use crate::desktop::{
    assets::icon_path,
    keys::{ScrollPageDown, ScrollPageUp, ScrollToBottom, ScrollToTop},
    view::{
        frame::{self, Footer, Header},
        scroll::ContentScroll,
        settings::{EMAIL, NAME, field_column, footer_kit, wrapped},
        shell::tokens::chrome_colors,
    },
};

/// The sheet's width (`05`, Add repository; B1).
const WIDTH: f32 = 470.;

/// A piece of text a test reads by `id`.
pub(super) fn text(
    id: impl Into<gpui_kit::ElementId>,
    value: impl Into<String>,
) -> impl IntoElement {
    let value = value.into();
    div()
        .id(id)
        .test_support()
        .aria_label(value.clone())
        .text_sm()
        .child(value)
}

/// A note line a test reads by `id`, as the one under Browse… (B6 item 1).
pub(super) fn note(id: &'static str, value: &'static str) -> impl IntoElement + use<> {
    div()
        .id(id)
        .test_support()
        .aria_label(value)
        .text_size(px(12.))
        .child(value)
}

/// The note both forms end with (B6 item 7; B8 item 7): removing a
/// repository later never touches its files.
pub(super) fn remove_note(id: &'static str, cx: &Context<AddRepositorySheet>) -> impl IntoElement {
    let colors = chrome_colors(cx);
    h_flex()
        .id(id)
        .test_support()
        .aria_label(REMOVE_NOTE)
        .items_start()
        .gap(px(6.))
        .text_size(px(12.))
        .text_color(colors.secondary)
        .child(
            Icon::default()
                .path(icon_path("shield-check"))
                .text_color(colors.teal)
                .w(px(14.))
                .h(px(14.)),
        )
        .child(div().flex_1().min_w_0().child(REMOVE_NOTE))
}

impl AddRepositorySheet {
    /// A labeled field: its label, then its input in its wrapper.
    pub(super) fn field(
        &self,
        id: &'static str,
        label: &'static str,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        self.field_with(id, label, None, cx)
    }

    /// `field`, its input led by the icon `icon`.
    fn field_with(
        &self,
        id: &'static str,
        label: &'static str,
        icon: Option<&'static str>,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let input = Input::new(&self.inputs[id]).aria_label(label);
        let input = match icon {
            Some(icon) => input.prefix(Icon::default().path(icon_path(icon)).text_color(muted)),
            None => input,
        };
        field_column()
            .gap_0p5()
            .child(div().text_xs().text_color(muted).child(label))
            .child(wrapped(id, label, &self.focus[id], &self.scroll, input).w_full())
    }

    /// A body button in its wrapper.
    fn button(
        &self,
        id: &'static str,
        label: &'static str,
        disabled: bool,
        on_click: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let button = Button::new((id, 0usize))
            .label(label)
            .disabled(disabled)
            .on_click(
                cx.listener(move |this, _: &ClickEvent, window, cx| on_click(this, window, cx)),
            );
        wrapped(id, label, &self.focus[id], &self.scroll, button)
    }

    /// The folder field with Browse… beside it, and the note under them
    /// (B6 item 1).
    pub(super) fn folder(
        &self,
        form: Form,
        label: &'static str,
        icon: Option<&'static str>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let row = h_flex()
            .gap_2()
            .items_end()
            .child(self.field_with(form.path(), label, icon, cx))
            .child(self.button(
                form.browse(),
                BROWSE,
                self.browse_disabled(),
                move |this, window, cx| this.browse(form, window, cx),
                cx,
            ));
        v_flex().gap_1().child(row).children(self.browse_note(form))
    }

    /// A footer submit: a primary Kit button drawn with the sheet's own
    /// handle, which the confirmation gives focus back to (B12, G13). While
    /// an add of this open is pending it reads "Adding…" (R-27). Disabled,
    /// it tracks no focus, and its wrapper holds the handle until it is
    /// enabled again (Ruling 18).
    fn submit_button(
        &self,
        form: Form,
        label: &'static str,
        disabled: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let id = form.submit();
        let label = if self.adding { ADDING } else { label };
        let button = Button::new((id, 0usize))
            .label(label)
            .primary()
            .disabled(disabled)
            .on_click(
                cx.listener(move |this, _: &ClickEvent, window, cx| this.submit(form, window, cx)),
            );
        footer_kit(id, label, &self.focus[id], (id, 0usize), button)
    }

    /// The footer's Cancel (B10).
    fn cancel_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let button = Button::new((CANCEL_ID, 0usize))
            .label(CANCEL)
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.cancel(cx)));
        footer_kit(
            CANCEL_ID,
            CANCEL,
            &self.focus[CANCEL_ID],
            (CANCEL_ID, 0usize),
            button,
        )
    }

    /// Add an existing folder (B6): the folder, its checks, the main
    /// branch, the prefix unless the folder is set up (R-31), the identity
    /// when Git has none, and the note.
    fn add_form(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let path = self.text(Form::Add.path(), cx);
        let identity = self.inspection.identity_required(&path).then(|| {
            v_flex()
                .id("add-identity")
                .gap_2()
                .child(
                    div()
                        .text_sm()
                        .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                        .child(IDENTITY_HEADING),
                )
                .child(text("add-identity-required", IDENTITY_REQUIRED))
                .child(self.field(Form::Add.name(), NAME, cx))
                .child(self.field(Form::Add.email(), EMAIL, cx))
        });
        let prefix = (!self.inspection.set_up()).then(|| {
            self.field(Form::Add.prefix(), PREFIX, cx)
                .into_any_element()
        });
        v_flex()
            .gap_3()
            .child(self.folder(Form::Add, FOLDER, Some("folder"), cx))
            .children(self.checks(cx))
            .child(self.branch_field(cx))
            .children(prefix)
            .children(identity)
            .child(remove_note("add-note", cx))
    }

    /// The sheet as this frame draws it. The builder Root keeps calls this
    /// on every render, so all its state is the sheet's.
    pub(crate) fn sheet(
        &mut self,
        sheet: Sheet,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Sheet {
        self.sync_branches(window, cx);
        let handle = self.scroll.handle().clone();
        let submit = match self.mode {
            Mode::Existing => {
                let disabled = !self.add_enabled(cx);
                self.submit_button(Form::Add, ADD, disabled, cx)
                    .into_any_element()
            }
            Mode::New => {
                let disabled = !self.create_enabled(cx);
                self.submit_button(Form::Create, CREATE, disabled, cx)
                    .into_any_element()
            }
        };
        let footer = Footer {
            hint: None,
            buttons: vec![self.cancel_button(cx).into_any_element(), submit],
        };
        let form = match self.mode {
            Mode::Existing => self.add_form(cx).into_any_element(),
            Mode::New => self.create_form(cx).into_any_element(),
        };
        sheet
            .size(px(WIDTH))
            .title(frame::sheet_header(
                Header {
                    icon: None,
                    title: ADD_TITLE.into(),
                    description: Some(ADD_DESCRIPTION.into()),
                    ids: ("add-sheet-title", Some("add-sheet-description")),
                },
                cx,
            ))
            .footer(frame::sheet_footer(footer, cx))
            // The Kit's body grows with what it holds; laid over it, the
            // sheet scrolls on its own handle, which reveals the focused
            // control and takes the page keys (B12, R-8).
            .child(
                div()
                    .id("add-sheet")
                    .test_support()
                    .absolute()
                    .inset_0()
                    .on_action(cx.listener(|this, _: &ScrollPageUp, _, cx| {
                        this.scroll_body(|scroll| scroll.page(-1.), cx)
                    }))
                    .on_action(cx.listener(|this, _: &ScrollPageDown, _, cx| {
                        this.scroll_body(|scroll| scroll.page(1.), cx)
                    }))
                    .on_action(cx.listener(|this, _: &ScrollToTop, _, cx| {
                        this.scroll_body(ContentScroll::to_top, cx)
                    }))
                    .on_action(cx.listener(|this, _: &ScrollToBottom, _, cx| {
                        this.scroll_body(ContentScroll::to_bottom, cx)
                    }))
                    .child(
                        div()
                            .id("add-scroll")
                            .test_support()
                            .size_full()
                            .overflow_y_scroll()
                            .track_scroll(&handle)
                            .child(
                                v_flex()
                                    .gap_4()
                                    .pr_4()
                                    .pb_4()
                                    .child(self.mode_control(window, cx))
                                    .child(form),
                            ),
                    )
                    .child(
                        div()
                            .absolute()
                            .inset_0()
                            .child(Scrollbar::vertical(&handle).id("add-scrollbar")),
                    ),
            )
    }

    /// A page key moves the body, and nothing behind the sheet.
    fn scroll_body(&mut self, scroll: impl Fn(&ContentScroll) -> bool, cx: &mut Context<Self>) {
        if scroll(&self.scroll) {
            cx.notify();
        }
    }
}
