//! One Activity row (C1): one line, 28 px, in this order: the time, the
//! kind's icon, the message, the repository tag and the button. The message
//! ends in an ellipsis and shrinks first, so the button never leaves the row
//! (decision 22). The tooltip is `tooltip_lines`, which is also the row's
//! accessible description; its accessible name is the message.

use gpui_kit::{
    AnyElement, App, ClickEvent, ElementId, FocusHandle, Hsla, InteractiveElement, IntoElement,
    MouseButton, MouseDownEvent, ParentElement, SharedString, StatefulInteractiveElement, Styled,
    TestSupportExt, WeakEntity, Window, canvas,
    component::{ActiveTheme, Disableable, Sizable, button::Button, tooltip::Tooltip},
    div,
    prelude::FluentBuilder,
    px,
};

use std::rc::Rc;

use super::RowBounds;
use crate::desktop::{
    keys::ActivityMenuChoice,
    model::activity::{
        activity_time,
        copy::{cant_reach_title, message_of, tooltip_lines},
        rows::{ActivityRow, Kind, Offer, RowKey},
    },
    view::{
        confirm::local_offset,
        rows::{ROW_HEIGHT, icon},
        shell::tokens::{ChromeColors, chrome_colors},
        workspace::Workspace,
    },
};

const TEXT_SIZE: f32 = 13.;
const TAG_SIZE: f32 = 11.5;
const ICON_SIZE: f32 = 13.;
const TIME_WIDTH: f32 = 40.;
const TAG_WIDTH: f32 = 140.;
const LEFT_PADDING: f32 = 12.;
const RIGHT_PADDING: f32 = 10.;
/// The highlighted row's focus ring.
const MARK_WIDTH: f32 = 2.;

/// What a row is drawn with that the row itself does not hold.
pub(super) struct Drawn {
    /// The row's index among the applied rows: its elements' ids.
    pub(super) ix: usize,
    pub(super) highlighted: bool,
    /// The list holds focus: the highlighted row shows its ring.
    pub(super) list_focused: bool,
    pub(super) button_focus: FocusHandle,
    pub(super) bounds: RowBounds,
    pub(super) workspace: WeakEntity<Workspace>,
}

/// The row's tooltip, line by line, with B6's time line (B5).
pub(crate) fn tooltip(row: &ActivityRow) -> Vec<String> {
    let (_, full) = activity_time(row.at, local_offset(row.at.unix_timestamp()));
    tooltip_lines(&row.row, &full)
}

/// A kind's icon and colour (A1's `kind`).
pub(crate) fn kind_icon(row: &ActivityRow, colors: &ChromeColors) -> (&'static str, Hsla) {
    let derived = &row.row;
    match derived.kind {
        Kind::Info => ("info", colors.secondary),
        Kind::Success => ("circle-check", colors.green),
        Kind::Sharing if derived.group.check || matches!(derived.key, RowKey::Brought { .. }) => {
            ("refresh-cw", colors.teal)
        }
        Kind::Sharing => ("cloud-upload", colors.teal),
        Kind::ToShare => ("cloud-upload", colors.accent),
        Kind::Failure
            if matches!(derived.key, RowKey::Repository(_))
                && derived
                    .repo
                    .as_deref()
                    .is_some_and(|repo| derived.title == cant_reach_title(repo)) =>
        {
            ("wifi-off", colors.red)
        }
        Kind::Failure => ("circle-alert", colors.red),
        Kind::Locked => ("lock", colors.accent),
        Kind::Working => ("loader-circle", colors.teal),
    }
}

/// The choice `offer` makes on `row`, as drawn: the row's key and revision,
/// not the log's current ones (R-55).
pub(crate) fn choice_of(row: &ActivityRow, offer: &Offer) -> ActivityMenuChoice {
    ActivityMenuChoice {
        key: row.row.key.clone(),
        action: offer.action.clone(),
        shown: offer.label.clone(),
        revision: row.revision,
    }
}

/// One drawn row.
pub(super) fn element(drawn: Drawn, row: &ActivityRow, cx: &mut App) -> AnyElement {
    let colors = chrome_colors(cx);
    let Drawn {
        ix,
        highlighted,
        list_focused,
        button_focus,
        bounds,
        workspace,
    } = drawn;
    let (clock, _) = activity_time(row.at, local_offset(row.at.unix_timestamp()));
    let lines = tooltip(row);
    let tooltip_text: SharedString = lines.join("\n").into();
    let message = message_of(&row.row);
    let (name, color) = kind_icon(row, &colors);
    let key = row.row.key.clone();
    let opened = key.clone();
    let hooked = workspace.clone();
    let pressed = workspace.clone();
    let button = row
        .row
        .button
        .as_ref()
        .map(|offer| action_button(ix, row, offer, highlighted, button_focus, pressed, cx));
    let tag = row.row.repo.clone().map(|repo| {
        div()
            .id(("activity-tag", ix))
            .test_support()
            .aria_label(repo.clone())
            .max_w(px(TAG_WIDTH))
            .overflow_hidden()
            .text_ellipsis()
            .whitespace_nowrap()
            .text_size(px(TAG_SIZE))
            .text_color(colors.secondary)
            .child(repo)
    });
    div()
        .id(("activity-row", ix))
        .test_support()
        .aria_label(message.clone())
        .aria_description(lines.join(" "))
        // A secondary press highlights the row and opens its menu at the
        // pointer, taken before anything chooses or focuses: the Workspace's
        // opener refuses behind a modal (R-36).
        .capture_any_mouse_down({
            let workspace = workspace.clone();
            let key = key.clone();
            move |event: &MouseDownEvent, window, cx| {
                if event.button == MouseButton::Right {
                    window.prevent_default();
                    let at = event.position;
                    let key = key.clone();
                    let _ = workspace.update(cx, |workspace, cx| {
                        workspace.open_activity_menu_at(key, at, window, cx)
                    });
                }
            }
        })
        // A left press highlights the row and focuses the list; the button
        // inside, whose click comes later, runs on its own.
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            let key = opened.clone();
            let _ = hooked.update(cx, |workspace, cx| {
                workspace.highlight_activity(key, window, cx)
            });
        })
        .relative()
        .flex()
        .flex_none()
        .w_full()
        .h(ROW_HEIGHT)
        .pl(px(LEFT_PADDING))
        .pr(px(RIGHT_PADDING))
        .gap(px(8.))
        .items_center()
        .text_size(px(TEXT_SIZE))
        .text_color(colors.text)
        .tooltip(move |window, cx| Tooltip::new(tooltip_text.clone()).build(window, cx))
        .when(highlighted, |this| this.bg(colors.selection))
        .when(!highlighted, |this| {
            this.hover(move |style| style.bg(colors.hover))
        })
        .child(
            div()
                .id(("activity-time", ix))
                .test_support()
                .aria_label(clock.clone())
                .w(px(TIME_WIDTH))
                .flex_none()
                .font_family(cx.theme().mono_font_family.clone())
                .text_size(px(TAG_SIZE))
                .text_color(colors.muted)
                .child(clock),
        )
        .child(icon(name, ICON_SIZE, color))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
                .text_ellipsis()
                .whitespace_nowrap()
                .child(message),
        )
        .children(tag)
        .children(button)
        .when(highlighted && list_focused, |this| {
            this.child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .border(px(MARK_WIDTH))
                    .border_color(colors.teal),
            )
        })
        // Where the row is, for a key-opened menu (Part D).
        .child(
            canvas(
                move |at, _, _| {
                    bounds.borrow_mut().insert(key, at);
                },
                |_, (), _, _| {},
            )
            .absolute()
            .size_full(),
        )
        .into_any_element()
}

/// The row's button, in Activity's list: its press is the list's choice.
fn action_button(
    ix: usize,
    row: &ActivityRow,
    offer: &Offer,
    highlighted: bool,
    focus: FocusHandle,
    workspace: WeakEntity<Workspace>,
    cx: &mut App,
) -> AnyElement {
    let choice = choice_of(row, offer);
    let press = Rc::new(move |window: &mut Window, cx: &mut App| {
        let choice = choice.clone();
        let _ = workspace.update(cx, |workspace, cx| {
            workspace.press_activity(choice, window, cx)
        });
    });
    button(
        ("activity-button", ix).into(),
        offer,
        highlighted,
        focus,
        press,
        cx,
    )
}

/// What a row's button does when pressed, by click or key.
pub(crate) type Press = Rc<dyn Fn(&mut Window, &mut App)>;

/// A row's button, here and in the Needs attention popover. It is a wrapper
/// a test finds by `id`, around a Kit button that draws it and takes a
/// pointer's click. The highlighted row's enabled button tracks the one
/// button handle, which is a tab stop, so Tab reaches it, Enter and Space
/// press it there, and a dialog it opens gives focus back to where the
/// list's press said (C3); every other button is no tab stop (R-33). A
/// disabled button takes no focus and does nothing.
pub(crate) fn button(
    id: ElementId,
    offer: &Offer,
    highlighted: bool,
    focus: FocusHandle,
    press: Press,
    cx: &mut App,
) -> AnyElement {
    let press_click = press.clone();
    let mut wrapper = div()
        .id(id.clone())
        .test_support()
        .aria_label(offer.label.clone())
        .flex_none();
    if highlighted && offer.enabled {
        let ring = cx.theme().ring;
        let keyboard = press.clone();
        wrapper = wrapper
            .track_focus(&focus)
            .rounded_sm()
            .focus(move |style| style.border_2().border_color(ring))
            .on_click(move |event: &ClickEvent, window, cx| {
                if event.is_keyboard() {
                    keyboard(window, cx);
                }
            });
    }
    wrapper
        .child(
            Button::new(ElementId::NamedChild(
                std::sync::Arc::new(id),
                "button".into(),
            ))
            .label(offer.label.clone())
            .small()
            .tab_stop(false)
            .disabled(!offer.enabled)
            .on_click(move |_, window, cx| press_click(window, cx)),
        )
        .into_any_element()
}
