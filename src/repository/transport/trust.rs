//! Durable host pins and the permanent fence after registry loss.
use super::super::{
    RepositoryError, RepositoryOperation, RepositoryService, cache_read_guard, cache_write_guard,
    open_registry, open_registry_read_only,
};
use super::{HostApproval, HostKeyIdentity, SshAuthority, SshTransportErrorKind};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use std::{
    fs::{self, OpenOptions},
    io::{self, Read, Write},
    path::Path,
};

const MARKER: &str = "ssh-host-trust-reapproval-required";
const MARKER_BYTES: &[u8] = b"manyhands SSH host trust reapproval required v1\n";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in super::super) struct HostTrustSnapshot {
    pub(super) pin: Option<HostKeyIdentity>,
    pub(super) reapproval_required: bool,
}

pub(in super::super) fn migrate_host_pins(
    transaction: &Transaction<'_>,
) -> Result<(), RepositoryError> {
    transaction
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS ssh_host_pins (
        host TEXT NOT NULL, port INTEGER NOT NULL, algorithm TEXT NOT NULL, sha256 TEXT NOT NULL,
        PRIMARY KEY (host, port))",
        )
        .map_err(RepositoryError::sqlite)
}

pub(in super::super) fn valid_identity(identity: &HostKeyIdentity) -> bool {
    matches!(
        identity.algorithm.as_str(),
        "ssh-ed25519"
            | "ssh-rsa"
            | "ssh-dss"
            | "ecdsa-sha2-nistp256"
            | "ecdsa-sha2-nistp384"
            | "ecdsa-sha2-nistp521"
    ) && identity
        .sha256
        .parse::<ssh_key::Fingerprint>()
        .is_ok_and(|value| {
            value.algorithm() == ssh_key::HashAlg::Sha256 && value.to_string() == identity.sha256
        })
}
pub(in super::super) fn valid_authority(authority: &SshAuthority) -> bool {
    let host = if authority.host.contains(':') {
        format!("[{}]", authority.host)
    } else {
        authority.host.clone()
    };
    super::endpoint::parse_ssh_endpoint(&format!("ssh://fixture@{host}:{}/fixture", authority.port))
        .is_ok_and(|endpoint| endpoint.authority == *authority)
}
fn read_pin(
    connection: &Connection,
    authority: &SshAuthority,
) -> Result<Option<HostKeyIdentity>, SshTransportErrorKind> {
    if !valid_authority(authority) {
        return Err(SshTransportErrorKind::RegistryUnavailable);
    }
    let pin = connection
        .query_row(
            "SELECT algorithm, sha256 FROM ssh_host_pins WHERE host=?1 AND port=?2",
            params![authority.host, authority.port],
            |row| {
                Ok(HostKeyIdentity {
                    algorithm: row.get(0)?,
                    sha256: row.get(1)?,
                })
            },
        )
        .optional()
        .map_err(|_| SshTransportErrorKind::RegistryUnavailable)?;
    if pin.as_ref().is_some_and(|pin| !valid_identity(pin)) {
        return Err(SshTransportErrorKind::RegistryUnavailable);
    }
    Ok(pin)
}
pub(super) fn approval_matches(
    approval: Option<&HostApproval>,
    authority: &SshAuthority,
    expected: Option<&HostKeyIdentity>,
    observed: &HostKeyIdentity,
) -> bool {
    approval.is_some_and(|approval| {
        approval.authority == *authority
            && approval.expected.as_ref() == expected
            && approval.presented == *observed
    })
}

impl RepositoryService {
    #[cfg(test)]
    pub(in super::super) fn read_host_pin(
        &self,
        authority: &SshAuthority,
    ) -> Result<Option<HostKeyIdentity>, SshTransportErrorKind> {
        self.read_host_trust(authority).map(|snapshot| snapshot.pin)
    }
    pub(in super::super) fn read_host_trust(
        &self,
        authority: &SshAuthority,
    ) -> Result<HostTrustSnapshot, SshTransportErrorKind> {
        let data = self
            .registry_path
            .parent()
            .ok_or(SshTransportErrorKind::RegistryUnavailable)?;
        let _guard = cache_read_guard(&self.registry_path, data, RepositoryOperation::OpenRegistry)
            .map_err(|_| SshTransportErrorKind::RegistryUnavailable)?;
        let reapproval_required =
            read_reapproval_marker(data).map_err(|_| SshTransportErrorKind::RegistryUnavailable)?;
        let connection = open_registry_read_only(&self.registry_path)
            .map_err(|_| SshTransportErrorKind::RegistryUnavailable)?;
        Ok(HostTrustSnapshot {
            pin: read_pin(&connection, authority)?,
            reapproval_required,
        })
    }
    /// Pins `identity` for `authority` as an exact approval does, through
    /// the code an operation uses and without contacting the host.
    #[doc(hidden)]
    pub fn approve_host_pin_for_testing(
        &self,
        authority: &SshAuthority,
        identity: &HostKeyIdentity,
    ) -> Result<(), SshTransportErrorKind> {
        let snapshot = self.read_host_trust(authority)?;
        let approval = HostApproval {
            authority: authority.clone(),
            expected: snapshot.pin.clone(),
            presented: identity.clone(),
        };
        self.finalize_host_trust(authority, &snapshot, identity, Some(&approval))
    }
    /// Leaves the marker that the recovery of a lost pin registry leaves.
    #[doc(hidden)]
    pub fn require_host_reapproval_for_testing(&self) -> Result<(), SshTransportErrorKind> {
        let data = self
            .registry_path
            .parent()
            .ok_or(SshTransportErrorKind::RegistryUnavailable)?;
        let _guard =
            cache_write_guard(&self.registry_path, data, RepositoryOperation::OpenRegistry)
                .map_err(|_| SshTransportErrorKind::RegistryUnavailable)?;
        publish_reapproval_marker(data).map_err(|_| SshTransportErrorKind::RegistryUnavailable)
    }
    /// Whether an operation would find the reapproval marker set.
    #[doc(hidden)]
    pub fn host_reapproval_required_for_testing(
        &self,
        authority: &SshAuthority,
    ) -> Result<bool, SshTransportErrorKind> {
        self.read_host_trust(authority)
            .map(|snapshot| snapshot.reapproval_required)
    }
    /// Pin-only convenience. Operations must use finalize_host_trust with their
    /// original marker snapshot to detect recovery across network work.
    #[cfg(test)]
    pub(in super::super) fn finalize_host_pin(
        &self,
        authority: &SshAuthority,
        expected: Option<&HostKeyIdentity>,
        observed: &HostKeyIdentity,
        approval: Option<&HostApproval>,
    ) -> Result<(), SshTransportErrorKind> {
        let snapshot = self.read_host_trust(authority)?;
        self.finalize_host_trust(
            authority,
            &HostTrustSnapshot {
                pin: expected.cloned(),
                ..snapshot
            },
            observed,
            approval,
        )
    }
    pub(in super::super) fn finalize_host_trust(
        &self,
        authority: &SshAuthority,
        snapshot: &HostTrustSnapshot,
        observed: &HostKeyIdentity,
        approval: Option<&HostApproval>,
    ) -> Result<(), SshTransportErrorKind> {
        let data = self
            .registry_path
            .parent()
            .ok_or(SshTransportErrorKind::RegistryUnavailable)?;
        let _guard =
            cache_write_guard(&self.registry_path, data, RepositoryOperation::OpenRegistry)
                .map_err(|_| SshTransportErrorKind::RegistryUnavailable)?;
        let marker =
            read_reapproval_marker(data).map_err(|_| SshTransportErrorKind::RegistryUnavailable)?;
        if marker != snapshot.reapproval_required {
            return Err(SshTransportErrorKind::HostTrustChanged);
        }
        if !valid_identity(observed) {
            return Err(SshTransportErrorKind::HostVerificationUnavailable);
        }
        let mut connection = open_registry(&self.registry_path, &mut |_| {})
            .map_err(|_| SshTransportErrorKind::RegistryUnavailable)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| SshTransportErrorKind::RegistryUnavailable)?;
        let current = read_pin(&tx, authority)?;
        let approved = approval_matches(approval, authority, snapshot.pin.as_ref(), observed);
        let already_installed = current == snapshot.pin
            && current.as_ref() == Some(observed)
            && approval.is_some_and(|approval| {
                approval.authority == *authority && approval.presented == *observed
            });
        if approval.is_some() && !approved && !already_installed {
            return Err(SshTransportErrorKind::HostTrustChanged);
        }
        if current != snapshot.pin && !(approved && current.as_ref() == Some(observed)) {
            return Err(SshTransportErrorKind::HostTrustChanged);
        }
        if !approved
            && (snapshot.pin.as_ref().is_some_and(|pin| pin != observed)
                || (snapshot.pin.is_none() && marker))
        {
            return Err(SshTransportErrorKind::HostTrustChanged);
        }
        if approved {
            tx.execute("INSERT INTO ssh_host_pins (host, port, algorithm, sha256) VALUES (?1, ?2, ?3, ?4)
                ON CONFLICT(host, port) DO UPDATE SET algorithm=excluded.algorithm, sha256=excluded.sha256",
                params![authority.host, authority.port, observed.algorithm, observed.sha256])
                .map_err(|_| SshTransportErrorKind::RegistryUnavailable)?;
        }
        tx.commit()
            .map_err(|_| SshTransportErrorKind::RegistryUnavailable)
    }
}

fn invalid_marker() -> io::Error {
    io::Error::other("SSH host trust recovery marker is unavailable")
}
pub(in super::super) fn read_reapproval_marker(data: &Path) -> io::Result<bool> {
    let path = data.join(MARKER);
    match fs::symlink_metadata(&path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(_) => return Err(invalid_marker()),
        Ok(metadata)
            if !metadata.file_type().is_file() || metadata.len() != MARKER_BYTES.len() as u64 =>
        {
            return Err(invalid_marker());
        }
        Ok(_) => {}
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT);
    }
    let file = options.open(path).map_err(|_| invalid_marker())?;
    if !file.metadata()?.is_file() {
        return Err(invalid_marker());
    }
    let mut bytes = Vec::new();
    file.take(MARKER_BYTES.len() as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes != MARKER_BYTES {
        return Err(invalid_marker());
    }
    Ok(true)
}
/// Caller holds the exclusive cache-recovery guard. Publish before touching the
/// database, so every interruption leaves either original pins or a trust fence.
pub(in super::super) fn publish_reapproval_marker(data: &Path) -> io::Result<()> {
    if read_reapproval_marker(data)? {
        // Windows FlushFileBuffers requires a handle with write access.
        OpenOptions::new()
            .read(true)
            .write(true)
            .open(data.join(MARKER))?
            .sync_all()?;
        return sync_marker_directory(data);
    }
    let mut temporary = tempfile::NamedTempFile::new_in(data)?;
    temporary.write_all(MARKER_BYTES)?;
    temporary.as_file().sync_all()?;
    #[cfg(unix)]
    temporary
        .persist_noclobber(data.join(MARKER))
        .map_err(|_| invalid_marker())?;
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{MOVEFILE_WRITE_THROUGH, MoveFileExW};
        let source: Vec<u16> = temporary
            .path()
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        let destination: Vec<u16> = data
            .join(MARKER)
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        // No replace-existing flag; both paths remain alive and NUL terminated.
        if unsafe {
            MoveFileExW(
                source.as_ptr(),
                destination.as_ptr(),
                MOVEFILE_WRITE_THROUGH,
            )
        } == 0
        {
            return Err(invalid_marker());
        }
    }
    sync_marker_directory(data)?;
    if !read_reapproval_marker(data)? {
        return Err(invalid_marker());
    }
    Ok(())
}
#[cfg(unix)]
fn sync_marker_directory(data: &Path) -> io::Result<()> {
    fs::File::open(data)?.sync_all()
}
#[cfg(windows)]
fn sync_marker_directory(_: &Path) -> io::Result<()> {
    // Windows publication above uses MoveFileExW(MOVEFILE_WRITE_THROUGH);
    // FlushFileBuffers on the file preceded publication. Directory flushing
    // is unsupported for ordinary directory handles on Windows.
    Ok(())
}
