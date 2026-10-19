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
        FolderCreateInput, IndexRefreshInput, Mutation, MutationCall, MutationDataDto, NoProgress,
        PrepareDataDto, RepoCreateInput, RepoRemoveInput, RepositoryService, RequestId,
        TicketCreateInput, TicketDraft,
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
        // configuration's removal.
        let repository = git2::Repository::open(&root).unwrap();
        let head = repository.head().unwrap().peel_to_tree().unwrap();
        let mut index = repository.index().unwrap();
        index.read_tree(&head).unwrap();
        index.write().unwrap();
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
                deps: Vec::new(),
                parent: None,
                initials: None,
            }),
        );
        assert_eq!(created.code, ResultCode::Ok, "{}", created.message);
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
