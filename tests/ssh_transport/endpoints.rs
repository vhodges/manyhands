use crate::{
    repository::transport::{operation_tests::*, *},
    ssh_remote::*,
};

pub const CASES: &[crate::ssh_harness::Case] = &[
    (
        "endpoint_backend_scp_port_characterization",
        backend_scp_port_characterization,
    ),
    ("endpoint_path_commands", path_commands),
    ("endpoint_url_delimiter_preflight", url_delimiter_preflight),
    ("endpoint_path_rewrite_preflight", path_rewrite_preflight),
];

fn path_commands() -> Result<(), FixtureError> {
    for (configured, expected) in [
        ("fixture@EXAMPLE.invalid:fixture.git", "fixture.git"),
        ("fixture@example.invalid:/fixture.git", "/fixture.git"),
        ("fixture@example.invalid:~/fixture.git", "~/fixture.git"),
        (
            "fixture@example.invalid:~user/fixture.git",
            "~user/fixture.git",
        ),
        ("fixture@example.invalid:/~/fixture.git", "~/fixture.git"),
        ("ssh://fixture@example.invalid/fixture.git", "/fixture.git"),
        (
            "ssh://fixture@example.invalid/~/fixture.git",
            "~/fixture.git",
        ),
        (
            "ssh://fixture@example.invalid/~user/fixture.git",
            "~user/fixture.git",
        ),
        (
            "fixture@example.invalid:fixture.git?other",
            "fixture.git?other",
        ),
        (
            "fixture@example.invalid:fixture.git#other",
            "fixture.git#other",
        ),
        (
            "fixture@example.invalid:/fixture.git?other",
            "/fixture.git?other",
        ),
        (
            "fixture@example.invalid:/fixture.git#other",
            "/fixture.git#other",
        ),
        ("fixture@[2001:db8::1]:fixture.git", "fixture.git"),
        ("ssh://fixture@[2001:db8::1]/fixture.git", "/fixture.git"),
    ] {
        for direction in [SshDirection::Fetch, SshDirection::Push] {
            let fixture = SshRemoteFixture::start()?;
            let connection = fixed(endpoint_connection_at(configured, fixture.address()))?;
            connect_command(&connection, expected, direction, &fixture)?;
        }
    }
    Ok(())
}

fn url_delimiter_preflight() -> Result<(), FixtureError> {
    for direction in [SshDirection::Fetch, SshDirection::Push] {
        for suffix in ["?other", "#other", "?", "#"] {
            let case = crate::session::Case::new(false)?;
            let repo = fixed(git2::Repository::open(&case.root))?;
            let url = format!("{}{suffix}", case.fixture.url());
            match direction {
                SshDirection::Fetch => fixed(repo.remote_set_url("origin", &url))?,
                SshDirection::Push => fixed(repo.remote_set_pushurl("origin", Some(&url)))?,
            }
            assert_preflight_rejected(&case, direction)?;
        }
    }
    Ok(())
}

fn path_rewrite_preflight() -> Result<(), FixtureError> {
    for direction in [SshDirection::Fetch, SshDirection::Push] {
        for reverse in [false, true] {
            let case = crate::session::Case::new(false)?;
            let repo = fixed(git2::Repository::open(&case.root))?;
            let relative = "fixture@127.0.0.1:fixture.git";
            let absolute = "ssh://fixture@127.0.0.1/fixture.git";
            let (from, to) = if reverse {
                (absolute, relative)
            } else {
                (relative, absolute)
            };
            fixed(repo.remote_set_url("origin", from))?;
            let rule = if direction == SshDirection::Fetch {
                "insteadOf"
            } else {
                "pushInsteadOf"
            };
            fixed(fixed(repo.config())?.set_str(&format!("url.{to}.{rule}"), from))?;
            assert_preflight_rejected(&case, direction)?;
        }
    }
    Ok(())
}

fn assert_preflight_rejected(
    case: &crate::session::Case,
    direction: SshDirection,
) -> Result<(), FixtureError> {
    let (mut session, requests) = crate::session::session(vec![]);
    let mut request = case.request();
    request.direction = direction;
    let error = case
        .service
        .verify_ssh_transport(request, &mut session)
        .unwrap_err();
    assert_eq!(error.kind, SshTransportErrorKind::ConfigurationInvalid);
    assert!(requests.borrow().is_empty());
    assert!(case.fixture.accepted_keys().is_empty());
    assert!(case.fixture.commands().is_empty());
    assert_eq!(case.fixture.helper_invocations(), 0);
    Ok(())
}

fn connect_command(
    url: &str,
    target: &str,
    direction: SshDirection,
    fixture: &SshRemoteFixture,
) -> Result<(), FixtureError> {
    fixture.expect_command_path(target);
    let root = fixed(tempfile::tempdir())?;
    let repo = fixed(git2::Repository::init(root.path()))?;
    let mut remote = fixed(repo.remote_anonymous(url))?;
    let mut callbacks = git2::RemoteCallbacks::new();
    callbacks.certificate_check(|_, _| Ok(git2::CertificateCheckStatus::CertificateOk));
    callbacks.credentials(|_, username, _| {
        git2::Cred::ssh_key(username.unwrap(), None, fixture.client_key_path(), None)
    });
    let connection = fixed(remote.connect_auth(
        match direction {
            SshDirection::Fetch => git2::Direction::Fetch,
            SshDirection::Push => git2::Direction::Push,
        },
        Some(callbacks),
        None,
    ))?;
    assert!(
        fixed(connection.list())?
            .iter()
            .any(|head| head.oid() == fixture.commit_id())
    );
    let command = match direction {
        SshDirection::Fetch => format!("git-upload-pack '{target}'"),
        SshDirection::Push => format!("git-receive-pack '{target}'"),
    };
    assert_eq!(fixture.commands(), vec![command.into_bytes()]);
    assert_eq!(
        fixture.accepted_keys(),
        vec![fixture.allowed_client_public_key()]
    );
    assert_eq!(fixture.helper_invocations(), 1);
    Ok(())
}

fn backend_scp_port_characterization() -> Result<(), FixtureError> {
    // Backend-only spelling: no production parser or public contract accepts it.
    for direction in [SshDirection::Fetch, SshDirection::Push] {
        let fixture = SshRemoteFixture::start()?;
        let url = format!("[fixture@{}]:fixture.git", fixture.address());
        connect_command(&url, "fixture.git", direction, &fixture)?;
    }
    Ok(())
}
