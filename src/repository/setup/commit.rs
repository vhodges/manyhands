//! The setup commit: the base's tree with `.manyhands/config.toml` set to
//! the setup's configuration, made without touching the repository's HEAD,
//! index or working tree (design Part D, publishing step 2, R-6).

use git2::{FileMode, ObjectType, Oid, Repository, Signature, Tree};

use crate::{canonical::CONFIG_PATH, repository::CommitIdentity};

/// The message of the first-time setup's commit.
pub(crate) const ENABLE_MESSAGE: &str = "Set up Manyhands";

/// The message of a change's commit.
pub(crate) const CHANGE_MESSAGE: &str = "Change Manyhands setup";

/// Makes the setup commit: one commit on `base`, whose tree is the base's
/// with the configuration file set to `configuration` at mode `100644`,
/// authored and committed by `identity`. No reference moves and nothing is
/// checked out: the caller names the commit by a reference of its own.
///
/// A base with something that is not a directory where `.manyhands` should
/// be is no base to build on: that is an error, not a replacement.
pub(crate) fn build(
    repository: &Repository,
    base: Oid,
    configuration: &[u8],
    message: &str,
    identity: &CommitIdentity,
) -> Result<Oid, git2::Error> {
    let base = repository.find_commit(base)?;
    let blob = repository.blob(configuration)?;
    let components: Vec<&str> = CONFIG_PATH.split('/').collect();
    let tree = with_blob(repository, Some(&base.tree()?), &components, blob)?;
    let tree = repository.find_tree(tree)?;
    let signature = Signature::now(&identity.name, &identity.email)?;
    repository.commit(None, &signature, &signature, message, &tree, &[&base])
}

/// `tree` with `blob` at `path` (a file at mode `100644`), as a new tree.
fn with_blob(
    repository: &Repository,
    tree: Option<&Tree<'_>>,
    path: &[&str],
    blob: Oid,
) -> Result<Oid, git2::Error> {
    let mut builder = repository.treebuilder(tree)?;
    match path {
        [name] => {
            builder.insert(name, blob, i32::from(FileMode::Blob))?;
        }
        [name, rest @ ..] => {
            let child = match tree.and_then(|tree| tree.get_name(name)) {
                None => None,
                Some(entry) if entry.kind() == Some(ObjectType::Tree) => {
                    Some(repository.find_tree(entry.id())?)
                }
                Some(_) => {
                    return Err(git2::Error::from_str(
                        "a file is where the configuration's directory should be",
                    ));
                }
            };
            let child = with_blob(repository, child.as_ref(), rest, blob)?;
            builder.insert(name, child, i32::from(FileMode::Tree))?;
        }
        [] => return Err(git2::Error::from_str("no path to put the configuration at")),
    }
    builder.write()
}

#[cfg(test)]
#[path = "commit_tests.rs"]
mod tests;
