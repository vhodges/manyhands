//! Whether a pending setup has merged: level-triggered, from what the local
//! primary holds now (design Part B, R-17).
use std::path::Path;

use super::record::SetupRecord;
use crate::canonical;

/// Whether the local primary already holds the setup `record`.
///
/// This reads `refs/heads/<primary_branch>`, never HEAD: HEAD may name the
/// setup branch, which holds the configuration before anyone merges it. The
/// setup is complete when that commit's tree holds a valid configuration,
/// read as the canonical reader reads it, that names the same primary
/// branch, and, for a change, whose blob is not the one the change began
/// from. A primary that does not exist, or holds no file, is not complete.
/// Git failing to answer is the error; the caller treats it as pending.
pub(crate) fn is_complete(
    repository: &git2::Repository,
    record: &SetupRecord,
) -> Result<bool, git2::Error> {
    let reference =
        match repository.find_reference(&format!("refs/heads/{}", record.primary_branch)) {
            Ok(reference) => reference,
            Err(error) if error.code() == git2::ErrorCode::NotFound => return Ok(false),
            Err(error) => return Err(error),
        };
    let tree = reference.peel_to_commit()?.tree()?;
    let entry = match tree.get_path(Path::new(canonical::CONFIG_PATH)) {
        Ok(entry) => entry,
        Err(error) if error.code() == git2::ErrorCode::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    let Ok(blob) = repository.find_blob(entry.id()) else {
        // A directory, or a submodule, where the file should be.
        return Ok(false);
    };
    let names_the_primary = std::str::from_utf8(blob.content())
        .ok()
        .and_then(|source| canonical::parse_repository_config(source).ok())
        .is_some_and(|configuration| configuration.primary_branch == record.primary_branch);
    Ok(names_the_primary && record.base_configuration_oid != Some(entry.id()))
}
