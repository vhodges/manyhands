//! Decision 34: `conflict show` and `conflict resolve` reach a promotion's or a
//! closure's context child, by the child's own ID or the parent's, and the same
//! lifecycle request then finishes. A `c5_lifecycle_child_conflict_` case per
//! row and verb, each on `c5_cancel`'s retained conflict (`conflicted`).

use std::path::Path;

use super::*;
use results::{CheckpointEffect, CleanupEffect, Outcome};

pub(crate) const CASES: &[ssh_harness::Case] = both_verbs!["c5_lifecycle_child_conflict_";
    "shown_by_either_id" => shown_by_either_id,
    "resolved_then_the_same_request" => resolved_then_the_same_request,
    "stale_token" => stale_token,
    "external_only_stays_external" => external_only_stays_external,
    "resolved_under_a_cancelled_parent" => resolved_under_a_cancelled_parent,
    "replay" => replay,
    "unconsented_resolution" => unconsented_resolution,
    "deleted_branch_after_a_resolution_loops" => deleted_branch_after_a_resolution_loops,
];

/// The lifecycle request, and the resolutions'.
const FIRST: &str = super::FIRST;
const RESOLVE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FK1";
const AGAIN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FK2";
const RESOLUTION: &str = "conflict_resolution_mutation.schema.json";

/// The conflict as `conflict show` reads it, by `operation`.
fn shown(w: &World, operation: OperationId) -> Result<ConflictDto, ReadError> {
    let repo = w.service.resolve_repository(&w.root).unwrap();
    w.service.show_conflict(&repo, operation)
}

/// A resolution of every path of `shown`, each to its local side.
fn local_sides(shown: &ConflictDto) -> Vec<(String, String)> {
    shown
        .paths
        .iter()
        .map(|path| (path.token.clone(), path.local.clone().expect("a side")))
        .collect()
}

/// `conflict resolve` of `operation` under `id`, its data checked against its
/// schema.
fn resolved(
    w: &World,
    id: &str,
    operation: OperationId,
    observation: &str,
    resolutions: Vec<(String, String)>,
) -> Envelope<MutationDataDto> {
    let envelope = w.service.execute(
        MutationCall {
            request_id: request(id),
            confirmation: None,
            mutation: Mutation::ConflictResolve(ConflictResolveInput {
                root: w.root.clone(),
                operation_id: operation,
                observation: observation.to_owned(),
                resolutions: resolutions
                    .into_iter()
                    .map(|(token, source)| ConflictResolutionInput { token, source })
                    .collect(),
            }),
        },
        &mut SessionCredentials::new(Provider),
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(envelope.command, "conflict resolve");
    let value = serde_json::to_value(&envelope).expect("an envelope serializes");
    schema::check_published("envelope.schema.json", &value).expect("the envelope schema");
    if !value["data"].is_null() {
        schema::check_published(RESOLUTION, &value["data"]).expect("the resolution schema");
    }
    envelope
}

/// The resolution's data.
fn resolution_of(envelope: &Envelope<MutationDataDto>) -> &ConflictResolutionMutationDto {
    match envelope.data.as_ref() {
        Some(MutationDataDto::ConflictResolution(data)) => data,
        other => panic!("resolution data: {other:?}"),
    }
}

/// The ticket's closure fields at `reference` of `repo`.
fn closure_fields(repo: &git2::Repository, reference: &str) -> Result<Vec<String>, FixtureError> {
    let tree = fixed(fixed(fixed(repo.find_reference(reference))?.peel_to_commit())?.tree())?;
    let Ok(entry) = tree.get_path(Path::new(&format!(".manyhands/tickets/{ITEM}/ticket.md")))
    else {
        return Ok(Vec::new());
    };
    let blob = fixed(repo.find_blob(entry.id()))?;
    Ok(String::from_utf8_lossy(blob.content())
        .lines()
        .filter(|line| line.starts_with("closed_at:") || line.starts_with("closed_by:"))
        .map(str::to_owned)
        .collect())
}

/// The child's conflict stands as it was: nothing resolved, written or pushed.
fn assert_untouched(w: &World, verb: Verb, child: OperationId, served: git2::Oid) {
    let index = git2::Repository::open(verb.worktree(&w.root))
        .and_then(|repo| repo.index())
        .expect("the context's index");
    assert!(index.has_conflicts(), "the conflict stands");
    assert_eq!(row_of(w, child).unwrap(), ("interrupted".to_owned(), false));
    assert_eq!(
        w.bare().unwrap().refname_to_id(verb.branch()).unwrap(),
        served,
        "nothing pushed"
    );
}

/// The same lifecycle request finishes: `ok`, cleanup complete, one merge on
/// primary, the record finished. For a closure, the closure's fields are
/// written once, by this call, with the consented identity, and the remote's
/// primary holds the same.
fn assert_lifecycle_finishes(
    w: &World,
    verb: Verb,
) -> Result<Envelope<MutationDataDto>, FixtureError> {
    if verb == Verb::Close {
        assert!(closure_fields(&w.repo()?, verb.branch())?.is_empty());
        assert!(closure_fields(&w.repo()?, MAIN)?.is_empty());
    }
    let done = resent(w, FIRST, verb);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    assert_eq!(done.effects.cleanup, CleanupEffect::Complete);
    assert_eq!(state_of(w, FIRST), Some(RequestState::Finished));
    assert_eq!(merges_on_primary(&w.repo()?)?, 1);
    if verb == Verb::Close {
        let fields = closure_fields(&w.repo()?, MAIN)?;
        assert_eq!(fields.len(), 2, "{fields:?}");
        assert!(fields.contains(&"closed_by: Fixture <fixture@example.invalid>".to_owned()));
        assert_eq!(closure_fields(&w.bare()?, MAIN)?, fields);
    }
    Ok(done)
}

/// The lifecycle's own answer names the parent, whose ID the conflict verbs
/// take, and the child's conflict among its blockers. Either ID reads the
/// same conflict: the child's, live and resolvable. The parent's ID is no
/// synchronization of its own, so it reads its child's.
fn shown_by_either_id(verb: Verb) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    conflicted(&w, verb)?;
    let (parent, child) = (parent_operation(&w, FIRST)?, child_id(&w, verb, FIRST)?);
    let answer = resent(&w, FIRST, verb);
    assert_eq!(answer.code, ResultCode::RecoveryRequired);
    assert_eq!(answer.operation_id, Some(parent.to_string()));
    assert!(
        lifecycle_of(&answer)
            .blockers
            .contains(&LifecycleBlocker::ConflictPending)
    );
    let by_child = fixed(shown(&w, child).map_err(|_| FixtureError))?;
    let by_parent = fixed(shown(&w, parent).map_err(|_| FixtureError))?;
    assert_eq!(by_parent, by_child);
    assert_eq!(by_child.operation_id, child.to_string());
    assert_eq!(by_child.comment_operation_id, None);
    assert_eq!(by_child.state, ConflictState::ConflictPending);
    assert_eq!(by_child.next_action, ConflictReadNextAction::Resolve);
    assert_eq!(by_child.stage, ConflictStage::Context);
    assert_eq!(
        by_child.target,
        match verb {
            Verb::Promote => ConflictTarget::Document,
            Verb::Close => ConflictTarget::Ticket,
        }
    );
    assert_eq!(by_child.item_id.as_deref(), Some(verb.id()));
    assert_eq!(
        by_child.eligibility,
        Some(ConflictReadEligibility::EligibleCanonical)
    );
    assert_eq!(by_child.paths.len(), 1);
    Ok(())
}

/// Resolved by the parent's ID: the resolution is the child's, published, and
/// the same lifecycle request then adopts it and finishes with one merge.
fn resolved_then_the_same_request(verb: Verb) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    conflicted(&w, verb)?;
    let (parent, child) = (parent_operation(&w, FIRST)?, child_id(&w, verb, FIRST)?);
    let shown = fixed(shown(&w, parent).map_err(|_| FixtureError))?;
    let answer = resolved(
        &w,
        RESOLVE,
        parent,
        shown.observation.as_deref().ok_or(FixtureError)?,
        local_sides(&shown),
    );
    assert_eq!(answer.code, ResultCode::Ok, "{:?}", answer.data);
    let data = resolution_of(&answer);
    assert_eq!(data.operation_id, child.to_string());
    assert_eq!(data.publication, PublicationEffect::Published);
    let commit = fixed(git2::Oid::from_str(
        data.commit_oid.as_deref().ok_or(FixtureError)?,
    ))?;
    assert_eq!(answer.effects.checkpoint, CheckpointEffect::Committed);
    assert_eq!(fixed(w.bare()?.refname_to_id(verb.branch()))?, commit);
    assert_eq!(row_of(&w, child)?.0, "completed");
    assert_eq!(state_of(&w, FIRST), Some(RequestState::Accepted));
    assert_lifecycle_finishes(&w, verb)?;
    let primary = fixed(w.repo()?.refname_to_id(MAIN))?;
    assert!(fixed(w.repo()?.graph_descendant_of(primary, commit))?);
    Ok(())
}

/// An observation or a token of another conflict is `external_change`, with
/// nothing written: the conflict stands for a resolution that matches it.
fn stale_token(verb: Verb) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    conflicted(&w, verb)?;
    let (parent, child) = (parent_operation(&w, FIRST)?, child_id(&w, verb, FIRST)?);
    let served = fixed(w.bare()?.refname_to_id(verb.branch()))?;
    let shown = fixed(shown(&w, child).map_err(|_| FixtureError))?;
    let observation = shown.observation.clone().ok_or(FixtureError)?;
    let other = format!("conflict-v1:{}", "0".repeat(64));
    let stale = resolved(&w, RESOLVE, parent, &other, local_sides(&shown));
    assert_eq!(stale.code, ResultCode::ExternalChange, "{:?}", stale.data);
    assert_eq!(stale.effects.write, results::WriteEffect::NotRequested);
    assert_untouched(&w, verb, child, served);
    let foreign = vec![(
        format!("conflict-path-v1:{}", "0".repeat(64)),
        local_sides(&shown).remove(0).1,
    )];
    let stale = resolved(&w, AGAIN, parent, &observation, foreign);
    assert_eq!(stale.code, ResultCode::ExternalChange, "{:?}", stale.data);
    assert_untouched(&w, verb, child, served);
    let resent_lifecycle = resent(&w, FIRST, verb);
    assert_eq!(resent_lifecycle.code, ResultCode::RecoveryRequired);
    let answer = resolved(&w, AGAIN, parent, &observation, local_sides(&shown));
    assert_eq!(answer.code, ResultCode::Ok, "{:?}", answer.data);
    assert_lifecycle_finishes(&w, verb)?;
    Ok(())
}

/// A child's conflict over a path that is no item's is repaired outside, as
/// a whole: the read says so, the resolution writes nothing, and the lifecycle
/// stays where it was.
fn external_only_stays_external(verb: Verb) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    let path = "notes/outside.txt";
    let worktree = verb.worktree(&w.root);
    diverged(&w, verb, path, b"the peer's notes\n")?;
    commit_in(&worktree, verb.branch(), path, b"the local notes\n")?;
    let confirmation = confirmation_of(&w.service, verb, &w.root);
    let stopped = called(&w.service, FIRST, Some(&confirmation), verb, &w.root);
    assert_eq!(
        stopped.code,
        ResultCode::RecoveryRequired,
        "{:?}",
        stopped.data
    );
    let (parent, child) = (parent_operation(&w, FIRST)?, child_id(&w, verb, FIRST)?);
    let served = fixed(w.bare()?.refname_to_id(verb.branch()))?;
    let shown = fixed(shown(&w, parent).map_err(|_| FixtureError))?;
    assert_eq!(shown.operation_id, child.to_string());
    assert_eq!(shown.state, ConflictState::ConflictPending);
    assert_eq!(shown.next_action, ConflictReadNextAction::ExternalRepair);
    assert_eq!(
        shown.eligibility,
        Some(ConflictReadEligibility::ExternalResolutionRequired)
    );
    let resolutions = shown
        .paths
        .iter()
        .map(|path| (path.token.clone(), "a source\n".to_owned()))
        .collect();
    let refused = resolved(
        &w,
        RESOLVE,
        parent,
        shown.observation.as_deref().ok_or(FixtureError)?,
        resolutions,
    );
    assert_eq!(refused.code, ResultCode::RecoveryRequired);
    assert_eq!(
        resolution_of(&refused)
            .conflict
            .as_ref()
            .map(|conflict| conflict.next_action),
        Some(ConflictNextAction::ExternalRepair)
    );
    assert_eq!(refused.effects.write, results::WriteEffect::NotRequested);
    assert_untouched(&w, verb, child, served);
    let again = resent(&w, FIRST, verb);
    assert_eq!(again.code, ResultCode::RecoveryRequired);
    assert_eq!(state_of(&w, FIRST), Some(RequestState::Accepted));
    assert_untouched(&w, verb, child, served);
    // The repair outside: A's side, committed as the merge with both parents.
    // The read then says `repaired`, and the same request finishes once.
    repaired_outside(&worktree, path, b"the local notes\n")?;
    let repo = fixed(
        w.service
            .resolve_repository(&w.root)
            .map_err(|_| FixtureError),
    )?;
    let read = fixed(
        w.service
            .show_conflict(&repo, parent)
            .map_err(|_| FixtureError),
    )?;
    assert_eq!(read.state, ConflictState::Repaired);
    assert_eq!(read.next_action, ConflictReadNextAction::Resume);
    assert_lifecycle_finishes(&w, verb)?;
    Ok(())
}

/// A repair with Git in `worktree`: `path` written as `bytes`, and the merge
/// committed with HEAD and `MERGE_HEAD` as its parents.
fn repaired_outside(worktree: &Path, path: &str, bytes: &[u8]) -> Result<(), FixtureError> {
    let repo = fixed(git2::Repository::open(worktree))?;
    fixed(std::fs::write(worktree.join(path), bytes))?;
    let mut index = fixed(repo.index())?;
    fixed(index.add_path(Path::new(path)))?;
    fixed(index.write())?;
    let tree = fixed(repo.find_tree(fixed(index.write_tree())?))?;
    let head = fixed(fixed(repo.head())?.peel_to_commit())?;
    let incoming = fixed(
        repo.find_reference("MERGE_HEAD")
            .and_then(|merge_head| merge_head.peel_to_commit()),
    )?;
    let signature = fixed(git2::Signature::now("Fixture", "fixture@example.invalid"))?;
    fixed(repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "Repaired outside",
        &tree,
        &[&head, &incoming],
    ))?;
    fixed(repo.cleanup_state())?;
    Ok(())
}

/// `request cancel` of the lifecycle request, then a resolution: the request
/// that holds the parent holds its child, so its flag stops the resolution's
/// publication, with the resolution committed and nothing pushed. The
/// lifecycle request then answers its own cancellation once, and the next
/// call publishes the resolution and finishes.
fn resolved_under_a_cancelled_parent(verb: Verb) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    conflicted(&w, verb)?;
    let parent = parent_operation(&w, FIRST)?;
    let served = fixed(w.bare()?.refname_to_id(verb.branch()))?;
    asked(&w.service, FIRST)?;
    let shown = fixed(shown(&w, parent).map_err(|_| FixtureError))?;
    let answer = resolved(
        &w,
        RESOLVE,
        parent,
        shown.observation.as_deref().ok_or(FixtureError)?,
        local_sides(&shown),
    );
    assert_eq!(answer.code, ResultCode::Cancelled, "{:?}", answer.data);
    assert_eq!(answer.outcome, Outcome::Cancelled);
    assert_eq!(answer.effects.checkpoint, CheckpointEffect::Committed);
    assert_ne!(answer.effects.publication, PublicationEffect::Published);
    let commit = fixed(git2::Oid::from_str(
        resolution_of(&answer)
            .commit_oid
            .as_deref()
            .ok_or(FixtureError)?,
    ))?;
    assert_eq!(
        fixed(w.bare()?.refname_to_id(verb.branch()))?,
        served,
        "nothing pushed"
    );
    assert_eq!(fixed(w.repo()?.refname_to_id(verb.branch()))?, commit);
    assert_cancelled(&w, FIRST, &resent(&w, FIRST, verb))?;
    assert_lifecycle_finishes(&w, verb)?;
    let primary = fixed(w.repo()?.refname_to_id(MAIN))?;
    assert!(fixed(w.repo()?.graph_descendant_of(primary, commit))?);
    Ok(())
}

/// A finished resolution, sent again under its ID and named this time by the
/// child's ID, is the same request: its answer again, with no connection. The
/// finished lifecycle request's replay is its answer too.
fn replay(verb: Verb) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    conflicted(&w, verb)?;
    let (parent, child) = (parent_operation(&w, FIRST)?, child_id(&w, verb, FIRST)?);
    let shown = fixed(shown(&w, parent).map_err(|_| FixtureError))?;
    let observation = shown.observation.clone().ok_or(FixtureError)?;
    let first = resolved(&w, RESOLVE, parent, &observation, local_sides(&shown));
    assert_eq!(first.code, ResultCode::Ok, "{:?}", first.data);
    let done = assert_lifecycle_finishes(&w, verb)?;
    let before = connections(&w);
    let again = resolved(&w, RESOLVE, child, &observation, local_sides(&shown));
    assert_eq!(again, first);
    assert_eq!(resent(&w, FIRST, verb), done);
    assert_eq!(connections(&w), before, "no connection");
    assert_eq!(merges_on_primary(&w.repo()?)?, 1);
    Ok(())
}

/// A resolution to the peer's side changes the change set the request
/// consented to. The resolution itself is `ok` and published; the same
/// lifecycle request then answers `external_change`, offering a new prepare,
/// with no merge on primary. Its record is finished, and a new prepare is free.
fn unconsented_resolution(verb: Verb) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    conflicted(&w, verb)?;
    let parent = parent_operation(&w, FIRST)?;
    let shown = fixed(shown(&w, parent).map_err(|_| FixtureError))?;
    let incoming = shown
        .paths
        .iter()
        .map(|path| (path.token.clone(), path.incoming.clone().expect("a side")))
        .collect();
    let answer = resolved(
        &w,
        RESOLVE,
        parent,
        shown.observation.as_deref().ok_or(FixtureError)?,
        incoming,
    );
    assert_eq!(answer.code, ResultCode::Ok, "{:?}", answer.data);
    assert_eq!(
        resolution_of(&answer).publication,
        PublicationEffect::Published
    );
    let refused = resent(&w, FIRST, verb);
    assert_eq!(
        refused.code,
        ResultCode::ExternalChange,
        "{:?}",
        refused.data
    );
    assert_eq!(actions(&refused), [RecoveryActionKind::RequestPrepare]);
    assert_eq!(merges_on_primary(&w.repo()?)?, 0);
    assert_eq!(state_of(&w, FIRST), Some(RequestState::Finished));
    let preview = w.service.prepare(&verb.mutation(&w.root, verb.id()));
    assert_eq!(preview.code, ResultCode::Ok, "a new prepare is free");
    Ok(())
}

/// The context's branch deleted at the remote before a resolution: the
/// resolution is committed and its publication answers
/// `remote_branch_deleted`, offering `request cancel` of the lifecycle
/// request that holds the child. Followed, the route loops: the cancellation
/// is answered `cancelled`, but the child holds a resolution, so the release
/// guards keep it and its reservation; `item republish` is then `busy`, and
/// the same request answers `remote_branch_deleted` again, twice over here. A Known Limit,
/// beside the released-resolution one.
fn deleted_branch_after_a_resolution_loops(verb: Verb) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    conflicted(&w, verb)?;
    let parent = parent_operation(&w, FIRST)?;
    let shown = fixed(shown(&w, parent).map_err(|_| FixtureError))?;
    fixed(fixed(w.bare()?.find_reference(verb.branch()))?.delete())?;
    let answer = resolved(
        &w,
        RESOLVE,
        parent,
        shown.observation.as_deref().ok_or(FixtureError)?,
        local_sides(&shown),
    );
    assert_eq!(
        answer.code,
        ResultCode::RemoteBranchDeleted,
        "{:?}",
        answer.data
    );
    assert_eq!(answer.effects.checkpoint, CheckpointEffect::Committed);
    assert_eq!(actions(&answer), [RecoveryActionKind::RequestCancel]);
    assert_eq!(
        answer.recovery[0].arguments.get("request_id"),
        Some(&serde_json::json!(FIRST))
    );
    for _ in 0..2 {
        let stopped = resent(&w, FIRST, verb);
        assert_eq!(stopped.code, ResultCode::RemoteBranchDeleted);
        assert_eq!(
            actions(&stopped),
            [
                RecoveryActionKind::RequestCancel,
                RecoveryActionKind::RepoInspect
            ]
        );
        asked(&w.service, FIRST)?;
        assert_cancelled(&w, FIRST, &resent(&w, FIRST, verb))?;
        assert!(reserved(&w), "the child holding a resolution is kept");
        let republish = Mutation::ItemRepublish(ItemRepublishInput {
            root: w.root.clone(),
            kind: match verb {
                Verb::Promote => AuthoringKind::Document,
                Verb::Close => AuthoringKind::Ticket,
            },
            id: fixed(verb.id().parse())?,
        });
        let preview = w.service.prepare(&republish);
        let consent = preview
            .data
            .as_ref()
            .and_then(PrepareDataDto::preview)
            .ok_or(FixtureError)?
            .confirmation_id
            .clone();
        let done = w.service.execute(
            MutationCall {
                request_id: request(AGAIN),
                confirmation: Some(ConfirmationId::parse(&consent).unwrap()),
                mutation: republish,
            },
            &mut SessionCredentials::new(Provider),
            &mut NoProgress,
            &CancellationToken::new(),
        );
        assert_eq!(done.code, ResultCode::Busy, "{:?}", done.data);
        assert!(fixed(w.bare())?.find_reference(verb.branch()).is_err());
    }
    assert_eq!(merges_on_primary(&w.repo()?)?, 0);
    assert_eq!(state_of(&w, FIRST), Some(RequestState::Accepted));
    Ok(())
}
