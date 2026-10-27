//! Ephemeral checkout-image proof using the SAME libgit2 filter pipeline as
//! checkout. git2 0.20 does not wrap this public libgit2 API. These narrow C ABI
//! declarations use the already linked library; no dependency or Git subprocess.
//! Neither file contents nor filtered output are persisted or diagnosed here.
use super::Error;
use std::{
    ffi::{CString, c_char, c_void},
    ptr,
};

#[repr(C)]
struct Buffer {
    ptr: *mut c_char,
    reserved: usize,
    size: usize,
}
#[repr(C)]
struct Oid {
    id: [u8; 20],
}
unsafe extern "C" {
    fn git_object_owner(object: *const c_void) -> *mut c_void;
    fn git_blob_lookup(out: *mut *mut c_void, repo: *mut c_void, id: *const Oid) -> i32;
    fn git_blob_free(blob: *mut c_void);
    fn git_filter_list_load(
        out: *mut *mut c_void,
        repo: *mut c_void,
        blob: *mut c_void,
        path: *const c_char,
        mode: i32,
        flags: u32,
    ) -> i32;
    fn git_filter_list_apply_to_blob(
        out: *mut Buffer,
        filters: *mut c_void,
        blob: *mut c_void,
    ) -> i32;
    fn git_filter_list_free(filters: *mut c_void);
    fn git_buf_dispose(buffer: *mut Buffer);
}
struct Image {
    buffer: Buffer,
    filters: *mut c_void,
    blob: *mut c_void,
}
impl Drop for Image {
    fn drop(&mut self) {
        // SAFETY: each pointer is either null or was allocated by its paired
        // libgit2 API. Dispose the potentially blob-borrowing buffer first.
        unsafe {
            git_buf_dispose(&mut self.buffer);
            git_filter_list_free(self.filters);
            git_blob_free(self.blob);
        }
    }
}

pub(in super::super) fn checkout_bytes(
    git: &git2::Repository,
    commit: git2::Oid,
    blob: git2::Oid,
    path: &str,
) -> Result<Vec<u8>, Error> {
    let context = git
        .find_commit(commit)
        .map_err(|_| Error::RecoveryRequired)?;
    let path = CString::new(path).map_err(|_| Error::RecoveryRequired)?;
    let id = Oid {
        id: blob
            .as_bytes()
            .try_into()
            .map_err(|_| Error::RecoveryRequired)?,
    };
    let mut image = Image {
        buffer: Buffer {
            ptr: ptr::null_mut(),
            reserved: 0,
            size: 0,
        },
        filters: ptr::null_mut(),
        blob: ptr::null_mut(),
    };
    // SAFETY: Commit::raw is git2's public handle; a commit is a git_object.
    // Its owner remains alive for all calls. The SHA-1 OID, NUL-terminated path,
    // git_buf layout and opaque outputs match libgit2 1.x's public headers.
    // Mode 0 is TO_WORKTREE; flags 0 match checkout's default filter session.
    unsafe {
        let repo = git_object_owner(context.raw().cast());
        if git_blob_lookup(&mut image.blob, repo, &id) < 0
            || git_filter_list_load(&mut image.filters, repo, image.blob, path.as_ptr(), 0, 0) < 0
            || git_filter_list_apply_to_blob(&mut image.buffer, image.filters, image.blob) < 0
        {
            return Err(Error::RecoveryRequired);
        }
        if image.buffer.size == 0 {
            return Ok(Vec::new());
        }
        if image.buffer.ptr.is_null() {
            return Err(Error::RecoveryRequired);
        }
        Ok(std::slice::from_raw_parts(image.buffer.ptr.cast::<u8>(), image.buffer.size).to_vec())
    }
}
