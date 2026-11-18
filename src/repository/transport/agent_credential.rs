//! A git2 credential that offers exactly one public key and signs through the
//! agent.
//!
//! `git2::Cred::ssh_key_from_agent` cannot do this: libgit2 tries every key the
//! agent holds, in the agent's order. libgit2 does have a credential that
//! takes one public key and a signing callback (`git_cred_ssh_custom_new`), and
//! `libgit2-sys` declares it, but `git2` 0.20 neither exposes it nor lets a raw
//! credential become a `git2::Cred`. This module is the only `unsafe` bridge,
//! and it is meant to go away when git2 has `Cred::ssh_custom`.
//!
//! The assumptions it rests on, each pinned by something:
//! - `git2::Cred` is one `*mut git_cred` (the const assertions below, and the
//!   exact `git2` version in Cargo.toml).
//! - libgit2-sys declares the sign callback as returning nothing, while the C
//!   type returns `int`, which libssh2 reads. The callback is written with the
//!   real signature and cast (the unit test drives a real signing round trip
//!   through libssh2 in the SSH-harness case).
//! - libssh2 frees the signature with its allocator, which is `malloc`.
use super::agent::AgentSigner;
use libgit2_sys as raw;
use std::{
    ffi::{CString, c_char, c_int, c_void},
    panic::{AssertUnwindSafe, catch_unwind},
};

type SignCallback = extern "C" fn(
    *mut raw::LIBSSH2_SESSION,
    *mut *mut u8,
    *mut usize,
    *const u8,
    usize,
    *mut *mut c_void,
) -> c_int;

/// The callback type as libgit2-sys declares it (no return value).
type DeclaredSignCallback = extern "C" fn(
    *mut raw::LIBSSH2_SESSION,
    *mut *mut u8,
    *mut usize,
    *const u8,
    usize,
    *mut *mut c_void,
);

extern "C" fn sign_callback(
    _session: *mut raw::LIBSSH2_SESSION,
    sig: *mut *mut u8,
    sig_len: *mut usize,
    data: *const u8,
    data_len: usize,
    abstract_: *mut *mut c_void,
) -> c_int {
    // No panic may unwind into C.
    catch_unwind(AssertUnwindSafe(|| {
        if sig.is_null() || sig_len.is_null() || data.is_null() || abstract_.is_null() {
            return -1;
        }
        // SAFETY: libgit2 passes `&cred->payload`, which `credential` set to a
        // pointer to an `AgentSigner` that outlives the connection, and libssh2
        // passes `data_len` readable bytes for the duration of the call.
        let (signer, data) = unsafe {
            (
                &*((*abstract_) as *const AgentSigner),
                std::slice::from_raw_parts(data, data_len),
            )
        };
        let Some(signature) = signer.sign(data) else {
            return -1;
        };
        // SAFETY: libssh2 takes ownership of `*sig` and frees it with malloc's
        // `free`; the copy is `signature.len()` bytes of a fresh allocation.
        unsafe {
            let buffer = libc::malloc(signature.len().max(1)) as *mut u8;
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

/// A credential offering `signer`'s key and signing through the agent.
/// `signer` must outlive every use of the credential, which is the
/// connection attempt it is returned into.
pub(super) fn credential(username: &str, signer: &AgentSigner) -> Result<git2::Cred, git2::Error> {
    const {
        assert!(size_of::<git2::Cred>() == size_of::<*mut raw::git_cred>());
        assert!(align_of::<git2::Cred>() == align_of::<*mut raw::git_cred>());
    }
    let username = CString::new(username).map_err(|_| git2::Error::from_str("invalid username"))?;
    // SAFETY: only the return type differs, and callers are C code, which reads
    // the `int` this function returns.
    let callback = unsafe {
        std::mem::transmute::<SignCallback, DeclaredSignCallback>(sign_callback as SignCallback)
    };
    let key = signer.key_blob();
    let mut out: *mut raw::git_cred = std::ptr::null_mut();
    // SAFETY: valid out pointer and NUL-terminated username; libgit2 copies the
    // username and public key; the payload is `signer`, which the caller keeps
    // alive.
    let status = unsafe {
        raw::git_cred_ssh_custom_new(
            &mut out,
            username.as_ptr(),
            key.as_ptr() as *const c_char,
            key.len(),
            Some(callback),
            signer as *const AgentSigner as *mut c_void,
        )
    };
    if status != 0 || out.is_null() {
        return Err(git2::Error::from_str("agent credential unavailable"));
    }
    // SAFETY: `git2::Cred` is exactly this pointer (asserted above) and frees
    // the credential when dropped.
    Ok(unsafe { std::mem::transmute::<*mut raw::git_cred, git2::Cred>(out) })
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    /// The wrapped pointer must be the libgit2 credential it was made from, of
    /// the custom-signing kind, with the username, and must drop cleanly.
    #[test]
    fn wraps_a_custom_signing_credential() {
        // libgit2 allocates through state git2 sets up on first use; in a
        // connection that has always happened before a credential is asked for.
        git2::Config::new().unwrap();
        let directory = tempfile::tempdir().unwrap();
        let key = ssh_key::PrivateKey::random(
            &mut ssh_key::rand_core::OsRng,
            ssh_key::Algorithm::Ed25519,
        )
        .unwrap();
        let path = directory.path().join("key");
        std::fs::write(
            &path,
            key.to_openssh(ssh_key::LineEnding::LF).unwrap().as_bytes(),
        )
        .unwrap();
        let agent = super::super::fake_agent::FakeAgent::start(vec![key]).unwrap();
        let signer = super::super::agent::lookup(Some(agent.socket()), &path).unwrap();
        let wrapped = credential("git", &signer).unwrap();
        assert_eq!(wrapped.credtype(), raw::GIT_CREDTYPE_SSH_CUSTOM);
        assert!(wrapped.has_username());
        drop(wrapped);
        assert!(credential("gi\0t", &signer).is_err());
    }
}
