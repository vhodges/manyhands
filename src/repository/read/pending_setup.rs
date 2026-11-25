//! A repository's pending setup, as `repo list` and `repo inspect` report it.

use rusqlite::Connection;

use super::{ProblemDto, PullRequestDto, ReadError, RepositorySetupDto};
use crate::{
    canonical, forge,
    repository::setup::record::{self, SetupRecord},
    results::{ProblemCode, ResultCode},
};

/// `read_pending_setup`, for a read that must answer whatever the record
/// holds: a record this build cannot read (`recovery_required`, or the
/// internal error a corrupt row converts to) reads as no setup, with the
/// `setup_unreadable` problem for the caller to report. A busy or
/// unavailable index still refuses.
pub(super) fn read_setup_or_problem(
    connection: &Connection,
    repository_id: i64,
    fetch_url: impl FnOnce(&str) -> Option<String>,
) -> Result<(Option<RepositorySetupDto>, Option<ProblemDto>), ReadError> {
    match read_pending_setup(connection, repository_id, fetch_url) {
        Ok(setup) => Ok((setup, None)),
        Err(error)
            if matches!(
                error.code(),
                ResultCode::RecoveryRequired | ResultCode::InternalError
            ) =>
        {
            Ok((
                None,
                Some(ProblemDto {
                    code: ProblemCode::SetupUnreadable,
                    path: None,
                    target_id: None,
                }),
            ))
        }
        Err(error) => Err(error),
    }
}

/// The setup of the registration `repository_id`, if it has one.
///
/// Every record reads as pending here. `fetch_url` is the setup remote's
/// fetch URL when the caller has it: `repo list` reads the index only and has
/// none, so the link of a forge whose address needs the URL has a null `url`.
pub(super) fn read_pending_setup(
    connection: &Connection,
    repository_id: i64,
    fetch_url: impl FnOnce(&str) -> Option<String>,
) -> Result<Option<RepositorySetupDto>, ReadError> {
    let Some(record) = record::read(connection, repository_id)? else {
        return Ok(None);
    };
    let url = fetch_url(&record.remote_name);
    setup_dto(&record, url.as_deref()).map(Some)
}

/// The setup's object. The link is built here, when read, from the record's
/// intended configuration and the remote's fetch URL; the record holds no
/// URL. A configuration that cannot be read is `recovery_required`.
fn setup_dto(
    record: &SetupRecord,
    fetch_url: Option<&str>,
) -> Result<RepositorySetupDto, ReadError> {
    let unreadable = || ReadError::new(ResultCode::RecoveryRequired);
    let configuration = std::str::from_utf8(&record.configuration)
        .ok()
        .and_then(|text| canonical::parse_repository_config(text).ok())
        .ok_or_else(unreadable)?;
    let branch = format!("manyhands/setup/{}", record.setup_id);
    let forge = forge::recognise(fetch_url, configuration.forge().as_ref());
    let link = forge::pull_request_link(forge.as_ref(), &branch, &record.primary_branch);
    Ok(RepositorySetupDto {
        id: record.setup_id.to_string(),
        kind: record.kind.as_str().to_owned(),
        state: record.state().as_str().to_owned(),
        request_id: record.request_id.to_string(),
        branch,
        remote: record.remote_name.clone(),
        integration: configuration.integration().as_str().to_owned(),
        pull_request: PullRequestDto::from(&link),
    })
}
