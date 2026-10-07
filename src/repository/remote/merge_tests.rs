use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use git2::{Index, IndexEntry, IndexTime, Repository, Signature};

use super::{
    CommitIdentityBoundary, ConflictCandidate, ConflictEligibility, ConflictEntryKind,
    ConflictObservation, ConflictStructure, IntegrationClassificationError, IntegrationDisposition,
    IntegrationStage, RedactedConflictBytes, ResolveSynchronizationRequest, classify_integration,
    commit_identity_boundary, conflict_eligibility, integration_stages,
};
use crate::{
    canonical::ItemId,
    repository::{AuthoringKind, OperationId, SynchronizationTarget},
};

const ITEM_ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";

fn oid(node: u8) -> git2::Oid {
    git2::Oid::from_bytes(&[node; 20]).unwrap()
}

#[test]
fn graph_classification_distinguishes_ordered_merge_cases() {
    let ancestor = |older, newer| {
        Ok::<_, &'static str>(matches!(
            (older, newer),
            (a, b) if a == oid(1) && [oid(2), oid(3), oid(4)].contains(&b)
                || (a == oid(2) && b == oid(3))
        ))
    };
    for (local, incoming, base, expected) in [
        (2, 2, true, Ok(IntegrationDisposition::Equal)),
        (
            3,
            2,
            true,
            Ok(IntegrationDisposition::IncomingAlreadyIntegrated),
        ),
        (2, 3, true, Ok(IntegrationDisposition::FastForward)),
        (3, 4, true, Ok(IntegrationDisposition::MergeRequired)),
        (
            3,
            4,
            false,
            Err(IntegrationClassificationError::UnrelatedHistories),
        ),
    ] {
        assert_eq!(
            classify_integration(oid(local), oid(incoming), ancestor, |_, _| Ok(base)),
            expected
        );
    }
    assert_eq!(
        classify_integration(
            oid(2),
            oid(3),
            |_, _| Err("missing object"),
            |_, _| Ok(true)
        ),
        Err(IntegrationClassificationError::Ancestry("missing object"))
    );
}

#[test]
fn context_stages_are_ordered_before_primary_without_contextless_placeholder() {
    let context = SynchronizationTarget::Context {
        kind: AuthoringKind::Ticket,
        item_id: ITEM_ID.parse::<ItemId>().unwrap(),
    };
    assert_eq!(
        integration_stages(&context, true),
        vec![IntegrationStage::Context, IntegrationStage::Primary]
    );
    assert_eq!(
        integration_stages(&context, false),
        vec![IntegrationStage::Primary]
    );
    assert_eq!(
        integration_stages(&SynchronizationTarget::Primary, true),
        vec![IntegrationStage::Primary]
    );
}

#[test]
fn eligibility_requires_the_entire_conflict_set_to_be_canonical_regular_utf8() {
    let canonical = [
        ConflictCandidate {
            kind: ConflictEntryKind::Document,
            structure: ConflictStructure::RegularUtf8SamePath,
        },
        ConflictCandidate {
            kind: ConflictEntryKind::Ticket,
            structure: ConflictStructure::RegularUtf8SamePath,
        },
        ConflictCandidate {
            kind: ConflictEntryKind::Comment,
            structure: ConflictStructure::RegularUtf8SamePath,
        },
    ];
    assert_eq!(
        conflict_eligibility(&canonical),
        ConflictEligibility::EligibleCanonical
    );
    for unsupported in [
        ConflictCandidate {
            kind: ConflictEntryKind::Configuration,
            structure: ConflictStructure::RegularUtf8SamePath,
        },
        ConflictCandidate {
            kind: ConflictEntryKind::Noncanonical,
            structure: ConflictStructure::RegularUtf8SamePath,
        },
        ConflictCandidate {
            kind: ConflictEntryKind::Document,
            structure: ConflictStructure::Binary,
        },
        ConflictCandidate {
            kind: ConflictEntryKind::Document,
            structure: ConflictStructure::Executable,
        },
        ConflictCandidate {
            kind: ConflictEntryKind::Document,
            structure: ConflictStructure::Symlink,
        },
        ConflictCandidate {
            kind: ConflictEntryKind::Document,
            structure: ConflictStructure::Rename,
        },
        ConflictCandidate {
            kind: ConflictEntryKind::Document,
            structure: ConflictStructure::Delete,
        },
        ConflictCandidate {
            kind: ConflictEntryKind::Document,
            structure: ConflictStructure::AddAdd,
        },
        ConflictCandidate {
            kind: ConflictEntryKind::Document,
            structure: ConflictStructure::IdentityChanged,
        },
    ] {
        assert_eq!(
            conflict_eligibility(&[canonical[0], unsupported]),
            ConflictEligibility::ExternalResolutionRequired,
            "{unsupported:?}"
        );
    }
    assert_eq!(
        conflict_eligibility(&[]),
        ConflictEligibility::ExternalResolutionRequired
    );
}

#[test]
fn redacted_recovery_contracts_never_format_caller_bytes_or_identity() {
    let secret = b"private resolution body";
    let request = ResolveSynchronizationRequest {
        root: PathBuf::from("/repository"),
        synchronization_id: OperationId::new(),
        attempt_id: OperationId::new(),
        observation: ConflictObservation::for_testing([7; 32]),
        resolutions: vec![(
            super::ConflictPathToken {
                observation: ConflictObservation::for_testing([7; 32]),
                ordinal: 0,
                path: b"private-path".to_vec(),
                base: None,
                base_mode: None,
                local: None,
                local_mode: None,
                incoming: None,
                incoming_mode: None,
            },
            RedactedConflictBytes::new(secret.to_vec()),
        )],
        identity: None,
    };
    let formatted = format!("{request:?}");
    assert!(!formatted.contains("private resolution body"));
    assert!(formatted.contains("redacted"));
    assert_eq!(
        commit_identity_boundary(None, None),
        CommitIdentityBoundary::ConfirmationRequired
    );
    assert_eq!(
        commit_identity_boundary(
            Some(&crate::repository::CommitIdentity {
                name: "Test".into(),
                email: "test@example.invalid".into(),
            }),
            None,
        ),
        CommitIdentityBoundary::EffectiveIdentity
    );
}

fn signature() -> Signature<'static> {
    Signature::new(
        "Merge Test",
        "merge@example.invalid",
        &git2::Time::new(0, 0),
    )
    .unwrap()
}

fn entry(path: &str, id: git2::Oid, mode: u32) -> IndexEntry {
    IndexEntry {
        ctime: IndexTime::new(0, 0),
        mtime: IndexTime::new(0, 0),
        dev: 0,
        ino: 0,
        mode,
        uid: 0,
        gid: 0,
        file_size: 0,
        id,
        flags: 0,
        flags_extended: 0,
        path: path.as_bytes().to_vec(),
    }
}

fn commit_with_file(
    repository: &Repository,
    parent: Option<&git2::Commit<'_>>,
    path: &str,
    content: &[u8],
) -> git2::Oid {
    let mut index = Index::new().unwrap();
    if let Some(parent) = parent {
        index.read_tree(&parent.tree().unwrap()).unwrap();
    }
    let blob = repository.blob(content).unwrap();
    index.add(&entry(path, blob, 0o100644)).unwrap();
    let tree = repository
        .find_tree(index.write_tree_to(repository).unwrap())
        .unwrap();
    repository
        .commit(
            None,
            &signature(),
            &signature(),
            "fixture",
            &tree,
            &parent.into_iter().collect::<Vec<_>>(),
        )
        .unwrap()
}

fn divergent_repository(
    path: &Path,
    local: &[u8],
    incoming: &[u8],
) -> (Repository, git2::Oid, git2::Oid) {
    let repository = Repository::init(path).unwrap();
    repository.set_head("refs/heads/main").unwrap();
    let base_oid = commit_with_file(&repository, None, "merge.txt", b"one\ntwo\nthree\n");
    let base = repository.find_commit(base_oid).unwrap();
    let local_oid = commit_with_file(&repository, Some(&base), "merge.txt", local);
    let incoming_oid = commit_with_file(&repository, Some(&base), "merge.txt", incoming);
    drop(base);
    repository
        .reference("refs/heads/main", local_oid, true, "fixture")
        .unwrap();
    repository.set_head("refs/heads/main").unwrap();
    let mut checkout = git2::build::CheckoutBuilder::new();
    checkout.force();
    repository.checkout_head(Some(&mut checkout)).unwrap();
    (repository, local_oid, incoming_oid)
}

fn snapshot_objects(repository: &Repository) -> BTreeSet<PathBuf> {
    let objects = repository.path().join("objects");
    let mut paths = BTreeSet::new();
    for prefix in fs::read_dir(objects).unwrap() {
        let prefix = prefix.unwrap();
        if prefix.file_type().unwrap().is_dir() {
            for object in fs::read_dir(prefix.path()).unwrap() {
                paths.insert(object.unwrap().path());
            }
        }
    }
    paths
}

#[derive(Debug, PartialEq, Eq)]
enum RefStorageEntry {
    Directory,
    File(Vec<u8>),
    Symlink(PathBuf),
}

fn snapshot_ref_storage_path(
    path: &Path,
    relative_path: &Path,
    snapshot: &mut BTreeMap<PathBuf, RefStorageEntry>,
) {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
        Err(error) => panic!("could not read ref storage {}: {error}", path.display()),
    };
    let file_type = metadata.file_type();
    if file_type.is_dir() {
        snapshot.insert(relative_path.to_owned(), RefStorageEntry::Directory);
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            snapshot_ref_storage_path(
                &entry.path(),
                &relative_path.join(entry.file_name()),
                snapshot,
            );
        }
    } else if file_type.is_file() {
        snapshot.insert(
            relative_path.to_owned(),
            RefStorageEntry::File(fs::read(path).unwrap()),
        );
    } else if file_type.is_symlink() {
        snapshot.insert(
            relative_path.to_owned(),
            RefStorageEntry::Symlink(fs::read_link(path).unwrap()),
        );
    } else {
        panic!("unexpected ref storage entry {}", path.display());
    }
}

fn snapshot_ref_storage(repository: &Repository) -> BTreeMap<PathBuf, RefStorageEntry> {
    let mut snapshot = BTreeMap::new();
    for (name, root) in [
        ("common", repository.commondir()),
        ("git", repository.path()),
    ] {
        for path in ["HEAD", "packed-refs", "refs", "logs"] {
            snapshot_ref_storage_path(&root.join(path), &Path::new(name).join(path), &mut snapshot);
        }
    }
    snapshot
}

#[test]
fn worker_mempack_merge_keeps_generated_blobs_out_of_destination_odb_until_import() {
    let directory = tempfile::tempdir().unwrap();
    let (primary, local_oid, incoming_oid) = divergent_repository(
        directory.path(),
        b"local\ntwo\nthree\n",
        b"one\ntwo\nincoming\n",
    );
    let before_objects = snapshot_objects(&primary);
    let before_ref_storage = snapshot_ref_storage(&primary);
    let before_index = fs::read(primary.path().join("index")).unwrap();
    let before_worktree = fs::read(directory.path().join("merge.txt")).unwrap();
    let before_head = primary.head().unwrap().target();
    let other = Repository::open(directory.path()).unwrap();

    let worker = Repository::open(directory.path()).unwrap();
    let odb = worker.odb().unwrap();
    let mempack = odb.add_new_mempack_backend(1000).unwrap();
    let local = worker.find_commit(local_oid).unwrap();
    let incoming = worker.find_commit(incoming_oid).unwrap();
    let index = worker.merge_commits(&local, &incoming, None).unwrap();
    assert!(!index.has_conflicts());
    let generated = index.get_path(Path::new("merge.txt"), 0).unwrap().id;
    assert_eq!(
        worker.find_blob(generated).unwrap().content(),
        b"local\ntwo\nincoming\n"
    );

    assert_eq!(snapshot_objects(&primary), before_objects);
    assert_eq!(snapshot_ref_storage(&primary), before_ref_storage);
    assert_eq!(
        fs::read(primary.path().join("index")).unwrap(),
        before_index
    );
    assert_eq!(
        fs::read(directory.path().join("merge.txt")).unwrap(),
        before_worktree
    );
    assert_eq!(primary.head().unwrap().target(), before_head);
    assert!(other.find_blob(generated).is_err());

    // Mempack object lifetime is bound to this backend: clearing it loses the
    // prepared blob until preparation is repeated, without touching disk.
    mempack.reset().unwrap();
    assert!(worker.find_blob(generated).is_err());
    let index = worker.merge_commits(&local, &incoming, None).unwrap();
    assert_eq!(
        index.get_path(Path::new("merge.txt"), 0).unwrap().id,
        generated
    );
    assert_eq!(snapshot_ref_storage(&primary), before_ref_storage);
    let generated_object = odb.read(generated).unwrap();
    let imported = primary
        .odb()
        .unwrap()
        .write(generated_object.kind(), generated_object.data())
        .unwrap();
    assert_eq!(imported, generated);
    assert_eq!(
        primary.find_blob(imported).unwrap().content(),
        b"local\ntwo\nincoming\n"
    );
    assert_eq!(primary.find_blob(imported).unwrap().id(), imported);
}

#[test]
fn merge_commits_is_index_only_but_merge_preserves_real_conflict_state_until_cleanup() {
    let directory = tempfile::tempdir().unwrap();
    let (mut repository, local_oid, incoming_oid) =
        divergent_repository(directory.path(), b"local\n", b"incoming\n");
    let local = repository.find_commit(local_oid).unwrap();
    let incoming = repository.find_commit(incoming_oid).unwrap();
    let index_only = repository.merge_commits(&local, &incoming, None).unwrap();
    assert!(index_only.has_conflicts());
    assert_eq!(repository.head().unwrap().target(), Some(local_oid));
    assert!(
        !fs::read_to_string(directory.path().join("merge.txt"))
            .unwrap()
            .contains("<<<<<<<")
    );
    drop(index_only);
    drop(local);
    drop(incoming);

    repository
        .branch("linked", &repository.find_commit(local_oid).unwrap(), false)
        .unwrap();
    let linked_path = directory.path().join("linked-worktree");
    let reference = repository.find_reference("refs/heads/linked").unwrap();
    let mut worktree_options = git2::WorktreeAddOptions::new();
    worktree_options.reference(Some(&reference));
    repository
        .worktree("linked-worktree", &linked_path, Some(&worktree_options))
        .unwrap();
    drop(reference);
    let mut linked = Repository::open(&linked_path).unwrap();
    assert!(!linked.index().unwrap().has_conflicts());

    let annotated = repository.find_annotated_commit(incoming_oid).unwrap();
    let mut checkout = git2::build::CheckoutBuilder::new();
    checkout.safe().overwrite_ignored(false);
    repository
        .merge(&[&annotated], None, Some(&mut checkout))
        .unwrap();
    drop(annotated);
    let index = repository.index().unwrap();
    assert!(index.has_conflicts());
    let conflict = index.conflicts().unwrap().next().unwrap().unwrap();
    assert!(conflict.ancestor.is_some() && conflict.our.is_some() && conflict.their.is_some());
    assert_eq!(conflict.our.unwrap().mode, 0o100644);
    assert_eq!(repository.head().unwrap().target(), Some(local_oid));
    assert!(
        fs::read_to_string(directory.path().join("merge.txt"))
            .unwrap()
            .contains("<<<<<<<")
    );
    let mut heads = Vec::new();
    repository
        .mergehead_foreach(|oid| {
            heads.push(*oid);
            true
        })
        .unwrap();
    assert_eq!(heads, vec![incoming_oid]);
    assert!(!linked.index().unwrap().has_conflicts());
    assert!(linked.mergehead_foreach(|_| true).is_err());
    drop(index);
    repository.cleanup_state().unwrap();
    assert!(repository.mergehead_foreach(|_| true).is_err());
}

#[test]
fn clean_merge_index_retains_both_parent_trees_and_never_selects_a_conflict_side() {
    let directory = tempfile::tempdir().unwrap();
    let (repository, local_oid, incoming_oid) = divergent_repository(
        directory.path(),
        b"local\ntwo\nthree\n",
        b"one\ntwo\nincoming\n",
    );
    let local = repository.find_commit(local_oid).unwrap();
    let incoming = repository.find_commit(incoming_oid).unwrap();
    let mut index = repository.merge_commits(&local, &incoming, None).unwrap();
    assert!(!index.has_conflicts());
    let tree = repository
        .find_tree(index.write_tree_to(&repository).unwrap())
        .unwrap();
    let merged = repository
        .commit(
            None,
            &signature(),
            &signature(),
            "merge",
            &tree,
            &[&local, &incoming],
        )
        .unwrap();
    let merged = repository.find_commit(merged).unwrap();
    assert_eq!(merged.parent_count(), 2);
    assert_eq!(merged.parent_id(0).unwrap(), local_oid);
    assert_eq!(merged.parent_id(1).unwrap(), incoming_oid);
    let blob = repository
        .find_blob(
            merged
                .tree()
                .unwrap()
                .get_path(Path::new("merge.txt"))
                .unwrap()
                .id(),
        )
        .unwrap();
    assert_eq!(blob.content(), b"local\ntwo\nincoming\n");
}

#[test]
fn safe_checkout_does_not_overwrite_ignored_or_untracked_collisions() {
    let directory = tempfile::tempdir().unwrap();
    let repository = Repository::init(directory.path()).unwrap();
    repository.set_head("refs/heads/main").unwrap();
    let initial = commit_with_file(&repository, None, ".gitignore", b"ignored\n");
    repository
        .reference("refs/heads/main", initial, true, "fixture")
        .unwrap();
    let mut checkout = git2::build::CheckoutBuilder::new();
    checkout.force();
    repository.checkout_head(Some(&mut checkout)).unwrap();
    fs::write(
        directory.path().join(".gitignore"),
        b"locally modified tracked file\n",
    )
    .unwrap();
    let mut index = repository.index().unwrap();
    let blob = repository.blob(b"tracked replacement").unwrap();
    index.add(&entry(".gitignore", blob, 0o100644)).unwrap();
    let mut safe = git2::build::CheckoutBuilder::new();
    safe.safe().overwrite_ignored(false);
    assert!(
        repository
            .checkout_index(Some(&mut index), Some(&mut safe))
            .is_err()
    );
    assert_eq!(
        fs::read(directory.path().join(".gitignore")).unwrap(),
        b"locally modified tracked file\n"
    );

    fs::write(directory.path().join("ignored"), b"do not replace").unwrap();
    let blob = repository.blob(b"tracked replacement").unwrap();
    index.add(&entry("ignored", blob, 0o100644)).unwrap();
    let mut safe = git2::build::CheckoutBuilder::new();
    safe.safe().overwrite_ignored(false);
    assert!(
        repository
            .checkout_index(Some(&mut index), Some(&mut safe))
            .is_err()
    );
    assert_eq!(
        fs::read(directory.path().join("ignored")).unwrap(),
        b"do not replace"
    );

    fs::write(directory.path().join("untracked"), b"do not replace").unwrap();
    let blob = repository.blob(b"tracked replacement").unwrap();
    index.add(&entry("untracked", blob, 0o100644)).unwrap();
    let mut safe = git2::build::CheckoutBuilder::new();
    safe.safe().overwrite_ignored(false);
    assert!(
        repository
            .checkout_index(Some(&mut index), Some(&mut safe))
            .is_err()
    );
    assert_eq!(
        fs::read(directory.path().join("untracked")).unwrap(),
        b"do not replace"
    );
}

#[cfg(unix)]
#[test]
fn safe_checkout_does_not_replace_symlink_or_file_directory_collisions() {
    use std::os::unix::fs::symlink;

    let directory = tempfile::tempdir().unwrap();
    let repository = Repository::init(directory.path()).unwrap();
    let mut index = Index::new().unwrap();
    let blob = repository.blob(b"replacement").unwrap();
    index.add(&entry("collision", blob, 0o100644)).unwrap();
    symlink("outside", directory.path().join("collision")).unwrap();
    let mut safe = git2::build::CheckoutBuilder::new();
    safe.safe().overwrite_ignored(false);
    assert!(
        repository
            .checkout_index(Some(&mut index), Some(&mut safe))
            .is_err()
    );
    assert!(
        fs::symlink_metadata(directory.path().join("collision"))
            .unwrap()
            .file_type()
            .is_symlink()
    );

    fs::remove_file(directory.path().join("collision")).unwrap();
    fs::create_dir(directory.path().join("collision")).unwrap();
    fs::write(directory.path().join("collision/child"), b"keep").unwrap();
    let mut safe = git2::build::CheckoutBuilder::new();
    safe.safe().overwrite_ignored(false);
    assert!(
        repository
            .checkout_index(Some(&mut index), Some(&mut safe))
            .is_err()
    );
    assert_eq!(
        fs::read(directory.path().join("collision/child")).unwrap(),
        b"keep"
    );
}
