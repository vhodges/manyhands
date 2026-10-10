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

/// Overwrites the loose object `oid` with bytes that are not an object,
/// so that reading it fails although it is there.
fn corrupt(repository: &Repository, oid: Oid) {
    let name = oid.to_string();
    let path = repository
        .path()
        .join("objects")
        .join(&name[..2])
        .join(&name[2..]);
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    permissions.set_readonly(false);
    fs::set_permissions(&path, permissions).unwrap();
    fs::write(&path, b"not an object").unwrap();
}

impl Fixture {
    fn evidence(&self, position: &Position) -> Result<PathEvidence, Unreadable> {
        path_evidence(self.root(), position, PATH, &|bytes| bytes == b"intended\n")
    }
}

#[test]
fn a_recorded_commit_that_cannot_be_read_is_not_taken_for_one_that_is_not_an_ancestor() {
    let fixture = fixture();
    let made = commit(
        &fixture.repository,
        BRANCH,
        Some(fixture.base),
        PATH,
        "intended\n",
    );
    // The recorded commit is on another branch, so the walk of this one
    // never reads it: only the question "is it an ancestor?" does.
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
    corrupt(&fixture.repository, elsewhere);
    // A fresh handle: the object may be cached in the one that wrote it.
    assert_eq!(fixture.intended(&position), Err(Unreadable));
    assert_eq!(fixture.evidence(&position).err(), Some(Unreadable));
    assert_eq!(
        path_commit(fixture.root(), &position, PATH),
        Err(Unreadable)
    );
    assert_eq!(
        confirms(fixture.root(), &position, made, PATH),
        Claim::Unreadable
    );

    // A recorded commit the repository does not hold is not an ancestor:
    // the range is the whole branch.
    let missing = Position {
        base_ref: Some(BRANCH.to_owned()),
        base_oid: Some(Oid::from_bytes(&[7; 20]).unwrap()),
    };
    assert_eq!(fixture.intended(&missing), Ok(Some(made)));
}

#[test]
fn a_file_that_cannot_be_read_is_not_taken_for_no_file() {
    let fixture = fixture();
    let made = commit(
        &fixture.repository,
        BRANCH,
        Some(fixture.base),
        PATH,
        "intended\n",
    );
    assert_eq!(fixture.intended(&fixture.position()), Ok(Some(made)));
    let blob = fixture
        .repository
        .find_commit(made)
        .unwrap()
        .tree()
        .unwrap()
        .get_path(Path::new(PATH))
        .unwrap()
        .id();
    corrupt(&fixture.repository, blob);
    assert_eq!(fixture.intended(&fixture.position()), Err(Unreadable));
    assert_eq!(
        fixture.evidence(&fixture.position()).err(),
        Some(Unreadable)
    );
    // Something other than a file at the path is no file, and is read
    // from the tree alone.
    let directory = commit(
        &fixture.repository,
        "refs/heads/directory",
        None,
        "tickets/a.md/inside",
        "x\n",
    );
    let position = Position {
        base_ref: Some("refs/heads/directory".to_owned()),
        base_oid: None,
    };
    assert_eq!(fixture.intended(&position), Ok(None));
    assert_eq!(
        confirms(fixture.root(), &position, directory, PATH),
        Claim::NotThisRequests
    );
}

/// The journal row's answer before a call, for the commit rule.
const STARTED: bool = true;
const NOT_STARTED: bool = false;

#[test]
fn the_request_s_commit_is_the_oldest_in_range_that_left_the_path_as_intended() {
    let fixture = fixture();
    let position = fixture.position();
    let nothing = fixture.evidence(&position).unwrap();
    assert_eq!(nothing.own(), None);
    assert!(!nothing.superseded());

    let made = commit(
        &fixture.repository,
        BRANCH,
        Some(fixture.base),
        PATH,
        "intended\n",
    );
    let unrelated = commit(&fixture.repository, BRANCH, Some(made), "other.md", "x\n");
    let one = fixture.evidence(&position).unwrap();
    assert_eq!(one.own(), Some(made));
    assert!(!one.superseded());

    // A later save of something else is on top: the commit is still the
    // request's, and the path is no longer what it intended.
    let later = commit(
        &fixture.repository,
        BRANCH,
        Some(unrelated),
        PATH,
        "later\n",
    );
    let two = fixture.evidence(&position).unwrap();
    assert_eq!(two.own(), Some(made));
    assert!(two.superseded());

    // The same content saved again by someone else is not the request's.
    commit(&fixture.repository, BRANCH, Some(later), PATH, "intended\n");
    let three = fixture.evidence(&position).unwrap();
    assert_eq!(three.own(), Some(made));
    assert!(!three.superseded());
}

#[test]
fn a_commit_is_reported_only_for_a_request_that_had_started_or_that_made_it() {
    let fixture = fixture();
    let position = fixture.position();
    let before = fixture.evidence(&position).unwrap();
    let made = commit(
        &fixture.repository,
        BRANCH,
        Some(fixture.base),
        PATH,
        "intended\n",
    );
    let after = fixture.evidence(&position).unwrap();

    // It was not there before the call: this call made it.
    assert_eq!(after.reported(NOT_STARTED, &before), Some(made));
    assert_eq!(after.reported(STARTED, &before), Some(made));
    // It was there before the call. An earlier attempt that had started
    // made it; with none, it is someone else's identical content.
    assert_eq!(after.reported(STARTED, &after), Some(made));
    assert_eq!(after.reported(NOT_STARTED, &after), None);
    // Nothing in range is nothing to report, started or not.
    assert_eq!(before.reported(STARTED, &before), None);

    // Someone else's identical commit was there before the call, the
    // content changed, and this call committed it again: the commit
    // reported is the one that was not there.
    let other = commit(&fixture.repository, BRANCH, Some(made), PATH, "other\n");
    let again = commit(&fixture.repository, BRANCH, Some(other), PATH, "intended\n");
    let later = fixture.evidence(&position).unwrap();
    assert_eq!(later.reported(NOT_STARTED, &after), Some(again));
    assert_eq!(later.reported(STARTED, &after), Some(made));
}
