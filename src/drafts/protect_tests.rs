use super::protect::*;
use std::io::{Read, Write};

#[cfg(unix)]
const OURS: u32 = 1000;
#[cfg(unix)]
const THEIRS: u32 = 1001;
/// `S_IFDIR`, the type bits `st_mode` carries for a directory.
#[cfg(unix)]
const DIRECTORY: u32 = 0o040_000;

#[cfg(unix)]
fn not_root() {
    assert!(
        unsafe { libc::geteuid() } != 0,
        "permission tests cannot run as root"
    );
}

#[cfg(unix)]
#[test]
fn the_data_directory_may_be_group_writable_but_not_other_writable() {
    for mode in [0o700, 0o755, 0o775, DIRECTORY | 0o775] {
        assert!(
            matches!(data_verdict(mode, OURS, OURS), Verdict::Ok),
            "{mode:o} of ours is accepted"
        );
    }
    assert!(matches!(data_verdict(0o777, OURS, OURS), Verdict::Refuse));
    assert!(matches!(data_verdict(0o702, OURS, OURS), Verdict::Refuse));
    assert!(matches!(data_verdict(0o700, THEIRS, OURS), Verdict::Refuse));
}

#[cfg(unix)]
#[test]
fn the_drafts_folder_is_ours_and_owner_only() {
    assert!(matches!(
        folder_verdict(0o700, OURS, OURS, true),
        Verdict::Ok
    ));
    assert!(matches!(
        folder_verdict(DIRECTORY | 0o700, OURS, OURS, true),
        Verdict::Ok
    ));
    assert!(matches!(
        folder_verdict(0o500, OURS, OURS, true),
        Verdict::Ok
    ));
    assert!(matches!(
        folder_verdict(0o755, OURS, OURS, true),
        Verdict::Tighten
    ));
    assert!(matches!(
        folder_verdict(0o700, THEIRS, OURS, true),
        Verdict::Refuse
    ));
    assert!(matches!(
        folder_verdict(0o700, OURS, OURS, false),
        Verdict::Refuse
    ));
}

#[test]
fn every_error_kind_has_its_own_fixed_sentence() {
    let kinds = [
        DraftErrorKind::Unavailable,
        DraftErrorKind::ProtectionUnavailable,
        DraftErrorKind::Access,
        DraftErrorKind::NoSpace,
        DraftErrorKind::OpenElsewhere,
        DraftErrorKind::HeldHere,
        DraftErrorKind::NotFound,
        DraftErrorKind::Io,
    ];
    let sentences: Vec<String> = kinds
        .iter()
        .map(|&kind| DraftError { kind }.to_string())
        .collect();
    for sentence in &sentences {
        assert!(sentence.ends_with('.'), "{sentence}");
        assert!(!sentence.contains('/') && !sentence.contains('\\'));
    }
    let mut unique = sentences.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), kinds.len());
}

#[test]
fn io_errors_map_to_kinds_without_their_message() {
    use std::io::{Error, ErrorKind};
    let cases = [
        (ErrorKind::PermissionDenied, DraftErrorKind::Access),
        (ErrorKind::StorageFull, DraftErrorKind::NoSpace),
        (ErrorKind::QuotaExceeded, DraftErrorKind::NoSpace),
        (ErrorKind::NotFound, DraftErrorKind::NotFound),
        (ErrorKind::AlreadyExists, DraftErrorKind::Io),
        (ErrorKind::Other, DraftErrorKind::Io),
    ];
    for (io, kind) in cases {
        let error = DraftError::from(Error::new(io, "/secret/path"));
        assert_eq!(error.kind, kind);
        assert!(!error.to_string().contains("secret"));
    }
}

#[test]
fn files_are_made_renamed_listed_read_and_removed_inside_the_folder() {
    let data = tempfile::tempdir().unwrap();
    let mut folder = Folder::open(data.path()).unwrap();
    let mut file = folder.create_new("a.tmp").unwrap();
    file.write_all(b"text").unwrap();
    file.sync_all().unwrap();
    drop(file);
    assert!(folder.create_new("a.tmp").is_err(), "creation is exclusive");
    folder.rename("a.tmp", "a.json").unwrap();
    folder.sync().unwrap();
    assert_eq!(folder.names().unwrap(), vec!["a.json".to_owned()]);
    assert_eq!(
        folder.names().unwrap(),
        vec!["a.json".to_owned()],
        "a second listing starts from the beginning"
    );
    let mut text = String::new();
    match folder.open_file("a.json") {
        Ok(Some(mut file)) => file.read_to_string(&mut text).unwrap(),
        Ok(None) | Err(_) => panic!("the draft opens"),
    };
    assert_eq!(text, "text");
    folder.remove("a.json").unwrap();
    assert!(folder.names().unwrap().is_empty());
    assert_eq!(
        folder.remove("a.json").map_err(|e| e.kind),
        Err(DraftErrorKind::NotFound)
    );
}

#[cfg(unix)]
#[test]
fn new_files_are_owner_only_whatever_the_umask() {
    use std::os::unix::fs::PermissionsExt;
    not_root();
    let data = tempfile::tempdir().unwrap();
    let mut folder = Folder::open(data.path()).unwrap();
    let file = folder.create_new("a.tmp").unwrap();
    assert_eq!(
        file.metadata().unwrap().permissions().mode() & 0o7777,
        0o600
    );
}

#[cfg(unix)]
#[test]
fn links_folders_and_fifos_are_not_draft_files() {
    use super::UnreadableReason;
    not_root();
    let data = tempfile::tempdir().unwrap();
    let folder = Folder::open(data.path()).unwrap();
    let drafts = data.path().join("drafts");
    std::fs::write(data.path().join("elsewhere"), b"x").unwrap();
    std::os::unix::fs::symlink(data.path().join("elsewhere"), drafts.join("link.json")).unwrap();
    std::fs::create_dir(drafts.join("folder.json")).unwrap();
    let fifo = std::ffi::CString::new(drafts.join("fifo.json").to_str().unwrap()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    for name in ["link.json", "folder.json", "fifo.json"] {
        match folder.open_file(name) {
            Ok(_) => panic!("{name} must not open"),
            Err(reason) => assert_eq!(reason, UnreadableReason::NotAFile, "{name}"),
        }
    }
}

#[test]
fn the_default_data_directory_is_absolute_or_unavailable() {
    match default_data_directory() {
        Ok(path) => assert!(path.is_absolute()),
        Err(error) => assert_eq!(error.kind, DraftErrorKind::Unavailable),
    }
}

#[test]
fn names_cannot_leave_the_folder() {
    let data = tempfile::tempdir().unwrap();
    let mut folder = Folder::open(data.path()).unwrap();
    drop(folder.create_new("a").unwrap());
    let outside = data.path().join("x");
    for name in ["../x", "sub/x", "..\\x", "", ".", "..", "a\0b"] {
        assert_eq!(
            folder.create_new(name).map(drop).map_err(|e| e.kind),
            Err(DraftErrorKind::Io),
            "{name:?}"
        );
        assert!(matches!(
            folder.open_file(name),
            Err(super::UnreadableReason::NotAFile)
        ));
        assert_eq!(
            folder.remove(name).map_err(|e| e.kind),
            Err(DraftErrorKind::Io)
        );
    }
    let absolute = outside.to_str().unwrap().to_owned();
    assert!(folder.create_new(&absolute).is_err());
    assert_eq!(
        folder.rename("a", "../b").map_err(|e| e.kind),
        Err(DraftErrorKind::Io)
    );
    assert_eq!(
        folder.rename("../x", "b").map_err(|e| e.kind),
        Err(DraftErrorKind::Io)
    );
    assert!(!outside.exists() && !data.path().join("b").exists());
    assert_eq!(folder.names().unwrap(), vec!["a".to_owned()]);
}

#[test]
fn a_name_that_is_gone_is_not_an_unreadable_file() {
    let data = tempfile::tempdir().unwrap();
    let mut folder = Folder::open(data.path()).unwrap();
    drop(folder.create_new("a.json").unwrap());
    folder.remove("a.json").unwrap();
    assert!(matches!(folder.open_file("a.json"), Ok(None)));
}

#[cfg(unix)]
#[test]
fn a_file_that_cannot_be_opened_is_access_not_gone() {
    use std::os::unix::fs::PermissionsExt;
    not_root();
    let data = tempfile::tempdir().unwrap();
    let mut folder = Folder::open(data.path()).unwrap();
    drop(folder.create_new("a.json").unwrap());
    let path = data.path().join("drafts/a.json");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
    let opened = folder.open_file("a.json");
    assert!(matches!(opened, Err(super::UnreadableReason::Access)));
}

#[test]
fn a_lock_file_is_made_once_and_opened_again() {
    let data = tempfile::tempdir().unwrap();
    let mut folder = Folder::open(data.path()).unwrap();
    let first = folder.open_lock("a.lock").unwrap();
    let second = folder.open_lock("a.lock").unwrap();
    assert!(first.metadata().unwrap().is_file() && second.metadata().unwrap().is_file());
    assert_eq!(folder.names().unwrap(), vec!["a.lock".to_owned()]);
    assert_eq!(
        folder.open_lock("../a.lock").map(drop).map_err(|e| e.kind),
        Err(DraftErrorKind::Io)
    );
}

#[cfg(unix)]
#[test]
fn a_lock_file_is_owner_only_and_never_a_link() {
    use std::os::unix::fs::PermissionsExt;
    not_root();
    let data = tempfile::tempdir().unwrap();
    let mut folder = Folder::open(data.path()).unwrap();
    let lock = folder.open_lock("a.lock").unwrap();
    assert_eq!(lock.metadata().unwrap().permissions().mode() & 0o077, 0);
    std::fs::write(data.path().join("elsewhere"), b"").unwrap();
    std::os::unix::fs::symlink(
        data.path().join("elsewhere"),
        data.path().join("drafts/b.lock"),
    )
    .unwrap();
    assert_eq!(
        folder.open_lock("b.lock").map(drop).map_err(|e| e.kind),
        Err(DraftErrorKind::ProtectionUnavailable)
    );
    std::fs::create_dir(data.path().join("drafts/c.lock")).unwrap();
    assert!(folder.open_lock("c.lock").is_err());
}
