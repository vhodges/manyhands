//! The pull-request promote through the runner (Wave 06 Cycle 04, Task 3):
//! the confirm of a document's promote in a pull-request repository, which
//! synchronizes and publishes the document's branch through its context
//! child and answers by the design's two answer tables. Every test asserts
//! that the local primary did not move, that no synchronization targeted
//! the primary, and that no promotion merge exists: the fake publisher
//! below refuses a primary target.
use super::*;
use crate::{
    canonical::Integration,
    repository::{
        promotion::lifecycle::{DocumentPromotionLifecycle, LifecycleRequest},
        remote::{observation::endpoint_digest_of, sync},
    },
};
use record::ChildRole;
use time::OffsetDateTime;

const PULL_REQUEST_CONFIG: &str = "format_version = 2\nintegration = \"pull_request\"\nprimary_branch = \"main\"\npublication_remote = \"origin\"\n";
const CLOCK: i64 = 1_795_000_000;
fn clock() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(CLOCK).unwrap()
}
/// A later clock than `CLOCK`: a second promote, a day on.
fn later() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(CLOCK + 86_400).unwrap()
}

fn context_ref() -> String {
    format!("refs/heads/manyhands/document/{ITEM}")
}
fn tracking_context_ref() -> String {
    format!("refs/remotes/origin/manyhands/document/{ITEM}")
}

/// A link row as stored: source, target, forge, URL and `given_at`.
type LinkRow = (String, String, Option<String>, Option<String>, i64);

impl Fixture {
    /// A published document in a pull-request repository: the remote and its
    /// tracking refs as `remote()` leaves them, and the pull-request
    /// configuration written, uncommitted, in the primary's worktree, so that
    /// the local primary never moves.
    fn pull_request() -> Self {
        let f = Fixture::new();
        f.remote();
        f.set_config(PULL_REQUEST_CONFIG);
        f
    }
    fn set_config(&self, source: &str) {
        write(&self.repo, ".manyhands/config.toml", source);
    }
    /// A new document: the remote holds no copy of its branch.
    fn unpublish(&self) {
        self.repo
            .find_reference(&tracking_context_ref())
            .unwrap()
            .delete()
            .unwrap();
    }
    fn tip(&self) -> Oid {
        self.repo.refname_to_id(&context_ref()).unwrap()
    }
    fn tracking_main(&self) -> Oid {
        self.repo.refname_to_id("refs/remotes/origin/main").unwrap()
    }
    fn set_tracking_main(&self, oid: Oid) {
        self.repo
            .reference("refs/remotes/origin/main", oid, true, "fixture fetch")
            .unwrap();
    }
    /// A save: one more line in the document, committed on the context.
    fn edit(&self, line: &str) -> Oid {
        let path = self.context.workdir().unwrap().join(DOC);
        let mut text = fs::read_to_string(&path).unwrap();
        text.push_str(line);
        text.push('\n');
        fs::write(path, text).unwrap();
        commit(&self.context)
    }
    fn blob_at(&self, commit: Oid, path: &str) -> Vec<u8> {
        let tree = self.repo.find_commit(commit).unwrap().tree().unwrap();
        let id = tree.get_path(std::path::Path::new(path)).unwrap().id();
        self.repo.find_blob(id).unwrap().content().to_vec()
    }
    /// A commit, moving no ref: `base`'s tree with `edits` written over it,
    /// on `parents`.
    fn commit_tree(&self, base: Oid, edits: &[(&str, &[u8])], parents: &[Oid]) -> Oid {
        let mut index = git2::Index::new().unwrap();
        index
            .read_tree(&self.repo.find_commit(base).unwrap().tree().unwrap())
            .unwrap();
        for (path, bytes) in edits {
            index
                .add(&git2::IndexEntry {
                    ctime: git2::IndexTime::new(0, 0),
                    mtime: git2::IndexTime::new(0, 0),
                    dev: 0,
                    ino: 0,
                    mode: 0o100644,
                    uid: 0,
                    gid: 0,
                    file_size: bytes.len() as u32,
                    id: self.repo.blob(bytes).unwrap(),
                    flags: 0,
                    flags_extended: 0,
                    path: path.as_bytes().to_vec(),
                })
                .unwrap();
        }
        let tree = self
            .repo
            .find_tree(index.write_tree_to(&self.repo).unwrap())
            .unwrap();
        let sig = git2::Signature::now("Forge", "forge@example.invalid").unwrap();
        let parents = parents
            .iter()
            .map(|oid| self.repo.find_commit(*oid).unwrap())
            .collect::<Vec<_>>();
        self.repo
            .commit(
                None,
                &sig,
                &sig,
                "forge",
                &tree,
                &parents.iter().collect::<Vec<_>>(),
            )
            .unwrap()
    }
    /// The forge's squash of the document's pull request: one commit on the
    /// tracking primary holding the tip's document. No ref moves.
    fn squash(&self, tip: Oid) -> Oid {
        let main = self.tracking_main();
        self.commit_tree(main, &[(DOC, &self.blob_at(tip, DOC))], &[main])
    }
    /// The forge's merge commit of the pull request. No ref moves.
    fn merge_commit(&self, tip: Oid) -> Oid {
        let main = self.tracking_main();
        self.commit_tree(main, &[(DOC, &self.blob_at(tip, DOC))], &[main, tip])
    }
    /// A later commit on the tracking primary that does not touch the
    /// document: the remote's primary moved on. No ref moves.
    fn primary_moves_on(&self) -> Oid {
        let main = self.tracking_main();
        self.commit_tree(main, &[("elsewhere.txt", b"elsewhere\n")], &[main])
    }
    fn link_row(&self) -> Option<LinkRow> {
        self.db()
            .query_row(
                "SELECT source,target,forge,url,given_at FROM pull_request_links WHERE kind='document' AND item_ulid=?1",
                [ITEM],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .ok()
    }
    fn link_rows(&self) -> i64 {
        self.db()
            .query_row("SELECT COUNT(*) FROM pull_request_links", [], |row| {
                row.get(0)
            })
            .unwrap()
    }
    fn promotion_rows(&self) -> i64 {
        self.db()
            .query_row("SELECT COUNT(*) FROM document_promotions", [], |row| {
                row.get(0)
            })
            .unwrap()
    }
    fn child_phase(&self, child: OperationId) -> Option<String> {
        self.db()
            .query_row(
                "SELECT phase FROM remote_operation_records WHERE operation_ulid=?1",
                [child.to_string()],
                |row| row.get(0),
            )
            .ok()
    }
}

/// The children of a pull-request promote, scripted. It publishes the
/// branch's tip as a child does, and never touches the primary: a primary
/// child is a failure the test sees in `primary_calls`, and a delete in
/// `deletes`.
#[derive(Default)]
struct PullRequestIo {
    children: Vec<(SynchronizationTarget, OperationId, bool)>,
    /// `refs/remotes/origin/main` as the child's fetch leaves it, written
    /// first, before any stop or publication.
    fetched: Option<Oid>,
    /// A scripted stop; `kept` reserves the child and leaves its row active.
    stop: Option<(SynchronizationError, bool)>,
    /// One real local update (the stage that merges the fetched primary
    /// into the branch), with the incoming copy of the branch.
    drive: Option<Oid>,
    primary_calls: usize,
    deletes: usize,
}
impl run::PromotionIo for PullRequestIo {
    fn synchronize(
        &mut self,
        service: &RepositoryService,
        req: SynchronizeRemoteRequest,
        _: &mut dyn SynchronizationObserver,
        execution: &record::ChildExecution,
    ) -> Result<SynchronizationResult, SynchronizationError> {
        self.children
            .push((req.target.clone(), req.operation_id, req.restart));
        if req.target == SynchronizationTarget::Primary {
            self.primary_calls += 1;
            return Err(SynchronizationError::RecoveryRequired);
        }
        let root = req.root.clone();
        // The acknowledgement of a cancelled row ends it, as a real child
        // does at its first safe point.
        let cancelling = state::with_transaction(service, &root, |tx, id| {
            Ok(state::read_operation(tx, id, req.operation_id)?
                .is_some_and(|row| row.cancel_requested))
        })
        .unwrap();
        if req.restart && cancelling {
            state::with_transaction(service, &root, |tx, _| {
                tx.execute(
                    "UPDATE remote_operation_records SET phase='cancelled',outcome='cancelled',cancel_requested=0 WHERE operation_ulid=?1",
                    [req.operation_id.to_string()],
                )
                .map_err(|_| state::recovery_required())?;
                Ok(())
            })
            .unwrap();
            return Err(SynchronizationError::Interrupted);
        }
        let repo = git2::Repository::open(&root).unwrap();
        if let Some(oid) = self.fetched.take() {
            repo.reference("refs/remotes/origin/main", oid, true, "fixture fetch")
                .unwrap();
        }
        let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
        let target = req.target.operation_target(&plan);
        if let Some((error, kept)) = self.stop.take() {
            if kept {
                let new = state::with_transaction(service, &root, |tx, id| {
                    Ok(state::read_operation(tx, id, req.operation_id)?.is_none())
                })
                .unwrap();
                assert!(matches!(
                    service
                        .reserve_promotion_child(&root, req.operation_id, &target, execution, new)
                        .unwrap(),
                    RemoteReservationOutcome::Reserved(_)
                ));
            }
            return Err(error);
        }
        let (oid, kind) = if let Some(incoming) = self.drive.take() {
            let merged =
                sync::promotion_test_drive_local_bound(service, &req, incoming, execution)?;
            (merged, "published")
        } else {
            let new = state::with_transaction(service, &root, |tx, id| {
                Ok(state::read_operation(tx, id, req.operation_id)?.is_none())
            })
            .unwrap();
            let reservation = service.reserve_promotion_child(
                &root,
                req.operation_id,
                &target,
                execution,
                new,
            )?;
            if matches!(reservation, RemoteReservationOutcome::Replay(_)) {
                service
                    .restart_promotion_child(&root, req.operation_id, &target, execution)
                    .unwrap();
            } else {
                assert!(matches!(reservation, RemoteReservationOutcome::Reserved(_)));
            }
            let tip = repo.refname_to_id(&context_ref()).unwrap();
            let current = repo.refname_to_id(&tracking_context_ref()).ok() == Some(tip);
            (
                tip,
                if current {
                    "already_current"
                } else {
                    "published"
                },
            )
        };
        complete_child_row(service, &root, execution, req.operation_id, oid, kind);
        repo.reference(&tracking_context_ref(), oid, true, "fixture publication")
            .unwrap();
        Ok(SynchronizationResult::Complete(if kind == "published" {
            SynchronizationOutcome::Published {
                target: req.target,
                oid,
            }
        } else {
            SynchronizationOutcome::AlreadyCurrent {
                target: req.target,
                oid,
            }
        }))
    }
    fn delete(
        &mut self,
        _: &RepositoryService,
        _: VerifySshTransportRequest,
        _: &RemoteRefPlan,
        _: &RemoteReservation,
        _: &ObservationConfiguration,
        _: Oid,
        _: &ItemId,
    ) -> Result<DeleteObservation, SynchronizationError> {
        self.deletes += 1;
        Err(SynchronizationError::RecoveryRequired)
    }
}

fn request_at(
    request: PromoteDocumentRequest,
    at: fn() -> OffsetDateTime,
) -> LifecycleRequest<PromotionExpectation> {
    LifecycleRequest {
        clock: at,
        ..run::lifecycle_request(request)
    }
}
fn run_at(
    f: &Fixture,
    request: PromoteDocumentRequest,
    io: &mut PullRequestIo,
    at: fn() -> OffsetDateTime,
) -> Result<DocumentPromotionResult, DocumentPromotionError> {
    run::run_with_io(
        &DocumentPromotionLifecycle,
        &f.service,
        request_at(request, at),
        io,
    )
}
/// The answer of a branch record, whichever way the index step went.
fn branch(
    result: Result<DocumentPromotionResult, DocumentPromotionError>,
) -> BranchPromotionOutcome {
    match result {
        Ok(DocumentPromotionResult::Branch(outcome)) => outcome,
        Ok(DocumentPromotionResult::BranchIndexPending(pending)) => pending.authoritative,
        other => panic!("not a branch answer: {other:?}"),
    }
}
/// What no pull-request promote does: move the local primary, synchronize
/// the primary, delete a branch, or merge anything of its own.
fn assert_primary_untouched(f: &Fixture, io: &PullRequestIo, primary: Oid, case: &str) {
    assert_eq!(f.primary(), primary, "{case}: the local primary moved");
    assert_eq!(io.primary_calls, 0, "{case}: the primary was synchronized");
    assert_eq!(io.deletes, 0, "{case}: a branch was deleted");
    assert_eq!(promotion_commits(&f.repo), 0, "{case}: a merge was made");
    // The second slot of the record is empty, and the audit accepts it.
    state::with_transaction(&f.service, f.root.path(), |tx, _| record::audit(tx)).unwrap();
}

/// C1 to C3: the first promote of a document gives its link: it publishes
/// the branch (or finds it published), answers `promoted`, stores the link of
/// the record's own remote and branch, and finishes with no integration.
#[test]
fn first_promote_publishes_and_gives_the_link() {
    type Setup = fn(&Fixture);
    let table: [(&str, Setup, bool); 3] = [
        ("a new document", |f| f.unpublish(), false),
        (
            "an edit of a merged document",
            |f| {
                let squash = f.squash(f.tip());
                f.set_tracking_main(squash);
                f.edit("an edit after the merge");
            },
            false,
        ),
        ("a branch already published by item sync", |_| {}, true),
    ];
    for (case, setup, current) in table {
        let f = Fixture::pull_request();
        setup(&f);
        let req = f.request();
        let (primary, tip, tracking_main) = (f.primary(), f.tip(), f.tracking_main());
        let mut io = PullRequestIo::default();
        let mut told = Told(Vec::new());
        let result = run::run_with_io_observed(
            &DocumentPromotionLifecycle,
            &f.service,
            request_at(req.clone(), clock),
            &mut io,
            &mut told,
        );
        let outcome = branch(result);
        assert_eq!(
            outcome.answer(),
            record::PromotionAnswer::Promoted,
            "{case}"
        );
        let (publication, category) = if current {
            (
                PromotionPublication::AlreadyCurrent { oid: tip },
                record::PublicationCategory::AlreadyCurrent,
            )
        } else {
            (
                PromotionPublication::Published { oid: tip },
                record::PublicationCategory::Published,
            )
        };
        assert_eq!(outcome.publication(), Some(publication), "{case}");
        // The record: a finished pull-request record, with what it began
        // with and nothing of an integration.
        let r = f.stored(req.operation_id);
        assert_eq!(r.step, record::PromotionStep::Complete, "{case}");
        assert_eq!(r.integration_mode, Integration::PullRequest, "{case}");
        assert_eq!(
            r.answer_code,
            Some(record::PromotionAnswer::Promoted),
            "{case}"
        );
        assert_eq!(r.publication, Some(category), "{case}");
        assert_eq!(
            r.situation,
            Some(record::PromotionSituation::First),
            "{case}"
        );
        assert_eq!(r.consented_context_oid, Some(tip), "{case}");
        assert_eq!(r.primary_branch.as_deref(), Some("main"), "{case}");
        assert_eq!(
            r.endpoint_digest,
            Some(endpoint_digest_of(&f.repo, "origin")),
            "{case}"
        );
        assert_eq!(r.context_oid, Some(tip), "{case}");
        assert_eq!(r.refusal, None, "{case}");
        assert_eq!(
            (r.base_oid, r.integration_oid, r.integration_kind, r.cleanup),
            (None, None, None, None),
            "{case}"
        );
        assert_eq!((r.primary_attempt, r.primary_sync_id), (0, None), "{case}");
        // The link: the document's own row, given at the clock's time.
        assert_eq!(
            f.link_row(),
            Some((
                format!("manyhands/document/{ITEM}"),
                "main".into(),
                None,
                None,
                CLOCK
            )),
            "{case}"
        );
        assert_eq!(f.link_rows(), 1, "{case}");
        // A promotion tells its observer of its child alone: it records
        // nothing and integrates nothing.
        assert_eq!(
            told.0,
            vec![run::RunEvent::ChildStarting(ChildRole::Context)],
            "{case}"
        );
        assert_eq!(f.tracking_main(), tracking_main, "{case}");
        assert_primary_untouched(&f, &io, primary, case);
        // The same request again gives the same answer, and runs nothing.
        let children = io.children.len();
        assert_eq!(branch(run_at(&f, req, &mut io, clock)), outcome, "{case}");
        assert_eq!(io.children.len(), children, "{case}");
        assert_eq!(f.link_rows(), 1, "{case}");
    }
}

/// The twin of an index step that failed: the record is finished, its answer
/// is the same, and the result says the index is owed.
#[test]
fn a_pull_request_promotion_whose_index_step_failed_is_branch_index_pending() {
    let f = Fixture::pull_request();
    let req = f.request();
    let failing = RepositoryService::open_at_with_failure_point_for_testing(
        f.data.path(),
        FailurePoint::RefreshFailure,
    )
    .unwrap();
    let mut io = PullRequestIo::default();
    let result = run::run_with_io(
        &DocumentPromotionLifecycle,
        &failing,
        request_at(req.clone(), clock),
        &mut io,
    );
    let Ok(DocumentPromotionResult::BranchIndexPending(pending)) = result else {
        panic!("not index pending: {result:?}");
    };
    assert_eq!(
        pending.authoritative.answer(),
        record::PromotionAnswer::Promoted
    );
    assert_eq!(
        f.stored(req.operation_id).step,
        record::PromotionStep::Complete
    );
    assert_eq!(f.link_rows(), 1);
    // The same request on a service that can index answers the same.
    let again = branch(run_at(&f, req, &mut io, clock));
    assert_eq!(again, pending.authoritative);
    assert_eq!(io.children.len(), 1);
    assert_primary_untouched(&f, &io, f.primary(), "index pending");
}

/// C4 to C7, A4: promoting again synchronizes the branch and gives the same
/// link: the row's, not a rebuilt one, with only its `given_at` renewed.
#[test]
fn promoting_again_synchronizes_and_gives_the_same_link() {
    type Change = fn(&Fixture, &mut PullRequestIo);
    let table: [(&str, Change, bool); 4] = [
        ("nothing new", |_, _| {}, true),
        (
            "after a save",
            |f, _| {
                f.edit("a save");
            },
            false,
        ),
        (
            "after the remote's primary moved",
            |f, io| {
                io.fetched = Some(f.primary_moves_on());
                io.drive = Some(f.tip());
            },
            false,
        ),
        (
            "after the forge setting changed",
            |f, _| {
                f.set_config(&format!(
                    "{PULL_REQUEST_CONFIG}[forge]\npull_request_url = \"https://forge.example/pr?source={{source}}&target={{target}}\"\n"
                ));
            },
            true,
        ),
    ];
    for (case, change, current) in table {
        let f = Fixture::pull_request();
        let primary = f.primary();
        let mut io = PullRequestIo::default();
        let first = f.request();
        branch(run_at(&f, first.clone(), &mut io, clock));
        let row = f.link_row().expect("the first promote gave its link");
        let old_tip = f.tip();
        change(&f, &mut io);
        let second = f.request();
        assert_ne!(first.operation_id, second.operation_id);
        let outcome = branch(run_at(&f, second.clone(), &mut io, later));
        let tip = f.tip();
        assert_eq!(
            outcome.answer(),
            record::PromotionAnswer::AlreadyPromoted,
            "{case}"
        );
        assert_eq!(
            outcome.publication(),
            Some(if current {
                PromotionPublication::AlreadyCurrent { oid: tip }
            } else {
                PromotionPublication::Published { oid: tip }
            }),
            "{case}"
        );
        let r = f.stored(second.operation_id);
        assert_eq!(r.step, record::PromotionStep::Complete, "{case}");
        assert_eq!(
            r.situation,
            Some(record::PromotionSituation::Again),
            "{case}"
        );
        assert_eq!(
            r.answer_code,
            Some(record::PromotionAnswer::AlreadyPromoted),
            "{case}"
        );
        assert_eq!(
            r.publication,
            Some(if current {
                record::PublicationCategory::AlreadyCurrent
            } else {
                record::PublicationCategory::Published
            }),
            "{case}"
        );
        // The same link, still the one row, renewed at the later clock.
        let renewed = f.link_row().unwrap();
        assert_eq!(
            (&renewed.0, &renewed.1, &renewed.2, &renewed.3),
            (&row.0, &row.1, &row.2, &row.3),
            "{case}"
        );
        assert_eq!(renewed.4, CLOCK + 86_400, "{case}");
        assert_eq!(f.link_rows(), 1, "{case}");
        // The first record is as it was.
        assert_eq!(
            f.stored(first.operation_id).answer_code,
            Some(record::PromotionAnswer::Promoted),
            "{case}"
        );
        if case == "after a save" {
            assert_ne!(tip, old_tip);
        }
        if case == "after the remote's primary moved" {
            let merge = f.repo.find_commit(tip).unwrap();
            assert_eq!(
                merge.summary(),
                Some(format!("Merge primary into document {ITEM}").as_str())
            );
            assert_eq!(merge.parent_id(0).unwrap(), old_tip);
            assert_eq!(merge.parent_count(), 2);
        }
        assert_primary_untouched(&f, &io, primary, case);
    }
}

/// C8 to C10: a document the primary holds gets no link: at prepare there is
/// nothing to confirm; a squash the child's own fetch brings, with the branch
/// kept, is merged in and published, and the record finishes; a merge the
/// fetch brings with the branch deleted ends the record: both answer
/// `primary_holds` and leave the link row as it was.
#[test]
fn a_held_document_gives_no_link() {
    // C8: nothing to confirm, and nothing written.
    {
        let f = Fixture::pull_request();
        let primary = f.primary();
        f.set_tracking_main(f.squash(f.tip()));
        let held = f
            .service
            .preview_document_promotion(PreviewDocumentPromotionRequest {
                root: f.root.path().into(),
                item_id: ITEM.parse().unwrap(),
            })
            .unwrap();
        assert!(matches!(held, DocumentPromotionPreflight::PrimaryHolds));
        assert_eq!(f.promotion_rows(), 0);
        assert_eq!(f.link_rows(), 0);
        assert_eq!(f.primary(), primary);
    }
    // C9 and C10, over a first promote and a promote again.
    for squash in [true, false] {
        for again in [false, true] {
            let case = format!(
                "{} promote {}",
                if again { "again" } else { "first" },
                if squash {
                    "squash"
                } else {
                    "merge, branch deleted"
                }
            );
            let f = Fixture::pull_request();
            let primary = f.primary();
            let mut io = PullRequestIo::default();
            if again {
                branch(run_at(&f, f.request(), &mut io, clock));
            }
            let row = f.link_row();
            let tip = f.tip();
            let req = f.request();
            if squash {
                // The child's fetch brings the squash, and its local update
                // merges the primary into the branch, which it publishes.
                io.fetched = Some(f.squash(tip));
                io.drive = Some(tip);
            } else {
                // The child's fetch brings the merge, and the branch is gone.
                io.fetched = Some(f.merge_commit(tip));
                io.stop = Some((SynchronizationError::RemoteContextDeleted, true));
            }
            let outcome = branch(run_at(&f, req.clone(), &mut io, later));
            assert_eq!(
                outcome.answer(),
                record::PromotionAnswer::PrimaryHolds,
                "{case}"
            );
            let r = f.stored(req.operation_id);
            if squash {
                let merged = f.tip();
                assert_ne!(merged, tip, "{case}");
                assert_eq!(
                    outcome.publication(),
                    Some(PromotionPublication::Published { oid: merged }),
                    "{case}"
                );
                assert_eq!(r.step, record::PromotionStep::Complete, "{case}");
                assert_eq!(
                    r.publication,
                    Some(record::PublicationCategory::Published),
                    "{case}"
                );
                assert_eq!(r.refusal, None, "{case}");
            } else {
                assert_eq!(outcome.publication(), None, "{case}");
                assert_eq!(r.step, record::PromotionStep::Abandoned, "{case}");
                assert_eq!(
                    r.refusal,
                    Some(record::Refusal::ReconfirmationRequired),
                    "{case}"
                );
                assert_eq!(r.publication, None, "{case}");
                // The child's row ended, and nothing holds the context.
                let child = r.context_sync_id.expect("the child was minted");
                assert_eq!(f.child_phase(child).as_deref(), Some("cancelled"), "{case}");
                assert!(io.children.iter().all(|c| !c.2), "{case}");
                assert_eq!(f.tip(), tip, "{case}");
            }
            assert_eq!(
                r.answer_code,
                Some(record::PromotionAnswer::PrimaryHolds),
                "{case}"
            );
            assert_eq!(
                r.situation,
                Some(if again {
                    record::PromotionSituation::Again
                } else {
                    record::PromotionSituation::First
                }),
                "{case}"
            );
            // The link row is as it was: none for a first promote, and the
            // earlier one, not renewed, for a second.
            assert_eq!(f.link_row(), row, "{case}");
            assert_primary_untouched(&f, &io, primary, &case);
        }
    }
}

/// C11: a branch deleted without a merge (a declined pull request) answers
/// `remote_branch_deleted`, ends the record and deletes the link row; after a
/// republish the next promote is a first one and answers `promoted`.
#[test]
fn a_declined_pull_request_answers_remote_branch_deleted() {
    let f = Fixture::pull_request();
    let primary = f.primary();
    let mut io = PullRequestIo::default();
    branch(run_at(&f, f.request(), &mut io, clock));
    assert!(f.link_row().is_some());
    let req = f.request();
    io.stop = Some((SynchronizationError::RemoteContextDeleted, true));
    let outcome = branch(run_at(&f, req.clone(), &mut io, later));
    assert_eq!(
        outcome.answer(),
        record::PromotionAnswer::RemoteBranchDeleted
    );
    assert_eq!(outcome.publication(), None);
    let r = f.stored(req.operation_id);
    assert_eq!(r.step, record::PromotionStep::Abandoned);
    assert_eq!(r.refusal, Some(record::Refusal::ReconfirmationRequired));
    assert_eq!(
        r.answer_code,
        Some(record::PromotionAnswer::RemoteBranchDeleted)
    );
    assert_eq!(r.situation, Some(record::PromotionSituation::Again));
    assert_eq!(f.link_row(), None);
    let child = r.context_sync_id.expect("the child was minted");
    assert_eq!(f.child_phase(child).as_deref(), Some("cancelled"));
    // The same request answers the same, and runs nothing.
    let children = io.children.len();
    assert_eq!(branch(run_at(&f, req, &mut io, later)), outcome);
    assert_eq!(io.children.len(), children);
    // After the republish the branch is published again: the next promote is
    // a first one.
    let republished = f.request();
    let outcome = branch(run_at(&f, republished.clone(), &mut io, later));
    assert_eq!(outcome.answer(), record::PromotionAnswer::Promoted);
    let r = f.stored(republished.operation_id);
    assert_eq!(r.situation, Some(record::PromotionSituation::First));
    assert_eq!(r.step, record::PromotionStep::Complete);
    assert_eq!(f.link_row().map(|row| row.4), Some(CLOCK + 86_400));
    assert_primary_untouched(&f, &io, primary, "declined");
}
