//! Which way a `remote select` goes (design Part F's table), and the
//! recording of the way that puts the change on a setup branch (K1 and
//! Part B's race rule). Nothing here reaches a remote or waits.

use std::path::{Path, PathBuf};

use git2::{Oid, Repository, RepositoryInitOptions, Signature, Time};
use rusqlite::params;

use super::{Change, ChangePath, ChangeSetupRequest, Planned, Refused, path_of};
use crate::repository::setup::{
    channel::SetupChannel,
    enable::SetupOutcome,
    publish::{SetupError, continue_setup},
    record::{self, SetupKind},
};
use crate::{
    canonical::{self, ForgeKind, ForgeSetting, Integration, RepositoryConfig},
    repository::{
        EnableRepositoryRequest, OperationFamily, OperationId, PublicationRemoteOutcome,
        RepositoryErrorKind, RepositoryService, RequestId, SetPublicationRemoteRequest,
        open_registry,
        request_store::{DigestSalt, IntentDigest, NewRequest, RequestOperation, ScopeKey},
        transport::{DeleteAttempt, SetupAdvertisement, SetupPush, SetupScope, SshTransportError},
    },
    results::ResultCode,
};

pub(crate) const REMOTE_URL: &str = "ssh://git@example.invalid/team/repo.git";
pub(crate) const OTHER_URL: &str = "ssh://git@example.invalid/other/repo.git";
const WEB_URL: &str = "https://example.invalid/team/repo.git";

pub(crate) fn config(source: &str) -> RepositoryConfig {
    canonical::parse_repository_config(source).unwrap()
}

pub(crate) fn bytes(config: &RepositoryConfig) -> Vec<u8> {
    canonical::serialize_repository_config(config)
        .unwrap()
        .into_bytes()
}

/// A direct configuration with no remote.
pub(crate) const DIRECT_NONE: &str = "format_version = 1\nprimary_branch = \"main\"\n";
/// A direct configuration on `origin`, with a prefix and a key this build does
/// not know, which a change must keep.
pub(crate) const DIRECT_ORIGIN: &str = "format_version = 1\nprimary_branch = \"main\"\npublication_remote = \"origin\"\nticket_slug_prefix = \"mh\"\nextra = true\n";
pub(crate) const DIRECT_ORIGIN_FORGE: &str = "format_version = 1\nprimary_branch = \"main\"\npublication_remote = \"origin\"\nticket_slug_prefix = \"mh\"\nextra = true\n\n[forge]\nkind = \"github\"\n";
pub(crate) const PR_ORIGIN: &str = "format_version = 2\nprimary_branch = \"main\"\npublication_remote = \"origin\"\nintegration = \"pull_request\"\nticket_slug_prefix = \"mh\"\nextra = true\n";
pub(crate) const PR_ORIGIN_FORGE: &str = "format_version = 2\nprimary_branch = \"main\"\npublication_remote = \"origin\"\nintegration = \"pull_request\"\nticket_slug_prefix = \"mh\"\nextra = true\n\n[forge]\nkind = \"gitlab\"\n";

fn github() -> ForgeSetting {
    ForgeSetting {
        kind: Some(ForgeKind::Github),
        web_url: None,
        pull_request_url: None,
    }
}

fn gitlab() -> ForgeSetting {
    ForgeSetting {
        kind: Some(ForgeKind::Gitlab),
        web_url: None,
        pull_request_url: None,
    }
}

/// A forge that is no forge: it needs a kind or a pull request address.
fn empty_forge() -> ForgeSetting {
    ForgeSetting {
        kind: None,
        web_url: Some("https://example.invalid".to_owned()),
        pull_request_url: None,
    }
}

/// What design Part F's table makes of one row.
enum Expect {
    AlreadyApplied,
    Direct,
    /// A setup branch, carrying this file.
    Setup(&'static str),
    Refused(Refused),
}

#[allow(clippy::type_complexity)]
fn the_table() -> Vec<(
    &'static str,
    &'static str,
    Option<&'static str>,
    Option<Integration>,
    Option<Option<ForgeSetting>>,
    Expect,
)> {
    use Expect::{AlreadyApplied, Direct, Refused as No, Setup};
    use Integration::{Direct as Dir, PullRequest as Pr};
    vec![
        // Any mode: equal to the current configuration.
        (
            "equal, direct",
            DIRECT_ORIGIN,
            Some("origin"),
            None,
            None,
            AlreadyApplied,
        ),
        (
            "equal, direct named",
            DIRECT_ORIGIN,
            Some("origin"),
            Some(Dir),
            None,
            AlreadyApplied,
        ),
        (
            "equal, no forge removed",
            DIRECT_ORIGIN,
            Some("origin"),
            None,
            Some(None),
            AlreadyApplied,
        ),
        (
            "equal, same forge",
            DIRECT_ORIGIN_FORGE,
            Some("origin"),
            None,
            Some(Some(github())),
            AlreadyApplied,
        ),
        (
            "equal, nothing selected",
            DIRECT_NONE,
            None,
            None,
            None,
            AlreadyApplied,
        ),
        (
            "equal, pull request",
            PR_ORIGIN,
            Some("origin"),
            None,
            None,
            AlreadyApplied,
        ),
        (
            "equal, pull request named",
            PR_ORIGIN,
            Some("origin"),
            Some(Pr),
            None,
            AlreadyApplied,
        ),
        // Direct to direct: today's commit on the primary.
        (
            "direct, first remote",
            DIRECT_NONE,
            Some("origin"),
            None,
            None,
            Direct,
        ),
        (
            "direct, another remote",
            DIRECT_ORIGIN,
            Some("backup"),
            None,
            None,
            Direct,
        ),
        ("direct, clearing", DIRECT_ORIGIN, None, None, None, Direct),
        (
            "direct, a forge change",
            DIRECT_ORIGIN,
            Some("origin"),
            None,
            Some(Some(gitlab())),
            Direct,
        ),
        (
            "direct, a forge removed",
            DIRECT_ORIGIN_FORGE,
            Some("origin"),
            None,
            Some(None),
            Direct,
        ),
        (
            "direct, another remote and a forge",
            DIRECT_ORIGIN,
            Some("backup"),
            Some(Dir),
            Some(Some(github())),
            Direct,
        ),
        // Direct to pull request, any remote: a setup branch.
        (
            "direct to pull request, the same remote",
            DIRECT_ORIGIN,
            Some("origin"),
            Some(Pr),
            None,
            Setup(
                "format_version = 2\nprimary_branch = \"main\"\npublication_remote = \"origin\"\nintegration = \"pull_request\"\nticket_slug_prefix = \"mh\"\nextra = true\n",
            ),
        ),
        (
            "direct to pull request, a first remote",
            DIRECT_NONE,
            Some("origin"),
            Some(Pr),
            None,
            Setup(
                "format_version = 2\nprimary_branch = \"main\"\npublication_remote = \"origin\"\nintegration = \"pull_request\"\n",
            ),
        ),
        (
            "direct to pull request, a different remote",
            DIRECT_ORIGIN,
            Some("backup"),
            Some(Pr),
            None,
            Setup(
                "format_version = 2\nprimary_branch = \"main\"\npublication_remote = \"backup\"\nintegration = \"pull_request\"\nticket_slug_prefix = \"mh\"\nextra = true\n",
            ),
        ),
        (
            "direct to pull request, with a forge",
            DIRECT_ORIGIN,
            Some("origin"),
            Some(Pr),
            Some(Some(github())),
            Setup(
                "format_version = 2\nprimary_branch = \"main\"\npublication_remote = \"origin\"\nintegration = \"pull_request\"\nticket_slug_prefix = \"mh\"\nextra = true\n\n[forge]\nkind = \"github\"\n",
            ),
        ),
        // Pull request, the same remote, either mode: a setup branch.
        (
            "pull request to direct",
            PR_ORIGIN,
            Some("origin"),
            Some(Dir),
            None,
            Setup(
                "format_version = 1\nprimary_branch = \"main\"\npublication_remote = \"origin\"\nticket_slug_prefix = \"mh\"\nextra = true\n",
            ),
        ),
        (
            "pull request, a forge change",
            PR_ORIGIN,
            Some("origin"),
            None,
            Some(Some(gitlab())),
            Setup(
                "format_version = 2\nprimary_branch = \"main\"\npublication_remote = \"origin\"\nintegration = \"pull_request\"\nticket_slug_prefix = \"mh\"\nextra = true\n\n[forge]\nkind = \"gitlab\"\n",
            ),
        ),
        (
            "pull request, a forge removed",
            PR_ORIGIN_FORGE,
            Some("origin"),
            None,
            Some(None),
            Setup(
                "format_version = 2\nprimary_branch = \"main\"\npublication_remote = \"origin\"\nintegration = \"pull_request\"\nticket_slug_prefix = \"mh\"\nextra = true\n",
            ),
        ),
        // Pull request, another remote: decision 8.
        (
            "pull request, another remote",
            PR_ORIGIN,
            Some("backup"),
            None,
            None,
            No(Refused::InvalidInput),
        ),
        (
            "pull request to direct, another remote",
            PR_ORIGIN,
            Some("backup"),
            Some(Dir),
            None,
            No(Refused::InvalidInput),
        ),
        // Validation comes first, as today.
        (
            "pull request, clearing the remote",
            PR_ORIGIN,
            None,
            None,
            None,
            No(Refused::InvalidConfiguration),
        ),
        (
            "pull request over no remote",
            DIRECT_NONE,
            None,
            Some(Pr),
            None,
            No(Refused::InvalidConfiguration),
        ),
        (
            "pull request over a cleared remote",
            DIRECT_ORIGIN,
            None,
            Some(Pr),
            None,
            No(Refused::InvalidConfiguration),
        ),
        (
            "a forge that is no forge",
            DIRECT_ORIGIN,
            Some("origin"),
            None,
            Some(Some(empty_forge())),
            No(Refused::InvalidConfiguration),
        ),
        (
            "a forge that is no forge, another remote of a pull request",
            PR_ORIGIN,
            Some("backup"),
            None,
            Some(Some(empty_forge())),
            No(Refused::InvalidConfiguration),
        ),
    ]
}

/// Part F's table, as a function of the configuration alone.
fn the_path_of_each_row() {
    for (what, current, name, integration, forge, expect) in the_table() {
        let current = config(current);
        let forge: Option<Option<&ForgeSetting>> = forge.as_ref().map(Option::as_ref);
        let path = path_of(
            &current,
            &Change {
                name,
                integration,
                forge,
            },
        );
        match (expect, path) {
            (Expect::AlreadyApplied, Ok(ChangePath::AlreadyApplied)) => {}
            (Expect::Direct, Ok(ChangePath::Direct)) => {}
            (Expect::Refused(refused), Err(found)) => assert_eq!(found, refused, "{what}"),
            (Expect::Setup(file), Ok(ChangePath::Setup(planned))) => {
                let expected = config(file);
                assert_eq!(planned.from, current, "{what}: what it was derived from");
                assert_eq!(planned.intended.configuration, expected, "{what}: the file");
                assert_eq!(
                    planned.intended.integration,
                    expected.integration(),
                    "{what}: the mode"
                );
                // The file is one the serializer writes.
                assert!(
                    canonical::serialize_repository_config(&planned.intended.configuration).is_ok(),
                    "{what}"
                );
            }
            (_, path) => panic!("{what}: found {path:?}"),
        }
    }
}

/// Codes of the two refusals.
fn the_codes_of_a_refusal() {
    assert_eq!(Refused::InvalidInput.code(), ResultCode::InvalidInput);
    assert_eq!(
        Refused::InvalidConfiguration.code(),
        ResultCode::InvalidConfiguration
    );
}

// ---- A repository that is enabled, for what the library does with a path.

pub(crate) struct World {
    _local: tempfile::TempDir,
    _data: tempfile::TempDir,
    pub(crate) root: PathBuf,
    pub(crate) service: RepositoryService,
    pub(crate) head: Oid,
}

impl World {
    /// A repository on `main` whose primary holds `configuration`, the
    /// remotes `origin` and `backup` at SSH-shaped addresses and `web` at an
    /// HTTPS one, an identity, and a registration.
    pub(crate) fn new(configuration: &str) -> Self {
        let local = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let root = local.path().canonicalize().unwrap();
        let mut options = RepositoryInitOptions::new();
        options.initial_head("main");
        let repository = Repository::init_opts(&root, &options).unwrap();
        {
            let mut git = repository.config().unwrap();
            git.set_str("user.name", "Fixture").unwrap();
            git.set_str("user.email", "fixture@example.invalid")
                .unwrap();
        }
        repository.remote("origin", REMOTE_URL).unwrap();
        repository.remote("backup", OTHER_URL).unwrap();
        repository.remote("web", WEB_URL).unwrap();
        std::fs::write(root.join("README.md"), "readme\n").unwrap();
        std::fs::create_dir_all(root.join(".manyhands")).unwrap();
        std::fs::write(
            root.join(canonical::CONFIG_PATH),
            bytes(&config(configuration)),
        )
        .unwrap();
        let mut index = repository.index().unwrap();
        index.add_path(Path::new("README.md")).unwrap();
        index.add_path(Path::new(canonical::CONFIG_PATH)).unwrap();
        index.write().unwrap();
        let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
        let signature =
            Signature::new("Fixture", "fixture@example.invalid", &Time::new(0, 0)).unwrap();
        let head = repository
            .commit(Some("HEAD"), &signature, &signature, "Base", &tree, &[])
            .unwrap();
        drop(tree);
        let data = tempfile::tempdir().unwrap();
        let service = RepositoryService::open_at(data.path())
            .unwrap()
            .with_lease_wait_for_testing(std::time::Duration::from_millis(1));
        service
            .enable(EnableRepositoryRequest {
                root: root.clone(),
                primary_branch: "main".into(),
                identity: None,
                operation_id: OperationId::new(),
            })
            .unwrap();
        Self {
            _local: local,
            _data: data,
            root,
            service,
            head,
        }
    }

    pub(crate) fn git(&self) -> Repository {
        Repository::open(&self.root).unwrap()
    }

    pub(crate) fn primary(&self) -> Oid {
        self.git().refname_to_id("refs/heads/main").unwrap()
    }

    pub(crate) fn configuration(&self) -> Vec<u8> {
        std::fs::read(self.root.join(canonical::CONFIG_PATH)).unwrap()
    }

    pub(crate) fn blob(&self) -> Oid {
        self.git()
            .head()
            .unwrap()
            .peel_to_tree()
            .unwrap()
            .get_path(Path::new(canonical::CONFIG_PATH))
            .unwrap()
            .id()
    }

    pub(crate) fn db(&self) -> rusqlite::Connection {
        open_registry(&self.service.registry_path, &mut |_| {}).unwrap()
    }

    pub(crate) fn count(&self, sql: &str) -> i64 {
        self.db().query_row(sql, [], |row| row.get(0)).unwrap()
    }

    pub(crate) fn id(&self) -> i64 {
        self.db()
            .query_row(
                "SELECT id FROM repositories WHERE root_path = ?1",
                [self.root.to_str().unwrap()],
                |row| row.get(0),
            )
            .unwrap()
    }

    pub(crate) fn planned(&self, name: &str, integration: Option<Integration>) -> Planned {
        let current = config(std::str::from_utf8(&self.configuration()).unwrap());
        match path_of(
            &current,
            &Change {
                name: Some(name),
                integration,
                forge: None,
            },
        ) {
            Ok(ChangePath::Setup(planned)) => *planned,
            other => panic!("not a setup: {other:?}"),
        }
    }

    pub(crate) fn first(&self, request: RequestId, planned: Planned) -> ChangeSetupRequest {
        ChangeSetupRequest {
            root: self.root.clone(),
            operation_id: OperationId::new(),
            request_id: request,
            name: planned.intended.configuration.publication_remote.clone(),
            planned: Some(planned),
            recorded: false,
        }
    }

    /// A call that publishes nothing: the session has no credentials.
    pub(crate) fn change(
        &self,
        request: ChangeSetupRequest,
    ) -> Result<SetupOutcome, crate::repository::RepositoryError> {
        self.service.change_setup(request, &|_, _| None)
    }

    pub(crate) fn accept(&self, request: RequestId, command: &str) {
        self.accept_with(request, command, OperationId::new());
    }

    /// An accepted request of `command` whose one local operation is
    /// `operation`.
    pub(crate) fn accept_with(&self, request: RequestId, command: &str, operation: OperationId) {
        let scope = ScopeKey::for_repository(&self.root).unwrap();
        let digest =
            IntentDigest::builder(DigestSalt::Request(request), command, &scope, "").finish();
        self.service
            .insert_request(&NewRequest {
                request_id: request,
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
                    operation_id: operation,
                }],
            })
            .unwrap();
    }

    /// A parent of a close or a promotion, at `step`.
    pub(crate) fn parent(&self, table: &str, step: &str) {
        let (column, value) = if table == "ticket_closures" {
            (", closure_kind", ", 'record'")
        } else {
            ("", "")
        };
        self.db()
            .execute(
                &format!(
                    "INSERT INTO {table}
                       (repository_id, operation_id, item_id, expectation, step, updated_at{column})
                     VALUES (?1, ?2, ?3, zeroblob(32), ?4, 0{value})"
                ),
                params![
                    self.id(),
                    OperationId::new().to_string(),
                    crate::canonical::ItemId::generate().to_string(),
                    step
                ],
            )
            .unwrap();
    }

    pub(crate) fn row(&self) -> Option<(String, Option<String>)> {
        self.db()
            .query_row(
                "SELECT state, completed_step FROM operation_records
                  WHERE action = 'set_publication_remote' ORDER BY id DESC LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .ok()
    }

    /// The local primary, its configuration and its registration are as they
    /// were: a pull-request path never commits to the primary.
    pub(crate) fn assert_primary_untouched(&self, what: &str, configuration: &[u8]) {
        assert_eq!(self.primary(), self.head, "{what}: the primary moved");
        assert_eq!(self.configuration(), configuration, "{what}: the file");
        assert_eq!(
            self.count("SELECT COUNT(*) FROM repositories WHERE config_blob_oid IS NULL"),
            0,
            "{what}: the registration lost its configuration blob"
        );
        assert!(
            self.git()
                .references_glob("refs/heads/manyhands/setup/*")
                .unwrap()
                .next()
                .is_none(),
            "{what}: the change made a setup branch itself"
        );
    }
}

const R1: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX1";
const R2: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX2";
const R3: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX3";

fn request(id: &str) -> RequestId {
    RequestId::parse(id).unwrap()
}

/// The direct path: a commit on the primary, with a forge change in the same
/// commit, and nothing written for an equal configuration.
fn the_direct_path_commits_the_forge_with_the_remote() {
    // Remote and forge in one commit.
    let w = World::new(DIRECT_ORIGIN);
    let before = w.primary();
    let forge = gitlab();
    let outcome = w
        .service
        .set_publication_remote_with(
            SetPublicationRemoteRequest {
                root: w.root.clone(),
                name: Some("backup".into()),
                operation_id: OperationId::new(),
            },
            Some(Some(&forge)),
            None,
        )
        .unwrap();
    let PublicationRemoteOutcome::Changed { commit_oid } = outcome else {
        panic!("{outcome:?}")
    };
    let git = w.git();
    let commit = git.find_commit(commit_oid).unwrap();
    assert_eq!(commit.parent_id(0).unwrap(), before, "one commit");
    let written = config(std::str::from_utf8(&w.configuration()).unwrap());
    assert_eq!(written.publication_remote.as_deref(), Some("backup"));
    assert_eq!(written.forge(), Some(forge.clone()));
    assert_eq!(written.integration(), Integration::Direct);
    assert_eq!(written.unknown["extra"], toml::Value::Boolean(true));
    assert_eq!(w.count("SELECT COUNT(*) FROM repository_setups"), 0);

    // The same remote and a changed forge: the forge alone is a change.
    let w = World::new(DIRECT_ORIGIN_FORGE);
    let before = w.primary();
    let outcome = w
        .service
        .set_publication_remote_with(
            SetPublicationRemoteRequest {
                root: w.root.clone(),
                name: Some("origin".into()),
                operation_id: OperationId::new(),
            },
            Some(None),
            None,
        )
        .unwrap();
    assert!(matches!(outcome, PublicationRemoteOutcome::Changed { .. }));
    assert_ne!(w.primary(), before, "a removed forge is a commit");
    let written = config(std::str::from_utf8(&w.configuration()).unwrap());
    assert_eq!(written.forge(), None);
    assert!(!written.unknown.contains_key("forge"));

    // An equal configuration, forge and all: nothing is committed.
    let w = World::new(DIRECT_ORIGIN_FORGE);
    let before = w.primary();
    let same = github();
    let outcome = w
        .service
        .set_publication_remote_with(
            SetPublicationRemoteRequest {
                root: w.root.clone(),
                name: Some("origin".into()),
                operation_id: OperationId::new(),
            },
            Some(Some(&same)),
            None,
        )
        .unwrap();
    // An equal selection names the head it found, as it always did.
    assert!(
        matches!(outcome, PublicationRemoteOutcome::Changed { commit_oid } if commit_oid == before),
        "{outcome:?}"
    );
    assert_eq!(w.primary(), before, "nothing is committed");
}

/// The setup path's recording, and its race rule (design Part B).
fn the_pull_request_path_records_a_change_and_never_commits() {
    // A first call records, once.
    let w = World::new(DIRECT_ORIGIN);
    let configuration = w.configuration();
    let blob = w.blob();
    let planned = w.planned("origin", Some(Integration::PullRequest));
    let intended = bytes(&planned.intended.configuration);
    let operation = OperationId::new();
    let outcome = w
        .change(ChangeSetupRequest {
            operation_id: operation,
            ..w.first(request(R1), planned.clone())
        })
        .unwrap();
    assert!(
        matches!(
            outcome,
            SetupOutcome::Recorded {
                stop: ResultCode::InternalError,
                ..
            }
        ),
        "{outcome:?}"
    );
    let record = record::read(&w.db(), w.id()).unwrap().expect("a record");
    assert_eq!(record.kind, SetupKind::Change);
    assert_eq!(record.remote_name, "origin");
    assert_eq!(record.primary_branch, "main");
    assert_eq!(record.base_configuration_oid, Some(blob));
    assert_eq!(record.configuration, intended);
    assert_eq!(record.request_id, request(R1));
    assert_eq!(
        w.row(),
        Some(("completed".to_owned(), Some("setup_recorded".to_owned())))
    );
    w.assert_primary_untouched("a first call", &configuration);

    // The same request again records nothing: its own record continues.
    // (The same request is the same operation: its row is the one it had.)
    let again = w
        .change(ChangeSetupRequest {
            operation_id: operation,
            ..w.first(request(R1), planned.clone())
        })
        .unwrap();
    assert!(matches!(again, SetupOutcome::Recorded { .. }), "{again:?}");
    let recorded = ChangeSetupRequest {
        operation_id: operation,
        planned: None,
        recorded: true,
        ..w.first(request(R1), planned.clone())
    };
    let continued = w.change(recorded).unwrap();
    assert!(
        matches!(continued, SetupOutcome::Recorded { .. }),
        "{continued:?}"
    );
    assert_eq!(w.count("SELECT COUNT(*) FROM repository_setups"), 1);
    assert_eq!(
        record::read(&w.db(), w.id()).unwrap().unwrap().setup_id,
        record.setup_id
    );
    w.assert_primary_untouched("the same request", &configuration);

    // Another request's setup is pending: `setup_pending`, finally, with
    // nothing recorded for it.
    let other = w.change(w.first(request(R2), planned)).unwrap();
    assert!(matches!(other, SetupOutcome::AnotherSetup), "{other:?}");
    assert_eq!(w.count("SELECT COUNT(*) FROM repository_setups"), 1);
    assert_eq!(
        record::read(&w.db(), w.id()).unwrap().unwrap().request_id,
        request(R1)
    );
    assert_eq!(
        w.count("SELECT COUNT(*) FROM operation_records WHERE state != 'completed'"),
        0
    );

    // Part B's race rule: what is in the change's way, `recovery_required`
    // and nothing recorded; the change's own request is not in its way.
    type Block = fn(&World);
    let in_the_way: Vec<(&str, Block)> = vec![
        ("an unfinished close", |w| {
            w.parent("ticket_closures", "confirmed")
        }),
        ("an unfinished promotion", |w| {
            w.parent("document_promotions", "integrating");
        }),
        ("an accepted close", |w| {
            w.accept(request(R3), "ticket close")
        }),
        ("an accepted promotion", |w| {
            w.accept(request(R3), "document promote")
        }),
        ("another accepted selection", |w| {
            w.accept(request(R3), "remote select")
        }),
    ];
    for (what, block) in in_the_way {
        let w = World::new(DIRECT_ORIGIN);
        let configuration = w.configuration();
        let planned = w.planned("origin", Some(Integration::PullRequest));
        // The change's own request is accepted, as the binding has it.
        w.accept(request(R1), "remote select");
        block(&w);
        let outcome = w.change(w.first(request(R1), planned.clone())).unwrap();
        assert!(
            matches!(outcome, SetupOutcome::InTheWay { .. }),
            "{what}: {outcome:?}"
        );
        assert_eq!(
            w.count("SELECT COUNT(*) FROM repository_setups"),
            0,
            "{what}"
        );
        assert_eq!(
            w.count("SELECT COUNT(*) FROM operation_records WHERE state != 'completed'"),
            0,
            "{what}: no operation is left in flight"
        );
        w.assert_primary_untouched(what, &configuration);
    }
    // A finished close, and the change's own accepted request alone, are
    // not in the way.
    let w = World::new(DIRECT_ORIGIN);
    let planned = w.planned("origin", Some(Integration::PullRequest));
    w.accept(request(R1), "remote select");
    w.parent("ticket_closures", "complete");
    w.parent("document_promotions", "superseded");
    let outcome = w.change(w.first(request(R1), planned)).unwrap();
    assert!(
        matches!(outcome, SetupOutcome::Recorded { .. }),
        "{outcome:?}"
    );
    assert_eq!(w.count("SELECT COUNT(*) FROM repository_setups"), 1);
}

/// The checks before the recording refuse as a selection does, and record
/// nothing.
fn the_checks_before_the_recording_refuse_and_record_nothing() {
    // Another remote than the planned one is not selectable: it does not
    // exist, or is not an SSH one.
    for (what, name, kind) in [
        (
            "a remote that is missing",
            "nowhere",
            RepositoryErrorKind::UnavailablePublicationRemote,
        ),
        (
            "a remote that is not SSH",
            "web",
            RepositoryErrorKind::InvalidPublicationRemote,
        ),
    ] {
        let w = World::new(DIRECT_ORIGIN);
        let configuration = w.configuration();
        let planned = w.planned(name, Some(Integration::PullRequest));
        let error = w.change(w.first(request(R1), planned)).unwrap_err();
        assert_eq!(error.kind, kind, "{what}");
        assert_eq!(
            w.count("SELECT COUNT(*) FROM repository_setups"),
            0,
            "{what}"
        );
        w.assert_primary_untouched(what, &configuration);
    }
    // The configuration is not the one the change was derived from.
    let w = World::new(DIRECT_ORIGIN);
    let planned = w.planned("origin", Some(Integration::PullRequest));
    let other = w.git();
    let mut changed = config(DIRECT_ORIGIN);
    changed
        .unknown
        .insert("extra".into(), toml::Value::Boolean(false));
    std::fs::write(w.root.join(canonical::CONFIG_PATH), bytes(&changed)).unwrap();
    let mut index = other.index().unwrap();
    index.add_path(Path::new(canonical::CONFIG_PATH)).unwrap();
    index.write().unwrap();
    let tree = other.find_tree(index.write_tree().unwrap()).unwrap();
    let signature = Signature::new("Fixture", "fixture@example.invalid", &Time::new(1, 0)).unwrap();
    let parent = other.head().unwrap().peel_to_commit().unwrap();
    other
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Another change",
            &tree,
            &[&parent],
        )
        .unwrap();
    let error = w.change(w.first(request(R1), planned)).unwrap_err();
    assert_eq!(error.kind, RepositoryErrorKind::ExternalChange);
    assert_eq!(w.count("SELECT COUNT(*) FROM repository_setups"), 0);
    // The configuration path is not clean.
    let w = World::new(DIRECT_ORIGIN);
    let planned = w.planned("origin", Some(Integration::PullRequest));
    let mut edited = config(DIRECT_ORIGIN);
    edited
        .unknown
        .insert("extra".into(), toml::Value::Boolean(false));
    std::fs::write(w.root.join(canonical::CONFIG_PATH), bytes(&edited)).unwrap();
    let error = w.change(w.first(request(R1), planned)).unwrap_err();
    assert_eq!(error.kind, RepositoryErrorKind::DirtyConfigurationPath);
    assert_eq!(w.count("SELECT COUNT(*) FROM repository_setups"), 0);
    // No identity to commit as: nothing is recorded.
    let w = World::new(DIRECT_ORIGIN);
    let planned = w.planned("origin", Some(Integration::PullRequest));
    let mut git = w
        .git()
        .config()
        .unwrap()
        .open_level(git2::ConfigLevel::Local)
        .unwrap();
    git.remove("user.name").unwrap();
    git.remove("user.email").unwrap();
    let outcome = w.change(w.first(request(R1), planned)).unwrap();
    assert!(
        matches!(outcome, SetupOutcome::IdentityRequired),
        "{outcome:?}"
    );
    assert_eq!(w.count("SELECT COUNT(*) FROM repository_setups"), 0);
}

/// Design Part F: the path of a `remote select`, from the table over the
/// configuration to the library's recording.
#[test]
fn remote_select_takes_the_path_of_the_design() {
    the_codes_of_a_refusal();
    the_path_of_each_row();
    the_direct_path_commits_the_forge_with_the_remote();
    the_pull_request_path_records_a_change_and_never_commits();
    the_checks_before_the_recording_refuse_and_record_nothing();
}

/// A channel a call that never reaches the remote does not use: publishing a
/// setup that is gone answers its request's stored result before it connects.
pub(crate) struct Never;

impl SetupChannel for Never {
    fn fetch_base(&mut self, _: &SetupScope) -> Result<SetupAdvertisement, SshTransportError> {
        panic!("a setup that is gone is not fetched");
    }

    fn push_branch(&mut self, _: &SetupScope, _: Oid) -> Result<SetupPush, SshTransportError> {
        panic!("a setup that is gone is not pushed");
    }

    fn delete_branch(
        &mut self,
        _: &SetupScope,
        _: Oid,
    ) -> Result<DeleteAttempt, SshTransportError> {
        panic!("a setup that is gone is not deleted");
    }

    fn observe_branch(&mut self, _: &SetupScope) -> Result<Option<Oid>, SshTransportError> {
        panic!("a setup that is gone is not observed");
    }
}

/// The removal of a setup's record, as the ending of a refused setup does it
/// (R-7): the request is finished in the same transaction.
pub(crate) fn end_the_setup(service: &RepositoryService, root: &Path, request: RequestId) {
    let mut db = open_registry(&service.registry_path, &mut |_| {}).unwrap();
    let id: i64 = db
        .query_row(
            "SELECT id FROM repositories WHERE root_path = ?1",
            [root.to_str().unwrap()],
            |row| row.get(0),
        )
        .unwrap();
    let tx = db.transaction().unwrap();
    assert!(
        crate::repository::mutation::finish_setup_request(
            &tx,
            id,
            request,
            ResultCode::RemoteAlreadySetUp,
            None,
            1_700_000_000,
        )
        .unwrap()
    );
    record::delete(&tx, id).unwrap();
    tx.commit().unwrap();
}

/// What the stored envelope of an ended setup request is, as publishing
/// answers it for a setup that is gone.
fn ended_with_the_stored_envelope(outcome: &SetupOutcome) -> bool {
    matches!(
        outcome,
        SetupOutcome::Ended(SetupError::Ended(result)) if result.code == ResultCode::RemoteAlreadySetUp
    )
}

/// Contract 13 and decision 8: a direct selection that finds a pull-request
/// primary under the lease commits nothing.
#[test]
fn a_direct_selection_refuses_a_pull_request_primary_under_the_lease() {
    let forge = gitlab();
    let rows: Vec<(&str, Change<'_>)> = vec![
        (
            "another remote",
            Change {
                name: Some("backup"),
                integration: None,
                forge: None,
            },
        ),
        (
            "a forge change",
            Change {
                name: Some("origin"),
                integration: None,
                forge: Some(Some(&forge)),
            },
        ),
    ];
    for (what, change) in rows {
        let w = World::new(PR_ORIGIN);
        let configuration = w.configuration();
        let error = w
            .service
            .set_publication_remote_with(
                SetPublicationRemoteRequest {
                    root: w.root.clone(),
                    name: change.name.map(str::to_owned),
                    operation_id: OperationId::new(),
                },
                change.forge,
                Some(&change),
            )
            .unwrap_err();
        assert_eq!(error.kind, RepositoryErrorKind::ExternalChange, "{what}");
        assert_eq!(w.primary(), w.head, "{what}: the primary moved");
        assert_eq!(w.configuration(), configuration, "{what}");
        assert_eq!(
            w.count("SELECT COUNT(*) FROM operation_records WHERE state != 'completed'"),
            0,
            "{what}: no operation is left in flight"
        );
    }
    // The guard lets through what is a direct selection, and an equal one.
    let w = World::new(DIRECT_ORIGIN);
    let change = Change {
        name: Some("backup"),
        integration: None,
        forge: None,
    };
    let outcome = w
        .service
        .set_publication_remote_with(
            SetPublicationRemoteRequest {
                root: w.root.clone(),
                name: Some("backup".into()),
                operation_id: OperationId::new(),
            },
            None,
            Some(&change),
        )
        .unwrap();
    assert!(matches!(outcome, PublicationRemoteOutcome::Changed { .. }));
    assert_ne!(w.primary(), w.head);
    let w = World::new(PR_ORIGIN);
    let change = Change {
        name: Some("origin"),
        integration: None,
        forge: None,
    };
    let outcome = w
        .service
        .set_publication_remote_with(
            SetPublicationRemoteRequest {
                root: w.root.clone(),
                name: Some("origin".into()),
                operation_id: OperationId::new(),
            },
            None,
            Some(&change),
        )
        .unwrap();
    assert!(matches!(outcome, PublicationRemoteOutcome::Changed { .. }));
    assert_eq!(w.primary(), w.head, "an equal selection commits nothing");
}

/// Never a second setup (L11): a call that is stale, because its request
/// recorded its setup and the setup ended meanwhile, finds its own row at
/// `setup_recorded` and records nothing: publishing answers the request's
/// stored envelope. The same holds for a call that is told the setup is
/// recorded, whose row's target is the remote it selects, not the record's.
#[test]
fn a_stale_first_call_never_records_a_second_change() {
    let w = World::new(DIRECT_ORIGIN);
    let planned = w.planned("origin", Some(Integration::PullRequest));
    let operation = OperationId::new();
    w.accept_with(request(R1), "remote select", operation);
    let first = w
        .change(ChangeSetupRequest {
            operation_id: operation,
            ..w.first(request(R1), planned.clone())
        })
        .unwrap();
    assert!(matches!(first, SetupOutcome::Recorded { .. }), "{first:?}");
    end_the_setup(&w.service, &w.root, request(R1));
    assert_eq!(w.count("SELECT COUNT(*) FROM repository_setups"), 0);

    let publish = |root: &Path, request_id: RequestId| {
        Some(continue_setup(&w.service, root, request_id, &mut Never))
    };
    // The stale call: it read its row before the first call recorded.
    let stale = w
        .service
        .change_setup(
            ChangeSetupRequest {
                operation_id: operation,
                ..w.first(request(R1), planned)
            },
            &publish,
        )
        .unwrap();
    assert!(ended_with_the_stored_envelope(&stale), "{stale:?}");
    assert_eq!(
        w.count("SELECT COUNT(*) FROM repository_setups"),
        0,
        "no second record"
    );
    assert_eq!(
        w.count("SELECT COUNT(*) FROM operation_records WHERE state != 'completed'"),
        0
    );
    w.assert_primary_untouched("the stale call", &w.configuration());

    // A call told the setup is recorded, with the record gone: the stored
    // envelope, and not an operation mismatch.
    let recorded = w
        .service
        .change_setup(
            ChangeSetupRequest {
                root: w.root.clone(),
                operation_id: operation,
                request_id: request(R1),
                name: Some("origin".to_owned()),
                planned: None,
                recorded: true,
            },
            &publish,
        )
        .unwrap();
    assert!(ended_with_the_stored_envelope(&recorded), "{recorded:?}");
}
