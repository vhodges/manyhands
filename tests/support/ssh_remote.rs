//! Disposable loopback SSH server. Only this test module launches Git helpers.
#![allow(dead_code)]

#[path = "ssh_server.rs"]
mod server;

use crate::repository::transport::HostKeyIdentity;
use russh::keys::{PrivateKey, PublicKey};
use std::{
    fmt,
    net::{SocketAddr, TcpListener},
    path::{Path, PathBuf},
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::{runtime::Runtime, sync::watch};

/// Dropping the controller aborts the held advertisement instead of releasing it.
pub struct AdvertisementHold {
    reached: std::sync::mpsc::Receiver<()>,
    release: tokio::sync::oneshot::Sender<()>,
}
pub(super) struct AdvertisementGate {
    pub reached: std::sync::mpsc::Sender<()>,
    pub release: tokio::sync::oneshot::Receiver<()>,
}
/// Fixed labels shared by the gated receiver-receipt regression and its worker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReceiveAuditEvent {
    PublicationHeld,
    TransportReturned,
    ReceiptWaiting,
    HelperReturned,
}
pub struct ReceiveAuditHold(tokio::sync::oneshot::Sender<()>);
impl ReceiveAuditHold {
    pub fn release(self) -> Result<(), FixtureError> {
        fixed(self.0.send(()))
    }
}
pub(super) struct ReceiveAuditGate {
    pub events: std::sync::mpsc::Sender<ReceiveAuditEvent>,
    pub release: tokio::sync::oneshot::Receiver<()>,
}
impl AdvertisementHold {
    pub fn wait_until_held(&self) -> Result<(), FixtureError> {
        fixed(self.reached.recv_timeout(Duration::from_secs(10)))
    }

    pub fn release(self) -> Result<(), FixtureError> {
        fixed(self.release.send(()))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FixtureError;

/// A closed set of refs in the fixture-owned bare repository, never a path or command.
#[derive(Clone, Copy)]
pub enum ObservationRef {
    RemoteTicket,
    LocalTicket,
    RemoteDocument,
    MalformedTicket,
}
impl ObservationRef {
    pub fn name(self) -> &'static str {
        match self {
            Self::RemoteTicket => "refs/heads/manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAV",
            Self::LocalTicket => "refs/heads/manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAW",
            Self::RemoteDocument => "refs/heads/manyhands/document/01ARZ3NDEKTSV4RRFFQ69G5FAX",
            Self::MalformedTicket => "refs/heads/manyhands/ticket/private-response-marker",
        }
    }
}
impl fmt::Display for FixtureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SSH fixture failed")
    }
}
impl std::error::Error for FixtureError {}
pub fn fixed<T, E>(result: Result<T, E>) -> Result<T, FixtureError> {
    result.map_err(|_| FixtureError)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitHelper {
    program: PathBuf,
    argument: Option<&'static str>,
}

impl From<PathBuf> for GitHelper {
    fn from(program: PathBuf) -> Self {
        Self {
            program,
            argument: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FixtureBoundary {
    Handshake,
    Authentication,
    ExecAcknowledgement,
    Advertisement,
    Transfer,
    AfterReceivePack,
}
#[derive(Clone, Copy)]
pub(super) enum Fault {
    Disconnect(FixtureBoundary),
    Stall(FixtureBoundary, Duration),
    Pace(Duration),
}

pub(super) struct Shared {
    pub host: Mutex<PrivateKey>,
    pub allowed: Mutex<PublicKey>,
    pub reject: AtomicBool,
    pub anonymous: AtomicBool,
    pub accepted: Mutex<Vec<Vec<u8>>>,
    pub helpers: AtomicUsize,
    pub command_path: Mutex<String>,
    pub commands: Mutex<Vec<Vec<u8>>>,
    pub active_helpers: AtomicUsize,
    pub completed_helpers: AtomicUsize,
    pub helper_tasks: Mutex<Vec<tokio::task::JoinHandle<()>>>,
    pub fault: Mutex<Option<Fault>>,
    pub advertisement_hold: Mutex<Option<AdvertisementGate>>,
    pub receive_audit_hold: Mutex<Option<ReceiveAuditGate>>,
    pub receive_receipt_events: Mutex<Option<std::sync::mpsc::Sender<ReceiveAuditEvent>>>,
    pub hostile: Mutex<Option<String>>,
    pub receive_status_withheld: AtomicBool,
    pub receive_race: Mutex<Option<(git2::Oid, git2::Oid)>>,
    pub receive_updates: Mutex<Vec<ReceiveUpdate>>,
    pub receive_updates_changed: Condvar,
    pub receive_advertisements: AtomicUsize,
    pub repository: PathBuf,
    pub upload: GitHelper,
    pub receive: GitHelper,
    pub shutdown: watch::Receiver<bool>,
}

/// A parsed receive command paired with child completion and actual owned ref proof.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReceiveUpdate {
    pub reference: String,
    pub old_oid: git2::Oid,
    pub new_oid: git2::Oid,
    pub accepted: bool,
}

pub struct SshRemoteFixture {
    runtime: Option<Runtime>,
    stop: watch::Sender<bool>,
    listener: Option<tokio::task::JoinHandle<()>>,
    shared: Arc<Shared>,
    address: SocketAddr,
    client_key: PathBuf,
    commit: git2::Oid,
    directory: tempfile::TempDir,
}

impl SshRemoteFixture {
    /// Hold the next proven ref effect after forwarding report-status but before
    /// publishing its receipt. Dropping the controller refuses accepted evidence.
    pub fn hold_receive_audit(
        &self,
        events: std::sync::mpsc::Sender<ReceiveAuditEvent>,
    ) -> Result<ReceiveAuditHold, FixtureError> {
        let mut hold = fixed(self.shared.receive_audit_hold.lock())?;
        let mut receipt_events = fixed(self.shared.receive_receipt_events.lock())?;
        if hold.is_some() || receipt_events.is_some() {
            return Err(FixtureError);
        }
        let (release, released) = tokio::sync::oneshot::channel();
        *receipt_events = Some(events.clone());
        *hold = Some(ReceiveAuditGate {
            events,
            release: released,
        });
        Ok(ReceiveAuditHold(release))
    }

    pub fn fixture_push_transport_returned(&self) -> Result<(), FixtureError> {
        if let Some(events) = fixed(self.shared.receive_receipt_events.lock())?.as_ref() {
            fixed(events.send(ReceiveAuditEvent::TransportReturned))?;
        }
        Ok(())
    }

    pub fn hold_advertisement(&self) -> Result<AdvertisementHold, FixtureError> {
        let mut gate = fixed(self.shared.advertisement_hold.lock())?;
        if gate.is_some() {
            return Err(FixtureError);
        }
        let (ready, reached) = std::sync::mpsc::channel();
        let (release, released) = tokio::sync::oneshot::channel();
        *gate = Some(AdvertisementGate {
            reached: ready,
            release: released,
        });
        Ok(AdvertisementHold { reached, release })
    }

    pub fn set_observation_ref(
        &self,
        reference: ObservationRef,
        present: bool,
    ) -> Result<(), FixtureError> {
        let repo = fixed(git2::Repository::open_bare(&self.shared.repository))?;
        if present {
            fixed(repo.reference(
                reference.name(),
                self.commit,
                true,
                "owned observation fixture",
            ))?;
        } else {
            fixed(fixed(repo.find_reference(reference.name()))?.delete())?;
        }
        Ok(())
    }

    pub fn start() -> Result<Self, FixtureError> {
        Self::start_with_helpers(server::discover_helpers(None)?)
    }

    pub fn start_with_helpers(helpers: (GitHelper, GitHelper)) -> Result<Self, FixtureError> {
        if !helpers.0.program.is_file() || !helpers.1.program.is_file() {
            return Err(FixtureError);
        }
        let directory = fixed(
            tempfile::Builder::new()
                .prefix("manyhands SSH fixture ")
                .tempdir(),
        )?;
        let repo_path = directory.path().join("owned bare repository.git");
        let repo = fixed(git2::Repository::init_bare(&repo_path))?;
        let blob = fixed(repo.blob(b"fixture\n"))?;
        let mut builder = fixed(repo.treebuilder(None))?;
        fixed(builder.insert("fixture.txt", blob, 0o100644))?;
        let tree_id = fixed(builder.write())?;
        let tree = fixed(repo.find_tree(tree_id))?;
        let signature = fixed(git2::Signature::now("Fixture", "fixture@example.invalid"))?;
        let commit = fixed(repo.commit(
            Some("refs/heads/main"),
            &signature,
            &signature,
            "fixture",
            &tree,
            &[],
        ))?;
        fixed(repo.set_head("refs/heads/main"))?;
        let client = generate_key()?;
        let client_key = directory.path().join("allowed client key");
        fixed(std::fs::write(
            &client_key,
            fixed(client.to_openssh(russh::keys::ssh_key::LineEnding::LF))?.as_bytes(),
        ))?;
        let host = generate_key()?;
        let listener = fixed(TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)))?;
        fixed(listener.set_nonblocking(true))?;
        let address = fixed(listener.local_addr())?;
        let mut builder = tokio::runtime::Builder::new_multi_thread();
        builder.worker_threads(2).enable_all();
        #[cfg(unix)]
        builder.on_thread_start(|| {
            crate::ssh_harness::signals::server_thread_start()
                .expect("fixture worker signal setup failed");
        });
        let runtime = fixed(builder.build())?;
        let (stop, shutdown) = watch::channel(false);
        let shared = Arc::new(Shared {
            host: Mutex::new(host),
            allowed: Mutex::new(client.public_key().clone()),
            reject: AtomicBool::new(false),
            anonymous: AtomicBool::new(false),
            accepted: Mutex::new(Vec::new()),
            helpers: AtomicUsize::new(0),
            command_path: Mutex::new("/fixture.git".into()),
            commands: Mutex::new(Vec::new()),
            active_helpers: AtomicUsize::new(0),
            completed_helpers: AtomicUsize::new(0),
            helper_tasks: Mutex::new(Vec::new()),
            fault: Mutex::new(None),
            advertisement_hold: Mutex::new(None),
            receive_audit_hold: Mutex::new(None),
            receive_receipt_events: Mutex::new(None),
            hostile: Mutex::new(None),
            receive_status_withheld: AtomicBool::new(false),
            receive_race: Mutex::new(None),
            receive_updates: Mutex::new(Vec::new()),
            receive_updates_changed: Condvar::new(),
            receive_advertisements: AtomicUsize::new(0),
            repository: repo_path,
            upload: helpers.0,
            receive: helpers.1,
            shutdown,
        });
        let task = runtime.block_on(async {
            let listener = fixed(tokio::net::TcpListener::from_std(listener))?;
            let (ready, started) = tokio::sync::oneshot::channel();
            let task = tokio::spawn(server::serve(listener, shared.clone(), ready));
            fixed(fixed(
                tokio::time::timeout(Duration::from_secs(5), started).await,
            )?)?;
            Ok::<_, FixtureError>(task)
        })?;
        Ok(Self {
            runtime: Some(runtime),
            stop,
            listener: Some(task),
            shared,
            address,
            client_key,
            commit,
            directory,
        })
    }

    /// One competing owned primary write after a complete ordinary command is
    /// received but before any of its bytes reach the real receive-pack child.
    pub fn race_primary_update(
        &self,
        expected: git2::Oid,
        competing: git2::Oid,
    ) -> Result<(), FixtureError> {
        let repo = fixed(git2::Repository::open_bare(&self.shared.repository))?;
        fixed(repo.find_commit(competing))?;
        if fixed(repo.refname_to_id("refs/heads/main"))? != expected
            || self.shared.receive_race.lock().unwrap().is_some()
        {
            return Err(FixtureError);
        }
        *self.shared.receive_race.lock().unwrap() = Some((expected, competing));
        Ok(())
    }
    pub fn receive_updates(&self) -> Vec<ReceiveUpdate> {
        self.shared.receive_updates.lock().unwrap().clone()
    }
    /// Await the exact receipt appended after a cursor captured before the push.
    /// Existing identical receipts and rejected commands cannot prove success.
    pub fn wait_for_receive_update(
        &self,
        after: usize,
        expected: &ReceiveUpdate,
    ) -> Result<(), FixtureError> {
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut updates = fixed(self.shared.receive_updates.lock())?;
        loop {
            let tail = updates.get(after..).ok_or(FixtureError)?;
            if let Some(update) = tail.iter().find(|update| {
                update.reference == expected.reference
                    && update.old_oid == expected.old_oid
                    && update.new_oid == expected.new_oid
            }) {
                return if update.accepted == expected.accepted {
                    Ok(())
                } else {
                    Err(FixtureError)
                };
            }
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .ok_or(FixtureError)?;
            if let Some(events) = fixed(self.shared.receive_receipt_events.lock())?.take() {
                fixed(events.send(ReceiveAuditEvent::ReceiptWaiting))?;
            }
            (updates, _) = fixed(
                self.shared
                    .receive_updates_changed
                    .wait_timeout(updates, remaining),
            )?;
        }
    }
    pub fn receive_advertisements(&self) -> usize {
        self.shared.receive_advertisements.load(Ordering::SeqCst)
    }
    pub fn clear_fault(&self) {
        *self.shared.fault.lock().unwrap() = None;
    }
    /// Receiver-native refusal of updates to the checked-out primary. No hooks/shell.
    pub fn reject_primary_updates(&self, reject: bool) -> Result<(), FixtureError> {
        let repo = fixed(git2::Repository::open_bare(&self.shared.repository))?;
        let mut config = fixed(repo.config())?;
        fixed(config.set_bool("core.bare", !reject))?;
        fixed(config.set_str(
            "receive.denyCurrentBranch",
            if reject { "refuse" } else { "ignore" },
        ))
    }
    pub fn receive_status_withheld(&self) -> bool {
        self.shared.receive_status_withheld.load(Ordering::SeqCst)
    }
    #[cfg(unix)]
    pub fn worker_child_signal_masks(&self) -> Result<[bool; 2], FixtureError> {
        self.runtime.as_ref().ok_or(FixtureError)?.block_on(async {
            let worker = fixed(
                tokio::spawn(async { crate::ssh_harness::signals::child_signal_blocked() }).await,
            )??;
            let blocking = fixed(
                tokio::task::spawn_blocking(crate::ssh_harness::signals::child_signal_blocked)
                    .await,
            )??;
            Ok([worker, blocking])
        })
    }
    pub fn hostile_rejection(&self, marker: &str) {
        assert!(
            marker.len() == b"non-fast-forward".len(),
            "hostile marker must preserve packet width"
        );
        *self.shared.hostile.lock().unwrap() = Some(marker.into());
    }
    pub fn url(&self) -> String {
        format!("ssh://fixture@{}/fixture.git", self.address)
    }
    /// Explicit adversarial mode; normal fixtures deny none authentication.
    pub fn accept_anonymous(&self) {
        self.shared.anonymous.store(true, Ordering::SeqCst);
    }
    pub fn address(&self) -> SocketAddr {
        self.address
    }
    pub fn host_identity(&self) -> HostKeyIdentity {
        let host = self.shared.host.lock().unwrap();
        HostKeyIdentity {
            algorithm: host.algorithm().to_string(),
            sha256: host
                .public_key()
                .fingerprint(russh::keys::HashAlg::Sha256)
                .to_string(),
        }
    }
    pub fn host_public_key(&self) -> PublicKey {
        self.shared.host.lock().unwrap().public_key().clone()
    }
    pub fn allowed_client_public_key(&self) -> Vec<u8> {
        self.shared.allowed.lock().unwrap().to_bytes().unwrap()
    }
    pub fn allow_client_public_key(&self, key: PublicKey) {
        *self.shared.allowed.lock().unwrap() = key;
    }
    pub fn accepted_keys(&self) -> Vec<Vec<u8>> {
        self.shared.accepted.lock().unwrap().clone()
    }
    /// Choose one exact virtual target; helpers always receive the owned repo path.
    pub fn expect_command_path(&self, path: &str) {
        assert!(
            [
                "fixture.git",
                "/fixture.git",
                "~/fixture.git",
                "~user/fixture.git",
                "fixture.git?other",
                "fixture.git#other",
                "/fixture.git?other",
                "/fixture.git#other",
            ]
            .contains(&path)
        );
        *self.shared.command_path.lock().unwrap() = path.into();
    }
    pub fn commands(&self) -> Vec<Vec<u8>> {
        self.shared.commands.lock().unwrap().clone()
    }
    pub fn helper_invocations(&self) -> usize {
        self.shared.helpers.load(Ordering::SeqCst)
    }
    pub fn active_helpers(&self) -> usize {
        self.shared.active_helpers.load(Ordering::SeqCst)
    }
    pub fn completed_helpers(&self) -> usize {
        self.shared.completed_helpers.load(Ordering::SeqCst)
    }
    pub fn client_key_path(&self) -> &Path {
        &self.client_key
    }
    pub fn commit_id(&self) -> git2::Oid {
        self.commit
    }
    pub fn root(&self) -> &Path {
        self.directory.path()
    }
    pub fn repository_path(&self) -> &Path {
        &self.shared.repository
    }
    pub fn rotate_host_key(&self) -> Result<(), FixtureError> {
        *self.shared.host.lock().unwrap() = generate_key()?;
        Ok(())
    }
    pub fn reject_client(&self) {
        self.shared.reject.store(true, Ordering::SeqCst);
    }
    pub fn restore_client(&self) {
        self.shared.reject.store(false, Ordering::SeqCst);
    }
    pub fn disconnect_at(&self, boundary: FixtureBoundary) {
        *self.shared.fault.lock().unwrap() = Some(Fault::Disconnect(boundary));
    }
    pub fn stall_at(&self, boundary: FixtureBoundary, delay: Duration) {
        *self.shared.fault.lock().unwrap() = Some(Fault::Stall(boundary, delay));
    }
    pub fn pace_transfer(&self, delay: Duration) {
        *self.shared.fault.lock().unwrap() = Some(Fault::Pace(delay));
    }
    pub fn shutdown(&mut self) -> Result<(), FixtureError> {
        let _ = self.stop.send(true);
        let mut joined = true;
        if let Some(runtime) = self.runtime.take() {
            if let Some(task) = self.listener.take() {
                joined = runtime.block_on(async {
                    matches!(
                        tokio::time::timeout(Duration::from_secs(5), task).await,
                        Ok(Ok(()))
                    )
                });
            }
            runtime.shutdown_timeout(Duration::from_secs(3));
        }
        if joined && self.active_helpers() == 0 {
            Ok(())
        } else {
            Err(FixtureError)
        }
    }
}

pub fn discover_helpers_at(directory: &Path) -> Result<(GitHelper, GitHelper), FixtureError> {
    server::discover_helpers(Some(directory))
}

impl Drop for SshRemoteFixture {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

pub fn generate_key() -> Result<PrivateKey, FixtureError> {
    let key = fixed(ssh_key::PrivateKey::random(
        &mut ssh_key::rand_core::OsRng,
        ssh_key::Algorithm::Ed25519,
    ))?;
    let encoded = fixed(key.to_openssh(ssh_key::LineEnding::LF))?;
    fixed(PrivateKey::from_openssh(encoded.as_bytes()))
}

pub fn receiver_command_fragmentation() -> Result<(), FixtureError> {
    server::receiver_command_fragmentation()
}
