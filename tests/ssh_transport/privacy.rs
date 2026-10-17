//! Regressions for secret diagnostics and persistence, using actual SSH operations.
use crate::{
    failures::*,
    repository::transport::{operation_tests::*, *},
    session::*,
    ssh_privacy::*,
    ssh_remote::*,
};
use std::{cell::Cell, error::Error};
pub const CASES: &[crate::ssh_harness::Case] = &[("transport_privacy", exercise)];
fn emit(error: &SshTransportError) {
    println!("{error}; {error:?}; {}", error.guidance());
    eprintln!("{error}; {error:?}; {}", error.guidance());
    let mut source = error.source();
    while let Some(error) = source {
        println!("{error}; {error:?}");
        eprintln!("{error}; {error:?}");
        source = error.source();
    }
}
fn exercise() -> Result<(), FixtureError> {
    let case = Case::new(false)?;
    seed(&case)?;
    let tag = crate::repository::SharedKeyId::new().to_string();
    let password = format!("correct-{tag}");
    let wrong = format!("incorrect-{tag}");
    let malformed = format!("malformed-{tag}");
    let callback = format!("callback-only-{tag}");
    // Exactly the native rejection text width permits packet-preserving mutation.
    let hostile = format!("bad{}", &tag[tag.len() - 13..]);
    let plain = fixed(std::fs::read(case.fixture.client_key_path()))?;
    let key = fixed(ssh_key::PrivateKey::from_openssh(&plain))?;
    let private = key
        .key_data()
        .ed25519()
        .ok_or(FixtureError)?
        .private
        .to_bytes();
    let encrypted = fixed(key.encrypt(&mut ssh_key::rand_core::OsRng, &password))?;
    let encoded = fixed(encrypted.to_openssh(ssh_key::LineEnding::LF))?;
    let mut probes = vec![
        password.as_bytes().to_vec(),
        wrong.as_bytes().to_vec(),
        malformed.as_bytes().to_vec(),
        callback.as_bytes().to_vec(),
        hostile.as_bytes().to_vec(),
        private[..16].to_vec(),
        private[16..].to_vec(),
    ];
    for bytes in [&plain[..], encoded.as_bytes()] {
        probes.push(bytes.to_vec());
        probes.extend(
            bytes
                .split(|b| *b == b'\n')
                .filter(|line| !line.is_empty() && !line.starts_with(b"-----"))
                .map(<[u8]>::to_vec),
        );
    }
    save(&probes)?;
    fixed(std::fs::write(
        case.fixture.client_key_path(),
        encoded.as_bytes(),
    ))?;
    let data = case.directory.path().join("data");
    let database = data.join("manyhands.sqlite3");
    let reader = fixed(rusqlite::Connection::open(&database))?;
    fixed(reader.execute_batch("BEGIN; SELECT count(*) FROM shared_ssh_keys;"))?;
    let before = Preservation::capture(&case)?;
    let (mut credentials, requests) = session(vec![secret(&wrong), secret(&password)]);
    emit(&case.verify(&mut credentials).unwrap_err());
    before.check(&case)?;
    let verified = fixed(case.verify(&mut credentials))?;
    println!("{verified:?}; {credentials:?}; {:?}", requests.borrow());
    for request in requests.borrow().iter() {
        println!("{}", request.reason.guidance());
    }
    before.check(&case)?;
    let (error, backend) = crate::repository::transport::tests::callback_privacy(
        case.fixture.client_key_path(),
        &callback,
    );
    assert!(
        error.kind == SshTransportErrorKind::EndpointChanged,
        "callback input must fail closed"
    );
    emit(&error);
    println!("{backend}");
    eprintln!("{backend}");
    credentials.clear();
    fixed(std::fs::write(case.fixture.client_key_path(), &malformed))?;
    let (mut malformed_session, _) = session(vec![secret(&wrong)]);
    emit(&case.verify(&mut malformed_session).unwrap_err());
    fixed(std::fs::write(case.fixture.client_key_path(), &plain))?;
    hostile_push(&case, &hostile, &probes)?;
    populated_storage(&case, &reader, &probes)?;
    assert!(
        !scan(&case.root, &probes)?.is_empty(),
        "Git metadata and canonical files must be scanned"
    );
    assert!(
        !scan(&data, &probes)?.is_empty(),
        "application storage must be scanned"
    );
    println!("SSH_OBSERVATION 505 1");
    Ok(())
}
fn hostile_push(case: &Case, hostile: &str, probes: &[Vec<u8>]) -> Result<(), FixtureError> {
    crate::transfer::commit(case)?;
    let server = fixed(git2::Repository::open_bare(case.fixture.repository_path()))?;
    fixed(fixed(server.config())?.set_bool("receive.denyNonFastForwards", true))?;
    case.fixture.hostile_rejection(hostile);
    // First prove the hostile text actually traverses SSH into native callbacks.
    let repo = fixed(git2::Repository::open(&case.root))?;
    let mut raw = fixed(repo.remote_anonymous(&case.fixture.url()))?;
    let observed = Cell::new(false);
    let sideband = Cell::new(false);
    let mut callbacks = git2::RemoteCallbacks::new();
    callbacks.certificate_check(|_, _| Ok(git2::CertificateCheckStatus::CertificateOk));
    callbacks.credentials(|_, _, _| {
        git2::Cred::ssh_key("fixture", None, case.fixture.client_key_path(), None)
    });
    callbacks.push_update_reference(|_, status| {
        if status.is_some_and(|text| text.contains(hostile)) {
            observed.set(true);
        }
        Ok(())
    });
    callbacks.sideband_progress(|bytes| {
        if clean(bytes, probes).is_err() {
            sideband.set(true);
        }
        true
    });
    let mut options = git2::PushOptions::new();
    options.remote_callbacks(callbacks);
    fixed(raw.push(&["+refs/heads/pushed:refs/heads/main"], Some(&mut options)))?;
    assert!(
        observed.get(),
        "hostile rejection must reach native status callback"
    );
    assert!(
        sideband.get(),
        "hostile sideband must reach native progress callback"
    );
    let (mut credentials, _) = session(vec![]);
    let mut request = case.request();
    request.direction = SshDirection::Push;
    let error = transfer(
        &case.service,
        request,
        &mut credentials,
        Transfer::RejectedPush,
        &Cell::new(0),
        || {},
    )
    .unwrap_err();
    assert!(
        error.kind == SshTransportErrorKind::PushRejected,
        "hostile rejection must remain typed"
    );
    emit(&error);
    assert!(
        fixed(server.refname_to_id("refs/heads/main"))? == case.fixture.commit_id(),
        "rejected server ref must remain unchanged"
    );
    Ok(())
}
fn populated_storage(
    case: &Case,
    reader: &rusqlite::Connection,
    probes: &[Vec<u8>],
) -> Result<(), FixtureError> {
    let data = case.directory.path().join("data");
    let database = data.join("manyhands.sqlite3");
    let wal = data.join("manyhands.sqlite3-wal");
    assert!(
        fixed(std::fs::metadata(&wal))?.len() > 32,
        "live WAL must contain frames"
    );
    let live = scan(&data, probes)?;
    assert!(
        live.iter().any(|(p, n)| p == &wal && *n > 32),
        "live WAL must be scanned"
    );
    let backup = data.join("recovery/backups");
    fixed(std::fs::create_dir_all(&backup))?;
    let snapshot = backup.join("registry.corrupt-snapshot");
    let wal_backup = backup.join("registry-wal.backup");
    fixed(std::fs::copy(&database, &snapshot))?;
    fixed(std::fs::copy(&wal, &wal_backup))?;
    fixed(reader.execute_batch("ROLLBACK"))?;
    let writer = fixed(rusqlite::Connection::open(&database))?;
    fixed(writer.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)"))?;
    let rollback_path = backup.join("rollback.sqlite3");
    fixed(std::fs::copy(&database, &rollback_path))?;
    let rollback = fixed(rusqlite::Connection::open(&rollback_path))?;
    fixed(rollback.execute_batch("PRAGMA journal_mode=DELETE; BEGIN IMMEDIATE; UPDATE shared_ssh_keys SET label='rollback evidence';"))?;
    let journal = backup.join("rollback.sqlite3-journal");
    assert!(
        fixed(std::fs::metadata(&journal))?.len() > 0,
        "live journal must be populated"
    );
    let scanned = scan(&data, probes)?;
    for path in [&database, &wal, &snapshot, &wal_backup, &journal] {
        assert!(
            scanned.iter().any(|(p, _)| p == path),
            "required storage artifact was not scanned"
        );
    }
    assert!(
        scanned.iter().map(|(_, n)| n).sum::<usize>() > 8192,
        "populated storage must be scanned"
    );
    fixed(rollback.execute_batch("ROLLBACK"))?;
    Ok(())
}
