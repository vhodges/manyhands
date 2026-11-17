//! Share changes through (Wave 04 Cycle 05a design Part C; decisions 3,
//! 17 and 20; R-30): a Kit select (`add-share`) in the existing-folder
//! mode, under the main branch, whose items are made when the check lands:
//! one per remote the check found, its name and its fetch location as the
//! library redacts it, then Keep on this computer. A remote that is not an
//! SSH address is greyed with why. Drawn for any folder but a registered
//! one; the new mode has no remotes, so its form has no select.
//!
//! **Chosen when the check lands** (decision 17): the configuration's
//! sharing remote when it names one the check found and sharing can use,
//! else Keep on this computer, so nothing is set up to share unless the
//! person picks a remote. A folder already set up whose Git has no identity
//! is held on Keep, with why (R-30): a selection commits, and enabling a
//! folder already set up writes no identity, so it would fail every time.
//!
//! **Refused as the main branch is** (B7; R-12): a Kit select's disabled
//! item is only drawn disabled, so the delegate refuses it in
//! `on_will_change`, and a `Confirm` naming an item that cannot be chosen
//! is ignored, the select going back to the choice before it. The sheet
//! keeps the choice itself, so an add sends only a remote sharing can use.
//!
//! **What the add sends** once it succeeds (C2), decided when Add is
//! pressed: the remote chosen when it is not the configuration's; "stop
//! sharing" (`None`) for Keep over a configured remote the check found and
//! sharing can use, so the result agrees with what the sheet showed;
//! nothing for the configuration's own choice, for a configured remote the
//! folder lacks or sharing cannot use, or while the select is held. The
//! workspace sends it (`workspace::add_sheet`).

use gpui_kit::{
    App, AppContext, Context, Entity, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, Subscription, TestSupportExt, Window,
    component::{
        ActiveTheme, Icon, IndexPath, h_flex,
        searchable_list::SearchableListChange,
        select::{Select, SelectDelegate, SelectEvent, SelectItem, SelectState},
        v_flex,
    },
    div, px,
};
use manyhands::repository::{ConfigurationState, IdentityAvailability, RepositoryInspectionDto};

use super::{
    AddRepositorySheet, KEEP_LOCAL, SHARE_HELP, SHARE_NEEDS_IDENTITY, SHARE_THROUGH, SSH_ONLY,
    checks::Inspection,
};
use crate::desktop::{
    assets::icon_path,
    view::{
        settings::{field_column, wrapped},
        shell::tokens::chrome_colors,
    },
};

/// The select's wrapper, which tracks its focus.
pub(super) const SHARE_ID: &str = "add-share";

/// One item of the select: a remote of the folder, or Keep on this
/// computer.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Share {
    /// The remote's name; `None` is Keep on this computer.
    pub(super) remote: Option<String>,
    /// The remote's fetch location, as the library redacts it.
    pub(super) location: Option<String>,
    /// Whether sharing can use it. Keep on this computer always can.
    pub(super) eligible: bool,
}

impl SelectItem for Share {
    type Value = Option<String>;

    fn title(&self) -> SharedString {
        self.remote
            .clone()
            .unwrap_or_else(|| KEEP_LOCAL.to_owned())
            .into()
    }

    fn value(&self) -> &Option<String> {
        &self.remote
    }

    fn disabled(&self) -> bool {
        !self.eligible
    }

    /// The name, its fetch location in mono, and why sharing cannot use
    /// it.
    fn render(&self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let (colors, mono) = (chrome_colors(cx), cx.theme().mono_font_family.clone());
        let line = |text: &str| {
            div()
                .text_size(px(11.))
                .text_color(colors.muted)
                .child(text.to_owned())
        };
        v_flex()
            .child(self.title())
            .children(
                self.location
                    .as_deref()
                    .map(|location| line(location).font_family(mono)),
            )
            .children((!self.eligible).then(|| line(SSH_ONLY).whitespace_normal()))
    }
}

/// The select's items, which refuse a remote sharing cannot use.
#[derive(Default)]
pub(crate) struct Shares(Vec<Share>);

impl Shares {
    pub(super) fn new(items: Vec<Share>) -> Self {
        Self(items)
    }
}

impl SelectDelegate for Shares {
    type Item = Share;

    fn items_count(&self, _: usize) -> usize {
        self.0.len()
    }

    fn item(&self, ix: IndexPath) -> Option<&Share> {
        self.0.get(ix.row)
    }

    fn position<V>(&self, value: &V) -> Option<IndexPath>
    where
        Share: SelectItem<Value = V>,
        V: PartialEq,
    {
        self.0
            .iter()
            .position(|share| share.value() == value)
            .map(|ix| IndexPath::default().row(ix))
    }

    /// A single choice: the item selected last replaces the selection,
    /// unless sharing cannot use it, when the selection stays as it was.
    fn on_will_change(
        &mut self,
        selection: &mut Vec<(IndexPath, Share)>,
        changes: &[SearchableListChange],
    ) {
        let chosen = changes.iter().rev().find_map(|change| match change {
            SearchableListChange::Select { index } => Some(*index),
            SearchableListChange::Deselect { .. } => None,
        });
        let Some(index) = chosen else {
            return;
        };
        if let Some(share) = self.item(index).filter(|share| !share.disabled()) {
            *selection = vec![(index, share.clone())];
        }
    }
}

/// R-30: a folder already set up whose Git has no identity is held on
/// Keep on this computer.
fn held(check: &RepositoryInspectionDto) -> bool {
    check.configuration.state == ConfigurationState::Valid
        && check.identity_state == IdentityAvailability::Required
}

/// Whether `name` is a remote the check found that sharing can use.
fn usable(check: &RepositoryInspectionDto, name: Option<&str>) -> bool {
    name.is_some_and(|name| {
        check
            .remotes
            .iter()
            .any(|remote| remote.name == name && remote.publication_eligible)
    })
}

/// The items a check gives the select, and the one chosen when it lands
/// (decision 17): the remotes, then Keep on this computer.
fn items(inspection: &Inspection) -> (Vec<Share>, Option<String>) {
    let keep = Share {
        remote: None,
        location: None,
        eligible: true,
    };
    let Inspection::Checked { inspection, .. } = inspection else {
        return (vec![keep], None);
    };
    let mut items: Vec<Share> = inspection
        .remotes
        .iter()
        .map(|remote| Share {
            remote: Some(remote.name.clone()),
            location: Some(remote.fetch_location.clone()),
            eligible: remote.publication_eligible,
        })
        .collect();
    items.push(keep);
    let configured = inspection.configuration.publication_remote.as_deref();
    let chosen = (!held(inspection) && usable(inspection, configured))
        .then(|| configured.map(str::to_owned))
        .flatten();
    (items, chosen)
}

/// What an add of `check` with `chosen` sends once it succeeds (C2): a
/// remote's name, `None` to stop sharing, or nothing at all.
fn share_of(check: &RepositoryInspectionDto, chosen: Option<&str>) -> Option<Option<String>> {
    if held(check) {
        return None;
    }
    let configured = check.configuration.publication_remote.as_deref();
    match chosen {
        Some(name) if Some(name) != configured => Some(Some(name.to_owned())),
        Some(_) => None,
        None if usable(check, configured) => Some(None),
        None => None,
    }
}

/// The share select, empty, and the sheet following its `Confirm`.
pub(super) fn share_select(
    window: &mut Window,
    cx: &mut Context<AddRepositorySheet>,
) -> (Entity<SelectState<Shares>>, Subscription) {
    let select = cx.new(|cx| SelectState::new(Shares::default(), None, window, cx));
    let subscription = cx.subscribe_in(
        &select,
        window,
        |this, _, event: &SelectEvent<Shares>, window, cx| this.share_confirmed(event, window, cx),
    );
    (select, subscription)
}

impl AddRepositorySheet {
    /// The check is held on Keep on this computer (R-30).
    fn held_on_keep(&self) -> bool {
        matches!(&self.inspection, Inspection::Checked { inspection, .. } if held(inspection))
    }

    /// Gives the select the items of the check now, and chooses what the
    /// check chooses (decision 17).
    pub(super) fn sync_shares(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (items, chosen) = items(&self.inspection);
        self.shares = items.clone();
        self.share_choice = chosen;
        self.show_share_choice(Shares::new(items), window, cx);
    }

    /// The select shows the sheet's choice, among `items` when given.
    fn show_share_choice(
        &mut self,
        items: impl Into<Option<Shares>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (items, chosen) = (items.into(), self.share_choice.clone());
        self.share.update(cx, |select, cx| {
            if let Some(items) = items {
                select.set_items(items, window, cx);
            }
            select.set_selected_value(&chosen, window, cx);
        });
    }

    /// A `Confirm` naming an item the select can choose is the choice; any
    /// other is ignored, the select going back to the choice before it
    /// (R-12; R-30).
    fn share_confirmed(
        &mut self,
        event: &SelectEvent<Shares>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let SelectEvent::Confirm(named) = event;
        let allowed = named.as_ref().filter(|named| {
            (!self.held_on_keep() || named.is_none())
                && self
                    .shares
                    .iter()
                    .any(|share| share.remote == **named && share.eligible)
        });
        match allowed {
            Some(named) => self.share_choice = named.clone(),
            None => self.show_share_choice(None, window, cx),
        }
        cx.notify();
    }

    /// What an add confirmed now sends once it succeeds (C2).
    pub(super) fn share(&self) -> Option<Option<String>> {
        let Inspection::Checked { inspection, .. } = &self.inspection else {
            return None;
        };
        share_of(inspection, self.share_choice.as_deref())
    }

    /// Share changes through: its label, the select in its wrapper, its
    /// help and, held, why (C1; R-30). None for a registered folder.
    pub(super) fn share_field(&self, cx: &Context<Self>) -> Option<impl IntoElement + use<>> {
        if matches!(&self.inspection, Inspection::Checked { inspection, .. } if inspection.registered)
        {
            return None;
        }
        let colors = chrome_colors(cx);
        let is_held = self.held_on_keep();
        let select = h_flex()
            .gap(px(6.))
            .items_center()
            .child(
                Icon::default()
                    .path(icon_path("cloud"))
                    .text_color(colors.secondary)
                    .w(px(14.))
                    .h(px(14.)),
            )
            .child(
                div().flex_1().child(
                    Select::new(&self.share)
                        .id("add-share-select")
                        .accessibility_label(SHARE_THROUGH)
                        .disabled(is_held),
                ),
            );
        let line = |id: &'static str, text: &'static str| {
            div()
                .id(id)
                .test_support()
                .aria_label(text)
                .text_xs()
                .text_color(colors.secondary)
                .child(text)
        };
        Some(
            field_column()
                .gap_0p5()
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(SHARE_THROUGH),
                )
                .child(
                    wrapped(
                        SHARE_ID,
                        SHARE_THROUGH,
                        &self.focus[SHARE_ID],
                        &self.scroll,
                        select,
                    )
                    .w_full(),
                )
                .child(line("add-share-help", SHARE_HELP))
                .children(is_held.then(|| line("add-share-held", SHARE_NEEDS_IDENTITY))),
        )
    }

    /// The items the select was given last (P13).
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn share_items(&self) -> &[Share] {
        &self.shares
    }

    /// The share the select has chosen: a remote's name, or `None` for
    /// Keep on this computer; `None` when nothing is chosen (P13, P14).
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn chosen_share(&self, cx: &App) -> Option<Option<String>> {
        self.share.read(cx).selected_value().cloned()
    }

    /// The select's own state (P13's and P14's `Confirm` rows).
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn share_select(&self) -> Entity<SelectState<Shares>> {
        self.share.clone()
    }

    /// Whether the select is held on Keep on this computer (R-30; P14).
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn share_held(&self) -> bool {
        self.held_on_keep()
    }

    /// What an add confirmed now would send once it succeeds (C2; P13,
    /// P14).
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn share_to_send(&self) -> Option<Option<String>> {
        self.share()
    }
}
