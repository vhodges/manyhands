//! The saved layout file and the decision to use it (Part G; R-18, R-28).
//!
//! The file is JSON: `version`, `area` (the dock area's own dump), `on`
//! (each panel's on flag by its stable name) and `shown` (each side's shown
//! flag). [`restore`] is pure: it takes what the store read and answers
//! either a saved layout or the default, and whether the restore notice
//! shows. Anything the dock area could not carry out, or a person could not
//! undo, restores the default instead (decision 5): the area's `load` trusts
//! its input, so every rule is checked here.

use std::collections::BTreeMap;

use gpui_kit::component::dock::{DockAreaState, DockPlacement, PanelInfo, PanelState};
use serde::{Deserialize, Serialize};

use super::{ITEM_AREA_NAME, Layout, PanelKind, SPLIT, Side, TAB_GROUP, arrangement_from};
use crate::desktop::services::layout::LayoutStoreError;

/// The version written into the file and the area's dump. The file's own
/// `version` decides; the area's is set to it and not trusted (R-28).
pub(crate) const LAYOUT_VERSION: usize = 1;

/// A dock under this many pixels could not be dragged back out: the Kit's
/// `load` does not floor it (`B/dock/dock_area.rs:748-750`).
const MIN_DOCK: f32 = 100.;

/// No size a person could have made is over this: a bigger one is damage.
const MAX_SIZE: f32 = 1_000_000.;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    version: usize,
    area: DockAreaState,
    on: BTreeMap<String, bool>,
    shown: BTreeMap<String, bool>,
}

#[derive(Serialize)]
struct FileRef<'a> {
    version: usize,
    area: &'a DockAreaState,
    on: BTreeMap<&'static str, bool>,
    shown: BTreeMap<&'static str, bool>,
}

/// A layout read from the file, and the area state to load for it.
#[derive(Debug, PartialEq)]
pub(crate) struct Saved {
    pub(crate) layout: Layout,
    /// Every dock's `open` flag is as written and ignored: the launch sets
    /// each from the layout.
    pub(crate) area: DockAreaState,
}

/// What the launch does with the file.
#[derive(Debug, PartialEq)]
pub(crate) enum Restore {
    Saved(Box<Saved>),
    /// The default layout; `notice` says whether the restore notice shows.
    Default {
        notice: bool,
    },
}

fn side_key(side: Side) -> &'static str {
    match side {
        Side::Left => "left",
        Side::Right => "right",
        Side::Bottom => "bottom",
    }
}

/// The bytes of the file for `layout` and the area's dump.
pub(crate) fn encode(layout: &Layout, area: &DockAreaState) -> Vec<u8> {
    let file = FileRef {
        version: LAYOUT_VERSION,
        area,
        on: PanelKind::ALL
            .into_iter()
            .map(|kind| (kind.name(), layout.is_on(kind)))
            .collect(),
        shown: Side::ALL
            .into_iter()
            .map(|side| (side_key(side), layout.is_shown(side)))
            .collect(),
    };
    serde_json::to_vec(&file).expect("plain data serializes")
}

/// Part G's table: no file is the first launch, and shows no notice; every
/// other way the file fails restores the default and shows it.
pub(crate) fn restore(read: Result<Option<Vec<u8>>, LayoutStoreError>) -> Restore {
    match read {
        Ok(None) => Restore::Default { notice: false },
        Ok(Some(bytes)) => match parse(&bytes) {
            Some(saved) => Restore::Saved(Box::new(saved)),
            None => Restore::Default { notice: true },
        },
        Err(_) => Restore::Default { notice: true },
    }
}

fn parse(bytes: &[u8]) -> Option<Saved> {
    let file: File = serde_json::from_slice(bytes).ok()?;
    if file.version != LAYOUT_VERSION {
        return None;
    }
    let on = flags(&file.on, PanelKind::ALL.map(PanelKind::name))?;
    let shown = flags(&file.shown, Side::ALL.map(side_key))?;
    let mut area = file.area;

    // The centre holds the item area and nothing else.
    let mut centre = Vec::new();
    check_node(&area.center, &mut centre)?;
    if centre != [ITEM_AREA_NAME] {
        return None;
    }

    // Each of the four panels is in exactly one dock, and each dock sits in
    // its own slot, large enough to drag.
    let mut sizes = Layout::default().sizes;
    let mut seen: Vec<PanelKind> = Vec::new();
    let docks = [
        (Side::Left, DockPlacement::Left, &area.left_dock),
        (Side::Right, DockPlacement::Right, &area.right_dock),
        (Side::Bottom, DockPlacement::Bottom, &area.bottom_dock),
    ];
    for (side, placement, dock) in docks {
        let Some(dock) = dock else {
            continue;
        };
        let size = dock.size().as_f32();
        if dock.placement() != placement || !(MIN_DOCK..=MAX_SIZE).contains(&size) {
            return None;
        }
        let mut names = Vec::new();
        check_node(dock.panel(), &mut names)?;
        if names.is_empty() {
            return None;
        }
        for name in names {
            let kind = PanelKind::from_name(name)?;
            if seen.contains(&kind) {
                return None;
            }
            seen.push(kind);
        }
        sizes[side.index()] = size;
    }
    if seen.len() != PanelKind::ALL.len() {
        return None;
    }

    area.version = Some(LAYOUT_VERSION);
    let layout = Layout {
        on,
        shown,
        arrangement: arrangement_from(&area),
        sizes,
    };
    Some(Saved { layout, area })
}

/// `map` holds exactly `keys`, in `keys`' order.
fn flags<const N: usize>(map: &BTreeMap<String, bool>, keys: [&str; N]) -> Option<[bool; N]> {
    if map.len() != N {
        return None;
    }
    let mut out = [false; N];
    for (flag, key) in out.iter_mut().zip(keys) {
        *flag = *map.get(key)?;
    }
    Some(out)
}

/// Checks one container the way the Kit's `dump` writes it, and collects
/// the leaf names under it. The Kit reads a node by its `info`, so the name
/// must agree with it (`B/dock/state_convert.rs:126-185`); a leaf is valid
/// only inside a `TabPanel`.
fn check_node<'a>(node: &'a PanelState, leaves: &mut Vec<&'a str>) -> Option<()> {
    match &node.info {
        PanelInfo::Stack { sizes, axis } => {
            let sized = sizes.len() == node.children.len()
                && sizes
                    .iter()
                    .all(|s| s.as_f32() > 0. && s.as_f32() <= MAX_SIZE);
            if node.panel_name != SPLIT || *axis > 1 || node.children.is_empty() || !sized {
                return None;
            }
            node.children
                .iter()
                .try_for_each(|child| check_node(child, leaves))
        }
        PanelInfo::Tabs { active_index } => {
            if node.panel_name != TAB_GROUP
                || node.children.is_empty()
                || *active_index >= node.children.len()
            {
                return None;
            }
            for child in &node.children {
                let bare = matches!(child.info, PanelInfo::Panel(_))
                    && child.children.is_empty()
                    && child.panel_name != SPLIT
                    && child.panel_name != TAB_GROUP;
                if !bare {
                    return None;
                }
                leaves.push(&child.panel_name);
            }
            Some(())
        }
        PanelInfo::Panel(_) => None,
    }
}

/// The state of an area holding `layout`: a test's way to make a valid file.
/// A dock is open as the layout says; the left dock's two groups divide
/// 0.6 : 1.4, as the launch builds them.
#[cfg(test)]
pub(crate) fn state_for(layout: &Layout, sizes: [f32; 3]) -> DockAreaState {
    use gpui_kit::{Axis, component::dock::DockState, px};

    let tabs = |group: &super::Group| PanelState {
        panel_name: TAB_GROUP.into(),
        children: group
            .panels
            .iter()
            .map(|kind| PanelState::new(kind.name()))
            .collect(),
        info: PanelInfo::tabs(group.active),
    };
    let dock = |side: Side, placement: DockPlacement| {
        let groups = layout.groups(side);
        let root = match groups {
            [] => return None,
            [only] => tabs(only),
            groups => PanelState {
                panel_name: SPLIT.into(),
                info: PanelInfo::stack(
                    groups
                        .iter()
                        .enumerate()
                        .map(|(ix, _)| match (side, groups.len()) {
                            (Side::Left, 2) => px(super::DEFAULT_LEFT_SPLIT[ix] * 1000.),
                            _ => px(500.),
                        })
                        .collect(),
                    if side == Side::Bottom {
                        Axis::Horizontal
                    } else {
                        Axis::Vertical
                    },
                ),
                children: groups.iter().map(tabs).collect(),
            },
        };
        Some(DockState::new(
            root,
            placement,
            px(sizes[side.index()]),
            layout.is_open(side),
        ))
    };
    DockAreaState {
        version: Some(LAYOUT_VERSION),
        center: PanelState {
            panel_name: SPLIT.into(),
            info: PanelInfo::stack(vec![px(800.)], Axis::Horizontal),
            children: vec![PanelState {
                panel_name: TAB_GROUP.into(),
                children: vec![PanelState::new(ITEM_AREA_NAME)],
                info: PanelInfo::tabs(0),
            }],
        },
        left_dock: dock(Side::Left, DockPlacement::Left),
        right_dock: dock(Side::Right, DockPlacement::Right),
        bottom_dock: dock(Side::Bottom, DockPlacement::Bottom),
    }
}
