//! The closed-ticket guard (C5): a new write to a ticket that carries
//! closure fields is refused. The ticket is read from Git only, from the
//! local primary branch tip and from the context branch tip when that
//! exists, never from a working tree: a dirty draft cannot hide a closure.
//! A copy is closed exactly as the closure observation decides
//! `AlreadyClosed`: a file naming either closure key.
use super::{
    RepositoryError, RepositoryErrorKind, RepositoryOperation,
    closure::observe::{self, FileClosure},
    promotion::lifecycle::LifecycleKind,
};
use crate::canonical::ItemId;

/// Refuses with `TicketClosed` when either branch tip holds `item` with
/// closure fields. A branch that does not exist, and a tip without the
/// ticket, hold no closed copy. The error's operation and root are the
/// caller's to set.
pub(super) fn require_ticket_open(
    repo: &git2::Repository,
    primary_branch: &str,
    item: &ItemId,
) -> Result<(), RepositoryError> {
    let root = repo.workdir().unwrap_or_else(|| repo.path());
    let path = observe::ticket_path(item);
    let context_branch = LifecycleKind::TicketClosure.context_branch(item);
    for branch in [primary_branch, context_branch.as_str()] {
        let tip = match repo.find_branch(branch, git2::BranchType::Local) {
            Ok(branch) => branch.get().peel_to_commit(),
            Err(error) if error.code() == git2::ErrorCode::NotFound => continue,
            Err(error) => Err(error),
        }
        .map_err(|error| {
            RepositoryError::git(RepositoryOperation::Read, Some(root.into()), error)
        })?;
        // The reader's own error text is the closure observation's; the
        // guard keeps its kind and says what it was doing.
        let ticket = observe::tree_ticket(repo, tip.tree_id(), &path, root).map_err(|error| {
            RepositoryError::new(
                RepositoryOperation::Read,
                Some(root.into()),
                error.kind,
                "the closed-ticket guard could not read the ticket from Git",
            )
        })?;
        let Some(ticket) = ticket else {
            continue;
        };
        if !matches!(observe::file_closure(&ticket.bytes), FileClosure::Open) {
            return Err(RepositoryError::new(
                RepositoryOperation::Read,
                Some(root.into()),
                RepositoryErrorKind::TicketClosed,
                "the ticket is closed",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "ticket_guard_tests.rs"]
mod tests;
