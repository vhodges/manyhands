//! The setup gate (Wave 06 Cycle 02, design Part B): while a repository's
//! setup is pending, the commands that must wait for it are refused with
//! `setup_pending`.
//!
//! The gate runs on a first call before the binding's `prepare`, on a
//! `prepare` before its preview, and inside the acceptance transaction. It
//! does not run on the re-entry or replay of a request already accepted, so
//! a setup's own request continues past it.
//!
//! A setup is pending while its record exists and the local primary does not
//! yet hold it (`is_complete`). A complete setup is read as none; finishing
//! it is the setup completion's work, not the gate's, so `check` and
//! `check_in` are read-only: no lock, no write. A repository the registry
//! does not hold, or holds with no record, refuses nothing. A record this
//! build cannot read refuses what the gate refuses, as `recovery_required`,
//! and lets through what carries on, so the way out (`repo remove`) runs.

use std::path::Path;

use rusqlite::{Connection, OptionalExtension};

use super::{
    Command,
    bind::{Answer, Place},
    command::SetupGate,
};
use crate::{
    repository::{
        ReadError, RepositoryOperation, RepositoryService,
        setup::{
            complete,
            record::{self, SetupKind, SetupRecord},
        },
    },
    results::ResultCode,
};

/// The root a place names, for a request that works in a repository.
pub(super) fn root_of(place: Place<'_>) -> Option<&Path> {
    match place {
        Place::Repository(root) | Place::NewRepository(root) => Some(root),
        Place::Application => None,
    }
}

/// Refuses `command` at `root` while a setup is pending, for a first call
/// and a prepare. A registry that cannot be read here refuses nothing: the
/// command's own reads answer for it as they always have.
pub(super) fn check(
    service: &RepositoryService,
    command: Command,
    root: &Path,
) -> Result<(), Answer> {
    if command.setup_gate() == SetupGate::Carries {
        return Ok(());
    }
    service
        .read_session(RepositoryOperation::Read, |connection| {
            Ok(look(connection, command, root))
        })
        .unwrap_or(Ok(()))
}

/// `check`, inside the acceptance transaction: the same refusal from the
/// same record, so a setup recorded between a first call's check and its
/// acceptance still refuses. Read-only.
pub(super) fn check_in(
    tx: &rusqlite::Transaction<'_>,
    command: Command,
    root: &Path,
) -> Result<(), Answer> {
    look(tx, command, root)
}

/// `check_in` for the acceptance transactions, which fail with a read
/// error: the refusal travels as its code.
pub(super) fn admit(
    tx: &rusqlite::Transaction<'_>,
    command: Command,
    place: Place<'_>,
) -> Result<(), ReadError> {
    let Some(root) = root_of(place) else {
        return Ok(());
    };
    check_in(tx, command, root).map_err(|answer| ReadError::new(answer.code))
}

fn look(connection: &Connection, command: Command, root: &Path) -> Result<(), Answer> {
    let gate = command.setup_gate();
    if gate == SetupGate::Carries {
        return Ok(());
    }
    // The registry holds the canonical root. One that does not exist, or
    // cannot be written as text, is none it holds.
    let Ok(root) = std::fs::canonicalize(root) else {
        return Ok(());
    };
    let Some(key) = root.to_str() else {
        return Ok(());
    };
    let unreadable = || Answer::stopped(ResultCode::RecoveryRequired);
    let Some(repository_id) = connection
        .query_row(
            "SELECT id FROM repositories WHERE root_path = ?1",
            [key],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|_| unreadable())?
    else {
        return Ok(());
    };
    let Some(record) = record::read(connection, repository_id).map_err(|_| unreadable())? else {
        return Ok(());
    };
    if gate == SetupGate::RefusedBeforeEnable && record.kind != SetupKind::Enable {
        return Ok(());
    }
    if holds(&root, &record) {
        return Ok(());
    }
    Err(Answer::stopped(ResultCode::SetupPending))
}

/// Whether the local primary at `root` already holds the setup. A
/// repository Git cannot open or read is not known to hold it.
fn holds(root: &Path, record: &SetupRecord) -> bool {
    git2::Repository::open(root)
        .and_then(|repository| complete::is_complete(&repository, record))
        .unwrap_or(false)
}

#[cfg(test)]
#[path = "setup_gate_tests.rs"]
mod tests;
