//! `operation resume`: the design's resume table (R-20), a `c4_resume_`
//! case per row, and more than one for rows 3 and 8; the operation list
//! listing what a resume now takes up; and Review Focus 2's resolution of a
//! conflict on a hostile path.
//!
//! The table's row for an operation an `item republish` request holds is
//! Task 5's (ruling P3), and has no case here.
//!
//! A raw peer clone makes the remote diverge, as a second user would. A
//! publication is stopped before it pushes by the test server's disconnect
//! at its advertisement, a resolution by a library failure point, and a
//! Ctrl-C by the call's token, cancelled from its sink. Every call goes
//! through `RepositoryService::execute`, scanned as `call` scans it.

use std::path::{Path, PathBuf};

use results::IntegrationEffect;

use super::*;

pub(crate) const CASES: &[ssh_harness::Case] = &[
    (
        "c4_resume_comment_publication",
        c4_resume_comment_publication,
    ),
    (
        "c4_resume_comment_identity_required",
        c4_resume_comment_identity_required,
    ),
    (
        "c4_resume_resolution_checkpointed",
        c4_resume_resolution_checkpointed,
    ),
    ("c4_resume_outside_repair", c4_resume_outside_repair),
    ("c4_resume_prepared_commit", c4_resume_prepared_commit),
    ("c4_resume_written_paths_only", c4_resume_written_paths_only),
    ("c4_resume_conflict_pending", c4_resume_conflict_pending),
    ("c4_resume_cancelled", c4_resume_cancelled),
    (
        "c4_resume_cancelled_synchronization",
        c4_resume_cancelled_synchronization,
    ),
    (
        "c4_resume_cancelled_with_work_owed",
        c4_resume_cancelled_with_work_owed,
    ),
    ("c4_resume_local_context_sync", c4_resume_local_context_sync),
    ("c4_resume_listed", c4_resume_listed),
    ("c4_resume_hostile_path", c4_resume_hostile_path),
];

const MAIN: &str = "refs/heads/main";
/// The request that publishes the item's editing branch, or brings primary
/// up to the server's, before a case diverges.
const SETUP: &str = "01ARZ3NDEKTSV4RRFFQ69G5FY0";
/// The request that stops: a `comment add`, or the synchronization that
/// stops on a conflict.
const STOPPED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FY2";
/// The request that resolves a conflict.
const RESOLVE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR1";
/// `operation resume`.
const RESUME: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR3";
/// A new synchronization, after a cancelled one.
const LATER: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR4";
/// The comment a case adds to the world's item.
const COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC1";
/// The comment's body: a probe, which no store or envelope may hold.
const COMMENT_BODY: &str = "operation-resume-private-comment-sentinel\n";
/// The author the world's identity writes.
const WORLD_AUTHOR: &str = "Fixture <fixture@example.invalid>";
/// The document of the hostile-path case.
const DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FDH";
/// A canonical document's path holding a terminal escape sequence and a
/// newline (Review Focus 2).
const HOSTILE_DOCUMENT: &str = "docs/hostile-\u{1b}[31mred\nline.md";
/// What the resolution commit's subject begins with (`sync.rs`).
const RESOLUTION_SUBJECT: &str = "Resolve synchronization";

/// A world with a trusted server.
fn world() -> Result<World, FixtureError> {
    let w = World::new()?;
    approve(&w)?;
    Ok(w)
}

/// `operation resume` of `operation` under the request `id`, on `service`,
/// with the world's passphrase, no sink and no cancellation, scanned as
/// `call` scans.
fn resumed_as(
    w: &World,
    service: &RepositoryService,
    id: &str,
    operation: &str,
) -> Envelope<MutationDataDto> {
    let envelope = call(
        w,
        service,
        id,
        resume(w, operation),
        Provider,
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(envelope.command, "operation resume");
    envelope
}

/// `resumed_as` under `RESUME`, on `service`.
fn resumed_on(
    w: &World,
    service: &RepositoryService,
    operation: &str,
) -> Envelope<MutationDataDto> {
    resumed_as(w, service, RESUME, operation)
}

fn resumed(w: &World, operation: &str) -> Envelope<MutationDataDto> {
    resumed_on(w, &w.service, operation)
}

/// `resumed_as` under `id`, on the world's service.
fn resumed_by(w: &World, id: &str, operation: &str) -> Envelope<MutationDataDto> {
    resumed_as(w, &w.service, id, operation)
}

/// A fresh service on the world's data directory.
fn fresh(w: &World) -> RepositoryService {
    RepositoryService::open_at(&w.data()).unwrap()
}

/// What a synchronization taken up by a resume reports once it published
/// `commit`: nothing written or committed by the resume itself.
fn publishing(commit: git2::Oid) -> Effects {
    Effects {
        publication: PublicationEffect::Published,
        discovery: DiscoveryEffect::Current,
        commit_oid: Some(commit.to_string()),
        ..Effects::not_requested()
    }
}

// A comment whose publication stopped.

/// A world with a trusted server and the item's editing branch published.
/// The comment's body is one of its probes.
fn comment_world() -> Result<(World, ItemContext), FixtureError> {
    let mut w = world()?;
    let context = w.context()?;
    w.probes.push(COMMENT_BODY.trim_end().as_bytes().to_vec());
    w.privacy()?;
    assert_eq!(execute(&w, SETUP, item_sync(&w)).code, ResultCode::Ok);
    Ok((w, context))
}

/// `comment add` of `COMMENT` on the world's item.
fn add(w: &World) -> Mutation {
    Mutation::CommentAdd(CommentAddInput {
        root: w.root.clone(),
        kind: AuthoringKind::Ticket,
        id: ITEM.parse().unwrap(),
        comment_id: COMMENT.parse().unwrap(),
        parent_id: None,
        body: COMMENT_BODY.to_owned(),
        created_by: None,
    })
}

fn comment_data(envelope: &Envelope<MutationDataDto>) -> Result<&CommentMutationDto, FixtureError> {
    match envelope.data.as_ref() {
        Some(MutationDataDto::Comment(data)) => Ok(data),
        _ => Err(FixtureError),
    }
}

/// The comment's file, relative to its item's editing worktree.
fn comment_path() -> String {
    format!(".manyhands/comments/{ITEM}/{COMMENT}.md")
}

/// Whether the server's copy of the item's editing branch holds the
/// comment.
fn served(w: &World) -> Result<bool, FixtureError> {
    let bare = w.bare()?;
    let Ok(tip) = bare.refname_to_id(CONTEXT) else {
        return Ok(false);
    };
    Ok(fixed(fixed(bare.find_commit(tip))?.tree())?
        .get_path(Path::new(&comment_path()))
        .is_ok())
}

/// The commits on the item's editing branch that checkpoint the comment.
fn checkpoints(w: &World) -> Result<Vec<git2::Oid>, FixtureError> {
    comment_domain::checkpoints(w, CONTEXT, &fixed(COMMENT.parse())?)
}

/// A comment checkpointed whose publication stopped before it pushed
/// anything, for the server could not be reached: the `comment add`
/// request `STOPPED`, its comment's operation and the synchronization
/// that publishes it.
struct StoppedComment {
    w: World,
    context: ItemContext,
    comment: OperationId,
    synchronization: OperationId,
    checkpoint: git2::Oid,
}

impl StoppedComment {
    fn new() -> Result<Self, FixtureError> {
        let (w, context) = comment_world()?;
        w.server.disconnect_at(FixtureBoundary::Advertisement);
        let stopped = execute(&w, STOPPED, add(&w));
        w.server.clear_fault();
        assert_eq!(
            stopped.code,
            ResultCode::TransportUnavailable,
            "{stopped:?}"
        );
        assert_eq!(stopped.outcome, Outcome::Partial);
        let data = comment_data(&stopped)?;
        assert_eq!(data.publication, PublicationEffect::Pending);
        let synchronization =
            OperationId::parse(&data.synchronization_id).map_err(|_| FixtureError)?;
        let comment = OperationId::parse(stopped.operation_id.as_deref().ok_or(FixtureError)?)
            .map_err(|_| FixtureError)?;
        // Kept for the same request (R-34).
        assert_eq!(
            record(&w, STOPPED).ok_or(FixtureError)?.state,
            RequestState::Accepted
        );
        let [checkpoint] = checkpoints(&w)?[..] else {
            return Err(FixtureError);
        };
        assert!(!served(&w)?);
        Ok(Self {
            w,
            context,
            comment,
            synchronization,
            checkpoint,
        })
    }

    fn file(&self) -> PathBuf {
        self.context.worktree.join(comment_path())
    }
}

/// Row 1: a comment's synchronization, or the comment's own operation,
/// with its checkpoint present and its publication pending, not cancelled:
/// the comment's synchronizer-driven retry with `restart`, which publishes
/// the comment and never writes it. The data is the comment's.
fn c4_resume_comment_publication() -> Result<(), FixtureError> {
    for by_synchronization in [false, true] {
        let c = StoppedComment::new()?;
        let w = &c.w;
        let named = if by_synchronization {
            c.synchronization
        } else {
            c.comment
        }
        .to_string();
        let written = fixed(std::fs::read(c.file()))?;
        let pushes = accepted(&w.server);

        let resumed = resumed(w, &named);
        assert_eq!(
            (resumed.outcome, resumed.code),
            (Outcome::Success, ResultCode::Ok),
            "{by_synchronization}: {resumed:?}"
        );
        // It published the comment, once, and wrote and committed nothing:
        // what it claims is what the synchronization did.
        assert_eq!(resumed.effects, publishing(c.checkpoint));
        assert!(resumed.recovery.is_empty());
        assert_eq!(resumed.operation_id, None);
        let data = comment_data(&resumed)?;
        assert_eq!(
            *data,
            CommentMutationDto {
                r#type: CommentMutationType::Comment,
                item_id: ITEM.to_owned(),
                comment_id: COMMENT.to_owned(),
                parent_id: None,
                created_by: Some(WORLD_AUTHOR.to_owned()),
                synchronization_id: c.synchronization.to_string(),
                publication: PublicationEffect::Published,
                pending_reason: None,
                conflict: None,
            }
        );
        assert!(served(w)?);
        assert_eq!(accepted(&w.server), pushes + 1);
        assert_eq!(checkpoints(w)?, [c.checkpoint]);
        assert_eq!(fixed(std::fs::read(c.file()))?, written);
        assert_eq!(phase(w, c.synchronization)?, "completed");
        assert!(!reserved(w));
        let stored = record(w, RESUME).ok_or(FixtureError)?;
        assert_eq!(stored.state, RequestState::Finished);
        assert!(stored.operations.is_empty());

        // Replayed from its record, data and all: no connection.
        let opened = connections(w);
        assert_eq!(execute(w, RESUME, resume(w, &named)), resumed);
        assert_eq!(connections(w), opened);
        // The comment's own request, sent again, finds it published, and
        // writes and pushes nothing.
        let again = execute(w, STOPPED, add(w));
        assert_eq!(again.code, ResultCode::Ok, "{again:?}");
        assert_eq!(
            comment_data(&again)?.publication,
            PublicationEffect::Published
        );
        assert_eq!(accepted(&w.server), pushes + 1);
        assert_eq!(checkpoints(w)?, [c.checkpoint]);
        assert_eq!(
            record(w, STOPPED).ok_or(FixtureError)?.state,
            RequestState::Finished
        );
    }
    Ok(())
}

/// Row 2: a comment request that stopped at `identity_required`. Its
/// binding was released (R-34), so nothing is taken up: the resume answers
/// its completed local row as F2 does, and the same `comment add` request
/// writes the comment.
fn c4_resume_comment_identity_required() -> Result<(), FixtureError> {
    let (w, context) = comment_world()?;
    comment_domain::remove_the_identity(&w)?;
    let stopped = execute(&w, STOPPED, add(&w));
    assert_eq!(stopped.code, ResultCode::IdentityRequired);
    let operation = stopped.operation_id.clone().ok_or(FixtureError)?;
    let opened = connections(&w);

    let noop = resumed(&w, &operation);
    assert_eq!(
        (noop.outcome, noop.code),
        (Outcome::Noop, ResultCode::AlreadyApplied),
        "{noop:?}"
    );
    assert_eq!(noop.effects, Effects::not_requested());
    assert!(noop.data.is_none());
    assert!(noop.recovery.is_empty());
    assert_no_record_of(&w, RESUME, &noop);
    assert!(!context.worktree.join(comment_path()).exists());
    assert_eq!(connections(&w), opened);

    // The same `comment add` request writes the comment, once.
    comment_domain::restore_the_identity(&w)?;
    let published = execute(&w, STOPPED, add(&w));
    assert_eq!(published.code, ResultCode::Ok, "{published:?}");
    assert_eq!(published.operation_id.as_deref(), Some(operation.as_str()));
    assert_eq!(checkpoints(&w)?.len(), 1);
    assert!(served(&w)?);
    Ok(())
}

// A synchronization whose conflict is resolved, repaired, or being
// resolved.

/// The peer commits `bytes` at `path` on `reference` as the server holds
/// it, and pushes it there. What the server holds is fetched to a ref of
/// the fixture's own first, so the peer's checked-out branch is never
/// fetched into.
fn remote_commit(
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

/// A world whose `repo sync` stopped on a conflict at one canonical path
/// of primary, with all three sides, and what `conflict show` gave of it.
struct Conflict {
    w: World,
    /// The synchronization.
    operation: OperationId,
    /// The conflicted path.
    path: String,
    /// The source the path is given under a title.
    source: fn(&str) -> String,
    local: git2::Oid,
    incoming: git2::Oid,
    observation: String,
    token: String,
}

/// The source of `merging::PRIMARY_TICKET`, titled `title`.
fn ticket(title: &str) -> String {
    merging::ticket_source(merging::PRIMARY_TICKET, title)
}

/// The source of `DOCUMENT`, titled `title`.
fn document(title: &str) -> String {
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: {DOCUMENT}\n\
         title: {title}\n---\nBody of {title}.\n"
    )
}

impl Conflict {
    /// A conflict in `merging::PRIMARY_TICKET`'s source.
    fn primary() -> Result<Self, FixtureError> {
        Self::at(
            world()?,
            &merging::ticket_path(merging::PRIMARY_TICKET),
            ticket,
        )
    }

    /// The peer adds `path` with `source("Base")` and primary is brought up
    /// to it; then each side gives it another title, and `repo sync` stops.
    fn at(w: World, path: &str, source: fn(&str) -> String) -> Result<Self, FixtureError> {
        remote_commit(&w, MAIN, path, source("Base").as_bytes())?;
        assert_current(&execute(&w, SETUP, repo_sync(&w)));
        let local = commit_in(&w.root, MAIN, path, source("Local").as_bytes())?;
        let incoming = remote_commit(&w, MAIN, path, source("Incoming").as_bytes())?;
        let stopped = execute(&w, STOPPED, repo_sync(&w));
        assert_eq!(stopped.code, ResultCode::RecoveryRequired, "{stopped:?}");
        let operation = OperationId::parse(stopped.operation_id.as_deref().ok_or(FixtureError)?)
            .map_err(|_| FixtureError)?;
        let mut conflict = Self {
            w,
            operation,
            path: path.to_owned(),
            source,
            local,
            incoming,
            observation: String::new(),
            token: String::new(),
        };
        let shown = conflict.show();
        assert_eq!(shown.state, ConflictState::ConflictPending);
        assert_eq!(shown.paths.len(), 1);
        conflict.observation = shown.observation.ok_or(FixtureError)?;
        conflict.token = shown.paths[0].token.clone();
        Ok(conflict)
    }

    fn show(&self) -> ConflictDto {
        let repo = self.w.service.resolve_repository(&self.w.root).unwrap();
        self.w.service.show_conflict(&repo, self.operation).unwrap()
    }

    /// `conflict resolve` of the path with its source titled `title`.
    fn titled(&self, title: &str) -> Mutation {
        Mutation::ConflictResolve(ConflictResolveInput {
            root: self.w.root.clone(),
            operation_id: self.operation,
            observation: self.observation.clone(),
            resolutions: vec![ConflictResolutionInput {
                token: self.token.clone(),
                source: (self.source)(title),
            }],
        })
    }

    /// One call of `conflict resolve` under `RESOLVE`, on `service`.
    fn resolve_on(&self, service: &RepositoryService) -> Envelope<MutationDataDto> {
        let envelope = call(
            &self.w,
            service,
            RESOLVE,
            self.titled("Resolved"),
            Provider,
            &mut NoProgress,
            &CancellationToken::new(),
        );
        assert_eq!(envelope.command, "conflict resolve");
        envelope
    }

    /// `conflict resolve` stopped at `point` by a service that fails there.
    fn interrupted_at(&self, point: FailurePoint) {
        let failing =
            RepositoryService::open_at_with_failure_point_for_testing(&self.w.data(), point)
                .unwrap();
        let stopped = self.resolve_on(&failing);
        assert!(stopped.code.failure_class().is_some(), "{point:?}");
    }

    fn tip(&self) -> git2::Oid {
        self.w.repo().unwrap().refname_to_id(MAIN).unwrap()
    }

    fn served(&self) -> git2::Oid {
        self.w.bare().unwrap().refname_to_id(MAIN).unwrap()
    }

    /// What the path holds in the worktree.
    fn written(&self) -> String {
        std::fs::read_to_string(self.w.root.join(&self.path)).unwrap()
    }

    /// The resolution commits on primary, newest first.
    fn resolutions(&self) -> Vec<git2::Oid> {
        let repo = self.w.repo().unwrap();
        let mut walk = repo.revwalk().unwrap();
        walk.push(self.tip()).unwrap();
        walk.map(Result::unwrap)
            .filter(|oid| {
                repo.find_commit(*oid)
                    .unwrap()
                    .summary()
                    .is_some_and(|summary| summary.starts_with(RESOLUTION_SUBJECT))
            })
            .collect()
    }

    /// The phase of the integration step whose conflict this is.
    fn step(&self) -> String {
        fixed(self.w.db().unwrap().query_row(
            "SELECT step.phase FROM remote_integration_steps step
               JOIN remote_operation_records operation ON operation.id = step.operation_record_id
              WHERE operation.operation_ulid = ?1 AND step.conflict_digest IS NOT NULL
              ORDER BY step.window_number DESC, step.ordinal DESC LIMIT 1",
            [self.operation.to_string()],
            |row| row.get(0),
        ))
        .unwrap()
    }

    /// How many resolution attempts, and records of their owners, the
    /// registry holds.
    fn attempts(&self) -> (i64, i64) {
        let db = self.w.db().unwrap();
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
    fn candidate(&self) -> git2::Oid {
        let candidate: String = self
            .w
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

    fn resume(&self) -> Envelope<MutationDataDto> {
        resumed_on(&self.w, &fresh(&self.w), &self.operation.to_string())
    }
}

/// A resolution checkpointed whose publication stopped before it pushed
/// anything: the resolution commit.
fn checkpointed(c: &Conflict) -> git2::Oid {
    c.w.server.disconnect_at(FixtureBoundary::Advertisement);
    let stopped = c.resolve_on(&c.w.service);
    c.w.server.clear_fault();
    assert_eq!(
        stopped.code,
        ResultCode::TransportUnavailable,
        "{stopped:?}"
    );
    assert_eq!(c.show().state, ConflictState::Resolved);
    let [commit] = c.resolutions()[..] else {
        panic!("one resolution commit");
    };
    commit
}

/// Row 3, a resolution checkpointed: a restart with the merging settings
/// publishes it. A synchronization reports no data.
fn c4_resume_resolution_checkpointed() -> Result<(), FixtureError> {
    let c = Conflict::primary()?;
    let commit = checkpointed(&c);
    let pushes = accepted(&c.w.server);
    let resumed = c.resume();
    assert_eq!(
        (resumed.outcome, resumed.code),
        (Outcome::Success, ResultCode::Ok),
        "{resumed:?}"
    );
    // The resolution commit is the merge the synchronization integrated.
    assert_eq!(
        resumed.effects,
        Effects {
            integration: IntegrationEffect::Complete,
            ..publishing(commit)
        }
    );
    assert!(resumed.data.is_none());
    assert!(resumed.recovery.is_empty());
    assert_eq!(c.served(), commit);
    assert_eq!(accepted(&c.w.server), pushes + 1);
    assert_eq!(c.resolutions(), [commit]);
    assert_eq!(c.show().state, ConflictState::Complete);
    Ok(())
}

/// Row 3, an outside repair: the operator commits a two-parent merge of
/// a file that is no canonical content, and the restart accepts it and
/// publishes it.
fn c4_resume_outside_repair() -> Result<(), FixtureError> {
    let w = world()?;
    let repo = w.repo()?;
    let local = commit_in(&w.root, MAIN, "conflict", b"local conflict\n")?;
    let incoming = remote_commit(&w, MAIN, "conflict", b"incoming conflict\n")?;
    let stopped = execute(&w, STOPPED, repo_sync(&w));
    assert_eq!(stopped.code, ResultCode::RecoveryRequired);
    let Some(MutationDataDto::SynchronizationConflict(data)) = &stopped.data else {
        return Err(FixtureError);
    };
    assert_eq!(
        data.conflict.next_action,
        ConflictNextAction::ExternalRepair
    );
    let operation = stopped.operation_id.clone().ok_or(FixtureError)?;
    // The whole committed merge, as the operator's own tools leave it.
    let mut index = fixed(git2::Index::new())?;
    fixed(index.read_tree(&fixed(fixed(repo.find_commit(local))?.tree())?))?;
    fixed(index.add(&git2::IndexEntry {
        ctime: git2::IndexTime::new(0, 0),
        mtime: git2::IndexTime::new(0, 0),
        dev: 0,
        ino: 0,
        mode: 0o100644,
        uid: 0,
        gid: 0,
        file_size: 0,
        id: fixed(repo.blob(b"repaired conflict\n"))?,
        flags: 0,
        flags_extended: 0,
        path: b"conflict".to_vec(),
    }))?;
    let tree = fixed(repo.find_tree(fixed(index.write_tree_to(&repo))?))?;
    let signature = fixed(git2::Signature::now("Fixture", "fixture@example.invalid"))?;
    let parents = [
        fixed(repo.find_commit(local))?,
        fixed(repo.find_commit(incoming))?,
    ];
    let repaired = fixed(repo.commit(
        None,
        &signature,
        &signature,
        "repair outside",
        &tree,
        &[&parents[0], &parents[1]],
    ))?;
    fixed(
        repo.checkout_tree(
            &fixed(repo.find_object(repaired, None))?,
            Some(
                git2::build::CheckoutBuilder::new()
                    .force()
                    .remove_untracked(true),
            ),
        ),
    )?;
    fixed(repo.reference(MAIN, repaired, true, "external repair"))?;
    let mut index = fixed(repo.index())?;
    fixed(index.read_tree(&fixed(fixed(repo.find_commit(repaired))?.tree())?))?;
    fixed(index.write())?;
    fixed(repo.cleanup_state())?;
    let pushes = accepted(&w.server);

    let resumed = resumed(&w, &operation);
    assert_eq!(
        (resumed.outcome, resumed.code),
        (Outcome::Success, ResultCode::Ok),
        "{resumed:?}"
    );
    assert_eq!(resumed.effects.publication, PublicationEffect::Published);
    assert_eq!(resumed.effects.commit_oid, Some(repaired.to_string()));
    assert!(resumed.data.is_none());
    assert_eq!(fixed(w.bare()?.refname_to_id(MAIN))?, repaired);
    assert_eq!(accepted(&w.server), pushes + 1);
    Ok(())
}

/// Row 4: an attempt whose commit is prepared, or whose ref moved, is
/// finished from durable state alone and published: the prepared commit,
/// and no second one. Task 4a's `c4_resolution_resume_finishes_a_prepared_commit`
/// added the route; this is the row's guard here.
fn c4_resume_prepared_commit() -> Result<(), FixtureError> {
    for point in [
        FailurePoint::ResolutionAfterCandidatePrepared,
        FailurePoint::ResolutionAfterRefTransition,
    ] {
        let c = Conflict::primary()?;
        c.interrupted_at(point);
        let prepared = c.candidate();
        let pushes = accepted(&c.w.server);
        let resumed = c.resume();
        assert_eq!(
            (resumed.outcome, resumed.code),
            (Outcome::Success, ResultCode::Ok),
            "{point:?}: {resumed:?}"
        );
        assert_eq!(resumed.effects.commit_oid, Some(prepared.to_string()));
        assert_eq!(resumed.effects.publication, PublicationEffect::Published);
        assert!(resumed.data.is_none());
        assert_eq!(c.resolutions(), [prepared], "{point:?}");
        assert_eq!(c.served(), prepared);
        assert_eq!(accepted(&c.w.server), pushes + 1);
    }
    Ok(())
}

/// Row 5: an attempt that has written paths only (`ResolutionPrepared`).
/// Only its own request goes on with it, or a new `conflict resolve` that
/// supersedes it: `original_request_required`, naming the attempt's
/// request, as the gate's local resume does (decision 20). Nothing is
/// written and nothing is connected to.
fn c4_resume_written_paths_only() -> Result<(), FixtureError> {
    let c = Conflict::primary()?;
    c.interrupted_at(FailurePoint::ResolutionAfterPathWrite);
    assert_eq!(c.step(), "resolution_prepared");
    let before = (c.tip(), c.written(), c.step(), c.attempts());
    let opened = connections(&c.w);

    let refused = c.resume();
    assert_eq!(
        (refused.outcome, refused.code),
        (Outcome::Blocked, ResultCode::OriginalRequestRequired),
        "{refused:?}"
    );
    assert_eq!(refused.effects, Effects::not_requested());
    assert!(refused.data.is_none());
    assert_eq!(actions(&refused), [RecoveryActionKind::RequestRetry]);
    assert_eq!(refused.recovery[0].arguments["request_id"], RESOLVE);
    assert_no_record_of(&c.w, RESUME, &refused);
    assert_eq!((c.tip(), c.written(), c.step(), c.attempts()), before);
    assert_eq!(c.written(), ticket("Resolved"));
    assert_eq!(connections(&c.w), opened);

    // The request it names finishes it, and publishes.
    let pushes = accepted(&c.w.server);
    let finished = c.resolve_on(&fresh(&c.w));
    assert_eq!(finished.code, ResultCode::Ok, "{finished:?}");
    let [commit] = c.resolutions()[..] else {
        return Err(FixtureError);
    };
    assert_eq!(c.served(), commit);
    assert_eq!(accepted(&c.w.server), pushes + 1);
    Ok(())
}

/// Row 7: a synchronization whose conflict is still pending is read again
/// with no transport: `recovery_required`, with the conflict's next step.
fn c4_resume_conflict_pending() -> Result<(), FixtureError> {
    let c = Conflict::primary()?;
    let opened = connections(&c.w);
    let pushes = accepted(&c.w.server);
    let pending = c.resume();
    assert_eq!(
        (pending.outcome, pending.code),
        (Outcome::Blocked, ResultCode::RecoveryRequired),
        "{pending:?}"
    );
    assert_eq!(pending.effects.integration, IntegrationEffect::Pending);
    assert_eq!(pending.effects.commit_oid, None);
    assert_eq!(
        pending.data,
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
    assert_eq!(actions(&pending), [RecoveryActionKind::RepoInspect]);
    assert_no_record_of(&c.w, RESUME, &pending);
    assert_eq!(c.step(), "conflict_pending");
    assert_eq!(c.tip(), c.local);
    assert_eq!(connections(&c.w), opened);
    assert_eq!(accepted(&c.w.server), pushes);
    Ok(())
}

/// Row 8: a cancelled operation, here a comment's publication cancelled by
/// a Ctrl-C before its push, whose row owes nothing. It is left to
/// inspect, `recovery_required`, by the synchronization's ID or the
/// comment's, and nothing takes it up (controller ruling, Task 4b row 8:
/// option B). A new request completes the work (fact 56). Every other
/// cancelled operation answers as F2: see the two cases below.
fn c4_resume_cancelled() -> Result<(), FixtureError> {
    for by_synchronization in [false, true] {
        let (w, _) = comment_world()?;
        let token = CancellationToken::new();
        let mut sink = CancelAt {
            stage: ProgressStage::Pushing,
            token: token.clone(),
            seen: Vec::new(),
        };
        let cancelled = call(
            &w,
            &w.service,
            STOPPED,
            add(&w),
            Provider,
            &mut sink,
            &token,
        );
        assert_eq!(cancelled.code, ResultCode::Cancelled, "{cancelled:?}");
        let data = comment_data(&cancelled)?;
        let synchronization =
            OperationId::parse(&data.synchronization_id).map_err(|_| FixtureError)?;
        assert_eq!(phase(&w, synchronization)?, "cancelled");
        // The row owes nothing: F2 would call it ended.
        assert_eq!(owes(&w, synchronization)?, (false, false));
        let named = if by_synchronization {
            synchronization.to_string()
        } else {
            cancelled.operation_id.clone().ok_or(FixtureError)?
        };
        let opened = connections(&w);
        let pushes = accepted(&w.server);

        let left = resumed(&w, &named);
        assert_eq!(
            (left.outcome, left.code),
            (Outcome::Blocked, ResultCode::RecoveryRequired),
            "{by_synchronization}: {left:?}"
        );
        assert_eq!(left.effects, Effects::not_requested());
        assert!(left.data.is_none());
        assert_eq!(actions(&left), [RecoveryActionKind::RepoInspect]);
        assert_no_record_of(&w, RESUME, &left);
        assert_eq!(phase(&w, synchronization)?, "cancelled");
        assert_eq!(connections(&w), opened);
        assert_eq!(accepted(&w.server), pushes);
        assert!(!served(&w)?);

        // A new synchronization publishes the comment.
        let later = execute(&w, LATER, item_sync(&w));
        assert_eq!(later.code, ResultCode::Ok, "{later:?}");
        assert!(served(&w)?);
    }
    Ok(())
}

/// What a remote operation's row owes: its index hand-off, and its
/// reconciliation.
fn owes(w: &World, operation: OperationId) -> Result<(bool, bool), FixtureError> {
    fixed(w.db()?.query_row(
        "SELECT index_pending, reconciliation_required FROM remote_operation_records
          WHERE operation_ulid = ?1",
        [operation.to_string()],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ))
}

/// Row 8, F2 unchanged: a plain `repo sync` cancelled by a Ctrl-C before
/// its push, whose row owes nothing, has ended, and its resume is a no-op,
/// `already_applied`. A new synchronization publishes the commit.
fn c4_resume_cancelled_synchronization() -> Result<(), FixtureError> {
    let w = world()?;
    let commit = advance(&w.repo()?, MAIN, "local-advance")?;
    let token = CancellationToken::new();
    let mut sink = CancelAt {
        stage: ProgressStage::Pushing,
        token: token.clone(),
        seen: Vec::new(),
    };
    let cancelled = call(
        &w,
        &w.service,
        STOPPED,
        repo_sync(&w),
        Provider,
        &mut sink,
        &token,
    );
    assert_eq!(cancelled.code, ResultCode::Cancelled, "{cancelled:?}");
    let operation = OperationId::parse(cancelled.operation_id.as_deref().ok_or(FixtureError)?)
        .map_err(|_| FixtureError)?;
    assert_eq!(phase(&w, operation)?, "cancelled");
    assert_eq!(owes(&w, operation)?, (false, false));
    let opened = connections(&w);
    let pushes = accepted(&w.server);

    let noop = resumed(&w, &operation.to_string());
    assert_eq!(
        (noop.outcome, noop.code),
        (Outcome::Noop, ResultCode::AlreadyApplied),
        "{noop:?}"
    );
    assert_eq!(noop.effects, Effects::not_requested());
    assert!(noop.data.is_none());
    assert!(noop.recovery.is_empty());
    assert_no_record_of(&w, RESUME, &noop);
    assert_eq!(phase(&w, operation)?, "cancelled");
    assert_eq!(connections(&w), opened);
    assert_eq!(accepted(&w.server), pushes);

    // A new synchronization publishes the commit.
    let later = execute(&w, LATER, repo_sync(&w));
    assert_eq!(later.code, ResultCode::Ok, "{later:?}");
    assert_eq!(fixed(w.bare()?.refname_to_id(MAIN))?, commit);
    Ok(())
}

/// Row 8, F2 unchanged: a synchronization cancelled with work outstanding,
/// for Git may hold something of what it began, is left to inspect,
/// `recovery_required`. Here a `repo sync` holds its reservation, and a
/// resume that took it up is stopped by its token.
fn c4_resume_cancelled_with_work_owed() -> Result<(), FixtureError> {
    let w = world()?;
    advance(&w.repo()?, MAIN, "local-advance")?;
    let held = held_before_a_push(&w, STOPPED);
    let operation = OperationId::parse(held.operation_id.as_deref().ok_or(FixtureError)?)
        .map_err(|_| FixtureError)?;
    let token = CancellationToken::new();
    let mut sink = CancelAt {
        stage: ProgressStage::Fetching,
        token: token.clone(),
        seen: Vec::new(),
    };
    let cancelled = call(
        &w,
        &w.service,
        RESUME,
        resume(&w, &operation.to_string()),
        Provider,
        &mut sink,
        &token,
    );
    assert_eq!(cancelled.code, ResultCode::Cancelled, "{cancelled:?}");
    assert_eq!(phase(&w, operation)?, "cancelled");
    assert_ne!(owes(&w, operation)?, (false, false));
    let opened = connections(&w);
    let pushes = accepted(&w.server);

    let left = resumed_by(&w, LATER, &operation.to_string());
    assert_eq!(
        (left.outcome, left.code),
        (Outcome::Blocked, ResultCode::RecoveryRequired),
        "{left:?}"
    );
    assert_eq!(left.effects, Effects::not_requested());
    assert!(left.data.is_none());
    assert_eq!(actions(&left), [RecoveryActionKind::RepoInspect]);
    assert_no_record_of(&w, LATER, &left);
    assert_eq!(phase(&w, operation)?, "cancelled");
    assert_eq!(connections(&w), opened);
    assert_eq!(accepted(&w.server), pushes);
    Ok(())
}

/// Row 9: a local-context sync is the gate's to route, and C4 changes
/// nothing in it (fact 9, decision 19). The resume of one stopped on
/// Task 1's conflict answers exactly as the gate's routing answered it
/// before C4's routes, with the gate's data, and never reaches the remote.
fn c4_resume_local_context_sync() -> Result<(), FixtureError> {
    let w = world()?;
    let operation = leave_local_sync_conflict(&w);
    let opened = connections(&w);
    let pushes = accepted(&w.server);
    let left = resumed(&w, &operation.to_string());
    assert_eq!(
        (left.outcome, left.code),
        (Outcome::Partial, ResultCode::RecoveryRequired),
        "{left:?}"
    );
    assert_eq!(
        left.effects,
        Effects {
            write: results::WriteEffect::Written,
            checkpoint: results::CheckpointEffect::Pending,
            discovery: DiscoveryEffect::Pending,
            integration: IntegrationEffect::Pending,
            ..Effects::not_requested()
        }
    );
    assert_eq!(
        left.data,
        Some(MutationDataDto::LocalContextSync(
            LocalContextSyncMutationDto {
                data_type: LocalContextSyncDataType::LocalContextSync,
                item_id: OTHER_ITEM.to_owned(),
                kind: "ticket".to_owned(),
                operation_id: Some(operation.to_string()),
                resolution_request_id: None,
                required_request_id: None,
                result_oid: None,
                state: LocalContextSyncState::ConflictPending,
                reason: LocalContextSyncReason::UnsupportedConflict,
                next_action: LocalContextSyncNextAction::ExternalRepair,
                eligibility: Some(LocalContextSyncEligibility::ExternalResolutionRequired),
                cancel_acknowledged: false,
            }
        ))
    );
    assert_eq!(actions(&left), [RecoveryActionKind::RepoInspect]);
    assert_eq!(connections(&w), opened);
    assert_eq!(accepted(&w.server), pushes);
    Ok(())
}

/// What `list_operations` gives of `operation`, when it lists it.
fn listed(w: &World, operation: OperationId) -> Option<OperationDto> {
    let repo = w.service.resolve_repository(&w.root).unwrap();
    w.service
        .list_operations(&repo)
        .unwrap()
        .items
        .into_iter()
        .find(|listed| listed.operation_id.as_deref() == Some(&*operation.to_string()))
}

/// `list_operations` lists the operations a resume now acts on: a
/// comment's synchronization whose publication stopped, and a
/// synchronization whose resolution is checkpointed. A resume then
/// publishes each.
fn c4_resume_listed() -> Result<(), FixtureError> {
    let c = StoppedComment::new()?;
    let comment = listed(&c.w, c.synchronization).ok_or(FixtureError)?;
    assert_eq!(comment.family, OperationFamily::Remote);
    assert_eq!(comment.action, OperationAction::SynchronizeContext);
    assert_eq!(comment.item_id.as_deref(), Some(ITEM));
    assert_eq!(
        resumed(&c.w, &c.synchronization.to_string()).code,
        ResultCode::Ok
    );
    assert!(served(&c.w)?);

    let r = Conflict::primary()?;
    let commit = checkpointed(&r);
    let resolution = listed(&r.w, r.operation).ok_or(FixtureError)?;
    assert_eq!(resolution.family, OperationFamily::Remote);
    assert_eq!(resolution.action, OperationAction::SynchronizePrimary);
    assert_eq!(r.resume().code, ResultCode::Ok);
    assert_eq!(r.served(), commit);
    Ok(())
}

/// The paths a commit's tree changes from `parent`'s, as raw bytes.
fn changed_paths(repo: &git2::Repository, parent: git2::Oid, commit: git2::Oid) -> Vec<Vec<u8>> {
    let old = repo.find_commit(parent).unwrap().tree().unwrap();
    let new = repo.find_commit(commit).unwrap().tree().unwrap();
    let diff = repo
        .diff_tree_to_tree(Some(&old), Some(&new), None)
        .unwrap();
    diff.deltas()
        .map(|delta| {
            delta
                .new_file()
                .path_bytes()
                .or_else(|| delta.old_file().path_bytes())
                .unwrap()
                .to_vec()
        })
        .collect()
}

/// Review Focus 2: a conflict on a canonical document whose path holds a
/// terminal escape sequence and a newline. Its token resolves through
/// `execute`, and the resolution commit changes that path and no other,
/// from either parent.
fn c4_resume_hostile_path() -> Result<(), FixtureError> {
    // Windows cannot name such a file, so no conflict can hold one there.
    if cfg!(windows) {
        return Ok(());
    }
    let c = Conflict::at(world()?, HOSTILE_DOCUMENT, document)?;
    let shown = c.show();
    assert_eq!(shown.paths[0].kind, ConflictPathKind::Document);
    assert_eq!(
        shown.paths[0].display_path,
        "docs/hostile-\\x1b[31mred\\x0aline.md"
    );
    let pushes = accepted(&c.w.server);
    let resolved = c.resolve_on(&c.w.service);
    assert_eq!(resolved.code, ResultCode::Ok, "{resolved:?}");
    let [commit] = c.resolutions()[..] else {
        return Err(FixtureError);
    };
    let repo = c.w.repo()?;
    let parents: Vec<_> = fixed(repo.find_commit(commit))?.parent_ids().collect();
    assert_eq!(parents, [c.local, c.incoming]);
    for parent in [c.local, c.incoming] {
        assert_eq!(
            changed_paths(&repo, parent, commit),
            [HOSTILE_DOCUMENT.as_bytes().to_vec()]
        );
    }
    assert_eq!(c.written(), document("Resolved"));
    // Nothing else was written in the worktree: no path the newline could
    // have split the name into.
    let statuses = fixed(
        repo.statuses(Some(
            git2::StatusOptions::new()
                .include_untracked(true)
                .recurse_untracked_dirs(true),
        )),
    )?;
    let dirty: Vec<_> = statuses
        .iter()
        .map(|entry| entry.path_bytes().to_vec())
        .collect();
    assert!(dirty.is_empty(), "{dirty:?}");
    assert_eq!(c.served(), commit);
    assert_eq!(accepted(&c.w.server), pushes + 1);
    Ok(())
}
