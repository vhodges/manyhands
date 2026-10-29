//! The item's header block (design, Header block; `item_view/01-item-frame.md`):
//! the breadcrumb, the title, the chips, the actions with the state pill,
//! and the banner slot. It sits above the item's scroll region and stays
//! in place (R-47).
//!
//! The actions are drawn before the title and chips, in a row laid out in
//! reverse, so they sit on the right, wrap below the title on a narrow
//! pane, and still come first in the tab order (strip, header actions,
//! chips, page). After the pill, Comments {n} jumps to the discussion
//! (`discussion`), then Edit while viewing, or Done while editing
//! (`edit`). Share and the primary action are not available yet (decision
//! 16): each is disabled, with "Available in a later release" as its
//! tooltip and its accessible description.

use std::path::Path;

use gpui_kit::{
    AnyElement, App, Context, FontWeight, InteractiveElement, IntoElement, ParentElement, Role,
    StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{
        ActiveTheme, Disableable, Icon, Sizable,
        button::{Button, ButtonVariants},
        h_flex, v_flex,
    },
    div, px, relative,
};
use manyhands::repository::{ItemDto, ItemDtoKind};
use time::OffsetDateTime;

use super::{ItemPane, chips::chips, pill, text, tokens::item_colors};
use crate::desktop::{
    assets::icon_path,
    model::{
        item::{APPROVE_AND_MERGE, CLOSE_TICKET, LATER, LATER_NOTE, SHARE},
        navigation::OpenTarget,
        tabs::TabKey,
    },
};

/// The block's padding, vertical and horizontal.
const PADDING: (f32, f32) = (14., 28.);
/// The gap between the breadcrumb, the title row and the banners.
const GAP: f32 = 10.;
const BREADCRUMB_SIZE: f32 = 12.;
const CRUMB_ICON: f32 = 12.;
const CHEVRON: f32 = 11.;
const LEAF_SIZE: f32 = 11.;
const TITLE_SIZE: f32 = 20.;
/// The title column's basis before the actions wrap below it.
const TITLE_BASIS: f32 = 300.;
/// The breadcrumb's separator, in its accessible name.
pub(crate) const CRUMB_SEPARATOR: &str = " › ";
/// A ticket's section in the breadcrumb.
pub(crate) const TICKETS: &str = "Tickets";

/// The repository's name, as the tab strip names it: its folder.
fn repository_name(repo: &str) -> String {
    Path::new(repo)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| repo.to_owned())
}

/// `path`'s folder, when it has one, and file name.
pub(super) fn folder_and_file(path: &str) -> (Option<String>, String) {
    match path.rsplit_once('/') {
        Some((folder, file)) if !folder.is_empty() => (Some(folder.to_owned()), file.to_owned()),
        _ => (None, path.to_owned()),
    }
}

/// The breadcrumb's parts after the repository: a ticket's Tickets and
/// short code (its ID when it has none); a document's or a file's folder
/// and file name. A document not yet read shows its ID.
pub(crate) fn crumbs(target: &OpenTarget, item: Option<&ItemDto>) -> (Option<String>, String) {
    match target {
        OpenTarget::Item {
            kind: ItemDtoKind::Ticket,
            id,
            ..
        } => {
            let code = item
                .and_then(|item| item.slug.clone())
                .unwrap_or_else(|| id.to_string());
            (Some(TICKETS.to_owned()), code)
        }
        OpenTarget::Item { id, .. } => match item {
            Some(item) => folder_and_file(&item.path),
            None => (None, id.to_string()),
        },
        OpenTarget::Inspect { path, .. } => folder_and_file(path),
    }
}

/// How a later action's button is drawn: the header's primary action is
/// the warning (amber) button.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Weight {
    Plain,
    Primary,
}

/// Where a later action shows its note, "Available in a later release",
/// besides its accessible description: as the button's tooltip in the
/// header, or written below it, in `page-muted` on the discussion's light
/// ground.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum NoteAt {
    Tooltip,
    BelowOnPage,
}

/// A later action: `label`, one of `LATER`, disabled, the `ix`th of the
/// tab's later actions.
///
/// Its `("later-action", ix)` group carries the role, so assistive tech
/// reaches the label and the note as its description; the Kit's button
/// takes no description of its own. The `("later-note", ix)` element holds
/// the note for the tab's rows of notes.
pub(super) fn later_action(
    ix: usize,
    label: &'static str,
    icon: Option<&'static str>,
    weight: Weight,
    note: NoteAt,
    cx: &App,
) -> AnyElement {
    debug_assert!(LATER.contains(&label), "{label} is not a later action");
    let mut button = Button::new(("later-button", ix))
        .label(label)
        .small()
        .disabled(true);
    if let Some(icon) = icon {
        button = button.icon(Icon::default().path(icon_path(icon)));
    }
    if weight == Weight::Primary {
        button = button.warning();
    }
    let action = div()
        .id(("later-action", ix))
        .test_support()
        .role(Role::Group)
        .aria_label(label)
        .aria_description(LATER_NOTE);
    match note {
        NoteAt::BelowOnPage => v_flex()
            .gap_1()
            .child(action.child(button))
            .child(
                text(("later-note", ix), LATER_NOTE)
                    .text_xs()
                    .text_color(item_colors(cx).page_muted),
            )
            .into_any_element(),
        NoteAt::Tooltip => action
            .flex_none()
            .child(
                div()
                    .id(("later-note", ix))
                    .test_support()
                    .aria_label(LATER_NOTE)
                    .child(button.tooltip(LATER_NOTE)),
            )
            .into_any_element(),
    }
}

/// The header's later actions for an item of `kind`: Share, and the
/// primary action of its kind, when the kind is known. Edit is available
/// (E-6).
pub(crate) fn header_later(kind: Option<ItemDtoKind>) -> Vec<&'static str> {
    let mut actions = vec![SHARE];
    match kind {
        Some(ItemDtoKind::Ticket) => actions.push(CLOSE_TICKET),
        Some(ItemDtoKind::Document) => actions.push(APPROVE_AND_MERGE),
        None => {}
    }
    actions
}

impl ItemPane {
    /// The kind of the item shown: its tab's, or for a file read by path,
    /// what the read gave.
    pub(super) fn shown_kind(&self, item: Option<&ItemDto>) -> Option<ItemDtoKind> {
        match self.model.target() {
            OpenTarget::Item { kind, .. } => Some(*kind),
            OpenTarget::Inspect { .. } => item.map(|item| item.kind),
        }
    }

    /// The header block, and how many later actions it drew: the
    /// discussion's Comment comes after them. A new item not
    /// created yet shows its draft's fields as its title, pill, chips and
    /// breadcrumb (`new_item_shown`); its actions and banners are the tab's
    /// own, as with no item read.
    pub(super) fn header(
        &self,
        read: Option<&ItemDto>,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> (AnyElement, usize) {
        let colors = item_colors(cx);
        let later = header_later(self.shown_kind(read));
        // Edited repair text is unsaved, while its repair runs too, as the
        // strip marks it (Part E review, I1).
        let state = self.edit_pill(cx).or_else(|| {
            self.repair_edited(cx)
                .then_some(pill::PillState::EditingUnsaved)
        });
        // The read is borrowed; only a new item's stand-in is built.
        let built = read.is_none().then(|| self.new_item_shown(cx)).flatten();
        let item = read.or(built.as_ref());
        let pill = item.map(|item| {
            let state = state.unwrap_or_else(|| pill::viewing_state(item, self.draft_kept(cx)));
            // A stand-in was read from nowhere: no provenance (M2).
            pill::pill(state, read, cx)
        });
        let actions = h_flex()
            .id("item-actions")
            .flex_none()
            .flex_wrap()
            .items_center()
            .gap(px(8.))
            .children(pill)
            .children(self.comments_button(window, cx))
            .children(self.edit_action(read, window, cx))
            .children(
                later
                    .iter()
                    .enumerate()
                    .map(|(ix, label)| {
                        let icon = match *label {
                            SHARE => Some("cloud-upload"),
                            _ => None,
                        };
                        let weight = match *label {
                            CLOSE_TICKET | APPROVE_AND_MERGE => Weight::Primary,
                            _ => Weight::Plain,
                        };
                        later_action(ix, label, icon, weight, NoteAt::Tooltip, cx)
                    })
                    .collect::<Vec<_>>(),
            );
        let title = item.and_then(|item| item.title.clone()).map(|title| {
            text("item-title", title)
                .text_size(px(TITLE_SIZE))
                .font_weight(FontWeight::SEMIBOLD)
                .line_height(relative(1.25))
                .text_color(colors.header_title)
        });
        // An item read by path has no chips: it has no details sheet.
        let chips = match (self.model.target(), item) {
            (OpenTarget::Item { kind, .. }, Some(item)) => Some(self.chip_row(
                chips(
                    item,
                    *kind,
                    self.model.relationships(),
                    OffsetDateTime::now_utc(),
                ),
                window,
                cx,
            )),
            _ => None,
        };
        let identity = v_flex()
            .flex_grow_1()
            .flex_shrink_1()
            .flex_basis(px(TITLE_BASIS))
            .min_w_0()
            .gap(px(8.))
            .children(title)
            .children(chips);
        // Reversed, so the actions come first in the tab order but sit on
        // the right; wrapped in reverse, so on a narrow pane they fall below
        // the title. In a wrap-reverse row `items_end` aligns to the top, and
        // `justify_start`, the physical start, packs a wrapped line left.
        let row = div()
            .flex()
            .flex_row_reverse()
            .flex_wrap_reverse()
            .items_end()
            .justify_start()
            .gap(px(16.))
            .child(actions)
            .child(identity);
        let banners = self.banners(read, cx);
        let slot = (!banners.is_empty())
            .then(|| v_flex().id("item-banners").gap(px(6.)).children(banners));
        let (vertical, horizontal) = PADDING;
        let header = v_flex()
            .id("item-header")
            .test_support()
            .flex_none()
            .w_full()
            .py(px(vertical))
            .px(px(horizontal))
            .gap(px(GAP))
            .bg(colors.header_block)
            .border_b_1()
            .border_color(colors.header_border)
            .child(self.breadcrumb(item, cx))
            .child(row)
            .children(slot)
            .into_any_element();
        (header, later.len())
    }

    /// `folder-git-2`, then repository › section › file, at 12 px, with no
    /// wrap.
    fn breadcrumb(&self, item: Option<&ItemDto>, cx: &App) -> impl IntoElement + use<> {
        let theme = cx.theme();
        let (secondary, faint) = (theme.muted_foreground, theme.border);
        let mono = theme.mono_font_family.clone();
        let repository = repository_name(self.repo());
        let (section, leaf) = crumbs(self.model.target(), item);
        let name = [
            Some(repository.clone()),
            section.clone(),
            Some(leaf.clone()),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(CRUMB_SEPARATOR);
        let chevron = || {
            Icon::default()
                .path(icon_path("chevron-right"))
                .flex_none()
                .w(px(CHEVRON))
                .h(px(CHEVRON))
                .text_color(faint)
        };
        h_flex()
            .id("item-breadcrumb")
            .test_support()
            .aria_label(name)
            .gap(px(6.))
            .items_center()
            .overflow_hidden()
            .whitespace_nowrap()
            .text_size(px(BREADCRUMB_SIZE))
            .text_color(secondary)
            .child(
                Icon::default()
                    .path(icon_path("folder-git-2"))
                    .flex_none()
                    .w(px(CRUMB_ICON))
                    .h(px(CRUMB_ICON)),
            )
            .child(div().flex_none().child(repository))
            .children(section.map(|section| {
                h_flex()
                    .flex_none()
                    .gap(px(6.))
                    .items_center()
                    .child(chevron())
                    .child(section)
            }))
            .child(chevron())
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .font_family(mono)
                    .text_size(px(LEAF_SIZE))
                    .child(leaf),
            )
    }

    /// Whether a draft of this item is kept or stored: attached to this
    /// tab, or listed for it.
    pub(super) fn draft_kept(&self, cx: &App) -> bool {
        let tab = self.tab_key();
        let registry = self.drafts().read(cx);
        match &tab {
            TabKey::Item { repo, kind, id } => {
                registry.attached(&tab).is_some() || registry.stored_for(repo, *kind, id).is_some()
            }
            TabKey::Inspect { .. } => false,
        }
    }
}
