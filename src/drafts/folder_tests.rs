use super::{DraftErrorKind, folder::Folder};
use std::io::Write;

#[test]
fn a_name_that_is_not_one_component_is_refused() {
    let data = tempfile::tempdir().unwrap();
    for name in ["", ".", "..", "a/b", "/a", "a\\b", "a\0b", "../a"] {
        assert_eq!(
            Folder::open_named(data.path(), name).err().map(|e| e.kind),
            Some(DraftErrorKind::Io),
            "folder name {name:?} is refused"
        );
    }
    assert!(
        !data.path().join("a").exists(),
        "a refused folder name makes nothing"
    );
    let mut folder = Folder::open_named(data.path(), "resolutions").unwrap();
    for name in ["", ".", "..", "a/b", "/a", "a\\b", "a\0b", "../a"] {
        let refused = |result: Result<(), super::DraftError>| result.err().map(|e| e.kind);
        assert_eq!(
            refused(folder.create_new(name).map(drop)),
            Some(DraftErrorKind::Io),
            "{name:?}"
        );
        assert_eq!(
            refused(folder.replace(name, b"x")),
            Some(DraftErrorKind::Io),
            "{name:?}"
        );
        assert_eq!(
            refused(folder.read(name).map(drop)),
            Some(DraftErrorKind::Io),
            "{name:?}"
        );
        assert_eq!(
            refused(folder.remove(name)),
            Some(DraftErrorKind::Io),
            "{name:?}"
        );
    }
    assert!(folder.list().unwrap().is_empty());
}

#[cfg(unix)]
#[test]
fn files_are_owner_only() {
    use std::os::unix::fs::PermissionsExt;
    assert!(
        unsafe { libc::geteuid() } != 0,
        "permission tests cannot run as root"
    );
    let mode =
        |path: &std::path::Path| std::fs::metadata(path).unwrap().permissions().mode() & 0o7777;
    let data = tempfile::tempdir().unwrap();
    let mut folder = Folder::open_named(data.path(), "resolutions").unwrap();
    let folder_path = data.path().join("resolutions");
    assert_eq!(mode(&folder_path), 0o700);
    folder.replace("a", b"one").unwrap();
    assert_eq!(mode(&folder_path.join("a")), 0o600);
    folder.replace("a", b"two").unwrap();
    assert_eq!(mode(&folder_path.join("a")), 0o600);
    drop(folder.create_new("b").unwrap());
    assert_eq!(mode(&folder_path.join("b")), 0o600);

    std::fs::set_permissions(&folder_path, std::fs::Permissions::from_mode(0o755)).unwrap();
    Folder::open_named(data.path(), "resolutions").unwrap();
    assert_eq!(mode(&folder_path), 0o700, "a loose folder is tightened");
}

#[test]
fn replace_swaps_whole_files() {
    let data = tempfile::tempdir().unwrap();
    let mut folder = Folder::open_named(data.path(), "resolutions").unwrap();
    assert_eq!(
        folder.read("a").err().map(|e| e.kind),
        Some(DraftErrorKind::NotFound)
    );
    folder.replace("a", b"a long first revision").unwrap();
    assert_eq!(folder.read("a").unwrap(), b"a long first revision");
    folder.replace("a", b"short").unwrap();
    assert_eq!(
        folder.read("a").unwrap(),
        b"short",
        "a shorter revision leaves no tail of the longer one"
    );
    let on_disk: Vec<String> = std::fs::read_dir(data.path().join("resolutions"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(on_disk, vec!["a".to_owned()], "no temporary file remains");
}

#[test]
fn list_returns_names_in_order() {
    let data = tempfile::tempdir().unwrap();
    let mut folder = Folder::open_named(data.path(), "resolutions").unwrap();
    for name in ["c", "a", "b"] {
        folder.replace(name, name.as_bytes()).unwrap();
    }
    let path = data.path().join("resolutions");
    std::fs::write(path.join(".a.0123456789abcdef.tmp"), b"x").unwrap();
    std::fs::write(path.join("a.lock"), b"").unwrap();
    assert_eq!(
        folder.list().unwrap(),
        vec!["a".to_owned(), "b".to_owned(), "c".to_owned()],
        "sorted, without temporaries or lock files"
    );
}

#[test]
fn remove_deletes_one_file() {
    let data = tempfile::tempdir().unwrap();
    let mut folder = Folder::open_named(data.path(), "resolutions").unwrap();
    folder.replace("a", b"one").unwrap();
    folder.replace("b", b"two").unwrap();
    folder.remove("a").unwrap();
    assert_eq!(folder.list().unwrap(), vec!["b".to_owned()]);
    assert_eq!(folder.read("b").unwrap(), b"two");
    assert_eq!(
        folder.remove("a").err().map(|e| e.kind),
        Some(DraftErrorKind::NotFound)
    );
}

#[test]
fn the_drafts_folder_is_unchanged() {
    let data = tempfile::tempdir().unwrap();
    let mut folder = Folder::open(data.path()).unwrap();
    let mut file = folder.create_new("a.tmp").unwrap();
    file.write_all(b"x").unwrap();
    drop(file);
    assert!(data.path().join("drafts/a.tmp").is_file());
    let mut named = Folder::open_named(data.path(), "resolutions").unwrap();
    named.replace("b", b"y").unwrap();
    assert!(data.path().join("resolutions/b").is_file());
    assert!(!data.path().join("drafts/b").exists());
    assert!(!data.path().join("resolutions/a.tmp").exists());
}
