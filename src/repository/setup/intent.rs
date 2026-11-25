//! What a first-time setup intends (Wave 06 Cycle 02, design Part E, "The
//! mode, for a repository without configuration"): the mode and the file
//! an enable writes, whether it commits the file to the primary at once or
//! puts it on a setup branch for a pull request.

use crate::{
    canonical::{self, ForgeSetting, Integration, RepositoryConfig},
    forge,
    results::ResultCode,
};

/// The mode a first-time enable has and the file it writes, before the short
/// code prefix is put in.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Intended {
    pub(crate) integration: Integration,
    pub(crate) configuration: RepositoryConfig,
}

/// Why an intent is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IntentError {
    /// A pull-request mode with no remote, or an invalid forge.
    InvalidInput,
    /// A remote that has no fetch URL: it is not a remote of the repository.
    InvalidRemote,
}

impl IntentError {
    pub(crate) const fn code(self) -> ResultCode {
        match self {
            Self::InvalidInput => ResultCode::InvalidInput,
            Self::InvalidRemote => ResultCode::InvalidRemote,
        }
    }
}

/// The mode: `integration` when given; otherwise the forge's default for the
/// remote's fetch URL when a remote is given; otherwise direct. The file is
/// built with the primary branch, the remote, the mode and the `[forge]`
/// table and validated by the serializer, so a pull-request mode with no
/// remote and an invalid forge are refused before anything is written.
pub(crate) fn for_enable(
    primary_branch: &str,
    publication_remote: Option<&str>,
    integration: Option<Integration>,
    forge_setting: Option<&ForgeSetting>,
    fetch_url: Option<&str>,
) -> Result<Intended, IntentError> {
    if publication_remote.is_some() && fetch_url.is_none() {
        return Err(IntentError::InvalidRemote);
    }
    let integration = match (integration, publication_remote) {
        (Some(integration), _) => integration,
        (None, Some(_)) => forge::default_integration(fetch_url, forge_setting),
        (None, None) => Integration::Direct,
    };
    let configuration = base_configuration(
        primary_branch,
        publication_remote,
        integration,
        forge_setting,
    );
    canonical::serialize_repository_config(&configuration)
        .map_err(|_| IntentError::InvalidInput)?;
    Ok(Intended {
        integration,
        configuration,
    })
}

/// The file's table for the mode and the forge, not yet validated: the
/// serializer does that, as for every configuration. A direct file is
/// today's, with no mode key.
pub(crate) fn base_configuration(
    primary_branch: &str,
    publication_remote: Option<&str>,
    integration: Integration,
    forge_setting: Option<&ForgeSetting>,
) -> RepositoryConfig {
    let mut unknown = toml::Table::new();
    if integration == Integration::PullRequest {
        unknown.insert(
            "integration".to_owned(),
            toml::Value::String(integration.as_str().to_owned()),
        );
    }
    if let Some(setting) = forge_setting {
        let mut table = toml::Table::new();
        if let Some(kind) = setting.kind {
            table.insert(
                "kind".to_owned(),
                toml::Value::String(kind.as_str().to_owned()),
            );
        }
        if let Some(url) = &setting.web_url {
            table.insert("web_url".to_owned(), toml::Value::String(url.clone()));
        }
        if let Some(url) = &setting.pull_request_url {
            table.insert(
                "pull_request_url".to_owned(),
                toml::Value::String(url.clone()),
            );
        }
        unknown.insert("forge".to_owned(), toml::Value::Table(table));
    }
    RepositoryConfig {
        primary_branch: primary_branch.to_owned(),
        publication_remote: publication_remote.map(str::to_owned),
        unknown,
    }
}

#[cfg(test)]
#[path = "intent_tests.rs"]
mod tests;
