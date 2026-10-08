//! Private Unix resolution primitives shared by Linux and macOS.
//!
//! Namespace operations are for cooperative writers, not arbitrary-inode CAS.
//! File::sync_all supplies ordinary fsync barriers; it does not promise macOS
//! F_FULLFSYNC/device-cache power-loss durability. Resolution adds loose-object
//! and ref/log barriers; packed/alternate object storage still depends on the
//! backend's existing fsync policy. These helpers never change Git config.

use std::{
    ffi::CString,
    fs::File,
    io,
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{ffi::OsStrExt, fs::MetadataExt},
    },
    path::{Component, Path},
};

pub(super) fn open_directory_at(parent: &File, name: &CString) -> io::Result<File> {
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(unsafe { File::from_raw_fd(fd) })
    }
}

/// Start at the real filesystem root, then pin every ancestor without following
/// symlinks. Callers supply canonical absolute repository paths, not aliases.
pub(super) fn open_directory(path: &Path) -> io::Result<File> {
    if !path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "absolute directory required",
        ));
    }
    let fd = unsafe {
        libc::open(
            c"/".as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    let mut directory = unsafe { File::from_raw_fd(fd) };
    for component in path.components() {
        match component {
            Component::RootDir => {}
            Component::Normal(name) => {
                let name = CString::new(name.as_bytes()).map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidInput, "invalid directory")
                })?;
                directory = open_directory_at(&directory, &name)?;
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "invalid directory",
                ));
            }
        }
    }
    Ok(directory)
}

/// Path-based libgit2 operations use only this revalidated private namespace.
pub(super) fn directory_matches(path: &Path, retained: &File) -> io::Result<bool> {
    let current = open_directory(path)?.metadata()?;
    let retained = retained.metadata()?;
    Ok((current.dev(), current.ino()) == (retained.dev(), retained.ino()))
}
