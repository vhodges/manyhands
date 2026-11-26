//! The closed-ticket guard (C5): a new write to a ticket that carries
//! closure fields is refused. The ticket is read from Git only, from the
//! local primary branch tip and, in a direct repository, from the context
//! branch tip when that exists, never from a working tree: a dirty draft
//! cannot hide a closure. In a pull-request repository the closure of a
//! closing ticket is only on its branch until the pull request merges, so
//! the guard reads the primary alone and the ticket takes writes meanwhile.
//! A copy is closed exactly as the closure observation decides
//! `AlreadyClosed`: a file naming either closure key.
use super::{
    RepositoryError, RepositoryErrorKind, RepositoryOperation,
    closure::observe::{self, FileClosure},
    promotion::lifecycle::LifecycleKind,
};
use crate::canonical::{self, CanonicalItem, Integration, ItemId};

/// Refuses with `TicketClosed` when `item` has closure fields at the local
/// primary's tip, or, in direct mode, at the context branch's tip too. A
/// branch that does not exist, and a tip without the ticket, hold no closed
/// copy. The error's operation and root are the caller's to set.
pub(super) fn require_ticket_open(
    repo: &git2::Repository,
    primary_branch: &str,
    item: &ItemId,
    integration: Integration,
) -> Result<(), RepositoryError> {
    let root = repo.workdir().unwrap_or_else(|| repo.path());
    let path = observe::ticket_path(item);
    let context_branch = LifecycleKind::TicketClosure.context_branch(item);
    let branches: &[&str] = match integration {
        Integration::Direct => &[primary_branch, context_branch.as_str()],
        Integration::PullRequest => &[primary_branch],
    };
    for &branch in branches {
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

/// Whether `ticket`, the result of a save in the context `repo` is the
/// worktree of, holds other `closed_at` or `closed_by` than the ticket at
/// the context branch's head. A head without the ticket holds an open one.
/// A head whose ticket does not parse and names a closure key counts as
/// changed, since nothing shows the save to leave it alone. The error's
/// operation and root are the caller's to set.
pub(super) fn closure_differs_from_head(
    repo: &git2::Repository,
    ticket: &canonical::Ticket,
) -> Result<bool, RepositoryError> {
    let root = repo.workdir().unwrap_or_else(|| repo.path());
    let path = observe::ticket_path(&ticket.id);
    let tree = repo
        .head()
        .and_then(|head| head.peel_to_commit())
        .map_err(|error| RepositoryError::git(RepositoryOperation::Read, Some(root.into()), error))?
        .tree_id();
    let held = observe::tree_ticket(repo, tree, &path, root)?;
    let head = match held.as_ref().map(|held| &held.bytes) {
        None => (None, None),
        Some(bytes) => {
            let parsed = std::str::from_utf8(bytes)
                .ok()
                .and_then(|source| canonical::parse_item(&path, source).ok());
            match parsed {
                Some(CanonicalItem::Ticket(head)) => (head.closed_at, head.closed_by),
                _ if matches!(observe::file_closure(bytes), FileClosure::Open) => (None, None),
                _ => return Ok(true),
            }
        }
    };
    Ok(head != (ticket.closed_at, ticket.closed_by.clone()))
}

#[cfg(test)]
#[path = "ticket_guard_tests.rs"]
mod tests;
