//! What the key verbs open: a registry this service migrated already is
//! opened as it is, and one whose file or schema changed is migrated again.
use std::fs;

use crate::repository::{REGISTRY_FILE, RepositoryService, discovery::migrations_for_testing};

#[test]
fn key_listing_after_open_runs_no_migration() {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    // Registry open's own migration ran before this count begins.
    let before = migrations_for_testing();
    for _ in 0..3 {
        assert!(service.list_shared_keys().unwrap().is_empty());
    }
    assert_eq!(migrations_for_testing() - before, 0);
}

#[test]
fn a_replaced_registry_file_migrates_again() {
    for change in ["replaced file", "changed schema"] {
        let data = tempfile::tempdir().unwrap();
        let registry = data.path().join(REGISTRY_FILE);
        let service = RepositoryService::open_at(data.path()).unwrap();
        service.list_shared_keys().unwrap();
        let before = migrations_for_testing();
        match change {
            // The same contents under a new file: its identity differs.
            "replaced file" => {
                let copy = data.path().join("replacement.sqlite3");
                rusqlite::Connection::open(&registry)
                    .unwrap()
                    .execute("VACUUM INTO ?1", [copy.to_str().unwrap()])
                    .unwrap();
                for suffix in ["-wal", "-shm"] {
                    let _ = fs::remove_file(data.path().join(format!("{REGISTRY_FILE}{suffix}")));
                }
                fs::rename(&copy, &registry).unwrap();
            }
            // The same file with a schema another connection changed.
            _ => rusqlite::Connection::open(&registry)
                .unwrap()
                .execute_batch("CREATE TABLE unrelated_fixture(value)")
                .unwrap(),
        }
        assert!(service.list_shared_keys().unwrap().is_empty(), "{change}");
        assert_eq!(migrations_for_testing() - before, 1, "{change}");
        service.list_shared_keys().unwrap();
        assert_eq!(migrations_for_testing() - before, 1, "{change}");
    }
}
