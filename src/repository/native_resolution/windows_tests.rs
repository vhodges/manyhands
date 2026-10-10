use super::*;

fn root() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

fn stock_style_reader(path: &Path) -> File {
    use std::os::windows::fs::OpenOptionsExt;

    std::fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .open(path)
        .unwrap()
}

#[test]
fn existing_stock_style_reader_permits_owned_image_reads() {
    let root = root();
    std::fs::write(root.path().join("image"), b"reader-compatible").unwrap();
    let reader = stock_style_reader(&root.path().join("image"));
    let directory = Directory::open(root.path()).unwrap();
    let image = directory.image("image").unwrap().unwrap();
    assert_eq!(image.bytes, b"reader-compatible");
    assert_eq!(
        read_owned(root.path(), Path::new("image")).unwrap(),
        Some(b"reader-compatible".to_vec())
    );
    drop(reader);
}

#[test]
fn retained_owned_image_permits_stock_style_reader() {
    let root = root();
    std::fs::write(root.path().join("image"), b"reader-compatible").unwrap();
    let directory = Directory::open(root.path()).unwrap();
    let image = directory.image("image").unwrap().unwrap();
    let mut reader = stock_style_reader(&root.path().join("image"));
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, image.bytes);
    directory.matches("image", &image).unwrap();
}

#[test]
fn retained_created_image_permits_stock_style_reader() {
    let root = root();
    let directory = Directory::open(root.path()).unwrap();
    let image = directory
        .create_image("created", b"reader-compatible")
        .unwrap();
    let mut reader = stock_style_reader(&root.path().join("created"));
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, image.bytes);
    directory.matches("created", &image).unwrap();
}

#[test]
fn retained_ancestors_prevent_directory_rename() {
    let root = root();
    std::fs::create_dir(root.path().join("parent")).unwrap();
    std::fs::create_dir(root.path().join("parent/child")).unwrap();
    let held = Directory::open(&root.path().join("parent/child")).unwrap();
    assert!(std::fs::rename(root.path().join("parent"), root.path().join("renamed")).is_err());
    held.revalidate().unwrap();
    drop(held);
    std::fs::rename(root.path().join("parent"), root.path().join("renamed")).unwrap();
}

#[test]
fn reads_exact_bytes_and_observes_absence_without_creating_files() {
    let root = root();
    let bytes = b"caller\r\nbytes\0\xff";
    std::fs::write(root.path().join("image"), bytes).unwrap();
    assert_eq!(
        read_owned(root.path(), Path::new("image")).unwrap(),
        Some(bytes.to_vec())
    );
    assert_eq!(
        read_owned(root.path(), Path::new("absent/image")).unwrap(),
        None
    );
    assert!(!root.path().join("absent").exists());
    std::fs::create_dir(root.path().join("directory")).unwrap();
    assert!(read_owned(root.path(), Path::new("directory")).is_err());
    assert!(read_owned(root.path(), Path::new("../image")).is_err());
    assert!(read_owned(root.path(), Path::new("NUL")).is_err());
    assert!(read_owned(root.path(), Path::new("image:stream")).is_err());
}

#[test]
fn junction_ancestors_and_leaves_are_refused() {
    let root = root();
    let outside = root.path().join("outside");
    let alias = root.path().join("alias");
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("image"), b"outside").unwrap();
    // Junction creation is unprivileged; unlike symlink tests this needs neither
    // developer mode nor SeCreateSymbolicLinkPrivilege on native Windows CI.
    assert!(
        std::process::Command::new("cmd.exe")
            .args(["/C", "mklink", "/J"])
            .arg(&alias)
            .arg(&outside)
            .status()
            .unwrap()
            .success()
    );
    assert!(Directory::open(&alias).is_err());
    assert!(read_owned(root.path(), Path::new("alias/image")).is_err());
    assert!(read_owned(root.path(), Path::new("alias")).is_err());
    assert_eq!(std::fs::read(outside.join("image")).unwrap(), b"outside");
    std::fs::remove_dir(alias).unwrap();
}

#[test]
fn hard_link_publication_is_absent_only_and_accepts_multiple_links() {
    let root = root();
    let directory = Directory::open(root.path()).unwrap();
    let image = directory
        .create_image("sentinel-anchor", b"sentinel")
        .unwrap();
    directory
        .publish_anchor("sentinel-anchor", &directory, "index.lock", &image)
        .unwrap();
    let published = directory.image("index.lock").unwrap().unwrap();
    assert_eq!(image.stamp.identity, published.stamp.identity);
    assert!(file_info(&published.file).unwrap().nNumberOfLinks >= 2);
    let refreshed = directory.image("sentinel-anchor").unwrap().unwrap();
    std::fs::write(root.path().join("foreign"), b"foreign").unwrap();
    assert_eq!(
        directory
            .publish_anchor("sentinel-anchor", &directory, "foreign", &refreshed)
            .unwrap_err()
            .kind(),
        io::ErrorKind::AlreadyExists
    );
    assert_eq!(
        std::fs::read(root.path().join("foreign")).unwrap(),
        b"foreign"
    );
    directory
        .publish_anchor("sentinel-anchor", &directory, "second-anchor", &refreshed)
        .unwrap();
    assert!(
        file_info(&directory.image("second-anchor").unwrap().unwrap().file)
            .unwrap()
            .nNumberOfLinks
            >= 3
    );
}

#[test]
fn verified_retirement_removes_only_the_matching_hard_link() {
    let root = root();
    let directory = Directory::open(root.path()).unwrap();
    let image = directory.create_image("anchor", b"sentinel").unwrap();
    directory
        .publish_anchor("anchor", &directory, "index.lock", &image)
        .unwrap();
    drop(image);
    let lock = directory.image("index.lock").unwrap().unwrap();
    directory.retire("index.lock", lock).unwrap();
    assert!(directory.image("index.lock").unwrap().is_none());
    assert_eq!(
        directory.image("anchor").unwrap().unwrap().bytes,
        b"sentinel"
    );
}

#[test]
fn retirement_with_anchor_image_deletes_target_link_and_preserves_anchor() {
    let root = root();
    let directory = Directory::open(root.path()).unwrap();
    let initial = directory.create_image("anchor", b"sentinel").unwrap();
    directory
        .publish_anchor("anchor", &directory, "index.lock", &initial)
        .unwrap();
    drop(initial);
    // Refresh after publication, then deliberately supply ownership proof opened
    // through the other hard link, not the pathname requested for retirement.
    let anchor = directory.image("anchor").unwrap().unwrap();
    let expected_identity = anchor.stamp.identity;
    directory.retire("index.lock", anchor).unwrap();
    assert!(directory.image("index.lock").unwrap().is_none());
    let retained = directory.image("anchor").unwrap().unwrap();
    assert_eq!(retained.stamp.identity, expected_identity);
    assert_eq!(retained.bytes, b"sentinel");
}

#[test]
fn verified_retirement_preserves_a_substituted_leaf() {
    let root = root();
    let directory = Directory::open(root.path()).unwrap();
    let image = directory.create_image("owned", b"owned").unwrap();
    std::fs::rename(root.path().join("owned"), root.path().join("retained")).unwrap();
    std::fs::write(root.path().join("owned"), b"foreign").unwrap();
    assert!(directory.retire("owned", image).is_err());
    assert_eq!(
        std::fs::read(root.path().join("owned")).unwrap(),
        b"foreign"
    );
    let owned = directory.image("owned").unwrap().unwrap();
    directory.retire("owned", owned).unwrap();
    assert!(directory.image("owned").unwrap().is_none());
}

#[test]
fn stable_image_detects_same_identity_content_changes() {
    let root = root();
    let directory = Directory::open(root.path()).unwrap();
    let image = directory.create_image("image", b"original").unwrap();
    std::fs::write(root.path().join("image"), b"modified").unwrap();
    assert!(directory.matches("image", &image).is_err());
}

#[test]
fn guarded_replacement_preserves_foreign_preimage_and_installs_exact_bytes() {
    let root = root();
    let git = root.path().join(".git");
    std::fs::create_dir(&git).unwrap();
    std::fs::create_dir(root.path().join("docs")).unwrap();
    let relative = Path::new("docs/document.md");
    std::fs::write(root.path().join(relative), b"original").unwrap();
    let bytes = b"resolved\r\n\0\xff";
    assert!(
        replace_owned(
            root.path(),
            relative,
            &git,
            bytes,
            super::super::owned_prewrite_digest(Some(b"foreign"))
        )
        .is_err()
    );
    assert_eq!(
        std::fs::read(root.path().join(relative)).unwrap(),
        b"original"
    );
    replace_owned(
        root.path(),
        relative,
        &git,
        bytes,
        super::super::owned_prewrite_digest(Some(b"original")),
    )
    .unwrap();
    assert_eq!(std::fs::read(root.path().join(relative)).unwrap(), bytes);
    replace_owned(
        root.path(),
        Path::new("docs/new.md"),
        &git,
        bytes,
        super::super::owned_prewrite_digest(None),
    )
    .unwrap();
    assert_eq!(
        std::fs::read(root.path().join("docs/new.md")).unwrap(),
        bytes
    );
}

#[test]
fn journaled_output_install_keeps_source_anchor_and_exact_installed_identity() {
    let root = root();
    let parent = Directory::open(root.path()).unwrap();
    let staging = parent.child("private", true).unwrap();
    let output = staging.create_image("index", b"serialized output").unwrap();
    parent.create_image("index", b"baseline").unwrap();
    staging
        .publish_anchor("index", &staging, "install", &output)
        .unwrap();
    let install = staging.image("install").unwrap().unwrap();
    let original = parent.image("index").unwrap().unwrap();
    staging
        .install_anchor("install", &parent, "index", &install, &original)
        .unwrap();
    assert!(staging.image("install").unwrap().is_none());
    let installed = parent.image("index").unwrap().unwrap();
    let anchored = staging.image("index").unwrap().unwrap();
    assert_eq!(installed.stamp.identity, output.stamp.identity);
    assert_eq!(installed.stamp.identity, anchored.stamp.identity);
    assert_eq!(installed.bytes, b"serialized output");
    assert_eq!(installed.bytes, anchored.bytes);
}

#[test]
fn install_renames_requested_link_when_proof_was_opened_through_anchor() {
    let root = root();
    let parent = Directory::open(root.path()).unwrap();
    let staging = parent.child("private", true).unwrap();
    let output = staging.create_image("anchor", b"caller\r\nbytes").unwrap();
    staging
        .publish_anchor("anchor", &staging, "install", &output)
        .unwrap();
    // Publication changes ChangeTime; refresh through the durable anchor, not
    // the disposable source role that the rename must remove.
    let prepared = staging.image("anchor").unwrap().unwrap();
    // Exercise a non-BMP UTF-16 name as well as an alternate proof anchor.
    let target = "target-\u{1f9f5}";
    let original = parent.create_image(target, b"old\nbytes").unwrap();
    staging
        .install_anchor("install", &parent, target, &prepared, &original)
        .unwrap();
    assert!(staging.image("install").unwrap().is_none());
    let anchor = staging.image("anchor").unwrap().unwrap();
    let installed = parent.image(target).unwrap().unwrap();
    assert!(anchor.bytes == prepared.bytes && installed.bytes == prepared.bytes);
    assert_eq!(anchor.stamp.identity, prepared.stamp.identity);
    assert_eq!(installed.stamp.identity, prepared.stamp.identity);
    let mut old_handle = original.file;
    old_handle.seek(SeekFrom::Start(0)).unwrap();
    let mut bytes = Vec::new();
    old_handle.read_to_end(&mut bytes).unwrap();
    assert!(bytes == b"old\nbytes");
}

#[test]
fn guarded_replacement_keeps_retained_target_handle_readable() {
    let root = root();
    let parent = Directory::open(root.path()).unwrap();
    let staging = parent.child("private", true).unwrap();
    let original = parent.create_image("target", b"original\r\n").unwrap();
    replace_owned(
        root.path(),
        Path::new("target"),
        staging.path(),
        b"resolved\n",
        super::super::owned_prewrite_digest(Some(b"original\r\n")),
    )
    .unwrap();
    let installed = parent.image("target").unwrap().unwrap();
    assert!(installed.bytes == b"resolved\n");
    assert_ne!(installed.stamp.identity, original.stamp.identity);
    let mut old_handle = original.file;
    old_handle.seek(SeekFrom::Start(0)).unwrap();
    let mut bytes = Vec::new();
    old_handle.read_to_end(&mut bytes).unwrap();
    assert!(bytes == b"original\r\n");
}

#[test]
fn readonly_target_replacement_is_an_honest_native_refusal() {
    let root = root();
    let parent = Directory::open(root.path()).unwrap();
    let staging = parent.child("private", true).unwrap();
    let output = staging.create_image("install", b"resolved").unwrap();
    parent.create_image("target", b"baseline").unwrap();
    let path = root.path().join("target");
    let mut permissions = std::fs::metadata(&path).unwrap().permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(&path, permissions).unwrap();
    let original = parent.image("target").unwrap().unwrap();
    let result = staging.install_anchor("install", &parent, "target", &output, &original);
    // Restore fixture permissions before assertions so an unexpected success
    // does not leave a read-only remnant that prevents TempDir cleanup.
    let mut permissions = std::fs::metadata(&path).unwrap().permissions();
    permissions.set_readonly(false);
    std::fs::set_permissions(&path, permissions).unwrap();
    assert_eq!(
        result.unwrap_err().raw_os_error(),
        Some(ERROR_ACCESS_DENIED as i32)
    );
    assert!(parent.image("target").unwrap().unwrap().bytes == b"baseline");
    staging.matches("install", &output).unwrap();
}

#[test]
fn install_refuses_byte_identical_source_substitution() {
    let root = root();
    let parent = Directory::open(root.path()).unwrap();
    let staging = parent.child("private", true).unwrap();
    let output = staging.create_image("install", b"output").unwrap();
    let original = parent.create_image("target", b"baseline").unwrap();
    std::fs::rename(staging.path().join("install"), staging.path().join("saved")).unwrap();
    let foreign = staging.create_image("install", b"output").unwrap();
    assert_ne!(output.stamp.identity, foreign.stamp.identity);
    assert!(
        staging
            .install_anchor("install", &parent, "target", &output, &original)
            .is_err()
    );
    parent.matches("target", &original).unwrap();
    staging.matches("install", &foreign).unwrap();
    assert!(staging.image("saved").unwrap().unwrap().bytes == b"output");
}

#[test]
fn absent_only_install_is_independent_and_never_overwrites() {
    let root = root();
    let parent = Directory::open(root.path()).unwrap();
    let staging = parent.child("private", true).unwrap();
    let first = staging.create_image("first", b"first").unwrap();
    staging
        .rename_image("first", &parent, "target", &first, None)
        .unwrap();
    assert!(staging.image("first").unwrap().is_none());
    let installed = parent.image("target").unwrap().unwrap();
    assert_eq!(installed.stamp.identity, first.stamp.identity);
    assert!(installed.bytes == b"first");
    let second = staging.create_image("second", b"second").unwrap();
    assert!(
        staging
            .rename_image("second", &parent, "target", &second, None)
            .is_err()
    );
    parent.matches("target", &installed).unwrap();
    staging.matches("second", &second).unwrap();
}

#[test]
fn journaled_output_install_preserves_substituted_original() {
    let root = root();
    let parent = Directory::open(root.path()).unwrap();
    let staging = parent.child("private", true).unwrap();
    let output = staging
        .create_image("install", b"serialized output")
        .unwrap();
    let original = parent.create_image("index", b"baseline").unwrap();
    std::fs::rename(root.path().join("index"), root.path().join("saved-index")).unwrap();
    std::fs::write(root.path().join("index"), b"foreign").unwrap();
    assert!(
        staging
            .install_anchor("install", &parent, "index", &output, &original)
            .is_err()
    );
    assert_eq!(parent.image("index").unwrap().unwrap().bytes, b"foreign");
    staging.matches("install", &output).unwrap();
}

#[test]
fn readonly_git_storage_has_a_best_effort_flush_without_functional_refusal() {
    let root = root();
    let parent = Directory::open(root.path()).unwrap();
    parent.create_image("object", b"immutable object").unwrap();
    let path = root.path().join("object");
    let mut permissions = std::fs::metadata(&path).unwrap().permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(&path, permissions).unwrap();
    parent.flush("object").unwrap();
    assert_eq!(
        parent.image("object").unwrap().unwrap().bytes,
        b"immutable object"
    );
    let mut permissions = std::fs::metadata(&path).unwrap().permissions();
    permissions.set_readonly(false);
    std::fs::set_permissions(&path, permissions).unwrap();
}
