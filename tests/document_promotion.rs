//! Public-API document promotion acceptance against two isolated SSH clones.
//! Exercises the owner-approved retained-R1 and discovery-title metadata bounds.
#![allow(clippy::result_large_err)]
pub use manyhands::*;
#[path = "document_promotion/journeys.rs"]
mod journeys;
#[path = "support/privacy_rows.rs"]
mod privacy_rows;
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

const DOC: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAX";
const DOCUMENT_PATH: &str = "docs/a.md";
const MAIN: &str = "refs/heads/main";
const CONTEXT: &str = "refs/heads/manyhands/document/01ARZ3NDEKTSV4RRFFQ69G5FAX";
const TITLE: &str = "promotion-private-title-canary";
const BODY: &str = "promotion-private-document-body-canary\n";
const COMMENT: &str = "promotion-private-comment-body-canary\n";
const REJECTED_TITLE: &str = "promotion-private-nonindexed-title-canary";

fn main() {
    // SAFETY: the real main owns initialization before any fixture/runner thread.
    if unsafe { ssh_harness::initialize() }.is_err() {
        eprintln!("document promotion initialization failed");
        std::process::exit(1);
    }
    if journeys::maybe_exit_child() {
        return;
    }
    let cases: Vec<_> = CASES.iter().chain(journeys::CASES).copied().collect();
    ssh_harness::run_with_short_output_privacy(&cases, OUTPUT_CONTROLS);
}

const CASES: &[ssh_harness::Case] = &[
    (
        "first_publication_deletes_verified_context_oid",
        first_publication_deletes_verified_context_oid,
    ),
    (
        "later_publication_deletes_verified_not_stale_tracking_oid",
        later_publication_deletes_verified_oid,
    ),
    ("privacy_metadata_checkpointed", || {
        privacy_title_policy(false)
    }),
    ("privacy_metadata_pinned_wal", || privacy_title_policy(true)),
    (
        "privacy_rejected_nonindexed_title_input",
        rejected_title_input,
    ),
    ("capture_privacy_controls_fail_closed", capture_controls),
    (
        "cache_loss_fresh_edit_remains_unknown_across_reopen",
        cache_loss_fresh_edit,
    ),
];
const OUTPUT_CONTROLS: &[ssh_harness::Case] = &[
    ("control_title_output", || output_control(TITLE)),
    ("control_body_output", || output_control(BODY)),
    ("control_comment_output", || output_control(COMMENT)),
    ("control_passphrase_output", || {
        output_control(remote_world::PASSWORD)
    }),
    ("control_endpoint_output", endpoint_output_control),
];

struct Side {
    service: RepositoryService,
    root: PathBuf,
    data: PathBuf,
}
impl Side {
    fn repo(&self) -> Result<git2::Repository, FixtureError> {
        fixed(git2::Repository::open(&self.root))
    }
    fn worktree(&self) -> PathBuf {
        self.root.join(".manyhands/worktrees").join(DOC)
    }
    fn db(&self) -> Result<rusqlite::Connection, FixtureError> {
        fixed(rusqlite::Connection::open(self.data.join(REGISTRY_FILE)))
    }
    fn target(&self) -> AuthoringTarget {
        AuthoringTarget {
            root: self.root.clone(),
            kind: AuthoringKind::Document,
            item_id: DOC.parse().unwrap(),
            intent: ContextIntent::Edit,
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
            REJECTED_TITLE.as_bytes().to_vec(),
            BODY.as_bytes().to_vec(),
            BODY.trim_end().as_bytes().to_vec(),
            COMMENT.as_bytes().to_vec(),
            COMMENT.trim_end().as_bytes().to_vec(),
        ];
        probes.extend(
            plain
                .split(|byte| *byte == b'\n')
                .filter(|line| line.len() > 40)
                .map(<[u8]>::to_vec),
        );
        probes.extend(
            encrypted
                .as_bytes()
                .split(|byte| *byte == b'\n')
                .filter(|line| line.len() > 40)
                .map(<[u8]>::to_vec),
        );
        ssh_privacy::save(&probes)?;
        let directory = fixed(tempfile::Builder::new().prefix("dp").tempdir())?;
        let root = directory.path().join("a");
        let repo = remote_world::clone_owned(&server, &root)?;
        identity(&repo)?;
        let service = fixed(RepositoryService::open_at(&directory.path().join("da")))?;
        fixed(service.enable(EnableRepositoryRequest {
            root: root.clone(),
            primary_branch: "main".into(),
            identity: None,
            operation_id: OperationId::new(),
        }))?;
        let source = fixed(canonical::serialize_repository_config(
            &canonical::RepositoryConfig {
                primary_branch: "main".into(),
                publication_remote: remote.then(|| "origin".into()),
                unknown: toml::Table::new(),
            },
        ))?;
        let document = fixed(canonical::serialize_item(
            &canonical::CanonicalItem::Document(canonical::Document {
                id: DOC.parse().unwrap(),
                title: "Shared document".into(),
                body: "Seed\n".into(),
                unknown: serde_yaml::Mapping::new(),
            }),
        ))?;
        commit_files(
            &repo,
            &[
                (".manyhands/config.toml", source.as_bytes()),
                (DOCUMENT_PATH, document.as_bytes()),
            ],
        )?;
        remote_world::push_peer(&repo, &server, MAIN)?;
        let other = directory.path().join("b");
        identity(&remote_world::clone_owned(&server, &other)?)?;
        let b_service = fixed(RepositoryService::open_at(&directory.path().join("db")))?;
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
                data: directory.path().join("da"),
            },
            b: Side {
                service: b_service,
                root: other,
                data: directory.path().join("db"),
            },
            probes,
            _directory: directory,
        };
        // The fixture's anonymous seed push does not update origin/main.
        // Observe it through the actual public boundary before testing D2.
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
        self.sync_target(side, document_target())
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
    fn save(&self, side: &Side) -> Result<git2::Oid, FixtureError> {
        self.save_body(side, BODY)
    }
    fn save_body(&self, side: &Side, body: &str) -> Result<git2::Oid, FixtureError> {
        let root = if side.worktree().exists() {
            side.worktree()
        } else {
            side.root.clone()
        };
        let observed =
            ExpectedPathObservation::from_bytes(&fixed(std::fs::read(root.join(DOCUMENT_PATH)))?);
        let result = fixed(side.service.save_document(SaveDocumentRequest {
            target: side.target(),
            source_path: Some(DOCUMENT_PATH.into()),
            destination_path: DOCUMENT_PATH.into(),
            draft: DocumentDraft {
                title: TITLE.into(),
                body: body.into(),
            },
            expected_source: Some(observed.clone()),
            expected_destination: observed,
        }))?;
        match result {
            SaveOutcome::Saved {
                checkpoint: LocalCheckpoint::Checkpointed { commit_oid },
                ..
            } => Ok(commit_oid),
            _ => Err(FixtureError),
        }
    }
    fn preview(&self) -> Result<DocumentPromotionPreview, FixtureError> {
        let preflight = fixed(self.a.service.preview_document_promotion(
            PreviewDocumentPromotionRequest {
                root: self.a.root.clone(),
                item_id: DOC.parse().unwrap(),
            },
        ))?;
        self.diagnostics(&format!("{preflight:?}"))?;
        match preflight {
            DocumentPromotionPreflight::Ready(p) => Ok(p),
            DocumentPromotionPreflight::Blocked { blockers } => {
                for b in blockers {
                    ssh_harness::observation(&[800, b as u128]);
                }
                Err(FixtureError)
            }
        }
    }
    fn promote(&self) -> Result<DocumentPromotionOutcome, FixtureError> {
        let request = self.request().expect("promotion preview ready");
        match self
            .execute(&self.a.service, request)
            .expect("promotion succeeds")
        {
            DocumentPromotionResult::Complete(outcome) => Ok(outcome),
            DocumentPromotionResult::IndexPending(_) => {
                ssh_harness::observation(&[801]);
                Err(FixtureError)
            }
        }
    }
    fn request(&self) -> Result<PromoteDocumentRequest, FixtureError> {
        let preview = self.preview()?;
        Ok(PromoteDocumentRequest {
            root: self.a.root.clone(),
            operation_id: OperationId::new(),
            item_id: DOC.parse().unwrap(),
            expected: preview.expectation(),
            approval: Some(self.approval()),
            confirmed_identity: None,
        })
    }
    fn execute(
        &self,
        service: &RepositoryService,
        request: PromoteDocumentRequest,
    ) -> Result<DocumentPromotionResult, DocumentPromotionError> {
        let result = service.promote_document(
            request,
            &mut SessionCredentials::new(remote_world::Provider),
        );
        self.diagnostics(&format!("{result:?}"))
            .expect("redacted promotion result");
        result
    }
    fn diagnostics(&self, rendered: &str) -> Result<(), FixtureError> {
        ssh_privacy::clean(rendered.as_bytes(), &self.probes)?;
        for side in [&self.a, &self.b] {
            let path = side.worktree().to_string_lossy().as_bytes().to_vec();
            ssh_privacy::clean(rendered.as_bytes(), &[path])?;
        }
        Ok(())
    }
    fn privacy(&self) -> Result<(), FixtureError> {
        ssh_privacy::save(&self.probes)?;
        for side in [&self.a, &self.b] {
            ssh_privacy::save(&[side.worktree().to_string_lossy().as_bytes().to_vec()])?;
            let db = side.db()?;
            let paths = vec![side.worktree().to_string_lossy().as_bytes().to_vec()];
            scan_workflow_rows(&db, &self.probes)?;
            scan_workflow_rows(&db, &paths)?;
            scan_rows(&db, &self.probes, true)?;
            let backup = side.data.join("v");
            if backup.exists() {
                fixed(std::fs::remove_file(&backup))?;
            }
            fixed(db.execute("VACUUM INTO ?1", [backup.to_str().ok_or(FixtureError)?]))?;
            let backup_db = fixed(rusqlite::Connection::open(&backup))?;
            scan_workflow_rows(&backup_db, &self.probes)?;
            scan_workflow_rows(&backup_db, &paths)?;
            scan_rows(&backup_db, &self.probes, true)?;
            // Only the accepted discovered_items.title metadata/copies are
            // exempt in raw stores. All workflow/backup rows use ALL probes.
            let store_probes: Vec<_> = self
                .probes
                .iter()
                .filter(|p| p.as_slice() != TITLE.as_bytes())
                .cloned()
                .collect();
            ssh_privacy::scan(&side.data, &store_probes)?;
        }
        Ok(())
    }
}

fn document_target() -> SynchronizationTarget {
    SynchronizationTarget::Context {
        kind: AuthoringKind::Document,
        item_id: DOC.parse().unwrap(),
    }
}
fn identity(repo: &git2::Repository) -> Result<(), FixtureError> {
    let mut config = fixed(repo.config())?;
    fixed(config.set_str("user.name", "Fixture"))?;
    fixed(config.set_str("user.email", "fixture@example.invalid"))
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
fn published(result: SynchronizationResult) -> Result<git2::Oid, FixtureError> {
    match result {
        SynchronizationResult::Complete(SynchronizationOutcome::Published { oid, .. }) => Ok(oid),
        _ => Err(FixtureError),
    }
}

fn first_publication_deletes_verified_context_oid() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let cursor = p.server.receive_updates().len();
    let base = fixed(p.a.repo()?.refname_to_id(MAIN))?;
    let context = p.save(&p.a)?;
    let outcome = p.promote()?;
    let privacy = p.privacy();
    assert_eq!(outcome.cleanup(), PromotionCleanup::Complete);
    let PromotionIntegration::Merged { oid } = outcome.integration() else {
        return Err(FixtureError);
    };
    let repo = p.a.repo()?;
    let merge = fixed(repo.find_commit(oid))?;
    assert_eq!(
        merge.summary(),
        Some(format!("Promote document {DOC}").as_str())
    );
    assert_eq!(merge.parent_ids().collect::<Vec<_>>(), [base, context]);
    assert_eq!(
        fixed(git2::Repository::open_bare(p.server.repository_path()))?
            .refname_to_id(MAIN)
            .ok(),
        Some(oid)
    );
    let updates = p.server.receive_updates();
    assert_eq!(updates.len() - cursor, 3);
    assert!(updates[cursor..].iter().all(|u| u.accepted));
    assert!(
        p.server
            .receive_updates()
            .iter()
            .any(|update| update.reference == CONTEXT
                && update.old_oid == context
                && update.new_oid.is_zero()
                && update.accepted)
    );
    assert!(!p.a.worktree().exists());
    assert!(p.a.repo()?.find_reference(CONTEXT).is_err());
    assert!(
        fixed(git2::Repository::open_bare(p.server.repository_path()))?
            .find_reference(CONTEXT)
            .is_err()
    );
    privacy
}

fn later_publication_deletes_verified_oid() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let old = p.save(&p.a)?;
    assert_eq!(published(fixed(p.sync(&p.a))?)?, old);
    // An already-current sync fetches T(C)=old. The later save/publication
    // verifies a new remote tip while the tracking observation remains old.
    fixed(p.sync(&p.a))?;
    let edited = p.save_body(&p.a, &(BODY.to_owned() + "later edit\n"))?;
    assert_ne!(edited, old);
    let outcome = p.promote()?;
    p.privacy()?;
    assert_eq!(outcome.cleanup(), PromotionCleanup::Complete);
    assert!(
        p.server
            .receive_updates()
            .iter()
            .any(|update| update.reference == CONTEXT
                && update.old_oid == edited
                && update.new_oid.is_zero()
                && update.accepted)
    );
    assert!(!p.a.worktree().exists());
    assert!(p.a.repo()?.find_reference(CONTEXT).is_err());
    Ok(())
}

fn privacy_title_policy(pinned: bool) -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let readers = [p.a.db()?, p.b.db()?];
    if pinned {
        for reader in &readers {
            fixed(reader.execute_batch("BEGIN; SELECT COUNT(*) FROM document_promotions;"))?;
        }
    }
    p.save(&p.a)?;
    journeys::comment(&p)?;
    p.promote()?;
    // B must independently discover the accepted title through the public poll,
    // not through an injected discovery row or shared application data.
    journeys::poll(&p, &p.b)?;
    if !pinned {
        for reader in &readers {
            let busy: i64 =
                fixed(reader.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| r.get(0)))?;
            assert_eq!(busy, 0);
        }
    }
    p.privacy()?;
    for (side_index, side) in [&p.a, &p.b].into_iter().enumerate() {
        assert_eq!(
            fixed(side.db()?.query_row(
                "SELECT COUNT(*) FROM discovered_items WHERE title=?1",
                [TITLE],
                |r| r.get::<_, i64>(0),
            ))?,
            1,
            "each live clone must index the accepted title"
        );
        let backup = fixed(rusqlite::Connection::open(side.data.join("v")))?;
        assert_eq!(
            fixed(backup.query_row(
                "SELECT COUNT(*) FROM discovered_items WHERE title=?1",
                [TITLE],
                |r| r.get::<_, i64>(0),
            ))?,
            1,
            "each backup must contain accepted discovery metadata"
        );
        let inventory = [
            REGISTRY_FILE.to_owned(),
            format!("{REGISTRY_FILE}-wal"),
            "v".into(),
        ];
        let mut title_copies = Vec::new();
        for (index, name) in inventory.into_iter().enumerate() {
            let path = side.data.join(name);
            let bytes = if index == 1 && !pinned && !path.exists() {
                Vec::new()
            } else {
                fixed(std::fs::read(path))?
            };
            title_copies.push(ssh_privacy::clean(&bytes, &[TITLE.as_bytes().to_vec()]).is_err());
            ssh_harness::observation(&[
                31,
                side_index as u128,
                index as u128,
                title_copies[index] as u128,
            ]);
        }
        assert!(title_copies[2]);
        assert!(title_copies[usize::from(pinned)]);
    }
    // Record positive copies before any negative SQL injection could itself
    // place the title into WAL/backup pages.
    drop(readers);
    let mut negative_controls = 0;
    for (side_index, side) in [&p.a, &p.b].into_iter().enumerate() {
        workflow_title_control(&side.db()?, &p.probes)?;
        negative_controls += 1;
        ssh_harness::observation(&[32, side_index as u128, 0]);
        let backup = fixed(rusqlite::Connection::open(side.data.join("v")))?;
        workflow_title_control(&backup, &p.probes)?;
        negative_controls += 1;
        ssh_harness::observation(&[32, side_index as u128, 1]);
    }
    assert_eq!(negative_controls, 4);
    assert!(p.diagnostics(TITLE).is_err());
    assert!(ssh_privacy::clean(REJECTED_TITLE.as_bytes(), &p.probes).is_err());
    p.privacy()?;
    Ok(())
}

fn operation_rows_snapshot(db: &rusqlite::Connection) -> Result<[u8; 32], FixtureError> {
    let mut query = fixed(db.prepare("SELECT * FROM operation_records ORDER BY id"))?;
    let columns = query.column_count();
    let rows = fixed(
        fixed(query.query_map([], |r| {
            (0..columns)
                .map(|c| r.get::<_, rusqlite::types::Value>(c))
                .collect::<Result<Vec<_>, _>>()
        }))?
        .collect::<Result<Vec<_>, _>>(),
    )?;
    Ok(*blake3::hash(format!("{rows:?}").as_bytes()).as_bytes())
}

fn workflow_title_control(
    db: &rusqlite::Connection,
    probes: &[Vec<u8>],
) -> Result<(), FixtureError> {
    scan_workflow_rows(db, probes)?;
    scan_rows(db, probes, true)?;
    let before = operation_rows_snapshot(db)?;
    let (id, original): (i64, Option<String>) = fixed(db.query_row(
        "SELECT id,target FROM operation_records ORDER BY id LIMIT 1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ))?;
    let transaction = fixed(db.unchecked_transaction())?;
    // Use a real existing operation journal row and its free-text metadata
    // target, leaving its action/state/category/identity constraints intact.
    // The only write is fixture-owned, transactional and rolled back before
    // any service call; no schema bypass or synthetic probe table is involved.
    assert_eq!(
        fixed(transaction.execute(
            "UPDATE operation_records SET target=?1 WHERE id=?2",
            rusqlite::params![format!("fixture/{TITLE}"), id],
        ))?,
        1
    );
    let actual: bool = fixed(transaction.query_row(
        "SELECT instr(target,?1)>0 FROM operation_records WHERE id=?2",
        rusqlite::params![TITLE, id],
        |r| r.get(0),
    ))?;
    assert!(
        actual,
        "the negative control must contaminate an actual workflow row"
    );
    assert_ne!(operation_rows_snapshot(&transaction)?, before);
    assert!(scan_workflow_rows(&transaction, probes).is_err());
    assert!(scan_rows(&transaction, probes, true).is_err());
    fixed(transaction.rollback())?;
    assert_eq!(operation_rows_snapshot(db)?, before);
    assert_eq!(
        fixed(db.query_row(
            "SELECT target FROM operation_records WHERE id=?1",
            [id],
            |r| r.get::<_, Option<String>>(0),
        ))?,
        original
    );
    scan_workflow_rows(db, probes)?;
    scan_rows(db, probes, true)
}

fn output_control(secret: &str) -> Result<(), FixtureError> {
    ssh_privacy::save(&[secret.as_bytes().to_vec()])?;
    // Only child-only controls print canaries. The parent scans before emitting
    // any child diagnostic; these bytes must never be forwarded.
    eprintln!("{secret}");
    Ok(())
}
fn endpoint_output_control() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.privacy()?;
    eprintln!("{}", p.server.url());
    Ok(())
}
fn capture_controls() -> Result<(), FixtureError> {
    ssh_privacy::save(&[b"promotion-control-private".to_vec()])?;
    for (case, _) in OUTPUT_CONTROLS {
        assert_eq!(
            ssh_harness::run_isolated_with_output_privacy(case),
            Err(ssh_harness::IsolationFailure::OutputPrivacy)
        );
    }
    ssh_harness::raw_output_regression()?;
    ssh_harness::capture_limit_regression()
}

fn rejected_title_input() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.save(&p.a)?;
    let before = fixed(std::fs::read(p.a.worktree().join(DOCUMENT_PATH)))?;
    let branch = fixed(p.a.repo()?.refname_to_id(CONTEXT))?;
    let expected = ExpectedPathObservation::from_bytes(&before);
    let request = SaveDocumentRequest {
        target: p.a.target(),
        source_path: Some(DOCUMENT_PATH.into()),
        destination_path: DOCUMENT_PATH.into(),
        draft: DocumentDraft {
            title: format!("{REJECTED_TITLE}\0"),
            body: BODY.to_owned() + COMMENT,
        },
        expected_source: Some(expected.clone()),
        expected_destination: expected,
    };
    let operation = request.target.operation_id;
    // This title crosses the actual authoring boundary and fails canonical NUL
    // validation before a write. Never format the request/draft itself.
    let error = match p.a.service.save_document(request) {
        Err(error) => error,
        Ok(_) => panic!("invalid canonical title must be rejected"),
    };
    assert_eq!(error.kind, RepositoryErrorKind::InvalidPath);
    p.diagnostics(&format!("{error:?} {error}"))?;
    assert_eq!(error.operation, RepositoryOperation::SaveDocument);
    assert!(
        error
            .to_string()
            .ends_with("title must be non-empty and contain no NUL characters")
    );
    assert!(fixed(std::fs::read(p.a.worktree().join(DOCUMENT_PATH)))? == before);
    assert_eq!(fixed(p.a.repo()?.refname_to_id(CONTEXT))?, branch);
    // The evidence must include a real attempted authoring operation, not only
    // a declared scanner probe that never enters production.
    assert_eq!(
        fixed(p.a.db()?.query_row(
            "SELECT COUNT(*) FROM operation_records WHERE operation_ulid=?1",
            [operation.to_string()],
            |r| r.get::<_, i64>(0),
        ))?,
        1
    );
    for side in [&p.a, &p.b] {
        let snapshot = fixed(side.service.repository_snapshot(&side.root))?;
        assert!(
            !snapshot
                .items
                .iter()
                .any(|item| item.title.contains(REJECTED_TITLE))
        );
        assert_eq!(
            fixed(side.db()?.query_row(
                "SELECT COUNT(*) FROM discovered_items WHERE instr(title,?1)>0",
                [REJECTED_TITLE],
                |r| r.get::<_, i64>(0),
            ))?,
            0
        );
    }
    p.privacy()
}

/// The approved Task 7 cache-loss sequence: no restoration of the erased
/// retirement row and no manufactured history or mocked synchronization.
fn cache_loss_fresh_edit() -> Result<(), FixtureError> {
    let mut p = Pair::new()?;
    p.save(&p.a)?;
    let outcome = p.promote()?;
    assert_eq!(outcome.cleanup(), PromotionCleanup::Complete);
    assert_eq!(
        fixed(
            p.a.db()?
                .query_row("SELECT COUNT(*) FROM retired_contexts", [], |r| r
                    .get::<_, i64>(0))
        )?,
        1
    );
    let merged = fixed(p.a.repo()?.refname_to_id(MAIN))?;
    assert!(p.a.repo()?.find_reference(CONTEXT).is_err());
    assert!(!p.a.worktree().exists());
    let first_privacy = p.privacy();

    // The service has no persistent connection. Remove the actual registry and
    // sidecars. There is no independently durable per-item retirement evidence.
    for entry in fixed(std::fs::read_dir(&p.a.data))? {
        let path = fixed(entry)?.path();
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with(REGISTRY_FILE))
        {
            fixed(std::fs::remove_file(path))?;
        }
    }
    // Exercise the established public corrupt-cache recovery path, which marks
    // history unknown. A merely missing registry is opened as a new store.
    fixed(std::fs::write(
        p.a.data.join(REGISTRY_FILE),
        b"lost registry",
    ))?;
    p.a.service = fixed(RepositoryService::open_at(&p.a.data))?;
    fixed(p.a.service.rebuild_repository(RebuildRepositoryRequest {
        root: p.a.root.clone(),
        operation_id: OperationId::new(),
    }))?;
    select_key(&p.a.service, &p.server)?;
    assert_eq!(
        fixed(
            p.a.db()?
                .query_row("SELECT COUNT(*) FROM retired_contexts", [], |r| r
                    .get::<_, i64>(0))
        )?,
        0
    );
    let context = fixed(p.a.service.prepare_context(p.a.target()))?;
    assert!(matches!(context, ContextProvisionOutcome::Created(_)));
    assert_eq!(fixed(p.a.repo()?.refname_to_id(CONTEXT))?, merged);
    // A distinct new save, rather than an unchanged document, must be eligible.
    let edited = p.save_body(&p.a, &(BODY.to_owned() + "fresh work\n"))?;
    let snapshot = fixed(p.a.service.remote_snapshot(&p.a.root))?;
    let evidence =
        snapshot.publication_evidence_for(AuthoringKind::Document, &DOC.parse().unwrap());
    ssh_harness::observation(&[match evidence {
        RemotePublicationEvidence::NeverPublished => 1,
        RemotePublicationEvidence::ObservedPublished => 2,
        RemotePublicationEvidence::HistoryUnknown => 4,
    }]);
    let receives = p.server.receive_updates();
    let image = journeys::image(&p.a.worktree())?;
    let mut synchronization = SynchronizeRemoteRequest {
        root: p.a.root.clone(),
        operation_id: OperationId::new(),
        target: document_target(),
        restart: false,
        confirmed_identity: None,
        approval: Some(p.approval()),
    };
    let publication = p.a.service.synchronize_remote(
        synchronization.clone(),
        &mut SessionCredentials::new(remote_world::Provider),
    );
    p.diagnostics(&format!("{publication:?}"))?;
    // Always scan the failure state before the contract assertion can panic.
    let final_privacy = p.privacy();
    ssh_harness::observation(&[
        20,
        matches!(publication, Err(SynchronizationError::HistoryUnknown)) as u128,
        first_privacy.is_ok() as u128,
        final_privacy.is_ok() as u128,
    ]);
    assert_eq!(fixed(p.a.repo()?.refname_to_id(MAIN))?, merged);
    assert_eq!(fixed(p.a.repo()?.refname_to_id(CONTEXT))?, edited);
    assert!(
        fixed(git2::Repository::open_bare(p.server.repository_path()))?
            .find_reference(CONTEXT)
            .is_err()
    );
    assert_eq!(evidence, RemotePublicationEvidence::HistoryUnknown);
    assert!(matches!(
        publication,
        Err(SynchronizationError::HistoryUnknown)
    ));
    assert_eq!(p.server.receive_updates(), receives);
    assert_eq!(journeys::image(&p.a.worktree())?, image);
    p.a.service = fixed(RepositoryService::open_at(&p.a.data))?;
    assert_eq!(
        fixed(
            p.a.db()?
                .query_row("SELECT COUNT(*) FROM retired_contexts", [], |r| r
                    .get::<_, i64>(0))
        )?,
        0
    );
    synchronization.restart = true;
    let replay = p.a.service.synchronize_remote(
        synchronization,
        &mut SessionCredentials::new(remote_world::Provider),
    );
    p.diagnostics(&format!("{replay:?}"))?;
    assert!(matches!(replay, Err(SynchronizationError::HistoryUnknown)));
    assert_eq!(p.server.receive_updates(), receives);
    assert_eq!(journeys::image(&p.a.worktree())?, image);
    first_privacy?;
    final_privacy?;
    p.privacy()
}

/// The column-scoped title exception, now in `tests/support/privacy_rows.rs`
/// (Cycle 10 named edit 4): the same tables, required table and exemption.
const ROW_POLICY: privacy_rows::RowPolicy<'static> = privacy_rows::RowPolicy {
    workflow: &[
        "document_promotions",
        "retired_contexts",
        "operation_records",
        "comment_publication_bindings",
    ],
    required: "document_promotions",
    exempt: &[("title", TITLE.as_bytes())],
};
fn scan_workflow_rows(db: &rusqlite::Connection, probes: &[Vec<u8>]) -> Result<(), FixtureError> {
    privacy_rows::scan_workflow_rows(db, probes, &ROW_POLICY)
}
fn scan_rows(
    db: &rusqlite::Connection,
    probes: &[Vec<u8>],
    include_cache: bool,
) -> Result<(), FixtureError> {
    privacy_rows::scan_rows(db, probes, include_cache, &ROW_POLICY)
}
