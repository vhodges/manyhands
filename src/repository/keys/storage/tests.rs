use super::*;

#[test]
fn finalized_write_closes_writer_and_identity_survives_reopening() {
    let home = tempfile::tempdir().unwrap();
    let store = KeyStore::for_home(home.path()).unwrap();
    let guard = store.lock().unwrap();
    let id = SharedKeyId::new();
    let mut file = guard.create_private(id).unwrap();
    file.write_all_and_sync(b"temporary test material").unwrap();
    let identity = file.identity();
    assert!(file.write_all_and_sync(b"must not append").is_err());
    drop(file);
    let reopened = guard.open_owned(id, KeyFileKind::Private).unwrap().unwrap();
    assert_eq!(identity, reopened.identity());
    assert!(std::fs::read(store.key_paths(id).0).unwrap() == b"temporary test material");
}

#[test]
fn creates_private_storage_without_permission_window() {
    let home = tempfile::tempdir().unwrap();
    let store = KeyStore::for_home(home.path()).unwrap();
    let guard = store.lock().unwrap();
    let id = SharedKeyId::new();
    let mut private = guard.create_private(id).unwrap();
    let public = guard.create_public(id).unwrap();
    let before = private.identity();
    private
        .write_all_and_sync(b"temporary test material")
        .unwrap();
    assert_ne!(before, private.identity());
    assert_eq!(
        FileIdentity::decode(&private.identity().encode()).unwrap(),
        private.identity()
    );
    assert!(
        guard
            .open_owned(id, KeyFileKind::Private)
            .unwrap()
            .is_some()
    );
    guard.sync_directory().unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let (private_path, public_path) = store.key_paths(id);
        for (path, mode) in [
            (home.path().join(".ssh"), 0o700),
            (home.path().join(".ssh/manyhands"), 0o700),
            (private_path, 0o600),
            (public_path, 0o644),
        ] {
            let metadata = std::fs::metadata(path).unwrap();
            assert_eq!(metadata.mode() & 0o777, mode);
            assert_eq!(metadata.uid(), unsafe { libc::geteuid() });
        }
    }
    private.remove().unwrap();
    public.remove().unwrap();
    assert!(
        guard
            .open_owned(id, KeyFileKind::Private)
            .unwrap()
            .is_none()
    );
}

#[test]
fn refuses_existing_private_path() {
    let home = tempfile::tempdir().unwrap();
    let store = KeyStore::for_home(home.path()).unwrap();
    let guard = store.lock().unwrap();
    let id = SharedKeyId::new();
    let mut file = guard.create_private(id).unwrap();
    file.write_all_and_sync(b"existing sentinel").unwrap();
    assert!(guard.create_private(id).is_err());
    assert!(std::fs::read(store.key_paths(id).0).unwrap() == b"existing sentinel");
}

#[test]
fn two_services_serialize_owned_file_changes() {
    let home = tempfile::tempdir().unwrap();
    let first = KeyStore::for_home(home.path()).unwrap();
    let second = KeyStore::for_home(home.path()).unwrap();
    let guard = first.lock().unwrap();
    let before = std::fs::read_dir(home.path().join(".ssh/manyhands"))
        .unwrap()
        .count();
    assert_eq!(
        second.lock().err().unwrap().kind,
        KeyMaterialErrorKind::Busy
    );
    assert_eq!(
        std::fs::read_dir(home.path().join(".ssh/manyhands"))
            .unwrap()
            .count(),
        before
    );
    drop(guard);
    assert!(second.lock().is_ok());
}

#[cfg(unix)]
mod unix {
    use super::*;
    use std::os::unix::fs::{PermissionsExt, symlink};

    #[test]
    fn restrictive_umask_creates_exact_owned_modes() {
        if std::env::var_os("MANYHANDS_STORAGE_UMASK_CHILD").is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "repository::keys::storage::tests::unix::restrictive_umask_creates_exact_owned_modes", "--test-threads=1"])
                .env("MANYHANDS_STORAGE_UMASK_CHILD", "1")
                .output().unwrap();
            assert!(
                output.status.success(),
                "isolated umask child failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        // This filtered child runs this one test and exits, so its process-wide
        // umask cannot race any test or alter the parent runner's environment.
        unsafe {
            libc::umask(0o077);
        }
        let home = tempfile::tempdir().unwrap();
        let store = KeyStore::for_home(home.path()).unwrap();
        let guard = store.lock().unwrap();
        let id = SharedKeyId::new();
        let mut private = guard.create_private(id).unwrap();
        let _public = guard.create_public(id).unwrap();
        private
            .write_all_and_sync(b"temporary test material")
            .unwrap();
        for (path, mode) in [
            (home.path().join(".ssh"), 0o700),
            (home.path().join(".ssh/manyhands"), 0o700),
            (store.key_paths(id).0, 0o600),
            (store.key_paths(id).1, 0o644),
        ] {
            assert_eq!(
                std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
                mode
            );
        }
    }

    #[test]
    fn refuses_unsafe_existing_store() {
        for (entry, mode) in [(".ssh", 0o777), (".ssh/manyhands", 0o755)] {
            let home = tempfile::tempdir().unwrap();
            let store = KeyStore::for_home(home.path()).unwrap();
            drop(store.lock().unwrap());
            std::fs::set_permissions(
                home.path().join(entry),
                std::fs::Permissions::from_mode(mode),
            )
            .unwrap();
            assert_eq!(
                store.lock().err().unwrap().kind,
                KeyMaterialErrorKind::ProtectionUnavailable
            );
        }
    }

    #[test]
    fn refuses_symlink_or_reparse_substitution() {
        let home = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        symlink(other.path(), home.path().join(".ssh")).unwrap();
        assert!(KeyStore::for_home(home.path()).unwrap().lock().is_err());
        assert_eq!(std::fs::read_dir(other.path()).unwrap().count(), 0);
        std::fs::remove_file(home.path().join(".ssh")).unwrap();
        let store = KeyStore::for_home(home.path()).unwrap();
        let guard = store.lock().unwrap();
        let id = SharedKeyId::new();
        let target = other.path().join("sentinel");
        std::fs::write(&target, b"untouched sentinel").unwrap();
        symlink(&target, store.key_paths(id).0).unwrap();
        assert!(guard.create_private(id).is_err());
        assert!(guard.open_owned(id, KeyFileKind::Private).is_err());
        assert!(std::fs::read(&target).unwrap() == b"untouched sentinel");
    }

    #[test]
    fn refuses_private_hardlinks() {
        let home = tempfile::tempdir().unwrap();
        let store = KeyStore::for_home(home.path()).unwrap();
        let guard = store.lock().unwrap();
        let id = SharedKeyId::new();
        let mut file = guard.create_private(id).unwrap();
        std::fs::hard_link(store.key_paths(id).0, home.path().join("alias")).unwrap();
        assert_eq!(
            guard
                .open_owned(id, KeyFileKind::Private)
                .err()
                .unwrap()
                .kind,
            KeyMaterialErrorKind::ProtectionUnavailable
        );
        assert!(file.write_all_and_sync(b"must not write").is_err());
        assert!(std::fs::read(home.path().join("alias")).unwrap().is_empty());
    }

    #[test]
    fn refuses_protection_changed_after_open_and_preserves_replacement_on_remove() {
        let home = tempfile::tempdir().unwrap();
        let store = KeyStore::for_home(home.path()).unwrap();
        let guard = store.lock().unwrap();
        let id = SharedKeyId::new();
        let mut file = guard.create_private(id).unwrap();
        let path = store.key_paths(id).0;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(file.write_all_and_sync(b"must not write").is_err());
        std::fs::rename(&path, home.path().join("old")).unwrap();
        std::fs::write(&path, b"replacement sentinel").unwrap();
        assert!(file.remove().is_err());
        assert!(std::fs::read(path).unwrap() == b"replacement sentinel");
    }

    #[test]
    fn imported_observation_follows_symlinks_but_rejects_nonregular_without_blocking() {
        let home = tempfile::tempdir().unwrap();
        let target = home.path().join("imported");
        std::fs::write(&target, b"unparsed material").unwrap();
        let link = home.path().join("link");
        symlink(&target, &link).unwrap();
        assert_eq!(
            observe_regular_source(&target).unwrap(),
            observe_regular_source(&link).unwrap()
        );
        assert_eq!(
            observe_regular_source(home.path()).err().unwrap().kind,
            KeyMaterialErrorKind::NotRegularFile
        );
        let fifo = home.path().join("fifo");
        use std::os::unix::ffi::OsStrExt;
        let cpath = std::ffi::CString::new(fifo.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(cpath.as_ptr(), 0o600) }, 0);
        assert_eq!(
            observe_regular_source(&fifo).err().unwrap().kind,
            KeyMaterialErrorKind::NotRegularFile
        );
        assert!(!home.path().join(".ssh").exists());
    }
}

#[test]
fn rejects_unknown_or_malformed_identity_encodings() {
    for value in [
        "",
        "unix-v2:0",
        "unix-v1",
        "windows-v1:00",
        "unix-v1:ffffffffffffffff:ffffffffffffffff:ffffffffffffffff:ffffffffffffffff:ffffffffffffffff:ffffffffffffffff:ffffffffffffffff:ffffffffffffffff:extra",
    ] {
        assert!(FileIdentity::decode(value).is_err());
    }
}

#[cfg(windows)]
#[path = "windows_tests.rs"]
mod windows;
