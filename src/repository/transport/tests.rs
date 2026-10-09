use super::callbacks::CallbackAttempt;
use super::trust::{HostTrustSnapshot, publish_reapproval_marker, read_reapproval_marker};
use super::*;
use crate::repository::{REGISTRY_FILE, RepositoryService};

fn prepared(path: &std::path::Path) -> PreparedSshAttempt {
    use crate::repository::keys::*;
    PreparedSshAttempt {
        registration: SharedKeyRegistration {
            id: SharedKeyId::new(),
            label: "selected".into(),
            ownership: SharedKeyOwnership::Imported,
            private_key_path: path.into(),
            public_key_path: None,
            public_key_fingerprint: None,
            private_source_state: PrivateKeySourceState::Available,
            public_metadata_state: PublicKeyMetadataState::NotProvided,
            selected: true,
        },
        source: KeySourceToken::observe(path).unwrap(),
        endpoint: super::endpoint::parse_ssh_endpoint(
            "ssh://fixture@example.invalid/repository.git",
        )
        .unwrap(),
        trust: HostTrustSnapshot {
            pin: None,
            reapproval_required: false,
        },
        approval: None,
    }
}

#[allow(dead_code)]
pub(crate) struct HandshakeProbe {
    pub(crate) connected: bool,
    pub(crate) backend_code: Option<git2::ErrorCode>,
    pub(crate) observed: Option<HostKeyIdentity>,
    pub(crate) failure: Option<SshTransportErrorKind>,
    pub(crate) passthrough: bool,
    pub(crate) key_submissions: u8,
    pub(crate) pin: Option<HostKeyIdentity>,
}
/// Only called by the custom, pre-thread initialized SSH test executable.
#[allow(dead_code)]
pub(crate) fn callback_handshake(
    url: &str,
    key: &std::path::Path,
    pin: Option<HostKeyIdentity>,
    approval: Option<HostApproval>,
    recovered: bool,
) -> HandshakeProbe {
    assert!(crate::runtime::git_transport_initialized());
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let mut prepared = prepared(key);
    prepared.endpoint = super::endpoint::parse_ssh_endpoint(url).unwrap();
    let authority = &prepared.endpoint.authority;
    if let Some(pin) = pin {
        service
            .finalize_host_pin(
                authority,
                None,
                &pin,
                Some(&self::approval(authority, None, &pin)),
            )
            .unwrap();
    }
    if recovered {
        std::fs::write(data.path().join(REGISTRY_FILE), b"corrupt registry").unwrap();
        service
            .reconcile_corrupt_cache_replacement(
                crate::repository::RepositoryOperation::RebuildRepository,
                data.path(),
            )
            .unwrap();
        assert!(read_reapproval_marker(data.path()).unwrap());
        assert_eq!(service.read_host_pin(authority).unwrap(), None);
    }
    prepared.trust = service.read_host_trust(authority).unwrap();
    prepared.approval = approval;
    let target = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(target.path()).unwrap();
    let mut remote = repo.remote_anonymous(url).unwrap();
    let attempt = CallbackAttempt::default();
    let result = remote.connect_auth(
        git2::Direction::Fetch,
        Some(super::callbacks::build_callbacks(&prepared, None, &attempt)),
        None,
    );
    let (connected, backend_code) = match result {
        Ok(connection) => {
            let observed = attempt.observed_host().expect("host callback must run");
            service
                .finalize_host_trust(
                    authority,
                    &prepared.trust,
                    &observed,
                    prepared.approval.as_ref(),
                )
                .unwrap();
            assert!(!connection.list().unwrap().is_empty());
            (true, None)
        }
        Err(error) => (false, Some(error.code())),
    };
    HandshakeProbe {
        connected,
        backend_code,
        observed: attempt.observed_host(),
        failure: attempt.failure(),
        passthrough: attempt.passthrough(),
        key_submissions: attempt.key_submissions(),
        pin: service.read_host_pin(authority).unwrap(),
    }
}

#[test]
fn transport_callback_host_decision_table() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut prepared = prepared(file.path());
    let presented = identity(1);
    let old = identity(2);
    let host = prepared.endpoint.authority.clone();
    let attempt = CallbackAttempt::default();
    assert!(matches!(
        attempt.check_host(&prepared, "example.invalid", Some(presented.clone())),
        Ok(git2::CertificateCheckStatus::CertificatePassthrough)
    ));
    assert_eq!(attempt.observed_host(), Some(presented.clone()));
    assert!(attempt.passthrough());
    prepared.trust.reapproval_required = true;
    let attempt = CallbackAttempt::default();
    assert!(
        attempt
            .check_host(&prepared, "example.invalid", Some(presented.clone()))
            .is_err()
    );
    assert_eq!(
        attempt.failure(),
        Some(SshTransportErrorKind::HostApprovalRequired {
            presented: presented.clone()
        })
    );
    prepared.approval = Some(approval(&host, None, &presented));
    assert!(matches!(
        CallbackAttempt::default().check_host(
            &prepared,
            "example.invalid",
            Some(presented.clone())
        ),
        Ok(git2::CertificateCheckStatus::CertificateOk)
    ));
    prepared.trust.pin = Some(old.clone());
    let attempt = CallbackAttempt::default();
    assert!(
        attempt
            .check_host(&prepared, "example.invalid", Some(presented.clone()))
            .is_err()
    );
    assert_eq!(
        attempt.failure(),
        Some(SshTransportErrorKind::HostReplacementRequired {
            expected: old.clone(),
            presented: presented.clone()
        })
    );
    prepared.approval = Some(approval(&host, Some(old), &presented));
    assert!(matches!(
        CallbackAttempt::default().check_host(
            &prepared,
            "example.invalid",
            Some(presented.clone())
        ),
        Ok(git2::CertificateCheckStatus::CertificateOk)
    ));
    prepared.approval = None;
    prepared.trust.pin = Some(presented.clone());
    assert!(matches!(
        CallbackAttempt::default().check_host(&prepared, "EXAMPLE.INVALID", Some(presented)),
        Ok(git2::CertificateCheckStatus::CertificateOk)
    ));
}

#[test]
fn transport_callback_refuses_absent_identity_and_wrong_authority() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let prepared = prepared(file.path());
    for (host, identity, expected) in [
        (
            "example.invalid",
            None,
            SshTransportErrorKind::HostVerificationUnavailable,
        ),
        (
            "other.invalid",
            Some(identity(1)),
            SshTransportErrorKind::EndpointChanged,
        ),
        (
            "example.invalid",
            Some(HostKeyIdentity {
                algorithm: "ssh-ed25519".into(),
                sha256: "SHA1:bad".into(),
            }),
            SshTransportErrorKind::HostVerificationUnavailable,
        ),
    ] {
        let attempt = CallbackAttempt::default();
        assert_eq!(
            attempt
                .check_host(&prepared, host, identity)
                .err()
                .unwrap()
                .message(),
            "SSH policy rejected"
        );
        assert_eq!(attempt.failure(), Some(expected));
    }
}

#[test]
fn transport_credentials_are_selected_bounded_and_no_fallback() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let prepared = prepared(file.path());
    let attempt = CallbackAttempt::default();
    let url = &prepared.endpoint.connection_url;
    // libgit2's C enum is signed on MSVC and unsigned elsewhere. Widen both
    // representations losslessly so the full credential type remains checked.
    assert_eq!(
        i64::from(
            attempt
                .credentials(&prepared, None, url, None, git2::CredentialType::USERNAME)
                .unwrap()
                .credtype()
        ),
        i64::from(git2::CredentialType::USERNAME.bits())
    );
    assert_eq!(
        i64::from(
            attempt
                .credentials(
                    &prepared,
                    None,
                    url,
                    Some("fixture"),
                    git2::CredentialType::SSH_KEY
                )
                .unwrap()
                .credtype()
        ),
        i64::from(git2::CredentialType::SSH_KEY.bits())
    );
    assert_eq!(attempt.key_submissions(), 1);
    assert!(
        attempt
            .credentials(
                &prepared,
                None,
                url,
                Some("fixture"),
                git2::CredentialType::SSH_KEY
            )
            .is_err()
    );
    assert_eq!(attempt.failure(), Some(SshTransportErrorKind::KeyRejected));
    let attempt = CallbackAttempt::default();
    attempt
        .credentials(&prepared, None, url, None, git2::CredentialType::USERNAME)
        .unwrap();
    assert!(
        attempt
            .credentials(&prepared, None, url, None, git2::CredentialType::USERNAME)
            .is_err()
    );
    for kind in [
        git2::CredentialType::DEFAULT,
        git2::CredentialType::USER_PASS_PLAINTEXT,
        git2::CredentialType::SSH_MEMORY,
        git2::CredentialType::SSH_INTERACTIVE,
        git2::CredentialType::SSH_CUSTOM,
    ] {
        let attempt = CallbackAttempt::default();
        assert_eq!(
            attempt
                .credentials(&prepared, None, url, Some("fixture"), kind)
                .err()
                .unwrap()
                .message(),
            "SSH policy rejected"
        );
        assert_eq!(attempt.failure(), Some(SshTransportErrorKind::KeyRejected));
        assert_eq!(attempt.key_submissions(), 0);
    }
    for (url, username) in [
        (
            "ssh://fixture@other.invalid/repository.git",
            Some("fixture"),
        ),
        (url.as_str(), Some("other")),
    ] {
        let attempt = CallbackAttempt::default();
        assert!(
            attempt
                .credentials(
                    &prepared,
                    None,
                    url,
                    username,
                    git2::CredentialType::SSH_KEY
                )
                .is_err()
        );
        assert_eq!(
            attempt.failure(),
            Some(SshTransportErrorKind::EndpointChanged)
        );
        assert_eq!(attempt.key_submissions(), 0);
    }
}

fn authority(port: u16) -> SshAuthority {
    SshAuthority {
        host: "example.invalid".into(),
        port,
    }
}
fn identity(byte: u8) -> HostKeyIdentity {
    let public = ssh_key::private::Ed25519Keypair::from_seed(&[byte; 32]);
    let public = ssh_key::PublicKey::from(public.public);
    HostKeyIdentity {
        algorithm: public.algorithm().to_string(),
        sha256: public.fingerprint(ssh_key::HashAlg::Sha256).to_string(),
    }
}
fn approval(
    authority: &SshAuthority,
    expected: Option<HostKeyIdentity>,
    presented: &HostKeyIdentity,
) -> HostApproval {
    HostApproval {
        authority: authority.clone(),
        expected,
        presented: presented.clone(),
    }
}

#[test]
fn transport_pin_migration_preserves_rows_and_separates_ports() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let host = authority(22);
    let key = identity(1);
    service
        .finalize_host_pin(&host, None, &key, Some(&approval(&host, None, &key)))
        .unwrap();
    drop(service);
    let service = RepositoryService::open_at(data.path()).unwrap();
    assert_eq!(service.read_host_pin(&host).unwrap(), Some(key));
    assert_eq!(service.read_host_pin(&authority(2222)).unwrap(), None);
}

#[test]
fn transport_pin_exact_approval_cas_and_idempotence() {
    let data = tempfile::tempdir().unwrap();
    let first = RepositoryService::open_at(data.path()).unwrap();
    let second = RepositoryService::open_at(data.path()).unwrap();
    let host = authority(22);
    let old = identity(1);
    let new = identity(2);
    let approve = approval(&host, None, &old);
    assert_eq!(
        first.finalize_host_pin(
            &host,
            None,
            &old,
            Some(&approval(&authority(23), None, &old))
        ),
        Err(SshTransportErrorKind::HostTrustChanged)
    );
    first
        .finalize_host_pin(&host, None, &old, Some(&approve))
        .unwrap();
    second
        .finalize_host_pin(&host, None, &old, Some(&approve))
        .unwrap();
    let installed = second.read_host_trust(&host).unwrap();
    second
        .finalize_host_trust(&host, &installed, &old, Some(&approve))
        .unwrap();
    assert_eq!(
        second.finalize_host_pin(&host, None, &new, Some(&approval(&host, None, &new))),
        Err(SshTransportErrorKind::HostTrustChanged)
    );
    assert_eq!(
        second.finalize_host_pin(&host, Some(&old), &new, None),
        Err(SshTransportErrorKind::HostTrustChanged)
    );
    second
        .finalize_host_pin(
            &host,
            Some(&old),
            &new,
            Some(&approval(&host, Some(old.clone()), &new)),
        )
        .unwrap();
    assert_eq!(
        first.finalize_host_pin(&host, Some(&old), &old, None),
        Err(SshTransportErrorKind::HostTrustChanged)
    );
    assert_eq!(first.read_host_pin(&host).unwrap(), Some(new));
}

#[test]
fn transport_unapproved_inherited_trust_does_not_create_pin() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    service
        .finalize_host_pin(&authority(22), None, &identity(1), None)
        .unwrap();
    assert_eq!(service.read_host_pin(&authority(22)).unwrap(), None);
}

#[test]
fn transport_malformed_pin_and_unavailable_registry_fail_closed() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let db = rusqlite::Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    db.execute(
        "INSERT INTO ssh_host_pins VALUES ('example.invalid', 22, 'ssh-ed25519', 'SHA256:bad')",
        [],
    )
    .unwrap();
    assert_eq!(
        service.read_host_pin(&authority(22)),
        Err(SshTransportErrorKind::RegistryUnavailable)
    );
    db.execute("DROP TABLE ssh_host_pins", []).unwrap();
    assert_eq!(
        service.read_host_pin(&authority(22)),
        Err(SshTransportErrorKind::RegistryUnavailable)
    );
}

#[test]
fn transport_recovery_marker_persists_and_invalid_entries_fail_closed() {
    let data = tempfile::tempdir().unwrap();
    assert!(!read_reapproval_marker(data.path()).unwrap());
    publish_reapproval_marker(data.path()).unwrap();
    let path = data.path().join("ssh-host-trust-reapproval-required");
    let bytes = std::fs::read(&path).unwrap();
    publish_reapproval_marker(data.path()).unwrap();
    assert!(read_reapproval_marker(data.path()).unwrap());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    std::fs::write(&path, b"malformed").unwrap();
    assert!(read_reapproval_marker(data.path()).is_err());
    assert!(publish_reapproval_marker(data.path()).is_err());
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    assert!(read_reapproval_marker(data.path()).is_err());
    assert!(publish_reapproval_marker(data.path()).is_err());
}

#[test]
fn transport_concurrent_recovery_invalidates_snapshot() {
    let data = tempfile::tempdir().unwrap();
    let first = RepositoryService::open_at(data.path()).unwrap();
    let host = authority(22);
    let snapshot = first.read_host_trust(&host).unwrap();
    assert_eq!(
        snapshot,
        HostTrustSnapshot {
            pin: None,
            reapproval_required: false
        }
    );
    publish_reapproval_marker(data.path()).unwrap();
    assert_eq!(
        first.finalize_host_trust(&host, &snapshot, &identity(1), None),
        Err(SshTransportErrorKind::HostTrustChanged)
    );
    let current = first.read_host_trust(&host).unwrap();
    assert_eq!(
        first.finalize_host_trust(&host, &current, &identity(1), None),
        Err(SshTransportErrorKind::HostTrustChanged)
    );
    first
        .finalize_host_trust(
            &host,
            &current,
            &identity(1),
            Some(&approval(&host, None, &identity(1))),
        )
        .unwrap();
    assert!(read_reapproval_marker(data.path()).unwrap());
}

#[test]
fn transport_two_services_racing_approvals_preserve_winner() {
    let data = tempfile::tempdir().unwrap();
    let first = RepositoryService::open_at(data.path()).unwrap();
    let second = RepositoryService::open_at(data.path()).unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let handles: Vec<_> = [first, second]
        .into_iter()
        .enumerate()
        .map(|(index, service)| {
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let intent = OriginalHostApproval::new(service, index as u8 + 1);
                barrier.wait();
                let result = intent.finalize();
                (intent, result)
            })
        })
        .collect();
    let results: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    reconcile_original_host_approvals(data.path(), results);
}

struct OriginalHostApproval {
    service: RepositoryService,
    host: SshAuthority,
    snapshot: HostTrustSnapshot,
    key: HostKeyIdentity,
    approval: HostApproval,
}
impl OriginalHostApproval {
    fn new(service: RepositoryService, seed: u8) -> Self {
        let host = authority(22);
        let key = identity(seed);
        let snapshot = service.read_host_trust(&host).unwrap();
        assert_eq!(snapshot.pin, None);
        let approval = approval(&host, None, &key);
        Self {
            service,
            host,
            snapshot,
            key,
            approval,
        }
    }

    fn finalize(&self) -> Result<(), SshTransportErrorKind> {
        self.service.finalize_host_trust(
            &self.host,
            &self.snapshot,
            &self.key,
            Some(&self.approval),
        )
    }
}

fn reconcile_original_host_approvals(
    data: &std::path::Path,
    results: Vec<(OriginalHostApproval, Result<(), SshTransportErrorKind>)>,
) {
    assert_eq!(results.len(), 2);
    assert_eq!(
        results.iter().filter(|(_, result)| result.is_ok()).count(),
        1
    );
    let winner = results
        .iter()
        .find(|(_, result)| result.is_ok())
        .unwrap()
        .0
        .key
        .clone();
    for (intent, original) in &results {
        // Live contention can exhaust the bounded cache lease before pin CAS.
        // Only after both requests finish must the unchanged loser see the CAS
        // refusal. No new snapshot, authority, identity, approval, or service.
        assert!(matches!(
            original,
            Ok(())
                | Err(SshTransportErrorKind::HostTrustChanged)
                | Err(SshTransportErrorKind::RegistryUnavailable)
        ));
        assert_eq!(
            intent.finalize(),
            if original.is_ok() {
                Ok(())
            } else {
                Err(SshTransportErrorKind::HostTrustChanged)
            }
        );
        assert_eq!(
            intent.service.read_host_pin(&intent.host).unwrap(),
            Some(winner.clone())
        );
    }
    drop(results);
    let reopened = RepositoryService::open_at(data).unwrap();
    assert_eq!(
        reopened.read_host_pin(&authority(22)).unwrap(),
        Some(winner)
    );
    let db = rusqlite::Connection::open(data.join(REGISTRY_FILE)).unwrap();
    assert_eq!(
        db.query_row("SELECT count(*) FROM ssh_host_pins", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap(),
        1
    );
}

#[test]
fn transport_original_approval_reconciles_after_cache_lease_contention() {
    use crate::repository::{RepositoryOperation, cache_write_guard};
    let data = tempfile::tempdir().unwrap();
    let winner = OriginalHostApproval::new(RepositoryService::open_at(data.path()).unwrap(), 1);
    let loser = OriginalHostApproval::new(RepositoryService::open_at(data.path()).unwrap(), 2);
    let winner_result = winner.finalize();
    assert_eq!(winner_result, Ok(()));
    let guard = cache_write_guard(
        &data.path().join(REGISTRY_FILE),
        data.path(),
        RepositoryOperation::OpenRegistry,
    )
    .unwrap();
    let worker = std::thread::spawn(move || {
        let result = loser.finalize();
        (loser, result)
    });
    let (loser, loser_result) = worker.join().unwrap();
    assert_eq!(
        loser_result,
        Err(SshTransportErrorKind::RegistryUnavailable)
    );
    drop(guard);
    reconcile_original_host_approvals(
        data.path(),
        vec![(winner, winner_result), (loser, loser_result)],
    );
}

#[test]
fn transport_readers_cannot_bypass_recovery_guard_or_partial_publication() {
    use crate::repository::{RepositoryOperation, cache_write_guard};
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let snapshot = service.read_host_trust(&authority(22)).unwrap();
    let guard = cache_write_guard(
        &data.path().join(REGISTRY_FILE),
        data.path(),
        RepositoryOperation::OpenRegistry,
    )
    .unwrap();
    publish_reapproval_marker(data.path()).unwrap();
    let reader = std::thread::spawn(move || service.read_host_trust(&authority(22)));
    assert_eq!(
        reader.join().unwrap(),
        Err(SshTransportErrorKind::RegistryUnavailable)
    );
    // Simulate interruption after marker publication, before database rename.
    drop(guard);
    let service = RepositoryService::open_at(data.path()).unwrap();
    assert_eq!(
        service.finalize_host_trust(&authority(22), &snapshot, &identity(1), None),
        Err(SshTransportErrorKind::HostTrustChanged)
    );
    // Simulate interruption after database rename, before fresh migration.
    std::fs::rename(
        data.path().join(REGISTRY_FILE),
        data.path().join("interrupted-registry"),
    )
    .unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    assert!(
        service
            .read_host_trust(&authority(22))
            .unwrap()
            .reapproval_required
    );
}

#[cfg(unix)]
#[test]
fn transport_marker_symlink_unreadable_and_publication_failure_are_closed() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let data = tempfile::tempdir().unwrap();
    publish_reapproval_marker(data.path()).unwrap();
    let marker = data.path().join("ssh-host-trust-reapproval-required");
    std::fs::set_permissions(&marker, std::fs::Permissions::from_mode(0o000)).unwrap();
    let unreadable = read_reapproval_marker(data.path());
    std::fs::set_permissions(&marker, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert!(unreadable.is_err());
    let actual = data.path().join("actual");
    std::fs::rename(&marker, &actual).unwrap();
    symlink(&actual, &marker).unwrap();
    assert!(read_reapproval_marker(data.path()).is_err());
    assert!(publish_reapproval_marker(data.path()).is_err());
    std::fs::remove_file(&marker).unwrap();
    std::fs::set_permissions(data.path(), std::fs::Permissions::from_mode(0o500)).unwrap();
    let result = publish_reapproval_marker(data.path());
    std::fs::set_permissions(data.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    assert!(result.is_err());
    assert!(!marker.exists());
}

/// Callback-only hostile username never enters repository configuration.
#[allow(dead_code)]
pub(crate) fn callback_privacy(
    key: &std::path::Path,
    username: &str,
) -> (SshTransportError, String) {
    let prepared = prepared(key);
    let attempt = CallbackAttempt::default();
    let backend = attempt
        .credentials(
            &prepared,
            None,
            "ssh://fixture@example.invalid/repository.git",
            Some(username),
            git2::CredentialType::SSH_KEY,
        )
        .err()
        .expect("callback username must reject");
    let error = SshTransportError {
        root: "/safe/root".into(),
        remote_name: "origin".into(),
        direction: SshDirection::Fetch,
        selected_key_id: Some(prepared.registration.id),
        authority: Some(prepared.endpoint.authority),
        kind: attempt.failure().expect("callback failure must be typed"),
    };
    (error, format!("{backend}; {backend:?}"))
}
