//! A repository row, the panel's own element (R-7): `folder-git-2`, the
//! name, and the state at the right, drawn to `03`'s anatomy.
//!
//! "Selected" has one meaning. The Kit's wrapper marks the row its cursor
//! is on, and announces it; the row sets no `aria_selected`, and keeps the
//! Kit's `selected` only to draw the focus ring on the cursor's row while
//! the list has focus. The chosen repository carries the selected
//! treatment and a mark labelled "Selected".

use gpui_kit::{
    App, Context, FocusHandle, FontWeight, Hsla, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, TestSupportExt, WeakEntity,
    Window, canvas,
    component::{
        IndexPath, Selectable,
        list::{ListDelegate, ListState as KitListState},
        tooltip::Tooltip,
    },
    div,
    prelude::FluentBuilder,
    px,
};

use super::{RepositoryRow, RowBounds};
use crate::desktop::{
    model::repository_state::Tone,
    view::{
        rows::{ROW_HEIGHT, icon},
        shell::tokens::{ChromeColors, chrome_colors},
        sidebar::SELECTED,
        workspace::Workspace,
    },
};

const TEXT_SIZE: f32 = 13.;
const STATE_SIZE: f32 = 11.5;
const FOLDER_ICON: f32 = 14.;
const STATE_ICON: f32 = 12.;
const LEFT_PADDING: f32 = 12.;
const RIGHT_PADDING: f32 = 10.;
/// The chosen row's left line and the cursor's focus ring.
const MARK_WIDTH: f32 = 2.;

/// The Kit list's delegate: the rows, the Kit cursor, which is not the
/// chosen repository until Enter or a click chooses it, and the list's
/// focus, so the cursor's row draws its ring while the list holds it.
pub(crate) struct RepositoryDelegate {
    rows: Vec<RepositoryRow>,
    cursor: Option<IndexPath>,
    focus: Option<FocusHandle>,
    /// The Workspace a row's pointer hook reaches, since a row is drawn in
    /// the list's context, not the Workspace's (fact 61; R-36).
    #[allow(dead_code)] // Used from Task 5
    workspace: WeakEntity<Workspace>,
    bounds: RowBounds,
}

impl RepositoryDelegate {
    pub(super) fn new(workspace: WeakEntity<Workspace>, bounds: RowBounds) -> Self {
        Self {
            rows: Vec::new(),
            cursor: None,
            focus: None,
            workspace,
            bounds,
        }
    }

    pub(super) fn set_focus(&mut self, focus: FocusHandle) {
        self.focus = Some(focus);
    }

    pub(super) fn set_rows(&mut self, rows: Vec<RepositoryRow>) {
        self.rows = rows;
    }

    pub(super) fn rows(&self) -> &[RepositoryRow] {
        &self.rows
    }

    pub(crate) fn row(&self, ix: IndexPath) -> Option<&RepositoryRow> {
        self.rows.get(ix.row)
    }
}

impl ListDelegate for RepositoryDelegate {
    type Item = RowElement;

    fn items_count(&self, _section: usize, _cx: &App) -> usize {
        self.rows.len()
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        window: &mut Window,
        _cx: &mut Context<KitListState<Self>>,
    ) -> Option<RowElement> {
        let row = self.rows.get(ix.row)?.clone();
        let focused = self
            .focus
            .as_ref()
            .is_some_and(|focus| focus.is_focused(window));
        Some(RowElement {
            index: ix.row,
            row,
            cursor: false,
            focused,
            bounds: self.bounds.clone(),
        })
    }

    /// An empty list draws nothing of the Kit's: the panel's body says what
    /// there is to say (fact 3).
    fn render_empty(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<KitListState<Self>>,
    ) -> impl IntoElement {
        div()
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _window: &mut Window,
        cx: &mut Context<KitListState<Self>>,
    ) {
        self.cursor = ix;
        cx.notify();
    }
}

/// One drawn row.
#[derive(IntoElement)]
pub(crate) struct RowElement {
    index: usize,
    row: RepositoryRow,
    /// The Kit cursor is on this row.
    cursor: bool,
    /// The list holds focus.
    focused: bool,
    bounds: RowBounds,
}

impl Selectable for RowElement {
    /// The Kit's cursor, which the row draws as its focus ring and never
    /// announces: the Kit's wrapper does.
    fn selected(mut self, selected: bool) -> Self {
        self.cursor = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.cursor
    }
}

/// A state's tone, in the chrome's colours.
fn tone_color(tone: Tone, colors: &ChromeColors) -> Hsla {
    match tone {
        Tone::Failure => colors.red,
        Tone::Warning => colors.accent,
        Tone::Working => colors.teal,
        Tone::Muted => colors.muted,
        Tone::Success => colors.green,
        Tone::Secondary => colors.secondary,
    }
}

impl RenderOnce for RowElement {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = chrome_colors(cx);
        let Self {
            index,
            row,
            cursor,
            focused,
            bounds,
        } = self;
        let tooltip: SharedString = row.tooltip.join("\n").into();
        let state = row.state.label.clone().map(|label| {
            let color = row
                .state
                .tone
                .map_or(colors.secondary, |tone| tone_color(tone, &colors));
            div()
                .id(("repo-state", index))
                .test_support()
                .aria_label(label.clone())
                .flex_none()
                .flex()
                .items_center()
                .gap(px(4.))
                .text_size(px(STATE_SIZE))
                .text_color(color)
                .children(row.state.icon.map(|name| icon(name, STATE_ICON, color)))
                .child(label)
        });
        let key = row.key.clone();
        div()
            .id(("repo-row", index))
            .test_support()
            .aria_label(row.name.clone())
            .aria_description(row.tooltip.join(" "))
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
            .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
            .when(row.chosen, |this| {
                this.bg(colors.selection).font_weight(FontWeight::SEMIBOLD)
            })
            .when(!row.chosen, |this| {
                this.hover(move |style| style.bg(colors.hover))
            })
            .child(icon("folder-git-2", FOLDER_ICON, colors.secondary))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .child(row.name.clone()),
            )
            .children(state)
            .when(row.chosen, |this| {
                this.child(
                    div()
                        .id(("repo-selected", index))
                        .test_support()
                        .aria_label(SELECTED)
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .left_0()
                        .w(px(MARK_WIDTH))
                        .bg(colors.accent),
                )
            })
            .when(cursor && focused, |this| {
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
    }
}
