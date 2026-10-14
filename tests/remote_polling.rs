//! Whole one-shot polling acceptance over real authenticated loopback SSH.
//! Public library only: no source inclusion, fabricated transport or cfg(test) hooks.
#![allow(clippy::result_large_err)]
pub use manyhands::{canonical, repository, runtime};
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

fn main() {
    // SAFETY: the real main owns initialization before any fixture/runtime thread.
    if unsafe { ssh_harness::initialize() }.is_err() {
        eprintln!("SSH poll initialization failed");
        std::process::exit(1);
    }
    ssh_harness::run_with_short_output_privacy(CASES, OUTPUT_CONTROLS);
}

const T: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
const D: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
const C: &str = "01CRZ3NDEKTSV4RRFFQ69G5FAV";
const DOCUMENT_COMMENT: &str = "01DRZ3NDEKTSV4RRFFQ69G5FAV";
const MAIN: &str = "refs/heads/main";
const TP: &str = ".manyhands/tickets/01ARZ3NDEKTSV4RRFFQ69G5FAW/ticket.md";
const DP: &str = "docs/d.md";
const PASSWORD: &str = "poll-private-passphrase-canary";
const BODY: &str = "poll-private-comment-body-canary";
const TITLE: &str = "poll-private-rejected-title-canary";
const DOCUMENT_TITLE: &str = "poll-private-accepted-document-title-canary";
const TICKET_TITLE: &str = "poll-private-accepted-ticket-title-canary";
const BAD_REF: &str = "refs/heads/manyhands/ticket/poll-private-malformed-canary";
const MODE: u32 = 0o100644;

const CASES: &[ssh_harness::Case] = &[
    (
        "poll_whole_review_endpoint_attention_automatic_retains_backoff",
        whole_review_endpoint_automatic,
    ),
    (
        "poll_whole_review_endpoint_attention_explicit_retains_backoff",
        whole_review_endpoint_explicit,
    ),
    (
        "poll_whole_review_disposition_fault_cancel_keeps_index_pending",
        whole_review_disposition_cancel,
    ),
    (
        "poll_whole_review_branch_record_fault_cancel_keeps_index_pending",
        whole_review_branch_cancel,
    ),
    (
        "poll_whole_review_new_auto_invalid_config_disabled",
        whole_review_auto_disabled,
    ),
    (
        "poll_whole_review_new_auto_invalid_config_paused",
        whole_review_auto_paused,
    ),
    (
        "poll_whole_review_new_auto_invalid_config_recovery_suspended",
        whole_review_auto_suspended,
    ),
    (
        "poll_whole_review_new_auto_invalid_config_not_due",
        whole_review_auto_not_due,
    ),
    (
        "poll_whole_review_invalid_config_automatic_retains_backoff",
        whole_review_attention_automatic,
    ),
    (
        "poll_whole_review_invalid_config_explicit_retains_backoff",
        whole_review_attention_explicit,
    ),
    ("poll_primary_fast_forward", primary_fast_forward),
    ("poll_document_fast_forward", document_fast_forward),
    ("poll_ticket_fast_forward", ticket_fast_forward),
    ("poll_all_three_fast_forward", all_three_fast_forward),
    ("poll_already_current", already_current),
    ("poll_materialize_document_once", materialize_document_once),
    ("poll_materialize_ticket_once", materialize_ticket_once),
    (
        "poll_concurrent_materialization_once",
        concurrent_materialization_once,
    ),
    ("poll_tracked_dirty", tracked_dirty),
    ("poll_staged_dirty", staged_dirty),
    ("poll_untracked_dirty", untracked_dirty),
    ("poll_git_conflicted_without_pending_record", git_conflicted),
    ("poll_local_ahead", local_ahead),
    ("poll_diverged", diverged),
    ("poll_context_wrong_branch", context_wrong_branch),
    ("poll_missing_worktree_directory", missing_directory),
    #[cfg(unix)]
    (
        "poll_unix_permission_denied_worktree_preserves_local_state",
        permission_denied_worktree,
    ),
    ("poll_primary_wrong_branch", primary_wrong_branch),
    ("poll_remote_deleted_preserves_local_work", remote_deleted),
    (
        "poll_remote_renamed_old_deleted_new_nonconforming",
        remote_renamed,
    ),
    ("poll_nonconforming_tree", nonconforming_tree),
    ("poll_malformed_family", malformed_family),
    ("poll_outside_family", outside_family),
    (
        "poll_preexisting_branch_blocks_materialization",
        branch_blocked,
    ),
    ("poll_preexisting_path_blocks_materialization", path_blocked),
    ("poll_history_unknown_after_registry_loss", history_unknown),
    (
        "poll_ignored_directory_collision_preserved",
        directory_collision,
    ),
    ("poll_foreign_worktree_ownership_refused", foreign_ownership),
    ("poll_dot_document_nonconforming", dot_document),
    ("poll_policy_disabled", policy_disabled),
    ("poll_policy_paused", policy_paused),
    ("poll_policy_recovery_suspended", policy_suspended),
    ("poll_policy_not_due", policy_not_due),
    ("poll_unreachable_backoff_cap_and_explicit_failure", backoff),
    ("poll_rejected_key_attention_without_backoff", rejected_key),
    ("poll_unapproved_host_without_backoff", unapproved_host),
    (
        "poll_locked_blocked_session_no_automatic_prompt",
        locked_session,
    ),
    (
        "poll_automatic_advertisement_yields_to_manual",
        advertisement_yield,
    ),
    (
        "poll_automatic_in_transfer_yields_to_manual",
        transfer_yield,
    ),
    (
        "poll_explicit_advertisement_makes_manual_busy",
        explicit_busy,
    ),
    ("poll_local_save_requests_automatic_yield", local_save_yield),
    (
        "poll_retained_real_divergence_conflict_blocks_both_invocations",
        retained_conflict,
    ),
    ("poll_cancel_before_transport", cancel_before_transport),
    (
        "poll_cancel_after_advertisement",
        cancel_after_advertisement,
    ),
    (
        "poll_cancel_between_observations",
        cancel_between_observations,
    ),
    ("poll_cancel_before_batch", cancel_before_batch),
    ("poll_cancel_after_batch", cancel_after_batch),
    ("poll_cancel_before_local_mutation", cancel_before_local),
    ("poll_cancel_before_discovery", cancel_before_discovery),
    ("poll_public_cancel_held_advertisement", public_cancel),
    ("poll_partial_batch_yield_report_and_restart", partial_batch),
    (
        "poll_disconnect_download_next_poll_finishes",
        disconnect_download,
    ),
    (
        "poll_disconnect_partial_download_automatic_backoff",
        disconnect_download_automatic,
    ),
    ("poll_upload_pack_counter_fragmentation", upload_counter),
    (
        "poll_failure_after_tracking_writes_restart",
        failure_after_tracking,
    ),
    ("poll_batch_commit_failure_atomic_restart", batch_failure),
    (
        "poll_fast_forward_before_disposition_failure_restart",
        disposition_failure,
    ),
    (
        "poll_materialization_intent_failure_restart",
        intent_failure,
    ),
    (
        "poll_materialization_branch_failure_restart",
        branch_failure,
    ),
    (
        "poll_materialization_worktree_failure_restart",
        worktree_failure,
    ),
    (
        "poll_materialization_branch_record_failure_retains_ambiguity",
        branch_record_failure,
    ),
    #[cfg(target_os = "linux")]
    ("poll_failed_checkout_retains_partial_d2", failed_checkout),
    (
        "poll_completion_transaction_failure_restart",
        completion_failure,
    ),
    (
        "poll_after_completion_failure_index_only_recovery",
        after_completion_failure,
    ),
    ("poll_refresh_failure_index_only_recovery", refresh_failure),
    (
        "poll_held_publication_remote_rename_fences",
        config_remote_rename,
    ),
    ("poll_held_endpoint_change_fences", config_endpoint),
    ("poll_held_selected_key_change_fences", config_key),
    (
        "poll_tracking_changed_before_target_fences",
        tracking_changed,
    ),
    (
        "poll_recovery_database_wal_backup_privacy",
        database_privacy,
    ),
    ("poll_output_capture_negative_controls", capture_controls),
    (
        "poll_snapshot_detects_type_only_changes",
        snapshot_type_control,
    ),
    (
        "poll_snapshot_detects_permission_only_changes",
        snapshot_permission_control,
    ),
    (
        "poll_snapshot_detects_name_content_boundary_changes",
        snapshot_boundary_control,
    ),
];

struct Provider {
    prompts: Arc<AtomicUsize>,
    cancelled: bool,
}
impl SessionCredentialProvider for Provider {
    fn request_passphrase(&mut self, _: &UnlockRequest) -> PassphraseResponse {
        self.prompts.fetch_add(1, Ordering::SeqCst);
        if self.cancelled {
            PassphraseResponse::Cancelled
        } else {
            PassphraseResponse::Supplied(SecretPassphrase::new(PASSWORD.into()).unwrap())
        }
    }
}
fn session() -> SessionCredentials<Provider> {
    SessionCredentials::new(Provider {
        prompts: Arc::new(AtomicUsize::new(0)),
        cancelled: false,
    })
}
fn reference(item: &str) -> String {
    format!(
        "refs/heads/manyhands/{}/{item}",
        if item == T { "ticket" } else { "document" }
    )
}
fn tracking(reference: &str) -> String {
    reference.replacen("refs/heads/", "refs/remotes/origin/", 1)
}
fn kind(item: &str) -> AuthoringKind {
    if item == T {
        AuthoringKind::Ticket
    } else {
        AuthoringKind::Document
    }
}
fn source(item: &str, title: &str, body: &str) -> String {
    let item = if item == T {
        canonical::CanonicalItem::Ticket(canonical::Ticket {
            id: item.parse().unwrap(),
            title: title.into(),
            body: body.into(),
            ticket_type: "task".into(),
            status: "open".into(),
            project: None,
            team: None,
            closed_at: None,
            closed_by: None,
            unknown: serde_yaml::Mapping::new(),
        })
    } else {
        canonical::CanonicalItem::Document(canonical::Document {
            id: item.parse().unwrap(),
            title: title.into(),
            body: body.into(),
            unknown: serde_yaml::Mapping::new(),
        })
    };
    canonical::serialize_item(&item).unwrap()
}
fn comment(item: &str) -> String {
    canonical::serialize_item(&canonical::CanonicalItem::Comment(canonical::Comment {
        id: comment_id(item).parse().unwrap(),
        item_id: item.parse().unwrap(),
        parent_id: None,
        created_at: time::OffsetDateTime::from_unix_timestamp(1_767_225_600).unwrap(),
        body: BODY.into(),
        unknown: serde_yaml::Mapping::new(),
    }))
    .unwrap()
}
type Change<'a> = (&'a str, Option<(&'a [u8], u32)>);
fn commit(
    repo: &git2::Repository,
    base: git2::Oid,
    changes: &[Change<'_>],
) -> Result<git2::Oid, FixtureError> {
    let parent = fixed(repo.find_commit(base))?;
    let mut index = fixed(git2::Index::new())?;
    fixed(index.read_tree(&fixed(parent.tree())?))?;
    for (path, change) in changes {
        match change {
            None => fixed(index.remove(Path::new(path), 0))?,
            Some((bytes, mode)) => fixed(index.add(&git2::IndexEntry {
                ctime: git2::IndexTime::new(0, 0),
                mtime: git2::IndexTime::new(0, 0),
                dev: 0,
                ino: 0,
                mode: *mode,
                uid: 0,
                gid: 0,
                file_size: bytes.len() as u32,
                id: fixed(repo.blob(bytes))?,
                flags: 0,
                flags_extended: 0,
                path: path.as_bytes().to_vec(),
            }))?,
        }
    }
    let tree = fixed(repo.find_tree(fixed(index.write_tree_to(repo))?))?;
    let sig = fixed(git2::Signature::now("Fixture", "fixture@example.invalid"))?;
    fixed(repo.commit(None, &sig, &sig, "owned fixture commit", &tree, &[&parent]))
}
fn install(repo: &git2::Repository, reference: &str, oid: git2::Oid) -> Result<(), FixtureError> {
    fixed(repo.checkout_tree(
        &fixed(repo.find_object(oid, None))?,
        Some(git2::build::CheckoutBuilder::new().safe()),
    ))?;
    fixed(repo.reference(reference, oid, true, "owned fixture update"))?;
    Ok(())
}
fn callbacks(server: &SshRemoteFixture) -> git2::RemoteCallbacks<'_> {
    let mut cb = git2::RemoteCallbacks::new();
    cb.credentials(|_, user, _| {
        git2::Cred::ssh_key(
            user.unwrap_or("fixture"),
            None,
            server.client_key_path(),
            Some(PASSWORD),
        )
    });
    let public = server.host_public_key().to_bytes().unwrap();
    cb.certificate_check(move |cert, _| {
        if cert.as_hostkey().and_then(|h| h.hostkey()) == Some(public.as_slice()) {
            Ok(git2::CertificateCheckStatus::CertificateOk)
        } else {
            Err(git2::Error::from_str("fixture host mismatch"))
        }
    });
    cb
}
fn clone_owned(server: &SshRemoteFixture, root: &Path) -> Result<git2::Repository, FixtureError> {
    let mut fetch = git2::FetchOptions::new();
    fetch.remote_callbacks(callbacks(server));
    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fetch);
    let repo = fixed(builder.clone(&server.url(), root))?;
    fixed(fixed(repo.config())?.set_str("user.name", "Fixture"))?;
    fixed(fixed(repo.config())?.set_str("user.email", "fixture@example.invalid"))?;
    Ok(repo)
}
fn push_peer(
    repo: &git2::Repository,
    server: &SshRemoteFixture,
    reference: &str,
) -> Result<(), FixtureError> {
    let bare = fixed(git2::Repository::open_bare(server.repository_path()))?;
    let old = bare.refname_to_id(reference).unwrap_or(git2::Oid::zero());
    let new = fixed(repo.refname_to_id(reference))?;
    let n = server.receive_updates().len();
    let mut remote = fixed(repo.remote_anonymous(&server.url()))?;
    let mut options = git2::PushOptions::new();
    options.remote_callbacks(callbacks(server));
    fixed(remote.push(&[&format!("{reference}:{reference}")], Some(&mut options)))?;
    server.wait_for_receive_update(
        n,
        &ReceiveUpdate {
            reference: reference.into(),
            old_oid: old,
            new_oid: new,
            accepted: true,
        },
    )
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
    fn worktree(&self, item: &str) -> PathBuf {
        self.root.join(".manyhands/worktrees").join(item)
    }
    fn db(&self) -> Result<rusqlite::Connection, FixtureError> {
        fixed(rusqlite::Connection::open(self.data.join(REGISTRY_FILE)))
    }
    fn reopen(&self) -> Result<RepositoryService, FixtureError> {
        fixed(RepositoryService::open_at(&self.data))
    }
}
/// Two independent clones, data stores and services, never shared local Git state.
struct Pair {
    server: SshRemoteFixture,
    a: Side,
    b: Side,
    _dir: tempfile::TempDir,
    probes: Vec<Vec<u8>>,
    metadata_probes: Vec<Vec<u8>>,
}
impl Pair {
    fn new() -> Result<Self, FixtureError> {
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
        let dir = fixed(tempfile::Builder::new().prefix("p").tempdir())?;
        let mut probes = vec![
            plain.clone(),
            encrypted.as_bytes().to_vec(),
            PASSWORD.as_bytes().to_vec(),
            BODY.as_bytes().to_vec(),
            TITLE.as_bytes().to_vec(),
            server.url().into_bytes(),
            BAD_REF.as_bytes().to_vec(),
        ];
        probes.extend(
            plain
                .split(|b| *b == b'\n')
                .filter(|s| s.len() > 40)
                .map(<[u8]>::to_vec),
        );
        let mut metadata_probes = probes.clone();
        metadata_probes.extend([
            DOCUMENT_TITLE.as_bytes().to_vec(),
            TICKET_TITLE.as_bytes().to_vec(),
        ]);
        // Captured output never has an index-content exception.
        ssh_privacy::save(&metadata_probes)?;
        let ar = dir.path().join("a");
        let ad = dir.path().join("x");
        let repo = clone_owned(&server, &ar)?;
        let sa = fixed(RepositoryService::open_at(&ad))?;
        fixed(sa.enable(EnableRepositoryRequest {
            root: ar.clone(),
            primary_branch: "main".into(),
            identity: None,
            operation_id: OperationId::new(),
        }))?;
        let mut config = fixed(std::fs::read_to_string(ar.join(".manyhands/config.toml")))?;
        config.push_str("publication_remote = \"origin\"\n");
        let oid = commit(
            &repo,
            fixed(repo.refname_to_id(MAIN))?,
            &[
                (".manyhands/config.toml", Some((config.as_bytes(), MODE))),
                (
                    TP,
                    Some((source(T, "Fixture ticket", "seed\n").as_bytes(), MODE)),
                ),
                (
                    DP,
                    Some((source(D, "Fixture document", "seed\n").as_bytes(), MODE)),
                ),
            ],
        )?;
        install(&repo, MAIN, oid)?;
        push_peer(&repo, &server, MAIN)?;
        let br = dir.path().join("b");
        let bd = dir.path().join("y");
        clone_owned(&server, &br)?;
        let sb = fixed(RepositoryService::open_at(&bd))?;
        fixed(sb.enable(EnableRepositoryRequest {
            root: br.clone(),
            primary_branch: "main".into(),
            identity: None,
            operation_id: OperationId::new(),
        }))?;
        for service in [&sa, &sb] {
            let RegisterSharedKeyOutcome::Registered(key) =
                fixed(service.register_shared_key(RegisterSharedKeyRequest {
                    label: "poll fixture".into(),
                    ownership: SharedKeyOwnership::Imported,
                    private_key_path: server.client_key_path().into(),
                    public_key_path: None,
                }))?
            else {
                return Err(FixtureError);
            };
            fixed(service.select_shared_key(key.id))?;
        }
        fixed(sb.refresh_repository(RefreshRepositoryRequest {
            root: br.clone(),
            operation_id: OperationId::new(),
        }))?;
        Ok(Self {
            server,
            a: Side {
                service: sa,
                root: ar,
                data: ad,
            },
            b: Side {
                service: sb,
                root: br,
                data: bd,
            },
            _dir: dir,
            probes,
            metadata_probes,
        })
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
    fn request(&self, invocation: RemotePollInvocation) -> PollPublicationRemoteRequest {
        PollPublicationRemoteRequest {
            root: self.b.root.clone(),
            operation_id: OperationId::new(),
            invocation,
            approval: Some(self.approval()),
            restart: false,
        }
    }
    fn poll_with(
        &self,
        service: &RepositoryService,
        request: PollPublicationRemoteRequest,
        session: &mut SessionCredentials<Provider>,
    ) -> Result<RemotePollResult, RemotePollError> {
        let n = self.no_push_baseline().expect("settled peer phase");
        let result = service.poll_publication_remote(request, session);
        self.no_push_since(n).expect("poll never receives");
        self.redacted(&format!("{result:?}"))
            .expect("redacted poll result");
        self.privacy().expect("private poll recovery");
        result
    }
    fn poll(&self) -> Result<RemotePollReport, FixtureError> {
        let (r, pending) = report(fixed(self.poll_with(
            &self.b.service,
            self.request(RemotePollInvocation::Explicit),
            &mut session(),
        ))?)?;
        assert!(!pending);
        assert_eq!(r.completion(), PollCompletion::Completed);
        Ok(r)
    }
    fn no_push_baseline(&self) -> Result<(usize, usize, usize), FixtureError> {
        self.server.wait_for_helper_audits()?;
        Ok((
            self.server.receive_advertisements(),
            self.server.receive_updates().len(),
            self.server.commands().len(),
        ))
    }
    fn no_push_since(&self, n: (usize, usize, usize)) -> Result<(), FixtureError> {
        self.server.wait_for_helper_audits()?;
        assert_eq!(self.server.receive_advertisements(), n.0);
        assert_eq!(self.server.receive_updates().len(), n.1);
        assert!(
            self.server.commands()[n.2..]
                .iter()
                .all(|c| !c.starts_with(b"git-receive-pack"))
        );
        Ok(())
    }
    fn redacted(&self, text: &str) -> Result<(), FixtureError> {
        ssh_privacy::clean(text.as_bytes(), &self.metadata_probes)
    }
    fn privacy(&self) -> Result<(), FixtureError> {
        ssh_privacy::save(&self.metadata_probes)?;
        for side in [&self.a, &self.b] {
            let db = side.db()?;
            scan_recovery_rows(&db, &self.metadata_probes)?;
            // Accepted titles are legitimate discovered_items content. All other
            // canaries, including the rejected title, remain whole-store probes.
            let inventory = ssh_privacy::scan(&side.data, &self.probes)?;
            assert!(
                inventory
                    .iter()
                    .any(|(p, n)| p.file_name().is_some_and(|f| f == REGISTRY_FILE) && *n > 0)
            );
        }
        Ok(())
    }
    fn context(&self, item: &str) -> Result<(), FixtureError> {
        fixed(self.b.service.prepare_context(AuthoringTarget {
            root: self.b.root.clone(),
            kind: kind(item),
            item_id: item.parse().unwrap(),
            intent: ContextIntent::Edit,
            operation_id: OperationId::new(),
        }))?;
        Ok(())
    }
    fn publish(
        &self,
        reference: &str,
        base: git2::Oid,
        changes: &[Change<'_>],
    ) -> Result<git2::Oid, FixtureError> {
        let repo = self.a.repo()?;
        let oid = commit(&repo, base, changes)?;
        if reference == MAIN {
            install(&repo, reference, oid)?;
        } else {
            fixed(repo.reference(reference, oid, false, "owned peer branch"))?;
        }
        push_peer(&repo, &self.server, reference)?;
        Ok(oid)
    }
    fn base(&self) -> Result<git2::Oid, FixtureError> {
        fixed(self.b.repo()?.refname_to_id(MAIN))
    }
    fn finish(&self) -> Result<(), FixtureError> {
        self.privacy()
    }
}
fn scan_recovery_rows(db: &rusqlite::Connection, probes: &[Vec<u8>]) -> Result<(), FixtureError> {
    let mut stmt = fixed(db.prepare(r"SELECT name FROM sqlite_master WHERE type='table' AND (name LIKE 'remote\_%' ESCAPE '\' OR name='operation_records') ORDER BY name"))?;
    let tables = fixed(
        fixed(stmt.query_map([], |r| r.get::<_, String>(0)))?.collect::<Result<Vec<_>, _>>(),
    )?;
    for required in [
        "remote_poll_dispositions",
        "remote_poll_materializations",
        "remote_operation_records",
        "remote_ref_observations",
    ] {
        assert!(tables.iter().any(|t| t == required));
    }
    for table in tables {
        let mut statement = fixed(db.prepare(&format!("SELECT * FROM {table}")))?;
        let columns = statement.column_count();
        let mut rows = fixed(statement.query([]))?;
        while let Some(row) = fixed(rows.next())? {
            for column in 0..columns {
                if let rusqlite::types::ValueRef::Text(bytes)
                | rusqlite::types::ValueRef::Blob(bytes) = fixed(row.get_ref(column))?
                {
                    ssh_privacy::clean(bytes, probes)?;
                }
            }
        }
    }
    Ok(())
}
fn report(result: RemotePollResult) -> Result<(RemotePollReport, bool), FixtureError> {
    let (outcome, pending) = match result {
        RemotePollResult::Complete(o) => (o, false),
        RemotePollResult::IndexPending(p) => (p.authoritative, true),
    };
    match outcome {
        RemotePollOutcome::Polled(r) => Ok((r, pending)),
        _ => Err(FixtureError),
    }
}
fn row<'a>(r: &'a RemotePollReport, reference: &str) -> &'a PollTargetReport {
    r.targets()
        .iter()
        .find(|t| t.target().is_some_and(|t| t.remote_ref() == reference))
        .expect("named poll target")
}
fn guidance(d: PollDisposition) -> &'static str {
    match d {
        PollDisposition::Dirty => "Save or otherwise handle local changes before polling again.",
        PollDisposition::Conflicted => {
            "Resolve the conflict or in-progress Git operation before polling again."
        }
        PollDisposition::LocalAhead => "Publish local commits with deliberate synchronization.",
        PollDisposition::Diverged => "Resolve divergent history with deliberate synchronization.",
        PollDisposition::Inaccessible => {
            "Restore the expected checked-out branch and registered worktree before polling again."
        }
        PollDisposition::RemotelyDeleted => {
            "The remote context is absent; local state was preserved."
        }
        PollDisposition::HistoryUnknown => {
            "Publication history is unknown; inspect remote state before proceeding."
        }
        PollDisposition::Nonconforming => {
            "Repair the remote context's canonical tree before materialization."
        }
        PollDisposition::BlockedByLocalState => {
            "Inspect the existing local branch or path before materialization."
        }
        PollDisposition::Malformed => "Repair the remote context's branch name.",
        PollDisposition::MaterializationIncomplete => {
            "Inspect and repair the retained partial materialization."
        }
        _ => panic!("exceptional guidance only"),
    }
}
fn unchanged(r: &RemotePollReport, reference: &str, d: PollDisposition) {
    assert_eq!(row(r, reference).disposition(), d);
    assert_eq!(d.guidance(), guidance(d));
}
// Length/type framing keeps structurally distinct entries distinct; hashes keep
// private paths and content out of failure diagnostics.
fn hash_bytes(hash: &mut blake3::Hasher, bytes: &[u8]) {
    hash.update(&(bytes.len() as u64).to_le_bytes());
    hash.update(bytes);
}
fn hash_permissions(hash: &mut blake3::Hasher, metadata: &std::fs::Metadata) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        hash.update(b"unix-mode");
        hash.update(&(metadata.permissions().mode() & 0o7777).to_le_bytes());
    }
    #[cfg(not(unix))]
    {
        hash.update(b"readonly");
        hash.update(&[u8::from(metadata.permissions().readonly())]);
    }
}
fn files(path: &Path, hash: &mut blake3::Hasher) -> Result<(), FixtureError> {
    let metadata = fixed(std::fs::symlink_metadata(path))?;
    if metadata.file_type().is_symlink() {
        hash.update(b"link");
        hash_permissions(hash, &metadata);
        hash_bytes(
            hash,
            fixed(std::fs::read_link(path))?
                .as_os_str()
                .as_encoded_bytes(),
        );
    } else if metadata.is_dir() {
        hash.update(b"directory");
        hash_permissions(hash, &metadata);
        let mut entries = fixed(std::fs::read_dir(path))?
            .map(|e| fixed(e).map(|e| e.path()))
            .collect::<Result<Vec<_>, _>>()?;
        entries.retain(|p| {
            !p.file_name()
                .is_some_and(|n| n == ".git" || n == "worktrees")
        });
        entries.sort();
        hash.update(&(entries.len() as u64).to_le_bytes());
        for entry in entries {
            hash_bytes(
                hash,
                entry.file_name().ok_or(FixtureError)?.as_encoded_bytes(),
            );
            files(&entry, hash)?;
        }
    } else if metadata.is_file() {
        hash.update(b"file");
        hash_permissions(hash, &metadata);
        hash_bytes(hash, &fixed(std::fs::read(path))?);
    } else {
        // Never open a device/FIFO/socket to obtain a snapshot.
        hash.update(b"other");
        hash_permissions(hash, &metadata);
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            hash.update(&metadata.mode().to_le_bytes());
        }
    }
    Ok(())
}
fn snapshot(side: &Side) -> Result<[u8; 32], FixtureError> {
    let repo = side.repo()?;
    let mut hash = blake3::Hasher::new();
    files(&repo.commondir().join("config"), &mut hash)?;
    let mut refs = fixed(repo.references())?
        .map(|r| {
            let r = fixed(r)?;
            Ok((fixed(r.name().ok_or(FixtureError))?.to_owned(), r.target()))
        })
        .collect::<Result<Vec<_>, FixtureError>>()?;
    refs.sort();
    for (name, oid) in refs {
        if !name.starts_with("refs/remotes/") {
            hash_bytes(&mut hash, name.as_bytes());
            hash_bytes(&mut hash, format!("{oid:?}").as_bytes());
        }
    }
    let mut paths = vec![side.root.clone()];
    for name in fixed(repo.worktrees())?.iter().flatten() {
        let w = fixed(repo.find_worktree(name))?;
        hash_bytes(&mut hash, name.as_bytes());
        hash_bytes(&mut hash, w.path().as_os_str().as_encoded_bytes());
        paths.push(w.path().into());
        files(&repo.commondir().join("worktrees").join(name), &mut hash)?;
    }
    for path in paths {
        if !path.exists() {
            hash.update(b"absent");
            continue;
        }
        let linked = fixed(git2::Repository::open(&path))?;
        for name in ["HEAD", "index", "MERGE_HEAD", "MERGE_MSG", "MERGE_MODE"] {
            hash_bytes(&mut hash, name.as_bytes());
            let file = linked.path().join(name);
            match std::fs::symlink_metadata(&file) {
                Ok(_) => files(&file, &mut hash)?,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    hash.update(b"missing");
                }
                Err(_) => return Err(FixtureError),
            }
        }
        if let Ok(statuses) = linked.statuses(None) {
            for status in statuses.iter() {
                hash_bytes(&mut hash, status.path_bytes());
                hash.update(&status.status().bits().to_le_bytes());
            }
        } else {
            hash.update(b"status unavailable");
        }
        if path.join(".git").is_file() {
            files(&path.join(".git"), &mut hash)?;
        }
        files(&path, &mut hash)?;
    }
    // Include deterministic path obstructions even when not registered.
    let base = side.root.join(".manyhands/worktrees");
    if base.is_dir() {
        files(&base, &mut hash)?;
    }
    // Missing/foreign-ownership fixtures park the original checkout beside the
    // clone. Its retained bytes are part of the preservation assertion too.
    let parked = side.root.parent().ok_or(FixtureError)?.join("m");
    if parked.is_dir() {
        files(&parked, &mut hash)?;
        files(&parked.join(".git"), &mut hash)?;
    }
    Ok(*hash.finalize().as_bytes())
}
fn objects(repo: &git2::Repository) -> Result<Vec<git2::Oid>, FixtureError> {
    let mut ids = Vec::new();
    fixed(fixed(repo.odb())?.foreach(|oid| {
        ids.push(*oid);
        true
    }))?;
    ids.sort();
    Ok(ids)
}
/// Controls use ignored, unregistered fixture entries: Git status cannot make
/// the filesystem snapshot look stronger by independently detecting the edit.
fn snapshot_control_root(p: &Pair) -> Result<PathBuf, FixtureError> {
    let root = p.b.root.join(".manyhands/worktrees/s");
    fixed(std::fs::create_dir_all(&root))?;
    Ok(root)
}
fn snapshot_type_control() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let n = p.no_push_baseline()?;
    let root = snapshot_control_root(&p)?;
    let entry = root.join("entry");
    fixed(std::fs::write(&entry, b""))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fixed(std::fs::set_permissions(
            &entry,
            std::fs::Permissions::from_mode(0o700),
        ))?;
    }
    let permissions = fixed(std::fs::metadata(&entry))?.permissions();
    let before = snapshot(&p.b)?;
    fixed(std::fs::rename(&entry, p._dir.path().join("type-file")))?;
    fixed(std::fs::create_dir(&entry))?;
    fixed(std::fs::set_permissions(&entry, permissions))?;
    let after = snapshot(&p.b)?;
    assert_ne!(before, after, "empty file and empty directory must differ");
    p.no_push_since(n)?;
    p.finish()
}
fn snapshot_permission_control() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let n = p.no_push_baseline()?;
    let entry = snapshot_control_root(&p)?.join("entry");
    fixed(std::fs::write(&entry, b"unchanged fixture bytes"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fixed(std::fs::set_permissions(
            &entry,
            std::fs::Permissions::from_mode(0o644),
        ))?;
    }
    let original = fixed(std::fs::metadata(&entry))?.permissions();
    let before = snapshot(&p.b)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // No execute/status/content change: both states are writable to owner.
        fixed(std::fs::set_permissions(
            &entry,
            std::fs::Permissions::from_mode(0o600),
        ))?;
    }
    #[cfg(not(unix))]
    {
        let mut changed = original.clone();
        changed.set_readonly(!original.readonly());
        fixed(std::fs::set_permissions(&entry, changed))?;
    }
    let after = snapshot(&p.b)?;
    fixed(std::fs::set_permissions(&entry, original))?;
    assert_ne!(before, after, "permission-only change must differ");
    assert_eq!(snapshot(&p.b)?, before);
    p.no_push_since(n)?;
    p.finish()
}
fn snapshot_boundary_control() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let n = p.no_push_baseline()?;
    let root = snapshot_control_root(&p)?;
    fixed(std::fs::write(root.join("a"), b"bc"))?;
    let before = snapshot(&p.b)?;
    fixed(std::fs::rename(root.join("a"), root.join("ab")))?;
    fixed(std::fs::write(root.join("ab"), b"c"))?;
    let after = snapshot(&p.b)?;
    assert_ne!(before, after, "name/content field boundaries must differ");
    p.no_push_since(n)?;
    p.finish()
}
fn clean_checkout(p: &Pair, item: Option<&str>, oid: git2::Oid) -> Result<(), FixtureError> {
    let reference = item.map(reference).unwrap_or_else(|| MAIN.into());
    let path = item
        .map(|i| p.b.worktree(i))
        .unwrap_or_else(|| p.b.root.clone());
    let repo = fixed(git2::Repository::open(&path))?;
    assert_eq!(fixed(repo.head())?.name(), Some(reference.as_str()));
    assert_eq!(fixed(repo.head())?.target(), Some(oid));
    assert_eq!(fixed(repo.refname_to_id(&tracking(&reference)))?, oid);
    assert_eq!(
        fixed(fixed(repo.index())?.write_tree())?,
        fixed(repo.find_commit(oid))?.tree_id()
    );
    let statuses = fixed(repo.statuses(Some(git2::StatusOptions::new().include_ignored(false))))?;
    assert!(statuses.is_empty());
    for file in ["fixture.txt", if item == Some(T) { TP } else { DP }] {
        let tree = fixed(fixed(repo.find_commit(oid))?.tree())?;
        let entry = fixed(tree.get_path(Path::new(file)))?;
        assert!(
            fixed(std::fs::read(path.join(file)))? == fixed(repo.find_blob(entry.id()))?.content()
        );
    }
    Ok(())
}
fn clean_case(items: &[Option<&str>]) -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let base = p.base()?;
    let sentinels = stale_tracking(&p, base)?;
    let mut tips = Vec::new();
    for item in items {
        if let Some(item) = item {
            p.context(item)?;
        }
        let reference = item.map(reference).unwrap_or_else(|| MAIN.into());
        let tip = p.publish(
            &reference,
            base,
            &[(
                "fixture.txt",
                Some((format!("incoming {reference}\n").as_bytes(), MODE)),
            )],
        )?;
        tips.push((*item, reference, tip));
    }
    let r = p.poll()?;
    assert_eq!(sentinel_states(&p.b.repo()?)?, sentinels);
    for (item, reference, tip) in &tips {
        assert_eq!(
            row(&r, reference).disposition(),
            PollDisposition::FastForwarded
        );
        assert_eq!(row(&r, reference).before_oid(), Some(base));
        assert_eq!(row(&r, reference).after_oid(), Some(*tip));
        clean_checkout(&p, *item, *tip)?;
    }
    let before = snapshot(&p.b)?;
    let odb = objects(&p.b.repo()?)?;
    let refs = all_refs(&p.b.repo()?)?;
    let again = p.poll()?;
    assert!(
        again
            .targets()
            .iter()
            .all(|t| t.disposition() == PollDisposition::Current)
    );
    assert_eq!(snapshot(&p.b)?, before);
    assert_eq!(objects(&p.b.repo()?)?, odb);
    assert_eq!(all_refs(&p.b.repo()?)?, refs);
    for (item, _, tip) in tips {
        clean_checkout(&p, item, tip)?;
    }
    p.finish()
}
fn primary_fast_forward() -> Result<(), FixtureError> {
    clean_case(&[None])
}
fn document_fast_forward() -> Result<(), FixtureError> {
    clean_case(&[Some(D)])
}
fn ticket_fast_forward() -> Result<(), FixtureError> {
    clean_case(&[Some(T)])
}
fn all_three_fast_forward() -> Result<(), FixtureError> {
    clean_case(&[None, Some(D), Some(T)])
}
fn already_current() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let base = p.base()?;
    let sentinels = stale_tracking(&p, base)?;
    for item in [D, T] {
        p.context(item)?;
        let peer = p.a.repo()?;
        fixed(peer.reference(&reference(item), base, false, "owned current peer branch"))?;
        push_peer(&peer, &p.server, &reference(item))?;
    }
    // Only this initial fetch may introduce the two context tracking refs.
    let initial = p.poll()?;
    assert_eq!(sentinel_states(&p.b.repo()?)?, sentinels);
    assert_eq!(initial.targets().len(), 3);
    assert!(
        initial
            .targets()
            .iter()
            .all(|t| t.disposition() == PollDisposition::Current)
    );
    let before = snapshot(&p.b)?;
    let odb = objects(&p.b.repo()?)?;
    let refs = all_refs(&p.b.repo()?)?;
    for _ in 0..2 {
        let r = p.poll()?;
        assert_eq!(r.targets().len(), 3);
        assert!(
            r.targets()
                .iter()
                .all(|t| t.disposition() == PollDisposition::Current)
        );
        assert_eq!(snapshot(&p.b)?, before);
        assert_eq!(objects(&p.b.repo()?)?, odb);
        assert_eq!(all_refs(&p.b.repo()?)?, refs);
        for item in [None, Some(D), Some(T)] {
            clean_checkout(&p, item, base)?;
        }
    }
    p.finish()
}

fn publish_new(p: &Pair, item: &str) -> Result<git2::Oid, FixtureError> {
    let cp = format!(".manyhands/comments/{item}/{}.md", comment_id(item));
    p.publish(
        &reference(item),
        p.base()?,
        &[
            (
                if item == T { TP } else { DP },
                Some((source(item, accepted_title(item), BODY).as_bytes(), MODE)),
            ),
            (&cp, Some((comment(item).as_bytes(), MODE))),
        ],
    )
}
fn accepted_title(item: &str) -> &'static str {
    if item == T {
        TICKET_TITLE
    } else {
        DOCUMENT_TITLE
    }
}
fn comment_id(item: &str) -> &'static str {
    if item == T { C } else { DOCUMENT_COMMENT }
}
fn discovered_once(p: &Pair, item: &str, tip: git2::Oid) -> Result<(), FixtureError> {
    let repo = p.b.repo()?;
    assert_eq!(
        fixed(repo.worktrees())?
            .iter()
            .flatten()
            .filter(|s| *s == item)
            .count(),
        1
    );
    assert_eq!(fixed(repo.refname_to_id(&reference(item)))?, tip);
    assert!(
        fixed(repo.config())?
            .get_string(&format!(
                "branch.{}.remote",
                reference(item).trim_start_matches("refs/heads/")
            ))
            .is_err()
    );
    clean_checkout(p, Some(item), tip)?;
    let snap = fixed(p.b.service.repository_snapshot(&p.b.root))?;
    let expected = fixed(std::fs::canonicalize(p.b.worktree(item)))?;
    let found = snap
        .items
        .iter()
        .filter(|i| {
            i.id.to_string() == item
                && std::fs::canonicalize(&i.context).is_ok_and(|context| context == expected)
        })
        .collect::<Vec<_>>();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].title, accepted_title(item));
    assert_eq!(found[0].comments.len(), 1);
    assert_eq!(found[0].comments[0].id.to_string(), comment_id(item));
    assert_eq!(fixed(p.b.db()?.query_row("SELECT count(*) FROM remote_poll_materializations WHERE item_id=?1 AND step='complete'",[item],|r|r.get::<_,i64>(0)))?,1);
    // RemoteSnapshot is diagnostic/recovery metadata, unlike the canonical
    // RepositorySnapshot.items content inspected above.
    p.redacted(&format!(
        "{:?}",
        fixed(p.b.service.remote_snapshot(&p.b.root))?
    ))?;
    Ok(())
}
fn materialize_once(item: &str) -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let tip = publish_new(&p, item)?;
    let r = p.poll()?;
    assert_eq!(
        row(&r, &reference(item)).disposition(),
        PollDisposition::Materialized
    );
    discovered_once(&p, item, tip)?;
    let before = snapshot(&p.b)?;
    let refs = all_refs(&p.b.repo()?)?;
    let intents = fixed(p.b.db()?.query_row(
        "SELECT operation_ulid,target_oid,step FROM remote_poll_materializations",
        [],
        |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        },
    ))?;
    for service in [&p.b.service, &p.b.reopen()?, &p.b.reopen()?] {
        let (r, pending) = report(fixed(p.poll_with(
            service,
            p.request(RemotePollInvocation::Explicit),
            &mut session(),
        ))?)?;
        assert!(!pending);
        assert_eq!(
            row(&r, &reference(item)).disposition(),
            PollDisposition::Current
        );
        assert_eq!(snapshot(&p.b)?, before);
        discovered_once(&p, item, tip)?;
        assert_eq!(all_refs(&p.b.repo()?)?, refs);
        assert_eq!(
            fixed(p.b.db()?.query_row(
                "SELECT operation_ulid,target_oid,step FROM remote_poll_materializations",
                [],
                |r| Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?
                ))
            ))?,
            intents
        );
    }
    p.finish()
}
fn materialize_document_once() -> Result<(), FixtureError> {
    materialize_once(D)
}
fn materialize_ticket_once() -> Result<(), FixtureError> {
    materialize_once(T)
}
fn concurrent_materialization_once() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let tip = publish_new(&p, T)?;
    let second = p.b.reopen()?;
    let gate = p.server.hold_advertisement()?;
    let baseline = p.no_push_baseline()?;
    let operations = count(&p.b, "remote_operation_records")?;
    std::thread::scope(|scope| -> Result<(), FixtureError> {
        let request = p.request(RemotePollInvocation::Explicit);
        let worker = scope.spawn(|| p.b.service.poll_publication_remote(request, &mut session()));
        gate.wait_until_held()?;
        assert!(matches!(
            second
                .poll_publication_remote(p.request(RemotePollInvocation::Explicit), &mut session()),
            Err(RemotePollError::Busy)
        ));
        assert_eq!(count(&p.b, "remote_operation_records")?, operations + 1);
        gate.release()?;
        let result = fixed(fixed(worker.join())?)?;
        p.redacted(&format!("{result:?}"))?;
        assert_eq!(
            row(&report(result)?.0, &reference(T)).disposition(),
            PollDisposition::Materialized
        );
        Ok(())
    })?;
    p.no_push_since(baseline)?;
    discovered_once(&p, T, tip)?;
    let before = snapshot(&p.b)?;
    fixed(p.poll_with(
        &second,
        p.request(RemotePollInvocation::Explicit),
        &mut session(),
    ))?;
    assert_eq!(snapshot(&p.b)?, before);
    p.finish()
}

#[derive(Clone, Copy)]
enum Exception {
    Tracked,
    Staged,
    Untracked,
    Conflicted,
    Ahead,
    Diverged,
    WrongBranch,
    Missing,
    PrimaryWrong,
}
fn exceptional(case: Exception) -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.context(T)?;
    let base = p.base()?;
    let tr = reference(T);
    let local = fixed(git2::Repository::open(p.b.worktree(T)))?;
    let expected = match case {
        Exception::Tracked | Exception::Staged => {
            fixed(std::fs::write(
                p.b.worktree(T).join("fixture.txt"),
                b"local work\n",
            ))?;
            if matches!(case, Exception::Staged) {
                let mut i = fixed(local.index())?;
                fixed(i.add_path(Path::new("fixture.txt")))?;
                fixed(i.write())?;
            }
            PollDisposition::Dirty
        }
        Exception::Untracked => {
            fixed(std::fs::write(
                p.b.worktree(T).join("untracked"),
                b"local work\n",
            ))?;
            PollDisposition::Dirty
        }
        Exception::Ahead | Exception::Diverged => {
            let oid = commit(&local, base, &[("local", Some((b"local\n", MODE)))])?;
            install(&local, &tr, oid)?;
            if matches!(case, Exception::Ahead) {
                PollDisposition::LocalAhead
            } else {
                PollDisposition::Diverged
            }
        }
        Exception::WrongBranch => {
            fixed(local.branch("other", &fixed(local.find_commit(base))?, false))?;
            fixed(local.set_head("refs/heads/other"))?;
            PollDisposition::Inaccessible
        }
        Exception::Missing => {
            drop(local);
            fixed(std::fs::rename(p.b.worktree(T), p._dir.path().join("m")))?;
            PollDisposition::Inaccessible
        }
        Exception::PrimaryWrong => {
            let repo = p.b.repo()?;
            fixed(repo.branch("other", &fixed(repo.find_commit(base))?, false))?;
            fixed(repo.set_head("refs/heads/other"))?;
            PollDisposition::Inaccessible
        }
        Exception::Conflicted => {
            let mine = commit(&local, base, &[("fixture.txt", Some((b"mine\n", MODE)))])?;
            install(&local, &tr, mine)?;
            let theirs = commit(&local, base, &[("fixture.txt", Some((b"theirs\n", MODE)))])?;
            let annotated = fixed(local.find_annotated_commit(theirs))?;
            fixed(local.merge(&[&annotated], None, None))?;
            assert!(fixed(local.index())?.has_conflicts());
            assert_eq!(count(&p.b, "remote_integration_steps")?, 0);
            PollDisposition::Conflicted
        }
    };
    let incoming_ref = if matches!(case, Exception::PrimaryWrong) {
        MAIN
    } else {
        &tr
    };
    let remote = if matches!(case, Exception::Ahead) {
        let peer = p.a.repo()?;
        fixed(peer.reference(&tr, base, false, "owned peer branch"))?;
        push_peer(&peer, &p.server, &tr)?;
        base
    } else {
        p.publish(incoming_ref, base, &[("remote", Some((b"remote\n", MODE)))])?
    };
    let before = snapshot(&p.b)?;
    let r = p.poll()?;
    unchanged(
        &r,
        if matches!(case, Exception::PrimaryWrong) {
            MAIN
        } else {
            &tr
        },
        expected,
    );
    assert_eq!(snapshot(&p.b)?, before);
    assert_eq!(
        fixed(p.b.repo()?.refname_to_id(&tracking(incoming_ref)))?,
        remote
    );
    p.finish()
}
fn tracked_dirty() -> Result<(), FixtureError> {
    exceptional(Exception::Tracked)
}
fn staged_dirty() -> Result<(), FixtureError> {
    exceptional(Exception::Staged)
}
fn untracked_dirty() -> Result<(), FixtureError> {
    exceptional(Exception::Untracked)
}
fn git_conflicted() -> Result<(), FixtureError> {
    exceptional(Exception::Conflicted)
}
fn local_ahead() -> Result<(), FixtureError> {
    exceptional(Exception::Ahead)
}
fn diverged() -> Result<(), FixtureError> {
    exceptional(Exception::Diverged)
}
fn context_wrong_branch() -> Result<(), FixtureError> {
    exceptional(Exception::WrongBranch)
}
fn missing_directory() -> Result<(), FixtureError> {
    exceptional(Exception::Missing)
}

#[cfg(unix)]
fn permission_denied_worktree() -> Result<(), FixtureError> {
    use std::os::unix::fs::PermissionsExt;

    struct RestorePermissions {
        path: PathBuf,
        original: std::fs::Permissions,
    }
    impl Drop for RestorePermissions {
        fn drop(&mut self) {
            // Also restore on a failed assertion or early fixture error. The
            // normal path explicitly checks restoration before result assertions.
            let _ = std::fs::set_permissions(&self.path, self.original.clone());
        }
    }

    let p = Pair::new()?;
    p.context(T)?;
    let base = p.base()?;
    let tr = reference(T);
    let incoming = p.publish(&tr, base, &[("remote", Some((b"remote\n", MODE)))])?;
    {
        let local = fixed(git2::Repository::open(p.b.worktree(T)))?;
        assert_eq!(fixed(local.head())?.target(), Some(base));
        assert_eq!(
            fixed(fixed(local.index())?.write_tree())?,
            fixed(local.find_commit(base))?.tree_id()
        );
        assert!(fixed(local.statuses(None))?.is_empty());
    }
    let before = snapshot(&p.b)?;
    let mut expected_refs = all_refs(&p.b.repo()?)?;
    // Fetch may create/advance this one tracking ref, but no other ref or
    // symbolic target may change while the checked-out context is inaccessible.
    let destination = tracking(&tr);
    if let Some(state) = expected_refs.iter_mut().find(|r| r.name == destination) {
        state.direct = Some(incoming);
        state.symbolic = None;
    } else {
        expected_refs.push(RefState {
            name: destination,
            direct: Some(incoming),
            symbolic: None,
        });
    }
    expected_refs.sort();
    let restore = RestorePermissions {
        path: p.b.worktree(T),
        original: fixed(std::fs::metadata(p.b.worktree(T)))?.permissions(),
    };
    fixed(std::fs::set_permissions(
        &restore.path,
        std::fs::Permissions::from_mode(0o000),
    ))?;
    let denied_read = std::fs::read(restore.path.join(".git"));
    let denied_directory = std::fs::read_dir(&restore.path);
    // Fail rather than silently passing/skipping when root or ambient privileges
    // make chmod ineffective. The guard restores permissions even on this panic.
    assert!(matches!(denied_read, Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied));
    assert!(matches!(denied_directory, Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied));

    let result = p.poll_with(
        &p.b.service,
        p.request(RemotePollInvocation::Explicit),
        &mut session(),
    );
    let denied_after = std::fs::read(restore.path.join(".git"));
    let mode_after = std::fs::metadata(&restore.path).map(|m| m.permissions().mode() & 0o7777);
    fixed(std::fs::set_permissions(
        &restore.path,
        restore.original.clone(),
    ))?;
    drop(restore);

    assert!(matches!(denied_after, Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied));
    assert_eq!(
        fixed(mode_after)?,
        0,
        "poll must preserve denied permissions"
    );
    let (r, _) = report(fixed(result)?)?;
    assert_eq!(r.completion(), PollCompletion::Completed);
    unchanged(&r, &tr, PollDisposition::Inaccessible);
    assert_eq!(snapshot(&p.b)?, before);
    assert_eq!(all_refs(&p.b.repo()?)?, expected_refs);
    assert_eq!(fixed(p.b.repo()?.refname_to_id(&tr))?, base);
    assert!(!p.b.worktree(T).join("remote").exists());
    p.finish()
}

fn primary_wrong_branch() -> Result<(), FixtureError> {
    exceptional(Exception::PrimaryWrong)
}
fn count(side: &Side, table: &str) -> Result<i64, FixtureError> {
    fixed(
        side.db()?
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0)),
    )
}
fn remote_deleted_case(rename: bool) -> Result<(), FixtureError> {
    let p = Pair::new()?;
    publish_new(&p, T)?;
    p.poll()?;
    fixed(std::fs::write(
        p.b.worktree(T).join("untracked"),
        b"retained local work\n",
    ))?;
    let bare = fixed(git2::Repository::open_bare(p.server.repository_path()))?;
    let tr = reference(T);
    let tip = fixed(bare.refname_to_id(&tr))?;
    let renamed = "01ARZ3NDEKTSV4RRFFQ69G5FAX";
    if rename {
        fixed(bare.reference(&reference(renamed), tip, false, "owned peer rename"))?;
    }
    fixed(fixed(bare.find_reference(&tr))?.delete())?;
    let before = snapshot(&p.b)?;
    let r = p.poll()?;
    unchanged(&r, &tr, PollDisposition::RemotelyDeleted);
    if rename {
        unchanged(&r, &reference(renamed), PollDisposition::Nonconforming);
        assert!(!p.b.worktree(renamed).exists());
    }
    assert_eq!(snapshot(&p.b)?, before);
    assert_eq!(fixed(p.b.repo()?.refname_to_id(&tracking(&tr)))?, tip);
    p.finish()
}
fn remote_deleted() -> Result<(), FixtureError> {
    remote_deleted_case(false)
}
fn remote_renamed() -> Result<(), FixtureError> {
    remote_deleted_case(true)
}
fn nonconforming_tree() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.publish(
        &reference(T),
        p.base()?,
        &[(TP, Some((source(T, TITLE, BODY).as_bytes(), 0o100755)))],
    )?;
    let before = snapshot(&p.b)?;
    unchanged(&p.poll()?, &reference(T), PollDisposition::Nonconforming);
    assert_eq!(snapshot(&p.b)?, before);
    assert_eq!(count(&p.b, "remote_poll_materializations")?, 0);
    p.finish()
}
fn malformed_family() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let oid = p.publish(
        BAD_REF,
        p.base()?,
        &[("malformed-only", Some((BODY.as_bytes(), MODE)))],
    )?;
    let before = snapshot(&p.b)?;
    let r = p.poll()?;
    let malformed = r
        .targets()
        .iter()
        .filter(|t| t.disposition() == PollDisposition::Malformed)
        .collect::<Vec<_>>();
    assert_eq!(malformed.len(), 1);
    assert!(malformed[0].target().is_none());
    assert_eq!(
        PollDisposition::Malformed.guidance(),
        guidance(PollDisposition::Malformed)
    );
    assert!(p.b.repo()?.find_commit(oid).is_err());
    assert!(p.b.repo()?.find_reference(&tracking(BAD_REF)).is_err());
    assert_eq!(snapshot(&p.b)?, before);
    p.finish()
}
fn outside_family() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let reference = "refs/heads/ordinary";
    let oid = p.publish(
        reference,
        p.base()?,
        &[("outside-only", Some((BODY.as_bytes(), MODE)))],
    )?;
    let before = snapshot(&p.b)?;
    assert_eq!(p.poll()?.targets().len(), 1);
    assert!(p.b.repo()?.find_commit(oid).is_err());
    assert!(p.b.repo()?.find_reference(&tracking(reference)).is_err());
    assert_eq!(snapshot(&p.b)?, before);
    p.finish()
}
fn blocked(branch: bool) -> Result<(), FixtureError> {
    let p = Pair::new()?;
    publish_new(&p, T)?;
    if branch {
        let repo = p.b.repo()?;
        fixed(repo.branch(
            reference(T).trim_start_matches("refs/heads/"),
            &fixed(repo.find_commit(p.base()?))?,
            false,
        ))?;
    } else {
        fixed(std::fs::create_dir_all(p.b.worktree(T)))?;
        fixed(std::fs::write(
            p.b.worktree(T).join("retained"),
            b"retain\n",
        ))?;
    }
    let before = snapshot(&p.b)?;
    unchanged(
        &p.poll()?,
        &reference(T),
        PollDisposition::BlockedByLocalState,
    );
    assert_eq!(snapshot(&p.b)?, before);
    assert_eq!(count(&p.b, "remote_poll_materializations")?, 0);
    p.finish()
}
fn branch_blocked() -> Result<(), FixtureError> {
    blocked(true)
}
fn path_blocked() -> Result<(), FixtureError> {
    blocked(false)
}
fn history_unknown() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.context(T)?;
    p.poll()?;
    let bare = fixed(git2::Repository::open_bare(p.server.repository_path()))?;
    assert!(bare.find_reference(&reference(T)).is_err());
    // A genuinely new application registry has no remote publication evidence.
    let data = p._dir.path().join("z");
    let service = fixed(RepositoryService::open_at(&data))?;
    fixed(service.enable(EnableRepositoryRequest {
        root: p.b.root.clone(),
        primary_branch: "main".into(),
        identity: None,
        operation_id: OperationId::new(),
    }))?;
    let RegisterSharedKeyOutcome::Registered(key) =
        fixed(service.register_shared_key(RegisterSharedKeyRequest {
            label: "new registry".into(),
            ownership: SharedKeyOwnership::Imported,
            private_key_path: p.server.client_key_path().into(),
            public_key_path: None,
        }))?
    else {
        return Err(FixtureError);
    };
    fixed(service.select_shared_key(key.id))?;
    let before = snapshot(&p.b)?;
    let (r, _) = report(fixed(p.poll_with(
        &service,
        p.request(RemotePollInvocation::Explicit),
        &mut session(),
    ))?)?;
    unchanged(&r, &reference(T), PollDisposition::HistoryUnknown);
    assert_eq!(snapshot(&p.b)?, before);
    ssh_privacy::scan(&data, &p.probes)?;
    p.finish()
}
fn directory_collision() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let repo = p.b.repo()?;
    let old = commit(
        &repo,
        p.base()?,
        &[
            ("dir/tracked", Some((b"old\n", MODE))),
            (
                ".gitignore",
                Some((b"dir/ignored\n.manyhands/worktrees/\n", MODE)),
            ),
        ],
    )?;
    install(&repo, MAIN, old)?;
    // Peer receives this base through owned fixture ODB copying, not poll writes.
    let peer = p.a.repo()?;
    let odb = fixed(peer.odb())?;
    let local_odb = fixed(repo.odb())?;
    for oid in objects(&repo)? {
        let object = fixed(local_odb.read(oid))?;
        fixed(odb.write(object.kind(), object.data()))?;
    }
    p.publish(
        MAIN,
        old,
        &[
            ("dir/tracked", None),
            ("dir", Some((b"incoming file\n", MODE))),
        ],
    )?;
    fixed(std::fs::write(
        p.b.root.join("dir/ignored"),
        b"must survive\n",
    ))?;
    let before = snapshot(&p.b)?;
    unchanged(&p.poll()?, MAIN, PollDisposition::Dirty);
    assert_eq!(snapshot(&p.b)?, before);
    p.finish()
}
fn foreign_ownership() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.context(T)?;
    let path = p.b.worktree(T);
    fixed(std::fs::rename(&path, p._dir.path().join("m")))?;
    let foreign = fixed(git2::Repository::init(&path))?;
    let blob = fixed(foreign.blob(b"foreign\n"))?;
    let mut b = fixed(foreign.treebuilder(None))?;
    fixed(b.insert("fixture.txt", blob, MODE as i32))?;
    let tree = fixed(foreign.find_tree(fixed(b.write())?))?;
    let sig = fixed(git2::Signature::now("Fixture", "fixture@example.invalid"))?;
    let oid = fixed(foreign.commit(Some(&reference(T)), &sig, &sig, "fixture", &tree, &[]))?;
    fixed(foreign.set_head(&reference(T)))?;
    fixed(foreign.checkout_head(None))?;
    p.publish(&reference(T), p.base()?, &[])?;
    let before = snapshot(&p.b)?;
    unchanged(&p.poll()?, &reference(T), PollDisposition::Inaccessible);
    assert_eq!(snapshot(&p.b)?, before);
    assert_eq!(fixed(foreign.head())?.target(), Some(oid));
    p.finish()
}
fn dot_document() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.publish(
        &reference(D),
        p.base()?,
        &[
            (DP, None),
            (
                "docs/.md",
                Some((source(D, "Fixture", BODY).as_bytes(), MODE)),
            ),
        ],
    )?;
    let before = snapshot(&p.b)?;
    unchanged(&p.poll()?, &reference(D), PollDisposition::Nonconforming);
    assert_eq!(snapshot(&p.b)?, before);
    p.finish()
}

fn policy_case(reason: PollIneligibility) -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let db = p.b.db()?;
    let (enabled, paused, suspended, next) = match reason {
        PollIneligibility::Disabled => (false, false, false, None),
        PollIneligibility::Paused => (true, true, false, None),
        PollIneligibility::RecoverySuspended => (true, false, true, None),
        PollIneligibility::NotDue => (
            true,
            false,
            false,
            Some(time::OffsetDateTime::now_utc().unix_timestamp() + 60),
        ),
    };
    fixed(db.execute("UPDATE remote_polling_state SET enabled=?1,paused=?2,recovery_suspended=?3,next_eligible_at=?4,latest_outcome='completed'",rusqlite::params![enabled,paused,suspended,next]))?;
    let before = fixed(p.b.service.remote_snapshot(&p.b.root))?;
    let connections = p.server.connections();
    let helpers = p.server.helper_invocations();
    let ops = count(&p.b, "remote_operation_records")?;
    let prompts = Arc::new(AtomicUsize::new(0));
    let mut s = SessionCredentials::new(Provider {
        prompts: prompts.clone(),
        cancelled: false,
    });
    let result = fixed(p.poll_with(
        &p.b.service,
        p.request(RemotePollInvocation::Automatic),
        &mut s,
    ))?;
    assert!(
        matches!(result,RemotePollResult::Complete(RemotePollOutcome::NotEligible{reason:r,..}) if r==reason)
    );
    assert_eq!(p.server.connections(), connections);
    assert_eq!(p.server.helper_invocations(), helpers);
    assert_eq!(prompts.load(Ordering::SeqCst), 0);
    assert_eq!(count(&p.b, "remote_operation_records")?, ops);
    assert_eq!(fixed(p.b.service.remote_snapshot(&p.b.root))?, before);
    let (r, pending) = report(fixed(p.poll_with(
        &p.b.service,
        p.request(RemotePollInvocation::Explicit),
        &mut s,
    ))?)?;
    assert!(!pending);
    let after = fixed(p.b.service.remote_snapshot(&p.b.root))?;
    assert_eq!(after.polling().enabled(), enabled);
    assert_eq!(after.polling().paused(), paused);
    assert_eq!(after.polling().recovery_suspended(), suspended);
    assert_eq!(
        r.next_eligible_at().unwrap() - r.completed_at(),
        time::Duration::seconds(300)
    );
    assert_eq!(after.next_eligible_at(), r.next_eligible_at());
    p.finish()
}
fn policy_disabled() -> Result<(), FixtureError> {
    policy_case(PollIneligibility::Disabled)
}
fn policy_paused() -> Result<(), FixtureError> {
    policy_case(PollIneligibility::Paused)
}
fn policy_suspended() -> Result<(), FixtureError> {
    policy_case(PollIneligibility::RecoverySuspended)
}
fn policy_not_due() -> Result<(), FixtureError> {
    policy_case(PollIneligibility::NotDue)
}

fn invalidate_config(p: &Pair, shape: usize) -> Result<(), FixtureError> {
    let path = p.b.root.join(".manyhands/config.toml");
    match shape {
        0 => fixed(std::fs::remove_file(path)),
        1 => fixed(std::fs::write(path, b"invalid TOML [")),
        2 => fixed(std::fs::write(
            path,
            b"format_version = 1\nprimary_branch = \"main\"\n",
        )),
        _ => unreachable!(),
    }
}
fn whole_review_invalid_auto(reason: PollIneligibility) -> Result<(), FixtureError> {
    for shape in 0..3 {
        let p = Pair::new()?;
        p.poll()?; // Establish an actual completed batch and a fresh index first.
        invalidate_config(&p, shape)?;
        let db = p.b.db()?;
        let next = time::OffsetDateTime::now_utc().unix_timestamp() + 60;
        let (enabled, paused, suspended) = match reason {
            PollIneligibility::Disabled => (false, false, false),
            PollIneligibility::Paused => (true, true, false),
            PollIneligibility::RecoverySuspended => (true, false, true),
            PollIneligibility::NotDue => (true, false, false),
        };
        fixed(db.execute("UPDATE remote_polling_state SET enabled=?1,paused=?2,recovery_suspended=?3,next_eligible_at=?4,automatic_backoff_seconds=120,latest_outcome='completed',latest_attempt_at=123", rusqlite::params![enabled,paused,suspended,next]))?;
        let before = fixed(p.b.service.remote_snapshot(&p.b.root))?;
        let local = snapshot(&p.b)?;
        let refs = all_refs(&p.b.repo()?)?;
        let batch = observation_digest(&p.b)?;
        fixed(db.execute_batch("CREATE TABLE whole_review_writes(n INTEGER);INSERT INTO whole_review_writes VALUES(0);CREATE TRIGGER whole_review_policy AFTER UPDATE ON remote_polling_state BEGIN UPDATE whole_review_writes SET n=n+1;END;CREATE TRIGGER whole_review_new_op AFTER INSERT ON remote_operation_records BEGIN UPDATE whole_review_writes SET n=n+1;END;CREATE TRIGGER whole_review_op AFTER UPDATE ON remote_operation_records BEGIN UPDATE whole_review_writes SET n=n+1;END;"))?;
        let connections = p.server.connections();
        let helpers = p.server.helper_invocations();
        let ops = count(&p.b, "remote_operation_records")?;
        let prompts = Arc::new(AtomicUsize::new(0));
        let mut s = SessionCredentials::new(Provider {
            prompts: prompts.clone(),
            cancelled: false,
        });
        let result = p.poll_with(
            &p.b.service,
            p.request(RemotePollInvocation::Automatic),
            &mut s,
        );
        assert!(
            matches!(result, Ok(RemotePollResult::Complete(RemotePollOutcome::NotEligible{reason:r,next_eligible_at:Some(deadline)})) if r==reason && deadline.unix_timestamp()==next)
        );
        assert_eq!(p.server.connections(), connections);
        assert_eq!(p.server.helper_invocations(), helpers);
        assert_eq!(prompts.load(Ordering::SeqCst), 0);
        assert_eq!(count(&p.b, "remote_operation_records")?, ops);
        assert_eq!(
            fixed(db.query_row("SELECT n FROM whole_review_writes", [], |r| {
                r.get::<_, i64>(0)
            }))?,
            0
        );
        assert_eq!(fixed(p.b.service.remote_snapshot(&p.b.root))?, before);
        assert_eq!(snapshot(&p.b)?, local);
        assert_eq!(all_refs(&p.b.repo()?)?, refs);
        assert_eq!(observation_digest(&p.b)?, batch);
        p.finish()?;
    }
    Ok(())
}
fn whole_review_auto_disabled() -> Result<(), FixtureError> {
    whole_review_invalid_auto(PollIneligibility::Disabled)
}
fn whole_review_auto_paused() -> Result<(), FixtureError> {
    whole_review_invalid_auto(PollIneligibility::Paused)
}
fn whole_review_auto_suspended() -> Result<(), FixtureError> {
    whole_review_invalid_auto(PollIneligibility::RecoverySuspended)
}
fn whole_review_auto_not_due() -> Result<(), FixtureError> {
    whole_review_invalid_auto(PollIneligibility::NotDue)
}

fn whole_review_invalid_attention(invocation: RemotePollInvocation) -> Result<(), FixtureError> {
    for shape in 0..3 {
        let p = Pair::new()?;
        p.poll()?;
        invalidate_config(&p, shape)?;
        fixed(p.b.db()?.execute_batch("UPDATE remote_polling_state SET automatic_backoff_seconds=120,next_eligible_at=12345,latest_attempt_at=123,latest_outcome='completed'"))?;
        let connections = p.server.connections();
        let local = snapshot(&p.b)?;
        let refs = all_refs(&p.b.repo()?)?;
        let result = p.poll_with(&p.b.service, p.request(invocation), &mut session());
        assert!(matches!(result, Err(RemotePollError::Transport(_))));
        let after = fixed(p.b.service.remote_snapshot(&p.b.root))?;
        assert_eq!(
            after.latest_outcome(),
            Some(RemoteOutcomeCategory::ConfigurationRequired)
        );
        assert_eq!(
            after.polling().automatic_backoff().map(|b| b.as_secs()),
            Some(120)
        );
        let attempt = after.latest_attempt_at().unwrap();
        assert!(attempt.unix_timestamp() > 123);
        assert_eq!(
            after.next_eligible_at().unwrap().unix_timestamp(),
            if invocation == RemotePollInvocation::Automatic {
                attempt.unix_timestamp() + 300
            } else {
                12345
            }
        );
        assert_eq!(p.server.connections(), connections);
        assert_eq!(snapshot(&p.b)?, local);
        assert_eq!(all_refs(&p.b.repo()?)?, refs);
        p.finish()?;
    }
    Ok(())
}
fn whole_review_attention_automatic() -> Result<(), FixtureError> {
    whole_review_invalid_attention(RemotePollInvocation::Automatic)
}
fn whole_review_attention_explicit() -> Result<(), FixtureError> {
    whole_review_invalid_attention(RemotePollInvocation::Explicit)
}

fn whole_review_endpoint_attention(invocation: RemotePollInvocation) -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.poll()?;
    fixed(p.b.db()?.execute_batch(
        "UPDATE remote_polling_state SET automatic_backoff_seconds=120,next_eligible_at=12345",
    ))?;
    fixed(
        p.b.repo()?
            .remote_set_url("origin", &format!("{}/changed", p.server.url())),
    )?;
    let before = snapshot(&p.b)?;
    let refs = all_refs(&p.b.repo()?)?;
    let prompts = Arc::new(AtomicUsize::new(0));
    let mut s = SessionCredentials::new(Provider {
        prompts: prompts.clone(),
        cancelled: true,
    });
    let result = p.poll_with(&p.b.service, p.request(invocation), &mut s);
    assert!(matches!(result, Err(RemotePollError::Transport(_))));
    let after = fixed(p.b.service.remote_snapshot(&p.b.root))?;
    assert_eq!(
        after.latest_outcome(),
        Some(RemoteOutcomeCategory::UnlockRequired)
    );
    assert_eq!(
        after.polling().automatic_backoff().map(|b| b.as_secs()),
        Some(120)
    );
    assert_eq!(
        after.next_eligible_at().unwrap().unix_timestamp(),
        if invocation == RemotePollInvocation::Automatic {
            after.latest_attempt_at().unwrap().unix_timestamp() + 300
        } else {
            12345
        }
    );
    // This fresh session is not blocked yet: its first cancelled unlock prompts
    // once in either mode. The separate blocked-session case proves no reprompt.
    assert_eq!(prompts.load(Ordering::SeqCst), 1);
    assert_eq!(snapshot(&p.b)?, before);
    assert_eq!(all_refs(&p.b.repo()?)?, refs);
    p.finish()
}
fn whole_review_endpoint_automatic() -> Result<(), FixtureError> {
    whole_review_endpoint_attention(RemotePollInvocation::Automatic)
}
fn whole_review_endpoint_explicit() -> Result<(), FixtureError> {
    whole_review_endpoint_attention(RemotePollInvocation::Explicit)
}

fn whole_review_effect_cancel(materialization: bool) -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let target_ref = if materialization {
        reference(T)
    } else {
        MAIN.into()
    };
    p.poll()?;
    let db = p.b.db()?;
    assert!(!fixed(db.query_row(
        "SELECT refresh_required FROM repositories",
        [],
        |r| r.get::<_, bool>(0)
    ))?);
    let tip = if materialization {
        publish_new(&p, T)?
    } else {
        p.publish(MAIN, p.base()?, &[("remote", Some((b"remote\n", MODE)))])?
    };
    fixed(db.execute_batch(if materialization {
        "CREATE TRIGGER whole_review_failure BEFORE UPDATE OF step ON remote_poll_materializations WHEN NEW.step='branch_created' BEGIN SELECT RAISE(ABORT,'fixed fault'); END;"
    } else {
        "CREATE TRIGGER whole_review_failure BEFORE UPDATE ON remote_poll_dispositions WHEN NEW.disposition='fast_forwarded' BEGIN SELECT RAISE(ABORT,'fixed fault'); END;"
    }))?;
    let req = p.request(RemotePollInvocation::Explicit);
    assert!(matches!(
        p.poll_with(&p.b.service, req.clone(), &mut session()),
        Err(RemotePollError::Repository(_))
    ));
    assert_eq!(fixed(p.b.repo()?.refname_to_id(&target_ref))?, tip);
    if materialization {
        assert!(!p.b.worktree(T).exists());
        assert_eq!(
            material_intent(&p)?,
            (
                req.operation_id.to_string(),
                tip.to_string(),
                "intended".into()
            )
        );
    } else {
        clean_checkout(&p, None, tip)?;
    }
    // The effect precedes its failed SQLite row write: this must already be
    // durable, even if a subsequent restart only acknowledges cancellation.
    assert!(fixed(db.query_row(
        "SELECT refresh_required FROM repositories",
        [],
        |r| r.get::<_, bool>(0)
    ))?);
    fixed(db.execute_batch("DROP TRIGGER whole_review_failure"))?;
    let before = snapshot(&p.b)?;
    let refs = all_refs(&p.b.repo()?)?;
    let second = p.b.reopen()?;
    fixed(second.cancel_remote_operation(&p.b.root, req.operation_id))?;
    let connections = p.server.connections();
    let mut restart = req.clone();
    restart.restart = true;
    let (cancelled, pending) = report(fixed(p.poll_with(&second, restart, &mut session()))?)?;
    assert!(pending);
    assert_eq!(cancelled.completion(), PollCompletion::Cancelled);
    assert_eq!(
        row(&cancelled, &target_ref).disposition(),
        PollDisposition::NotReached
    );
    assert_eq!(p.server.connections(), connections);
    assert_eq!(snapshot(&p.b)?, before);
    assert_eq!(all_refs(&p.b.repo()?)?, refs);
    fixed(second.refresh_repository(RefreshRepositoryRequest {
        root: p.b.root.clone(),
        operation_id: OperationId::new(),
    }))?;
    let (replay, pending) = report(fixed(p.poll_with(&second, req, &mut session()))?)?;
    assert!(!pending);
    assert_eq!(replay, cancelled);
    assert_eq!(p.server.connections(), connections);
    assert_eq!(snapshot(&p.b)?, before);
    assert_eq!(all_refs(&p.b.repo()?)?, refs);
    p.finish()
}
fn whole_review_disposition_cancel() -> Result<(), FixtureError> {
    whole_review_effect_cancel(false)
}
fn whole_review_branch_cancel() -> Result<(), FixtureError> {
    whole_review_effect_cancel(true)
}
fn backoff() -> Result<(), FixtureError> {
    let mut p = Pair::new()?;
    p.server.shutdown()?;
    for expected in [60, 120, 240, 480, 900, 900] {
        fixed(
            p.b.db()?
                .execute("UPDATE remote_polling_state SET next_eligible_at=NULL", []),
        )?;
        assert!(matches!(
            p.poll_with(
                &p.b.service,
                p.request(RemotePollInvocation::Automatic),
                &mut session()
            ),
            Err(RemotePollError::Transport(_))
        ));
        let snap = fixed(p.b.service.remote_snapshot(&p.b.root))?;
        assert_eq!(
            snap.polling().automatic_backoff().unwrap().as_secs(),
            expected
        );
        assert_eq!(
            snap.latest_outcome(),
            Some(RemoteOutcomeCategory::TransportUnavailable)
        );
        assert_eq!(
            snap.next_eligible_at().unwrap() - snap.latest_attempt_at().unwrap(),
            time::Duration::seconds(expected as i64)
        );
    }
    let snap = fixed(p.b.service.remote_snapshot(&p.b.root))?;
    assert!(matches!(
        p.poll_with(
            &p.b.service,
            p.request(RemotePollInvocation::Explicit),
            &mut session()
        ),
        Err(RemotePollError::Transport(_))
    ));
    let after = fixed(p.b.service.remote_snapshot(&p.b.root))?;
    assert_eq!(after.polling(), snap.polling());
    assert_eq!(after.next_eligible_at(), snap.next_eligible_at());
    p.finish()
}
fn attention(reject: bool) -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let mut req = p.request(RemotePollInvocation::Automatic);
    if reject {
        p.server.reject_client();
    } else {
        req.approval = None;
    }
    assert!(matches!(
        p.poll_with(&p.b.service, req, &mut session()),
        Err(RemotePollError::Transport(_))
    ));
    let snap = fixed(p.b.service.remote_snapshot(&p.b.root))?;
    assert_eq!(
        snap.latest_outcome(),
        Some(if reject {
            RemoteOutcomeCategory::UnlockRequired
        } else {
            RemoteOutcomeCategory::HostApprovalRequired
        })
    );
    assert!(snap.polling().automatic_backoff().is_none());
    p.finish()
}
fn rejected_key() -> Result<(), FixtureError> {
    attention(true)
}
fn unapproved_host() -> Result<(), FixtureError> {
    attention(false)
}
fn locked_session() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let prompts = Arc::new(AtomicUsize::new(0));
    let mut s = SessionCredentials::new(Provider {
        prompts: prompts.clone(),
        cancelled: true,
    });
    assert!(matches!(
        p.poll_with(
            &p.b.service,
            p.request(RemotePollInvocation::Explicit),
            &mut s
        ),
        Err(RemotePollError::Transport(_))
    ));
    let n = prompts.load(Ordering::SeqCst);
    assert_eq!(n, 1);
    let helpers = p.server.helper_invocations();
    assert!(matches!(
        p.poll_with(
            &p.b.service,
            p.request(RemotePollInvocation::Automatic),
            &mut s
        ),
        Err(RemotePollError::Transport(_))
    ));
    assert_eq!(prompts.load(Ordering::SeqCst), n);
    assert_eq!(p.server.helper_invocations(), helpers);
    // Host verification precedes the blocked authentication callback: a TCP/SSH
    // handshake may occur, but neither a prompt nor a Git command is allowed.
    p.finish()
}

fn sync_request(p: &Pair, target: SynchronizationTarget) -> SynchronizeRemoteRequest {
    SynchronizeRemoteRequest {
        root: p.b.root.clone(),
        operation_id: OperationId::new(),
        target,
        restart: false,
        confirmed_identity: None,
        approval: Some(p.approval()),
    }
}
fn prepare_transfer(p: &Pair) -> Result<git2::Oid, FixtureError> {
    let mut state = 41u64;
    let payload = (0..1_600_000)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect::<Vec<_>>();
    p.publish(MAIN, p.base()?, &[("payload", Some((&payload, MODE)))])
}
fn priority(transfer: bool, explicit: bool, local: bool) -> Result<(), FixtureError> {
    let t0 = std::time::Instant::now();
    let p = Pair::new()?;
    ssh_harness::observation(&[1, t0.elapsed().as_millis()]);
    if transfer {
        prepare_transfer(&p)?;
        p.server.pace_transfer(std::time::Duration::from_millis(50));
    }
    let second = p.b.reopen()?;
    let gate = if transfer {
        p.server.hold_transfer_after(131_072)?
    } else {
        p.server.hold_advertisement()?
    };
    let before = snapshot(&p.b)?;
    let refs_before = all_refs(&p.b.repo()?)?;
    ssh_harness::observation(&[2, t0.elapsed().as_millis()]);
    let n = p.no_push_baseline()?;
    ssh_harness::observation(&[3, t0.elapsed().as_millis()]);
    let local_ops = count(&p.b, "operation_records")?;
    let invocation = if explicit {
        RemotePollInvocation::Explicit
    } else {
        RemotePollInvocation::Automatic
    };
    let request = p.request(invocation);
    let poll_id = request.operation_id;
    let manual = sync_request(&p, SynchronizationTarget::Primary);
    let save = SaveTicketRequest {
        target: AuthoringTarget {
            root: p.b.root.clone(),
            kind: AuthoringKind::Ticket,
            item_id: T.parse().unwrap(),
            intent: ContextIntent::Edit,
            operation_id: OperationId::new(),
        },
        draft: TicketDraft {
            title: "Saved local".into(),
            body: BODY.into(),
            ticket_type: "task".into(),
            status: "open".into(),
            project: None,
            team: None,
        },
        expected_path: ExpectedPathObservation::from_bytes(&fixed(std::fs::read(
            p.b.root.join(TP),
        ))?),
    };
    std::thread::scope(|scope| -> Result<(), FixtureError> {
        let worker = scope.spawn(|| p.b.service.poll_publication_remote(request, &mut session()));
        ssh_harness::observation(&[4, t0.elapsed().as_millis()]);
        gate.wait_until_held()?;
        ssh_harness::observation(&[5, t0.elapsed().as_millis()]);
        if local {
            assert!(
                matches!(second.save_ticket(save.clone()),Err(e) if e.kind==RepositoryErrorKind::RepositoryBusy)
            );
            assert_eq!(count(&p.b, "operation_records")?, local_ops);
        } else {
            let attempt = second.synchronize_remote(manual.clone(), &mut session());
            assert!(if explicit {
                matches!(attempt, Err(SynchronizationError::Busy))
            } else {
                matches!(attempt, Err(SynchronizationError::PollYielding))
            });
            p.redacted(&format!("{attempt:?}"))?;
        }
        assert_eq!(snapshot(&p.b)?, before);
        assert_eq!(all_refs(&p.b.repo()?)?, refs_before);
        gate.release()?;
        ssh_harness::observation(&[6, t0.elapsed().as_millis()]);
        let result = fixed(worker.join())?;
        ssh_harness::observation(&[7, t0.elapsed().as_millis()]);
        p.redacted(&format!("{result:?}"))?;
        if explicit {
            assert_eq!(
                report(fixed(result)?)?.0.completion(),
                PollCompletion::Completed
            );
        } else {
            assert!(
                matches!(result, Err(RemotePollError::Interrupted))
                    || matches!(result,Ok(RemotePollResult::Complete(RemotePollOutcome::Polled(ref r))) if r.completion()==PollCompletion::Interrupted)
            );
            assert_eq!(snapshot(&p.b)?, before);
            assert_eq!(all_refs(&p.b.repo()?)?, refs_before);
        }
        Ok(())
    })?;
    ssh_harness::observation(&[8, t0.elapsed().as_millis()]);
    p.no_push_since(n)?;
    ssh_harness::observation(&[9, t0.elapsed().as_millis()]);
    p.privacy()?;
    p.server.clear_fault();
    let inspection = fixed(p.b.service.active_remote_operation(&p.b.root))?;
    assert!(inspection.is_none());
    let phase = fixed(p.b.db()?.query_row(
        "SELECT phase FROM remote_operation_records WHERE operation_ulid=?1",
        [poll_id.to_string()],
        |r| r.get::<_, String>(0),
    ))?;
    assert_eq!(phase, if explicit { "completed" } else { "interrupted" });
    // Retry belongs to a manual phase and is deliberately outside the poll counter scope.
    if local {
        assert!(matches!(
            fixed(second.save_ticket(save))?,
            SaveOutcome::Saved { .. } | SaveOutcome::IndexPending { .. }
        ));
    } else {
        assert!(matches!(
            fixed(second.synchronize_remote(manual, &mut session()))?,
            SynchronizationResult::Complete(_)
        ));
    }
    ssh_harness::observation(&[10, t0.elapsed().as_millis()]);
    p.finish()
}
fn advertisement_yield() -> Result<(), FixtureError> {
    priority(false, false, false)
}
fn transfer_yield() -> Result<(), FixtureError> {
    priority(true, false, false)
}
fn explicit_busy() -> Result<(), FixtureError> {
    priority(false, true, false)
}
fn local_save_yield() -> Result<(), FixtureError> {
    priority(false, false, true)
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct RefState {
    name: String,
    direct: Option<git2::Oid>,
    symbolic: Option<String>,
}
fn all_refs(repo: &git2::Repository) -> Result<Vec<RefState>, FixtureError> {
    let mut refs = fixed(repo.references())?
        .map(|r| {
            let r = fixed(r)?;
            Ok(RefState {
                name: fixed(r.name().ok_or(FixtureError))?.to_owned(),
                direct: r.target(),
                symbolic: r.symbolic_target().map(str::to_owned),
            })
        })
        .collect::<Result<Vec<_>, FixtureError>>()?;
    refs.sort();
    Ok(refs)
}
const STALE_TRACKING: &str = "refs/remotes/origin/stale";
const SYMBOLIC_TRACKING: &str = "refs/remotes/unused/HEAD";
fn sentinel_states(repo: &git2::Repository) -> Result<Vec<RefState>, FixtureError> {
    Ok(all_refs(repo)?
        .into_iter()
        .filter(|r| [STALE_TRACKING, SYMBOLIC_TRACKING].contains(&r.name.as_str()))
        .collect())
}
fn stale_tracking(p: &Pair, oid: git2::Oid) -> Result<Vec<RefState>, FixtureError> {
    let repo = p.b.repo()?;
    fixed(repo.reference(STALE_TRACKING, oid, false, "owned D1 sentinel"))?;
    fixed(repo.reference_symbolic(
        SYMBOLIC_TRACKING,
        STALE_TRACKING,
        false,
        "owned unrelated sentinel",
    ))?;
    // The complete inventory must preserve these even with backend prune enabled.
    fixed(fixed(repo.config())?.set_bool("fetch.prune", true))?;
    let observed = sentinel_states(&repo)?;
    let mut expected = vec![
        RefState {
            name: STALE_TRACKING.into(),
            direct: Some(oid),
            symbolic: None,
        },
        RefState {
            name: SYMBOLIC_TRACKING.into(),
            direct: None,
            symbolic: Some(STALE_TRACKING.into()),
        },
    ];
    expected.sort();
    assert_eq!(observed, expected);
    Ok(observed)
}
fn retained_conflict() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.context(T)?;
    let base = p.base()?;
    let local = fixed(git2::Repository::open(p.b.worktree(T)))?;
    let mine = commit(
        &local,
        base,
        &[(
            TP,
            Some((source(T, "Local", "local conflict\n").as_bytes(), MODE)),
        )],
    )?;
    install(&local, &reference(T), mine)?;
    p.publish(
        &reference(T),
        base,
        &[(TP, Some((source(T, "Peer", BODY).as_bytes(), MODE)))],
    )?;
    let sync = sync_request(
        &p,
        SynchronizationTarget::Context {
            kind: AuthoringKind::Ticket,
            item_id: T.parse().unwrap(),
        },
    );
    assert!(matches!(
        p.b.service.synchronize_remote(sync, &mut session()),
        Err(SynchronizationError::ConflictPending { .. })
    ));
    assert!(fixed(fixed(git2::Repository::open(p.b.worktree(T)))?.index())?.has_conflicts());
    let before = snapshot(&p.b)?;
    let refs = all_refs(&p.b.repo()?)?;
    let connections = p.server.connections();
    let helpers = p.server.helper_invocations();
    let ops = count(&p.b, "remote_operation_records")?;
    for invocation in [
        RemotePollInvocation::Automatic,
        RemotePollInvocation::Explicit,
    ] {
        assert!(matches!(
            p.poll_with(&p.b.service, p.request(invocation), &mut session()),
            Err(RemotePollError::ConflictPending)
        ));
        assert_eq!(p.server.connections(), connections);
        assert_eq!(p.server.helper_invocations(), helpers);
        assert_eq!(snapshot(&p.b)?, before);
        assert_eq!(all_refs(&p.b.repo()?)?, refs);
        assert_eq!(count(&p.b, "remote_operation_records")?, ops);
    }
    p.finish()
}
fn cancel_at(point: &str) -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.poll()?; // establish the endpoint generation before seeding policy metadata
    fixed(p.b.db()?.execute_batch(
        "UPDATE remote_polling_state SET automatic_backoff_seconds=120,next_eligible_at=12345;",
    ))?;
    let base = p.base()?;
    let incoming = p.publish(MAIN, base, &[("fixture.txt", Some((b"incoming\n", MODE)))])?;
    fixed(p.b.db()?.execute_batch(&format!("CREATE TRIGGER poll_cancel AFTER UPDATE OF completed_step ON remote_operation_records WHEN NEW.completed_step='{point}' BEGIN UPDATE remote_operation_records SET cancel_requested=1 WHERE id=NEW.id; END;")))?;
    let before = snapshot(&p.b)?;
    let locals = count(&p.b, "operation_records")?;
    let (r, pending) = report(fixed(p.poll_with(
        &p.b.service,
        p.request(RemotePollInvocation::Explicit),
        &mut session(),
    ))?)?;
    assert_eq!(r.completion(), PollCompletion::Cancelled);
    let changed = point == "before_discovery";
    assert_eq!(pending, changed);
    assert_eq!(
        fixed(p.b.repo()?.refname_to_id(MAIN))?,
        if changed { incoming } else { base }
    );
    if changed {
        assert_eq!(r.targets()[0].disposition(), PollDisposition::FastForwarded);
    } else {
        assert_eq!(snapshot(&p.b)?, before);
        assert!(
            r.targets()
                .iter()
                .all(|t| t.disposition() == PollDisposition::NotReached)
        );
    }
    assert_eq!(count(&p.b, "operation_records")?, locals);
    assert!(fixed(p.b.service.active_remote_operation(&p.b.root))?.is_none());
    let policy = fixed(p.b.service.remote_snapshot(&p.b.root))?;
    assert_eq!(policy.polling().automatic_backoff().unwrap().as_secs(), 120);
    assert_eq!(policy.next_eligible_at().unwrap().unix_timestamp(), 12345);
    p.finish()
}
fn cancel_before_transport() -> Result<(), FixtureError> {
    cancel_at("before_transport")
}
fn cancel_after_advertisement() -> Result<(), FixtureError> {
    cancel_at("after_advertisement")
}
fn cancel_between_observations() -> Result<(), FixtureError> {
    cancel_at("between_observations")
}
fn cancel_before_batch() -> Result<(), FixtureError> {
    cancel_at("before_batch_commit")
}
fn cancel_after_batch() -> Result<(), FixtureError> {
    cancel_at("after_batch_commit")
}
fn cancel_before_local() -> Result<(), FixtureError> {
    cancel_at("before_local_mutation")
}
fn cancel_before_discovery() -> Result<(), FixtureError> {
    cancel_at("before_discovery")
}
fn public_cancel() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let gate = p.server.hold_advertisement()?;
    let second = p.b.reopen()?;
    let before = snapshot(&p.b)?;
    let n = p.no_push_baseline()?;
    let req = p.request(RemotePollInvocation::Explicit);
    let id = req.operation_id;
    std::thread::scope(|scope| -> Result<(), FixtureError> {
        let worker = scope.spawn(|| p.b.service.poll_publication_remote(req, &mut session()));
        gate.wait_until_held()?;
        fixed(second.cancel_remote_operation(&p.b.root, id))?;
        gate.release()?;
        let (r, _) = report(fixed(fixed(worker.join())?)?)?;
        assert_eq!(r.completion(), PollCompletion::Cancelled);
        assert!(r.targets().is_empty());
        p.redacted(&format!("{r:?}"))?;
        Ok(())
    })?;
    p.no_push_since(n)?;
    assert_eq!(snapshot(&p.b)?, before);
    p.finish()
}
fn partial_batch() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.context(T)?;
    let base = p.base()?;
    let main = p.publish(
        MAIN,
        base,
        &[("fixture.txt", Some((b"primary incoming\n", MODE)))],
    )?;
    let ticket = p.publish(
        &reference(T),
        base,
        &[("fixture.txt", Some((b"ticket incoming\n", MODE)))],
    )?;
    fixed(p.b.db()?.execute_batch("CREATE TRIGGER stop_after_primary AFTER UPDATE ON remote_poll_dispositions WHEN NEW.ordinal=0 AND NEW.disposition='fast_forwarded' BEGIN UPDATE remote_operation_records SET yield_requested=1 WHERE operation_ulid=NEW.operation_ulid; END;"))?;
    let locals = count(&p.b, "operation_records")?;
    let req = p.request(RemotePollInvocation::Explicit);
    let (r, pending) = report(fixed(p.poll_with(
        &p.b.service,
        req.clone(),
        &mut session(),
    ))?)?;
    assert!(pending);
    assert_eq!(r.completion(), PollCompletion::Interrupted);
    assert_eq!(
        r.targets()
            .iter()
            .map(|t| t.disposition())
            .collect::<Vec<_>>(),
        [PollDisposition::FastForwarded, PollDisposition::NotReached]
    );
    assert_eq!(r.targets()[0].before_oid(), Some(base));
    assert_eq!(r.targets()[0].after_oid(), Some(main));
    assert_eq!(r.targets()[1].before_oid(), None);
    assert_eq!(fixed(p.b.repo()?.refname_to_id(&reference(T)))?, base);
    assert_eq!(count(&p.b, "operation_records")?, locals);
    fixed(p.b.db()?.execute_batch("DROP TRIGGER stop_after_primary"))?;
    let second = p.b.reopen()?;
    let mut restart = req.clone();
    restart.restart = true;
    let (done, pending) = report(fixed(p.poll_with(&second, restart, &mut session()))?)?;
    assert!(!pending);
    assert_eq!(row(&done, MAIN).disposition(), PollDisposition::Current);
    assert_eq!(
        row(&done, &reference(T)).disposition(),
        PollDisposition::FastForwarded
    );
    clean_checkout(&p, Some(T), ticket)?;
    let later = p.poll()?;
    let (old, _) = report(fixed(p.poll_with(&second, req, &mut session()))?)?;
    assert!(old.targets().is_empty());
    assert_eq!(old.completed_at(), done.completed_at());
    assert_eq!(old.next_eligible_at(), done.next_eligible_at());
    assert_ne!(later.operation_id(), old.operation_id());
    p.finish()
}
fn sql_failure(sql: &str, stage: &str) -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let base = p.base()?;
    let incoming = p.publish(MAIN, base, &[("fixture.txt", Some((b"incoming\n", MODE)))])?;
    fixed(p.b.db()?.execute_batch(sql))?;
    let req = p.request(RemotePollInvocation::Explicit);
    let before = snapshot(&p.b)?;
    assert!(matches!(
        p.poll_with(&p.b.service, req.clone(), &mut session()),
        Err(RemotePollError::Repository(_))
    ));
    assert_eq!(
        fixed(p.b.repo()?.refname_to_id("refs/remotes/origin/main"))?,
        incoming
    );
    let changed = matches!(stage, "disposition" | "completion");
    assert_eq!(
        fixed(p.b.repo()?.refname_to_id(MAIN))?,
        if changed { incoming } else { base }
    );
    if !changed {
        assert_eq!(snapshot(&p.b)?, before);
    }
    if stage == "batch" {
        assert_eq!(count(&p.b, "remote_observation_batches")?, 0);
        assert_eq!(count(&p.b, "remote_poll_dispositions")?, 0);
    }
    if stage == "disposition" {
        let disposition = fixed(p.b.db()?.query_row(
            "SELECT disposition FROM remote_poll_dispositions WHERE ordinal=0",
            [],
            |r| r.get::<_, String>(0),
        ))?;
        assert_eq!(disposition, "not_reached");
    }
    fixed(p.b.db()?.execute_batch("DROP TRIGGER poll_failure"))?;
    let second = p.b.reopen()?;
    let before_restart = snapshot(&p.b)?;
    let mut restart = req;
    restart.restart = true;
    let (r, pending) = report(fixed(p.poll_with(&second, restart, &mut session()))?)?;
    assert!(!pending);
    assert_eq!(r.completion(), PollCompletion::Completed);
    assert_eq!(
        row(&r, MAIN).disposition(),
        if changed {
            PollDisposition::Current
        } else {
            PollDisposition::FastForwarded
        }
    );
    if changed {
        assert_eq!(snapshot(&p.b)?, before_restart);
    }
    clean_checkout(&p, None, incoming)?;
    p.finish()
}
fn failure_after_tracking() -> Result<(), FixtureError> {
    sql_failure(
        "CREATE TRIGGER poll_failure BEFORE UPDATE OF completed_step ON remote_operation_records WHEN NEW.completed_step='after_advertisement' BEGIN SELECT RAISE(ABORT,'fixed fault'); END;",
        "tracking",
    )
}
fn batch_failure() -> Result<(), FixtureError> {
    sql_failure(
        "CREATE TRIGGER poll_failure BEFORE INSERT ON remote_poll_dispositions BEGIN SELECT RAISE(ABORT,'fixed fault'); END;",
        "batch",
    )
}
fn disposition_failure() -> Result<(), FixtureError> {
    sql_failure(
        "CREATE TRIGGER poll_failure BEFORE UPDATE ON remote_poll_dispositions WHEN NEW.disposition='fast_forwarded' BEGIN SELECT RAISE(ABORT,'fixed fault'); END;",
        "disposition",
    )
}
fn completion_failure() -> Result<(), FixtureError> {
    sql_failure(
        "CREATE TRIGGER poll_failure BEFORE UPDATE OF phase ON remote_operation_records WHEN NEW.phase='completed' BEGIN SELECT RAISE(ABORT,'fixed fault'); END;",
        "completion",
    )
}
fn disconnect_download_case(invocation: RemotePollInvocation) -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.poll()?; // retain an actual complete earlier batch and endpoint generation
    let incoming = prepare_transfer(&p)?;
    fixed(p.b.db()?.execute(
        "UPDATE remote_polling_state SET automatic_backoff_seconds=120,next_eligible_at=?1",
        [if invocation == RemotePollInvocation::Automatic {
            None
        } else {
            Some(12345i64)
        }],
    ))?;
    let policy = fixed(p.b.service.remote_snapshot(&p.b.root))?;
    let before = snapshot(&p.b)?;
    let refs = all_refs(&p.b.repo()?)?;
    let batch = observation_digest(&p.b)?;
    let odb = objects(&p.b.repo()?)?;
    let pack_bytes = p.server.upload_pack_bytes();
    let disconnects = p.server.byte_disconnects();
    let gate = p.server.hold_transfer_after(131_072)?;
    let n = p.no_push_baseline()?;
    let req = p.request(invocation);
    std::thread::scope(|scope| -> Result<(), FixtureError> {
        let request = req.clone();
        let worker = scope.spawn(|| p.b.service.poll_publication_remote(request, &mut session()));
        gate.wait_until_held()?;
        // Parsed sideband-1 PACK payload was really forwarded, not ACKs or
        // progress text. The fixture is now held before its next response.
        assert!(p.server.upload_pack_bytes() - pack_bytes >= 65_536);
        assert_eq!(all_refs(&p.b.repo()?)?, refs);
        p.server.disconnect_after_transfer_bytes(131_072);
        gate.release()?;
        let result = fixed(worker.join())?;
        assert!(matches!(result, Err(RemotePollError::Transport(_))));
        p.redacted(&format!("{result:?}"))?;
        Ok(())
    })?;
    p.no_push_since(n)?;
    assert_eq!(p.server.byte_disconnects(), disconnects + 1);
    assert_eq!(snapshot(&p.b)?, before);
    assert_eq!(all_refs(&p.b.repo()?)?, refs);
    assert_eq!(observation_digest(&p.b)?, batch);
    assert_eq!(count(&p.b, "remote_observation_batches")?, 1);
    let after_odb = objects(&p.b.repo()?)?;
    // Complete pre-existing objects are retained. A backend may keep additional
    // valid objects from a partial download; they cannot authorize ref writes.
    assert!(odb.iter().all(|oid| after_odb.contains(oid)));
    for oid in &after_odb {
        fixed(fixed(p.b.repo()?.odb())?.read(*oid))?;
    }
    ssh_harness::observation(&[
        p.server.upload_pack_bytes().saturating_sub(pack_bytes) as u128,
        (after_odb.len() - odb.len()) as u128,
        u128::from(p.b.repo()?.find_commit(incoming).is_ok()),
    ]);
    let (phase, outcome) = fixed(p.b.db()?.query_row(
        "SELECT phase,outcome FROM remote_operation_records WHERE operation_ulid=?1",
        [req.operation_id.to_string()],
        |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
    ))?;
    assert_eq!(phase, "failed");
    assert_eq!(outcome, "transport_unavailable");
    let failed = fixed(p.b.service.remote_snapshot(&p.b.root))?;
    assert_eq!(
        failed.latest_outcome(),
        Some(RemoteOutcomeCategory::TransportUnavailable)
    );
    if invocation == RemotePollInvocation::Automatic {
        assert_eq!(failed.polling().automatic_backoff().unwrap().as_secs(), 240);
        assert_eq!(
            failed.next_eligible_at().unwrap() - failed.latest_attempt_at().unwrap(),
            time::Duration::seconds(240)
        );
    } else {
        assert_eq!(failed.polling(), policy.polling());
        assert_eq!(failed.next_eligible_at(), policy.next_eligible_at());
    }
    p.privacy()?;
    p.server.clear_fault();
    let second = p.b.reopen()?;
    let connections = p.server.connections();
    let (replay, _) = report(fixed(p.poll_with(&second, req, &mut session()))?)?;
    assert_eq!(
        replay.outcome(),
        Some(RemoteOutcomeCategory::TransportUnavailable)
    );
    assert_eq!(p.server.connections(), connections);
    assert_eq!(observation_digest(&p.b)?, batch);
    let (r, _) = report(fixed(p.poll_with(
        &second,
        p.request(RemotePollInvocation::Explicit),
        &mut session(),
    ))?)?;
    assert_eq!(row(&r, MAIN).disposition(), PollDisposition::FastForwarded);
    clean_checkout(&p, None, incoming)?;
    p.finish()
}
fn disconnect_download() -> Result<(), FixtureError> {
    disconnect_download_case(RemotePollInvocation::Explicit)
}
fn disconnect_download_automatic() -> Result<(), FixtureError> {
    disconnect_download_case(RemotePollInvocation::Automatic)
}
fn upload_counter() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    upload_pack_counter_fragmentation()?;
    let n = p.no_push_baseline()?;
    p.no_push_since(n)?;
    p.finish()
}
fn observation_digest(side: &Side) -> Result<[u8; 32], FixtureError> {
    let db = side.db()?;
    let mut hash = blake3::Hasher::new();
    for table in [
        "remote_observation_batches",
        "remote_ref_observations",
        "remote_poll_dispositions",
    ] {
        hash.update(table.as_bytes());
        let mut statement = fixed(db.prepare(&format!("SELECT * FROM {table} ORDER BY rowid")))?;
        let columns = statement.column_count();
        let mut rows = fixed(statement.query([]))?;
        while let Some(row) = fixed(rows.next())? {
            for column in 0..columns {
                hash.update(
                    format!("{:?}", fixed(row.get::<_, rusqlite::types::Value>(column))?)
                        .as_bytes(),
                );
                hash.update(&[0]);
            }
        }
    }
    Ok(*hash.finalize().as_bytes())
}
fn material_failure(point: FailurePoint) -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let tip = publish_new(&p, T)?;
    let service = fixed(RepositoryService::open_at_with_failure_point_for_testing(
        &p.b.data, point,
    ))?;
    let req = p.request(RemotePollInvocation::Explicit);
    assert!(
        matches!(p.poll_with(&service,req.clone(),&mut session()),Err(RemotePollError::Repository(e)) if e.kind==RepositoryErrorKind::InjectedFailure)
    );
    let branch = point != FailurePoint::PollAfterMaterializationIntent;
    let worktree = point == FailurePoint::PollAfterMaterializationWorktree;
    assert_eq!(p.b.repo()?.find_reference(&reference(T)).is_ok(), branch);
    assert_eq!(p.b.worktree(T).exists(), worktree);
    assert_eq!(p.b.repo()?.find_worktree(T).is_ok(), worktree);
    let intent = material_intent(&p)?;
    assert_eq!(intent.0, req.operation_id.to_string());
    assert_eq!(intent.1, tip.to_string());
    assert_eq!(intent.2, if branch { "branch_created" } else { "intended" });
    let before = snapshot(&p.b)?;
    let second = p.b.reopen()?;
    let mut restart = req;
    restart.restart = true;
    let (r, pending) = report(fixed(p.poll_with(&second, restart, &mut session()))?)?;
    assert!(!pending);
    assert_eq!(
        row(&r, &reference(T)).disposition(),
        if worktree {
            PollDisposition::Current
        } else {
            PollDisposition::Materialized
        }
    );
    if worktree {
        assert_eq!(snapshot(&p.b)?, before);
    }
    assert_eq!(
        material_intent(&p)?,
        (intent.0, intent.1, "complete".into())
    );
    discovered_once(&p, T, tip)?;
    p.finish()
}
fn material_intent(p: &Pair) -> Result<(String, String, String), FixtureError> {
    fixed(p.b.db()?.query_row(
        "SELECT operation_ulid,target_oid,step FROM remote_poll_materializations WHERE item_id=?1",
        [T],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    ))
}
fn intent_failure() -> Result<(), FixtureError> {
    material_failure(FailurePoint::PollAfterMaterializationIntent)
}
fn branch_failure() -> Result<(), FixtureError> {
    material_failure(FailurePoint::PollAfterMaterializationBranch)
}
fn worktree_failure() -> Result<(), FixtureError> {
    material_failure(FailurePoint::PollAfterMaterializationWorktree)
}
fn branch_record_failure() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let tip = publish_new(&p, T)?;
    fixed(p.b.db()?.execute_batch("CREATE TRIGGER poll_failure BEFORE UPDATE OF step ON remote_poll_materializations WHEN NEW.step='branch_created' BEGIN SELECT RAISE(ABORT,'fixed fault'); END;"))?;
    let req = p.request(RemotePollInvocation::Explicit);
    assert!(matches!(
        p.poll_with(&p.b.service, req.clone(), &mut session()),
        Err(RemotePollError::Repository(_))
    ));
    assert_eq!(fixed(p.b.repo()?.refname_to_id(&reference(T)))?, tip);
    assert_eq!(material_intent(&p)?.2, "intended");
    assert!(!p.b.worktree(T).exists());
    fixed(p.b.db()?.execute_batch("DROP TRIGGER poll_failure"))?;
    let before = snapshot(&p.b)?;
    let second = p.b.reopen()?;
    let mut restart = req;
    restart.restart = true;
    let (r, _) = report(fixed(p.poll_with(&second, restart, &mut session()))?)?;
    unchanged(
        &r,
        &reference(T),
        PollDisposition::MaterializationIncomplete,
    );
    assert_eq!(snapshot(&p.b)?, before);
    assert_eq!(material_intent(&p)?.2, "intended");
    p.finish()
}
#[cfg(target_os = "linux")]
fn failed_checkout() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let tip = p.publish(
        &reference(T),
        p.base()?,
        &[
            ("a", Some((b"retained", MODE))),
            ("z", Some((&vec![b'x'; 8192], 0o120000))),
        ],
    )?;
    let service = fixed(RepositoryService::open_at_with_failure_point_for_testing(
        &p.b.data,
        FailurePoint::PollAfterFailedMaterializationWorktree,
    ))?;
    let req = p.request(RemotePollInvocation::Explicit);
    assert!(
        matches!(p.poll_with(&service,req.clone(),&mut session()),Err(RemotePollError::Repository(e)) if e.kind==RepositoryErrorKind::InjectedFailure)
    );
    assert_eq!(fixed(p.b.repo()?.refname_to_id(&reference(T)))?, tip);
    assert!(p.b.repo()?.find_worktree(T).is_ok());
    assert_eq!(
        fixed(std::fs::read(p.b.worktree(T).join("a")))?,
        b"retained"
    );
    assert!(p.b.worktree(T).join(".git").is_file());
    let admin = p.b.repo()?.commondir().join("worktrees").join(T);
    for f in ["HEAD", "gitdir", "commondir"] {
        assert!(admin.join(f).is_file());
    }
    assert!(!admin.join("index").exists());
    assert_eq!(material_intent(&p)?.2, "branch_created");
    let before = snapshot(&p.b)?;
    let second = p.b.reopen()?;
    let mut restart = req;
    restart.restart = true;
    let (r, _) = report(fixed(p.poll_with(&second, restart, &mut session()))?)?;
    unchanged(
        &r,
        &reference(T),
        PollDisposition::MaterializationIncomplete,
    );
    assert_eq!(snapshot(&p.b)?, before);
    unchanged(
        &p.poll()?,
        &reference(T),
        PollDisposition::MaterializationIncomplete,
    );
    assert_eq!(snapshot(&p.b)?, before);
    p.finish()
}
fn index_failure(point: FailurePoint) -> Result<(), FixtureError> {
    use rusqlite::OptionalExtension;
    let p = Pair::new()?;
    let tip = p.publish(
        MAIN,
        p.base()?,
        &[("fixture.txt", Some((b"incoming\n", MODE)))],
    )?;
    let service = fixed(RepositoryService::open_at_with_failure_point_for_testing(
        &p.b.data, point,
    ))?;
    let req = p.request(RemotePollInvocation::Explicit);
    let result = p.poll_with(&service, req.clone(), &mut session());
    if point == FailurePoint::PollAfterCompletion {
        assert!(
            matches!(result,Err(RemotePollError::Repository(e)) if e.kind==RepositoryErrorKind::InjectedFailure)
        );
    } else {
        assert!(report(fixed(result)?)?.1);
    }
    clean_checkout(&p, None, tip)?;
    assert!(refresh_required(&p)?);
    let second = p.b.reopen()?;
    let helpers = p.server.helper_invocations();
    let (r, pending) = report(fixed(p.poll_with(&second, req.clone(), &mut session()))?)?;
    assert!(pending);
    assert_eq!(r.completion(), PollCompletion::Completed);
    assert_eq!(row(&r, MAIN).disposition(), PollDisposition::FastForwarded);
    assert_eq!(p.server.helper_invocations(), helpers);
    let id=fixed(fixed(p.b.db()?.query_row("SELECT operation_ulid FROM operation_records WHERE action='refresh' AND state!='completed' ORDER BY id DESC LIMIT 1",[],|r|r.get::<_,String>(0)).optional())?.map(|s|OperationId::parse(&s)).transpose())?.unwrap_or_default();
    assert_ne!(id, req.operation_id);
    assert!(matches!(
        fixed(second.refresh_repository(RefreshRepositoryRequest {
            root: p.b.root.clone(),
            operation_id: id
        }))?,
        RefreshOutcome::Refreshed { .. }
    ));
    assert_eq!(p.server.helper_invocations(), helpers);
    assert!(!refresh_required(&p)?);
    let (replayed, pending) = report(fixed(p.poll_with(&second, req, &mut session()))?)?;
    assert!(!pending);
    assert_eq!(replayed, r);
    assert_eq!(p.server.helper_invocations(), helpers);
    p.finish()
}
fn refresh_required(p: &Pair) -> Result<bool, FixtureError> {
    fixed(
        p.b.db()?
            .query_row("SELECT refresh_required FROM repositories", [], |r| {
                r.get(0)
            }),
    )
}
fn after_completion_failure() -> Result<(), FixtureError> {
    index_failure(FailurePoint::PollAfterCompletion)
}
fn refresh_failure() -> Result<(), FixtureError> {
    index_failure(FailurePoint::BeforeIndexTransactionCommit)
}
fn config_change(mode: u8) -> Result<(), FixtureError> {
    let p = Pair::new()?;
    prepare_transfer(&p)?;
    let gate = p.server.hold_advertisement()?;
    let second = p.b.reopen()?;
    let n = p.no_push_baseline()?;
    std::thread::scope(|scope| -> Result<(), FixtureError> {
        let req = p.request(RemotePollInvocation::Explicit);
        let worker = scope.spawn(|| p.b.service.poll_publication_remote(req, &mut session()));
        gate.wait_until_held()?;
        match mode {
            0 => {
                let config = p.b.root.join(".manyhands/config.toml");
                let source = fixed(std::fs::read_to_string(&config))?;
                fixed(std::fs::write(
                    config,
                    source.replace(
                        "publication_remote = \"origin\"",
                        "publication_remote = \"renamed\"",
                    ),
                ))?;
                fixed(p.b.repo()?.remote_rename("origin", "renamed"))?;
            }
            1 => fixed(
                p.b.repo()?
                    .remote_set_url("origin", "ssh://fixture@127.0.0.1:1/changed"),
            )?,
            _ => {
                fixed(second.clear_shared_key_selection())?;
            }
        }
        let before = snapshot(&p.b)?;
        let refs = all_refs(&p.b.repo()?)?;
        gate.release()?;
        let result = fixed(worker.join())?;
        assert!(matches!(
            result,
            Err(RemotePollError::Interrupted) | Err(RemotePollError::Transport(_))
        ));
        p.redacted(&format!("{result:?}"))?;
        assert_eq!(snapshot(&p.b)?, before);
        assert_eq!(all_refs(&p.b.repo()?)?, refs);
        assert_eq!(count(&p.b, "remote_observation_batches")?, 0);
        Ok(())
    })?;
    p.no_push_since(n)?;
    p.finish()
}
fn config_remote_rename() -> Result<(), FixtureError> {
    config_change(0)
}
fn config_endpoint() -> Result<(), FixtureError> {
    config_change(1)
}
fn config_key() -> Result<(), FixtureError> {
    config_change(2)
}
fn tracking_changed() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let base = p.base()?;
    let tip = p.publish(MAIN, base, &[("fixture.txt", Some((b"incoming\n", MODE)))])?;
    let root = p.b.root.clone();
    p.b.service
        .set_poll_boundary_hook_for_testing(move |point, ordinal| {
            if point == FailurePoint::PollBeforeTarget && ordinal == Some(0) {
                let repo = git2::Repository::open(&root).unwrap();
                repo.reference(
                    "refs/remotes/origin/main",
                    base,
                    true,
                    "owned tracking race",
                )
                .unwrap();
            }
        });
    let before = snapshot(&p.b)?;
    let (r, _) = report(fixed(p.poll_with(
        &p.b.service,
        p.request(RemotePollInvocation::Explicit),
        &mut session(),
    ))?)?;
    assert_eq!(r.completion(), PollCompletion::Interrupted);
    assert_eq!(r.targets()[0].disposition(), PollDisposition::NotReached);
    assert_eq!(fixed(p.b.repo()?.refname_to_id(MAIN))?, base);
    assert_eq!(
        fixed(p.b.repo()?.refname_to_id("refs/remotes/origin/main"))?,
        base
    );
    assert!(p.b.repo()?.find_commit(tip).is_ok());
    assert_eq!(snapshot(&p.b)?, before);
    assert_eq!(count(&p.b, "remote_observation_batches")?, 1);
    p.finish()
}
fn database_privacy() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let db = p.b.db()?;
    fixed(db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0;"))?;
    let ticket = publish_new(&p, T)?;
    let document = publish_new(&p, D)?;
    let success = p.poll()?;
    assert_eq!(
        row(&success, &reference(T)).disposition(),
        PollDisposition::Materialized
    );
    assert_eq!(
        row(&success, &reference(D)).disposition(),
        PollDisposition::Materialized
    );
    discovered_once(&p, T, ticket)?;
    discovered_once(&p, D, document)?;
    for title in [TICKET_TITLE, DOCUMENT_TITLE] {
        assert!(
            fixed(db.query_row(
                "SELECT count(*) FROM discovered_items WHERE title=?1",
                [title],
                |r| r.get::<_, i64>(0)
            ))? > 0
        );
        // Probe controls: only canonical/raw-store scans may allow these titles.
        ssh_privacy::clean(title.as_bytes(), &p.probes)?;
        assert!(ssh_privacy::clean(title.as_bytes(), &p.metadata_probes).is_err());
        assert!(p.redacted(title).is_err());
    }
    let new = "01ARZ3NDEKTSV4RRFFQ69G5FAX";
    p.publish(
        &reference(new),
        p.base()?,
        &[(
            "docs/private.md",
            Some((source(new, TITLE, BODY).as_bytes(), 0o100755)),
        )],
    )?;
    p.poll()?;
    let backup = p.b.data.join("v");
    fixed(db.execute("VACUUM INTO ?1", [backup.to_string_lossy().as_ref()]))?;
    let backup_db = fixed(rusqlite::Connection::open(&backup))?;
    scan_recovery_rows(&backup_db, &p.metadata_probes)?;
    for title in [TICKET_TITLE, DOCUMENT_TITLE] {
        assert!(
            fixed(backup_db.query_row(
                "SELECT count(*) FROM discovered_items WHERE title=?1",
                [title],
                |r| r.get::<_, i64>(0)
            ))? > 0
        );
    }
    // The same scanner must refuse accepted titles in recovery metadata even
    // though their legitimate canonical index content is allowed in raw stores.
    fixed(backup_db.execute_batch("CREATE TABLE remote_title_probe(value TEXT);"))?;
    for title in [TICKET_TITLE, DOCUMENT_TITLE] {
        fixed(backup_db.execute("INSERT INTO remote_title_probe VALUES(?1)", [title]))?;
        assert!(scan_recovery_rows(&backup_db, &p.metadata_probes).is_err());
        fixed(backup_db.execute("DELETE FROM remote_title_probe", []))?;
    }
    fixed(backup_db.execute_batch("DROP TABLE remote_title_probe;"))?;
    scan_recovery_rows(&backup_db, &p.metadata_probes)?;
    fixed(db.execute_batch("BEGIN IMMEDIATE; CREATE TABLE privacy_fixed(category TEXT); INSERT INTO privacy_fixed VALUES('fixed');"))?;
    let inventory = ssh_privacy::scan(&p.b.data, &p.probes)?;
    let expected_backup = fixed(std::fs::canonicalize(&backup))?;
    assert!(inventory.iter().any(|(path, n)| {
        path.canonicalize()
            .is_ok_and(|path| path == expected_backup)
            && *n > 0
    }));
    assert!(inventory.iter().any(|(path, n)| {
        path.file_name()
            .is_some_and(|name| name.to_string_lossy().ends_with("-wal"))
            && *n > 0
    }));
    assert!(count(&p.b, "remote_poll_dispositions")? > 0);
    assert!(count(&p.b, "remote_poll_materializations")? > 0);
    p.redacted(&format!(
        "{:?}",
        fixed(p.b.service.remote_snapshot(&p.b.root))?
    ))?;
    fixed(db.execute_batch("ROLLBACK"))?;
    p.finish()
}
const CONTROL: &[u8] = b"poll-private-output-control-canary";
const OUTPUT_CONTROLS: &[ssh_harness::Case] = &[
    ("poll_control_stdout", control_stdout),
    ("poll_control_stderr_failure", control_stderr),
    ("poll_control_clean_failure", control_failure),
    ("poll_control_missing_inventory", control_missing),
    (
        "poll_control_accepted_document_title_stdout",
        control_document_title,
    ),
    (
        "poll_control_accepted_ticket_title_stderr_failure",
        control_ticket_title,
    ),
];
fn control_document_title() -> Result<(), FixtureError> {
    use std::io::Write;
    ssh_privacy::save(&[DOCUMENT_TITLE.as_bytes().to_vec()])?;
    fixed(std::io::stdout().write_all(DOCUMENT_TITLE.as_bytes()))
}
fn control_ticket_title() -> Result<(), FixtureError> {
    use std::io::Write;
    ssh_privacy::save(&[TICKET_TITLE.as_bytes().to_vec()])?;
    let mut bytes = vec![b'x'; 20_478];
    bytes.extend_from_slice(TICKET_TITLE.as_bytes());
    fixed(std::io::stderr().write_all(&bytes))?;
    Err(FixtureError)
}
fn control_stdout() -> Result<(), FixtureError> {
    use std::io::Write;
    ssh_privacy::save(&[CONTROL.to_vec()])?;
    fixed(std::io::stdout().write_all(CONTROL))
}
fn control_stderr() -> Result<(), FixtureError> {
    use std::io::Write;
    ssh_privacy::save(&[CONTROL.to_vec()])?;
    let mut bytes = vec![b'x'; 20_478];
    bytes.extend_from_slice(CONTROL);
    fixed(std::io::stderr().write_all(&bytes))?;
    Err(FixtureError)
}
fn control_failure() -> Result<(), FixtureError> {
    ssh_privacy::save(&[CONTROL.to_vec()])?;
    Err(FixtureError)
}
fn control_missing() -> Result<(), FixtureError> {
    Ok(())
}
fn capture_controls() -> Result<(), FixtureError> {
    use ssh_harness::IsolationFailure as F;
    let p = Pair::new()?;
    for (case, expected) in [
        ("poll_control_stdout", F::OutputPrivacy),
        ("poll_control_stderr_failure", F::OutputPrivacy),
        ("poll_control_clean_failure", F::Child),
        ("poll_control_missing_inventory", F::ProbeInventory),
        (
            "poll_control_accepted_document_title_stdout",
            F::OutputPrivacy,
        ),
        (
            "poll_control_accepted_ticket_title_stderr_failure",
            F::OutputPrivacy,
        ),
    ] {
        assert_eq!(
            ssh_harness::run_isolated_with_output_privacy(case),
            Err(expected)
        );
    }
    let n = p.no_push_baseline()?;
    p.no_push_since(n)?;
    p.finish()
}
