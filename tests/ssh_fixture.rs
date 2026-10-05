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

use ssh_remote::{FixtureBoundary, FixtureError, SshRemoteFixture, fixed, generate_key};
use std::{
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

fn main() {
    // SAFETY: actual main, before fixture, runner, runtime, or watchdog threads.
    if unsafe { ssh_harness::initialize() }.is_err() {
        eprintln!("SSH fixture initialization failed");
        std::process::exit(1);
    }
    ssh_harness::run(&[
        ("allowed_client", allowed_client),
        ("trust_known_hosts", trust_known_hosts),
        (
            "trust_pin_overrides_known_hosts",
            trust_pin_overrides_known_hosts,
        ),
        ("trust_recovered_known_hosts", trust_recovered_known_hosts),
        ("trust_unknown_host", trust_unknown_host),
        ("trust_exact_approval", trust_exact_approval),
        ("trust_selected_key_rejected", trust_selected_key_rejected),
        (
            "trust_default_key_fallback_refused",
            trust_default_key_fallback_refused,
        ),
        ("encrypted_ed25519", encrypted_ed25519),
        ("wrong_key_denied", wrong_key_denied),
        ("other_auth_denied", other_auth_denied),
        ("host_rotation_and_rejection", host_rotation_and_rejection),
        ("push_round_trip", push_round_trip),
        ("restricted_commands", restricted_commands),
        ("helper_eof_exit", helper_eof_exit),
        ("startup_failure", startup_failure),
        ("helper_spawn_failure", helper_spawn_failure),
        ("helper_lookup_spaces", helper_lookup_spaces),
        ("builtin_helpers_round_trip", builtin_helpers_round_trip),
        ("cleanup_failure", cleanup_failure),
        ("cleanup_live_helper", cleanup_live_helper),
        ("recoverable_stall", recoverable_stall),
        ("handshake_timeout", handshake_timeout),
        ("authentication_timeout", authentication_timeout),
        ("advertisement_timeout", advertisement_timeout),
        ("exec_acknowledgement_timeout", exec_acknowledgement_timeout),
        ("transfer_timeout", transfer_timeout),
        ("progressing_transfer", progressing_transfer),
    ]);
}

fn callbacks(key: &Path, password: Option<&str>) -> git2::RemoteCallbacks<'static> {
    let key = key.to_owned();
    let password = password.map(str::to_owned);
    let mut callbacks = git2::RemoteCallbacks::new();
    callbacks.certificate_check(|_, _| Ok(git2::CertificateCheckStatus::CertificateOk));
    callbacks.credentials(move |_, username, _| {
        git2::Cred::ssh_key(
            username.unwrap_or("fixture"),
            None,
            &key,
            password.as_deref(),
        )
    });
    callbacks
}

fn advertise(
    fixture: &SshRemoteFixture,
    key: &Path,
    password: Option<&str>,
) -> Result<Vec<git2::Oid>, git2::Error> {
    let root = tempfile::tempdir().map_err(|_| git2::Error::from_str("fixture"))?;
    let repo = git2::Repository::init(root.path())?;
    let mut remote = repo.remote_anonymous(&fixture.url())?;
    let connection =
        remote.connect_auth(git2::Direction::Fetch, Some(callbacks(key, password)), None)?;
    Ok(connection.list()?.iter().map(|head| head.oid()).collect())
}
fn allowed_client() -> Result<(), FixtureError> {
    let fixture = SshRemoteFixture::start()?;
    assert!(
        fixed(advertise(&fixture, fixture.client_key_path(), None))?.contains(&fixture.commit_id())
    );
    assert_eq!(
        fixture.accepted_keys(),
        vec![fixture.allowed_client_public_key()]
    );
    assert_eq!(fixture.helper_invocations(), 1);
    Ok(())
}

fn trust_known_hosts() -> Result<(), FixtureError> {
    trust_case(TrustCase::KnownHosts)
}
fn trust_pin_overrides_known_hosts() -> Result<(), FixtureError> {
    trust_case(TrustCase::PinOverridesKnownHosts)
}
fn trust_recovered_known_hosts() -> Result<(), FixtureError> {
    trust_case(TrustCase::RecoveredKnownHosts)
}
fn trust_unknown_host() -> Result<(), FixtureError> {
    trust_case(TrustCase::UnknownHost)
}
fn trust_exact_approval() -> Result<(), FixtureError> {
    trust_case(TrustCase::ExactApproval)
}
fn trust_selected_key_rejected() -> Result<(), FixtureError> {
    trust_case(TrustCase::SelectedKeyRejected)
}
fn trust_default_key_fallback_refused() -> Result<(), FixtureError> {
    trust_case(TrustCase::DefaultKeyFallbackRefused)
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum TrustCase {
    KnownHosts,
    PinOverridesKnownHosts,
    RecoveredKnownHosts,
    UnknownHost,
    ExactApproval,
    SelectedKeyRejected,
    DefaultKeyFallbackRefused,
}
fn trust_case(mode: TrustCase) -> Result<(), FixtureError> {
    use repository::transport::{
        HostApproval, HostKeyIdentity, SshAuthority, SshTransportErrorKind,
    };
    let fixture = SshRemoteFixture::start()?;
    let known_hosts =
        std::path::PathBuf::from(std::env::var_os("MANYHANDS_SSH_TEST_HOME").ok_or(FixtureError)?)
            .join(".ssh/known_hosts");
    if mode != TrustCase::UnknownHost && mode != TrustCase::ExactApproval {
        let entry = format!(
            "[127.0.0.1]:{} {}\n",
            fixture.address().port(),
            fixed(fixture.host_public_key().to_openssh())?
        );
        fixed(std::fs::write(&known_hosts, entry))?;
    }
    let before = fixed(std::fs::read(&known_hosts))?;
    let observed = fixture.host_identity();
    let old_key = generate_key()?;
    let old = HostKeyIdentity {
        algorithm: old_key.algorithm().to_string(),
        sha256: old_key
            .public_key()
            .fingerprint(russh::keys::HashAlg::Sha256)
            .to_string(),
    };
    let authority = SshAuthority {
        host: "127.0.0.1".into(),
        port: fixture.address().port(),
    };
    let approval = (mode == TrustCase::ExactApproval).then(|| HostApproval {
        authority,
        expected: None,
        presented: observed.clone(),
    });
    if mode == TrustCase::SelectedKeyRejected {
        fixture.reject_client();
    }
    if mode == TrustCase::DefaultKeyFallbackRefused {
        let home = std::path::PathBuf::from(
            std::env::var_os("MANYHANDS_SSH_TEST_HOME").ok_or(FixtureError)?,
        );
        let default_key = fixed(russh::keys::load_secret_key(
            home.join(".ssh/id_ed25519"),
            None,
        ))?;
        fixture.allow_client_public_key(default_key.public_key().clone());
    }
    let probe = repository::transport::tests::callback_handshake(
        &fixture.url(),
        fixture.client_key_path(),
        (mode == TrustCase::PinOverridesKnownHosts || mode == TrustCase::RecoveredKnownHosts)
            .then_some(old.clone()),
        approval,
        mode == TrustCase::RecoveredKnownHosts,
    );
    assert_eq!(probe.observed, Some(observed.clone()));
    match mode {
        TrustCase::KnownHosts => {
            assert!(probe.connected);
            assert!(probe.passthrough);
            assert_eq!(probe.pin, None);
        }
        TrustCase::PinOverridesKnownHosts => {
            assert!(!probe.connected);
            assert_eq!(
                probe.failure,
                Some(SshTransportErrorKind::HostReplacementRequired {
                    expected: old,
                    presented: observed
                })
            );
        }
        TrustCase::RecoveredKnownHosts => {
            assert!(!probe.connected);
            assert!(!probe.passthrough);
            assert_eq!(
                probe.failure,
                Some(SshTransportErrorKind::HostApprovalRequired {
                    presented: observed
                })
            );
        }
        TrustCase::UnknownHost => {
            assert!(!probe.connected);
            assert!(probe.passthrough);
            assert_eq!(probe.backend_code, Some(git2::ErrorCode::Certificate));
        }
        TrustCase::ExactApproval => {
            assert!(probe.connected);
            assert_eq!(probe.pin, Some(observed));
        }
        TrustCase::SelectedKeyRejected | TrustCase::DefaultKeyFallbackRefused => {
            assert!(!probe.connected);
            assert_eq!(probe.failure, Some(SshTransportErrorKind::KeyRejected));
            assert_eq!(probe.key_submissions, 1);
            assert!(fixture.accepted_keys().is_empty());
        }
    }
    if probe.connected {
        assert_eq!(
            fixture.accepted_keys(),
            vec![fixture.allowed_client_public_key()]
        );
        assert_eq!(probe.key_submissions, 1);
    } else {
        assert_eq!(fixture.helper_invocations(), 0);
    }
    assert_eq!(fixed(std::fs::read(&known_hosts))?, before);
    Ok(())
}
fn encrypted_ed25519() -> Result<(), FixtureError> {
    let fixture = SshRemoteFixture::start()?;
    let key = fixed(ssh_key::PrivateKey::read_openssh_file(
        fixture.client_key_path(),
    ))?;
    let encrypted = fixed(key.encrypt(&mut ssh_key::rand_core::OsRng, "fixture-only secret"))?;
    fixed(std::fs::write(
        fixture.client_key_path(),
        fixed(encrypted.to_openssh(ssh_key::LineEnding::LF))?.as_bytes(),
    ))?;
    assert!(
        fixed(advertise(
            &fixture,
            fixture.client_key_path(),
            Some("fixture-only secret")
        ))?
        .contains(&fixture.commit_id())
    );
    assert_eq!(
        fixture.accepted_keys(),
        vec![fixture.allowed_client_public_key()]
    );
    Ok(())
}
fn wrong_key_denied() -> Result<(), FixtureError> {
    let fixture = SshRemoteFixture::start()?;
    let wrong = fixture.root().join("wrong key");
    fixed(std::fs::write(
        &wrong,
        fixed(generate_key()?.to_openssh(russh::keys::ssh_key::LineEnding::LF))?.as_bytes(),
    ))?;
    let error = advertise(&fixture, &wrong, None)
        .err()
        .ok_or(FixtureError)?;
    assert_eq!(error.code(), git2::ErrorCode::Auth);
    assert!(fixture.accepted_keys().is_empty());
    assert_eq!(fixture.helper_invocations(), 0);
    Ok(())
}
fn host_rotation_and_rejection() -> Result<(), FixtureError> {
    let fixture = SshRemoteFixture::start()?;
    let original = fixture.host_identity();
    fixture.rotate_host_key()?;
    assert_ne!(fixture.host_identity(), original);
    fixed(advertise(&fixture, fixture.client_key_path(), None))?;
    fixture.reject_client();
    assert_eq!(
        advertise(&fixture, fixture.client_key_path(), None)
            .err()
            .ok_or(FixtureError)?
            .code(),
        git2::ErrorCode::Auth
    );
    assert_eq!(fixture.helper_invocations(), 1);
    Ok(())
}
fn push_round_trip() -> Result<(), FixtureError> {
    let fixture = SshRemoteFixture::start()?;
    push_to_fixture(&fixture)?;
    assert_eq!(
        fixture.accepted_keys(),
        vec![fixture.allowed_client_public_key()]
    );
    Ok(())
}
fn push_to_fixture(fixture: &SshRemoteFixture) -> Result<(), FixtureError> {
    let root = fixed(tempfile::tempdir())?;
    let repo = fixed(git2::Repository::init(root.path()))?;
    let tree_id = fixed(fixed(repo.treebuilder(None))?.write())?;
    let tree = fixed(repo.find_tree(tree_id))?;
    let signature = fixed(git2::Signature::now("Fixture", "fixture@example.invalid"))?;
    let oid = fixed(repo.commit(
        Some("refs/heads/pushed"),
        &signature,
        &signature,
        "pushed fixture",
        &tree,
        &[],
    ))?;
    let mut remote = fixed(repo.remote_anonymous(&fixture.url()))?;
    let mut options = git2::PushOptions::new();
    options.remote_callbacks(callbacks(fixture.client_key_path(), None));
    fixed(remote.push(&["refs/heads/pushed:refs/heads/pushed"], Some(&mut options)))?;
    let server_repo = fixed(git2::Repository::open_bare(fixture.repository_path()))?;
    assert_eq!(fixed(server_repo.refname_to_id("refs/heads/pushed"))?, oid);
    Ok(())
}
fn other_auth_denied() -> Result<(), FixtureError> {
    let fixture = SshRemoteFixture::start()?;
    async_case(async {
        let mut client = fixed(
            russh::client::connect(
                Arc::new(russh::client::Config::default()),
                fixture.address(),
                Client,
            )
            .await,
        )?;
        assert!(!fixed(client.authenticate_none("fixture").await)?.success());
        assert!(
            !fixed(
                client
                    .authenticate_password("fixture", "fixture-only secret")
                    .await
            )?
            .success()
        );
        assert!(fixture.accepted_keys().is_empty());
        assert_eq!(fixture.helper_invocations(), 0);
        Ok(())
    })
}
struct Client;
impl russh::client::Handler for Client {
    type Error = russh::Error;
    async fn check_server_key(
        &mut self,
        _: &russh::keys::PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}
async fn client(fixture: &SshRemoteFixture) -> Result<russh::client::Handle<Client>, FixtureError> {
    let mut client = fixed(
        russh::client::connect(
            Arc::new(russh::client::Config::default()),
            fixture.address(),
            Client,
        )
        .await,
    )?;
    let key = fixed(russh::keys::load_secret_key(
        fixture.client_key_path(),
        None,
    ))?;
    let result = fixed(
        client
            .authenticate_publickey(
                "fixture",
                russh::keys::PrivateKeyWithHashAlg::new(Arc::new(key), None),
            )
            .await,
    )?;
    assert!(result.success());
    Ok(client)
}
fn async_case<F: std::future::Future<Output = Result<(), FixtureError>>>(
    future: F,
) -> Result<(), FixtureError> {
    fixed(
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build(),
    )?
    .block_on(async { fixed(tokio::time::timeout(Duration::from_secs(10), future).await)? })
}
fn restricted_commands() -> Result<(), FixtureError> {
    let fixture = SshRemoteFixture::start()?;
    reject_commands(&fixture)
}
fn reject_commands(fixture: &SshRemoteFixture) -> Result<(), FixtureError> {
    let helpers_before = fixture.helper_invocations();
    async_case(async {
        let client = client(fixture).await?;
        for command in [
            "git-upload-pack '/other.git'",
            "git-upload-pack '/fixture.git'; exit",
            "git-upload-pack /fixture.git",
            "git-receive-pack '../fixture.git'",
            "whoami",
        ] {
            let mut channel = fixed(client.channel_open_session().await)?;
            fixed(channel.exec(true, command).await)?;
            let mut rejected = false;
            while let Some(message) = channel.wait().await {
                if matches!(message, russh::ChannelMsg::Failure) {
                    rejected = true;
                    break;
                }
            }
            assert!(rejected);
        }
        let mut channel = fixed(client.channel_open_session().await)?;
        fixed(channel.request_shell(true).await)?;
        assert!(matches!(
            channel.wait().await,
            Some(russh::ChannelMsg::Failure)
        ));
        fixed(channel.request_pty(true, "xterm", 80, 24, 0, 0, &[]).await)?;
        assert!(matches!(
            channel.wait().await,
            Some(russh::ChannelMsg::Failure)
        ));
        assert!(
            client
                .channel_open_direct_tcpip("127.0.0.1", 22, "127.0.0.1", 1234)
                .await
                .is_err()
        );
        assert!(client.tcpip_forward("127.0.0.1", 0).await.is_err());
        assert_eq!(fixture.helper_invocations(), helpers_before);
        Ok(())
    })
}
fn helper_eof_exit() -> Result<(), FixtureError> {
    let fixture = SshRemoteFixture::start()?;
    async_case(async {
        let client = client(&fixture).await?;
        let mut channel = fixed(client.channel_open_session().await)?;
        fixed(channel.exec(true, "git-receive-pack '/fixture.git'").await)?;
        fixed(channel.data(&b"0000"[..]).await)?;
        fixed(channel.eof().await)?;
        let (mut bytes, mut eof, mut exit) = (0, false, None);
        while let Some(message) = channel.wait().await {
            match message {
                russh::ChannelMsg::Data { data } => bytes += data.len(),
                russh::ChannelMsg::Eof => eof = true,
                russh::ChannelMsg::ExitStatus { exit_status } => exit = Some(exit_status),
                _ => {}
            }
        }
        assert!(bytes > 0);
        assert!(eof);
        assert_eq!(exit, Some(0));
        Ok(())
    })
}
fn startup_failure() -> Result<(), FixtureError> {
    let root = fixed(tempfile::tempdir())?;
    assert!(
        SshRemoteFixture::start_with_helpers((
            root.path().join("missing").into(),
            root.path().join("missing").into()
        ))
        .is_err()
    );
    Ok(())
}
fn helper_lookup_spaces() -> Result<(), FixtureError> {
    let directory = fixed(
        tempfile::Builder::new()
            .prefix("Git helper path with spaces ")
            .tempdir(),
    )?;
    let suffix = if cfg!(windows) { ".exe" } else { "" };
    let upload = directory.path().join(format!("git-upload-pack{suffix}"));
    let receive = directory.path().join(format!("git-receive-pack{suffix}"));
    fixed(std::fs::write(&upload, b""))?;
    fixed(std::fs::write(&receive, b""))?;
    assert_eq!(
        ssh_remote::discover_helpers_at(directory.path())?,
        (upload.into(), receive.into())
    );
    Ok(())
}
fn helper_spawn_failure() -> Result<(), FixtureError> {
    let directory = fixed(tempfile::tempdir())?;
    let invalid = directory.path().join("invalid executable");
    fixed(std::fs::write(&invalid, b"invalid executable"))?;
    let mut fixture =
        SshRemoteFixture::start_with_helpers((invalid.clone().into(), invalid.into()))?;
    let path = fixture.root().to_owned();
    assert!(advertise(&fixture, fixture.client_key_path(), None).is_err());
    assert_eq!(fixture.helper_invocations(), 0);
    fixture.shutdown()?;
    drop(fixture);
    assert!(!path.exists());
    Ok(())
}
fn builtin_helpers_round_trip() -> Result<(), FixtureError> {
    // Git for Windows can omit the dashed executable aliases. Exercise that
    // packaging on every platform without changing the process environment.
    let directory = fixed(
        tempfile::Builder::new()
            .prefix("empty Git exec path ")
            .tempdir(),
    )?;
    let helpers = ssh_remote::discover_helpers_at(directory.path())?;
    let mut fixture = SshRemoteFixture::start_with_helpers(helpers)?;
    assert!(
        fixed(advertise(&fixture, fixture.client_key_path(), None))?.contains(&fixture.commit_id())
    );
    push_to_fixture(&fixture)?;
    assert_eq!(fixture.helper_invocations(), 2);
    assert_eq!(
        fixture.accepted_keys(),
        vec![fixture.allowed_client_public_key(); 2]
    );
    reject_commands(&fixture)?;
    let root = fixture.root().to_owned();
    fixture.shutdown()?;
    assert_eq!(fixture.active_helpers(), 0);
    drop(fixture);
    assert!(!root.exists());
    Ok(())
}
fn cleanup_failure() -> Result<(), FixtureError> {
    let fixture = SshRemoteFixture::start()?;
    let path = fixture.root().to_owned();
    let address = fixture.address();
    fixture.disconnect_at(FixtureBoundary::Advertisement);
    assert!(advertise(&fixture, fixture.client_key_path(), None).is_err());
    drop(fixture);
    assert!(!path.exists());
    assert!(std::net::TcpStream::connect_timeout(&address, Duration::from_secs(1)).is_err());
    Ok(())
}
fn cleanup_live_helper() -> Result<(), FixtureError> {
    let mut fixture = SshRemoteFixture::start()?;
    let root = fixed(tempfile::tempdir())?;
    let repo = fixed(git2::Repository::init(root.path()))?;
    let mut remote = fixed(repo.remote_anonymous(&fixture.url()))?;
    let connection = fixed(remote.connect_auth(
        git2::Direction::Fetch,
        Some(callbacks(fixture.client_key_path(), None)),
        None,
    ))?;
    assert_eq!(fixture.active_helpers(), 1);
    fixture.shutdown()?;
    assert_eq!(fixture.active_helpers(), 0);
    assert_eq!(fixture.completed_helpers(), 1);
    drop(connection);
    Ok(())
}
fn recoverable_stall() -> Result<(), FixtureError> {
    let fixture = SshRemoteFixture::start()?;
    fixture.stall_at(FixtureBoundary::Authentication, Duration::from_secs(2));
    let start = Instant::now();
    fixed(advertise(&fixture, fixture.client_key_path(), None))?;
    assert!(start.elapsed() >= Duration::from_secs(2));
    assert!(start.elapsed() < Duration::from_secs(10));
    Ok(())
}
fn timeout_case(boundary: FixtureBoundary) -> Result<(), FixtureError> {
    let fixture = SshRemoteFixture::start()?;
    fixture.stall_at(boundary, Duration::from_secs(45));
    let root = fixed(tempfile::tempdir())?;
    let repo = fixed(git2::Repository::init(root.path()))?;
    let mut remote = fixed(repo.remote_anonymous(&fixture.url()))?;
    let start = Instant::now();
    let result = if boundary == FixtureBoundary::Transfer {
        let mut options = git2::FetchOptions::new();
        options.remote_callbacks(callbacks(fixture.client_key_path(), None));
        remote.fetch(
            &["refs/heads/main:refs/remotes/origin/main"],
            Some(&mut options),
            None,
        )
    } else {
        remote
            .connect_auth(
                git2::Direction::Fetch,
                Some(callbacks(fixture.client_key_path(), None)),
                None,
            )
            .map(drop)
    };
    let elapsed = start.elapsed();
    ssh_harness::observation(&[1, elapsed.as_millis(), u128::from(result.is_ok())]);
    let error = result.err().ok_or(FixtureError)?;
    ssh_harness::observation(&[3, error.code() as u128, error.class() as u128]);
    eprintln!(
        "boundary {boundary:?}: code {:?}, class {:?}, return {:?}",
        error.code(),
        error.class(),
        elapsed
    );
    assert!(elapsed >= Duration::from_secs(28));
    // A failed command-control call performs additional channel cleanup inside
    // libgit2 before returning. Our server releases that call after 45 seconds.
    let upper = if boundary == FixtureBoundary::ExecAcknowledgement {
        55
    } else {
        40
    };
    assert!(elapsed < Duration::from_secs(upper));
    assert_ne!(error.code(), git2::ErrorCode::Auth);
    let teardown = Instant::now();
    drop(remote);
    drop(fixture);
    eprintln!("teardown {:?}", teardown.elapsed());
    ssh_harness::observation(&[2, teardown.elapsed().as_millis()]);
    Ok(())
}
fn handshake_timeout() -> Result<(), FixtureError> {
    timeout_case(FixtureBoundary::Handshake)
}
fn authentication_timeout() -> Result<(), FixtureError> {
    timeout_case(FixtureBoundary::Authentication)
}
fn advertisement_timeout() -> Result<(), FixtureError> {
    timeout_case(FixtureBoundary::Advertisement)
}
fn exec_acknowledgement_timeout() -> Result<(), FixtureError> {
    timeout_case(FixtureBoundary::ExecAcknowledgement)
}
fn transfer_timeout() -> Result<(), FixtureError> {
    timeout_case(FixtureBoundary::Transfer)
}
fn progressing_transfer() -> Result<(), FixtureError> {
    let fixture = SshRemoteFixture::start()?;
    // Deterministic incompressible blob creates many independent SSH read calls.
    let repository = fixed(git2::Repository::open_bare(fixture.repository_path()))?;
    let mut bytes = vec![0u8; 320_000];
    let mut value = 1u64;
    for byte in &mut bytes {
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        *byte = value as u8;
    }
    let blob = fixed(repository.blob(&bytes))?;
    let mut builder = fixed(repository.treebuilder(None))?;
    fixed(builder.insert("payload", blob, 0o100644))?;
    let tree_id = fixed(builder.write())?;
    let tree = fixed(repository.find_tree(tree_id))?;
    let parent = fixed(repository.find_commit(fixture.commit_id()))?;
    let signature = fixed(git2::Signature::now("Fixture", "fixture@example.invalid"))?;
    let oid = fixed(repository.commit(
        Some("refs/heads/main"),
        &signature,
        &signature,
        "large",
        &tree,
        &[&parent],
    ))?;
    fixture.pace_transfer(Duration::from_secs(2));
    let target = fixed(tempfile::tempdir())?;
    let repo = fixed(git2::Repository::init(target.path()))?;
    let mut remote = fixed(repo.remote_anonymous(&fixture.url()))?;
    let mut options = git2::FetchOptions::new();
    options.remote_callbacks(callbacks(fixture.client_key_path(), None));
    let start = Instant::now();
    fixed(remote.fetch(
        &["refs/heads/main:refs/remotes/origin/main"],
        Some(&mut options),
        None,
    ))?;
    ssh_harness::observation(&[4, start.elapsed().as_millis()]);
    assert!(start.elapsed() > Duration::from_secs(30));
    assert!(start.elapsed() < Duration::from_secs(90));
    assert_eq!(fixed(repo.refname_to_id("refs/remotes/origin/main"))?, oid);
    assert_eq!(
        fixture.accepted_keys(),
        vec![fixture.allowed_client_public_key()]
    );
    Ok(())
}
