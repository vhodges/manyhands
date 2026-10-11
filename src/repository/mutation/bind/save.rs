//! What any binding of an authoring save derives from the repository: the
//! effects a result reports, the commit it names and what the call leaves.
//!
//! A save writes one canonical file in an item's editing context and
//! commits it there. Nothing here knows which item or which command: a
//! binding gives its file's path, the file's bytes as its context holds
//! them now, and how to tell that bytes are what the request intends.

use std::path::{Path, PathBuf};

use git2::Oid;

use crate::results::{Effects, ResultCode};

use super::{
    super::{
        super::{
            ExpectedPathObservation, RepositoryError, RepositoryErrorKind, RepositoryService,
            SaveOutcome,
            recovery::{FinalKind, JournalRow},
        },
        evidence::{self, Claim, PathEvidence, Position, Unreadable},
        observe,
        outcome::{self, SaveReport},
    },
    Accepted, Answer, Before, Found, Ran, Standing,
};

/// What the binding of an authoring save gives the first call and the
/// re-entry of its request, which are the same for every such save.
pub(crate) trait Save {
    /// The path of the request's file in its editing context, with
    /// forward slashes: where the request's commit is judged.
    fn path(&self) -> String;

    /// The worktree of the item's editing context.
    fn worktree(&self, root: &Path) -> PathBuf;

    /// The request's file in its editing context, as it is now. `None`
    /// when there is no context, no file, or the file cannot be read.
    fn context_file(&self, root: &Path) -> Option<Vec<u8>>;

    /// Whether `bytes` are what the request intends its file to hold.
    fn holds_bytes(&self, bytes: &[u8]) -> bool;

    /// The one domain call, under the recorded operation ID and
    /// expectation.
    fn call_domain(
        &self,
        service: &RepositoryService,
        accepted: &Accepted,
    ) -> Result<SaveOutcome, RepositoryError>;

    /// The answer of a request that has been accepted, with what the
    /// command reports of its item as the repository at `root` holds it.
    fn answer(&self, code: ResultCode, effects: Effects, root: &Path) -> Answer;

    /// Sets what `prepare` worked out for the domain call, for a request
    /// that is entered again, and makes none of its checks.
    fn resume(&mut self, service: &RepositoryService, accepted: &Accepted) -> Result<(), Answer>;
}

/// What Git shows of the save's path since the request was accepted.
fn path_evidence<S: Save + ?Sized>(
    save: &S,
    accepted: &Accepted,
) -> Result<PathEvidence, Unreadable> {
    evidence::path_evidence(
        &accepted.root(),
        &accepted.position,
        &save.path(),
        &accepted.expectation(),
        &|bytes| save.holds_bytes(bytes),
    )
}

/// The already-applied rule, for the save's file in its editing context:
/// whether it is what the request intends and is committed at its branch
/// tip.
fn already_applied<S: Save + ?Sized>(
    save: &S,
    root: &Path,
    file: Option<&[u8]>,
) -> Result<bool, Unreadable> {
    match file {
        Some(bytes) if save.holds_bytes(bytes) => {
            observe::is_committed(&save.worktree(root), &save.path(), bytes).map_err(|_| Unreadable)
        }
        Some(_) | None => Ok(false),
    }
}

/// The first call of an accepted save: the domain call, and what the
/// repository shows of it afterwards.
pub(crate) fn run<S: Save + ?Sized>(
    save: &S,
    service: &RepositoryService,
    accepted: &Accepted,
) -> Ran {
    let path = save.path();
    let root = accepted.root();
    let expected = accepted.expectation();
    let result = save.call_domain(service, accepted);
    let file = save.context_file(&root);
    let (code, effects, standing) = match result {
        Ok(outcome) => {
            let report = SaveReport::of(outcome);
            // The commit and the effects come from what Git shows, made
            // after the domain call: a save that changed nothing can
            // name a commit it did not make.
            match confirmed(&root, &accepted.position, &path, report.claimed()) {
                Ok(commit) => {
                    let (code, effects) = report.result(commit);
                    let standing = standing(report, || accepted.journal_row(service));
                    (code, effects, standing)
                }
                // The domain names a commit and Git cannot be read to
                // say whose it is. Calling that a no-op would be stored
                // and replayed for good, so no result is claimed: the
                // answer is an internal failure with no effect and no
                // commit, and the record stays accepted for a retry.
                Err(Unreadable) => (
                    ResultCode::InternalError,
                    Effects::not_requested(),
                    Standing::Unconfirmed,
                ),
            }
        }
        Err(error) => {
            // Only the kind is read: the error's text can hold a path
            // or a backend's message.
            let code = outcome::repository_error_code(error.kind);
            let journal = accepted.journal_row(service);
            let holds = |bytes: &[u8]| save.holds_bytes(bytes);
            let effects = stopped_effects(
                journal.as_ref(),
                // A range that cannot be read shows no commit. The
                // record stays accepted either way: the row is in
                // flight.
                || {
                    evidence::intended_commit(&root, &accepted.position, &path, &holds)
                        .ok()
                        .flatten()
                },
                file.as_deref(),
                &expected,
                &holds,
            );
            (code, effects, Standing::Stopped { journal })
        }
    };
    Ran {
        answer: save.answer(code, effects, &root),
        standing,
    }
}

/// Re-entry of a save, before the domain call: sets up the call, reads
/// the evidence and decides from it whether the domain is called at all.
pub(crate) fn reenter<S: Save + ?Sized>(
    save: &mut S,
    service: &RepositoryService,
    accepted: &Accepted,
    journal: JournalRow,
) -> Result<Before, Ran> {
    let root = accepted.root();
    if let Err(answer) = save.resume(service, accepted) {
        return Err(Ran {
            answer,
            standing: Standing::NotRun { journal },
        });
    }
    let evidence = match path_evidence(save, accepted) {
        Ok(evidence) => evidence,
        // Git could not be read, so nothing is known of what an
        // earlier attempt committed: nothing is run, finished or
        // deleted on that.
        Err(Unreadable) => {
            return Err(Ran {
                answer: Answer::stopped(ResultCode::InternalError),
                standing: Standing::Unconfirmed,
            });
        }
    };
    let before = Before { journal, evidence };
    match before.found() {
        Found::Continue => Ok(before),
        Found::Done { commit } => {
            let (code, effects) = settled(Some(commit));
            Err(Ran {
                answer: save.answer(code, effects, &root),
                standing: Standing::Final,
            })
        }
        Found::Foreign { own } => Err(Ran {
            answer: save.answer(
                ResultCode::ExternalChange,
                outcome::stopped_save_effects(own, own.is_some()),
                &root,
            ),
            standing: Standing::Stopped {
                journal: Some(before.journal),
            },
        }),
    }
}

/// Re-entry of a save, the domain call. The commit is taken from the
/// evidence read before and after the call, never from what the domain
/// names: a second call of a save that committed names none.
pub(crate) fn rerun<S: Save + ?Sized>(
    save: &S,
    service: &RepositoryService,
    accepted: &Accepted,
    before: &Before,
) -> Ran {
    let root = accepted.root();
    let result = save.call_domain(service, accepted);
    // What Git shows now.
    let after = path_evidence(save, accepted);
    let file = save.context_file(&root);
    let (code, effects, standing) = match result {
        Ok(outcome) => match (SaveReport::of(outcome), after) {
            (SaveReport::IdentityRequired, _) => (
                ResultCode::IdentityRequired,
                Effects::not_requested(),
                Standing::Stopped {
                    journal: accepted.journal_row(service),
                },
            ),
            (report @ SaveReport::Saved { .. }, Ok(after)) => {
                let (code, effects) = report.result(before.reported(&after));
                // The journal is not read here: `standing` asks for
                // it only for a save that needs an identity, which is
                // matched above.
                let standing = standing(report, || accepted.journal_row(service));
                (code, effects, standing)
            }
            // The save returned and Git cannot be read to say whether
            // the request committed. A no-op stored now would be
            // replayed for good.
            (SaveReport::Saved { .. }, Err(Unreadable)) => (
                ResultCode::InternalError,
                Effects::not_requested(),
                Standing::Unconfirmed,
            ),
        },
        Err(error) => {
            // No attempt of this request had started, and the domain
            // found the file other than the request expected: the
            // content came from elsewhere. Asking for the state that
            // already exists is not a conflict.
            let applied = if error.kind == RepositoryErrorKind::ExternalChange && !before.started()
            {
                Some(already_applied(save, &root, file.as_deref()))
            } else {
                None
            };
            match applied {
                Some(Ok(true)) => {
                    let (code, effects) = settled(None);
                    (code, effects, Standing::Final)
                }
                Some(Err(Unreadable)) => (
                    ResultCode::InternalError,
                    Effects::not_requested(),
                    Standing::Unconfirmed,
                ),
                Some(Ok(false)) | None => {
                    let code = outcome::repository_error_code(error.kind);
                    let journal = accepted.journal_row(service);
                    let completed = matches!(
                        journal,
                        Some(JournalRow::Final {
                            kind: FinalKind::Completed,
                            owes_work: false,
                            checkpointed: _,
                        })
                    );
                    let commit = after.as_ref().ok().and_then(|after| before.reported(after));
                    match (completed, commit, after) {
                        // This call was refused, and the request's work
                        // is done all the same: its operation has
                        // completed and its commit is there, made by an
                        // earlier attempt or by a call of the same
                        // request that ran beside this one. What is
                        // true of the request is its result.
                        (true, Some(commit), _) => {
                            let (code, effects) = settled(Some(commit));
                            (code, effects, Standing::Final)
                        }
                        // The operation completed and Git cannot be
                        // read to say whether the request committed.
                        // Deleting the record now could lose a commit's
                        // report: it is left for a retry to settle.
                        (true, None, Err(Unreadable)) => {
                            (code, Effects::not_requested(), Standing::Unconfirmed)
                        }
                        (true, None, Ok(_)) | (false, _, _) => {
                            // A range that cannot be read shows no
                            // commit. The record stays accepted either
                            // way: the row is in flight.
                            let effects = stopped_effects(
                                journal.as_ref(),
                                || commit,
                                file.as_deref(),
                                &accepted.expectation(),
                                &|bytes| save.holds_bytes(bytes),
                            );
                            (code, effects, Standing::Stopped { journal })
                        }
                    }
                }
            }
        }
    };
    Ran {
        answer: save.answer(code, effects, &root),
        standing,
    }
}

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
