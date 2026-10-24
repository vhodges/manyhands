use std::{collections::HashSet, path::Path};

use manyhands::{
    repository::{
        InitialsChange, OperationId, RelationshipWrite, SharedKeyId,
        keys::{KeyStore, SecretPassphrase},
        transport::SshAuthority,
    },
    results::ResultCode,
};

use super::{Intent, NotBuilt};
use crate::desktop::{services::KeyStoreUnavailable, test_support::fixtures::Fixture};

const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG0";

fn authority() -> SshAuthority {
    SshAuthority {
        host: "git.example.invalid".to_owned(),
        port: 22,
    }
}

/// One intent of every variant, over `root`. The match below fails to
/// compile when a variant is added and not listed here.
fn every_intent(root: &Path) -> Vec<Intent> {
    let root = root.to_owned();
    let intents = vec![
        Intent::RepoCreate {
            root: root.join("new"),
            primary_branch: "main".to_owned(),
            identity: Some(("Ada Example".to_owned(), "ada@example.invalid".to_owned())),
            slug_prefix: None,
        },
        Intent::RepoEnable {
            root: root.clone(),
            primary_branch: "main".to_owned(),
            identity: None,
            slug_prefix: Some("ab".to_owned()),
        },
        Intent::RepoRemove { root: root.clone() },
        Intent::RepoIdentitySet {
            root: root.clone(),
            name: "Ada Example".to_owned(),
            email: "ada@example.invalid".to_owned(),
            initials: InitialsChange::Unchanged,
        },
        Intent::RemoteAdd {
            root: root.clone(),
            name: "origin".to_owned(),
            url: "ssh://git@git.example.invalid/a.git".to_owned(),
        },
        Intent::RemoteRemove {
            root: root.clone(),
            name: "origin".to_owned(),
        },
        Intent::RemoteSelect {
            root: root.clone(),
            name: None,
        },
        Intent::IndexRefresh { root: root.clone() },
        Intent::IndexRebuild { root: root.clone() },
        Intent::KeyGenerate {
            label: "laptop".to_owned(),
            protected: false,
        },
        Intent::KeyImport {
            label: "imported".to_owned(),
            private_key_path: root.join("id_ed25519"),
            public_key_path: None,
        },
        Intent::KeySelect {
            id: SharedKeyId::new(),
        },
        Intent::KeyClear,
        Intent::KeyRemove {
            id: SharedKeyId::new(),
        },
        Intent::KeyDelete {
            id: SharedKeyId::new(),
        },
        Intent::HostApprove {
            root: root.clone(),
            authority: authority(),
            algorithm: "ssh-ed25519".to_owned(),
            fingerprint: "SHA256:new".to_owned(),
        },
        Intent::HostReplace {
            root: root.clone(),
            authority: authority(),
            algorithm: "ssh-ed25519".to_owned(),
            fingerprint: "SHA256:new".to_owned(),
            old_fingerprint: "SHA256:old".to_owned(),
        },
        Intent::OperationResume {
            root: root.clone(),
            operation_id: OperationId::new(),
        },
        Intent::OperationAbandon {
            root: root.clone(),
            operation_id: OperationId::new(),
        },
        Intent::DocumentSave {
            root: root.clone(),
            id: ITEM.parse().unwrap(),
            title: "Title".to_owned(),
            body: "Body".to_owned(),
            observation: Some("observation".to_owned()),
        },
        Intent::TicketSave {
            root,
            id: ITEM.parse().unwrap(),
            title: "Title".to_owned(),
            ticket_type: "task".to_owned(),
            status: "open".to_owned(),
            project: None,
            team: None,
            body: "Body".to_owned(),
            deps: RelationshipWrite::Unchanged,
            parent: RelationshipWrite::Unchanged,
            observation: "observation".to_owned(),
        },
    ];
    let listed: HashSet<usize> = intents.iter().map(variant).collect();
    assert_eq!(listed.len(), 21, "one intent of every variant");
    intents
}

fn variant(intent: &Intent) -> usize {
    match intent {
        Intent::RepoCreate { .. } => 0,
        Intent::RepoEnable { .. } => 1,
        Intent::RepoRemove { .. } => 2,
        Intent::RepoIdentitySet { .. } => 3,
        Intent::RemoteAdd { .. } => 4,
        Intent::RemoteRemove { .. } => 5,
        Intent::RemoteSelect { .. } => 6,
        Intent::IndexRefresh { .. } => 7,
        Intent::IndexRebuild { .. } => 8,
        Intent::KeyGenerate { .. } => 9,
        Intent::KeyImport { .. } => 10,
        Intent::KeySelect { .. } => 11,
        Intent::KeyClear => 12,
        Intent::KeyRemove { .. } => 13,
        Intent::KeyDelete { .. } => 14,
        Intent::HostApprove { .. } => 15,
        Intent::HostReplace { .. } => 16,
        Intent::OperationResume { .. } => 17,
        Intent::OperationAbandon { .. } => 18,
        Intent::DocumentSave { .. } => 19,
        Intent::TicketSave { .. } => 20,
    }
}

/// A confirmed command previews (or fails its checks); every other
/// command is `not_confirmable`. `prepare` decides that from the command
/// alone, so the inputs need not be valid.
#[test]
fn confirmed_agrees_with_prepare_for_every_intent() {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    let store = KeyStore::for_home(fixture.home()).unwrap();
    for intent in every_intent(&root) {
        let mutation = intent.build(None, || Ok(store.clone())).ok().unwrap();
        let prepared = fixture.service().prepare(&mutation);
        assert_eq!(
            prepared.code != ResultCode::NotConfirmable,
            intent.confirmed(),
            "{} ({})",
            intent.label(),
            mutation.command(),
        );
    }
}

/// Each intent builds the library command of the same name, and runs as
/// its own job kind.
#[test]
fn every_intent_builds_its_own_command() {
    let fixture = Fixture::new();
    let store = KeyStore::for_home(fixture.home()).unwrap();
    let expected = [
        "repo create",
        "repo enable",
        "repo remove",
        "repo identity-set",
        "remote add",
        "remote remove",
        "remote select",
        "index refresh",
        "index rebuild",
        "key generate",
        "key import",
        "key select",
        "key clear",
        "key remove",
        "key delete",
        "host approve",
        "host replace",
        "operation resume",
        "operation abandon",
        "document save",
        "ticket save",
    ];
    let intents = every_intent(fixture.home());
    let mut kinds = HashSet::new();
    for (intent, command) in intents.iter().zip(expected) {
        let mutation = intent.build(None, || Ok(store.clone())).ok().unwrap();
        assert_eq!(mutation.command(), command);
        assert!(
            kinds.insert(intent.job_kind()),
            "{command}: a kind of its own"
        );
        assert!(!intent.label().is_empty());
    }
}

#[test]
fn a_protected_key_generation_without_a_secret_is_not_built() {
    let intent = Intent::KeyGenerate {
        label: "laptop".to_owned(),
        protected: true,
    };
    let built = intent.build(None, || panic!("no store is asked for"));
    assert!(matches!(built, Err(NotBuilt::MissingSecret)));

    let fixture = Fixture::new();
    let store = KeyStore::for_home(fixture.home()).unwrap();
    let secret = SecretPassphrase::new("correct horse".to_owned()).unwrap();
    let built = intent.build(Some(secret), || Ok(store.clone()));
    assert!(built.is_ok());
}

#[test]
fn a_key_command_without_a_key_store_is_not_built() {
    let intent = Intent::KeyDelete {
        id: SharedKeyId::new(),
    };
    let built = intent.build(None, || Err(KeyStoreUnavailable));
    assert!(matches!(built, Err(NotBuilt::KeyStoreUnavailable)));

    // A command that needs no store never asks for one.
    let intent = Intent::KeyClear;
    assert!(intent.build(None, || panic!("no store")).is_ok());
}

/// Ruling R-B2: only the intents that can change items make open tabs
/// read again. Every variant is named.
#[test]
fn changes_items_names_the_intents_that_can_change_items() {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    for intent in every_intent(&root) {
        let expected = match variant(&intent) {
            // RepoCreate, RepoEnable, IndexRefresh, IndexRebuild,
            // OperationResume, OperationAbandon, DocumentSave, TicketSave.
            0 | 1 | 7 | 8 | 17..=20 => true,
            // RepoRemove closes the tabs instead; the rest change
            // settings, keys or hosts, never an item.
            2..=6 | 9..=16 => false,
            other => unreachable!("variant {other}"),
        };
        assert_eq!(intent.changes_items(), expected, "{}", intent.label());
    }
}
