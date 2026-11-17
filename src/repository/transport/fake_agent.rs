//! A fake ssh-agent on a Unix socket for tests. It lists the identities it was
//! given, signs only on request, and records the key blob of every sign request.
//! Library unit tests and the SSH-harness target both include this one file.
use signature::Signer;
use std::{
    io::{Read, Write},
    os::unix::net::{UnixListener, UnixStream},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
};

const REQUEST_IDENTITIES: u8 = 11;
const IDENTITIES_ANSWER: u8 = 12;
const SIGN_REQUEST: u8 = 13;
const SIGN_RESPONSE: u8 = 14;
const FAILURE: u8 = 5;

/// How the agent misbehaves; `Normal` does not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Behavior {
    Normal,
    /// Answers every sign request with FAILURE.
    RefuseSign,
    /// Answers every request with bytes that are not a valid reply.
    Garbage,
    /// Claims a reply far longer than any agent sends.
    Oversized,
    /// Signs, but names another algorithm than the key's.
    WrongAlgorithm,
    /// Reads requests and never answers.
    Silent,
}

pub struct FakeAgent {
    socket: PathBuf,
    signed: Arc<Mutex<Vec<Vec<u8>>>>,
    listed: Arc<Mutex<usize>>,
    _dir: tempfile::TempDir,
}

impl FakeAgent {
    pub fn start(identities: Vec<ssh_key::PrivateKey>) -> std::io::Result<Self> {
        Self::start_with(identities, Behavior::Normal)
    }

    pub fn start_with(
        identities: Vec<ssh_key::PrivateKey>,
        behavior: Behavior,
    ) -> std::io::Result<Self> {
        let dir = tempfile::tempdir()?;
        let socket = dir.path().join("agent.sock");
        let listener = UnixListener::bind(&socket)?;
        let signed = Arc::new(Mutex::new(Vec::new()));
        let listed = Arc::new(Mutex::new(0));
        let (log, count) = (signed.clone(), listed.clone());
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let (identities, log, count) = (identities.clone(), log.clone(), count.clone());
                thread::spawn(move || serve(stream, &identities, behavior, &log, &count));
            }
        });
        Ok(Self {
            socket,
            signed,
            listed,
            _dir: dir,
        })
    }

    pub fn socket(&self) -> &Path {
        &self.socket
    }

    /// The key blob of every sign request the agent received, in order.
    pub fn signed(&self) -> Vec<Vec<u8>> {
        self.signed.lock().unwrap().clone()
    }

    /// How many identity lists the agent has answered.
    pub fn lists(&self) -> usize {
        *self.listed.lock().unwrap()
    }
}

fn read_message(stream: &mut UnixStream) -> Option<Vec<u8>> {
    let mut length = [0u8; 4];
    stream.read_exact(&mut length).ok()?;
    let mut body = vec![0u8; u32::from_be_bytes(length) as usize];
    stream.read_exact(&mut body).ok()?;
    Some(body)
}

fn string(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend((bytes.len() as u32).to_be_bytes());
    out.extend(bytes);
}

fn take_string<'a>(body: &mut &'a [u8]) -> Option<&'a [u8]> {
    let length = u32::from_be_bytes(body.get(..4)?.try_into().ok()?) as usize;
    let value = body.get(4..4 + length)?;
    *body = &body[4 + length..];
    Some(value)
}

fn reply(stream: &mut UnixStream, body: &[u8]) -> std::io::Result<()> {
    stream.write_all(&(body.len() as u32).to_be_bytes())?;
    stream.write_all(body)
}

fn serve(
    mut stream: UnixStream,
    identities: &[ssh_key::PrivateKey],
    behavior: Behavior,
    log: &Mutex<Vec<Vec<u8>>>,
    lists: &Mutex<usize>,
) {
    while let Some(message) = read_message(&mut stream) {
        match behavior {
            Behavior::Silent => continue,
            Behavior::Garbage => {
                if reply(&mut stream, &[0xff, 0xff, 0xff]).is_err() {
                    return;
                }
                continue;
            }
            Behavior::Oversized => {
                let _ = stream.write_all(&(64u32 * 1024 * 1024).to_be_bytes());
                return;
            }
            _ => {}
        }
        let mut out = Vec::new();
        match message.first().copied() {
            Some(REQUEST_IDENTITIES) => {
                *lists.lock().unwrap() += 1;
                out.push(IDENTITIES_ANSWER);
                out.extend((identities.len() as u32).to_be_bytes());
                for key in identities {
                    string(&mut out, &key.public_key().to_bytes().unwrap());
                    string(&mut out, b"fake agent identity");
                }
            }
            Some(SIGN_REQUEST) => {
                let mut rest = &message[1..];
                let (Some(blob), Some(data)) = (take_string(&mut rest), take_string(&mut rest))
                else {
                    out.push(FAILURE);
                    let _ = reply(&mut stream, &out);
                    continue;
                };
                log.lock().unwrap().push(blob.to_vec());
                let key = identities
                    .iter()
                    .find(|key| key.public_key().to_bytes().is_ok_and(|bytes| bytes == blob));
                let signature = match behavior {
                    Behavior::RefuseSign => None,
                    _ => key.and_then(|key| Signer::<ssh_key::Signature>::try_sign(key, data).ok()),
                };
                match signature {
                    Some(signature) => {
                        out.push(SIGN_RESPONSE);
                        let mut wire = Vec::new();
                        let genuine = signature.algorithm();
                        let algorithm = if behavior == Behavior::WrongAlgorithm {
                            "ssh-rsa"
                        } else {
                            genuine.as_str()
                        };
                        string(&mut wire, algorithm.as_bytes());
                        string(&mut wire, signature.as_bytes());
                        string(&mut out, &wire);
                    }
                    None => out.push(FAILURE),
                }
            }
            _ => out.push(FAILURE),
        }
        if reply(&mut stream, &out).is_err() {
            return;
        }
    }
}
