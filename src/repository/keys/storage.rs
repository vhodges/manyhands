//! Handle-based protected storage. Raw material operations stay inside the crate.
use super::{KeyMaterialAction, KeyMaterialError, KeyMaterialErrorKind, SharedKeyId};
use fs4::fs_std::FileExt;
use std::{
    fs::File,
    io,
    path::{Path, PathBuf},
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

#[cfg(unix)]
#[path = "storage/unix.rs"]
mod platform;
#[cfg(windows)]
#[path = "storage/windows.rs"]
mod platform;

#[derive(Clone, Debug)]
pub struct KeyStore {
    home: PathBuf,
}

impl KeyStore {
    pub fn for_current_user() -> Result<Self, KeyMaterialError> {
        let dirs = directories::BaseDirs::new()
            .ok_or_else(|| error(KeyMaterialErrorKind::HomeUnavailable))?;
        Self::for_home(dirs.home_dir())
    }

    /// Configure an existing absolute home without creating SSH directories.
    /// Only the home anchor is resolved; descendants are opened without following links.
    pub fn for_home(home: &Path) -> Result<Self, KeyMaterialError> {
        if !home.is_absolute() {
            return Err(error(KeyMaterialErrorKind::HomeUnavailable));
        }
        let home = home
            .canonicalize()
            .map_err(|_| error(KeyMaterialErrorKind::HomeUnavailable))?;
        Ok(Self { home })
    }

    pub(crate) fn key_paths(&self, id: SharedKeyId) -> (PathBuf, PathBuf) {
        let directory = self.home.join(".ssh").join("manyhands");
        (
            directory.join(id.to_string()),
            directory.join(format!("{id}.pub")),
        )
    }

    pub(crate) fn lock(&self) -> Result<OwnedStoreGuard, KeyMaterialError> {
        let store = platform::Store::open(&self.home)?;
        let lock = store.open_lock()?;
        let deadline = Instant::now() + Duration::from_millis(250);
        loop {
            match FileExt::try_lock_exclusive(&lock) {
                Ok(true) => break,
                Ok(false) => {}
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                Err(_) => return Err(error(KeyMaterialErrorKind::StorageUnavailable)),
            }
            if Instant::now() >= deadline {
                return Err(error(KeyMaterialErrorKind::Busy));
            }
            thread::sleep(Duration::from_millis(10));
        }
        store.validate()?;
        Ok(OwnedStoreGuard(Arc::new(LockedStore {
            store,
            _lock: lock,
        })))
    }
}

struct LockedStore {
    store: platform::Store,
    _lock: File,
}
pub(crate) struct OwnedStoreGuard(Arc<LockedStore>);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KeyFileKind {
    Private,
    Public,
}

impl KeyFileKind {
    fn name(self, id: SharedKeyId) -> String {
        match self {
            Self::Private => id.to_string(),
            Self::Public => format!("{id}.pub"),
        }
    }
}

impl OwnedStoreGuard {
    pub(crate) fn create_private(&self, id: SharedKeyId) -> Result<OwnedKeyFile, KeyMaterialError> {
        self.create(id, KeyFileKind::Private)
    }
    pub(crate) fn create_public(&self, id: SharedKeyId) -> Result<OwnedKeyFile, KeyMaterialError> {
        self.create(id, KeyFileKind::Public)
    }
    fn create(&self, id: SharedKeyId, kind: KeyFileKind) -> Result<OwnedKeyFile, KeyMaterialError> {
        self.0.store.validate()?;
        let file = self.0.store.create(&kind.name(id), kind)?;
        self.wrap(file, id, kind)
    }
    pub(crate) fn open_owned(
        &self,
        id: SharedKeyId,
        kind: KeyFileKind,
    ) -> Result<Option<OwnedKeyFile>, KeyMaterialError> {
        self.0.store.validate()?;
        self.0
            .store
            .open_file(&kind.name(id), kind)?
            .map(|file| self.wrap(file, id, kind))
            .transpose()
    }
    fn wrap(
        &self,
        file: File,
        id: SharedKeyId,
        kind: KeyFileKind,
    ) -> Result<OwnedKeyFile, KeyMaterialError> {
        let identity = platform::identity(&file)?;
        Ok(OwnedKeyFile {
            file,
            identity,
            owner: self.0.clone(),
            id,
            kind,
        })
    }
    pub(crate) fn sync_directory(&self) -> Result<(), KeyMaterialError> {
        self.0.store.sync_directory()
    }
}

pub(crate) struct OwnedKeyFile {
    file: File,
    identity: FileIdentity,
    owner: Arc<LockedStore>,
    id: SharedKeyId,
    kind: KeyFileKind,
}
impl OwnedKeyFile {
    pub(crate) fn identity(&self) -> FileIdentity {
        self.identity.clone()
    }
    /// Write once, flush, and finalize to a read-only handle. Further writes fail.
    /// Freshness is captured only after the writing handle has closed, including
    /// on Windows where its close can finalize last-write/change timestamps.
    pub(crate) fn write_all_and_sync(&mut self, bytes: &[u8]) -> Result<(), KeyMaterialError> {
        use std::io::Write;
        self.owner.store.validate()?;
        self.owner
            .store
            .validate_file(&self.file, &self.kind.name(self.id), self.kind)?;
        if platform::identity(&self.file)? != self.identity {
            return Err(error(KeyMaterialErrorKind::SourceChanged));
        }
        self.file
            .write_all(bytes)
            .and_then(|()| self.file.sync_all())
            .map_err(|_| error(KeyMaterialErrorKind::StorageUnavailable))?;
        let retained = self
            .owner
            .store
            .open_file(&self.kind.name(self.id), self.kind)?
            .ok_or_else(|| error(KeyMaterialErrorKind::SourceChanged))?;
        // Both handles must identify the same object before the writer closes.
        // Keeping the validated reader open pins that object across finalization.
        if platform::identity(&retained)? != platform::identity(&self.file)? {
            return Err(error(KeyMaterialErrorKind::SourceChanged));
        }
        drop(std::mem::replace(&mut self.file, retained));
        self.identity = platform::identity(&self.file)?;
        Ok(())
    }
    /// Read through a bounded protected handle; never reopen a secret by raw path.
    pub(crate) fn read_secret(&mut self) -> Result<zeroize::Zeroizing<Vec<u8>>, KeyMaterialError> {
        use std::io::{Read, Seek, SeekFrom};
        const LIMIT: u64 = 16 * 1024;
        self.owner.store.validate()?;
        self.owner
            .store
            .validate_file(&self.file, &self.kind.name(self.id), self.kind)?;
        if platform::identity(&self.file)? != self.identity {
            return Err(error(KeyMaterialErrorKind::SourceChanged));
        }
        let size = self.file.metadata().map_err(io_error)?.len();
        if size > LIMIT {
            return Err(error(KeyMaterialErrorKind::InvalidGeneratedKey));
        }
        // Reserve the entire bound up front so concurrent growth cannot make
        // Vec reallocate and leave a previous secret allocation unerased.
        let mut bytes = zeroize::Zeroizing::new(Vec::with_capacity((LIMIT + 1) as usize));
        self.file.seek(SeekFrom::Start(0)).map_err(io_error)?;
        Read::by_ref(&mut self.file)
            .take(LIMIT + 1)
            .read_to_end(&mut bytes)
            .map_err(io_error)?;
        if bytes.len() as u64 > LIMIT || platform::identity(&self.file)? != self.identity {
            return Err(error(KeyMaterialErrorKind::SourceChanged));
        }
        self.owner
            .store
            .validate_file(&self.file, &self.kind.name(self.id), self.kind)?;
        Ok(bytes)
    }
    pub(crate) fn remove(self) -> Result<(), KeyMaterialError> {
        self.owner.store.validate()?;
        self.owner
            .store
            .validate_file(&self.file, &self.kind.name(self.id), self.kind)?;
        if platform::identity(&self.file)? != self.identity {
            return Err(error(KeyMaterialErrorKind::SourceChanged));
        }
        self.owner
            .store
            .remove(&self.file, &self.kind.name(self.id))
    }
}

/// Versioned metadata only; never hashes or contains material bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FileIdentity {
    platform: &'static str,
    fields: [u64; 8],
}
impl FileIdentity {
    pub(crate) fn encode(&self) -> String {
        let mut encoded = self.platform.to_owned();
        for field in self.fields {
            encoded.push(':');
            encoded.push_str(&format!("{field:016x}"));
        }
        encoded
    }
    pub(crate) fn decode(encoded: &str) -> Result<Self, KeyMaterialError> {
        let invalid = || error(KeyMaterialErrorKind::OwnershipUnverified);
        let mut parts = encoded.split(':');
        let platform = match parts.next() {
            Some("unix-v1") => "unix-v1",
            Some("windows-v1") => "windows-v1",
            _ => return Err(invalid()),
        };
        let mut fields = [0; 8];
        for field in &mut fields {
            let part = parts.next().ok_or_else(invalid)?;
            if part.len() != 16
                || !part
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                return Err(invalid());
            }
            *field = u64::from_str_radix(part, 16).map_err(|_| invalid())?;
        }
        if parts.next().is_some() {
            return Err(invalid());
        }
        Ok(Self { platform, fields })
    }
}

#[allow(dead_code, reason = "inspection consumer arrives in Task 5")]
pub(crate) fn observe_regular_source(path: &Path) -> Result<FileIdentity, KeyMaterialError> {
    platform::observe_regular_source(path).map_err(|mut e| {
        e.operation = KeyMaterialAction::Inspect;
        e
    })
}
fn error(kind: KeyMaterialErrorKind) -> KeyMaterialError {
    KeyMaterialError {
        operation: KeyMaterialAction::Generate,
        key_id: None,
        operation_id: None,
        kind,
    }
}
fn io_error(e: io::Error) -> KeyMaterialError {
    #[cfg(unix)]
    if e.raw_os_error() == Some(libc::ELOOP) {
        return error(KeyMaterialErrorKind::UnsafePath);
    }
    error(match e.kind() {
        io::ErrorKind::NotFound => KeyMaterialErrorKind::SourceMissing,
        io::ErrorKind::PermissionDenied => KeyMaterialErrorKind::ProtectionUnavailable,
        io::ErrorKind::AlreadyExists => KeyMaterialErrorKind::UnsafePath,
        _ => KeyMaterialErrorKind::StorageUnavailable,
    })
}

#[cfg(test)]
#[path = "storage/tests.rs"]
mod tests;
