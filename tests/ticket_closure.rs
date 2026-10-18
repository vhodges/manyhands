//! Public-API ticket closure acceptance against two isolated SSH clones
//! (Wave 02 Cycle 10 Task 8). Only the public library, the owned SSH fixture
//! and fixture-only Git/SQL controls are used.
#![allow(clippy::result_large_err)]
pub use manyhands::*;
#[path = "ticket_closure/checkpoint.rs"]
mod checkpoint;
#[path = "ticket_closure/exits.rs"]
mod exits;
#[path = "ticket_closure/journeys.rs"]
mod journeys;
#[path = "ticket_closure/privacy.rs"]
mod privacy;
#[path = "support/privacy_rows.rs"]
mod privacy_rows;
#[path = "ticket_closure/races.rs"]
mod races;
#[path = "support/remote_world.rs"]
mod remote_world;
#[path = "support/ssh_harness.rs"]
mod ssh_harness;
#[path = "support/ssh_privacy.rs"]
mod ssh_privacy;
#[path = "support/ssh_remote.rs"]
mod ssh_remote;

use repository::{keys::*, transport::*, *};
use ssh_remote::{FixtureError, SshRemoteFixture, fixed};
use std::path::{Path, PathBuf};

/// The fixture's `ObservationRef::RemoteTicket` names this ticket's context.
const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
const TICKET_PATH: &str = ".manyhands/tickets/01ARZ3NDEKTSV4RRFFQ69G5FAV/ticket.md";
const MAIN: &str = "refs/heads/main";
const TRACKING_MAIN: &str = "refs/remotes/origin/main";
const CONTEXT: &str = "refs/heads/manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAV";
const TRACKING: &str = "refs/remotes/origin/manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAV";
const TITLE: &str = "closure-private-title-canary";
const SEED: &str = "closure-private-seed-body-canary\n";
const BODY: &str = "closure-private-ticket-body-canary\n";
const COMMENT: &str = "closure-private-comment-body-canary\n";
const NAME: &str = "closure-private-name-canary";
const EMAIL: &str = "closure-private-email@example.invalid";
const PEER_NAME: &str = "closure-peer-name-canary";
const PEER_EMAIL: &str = "closure-peer-email@example.invalid";

fn main() {
    // SAFETY: the real main owns initialization before any fixture/runner thread.
    if unsafe { ssh_harness::initialize() }.is_err() {
        eprintln!("ticket closure initialization failed");
        std::process::exit(1);
    }
    if exits::maybe_child() {
        return;
    }
    let cases: Vec<_> = journeys::CASES
        .iter()
        .chain(journeys::BLOCKERS)
        .chain(checkpoint::CASES)
        .chain(races::CASES)
        .chain(exits::CASES)
        .chain(privacy::CASES)
        .copied()
        .collect();
    ssh_harness::run_with_short_output_privacy(&cases, privacy::OUTPUT_CONTROLS);
}

fn closed_by(name: &str, email: &str) -> String {
    format!("{name} <{email}>")
}
fn item() -> canonical::ItemId {
    TICKET.parse().unwrap()
}
fn close_subject() -> String {
    format!("Close ticket {TICKET}")
}
fn merge_subject() -> String {
    format!("Integrate closed ticket {TICKET}")
}

struct Side {
    service: RepositoryService,
    root: PathBuf,
    data: PathBuf,
    name: &'static str,
    email: &'static str,
}
impl Side {
    fn repo(&self) -> Result<git2::Repository, FixtureError> {
        fixed(git2::Repository::open(&self.root))
    }
    fn worktree(&self) -> PathBuf {
        self.root.join(".manyhands/worktrees").join(TICKET)
    }
    fn db(&self) -> Result<rusqlite::Connection, FixtureError> {
        fixed(rusqlite::Connection::open(self.data.join(REGISTRY_FILE)))
    }
    fn closed_by(&self) -> String {
        closed_by(self.name, self.email)
    }
    fn target(&self, intent: ContextIntent) -> AuthoringTarget {
        AuthoringTarget {
            root: self.root.clone(),
            kind: AuthoringKind::Ticket,
            item_id: item(),
            intent,
            operation_id: OperationId::new(),
        }
    }
}

struct Pair {
    server: SshRemoteFixture,
    a: Side,
    b: Side,
    probes: Vec<Vec<u8>>,
    _directory: tempfile::TempDir,
}
impl Pair {
    fn new() -> Result<Self, FixtureError> {
        Self::new_with_remote(true)
    }
    fn new_with_remote(remote: bool) -> Result<Self, FixtureError> {
        let server = SshRemoteFixture::start()?;
        let plain = fixed(std::fs::read(server.client_key_path()))?;
        let key = fixed(ssh_key::PrivateKey::read_openssh_file(
            server.client_key_path(),
        ))?;
        let mut salt = vec![0; 16];
        ssh_key::rand_core::RngCore::fill_bytes(&mut ssh_key::rand_core::OsRng, &mut salt);
        let encrypted = fixed(
            fixed(key.encrypt_with(
                ssh_key::Cipher::Aes256Ctr,
                ssh_key::Kdf::Bcrypt { salt, rounds: 1 },
                ssh_key::rand_core::RngCore::next_u32(&mut ssh_key::rand_core::OsRng),
                remote_world::PASSWORD,
            ))?
            .to_openssh(ssh_key::LineEnding::LF),
        )?;
        fixed(std::fs::write(
            server.client_key_path(),
            encrypted.as_bytes(),
        ))?;
        let mut probes = vec![
            plain.clone(),
            encrypted.as_bytes().to_vec(),
            server.url().into_bytes(),
            remote_world::PASSWORD.as_bytes().to_vec(),
            TITLE.as_bytes().to_vec(),
            SEED.trim_end().as_bytes().to_vec(),
            BODY.trim_end().as_bytes().to_vec(),
            COMMENT.trim_end().as_bytes().to_vec(),
        ];
        probes.extend(identity_probes());
        probes.extend(
            plain
                .split(|byte| *byte == b'\n')
                .chain(encrypted.as_bytes().split(|byte| *byte == b'\n'))
                .filter(|line| line.len() > 40)
                .map(<[u8]>::to_vec),
        );
        ssh_privacy::save(&probes)?;
        let directory = fixed(tempfile::Builder::new().prefix("tc").tempdir())?;
        // One canonical spelling of every root: snapshots and paths compare
        // through it on every platform.
        let base = fixed(std::fs::canonicalize(directory.path()))?;
        let root = base.join("a");
        let repo = remote_world::clone_owned(&server, &root)?;
        identity(&repo, NAME, EMAIL)?;
        let service = fixed(RepositoryService::open_at(&base.join("da")))?;
        fixed(service.enable(EnableRepositoryRequest {
            root: root.clone(),
            primary_branch: "main".into(),
            identity: None,
            operation_id: OperationId::new(),
        }))?;
        let config = fixed(canonical::serialize_repository_config(
            &canonical::RepositoryConfig {
                primary_branch: "main".into(),
                publication_remote: remote.then(|| "origin".into()),
                unknown: toml::Table::new(),
            },
        ))?;
        commit_files(
            &repo,
            &[
                (".manyhands/config.toml", config.as_bytes()),
                (TICKET_PATH, ticket_source(SEED, None).as_bytes()),
            ],
        )?;
        remote_world::push_peer(&repo, &server, MAIN)?;
        let other = base.join("b");
        identity(
            &remote_world::clone_owned(&server, &other)?,
            PEER_NAME,
            PEER_EMAIL,
        )?;
        let b_service = fixed(RepositoryService::open_at(&base.join("db")))?;
        fixed(b_service.enable(EnableRepositoryRequest {
            root: other.clone(),
            primary_branch: "main".into(),
            identity: None,
            operation_id: OperationId::new(),
        }))?;
        for service in [&service, &b_service] {
            select_key(service, &server)?;
        }
        let pair = Self {
            server,
            a: Side {
                service,
                root,
                data: base.join("da"),
                name: NAME,
                email: EMAIL,
            },
            b: Side {
                service: b_service,
                root: other,
                data: base.join("db"),
                name: PEER_NAME,
                email: PEER_EMAIL,
            },
            probes,
            _directory: directory,
        };
        // The anonymous seed push does not update origin/main: observe it
        // through the public boundary first.
        for side in [&pair.a, &pair.b].into_iter().filter(|_| remote) {
            fixed(pair.sync_target(side, SynchronizationTarget::Primary))?;
        }
        Ok(pair)
    }
    fn approval(&self) -> HostApproval {
        HostApproval {
            authority: SshAuthority {
                host: "127.0.0.1".into(),
                port: self.server.address().port(),
            },
            expected: None,
            presented: self.server.host_identity(),
        }
    }
    fn sync(&self, side: &Side) -> Result<SynchronizationResult, SynchronizationError> {
        self.sync_target(side, ticket_target())
    }
    fn sync_target(
        &self,
        side: &Side,
        target: SynchronizationTarget,
    ) -> Result<SynchronizationResult, SynchronizationError> {
        let result = side.service.synchronize_remote(
            SynchronizeRemoteRequest {
                root: side.root.clone(),
                operation_id: OperationId::new(),
                target,
                restart: false,
                confirmed_identity: None,
                approval: Some(self.approval()),
            },
            &mut SessionCredentials::new(remote_world::Provider),
        );
        self.diagnostics(&format!("{result:?}"))
            .expect("private synchronization diagnostics");
        result
    }
    /// A product edit of the ticket in `side`'s context (provisioned on the
    /// first edit), checkpointed there.
    fn edit(&self, side: &Side, body: &str) -> Result<git2::Oid, FixtureError> {
        let root = if side.worktree().exists() {
            side.worktree()
        } else {
            side.root.clone()
        };
        let observed =
            ExpectedPathObservation::from_bytes(&fixed(std::fs::read(root.join(TICKET_PATH)))?);
        let result = side.service.save_ticket(SaveTicketRequest {
            target: side.target(ContextIntent::Edit),
            draft: draft(body),
            expected_path: observed,
        });
        self.diagnostics(&format!(
            "{:?}",
            result.as_ref().map(|_| ()).map_err(|e| e.kind)
        ))?;
        match fixed(result)? {
            SaveOutcome::Saved {
                checkpoint: LocalCheckpoint::Checkpointed { commit_oid },
                ..
            } => Ok(commit_oid),
            _ => Err(FixtureError),
        }
    }
    fn preflight(&self, side: &Side) -> Result<TicketClosurePreflight, FixtureError> {
        let preflight = fixed(
            side.service
                .preview_ticket_closure(PreviewTicketClosureRequest {
                    root: side.root.clone(),
                    item_id: item(),
                    confirmed_identity: None,
                }),
        )?;
        self.diagnostics(&format!("{preflight:?}"))?;
        Ok(preflight)
    }
    fn preview(&self, side: &Side) -> Result<TicketClosurePreview, FixtureError> {
        match self.preflight(side)? {
            TicketClosurePreflight::Ready(p) => Ok(p),
            TicketClosurePreflight::Blocked { blockers, .. } => {
                for b in blockers {
                    ssh_harness::observation(&[800, b as u128]);
                }
                Err(FixtureError)
            }
        }
    }
    fn blockers(&self, side: &Side) -> Result<Vec<ClosureBlocker>, FixtureError> {
        match self.preflight(side)? {
            TicketClosurePreflight::Blocked { blockers, .. } => Ok(blockers),
            TicketClosurePreflight::Ready(_) => Err(FixtureError),
        }
    }
    fn request(&self, side: &Side) -> Result<CloseTicketRequest, FixtureError> {
        let preview = self.preview(side)?;
        Ok(CloseTicketRequest {
            root: side.root.clone(),
            operation_id: OperationId::new(),
            item_id: item(),
            expected: preview.expectation(),
            approval: Some(self.approval()),
            confirmed_identity: None,
        })
    }
    fn execute(
        &self,
        service: &RepositoryService,
        request: CloseTicketRequest,
    ) -> Result<TicketClosureResult, TicketClosureError> {
        let result = service.close_ticket(
            request,
            &mut SessionCredentials::new(remote_world::Provider),
        );
        self.diagnostics(&format!("{result:?}"))
            .expect("redacted closure result");
        if let Err(error) = &result {
            self.diagnostics(&error.to_string())
                .expect("redacted closure error");
            ssh_harness::observation(&[700, category(error)]);
        }
        result
    }
    fn run(
        &self,
        side: &Side,
        request: &CloseTicketRequest,
    ) -> Result<TicketClosureOutcome, FixtureError> {
        Ok(authoritative(fixed(
            self.execute(&side.service, request.clone()),
        )?))
    }
    /// Preview, confirm and close in `side`; the result must be complete.
    fn close(&self, side: &Side) -> Result<TicketClosureOutcome, FixtureError> {
        let request = self.request(side)?;
        match fixed(self.execute(&side.service, request))? {
            TicketClosureResult::Complete(outcome) => Ok(outcome),
            TicketClosureResult::IndexPending(_) => Err(FixtureError),
        }
    }
    fn diagnostics(&self, rendered: &str) -> Result<(), FixtureError> {
        ssh_privacy::clean(rendered.as_bytes(), &self.probes)?;
        for side in [&self.a, &self.b] {
            let path = side.worktree().to_string_lossy().as_bytes().to_vec();
            ssh_privacy::clean(rendered.as_bytes(), &[path])?;
        }
        Ok(())
    }
    /// Every workflow row (live and backup) against every probe; every table
    /// against every probe but the accepted discovery values, which only
    /// their own `discovered_items` column may hold; and the raw store.
    fn privacy(&self) -> Result<(), FixtureError> {
        ssh_privacy::save(&self.probes)?;
        let exempt = self.exempt();
        let refs = exempt_refs(&exempt);
        let policy = row_policy(&refs);
        let cache_probes = cache_probes(&self.probes);
        let store_probes: Vec<_> = cache_probes
            .iter()
            .filter(|p| {
                p.as_slice() != TITLE.as_bytes()
                    && !exempt
                        .iter()
                        .any(|(_, value)| value.as_slice() == p.as_slice())
            })
            .cloned()
            .collect();
        for side in [&self.a, &self.b] {
            let paths = vec![side.worktree().to_string_lossy().as_bytes().to_vec()];
            ssh_privacy::save(&paths)?;
            let db = side.db()?;
            privacy_rows::scan_workflow_rows(&db, &self.probes, &policy)?;
            privacy_rows::scan_workflow_rows(&db, &paths, &policy)?;
            privacy_rows::scan_rows(&db, &cache_probes, true, &policy)?;
            let backup = side.data.join("v");
            if backup.exists() {
                fixed(std::fs::remove_file(&backup))?;
            }
            fixed(db.execute("VACUUM INTO ?1", [backup.to_str().ok_or(FixtureError)?]))?;
            let backup_db = fixed(rusqlite::Connection::open(&backup))?;
            privacy_rows::scan_workflow_rows(&backup_db, &self.probes, &policy)?;
            privacy_rows::scan_workflow_rows(&backup_db, &paths, &policy)?;
            privacy_rows::scan_rows(&backup_db, &cache_probes, true, &policy)?;
            ssh_privacy::scan(&side.data, &store_probes)?;
        }
        Ok(())
    }
    /// `(column, value)`: the ticket title and both clones' `closed_by`.
    fn exempt(&self) -> Vec<(&'static str, Vec<u8>)> {
        vec![
            ("title", TITLE.as_bytes().to_vec()),
            ("closed_by", self.a.closed_by().into_bytes()),
            ("closed_by", self.b.closed_by().into_bytes()),
        ]
    }
}

/// The tables closure adds to the workflow set, and its exemptions.
fn row_policy<'a>(exempt: &'a [(&'a str, &'a [u8])]) -> privacy_rows::RowPolicy<'a> {
    privacy_rows::RowPolicy {
        workflow: &[
            "ticket_closures",
            "document_promotions",
            "retired_contexts",
            "operation_records",
            "comment_publication_bindings",
        ],
        required: "ticket_closures",
        exempt,
    }
}
fn exempt_refs<'a>(exempt: &'a [(&'static str, Vec<u8>)]) -> Vec<(&'static str, &'a [u8])> {
    exempt
        .iter()
        .map(|(column, value)| (*column, value.as_slice()))
        .collect()
}
/// Each clone's name and email alone, and both `closed_by` texts.
fn identity_probes() -> Vec<Vec<u8>> {
    [
        NAME.to_owned(),
        EMAIL.to_owned(),
        PEER_NAME.to_owned(),
        PEER_EMAIL.to_owned(),
        closed_by(NAME, EMAIL),
        closed_by(PEER_NAME, PEER_EMAIL),
    ]
    .into_iter()
    .map(String::into_bytes)
    .collect()
}
/// A `closed_by` cell holds the name and email it is made of: the cache
/// scan, whose exemption is the exact `closed_by` value, omits those parts.
/// Workflow rows are always scanned with them.
fn cache_probes(probes: &[Vec<u8>]) -> Vec<Vec<u8>> {
    let parts = [NAME, EMAIL, PEER_NAME, PEER_EMAIL];
    probes
        .iter()
        .filter(|p| !parts.iter().any(|part| p.as_slice() == part.as_bytes()))
        .cloned()
        .collect()
}

fn ticket_target() -> SynchronizationTarget {
    SynchronizationTarget::Context {
        kind: AuthoringKind::Ticket,
        item_id: item(),
    }
}
fn draft(body: &str) -> TicketDraft {
    TicketDraft {
        title: TITLE.into(),
        ticket_type: "task".into(),
        status: "open".into(),
        project: None,
        team: None,
        body: body.into(),
    }
}
/// The canonical ticket, optionally carrying `(closed_at, closed_by)`.
fn ticket_source(body: &str, closure: Option<(i64, &str)>) -> String {
    canonical::serialize_item(&canonical::CanonicalItem::Ticket(canonical::Ticket {
        id: item(),
        title: TITLE.into(),
        ticket_type: "task".into(),
        status: "open".into(),
        project: None,
        team: None,
        closed_at: closure.map(|(at, _)| time::OffsetDateTime::from_unix_timestamp(at).unwrap()),
        closed_by: closure.map(|(_, by)| by.to_owned()),
        body: body.into(),
        unknown: serde_yaml::Mapping::new(),
    }))
    .unwrap()
}
fn identity(repo: &git2::Repository, name: &str, email: &str) -> Result<(), FixtureError> {
    let mut config = fixed(repo.config())?;
    fixed(config.set_str("core.autocrlf", "false"))?;
    fixed(config.set_str("user.name", name))?;
    fixed(config.set_str("user.email", email))
}
fn select_key(service: &RepositoryService, server: &SshRemoteFixture) -> Result<(), FixtureError> {
    let RegisterSharedKeyOutcome::Registered(key) =
        fixed(service.register_shared_key(RegisterSharedKeyRequest {
            label: "fixture".into(),
            ownership: SharedKeyOwnership::Imported,
            private_key_path: server.client_key_path().into(),
            public_key_path: None,
        }))?
    else {
        return Err(FixtureError);
    };
    fixed(service.select_shared_key(key.id))?;
    Ok(())
}
/// A fixture commit on `repo`'s checked-out branch (the user's own Git).
fn commit_files(
    repo: &git2::Repository,
    changes: &[(&str, &[u8])],
) -> Result<git2::Oid, FixtureError> {
    for (path, bytes) in changes {
        let path = repo.workdir().ok_or(FixtureError)?.join(path);
        fixed(std::fs::create_dir_all(path.parent().ok_or(FixtureError)?))?;
        fixed(std::fs::write(path, bytes))?;
    }
    let mut index = fixed(repo.index())?;
    for (path, _) in changes {
        fixed(index.add_path(Path::new(path)))?;
    }
    fixed(index.write())?;
    let tree = fixed(repo.find_tree(fixed(index.write_tree())?))?;
    let parent = fixed(fixed(repo.head())?.peel_to_commit())?;
    let sig = fixed(git2::Signature::now("Fixture", "fixture@example.invalid"))?;
    fixed(repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        "Fixture checkpoint",
        &tree,
        &[&parent],
    ))
}
fn authoritative(result: TicketClosureResult) -> TicketClosureOutcome {
    match result {
        TicketClosureResult::Complete(o) => o,
        TicketClosureResult::IndexPending(p) => p.authoritative,
    }
}
/// Fixed categories only: never a result's `Debug`, paths or private data.
fn category(error: &TicketClosureError) -> u128 {
    match error {
        TicketClosureError::Repository(_) => 10,
        TicketClosureError::Transport(_) => 11,
        TicketClosureError::Blocked(_) => 12,
        TicketClosureError::ExternalChange => 13,
        TicketClosureError::ReconfirmationRequired => 14,
        TicketClosureError::PrimaryAdvanced => 15,
        TicketClosureError::IdentityRequired { .. } => 16,
        TicketClosureError::ContextSynchronization(_) => 17,
        TicketClosureError::Busy => 18,
        TicketClosureError::PollYielding => 19,
        TicketClosureError::Interrupted => 20,
        TicketClosureError::OperationMismatch => 21,
        TicketClosureError::RecoveryRequired => 22,
    }
}

fn oid(side: &Side, reference: &str) -> Result<git2::Oid, FixtureError> {
    fixed(side.repo()?.refname_to_id(reference))
}
fn server_oid(p: &Pair, reference: &str) -> Result<git2::Oid, FixtureError> {
    fixed(fixed(git2::Repository::open_bare(p.server.repository_path()))?.refname_to_id(reference))
}
fn linked(side: &Side) -> Result<git2::Repository, FixtureError> {
    fixed(git2::Repository::open(side.worktree()))
}
fn count(side: &Side, table: &str) -> Result<i64, FixtureError> {
    fixed(
        side.db()?
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0)),
    )
}
fn step(side: &Side, id: OperationId) -> Result<String, FixtureError> {
    fixed(side.db()?.query_row(
        "SELECT step FROM ticket_closures WHERE operation_id=?1",
        [id.to_string()],
        |r| r.get(0),
    ))
}
/// The stored `(closed_at, closure_oid)` of a closure record.
fn stored_closure(
    side: &Side,
    id: OperationId,
) -> Result<(Option<String>, Option<String>), FixtureError> {
    fixed(side.db()?.query_row(
        "SELECT closed_at,closure_oid FROM ticket_closures WHERE operation_id=?1",
        [id.to_string()],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ))
}
/// Commits with `subject` anywhere in `side`'s object database.
fn commits(side: &Side, subject: &str) -> Result<usize, FixtureError> {
    let repo = side.repo()?;
    let odb = fixed(repo.odb())?;
    let mut n = 0;
    fixed(odb.foreach(|id| {
        if repo
            .find_commit(*id)
            .is_ok_and(|c| c.summary() == Some(subject))
        {
            n += 1;
        }
        true
    }))?;
    Ok(n)
}
/// Commits with `subject` reachable from `tip`.
fn reachable(side: &Side, tip: git2::Oid, subject: &str) -> Result<Vec<git2::Oid>, FixtureError> {
    let repo = side.repo()?;
    let mut walk = fixed(repo.revwalk())?;
    fixed(walk.push(tip))?;
    let mut found = Vec::new();
    for id in walk {
        let id = fixed(id)?;
        if fixed(repo.find_commit(id))?.summary() == Some(subject) {
            found.push(id);
        }
    }
    Ok(found)
}
fn blob(repo: &git2::Repository, id: git2::Oid, path: &str) -> Result<Vec<u8>, FixtureError> {
    let tree = fixed(fixed(repo.find_commit(id))?.tree())?;
    Ok(
        fixed(repo.find_blob(fixed(tree.get_path(Path::new(path)))?.id()))?
            .content()
            .to_vec(),
    )
}
/// The ticket as the commit `id` of `side` holds it.
fn ticket_at(side: &Side, id: git2::Oid) -> Result<canonical::Ticket, FixtureError> {
    let bytes = blob(&side.repo()?, id, TICKET_PATH)?;
    match fixed(canonical::parse_item(
        Path::new(TICKET_PATH),
        fixed(std::str::from_utf8(&bytes))?,
    ))? {
        canonical::CanonicalItem::Ticket(t) => Ok(t),
        _ => Err(FixtureError),
    }
}
/// `[context pushes, primary pushes, context deletions]` after `cursor`,
/// every one accepted.
fn receives(p: &Pair, cursor: usize) -> Result<[usize; 3], FixtureError> {
    p.server.wait_for_helper_audits()?;
    let mut counts = [0; 3];
    for u in &p.server.receive_updates()[cursor..] {
        assert!(u.accepted);
        if u.reference == MAIN {
            counts[1] += 1;
        } else if u.reference == CONTEXT {
            counts[if u.new_oid.is_zero() { 2 } else { 0 }] += 1;
        } else {
            return Err(FixtureError);
        }
    }
    Ok(counts)
}
/// C1: no context push after `cursor` ever carried `checkpoint` or a
/// descendant of it.
fn context_never_held(p: &Pair, cursor: usize, checkpoint: git2::Oid) -> Result<(), FixtureError> {
    p.server.wait_for_helper_audits()?;
    let repo = p.a.repo()?;
    for u in &p.server.receive_updates()[cursor..] {
        if u.reference == CONTEXT && !u.new_oid.is_zero() {
            assert_ne!(u.new_oid, checkpoint);
            assert!(!fixed(repo.graph_descendant_of(u.new_oid, checkpoint))?);
        }
    }
    Ok(())
}
fn poll(p: &Pair, side: &Side) -> Result<RemotePollReport, FixtureError> {
    let result = side.service.poll_publication_remote(
        PollPublicationRemoteRequest {
            root: side.root.clone(),
            operation_id: OperationId::new(),
            invocation: RemotePollInvocation::Explicit,
            approval: Some(p.approval()),
            restart: false,
        },
        &mut SessionCredentials::new(remote_world::Provider),
    );
    p.diagnostics(&format!("{result:?}"))?;
    let result = match fixed(result)? {
        RemotePollResult::Complete(r) => r,
        RemotePollResult::IndexPending(i) => i.authoritative,
    };
    match result {
        RemotePollOutcome::Polled(r) => Ok(r),
        _ => Err(FixtureError),
    }
}
/// The poll's disposition for the ticket context, if it reported one.
fn ticket_disposition(report: &RemotePollReport) -> Option<PollDisposition> {
    report
        .targets()
        .iter()
        .find(|r| r.item_id().is_some_and(|id| id.to_string() == TICKET))
        .map(|r| r.disposition())
}
fn peer_fetch(
    p: &Pair,
    side: &Side,
    reference: &str,
    tracking: &str,
) -> Result<git2::Oid, FixtureError> {
    let repo = side.repo()?;
    let mut remote = fixed(repo.remote_anonymous(&p.server.url()))?;
    let mut options = git2::FetchOptions::new();
    options.remote_callbacks(remote_world::callbacks(&p.server));
    fixed(remote.fetch(
        &[&format!("+{reference}:{tracking}")],
        Some(&mut options),
        None,
    ))?;
    fixed(repo.refname_to_id(tracking))
}
/// Another clone deletes the remote context with its own Git.
fn peer_delete(p: &Pair) -> Result<(), FixtureError> {
    let old = server_oid(p, CONTEXT)?;
    let before = p.server.receive_updates().len();
    let repo = p.b.repo()?;
    let mut remote = fixed(repo.remote_anonymous(&p.server.url()))?;
    let mut options = git2::PushOptions::new();
    options.remote_callbacks(remote_world::callbacks(&p.server));
    fixed(remote.push(&[&format!(":{CONTEXT}")], Some(&mut options)))?;
    p.server.fixture_push_transport_returned()?;
    p.server.wait_for_receive_update(
        before,
        &ssh_remote::ReceiveUpdate {
            reference: CONTEXT.into(),
            old_oid: old,
            new_oid: git2::Oid::zero(),
            accepted: true,
        },
    )
}
/// The established public corrupt-cache path: the registry is replaced and
/// rebuilt, and publication history becomes unknown.
fn lose_registry(server: &SshRemoteFixture, side: &mut Side) -> Result<(), FixtureError> {
    for entry in fixed(std::fs::read_dir(&side.data))? {
        let path = fixed(entry)?.path();
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with(REGISTRY_FILE))
        {
            fixed(std::fs::remove_file(path))?;
        }
    }
    fixed(std::fs::write(
        side.data.join(REGISTRY_FILE),
        b"lost registry",
    ))?;
    side.service = fixed(RepositoryService::open_at(&side.data))?;
    fixed(side.service.rebuild_repository(RebuildRepositoryRequest {
        root: side.root.clone(),
        operation_id: OperationId::new(),
    }))?;
    select_key(&side.service, server)?;
    Ok(())
}
/// Resolve every conflicted path of `child` to its local side.
fn resolve_local(p: &Pair, side: &Side, child: OperationId) -> Result<git2::Oid, FixtureError> {
    let inspection = fixed(
        side.service
            .inspect_synchronization_recovery(&side.root, child),
    )?;
    p.diagnostics(&format!("{inspection:?}"))?;
    let mut results = Vec::new();
    for path in &inspection.paths {
        let sides = fixed(side.service.read_synchronization_conflict(&path.token))?;
        p.diagnostics(&format!("{sides:?}"))?;
        results.push((path.token.clone(), sides.local.ok_or(FixtureError)?));
    }
    let request = ResolveSynchronizationRequest::new(
        side.root.clone(),
        child,
        OperationId::new(),
        inspection.observation,
        results,
        None,
    );
    let result = fixed(side.service.resolve_synchronization(request))?;
    p.diagnostics(&format!("{result:?}"))?;
    match result {
        ResolveSynchronizationOutcome::LocalCheckpointComplete { commit_oid } => Ok(commit_oid),
        _ => Err(FixtureError),
    }
}
/// The context child ID of a closure.
fn context_child(side: &Side, id: OperationId) -> Result<(Option<String>, i64), FixtureError> {
    fixed(side.db()?.query_row(
        "SELECT context_sync_id,context_attempt FROM ticket_closures WHERE operation_id=?1",
        [id.to_string()],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ))
}
fn primary_child(side: &Side, id: OperationId) -> Result<(Option<String>, i64), FixtureError> {
    fixed(side.db()?.query_row(
        "SELECT primary_sync_id,primary_attempt FROM ticket_closures WHERE operation_id=?1",
        [id.to_string()],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ))
}

/// All directory entries and file bytes under `root`, read through one
/// canonical root. The repository operation lock under `.git` is recorded
/// by name only: a native lease holds it with a mandatory lock.
fn image(root: &Path) -> Result<[u8; 32], FixtureError> {
    fn frame(hash: &mut blake3::Hasher, bytes: &[u8]) -> Result<(), FixtureError> {
        hash.update(&fixed(u64::try_from(bytes.len()))?.to_be_bytes());
        hash.update(bytes);
        Ok(())
    }
    fn add(path: &Path, hash: &mut blake3::Hasher) -> Result<(), FixtureError> {
        let mut paths = fixed(std::fs::read_dir(path))?
            .map(|e| fixed(e).map(|e| e.path()))
            .collect::<Result<Vec<_>, _>>()?;
        paths.sort();
        hash.update(&fixed(u64::try_from(paths.len()))?.to_be_bytes());
        for path in paths {
            let name = path.file_name().ok_or(FixtureError)?;
            frame(hash, name.as_encoded_bytes())?;
            let meta = fixed(std::fs::symlink_metadata(&path))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                hash.update(&(meta.permissions().mode() & 0o7777).to_be_bytes());
            }
            if meta.file_type().is_symlink() {
                hash.update(b"l");
                frame(
                    hash,
                    fixed(std::fs::read_link(&path))?
                        .as_os_str()
                        .as_encoded_bytes(),
                )?;
            } else if meta.is_dir() {
                hash.update(b"d");
                add(&path, hash)?;
            } else if meta.is_file() {
                hash.update(b"f");
                if name == "manyhands-operation.lock" {
                    continue;
                }
                frame(hash, &fixed(std::fs::read(&path))?)?;
            } else {
                return Err(FixtureError);
            }
        }
        Ok(())
    }
    let root = fixed(std::fs::canonicalize(root))?;
    let mut hash = blake3::Hasher::new();
    hash.update(b"manyhands closure fixture image v1");
    add(&root, &mut hash)?;
    Ok(*hash.finalize().as_bytes())
}
fn all_images(p: &Pair) -> Result<[[u8; 32]; 3], FixtureError> {
    Ok([
        image(&p.a.root)?,
        image(&p.b.root)?,
        image(p.server.repository_path())?,
    ])
}
/// Run `request` with a service that stops at `point`; nothing complete.
fn stopped(
    p: &Pair,
    side: &Side,
    request: &CloseTicketRequest,
    point: FailurePoint,
) -> Result<(), FixtureError> {
    let service = fixed(RepositoryService::open_at_with_failure_point_for_testing(
        &side.data, point,
    ))?;
    let result = p.execute(&service, request.clone());
    assert!(
        result.is_err()
            || matches!(result, Ok(ref r) if authoritative(r.clone()).cleanup() != PromotionCleanup::Complete)
    );
    Ok(())
}
/// A commit on `repo`'s checked-out branch that removes `path`.
fn commit_removal(repo: &git2::Repository, path: &str) -> Result<git2::Oid, FixtureError> {
    fixed(std::fs::remove_file(
        repo.workdir().ok_or(FixtureError)?.join(path),
    ))?;
    let mut index = fixed(repo.index())?;
    fixed(index.remove_path(Path::new(path)))?;
    fixed(index.write())?;
    let tree = fixed(repo.find_tree(fixed(index.write_tree())?))?;
    let parent = fixed(fixed(repo.head())?.peel_to_commit())?;
    let sig = fixed(git2::Signature::now("Fixture", "fixture@example.invalid"))?;
    fixed(repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        "Fixture removal",
        &tree,
        &[&parent],
    ))
}
/// A fixed number per synchronization error variant, for observations.
fn sync_category(error: &SynchronizationError) -> u128 {
    match error {
        SynchronizationError::Repository(_) => 1,
        SynchronizationError::Transport(_) => 2,
        SynchronizationError::Busy => 3,
        SynchronizationError::PollYielding => 4,
        SynchronizationError::Interrupted => 5,
        SynchronizationError::TargetNotMaterialized => 6,
        SynchronizationError::WorktreeNotClean { .. } => 7,
        SynchronizationError::WorktreeConflicted { .. } => 8,
        SynchronizationError::PrimaryMissing => 9,
        SynchronizationError::RemoteContextDeleted => 10,
        SynchronizationError::HistoryUnknown => 11,
        SynchronizationError::ConflictPending { .. } => 12,
        SynchronizationError::IdentityRequired { .. } => 13,
        SynchronizationError::ExternalResolutionRequired { .. } => 14,
        SynchronizationError::MergeRequired { .. } => 15,
        SynchronizationError::PushRejected => 16,
        SynchronizationError::ExternalChange => 17,
        SynchronizationError::RecoveryRequired => 18,
        #[allow(unreachable_patterns)]
        _ => 99,
    }
}
