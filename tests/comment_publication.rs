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
use rusqlite::OptionalExtension;
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
    ssh_harness::run_with_output_privacy(CASES, OUTPUT_CONTROLS);
}
const CASES: &[ssh_harness::Case] = &[
    (
        "local_discovery_failure_blocks_child_until_body_free_repair",
        local_discovery_failure,
    ),
    (
        "completed_local_handoff_stop_reuses_unused_child",
        local_handoff_stop,
    ),
    (
        "publication_recovery_wal_backups_and_diagnostics_are_private",
        privacy_wal_backup,
    ),
    (
        "unrelated_worktree_and_index_preservation_matrix",
        preservation_matrix,
    ),
    (
        "durable_pre_effect_stops_never_guess_replacement",
        pre_effect_stops,
    ),
    (
        "compound_request_confirmation_only_authors_child_merge",
        compound_confirmation,
    ),
    (
        "real_ssh_missing_or_mismatched_binding_never_resubmits",
        binding_negatives,
    ),
    ("capture_privacy_controls_fail_closed", capture_controls),
    (
        "post_checkpoint_identity_confirmation_completes_merge",
        identity_confirmation,
    ),
    (
        "distinct_push_endpoint_proves_comment_publication",
        distinct_push,
    ),
    (
        "observed_remote_context_deletion_is_saved_pending",
        remote_deleted,
    ),
    ("changed_endpoint_fences_body_free_restart", endpoint_fenced),
    ("changed_host_requires_exact_reapproval", changed_host),
    (
        "wrong_selected_key_preserves_checkpoint",
        wrong_selected_key,
    ),
    ("real_ssh_local_handoff_faults_reuse_identity", local_faults),
    (
        "receiver_rejection_retries_without_second_comment",
        receiver_rejection,
    ),
    (
        "public_retry_rejects_committed_comment_replacement",
        excluded_comment,
    ),
    ("unlock_cancel_stays_saved", || authentication_failure(0)),
    ("unlock_unavailable_stays_saved", || {
        authentication_failure(1)
    }),
    ("wrong_passphrase_stays_saved", || authentication_failure(2)),
    ("unapproved_host_stays_saved", || authentication_failure(3)),
    ("server_key_rejection_stays_saved", || {
        authentication_failure(4)
    }),
    ("no_selected_key_stays_saved", || authentication_failure(5)),
    (
        "accepted_unacknowledged_push_is_observed_not_repeated",
        || ambiguous_push(false),
    ),
    (
        "accepted_push_verification_persistence_is_reconciled",
        || ambiguous_push(true),
    ),
    (
        "clean_two_clone_merge_publishes_prior_context_work",
        clean_divergence,
    ),
    (
        "collaborator_publication_relays_already_current",
        collaborator_current,
    ),
    (
        "external_two_parent_repair_publishes_original_comment",
        external_repair,
    ),
    (
        "retained_conflict_blocks_other_context_as_saved_busy",
        conflict_other_context,
    ),
    (
        "conflict_cancel_resolution_retries_original_comment",
        conflict_cancel_resolution,
    ),
    (
        "interrupted_child_requires_explicit_restart",
        interrupted_restart,
    ),
    (
        "terminal_cancel_uses_later_ordinary_context_sync",
        terminal_cancel,
    ),
    (
        "historical_replay_uses_original_blob_not_current_text",
        historical_replay,
    ),
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
    response: usize,
}
impl Provider {
    fn new(prompts: Arc<AtomicUsize>) -> Self {
        Self {
            prompts,
            action: None,
            response: 0,
        }
    }
}
impl SessionCredentialProvider for Provider {
    fn request_passphrase(&mut self, _: &UnlockRequest) -> PassphraseResponse {
        self.prompts.fetch_add(1, Ordering::SeqCst);
        if let Some(action) = self.action.take() {
            action();
        }
        match self.response {
            1 => PassphraseResponse::Cancelled,
            2 => PassphraseResponse::Unavailable,
            3 => PassphraseResponse::Supplied(
                SecretPassphrase::new("cp-wrong-passphrase-canary".into()).unwrap(),
            ),
            _ => PassphraseResponse::Supplied(SecretPassphrase::new(PASSWORD.into()).unwrap()),
        }
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

fn receiver_receipt(
    server: &SshRemoteFixture,
    receipt: &CommentReceipt,
    cursor: usize,
) -> Result<(), FixtureError> {
    server.wait_for_receive_update(
        cursor,
        &ReceiveUpdate {
            reference: format!("refs/heads/{}", receipt.context_branch),
            old_oid: git2::Oid::zero(),
            new_oid: receipt.checkpoint_oid,
            accepted: true,
        },
    )
}
fn remote_refs(
    server: &SshRemoteFixture,
) -> Result<Vec<(String, Option<git2::Oid>)>, FixtureError> {
    let repo = fixed(git2::Repository::open_bare(server.repository_path()))?;
    let mut refs = Vec::new();
    for reference in fixed(repo.references())? {
        let reference = fixed(reference)?;
        refs.push((
            reference.name().ok_or(FixtureError)?.to_owned(),
            reference.target(),
        ));
    }
    refs.sort();
    Ok(refs)
}
fn binding_evidence(
    side: &Side,
    receipt: &CommentReceipt,
) -> Result<(String, String, String), FixtureError> {
    fixed(side.db()?.query_row("SELECT synchronization_ulid,created_at,checkpoint_oid FROM comment_publication_bindings WHERE operation_ulid=?1", [receipt.operation_id.to_string()], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))))
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
        let probes = ssh_privacy::load(&PathBuf::from(
            std::env::var_os(ssh_privacy::PROBES).ok_or(FixtureError)?,
        ))?;
        for side in [&self.a, &self.b] {
            ssh_privacy::scan(&side.data, &probes)?;
        }
        Ok(())
    }
    fn diagnostics(&self, rendered: &str) -> Result<(), FixtureError> {
        let probes = ssh_privacy::load(&PathBuf::from(
            std::env::var_os(ssh_privacy::PROBES).ok_or(FixtureError)?,
        ))?;
        ssh_privacy::clean(rendered.as_bytes(), &probes)
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
                CommentPublicationState::Pending {
                    reason: CommentPublicationPendingReason::NoPublicationRemote,
                } => 3,
                CommentPublicationState::Pending {
                    reason: CommentPublicationPendingReason::LocalRecoveryRequired,
                } => 4,
                CommentPublicationState::Pending {
                    reason: CommentPublicationPendingReason::Synchronization(error),
                } => match *error {
                    SynchronizationError::HistoryUnknown => 112,
                    SynchronizationError::RecoveryRequired => 101,
                    SynchronizationError::ExternalChange => 116,
                    SynchronizationError::ConflictPending { .. } => 117,
                    SynchronizationError::PushRejected => 114,
                    SynchronizationError::Repository(error) => 200 + error.kind as u128,
                    SynchronizationError::Transport(error) => match error.kind {
                        SshTransportErrorKind::HostApprovalRequired { .. } => 151,
                        SshTransportErrorKind::HostReplacementRequired { .. } => 152,
                        SshTransportErrorKind::EndpointChanged => 153,
                        SshTransportErrorKind::HostTrustChanged => 161,
                        SshTransportErrorKind::RegistryUnavailable => 162,
                        SshTransportErrorKind::ProtocolFailure => 163,
                        SshTransportErrorKind::TransportUnavailable => 164,
                        SshTransportErrorKind::RemoteUnavailable => 165,
                        SshTransportErrorKind::SelectionChanged => 166,
                        SshTransportErrorKind::KeySourceChanged => 167,
                        SshTransportErrorKind::NoSelectedKey => 168,
                        SshTransportErrorKind::UnlockFailed => 169,
                        SshTransportErrorKind::KeyRejected => 170,
                        _ => 171,
                    },
                    _ => 199,
                },
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
        let (receipt, state, index) = saved(fixed(
            pair.a.service.submit_comment(request.clone(), &mut session),
        )?)?;
        assert_eq!(receipt.operation_id, request.comment.target.operation_id);
        assert_eq!(receipt.kind, kind);
        assert_eq!(receipt.item_id, request.comment.target.item_id);
        assert_eq!(receipt.comment_id, request.comment.comment_id);
        assert_eq!(receipt.parent_id, request.comment.parent_id);
        assert_eq!(receipt.root, fixed(pair.a.root.canonicalize())?);
        assert_eq!(
            receipt.comment_path,
            PathBuf::from(format!(
                ".manyhands/comments/{}/{}.md",
                receipt.item_id, receipt.comment_id
            ))
        );
        let source = fixed(std::fs::read_to_string(
            pair.a.context(&receipt.item_id).join(&receipt.comment_path),
        ))?;
        let canonical = match fixed(canonical::parse_item(&receipt.comment_path, &source))? {
            canonical::CanonicalItem::Comment(c) => c,
            _ => return Err(FixtureError),
        };
        assert_eq!(canonical.id, receipt.comment_id);
        assert_eq!(canonical.item_id, receipt.item_id);
        assert_eq!(canonical.parent_id, receipt.parent_id);
        assert_eq!(canonical.body, BODY);
        assert!(canonical.unknown.is_empty());
        let created = fixed(pair.a.db()?.query_row(
            "SELECT created_at FROM comment_publication_bindings WHERE operation_ulid=?1",
            [receipt.operation_id.to_string()],
            |r| r.get::<_, String>(0),
        ))?;
        assert_eq!(
            created,
            canonical.created_at.unix_timestamp_nanos().to_string()
        );
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
        assert_eq!(
            fixed(std::fs::read_to_string(
                pair.a.context(&receipt.item_id).join(&receipt.comment_path)
            ))?,
            source
        );
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
    let db = pair.a.db()?;
    assert_eq!(
        fixed(
            db.query_row("SELECT COUNT(*) FROM remote_operation_records", [], |r| r
                .get::<_, i64>(
                0
            ))
        )?,
        3
    );
    assert_eq!(fixed(db.query_row("SELECT COUNT(*) FROM comment_publication_bindings b JOIN remote_operation_records r ON r.operation_ulid=b.synchronization_ulid WHERE r.action='synchronize_context' AND r.item_id=b.item_ulid AND r.kind=b.kind", [], |r|r.get::<_,i64>(0)))?, 3);
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
    let snapshot = fixed(pair.a.service.repository_snapshot(&pair.a.root))?;
    assert!(
        snapshot.items.iter().any(|item| item.id == receipt.item_id
            && item
                .comments
                .iter()
                .any(|comment| comment.id == receipt.comment_id)),
        "saved local comment is not discoverable"
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
    let cursor = pair.server.receive_updates().len();
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
    receiver_receipt(&pair.server, &receipt, cursor)?;
    pair.server.wait_for_helper_audits()?;
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
    let head =
        fixed(fixed(git2::Repository::open(pair.a.context(&receipt.item_id)))?.head())?.target();
    let repository = fixed(git2::Repository::open(pair.a.context(&receipt.item_id)))?;
    let commits = commit_inventory(&repository)?;
    let journals = integration_counts(&pair.a)?;
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
    assert_eq!(
        fixed(fixed(git2::Repository::open(pair.a.context(&receipt.item_id)))?.head())?.target(),
        head
    );
    assert_eq!(commit_inventory(&repository)?, commits);
    assert_eq!(integration_counts(&pair.a)?, journals);
    assert!(
        fixed(reopened.repository_snapshot(&pair.a.root))?
            .items
            .iter()
            .any(|item| item.id == receipt.item_id
                && item
                    .comments
                    .iter()
                    .any(|comment| comment.id == receipt.comment_id))
    );
    pair.privacy()
}

fn retry_request(
    pair: &Pair,
    receipt: &CommentReceipt,
    restart: bool,
) -> RetryCommentPublicationRequest {
    RetryCommentPublicationRequest {
        root: pair.a.root.clone(),
        operation_id: receipt.operation_id,
        approval: Some(pair.approval()),
        confirmed_identity: None,
        restart,
    }
}

const OUTPUT_CONTROLS: &[ssh_harness::Case] = &[
    ("private_capture_leak_control", private_capture_leak_control),
    (
        "missing_probe_inventory_control",
        missing_probe_inventory_control,
    ),
];
fn private_capture_leak_control() -> Result<(), FixtureError> {
    ssh_privacy::save(&[b"synthetic-output-private-canary".to_vec()])?;
    println!("synthetic-output-private-canary");
    Ok(())
}
fn missing_probe_inventory_control() -> Result<(), FixtureError> {
    Ok(())
}
fn capture_controls() -> Result<(), FixtureError> {
    ssh_privacy::save(&[b"capture-control-parent-canary".to_vec()])?;
    assert_eq!(
        ssh_harness::run_isolated_with_output_privacy("private_capture_leak_control"),
        Err(ssh_harness::IsolationFailure::OutputPrivacy)
    );
    assert_eq!(
        ssh_harness::run_isolated_with_output_privacy("missing_probe_inventory_control"),
        Err(ssh_harness::IsolationFailure::ProbeInventory)
    );
    Ok(())
}

fn compound_confirmation() -> Result<(), FixtureError> {
    let pair = Pair::new(true)?;
    save_document_title(&pair.a, "Local title")?;
    save_document_body(
        &pair.b,
        (1..=12)
            .map(|n| format!("shared line {n}\n"))
            .collect::<String>()
            + "remote tail\n",
    )?;
    sync_side(&pair, &pair.b, AuthoringKind::Document, fixed(DOC.parse())?)?;
    let mut config = fixed(pair.a.repo()?.config())?;
    fixed(config.remove("user.name"))?;
    fixed(config.remove("user.email"))?;
    let mut probe_request = SynchronizeRemoteRequest {
        root: pair.a.root.clone(),
        operation_id: OperationId::new(),
        target: SynchronizationTarget::Context {
            kind: AuthoringKind::Document,
            item_id: fixed(DOC.parse())?,
        },
        approval: Some(pair.approval()),
        confirmed_identity: None,
        restart: false,
    };
    let probe = pair.a.service.synchronize_remote(
        probe_request.clone(),
        &mut SessionCredentials::new(Provider::new(Arc::new(AtomicUsize::new(0)))),
    );
    let expected = match probe {
        Err(SynchronizationError::IdentityRequired {
            expected_configuration,
            ..
        }) => expected_configuration,
        Err(error) => {
            let _ = published(CommentPublicationState::Pending {
                reason: CommentPublicationPendingReason::Synchronization(Box::new(error)),
            });
            return Err(FixtureError);
        }
        _ => return Err(FixtureError),
    };
    fixed(
        pair.a
            .service
            .cancel_remote_operation(&pair.a.root, probe_request.operation_id),
    )?;
    probe_request.restart = true;
    assert!(matches!(
        pair.a.service.synchronize_remote(
            probe_request,
            &mut SessionCredentials::new(Provider::new(Arc::new(AtomicUsize::new(0))))
        ),
        Err(SynchronizationError::Interrupted)
    ));
    fixed(config.set_str("user.name", "Publication fixture"))?;
    fixed(config.set_str("user.email", "publication@example.invalid"))?;
    let root = pair.a.root.clone();
    let mut provider = Provider::new(Arc::new(AtomicUsize::new(0)));
    provider.action = Some(Box::new(move || {
        let mut config = git2::Repository::open(root).unwrap().config().unwrap();
        config.remove("user.name").unwrap();
        config.remove("user.email").unwrap();
    }));
    let mut request = pair.request(&pair.a, AuthoringKind::Document, None);
    request.confirmed_identity = Some(ConfirmedCommitIdentity {
        confirmation_id: OperationId::new(),
        identity: CommitIdentity {
            name: "Confirmed compound merge".into(),
            email: "compound@example.invalid".into(),
        },
        expected_configuration: expected,
    });
    let (receipt, state, _) = saved(fixed(
        pair.a
            .service
            .submit_comment(request, &mut SessionCredentials::new(provider)),
    )?)?;
    let oid = published(state)?;
    assert_eq!(
        fixed(pair.a.repo()?.find_commit(receipt.checkpoint_oid))?
            .author()
            .name(),
        Some("Publication fixture")
    );
    assert_eq!(
        fixed(pair.a.repo()?.find_commit(oid))?.author().name(),
        Some("Confirmed compound merge")
    );
    pair.proof(&receipt, oid)?;
    pair.privacy()
}

fn binding_negatives() -> Result<(), FixtureError> {
    for corruption in 0..8 {
        let pair = Pair::new(true)?;
        let item: canonical::ItemId = fixed(DOC.parse())?;
        fixed(pair.a.service.prepare_context(AuthoringTarget {
            root: pair.a.root.clone(),
            kind: AuthoringKind::Document,
            item_id: item.clone(),
            intent: ContextIntent::Edit,
            operation_id: OperationId::new(),
        }))?;
        let dirty = pair.a.context(&item).join("unrelated");
        fixed(std::fs::write(dirty, b"unsaved"))?;
        let request = pair.request(&pair.a, AuthoringKind::Document, None);
        let mut session = SessionCredentials::new(Provider::new(Arc::new(AtomicUsize::new(0))));
        let (receipt, _, _) = saved(fixed(
            pair.a.service.submit_comment(request.clone(), &mut session),
        )?)?;
        let context = fixed(git2::Repository::open(pair.a.context(&item)))?;
        let index_before = fixed(std::fs::read(context.path().join("index")))?;
        let file_before = fixed(std::fs::read(
            pair.a.context(&item).join(&receipt.comment_path),
        ))?;
        let remote_before = remote_refs(&pair.server)?;
        let calls = pair.server.helper_invocations();
        if corruption == 0 || corruption >= 3 {
            let mut mismatch = request;
            match corruption {
                0 => mismatch.comment.parent_id = Some(canonical::ItemId::generate()),
                3 => mismatch.comment.target.item_id = canonical::ItemId::generate(),
                4 => mismatch.comment.target.kind = AuthoringKind::Ticket,
                5 => mismatch.comment.comment_id = canonical::ItemId::generate(),
                6 => mismatch.comment.target.operation_id = OperationId::new(),
                _ => mismatch.comment.target.root = pair.b.root.clone(),
            };
            let error = pair
                .a
                .service
                .submit_comment(mismatch, &mut session)
                .err()
                .ok_or(FixtureError)?;
            assert!(matches!(
                error.kind,
                RepositoryErrorKind::OperationMismatch | RepositoryErrorKind::RecoveryRequired
            ));
            pair.diagnostics(&format!("{error:?}\n{error}"))?;
        } else {
            if corruption == 1 {
                fixed(pair.a.db()?.execute(
                    "DELETE FROM comment_publication_bindings WHERE operation_ulid=?1",
                    [receipt.operation_id.to_string()],
                ))?;
            } else {
                fixed(pair.a.db()?.execute("UPDATE comment_publication_bindings SET checkpoint_oid='0000000000000000000000000000000000000001' WHERE operation_ulid=?1",[receipt.operation_id.to_string()]))?;
            }
            let error = pair
                .a
                .service
                .retry_comment_publication(retry_request(&pair, &receipt, false), &mut session)
                .err()
                .ok_or(FixtureError)?;
            assert_eq!(error.kind, RepositoryErrorKind::RecoveryRequired);
            pair.diagnostics(&format!("{error:?}\n{error}"))?;
        }
        assert_eq!(pair.server.helper_invocations(), calls);
        assert_eq!(
            fixed(std::fs::read(context.path().join("index")))?,
            index_before
        );
        assert_eq!(
            fixed(std::fs::read(
                pair.a.context(&item).join(&receipt.comment_path)
            ))?,
            file_before
        );
        assert_eq!(remote_refs(&pair.server)?, remote_before);
        assert_eq!(
            fixed(fixed(git2::Repository::open(pair.a.context(&item)))?.head())?.target(),
            Some(receipt.checkpoint_oid)
        );
        assert_eq!(
            checkpoint_objects(
                &fixed(git2::Repository::open(pair.a.context(&item)))?,
                &receipt.comment_id
            )?,
            vec![receipt.checkpoint_oid]
        );
        assert_eq!(
            fixed(pair.a.db()?.query_row(
                "SELECT COUNT(*) FROM remote_operation_records",
                [],
                |r| r.get::<_, i64>(0)
            ))?,
            0
        );
        assert_eq!(
            fixed(pair.a.db()?.query_row(
                "SELECT COUNT(*) FROM comment_publication_bindings",
                [],
                |r| r.get::<_, i64>(0)
            ))?,
            if corruption == 1 { 0 } else { 1 }
        );
        pair.privacy()?;
    }
    Ok(())
}

type IndexImage = (
    Vec<u8>,
    git2::Oid,
    u32,
    u16,
    u16,
    u32,
    u32,
    u32,
    u32,
    u32,
    (i32, u32, i32, u32),
);
fn unrelated_index(repo: &git2::Repository, owned: &Path) -> Result<Vec<IndexImage>, FixtureError> {
    Ok(fixed(repo.index())?
        .iter()
        .filter(|entry| entry.path != owned.to_str().unwrap().as_bytes())
        .map(|entry| {
            (
                entry.path,
                entry.id,
                entry.mode,
                entry.flags,
                entry.flags_extended,
                entry.dev,
                entry.ino,
                entry.uid,
                entry.gid,
                entry.file_size,
                (
                    entry.ctime.seconds(),
                    entry.ctime.nanoseconds(),
                    entry.mtime.seconds(),
                    entry.mtime.nanoseconds(),
                ),
            )
        })
        .collect())
}

fn preservation_matrix() -> Result<(), FixtureError> {
    for category in 0..5 {
        let pair = Pair::new(true)?;
        let item: canonical::ItemId = fixed(DOC.parse())?;
        fixed(pair.a.service.prepare_context(AuthoringTarget {
            root: pair.a.root.clone(),
            kind: AuthoringKind::Document,
            item_id: item.clone(),
            intent: ContextIntent::Edit,
            operation_id: OperationId::new(),
        }))?;
        let repo = fixed(git2::Repository::open(pair.a.context(&item)))?;
        let reference = format!("refs/heads/manyhands/document/{DOC}");
        let mut bytes = None;
        match category {
            0 => {
                let path = pair.a.context(&item).join("staged.txt");
                fixed(std::fs::write(path, b"unrelated staged bytes"))?;
                let mut index = fixed(repo.index())?;
                fixed(index.add_path(Path::new("staged.txt")))?;
                fixed(index.write())?;
                bytes = Some(("staged.txt", b"unrelated staged bytes".to_vec()));
            }
            1 => {
                fixed(std::fs::write(
                    pair.a.context(&item).join("fixture.txt"),
                    b"unrelated unstaged bytes",
                ))?;
                bytes = Some(("fixture.txt", b"unrelated unstaged bytes".to_vec()));
            }
            2 => {
                fixed(std::fs::remove_file(
                    pair.a.context(&item).join("fixture.txt"),
                ))?;
            }
            3 => {
                let base = fixed(repo.refname_to_id(&reference))?;
                let ours = change(
                    &repo,
                    &[base],
                    &[("fixture.txt", Some(b"local code\n"))],
                    "local code",
                )?;
                let theirs = change(
                    &repo,
                    &[base],
                    &[("fixture.txt", Some(b"incoming code\n"))],
                    "incoming code",
                )?;
                install(&repo, &reference, ours)?;
                fixed(repo.merge(&[&fixed(repo.find_annotated_commit(theirs))?], None, None))?;
                assert!(fixed(repo.index())?.has_conflicts());
                bytes = Some((
                    "fixture.txt",
                    fixed(std::fs::read(pair.a.context(&item).join("fixture.txt")))?,
                ));
            }
            _ => {
                let base = fixed(repo.refname_to_id(&reference))?;
                let next = change(
                    &repo,
                    &[base],
                    &[(".gitignore", Some(b"ignored-draft\n"))],
                    "ignore policy",
                )?;
                install(&repo, &reference, next)?;
                fixed(std::fs::write(
                    pair.a.context(&item).join("ignored-draft"),
                    b"unrelated ignored bytes",
                ))?;
                bytes = Some(("ignored-draft", b"unrelated ignored bytes".to_vec()));
            }
        }
        let request = pair.request(&pair.a, AuthoringKind::Document, None);
        let owned = PathBuf::from(format!(
            ".manyhands/comments/{}/{}.md",
            item, request.comment.comment_id
        ));
        let before = unrelated_index(&repo, &owned)?;
        let (receipt, state, _) = saved(fixed(pair.a.service.submit_comment(
            request,
            &mut SessionCredentials::new(Provider::new(Arc::new(AtomicUsize::new(0)))),
        ))?)?;
        assert!(
            unrelated_index(&repo, &owned)? == before,
            "unrelated index mismatch; category"
        );
        if let Some((path, bytes)) = bytes {
            assert!(
                fixed(std::fs::read(pair.a.context(&item).join(path)))? == bytes,
                "unrelated bytes mismatch; category"
            );
        } else {
            assert!(!pair.a.context(&item).join("fixture.txt").exists());
        }
        if category == 4 {
            pair.proof(&receipt, published(state)?)?;
        } else {
            assert!(
                matches!(state,CommentPublicationState::Pending {reason:CommentPublicationPendingReason::Synchronization(e)} if matches!(*e,SynchronizationError::WorktreeNotClean {..}|SynchronizationError::WorktreeConflicted {..}))
            );
        }
        pair.privacy()?;
    }
    Ok(())
}

fn pre_effect_stops() -> Result<(), FixtureError> {
    for point in [
        FailurePoint::CommentAfterDestinationPrepared,
        FailurePoint::CommentAfterCheckpointIntent,
    ] {
        let pair = Pair::new(true)?;
        let request = pair.request(&pair.a, AuthoringKind::Document, None);
        let service = fixed(RepositoryService::open_at_with_failure_point_for_testing(
            &pair.a.data,
            point,
        ))?;
        assert!(
            service
                .submit_comment(
                    request.clone(),
                    &mut SessionCredentials::new(Provider::new(Arc::new(AtomicUsize::new(0))))
                )
                .is_err()
        );
        let path = pair
            .a
            .context(&request.comment.target.item_id)
            .join(format!(
                ".manyhands/comments/{}/{}.md",
                request.comment.target.item_id, request.comment.comment_id
            ));
        let bytes = std::fs::read(&path).ok();
        let time: String = fixed(pair.a.db()?.query_row(
            "SELECT created_at FROM comment_publication_bindings",
            [],
            |r| r.get(0),
        ))?;
        let head = fixed(
            fixed(git2::Repository::open(
                pair.a.context(&request.comment.target.item_id),
            ))?
            .head(),
        )?
        .target();
        let reopened = fixed(RepositoryService::open_at(&pair.a.data))?;
        let calls = pair.server.helper_invocations();
        assert!(
            reopened
                .submit_comment(
                    request.clone(),
                    &mut SessionCredentials::new(Provider::new(Arc::new(AtomicUsize::new(0))))
                )
                .is_err()
        );
        assert_eq!(std::fs::read(path).ok(), bytes);
        assert_eq!(
            fixed(
                fixed(git2::Repository::open(
                    pair.a.context(&request.comment.target.item_id)
                ))?
                .head()
            )?
            .target(),
            head
        );
        assert_eq!(
            fixed(pair.a.db()?.query_row(
                "SELECT created_at FROM comment_publication_bindings",
                [],
                |r| r.get::<_, String>(0)
            ))?,
            time
        );
        assert_eq!(pair.server.helper_invocations(), calls);
        pair.privacy()?;
    }
    Ok(())
}

fn local_discovery_failure() -> Result<(), FixtureError> {
    let pair = Pair::new(true)?;
    fixed(pair.a.db()?.execute_batch("CREATE TRIGGER stop_local_discovery BEFORE INSERT ON discovered_comments BEGIN SELECT RAISE(ABORT,'fixed failure'); END;"))?;
    let prompts = Arc::new(AtomicUsize::new(0));
    let mut session = SessionCredentials::new(Provider::new(prompts.clone()));
    let calls = pair.server.helper_invocations();
    let updates = pair.server.receive_updates().len();
    let (receipt, state, index) = saved(fixed(pair.a.service.submit_comment(
        pair.request(&pair.a, AuthoringKind::Document, None),
        &mut session,
    ))?)?;
    assert!(index.local_pending && !index.remote_pending);
    assert!(matches!(
        state,
        CommentPublicationState::Pending {
            reason: CommentPublicationPendingReason::LocalRecoveryRequired
        }
    ));
    assert_eq!(prompts.load(Ordering::SeqCst), 0);
    assert_eq!(pair.server.helper_invocations(), calls);
    assert_eq!(pair.server.receive_updates().len(), updates);
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
    let bytes = fixed(std::fs::read(
        pair.a.context(&receipt.item_id).join(&receipt.comment_path),
    ))?;
    fixed(
        pair.a
            .db()?
            .execute_batch("DROP TRIGGER stop_local_discovery;"),
    )?;
    let reopened = fixed(RepositoryService::open_at(&pair.a.data))?;
    let (again, state, index) = saved(fixed(
        reopened.retry_comment_publication(retry_request(&pair, &receipt, false), &mut session),
    )?)?;
    assert_eq!(again.operation_id, receipt.operation_id);
    assert_eq!(again.synchronization_id, receipt.synchronization_id);
    assert_eq!(again.checkpoint_oid, receipt.checkpoint_oid);
    assert!(!index.local_pending);
    assert_eq!(
        fixed(std::fs::read(
            pair.a.context(&receipt.item_id).join(&receipt.comment_path)
        ))?,
        bytes
    );
    pair.proof(&again, published(state)?)?;
    pair.privacy()
}

fn local_handoff_stop() -> Result<(), FixtureError> {
    let pair = Pair::new(true)?;
    let service = fixed(RepositoryService::open_at_with_failure_point_for_testing(
        &pair.a.data,
        FailurePoint::CommentAfterLocalHandoff,
    ))?;
    let prompts = Arc::new(AtomicUsize::new(0));
    let mut session = SessionCredentials::new(Provider::new(prompts.clone()));
    let calls = pair.server.helper_invocations();
    let (receipt, state, index) = saved(fixed(service.submit_comment(
        pair.request(&pair.a, AuthoringKind::Document, None),
        &mut session,
    ))?)?;
    assert!(!index.local_pending);
    assert!(matches!(state, CommentPublicationState::Pending { .. }));
    assert_eq!(prompts.load(Ordering::SeqCst), 0);
    assert_eq!(pair.server.helper_invocations(), calls);
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
    let reopened = fixed(RepositoryService::open_at(&pair.a.data))?;
    let (again, state, _) = saved(fixed(
        reopened.retry_comment_publication(retry_request(&pair, &receipt, false), &mut session),
    )?)?;
    assert_eq!(again.checkpoint_oid, receipt.checkpoint_oid);
    assert_eq!(again.synchronization_id, receipt.synchronization_id);
    pair.proof(&again, published(state)?)?;
    pair.privacy()
}

fn transport_kind(state: CommentPublicationState) -> Result<SshTransportErrorKind, FixtureError> {
    if let CommentPublicationState::Pending {
        reason: CommentPublicationPendingReason::Synchronization(error),
    } = state
        && let SynchronizationError::Transport(error) = *error
    {
        return Ok(error.kind);
    }
    Err(FixtureError)
}

fn identity_confirmation() -> Result<(), FixtureError> {
    let pair = Pair::new(true)?;
    save_document_title(&pair.a, "Local title")?;
    save_document_body(
        &pair.b,
        (1..=12)
            .map(|n| format!("shared line {n}\n"))
            .collect::<String>()
            + "remote tail\n",
    )?;
    sync_side(&pair, &pair.b, AuthoringKind::Document, fixed(DOC.parse())?)?;
    let root = pair.a.root.clone();
    let mut provider = Provider::new(Arc::new(AtomicUsize::new(0)));
    provider.action = Some(Box::new(move || {
        let mut config = git2::Repository::open(root).unwrap().config().unwrap();
        config.remove("user.name").unwrap();
        config.remove("user.email").unwrap();
    }));
    let mut session = SessionCredentials::new(provider);
    let (receipt, state, _) = saved(fixed(pair.a.service.submit_comment(
        pair.request(&pair.a, AuthoringKind::Document, None),
        &mut session,
    ))?)?;
    let expected = match state {
        CommentPublicationState::Pending {
            reason: CommentPublicationPendingReason::Synchronization(e),
        } => match *e {
            SynchronizationError::IdentityRequired {
                expected_configuration,
                ..
            } => expected_configuration,
            _ => return Err(FixtureError),
        },
        _ => return Err(FixtureError),
    };
    assert_eq!(
        fixed(pair.a.repo()?.find_commit(receipt.checkpoint_oid))?
            .author()
            .name(),
        Some("Publication fixture")
    );
    let mut retry = retry_request(&pair, &receipt, true);
    retry.confirmed_identity = Some(ConfirmedCommitIdentity {
        confirmation_id: OperationId::new(),
        identity: CommitIdentity {
            name: "Confirmed merge".into(),
            email: "confirmed@example.invalid".into(),
        },
        expected_configuration: expected,
    });
    let (again, state, _) = saved(fixed(
        pair.a
            .service
            .retry_comment_publication(retry, &mut session),
    )?)?;
    let oid = published(state)?;
    assert_eq!(
        fixed(pair.a.repo()?.find_commit(oid))?.author().name(),
        Some("Confirmed merge")
    );
    assert_eq!(
        fixed(pair.a.repo()?.find_commit(receipt.checkpoint_oid))?
            .author()
            .name(),
        Some("Publication fixture")
    );
    assert_eq!(again.checkpoint_oid, receipt.checkpoint_oid);
    pair.proof(&again, oid)?;
    pair.privacy()
}

fn prove_at_server(
    server: &SshRemoteFixture,
    receipt: &CommentReceipt,
    oid: git2::Oid,
) -> Result<(), FixtureError> {
    let remote = fixed(git2::Repository::open_bare(server.repository_path()))?;
    let local = fixed(git2::Repository::open(&receipt.root))?;
    assert_eq!(
        fixed(remote.refname_to_id(&format!("refs/heads/{}", receipt.context_branch)))?,
        oid
    );
    assert!(
        oid == receipt.checkpoint_oid
            || fixed(remote.graph_descendant_of(oid, receipt.checkpoint_oid))?
    );
    assert_eq!(
        fixed(fixed(fixed(remote.find_commit(oid))?.tree())?.get_path(&receipt.comment_path))?.id(),
        fixed(
            fixed(fixed(local.find_commit(receipt.checkpoint_oid))?.tree())?
                .get_path(&receipt.comment_path)
        )?
        .id()
    );
    Ok(())
}

fn distinct_push() -> Result<(), FixtureError> {
    let pair = Pair::new(true)?;
    let destination = SshRemoteFixture::start()?;
    destination.allow_client_public_key(fixed(russh::keys::PublicKey::from_bytes(
        &pair.server.allowed_client_public_key(),
    ))?);
    ssh_privacy::save(&[destination.url().into_bytes()])?;
    let source = pair.a.repo()?;
    let odb = fixed(source.odb())?;
    let remote = fixed(git2::Repository::open_bare(destination.repository_path()))?;
    let target_odb = fixed(remote.odb())?;
    let mut ids = Vec::new();
    fixed(odb.foreach(|oid| {
        ids.push(*oid);
        true
    }))?;
    for id in ids {
        let object = fixed(odb.read(id))?;
        fixed(target_odb.write(object.kind(), object.data()))?;
    }
    let main = fixed(source.refname_to_id("refs/heads/main"))?;
    fixed(remote.reference("refs/heads/main", main, true, "fixture seed"))?;
    fixed(remote.reference(
        &format!("refs/heads/manyhands/document/{DOC}"),
        main,
        true,
        "fixture context seed",
    ))?;
    fixed(source.remote_set_pushurl("origin", Some(&destination.url())))?;
    let mut session = SessionCredentials::new(Provider::new(Arc::new(AtomicUsize::new(0))));
    fixed(pair.a.service.verify_ssh_transport(
        VerifySshTransportRequest {
            root: pair.a.root.clone(),
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
        &mut session,
    ))?;
    fixed(pair.a.service.verify_ssh_transport(
        VerifySshTransportRequest {
            root: pair.a.root.clone(),
            direction: SshDirection::Fetch,
            approval: Some(pair.approval()),
        },
        &mut session,
    ))?;
    let mut request = pair.request(&pair.a, AuthoringKind::Document, None);
    request.approval = None;
    let (receipt, state, _) = saved(fixed(pair.a.service.submit_comment(request, &mut session))?)?;
    let oid = published(state)?;
    prove_at_server(&destination, &receipt, oid)?;
    assert!(
        fixed(git2::Repository::open_bare(pair.server.repository_path()))?
            .find_reference(&format!("refs/heads/{}", receipt.context_branch))
            .is_err()
    );
    pair.privacy()
}

fn remote_deleted() -> Result<(), FixtureError> {
    let pair = Pair::new(true)?;
    let mut session = SessionCredentials::new(Provider::new(Arc::new(AtomicUsize::new(0))));
    let (first, state, _) = saved(fixed(pair.a.service.submit_comment(
        pair.request(&pair.a, AuthoringKind::Document, None),
        &mut session,
    ))?)?;
    published(state)?;
    let remote = fixed(git2::Repository::open_bare(pair.server.repository_path()))?;
    fixed(fixed(remote.find_reference(&format!("refs/heads/{}", first.context_branch)))?.delete())?;
    let updates = pair.server.receive_updates().len();
    let (receipt, state, _) = saved(fixed(pair.a.service.submit_comment(
        pair.request(&pair.a, AuthoringKind::Document, None),
        &mut session,
    ))?)?;
    assert!(
        matches!(state,CommentPublicationState::Pending {reason:CommentPublicationPendingReason::Synchronization(e)} if matches!(*e,SynchronizationError::RemoteContextDeleted))
    );
    assert!(
        pair.a
            .context(&receipt.item_id)
            .join(&receipt.comment_path)
            .is_file()
    );
    assert_eq!(pair.server.receive_updates().len(), updates);
    pair.privacy()
}

fn endpoint_fenced() -> Result<(), FixtureError> {
    let pair = Pair::new(true)?;
    let mut session = SessionCredentials::new(Provider::new(Arc::new(AtomicUsize::new(0))));
    pair.server.disconnect_at(FixtureBoundary::Advertisement);
    let (receipt, _, _) = saved(fixed(pair.a.service.submit_comment(
        pair.request(&pair.a, AuthoringKind::Document, None),
        &mut session,
    ))?)?;
    pair.server.clear_fault();
    let destination = SshRemoteFixture::start()?;
    let url = destination.url();
    ssh_privacy::save(&[url.as_bytes().to_vec()])?;
    let before: (i64, Option<Vec<u8>>) = fixed(pair.a.db()?.query_row(
        "SELECT configuration_generation,endpoint_digest FROM remote_polling_state",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ))?;
    let child_generation: i64 = fixed(pair.a.db()?.query_row(
        "SELECT configuration_generation FROM remote_operation_records WHERE operation_ulid=?1",
        [receipt.synchronization_id.to_string()],
        |r| r.get(0),
    ))?;
    fixed(pair.a.repo()?.remote_set_pushurl("origin", Some(&url)))?;
    let calls = pair.server.helper_invocations();
    let (again, state, _) = saved(fixed(
        fixed(RepositoryService::open_at(&pair.a.data))?
            .retry_comment_publication(retry_request(&pair, &receipt, true), &mut session),
    )?)?;
    assert_eq!(again.checkpoint_oid, receipt.checkpoint_oid);
    assert!(
        matches!(state,CommentPublicationState::Pending {reason:CommentPublicationPendingReason::Synchronization(e)} if matches!(*e,SynchronizationError::ExternalChange|SynchronizationError::RecoveryRequired))
    );
    assert_eq!(pair.server.helper_invocations(), calls);
    let after: (i64, Option<Vec<u8>>) = fixed(pair.a.db()?.query_row(
        "SELECT configuration_generation,endpoint_digest FROM remote_polling_state",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ))?;
    assert!(
        after.0 > before.0 && after.1 != before.1,
        "endpoint generation/digest was not fenced"
    );
    assert_eq!(
        fixed(pair.a.db()?.query_row(
            "SELECT configuration_generation FROM remote_operation_records WHERE operation_ulid=?1",
            [receipt.synchronization_id.to_string()],
            |r| r.get::<_, i64>(0)
        ))?,
        child_generation
    );
    assert_eq!(again.synchronization_id, receipt.synchronization_id);
    assert_eq!(destination.helper_invocations(), 0);
    assert_eq!(destination.authentication_counts(), (0, 0));
    assert_eq!(destination.receive_updates().len(), 0);
    assert_eq!(
        fixed(
            pair.a
                .db()?
                .query_row("SELECT COUNT(*) FROM remote_operation_records", [], |r| r
                    .get::<_, i64>(
                    0
                ))
        )?,
        1
    );
    assert_eq!(
        fixed(pair.a.db()?.query_row(
            "SELECT COUNT(*) FROM comment_publication_bindings",
            [],
            |r| r.get::<_, i64>(0)
        ))?,
        1
    );
    pair.privacy()
}

fn changed_host() -> Result<(), FixtureError> {
    let pair = Pair::new(true)?;
    let mut session = SessionCredentials::new(Provider::new(Arc::new(AtomicUsize::new(0))));
    let (_, state, _) = saved(fixed(pair.a.service.submit_comment(
        pair.request(&pair.a, AuthoringKind::Document, None),
        &mut session,
    ))?)?;
    published(state)?;
    pair.server.rotate_host_key()?;
    let mut request = pair.request(&pair.a, AuthoringKind::Document, None);
    request.approval = None;
    let (receipt, state, _) = saved(fixed(pair.a.service.submit_comment(request, &mut session))?)?;
    let (expected, presented) = match transport_kind(state)? {
        SshTransportErrorKind::HostReplacementRequired {
            expected,
            presented,
        } => (expected, presented),
        _ => return Err(FixtureError),
    };
    let mut retry = retry_request(&pair, &receipt, true);
    retry.approval = Some(HostApproval {
        authority: SshAuthority {
            host: "127.0.0.1".into(),
            port: pair.server.address().port(),
        },
        expected: Some(expected),
        presented,
    });
    let (again, state, _) = saved(fixed(
        pair.a
            .service
            .retry_comment_publication(retry, &mut session),
    )?)?;
    pair.proof(&again, published(state)?)?;
    pair.privacy()
}

fn wrong_selected_key() -> Result<(), FixtureError> {
    let pair = Pair::new(true)?;
    let wrong = generate_key()?;
    let text = fixed(wrong.to_openssh(russh::keys::ssh_key::LineEnding::LF))?;
    ssh_privacy::save(&[text.as_bytes().to_vec()])?;
    let path = pair._directory.path().join("wk");
    fixed(std::fs::write(&path, text.as_bytes()))?;
    let RegisterSharedKeyOutcome::Registered(key) = fixed(pair.a.service.register_shared_key(
        RegisterSharedKeyRequest {
            label: "wrong selected".into(),
            ownership: SharedKeyOwnership::Imported,
            private_key_path: path,
            public_key_path: None,
        },
    ))?
    else {
        return Err(FixtureError);
    };
    fixed(pair.a.service.select_shared_key(key.id))?;
    let authentication = pair.server.authentication_counts();
    let provider = Provider::new(Arc::new(AtomicUsize::new(0)));
    let (receipt, state, _) = saved(fixed(pair.a.service.submit_comment(
        pair.request(&pair.a, AuthoringKind::Document, None),
        &mut SessionCredentials::new(provider),
    ))?)?;
    let CommentPublicationState::Pending {
        reason: CommentPublicationPendingReason::Synchronization(error),
    } = state
    else {
        return Err(FixtureError);
    };
    pair.diagnostics(&format!("{error:?}\n{error}"))?;
    let SynchronizationError::Transport(error) = *error else {
        return Err(FixtureError);
    };
    assert_eq!(error.selected_key_id, Some(key.id));
    assert!(matches!(
        error.kind,
        SshTransportErrorKind::KeyRejected
            | SshTransportErrorKind::UnlockFailed
            | SshTransportErrorKind::TransportUnavailable
    ));
    let attempted = pair.server.authentication_counts();
    assert!(
        attempted.0 > authentication.0 && attempted.1 > authentication.1,
        "wrong selected key was not attempted and rejected"
    );
    assert!(
        pair.a
            .context(&receipt.item_id)
            .join(&receipt.comment_path)
            .is_file()
    );
    assert!(
        !pair
            .server
            .accepted_keys()
            .iter()
            .any(|public| *public == fixed(wrong.public_key().to_bytes()).unwrap())
    );
    pair.privacy()
}

fn checkpoint_count(repo: &git2::Repository) -> Result<usize, FixtureError> {
    let mut walk = fixed(repo.revwalk())?;
    fixed(walk.push_head())?;
    let mut count = 0;
    for oid in walk {
        if fixed(repo.find_commit(fixed(oid)?))?
            .summary()
            .is_some_and(|message| message.starts_with("Checkpoint comment "))
        {
            count += 1;
        }
    }
    Ok(count)
}

fn checkpoint_objects(
    repo: &git2::Repository,
    id: &canonical::ItemId,
) -> Result<Vec<git2::Oid>, FixtureError> {
    let subject = format!("Checkpoint comment {id}");
    let mut objects = Vec::new();
    fixed(fixed(repo.odb())?.foreach(|oid| {
        if let Ok(commit) = repo.find_commit(*oid)
            && commit.message() == Some(subject.as_str())
        {
            objects.push(*oid);
        }
        true
    }))?;
    objects.sort();
    Ok(objects)
}
fn commit_inventory(repo: &git2::Repository) -> Result<Vec<git2::Oid>, FixtureError> {
    let mut commits = Vec::new();
    let odb = fixed(repo.odb())?;
    let mut readable = true;
    fixed(odb.foreach(|oid| {
        match odb.read_header(*oid) {
            Ok((_, git2::ObjectType::Commit)) => commits.push(*oid),
            Ok(_) => {}
            Err(_) => readable = false,
        }
        true
    }))?;
    if !readable {
        return Err(FixtureError);
    }
    commits.sort();
    Ok(commits)
}
fn integration_counts(side: &Side) -> Result<(i64, i64, i64), FixtureError> {
    fixed(side.db()?.query_row("SELECT (SELECT COUNT(*) FROM remote_integration_windows),(SELECT COUNT(*) FROM remote_integration_steps),(SELECT COUNT(*) FROM remote_resolution_attempts)", [], |r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))))
}

fn local_faults() -> Result<(), FixtureError> {
    for fault in 0..6 {
        let pair = Pair::new(true)?;
        let request = pair.request(&pair.a, AuthoringKind::Document, None);
        if fault == 0 {
            fixed(pair.a.db()?.execute_batch("CREATE TRIGGER stop_binding BEFORE INSERT ON comment_publication_bindings BEGIN SELECT RAISE(ABORT,'fixed failure'); END;"))?;
        }
        if fault == 3 {
            fixed(pair.a.db()?.execute_batch("CREATE TRIGGER stop_receipt BEFORE UPDATE OF checkpoint_oid ON comment_publication_bindings BEGIN SELECT RAISE(ABORT,'fixed failure'); END;"))?;
        }
        if fault == 4 {
            fixed(pair.a.db()?.execute_batch("CREATE TRIGGER stop_observation BEFORE UPDATE ON operation_records WHEN NEW.completed_step='authoring_checkpoint_observed' BEGIN SELECT RAISE(ABORT,'fixed failure'); END;"))?;
        }
        let service = if let Some(point) = match fault {
            1 => Some(FailurePoint::BeforeItemWrite),
            2 => Some(FailurePoint::BeforeCheckpointCommit),
            5 => Some(FailurePoint::BeforeRegistryWrite),
            _ => None,
        } {
            fixed(RepositoryService::open_at_with_failure_point_for_testing(
                &pair.a.data,
                point,
            ))?
        } else {
            fixed(RepositoryService::open_at(&pair.a.data))?
        };
        let mut session = SessionCredentials::new(Provider::new(Arc::new(AtomicUsize::new(0))));
        let first = service.submit_comment(request.clone(), &mut session);
        let known = match &first {
            Ok(CommentSubmissionOutcome::Saved { receipt, .. }) => Some((
                receipt.operation_id,
                receipt.synchronization_id,
                receipt.checkpoint_oid,
                receipt.comment_id.clone(),
            )),
            _ => None,
        };
        let before:Option<(String,Option<String>,Option<String>)>=fixed(pair.a.db()?.query_row("SELECT synchronization_ulid,created_at,checkpoint_oid FROM comment_publication_bindings WHERE operation_ulid=?1",[request.comment.target.operation_id.to_string()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional())?;
        let path = pair
            .a
            .context(&request.comment.target.item_id)
            .join(format!(
                ".manyhands/comments/{}/{}.md",
                request.comment.target.item_id, request.comment.comment_id
            ));
        let retained = std::fs::read(&path).ok();
        if fault == 0 {
            fixed(pair.a.db()?.execute_batch("DROP TRIGGER stop_binding;"))?;
        }
        if fault == 3 {
            fixed(pair.a.db()?.execute_batch("DROP TRIGGER stop_receipt;"))?;
        }
        if fault == 4 {
            fixed(pair.a.db()?.execute_batch("DROP TRIGGER stop_observation;"))?;
        }
        let reopened = fixed(RepositoryService::open_at(&pair.a.data))?;
        let outcome = if fault < 3 {
            assert!(first.is_err());
            fixed(reopened.submit_comment(request.clone(), &mut session))?
        } else {
            let (receipt, _, _) = saved(fixed(first)?)?;
            fixed(
                reopened
                    .retry_comment_publication(retry_request(&pair, &receipt, false), &mut session),
            )?
        };
        let (receipt, state, _) = saved(outcome)?;
        assert_eq!(receipt.operation_id, request.comment.target.operation_id);
        assert_eq!(receipt.item_id, request.comment.target.item_id);
        assert_eq!(receipt.comment_id, request.comment.comment_id);
        assert_eq!(receipt.parent_id, request.comment.parent_id);
        if let Some((action, child, checkpoint, comment)) = known {
            assert_eq!(
                (
                    receipt.operation_id,
                    receipt.synchronization_id,
                    receipt.checkpoint_oid,
                    receipt.comment_id.clone()
                ),
                (action, child, checkpoint, comment)
            );
        }
        let after:(String,String,String)=fixed(pair.a.db()?.query_row("SELECT synchronization_ulid,created_at,checkpoint_oid FROM comment_publication_bindings WHERE operation_ulid=?1",[receipt.operation_id.to_string()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))))?;
        if let Some((child, time, checkpoint)) = before {
            assert_eq!(child, after.0);
            if let Some(time) = time {
                assert_eq!(time, after.1);
            }
            if let Some(checkpoint) = checkpoint {
                assert_eq!(checkpoint, after.2);
            }
        }
        let oid = published(state)?;
        pair.proof(&receipt, oid)?;
        if let Some(bytes) = retained {
            assert_eq!(fixed(std::fs::read(path))?, bytes);
        }
        assert_eq!(
            checkpoint_count(&fixed(git2::Repository::open(
                pair.a.context(&receipt.item_id)
            ))?)?,
            1
        );
        assert_eq!(
            checkpoint_objects(
                &fixed(git2::Repository::open(pair.a.context(&receipt.item_id)))?,
                &receipt.comment_id
            )?,
            vec![receipt.checkpoint_oid]
        );
        assert_eq!(
            fixed(pair.a.db()?.query_row(
                "SELECT COUNT(*) FROM remote_operation_records",
                [],
                |r| r.get::<_, i64>(0)
            ))?,
            1
        );
        assert_eq!(
            fixed(pair.a.db()?.query_row(
                "SELECT COUNT(*) FROM comment_publication_bindings",
                [],
                |r| r.get::<_, i64>(0)
            ))?,
            1
        );
        pair.privacy()?;
    }
    Ok(())
}

fn receiver_rejection() -> Result<(), FixtureError> {
    let pair = Pair::new(true)?;
    let base = save_document_title(&pair.b, "Remote base")?;
    sync_side(&pair, &pair.b, AuthoringKind::Document, fixed(DOC.parse())?)?;
    fixed(pair.a.service.prepare_context(AuthoringTarget {
        root: pair.a.root.clone(),
        kind: AuthoringKind::Document,
        item_id: fixed(DOC.parse())?,
        intent: ContextIntent::Edit,
        operation_id: OperationId::new(),
    }))?;
    sync_side(&pair, &pair.a, AuthoringKind::Document, fixed(DOC.parse())?)?;
    let bare = fixed(git2::Repository::open_bare(pair.server.repository_path()))?;
    fixed(fixed(bare.config())?.set_str("user.name", "Fixture"))?;
    fixed(fixed(bare.config())?.set_str("user.email", "fixture@example.invalid"))?;
    let competing = change(
        &bare,
        &[base],
        &[("race.txt", Some(b"receiver competitor"))],
        "receiver race",
    )?;
    let reference = format!("refs/heads/manyhands/document/{DOC}");
    pair.server.race_update(&reference, base, competing)?;
    pair.server.hostile_rejection("cp-server-canary");
    ssh_privacy::save(&[b"cp-server-canary".to_vec()])?;
    let mut session = SessionCredentials::new(Provider::new(Arc::new(AtomicUsize::new(0))));
    let (receipt, state, _) = saved(fixed(pair.a.service.submit_comment(
        pair.request(&pair.a, AuthoringKind::Document, None),
        &mut session,
    ))?)?;
    assert!(
        matches!(state,CommentPublicationState::Pending {reason:CommentPublicationPendingReason::Synchronization(e)} if matches!(*e,SynchronizationError::PushRejected))
    );
    let (again, state, _) = saved(fixed(
        pair.a
            .service
            .retry_comment_publication(retry_request(&pair, &receipt, true), &mut session),
    )?)?;
    pair.proof(&again, published(state)?)?;
    assert_eq!(
        checkpoint_count(&fixed(git2::Repository::open(
            pair.a.context(&receipt.item_id)
        ))?)?,
        1
    );
    pair.privacy()
}

fn privacy_wal_backup() -> Result<(), FixtureError> {
    let pair = Pair::new(true)?;
    let db = pair.a.db()?;
    fixed(db.execute_batch("PRAGMA journal_mode=WAL;"))?;
    let request = pair.request(&pair.a, AuthoringKind::Document, None);
    let digest = blake3::hash(request.comment.body.as_bytes());
    ssh_privacy::save(&[digest.as_bytes().to_vec()])?;
    let hash = digest.to_hex().to_string();
    ssh_privacy::save(&[hash.as_bytes().to_vec()])?;
    ssh_privacy::clean(format!("{request:?}").as_bytes(), &pair.probes)?;
    let (receipt, state, _) = saved(fixed(pair.a.service.submit_comment(
        request,
        &mut SessionCredentials::new(Provider::new(Arc::new(AtomicUsize::new(0)))),
    ))?)?;
    pair.proof(&receipt, published(state)?)?;
    assert!(
        fixed(std::fs::metadata(
            pair.a.data.join(format!("{REGISTRY_FILE}-wal"))
        ))?
        .len()
            > 0,
        "live WAL surface absent"
    );
    ssh_privacy::clean(format!("{receipt:?}").as_bytes(), &pair.probes)?;
    fixed(db.execute(
        "VACUUM INTO ?1",
        [pair.a.data.join("backup").to_str().ok_or(FixtureError)?],
    ))?;
    assert!(
        fixed(std::fs::metadata(pair.a.data.join("backup")))?.len() > 0,
        "backup surface absent"
    );
    pair.privacy()?;
    ssh_privacy::scan(&pair.a.data, &[hash.as_bytes().to_vec()])?;
    Ok(())
}

fn excluded_comment() -> Result<(), FixtureError> {
    for category in 0..3 {
        let pair = Pair::new(true)?;
        let item: canonical::ItemId = fixed(DOC.parse())?;
        fixed(pair.a.service.prepare_context(AuthoringTarget {
            root: pair.a.root.clone(),
            kind: AuthoringKind::Document,
            item_id: item.clone(),
            intent: ContextIntent::Edit,
            operation_id: OperationId::new(),
        }))?;
        let dirty = pair.a.context(&item).join("unrelated");
        fixed(std::fs::write(&dirty, b"unsaved"))?;
        let mut session = SessionCredentials::new(Provider::new(Arc::new(AtomicUsize::new(0))));
        let (receipt, _, _) = saved(fixed(pair.a.service.submit_comment(
            pair.request(&pair.a, AuthoringKind::Document, None),
            &mut session,
        ))?)?;
        fixed(std::fs::remove_file(dirty))?;
        let repo = fixed(git2::Repository::open(pair.a.context(&item)))?;
        let original = fixed(std::fs::read_to_string(
            pair.a.context(&item).join(&receipt.comment_path),
        ))?;
        let replacement = original.replace(BODY, "committed-replacement-canary\n");
        ssh_privacy::save(&[b"committed-replacement-canary".to_vec()])?;
        let changed = if category == 2 {
            fixed(repo.find_commit(receipt.checkpoint_oid))?
                .parent_id(0)
                .map_err(|_| FixtureError)?
        } else {
            change(
                &repo,
                &[receipt.checkpoint_oid],
                &[(
                    fixed(receipt.comment_path.to_str().ok_or(FixtureError))?,
                    if category == 0 {
                        None
                    } else {
                        Some(replacement.as_bytes())
                    },
                )],
                "external removal",
            )?
        };
        install(
            &repo,
            &format!("refs/heads/{}", receipt.context_branch),
            changed,
        )?;
        let index = fixed(std::fs::read(repo.path().join("index")))?;
        let bytes = std::fs::read(pair.a.context(&item).join(&receipt.comment_path)).ok();
        let calls = pair.server.helper_invocations();
        let (again, state, _) = saved(fixed(
            pair.a
                .service
                .retry_comment_publication(retry_request(&pair, &receipt, false), &mut session),
        )?)?;
        assert_eq!(again.checkpoint_oid, receipt.checkpoint_oid);
        assert!(
            matches!(state,CommentPublicationState::Pending {reason:CommentPublicationPendingReason::Synchronization(e)} if matches!(*e,SynchronizationError::RecoveryRequired))
        );
        assert_eq!(pair.server.helper_invocations(), calls);
        assert_eq!(fixed(repo.head())?.target(), Some(changed));
        assert_eq!(fixed(std::fs::read(repo.path().join("index")))?, index);
        assert_eq!(
            std::fs::read(pair.a.context(&item).join(&receipt.comment_path)).ok(),
            bytes
        );
        assert_eq!(
            fixed(pair.a.db()?.query_row(
                "SELECT COUNT(*) FROM comment_publication_bindings",
                [],
                |r| r.get::<_, i64>(0)
            ))?,
            1
        );
        assert_eq!(
            fixed(pair.a.db()?.query_row(
                "SELECT COUNT(*) FROM remote_operation_records",
                [],
                |r| r.get::<_, i64>(0)
            ))?,
            0
        );
        pair.privacy()?;
    }
    Ok(())
}

fn authentication_failure(mode: usize) -> Result<(), FixtureError> {
    let pair = Pair::new(true)?;
    ssh_privacy::save(&[b"cp-wrong-passphrase-canary".to_vec()])?;
    if mode == 5 {
        let (_, state, _) = saved(fixed(pair.a.service.submit_comment(
            pair.request(&pair.a, AuthoringKind::Document, None),
            &mut SessionCredentials::new(Provider::new(Arc::new(AtomicUsize::new(0)))),
        ))?)?;
        published(state)?;
    }
    let mut request = pair.request(&pair.a, AuthoringKind::Document, None);
    let prompts = Arc::new(AtomicUsize::new(0));
    let mut provider = Provider::new(prompts.clone());
    provider.response = match mode {
        0 => 1,
        1 => 2,
        2 => 3,
        _ => 0,
    };
    if mode == 3 {
        request.approval = None;
    }
    if mode == 4 {
        pair.server.reject_client();
    }
    if mode == 5 {
        fixed(pair.a.service.clear_shared_key_selection())?;
    }
    let (receipt, state, _) = saved(fixed(
        pair.a
            .service
            .submit_comment(request, &mut SessionCredentials::new(provider)),
    )?)?;
    let generation: i64 = fixed(pair.a.db()?.query_row(
        "SELECT configuration_generation FROM remote_polling_state",
        [],
        |r| r.get(0),
    ))?;
    let child_generation: i64 = fixed(pair.a.db()?.query_row(
        "SELECT configuration_generation FROM remote_operation_records WHERE operation_ulid=?1",
        [receipt.synchronization_id.to_string()],
        |r| r.get(0),
    ))?;
    if let CommentPublicationState::Pending {
        reason: CommentPublicationPendingReason::Synchronization(error),
    } = &state
    {
        pair.diagnostics(&format!("{error:?}\n{error}"))?;
    }
    let kind = transport_kind(state)?;
    match mode {
        0 => assert_eq!(kind, SshTransportErrorKind::UnlockCancelled),
        1 => assert_eq!(kind, SshTransportErrorKind::ProviderUnavailable),
        2 => assert_eq!(kind, SshTransportErrorKind::TransportUnavailable),
        3 => {
            assert!(matches!(
                kind,
                SshTransportErrorKind::HostApprovalRequired { .. }
            ));
            assert_eq!(prompts.load(Ordering::SeqCst), 0);
        }
        // Imported-key authentication cannot disambiguate backend denial from
        // passphrase/connection failure; retain the delegated fixed category.
        4 => assert!(matches!(
            kind,
            SshTransportErrorKind::TransportUnavailable
                | SshTransportErrorKind::UnlockFailed
                | SshTransportErrorKind::KeyRejected
        )),
        _ => assert_eq!(kind, SshTransportErrorKind::NoSelectedKey),
    }
    pair.server.restore_client();
    if mode == 5 {
        let keys = fixed(pair.a.service.list_shared_keys())?;
        fixed(pair.a.service.select_shared_key(keys[0].id))?;
    }
    let mut session = SessionCredentials::new(Provider::new(Arc::new(AtomicUsize::new(0))));
    let (again, state, _) = saved(fixed(
        pair.a
            .service
            .retry_comment_publication(retry_request(&pair, &receipt, true), &mut session),
    )?)?;
    assert_eq!(again.checkpoint_oid, receipt.checkpoint_oid);
    assert_eq!(again.synchronization_id, receipt.synchronization_id);
    if mode == 5 {
        // Changing shared selection deliberately advances the existing remote
        // generation. The old child is fenced, never silently rebound.
        assert!(
            matches!(state,CommentPublicationState::Pending {reason:CommentPublicationPendingReason::Synchronization(e)} if matches!(*e,SynchronizationError::RecoveryRequired))
        );
        assert!(
            fixed(pair.a.db()?.query_row(
                "SELECT configuration_generation FROM remote_polling_state",
                [],
                |r| r.get::<_, i64>(0)
            ))? > generation
        );
        assert_eq!(fixed(pair.a.db()?.query_row("SELECT configuration_generation FROM remote_operation_records WHERE operation_ulid=?1", [receipt.synchronization_id.to_string()], |r| r.get::<_,i64>(0)))?, child_generation);
        assert_eq!(
            fixed(pair.a.db()?.query_row(
                "SELECT COUNT(*) FROM remote_operation_records WHERE operation_ulid=?1",
                [receipt.synchronization_id.to_string()],
                |r| r.get::<_, i64>(0)
            ))?,
            1
        );
        let oid = sync_side(&pair, &pair.a, again.kind, again.item_id.clone())?;
        pair.proof(&again, oid)?;
    } else {
        pair.proof(&again, published(state)?)?;
    }
    pair.privacy()
}

fn ambiguous_push(persistence: bool) -> Result<(), FixtureError> {
    let pair = Pair::new(true)?;
    let cursor = pair.server.receive_updates().len();
    let mut session = SessionCredentials::new(Provider::new(Arc::new(AtomicUsize::new(0))));
    if persistence {
        fixed(pair.a.db()?.execute_batch("CREATE TRIGGER stop_verification BEFORE UPDATE ON remote_operation_records WHEN NEW.sync_checkpoint='push_verified' BEGIN SELECT RAISE(ABORT,'fixed failure'); END;"))?;
    } else {
        pair.server.disconnect_at(FixtureBoundary::AfterReceivePack);
    }
    let (receipt, state, _) = saved(fixed(pair.a.service.submit_comment(
        pair.request(&pair.a, AuthoringKind::Document, None),
        &mut session,
    ))?)?;
    assert!(matches!(state, CommentPublicationState::Pending { .. }));
    pair.proof(&receipt, receipt.checkpoint_oid)?;
    receiver_receipt(&pair.server, &receipt, cursor)?;
    pair.server.wait_for_helper_audits()?;
    let updates = pair.server.receive_updates().len();
    let observations = pair.server.receive_advertisements();
    if persistence {
        fixed(
            pair.a
                .db()?
                .execute_batch("DROP TRIGGER stop_verification;"),
        )?;
    } else {
        assert!(pair.server.receive_status_withheld());
        pair.server.clear_fault();
    }
    let reopened = fixed(RepositoryService::open_at(&pair.a.data))?;
    let (again, state, _) = saved(fixed(
        reopened.retry_comment_publication(retry_request(&pair, &receipt, true), &mut session),
    )?)?;
    assert_eq!(again.checkpoint_oid, receipt.checkpoint_oid);
    pair.proof(&again, published(state)?)?;
    pair.server.wait_for_helper_audits()?;
    assert_eq!(pair.server.receive_updates().len(), updates);
    assert!(
        pair.server.receive_advertisements() > observations,
        "accepted push was not freshly re-observed"
    );
    pair.privacy()
}

fn save_document_body(side: &Side, body: String) -> Result<git2::Oid, FixtureError> {
    let item: canonical::ItemId = fixed(DOC.parse())?;
    fixed(side.service.prepare_context(AuthoringTarget {
        root: side.root.clone(),
        kind: AuthoringKind::Document,
        item_id: item.clone(),
        intent: ContextIntent::Edit,
        operation_id: OperationId::new(),
    }))?;
    let bytes = fixed(std::fs::read(side.context(&item).join("docs/a.md")))?;
    let title = match fixed(canonical::parse_item(
        Path::new("docs/a.md"),
        fixed(std::str::from_utf8(&bytes))?,
    ))? {
        canonical::CanonicalItem::Document(d) => d.title,
        _ => return Err(FixtureError),
    };
    let result = fixed(side.service.save_document(SaveDocumentRequest {
        target: AuthoringTarget {
            root: side.root.clone(),
            kind: AuthoringKind::Document,
            item_id: item,
            intent: ContextIntent::Edit,
            operation_id: OperationId::new(),
        },
        source_path: Some("docs/a.md".into()),
        destination_path: "docs/a.md".into(),
        draft: DocumentDraft { title, body },
        expected_source: Some(ExpectedPathObservation::from_bytes(&bytes)),
        expected_destination: ExpectedPathObservation::from_bytes(&bytes),
    }))?;
    match result {
        SaveOutcome::Saved {
            checkpoint: LocalCheckpoint::Checkpointed { commit_oid },
            ..
        } => Ok(commit_oid),
        _ => Err(FixtureError),
    }
}

fn clean_divergence() -> Result<(), FixtureError> {
    let pair = Pair::new(true)?;
    let prior = save_document_title(&pair.a, "Local title")?;
    let remote = save_document_body(
        &pair.b,
        (1..=12)
            .map(|n| format!("shared line {n}\n"))
            .collect::<String>()
            + "remote tail\n",
    )?;
    sync_side(&pair, &pair.b, AuthoringKind::Document, fixed(DOC.parse())?)?;
    let bare = fixed(git2::Repository::open_bare(pair.server.repository_path()))?;
    let primary = fixed(bare.refname_to_id("refs/heads/main"))?;
    fixed(bare.reference("refs/heads/unrelated", primary, true, "fixture"))?;
    let remote_before = remote_refs(&pair.server)?;
    let root = pair.a.repo()?;
    let main = fixed(root.refname_to_id("refs/heads/main"))?;
    fixed(std::fs::write(
        root.path().join("FETCH_HEAD"),
        b"cp-fetch-head-sentinel",
    ))?;
    fixed(root.reference("refs/remotes/origin/unrelated", main, true, "fixture"))?;
    let primary_index = fixed(std::fs::read(root.path().join("index")))?;
    let unsaved = b"primary buffer remains uncheckpointed\n";
    fixed(std::fs::write(pair.a.root.join("docs/a.md"), unsaved))?;
    let (receipt, state, _) = saved(fixed(pair.a.service.submit_comment(
        pair.request(&pair.a, AuthoringKind::Document, None),
        &mut SessionCredentials::new(Provider::new(Arc::new(AtomicUsize::new(0)))),
    ))?)?;
    let oid = published(state)?;
    pair.proof(&receipt, oid)?;
    let published_tree = fixed(fixed(bare.find_commit(oid))?.tree())?;
    let document_blob =
        fixed(bare.find_blob(fixed(published_tree.get_path(Path::new("docs/a.md")))?.id()))?;
    let canonical::CanonicalItem::Document(document) = fixed(canonical::parse_item(
        Path::new("docs/a.md"),
        fixed(std::str::from_utf8(document_blob.content()))?,
    ))?
    else {
        return Err(FixtureError);
    };
    assert_eq!(document.title, "Local title");
    assert!(document.body.ends_with("remote tail\n"));
    let context_ref = format!("refs/heads/{}", receipt.context_branch);
    assert_eq!(
        remote_refs(&pair.server)?
            .into_iter()
            .filter(|(name, _)| name != &context_ref)
            .collect::<Vec<_>>(),
        remote_before
            .into_iter()
            .filter(|(name, _)| name != &context_ref)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        checkpoint_objects(&root, &receipt.comment_id)?,
        vec![receipt.checkpoint_oid]
    );
    assert!(
        fixed(root.graph_descendant_of(oid, prior))?
            && fixed(root.graph_descendant_of(oid, remote))?
    );
    assert_eq!(fixed(root.refname_to_id("refs/heads/main"))?, main);
    assert_eq!(
        fixed(root.refname_to_id("refs/remotes/origin/unrelated"))?,
        main
    );
    assert_eq!(
        fixed(std::fs::read(root.path().join("FETCH_HEAD")))?,
        b"cp-fetch-head-sentinel"
    );
    assert_eq!(
        fixed(std::fs::read(root.path().join("index")))?,
        primary_index
    );
    assert_eq!(
        fixed(std::fs::read(pair.a.root.join("docs/a.md")))?,
        unsaved
    );
    pair.privacy()
}

fn collaborator_current() -> Result<(), FixtureError> {
    let pair = Pair::new(true)?;
    let item: canonical::ItemId = fixed(DOC.parse())?;
    fixed(pair.a.service.prepare_context(AuthoringTarget {
        root: pair.a.root.clone(),
        kind: AuthoringKind::Document,
        item_id: item.clone(),
        intent: ContextIntent::Edit,
        operation_id: OperationId::new(),
    }))?;
    let dirty = pair.a.context(&item).join("unrelated");
    fixed(std::fs::write(&dirty, b"unsaved"))?;
    let mut session = SessionCredentials::new(Provider::new(Arc::new(AtomicUsize::new(0))));
    let (receipt, state, _) = saved(fixed(pair.a.service.submit_comment(
        pair.request(&pair.a, AuthoringKind::Document, None),
        &mut session,
    ))?)?;
    assert!(matches!(state, CommentPublicationState::Pending { .. }));
    // Separate collaborator Git tooling publishes the already committed branch
    // through real authenticated SSH, outside the unused bound operation.
    push(
        &pair.server,
        &pair.a.repo()?,
        &format!("refs/heads/{}", receipt.context_branch),
    )?;
    fixed(std::fs::remove_file(dirty))?;
    let (again, state, _) = saved(fixed(
        pair.a
            .service
            .retry_comment_publication(retry_request(&pair, &receipt, false), &mut session),
    )?)?;
    assert!(matches!(
        state,
        CommentPublicationState::AlreadyCurrent { .. }
    ));
    pair.proof(&again, published(state)?)?;
    pair.privacy()
}

fn external_repair() -> Result<(), FixtureError> {
    let pair = Pair::new(true)?;
    save_document_title(&pair.a, "Local title")?;
    let incoming = save_document_title(&pair.b, "Remote title")?;
    sync_side(&pair, &pair.b, AuthoringKind::Document, fixed(DOC.parse())?)?;
    let mut session = SessionCredentials::new(Provider::new(Arc::new(AtomicUsize::new(0))));
    let (receipt, state, _) = saved(fixed(pair.a.service.submit_comment(
        pair.request(&pair.a, AuthoringKind::Document, None),
        &mut session,
    ))?)?;
    assert!(
        matches!(state,CommentPublicationState::Pending {reason:CommentPublicationPendingReason::Synchronization(e)} if matches!(*e,SynchronizationError::ConflictPending {..}))
    );
    let repository = fixed(git2::Repository::open(pair.a.context(&receipt.item_id)))?;
    let original_binding = binding_evidence(&pair.a, &receipt)?;
    let repaired = change(
        &repository,
        &[receipt.checkpoint_oid, incoming],
        &[],
        "External exact repair",
    )?;
    fixed(repository.checkout_tree(
        &fixed(repository.find_object(repaired, None))?,
        Some(git2::build::CheckoutBuilder::new().force()),
    ))?;
    fixed(repository.reference(
        &format!("refs/heads/{}", receipt.context_branch),
        repaired,
        true,
        "external repair",
    ))?;
    fixed(repository.cleanup_state())?;
    let (again, state, _) = saved(fixed(
        pair.a
            .service
            .retry_comment_publication(retry_request(&pair, &receipt, true), &mut session),
    )?)?;
    assert_eq!(again.checkpoint_oid, receipt.checkpoint_oid);
    let authority = published(state)?;
    assert_eq!(authority, repaired);
    let commit = fixed(repository.find_commit(authority))?;
    assert_eq!(
        commit.parent_ids().collect::<Vec<_>>(),
        vec![receipt.checkpoint_oid, incoming]
    );
    assert_eq!(binding_evidence(&pair.a, &again)?, original_binding);
    assert_eq!(
        checkpoint_objects(&repository, &again.comment_id)?,
        vec![receipt.checkpoint_oid]
    );
    pair.proof(&again, authority)?;
    pair.privacy()
}

fn conflict_other_context() -> Result<(), FixtureError> {
    let pair = Pair::new(true)?;
    save_document_title(&pair.a, "Local title")?;
    save_document_title(&pair.b, "Remote title")?;
    sync_side(&pair, &pair.b, AuthoringKind::Document, fixed(DOC.parse())?)?;
    let mut session = SessionCredentials::new(Provider::new(Arc::new(AtomicUsize::new(0))));
    let (_, state, _) = saved(fixed(pair.a.service.submit_comment(
        pair.request(&pair.a, AuthoringKind::Document, None),
        &mut session,
    ))?)?;
    assert!(
        matches!(state,CommentPublicationState::Pending {reason:CommentPublicationPendingReason::Synchronization(e)} if matches!(*e,SynchronizationError::ConflictPending {..}))
    );
    let (receipt, state, _) = saved(fixed(pair.a.service.submit_comment(
        pair.request(&pair.a, AuthoringKind::Ticket, None),
        &mut session,
    ))?)?;
    assert!(
        matches!(state,CommentPublicationState::Pending {reason:CommentPublicationPendingReason::Synchronization(e)} if matches!(*e,SynchronizationError::Busy))
    );
    assert!(
        pair.a
            .context(&receipt.item_id)
            .join(&receipt.comment_path)
            .is_file()
    );
    pair.privacy()
}

fn save_document_title(side: &Side, title: &str) -> Result<git2::Oid, FixtureError> {
    let item: canonical::ItemId = fixed(DOC.parse())?;
    fixed(side.service.prepare_context(AuthoringTarget {
        root: side.root.clone(),
        kind: AuthoringKind::Document,
        item_id: item.clone(),
        intent: ContextIntent::Edit,
        operation_id: OperationId::new(),
    }))?;
    let bytes = fixed(std::fs::read(side.context(&item).join("docs/a.md")))?;
    let document = match fixed(canonical::parse_item(
        Path::new("docs/a.md"),
        fixed(std::str::from_utf8(&bytes))?,
    ))? {
        canonical::CanonicalItem::Document(d) => d,
        _ => return Err(FixtureError),
    };
    let result = fixed(side.service.save_document(SaveDocumentRequest {
        target: AuthoringTarget {
            root: side.root.clone(),
            kind: AuthoringKind::Document,
            item_id: item,
            intent: ContextIntent::Edit,
            operation_id: OperationId::new(),
        },
        source_path: Some("docs/a.md".into()),
        destination_path: "docs/a.md".into(),
        draft: DocumentDraft {
            title: title.into(),
            body: document.body,
        },
        expected_source: Some(ExpectedPathObservation::from_bytes(&bytes)),
        expected_destination: ExpectedPathObservation::from_bytes(&bytes),
    }))?;
    match result {
        SaveOutcome::Saved {
            checkpoint: LocalCheckpoint::Checkpointed { commit_oid },
            ..
        } => Ok(commit_oid),
        _ => Err(FixtureError),
    }
}

fn sync_side(
    pair: &Pair,
    side: &Side,
    kind: AuthoringKind,
    item: canonical::ItemId,
) -> Result<git2::Oid, FixtureError> {
    let result = fixed(side.service.synchronize_remote(
        SynchronizeRemoteRequest {
            root: side.root.clone(),
            operation_id: OperationId::new(),
            target: SynchronizationTarget::Context {
                kind,
                item_id: item,
            },
            approval: Some(pair.approval()),
            confirmed_identity: None,
            restart: false,
        },
        &mut SessionCredentials::new(Provider::new(Arc::new(AtomicUsize::new(0)))),
    ))?;
    let outcome = match result {
        SynchronizationResult::Complete(o) => o,
        SynchronizationResult::IndexPending(i) => i.authoritative,
    };
    match outcome {
        SynchronizationOutcome::Published { oid, .. }
        | SynchronizationOutcome::AlreadyCurrent { oid, .. } => Ok(oid),
        _ => Err(FixtureError),
    }
}

fn conflict_cancel_resolution() -> Result<(), FixtureError> {
    let pair = Pair::new(true)?;
    save_document_title(&pair.a, "Local title")?;
    let remote = save_document_title(&pair.b, "Remote title")?;
    assert_eq!(
        sync_side(&pair, &pair.b, AuthoringKind::Document, fixed(DOC.parse())?)?,
        remote
    );
    let mut session = SessionCredentials::new(Provider::new(Arc::new(AtomicUsize::new(0))));
    let (receipt, state, _) = saved(fixed(pair.a.service.submit_comment(
        pair.request(&pair.a, AuthoringKind::Document, None),
        &mut session,
    ))?)?;
    assert!(
        matches!(state,CommentPublicationState::Pending {reason:CommentPublicationPendingReason::Synchronization(e)} if matches!(*e,SynchronizationError::ConflictPending {operation_id,..} if operation_id==receipt.synchronization_id))
    );
    assert!(!fixed(pair.a.service.cancel_comment_publication(
        &pair.a.root,
        receipt.operation_id
    ))?);
    let (_, state, _) = saved(fixed(
        pair.a
            .service
            .retry_comment_publication(retry_request(&pair, &receipt, true), &mut session),
    )?)?;
    assert!(
        matches!(state,CommentPublicationState::Pending {reason:CommentPublicationPendingReason::Synchronization(e)} if matches!(*e,SynchronizationError::ConflictPending {..}))
    );
    let resolving = fixed(RepositoryService::open_at_with_failure_point_for_testing(
        &pair.a.data,
        FailurePoint::ResolutionBeforeMetadataRetirement,
    ))?;
    let inspection = fixed(
        resolving.inspect_synchronization_recovery(&pair.a.root, receipt.synchronization_id),
    )?;
    let mut resolutions = Vec::new();
    for path in &inspection.paths {
        let sides = fixed(resolving.read_synchronization_conflict(&path.token))?;
        resolutions.push((path.token.clone(), sides.local.ok_or(FixtureError)?));
    }
    let request = ResolveSynchronizationRequest::new(
        pair.a.root.clone(),
        receipt.synchronization_id,
        OperationId::new(),
        inspection.observation,
        resolutions,
        None,
    );
    assert!(resolving.resolve_synchronization(request.clone()).is_err());
    let original_binding = binding_evidence(&pair.a, &receipt)?;
    let original_source = fixed(std::fs::read(
        pair.a.context(&receipt.item_id).join(&receipt.comment_path),
    ))?;
    let attempt: (String, String, String, String, String) = fixed(pair.a.db()?.query_row("SELECT a.attempt_ulid,a.candidate_oid,a.checkpoint_oid,s.phase,i.phase FROM remote_resolution_attempts a JOIN remote_integration_steps s ON s.id=a.integration_step_id JOIN remote_resolution_index_artifacts i ON i.attempt_id=a.id JOIN remote_operation_records r ON r.id=a.operation_record_id WHERE r.operation_ulid=?1", [receipt.synchronization_id.to_string()], |r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))))?;
    assert_eq!(attempt.1, attempt.2);
    assert_eq!(attempt.3, "applied");
    assert_eq!(attempt.4, "published");
    drop(resolving);
    let reopened = fixed(RepositoryService::open_at(&pair.a.data))?;
    assert!(fixed(reopened.cancel_comment_publication(
        &pair.a.root,
        receipt.operation_id
    ))?);
    let (_, state, _) = saved(fixed(
        reopened.retry_comment_publication(retry_request(&pair, &receipt, true), &mut session),
    )?)?;
    assert!(
        matches!(state,CommentPublicationState::Pending {reason:CommentPublicationPendingReason::Synchronization(e)} if matches!(*e,SynchronizationError::Interrupted))
    );
    let phase = fixed(pair.a.db()?.query_row(
        "SELECT phase FROM remote_operation_records WHERE operation_ulid=?1",
        [receipt.synchronization_id.to_string()],
        |r| r.get::<_, String>(0),
    ))?;
    ssh_harness::observation(&[match phase.as_str() {
        "cancelled" => 201,
        "interrupted" => 202,
        "reconciling" => 203,
        _ => 204,
    }]);
    assert_eq!(phase, "interrupted");
    let ResolveSynchronizationOutcome::LocalCheckpointComplete { commit_oid } =
        fixed(reopened.resolve_synchronization(request))?
    else {
        return Err(FixtureError);
    };
    assert_eq!(commit_oid.to_string(), attempt.1);
    let repository = fixed(git2::Repository::open(pair.a.context(&receipt.item_id)))?;
    assert_eq!(
        fixed(repository.find_commit(commit_oid))?
            .parent_ids()
            .collect::<Vec<_>>(),
        vec![receipt.checkpoint_oid, remote]
    );
    let completed: (String, String, String) = fixed(pair.a.db()?.query_row("SELECT attempt_ulid,candidate_oid,checkpoint_oid FROM remote_resolution_attempts WHERE attempt_ulid=?1", [&attempt.0], |r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))))?;
    assert_eq!(completed, (attempt.0, attempt.1, attempt.2));
    let (again, state, _) = saved(fixed(
        reopened.retry_comment_publication(retry_request(&pair, &receipt, true), &mut session),
    )?)?;
    assert_eq!(again.checkpoint_oid, receipt.checkpoint_oid);
    assert_eq!(again.synchronization_id, receipt.synchronization_id);
    let authority = published(state)?;
    assert_eq!(authority, commit_oid);
    assert_eq!(binding_evidence(&pair.a, &again)?, original_binding);
    assert_eq!(
        fixed(std::fs::read(
            pair.a.context(&receipt.item_id).join(&receipt.comment_path)
        ))?,
        original_source
    );
    assert_eq!(
        checkpoint_objects(&repository, &receipt.comment_id)?,
        vec![receipt.checkpoint_oid]
    );
    pair.proof(&again, authority)?;
    pair.privacy()
}

fn interrupted_restart() -> Result<(), FixtureError> {
    let pair = Pair::new(true)?;
    let mut session = SessionCredentials::new(Provider::new(Arc::new(AtomicUsize::new(0))));
    pair.server.disconnect_at(FixtureBoundary::Advertisement);
    let (receipt, state, _) = saved(fixed(pair.a.service.submit_comment(
        pair.request(&pair.a, AuthoringKind::Document, None),
        &mut session,
    ))?)?;
    assert!(matches!(state, CommentPublicationState::Pending { .. }));
    pair.server.clear_fault();
    let calls = pair.server.helper_invocations();
    let (again, state, _) = saved(fixed(
        pair.a
            .service
            .retry_comment_publication(retry_request(&pair, &receipt, false), &mut session),
    )?)?;
    assert_eq!(again.checkpoint_oid, receipt.checkpoint_oid);
    assert!(
        matches!(state,CommentPublicationState::Pending {reason:CommentPublicationPendingReason::Synchronization(error)} if matches!(*error,SynchronizationError::RecoveryRequired))
    );
    assert_eq!(pair.server.helper_invocations(), calls);
    let reopened = fixed(RepositoryService::open_at(&pair.a.data))?;
    let (again, state, _) = saved(fixed(
        reopened.retry_comment_publication(retry_request(&pair, &receipt, true), &mut session),
    )?)?;
    assert_eq!(again.synchronization_id, receipt.synchronization_id);
    pair.proof(&again, published(state)?)?;
    assert_eq!(
        fixed(
            pair.a
                .db()?
                .query_row("SELECT COUNT(*) FROM remote_operation_records", [], |r| r
                    .get::<_, i64>(
                    0
                ))
        )?,
        1
    );
    pair.privacy()
}

fn terminal_cancel() -> Result<(), FixtureError> {
    let pair = Pair::new(true)?;
    let request = pair.request(&pair.a, AuthoringKind::Document, None);
    let root = pair.a.root.clone();
    let data = pair.a.data.clone();
    let id = request.comment.target.operation_id;
    let mut provider = Provider::new(Arc::new(AtomicUsize::new(0)));
    provider.action = Some(Box::new(move || {
        assert!(
            RepositoryService::open_at(&data)
                .unwrap()
                .cancel_comment_publication(&root, id)
                .unwrap()
        );
    }));
    let mut session = SessionCredentials::new(provider);
    let (receipt, state, _) = saved(fixed(pair.a.service.submit_comment(request, &mut session))?)?;
    assert!(matches!(state, CommentPublicationState::Pending { .. }));
    assert_eq!(
        fixed(pair.a.db()?.query_row(
            "SELECT phase FROM remote_operation_records WHERE operation_ulid=?1",
            [receipt.synchronization_id.to_string()],
            |r| r.get::<_, String>(0)
        ))?,
        "cancelled"
    );
    let calls = pair.server.helper_invocations();
    let (_, state, _) = saved(fixed(
        pair.a
            .service
            .retry_comment_publication(retry_request(&pair, &receipt, true), &mut session),
    )?)?;
    assert!(
        matches!(state,CommentPublicationState::Pending {reason:CommentPublicationPendingReason::Synchronization(e)} if matches!(*e,SynchronizationError::Interrupted))
    );
    assert_eq!(pair.server.helper_invocations(), calls);
    let ordinary = fixed(pair.a.service.synchronize_remote(
        SynchronizeRemoteRequest {
            root: pair.a.root.clone(),
            operation_id: OperationId::new(),
            target: SynchronizationTarget::Context {
                kind: receipt.kind,
                item_id: receipt.item_id.clone(),
            },
            approval: Some(pair.approval()),
            confirmed_identity: None,
            restart: false,
        },
        &mut session,
    ))?;
    let outcome = match ordinary {
        SynchronizationResult::Complete(o) => o,
        SynchronizationResult::IndexPending(i) => i.authoritative,
    };
    let oid = match outcome {
        SynchronizationOutcome::Published { oid, .. }
        | SynchronizationOutcome::AlreadyCurrent { oid, .. } => oid,
        _ => return Err(FixtureError),
    };
    pair.proof(&receipt, oid)?;
    assert_eq!(
        fixed(pair.a.db()?.query_row(
            "SELECT COUNT(*) FROM comment_publication_bindings",
            [],
            |r| r.get::<_, i64>(0)
        ))?,
        1
    );
    pair.privacy()
}

fn historical_replay() -> Result<(), FixtureError> {
    let pair = Pair::new(true)?;
    let request = pair.request(&pair.a, AuthoringKind::Document, None);
    let mut session = SessionCredentials::new(Provider::new(Arc::new(AtomicUsize::new(0))));
    let (receipt, state, _) = saved(fixed(
        pair.a.service.submit_comment(request.clone(), &mut session),
    )?)?;
    let oid = published(state)?;
    let path = pair.a.context(&receipt.item_id).join(&receipt.comment_path);
    let original = fixed(std::fs::read_to_string(&path))?;
    let altered = original.replace(BODY, "edited-current-body-canary\n");
    fixed(std::fs::write(&path, &altered))?;
    let calls = pair.server.helper_invocations();
    let (again, state, _) = saved(fixed(
        pair.a.service.submit_comment(request.clone(), &mut session),
    )?)?;
    assert_eq!(again.checkpoint_oid, receipt.checkpoint_oid);
    assert_eq!(published(state)?, oid);
    assert_eq!(fixed(std::fs::read_to_string(&path))?, altered);
    let mut wrong = request;
    wrong.comment.body = "edited-current-body-canary\n".into();
    assert!(pair.a.service.submit_comment(wrong, &mut session).is_err());
    fixed(
        pair.a
            .repo()?
            .remote_set_pushurl("origin", Some("ssh://fixture@127.0.0.1:1/unavailable")),
    )?;
    let (_, state, _) = saved(fixed(
        pair.a
            .service
            .retry_comment_publication(retry_request(&pair, &receipt, false), &mut session),
    )?)?;
    assert_eq!(published(state)?, oid);
    assert_eq!(pair.server.helper_invocations(), calls);
    pair.proof(&receipt, oid)?;
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
