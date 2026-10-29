//! A ticket's Waiting on and Parent while editing (design, Relationships
//! (contract 10); R-26, R-38, R-54, R-72, R-74): the decisions are
//! `model::relations_edit`'s, the rows are drawn by `sheet::ticket`, and
//! the `picker` is Add…'s and Change…'s; this holds the state, the check
//! and its answer, the save gate's part, and the names.
//!
//! - **The rows.** While the session is shown, Waiting on is a chip per
//!   dependency, in ID order (R-72): lifecycle icon, short code and title,
//!   its state ("not found, kept" when no ticket the index holds has its
//!   ID), its ID and Remove, then Add…; Parent is its chip, then Change…
//!   and Clear. Remove, Add…, Change… and Clear are offered only while the
//!   tab is Editing or Saving; in the review the chips are the session's,
//!   read only, and while Preparing or Loading, and in a kept session, the
//!   sheet keeps D1's rows (R-74). Dependents and Children are always D1's
//!   rows (R-54).
//! - **A change** goes into the session's fields through `change_fields`,
//!   which marks the tab dirty and feeds the draft exactly as typing does.
//!   Add… and Change… choose from the `picker`.
//! - **The check** (R-26). Every change runs `check_ticket_relationships`
//!   over the whole proposed set, under the tab's stamp, off the UI thread;
//!   an answer asked before a newer change is dropped when it lands. A set
//!   equal to the base's is not checked: its save sends both as
//!   `Unchanged`, which the library does not check. The check is advisory:
//!   it reads the index only, and while one runs nothing waits.
//! - **A rejection** is explained under the field whose change it
//!   answered, naming its tickets by short code and title where a loaded
//!   row gives them, else by ID, and every save waits through the one gate
//!   (`save_blocked`) while the session proposes what it answered. A save
//!   the library refused for its relationships, with its IDs in
//!   `rejected`, is held and explained the same way, under Waiting on when
//!   the dependencies it sent differ from the base, else under Parent. The
//!   session stays dirty; nothing is lost.
//! - **Notes.** An answer from an index that is not current, or
//!   incomplete, adds "Save checks again" under the relationships.
//!
//! Everything here is drawn in the sheet's body, a `draft-section` while
//! editing (R-38). IDs, short codes and titles are field values, so
//! nothing here derives `Debug`.

mod picker;

use std::{collections::HashMap, path::PathBuf};

use gpui_kit::{App, Context, FocusHandle, Task, Window};
use manyhands::{
    canonical::ItemId,
    repository::{ItemDtoKind, RelationshipCheckDto, RelationshipWrite},
    results::ResultCode,
};

use super::ItemPane;
use crate::desktop::{
    model::{
        navigation::OpenTarget,
        relations_edit::{
            Answer, Candidate, Proposed, RelationField, explanation, save_field, save_reason,
        },
        stamp::{Generations, Stamp, ViewScope},
    },
    services::{
        ReadKind, ReadResult,
        requests::{DesktopRequestId, Intent},
    },
};

/// The tab's relationship edits: the check and its answer, the tickets the
/// picker has named, the open picker, and the controls' focus.
pub(super) struct RelationsEdit {
    scope: ViewScope,
    /// The check's stamps: every change moves them on.
    generations: Generations,
    /// The picker's reads' stamps, kept from one picker to the next.
    lists: Generations,
    slugs: Generations,
    /// The check in flight; replacing it drops the last.
    check: Option<Task<()>>,
    /// The proposal last checked, or held as it was.
    asked: Option<Proposed>,
    answer: Option<Answer>,
    /// Tickets as the picker's reads named them, by ID.
    known: HashMap<String, Candidate>,
    picker: Option<picker::Picker>,
    pub(super) add: FocusHandle,
    pub(super) change: FocusHandle,
    pub(super) clear: FocusHandle,
    /// One per chip, by its place.
    pub(super) remove: Vec<FocusHandle>,
}

impl RelationsEdit {
    /// The edits of a tab whose reads are stamped under `scope`.
    pub(super) fn new(scope: ViewScope, cx: &mut App) -> Self {
        Self {
            scope,
            generations: Generations::default(),
            lists: Generations::default(),
            slugs: Generations::default(),
            check: None,
            asked: None,
            answer: None,
            known: HashMap::new(),
            picker: None,
            add: cx.focus_handle().tab_stop(true),
            change: cx.focus_handle().tab_stop(true),
            clear: cx.focus_handle().tab_stop(true),
            remove: Vec::new(),
        }
    }
}

impl ItemPane {
    /// This tab's ticket's ID.
    fn own_id(&self) -> Option<&ItemId> {
        match self.model.target() {
            OpenTarget::Item {
                kind: ItemDtoKind::Ticket,
                id,
                ..
            } => Some(id),
            _ => None,
        }
    }

    /// What the session proposes now, kept or not, and what its base does.
    fn proposals(&self) -> Option<(Proposed, Proposed)> {
        let (meta, base) = self.session_fields()?;
        Some((Proposed::of(meta)?, Proposed::of(base)?))
    }

    /// The answer that holds now: of what the session proposes, where that
    /// differs from its base.
    fn current_answer(&self) -> Option<&Answer> {
        let answer = self.relations.answer.as_ref()?;
        let (proposed, base) = self.proposals()?;
        (answer.proposed == proposed && proposed != base).then_some(answer)
    }

    /// Why every save waits for the relationships: a rejection held for
    /// what the session proposes now (`save_blocked`). Not while a check
    /// runs: the check is advisory, and the library checks again.
    pub(super) fn relations_blocked(&self) -> Option<&'static str> {
        save_reason(self.current_answer()?.rejection.as_ref()?.code)
    }

    /// A control's change: the session proposes `proposed`, changed under
    /// `field`, and it is checked.
    fn change_relations(
        &mut self,
        proposed: Proposed,
        field: RelationField,
        cx: &mut Context<Self>,
    ) {
        if !self.controls_editable() {
            return;
        }
        let Some(meta) = self.session_meta() else {
            return;
        };
        let changed = proposed.applied_to(meta);
        if changed == *meta {
            cx.notify();
            return;
        }
        self.change_fields(changed, cx);
        self.check_relations(field, cx);
    }

    /// Checks what the session proposes now, under a new stamp, so any
    /// answer asked before it is dropped when it lands (R-26).
    #[allow(clippy::result_large_err)]
    fn check_relations(&mut self, field: RelationField, cx: &mut Context<Self>) {
        let stamp = self
            .relations
            .generations
            .next(self.relations.scope.clone());
        self.relations.answer = None;
        self.relations.check = None;
        let Some((proposed, base)) = self.proposals() else {
            return;
        };
        self.relations.asked = Some(proposed.clone());
        if proposed == base {
            return;
        }
        let (Some(request), Some(id)) = (proposed.request(), self.own_id().cloned()) else {
            return;
        };
        let root = PathBuf::from(self.repo());
        self.relations.check = Some(self.services.read(
            ReadKind::RelationshipCheck,
            stamp,
            move |service| {
                let repo = service.resolve_repository(&root)?;
                service.check_ticket_relationships(&repo, &id, &request)
            },
            cx,
            move |pane: &mut Self, stamp, result, cx| {
                pane.check_landed(stamp, proposed, field, result, cx)
            },
        ));
        cx.notify();
    }

    /// The check landed: kept when it is the latest asked. One that could
    /// not answer says nothing; the save checks again.
    fn check_landed(
        &mut self,
        stamp: Stamp,
        proposed: Proposed,
        field: RelationField,
        result: ReadResult<RelationshipCheckDto>,
        cx: &mut Context<Self>,
    ) {
        if !self.relations.generations.is_current(&stamp) {
            return;
        }
        if let ReadResult::Ok(check) = result {
            let answer = Answer::of_check(proposed, field, check);
            // A save's refusal of the same proposal keeps its rejection.
            let answer = match &self.relations.answer {
                Some(held) => answer.after(held),
                None => answer,
            };
            self.relations.answer = Some(answer);
        }
        cx.notify();
    }

    /// A session opened: nothing an earlier one was answered holds, and a
    /// continued draft whose relationships differ from the base is checked.
    pub(super) fn relations_opened(&mut self, cx: &mut Context<Self>) {
        self.relations.picker = None;
        self.relations_rebased(cx);
    }

    /// The base moved under the session, as the review's Use these choices
    /// moves it, or a session opened: what it proposes is checked against
    /// the base it now has, changed or not (Task 21 fix 1).
    pub(super) fn relations_rebased(&mut self, cx: &mut Context<Self>) {
        self.relations.asked = None;
        self.follow_relations(cx);
    }

    /// The session ended: nothing it was answered, nor any ticket its
    /// picker named, outlives it (Task 21 fix 1).
    pub(super) fn relations_ended(&mut self) {
        let relations = &mut self.relations;
        relations.generations.next(relations.scope.clone());
        relations.check = None;
        relations.asked = None;
        relations.answer = None;
        relations.known.clear();
        relations.picker = None;
    }

    /// The session's proposal changed by another way than a control: a
    /// changed one is checked, under the field that changed; with nothing
    /// asked yet, under the field that differs from the base.
    fn follow_relations(&mut self, cx: &mut Context<Self>) {
        let Some((proposed, base)) = self.proposals() else {
            return;
        };
        let field = match &self.relations.asked {
            Some(asked) if *asked == proposed => return,
            Some(asked) if asked.deps == proposed.deps => RelationField::Parent,
            Some(_) => RelationField::WaitingOn,
            None => save_field(&proposed, &base),
        };
        self.check_relations(field, cx);
    }

    /// The save `request` was refused with `code`. A relationship rejection
    /// (fact 11), its IDs in `rejected`, is held and explained as the
    /// check's is, while the session proposes what it sent; a newer change
    /// keeps its own check. The refusal changes only the rejection: what
    /// the check said of the same proposal's unresolved IDs and index stays,
    /// and a check of it still running lands without undoing the refusal
    /// (`Answer::after`; Task 21 fix 1).
    pub(super) fn relations_refused(
        &mut self,
        request: DesktopRequestId,
        code: ResultCode,
        cx: &mut Context<Self>,
    ) {
        let Some((proposed, base)) = self.proposals() else {
            return;
        };
        let operations = self.services.operations().read(cx);
        let Some(Intent::TicketSave { deps, parent, .. }) =
            operations.request(request).map(|request| &request.intent)
        else {
            return;
        };
        let sent = Proposed {
            deps: match deps {
                RelationshipWrite::Unchanged => base.deps.clone(),
                RelationshipWrite::Clear => Vec::new(),
                RelationshipWrite::Set(deps) => deps.clone(),
            },
            parent: match parent {
                RelationshipWrite::Unchanged => base.parent.clone(),
                RelationshipWrite::Clear => None,
                RelationshipWrite::Set(parent) => Some(parent.clone()),
            },
        };
        let ids = operations.rejected(request).to_vec();
        if sent != proposed {
            return;
        }
        let last = self.relations.answer.as_ref();
        let Some(answer) = Answer::of_save(sent, &base, code, ids, last) else {
            return;
        };
        self.relations.asked = Some(proposed);
        self.relations.answer = Some(answer);
        cx.notify();
    }

    /// What names the ticket `id`: the ticket itself as read, then D1's rows
    /// of its relationships, from the item's own reads; the picker's reads
    /// only for a ticket those do not have (Task 21 fix 1). `None` where
    /// nothing loaded names it.
    pub(super) fn relation_candidate(&self, id: &str) -> Option<Candidate> {
        let item = self.model.item().rows();
        if let Some(item) = item.filter(|item| item.id.as_deref() == Some(id)) {
            return Candidate::of_item(item);
        }
        let row = self.model.relationships().and_then(|relationships| {
            relationships
                .dependency_rows()
                .into_iter()
                .chain(relationships.parent_row())
                .chain(relationships.dependent_rows())
                .chain(relationships.child_rows())
                .find(|row| row.id == id && row.resolved)
        });
        match row {
            Some(row) => Some(Candidate {
                id: row.id,
                slug: row.slug,
                title: row.title,
                lifecycle: row.lifecycle,
            }),
            None => self.relations.known.get(id).cloned(),
        }
    }

    /// The ticket `id` by short code and title, or by its ID.
    fn relation_name(&self, id: &str) -> String {
        self.relation_candidate(id)
            .map_or_else(|| id.to_owned(), |candidate| candidate.named())
    }

    /// Whether no ticket the index holds has the ID `id`: as the answer
    /// that holds says, or else as the item's read says of its own
    /// dependencies and parent. A ticket neither names is taken as found.
    pub(super) fn relation_unresolved(&self, id: &str) -> bool {
        if let Some(answer) = self.current_answer() {
            return answer.unresolved.iter().any(|unresolved| unresolved == id);
        }
        self.model.relationships().is_some_and(|relationships| {
            relationships
                .dependency_rows()
                .into_iter()
                .chain(relationships.parent_row())
                .any(|row| row.id == id && !row.resolved)
        })
    }

    /// The words of the rejection that holds, when it is explained under
    /// `field`: its tickets named as `relation_name` names them.
    pub(super) fn relation_explanation(&self, field: RelationField) -> Option<String> {
        let answer = self
            .current_answer()
            .filter(|answer| answer.field == field)?;
        let own = self.own_id()?.to_string();
        explanation(answer.rejection.as_ref()?, field, &own, |id| {
            self.relation_name(id)
        })
    }

    /// Whether the answer that holds came from an index that is behind or
    /// incomplete: the save checks again.
    pub(super) fn relations_partial(&self) -> bool {
        self.current_answer().is_some_and(|answer| answer.partial)
    }

    /// Remove: the dependency at `ix` goes, and focus moves to Add….
    pub(super) fn remove_dependency(
        &mut self,
        ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(proposed) = self.session_meta().and_then(Proposed::of) else {
            return;
        };
        self.change_relations(
            proposed.without_dependency_at(ix),
            RelationField::WaitingOn,
            cx,
        );
        window.focus(&self.relations.add, cx);
    }

    /// Clear: the ticket has no parent, and focus moves to Change….
    pub(super) fn clear_parent(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(proposed) = self.session_meta().and_then(Proposed::of) else {
            return;
        };
        self.change_relations(proposed.with_parent(None), RelationField::Parent, cx);
        window.focus(&self.relations.change, cx);
    }
}
