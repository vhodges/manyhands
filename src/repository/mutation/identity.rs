//! What identifies a request: its ID, the ID of the confirmation it was
//! given, the scope it works in, and the digest of what it asks for.

use std::{fmt, path::Path};

/// Defines an ID that is a canonical uppercase ULID, as `OperationId` is.
macro_rules! ulid_id {
    ($(#[$meta:meta])* $name:ident, $error:ident, $what:literal) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub struct $name(ulid::Ulid);

        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct $error;

        impl $name {
            pub fn new() -> Self {
                Self(ulid::Ulid::new())
            }

            pub fn parse(value: &str) -> Result<Self, $error> {
                let id: ulid::Ulid = value.parse().map_err(|_| $error)?;
                (id.to_string() == value).then_some(Self(id)).ok_or($error)
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }

        impl fmt::Display for $error {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(concat!($what, " must be a canonical uppercase ULID"))
            }
        }

        impl std::error::Error for $error {}
    };
}

ulid_id!(
    /// Names one request to change Manyhands state. The caller invents it
    /// and repeats it to retry the same request.
    RequestId,
    RequestIdParseError,
    "request ID"
);

ulid_id!(
    /// Names one preview that a later request may present as its consent.
    ConfirmationId,
    ConfirmationIdParseError,
    "confirmation ID"
);

/// What a request works in: one repository, by its canonical root, or the
/// application for the commands that belong to no repository.
///
/// It is a path string and not a registration: a request to create a
/// repository exists before any registration does, and a request to remove
/// one outlives it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ScopeKey(String);

impl ScopeKey {
    /// The key of the commands that belong to no repository.
    pub const APPLICATION: &'static str = "application";

    pub fn application() -> Self {
        Self(Self::APPLICATION.to_owned())
    }

    /// The key of the repository at `root`, which must exist: its
    /// canonical path. `None` when the path cannot be resolved or cannot
    /// be written as text.
    pub fn for_repository(root: &Path) -> Option<Self> {
        let root = std::fs::canonicalize(root).ok()?;
        root.to_str().map(|root| Self(root.to_owned()))
    }

    /// The key of a repository that `root` is about to name: the canonical
    /// parent joined with the final name. That is the canonical root once
    /// the directory exists, so the key is the same before and after it is
    /// created. `None` when `root` has no parent or name, the parent
    /// cannot be resolved, or the result cannot be written as text.
    pub fn for_new_repository(root: &Path) -> Option<Self> {
        let name = root.file_name()?;
        let parent = std::fs::canonicalize(root.parent()?).ok()?;
        parent.join(name).to_str().map(|root| Self(root.to_owned()))
    }

    /// A key as a record holds it.
    pub fn from_stored(stored: &str) -> Self {
        Self(stored.to_owned())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The repository root this key names, or `None` for the application.
    pub fn repository(&self) -> Option<&str> {
        (self.0 != Self::APPLICATION).then_some(self.0.as_str())
    }
}

/// What makes two records' digests differ for equal input: the ID of the
/// record the digest is stored in. The tables then do not reveal that two
/// requests carried the same content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DigestSalt {
    Request(RequestId),
    Confirmation(ConfirmationId),
}

/// One typed input of a command, as the digest reads it. Absent, null and
/// empty are different values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldValue<'a> {
    /// The caller did not supply the field.
    Absent,
    /// The caller supplied the field as nothing, which clears it.
    Null,
    Text(&'a str),
    /// A Markdown body or source. It is fed to the digest and stored
    /// nowhere.
    Bytes(&'a [u8]),
    List(&'a [&'a str]),
    Flag(bool),
}

const DOMAIN: &[u8] = b"manyhands request v1";

/// The digest of a request's command, repository, target and semantic
/// input: what a retry must repeat to be the same request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct IntentDigest([u8; 32]);

impl IntentDigest {
    /// Begins a digest. `fields` are then added in the binding's fixed
    /// order.
    pub fn builder(
        salt: DigestSalt,
        command: &str,
        repository: &ScopeKey,
        target: &str,
    ) -> IntentDigestBuilder {
        let mut builder = IntentDigestBuilder(blake3::Hasher::new());
        builder.part(DOMAIN);
        let (kind, salt) = match salt {
            DigestSalt::Request(id) => (0, id.to_string()),
            DigestSalt::Confirmation(id) => (1, id.to_string()),
        };
        builder.tag(kind);
        builder.part(salt.as_bytes());
        builder.part(command.as_bytes());
        builder.part(repository.as_str().as_bytes());
        builder.part(target.as_bytes());
        builder
    }

    /// A digest as a record holds it: 64 lowercase hexadecimal digits.
    pub fn from_stored(stored: &str) -> Option<Self> {
        stored_digest(stored).map(Self)
    }
}

/// The 32 bytes a record holds as 64 lowercase hexadecimal digits, or
/// `None` for anything else.
pub(super) fn stored_digest(stored: &str) -> Option<[u8; 32]> {
    let is_canonical = stored.len() == 64
        && stored
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    if !is_canonical {
        return None;
    }
    blake3::Hash::from_hex(stored)
        .ok()
        .map(|hash| *hash.as_bytes())
}

impl fmt::Display for IntentDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(blake3::Hash::from_bytes(self.0).to_hex().as_str())
    }
}

/// Feeds a digest its parts. Every part is prefixed with its length, and a
/// field carries its name and the kind of its value, so no two different
/// inputs feed the same bytes.
pub struct IntentDigestBuilder(blake3::Hasher);

impl IntentDigestBuilder {
    fn tag(&mut self, tag: u8) {
        self.0.update(&[tag]);
    }

    fn length(&mut self, length: usize) {
        // A length always fits: `usize` is at most 64 bits wide.
        self.0.update(&(length as u64).to_le_bytes());
    }

    fn part(&mut self, bytes: &[u8]) {
        self.length(bytes.len());
        self.0.update(bytes);
    }

    pub fn field(mut self, name: &str, value: FieldValue<'_>) -> Self {
        self.part(name.as_bytes());
        match value {
            FieldValue::Absent => self.tag(0),
            FieldValue::Null => self.tag(1),
            FieldValue::Text(text) => {
                self.tag(2);
                self.part(text.as_bytes());
            }
            FieldValue::Bytes(bytes) => {
                self.tag(3);
                self.part(bytes);
            }
            FieldValue::List(entries) => {
                self.tag(4);
                self.length(entries.len());
                for entry in entries {
                    self.part(entry.as_bytes());
                }
            }
            FieldValue::Flag(flag) => {
                self.tag(5);
                self.tag(u8::from(flag));
            }
        }
        self
    }

    pub fn finish(self) -> IntentDigest {
        IntentDigest(*self.0.finalize().as_bytes())
    }
}
