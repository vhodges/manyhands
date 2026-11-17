//! A fake ssh-agent on a Unix socket: lists the identities it was given and
//! signs only on request, recording the key blob of every sign request.
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

pub struct FakeAgent {
    socket: PathBuf,
    signed: Arc<Mutex<Vec<Vec<u8>>>>,
    _dir: tempfile::TempDir,
}

impl FakeAgent {
    pub fn start(identities: Vec<ssh_key::PrivateKey>) -> std::io::Result<Self> {
        let dir = tempfile::tempdir()?;
        let socket = dir.path().join("agent.sock");
        let listener = UnixListener::bind(&socket)?;
        let signed = Arc::new(Mutex::new(Vec::new()));
        let log = signed.clone();
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let (identities, log) = (identities.clone(), log.clone());
                thread::spawn(move || serve(stream, &identities, &log));
            }
        });
        Ok(Self {
            socket,
            signed,
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

fn serve(mut stream: UnixStream, identities: &[ssh_key::PrivateKey], log: &Mutex<Vec<Vec<u8>>>) {
    while let Some(message) = read_message(&mut stream) {
        let mut out = Vec::new();
        match message.first().copied() {
            Some(REQUEST_IDENTITIES) => {
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
                match key.and_then(|key| Signer::<ssh_key::Signature>::try_sign(key, data).ok()) {
                    Some(signature) => {
                        out.push(SIGN_RESPONSE);
                        let mut wire = Vec::new();
                        string(&mut wire, signature.algorithm().as_str().as_bytes());
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

// ---- Spike: a git2 credential whose signing is delegated to an agent ----

use libgit2_sys as raw;
use std::{
    ffi::{CString, c_char, c_int, c_void},
    panic::{AssertUnwindSafe, catch_unwind},
};

/// What the sign callback needs. It must outlive the connection.
pub struct AgentSigner {
    pub socket: PathBuf,
    pub key_blob: Vec<u8>,
}

type SignCallback = extern "C" fn(
    *mut raw::LIBSSH2_SESSION,
    *mut *mut u8,
    *mut usize,
    *const u8,
    usize,
    *mut *mut c_void,
) -> c_int;

fn sign_through_agent(signer: &AgentSigner, data: &[u8]) -> Option<Vec<u8>> {
    let mut stream = UnixStream::connect(&signer.socket).ok()?;
    let mut request = vec![SIGN_REQUEST];
    string(&mut request, &signer.key_blob);
    string(&mut request, data);
    request.extend(0u32.to_be_bytes());
    reply(&mut stream, &request).ok()?;
    let response = read_message(&mut stream)?;
    if response.first() != Some(&SIGN_RESPONSE) {
        return None;
    }
    // The agent answers string(string(algorithm), string(blob)). libssh2 adds
    // the algorithm name itself, so it wants only the bare blob.
    let mut rest = &response[1..];
    let mut wire = take_string(&mut rest)?;
    take_string(&mut wire)?;
    take_string(&mut wire).map(<[u8]>::to_vec)
}

extern "C" fn sign_callback(
    _session: *mut raw::LIBSSH2_SESSION,
    sig: *mut *mut u8,
    sig_len: *mut usize,
    data: *const u8,
    data_len: usize,
    abstract_: *mut *mut c_void,
) -> c_int {
    catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: libgit2 passes `&cred->payload`, which is the `AgentSigner`
        // pointer given to `custom_credential`; libssh2 passes the data it
        // wants signed for the duration of the call.
        let (signer, data) = unsafe {
            (
                &*(*abstract_ as *const AgentSigner),
                std::slice::from_raw_parts(data, data_len),
            )
        };
        let Some(signature) = sign_through_agent(signer, data) else {
            return -1;
        };
        // SAFETY: libssh2 frees `sig` with its allocator, which is malloc here.
        unsafe {
            let buffer = libc::malloc(signature.len()) as *mut u8;
            if buffer.is_null() {
                return -1;
            }
            std::ptr::copy_nonoverlapping(signature.as_ptr(), buffer, signature.len());
            *sig = buffer;
            *sig_len = signature.len();
        }
        0
    }))
    .unwrap_or(-1)
}

/// A credential that offers exactly `signer.key_blob` and signs through the agent.
pub fn custom_credential(username: &str, signer: &AgentSigner) -> Result<git2::Cred, git2::Error> {
    const {
        assert!(size_of::<git2::Cred>() == size_of::<*mut raw::git_cred>());
        assert!(align_of::<git2::Cred>() == align_of::<*mut raw::git_cred>());
    }
    let username = CString::new(username).map_err(|_| git2::Error::from_str("username"))?;
    let mut out: *mut raw::git_cred = std::ptr::null_mut();
    // libgit2-sys declares the callback as returning nothing; the C type
    // returns int. Calls come from C, so the real signature is what matters.
    // SAFETY: same arguments, and the return value is read by libssh2 as int.
    let callback: extern "C" fn(
        *mut raw::LIBSSH2_SESSION,
        *mut *mut u8,
        *mut usize,
        *const u8,
        usize,
        *mut *mut c_void,
    ) = unsafe { std::mem::transmute::<SignCallback, _>(sign_callback) };
    // SAFETY: valid out pointer and C strings; `signer` outlives the connection
    // by the caller's contract.
    let status = unsafe {
        raw::git_cred_ssh_custom_new(
            &mut out,
            username.as_ptr(),
            signer.key_blob.as_ptr() as *const c_char,
            signer.key_blob.len(),
            Some(callback),
            signer as *const AgentSigner as *mut c_void,
        )
    };
    if status != 0 || out.is_null() {
        return Err(git2::Error::from_str("custom credential"));
    }
    // SAFETY: `git2::Cred` is a single `*mut git_cred`, checked above and by the
    // pinned git2 version; it frees the credential on drop.
    Ok(unsafe { std::mem::transmute::<*mut raw::git_cred, git2::Cred>(out) })
}
