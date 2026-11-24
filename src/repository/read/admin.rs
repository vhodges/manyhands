//! Repository administration reads: the registrations, one repository's
//! inspection, its commit identity and its remotes.

use std::path::Path;

use git2::{Config, ConfigLevel, Repository};
use time::OffsetDateTime;

use super::{
    Accessibility, ConfigurationDto, ConfigurationState, ForgeDto, IdentityAvailability,
    IdentityDto, IdentitySource, InitialsSource, ProblemDto, ReadError, RemoteDto, RemoteListDto,
    RepositoryInspectionDto, RepositoryListDto, RepositorySummaryDto, ResolvedRepository,
    index_state,
    resolve::{at_root, open_exactly, selected_repository},
    status::index_problems,
};
use crate::{
    canonical,
    repository::{
        ConfigurationInspection, DirtyContexts, IdentityConfigProvider, IdentityInspection,
        RemoteInfo, RepositoryError, RepositoryIdentityConfig, RepositoryOperation,
        RepositoryService, SuppliedIdentityConfig, bridges::identity::local_initials,
        read_configuration_guarded, remote_info_for, resolve_identity,
    },
    results::{
        ProblemCode, REDACTED, ResultCode, absolute_path_string, redact_url, relative_path_string,
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
        target_id: None,
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
        integration: None,
        forge: None,
    }
}

/// A valid configuration's object: `configuration_dto` with its mode and
/// forge.
fn valid_configuration_dto(
    primary_branch: String,
    publication_remote: Option<String>,
    integration: Option<String>,
    forge: Option<ForgeDto>,
) -> ConfigurationDto {
    ConfigurationDto {
        integration,
        forge,
        ..configuration_dto(
            ConfigurationState::Valid,
            Some(primary_branch),
            publication_remote,
            None,
        )
    }
}

/// `value` when it is the contract spelling of a mode this build knows.
fn known_integration(value: &str) -> Option<String> {
    use canonical::Integration::{Direct, PullRequest};
    [Direct.as_str(), PullRequest.as_str()]
        .contains(&value)
        .then(|| value.to_owned())
}

/// `value` when it is the contract spelling of a forge kind this build knows.
fn known_forge_kind(value: &str) -> Option<String> {
    use canonical::ForgeKind::{BitbucketCloud, BitbucketDataCenter, Github, Gitlab};
    [
        BitbucketCloud.as_str(),
        BitbucketDataCenter.as_str(),
        Github.as_str(),
        Gitlab.as_str(),
    ]
    .contains(&value)
    .then(|| value.to_owned())
}

/// The mode and `[forge]` columns of a stored configuration row:
/// `persist_configuration` sets them for a valid configuration and leaves
/// them null for any other row.
#[derive(Default)]
pub(super) struct StoredMode {
    pub(super) integration: Option<String>,
    pub(super) forge_kind: Option<String>,
    pub(super) forge_web_url: Option<String>,
    pub(super) forge_pull_request_url: Option<String>,
}

/// A stored forge: its kind, web URL and pull request URL template.
pub(super) type StoredForge = (Option<String>, Option<String>, Option<String>);

impl StoredMode {
    /// What a valid row's columns read as: its mode, and its forge when it
    /// has one. A valid row whose mode is null was stored before the columns
    /// existed and not refreshed since. No build before them wrote either
    /// key, so it reads as direct with no forge.
    pub(super) fn read(self) -> (String, Option<StoredForge>) {
        let Some(integration) = self.integration else {
            return (canonical::Integration::Direct.as_str().to_owned(), None);
        };
        let forge = (self.forge_kind.is_some()
            || self.forge_web_url.is_some()
            || self.forge_pull_request_url.is_some())
        .then_some((
            self.forge_kind,
            self.forge_web_url,
            self.forge_pull_request_url,
        ));
        (integration, forge)
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
    mode: StoredMode,
) -> ConfigurationDto {
    match (state, primary_branch) {
        (None | Some("missing"), _) => {
            configuration_dto(ConfigurationState::Missing, None, None, None)
        }
        (Some("valid"), Some(primary_branch)) => {
            let (integration, forge) = mode.read();
            // A value a later build wrote, which this one does not know,
            // reads as null, so the object always satisfies the schema.
            valid_configuration_dto(
                primary_branch,
                publication_remote,
                known_integration(&integration),
                forge.map(|(kind, web_url, pull_request_url)| ForgeDto {
                    kind: kind.and_then(|kind| known_forge_kind(&kind)),
                    web_url,
                    pull_request_url,
                }),
            )
        }
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
        ConfigurationInspection::Valid(configuration) => {
            let integration = configuration.integration().as_str().to_owned();
            let forge = configuration.forge().map(|forge| ForgeDto {
                kind: forge.kind.map(|kind| kind.as_str().to_owned()),
                web_url: forge.web_url,
                pull_request_url: forge.pull_request_url,
            });
            valid_configuration_dto(
                configuration.primary_branch,
                configuration.publication_remote,
                Some(integration),
                forge,
            )
        }
        ConfigurationInspection::Invalid(problem) => configuration_dto(
            ConfigurationState::Invalid,
            None,
            None,
            Some(ProblemCode::from(&problem.code)),
        ),
    }
}

/// Whether Git reaches the remote `name` through a remote helper it is
/// configured with, `remote.<name>.vcs`. Its locations are then whatever
/// that program takes, so nothing about their form says what they hold.
/// A configuration that cannot be read is taken to configure one.
fn has_remote_helper(repository: &Repository, name: &str) -> bool {
    match repository.config().and_then(|mut config| config.snapshot()) {
        Ok(config) => config.get_entry(&format!("remote.{name}.vcs")).is_ok(),
        Err(_) => true,
    }
}

/// Remotes in name order, their locations redacted: each in part, as
/// `redact_url` does it, or whole for a remote with a helper configured.
fn remote_dtos(
    repository: &Repository,
    remotes: Vec<RemoteInfo>,
    publication_remote: Option<&str>,
) -> Vec<RemoteDto> {
    let location = |helper: bool, url: &str| {
        if helper {
            REDACTED.to_owned()
        } else {
            redact_url(url)
        }
    };
    let mut remotes: Vec<_> = remotes
        .into_iter()
        .map(|remote| (has_remote_helper(repository, &remote.name), remote))
        .map(|(helper, remote)| RemoteDto {
            fetch_location: location(helper, &remote.fetch_url),
            push_location: location(helper, &remote.push_url),
            publication_eligible: remote.publication_eligible,
            selected_for_publication: publication_remote == Some(remote.name.as_str()),
            name: remote.name,
        })
        .collect();
    remotes.sort_by(|left, right| left.name.cmp(&right.name));
    remotes
}

/// The identity a commit in `repository` would carry, with the level of
/// configuration it comes from, and the initials a short code would be
/// composed with. The repository's own configuration is read first for
/// both; `effective_config` is what Git finds at every level.
pub(crate) fn identity_dto(
    repository: &Repository,
    root: &Path,
    effective_config: &Config,
    operation: RepositoryOperation,
) -> Result<IdentityDto, ReadError> {
    let git = |error| RepositoryError::git(operation, Some(root.to_owned()), error);
    let local_config = repository.config().map_err(git)?;
    let identity = resolve_identity(&local_config, effective_config, root, operation)?;
    let (name, email, source) = match identity {
        Some((identity, level)) => {
            let Some(source) = identity_source(level) else {
                return Err(ReadError::new(ResultCode::InternalError));
            };
            (Some(identity.name), Some(identity.email), source)
        }
        None => (None, None, IdentitySource::None),
    };
    // As a short code is composed: the repository's own initials come
    // first, and ones that do not fit are not replaced by derived ones.
    let (initials, initials_source) = match local_initials(repository).map_err(git)? {
        Some(configured) => match canonical::normalize_initials(&configured) {
            Ok(initials) => (Some(initials), InitialsSource::Repository),
            Err(canonical::InitialsProblem::Invalid) => (None, InitialsSource::None),
        },
        None => match name.as_deref().and_then(canonical::derive_initials) {
            Some(initials) => (Some(initials), InitialsSource::Derived),
            None => (None, InitialsSource::None),
        },
    };
    Ok(IdentityDto {
        name,
        email,
        source,
        initials,
        initials_source,
    })
}

impl RepositoryService {
    /// Every registration, ordered by root.
    ///
    /// This reads the index and nothing else. A root is reported as it was
    /// last stored, whatever has since happened to it on disk. Each counts
    /// the problems its `index status` lists.
    pub fn list_repositories(&self) -> Result<RepositoryListDto, ReadError> {
        self.read_session(RepositoryOperation::Read, |connection| {
            let mut statement = connection.prepare(
                "SELECT repositories.root_path,
                        repositories.enabled_at,
                        repositories.accessibility,
                        repositories.refresh_required,
                        repositories.refreshed_at,
                        EXISTS(SELECT 1 FROM contexts
                                WHERE contexts.repository_id = repositories.id
                                  AND contexts.kind != 'invalid'),
                        configuration.state,
                        configuration.primary_branch,
                        configuration.publication_remote,
                        configuration.invalid_code,
                        repositories.id,
                        configuration.integration,
                        configuration.forge_kind,
                        configuration.forge_web_url,
                        configuration.forge_pull_request_url
                   FROM repositories
                   LEFT JOIN configuration_observations AS configuration
                     ON configuration.repository_id = repositories.id
                  ORDER BY repositories.root_path ASC",
            )?;
            let rows = statement
                .query_map([], |row| {
                    let root: String = row.get(0)?;
                    let enabled_at: i64 = row.get(1)?;
                    let accessibility: String = row.get(2)?;
                    let refresh_required: bool = row.get(3)?;
                    let state: Option<String> = row.get(6)?;
                    let invalid_code: Option<String> = row.get(9)?;
                    let registration = row.get::<_, i64>(10)?;
                    let any_dirty = DirtyContexts::read(connection, registration)?.any();
                    Ok((
                        RepositorySummaryDto {
                            root: root.clone(),
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
                                StoredMode {
                                    integration: row.get(11)?,
                                    forge_kind: row.get(12)?,
                                    forge_web_url: row.get(13)?,
                                    forge_pull_request_url: row.get(14)?,
                                },
                            ),
                            index: index_state(
                                refresh_required,
                                row.get(4)?,
                                row.get(5)?,
                                any_dirty,
                            ),
                            problem_count: 0,
                        },
                        ResolvedRepository::registered(registration, root.into()),
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            let mut items = Vec::new();
            for (mut item, repo) in rows {
                item.problem_count =
                    u64::try_from(index_problems(connection, &repo)?.len()).unwrap_or_default();
                items.push(item);
            }
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
        let registration = self.registration_id(root)?;
        let registered = registration.is_some();
        let inspection = self.inspect_reporting_detached_head(root)?;
        // A root that cannot be written as text cannot be enabled either.
        let (Some(selected_path), Some(root)) = (
            absolute_path_string(selected),
            absolute_path_string(&inspection.root),
        ) else {
            return Err(ReadError::invalid_path());
        };
        let mut configuration = inspected_configuration(inspection.configuration);
        // The inspection opened the root; it is opened again here only to
        // ask whether each remote has a helper configured.
        let repository = open_exactly(Path::new(&root)).map_err(|error| {
            ReadError::new(ResultCode::RepositoryInaccessible).with_source(error)
        })?;
        // The root as the registry stores it, which is the one the
        // registration was looked up by.
        //
        // This is the verb every `recovery_required` points at, so rows that
        // cannot be read do not refuse it: the list is empty and a problem
        // says why, so that an empty list is not taken for nothing being
        // unfinished. A busy or unavailable index is still refused.
        let operations = match self.inspected_operations(Path::new(&root), registration) {
            Ok(operations) => operations,
            Err(error)
                if matches!(
                    error.code(),
                    ResultCode::RecoveryRequired | ResultCode::InternalError
                ) =>
            {
                configuration.problems.push(ProblemDto {
                    code: ProblemCode::OperationsUnreadable,
                    path: None,
                    target_id: None,
                });
                Vec::new()
            }
            Err(error) => return Err(error),
        };
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
                &repository,
                inspection.remotes,
                configuration.publication_remote.as_deref(),
            ),
            configuration,
            operations,
        })
    }

    /// The identity a commit in this repository would carry, and the Git
    /// configuration level it comes from.
    ///
    /// This reads Git configuration only and uses no read session. It is
    /// never `busy` under the exclusive index lock, and it does not check
    /// that the registration still exists: for a repository removed from
    /// Manyhands since it was resolved it still answers.
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
        let effective_config = identity_config
            .effective_config(&repository, repo.root(), operation)
            .map_err(|error| repo.failure(error))?;
        identity_dto(&repository, repo.root(), &effective_config, operation)
            .map_err(|error| repo.failure(error))
    }

    /// The repository's remotes in name order, with any credential removed
    /// from their locations, and which of them publication uses. Nothing
    /// is contacted.
    ///
    /// This reads Git and the configuration file only and uses no read
    /// session. It is never `busy` under the exclusive index lock, and it
    /// does not check that the registration still exists: for a repository
    /// removed from Manyhands since it was resolved it still answers.
    pub fn list_remotes_redacted(
        &self,
        repo: &ResolvedRepository,
    ) -> Result<RemoteListDto, ReadError> {
        let operation = RepositoryOperation::Read;
        let repository = repo.open()?;
        let remotes = remote_info_for(&repository, repo.root(), operation)
            .map_err(|error| repo.failure(error))?;
        let configuration = read_configuration_guarded(repo.root(), operation)
            .map_err(|error| repo.failure(error))?;
        let publication_remote = match &configuration {
            ConfigurationInspection::Valid(configuration) => {
                configuration.publication_remote.as_deref()
            }
            ConfigurationInspection::Missing | ConfigurationInspection::Invalid(_) => None,
        };
        Ok(RemoteListDto {
            items: remote_dtos(&repository, remotes, publication_remote),
            complete: true,
        })
    }
}
