//! The drafts folder's owner protection on Unix (decision 5).

use manyhands::drafts::{DraftErrorKind, folder::Folder};
use std::{
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
};

fn not_root() {
    assert!(
        unsafe { libc::geteuid() } != 0,
        "permission tests cannot run as root"
    );
}

fn mode(path: &Path) -> u32 {
    std::fs::symlink_metadata(path)
        .unwrap()
        .permissions()
        .mode()
        & 0o7777
}

fn set_mode(path: &Path, mode: u32) {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
}

/// A data directory at `mode` inside a fresh temporary directory.
fn data_at(root: &tempfile::TempDir, mode: u32) -> PathBuf {
    let data = root.path().join("data");
    std::fs::create_dir(&data).unwrap();
    set_mode(&data, mode);
    data
}

fn refused(data: &Path) -> DraftErrorKind {
    match Folder::open(data) {
        Ok(_) => panic!("the folder must be refused"),
        Err(error) => error.kind,
    }
}

#[test]
fn a_fresh_data_directory_gets_an_owner_only_drafts_folder() {
    not_root();
    let root = tempfile::tempdir().unwrap();
    let data = data_at(&root, 0o700);
    Folder::open(&data).unwrap();
    let drafts = data.join("drafts");
    assert!(std::fs::symlink_metadata(&drafts).unwrap().is_dir());
    assert_eq!(mode(&drafts), 0o700);
}

#[test]
fn a_group_writable_data_directory_is_accepted() {
    not_root();
    let root = tempfile::tempdir().unwrap();
    let data = data_at(&root, 0o775);
    Folder::open(&data).unwrap();
    assert_eq!(mode(&data.join("drafts")), 0o700);
}

#[test]
fn an_other_writable_data_directory_is_refused() {
    not_root();
    let root = tempfile::tempdir().unwrap();
    let data = data_at(&root, 0o777);
    assert_eq!(refused(&data), DraftErrorKind::ProtectionUnavailable);
    assert!(!data.join("drafts").exists(), "nothing is made in it");
}

#[test]
fn a_linked_data_directory_is_refused() {
    not_root();
    let root = tempfile::tempdir().unwrap();
    let real = data_at(&root, 0o700);
    let link = root.path().join("link");
    symlink(&real, &link).unwrap();
    assert_eq!(refused(&link), DraftErrorKind::ProtectionUnavailable);
}

#[test]
fn an_open_drafts_folder_is_tightened_to_owner_only() {
    not_root();
    let root = tempfile::tempdir().unwrap();
    let data = data_at(&root, 0o700);
    let drafts = data.join("drafts");
    std::fs::create_dir(&drafts).unwrap();
    set_mode(&drafts, 0o755);
    Folder::open(&data).unwrap();
    assert_eq!(mode(&drafts), 0o700);
}

#[test]
fn tightening_never_adds_a_permission() {
    not_root();
    let root = tempfile::tempdir().unwrap();
    let data = data_at(&root, 0o700);
    let drafts = data.join("drafts");
    std::fs::create_dir(&drafts).unwrap();
    set_mode(&drafts, 0o500);
    let opened = Folder::open(&data).is_ok();
    let after = mode(&drafts);
    set_mode(&drafts, 0o700);
    assert!(opened);
    assert_eq!(after, 0o500);
}

#[test]
fn a_linked_drafts_folder_is_refused() {
    not_root();
    let root = tempfile::tempdir().unwrap();
    let data = data_at(&root, 0o700);
    let elsewhere = root.path().join("elsewhere");
    std::fs::create_dir(&elsewhere).unwrap();
    set_mode(&elsewhere, 0o700);
    symlink(&elsewhere, data.join("drafts")).unwrap();
    assert_eq!(refused(&data), DraftErrorKind::ProtectionUnavailable);
}

#[test]
fn a_drafts_file_is_refused() {
    not_root();
    let root = tempfile::tempdir().unwrap();
    let data = data_at(&root, 0o700);
    std::fs::write(data.join("drafts"), b"").unwrap();
    assert_eq!(refused(&data), DraftErrorKind::ProtectionUnavailable);
}

#[test]
fn a_swapped_drafts_folder_cannot_redirect_a_write() {
    not_root();
    let root = tempfile::tempdir().unwrap();
    let data = data_at(&root, 0o700);
    let mut folder = Folder::open(&data).unwrap();
    std::fs::rename(data.join("drafts"), data.join("moved")).unwrap();
    std::fs::create_dir(data.join("drafts")).unwrap();
    folder.create_new("a.tmp").unwrap();
    assert!(
        data.join("moved/a.tmp").exists(),
        "the write stays in the held folder"
    );
    assert!(!data.join("drafts/a.tmp").exists());
}

#[test]
fn a_removed_drafts_folder_is_made_again_once() {
    not_root();
    let root = tempfile::tempdir().unwrap();
    let data = data_at(&root, 0o700);
    let mut folder = Folder::open(&data).unwrap();
    std::fs::remove_dir(data.join("drafts")).unwrap();
    folder.create_new("a.tmp").unwrap();
    assert!(data.join("drafts/a.tmp").exists());
    assert_eq!(mode(&data.join("drafts")), 0o700);
}
