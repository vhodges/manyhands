//! Observation tokens: how a request shows it has seen the current state.
//!
//! A caller returns the token `show_item` gave it. A new request's binding
//! reads the item's effective copy the way that read does and compares. A
//! token that matches gives the digest the domain operation checks again
//! under its lease; one that does not is answered by the already-applied
//! rule.

use std::{ffi::OsStr, path::Path};

use crate::results::ResultCode;

use super::super::ExpectedPathObservation;

/// What the absent token digests in place of a file's bytes.
const ABSENT_MARKER: &[u8] = b"manyhands absent path v1";

/// The observation token of a path where nothing exists: `v1:` and the
/// lowercase hexadecimal BLAKE3 digest of these bytes, in this order:
///
/// 1. one byte, `0x02`, which no file's token begins with;
/// 2. the fixed marker `manyhands absent path v1`;
/// 3. one byte, `0x01` when the context has a branch and `0x00` when it
///    has none; and, only when it has one, the branch name's length in
///    bytes as an unsigned 64-bit little-endian integer followed by the
///    name;
/// 4. the repository-relative path's length in bytes, encoded the same
///    way, followed by the path with forward slashes.
pub(crate) fn absent_token(branch: Option<&str>, path: &str) -> String {
    fn part(hasher: &mut blake3::Hasher, bytes: &[u8]) {
        hasher.update(&(bytes.len() as u64).to_le_bytes());
        hasher.update(bytes);
    }
    let mut hasher = blake3::Hasher::new();
    hasher.update(&[2]);
    hasher.update(ABSENT_MARKER);
    match branch {
        Some(branch) => {
            hasher.update(&[1]);
            part(&mut hasher, branch.as_bytes());
        }
        None => {
            hasher.update(&[0]);
        }
    }
    part(&mut hasher, path.as_bytes());
    format!("v1:{}", hasher.finalize().to_hex())
}

/// What the observation digest is begun with.
const OBSERVATION_DOMAIN: &[u8] = b"manyhands observation v1";

/// One part of the state a command that needs confirmation binds, as a
/// digest. No item token covers this state.
///
/// What a confirmation stores is two of these joined, `ObservationDigest`:
/// one of the state the command changes, and one of what it reads and
/// does not change. They are kept apart because they are compared at
/// different times; see `ObservationDigest`.
///
/// Each value is fed under its name with its length, and one that is
/// absent is told apart from one that is empty, so no two different
/// observations feed the same bytes. The digest holds nothing that can be
/// read back.
pub(crate) struct Observation(blake3::Hasher);

impl Observation {
    /// Begins the observation of `command`.
    pub(crate) fn of(command: &str) -> Self {
        let mut observation = Self(blake3::Hasher::new());
        observation.part(OBSERVATION_DOMAIN);
        observation.part(command.as_bytes());
        observation
    }

    fn part(&mut self, bytes: &[u8]) {
        self.0.update(&(bytes.len() as u64).to_le_bytes());
        self.0.update(bytes);
    }

    /// What was observed under `name`.
    pub(crate) fn text(mut self, name: &str, value: &str) -> Self {
        self.part(name.as_bytes());
        self.0.update(&[1]);
        self.part(value.as_bytes());
        self
    }

    /// Nothing was there to observe under `name`.
    pub(crate) fn absent(mut self, name: &str) -> Self {
        self.part(name.as_bytes());
        self.0.update(&[0]);
        self
    }

    /// The part's digest: the first 128 bits of the hash, as 32
    /// lowercase hexadecimal digits. Two of them are what one record
    /// column holds.
    pub(crate) fn finish(self) -> String {
        self.0.finalize().to_hex()[..PART_DIGITS].to_owned()
    }
}

/// The hexadecimal digits of one part of an observation digest.
const PART_DIGITS: usize = 32;

/// What a preview observed, as a confirmation stores it: the digest of
/// the state the command changes, followed by the digest of what the
/// command reads and never changes, which for the bound commands is the
/// identity it will commit as. Together they are the 64 lowercase
/// hexadecimal digits the record's column holds.
///
/// A new request is accepted only when both are what a fresh observation
/// gives. A request that is entered again compares them apart:
///
/// - `unchanging` is compared on every call of the request. Nothing the
///   request does changes it, so a difference is always a change from
///   elsewhere, however far the request has got.
/// - `state` is compared only while the request's operation has
///   recorded no step. From its first step on, the request's own work
///   has changed what would be observed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ObservationDigest {
    pub(crate) state: String,
    pub(crate) unchanging: String,
}

impl ObservationDigest {
    /// The digest as a confirmation record holds it.
    pub(crate) fn to_stored(&self) -> String {
        format!("{}{}", self.state, self.unchanging)
    }

    /// The two parts of a digest a record holds. `None` for anything
    /// that is not one: nothing is compared with it, and nothing is run
    /// on it.
    pub(crate) fn from_stored(stored: &str) -> Option<Self> {
        let well_formed = stored.len() == 2 * PART_DIGITS
            && stored
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
        well_formed.then(|| {
            let (state, unchanging) = stored.split_at(PART_DIGITS);
            Self {
                state: state.to_owned(),
                unchanging: unchanging.to_owned(),
            }
        })
    }
}

/// A file as a read found it: its bytes and the token the read gives.
pub(crate) struct ObservedFile {
    pub(crate) token: String,
    pub(crate) bytes: Vec<u8>,
}

impl ObservedFile {
    /// What the domain operation is to expect at the path, when `token` is
    /// the file's: the digest of the same bytes the token was made from.
    /// `None` for a token that is not the file's.
    pub(crate) fn check(&self, token: &str) -> Option<ExpectedPathObservation> {
        (self.token == token).then(|| ExpectedPathObservation::from_bytes(&self.bytes))
    }
}

/// Whether `bytes`, the file at `path` in the worktree `directory`, are
/// the blob at that worktree's head: committed and unmodified.
///
/// For an item with no editing context this is the primary-is-committed
/// rule. Its effective copy is the file in the primary worktree, and the
/// domain operation creates the context by checking out primary's head, so
/// the two are the same bytes only when this holds. For a stale token it
/// is the "committed at its branch tip" of the already-applied rule.
///
/// A repository whose attributes rewrite the file on checkout can differ
/// although this holds; that is a stated limit.
pub(crate) fn is_committed(
    directory: &Path,
    path: &str,
    bytes: &[u8],
) -> Result<bool, git2::Error> {
    // Exactly this worktree, never a repository above it.
    let repository = git2::Repository::open_ext(
        directory,
        git2::RepositoryOpenFlags::NO_SEARCH,
        &[] as &[&OsStr],
    )?;
    let head = match repository.head() {
        Ok(head) => head.peel_to_tree()?,
        Err(error)
            if matches!(
                error.code(),
                git2::ErrorCode::UnbornBranch | git2::ErrorCode::NotFound
            ) =>
        {
            return Ok(false);
        }
        Err(error) => return Err(error),
    };
    let entry = match head.get_path(Path::new(path)) {
        Ok(entry) => entry,
        Err(error) if error.code() == git2::ErrorCode::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    match repository.find_blob(entry.id()) {
        Ok(blob) => Ok(blob.content() == bytes),
        // Something other than a file is committed there.
        Err(error) if error.code() == git2::ErrorCode::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

/// The already-applied rule, for a token that is not the file's.
///
/// `as_intended` says the item as it now is equals what the request
/// intends, by parsed fields and ignoring the fields written once;
/// `committed` says the file is committed at its branch tip. When both
/// hold the request's end state already holds and nothing is written.
/// Anything else is a change from elsewhere.
pub(crate) fn stale_token_code(as_intended: bool, committed: bool) -> ResultCode {
    if as_intended && committed {
        ResultCode::AlreadyApplied
    } else {
        ResultCode::ExternalChange
    }
}

#[cfg(test)]
#[path = "observe_tests.rs"]
mod tests;
