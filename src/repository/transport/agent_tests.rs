use super::*;
use crate::repository::transport::fake_agent::{Behavior, FakeAgent};
use std::time::Duration;

fn random_key() -> ssh_key::PrivateKey {
    ssh_key::PrivateKey::random(&mut ssh_key::rand_core::OsRng, ssh_key::Algorithm::Ed25519)
        .unwrap()
}

fn write_key(directory: &Path, name: &str, key: &ssh_key::PrivateKey) -> PathBuf {
    let path = directory.join(name);
    std::fs::write(
        &path,
        key.to_openssh(ssh_key::LineEnding::LF).unwrap().as_bytes(),
    )
    .unwrap();
    path
}

fn blob(key: &ssh_key::PrivateKey) -> Vec<u8> {
    key.public_key().to_bytes().unwrap()
}

/// Every reason the agent cannot serve the selected key, and the one case that
/// it can. Each non-success must leave the caller on the key file flow.
#[test]
fn lookup_reports_why_the_agent_is_unusable() {
    enum Socket {
        Unset,
        Missing,
        Agent(Vec<ssh_key::PrivateKey>, Behavior),
    }
    enum KeyFile {
        Selected,
        Missing,
        NotAKey,
    }
    let selected = random_key();
    let cases = [
        (
            "agent holds the selected key",
            Socket::Agent(vec![random_key(), selected.clone()], Behavior::Normal),
            KeyFile::Selected,
            Ok(()),
        ),
        (
            "SSH_AUTH_SOCK unset",
            Socket::Unset,
            KeyFile::Selected,
            Err(AgentUnavailable::Unset),
        ),
        (
            "socket does not exist",
            Socket::Missing,
            KeyFile::Selected,
            Err(AgentUnavailable::Unreachable),
        ),
        (
            "empty agent",
            Socket::Agent(vec![], Behavior::Normal),
            KeyFile::Selected,
            Err(AgentUnavailable::KeyAbsent),
        ),
        (
            "agent holds only other keys",
            Socket::Agent(vec![random_key(), random_key()], Behavior::Normal),
            KeyFile::Selected,
            Err(AgentUnavailable::KeyAbsent),
        ),
        (
            "agent answers with garbage",
            Socket::Agent(vec![selected.clone()], Behavior::Garbage),
            KeyFile::Selected,
            Err(AgentUnavailable::Protocol),
        ),
        (
            "agent claims an oversized reply",
            Socket::Agent(vec![selected.clone()], Behavior::Oversized),
            KeyFile::Selected,
            Err(AgentUnavailable::Protocol),
        ),
        (
            "agent never answers",
            Socket::Agent(vec![selected.clone()], Behavior::Silent),
            KeyFile::Selected,
            Err(AgentUnavailable::Protocol),
        ),
        (
            "key file missing",
            Socket::Agent(vec![selected.clone()], Behavior::Normal),
            KeyFile::Missing,
            Err(AgentUnavailable::KeyUnreadable),
        ),
        (
            "key file is not an OpenSSH private key",
            Socket::Agent(vec![selected.clone()], Behavior::Normal),
            KeyFile::NotAKey,
            Err(AgentUnavailable::KeyUnreadable),
        ),
    ];
    for (name, socket, key_file, expected) in cases {
        let directory = tempfile::tempdir().unwrap();
        let key_path = match key_file {
            KeyFile::Selected => write_key(directory.path(), "key", &selected),
            KeyFile::Missing => directory.path().join("missing"),
            KeyFile::NotAKey => {
                let path = directory.path().join("key");
                std::fs::write(&path, b"not a key").unwrap();
                path
            }
        };
        let agent;
        let socket_path = match socket {
            Socket::Unset => None,
            Socket::Missing => Some(directory.path().join("no-such-socket")),
            Socket::Agent(identities, behavior) => {
                agent = FakeAgent::start_with(identities, behavior).unwrap();
                Some(agent.socket().to_owned())
            }
        };
        let result = lookup_with_timeouts(
            socket_path.as_deref(),
            &key_path,
            Duration::from_millis(200),
            Duration::from_millis(200),
        );
        assert_eq!(result.map(|_| ()), expected, "{name}");
    }
}

/// A signature is only ever the bare blob for the selected key, and anything
/// the agent gets wrong means no signature at all.
#[test]
fn sign_returns_the_blob_or_nothing() {
    let selected = random_key();
    for (name, behavior, expected_len) in [
        ("normal", Behavior::Normal, Some(64)),
        ("agent refuses", Behavior::RefuseSign, None),
        (
            "agent names another algorithm",
            Behavior::WrongAlgorithm,
            None,
        ),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let key_path = write_key(directory.path(), "key", &selected);
        let agent = FakeAgent::start_with(vec![selected.clone()], behavior).unwrap();
        // Listing is not under test here, so list through a normal agent copy.
        let listing = FakeAgent::start(vec![selected.clone()]).unwrap();
        let mut signer = lookup(Some(listing.socket()), &key_path).unwrap();
        signer.socket = agent.socket().to_owned();
        assert_eq!(
            signer.sign(b"data to sign").map(|blob| blob.len()),
            expected_len,
            "{name}"
        );
    }
}

/// The reason for writing a client at all: with decoys listed first, only the
/// selected key's blob is ever sent in a sign request.
#[test]
fn only_the_selected_key_is_ever_asked_to_sign() {
    let directory = tempfile::tempdir().unwrap();
    let selected = random_key();
    let key_path = write_key(directory.path(), "key", &selected);
    let agent = FakeAgent::start(vec![random_key(), random_key(), selected.clone()]).unwrap();
    let signer = lookup(Some(agent.socket()), &key_path).unwrap();
    assert_eq!(signer.key_blob(), blob(&selected));
    assert!(signer.sign(b"first").is_some());
    assert!(signer.sign(b"second").is_some());
    assert_eq!(agent.signed(), vec![blob(&selected), blob(&selected)]);
    assert!(agent.lists() >= 1);
}
