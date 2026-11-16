//! The Add repository sheet as the workspace keeps it (Wave 04 Cycle 05a
//! design B1 to B3, B9, B10's "Adding"). Every way in (AW-1 to AW-5) calls
//! `show_add_repository`; the sheet goes on Root's one sheet layer, in place
//! of Settings or the details sheet, and the workspace owns it
//! (`SheetOwner::AddRepository`), so `sync_sheet` keeps it whatever item is
//! shown. The Kit closes it on Escape, its close button and the scrim, and
//! its `on_close` runs `add_closed(Kept)` at once; every other close calls
//! `add_closed` itself.
//!
//! Where focus goes is B9's table (G1 to G23), met with Cycle 04's
//! mechanism for Settings:
//! - **The origin** is the first of the way in's own, the one
//!   `choose_activity` gives (`settings_origin`), what an open workspace
//!   layer returns to, and the control focused (C4 A2). It is focused
//!   before `open_sheet`, so the Kit's own record is the origin too, and its
//!   region is recorded for an origin no longer drawn at the close
//!   (`settings_return`, C4 R-45).
//! - **Opened from Settings** (AW-3), the sheet replaces Settings: it takes
//!   Settings' own origin O_s first, then closes Settings quietly, which
//!   empties a typed passphrase. That it came from Settings is kept apart
//!   from O_s, which may be none (Ruling 16). Its closes return to
//!   Settings: on its Add or create a repository… button, or, after a
//!   success, on the new repository (G4, G5).
//! - **A close waits for dialogs.** A close that is not quiet waits, whole,
//!   while a dialog is open over the sheet: the owner and the open flag
//!   stay until it runs (Ruling 17; G23), since the Kit's `close_sheet`
//!   moves focus without looking (C4 A7 item 5).
//! - **Adding.** The sheet says which request and attempt its confirmation
//!   queued; the workspace keeps the add, with the open it came from
//!   (`PendingAdd`; R-26, R-28), until its request ends, and looks at it at
//!   each change of the request model, after the confirmation's own
//!   observer has closed it (`defer_in`; R-25). A success closes the sheet
//!   of the same open; one landing after that open closed clears its text
//!   and check (decision 16); a later open keeps its text and focus, and
//!   forgets only a stale check of the folder added (Ruling 19; B10).

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use gpui_kit::{
    AppContext, Context, Entity, FocusHandle, Subscription, WeakFocusHandle, Window,
    component::WindowExt,
};
use manyhands::results::{Outcome, RecoveryActionKind, ResultCode};

use super::{
    QuitPhase, Region, Workspace, content::SheetOwner, render::modal_open,
    settings_sheet::SettingsReturn,
};
use crate::desktop::{
    services::{
        Services,
        requests::{DesktopRequestId, RequestState},
    },
    view::{
        add_repository::{AddEvent, AddRepositorySheet},
        confirm::offers_prepare_again,
        scroll::ContentScroll,
        settings::{OpenFlag, SettingsRoute},
    },
};

/// Where the Add repository sheet opens.
pub(crate) enum AddRoute {
    /// The existing-folder mode, as last left (decision 16).
    New,
    /// The existing-folder mode, `root` filled and checked (AW-4, AW-5).
    Inspect(PathBuf),
}

/// How the Add repository sheet closed (B3).
#[derive(Clone, PartialEq, Eq)]
pub(in crate::desktop::view) enum AddClose {
    /// Escape, the close button, the scrim, Cancel: typed text is kept.
    Kept,
    /// A successful add of `root`: typed text is cleared.
    Added(PathBuf),
    /// Open its settings: Settings opens on `root`.
    ToSettings(PathBuf),
    /// Quitting, or Settings opening over it: no focus step.
    Quiet,
}

/// An add confirmed from the sheet, until its request ends (B10).
struct PendingAdd {
    /// The attempt the confirmation queued; a newer one replaces it.
    attempt: u32,
    /// The sheet's open number when it was confirmed (R-26).
    session: u64,
    /// The share choice to send on success (Part C), if any.
    share: Option<Option<String>>,
}

/// What the workspace keeps of the Add repository sheet (B4, B10).
pub(super) struct AddState {
    sheet: Entity<AddRepositorySheet>,
    /// The sheet's own open flag, which it is given (B4): set at each open,
    /// cleared at once by `add_closed`.
    open: OpenFlag,
    /// The number of the open now, or last (R-26).
    session: u64,
    /// Where focus returns at the close, held weakly.
    origin: Option<WeakFocusHandle>,
    /// Opened from Settings: Settings' own origin O_s, or `None` when it
    /// recorded none (B2 step 2; Ruling 16).
    from_settings: Option<Option<WeakFocusHandle>>,
    /// Each add confirmed from the sheet, by request (B10).
    pending: HashMap<DesktopRequestId, PendingAdd>,
    /// A close waiting for the dialogs over the sheet (Ruling 17).
    waiting: Option<AddClose>,
    _events: Subscription,
}

impl AddState {
    /// The sheet, made once, drawn in a region of the content column's
    /// scroll that keeps its focused control in view (R-8; Ruling 14).
    pub(super) fn new(
        services: Services,
        column: &ContentScroll,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) -> Self {
        let open = OpenFlag::default();
        let sheet = {
            let (scroll, open) = (column.region_keeping(), open.clone());
            cx.new(|cx| AddRepositorySheet::new(services, scroll, open, window, cx))
        };
        let events = cx.subscribe_in(&sheet, window, |this, _, event: &AddEvent, window, cx| {
            this.add_event(event, window, cx)
        });
        Self {
            sheet,
            open,
            session: 0,
            origin: None,
            from_settings: None,
            pending: HashMap::new(),
            waiting: None,
            _events: events,
        }
    }
}

impl Workspace {
    /// The Add sheet is open: it owns the sheet, and the sheet is active.
    pub(super) fn add_open(&self, window: &mut Window, cx: &mut gpui_kit::App) -> bool {
        matches!(self.content.sheet, Some(SheetOwner::AddRepository)) && window.has_active_sheet(cx)
    }

    /// The sheet's own open flag is set (B1 case 3).
    pub(super) fn add_flagged(&self) -> bool {
        self.add.open.get()
    }

    /// Settings' origin O_s, when the sheet came from Settings and Settings
    /// recorded one: where Settings opened over the sheet returns (R-22).
    pub(super) fn add_return(&self) -> Option<FocusHandle> {
        self.add
            .from_settings
            .clone()
            .flatten()
            .and_then(|origin| origin.upgrade())
    }

    /// The Add folder field, focused: the sheet's first control (G1, G18).
    pub(super) fn focus_add_path(&self, window: &mut Window, cx: &mut Context<Self>) {
        let path = self.add.sheet.read(cx).add_path_focus(cx);
        window.focus(&path, cx);
    }

    /// Opens the Add repository sheet at `route` (B2). `origin` is where
    /// focus returns on close; with none, C4 A2's next ways to it. Nothing
    /// opens under a dialog, over Resolve conflict (Wave 04 Cycle 05b
    /// decision 9) or while quitting. Open already, the route
    /// applies in place and the origin, and any return to Settings, are
    /// kept (step 5). From Settings, Settings' origin is taken and Settings
    /// closes quietly (step 2); a details sheet gives way (step 3, R-21).
    pub(in crate::desktop::view) fn show_add_repository(
        &mut self,
        route: AddRoute,
        origin: Option<FocusHandle>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if window.has_active_dialog(cx)
            || self.quit != QuitPhase::Running
            || self.resolve_open(window, cx)
        {
            return;
        }
        // Closed by the Kit unseen: quietly, so no return to Settings follows.
        if matches!(self.content.sheet, Some(SheetOwner::AddRepository))
            && !window.has_active_sheet(cx)
        {
            self.add_closed(AddClose::Quiet, window, cx);
        }
        // Every open records the panel's selection Settings follows, as an
        // open of Settings does (R-55; Ruling 15).
        let panel = self.navigation.selected().map(PathBuf::from);
        self.content
            .settings
            .update(cx, |settings, _| settings.saw_panel(panel));
        let sheet = self.add.sheet.clone();
        // The existing-folder mode, and for an `Inspect` its folder filled
        // and checked (B2 steps 5 and 6).
        if self.add_open(window, cx) {
            sheet.update(cx, |sheet, cx| sheet.route(route, window, cx));
            self.focus_add_path(window, cx);
            return;
        }
        self.add.from_settings = None;
        if matches!(self.content.sheet, Some(SheetOwner::Settings)) {
            let o_s = self
                .content
                .settings
                .update(cx, |settings, _| settings.take_origin());
            self.add.from_settings = Some(o_s.as_ref().map(FocusHandle::downgrade));
            self.settings_closed(true, window, cx);
        }
        self.content.details = None;
        if window.has_active_sheet(cx) {
            self.content.sheet = None;
            window.close_sheet(cx);
        }
        let origin = origin
            .or_else(|| self.settings_origin.clone())
            .or_else(|| self.take_layer_origin(cx))
            .or_else(|| window.focused(cx));
        let region = origin
            .as_ref()
            .and_then(|origin| self.origin_region(origin, window));
        self.settings_return = SettingsReturn::recorded(region);
        if let Some(origin) = origin.as_ref().filter(|origin| !origin.is_focused(window)) {
            window.focus(origin, cx);
        }
        self.add.origin = origin.as_ref().map(FocusHandle::downgrade);
        self.add.session += 1;
        self.add.open.set(true);
        self.content.sheet = Some(SheetOwner::AddRepository);
        self.adding_changed(cx);
        let workspace = cx.entity().downgrade();
        let drawn = sheet.clone();
        window.open_sheet(cx, move |kit, window, cx| {
            // Escape, the close button and the scrim: the Kit has closed
            // the sheet, and it closes now (C4 A7 item 10).
            let workspace = workspace.clone();
            let kit = kit.on_close(move |_, window, cx| {
                let _ = workspace.update(cx, |workspace, cx| {
                    if matches!(workspace.content.sheet, Some(SheetOwner::AddRepository)) {
                        workspace.add_closed(AddClose::Kept, window, cx);
                    }
                });
            });
            drawn.update(cx, |sheet, cx| sheet.sheet(kit, window, cx))
        });
        sheet.update(cx, |sheet, cx| sheet.route(route, window, cx));
        // After `open_sheet` has focused Root's own handle (fact 34).
        self.focus_add_path(window, cx);
        cx.notify();
    }

    /// The Add sheet closed `how` (B3). Not quiet, it waits, whole, while a
    /// dialog is open (Ruling 17), and moves nothing once closed already. One
    /// the Kit did not make closes the Kit's sheet first, never another
    /// owner's. An Add waiting for its check goes no further; a success
    /// clears the typed text and the check (decision 16). Then, unless quiet:
    /// Open its settings opens Settings on the repository; from Settings,
    /// Settings opens again, on the new repository after a success, else on
    /// its Add or create a repository…; otherwise focus goes back to the
    /// origin, else its region, else the content region.
    pub(in crate::desktop::view) fn add_closed(
        &mut self,
        how: AddClose,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let owned = matches!(self.content.sheet, Some(SheetOwner::AddRepository));
        let (quiet, added) = (how == AddClose::Quiet, matches!(how, AddClose::Added(_)));
        // Closed already (two closes in one frame, or one deferred past a
        // quiet close): nothing moves; a success still clears (B10).
        let closed = !quiet && !owned && !self.add.open.get() && self.add.waiting.is_none();
        if !quiet && !closed && window.has_active_dialog(cx) {
            // A frame that finds it waiting already asks for no other.
            if self.add.waiting.as_ref() != Some(&how) {
                self.add.waiting = Some(how);
                cx.notify();
            }
            return;
        }
        self.add.waiting = None;
        if owned && window.has_active_sheet(cx) {
            window.close_sheet(cx);
        }
        if owned {
            self.content.sheet = None;
        }
        self.add.open.set(false);
        self.add.sheet.update(cx, |sheet, cx| {
            sheet.forget_pending_submit();
            if added {
                sheet.clear(window, cx);
            }
        });
        if closed {
            return;
        }
        let origin = self.add.origin.take().and_then(|origin| origin.upgrade());
        let from_settings = self
            .add
            .from_settings
            .take()
            .map(|o_s| o_s.and_then(|o_s| o_s.upgrade()));
        let region = std::mem::take(&mut self.settings_return).region();
        cx.notify();
        if quiet || modal_open(window, cx) {
            return;
        }
        let settings = match (how, from_settings) {
            (AddClose::ToSettings(root), Some(o_s)) => Some((SettingsRoute::Manage(root), o_s)),
            (AddClose::ToSettings(root), None) => {
                Some((SettingsRoute::Manage(root), origin.clone()))
            }
            (AddClose::Added(root), Some(o_s)) => Some((SettingsRoute::Manage(root), o_s)),
            (_, Some(o_s)) => Some((SettingsRoute::AddButton, o_s)),
            (_, None) => None,
        };
        match (settings, origin) {
            // After the event that closed the sheet: the Kit's Escape can
            // reach the closed sheet's close again in the same keystroke,
            // and each close the Kit makes closes the sheet open then, so
            // Settings opened at once would close with it. Deferred, a
            // repeated close finds no sheet and no owner, and does nothing.
            (Some((route, settings_origin)), _) => cx.defer_in(window, move |this, window, cx| {
                // Settings recorded no origin: it opens with none again, and
                // its own close goes to the content region (C4 F10).
                if settings_origin.is_none() {
                    window.blur(cx);
                }
                this.show_settings(route, settings_origin, window, cx);
            }),
            (None, Some(origin)) => {
                window.focus(&origin, cx);
                self.settings_return = SettingsReturn::armed(region);
            }
            (None, None) => self.focus_region(region.unwrap_or(Region::Content), window, cx),
        }
    }

    /// A close that waited for the dialogs over the sheet runs once none is
    /// open, after the frame (Ruling 17).
    pub(super) fn run_waiting_add_close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.add.waiting.is_none() || window.has_active_dialog(cx) {
            return;
        }
        if let Some(how) = self.add.waiting.take() {
            cx.defer_in(window, move |this, window, cx| {
                this.add_closed(how, window, cx)
            });
        }
    }

    /// What the sheet asks (B10).
    fn add_event(&mut self, event: &AddEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event {
            AddEvent::Confirmed {
                request,
                attempt,
                share,
            } => {
                let session = self.add.session;
                self.add.pending.insert(
                    *request,
                    PendingAdd {
                        attempt: *attempt,
                        session,
                        share: share.clone(),
                    },
                );
                self.adding_changed(cx);
            }
            AddEvent::Cancel => self.add_closed(AddClose::Kept, window, cx),
            AddEvent::OpenSettings(root) => {
                self.add_closed(AddClose::ToSettings(root.clone()), window, cx)
            }
        }
    }

    /// Tells the sheet whether an add of its open is pending (R-27).
    fn adding_changed(&mut self, cx: &mut Context<Self>) {
        let session = self.add.session;
        let adding = self
            .add
            .pending
            .values()
            .any(|entry| entry.session == session);
        self.add
            .sheet
            .update(cx, |sheet, cx| sheet.set_adding(adding, cx));
    }

    /// Each pending add, at a change of the request model (B10): absent, an
    /// older attempt, or a stale answer that offers Prepare again, still
    /// waits; a newer attempt replaces the entry's; a success, or a
    /// discovery still pending, is added; an answer with a Retry or Prepare
    /// again still waits for that attempt; anything else ends it. An added
    /// or ended entry is taken out now and handled once the confirmation's
    /// own observer has closed it (`defer_in`; R-25).
    pub(super) fn adds_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.add.pending.is_empty() {
            return;
        }
        let mut done: Vec<(DesktopRequestId, Option<PathBuf>)> = Vec::new();
        {
            let operations = self.services.operations().read(cx);
            for (id, entry) in self.add.pending.iter_mut() {
                let Some(request) = operations.request(*id) else {
                    continue;
                };
                if request.attempt < entry.attempt {
                    continue;
                }
                entry.attempt = request.attempt;
                match &request.state {
                    RequestState::Queued | RequestState::Running { .. } => {}
                    RequestState::Finished(outcome, code, _, recovery, _) => {
                        if offers_prepare_again(*code, recovery) {
                            continue;
                        }
                        let registered = matches!(
                            (outcome, code),
                            (Outcome::Success, ResultCode::Ok)
                                | (Outcome::Partial, ResultCode::DiscoveryPending)
                        );
                        let again = recovery.iter().any(|action| {
                            matches!(
                                action.action,
                                RecoveryActionKind::RequestRetry
                                    | RecoveryActionKind::RequestPrepare
                            )
                        });
                        if registered {
                            let root = request
                                .registered_root
                                .clone()
                                .or_else(|| request.intent.root().map(Path::to_owned));
                            done.push((*id, root));
                        } else if !again {
                            done.push((*id, None));
                        }
                    }
                    RequestState::Removed | RequestState::NotSent(_) | RequestState::Dismissed => {
                        done.push((*id, None))
                    }
                }
            }
        }
        if done.is_empty() {
            return;
        }
        for (id, root) in done {
            if let Some(entry) = self.add.pending.remove(&id) {
                cx.defer_in(window, move |this, window, cx| {
                    this.add_ended(entry, root, window, cx)
                });
            }
        }
        self.adding_changed(cx);
    }

    /// A pending add ended: added at `root`, or not (`None`). Added: its
    /// share is sent (C2); the sheet of the same open closes, or, that open
    /// closed since, forgets its check and text; a later open forgets only a
    /// stale check of `root`, its text and focus kept (R-26; Ruling 19 C-5; B10).
    fn add_ended(
        &mut self,
        entry: PendingAdd,
        root: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(root) = root else {
            return;
        };
        self.send_share(entry.share, &root);
        if entry.session != self.add.session {
            self.add
                .sheet
                .update(cx, |sheet, cx| sheet.forget_check_of(&root, cx));
            return;
        }
        if self.add.open.get() && matches!(self.content.sheet, Some(SheetOwner::AddRepository)) {
            self.add_closed(AddClose::Added(root), window, cx);
        } else {
            self.add
                .sheet
                .update(cx, |sheet, cx| sheet.clear(window, cx));
        }
    }

    /// The Add sheet the workspace keeps.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn add_sheet(&self) -> Option<Entity<AddRepositorySheet>> {
        Some(self.add.sheet.clone())
    }
}

mod share;
#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "add_sheet_tests.rs"]
mod tests;
