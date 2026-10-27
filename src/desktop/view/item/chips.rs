//! The header's chips (design, Header block): each is a button that asks
//! for the details sheet on its field, by emitting `OpenDetails`; Task 6
//! opens the sheet from it.
//!
//! A ticket shows Type, Status (tinted for known values), Project, Team,
//! Parent by its short code, and Waiting on while it is blocked. A
//! document shows Status, Author and Date, each only when its key holds a
//! string (fact 58), and Updated, relative to `changed_at`. Both end with
//! All details. An item read by path has none.

use gpui_kit::{
    App, ClickEvent, Context, EventEmitter, FocusHandle, Hsla, InteractiveElement, IntoElement,
    ParentElement, Role, StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{ActiveTheme, Icon, h_flex},
    div, px,
};
use manyhands::repository::{ItemDto, ItemDtoKind, ReadinessReasonCode, ReadinessState};
use serde_json::Value;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use super::ItemPane;
use crate::desktop::{assets::icon_path, model::relationships::Relationships};

/// A chip's text.
const TEXT_SIZE: f32 = 11.5;
/// All details' `sliders-horizontal`.
const ICON_SIZE: f32 = 12.;
/// What a ticket waiting only on a cycle is waiting on.
pub(crate) const A_CYCLE: &str = "a dependency cycle";
pub(crate) const ALL_DETAILS: &str = "All details";
pub(crate) const JUST_NOW: &str = "just now";

/// The details sheet's field a chip opens it on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum ChipField {
    Type,
    Status,
    Project,
    Team,
    Parent,
    WaitingOn,
    Author,
    Date,
    Updated,
    All,
}

impl ChipField {
    /// Every field, in the order the pane keeps their focus.
    pub(crate) const FIELDS: [Self; 10] = [
        Self::Type,
        Self::Status,
        Self::Project,
        Self::Team,
        Self::Parent,
        Self::WaitingOn,
        Self::Author,
        Self::Date,
        Self::Updated,
        Self::All,
    ];

    /// The chip's key, before its value.
    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::Type => "Type",
            Self::Status => "Status",
            Self::Project => "Project",
            Self::Team => "Team",
            Self::Parent => "Parent",
            Self::WaitingOn => "Waiting on",
            Self::Author => "Author",
            Self::Date => "Date",
            Self::Updated => "Updated",
            Self::All => ALL_DETAILS,
        }
    }

    /// The chip's element ID.
    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::Type => "chip-type",
            Self::Status => "chip-status",
            Self::Project => "chip-project",
            Self::Team => "chip-team",
            Self::Parent => "chip-parent",
            Self::WaitingOn => "chip-waiting-on",
            Self::Author => "chip-author",
            Self::Date => "chip-date",
            Self::Updated => "chip-updated",
            Self::All => "chip-all",
        }
    }

    fn index(self) -> usize {
        Self::FIELDS
            .iter()
            .position(|field| *field == self)
            .unwrap_or_default()
    }
}

/// A chip asked for the details sheet on its field; Task 6 opens it.
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 6.
pub(crate) struct OpenDetails(pub ChipField);

impl EventEmitter<OpenDetails> for ItemPane {}

/// How a chip is drawn besides its key and value.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Look {
    Plain,
    /// A status the design tints, in its tone.
    Tinted(Tint),
    /// Waiting on: red.
    Blocked,
    /// All details: dashed, with its icon.
    All,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tint {
    Working,
    Done,
    Draft,
    Blocked,
}

/// One chip: its field and the value it shows. A value is a field's own
/// text, never body text.
pub(crate) struct Chip {
    pub field: ChipField,
    pub value: Option<String>,
    look: Look,
}

impl Chip {
    fn new(field: ChipField, value: String) -> Self {
        Self {
            field,
            value: Some(value),
            look: Look::Plain,
        }
    }

    /// The chip's accessible name: its key and value.
    pub(crate) fn label(&self) -> String {
        match &self.value {
            Some(value) => format!("{} {value}", self.field.key()),
            None => self.field.key().to_owned(),
        }
    }
}

/// The tint of a known status value, by its words.
fn tint(status: &str) -> Option<Tint> {
    match status.trim().to_lowercase().as_str() {
        "open" | "todo" | "to do" | "in progress" | "in-progress" | "in_progress" | "active" => {
            Some(Tint::Working)
        }
        "done" | "closed" | "complete" | "completed" | "approved" => Some(Tint::Done),
        "draft" | "review" | "in review" => Some(Tint::Draft),
        "blocked" => Some(Tint::Blocked),
        _ => None,
    }
}

fn status_chip(status: String) -> Chip {
    let look = match tint(&status) {
        Some(tint) => Look::Tinted(tint),
        None => Look::Plain,
    };
    Chip {
        look,
        ..Chip::new(ChipField::Status, status)
    }
}

/// When `at` was, from `now`: "just now", "{n} min ago", "1 hour ago",
/// "{n} hours ago", "1 day ago", "{n} days ago", then the date. A time
/// that cannot be read is shown as given; one after `now` is just now.
pub(crate) fn relative_time(at: &str, now: OffsetDateTime) -> String {
    let Ok(then) = OffsetDateTime::parse(at, &Rfc3339) else {
        return at.to_owned();
    };
    let seconds = (now - then).whole_seconds();
    let (minutes, hours, days) = (seconds / 60, seconds / 3600, seconds / 86_400);
    match () {
        _ if minutes < 1 => JUST_NOW.to_owned(),
        _ if hours < 1 => format!("{minutes} min ago"),
        _ if hours == 1 => "1 hour ago".to_owned(),
        _ if days < 1 => format!("{hours} hours ago"),
        _ if days == 1 => "1 day ago".to_owned(),
        _ if days < 30 => format!("{days} days ago"),
        _ => {
            let date = then.date();
            format!(
                "{:04}-{:02}-{:02}",
                date.year(),
                u8::from(date.month()),
                date.day()
            )
        }
    }
}

/// What a blocked ticket waits on: each open or unresolved dependency the
/// readiness names, by its short code once the tree named it, else its
/// ID; a cycle alone is "a dependency cycle".
fn waiting_on(item: &ItemDto, relationships: Option<&Relationships>) -> Option<String> {
    let readiness = item.readiness.as_ref()?;
    if readiness.state != ReadinessState::Blocked {
        return None;
    }
    let rows = relationships
        .map(Relationships::dependency_rows)
        .unwrap_or_default();
    let mut names: Vec<String> = Vec::new();
    for reason in &readiness.reasons {
        if reason.code == ReadinessReasonCode::DependencyCycle {
            continue;
        }
        for id in &reason.ids {
            let name = rows
                .iter()
                .find(|row| &row.id == id)
                .map(|row| row.name().short_or(id).to_owned())
                .unwrap_or_else(|| id.clone());
            if !names.contains(&name) {
                names.push(name);
            }
        }
    }
    Some(if names.is_empty() {
        A_CYCLE.to_owned()
    } else {
        names.join(", ")
    })
}

/// A document's other key as a chip value, when it holds a string.
fn string_key(item: &ItemDto, key: &str) -> Option<String> {
    match item.unknown_metadata.get(key)? {
        Value::String(value) => Some(value.clone()),
        _ => None,
    }
}

/// The chips of `item`, read as `kind`, in the design's order, ending
/// with All details.
pub(crate) fn chips(
    item: &ItemDto,
    kind: ItemDtoKind,
    relationships: Option<&Relationships>,
    now: OffsetDateTime,
) -> Vec<Chip> {
    let mut chips = Vec::new();
    match kind {
        ItemDtoKind::Ticket => {
            if let Some(value) = item.ticket_type.clone() {
                chips.push(Chip::new(ChipField::Type, value));
            }
            if let Some(status) = item.status.clone() {
                chips.push(status_chip(status));
            }
            if let Some(value) = item.project.clone() {
                chips.push(Chip::new(ChipField::Project, value));
            }
            if let Some(value) = item.team.clone() {
                chips.push(Chip::new(ChipField::Team, value));
            }
            if let Some(parent) = &item.parent {
                let named = relationships.and_then(Relationships::parent_row);
                let value = named
                    .as_ref()
                    .map(|row| row.name().short_or(&row.id).to_owned())
                    .unwrap_or_else(|| parent.id.clone());
                chips.push(Chip::new(ChipField::Parent, value));
            }
            if let Some(value) = waiting_on(item, relationships) {
                chips.push(Chip {
                    look: Look::Blocked,
                    ..Chip::new(ChipField::WaitingOn, value)
                });
            }
        }
        ItemDtoKind::Document => {
            if let Some(status) = string_key(item, "status") {
                chips.push(status_chip(status));
            }
            if let Some(value) = string_key(item, "author") {
                chips.push(Chip::new(ChipField::Author, value));
            }
            if let Some(value) = string_key(item, "date") {
                chips.push(Chip::new(ChipField::Date, value));
            }
            if let Some(at) = &item.changed_at {
                chips.push(Chip::new(ChipField::Updated, relative_time(at, now)));
            }
        }
    }
    chips.push(Chip {
        field: ChipField::All,
        value: None,
        look: Look::All,
    });
    chips
}

/// One focus handle per field, each a tab stop, kept by the pane so the
/// sheet can give focus back to the chip that opened it.
pub(super) fn chip_focus(cx: &mut App) -> Vec<FocusHandle> {
    ChipField::FIELDS
        .iter()
        .map(|_| cx.focus_handle().tab_stop(true))
        .collect()
}

impl ItemPane {
    /// The chips, wrapping, gap 6.
    pub(super) fn chip_row(
        &self,
        chips: Vec<Chip>,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let chips: Vec<_> = chips
            .into_iter()
            .map(|chip| self.chip(chip, window, cx))
            .collect();
        h_flex()
            .id("item-chips")
            .flex_wrap()
            .gap(px(6.))
            .children(chips)
    }

    fn chip(
        &self,
        chip: Chip,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let theme = cx.theme();
        let (secondary, ring, hover) = (theme.muted_foreground, theme.ring, theme.info);
        let field = chip.field;
        let focus = self.chip_focus[field.index()].clone();
        let focused = focus.is_focused(window);
        let (fill, text): (Option<Hsla>, Hsla) = match chip.look {
            Look::Plain => (Some(theme.secondary), theme.foreground),
            Look::Tinted(tint) => {
                let tone = match tint {
                    Tint::Working => theme.info,
                    Tint::Done => theme.success,
                    Tint::Draft => theme.warning,
                    Tint::Blocked => theme.danger,
                };
                (Some(tone.opacity(0.13)), tone)
            }
            Look::Blocked => (Some(theme.danger.opacity(0.13)), theme.danger),
            Look::All => (None, secondary),
        };
        let label = chip.label();
        let ring = focused.then(|| {
            div()
                .absolute()
                .top(px(-3.))
                .bottom(px(-3.))
                .left(px(-3.))
                .right(px(-3.))
                .rounded(px(6.))
                .border_2()
                .border_color(ring)
        });
        let mut element = div()
            .id(field.id())
            .test_support()
            .role(Role::Button)
            .aria_label(label)
            .track_focus(&focus)
            .relative()
            .flex()
            .flex_none()
            .gap(px(5.))
            .items_center()
            .px(px(8.))
            .py(px(2.))
            .rounded(px(4.))
            .border_1()
            .border_color(theme.input)
            .text_color(text)
            .text_size(px(TEXT_SIZE))
            .whitespace_nowrap()
            .cursor_pointer()
            .hover(move |style| style.border_color(hover))
            .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| cx.emit(OpenDetails(field))));
        if let Some(fill) = fill {
            element = element.bg(fill);
        }
        element = match chip.value {
            Some(value) => element
                .child(div().text_color(secondary).child(field.key()))
                .child(value),
            None => element
                .border_dashed()
                .child(
                    Icon::default()
                        .path(icon_path("sliders-horizontal"))
                        .w(px(ICON_SIZE))
                        .h(px(ICON_SIZE)),
                )
                .child(field.key()),
        };
        element.children(ring)
    }
}
