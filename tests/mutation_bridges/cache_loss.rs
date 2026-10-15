//! After the cache is lost, for the bridges: the request records, the
//! confirmations, the journals, the registration and the host pins are
//! gone with the database, and a retried request is decided from Git and
//! the file system. The same rule for the saves and the keys is in
//! `tests/mutation_replay/cache_loss.rs`; these are here because every
//! test of a bridge keeps to a Git home of its own.

use std::{fs, path::Path};

use manyhands::{
    repository::{
        FolderCreateInput, HostApproveInput, HostReplaceInput, InitialsChange, Mutation,
        PreviewDetailsDto, RepoIdentitySetInput,
        transport::{HostKeyIdentity, SshAuthority},
    },
    results::{DiscoveryEffect, Outcome, ResultCode},
};

use super::support::{
    confirmation::{Desk, Home, Plain},
    git_home,
    mutation::{REQUEST_1, REQUEST_2, REQUEST_3, World, recovery_actions},
};

/// Deletes the database under `data`, as `World::lose_cache` does.
fn lose_cache(data: &Path) {
    for entry in fs::read_dir(data).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if name.starts_with("manyhands.sqlite3") && !name.ends_with(".lock") {
            fs::remove_file(&path).unwrap();
        }
    }
}

/// An identity is set on a repository that need not be registered, so
/// nothing but the confirmation is lost with the cache: a request that
/// presents one from before is `confirmation_not_found` and writes
/// nothing, and once the identity is set a new preview is a no-op.
#[test]
fn an_identity_request_after_the_cache_is_lost_needs_a_new_preview() {
    git_home::isolated();
    let mut desk = Desk::new();
    let plain = Plain::new();
    let set = || {
        Mutation::RepoIdentitySet(RepoIdentitySetInput {
            root: plain.root.clone(),
            name: "Ada Example".to_owned(),
            email: "ada@example.invalid".to_owned(),
            initials: InitialsChange::Unchanged,
        })
    };
    let local = || fs::read(plain.root.join(".git/config")).unwrap();

    // A preview, and then the loss: its confirmation is gone.
    let (confirmation, _) = desk.preview(&set());
    let before = local();
    lose_cache(desk.data.path());
    desk.reopen();
    let refused = desk.execute(REQUEST_1, &confirmation, set());
    assert_eq!(refused.code, ResultCode::ConfirmationNotFound);
    assert_eq!(recovery_actions(&refused), ["request.prepare"]);
    assert_eq!(local(), before);
    desk.assert_nothing_left(REQUEST_1);

    // A request that was finished, and then the loss: its record and
    // its confirmation are gone, and the identity is in Git.
    let (confirmation, _) = desk.preview(&set());
    let done = desk.execute(REQUEST_1, &confirmation, set());
    assert_eq!(done.code, ResultCode::Ok, "{}", done.message);
    let written = local();
    lose_cache(desk.data.path());
    desk.reopen();
    let retried = desk.execute(REQUEST_1, &confirmation, set());
    assert_eq!(retried.code, ResultCode::ConfirmationNotFound);
    assert_eq!(recovery_actions(&retried), ["request.prepare"]);
    desk.assert_nothing_left(REQUEST_1);
    // The new preview shows the end state: nothing is left to confirm.
    let noop = desk.prepare(&set());
    assert_eq!(
        (noop.outcome, noop.code),
        (Outcome::Noop, ResultCode::AlreadyApplied)
    );
    assert_eq!(noop.data, None);
    assert_eq!(local(), written);
}

const LOCATION: &str = "ssh://fixture@git.example.invalid:2222/project.git";

fn authority() -> SshAuthority {
    SshAuthority {
        host: "git.example.invalid".to_owned(),
        port: 2222,
    }
}

fn key(letter: &str) -> HostKeyIdentity {
    HostKeyIdentity {
        algorithm: "ssh-ed25519".to_owned(),
        sha256: format!("SHA256:{}A", letter.repeat(42)),
    }
}

/// The pins are in the database and go with it. A host approval from
/// before is refused before anything is observed or connected to, for
/// its confirmation; a new preview is refused until the repository is
/// rebuilt. Then a preview shows the host with no pin, and a
/// replacement of the pin that was lost has nothing to replace. Nothing
/// answers at the host, so a connection would be a failure of the
/// transport.
#[test]
fn a_host_approval_after_the_cache_is_lost_is_of_a_host_with_no_pin() {
    git_home::isolated();
    let mut home = Home::new();
    home.add_remote("origin", LOCATION);
    home.select(REQUEST_3, Some("origin"));
    let root = home.root().to_owned();
    let (old, new) = (key("a"), key("b"));
    home.service()
        .approve_host_pin_for_testing(&authority(), &old)
        .unwrap();
    let replace = || {
        Mutation::HostReplace(HostReplaceInput {
            root: root.clone(),
            authority: authority(),
            algorithm: new.algorithm.clone(),
            fingerprint: new.sha256.clone(),
            old_fingerprint: old.sha256.clone(),
        })
    };
    let approve = || {
        Mutation::HostApprove(HostApproveInput {
            root: root.clone(),
            authority: authority(),
            algorithm: new.algorithm.clone(),
            fingerprint: new.sha256.clone(),
        })
    };
    let (confirmation, _) = home.preview(&replace());

    home.world.lose_cache();
    let refused = home.execute(REQUEST_1, &confirmation, replace());
    assert_eq!(refused.code, ResultCode::ConfirmationNotFound);
    assert_eq!(recovery_actions(&refused), ["request.prepare"]);
    assert!(home.world.record(REQUEST_1).is_none());
    // The preview that is offered says what is in the way of it.
    let unregistered = home.prepare(&replace());
    assert_eq!(unregistered.code, ResultCode::RepositoryNotRegistered);
    assert_eq!(recovery_actions(&unregistered), ["index.rebuild"]);

    home.world.rebuild();

    // The pin that was to be replaced is gone.
    let nothing_to_replace = home.prepare(&replace());
    assert_eq!(nothing_to_replace.code, ResultCode::InvalidInput);
    let (_, preview) = home.preview(&approve());
    let PreviewDetailsDto::Host(details) = &preview.details else {
        panic!("the details of a host preview");
    };
    assert_eq!(details.pinned_fingerprint, None);
}

/// A folder that was made is there. Its request, retried once the
/// repository is registered again, is a new request that finds its end
/// state: a no-op that makes nothing.
#[test]
fn a_completed_folder_creation_retried_after_the_cache_is_lost_is_already_applied() {
    git_home::isolated();
    let mut world = World::new();
    let create = |world: &World| {
        Mutation::FolderCreate(FolderCreateInput {
            root: world.root.clone(),
            path: "docs/a/b".to_owned(),
        })
    };
    let made = world.execute(REQUEST_1, create(&world));
    assert_eq!(made.code, ResultCode::Ok, "{}", made.message);
    assert!(world.root.join("docs/a/b").is_dir());
    let head = world.primary_head();

    world.lose_cache();
    let unregistered = world.execute(REQUEST_1, create(&world));
    assert_eq!(unregistered.code, ResultCode::RepositoryNotRegistered);
    assert_eq!(recovery_actions(&unregistered), ["index.rebuild"]);
    assert_eq!(world.request_rows(), 0);

    world.rebuild();
    let retried = world.execute(REQUEST_1, create(&world));
    assert_eq!(
        (retried.outcome, retried.code),
        (Outcome::Noop, ResultCode::AlreadyApplied),
        "{}",
        retried.message
    );
    assert_eq!(retried.effects.discovery, DiscoveryEffect::Current);
    assert!(world.root.join("docs/a/b").is_dir());
    assert_eq!(world.primary_head(), head);
    assert_eq!(world.pending_journal_rows(), 0);
    // Another request for the same folder is the same no-op.
    let other = world.execute(REQUEST_2, create(&world));
    assert_eq!(other.code, ResultCode::AlreadyApplied);
}
