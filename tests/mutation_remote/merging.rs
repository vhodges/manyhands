//! `item sync` and `repo sync` merge divergent history: the design's
//! "Required outcomes: divergent synchronization" table, one case per
//! row, each for both commands where the row applies to both.
//!
//! A raw peer clone makes the remote diverge, with `commit` and
//! `push_peer`, as a second user would. Every call goes through
//! `RepositoryService::execute`, and every envelope and store is scanned
//! for the world's private material, as `call` scans them.
//!
//! A row the library answers alike for a person and for a front end is
//! asserted here. What only a front end decides is not: an exit status,
//! C2's bounded retry of a `busy` or a partial result, and what a Ctrl-C
//! is mapped to. A Ctrl-C reaches the library as the call's cancellation
//! token, and the rows that name one are asserted through it.

use std::path::{Path, PathBuf};

use results::{FailureClass, IntegrationEffect};

use super::*;

pub(crate) const CASES: &[ssh_harness::Case] = &[
    ("c4_merging_already_current", c4_merging_already_current),
    ("c4_merging_local_ahead", c4_merging_local_ahead),
    ("c4_merging_local_behind", c4_merging_local_behind),
    ("c4_merging_clean_divergence", c4_merging_clean_divergence),
    (
        "c4_merging_canonical_conflict",
        c4_merging_canonical_conflict,
    ),
    (
        "c4_merging_context_merged_then_primary_conflicts",
        c4_merging_context_merged_then_primary_conflicts,
    ),
    (
        "c4_merging_noncanonical_conflict",
        c4_merging_noncanonical_conflict,
    ),
    ("c4_merging_mixed_conflict", c4_merging_mixed_conflict),
    (
        "c4_merging_same_request_while_the_conflict_is_pending",
        c4_merging_same_request_while_the_conflict_is_pending,
    ),
    (
        "c4_merging_new_sync_while_a_conflict_is_pending",
        c4_merging_new_sync_while_a_conflict_is_pending,
    ),
    (
        "c4_merging_unfinished_local_sync",
        c4_merging_unfinished_local_sync,
    ),
    (
        "c4_merging_worktree_not_clean",
        c4_merging_worktree_not_clean,
    ),
    (
        "c4_merging_worktree_in_an_outside_merge",
        c4_merging_worktree_in_an_outside_merge,
    ),
    (
        "c4_merging_advertisement_moved",
        c4_merging_advertisement_moved,
    ),
    (
        "c4_merging_push_rejected_with_no_merge",
        c4_merging_push_rejected_with_no_merge,
    ),
    (
        "c4_merging_push_rejected_after_a_merge",
        c4_merging_push_rejected_after_a_merge,
    ),
    (
        "c4_merging_index_refresh_fails_after_publication",
        c4_merging_index_refresh_fails_after_publication,
    ),
    ("c4_merging_identity_required", c4_merging_identity_required),
    (
        "c4_merging_identity_set_during_the_stopped_merge",
        c4_merging_identity_set_during_the_stopped_merge,
    ),
    (
        "c4_merging_identity_then_the_same_request",
        c4_merging_identity_then_the_same_request,
    ),
    (
        "c4_merging_remote_context_deleted",
        c4_merging_remote_context_deleted,
    ),
    (
        "c4_merging_no_publication_remote",
        c4_merging_no_publication_remote,
    ),
    (
        "c4_merging_cancelled_at_a_safe_point",
        c4_merging_cancelled_at_a_safe_point,
    ),
    (
        "c4_merging_cancelled_after_the_push_returned",
        c4_merging_cancelled_after_the_push_returned,
    ),
    (
        "c4_merging_unsettled_push_then_cancelled",
        c4_merging_unsettled_push_then_cancelled,
    ),
    (
        "c4_merging_cancelled_during_a_blocked_network_call",
        c4_merging_cancelled_during_a_blocked_network_call,
    ),
];

/// The request that publishes an item's editing branch, or brings
/// primary up to the server's, before a case begins.
const SETUP: &str = "01ARZ3NDEKTSV4RRFFQ69G5FY0";
/// A request no other of a case uses.
const REQUEST_5: &str = "01ARZ3NDEKTSV4RRFFQ69G5FY5";
/// A ticket a primary-branch case keeps on primary: canonical content
/// both sides of a merge can change.
pub(super) const PRIMARY_TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAX";
/// The identity a case sets with `repo identity-set` once the world's
/// own is removed.
pub(super) const MERGE_NAME: &str = "Merge Author";
pub(super) const MERGE_EMAIL: &str = "merge-author@example.invalid";

/// The branch a case synchronizes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Branch {
    /// `repo sync`, of primary.
    Primary,
    /// `item sync`, of the world's item's editing branch.
    Item,
}

const BOTH: [Branch; 2] = [Branch::Primary, Branch::Item];

/// A world with a trusted server and the branch a case synchronizes.
/// For an item, its editing branch exists and the server holds it.
struct Synced {
    w: World,
    branch: Branch,
    /// The item's editing worktree, for an item.
    context: Option<PathBuf>,
}

impl Synced {
    fn new(branch: Branch) -> Result<Self, FixtureError> {
        let mut w = World::new()?;
        approve(&w)?;
        let context = match branch {
            Branch::Primary => None,
            Branch::Item => {
                let context = w.context()?;
                let published = execute(&w, SETUP, item_sync(&w));
                assert_eq!(published.code, ResultCode::Ok);
                Some(context.worktree)
            }
        };
        Ok(Self { w, branch, context })
    }

    fn mutation(&self) -> Mutation {
        match self.branch {
            Branch::Primary => repo_sync(&self.w),
            Branch::Item => item_sync(&self.w),
        }
    }

    fn execute(&self, id: &str) -> Envelope<MutationDataDto> {
        execute(&self.w, id, self.mutation())
    }

    /// The branch, as a full reference name.
    fn reference(&self) -> &'static str {
        match self.branch {
            Branch::Primary => "refs/heads/main",
            Branch::Item => CONTEXT,
        }
    }

    /// The worktree the branch is checked out in.
    fn worktree(&self) -> &Path {
        self.context.as_deref().unwrap_or(&self.w.root)
    }

    /// The subject of the merge commit a synchronization of the branch
    /// makes of what the remote holds of the same branch.
    fn subject(&self) -> String {
        match self.branch {
            Branch::Primary => "Merge remote primary".to_owned(),
            Branch::Item => format!("Merge remote context {ITEM}"),
        }
    }

    /// The canonical file both sides change for a canonical conflict.
    fn ticket_path(&self) -> String {
        match self.branch {
            Branch::Primary => ticket_path(PRIMARY_TICKET),
            Branch::Item => ticket_path(ITEM),
        }
    }

    fn ticket_id(&self) -> &'static str {
        match self.branch {
            Branch::Primary => PRIMARY_TICKET,
            Branch::Item => ITEM,
        }
    }

    fn local_tip(&self) -> Result<git2::Oid, FixtureError> {
        fixed(self.w.repo()?.refname_to_id(self.reference()))
    }

    fn served_tip(&self) -> Result<git2::Oid, FixtureError> {
        fixed(self.w.bare()?.refname_to_id(self.reference()))
    }

    /// Commits `bytes` at `path` on the branch, in its worktree, which is
    /// left clean.
    fn local_commit(&self, path: &str, bytes: &[u8]) -> Result<git2::Oid, FixtureError> {
        commit_in(self.worktree(), self.reference(), path, bytes)
    }

    /// The peer commits `bytes` at `path` on the branch as the server
    /// holds it, and pushes it there.
    fn remote_commit(&self, path: &str, bytes: &[u8]) -> Result<git2::Oid, FixtureError> {
        let reference = self.reference();
        if self.branch == Branch::Item {
            fetch_into_peer(&self.w, reference)?;
        }
        let parent = fixed(self.w.peer.refname_to_id(reference))?;
        let new = commit_path(&self.w.peer, parent, path, bytes)?;
        fixed(self.w.peer.reference(reference, new, true, "fixture"))?;
        push_peer(&self.w.peer, &self.w.server, reference)?;
        Ok(new)
    }

    /// History that merges cleanly: a file of each side's own.
    fn diverge(&self) -> Result<(git2::Oid, git2::Oid), FixtureError> {
        let local = self.local_commit("local-divergent", b"local-divergent")?;
        let incoming = self.remote_commit("remote-divergent", b"remote-divergent")?;
        Ok((local, incoming))
    }

    /// History that conflicts in canonical content: each side gives the
    /// same ticket another title. The ticket is on the branch at the
    /// merge base, so the conflict has all three sides.
    fn diverge_on_a_ticket(&self) -> Result<(git2::Oid, git2::Oid), FixtureError> {
        let path = self.ticket_path();
        let id = self.ticket_id();
        if self.branch == Branch::Primary {
            // Primary holds no ticket of its own yet: the peer adds one,
            // and primary is brought up to it.
            self.remote_commit(&path, ticket_source(id, "Base").as_bytes())?;
            assert_current(&self.execute(SETUP));
        }
        let local = self.local_commit(&path, ticket_source(id, "Local").as_bytes())?;
        let incoming = self.remote_commit(&path, ticket_source(id, "Incoming").as_bytes())?;
        Ok((local, incoming))
    }

    /// History that conflicts in a file that is no canonical content:
    /// each side adds it with other bytes.
    fn diverge_on_a_plain_file(&self) -> Result<(git2::Oid, git2::Oid), FixtureError> {
        let local = self.local_commit("conflict", b"local conflict")?;
        let incoming = self.remote_commit("conflict", b"incoming conflict")?;
        Ok((local, incoming))
    }

    /// Asserts that `oid` is one merge commit of `local` and `incoming`,
    /// in that order, with the subject fact 25 gives it.
    fn assert_merge(
        &self,
        oid: git2::Oid,
        local: git2::Oid,
        incoming: git2::Oid,
    ) -> Result<(), FixtureError> {
        let repo = self.w.repo()?;
        let merge = fixed(repo.find_commit(oid))?;
        assert_eq!(merge.parent_ids().collect::<Vec<_>>(), [local, incoming]);
        assert_eq!(merge.summary(), Some(self.subject().as_str()));
        Ok(())
    }

    /// Asserts that nothing of a merge was begun: no integration window,
    /// no merge commit, no merge state in the worktree.
    fn assert_no_merge(&self, local: git2::Oid) -> Result<(), FixtureError> {
        assert_eq!(merge_rows(&self.w)?, 0);
        assert_eq!(merge_commits(&self.w.repo()?)?, 0);
        assert_eq!(self.local_tip()?, local);
        let worktree = fixed(git2::Repository::open(self.worktree()))?;
        assert!(!worktree.path().join("MERGE_HEAD").exists());
        assert!(!fixed(worktree.index())?.has_conflicts());
        Ok(())
    }

    /// Asserts that the merge the synchronization began is left in its
    /// worktree, with the branch where the merge began.
    fn assert_merge_left(&self, local: git2::Oid) -> Result<(), FixtureError> {
        assert_eq!(self.local_tip()?, local);
        let worktree = fixed(git2::Repository::open(self.worktree()))?;
        assert!(worktree.path().join("MERGE_HEAD").exists());
        assert!(fixed(worktree.index())?.has_conflicts());
        Ok(())
    }
}

pub(super) fn ticket_path(id: &str) -> String {
    format!(".manyhands/tickets/{id}/ticket.md")
}

/// A ticket's canonical source, titled `title`.
pub(super) fn ticket_source(id: &str, title: &str) -> String {
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {id}\ntitle: {title}\n\
         type: task\nstatus: open\n---\nBody of {title}.\n"
    )
}

/// Brings the peer's `reference` up to the server's.
pub(super) fn fetch_into_peer(w: &World, reference: &str) -> Result<(), FixtureError> {
    let mut remote = fixed(w.peer.remote_anonymous(&w.server.url()))?;
    let mut options = git2::FetchOptions::new();
    options.remote_callbacks(callbacks(&w.server));
    fixed(remote.fetch(
        &[&format!("+{reference}:{reference}")],
        Some(&mut options),
        None,
    ))
}

/// The data of a synchronization that stopped on a conflict.
fn conflict_data(
    envelope: &Envelope<MutationDataDto>,
) -> Result<&SynchronizationConflictMutationDto, FixtureError> {
    match envelope.data.as_ref() {
        Some(MutationDataDto::SynchronizationConflict(data)) => Ok(data),
        _ => Err(FixtureError),
    }
}

/// Asserts that `envelope` stopped on a conflict of its own operation,
/// whose next step is `next_action`.
fn assert_conflict(
    envelope: &Envelope<MutationDataDto>,
    next_action: ConflictNextAction,
) -> Result<(), FixtureError> {
    assert_eq!(envelope.code, ResultCode::RecoveryRequired);
    assert_eq!(envelope.effects, merge_pending());
    let data = conflict_data(envelope)?;
    assert_eq!(
        data.r#type,
        SynchronizationConflictType::SynchronizationConflict
    );
    assert_eq!(
        data.conflict,
        ConflictNextDto {
            operation_id: envelope.operation_id.clone().ok_or(FixtureError)?,
            next_action,
        }
    );
    Ok(())
}

/// A merge begun and not finished, and nothing published.
fn merge_pending() -> Effects {
    Effects {
        publication: PublicationEffect::Pending,
        integration: IntegrationEffect::Pending,
        ..Effects::not_requested()
    }
}

/// Nothing published, and nothing else said.
fn publication_pending() -> Effects {
    Effects {
        publication: PublicationEffect::Pending,
        ..Effects::not_requested()
    }
}

/// `effects`, naming the merge commit `merge` the request made: the
/// integration complete, and the commit, unless one is named already.
fn merged(effects: Effects, merge: git2::Oid) -> Effects {
    Effects {
        integration: IntegrationEffect::Complete,
        commit_oid: Some(
            effects
                .commit_oid
                .clone()
                .unwrap_or_else(|| merge.to_string()),
        ),
        ..effects
    }
}

fn c4_merging_already_current() -> Result<(), FixtureError> {
    for branch in BOTH {
        let s = Synced::new(branch)?;
        let local = s.local_tip()?;
        let pushes = accepted(&s.w.server);
        let current = s.execute(REQUEST_1);
        assert_current(&current);
        assert!(current.data.is_none());
        assert!(current.recovery.is_empty());
        assert_eq!(s.served_tip()?, local);
        assert_eq!(accepted(&s.w.server), pushes);
        s.assert_no_merge(local)?;
    }
    Ok(())
}

fn c4_merging_local_ahead() -> Result<(), FixtureError> {
    for branch in BOTH {
        let s = Synced::new(branch)?;
        let local = s.local_commit("local-ahead", b"local-ahead")?;
        let pushes = accepted(&s.w.server);
        let published = s.execute(REQUEST_1);
        assert_eq!(published.code, ResultCode::Ok);
        assert_eq!(published.outcome, Outcome::Success);
        assert_eq!(published.effects, super::published(local));
        assert!(published.data.is_none());
        assert!(published.recovery.is_empty());
        assert_eq!(s.served_tip()?, local);
        assert_eq!(accepted(&s.w.server), pushes + 1);
        s.assert_no_merge(local)?;
    }
    Ok(())
}

fn c4_merging_local_behind() -> Result<(), FixtureError> {
    for branch in BOTH {
        let s = Synced::new(branch)?;
        let incoming = s.remote_commit("remote-ahead", b"remote-ahead")?;
        let pushes = accepted(&s.w.server);
        let current = s.execute(REQUEST_1);
        // The local branch fast-forwards, and nothing is pushed (fact 63).
        assert_current(&current);
        assert!(current.data.is_none());
        assert!(current.recovery.is_empty());
        assert_eq!(s.local_tip()?, incoming);
        assert_eq!(s.served_tip()?, incoming);
        assert_eq!(accepted(&s.w.server), pushes);
        s.assert_no_merge(incoming)?;
    }
    Ok(())
}

fn c4_merging_clean_divergence() -> Result<(), FixtureError> {
    for branch in BOTH {
        let s = Synced::new(branch)?;
        let (local, incoming) = s.diverge()?;
        let pushes = accepted(&s.w.server);
        let merged = s.execute(REQUEST_1);
        assert_eq!(merged.code, ResultCode::Ok);
        assert_eq!(merged.outcome, Outcome::Success);
        assert!(merged.data.is_none());
        assert!(merged.recovery.is_empty());
        let merge = s.local_tip()?;
        // The merge commit is named: the integration complete.
        assert_eq!(merged.effects, self::merged(super::published(merge), merge));
        // One merge commit of the two, in that order, published.
        s.assert_merge(merge, local, incoming)?;
        assert_eq!(merge_commits(&s.w.repo()?)?, 1);
        assert_eq!(s.served_tip()?, merge);
        assert_eq!(accepted(&s.w.server), pushes + 1);
        // No rebase: the remote keeps its earlier tip as an ancestor.
        assert!(fixed(s.w.bare()?.graph_descendant_of(merge, incoming))?);
        assert!(!reserved(&s.w));
        // The finished request is answered from its record.
        let opened = connections(&s.w);
        assert_eq!(s.execute(REQUEST_1), merged);
        assert_eq!(connections(&s.w), opened);
    }
    Ok(())
}

fn c4_merging_canonical_conflict() -> Result<(), FixtureError> {
    for branch in BOTH {
        let s = Synced::new(branch)?;
        let (local, incoming) = s.diverge_on_a_ticket()?;
        let pushes = accepted(&s.w.server);
        let stopped = s.execute(REQUEST_1);
        assert_conflict(&stopped, ConflictNextAction::Resolve)?;
        assert_eq!(stopped.outcome, Outcome::Blocked);
        assert_eq!(
            actions(&stopped),
            [
                RecoveryActionKind::RequestRetry,
                RecoveryActionKind::RepoInspect
            ]
        );
        // The merge is left, the reservation is released, and the record
        // stays for the same request.
        s.assert_merge_left(local)?;
        assert!(!reserved(&s.w));
        assert_eq!(
            record(&s.w, REQUEST_1).ok_or(FixtureError)?.state,
            RequestState::Accepted
        );
        assert_eq!(s.served_tip()?, incoming);
        assert_eq!(accepted(&s.w.server), pushes);
    }
    Ok(())
}

fn c4_merging_context_merged_then_primary_conflicts() -> Result<(), FixtureError> {
    let mut w = World::new()?;
    approve(&w)?;
    // A file primary and the item's branch share from the start.
    let peer_main = fixed(w.peer.refname_to_id("refs/heads/main"))?;
    let shared = commit_path(&w.peer, peer_main, "shared", b"shared base")?;
    fixed(w.peer.reference("refs/heads/main", shared, true, "fixture"))?;
    push_peer(&w.peer, &w.server, "refs/heads/main")?;
    assert_current(&execute(&w, REQUEST_4, repo_sync(&w)));
    let worktree = w.context()?.worktree;
    assert_eq!(execute(&w, SETUP, item_sync(&w)).code, ResultCode::Ok);
    let s = Synced {
        w,
        branch: Branch::Item,
        context: Some(worktree),
    };
    // The item's branch: a change here, and another at the remote, which
    // merge cleanly (stage 0).
    let local = s.local_commit("shared", b"shared on the item")?;
    let incoming = s.remote_commit("remote-divergent", b"remote-divergent")?;
    // Primary at the remote changes the shared file another way, which
    // conflicts with the item's (stage 1). It is no canonical content.
    let peer_main = fixed(s.w.peer.refname_to_id("refs/heads/main"))?;
    let primary = commit_path(&s.w.peer, peer_main, "shared", b"shared on primary")?;
    fixed(
        s.w.peer
            .reference("refs/heads/main", primary, true, "fixture"),
    )?;
    push_peer(&s.w.peer, &s.w.server, "refs/heads/main")?;
    let pushes = accepted(&s.w.server);

    let stopped = s.execute(REQUEST_1);
    assert_conflict(&stopped, ConflictNextAction::ExternalRepair)?;
    // The stage-0 merge commit is durable.
    assert_eq!(stopped.outcome, Outcome::Partial);
    assert_eq!(
        actions(&stopped),
        [
            RecoveryActionKind::RequestRetry,
            RecoveryActionKind::RepoInspect
        ]
    );
    let merge = s.local_tip()?;
    s.assert_merge(merge, local, incoming)?;
    s.assert_merge_left(merge)?;
    assert!(!reserved(&s.w));
    assert_eq!(s.served_tip()?, incoming);
    assert_eq!(accepted(&s.w.server), pushes);
    Ok(())
}

fn c4_merging_noncanonical_conflict() -> Result<(), FixtureError> {
    for branch in BOTH {
        let s = Synced::new(branch)?;
        let (local, incoming) = s.diverge_on_a_plain_file()?;
        let pushes = accepted(&s.w.server);
        let stopped = s.execute(REQUEST_1);
        assert_conflict(&stopped, ConflictNextAction::ExternalRepair)?;
        assert_eq!(stopped.outcome, Outcome::Blocked);
        s.assert_merge_left(local)?;
        assert!(!reserved(&s.w));
        assert_eq!(s.served_tip()?, incoming);
        assert_eq!(accepted(&s.w.server), pushes);
    }
    Ok(())
}

/// A conflict whose paths are mixed, a ticket and a file that is no
/// canonical content, is repaired outside as a whole: the domain resolves
/// a set only when every path is canonical, so the next step is
/// `external_repair`.
fn c4_merging_mixed_conflict() -> Result<(), FixtureError> {
    for branch in BOTH {
        let s = Synced::new(branch)?;
        s.diverge_on_a_ticket()?;
        let local = s.local_commit("conflict", b"local conflict")?;
        s.remote_commit("conflict", b"incoming conflict")?;
        let stopped = s.execute(REQUEST_1);
        assert_conflict(&stopped, ConflictNextAction::ExternalRepair)?;
        assert_eq!(stopped.outcome, Outcome::Blocked);
        s.assert_merge_left(local)?;
        // Both paths are in conflict.
        let worktree = fixed(git2::Repository::open(s.worktree()))?;
        assert_eq!(fixed(fixed(worktree.index())?.conflicts())?.count(), 2);
    }
    Ok(())
}

fn c4_merging_same_request_while_the_conflict_is_pending() -> Result<(), FixtureError> {
    for branch in BOTH {
        let s = Synced::new(branch)?;
        let (local, _) = s.diverge_on_a_ticket()?;
        let first = s.execute(REQUEST_1);
        assert_conflict(&first, ConflictNextAction::Resolve)?;
        let opened = connections(&s.w);
        let pushes = accepted(&s.w.server);
        let rows = merge_rows(&s.w)?;
        // The same conflict, read again with no transport (fact 26).
        let again = s.execute(REQUEST_1);
        assert_eq!(again, first);
        assert_eq!(connections(&s.w), opened);
        assert_eq!(accepted(&s.w.server), pushes);
        assert_eq!(merge_rows(&s.w)?, rows);
        s.assert_merge_left(local)?;
        assert_eq!(
            record(&s.w, REQUEST_1).ok_or(FixtureError)?.state,
            RequestState::Accepted
        );
    }
    Ok(())
}

/// Asserts that `envelope` is `busy`, and that nothing came of it:
/// nothing recorded, no effect, no data, and the same request offered.
fn assert_busy(w: &World, id: &str, envelope: &Envelope<MutationDataDto>) {
    assert_eq!(envelope.code, ResultCode::Busy);
    assert_eq!(envelope.failure_class(), Some(FailureClass::Transient));
    assert_eq!(envelope.effects, Effects::not_requested());
    assert!(envelope.data.is_none());
    assert_eq!(actions(envelope), [RecoveryActionKind::RequestRetry]);
    assert!(record(w, id).is_none());
}

/// Any new synchronization while a conflict another operation holds is
/// pending is `busy`, of either command and whatever it synchronizes.
///
/// One whose worktrees are clean reaches the reservation, which the
/// pending conflict holds (fact 27). One that inspects the worktree the
/// pending merge is in is refused before it reserves, since the domain
/// inspects its target first: a new `item sync` of the item whose merge
/// is pending, and, while primary's is, any new synchronization, since an
/// item's inspects primary's worktree as well as its own. The binding
/// answers that refusal as the reservation would. The same request, which
/// `restart` takes up, reads the conflict instead (the case above).
fn c4_merging_new_sync_while_a_conflict_is_pending() -> Result<(), FixtureError> {
    // The item's conflict is pending; another item has a branch of its
    // own.
    let s = Synced::new(Branch::Item)?;
    assert_eq!(
        execute(&s.w, REQUEST_4, create_other(&s.w)).code,
        ResultCode::Ok
    );
    let (local, _) = s.diverge_on_a_ticket()?;
    assert_conflict(&s.execute(REQUEST_1), ConflictNextAction::Resolve)?;
    let pushes = accepted(&s.w.server);
    let rows = merge_rows(&s.w)?;
    let another_item = Mutation::ItemSync(ItemSyncInput {
        root: s.w.root.clone(),
        kind: AuthoringKind::Ticket,
        id: OTHER_ITEM.parse().unwrap(),
    });
    for (id, mutation) in [
        (REQUEST_2, repo_sync(&s.w)),
        (REQUEST_3, another_item),
        // The item itself, under a new request.
        (REQUEST_5, item_sync(&s.w)),
    ] {
        assert_busy(&s.w, id, &execute(&s.w, id, mutation));
    }
    assert_eq!(accepted(&s.w.server), pushes);
    assert_eq!(merge_rows(&s.w)?, rows);
    s.assert_merge_left(local)?;
    // The pending conflict is still the same request's to read.
    assert_conflict(&s.execute(REQUEST_1), ConflictNextAction::Resolve)?;

    // Primary's conflict is pending; an item has a branch of its own.
    let mut s = Synced::new(Branch::Primary)?;
    s.w.context()?;
    let (local, _) = s.diverge_on_a_ticket()?;
    assert_conflict(&s.execute(REQUEST_1), ConflictNextAction::Resolve)?;
    let pushes = accepted(&s.w.server);
    let rows = merge_rows(&s.w)?;
    for (id, mutation) in [(REQUEST_2, repo_sync(&s.w)), (REQUEST_3, item_sync(&s.w))] {
        assert_busy(&s.w, id, &execute(&s.w, id, mutation));
    }
    assert_eq!(accepted(&s.w.server), pushes);
    assert_eq!(merge_rows(&s.w)?, rows);
    s.assert_merge_left(local)?;
    Ok(())
}

fn c4_merging_unfinished_local_sync() -> Result<(), FixtureError> {
    for branch in BOTH {
        let s = Synced::new(branch)?;
        let (local, incoming) = s.diverge()?;
        leave_local_sync_conflict(&s.w);
        let local = if branch == Branch::Primary {
            // The other item's conflict is made on primary.
            s.local_tip()?
        } else {
            local
        };
        let opened = connections(&s.w);
        let pushes = accepted(&s.w.server);
        let refused = s.execute(REQUEST_1);
        assert_eq!(refused.code, ResultCode::RecoveryRequired);
        assert_eq!(refused.outcome, Outcome::Blocked);
        assert_eq!(refused.effects, Effects::not_requested());
        assert!(refused.data.is_none());
        assert_eq!(actions(&refused), [RecoveryActionKind::RepoInspect]);
        assert!(record(&s.w, REQUEST_1).is_none());
        assert!(!reserved(&s.w));
        assert_eq!(connections(&s.w), opened);
        assert_eq!(accepted(&s.w.server), pushes);
        assert_eq!(s.served_tip()?, incoming);
        s.assert_no_merge(local)?;
    }
    Ok(())
}

fn c4_merging_worktree_not_clean() -> Result<(), FixtureError> {
    for branch in BOTH {
        let s = Synced::new(branch)?;
        let (local, incoming) = s.diverge()?;
        fixed(std::fs::write(
            s.worktree().join("local-divergent"),
            b"not committed",
        ))?;
        let pushes = accepted(&s.w.server);
        let refused = s.execute(REQUEST_1);
        assert_eq!(refused.code, ResultCode::WorktreeNotClean);
        assert_eq!(refused.outcome, Outcome::Blocked);
        assert!(!refused.effects.is_durable());
        assert!(refused.data.is_none());
        assert_eq!(accepted(&s.w.server), pushes);
        assert_eq!(s.served_tip()?, incoming);
        assert_eq!(
            fixed(std::fs::read(s.worktree().join("local-divergent")))?,
            b"not committed"
        );
        assert_eq!(merge_commits(&s.w.repo()?)?, 0);
        assert_eq!(s.local_tip()?, local);
    }
    Ok(())
}

fn c4_merging_worktree_in_an_outside_merge() -> Result<(), FixtureError> {
    for branch in BOTH {
        let s = Synced::new(branch)?;
        let (local, incoming) = s.diverge()?;
        // A merge of the user's own, stopped on a conflict, in the
        // branch's worktree.
        let worktree = fixed(git2::Repository::open(s.worktree()))?;
        let side = commit_path(&worktree, local, "local-divergent", b"outside side")?;
        let head = commit_path(&worktree, local, "local-divergent", b"outside head")?;
        fixed(worktree.checkout_tree(
            &fixed(worktree.find_object(head, None))?,
            Some(git2::build::CheckoutBuilder::new().safe()),
        ))?;
        fixed(worktree.reference(s.reference(), head, true, "fixture"))?;
        let annotated = fixed(worktree.find_annotated_commit(side))?;
        fixed(worktree.merge(&[&annotated], None, None))?;
        assert!(fixed(worktree.index())?.has_conflicts());
        let pushes = accepted(&s.w.server);
        let refused = s.execute(REQUEST_1);
        assert_eq!(refused.code, ResultCode::WorktreeConflicted);
        assert_eq!(refused.outcome, Outcome::Blocked);
        assert!(!refused.effects.is_durable());
        assert!(refused.data.is_none());
        assert_eq!(accepted(&s.w.server), pushes);
        assert_eq!(s.served_tip()?, incoming);
        assert_eq!(s.local_tip()?, head);
        assert!(fixed(worktree.index())?.has_conflicts());
        assert_eq!(merge_commits(&s.w.repo()?)?, 0);
    }
    Ok(())
}

fn c4_merging_advertisement_moved() -> Result<(), FixtureError> {
    for branch in BOTH {
        let s = Synced::new(branch)?;
        let (local, _) = s.diverge()?;
        let bare = s.w.server.repository_path().to_owned();
        let reference = s.reference();
        let once = std::cell::Cell::new(false);
        let moved = std::rc::Rc::new(std::cell::Cell::new(None));
        let hook = {
            let moved = moved.clone();
            repository::transport::operation_tests::install_hook(move |point| {
                if point == repository::transport::operation_tests::Checkpoint::TrackingDownloaded
                    && !once.replace(true)
                {
                    let repo = git2::Repository::open_bare(&bare).unwrap();
                    let old = repo.refname_to_id(reference).unwrap();
                    let new = commit(&repo, old, "moved-during-fetch", b"moved").unwrap();
                    repo.reference(reference, new, true, "fixture").unwrap();
                    moved.set(Some(new));
                }
            })
        };
        let pushes = accepted(&s.w.server);
        let refused = s.execute(REQUEST_1);
        drop(hook);
        assert_eq!(refused.code, ResultCode::ExternalChange);
        assert_eq!(refused.outcome, Outcome::Blocked);
        assert!(!refused.effects.is_durable());
        assert!(refused.data.is_none());
        // Nothing was pushed: the server holds what moved it.
        assert_eq!(Some(s.served_tip()?), moved.get());
        assert_eq!(accepted(&s.w.server), pushes);
        assert_eq!(merge_commits(&s.w.repo()?)?, 0);
        assert_eq!(s.local_tip()?, local);
    }
    Ok(())
}

/// Has the server refuse the next push to the branch: by its own
/// configuration for primary, and for an item's branch by one competing
/// write, made as the push arrives, that the push was not made on.
fn refuse_the_push(s: &Synced) -> Result<(), FixtureError> {
    match s.branch {
        Branch::Primary => s.w.server.reject_primary_updates(true),
        Branch::Item => {
            let bare = s.w.bare()?;
            let served = s.served_tip()?;
            let competing = commit(&bare, served, "competing", b"competing")?;
            s.w.server.race_update(CONTEXT, served, competing)
        }
    }
}

fn c4_merging_push_rejected_with_no_merge() -> Result<(), FixtureError> {
    for branch in BOTH {
        let s = Synced::new(branch)?;
        let local = s.local_commit("local-ahead", b"local-ahead")?;
        refuse_the_push(&s)?;
        let rejected = s.execute(REQUEST_1);
        assert_eq!(rejected.code, ResultCode::PushRejected);
        assert_eq!(rejected.outcome, Outcome::Blocked);
        // Nothing durable.
        assert_eq!(rejected.effects, publication_pending());
        assert!(rejected.data.is_none());
        assert_eq!(actions(&rejected), [RecoveryActionKind::RequestRetry]);
        assert_ne!(s.served_tip()?, local);
        s.assert_no_merge(local)?;
    }
    Ok(())
}

fn c4_merging_push_rejected_after_a_merge() -> Result<(), FixtureError> {
    for branch in BOTH {
        let s = Synced::new(branch)?;
        let (local, incoming) = s.diverge()?;
        refuse_the_push(&s)?;
        let rejected = s.execute(REQUEST_1);
        let merge = s.local_tip()?;
        assert_eq!(rejected.code, ResultCode::PushRejected);
        // The merge commit stays, and is named: what stopped is partial.
        assert_eq!(rejected.outcome, Outcome::Partial);
        assert_eq!(rejected.effects, merged(publication_pending(), merge));
        assert!(rejected.data.is_none());
        assert_eq!(actions(&rejected), [RecoveryActionKind::RequestRetry]);
        s.assert_merge(merge, local, incoming)?;
        assert_ne!(s.served_tip()?, merge);
        assert_eq!(
            record(&s.w, REQUEST_1).ok_or(FixtureError)?.state,
            RequestState::Accepted
        );
        // Its request ID with another command is a mismatch, which says
        // what the repository shows of the request: the merge, partial.
        let other = match branch {
            Branch::Primary => item_sync(&s.w),
            Branch::Item => repo_sync(&s.w),
        };
        let mismatched = execute(&s.w, REQUEST_1, other);
        assert_eq!(mismatched.code, ResultCode::RequestMismatch);
        assert_eq!(mismatched.outcome, Outcome::Partial);
        assert_eq!(mismatched.effects, merged(publication_pending(), merge));
    }
    Ok(())
}

fn c4_merging_index_refresh_fails_after_publication() -> Result<(), FixtureError> {
    for branch in BOTH {
        let s = Synced::new(branch)?;
        let (local, incoming) = s.diverge()?;
        let pushes = accepted(&s.w.server);
        let db = s.w.db()?;
        fixed(db.execute_batch("CREATE TRIGGER fail_handoff BEFORE UPDATE ON operation_records WHEN NEW.state='completed' AND NEW.action='refresh' BEGIN SELECT RAISE(ABORT,'fixed failure'); END"))?;
        let partial = s.execute(REQUEST_1);
        let merge = s.local_tip()?;
        assert_eq!(partial.code, ResultCode::DiscoveryPending);
        assert_eq!(partial.outcome, Outcome::Partial);
        assert_eq!(
            partial.effects,
            Effects {
                publication: PublicationEffect::Published,
                discovery: DiscoveryEffect::Pending,
                integration: IntegrationEffect::Complete,
                commit_oid: Some(merge.to_string()),
                ..Effects::not_requested()
            }
        );
        assert!(partial.data.is_none());
        assert_eq!(actions(&partial), [RecoveryActionKind::OperationResume]);
        s.assert_merge(merge, local, incoming)?;
        assert_eq!(s.served_tip()?, merge);
        assert_eq!(accepted(&s.w.server), pushes + 1);
        fixed(db.execute_batch("DROP TRIGGER fail_handoff"))?;
    }
    Ok(())
}

/// Removes the identity the world gives its repository: the repository
/// then has none to commit a merge with (fact 60 keeps any other out).
fn remove_the_identity(w: &World) -> Result<(), FixtureError> {
    let mut config = fixed(fixed(w.repo()?.config())?.open_level(git2::ConfigLevel::Local))?;
    fixed(config.remove("user.name"))?;
    fixed(config.remove("user.email"))?;
    Ok(())
}

/// Sets the repository's identity through `repo identity-set`, which is
/// confirmed: a preview, then the request with its confirmation.
pub(super) fn set_the_identity(
    w: &World,
    id: &str,
) -> Result<Envelope<MutationDataDto>, FixtureError> {
    let set = || {
        Mutation::RepoIdentitySet(RepoIdentitySetInput {
            root: w.root.clone(),
            name: MERGE_NAME.into(),
            email: MERGE_EMAIL.into(),
            initials: InitialsChange::Unchanged,
        })
    };
    let preview = w.service.prepare(&set());
    assert_eq!(preview.code, ResultCode::Ok);
    let consent = preview
        .data
        .and_then(PrepareDataDto::into_preview)
        .ok_or(FixtureError)?
        .confirmation_id;
    Ok(presenting(w, id, Some(&consent), set()))
}

/// A divergent synchronization stopped for want of an identity, and what
/// it was answered with.
fn stopped_for_an_identity(
    branch: Branch,
) -> Result<(Synced, git2::Oid, git2::Oid, Envelope<MutationDataDto>), FixtureError> {
    let s = Synced::new(branch)?;
    let (local, incoming) = s.diverge()?;
    remove_the_identity(&s.w)?;
    let stopped = s.execute(REQUEST_1);
    Ok((s, local, incoming, stopped))
}

fn c4_merging_identity_required() -> Result<(), FixtureError> {
    for branch in BOTH {
        let (s, local, incoming, stopped) = stopped_for_an_identity(branch)?;
        assert_eq!(stopped.code, ResultCode::IdentityRequired);
        assert_eq!(stopped.outcome, Outcome::Blocked);
        assert!(!stopped.effects.is_durable());
        assert!(stopped.data.is_none());
        assert_eq!(
            actions(&stopped),
            [
                RecoveryActionKind::RequestRetry,
                RecoveryActionKind::RepoIdentitySet
            ]
        );
        assert_eq!(merge_commits(&s.w.repo()?)?, 0);
        assert_eq!(s.local_tip()?, local);
        assert_eq!(s.served_tip()?, incoming);
        assert_eq!(
            record(&s.w, REQUEST_1).ok_or(FixtureError)?.state,
            RequestState::Accepted
        );
    }
    Ok(())
}

fn c4_merging_identity_set_during_the_stopped_merge() -> Result<(), FixtureError> {
    for branch in BOTH {
        let (s, _, _, stopped) = stopped_for_an_identity(branch)?;
        assert_eq!(stopped.code, ResultCode::IdentityRequired);
        // Not refused by the stopped merge (fact 28): no workaround.
        let set = set_the_identity(&s.w, REQUEST_2)?;
        assert_eq!(set.code, ResultCode::Ok);
        assert_eq!(set.outcome, Outcome::Success);
        let config = fixed(s.w.repo()?.config())?;
        assert_eq!(fixed(config.get_string("user.name"))?, MERGE_NAME);
        assert_eq!(fixed(config.get_string("user.email"))?, MERGE_EMAIL);
    }
    Ok(())
}

fn c4_merging_identity_then_the_same_request() -> Result<(), FixtureError> {
    for branch in BOTH {
        let (s, local, incoming, stopped) = stopped_for_an_identity(branch)?;
        assert_eq!(stopped.code, ResultCode::IdentityRequired);
        assert_eq!(set_the_identity(&s.w, REQUEST_2)?.code, ResultCode::Ok);
        let pushes = accepted(&s.w.server);
        let merged = s.execute(REQUEST_1);
        assert_eq!(merged.code, ResultCode::Ok);
        assert_eq!(merged.outcome, Outcome::Success);
        assert_eq!(merged.operation_id, stopped.operation_id);
        let merge = s.local_tip()?;
        assert_eq!(merged.effects, self::merged(super::published(merge), merge));
        // Committed once, by the identity that was set, and published once.
        s.assert_merge(merge, local, incoming)?;
        let repo = s.w.repo()?;
        assert_eq!(merge_commits(&repo)?, 1);
        assert_eq!(
            fixed(repo.find_commit(merge))?.author().name(),
            Some(MERGE_NAME)
        );
        assert_eq!(s.served_tip()?, merge);
        assert_eq!(accepted(&s.w.server), pushes + 1);
    }
    Ok(())
}

fn c4_merging_remote_context_deleted() -> Result<(), FixtureError> {
    let s = Synced::new(Branch::Item)?;
    let local = s.local_commit("local-ahead", b"local-ahead")?;
    fixed(fixed(s.w.bare()?.find_reference(CONTEXT))?.delete())?;
    let pushes = accepted(&s.w.server);
    let refused = s.execute(REQUEST_1);
    assert_eq!(refused.code, ResultCode::RemoteBranchDeleted);
    assert_eq!(refused.outcome, Outcome::Blocked);
    assert!(!refused.effects.is_durable());
    assert!(refused.data.is_none());
    assert_eq!(actions(&refused), [RecoveryActionKind::RequestCancel]);
    // Nothing is recreated.
    assert!(s.w.bare()?.find_reference(CONTEXT).is_err());
    assert_eq!(accepted(&s.w.server), pushes);
    s.assert_no_merge(local)?;
    Ok(())
}

fn c4_merging_no_publication_remote() -> Result<(), FixtureError> {
    for branch in BOTH {
        let s = Synced::new(branch)?;
        let (local, incoming) = s.diverge()?;
        let config = s.w.root.join(".manyhands/config.toml");
        let source = fixed(std::fs::read_to_string(&config))?;
        fixed(std::fs::write(
            &config,
            source.replace("publication_remote = \"origin\"\n", ""),
        ))?;
        let opened = connections(&s.w);
        let refused = s.execute(REQUEST_1);
        assert_eq!(refused.code, ResultCode::PublicationRemoteRequired);
        assert_eq!(refused.outcome, Outcome::Blocked);
        assert_eq!(refused.effects, Effects::not_requested());
        assert!(refused.data.is_none());
        assert!(refused.recovery.is_empty());
        assert!(record(&s.w, REQUEST_1).is_none());
        assert_eq!(connections(&s.w), opened);
        assert_eq!(s.served_tip()?, incoming);
        assert_eq!(s.local_tip()?, local);
        assert_eq!(merge_commits(&s.w.repo()?)?, 0);
    }
    Ok(())
}

/// One call of the case's synchronization whose sink cancels its token
/// when it is told of `stage`: what a Ctrl-C at that point does.
fn cancelled_at(s: &Synced, id: &str, stage: ProgressStage) -> Envelope<MutationDataDto> {
    let token = CancellationToken::new();
    let mut sink = CancelAt {
        stage,
        token: token.clone(),
        seen: Vec::new(),
    };
    let envelope = call(
        &s.w,
        &s.w.service,
        id,
        s.mutation(),
        Provider,
        &mut sink,
        &token,
    );
    assert!(token.is_cancelled());
    envelope
}

fn c4_merging_cancelled_at_a_safe_point() -> Result<(), FixtureError> {
    for branch in BOTH {
        let s = Synced::new(branch)?;
        let (local, incoming) = s.diverge()?;
        let pushes = accepted(&s.w.server);
        // At the safe point before the push: the merge commit is made.
        let cancelled = cancelled_at(&s, REQUEST_1, ProgressStage::Pushing);
        let merge = s.local_tip()?;
        assert_eq!(cancelled.code, ResultCode::Cancelled);
        assert_eq!(cancelled.outcome, Outcome::Cancelled);
        // The effects name what completed: the merge commit, and nothing
        // published.
        assert_eq!(cancelled.effects, merged(publication_pending(), merge));
        assert!(cancelled.data.is_none());
        assert!(cancelled.recovery.is_empty());
        s.assert_merge(merge, local, incoming)?;
        assert_eq!(s.served_tip()?, incoming);
        assert_eq!(accepted(&s.w.server), pushes);
        assert!(!reserved(&s.w));
        // The operation is final (fact 56): the same request says so.
        let operation = OperationId::parse(cancelled.operation_id.as_deref().ok_or(FixtureError)?)
            .map_err(|_| FixtureError)?;
        assert_eq!(phase(&s.w, operation)?, "cancelled");
        assert_eq!(s.execute(REQUEST_1), cancelled);
        // A new request completes it: the merge commit is published, and
        // no other is made.
        let completed = s.execute(REQUEST_2);
        assert_eq!(completed.code, ResultCode::Ok);
        assert_eq!(completed.effects, super::published(merge));
        assert_eq!(s.served_tip()?, merge);
        assert_eq!(merge_commits(&s.w.repo()?)?, 1);
        assert_eq!(accepted(&s.w.server), pushes + 1);
    }
    Ok(())
}

fn c4_merging_cancelled_after_the_push_returned() -> Result<(), FixtureError> {
    for branch in BOTH {
        let s = Synced::new(branch)?;
        let (local, incoming) = s.diverge()?;
        let pushes = accepted(&s.w.server);
        let cancelled = cancelled_at(&s, REQUEST_1, ProgressStage::Verifying);
        let merge = s.local_tip()?;
        assert_eq!(cancelled.code, ResultCode::Cancelled);
        assert_eq!(cancelled.outcome, Outcome::Cancelled);
        // The merge commit, published, as the stored push says (fact 74).
        assert_eq!(
            cancelled.effects,
            merged(published_and_cancelled(merge), merge)
        );
        assert!(cancelled.recovery.is_empty());
        s.assert_merge(merge, local, incoming)?;
        assert_eq!(s.served_tip()?, merge);
        assert_eq!(accepted(&s.w.server), pushes + 1);
        // A new request verifies it, and pushes nothing.
        assert_current(&s.execute(REQUEST_2));
        assert_eq!(accepted(&s.w.server), pushes + 1);
        assert_eq!(merge_commits(&s.w.repo()?)?, 1);
    }
    Ok(())
}

fn c4_merging_unsettled_push_then_cancelled() -> Result<(), FixtureError> {
    for branch in BOTH {
        let s = Synced::new(branch)?;
        let (local, incoming) = s.diverge()?;
        // The server takes the merge's push, and the call never learns of
        // it.
        s.w.server.disconnect_at(FixtureBoundary::AfterReceivePack);
        let lost = s.execute(REQUEST_1);
        assert_eq!(lost.code, ResultCode::TransportUnavailable);
        // After a merge commit, which stays and is named.
        let merge = s.local_tip()?;
        assert_eq!(lost.outcome, Outcome::Partial);
        assert_eq!(lost.effects, merged(publication_pending(), merge));
        s.assert_merge(merge, local, incoming)?;
        // From here on the server cannot be reached at all.
        s.w.server.disconnect_at(FixtureBoundary::Handshake);
        let asked =
            s.w.service
                .cancel_request(request(REQUEST_1))
                .data
                .ok_or(FixtureError)?;
        assert!(asked.requested);
        // The later call begins with that unsettled push, and is
        // cancelled: the publication is not confirmed.
        let ended = s.execute(REQUEST_1);
        assert_eq!(ended.code, ResultCode::RecoveryRequired);
        assert_eq!(ended.outcome, Outcome::Partial);
        assert_eq!(ended.effects, merged(publication_pending(), merge));
        assert!(ended.data.is_none());
        assert_eq!(actions(&ended), [RecoveryActionKind::RepoInspect]);
        assert_eq!(ended.operation_id, lost.operation_id);
        assert!(!reserved(&s.w));
        assert_eq!(
            record(&s.w, REQUEST_1).ok_or(FixtureError)?.state,
            RequestState::Finished
        );
        assert_eq!(merge_commits(&s.w.repo()?)?, 1);
    }
    Ok(())
}

/// A call of the case's synchronization on another thread, by a service
/// of its own on the world's data, with `token` as its cancellation
/// token.
fn elsewhere(
    s: &Synced,
    id: &'static str,
    token: CancellationToken,
) -> Result<std::thread::JoinHandle<Envelope<MutationDataDto>>, FixtureError> {
    let service = fixed(RepositoryService::open_at(&s.w.data()))?;
    let root = s.w.root.clone();
    let branch = s.branch;
    Ok(std::thread::spawn(move || {
        let mutation = match branch {
            Branch::Primary => Mutation::RepoSync(RepoSyncInput { root }),
            Branch::Item => Mutation::ItemSync(ItemSyncInput {
                root,
                kind: AuthoringKind::Ticket,
                id: ITEM.parse().unwrap(),
            }),
        };
        service.execute(
            MutationCall {
                request_id: request(id),
                confirmation: None,
                mutation,
            },
            &mut SessionCredentials::new(Provider),
            &mut NoProgress,
            &token,
        )
    }))
}

/// What a Ctrl-C reaches the library as: the call's own token, cancelled
/// while the call is blocked inside a transport call (the fetch's
/// advertisement, held by the server), before it merges. The token is
/// read at the next safe point once the call returns: the answer is
/// `cancelled`, with its effects intact.
fn c4_merging_cancelled_during_a_blocked_network_call() -> Result<(), FixtureError> {
    for branch in BOTH {
        let s = Synced::new(branch)?;
        let (local, incoming) = s.diverge()?;
        let pushes = accepted(&s.w.server);
        let hold = s.w.server.hold_advertisement()?;
        let token = CancellationToken::new();
        let running = elsewhere(&s, REQUEST_1, token.clone())?;
        hold.wait_until_held()?;
        token.cancel();
        hold.release()?;
        let cancelled = joined(&s.w, running)?;
        assert_eq!(cancelled.code, ResultCode::Cancelled);
        assert_eq!(cancelled.outcome, Outcome::Cancelled);
        // Nothing merged and nothing published.
        assert_eq!(cancelled.effects, publication_pending());
        assert!(cancelled.data.is_none());
        assert!(cancelled.recovery.is_empty());
        assert!(!reserved(&s.w));
        assert_eq!(
            record(&s.w, REQUEST_1).ok_or(FixtureError)?.state,
            RequestState::Finished
        );
        assert_eq!(accepted(&s.w.server), pushes);
        assert_eq!(s.served_tip()?, incoming);
        s.assert_no_merge(local)?;
    }
    Ok(())
}
