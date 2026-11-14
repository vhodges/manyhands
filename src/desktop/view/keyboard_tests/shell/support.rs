//! What the shell key tests share: the window, the layout, the regions and
//! footer controls as drawn, and the tour F6 makes.

use std::path::PathBuf;

use gpui_kit::{TestAppContext, test::TestWindowExt};

use super::super::Walk;
use crate::desktop::{
    model::layout::{Layout, PanelKind},
    test_support::fixtures::Fixture,
    view::{
        footer::{
            Control, attention_text, dock_toggle_name, panel_toggle_name, tests::support::layout,
        },
        harness::Shell,
        workspace::{REGIONS, Region},
    },
};

use PanelKind::{Activity, Documents, Repositories, Tickets};

/// A window over no repository: the default layout, focus on Repositories.
pub(super) fn plain(cx: &mut TestAppContext) -> Walk {
    Walk::over(Shell::open(Fixture::new(), cx), PathBuf::new())
}

pub(super) fn layout_of(walk: &Walk, cx: &mut TestAppContext) -> Layout {
    layout(&walk.shell, cx)
}

/// The region of the panel `kind`.
pub(super) fn region_of(kind: PanelKind) -> Region {
    match kind {
        Repositories => Region::Repositories,
        Documents => Region::Documents,
        Tickets => Region::Tickets,
        Activity => Region::Operations,
    }
}

/// The region that holds focus now, if one does.
pub(super) fn focused_region(walk: &Walk, cx: &mut TestAppContext) -> Option<Region> {
    walk.shell.frame(cx, |window, _| {
        REGIONS.into_iter().find(|region| {
            window.try_find(region.id()).and_then(|each| each.focused()) == Some(true)
        })
    })
}

/// Exactly `region` holds focus among the regions drawn (a hidden panel has
/// none), and it alone draws its ring, which is visible.
pub(super) fn ringed_on(walk: &Walk, region: Region, cx: &mut TestAppContext) {
    walk.shell.frame(cx, |window, _| {
        for each in REGIONS {
            let focused = window.try_find(each.id()).and_then(|r| r.focused());
            assert_eq!(
                focused == Some(true),
                each == region,
                "{each:?} holds focus"
            );
            let ring = window.try_find(each.ring_id());
            assert_eq!(ring.is_some(), each == region, "{each:?} draws a ring");
        }
        assert!(
            window.find(region.ring_id()).visible(),
            "the ring is visible"
        );
    });
}

/// The footer control `control` holds focus and draws its ring, which is
/// visible.
pub(super) fn on_control(walk: &Walk, control: Control, cx: &mut TestAppContext) {
    walk.shell.frame(cx, |window, _| {
        let id = control.id();
        assert_eq!(window.find(id).focused(), Some(true), "{id} holds focus");
        let ring = window.try_find((id, 1usize));
        assert!(ring.is_some_and(|ring| ring.visible()), "{id} draws a ring");
    });
}

/// Tab, from the item area, to `control`.
pub(super) fn reach(walk: &Walk, control: Control, cx: &mut TestAppContext) {
    walk.keys("secondary-3", cx);
    walk.tab_to("tab", control.id(), cx);
    on_control(walk, control, cx);
}

/// What the screen reader says of `control` while `layout` is shown: its
/// name and toggled state.
pub(super) fn said(control: Control, layout: &Layout, attention: usize) -> (String, Option<bool>) {
    match control {
        Control::Dock(side) => {
            let open = layout.is_open(side);
            (dock_toggle_name(side, open), Some(open))
        }
        Control::Panel(kind) => {
            let displayed = layout.is_displayed(kind);
            (panel_toggle_name(kind, displayed), Some(displayed))
        }
        Control::Attention => (attention_text(attention).unwrap_or_default(), None),
        Control::Settings => ("Settings".to_owned(), None),
        // Shown only where something is scheduled: the windows `said` serves
        // schedule nothing, and `status_items_say_and_choose` reads them.
        Control::UpdateCheck | Control::ToShare => {
            panic!("{control:?} is not shown in a window with nothing scheduled")
        }
    }
}

/// The regions F6 and Shift F6 visit, from the item area round to it.
pub(super) fn tour(walk: &Walk, key: &str, cx: &mut TestAppContext) -> Vec<Region> {
    walk.keys("secondary-3", cx);
    ringed_on(walk, Region::Content, cx);
    let mut seen = Vec::new();
    for _ in 0..REGIONS.len() {
        walk.keys(key, cx);
        match focused_region(walk, cx) {
            Some(Region::Content) => return seen,
            Some(region) => seen.push(region),
            None => panic!("{key} left every region"),
        }
    }
    panic!("{key} never came back to the item area: {seen:?}");
}
