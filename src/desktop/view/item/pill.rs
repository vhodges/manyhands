//! The state pill (design, State pill): where the tab stands, in an icon
//! and words that always go together, never colour alone. Its tooltip
//! gives the context kind, branch and worktree.
//!
//! While viewing, the pill reads the item: a context that needs recovery,
//! a closed ticket, a kept or stored draft, then the copy it was read
//! from. The editing states are named here for Tasks 14 and 15.

use gpui_kit::{
    App, Hsla, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled,
    TestSupportExt,
    component::{ActiveTheme, Icon, h_flex, spinner::Spinner, tooltip::Tooltip},
    div, px,
};
use manyhands::repository::{ClosureState, ItemContextKind, ItemDto};

use crate::desktop::{
    assets::icon_path,
    model::item::{context_problems, provenance_label},
};

/// The pill's text.
const TEXT_SIZE: f32 = 12.;
/// Its icon.
const ICON_SIZE: f32 = 13.;

/// Where the tab stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PillState {
    /// Viewing the primary copy.
    ViewingShared,
    /// Viewing the item's own context copy.
    ViewingPrivate,
    /// Viewing, with a kept or stored draft (decision 15).
    ViewingDraftKept,
    /// Preparing or Loading the private copy.
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 14.
    Preparing,
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 15.
    Saving,
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 15.
    Review,
    /// Editing, dirty.
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 14.
    EditingUnsaved,
    /// Editing, saved and clean; `not_shared` with a selected publication
    /// remote.
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 15.
    EditingSaved { not_shared: bool },
    /// A closed ticket.
    Closed,
    /// A closed ticket, with a kept or stored draft.
    ClosedDraftKept,
    /// The item's context needs recovery.
    NeedsAttention,
    /// Recovery needed, with a kept or stored draft.
    NeedsAttentionDraftKept,
}

/// The pill's words for `state`.
pub(crate) fn pill_words(state: PillState) -> &'static str {
    match state {
        PillState::ViewingShared => "Viewing the shared copy",
        PillState::ViewingPrivate => "Viewing your private copy",
        PillState::ViewingDraftKept => "Viewing · unsaved changes kept",
        PillState::Preparing => "Getting a private copy ready…",
        PillState::Saving => "Saving…",
        PillState::Review => "Reviewing changes",
        PillState::EditingUnsaved => "Editing · unsaved changes",
        PillState::EditingSaved { not_shared: false } => "Editing · saved on this computer",
        PillState::EditingSaved { not_shared: true } => {
            "Editing · saved on this computer · not shared yet"
        }
        PillState::Closed => "Closed",
        PillState::ClosedDraftKept => "Closed · unsaved changes kept",
        PillState::NeedsAttention => "Needs attention",
        PillState::NeedsAttentionDraftKept => "Needs attention · unsaved changes kept",
    }
}

/// The viewing state of `item`, read while no edit session lives:
/// recovery first, then closure, then a kept or stored draft, then the
/// copy it was read from. An unverified root is the shared copy.
pub(crate) fn viewing_state(item: &ItemDto, draft_kept: bool) -> PillState {
    if !context_problems(item).is_empty() {
        if draft_kept {
            PillState::NeedsAttentionDraftKept
        } else {
            PillState::NeedsAttention
        }
    } else if item
        .closure
        .as_ref()
        .is_some_and(|closure| closure.state == ClosureState::Closed)
    {
        if draft_kept {
            PillState::ClosedDraftKept
        } else {
            PillState::Closed
        }
    } else if draft_kept {
        PillState::ViewingDraftKept
    } else if item.context.kind == ItemContextKind::Active {
        PillState::ViewingPrivate
    } else {
        PillState::ViewingShared
    }
}

/// What the pill draws beside its words.
enum Mark {
    Icon(&'static str),
    Spinner,
}

/// The mark and its colour for `state` (design, State pill).
fn look(state: PillState, cx: &App) -> (Mark, Hsla) {
    let theme = cx.theme();
    match state {
        PillState::ViewingShared | PillState::ViewingPrivate => {
            (Mark::Icon("circle-check"), theme.success)
        }
        PillState::ViewingDraftKept
        | PillState::Review
        | PillState::EditingUnsaved
        | PillState::EditingSaved { .. } => (Mark::Icon("pencil"), theme.warning),
        PillState::Preparing | PillState::Saving => (Mark::Spinner, theme.info),
        PillState::Closed | PillState::ClosedDraftKept => {
            (Mark::Icon("archive"), theme.secondary_foreground)
        }
        PillState::NeedsAttention | PillState::NeedsAttentionDraftKept => {
            (Mark::Icon("circle-alert"), theme.danger)
        }
    }
}

/// The pill for `state`, with `item`'s context in its tooltip.
pub(super) fn pill(state: PillState, item: &ItemDto, cx: &App) -> impl IntoElement + use<> {
    let theme = cx.theme();
    let (mark, color) = look(state, cx);
    let words = pill_words(state);
    let tooltip = provenance_label(&item.context);
    let mark = match mark {
        Mark::Icon(name) => Icon::default()
            .path(icon_path(name))
            .w(px(ICON_SIZE))
            .h(px(ICON_SIZE))
            .text_color(color)
            .into_any_element(),
        Mark::Spinner => Spinner::new().color(color).into_any_element(),
    };
    div()
        .id("item-pill")
        .test_support()
        .aria_label(words)
        .flex_none()
        .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
        .child(
            h_flex()
                .gap(px(6.))
                .items_center()
                .px(px(10.))
                .py(px(4.))
                .rounded(px(5.))
                .bg(theme.secondary)
                .border_1()
                .border_color(theme.border)
                .whitespace_nowrap()
                .text_size(px(TEXT_SIZE))
                .text_color(color)
                .child(mark)
                .child(words),
        )
}
