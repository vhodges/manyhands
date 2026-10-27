//! The item's header block (design, Header block; `item_view/01-item-frame.md`):
//! the breadcrumb, the title, the chips, the actions with the state pill,
//! and the banner slot. It sits above the item's scroll region and stays
//! in place (R-47).
//!
//! The actions are drawn before the title and chips, in a row laid out in
//! reverse, so they sit on the right, wrap below the title on a narrow
//! pane, and still come first in the tab order (strip, header actions,
//! chips, page). Edit, Share and the primary action are not available yet
//! (decision 16): each is disabled, with "Available in a later release"
//! as its tooltip and its note.

use std::path::Path;

use gpui_kit::{
    AnyElement, App, Context, FontWeight, InteractiveElement, IntoElement, ParentElement,
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
        item::{LATER, LATER_NOTE},
        navigation::OpenTarget,
        tabs::TabKey,
    },
};

/// The header's later actions, by their place in `LATER`.
const EDIT: &str = LATER[0];
const SHARE: &str = LATER[2];
const APPROVE: &str = LATER[3];
const CLOSE: &str = LATER[4];
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
fn folder_and_file(path: &str) -> (Option<String>, String) {
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

/// A later action: `label`, disabled, `ix`th of the tab's later actions,
/// with "Available in a later release" as its note. In the header the
/// note is its tooltip; elsewhere (`text_note`) it is written below it.
pub(super) fn later_action(
    ix: usize,
    label: &'static str,
    icon: Option<&'static str>,
    primary: bool,
    text_note: bool,
    cx: &App,
) -> AnyElement {
    let mut button = Button::new(("later-button", ix))
        .label(label)
        .small()
        .disabled(true);
    if let Some(icon) = icon {
        button = button.icon(Icon::default().path(icon_path(icon)));
    }
    if primary {
        button = button.warning();
    }
    if text_note {
        let muted = cx.theme().muted_foreground;
        v_flex()
            .gap_1()
            .child(
                div()
                    .id(("later-action", ix))
                    .test_support()
                    .aria_label(label)
                    .child(button),
            )
            .child(
                text(("later-note", ix), LATER_NOTE)
                    .text_xs()
                    .text_color(muted),
            )
            .into_any_element()
    } else {
        div()
            .id(("later-action", ix))
            .test_support()
            .aria_label(label)
            .flex_none()
            .child(
                div()
                    .id(("later-note", ix))
                    .test_support()
                    .aria_label(LATER_NOTE)
                    .child(button.tooltip(LATER_NOTE)),
            )
            .into_any_element()
    }
}

/// The header's later actions for an item of `kind`: Edit, Share, and the
/// primary action of its kind, when the kind is known.
pub(crate) fn header_later(kind: Option<ItemDtoKind>) -> Vec<&'static str> {
    let mut actions = vec![EDIT, SHARE];
    match kind {
        Some(ItemDtoKind::Ticket) => actions.push(CLOSE),
        Some(ItemDtoKind::Document) => actions.push(APPROVE),
        None => {}
    }
    actions
}

impl ItemPane {
    /// The kind of the item shown: its tab's, or for a file read by path,
    /// what the read gave.
    fn shown_kind(&self, item: Option<&ItemDto>) -> Option<ItemDtoKind> {
        match self.model.target() {
            OpenTarget::Item { kind, .. } => Some(*kind),
            OpenTarget::Inspect { .. } => item.map(|item| item.kind),
        }
    }

    /// The header block, and how many later actions it and its banners
    /// drew: the discussion's Comment comes after them.
    pub(super) fn header(
        &self,
        item: Option<&ItemDto>,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> (AnyElement, usize) {
        let colors = item_colors(cx);
        let later = header_later(self.shown_kind(item));
        let pill =
            item.map(|item| pill::pill(pill::viewing_state(item, self.draft_kept(cx)), item, cx));
        let actions = h_flex()
            .id("item-actions")
            .flex_none()
            .flex_wrap()
            .items_center()
            .gap(px(8.))
            .children(pill)
            .children(
                later
                    .iter()
                    .enumerate()
                    .map(|(ix, label)| {
                        let icon = match *label {
                            EDIT => Some("pencil"),
                            SHARE => Some("cloud-upload"),
                            _ => None,
                        };
                        let primary = matches!(*label, CLOSE | APPROVE);
                        later_action(ix, label, icon, primary, false, cx)
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
        let (banners, banner_later) = self.banners(item, later.len(), cx);
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
        (header, later.len() + banner_later)
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
