//! Shared helpers for the draft store tests. Draft types derive no `Debug`,
//! so these helpers read them through `match` rather than `assert_eq!`.

use manyhands::drafts::{
    DraftClaim, DraftError, DraftErrorKind, DraftFields, DraftId, DraftStore, DraftTarget, Entry,
    Held, ItemKind, Listing,
};
use std::path::{Path, PathBuf};

pub fn target() -> DraftTarget {
    DraftTarget {
        repository: "/home/u/work/project".to_owned(),
        item_kind: ItemKind::Document,
        item_id: "01M5CS4MJ0G7B0MPJD0KN0QMYA".parse().expect("item id"),
        comment: None,
    }
}

/// Document fields whose body names the revision they were written at.
pub fn fields(revision: u64) -> DraftFields {
    DraftFields::Document {
        title: "Title".to_owned(),
        body: format!("body at revision {revision}"),
    }
}

pub fn store(data: &Path) -> DraftStore {
    DraftStore::open_in(data).expect("the store opens")
}

pub fn create(store: &DraftStore) -> DraftClaim {
    store
        .create(target(), None, fields(1))
        .expect("the draft is created")
}

pub fn claim(store: &DraftStore, id: DraftId) -> DraftClaim {
    match store.claim(id) {
        Ok(claim) => claim,
        Err(error) => panic!("the draft must be claimed: {error:?}"),
    }
}

pub fn claim_error(store: &DraftStore, id: DraftId) -> DraftErrorKind {
    match store.claim(id) {
        Ok(_) => panic!("the claim must be refused"),
        Err(error) => error.kind,
    }
}

pub fn write(claim: &mut DraftClaim, revision: u64) -> Result<u64, DraftError> {
    claim
        .write(revision, fields(revision), None)
        .map(|written| written.revision)
}

pub fn list(store: &DraftStore) -> Vec<Listing> {
    store.list().expect("the store lists")
}

/// The one listing for `id`.
pub fn listing(store: &DraftStore, id: DraftId) -> Listing {
    let mut found: Vec<Listing> = list(store).into_iter().filter(|l| l.id == id).collect();
    assert_eq!(found.len(), 1, "one listing for {id}");
    found.pop().unwrap()
}

/// The revision `id` lists at; the draft must be readable, whole, and its
/// text that of the revision.
pub fn listed_revision(store: &DraftStore, id: DraftId) -> u64 {
    match listing(store, id).entry {
        Entry::Draft(draft) => {
            assert!(
                draft.current == fields(draft.revision),
                "revision {} is whole",
                draft.revision
            );
            assert!(draft.id == id);
            draft.revision
        }
        Entry::Unreadable(reason) => panic!("{id} is unreadable: {reason:?}"),
    }
}

pub fn held(store: &DraftStore, id: DraftId) -> Held {
    listing(store, id).held
}

pub fn drafts(data: &Path) -> PathBuf {
    data.join("drafts")
}

/// The names in `drafts/`, sorted.
pub fn names(data: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(drafts(data))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

pub fn temporary_files(data: &Path) -> Vec<String> {
    names(data)
        .into_iter()
        .filter(|name| name.ends_with(".tmp"))
        .collect()
}

/// Runs this test binary's test `test` alone in a child process with `env`.
pub fn child(test: &str, env: &[(&str, &str)]) -> std::process::Command {
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command.args(["--exact", test, "--nocapture", "--test-threads=1"]);
    for (key, value) in env {
        command.env(key, value);
    }
    command
}

/// Asserts that a child's standard output shows it ran exactly one test. A
/// filter that matches no test succeeds too.
pub fn ran_one_test(stdout: &str) {
    assert!(
        stdout.contains("running 1 test\n"),
        "the child must run exactly one test:\n{stdout}"
    );
}
