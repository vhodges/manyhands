//! A ticket's dependencies and parent while editing, as pure decisions
//! (design, Relationships (contract 10); R-26, R-72): what a draft
//! proposes, a dependency added or removed, what the relationship check
//! answered and the words that explain it, why Save waits, and the tickets
//! the picker offers.
//!
//! - **Order** (R-72). A draft's `deps` are in ID order, as the library
//!   writes them: a dependency goes in where its ID sorts, and one removed
//!   leaves the rest as they were, so adding and then removing one returns
//!   to the base exactly.
//! - **The check** (R-26) is advisory: it reads the index only, and the
//!   save checks again. A rejection names a set of IDs, never a path; the
//!   ticket alone is the ticket naming itself.
//! - **Which field.** The answer does not say whether a cycle runs through
//!   `deps` or `parent`: it is explained under the field whose change it
//!   answered, and a save's under Waiting on when the `deps` it sent differ
//!   from the base, else under Parent.
//! - **The picker** offers the listed tickets whose title holds the filter,
//!   ignoring case, after those whose short code is exactly it, as the
//!   library finds them (`find_tickets_by_slug`).
//!
//! Every text is fixed or the library's own. IDs, short codes and titles
//! are field values, so nothing here that holds one derives `Debug`.

use manyhands::{
    canonical::ItemId,
    drafts::DraftFields,
    repository::{
        ClosureState, IndexState, ItemDto, ItemListDto, ProposedRelationships,
        RelationshipCheckDto, RelationshipRejectionDto,
    },
    results::ResultCode,
};

use super::{names::TicketName, tickets::Lifecycle};

/// Waiting on's control that opens the picker.
pub(crate) const ADD: &str = "Add\u{2026}";
/// Parent's control that opens the picker.
pub(crate) const CHANGE: &str = "Change\u{2026}";
/// Parent's control that removes it.
pub(crate) const CLEAR: &str = "Clear";
/// A dependency's control that removes it.
pub(crate) const REMOVE: &str = "Remove";
/// The picker's filter box, before anything is typed.
pub(crate) const FILTER: &str = "Filter by title, or type a short code";
/// The picker with nothing to offer.
pub(crate) const NO_MATCH: &str = "No ticket matches";
/// The picker whose list could not be read.
pub(crate) const NOT_LISTED: &str = "Could not list the tickets";

/// The ticket alone in a cycle, under Waiting on.
pub(crate) const CANNOT_DEPEND_ON_ITSELF: &str = "A ticket cannot depend on itself.";
/// The ticket alone in a cycle, under Parent.
pub(crate) const CANNOT_BE_OWN_PARENT: &str = "A ticket cannot be its own parent.";
/// Save's reason for a cycle: its tooltip and the toolbar's note.
pub(crate) const SAVE_CYCLE: &str = "A ticket cannot be saved with a relationship cycle.";
/// Save's reason for `invalid_relationship`.
pub(crate) const SAVE_INVALID: &str = "A ticket's dependencies and parent must be tickets.";
/// Save's reason while a check of what the session proposes runs (Task 27
/// fix 1, B4; contract 10).
pub(crate) const SAVE_CHECKING: &str = "Save waits while the relationships are checked.";
/// An unresolved dependency or parent, on its chip or row.
pub(crate) const NOT_FOUND_KEPT: &str = "not found, kept";
/// An answer from an index that is behind or incomplete, under the
/// relationships.
pub(crate) const SAVE_CHECKS_AGAIN: &str = "The index may be missing tickets. Save checks again.";

/// How many tickets the picker offers at most.
const OFFERED: usize = 8;

/// A cycle, under its field: `names` are its tickets, joined with ", ".
pub(crate) fn makes_a_cycle(names: &str) -> String {
    format!("This makes a cycle: {names}.")
}

/// `invalid_relationship`, under its field: `names` are the IDs that are
/// not tickets, joined with ", ".
pub(crate) fn not_a_ticket(names: &str) -> String {
    format!("Not a ticket: {names}.")
}

/// The field a relationship change, and so its answer, is under.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum RelationField {
    WaitingOn,
    Parent,
}

/// A ticket's dependencies and parent as a draft proposes them, by ID.
#[derive(Clone, Default, PartialEq, Eq)]
pub(crate) struct Proposed {
    pub(crate) deps: Vec<String>,
    pub(crate) parent: Option<String>,
}

impl Proposed {
    /// What `fields` propose; `None` for a document's or a comment's.
    pub(crate) fn of(fields: &DraftFields) -> Option<Self> {
        match fields {
            DraftFields::Ticket { deps, parent, .. } => Some(Self {
                deps: deps.clone(),
                parent: parent.clone(),
            }),
            _ => None,
        }
    }

    /// `fields` proposing these instead; a document's or a comment's stay
    /// as they are.
    pub(crate) fn applied_to(&self, fields: &DraftFields) -> DraftFields {
        let mut fields = fields.clone();
        if let DraftFields::Ticket { deps, parent, .. } = &mut fields {
            deps.clone_from(&self.deps);
            parent.clone_from(&self.parent);
        }
        fields
    }

    /// With `id` among the dependencies, where its ID sorts (R-72); as it
    /// is when `id` is one already.
    pub(crate) fn with_dependency(&self, id: &str) -> Self {
        let mut next = self.clone();
        if !next.deps.iter().any(|dep| dep == id) {
            let at = next
                .deps
                .iter()
                .position(|dep| dep.as_str() > id)
                .unwrap_or(next.deps.len());
            next.deps.insert(at, id.to_owned());
        }
        next
    }

    /// Without the dependency at `ix`; the others stay as they were.
    pub(crate) fn without_dependency_at(&self, ix: usize) -> Self {
        let mut next = self.clone();
        if ix < next.deps.len() {
            next.deps.remove(ix);
        }
        next
    }

    /// With `parent`, or none.
    pub(crate) fn with_parent(&self, parent: Option<&str>) -> Self {
        Self {
            parent: parent.map(str::to_owned),
            ..self.clone()
        }
    }

    /// The check's input. `None` when one of the IDs is not an ID: the
    /// save then decides.
    pub(crate) fn request(&self) -> Option<ProposedRelationships> {
        let deps = self
            .deps
            .iter()
            .map(|id| id.parse().ok())
            .collect::<Option<Vec<ItemId>>>()?;
        let parent = match &self.parent {
            Some(parent) => Some(parent.parse().ok()?),
            None => None,
        };
        Some(ProposedRelationships { deps, parent })
    }
}

/// Why a save of a proposal is refused: the library's code and IDs.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Rejection {
    pub(crate) code: ResultCode,
    /// In ID order, each once: a cycle's tickets, the ticket included, or
    /// the IDs that are not tickets.
    pub(crate) ids: Vec<String>,
}

/// What the check, or a save, answered of a proposal.
#[derive(Clone)]
pub(crate) struct Answer {
    pub(crate) proposed: Proposed,
    /// Where it is explained.
    pub(crate) field: RelationField,
    pub(crate) rejection: Option<Rejection>,
    /// The proposed IDs nothing the index holds has: accepted, and kept.
    pub(crate) unresolved: Vec<String>,
    /// It was made from an index that is not current, or incomplete, so it
    /// may lack tickets: the save checks again.
    pub(crate) partial: bool,
    /// Its rejection is a save's refusal, not the check's.
    pub(crate) refused: bool,
}

impl Answer {
    /// The check's answer for `proposed`, changed under `field`.
    pub(crate) fn of_check(
        proposed: Proposed,
        field: RelationField,
        check: RelationshipCheckDto,
    ) -> Self {
        Self {
            proposed,
            field,
            rejection: check.rejection.map(|rejection| Rejection {
                code: rejection.code,
                ids: rejection.ids,
            }),
            unresolved: check.unresolved,
            partial: !check.complete || check.index.state != IndexState::Current,
            refused: false,
        }
    }

    /// A save of `proposed` over `base` answered `code` with `ids` in its
    /// `rejected`; `None` for a code no rejection carries. The refusal
    /// changes only the rejection: what `last`, the check's answer of the
    /// same proposal, said of unresolved IDs and the index is kept (Task 21
    /// fix 1). With no such answer it says nothing of them.
    pub(crate) fn of_save(
        proposed: Proposed,
        base: &Proposed,
        code: ResultCode,
        ids: Vec<String>,
        last: Option<&Self>,
    ) -> Option<Self> {
        if !RelationshipRejectionDto::CODES.contains(&code) {
            return None;
        }
        let (unresolved, partial) = last
            .filter(|last| last.proposed == proposed)
            .map(|last| (last.unresolved.clone(), last.partial))
            .unwrap_or_default();
        Some(Self {
            field: save_field(&proposed, base),
            proposed,
            rejection: Some(Rejection { code, ids }),
            unresolved,
            partial,
            refused: true,
        })
    }

    /// This, a check's answer that landed after `held`, a save's refusal of
    /// the same proposal: the refusal's rejection, and the field it is
    /// explained under, stay; this check's unresolved IDs and index are
    /// taken. Any other `held` changes nothing.
    pub(crate) fn after(self, held: &Self) -> Self {
        if !held.refused || held.proposed != self.proposed {
            return self;
        }
        Self {
            field: held.field,
            rejection: held.rejection.clone(),
            refused: true,
            ..self
        }
    }
}

/// Where a save's answer is explained: under Waiting on when the
/// dependencies it sent differ from the base's, else under Parent.
pub(crate) fn save_field(sent: &Proposed, base: &Proposed) -> RelationField {
    if sent.deps == base.deps {
        RelationField::Parent
    } else {
        RelationField::WaitingOn
    }
}

/// Save's reason for `code`, a rejection's.
pub(crate) fn save_reason(code: ResultCode) -> Option<&'static str> {
    match code {
        ResultCode::RelationshipCycle => Some(SAVE_CYCLE),
        ResultCode::InvalidRelationship => Some(SAVE_INVALID),
        _ => None,
    }
}

/// The words under `field` for `rejection` of the ticket `own`, each ticket
/// named by `name`. The ticket alone is the ticket naming itself.
pub(crate) fn explanation(
    rejection: &Rejection,
    field: RelationField,
    own: &str,
    name: impl Fn(&str) -> String,
) -> Option<String> {
    let names = || {
        rejection
            .ids
            .iter()
            .map(|id| name(id))
            .collect::<Vec<_>>()
            .join(", ")
    };
    match rejection.code {
        ResultCode::RelationshipCycle if rejection.ids.len() == 1 && rejection.ids[0] == own => {
            Some(
                match field {
                    RelationField::WaitingOn => CANNOT_DEPEND_ON_ITSELF,
                    RelationField::Parent => CANNOT_BE_OWN_PARENT,
                }
                .to_owned(),
            )
        }
        ResultCode::RelationshipCycle => Some(makes_a_cycle(&names())),
        ResultCode::InvalidRelationship => Some(not_a_ticket(&names())),
        _ => None,
    }
}

/// A ticket as a library row gives it: what the picker offers, and what
/// names a ticket elsewhere.
#[derive(Clone)]
pub(crate) struct Candidate {
    pub(crate) id: String,
    pub(crate) slug: Option<String>,
    pub(crate) title: Option<String>,
    pub(crate) lifecycle: Option<Lifecycle>,
}

impl Candidate {
    /// `item`, a ticket with an ID.
    pub(crate) fn of_item(item: &ItemDto) -> Option<Self> {
        Some(Self {
            id: item.id.clone()?,
            slug: item.slug.clone(),
            title: item.title.clone(),
            lifecycle: item.closure.as_ref().map(|closure| match closure.state {
                ClosureState::Open => Lifecycle::Open,
                ClosureState::Closed => Lifecycle::Closed,
            }),
        })
    }

    /// The tickets of `list` with an ID, in its order.
    pub(crate) fn of_list(list: &ItemListDto) -> Vec<Self> {
        list.items.iter().filter_map(Self::of_item).collect()
    }

    pub(crate) fn name(&self) -> TicketName<'_> {
        TicketName {
            slug: self.slug.as_deref(),
            title: self.title.as_deref(),
        }
    }

    /// Its short code and title, or its ID where it has neither.
    pub(crate) fn named(&self) -> String {
        self.name().with_short(&self.id)
    }
}

/// The tickets the picker offers while its filter holds `filter`: those
/// whose short code is exactly it (`exact`, the library's matches), then
/// those `listed` whose title holds it, ignoring case, every one while it
/// is empty; never one of `left_out`; each once; at most a handful.
pub(crate) fn offered(
    listed: &[Candidate],
    exact: &[Candidate],
    filter: &str,
    left_out: &[&str],
) -> Vec<Candidate> {
    let wanted = filter.trim().to_lowercase();
    let titled = listed.iter().filter(|candidate| {
        wanted.is_empty()
            || candidate
                .title
                .as_deref()
                .is_some_and(|title| title.to_lowercase().contains(&wanted))
    });
    let mut offered: Vec<Candidate> = Vec::new();
    for candidate in exact.iter().chain(titled) {
        if offered.len() == OFFERED {
            break;
        }
        if left_out.contains(&candidate.id.as_str())
            || offered.iter().any(|shown| shown.id == candidate.id)
        {
            continue;
        }
        offered.push(candidate.clone());
    }
    offered
}

#[cfg(test)]
#[path = "relations_edit_tests.rs"]
mod tests;
