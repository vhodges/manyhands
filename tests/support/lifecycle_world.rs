//! The worlds of the lifecycle rows: a document or a ticket with an editing
//! context, in the two-clone world of `remote_world` or in a repository with
//! no publication remote at all.
//!
//! `tests/ticket_closure.rs` and `tests/document_promotion.rs` keep their
//! helpers private, and small duplication is accepted over moving them
//! (design, Testing).
#![allow(dead_code)]

use crate::{
    remote_world::{BODY, ITEM, Provider, World},
    repository::{keys::*, transport::*, *},
    ssh_remote::*,
};
use std::path::{Path, PathBuf};

/// The world's document.
pub const DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FDC";
pub const DOCUMENT_PATH: &str = "docs/lifecycle.md";
pub const DOCUMENT_BRANCH: &str = "refs/heads/manyhands/document/01ARZ3NDEKTSV4RRFFQ69G5FDC";
pub const TICKET_BRANCH: &str = "refs/heads/manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAW";
pub const MAIN: &str = "refs/heads/main";

/// Which of the two verbs a row is of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verb {
    Promote,
    Close,
}

impl Verb {
    pub fn id(self) -> &'static str {
        match self {
            Self::Promote => DOCUMENT,
            Self::Close => ITEM,
        }
    }

    pub fn branch(self) -> &'static str {
        match self {
            Self::Promote => DOCUMENT_BRANCH,
            Self::Close => TICKET_BRANCH,
        }
    }

    /// The other verb's item is of the wrong kind for this one.
    pub fn other(self) -> Self {
        match self {
            Self::Promote => Self::Close,
            Self::Close => Self::Promote,
        }
    }

    pub fn command(self) -> &'static str {
        match self {
            Self::Promote => "document promote",
            Self::Close => "ticket close",
        }
    }

    /// The first part of a golden fixture's name.
    pub fn golden(self) -> &'static str {
        match self {
            Self::Promote => "document_promote",
            Self::Close => "ticket_close",
        }
    }

    pub fn mutation(self, root: &Path, id: &str) -> Mutation {
        let root = root.to_owned();
        let id = id.parse().unwrap();
        match self {
            Self::Promote => Mutation::DocumentPromote(DocumentPromoteInput { root, id }),
            Self::Close => Mutation::TicketClose(TicketCloseInput { root, id }),
        }
    }

    pub fn worktree(self, root: &Path) -> PathBuf {
        root.join(".manyhands/worktrees").join(self.id())
    }

    /// Saves the verb's item: an editing context with a commit primary does
    /// not hold.
    pub fn save(self, service: &RepositoryService, root: &Path) -> Result<(), FixtureError> {
        let target = AuthoringTarget {
            root: root.to_owned(),
            kind: match self {
                Self::Promote => AuthoringKind::Document,
                Self::Close => AuthoringKind::Ticket,
            },
            item_id: fixed(self.id().parse())?,
            intent: ContextIntent::Create,
            operation_id: OperationId::new(),
        };
        let saved = match self {
            Self::Promote => service.save_document(SaveDocumentRequest {
                target,
                source_path: None,
                destination_path: DOCUMENT_PATH.into(),
                draft: DocumentDraft {
                    title: "Fixture".into(),
                    body: BODY.into(),
                },
                expected_source: None,
                expected_destination: ExpectedPathObservation::Missing,
            }),
            Self::Close => service.save_ticket(SaveTicketRequest {
                target,
                draft: TicketDraft {
                    title: "Fixture".into(),
                    body: BODY.into(),
                    ticket_type: "task".into(),
                    status: "open".into(),
                    project: None,
                    team: None,
                },
                expected_path: ExpectedPathObservation::Missing,
            }),
        };
        match fixed(saved)? {
            SaveOutcome::Saved { .. } => Ok(()),
            _ => Err(FixtureError),
        }
    }
}

/// Has the service trust the server's host key, as a host approval does:
/// a lifecycle of the boundary carries no approval of its own.
pub fn trust(w: &World) -> Result<(), FixtureError> {
    fixed(w.service.verify_ssh_transport(
        VerifySshTransportRequest {
            root: w.root.clone(),
            direction: SshDirection::Fetch,
            approval: Some(HostApproval {
                authority: SshAuthority {
                    host: "127.0.0.1".into(),
                    port: w.server.address().port(),
                },
                expected: None,
                presented: w.server.host_identity(),
            }),
        },
        &mut SessionCredentials::new(Provider),
    ))?;
    Ok(())
}

/// A two-clone world whose host is trusted, whose primary is observed as the
/// remote holds it, and which holds the verb's item, saved and unpublished.
pub fn remote(verb: Verb) -> Result<World, FixtureError> {
    let w = World::new()?;
    trust(&w)?;
    // The fixture's seed push does not move the tracking reference.
    fixed(w.sync(w.primary()))?;
    verb.save(&w.service, &w.root)?;
    Ok(w)
}

/// A repository with no publication remote.
pub struct Local {
    pub service: RepositoryService,
    pub root: PathBuf,
    pub directory: tempfile::TempDir,
}

impl Local {
    /// Enabled, with an identity in its own configuration, holding nothing.
    pub fn empty() -> Result<Self, FixtureError> {
        let directory = fixed(tempfile::tempdir())?;
        let root = directory.path().join("local");
        let repo = initialized(&root)?;
        let service = fixed(RepositoryService::open_at(&directory.path().join("data")))?;
        fixed(service.enable(EnableRepositoryRequest {
            root: root.clone(),
            primary_branch: "main".into(),
            identity: None,
            operation_id: OperationId::new(),
        }))?;
        drop(repo);
        Ok(Self {
            service,
            root,
            directory,
        })
    }

    /// `empty`, holding the verb's item, saved.
    pub fn holding(verb: Verb) -> Result<Self, FixtureError> {
        let local = Self::empty()?;
        verb.save(&local.service, &local.root)?;
        Ok(local)
    }

    pub fn repo(&self) -> Result<git2::Repository, FixtureError> {
        fixed(git2::Repository::open(&self.root))
    }

    pub fn db(&self) -> Result<rusqlite::Connection, FixtureError> {
        fixed(rusqlite::Connection::open(
            self.directory.path().join("data").join(REGISTRY_FILE),
        ))
    }
}

/// A Git repository on `main` with one commit and an identity, not enabled.
pub fn initialized(root: &Path) -> Result<git2::Repository, FixtureError> {
    // A case's output is scanned for what it saved, which a world with a
    // server saves too: the source a save writes.
    crate::ssh_privacy::save(&[BODY.as_bytes().to_vec()])?;
    let mut options = git2::RepositoryInitOptions::new();
    options.initial_head("main");
    let repo = fixed(git2::Repository::init_opts(root, &options))?;
    {
        let mut config = fixed(repo.config())?;
        fixed(config.set_str("user.name", "Fixture"))?;
        fixed(config.set_str("user.email", "fixture@example.invalid"))?;
        let tree = fixed(fixed(repo.index())?.write_tree())?;
        let tree = fixed(repo.find_tree(tree))?;
        let sig = fixed(git2::Signature::now("Fixture", "fixture@example.invalid"))?;
        fixed(repo.commit(Some("HEAD"), &sig, &sig, "owned seed", &tree, &[]))?;
    }
    Ok(repo)
}

/// Removes the repository's own identity, which leaves it none: the harness
/// has no global configuration.
pub fn forget_identity(repo: &git2::Repository) -> Result<(), FixtureError> {
    let config = fixed(repo.config())?;
    let mut local = fixed(config.open_level(git2::ConfigLevel::Local))?;
    fixed(local.remove("user.name"))?;
    fixed(local.remove("user.email"))
}

/// Leaves a file primary's worktree does not know, which makes it dirty.
pub fn dirty_primary(root: &Path) -> Result<(), FixtureError> {
    fixed(std::fs::write(root.join("dirty-primary.txt"), b"dirty\n"))
}

/// Moves primary to the verb's branch, as a merge outside Manyhands would,
/// checking its tree out. The branch's commits are then contained.
pub fn fast_forward_primary(
    repo: &git2::Repository,
    verb: Verb,
) -> Result<git2::Oid, FixtureError> {
    let tip = fixed(repo.refname_to_id(verb.branch()))?;
    fixed(repo.checkout_tree(
        &fixed(repo.find_object(tip, None))?,
        Some(git2::build::CheckoutBuilder::new().safe()),
    ))?;
    fixed(repo.reference(MAIN, tip, true, "owned fixture merge"))?;
    Ok(tip)
}

/// Removes the verb's worktree and branch, leaving its commits on primary.
pub fn retire_context(
    repo: &git2::Repository,
    root: &Path,
    verb: Verb,
) -> Result<(), FixtureError> {
    fixed(std::fs::remove_dir_all(verb.worktree(root)))?;
    let worktree = fixed(repo.find_worktree(verb.id()))?;
    let mut prune = git2::WorktreePruneOptions::new();
    prune.valid(true).working_tree(true);
    fixed(worktree.prune(Some(&mut prune)))?;
    fixed(fixed(repo.find_reference(verb.branch()))?.delete())
}
