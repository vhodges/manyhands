//! Decision 5: `drafts/` is held open and every operation is relative to it,
//! as `repository/keys/storage/unix.rs` does for the key store.

use super::{
    DraftError, DraftErrorKind, Folder, Verdict, data_verdict, entry_name, folder_verdict,
};
use crate::drafts::UnreadableReason;
use std::{
    ffi::{CStr, CString},
    fs::{File, OpenOptions},
    io,
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::fs::{MetadataExt, OpenOptionsExt},
    },
    path::Path,
};

const DRAFTS: &str = "drafts";

impl Folder {
    pub(crate) fn open(data: &Path) -> Result<Folder, DraftError> {
        Ok(Folder {
            path: data.join(DRAFTS),
            handle: open_folder(data)?,
        })
    }

    /// Opens `drafts/` again, with the same checks, after the held folder
    /// was removed.
    pub(crate) fn reopen(&mut self) -> Result<(), DraftError> {
        self.handle = open_folder(self.data()?)?;
        Ok(())
    }

    /// A new owner-only file, made exclusively and never through a link. If
    /// the held folder has been removed, `drafts/` is reopened once.
    pub(crate) fn create_new(&mut self, name: &str) -> Result<File, DraftError> {
        let name = self::name(name)?;
        let create = |folder: &Folder| {
            open_at(
                &folder.handle,
                &name,
                libc::O_RDWR | libc::O_CREAT | libc::O_EXCL,
                0o600,
            )
        };
        check_folder(&self.handle)?;
        let file = match create(self) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                self.reopen()?;
                create(self)?
            }
            result => result?,
        };
        // The umask can only clear bits of 0600; make sure of the owner's.
        if unsafe { libc::fchmod(file.as_raw_fd(), 0o600) } != 0 {
            return Err(io::Error::last_os_error().into());
        }
        Ok(file)
    }

    /// A draft's lock file, opened for locking and made (0600) if absent;
    /// never through a link. If the held folder has been removed, `drafts/`
    /// is reopened once.
    pub(crate) fn open_lock(&mut self, name: &str) -> Result<File, DraftError> {
        let name = self::name(name)?;
        let open =
            |folder: &Folder| open_at(&folder.handle, &name, libc::O_RDWR | libc::O_CREAT, 0o600);
        let file = match open(self) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                self.reopen()?;
                open(self)?
            }
            result => result.map_err(protection)?,
        };
        let metadata = file.metadata()?;
        if !metadata.is_file() {
            return Err(DraftError::new(DraftErrorKind::Io));
        }
        if metadata.uid() != unsafe { libc::geteuid() } {
            return Err(DraftError::new(DraftErrorKind::ProtectionUnavailable));
        }
        Ok(file)
    }

    /// An existing draft file, for reading: a regular file, not a link, ours.
    /// `None` when there is no such file, for example because another
    /// process removed it after the folder was listed.
    pub(crate) fn open_file(&self, name: &str) -> Result<Option<File>, UnreadableReason> {
        let name = self::name(name).map_err(|_| UnreadableReason::NotAFile)?;
        let file = match open_at(&self.handle, &name, libc::O_RDONLY, 0) {
            Ok(file) => file,
            Err(error) => return unopened(&error),
        };
        let metadata = file.metadata().map_err(|_| UnreadableReason::Access)?;
        if !metadata.is_file() {
            return Err(UnreadableReason::NotAFile);
        }
        if metadata.uid() != unsafe { libc::geteuid() } {
            return Err(UnreadableReason::NotOwned);
        }
        Ok(Some(file))
    }

    pub(crate) fn rename(&self, from: &str, to: &str) -> Result<(), DraftError> {
        let (from, to) = (name(from)?, name(to)?);
        let fd = self.handle.as_raw_fd();
        if unsafe { libc::renameat(fd, from.as_ptr(), fd, to.as_ptr()) } != 0 {
            return Err(io::Error::last_os_error().into());
        }
        Ok(())
    }

    pub(crate) fn remove(&self, name: &str) -> Result<(), DraftError> {
        let name = self::name(name)?;
        if unsafe { libc::unlinkat(self.handle.as_raw_fd(), name.as_ptr(), 0) } != 0 {
            return Err(io::Error::last_os_error().into());
        }
        Ok(())
    }

    pub(crate) fn sync(&self) -> Result<(), DraftError> {
        Ok(self.handle.sync_all()?)
    }

    /// The names in the folder, `.` and `..` and names that are not UTF-8
    /// left out.
    pub(crate) fn names(&self) -> Result<Vec<String>, DraftError> {
        let fd = unsafe { libc::dup(self.handle.as_raw_fd()) };
        if fd < 0 {
            return Err(io::Error::last_os_error().into());
        }
        let directory = unsafe { libc::fdopendir(fd) };
        if directory.is_null() {
            let error = io::Error::last_os_error();
            unsafe { libc::close(fd) };
            return Err(error.into());
        }
        let directory = Directory(directory);
        // The duplicate shares the handle's position; start from the top.
        unsafe { libc::rewinddir(directory.0) };
        let mut names = Vec::new();
        loop {
            unsafe { *errno() = 0 };
            let entry = unsafe { libc::readdir(directory.0) };
            if entry.is_null() {
                if unsafe { *errno() } != 0 {
                    return Err(io::Error::last_os_error().into());
                }
                break;
            }
            let entry = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) };
            if let Ok(entry) = entry.to_str()
                && entry != "."
                && entry != ".."
            {
                names.push(entry.to_owned());
            }
        }
        Ok(names)
    }
}

/// A directory stream, closed (with its descriptor) on drop.
struct Directory(*mut libc::DIR);

impl Drop for Directory {
    fn drop(&mut self) {
        unsafe { libc::closedir(self.0) };
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn errno() -> *mut libc::c_int {
    unsafe { libc::__errno_location() }
}

#[cfg(any(target_os = "macos", target_os = "ios", target_os = "freebsd"))]
fn errno() -> *mut libc::c_int {
    unsafe { libc::__error() }
}

/// Opens (making it if absent) `<data>/drafts` and checks both folders.
fn open_folder(data: &Path) -> Result<File, DraftError> {
    let open = || {
        OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(data)
    };
    let data = match open() {
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            // Made as the library makes it, with the user's umask.
            std::fs::create_dir_all(data)?;
            open()
        }
        result => result,
    }
    .map_err(protection)?;
    let metadata = data.metadata()?;
    if data_verdict(metadata.mode(), metadata.uid(), unsafe { libc::geteuid() }) != Verdict::Ok {
        return Err(DraftError::new(DraftErrorKind::ProtectionUnavailable));
    }
    let drafts = name(DRAFTS)?;
    if unsafe { libc::mkdirat(data.as_raw_fd(), drafts.as_ptr(), 0o700) } == 0 {
        data.sync_all()?;
    } else {
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::AlreadyExists {
            return Err(error.into());
        }
    }
    let folder =
        open_at(&data, &drafts, libc::O_RDONLY | libc::O_DIRECTORY, 0).map_err(protection)?;
    check_folder(&folder)?;
    Ok(folder)
}

/// `drafts/` must be our directory; group and other bits are cleared.
fn check_folder(folder: &File) -> Result<(), DraftError> {
    let metadata = folder.metadata()?;
    match folder_verdict(
        metadata.mode(),
        metadata.uid(),
        unsafe { libc::geteuid() },
        metadata.is_dir(),
    ) {
        Verdict::Ok => Ok(()),
        Verdict::Tighten => {
            if unsafe { libc::fchmod(folder.as_raw_fd(), (metadata.mode() & 0o700) as _) } != 0 {
                return Err(DraftError::new(DraftErrorKind::ProtectionUnavailable));
            }
            Ok(())
        }
        Verdict::Refuse => Err(DraftError::new(DraftErrorKind::ProtectionUnavailable)),
    }
}

/// A link where a folder was expected is a protection failure.
fn protection(error: io::Error) -> DraftError {
    if is_link_or_not_directory(&error) {
        DraftError::new(DraftErrorKind::ProtectionUnavailable)
    } else {
        error.into()
    }
}

/// `O_NOFOLLOW` answers `ELOOP` for a link (`EMLINK` on FreeBSD), and
/// `O_DIRECTORY` answers `ENOTDIR` for anything but a folder.
fn is_link_or_not_directory(error: &io::Error) -> bool {
    matches!(
        error.raw_os_error(),
        Some(libc::ELOOP | libc::EMLINK | libc::ENOTDIR)
    )
}

/// A checked entry name (see `entry_name`), ready for a `*at` call.
fn name(name: &str) -> Result<CString, DraftError> {
    CString::new(entry_name(name)?).map_err(|_| DraftError::new(DraftErrorKind::Io))
}

/// Opens `name`, already checked by `name`, directly inside `parent`.
fn open_at(parent: &File, name: &CStr, flags: i32, mode: libc::mode_t) -> io::Result<File> {
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            // O_NONBLOCK keeps a FIFO from hanging the open before fstat
            // rejects it.
            flags | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            // mode_t is u16 on macOS; C variadic arguments need promotion.
            mode as libc::c_uint,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}

/// Why an existing name could not be opened as a draft file: `None` when it
/// is gone, a link or folder is not a file, and anything else is access.
fn unopened(error: &io::Error) -> Result<Option<File>, UnreadableReason> {
    if error.kind() == io::ErrorKind::NotFound {
        Ok(None)
    } else if is_link_or_not_directory(error) {
        Err(UnreadableReason::NotAFile)
    } else {
        Err(UnreadableReason::Access)
    }
}
