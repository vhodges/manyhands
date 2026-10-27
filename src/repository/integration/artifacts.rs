//! Bounded Git effects. The adapter journals intent and retains the common-Git
//! lease throughout; these primitives do not claim transactional checkout.
use super::Error;
use crate::repository::native_resolution;
#[cfg(unix)]
use std::{
    ffi::CString,
    io::{Read, Seek, SeekFrom},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::fs::MetadataExt,
    },
};

#[cfg(any(unix, windows))]
pub(in super::super) struct IndexFileImage {
    pub(in super::super) bytes: Vec<u8>,
    pub(in super::super) device: u64,
    pub(in super::super) inode: u64,
}

#[cfg(unix)]
pub(in super::super) fn index_file_image_at(
    parent: &std::fs::File,
    name: &CString,
) -> Result<(std::fs::File, IndexFileImage), Error> {
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        )
    };
    if fd < 0 {
        return Err(Error::ExternalChange);
    }
    let mut file = unsafe { std::fs::File::from_raw_fd(fd) };
    let metadata = file.metadata().map_err(|_| Error::ExternalChange)?;
    if !metadata.is_file() || metadata.mode() & 0o111 != 0 {
        return Err(Error::ExternalChange);
    }
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|_| Error::ExternalChange)?;
    file.seek(SeekFrom::Start(0))
        .map_err(|_| Error::ExternalChange)?;
    Ok((
        file,
        IndexFileImage {
            bytes,
            device: metadata.dev(),
            inode: metadata.ino(),
        },
    ))
}

#[cfg(unix)]
pub(in super::super) fn index_leaf_present(
    parent: &std::fs::File,
    name: &CString,
) -> Result<bool, Error> {
    let mut metadata = std::mem::MaybeUninit::<libc::stat>::uninit();
    let result = unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            name.as_ptr(),
            metadata.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result == 0 {
        Ok(true)
    } else if std::io::Error::last_os_error().kind() == std::io::ErrorKind::NotFound {
        Ok(false)
    } else {
        Err(Error::ExternalChange)
    }
}

/// Pinned target gitdir. Callers supply only fixed, adapter-owned member names
/// and journaled digests, under their lease. This is not an inode-exchange API.
#[cfg(unix)]
pub(in super::super) struct MergeMetadataDirectory {
    parent: std::fs::File,
}

#[cfg(unix)]
impl MergeMetadataDirectory {
    pub(in super::super) fn open(repository: &git2::Repository) -> Result<Self, Error> {
        native_resolution::open_directory(repository.path())
            .map(|parent| Self { parent })
            .map_err(|_| Error::RecoveryRequired)
    }

    pub(in super::super) fn digest(&self, member: &str) -> Result<Option<[u8; 32]>, Error> {
        let name = CString::new(member).expect("fixed index name");
        if !index_leaf_present(&self.parent, &name).map_err(|_| Error::RecoveryRequired)? {
            return Ok(None);
        }
        let (file, image) =
            index_file_image_at(&self.parent, &name).map_err(|_| Error::RecoveryRequired)?;
        if file
            .metadata()
            .map_err(|_| Error::RecoveryRequired)?
            .nlink()
            != 1
        {
            return Err(Error::RecoveryRequired);
        }
        Ok(Some(*blake3::hash(&image.bytes).as_bytes()))
    }

    pub(in super::super) fn retire(&self, member: &str, expected: [u8; 32]) -> Result<(), Error> {
        if self.digest(member)? != Some(expected) {
            return Err(Error::RecoveryRequired);
        }
        let name = CString::new(member).expect("fixed index name");
        if unsafe { libc::unlinkat(self.parent.as_raw_fd(), name.as_ptr(), 0) } != 0 {
            return Err(Error::RecoveryRequired);
        }
        self.parent.sync_all().map_err(|_| Error::RecoveryRequired)
    }
}

#[cfg(windows)]
pub(in super::super) struct MergeMetadataDirectory {
    parent: native_resolution::Directory,
}

#[cfg(windows)]
impl MergeMetadataDirectory {
    pub(in super::super) fn open(repository: &git2::Repository) -> Result<Self, Error> {
        native_resolution::Directory::open(repository.path())
            .map(|parent| Self { parent })
            .map_err(|_| Error::RecoveryRequired)
    }
    pub(in super::super) fn digest(&self, member: &str) -> Result<Option<[u8; 32]>, Error> {
        Ok(self
            .parent
            .image(member)
            .map_err(|_| Error::RecoveryRequired)?
            .map(|image| *blake3::hash(&image.bytes).as_bytes()))
    }
    pub(in super::super) fn retire(&self, member: &str, expected: [u8; 32]) -> Result<(), Error> {
        let image = self
            .parent
            .image(member)
            .map_err(|_| Error::RecoveryRequired)?
            .ok_or(Error::RecoveryRequired)?;
        if *blake3::hash(&image.bytes).as_bytes() != expected {
            return Err(Error::RecoveryRequired);
        }
        self.parent
            .retire(member, image)
            .map_err(|_| Error::RecoveryRequired)
    }
}

#[cfg(not(any(unix, windows)))]
pub(in super::super) struct MergeMetadataDirectory;

#[cfg(not(any(unix, windows)))]
impl MergeMetadataDirectory {
    pub(in super::super) fn open(_: &git2::Repository) -> Result<Self, Error> {
        Err(Error::RecoveryRequired)
    }
    pub(in super::super) fn digest(&self, _: &str) -> Result<Option<[u8; 32]>, Error> {
        Err(Error::RecoveryRequired)
    }
    pub(in super::super) fn retire(&self, _: &str, _: [u8; 32]) -> Result<(), Error> {
        Err(Error::RecoveryRequired)
    }
}

/// Lock/compare before checkout, then commit the ref against OLD HEAD. `authorize`
/// runs before even a backend lock is created. `after_checkout` preserves the
/// adapter's existing fault/interleaving seam; it grants no new authority.
pub(in super::super) fn fast_forward<E: From<Error>>(
    repository: &git2::Repository,
    branch: &str,
    old: git2::Oid,
    new: git2::Oid,
    authorize: impl FnOnce() -> Result<(), E>,
    after_checkout: impl FnOnce(),
) -> Result<(), E> {
    authorize()?;
    let commit = repository
        .find_commit(new)
        .map_err(|_| Error::RecoveryRequired)?;
    let mut transaction = repository
        .transaction()
        .map_err(|_| Error::ExternalChange)?;
    transaction
        .lock_ref(branch)
        .map_err(|_| Error::ExternalChange)?;
    if repository.refname_to_id(branch).ok() != Some(old)
        || repository
            .find_reference("HEAD")
            .ok()
            .and_then(|r| r.symbolic_target().map(str::to_owned))
            .as_deref()
            != Some(branch)
    {
        return Err(Error::ExternalChange.into());
    }
    let mut checkout = git2::build::CheckoutBuilder::new();
    checkout.safe().overwrite_ignored(false);
    repository
        .checkout_tree(commit.as_object(), Some(&mut checkout))
        .map_err(|_| Error::RecoveryRequired)?;
    after_checkout();
    if repository.refname_to_id(branch).ok() != Some(old)
        || repository
            .find_reference("HEAD")
            .ok()
            .and_then(|r| r.symbolic_target().map(str::to_owned))
            .as_deref()
            != Some(branch)
    {
        return Err(Error::RecoveryRequired.into());
    }
    transaction
        .set_target(branch, new, None, "manyhands synchronization")
        .map_err(|_| Error::RecoveryRequired)?;
    transaction.commit().map_err(|_| Error::RecoveryRequired)?;
    Ok(())
}

/// Initial conflict installation is one coarse backend call. Intent and baseline
/// proof MUST precede this call; metadata ownership is captured immediately after
/// it, under the same lease. An ambiguous interrupted image is never auto-reset.
pub(in super::super) fn install_conflict(
    repository: &git2::Repository,
    incoming: git2::Oid,
) -> Result<(), Error> {
    let annotated = repository
        .find_annotated_commit(incoming)
        .map_err(|_| Error::RecoveryRequired)?;
    let mut checkout = git2::build::CheckoutBuilder::new();
    checkout.safe().overwrite_ignored(false);
    repository
        .merge(&[&annotated], None, Some(&mut checkout))
        .map_err(|_| Error::RecoveryRequired)
}

pub(in super::super) fn local_checkout<E: From<Error>>(
    repository: &git2::Repository,
    candidate: git2::Oid,
    authorize: impl FnOnce() -> Result<(), E>,
    mut progress: impl FnMut() -> Result<(), E>,
) -> Result<(), E> {
    authorize()?;
    let commit = repository
        .find_commit(candidate)
        .map_err(|_| Error::RecoveryRequired)?;
    let mut interrupted = None;
    let result = {
        let mut checkout = git2::build::CheckoutBuilder::new();
        checkout
            .safe()
            .overwrite_ignored(false)
            .progress(|path, completed, _| {
                if path.is_some() && completed > 0 && interrupted.is_none() {
                    interrupted = progress().err();
                }
            });
        repository.checkout_tree(commit.as_object(), Some(&mut checkout))
    };
    if let Some(error) = interrupted {
        return Err(error);
    }
    result.map_err(|_| Error::RecoveryRequired.into())
}

pub(in super::super) fn local_reference<E: From<Error>>(
    repository: &git2::Repository,
    transition: (&str, git2::Oid, git2::Oid),
    signature: &git2::Signature<'_>,
    message: &str,
    mut authorize: impl FnMut() -> Result<(), E>,
    before_commit: impl FnOnce() -> Result<(), E>,
) -> Result<(), E> {
    authorize()?;
    let (branch, old, new) = transition;
    let mut tx = repository
        .transaction()
        .map_err(|_| Error::RecoveryRequired)?;
    tx.lock_ref(branch).map_err(|_| Error::RecoveryRequired)?;
    if repository.refname_to_id(branch).ok() != Some(old)
        || repository
            .head()
            .ok()
            .and_then(|h| h.name().map(str::to_owned))
            .as_deref()
            != Some(branch)
    {
        return Err(Error::ExternalChange.into());
    }
    authorize()?;
    tx.set_target(branch, new, Some(signature), message)
        .map_err(|_| Error::RecoveryRequired)?;
    // A process exit here leaves libgit2's actual lock file, without Drop.
    before_commit()?;
    tx.commit().map_err(|_| Error::RecoveryRequired.into())
}
