//! `conflict resolve`: the design's "Required outcomes: interrupted
//! resolution" table, one `c4_resolution_` case per row; Review Focus 3;
//! the binding's validation, in step 4's order; its data and goldens; its
//! row of the conformance table; and the review's first fix round:
//! `request cancel` of the request that holds the synchronization (step 6),
//! an attempt ID that collides (R-14), a refused supersede, an edited path
//! whose write was not observed, and a synchronization with no conflict;
//! and the Part A review's fix round: a refusal that wrote nothing closes
//! its attempt, the original request after a hand edit, a supersede whose
//! new request is refused, and a continuation that meets a deleted branch.
//!
//! A raw peer clone makes the remote diverge, as a second user would, and
//! a synchronization stops on the conflict. The resolution is presented as
//! `conflict show` gives it. It is interrupted by every library failure
//! point around resolution (`sync.rs`; `state.rs` has none):
//! `ResolutionAfterPathWrite`, `ResolutionAfterCandidatePrepared`,
//! `ResolutionAfterRefTransition`, `ResolutionAfterCheckpointObservation`,
//! `ResolutionBeforeMetadataRetirement`,
//! `ResolutionAfterIndexLockRetirement` and
//! `ResolutionAfterMetadataCleanup`; before the first path write, where no
//! failure point exists (fact 64), by an SQL trigger that refuses the first
//! resolution path's insert; before publication by the test server's
//! disconnect at its advertisement; and during publication by its withheld
//! push status. Each retry runs in a fresh
//! `RepositoryService` opened on the same data directory, so no memory of
//! the first call carries over (decision 13).

use std::path::{Path, PathBuf};

use results::{CheckpointEffect, IntegrationEffect, WriteEffect};

use super::*;

pub(crate) const CASES: &[ssh_harness::Case] = &[
    (
        "c4_resolution_before_any_path_write",
        c4_resolution_before_any_path_write,
    ),
    (
        "c4_resolution_after_paths_written",
        c4_resolution_after_paths_written,
    ),
    (
        "c4_resolution_after_commit_prepared",
        c4_resolution_after_commit_prepared,
    ),
    (
        "c4_resolution_after_ref_moved",
        c4_resolution_after_ref_moved,
    ),
    (
        "c4_resolution_after_checkpoint",
        c4_resolution_after_checkpoint,
    ),
    (
        "c4_resolution_during_publication",
        c4_resolution_during_publication,
    ),
    (
        "c4_resolution_another_source_is_a_mismatch",
        c4_resolution_another_source_is_a_mismatch,
    ),
    (
        "c4_resolution_new_request_supersedes_written_paths",
        c4_resolution_new_request_supersedes_written_paths,
    ),
    (
        "c4_resolution_new_request_after_a_written_path_is_edited",
        c4_resolution_new_request_after_a_written_path_is_edited,
    ),
    (
        "c4_resolution_resume_finishes_a_prepared_commit",
        c4_resolution_resume_finishes_a_prepared_commit,
    ),
    (
        "c4_resolution_new_request_after_commit_prepared",
        c4_resolution_new_request_after_commit_prepared,
    ),
    (
        "c4_resolution_configuration_changed",
        c4_resolution_configuration_changed,
    ),
    (
        "c4_resolution_published_by_another_request",
        c4_resolution_published_by_another_request,
    ),
    (
        "c4_resolution_new_request_after_success",
        c4_resolution_new_request_after_success,
    ),
    (
        "c4_resolution_unfinished_local_sync",
        c4_resolution_unfinished_local_sync,
    ),
    (
        "c4_resolution_resume_before_publication",
        c4_resolution_resume_before_publication,
    ),
    (
        "c4_resolution_resume_after_publication",
        c4_resolution_resume_after_publication,
    ),
    (
        "c4_resolution_cancelled_before_publication",
        c4_resolution_cancelled_before_publication,
    ),
    (
        "c4_resolution_cancelled_during_publication",
        c4_resolution_cancelled_during_publication,
    ),
    (
        "c4_resolution_duplicate_token",
        c4_resolution_duplicate_token,
    ),
    ("c4_resolution_foreign_token", c4_resolution_foreign_token),
    ("c4_resolution_publishes", c4_resolution_publishes),
    ("c4_resolution_syntax", c4_resolution_syntax),
    (
        "c4_resolution_another_observation",
        c4_resolution_another_observation,
    ),
    ("c4_resolution_external_only", c4_resolution_external_only),
    ("c4_resolution_wrong_count", c4_resolution_wrong_count),
    (
        "c4_resolution_operation_lookup",
        c4_resolution_operation_lookup,
    ),
    (
        "c4_resolution_comment_conflict",
        c4_resolution_comment_conflict,
    ),
    (
        "c4_resolution_local_sync_operation",
        c4_resolution_local_sync_operation,
    ),
    ("c4_resolution_conformance", c4_resolution_conformance),
    (
        "c4_resolution_original_request_cancels_before_push_intent",
        c4_resolution_original_request_cancels_before_push_intent,
    ),
    (
        "c4_resolution_original_request_cancels_after_push_intent",
        c4_resolution_original_request_cancels_after_push_intent,
    ),
    (
        "c4_resolution_comment_request_cancels",
        c4_resolution_comment_request_cancels,
    ),
    (
        "c4_resolution_attempt_id_collision",
        c4_resolution_attempt_id_collision,
    ),
    (
        "c4_resolution_refused_supersede_releases_the_request",
        c4_resolution_refused_supersede_releases_the_request,
    ),
    (
        "c4_resolution_unobserved_path_edited",
        c4_resolution_unobserved_path_edited,
    ),
    (
        "c4_resolution_synchronization_without_conflict",
        c4_resolution_synchronization_without_conflict,
    ),
    (
        "c4_resolution_refused_at_a_git_lock",
        c4_resolution_refused_at_a_git_lock,
    ),
    (
        "c4_resolution_original_request_after_an_edit",
        c4_resolution_original_request_after_an_edit,
    ),
    (
        "c4_resolution_superseded_then_refused",
        c4_resolution_superseded_then_refused,
    ),
    (
        "c4_resolution_continuation_meets_a_deleted_branch",
        c4_resolution_continuation_meets_a_deleted_branch,
    ),
];

const MAIN: &str = "refs/heads/main";
const SCHEMA: &str = "conflict_resolution_mutation.schema.json";
/// The request that publishes the item's editing branch, or brings primary
/// up to the server's, before a case diverges.
const SETUP: &str = "01ARZ3NDEKTSV4RRFFQ69G5FY0";
/// The synchronization that stops on the conflict.
const SYNC: &str = "01ARZ3NDEKTSV4RRFFQ69G5FY2";
/// The request that resolves it.
const RESOLVE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR1";
/// Another resolution request.
const OTHER: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR2";
/// `operation resume`.
const RESUME: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR3";
/// A new synchronization after the conflict.
const LATER: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR4";
/// The comment of the comment case.
const COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC1";
/// The second ticket of the conflict with two paths.
const SECOND_TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAY";
/// A third resolution request.
const THIRD: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR5";
/// A source the stores and the envelopes must never hold.
const PRIVATE: &str = "conflict-resolve-private-source-sentinel";
/// A well-formed token that no conflict issued.
const NO_SUCH_TOKEN: &str =
    "conflict-path-v1:0000000000000000000000000000000000000000000000000000000000000000";
/// A well-formed observation that no conflict has.
const NO_SUCH_OBSERVATION: &str =
    "conflict-v1:0000000000000000000000000000000000000000000000000000000000000000";
/// What the resolution commit's subject begins with (`sync.rs`).
const RESOLUTION_SUBJECT: &str = "Resolve synchronization";

/// A world whose synchronization stopped on a conflict in a ticket's
/// canonical source, with all three sides, and what `conflict show` gave
/// of it.
struct Conflict {
    w: World,
    /// The synchronization.
    operation: OperationId,
    /// The branch it synchronizes.
    reference: &'static str,
    /// The worktree that branch is checked out in.
    worktree: PathBuf,
    /// The ticket the conflict is in.
    ticket: &'static str,
    local: git2::Oid,
    incoming: git2::Oid,
    observation: String,
    tokens: Vec<String>,
}

/// A world with a trusted server.
pub(super) fn world() -> Result<World, FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    Ok(w)
}

/// The peer commits `bytes` at `path` on `reference` as the server holds
/// it, and pushes it there. What the server holds is fetched to a ref of
/// the fixture's own first, so the peer's checked-out branch is never
/// fetched into.
pub(super) fn remote_commit(
    w: &World,
    reference: &str,
    path: &str,
    bytes: &[u8],
) -> Result<git2::Oid, FixtureError> {
    let served = format!("refs/fixture/{}", reference.trim_start_matches("refs/"));
    let mut remote = fixed(w.peer.remote_anonymous(&w.server.url()))?;
    let mut options = git2::FetchOptions::new();
    options.remote_callbacks(callbacks(&w.server));
    fixed(remote.fetch(
        &[&format!("+{reference}:{served}")],
        Some(&mut options),
        None,
    ))?;
    let parent = fixed(w.peer.refname_to_id(&served))?;
    let new = commit_path(&w.peer, parent, path, bytes)?;
    fixed(w.peer.reference(reference, new, true, "fixture"))?;
    push_peer(&w.peer, &w.server, reference)?;
    Ok(new)
}

impl Conflict {
    /// A conflict on primary, which `repo sync` stops on.
    fn primary() -> Result<Self, FixtureError> {
        Self::primary_in(world()?)
    }

    fn primary_in(w: World) -> Result<Self, FixtureError> {
        let ticket = merging::PRIMARY_TICKET;
        let path = merging::ticket_path(ticket);
        // Primary holds no ticket of its own yet: the peer adds one, and
        // primary is brought up to it.
        remote_commit(
            &w,
            MAIN,
            &path,
            merging::ticket_source(ticket, "Base").as_bytes(),
        )?;
        assert_current(&execute(&w, SETUP, repo_sync(&w)));
        let worktree = w.root.clone();
        Self::stopped(w, SYNC, MAIN, worktree, ticket, repo_sync)
    }

    /// A conflict in the world's item's editing branch, which `item sync`
    /// stops on. `before` runs once the branch is published, before the
    /// two sides diverge.
    fn item(before: impl FnOnce(&World) -> Result<(), FixtureError>) -> Result<Self, FixtureError> {
        let mut w = world()?;
        let context = w.context()?;
        assert_eq!(execute(&w, SETUP, item_sync(&w)).code, ResultCode::Ok);
        before(&w)?;
        Self::stopped(w, SYNC, CONTEXT, context.worktree, ITEM, item_sync)
    }

    /// Each side gives `ticket` another title, and `mutation`, under the
    /// request `sync`, stops on it.
    fn stopped(
        w: World,
        sync: &str,
        reference: &'static str,
        worktree: PathBuf,
        ticket: &'static str,
        mutation: fn(&World) -> Mutation,
    ) -> Result<Self, FixtureError> {
        let path = merging::ticket_path(ticket);
        let local = commit_in(
            &worktree,
            reference,
            &path,
            merging::ticket_source(ticket, "Local").as_bytes(),
        )?;
        let incoming = remote_commit(
            &w,
            reference,
            &path,
            merging::ticket_source(ticket, "Incoming").as_bytes(),
        )?;
        let stopped = execute(&w, sync, mutation(&w));
        assert_eq!(stopped.code, ResultCode::RecoveryRequired);
        let operation = OperationId::parse(stopped.operation_id.as_deref().ok_or(FixtureError)?)
            .map_err(|_| FixtureError)?;
        let mut conflict = Self {
            w,
            operation,
            reference,
            worktree,
            ticket,
            local,
            incoming,
            observation: String::new(),
            tokens: Vec::new(),
        };
        let shown = conflict.show();
        assert_eq!(shown.state, ConflictState::ConflictPending);
        conflict.observation = shown.observation.ok_or(FixtureError)?;
        conflict.tokens = shown.paths.into_iter().map(|path| path.token).collect();
        Ok(conflict)
    }

    /// What `conflict show` reads of the conflict now.
    fn show(&self) -> ConflictDto {
        let repo = self.w.service.resolve_repository(&self.w.root).unwrap();
        self.w.service.show_conflict(&repo, self.operation).unwrap()
    }

    fn path(&self) -> String {
        merging::ticket_path(self.ticket)
    }

    /// The ticket's source, titled `title`.
    fn source(&self, title: &str) -> String {
        merging::ticket_source(self.ticket, title)
    }

    /// `conflict resolve` of the conflict's paths with `sources`, in the
    /// order `conflict show` gave the paths, with the observation it gave.
    fn with(&self, sources: &[String]) -> Mutation {
        assert_eq!(sources.len(), self.tokens.len());
        resolve(
            &self.w,
            self.operation,
            &self.observation,
            self.tokens.iter().cloned().zip(sources.iter().cloned()),
        )
    }

    /// `conflict resolve` of the one path with the source titled `title`.
    fn titled(&self, title: &str) -> Mutation {
        self.with(&[self.source(title)])
    }

    /// One call of `conflict resolve` on `service`, checked as every call is.
    fn on(
        &self,
        service: &RepositoryService,
        id: &str,
        mutation: Mutation,
    ) -> Envelope<MutationDataDto> {
        resolved_on(&self.w, service, id, mutation)
    }

    fn execute(&self, id: &str, mutation: Mutation) -> Envelope<MutationDataDto> {
        self.on(&self.w.service, id, mutation)
    }

    /// One call on a service that fails at `point`.
    fn interrupted_at(
        &self,
        point: FailurePoint,
        id: &str,
        title: &str,
    ) -> Envelope<MutationDataDto> {
        let failing =
            RepositoryService::open_at_with_failure_point_for_testing(&self.w.data(), point)
                .unwrap();
        let stopped = self.on(&failing, id, self.titled(title));
        assert!(stopped.code.failure_class().is_some(), "{point:?}");
        // The request has no operation of its own: settling deletes its
        // record on any stop (fact 67).
        assert!(record(&self.w, id).is_none(), "{point:?}");
        stopped
    }

    fn tip(&self) -> git2::Oid {
        self.w
            .repo()
            .unwrap()
            .refname_to_id(self.reference)
            .unwrap()
    }

    fn served(&self) -> git2::Oid {
        self.w
            .bare()
            .unwrap()
            .refname_to_id(self.reference)
            .unwrap()
    }

    /// The resolution commits on the branch, newest first.
    fn resolutions(&self) -> Vec<git2::Oid> {
        resolutions_at(&self.w, self.tip())
    }

    /// What the ticket's file in the worktree holds.
    fn written(&self) -> String {
        std::fs::read_to_string(self.worktree.join(self.path())).unwrap()
    }

    /// The phase of the integration step whose conflict this is.
    fn step(&self) -> String {
        conflict_step(&self.w, self.operation)
    }

    /// Asserts that `envelope` is the request's result once the conflict is
    /// resolved and published: one resolution commit, of the conflict's
    /// local and incoming parents, which the server holds, pushed once
    /// since `pushes`. Returns the commit.
    fn assert_published(&self, envelope: &Envelope<MutationDataDto>, pushes: usize) -> git2::Oid {
        assert_eq!(envelope.code, ResultCode::Ok, "{envelope:?}");
        assert_eq!(envelope.outcome, Outcome::Success);
        let resolutions = self.resolutions();
        assert_eq!(resolutions.len(), 1);
        let commit = resolutions[0];
        let parents: Vec<_> = self
            .w
            .repo()
            .unwrap()
            .find_commit(commit)
            .unwrap()
            .parent_ids()
            .collect();
        assert_eq!(parents, [self.local, self.incoming]);
        assert_eq!(envelope.effects, published(commit));
        assert_eq!(
            envelope.data,
            Some(data_of(
                self.operation,
                Some(commit),
                PublicationEffect::Published,
                None
            ))
        );
        assert!(envelope.recovery.is_empty());
        assert_eq!(envelope.operation_id, None);
        assert_eq!(self.served(), commit);
        assert_eq!(accepted(&self.w.server), pushes + 1);
        assert_eq!(self.show().state, ConflictState::Complete);
        commit
    }
}

/// `conflict resolve` of `operation` with `observation` and `resolutions`.
fn resolve(
    w: &World,
    operation: OperationId,
    observation: &str,
    resolutions: impl IntoIterator<Item = (String, String)>,
) -> Mutation {
    Mutation::ConflictResolve(ConflictResolveInput {
        root: w.root.clone(),
        operation_id: operation,
        observation: observation.to_owned(),
        resolutions: resolutions
            .into_iter()
            .map(|(token, source)| ConflictResolutionInput { token, source })
            .collect(),
    })
}

/// One call of `conflict resolve` on `service`, with the world's
/// passphrase, no sink and no cancellation: scanned as `call` scans, and
/// its data checked against the published schema.
fn resolved_on(
    w: &World,
    service: &RepositoryService,
    id: &str,
    mutation: Mutation,
) -> Envelope<MutationDataDto> {
    checked(call(
        w,
        service,
        id,
        mutation,
        Provider,
        &mut NoProgress,
        &CancellationToken::new(),
    ))
}

fn checked(envelope: Envelope<MutationDataDto>) -> Envelope<MutationDataDto> {
    assert_eq!(envelope.command, "conflict resolve");
    let value = serde_json::to_value(&envelope).expect("an envelope serializes");
    if !value["data"].is_null() {
        schema::check_published(SCHEMA, &value["data"]).expect("the resolution data schema");
    }
    envelope
}

/// The resolution commits that `tip` holds, newest first.
pub(super) fn resolutions_at(w: &World, tip: git2::Oid) -> Vec<git2::Oid> {
    let repo = w.repo().unwrap();
    let mut walk = repo.revwalk().unwrap();
    walk.push(tip).unwrap();
    walk.map(Result::unwrap)
        .filter(|oid| {
            repo.find_commit(*oid)
                .unwrap()
                .summary()
                .is_some_and(|summary| summary.starts_with(RESOLUTION_SUBJECT))
        })
        .collect()
}

/// The phase of the newest integration step of the synchronization
/// `operation` that met a conflict.
pub(super) fn conflict_step(w: &World, operation: OperationId) -> String {
    fixed(w.db().unwrap().query_row(
        "SELECT step.phase FROM remote_integration_steps step
           JOIN remote_operation_records operation ON operation.id = step.operation_record_id
          WHERE operation.operation_ulid = ?1 AND step.conflict_digest IS NOT NULL
          ORDER BY step.window_number DESC, step.ordinal DESC LIMIT 1",
        [operation.to_string()],
        |row| row.get(0),
    ))
    .unwrap()
}

/// A fresh service on the world's data directory.
pub(super) fn fresh(w: &World) -> RepositoryService {
    RepositoryService::open_at(&w.data()).unwrap()
}

/// What a resolution checkpointed and published reports.
fn published(commit: git2::Oid) -> Effects {
    Effects {
        write: WriteEffect::Written,
        checkpoint: CheckpointEffect::Committed,
        integration: IntegrationEffect::Complete,
        publication: PublicationEffect::Published,
        discovery: DiscoveryEffect::Current,
        commit_oid: Some(commit.to_string()),
        ..Effects::not_requested()
    }
}

/// What a resolution checkpointed and not yet published reports.
fn resolved_only(commit: git2::Oid) -> Effects {
    Effects {
        publication: PublicationEffect::Pending,
        discovery: DiscoveryEffect::Pending,
        ..published(commit)
    }
}

fn data_of(
    operation: OperationId,
    commit: Option<git2::Oid>,
    publication: PublicationEffect,
    conflict: Option<ConflictNextDto>,
) -> MutationDataDto {
    MutationDataDto::ConflictResolution(ConflictResolutionMutationDto {
        r#type: ConflictResolutionMutationType::ConflictResolution,
        operation_id: operation.to_string(),
        commit_oid: commit.map(|commit| commit.to_string()),
        publication,
        conflict,
    })
}

/// How many resolution attempts, and records of their owners, the
/// registry holds.
pub(super) fn attempts(w: &World) -> (i64, i64) {
    let db = w.db().unwrap();
    let count = |table: &str| -> i64 {
        db.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap()
    };
    (
        count("remote_resolution_attempts"),
        count("remote_resolution_owners"),
    )
}

/// The candidate commit the one attempt recorded.
pub(super) fn candidate(w: &World) -> git2::Oid {
    let candidate: String = w
        .db()
        .unwrap()
        .query_row(
            "SELECT candidate_oid FROM remote_resolution_attempts",
            [],
            |row| row.get(0),
        )
        .unwrap();
    git2::Oid::from_str(&candidate).unwrap()
}

/// Checks `envelope` against its golden, with the synchronization's ID
/// given a placeholder of its own.
fn assert_resolution_golden(
    w: &World,
    name: &str,
    operation: OperationId,
    envelope: &Envelope<MutationDataDto>,
) {
    let value = serde_json::to_value(envelope).unwrap();
    let operation = operation.to_string();
    let root = w.root.canonicalize().unwrap();
    let data_schema = (!value["data"].is_null()).then_some(SCHEMA);
    mutation_golden::assert_contract_at(
        name,
        root.to_str().unwrap(),
        data_schema,
        &[(operation.as_str(), "<synchronization>")],
        &value,
    );
}

/// Asserts that `envelope` refused the request with `code` having written
/// nothing: no effect, no record, and the conflict, the branch and the
/// registry's attempts as they were.
fn assert_nothing_written(
    c: &Conflict,
    id: &str,
    envelope: &Envelope<MutationDataDto>,
    code: ResultCode,
    before: (git2::Oid, String, String, (i64, i64)),
) {
    assert_eq!(envelope.code, code, "{envelope:?}");
    assert_eq!(envelope.effects, Effects::not_requested());
    assert!(record(&c.w, id).is_none());
    assert_eq!((c.tip(), c.written(), c.step(), attempts(&c.w)), before);
}

/// What `assert_nothing_written` compares.
fn state_of(c: &Conflict) -> (git2::Oid, String, String, (i64, i64)) {
    (c.tip(), c.written(), c.step(), attempts(&c.w))
}

/// Has the registry refuse the first resolution path's insert: the one
/// interruption no failure point gives (fact 64).
const REFUSE_PATHS: &str = "CREATE TRIGGER c4_refuse_resolution_path BEFORE INSERT ON remote_resolution_paths BEGIN SELECT RAISE(ABORT, 'refused by the case'); END;";
const ALLOW_PATHS: &str = "DROP TRIGGER c4_refuse_resolution_path;";

fn c4_resolution_before_any_path_write() -> Result<(), FixtureError> {
    let c = Conflict::primary()?;
    let pushes = accepted(&c.w.server);
    fixed(c.w.db()?.execute_batch(REFUSE_PATHS))?;
    let stopped = c.execute(RESOLVE, c.titled("Resolved"));
    assert_eq!(stopped.code, ResultCode::RecoveryRequired);
    assert_eq!(stopped.effects, Effects::not_requested());
    assert!(record(&c.w, RESOLVE).is_none());
    // No attempt was recorded and no path written; the request is recorded
    // as the owner of the attempt it derived.
    assert_eq!(attempts(&c.w), (0, 1));
    assert_eq!(c.step(), "conflict_pending");
    assert!(c.resolutions().is_empty());
    fixed(c.w.db()?.execute_batch(ALLOW_PATHS))?;
    let retried = c.on(&fresh(&c.w), RESOLVE, c.titled("Resolved"));
    c.assert_published(&retried, pushes);
    assert_eq!(c.written(), c.source("Resolved"));
    Ok(())
}

fn c4_resolution_after_paths_written() -> Result<(), FixtureError> {
    let c = Conflict::primary()?;
    let pushes = accepted(&c.w.server);
    let stopped = c.interrupted_at(FailurePoint::ResolutionAfterPathWrite, RESOLVE, "Resolved");
    assert_eq!(stopped.outcome, Outcome::Partial);
    assert_eq!(c.step(), "resolution_prepared");
    assert_eq!(c.written(), c.source("Resolved"));
    // `conflict show` names the request whose attempt it is (R-35).
    assert_eq!(c.show().request_id.as_deref(), Some(RESOLVE));
    let retried = c.on(&fresh(&c.w), RESOLVE, c.titled("Resolved"));
    c.assert_published(&retried, pushes);
    assert_eq!(attempts(&c.w), (1, 1));
    Ok(())
}

fn c4_resolution_after_commit_prepared() -> Result<(), FixtureError> {
    let c = Conflict::primary()?;
    let pushes = accepted(&c.w.server);
    c.interrupted_at(
        FailurePoint::ResolutionAfterCandidatePrepared,
        RESOLVE,
        "Resolved",
    );
    assert_eq!(c.step(), "commit_prepared");
    let prepared = candidate(&c.w);
    // The ref has not moved.
    assert_eq!(c.tip(), c.local);
    let retried = c.on(&fresh(&c.w), RESOLVE, c.titled("Resolved"));
    assert_eq!(c.assert_published(&retried, pushes), prepared);
    Ok(())
}

fn c4_resolution_after_ref_moved() -> Result<(), FixtureError> {
    for point in [
        FailurePoint::ResolutionAfterRefTransition,
        FailurePoint::ResolutionAfterCheckpointObservation,
    ] {
        let c = Conflict::primary()?;
        let pushes = accepted(&c.w.server);
        c.interrupted_at(point, RESOLVE, "Resolved");
        let prepared = candidate(&c.w);
        assert_eq!(c.tip(), prepared, "{point:?}");
        assert_eq!(c.served(), c.incoming, "{point:?}");
        let retried = c.on(&fresh(&c.w), RESOLVE, c.titled("Resolved"));
        assert_eq!(c.assert_published(&retried, pushes), prepared, "{point:?}");
    }
    Ok(())
}

/// Row 4 again, at the failure points after the checkpoint is observed:
/// before the merge metadata is retired, after the index sentinel is
/// released, and after the metadata is cleaned up, before the attempt is
/// finalized.
fn c4_resolution_after_checkpoint() -> Result<(), FixtureError> {
    for point in [
        FailurePoint::ResolutionBeforeMetadataRetirement,
        FailurePoint::ResolutionAfterIndexLockRetirement,
        FailurePoint::ResolutionAfterMetadataCleanup,
    ] {
        let c = Conflict::primary()?;
        let pushes = accepted(&c.w.server);
        c.interrupted_at(point, RESOLVE, "Resolved");
        let prepared = candidate(&c.w);
        assert_eq!(c.tip(), prepared, "{point:?}");
        assert_eq!(c.step(), "applied", "{point:?}");
        let retried = c.on(&fresh(&c.w), RESOLVE, c.titled("Resolved"));
        assert_eq!(c.assert_published(&retried, pushes), prepared, "{point:?}");
    }
    Ok(())
}

fn c4_resolution_during_publication() -> Result<(), FixtureError> {
    let c = Conflict::primary()?;
    let pushes = accepted(&c.w.server);
    c.w.server.disconnect_at(FixtureBoundary::AfterReceivePack);
    let lost = c.execute(RESOLVE, c.titled("Resolved"));
    // The server took the push and the client never learned of it.
    assert!(c.w.server.receive_status_withheld());
    assert_eq!(lost.code, ResultCode::TransportUnavailable);
    assert_eq!(lost.outcome, Outcome::Partial);
    let commit = c.resolutions()[0];
    assert_eq!(lost.effects, resolved_only(commit));
    assert_eq!(accepted(&c.w.server), pushes + 1);
    assert!(record(&c.w, RESOLVE).is_none());
    c.w.server.clear_fault();
    // Published once: the same request reconciles the push it lost.
    let retried = c.on(&fresh(&c.w), RESOLVE, c.titled("Resolved"));
    assert_eq!(retried.code, ResultCode::Ok);
    assert_eq!(retried.effects, published(commit));
    assert_eq!(c.served(), commit);
    assert_eq!(accepted(&c.w.server), pushes + 1);
    assert_eq!(c.resolutions(), [commit]);
    Ok(())
}

/// How a case interrupts the first request.
#[derive(Clone, Copy, Debug)]
enum Interruption {
    BeforeAnyPathWrite,
    At(FailurePoint),
}

impl Interruption {
    fn interrupt(self, c: &Conflict, id: &str, title: &str) {
        match self {
            Self::BeforeAnyPathWrite => {
                c.w.db().unwrap().execute_batch(REFUSE_PATHS).unwrap();
                let stopped = c.execute(id, c.titled(title));
                assert_eq!(stopped.code, ResultCode::RecoveryRequired);
                c.w.db().unwrap().execute_batch(ALLOW_PATHS).unwrap();
            }
            Self::At(point) => {
                c.interrupted_at(point, id, title);
            }
        }
    }
}

fn c4_resolution_another_source_is_a_mismatch() -> Result<(), FixtureError> {
    for interruption in [
        Interruption::BeforeAnyPathWrite,
        Interruption::At(FailurePoint::ResolutionAfterPathWrite),
        Interruption::At(FailurePoint::ResolutionAfterRefTransition),
    ] {
        let c = Conflict::primary()?;
        interruption.interrupt(&c, RESOLVE, "Resolved");
        let before = state_of(&c);
        let opened = connections(&c.w);
        let changed = c.on(&fresh(&c.w), RESOLVE, c.titled("Changed"));
        // The command records no operation of its own (R-32): the refusal
        // carries no effects, and is an input error.
        assert_eq!(changed.outcome, Outcome::Error, "{interruption:?}");
        assert!(changed.data.is_none(), "{interruption:?}");
        assert!(changed.recovery.is_empty(), "{interruption:?}");
        assert_nothing_written(&c, RESOLVE, &changed, ResultCode::RequestMismatch, before);
        assert_eq!(connections(&c.w), opened);
        // The same input still finishes it.
        let pushes = accepted(&c.w.server);
        let retried = c.on(&fresh(&c.w), RESOLVE, c.titled("Resolved"));
        c.assert_published(&retried, pushes);
    }
    Ok(())
}

fn c4_resolution_new_request_supersedes_written_paths() -> Result<(), FixtureError> {
    let c = Conflict::primary()?;
    let pushes = accepted(&c.w.server);
    c.interrupted_at(FailurePoint::ResolutionAfterPathWrite, RESOLVE, "Resolved");
    assert_eq!(c.show().request_id.as_deref(), Some(RESOLVE));
    // The first request is lost. Its written path is unchanged, and a new
    // request, with a source of its own, supersedes its attempt.
    let superseding = c.on(&fresh(&c.w), OTHER, c.titled("Superseding"));
    c.assert_published(&superseding, pushes);
    assert_eq!(c.written(), c.source("Superseding"));
    // The old attempt is closed: one attempt, the new request's.
    assert_eq!(attempts(&c.w), (1, 1));
    let owner: String = fixed(c.w.db()?.query_row(
        "SELECT request_ulid FROM remote_resolution_owners",
        [],
        |row| row.get(0),
    ))?;
    assert_eq!(owner, OTHER);
    // The first request, sent again, finds no attempt of its own: the
    // conflict is resolved, and it is told to look.
    let late = c.on(&fresh(&c.w), RESOLVE, c.titled("Resolved"));
    assert_eq!(late.code, ResultCode::RecoveryRequired);
    assert_eq!(actions(&late), [RecoveryActionKind::RepoInspect]);
    assert_eq!(late.effects, Effects::not_requested());
    Ok(())
}

/// Row 8: a new request, `OTHER`, after `RESOLVE`'s written path was edited
/// by hand. It is `original_request_required`, naming `RESOLVE`, with
/// nothing written and the edit intact.
fn superseding_after_an_edit(c: &Conflict) -> Envelope<MutationDataDto> {
    let before = state_of(c);
    let refused = c.on(&fresh(&c.w), OTHER, c.titled("Superseding"));
    assert_eq!(refused.outcome, Outcome::Blocked);
    assert!(refused.data.is_none());
    assert_eq!(refused.recovery.len(), 1);
    assert_eq!(refused.recovery[0].action, RecoveryActionKind::RequestRetry);
    assert_eq!(
        refused.recovery[0].arguments.get("request_id"),
        Some(&serde_json::json!(RESOLVE))
    );
    assert_nothing_written(
        c,
        OTHER,
        &refused,
        ResultCode::OriginalRequestRequired,
        before,
    );
    assert_eq!(c.written(), c.source("Edited by hand"));
    assert_eq!(c.show().request_id.as_deref(), Some(RESOLVE));
    refused
}

fn c4_resolution_new_request_after_a_written_path_is_edited() -> Result<(), FixtureError> {
    let c = Conflict::primary()?;
    c.interrupted_at(FailurePoint::ResolutionAfterPathWrite, RESOLVE, "Resolved");
    std::fs::write(c.worktree.join(c.path()), c.source("Edited by hand")).unwrap();
    let refused = superseding_after_an_edit(&c);
    assert_resolution_golden(
        &c.w,
        "conflict_resolve_original_request_required",
        c.operation,
        &refused,
    );
    Ok(())
}

fn c4_resolution_resume_finishes_a_prepared_commit() -> Result<(), FixtureError> {
    for point in [
        FailurePoint::ResolutionAfterCandidatePrepared,
        FailurePoint::ResolutionAfterRefTransition,
    ] {
        let c = Conflict::primary()?;
        let pushes = accepted(&c.w.server);
        c.interrupted_at(point, RESOLVE, "Resolved");
        let prepared = candidate(&c.w);
        // The first request is lost: `operation resume`, which presents no
        // source, finishes the attempt from durable state alone.
        let resumed = call(
            &c.w,
            &fresh(&c.w),
            RESUME,
            resume(&c.w, &c.operation.to_string()),
            Provider,
            &mut NoProgress,
            &CancellationToken::new(),
        );
        assert_eq!(resumed.code, ResultCode::Ok, "{point:?}: {resumed:?}");
        assert_eq!(resumed.effects.commit_oid, Some(prepared.to_string()));
        assert_eq!(resumed.effects.publication, PublicationEffect::Published);
        assert_eq!(c.resolutions(), [prepared], "{point:?}");
        assert_eq!(c.served(), prepared, "{point:?}");
        assert_eq!(accepted(&c.w.server), pushes + 1);
        assert_eq!(c.show().state, ConflictState::Complete);
    }
    Ok(())
}

fn c4_resolution_new_request_after_commit_prepared() -> Result<(), FixtureError> {
    for point in [
        FailurePoint::ResolutionAfterCandidatePrepared,
        FailurePoint::ResolutionAfterRefTransition,
    ] {
        let c = Conflict::primary()?;
        c.interrupted_at(point, RESOLVE, "Resolved");
        let before = state_of(&c);
        let refused = c.on(&fresh(&c.w), OTHER, c.titled("Another"));
        assert_eq!(refused.outcome, Outcome::Blocked, "{point:?}");
        assert!(refused.data.is_none());
        // Its next step is `operation resume`, which finishes it.
        assert_eq!(
            actions(&refused),
            [
                RecoveryActionKind::OperationResume,
                RecoveryActionKind::RepoInspect
            ],
            "{point:?}"
        );
        assert_eq!(
            refused.recovery[0].operation_id.as_deref(),
            Some(c.operation.to_string().as_str())
        );
        assert_nothing_written(&c, OTHER, &refused, ResultCode::RecoveryRequired, before);
    }
    Ok(())
}

fn c4_resolution_configuration_changed() -> Result<(), FixtureError> {
    let c = Conflict::item(|_| Ok(()))?;
    let pushes = accepted(&c.w.server);
    c.interrupted_at(FailurePoint::ResolutionAfterPathWrite, RESOLVE, "Resolved");
    // `config.toml` is changed after the paths are written: the live
    // observation is no longer the one the request presents.
    let config = c.w.root.join(".manyhands/config.toml");
    let mut source = std::fs::read_to_string(&config).unwrap();
    source.push_str("# changed after the paths were written\n");
    commit_in(&c.w.root, MAIN, ".manyhands/config.toml", source.as_bytes())?;
    assert_ne!(
        c.show().observation.as_deref(),
        Some(c.observation.as_str())
    );
    // The stored input digest matches: the same request finishes it.
    let retried = c.on(&fresh(&c.w), RESOLVE, c.titled("Resolved"));
    c.assert_published(&retried, pushes);
    Ok(())
}

fn c4_resolution_published_by_another_request() -> Result<(), FixtureError> {
    let c = Conflict::primary()?;
    let pushes = accepted(&c.w.server);
    // The resolution is checkpointed and the publication stops before it
    // pushes anything.
    c.w.server.disconnect_at(FixtureBoundary::Advertisement);
    let stopped = c.execute(RESOLVE, c.titled("Resolved"));
    c.w.server.clear_fault();
    assert_eq!(stopped.code, ResultCode::TransportUnavailable);
    let commit = c.resolutions()[0];
    assert_eq!(stopped.effects, resolved_only(commit));
    // Another request publishes it.
    let resumed = execute(&c.w, RESUME, resume(&c.w, &c.operation.to_string()));
    assert_eq!(resumed.code, ResultCode::Ok);
    assert_eq!(c.served(), commit);
    assert_eq!(accepted(&c.w.server), pushes + 1);
    let opened = connections(&c.w);
    // The same request is a no-op, with the same commit.
    let again = c.on(&fresh(&c.w), RESOLVE, c.titled("Resolved"));
    assert_eq!(again.code, ResultCode::AlreadyApplied);
    assert_eq!(again.outcome, Outcome::Noop);
    assert_eq!(again.effects.commit_oid, Some(commit.to_string()));
    assert_eq!(
        again.data,
        Some(data_of(
            c.operation,
            Some(commit),
            PublicationEffect::Current,
            None
        ))
    );
    assert_eq!(connections(&c.w), opened);
    assert_eq!(accepted(&c.w.server), pushes + 1);
    assert_eq!(c.resolutions(), [commit]);
    Ok(())
}

fn c4_resolution_new_request_after_success() -> Result<(), FixtureError> {
    let c = Conflict::primary()?;
    let pushes = accepted(&c.w.server);
    let resolved = c.execute(RESOLVE, c.titled("Resolved"));
    c.assert_published(&resolved, pushes);
    let before = state_of(&c);
    for title in ["Resolved", "Another"] {
        let refused = c.on(&fresh(&c.w), OTHER, c.titled(title));
        assert_eq!(refused.outcome, Outcome::Blocked);
        assert_eq!(actions(&refused), [RecoveryActionKind::RepoInspect]);
        assert_nothing_written(
            &c,
            OTHER,
            &refused,
            ResultCode::RecoveryRequired,
            before.clone(),
        );
    }
    assert_eq!(accepted(&c.w.server), pushes + 1);
    Ok(())
}

/// The states of a local-context sync's two rows, which the case changes
/// to stage what the gate refuses to stage (see below).
fn local_states(w: &World, operation: OperationId) -> (String, String) {
    let db = w.db().unwrap();
    (
        db.query_row(
            "SELECT state FROM local_context_sync_records WHERE operation_ulid=?1",
            [operation.to_string()],
            |row| row.get(0),
        )
        .unwrap(),
        db.query_row(
            "SELECT state FROM operation_records WHERE operation_ulid=?1",
            [operation.to_string()],
            |row| row.get(0),
        )
        .unwrap(),
    )
}

fn set_local_states(w: &World, operation: OperationId, (sync, record): (&str, &str)) {
    let db = w.db().unwrap();
    db.execute(
        "UPDATE local_context_sync_records SET state=?2 WHERE operation_ulid=?1",
        rusqlite::params![operation.to_string(), sync],
    )
    .unwrap();
    db.execute(
        "UPDATE operation_records SET state=?2 WHERE operation_ulid=?1",
        rusqlite::params![operation.to_string(), record],
    )
    .unwrap();
}

/// R-38. The gate refuses a local-context sync while a remote operation is
/// retained, and every remote reservation while a local-context sync is
/// unfinished (`local_sync/record.rs`, `reservation.rs`), so the two cannot
/// be staged one after the other. The local sync is left in its conflict
/// through the gate's own mutation (Task 1's helper), its two rows read as
/// ended while the remote conflict is made and its resolution interrupted,
/// and then put back as they were.
fn c4_resolution_unfinished_local_sync() -> Result<(), FixtureError> {
    let mut local = None;
    let c = Conflict::item(|w| {
        let operation = leave_local_sync_conflict(w);
        local = Some((operation, local_states(w, operation)));
        set_local_states(w, operation, ("cancelled", "completed"));
        Ok(())
    })?;
    let (operation, (sync, row)) = local.ok_or(FixtureError)?;
    let pushes = accepted(&c.w.server);
    c.interrupted_at(FailurePoint::ResolutionAfterPathWrite, RESOLVE, "Resolved");
    set_local_states(&c.w, operation, (&sync, &row));
    let before = state_of(&c);
    let opened = connections(&c.w);
    let refused = c.on(&fresh(&c.w), RESOLVE, c.titled("Resolved"));
    assert_eq!(refused.outcome, Outcome::Blocked);
    assert_eq!(actions(&refused), [RecoveryActionKind::RepoInspect]);
    assert!(refused.data.is_none());
    assert_nothing_written(&c, RESOLVE, &refused, ResultCode::RecoveryRequired, before);
    assert_eq!(connections(&c.w), opened);
    // The attempt is kept, and so is the request's ownership of it: once
    // the local sync ends, the same retry finishes it.
    assert_eq!(attempts(&c.w), (1, 1));
    set_local_states(&c.w, operation, ("cancelled", "completed"));
    let retried = c.on(&fresh(&c.w), RESOLVE, c.titled("Resolved"));
    c.assert_published(&retried, pushes);
    Ok(())
}

fn c4_resolution_resume_before_publication() -> Result<(), FixtureError> {
    let c = Conflict::primary()?;
    let pushes = accepted(&c.w.server);
    c.w.server.disconnect_at(FixtureBoundary::Advertisement);
    let stopped = c.execute(RESOLVE, c.titled("Resolved"));
    c.w.server.clear_fault();
    assert_eq!(stopped.code, ResultCode::TransportUnavailable);
    assert_eq!(stopped.outcome, Outcome::Partial);
    let commit = c.resolutions()[0];
    assert_eq!(c.show().state, ConflictState::Resolved);
    let resumed = call(
        &c.w,
        &fresh(&c.w),
        RESUME,
        resume(&c.w, &c.operation.to_string()),
        Provider,
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(resumed.code, ResultCode::Ok);
    assert_eq!(resumed.effects.publication, PublicationEffect::Published);
    assert_eq!(c.served(), commit);
    assert_eq!(accepted(&c.w.server), pushes + 1);
    Ok(())
}

fn c4_resolution_resume_after_publication() -> Result<(), FixtureError> {
    let c = Conflict::primary()?;
    let pushes = accepted(&c.w.server);
    let resolved = c.execute(RESOLVE, c.titled("Resolved"));
    c.assert_published(&resolved, pushes);
    let opened = connections(&c.w);
    let resumed = execute(&c.w, RESUME, resume(&c.w, &c.operation.to_string()));
    assert_eq!(resumed.code, ResultCode::AlreadyApplied);
    assert_eq!(resumed.outcome, Outcome::Noop);
    assert_eq!(connections(&c.w), opened);
    assert_eq!(accepted(&c.w.server), pushes + 1);
    Ok(())
}

/// One call of `conflict resolve` whose sink cancels its token when it is
/// told of `stage`: what a Ctrl-C at that point does.
fn cancelled_at(c: &Conflict, stage: ProgressStage) -> Envelope<MutationDataDto> {
    let token = CancellationToken::new();
    let mut sink = CancelAt {
        stage,
        token: token.clone(),
        seen: Vec::new(),
    };
    let envelope = checked(call(
        &c.w,
        &c.w.service,
        RESOLVE,
        c.titled("Resolved"),
        Provider,
        &mut sink,
        &token,
    ));
    assert!(token.is_cancelled());
    envelope
}

fn c4_resolution_cancelled_before_publication() -> Result<(), FixtureError> {
    let c = Conflict::primary()?;
    let pushes = accepted(&c.w.server);
    let opened = connections(&c.w);
    // The token is cancelled as the resolution begins: it is not
    // interrupted, and the continuation never starts.
    let cancelled = cancelled_at(&c, ProgressStage::Writing);
    assert_eq!(cancelled.code, ResultCode::Cancelled);
    assert_eq!(cancelled.outcome, Outcome::Cancelled);
    let commit = c.resolutions()[0];
    // The checkpoint is committed.
    assert_eq!(cancelled.effects, resolved_only(commit));
    assert_eq!(
        cancelled.data,
        Some(data_of(
            c.operation,
            Some(commit),
            PublicationEffect::Pending,
            None
        ))
    );
    assert_eq!(c.tip(), commit);
    assert_eq!(connections(&c.w), opened);
    assert_eq!(
        record(&c.w, RESOLVE).ok_or(FixtureError)?.state,
        RequestState::Finished
    );
    // The synchronization stays resolved, and `operation resume`
    // publishes it.
    assert_eq!(c.show().state, ConflictState::Resolved);
    let resumed = execute(&c.w, RESUME, resume(&c.w, &c.operation.to_string()));
    assert_eq!(resumed.code, ResultCode::Ok);
    assert_eq!(c.served(), commit);
    assert_eq!(accepted(&c.w.server), pushes + 1);
    Ok(())
}

fn c4_resolution_cancelled_during_publication() -> Result<(), FixtureError> {
    let c = Conflict::primary()?;
    let pushes = accepted(&c.w.server);
    let cancelled = cancelled_at(&c, ProgressStage::Pushing);
    assert_eq!(cancelled.code, ResultCode::Cancelled);
    assert_eq!(cancelled.outcome, Outcome::Cancelled);
    let commit = c.resolutions()[0];
    assert_eq!(cancelled.effects, resolved_only(commit));
    assert_eq!(c.served(), c.incoming);
    assert_eq!(accepted(&c.w.server), pushes);
    // The synchronization is final (fact 56).
    assert_eq!(phase(&c.w, c.operation)?, "cancelled");
    assert_eq!(c.show().state, ConflictState::Cancelled);
    // A new synchronization publishes the resolution commit.
    let later = execute(&c.w, LATER, repo_sync(&c.w));
    assert_eq!(later.code, ResultCode::Ok);
    assert_eq!(c.served(), commit);
    assert_eq!(c.resolutions(), [commit]);
    Ok(())
}

/// Adds `PRIVATE` to the world's probes: no store and no envelope may
/// hold it from now on.
fn private(c: &mut Conflict) -> Result<(), FixtureError> {
    c.w.probes.push(PRIVATE.as_bytes().to_vec());
    c.w.privacy()
}

fn c4_resolution_duplicate_token() -> Result<(), FixtureError> {
    let mut c = Conflict::primary()?;
    private(&mut c)?;
    let before = state_of(&c);
    let opened = connections(&c.w);
    let token = c.tokens[0].clone();
    let source = format!("{}{PRIVATE}\n", c.source("Resolved"));
    // The same token twice: refused before anything is read.
    let duplicate = c.execute(
        RESOLVE,
        resolve(
            &c.w,
            c.operation,
            &c.observation,
            [(token.clone(), source.clone()), (token, source)],
        ),
    );
    assert_eq!(duplicate.outcome, Outcome::Error);
    assert!(duplicate.data.is_none());
    assert_nothing_written(&c, RESOLVE, &duplicate, ResultCode::InvalidInput, before);
    assert_eq!(attempts(&c.w), (0, 0));
    assert_eq!(connections(&c.w), opened);
    // No source reached the envelope (`call` scans it) or a store.
    c.w.privacy()?;
    Ok(())
}

fn c4_resolution_foreign_token() -> Result<(), FixtureError> {
    // One synchronization's conflict is resolved, and another meets a
    // conflict of its own.
    let first = Conflict::primary()?;
    let pushes = accepted(&first.w.server);
    let resolved = first.execute(OTHER, first.titled("Resolved"));
    first.assert_published(&resolved, pushes);
    let foreign = first.tokens[0].clone();
    let first_operation = first.operation;
    let mut c = Conflict::stopped(
        first.w,
        LATER,
        MAIN,
        first.worktree,
        first.ticket,
        repo_sync,
    )?;
    assert_ne!(c.operation, first_operation);
    private(&mut c)?;
    let before = state_of(&c);
    // The other operation's token, with this conflict's observation.
    let refused = c.execute(
        RESOLVE,
        resolve(
            &c.w,
            c.operation,
            &c.observation,
            [(foreign, format!("{}{PRIVATE}\n", c.source("Resolved")))],
        ),
    );
    assert!(refused.data.is_none());
    assert_nothing_written(&c, RESOLVE, &refused, ResultCode::ExternalChange, before);
    assert_eq!(attempts(&c.w), (1, 1));
    c.w.privacy()?;
    Ok(())
}

fn c4_resolution_publishes() -> Result<(), FixtureError> {
    let c = Conflict::primary()?;
    let pushes = accepted(&c.w.server);
    let resolved = c.execute(RESOLVE, c.titled("Resolved"));
    let commit = c.assert_published(&resolved, pushes);
    assert_eq!(c.written(), c.source("Resolved"));
    assert_eq!(
        record(&c.w, RESOLVE).ok_or(FixtureError)?.state,
        RequestState::Finished
    );
    // The record holds no operation, and the same request is answered
    // from it.
    assert!(
        record(&c.w, RESOLVE)
            .ok_or(FixtureError)?
            .operations
            .is_empty()
    );
    assert_eq!(c.execute(RESOLVE, c.titled("Resolved")), resolved);
    assert_eq!(accepted(&c.w.server), pushes + 1);
    assert_eq!(c.resolutions(), [commit]);
    assert_resolution_golden(&c.w, "conflict_resolve", c.operation, &resolved);
    Ok(())
}

fn c4_resolution_syntax() -> Result<(), FixtureError> {
    let c = Conflict::primary()?;
    let before = state_of(&c);
    let token = c.tokens[0].clone();
    let source = c.source("Resolved");
    for (observation, resolutions) in [
        // A malformed observation.
        (
            "conflict-v1:nothex".to_owned(),
            vec![(token.clone(), source.clone())],
        ),
        (
            c.observation.to_uppercase(),
            vec![(token.clone(), source.clone())],
        ),
        // A malformed token.
        (
            c.observation.clone(),
            vec![("token".to_owned(), source.clone())],
        ),
        // No resolution.
        (c.observation.clone(), Vec::new()),
    ] {
        let refused = c.execute(
            RESOLVE,
            resolve(&c.w, c.operation, &observation, resolutions),
        );
        assert_nothing_written(
            &c,
            RESOLVE,
            &refused,
            ResultCode::InvalidInput,
            before.clone(),
        );
    }
    assert_eq!(attempts(&c.w), (0, 0));
    Ok(())
}

fn c4_resolution_another_observation() -> Result<(), FixtureError> {
    let c = Conflict::primary()?;
    let before = state_of(&c);
    let stale = c.execute(
        RESOLVE,
        resolve(
            &c.w,
            c.operation,
            NO_SUCH_OBSERVATION,
            [(c.tokens[0].clone(), c.source("Resolved"))],
        ),
    );
    assert!(stale.data.is_none());
    assert!(stale.recovery.is_empty());
    assert_nothing_written(&c, RESOLVE, &stale, ResultCode::ExternalChange, before);
    assert_eq!(attempts(&c.w), (0, 0));
    assert_resolution_golden(
        &c.w,
        "conflict_resolve_external_change",
        c.operation,
        &stale,
    );
    Ok(())
}

fn c4_resolution_wrong_count() -> Result<(), FixtureError> {
    let c = Conflict::primary()?;
    let before = state_of(&c);
    // Count is checked before membership (fact 53): an extra token that
    // is no path of this conflict is a count, not a stale token.
    let counted = c.execute(
        RESOLVE,
        resolve(
            &c.w,
            c.operation,
            &c.observation,
            [
                (c.tokens[0].clone(), c.source("Resolved")),
                (NO_SUCH_TOKEN.to_owned(), c.source("Resolved")),
            ],
        ),
    );
    assert!(counted.data.is_none());
    assert_nothing_written(
        &c,
        RESOLVE,
        &counted,
        ResultCode::InvalidInput,
        before.clone(),
    );
    // A token of no path of it, in the place of the one path's.
    let stale = c.execute(
        RESOLVE,
        resolve(
            &c.w,
            c.operation,
            &c.observation,
            [(NO_SUCH_TOKEN.to_owned(), c.source("Resolved"))],
        ),
    );
    assert_nothing_written(&c, RESOLVE, &stale, ResultCode::ExternalChange, before);
    assert_eq!(attempts(&c.w), (0, 0));
    assert_resolution_golden(
        &c.w,
        "conflict_resolve_invalid_input",
        c.operation,
        &counted,
    );
    Ok(())
}

fn c4_resolution_external_only() -> Result<(), FixtureError> {
    let w = world()?;
    // Each side adds a file that is no canonical content, with other bytes.
    let local = commit_in(&w.root, MAIN, "conflict", b"local conflict\n")?;
    let incoming = remote_commit(&w, MAIN, "conflict", b"incoming conflict\n")?;
    let stopped = execute(&w, SYNC, repo_sync(&w));
    assert_eq!(stopped.code, ResultCode::RecoveryRequired);
    let operation = OperationId::parse(stopped.operation_id.as_deref().ok_or(FixtureError)?)
        .map_err(|_| FixtureError)?;
    let repo = w.service.resolve_repository(&w.root).unwrap();
    let shown = w.service.show_conflict(&repo, operation).unwrap();
    assert_eq!(shown.next_action, ConflictReadNextAction::ExternalRepair);
    let refused = resolved_on(
        &w,
        &w.service,
        RESOLVE,
        resolve(
            &w,
            operation,
            shown.observation.as_deref().ok_or(FixtureError)?,
            [(shown.paths[0].token.clone(), "resolved\n".to_owned())],
        ),
    );
    assert_eq!(refused.code, ResultCode::RecoveryRequired);
    assert_eq!(refused.effects, Effects::not_requested());
    assert_eq!(
        refused.data,
        Some(data_of(
            operation,
            None,
            PublicationEffect::NotRequested,
            Some(ConflictNextDto {
                operation_id: operation.to_string(),
                next_action: ConflictNextAction::ExternalRepair,
            })
        ))
    );
    assert_eq!(actions(&refused), [RecoveryActionKind::RepoInspect]);
    assert!(record(&w, RESOLVE).is_none());
    assert_eq!(attempts(&w), (0, 0));
    assert_eq!(w.repo()?.refname_to_id(MAIN).unwrap(), local);
    assert_eq!(w.bare()?.refname_to_id(MAIN).unwrap(), incoming);
    Ok(())
}

fn c4_resolution_operation_lookup() -> Result<(), FixtureError> {
    let c = Conflict::primary()?;
    let input = |operation| {
        resolve(
            &c.w,
            operation,
            &c.observation,
            [(c.tokens[0].clone(), c.source("Resolved"))],
        )
    };
    // An ID no operation of the repository has.
    let unknown = c.execute(RESOLVE, input(OperationId::new()));
    assert_eq!(unknown.code, ResultCode::OperationNotFound);
    assert!(record(&c.w, RESOLVE).is_none());
    // An operation that is no synchronization: a ticket's creation.
    let created = execute(&c.w, LATER, create_other(&c.w));
    assert_eq!(created.code, ResultCode::Ok);
    let other = OperationId::parse(created.operation_id.as_deref().ok_or(FixtureError)?)
        .map_err(|_| FixtureError)?;
    let refused = c.execute(RESOLVE, input(other));
    assert_eq!(refused.code, ResultCode::InvalidInput);
    assert!(record(&c.w, RESOLVE).is_none());
    assert_eq!(attempts(&c.w), (0, 0));
    Ok(())
}

/// A local-context sync's ID is `invalid_input` (decision 19, R-37).
fn c4_resolution_local_sync_operation() -> Result<(), FixtureError> {
    let w = world()?;
    let operation = leave_local_sync_conflict(&w);
    let refused = resolved_on(
        &w,
        &w.service,
        RESOLVE,
        resolve(
            &w,
            operation,
            NO_SUCH_OBSERVATION,
            [(NO_SUCH_TOKEN.to_owned(), "source\n".to_owned())],
        ),
    );
    assert_eq!(refused.code, ResultCode::InvalidInput);
    assert_eq!(refused.effects, Effects::not_requested());
    assert!(record(&w, RESOLVE).is_none());
    Ok(())
}

/// A comment whose synchronization stopped on its item's conflict: its
/// `comment add` request is `SYNC`, and still holds its operation.
struct CommentConflict {
    w: World,
    /// The comment's operation.
    comment: OperationId,
    /// The synchronization that publishes it.
    synchronization: OperationId,
    local: git2::Oid,
    incoming: git2::Oid,
    observation: String,
    token: String,
}

impl CommentConflict {
    fn new() -> Result<Self, FixtureError> {
        let mut w = world()?;
        let context = w.context()?;
        assert_eq!(execute(&w, SETUP, item_sync(&w)).code, ResultCode::Ok);
        let path = merging::ticket_path(ITEM);
        let local = commit_in(
            &context.worktree,
            CONTEXT,
            &path,
            merging::ticket_source(ITEM, "Local").as_bytes(),
        )?;
        let incoming = remote_commit(
            &w,
            CONTEXT,
            &path,
            merging::ticket_source(ITEM, "Incoming").as_bytes(),
        )?;
        let added = execute(&w, SYNC, comment_add(&w));
        assert_eq!(added.code, ResultCode::RecoveryRequired);
        let Some(MutationDataDto::Comment(data)) = &added.data else {
            return Err(FixtureError);
        };
        let synchronization =
            OperationId::parse(&data.synchronization_id).map_err(|_| FixtureError)?;
        let comment = OperationId::parse(added.operation_id.as_deref().ok_or(FixtureError)?)
            .map_err(|_| FixtureError)?;
        let repo = w.service.resolve_repository(&w.root).unwrap();
        let shown = w.service.show_conflict(&repo, comment).unwrap();
        Ok(Self {
            observation: shown.observation.ok_or(FixtureError)?,
            token: shown.paths[0].token.clone(),
            w,
            comment,
            synchronization,
            local,
            incoming,
        })
    }

    /// `conflict resolve` of the conflict, naming it by `operation`.
    fn resolve(&self, operation: OperationId) -> Mutation {
        resolve(
            &self.w,
            operation,
            &self.observation,
            [(self.token.clone(), merging::ticket_source(ITEM, "Resolved"))],
        )
    }
}

fn c4_resolution_comment_conflict() -> Result<(), FixtureError> {
    let c = CommentConflict::new()?;
    let w = &c.w;
    let pushes = accepted(&w.server);
    // By the comment's operation ID (R-5): its synchronization is resolved,
    // and its retry publishes the comment.
    let resolved = resolved_on(w, &w.service, RESOLVE, c.resolve(c.comment));
    assert_eq!(resolved.code, ResultCode::Ok, "{resolved:?}");
    let Some(MutationDataDto::ConflictResolution(data)) = &resolved.data else {
        return Err(FixtureError);
    };
    // The data names the synchronization.
    assert_eq!(data.operation_id, c.synchronization.to_string());
    assert_eq!(data.publication, PublicationEffect::Published);
    let commit = git2::Oid::from_str(data.commit_oid.as_deref().ok_or(FixtureError)?)
        .map_err(|_| FixtureError)?;
    // The local parent is the comment's checkpoint, on the local side.
    let repo = w.repo()?;
    let parents: Vec<_> = fixed(repo.find_commit(commit))?.parent_ids().collect();
    assert_eq!(parents[1], c.incoming);
    assert!(fixed(repo.graph_descendant_of(parents[0], c.local))?);
    let served = w.bare()?;
    let tip = fixed(served.refname_to_id(CONTEXT))?;
    assert!(
        fixed(fixed(served.find_commit(tip))?.tree())?
            .get_path(Path::new(&format!(
                ".manyhands/comments/{ITEM}/{COMMENT}.md"
            )))
            .is_ok()
    );
    assert_eq!(accepted(&w.server), pushes + 1);
    // The same request, naming the synchronization by its own ID, is the
    // same request: the digest covers the resolved synchronization's ID
    // (design, "Conflict Resolution"), and its result is given again.
    let by_synchronization = resolved_on(w, &w.service, RESOLVE, c.resolve(c.synchronization));
    assert_eq!(by_synchronization, resolved);
    // The comment's own request, sent again, finds it published.
    let again = execute(w, SYNC, comment_add(w));
    assert_eq!(again.code, ResultCode::Ok);
    assert_eq!(accepted(&w.server), pushes + 1);
    Ok(())
}

/// The comment of the comment case.
fn comment_add(w: &World) -> Mutation {
    Mutation::CommentAdd(CommentAddInput {
        root: w.root.clone(),
        kind: AuthoringKind::Ticket,
        id: ITEM.parse().unwrap(),
        comment_id: COMMENT.parse().unwrap(),
        parent_id: None,
        body: "A comment of the resolution case.\n".to_owned(),
        created_by: None,
    })
}

/// A sink that sends `request cancel` of `request`, from a service of its
/// own, the first time it is told of `stage`: what another process's
/// `request cancel` does at that point of the call.
struct AskAt {
    stage: ProgressStage,
    request: &'static str,
    elsewhere: RepositoryService,
    asked: Option<CancelDto>,
}

impl AskAt {
    fn new(w: &World, request: &'static str, stage: ProgressStage) -> Self {
        Self {
            stage,
            request,
            elsewhere: fresh(w),
            asked: None,
        }
    }
}

impl ProgressSink for AskAt {
    fn event(&mut self, event: &ProgressEvent) {
        if event.stage == self.stage && self.asked.is_none() {
            self.asked = self
                .elsewhere
                .cancel_request(super::request(self.request))
                .data;
        }
    }
}

/// One call of `conflict resolve` under `RESOLVE` whose sink sends `request
/// cancel` of `holder` at `stage`, and what that `request cancel` answered.
fn asked_at(
    w: &World,
    mutation: Mutation,
    holder: &'static str,
    stage: ProgressStage,
) -> Result<(Envelope<MutationDataDto>, CancelDto), FixtureError> {
    let mut sink = AskAt::new(w, holder, stage);
    let envelope = checked(call(
        w,
        &w.service,
        RESOLVE,
        mutation,
        Provider,
        &mut sink,
        &CancellationToken::new(),
    ));
    Ok((envelope, sink.asked.ok_or(FixtureError)?))
}

/// Asserts that the request `holder`, which holds the synchronization, is
/// still accepted with its flag set: the continuation that stopped it did
/// not settle it.
fn assert_left_to_its_holder(w: &World, holder: &str) -> Result<(), FixtureError> {
    let held = record(w, holder).ok_or(FixtureError)?;
    assert_eq!(held.state, RequestState::Accepted);
    assert!(held.cancel_requested);
    Ok(())
}

/// Asserts that the holder's own next call settled its record as F2
/// settles a cancelled operation's: finished, with its flag cleared, and
/// the reservation released.
fn assert_settled_by_its_holder(w: &World, holder: &str) -> Result<(), FixtureError> {
    assert!(!reserved(w));
    let held = record(w, holder).ok_or(FixtureError)?;
    assert_eq!(held.state, RequestState::Finished);
    assert!(!held.cancel_requested);
    Ok(())
}

/// Asserts that `settled` is the answer of an operation cancelled before
/// it intended a push.
fn assert_cancelled(settled: &Envelope<MutationDataDto>) {
    assert_eq!(settled.code, ResultCode::Cancelled, "{settled:?}");
    assert_eq!(settled.outcome, Outcome::Cancelled);
    assert!(settled.recovery.is_empty());
}

/// I1: `request cancel` of the synchronization's own request, while the
/// resolution's continuation fetches. The operation intends no push yet.
fn c4_resolution_original_request_cancels_before_push_intent() -> Result<(), FixtureError> {
    let c = Conflict::primary()?;
    let pushes = accepted(&c.w.server);
    // The continuation reports no stage of its own before it fetches: its
    // first is the synchronization's first safe point.
    let (cancelled, asked) = asked_at(&c.w, c.titled("Resolved"), SYNC, ProgressStage::Fetching)?;
    assert!(asked.requested);
    assert_eq!(cancelled.code, ResultCode::Cancelled, "{cancelled:?}");
    assert_eq!(cancelled.outcome, Outcome::Cancelled);
    let commit = c.resolutions()[0];
    assert_eq!(cancelled.effects, resolved_only(commit));
    assert_eq!(c.served(), c.incoming);
    assert_eq!(accepted(&c.w.server), pushes);
    assert_eq!(phase(&c.w, c.operation)?, "cancelled");
    assert_eq!(
        record(&c.w, RESOLVE).ok_or(FixtureError)?.state,
        RequestState::Finished
    );
    assert_left_to_its_holder(&c.w, SYNC)?;
    let settled = execute(&c.w, SYNC, repo_sync(&c.w));
    assert_cancelled(&settled);
    assert_settled_by_its_holder(&c.w, SYNC)?;
    assert_eq!(accepted(&c.w.server), pushes);
    Ok(())
}

/// I1: `request cancel` of the synchronization's own request once the
/// operation has recorded its intent to push, when `request cancel` no
/// longer asks the operation itself (`progress.rs`). The continuation
/// passes it on at the safe point before the push.
fn c4_resolution_original_request_cancels_after_push_intent() -> Result<(), FixtureError> {
    let c = Conflict::primary()?;
    let pushes = accepted(&c.w.server);
    let (cancelled, asked) = asked_at(&c.w, c.titled("Resolved"), SYNC, ProgressStage::Pushing)?;
    assert!(asked.requested);
    assert_eq!(cancelled.code, ResultCode::Cancelled, "{cancelled:?}");
    assert_eq!(cancelled.outcome, Outcome::Cancelled);
    let commit = c.resolutions()[0];
    assert_eq!(cancelled.effects, resolved_only(commit));
    // Nothing was pushed.
    assert_eq!(c.served(), c.incoming);
    assert_eq!(accepted(&c.w.server), pushes);
    assert_eq!(phase(&c.w, c.operation)?, "cancelled");
    assert_left_to_its_holder(&c.w, SYNC)?;
    let settled = execute(&c.w, SYNC, repo_sync(&c.w));
    // The operation recorded its intent to push before it was cancelled,
    // and the holder's own call cannot tell that nothing was pushed: F2
    // says the publication is not confirmed (fact 74), as for any
    // operation cancelled with a push intended.
    assert_eq!(settled.code, ResultCode::RecoveryRequired, "{settled:?}");
    assert_eq!(settled.outcome, Outcome::Partial);
    assert_eq!(settled.effects.publication, PublicationEffect::Pending);
    assert_eq!(settled.effects.commit_oid, Some(commit.to_string()));
    assert_eq!(actions(&settled), [RecoveryActionKind::RepoInspect]);
    assert_settled_by_its_holder(&c.w, SYNC)?;
    assert_eq!(accepted(&c.w.server), pushes);
    // A new synchronization publishes the resolution commit.
    let later = execute(&c.w, LATER, repo_sync(&c.w));
    assert_eq!(later.code, ResultCode::Ok);
    assert_eq!(c.served(), commit);
    Ok(())
}

/// I1: a comment's synchronization, whose `comment add` request holds the
/// comment's operation and not the synchronization (fact 79), cancelled
/// through that request.
fn c4_resolution_comment_request_cancels() -> Result<(), FixtureError> {
    let c = CommentConflict::new()?;
    let w = &c.w;
    let pushes = accepted(&w.server);
    let (cancelled, asked) = asked_at(w, c.resolve(c.comment), SYNC, ProgressStage::Fetching)?;
    assert!(asked.requested);
    assert_eq!(cancelled.code, ResultCode::Cancelled, "{cancelled:?}");
    assert_eq!(cancelled.outcome, Outcome::Cancelled);
    assert_eq!(accepted(&w.server), pushes);
    assert_eq!(phase(w, c.synchronization)?, "cancelled");
    assert_left_to_its_holder(w, SYNC)?;
    let settled = execute(w, SYNC, comment_add(w));
    assert_cancelled(&settled);
    assert_settled_by_its_holder(w, SYNC)?;
    assert_eq!(accepted(&w.server), pushes);
    Ok(())
}

/// S1 (R-14): an attempt ID that is already some other row's. The library
/// refuses it before it records anything, and so does the binding, before
/// it records the request as the attempt's owner.
fn c4_resolution_attempt_id_collision() -> Result<(), FixtureError> {
    let c = Conflict::primary()?;
    let (window, ordinal): (u32, u8) = fixed(c.w.db()?.query_row(
        "SELECT step.window_number, step.ordinal FROM remote_integration_steps step
           JOIN remote_operation_records operation ON operation.id = step.operation_record_id
          WHERE operation.operation_ulid = ?1 AND step.conflict_digest IS NOT NULL",
        [c.operation.to_string()],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ))?;
    let derived =
        repository::conflict_resolve_attempt(request(RESOLVE), c.operation, window, ordinal);
    // A finished operation of another repository's journal has that ID:
    // one of the tables `validate_child_id` reads.
    let plant =
        "INSERT INTO operation_records(root_path, operation_ulid, action, state, observed_at)
                 VALUES ('/c4-resolution-collision', ?1, 'ticket_create', 'completed', 0)";
    fixed(c.w.db()?.execute(plant, [derived.to_string()]))?;
    let before = state_of(&c);
    let opened = connections(&c.w);
    let refused = c.execute(RESOLVE, c.titled("Resolved"));
    // As the library maps its refusal of the ID.
    assert_eq!(refused.outcome, Outcome::Blocked);
    assert_nothing_written(&c, RESOLVE, &refused, ResultCode::RecoveryRequired, before);
    let paths: i64 = fixed(c.w.db()?.query_row(
        "SELECT COUNT(*) FROM remote_resolution_paths",
        [],
        |row| row.get(0),
    ))?;
    assert_eq!(paths, 0);
    assert!(!c.w.root.join(".git/index.lock").exists());
    assert_eq!(connections(&c.w), opened);
    // Once the ID is free again, the same request resolves the conflict.
    fixed(c.w.db()?.execute(
        "DELETE FROM operation_records WHERE operation_ulid = ?1",
        [derived.to_string()],
    ))?;
    let pushes = accepted(&c.w.server);
    let resolved = c.execute(RESOLVE, c.titled("Resolved"));
    c.assert_published(&resolved, pushes);
    Ok(())
}

/// Minor 1: a supersede refused once the new request is recorded as its
/// attempt's owner, here by the old attempt's index sentinel, which no
/// longer holds what that attempt wrote. The domain never recorded the new
/// attempt, so the new request is not held to its input.
fn c4_resolution_refused_supersede_releases_the_request() -> Result<(), FixtureError> {
    let c = Conflict::primary()?;
    let pushes = accepted(&c.w.server);
    c.interrupted_at(FailurePoint::ResolutionAfterPathWrite, RESOLVE, "Resolved");
    let lock = c.w.root.join(".git/index.lock");
    let sentinel = fixed(std::fs::read(&lock))?;
    // Rewritten in place: the same file, with other bytes.
    fixed(std::fs::write(&lock, b"not the attempt's sentinel\n"))?;
    let refused = c.on(&fresh(&c.w), OTHER, c.titled("Superseding"));
    assert_eq!(refused.code, ResultCode::ExternalChange, "{refused:?}");
    assert!(record(&c.w, OTHER).is_none());
    // One attempt, and one owner: the first request's.
    assert_eq!(attempts(&c.w), (1, 1));
    let owner: String = fixed(c.w.db()?.query_row(
        "SELECT request_ulid FROM remote_resolution_owners",
        [],
        |row| row.get(0),
    ))?;
    assert_eq!(owner, RESOLVE);
    assert_eq!(c.written(), c.source("Resolved"));
    // The sentinel is put back. The same request, with another input, is
    // no mismatch: it supersedes the attempt and publishes.
    fixed(std::fs::write(&lock, &sentinel))?;
    let superseding = c.on(&fresh(&c.w), OTHER, c.titled("Another"));
    c.assert_published(&superseding, pushes);
    assert_eq!(c.written(), c.source("Another"));
    Ok(())
}

/// A conflict on primary in two tickets' canonical sources.
fn two_paths() -> Result<Conflict, FixtureError> {
    let w = world()?;
    let second = merging::ticket_path(SECOND_TICKET);
    for ticket in [merging::PRIMARY_TICKET, SECOND_TICKET] {
        remote_commit(
            &w,
            MAIN,
            &merging::ticket_path(ticket),
            merging::ticket_source(ticket, "Base").as_bytes(),
        )?;
    }
    assert_current(&execute(&w, SETUP, repo_sync(&w)));
    commit_in(
        &w.root,
        MAIN,
        &second,
        merging::ticket_source(SECOND_TICKET, "Local").as_bytes(),
    )?;
    remote_commit(
        &w,
        MAIN,
        &second,
        merging::ticket_source(SECOND_TICKET, "Incoming").as_bytes(),
    )?;
    let worktree = w.root.clone();
    let c = Conflict::stopped(w, SYNC, MAIN, worktree, merging::PRIMARY_TICKET, repo_sync)?;
    assert_eq!(c.tokens.len(), 2);
    Ok(c)
}

/// The ticket of each path of the conflict, in the order `conflict show`
/// gives the paths.
fn tickets_of(c: &Conflict) -> Vec<&'static str> {
    c.show()
        .paths
        .iter()
        .map(|path| {
            if path.display_path.contains(SECOND_TICKET) {
                SECOND_TICKET
            } else {
                merging::PRIMARY_TICKET
            }
        })
        .collect()
}

/// `conflict resolve` of every path of the conflict, each with its ticket's
/// source titled `title`.
fn all_titled(c: &Conflict, title: &str) -> Mutation {
    let sources: Vec<String> = tickets_of(c)
        .into_iter()
        .map(|ticket| merging::ticket_source(ticket, title))
        .collect();
    c.with(&sources)
}

/// Minor 4: an attempt stopped after its first path's write, with its
/// second path not observed. A hand edit of that path holds neither the
/// path's pre-write image nor its result, and blocks a supersede.
fn c4_resolution_unobserved_path_edited() -> Result<(), FixtureError> {
    let c = two_paths()?;
    let failing = fixed(RepositoryService::open_at_with_failure_point_for_testing(
        &c.w.data(),
        FailurePoint::ResolutionAfterPathWrite,
    ))?;
    let stopped = c.on(&failing, RESOLVE, all_titled(&c, "Resolved"));
    assert!(stopped.code.failure_class().is_some(), "{stopped:?}");
    let unobserved: i64 = fixed(c.w.db()?.query_row(
        "SELECT COUNT(*) FROM remote_resolution_paths WHERE applied = 0",
        [],
        |row| row.get(0),
    ))?;
    assert_eq!(unobserved, 1);
    // The path that does not hold its result is the one not observed.
    let pending = tickets_of(&c)
        .into_iter()
        .find(|ticket| {
            std::fs::read_to_string(c.worktree.join(merging::ticket_path(ticket))).unwrap()
                != merging::ticket_source(ticket, "Resolved")
        })
        .ok_or(FixtureError)?;
    let path = c.worktree.join(merging::ticket_path(pending));
    let edit = merging::ticket_source(pending, "Edited by hand");
    fixed(std::fs::write(&path, &edit))?;
    let before = state_of(&c);
    let refused = c.on(&fresh(&c.w), OTHER, all_titled(&c, "Superseding"));
    assert_eq!(refused.outcome, Outcome::Blocked, "{refused:?}");
    assert_eq!(actions(&refused), [RecoveryActionKind::RequestRetry]);
    assert_eq!(
        refused.recovery[0].arguments.get("request_id"),
        Some(&serde_json::json!(RESOLVE))
    );
    assert_nothing_written(
        &c,
        OTHER,
        &refused,
        ResultCode::OriginalRequestRequired,
        before,
    );
    assert_eq!(fixed(std::fs::read_to_string(&path))?, edit);
    Ok(())
}

/// Minor 7: a synchronization that met no conflict has none to resolve.
fn c4_resolution_synchronization_without_conflict() -> Result<(), FixtureError> {
    let w = world()?;
    advance(&w.repo()?, MAIN, "local-advance")?;
    let synced = execute(&w, SYNC, repo_sync(&w));
    assert_eq!(synced.code, ResultCode::Ok);
    let operation = OperationId::parse(synced.operation_id.as_deref().ok_or(FixtureError)?)
        .map_err(|_| FixtureError)?;
    let opened = connections(&w);
    let refused = resolved_on(
        &w,
        &w.service,
        RESOLVE,
        resolve(
            &w,
            operation,
            NO_SUCH_OBSERVATION,
            [(NO_SUCH_TOKEN.to_owned(), "source\n".to_owned())],
        ),
    );
    assert_eq!(refused.code, ResultCode::InvalidInput);
    assert_eq!(refused.effects, Effects::not_requested());
    assert!(refused.data.is_none());
    assert!(record(&w, RESOLVE).is_none());
    assert_eq!(attempts(&w), (0, 0));
    assert_eq!(connections(&w), opened);
    Ok(())
}

/// How many resolution paths the registry holds.
fn path_rows(w: &World) -> i64 {
    w.db()
        .unwrap()
        .query_row("SELECT COUNT(*) FROM remote_resolution_paths", [], |row| {
            row.get(0)
        })
        .unwrap()
}

/// Gives the repository `identity`, or takes its own away.
fn set_identity(w: &World, identity: Option<(&str, &str)>) -> Result<(), FixtureError> {
    let mut config = fixed(fixed(w.repo()?.config())?.open_level(git2::ConfigLevel::Local))?;
    match identity {
        Some((name, email)) => {
            fixed(config.set_str("user.name", name))?;
            fixed(config.set_str("user.email", email))?;
        }
        None => {
            fixed(config.remove("user.name"))?;
            fixed(config.remove("user.email"))?;
        }
    }
    Ok(())
}

/// I1, the fix: a refusal after the attempt began its paths and before it
/// wrote any, here for a Git `index.lock` the attempt does not own. The
/// rows prove nothing was written, so the attempt is closed as a supersede
/// closes one, and the conflict reads as one no request has begun to
/// resolve.
fn c4_resolution_refused_at_a_git_lock() -> Result<(), FixtureError> {
    let c = Conflict::primary()?;
    let pushes = accepted(&c.w.server);
    let lock = c.w.root.join(".git/index.lock");
    fixed(std::fs::write(&lock, b"held by another process\n"))?;
    let (tip, written) = (c.tip(), c.written());
    let refused = c.execute(RESOLVE, c.titled("Resolved"));
    // As today.
    assert_eq!(refused.code, ResultCode::ExternalChange, "{refused:?}");
    assert_eq!(refused.effects, Effects::not_requested());
    assert!(record(&c.w, RESOLVE).is_none());
    // No attempt, path or owner row, and the step as before any attempt.
    assert_eq!(attempts(&c.w), (0, 0));
    assert_eq!(path_rows(&c.w), 0);
    assert_eq!(c.step(), "conflict_pending");
    assert_eq!((c.tip(), c.written()), (tip, written));
    fixed(std::fs::remove_file(&lock))?;
    let shown = c.show();
    assert_eq!(shown.state, ConflictState::ConflictPending);
    assert_eq!(shown.next_action, ConflictReadNextAction::Resolve);
    assert_eq!(shown.request_id, None);
    assert!(!shown.historical);
    // The file is edited by hand, and a new request resolves the conflict.
    fixed(std::fs::write(
        c.worktree.join(c.path()),
        c.source("Edited by hand"),
    ))?;
    let resolved = c.on(&fresh(&c.w), OTHER, c.titled("Resolved"));
    c.assert_published(&resolved, pushes);
    Ok(())
}

/// I1, the escape: the first request's written path is edited by hand. No
/// request writes over the edit; once the path holds what the request wrote
/// there again, the same request goes on.
fn c4_resolution_original_request_after_an_edit() -> Result<(), FixtureError> {
    let c = Conflict::primary()?;
    let pushes = accepted(&c.w.server);
    c.interrupted_at(FailurePoint::ResolutionAfterPathWrite, RESOLVE, "Resolved");
    let path = c.worktree.join(c.path());
    fixed(std::fs::write(&path, c.source("Edited by hand")))?;
    // A new request (row 8).
    superseding_after_an_edit(&c);
    // The same request, with the same input.
    let before = state_of(&c);
    let refused = c.on(&fresh(&c.w), RESOLVE, c.titled("Resolved"));
    assert_nothing_written(&c, RESOLVE, &refused, ResultCode::ExternalChange, before);
    assert_eq!(c.written(), c.source("Edited by hand"));
    // It says which path, and how to go on.
    assert_eq!(
        refused.message,
        format!(
            "What the request was based on has changed. The path '{}' no longer \
             holds what this request wrote there: restore it to this request's \
             resolution, or to its content before the write, and send the request \
             again.",
            c.path()
        )
    );
    // The request's resolution is written back: the same request goes on.
    fixed(std::fs::write(&path, c.source("Resolved")))?;
    let retried = c.on(&fresh(&c.w), RESOLVE, c.titled("Resolved"));
    c.assert_published(&retried, pushes);
    Ok(())
}

/// M1: a supersede whose new request is then refused before its own attempt
/// is recorded, here for want of an identity. The superseded attempt is
/// closed with its step put back, so the conflict reads as one with no
/// attempt: `resolve`, no request named, and a resume meets the conflict
/// as it would before any attempt.
fn c4_resolution_superseded_then_refused() -> Result<(), FixtureError> {
    let c = Conflict::primary()?;
    let pushes = accepted(&c.w.server);
    c.interrupted_at(FailurePoint::ResolutionAfterPathWrite, RESOLVE, "Resolved");
    set_identity(&c.w, None)?;
    let refused = c.on(&fresh(&c.w), OTHER, c.titled("Superseding"));
    assert_eq!(refused.code, ResultCode::IdentityRequired, "{refused:?}");
    assert_eq!(refused.effects, Effects::not_requested());
    assert!(record(&c.w, OTHER).is_none());
    assert_eq!(attempts(&c.w), (0, 0));
    assert_eq!(c.step(), "conflict_pending");
    let shown = c.show();
    assert_eq!(shown.state, ConflictState::ConflictPending);
    assert_eq!(shown.next_action, ConflictReadNextAction::Resolve);
    assert_eq!(shown.request_id, None);
    assert!(!shown.historical);
    set_identity(&c.w, Some(("Fixture", "fixture@example.invalid")))?;
    let resumed = execute(&c.w, RESUME, resume(&c.w, &c.operation.to_string()));
    assert_eq!(resumed.code, ResultCode::RecoveryRequired, "{resumed:?}");
    assert_eq!(
        resumed.data,
        Some(MutationDataDto::SynchronizationConflict(
            SynchronizationConflictMutationDto {
                r#type: SynchronizationConflictType::SynchronizationConflict,
                conflict: ConflictNextDto {
                    operation_id: c.operation.to_string(),
                    next_action: ConflictNextAction::Resolve,
                },
            }
        ))
    );
    // A third request resolves it, from the conflict as it is read now.
    let shown = c.show();
    let third = resolve(
        &c.w,
        c.operation,
        shown.observation.as_deref().ok_or(FixtureError)?,
        [(shown.paths[0].token.clone(), c.source("Third"))],
    );
    let resolved = c.on(&fresh(&c.w), THIRD, third);
    c.assert_published(&resolved, pushes);
    assert_eq!(c.written(), c.source("Third"));
    Ok(())
}

/// M9: the continuation meets the item's branch deleted on the remote. The
/// synchronization's own request holds the reservation, and `request
/// cancel` of it, then that request once more, lets the reservation go:
/// `request cancel` never reaches a `conflict resolve` request (R-32), so
/// the action names the synchronization's request.
fn c4_resolution_continuation_meets_a_deleted_branch() -> Result<(), FixtureError> {
    let c = Conflict::item(|_| Ok(()))?;
    fixed(fixed(c.w.bare()?.find_reference(CONTEXT))?.delete())?;
    let stopped = c.execute(RESOLVE, c.titled("Resolved"));
    assert_eq!(stopped.code, ResultCode::RemoteBranchDeleted, "{stopped:?}");
    let commit = c.resolutions()[0];
    assert_eq!(stopped.effects.commit_oid, Some(commit.to_string()));
    assert_eq!(actions(&stopped), [RecoveryActionKind::RequestCancel]);
    assert_eq!(
        stopped.recovery[0].arguments.get("request_id"),
        Some(&serde_json::json!(SYNC))
    );
    assert!(reserved(&c.w));
    let asked =
        c.w.service
            .cancel_request(request(SYNC))
            .data
            .ok_or(FixtureError)?;
    assert!(asked.requested);
    let ended = execute(&c.w, SYNC, item_sync(&c.w));
    assert_eq!(ended.code, ResultCode::Cancelled, "{ended:?}");
    assert!(!reserved(&c.w));
    let previewed = republish::prepared(&c.w);
    assert_ne!(previewed.code, ResultCode::Busy, "{previewed:?}");
    Ok(())
}

// The conformance table's row of `conflict resolve`, driven by a case of
// its own (owner, 2026-10-28): its rules that need a finished request each
// build a world with a conflict, which the target's other conformance case
// could not hold within its time.

/// The world a rule of the conformance row is driven in: plain until a
/// request needs the conflict it resolves.
enum Fixture {
    Plain(Box<World>),
    Conflicted(Box<Conflict>),
}

impl Fixture {
    fn world(&self) -> &World {
        match self {
            Self::Plain(w) => w,
            Self::Conflicted(c) => &c.w,
        }
    }
}

/// A world with a trusted server, with `rule` driven through the requests
/// `ask` gives. A request that is to finish (`Same`) or to be compared with
/// one that did (`Changed`) resolves a conflict on primary, made the first
/// time one is asked for. A request that is refused before it is accepted
/// needs none: a duplicate token is refused before anything is read, and a
/// confirmation before the request is prepared.
pub(super) fn conforming(rule: &mut dyn FnMut(&conformance::Subject<'_>)) -> Result<(), String> {
    use conformance::{Ask, NO_SUCH_CONFIRMATION, Recorded, Subject};
    let fixture = std::cell::RefCell::new(Some(Fixture::Plain(Box::new(
        world().map_err(|_| "a world with a server".to_owned())?,
    ))));
    let conflicted = || {
        let mut state = fixture.borrow_mut();
        if let Some(Fixture::Plain(_)) = state.as_ref()
            && let Some(Fixture::Plain(w)) = state.take()
        {
            let c = Conflict::primary_in(*w).expect("a conflict on primary");
            *state = Some(Fixture::Conflicted(Box::new(c)));
        }
    };
    let presented = |id: &str, ask: Ask| -> Vec<u8> {
        if matches!(ask, Ask::Same | Ask::Changed) {
            conflicted();
        }
        let state = fixture.borrow();
        let state = state.as_ref().expect("a fixture");
        let w = state.world();
        let mutation = match (ask, state) {
            (Ask::Same, Fixture::Conflicted(c)) => c.titled("Resolved"),
            // Another source: a changed input under a finished request's
            // ID.
            (Ask::Changed, Fixture::Conflicted(c)) => c.titled("Changed"),
            // The same token twice, refused before the request is accepted.
            (Ask::Refused, _) => resolve(
                w,
                OperationId::new(),
                NO_SUCH_OBSERVATION,
                [
                    (NO_SUCH_TOKEN.to_owned(), "source\n".to_owned()),
                    (NO_SUCH_TOKEN.to_owned(), "source\n".to_owned()),
                ],
            ),
            (Ask::Unconsented, _) => resolve(
                w,
                OperationId::new(),
                NO_SUCH_OBSERVATION,
                [(NO_SUCH_TOKEN.to_owned(), "source\n".to_owned())],
            ),
            (Ask::Same | Ask::Changed, Fixture::Plain(_)) => unreachable!("made above"),
        };
        let confirmation = match ask {
            Ask::Same | Ask::Changed | Ask::Refused => None,
            Ask::Unconsented => Some(NO_SUCH_CONFIRMATION),
        };
        let envelope = presenting(w, id, confirmation, mutation);
        serde_json::to_vec(&envelope).expect("an envelope serializes")
    };
    rule(&Subject {
        // A resolution needs no confirmation.
        outdate: None,
        call: &presented,
        record: &|id| {
            let state = fixture.borrow();
            record(state.as_ref().expect("a fixture").world(), id).map(|record| Recorded {
                finished: record.state == RequestState::Finished,
                image: format!("{record:?}"),
            })
        },
        request_rows: &|| {
            let state = fixture.borrow();
            state
                .as_ref()
                .expect("a fixture")
                .world()
                .db()
                .expect("the database")
                .query_row("SELECT COUNT(*) FROM request_records", [], |row| row.get(0))
                .expect("the request records")
        },
    });
    Ok(())
}

/// The row of `conflict resolve`. The rows of the other commands of this
/// target are driven by its other conformance cases.
fn c4_resolution_conformance() -> Result<(), FixtureError> {
    conformance_row("conflict resolve")
}
