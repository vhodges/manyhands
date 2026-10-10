use std::{fs, path::Path};

use git2::{Repository, Signature, Time};

use super::*;
/// A file's token, written out from its published format.
fn observation_token_for_testing(branch: Option<&str>, path: &str, source: &[u8]) -> String {
    let mut hasher = blake3::Hasher::new();
    match branch {
        Some(branch) => {
            hasher.update(&[1]);
            hasher.update(&(branch.len() as u64).to_le_bytes());
            hasher.update(branch.as_bytes());
        }
        None => {
            hasher.update(&[0]);
        }
    }
    for bytes in [path.as_bytes(), source] {
        hasher.update(&(bytes.len() as u64).to_le_bytes());
        hasher.update(bytes);
    }
    format!("v1:{}", hasher.finalize().to_hex())
}

const PATH: &str = ".manyhands/tickets/01ARZ3NDEKTSV4RRFFQ69G5FC0/ticket.md";

#[test]
fn the_absent_token_names_its_branch_and_path_and_is_no_file_s_token() {
    let token = absent_token(Some("main"), PATH);
    assert!(token.starts_with("v1:"));
    assert_eq!(token.len(), 3 + 64);
    assert_eq!(token, absent_token(Some("main"), PATH));
    assert_ne!(token, absent_token(Some("other"), PATH));
    assert_ne!(token, absent_token(None, PATH));
    assert_ne!(token, absent_token(Some("main"), "docs/a.md"));
    // Not the token of an empty file there, nor of a file holding the marker.
    assert_ne!(
        token,
        observation_token_for_testing(Some("main"), PATH, b"")
    );
    assert_ne!(
        token,
        observation_token_for_testing(Some("main"), PATH, ABSENT_MARKER)
    );
}

fn commit(repository: &Repository, path: &str) {
    let mut index = repository.index().unwrap();
    index.add_path(Path::new(path)).unwrap();
    index.write().unwrap();
    let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
    let signature = Signature::new("Test", "test@example.invalid", &Time::new(0, 0)).unwrap();
    let parent = repository
        .head()
        .ok()
        .and_then(|head| head.peel_to_commit().ok());
    let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();
    repository
        .commit(Some("HEAD"), &signature, &signature, "c", &tree, &parents)
        .unwrap();
}

#[test]
fn a_file_is_committed_when_it_is_the_blob_at_the_head_of_its_worktree() {
    let directory = tempfile::tempdir().unwrap();
    let repository = Repository::init(directory.path()).unwrap();
    let file = directory.path().join("a.md");

    // No commit at all.
    fs::write(&file, "one\n").unwrap();
    assert!(!is_committed(directory.path(), "a.md", b"one\n").unwrap());

    commit(&repository, "a.md");
    assert!(is_committed(directory.path(), "a.md", b"one\n").unwrap());
    // Edited and not committed.
    assert!(!is_committed(directory.path(), "a.md", b"two\n").unwrap());
    // Never committed.
    assert!(!is_committed(directory.path(), "b.md", b"one\n").unwrap());
    // Nothing is a repository there.
    assert!(is_committed(&directory.path().join("absent"), "a.md", b"one\n").is_err());
}

#[test]
fn a_stale_token_is_already_applied_only_for_the_intended_content_committed() {
    assert_eq!(stale_token_code(true, true), ResultCode::AlreadyApplied);
    assert_eq!(stale_token_code(true, false), ResultCode::ExternalChange);
    assert_eq!(stale_token_code(false, true), ResultCode::ExternalChange);
    assert_eq!(stale_token_code(false, false), ResultCode::ExternalChange);
}

#[test]
fn a_token_that_matches_gives_the_digest_of_the_bytes_it_was_made_from() {
    let observed = ObservedFile {
        token: observation_token_for_testing(Some("main"), PATH, b"bytes"),
        bytes: b"bytes".to_vec(),
    };
    assert_eq!(
        observed.check(&observed.token.clone()),
        Some(ExpectedPathObservation::from_bytes(b"bytes"))
    );
    assert_eq!(observed.check("v1:stale"), None);
    assert_eq!(observed.check(""), None);
}
