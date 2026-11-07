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
//! (`edit`), then Share (`share`), then the item's lifecycle action: Close
//! ticket… on a ticket the lifecycle has not closed, Approve and merge… on a
//! document (D5, A1). It opens the lifecycle dialog (`view::lifecycle`,
//! A2), the tab's unsaved edits saved first; on a tab whose item has no ID,
//! a new item not created yet or a nonconforming file, it is disabled and
//! says why.

use std::path::{Path, PathBuf};

use gpui_kit::{
    AnyElement, App, ClickEvent, Context, FontWeight, InteractiveElement, IntoElement,
    ParentElement, StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{
        ActiveTheme, Disableable, Icon, Sizable, WindowExt,
        button::{Button, ButtonVariants},
        h_flex, v_flex,
    },
    div, px, relative,
};
use manyhands::{
    canonical::ItemId,
    repository::{ClosureState, ItemDto, ItemDtoKind},
};
use time::OffsetDateTime;

use super::{ItemPane, SaveFirst, chips::chips, pill, text, tokens::item_colors};
use crate::desktop::{
    assets::icon_path,
    model::{
        edit::Phase,
        item::{APPROVE_AND_MERGE, CLOSE_TICKET},
        navigation::OpenTarget,
        tabs::TabKey,
    },
    services::requests::Intent,
    view::lifecycle::{self, Heading, Opening},
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
/// The header's lifecycle action: Close ticket… or Approve and merge….
pub(crate) const LIFECYCLE_ACTION: &str = "item-lifecycle-action";
/// The lifecycle action's wrapper, which tracks its focus.
const LIFECYCLE_FOCUS: &str = "item-lifecycle-focus";
/// Why the lifecycle action is disabled, as an element that carries it as
/// its label.
pub(crate) const LIFECYCLE_NOTE: &str = "item-lifecycle-note";
/// A new item not created yet has no ID to close or merge.
pub(crate) const NOT_CREATED: &str = "Save this new item first: it is not created yet.";
/// A nonconforming file has no ID until it is repaired.
pub(crate) const NO_ID: &str = "This file has no ID until it is repaired.";

/// What the lifecycle action acts on.
enum LifecycleTarget {
    /// The item as read, by its ID.
    Ready {
        intent: Box<Intent>,
        heading: Heading,
    },
    /// No ID: why.
    NoId(&'static str),
    /// Not read yet.
    NotRead,
}

/// The repository's name, as the tab strip names it: its folder.
pub(super) fn repository_name(repo: &str) -> String {
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

/// The header's lifecycle action for an item of `kind`, as `item` reads:
/// Close ticket… for a ticket, unless the lifecycle closed it (its closure,
/// not its status text, says `closed`), and Approve and merge… for a
/// document. None while the kind is not known.
pub(crate) fn lifecycle_action(
    kind: Option<ItemDtoKind>,
    item: Option<&ItemDto>,
) -> Option<&'static str> {
    match kind? {
        ItemDtoKind::Ticket => {
            let closed = item
                .and_then(|item| item.closure.as_ref())
                .is_some_and(|closure| closure.state == ClosureState::Closed);
            (!closed).then_some(CLOSE_TICKET)
        }
        ItemDtoKind::Document => Some(APPROVE_AND_MERGE),
    }
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

    /// The header block. A new item not
    /// created yet shows its draft's fields as its title, pill, chips and
    /// breadcrumb (`new_item_shown`); its actions and banners are the tab's
    /// own, as with no item read.
    pub(super) fn header(
        &self,
        read: Option<&ItemDto>,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let colors = item_colors(cx);
        // What is unsaved is what the strip marks (`unsaved`): edited repair
        // text, while its repair runs too, in the editing words (Part E
        // review, I1), and a draft kept or stored in the viewing words.
        let state = self.edit_pill(cx).or_else(|| {
            self.repair_edited(cx)
                .then_some(pill::PillState::EditingUnsaved)
        });
        // The read is borrowed; only a new item's stand-in is built.
        let built = read.is_none().then(|| self.new_item_shown(cx)).flatten();
        let item = read.or(built.as_ref());
        // A run of the item, or its record, says so first (D5 A2); its
        // dialog, open in front of the tab, says so itself.
        let progress = self
            .run_locks(cx)
            .then(|| self.refused_words().to_owned())
            .or_else(|| self.share_progress(cx));
        let pill = item.map(|item| {
            // A running share or post of the item says its stage.
            if let Some(words) = progress {
                return pill::progress_pill(words, read, cx).into_any_element();
            }
            let viewing = || pill::viewing_state(item, self.unsaved(cx));
            // A finished share of the item says its outcome while no edit
            // session says more and the item reads as the share left it,
            // its shared or its private copy, or it stopped on a conflict:
            // a kept draft, a closed ticket, an item that needs attention
            // and a read changed since say theirs.
            if state.is_none()
                && let Some(outcome) = self.share_outcome_shown(read)
                && (outcome.conflict
                    || matches!(
                        viewing(),
                        pill::PillState::ViewingShared | pill::PillState::ViewingPrivate
                    ))
            {
                return pill::outcome_pill(&outcome.words, outcome.attention, read, cx)
                    .into_any_element();
            }
            let state = state.unwrap_or_else(viewing);
            // A stand-in was read from nowhere: no provenance (M2).
            pill::pill(state, read, cx).into_any_element()
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
            .children(self.share_button(window, cx))
            .children(self.lifecycle_button(item, cx));
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
        v_flex()
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
            .into_any_element()
    }

    /// The lifecycle action's button, when the item offers one: live once
    /// the item is read with its ID. A new item not created yet, and a
    /// nonconforming file, have no ID: the action is drawn disabled, saying
    /// why. While the item is not read yet it waits, disabled.
    fn lifecycle_button(
        &self,
        item: Option<&ItemDto>,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let label = lifecycle_action(self.shown_kind(item), item)?;
        let target = self.lifecycle_target(cx);
        let ready = matches!(target, LifecycleTarget::Ready { .. }) && !self.lifecycle_busy(cx);
        let mut button = Button::new(LIFECYCLE_ACTION)
            .label(label)
            .small()
            .warning()
            .disabled(!ready)
            .on_click(cx.listener(|pane, _: &ClickEvent, window, cx| {
                pane.open_lifecycle(window, cx);
            }));
        let note = match target {
            LifecycleTarget::NoId(reason) => {
                button = button.tooltip(reason);
                Some(reason)
            }
            _ => None,
        };
        // The wrapper tracks the action's focus, so its dialog gives focus
        // back here (A2, A21), as the operation area's buttons do. Focused,
        // it shows a ring, and Enter or Space opens the dialog again; Tab
        // goes on to the Kit button inside. A pointer's click is the
        // button's only.
        let ring = cx.theme().ring;
        let wrapper = div()
            .id(LIFECYCLE_FOCUS)
            .flex_none()
            .rounded_sm()
            .track_focus(&self.lifecycle_focus)
            .focus(move |style| style.border_2().border_color(ring))
            .on_click(cx.listener(|pane, event: &ClickEvent, window, cx| {
                if event.is_keyboard() {
                    pane.open_lifecycle(window, cx);
                }
            }));
        let wrapper = match note {
            Some(reason) => wrapper.child(
                div()
                    .id(LIFECYCLE_NOTE)
                    .test_support()
                    .aria_label(reason)
                    .child(button),
            ),
            None => wrapper.child(button),
        };
        Some(wrapper.into_any_element())
    }

    /// The header's lifecycle action's focus, where its dialog gives focus
    /// back.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn lifecycle_focus(&self) -> gpui_kit::FocusHandle {
        self.lifecycle_focus.clone()
    }

    /// What the lifecycle action acts on: the item as read, by its ID.
    fn lifecycle_target(&self, cx: &App) -> LifecycleTarget {
        if self.shows_new_item(cx) {
            return LifecycleTarget::NoId(NOT_CREATED);
        }
        let Some(item) = self.model.item().rows() else {
            return LifecycleTarget::NotRead;
        };
        let Some(id) = item.id.as_deref().and_then(|id| id.parse::<ItemId>().ok()) else {
            return LifecycleTarget::NoId(NO_ID);
        };
        let root = PathBuf::from(self.repo());
        let intent = match item.kind {
            ItemDtoKind::Ticket => Intent::TicketClose { root, id },
            ItemDtoKind::Document => Intent::DocumentPromote { root, id },
        };
        let mut heading = Heading::of(
            item,
            item.id.as_deref().unwrap_or_default(),
            repository_name(self.repo()),
        );
        // R-35: the dialog names the primary branch before a preview does.
        heading.primary = self.primary_branch().map(str::to_owned);
        LifecycleTarget::Ready {
            intent: Box::new(intent),
            heading,
        }
    }

    /// Close ticket… or Approve and merge… pressed: the tab saves its
    /// unsaved edits first, then the lifecycle dialog prepares (D5, A2). A
    /// tab whose item has no ID opens nothing; neither does a save that
    /// could not start, the tab saying why.
    fn open_lifecycle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let LifecycleTarget::Ready { intent, heading } = self.lifecycle_target(cx) else {
            return;
        };
        if self.lifecycle_busy(cx) {
            return;
        }
        if window.has_active_dialog(cx) {
            return;
        }
        let saving = match self.start_save_first(cx) {
            Some(SaveFirst::Failed) => return,
            Some(_) => Some(cx.entity().downgrade()),
            None => None,
        };
        let origin = self.lifecycle_focus.clone();
        let opening = Opening {
            intent: *intent,
            heading,
            origin,
            saving,
            following: None,
            repaired: false,
        };
        lifecycle::open(opening, &self.services, window, cx);
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

    /// Whether the tab holds what is not saved: the strip's unsaved mark,
    /// which the header's pill agrees with (Task 27 fix 1, B3). While the
    /// tab is Editing, it is the session's text differing from its base,
    /// as the pill's "Editing · unsaved changes" says, however a draft of
    /// it stands: a draft at its base holds nothing unsaved (ruling D1;
    /// N3's close dialog says so too). Otherwise it is that, a draft of the
    /// item kept or stored, or edited repair text.
    pub(in crate::desktop::view) fn unsaved(&self, cx: &App) -> bool {
        if self.edit.phase() == Phase::Editing {
            return self.edit_dirty(cx) || self.repair_edited(cx);
        }
        self.unsaved_edit(cx) || self.draft_kept(cx) || self.repair_edited(cx)
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
