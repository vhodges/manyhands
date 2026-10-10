//! Public compound API acceptance against an isolated authenticated SSH remote.
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
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

const DOC: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
const TICKET: &str = "01BX5ZZKBKACTAV9WEVGEMMVRZ";
const PASSWORD: &str = "cp-private-passphrase-canary";
const BODY: &str = "cp-private-comment-body-canary\n";

fn main() {
    // SAFETY: this real main initializes libgit2 before any fixture thread.
    if unsafe { ssh_harness::initialize() }.is_err() {
        eprintln!("comment publication initialization failed");
        std::process::exit(1);
    }
    ssh_harness::run_with_output_privacy(CASES, &[]);
}
const CASES: &[ssh_harness::Case] = &[
    ("document_root_and_nested_replies", || {
        published_thread(AuthoringKind::Document)
    }),
    ("ticket_root_and_nested_replies", || {
        published_thread(AuthoringKind::Ticket)
    }),
    ("no_remote_then_body_free_publication", local_then_remote),
    ("dirty_context_retains_saved_receipt", dirty_context),
    (
        "checkpoint_and_discovery_precede_unleased_prompt",
        ordered_prompt,
    ),
    ("other_context_busy_after_checkpoint", other_context_busy),
    (
        "published_remote_index_pending_retry_is_discovery_only",
        remote_index_pending,
    ),
    (
        "local_only_dirty_context_has_zero_transport",
        local_only_dirty,
    ),
];

struct Provider {
    prompts: Arc<AtomicUsize>,
    action: Option<Box<dyn FnOnce() + Send>>,
}
impl Provider {
    fn new(prompts: Arc<AtomicUsize>) -> Self {
        Self {
            prompts,
            action: None,
        }
    }
}
impl SessionCredentialProvider for Provider {
    fn request_passphrase(&mut self, _: &UnlockRequest) -> PassphraseResponse {
        self.prompts.fetch_add(1, Ordering::SeqCst);
        if let Some(action) = self.action.take() {
            action();
        }
        PassphraseResponse::Supplied(SecretPassphrase::new(PASSWORD.into()).unwrap())
    }
}
fn callbacks(server: &SshRemoteFixture) -> git2::RemoteCallbacks<'_> {
    let mut callbacks = git2::RemoteCallbacks::new();
    callbacks.credentials(|_, user, _| {
        git2::Cred::ssh_key(
            user.unwrap_or("fixture"),
            None,
            server.client_key_path(),
            Some(PASSWORD),
        )
    });
    let public = server.host_public_key().to_bytes().unwrap();
    callbacks.certificate_check(move |cert, _| {
        if cert.as_hostkey().and_then(|host| host.hostkey()) == Some(public.as_slice()) {
            Ok(git2::CertificateCheckStatus::CertificateOk)
        } else {
            Err(git2::Error::from_str("fixture trust mismatch"))
        }
    });
    callbacks
}
fn clone_owned(server: &SshRemoteFixture, root: &Path) -> Result<git2::Repository, FixtureError> {
    let mut fetch = git2::FetchOptions::new();
    fetch.remote_callbacks(callbacks(server));
    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fetch);
    builder.remote_create(|repository, name, url| {
        repository.config()?.set_bool("core.autocrlf", false)?;
        repository.remote(name, url)
    });
    let repository = fixed(builder.clone(&server.url(), root))?;
    fixed(fixed(repository.config())?.set_str("user.name", "Publication fixture"))?;
    fixed(fixed(repository.config())?.set_str("user.email", "publication@example.invalid"))?;
    Ok(repository)
}
fn change(
    repo: &git2::Repository,
    parents: &[git2::Oid],
    changes: &[(&str, Option<&[u8]>)],
    subject: &str,
) -> Result<git2::Oid, FixtureError> {
    let parents = parents
        .iter()
        .map(|oid| fixed(repo.find_commit(*oid)))
        .collect::<Result<Vec<_>, _>>()?;
    let mut index = fixed(git2::Index::new())?;
    fixed(index.read_tree(&fixed(parents[0].tree())?))?;
    for (path, bytes) in changes {
        if let Some(bytes) = bytes {
            fixed(index.add(&git2::IndexEntry {
                ctime: git2::IndexTime::new(0, 0),
                mtime: git2::IndexTime::new(0, 0),
                dev: 0,
                ino: 0,
                mode: 0o100644,
                uid: 0,
                gid: 0,
                file_size: bytes.len() as u32,
                id: fixed(repo.blob(bytes))?,
                flags: 0,
                flags_extended: 0,
                path: path.as_bytes().to_vec(),
            }))?;
        } else {
            fixed(index.remove(Path::new(path), 0))?;
        }
    }
    let tree = fixed(repo.find_tree(fixed(index.write_tree_to(repo))?))?;
    let signature = fixed(repo.signature())?;
    fixed(repo.commit(
        None,
        &signature,
        &signature,
        subject,
        &tree,
        &parents.iter().collect::<Vec<_>>(),
    ))
}
fn install(repo: &git2::Repository, reference: &str, oid: git2::Oid) -> Result<(), FixtureError> {
    fixed(repo.checkout_tree(
        &fixed(repo.find_object(oid, None))?,
        Some(git2::build::CheckoutBuilder::new().safe()),
    ))?;
    fixed(repo.reference(reference, oid, true, "fixture advance"))?;
    Ok(())
}
fn push(
    server: &SshRemoteFixture,
    repo: &git2::Repository,
    reference: &str,
) -> Result<(), FixtureError> {
    let mut remote = fixed(repo.remote_anonymous(&server.url()))?;
    let mut options = git2::PushOptions::new();
    options.remote_callbacks(callbacks(server));
    fixed(remote.push(&[&format!("{reference}:{reference}")], Some(&mut options)))?;
    server.fixture_push_transport_returned()
}
struct Side {
    service: RepositoryService,
    root: PathBuf,
    data: PathBuf,
}
impl Side {
    fn repo(&self) -> Result<git2::Repository, FixtureError> {
        fixed(git2::Repository::open(&self.root))
    }
    fn context(&self, item: &canonical::ItemId) -> PathBuf {
        self.root
            .join(".manyhands/worktrees")
            .join(item.to_string())
    }
    fn db(&self) -> Result<rusqlite::Connection, FixtureError> {
        fixed(rusqlite::Connection::open(self.data.join(REGISTRY_FILE)))
    }
}
struct Pair {
    server: SshRemoteFixture,
    a: Side,
    b: Side,
    _directory: tempfile::TempDir,
    probes: Vec<Vec<u8>>,
}
impl Pair {
    fn new(remote: bool) -> Result<Self, FixtureError> {
        ssh_privacy::save(&[PASSWORD.as_bytes().to_vec(), BODY.as_bytes().to_vec()])?;
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
                PASSWORD,
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
            PASSWORD.as_bytes().to_vec(),
            BODY.as_bytes().to_vec(),
            server.url().into_bytes(),
        ];
        probes.extend(
            plain
                .split(|byte| *byte == b'\n')
                .filter(|line| line.len() > 40)
                .map(<[u8]>::to_vec),
        );
        ssh_privacy::save(&probes)?;
        let directory = fixed(tempfile::Builder::new().prefix("cp").tempdir())?;
        let root = directory.path().join("a");
        let repository = clone_owned(&server, &root)?;
        // Seed a complete canonical born repository BEFORE service operations.
        // Fixture checkout is not a refresh after an authoring save.
        let config = fixed(canonical::serialize_repository_config(
            &canonical::RepositoryConfig {
                primary_branch: "main".into(),
                publication_remote: remote.then(|| "origin".into()),
                unknown: toml::Table::new(),
            },
        ))?;
        let doc = fixed(canonical::serialize_item(
            &canonical::CanonicalItem::Document(canonical::Document {
                id: fixed(DOC.parse())?,
                title: "Shared document".into(),
                body: (1..=12)
                    .map(|n| format!("shared line {n}\n"))
                    .collect::<String>()
                    + "tail\n",
                unknown: serde_yaml::Mapping::new(),
            }),
        ))?;
        let ticket = fixed(canonical::serialize_item(
            &canonical::CanonicalItem::Ticket(canonical::Ticket {
                id: fixed(TICKET.parse())?,
                title: "Shared ticket".into(),
                ticket_type: "task".into(),
                status: "open".into(),
                project: None,
                team: None,
                closed_at: None,
                closed_by: None,
                body: "ticket body\n".into(),
                unknown: serde_yaml::Mapping::new(),
            }),
        ))?;
        let ticket_path = format!(".manyhands/tickets/{TICKET}/ticket.md");
        let head = fixed(repository.refname_to_id("refs/heads/main"))?;
        let seed = change(
            &repository,
            &[head],
            &[
                (".manyhands/config.toml", Some(config.as_bytes())),
                ("docs/a.md", Some(doc.as_bytes())),
                (&ticket_path, Some(ticket.as_bytes())),
            ],
            "Fixture seed",
        )?;
        install(&repository, "refs/heads/main", seed)?;
        push(&server, &repository, "refs/heads/main")?;
        let other = directory.path().join("b");
        clone_owned(&server, &other)?;
        let side = |root: PathBuf, name: &str| -> Result<Side, FixtureError> {
            let data = directory.path().join(name);
            let service = fixed(RepositoryService::open_at(&data))?;
            fixed(service.enable(EnableRepositoryRequest {
                root: root.clone(),
                primary_branch: "main".into(),
                identity: None,
                operation_id: OperationId::new(),
            }))?;
            let RegisterSharedKeyOutcome::Registered(key) =
                fixed(service.register_shared_key(RegisterSharedKeyRequest {
                    label: "fixture selected".into(),
                    ownership: SharedKeyOwnership::Imported,
                    private_key_path: server.client_key_path().into(),
                    public_key_path: None,
                }))?
            else {
                return Err(FixtureError);
            };
            fixed(service.select_shared_key(key.id))?;
            Ok(Side {
                service,
                root,
                data,
            })
        };
        let a = side(root, "da")?;
        let b = side(other, "db")?;
        let pair = Self {
            server,
            a,
            b,
            _directory: directory,
            probes,
        };
        pair.privacy()?;
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
    fn request(
        &self,
        side: &Side,
        kind: AuthoringKind,
        parent_id: Option<canonical::ItemId>,
    ) -> PublishCommentRequest {
        PublishCommentRequest {
            comment: SubmitCommentRequest {
                target: AuthoringTarget {
                    root: side.root.clone(),
                    kind,
                    item_id: (if kind == AuthoringKind::Document {
                        DOC
                    } else {
                        TICKET
                    })
                    .parse()
                    .unwrap(),
                    intent: ContextIntent::Edit,
                    operation_id: OperationId::new(),
                },
                comment_id: canonical::ItemId::generate(),
                parent_id,
                body: BODY.into(),
                expected_destination: ExpectedPathObservation::Missing,
            },
            approval: Some(self.approval()),
            confirmed_identity: None,
        }
    }
    fn privacy(&self) -> Result<(), FixtureError> {
        ssh_privacy::save(&self.probes)?;
        for side in [&self.a, &self.b] {
            ssh_privacy::scan(&side.data, &self.probes)?;
        }
        Ok(())
    }
    fn proof(&self, receipt: &CommentReceipt, oid: git2::Oid) -> Result<(), FixtureError> {
        let remote = fixed(git2::Repository::open_bare(self.server.repository_path()))?;
        assert_eq!(
            fixed(remote.refname_to_id(&format!("refs/heads/{}", receipt.context_branch)))?,
            oid
        );
        assert!(
            oid == receipt.checkpoint_oid
                || fixed(remote.graph_descendant_of(oid, receipt.checkpoint_oid))?
        );
        let local = fixed(git2::Repository::open(&receipt.root))?;
        let original = fixed(
            fixed(fixed(local.find_commit(receipt.checkpoint_oid))?.tree())?
                .get_path(&receipt.comment_path),
        )?;
        let published =
            fixed(fixed(fixed(remote.find_commit(oid))?.tree())?.get_path(&receipt.comment_path))?;
        assert_eq!(original.id(), published.id());
        assert_eq!(original.filemode(), published.filemode());
        Ok(())
    }
}
fn saved(
    value: CommentSubmissionOutcome,
) -> Result<
    (
        CommentReceipt,
        CommentPublicationState,
        CommentIndexingState,
    ),
    FixtureError,
> {
    match value {
        CommentSubmissionOutcome::Saved {
            receipt,
            publication,
            indexing,
            ..
        } => Ok((*receipt, publication, indexing)),
        _ => Err(FixtureError),
    }
}
fn published(value: CommentPublicationState) -> Result<git2::Oid, FixtureError> {
    match value {
        CommentPublicationState::Published { oid }
        | CommentPublicationState::AlreadyCurrent { oid } => Ok(oid),
        other => {
            ssh_harness::observation(&[match other {
                CommentPublicationState::Pending { .. } => 1,
                _ => 2,
            }]);
            Err(FixtureError)
        }
    }
}
fn published_thread(kind: AuthoringKind) -> Result<(), FixtureError> {
    let pair = Pair::new(true)?;
    let main = fixed(pair.a.repo()?.refname_to_id("refs/heads/main"))?;
    let prompts = Arc::new(AtomicUsize::new(0));
    let mut session = SessionCredentials::new(Provider::new(prompts.clone()));
    let mut parent = None;
    for _ in 0..3 {
        let request = pair.request(&pair.a, kind, parent.clone());
        let (receipt, state, index) =
            saved(fixed(pair.a.service.submit_comment(request, &mut session))?)?;
        let oid = published(state)?;
        pair.proof(&receipt, oid)?;
        assert!(!index.local_pending && !index.remote_pending);
        parent = Some(receipt.comment_id.clone());
        let calls = pair.server.helper_invocations();
        let (again, state, _) = saved(fixed(pair.a.service.retry_comment_publication(
            RetryCommentPublicationRequest {
                root: pair.a.root.clone(),
                operation_id: receipt.operation_id,
                approval: None,
                confirmed_identity: None,
                restart: false,
            },
            &mut session,
        ))?)?;
        assert_eq!(again.checkpoint_oid, receipt.checkpoint_oid);
        assert_eq!(again.synchronization_id, receipt.synchronization_id);
        assert_eq!(published(state)?, oid);
        assert_eq!(pair.server.helper_invocations(), calls);
    }
    assert_eq!(
        fixed(pair.a.repo()?.refname_to_id("refs/heads/main"))?,
        main
    );
    assert_eq!(prompts.load(Ordering::SeqCst), 1);
    let item: canonical::ItemId = fixed(
        (if kind == AuthoringKind::Document {
            DOC
        } else {
            TICKET
        })
        .parse(),
    )?;
    let linked = fixed(git2::Repository::open(pair.a.context(&item)))?;
    let mut walk = fixed(linked.revwalk())?;
    fixed(walk.push_head())?;
    let mut checkpoints = 0;
    for oid in walk {
        if fixed(linked.find_commit(fixed(oid)?))?
            .summary()
            .is_some_and(|subject| subject.starts_with("Checkpoint comment "))
        {
            checkpoints += 1;
        }
    }
    assert_eq!(checkpoints, 3);
    let snapshot = fixed(pair.a.service.repository_snapshot(&pair.a.root))?;
    let found = snapshot
        .items
        .iter()
        .find(|entry| entry.id == item)
        .ok_or(FixtureError)?;
    assert_eq!(found.comments.len(), 1);
    assert_eq!(found.comments[0].replies.len(), 1);
    assert_eq!(found.comments[0].replies[0].replies.len(), 1);
    assert_eq!(
        fixed(pair.a.db()?.query_row(
            "SELECT COUNT(*) FROM comment_publication_bindings",
            [],
            |r| r.get::<_, i64>(0)
        ))?,
        3
    );
    pair.privacy()
}
fn local_then_remote() -> Result<(), FixtureError> {
    let pair = Pair::new(false)?;
    let prompts = Arc::new(AtomicUsize::new(0));
    let mut session = SessionCredentials::new(Provider::new(prompts.clone()));
    let before = pair.server.helper_invocations();
    let (receipt, state, index) = saved(fixed(pair.a.service.submit_comment(
        pair.request(&pair.a, AuthoringKind::Document, None),
        &mut session,
    ))?)?;
    assert!(matches!(
        state,
        CommentPublicationState::Pending {
            reason: CommentPublicationPendingReason::NoPublicationRemote
        }
    ));
    assert!(!index.local_pending);
    assert_eq!(prompts.load(Ordering::SeqCst), 0);
    assert_eq!(pair.server.helper_invocations(), before);
    assert_eq!(
        fixed(
            pair.a
                .db()?
                .query_row("SELECT COUNT(*) FROM remote_operation_records", [], |r| r
                    .get::<_, i64>(
                    0
                ))
        )?,
        0
    );
    fixed(
        pair.a
            .service
            .set_publication_remote(SetPublicationRemoteRequest {
                root: pair.a.root.clone(),
                name: Some("origin".into()),
                operation_id: OperationId::new(),
            }),
    )?;
    let (again, state, _) = saved(fixed(pair.a.service.retry_comment_publication(
        RetryCommentPublicationRequest {
            root: pair.a.root.clone(),
            operation_id: receipt.operation_id,
            approval: Some(pair.approval()),
            confirmed_identity: None,
            restart: false,
        },
        &mut session,
    ))?)?;
    assert_eq!(again.checkpoint_oid, receipt.checkpoint_oid);
    assert_eq!(again.synchronization_id, receipt.synchronization_id);
    pair.proof(&again, published(state)?)?;
    pair.privacy()
}
fn dirty_context() -> Result<(), FixtureError> {
    let pair = Pair::new(true)?;
    let item = DOC.parse().unwrap();
    let context = fixed(pair.a.service.prepare_context(AuthoringTarget {
        root: pair.a.root.clone(),
        kind: AuthoringKind::Document,
        item_id: item,
        intent: ContextIntent::Edit,
        operation_id: OperationId::new(),
    }))?;
    let context = match context {
        ContextProvisionOutcome::Created(c) | ContextProvisionOutcome::Reused(c) => c,
        _ => return Err(FixtureError),
    };
    fixed(std::fs::write(
        context.worktree.join("unrelated"),
        b"unsaved unrelated file",
    ))?;
    let mut session = SessionCredentials::new(Provider::new(Arc::new(AtomicUsize::new(0))));
    let (receipt, state, _) = saved(fixed(pair.a.service.submit_comment(
        pair.request(&pair.a, AuthoringKind::Document, None),
        &mut session,
    ))?)?;
    assert!(
        matches!(state,CommentPublicationState::Pending {reason:CommentPublicationPendingReason::Synchronization(error)} if matches!(*error,SynchronizationError::WorktreeNotClean {..}))
    );
    assert!(context.worktree.join(&receipt.comment_path).is_file());
    assert_eq!(
        fixed(std::fs::read(context.worktree.join("unrelated")))?,
        b"unsaved unrelated file"
    );
    pair.privacy()
}

fn ordered_prompt() -> Result<(), FixtureError> {
    let pair = Pair::new(true)?;
    let request = pair.request(&pair.a, AuthoringKind::Document, None);
    let root = pair.a.root.clone();
    let data = pair.a.data.clone();
    let path = pair
        .a
        .context(&request.comment.target.item_id)
        .join(format!(
            ".manyhands/comments/{}/{}.md",
            request.comment.target.item_id, request.comment.comment_id
        ));
    let id = request.comment.target.operation_id.to_string();
    let comment = request.comment.comment_id.to_string();
    let mut provider = Provider::new(Arc::new(AtomicUsize::new(0)));
    provider.action = Some(Box::new(move || {
        assert!(path.is_file());
        let db = rusqlite::Connection::open(data.join(REGISTRY_FILE)).unwrap();
        assert_eq!(
            db.query_row(
                "SELECT state FROM operation_records WHERE operation_ulid=?1",
                [id],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "completed"
        );
        assert_eq!(
            db.query_row(
                "SELECT COUNT(*) FROM discovered_comments WHERE comment_id=?1",
                [comment],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
        let lease = RepositoryService::hold_lease_for_testing(
            &root,
            &data,
            repository::LeaseKind::Repository,
        )
        .unwrap();
        drop(lease);
        let lease = RepositoryService::hold_lease_for_testing(
            &root,
            &data,
            repository::LeaseKind::CacheWrite,
        )
        .unwrap();
        drop(lease);
    }));
    let (receipt, state, _) = saved(fixed(
        pair.a
            .service
            .submit_comment(request, &mut SessionCredentials::new(provider)),
    )?)?;
    pair.proof(&receipt, published(state)?)?;
    pair.privacy()
}

fn other_context_busy() -> Result<(), FixtureError> {
    let pair = Pair::new(true)?;
    fixed(
        pair.a
            .service
            .set_publication_remote(SetPublicationRemoteRequest {
                root: pair.a.root.clone(),
                name: Some("origin".into()),
                operation_id: OperationId::new(),
            }),
    )?;
    fixed(pair.a.service.synchronize_remote(
        SynchronizeRemoteRequest {
            root: pair.a.root.clone(),
            operation_id: OperationId::new(),
            target: SynchronizationTarget::Primary,
            approval: Some(pair.approval()),
            confirmed_identity: None,
            restart: false,
        },
        &mut SessionCredentials::new(Provider::new(Arc::new(AtomicUsize::new(0)))),
    ))?;
    let plan = fixed(RemoteRefPlan::from_configuration("origin", "main"))?;
    let other = fixed(RemoteOperationTarget::for_context(
        &plan,
        RemoteOperationAction::SynchronizeContext,
        AuthoringKind::Ticket,
        canonical::ItemId::generate(),
    ))?;
    fixed(
        pair.a
            .service
            .reserve_remote_operation(&pair.a.root, OperationId::new(), &other),
    )?;
    let prompts = Arc::new(AtomicUsize::new(0));
    let calls = pair.server.helper_invocations();
    let result = pair.a.service.submit_comment(
        pair.request(&pair.a, AuthoringKind::Document, None),
        &mut SessionCredentials::new(Provider::new(prompts.clone())),
    );
    if let Err(error) = &result {
        ssh_harness::observation(&[10 + error.kind as u128]);
    }
    let (receipt, state, index) = saved(fixed(result)?)?;
    if let CommentPublicationState::Pending {
        reason: CommentPublicationPendingReason::Synchronization(error),
    } = &state
    {
        ssh_harness::observation(&[match &**error {
            SynchronizationError::Busy => 100,
            SynchronizationError::RecoveryRequired => 101,
            SynchronizationError::Repository(_) => 102,
            SynchronizationError::Transport(_) => 103,
            SynchronizationError::WorktreeNotClean { .. } => 105,
            SynchronizationError::WorktreeConflicted { .. } => 106,
            SynchronizationError::TargetNotMaterialized => 107,
            SynchronizationError::PrimaryMissing => 108,
            SynchronizationError::Interrupted => 109,
            SynchronizationError::PollYielding => 110,
            SynchronizationError::RemoteContextDeleted => 111,
            SynchronizationError::HistoryUnknown => 112,
            SynchronizationError::MergeRequired { .. } => 113,
            SynchronizationError::PushRejected => 114,
            SynchronizationError::ExternalResolutionRequired { .. } => 115,
            SynchronizationError::ExternalChange => 116,
            SynchronizationError::ConflictPending { .. } => 117,
            SynchronizationError::IdentityRequired { .. } => 118,
        }]);
    }
    assert!(!index.local_pending);
    assert!(
        matches!(state,CommentPublicationState::Pending {reason:CommentPublicationPendingReason::Synchronization(error)} if matches!(*error,SynchronizationError::Busy))
    );
    assert_eq!(prompts.load(Ordering::SeqCst), 0);
    assert_eq!(pair.server.helper_invocations(), calls);
    assert!(
        pair.a
            .context(&receipt.item_id)
            .join(&receipt.comment_path)
            .is_file()
    );
    pair.privacy()
}

fn remote_index_pending() -> Result<(), FixtureError> {
    let pair = Pair::new(true)?;
    let data = pair.a.data.clone();
    let mut provider = Provider::new(Arc::new(AtomicUsize::new(0)));
    provider.action = Some(Box::new(move || {
        rusqlite::Connection::open(data.join(REGISTRY_FILE)).unwrap().execute_batch("CREATE TRIGGER stop_remote_discovery BEFORE INSERT ON discovered_comments BEGIN SELECT RAISE(ABORT,'fixed failure'); END;").unwrap();
    }));
    let mut session = SessionCredentials::new(provider);
    let (receipt, state, index) = saved(fixed(pair.a.service.submit_comment(
        pair.request(&pair.a, AuthoringKind::Document, None),
        &mut session,
    ))?)?;
    let oid = published(state)?;
    pair.proof(&receipt, oid)?;
    assert!(!index.local_pending && index.remote_pending);
    fixed(
        pair.a
            .db()?
            .execute_batch("DROP TRIGGER stop_remote_discovery;"),
    )?;
    let reopened = fixed(RepositoryService::open_at(&pair.a.data))?;
    let calls = pair.server.helper_invocations();
    let updates = pair.server.receive_updates().len();
    let (again, state, index) = saved(fixed(reopened.retry_comment_publication(
        RetryCommentPublicationRequest {
            root: pair.a.root.clone(),
            operation_id: receipt.operation_id,
            approval: None,
            confirmed_identity: None,
            restart: false,
        },
        &mut session,
    ))?)?;
    assert_eq!(published(state)?, oid);
    assert_eq!(again.checkpoint_oid, receipt.checkpoint_oid);
    assert!(!index.local_pending && !index.remote_pending);
    assert_eq!(pair.server.helper_invocations(), calls);
    assert_eq!(pair.server.receive_updates().len(), updates);
    pair.privacy()
}

fn local_only_dirty() -> Result<(), FixtureError> {
    let pair = Pair::new(false)?;
    let item = DOC.parse().unwrap();
    fixed(pair.a.service.prepare_context(AuthoringTarget {
        root: pair.a.root.clone(),
        kind: AuthoringKind::Document,
        item_id: item,
        intent: ContextIntent::Edit,
        operation_id: OperationId::new(),
    }))?;
    let path = pair.a.context(&DOC.parse().unwrap()).join("unrelated");
    fixed(std::fs::write(&path, b"uncheckpointed work"))?;
    let prompts = Arc::new(AtomicUsize::new(0));
    let calls = pair.server.helper_invocations();
    let (_, state, index) = saved(fixed(pair.a.service.submit_comment(
        pair.request(&pair.a, AuthoringKind::Document, None),
        &mut SessionCredentials::new(Provider::new(prompts.clone())),
    ))?)?;
    assert!(matches!(
        state,
        CommentPublicationState::Pending {
            reason: CommentPublicationPendingReason::NoPublicationRemote
        }
    ));
    assert!(!index.local_pending);
    assert_eq!(prompts.load(Ordering::SeqCst), 0);
    assert_eq!(pair.server.helper_invocations(), calls);
    assert_eq!(fixed(std::fs::read(path))?, b"uncheckpointed work");
    assert_eq!(
        fixed(
            pair.a
                .db()?
                .query_row("SELECT COUNT(*) FROM remote_operation_records", [], |r| r
                    .get::<_, i64>(
                    0
                ))
        )?,
        0
    );
    pair.privacy()
}
