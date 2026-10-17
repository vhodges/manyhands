use crate::{
    repository::{OperationId, keys::*},
    session::*,
    ssh_remote::*,
};

pub const CASES: &[crate::ssh_harness::Case] = &[
    ("generated_plain", generated_plain),
    ("generated_encrypted", generated_encrypted),
    ("external_rsa_pem", external_rsa_pem),
];
fn generated_plain() -> Result<(), FixtureError> {
    generated(false)
}
fn generated_encrypted() -> Result<(), FixtureError> {
    generated(true)
}
fn generated(encrypted: bool) -> Result<(), FixtureError> {
    let mut case = Case::new(false)?;
    let home = case.directory.path().join("key home");
    fixed(std::fs::create_dir(&home))?;
    let store = fixed(KeyStore::for_home(&home))?;
    let protection = if encrypted {
        KeyProtection::Passphrase(fixed(SecretPassphrase::new(PASSWORD.into()))?)
    } else {
        KeyProtection::Unencrypted
    };
    let GenerateSharedKeyOutcome::Created(registration) = fixed(case.service.generate_shared_key(
        &store,
        GenerateSharedKeyRequest {
            operation_id: OperationId::new(),
            label: "generated".into(),
            protection,
        },
    ))?
    else {
        return Err(FixtureError);
    };
    let public = fixed(russh::keys::load_public_key(
        registration.public_key_path.as_ref().ok_or(FixtureError)?,
    ))?;
    case.fixture.allow_client_public_key(public);
    fixed(case.service.select_shared_key(registration.id))?;
    case.registration = registration;
    exercise(&case, encrypted)
}
fn external_rsa_pem() -> Result<(), FixtureError> {
    let case = Case::new(false)?;
    // Only fixture generation invokes OpenSSL. Imported PEM bytes remain in place.
    let mut child = fixed(
        std::process::Command::new("openssl")
            .args(["genrsa", "-traditional", "-out"])
            .arg(case.fixture.client_key_path())
            .arg("2048")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn(),
    )?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                assert!(status.success());
                break;
            }
            Ok(None) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(10))
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(FixtureError);
            }
        }
    }
    assert!(ssh_key::PrivateKey::read_openssh_file(case.fixture.client_key_path()).is_err());
    let key = fixed(russh::keys::load_secret_key(
        case.fixture.client_key_path(),
        None,
    ))?;
    case.fixture
        .allow_client_public_key(key.public_key().clone());
    exercise(&case, false)
}
fn exercise(case: &Case, encrypted: bool) -> Result<(), FixtureError> {
    let before = snapshot(&case.root)?;
    let source = fixed(std::fs::read(&case.registration.private_key_path))?;
    let registrations = fixed(case.service.list_shared_keys())?;
    let (mut session, requests) = session(if encrypted {
        vec![secret(PASSWORD)]
    } else {
        vec![]
    });
    for _ in 0..2 {
        let verified = fixed(case.verify(&mut session))?;
        assert_eq!(verified.selected_key_id, case.registration.id);
        assert_eq!(verified.host_key, case.fixture.host_identity());
    }
    assert_eq!(
        case.fixture.accepted_keys(),
        vec![case.fixture.allowed_client_public_key(); 2]
    );
    assert_eq!(requests.borrow().len(), usize::from(encrypted));
    assert_eq!(before, snapshot(&case.root)?);
    assert!(source == fixed(std::fs::read(&case.registration.private_key_path))?);
    assert_eq!(registrations, fixed(case.service.list_shared_keys())?);
    Ok(())
}
