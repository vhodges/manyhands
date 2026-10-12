//! Local identity: the name, email and initials a repository's own Git
//! configuration holds.

use git2::{Config, ConfigLevel, Repository};

/// The key of the initials a user set for this repository, in its own
/// Git configuration.
const INITIALS_KEY: &str = "manyhands.initials";

/// The repository's own configuration, and nothing of any other level.
fn local_config(repository: &Repository) -> Result<Config, git2::Error> {
    repository.config()?.open_level(ConfigLevel::Local)
}

/// One value of the repository's own configuration, or `None` when it
/// holds none under `key`.
fn local_value(config: &Config, key: &str) -> Result<Option<String>, git2::Error> {
    match config.get_string(key) {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.code() == git2::ErrorCode::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// `manyhands.initials` in the repository's own configuration, and at no
/// other level: a value a user set for another repository, or for every
/// one, is not this repository's.
pub(crate) fn local_initials(repository: &Repository) -> Result<Option<String>, git2::Error> {
    local_value(&local_config(repository)?, INITIALS_KEY)
}
