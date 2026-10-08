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
            StoredSharedKeysError, bounded_public_key_contents, openssh_public_key_fingerprint,
            stored_shared_key_registrations,
        },
        transport::{SshAuthority, endpoint::parse_ssh_endpoint, trust::read_reapproval_marker},
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

/// The one line of OpenSSH public key text in a public key file, without
/// its line ending, and its fingerprint.
///
/// A file that holds anything else is refused whole, so that a file put in
/// a public key's place is never returned as one: text of more than one
/// line, such as a private key, has no public key to show.
fn public_key_line(contents: &[u8]) -> Option<(String, String)> {
    let text = std::str::from_utf8(contents).ok()?;
    let line = text.trim_end_matches(['\r', '\n']);
    if line.chars().any(char::is_control) {
        return None;
    }
    let fingerprint = openssh_public_key_fingerprint(line)?;
    Some((line.to_owned(), fingerprint))
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
    let pins = statement
        .query_map(
            (
                authority.map(|authority| authority.host.as_str()),
                authority.map(|authority| authority.port),
            ),
            |row| {
                Ok(HostPinDto {
                    host: row.get(0)?,
                    port: row.get(1)?,
                    algorithm: row.get(2)?,
                    sha256: row.get(3)?,
                    reapproval_required,
                })
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(pins)
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
    /// `public_key_unavailable` when the registration has no public key
    /// file, or the file is missing, unreadable, not a regular file, larger
    /// than 16 KiB, or anything but one line of OpenSSH public key text.
    /// The private key file is not touched.
    pub fn public_key_text(&self, id: SharedKeyId) -> Result<PublicKeyDto, ReadError> {
        let registration = self.stored_key(id)?;
        let unavailable = || ReadError::new(ResultCode::PublicKeyUnavailable);
        let path = registration
            .public_key_path
            .as_deref()
            .ok_or_else(unavailable)?;
        let contents = bounded_public_key_contents(path).ok_or_else(unavailable)?;
        let (public_key, fingerprint) = public_key_line(&contents).ok_or_else(unavailable)?;
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
        Ok(HostPinListDto {
            items: self.stored_host_pins(None)?,
            complete: true,
        })
    }

    /// The pin for one authority. The host is compared as a pin stores it,
    /// so its letter case and the written form of an address do not matter.
    pub fn inspect_host(&self, authority: &SshAuthority) -> Result<HostPinDto, ReadError> {
        let not_found = || ReadError::new(ResultCode::AuthorityNotFound);
        // The index is consulted even for an authority no pin can have, so
        // that a busy or unavailable index is reported as that.
        let Some(authority) = stored_authority(authority) else {
            self.stored_host_pins(None)?;
            return Err(not_found());
        };
        self.stored_host_pins(Some(&authority))?
            .into_iter()
            .next()
            .ok_or_else(not_found)
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
    ) -> Result<Vec<HostPinDto>, ReadError> {
        let data_directory = self.data_directory();
        self.read_session(RepositoryOperation::Read, |connection| {
            let reapproval_required = read_reapproval_marker(data_directory)
                .map_err(|error| ReadError::new(ResultCode::InternalError).with_source(error))?;
            host_pins(connection, authority, reapproval_required)
        })
    }
}
