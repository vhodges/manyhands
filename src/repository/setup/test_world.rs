//! What the tests of the setup poll and of completion share: a registered
//! repository on `main` with a recorded setup, a bare repository that
//! stands for the setup's remote, and a stubbed channel over it. Nothing
//! here waits, and nothing reaches a network.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use git2::{Oid, Repository, Signature};
use rusqlite::{Connection, params};

use super::{
    channel::SetupChannel,
    commit,
    record::{self, SetupKind, SetupRecord},
};
use crate::{
    canonical::{self, CONFIG_PATH, RepositoryConfig},
    repository::{
        CommitIdentity, OperationFamily, OperationId, REGISTRY_FILE, RepositoryService, RequestId,
        open_registry, register_setup_pending,
        request_store::{
            DigestSalt, IntentDigest, NewRequest, RequestOperation, RequestRecord, ScopeKey,
        },
        transport::{
            DeleteAttempt, SetupAdvertisement, SetupPush, SetupScope, SshDirection,
            SshTransportError, SshTransportErrorKind,
        },
    },
};

pub(crate) const SETUP: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
pub(crate) const PRIMARY: &str = "main";
/// The publication remote a change begins from.
pub(crate) const CURRENT: &str = "origin";
/// A remote a setup can be on that is not the current one.
pub(crate) const OTHER: &str = "upstream";

/// The time every setup operation of a test world sees.
pub(crate) const NOW: i64 = 1_800_000_000;

struct Fixed;

impl crate::repository::mutation::Clock for Fixed {
    fn now(&self) -> time::OffsetDateTime {
        time::OffsetDateTime::from_unix_timestamp(NOW).unwrap()
    }
}

pub(crate) fn identity() -> CommitIdentity {
    CommitIdentity {
        name: "Fixture".into(),
        email: "fixture@example.invalid".into(),
    }
}

fn signature() -> Signature<'static> {
    Signature::now("Fixture", "fixture@example.invalid").unwrap()
}

/// A configuration file for `primary` on `remote`, in `integration` mode.
pub(crate) fn configuration_for(primary: &str, remote: &str, integration: &str) -> Vec<u8> {
    let mut unknown = toml::Table::new();
    unknown.insert("integration".into(), integration.into());
    canonical::serialize_repository_config(&RepositoryConfig {
        primary_branch: primary.into(),
        publication_remote: Some(remote.into()),
        unknown,
    })
    .unwrap()
    .into_bytes()
}

/// `configuration`, edited in review: it holds a short code prefix too.
pub(crate) fn edited_configuration(remote: &str, integration: &str) -> Vec<u8> {
    let mut unknown = toml::Table::new();
    unknown.insert("integration".into(), integration.into());
    unknown.insert(canonical::SLUG_PREFIX_KEY.into(), "mh".into());
    canonical::serialize_repository_config(&RepositoryConfig {
        primary_branch: PRIMARY.into(),
        publication_remote: Some(remote.into()),
        unknown,
    })
    .unwrap()
    .into_bytes()
}

pub(crate) fn configuration(remote: &str, integration: &str) -> Vec<u8> {
    configuration_for(PRIMARY, remote, integration)
}

/// A channel over the bare repository that stands for the remote: it
/// answers the fetch as the transport does, counts connections, and can
/// fail the fetch. A poll never pushes or observes, so those panic.
pub(crate) struct Stub {
    pub(crate) local: PathBuf,
    pub(crate) remote: PathBuf,
    pub(crate) fetches: usize,
    pub(crate) fails: Option<SshTransportErrorKind>,
}

impl SetupChannel for Stub {
    fn fetch_base(&mut self, scope: &SetupScope) -> Result<SetupAdvertisement, SshTransportError> {
        self.fetches += 1;
        if let Some(kind) = self.fails.clone() {
            return Err(SshTransportError {
                root: self.local.clone(),
                remote_name: scope.remote_name.clone(),
                direction: SshDirection::Fetch,
                selected_key_id: None,
                authority: None,
                kind,
            });
        }
        let remote = Repository::open(&self.remote).unwrap();
        let target = |name: &str| remote.find_reference(name).ok().and_then(|r| r.target());
        let found = SetupAdvertisement {
            primary: target(&format!("refs/heads/{}", scope.primary_branch)),
            setup: target(&scope.branch_ref()),
        };
        if found.primary.is_some() {
            let local = Repository::open(&self.local).unwrap();
            local
                .remote_anonymous(self.remote.to_str().unwrap())
                .unwrap()
                .fetch(
                    &[format!(
                        "+refs/heads/{}:{}",
                        scope.primary_branch,
                        scope.base_ref()
                    )],
                    None,
                    None,
                )
                .unwrap();
        }
        Ok(found)
    }

    fn push_branch(&mut self, _: &SetupScope, _: Oid) -> Result<SetupPush, SshTransportError> {
        panic!("a poll never pushes");
    }

    fn delete_branch(
        &mut self,
        _: &SetupScope,
        _: Oid,
    ) -> Result<DeleteAttempt, SshTransportError> {
        panic!("a poll never deletes");
    }

    fn observe_branch(&mut self, _: &SetupScope) -> Result<Option<Oid>, SshTransportError> {
        panic!("a poll learns of the setup branch from its one fetch");
    }
}

pub(crate) struct World {
    pub(crate) service: RepositoryService,
    pub(crate) data: tempfile::TempDir,
    _local_directory: tempfile::TempDir,
    remote_directory: tempfile::TempDir,
    pub(crate) root: PathBuf,
    pub(crate) kind: SetupKind,
    pub(crate) repository_id: i64,
    pub(crate) request_id: RequestId,
    /// The commit the local repository and the remote began from.
    pub(crate) c0: Oid,
}

impl World {
    /// A registered repository on `main` with a repository-local identity,
    /// the remotes `origin` and `upstream` at addresses nothing contacts, a
    /// bare repository for the setup's remote that holds the same primary,
    /// and a recorded setup of `kind` on `setup_remote` whose request has
    /// its own row completed at `setup_recorded`. A change begins from a
    /// primary that holds a configuration naming `origin`; a first-time
    /// setup from one that holds none. The setup's configuration names
    /// `setup_remote` and the mode `pull_request`.
    pub(crate) fn new(kind: SetupKind, setup_remote: &str, published: bool) -> Self {
        Self::with_configuration(
            kind,
            setup_remote,
            published,
            configuration(setup_remote, "pull_request"),
        )
    }

    pub(crate) fn with_configuration(
        kind: SetupKind,
        setup_remote: &str,
        published: bool,
        intended: Vec<u8>,
    ) -> Self {
        let local_directory = tempfile::tempdir().unwrap();
        let root = local_directory.path().canonicalize().unwrap();
        let mut options = git2::RepositoryInitOptions::new();
        options.initial_head(PRIMARY);
        let local = Repository::init_opts(&root, &options).unwrap();
        {
            let mut config = local.config().unwrap();
            config.set_str("user.name", "Fixture").unwrap();
            config
                .set_str("user.email", "fixture@example.invalid")
                .unwrap();
        }
        let mut index = local.index().unwrap();
        let mut files: Vec<(&str, Vec<u8>)> = vec![("README.md", b"readme".to_vec())];
        if kind == SetupKind::Change {
            files.push((CONFIG_PATH, configuration(CURRENT, "direct")));
        }
        for (path, content) in &files {
            let full = root.join(path);
            std::fs::create_dir_all(full.parent().unwrap()).unwrap();
            std::fs::write(&full, content).unwrap();
            index.add_path(Path::new(path)).unwrap();
        }
        index.write().unwrap();
        let tree = local.find_tree(index.write_tree().unwrap()).unwrap();
        let c0 = local
            .commit(Some("HEAD"), &signature(), &signature(), "Base", &tree, &[])
            .unwrap();
        local
            .remote(CURRENT, "ssh://git@github.com/team/repo.git")
            .unwrap();
        local
            .remote(OTHER, "ssh://git@github.com/other/repo.git")
            .unwrap();
        let base_configuration_oid = (kind == SetupKind::Change)
            .then(|| tree.get_path(Path::new(CONFIG_PATH)).unwrap().id());
        drop(tree);

        // The remote: a bare clone of the local repository.
        let remote_directory = tempfile::tempdir().unwrap();
        git2::build::RepoBuilder::new()
            .bare(true)
            .clone(root.to_str().unwrap(), remote_directory.path())
            .unwrap();

        let data = tempfile::tempdir().unwrap();
        // No test waits out a lease's real timeout.
        let service = RepositoryService::open_at(data.path())
            .unwrap()
            .with_lease_wait_for_testing(std::time::Duration::from_millis(1))
            .with_clock_for_testing(std::sync::Arc::new(Fixed));
        let request_id = RequestId::new();
        let operation_id = OperationId::new();
        // A change begins from an enabled repository with `origin` selected, as
        // a direct enable and a selection leave it: its configuration blob
        // and its stored plan. A first-time setup is registered with neither.
        if kind == SetupKind::Change {
            service
                .reconcile_registration(
                    &local,
                    &root,
                    crate::repository::RepositoryOperation::SetPublicationRemote,
                )
                .unwrap();
        }
        let mut db = open_registry(&data.path().join(REGISTRY_FILE), &mut |_| {}).unwrap();
        let tx = db.transaction().unwrap();
        let repository_id = match kind {
            SetupKind::Enable => register_setup_pending(&tx, &root).unwrap(),
            SetupKind::Change => tx
                .query_row(
                    "SELECT id FROM repositories WHERE root_path = ?1",
                    [root.to_str().unwrap()],
                    |row| row.get(0),
                )
                .unwrap(),
        };
        record::insert(
            &tx,
            repository_id,
            &SetupRecord {
                setup_id: SETUP.parse().unwrap(),
                kind,
                remote_name: setup_remote.into(),
                primary_branch: PRIMARY.into(),
                endpoint_digest: [1; 32],
                configuration: intended,
                base_configuration_oid,
                base_oid: None,
                commit_oid: None,
                published_oid: None,
                request_id,
                created_at: 1_700_000_000,
            },
        )
        .unwrap();
        tx.commit().unwrap();
        drop(db);

        let command = match kind {
            SetupKind::Enable => "repo enable",
            SetupKind::Change => "remote select",
        };
        let scope = ScopeKey::for_repository(&root).unwrap();
        let digest =
            IntentDigest::builder(DigestSalt::Request(request_id), command, &scope, "").finish();
        service
            .insert_request(&NewRequest {
                request_id,
                scope,
                command: command.into(),
                target: String::new(),
                intent_digest: digest,
                confirmation: None,
                base_ref: None,
                base_oid: None,
                expected_digest: None,
                operations: vec![RequestOperation {
                    family: OperationFamily::Local,
                    operation_id,
                }],
            })
            .unwrap();
        let world = Self {
            service,
            data,
            _local_directory: local_directory,
            remote_directory,
            root,
            kind,
            repository_id,
            request_id,
            c0,
        };
        world
            .db()
            .execute(
                "INSERT INTO operation_records
                    (repository_id,root_path,operation_ulid,action,target,state,completed_step,observed_at)
                 VALUES (?1,?2,?3,?4,'','completed','setup_recorded',1)",
                params![
                    repository_id,
                    world.root.to_str().unwrap(),
                    operation_id.to_string(),
                    if kind == SetupKind::Enable { "enable" } else { "set_publication_remote" },
                ],
            )
            .unwrap();
        if published {
            world.publish();
        }
        world
    }

    /// The state after publishing: the setup commit on the base, the local
    /// branch, the branch on the remote, and the record `published`. The
    /// private base ref is left as a fetch would.
    fn publish(&self) {
        let record = self.record().unwrap();
        let message = match self.kind {
            SetupKind::Enable => commit::ENABLE_MESSAGE,
            SetupKind::Change => commit::CHANGE_MESSAGE,
        };
        let local = self.git();
        let made =
            commit::build(&local, self.c0, &record.configuration, message, &identity()).unwrap();
        local
            .reference(&self.branch_ref(), made, false, "setup")
            .unwrap();
        local
            .remote_anonymous(self.remote_directory.path().to_str().unwrap())
            .unwrap()
            .push(&[format!("{0}:{0}", self.branch_ref())], None)
            .unwrap();
        local
            .reference(&self.base_ref(), self.c0, true, "base")
            .unwrap();
        let mut db = self.db();
        let tx = db.transaction().unwrap();
        record::record_commit(&tx, self.repository_id, self.c0, made).unwrap();
        record::record_published(&tx, self.repository_id, made).unwrap();
        tx.commit().unwrap();
    }

    pub(crate) fn branch_ref(&self) -> String {
        format!("refs/heads/manyhands/setup/{SETUP}")
    }

    /// A linked worktree, outside the root, that has the setup branch checked
    /// out; the root's own HEAD is untouched. Its administrative directory is
    /// `<root>/.git/worktrees/linked`.
    pub(crate) fn check_the_setup_branch_out_in_a_linked_worktree(&self) {
        let git = self.git();
        let reference = git.find_reference(&self.branch_ref()).unwrap();
        let mut options = git2::WorktreeAddOptions::new();
        options.reference(Some(&reference));
        git.worktree("linked", &self.data.path().join("linked"), Some(&options))
            .unwrap();
    }

    /// The linked worktree is gone: its registration is removed, as a
    /// prune does for one whose directory was deleted.
    pub(crate) fn drop_the_linked_worktree(&self) {
        std::fs::remove_dir_all(self.root.join(".git/worktrees/linked")).unwrap();
    }

    pub(crate) fn base_ref(&self) -> String {
        format!("refs/manyhands/setups/{SETUP}/base")
    }

    pub(crate) fn db(&self) -> Connection {
        open_registry(&self.data.path().join(REGISTRY_FILE), &mut |_| {}).unwrap()
    }

    pub(crate) fn git(&self) -> Repository {
        Repository::open(&self.root).unwrap()
    }

    pub(crate) fn remote(&self) -> Repository {
        Repository::open(self.remote_directory.path()).unwrap()
    }

    pub(crate) fn stub(&self) -> Stub {
        Stub {
            local: self.root.clone(),
            remote: self.remote_directory.path().to_owned(),
            fetches: 0,
            fails: None,
        }
    }

    pub(crate) fn record(&self) -> Option<SetupRecord> {
        record::read(&self.db(), self.repository_id).unwrap()
    }

    pub(crate) fn request(&self) -> RequestRecord {
        self.service
            .request_record(self.request_id)
            .unwrap()
            .unwrap()
    }

    pub(crate) fn local_ref(&self, name: &str) -> Option<Oid> {
        self.git()
            .find_reference(name)
            .ok()
            .and_then(|r| r.target())
    }

    pub(crate) fn remote_ref(&self, name: &str) -> Option<Oid> {
        self.remote()
            .find_reference(name)
            .ok()
            .and_then(|r| r.target())
    }

    /// The tip of the remote's primary.
    pub(crate) fn remote_primary(&self) -> Oid {
        self.remote_ref(&format!("refs/heads/{PRIMARY}")).unwrap()
    }

    /// The tip of the local primary.
    pub(crate) fn local_primary(&self) -> Oid {
        self.local_ref(&format!("refs/heads/{PRIMARY}")).unwrap()
    }

    /// The setup commit the record names.
    pub(crate) fn setup_commit(&self) -> Oid {
        self.record()
            .and_then(|record| record.commit_oid)
            .expect("a published setup")
    }

    /// The remote's primary moves on with a commit that has nothing to do
    /// with the setup.
    pub(crate) fn remote_moves_on(&self) -> Oid {
        let remote = self.remote();
        let tip = self.remote_primary();
        let parent = remote.find_commit(tip).unwrap();
        let blob = remote.blob(b"more").unwrap();
        let mut builder = remote.treebuilder(Some(&parent.tree().unwrap())).unwrap();
        builder.insert("NOTES.md", blob, 0o100644).unwrap();
        let tree = remote.find_tree(builder.write().unwrap()).unwrap();
        remote
            .commit(
                Some(&format!("refs/heads/{PRIMARY}")),
                &signature(),
                &signature(),
                "Other work",
                &tree,
                &[&parent],
            )
            .unwrap()
    }

    /// A squash of the setup: one commit on the remote's primary, with the
    /// configuration `merged`, which has the setup commit for no parent.
    pub(crate) fn squash(&self, merged: &[u8]) -> Oid {
        let remote = self.remote();
        let squashed = commit::build(
            &remote,
            self.remote_primary(),
            merged,
            "Squashed setup",
            &identity(),
        )
        .unwrap();
        remote
            .reference(&format!("refs/heads/{PRIMARY}"), squashed, true, "squash")
            .unwrap();
        squashed
    }

    /// A merge commit of the setup commit into the remote's primary, with
    /// the configuration `merged`.
    pub(crate) fn merge_commit(&self, merged: &[u8]) -> Oid {
        let remote = self.remote();
        let tip = remote.find_commit(self.remote_primary()).unwrap();
        let setup = remote.find_commit(self.setup_commit()).unwrap();
        let as_squashed = commit::build(&remote, tip.id(), merged, "tree", &identity()).unwrap();
        let tree = remote.find_commit(as_squashed).unwrap().tree().unwrap();
        remote
            .commit(
                Some(&format!("refs/heads/{PRIMARY}")),
                &signature(),
                &signature(),
                "Merge pull request",
                &tree,
                &[&tip, &setup],
            )
            .unwrap()
    }

    /// A fast-forward of the remote's primary to the setup commit, which is
    /// only one while the primary is where the setup began.
    pub(crate) fn fast_forward(&self) -> Oid {
        let setup = self.setup_commit();
        self.remote()
            .reference(&format!("refs/heads/{PRIMARY}"), setup, true, "ff")
            .unwrap();
        setup
    }

    /// The remote's primary is deleted.
    pub(crate) fn delete_remote_primary(&self) {
        self.remote()
            .find_reference(&format!("refs/heads/{PRIMARY}"))
            .unwrap()
            .delete()
            .unwrap();
    }

    /// What a `git pull` of the remote's primary leaves, or a crash after the
    /// activation's fast-forward: the commit fetched into the private base
    /// ref, the working tree at it, and the local primary moved to it.
    pub(crate) fn pull(&self, tip: Oid) {
        let local = self.git();
        local
            .remote_anonymous(self.remote_directory.path().to_str().unwrap())
            .unwrap()
            .fetch(
                &[format!("+refs/heads/{PRIMARY}:{}", self.base_ref())],
                None,
                None,
            )
            .unwrap();
        let commit = local.find_commit(tip).unwrap();
        let mut checkout = git2::build::CheckoutBuilder::new();
        checkout.safe();
        local
            .checkout_tree(commit.as_object(), Some(&mut checkout))
            .unwrap();
        local
            .reference(&format!("refs/heads/{PRIMARY}"), tip, true, "pull")
            .unwrap();
    }

    /// The remote's setup branch is deleted, as a forge does after a merge
    /// or someone does by hand.
    pub(crate) fn delete_remote_branch(&self) {
        self.remote()
            .find_reference(&self.branch_ref())
            .unwrap()
            .delete()
            .unwrap();
    }

    /// A commit on the local primary that the remote does not have, with
    /// the working tree and the index at it.
    pub(crate) fn local_commit(&self) -> Oid {
        let repository = self.git();
        std::fs::write(self.root.join("LOCAL.md"), "local").unwrap();
        let mut index = repository.index().unwrap();
        index.add_path(Path::new("LOCAL.md")).unwrap();
        index.write().unwrap();
        let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
        let parent = repository.head().unwrap().peel_to_commit().unwrap();
        repository
            .commit(
                Some("HEAD"),
                &signature(),
                &signature(),
                "Local work",
                &tree,
                &[&parent],
            )
            .unwrap()
    }

    /// The root is on another branch.
    pub(crate) fn check_out_another_branch(&self) {
        let repository = self.git();
        let head = repository.head().unwrap().peel_to_commit().unwrap();
        repository.branch("elsewhere", &head, false).unwrap();
        repository.set_head("refs/heads/elsewhere").unwrap();
    }

    /// An untracked file in the root.
    pub(crate) fn leave_an_untracked_file(&self) {
        std::fs::write(self.root.join("scratch.txt"), "mine").unwrap();
    }

    /// Every reference of the local repository, by name.
    pub(crate) fn refs(&self) -> BTreeMap<String, Option<Oid>> {
        self.git()
            .references()
            .unwrap()
            .map(|reference| {
                let reference = reference.unwrap();
                (reference.name().unwrap().to_owned(), reference.target())
            })
            .collect()
    }

    pub(crate) fn count(&self, sql: &str) -> i64 {
        self.db().query_row(sql, [], |row| row.get(0)).unwrap()
    }
}

/// The rows of `table`, each as the text of every column but `skip`.
pub(crate) fn dump(db: &Connection, table: &str, skip: &[&str]) -> Vec<BTreeMap<String, String>> {
    let mut statement = db.prepare(&format!("SELECT * FROM {table}")).unwrap();
    let names: Vec<String> = statement
        .column_names()
        .into_iter()
        .map(str::to_owned)
        .collect();
    statement
        .query_map([], |row| {
            let mut map = BTreeMap::new();
            for (index, name) in names.iter().enumerate() {
                if skip.contains(&name.as_str()) {
                    continue;
                }
                let value: rusqlite::types::Value = row.get(index)?;
                map.insert(name.clone(), format!("{value:?}"));
            }
            Ok(map)
        })
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

/// A `ticket create`, the command a setup refuses while it is pending.
pub(crate) fn create_ticket(
    world: &World,
) -> crate::results::Envelope<crate::repository::MutationDataDto> {
    let mut session = crate::repository::keys::SessionCredentials::new(NoPassphrase);
    world.service.execute(
        crate::repository::MutationCall {
            request_id: crate::repository::RequestId::new(),
            confirmation: None,
            mutation: crate::repository::Mutation::TicketCreate(
                crate::repository::TicketCreateInput {
                    root: world.root.clone(),
                    id: "01ARZ3NDEKTSV4RRFFQ69G5FAW".parse().unwrap(),
                    draft: crate::repository::TicketDraft {
                        title: "A title".to_owned(),
                        ticket_type: "task".to_owned(),
                        status: "open".to_owned(),
                        project: None,
                        team: None,
                        body: String::new(),
                    },
                    deps: Vec::new(),
                    parent: None,
                    initials: None,
                },
            ),
        },
        &mut session,
        &mut crate::repository::NoProgress,
        &crate::repository::CancellationToken::new(),
    )
}

/// A session whose credentials are never asked for.
struct NoPassphrase;

impl crate::repository::keys::SessionCredentialProvider for NoPassphrase {
    fn request_passphrase(
        &mut self,
        _: &crate::repository::keys::UnlockRequest,
    ) -> crate::repository::keys::PassphraseResponse {
        panic!("an activation asks for no passphrase")
    }
}
