use super::*;

#[test]
fn imported_observation_checks_regular_directory_and_missing_sources() {
    let home = tempfile::tempdir().unwrap();
    let source = home.path().join("imported");
    std::fs::write(&source, b"unparsed imported material").unwrap();
    assert!(observe_regular_source(&source).is_ok());
    assert_eq!(
        observe_regular_source(home.path()).err().unwrap().kind,
        KeyMaterialErrorKind::NotRegularFile
    );
    assert_eq!(
        observe_regular_source(&home.path().join("missing"))
            .err()
            .unwrap()
            .kind,
        KeyMaterialErrorKind::SourceMissing
    );
    assert!(!home.path().join(".ssh").exists());
}

#[test]
fn refuses_unsafe_existing_store() {
    let home = tempfile::tempdir().unwrap();
    let store = KeyStore::for_home(home.path()).unwrap();
    drop(store.lock().unwrap());
    platform::set_test_security(&home.path().join(".ssh/manyhands"), "D:P(A;;FA;;;WD)");
    assert_eq!(
        store.lock().err().unwrap().kind,
        KeyMaterialErrorKind::ProtectionUnavailable
    );
}

#[test]
fn refuses_inherited_everyone_grants_and_null_dacl() {
    for dacl in ["D:AI(A;ID;FA;;;WD)", "D:NO_ACCESS_CONTROL"] {
        let home = tempfile::tempdir().unwrap();
        let store = KeyStore::for_home(home.path()).unwrap();
        drop(store.lock().unwrap());
        platform::set_test_security(&home.path().join(".ssh/manyhands"), dacl);
        assert_eq!(
            store.lock().err().unwrap().kind,
            KeyMaterialErrorKind::ProtectionUnavailable
        );
    }
}

#[test]
fn validates_explicit_current_owner_and_rejects_another_owner() {
    assert!(platform::test_security_descriptor_is_accepted(
        None,
        "D:P(A;;FA;;;CURRENT)"
    ));
    assert!(!platform::test_security_descriptor_is_accepted(
        Some("S-1-1-0"),
        "D:P(A;;FA;;;CURRENT)"
    ));
    assert!(!platform::test_security_descriptor_is_accepted(
        None,
        "D:P(A;;FA;;;CURRENT)(A;;FR;;;WD)"
    ));
    assert!(!platform::test_security_descriptor_is_accepted(
        None,
        "D:P(A;ID;FA;;;CURRENT)"
    ));
}

#[test]
fn refuses_private_hardlinks() {
    let home = tempfile::tempdir().unwrap();
    let store = KeyStore::for_home(home.path()).unwrap();
    let guard = store.lock().unwrap();
    let id = SharedKeyId::new();
    let mut file = guard.create_private(id).unwrap();
    std::fs::hard_link(store.key_paths(id).0, home.path().join("alias")).unwrap();
    assert!(guard.open_owned(id, KeyFileKind::Private).is_err());
    assert!(file.write_all_and_sync(b"must not write").is_err());
    assert!(std::fs::read(home.path().join("alias")).unwrap().is_empty());
}

#[test]
fn refuses_symlink_or_reparse_substitution() {
    let home = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    match std::os::windows::fs::symlink_dir(other.path(), home.path().join(".ssh")) {
        Ok(()) => {
            assert!(KeyStore::for_home(home.path()).unwrap().lock().is_err());
            assert_eq!(std::fs::read_dir(other.path()).unwrap().count(), 0);
        }
        Err(e) if e.raw_os_error() == Some(1314) => {
            eprintln!(
                "fixture limitation: Windows runner lacks symlink privilege; deterministic reparse attribute rejection remains covered"
            );
        }
        Err(e) => panic!("could not create reparse fixture: {e}"),
    }
    assert!(!platform::test_attributes_are_accepted(0x400));
    assert!(platform::test_attributes_are_accepted(0x80));
}
