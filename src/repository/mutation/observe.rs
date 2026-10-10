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
// No bound command takes an absent token yet: `observe_path` and
// `document move` are the first to.
#[allow(dead_code)]
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
#[allow(dead_code)]
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
