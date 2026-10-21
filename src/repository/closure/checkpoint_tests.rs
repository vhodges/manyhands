use super::super::test_support::{
    self as support, ID, at, checkout, commit, id, parse_ticket, read, repository_ticket, ticket,
    write,
};
use super::*;
use crate::repository::{
    OperationId, PreviewTicketClosureRequest, RepositoryErrorKind, RepositoryOperation,
    closure::{
        observe::{closed_ticket_bytes, prepare_observation},
        record::{self, ClosureKind, ClosureStep},
    },
    coordination::repository_lease,
    promotion::{
        DocumentPromotionError, integrate,
        lifecycle::{LeasedIntegration, PreIntegration},
    },
    remote::state,
};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

const CLOSER: &str = "Closer Sentinel <closer-sentinel@example.invalid>";

fn closer() -> CommitIdentity {
    CommitIdentity {
        name: "Closer Sentinel".into(),
        email: "closer-sentinel@example.invalid".into(),
    }
}
fn context_ref() -> String {
    format!("refs/heads/manyhands/ticket/{ID}")
}
fn tree_entry(repo: &git2::Repository, tree: Oid, path: &str) -> (Vec<u8>, i32) {
    let tree = repo.find_tree(tree).unwrap();
    let entry = tree.get_path(Path::new(path)).unwrap();
    (
        repo.find_blob(entry.id()).unwrap().content().to_vec(),
        entry.filemode(),
    )
}
fn commit_entry(repo: &git2::Repository, commit: Oid, path: &str) -> (Vec<u8>, i32) {
    tree_entry(repo, repo.find_commit(commit).unwrap().tree_id(), path)
}
/// A detached commit over `parent` replacing one path's bytes and mode.
fn tree_commit(repo: &git2::Repository, parent: Oid, path: &str, bytes: &[u8], mode: u32) -> Oid {
    support::tree_commit(repo, parent, &[(path, Some((bytes, mode)))])
}
/// Every ref, and every file under the root including `.git`, hashed.
fn image(
    root: &Path,
    repo: &git2::Repository,
) -> (BTreeMap<String, Oid>, BTreeMap<PathBuf, Vec<u8>>) {
    fn visit(base: &Path, path: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                // Loose objects are the only thing prepare may add.
                if path.ends_with(".git/objects") {
                    continue;
                }
                visit(base, &path, out);
            } else if !path.ends_with("manyhands-operation.lock") {
                out.insert(
                    path.strip_prefix(base).unwrap().into(),
                    blake3::hash(&fs::read(&path).unwrap()).as_bytes().to_vec(),
                );
            }
        }
    }
    let refs = repo
        .references()
        .unwrap()
        .filter_map(|r| {
            let r = r.ok()?;
            Some((r.name()?.to_owned(), r.target()?))
        })
        .collect();
    let mut files = BTreeMap::new();
    visit(root, root, &mut files);
    (refs, files)
}

/// The shared closure fixture, with this module's own helpers.
struct Fixture(support::Fixture);
impl std::ops::Deref for Fixture {
    type Target = support::Fixture;
    fn deref(&self) -> &support::Fixture {
        &self.0
    }
}

impl Fixture {
    /// An enabled repository with a ticket context whose body differs from
    /// the primary's open ticket.
    fn new(remote: bool) -> Self {
        Self(support::Fixture::new(remote))
    }
    fn context_oid(&self) -> Oid {
        self.repo.refname_to_id(&context_ref()).unwrap()
    }
    fn primary(&self) -> Oid {
        self.repo.refname_to_id("refs/heads/main").unwrap()
    }
    /// Commits `edit` of the ticket's text on the primary worktree.
    fn advance_primary(&self, edit: impl Fn(String) -> String) -> Oid {
        let source = String::from_utf8(read(&self.repo, &ticket())).unwrap();
        write(&self.repo, &ticket(), edit(source));
        commit(&self.repo)
    }
    fn observation_with(
        &self,
        confirmation: Option<ConfirmedCommitIdentity>,
    ) -> ClosureObservation {
        match prepare_observation(
            &self.service,
            &PreviewTicketClosureRequest {
                root: self.root.path().into(),
                item_id: id(),
                confirmed_identity: confirmation,
            },
        )
        .unwrap()
        .observed
        {
            Ok(o) => o,
            Err(refusal) => panic!("expected ready: {:?}", refusal.blockers),
        }
    }
    fn observation(&self) -> ClosureObservation {
        self.observation_with(None)
    }
    /// A closure record at `context_synchronized` for the current context.
    fn record(&self, kind: ClosureKind) -> ClosureRecord {
        let observation = self.observation();
        let context = self.context_oid();
        state::with_transaction(&self.service, self.root.path(), |tx, rid| {
            let mut r = record::begin_record(
                tx,
                rid,
                OperationId::new(),
                &id(),
                observation.expectation,
                observation.remote.as_deref(),
                kind,
            )?;
            r.context_oid = Some(context);
            r.remote_context_oid = observation.tracking_context;
            r.step = ClosureStep::ContextSynchronized;
            record::save_record(tx, rid, &r)?;
            Ok(r)
        })
        .unwrap()
    }
    /// A `closing` record whose checkpoint was built but not referenced.
    fn closing(&self, closed_at: OffsetDateTime) -> (ClosureRecord, PreparedClosure) {
        let r = self.record(ClosureKind::Record);
        let prepared =
            prepare_closure(&self.repo, &self.observation(), closed_at, &closer()).unwrap();
        state::with_transaction(&self.service, self.root.path(), |tx, rid| {
            record::begin_closing(
                tx,
                rid,
                r.operation_id,
                self.base,
                prepared.closed_at,
                prepared.checkpoint_commit,
            )
        })
        .unwrap();
        (self.reload(&r), prepared)
    }
    fn reload(&self, r: &ClosureRecord) -> ClosureRecord {
        state::with_transaction(&self.service, self.root.path(), |tx, rid| {
            record::read_record(tx, rid, r.operation_id)
        })
        .unwrap()
        .unwrap()
    }
    fn integration_context(&self, oid: Oid) -> integrate::IntegrationContext {
        integrate::IntegrationContext {
            item_id: id(),
            primary_branch: "main".into(),
            oid,
        }
    }
    /// The closure hook under the lease, as the generic integration calls it
    /// after revalidation.
    fn hook_with(
        &self,
        service: &RepositoryService,
        r: &ClosureRecord,
        confirmation: Option<&ConfirmedCommitIdentity>,
    ) -> Result<PreIntegration, DocumentPromotionError> {
        let observation = self.observation_with(confirmation.cloned());
        self.hook_observed(service, &observation, r, confirmation)
    }
    /// The hook with a given observation, as revalidation hands it over.
    fn hook_observed(
        &self,
        service: &RepositoryService,
        observation: &ClosureObservation,
        r: &ClosureRecord,
        confirmation: Option<&ConfirmedCommitIdentity>,
    ) -> Result<PreIntegration, DocumentPromotionError> {
        let _lease =
            repository_lease(&self.repo, self.root.path(), RepositoryOperation::Read).unwrap();
        let context = self.integration_context(r.context_oid.unwrap());
        let mut stage = LeasedIntegration {
            repository: &self.repo,
            root: self.root.path(),
            context: &context,
            base: observation.base,
            record: Some(r),
        };
        pre_integration(service, observation, confirmation, &mut stage)
    }
    fn hook(&self, r: &ClosureRecord) -> Result<PreIntegration, DocumentPromotionError> {
        self.hook_with(&self.service, r, None)
    }
    /// The written-closure resume under the lease.
    fn resume(&self, r: &ClosureRecord) -> Result<Option<PreIntegration>, DocumentPromotionError> {
        let _lease =
            repository_lease(&self.repo, self.root.path(), RepositoryOperation::Read).unwrap();
        let context = self.integration_context(r.context_oid.unwrap());
        let mut stage = LeasedIntegration {
            repository: &self.repo,
            root: self.root.path(),
            context: &context,
            base: self.primary(),
            record: Some(r),
        };
        resume_written(&self.service, &mut stage)
    }
    /// Applies a hook's checkpoint through the shared integration.
    fn integrate(&self, checkpoint: PreIntegration) -> Oid {
        let lease =
            repository_lease(&self.repo, self.root.path(), RepositoryOperation::Read).unwrap();
        let prepared = integrate::prepare_checkpoint_integration::<DocumentPromotionError>(
            LifecycleKind::TicketClosure,
            git2::Repository::open(self.root.path()).unwrap(),
            self.root.path().into(),
            lease,
            self.integration_context(self.context_oid()),
            self.primary(),
            checkpoint,
        )
        .unwrap();
        match prepared
            .write_commit_for::<DocumentPromotionError>()
            .unwrap()
            .apply_for::<DocumentPromotionError>()
            .unwrap()
        {
            integrate::Integrated::Merged { oid } => oid,
            other => panic!("expected a merge: {other:?}"),
        }
    }
    fn statuses(&self) -> Vec<(String, git2::Status)> {
        let mut options = git2::StatusOptions::new();
        options.include_untracked(true).recurse_untracked_dirs(true);
        self.context
            .statuses(Some(&mut options))
            .unwrap()
            .iter()
            .map(|e| (e.path().unwrap().to_owned(), e.status()))
            .collect()
    }
}

fn checkpoint_of(result: PreIntegration) -> (Oid, Oid, Oid) {
    match result {
        PreIntegration::Checkpoint {
            base,
            merged_context,
            result_tree,
            ..
        } => (base, merged_context, result_tree),
        PreIntegration::None => panic!("expected a checkpoint"),
    }
}

// --- Review Focus 1–3 (R-P1: the one shared `closed_ticket_bytes`) ---

fn assert_only_closure_added(source: &str, closed: &[u8]) {
    let original = parse_ticket(source.as_bytes());
    let after = parse_ticket(closed);
    assert_eq!(after.id, original.id);
    assert_eq!(after.title, original.title);
    assert_eq!(after.ticket_type, original.ticket_type);
    assert_eq!(after.status, original.status);
    assert_eq!(after.project, original.project);
    assert_eq!(after.team, original.team);
    assert_eq!(after.body, original.body);
    assert_eq!(after.unknown, original.unknown);
    assert_eq!(
        after.unknown.get("wave"),
        Some(&serde_yaml::Value::from("02"))
    );
    assert_eq!(
        after.unknown.get("cycle"),
        Some(&serde_yaml::Value::from("10"))
    );
    assert_eq!(
        (original.closed_at, original.closed_by.clone()),
        (None, None)
    );
    assert_eq!(after.closed_at, Some(at(1_700_000_000)));
    assert_eq!(after.closed_by.as_deref(), Some(CLOSER));
    let mut expected = original;
    expected.closed_at = Some(at(1_700_000_000));
    expected.closed_by = Some(CLOSER.into());
    assert_eq!(after, expected);
}

#[test]
fn closed_ticket_bytes_preserves_hand_written_fields() {
    let source = repository_ticket("\n");
    let closed = closed_ticket_bytes(source.as_bytes(), at(1_700_000_000), CLOSER).unwrap();
    assert_only_closure_added(&source, &closed);
    // Canonical re-serialization: keys move, values and the body do not.
    let text = String::from_utf8(closed).unwrap();
    assert!(text.contains("closed_at: 2023-11-14T22:13:20Z\n"), "{text}");
    assert!(text.ends_with("- [x] Create the implementation plan.\n"));
}

#[test]
fn closed_ticket_bytes_crlf() {
    let source = repository_ticket("\r\n");
    let closed = closed_ticket_bytes(source.as_bytes(), at(1_700_000_000), CLOSER).unwrap();
    assert_only_closure_added(&source, &closed);
    // The body keeps its CRLF line endings byte for byte.
    assert!(
        String::from_utf8(closed)
            .unwrap()
            .ends_with("## Planning\r\n\r\n- [x] Create the implementation plan.\r\n")
    );
}

#[test]
fn checkpoint_preserves_executable_mode() {
    let f = Fixture::new(false);
    let bytes = read(&f.context, &ticket());
    let executable = tree_commit(&f.context, f.context_oid(), &ticket(), &bytes, 0o100755);
    checkout(&f.context, executable);
    assert_eq!(f.context_oid(), executable);
    let prepared =
        prepare_closure(&f.repo, &f.observation(), at(1_700_000_000), &closer()).unwrap();
    let (closed, mode) = commit_entry(&f.repo, prepared.checkpoint_commit, &ticket());
    assert_eq!(mode, 0o100755);
    assert_eq!(
        f.repo.find_blob(prepared.ticket_blob).unwrap().content(),
        closed
    );
    assert_eq!(
        tree_entry(&f.repo, prepared.result_tree, &ticket()).1,
        0o100755
    );
}

#[test]
fn closed_at_is_whole_utc_seconds() {
    let f = Fixture::new(false);
    let local = at(1_700_000_000)
        .replace_nanosecond(987_654_321)
        .unwrap()
        .to_offset(time::UtcOffset::from_hms(5, 30, 0).unwrap());
    let prepared = prepare_closure(&f.repo, &f.observation(), local, &closer()).unwrap();
    assert_eq!(prepared.closed_at, at(1_700_000_000));
    assert_eq!(prepared.closed_at.offset(), time::UtcOffset::UTC);
    let text = String::from_utf8(
        f.repo
            .find_blob(prepared.ticket_blob)
            .unwrap()
            .content()
            .to_vec(),
    )
    .unwrap();
    assert!(text.contains("closed_at: 2023-11-14T22:13:20Z\n"), "{text}");
    assert_eq!(whole_utc_seconds(local), at(1_700_000_000));
    let now = now_whole_utc_seconds();
    assert_eq!((now.offset(), now.nanosecond()), (time::UtcOffset::UTC, 0));
}

// --- The checkpoint objects ---

#[test]
fn checkpoint_commit_has_exact_subject_parent_and_single_path() {
    let f = Fixture::new(false);
    let context = f.context_oid();
    let observation = f.observation();
    let prepared = prepare_closure(&f.repo, &observation, at(1_700_000_000), &closer()).unwrap();
    let commit = f.repo.find_commit(prepared.checkpoint_commit).unwrap();
    assert_eq!(
        commit.summary(),
        Some(format!("Close ticket {ID}").as_str())
    );
    assert_eq!(
        commit.message(),
        Some(format!("Close ticket {ID}").as_str())
    );
    assert_eq!(commit.parent_ids().collect::<Vec<_>>(), vec![context]);
    for signature in [commit.author(), commit.committer()] {
        assert_eq!(signature.name(), Some("Closer Sentinel"));
        assert_eq!(signature.email(), Some("closer-sentinel@example.invalid"));
        assert_eq!(signature.when().seconds(), 1_700_000_000);
        assert_eq!(signature.when().offset_minutes(), 0);
    }
    let diff = f
        .repo
        .diff_tree_to_tree(
            Some(&f.repo.find_commit(context).unwrap().tree().unwrap()),
            Some(&commit.tree().unwrap()),
            None,
        )
        .unwrap();
    let paths: Vec<_> = diff
        .deltas()
        .map(|d| d.new_file().path().unwrap().to_owned())
        .collect();
    assert_eq!(paths, vec![PathBuf::from(ticket())]);
    let (closed, mode) = commit_entry(&f.repo, prepared.checkpoint_commit, &ticket());
    assert_eq!(mode, 0o100644);
    assert_eq!(
        closed,
        closed_ticket_bytes(&read(&f.context, &ticket()), at(1_700_000_000), CLOSER).unwrap()
    );
    // The same inputs build the same commit (a resume reuses it).
    assert_eq!(
        prepare_closure(&f.repo, &observation, at(1_700_000_000), &closer())
            .unwrap()
            .checkpoint_commit,
        prepared.checkpoint_commit
    );
    // `base ≤ context`: the result is the checkpoint tree itself.
    assert_eq!(prepared.result_tree, commit.tree_id());
}

#[test]
fn prepare_builds_nothing_referenced() {
    let f = Fixture::new(false);
    let observation = f.observation();
    // A divergent primary forces the in-memory merge too.
    f.advance_primary(|s| s);
    write(&f.repo, "primary.txt", "primary\n");
    commit(&f.repo);
    let before = image(f.root.path(), &f.repo);
    let prepared =
        prepare_closure(&f.repo, &f.observation(), at(1_700_000_000), &closer()).unwrap();
    assert_eq!(image(f.root.path(), &f.repo), before);
    assert!(f.repo.find_commit(prepared.checkpoint_commit).is_ok());
    assert!(f.repo.find_tree(prepared.result_tree).is_ok());
    assert_ne!(observation.base, f.observation().base);
}

#[test]
fn no_remote_conflicting_merge_refuses_before_any_ref_moves() {
    let f = Fixture::new(false);
    let r = f.record(ClosureKind::Record);
    let mut observation = f.observation();
    // The primary edits the line the closure fields are inserted next to.
    // The preview now reports that as `IntegrationConflict` (final review
    // I-1); execution's refusal stays the safety net for an observation
    // that did not see it, here one handed over with the moved base.
    f.advance_primary(|s| s.replace("status: open", "status: doing"));
    observation.base = f.primary();
    assert_eq!(observation.integration, PlannedIntegration::MergeCommit);
    assert!(matches!(
        prepare_closure(&f.repo, &observation, at(1_700_000_000), &closer()),
        Err(ClosureStop::Refused)
    ));
    let before = image(f.root.path(), &f.repo);
    let result = f.hook_observed(&f.service, &observation, &r, None);
    assert!(
        matches!(result, Err(DocumentPromotionError::ReconfirmationRequired)),
        "{result:?}"
    );
    assert_eq!(image(f.root.path(), &f.repo).0, before.0);
    assert_eq!(f.reload(&r).step, ClosureStep::ContextSynchronized);
    assert_eq!(f.reload(&r).closure_oid, None);
}

#[test]
fn move_context_leaves_worktree_clean_at_checkpoint() {
    let f = Fixture::new(false);
    let context = f.context_oid();
    let prepared =
        prepare_closure(&f.repo, &f.observation(), at(1_700_000_000), &closer()).unwrap();
    move_context_to_checkpoint(&f.context, &id(), context, prepared.checkpoint_commit).unwrap();
    assert_eq!(f.context_oid(), prepared.checkpoint_commit);
    assert_eq!(f.statuses(), vec![]);
    assert_eq!(
        read(&f.context, &ticket()),
        f.repo.find_blob(prepared.ticket_blob).unwrap().content()
    );
    // The guard: a second move from the old tip is refused.
    assert!(
        move_context_to_checkpoint(&f.context, &id(), context, prepared.checkpoint_commit).is_err()
    );
    assert_eq!(f.primary(), f.base);
}

// --- reconcile_closing, one row each ---

#[test]
fn reconcile_ref_at_context_and_clean_is_nothing_written() {
    let f = Fixture::new(false);
    let (r, _) = f.closing(at(1_700_000_000));
    assert_eq!(
        reconcile_closing(&f.context, &r).unwrap(),
        ClosingState::NothingWritten
    );
}

/// Final review M-1: an unchanged ticket path proves the checkout never ran,
/// whatever else the worktree holds; revalidation reports the stray file as
/// `ContextDirty`.
#[test]
fn reconcile_unchanged_ticket_with_stray_file_is_nothing_written() {
    let f = Fixture::new(false);
    let (r, _) = f.closing(at(1_700_000_000));
    write(&f.context, "stray.txt", "stray\n");
    assert_eq!(
        f.statuses(),
        vec![("stray.txt".into(), git2::Status::WT_NEW)]
    );
    assert_eq!(
        reconcile_closing(&f.context, &r).unwrap(),
        ClosingState::NothingWritten
    );
}

#[test]
fn reconcile_checkout_without_ref_update_is_checkout_done() {
    // R-T0-3: a stop between checkout and ref update leaves the ticket path
    // changed in both the index and the worktree.
    let f = Fixture::new(false);
    let (r, prepared) = f.closing(at(1_700_000_000));
    let stopped = move_context_to_checkpoint_with(
        &f.context,
        &id(),
        r.context_oid.unwrap(),
        prepared.checkpoint_commit,
        || Err(state::recovery_required()),
    );
    assert!(stopped.is_err());
    assert_eq!(f.context_oid(), r.context_oid.unwrap());
    let statuses = f.statuses();
    assert_eq!(statuses.len(), 1);
    assert_eq!(statuses[0].0, ticket());
    assert!(statuses[0].1.contains(git2::Status::INDEX_MODIFIED));
    assert_eq!(
        reconcile_closing(&f.context, &r).unwrap(),
        ClosingState::CheckoutDone
    );
}

#[test]
fn reconcile_worktree_only_checkout_is_checkout_done() {
    let f = Fixture::new(false);
    let (r, prepared) = f.closing(at(1_700_000_000));
    write(
        &f.context,
        &ticket(),
        f.repo.find_blob(prepared.ticket_blob).unwrap().content(),
    );
    let statuses = f.statuses();
    assert_eq!(statuses, vec![(ticket(), git2::Status::WT_MODIFIED)]);
    assert_eq!(
        reconcile_closing(&f.context, &r).unwrap(),
        ClosingState::CheckoutDone
    );
}

#[test]
fn reconcile_ticket_differing_from_the_checkpoint_is_unrecognized() {
    let f = Fixture::new(false);
    let (r, _) = f.closing(at(1_700_000_000));
    write(&f.context, &ticket(), "hand edited\n");
    assert_eq!(
        reconcile_closing(&f.context, &r).unwrap(),
        ClosingState::Unrecognized
    );
}

#[test]
fn reconcile_another_changed_path_is_unrecognized() {
    let f = Fixture::new(false);
    let (r, prepared) = f.closing(at(1_700_000_000));
    write(
        &f.context,
        &ticket(),
        f.repo.find_blob(prepared.ticket_blob).unwrap().content(),
    );
    write(&f.context, "other.txt", "stray\n");
    assert_eq!(
        reconcile_closing(&f.context, &r).unwrap(),
        ClosingState::Unrecognized
    );
}

#[test]
fn reconcile_ref_at_closure_is_checkpoint_written() {
    let f = Fixture::new(false);
    let (r, prepared) = f.closing(at(1_700_000_000));
    move_context_to_checkpoint(
        &f.context,
        &id(),
        r.context_oid.unwrap(),
        prepared.checkpoint_commit,
    )
    .unwrap();
    assert_eq!(
        reconcile_closing(&f.context, &r).unwrap(),
        ClosingState::CheckpointWritten {
            tip: prepared.checkpoint_commit
        }
    );
}

#[test]
fn reconcile_adoptable_descendant_is_checkpoint_written() {
    let f = Fixture::new(false);
    let (r, prepared) = f.closing(at(1_700_000_000));
    move_context_to_checkpoint(
        &f.context,
        &id(),
        r.context_oid.unwrap(),
        prepared.checkpoint_commit,
    )
    .unwrap();
    // A deliberate synchronization merging a primary advance over the
    // checkpoint: the ticket keeps the same closure.
    write(&f.repo, "primary.txt", "primary\n");
    let advanced = commit(&f.repo);
    let sig = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
    let local = f.context.find_commit(prepared.checkpoint_commit).unwrap();
    let incoming = f.context.find_commit(advanced).unwrap();
    let mut index = f.context.merge_commits(&local, &incoming, None).unwrap();
    let tree = f
        .context
        .find_tree(index.write_tree_to(&f.context).unwrap())
        .unwrap();
    let merge = f
        .context
        .commit(None, &sig, &sig, "synchronize", &tree, &[&local, &incoming])
        .unwrap();
    checkout(&f.context, merge);
    assert_eq!(
        reconcile_closing(&f.context, &r).unwrap(),
        ClosingState::CheckpointWritten { tip: merge }
    );
}

#[test]
fn reconcile_descendant_with_different_closure_is_unrecognized() {
    let f = Fixture::new(false);
    let (r, prepared) = f.closing(at(1_700_000_000));
    move_context_to_checkpoint(
        &f.context,
        &id(),
        r.context_oid.unwrap(),
        prepared.checkpoint_commit,
    )
    .unwrap();
    let other = closed_ticket_bytes(
        &commit_entry(&f.repo, r.context_oid.unwrap(), &ticket()).0,
        at(1_700_000_001),
        CLOSER,
    )
    .unwrap();
    let changed = tree_commit(
        &f.context,
        prepared.checkpoint_commit,
        &ticket(),
        &other,
        0o100644,
    );
    checkout(&f.context, changed);
    assert_eq!(
        reconcile_closing(&f.context, &r).unwrap(),
        ClosingState::Unrecognized
    );
}

#[test]
fn reconcile_unrelated_tip_is_unrecognized() {
    let f = Fixture::new(false);
    let (r, _) = f.closing(at(1_700_000_000));
    write(&f.context, "other.txt", "other\n");
    commit(&f.context);
    assert_eq!(
        reconcile_closing(&f.context, &r).unwrap(),
        ClosingState::Unrecognized
    );
}

// --- Authority and the post-checkpoint merge (C9) ---

#[test]
fn checkpoint_authority_reads_commit_not_config() {
    let f = Fixture::new(false);
    let prepared =
        prepare_closure(&f.repo, &f.observation(), at(1_700_000_000), &closer()).unwrap();
    let mut config = f.repo.config().unwrap();
    config.set_str("user.name", "Changed").unwrap();
    config
        .set_str("user.email", "changed@example.invalid")
        .unwrap();
    let authority = checkpoint_authority(&f.repo, &id(), prepared.checkpoint_commit).unwrap();
    assert_eq!(authority.closed_by, CLOSER);
    assert_eq!(authority.signature.name, "Closer Sentinel");
    assert_eq!(authority.signature.email, "closer-sentinel@example.invalid");
    // Not a checkpoint, or another ticket's: refused.
    assert!(checkpoint_authority(&f.repo, &id(), f.base).is_err());
    let other: ItemId = "01BX5ZZKBKACTAV9WEVGEMMVRZ".parse().unwrap();
    assert!(checkpoint_authority(&f.repo, &other, prepared.checkpoint_commit).is_err());
}

#[test]
fn post_checkpoint_merges_advanced_base_in_memory() {
    let f = Fixture::new(false);
    let prepared =
        prepare_closure(&f.repo, &f.observation(), at(1_700_000_000), &closer()).unwrap();
    // Without the advance, `base ≤ tip`: the tip's tree.
    assert_eq!(
        post_checkpoint_result(&f.repo, f.base, prepared.checkpoint_commit)
            .unwrap()
            .unwrap(),
        f.repo
            .find_commit(prepared.checkpoint_commit)
            .unwrap()
            .tree_id()
    );
    write(&f.repo, "primary.txt", "primary\n");
    let advanced = commit(&f.repo);
    let before = image(f.root.path(), &f.repo);
    let tree = post_checkpoint_result(&f.repo, advanced, prepared.checkpoint_commit)
        .unwrap()
        .unwrap();
    assert_eq!(image(f.root.path(), &f.repo), before);
    assert_eq!(tree_entry(&f.repo, tree, "primary.txt").0, b"primary\n");
    assert_eq!(
        tree_entry(&f.repo, tree, &ticket()).0,
        f.repo.find_blob(prepared.ticket_blob).unwrap().content()
    );
}

#[test]
fn post_checkpoint_conflict_reports_conflict() {
    let f = Fixture::new(false);
    let prepared =
        prepare_closure(&f.repo, &f.observation(), at(1_700_000_000), &closer()).unwrap();
    let advanced = f.advance_primary(|s| s.replace("status: open", "status: doing"));
    let before = image(f.root.path(), &f.repo);
    assert_eq!(
        post_checkpoint_result(&f.repo, advanced, prepared.checkpoint_commit).unwrap(),
        Err(())
    );
    assert_eq!(image(f.root.path(), &f.repo).0, before.0);
}

// --- The hook ---

#[test]
fn hook_writes_closing_then_moves_the_context_and_marks_refresh() {
    let f = Fixture::new(false);
    let r = f.record(ClosureKind::Record);
    let context = f.context_oid();
    f.db()
        .execute("UPDATE repositories SET refresh_required=0", [])
        .unwrap();
    let (base, merged, tree) = checkpoint_of(f.hook(&r).unwrap());
    let stored = f.reload(&r);
    assert_eq!(stored.step, ClosureStep::Closing);
    assert_eq!(stored.closure_oid, Some(merged));
    assert_eq!(stored.base_oid, Some(base));
    assert_eq!(stored.context_oid, Some(context));
    assert_eq!(base, f.base);
    assert_eq!(f.context_oid(), merged);
    assert_eq!(f.statuses(), vec![]);
    assert_eq!(tree, f.repo.find_commit(merged).unwrap().tree_id());
    assert_eq!(f.primary(), f.base);
    let closed_at = stored.closed_at.unwrap();
    assert_eq!(
        (closed_at.offset(), closed_at.nanosecond()),
        (time::UtcOffset::UTC, 0)
    );
    let refresh: bool = f
        .db()
        .query_row("SELECT refresh_required FROM repositories", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert!(refresh);
    // Integration merges the checkpoint with the exact closure subject.
    let merge = f.integrate(f.hook_resumed(&stored));
    let commit = f.repo.find_commit(merge).unwrap();
    assert_eq!(
        commit.summary(),
        Some(format!("Integrate closed ticket {ID}").as_str())
    );
    assert_eq!(
        commit.parent_ids().collect::<Vec<_>>(),
        vec![f.base, merged]
    );
    assert_eq!(f.primary(), merge);
}

#[test]
fn failure_points_stop_at_the_design_positions() {
    for (point, expected) in [
        (FailurePoint::ClosureAfterClosingRecorded, "nothing_written"),
        (
            FailurePoint::ClosureDuringCheckpointCheckout,
            "checkout_done",
        ),
        (FailurePoint::AfterClosureCheckpoint, "checkpoint_written"),
    ] {
        let f = Fixture::new(false);
        let r = f.record(ClosureKind::Record);
        let failing =
            RepositoryService::open_at_with_failure_point_for_testing(f.data.path(), point)
                .unwrap();
        let result = f.hook_with(&failing, &r, None);
        assert!(
            matches!(&result, Err(DocumentPromotionError::Repository(e)) if e.kind == RepositoryErrorKind::InjectedFailure),
            "{point:?}: {result:?}"
        );
        let stored = f.reload(&r);
        assert_eq!(stored.step, ClosureStep::Closing, "{point:?}");
        let state = reconcile_closing(&f.context, &stored).unwrap();
        let name = match state {
            ClosingState::NothingWritten => "nothing_written",
            ClosingState::CheckoutDone => "checkout_done",
            ClosingState::CheckpointWritten { tip } => {
                assert_eq!(Some(tip), stored.closure_oid);
                "checkpoint_written"
            }
            ClosingState::Unrecognized => "unrecognized",
        };
        assert_eq!(name, expected, "{point:?}");
        assert_eq!(f.primary(), f.base, "{point:?}");
    }
}

#[test]
fn resume_nothing_written_rebuilds_the_same_checkpoint_with_the_stored_time() {
    let f = Fixture::new(false);
    let r = f.record(ClosureKind::Record);
    let failing = RepositoryService::open_at_with_failure_point_for_testing(
        f.data.path(),
        FailurePoint::ClosureAfterClosingRecorded,
    )
    .unwrap();
    assert!(f.hook_with(&failing, &r, None).is_err());
    let stored = f.reload(&r);
    assert_eq!(f.resume(&stored).unwrap(), None);
    let (_, merged, _) = checkpoint_of(f.hook(&stored).unwrap());
    let again = f.reload(&r);
    assert_eq!(again.closure_oid, Some(merged));
    assert_eq!(stored.closure_oid, Some(merged));
    assert_eq!(again.closed_at, stored.closed_at);
    assert_eq!(f.context_oid(), merged);
}

#[test]
fn resume_checkout_done_finishes_the_ref_update() {
    let f = Fixture::new(false);
    let r = f.record(ClosureKind::Record);
    let failing = RepositoryService::open_at_with_failure_point_for_testing(
        f.data.path(),
        FailurePoint::ClosureDuringCheckpointCheckout,
    )
    .unwrap();
    assert!(f.hook_with(&failing, &r, None).is_err());
    let stored = f.reload(&r);
    let (base, merged, _) = checkpoint_of(f.resume(&stored).unwrap().unwrap());
    assert_eq!(Some(merged), stored.closure_oid);
    assert_eq!(base, f.base);
    assert_eq!(f.context_oid(), merged);
    assert_eq!(f.statuses(), vec![]);
}

#[test]
fn resume_after_checkpoint_uses_commit_authority_and_current_base() {
    let f = Fixture::new(false);
    let r = f.record(ClosureKind::Record);
    let failing = RepositoryService::open_at_with_failure_point_for_testing(
        f.data.path(),
        FailurePoint::AfterClosureCheckpoint,
    )
    .unwrap();
    assert!(f.hook_with(&failing, &r, None).is_err());
    let stored = f.reload(&r);
    // Configuration and the primary both change after the stop.
    let mut config = f.repo.config().unwrap();
    config.set_str("user.name", "Changed").unwrap();
    config
        .set_str("user.email", "changed@example.invalid")
        .unwrap();
    write(&f.repo, "primary.txt", "primary\n");
    let advanced = commit(&f.repo);
    let resumed = f.resume(&stored).unwrap().unwrap();
    let PreIntegration::Checkpoint {
        base,
        merged_context,
        result_tree,
        ref signer,
    } = resumed
    else {
        panic!("expected a checkpoint")
    };
    assert_eq!(base, advanced);
    assert_eq!(Some(merged_context), stored.closure_oid);
    // The checkpoint's author, not the changed configuration.
    assert_eq!(signer.email, "fixture@example.invalid");
    assert_eq!(
        tree_entry(&f.repo, result_tree, "primary.txt").0,
        b"primary\n"
    );
    // The record now carries the base the merge uses.
    assert_eq!(f.reload(&r).base_oid, Some(advanced));
    let merge = f.integrate(resumed);
    let commit = f.repo.find_commit(merge).unwrap();
    assert_eq!(commit.author().email(), Some("fixture@example.invalid"));
    assert_eq!(
        commit.parent_ids().collect::<Vec<_>>(),
        vec![advanced, merged_context]
    );
}

#[test]
fn resume_after_checkpoint_conflict_is_recovery_required_and_record_kept() {
    let f = Fixture::new(false);
    let r = f.record(ClosureKind::Record);
    let failing = RepositoryService::open_at_with_failure_point_for_testing(
        f.data.path(),
        FailurePoint::AfterClosureCheckpoint,
    )
    .unwrap();
    assert!(f.hook_with(&failing, &r, None).is_err());
    let stored = f.reload(&r);
    f.advance_primary(|s| s.replace("status: open", "status: doing"));
    let result = f.resume(&stored);
    assert!(
        matches!(result, Err(DocumentPromotionError::RecoveryRequired)),
        "{result:?}"
    );
    assert_eq!(f.reload(&r), stored);
}

#[test]
fn contained_open_closes_with_in_memory_merge() {
    let f = Fixture::new(false);
    // The context was merged by hand without closure fields, then the
    // primary moved on.
    checkout(&f.repo, f.context_oid());
    write(&f.repo, "primary.txt", "primary\n");
    let base = commit(&f.repo);
    let observation = f.observation();
    assert_eq!(observation.integration, PlannedIntegration::MergeCommit);
    assert!(!observation.needs_context_child);
    let r = f.record(ClosureKind::Record);
    let (hook_base, merged, tree) = checkpoint_of(f.hook(&r).unwrap());
    assert_eq!(hook_base, base);
    assert_eq!(
        f.repo
            .find_commit(merged)
            .unwrap()
            .parent_ids()
            .collect::<Vec<_>>(),
        vec![r.context_oid.unwrap()]
    );
    assert_eq!(tree_entry(&f.repo, tree, "primary.txt").0, b"primary\n");
    let merge = f.integrate(f.hook_resumed(&f.reload(&r)));
    assert_eq!(
        f.repo
            .find_commit(merge)
            .unwrap()
            .parent_ids()
            .collect::<Vec<_>>(),
        vec![base, merged]
    );
    let closed = parse_ticket(&commit_entry(&f.repo, merge, &ticket()).0);
    assert_eq!(closed.status, "open");
    assert!(closed.closed_at.is_some() && closed.closed_by.is_some());
}

#[test]
fn existing_closure_writes_no_checkpoint() {
    let f = Fixture::new(false);
    let closed =
        closed_ticket_bytes(&read(&f.context, &ticket()), at(1_700_000_000), CLOSER).unwrap();
    write(&f.context, &ticket(), closed);
    commit(&f.context);
    let r = f.record(ClosureKind::Existing);
    // Saving the fixture can advance the persisted timestamp before either operation.
    let r = f.reload(&r);
    let before = image(f.root.path(), &f.repo);
    assert_eq!(f.hook(&r).unwrap(), PreIntegration::None);
    assert_eq!(f.resume(&r).unwrap(), None);
    assert_eq!(image(f.root.path(), &f.repo), before);
    assert_eq!(f.reload(&r), r);
}

// --- Identity (T3-1, Q3-1) ---

impl Fixture {
    fn missing_identity(&self) {
        let mut config = self.repo.config().unwrap();
        config.set_str("user.name", "").unwrap();
        config.set_str("user.email", "").unwrap();
    }
    fn confirmation(&self) -> ConfirmedCommitIdentity {
        let configuration = match prepare_observation(
            &self.service,
            &PreviewTicketClosureRequest {
                root: self.root.path().into(),
                item_id: id(),
                confirmed_identity: None,
            },
        )
        .unwrap()
        .observed
        {
            Err(refusal) => refusal.identity_configuration.expect("identity required"),
            Ok(_) => panic!("expected IdentityRequired"),
        };
        ConfirmedCommitIdentity {
            confirmation_id: OperationId::new(),
            identity: closer(),
            expected_configuration: configuration,
        }
    }
    /// The record for a confirmed identity's digest.
    fn confirmed_record(&self, confirmation: &ConfirmedCommitIdentity) -> ClosureRecord {
        let observation = self.observation_with(Some(confirmation.clone()));
        let context = self.context_oid();
        state::with_transaction(&self.service, self.root.path(), |tx, rid| {
            let mut r = record::begin_record(
                tx,
                rid,
                OperationId::new(),
                &id(),
                observation.expectation,
                None,
                ClosureKind::Record,
            )?;
            r.context_oid = Some(context);
            r.step = ClosureStep::ContextSynchronized;
            record::save_record(tx, rid, &r)?;
            Ok(r)
        })
        .unwrap()
    }
    /// The checkpoint the C9 resume reports for a record whose context ref
    /// has moved.
    fn hook_resumed(&self, r: &ClosureRecord) -> PreIntegration {
        self.resume(r).unwrap().expect("a written checkpoint")
    }
}

#[test]
fn confirmed_identity_persisted_before_checkpoint() {
    let f = Fixture::new(false);
    f.missing_identity();
    f.repo
        .config()
        .unwrap()
        .set_str("unrelated.setting", "kept")
        .unwrap();
    let confirmation = f.confirmation();
    let r = f.confirmed_record(&confirmation);
    let (_, merged, _) = checkpoint_of(f.hook_with(&f.service, &r, Some(&confirmation)).unwrap());
    let config = git2::Repository::open(f.root.path())
        .unwrap()
        .config()
        .unwrap();
    assert_eq!(config.get_string("user.name").unwrap(), "Closer Sentinel");
    assert_eq!(
        config.get_string("user.email").unwrap(),
        "closer-sentinel@example.invalid"
    );
    assert_eq!(config.get_string("unrelated.setting").unwrap(), "kept");
    let commit = f.repo.find_commit(merged).unwrap();
    assert_eq!(
        commit.author().email(),
        Some("closer-sentinel@example.invalid")
    );
    assert_eq!(
        checkpoint_authority(&f.repo, &id(), merged)
            .unwrap()
            .closed_by,
        CLOSER
    );
}

#[test]
fn confirmed_identity_overridden_by_include_writes_no_checkpoint() {
    // Q3-1: a relative include that would override the persisted identity
    // refuses before any install, commit or ref move.
    let f = Fixture::new(false);
    f.missing_identity();
    let config = f.repo.path().join("config");
    let mut source = fs::read(&config).unwrap();
    source.extend_from_slice(b"\n[include]\npath = identity.inc\n");
    fs::write(&config, source).unwrap();
    fs::write(f.repo.path().join("identity.inc"), b"[user]\nemail =\n").unwrap();
    let confirmation = f.confirmation();
    let r = f.confirmed_record(&confirmation);
    let context = f.context_oid();
    let before = image(f.root.path(), &f.repo);
    let config_before = fs::read(&config).unwrap();
    let result = f.hook_with(&f.service, &r, Some(&confirmation));
    // `would_persist_identity` refuses: the integration's recovery category.
    assert!(
        matches!(&result, Err(DocumentPromotionError::Repository(e)) if e.kind == RepositoryErrorKind::RecoveryRequired),
        "{result:?}"
    );
    assert_eq!(image(f.root.path(), &f.repo), before);
    assert_eq!(fs::read(&config).unwrap(), config_before);
    assert!(!f.repo.path().join("config.lock").exists());
    assert_eq!(f.context_oid(), context);
    assert_eq!(f.reload(&r), r);
}

/// Final review M-3 (design C2): a live identity that differs from the
/// consented `closed_by` when the hook runs is `ExternalChange`, and nothing
/// is written.
#[test]
fn changed_identity_at_the_hook_is_external_change() {
    let f = Fixture::new(false);
    let r = f.record(ClosureKind::Record);
    let observation = f.observation();
    f.repo
        .config()
        .unwrap()
        .set_str("user.email", "changed@example.invalid")
        .unwrap();
    let context = f.context_oid();
    let before = image(f.root.path(), &f.repo);
    let result = f.hook_observed(&f.service, &observation, &r, None);
    assert!(
        matches!(result, Err(DocumentPromotionError::ExternalChange)),
        "{result:?}"
    );
    assert_eq!(image(f.root.path(), &f.repo), before);
    assert_eq!(f.context_oid(), context);
    assert_eq!(f.reload(&r), r);
}

/// Final review M-4: a confirmed identity other than the consented
/// `closed_by` is refused before it is persisted to local configuration.
#[test]
fn mismatched_confirmed_identity_persists_nothing() {
    let f = Fixture::new(false);
    f.missing_identity();
    let confirmation = f.confirmation();
    let r = f.confirmed_record(&confirmation);
    let observation = f.observation_with(Some(confirmation.clone()));
    let other = ConfirmedCommitIdentity {
        identity: CommitIdentity {
            name: "Other Closer".into(),
            email: "other-closer@example.invalid".into(),
        },
        ..confirmation
    };
    let config = f.repo.path().join("config");
    let config_before = fs::read(&config).unwrap();
    let before = image(f.root.path(), &f.repo);
    let result = f.hook_observed(&f.service, &observation, &r, Some(&other));
    assert!(
        matches!(result, Err(DocumentPromotionError::ExternalChange)),
        "{result:?}"
    );
    assert_eq!(fs::read(&config).unwrap(), config_before);
    assert!(!f.repo.path().join("config.lock").exists());
    assert_eq!(image(f.root.path(), &f.repo), before);
    assert_eq!(f.reload(&r), r);
}

#[test]
fn adopt_written_closure_only_while_closing() {
    let f = Fixture::new(false);
    let r = f.record(ClosureKind::Record);
    let adopt = |base: Oid, adopted: Option<Oid>| {
        state::with_transaction(&f.service, f.root.path(), |tx, rid| {
            record::adopt_written_closure(tx, rid, r.operation_id, &f.repo, base, adopted)
        })
    };
    assert!(adopt(f.base, None).is_err());
    let (closing, _) = (f.reload(&r), ());
    assert_eq!(closing.step, ClosureStep::ContextSynchronized);
    let failing = RepositoryService::open_at_with_failure_point_for_testing(
        f.data.path(),
        FailurePoint::AfterClosureCheckpoint,
    )
    .unwrap();
    assert!(f.hook_with(&failing, &r, None).is_err());
    let stored = f.reload(&r);
    write(&f.repo, "primary.txt", "primary\n");
    let advanced = commit(&f.repo);
    // Neither the checkpoint itself nor a commit outside it can be adopted.
    assert!(adopt(advanced, stored.closure_oid).is_err());
    assert!(adopt(advanced, Some(advanced)).is_err());
    let descendant = tree_commit(
        &f.repo,
        stored.closure_oid.unwrap(),
        "other.txt",
        b"other\n",
        0o100644,
    );
    adopt(advanced, Some(descendant)).unwrap();
    let adopted = f.reload(&r);
    assert_eq!(adopted.base_oid, Some(advanced));
    assert_eq!(adopted.closure_oid, stored.closure_oid);
    assert_eq!(adopted.adopted_context_oid, Some(descendant));
    assert_eq!(adopted.merged_context(), Some(descendant));
    assert_eq!(adopted.closed_at, stored.closed_at);
    assert_eq!(adopted.step, ClosureStep::Closing);
    // An ordinary save cannot change it.
    let mut sneaked = adopted.clone();
    sneaked.adopted_context_oid = None;
    assert!(
        state::with_transaction(&f.service, f.root.path(), |tx, rid| record::save_record(
            tx, rid, &sneaked
        ))
        .is_err()
    );
}

// --- Fix round 1 ---

impl Fixture {
    /// A remote fixture whose ticket context is also tracked as published.
    fn remote() -> Self {
        let f = Self::new(true);
        f.tracking(&format!("manyhands/ticket/{ID}"), f.context_oid());
        f
    }
    /// The hook stopped at `point`; the stored record.
    fn stopped_at(&self, point: FailurePoint, r: &ClosureRecord) -> ClosureRecord {
        let failing =
            RepositoryService::open_at_with_failure_point_for_testing(self.data.path(), point)
                .unwrap();
        assert!(self.hook_with(&failing, r, None).is_err());
        self.reload(r)
    }
    /// A deliberate synchronization over the checkpoint at the context tip:
    /// a primary advance merged in, the context side first. Returns the
    /// advanced primary and the merge.
    fn synchronize_context(&self, tip: Oid) -> (Oid, Oid) {
        write(&self.repo, "primary.txt", "primary\n");
        let advanced = commit(&self.repo);
        let sig = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
        let local = self.context.find_commit(tip).unwrap();
        let incoming = self.context.find_commit(advanced).unwrap();
        let mut index = self.context.merge_commits(&local, &incoming, None).unwrap();
        let tree = self
            .context
            .find_tree(index.write_tree_to(&self.context).unwrap())
            .unwrap();
        let merge = self
            .context
            .commit(None, &sig, &sig, "synchronize", &tree, &[&local, &incoming])
            .unwrap();
        checkout(&self.context, merge);
        (advanced, merge)
    }
    /// A commit over the tracking primary, as a poll would bring it.
    fn advance_tracking(&self) -> Oid {
        let tracking = self.repo.refname_to_id("refs/remotes/origin/main").unwrap();
        let advanced = tree_commit(&self.repo, tracking, "remote.txt", b"remote\n", 0o100644);
        self.tracking("main", advanced);
        advanced
    }
    /// The record moved to `integrating` as the runner records it.
    fn to_integrating(&self, r: &ClosureRecord) -> ClosureRecord {
        let mut r = r.clone();
        r.integration_kind = Some(crate::repository::promotion::record::IntegrationKind::Merged);
        r.step = ClosureStep::Integrating;
        state::with_transaction(&self.service, self.root.path(), |tx, rid| {
            record::save_record(tx, rid, &r)
        })
        .unwrap();
        self.reload(&r)
    }
    fn refresh_required(&self) -> bool {
        self.db()
            .query_row("SELECT refresh_required FROM repositories", [], |r| {
                r.get(0)
            })
            .unwrap()
    }
    /// The hook over a given (possibly stale) observation and base.
    fn hook_at(
        &self,
        r: &ClosureRecord,
        observation: &ClosureObservation,
        base: Oid,
    ) -> Result<PreIntegration, DocumentPromotionError> {
        let _lease =
            repository_lease(&self.repo, self.root.path(), RepositoryOperation::Read).unwrap();
        let context = self.integration_context(r.context_oid.unwrap());
        let mut stage = LeasedIntegration {
            repository: &self.repo,
            root: self.root.path(),
            context: &context,
            base,
            record: Some(r),
        };
        pre_integration(&self.service, observation, None, &mut stage)
    }
}

#[test]
fn adopted_synchronization_is_integrated_and_the_checkpoint_kept() {
    let f = Fixture::new(false);
    let r = f.record(ClosureKind::Record);
    let stored = f.stopped_at(FailurePoint::AfterClosureCheckpoint, &r);
    let checkpoint = stored.closure_oid.unwrap();
    let (advanced, merge) = f.synchronize_context(checkpoint);
    assert_eq!(f.context_oid(), merge);
    f.db()
        .execute("UPDATE repositories SET refresh_required=0", [])
        .unwrap();
    let first = f.resume(&stored).unwrap().unwrap();
    // The context ref has moved: refresh is required again (idempotent).
    assert!(f.refresh_required());
    let adopted = f.reload(&r);
    assert_eq!(adopted.closure_oid, Some(checkpoint));
    assert_eq!(adopted.adopted_context_oid, Some(merge));
    assert_eq!(adopted.merged_context(), Some(merge));
    assert_eq!(adopted.base_oid, Some(advanced));
    // A second resume after adoption reports the same merge and changes
    // nothing.
    let second = f.resume(&adopted).unwrap().unwrap();
    assert_eq!(second, first);
    assert_eq!(f.reload(&r), adopted);
    let PreIntegration::Checkpoint {
        base,
        merged_context,
        ref signer,
        ..
    } = second
    else {
        panic!("expected a checkpoint")
    };
    assert_eq!((base, merged_context), (advanced, merge));
    // The signer is the checkpoint's author; the merge is no checkpoint.
    let author = f
        .repo
        .find_commit(checkpoint)
        .unwrap()
        .author()
        .email()
        .map(str::to_owned);
    assert_eq!(Some(signer.email.clone()), author);
    assert!(checkpoint_authority(&f.repo, &id(), merge).is_err());
    let integrated = f.integrate(second);
    assert_eq!(
        f.repo
            .find_commit(integrated)
            .unwrap()
            .parent_ids()
            .collect::<Vec<_>>(),
        vec![advanced, merge]
    );
    assert_eq!(f.reload(&r).closure_oid, Some(checkpoint));
}

#[test]
fn integrating_resume_refuses_a_primary_moved_since_the_record() {
    let f = Fixture::new(false);
    let r = f.record(ClosureKind::Record);
    let stored = f.stopped_at(FailurePoint::AfterClosureCheckpoint, &r);
    let integrating = f.to_integrating(&stored);
    // The primary at the recorded base: resumed with the recorded base.
    let (base, merged, _) = checkpoint_of(f.resume(&integrating).unwrap().unwrap());
    assert_eq!(
        (Some(base), Some(merged)),
        (integrating.base_oid, integrating.closure_oid)
    );
    // The primary advances between the runner's lease and this one.
    write(&f.repo, "primary.txt", "primary\n");
    commit(&f.repo);
    let result = f.resume(&integrating);
    assert!(
        matches!(result, Err(DocumentPromotionError::RecoveryRequired)),
        "{result:?}"
    );
    assert_eq!(f.reload(&r), integrating);
}

#[test]
fn remote_primary_advance_from_context_synchronized_is_primary_advanced() {
    let f = Fixture::remote();
    let r = f.record(ClosureKind::Record);
    f.advance_tracking();
    let before = image(f.root.path(), &f.repo);
    let result = f.hook(&r);
    assert!(
        matches!(result, Err(DocumentPromotionError::PrimaryAdvanced)),
        "{result:?}"
    );
    assert_eq!(image(f.root.path(), &f.repo).0, before.0);
    assert_eq!(f.reload(&r), r);
}

#[test]
fn remote_primary_advance_from_closing_is_reconfirmation_required() {
    let f = Fixture::remote();
    let r = f.record(ClosureKind::Record);
    let stored = f.stopped_at(FailurePoint::ClosureAfterClosingRecorded, &r);
    assert_eq!(f.resume(&stored).unwrap(), None);
    f.advance_tracking();
    let result = f.hook(&stored);
    assert!(
        matches!(result, Err(DocumentPromotionError::ReconfirmationRequired)),
        "{result:?}"
    );
    assert_eq!(f.context_oid(), stored.context_oid.unwrap());
    assert_eq!(f.reload(&r), stored);
}

#[test]
fn remote_resume_merges_over_the_tracking_base_in_memory() {
    let f = Fixture::remote();
    let r = f.record(ClosureKind::Record);
    let stored = f.stopped_at(FailurePoint::AfterClosureCheckpoint, &r);
    let tracking = f.advance_tracking();
    let local = f.primary();
    let before = image(f.root.path(), &f.repo).0;
    let (base, merged, tree) = checkpoint_of(f.resume(&stored).unwrap().unwrap());
    assert_eq!(base, tracking);
    assert_eq!(Some(merged), stored.closure_oid);
    // The in-memory merge holds the remote's change and the closure, and no
    // ref moved, the context branch on the remote included.
    assert_eq!(tree_entry(&f.repo, tree, "remote.txt").0, b"remote\n");
    assert!(
        parse_ticket(&tree_entry(&f.repo, tree, &ticket()).0)
            .closed_at
            .is_some()
    );
    let mut after = image(f.root.path(), &f.repo).0;
    assert_eq!(after.remove("refs/remotes/origin/main"), Some(tracking));
    let mut expected = before;
    expected.remove("refs/remotes/origin/main");
    assert_eq!(after, expected);
    assert_eq!(f.primary(), local);
    assert_eq!(f.reload(&r).base_oid, Some(tracking));
}

#[test]
fn remote_resume_with_local_primary_ahead_is_recovery_required() {
    let f = Fixture::remote();
    let r = f.record(ClosureKind::Record);
    let stored = f.stopped_at(FailurePoint::AfterClosureCheckpoint, &r);
    write(&f.repo, "local.txt", "local\n");
    commit(&f.repo);
    let result = f.resume(&stored);
    assert!(
        matches!(result, Err(DocumentPromotionError::RecoveryRequired)),
        "{result:?}"
    );
    assert_eq!(f.reload(&r), stored);
}

#[test]
fn written_effect_with_unreadable_primary_is_recovery_required() {
    let f = Fixture::new(false);
    let missing = "refs/heads/missing";
    let read =
        |written| integrate::written_primary::<DocumentPromotionError>(&f.repo, missing, written);
    // Nothing written: the ordinary path reports the change itself.
    assert!(matches!(read(false), Ok(None)));
    // A written checkpoint is never left to an abandonable refusal.
    assert!(matches!(
        read(true),
        Err(DocumentPromotionError::RecoveryRequired)
    ));
    assert_eq!(
        integrate::written_primary::<DocumentPromotionError>(&f.repo, "refs/heads/main", true)
            .unwrap(),
        Some(f.base)
    );
}

#[test]
fn reconcile_checkout_done_under_line_ending_filters() {
    // The decision reads Git's status and index, never raw bytes: with
    // `core.autocrlf` the checked-out file differs from the Blob bytes.
    let f = Fixture::new(false);
    let (r, prepared) = f.closing(at(1_700_000_000));
    f.repo
        .config()
        .unwrap()
        .set_bool("core.autocrlf", true)
        .unwrap();
    let context = git2::Repository::open(f.context.workdir().unwrap()).unwrap();
    assert!(
        move_context_to_checkpoint_with(
            &context,
            &id(),
            r.context_oid.unwrap(),
            prepared.checkpoint_commit,
            || Err(state::recovery_required()),
        )
        .is_err()
    );
    assert_ne!(
        read(&context, &ticket()),
        f.repo.find_blob(prepared.ticket_blob).unwrap().content()
    );
    assert_eq!(
        reconcile_closing(&context, &r).unwrap(),
        ClosingState::CheckoutDone
    );
}

#[test]
fn closed_base_with_uncontained_context_refuses_before_any_write() {
    let f = Fixture::new(false);
    let r = f.record(ClosureKind::Record);
    let observation = f.observation();
    // Another closure reaches the primary after the observation.
    let closed = closed_ticket_bytes(&read(&f.repo, &ticket()), at(1_700_000_000), CLOSER).unwrap();
    write(&f.repo, &ticket(), closed);
    let base = commit(&f.repo);
    let before = image(f.root.path(), &f.repo);
    let result = f.hook_at(&r, &observation, base);
    assert!(
        matches!(result, Err(DocumentPromotionError::ReconfirmationRequired)),
        "{result:?}"
    );
    assert_eq!(image(f.root.path(), &f.repo).0, before.0);
    assert_eq!(f.reload(&r), r);
}

// --- Fix round 2: an adopted context through `integrating` ---

impl Fixture {
    /// A checkpoint stopped after its ref moved, a deliberate synchronization
    /// over it on the context, and the C9 resume that adopted it while
    /// `closing`. Returns the adopted record, the checkpoint, the advanced
    /// base and the adopted merge.
    fn adopted(&self, r: &ClosureRecord) -> (ClosureRecord, Oid, Oid, Oid) {
        let stored = self.stopped_at(FailurePoint::AfterClosureCheckpoint, r);
        let checkpoint = stored.closure_oid.unwrap();
        let (advanced, merge) = self.synchronize_context(checkpoint);
        self.resume(&stored).unwrap().unwrap();
        let adopted = self.reload(r);
        assert_eq!(adopted.step, ClosureStep::Closing);
        assert_eq!(adopted.adopted_context_oid, Some(merge));
        (adopted, checkpoint, advanced, merge)
    }
    fn adopt(
        &self,
        r: &ClosureRecord,
        base: Oid,
        adopted: Option<Oid>,
    ) -> Result<(), RepositoryError> {
        state::with_transaction(&self.service, self.root.path(), |tx, rid| {
            record::adopt_written_closure(tx, rid, r.operation_id, &self.repo, base, adopted)
        })
    }
}

#[test]
fn integrating_resume_merges_the_adopted_context() {
    let f = Fixture::new(false);
    let r = f.record(ClosureKind::Record);
    let (adopted, checkpoint, advanced, merge) = f.adopted(&r);
    // The runner records `integrating` with the adopted context in place.
    let integrating = f.to_integrating(&adopted);
    assert_eq!(integrating.step, ClosureStep::Integrating);
    assert_eq!(integrating.adopted_context_oid, Some(merge));
    assert_eq!(integrating.merged_context(), Some(merge));
    // The next resume goes through the `integrating` arm: the recorded base
    // and the merged context, never the checkpoint alone.
    let resumed = f.resume(&integrating).unwrap().unwrap();
    let PreIntegration::Checkpoint {
        base,
        merged_context,
        ref signer,
        ..
    } = resumed
    else {
        panic!("expected a checkpoint")
    };
    assert_eq!((base, merged_context), (advanced, merge));
    assert_eq!(signer.email, "fixture@example.invalid");
    let integrated = f.integrate(resumed);
    assert_eq!(
        f.repo
            .find_commit(integrated)
            .unwrap()
            .parent_ids()
            .collect::<Vec<_>>(),
        vec![advanced, merge]
    );
    // The record's crash-reconciliation key is the same pair.
    let stored = f.reload(&r);
    assert_eq!(stored, integrating);
    assert_eq!(stored.closure_oid, Some(checkpoint));
    assert_eq!(
        (stored.base_oid, stored.merged_context()),
        (Some(advanced), Some(merge))
    );
}

#[test]
fn adopt_at_integrating_accepts_only_the_recorded_values() {
    let f = Fixture::new(false);
    let r = f.record(ClosureKind::Record);
    let (adopted, checkpoint, advanced, merge) = f.adopted(&r);
    let integrating = f.to_integrating(&adopted);
    // The recorded values: accepted, nothing written.
    f.adopt(&integrating, advanced, Some(merge)).unwrap();
    assert_eq!(f.reload(&r), integrating);
    // A different tip (even one over the checkpoint), the checkpoint itself,
    // or a different base: refused, and the record is unchanged.
    let other = tree_commit(&f.repo, merge, "other.txt", b"other\n", 0o100644);
    for (base, tip) in [
        (advanced, Some(other)),
        (advanced, None),
        (advanced, Some(checkpoint)),
        (f.base, Some(merge)),
    ] {
        assert!(f.adopt(&integrating, base, tip).is_err(), "{base} {tip:?}");
        assert_eq!(f.reload(&r), integrating);
    }
}

#[test]
fn closing_context_returned_to_the_checkpoint_clears_the_adoption() {
    let f = Fixture::new(false);
    let r = f.record(ClosureKind::Record);
    let (adopted, checkpoint, advanced, _) = f.adopted(&r);
    // The context ref goes back to the checkpoint while still `closing`.
    checkout(&f.context, checkpoint);
    assert_eq!(f.context_oid(), checkpoint);
    let (base, merged, _) = checkpoint_of(f.resume(&adopted).unwrap().unwrap());
    assert_eq!((base, merged), (advanced, checkpoint));
    let cleared = f.reload(&r);
    assert_eq!(cleared.step, ClosureStep::Closing);
    assert_eq!(cleared.adopted_context_oid, None);
    assert_eq!(cleared.closure_oid, Some(checkpoint));
    assert_eq!(cleared.merged_context(), Some(checkpoint));
    assert_eq!(cleared.base_oid, Some(advanced));
}
