use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use manyhands::{
    repository::{
        AuthoringKind, ContextPrepareIntent, InitialsChange, Mutation, OperationId,
        RelationshipWrite, SharedKeyId,
        keys::{KeyStore, SecretPassphrase},
        transport::SshAuthority,
    },
    results::ResultCode,
};

use super::{CommentBody, Intent, NotBuilt};
use crate::desktop::{
    services::{JobKind, KeyStoreUnavailable},
    test_support::fixtures::Fixture,
};

const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG0";
const COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG1";
const PARENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG2";

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
            root: root.clone(),
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
        Intent::ContextPrepare {
            root: root.clone(),
            kind: AuthoringKind::Document,
            id: ITEM.parse().unwrap(),
            intent: ContextPrepareIntent::Edit,
        },
        Intent::ContextRelease {
            root: root.clone(),
            kind: AuthoringKind::Ticket,
            id: ITEM.parse().unwrap(),
        },
        Intent::DocumentCreate {
            root: root.clone(),
            id: ITEM.parse().unwrap(),
            path: PathBuf::from("docs/new.md"),
            title: "Title".to_owned(),
            body: "Body".to_owned(),
        },
        Intent::TicketCreate {
            root: root.clone(),
            id: ITEM.parse().unwrap(),
            title: "Title".to_owned(),
            ticket_type: "task".to_owned(),
            status: "open".to_owned(),
            project: None,
            team: None,
            body: "Body".to_owned(),
            deps: Vec::new(),
            parent: None,
        },
        Intent::DocumentMove {
            root: root.clone(),
            id: ITEM.parse().unwrap(),
            observation: Some("observation".to_owned()),
            destination_path: PathBuf::from("docs/moved.md"),
            destination_observation: "absent".to_owned(),
        },
        Intent::FolderCreate {
            root: root.clone(),
            path: "docs/folder".to_owned(),
        },
        Intent::DocumentRepair {
            root: root.clone(),
            path: PathBuf::from("docs/broken.md"),
            observation: "observation".to_owned(),
            source: "source".to_owned(),
        },
        Intent::TicketRepair {
            root: root.clone(),
            path: PathBuf::from(".manyhands/tickets/x/ticket.md"),
            observation: "observation".to_owned(),
            source: "source".to_owned(),
        },
        Intent::CommentAdd {
            root: root.clone(),
            kind: AuthoringKind::Ticket,
            id: ITEM.parse().unwrap(),
            comment_id: COMMENT.parse().unwrap(),
            parent_id: None,
            body: CommentBody::new("Body".to_owned()),
        },
        Intent::ItemSync {
            root: root.clone(),
            kind: AuthoringKind::Document,
            id: ITEM.parse().unwrap(),
        },
        Intent::RepoSync { root: root.clone() },
        Intent::DocumentPromote {
            root: root.clone(),
            id: ITEM.parse().unwrap(),
        },
        Intent::TicketClose {
            root,
            id: ITEM.parse().unwrap(),
        },
    ];
    let listed: HashSet<usize> = intents.iter().map(variant).collect();
    assert_eq!(listed.len(), 34, "one intent of every variant");
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
        Intent::ContextPrepare { .. } => 21,
        Intent::ContextRelease { .. } => 22,
        Intent::DocumentCreate { .. } => 23,
        Intent::TicketCreate { .. } => 24,
        Intent::DocumentMove { .. } => 25,
        Intent::FolderCreate { .. } => 26,
        Intent::DocumentRepair { .. } => 27,
        Intent::TicketRepair { .. } => 28,
        Intent::CommentAdd { .. } => 29,
        Intent::ItemSync { .. } => 30,
        Intent::RepoSync { .. } => 31,
        Intent::DocumentPromote { .. } => 32,
        Intent::TicketClose { .. } => 33,
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
        "context prepare",
        "context release",
        "document create",
        "ticket create",
        "document move",
        "folder create",
        "document repair",
        "ticket repair",
        "comment add",
        "item sync",
        "repo sync",
        "document promote",
        "ticket close",
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
            // OperationResume, OperationAbandon, DocumentSave, TicketSave,
            // the eight Part C commands, and the comment and the two syncs.
            0 | 1 | 7 | 8 | 17..=33 => true,
            // RepoRemove closes the tabs instead; the rest change
            // settings, keys or hosts, never an item.
            2..=6 | 9..=16 => false,
            other => unreachable!("variant {other}"),
        };
        assert_eq!(intent.changes_items(), expected, "{}", intent.label());
    }
}

/// A comment is built with the IDs and text it was asked with, and no
/// author: the library takes the Git identity.
#[test]
fn comment_add_builds_its_mutation_with_no_author() {
    let root = PathBuf::from("/repo");
    let intent = Intent::CommentAdd {
        root: root.clone(),
        kind: AuthoringKind::Ticket,
        id: ITEM.parse().unwrap(),
        comment_id: COMMENT.parse().unwrap(),
        parent_id: Some(PARENT.parse().unwrap()),
        body: CommentBody::new("A reply.".to_owned()),
    };
    let Ok(Mutation::CommentAdd(input)) = intent.build(None, || panic!("no store")) else {
        panic!("a comment builds `comment add`");
    };
    assert_eq!(input.root, root);
    assert_eq!(input.kind, AuthoringKind::Ticket);
    assert_eq!(input.id.to_string(), ITEM);
    assert_eq!(input.comment_id.to_string(), COMMENT);
    assert_eq!(
        input.parent_id.map(|id| id.to_string()).as_deref(),
        Some(PARENT)
    );
    assert_eq!(input.body, "A reply.");
    assert_eq!(input.created_by, None);
}

#[test]
fn item_and_repo_sync_build_their_mutations() {
    let root = PathBuf::from("/repo");
    let intent = Intent::ItemSync {
        root: root.clone(),
        kind: AuthoringKind::Document,
        id: ITEM.parse().unwrap(),
    };
    let Ok(Mutation::ItemSync(input)) = intent.build(None, || panic!("no store")) else {
        panic!("an item sync builds `item sync`");
    };
    assert_eq!(input.root, root);
    assert_eq!(input.kind, AuthoringKind::Document);
    assert_eq!(input.id.to_string(), ITEM);

    let intent = Intent::RepoSync { root: root.clone() };
    let Ok(Mutation::RepoSync(input)) = intent.build(None, || panic!("no store")) else {
        panic!("a repository sync builds `repo sync`");
    };
    assert_eq!(input.root, root);
}

/// The three new labels are fixed text: no title or comment text is put
/// in them.
#[test]
fn labels_are_fixed_text() {
    let intents = every_intent(Path::new("/repo"));
    let label = |index: usize| {
        intents
            .iter()
            .find(|intent| variant(intent) == index)
            .map(Intent::label)
            .unwrap()
    };
    assert_eq!(label(29), "Post comment");
    assert_eq!(label(30), "Share item");
    assert_eq!(label(31), "Share main");
}

/// Only the commands the library stops at a synchronization's safe points
/// can be stopped after acceptance (fact 9). Every variant is named.
#[test]
fn syncs_comments_and_resume_stop_at_safe_points_and_saves_do_not() {
    for intent in every_intent(Path::new("/repo")) {
        let expected = match variant(&intent) {
            // OperationResume, CommentAdd, ItemSync, RepoSync,
            // DocumentPromote, TicketClose.
            17 | 29..=33 => true,
            // Every local command, the saves among them.
            0..=16 | 18..=28 => false,
            other => unreachable!("variant {other}"),
        };
        assert_eq!(
            intent.stops_at_safe_points(),
            expected,
            "{}",
            intent.label()
        );
    }
}

/// Approve and merge… builds `document promote` of the document it was
/// asked for, confirmed, stopping at safe points, and changing items.
#[test]
fn document_promote_builds_its_mutation() {
    let root = PathBuf::from("/repo");
    let intent = Intent::DocumentPromote {
        root: root.clone(),
        id: ITEM.parse().unwrap(),
    };
    let Ok(Mutation::DocumentPromote(input)) = intent.build(None, || panic!("no store")) else {
        panic!("an approval builds `document promote`");
    };
    assert_eq!(input.root, root);
    assert_eq!(input.id.to_string(), ITEM);
    assert_eq!(intent.label(), "Approve and merge");
    assert_eq!(intent.job_kind(), JobKind::DocumentPromote);
    assert_eq!(intent.root(), Some(root.as_path()));
    assert!(intent.confirmed());
    assert!(intent.changes_items());
    assert!(intent.stops_at_safe_points());
}

/// Close ticket… builds `ticket close` of the ticket it was asked for,
/// confirmed, stopping at safe points, and changing items.
#[test]
fn ticket_close_builds_its_mutation() {
    let root = PathBuf::from("/repo");
    let intent = Intent::TicketClose {
        root: root.clone(),
        id: ITEM.parse().unwrap(),
    };
    let Ok(Mutation::TicketClose(input)) = intent.build(None, || panic!("no store")) else {
        panic!("a close builds `ticket close`");
    };
    assert_eq!(input.root, root);
    assert_eq!(input.id.to_string(), ITEM);
    assert_eq!(intent.label(), "Close ticket");
    assert_eq!(intent.job_kind(), JobKind::TicketClose);
    assert_eq!(intent.root(), Some(root.as_path()));
    assert!(intent.confirmed());
    assert!(intent.changes_items());
    assert!(intent.stops_at_safe_points());
}
