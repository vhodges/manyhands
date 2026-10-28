//! The work contexts this desktop process made (decision 18, R-75): every
//! item whose `context prepare` this process saw answer `ok` with
//! `created: true`, and into which no save has committed since.
//!
//! It is one set for the process, kept by the services and fed from the
//! worker's answers as they land, so a context made for a tab that closed
//! while preparing is in it as well (R-37). Leaving and discarding consult
//! it; a context it does not hold is never released by them. A prepare
//! that answered `discovery_pending` enters it only once a retry answers
//! `ok` (R-77). A save that commits into a context takes it out: it holds
//! work now.

use std::collections::HashSet;

use manyhands::{
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
}

/// The contexts this process made and saved nothing into.
#[derive(Default)]
pub(crate) struct CreatedContexts {
    contexts: HashSet<ContextKey>,
}

impl CreatedContexts {
    /// Whether this process made `key`'s context and saved nothing into it.
    #[cfg_attr(not(test), allow(dead_code))] // Called from Task 17.
    pub(crate) fn contains(&self, key: &ContextKey) -> bool {
        self.contexts.contains(key)
    }

    /// A finished request: a `context prepare` that answered `ok` with
    /// `created: true` adds its item's context, and a save whose checkpoint
    /// committed takes its item's out (R-75), whether or not its tab is
    /// still open. Any other answer, and any other request, changes
    /// nothing.
    pub(crate) fn finished(&mut self, intent: &Intent, envelope: &Envelope<MutationDataDto>) {
        let saved = match intent {
            Intent::DocumentSave { root, id, .. } => Some((root, ItemDtoKind::Document, id)),
            Intent::TicketSave { root, id, .. } => Some((root, ItemDtoKind::Ticket, id)),
            _ => None,
        };
        if let Some((root, kind, id)) = saved {
            if envelope.effects.checkpoint == CheckpointEffect::Committed
                && let Some(root) = root.to_str()
            {
                self.remove(&ContextKey::new(root, kind, &id.to_string()));
            }
            return;
        }
        let Intent::ContextPrepare { root, kind, id, .. } = intent else {
            return;
        };
        let created = envelope.code == ResultCode::Ok
            && matches!(&envelope.data, Some(MutationDataDto::Context(data)) if data.created);
        let Some(root) = root.to_str() else {
            return;
        };
        if created {
            let kind = match kind {
                AuthoringKind::Document => ItemDtoKind::Document,
                AuthoringKind::Ticket => ItemDtoKind::Ticket,
            };
            self.contexts
                .insert(ContextKey::new(root, kind, &id.to_string()));
        }
    }

    /// A save committed into `key`'s context, or it was released: it no
    /// longer counts as made and unused.
    pub(crate) fn remove(&mut self, key: &ContextKey) -> bool {
        self.contexts.remove(key)
    }
}

#[cfg(test)]
#[path = "created_contexts_tests.rs"]
mod tests;
