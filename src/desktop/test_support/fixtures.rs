//! Real repositories in temporary directories, made through the library.
//!
//! States that no local library operation produces are made the way the
//! library's own read tests make them: write the file, commit it with
//! `git2`, then refresh through the library. Only this module does that;
//! desktop code never does.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use manyhands::{
    canonical::ItemId,
    repository::{
        CancellationToken, CommitIdentity, ConfirmationId, DocumentCreateInput, DocumentDraft,
        FailurePoint, FolderCreateInput, IndexRefreshInput, Mutation, MutationCall,
        MutationDataDto, NoProgress, OperationAbandonInput, OperationId, OperationResumeInput,
        PrepareDataDto, RelationshipWrite, RepoCreateInput, RepoEnableInput, RepoRemoveInput,
        RepositoryService, RequestId, TicketCreateInput, TicketDraft, TicketSaveInput,
        keys::{PassphraseResponse, SessionCredentialProvider, SessionCredentials, UnlockRequest},
    },
    results::{Envelope, ResultCode},
};

use super::{delaying::DelayingBackend, git_home};

/// The identity every fixture repository is created and committed as.
pub(crate) fn identity() -> CommitIdentity {
    CommitIdentity {
        name: "Ada Example".to_owned(),
        email: "ada@example.invalid".to_owned(),
    }
}

/// A provider with no passphrase to give: nobody is there to ask.
pub(crate) struct NoPassphrase;

impl SessionCredentialProvider for NoPassphrase {
    fn request_passphrase(&mut self, _request: &UnlockRequest) -> PassphraseResponse {
        PassphraseResponse::Unavailable
    }
}

/// A data directory, a home for keys, a parent for repositories and a
/// service over the data directory.
pub(crate) struct Fixture {
    // Fields drop in declaration order: the service closes before its
    // directories are removed, unless a backend still shares it. A test
    // keeps the fixture for as long as anything it built can read.
    service: Arc<RepositoryService>,
    data: tempfile::TempDir,
    home: tempfile::TempDir,
    repositories: tempfile::TempDir,
}

impl Fixture {
    /// Isolates Git first, then makes the directories and opens the
    /// service.
    pub(crate) fn new() -> Self {
        git_home::isolated();
        let data = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let repositories = tempfile::tempdir().unwrap();
        let service = Arc::new(RepositoryService::open_at(data.path()).unwrap());
        Self {
            service,
            data,
            home,
            repositories,
        }
    }

    pub(crate) fn service(&self) -> &RepositoryService {
        &self.service
    }

    /// A delaying backend over this fixture's own service and home: the
    /// desktop under test and the fixture see the same service.
    pub(crate) fn delaying_backend(&self) -> Arc<DelayingBackend> {
        Arc::new(DelayingBackend::new(
            self.service.clone(),
            self.home().to_owned(),
        ))
    }

    pub(crate) fn data_dir(&self) -> &Path {
        self.data.path()
    }

    pub(crate) fn home(&self) -> &Path {
        self.home.path()
    }

    /// Creates and registers a Manyhands repository named `name` through
    /// `prepare` and `execute` of `repo create`, and gives its canonical
    /// root.
    pub(crate) fn create_repository(&self, name: &str) -> PathBuf {
        let root = self.repositories.path().canonicalize().unwrap().join(name);
        let input = || RepoCreateInput {
            root: root.clone(),
            primary_branch: "main".to_owned(),
            identity: Some(identity()),
            slug_prefix: None,
        };
        let prepared = self.service.prepare(&Mutation::RepoCreate(input()));
        assert_eq!(prepared.code, ResultCode::Ok, "{}", prepared.message);
        let Some(PrepareDataDto::Preview(preview)) = prepared.data else {
            panic!("repo create gave no preview");
        };
        let confirmation = ConfirmationId::parse(&preview.confirmation_id).unwrap();
        let created = self.execute(Some(confirmation), Mutation::RepoCreate(input()));
        assert_eq!(created.code, ResultCode::Ok, "{}", created.message);
        // `repo create` commits the configuration without updating Git's
        // index, which then shows it deleted and untracked: the library's
        // own fixtures add it back the same way (`tests/support/items.rs`,
        // `commit`). Without this, `ticket create` answers
        // `worktree_not_clean` and `commit_file` would commit the
        // configuration's removal. This works around ticket
        // 01M5B0Z6PNP7X4WH4JV3N0VQJ6 ("Enablement must leave the Git index
        // current for the configuration it commits"); remove this reset
        // when that ticket merges.
        let repository = git2::Repository::open(&root).unwrap();
        let head = repository.head().unwrap().peel_to_tree().unwrap();
        let mut index = repository.index().unwrap();
        index.read_tree(&head).unwrap();
        index.write().unwrap();
        root
    }

    /// The canonical directory the fixture's repositories are made in.
    pub(crate) fn parent(&self) -> PathBuf {
        self.repositories.path().canonicalize().unwrap()
    }

    /// Makes the empty directory `name` in `parent()` and gives it.
    pub(crate) fn empty_directory(&self, name: &str) -> PathBuf {
        let path = self.parent().join(name);
        fs::create_dir(&path).unwrap();
        path
    }

    /// Makes a Git repository `name` in `parent()` that is not a Manyhands
    /// repository, as `tests/support` `born_repository` does: `main`
    /// checked out, one commit of `fixture.txt`, made with `git2`. Its own
    /// configuration holds `identity` when given, and no identity
    /// otherwise; the isolated Git home holds none either.
    pub(crate) fn plain_repository(&self, name: &str, identity: Option<CommitIdentity>) -> PathBuf {
        let root = self.parent().join(name);
        let mut options = git2::RepositoryInitOptions::new();
        options.initial_head("main");
        let repository = git2::Repository::init_opts(&root, &options).unwrap();
        let mut config = git2::Config::open(&repository.path().join("config")).unwrap();
        config.set_bool("core.autocrlf", false).unwrap();
        if let Some(identity) = &identity {
            config.set_str("user.name", &identity.name).unwrap();
            config.set_str("user.email", &identity.email).unwrap();
        }
        fs::write(root.join("fixture.txt"), "fixture\n").unwrap();
        let mut index = repository.index().unwrap();
        index.add_path(Path::new("fixture.txt")).unwrap();
        index.write().unwrap();
        let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
        let author = self::identity();
        let signature = git2::Signature::now(&author.name, &author.email).unwrap();
        repository
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                "Initial fixture commit",
                &tree,
                &[],
            )
            .unwrap();
        root
    }

    /// Writes `bytes` at `rel_path` under `root` and commits it on the
    /// checked-out branch with `git2`, as no local operation would.
    pub(crate) fn commit_file(&self, root: &Path, rel_path: &str, bytes: &[u8]) {
        let path = root.join(rel_path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, bytes).unwrap();
        let repository = git2::Repository::open(root).unwrap();
        let mut index = repository.index().unwrap();
        index.add_path(Path::new(rel_path)).unwrap();
        index.write().unwrap();
        let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
        let parent = repository.head().unwrap().peel_to_commit().unwrap();
        let identity = identity();
        let signature = git2::Signature::now(&identity.name, &identity.email).unwrap();
        repository
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                &format!("Add {rel_path}"),
                &tree,
                &[&parent],
            )
            .unwrap();
    }

    /// Creates a ticket with `id`, `title` and the status text `status`
    /// in `root` through `execute` of `ticket create`. It is open: no
    /// local operation closes a ticket.
    pub(crate) fn create_ticket(&self, root: &Path, id: &str, title: &str, status: &str) {
        self.create_ticket_with_deps(root, id, title, status, &[]);
    }

    /// Creates a ticket as `create_ticket` does, depending on `deps`
    /// through the create's own `deps`. The library accepts a dependency
    /// no context holds, which it reports unresolved, and refuses one
    /// that would close a cycle: `commit_dependency_cycle` makes that.
    pub(crate) fn create_ticket_with_deps(
        &self,
        root: &Path,
        id: &str,
        title: &str,
        status: &str,
        deps: &[&str],
    ) {
        self.create_related_ticket(root, id, title, status, deps, None);
    }

    /// Creates a ticket as `create_ticket_with_deps` does, with `parent`
    /// through the create's own `parent`. Its short code is the one the
    /// create derives.
    pub(crate) fn create_related_ticket(
        &self,
        root: &Path,
        id: &str,
        title: &str,
        status: &str,
        deps: &[&str],
        parent: Option<&str>,
    ) {
        let created = self.execute(
            None,
            Mutation::TicketCreate(TicketCreateInput {
                root: root.to_owned(),
                id: item_id(id),
                draft: TicketDraft {
                    title: title.to_owned(),
                    ticket_type: "task".to_owned(),
                    status: status.to_owned(),
                    project: None,
                    team: None,
                    body: format!("Body of {title}.\n"),
                },
                deps: deps.iter().map(|dep| (*dep).to_owned()).collect(),
                parent: parent.map(str::to_owned),
                initials: None,
            }),
        );
        assert_eq!(created.code, ResultCode::Ok, "{}", created.message);
    }

    /// Commits two open tickets, `a` and `b`, to `root`'s primary branch
    /// carrying the same short code `slug`, as `tests/read_relationships.rs`
    /// makes a shared one: `ticket slug assign` refuses a code another
    /// ticket has, and create derives its own. Refresh to index them.
    pub(crate) fn commit_duplicate_slug(
        &self,
        root: &Path,
        a: (&str, &str),
        b: (&str, &str),
        slug: &str,
    ) {
        for (id, title) in [a, b] {
            self.commit_ticket(root, id, title, &format!("slug: {slug}\n"));
        }
    }

    /// Commits two open tickets, `a` and `b`, each depending on the other,
    /// to `root`'s primary branch, as `tests/read_relationships.rs` makes a
    /// cycle: create and save refuse a write that closes one. Refresh to
    /// index them.
    pub(crate) fn commit_dependency_cycle(&self, root: &Path, a: (&str, &str), b: (&str, &str)) {
        self.commit_dependency_ring(root, &[a, b]);
    }

    /// Commits the open tickets `ring`, each depending on the next and the
    /// last on the first, by the same recipe as `commit_dependency_cycle`.
    /// A ring of more than sixteen gives a cycle reason the library marks
    /// incomplete. Refresh to index them.
    pub(crate) fn commit_dependency_ring(&self, root: &Path, ring: &[(&str, &str)]) {
        for (ix, (id, title)) in ring.iter().enumerate() {
            let (next, _) = ring[(ix + 1) % ring.len()];
            self.commit_ticket(root, id, title, &format!("deps:\n  - {next}\n"));
        }
    }

    /// Creates a document with `id` and `title` at `path`, below `docs/`,
    /// in `root` through `execute` of `document create`.
    pub(crate) fn create_document(&self, root: &Path, id: &str, path: &str, title: &str) {
        let created = self.execute(
            None,
            Mutation::DocumentCreate(DocumentCreateInput {
                root: root.to_owned(),
                id: item_id(id),
                path: PathBuf::from(path),
                draft: DocumentDraft {
                    title: title.to_owned(),
                    body: format!("# {title}\n"),
                },
            }),
        );
        assert_eq!(created.code, ResultCode::Ok, "{}", created.message);
    }

    /// Makes the folder `path`, below `docs/`, in `root`'s primary
    /// worktree through `execute` of `folder create`, which also refreshes
    /// the index.
    pub(crate) fn create_folder(&self, root: &Path, path: &str) {
        let created = self.execute(
            None,
            Mutation::FolderCreate(FolderCreateInput {
                root: root.to_owned(),
                path: path.to_owned(),
            }),
        );
        assert_eq!(created.code, ResultCode::Ok, "{}", created.message);
    }

    /// Commits a lifecycle-closed ticket's canonical file, carrying
    /// `closed_at` and `closed_by` as `tests/support/items.rs` writes it:
    /// no local operation closes a ticket. Refresh to index it.
    pub(crate) fn commit_closed_ticket(&self, root: &Path, id: &str, title: &str) {
        let source = format!(
            "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {id}\ntitle: {title}\n\
             type: task\nstatus: done\nclosed_at: 2026-09-30T12:34:56Z\n\
             closed_by: Ada Lovelace <ada@example.invalid>\n---\nBody of {title}.\n"
        );
        self.commit_file(root, &ticket_path(id), source.as_bytes());
    }

    /// Commits a file under `.manyhands/tickets/` whose front matter gives
    /// a key twice, as `tests/read_items.rs` makes a nonconforming ticket.
    /// Refresh to index it.
    pub(crate) fn commit_nonconforming_ticket(&self, root: &Path, id: &str) {
        let source = "---\nmanyhands_managed: true\nmanyhands_kind: ticket\n\
                      repeated: 1\nrepeated: 2\n---\nBroken.\n";
        self.commit_file(root, &ticket_path(id), source.as_bytes());
    }

    /// Commits a file under `docs/` carrying only the managed marker, as
    /// `tests/read_items.rs` makes a nonconforming document. Refresh to
    /// index it.
    pub(crate) fn commit_nonconforming_document(&self, root: &Path, rel_path: &str) {
        self.commit_file(
            root,
            rel_path,
            b"---\nmanyhands_managed: true\n---\nOnly a marker.\n",
        );
    }

    /// Commits an open ticket's canonical file to `root`'s primary branch,
    /// with `extra` front matter lines (each ending in a newline) after the
    /// fields Manyhands defines, as `tests/read_items.rs` writes one. Its
    /// copy is the primary one. Refresh to index it.
    pub(crate) fn commit_ticket(&self, root: &Path, id: &str, title: &str, extra: &str) {
        let source = ticket_source(id, title, extra);
        self.commit_file(root, &ticket_path(id), source.as_bytes());
    }

    /// Saves the ticket `id` of `root` with `title` through `execute` of
    /// `ticket save`, from the observation `show_item` gives now. Saving a
    /// ticket that has only its primary copy provisions its own item
    /// context: a branch and a worktree.
    pub(crate) fn save_ticket(&self, root: &Path, id: &str, title: &str) {
        let repo = self.service.resolve_repository(root).unwrap();
        let shown = self.service.show_item(&repo, &item_id(id)).unwrap();
        let saved = self.execute(
            None,
            Mutation::TicketSave(TicketSaveInput {
                root: root.to_owned(),
                id: item_id(id),
                draft: TicketDraft {
                    title: title.to_owned(),
                    ticket_type: shown.ticket_type.unwrap(),
                    status: shown.status.unwrap(),
                    project: shown.project,
                    team: shown.team,
                    body: shown.body.unwrap_or_default(),
                },
                deps: RelationshipWrite::Unchanged,
                parent: RelationshipWrite::Unchanged,
                observation: shown.observation.unwrap(),
            }),
        );
        assert_eq!(saved.code, ResultCode::Ok, "{}", saved.message);
    }

    /// Commits a comment on `item` to `root`'s primary branch, as
    /// `tests/support/items.rs` writes one: `extra` front matter lines
    /// follow the required fields, so a comment without `created_by` has
    /// no author. Refresh to index it.
    pub(crate) fn commit_comment(
        &self,
        root: &Path,
        item: &str,
        id: &str,
        parent: Option<&str>,
        created_at: &str,
        extra: &str,
    ) {
        let parent = parent
            .map(|parent| format!("parent_id: {parent}\n"))
            .unwrap_or_default();
        let source = format!(
            "---\nmanyhands_managed: true\nmanyhands_kind: comment\nid: {id}\nitem_id: {item}\n\
             {parent}created_at: {created_at}\n{extra}---\nBody of {id}.\n"
        );
        self.commit_file(
            root,
            &format!(".manyhands/comments/{item}/{id}.md"),
            source.as_bytes(),
        );
    }

    /// Overwrites `rel_path` under `root` with bytes that are not UTF-8,
    /// without committing, as `tests/read_items.rs` makes a file with no
    /// source to give. Not refreshed: the index still holds the item.
    pub(crate) fn write_non_utf8(&self, root: &Path, rel_path: &str) {
        fs::write(root.join(rel_path), [0xff, 0xfe, 0x00]).unwrap();
    }

    /// Deletes `rel_path` under `root` and commits the deletion with
    /// `git2`. Refresh for the index to lose it.
    pub(crate) fn commit_removal(&self, root: &Path, rel_path: &str) {
        fs::remove_file(root.join(rel_path)).unwrap();
        let repository = git2::Repository::open(root).unwrap();
        let mut index = repository.index().unwrap();
        index.remove_path(Path::new(rel_path)).unwrap();
        index.write().unwrap();
        let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
        let parent = repository.head().unwrap().peel_to_commit().unwrap();
        let identity = identity();
        let signature = git2::Signature::now(&identity.name, &identity.email).unwrap();
        repository
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                &format!("Remove {rel_path}"),
                &tree,
                &[&parent],
            )
            .unwrap();
    }

    /// Gives the document `id`, which lives in its own item worktree of
    /// `root`, a second source with its ID in that worktree, committed on
    /// the worktree's branch: the recipe of `tests/discovery_rebuild.rs`
    /// (`document_worktree_with_a_second_source_for_its_item_is_rejected`)
    /// for a context problem. Refresh for the index to reject the context.
    /// Gives the worktree.
    pub(crate) fn commit_duplicate_context(&self, root: &Path, id: &str) -> PathBuf {
        let worktree = root
            .join(".manyhands/worktrees")
            .join(id)
            .canonicalize()
            .unwrap();
        let source = format!(
            "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: {id}\ntitle: Copy\n---\n"
        );
        self.commit_file(&worktree, "docs/copy.md", source.as_bytes());
        worktree
    }

    /// Refreshes the index of `root` through the library.
    pub(crate) fn refresh(&self, root: &Path) {
        let refreshed = self.execute(
            None,
            Mutation::IndexRefresh(IndexRefreshInput {
                root: root.to_owned(),
            }),
        );
        assert_eq!(refreshed.code, ResultCode::Ok, "{}", refreshed.message);
    }

    fn execute(
        &self,
        confirmation: Option<ConfirmationId>,
        mutation: Mutation,
    ) -> Envelope<MutationDataDto> {
        self.service.execute(
            MutationCall {
                request_id: RequestId::new(),
                confirmation,
                mutation,
            },
            &mut SessionCredentials::new(NoPassphrase),
            &mut NoProgress,
            &CancellationToken::new(),
        )
    }
}

/// An item ID from its text.
pub(crate) fn item_id(id: &str) -> ItemId {
    id.parse().unwrap()
}

/// Where a ticket's canonical file is, relative to its context.
pub(crate) fn ticket_path(id: &str) -> String {
    format!(".manyhands/tickets/{id}/ticket.md")
}

/// An open ticket's canonical source, with `extra` front matter lines.
pub(crate) fn ticket_source(id: &str, title: &str, extra: &str) -> String {
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {id}\ntitle: {title}\n\
         type: task\nstatus: open\n{extra}---\nBody of {title}.\n"
    )
}

/// Enables the plain repository `root`, which has an identity, on `main`
/// through `service`'s own `prepare` and `execute` of `repo enable`: a
/// second actor, on a service other than the desktop's.
pub(crate) fn enable_through(service: &RepositoryService, root: &Path) {
    let input = || {
        Mutation::RepoEnable(RepoEnableInput {
            root: root.to_owned(),
            primary_branch: "main".to_owned(),
            identity: None,
            slug_prefix: None,
        })
    };
    let prepared = service.prepare(&input());
    assert_eq!(prepared.code, ResultCode::Ok, "{}", prepared.message);
    let Some(PrepareDataDto::Preview(preview)) = prepared.data else {
        panic!("repo enable gave no preview");
    };
    let confirmation = ConfirmationId::parse(&preview.confirmation_id).unwrap();
    let enabled = execute_through(service, Some(confirmation), input());
    assert_eq!(enabled.code, ResultCode::Ok, "{}", enabled.message);
}

/// Removes the registration of `root` through `service`'s own `prepare`
/// and `execute` of `repo remove`: the second actor of a test, on a
/// service other than the desktop's.
pub(crate) fn remove_registration(service: &RepositoryService, root: &Path) {
    let input = || RepoRemoveInput {
        root: root.to_owned(),
    };
    let prepared = service.prepare(&Mutation::RepoRemove(input()));
    assert_eq!(prepared.code, ResultCode::Ok, "{}", prepared.message);
    let Some(PrepareDataDto::Preview(preview)) = prepared.data else {
        panic!("repo remove gave no preview");
    };
    let confirmation = ConfirmationId::parse(&preview.confirmation_id).unwrap();
    let removed = service.execute(
        MutationCall {
            request_id: RequestId::new(),
            confirmation: Some(confirmation),
            mutation: Mutation::RepoRemove(input()),
        },
        &mut SessionCredentials::new(NoPassphrase),
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(removed.code, ResultCode::Ok, "{}", removed.message);
}

/// Refreshes the index of `root` through `service`: a second actor's
/// refresh, on a service other than the desktop's.
pub(crate) fn refresh_through(service: &RepositoryService, root: &Path) {
    let refreshed = service.execute(
        MutationCall {
            request_id: RequestId::new(),
            confirmation: None,
            mutation: Mutation::IndexRefresh(IndexRefreshInput {
                root: root.to_owned(),
            }),
        },
        &mut SessionCredentials::new(NoPassphrase),
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(refreshed.code, ResultCode::Ok, "{}", refreshed.message);
}

impl Fixture {
    /// Saves the ticket `id` of `root` with `title` through a service of
    /// its own, opened over this fixture's data directory with
    /// `FailurePoint::BeforeIndexTransactionCommit`: the recipe of
    /// `tests/mutation_confirmation/abandon.rs`
    /// (`an_abandoned_operation_that_had_committed_keeps_its_commit_and_says_so`)
    /// and of `tests/mutation_replay/resume.rs` (`save_owing_its_hand_off`).
    /// The save commits, its index hand-off fails, and its local operation
    /// is left pending, waiting to be resumed. Gives its operation ID.
    pub(crate) fn strand_ticket_save(&self, root: &Path, id: &str, title: &str) -> String {
        let failing = RepositoryService::open_at_with_failure_point_for_testing(
            self.data_dir(),
            FailurePoint::BeforeIndexTransactionCommit,
        )
        .unwrap();
        let repo = failing.resolve_repository(root).unwrap();
        let shown = failing.show_item(&repo, &item_id(id)).unwrap();
        let owed = failing.execute(
            MutationCall {
                request_id: RequestId::new(),
                confirmation: None,
                mutation: Mutation::TicketSave(TicketSaveInput {
                    root: root.to_owned(),
                    id: item_id(id),
                    draft: TicketDraft {
                        title: title.to_owned(),
                        ticket_type: shown.ticket_type.unwrap(),
                        status: shown.status.unwrap(),
                        project: shown.project,
                        team: shown.team,
                        body: shown.body.unwrap_or_default(),
                    },
                    deps: RelationshipWrite::Unchanged,
                    parent: RelationshipWrite::Unchanged,
                    observation: shown.observation.unwrap(),
                }),
            },
            &mut SessionCredentials::new(NoPassphrase),
            &mut NoProgress,
            &CancellationToken::new(),
        );
        assert_eq!(owed.code, ResultCode::DiscoveryPending, "{}", owed.message);
        owed.operation_id.expect("the save's operation")
    }
}

/// Executes `mutation` through `service`, with no confirmation: a second
/// actor's call, on a service other than the desktop's.
fn execute_through(
    service: &RepositoryService,
    confirmation: Option<ConfirmationId>,
    mutation: Mutation,
) -> Envelope<MutationDataDto> {
    service.execute(
        MutationCall {
            request_id: RequestId::new(),
            confirmation,
            mutation,
        },
        &mut SessionCredentials::new(NoPassphrase),
        &mut NoProgress,
        &CancellationToken::new(),
    )
}

/// Abandons `root`'s pending operation `operation_id` through `service`'s
/// own `prepare` and `execute` of `operation abandon`: a second actor.
pub(crate) fn abandon_through(service: &RepositoryService, root: &Path, operation_id: &str) {
    let input = || {
        Mutation::OperationAbandon(OperationAbandonInput {
            root: root.to_owned(),
            operation_id: OperationId::parse(operation_id).unwrap(),
        })
    };
    let prepared = service.prepare(&input());
    assert_eq!(prepared.code, ResultCode::Ok, "{}", prepared.message);
    let Some(PrepareDataDto::Preview(preview)) = prepared.data else {
        panic!("operation abandon gave no preview");
    };
    let confirmation = ConfirmationId::parse(&preview.confirmation_id).unwrap();
    let abandoned = execute_through(service, Some(confirmation), input());
    assert_eq!(abandoned.code, ResultCode::Ok, "{}", abandoned.message);
}

/// Resumes `root`'s pending operation `operation_id` through `service`:
/// a second actor's `operation resume`, which finishes it.
pub(crate) fn resume_through(service: &RepositoryService, root: &Path, operation_id: &str) {
    let resumed = execute_through(
        service,
        None,
        Mutation::OperationResume(OperationResumeInput {
            root: root.to_owned(),
            operation_id: OperationId::parse(operation_id).unwrap(),
        }),
    );
    assert_eq!(resumed.code, ResultCode::Ok, "{}", resumed.message);
}
