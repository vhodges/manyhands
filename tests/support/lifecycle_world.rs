//! The worlds of the lifecycle rows: a document or a ticket with an editing
//! context, in the two-clone world of `remote_world` or in a repository with
//! no publication remote at all.
//!
//! `tests/ticket_closure.rs` and `tests/document_promotion.rs` keep their
//! helpers private, and small duplication is accepted over moving them
//! (design, Testing).

use crate::{
    mutation_golden, record,
    remote_world::{BODY, ITEM, Provider, World},
    repository::{keys::*, transport::*, *},
    ssh_harness,
    ssh_remote::*,
};
use std::path::{Path, PathBuf};

/// A case per row and verb, each named `<prefix><row>_promote` or `_close`.
macro_rules! both_verbs {
    ($prefix:literal; $( $name:literal => $row:expr ),+ $(,)?) => {
        &[$(
            (concat!($prefix, $name, "_promote"), || $row(Verb::Promote)),
            (concat!($prefix, $name, "_close"), || $row(Verb::Close)),
        )+]
    };
}
pub(crate) use both_verbs;

/// The world's document.
pub const DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FDC";
pub const DOCUMENT_PATH: &str = "docs/lifecycle.md";
pub const DOCUMENT_BRANCH: &str = "refs/heads/manyhands/document/01ARZ3NDEKTSV4RRFFQ69G5FDC";
pub const TICKET_BRANCH: &str = "refs/heads/manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAW";
pub const MAIN: &str = "refs/heads/main";
/// What a preview and a result may never hold, as a world's `BODY` is.
pub const TITLE: &str = "lifecycle-title-private-sentinel";

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
                    title: TITLE.into(),
                    body: BODY.into(),
                },
                expected_source: None,
                expected_destination: ExpectedPathObservation::Missing,
            }),
            Self::Close => service.save_ticket(SaveTicketRequest {
                target,
                draft: TicketDraft {
                    title: TITLE.into(),
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
/// remote holds it, and which holds the verb's item, saved and unpublished. Its
/// key needs no passphrase, which saves a row about nine seconds of its fifteen.
pub fn remote(verb: Verb) -> Result<World, FixtureError> {
    ready(World::plain_key()?, verb)
}

/// `remote`, for a row that asks about unlocking, or that stops a child from
/// inside the credentials it asks for: the world's key asks for its passphrase.
pub fn protected(verb: Verb) -> Result<World, FixtureError> {
    ready(World::new()?, verb)
}

fn ready(w: World, verb: Verb) -> Result<World, FixtureError> {
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

/// The verb's parent table.
pub fn table(verb: Verb) -> &'static str {
    match verb {
        Verb::Promote => "document_promotions",
        Verb::Close => "ticket_closures",
    }
}

/// The synchronization target of the verb's context.
pub fn context_target(verb: Verb) -> Result<SynchronizationTarget, FixtureError> {
    Ok(SynchronizationTarget::Context {
        kind: match verb {
            Verb::Promote => AuthoringKind::Document,
            Verb::Close => AuthoringKind::Ticket,
        },
        item_id: fixed(verb.id().parse())?,
    })
}

/// Two case lists as one.
pub const fn joined_cases<const N: usize>(
    first: &[ssh_harness::Case],
    second: &[ssh_harness::Case],
) -> [ssh_harness::Case; N] {
    let mut all = [first[0]; N];
    let mut at = 0;
    while at < first.len() {
        all[at] = first[at];
        at += 1;
    }
    while at < N {
        all[at] = second[at - first.len()];
        at += 1;
    }
    all
}

/// The commits on primary that merged a context: one, however many calls.
pub fn merges_on_primary(repo: &git2::Repository) -> Result<usize, FixtureError> {
    let mut walk = fixed(repo.revwalk())?;
    fixed(walk.push(fixed(repo.refname_to_id(MAIN))?))?;
    // Primary's own line: a context's synchronization merges primary into it,
    // on the side.
    fixed(walk.simplify_first_parent())?;
    let mut merges = 0;
    for oid in walk {
        if fixed(repo.find_commit(fixed(oid)?))?.parent_count() == 2 {
            merges += 1;
        }
    }
    Ok(merges)
}

/// The state of the request's record, if it has one.
pub fn state_of(w: &World, id: &str) -> Option<RequestState> {
    record(w, id).map(|record| record.state)
}

/// A service with no failure staged, over the same data.
pub fn clean_service(data: &Path) -> Result<RepositoryService, FixtureError> {
    fixed(RepositoryService::open_at(data))
}

/// A service that stops at `point`, over the same data.
pub fn failing_service(
    data: &Path,
    point: FailurePoint,
) -> Result<RepositoryService, FixtureError> {
    fixed(RepositoryService::open_at_with_failure_point_for_testing(
        data, point,
    ))
}

/// A local world's application data.
pub fn local_data(local: &Local) -> std::path::PathBuf {
    local.directory.path().join("data")
}

/// The result against its golden, with what a run makes up replaced.
pub fn golden_at(
    root: &std::path::Path,
    name: &str,
    data_schema: Option<&str>,
    envelope: &impl serde::Serialize,
) {
    let value = serde_json::to_value(envelope).unwrap();
    let mut replaced: Vec<(String, &str)> = Vec::new();
    for (pointer, placeholder) in [
        ("/data/confirmation_id", "<confirmation>"),
        ("/data/expires_at", "<expires>"),
        ("/data/details/context_oid", "<context>"),
        ("/data/details/primary_oid", "<primary>"),
        ("/data/checkpoint_oid", "<checkpoint>"),
        ("/data/integration_oid", "<integration>"),
        ("/data/publication_oid", "<publication>"),
        ("/effects/commit_oid", "<integration>"),
    ] {
        if let Some(found) = value.pointer(pointer).and_then(|found| found.as_str()) {
            replaced.push((found.to_owned(), placeholder));
        }
    }
    let replaced: Vec<(&str, &str)> = replaced
        .iter()
        .map(|(found, placeholder)| (found.as_str(), *placeholder))
        .collect();
    let root = root.canonicalize().unwrap();
    mutation_golden::assert_contract_at(
        name,
        root.to_str().unwrap(),
        data_schema,
        &replaced,
        &value,
    );
}

/// The golden behind a lifecycle row of the CLI contract RFC's recovery tables
/// (decision 4), `ticket_close_<row>`. The rows reach `LifecycleBinding`'s
/// shared code, so the close's golden stands for both verbs (decision 27),
/// and a promotion's run asserts the same answer without one.
pub fn recovery_golden(
    verb: Verb,
    root: &Path,
    row: &str,
    envelope: &crate::Envelope<MutationDataDto>,
) {
    if verb == Verb::Close {
        golden_at(
            root,
            &format!("{}_{row}", verb.golden()),
            envelope
                .data
                .as_ref()
                .map(|_| "lifecycle_mutation.schema.json"),
            envelope,
        );
    }
}
