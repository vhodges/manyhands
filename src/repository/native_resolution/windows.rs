//! Retained Win32 namespace primitives for cooperative resolution writers.
//!
//! All ancestor handles deny delete sharing; regular leaves permit it so stock
//! libgit2 and native replacement can operate. No reparse point is followed.
//! Files use FlushFileBuffers (`File::sync_all`); Win32 has no unprivileged
//! directory-fsync equivalent, so this is not a power-loss durability promise.
//! Read-only Git storage may deny writable flush handles. Known access/unsupported
//! flush limits are best-effort barriers, not functional refusals or claims of
//! durability; unexpected errors still stop resolution.

use std::{
    alloc::{Layout, alloc_zeroed, dealloc},
    ffi::OsStr,
    fs::File,
    io::{self, Read, Seek, SeekFrom, Write},
    mem::{align_of, offset_of, size_of, zeroed},
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle},
    },
    path::{Component, Path, PathBuf, Prefix},
    ptr::{null, null_mut},
};
use windows_sys::Win32::{
    Foundation::*,
    Storage::FileSystem::*,
    System::WindowsProgramming::{
        FILE_RENAME_FLAG_POSIX_SEMANTICS, FILE_RENAME_FLAG_REPLACE_IF_EXISTS,
    },
};

#[path = "windows_path.rs"]
mod windows_path;
use windows_path::validate_component;
#[cfg(test)]
#[path = "windows_tests.rs"]
mod windows_tests;

fn changed() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "resolution namespace or image changed",
    )
}

fn wide(path: &Path) -> io::Result<Vec<u16>> {
    let mut value: Vec<_> = path.as_os_str().encode_wide().collect();
    if value.contains(&0) {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    value.push(0);
    Ok(value)
}

fn file_info(file: &File) -> io::Result<BY_HANDLE_FILE_INFORMATION> {
    let mut info = unsafe { zeroed() };
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(info)
}

fn check_kind(file: &File, directory: bool) -> io::Result<()> {
    let info = file_info(file)?;
    if unsafe { GetFileType(file.as_raw_handle()) } != FILE_TYPE_DISK
        || info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
        || (info.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0) != directory
    {
        return Err(changed());
    }
    Ok(())
}

/// Existing journal device/inode fields can represent the volume serial and
/// Win32 64-bit file index respectively, without changing the state schema.
pub(super) fn identity(file: &File) -> io::Result<[u64; 2]> {
    let info = file_info(file)?;
    Ok([
        u64::from(info.dwVolumeSerialNumber),
        (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow),
    ])
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Stamp {
    pub(super) identity: [u64; 2],
    pub(super) size: u64,
    pub(super) created: i64,
    pub(super) modified: i64,
    pub(super) changed: i64,
    pub(super) attributes: u32,
}

pub(super) fn stamp(file: &File) -> io::Result<Stamp> {
    check_kind(file, false)?;
    let info = file_info(file)?;
    let mut basic: FILE_BASIC_INFO = unsafe { zeroed() };
    if unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FileBasicInfo,
            (&mut basic as *mut FILE_BASIC_INFO).cast(),
            size_of::<FILE_BASIC_INFO>() as u32,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(Stamp {
        identity: identity(file)?,
        size: (u64::from(info.nFileSizeHigh) << 32) | u64::from(info.nFileSizeLow),
        created: basic.CreationTime,
        modified: basic.LastWriteTime,
        changed: basic.ChangeTime,
        attributes: basic.FileAttributes,
    })
}

fn open(path: &Path, directory: bool, create: bool, write: bool) -> io::Result<File> {
    let access = if directory {
        FILE_READ_ATTRIBUTES
    } else {
        GENERIC_READ | if write { GENERIC_WRITE } else { 0 }
    };
    // Ordinary images must coexist with stock libgit2 readers, whose sharing
    // mask omits FILE_SHARE_DELETE. Allow delete sharing without requesting it.
    open_with_access(path, directory, create, access)
}

fn open_retirement_target(path: &Path) -> io::Result<File> {
    open_with_access(path, false, false, GENERIC_READ | DELETE)
}

fn open_with_access(path: &Path, directory: bool, create: bool, access: u32) -> io::Result<File> {
    let path = wide(path)?;
    let handle = unsafe {
        CreateFileW(
            path.as_ptr(),
            access,
            FILE_SHARE_READ | FILE_SHARE_WRITE | if directory { 0 } else { FILE_SHARE_DELETE },
            null(),
            if create { CREATE_NEW } else { OPEN_EXISTING },
            FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS,
            null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    let file = unsafe { File::from_raw_handle(handle) };
    check_kind(&file, directory)?;
    Ok(file)
}

struct Ancestor {
    path: PathBuf,
    file: File,
    identity: [u64; 2],
}

pub(super) struct Directory {
    path: PathBuf,
    ancestors: Vec<Ancestor>,
}

impl Directory {
    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    pub(super) fn identity(&self) -> io::Result<[u64; 2]> {
        self.revalidate()?;
        Ok(self.ancestors.last().ok_or_else(changed)?.identity)
    }

    pub(super) fn child(&self, name: &str, create: bool) -> io::Result<Self> {
        self.revalidate()?;
        let path = self.leaf(name)?;
        if create {
            match std::fs::create_dir(&path) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error),
            }
        }
        let child = Self::open(&path)?;
        self.revalidate()?;
        Ok(child)
    }

    pub(super) fn open(path: &Path) -> io::Result<Self> {
        if !path.is_absolute() {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let mut components = path.components();
        let Some(Component::Prefix(prefix)) = components.next() else {
            return Err(io::ErrorKind::InvalidInput.into());
        };
        match prefix.kind() {
            Prefix::Disk(_) | Prefix::VerbatimDisk(_) => {}
            Prefix::UNC(server, share) | Prefix::VerbatimUNC(server, share) => {
                validate_component(server)?;
                validate_component(share)?;
            }
            _ => return Err(io::ErrorKind::InvalidInput.into()),
        }
        if components.next() != Some(Component::RootDir) {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let mut directory = Self {
            path: PathBuf::from(prefix.as_os_str()),
            ancestors: Vec::new(),
        };
        directory.path.push(Path::new("\\"));
        directory.pin()?;
        for component in components {
            let Component::Normal(name) = component else {
                return Err(io::ErrorKind::InvalidInput.into());
            };
            directory.descend(name, false)?;
        }
        Ok(directory)
    }

    fn pin(&mut self) -> io::Result<()> {
        let file = open(&self.path, true, false, false)?;
        self.ancestors.push(Ancestor {
            path: self.path.clone(),
            identity: identity(&file)?,
            file,
        });
        Ok(())
    }

    fn descend(&mut self, name: &OsStr, create: bool) -> io::Result<()> {
        validate_component(name)?;
        self.path.push(name);
        if create {
            match std::fs::create_dir(&self.path) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error),
            }
        }
        self.pin()
    }

    pub(super) fn revalidate(&self) -> io::Result<()> {
        for ancestor in &self.ancestors {
            check_kind(&ancestor.file, true)?;
            if identity(&ancestor.file)? != ancestor.identity
                || identity(&open(&ancestor.path, true, false, false)?)? != ancestor.identity
            {
                return Err(changed());
            }
        }
        Ok(())
    }

    fn leaf(&self, name: &str) -> io::Result<PathBuf> {
        validate_component(OsStr::new(name))?;
        Ok(self.path.join(name))
    }

    pub(super) fn image(&self, name: &str) -> io::Result<Option<Image>> {
        self.revalidate()?;
        let path = self.leaf(name)?;
        let file = match open(&path, false, false, false) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        let image = Image::read(file)?;
        if stamp(&open(&path, false, false, false)?)? != image.stamp {
            return Err(changed());
        }
        self.revalidate()?;
        Ok(Some(image))
    }

    pub(super) fn file(&self, name: &str) -> io::Result<Option<File>> {
        self.revalidate()?;
        match open(&self.leaf(name)?, false, false, false) {
            Ok(file) => {
                self.revalidate()?;
                Ok(Some(file))
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    /// FlushFileBuffers requires a writable handle. Existing read-only Git
    /// storage is still observable; only documented access/storage limitations
    /// weaken this barrier. No error from namespace validation is suppressed.
    pub(super) fn flush(&self, name: &str) -> io::Result<()> {
        self.revalidate()?;
        let file = match open(&self.leaf(name)?, false, false, true) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) if error.raw_os_error() == Some(ERROR_ACCESS_DENIED as i32) => {
                self.image(name)?.ok_or_else(changed)?;
                return self.revalidate();
            }
            Err(error) => return Err(error),
        };
        match file.sync_all() {
            Ok(()) => {}
            Err(error) if matches!(error.raw_os_error(), Some(code) if code == ERROR_INVALID_FUNCTION as i32 || code == ERROR_NOT_SUPPORTED as i32) =>
                {}
            Err(error) => return Err(error),
        }
        self.revalidate()
    }

    pub(super) fn create_image(&self, name: &str, bytes: &[u8]) -> io::Result<Image> {
        self.revalidate()?;
        let mut file = open(&self.leaf(name)?, false, true, true)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        let image = Image::read(file)?;
        if image.bytes != bytes {
            return Err(changed());
        }
        self.matches(name, &image)?;
        Ok(image)
    }

    pub(super) fn matches(&self, name: &str, image: &Image) -> io::Result<()> {
        if stamp(&image.file)? != image.stamp
            || self
                .image(name)?
                .is_none_or(|current| current.stamp != image.stamp || current.bytes != image.bytes)
        {
            return Err(changed());
        }
        Ok(())
    }

    /// Absent-only publication; callers must journal durable proof first. Multiple
    /// links are intentional ownership anchors, not a reason to reject a file.
    pub(super) fn publish_anchor(
        &self,
        source: &str,
        destination: &Self,
        target: &str,
        image: &Image,
    ) -> io::Result<()> {
        self.matches(source, image)?;
        destination.revalidate()?;
        let source_path = wide(&self.leaf(source)?)?;
        let target_path = destination.leaf(target)?;
        let target = wide(&target_path)?;
        if unsafe { CreateHardLinkW(target.as_ptr(), source_path.as_ptr(), null()) } == 0 {
            return Err(io::Error::last_os_error());
        }
        // Creating a link can change ChangeTime. Refresh and compare identity and
        // bytes instead of confusing a valid extra anchor with content mutation.
        let linked = Image::read(open(&target_path, false, false, false)?)?;
        let source_image = self.image(source)?.ok_or_else(changed)?;
        if linked.stamp.identity != image.stamp.identity
            || linked.bytes != image.bytes
            || source_image.stamp.identity != image.stamp.identity
            || source_image.bytes != image.bytes
            || file_info(&linked.file)?.nNumberOfLinks < 2
            || identity(&image.file)? != image.stamp.identity
        {
            return Err(changed());
        }
        self.revalidate()?;
        destination.revalidate()
    }

    /// Ownership proof may have been opened through a different hard-link anchor.
    /// Delete only through a freshly retained handle opened by the requested name,
    /// then close both handles before proving absence. Preserve changed pathnames.
    pub(super) fn retire(&self, name: &str, image: Image) -> io::Result<()> {
        self.revalidate()?;
        let target = Image::read(open_retirement_target(&self.leaf(name)?)?)?;
        if stamp(&image.file)? != image.stamp
            || target.stamp != image.stamp
            || target.bytes != image.bytes
        {
            return Err(changed());
        }
        self.matches(name, &target)?;
        let disposition = FILE_DISPOSITION_INFO { DeleteFile: true };
        if unsafe {
            SetFileInformationByHandle(
                target.file.as_raw_handle(),
                FileDispositionInfo,
                (&disposition as *const FILE_DISPOSITION_INFO).cast(),
                size_of::<FILE_DISPOSITION_INFO>() as u32,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        drop(target);
        drop(image);
        self.revalidate()?;
        if self.image(name)?.is_some() {
            return Err(changed());
        }
        Ok(())
    }

    /// Install an already journaled private output through a disposable hard-link
    /// role. Its durable source anchor survives the same-volume native rename.
    pub(super) fn install_anchor(
        &self,
        source: &str,
        destination: &Self,
        target: &str,
        prepared: &Image,
        original: &Image,
    ) -> io::Result<()> {
        self.rename_image(source, destination, target, prepared, Some(original))?;
        destination.flush(target)?;
        self.revalidate()?;
        destination.revalidate()
    }

    /// Rename the requested source role, never the handle supplied as ownership
    /// proof: that handle may have been opened through a durable hard-link anchor.
    /// Retained parent pins and pre/post proofs exclude cooperative substitution;
    /// this is not namespace CAS against arbitrary concurrent writers.
    fn rename_image(
        &self,
        source: &str,
        destination: &Self,
        target: &str,
        prepared: &Image,
        original: Option<&Image>,
    ) -> io::Result<()> {
        if self.identity()?[0] != destination.identity()?[0] {
            return Err(changed());
        }
        let source_image = Image::read(open_with_access(
            &self.leaf(source)?,
            false,
            false,
            GENERIC_READ | DELETE,
        )?)?;
        if source_image.stamp != prepared.stamp || source_image.bytes != prepared.bytes {
            return Err(changed());
        }
        let target_path = wide(&destination.leaf(target)?)?;
        // Follow std's Windows rename buffer construction: actual trailing-field
        // offset, native struct alignment, UTF-16 byte length excluding the NUL.
        // Full absolute paths have already passed component and NUL validation.
        let name_bytes = (target_path.len() - 1)
            .checked_mul(size_of::<u16>())
            .and_then(|length| u32::try_from(length).ok())
            .ok_or(io::ErrorKind::InvalidInput)?;
        let buffer_size = offset_of!(FILE_RENAME_INFO, FileName)
            .checked_add(name_bytes as usize)
            .and_then(|length| length.checked_add(size_of::<u16>()))
            .ok_or(io::ErrorKind::InvalidInput)?;
        let buffer_len = u32::try_from(buffer_size).map_err(|_| io::ErrorKind::InvalidInput)?;
        let layout = Layout::from_size_align(
            buffer_size.max(size_of::<FILE_RENAME_INFO>()),
            align_of::<FILE_RENAME_INFO>(),
        )
        .map_err(|_| io::ErrorKind::InvalidInput)?;
        self.matches(source, prepared)?;
        self.matches(source, &source_image)?;
        match original {
            Some(image) => destination.matches(target, image)?,
            None if destination.image(target)?.is_some() => return Err(changed()),
            None => {}
        }
        self.revalidate()?;
        destination.revalidate()?;
        // SAFETY: layout provides FILE_RENAME_INFO alignment and space for the
        // complete header plus the UTF-16 name and NUL. Raw field pointers retain
        // the allocation's provenance for the variable-length trailing field.
        let result = unsafe {
            let info = alloc_zeroed(layout).cast::<FILE_RENAME_INFO>();
            if info.is_null() {
                return Err(io::ErrorKind::OutOfMemory.into());
            }
            (&raw mut (*info).Anonymous).write(FILE_RENAME_INFO_0 {
                Flags: if original.is_some() {
                    FILE_RENAME_FLAG_REPLACE_IF_EXISTS | FILE_RENAME_FLAG_POSIX_SEMANTICS
                } else {
                    0
                },
            });
            (&raw mut (*info).RootDirectory).write(null_mut());
            (&raw mut (*info).FileNameLength).write(name_bytes);
            target_path.as_ptr().copy_to_nonoverlapping(
                (&raw mut (*info).FileName).cast::<u16>(),
                target_path.len(),
            );
            let status = SetFileInformationByHandle(
                source_image.file.as_raw_handle(),
                FileRenameInfoEx,
                info.cast(),
                buffer_len,
            );
            // Capture the rename error before deallocation; no classic fallback,
            // readonly override, or suppression of ACL/unsupported errors.
            let result = if status == 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            };
            dealloc(info.cast(), layout);
            result
        };
        result?;
        let installed = destination.image(target)?.ok_or_else(changed)?;
        let retained = Image::read(prepared.file.try_clone()?)?;
        if installed.stamp.identity != prepared.stamp.identity
            || installed.bytes != prepared.bytes
            || retained.stamp.identity != prepared.stamp.identity
            || retained.bytes != prepared.bytes
            || self.image(source)?.is_some()
        {
            return Err(changed());
        }
        if let Some(original) = original {
            let retained = Image::read(original.file.try_clone()?)?;
            if retained.stamp.identity != original.stamp.identity
                || retained.bytes != original.bytes
            {
                return Err(changed());
            }
        }
        self.revalidate()?;
        destination.revalidate()
    }
}

pub(super) struct Image {
    pub(super) file: File,
    pub(super) bytes: Vec<u8>,
    pub(super) stamp: Stamp,
}

impl Image {
    fn read(mut file: File) -> io::Result<Self> {
        let before = stamp(&file)?;
        file.seek(SeekFrom::Start(0))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        if before != stamp(&file)? || bytes.len() as u64 != before.size {
            return Err(changed());
        }
        file.seek(SeekFrom::Start(0))?;
        Ok(Self {
            file,
            bytes,
            stamp: before,
        })
    }
}

pub(super) fn owned_parent(
    root: &Path,
    relative: &Path,
    create: bool,
) -> io::Result<(Directory, String)> {
    // Validate the entire role before performing even a directory creation.
    let names = relative
        .components()
        .map(|component| {
            let Component::Normal(name) = component else {
                return Err(io::ErrorKind::InvalidInput.into());
            };
            validate_component(name)?;
            name.to_str()
                .map(str::to_owned)
                .ok_or_else(|| io::ErrorKind::InvalidInput.into())
        })
        .collect::<io::Result<Vec<_>>>()?;
    let (leaf, parents) = names.split_last().ok_or(io::ErrorKind::InvalidInput)?;
    let mut parent = Directory::open(root)?;
    for name in parents {
        parent.descend(OsStr::new(name), create)?;
    }
    Ok((parent, leaf.clone()))
}

pub(super) fn read_owned(root: &Path, relative: &Path) -> io::Result<Option<Vec<u8>>> {
    let (parent, leaf) = match owned_parent(root, relative, false) {
        Ok(value) => value,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    Ok(parent.image(&leaf)?.map(|image| image.bytes))
}

/// Same-volume private source, pre/post handle proof, and native replacement.
/// This is cooperative-writer exclusion, not arbitrary-writer inode CAS.
pub(super) fn replace_owned(
    root: &Path,
    relative: &Path,
    private: &Path,
    bytes: &[u8],
    expected: [u8; 32],
) -> io::Result<()> {
    let (parent, leaf) = owned_parent(root, relative, true)?;
    let original = parent.image(&leaf)?;
    if super::owned_prewrite_digest(original.as_ref().map(|image| image.bytes.as_slice()))
        != expected
    {
        return Err(changed());
    }
    let staging = Directory::open(private)?;
    // Establish the volume from both retained parents before writing staging
    // bytes; rename_image repeats this proof before the namespace effect.
    if parent.identity()?[0] != staging.identity()?[0] {
        return Err(changed());
    }
    let temp = format!(".manyhands-write-{}", ulid::Ulid::new());
    let written = staging.create_image(&temp, bytes)?;
    staging.rename_image(&temp, &parent, &leaf, &written, original.as_ref())?;
    written.file.sync_all()?;
    parent.revalidate()?;
    staging.revalidate()
}
