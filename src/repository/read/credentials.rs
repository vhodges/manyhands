//! Credential reads: key registrations, public key text and host pins.
//!
//! These are application-wide and take no repository. None of them opens a
//! private key file, takes the key-store lock or can prompt: a registration
//! is reported as the index stores it, and the one file read here is a
//! registration's public key file.

use std::path::Path;

use rusqlite::Connection;

use super::{
    HostPinDto, HostPinListDto, KeyDto, KeyListDto, KeyOwnership, KeyPrivateSourceState,
    KeyPublicMetadataState, PublicKeyDto, ReadError,
};
use crate::{
    repository::{
        PrivateKeySourceState, PublicKeyMetadataState, RepositoryOperation, RepositoryService,
        SharedKeyId, SharedKeyOwnership, SharedKeyRegistration,
        keys::{
            StoredSharedKeysError, bounded_public_key_contents, openssh_public_key,
            public_key_fingerprint, stored_shared_key_registrations,
        },
        transport::{
            HostKeyIdentity, SshAuthority,
            endpoint::parse_ssh_endpoint,
            trust::{read_reapproval_marker, valid_authority, valid_identity},
        },
    },
    results::{ResultCode, absolute_path_string},
};

impl From<StoredSharedKeysError> for ReadError {
    fn from(error: StoredSharedKeysError) -> Self {
        match error {
            StoredSharedKeysError::Sqlite(error) => error.into(),
            StoredSharedKeysError::InvalidMetadata(error) => error.into(),
        }
    }
}

fn key_dto(registration: &SharedKeyRegistration) -> Result<KeyDto, ReadError> {
    // A stored path is absolute UTF-8 text, or its row would not decode.
    let path = |path: &Path| {
        absolute_path_string(path).ok_or_else(|| ReadError::new(ResultCode::InternalError))
    };
    Ok(KeyDto {
        id: registration.id.to_string(),
        label: registration.label.clone(),
        ownership: match registration.ownership {
            SharedKeyOwnership::Imported => KeyOwnership::Imported,
            SharedKeyOwnership::Generated => KeyOwnership::Generated,
        },
        selected: registration.selected,
        fingerprint: registration.public_key_fingerprint.clone(),
        private_source_state: match registration.private_source_state {
            PrivateKeySourceState::Available => KeyPrivateSourceState::Available,
            PrivateKeySourceState::Missing => KeyPrivateSourceState::Missing,
            PrivateKeySourceState::Unavailable => KeyPrivateSourceState::Unavailable,
        },
        public_metadata_state: match registration.public_metadata_state {
            PublicKeyMetadataState::NotProvided => KeyPublicMetadataState::NotProvided,
            PublicKeyMetadataState::FingerprintAvailable => KeyPublicMetadataState::Available,
            PublicKeyMetadataState::Unavailable => KeyPublicMetadataState::Unavailable,
        },
        private_key_path: path(&registration.private_key_path)?,
        public_key_path: registration
            .public_key_path
            .as_deref()
            .map(path)
            .transpose()?,
    })
}

/// The longest comment a read returns with a public key. The key itself is
/// bounded by the 16 KiB a public key file may hold, so a large key is not
/// refused for the length of its line.
const LONGEST_PUBLIC_KEY_COMMENT: usize = 256;

/// What an armored private key says of itself, which no comment may say.
const PRIVATE_KEY_ARMOR: &str = "PRIVATE KEY";

/// Whether text can be shown as it is: printable ASCII and nothing else,
/// so no line break, control character or direction override.
fn plain_text(text: &str) -> bool {
    text.bytes().all(|byte| matches!(byte, 0x20..=0x7e))
}

/// The canonical one-line encoding of the public key a file holds, and the
/// key's fingerprint.
///
/// Nothing of the file is returned as it was read. The key is parsed and
/// written out again, and the only free text in the result is the key's
/// comment, which must be plain text of at most 256 bytes that does not
/// name a private key. A file that is anything else has no public key to
/// show.
fn canonical_public_key(contents: &[u8]) -> Option<(String, String)> {
    let public_key = openssh_public_key(std::str::from_utf8(contents).ok()?)?;
    let comment = public_key.comment();
    if comment.len() > LONGEST_PUBLIC_KEY_COMMENT
        || !plain_text(comment)
        || comment.contains(PRIVATE_KEY_ARMOR)
    {
        return None;
    }
    let line = public_key.to_openssh().ok()?;
    // Checked again on what is returned, whatever the encoder made of it.
    if !plain_text(&line) || line.contains(PRIVATE_KEY_ARMOR) {
        return None;
    }
    Some((line, public_key_fingerprint(&public_key)))
}

/// The authority as a pin stores it: the host in lower case, an address in
/// its canonical text. `None` when no pin can have this authority.
fn stored_authority(authority: &SshAuthority) -> Option<SshAuthority> {
    // Neither character can be in a host, and either would move the text
    // after it out of the host's place in the location parsed below.
    if authority.host.contains(['/', '@']) {
        return None;
    }
    // An IPv6 address is held bare and written in brackets, as the pin
    // registry writes it to check an authority before storing a pin.
    let host = if authority.host.contains(':') {
        format!("[{}]", authority.host)
    } else {
        authority.host.clone()
    };
    parse_ssh_endpoint(&format!("ssh://pin@{host}:{}/pin", authority.port))
        .ok()
        .map(|endpoint| endpoint.authority)
        .filter(|stored| stored.port == authority.port)
}

fn host_pins(
    connection: &Connection,
    authority: Option<&SshAuthority>,
    reapproval_required: bool,
) -> Result<Vec<HostPinDto>, ReadError> {
    // `host` is ASCII in lower case, so byte order is the order a reader
    // expects; `port` is an integer and orders as one.
    let mut statement = connection.prepare(
        "SELECT host, port, algorithm, sha256 FROM ssh_host_pins
          WHERE (?1 IS NULL OR (host = ?1 AND port = ?2))
          ORDER BY host ASC, port ASC",
    )?;
    let rows = statement
        .query_map(
            (
                authority.map(|authority| authority.host.as_str()),
                authority.map(|authority| authority.port),
            ),
            |row| {
                Ok((
                    SshAuthority {
                        host: row.get(0)?,
                        port: row.get(1)?,
                    },
                    HostKeyIdentity {
                        algorithm: row.get(2)?,
                        sha256: row.get(3)?,
                    },
                ))
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;
    rows.into_iter()
        .map(|(authority, identity)| {
            // What the transport would refuse to trust is not reported as
            // a pin: one such row fails the read, with no partial list.
            if !valid_authority(&authority) || !valid_identity(&identity) {
                return Err(ReadError::new(ResultCode::InternalError));
            }
            Ok(HostPinDto {
                host: authority.host,
                port: authority.port,
                algorithm: identity.algorithm,
                sha256: identity.sha256,
                reapproval_required,
            })
        })
        .collect()
}

impl RepositoryService {
    /// Every key registration, in the order the keys were registered.
    ///
    /// This reads the index and nothing else. The states and the fingerprint
    /// are those stored with the registration; no key file is opened.
    pub fn list_keys(&self) -> Result<KeyListDto, ReadError> {
        let items = self
            .stored_keys()?
            .iter()
            .map(key_dto)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(KeyListDto {
            items,
            complete: true,
        })
    }

    /// One key registration, as `list_keys` reports it.
    pub fn show_key(&self, id: SharedKeyId) -> Result<KeyDto, ReadError> {
        key_dto(&self.stored_key(id)?)
    }

    /// The public key now in a registration's public key file, its
    /// fingerprint, and whether that is the fingerprint the registration
    /// stores.
    ///
    /// `public_key` is the canonical encoding of the key the file holds,
    /// not the file's text. `public_key_unavailable` when the registration
    /// has no public key file; when that path is the private key path of
    /// any registration, in which case the file is not opened; or when the
    /// file is missing, unreadable, not a regular file, larger than 16 KiB
    /// or not an OpenSSH public key whose comment is plain text. No private
    /// key file is touched.
    pub fn public_key_text(&self, id: SharedKeyId) -> Result<PublicKeyDto, ReadError> {
        let registrations = self.stored_keys()?;
        let registration = registrations
            .iter()
            .find(|registration| registration.id == id)
            .ok_or_else(|| ReadError::new(ResultCode::KeyNotFound))?;
        let unavailable = || ReadError::new(ResultCode::PublicKeyUnavailable);
        let path = registration
            .public_key_path
            .as_deref()
            .ok_or_else(unavailable)?;
        // Decided from the stored paths, before anything is opened.
        if registrations
            .iter()
            .any(|registration| registration.private_key_path == path)
        {
            return Err(unavailable());
        }
        let contents = bounded_public_key_contents(path).ok_or_else(unavailable)?;
        let (public_key, fingerprint) = canonical_public_key(&contents).ok_or_else(unavailable)?;
        Ok(PublicKeyDto {
            id: registration.id.to_string(),
            matches_registration: registration.public_key_fingerprint.as_deref()
                == Some(fingerprint.as_str()),
            public_key,
            fingerprint,
        })
    }

    /// Every host pin, ordered by host and then port.
    pub fn list_host_pins(&self) -> Result<HostPinListDto, ReadError> {
        let (items, reapproval_required) = self.stored_host_pins(None)?;
        Ok(HostPinListDto {
            items,
            complete: true,
            reapproval_required,
        })
    }

    /// The pin for one authority. The host is compared as a pin stores it,
    /// so its letter case and the written form of an address do not matter.
    pub fn inspect_host(&self, authority: &SshAuthority) -> Result<HostPinDto, ReadError> {
        let not_found = || ReadError::new(ResultCode::AuthorityNotFound);
        // No pin can have this authority, so no row is read for it. The
        // session is still opened, so that a busy or unavailable index is
        // reported as that.
        let Some(authority) = stored_authority(authority) else {
            self.read_session(RepositoryOperation::Read, |_| Ok(()))?;
            return Err(not_found());
        };
        let (pins, _) = self.stored_host_pins(Some(&authority))?;
        pins.into_iter().next().ok_or_else(not_found)
    }

    /// Where the index is, and the reapproval marker beside it.
    fn data_directory(&self) -> &Path {
        self.registry_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
    }

    fn stored_keys(&self) -> Result<Vec<SharedKeyRegistration>, ReadError> {
        let operation = RepositoryOperation::Read;
        let data_directory = self.data_directory();
        self.read_session(operation, |connection| {
            Ok(stored_shared_key_registrations(
                connection,
                operation,
                data_directory,
            )?)
        })
    }

    fn stored_key(&self, id: SharedKeyId) -> Result<SharedKeyRegistration, ReadError> {
        self.stored_keys()?
            .into_iter()
            .find(|registration| registration.id == id)
            .ok_or_else(|| ReadError::new(ResultCode::KeyNotFound))
    }

    /// The pins, and the reapproval marker read under the same shared lock.
    fn stored_host_pins(
        &self,
        authority: Option<&SshAuthority>,
    ) -> Result<(Vec<HostPinDto>, bool), ReadError> {
        let data_directory = self.data_directory();
        self.read_session(RepositoryOperation::Read, |connection| {
            let reapproval_required = read_reapproval_marker(data_directory)
                .map_err(|error| ReadError::new(ResultCode::InternalError).with_source(error))?;
            Ok((
                host_pins(connection, authority, reapproval_required)?,
                reapproval_required,
            ))
        })
    }
}
