use super::{FileIdentity, KeyFileKind, KeyMaterialError, KeyMaterialErrorKind, error, io_error};
use std::{
    ffi::c_void,
    fs::File,
    mem::{size_of, zeroed},
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle},
    },
    path::{Path, PathBuf},
    ptr::{null, null_mut},
};
use windows_sys::Win32::{
    Foundation::*,
    Security::{Authorization::*, *},
    Storage::FileSystem::*,
    System::Threading::{GetCurrentProcess, OpenProcessToken},
};

pub(super) struct Store {
    _home: File,
    ssh: File,
    directory: File,
    path: PathBuf,
}
impl Store {
    pub(super) fn open(home: &Path) -> Result<Self, KeyMaterialError> {
        let anchor = open_handle(home, false, true, false, None).map_err(io_error)?;
        check_attributes(&anchor, true)?;
        let security = owner_security()?;
        let ssh_path = home.join(".ssh");
        let ssh = directory(&ssh_path, &security)?;
        let path = ssh_path.join("manyhands");
        let directory = directory(&path, &security)?;
        Ok(Self {
            _home: anchor,
            ssh,
            directory,
            path,
        })
    }
    pub(super) fn validate(&self) -> Result<(), KeyMaterialError> {
        check_attributes(&self.ssh, true)?;
        check_attributes(&self.directory, true)?;
        verify_security(&self.ssh)?;
        verify_security(&self.directory)
    }
    pub(super) fn open_lock(&self) -> Result<File, KeyMaterialError> {
        let path = self.path.join(".lock");
        let security = owner_security()?;
        let file = match open_handle(&path, true, false, true, Some(&security)) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                open_handle(&path, false, false, true, None).map_err(io_error)?
            }
            Err(e) => return Err(io_error(e)),
        };
        validate_regular(&file)?;
        Ok(file)
    }
    pub(super) fn create(&self, name: &str, _kind: KeyFileKind) -> Result<File, KeyMaterialError> {
        let security = owner_security()?;
        let file = open_handle(&self.path.join(name), true, false, true, Some(&security))
            .map_err(io_error)?;
        validate_regular(&file)?;
        Ok(file)
    }
    pub(super) fn open_file(
        &self,
        name: &str,
        _kind: KeyFileKind,
    ) -> Result<Option<File>, KeyMaterialError> {
        let file = match open_handle(&self.path.join(name), false, false, false, None) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(io_error(e)),
        };
        validate_regular(&file)?;
        Ok(Some(file))
    }
    pub(super) fn validate_file(
        &self,
        file: &File,
        name: &str,
        kind: KeyFileKind,
    ) -> Result<(), KeyMaterialError> {
        validate_regular(file)?;
        let current = self
            .open_file(name, kind)?
            .ok_or_else(|| error(KeyMaterialErrorKind::SourceChanged))?;
        if identity(&current)? != identity(file)? {
            return Err(error(KeyMaterialErrorKind::SourceChanged));
        }
        Ok(())
    }
    pub(super) fn remove(&self, file: &File, _name: &str) -> Result<(), KeyMaterialError> {
        let disposition = FILE_DISPOSITION_INFO { DeleteFile: true };
        if unsafe {
            SetFileInformationByHandle(
                file.as_raw_handle(),
                FileDispositionInfo,
                (&disposition as *const FILE_DISPOSITION_INFO).cast(),
                size_of::<FILE_DISPOSITION_INFO>() as u32,
            )
        } == 0
        {
            return Err(io_error(std::io::Error::last_os_error()));
        }
        Ok(())
    }
    pub(super) fn sync_directory(&self) -> Result<(), KeyMaterialError> {
        // Win32 provides no unprivileged directory fsync equivalent. Individual
        // material files use FlushFileBuffers via File::sync_all; directory-entry
        // durability across power loss is not guaranteed on this platform.
        self.validate()
    }
}

fn wide(path: &Path) -> std::io::Result<Vec<u16>> {
    let mut value: Vec<u16> = path.as_os_str().encode_wide().collect();
    if value.contains(&0) {
        return Err(std::io::ErrorKind::InvalidInput.into());
    }
    value.push(0);
    Ok(value)
}
fn open_handle(
    path: &Path,
    create: bool,
    directory: bool,
    write: bool,
    security: Option<&LocalMemory>,
) -> std::io::Result<File> {
    let path = wide(path)?;
    let attributes = security.map(|s| SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: s.0,
        bInheritHandle: 0,
    });
    // Directory handles deny delete sharing, pinning each ancestor while a
    // descendant operation uses its path. Files are deleted by their own handle.
    let access = if directory {
        FILE_READ_ATTRIBUTES | READ_CONTROL
    } else {
        GENERIC_READ | DELETE | if write { GENERIC_WRITE } else { 0 }
    };
    let sharing =
        FILE_SHARE_READ | FILE_SHARE_WRITE | if directory { 0 } else { FILE_SHARE_DELETE };
    let handle = unsafe {
        CreateFileW(
            path.as_ptr(),
            access,
            sharing,
            attributes.as_ref().map_or(null(), |a| a),
            if create { CREATE_NEW } else { OPEN_EXISTING },
            FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS,
            null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(std::io::Error::last_os_error());
    }
    Ok(unsafe { File::from_raw_handle(handle) })
}
fn directory(path: &Path, security: &LocalMemory) -> Result<File, KeyMaterialError> {
    let wide_path = wide(path).map_err(io_error)?;
    let attributes = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: security.0,
        bInheritHandle: 0,
    };
    if unsafe { CreateDirectoryW(wide_path.as_ptr(), &attributes) } == 0 {
        let e = std::io::Error::last_os_error();
        if e.kind() != std::io::ErrorKind::AlreadyExists {
            return Err(io_error(e));
        }
    }
    let file = open_handle(path, false, true, false, None).map_err(io_error)?;
    check_attributes(&file, true)?;
    verify_security(&file)?;
    Ok(file)
}
fn file_info(file: &File) -> Result<BY_HANDLE_FILE_INFORMATION, KeyMaterialError> {
    let mut info = unsafe { zeroed() };
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
        return Err(error(KeyMaterialErrorKind::ProtectionUnavailable));
    }
    Ok(info)
}
fn check_attributes(file: &File, directory: bool) -> Result<(), KeyMaterialError> {
    let info = file_info(file)?;
    reject_reparse(info.dwFileAttributes)?;
    if unsafe { GetFileType(file.as_raw_handle()) } != FILE_TYPE_DISK
        || (info.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0) != directory
    {
        return Err(error(KeyMaterialErrorKind::NotRegularFile));
    }
    Ok(())
}
fn reject_reparse(attributes: u32) -> Result<(), KeyMaterialError> {
    if attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(error(KeyMaterialErrorKind::UnsafePath));
    }
    Ok(())
}
fn validate_regular(file: &File) -> Result<(), KeyMaterialError> {
    check_attributes(file, false)?;
    if file_info(file)?.nNumberOfLinks != 1 {
        return Err(error(KeyMaterialErrorKind::ProtectionUnavailable));
    }
    verify_security(file)
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
struct CurrentUser {
    buffer: Vec<usize>,
}
impl CurrentUser {
    fn get() -> Result<Self, KeyMaterialError> {
        let mut handle = null_mut();
        if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut handle) } == 0 {
            return Err(error(KeyMaterialErrorKind::ProtectionUnavailable));
        }
        let token = Token(handle);
        let mut size = 0;
        unsafe {
            GetTokenInformation(token.0, TokenUser, null_mut(), 0, &mut size);
        }
        if size == 0 {
            return Err(error(KeyMaterialErrorKind::ProtectionUnavailable));
        }
        let mut buffer = vec![0usize; (size as usize).div_ceil(size_of::<usize>())];
        if unsafe {
            GetTokenInformation(
                token.0,
                TokenUser,
                buffer.as_mut_ptr().cast(),
                size,
                &mut size,
            )
        } == 0
        {
            return Err(error(KeyMaterialErrorKind::ProtectionUnavailable));
        }
        Ok(Self { buffer })
    }
    fn sid(&self) -> PSID {
        unsafe { (*(self.buffer.as_ptr().cast::<TOKEN_USER>())).User.Sid }
    }
    fn sid_string(&self) -> Result<String, KeyMaterialError> {
        let mut text = null_mut();
        if unsafe { ConvertSidToStringSidW(self.sid(), &mut text) } == 0 {
            return Err(error(KeyMaterialErrorKind::ProtectionUnavailable));
        }
        let allocation = LocalMemory(text.cast());
        let mut len = 0;
        while unsafe { *text.add(len) } != 0 {
            len += 1;
        }
        let result = String::from_utf16(unsafe { std::slice::from_raw_parts(text, len) })
            .map_err(|_| error(KeyMaterialErrorKind::ProtectionUnavailable));
        drop(allocation);
        result
    }
}
fn descriptor(sddl: &str) -> Result<LocalMemory, KeyMaterialError> {
    let text: Vec<u16> = sddl.encode_utf16().chain(Some(0)).collect();
    let mut descriptor = null_mut();
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            text.as_ptr(),
            SDDL_REVISION_1,
            &mut descriptor,
            null_mut(),
        )
    } == 0
    {
        return Err(error(KeyMaterialErrorKind::ProtectionUnavailable));
    }
    Ok(LocalMemory(descriptor))
}
fn owner_security() -> Result<LocalMemory, KeyMaterialError> {
    let sid = CurrentUser::get()?.sid_string()?;
    descriptor(&format!("O:{sid}D:P(A;;FA;;;{sid})"))
}
fn verify_security(file: &File) -> Result<(), KeyMaterialError> {
    let mut descriptor = null_mut();
    if unsafe {
        GetSecurityInfo(
            file.as_raw_handle(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
            &mut descriptor,
        )
    } != ERROR_SUCCESS
    {
        return Err(error(KeyMaterialErrorKind::ProtectionUnavailable));
    }
    let descriptor = LocalMemory(descriptor);
    validate_descriptor(&descriptor, &CurrentUser::get()?)
}
fn validate_descriptor(
    descriptor: &LocalMemory,
    user: &CurrentUser,
) -> Result<(), KeyMaterialError> {
    let denied = || error(KeyMaterialErrorKind::ProtectionUnavailable);
    let mut owner = null_mut();
    let mut defaulted = 0;
    let mut control = 0;
    let mut revision = 0;
    let mut present = 0;
    let mut acl = null_mut();
    if unsafe { GetSecurityDescriptorOwner(descriptor.0, &mut owner, &mut defaulted) } == 0
        || owner.is_null()
        || unsafe { EqualSid(owner, user.sid()) } == 0
    {
        return Err(denied());
    }
    if unsafe { GetSecurityDescriptorControl(descriptor.0, &mut control, &mut revision) } == 0
        || control & SE_DACL_PROTECTED == 0
    {
        return Err(denied());
    }
    if unsafe { GetSecurityDescriptorDacl(descriptor.0, &mut present, &mut acl, &mut defaulted) }
        == 0
        || present == 0
        || acl.is_null()
        || unsafe { IsValidAcl(acl) } == 0
    {
        return Err(denied());
    }
    let mut rights = 0;
    for index in 0..unsafe { (*acl).AceCount } {
        let mut ace = null_mut();
        if unsafe { GetAce(acl, u32::from(index), &mut ace) } == 0 || ace.is_null() {
            return Err(denied());
        }
        let header = unsafe { &*ace.cast::<ACE_HEADER>() };
        // Only explicit, effective current-user allow ACEs are accepted. Reject
        // object/callback/unknown ACEs conservatively rather than guessing grants.
        if header.AceType != 0
            || u32::from(header.AceFlags) & (INHERITED_ACE | INHERIT_ONLY_ACE) != 0
            || usize::from(header.AceSize) < size_of::<ACCESS_ALLOWED_ACE>()
        {
            return Err(denied());
        }
        let allow = unsafe { &*ace.cast::<ACCESS_ALLOWED_ACE>() };
        let sid = (&allow.SidStart as *const u32).cast_mut().cast();
        if unsafe { IsValidSid(sid) } == 0 || unsafe { EqualSid(sid, user.sid()) } == 0 {
            return Err(denied());
        }
        rights |= allow.Mask;
    }
    if rights & FILE_ALL_ACCESS != FILE_ALL_ACCESS {
        return Err(denied());
    }
    Ok(())
}

pub(super) fn identity(file: &File) -> Result<FileIdentity, KeyMaterialError> {
    let mut id: FILE_ID_INFO = unsafe { zeroed() };
    let mut basic: FILE_BASIC_INFO = unsafe { zeroed() };
    if unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FileIdInfo,
            (&mut id as *mut FILE_ID_INFO).cast(),
            size_of::<FILE_ID_INFO>() as u32,
        )
    } == 0
        || unsafe {
            GetFileInformationByHandleEx(
                file.as_raw_handle(),
                FileBasicInfo,
                (&mut basic as *mut FILE_BASIC_INFO).cast(),
                size_of::<FILE_BASIC_INFO>() as u32,
            )
        } == 0
    {
        return Err(error(KeyMaterialErrorKind::ProtectionUnavailable));
    }
    let info = file_info(file)?;
    let lo = u64::from_le_bytes(
        id.FileId.Identifier[..8]
            .try_into()
            .expect("fixed identity size"),
    );
    let hi = u64::from_le_bytes(
        id.FileId.Identifier[8..]
            .try_into()
            .expect("fixed identity size"),
    );
    Ok(FileIdentity {
        platform: "windows-v1",
        fields: [
            id.VolumeSerialNumber,
            lo,
            hi,
            (u64::from(info.nFileSizeHigh) << 32) | u64::from(info.nFileSizeLow),
            basic.LastWriteTime as u64,
            basic.ChangeTime as u64,
            u64::from(basic.FileAttributes),
            0,
        ],
    })
}
pub(super) fn observe_regular_source(path: &Path) -> Result<FileIdentity, KeyMaterialError> {
    // Imported paths follow reparse points, but device/pipe namespace paths are
    // rejected before opening, avoiding a blocking named-pipe connect.
    let resolved = path.canonicalize().map_err(|e| {
        error(if e.kind() == std::io::ErrorKind::NotFound {
            KeyMaterialErrorKind::SourceMissing
        } else {
            KeyMaterialErrorKind::SourceUnreadable
        })
    })?;
    if !std::fs::metadata(&resolved).map_err(io_error)?.is_file() {
        return Err(error(KeyMaterialErrorKind::NotRegularFile));
    }
    let wide_path = wide(&resolved).map_err(io_error)?;
    let handle = unsafe {
        CreateFileW(
            wide_path.as_ptr(),
            GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(error(KeyMaterialErrorKind::SourceUnreadable));
    }
    let file = unsafe { File::from_raw_handle(handle) };
    check_attributes(&file, false)?;
    identity(&file)
}

#[cfg(test)]
pub(super) fn test_security_descriptor_is_accepted(owner: Option<&str>, dacl: &str) -> bool {
    let user = CurrentUser::get().unwrap();
    let sid = user.sid_string().unwrap();
    let descriptor = descriptor(&format!(
        "O:{}{}",
        owner.unwrap_or(&sid),
        dacl.replace("CURRENT", &sid)
    ))
    .unwrap();
    validate_descriptor(&descriptor, &user).is_ok()
}
#[cfg(test)]
pub(super) fn set_test_security(path: &Path, dacl: &str) {
    let sid = CurrentUser::get().unwrap().sid_string().unwrap();
    let descriptor = descriptor(&format!("O:{sid}{dacl}")).unwrap();
    let path = wide(path).unwrap();
    let flags = if dacl.starts_with("D:P") {
        PROTECTED_DACL_SECURITY_INFORMATION
    } else {
        UNPROTECTED_DACL_SECURITY_INFORMATION
    };
    assert_ne!(
        unsafe {
            SetFileSecurityW(
                path.as_ptr(),
                DACL_SECURITY_INFORMATION | flags,
                descriptor.0,
            )
        },
        0
    );
}
#[cfg(test)]
pub(super) fn test_attributes_are_accepted(attributes: u32) -> bool {
    reject_reparse(attributes).is_ok()
}
