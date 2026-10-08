//! Repository administration reads: the registrations, one repository's
//! inspection, its commit identity and its remotes.

use std::path::Path;

use git2::{Config, ConfigLevel};
use time::OffsetDateTime;

use super::{
    Accessibility, ConfigurationDto, ConfigurationState, IdentityAvailability, IdentityDto,
    IdentitySource, ProblemDto, ReadError, RemoteDto, RemoteListDto, RepositoryInspectionDto,
    RepositoryListDto, RepositorySummaryDto, ResolvedRepository, index_state,
    resolve::{at_root, selected_repository},
};
use crate::{
    canonical,
    repository::{
        ConfigurationInspection, IdentityConfigProvider, IdentityInspection, RemoteInfo,
        RepositoryError, RepositoryIdentityConfig, RepositoryOperation, RepositoryService,
        SuppliedIdentityConfig, read_configuration_for, remote_info_for, resolve_identity,
    },
    results::{
        ProblemCode, ResultCode, absolute_path_string, redact_url, relative_path_string,
        timestamp_string,
    },
};

/// The contract name of the configuration level an identity was found at,
/// or `None` for a level identity resolution does not read.
pub(super) fn identity_source(level: ConfigLevel) -> Option<IdentitySource> {
    match level {
        ConfigLevel::Local => Some(IdentitySource::Repository),
        ConfigLevel::Global => Some(IdentitySource::Global),
        ConfigLevel::XDG => Some(IdentitySource::Xdg),
        ConfigLevel::System => Some(IdentitySource::System),
        ConfigLevel::ProgramData => Some(IdentitySource::ProgramData),
        ConfigLevel::App => Some(IdentitySource::Application),
        ConfigLevel::Worktree | ConfigLevel::Highest => None,
    }
}

/// A problem with the repository's configuration file. The problem's own
/// text is not published; its code selects the guidance.
fn configuration_problem(code: ProblemCode) -> ProblemDto {
    ProblemDto {
        code,
        path: relative_path_string(Path::new(canonical::CONFIG_PATH)),
    }
}

fn configuration_dto(
    state: ConfigurationState,
    primary_branch: Option<String>,
    publication_remote: Option<String>,
    problem: Option<ProblemCode>,
) -> ConfigurationDto {
    ConfigurationDto {
        state,
        primary_branch,
        publication_remote,
        problems: problem.map(configuration_problem).into_iter().collect(),
    }
}

/// The configuration the index stored for a registration. A registration
/// with no observation is `missing`, as the snapshot reads it; a row this
/// build cannot read is `invalid` with an unknown problem.
pub(super) fn stored_configuration(
    state: Option<&str>,
    primary_branch: Option<String>,
    publication_remote: Option<String>,
    invalid_code: Option<&str>,
) -> ConfigurationDto {
    match (state, primary_branch) {
        (None | Some("missing"), _) => {
            configuration_dto(ConfigurationState::Missing, None, None, None)
        }
        (Some("valid"), Some(primary_branch)) => configuration_dto(
            ConfigurationState::Valid,
            Some(primary_branch),
            publication_remote,
            None,
        ),
        (Some("invalid"), _) => configuration_dto(
            ConfigurationState::Invalid,
            None,
            None,
            Some(ProblemCode::from_stored(invalid_code.unwrap_or_default())),
        ),
        (Some(_), _) => configuration_dto(
            ConfigurationState::Invalid,
            None,
            None,
            Some(ProblemCode::UnknownProblem),
        ),
    }
}

fn inspected_configuration(configuration: ConfigurationInspection) -> ConfigurationDto {
    match configuration {
        ConfigurationInspection::Missing => {
            configuration_dto(ConfigurationState::Missing, None, None, None)
        }
        ConfigurationInspection::Valid(configuration) => configuration_dto(
            ConfigurationState::Valid,
            Some(configuration.primary_branch),
            configuration.publication_remote,
            None,
        ),
        ConfigurationInspection::Invalid(problem) => configuration_dto(
            ConfigurationState::Invalid,
            None,
            None,
            Some(ProblemCode::from(&problem.code)),
        ),
    }
}

/// Remotes in name order, their locations redacted.
fn remote_dtos(remotes: Vec<RemoteInfo>, publication_remote: Option<&str>) -> Vec<RemoteDto> {
    let mut remotes: Vec<_> = remotes
        .into_iter()
        .map(|remote| RemoteDto {
            fetch_location: redact_url(&remote.fetch_url),
            push_location: redact_url(&remote.push_url),
            publication_eligible: remote.publication_eligible,
            selected_for_publication: publication_remote == Some(remote.name.as_str()),
            name: remote.name,
        })
        .collect();
    remotes.sort_by(|left, right| left.name.cmp(&right.name));
    remotes
}

impl RepositoryService {
    /// Every registration, ordered by root.
    ///
    /// This reads the index and nothing else. A root is reported as it was
    /// last stored, whatever has since happened to it on disk.
    pub fn list_repositories(&self) -> Result<RepositoryListDto, ReadError> {
        self.read_session(RepositoryOperation::Read, |connection| {
            let mut statement = connection.prepare(
                "SELECT repositories.root_path,
                        repositories.enabled_at,
                        repositories.accessibility,
                        repositories.refresh_required,
                        repositories.refreshed_at,
                        EXISTS(SELECT 1 FROM contexts
                                WHERE contexts.repository_id = repositories.id),
                        configuration.state,
                        configuration.primary_branch,
                        configuration.publication_remote,
                        configuration.invalid_code,
                        (SELECT COUNT(*) FROM problems
                          WHERE problems.repository_id = repositories.id)
                   FROM repositories
                   LEFT JOIN configuration_observations AS configuration
                     ON configuration.repository_id = repositories.id
                  ORDER BY repositories.root_path ASC",
            )?;
            let items = statement
                .query_map([], |row| {
                    let enabled_at: i64 = row.get(1)?;
                    let accessibility: String = row.get(2)?;
                    let refresh_required: bool = row.get(3)?;
                    let state: Option<String> = row.get(6)?;
                    let invalid_code: Option<String> = row.get(9)?;
                    let problem_count: i64 = row.get(10)?;
                    Ok(RepositorySummaryDto {
                        root: row.get(0)?,
                        enabled_at: OffsetDateTime::from_unix_timestamp(enabled_at)
                            .ok()
                            .and_then(timestamp_string),
                        accessibility: match accessibility.as_str() {
                            "accessible" => Accessibility::Accessible,
                            _ => Accessibility::Inaccessible,
                        },
                        configuration: stored_configuration(
                            state.as_deref(),
                            row.get(7)?,
                            row.get(8)?,
                            invalid_code.as_deref(),
                        ),
                        index: index_state(refresh_required, row.get(4)?, row.get(5)?),
                        problem_count: u64::try_from(problem_count).unwrap_or_default(),
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(RepositoryListDto {
                items,
                complete: true,
            })
        })
    }

    /// Inspects the repository `path` selects, whether or not it is
    /// registered, and enables nothing.
    ///
    /// The path is resolved as `resolve_repository` resolves it, short of
    /// requiring a registration: `registered` reports that instead. A linked
    /// worktree is inspected as the repository that owns it; `selected_path`
    /// is the path that was given and `root` the repository's.
    pub fn inspect_repository(&self, path: &Path) -> Result<RepositoryInspectionDto, ReadError> {
        let selected = selected_repository(path)?;
        self.inspect_selected(&selected.selected, &selected.root)
            .map_err(|error| at_root(error, &selected.root))
    }

    fn inspect_selected(
        &self,
        selected: &Path,
        root: &Path,
    ) -> Result<RepositoryInspectionDto, ReadError> {
        let registered = self.registration_id(root)?.is_some();
        let inspection = self.inspect_reporting_detached_head(root)?;
        // A root that cannot be written as text cannot be enabled either.
        let (Some(selected_path), Some(root)) = (
            absolute_path_string(selected),
            absolute_path_string(&inspection.root),
        ) else {
            return Err(ReadError::invalid_path());
        };
        let configuration = inspected_configuration(inspection.configuration);
        Ok(RepositoryInspectionDto {
            selected_path,
            root,
            registered,
            head_branch: inspection.head_branch,
            local_branches: inspection.local_branches,
            identity_state: match inspection.identity {
                IdentityInspection::Available => IdentityAvailability::Available,
                IdentityInspection::Required => IdentityAvailability::Required,
            },
            remotes: remote_dtos(
                inspection.remotes,
                configuration.publication_remote.as_deref(),
            ),
            configuration,
        })
    }

    /// The identity a commit in this repository would carry, and the Git
    /// configuration level it comes from.
    pub fn repository_identity(&self, repo: &ResolvedRepository) -> Result<IdentityDto, ReadError> {
        self.identity_with_provider(repo, &mut RepositoryIdentityConfig)
    }

    #[doc(hidden)]
    pub fn repository_identity_with_config_for_testing(
        &self,
        repo: &ResolvedRepository,
        effective_config: &mut Config,
    ) -> Result<IdentityDto, ReadError> {
        self.identity_with_provider(repo, &mut SuppliedIdentityConfig { effective_config })
    }

    fn identity_with_provider(
        &self,
        repo: &ResolvedRepository,
        identity_config: &mut impl IdentityConfigProvider,
    ) -> Result<IdentityDto, ReadError> {
        let operation = RepositoryOperation::Read;
        let repository = repo.open()?;
        let mut resolve = || {
            let local_config = repository.config().map_err(|error| {
                RepositoryError::git(operation, Some(repo.root().to_owned()), error)
            })?;
            let effective_config =
                identity_config.effective_config(&repository, repo.root(), operation)?;
            resolve_identity(&local_config, &effective_config, repo.root(), operation)
        };
        let Some((identity, level)) = resolve().map_err(|error| repo.failure(error))? else {
            return Ok(IdentityDto {
                name: None,
                email: None,
                source: IdentitySource::None,
            });
        };
        let Some(source) = identity_source(level) else {
            return Err(repo.failure(ReadError::new(ResultCode::InternalError)));
        };
        Ok(IdentityDto {
            name: Some(identity.name),
            email: Some(identity.email),
            source,
        })
    }

    /// The repository's remotes in name order, with any credential removed
    /// from their locations, and which of them publication uses. Nothing
    /// is contacted.
    pub fn list_remotes_redacted(
        &self,
        repo: &ResolvedRepository,
    ) -> Result<RemoteListDto, ReadError> {
        let operation = RepositoryOperation::Read;
        let repository = repo.open()?;
        let remotes = remote_info_for(&repository, repo.root(), operation)
            .map_err(|error| repo.failure(error))?;
        let configuration =
            read_configuration_for(repo.root(), operation).map_err(|error| repo.failure(error))?;
        let publication_remote = match &configuration {
            ConfigurationInspection::Valid(configuration) => {
                configuration.publication_remote.as_deref()
            }
            ConfigurationInspection::Missing | ConfigurationInspection::Invalid(_) => None,
        };
        Ok(RemoteListDto {
            items: remote_dtos(remotes, publication_remote),
            complete: true,
        })
    }
}
