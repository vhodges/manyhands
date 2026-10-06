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
    ssh_harness::run(CASES);
}
const CASES: &[ssh_harness::Case] = &[
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
    let mut remote = fixed(repo.remote_anonymous(&fixture.url()))?;
    let mut options = git2::PushOptions::new();
    options.remote_callbacks(callbacks(fixture));
    fixed(remote.push(&[&format!("{reference}:{reference}")], Some(&mut options)))
}
fn commit(
    repo: &git2::Repository,
    parent: git2::Oid,
    name: &str,
    bytes: &[u8],
) -> Result<git2::Oid, FixtureError> {
    let parent = fixed(repo.find_commit(parent))?;
    let mut builder = fixed(repo.treebuilder(Some(&fixed(parent.tree())?)))?;
    fixed(builder.insert(name, fixed(repo.blob(bytes))?, 0o100644))?;
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
    outcome(
        fixed(w.sync(w.context_request()))?,
        false,
        w.context_request().target,
        first,
    );
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
    advance(&repo, "refs/heads/main", "local-divergent")?;
    advance(&w.peer, "refs/heads/main", "remote-divergent")?;
    push_peer(&w.peer, &w.server, "refs/heads/main")?;
    let before = physical(&w.root, &w.root)?;
    let n = w.server.receive_updates().len();
    assert!(matches!(
        w.sync(w.primary()),
        Err(SynchronizationError::MergeRequired { .. })
    ));
    assert_eq!(physical(&w.root, &w.root)?, before);
    assert_eq!(w.server.receive_updates().len(), n);
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
    let context = w.context()?;
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
    let before = physical(&w.root, &context.worktree)?;
    let primary = physical(&w.root, &w.root)?;
    let n = w.server.receive_updates().len();
    assert!(matches!(
        w.sync(w.context_request()),
        Err(SynchronizationError::MergeRequired { .. })
    ));
    assert_eq!(physical(&w.root, &context.worktree)?, before);
    assert_eq!(physical(&w.root, &w.root)?, primary);
    assert_eq!(fixed(w.repo()?.refname_to_id(CONTEXT))?, local);
    assert_eq!(w.server.receive_updates().len(), n);
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
    let mut request = w.context_request();
    request.approval = None;
    assert!(matches!(
        w.sync(request),
        Err(SynchronizationError::HistoryUnknown)
    ));
    assert_eq!(physical(&w.root, &context.worktree)?, before);
    assert_eq!(w.server.receive_updates().len(), n);
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
