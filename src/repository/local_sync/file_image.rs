//! Read bytes and relevant Git mode from the same no-follow descriptor.
//! Windows has no portable Unix executable bit; it is explicitly unobserved.
use std::{io, path::Path};
pub(super) struct Image {
    pub bytes: Vec<u8>,
    pub mode: Option<u32>,
}

#[cfg(unix)]
pub(super) fn read(root: &Path, relative: &Path) -> io::Result<Option<Image>> {
    use crate::repository::native_resolution;
    use std::{
        ffi::CString,
        io::Read,
        os::{
            fd::{AsRawFd, FromRawFd},
            unix::{ffi::OsStrExt, fs::MetadataExt},
        },
        path::Component,
    };
    let invalid = || io::Error::new(io::ErrorKind::InvalidInput, "invalid owned file image");
    if relative.as_os_str().is_empty()
        || !relative
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
    {
        return Err(invalid());
    }
    let mut parent = native_resolution::open_directory(root)?;
    for component in relative.parent().ok_or_else(invalid)?.components() {
        let Component::Normal(name) = component else {
            return Err(invalid());
        };
        let name = CString::new(name.as_bytes()).map_err(|_| invalid())?;
        parent = match native_resolution::open_directory_at(&parent, &name) {
            Ok(p) => p,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
    }
    let leaf = CString::new(relative.file_name().ok_or_else(invalid)?.as_bytes())
        .map_err(|_| invalid())?;
    // SAFETY: live pinned directory and a validated NUL-terminated leaf. Own the
    // successful descriptor exactly once; no symlink or blocking special file.
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            leaf.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        )
    };
    if fd < 0 {
        let error = io::Error::last_os_error();
        return if error.kind() == io::ErrorKind::NotFound {
            Ok(None)
        } else {
            Err(error)
        };
    }
    let mut file = unsafe { std::fs::File::from_raw_fd(fd) };
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(invalid());
    }
    let mode = if metadata.mode() & 0o111 == 0 {
        0o100644
    } else {
        0o100755
    };
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    let after = file.metadata()?;
    if after.len() != bytes.len() as u64 || after.mode() & 0o111 != metadata.mode() & 0o111 {
        return Err(invalid());
    }
    Ok(Some(Image {
        bytes,
        mode: Some(mode),
    }))
}
#[cfg(not(unix))]
pub(super) fn read(root: &Path, relative: &Path) -> io::Result<Option<Image>> {
    crate::repository::owned_file_bytes(
        root,
        relative,
        crate::repository::RepositoryOperation::ContextSyncLocal,
        root,
    )
    .map(|value| value.map(|bytes| Image { bytes, mode: None }))
    .map_err(|error| {
        let kind = error
            .source
            .as_deref()
            .and_then(|source| source.downcast_ref::<io::Error>())
            .map_or(io::ErrorKind::InvalidInput, io::Error::kind);
        io::Error::new(kind, "owned file image is inaccessible")
    })
}
