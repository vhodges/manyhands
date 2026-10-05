use crate::{
    repository::{RepositoryService, keys::*, transport::*},
    ssh_remote::*,
};
use std::{cell::RefCell, collections::VecDeque, path::PathBuf, rc::Rc};

pub const CASES: &[crate::ssh_harness::Case] = &[
    ("verify_plain", verify_plain),
    ("verify_encrypted", verify_encrypted),
    ("wrong_passphrase_retry", wrong_passphrase_retry),
    ("cancelled_retry", cancelled_retry),
    ("unavailable_retry", unavailable_retry),
    ("wrong_plain_key_prompts", wrong_plain_key_prompts),
    ("cached_rejection", cached_rejection),
    ("advertisement_failure", advertisement_failure),
];

pub struct Provider {
    pub requests: Rc<RefCell<Vec<UnlockRequest>>>,
    responses: VecDeque<PassphraseResponse>,
}
impl SessionCredentialProvider for Provider {
    fn request_passphrase(&mut self, request: &UnlockRequest) -> PassphraseResponse {
        self.requests.borrow_mut().push(request.clone());
        assert_eq!(request.reason, UnlockReason::AuthenticationAmbiguous);
        assert!(request.reason.guidance().contains("passphrase"));
        assert!(
            request
                .reason
                .guidance()
                .contains("server may have rejected")
        );
        self.responses
            .pop_front()
            .expect("one bounded provider response")
    }
}
pub fn session(
    responses: Vec<PassphraseResponse>,
) -> (
    SessionCredentials<Provider>,
    Rc<RefCell<Vec<UnlockRequest>>>,
) {
    let requests = Rc::new(RefCell::new(Vec::new()));
    (
        SessionCredentials::new(Provider {
            requests: requests.clone(),
            responses: responses.into(),
        }),
        requests,
    )
}
pub fn secret(value: &str) -> PassphraseResponse {
    PassphraseResponse::Supplied(SecretPassphrase::new(value.into()).unwrap())
}
pub const PASSWORD: &str = "transport fixture secret";

pub struct Case {
    pub fixture: SshRemoteFixture,
    pub service: RepositoryService,
    pub root: PathBuf,
    pub registration: SharedKeyRegistration,
    pub directory: tempfile::TempDir,
}
impl Case {
    pub fn new(encrypted: bool) -> Result<Self, FixtureError> {
        let fixture = SshRemoteFixture::start()?;
        if encrypted {
            let key = fixed(ssh_key::PrivateKey::read_openssh_file(
                fixture.client_key_path(),
            ))?;
            let key = fixed(key.encrypt(&mut ssh_key::rand_core::OsRng, PASSWORD))?;
            fixed(std::fs::write(
                fixture.client_key_path(),
                fixed(key.to_openssh(ssh_key::LineEnding::LF))?.as_bytes(),
            ))?;
        }
        let directory = fixed(tempfile::tempdir())?;
        let root = directory.path().join("repo");
        let repo = fixed(git2::Repository::init(&root))?;
        fixed(repo.remote("origin", &fixture.url()))?;
        fixed(std::fs::create_dir(root.join(".manyhands")))?;
        fixed(std::fs::write(
            root.join(".manyhands/config.toml"),
            "format_version = 1\nprimary_branch = \"main\"\npublication_remote = \"origin\"\n",
        ))?;
        let service = fixed(RepositoryService::open_at(&directory.path().join("data")))?;
        let RegisterSharedKeyOutcome::Registered(registration) =
            fixed(service.register_shared_key(RegisterSharedKeyRequest {
                label: "selected".into(),
                ownership: SharedKeyOwnership::Imported,
                private_key_path: fixture.client_key_path().into(),
                public_key_path: None,
            }))?
        else {
            return Err(FixtureError);
        };
        fixed(service.select_shared_key(registration.id))?;
        Ok(Self {
            fixture,
            service,
            root,
            registration,
            directory,
        })
    }
    pub fn request(&self) -> VerifySshTransportRequest {
        VerifySshTransportRequest {
            root: self.root.clone(),
            direction: SshDirection::Fetch,
            approval: Some(HostApproval {
                authority: SshAuthority {
                    host: "127.0.0.1".into(),
                    port: self.fixture.address().port(),
                },
                expected: None,
                presented: self.fixture.host_identity(),
            }),
        }
    }
    #[allow(clippy::result_large_err)]
    pub fn verify(
        &self,
        session: &mut SessionCredentials<Provider>,
    ) -> Result<SshTransportVerified, SshTransportError> {
        self.service.verify_ssh_transport(self.request(), session)
    }
}
fn verify_plain() -> Result<(), FixtureError> {
    verify_key(false)
}
fn verify_encrypted() -> Result<(), FixtureError> {
    verify_key(true)
}
fn verify_key(encrypted: bool) -> Result<(), FixtureError> {
    let case = Case::new(encrypted)?;
    let before = snapshot(&case.root)?;
    let private = fixed(std::fs::read(case.fixture.client_key_path()))?;
    let registrations = fixed(case.service.list_shared_keys())?;
    let (mut credentials, requests) = session(if encrypted {
        vec![secret(PASSWORD)]
    } else {
        vec![]
    });
    for _ in 0..2 {
        let verified = fixed(case.verify(&mut credentials))?;
        assert_eq!(verified.selected_key_id, case.registration.id);
        assert_eq!(verified.host_key, case.fixture.host_identity());
    }
    assert_eq!(requests.borrow().len(), usize::from(encrypted));
    assert_eq!(
        case.fixture.accepted_keys(),
        vec![case.fixture.allowed_client_public_key(); 2]
    );
    assert_eq!(before, snapshot(&case.root)?);
    assert!(fixed(std::fs::read(case.fixture.client_key_path()))? == private);
    assert_eq!(registrations, fixed(case.service.list_shared_keys())?);
    Ok(())
}
pub fn snapshot(root: &std::path::Path) -> Result<Vec<(PathBuf, Vec<u8>)>, FixtureError> {
    fn collect(
        root: &std::path::Path,
        path: &std::path::Path,
        files: &mut Vec<(PathBuf, Vec<u8>)>,
    ) -> Result<(), FixtureError> {
        for entry in fixed(std::fs::read_dir(path))? {
            let path = fixed(entry)?.path();
            if path.is_dir() {
                collect(root, &path, files)?;
            } else {
                files.push((
                    fixed(path.strip_prefix(root))?.into(),
                    fixed(std::fs::read(path))?,
                ));
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    collect(root, root, &mut files)?;
    files.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(files)
}
fn wrong_passphrase_retry() -> Result<(), FixtureError> {
    retry(secret("wrong"), SshTransportErrorKind::TransportUnavailable)
}
fn cancelled_retry() -> Result<(), FixtureError> {
    retry(
        PassphraseResponse::Cancelled,
        SshTransportErrorKind::UnlockCancelled,
    )
}
fn unavailable_retry() -> Result<(), FixtureError> {
    retry(
        PassphraseResponse::Unavailable,
        SshTransportErrorKind::ProviderUnavailable,
    )
}
fn retry(first: PassphraseResponse, expected: SshTransportErrorKind) -> Result<(), FixtureError> {
    let case = Case::new(true)?;
    let (mut credentials, requests) = session(vec![first, secret(PASSWORD)]);
    let error = case.verify(&mut credentials).unwrap_err();
    assert_eq!(error.kind, expected);
    assert_eq!(error.selected_key_id, Some(case.registration.id));
    assert_eq!(requests.borrow().len(), 1);
    assert!(!credentials.has_cached_passphrase(&requests.borrow()[0]));
    fixed(case.verify(&mut credentials))?;
    assert_eq!(requests.borrow().len(), 2);
    Ok(())
}
fn wrong_plain_key_prompts() -> Result<(), FixtureError> {
    let case = Case::new(false)?;
    case.fixture.reject_client();
    let (mut credentials, requests) = session(vec![PassphraseResponse::Cancelled]);
    assert_eq!(
        case.verify(&mut credentials).unwrap_err().kind,
        SshTransportErrorKind::UnlockCancelled
    );
    assert_eq!(requests.borrow().len(), 1);
    assert_eq!(case.fixture.helper_invocations(), 0);
    Ok(())
}
fn cached_rejection() -> Result<(), FixtureError> {
    let case = Case::new(true)?;
    let (mut credentials, requests) = session(vec![secret(PASSWORD), secret(PASSWORD)]);
    fixed(case.verify(&mut credentials))?;
    case.fixture.reject_client();
    assert_eq!(
        case.verify(&mut credentials).unwrap_err().kind,
        SshTransportErrorKind::UnlockFailed
    );
    assert_eq!(requests.borrow().len(), 1);
    assert!(!credentials.has_cached_passphrase(&requests.borrow()[0]));
    case.fixture.restore_client();
    fixed(case.verify(&mut credentials))?;
    assert_eq!(requests.borrow().len(), 2);
    Ok(())
}
fn advertisement_failure() -> Result<(), FixtureError> {
    let case = Case::new(true)?;
    case.fixture.disconnect_at(FixtureBoundary::Advertisement);
    let (mut credentials, requests) = session(vec![secret(PASSWORD)]);
    let kind = case.verify(&mut credentials).unwrap_err().kind;
    assert!(matches!(
        kind,
        SshTransportErrorKind::TransportUnavailable
            | SshTransportErrorKind::RemoteUnavailable
            | SshTransportErrorKind::ProtocolFailure
    ));
    assert_eq!(requests.borrow().len(), 1);
    assert!(!credentials.has_cached_passphrase(&requests.borrow()[0]));
    assert_eq!(
        case.fixture.accepted_keys(),
        vec![case.fixture.allowed_client_public_key()]
    );
    Ok(())
}
