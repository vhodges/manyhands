//! The layout model: the default, the operations of Part B's table and
//! F6's stops.
//!
//! Layouts are written in a short notation. `arrangement` lists the sides
//! left, right, bottom, separated by `|`; a side's groups by `/`; a
//! group's tabs by `+`; the active tab, if not the first, ends in `*`.
//! Panels are `R`epositories, `T`ickets, `D`ocuments and `A`ctivity;
//! sides are `l`, `r` and `b`.

use gpui_kit::{
    component::dock::{DockAreaState, DockPlacement, DockState, PanelInfo, PanelState},
    px,
};

use super::{
    Arrangement, DEFAULT_LEFT_SPLIT, ITEM_AREA_NAME, Layout, PanelKind, Side, Stop,
    arrangement_from, f6_stops,
};

const PANELS: [(char, PanelKind); 4] = [
    ('R', PanelKind::Repositories),
    ('T', PanelKind::Tickets),
    ('D', PanelKind::Documents),
    ('A', PanelKind::Activity),
];
const SIDES: [(char, Side); 3] = [('l', Side::Left), ('r', Side::Right), ('b', Side::Bottom)];

fn panel(letter: char) -> PanelKind {
    PANELS.iter().find(|(l, _)| *l == letter).unwrap().1
}

fn letter(kind: PanelKind) -> char {
    PANELS.iter().find(|(_, k)| *k == kind).unwrap().0
}

fn side(letter: char) -> Side {
    SIDES.iter().find(|(l, _)| *l == letter).unwrap().1
}

/// A layout with every panel on and every dock shown, except `off` panels
/// and `hidden` docks.
fn layout(arrangement: &str, off: &str, hidden: &str) -> Layout {
    let mut layout = Layout {
        arrangement: Arrangement::parse(arrangement),
        ..Layout::default()
    };
    for letter in off.chars() {
        layout.on[panel(letter).index()] = false;
    }
    for letter in hidden.chars() {
        layout.shown[side(letter).index()] = false;
    }
    layout
}

impl Arrangement {
    fn parse(text: &str) -> Self {
        let mut arrangement = Arrangement::default();
        for (side, groups) in Side::ALL.into_iter().zip(text.split('|')) {
            for group in groups.split('/').filter(|group| !group.is_empty()) {
                let mut panels = Vec::new();
                let mut active = 0;
                for tab in group.split('+') {
                    panels.push(panel(tab.chars().next().unwrap()));
                    if tab.ends_with('*') {
                        active = panels.len() - 1;
                    }
                }
                arrangement.push_group(side, panels, active);
            }
        }
        arrangement
    }
}

/// What a layout reads as, in the table's columns: the panels that are on,
/// the docks shown, the panels displayed, the docks open, and each group's
/// selected tab (groups joined by `/`, sides by `|`).
#[derive(Debug, PartialEq)]
struct Read {
    on: String,
    shown: String,
    displayed: String,
    open: String,
    selected: String,
}

fn read(layout: &Layout) -> Read {
    let panels = |keep: &dyn Fn(PanelKind) -> bool| {
        PanelKind::ALL
            .into_iter()
            .filter(|kind| keep(*kind))
            .map(letter)
            .collect()
    };
    let sides = |keep: &dyn Fn(Side) -> bool| {
        SIDES
            .iter()
            .filter(|(_, side)| keep(*side))
            .map(|(letter, _)| *letter)
            .collect()
    };
    Read {
        on: panels(&|kind| layout.is_on(kind)),
        shown: sides(&|side| layout.is_shown(side)),
        displayed: panels(&|kind| layout.is_displayed(kind)),
        open: sides(&|side| layout.is_open(side)),
        selected: Side::ALL
            .into_iter()
            .map(|side| {
                layout
                    .groups(side)
                    .iter()
                    .map(|group| letter(group.active_panel().unwrap()).to_string())
                    .collect::<Vec<_>>()
                    .join("/")
            })
            .collect::<Vec<_>>()
            .join("|"),
    }
}

fn expect(on: &str, shown: &str, displayed: &str, open: &str, selected: &str) -> Read {
    Read {
        on: on.into(),
        shown: shown.into(),
        displayed: displayed.into(),
        open: open.into(),
        selected: selected.into(),
    }
}

#[test]
fn the_default_is_1b() {
    let layout = Layout::default();
    assert_eq!(
        read(&layout),
        expect("RTDA", "lrb", "RTDA", "lrb", "R/D|T|A")
    );
    // Left: Repositories over Documents, as two groups, 0.6 : 1.4.
    assert_eq!(
        layout
            .groups(Side::Left)
            .iter()
            .map(|g| g.panels.clone())
            .collect::<Vec<_>>(),
        [vec![PanelKind::Repositories], vec![PanelKind::Documents]]
    );
    assert_eq!(DEFAULT_LEFT_SPLIT, [0.6, 1.4]);
    // The names a saved layout knows the panels by, and their labels.
    assert_eq!(ITEM_AREA_NAME, "ItemAreaPanel");
    let names = PanelKind::ALL.map(|kind| {
        assert_eq!(PanelKind::from_name(kind.name()), Some(kind));
        (kind.name(), kind.label())
    });
    assert_eq!(
        names,
        [
            ("RepositoriesPanel", "Repositories"),
            ("TicketsPanel", "Tickets"),
            ("DocumentsPanel", "Documents"),
            ("ActivityPanel", "Activity"),
        ]
    );
    assert_eq!(
        [Side::Left, Side::Right, Side::Bottom].map(|side| layout.size(side)),
        [290., 440., 150.]
    );
    assert_eq!(
        f6_stops(&layout),
        [
            Stop::Panel(PanelKind::Repositories),
            Stop::Panel(PanelKind::Documents),
            Stop::ItemArea,
            Stop::Panel(PanelKind::Tickets),
            Stop::Panel(PanelKind::Activity),
        ]
    );
}

enum Op {
    TogglePanel(char),
    ToggleDock(char),
    Reveal(char),
    MoveTo(char, char),
}

#[test]
fn operations_follow_the_rules() {
    use Op::*;
    // (case, start, operation, then: on, shown, displayed, open, selected)
    let rows: Vec<(&str, Layout, Op, Read)> = vec![
        // P2: a displayed panel goes off; the rest keeps its state.
        (
            "P2 a displayed panel goes off",
            layout("R/D|T|A", "", ""),
            TogglePanel('D'),
            expect("RTA", "lrb", "RTA", "lrb", "R/D|T|A"),
        ),
        (
            "P2 the active tab goes off and the next on becomes active",
            layout("R+T*+D|A|", "", ""),
            TogglePanel('T'),
            expect("RDA", "lrb", "DA", "lr", "D|A|"),
        ),
        (
            "P2 the last tab goes off and the first on becomes active",
            layout("R+T+D*|A|", "", ""),
            TogglePanel('D'),
            expect("RTA", "lrb", "RA", "lr", "R|A|"),
        ),
        (
            "P2 the only tab on goes off and stays selected",
            layout("R+D*|T|A", "R", ""),
            TogglePanel('D'),
            expect("TA", "lrb", "TA", "rb", "D|T|A"),
        ),
        // P4: the last panel on in a dock goes off; the dock is no longer open.
        (
            "P4 the last panel on in a side dock",
            layout("R/D|T|A", "", ""),
            TogglePanel('T'),
            expect("RDA", "lrb", "RDA", "lb", "R/D|T|A"),
        ),
        (
            "P4 the last panel on in the bottom dock",
            layout("R/D|T|A", "", ""),
            TogglePanel('A'),
            expect("RTD", "lrb", "RTD", "lr", "R/D|T|A"),
        ),
        // P3: a panel not displayed is revealed.
        (
            "P3 a panel that is off",
            layout("R/D|T|A", "D", ""),
            TogglePanel('D'),
            expect("RTDA", "lrb", "RTDA", "lrb", "R/D|T|A"),
        ),
        (
            "P3 a panel in a dock that is not shown",
            layout("R/D|T|A", "", "r"),
            TogglePanel('T'),
            expect("RTDA", "lrb", "RTDA", "lrb", "R/D|T|A"),
        ),
        (
            "P3 an inactive tab that is on selects its tab",
            layout("R+D*|T|A", "", ""),
            TogglePanel('R'),
            expect("RTDA", "lrb", "RTA", "lrb", "R|T|A"),
        ),
        (
            "P3 an inactive tab that is off turns on and is selected",
            layout("R+D*|T|A", "R", ""),
            TogglePanel('R'),
            expect("RTDA", "lrb", "RTA", "lrb", "R|T|A"),
        ),
        // P5: a dock toggle closes the dock, and again opens it.
        (
            "P5 an open dock closes, which panels are on unchanged",
            layout("R/D|T|A", "", ""),
            ToggleDock('l'),
            expect("RTDA", "rb", "TA", "rb", "R/D|T|A"),
        ),
        (
            "P5 a closed dock opens with the same panels on",
            layout("R/D|T|A", "D", "l"),
            ToggleDock('l'),
            expect("RTA", "lrb", "RTA", "lrb", "R/D|T|A"),
        ),
        // P6, decision 6: a dock with every panel off shows them all.
        (
            "P6 a shown dock with every panel off",
            layout("R/D|T|A", "RD", ""),
            ToggleDock('l'),
            expect("RTDA", "lrb", "RTDA", "lrb", "R/D|T|A"),
        ),
        (
            "P6 a hidden dock with every panel off",
            layout("R/D|T|A", "T", "r"),
            ToggleDock('r'),
            expect("RTDA", "lrb", "RTDA", "lrb", "R/D|T|A"),
        ),
        // P7: a side with no panels has no toggle.
        (
            "P7 a side with no panels stays hidden",
            layout("R/D/T||A", "", "r"),
            ToggleDock('r'),
            expect("RTDA", "lb", "RTDA", "lb", "R/D/T||A"),
        ),
        // P8: a menu move puts the panel in a new group at the end of the
        // target dock, revealed.
        (
            "P8 into an open dock",
            layout("R/D|T|A", "", ""),
            MoveTo('T', 'l'),
            expect("RTDA", "lrb", "RTDA", "lb", "R/D/T||A"),
        ),
        (
            "P8 into a closed dock",
            layout("R/D|T|A", "", "l"),
            MoveTo('T', 'l'),
            expect("RTDA", "lrb", "RTDA", "lb", "R/D/T||A"),
        ),
        (
            "P8 into a side with no panels",
            layout("R/D/T||A", "", ""),
            MoveTo('R', 'r'),
            expect("RTDA", "lrb", "RTDA", "lrb", "D/T|R|A"),
        ),
        (
            "P8 from a closed dock, a panel that is off",
            layout("R/D|T|A", "T", "r"),
            MoveTo('T', 'b'),
            expect("RTDA", "lb", "RTDA", "lb", "R/D||A/T"),
        ),
        (
            "P8 out of a tab group leaves its active tab valid",
            layout("R+D*|T|A", "", ""),
            MoveTo('D', 'b'),
            expect("RTDA", "lrb", "RTDA", "lrb", "R|T|A/D"),
        ),
        // P9: the current side changes nothing.
        (
            "P9 the current side leaves a panel that is off, off and in its group",
            layout("R+D*|T|A", "R", "b"),
            MoveTo('R', 'l'),
            expect("TDA", "lr", "TD", "lr", "D|T|A"),
        ),
        (
            "P9 the current side keeps the active tab in its group",
            layout("R+D*|T|A", "", "b"),
            MoveTo('D', 'l'),
            expect("RTDA", "lr", "TD", "lr", "D|T|A"),
        ),
        // P13: a reveal turns the panel on, shows its dock and selects its tab.
        (
            "P13 a panel that is off in a hidden dock",
            layout("R/D|T|A", "A", "b"),
            Reveal('A'),
            expect("RTDA", "lrb", "RTDA", "lrb", "R/D|T|A"),
        ),
        (
            "P13 an inactive tab",
            layout("R+D*|T|A", "", ""),
            Reveal('R'),
            expect("RTDA", "lrb", "RTA", "lrb", "R|T|A"),
        ),
        (
            "P13 a panel already displayed",
            layout("R+D*|T|A", "", ""),
            Reveal('D'),
            expect("RTDA", "lrb", "TDA", "lrb", "D|T|A"),
        ),
    ];
    for (case, mut start, op, then) in rows {
        match op {
            TogglePanel(p) => start.toggle_panel(panel(p)),
            ToggleDock(s) => start.toggle_dock(side(s)),
            Reveal(p) => start.reveal(panel(p)),
            MoveTo(p, s) => start.move_to(panel(p), side(s)),
        }
        assert_eq!(read(&start), then, "{case}");
    }
}

fn leaf(kind: PanelKind) -> PanelState {
    PanelState::new(kind.name())
}

fn tabs(panels: &[PanelKind], active: usize) -> PanelState {
    PanelState {
        panel_name: "TabPanel".into(),
        children: panels.iter().copied().map(leaf).collect(),
        info: PanelInfo::tabs(active),
    }
}

fn stack(children: Vec<PanelState>) -> PanelState {
    PanelState {
        panel_name: "StackPanel".into(),
        info: PanelInfo::stack(vec![px(100.); children.len()], gpui_kit::Axis::Vertical),
        children,
    }
}

fn dock(panel: PanelState, placement: DockPlacement) -> DockState {
    DockState::new(panel, placement, px(200.), true)
}

fn stops(layout: &Layout) -> String {
    f6_stops(layout)
        .into_iter()
        .map(|stop| match stop {
            Stop::Panel(kind) => letter(kind),
            Stop::ItemArea => 'I',
        })
        .collect()
}

#[test]
fn f6_stops_follow_the_arrangement() {
    use PanelKind::*;
    // (case, layout, stops): `I` is the item area.
    let rows = [
        ("the default", layout("R/D|T|A", "", ""), "RDITA"),
        ("a hidden panel", layout("R/D|T|A", "R", ""), "DITA"),
        ("a hidden dock", layout("R/D|T|A", "", "r"), "RDIA"),
        (
            "a dock with every panel off",
            layout("R/D|T|A", "T", ""),
            "RDIA",
        ),
        ("a hidden bottom dock", layout("R/D|T|A", "", "b"), "RDIT"),
        (
            "a tabbed group shows its selected tab",
            layout("R+D*|T|A", "", ""),
            "DITA",
        ),
        (
            "the selected tab is off",
            layout("R+D*|T|A", "D", ""),
            "RITA",
        ),
        (
            "the bottom dock runs left to right",
            layout("R|T|A/D", "", ""),
            "RITAD",
        ),
        ("a side with no panels", layout("R/D/T||A", "", ""), "RDTIA"),
        ("nothing displayed", layout("R/D|T|A", "RTDA", ""), "I"),
    ];
    for (case, layout, stops_then) in rows {
        assert_eq!(stops(&layout), stops_then, "{case}");
    }

    // The arrangement is read from a dump: leaf names in order, each
    // group's active index, nested splits in order, anything else skipped.
    let state = DockAreaState {
        version: Some(1),
        center: leaf(Repositories),
        left_dock: Some(dock(
            stack(vec![
                tabs(&[Repositories, Documents], 1),
                tabs(&[Tickets], 0),
            ]),
            DockPlacement::Left,
        )),
        right_dock: Some(dock(
            stack(vec![stack(vec![tabs(&[Activity], 0)])]),
            DockPlacement::Right,
        )),
        bottom_dock: None,
    };
    let mut layout = Layout::default();
    layout.set_arrangement(arrangement_from(&state));
    assert_eq!(read(&layout).selected, "D/T|A|");
    assert_eq!(
        layout.groups(Side::Left)[0].panels,
        [Repositories, Documents]
    );
    // The bottom dock has no panels, so it is no stop and is not open.
    assert_eq!(stops(&layout), "DTIA");
    assert!(!layout.is_open(Side::Bottom));

    // An unknown leaf selected as the active tab, a bare known leaf at a
    // dock's root, and an empty group: the unknown leaf and the empty
    // group are skipped, and the active index follows the panels kept.
    let state = DockAreaState {
        version: Some(1),
        center: leaf(Repositories),
        left_dock: Some(dock(
            stack(vec![
                tabs(&[Repositories, Documents], 1),
                tabs(&[], 0),
                tabs(&[Activity], 0),
            ]),
            DockPlacement::Left,
        )),
        right_dock: Some(dock(leaf(Tickets), DockPlacement::Right)),
        bottom_dock: None,
    };
    let mut unknown = state.clone();
    unknown.left_dock = Some(dock(
        stack(vec![PanelState {
            panel_name: "TabPanel".into(),
            children: vec![
                leaf(Repositories),
                PanelState::new("ItemAreaPanel"),
                leaf(Documents),
            ],
            info: PanelInfo::tabs(1),
        }]),
        DockPlacement::Left,
    ));
    let arrangement = arrangement_from(&state);
    assert_eq!(arrangement.groups(Side::Left).len(), 2);
    assert_eq!(arrangement.groups(Side::Right)[0].panels, [Tickets]);
    assert_eq!(arrangement.groups(Side::Left)[0].active, 1);
    // The unknown leaf was selected, so the index now names the panel that
    // took its place.
    let from_unknown = arrangement_from(&unknown);
    let group = &from_unknown.groups(Side::Left)[0];
    assert_eq!(group.panels, [Repositories, Documents]);
    assert_eq!(group.active, 1);
}
