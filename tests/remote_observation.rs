pub use manyhands::{repository, runtime};
use std::fs;

use manyhands::repository::{
    AuthoringKind, OperationId, PollingInterval, REGISTRY_FILE, RebuildRepositoryRequest,
    RemotePublicationEvidence, RepositoryErrorKind, RepositoryService,
};
use repository::{
    AuthoringTarget, ContextIntent, ContextProvisionOutcome, ObservePublicationRemoteRequest,
    RemoteContextState, RemoteOutcomeCategory, RemotePollInvocation, RemoteRefClassification,
    keys::*, transport::*,
};
use rusqlite::Connection;
use ssh_remote::{ObservationRef, SshRemoteFixture};

#[path = "support/ssh_harness.rs"]
mod ssh_harness;
#[path = "support/ssh_privacy.rs"]
mod ssh_privacy;
#[path = "support/ssh_remote.rs"]
mod ssh_remote;
mod support;

fn main() {
    // SAFETY: real main, before test, runtime, or watchdog threads exist.
    if unsafe { ssh_harness::initialize() }.is_err() {
        eprintln!("remote observation initialization failed");
        std::process::exit(1);
    }
    macro_rules! cases {
        ($($name:ident),+ $(,)?) => {
            ssh_harness::run(&[$((stringify!($name), || { $name(); Ok(()) })),+]);
        };
    }
    cases!(
        migration_preserves_cycle03_rows_and_installs_one_default_policy,
        cache_replacement_publishes_remote_history_marker,
        remote_marker_failure_aborts_replacement_before_renaming_registry,
        polling_policy_survives_reopen_and_local_snapshot_stays_intact,
        corrupt_remote_policy_requires_recovery_without_repair_or_raw_error,
        absent_context_remains_history_unknown_after_cache_loss_and_restart,
        explicit_recovery_resume_preserves_pause_and_unknown_history,
        reopening_does_not_repair_deleted_policy_or_partial_remote_schema,
        authenticated_exceptional_states_preserve_local_contexts,
        publication_remote_lifecycle_invalidates_observations,
        same_name_endpoint_replacement_after_reopen_is_not_remote_deletion,
        authenticated_absence_after_cache_loss_stays_history_unknown,
        authenticated_poll_yields_and_cancels_before_publishing,
        abandoned_advertisement_hold_aborts_without_publishing,
        authenticated_batch_failure_is_atomic_and_restartable,
    );
}

fn migration_preserves_cycle03_rows_and_installs_one_default_policy() {
    let data = tempfile::tempdir().unwrap();
    let registry = data.path().join(REGISTRY_FILE);
    let connection = Connection::open(&registry).unwrap();
    connection.execute_batch("CREATE TABLE repositories (
        id INTEGER PRIMARY KEY, root_path TEXT NOT NULL UNIQUE, enabled_at INTEGER NOT NULL,
        accessibility TEXT NOT NULL, config_blob_oid TEXT, refresh_required INTEGER NOT NULL);
        INSERT INTO repositories VALUES (7, '/fixture', 123, 'accessible', NULL, 0);
        CREATE TABLE operation_records (
          id INTEGER PRIMARY KEY, repository_id INTEGER, root_path TEXT NOT NULL,
          operation_ulid TEXT, action TEXT NOT NULL, target TEXT, item_id TEXT,
          context_path TEXT, state TEXT NOT NULL, completed_step TEXT, observed_at INTEGER NOT NULL,
          persisted_context_count INTEGER NOT NULL DEFAULT 0, redacted_error TEXT);
        INSERT INTO operation_records VALUES (13,7,'/fixture',NULL,'refresh',NULL,NULL,NULL,'completed',NULL,123,0,NULL);").unwrap();
    drop(connection);
    for _ in 0..3 {
        RepositoryService::open_at(data.path()).unwrap();
    }
    let connection = Connection::open(&registry).unwrap();
    assert_eq!(
        connection
            .query_row("SELECT root_path FROM repositories WHERE id=7", [], |r| {
                r.get::<_, String>(0)
            })
            .unwrap(),
        "/fixture"
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT action FROM operation_records WHERE id=13",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
        "refresh"
    );
    let tables: i64 = connection.query_row("SELECT count(*) FROM sqlite_master WHERE type='table' AND name IN ('remote_polling_state','remote_observation_batches','remote_ref_observations','remote_context_states','remote_operation_records')", [], |r| r.get(0)).unwrap();
    assert_eq!(tables, 5, "dedicated remote schema missing");
    assert_eq!(connection.query_row("SELECT enabled,paused,interval_seconds,recovery_suspended FROM remote_polling_state WHERE repository_id=7", [], |r| Ok((r.get::<_, i64>(0)?,r.get::<_, i64>(1)?,r.get::<_, i64>(2)?,r.get::<_, i64>(3)?))).unwrap(), (1,0,300,0));
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM remote_polling_state", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
}

fn cache_replacement_publishes_remote_history_marker() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    fs::write(data.path().join(REGISTRY_FILE), b"corrupt registry").unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    service
        .rebuild_repository(RebuildRepositoryRequest {
            root: fixture.root.clone(),
            operation_id: OperationId::new(),
        })
        .unwrap();
    assert_eq!(
        fs::read(data.path().join("remote-history-recovery-required")).unwrap(),
        b"manyhands remote history recovery required v1\n"
    );
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    assert_eq!(
        connection
            .query_row(
                "SELECT paused,recovery_suspended,history_unknown FROM remote_polling_state",
                [],
                |r| Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, i64>(2)?
                ))
            )
            .unwrap(),
        (0, 1, 1)
    );
}

fn remote_marker_failure_aborts_replacement_before_renaming_registry() {
    for directory in [false, true] {
        let fixture = support::born_repository();
        let data = tempfile::tempdir().unwrap();
        let marker = data.path().join("remote-history-recovery-required");
        if directory {
            fs::create_dir(&marker).unwrap();
        } else {
            fs::write(&marker, b"invalid marker").unwrap();
        }
        let registry = data.path().join(REGISTRY_FILE);
        fs::write(&registry, b"corrupt registry").unwrap();
        let service = RepositoryService::open_at(data.path()).unwrap();
        assert!(
            service
                .rebuild_repository(RebuildRepositoryRequest {
                    root: fixture.root.clone(),
                    operation_id: OperationId::new()
                })
                .is_err()
        );
        assert_eq!(fs::read(registry).unwrap(), b"corrupt registry");
        assert!(!fs::read_dir(data.path()).unwrap().any(|entry| {
            entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains(".corrupt-")
        }));
    }
}

fn polling_policy_survives_reopen_and_local_snapshot_stays_intact() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let before = enabled.service.repository_snapshot(&fixture.root).unwrap();
    enabled
        .service
        .set_remote_polling(
            &fixture.root,
            true,
            true,
            PollingInterval::from_seconds(60).unwrap(),
        )
        .unwrap();
    let reopened = RepositoryService::open_at(enabled.data_directory.path()).unwrap();
    let after = reopened.repository_snapshot(&fixture.root).unwrap();
    assert_eq!(before.items, after.items);
    assert_eq!(before.contexts, after.contexts);
    assert_eq!(before.problems, after.problems);
    assert!(after.remote.polling().paused());
    assert_eq!(after.remote.polling().interval().as_secs(), 60);
}

fn corrupt_remote_policy_requires_recovery_without_repair_or_raw_error() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let connection = Connection::open(enabled.data_directory.path().join(REGISTRY_FILE)).unwrap();
    connection.execute_batch("PRAGMA ignore_check_constraints=ON; UPDATE remote_polling_state SET interval_seconds=1,latest_outcome='SECRET_RESPONSE_SENTINEL'").unwrap();
    let error = enabled
        .service
        .repository_snapshot(&fixture.root)
        .unwrap_err();
    assert_eq!(error.kind, RepositoryErrorKind::RecoveryRequired);
    assert!(!format!("{error:?} {error}").contains("SECRET_RESPONSE_SENTINEL"));
    assert!(
        enabled
            .service
            .set_remote_polling(
                &fixture.root,
                true,
                false,
                PollingInterval::from_seconds(300).unwrap()
            )
            .is_err()
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT interval_seconds FROM remote_polling_state",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
}

fn absent_context_remains_history_unknown_after_cache_loss_and_restart() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    fs::write(data.path().join(REGISTRY_FILE), b"corrupt registry").unwrap();
    RepositoryService::open_at(data.path())
        .unwrap()
        .rebuild_repository(RebuildRepositoryRequest {
            root: fixture.root.clone(),
            operation_id: OperationId::new(),
        })
        .unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let remote = service.remote_snapshot(&fixture.root).unwrap();
    let id = "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap();
    assert!(remote.polling().recovery_suspended());
    assert!(!remote.polling().paused());
    assert_eq!(
        remote.publication_evidence_for(AuthoringKind::Ticket, &id),
        RemotePublicationEvidence::HistoryUnknown
    );
}

fn explicit_recovery_resume_preserves_pause_and_unknown_history() {
    for paused in [false, true] {
        let fixture = support::born_repository();
        let data = tempfile::tempdir().unwrap();
        fs::write(data.path().join(REGISTRY_FILE), b"corrupt registry").unwrap();
        let service = RepositoryService::open_at(data.path()).unwrap();
        service
            .rebuild_repository(RebuildRepositoryRequest {
                root: fixture.root.clone(),
                operation_id: OperationId::new(),
            })
            .unwrap();
        service
            .set_remote_polling(
                &fixture.root,
                false,
                paused,
                PollingInterval::from_seconds(120).unwrap(),
            )
            .unwrap();
        service
            .resume_remote_polling_after_recovery(&fixture.root)
            .unwrap();
        let reopened = RepositoryService::open_at(data.path()).unwrap();
        let snapshot = reopened.remote_snapshot(&fixture.root).unwrap();
        assert!(!snapshot.polling().recovery_suspended());
        assert_eq!(snapshot.polling().paused(), paused);
        assert!(!snapshot.polling().enabled());
        assert_eq!(snapshot.polling().interval().as_secs(), 120);
        assert_eq!(
            snapshot.publication_evidence_for(
                AuthoringKind::Ticket,
                &"01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap()
            ),
            RemotePublicationEvidence::HistoryUnknown
        );
        reopened
            .resume_remote_polling_after_recovery(&fixture.root)
            .unwrap();
        assert_eq!(reopened.remote_snapshot(&fixture.root).unwrap(), snapshot);
    }
}

fn reopening_does_not_repair_deleted_policy_or_partial_remote_schema() {
    for corruption in [
        "DELETE FROM remote_polling_state",
        "DROP TABLE remote_context_states",
    ] {
        let fixture = support::born_repository();
        let enabled = support::enabled_repository(&fixture);
        let connection =
            Connection::open(enabled.data_directory.path().join(REGISTRY_FILE)).unwrap();
        connection.execute_batch(corruption).unwrap();
        let error = match RepositoryService::open_at(enabled.data_directory.path()) {
            Ok(_) => panic!("corrupt remote state was silently repaired"),
            Err(error) => error,
        };
        assert_eq!(error.kind, RepositoryErrorKind::RecoveryRequired);
    }
}

struct NoPrompt;
impl SessionCredentialProvider for NoPrompt {
    fn request_passphrase(&mut self, _: &UnlockRequest) -> PassphraseResponse {
        panic!("plain selected fixture key must not prompt");
    }
}

struct ObservationCase {
    remote: SshRemoteFixture,
    local: support::TestRepository,
    enabled: support::EnabledRepository,
}
impl ObservationCase {
    fn preservation(
        &self,
    ) -> (
        support::RepositoryAndWorktreeSnapshot,
        std::collections::BTreeMap<std::path::PathBuf, support::FilesystemEntry>,
    ) {
        (
            support::repository_and_worktree_snapshot(&self.local),
            support::repository_git_file_bytes(&self.local),
        )
    }

    fn new() -> Self {
        let remote = SshRemoteFixture::start().unwrap();
        let local = support::born_repository();
        let enabled = support::enabled_repository(&local);
        local.repository.remote("origin", &remote.url()).unwrap();
        // Commit configuration before creating real local authoring worktrees.
        support::commit_tracked_configuration(
            &local,
            "format_version = 1\nprimary_branch = \"main\"\npublication_remote = \"origin\"\n",
        );
        let case = Self {
            remote,
            local,
            enabled,
        };
        case.register_key();
        fs::write(
            case.local.repository.path().join("FETCH_HEAD"),
            b"prior fetch must survive\n",
        )
        .unwrap();
        case
    }

    fn register_key(&self) {
        let RegisterSharedKeyOutcome::Registered(key) = self
            .enabled
            .service
            .register_shared_key(RegisterSharedKeyRequest {
                label: "observation fixture".into(),
                ownership: SharedKeyOwnership::Imported,
                private_key_path: self.remote.client_key_path().into(),
                public_key_path: None,
            })
            .unwrap()
        else {
            panic!("fixture key registration failed")
        };
        self.enabled.service.select_shared_key(key.id).unwrap();
    }

    fn request(&self) -> ObservePublicationRemoteRequest {
        ObservePublicationRemoteRequest {
            root: self.local.root.clone(),
            operation_id: OperationId::new(),
            invocation: RemotePollInvocation::Explicit,
            restart: false,
            approval: Some(HostApproval {
                authority: SshAuthority {
                    host: "127.0.0.1".into(),
                    port: self.remote.address().port(),
                },
                expected: None,
                presented: self.remote.host_identity(),
            }),
        }
    }

    fn observe(&self) -> repository::RemoteObservationOutcome {
        let result = self
            .enabled
            .service
            .observe_publication_remote(self.request(), &mut SessionCredentials::new(NoPrompt))
            .unwrap();
        assert_eq!(result.category(), RemoteOutcomeCategory::Completed);
        result
    }

    fn local_ticket(&self, id: &str) {
        let result = self
            .enabled
            .service
            .prepare_context(AuthoringTarget {
                root: self.local.root.clone(),
                kind: AuthoringKind::Ticket,
                item_id: id.parse().unwrap(),
                intent: ContextIntent::Create,
                operation_id: OperationId::new(),
            })
            .unwrap();
        let ContextProvisionOutcome::Created(context) = result else {
            panic!("local context missing")
        };
        self.enabled
            .service
            .save_ticket(repository::SaveTicketRequest {
                target: AuthoringTarget {
                    root: self.local.root.clone(),
                    kind: AuthoringKind::Ticket,
                    item_id: id.parse().unwrap(),
                    intent: ContextIntent::Create,
                    operation_id: OperationId::new(),
                },
                draft: repository::TicketDraft {
                    title: "Local observation fixture".into(),
                    ticket_type: "feature".into(),
                    status: "open".into(),
                    project: None,
                    team: None,
                    body: "canonical local content must survive".into(),
                },
                expected_path: repository::ExpectedPathObservation::Missing,
            })
            .unwrap();
        fs::write(
            context.worktree.join("untracked-local-sentinel"),
            b"local context must survive\n",
        )
        .unwrap();
    }
}

fn publication_remote_lifecycle_invalidates_observations() {
    let case = ObservationCase::new();
    case.observe();
    case.enabled
        .service
        .set_publication_remote(repository::SetPublicationRemoteRequest {
            root: case.local.root.clone(),
            name: None,
            operation_id: OperationId::new(),
        })
        .unwrap();
    let snapshot = case
        .enabled
        .service
        .remote_snapshot(&case.local.root)
        .unwrap();
    assert!(snapshot.observations().is_empty());
}

fn same_name_endpoint_replacement_after_reopen_is_not_remote_deletion() {
    let mut case = ObservationCase::new();
    case.remote
        .set_observation_ref(ObservationRef::LocalTicket, true)
        .unwrap();
    case.observe();
    // An independent endpoint advertises no context from the former repository.
    let replacement = SshRemoteFixture::start().unwrap();
    replacement.allow_client_public_key(
        russh::keys::PublicKey::from_bytes(&case.remote.allowed_client_public_key()).unwrap(),
    );
    case.local
        .repository
        .remote_set_url("origin", &replacement.url())
        .unwrap();
    let _original = std::mem::replace(&mut case.remote, replacement);
    case.enabled.service = RepositoryService::open_at(case.enabled.data_directory.path()).unwrap();
    // Keep the same registered key to isolate endpoint identity.
    let after = case.observe();
    assert!(after.snapshot().contexts().is_empty());
    assert_eq!(
        after.snapshot().publication_evidence_for(
            AuthoringKind::Ticket,
            &"01ARZ3NDEKTSV4RRFFQ69G5FAW".parse().unwrap()
        ),
        RemotePublicationEvidence::HistoryUnknown
    );
}

// Catches incomplete advertisement capture, inferred deletion on transport failure,
// accidental materialization, and first-publication inference after observed publication.
fn authenticated_exceptional_states_preserve_local_contexts() {
    let case = ObservationCase::new();
    case.local_ticket("01ARZ3NDEKTSV4RRFFQ69G5FAW");
    case.local_ticket("01ARZ3NDEKTSV4RRFFQ69G5FAY");
    for reference in [
        ObservationRef::RemoteTicket,
        ObservationRef::LocalTicket,
        ObservationRef::RemoteDocument,
        ObservationRef::MalformedTicket,
    ] {
        case.remote.set_observation_ref(reference, true).unwrap();
    }
    let before = case.preservation();
    let local_before = case
        .enabled
        .service
        .repository_snapshot(&case.local.root)
        .unwrap();
    assert_eq!(
        local_before.items.len(),
        2,
        "real canonical local contexts required"
    );
    let first = case.observe();
    assert_eq!(
        first
            .snapshot()
            .contexts()
            .iter()
            .find(|c| c.remote_ref() == Some(ObservationRef::LocalTicket.name()))
            .unwrap()
            .state(),
        RemoteContextState::Observed,
    );
    assert_eq!(
        first.snapshot().observations().len(),
        5,
        "complete primary and family advertisement required"
    );
    for reference in [ObservationRef::RemoteTicket, ObservationRef::RemoteDocument] {
        let context = first
            .snapshot()
            .contexts()
            .iter()
            .find(|c| c.remote_ref() == Some(reference.name()))
            .unwrap();
        assert_eq!(context.state(), RemoteContextState::Unmaterialized);
        assert_eq!(context.advertised_oid(), Some(case.remote.commit_id()));
        assert_eq!(context.tracking_oid(), None);
    }
    let malformed = first
        .snapshot()
        .observations()
        .iter()
        .find(|o| o.classification() == &RemoteRefClassification::MalformedContext)
        .unwrap();
    assert!(malformed.remote_ref().is_none());
    assert!(
        first
            .snapshot()
            .contexts()
            .iter()
            .any(|c| c.state() == RemoteContextState::Malformed)
    );
    let published = "01ARZ3NDEKTSV4RRFFQ69G5FAW".parse().unwrap();
    let unpublished = "01ARZ3NDEKTSV4RRFFQ69G5FAY".parse().unwrap();
    assert_eq!(
        first
            .snapshot()
            .publication_evidence_for(AuthoringKind::Ticket, &published),
        RemotePublicationEvidence::ObservedPublished
    );
    assert_eq!(
        first
            .snapshot()
            .publication_evidence_for(AuthoringKind::Ticket, &unpublished),
        RemotePublicationEvidence::NeverPublished
    );
    case.remote
        .set_observation_ref(ObservationRef::LocalTicket, false)
        .unwrap();
    case.remote
        .disconnect_at(ssh_remote::FixtureBoundary::Advertisement);
    let error = case
        .enabled
        .service
        .observe_publication_remote(case.request(), &mut SessionCredentials::new(NoPrompt))
        .unwrap_err();
    assert!(matches!(
        error,
        repository::RemoteObservationError::Transport(_)
    ));
    let failed = case
        .enabled
        .service
        .remote_snapshot(&case.local.root)
        .unwrap();
    assert_eq!(failed.observations(), first.snapshot().observations());
    assert_eq!(failed.contexts(), first.snapshot().contexts());
    case.remote.pace_transfer(std::time::Duration::ZERO);
    let second = case.observe();
    assert_eq!(second.snapshot().observations().len(), 4);
    let deleted = second
        .snapshot()
        .contexts()
        .iter()
        .find(|c| c.item_id() == Some(&published))
        .unwrap();
    assert_eq!(deleted.state(), RemoteContextState::RemotelyDeleted);
    assert_eq!(
        deleted.publication_evidence(),
        RemotePublicationEvidence::ObservedPublished
    );
    assert_eq!(deleted.advertised_oid(), Some(case.remote.commit_id()));
    assert_eq!(
        second
            .snapshot()
            .publication_evidence_for(AuthoringKind::Ticket, &unpublished),
        RemotePublicationEvidence::NeverPublished
    );
    assert_eq!(
        case.remote.accepted_keys(),
        vec![case.remote.allowed_client_public_key(); 3]
    );
    assert!(
        before == case.preservation(),
        "observation mutated local Git or worktrees"
    );
    let local_after = case
        .enabled
        .service
        .repository_snapshot(&case.local.root)
        .unwrap();
    assert_eq!(local_before.contexts, local_after.contexts);
    assert_eq!(local_before.items, local_after.items);
    assert_eq!(local_before.problems, local_after.problems);
    let probes = vec![
        b"private-response-marker".to_vec(),
        fs::read(case.remote.client_key_path()).unwrap(),
    ];
    ssh_privacy::clean(
        format!("{first:?} {second:?} {error:?} {error}").as_bytes(),
        &probes,
    )
    .unwrap();
    ssh_privacy::scan(case.enabled.data_directory.path(), &probes).unwrap();
}

// Catches loss of the cache-loss marker across restart or a successful absent batch.
fn authenticated_absence_after_cache_loss_stays_history_unknown() {
    let mut case = ObservationCase::new();
    let id = "01ARZ3NDEKTSV4RRFFQ69G5FAW".parse().unwrap();
    case.local_ticket("01ARZ3NDEKTSV4RRFFQ69G5FAW");
    assert_eq!(
        case.observe()
            .snapshot()
            .publication_evidence_for(AuthoringKind::Ticket, &id),
        RemotePublicationEvidence::NeverPublished
    );
    let before = case.preservation();
    fs::write(
        case.enabled.data_directory.path().join(REGISTRY_FILE),
        b"lost remote history",
    )
    .unwrap();
    let service = RepositoryService::open_at(case.enabled.data_directory.path()).unwrap();
    service
        .rebuild_repository(RebuildRepositoryRequest {
            root: case.local.root.clone(),
            operation_id: OperationId::new(),
        })
        .unwrap();
    case.enabled.service = RepositoryService::open_at(case.enabled.data_directory.path()).unwrap();
    case.register_key();
    let after = case.observe();
    assert_eq!(after.snapshot().observations().len(), 1);
    assert_eq!(
        after
            .snapshot()
            .publication_evidence_for(AuthoringKind::Ticket, &id),
        RemotePublicationEvidence::HistoryUnknown
    );
    let local = after
        .snapshot()
        .contexts()
        .iter()
        .find(|c| c.item_id() == Some(&id))
        .unwrap();
    assert_eq!(local.state(), RemoteContextState::HistoryUnknown);
    assert!(after.snapshot().polling().recovery_suspended());
    let reopened = RepositoryService::open_at(case.enabled.data_directory.path()).unwrap();
    assert_eq!(
        reopened.remote_snapshot(&case.local.root).unwrap(),
        *after.snapshot()
    );
    assert!(
        before == case.preservation(),
        "cache recovery or observation mutated local contexts"
    );
}

// Catches premature batch publication or reservation release while an authenticated
// poll is in flight. The competing service uses public APIs, without recorder hooks.
fn authenticated_poll_yields_and_cancels_before_publishing() {
    for cancel in [false, true] {
        let case = ObservationCase::new();
        let prior = case.observe();
        case.remote
            .set_observation_ref(ObservationRef::RemoteTicket, true)
            .unwrap();
        let before = case.preservation();
        let mut poll = case.request();
        poll.invocation = RemotePollInvocation::Automatic;
        let manual = case.request();
        let data = case.enabled.data_directory.path().to_owned();
        let remote = &case.remote;
        let hold = remote.hold_advertisement().unwrap();
        std::thread::scope(|scope| {
            let poll = &poll;
            let controller = scope.spawn(move || {
                hold.wait_until_held().unwrap();
                let service = RepositoryService::open_at(&data).unwrap();
                assert_eq!(
                    service
                        .active_remote_operation(&poll.root)
                        .unwrap()
                        .unwrap()
                        .operation_id(),
                    poll.operation_id
                );
                if cancel {
                    service
                        .cancel_remote_operation(&poll.root, poll.operation_id)
                        .unwrap();
                } else {
                    assert!(matches!(
                        service.observe_publication_remote(
                            manual,
                            &mut SessionCredentials::new(NoPrompt)
                        ),
                        Err(repository::RemoteObservationError::PollYielding)
                    ));
                }
                let active = service
                    .active_remote_operation(&poll.root)
                    .unwrap()
                    .unwrap();
                assert_eq!(active.operation_id(), poll.operation_id);
                assert_eq!(active.cancel_requested(), cancel);
                assert_eq!(active.yield_requested(), !cancel);
                assert_eq!(
                    service
                        .remote_snapshot(&poll.root)
                        .unwrap()
                        .observations()
                        .len(),
                    1
                );
                hold.release().unwrap();
            });
            let result = case
                .enabled
                .service
                .observe_publication_remote(poll.clone(), &mut SessionCredentials::new(NoPrompt));
            controller.join().unwrap();
            if cancel {
                assert_eq!(result.unwrap().category(), RemoteOutcomeCategory::Cancelled);
            } else {
                assert!(matches!(
                    result,
                    Err(repository::RemoteObservationError::Interrupted)
                ));
            }
        });
        assert!(
            case.enabled
                .service
                .active_remote_operation(&case.local.root)
                .unwrap()
                .is_none()
        );
        let interrupted = case
            .enabled
            .service
            .remote_snapshot(&case.local.root)
            .unwrap();
        assert_eq!(interrupted.observations(), prior.snapshot().observations());
        assert_eq!(interrupted.contexts(), prior.snapshot().contexts());
        assert_eq!(
            remote.accepted_keys(),
            vec![remote.allowed_client_public_key(); 2]
        );
        let completed = case.observe();
        assert_eq!(completed.snapshot().observations().len(), 2);
        assert!(
            before == case.preservation(),
            "poll/manual handoff changed Git"
        );
    }
}

// Catches a lost controller leaving a fixture hung or publishing an unapproved batch.
fn abandoned_advertisement_hold_aborts_without_publishing() {
    let case = ObservationCase::new();
    let prior = case.observe();
    case.remote
        .set_observation_ref(ObservationRef::RemoteTicket, true)
        .unwrap();
    let before = case.preservation();
    let hold = case.remote.hold_advertisement().unwrap();
    std::thread::scope(|scope| {
        let controller = scope.spawn(move || {
            hold.wait_until_held().unwrap();
            // Mirrors unwinding the controller before its explicit release.
            drop(hold);
        });
        let result = case
            .enabled
            .service
            .observe_publication_remote(case.request(), &mut SessionCredentials::new(NoPrompt));
        controller.join().unwrap();
        assert!(matches!(
            result,
            Err(repository::RemoteObservationError::Transport(_))
        ));
    });
    let after = case
        .enabled
        .service
        .remote_snapshot(&case.local.root)
        .unwrap();
    assert_eq!(after.observations(), prior.snapshot().observations());
    assert_eq!(after.contexts(), prior.snapshot().contexts());
    assert!(before == case.preservation());
    assert_eq!(
        case.observe().snapshot().observations().len(),
        2,
        "aborted hold must be consumed"
    );
}

// Catches a partial batch, deletion, or leaked raw SQLite error when the second
// ref insert fails after an earlier ref and the old batch were already changed.
fn authenticated_batch_failure_is_atomic_and_restartable() {
    let case = ObservationCase::new();
    case.remote
        .set_observation_ref(ObservationRef::LocalTicket, true)
        .unwrap();
    let first = case.observe();
    case.remote
        .set_observation_ref(ObservationRef::LocalTicket, false)
        .unwrap();
    case.remote
        .set_observation_ref(ObservationRef::RemoteDocument, true)
        .unwrap();
    let before = case.preservation();
    let db = Connection::open(case.enabled.data_directory.path().join(REGISTRY_FILE)).unwrap();
    db.execute_batch("CREATE TRIGGER fail_second_ref BEFORE INSERT ON remote_ref_observations WHEN NEW.ordinal=1 BEGIN SELECT RAISE(ABORT,'private-atomicity-response'); END;").unwrap();
    let mut request = case.request();
    let error = case
        .enabled
        .service
        .observe_publication_remote(request.clone(), &mut SessionCredentials::new(NoPrompt))
        .unwrap_err();
    assert!(matches!(
        error,
        repository::RemoteObservationError::Repository(_)
    ));
    assert!(!format!("{error:?} {error}").contains("private-atomicity-response"));
    let failed = case
        .enabled
        .service
        .remote_snapshot(&case.local.root)
        .unwrap();
    assert_eq!(failed.observations(), first.snapshot().observations());
    assert_eq!(failed.contexts(), first.snapshot().contexts());
    assert_eq!(
        db.query_row("SELECT count(*) FROM remote_observation_batches", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM remote_ref_observations", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        2
    );
    db.execute_batch("DROP TRIGGER fail_second_ref").unwrap();
    request.restart = true;
    let final_batch = case
        .enabled
        .service
        .observe_publication_remote(request, &mut SessionCredentials::new(NoPrompt))
        .unwrap();
    assert_eq!(final_batch.category(), RemoteOutcomeCategory::Completed);
    assert_eq!(final_batch.snapshot().observations().len(), 2);
    assert!(
        final_batch
            .snapshot()
            .contexts()
            .iter()
            .any(
                |c| c.remote_ref() == Some(ObservationRef::LocalTicket.name())
                    && c.state() == RemoteContextState::RemotelyDeleted
            )
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM remote_observation_batches", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert!(
        before == case.preservation(),
        "failed/restarted observation changed Git"
    );
}
