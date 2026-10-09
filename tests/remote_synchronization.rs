//! Real authenticated two-clone synchronization acceptance, isolated before threads.
#![allow(clippy::result_large_err)]
#[allow(dead_code, unused_imports)]
#[path = "../src/lib.rs"]
mod production;
pub use production::*;
#[path = "support/ssh_harness.rs"]
mod ssh_harness;
#[path = "support/ssh_privacy.rs"]
mod ssh_privacy;
#[path = "support/ssh_remote.rs"]
mod ssh_remote;

use repository::{keys::*, transport::*, *};
use ssh_remote::*;
use std::{
    cell::Cell,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

fn main() {
    // SAFETY: real main, before any fixture, runner or runtime thread exists.
    if unsafe { ssh_harness::initialize() }.is_err() {
        eprintln!("SSH synchronization initialization failed");
        std::process::exit(1);
    }
    ssh_harness::run_with_output_privacy(CASES, OUTPUT_CONTROLS);
}
const CASES: &[ssh_harness::Case] = &[
    ("ignored_primary_file_collision", || {
        ignored_collision(false, IgnoredCollision::File)
    }),
    ("ignored_context_file_collision", || {
        ignored_collision(true, IgnoredCollision::File)
    }),
    ("ignored_primary_directory_collision", || {
        ignored_collision(false, IgnoredCollision::Directory)
    }),
    ("ignored_context_directory_collision", || {
        ignored_collision(true, IgnoredCollision::Directory)
    }),
    ("ignored_primary_incoming_directory_collision", || {
        ignored_collision(false, IgnoredCollision::IncomingDirectory)
    }),
    ("ignored_context_incoming_directory_collision", || {
        ignored_collision(true, IgnoredCollision::IncomingDirectory)
    }),
    ("ignored_primary_symlink_file_collision", || {
        ignored_collision(false, IgnoredCollision::SymlinkFile)
    }),
    ("ignored_context_symlink_file_collision", || {
        ignored_collision(true, IgnoredCollision::SymlinkFile)
    }),
    ("ignored_primary_symlink_directory_collision", || {
        ignored_collision(false, IgnoredCollision::SymlinkDirectory)
    }),
    ("ignored_context_symlink_directory_collision", || {
        ignored_collision(true, IgnoredCollision::SymlinkDirectory)
    }),
    ("ignored_primary_noncolliding_control", || {
        ignored_noncolliding(false)
    }),
    ("ignored_context_noncolliding_control", || {
        ignored_noncolliding(true)
    }),
    ("fixture_push_waits_for_receiver_receipt", || {
        fixture_push_waits_for_receiver_receipt(false)
    }),
    ("fixture_push_waits_for_duplicate_receiver_receipt", || {
        fixture_push_waits_for_receiver_receipt(true)
    }),
    ("fixture_receiver_controller_timeout_joins_worker", || {
        fixture_receiver_controller_cleanup(false)
    }),
    ("fixture_receiver_controller_unwind_joins_worker", || {
        fixture_receiver_controller_cleanup(true)
    }),
    (
        "fixture_noop_push_needs_no_receiver_update",
        fixture_noop_push,
    ),
    (
        "fixture_rejected_push_is_not_a_success_receipt",
        fixture_rejected_push,
    ),
    (
        "synchronization_raw_capture_privacy",
        synchronization_raw_capture_privacy,
    ),
    (
        "synchronization_probe_union_privacy",
        synchronization_probe_union_privacy,
    ),
    (
        "synchronization_probe_inventory_fail_closed",
        synchronization_probe_inventory_fail_closed,
    ),
    ("hostile_endpoint_redaction", hostile_endpoint_redaction),
    (
        "receiver_expected_old_divergent_race",
        receiver_expected_old_divergent_race,
    ),
    (
        "receiver_command_fragmentation",
        receiver_command_fragmentation,
    ),
    (
        "primary_current_fast_forward_local_ahead",
        primary_current_fast_forward_local_ahead,
    ),
    (
        "context_first_current_fast_forward_local_ahead",
        context_first_current_fast_forward_local_ahead,
    ),
    (
        "receiver_rejection_and_exact_restart",
        receiver_rejection_and_exact_restart,
    ),
    (
        "distinct_push_ancestry_and_tracking",
        distinct_push_ancestry_and_tracking,
    ),
    (
        "primary_dirty_conflicted_preservation",
        primary_dirty_conflicted_preservation,
    ),
    (
        "context_dirty_wrong_detached_preservation",
        context_dirty_wrong_detached_preservation,
    ),
    (
        "primary_divergence_preservation",
        primary_divergence_preservation,
    ),
    (
        "context_remote_divergence_preservation",
        context_remote_divergence_preservation,
    ),
    (
        "context_virtual_primary_divergence_preservation",
        context_virtual_primary_divergence_preservation,
    ),
    (
        "primary_conflict_is_retained_and_inspectable",
        primary_conflict_is_retained_and_inspectable,
    ),
    (
        "binary_primary_conflict_side_read_is_refused",
        binary_primary_conflict_side_read_is_refused,
    ),
    (
        "symlink_primary_conflict_side_read_is_refused",
        symlink_primary_conflict_side_read_is_refused,
    ),
    (
        "mixed_mode_primary_conflict_side_read_is_refused",
        mixed_mode_primary_conflict_side_read_is_refused,
    ),
    (
        "worktree_symlink_is_never_followed_during_conflict_side_read",
        worktree_symlink_is_never_followed_during_conflict_side_read,
    ),
    (
        "context_stage_is_retained_when_primary_conflicts",
        context_stage_is_retained_when_primary_conflicts,
    ),
    ("remote_change_during_fetch", remote_change_during_fetch),
    (
        "verified_context_immediate_deletion",
        verified_context_immediate_deletion,
    ),
    (
        "incompatible_generation_history_unknown",
        incompatible_generation_history_unknown,
    ),
    (
        "post_accept_disconnect_exact_restart",
        post_accept_disconnect_exact_restart,
    ),
    (
        "post_accept_persistence_exact_restart",
        post_accept_persistence_exact_restart,
    ),
    (
        "merge_candidate_race_continues_through_appended_publication",
        merge_candidate_race_continues_through_appended_publication,
    ),
    (
        "merge_candidate_continuation_ambiguous_acceptance_verifies_same_attempt",
        merge_candidate_continuation_ambiguous_acceptance_verifies_same_attempt,
    ),
    ("merge_candidate_publication_stop_at_settlement", || {
        merge_candidate_publication_stop(PublicationStop::Settlement)
    }),
    ("merge_candidate_publication_stop_at_window_append", || {
        merge_candidate_publication_stop(PublicationStop::Window)
    }),
    ("merge_candidate_continuation_cancel_before_push", || {
        merge_candidate_continuation_intervention(false)
    }),
    (
        "merge_candidate_continuation_takeover_after_push_return",
        || merge_candidate_continuation_intervention(true),
    ),
    (
        "merge_candidate_publication_stop_at_stage_observation",
        || merge_candidate_publication_stop(PublicationStop::Stage),
    ),
    ("merge_candidate_publication_stop_at_prepared", || {
        merge_candidate_publication_stop(PublicationStop::Prepared)
    }),
    ("merge_candidate_publication_stop_at_returned", || {
        merge_candidate_publication_stop(PublicationStop::Returned)
    }),
    ("merge_candidate_publication_stop_at_verified", || {
        merge_candidate_publication_stop(PublicationStop::Verified)
    }),
    ("merge_candidate_publication_stop_at_classification", || {
        merge_candidate_publication_stop(PublicationStop::Classification)
    }),
    (
        "merge_candidate_ambiguous_acceptance_exact_restart",
        merge_candidate_ambiguous_acceptance_exact_restart,
    ),
    (
        "merge_candidate_accepted_then_advanced_is_contained_without_push",
        merge_candidate_accepted_then_advanced_is_contained_without_push,
    ),
    (
        "merge_candidate_context_deleted_after_push_is_not_recreated",
        merge_candidate_context_deleted_after_push_is_not_recreated,
    ),
    (
        "merge_candidate_endpoint_change_fences_continuation",
        merge_candidate_endpoint_change_fences_continuation,
    ),
    (
        "merge_candidate_push_only_divergence_is_never_inferred_from_fetch",
        merge_candidate_push_only_divergence_is_never_inferred_from_fetch,
    ),
    (
        "published_index_pending_refresh_only",
        published_index_pending_refresh_only,
    ),
    (
        "already_current_index_pending_refresh_only",
        already_current_index_pending_refresh_only,
    ),
    (
        "completed_refresh_index_flag_no_rescan",
        completed_refresh_index_flag_no_rescan,
    ),
];
const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
const CONTEXT: &str = "refs/heads/manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAW";
const PASSWORD: &str = "sync-acceptance-passphrase-private";
const BODY: &str = "canonical-Markdown-private-sync-sentinel";
const HOSTILE: &str = "server-private!!"; // receive report-status packet width: 16
struct Provider;
impl SessionCredentialProvider for Provider {
    fn request_passphrase(&mut self, _: &UnlockRequest) -> PassphraseResponse {
        PassphraseResponse::Supplied(SecretPassphrase::new(PASSWORD.into()).unwrap())
    }
}
fn callbacks(fixture: &SshRemoteFixture) -> git2::RemoteCallbacks<'_> {
    let mut cb = git2::RemoteCallbacks::new();
    cb.credentials(|_, user, _| {
        git2::Cred::ssh_key(
            user.unwrap_or("fixture"),
            None,
            fixture.client_key_path(),
            Some(PASSWORD),
        )
    });
    let public = fixture.host_public_key().to_bytes().unwrap();
    cb.certificate_check(move |cert, _| {
        if cert.as_hostkey().and_then(|host| host.hostkey()) == Some(public.as_slice()) {
            Ok(git2::CertificateCheckStatus::CertificateOk)
        } else {
            Err(git2::Error::from_str("fixture host mismatch"))
        }
    });
    cb
}
fn clone_owned(fixture: &SshRemoteFixture, root: &Path) -> Result<git2::Repository, FixtureError> {
    let mut fetch = git2::FetchOptions::new();
    fetch.remote_callbacks(callbacks(fixture));
    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fetch);
    fixed(builder.clone(&fixture.url(), root))
}
fn push_peer(
    repo: &git2::Repository,
    fixture: &SshRemoteFixture,
    reference: &str,
) -> Result<(), FixtureError> {
    // Snapshot the receiver and cursor before sending, not after report-status.
    // This helper supports one ordinary ref update, never a forced/multi-ref push.
    let before = fixture.receive_updates().len();
    let receiver = fixed(git2::Repository::open_bare(fixture.repository_path()))?;
    let old = match receiver.refname_to_id(reference) {
        Ok(oid) => oid,
        Err(error) if error.code() == git2::ErrorCode::NotFound => git2::Oid::zero(),
        Err(_) => return Err(FixtureError),
    };
    let new = fixed(repo.refname_to_id(reference))?;
    if old == new {
        // The owned receiver already has this OID. libgit2 can still send an
        // old==new command, whose audit is not an accepted ref update. Avoid
        // creating that late no-op record or waiting for nonexistent success.
        return Ok(());
    }
    let mut remote = fixed(repo.remote_anonymous(&fixture.url()))?;
    let mut options = git2::PushOptions::new();
    options.remote_callbacks(callbacks(fixture));
    fixed(remote.push(&[&format!("{reference}:{reference}")], Some(&mut options)))?;
    fixture.fixture_push_transport_returned()?;
    fixture.wait_for_receive_update(
        before,
        &ReceiveUpdate {
            reference: reference.into(),
            old_oid: old,
            new_oid: new,
            accepted: true,
        },
    )
}
fn commit(
    repo: &git2::Repository,
    parent: git2::Oid,
    name: &str,
    bytes: &[u8],
) -> Result<git2::Oid, FixtureError> {
    commit_with_mode(repo, parent, name, bytes, 0o100644)
}
fn commit_with_mode(
    repo: &git2::Repository,
    parent: git2::Oid,
    name: &str,
    bytes: &[u8],
    mode: i32,
) -> Result<git2::Oid, FixtureError> {
    let parent = fixed(repo.find_commit(parent))?;
    let mut builder = fixed(repo.treebuilder(Some(&fixed(parent.tree())?)))?;
    fixed(builder.insert(name, fixed(repo.blob(bytes))?, mode))?;
    let tree = fixed(repo.find_tree(fixed(builder.write())?))?;
    let sig = fixed(git2::Signature::now("Fixture", "fixture@example.invalid"))?;
    fixed(repo.commit(None, &sig, &sig, "owned fixture child", &tree, &[&parent]))
}
fn advance(
    repo: &git2::Repository,
    reference: &str,
    name: &str,
) -> Result<git2::Oid, FixtureError> {
    let old = fixed(repo.refname_to_id(reference))?;
    let new = commit(repo, old, name, name.as_bytes())?;
    fixed(repo.checkout_tree(
        &fixed(repo.find_object(new, None))?,
        Some(git2::build::CheckoutBuilder::new().safe()),
    ))?;
    fixed(repo.reference(reference, new, true, "owned fixture advance"))?;
    Ok(new)
}
struct World {
    server: SshRemoteFixture,
    service: RepositoryService,
    root: PathBuf,
    peer: git2::Repository,
    directory: tempfile::TempDir,
    probes: Vec<Vec<u8>>,
    paths: Vec<Vec<u8>>,
}
impl World {
    fn new() -> Result<Self, FixtureError> {
        let server = SshRemoteFixture::start()?;
        let plain = fixed(std::fs::read(server.client_key_path()))?;
        let key = fixed(ssh_key::PrivateKey::read_openssh_file(
            server.client_key_path(),
        ))?;
        let encrypted = fixed(
            fixed(key.encrypt(&mut ssh_key::rand_core::OsRng, PASSWORD))?
                .to_openssh(ssh_key::LineEnding::LF),
        )?;
        fixed(std::fs::write(
            server.client_key_path(),
            encrypted.as_bytes(),
        ))?;
        let directory = fixed(tempfile::tempdir())?;
        let root = directory.path().join("clone-a");
        let repo = clone_owned(&server, &root)?;
        fixed(fixed(repo.config())?.set_str("user.name", "Fixture"))?;
        fixed(fixed(repo.config())?.set_str("user.email", "fixture@example.invalid"))?;
        let service = fixed(RepositoryService::open_at(&directory.path().join("data")))?;
        fixed(service.enable(EnableRepositoryRequest {
            root: root.clone(),
            primary_branch: "main".into(),
            identity: None,
            operation_id: OperationId::new(),
        }))?;
        let config = root.join(".manyhands/config.toml");
        let mut source = fixed(std::fs::read_to_string(&config))?;
        source.push_str("publication_remote = \"origin\"\n");
        fixed(std::fs::write(&config, source))?;
        let mut index = fixed(repo.index())?;
        fixed(index.add_path(Path::new(".manyhands/config.toml")))?;
        fixed(index.write())?;
        let tree = fixed(repo.find_tree(fixed(index.write_tree())?))?;
        let parent = fixed(fixed(repo.head())?.peel_to_commit())?;
        let sig = fixed(git2::Signature::now("Fixture", "fixture@example.invalid"))?;
        fixed(repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "owned configuration",
            &tree,
            &[&parent],
        ))?;
        push_peer(&repo, &server, "refs/heads/main")?;
        let peer = clone_owned(&server, &directory.path().join("clone-b"))?;
        let RegisterSharedKeyOutcome::Registered(key) =
            fixed(service.register_shared_key(RegisterSharedKeyRequest {
                label: "acceptance-selected".into(),
                ownership: SharedKeyOwnership::Imported,
                private_key_path: server.client_key_path().into(),
                public_key_path: None,
            }))?
        else {
            return Err(FixtureError);
        };
        fixed(service.select_shared_key(key.id))?;
        let private_fragments = plain
            .split(|byte| *byte == b'\n')
            .filter(|line| line.len() > 40)
            .map(<[u8]>::to_vec)
            .collect::<Vec<_>>();
        let mut probes = vec![
            plain,
            encrypted.as_bytes().to_vec(),
            PASSWORD.as_bytes().to_vec(),
            server.url().into_bytes(),
            HOSTILE.as_bytes().to_vec(),
            BODY.as_bytes().to_vec(),
        ];
        probes.extend(private_fragments);
        ssh_privacy::save(&probes)?;
        let world = Self {
            server,
            service,
            root,
            peer,
            directory,
            probes,
            paths: Vec::new(),
        };
        world.privacy()?;
        Ok(world)
    }
    fn data(&self) -> PathBuf {
        self.directory.path().join("data")
    }
    fn db(&self) -> Result<rusqlite::Connection, FixtureError> {
        fixed(rusqlite::Connection::open(self.data().join(REGISTRY_FILE)))
    }
    fn repo(&self) -> Result<git2::Repository, FixtureError> {
        fixed(git2::Repository::open(&self.root))
    }
    fn bare(&self) -> Result<git2::Repository, FixtureError> {
        fixed(git2::Repository::open_bare(self.server.repository_path()))
    }
    fn request(&self, target: SynchronizationTarget) -> SynchronizeRemoteRequest {
        SynchronizeRemoteRequest {
            root: self.root.clone(),
            operation_id: OperationId::new(),
            target,
            restart: false,
            confirmed_identity: None,
            approval: Some(HostApproval {
                authority: SshAuthority {
                    host: "127.0.0.1".into(),
                    port: self.server.address().port(),
                },
                expected: None,
                presented: self.server.host_identity(),
            }),
        }
    }
    fn sync(
        &self,
        request: SynchronizeRemoteRequest,
    ) -> Result<SynchronizationResult, SynchronizationError> {
        let result = self
            .service
            .synchronize_remote(request, &mut SessionCredentials::new(Provider));
        let formatted = match &result {
            Ok(value) => format!("{value:?}"),
            Err(error) => format!("{error:?} {error}"),
        };
        ssh_privacy::clean(formatted.as_bytes(), &self.probes)
            .expect("redacted synchronization output");
        if !self.paths.is_empty() {
            ssh_privacy::clean(formatted.as_bytes(), &self.paths)
                .expect("redacted target worktree path");
        }
        self.privacy()
            .expect("redacted durable synchronization state");
        result
    }
    fn primary(&self) -> SynchronizeRemoteRequest {
        self.request(SynchronizationTarget::Primary)
    }
    fn context_request(&self) -> SynchronizeRemoteRequest {
        self.request(SynchronizationTarget::Context {
            kind: AuthoringKind::Ticket,
            item_id: ITEM.parse().unwrap(),
        })
    }
    fn context(&mut self) -> Result<ItemContext, FixtureError> {
        let saved = fixed(self.service.save_ticket(SaveTicketRequest {
            target: AuthoringTarget {
                root: self.root.clone(),
                kind: AuthoringKind::Ticket,
                item_id: fixed(ITEM.parse())?,
                intent: ContextIntent::Create,
                operation_id: OperationId::new(),
            },
            draft: TicketDraft {
                title: "Fixture".into(),
                body: BODY.into(),
                ticket_type: "task".into(),
                status: "open".into(),
                project: None,
                team: None,
            },
            expected_path: ExpectedPathObservation::Missing,
        }))?;
        let context = match saved {
            SaveOutcome::Saved { context, .. } | SaveOutcome::IndexPending { context, .. } => {
                context
            }
            _ => return Err(FixtureError),
        };
        let repo = fixed(git2::Repository::open(&context.worktree))?;
        let mut index = fixed(repo.index())?;
        fixed(index.read_tree(&fixed(fixed(repo.head())?.peel_to_tree())?))?;
        fixed(index.write())?;
        // This is forbidden in synchronization rows/errors, but intentional in
        // discovery context/registry path columns and original Git administration.
        self.paths
            .push(context.worktree.to_string_lossy().as_bytes().to_vec());
        self.probes.push(fixed(std::fs::read(
            context
                .worktree
                .join(format!(".manyhands/tickets/{ITEM}/ticket.md")),
        ))?);
        let mut output_probes = self.probes.clone();
        output_probes.extend(self.paths.iter().cloned());
        ssh_privacy::save(&output_probes)?;
        Ok(context)
    }
    fn privacy(&self) -> Result<(), FixtureError> {
        let mut output_probes = self.probes.clone();
        output_probes.extend(self.paths.iter().cloned());
        ssh_privacy::save(&output_probes)?;
        let db = self.db()?;
        let mut scans = 0;
        for table in [
            "remote_operation_records",
            "operation_records",
            "remote_ref_observations",
        ] {
            let mut statement = fixed(db.prepare(&format!("SELECT * FROM {table}")))?;
            let columns = statement.column_count();
            let mut rows = fixed(statement.query([]))?;
            while let Some(row) = fixed(rows.next())? {
                for column in 0..columns {
                    if let rusqlite::types::ValueRef::Text(bytes)
                    | rusqlite::types::ValueRef::Blob(bytes) = fixed(row.get_ref(column))?
                    {
                        ssh_privacy::clean(bytes, &self.probes)?;
                        if !self.paths.is_empty() {
                            ssh_privacy::clean(bytes, &self.paths)?;
                        }
                    }
                }
                scans += 1;
            }
        }
        assert!(scans > 0, "privacy row inventory must be nonempty");
        // Generated whole stores may legitimately retain discovery worktree paths;
        // ALL rows above use the stronger path probe. Registry root/key-source
        // paths and discovery path metadata are not secret or canonical content.
        let inventory = ssh_privacy::scan(&self.data(), &self.probes)?;
        assert!(inventory.iter().any(|(path, bytes)| {
            path.file_name().is_some_and(|name| name == REGISTRY_FILE) && *bytes > 0
        }));
        Ok(())
    }
}
fn outcome(
    result: SynchronizationResult,
    published: bool,
    target: SynchronizationTarget,
    oid: git2::Oid,
) {
    let expected = if published {
        SynchronizationOutcome::Published { target, oid }
    } else {
        SynchronizationOutcome::AlreadyCurrent { target, oid }
    };
    assert_eq!(result, SynchronizationResult::Complete(expected));
}
fn accepted(server: &SshRemoteFixture) -> usize {
    server
        .receive_updates()
        .iter()
        .filter(|update| update.accepted)
        .count()
}
fn one_update(
    server: &SshRemoteFixture,
    before: usize,
    reference: &str,
    old: git2::Oid,
    new: git2::Oid,
) {
    let updates = server.receive_updates();
    assert_eq!(updates.len(), before + 1);
    assert_eq!(
        updates[before],
        ReceiveUpdate {
            reference: reference.into(),
            old_oid: old,
            new_oid: new,
            accepted: true
        }
    );
}
// Fixed-size hashes avoid ever rendering snapshots containing private bytes/paths.
fn physical(root: &Path, worktree: &Path) -> Result<[u8; 32], FixtureError> {
    let repo = fixed(git2::Repository::open(root))?;
    let linked = fixed(git2::Repository::open(worktree))?;
    let mut hash = blake3::Hasher::new();
    let mut refs = fixed(repo.references_glob("refs/heads/*"))?
        .map(|r| {
            let r = fixed(r)?;
            Ok(format!("{} {:?}", r.name().unwrap(), r.target()))
        })
        .collect::<Result<Vec<_>, FixtureError>>()?;
    refs.sort();
    for reference in refs {
        hash.update(reference.as_bytes());
    }
    for name in ["HEAD", "index"] {
        hash.update(&fixed(std::fs::read(linked.path().join(name)))?);
    }
    fn files(path: &Path, hash: &mut blake3::Hasher) -> Result<(), FixtureError> {
        let mut entries = fixed(std::fs::read_dir(path))?
            .map(|e| fixed(e).map(|e| e.path()))
            .collect::<Result<Vec<_>, _>>()?;
        entries.sort();
        for entry in entries {
            if entry
                .file_name()
                .is_some_and(|name| name == ".git" || name == "worktrees")
            {
                continue;
            }
            hash.update(entry.file_name().unwrap().as_encoded_bytes());
            if entry.is_dir() {
                files(&entry, hash)?;
            } else {
                hash.update(&fixed(std::fs::read(entry))?);
            }
        }
        Ok(())
    }
    files(worktree, &mut hash)?;
    let mut options = git2::StatusOptions::new();
    options
        .include_untracked(true)
        .include_ignored(true)
        .recurse_untracked_dirs(true);
    for entry in fixed(linked.statuses(Some(&mut options)))?.iter() {
        hash.update(entry.path().unwrap().as_bytes());
        hash.update(&entry.status().bits().to_le_bytes());
    }
    Ok(*hash.finalize().as_bytes())
}
fn receiver_command_fragmentation() -> Result<(), FixtureError> {
    save_control_probes()?;
    ssh_remote::receiver_command_fragmentation()
}
fn primary_current_fast_forward_local_ahead() -> Result<(), FixtureError> {
    let w = World::new()?;
    let repo = w.repo()?;
    let base = fixed(repo.refname_to_id("refs/heads/main"))?;
    let fh = repo.path().join("FETCH_HEAD");
    fixed(std::fs::write(&fh, b"FETCH_HEAD-preservation-sentinel"))?;
    fixed(repo.reference("refs/remotes/origin/unrelated", base, true, "fixture"))?;
    let n = w.server.receive_updates().len();
    outcome(
        fixed(w.sync(w.primary()))?,
        false,
        SynchronizationTarget::Primary,
        base,
    );
    assert_eq!(w.server.receive_updates().len(), n);
    let advanced = advance(&w.peer, "refs/heads/main", "peer-advance")?;
    push_peer(&w.peer, &w.server, "refs/heads/main")?;
    let n = w.server.receive_updates().len();
    outcome(
        fixed(w.sync(w.primary()))?,
        false,
        SynchronizationTarget::Primary,
        advanced,
    );
    assert_eq!(fixed(repo.refname_to_id("refs/heads/main"))?, advanced);
    assert_eq!(
        fixed(std::fs::read(w.root.join("peer-advance")))?,
        b"peer-advance"
    );
    assert_eq!(
        fixed(fixed(repo.index())?.write_tree())?,
        fixed(repo.find_commit(advanced))?.tree_id()
    );
    assert_eq!(w.server.receive_updates().len(), n);
    let local = advance(&repo, "refs/heads/main", "local-advance")?;
    outcome(
        fixed(w.sync(w.primary()))?,
        true,
        SynchronizationTarget::Primary,
        local,
    );
    one_update(&w.server, n, "refs/heads/main", advanced, local);
    assert_eq!(fixed(w.bare()?.refname_to_id("refs/heads/main"))?, local);
    assert_eq!(
        fixed(std::fs::read(fh))?,
        b"FETCH_HEAD-preservation-sentinel"
    );
    assert_eq!(
        fixed(repo.refname_to_id("refs/remotes/origin/unrelated"))?,
        base
    );
    assert!(w.server.receive_advertisements() > 0);
    Ok(())
}
fn context_first_current_fast_forward_local_ahead() -> Result<(), FixtureError> {
    let mut w = World::new()?;
    let context = w.context()?;
    let repo = fixed(git2::Repository::open(&context.worktree))?;
    let main_before = primary_image(&w.root)?;
    let first = fixed(repo.refname_to_id(CONTEXT))?;
    let n = w.server.receive_updates().len();
    outcome(
        fixed(w.sync(w.context_request()))?,
        true,
        w.context_request().target,
        first,
    );
    one_update(&w.server, n, CONTEXT, git2::Oid::zero(), first);
    let current_updates = w.server.receive_updates();
    outcome(
        fixed(w.sync(w.context_request()))?,
        false,
        w.context_request().target,
        first,
    );
    assert_eq!(w.server.receive_updates(), current_updates);
    let mut remote = fixed(w.peer.find_remote("origin"))?;
    let mut fetch = git2::FetchOptions::new();
    fetch.remote_callbacks(callbacks(&w.server));
    fixed(remote.fetch(
        &[&format!(
            "{CONTEXT}:refs/remotes/origin/manyhands/ticket/{ITEM}"
        )],
        Some(&mut fetch),
        None,
    ))?;
    let next = commit(&w.peer, first, "context-remote", b"context-remote")?;
    fixed(
        w.peer
            .reference(CONTEXT, next, true, "owned authenticated peer context"),
    )?;
    push_peer(&w.peer, &w.server, CONTEXT)?;
    let n = w.server.receive_updates().len();
    outcome(
        fixed(w.sync(w.context_request()))?,
        false,
        w.context_request().target,
        next,
    );
    assert_eq!(fixed(repo.refname_to_id(CONTEXT))?, next);
    assert_eq!(
        fixed(std::fs::read(context.worktree.join("context-remote")))?,
        b"context-remote"
    );
    assert_eq!(
        fixed(fixed(repo.index())?.write_tree())?,
        fixed(repo.find_commit(next))?.tree_id()
    );
    assert_eq!(w.server.receive_updates().len(), n);
    let local = advance(&repo, CONTEXT, "context-local")?;
    outcome(
        fixed(w.sync(w.context_request()))?,
        true,
        w.context_request().target,
        local,
    );
    one_update(&w.server, n, CONTEXT, next, local);
    // Compare primary worktree/index bytes separately: context ref legitimately changes.
    assert_eq!(
        fixed(w.repo()?.refname_to_id("refs/heads/main"))?,
        fixed(w.peer.refname_to_id("refs/heads/main"))?
    );
    assert_eq!(primary_image(&w.root)?, main_before);
    assert_eq!(
        fixed(std::fs::read(w.root.join("fixture.txt")))?,
        b"fixture\n"
    );
    Ok(())
}
fn receiver_rejection_and_exact_restart() -> Result<(), FixtureError> {
    let w = World::new()?;
    let repo = w.repo()?;
    let old = fixed(w.bare()?.refname_to_id("refs/heads/main"))?;
    let new = advance(&repo, "refs/heads/main", "rejected-local")?;
    w.server.reject_primary_updates(true)?;
    w.server.hostile_rejection(HOSTILE);
    let before = physical(&w.root, &w.root)?;
    let n = w.server.receive_updates().len();
    let req = w.primary();
    assert!(matches!(
        w.sync(req.clone()),
        Err(SynchronizationError::PushRejected)
    ));
    assert_eq!(physical(&w.root, &w.root)?, before);
    assert_eq!(fixed(w.bare()?.refname_to_id("refs/heads/main"))?, old);
    let updates = w.server.receive_updates();
    assert_eq!(updates.len(), n + 1);
    assert!(!updates[n].accepted);
    assert_eq!(updates[n].new_oid, new);
    w.server.reject_primary_updates(false)?;
    let mut restart = req;
    restart.restart = true;
    outcome(
        fixed(w.sync(restart))?,
        true,
        SynchronizationTarget::Primary,
        new,
    );
    one_update(&w.server, n + 1, "refs/heads/main", old, new);
    Ok(())
}
fn distinct_push_ancestry_and_tracking() -> Result<(), FixtureError> {
    let mut w = World::new()?;
    let destination = SshRemoteFixture::start()?;
    destination.allow_client_public_key(fixed(russh::keys::PublicKey::from_bytes(
        &w.server.allowed_client_public_key(),
    ))?);
    let repo = w.repo()?;
    let destination_repo = fixed(git2::Repository::open_bare(destination.repository_path()))?;
    let source_odb = fixed(repo.odb())?;
    let target_odb = fixed(destination_repo.odb())?;
    let mut objects = Vec::new();
    fixed(source_odb.foreach(|oid| {
        objects.push(*oid);
        true
    }))?;
    for oid in objects {
        let object = fixed(source_odb.read(oid))?;
        assert_eq!(fixed(target_odb.write(object.kind(), object.data()))?, oid);
    }
    fixed(destination_repo.reference(
        "refs/heads/main",
        w.server.commit_id(),
        true,
        "owned shared ancestry",
    ))?;
    fixed(repo.remote_set_pushurl("origin", Some(&destination.url())))?;
    w.probes.push(destination.url().into_bytes());
    fixed(w.service.verify_ssh_transport(
        VerifySshTransportRequest {
            root: w.root.clone(),
            direction: SshDirection::Fetch,
            approval: w.primary().approval,
        },
        &mut SessionCredentials::new(Provider),
    ))?;
    // Trust separately using the public boundary; the synchronization approval
    // still pertains only to Fetch, never implicitly to this distinct authority.
    fixed(w.service.verify_ssh_transport(
        VerifySshTransportRequest {
            root: w.root.clone(),
            direction: SshDirection::Push,
            approval: Some(HostApproval {
                authority: SshAuthority {
                    host: "127.0.0.1".into(),
                    port: destination.address().port(),
                },
                expected: None,
                presented: destination.host_identity(),
            }),
        },
        &mut SessionCredentials::new(Provider),
    ))?;

    let fetch = fixed(repo.refname_to_id("refs/heads/main"))?;
    let n = destination.receive_updates().len();
    let mut request = w.primary();
    request.approval = None;
    outcome(
        fixed(w.sync(request))?,
        true,
        SynchronizationTarget::Primary,
        fetch,
    );
    one_update(
        &destination,
        n,
        "refs/heads/main",
        w.server.commit_id(),
        fetch,
    );
    assert_eq!(
        fixed(repo.refname_to_id("refs/remotes/origin/main"))?,
        fetch
    );
    assert_eq!(fixed(w.bare()?.refname_to_id("refs/heads/main"))?, fetch);
    // A genuine new commit only at Push is downloaded for ancestry, not assumed
    // equivalent to Fetch or inferred from saved OIDs.
    let push = fixed(git2::Repository::open_bare(destination.repository_path()))?;
    let divergent = commit(&push, fetch, "push-only", b"push-only")?;
    fixed(push.reference("refs/heads/main", divergent, true, "fixture"))?;
    assert!(repo.find_commit(divergent).is_err());
    let local = advance(&repo, "refs/heads/main", "fetch-local")?;
    let before = physical(&w.root, &w.root)?;
    let n = destination.receive_updates().len();
    let mut request = w.primary();
    request.approval = None;
    assert!(matches!(
        w.sync(request),
        Err(SynchronizationError::PushRejected)
    ));
    assert_eq!(physical(&w.root, &w.root)?, before);
    assert!(repo.find_commit(divergent).is_ok());
    assert!(!fixed(repo.graph_descendant_of(local, divergent))?);
    assert!(!fixed(repo.graph_descendant_of(divergent, local))?);
    assert_eq!(destination.receive_updates().len(), n);
    assert_eq!(fixed(push.refname_to_id("refs/heads/main"))?, divergent);
    assert_eq!(
        fixed(repo.refname_to_id("refs/remotes/origin/main"))?,
        fetch
    );
    w.privacy()?;
    Ok(())
}
fn primary_dirty_conflicted_preservation() -> Result<(), FixtureError> {
    for conflicted in [false, true] {
        let w = World::new()?;
        let repo = w.repo()?;
        if conflicted {
            let mut index = fixed(repo.index())?;
            let entry = index
                .get_path(Path::new("fixture.txt"), 0)
                .ok_or(FixtureError)?;
            fixed(index.remove_path(Path::new("fixture.txt")))?;
            for stage in 1..=3 {
                let mut copy = git2::IndexEntry {
                    ctime: entry.ctime,
                    mtime: entry.mtime,
                    dev: entry.dev,
                    ino: entry.ino,
                    mode: entry.mode,
                    uid: entry.uid,
                    gid: entry.gid,
                    file_size: entry.file_size,
                    id: entry.id,
                    flags: (entry.flags & !0x3000) | (stage << 12),
                    flags_extended: entry.flags_extended,
                    path: entry.path.clone(),
                };
                copy.id = fixed(repo.blob(format!("stage-{stage}").as_bytes()))?;
                fixed(index.add(&copy))?;
            }
            fixed(index.write())?;
            assert!(index.has_conflicts());
        } else {
            fixed(std::fs::write(w.root.join("fixture.txt"), b"dirty-local"))?;
        }
        let before = physical(&w.root, &w.root)?;
        let auth = w.server.accepted_keys().len();
        let error = w.sync(w.primary()).unwrap_err();
        assert!(if conflicted {
            matches!(error, SynchronizationError::WorktreeConflicted { .. })
        } else {
            matches!(error, SynchronizationError::WorktreeNotClean { .. })
        });
        assert_eq!(physical(&w.root, &w.root)?, before);
        assert_eq!(w.server.accepted_keys().len(), auth);
    }
    Ok(())
}
fn context_dirty_wrong_detached_preservation() -> Result<(), FixtureError> {
    for mode in 0..3 {
        let mut w = World::new()?;
        let context = w.context()?;
        let linked = fixed(git2::Repository::open(&context.worktree))?;
        let oid = fixed(linked.refname_to_id(CONTEXT))?;
        if mode == 0 {
            fixed(std::fs::write(
                context.worktree.join("fixture.txt"),
                b"dirty-context",
            ))?;
        } else if mode == 1 {
            fixed(linked.reference("refs/heads/wrong-context", oid, true, "fixture"))?;
            fixed(linked.set_head("refs/heads/wrong-context"))?;
        } else {
            fixed(linked.set_head_detached(oid))?;
        }
        let before = physical(&w.root, &context.worktree)?;
        let auth = w.server.accepted_keys().len();
        let error = w.sync(w.context_request()).unwrap_err();
        assert!(if mode == 0 {
            matches!(error, SynchronizationError::WorktreeNotClean { .. })
        } else {
            matches!(error, SynchronizationError::TargetNotMaterialized)
        });
        assert_eq!(physical(&w.root, &context.worktree)?, before);
        assert_eq!(w.server.accepted_keys().len(), auth);
    }
    Ok(())
}
fn primary_divergence_preservation() -> Result<(), FixtureError> {
    let w = World::new()?;
    let repo = w.repo()?;
    let local = advance(&repo, "refs/heads/main", "local-divergent")?;
    let incoming = advance(&w.peer, "refs/heads/main", "remote-divergent")?;
    push_peer(&w.peer, &w.server, "refs/heads/main")?;
    let n = w.server.receive_updates().len();
    let result = fixed(w.sync(w.primary()))?;
    let SynchronizationResult::Complete(SynchronizationOutcome::Published { oid, .. }) = result
    else {
        return Err(FixtureError);
    };
    let merge = fixed(repo.find_commit(oid))?;
    assert_eq!(merge.parent_count(), 2);
    assert_eq!(fixed(merge.parent_id(0))?, local);
    assert_eq!(fixed(merge.parent_id(1))?, incoming);
    assert_eq!(fixed(w.bare()?.refname_to_id("refs/heads/main"))?, oid);
    assert_eq!(w.server.receive_updates().len(), n + 1);
    Ok(())
}
fn context_remote_divergence_preservation() -> Result<(), FixtureError> {
    context_divergence(false)
}
fn context_virtual_primary_divergence_preservation() -> Result<(), FixtureError> {
    context_divergence(true)
}
fn context_divergence(virtual_primary: bool) -> Result<(), FixtureError> {
    let mut w = World::new()?;
    let _context = w.context()?;
    fixed(w.sync(w.context_request()))?;
    let server = w.bare()?;
    let base = fixed(server.refname_to_id("refs/heads/main"))?;
    let local = fixed(server.refname_to_id(CONTEXT))?;
    let remote = commit(
        &server,
        if virtual_primary { local } else { base },
        "remote-context",
        b"remote-context",
    )?;
    fixed(server.reference(CONTEXT, remote, true, "fixture"))?;
    if virtual_primary {
        let primary = commit(&server, base, "primary-divergent", b"primary-divergent")?;
        fixed(server.reference("refs/heads/main", primary, true, "fixture"))?;
    }
    let primary = if virtual_primary {
        Some(fixed(server.refname_to_id("refs/heads/main"))?)
    } else {
        None
    };
    let n = w.server.receive_updates().len();
    let result = fixed(w.sync(w.context_request()))?;
    let SynchronizationResult::Complete(SynchronizationOutcome::Published { oid, .. }) = result
    else {
        return Err(FixtureError);
    };
    let repository = w.repo()?;
    let merge = fixed(repository.find_commit(oid))?;
    assert_eq!(merge.parent_count(), 2);
    assert_eq!(
        fixed(merge.parent_id(0))?,
        if virtual_primary { remote } else { local }
    );
    assert_eq!(fixed(merge.parent_id(1))?, primary.unwrap_or(remote));
    assert_eq!(fixed(w.bare()?.refname_to_id(CONTEXT))?, oid);
    assert_eq!(w.server.receive_updates().len(), n + 1);
    Ok(())
}
fn primary_conflict_is_retained_and_inspectable() -> Result<(), FixtureError> {
    let w = World::new()?;
    let repo = w.repo()?;
    let local_base = fixed(repo.refname_to_id("refs/heads/main"))?;
    let local = commit(&repo, local_base, "conflict", b"local conflict")?;
    fixed(repo.checkout_tree(
        &fixed(repo.find_object(local, None))?,
        Some(git2::build::CheckoutBuilder::new().safe()),
    ))?;
    fixed(repo.reference("refs/heads/main", local, true, "fixture"))?;
    let incoming_base = fixed(w.peer.refname_to_id("refs/heads/main"))?;
    let incoming = commit(&w.peer, incoming_base, "conflict", b"incoming conflict")?;
    fixed(
        w.peer
            .reference("refs/heads/main", incoming, true, "fixture"),
    )?;
    push_peer(&w.peer, &w.server, "refs/heads/main")?;
    let request = w.primary();
    let operation_id = request.operation_id;
    let receives = w.server.receive_updates().len();
    assert!(matches!(
        w.sync(request),
        Err(SynchronizationError::ConflictPending {
            operation_id: actual,
            stage: SynchronizationStage::Primary,
            ..
        }) if actual == operation_id
    ));
    drop(repo);
    let mut repo = w.repo()?;
    assert_eq!(fixed(repo.head())?.target(), Some(local));
    assert!(fixed(repo.index())?.has_conflicts());
    let mut merge_heads = Vec::new();
    fixed(repo.mergehead_foreach(|oid| {
        merge_heads.push(*oid);
        true
    }))?;
    assert_eq!(merge_heads, vec![incoming]);
    assert!(fixed(std::fs::read_to_string(w.root.join("conflict")))?.contains("<<<<<<<"));
    assert_eq!(w.server.receive_updates().len(), receives);

    let inspection = fixed(
        w.service
            .inspect_synchronization_recovery(&w.root, operation_id),
    )?;
    assert_eq!(inspection.local_parent, local);
    assert_eq!(inspection.incoming_parent, incoming);
    assert_eq!(inspection.paths.len(), 1);
    assert!(!format!("{inspection:?}").contains("<<<<<<<"));
    let sides = fixed(
        w.service
            .read_synchronization_conflict(&inspection.paths[0].token),
    )?;
    assert!(sides.local.is_some() && sides.incoming.is_some());
    assert!(!format!("{sides:?}").contains("conflict"));
    fixed(std::fs::write(
        w.root.join(".manyhands/config.toml"),
        "format_version = 1\nprimary_branch = \"main\"\n# stale\n",
    ))?;
    assert!(matches!(
        w.service
            .read_synchronization_conflict(&inspection.paths[0].token),
        Err(SynchronizationError::ExternalChange)
    ));
    Ok(())
}

fn binary_primary_conflict_side_read_is_refused() -> Result<(), FixtureError> {
    let w = World::new()?;
    let repo = w.repo()?;
    let base = fixed(repo.refname_to_id("refs/heads/main"))?;
    let local = commit(&repo, base, "fixture.txt", b"LOCAL_BINARY_CANARY\0")?;
    fixed(repo.checkout_tree(
        &fixed(repo.find_object(local, None))?,
        Some(git2::build::CheckoutBuilder::new().safe()),
    ))?;
    fixed(repo.reference("refs/heads/main", local, true, "fixture"))?;
    let peer_base = fixed(w.peer.refname_to_id("refs/heads/main"))?;
    let incoming = commit(
        &w.peer,
        peer_base,
        "fixture.txt",
        b"INCOMING_BINARY_CANARY\0",
    )?;
    fixed(
        w.peer
            .reference("refs/heads/main", incoming, true, "fixture"),
    )?;
    push_peer(&w.peer, &w.server, "refs/heads/main")?;
    let request = w.primary();
    let operation_id = request.operation_id;
    assert!(matches!(
        w.sync(request),
        Err(SynchronizationError::ConflictPending { .. })
    ));
    let inspection = fixed(
        w.service
            .inspect_synchronization_recovery(&w.root, operation_id),
    )?;
    assert_eq!(inspection.paths.len(), 1);
    assert!(matches!(
        w.service
            .read_synchronization_conflict(&inspection.paths[0].token),
        Err(SynchronizationError::ExternalResolutionRequired { .. })
    ));
    assert!(!format!("{inspection:?}").contains("BINARY_CANARY"));
    Ok(())
}

fn symlink_primary_conflict_side_read_is_refused() -> Result<(), FixtureError> {
    let w = World::new()?;
    let repo = w.repo()?;
    let base = fixed(repo.refname_to_id("refs/heads/main"))?;
    let local = commit_with_mode(&repo, base, "fixture.txt", b"LOCAL_LINK_CANARY", 0o120000)?;
    fixed(repo.checkout_tree(
        &fixed(repo.find_object(local, None))?,
        Some(git2::build::CheckoutBuilder::new().safe()),
    ))?;
    fixed(repo.reference("refs/heads/main", local, true, "fixture"))?;
    let peer_base = fixed(w.peer.refname_to_id("refs/heads/main"))?;
    let incoming = commit_with_mode(
        &w.peer,
        peer_base,
        "fixture.txt",
        b"INCOMING_LINK_CANARY",
        0o120000,
    )?;
    fixed(
        w.peer
            .reference("refs/heads/main", incoming, true, "fixture"),
    )?;
    push_peer(&w.peer, &w.server, "refs/heads/main")?;
    let request = w.primary();
    let operation_id = request.operation_id;
    assert!(matches!(
        w.sync(request),
        Err(SynchronizationError::ConflictPending { .. })
    ));
    let inspection = fixed(
        w.service
            .inspect_synchronization_recovery(&w.root, operation_id),
    )?;
    assert_eq!(inspection.paths.len(), 1);
    assert!(matches!(
        w.service
            .read_synchronization_conflict(&inspection.paths[0].token),
        Err(SynchronizationError::ExternalResolutionRequired { .. })
    ));
    assert!(!format!("{inspection:?}").contains("LINK_CANARY"));
    Ok(())
}

fn mixed_mode_primary_conflict_side_read_is_refused() -> Result<(), FixtureError> {
    let w = World::new()?;
    let repo = w.repo()?;
    let base = fixed(repo.refname_to_id("refs/heads/main"))?;
    let local = commit(&repo, base, "fixture.txt", b"LOCAL_REGULAR_CANARY\n")?;
    fixed(repo.checkout_tree(
        &fixed(repo.find_object(local, None))?,
        Some(git2::build::CheckoutBuilder::new().safe()),
    ))?;
    fixed(repo.reference("refs/heads/main", local, true, "fixture"))?;
    let peer_base = fixed(w.peer.refname_to_id("refs/heads/main"))?;
    let incoming = commit_with_mode(
        &w.peer,
        peer_base,
        "fixture.txt",
        b"INCOMING_LINK_CANARY",
        0o120000,
    )?;
    fixed(
        w.peer
            .reference("refs/heads/main", incoming, true, "fixture"),
    )?;
    push_peer(&w.peer, &w.server, "refs/heads/main")?;
    let request = w.primary();
    let operation_id = request.operation_id;
    assert!(matches!(
        w.sync(request),
        Err(SynchronizationError::ConflictPending { .. })
    ));
    let inspection = fixed(
        w.service
            .inspect_synchronization_recovery(&w.root, operation_id),
    )?;
    assert_eq!(inspection.paths.len(), 1);
    assert!(matches!(
        w.service
            .read_synchronization_conflict(&inspection.paths[0].token),
        Err(SynchronizationError::ExternalResolutionRequired { .. })
    ));
    assert!(!format!("{inspection:?}").contains("REGULAR_CANARY"));
    assert!(!format!("{inspection:?}").contains("LINK_CANARY"));
    Ok(())
}

fn worktree_symlink_is_never_followed_during_conflict_side_read() -> Result<(), FixtureError> {
    let w = World::new()?;
    let repo = w.repo()?;
    let base = fixed(repo.refname_to_id("refs/heads/main"))?;
    let local = commit(&repo, base, "fixture.txt", b"LOCAL_REGULAR_CANARY\n")?;
    fixed(repo.checkout_tree(
        &fixed(repo.find_object(local, None))?,
        Some(git2::build::CheckoutBuilder::new().safe()),
    ))?;
    fixed(repo.reference("refs/heads/main", local, true, "fixture"))?;
    let peer_base = fixed(w.peer.refname_to_id("refs/heads/main"))?;
    let incoming = commit(
        &w.peer,
        peer_base,
        "fixture.txt",
        b"INCOMING_REGULAR_CANARY\n",
    )?;
    fixed(
        w.peer
            .reference("refs/heads/main", incoming, true, "fixture"),
    )?;
    push_peer(&w.peer, &w.server, "refs/heads/main")?;
    let request = w.primary();
    let operation_id = request.operation_id;
    assert!(matches!(
        w.sync(request),
        Err(SynchronizationError::ConflictPending { .. })
    ));
    let inspection = fixed(
        w.service
            .inspect_synchronization_recovery(&w.root, operation_id),
    )?;
    assert_eq!(inspection.paths.len(), 1);
    let sentinel = w.root.join("outside-conflict-canary");
    fixed(std::fs::write(&sentinel, b"EXTERNAL_SYMLINK_CANARY"))?;
    let conflict_path = w.root.join("fixture.txt");
    fixed(std::fs::remove_file(&conflict_path))?;
    owned_symlink(&sentinel, &conflict_path, false)?;
    let sides = fixed(
        w.service
            .read_synchronization_conflict(&inspection.paths[0].token),
    )?;
    assert!(sides.current.is_none());
    assert!(!format!("{sides:?}").contains("EXTERNAL_SYMLINK_CANARY"));
    Ok(())
}

fn context_stage_is_retained_when_primary_conflicts() -> Result<(), FixtureError> {
    let mut w = World::new()?;
    let context = w.context()?;
    fixed(w.sync(w.context_request()))?;
    let server = w.bare()?;
    let base = fixed(server.refname_to_id("refs/heads/main"))?;
    let context_old = fixed(server.refname_to_id(CONTEXT))?;
    let context_next = commit(&server, context_old, "conflict", b"context side")?;
    fixed(server.reference(CONTEXT, context_next, true, "fixture"))?;
    let primary = commit(&server, base, "conflict", b"primary side")?;
    fixed(server.reference("refs/heads/main", primary, true, "fixture"))?;
    let request = w.context_request();
    let operation_id = request.operation_id;
    let receives = w.server.receive_updates().len();
    assert!(matches!(
        w.sync(request),
        Err(SynchronizationError::ConflictPending {
            operation_id: actual,
            stage: SynchronizationStage::Primary,
            ..
        }) if actual == operation_id
    ));
    let repository = w.repo()?;
    assert_eq!(fixed(repository.refname_to_id(CONTEXT))?, context_next);
    let context_repo = fixed(git2::Repository::open(&context.worktree))?;
    assert_eq!(fixed(context_repo.head())?.target(), Some(context_next));
    assert!(fixed(context_repo.index())?.has_conflicts());
    assert!(fixed(std::fs::read_to_string(context.worktree.join("conflict")))?.contains("<<<<<<<"));
    assert_eq!(w.server.receive_updates().len(), receives);
    let inspection = fixed(
        w.service
            .inspect_synchronization_recovery(&w.root, operation_id),
    )?;
    assert_eq!(inspection.stage, SynchronizationStage::Primary);
    assert_eq!(inspection.local_parent, context_next);
    assert_eq!(inspection.incoming_parent, primary);
    Ok(())
}

fn remote_change_during_fetch() -> Result<(), FixtureError> {
    let w = World::new()?;
    let bare_path = w.server.repository_path().to_owned();
    let before = physical(&w.root, &w.root)?;
    let once = Cell::new(false);
    let hook = repository::transport::operation_tests::install_hook(move |point| {
        if point == repository::transport::operation_tests::Checkpoint::TrackingDownloaded
            && !once.replace(true)
        {
            let repo = git2::Repository::open_bare(&bare_path).unwrap();
            let old = repo.refname_to_id("refs/heads/main").unwrap();
            let new = commit(&repo, old, "changed-during-fetch", b"changed-during-fetch").unwrap();
            repo.reference("refs/heads/main", new, true, "fixture")
                .unwrap();
        }
    });
    assert!(matches!(
        w.sync(w.primary()),
        Err(SynchronizationError::ExternalChange)
    ));
    drop(hook);
    assert_eq!(physical(&w.root, &w.root)?, before);
    Ok(())
}
fn verified_context_immediate_deletion() -> Result<(), FixtureError> {
    let mut w = World::new()?;
    let context = w.context()?;
    fixed(w.sync(w.context_request()))?;
    fixed(fixed(w.bare()?.find_reference(CONTEXT))?.delete())?;
    let before = physical(&w.root, &context.worktree)?;
    let n = w.server.receive_updates().len();
    assert!(matches!(
        w.sync(w.context_request()),
        Err(SynchronizationError::RemoteContextDeleted)
    ));
    assert_eq!(physical(&w.root, &context.worktree)?, before);
    assert_eq!(w.server.receive_updates().len(), n);
    assert!(w.bare()?.find_reference(CONTEXT).is_err());
    Ok(())
}
fn incompatible_generation_history_unknown() -> Result<(), FixtureError> {
    let mut w = World::new()?;
    let context = w.context()?;
    fixed(w.sync(w.context_request()))?;
    fixed(fixed(w.bare()?.find_reference(CONTEXT))?.delete())?;
    // A distinct, separately trusted destination creates a genuinely incompatible
    // generation. An explicit pushurl identical to Fetch would NOT change identity.
    let destination = SshRemoteFixture::start()?;
    destination.allow_client_public_key(fixed(russh::keys::PublicKey::from_bytes(
        &w.server.allowed_client_public_key(),
    ))?);
    w.probes.push(destination.url().into_bytes());
    let repo = w.repo()?;
    fixed(repo.remote_set_pushurl("origin", Some(&destination.url())))?;
    fixed(w.service.verify_ssh_transport(
        VerifySshTransportRequest {
            root: w.root.clone(),
            direction: SshDirection::Push,
            approval: Some(HostApproval {
                authority: SshAuthority {
                    host: "127.0.0.1".into(),
                    port: destination.address().port(),
                },
                expected: None,
                presented: destination.host_identity(),
            }),
        },
        &mut SessionCredentials::new(Provider),
    ))?;
    let before = physical(&w.root, &context.worktree)?;
    let n = w.server.receive_updates().len();
    let push_updates = destination.receive_updates();
    let push_accepted = accepted(&destination);
    let push_repo = fixed(git2::Repository::open_bare(destination.repository_path()))?;
    assert!(push_repo.find_reference(CONTEXT).is_err());
    let mut request = w.context_request();
    request.approval = None;
    assert!(matches!(
        w.sync(request),
        Err(SynchronizationError::HistoryUnknown)
    ));
    assert_eq!(physical(&w.root, &context.worktree)?, before);
    assert_eq!(w.server.receive_updates().len(), n);
    assert_eq!(destination.receive_updates(), push_updates);
    assert_eq!(accepted(&destination), push_accepted);
    assert!(push_repo.find_reference(CONTEXT).is_err());
    Ok(())
}
fn post_accept_disconnect_exact_restart() -> Result<(), FixtureError> {
    acceptance_restart(true)
}
fn post_accept_persistence_exact_restart() -> Result<(), FixtureError> {
    acceptance_restart(false)
}
fn acceptance_restart(disconnect: bool) -> Result<(), FixtureError> {
    let w = World::new()?;
    let repo = w.repo()?;
    let old = fixed(w.bare()?.refname_to_id("refs/heads/main"))?;
    let candidate = advance(&repo, "refs/heads/main", "ambiguous-candidate")?;
    let before = physical(&w.root, &w.root)?;
    let req = w.primary();
    let db = w.db()?;
    // Exercise real synchronization writes with live WAL retention on this owned
    // database. Normal service policy is unchanged; other cases use its default.
    fixed(db.execute_batch("PRAGMA journal_mode=WAL;"))?;
    if disconnect {
        w.server.disconnect_at(FixtureBoundary::AfterReceivePack);
    } else {
        fixed(db.execute_batch("CREATE TRIGGER fail_verify BEFORE UPDATE ON remote_operation_records WHEN NEW.sync_checkpoint='push_verified' BEGIN SELECT RAISE(ABORT,'fixed failure'); END"))?;
    }
    let n = w.server.receive_updates().len();
    let accepted_before = accepted(&w.server);
    assert!(w.sync(req.clone()).is_err());
    assert_eq!(
        fixed(w.bare()?.refname_to_id("refs/heads/main"))?,
        candidate
    );
    one_update(&w.server, n, "refs/heads/main", old, candidate);
    assert_eq!(accepted(&w.server), accepted_before + 1);
    if disconnect {
        assert!(w.server.receive_status_withheld());
        w.server.clear_fault();
    } else {
        fixed(db.execute_batch("DROP TRIGGER fail_verify"))?;
    }
    let commits = commit_inventory(&w.bare()?)?;
    let auth = w.server.accepted_keys().len();
    assert!(matches!(
        w.sync(req.clone()),
        Err(SynchronizationError::RecoveryRequired)
    ));
    assert_eq!(w.server.accepted_keys().len(), auth);
    let restarted = fixed(RepositoryService::open_at(&w.data()))?;
    let mut restart = req.clone();
    restart.restart = true;
    let result =
        fixed(restarted.synchronize_remote(restart, &mut SessionCredentials::new(Provider)))?;
    ssh_privacy::clean(format!("{result:?}").as_bytes(), &w.probes)?;
    w.privacy()?;
    outcome(result, true, SynchronizationTarget::Primary, candidate);
    assert_eq!(w.server.receive_updates().len(), n + 1);
    assert_eq!(accepted(&w.server), accepted_before + 1);
    assert_eq!(commit_inventory(&w.bare()?)?, commits);
    assert_eq!(physical(&w.root, &w.root)?, before);
    let auth = w.server.accepted_keys().len();
    outcome(
        fixed(w.sync(req.clone()))?,
        true,
        SynchronizationTarget::Primary,
        candidate,
    );
    assert_eq!(w.server.accepted_keys().len(), auth);
    let count: i64 = fixed(db.query_row(
        "SELECT COUNT(*) FROM operation_records WHERE operation_ulid=?1 AND action='refresh'",
        [req.operation_id.to_string()],
        |r| r.get(0),
    ))?;
    assert_eq!(count, 1);
    // A real generated diagnostic backup and live WAL are scanned, not imaginary
    // artifact names. Original key/config/Git/canonical sources are excluded.
    let backup = w.data().join("synchronization-diagnostic.sqlite");
    fixed(db.execute("VACUUM INTO ?1", [backup.to_string_lossy().as_ref()]))?;
    let diagnostic = fixed(rusqlite::Connection::open(&backup))?;
    fixed(diagnostic.execute_batch("PRAGMA journal_mode=DELETE; BEGIN IMMEDIATE; CREATE TABLE fixture_diagnostic(category TEXT); INSERT INTO fixture_diagnostic VALUES('fixed');"))?;
    let inventory = ssh_privacy::scan(&w.data(), &w.probes)?;
    assert!(
        inventory
            .iter()
            .any(|(path, bytes)| path.to_string_lossy().ends_with("-journal") && *bytes > 0)
    );
    assert!(
        inventory
            .iter()
            .any(|(path, bytes)| path.to_string_lossy().ends_with("-wal") && *bytes > 0)
    );
    w.privacy()?;
    fixed(diagnostic.execute_batch("ROLLBACK"))?;
    Ok(())
}
type AttemptRow = (
    i64,
    String,
    Option<String>,
    String,
    Option<i64>,
    Option<String>,
    String,
);
fn publication_attempts(w: &World) -> Result<Vec<AttemptRow>, FixtureError> {
    let db = w.db()?;
    let mut statement = fixed(db.prepare("SELECT number,previous_oid,previous_advertised_oid,previous_disposition,window_number,candidate_oid,phase FROM remote_publication_attempts ORDER BY number"))?;
    let rows = fixed(statement.query_map([], |row| {
        Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
            row.get(4)?,
            row.get(5)?,
            row.get(6)?,
        ))
    }))?;
    fixed(rows.collect::<Result<Vec<_>, _>>())
}
/// sync_checkpoint, local_oid, push_oid, authoritative_oid.
type LegacyPush = (String, Option<String>, Option<String>, Option<String>);
/// The legacy envelope's checkpoint, local OID and Push intent columns.
fn legacy_push(w: &World, operation: OperationId) -> Result<LegacyPush, FixtureError> {
    fixed(w.db()?.query_row(
        "SELECT sync_checkpoint,local_oid,push_oid,authoritative_oid FROM remote_operation_records WHERE operation_ulid=?1",
        [operation.to_string()],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    ))
}
fn merge_commits(repo: &git2::Repository) -> Result<usize, FixtureError> {
    let mut count = 0;
    for oid in commit_inventory(repo)? {
        if fixed(repo.find_commit(oid))?.parent_count() == 2 {
            count += 1;
        }
    }
    Ok(count)
}
fn parents(repo: &git2::Repository, oid: git2::Oid) -> Result<Vec<git2::Oid>, FixtureError> {
    Ok(fixed(repo.find_commit(oid))?.parent_ids().collect())
}
/// A merge candidate loses the receive race twice. Each deliberate invocation
/// reconciles the newest immutable Push intent first, integrates exactly one
/// new window, makes exactly one push attempt and never regenerates, resets or
/// force-pushes an earlier merge. The legacy intent is never overwritten.
fn merge_candidate_race_continues_through_appended_publication() -> Result<(), FixtureError> {
    let w = World::new()?;
    let repo = w.repo()?;
    let server = w.bare()?;
    let local = advance(&repo, "refs/heads/main", "local-divergent")?;
    let incoming = advance(&w.peer, "refs/heads/main", "remote-divergent")?;
    push_peer(&w.peer, &w.server, "refs/heads/main")?;
    let first_race = commit(&server, incoming, "race-one", b"race-one")?;
    w.server.race_primary_update(incoming, first_race)?;
    let n = w.server.receive_updates().len();
    let req = w.primary();
    assert!(matches!(
        w.sync(req.clone()),
        Err(SynchronizationError::PushRejected)
    ));
    let merged = fixed(repo.refname_to_id("refs/heads/main"))?;
    assert_eq!(parents(&repo, merged)?, [local, incoming]);
    assert_eq!(fixed(server.refname_to_id("refs/heads/main"))?, first_race);
    let updates = w.server.receive_updates();
    assert_eq!(updates.len(), n + 1);
    assert_eq!(
        updates[n],
        ReceiveUpdate {
            reference: "refs/heads/main".into(),
            old_oid: incoming,
            new_oid: merged,
            accepted: false
        }
    );
    let legacy = legacy_push(&w, req.operation_id)?;
    assert_eq!(
        legacy,
        (
            "push_prepared".into(),
            Some(merged.to_string()),
            Some(merged.to_string()),
            None
        )
    );
    assert!(publication_attempts(&w)?.is_empty());
    assert_eq!(merge_commits(&repo)?, 1);
    // An ordinary duplicate is typed Recovery with no network or local effect.
    let before = physical(&w.root, &w.root)?;
    let auth = w.server.accepted_keys().len();
    assert!(matches!(
        w.sync(req.clone()),
        Err(SynchronizationError::RecoveryRequired)
    ));
    assert_eq!(w.server.accepted_keys().len(), auth);
    assert_eq!(physical(&w.root, &w.root)?, before);
    // First deliberate retry: the continuation itself loses another race.
    let mut restart = req.clone();
    restart.restart = true;
    let advertisements = w.server.receive_advertisements();
    let second_race = commit(&server, first_race, "race-two", b"race-two")?;
    w.server.race_primary_update(first_race, second_race)?;
    assert!(matches!(
        w.sync(restart.clone()),
        Err(SynchronizationError::PushRejected)
    ));
    let continued = fixed(repo.refname_to_id("refs/heads/main"))?;
    assert_eq!(parents(&repo, continued)?, [merged, first_race]);
    assert_eq!(merge_commits(&repo)?, 2);
    assert_eq!(fixed(server.refname_to_id("refs/heads/main"))?, second_race);
    let updates = w.server.receive_updates();
    assert_eq!(updates.len(), n + 2);
    assert_eq!(
        updates[n + 1],
        ReceiveUpdate {
            reference: "refs/heads/main".into(),
            old_oid: first_race,
            new_oid: continued,
            accepted: false
        }
    );
    // Bounded: this invocation observed the Push endpoint, but no
    // refetch/merge/push loop chased the racing remote (compared below with
    // the single successful pass, which additionally verifies its push).
    let raced_advertisements = w.server.receive_advertisements() - advertisements;
    assert!(raced_advertisements > 0);
    assert_eq!(legacy_push(&w, req.operation_id)?, legacy);
    assert_eq!(
        publication_attempts(&w)?,
        vec![(
            1,
            merged.to_string(),
            Some(first_race.to_string()),
            "displaced".into(),
            Some(2),
            Some(continued.to_string()),
            "prepared".into()
        )]
    );
    assert!(fixed(repo.statuses(None))?.is_empty());
    // Second deliberate retry: reconcile attempt 1, append attempt 2, publish.
    let advertisements = w.server.receive_advertisements();
    let result = fixed(w.sync(restart.clone()))?;
    assert!(raced_advertisements <= w.server.receive_advertisements() - advertisements);
    let SynchronizationResult::Complete(SynchronizationOutcome::Published { oid, .. }) = result
    else {
        return Err(FixtureError);
    };
    assert_eq!(parents(&repo, oid)?, [continued, second_race]);
    assert_eq!(fixed(repo.refname_to_id("refs/heads/main"))?, oid);
    assert_eq!(fixed(w.bare()?.refname_to_id("refs/heads/main"))?, oid);
    assert_eq!(merge_commits(&repo)?, 3);
    one_update(&w.server, n + 2, "refs/heads/main", second_race, oid);
    assert_eq!(
        legacy_push(&w, req.operation_id)?,
        (
            "discovery_pending".into(),
            Some(merged.to_string()),
            Some(merged.to_string()),
            Some(oid.to_string())
        )
    );
    assert_eq!(
        publication_attempts(&w)?,
        vec![
            (
                1,
                merged.to_string(),
                Some(first_race.to_string()),
                "displaced".into(),
                Some(2),
                Some(continued.to_string()),
                "prepared".into()
            ),
            (
                2,
                continued.to_string(),
                Some(second_race.to_string()),
                "displaced".into(),
                Some(3),
                Some(oid.to_string()),
                "verified".into()
            )
        ]
    );
    let steps: Vec<(i64, String, String)> = {
        let db = w.db()?;
        let mut statement = fixed(db.prepare(
            "SELECT window_number,phase,result_oid FROM remote_integration_steps ORDER BY window_number",
        ))?;
        let rows =
            fixed(statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))))?;
        fixed(rows.collect::<Result<Vec<_>, _>>())?
    };
    assert_eq!(
        steps,
        vec![
            (1, "applied".into(), merged.to_string()),
            (2, "applied".into(), continued.to_string()),
            (3, "applied".into(), oid.to_string())
        ]
    );
    w.privacy()?;
    // Authority replays without transport, integration or another push.
    let auth = w.server.accepted_keys().len();
    outcome(
        fixed(w.sync(req.clone()))?,
        true,
        SynchronizationTarget::Primary,
        oid,
    );
    outcome(
        fixed(w.sync(restart))?,
        true,
        SynchronizationTarget::Primary,
        oid,
    );
    assert_eq!(w.server.accepted_keys().len(), auth);
    assert_eq!(w.server.receive_updates().len(), n + 3);
    assert_eq!(merge_commits(&repo)?, 3);
    Ok(())
}
/// The continuation's own push is accepted but its status never arrives. The
/// next invocation proves it from the Push advertisement and verifies that
/// same attempt: no second attempt row, no second push, no further merge.
fn merge_candidate_continuation_ambiguous_acceptance_verifies_same_attempt()
-> Result<(), FixtureError> {
    let w = World::new()?;
    let repo = w.repo()?;
    let server = w.bare()?;
    let local = advance(&repo, "refs/heads/main", "local-divergent")?;
    let incoming = advance(&w.peer, "refs/heads/main", "remote-divergent")?;
    push_peer(&w.peer, &w.server, "refs/heads/main")?;
    let race = commit(&server, incoming, "race-one", b"race-one")?;
    w.server.race_primary_update(incoming, race)?;
    let n = w.server.receive_updates().len();
    let req = w.primary();
    assert!(matches!(
        w.sync(req.clone()),
        Err(SynchronizationError::PushRejected)
    ));
    let merged = fixed(repo.refname_to_id("refs/heads/main"))?;
    assert_eq!(parents(&repo, merged)?, [local, incoming]);
    let legacy = legacy_push(&w, req.operation_id)?;
    let mut restart = req.clone();
    restart.restart = true;
    w.server.disconnect_at(FixtureBoundary::AfterReceivePack);
    assert!(w.sync(restart.clone()).is_err());
    assert!(w.server.receive_status_withheld());
    w.server.clear_fault();
    let continued = fixed(repo.refname_to_id("refs/heads/main"))?;
    assert_eq!(parents(&repo, continued)?, [merged, race]);
    assert_eq!(
        fixed(w.bare()?.refname_to_id("refs/heads/main"))?,
        continued
    );
    let updates = w.server.receive_updates();
    assert_eq!(updates.len(), n + 2);
    assert_eq!(
        updates[n + 1],
        ReceiveUpdate {
            reference: "refs/heads/main".into(),
            old_oid: race,
            new_oid: continued,
            accepted: true
        }
    );
    let pending = publication_attempts(&w)?;
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].5, Some(continued.to_string()));
    assert_ne!(pending[0].6, "verified");
    assert_eq!(legacy_push(&w, req.operation_id)?, legacy);
    let before = physical(&w.root, &w.root)?;
    let commits = commit_inventory(&w.bare()?)?;
    outcome(
        fixed(w.sync(restart))?,
        true,
        SynchronizationTarget::Primary,
        continued,
    );
    assert_eq!(w.server.receive_updates().len(), n + 2);
    assert_eq!(commit_inventory(&w.bare()?)?, commits);
    assert_eq!(physical(&w.root, &w.root)?, before);
    assert_eq!(merge_commits(&repo)?, 2);
    assert_eq!(
        publication_attempts(&w)?,
        vec![(
            1,
            merged.to_string(),
            Some(race.to_string()),
            "displaced".into(),
            Some(2),
            Some(continued.to_string()),
            "verified".into()
        )]
    );
    assert_eq!(
        legacy_push(&w, req.operation_id)?,
        (
            "discovery_pending".into(),
            Some(merged.to_string()),
            Some(merged.to_string()),
            Some(continued.to_string())
        )
    );
    Ok(())
}
/// Every durable integration and publication evidence row together with the
/// legacy envelope's checkpoint and Push columns, as one fixed-size digest.
fn recorded_evidence(w: &World, operation: OperationId) -> Result<[u8; 32], FixtureError> {
    let db = w.db()?;
    let mut hash = blake3::Hasher::new();
    hash.update(format!("{:?}", legacy_push(w, operation)?).as_bytes());
    for table in [
        "remote_integration_windows",
        "remote_integration_steps",
        "remote_integration_merge_metadata",
        "remote_publication_attempts",
        "remote_observation_batches",
        "remote_ref_observations",
    ] {
        let mut statement = fixed(db.prepare(&format!("SELECT * FROM {table} ORDER BY 1,2")))?;
        let columns = statement.column_count();
        let mut rows = fixed(statement.query([]))?;
        hash.update(table.as_bytes());
        while let Some(row) = fixed(rows.next())? {
            for column in 0..columns {
                hash.update(format!(" {:?}", fixed(row.get_ref(column))?).as_bytes());
            }
            hash.update(b"\n");
        }
    }
    Ok(*hash.finalize().as_bytes())
}
/// HEAD and primary-branch ref logs of the local clone, as one digest.
fn ref_logs(repo: &git2::Repository) -> Result<[u8; 32], FixtureError> {
    let mut hash = blake3::Hasher::new();
    for name in ["logs/HEAD", "logs/refs/heads/main"] {
        hash.update(&fixed(std::fs::read(repo.path().join(name)))?);
        hash.update(&[0]);
    }
    Ok(*hash.finalize().as_bytes())
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum PublicationStop {
    Settlement,
    Window,
    Stage,
    Prepared,
    Returned,
    Verified,
    Classification,
}
/// The continuation of a displaced merge candidate stops at one durable
/// transition of its publication attempt: the settlement that opens it, the
/// append of the continuation's window, the observation of its merge stage,
/// then `prepared`, `returned`, `verified`, and the classification that follows.
/// Deliberate restarts with no network then leave every evidence row, ref, ref
/// log, index and worktree byte exactly as the stop left them. Once the
/// network is back, one restart converges on a single published continuation:
/// one merge of the recorded parents, one accepted push, one attempt row.
fn merge_candidate_publication_stop(stop: PublicationStop) -> Result<(), FixtureError> {
    use PublicationStop as Stop;
    let w = World::new()?;
    let repo = w.repo()?;
    let server = w.bare()?;
    let local = advance(&repo, "refs/heads/main", "local-divergent")?;
    let incoming = advance(&w.peer, "refs/heads/main", "remote-divergent")?;
    push_peer(&w.peer, &w.server, "refs/heads/main")?;
    let race = commit(&server, incoming, "race-one", b"race-one")?;
    w.server.race_primary_update(incoming, race)?;
    let n = w.server.receive_updates().len();
    let req = w.primary();
    assert!(matches!(
        w.sync(req.clone()),
        Err(SynchronizationError::PushRejected)
    ));
    let merged = fixed(repo.refname_to_id("refs/heads/main"))?;
    assert_eq!(parents(&repo, merged)?, [local, incoming]);
    assert_eq!(w.server.receive_updates().len(), n + 1);
    let legacy = legacy_push(&w, req.operation_id)?;
    let mut restart = req.clone();
    restart.restart = true;
    let db = w.db()?;
    fixed(db.execute_batch(match stop {
        Stop::Settlement => "CREATE TRIGGER publication_stop BEFORE INSERT ON remote_publication_attempts BEGIN SELECT RAISE(ABORT,'fixed failure'); END",
        Stop::Window => "CREATE TRIGGER publication_stop BEFORE INSERT ON remote_integration_windows WHEN NEW.number=2 BEGIN SELECT RAISE(ABORT,'fixed failure'); END",
        Stop::Stage => "CREATE TRIGGER publication_stop BEFORE UPDATE ON remote_integration_steps WHEN NEW.phase='applied' AND NEW.window_number=2 BEGIN SELECT RAISE(ABORT,'fixed failure'); END",
        Stop::Prepared => "CREATE TRIGGER publication_stop BEFORE UPDATE ON remote_publication_attempts WHEN NEW.phase='prepared' BEGIN SELECT RAISE(ABORT,'fixed failure'); END",
        Stop::Returned => "CREATE TRIGGER publication_stop BEFORE UPDATE ON remote_publication_attempts WHEN NEW.phase='returned' BEGIN SELECT RAISE(ABORT,'fixed failure'); END",
        Stop::Verified => "CREATE TRIGGER publication_stop BEFORE UPDATE ON remote_publication_attempts WHEN NEW.phase='verified' BEGIN SELECT RAISE(ABORT,'fixed failure'); END",
        Stop::Classification => "CREATE TRIGGER publication_stop BEFORE UPDATE ON remote_operation_records WHEN NEW.sync_checkpoint='discovery_pending' BEGIN SELECT RAISE(ABORT,'fixed failure'); END",
    }))?;
    assert!(matches!(
        w.sync(restart.clone()),
        Err(SynchronizationError::Repository(RepositoryError {
            kind: RepositoryErrorKind::RecoveryRequired,
            ..
        }))
    ));
    fixed(db.execute_batch("DROP TRIGGER publication_stop"))?;
    // Exactly the effects that precede the stopped transition exist.
    let pushed = matches!(stop, Stop::Returned | Stop::Verified | Stop::Classification);
    let stopped_head = fixed(repo.refname_to_id("refs/heads/main"))?;
    let stopped = publication_attempts(&w)?;
    if matches!(stop, Stop::Settlement | Stop::Window) {
        // No continuation merge exists yet; the window stop already settled.
        assert_eq!(stopped.len(), usize::from(stop == Stop::Window));
        if let Some(open) = stopped.first() {
            assert_eq!((open.5.as_deref(), open.6.as_str()), (None, "open"));
        }
        assert_eq!(stopped_head, merged);
        assert_eq!(merge_commits(&repo)?, 1);
    } else {
        assert_eq!(parents(&repo, stopped_head)?, [merged, race]);
        assert_eq!(merge_commits(&repo)?, 2);
        assert_eq!(stopped.len(), 1);
        assert_eq!(
            stopped[0].6,
            match stop {
                Stop::Stage | Stop::Prepared => "open",
                Stop::Returned => "prepared",
                Stop::Verified => "returned",
                _ => "verified",
            }
        );
        assert_eq!(
            stopped[0].5,
            (!matches!(stop, Stop::Stage | Stop::Prepared)).then(|| stopped_head.to_string())
        );
    }
    assert_eq!(
        fixed(w.bare()?.refname_to_id("refs/heads/main"))?,
        if pushed { stopped_head } else { race }
    );
    assert_eq!(
        w.server.receive_updates().len(),
        n + 1 + usize::from(pushed)
    );
    assert_eq!(legacy_push(&w, req.operation_id)?, legacy);
    // No network: each restart is a new process that reaches the same state.
    let mut evidence = recorded_evidence(&w, req.operation_id)?;
    let before = physical(&w.root, &w.root)?;
    let logs = ref_logs(&repo)?;
    let commits = commit_inventory(&w.bare()?)?;
    let local_commits = commit_inventory(&repo)?;
    let updates = w.server.receive_updates();
    w.server.disconnect_at(FixtureBoundary::Handshake);
    for round in 0..3 {
        let offline = fixed(RepositoryService::open_at(&w.data()))?;
        assert!(matches!(
            offline.synchronize_remote(restart.clone(), &mut SessionCredentials::new(Provider)),
            Err(SynchronizationError::Transport(_))
        ));
        let recorded = recorded_evidence(&w, req.operation_id)?;
        if stop == Stop::Stage && round == 0 {
            // Local-first: the first restart records the stage effect that
            // already happened, without the network and without touching Git.
            assert!(recorded != evidence);
            evidence = recorded;
        }
        assert!(recorded == evidence);
        assert!(physical(&w.root, &w.root)? == before);
        assert!(ref_logs(&repo)? == logs);
        assert_eq!(commit_inventory(&repo)?, local_commits);
        assert_eq!(commit_inventory(&w.bare()?)?, commits);
        assert_eq!(w.server.receive_updates(), updates);
    }
    w.server.clear_fault();
    w.privacy()?;
    let online = fixed(RepositoryService::open_at(&w.data()))?;
    let result =
        fixed(online.synchronize_remote(restart.clone(), &mut SessionCredentials::new(Provider)))?;
    ssh_privacy::clean(format!("{result:?}").as_bytes(), &w.probes)?;
    let SynchronizationResult::Complete(SynchronizationOutcome::Published { oid, .. }) = result
    else {
        return Err(FixtureError);
    };
    assert_eq!(parents(&repo, oid)?, [merged, race]);
    if !matches!(stop, Stop::Settlement | Stop::Window) {
        // The stopped continuation is published as it was, never regenerated.
        assert_eq!(oid, stopped_head);
        assert!(ref_logs(&repo)? == logs);
    }
    assert_eq!(fixed(repo.refname_to_id("refs/heads/main"))?, oid);
    assert_eq!(fixed(w.bare()?.refname_to_id("refs/heads/main"))?, oid);
    assert_eq!(merge_commits(&repo)?, 2);
    // One push of the continuation in total, across the stop and the restarts.
    let updates = w.server.receive_updates();
    assert_eq!(updates.len(), n + 2);
    assert_eq!(
        updates[n + 1],
        ReceiveUpdate {
            reference: "refs/heads/main".into(),
            old_oid: race,
            new_oid: oid,
            accepted: true
        }
    );
    assert_eq!(
        publication_attempts(&w)?,
        vec![(
            1,
            merged.to_string(),
            Some(race.to_string()),
            "displaced".into(),
            Some(2),
            Some(oid.to_string()),
            "verified".into()
        )]
    );
    assert_eq!(
        legacy_push(&w, req.operation_id)?,
        (
            "discovery_pending".into(),
            Some(merged.to_string()),
            Some(merged.to_string()),
            Some(oid.to_string())
        )
    );
    let steps: Vec<(i64, String, String)> = {
        let mut statement = fixed(db.prepare(
            "SELECT window_number,phase,result_oid FROM remote_integration_steps ORDER BY window_number",
        ))?;
        let rows =
            fixed(statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))))?;
        fixed(rows.collect::<Result<Vec<_>, _>>())?
    };
    assert_eq!(
        steps,
        vec![
            (1, "applied".into(), merged.to_string()),
            (2, "applied".into(), oid.to_string())
        ]
    );
    w.privacy()?;
    // Authority replays without transport, integration or another push.
    let auth = w.server.accepted_keys().len();
    outcome(
        fixed(w.sync(req.clone()))?,
        true,
        SynchronizationTarget::Primary,
        oid,
    );
    outcome(
        fixed(w.sync(restart))?,
        true,
        SynchronizationTarget::Primary,
        oid,
    );
    assert_eq!(w.server.accepted_keys().len(), auth);
    assert_eq!(w.server.receive_updates().len(), n + 2);
    assert_eq!(merge_commits(&repo)?, 2);
    Ok(())
}
/// A second party intervenes on the attempt route of the public entry point.
/// A cancellation that becomes durable with the attempt's push intent is
/// honoured at the BeforePush boundary: nothing is pushed and, with no
/// conflict pending, the operation is terminal as in Cycle 05. A takeover that
/// becomes durable with the returned push fences the old owner at the
/// AfterPushReturn boundary; the next deliberate restart proves that one push
/// from the Push advertisement and verifies the same attempt. A trigger cannot
/// run a second service, so each intervention is its durable effect.
fn merge_candidate_continuation_intervention(takeover: bool) -> Result<(), FixtureError> {
    let w = World::new()?;
    let repo = w.repo()?;
    let server = w.bare()?;
    let local = advance(&repo, "refs/heads/main", "local-divergent")?;
    let incoming = advance(&w.peer, "refs/heads/main", "remote-divergent")?;
    push_peer(&w.peer, &w.server, "refs/heads/main")?;
    let race = commit(&server, incoming, "race-one", b"race-one")?;
    w.server.race_primary_update(incoming, race)?;
    let n = w.server.receive_updates().len();
    let req = w.primary();
    assert!(matches!(
        w.sync(req.clone()),
        Err(SynchronizationError::PushRejected)
    ));
    let merged = fixed(repo.refname_to_id("refs/heads/main"))?;
    assert_eq!(parents(&repo, merged)?, [local, incoming]);
    let legacy = legacy_push(&w, req.operation_id)?;
    let mut restart = req.clone();
    restart.restart = true;
    let db = w.db()?;
    fixed(db.execute_batch(if takeover {
        "CREATE TRIGGER intervention AFTER UPDATE ON remote_publication_attempts WHEN NEW.phase='returned' BEGIN UPDATE remote_operation_records SET owner_epoch=owner_epoch+1; END"
    } else {
        "CREATE TRIGGER intervention AFTER UPDATE ON remote_publication_attempts WHEN NEW.phase='prepared' BEGIN UPDATE remote_operation_records SET cancel_requested=1; END"
    }))?;
    let stopped = w.sync(restart.clone());
    fixed(db.execute_batch("DROP TRIGGER intervention"))?;
    let continued = fixed(repo.refname_to_id("refs/heads/main"))?;
    assert_eq!(parents(&repo, continued)?, [merged, race]);
    let attempts = publication_attempts(&w)?;
    assert_eq!(attempts.len(), 1);
    assert_eq!(attempts[0].5, Some(continued.to_string()));
    assert_eq!(legacy_push(&w, req.operation_id)?, legacy);
    let before = physical(&w.root, &w.root)?;
    if !takeover {
        assert!(matches!(stopped, Err(SynchronizationError::Interrupted)));
        assert_eq!(attempts[0].6, "prepared");
        // Nothing was pushed, and the same ID only replays the interruption.
        assert_eq!(w.server.receive_updates().len(), n + 1);
        assert_eq!(fixed(w.bare()?.refname_to_id("refs/heads/main"))?, race);
        let auth = w.server.accepted_keys().len();
        for _ in 0..2 {
            assert!(matches!(
                w.sync(restart.clone()),
                Err(SynchronizationError::Interrupted)
            ));
        }
        assert_eq!(w.server.accepted_keys().len(), auth);
        assert_eq!(w.server.receive_updates().len(), n + 1);
        assert_eq!(publication_attempts(&w)?, attempts);
        assert!(physical(&w.root, &w.root)? == before);
        assert_eq!(merge_commits(&repo)?, 2);
        return Ok(());
    }
    assert!(matches!(
        stopped,
        Err(SynchronizationError::Repository(RepositoryError {
            kind: RepositoryErrorKind::RecoveryRequired,
            ..
        }))
    ));
    // The push happened once; the fenced owner recorded nothing after it.
    assert_eq!(attempts[0].6, "returned");
    assert_eq!(
        fixed(w.bare()?.refname_to_id("refs/heads/main"))?,
        continued
    );
    one_update(&w.server, n + 1, "refs/heads/main", race, continued);
    let later = fixed(RepositoryService::open_at(&w.data()))?;
    let result = fixed(later.synchronize_remote(restart, &mut SessionCredentials::new(Provider)))?;
    ssh_privacy::clean(format!("{result:?}").as_bytes(), &w.probes)?;
    w.privacy()?;
    outcome(result, true, SynchronizationTarget::Primary, continued);
    assert_eq!(w.server.receive_updates().len(), n + 2);
    assert_eq!(merge_commits(&repo)?, 2);
    assert!(physical(&w.root, &w.root)? == before);
    let verified = publication_attempts(&w)?;
    assert_eq!(verified.len(), 1);
    assert_eq!(verified[0].6, "verified");
    assert_eq!(verified[0].5, Some(continued.to_string()));
    Ok(())
}
/// Ambiguous acceptance of a MERGE candidate: the receiver took the push but
/// the status never arrived. Restart proves it from the Push advertisement and
/// neither pushes again nor makes another merge or publication attempt.
fn merge_candidate_ambiguous_acceptance_exact_restart() -> Result<(), FixtureError> {
    let w = World::new()?;
    let repo = w.repo()?;
    let local = advance(&repo, "refs/heads/main", "local-divergent")?;
    let incoming = advance(&w.peer, "refs/heads/main", "remote-divergent")?;
    push_peer(&w.peer, &w.server, "refs/heads/main")?;
    let n = w.server.receive_updates().len();
    let req = w.primary();
    w.server.disconnect_at(FixtureBoundary::AfterReceivePack);
    assert!(w.sync(req.clone()).is_err());
    assert!(w.server.receive_status_withheld());
    w.server.clear_fault();
    let merged = fixed(repo.refname_to_id("refs/heads/main"))?;
    assert_eq!(parents(&repo, merged)?, [local, incoming]);
    assert_eq!(fixed(w.bare()?.refname_to_id("refs/heads/main"))?, merged);
    one_update(&w.server, n, "refs/heads/main", incoming, merged);
    let before = physical(&w.root, &w.root)?;
    let commits = commit_inventory(&w.bare()?)?;
    let mut restart = req.clone();
    restart.restart = true;
    outcome(
        fixed(w.sync(restart))?,
        true,
        SynchronizationTarget::Primary,
        merged,
    );
    assert_eq!(w.server.receive_updates().len(), n + 1);
    assert_eq!(commit_inventory(&w.bare()?)?, commits);
    assert_eq!(physical(&w.root, &w.root)?, before);
    assert_eq!(merge_commits(&repo)?, 1);
    assert!(publication_attempts(&w)?.is_empty());
    assert_eq!(
        legacy_push(&w, req.operation_id)?,
        (
            "discovery_pending".into(),
            Some(merged.to_string()),
            Some(merged.to_string()),
            Some(merged.to_string())
        )
    );
    Ok(())
}
/// The ambiguous merge push was accepted and another writer then built on it.
/// The old intent is recorded as contained; the continuation fast-forwards to
/// the advertised descendant and is already current without any push.
fn merge_candidate_accepted_then_advanced_is_contained_without_push() -> Result<(), FixtureError> {
    let w = World::new()?;
    let repo = w.repo()?;
    let local = advance(&repo, "refs/heads/main", "local-divergent")?;
    let incoming = advance(&w.peer, "refs/heads/main", "remote-divergent")?;
    push_peer(&w.peer, &w.server, "refs/heads/main")?;
    let n = w.server.receive_updates().len();
    let req = w.primary();
    w.server.disconnect_at(FixtureBoundary::AfterReceivePack);
    assert!(w.sync(req.clone()).is_err());
    w.server.clear_fault();
    let merged = fixed(repo.refname_to_id("refs/heads/main"))?;
    assert_eq!(parents(&repo, merged)?, [local, incoming]);
    let server = w.bare()?;
    assert_eq!(fixed(server.refname_to_id("refs/heads/main"))?, merged);
    let ahead = commit(&server, merged, "after-accept", b"after-accept")?;
    fixed(server.reference("refs/heads/main", ahead, true, "fixture"))?;
    let legacy = legacy_push(&w, req.operation_id)?;
    let mut restart = req.clone();
    restart.restart = true;
    outcome(
        fixed(w.sync(restart))?,
        false,
        SynchronizationTarget::Primary,
        ahead,
    );
    assert_eq!(fixed(repo.refname_to_id("refs/heads/main"))?, ahead);
    assert!(fixed(repo.statuses(None))?.is_empty());
    assert_eq!(fixed(w.bare()?.refname_to_id("refs/heads/main"))?, ahead);
    assert_eq!(w.server.receive_updates().len(), n + 1);
    assert_eq!(merge_commits(&repo)?, 1);
    assert_eq!(
        publication_attempts(&w)?,
        vec![(
            1,
            merged.to_string(),
            Some(ahead.to_string()),
            "contained".into(),
            Some(2),
            Some(ahead.to_string()),
            "verified".into()
        )]
    );
    let after = legacy_push(&w, req.operation_id)?;
    assert_eq!((&after.1, &after.2), (&legacy.1, &legacy.2));
    assert_eq!(after.3, Some(ahead.to_string()));
    Ok(())
}
/// A context merge candidate was pushed ambiguously and the remote context was
/// then deleted. Publication never recreates it: typed error, merge intact,
/// no push, no appended attempt.
fn merge_candidate_context_deleted_after_push_is_not_recreated() -> Result<(), FixtureError> {
    let mut w = World::new()?;
    let context = w.context()?;
    fixed(w.sync(w.context_request()))?;
    let server = w.bare()?;
    let base = fixed(server.refname_to_id("refs/heads/main"))?;
    let local = fixed(server.refname_to_id(CONTEXT))?;
    let remote = commit(&server, base, "remote-context", b"remote-context")?;
    fixed(server.reference(CONTEXT, remote, true, "fixture"))?;
    let n = w.server.receive_updates().len();
    let req = w.context_request();
    w.server.disconnect_at(FixtureBoundary::AfterReceivePack);
    assert!(w.sync(req.clone()).is_err());
    w.server.clear_fault();
    let repository = w.repo()?;
    let merged = fixed(repository.refname_to_id(CONTEXT))?;
    assert_eq!(parents(&repository, merged)?, [local, remote]);
    one_update(&w.server, n, CONTEXT, remote, merged);
    fixed(fixed(w.bare()?.find_reference(CONTEXT))?.delete())?;
    let before = physical(&w.root, &context.worktree)?;
    let legacy = legacy_push(&w, req.operation_id)?;
    let mut restart = req.clone();
    restart.restart = true;
    for _ in 0..2 {
        assert!(matches!(
            w.sync(restart.clone()),
            Err(SynchronizationError::RemoteContextDeleted)
        ));
        assert_eq!(physical(&w.root, &context.worktree)?, before);
        assert_eq!(w.server.receive_updates().len(), n + 1);
        assert!(w.bare()?.find_reference(CONTEXT).is_err());
        assert!(publication_attempts(&w)?.is_empty());
        assert_eq!(legacy_push(&w, req.operation_id)?, legacy);
        assert_eq!(fixed(repository.refname_to_id(CONTEXT))?, merged);
    }
    Ok(())
}
/// After a merge candidate's Push intent, the publication endpoint changes.
/// The old generation's intent is never reconciled against, or continued on,
/// a different endpoint.
fn merge_candidate_endpoint_change_fences_continuation() -> Result<(), FixtureError> {
    let mut w = World::new()?;
    let repo = w.repo()?;
    let server = w.bare()?;
    // Trust a second destination up front, so that a later switch to it could
    // only be refused by the configuration-generation fence.
    let destination = SshRemoteFixture::start()?;
    destination.allow_client_public_key(fixed(russh::keys::PublicKey::from_bytes(
        &w.server.allowed_client_public_key(),
    ))?);
    w.probes.push(destination.url().into_bytes());
    fixed(repo.remote_set_pushurl("origin", Some(&destination.url())))?;
    fixed(w.service.verify_ssh_transport(
        VerifySshTransportRequest {
            root: w.root.clone(),
            direction: SshDirection::Push,
            approval: Some(HostApproval {
                authority: SshAuthority {
                    host: "127.0.0.1".into(),
                    port: destination.address().port(),
                },
                expected: None,
                presented: destination.host_identity(),
            }),
        },
        &mut SessionCredentials::new(Provider),
    ))?;
    fixed(repo.remote_set_pushurl("origin", None))?;
    let local = advance(&repo, "refs/heads/main", "local-divergent")?;
    let incoming = advance(&w.peer, "refs/heads/main", "remote-divergent")?;
    push_peer(&w.peer, &w.server, "refs/heads/main")?;
    let race = commit(&server, incoming, "race-one", b"race-one")?;
    w.server.race_primary_update(incoming, race)?;
    let n = w.server.receive_updates().len();
    let req = w.primary();
    assert!(matches!(
        w.sync(req.clone()),
        Err(SynchronizationError::PushRejected)
    ));
    let merged = fixed(repo.refname_to_id("refs/heads/main"))?;
    assert_eq!(parents(&repo, merged)?, [local, incoming]);
    fixed(repo.remote_set_pushurl("origin", Some(&destination.url())))?;
    let before = physical(&w.root, &w.root)?;
    let legacy = legacy_push(&w, req.operation_id)?;
    let authenticated = destination.accepted_keys().len();
    let mut restart = req.clone();
    restart.restart = true;
    for _ in 0..2 {
        // The generation fence itself: a fenced owner, never a transport,
        // host-approval or push outcome.
        assert!(matches!(
            w.sync(restart.clone()),
            Err(SynchronizationError::Repository(error))
                if error.kind == RepositoryErrorKind::RecoveryRequired
        ));
        // Refused before any connection: the already trusted destination saw
        // no authentication, so this is the fence and not a transport failure.
        assert_eq!(destination.accepted_keys().len(), authenticated);
        let generations: (i64, i64) = fixed(w.db()?.query_row(
            "SELECT (SELECT configuration_generation FROM remote_operation_records WHERE operation_ulid=?1),(SELECT configuration_generation FROM remote_polling_state)",
            [req.operation_id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ))?;
        assert!(generations.0 < generations.1);
        assert_eq!(physical(&w.root, &w.root)?, before);
        assert_eq!(w.server.receive_updates().len(), n + 1);
        assert!(destination.receive_updates().is_empty());
        assert!(publication_attempts(&w)?.is_empty());
        assert_eq!(legacy_push(&w, req.operation_id)?, legacy);
        assert_eq!(merge_commits(&repo)?, 1);
    }
    Ok(())
}
/// Fetch and Push are distinct endpoints. A merge candidate built from the
/// Fetch tip is refused by a Push endpoint holding push-only history; that
/// history is downloaded for ancestry only and is never merged, and no retry
/// regenerates the merge or infers that the two endpoints agree.
fn merge_candidate_push_only_divergence_is_never_inferred_from_fetch() -> Result<(), FixtureError> {
    let mut w = World::new()?;
    let destination = SshRemoteFixture::start()?;
    destination.allow_client_public_key(fixed(russh::keys::PublicKey::from_bytes(
        &w.server.allowed_client_public_key(),
    ))?);
    let repo = w.repo()?;
    let push = fixed(git2::Repository::open_bare(destination.repository_path()))?;
    let source_odb = fixed(repo.odb())?;
    let target_odb = fixed(push.odb())?;
    let mut objects = Vec::new();
    fixed(source_odb.foreach(|oid| {
        objects.push(*oid);
        true
    }))?;
    for oid in objects {
        let object = fixed(source_odb.read(oid))?;
        assert_eq!(fixed(target_odb.write(object.kind(), object.data()))?, oid);
    }
    let base = w.server.commit_id();
    fixed(push.reference("refs/heads/main", base, true, "owned shared ancestry"))?;
    fixed(repo.remote_set_pushurl("origin", Some(&destination.url())))?;
    w.probes.push(destination.url().into_bytes());
    fixed(w.service.verify_ssh_transport(
        VerifySshTransportRequest {
            root: w.root.clone(),
            direction: SshDirection::Fetch,
            approval: w.primary().approval,
        },
        &mut SessionCredentials::new(Provider),
    ))?;
    fixed(w.service.verify_ssh_transport(
        VerifySshTransportRequest {
            root: w.root.clone(),
            direction: SshDirection::Push,
            approval: Some(HostApproval {
                authority: SshAuthority {
                    host: "127.0.0.1".into(),
                    port: destination.address().port(),
                },
                expected: None,
                presented: destination.host_identity(),
            }),
        },
        &mut SessionCredentials::new(Provider),
    ))?;
    let local = advance(&repo, "refs/heads/main", "local-divergent")?;
    let incoming = advance(&w.peer, "refs/heads/main", "remote-divergent")?;
    push_peer(&w.peer, &w.server, "refs/heads/main")?;
    let push_only = commit(&push, base, "push-only", b"push-only")?;
    fixed(push.reference("refs/heads/main", push_only, true, "fixture"))?;
    assert!(repo.find_commit(push_only).is_err());
    let fetch_updates = w.server.receive_updates().len();
    let mut req = w.primary();
    req.approval = None;
    assert!(matches!(
        w.sync(req.clone()),
        Err(SynchronizationError::PushRejected)
    ));
    let merged = fixed(repo.refname_to_id("refs/heads/main"))?;
    assert_eq!(parents(&repo, merged)?, [local, incoming]);
    assert!(repo.find_commit(push_only).is_ok());
    assert!(!fixed(repo.graph_descendant_of(merged, push_only))?);
    let before = physical(&w.root, &w.root)?;
    let mut restart = req.clone();
    restart.restart = true;
    for _ in 0..2 {
        assert!(matches!(
            w.sync(restart.clone()),
            Err(SynchronizationError::PushRejected)
        ));
        assert_eq!(physical(&w.root, &w.root)?, before);
        assert_eq!(fixed(repo.refname_to_id("refs/heads/main"))?, merged);
        assert_eq!(merge_commits(&repo)?, 1);
        assert!(destination.receive_updates().is_empty());
        assert_eq!(w.server.receive_updates().len(), fetch_updates);
        assert_eq!(fixed(push.refname_to_id("refs/heads/main"))?, push_only);
        assert_eq!(
            fixed(repo.refname_to_id("refs/remotes/origin/main"))?,
            incoming
        );
        assert!(publication_attempts(&w)?.is_empty());
        assert_eq!(
            fixed(
                w.db()?
                    .query_row("SELECT count(*) FROM remote_integration_steps", [], |row| {
                        row.get::<_, i64>(0)
                    })
            )?,
            1
        );
    }
    w.privacy()?;
    Ok(())
}
fn published_index_pending_refresh_only() -> Result<(), FixtureError> {
    index_pending(true, false)
}
fn already_current_index_pending_refresh_only() -> Result<(), FixtureError> {
    index_pending(false, false)
}
fn completed_refresh_index_flag_no_rescan() -> Result<(), FixtureError> {
    index_pending(true, true)
}
fn index_pending(publish: bool, flag_failure: bool) -> Result<(), FixtureError> {
    let mut w = World::new()?;
    let repo = w.repo()?;
    let oid = if publish {
        advance(&repo, "refs/heads/main", "index-candidate")?
    } else {
        fixed(repo.refname_to_id("refs/heads/main"))?
    };
    let expected = if publish {
        SynchronizationOutcome::Published {
            target: SynchronizationTarget::Primary,
            oid,
        }
    } else {
        SynchronizationOutcome::AlreadyCurrent {
            target: SynchronizationTarget::Primary,
            oid,
        }
    };
    let db = w.db()?;
    if flag_failure {
        fixed(db.execute_batch("CREATE TRIGGER fail_handoff BEFORE UPDATE OF index_pending ON remote_operation_records WHEN NEW.index_pending=0 BEGIN SELECT RAISE(ABORT,'fixed failure'); END"))?;
    } else {
        fixed(db.execute_batch("CREATE TRIGGER fail_handoff BEFORE UPDATE ON operation_records WHEN NEW.state='completed' AND NEW.action='refresh' BEGIN SELECT RAISE(ABORT,'fixed failure'); END"))?;
    }
    let scans = Arc::new(AtomicUsize::new(0));
    let observed = scans.clone();
    w.service.set_observation_hook_for_testing(move || {
        observed.fetch_add(1, Ordering::SeqCst);
    });
    let req = w.primary();
    assert_eq!(
        fixed(w.sync(req.clone()))?,
        SynchronizationResult::IndexPending(IndexPending::new(expected.clone()))
    );
    assert_eq!(scans.load(Ordering::SeqCst), 1);
    let before = physical(&w.root, &w.root)?;
    let commands = w.server.commands().len();
    let auth = w.server.accepted_keys().len();
    let updates = w.server.receive_updates();
    let observed = scans.clone();
    w.service.set_observation_hook_for_testing(move || {
        observed.fetch_add(1, Ordering::SeqCst);
    });
    if flag_failure {
        assert_eq!(
            fixed(w.sync(req.clone()))?,
            SynchronizationResult::IndexPending(IndexPending::new(expected.clone()))
        );
        assert_eq!(scans.load(Ordering::SeqCst), 1);
    }
    fixed(db.execute_batch("DROP TRIGGER fail_handoff"))?;
    // Freeze original authority despite new dirty bytes/current endpoint config.
    let hostile_url = "ssh://fixture:raw-secret-url@127.0.0.1:9/fixture.git";
    w.probes.push(hostile_url.as_bytes().to_vec());
    ssh_privacy::save(&w.probes)?;
    fixed(repo.remote_set_url("origin", hostile_url))?;
    outcome(
        fixed(w.sync(req.clone()))?,
        publish,
        SynchronizationTarget::Primary,
        oid,
    );
    assert_eq!(
        scans.load(Ordering::SeqCst),
        if flag_failure { 1 } else { 2 }
    );
    assert_eq!(w.server.accepted_keys().len(), auth);
    assert_eq!(w.server.commands().len(), commands);
    assert_eq!(w.server.receive_updates(), updates);
    assert_eq!(physical(&w.root, &w.root)?, before);
    w.service
        .set_observation_hook_for_testing(|| panic!("completed handoff must not scan"));
    outcome(
        fixed(w.sync(req))?,
        publish,
        SynchronizationTarget::Primary,
        oid,
    );
    assert_eq!(w.server.commands().len(), commands);
    Ok(())
}
fn primary_image(root: &Path) -> Result<[u8; 32], FixtureError> {
    let repo = fixed(git2::Repository::open(root))?;
    let mut hash = blake3::Hasher::new();
    for name in ["HEAD", "index"] {
        hash.update(&fixed(std::fs::read(repo.path().join(name)))?);
    }
    hash.update(fixed(repo.refname_to_id("refs/heads/main"))?.as_bytes());
    for entry in fixed(repo.index())?.iter() {
        hash.update(&entry.path);
        hash.update(&fixed(std::fs::read(
            root.join(fixed(std::str::from_utf8(&entry.path))?),
        ))?);
    }
    Ok(*hash.finalize().as_bytes())
}
fn receiver_expected_old_divergent_race() -> Result<(), FixtureError> {
    let w = World::new()?;
    let repo = w.repo()?;
    let server = w.bare()?;
    let old = fixed(server.refname_to_id("refs/heads/main"))?;
    let candidate = advance(&repo, "refs/heads/main", "race-candidate")?;
    let competing = commit(&server, old, "race-competitor", b"race-competitor")?;
    w.server.race_primary_update(old, competing)?;
    let before = physical(&w.root, &w.root)?;
    let n = w.server.receive_updates().len();
    let req = w.primary();
    assert!(matches!(
        w.sync(req.clone()),
        Err(SynchronizationError::PushRejected)
    ));
    let updates = w.server.receive_updates();
    assert_eq!(updates.len(), n + 1);
    assert_eq!(
        updates[n],
        ReceiveUpdate {
            reference: "refs/heads/main".into(),
            old_oid: old,
            new_oid: candidate,
            accepted: false
        }
    );
    assert_eq!(fixed(server.refname_to_id("refs/heads/main"))?, competing);
    assert!(!fixed(server.graph_descendant_of(candidate, competing))?);
    assert!(!fixed(server.graph_descendant_of(competing, candidate))?);
    assert_eq!(physical(&w.root, &w.root)?, before);
    let (authority, checkpoint, refresh): (Option<String>, String, i64) = fixed(w.db()?.query_row(
        "SELECT authoritative_kind, sync_checkpoint, (SELECT COUNT(*) FROM operation_records WHERE operation_ulid=?1 AND action='refresh') FROM remote_operation_records WHERE operation_ulid=?1",
        [req.operation_id.to_string()], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))))?;
    assert!(authority.is_none());
    assert_eq!(checkpoint, "push_prepared");
    assert_eq!(refresh, 0);
    Ok(())
}
fn commit_inventory(repo: &git2::Repository) -> Result<Vec<git2::Oid>, FixtureError> {
    let odb = fixed(repo.odb())?;
    let mut ids = Vec::new();
    fixed(odb.foreach(|oid| {
        ids.push(*oid);
        true
    }))?;
    let mut commits = Vec::new();
    for id in ids {
        if fixed(odb.read(id))?.kind() == git2::ObjectType::Commit {
            commits.push(id);
        }
    }
    commits.sort();
    assert!(!commits.is_empty());
    Ok(commits)
}
fn hostile_endpoint_redaction() -> Result<(), FixtureError> {
    let mut w = World::new()?;
    let hostile_url = "ssh://fixture:raw-secret-url-private@127.0.0.1:9/fixture.git";
    w.probes.push(hostile_url.as_bytes().to_vec());
    let repo = w.repo()?;
    fixed(repo.remote_set_url("origin", hostile_url))?;
    let before = physical(&w.root, &w.root)?;
    let auth = w.server.accepted_keys().len();
    assert!(w.sync(w.primary()).is_err());
    assert_eq!(w.server.accepted_keys().len(), auth);
    assert_eq!(physical(&w.root, &w.root)?, before);
    Ok(())
}

// Only nested runner captures execute these controlled leaks. Their raw buffers
// must never reach the outer runner or tool output, including on child failure.
const OUTPUT_CONTROLS: &[ssh_harness::Case] = &[
    ("probe_stdout_success", probe_stdout_success),
    ("probe_stderr_success", probe_stderr_success),
    ("probe_stdout_failure", probe_stdout_failure),
    ("probe_stderr_failure", probe_stderr_failure),
    ("probe_clean_failure", probe_clean_failure),
    ("probe_inventory_missing", probe_inventory_missing),
    ("probe_inventory_empty", probe_inventory_empty),
    ("probe_inventory_non_file", probe_inventory_non_file),
    ("probe_inventory_corrupt", probe_inventory_corrupt),
    ("probe_first_world_url", probe_first_world_url),
    ("probe_first_world_key", probe_first_world_key),
];
fn save_control_probes() -> Result<(), FixtureError> {
    ssh_privacy::save(&[b"runner-private-capture-control".to_vec()])
}
fn emit_control(stderr: bool, fail: bool) -> Result<(), FixtureError> {
    use std::io::Write;
    save_control_probes()?;
    let mut bytes = vec![b'x'; 20_478];
    bytes.extend_from_slice(b"runner-private-capture-control");
    if stderr {
        fixed(std::io::stderr().write_all(&bytes))?;
    } else {
        fixed(std::io::stdout().write_all(&bytes))?;
    }
    if fail { Err(FixtureError) } else { Ok(()) }
}
fn probe_stdout_success() -> Result<(), FixtureError> {
    emit_control(false, false)
}
fn probe_stderr_success() -> Result<(), FixtureError> {
    emit_control(true, false)
}
fn probe_stdout_failure() -> Result<(), FixtureError> {
    emit_control(false, true)
}
fn probe_stderr_failure() -> Result<(), FixtureError> {
    emit_control(true, true)
}
fn probe_clean_failure() -> Result<(), FixtureError> {
    save_control_probes()?;
    Err(FixtureError)
}
fn probe_inventory_missing() -> Result<(), FixtureError> {
    Ok(())
}
fn probe_inventory_empty() -> Result<(), FixtureError> {
    let root = PathBuf::from(std::env::var_os(ssh_privacy::PROBES).ok_or(FixtureError)?);
    fixed(std::fs::write(
        root.join(blake3::hash(b"").to_hex().as_str()),
        b"",
    ))
}
fn probe_inventory_non_file() -> Result<(), FixtureError> {
    save_control_probes()?;
    let root = PathBuf::from(std::env::var_os(ssh_privacy::PROBES).ok_or(FixtureError)?);
    fixed(std::fs::create_dir(root.join("not-a-probe-file")))
}
fn probe_inventory_corrupt() -> Result<(), FixtureError> {
    save_control_probes()?;
    let root = PathBuf::from(std::env::var_os(ssh_privacy::PROBES).ok_or(FixtureError)?);
    let bytes = b"runner-private-capture-control";
    fixed(std::fs::write(
        root.join(blake3::hash(bytes).to_hex().as_str()),
        b"corrupt probe",
    ))?;
    assert!(save_control_probes().is_err());
    Ok(())
}
fn synchronization_raw_capture_privacy() -> Result<(), FixtureError> {
    save_control_probes()?;
    for case in [
        "probe_stdout_success",
        "probe_stderr_success",
        "probe_stdout_failure",
        "probe_stderr_failure",
    ] {
        assert_eq!(
            ssh_harness::run_isolated_with_output_privacy(case),
            Err(ssh_harness::IsolationFailure::OutputPrivacy)
        );
    }
    assert_eq!(
        ssh_harness::run_isolated_with_output_privacy("probe_clean_failure"),
        Err(ssh_harness::IsolationFailure::Child)
    );
    Ok(())
}
fn synchronization_probe_inventory_fail_closed() -> Result<(), FixtureError> {
    save_control_probes()?;
    for case in [
        "probe_inventory_missing",
        "probe_inventory_empty",
        "probe_inventory_non_file",
        "probe_inventory_corrupt",
    ] {
        assert_eq!(
            ssh_harness::run_isolated_with_output_privacy(case),
            Err(ssh_harness::IsolationFailure::ProbeInventory)
        );
    }
    assert!(ssh_privacy::save(&[]).is_err());
    assert!(ssh_privacy::save(&[Vec::new()]).is_err());
    Ok(())
}
fn first_world_output(key: bool) -> Result<(), FixtureError> {
    use std::io::Write;
    let first = World::new()?;
    let private_key = first.probes[0].clone();
    let url = first.server.url();
    let second = World::new()?;
    let third = World::new()?;
    assert!(url != second.server.url() && url != third.server.url());
    assert!(private_key != second.probes[0] && private_key != third.probes[0]);
    // Repeat saves too: the union must survive Worlds AND individual snapshots.
    second.privacy()?;
    third.privacy()?;
    let root = PathBuf::from(std::env::var_os(ssh_privacy::PROBES).ok_or(FixtureError)?);
    let inventory = ssh_privacy::load(&root)?;
    assert!(inventory.iter().any(|probe| probe == &private_key));
    assert!(inventory.iter().any(|probe| probe == url.as_bytes()));
    if key {
        fixed(std::io::stderr().write_all(&private_key))?;
    } else {
        fixed(std::io::stdout().write_all(url.as_bytes()))?;
    }
    Ok(())
}
fn probe_first_world_url() -> Result<(), FixtureError> {
    first_world_output(false)
}
fn probe_first_world_key() -> Result<(), FixtureError> {
    first_world_output(true)
}
fn synchronization_probe_union_privacy() -> Result<(), FixtureError> {
    save_control_probes()?;
    for case in ["probe_first_world_url", "probe_first_world_key"] {
        assert_eq!(
            ssh_harness::run_isolated_with_output_privacy(case),
            Err(ssh_harness::IsolationFailure::OutputPrivacy)
        );
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum IgnoredCollision {
    File,
    Directory,
    IncomingDirectory,
    SymlinkFile,
    SymlinkDirectory,
}
fn persistent_ignore(repo: &git2::Repository) -> Result<(), FixtureError> {
    let exclude = repo.commondir().join("info/exclude");
    fixed(std::fs::create_dir_all(exclude.parent().unwrap()))?;
    let mut bytes = std::fs::read(&exclude).unwrap_or_default();
    bytes.extend_from_slice(b"\n/safety-ignored\n/safety-control\n");
    fixed(std::fs::write(exclude, bytes))
}
fn owned_symlink(target: &Path, link: &Path, directory: bool) -> Result<(), FixtureError> {
    // Capability failures fail the case; no silent native skip or passed proof.
    #[cfg(unix)]
    {
        let _ = directory;
        fixed(std::os::unix::fs::symlink(target, link))
    }
    #[cfg(windows)]
    {
        if directory {
            fixed(std::os::windows::fs::symlink_dir(target, link))
        } else {
            fixed(std::os::windows::fs::symlink_file(target, link))
        }
    }
}
fn collision_target(
    w: &mut World,
    context: bool,
) -> Result<(git2::Repository, SynchronizeRemoteRequest, &'static str), FixtureError> {
    if !context {
        return Ok((w.repo()?, w.primary(), "refs/heads/main"));
    }
    let c = w.context()?;
    fixed(w.sync(w.context_request()))?;
    let mut remote = fixed(w.peer.find_remote("origin"))?;
    let mut fetch = git2::FetchOptions::new();
    fetch.remote_callbacks(callbacks(&w.server));
    fixed(remote.fetch(
        &[&format!(
            "{CONTEXT}:refs/remotes/origin/manyhands/ticket/{ITEM}"
        )],
        Some(&mut fetch),
        None,
    ))?;
    Ok((
        fixed(git2::Repository::open(&c.worktree))?,
        w.context_request(),
        CONTEXT,
    ))
}
fn incoming_owned_paths(
    w: &World,
    parent: git2::Oid,
    reference: &str,
    directory: bool,
) -> Result<git2::Oid, FixtureError> {
    let mut index = fixed(w.peer.index())?;
    fixed(index.read_tree(&fixed(fixed(w.peer.find_commit(parent))?.tree())?))?;
    let workdir = w.peer.workdir().unwrap();
    let name = if directory {
        "safety-ignored/child"
    } else {
        "safety-ignored"
    };
    if directory {
        fixed(std::fs::create_dir_all(workdir.join("safety-ignored")))?;
    }
    fixed(std::fs::write(workdir.join(name), b"incoming tracked data"))?;
    // Explicit owned index paths, not a product force checkout/push. Include
    // another tracked change to observe any partial checkout rather than assume.
    fixed(index.add_path(Path::new(name)))?;
    fixed(std::fs::write(
        workdir.join("fixture.txt"),
        b"incoming tracked change",
    ))?;
    fixed(index.add_path(Path::new("fixture.txt")))?;
    let tree = fixed(w.peer.find_tree(fixed(index.write_tree())?))?;
    let parent = fixed(w.peer.find_commit(parent))?;
    let sig = fixed(git2::Signature::now("Fixture", "fixture@example.invalid"))?;
    let next = fixed(w.peer.commit(
        None,
        &sig,
        &sig,
        "owned incoming collision",
        &tree,
        &[&parent],
    ))?;
    fixed(
        w.peer
            .reference(reference, next, true, "owned collision descendant"),
    )?;
    push_peer(&w.peer, &w.server, reference)?;
    Ok(next)
}
fn ignored_collision(context: bool, kind: IgnoredCollision) -> Result<(), FixtureError> {
    let mut w = World::new()?;
    let (repo, req, reference) = collision_target(&mut w, context)?;
    let workdir = repo.workdir().unwrap();
    persistent_ignore(&repo)?;
    let ignored = workdir.join("safety-ignored");
    let target = w.directory.path().join("owned-symlink-target");
    let directory = matches!(
        kind,
        IgnoredCollision::Directory | IgnoredCollision::SymlinkDirectory
    );
    if directory {
        let p = if matches!(kind, IgnoredCollision::Directory) {
            &ignored
        } else {
            &target
        };
        fixed(std::fs::create_dir(p))?;
        fixed(std::fs::write(p.join("private-child"), BODY.as_bytes()))?;
    } else {
        let p = if matches!(kind, IgnoredCollision::SymlinkFile) {
            &target
        } else {
            &ignored
        };
        fixed(std::fs::write(p, BODY.as_bytes()))?;
    }
    let symlink = matches!(
        kind,
        IgnoredCollision::SymlinkFile | IgnoredCollision::SymlinkDirectory
    );
    if symlink {
        owned_symlink(&target, &ignored, directory)?;
    }
    let old = fixed(repo.refname_to_id(reference))?;
    let incoming = incoming_owned_paths(
        &w,
        old,
        reference,
        matches!(kind, IgnoredCollision::IncomingDirectory),
    )?;
    let mut options = git2::StatusOptions::new();
    options.include_ignored(true);
    assert!(
        fixed(repo.statuses(Some(&mut options)))?
            .iter()
            .any(|e| e.status().contains(git2::Status::IGNORED))
    );
    let before = physical(&w.root, workdir)?;
    let updates = w.server.receive_updates();
    let req_id = req.operation_id;
    // Must reach durable LocalPrepared, not blanket-reject all ignored artifacts.
    let result = w.sync(req);
    if result.is_ok() {
        let private = if directory {
            ignored.join("private-child")
        } else {
            ignored.clone()
        };
        assert!(
            std::fs::read(private).ok().as_deref() == Some(BODY.as_bytes()),
            "ignored user content discarded after successful integration"
        );
    }
    assert!(
        matches!(result, Err(SynchronizationError::RecoveryRequired)),
        "ignored collision must reject integration"
    );
    assert_eq!(physical(&w.root, workdir)?, before);
    assert_eq!(fixed(repo.refname_to_id(reference))?, old);
    assert_eq!(w.server.receive_updates(), updates);
    assert_eq!(fixed(w.bare()?.refname_to_id(reference))?, incoming);
    let checkpoint:(String,Option<String>) = fixed(w.db()?.query_row("SELECT sync_checkpoint,authoritative_kind FROM remote_operation_records WHERE operation_ulid=?1",[req_id.to_string()],|r|Ok((r.get(0)?,r.get(1)?))))?;
    assert_eq!(checkpoint, ("local_prepared".into(), None));
    if symlink {
        assert!(
            fixed(std::fs::symlink_metadata(&ignored))?
                .file_type()
                .is_symlink()
        );
        assert!(
            fixed(std::fs::read_link(&ignored))? == target,
            "ignored symlink identity preserved"
        );
    }
    let private = if directory {
        ignored.join("private-child")
    } else {
        ignored.clone()
    };
    assert!(
        fixed(std::fs::read(private))? == BODY.as_bytes(),
        "ignored private bytes preserved"
    );
    if symlink {
        let private = if directory {
            target.join("private-child")
        } else {
            target
        };
        assert!(
            fixed(std::fs::read(private))? == BODY.as_bytes(),
            "symlink target data preserved"
        );
    }
    Ok(())
}
fn ignored_noncolliding(context: bool) -> Result<(), FixtureError> {
    let mut w = World::new()?;
    let (repo, req, reference) = collision_target(&mut w, context)?;
    let workdir = repo.workdir().unwrap();
    persistent_ignore(&repo)?;
    fixed(std::fs::write(
        workdir.join("safety-control"),
        BODY.as_bytes(),
    ))?;
    let old = fixed(repo.refname_to_id(reference))?;
    let next = incoming_owned_paths(&w, old, reference, false)?;
    let updates = w.server.receive_updates();
    outcome(fixed(w.sync(req.clone()))?, false, req.target, next);
    assert_eq!(fixed(repo.refname_to_id(reference))?, next);
    assert_eq!(
        fixed(fixed(repo.index())?.write_tree())?,
        fixed(repo.find_commit(next))?.tree_id()
    );
    assert!(fixed(std::fs::read(workdir.join("safety-control")))? == BODY.as_bytes());
    assert_eq!(w.server.receive_updates(), updates);
    Ok(())
}

fn fixture_push_waits_for_receiver_receipt(duplicate: bool) -> Result<(), FixtureError> {
    let w = World::new()?;
    let repo = w.repo()?;
    let reference = "refs/heads/main";
    persistent_ignore(&repo)?;
    fixed(std::fs::write(
        w.root.join("safety-control"),
        BODY.as_bytes(),
    ))?;
    let before = w.server.receive_updates();
    let old = if duplicate {
        // Re-send the same old/new pair as the setup push. Its earlier receipt
        // must not satisfy the new push's wait.
        let old = w.server.commit_id();
        fixed(
            w.bare()?
                .reference(reference, old, true, "owned duplicate receipt"),
        )?;
        old
    } else {
        fixed(repo.refname_to_id(reference))?
    };
    let (w, next) = fixture_push_with_audit_controller(w, old, duplicate, || Ok(()))?;
    let updates = w.server.receive_updates();
    assert!(updates.len() == before.len() + 1, "exactly one new receipt");
    assert!(
        updates.last()
            == Some(&ReceiveUpdate {
                reference: reference.into(),
                old_oid: old,
                new_oid: next,
                accepted: true,
            }),
        "exact accepted receiver receipt"
    );
    outcome(
        fixed(w.sync(w.primary()))?,
        false,
        SynchronizationTarget::Primary,
        next,
    );
    assert_eq!(fixed(repo.refname_to_id(reference))?, next);
    assert_eq!(
        fixed(fixed(repo.index())?.write_tree())?,
        fixed(repo.find_commit(next))?.tree_id()
    );
    assert!(fixed(std::fs::read(w.root.join("safety-control")))? == BODY.as_bytes());
    assert_eq!(w.server.receive_updates(), updates);
    Ok(())
}

fn fixture_push_with_audit_controller(
    w: World,
    old: git2::Oid,
    duplicate: bool,
    after_held: impl FnOnce() -> Result<(), FixtureError>,
) -> Result<(World, git2::Oid), FixtureError> {
    use std::{sync::mpsc, time::Duration};
    let reference = "refs/heads/main";
    let (events, progress) = mpsc::channel();
    std::thread::scope(|scope| {
        // Keep the hold inside the scope closure: every error and unwind drops it
        // before scope auto-joins the worker. A cancelled controller must not detach
        // the World owner or join while its receipt publication is still held.
        let hold = w.server.hold_receive_audit(events.clone())?;
        let worker = scope.spawn(move || {
            let result = if duplicate {
                push_peer(&w.peer, &w.server, reference)
                    .and_then(|()| fixed(w.peer.refname_to_id(reference)))
            } else {
                incoming_owned_paths(&w, old, reference, false)
            };
            let _ = events.send(ReceiveAuditEvent::HelperReturned);
            (w, result)
        });
        let mut held = false;
        let mut transport_returned = false;
        let mut receipt_waiting = false;
        while !(held && transport_returned && receipt_waiting) {
            match fixed(progress.recv_timeout(Duration::from_secs(10)))? {
                ReceiveAuditEvent::PublicationHeld => held = true,
                ReceiveAuditEvent::TransportReturned => transport_returned = true,
                ReceiveAuditEvent::ReceiptWaiting => receipt_waiting = true,
                ReceiveAuditEvent::HelperReturned => {
                    panic!("fixture push returned before receiver receipt publication")
                }
            }
        }
        // No timing sleep: the real report-status returned, the ref effect is proven,
        // and the helper acknowledged waiting for evidence still held by this gate.
        after_held()?;
        hold.release()?;
        let (w, next) = fixed(worker.join())?;
        Ok((w, next?))
    })
}

fn fixture_receiver_controller_cleanup(unwind: bool) -> Result<(), FixtureError> {
    use std::{panic::AssertUnwindSafe, sync::mpsc, time::Duration};
    let w = World::new()?;
    let old = fixed(w.peer.refname_to_id("refs/heads/main"))?;
    let roots = [
        w.directory.path().to_path_buf(),
        w.server.root().to_path_buf(),
    ];
    let reached = std::sync::atomic::AtomicBool::new(false);
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        fixture_push_with_audit_controller(w, old, false, || {
            reached.store(true, Ordering::SeqCst);
            if unwind {
                panic!("forced receiver audit controller unwind");
            }
            // A live sender with no message makes this controller timeout exact;
            // it does not change the transport or receipt-wait deadlines.
            let (_sender, receiver) = mpsc::channel::<()>();
            fixed(receiver.recv_timeout(Duration::ZERO))
        })
    }));
    assert!(reached.load(Ordering::SeqCst), "publication gate reached");
    assert!(
        if unwind {
            result.is_err()
        } else {
            matches!(result, Ok(Err(FixtureError)))
        },
        "forced controller failure observed"
    );
    assert!(
        roots.iter().all(|root| !root.exists()),
        "worker joined and fixture teardown finished before controller exit"
    );
    Ok(())
}

fn fixture_noop_push() -> Result<(), FixtureError> {
    let w = World::new()?;
    let before = w.server.receive_updates();
    let oid = fixed(w.peer.refname_to_id("refs/heads/main"))?;
    push_peer(&w.peer, &w.server, "refs/heads/main")?;
    outcome(
        fixed(w.sync(w.primary()))?,
        false,
        SynchronizationTarget::Primary,
        oid,
    );
    assert_eq!(w.server.receive_updates(), before);
    Ok(())
}

fn fixture_rejected_push() -> Result<(), FixtureError> {
    let w = World::new()?;
    let reference = "refs/heads/main";
    let before = w.server.receive_updates().len();
    let old = fixed(w.bare()?.refname_to_id(reference))?;
    let new = advance(&w.peer, reference, "owned-rejected-update")?;
    w.server.reject_primary_updates(true)?;
    assert!(
        push_peer(&w.peer, &w.server, reference).is_err(),
        "receiver rejection is not helper success"
    );
    w.server.wait_for_receive_update(
        before,
        &ReceiveUpdate {
            reference: reference.into(),
            old_oid: old,
            new_oid: new,
            accepted: false,
        },
    )?;
    assert_eq!(fixed(w.bare()?.refname_to_id(reference))?, old);
    assert!(w.server.receive_updates().len() == before + 1);
    Ok(())
}
