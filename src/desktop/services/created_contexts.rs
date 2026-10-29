//! The work contexts this desktop process made (decision 18, R-75): every
//! item whose `context prepare` this process saw answer `ok` with
//! `created: true`, and into which no save has committed since.
//!
//! It is one set for the process, kept by the services and fed from the
//! worker's answers as they land, so a context made for a tab that closed
//! while preparing is in it as well (R-37). Leaving and discarding consult
//! it (`release`); a context it does not hold is never released by them. A
//! prepare that answered `discovery_pending` enters it only once a retry
//! answers `ok` (R-77). A save, or a new item's create, that commits into a
//! context takes it out: it holds work now. A release leaves it in: the
//! release service keeps what it has released.

use std::{collections::HashSet, path::PathBuf};

use manyhands::{
    canonical::ItemId,
    repository::{AuthoringKind, ItemDtoKind, MutationDataDto},
    results::{CheckpointEffect, Envelope, ResultCode},
};

use super::requests::Intent;

/// An item's context, by its repository's registration root, its kind and
/// its ID.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct ContextKey {
    pub root: String,
    pub kind: ItemDtoKind,
    pub id: String,
}

impl ContextKey {
    pub(crate) fn new(root: &str, kind: ItemDtoKind, id: &str) -> Self {
        Self {
            root: root.to_owned(),
            kind,
            id: id.to_owned(),
        }
    }

    /// The item a context command, a save or a new item's create names.
    pub(crate) fn of(intent: &Intent) -> Option<Self> {
        let (root, kind, id) = match intent {
            Intent::ContextPrepare { root, kind, id, .. }
            | Intent::ContextRelease { root, kind, id } => (root, dto_kind(*kind), id),
            Intent::DocumentSave { root, id, .. } | Intent::DocumentCreate { root, id, .. } => {
                (root, ItemDtoKind::Document, id)
            }
            Intent::TicketSave { root, id, .. } | Intent::TicketCreate { root, id, .. } => {
                (root, ItemDtoKind::Ticket, id)
            }
            _ => return None,
        };
        Some(Self::new(root.to_str()?, kind, &id.to_string()))
    }

    /// Whether `intent` prepares this item's context.
    pub(crate) fn prepared_by(&self, intent: &Intent) -> bool {
        matches!(intent, Intent::ContextPrepare { .. }) && Self::of(intent).as_ref() == Some(self)
    }

    /// Whether `intent` releases this item's context.
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 24.
    pub(crate) fn released_by(&self, intent: &Intent) -> bool {
        matches!(intent, Intent::ContextRelease { .. }) && Self::of(intent).as_ref() == Some(self)
    }

    /// Whether `intent` saves this item, or creates it.
    pub(crate) fn saved_by(&self, intent: &Intent) -> bool {
        matches!(
            intent,
            Intent::DocumentSave { .. }
                | Intent::TicketSave { .. }
                | Intent::DocumentCreate { .. }
                | Intent::TicketCreate { .. }
        ) && Self::of(intent).as_ref() == Some(self)
    }

    /// The `context release` of this item's context; `None` for an ID the
    /// item types do not take.
    pub(crate) fn release(&self) -> Option<Intent> {
        let id: ItemId = self.id.parse().ok()?;
        let kind = match self.kind {
            ItemDtoKind::Document => AuthoringKind::Document,
            ItemDtoKind::Ticket => AuthoringKind::Ticket,
        };
        Some(Intent::ContextRelease {
            root: PathBuf::from(&self.root),
            kind,
            id,
        })
    }
}

fn dto_kind(kind: AuthoringKind) -> ItemDtoKind {
    match kind {
        AuthoringKind::Document => ItemDtoKind::Document,
        AuthoringKind::Ticket => ItemDtoKind::Ticket,
    }
}

/// The contexts this process made and saved nothing into.
#[derive(Default)]
pub(crate) struct CreatedContexts {
    contexts: HashSet<ContextKey>,
}

impl CreatedContexts {
    /// Whether this process made `key`'s context and saved nothing into it.
    pub(crate) fn contains(&self, key: &ContextKey) -> bool {
        self.contexts.contains(key)
    }

    /// A finished request: a `context prepare` that answered `ok` with
    /// `created: true` adds its item's context, and a save or a new item's
    /// create whose checkpoint committed takes its item's out (R-75),
    /// whether or not its tab is still open. Any other answer, and any
    /// other request, changes nothing. True when the answer made the
    /// context: a tab that closed while it prepared releases it then
    /// (R-37).
    pub(crate) fn finished(
        &mut self,
        intent: &Intent,
        envelope: &Envelope<MutationDataDto>,
    ) -> bool {
        let Some(key) = ContextKey::of(intent) else {
            return false;
        };
        match intent {
            Intent::DocumentSave { .. }
            | Intent::TicketSave { .. }
            | Intent::DocumentCreate { .. }
            | Intent::TicketCreate { .. } => {
                if envelope.effects.checkpoint == CheckpointEffect::Committed {
                    self.remove(&key);
                }
                false
            }
            Intent::ContextPrepare { .. } => {
                let created = envelope.code == ResultCode::Ok
                    && matches!(&envelope.data, Some(MutationDataDto::Context(data)) if data.created);
                if created {
                    self.contexts.insert(key);
                }
                created
            }
            _ => false,
        }
    }

    /// A save committed into `key`'s context: it no longer counts as made
    /// and unused.
    pub(crate) fn remove(&mut self, key: &ContextKey) -> bool {
        self.contexts.remove(key)
    }
}

#[cfg(test)]
#[path = "created_contexts_tests.rs"]
mod tests;
