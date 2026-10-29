//! The stale-edit review (design, Stale-edit review; decision 4; R-36,
//! R-44, R-68, R-79): how it opens, what it holds while open, and its two
//! ways out. What it draws is `review_view`; its own model, the three
//! values per field, the choices and the body's result, is
//! `model::review`.
//!
//! - **It opens** on an `external_change` answer to Save, on Save over an
//!   unverified base (R-36), and on Compare… of the change banner (R-68),
//!   a kept session's included; a restored draft whose base the branch
//!   alone did not move opens it too (R-42). It never opens by itself.
//! - **On opening** the item is read again, under a new stamp. Once a read
//!   of the tab lands, that one or a newer one that superseded it (the
//!   workspace's reload after a request, a list's Retry), as the next frame
//!   is drawn, the review takes mine (the session's current), theirs (the
//!   read, normalized, with its observation) and the base (the session's
//!   own, each field the read gives as the file has it taken as theirs,
//!   normalized: Part C fix 1, M1), and the body's result goes into an
//!   editor of its own, exactly
//!   (R-79), in an edit host context, so Escape there focuses Done (R-59).
//!   While it reads, Cancel is offered; a read that fails says so, with
//!   Retry and Cancel.
//! - **It never opens** while a save is in flight or kept for its Retry:
//!   the save's answer, or its Retry, comes first.
//! - **While it is open** the draft feed is suspended, here and in D2's
//!   schedule: nothing is written to the draft or the store. The page's
//!   place is the review's, the toolbar is hidden, and Done, Save and the
//!   save banners' Retry and Resume are disabled.
//! - **Use these choices**, once every both-changed field has a choice:
//!   the base becomes theirs; the session's fields become the chosen ones
//!   and the body's result; the edit editor takes the result exactly,
//!   which restarts undo (R-18, R-79), its caret kept, clamped, and its
//!   baseline theirs; D2's draft moves onto the new base and takes the
//!   fields, written at once, or begins on it; and the feed resumes. When
//!   the choices leave nothing changed, the session is clean and its draft
//!   is discarded instead, so a clean tab never asks on close. A change
//!   banner from an `external_change` answer is resolved; any other
//!   answer's banner stays.
//! - **Cancel** shows mine again in the edit editor, which the review never
//!   touched, and resumes the feed. The draft and its file are unchanged.
//!
//! Switching the body's choice once its result holds edits asks first:
//! "Replace your edits?". Nothing here derives `Debug`: the review holds
//! draft text.

use std::rc::Rc;

use gpui_kit::{
    App, AppContext, ClickEvent, Context, Entity, FocusHandle, FontWeight, InteractiveElement,
    IntoElement, ParentElement, StatefulInteractiveElement, Styled, Subscription, TestSupportExt,
    Window,
    component::{
        WindowExt,
        button::{Button, ButtonVariants},
        dialog::Confirm,
        h_flex, v_flex,
    },
    div,
};
use manyhands::{drafts::DraftFields, results::ResultCode};

use super::{DraftLink, Phase, saved::body_of};
use crate::desktop::{
    model::{
        edit::base_for_edit,
        links::LinkTarget,
        review::{
            BodyChoice, Choice, KEEP_MY_EDITS, REPLACE, REPLACE_DETAIL, REPLACE_YOUR_EDITS, Review,
            base_as_read,
        },
    },
    view::{
        editor::host::{EditorHost, EditorHostEvent},
        item::ItemPane,
    },
};

/// What Replace does once "Replace your edits?" is answered.
type Replace = Rc<dyn Fn(&mut Window, &mut App)>;

/// The review while it is open: the read it waits for, then its model and
/// the body's result editor. It holds draft text, so it derives nothing.
pub(super) struct ReviewState {
    /// A read of the tab landed while the review had no model: the model is
    /// built from it as the next frame is drawn. Every read that lands is at
    /// least as new as the one the review opened on, which a later reload
    /// of the tab supersedes (I1).
    pub(super) landed: bool,
    /// The read failed, or gave no item to review: Retry reads again.
    pub(super) failed: bool,
    pub(super) model: Option<Review>,
    /// The body's result, once the body has a side.
    pub(super) host: Option<Entity<EditorHost>>,
    /// The review itself, which takes the focus as it opens.
    pub(super) focus: FocusHandle,
    /// The wrappers around its actions, then each field's choices, by the
    /// model's row: a Kit button's own focus is private, so focus inside
    /// one of these scrolls it into view.
    pub(super) lines: Vec<FocusHandle>,
    _host: Vec<Subscription>,
}

impl ItemPane {
    /// The review opens (R-36, R-68): from Save, its answer, a banner's
    /// Compare… or Review…, or a restored draft. The item is read again;
    /// the draft feed is suspended until Use these choices or Cancel.
    pub(in crate::desktop::view::item) fn open_review(&mut self, cx: &mut Context<Self>) {
        // Not while a save is in flight, or kept for its Retry: its answer,
        // or its Retry, comes first (I2).
        if self.shown_session().is_none()
            || self.edit.phase == Phase::Saving
            || self.edit.save.run.is_some()
        {
            return;
        }
        self.edit.phase = Phase::Review;
        if let Some(id) = self.live_draft(cx) {
            self.services
                .drafts()
                .update(cx, |registry, _| registry.suspend(id));
        }
        self.reload(cx);
        self.edit.review = Some(ReviewState {
            landed: false,
            failed: false,
            model: None,
            host: None,
            focus: cx.focus_handle(),
            lines: Vec::new(),
            _host: Vec::new(),
        });
        cx.notify();
    }

    /// Retry of a read that failed: the item is read again.
    pub(super) fn review_retry(&mut self, cx: &mut Context<Self>) {
        self.reload(cx);
        if let Some(review) = &mut self.edit.review {
            review.failed = false;
        }
        cx.notify();
    }

    /// The review closes: its state goes, and D2's schedule of the
    /// session's draft resumes. Whatever ends it, Use, Cancel or the
    /// session ending, comes here.
    pub(super) fn close_review(&mut self, cx: &mut Context<Self>) {
        if self.edit.review.take().is_none() {
            return;
        }
        if let Some(id) = self.live_draft(cx) {
            self.services
                .drafts()
                .update(cx, |registry, cx| registry.resume(id, cx));
        }
    }

    /// As a frame is drawn: the read the review opened on has landed, and
    /// the review takes its three values. A read that kept older rows, or
    /// found no conforming item, fails, for Retry.
    pub(in crate::desktop::view::item) fn sync_review(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self
            .edit
            .review
            .as_ref()
            .is_some_and(|review| review.landed)
        {
            return;
        }
        let fresh = self.model.not_refreshed().is_none();
        let read = self.model.item().rows().filter(|_| fresh);
        let theirs = read.and_then(base_for_edit);
        let raw = read.and_then(DraftFields::of);
        let Some(session) = &self.edit.session else {
            return;
        };
        let (base, mine) = (session.base.fields.clone(), session.current(cx));
        let Some(review) = &mut self.edit.review else {
            return;
        };
        review.landed = false;
        let (Some(theirs), Some(raw)) = (theirs, raw) else {
            review.failed = true;
            cx.notify();
            return;
        };
        review.failed = false;
        // M1: the session's own base, a restored draft's kept one included,
        // holds no normalization-only difference from theirs.
        let base = base_as_read(&base, &raw, &theirs.fields);
        let model = Review::new(base, theirs, mine);
        review.lines = (0..=model.rows().len())
            .map(|_| cx.focus_handle())
            .collect();
        review.model = Some(model);
        let focus = review.focus.clone();
        self.show_result(window, cx);
        // Once the frame that draws it is drawn.
        cx.defer_in(window, move |_, window, cx| window.focus(&focus, cx));
        cx.notify();
    }

    /// A field's choice. The body's, once its result holds edits, asks
    /// first; any other choice takes at once. Nothing is written.
    pub(super) fn review_choose(
        &mut self,
        ix: usize,
        choice: Choice,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(model) = self
            .edit
            .review
            .as_mut()
            .and_then(|review| review.model.as_mut())
        else {
            return;
        };
        if Some(ix) != model.body_row() {
            model.choose(ix, choice);
            cx.notify();
            return;
        }
        match model.choose_body(choice) {
            BodyChoice::Chosen => self.show_result(window, cx),
            BodyChoice::AskFirst => self.ask_replace(choice, window, cx),
        }
        cx.notify();
    }

    /// "Replace your edits?": Replace takes `choice` for the body, its
    /// edits replaced; Keep my edits, the default, changes nothing.
    fn ask_replace(&mut self, choice: Choice, window: &mut Window, cx: &mut Context<Self>) {
        let pane = cx.entity().downgrade();
        let replace: Replace = Rc::new(move |window, cx| {
            let _ = pane.update(cx, |pane, cx| {
                if let Some(model) = pane
                    .edit
                    .review
                    .as_mut()
                    .and_then(|review| review.model.as_mut())
                {
                    model.replace_body(choice);
                }
                pane.show_result(window, cx);
                cx.notify();
            });
        });
        let origin = window.focused(cx);
        window.open_dialog(cx, move |base, _, _| {
            let close = origin.clone();
            base.close_button(false)
                .overlay_closable(false)
                .child(replace_body(&replace))
                // Enter: Keep my edits, the default; the Kit closes it.
                .on_ok(|_, _, _| true)
                .on_cancel(|_, _, _| true)
                .on_close(move |_, window, cx| {
                    if let Some(origin) = &close {
                        window.focus(origin, cx);
                    }
                })
        });
    }

    /// The body's result in the review's own editor: made once the body
    /// has a side, its text the result exactly (R-79), and put in again
    /// exactly when a choice replaces it.
    fn show_result(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (done, scroll) = (self.edit.done.clone(), self.scroll.clone());
        let Some(review) = &mut self.edit.review else {
            return;
        };
        let Some(result) = review
            .model
            .as_ref()
            .and_then(|model| model.result())
            .map(str::to_owned)
        else {
            return;
        };
        if let Some(host) = &review.host {
            if host.read(cx).text(cx) != result {
                host.update(cx, |host, cx| host.put_exact(&result, 0, cx));
            }
            return;
        }
        let images = self.images.clone();
        let host = cx.new(|cx| EditorHost::new("", None, window, cx));
        host.update(cx, |host, cx| {
            host.place(done, &scroll, window, cx);
            host.show_images(&images, cx);
            host.put_exact(&result, 0, cx);
        });
        review._host = vec![
            cx.subscribe(&host, |pane, host, event: &EditorHostEvent, cx| {
                if matches!(event, EditorHostEvent::Edited) {
                    pane.review_edited(&host, cx);
                }
            }),
            cx.subscribe(&host, |pane, _, target: &LinkTarget, cx| {
                pane.follow_link(target, cx)
            }),
        ];
        review.host = Some(host);
    }

    /// The body's result was edited in the review's editor: the model keeps
    /// it. Nothing is fed to the draft.
    fn review_edited(&mut self, host: &Entity<EditorHost>, cx: &mut Context<Self>) {
        let text = host.read(cx).text(cx);
        if let Some(model) = self
            .edit
            .review
            .as_mut()
            .and_then(|review| review.model.as_mut())
        {
            model.edit_result(text);
        }
        cx.notify();
    }

    /// Use these choices (R-44, R-18, R-79): the session goes on over
    /// theirs, with the chosen fields and the body's result, and the feed
    /// resumes. Not before every both-changed field has a choice.
    pub(super) fn review_use(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let chosen = self
            .edit
            .review
            .as_ref()
            .and_then(|review| review.model.as_ref())
            .and_then(|model| Some((model.chosen()?, model.theirs().clone())));
        let Some((chosen, theirs)) = chosen else {
            return;
        };
        self.close_review(cx);
        self.edit.phase = Phase::Editing;
        // The change it reviewed is resolved: the base is the current
        // version. Another answer's banner, a hand-off owed, stays.
        if self
            .edit
            .save
            .answer
            .as_ref()
            .is_some_and(|answer| answer.code == ResultCode::ExternalChange)
        {
            self.edit.save.answer = None;
        }
        let draft = self.live_draft(cx);
        let tab = self.tab_key();
        let Some(session) = &mut self.edit.session else {
            return;
        };
        let result = body_of(&chosen).to_owned();
        let baseline = body_of(&theirs.fields).to_owned();
        session.base = theirs.clone();
        session.meta = chosen;
        let editor = session.host.read(cx).editor().clone();
        session
            .host
            .update(cx, |host, cx| host.rebase(baseline, &result, cx));
        // M3: choices that change nothing leave a clean session, and its
        // draft would hold nothing the item does not: it is discarded, as
        // the tab's, so a clean tab never asks on close.
        let clean = !session.dirty(cx);
        if let Some(id) = draft {
            if clean {
                session.draft = DraftLink::None;
                self.services.drafts().update(cx, |registry, cx| {
                    registry.detach(&tab);
                    registry.discard(id, cx).detach();
                });
            } else {
                self.services
                    .drafts()
                    .update(cx, |registry, cx| registry.adopt_base(id, theirs, cx));
            }
        }
        // D2's draft takes the chosen fields, or one begins on the new base.
        self.feed(cx);
        self.sync_status(cx);
        // The base moved: what the session proposes is checked against it
        // again, changed or not (R-26; Task 21 fix 1).
        self.relations_rebased(cx);
        cx.defer_in(window, move |_, window, cx| {
            editor.update(cx, |editor, cx| editor.focus(window, cx));
        });
        cx.notify();
    }

    /// Cancel: mine is in the edit editor again, as it was, and the feed
    /// resumes. The draft and its file are unchanged.
    pub(super) fn review_cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.edit.review.is_none() {
            return;
        }
        self.close_review(cx);
        self.edit.phase = Phase::Editing;
        let Some(session) = &self.edit.session else {
            return;
        };
        let editor = session.host.read(cx).editor().clone();
        // Anything the edit editor took meanwhile is fed now; mine, as D2
        // holds it, is no edit.
        self.feed(cx);
        cx.defer_in(window, move |_, window, cx| {
            editor.update(cx, |editor, cx| editor.focus(window, cx));
        });
        cx.notify();
    }
}

/// The dialog's message and its two buttons, each pressed by a click or by
/// Enter on it.
fn replace_body(replace: &Replace) -> impl IntoElement + use<> {
    let button = |id: &'static str, label: &'static str, proceed: bool| {
        let replace = replace.clone();
        let press = Rc::new(move |window: &mut Window, cx: &mut App| {
            window.close_dialog(cx);
            if proceed {
                replace(window, cx);
            }
        });
        let mut button = Button::new(format!("{id}-button")).label(label);
        if proceed {
            button = button.danger();
        }
        let click = press.clone();
        div()
            .id(id)
            .test_support()
            .aria_label(label)
            .on_action(move |_: &Confirm, window, cx| press(window, cx))
            .child(button.on_click(move |_: &ClickEvent, window, cx| click(window, cx)))
    };
    v_flex()
        .id("review-replace-dialog")
        .test_support()
        .gap_2()
        .child(
            div()
                .id("review-replace-message")
                .test_support()
                .aria_label(REPLACE_YOUR_EDITS)
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .child(REPLACE_YOUR_EDITS),
        )
        .child(
            div()
                .id("review-replace-detail")
                .test_support()
                .aria_label(REPLACE_DETAIL)
                .text_sm()
                .child(REPLACE_DETAIL),
        )
        .child(
            h_flex()
                .gap_2()
                .child(button("review-replace-no", KEEP_MY_EDITS, false))
                .child(button("review-replace-yes", REPLACE, true)),
        )
}
