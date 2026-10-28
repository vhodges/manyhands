//! The restored-draft route (design, Restored drafts and the close dialog;
//! R-35, R-42, R-79): Continue editing of a draft stored for the tab's
//! item or restored in it, and Save from a restored draft.
//!
//! - **Continue editing** (the draft banner, D2's restored section, Edit,
//!   `e`). A stored draft is claimed into the tab first, as D2's Restore
//!   claims it; a refused claim says why, in D2's words, and prepares
//!   nothing. Then `context prepare` runs with the Edit intent, as Edit's
//!   does (`prepare`): a refusal leaves the draft restored in the tab. Once
//!   the item is read again in that context, the edit editor opens on the
//!   draft: its text goes in exactly (R-79), the caret at the start, and
//!   the tab is dirty, with Save enabled. The base is the draft's own. It
//!   takes the read's token and context only when the branch alone changed
//!   the token (`model::edit::adopted_base`), and D2's draft moves onto it;
//!   otherwise the review opens (`review`).
//! - **Save** from a restored draft (D2's section, or the close dialog)
//!   runs those steps, then saves.
//! - **A new item's draft** (`NotCreated`, R-20) is continued through
//!   `context prepare` with the Create intent, which is Task 22's
//!   (`continue_new_item`). Until then its Continue editing and Save are
//!   unavailable, with "Available in a later release".
//!
//! A context is so always provisioned through `context prepare`. Nothing
//! here holds draft text but the session it opens.

use gpui_kit::{
    AnyElement, App, ClickEvent, Context, InteractiveElement, IntoElement, ParentElement, Role,
    StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{Disableable, button::Button},
    div,
};
use manyhands::{
    drafts::{DraftBase, DraftId, TargetState},
    repository::ItemDto,
};

use super::{
    DraftLink, Phase,
    open::{Opening, Text},
    saved::body_of,
};
use crate::desktop::{
    model::{
        edit::save_state::SAVE,
        edit::{CONTINUE_EDITING, adopted_base, base_for_edit, edit_offered, new_item_base},
        item::LATER_NOTE,
        tabs::TabKey,
    },
    view::item::ItemPane,
};

/// What Continue editing goes on to once the editor opens.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Then {
    Edit,
    Save,
}

/// The draft Continue editing continues, from its prepare until the
/// editor opens on it.
pub(super) struct Continuing {
    draft: DraftId,
    then: Then,
}

/// The draft Continue editing would continue.
#[derive(Clone, Copy)]
enum Continued {
    /// Restored in this tab, with no edit session.
    Restored(DraftId),
    /// Stored for this tab's item, not claimed by this process.
    Stored(DraftId),
}

impl ItemPane {
    /// The draft of this tab's item that Continue editing continues: one
    /// restored here with no edit session, else one stored for the item.
    fn continued(&self, cx: &App) -> Option<Continued> {
        if self.has_session() {
            return None;
        }
        let tab = self.tab_key();
        let TabKey::Item { repo, kind, id } = &tab else {
            return None;
        };
        let registry = self.services.drafts().read(cx);
        if let Some(id) = registry.attached(&tab) {
            return Some(Continued::Restored(id));
        }
        registry.stored_for(repo, *kind, id).map(Continued::Stored)
    }

    /// Whether the draft Continue editing would continue is a new item's
    /// (R-20): its base has neither observation nor source. Task 22
    /// continues it through Create (`continue_new_item`).
    pub(in crate::desktop::view::item) fn continues_new_item(&self, cx: &App) -> bool {
        let registry = self.services.drafts().read(cx);
        let base = match self.continued(cx) {
            Some(Continued::Restored(id)) => {
                registry.session(id).and_then(|session| session.base())
            }
            Some(Continued::Stored(id)) => registry
                .listed(id)
                .and_then(|(draft, _)| draft.base.as_ref()),
            None => None,
        };
        base.is_some_and(new_item_base)
    }

    /// Continue editing the draft stored or restored here, going on to
    /// `then` once the editor opens. False when there is none, so Edit
    /// prepares as it does for an item with no draft.
    pub(super) fn continue_requested(&mut self, then: Then, cx: &mut Context<Self>) -> bool {
        let Some(continued) = self.continued(cx) else {
            return false;
        };
        if self.continues_new_item(cx) {
            self.continue_new_item(cx);
            return true;
        }
        match continued {
            Continued::Restored(id) => self.continue_restored(id, then, cx),
            Continued::Stored(id) => self.claim(
                id,
                move |pane, id, cx| pane.continue_restored(id, then, cx),
                cx,
            ),
        }
        true
    }

    /// Task 22's hook: a new item's draft is continued through `context
    /// prepare` with the Create intent, as in Part D (R-42). Until then it
    /// starts nothing; its Continue editing and Save are unavailable.
    fn continue_new_item(&mut self, cx: &mut Context<Self>) {
        cx.notify();
    }

    /// The restored draft `draft` is continued: the context is prepared,
    /// and the editor opens on the draft once the item is read again.
    fn continue_restored(&mut self, draft: DraftId, then: Then, cx: &mut Context<Self>) {
        if self.edit.phase != Phase::Viewing {
            return;
        }
        self.edit.continuing = Some(Continuing { draft, then });
        self.prepare(cx);
        if !matches!(self.edit.phase, Phase::Preparing(_)) {
            self.edit.continuing = None;
        }
    }

    /// A close dialog's Save still in Continue editing's steps was
    /// cancelled, or the dialog closed otherwise: the editor still opens on
    /// the draft, but nothing is saved (Task 18 fix 1, M1).
    pub(in crate::desktop::view) fn continue_without_saving(&mut self) {
        if let Some(continuing) = &mut self.edit.continuing {
            continuing.then = Then::Edit;
        }
    }

    /// The registry changed: a continued draft that has ended (discarded
    /// elsewhere, or let go of) is no longer continued, so a later Retry
    /// of its prepare opens the item as Edit does (M1).
    pub(in crate::desktop::view::item) fn follow_continuing(&mut self, cx: &App) {
        let ended = self.edit.continuing.as_ref().is_some_and(|continuing| {
            self.services
                .drafts()
                .read(cx)
                .current_id(continuing.draft)
                .is_none()
        });
        if ended {
            self.edit.continuing = None;
        }
    }

    /// Save from D2's restored section: Continue editing's steps, then the
    /// save.
    fn save_restored(&mut self, cx: &mut Context<Self>) {
        if self.edit_available(cx) {
            self.continue_requested(Then::Save, cx);
        }
    }

    /// Loading's read, `item`, landed for a continued draft: the edit
    /// editor opens on it (R-42, R-79). A draft gone meanwhile opens
    /// nothing.
    pub(super) fn open_restored(
        &mut self,
        continuing: Continuing,
        item: &ItemDto,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let drafts = self.services.drafts().clone();
        let registry = drafts.read(cx);
        let Some(id) = registry.current_id(continuing.draft) else {
            return;
        };
        let Some(session) = registry.session(id) else {
            return;
        };
        let (Some(read), Some(body)) = (base_for_edit(item), item.body.as_deref()) else {
            return;
        };
        let meta = session.current().clone();
        let drafted = session.base().cloned();
        let adopted = drafted.as_ref().and_then(|base| adopted_base(base, item));
        // A draft with no base is unverified against the item as read.
        let base = adopted.clone().or(drafted).unwrap_or(DraftBase {
            observation: None,
            ..read
        });
        if let Some(adopted) = &adopted {
            let adopted = adopted.clone();
            drafts.update(cx, |registry, cx| registry.adopt_base(id, adopted, cx));
        }
        let text = body_of(&meta).to_owned();
        let opening = Opening {
            base,
            meta,
            draft: DraftLink::Live(id),
        };
        self.open_session(opening, body, Text::Draft(&text), window, cx);
        if adopted.is_none() {
            self.open_review(cx);
        } else if continuing.then == Then::Save {
            // Once the frame that opened the editor is drawn.
            cx.defer_in(window, |pane, _, cx| {
                pane.start_save(cx);
            });
        }
    }

    /// Whether the draft restored here holds what the item does not: its
    /// Save has something to save.
    pub(in crate::desktop::view::item) fn restored_unsaved(&self, cx: &App) -> bool {
        let registry = self.services.drafts().read(cx);
        let Some(id) = registry.attached(&self.tab_key()) else {
            return false;
        };
        let Some(item) = self.model.item().rows() else {
            return false;
        };
        edit_offered(self.model.target(), item)
            && !matches!(
                registry.classify(id, Ok(item)),
                Some(TargetState::AlreadySaved) | None
            )
    }

    /// Continue editing, as the button `id` of the draft banner or D2's
    /// section: offered while Edit can start, and unavailable, with its
    /// note, for a new item's draft.
    pub(in crate::desktop::view::item) fn continue_button(
        &self,
        id: &'static str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let enabled = self.edit_available(cx);
        action(id, CONTINUE_EDITING, enabled, self.continues_new_item(cx))
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                this.edit_requested(window, cx);
            }))
            .into_any_element()
    }

    /// D2's section's Save: offered while Edit can start and the draft
    /// holds what the item does not; unavailable, with its note, for a new
    /// item's draft.
    pub(in crate::desktop::view::item) fn restored_save_button(
        &self,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let enabled = self.edit_available(cx) && self.restored_unsaved(cx);
        action("draft-save", SAVE, enabled, self.continues_new_item(cx))
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.save_restored(cx)))
            .into_any_element()
    }
}

/// A draft action, the button `id`-button in the wrapper `id`: enabled or
/// not, and, `later`, unavailable with "Available in a later release".
fn action(id: &'static str, label: &'static str, enabled: bool, later: bool) -> Action {
    Action {
        id,
        label,
        enabled: enabled && !later,
        later,
    }
}

/// A draft action before it takes its press.
struct Action {
    id: &'static str,
    label: &'static str,
    enabled: bool,
    later: bool,
}

impl Action {
    fn on_click(
        self,
        press: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> impl IntoElement {
        let button = Button::new(gpui_kit::ElementId::from(format!("{}-button", self.id)))
            .label(self.label)
            .disabled(!self.enabled)
            .on_click(press);
        let wrapper = div()
            .id(self.id)
            .test_support()
            .role(Role::Group)
            .aria_label(self.label)
            .flex_none();
        if !self.later {
            return wrapper.child(button);
        }
        wrapper.aria_description(LATER_NOTE).child(
            div()
                .id(gpui_kit::ElementId::from(format!("{}-note", self.id)))
                .test_support()
                .aria_label(LATER_NOTE)
                .child(button.tooltip(LATER_NOTE)),
        )
    }
}
