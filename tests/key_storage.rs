use manyhands::repository::keys::{KeyMaterialErrorKind, KeyStore};

#[test]
fn store_configuration_does_not_create_ssh_directory() {
    let home = tempfile::tempdir().unwrap();
    assert!(KeyStore::for_home(home.path()).is_ok());
    assert!(!home.path().join(".ssh").exists());
}

#[test]
fn relative_home_is_rejected() {
    assert_eq!(
        KeyStore::for_home(std::path::Path::new("relative"))
            .err()
            .unwrap()
            .kind,
        KeyMaterialErrorKind::HomeUnavailable
    );
}
