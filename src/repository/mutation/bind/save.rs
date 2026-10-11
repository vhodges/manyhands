//! What any binding of an authoring save derives from the repository: the
//! effects a result reports, the commit it names and what the call leaves.
//!
//! A save writes one canonical file in an item's editing context and
//! commits it there. Nothing here knows which item or which command: a
//! binding gives its file's path, the file's bytes as its context holds
//! them now, and how to tell that bytes are what the request intends.

use std::path::{Path, PathBuf};

use git2::Oid;

use crate::{
    canonical::ItemId,
    results::{Effects, ResultCode},
};

use super::{
    super::{
        super::{
            AuthoringKind, ExpectedPathObservation, ItemDto, ItemDtoKind, RepositoryError,
            RepositoryErrorKind, RepositoryOperation, RepositoryService, ResolvedRepository,
            SaveOutcome, owned_file_bytes,
            recovery::{FinalKind, JournalRow},
        },
        context::context_names,
        dto::MutationDataDto,
        evidence::{self, Claim, PathEvidence, Position, Unreadable},
        observe::{self, ObservedFile},
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

    /// What the request expected at `path` when it was accepted. For a
    /// save of a file where it is, the recorded expectation.
    fn expected(&self, accepted: &Accepted) -> ExpectedPathObservation {
        accepted.expectation()
    }

    /// What Git shows of another path the request changes, since the
    /// request was accepted: for a move, the path it moves its document
    /// from. A commit from elsewhere to that path stops the request as one
    /// to `path` does. `None` for a save of one file.
    fn other_path(&self, _accepted: &Accepted) -> Result<Option<PathEvidence>, Unreadable> {
        Ok(None)
    }

    /// Sets what `prepare` worked out for the domain call, for a request
    /// that is entered again, and makes none of its checks.
    fn resume(&mut self, service: &RepositoryService, accepted: &Accepted) -> Result<(), Answer>;
}

/// The item a request writes, as `show_item` reads its effective copy:
/// the item, its file's bytes and token, and what `parse` makes of the
/// file. An item of another kind, one with no file, a file `parse` does
/// not take, and one whose ID, by `id_of`, is another's are
/// `item_not_found`.
pub(crate) fn read_item<T>(
    service: &RepositoryService,
    repository: &ResolvedRepository,
    id: &ItemId,
    kind: ItemDtoKind,
    parse: impl FnOnce(&str, &[u8]) -> Option<T>,
    id_of: impl FnOnce(&T) -> &ItemId,
) -> Result<(ItemDto, ObservedFile, T), Answer> {
    let not_found = || Answer::stopped(ResultCode::ItemNotFound);
    let mut item = service
        .show_item(repository, id)
        .map_err(|error| Answer::of_read(&error))?;
    let (true, Some(source), Some(observation)) = (
        item.kind == kind,
        item.source.take(),
        item.observation.take(),
    ) else {
        return Err(not_found());
    };
    let current = parse(&item.path, source.as_bytes()).ok_or_else(not_found)?;
    if id_of(&current) != id {
        return Err(not_found());
    }
    let observed = ObservedFile {
        token: observation,
        bytes: source.into_bytes(),
    };
    Ok((item, observed, current))
}

/// The already-applied rule, for a request whose token is not the file's
/// or whose end state may already hold: `bytes` are the file at `path`
/// in the worktree `worktree`, and `as_intended` says they are what the
/// request intends. A no-op when they are and the file is committed at
/// its branch tip; an external change otherwise.
pub(crate) fn stale_token_answer(
    worktree: &Path,
    path: &str,
    bytes: &[u8],
    as_intended: bool,
    data: MutationDataDto,
) -> Answer {
    // A worktree that cannot be read is not known to be committed.
    let committed = observe::is_committed(worktree, path, bytes).unwrap_or(false);
    let code = observe::stale_token_code(as_intended, committed);
    let mut answer = Answer::stopped(code).with_data(data);
    if code == ResultCode::AlreadyApplied {
        (_, answer.effects) = settled(None);
    }
    answer
}

/// The primary-is-committed rule. With no editing context the item's
/// effective copy is primary's file, and the domain creates the context
/// from primary's head: the same bytes only if the file is committed and
/// unmodified. Otherwise the answer is `worktree_not_clean`.
pub(crate) fn primary_is_committed(
    root: &Path,
    kind: &AuthoringKind,
    id: &str,
    path: &str,
    bytes: &[u8],
) -> Result<(), Answer> {
    if !context_names(root, kind, id).worktree.is_dir()
        && !observe::is_committed(root, path, bytes).unwrap_or(false)
    {
        return Err(Answer::stopped(ResultCode::WorktreeNotClean));
    }
    Ok(())
}

/// What Git shows of the save's path since the request was accepted.
///
/// For a request that empties another path in the same commit, a commit
/// that still holds a file there is not its own.
fn path_evidence<S: Save + ?Sized>(
    save: &S,
    accepted: &Accepted,
) -> Result<PathEvidence, Unreadable> {
    let evidence = evidence::path_evidence(
        &accepted.root(),
        &accepted.position,
        &save.path(),
        &save.expected(accepted),
        &|bytes| save.holds_bytes(bytes),
    )?;
    Ok(match save.other_path(accepted)? {
        Some(other) => evidence.together_with(&other),
        None => evidence,
    })
}

/// The file a request that is entered again was accepted against: the
/// bytes its recorded expectation is the digest of. They are looked for
/// at `path` in the item's editing context, in the primary worktree,
/// which is the effective copy until a context exists, and in the commit
/// the request was accepted at.
///
/// When none of those is it, the file had been edited and not committed
/// when the request was accepted, and an earlier attempt, or someone
/// else, has written it since. Then `otherwise`, the item's file as it is
/// now, is all there is to go by.
///
/// A command whose input does not say what the item's fields are to be,
/// a short code assignment or a move, writes back the fields of this
/// file. Taking them from the file as it is now instead would have the
/// domain find the file as intended whatever someone else had done to it
/// since, and commit their edit as the request's own.
pub(crate) fn found_file(
    accepted: &Accepted,
    worktree: &Path,
    path: &str,
    otherwise: Option<Vec<u8>>,
) -> Option<Vec<u8>> {
    let root = accepted.root();
    let expected = accepted.expectation();
    let read = |directory: &Path| {
        owned_file_bytes(directory, Path::new(path), RepositoryOperation::Read, &root)
            .ok()
            .flatten()
    };
    let committed = accepted
        .position
        .base_oid
        .and_then(|base| evidence::file_at(&root, base, path).ok().flatten());
    [read(worktree), read(&root), committed]
        .into_iter()
        .flatten()
        .find(|bytes| ExpectedPathObservation::from_bytes(bytes) == expected)
        .or(otherwise)
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
    let expected = save.expected(accepted);
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
    let foreign = |own: Option<Oid>, before: Before| {
        Err(Ran {
            answer: save.answer(
                ResultCode::ExternalChange,
                outcome::stopped_save_effects(own, own.is_some()),
                &root,
            ),
            standing: Standing::Stopped {
                journal: Some(before.journal),
            },
        })
    };
    match before.found() {
        Found::Continue => match save.other_path(accepted) {
            Ok(Some(other)) if other.superseded(before.committing()) => {
                foreign(before.own(), before)
            }
            Ok(Some(_) | None) => Ok(before),
            Err(Unreadable) => Err(Ran {
                answer: Answer::stopped(ResultCode::InternalError),
                standing: Standing::Unconfirmed,
            }),
        },
        Found::Done { commit } => {
            let (code, effects) = settled(Some(commit));
            Err(Ran {
                answer: save.answer(code, effects, &root),
                standing: Standing::Final,
            })
        }
        Found::Foreign { own } => foreign(own, before),
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
            let code = outcome::repository_error_code(error.kind);
            let journal = accepted.journal_row(service);
            let (completed, checkpointed) = match journal {
                Some(JournalRow::Final {
                    kind: FinalKind::Completed,
                    owes_work: false,
                    checkpointed,
                }) => (true, checkpointed),
                Some(
                    JournalRow::Absent
                    | JournalRow::Pending(_)
                    | JournalRow::Final {
                        kind: _,
                        owes_work: _,
                        checkpointed: _,
                    },
                )
                | None => (false, false),
            };
            let commit = after.as_ref().ok().and_then(|after| before.reported(after));
            match refused(completed, checkpointed, commit, after.is_ok(), applied) {
                Refused::Done(commit) => {
                    let (code, effects) = settled(Some(commit));
                    (code, effects, Standing::Final)
                }
                Refused::AlreadyApplied => {
                    let (code, effects) = settled(None);
                    (code, effects, Standing::Final)
                }
                Refused::Unreadable => (
                    ResultCode::InternalError,
                    Effects::not_requested(),
                    Standing::Unconfirmed,
                ),
                Refused::CompletedUnread => (code, Effects::not_requested(), Standing::Unconfirmed),
                Refused::Stopped => {
                    // A range that cannot be read shows no commit. The
                    // record stays accepted either way: the row is in
                    // flight.
                    let effects = stopped_effects(
                        journal.as_ref(),
                        || commit,
                        file.as_deref(),
                        &save.expected(accepted),
                        &|bytes| save.holds_bytes(bytes),
                    );
                    (code, effects, Standing::Stopped { journal })
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

/// What a re-run the domain refused comes to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Refused {
    /// The request's work is done all the same, with this commit.
    Done(Oid),
    /// The request's end state already holds and no commit is its own.
    AlreadyApplied,
    /// Git could not be read to say whether the end state holds.
    Unreadable,
    /// The operation completed and Git cannot be read to say whether the
    /// request committed. Deleting the record now could lose a commit's
    /// report: it is left for a retry to settle.
    CompletedUnread,
    /// The request stopped: what it left is the journal row's to say.
    Stopped,
}

/// Decides what a refused re-run comes to. `completed` says the
/// operation's row is final, completed and owes no work, and
/// `checkpointed` that it passed its checkpoint; `commit` is the commit
/// the evidence reports as the request's, and `evidence_read` says the
/// evidence after the call could be read. `applied` is the
/// already-applied rule's answer, where it was asked: only of a request
/// no attempt of which had started, refused as an external change.
fn refused(
    completed: bool,
    checkpointed: bool,
    commit: Option<Oid>,
    evidence_read: bool,
    applied: Option<Result<bool, Unreadable>>,
) -> Refused {
    // This call was refused, and the request committed all the same: its
    // operation has completed, having passed its checkpoint, and its
    // commit is there. This is asked first. A call that ran beside this
    // one can have committed between this call's reading and its domain
    // call; the file is then as intended and committed, and the
    // already-applied rule, asked first, would finish the record as a
    // no-op with no commit, for a request that made one.
    if let (true, true, Some(commit)) = (completed, checkpointed, commit) {
        return Refused::Done(commit);
    }
    match applied {
        Some(Ok(true)) => return Refused::AlreadyApplied,
        Some(Err(Unreadable)) => return Refused::Unreadable,
        Some(Ok(false)) | None => {}
    }
    match (completed, commit, evidence_read) {
        // Its operation has completed and its commit is there, made by an
        // earlier attempt or by a call of the same request that ran
        // beside this one. What is true of the request is its result.
        (true, Some(commit), _) => Refused::Done(commit),
        (true, None, false) => Refused::CompletedUnread,
        (true, None, true) | (false, _, _) => Refused::Stopped,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commit() -> Oid {
        Oid::from_bytes(&[7; 20]).unwrap()
    }

    #[test]
    fn a_refused_re_run_of_a_request_that_committed_is_not_a_no_op() {
        // The file is as intended and committed, so the already-applied
        // rule would say the end state holds: it does, because a call of
        // this request that ran beside this one committed it.
        assert_eq!(
            refused(true, true, Some(commit()), true, Some(Ok(true))),
            Refused::Done(commit())
        );
        assert_eq!(
            refused(true, true, Some(commit()), true, Some(Err(Unreadable))),
            Refused::Done(commit())
        );
    }

    #[test]
    fn a_refused_re_run_is_already_applied_only_with_no_commit_of_its_own() {
        // Content from elsewhere: no commit is the request's.
        assert_eq!(
            refused(false, false, None, true, Some(Ok(true))),
            Refused::AlreadyApplied
        );
        assert_eq!(
            refused(true, false, None, true, Some(Ok(true))),
            Refused::AlreadyApplied
        );
        // A row that never passed its checkpoint made no commit, whatever
        // the evidence shows: the rule is asked as before.
        assert_eq!(
            refused(true, false, Some(commit()), true, Some(Ok(true))),
            Refused::AlreadyApplied
        );
        assert_eq!(
            refused(false, false, None, true, Some(Err(Unreadable))),
            Refused::Unreadable
        );
    }

    #[test]
    fn a_refused_re_run_the_rule_does_not_answer_follows_its_row_and_its_evidence() {
        for applied in [None, Some(Ok(false))] {
            assert_eq!(
                refused(true, true, Some(commit()), true, applied),
                Refused::Done(commit())
            );
            assert_eq!(
                refused(true, false, Some(commit()), true, applied),
                Refused::Done(commit())
            );
            assert_eq!(
                refused(true, true, None, false, applied),
                Refused::CompletedUnread
            );
            assert_eq!(refused(true, true, None, true, applied), Refused::Stopped);
            assert_eq!(
                refused(false, false, Some(commit()), true, applied),
                Refused::Stopped
            );
            assert_eq!(
                refused(false, false, None, false, applied),
                Refused::Stopped
            );
        }
    }
}
