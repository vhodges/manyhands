//! A small ssh-agent client: finds the selected key among the agent's
//! identities and signs with it, and never signs with any other.
//!
//! The agent is reached through the socket path the service captured from
//! SSH_AUTH_SOCK. Nothing here reads configuration files, and nothing from the
//! agent (comments, other keys, the socket path) leaves this module: every
//! failure is one of a closed set of reasons that carry no detail.
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

/// Longest message either side may send. An identities list for a very full
/// agent is far smaller.
const MAX_MESSAGE: usize = 256 * 1024;
/// Longest private key file read to learn its public half.
const MAX_KEY_FILE: u64 = 256 * 1024;
/// A local socket answers at once, so waiting longer means a dead agent.
const LIST_TIMEOUT: Duration = Duration::from_secs(5);
/// Longer than listing: an agent that confirms each use may wait for a person.
const SIGN_TIMEOUT: Duration = Duration::from_secs(30);

const REQUEST_IDENTITIES: u8 = 11;
const IDENTITIES_ANSWER: u8 = 12;
const SIGN_REQUEST: u8 = 13;
const SIGN_RESPONSE: u8 = 14;

/// Why the agent cannot be used for the selected key. Every reason leads to
/// the same behavior (the key file flow); the variants exist so tests pin
/// each one and a message can name the useful ones later.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AgentUnavailable {
    /// No agent support on this platform.
    #[cfg_attr(unix, allow(dead_code))]
    Unsupported,
    /// SSH_AUTH_SOCK was not set (or was empty).
    Unset,
    /// The socket could not be connected to.
    Unreachable,
    /// The agent's answer was malformed, oversized or did not arrive.
    Protocol,
    /// The selected key's file could not be read as an OpenSSH private key.
    KeyUnreadable,
    /// The selected key is of a type this slice does not sign for (RSA, and
    /// anything not ed25519 or ECDSA).
    KeyTypeUnsupported,
    /// The agent does not hold the selected key.
    KeyAbsent,
}

/// What signing for the selected key needs. It is the credential's payload and
/// must outlive the connection that uses it.
pub(super) struct AgentSigner {
    #[cfg_attr(not(unix), allow(dead_code))]
    socket: PathBuf,
    #[cfg_attr(not(unix), allow(dead_code))]
    key_blob: Vec<u8>,
    #[cfg_attr(not(unix), allow(dead_code))]
    sign_timeout: Duration,
}

impl AgentSigner {
    /// The selected key's public key in SSH wire format: the one key libssh2
    /// is told to offer.
    pub(super) fn key_blob(&self) -> &[u8] {
        &self.key_blob
    }

    /// Sign `data` with the selected key. Returns the bare signature blob, which
    /// is what libssh2 wants: it writes the algorithm name itself. `None` when
    /// the agent refuses, fails, times out, or answers with another algorithm.
    pub(super) fn sign(&self, data: &[u8]) -> Option<Vec<u8>> {
        #[cfg(not(unix))]
        {
            let _ = data;
            None
        }
        #[cfg(unix)]
        {
            let mut request = vec![SIGN_REQUEST];
            put_string(&mut request, &self.key_blob);
            put_string(&mut request, data);
            request.extend(0u32.to_be_bytes());
            let response = exchange(&self.socket, self.sign_timeout, &request).ok()?;
            let (&kind, rest) = response.split_first()?;
            if kind != SIGN_RESPONSE {
                return None;
            }
            let mut rest = rest;
            let mut wire = take_string(&mut rest)?;
            let algorithm = take_string(&mut wire)?;
            let blob = take_string(&mut wire)?;
            // An agent answering for another algorithm than the key's is not
            // answering for this key.
            (Some(algorithm) == key_algorithm(&self.key_blob)).then(|| blob.to_vec())
        }
    }
}

/// Find the selected key (by its private key file) among the agent's
/// identities.
pub(super) fn lookup(
    socket: Option<&Path>,
    private_key_path: &Path,
) -> Result<AgentSigner, AgentUnavailable> {
    lookup_with_timeouts(socket, private_key_path, LIST_TIMEOUT, SIGN_TIMEOUT)
}

fn lookup_with_timeouts(
    socket: Option<&Path>,
    private_key_path: &Path,
    list_timeout: Duration,
    sign_timeout: Duration,
) -> Result<AgentSigner, AgentUnavailable> {
    #[cfg(not(unix))]
    {
        let _ = (socket, private_key_path, list_timeout, sign_timeout);
        Err(AgentUnavailable::Unsupported)
    }
    #[cfg(unix)]
    {
        let socket = socket.ok_or(AgentUnavailable::Unset)?;
        let key_blob = selected_key_blob(private_key_path)?;
        let response = exchange(socket, list_timeout, &[REQUEST_IDENTITIES])?;
        let (&kind, rest) = response.split_first().ok_or(AgentUnavailable::Protocol)?;
        if kind != IDENTITIES_ANSWER {
            return Err(AgentUnavailable::Protocol);
        }
        let count = rest
            .get(..4)
            .and_then(|bytes| bytes.try_into().ok())
            .map(u32::from_be_bytes)
            .ok_or(AgentUnavailable::Protocol)?;
        let mut rest = &rest[4..];
        let mut held = false;
        for _ in 0..count {
            // Both fields are read, so a truncated list is malformed even
            // when the selected key came first. The comment is dropped here.
            let blob = take_string(&mut rest).ok_or(AgentUnavailable::Protocol)?;
            take_string(&mut rest).ok_or(AgentUnavailable::Protocol)?;
            held |= blob == key_blob.as_slice();
        }
        if !held {
            return Err(AgentUnavailable::KeyAbsent);
        }
        Ok(AgentSigner {
            socket: socket.to_owned(),
            key_blob,
            sign_timeout,
        })
    }
}

/// The selected key's public key blob, read from its private key file (an
/// OpenSSH private key shows its public half even when it is encrypted).
#[cfg(unix)]
fn selected_key_blob(path: &Path) -> Result<Vec<u8>, AgentUnavailable> {
    use std::io::Read;
    let mut bytes = zeroize::Zeroizing::new(Vec::new());
    std::fs::File::open(path)
        .and_then(|file| file.take(MAX_KEY_FILE).read_to_end(&mut bytes))
        .map_err(|_| AgentUnavailable::KeyUnreadable)?;
    let key = ssh_key::PrivateKey::from_openssh(bytes.as_slice())
        .map_err(|_| AgentUnavailable::KeyUnreadable)?;
    let blob = key
        .public_key()
        .to_bytes()
        .map_err(|_| AgentUnavailable::KeyUnreadable)?;
    match key_algorithm(&blob) {
        Some(
            b"ssh-ed25519"
            | b"ecdsa-sha2-nistp256"
            | b"ecdsa-sha2-nistp384"
            | b"ecdsa-sha2-nistp521",
        ) => Ok(blob),
        _ => Err(AgentUnavailable::KeyTypeUnsupported),
    }
}

/// The algorithm name a public key blob begins with.
fn key_algorithm(blob: &[u8]) -> Option<&[u8]> {
    let mut rest = blob;
    take_string(&mut rest)
}

fn put_string(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend((bytes.len() as u32).to_be_bytes());
    out.extend(bytes);
}

fn take_string<'a>(body: &mut &'a [u8]) -> Option<&'a [u8]> {
    let length = u32::from_be_bytes(body.get(..4)?.try_into().ok()?) as usize;
    let value = body.get(4..)?.get(..length)?;
    *body = &body[4 + length..];
    Some(value)
}

/// One request and its answer on a fresh connection.
#[cfg(unix)]
fn exchange(socket: &Path, timeout: Duration, request: &[u8]) -> Result<Vec<u8>, AgentUnavailable> {
    use std::{io::Read, io::Write, os::unix::net::UnixStream};
    let mut stream = UnixStream::connect(socket).map_err(|_| AgentUnavailable::Unreachable)?;
    let protocol = |_| AgentUnavailable::Protocol;
    stream.set_read_timeout(Some(timeout)).map_err(protocol)?;
    stream.set_write_timeout(Some(timeout)).map_err(protocol)?;
    stream
        .write_all(&(request.len() as u32).to_be_bytes())
        .and_then(|()| stream.write_all(request))
        .map_err(protocol)?;
    let mut length = [0u8; 4];
    stream.read_exact(&mut length).map_err(protocol)?;
    let length = u32::from_be_bytes(length) as usize;
    if length == 0 || length > MAX_MESSAGE {
        return Err(AgentUnavailable::Protocol);
    }
    let mut body = vec![0u8; length];
    stream.read_exact(&mut body).map_err(protocol)?;
    Ok(body)
}

#[cfg(all(test, unix))]
#[path = "agent_tests.rs"]
mod tests;
