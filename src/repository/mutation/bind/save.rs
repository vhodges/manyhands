//! What any binding of an authoring save derives from the repository: the
//! effects a result reports, the commit it names and what the call leaves.
//!
//! A save writes one canonical file in an item's editing context and
//! commits it there. Nothing here knows which item or which command: a
//! binding gives its file's path, the file's bytes as its context holds
//! them now, and how to tell that bytes are what the request intends.

use std::path::Path;

use git2::Oid;

use crate::results::{Effects, ResultCode};

use super::{
    super::{
        super::{ExpectedPathObservation, recovery::JournalRow},
        evidence::{self, Claim, Position, Unreadable},
        outcome::{self, SaveReport},
    },
    Standing,
};

/// The code and effects of a save that left nothing to do, with the commit
/// the evidence shows to be the request's, if it made one.
pub(crate) fn settled(commit: Option<Oid>) -> (ResultCode, Effects) {
    SaveReport::Saved {
        claimed: None,
        discovered: true,
    }
    .result(commit)
}

/// The commit a first call of a save reports: the one the domain names,
/// when Git shows it to be the request's. A save that changed nothing can
/// name a commit it did not make, and that is no commit of the request's.
pub(crate) fn confirmed(
    root: &Path,
    position: &Position,
    path: &str,
    claimed: Option<Oid>,
) -> Result<Option<Oid>, Unreadable> {
    match claimed.map(|claimed| (claimed, evidence::confirms(root, position, claimed, path))) {
        Some((claimed, Claim::Confirmed)) => Ok(Some(claimed)),
        Some((_, Claim::NotThisRequests)) | None => Ok(None),
        Some((_, Claim::Unreadable)) => Err(Unreadable),
    }
}

/// What a save that returned a result leaves. `journal` reads the
/// operation's row, and is asked only when the save stopped for want of
/// an identity.
pub(crate) fn standing(
    report: SaveReport,
    journal: impl FnOnce() -> Option<JournalRow>,
) -> Standing {
    match report {
        SaveReport::IdentityRequired => Standing::Stopped { journal: journal() },
        SaveReport::Saved {
            discovered: true, ..
        } => Standing::Final,
        SaveReport::Saved {
            discovered: false, ..
        } => Standing::Owed,
    }
}

/// The effects of a save that stopped with an error.
///
/// The domain returns the same kinds before and after an effect, so what
/// was done is read from the journal row and the repository. `journal` is
/// the operation's row after the call; one that could not be read is taken
/// to be in flight. With nothing in flight there is no effect. Otherwise
/// `commit` gives the request's commit, and is asked only then; and the
/// file was written when `file`, its bytes in the editing context, is what
/// the request intends and is not what it `expected` to find.
pub(crate) fn stopped_effects(
    journal: Option<&JournalRow>,
    commit: impl FnOnce() -> Option<Oid>,
    file: Option<&[u8]>,
    expected: &ExpectedPathObservation,
    holds: &dyn Fn(&[u8]) -> bool,
) -> Effects {
    if !journal.is_none_or(JournalRow::in_flight) {
        return Effects::not_requested();
    }
    let commit = commit();
    let written = file.is_some_and(|bytes| {
        ExpectedPathObservation::from_bytes(bytes) != *expected && holds(bytes)
    });
    outcome::stopped_save_effects(commit, commit.is_some() || written)
}

/// What the repository shows of an unfinished save whose operation ran,
/// without the request's input: a checkpoint when a commit in range
/// changed the file at `path`, and a write when `file`, its bytes in the
/// editing context, is no longer what the request `expected` to find.
///
/// A file that is not there was not written: with no editing context, or
/// no file in it, nothing is reported, whatever the request expected. A
/// range that cannot be read shows no commit.
pub(crate) fn recorded_effects(
    root: &Path,
    position: &Position,
    path: &str,
    file: Option<&[u8]>,
    expected: Option<&ExpectedPathObservation>,
) -> Effects {
    let commit = evidence::path_commit(root, position, path).ok().flatten();
    let written = file.is_some_and(|bytes| {
        expected.is_some_and(|expected| ExpectedPathObservation::from_bytes(bytes) != *expected)
    });
    outcome::stopped_save_effects(commit, written)
}
