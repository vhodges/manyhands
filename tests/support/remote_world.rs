//! The two-clone synchronization world: an owned SSH server, a clone
//! that Manyhands is enabled in with the server as its publication
//! remote, and a peer clone that moves the server's branches.
//!
//! This is shared by the test targets that synchronize against a real
//! server. Each declares `ssh_remote` and `ssh_privacy` beside it.
#![allow(dead_code)]

use crate::{
    repository::{keys::*, transport::*, *},
    ssh_privacy,
    ssh_remote::*,
};
use std::path::{Path, PathBuf};

pub const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
pub const CONTEXT: &str = "refs/heads/manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAW";
pub const PASSWORD: &str = "sync-acceptance-passphrase-private";
pub const BODY: &str = "canonical-Markdown-private-sync-sentinel";
pub const HOSTILE: &str = "server-private!!"; // receive report-status packet width: 16
pub struct Provider;
impl SessionCredentialProvider for Provider {
    fn request_passphrase(&mut self, _: &UnlockRequest) -> PassphraseResponse {
        PassphraseResponse::Supplied(SecretPassphrase::new(PASSWORD.into()).unwrap())
    }
}
pub fn callbacks(fixture: &SshRemoteFixture) -> git2::RemoteCallbacks<'_> {
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
pub fn clone_owned(
    fixture: &SshRemoteFixture,
    root: &Path,
) -> Result<git2::Repository, FixtureError> {
    let mut fetch = git2::FetchOptions::new();
    fetch.remote_callbacks(callbacks(fixture));
    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fetch);
    fixed(builder.clone(&fixture.url(), root))
}
pub fn push_peer(
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
pub fn commit(
    repo: &git2::Repository,
    parent: git2::Oid,
    name: &str,
    bytes: &[u8],
) -> Result<git2::Oid, FixtureError> {
    commit_with_mode(repo, parent, name, bytes, 0o100644)
}
pub fn commit_with_mode(
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
pub fn advance(
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
pub struct World {
    pub server: SshRemoteFixture,
    pub service: RepositoryService,
    pub root: PathBuf,
    pub peer: git2::Repository,
    pub directory: tempfile::TempDir,
    pub probes: Vec<Vec<u8>>,
    pub paths: Vec<Vec<u8>>,
}
impl World {
    pub fn new() -> Result<Self, FixtureError> {
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
    pub fn data(&self) -> PathBuf {
        self.directory.path().join("data")
    }
    pub fn db(&self) -> Result<rusqlite::Connection, FixtureError> {
        fixed(rusqlite::Connection::open(self.data().join(REGISTRY_FILE)))
    }
    pub fn repo(&self) -> Result<git2::Repository, FixtureError> {
        fixed(git2::Repository::open(&self.root))
    }
    pub fn bare(&self) -> Result<git2::Repository, FixtureError> {
        fixed(git2::Repository::open_bare(self.server.repository_path()))
    }
    pub fn request(&self, target: SynchronizationTarget) -> SynchronizeRemoteRequest {
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
    pub fn sync(
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
    pub fn primary(&self) -> SynchronizeRemoteRequest {
        self.request(SynchronizationTarget::Primary)
    }
    pub fn context_request(&self) -> SynchronizeRemoteRequest {
        self.request(SynchronizationTarget::Context {
            kind: AuthoringKind::Ticket,
            item_id: ITEM.parse().unwrap(),
        })
    }
    pub fn context(&mut self) -> Result<ItemContext, FixtureError> {
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
    pub fn privacy(&self) -> Result<(), FixtureError> {
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
pub fn outcome(
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
pub fn accepted(server: &SshRemoteFixture) -> usize {
    server
        .receive_updates()
        .iter()
        .filter(|update| update.accepted)
        .count()
}
pub fn one_update(
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
