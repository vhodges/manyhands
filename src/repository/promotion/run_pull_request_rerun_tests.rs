//! A pull-request promote that stopped, was run again, crashed, was held or
//! cancelled, and what it gives retirement (Wave 06 Cycle 04, Task 4): the
//! design's required outcomes of a re-run (RR1 to RR8, tried in the order
//! RR2, RR7, RR3, RR6, RR5, RR4, RR1), of Part C (C12 to C23, C25 to C29) and
//! of the fence (C22), written before the behaviour that gives them.
//!
//! Each test is a table: one row per outcome, every row on a fixture of its
//! own, every row's mismatches reported together, so that a red run says
//! which rows fail and how. A row that panics is a mismatch of its own. Every
//! row asserts that the local primary did not move, that no synchronization
//! targeted the primary and that no merge of the promotion exists
//! (`untouched`), as Task 3's tests do.
use super::*;
use crate::repository::{
    Mutation, MutationCall, NoProgress, RequestId,
    promotion::record::{PromotionAnswer, PromotionStep, Refusal},
};
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

const EXTRA: &str = "notes/extra.txt";
const DIRECT_CONFIG: &str =
    "format_version = 1\nprimary_branch = \"main\"\npublication_remote = \"origin\"\n";
const FORGE_CONFIG: &str =
    "[forge]\npull_request_url = \"https://forge.example/pr?source={source}&target={target}\"\n";

macro_rules! need {
    ($condition:expr, $($message:tt)+) => {
        if !$condition {
            return Err(format!($($message)+));
        }
    };
}

type Row = (&'static str, fn(&Fixture) -> Result<(), String>);

/// Runs each row on a fixture of its own (a pull-request repository with the
/// document published), and reports every row that does not give its
/// outcome.
fn rows(table: &[Row]) {
    let mut wrong = Vec::new();
    for (name, row) in table {
        let ran = catch_unwind(AssertUnwindSafe(|| row(&Fixture::pull_request())));
        match ran {
            Ok(Ok(())) => {}
            Ok(Err(mismatch)) => wrong.push(format!("{name}: {mismatch}")),
            Err(payload) => {
                let text = payload
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| {
                        payload
                            .downcast_ref::<&str>()
                            .map(|text| (*text).to_owned())
                    })
                    .unwrap_or_else(|| "a panic".to_owned());
                wrong.push(format!("{name}: panicked: {text}"));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "{} of {} rows:\n{}",
        wrong.len(),
        table.len(),
        wrong.join("\n")
    );
}

fn transport_stop() -> SynchronizationError {
    SynchronizationError::Transport(crate::repository::transport::SshTransportError {
        root: "/private/path/sentinel".into(),
        remote_name: "private-endpoint-sentinel".into(),
        direction: crate::repository::transport::SshDirection::Push,
        selected_key_id: None,
        authority: None,
        kind: crate::repository::transport::SshTransportErrorKind::TransportUnavailable,
    })
}

fn go(
    f: &Fixture,
    req: &PromoteDocumentRequest,
    io: &mut PullRequestIo,
) -> Result<DocumentPromotionResult, DocumentPromotionError> {
    run_at(f, req.clone(), io, clock)
}

fn stops_at_its_child(
    result: &Result<DocumentPromotionResult, DocumentPromotionError>,
    what: &str,
) -> Result<(), String> {
    need!(
        matches!(
            result,
            Err(DocumentPromotionError::ContextSynchronization(_))
        ),
        "{what}: expected the child's own stop, got {result:?}"
    );
    Ok(())
}

/// The first run of a new request, which stops at its context child with a
/// transport stop (`kept` says what the child's row is left as, none for a
/// stop that leaves no row): the record is `confirmed`.
fn stopped(
    f: &Fixture,
    io: &mut PullRequestIo,
    kept: Option<Kept>,
) -> Result<PromoteDocumentRequest, String> {
    let req = f.request();
    io.stop = Some((transport_stop(), kept.is_some()));
    io.kept_as = kept.unwrap_or_default();
    let result = go(f, &req, io);
    stops_at_its_child(&result, "the first run")?;
    let r = f.stored(req.operation_id);
    need!(
        r.step == PromotionStep::Confirmed,
        "the first run left {:?}, not confirmed",
        r.step
    );
    Ok(req)
}

/// The attempts of the context child that `child()` minted, not the calls
/// that restarted one: an acknowledgement of a cancelled row is a call of the
/// fake too, and synchronizes nothing.
fn attempts(io: &PullRequestIo) -> usize {
    io.children
        .iter()
        .filter(|(_, _, restart)| !restart)
        .count()
}

fn published(f: &Fixture) -> Option<Oid> {
    f.repo.refname_to_id(&tracking_context_ref()).ok()
}

/// Child rows that hold the context: none once a record has let go.
fn active_children(f: &Fixture) -> i64 {
    f.db()
        .query_row(
            "SELECT count(*) FROM remote_operation_records WHERE action='synchronize_context' AND phase IN ('reserved','advertising','persisting','fetch_prepared','fetch_observed','local_prepared','local_fast_forwarded','push_prepared','push_returned','push_verified','reconciling')",
            [],
            |row| row.get(0),
        )
        .unwrap()
}

/// What no pull-request promote does, as `assert_primary_untouched` says it,
/// as a row's mismatch.
fn untouched(f: &Fixture, ios: &[&PullRequestIo], primary: Oid) -> Result<(), String> {
    need!(f.primary() == primary, "the local primary moved");
    for io in ios {
        need!(io.primary_calls == 0, "the primary was synchronized");
        need!(io.deletes == 0, "a branch was deleted");
    }
    need!(
        promotion_commits(&f.repo) == 0,
        "a merge of the promotion was made"
    );
    let audit = state::with_transaction(&f.service, f.root.path(), |tx, _| record::audit(tx));
    need!(
        audit.is_ok(),
        "the registry audit refuses the record: {audit:?}"
    );
    Ok(())
}

fn answered(
    result: Result<DocumentPromotionResult, DocumentPromotionError>,
) -> Result<BranchPromotionOutcome, String> {
    match result {
        Ok(DocumentPromotionResult::Branch(outcome)) => Ok(outcome),
        Ok(DocumentPromotionResult::BranchIndexPending(pending)) => Ok(pending.authoritative),
        other => Err(format!("not an answer: {other:?}")),
    }
}

/// A finished record that answered `want`: `complete`, no refusal.
fn finishes(
    f: &Fixture,
    req: &PromoteDocumentRequest,
    result: Result<DocumentPromotionResult, DocumentPromotionError>,
    want: PromotionAnswer,
) -> Result<BranchPromotionOutcome, String> {
    let outcome = answered(result)?;
    need!(
        outcome.answer() == want,
        "answered {:?}, not {want:?}",
        outcome.answer()
    );
    let r = f.stored(req.operation_id);
    need!(
        r.step == PromotionStep::Complete,
        "the record is {:?}, not complete",
        r.step
    );
    need!(
        r.refusal.is_none(),
        "a finished record holds {:?}",
        r.refusal
    );
    need!(
        r.answer_code == Some(want),
        "the record stores {:?}",
        r.answer_code
    );
    Ok(outcome)
}

/// An ended record (the second answer table): `abandoned` with the refusal
/// `reconfirmation_required` and the answer `want`, nothing published and no
/// child holding the context.
fn ends(
    f: &Fixture,
    req: &PromoteDocumentRequest,
    result: Result<DocumentPromotionResult, DocumentPromotionError>,
    want: PromotionAnswer,
) -> Result<(), String> {
    let outcome = answered(result)?;
    need!(
        outcome.answer() == want,
        "answered {:?}, not {want:?}",
        outcome.answer()
    );
    need!(
        outcome.publication().is_none(),
        "an ended record reports a publication"
    );
    let r = f.stored(req.operation_id);
    need!(
        r.step == PromotionStep::Abandoned,
        "the record is {:?}, not abandoned",
        r.step
    );
    need!(
        r.refusal == Some(Refusal::ReconfirmationRequired),
        "the record's refusal is {:?}",
        r.refusal
    );
    need!(
        r.answer_code == Some(want),
        "the record stores {:?}",
        r.answer_code
    );
    need!(active_children(f) == 0, "a child still holds the context");
    Ok(())
}

/// A refused record: `external_change`, `abandoned`, no answer, nothing
/// published, no child run since `children`, none holding the context, and
/// the same request answers the same again.
fn refuses(
    f: &Fixture,
    req: &PromoteDocumentRequest,
    io: &mut PullRequestIo,
    result: Result<DocumentPromotionResult, DocumentPromotionError>,
    children: usize,
    tracking: Option<Oid>,
    links: i64,
) -> Result<(), String> {
    need!(
        matches!(result, Err(DocumentPromotionError::ExternalChange)),
        "expected external_change, got {result:?}"
    );
    let r = f.stored(req.operation_id);
    need!(
        r.step == PromotionStep::Abandoned,
        "the record is {:?}, not abandoned",
        r.step
    );
    need!(
        r.refusal == Some(Refusal::ExternalChange),
        "the record's refusal is {:?}",
        r.refusal
    );
    need!(r.answer_code.is_none(), "a refused record holds an answer");
    need!(
        attempts(io) == children,
        "a child ran: {} attempts, not {children}",
        attempts(io)
    );
    need!(published(f) == tracking, "the branch was published");
    need!(f.link_rows() == links, "a link was given");
    need!(active_children(f) == 0, "a child still holds the context");
    let again = go(f, req, io);
    need!(
        matches!(again, Err(DocumentPromotionError::ExternalChange)),
        "the same request again: {again:?}"
    );
    need!(attempts(io) == children, "the same request ran a child");
    Ok(())
}

/// A process lost inside the child, after it completed its row and moved its
/// tracking ref and before it returned to the runner.
fn lost(
    f: &Fixture,
    req: &PromoteDocumentRequest,
    io: &mut PullRequestIo,
    at: fn() -> OffsetDateTime,
) -> Result<(), String> {
    io.crash_after_completion = true;
    let ran = catch_unwind(AssertUnwindSafe(|| run_at(f, req.clone(), io, at)));
    io.crash_after_completion = false;
    match ran {
        Err(payload) if payload.is::<Crash>() => {
            let r = f.stored(req.operation_id);
            need!(
                r.step == PromotionStep::Confirmed,
                "the lost run left {:?}",
                r.step
            );
            let child = r.context_sync_id.ok_or("no child was minted")?;
            need!(
                f.child_phase(child).as_deref() == Some("completed"),
                "the child is {:?}",
                f.child_phase(child)
            );
            Ok(())
        }
        Err(payload) => resume_unwind(payload),
        Ok(result) => Err(format!("the run was not lost: {result:?}")),
    }
}

/// A process lost once the record was `context_synchronized`.
fn lost_after_synchronized(
    f: &Fixture,
    req: &PromoteDocumentRequest,
    io: &mut PullRequestIo,
) -> Result<(), String> {
    let failing = RepositoryService::open_at_with_failure_point_for_testing(
        f.data.path(),
        FailurePoint::AfterContextSync,
    )
    .unwrap();
    let result = run::run_with_io(
        &DocumentPromotionLifecycle,
        &failing,
        request_at(req.clone(), clock),
        io,
    );
    need!(result.is_err(), "the run was not lost: {result:?}");
    let r = f.stored(req.operation_id);
    need!(
        r.step == PromotionStep::ContextSynchronized,
        "the lost run left {:?}",
        r.step
    );
    Ok(())
}

// What a person or another clone does while a request waits.

fn save(f: &Fixture) {
    f.edit("a save");
}
fn git_commit(f: &Fixture) {
    write(&f.context, EXTRA, "committed with Git\n");
    commit(&f.context);
}
/// A poll's fast-forward of the branch to another clone's publication.
fn polled(f: &Fixture) {
    let tip = f.tip();
    let other = f.commit_tree(tip, &[(EXTRA, b"another clone's publication\n")], &[tip]);
    f.repo
        .reference(&tracking_context_ref(), other, true, "fixture fetch")
        .unwrap();
    checkout(&f.context, other);
}
fn rename_remote(f: &Fixture) {
    f.repo
        .remote("other", "ssh://fixture@127.0.0.1:1/repository")
        .unwrap();
    f.set_config(&PULL_REQUEST_CONFIG.replace("origin", "other"));
}
/// The remote configured back as `rename_remote` found it: the extra remote
/// removed, and the configuration as it was.
fn rename_back(f: &Fixture) {
    f.repo.remote_delete("other").unwrap();
    f.set_config(PULL_REQUEST_CONFIG);
}
fn move_endpoint_back(f: &Fixture) {
    f.repo
        .remote_set_url("origin", "ssh://fixture@127.0.0.1:1/repository")
        .unwrap();
}
fn move_endpoint(f: &Fixture) {
    f.repo
        .remote_set_url("origin", "ssh://fixture@127.0.0.1:2/moved")
        .unwrap();
}
fn move_push_endpoint(f: &Fixture) {
    f.repo
        .remote_set_pushurl("origin", Some("ssh://fixture@127.0.0.1:3/pushed"))
        .unwrap();
}
fn unreadable_configuration(f: &Fixture) {
    f.set_config("this is not { a configuration");
}
/// The document's context removed, as a retirement or a person leaves it:
/// the worktree and the branch.
fn remove_context(f: &Fixture) {
    f.repo
        .find_worktree(ITEM)
        .unwrap()
        .prune(Some(
            git2::WorktreePruneOptions::new()
                .valid(true)
                .locked(true)
                .working_tree(true),
        ))
        .unwrap();
    f.repo
        .find_branch(
            &format!("manyhands/document/{ITEM}"),
            git2::BranchType::Local,
        )
        .unwrap()
        .delete()
        .unwrap();
}
/// The forge merged the pull request by squash: the tracking primary holds
/// the consented tip's document.
fn merged(f: &Fixture) {
    f.set_tracking_main(f.squash(f.tip()));
}
fn cancel_at(f: &Fixture, op: OperationId, at: FailurePoint) {
    let root = f.root.path().to_owned();
    let mut once = false;
    hook(f, move |service, point| {
        if point == at && !once {
            once = true;
            assert!(service.cancel_document_promotion(&root, op).unwrap());
        }
    });
}
/// A save made once the run is at `at`.
fn save_at(f: &Fixture, at: FailurePoint) {
    let context = f.context.workdir().unwrap().to_owned();
    let mut once = false;
    hook(f, move |_, point| {
        if point == at && !once {
            once = true;
            let repo = git2::Repository::open(&context).unwrap();
            let path = context.join(DOC);
            let mut text = fs::read_to_string(&path).unwrap();
            text.push_str("a save between the check and the child\n");
            fs::write(path, text).unwrap();
            commit(&repo);
        }
    });
}

/// A stopped request, a change, the same request again.
struct Rerun {
    req: PromoteDocumentRequest,
    io: PullRequestIo,
    primary: Oid,
    children: usize,
    tracking: Option<Oid>,
    links: i64,
    result: Result<DocumentPromotionResult, DocumentPromotionError>,
}
fn rerun(
    f: &Fixture,
    kept: Option<Kept>,
    change: impl FnOnce(&Fixture, &mut PullRequestIo),
) -> Result<Rerun, String> {
    let primary = f.primary();
    let mut io = PullRequestIo::default();
    let req = stopped(f, &mut io, kept)?;
    let children = attempts(&io);
    change(f, &mut io);
    // What stands once the change is made: the run must add nothing to it.
    let tracking = published(f);
    let links = f.link_rows();
    let result = go(f, &req, &mut io);
    Ok(Rerun {
        req,
        io,
        primary,
        children,
        tracking,
        links,
        result,
    })
}
/// RR4 and RR5: the record ends refused, before the child runs again.
fn is_refused(
    f: &Fixture,
    kept: Option<Kept>,
    change: impl FnOnce(&Fixture, &mut PullRequestIo),
) -> Result<(), String> {
    let mut r = rerun(f, kept, change)?;
    refuses(
        f, &r.req, &mut r.io, r.result, r.children, r.tracking, r.links,
    )?;
    untouched(f, &[&r.io], r.primary)
}
/// RR3 and RR1: the same request goes on to its first answer.
fn goes_on(
    f: &Fixture,
    kept: Option<Kept>,
    change: impl FnOnce(&Fixture, &mut PullRequestIo),
) -> Result<(), String> {
    let r = rerun(f, kept, change)?;
    finishes(f, &r.req, r.result, PromotionAnswer::Promoted)?;
    need!(attempts(&r.io) > r.children, "the child did not run again");
    untouched(f, &[&r.io], r.primary)
}

/// A conflict of the context child, retained: the branch holds a line of the
/// document that the incoming copy of the branch changes too. Returns the
/// request, the child and the local bytes of the document.
fn conflicted(
    f: &Fixture,
    io: &mut PullRequestIo,
) -> Result<(PromoteDocumentRequest, OperationId, Vec<u8>), String> {
    let base = f.tip();
    let first = f.blob_at(base, DOC);
    f.edit("a local line");
    let local = f.blob_at(f.tip(), DOC);
    let req = f.request();
    io.drive = Some(f.commit_tree(
        base,
        &[(DOC, &[&first[..], b"a remote line\n"].concat())],
        &[base],
    ));
    let result = go(f, &req, io);
    need!(
        matches!(&result, Err(DocumentPromotionError::ContextSynchronization(e)) if matches!(**e, SynchronizationError::ConflictPending { .. })),
        "expected a conflict, got {result:?}"
    );
    let child = f
        .stored(req.operation_id)
        .context_sync_id
        .ok_or("no child was minted")?;
    Ok((req, child, local))
}

/// Resolves a retained conflict of `child` with `local` bytes, through the
/// conflict verbs' library calls.
fn resolve(f: &Fixture, child: OperationId, local: Vec<u8>) {
    use crate::repository::{
        RedactedConflictBytes, ResolveSynchronizationOutcome, ResolveSynchronizationRequest,
    };
    let inspection = f
        .service
        .inspect_synchronization_recovery(f.root.path(), child)
        .unwrap();
    assert_eq!(inspection.paths.len(), 1);
    f.service
        .read_synchronization_conflict(&inspection.paths[0].token)
        .unwrap();
    let resolution = ResolveSynchronizationRequest::new(
        f.root.path().into(),
        child,
        OperationId::new(),
        inspection.observation.clone(),
        vec![(
            inspection.paths[0].token.clone(),
            RedactedConflictBytes::from_bytes(local),
        )],
        None,
    );
    assert!(matches!(
        f.service.resolve_synchronization(resolution).unwrap(),
        ResolveSynchronizationOutcome::LocalCheckpointComplete { .. }
    ));
}

/// A request run again after its child completed, however the run was lost:
/// the child's row is complete, the record is not finished.
#[derive(Clone, Copy)]
enum Lost {
    /// Before `context_synchronized` was saved.
    BeforeSaved,
    /// After it.
    AfterSaved,
}
/// A new request whose child completes and whose process is lost as `how`
/// says, then `change`, then the same request on a fresh publisher: it runs
/// no child, and finishes by the first answer table. `links` is the number
/// of link rows it leaves.
fn completed_then(
    f: &Fixture,
    how: Lost,
    change: fn(&Fixture),
    want: PromotionAnswer,
    links: i64,
) -> Result<(), String> {
    let primary = f.primary();
    let req = f.request();
    let mut io = PullRequestIo::default();
    match how {
        Lost::BeforeSaved => lost(f, &req, &mut io, clock)?,
        Lost::AfterSaved => lost_after_synchronized(f, &req, &mut io)?,
    }
    let children = io.children.len();
    change(f);
    let mut again = PullRequestIo::default();
    let result = go(f, &req, &mut again);
    finishes(f, &req, result, want)?;
    need!(again.children.is_empty(), "the child ran a second time");
    need!(
        io.children.len() == children,
        "the lost run's child ran again"
    );
    need!(
        f.link_rows() == links,
        "{} link rows, not {links}",
        f.link_rows()
    );
    untouched(f, &[&io, &again], primary)
}

fn nothing(_: &Fixture) {}
/// A new attempt cancelled before its push, whose intent the record settled
/// as unsent: the row is `cancelled` at `before_push`, and its outcome says
/// the intent was answered, so no later synchronization counts it unanswered.
fn settled(f: &Fixture, req: &PromoteDocumentRequest) -> Result<(), String> {
    let child = f
        .stored(req.operation_id)
        .context_sync_id
        .ok_or("no child was minted")?;
    let row: (String, Option<String>, Option<String>) = f
        .db()
        .query_row(
            "SELECT phase,completed_step,outcome FROM remote_operation_records WHERE operation_ulid=?1",
            [child.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|e| format!("the child's row: {e}"))?;
    need!(
        row == (
            "cancelled".to_owned(),
            Some("before_push".to_owned()),
            Some("protocol_rejected".to_owned())
        ),
        "the child's row is {row:?}, not cancelled at before_push and settled"
    );
    Ok(())
}

/// RR1 to RR8, in the order the table tries them: RR2, RR7, RR3, RR6, RR5,
/// RR4, RR1. Each row is a place where two rows could both match, or the row
/// alone.
#[test]
fn re_run_outcomes() {
    rows(&[
        // RR1: the branch moved only by the record's own child.
        ("RR1 the child's fast-forward, then a stop", |f| {
            let primary = f.primary();
            let tip = f.tip();
            let mut io = PullRequestIo {
                drive: Some(f.commit_tree(tip, &[(EXTRA, b"another clone's save\n")], &[tip])),
                stop_after_drive: Some(transport_stop()),
                ..PullRequestIo::default()
            };
            let req = f.request();
            stops_at_its_child(&go(f, &req, &mut io), "the first run")?;
            need!(f.tip() != tip, "the child did not move the branch");
            need!(
                f.stored(req.operation_id).step == PromotionStep::Confirmed,
                "the record left confirmed"
            );
            let children = io.children.len();
            let result = go(f, &req, &mut io);
            finishes(f, &req, result, PromotionAnswer::Promoted)?;
            need!(io.children.len() > children, "the child did not run again");
            untouched(f, &[&io], primary)
        }),
        ("RR1 the child's Merge remote context, then a stop", |f| {
            let primary = f.primary();
            let tip = f.tip();
            let parent = f.repo.find_commit(tip).unwrap().parent_id(0).unwrap();
            let mut io = PullRequestIo {
                drive: Some(f.commit_tree(
                    parent,
                    &[(EXTRA, b"another clone's save\n")],
                    &[parent],
                )),
                stop_after_drive: Some(transport_stop()),
                ..PullRequestIo::default()
            };
            let req = f.request();
            stops_at_its_child(&go(f, &req, &mut io), "the first run")?;
            need!(f.tip() != tip, "the child did not move the branch");
            need!(
                f.repo.find_commit(f.tip()).unwrap().summary()
                    == Some(format!("Merge remote context {ITEM}").as_str()),
                "the child made no merge"
            );
            let children = io.children.len();
            let result = go(f, &req, &mut io);
            finishes(f, &req, result, PromotionAnswer::Promoted)?;
            need!(io.children.len() > children, "the child did not run again");
            untouched(f, &[&io], primary)
        }),
        (
            "RR1 the child's Merge primary into document, then a stop",
            |f| {
                let primary = f.primary();
                let tip = f.tip();
                let mut io = PullRequestIo {
                    fetched: Some(f.primary_moves_on()),
                    drive: Some(tip),
                    stop_after_drive: Some(transport_stop()),
                    ..PullRequestIo::default()
                };
                let req = f.request();
                stops_at_its_child(&go(f, &req, &mut io), "the first run")?;
                need!(
                    f.repo.find_commit(f.tip()).unwrap().summary()
                        == Some(format!("Merge primary into document {ITEM}").as_str()),
                    "the child made no merge"
                );
                let children = io.children.len();
                let result = go(f, &req, &mut io);
                finishes(f, &req, result, PromotionAnswer::Promoted)?;
                need!(io.children.len() > children, "the child did not run again");
                untouched(f, &[&io], primary)
            },
        ),
        // RR1 across attempts: the record keeps only the latest attempt, so
        // the head the child's merge made is the record's own (the accepted
        // head), though the attempt that made it is gone.
        (
            "RR1 across attempts: the child's merge and a stop, a stop with no row, then the same request goes on",
            |f| {
                let primary = f.primary();
                let tip = f.tip();
                let mut io = PullRequestIo {
                    fetched: Some(f.primary_moves_on()),
                    drive: Some(tip),
                    stop_after_drive: Some(transport_stop()),
                    ..PullRequestIo::default()
                };
                let req = f.request();
                stops_at_its_child(&go(f, &req, &mut io), "the first run")?;
                let merge = f.tip();
                need!(merge != tip, "the child did not merge");
                io.stop = Some((SynchronizationError::Busy, false));
                stops_at_its_child(&go(f, &req, &mut io), "the second run")?;
                let r = f.stored(req.operation_id);
                need!(
                    r.step == PromotionStep::Confirmed && r.refusal.is_none(),
                    "the record is {:?} with {:?}",
                    r.step,
                    r.refusal
                );
                let result = go(f, &req, &mut io);
                finishes(f, &req, result, PromotionAnswer::Promoted)?;
                need!(published(f) == Some(merge), "the merge was not published");
                untouched(f, &[&io], primary)
            },
        ),
        // RR4 before RR1: the child's own merge, and a commit it did not make.
        ("RR4 over RR1: the child's merge, then a save", |f| {
            let tip = f.tip();
            let primary = f.primary();
            let mut io = PullRequestIo {
                fetched: Some(f.primary_moves_on()),
                drive: Some(tip),
                stop_after_drive: Some(transport_stop()),
                ..PullRequestIo::default()
            };
            let req = f.request();
            stops_at_its_child(&go(f, &req, &mut io), "the first run")?;
            let children = attempts(&io);
            let tracking = published(f);
            save(f);
            let result = go(f, &req, &mut io);
            refuses(f, &req, &mut io, result, children, tracking, 0)?;
            untouched(f, &[&io], primary)
        }),
        // RR2: the child completed.
        ("RR2 completed before context_synchronized was saved", |f| {
            completed_then(f, Lost::BeforeSaved, nothing, PromotionAnswer::Promoted, 1)
        }),
        ("RR2 completed and context_synchronized saved", |f| {
            completed_then(f, Lost::AfterSaved, nothing, PromotionAnswer::Promoted, 1)
        }),
        (
            "RR2 completed, the context branch gone before it was saved (and RR8: no link)",
            |f| {
                completed_then(
                    f,
                    Lost::BeforeSaved,
                    remove_context,
                    PromotionAnswer::Promoted,
                    0,
                )
            },
        ),
        (
            "RR2 completed, the context branch gone after it was saved (and RR8: no link)",
            |f| {
                completed_then(
                    f,
                    Lost::AfterSaved,
                    remove_context,
                    PromotionAnswer::Promoted,
                    0,
                )
            },
        ),
        (
            "RR2 completed, the publication remote renamed after it",
            |f| {
                completed_then(
                    f,
                    Lost::BeforeSaved,
                    rename_remote,
                    PromotionAnswer::Promoted,
                    1,
                )
            },
        ),
        (
            "RR2 completed, the remote renamed after context_synchronized",
            |f| {
                completed_then(
                    f,
                    Lost::AfterSaved,
                    rename_remote,
                    PromotionAnswer::Promoted,
                    1,
                )
            },
        ),
        ("RR2 completed, the fetch endpoint changed after it", |f| {
            completed_then(
                f,
                Lost::BeforeSaved,
                move_endpoint,
                PromotionAnswer::Promoted,
                1,
            )
        }),
        ("RR2 completed, the push endpoint changed after it", |f| {
            completed_then(
                f,
                Lost::BeforeSaved,
                move_push_endpoint,
                PromotionAnswer::Promoted,
                1,
            )
        }),
        (
            "RR2 completed, the configuration unreadable after it",
            |f| {
                completed_then(
                    f,
                    Lost::BeforeSaved,
                    unreadable_configuration,
                    PromotionAnswer::Promoted,
                    1,
                )
            },
        ),
        (
            "RR2 completed, the configuration unreadable after context_synchronized",
            |f| {
                completed_then(
                    f,
                    Lost::AfterSaved,
                    unreadable_configuration,
                    PromotionAnswer::Promoted,
                    1,
                )
            },
        ),
        ("RR2 a save after the child completed", |f| {
            completed_then(f, Lost::BeforeSaved, save, PromotionAnswer::Promoted, 1)
        }),
        (
            "RR2 over RR7: completed and the branch gone and the remote renamed",
            |f| {
                completed_then(
                    f,
                    Lost::BeforeSaved,
                    |f| {
                        remove_context(f);
                        rename_remote(f);
                    },
                    PromotionAnswer::Promoted,
                    0,
                )
            },
        ),
        (
            "RR2 cancellation asked after the child completed (C26)",
            |f| {
                let primary = f.primary();
                let req = f.request();
                cancel_at(f, req.operation_id, FailurePoint::AfterContextSync);
                let mut io = PullRequestIo::default();
                let result = go(f, &req, &mut io);
                finishes(f, &req, result, PromotionAnswer::Promoted)?;
                need!(f.link_rows() == 1, "no link was given");
                untouched(f, &[&io], primary)
            },
        ),
        (
            "RR2 table 1 is judged on the child's authority OID, not the consented tip",
            |f| {
                // The forge squashed the consented tip, and the child's fetch
                // brought it; the child also brought another clone's later
                // save, which its merge of the primary keeps. The consented
                // tip alone is held; the published merge is not.
                let primary = f.primary();
                let tip = f.tip();
                let later = f.commit_tree(tip, &[(EXTRA, b"another clone's later save\n")], &[tip]);
                let mut io = PullRequestIo {
                    fetched: Some(f.squash(tip)),
                    drive: Some(later),
                    ..PullRequestIo::default()
                };
                let req = f.request();
                let result = go(f, &req, &mut io);
                finishes(f, &req, result, PromotionAnswer::Promoted)?;
                need!(f.link_rows() == 1, "no link was given");
                untouched(f, &[&io], primary)
            },
        ),
        (
            "RR2 table 1 is judged on the child's authority OID: another clone's later save, squashed",
            |f| {
                // Another clone published a later save of the document, which
                // the forge squashed; the child fast-forwards to it. The
                // authority is that save and the primary holds it; the
                // consented tip is not what the branch holds any more, so
                // judging on it would read the document as not held.
                let primary = f.primary();
                let tip = f.tip();
                let first = f.blob_at(tip, DOC);
                let later = f.commit_tree(
                    tip,
                    &[(DOC, &[&first[..], b"another clone's later line\n"].concat())],
                    &[tip],
                );
                let mut io = PullRequestIo {
                    fetched: Some(f.squash(later)),
                    drive: Some(later),
                    ..PullRequestIo::default()
                };
                let req = f.request();
                let result = go(f, &req, &mut io);
                finishes(f, &req, result, PromotionAnswer::PrimaryHolds)?;
                need!(f.link_rows() == 0, "a link was given");
                untouched(f, &[&io], primary)
            },
        ),
        (
            "RR2 table 2 reads R as absent after remote_branch_deleted",
            |f| {
                // Another clone's publication is ahead of the consented tip;
                // the forge squashed the tip and deleted the branch. A fetch
                // does not prune the tracking branch, which reads as absent.
                let primary = f.primary();
                let tip = f.tip();
                let ahead = f.commit_tree(tip, &[(EXTRA, b"another clone's save\n")], &[tip]);
                f.repo
                    .reference(&tracking_context_ref(), ahead, true, "fixture fetch")
                    .unwrap();
                let mut io = PullRequestIo {
                    fetched: Some(f.squash(tip)),
                    stop: Some((SynchronizationError::RemoteContextDeleted, true)),
                    ..PullRequestIo::default()
                };
                let req = f.request();
                let result = go(f, &req, &mut io);
                ends(f, &req, result, PromotionAnswer::PrimaryHolds)?;
                untouched(f, &[&io], primary)
            },
        ),
        // RR7: the context branch is absent before the child.
        (
            "RR7 the branch removed, the document merged: primary_holds",
            |f| {
                let primary = f.primary();
                let mut io = PullRequestIo::default();
                let req = stopped(f, &mut io, None)?;
                merged(f);
                remove_context(f);
                let children = io.children.len();
                let result = go(f, &req, &mut io);
                ends(f, &req, result, PromotionAnswer::PrimaryHolds)?;
                need!(io.children.len() == children, "a child ran");
                untouched(f, &[&io], primary)
            },
        ),
        ("RR7 the branch removed, not merged: context_gone", |f| {
            let primary = f.primary();
            let mut io = PullRequestIo::default();
            let req = stopped(f, &mut io, None)?;
            remove_context(f);
            let children = io.children.len();
            let result = go(f, &req, &mut io);
            ends(f, &req, result, PromotionAnswer::ContextGone)?;
            need!(io.children.len() == children, "a child ran");
            need!(f.link_rows() == 0, "a link was given");
            untouched(f, &[&io], primary)
        }),
        (
            "RR7 over RR5: the branch removed and the remote renamed",
            |f| {
                let primary = f.primary();
                let mut io = PullRequestIo::default();
                let req = stopped(f, &mut io, None)?;
                remove_context(f);
                rename_remote(f);
                let result = go(f, &req, &mut io);
                ends(f, &req, result, PromotionAnswer::ContextGone)?;
                untouched(f, &[&io], primary)
            },
        ),
        (
            "RR7 over RR5: the branch removed and the endpoint changed",
            |f| {
                let primary = f.primary();
                let mut io = PullRequestIo::default();
                let req = stopped(f, &mut io, None)?;
                remove_context(f);
                move_endpoint(f);
                let result = go(f, &req, &mut io);
                ends(f, &req, result, PromotionAnswer::ContextGone)?;
                untouched(f, &[&io], primary)
            },
        ),
        ("RR7 with a live child that can be let go", |f| {
            let primary = f.primary();
            let mut io = PullRequestIo::default();
            let req = stopped(f, &mut io, Some(Kept::Reserved))?;
            remove_context(f);
            let result = go(f, &req, &mut io);
            ends(f, &req, result, PromotionAnswer::ContextGone)?;
            untouched(f, &[&io], primary)
        }),
        // RR3: an attempt may have pushed.
        // While renamed the replacement attempt is refused at its reservation
        // (the record-remote identity check): the record waits, refusing
        // nothing, and goes on once the remote is configured back (decision
        // 17, known limit K-20).
        (
            "RR3 may have pushed, then the remote renamed: the record waits; configured back: goes on",
            |f| {
                let primary = f.primary();
                let mut io = PullRequestIo::default();
                let req = stopped(f, &mut io, Some(Kept::MayHavePushed))?;
                let tracking = published(f);
                rename_remote(f);
                let result = go(f, &req, &mut io);
                stops_at_its_child(&result, "the record while the remote is renamed")?;
                let r = f.stored(req.operation_id);
                need!(
                    r.step == PromotionStep::Confirmed && r.refusal.is_none(),
                    "the record is {:?} with {:?}",
                    r.step,
                    r.refusal
                );
                need!(
                    r.context_pushed,
                    "the record does not know an attempt may have pushed"
                );
                need!(published(f) == tracking, "something was published");
                rename_back(f);
                let result = go(f, &req, &mut io);
                finishes(f, &req, result, PromotionAnswer::Promoted)?;
                untouched(f, &[&io], primary)
            },
        ),
        ("RR3 may have pushed, then the endpoint changed", |f| {
            goes_on(f, Some(Kept::MayHavePushed), |f, _| move_endpoint(f))
        }),
        // RR3 across attempts: the record keeps only the latest attempt, so
        // that an attempt may have pushed is the record's own (the flag),
        // though the attempt's row is gone.
        (
            "RR3 across attempts: an attempt may have pushed, a stop with no row, then a save: goes on",
            |f| {
                let primary = f.primary();
                let mut io = PullRequestIo::default();
                let req = stopped(f, &mut io, Some(Kept::MayHavePushed))?;
                io.stop = Some((SynchronizationError::Busy, false));
                stops_at_its_child(&go(f, &req, &mut io), "the second run")?;
                need!(
                    f.stored(req.operation_id).refusal.is_none(),
                    "the record was refused"
                );
                save(f);
                let children = attempts(&io);
                // The new attempt is told of its safe points, as a real child
                // tells them: the start check admits it only because the
                // record knows an attempt may have pushed.
                io.paced = Some(Paced::default());
                let result = go(f, &req, &mut io);
                finishes(f, &req, result, PromotionAnswer::Promoted)?;
                need!(attempts(&io) > children, "the child did not run again");
                untouched(f, &[&io], primary)
            },
        ),
        ("RR3 may have pushed, then a save", |f| {
            goes_on(f, Some(Kept::MayHavePushed), |f, _| save(f))
        }),
        ("RR3 may have pushed, then a poll's fast-forward", |f| {
            goes_on(f, Some(Kept::MayHavePushed), |f, _| polled(f))
        }),
        // RR6: the child is retained; whatever changed, the record waits.
        // C29: the remote, or its endpoint, changed and changed back with no
        // call of the request between (any call would register the change
        // and strand the held child: decision 19, vh-zn1fd); then resolved.
        (
            "RR6 a conflict, the remote renamed and renamed back with no call in between; resolved: finishes (C29)",
            |f| {
                let primary = f.primary();
                let mut io = PullRequestIo::default();
                let (req, child, local) = conflicted(f, &mut io)?;
                rename_remote(f);
                rename_back(f);
                resolve(f, child, local);
                let result = go(f, &req, &mut io);
                finishes(f, &req, result, PromotionAnswer::Promoted)?;
                need!(
                    f.stored(req.operation_id).context_sync_id == Some(child),
                    "the resolved child is not the record's"
                );
                untouched(f, &[&io], primary)
            },
        ),
        (
            "RR6 a conflict, the endpoint changed and changed back with no call in between; resolved: finishes",
            |f| {
                let primary = f.primary();
                let mut io = PullRequestIo::default();
                let (req, child, local) = conflicted(f, &mut io)?;
                move_endpoint(f);
                move_endpoint_back(f);
                resolve(f, child, local);
                let result = go(f, &req, &mut io);
                finishes(f, &req, result, PromotionAnswer::Promoted)?;
                need!(
                    f.stored(req.operation_id).context_sync_id == Some(child),
                    "the resolved child is not the record's"
                );
                untouched(f, &[&io], primary)
            },
        ),
        (
            "RR6 a retained child, then a save and a remote change: the stop stands",
            |f| {
                let primary = f.primary();
                let mut io = PullRequestIo::default();
                let req = stopped(f, &mut io, Some(Kept::Retained))?;
                save(f);
                rename_remote(f);
                io.stop = Some((SynchronizationError::PushRejected, false));
                let result = go(f, &req, &mut io);
                stops_at_its_child(&result, "the record while the child is retained")?;
                let r = f.stored(req.operation_id);
                need!(
                    r.step == PromotionStep::Confirmed && r.refusal.is_none(),
                    "the record is {:?} with {:?}",
                    r.step,
                    r.refusal
                );
                untouched(f, &[&io], primary)
            },
        ),
        (
            "RR6 a retained child, then a poll's fast-forward: the stop stands",
            |f| {
                let primary = f.primary();
                let mut io = PullRequestIo::default();
                let req = stopped(f, &mut io, Some(Kept::Retained))?;
                polled(f);
                io.stop = Some((SynchronizationError::PushRejected, false));
                let result = go(f, &req, &mut io);
                stops_at_its_child(&result, "the record while the child is retained")?;
                need!(
                    f.stored(req.operation_id).refusal.is_none(),
                    "the record was refused"
                );
                untouched(f, &[&io], primary)
            },
        ),
        // RR5: the remote or its endpoint changed before any push.
        ("RR5 the publication remote renamed", |f| {
            is_refused(f, None, |f, _| rename_remote(f))
        }),
        ("RR5 the fetch endpoint changed", |f| {
            is_refused(f, None, |f, _| move_endpoint(f))
        }),
        ("RR5 the push endpoint changed", |f| {
            is_refused(f, None, |f, _| move_push_endpoint(f))
        }),
        (
            "RR5 the remote renamed with a live child that can be let go",
            |f| is_refused(f, Some(Kept::Reserved), |f, _| rename_remote(f)),
        ),
        (
            "RR5 the endpoint changed with a live child that can be let go",
            |f| is_refused(f, Some(Kept::Reserved), |f, _| move_endpoint(f)),
        ),
        // RR4: the branch holds a commit its child did not make.
        ("RR4 a save", |f| is_refused(f, None, |f, _| save(f))),
        ("RR4 a commit with Git", |f| {
            is_refused(f, None, |f, _| git_commit(f))
        }),
        (
            "RR4 a poll's fast-forward to another clone's publication",
            |f| is_refused(f, None, |f, _| polled(f)),
        ),
        ("RR4 a save with a live child that can be let go", |f| {
            is_refused(f, Some(Kept::Reserved), |f, _| save(f))
        }),
        ("RR4 a save between the check and the child's start", |f| {
            let primary = f.primary();
            let req = f.request();
            let tracking = published(f);
            save_at(f, FailurePoint::AfterConfirmed);
            let mut io = PullRequestIo::default();
            let result = go(f, &req, &mut io);
            refuses(f, &req, &mut io, result, 0, tracking, 0)?;
            untouched(f, &[&io], primary)
        }),
        // RR4 where a save lands relative to a new attempt's safe points.
        (
            "RR4 a save after the table, before the child read its head",
            |f| {
                let primary = f.primary();
                let req = f.request();
                let tracking = published(f);
                let mut io = PullRequestIo {
                    paced: Some(Paced::saving(SaveAt::BeforeStart)),
                    ..PullRequestIo::default()
                };
                let result = go(f, &req, &mut io);
                refuses(f, &req, &mut io, result, 1, tracking, 0)?;
                // Ended at the start check: the child fetched and recorded nothing
                // (the push check would end it later, with an intent).
                let child = f
                    .stored(req.operation_id)
                    .context_sync_id
                    .ok_or("no child was minted")?;
                let row: (String, Option<String>) = f
                .db()
                .query_row(
                    "SELECT phase,completed_step FROM remote_operation_records WHERE operation_ulid=?1",
                    [child.to_string()],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .map_err(|e| format!("the child's row: {e}"))?;
                need!(
                    row == ("cancelled".to_owned(), Some("before_fetch".to_owned())),
                    "the child's row is {row:?}, not cancelled at before_fetch"
                );
                untouched(f, &[&io], primary)
            },
        ),
        (
            "RR4 a save after a merging child read its head, before its merge",
            |f| {
                let primary = f.primary();
                let tip = f.tip();
                let req = f.request();
                let tracking = published(f);
                let mut io = PullRequestIo {
                    drive: Some(tip),
                    fetched: Some(f.primary_moves_on()),
                    paced: Some(Paced::saving(SaveAt::AfterFetch)),
                    ..PullRequestIo::default()
                };
                let result = go(f, &req, &mut io);
                refuses(f, &req, &mut io, result, 1, tracking, 0)?;
                settled(f, &req)?;
                need!(
                    f.repo.find_commit(f.tip()).unwrap().summary()
                        == Some(format!("Merge primary into document {ITEM}").as_str()),
                    "the child's merge onto the save is not kept on the branch"
                );
                untouched(f, &[&io], primary)
            },
        ),
        (
            "RR4 a save after a new attempt recorded its push intent, before its push check",
            |f| {
                let primary = f.primary();
                let req = f.request();
                let tracking = published(f);
                let mut io = PullRequestIo {
                    paced: Some(Paced::saving(SaveAt::BeforePush)),
                    ..PullRequestIo::default()
                };
                let result = go(f, &req, &mut io);
                refuses(f, &req, &mut io, result, 1, tracking, 0)?;
                settled(f, &req)?;
                untouched(f, &[&io], primary)
            },
        ),
        (
            "RR4 a save after the push check, before the child's own head check: refused",
            |f| {
                let primary = f.primary();
                let req = f.request();
                let tracking = published(f);
                let mut io = PullRequestIo {
                    paced: Some(Paced::saving(SaveAt::AfterPushCheck)),
                    ..PullRequestIo::default()
                };
                let result = go(f, &req, &mut io);
                refuses(f, &req, &mut io, result, 1, tracking, 0)?;
                settled(f, &req)?;
                need!(
                    !f.stored(req.operation_id).context_pushed,
                    "the record reads the refused push as one that may have happened"
                );
                untouched(f, &[&io], primary)
            },
        ),
        // RR1: the child's own moves reach its push.
        (
            "RR1 the child's fast-forward to another clone's publication reaches its push",
            |f| {
                let primary = f.primary();
                let tip = f.tip();
                let ahead =
                    f.commit_tree(tip, &[(EXTRA, b"another clone's publication\n")], &[tip]);
                let req = f.request();
                let mut io = PullRequestIo {
                    paced: Some(Paced {
                        fast_forward_to: Some(ahead),
                        ..Paced::default()
                    }),
                    ..PullRequestIo::default()
                };
                let result = go(f, &req, &mut io);
                finishes(f, &req, result, PromotionAnswer::Promoted)?;
                need!(
                    published(f) == Some(ahead),
                    "the fast-forward was not published"
                );
                untouched(f, &[&io], primary)
            },
        ),
        ("RR1 the child's merge reaches its push", |f| {
            let primary = f.primary();
            let tip = f.tip();
            let req = f.request();
            let mut io = PullRequestIo {
                drive: Some(tip),
                fetched: Some(f.primary_moves_on()),
                paced: Some(Paced::default()),
                ..PullRequestIo::default()
            };
            let result = go(f, &req, &mut io);
            finishes(f, &req, result, PromotionAnswer::Promoted)?;
            need!(
                f.repo.find_commit(f.tip()).unwrap().summary()
                    == Some(format!("Merge primary into document {ITEM}").as_str()),
                "the child made no merge"
            );
            need!(published(f) == Some(f.tip()), "the merge was not published");
            untouched(f, &[&io], primary)
        }),
        // RR6: a held restart goes on unchecked, though a save landed after
        // its conflict was resolved (the restart publishes it: K-16).
        (
            "RR6 a conflict at the primary stage, resolved, then a save: the same request publishes the save",
            |f| {
                let primary = f.primary();
                let tip = f.tip();
                let merge = f.merge_commit(tip);
                let first = f.blob_at(merge, DOC);
                let ahead = f.commit_tree(
                    merge,
                    &[(DOC, &[&first[..], b"a remote line\n"].concat())],
                    &[merge],
                );
                f.edit("a local line");
                let local = f.blob_at(f.tip(), DOC);
                let req = f.request();
                let mut io = PullRequestIo {
                    fetched: Some(ahead),
                    drive: Some(tip),
                    ..PullRequestIo::default()
                };
                let result = go(f, &req, &mut io);
                need!(
                    matches!(&result, Err(DocumentPromotionError::ContextSynchronization(e)) if matches!(**e, SynchronizationError::ConflictPending { .. })),
                    "expected a conflict, got {result:?}"
                );
                let child = f
                    .stored(req.operation_id)
                    .context_sync_id
                    .ok_or("no child was minted")?;
                resolve(f, child, local);
                let saved = f.edit("a save after the resolution");
                io.paced = Some(Paced::default());
                let result = go(f, &req, &mut io);
                finishes(f, &req, result, PromotionAnswer::Promoted)?;
                need!(
                    f.stored(req.operation_id).context_sync_id == Some(child),
                    "the resolved child is not the record's"
                );
                need!(published(f) == Some(saved), "the save was not published");
                untouched(f, &[&io], primary)
            },
        ),
        (
            "RR4 a save and the remote renamed: refused either way",
            |f| {
                is_refused(f, None, |f, _| {
                    save(f);
                    rename_remote(f);
                })
            },
        ),
    ]);
}

/// The heads the record's child counts as its own (`own_heads`), for an
/// attempt that began at `began` and applied `steps`, each from one head to
/// another. The heads are named: `T` the accepted head (the consented tip),
/// `a` and `b` commits the child made, `s` a save, `x` a head nobody made.
/// The row and its steps are the reservation layer's own (`prepare`, `begin`
/// and `observe` of each step); the answer is mapped back to the names.
fn own_heads_of(f: &Fixture, began: char, steps: &[(char, char)]) -> Result<String, String> {
    let tip = f.tip();
    let a = f.commit_tree(tip, &[(EXTRA, b"a\n")], &[tip]);
    let b = f.commit_tree(a, &[(EXTRA, b"b\n")], &[a]);
    let save = f.commit_tree(tip, &[(EXTRA, b"save\n")], &[tip]);
    let nobody = f.commit_tree(tip, &[(EXTRA, b"nobody\n")], &[tip]);
    let named = [('T', tip), ('a', a), ('b', b), ('s', save), ('x', nobody)];
    let oid = |name: char| named.iter().find(|(n, _)| *n == name).unwrap().1;
    let mut io = PullRequestIo::default();
    let req = stopped(f, &mut io, Some(Kept::Reserved))?;
    let record = f.stored(req.operation_id);
    let child = record.context_sync_id.ok_or("no child was minted")?;
    let heads = state::with_transaction(&f.service, f.root.path(), |tx, id| {
        tx.execute(
            "UPDATE remote_operation_records SET phase='fetch_prepared',sync_checkpoint='fetch_prepared',expected_oid=?2,local_oid=?2 WHERE operation_ulid=?1",
            [child.to_string(), oid(began).to_string()],
        )
        .map_err(|_| state::recovery_required())?;
        for (ordinal, (from, to)) in steps.iter().enumerate() {
            state::applied_step_for_test(tx, id, child, ordinal as u8, oid(*from), oid(*to))?;
        }
        let row = match record::child_state(tx, &record, record::ChildRole::Context)? {
            record::ChildState::Unowned(row) | record::ChildState::Retained(row) => row,
            _ => return Err(state::recovery_required()),
        };
        // `promotion_child_moves` is what the chain is built from.
        let moves = crate::repository::remote::reservation::promotion_child_moves(tx, &row)?;
        assert_eq!(
            moves,
            steps
                .iter()
                .map(|(from, to)| (oid(*from), oid(*to)))
                .collect::<Vec<_>>(),
            "the steps the layer recorded"
        );
        crate::repository::promotion::rerun::own_heads(tx, record.consented_context_oid.unwrap(), Some(&row))
    })
    .map_err(|e| format!("{e:?}"))?;
    Ok(heads
        .iter()
        .map(|head| named.iter().find(|(_, o)| o == head).unwrap().0)
        .collect())
}

/// A chain of heads the child made counts, and nothing else does: a step
/// that starts at a save (a merge built on a save, B1) leaves it out, and so
/// does every step of an attempt that did not begin at the accepted head.
#[test]
fn own_heads_follow_only_the_chain_from_the_accepted_head() {
    type Case = (&'static str, char, &'static [(char, char)], &'static str);
    let table: &[Case] = &[
        ("no step", 'T', &[], "T"),
        ("one step from the accepted head", 'T', &[('T', 'a')], "Ta"),
        (
            "two steps, each from the head before it",
            'T',
            &[('T', 'a'), ('a', 'b')],
            "Tab",
        ),
        // The step guard: a step counts only from a head already counted.
        (
            "a step from a save is not the child's own",
            'T',
            &[('s', 'b')],
            "T",
        ),
        (
            "a chain broken by a save: the step after it is not counted",
            'T',
            &[('T', 'a'), ('s', 'b')],
            "Ta",
        ),
        ("a step from a head nobody made", 'T', &[('x', 'a')], "T"),
        // The start guard: an attempt that began off the accepted head
        // counts nothing, whatever its steps say.
        ("an attempt that began at a save", 's', &[('T', 'a')], "T"),
        (
            "an attempt that began at a save, with a chain on it",
            's',
            &[('s', 'b')],
            "T",
        ),
    ];
    let mut wrong = Vec::new();
    for (name, began, steps, want) in table {
        match catch_unwind(AssertUnwindSafe(|| {
            own_heads_of(&Fixture::pull_request(), *began, steps)
        })) {
            Ok(Ok(heads)) if heads == *want => {}
            Ok(Ok(heads)) => wrong.push(format!("{name}: counted {heads}, not {want}")),
            Ok(Err(mismatch)) => wrong.push(format!("{name}: {mismatch}")),
            Err(_) => wrong.push(format!("{name}: panicked")),
        }
    }
    assert!(
        wrong.is_empty(),
        "{} rows:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

/// A new document, run again after a stop with its first answer: the table
/// rows above start from a published document; this one has no remote copy.
/// The branch gone before the child (RR7) of a promote that was `again`
/// ends as `context_gone` and leaves the earlier link row untouched, and a
/// finish with the context absent renews nothing (RR8).
#[test]
fn a_finish_or_an_end_without_a_context_leaves_the_link_row_alone() {
    rows(&[
        ("RR8 again: the finish renews no link row", |f| {
            let primary = f.primary();
            let mut io = PullRequestIo::default();
            let first = f.request();
            let result = go(f, &first, &mut io);
            finishes(f, &first, result, PromotionAnswer::Promoted)?;
            let row = f.link_row().ok_or("no link row after the first promote")?;
            save(f);
            let second = f.request();
            lost(f, &second, &mut io, later)?;
            remove_context(f);
            let result = run_at(f, second.clone(), &mut io, later);
            finishes(f, &second, result, PromotionAnswer::AlreadyPromoted)?;
            need!(f.link_row() == Some(row), "the link row changed");
            untouched(f, &[&io], primary)
        }),
        ("RR7 again: context_gone leaves the link row", |f| {
            let primary = f.primary();
            let mut io = PullRequestIo::default();
            let first = f.request();
            let result = go(f, &first, &mut io);
            finishes(f, &first, result, PromotionAnswer::Promoted)?;
            let row = f.link_row();
            let second = f.request();
            io.stop = Some((transport_stop(), false));
            stops_at_its_child(&run_at(f, second.clone(), &mut io, later), "the second run")?;
            remove_context(f);
            let result = run_at(f, second.clone(), &mut io, later);
            ends(f, &second, result, PromotionAnswer::ContextGone)?;
            need!(f.link_row() == row, "the link row changed");
            untouched(f, &[&io], primary)
        }),
    ]);
}

/// RR8's answer (design Part D): a `promoted` record that wrote no link row,
/// because its context was gone when it finished, still gives the link, which
/// its answer's data builds at answer time from the record's own remote and
/// primary branch as the record builds the one it writes: the same link, with
/// the context there and with it gone, and none for a record this build
/// cannot find.
#[test]
fn a_promote_without_a_link_row_still_gives_its_link() {
    fn built(
        f: &Fixture,
        req: &PromoteDocumentRequest,
    ) -> Option<(String, String, Option<String>, Option<String>)> {
        let repository_id = f.stored(req.operation_id).repository_id;
        answer::withheld_link(&f.db(), repository_id, f.root.path(), req.operation_id).map(|link| {
            (
                link.source,
                link.target,
                link.forge.map(|forge| forge.as_str().to_owned()),
                link.url,
            )
        })
    }
    rows(&[
        ("the context there: the link the record wrote", |f| {
            let mut io = PullRequestIo::default();
            let req = f.request();
            let result = go(f, &req, &mut io);
            finishes(f, &req, result, PromotionAnswer::Promoted)?;
            let (source, target, forge, url, _) =
                f.link_row().ok_or("no link row after the first promote")?;
            need!(
                built(f, &req) == Some((source, target, forge, url)),
                "the link built at answer time is not the one written: {:?}",
                built(f, &req)
            );
            Ok(())
        }),
        ("the context gone: no row, and the same link", |f| {
            let written = {
                // What a record with its context writes, on a fixture of its
                // own: the link carries the document's branch and primary.
                let mut io = PullRequestIo::default();
                let req = f.request();
                lost(f, &req, &mut io, clock)?;
                remove_context(f);
                let result = run_at(f, req.clone(), &mut io, later);
                finishes(f, &req, result, PromotionAnswer::Promoted)?;
                need!(f.link_row().is_none(), "a link row was written");
                built(f, &req)
            };
            let (source, target, ..) = written.ok_or("no link was built")?;
            need!(
                source == format!("manyhands/document/{ITEM}") && target == "main",
                "the link is {source} into {target}"
            );
            Ok(())
        }),
        ("a record this build cannot find builds none", |f| {
            let mut io = PullRequestIo::default();
            let req = f.request();
            let result = go(f, &req, &mut io);
            finishes(f, &req, result, PromotionAnswer::Promoted)?;
            let repository_id = f.stored(req.operation_id).repository_id;
            need!(
                answer::withheld_link(&f.db(), repository_id, f.root.path(), OperationId::new())
                    .is_none(),
                "a link was built for no record"
            );
            Ok(())
        }),
    ]);
}

/// C12: a context child that stops (transport, key, host, push rejected,
/// busy, poll yielding, identity) answers as `item sync` answers the stop;
/// the record stays `confirmed`, a new request is `busy` and writes nothing,
/// and the same request continues (RR1), also after the child's own merge
/// moved the branch. C13: a conflict is retained, resolved through the
/// conflict verbs by the record's ID, and the same request continues.
#[test]
fn stops_of_the_context_child() {
    use crate::repository::transport::SshTransportErrorKind as K;
    fn stop_of(kind: K) -> SynchronizationError {
        match transport_stop() {
            SynchronizationError::Transport(mut error) => {
                error.kind = kind;
                SynchronizationError::Transport(error)
            }
            other => other,
        }
    }
    /// The first run stops as `stop` says (`after_merge`: after the child's
    /// own merge of the primary into the branch); a new request is refused
    /// busy; the same request goes on.
    fn stops_and_continues(
        f: &Fixture,
        stop: SynchronizationError,
        after_merge: bool,
    ) -> Result<(), String> {
        let primary = f.primary();
        let mut io = PullRequestIo::default();
        let req = f.request();
        let tip = f.tip();
        if after_merge {
            io.fetched = Some(f.primary_moves_on());
            io.drive = Some(tip);
            io.stop_after_drive = Some(stop);
        } else {
            io.stop = Some((stop, false));
        }
        stops_at_its_child(&go(f, &req, &mut io), "the first run")?;
        need!(
            (f.tip() != tip) == after_merge,
            "the branch moved: {}",
            f.tip() != tip
        );
        let r = f.stored(req.operation_id);
        need!(
            r.step == PromotionStep::Confirmed,
            "the record is {:?}",
            r.step
        );
        need!(r.context_oid.is_none(), "the record holds a context");
        // A new request is held off, and writes nothing.
        let other = PromoteDocumentRequest {
            operation_id: OperationId::new(),
            ..req.clone()
        };
        let mut other_io = PullRequestIo::default();
        let refused = go(f, &other, &mut other_io);
        need!(
            matches!(refused, Err(DocumentPromotionError::Busy)),
            "a new request: {refused:?}"
        );
        need!(other_io.children.is_empty(), "a new request ran a child");
        need!(f.promotion_rows() == 1, "{} records", f.promotion_rows());
        need!(f.link_rows() == 0, "a link was given");
        // The same request goes on.
        let result = go(f, &req, &mut io);
        finishes(f, &req, result, PromotionAnswer::Promoted)?;
        untouched(f, &[&io, &other_io], primary)
    }
    rows(&[
        ("C12 transport", |f| {
            stops_and_continues(f, stop_of(K::TransportUnavailable), false)
        }),
        ("C12 key", |f| {
            stops_and_continues(f, stop_of(K::NoSelectedKey), false)
        }),
        ("C12 host", |f| {
            stops_and_continues(f, stop_of(K::HostTrustChanged), false)
        }),
        ("C12 push rejected", |f| {
            stops_and_continues(f, SynchronizationError::PushRejected, false)
        }),
        ("C12 busy", |f| {
            stops_and_continues(f, SynchronizationError::Busy, false)
        }),
        ("C12 poll yielding", |f| {
            stops_and_continues(f, SynchronizationError::PollYielding, false)
        }),
        ("C12 identity required", |f| {
            stops_and_continues(
                f,
                SynchronizationError::IdentityRequired {
                    target: SynchronizationTarget::Context {
                        kind: AuthoringKind::Document,
                        item_id: ITEM.parse().unwrap(),
                    },
                    expected_configuration: ExpectedConfiguration::for_testing(),
                },
                false,
            )
        }),
        ("C12 transport after the child's own merge", |f| {
            stops_and_continues(f, stop_of(K::TransportUnavailable), true)
        }),
        ("C12 push rejected after the child's own merge", |f| {
            stops_and_continues(f, SynchronizationError::PushRejected, true)
        }),
        ("C12 identity required after the child's own merge", |f| {
            stops_and_continues(
                f,
                SynchronizationError::IdentityRequired {
                    target: SynchronizationTarget::Context {
                        kind: AuthoringKind::Document,
                        item_id: ITEM.parse().unwrap(),
                    },
                    expected_configuration: ExpectedConfiguration::for_testing(),
                },
                true,
            )
        }),
        (
            "C12 a kept child's stop leaves its row, and the same request goes on",
            |f| {
                let primary = f.primary();
                let mut io = PullRequestIo::default();
                let req = stopped(f, &mut io, Some(Kept::Reserved))?;
                need!(active_children(f) == 1, "the stop kept no row");
                let other = PromoteDocumentRequest {
                    operation_id: OperationId::new(),
                    ..req.clone()
                };
                let mut other_io = PullRequestIo::default();
                let refused = go(f, &other, &mut other_io);
                need!(
                    matches!(refused, Err(DocumentPromotionError::Busy)),
                    "a new request: {refused:?}"
                );
                let result = go(f, &req, &mut io);
                finishes(f, &req, result, PromotionAnswer::Promoted)?;
                untouched(f, &[&io, &other_io], primary)
            },
        ),
        (
            "C13 a conflict is retained, resolved by the record's ID, and the same request goes on",
            |f| {
                let primary = f.primary();
                let mut io = PullRequestIo::default();
                let (req, child, local) = conflicted(f, &mut io)?;
                let r = f.stored(req.operation_id);
                need!(
                    r.step == PromotionStep::Confirmed,
                    "the record is {:?}",
                    r.step
                );
                need!(r.context_oid.is_none(), "the record holds a context");
                let other = PromoteDocumentRequest {
                    operation_id: OperationId::new(),
                    ..req.clone()
                };
                let mut other_io = PullRequestIo::default();
                let refused = go(f, &other, &mut other_io);
                need!(
                    matches!(refused, Err(DocumentPromotionError::Busy)),
                    "a new request: {refused:?}"
                );
                need!(other_io.children.is_empty(), "a new request ran a child");
                resolve(f, child, local);
                let result = go(f, &req, &mut io);
                finishes(f, &req, result, PromotionAnswer::Promoted)?;
                need!(
                    f.stored(req.operation_id).context_sync_id == Some(child),
                    "the resolved child is not the record's"
                );
                need!(f.link_rows() == 1, "no link was given");
                untouched(f, &[&io, &other_io], primary)
            },
        ),
    ]);
}

/// C14: a crash after the child completed and before the record finished,
/// then the same request: the child replays (no second push) and the first
/// table answers. C15: a crash after the record finished and before the
/// envelope was stored, then the same request: the answer is rebuilt from
/// `answer_code`, running nothing and writing no link row again.
#[test]
fn crash_and_replay() {
    rows(&[
        (
            "C14 a first promote lost before context_synchronized was saved",
            |f| completed_then(f, Lost::BeforeSaved, nothing, PromotionAnswer::Promoted, 1),
        ),
        (
            "C14 a first promote lost after context_synchronized was saved",
            |f| completed_then(f, Lost::AfterSaved, nothing, PromotionAnswer::Promoted, 1),
        ),
        (
            "C14 a promote again lost before context_synchronized was saved",
            |f| {
                let primary = f.primary();
                let mut io = PullRequestIo::default();
                let first = f.request();
                let result = go(f, &first, &mut io);
                finishes(f, &first, result, PromotionAnswer::Promoted)?;
                let row = f.link_row().ok_or("no link row")?;
                save(f);
                let second = f.request();
                lost(f, &second, &mut io, later)?;
                let mut again = PullRequestIo::default();
                let result = run_at(f, second.clone(), &mut again, later);
                finishes(f, &second, result, PromotionAnswer::AlreadyPromoted)?;
                need!(again.children.is_empty(), "the child ran a second time");
                let renewed = f.link_row().ok_or("the link row went")?;
                need!(
                    (&renewed.0, &renewed.1, &renewed.2, &renewed.3)
                        == (&row.0, &row.1, &row.2, &row.3)
                        && renewed.4 == CLOCK + 86_400,
                    "the link row is {renewed:?}"
                );
                untouched(f, &[&io, &again], primary)
            },
        ),
        (
            "C15 a finished record answers the same request from answer_code, whatever the link row is",
            |f| {
                let primary = f.primary();
                let mut io = PullRequestIo::default();
                let req = f.request();
                let first = answered(go(f, &req, &mut io))?;
                // The envelope was lost, and the link row went since.
                f.db()
                    .execute("DELETE FROM pull_request_links", [])
                    .unwrap();
                let reopened = RepositoryService::open_at(f.data.path()).unwrap();
                let mut again = PullRequestIo::default();
                let result = run::run_with_io(
                    &DocumentPromotionLifecycle,
                    &reopened,
                    request_at(req.clone(), later),
                    &mut again,
                );
                let second = answered(result)?;
                need!(second == first, "the answer changed: {second:?}");
                need!(again.children.is_empty(), "a child ran");
                need!(f.link_rows() == 0, "the link row was written again");
                untouched(f, &[&io, &again], primary)
            },
        ),
    ]);
}

/// C16: a new request while a record is unfinished: the prepare answers
/// `recovery_required` naming the holder's request, a confirm `busy`, and
/// nothing is written. C17: `request cancel` before or during the child
/// answers `cancelled` and leaves the record unfinished, and the same request
/// continues, replacing a cancelled child. C21: `operation list` and
/// `operation show` list the record as a `promote` with its request;
/// `operation resume` answers `original_request_required`; the child's
/// abandon is refused.
#[test]
fn holds_cancellation_and_listing() {
    rows(&[
        (
            "C16 a new request is held at prepare and busy at confirm",
            |f| {
                let primary = f.primary();
                let mut io = PullRequestIo::default();
                let req = stopped(f, &mut io, None)?;
                let prepared = f.service.prepare(&Mutation::DocumentPromote(
                    crate::repository::DocumentPromoteInput {
                        root: f.root.path().into(),
                        id: ITEM.parse().unwrap(),
                    },
                ));
                let value = serde_json::to_value(&prepared).unwrap();
                need!(
                    value["code"] == "recovery_required",
                    "the prepare answered {value}"
                );
                need!(
                    value["recovery"].as_array().is_some_and(|actions| actions
                        .iter()
                        .any(|action| action["operation_id"] == req.operation_id.to_string())),
                    "the prepare does not name the holder: {value}"
                );
                let other = PromoteDocumentRequest {
                    operation_id: OperationId::new(),
                    ..req.clone()
                };
                let mut other_io = PullRequestIo::default();
                let refused = go(f, &other, &mut other_io);
                need!(
                    matches!(refused, Err(DocumentPromotionError::Busy)),
                    "a new request: {refused:?}"
                );
                need!(other_io.children.is_empty(), "a new request ran a child");
                need!(f.promotion_rows() == 1, "{} records", f.promotion_rows());
                untouched(f, &[&io, &other_io], primary)
            },
        ),
        (
            "C17 cancelled before the child, the same request continues",
            |f| {
                let primary = f.primary();
                let req = f.request();
                cancel_at(f, req.operation_id, FailurePoint::AfterConfirmed);
                let mut io = PullRequestIo::default();
                let result = go(f, &req, &mut io);
                need!(
                    matches!(result, Err(DocumentPromotionError::Interrupted)),
                    "cancelled before the child: {result:?}"
                );
                need!(io.children.is_empty(), "the child ran");
                need!(
                    f.stored(req.operation_id).step == PromotionStep::Confirmed,
                    "the record is finished"
                );
                let result = go(f, &req, &mut io);
                finishes(f, &req, result, PromotionAnswer::Promoted)?;
                untouched(f, &[&io], primary)
            },
        ),
        (
            "C17 cancelled during the child, the same request replaces the child",
            |f| {
                let primary = f.primary();
                let req = f.request();
                let mut io = PullRequestIo {
                    cancel_in_child: Some(req.operation_id),
                    ..PullRequestIo::default()
                };
                let result = go(f, &req, &mut io);
                need!(
                    matches!(result, Err(DocumentPromotionError::Interrupted)),
                    "cancelled during the child: {result:?}"
                );
                let first = f
                    .stored(req.operation_id)
                    .context_sync_id
                    .ok_or("no child was minted")?;
                need!(
                    f.stored(req.operation_id).step == PromotionStep::Confirmed,
                    "the record is finished"
                );
                let result = go(f, &req, &mut io);
                finishes(f, &req, result, PromotionAnswer::Promoted)?;
                need!(
                    f.stored(req.operation_id).context_sync_id != Some(first),
                    "the cancelled child was not replaced"
                );
                untouched(f, &[&io], primary)
            },
        ),
        ("C21 operations list and abandon", |f| {
            use crate::repository::{OperationAction, OperationNextAction};
            let primary = f.primary();
            let mut io = PullRequestIo::default();
            let req = f.request();
            // The record's request is the one the registry holds for its
            // operation, as a request that was accepted leaves it.
            f.db()
                .execute_batch(&format!(
                    "INSERT INTO request_records
                         (request_ulid, attempt, scope_key, command, target, intent_digest, state, accepted_at)
                     VALUES ('01ARZ3NDEKTSV4RRFFQ69G5FAX', 1, '{}', 'document promote', 'document', '{}', 'accepted', 1);
                     INSERT INTO request_operations VALUES ('01ARZ3NDEKTSV4RRFFQ69G5FAX', 0, 'remote', '{}')",
                    f.root.path().to_str().unwrap(),
                    "0".repeat(64),
                    req.operation_id
                ))
                .unwrap();
            io.stop = Some((transport_stop(), true));
            stops_at_its_child(&go(f, &req, &mut io), "the first run")?;
            let child = f
                .stored(req.operation_id)
                .context_sync_id
                .ok_or("no child was minted")?;
            let repository = f.service.resolve_repository(f.root.path()).unwrap();
            let list = f.service.list_operations(&repository).unwrap();
            let of = |operation: OperationId| {
                list.items
                    .iter()
                    .filter(|row| {
                        row.operation_id.as_deref() == Some(operation.to_string().as_str())
                    })
                    .collect::<Vec<_>>()
            };
            let [parent] = of(req.operation_id)[..] else {
                return Err(format!("the record is not listed once: {list:?}"));
            };
            need!(
                parent.action == OperationAction::Promote,
                "listed as {:?}",
                parent.action
            );
            need!(parent.state == "confirmed", "listed as {}", parent.state);
            need!(
                parent.next_action == Some(OperationNextAction::Resume),
                "listed with {:?}",
                parent.next_action
            );
            let [row] = of(child)[..] else {
                return Err(format!("the child is not listed once: {list:?}"));
            };
            need!(
                row.action == OperationAction::SynchronizeContext && row.next_action.is_none(),
                "the child is listed as {:?}",
                row.action
            );
            // Its abandon is refused: the record is its owner.
            let abandonable = state::with_transaction(&f.service, f.root.path(), |tx, id| {
                let stored = state::read_operation(tx, id, child)?.unwrap();
                crate::repository::remote::reservation::abandonable(tx, &stored)
            })
            .unwrap();
            need!(!abandonable, "the child can be abandoned");
            untouched(f, &[&io], primary)
        }),
        (
            "C21 operation resume of a record another request holds",
            |f| {
                let primary = f.primary();
                let mut io = PullRequestIo::default();
                let req = f.request();
                // The record's request is the one the registry holds for its
                // operation, as a request that was accepted leaves it.
                f.db()
                .execute_batch(&format!(
                    "INSERT INTO request_records
                         (request_ulid, attempt, scope_key, command, target, intent_digest, state, accepted_at)
                     VALUES ('01ARZ3NDEKTSV4RRFFQ69G5FAX', 1, '{}', 'document promote', 'document', '{}', 'accepted', 1);
                     INSERT INTO request_operations VALUES ('01ARZ3NDEKTSV4RRFFQ69G5FAX', 0, 'remote', '{}')",
                    f.root.path().to_str().unwrap(),
                    "0".repeat(64),
                    req.operation_id
                ))
                .unwrap();
                io.stop = Some((transport_stop(), true));
                stops_at_its_child(&go(f, &req, &mut io), "the first run")?;
                let resumed = execute(
                    f,
                    Mutation::OperationResume(crate::repository::OperationResumeInput {
                        root: f.root.path().into(),
                        operation_id: req.operation_id,
                    }),
                );
                need!(
                    resumed["code"] == "original_request_required",
                    "resume answered {resumed}"
                );
                need!(
                    f.stored(req.operation_id).step == PromotionStep::Confirmed,
                    "resume moved the record"
                );
                untouched(f, &[&io], primary)
            },
        ),
    ]);
}

/// A mutation through the production boundary, as the envelope's JSON reads.
fn execute(f: &Fixture, mutation: Mutation) -> serde_json::Value {
    let envelope = f.service.execute(
        MutationCall {
            request_id: RequestId::new(),
            confirmation: None,
            mutation,
        },
        &mut SessionCredentials::new(NoCredentials),
        &mut NoProgress,
        &crate::repository::CancellationToken::new(),
    );
    serde_json::to_value(&envelope).unwrap()
}

fn give_link(f: &Fixture) {
    let link = crate::repository::closure::checkpoint::context_branch_link(
        &f.repo,
        "main",
        None,
        "origin",
        &format!("manyhands/document/{ITEM}"),
    );
    state::with_transaction(&f.service, f.root.path(), |tx, id| {
        crate::repository::pull_request_links::upsert_link(tx, id, "document", ITEM, &link, 1)
            .map_err(|_| state::recovery_required())
    })
    .unwrap();
}

/// C18: prepared in one mode, confirmed after a switch either way: in the
/// library a refused record of the confirm's mode. C19: between prepare and
/// confirm, a save, a fetch of another clone's publication, a link given,
/// (for `first`) a change of the forge setting or of the URL, or a fetched
/// merge: `external_change` as a refused record; after a fetched merge a new
/// prepare answers `already_applied` with no data. C23: `--confirm` with a
/// confirmation after a prepare that answered `already_applied` is
/// `confirmation_not_found`. And what is not a difference: a poll that
/// creates `R` at this clone's own push, and, for `again`, a change of the
/// forge setting.
#[test]
fn consent_differences() {
    /// A confirm of a request prepared before `change`: refused, as a record
    /// of `mode` (the confirm's), with the reason `external_change`.
    fn differs(
        f: &Fixture,
        mode: crate::canonical::Integration,
        change: fn(&Fixture),
    ) -> Result<(), String> {
        let primary = f.primary();
        let req = f.request();
        change(f);
        let (tracking, links) = (published(f), f.link_rows());
        let mut io = PullRequestIo::default();
        let result = go(f, &req, &mut io);
        refuses_in(f, &req, &mut io, result, tracking, links, mode)?;
        untouched(f, &[&io], primary)
    }
    fn refuses_in(
        f: &Fixture,
        req: &PromoteDocumentRequest,
        io: &mut PullRequestIo,
        result: Result<DocumentPromotionResult, DocumentPromotionError>,
        tracking: Option<Oid>,
        links: i64,
        mode: crate::canonical::Integration,
    ) -> Result<(), String> {
        refuses(f, req, io, result, 0, tracking, links)?;
        let r = f.stored(req.operation_id);
        need!(
            r.integration_mode == mode,
            "the record is {:?}",
            r.integration_mode
        );
        Ok(())
    }
    use crate::canonical::Integration;
    rows(&[
        (
            "C18 prepared direct, confirmed after a switch to pull-request",
            |_| {
                let f = Fixture::new();
                f.remote();
                let primary = f.primary();
                let req = f.request();
                f.set_config(PULL_REQUEST_CONFIG);
                let mut io = PullRequestIo::default();
                let tracking = published(&f);
                let result = go(&f, &req, &mut io);
                refuses_in(
                    &f,
                    &req,
                    &mut io,
                    result,
                    tracking,
                    0,
                    Integration::PullRequest,
                )?;
                untouched(&f, &[&io], primary)
            },
        ),
        (
            "C18 prepared pull-request, confirmed after a switch to direct",
            |f| differs(f, Integration::Direct, |f| f.set_config(DIRECT_CONFIG)),
        ),
        ("C19 a save", |f| differs(f, Integration::PullRequest, save)),
        ("C19 a fetch of another clone's publication", |f| {
            differs(f, Integration::PullRequest, |f| {
                let tip = f.tip();
                let other = f.commit_tree(tip, &[(EXTRA, b"another clone's save\n")], &[tip]);
                f.repo
                    .reference(&tracking_context_ref(), other, true, "fixture fetch")
                    .unwrap();
            })
        }),
        ("C19 a link given", |f| {
            differs(f, Integration::PullRequest, give_link)
        }),
        ("C19 first: a change of the forge setting", |f| {
            differs(f, Integration::PullRequest, |f| {
                f.set_config(&format!("{PULL_REQUEST_CONFIG}{FORGE_CONFIG}"))
            })
        }),
        ("C19 first: a change of the remote URL", |f| {
            differs(f, Integration::PullRequest, |f| {
                f.repo
                    .remote_set_url("origin", "ssh://git@github.com/acme/widgets.git")
                    .unwrap();
            })
        }),
        (
            "C19 a fetched merge: external_change, then a new prepare is already_applied",
            |f| {
                differs(f, Integration::PullRequest, merged)?;
                let held = f
                    .service
                    .preview_document_promotion(PreviewDocumentPromotionRequest {
                        root: f.root.path().into(),
                        item_id: ITEM.parse().unwrap(),
                    })
                    .unwrap();
                need!(
                    matches!(held, DocumentPromotionPreflight::PrimaryHolds),
                    "the prepare after the merge: {held:?}"
                );
                Ok(())
            },
        ),
        (
            "not a difference: a poll creating R at this clone's own push",
            |f| {
                let primary = f.primary();
                f.unpublish();
                let req = f.request();
                f.repo
                    .reference(&tracking_context_ref(), f.tip(), true, "fixture poll")
                    .unwrap();
                let mut io = PullRequestIo::default();
                let result = go(f, &req, &mut io);
                finishes(f, &req, result, PromotionAnswer::Promoted)?;
                untouched(f, &[&io], primary)
            },
        ),
        (
            "not a difference: again, a change of the forge setting after prepare",
            |f| {
                let primary = f.primary();
                let mut io = PullRequestIo::default();
                let first = f.request();
                let result = go(f, &first, &mut io);
                finishes(f, &first, result, PromotionAnswer::Promoted)?;
                let req = f.request();
                f.set_config(&format!("{PULL_REQUEST_CONFIG}{FORGE_CONFIG}"));
                let result = run_at(f, req.clone(), &mut io, later);
                finishes(f, &req, result, PromotionAnswer::AlreadyPromoted)?;
                untouched(f, &[&io], primary)
            },
        ),
        (
            "C23 a confirmation after a prepare that answered already_applied is not found",
            |f| {
                merged(f);
                let input = || crate::repository::DocumentPromoteInput {
                    root: f.root.path().into(),
                    id: ITEM.parse().unwrap(),
                };
                let prepared =
                    serde_json::to_value(f.service.prepare(&Mutation::DocumentPromote(input())))
                        .unwrap();
                need!(
                    prepared["code"] == "already_applied" && prepared["data"].is_null(),
                    "the prepare answered {prepared}"
                );
                let answer = f.service.execute(
                    MutationCall {
                        request_id: RequestId::new(),
                        confirmation: Some(
                            crate::repository::ConfirmationId::parse("01ARZ3NDEKTSV4RRFFQ69G5FN9")
                                .unwrap(),
                        ),
                        mutation: Mutation::DocumentPromote(input()),
                    },
                    &mut SessionCredentials::new(NoCredentials),
                    &mut NoProgress,
                    &crate::repository::CancellationToken::new(),
                );
                need!(
                    answer.code == crate::results::ResultCode::ConfirmationNotFound,
                    "the confirm answered {:?}",
                    answer.code
                );
                need!(f.promotion_rows() == 0, "a record was written");
                Ok(())
            },
        ),
    ]);
}

/// A retiring poll (`PollOnceRetiring`) after a fetch that advertised only
/// the primary, at `primary`. The document context's remote branch, which an
/// earlier fetch saw published at its tracking tip, is gone from the
/// advertisement. Returns the envelope.
fn retiring_poll(f: &Fixture, primary: Oid) -> serde_json::Value {
    use crate::repository::RemoteRefObservation;
    let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
    let configuration = f
        .service
        .observation_configuration(f.root.path(), &plan)
        .unwrap();
    let tip = f.repo.refname_to_id(&tracking_context_ref()).unwrap();
    state::with_transaction(&f.service, f.root.path(), |tx, id| {
        state::configure_endpoints(tx, id, &plan, &configuration.endpoint_digest())?;
        let prior = RemoteRefObservation::from_advertisement(
            &plan,
            plan.context(AuthoringKind::Document, &ITEM.parse().unwrap())
                .remote_ref(),
            tip,
            Some(tip),
        )
        .unwrap();
        let generation = state::generation(tx, id)?;
        state::persist_complete_advertisement(tx, id, &plan, generation, &[prior], 1)?;
        Ok(())
    })
    .unwrap();
    f.repo
        .reference("refs/remotes/origin/main", primary, true, "fixture fetch")
        .unwrap();
    crate::repository::remote::synthetic_fetch(vec![("refs/heads/main".into(), primary)]);
    execute(
        f,
        Mutation::PollOnceRetiring(crate::repository::PollOnceInput {
            root: f.root.path().into(),
        }),
    )
}

/// The forge merged the pull request by squash and deleted the branch: the
/// tracking primary and the local primary (fast-forwarded by the poll, as it
/// is clean) hold the document. Returns the primary.
fn forge_merged_and_deleted(f: &Fixture) -> Oid {
    let squash = f.squash(f.tip());
    f.set_tracking_main(squash);
    checkout(&f.repo, squash);
    squash
}

fn context_is_there(f: &Fixture) -> bool {
    f.repo.refname_to_id(&context_ref()).is_ok()
        && f.root
            .path()
            .join(".manyhands/worktrees")
            .join(ITEM)
            .exists()
}

/// C22: while a stopped record waits, the forge merges and deletes the
/// branch, and the poll's retirement or `context retire` runs: both leave
/// the context (F1, F2); the same request's child ends with
/// `remote_branch_deleted`, R-3 holds, `already_applied` with `primary_holds`;
/// the next poll retires the context. The control row runs the same poll
/// with no record at all, and retires.
#[test]
fn retirement_waits_for_a_stopped_promote() {
    // The control: with no record the poll retires the context, so that the
    // rows below fail for the record's sake and not the fixture's.
    {
        let f = Fixture::pull_request();
        f.set_config(PULL_REQUEST_CONFIG);
        commit(&f.repo);
        f.set_tracking_main(f.primary());
        let squash = forge_merged_and_deleted(&f);
        let value = retiring_poll(&f, squash);
        assert!(!context_is_there(&f), "the control did not retire: {value}");
    }
    rows(&[("C22 F1, F2 and the next poll", |f| {
        f.set_config(PULL_REQUEST_CONFIG);
        commit(&f.repo);
        f.set_tracking_main(f.primary());
        let mut io = PullRequestIo::default();
        let req = stopped(f, &mut io, None)?;
        let squash = forge_merged_and_deleted(f);
        let primary = f.primary();
        need!(primary == squash, "the local primary is not the squash");
        // F1: the poll sees the context promoted and leaves it.
        let value = retiring_poll(f, squash);
        let target = value["data"]["poll"]["targets"]
            .as_array()
            .and_then(|targets| targets.iter().find(|t| t["item_id"] == ITEM))
            .ok_or(format!(
                "the document context is not a poll target: {value}"
            ))?;
        need!(target["disposition"] == "promoted", "{value}");
        need!(context_is_there(f), "the poll retired the context: {value}");
        // F2: `context retire` is refused, naming the record's request.
        let refused = execute(
            f,
            Mutation::ContextRetire(crate::repository::ContextRetireInput {
                root: f.root.path().into(),
                kind: AuthoringKind::Document,
                id: ITEM.parse().unwrap(),
            }),
        );
        need!(
            refused["code"] == "recovery_required"
                && refused["data"]["blocker"]["operation_id"] == req.operation_id.to_string(),
            "context retire answered {refused}"
        );
        need!(context_is_there(f), "context retire retired the context");
        // The same request: the branch is gone on the remote, and the
        // primary holds the document.
        io.stop = Some((SynchronizationError::RemoteContextDeleted, true));
        let result = go(f, &req, &mut io);
        ends(f, &req, result, PromotionAnswer::PrimaryHolds)?;
        need!(f.link_rows() == 0, "a link was given");
        need!(f.primary() == primary, "the local primary moved");
        need!(
            io.primary_calls == 0 && io.deletes == 0,
            "a primary child or a delete"
        );
        // The next poll retires the context (K-14).
        let value = retiring_poll(f, squash);
        need!(
            !context_is_there(f),
            "the next poll left the context: {value}"
        );
        Ok(())
    })]);
}

/// F3: a retirement of the document claimed before the promotion record
/// began: the promotion's confirm or its child does not run under it. It
/// answers `busy`, or `recovery_required` naming the retirement, runs no
/// child, publishes nothing and gives no link. (The retirement is the
/// registry rows a claimed retirement is, as the retirement's own tests
/// write them; the confirm is the runner's, so this row sits with the
/// promote and not with the fence's table.)
#[test]
fn a_retirement_claimed_before_the_record_began_holds_the_promotion_off() {
    rows(&[("F3 a claimed retirement of the document", |f| {
        let primary = f.primary();
        // The consent is taken before the retirement claims the context.
        let req = f.request();
        let retirement = OperationId::new();
        let db = f.db();
        db.execute(
            "INSERT INTO operation_records(repository_id,root_path,operation_ulid,action,target,state,observed_at) SELECT id,root_path,?1,'retire_context',?2,'created',1 FROM repositories",
            rusqlite::params![
                retirement.to_string(),
                format!("authoring-context-v1/document/{ITEM}/Retire")
            ],
        )
        .unwrap();
        db.execute(
            "INSERT INTO context_retirement_records(operation_ulid,local_record_id,repository_id,format_version,kind,item_id,initial_resources,selection_epoch,instance_verified,materialization_capture,materialization_settled,owner_epoch,phase,worktree_removal,branch_removal,absence_basis) SELECT operation_ulid,id,repository_id,1,'document',?2,'absent',1,0,'absent',0,1,'selected','none','none','none' FROM operation_records WHERE operation_ulid=?1",
            rusqlite::params![retirement.to_string(), ITEM],
        )
        .unwrap();
        db.execute(
            "INSERT INTO context_retirement_attempts(operation_ulid,owner_epoch,repository_id,phase) SELECT operation_ulid,1,repository_id,'active' FROM context_retirement_records WHERE operation_ulid=?1",
            [retirement.to_string()],
        )
        .unwrap();
        let tracking = published(f);
        let mut io = PullRequestIo::default();
        let result = go(f, &req, &mut io);
        let names_retirement = |error: &RepositoryError| {
            matches!(
                error.kind,
                RepositoryErrorKind::RepositoryBusy | RepositoryErrorKind::RecoveryRequired
            ) && error.blocker().map(|blocker| blocker.operation_id) == Some(retirement)
        };
        let held = match &result {
            Err(DocumentPromotionError::Busy) => true,
            Err(DocumentPromotionError::Repository(error)) => names_retirement(error),
            Err(DocumentPromotionError::ContextSynchronization(stop)) => {
                matches!(&**stop, SynchronizationError::Repository(error) if names_retirement(error))
            }
            _ => false,
        };
        need!(
            held,
            "the promotion was not held off by the retirement: {result:?}"
        );
        let rows: i64 = f
            .db()
            .query_row(
                "SELECT count(*) FROM remote_operation_records WHERE action='synchronize_context'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        need!(
            rows == 0,
            "{rows} child rows were reserved under the retirement"
        );
        need!(published(f) == tracking, "the branch was published");
        need!(f.link_rows() == 0, "a link was given");
        untouched(f, &[&io], primary)
    })]);
}
