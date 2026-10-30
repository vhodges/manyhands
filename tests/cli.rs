//! The CLI contract exercised through the real binary.

use std::{
    collections::BTreeSet,
    fs,
    path::PathBuf,
    time::{Duration, Instant},
};

use manyhands::repository::{LeaseKind, RepositoryService};
use support::cli::{CLI_CASES, CLI_TEXT_CASES, cli_fixture_directory};

mod support;

#[path = "cli/admin.rs"]
mod admin;

#[path = "cli/authoring_exits.rs"]
mod authoring_exits;

#[path = "cli/authoring_reads.rs"]
mod authoring_reads;

#[path = "cli/boundaries.rs"]
mod boundaries;

#[path = "cli/collaboration.rs"]
mod collaboration;

#[path = "cli/conflicts.rs"]
mod conflicts;

#[path = "cli/discussion.rs"]
mod discussion;

#[path = "cli/content.rs"]
mod content;

#[path = "cli/documents.rs"]
mod documents;

#[path = "cli/help.rs"]
mod help;

#[path = "cli/folders.rs"]
mod folders;
#[path = "cli/index.rs"]
mod index;

#[path = "cli/exits.rs"]
mod exits;
#[path = "cli/human.rs"]
mod human;
#[path = "cli/input_schemas.rs"]
mod input_schemas;
#[path = "cli/keys.rs"]
mod keys;
#[path = "cli/mutation.rs"]
mod mutation;
#[path = "cli/privacy.rs"]
mod privacy;
#[cfg(unix)]
#[path = "cli/prompts.rs"]
mod prompts;
#[path = "cli/remotes.rs"]
mod remotes;
#[path = "cli/repair.rs"]
mod repair;
#[path = "cli/repository.rs"]
mod repository;
#[path = "cli/republish.rs"]
mod republish;
#[path = "cli/requests.rs"]
mod requests;

#[path = "cli/relationships.rs"]
mod relationships;

#[path = "cli/ticket_authoring.rs"]
mod ticket_authoring;

#[test]
fn fixture_directory_is_exactly_the_registered_cases() {
    let actual: BTreeSet<_> = fs::read_dir(cli_fixture_directory())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    let expected: BTreeSet<_> = CLI_CASES
        .iter()
        .map(|name| format!("{name}.json"))
        .chain(CLI_TEXT_CASES.iter().map(|name| format!("{name}.txt")))
        .collect();
    assert_eq!(actual, expected);
    assert_eq!(expected.len(), CLI_CASES.len() + CLI_TEXT_CASES.len());
}

#[test]
fn index_lease_child() {
    let Ok(root) = std::env::var("MANYHANDS_LEASE_ROOT") else {
        return;
    };
    let data_directory = PathBuf::from(std::env::var("MANYHANDS_LEASE_DATA_DIRECTORY").unwrap());
    let kind = LeaseKind::parse(&std::env::var("MANYHANDS_LEASE_KIND").unwrap()).unwrap();
    let ready = PathBuf::from(std::env::var("MANYHANDS_LEASE_READY").unwrap());
    let release = PathBuf::from(std::env::var("MANYHANDS_LEASE_RELEASE").unwrap());
    let _holder = RepositoryService::hold_lease_for_testing(
        std::path::Path::new(&root),
        &data_directory,
        kind,
    )
    .unwrap();
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(ready)
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !release.exists() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
}
