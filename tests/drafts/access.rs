//! The drafts folder's owner protection on Unix (decision 5).

use crate::support::*;
use manyhands::drafts::{
    DraftErrorKind, DraftId, DraftStore, Entry, UnreadableReason, folder::Folder,
};
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

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/drafts")
            .join(name),
    )
    .expect("fixture")
}

fn reason_of(store: &DraftStore, id: DraftId) -> UnreadableReason {
    match listing(store, id).entry {
        Entry::Unreadable(reason) => reason,
        Entry::Draft(_) => panic!("{id} must be unreadable"),
    }
}

#[test]
fn an_unreadable_draft_file_is_listed_and_left_in_place() {
    not_root();
    let root = tempfile::tempdir().unwrap();
    let data = data_at(&root, 0o700);
    let store = store(&data);
    let id = create(&store).draft().id;
    let file = drafts(&data).join(format!("{id}.json"));
    let bytes = std::fs::read(&file).unwrap();
    set_mode(&file, 0o000);
    let reason = reason_of(&store, id);
    let claimed = store.claim(id).err().map(|e| e.kind);
    let exported = store.raw_bytes(id).map_err(|e| e.kind);
    set_mode(&file, 0o600);
    assert_eq!(reason, UnreadableReason::Access);
    assert_eq!(claimed, Some(DraftErrorKind::Access));
    assert_eq!(exported, Err(DraftErrorKind::Access));
    assert_eq!(std::fs::read(&file).unwrap(), bytes);
}

#[test]
fn a_draft_name_that_is_a_folder_or_a_link_is_not_a_file() {
    not_root();
    let root = tempfile::tempdir().unwrap();
    let data = data_at(&root, 0o700);
    let store = store(&data);
    let folder = DraftId::generate();
    std::fs::create_dir(drafts(&data).join(format!("{folder}.json"))).unwrap();
    let link = DraftId::generate();
    let target = root.path().join("elsewhere.json");
    std::fs::write(&target, fixture("v1-document.json")).unwrap();
    symlink(&target, drafts(&data).join(format!("{link}.json"))).unwrap();
    assert_eq!(reason_of(&store, folder), UnreadableReason::NotAFile);
    assert_eq!(reason_of(&store, link), UnreadableReason::NotAFile);
    assert!(drafts(&data).join(format!("{folder}.json")).is_dir());
    assert!(
        std::fs::symlink_metadata(drafts(&data).join(format!("{link}.json")))
            .unwrap()
            .is_symlink()
    );
}

#[test]
fn a_read_only_drafts_folder_fails_the_write_and_keeps_the_previous_revision() {
    not_root();
    let root = tempfile::tempdir().unwrap();
    let data = data_at(&root, 0o700);
    let store = store(&data);
    let mut claim = create(&store);
    let id = claim.draft().id;
    set_mode(&drafts(&data), 0o500);
    let written = write(&mut claim, 2).map_err(|e| e.kind);
    let reopened = DraftStore::open_in(&data).is_ok();
    let after = mode(&drafts(&data));
    set_mode(&drafts(&data), 0o700);
    assert_eq!(written, Err(DraftErrorKind::Access));
    assert!(reopened);
    assert_eq!(after, 0o500, "reopening adds no permission");
    assert_eq!(claim.draft().revision, 1);
    drop(claim);
    assert_eq!(listed_revision(&store, id), 1);
}

#[test]
fn damaged_and_newer_files_survive_the_store_byte_for_byte() {
    not_root();
    let root = tempfile::tempdir().unwrap();
    let data = data_at(&root, 0o700);
    let store = store(&data);
    let other = create(&store).draft().id;
    let newer = DraftId::generate();
    let damaged = DraftId::generate();
    let truncated = DraftId::generate();
    let ticket = fixture("v1-ticket.json");
    let files = [
        (newer, fixture("v2-ticket.json")),
        (damaged, b"not json".to_vec()),
        (truncated, ticket[..40].to_vec()),
    ];
    for (id, bytes) in &files {
        std::fs::write(drafts(&data).join(format!("{id}.json")), bytes).unwrap();
    }
    assert_eq!(
        reason_of(&store, newer),
        UnreadableReason::NewerFormat { version: 2 }
    );
    assert_eq!(reason_of(&store, damaged), UnreadableReason::Damaged);
    assert_eq!(reason_of(&store, truncated), UnreadableReason::Damaged);
    drop(claim(&store, other));
    drop(create(&store));
    assert_eq!(claim_error(&store, newer), DraftErrorKind::Io);
    for (id, bytes) in &files {
        assert_eq!(
            &std::fs::read(drafts(&data).join(format!("{id}.json"))).unwrap(),
            bytes
        );
        assert_eq!(store.raw_bytes(*id).as_ref().map_err(|e| e.kind), Ok(bytes));
    }

    assert_eq!(store.discard_unreadable(damaged), Ok(()));
    assert!(!drafts(&data).join(format!("{damaged}.json")).exists());
    assert!(!drafts(&data).join(format!("{damaged}.lock")).exists());
    assert_eq!(
        store.raw_bytes(damaged).map_err(|e| e.kind),
        Err(DraftErrorKind::NotFound)
    );
    for (id, bytes) in files.iter().filter(|(id, _)| *id != damaged) {
        assert_eq!(
            &std::fs::read(drafts(&data).join(format!("{id}.json"))).unwrap(),
            bytes
        );
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
