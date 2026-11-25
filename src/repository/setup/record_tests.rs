use super::*;
use crate::repository::{
    CommitIdentity, EnableRepositoryRequest, OperationId, REGISTRY_FILE, RebuildRepositoryRequest,
    RepositoryService, discovery::migrate_registry, open_registry, register_setup_pending,
};
use rusqlite::types::ValueRef;
use std::path::{Path, PathBuf};

const SETUP: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
const OTHER_SETUP: &str = "01BX5ZZKBKACTAV9WEVGEMMVRZ";

fn db() -> Connection {
    let mut db = Connection::open_in_memory().unwrap();
    migrate_registry(&mut db).unwrap();
    for (id, root) in [(1, "/fixture"), (2, "/other")] {
        db.execute(
            "INSERT INTO repositories(id,root_path,enabled_at,accessibility,refresh_required)
             VALUES(?1,?2,123,'accessible',0)",
            params![id, root],
        )
        .unwrap();
    }
    db
}

fn oid(digit: char) -> Oid {
    Oid::from_str(&digit.to_string().repeat(40)).unwrap()
}

fn record(kind: SetupKind) -> SetupRecord {
    SetupRecord {
        setup_id: SETUP.parse().unwrap(),
        kind,
        remote_name: "origin".into(),
        primary_branch: "main".into(),
        endpoint_digest: [7; 32],
        configuration: b"primary_branch = \"main\"\n".to_vec(),
        base_configuration_oid: (kind == SetupKind::Change).then(|| oid('a')),
        base_oid: None,
        commit_oid: None,
        published_oid: None,
        request_id: RequestId::new(),
        created_at: 1_700_000_000,
    }
}

fn insert_in(db: &mut Connection, repository_id: i64, record: &SetupRecord) -> Result<(), ()> {
    let tx = db.transaction().unwrap();
    let result = insert(&tx, repository_id, record).map_err(drop);
    tx.commit().unwrap();
    result
}

fn rows(db: &Connection) -> i64 {
    db.query_row("SELECT COUNT(*) FROM repository_setups", [], |row| {
        row.get(0)
    })
    .unwrap()
}

#[test]
fn setup_records_round_trip_and_enforce_their_rules() {
    let mut db = db();
    // Each kind is read back as it was written, with its state.
    for (repository, kind) in [(1, SetupKind::Enable), (2, SetupKind::Change)] {
        let mut written = record(kind);
        if repository == 2 {
            written.setup_id = OTHER_SETUP.parse().unwrap();
        }
        insert_in(&mut db, repository, &written).unwrap();
        let read = read(&db, repository).unwrap().unwrap();
        assert_eq!(read, written);
        assert_eq!(read.state(), SetupState::Recorded);
    }
    assert_eq!(super::read(&db, 3).unwrap(), None);
    // One setup per repository: a second is refused, whatever it holds.
    let second = SetupRecord {
        setup_id: "01CZ3NDEKTSV4RRFFQ69G5FAVX".parse().unwrap(),
        ..record(SetupKind::Enable)
    };
    assert!(insert_in(&mut db, 1, &second).is_err());
    // Each rule of the table, as a record that breaks it.
    let cases: Vec<(&str, SetupRecord)> = vec![
        (
            "a change without its base configuration",
            SetupRecord {
                base_configuration_oid: None,
                ..record(SetupKind::Change)
            },
        ),
        (
            "an enable that names a base configuration",
            SetupRecord {
                base_configuration_oid: Some(oid('a')),
                ..record(SetupKind::Enable)
            },
        ),
        (
            "a commit without a base",
            SetupRecord {
                commit_oid: Some(oid('c')),
                ..record(SetupKind::Enable)
            },
        ),
        (
            "a base without a commit",
            SetupRecord {
                base_oid: Some(oid('b')),
                ..record(SetupKind::Enable)
            },
        ),
        (
            "a published commit that is not the setup commit",
            SetupRecord {
                base_oid: Some(oid('b')),
                commit_oid: Some(oid('c')),
                published_oid: Some(oid('d')),
                ..record(SetupKind::Enable)
            },
        ),
        (
            "a published commit with no setup commit",
            SetupRecord {
                published_oid: Some(oid('d')),
                ..record(SetupKind::Enable)
            },
        ),
        (
            "an all-zero object ID",
            SetupRecord {
                base_oid: Some(Oid::zero()),
                commit_oid: Some(oid('c')),
                ..record(SetupKind::Enable)
            },
        ),
    ];
    for (name, case) in cases {
        let mut fresh = self::db();
        assert!(insert_in(&mut fresh, 1, &case).is_err(), "{name}");
        assert_eq!(rows(&fresh), 0, "{name}");
    }
    // Rules a typed record cannot break are held by the table itself.
    let fresh = self::db();
    for (name, sql) in [
        (
            "a non-hex object ID",
            "INSERT INTO repository_setups VALUES (1,'01ARZ3NDEKTSV4RRFFQ69G5FAV','change','origin','main',zeroblob(32),x'',
             'zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz',NULL,NULL,NULL,'01ARZ3NDEKTSV4RRFFQ69G5FAV',1)",
        ),
        (
            "an upper-case object ID",
            "INSERT INTO repository_setups VALUES (1,'01ARZ3NDEKTSV4RRFFQ69G5FAV','change','origin','main',zeroblob(32),x'',
             'AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA',NULL,NULL,NULL,'01ARZ3NDEKTSV4RRFFQ69G5FAV',1)",
        ),
        (
            "a digest of 31 bytes",
            "INSERT INTO repository_setups VALUES (1,'01ARZ3NDEKTSV4RRFFQ69G5FAV','enable','origin','main',zeroblob(31),x'',
             NULL,NULL,NULL,NULL,'01ARZ3NDEKTSV4RRFFQ69G5FAV',1)",
        ),
        (
            "a digest of 33 bytes",
            "INSERT INTO repository_setups VALUES (1,'01ARZ3NDEKTSV4RRFFQ69G5FAV','enable','origin','main',zeroblob(33),x'',
             NULL,NULL,NULL,NULL,'01ARZ3NDEKTSV4RRFFQ69G5FAV',1)",
        ),
        (
            "a digest stored as text",
            "INSERT INTO repository_setups VALUES (1,'01ARZ3NDEKTSV4RRFFQ69G5FAV','enable','origin','main',
             '01234567890123456789012345678901',x'',NULL,NULL,NULL,NULL,'01ARZ3NDEKTSV4RRFFQ69G5FAV',1)",
        ),
        (
            "an unknown kind",
            "INSERT INTO repository_setups VALUES (1,'01ARZ3NDEKTSV4RRFFQ69G5FAV','other','origin','main',zeroblob(32),x'',
             NULL,NULL,NULL,NULL,'01ARZ3NDEKTSV4RRFFQ69G5FAV',1)",
        ),
    ] {
        assert!(fresh.execute(sql, []).is_err(), "{name}");
    }
    // The commit and its publication are recorded once, in order, and
    // repeating a step is harmless.
    let mut db = self::db();
    insert_in(&mut db, 1, &record(SetupKind::Enable)).unwrap();
    let tx = db.transaction().unwrap();
    assert!(record_published(&tx, 1, oid('c')).is_err(), "no commit yet");
    record_commit(&tx, 1, oid('b'), oid('c')).unwrap();
    record_commit(&tx, 1, oid('b'), oid('c')).unwrap();
    assert!(record_commit(&tx, 1, oid('b'), oid('d')).is_err());
    assert!(record_published(&tx, 1, oid('d')).is_err());
    assert!(
        record_commit(&tx, 2, oid('b'), oid('c')).is_err(),
        "no record"
    );
    record_published(&tx, 1, oid('c')).unwrap();
    record_published(&tx, 1, oid('c')).unwrap();
    tx.commit().unwrap();
    let read = super::read(&db, 1).unwrap().unwrap();
    assert_eq!(
        (
            read.base_oid,
            read.commit_oid,
            read.published_oid,
            read.state()
        ),
        (
            Some(oid('b')),
            Some(oid('c')),
            Some(oid('c')),
            SetupState::Published
        )
    );
    // A row that is none is corruption, not a record.
    db.execute_batch("PRAGMA ignore_check_constraints = ON")
        .unwrap();
    db.execute("UPDATE repository_setups SET setup_id = 'not a ulid'", [])
        .unwrap();
    assert!(super::read(&db, 1).is_err());
    // Delete says whether there was a record.
    let tx = db.transaction().unwrap();
    assert!(delete(&tx, 1).unwrap());
    assert!(!delete(&tx, 1).unwrap());
    tx.commit().unwrap();
    assert_eq!(rows(&db), 0);
}

#[test]
fn registering_a_setup_adds_the_repository_without_a_configuration_blob() {
    let mut db = db();
    let tx = db.transaction().unwrap();
    let id = register_setup_pending(&tx, Path::new("/new")).unwrap();
    // Again is the same registration, and an existing one keeps its blob.
    assert_eq!(register_setup_pending(&tx, Path::new("/new")).unwrap(), id);
    tx.execute(
        "UPDATE repositories SET config_blob_oid = ?2 WHERE id = ?1",
        params![id, "a".repeat(40)],
    )
    .unwrap();
    assert_eq!(register_setup_pending(&tx, Path::new("/new")).unwrap(), id);
    tx.commit().unwrap();
    let (blob, accessibility, refresh): (Option<String>, String, i64) = db
        .query_row(
            "SELECT config_blob_oid, accessibility, refresh_required FROM repositories WHERE id = ?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(
        (blob, accessibility.as_str(), refresh),
        (Some("a".repeat(40)), "accessible", 1)
    );
    let fresh = {
        let tx = db.transaction().unwrap();
        let id = register_setup_pending(&tx, Path::new("/newer")).unwrap();
        tx.commit().unwrap();
        id
    };
    let blob: Option<String> = db
        .query_row(
            "SELECT config_blob_oid FROM repositories WHERE id = ?1",
            [fresh],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(blob, None);
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM repositories", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        4
    );
}

struct Fixture {
    service: RepositoryService,
    data: tempfile::TempDir,
    root: PathBuf,
    _directory: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap();
        let data = tempfile::tempdir().unwrap();
        let service = RepositoryService::open_at(data.path()).unwrap();
        let mut options = git2::RepositoryInitOptions::new();
        options.initial_head("main");
        drop(git2::Repository::init_opts(&root, &options).unwrap());
        service
            .enable(EnableRepositoryRequest {
                root: root.clone(),
                primary_branch: "main".into(),
                identity: Some(CommitIdentity {
                    name: "Setup Test".into(),
                    email: "setup@example.invalid".into(),
                }),
                operation_id: OperationId::new(),
            })
            .unwrap();
        Self {
            service,
            data,
            root,
            _directory: directory,
        }
    }

    fn db(&self) -> Connection {
        open_registry(&self.data.path().join(REGISTRY_FILE), &mut |_| {}).unwrap()
    }
}

#[test]
fn a_setup_record_survives_a_rebuild_and_goes_with_remove() {
    let f = Fixture::new();
    let mut db = f.db();
    let id: i64 = db
        .query_row("SELECT id FROM repositories", [], |row| row.get(0))
        .unwrap();
    let written = record(SetupKind::Enable);
    insert_in(&mut db, id, &written).unwrap();
    drop(db);
    f.service
        .rebuild_repository(RebuildRepositoryRequest {
            root: f.root.clone(),
            operation_id: OperationId::new(),
        })
        .unwrap();
    assert_eq!(read(&f.db(), id).unwrap(), Some(written));
    f.service
        .remove_registration(crate::repository::RemoveRegistrationRequest {
            root: f.root.clone(),
            operation_id: OperationId::new(),
        })
        .unwrap();
    assert_eq!(rows(&f.db()), 0);
}

#[test]
fn a_setup_record_holds_no_remote_url_credential_or_identity() {
    const URL: &str = "ssh://git@sentinel-host.invalid/secret/repository.git";
    const NAME: &str = "Identity Sentinel";
    const EMAIL: &str = "identity-sentinel@example.invalid";
    const PASSPHRASE: &str = "passphrase-sentinel";
    let mut db = db();
    // The fixture's repository has the remote and the identity configured;
    // a record is made from it as the setup operations will make one.
    let directory = tempfile::tempdir().unwrap();
    let git = git2::Repository::init(directory.path()).unwrap();
    git.remote("origin", URL).unwrap();
    git.config().unwrap().set_str("user.name", NAME).unwrap();
    git.config().unwrap().set_str("user.email", EMAIL).unwrap();
    let remote = git.find_remote("origin").unwrap();
    assert_eq!(remote.url(), Some(URL));
    let mut written = record(SetupKind::Change);
    written.remote_name = remote.name().unwrap().to_owned();
    insert_in(&mut db, 1, &written).unwrap();
    let tx = db.transaction().unwrap();
    record_commit(&tx, 1, oid('b'), oid('c')).unwrap();
    record_published(&tx, 1, oid('c')).unwrap();
    tx.commit().unwrap();
    let mut statement = db.prepare("SELECT * FROM repository_setups").unwrap();
    let width = statement.column_count();
    let mut cells = Vec::new();
    let mut rows = statement.query([]).unwrap();
    while let Some(row) = rows.next().unwrap() {
        for column in 0..width {
            cells.push(match row.get_ref(column).unwrap() {
                ValueRef::Null => String::new(),
                ValueRef::Integer(value) => value.to_string(),
                ValueRef::Real(value) => value.to_string(),
                ValueRef::Text(bytes) | ValueRef::Blob(bytes) => {
                    String::from_utf8_lossy(bytes).into_owned()
                }
            });
        }
    }
    assert_eq!(cells.len(), width);
    for cell in &cells {
        for sentinel in [URL, "sentinel-host", NAME, EMAIL, PASSPHRASE, "ssh://", "@"] {
            assert!(!cell.contains(sentinel), "{sentinel:?} in {cell:?}");
        }
    }
    // Its Debug form leaves out the configuration bytes and the digest.
    let debug = format!("{:?}", super::read(&db, 1).unwrap().unwrap());
    assert!(
        !debug.contains("configuration:") && !debug.contains("endpoint_digest"),
        "{debug}"
    );
}
