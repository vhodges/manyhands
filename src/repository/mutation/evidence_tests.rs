use std::{fs, path::Path};

use git2::{Repository, Signature, Time};

use super::*;

const PATH: &str = "tickets/a.md";
const BRANCH: &str = "refs/heads/item";

/// Commits `content` at `path` on `reference`, on top of `parent`.
fn commit(
    repository: &Repository,
    reference: &str,
    parent: Option<Oid>,
    path: &str,
    content: &str,
) -> Oid {
    let signature = Signature::new("Test", "test@example.invalid", &Time::new(0, 0)).unwrap();
    let parent = parent.map(|parent| repository.find_commit(parent).unwrap());
    let mut index = git2::Index::new().unwrap();
    if let Some(parent) = &parent {
        index.read_tree(&parent.tree().unwrap()).unwrap();
    }
    let blob = repository.blob(content.as_bytes()).unwrap();
    index
        .add(&git2::IndexEntry {
            ctime: git2::IndexTime::new(0, 0),
            mtime: git2::IndexTime::new(0, 0),
            dev: 0,
            ino: 0,
            mode: 0o100644,
            uid: 0,
            gid: 0,
            file_size: content.len() as u32,
            id: blob,
            flags: 0,
            flags_extended: 0,
            path: path.as_bytes().to_vec(),
        })
        .unwrap();
    let tree = repository
        .find_tree(index.write_tree_to(repository).unwrap())
        .unwrap();
    let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();
    let oid = repository
        .commit(None, &signature, &signature, "c", &tree, &parents)
        .unwrap();
    repository.reference(reference, oid, true, "test").unwrap();
    oid
}

struct Fixture {
    directory: tempfile::TempDir,
    repository: Repository,
    /// Where the branch stood when the request was accepted.
    base: Oid,
}

fn fixture() -> Fixture {
    let directory = tempfile::tempdir().unwrap();
    let repository = Repository::init(directory.path()).unwrap();
    let base = commit(&repository, BRANCH, None, PATH, "before\n");
    Fixture {
        directory,
        repository,
        base,
    }
}

impl Fixture {
    fn root(&self) -> &Path {
        self.directory.path()
    }

    fn position(&self) -> Position {
        Position {
            base_ref: Some(BRANCH.to_owned()),
            base_oid: Some(self.base),
        }
    }

    fn intended(&self, position: &Position) -> Result<Option<Oid>, Unreadable> {
        intended_commit(self.root(), position, PATH, &|bytes| bytes == b"intended\n")
    }
}

#[test]
fn a_commit_in_range_that_left_the_path_as_intended_is_found() {
    let fixture = fixture();
    let made = commit(
        &fixture.repository,
        BRANCH,
        Some(fixture.base),
        PATH,
        "intended\n",
    );
    // An unrelated commit on top does not hide it.
    commit(&fixture.repository, BRANCH, Some(made), "other.md", "x\n");
    assert_eq!(fixture.intended(&fixture.position()), Ok(Some(made)));
    assert_eq!(
        confirms(fixture.root(), &fixture.position(), made, PATH),
        Claim::Confirmed
    );
}

#[test]
fn a_commit_in_range_that_left_the_path_as_something_else_is_not_the_request_s() {
    let fixture = fixture();
    let made = commit(
        &fixture.repository,
        BRANCH,
        Some(fixture.base),
        PATH,
        "intended\n",
    );
    let foreign = commit(&fixture.repository, BRANCH, Some(made), PATH, "foreign\n");
    assert_eq!(fixture.intended(&fixture.position()), Ok(None));
    // Both changed the path and are in range, whatever they left there.
    assert_eq!(
        confirms(fixture.root(), &fixture.position(), foreign, PATH),
        Claim::Confirmed
    );
}

#[test]
fn nothing_in_range_is_no_commit() {
    let fixture = fixture();
    assert_eq!(fixture.intended(&fixture.position()), Ok(None));
    // The recorded commit itself is not in range, although it changed the
    // path: a save that changed nothing names the head it found.
    assert_eq!(
        confirms(fixture.root(), &fixture.position(), fixture.base, PATH),
        Claim::NotThisRequests
    );
    // In range, and did not change the path.
    let unrelated = commit(
        &fixture.repository,
        BRANCH,
        Some(fixture.base),
        "other.md",
        "x\n",
    );
    assert_eq!(fixture.intended(&fixture.position()), Ok(None));
    assert_eq!(
        confirms(fixture.root(), &fixture.position(), unrelated, PATH),
        Claim::NotThisRequests
    );
    // A branch that was never created holds nothing.
    let absent = Position {
        base_ref: Some("refs/heads/absent".to_owned()),
        base_oid: Some(fixture.base),
    };
    assert_eq!(fixture.intended(&absent), Ok(None));
}

#[test]
fn a_recorded_commit_that_is_not_an_ancestor_gives_the_whole_branch() {
    let fixture = fixture();
    let made = commit(
        &fixture.repository,
        BRANCH,
        Some(fixture.base),
        PATH,
        "intended\n",
    );
    let elsewhere = commit(
        &fixture.repository,
        "refs/heads/elsewhere",
        None,
        PATH,
        "e\n",
    );
    let position = Position {
        base_ref: Some(BRANCH.to_owned()),
        base_oid: Some(elsewhere),
    };
    assert_eq!(fixture.intended(&position), Ok(Some(made)));
    // The branch's first commit is in range now.
    assert_eq!(
        confirms(fixture.root(), &position, fixture.base, PATH),
        Claim::Confirmed
    );
    // And with nothing recorded.
    let nothing = Position {
        base_ref: Some(BRANCH.to_owned()),
        base_oid: None,
    };
    assert_eq!(fixture.intended(&nothing), Ok(Some(made)));
}

#[test]
fn a_repository_that_cannot_be_read_is_neither_answer() {
    let fixture = fixture();
    let nowhere = fixture.root().join("absent");
    assert_eq!(
        confirms(&nowhere, &fixture.position(), fixture.base, PATH),
        Claim::Unreadable
    );
    assert_eq!(
        intended_commit(&nowhere, &fixture.position(), PATH, &|_| true),
        Err(Unreadable)
    );
    // A claimed commit the repository does not hold.
    fs::write(fixture.root().join("x"), "x").unwrap();
    let made = commit(
        &fixture.repository,
        BRANCH,
        Some(fixture.base),
        PATH,
        "intended\n",
    );
    let missing = Oid::from_bytes(&[7; 20]).unwrap();
    assert_eq!(
        confirms(fixture.root(), &fixture.position(), missing, PATH),
        Claim::NotThisRequests
    );
    assert_eq!(
        confirms(fixture.root(), &fixture.position(), made, PATH),
        Claim::Confirmed
    );
}
