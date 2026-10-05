use super::{FileIdentity, KeyFileKind, KeyMaterialError, KeyMaterialErrorKind, error, io_error};
use std::{
    ffi::CString,
    fs::{File, Metadata, OpenOptions},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{
            ffi::OsStrExt,
            fs::{MetadataExt, OpenOptionsExt},
        },
    },
    path::Path,
};

pub(super) struct Store {
    home: File,
    ssh: File,
    directory: File,
}

impl Store {
    pub(super) fn open(home: &Path) -> Result<Self, KeyMaterialError> {
        let home = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(home)
            .map_err(io_error)?;
        let ssh = directory(&home, ".ssh", false)?;
        let directory = directory(&ssh, "manyhands", true)?;
        Ok(Self {
            home,
            ssh,
            directory,
        })
    }
    pub(super) fn validate(&self) -> Result<(), KeyMaterialError> {
        validate_directory(&self.ssh, false)?;
        validate_directory(&self.directory, true)?;
        same_entry(&self.home, ".ssh", &self.ssh)?;
        same_entry(&self.ssh, "manyhands", &self.directory)
    }
    pub(super) fn open_lock(&self) -> Result<File, KeyMaterialError> {
        let file = match open_at(
            &self.directory,
            ".lock",
            libc::O_RDWR | libc::O_CREAT | libc::O_EXCL,
            0o600,
        ) {
            Ok(file) => {
                normalize_created_mode(&file, 0o600, false)?;
                file
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                open_at(&self.directory, ".lock", libc::O_RDWR, 0).map_err(io_error)?
            }
            Err(e) => return Err(io_error(e)),
        };
        validate_regular(&file, KeyFileKind::Private)?;
        Ok(file)
    }
    pub(super) fn create(&self, name: &str, kind: KeyFileKind) -> Result<File, KeyMaterialError> {
        let mode = match kind {
            KeyFileKind::Private => 0o600,
            KeyFileKind::Public => 0o644,
        };
        let file = open_at(
            &self.directory,
            name,
            libc::O_RDWR | libc::O_CREAT | libc::O_EXCL,
            0o600,
        )
        .map_err(io_error)?;
        // Exclusively-created files start owner-only. Normalize only this new
        // handle before any bytes, so umask cannot break the required final mode.
        normalize_created_mode(&file, mode, false)?;
        validate_regular(&file, kind)?;
        Ok(file)
    }
    pub(super) fn open_file(
        &self,
        name: &str,
        kind: KeyFileKind,
    ) -> Result<Option<File>, KeyMaterialError> {
        let file = match open_at(&self.directory, name, libc::O_RDONLY, 0) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(io_error(e)),
        };
        validate_regular(&file, kind)?;
        Ok(Some(file))
    }
    pub(super) fn validate_file(
        &self,
        file: &File,
        name: &str,
        kind: KeyFileKind,
    ) -> Result<(), KeyMaterialError> {
        validate_regular(file, kind)?;
        same_entry(&self.directory, name, file)
    }
    #[allow(dead_code, reason = "deletion consumer arrives in Task 6")]
    pub(super) fn remove(&self, file: &File, name: &str) -> Result<(), KeyMaterialError> {
        // A same-user adversarial rename between this check and unlinkat is outside
        // the threat model. Cooperating material operations hold the store lock.
        same_entry(&self.directory, name, file)?;
        let name = CString::new(name).map_err(|_| error(KeyMaterialErrorKind::UnsafePath))?;
        if unsafe { libc::unlinkat(self.directory.as_raw_fd(), name.as_ptr(), 0) } != 0 {
            return Err(io_error(std::io::Error::last_os_error()));
        }
        Ok(())
    }
    pub(super) fn sync_directory(&self) -> Result<(), KeyMaterialError> {
        self.directory.sync_all().map_err(io_error)
    }
}

fn open_at(parent: &File, name: &str, flags: i32, mode: libc::mode_t) -> std::io::Result<File> {
    let name =
        CString::new(name).map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            flags | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            mode,
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}
fn directory(parent: &File, name: &str, strict: bool) -> Result<File, KeyMaterialError> {
    let cname = CString::new(name).map_err(|_| error(KeyMaterialErrorKind::UnsafePath))?;
    let created = if unsafe { libc::mkdirat(parent.as_raw_fd(), cname.as_ptr(), 0o700) } != 0 {
        let e = std::io::Error::last_os_error();
        if e.kind() != std::io::ErrorKind::AlreadyExists {
            return Err(io_error(e));
        }
        false
    } else {
        parent.sync_all().map_err(io_error)?;
        true
    };
    let file = open_at(parent, name, libc::O_RDONLY | libc::O_DIRECTORY, 0).map_err(io_error)?;
    if created {
        normalize_created_mode(&file, 0o700, true)?;
    }
    validate_directory(&file, strict)?;
    Ok(file)
}
fn owned(metadata: &Metadata) -> bool {
    metadata.uid() == unsafe { libc::geteuid() }
}
fn normalize_created_mode(
    file: &File,
    mode: libc::mode_t,
    directory: bool,
) -> Result<(), KeyMaterialError> {
    let m = file.metadata().map_err(io_error)?;
    let initial_mode = if directory { 0o700 } else { 0o600 };
    if !owned(&m)
        || m.is_dir() != directory
        || (!directory && (!m.is_file() || m.nlink() != 1))
        || m.mode() & 0o7777 & !initial_mode != 0
    {
        return Err(error(KeyMaterialErrorKind::ProtectionUnavailable));
    }
    if unsafe { libc::fchmod(file.as_raw_fd(), mode) } != 0 {
        return Err(error(KeyMaterialErrorKind::ProtectionUnavailable));
    }
    Ok(())
}
fn validate_directory(file: &File, strict: bool) -> Result<(), KeyMaterialError> {
    let m = file.metadata().map_err(io_error)?;
    if !m.is_dir()
        || !owned(&m)
        || (strict && m.mode() & 0o7777 != 0o700)
        || (!strict && m.mode() & 0o7022 != 0)
    {
        return Err(error(KeyMaterialErrorKind::ProtectionUnavailable));
    }
    Ok(())
}
fn validate_regular(file: &File, kind: KeyFileKind) -> Result<(), KeyMaterialError> {
    let m = file.metadata().map_err(io_error)?;
    if !m.is_file() {
        return Err(error(KeyMaterialErrorKind::NotRegularFile));
    }
    let mode = match kind {
        KeyFileKind::Private => 0o600,
        KeyFileKind::Public => 0o644,
    };
    if !owned(&m) || m.mode() & 0o7777 != mode || m.nlink() != 1 {
        return Err(error(KeyMaterialErrorKind::ProtectionUnavailable));
    }
    Ok(())
}
fn same_entry(parent: &File, name: &str, file: &File) -> Result<(), KeyMaterialError> {
    let actual = open_at(parent, name, libc::O_RDONLY, 0)
        .map_err(|_| error(KeyMaterialErrorKind::SourceChanged))?;
    let expected = file.metadata().map_err(io_error)?;
    let actual = actual.metadata().map_err(io_error)?;
    if expected.dev() != actual.dev() || expected.ino() != actual.ino() {
        return Err(error(KeyMaterialErrorKind::SourceChanged));
    }
    Ok(())
}
pub(super) fn identity(file: &File) -> Result<FileIdentity, KeyMaterialError> {
    let m = file.metadata().map_err(io_error)?;
    Ok(FileIdentity {
        platform: "unix-v1",
        fields: [
            m.dev(),
            m.ino(),
            m.len(),
            m.mtime() as u64,
            m.mtime_nsec() as u64,
            m.ctime() as u64,
            m.ctime_nsec() as u64,
            u64::from(m.mode()),
        ],
    })
}
#[allow(dead_code, reason = "inspection consumer arrives in Task 5")]
pub(super) fn observe_regular_source(path: &Path) -> Result<FileIdentity, KeyMaterialError> {
    // O_NONBLOCK prevents FIFOs and devices from hanging before fstat rejects them.
    let _ = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| error(KeyMaterialErrorKind::UnsafePath))?;
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK | libc::O_CLOEXEC)
        .open(path)
        .map_err(|e| {
            error(if e.kind() == std::io::ErrorKind::NotFound {
                KeyMaterialErrorKind::SourceMissing
            } else {
                KeyMaterialErrorKind::SourceUnreadable
            })
        })?;
    if !file.metadata().map_err(io_error)?.is_file() {
        return Err(error(KeyMaterialErrorKind::NotRegularFile));
    }
    identity(&file)
}
