//! The `setup` data of a mutation's answer, read from the record.

use std::path::Path;

use super::{lock::read_record, publish::Published, record::SetupState};
use crate::repository::{
    MutationDataDto, RepositoryService,
    mutation::SetupMutationDto,
    read::pending_setup::{setup_of, setup_with_url},
};

/// The setup of the repository registered at `root`, as an answer carries it:
/// the record as it stands, with the link built from the remote's fetch URL as
/// `repo inspect` builds it; or, for a setup `published` just made, in state
/// `published` with its link. `None` when there is no record or it cannot be
/// read: the answer then carries no setup.
pub(crate) fn of(
    service: &RepositoryService,
    root: &Path,
    published: Option<&Published>,
) -> Option<MutationDataDto> {
    let (_, record) = read_record(service, root).ok().flatten()?;
    let setup = match published {
        // What was published is the record's own setup.
        Some(published) if published.setup_id != record.setup_id => return None,
        Some(published) => setup_of(&record, SetupState::Published, &published.link),
        None => {
            let url = git2::Repository::open(root).ok().and_then(|repository| {
                repository
                    .find_remote(&record.remote_name)
                    .ok()
                    .and_then(|remote| remote.url().map(str::to_owned))
            });
            setup_with_url(&record, url.as_deref())
        }
    }
    .ok()?;
    Some(MutationDataDto::Setup(Box::new(SetupMutationDto::of(
        setup,
    ))))
}
