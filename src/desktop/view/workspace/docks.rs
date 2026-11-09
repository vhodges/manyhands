//! The layout operations and the dock area's event handler (Part C).
//!
//! Each operation changes the layout, then reconciles the area with it,
//! checks focus and schedules a save (Part B). An operation that changes
//! nothing does none of those: it moves no focus and saves nothing.

use gpui_kit::{Context, Window, component::dock::DockEvent};

use super::Workspace;
use crate::desktop::{
    model::layout::{Layout, PanelKind, Side, arrangement_from},
    view::dock::{
        Slot,
        accepted::{centre_is_item_area, open_in, same_layout},
    },
};

impl Workspace {
    /// A panel toggle: a displayed panel goes off, any other is revealed.
    pub(in crate::desktop::view) fn toggle_panel(
        &mut self,
        kind: PanelKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.change_layout(window, cx, |layout| layout.toggle_panel(kind));
    }

    /// A dock toggle.
    pub(in crate::desktop::view) fn toggle_dock(
        &mut self,
        side: Side,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.change_layout(window, cx, |layout| layout.toggle_dock(side));
    }

    /// The panel goes on, its dock is shown and its tab is selected.
    pub(in crate::desktop::view) fn reveal(
        &mut self,
        kind: PanelKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.change_layout(window, cx, |layout| layout.reveal(kind));
    }

    /// The dock menu: the panel moves to the end of `side` as a new group
    /// and is revealed (R-5). Its current side changes nothing.
    ///
    /// The area makes the move and the layout reads the arrangement back
    /// from it, as it does after a drag, which reveals a panel whose side
    /// changed (R-36). `Layout::move_to` is the pure model of this move,
    /// and a test holds the two together.
    pub(in crate::desktop::view) fn dock_panel(
        &mut self,
        kind: PanelKind,
        side: Side,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.layout.side_of(kind) == Some(side) {
            return;
        }
        let before = self.layout.clone();
        self.dock
            .move_to_side(kind, side, &mut self.layout, window, cx);
        let dump = self.dock.area().read(cx).dump(cx);
        self.layout.set_arrangement(arrangement_from(&dump));
        if self.layout != before {
            self.settle(window, cx);
        }
    }

    /// A tab on the closed bottom strip: the panel is revealed, which opens
    /// the dock, and takes focus, as a tab click does (R-35, P12).
    pub(in crate::desktop::view) fn reveal_and_focus(
        &mut self,
        kind: PanelKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.reveal(kind, window, cx);
        self.focus_region(Slot::Panel(kind).region(), window, cx);
    }

    fn change_layout(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        edit: impl FnOnce(&mut Layout),
    ) {
        let before = self.layout.clone();
        edit(&mut self.layout);
        if self.layout != before {
            self.settle(window, cx);
        }
    }

    /// The tail of every operation: reconcile, the focus check, a save.
    fn settle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.dock.reconcile(&mut self.layout, window, cx);
        self.check_focus(window, cx);
        self.dock.request_save();
        cx.notify();
    }

    /// After each `LayoutChanged` (R-6). The same for every event, and
    /// idempotent, since GPUI delivers an event after the edit that caused
    /// it and the shell cannot tell its own edits from the person's.
    pub(in crate::desktop::view) fn dock_event(
        &mut self,
        event: &DockEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !matches!(event, DockEvent::LayoutChanged) {
            return;
        }
        let area = self.dock.area().clone();
        let dump = area.read(cx).dump(cx);
        // 1. Nothing the shell has not accepted.
        if same_layout(self.dock.accepted(), &dump) {
            return;
        }
        // 2. A drop landed on the item area: the layout as it was. This
        // resets focus and element state in the area (K-2).
        if !centre_is_item_area(&dump) {
            let accepted = self.dock.accepted().clone();
            // The Kit's `load` always answers `Ok` (fact 5). Were it ever to
            // fail, the dropped panel would stay in the centre with no
            // further event to put it back.
            let loaded = area.update(cx, |area, cx| area.load(accepted, window, cx));
            debug_assert!(loaded.is_ok(), "the accepted layout loads");
            return;
        }
        // 3. Accept it. A dock the Kit opened (an edge drag) counts as its
        // dock toggle. This is read before the arrangement, which can
        // change what the layout calls open, against the accepted state, so
        // the shell's own toggles are not counted twice. Each term is
        // needed: a dock the layout closed is not opened by an edit
        // elsewhere (the dump says open), and a dock the shell opens is not
        // toggled again (the layout says open). Term 1 is defensive:
        // reconcile keeps the accepted state's open flag equal to the
        // layout's, so no state makes it the only term that fails.
        let kit_opened: Vec<Side> = Side::ALL
            .into_iter()
            .filter(|side| {
                !open_in(self.dock.accepted(), *side)
                    && open_in(&dump, *side)
                    && !self.layout.is_open(*side)
            })
            .collect();
        self.layout.set_arrangement(arrangement_from(&dump));
        for side in kit_opened {
            if !self.layout.is_open(side) {
                self.layout.toggle_dock(side);
            }
        }
        for side in Side::ALL {
            let size = area
                .read(cx)
                .dock_size(crate::desktop::view::dock::placement(side));
            if let Some(size) = size {
                self.layout.set_size(side, size.as_f32());
            }
        }
        self.dock.reconcile(&mut self.layout, window, cx);
        // After reconcile, so the events it caused stop at step 1.
        let settled = area.read(cx).dump(cx);
        self.dock.accept(settled);
        self.check_focus(window, cx);
        self.dock.request_save();
        cx.notify();
    }
}
