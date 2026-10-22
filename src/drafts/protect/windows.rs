//! Decision 3: `drafts/` inherits the ACL of `%LOCALAPPDATA%` and no DACL is
//! written. Operations are path based. The folder must be no reparse point,
//! and its owner must be the process token's user or its default owner
//! (ruling R4). Call patterns follow `repository/keys/storage/windows.rs`.

use super::{DraftError, DraftErrorKind, Folder};
use crate::drafts::UnreadableReason;
use std::{
    ffi::c_void,
    fs::{File, OpenOptions},
    io,
    mem::size_of,
    os::windows::{
        ffi::OsStrExt,
        fs::{MetadataExt, OpenOptionsExt},
        io::{AsRawHandle, FromRawHandle},
    },
    path::Path,
    ptr::{null, null_mut},
};
use windows_sys::Win32::{
    Foundation::*,
    Security::{Authorization::*, *},
    Storage::FileSystem::*,
    System::Threading::{GetCurrentProcess, OpenProcessToken},
};

const DRAFTS: &str = "drafts";

impl Folder {
    pub(crate) fn open(data: &Path) -> Result<Folder, DraftError> {
        let folder = Folder {
            path: data.join(DRAFTS),
        };
        open_folder(data, &folder.path)?;
        Ok(folder)
    }

    /// Makes or checks `drafts/` again, after it was removed.
    pub(crate) fn reopen(&mut self) -> Result<(), DraftError> {
        open_folder(self.data()?, &self.path)
    }

    /// A new file, made exclusively. If `drafts/` has been removed, it is
    /// made again once.
    pub(crate) fn create_new(&mut self, name: &str) -> Result<File, DraftError> {
        check_folder(&self.path)?;
        let path = self.path.join(name);
        let create = || {
            OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .open(&path)
        };
        match create() {
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                self.reopen()?;
                Ok(create()?)
            }
            result => Ok(result?),
        }
    }

    /// An existing draft file, for reading: a regular file, no reparse point.
    /// Draft files inherit the folder's ACL; their owner is not checked.
    // Used by the draft store (Task 3).
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn open_file(&self, name: &str) -> Result<File, UnreadableReason> {
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS)
            .open(self.path.join(name))
            .map_err(|_| UnreadableReason::Access)?;
        let metadata = file.metadata().map_err(|_| UnreadableReason::Access)?;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 || !metadata.is_file() {
            return Err(UnreadableReason::NotAFile);
        }
        Ok(file)
    }

    // Used by the draft store (Task 3).
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn rename(&self, from: &str, to: &str) -> Result<(), DraftError> {
        Ok(std::fs::rename(self.path.join(from), self.path.join(to))?)
    }

    // Used by the draft store (Task 3).
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn remove(&self, name: &str) -> Result<(), DraftError> {
        Ok(std::fs::remove_file(self.path.join(name))?)
    }

    /// Win32 has no unprivileged folder sync; files are flushed one by one.
    // Used by the draft store (Task 3).
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn sync(&self) -> Result<(), DraftError> {
        Ok(())
    }

    // Used by the draft store (Task 3).
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn names(&self) -> Result<Vec<String>, DraftError> {
        let mut names = Vec::new();
        for entry in std::fs::read_dir(&self.path)? {
            if let Ok(name) = entry?.file_name().into_string() {
                names.push(name);
            }
        }
        Ok(names)
    }
}

/// Makes `drafts/` if absent and checks it.
fn open_folder(data: &Path, path: &Path) -> Result<(), DraftError> {
    std::fs::create_dir_all(data)?;
    if let Err(error) = std::fs::create_dir(path)
        && error.kind() != io::ErrorKind::AlreadyExists
    {
        return Err(error.into());
    }
    check_folder(path)
}

/// `drafts/` must be a folder, no reparse point, owned by this user.
fn check_folder(path: &Path) -> Result<(), DraftError> {
    let refused = || DraftError::new(DraftErrorKind::ProtectionUnavailable);
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 || !metadata.is_dir() {
        return Err(refused());
    }
    let folder = open_directory(path)?;
    let metadata = folder.metadata()?;
    if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 || !metadata.is_dir() {
        return Err(refused());
    }
    if !owned_by_us(&folder)? {
        return Err(refused());
    }
    Ok(())
}

fn wide(path: &Path) -> io::Result<Vec<u16>> {
    let mut value: Vec<u16> = path.as_os_str().encode_wide().collect();
    if value.contains(&0) {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    value.push(0);
    Ok(value)
}

/// A handle on the folder itself, never through a reparse point.
fn open_directory(path: &Path) -> io::Result<File> {
    let path = wide(path)?;
    let handle = unsafe {
        CreateFileW(
            path.as_ptr(),
            FILE_READ_ATTRIBUTES | READ_CONTROL,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            null(),
            OPEN_EXISTING,
            FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS,
            null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { File::from_raw_handle(handle) })
}

/// Whether the folder's owner is the token's user or its default owner.
fn owned_by_us(folder: &File) -> Result<bool, DraftError> {
    let refused = || DraftError::new(DraftErrorKind::ProtectionUnavailable);
    let mut owner = null_mut();
    let mut descriptor = null_mut();
    if unsafe {
        GetSecurityInfo(
            folder.as_raw_handle(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION,
            &mut owner,
            null_mut(),
            null_mut(),
            null_mut(),
            &mut descriptor,
        )
    } != ERROR_SUCCESS
    {
        return Err(refused());
    }
    // `owner` points into the descriptor, freed when this drops.
    let _descriptor = LocalMemory(descriptor);
    if owner.is_null() {
        return Err(refused());
    }
    let mut handle = null_mut();
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut handle) } == 0 {
        return Err(refused());
    }
    let token = Token(handle);
    let user = token_information(&token, TokenUser)?;
    let user = unsafe { (*(user.as_ptr().cast::<TOKEN_USER>())).User.Sid };
    let default_owner = token_information(&token, TokenOwner)?;
    let default_owner = unsafe { (*(default_owner.as_ptr().cast::<TOKEN_OWNER>())).Owner };
    Ok(unsafe { EqualSid(owner, user) } != 0 || unsafe { EqualSid(owner, default_owner) } != 0)
}

/// One class of the token's information, in a buffer aligned for it.
fn token_information(
    token: &Token,
    class: TOKEN_INFORMATION_CLASS,
) -> Result<Vec<usize>, DraftError> {
    let refused = || DraftError::new(DraftErrorKind::ProtectionUnavailable);
    let mut size = 0;
    unsafe {
        GetTokenInformation(token.0, class, null_mut(), 0, &mut size);
    }
    if size == 0 {
        return Err(refused());
    }
    let mut buffer = vec![0usize; (size as usize).div_ceil(size_of::<usize>())];
    if unsafe { GetTokenInformation(token.0, class, buffer.as_mut_ptr().cast(), size, &mut size) }
        == 0
    {
        return Err(refused());
    }
    Ok(buffer)
}

struct LocalMemory(*mut c_void);
impl Drop for LocalMemory {
    fn drop(&mut self) {
        unsafe {
            LocalFree(self.0);
        }
    }
}

struct Token(HANDLE);
impl Drop for Token {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
