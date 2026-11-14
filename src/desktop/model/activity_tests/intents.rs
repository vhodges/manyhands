//! Fixtures for Activity's model tests that build library values: a
//! request of every intent, a share's answer by its outcome, a lifecycle's
//! answer, and the check that no row holds what Part G bars.

use std::path::PathBuf;

use manyhands::{
    repository::{
        AuthoringKind, ContextPrepareIntent, InitialsChange, MutationDataDto, OperationId,
        RelationshipWrite, SharedKeyId,
    },
    results::{Effects, Envelope, Outcome, ResultCode, Scope},
};

use super::support::{DRAFT_TITLE, PRIMARY, ROOT, answer, authority, item, root};
use crate::desktop::{
    model::activity::rows::Derived,
    services::requests::{CommentBody, Intent},
};

/// The intent a name of the copy table stands for, over `ROOT` and `ITEM`.
pub(super) fn intent(name: &str) -> Intent {
    let root = root();
    let key = SharedKeyId::new();
    let identity = Some(("Ada Example".to_owned(), "ada@example.invalid".to_owned()));
    let operation_id = OperationId::new();
    let resolutions = Vec::new();
    let observation = "observation".to_owned();
    let id = item();
    let (kind, document) = (AuthoringKind::Ticket, AuthoringKind::Document);
    match name {
        "RepoCreate" => Intent::RepoCreate {
            root,
            primary_branch: PRIMARY.to_owned(),
            identity,
            slug_prefix: None,
        },
        "RepoEnable" => Intent::RepoEnable {
            root,
            primary_branch: PRIMARY.to_owned(),
            identity: None,
            slug_prefix: None,
        },
        "RepoRemove" => Intent::RepoRemove { root },
        "RepoIdentitySet" => Intent::RepoIdentitySet {
            root,
            name: "Ada Example".to_owned(),
            email: "ada@example.invalid".to_owned(),
            initials: InitialsChange::Unchanged,
        },
        "RemoteAdd" => Intent::RemoteAdd {
            root,
            name: "origin".to_owned(),
            url: "ssh://user:secret@git.example.invalid/path.git".to_owned(),
        },
        "RemoteRemove" => Intent::RemoteRemove {
            root,
            name: "origin".to_owned(),
        },
        "RemoteSelect" => Intent::RemoteSelect {
            root,
            name: Some("origin".to_owned()),
        },
        "RemoteSelectNone" => Intent::RemoteSelect { root, name: None },
        "IndexRefresh" => Intent::IndexRefresh { root },
        "IndexRebuild" => Intent::IndexRebuild { root },
        "PollOnce" => Intent::PollOnce { root },
        "PollConfigure" => Intent::PollConfigure {
            root,
            interval_seconds: 600,
        },
        "PollPause" => Intent::PollPause { root },
        "PollResume" => Intent::PollResume { root },
        "RepoSync" => Intent::RepoSync { root },
        "FolderCreate" => Intent::FolderCreate {
            root,
            path: "docs/folder".to_owned(),
        },
        "KeyGenerate" => Intent::KeyGenerate {
            label: "laptop".to_owned(),
            protected: false,
        },
        "KeyImport" => Intent::KeyImport {
            label: "imported".to_owned(),
            private_key_path: PathBuf::from("/home/ada/.ssh/id_ed25519"),
            public_key_path: None,
        },
        "KeySelect" => Intent::KeySelect { id: key },
        "KeyClear" => Intent::KeyClear,
        "KeyRemove" => Intent::KeyRemove { id: key },
        "KeyDelete" => Intent::KeyDelete { id: key },
        "HostApprove" => Intent::HostApprove {
            root,
            authority: authority(),
            algorithm: "ssh-ed25519".to_owned(),
            fingerprint: "SHA256:new".to_owned(),
        },
        "HostReplace" => Intent::HostReplace {
            root,
            authority: authority(),
            algorithm: "ssh-ed25519".to_owned(),
            fingerprint: "SHA256:new".to_owned(),
            old_fingerprint: "SHA256:old".to_owned(),
        },
        "OperationResume" => Intent::OperationResume { root, operation_id },
        "OperationAbandon" => Intent::OperationAbandon { root, operation_id },
        "ConflictResolve" => Intent::ConflictResolve {
            root,
            operation_id,
            observation,
            resolutions,
        },
        "LocalContextResolve" => Intent::LocalContextResolve {
            root,
            operation_id,
            observation,
            resolutions,
        },
        "DocumentSave" => Intent::DocumentSave {
            root,
            id,
            title: DRAFT_TITLE.to_owned(),
            body: "Body".to_owned(),
            observation: None,
        },
        "TicketSave" => Intent::TicketSave {
            root,
            id,
            title: DRAFT_TITLE.to_owned(),
            ticket_type: "task".to_owned(),
            status: "open".to_owned(),
            project: None,
            team: None,
            body: "Body".to_owned(),
            deps: RelationshipWrite::Unchanged,
            parent: RelationshipWrite::Unchanged,
            observation,
        },
        "DocumentCreate" => Intent::DocumentCreate {
            root,
            id,
            path: PathBuf::from("docs/new.md"),
            title: DRAFT_TITLE.to_owned(),
            body: "Body".to_owned(),
        },
        "TicketCreate" => Intent::TicketCreate {
            root,
            id,
            title: DRAFT_TITLE.to_owned(),
            ticket_type: "task".to_owned(),
            status: "open".to_owned(),
            project: None,
            team: None,
            body: "Body".to_owned(),
            deps: Vec::new(),
            parent: None,
        },
        "DocumentMove" => Intent::DocumentMove {
            root,
            id,
            observation: None,
            destination_path: PathBuf::from("docs/moved.md"),
            destination_observation: "absent".to_owned(),
        },
        "DocumentPromote" => Intent::DocumentPromote { root, id },
        "TicketClose" => Intent::TicketClose { root, id },
        "ContextPrepare" => Intent::ContextPrepare {
            root,
            kind: document,
            id,
            intent: ContextPrepareIntent::Edit,
        },
        "ContextRelease" => Intent::ContextRelease { root, kind, id },
        "ContextRetire" => Intent::ContextRetire { root, kind, id },
        "CommentAdd" => Intent::CommentAdd {
            root,
            kind,
            id,
            comment_id: "01ARZ3NDEKTSV4RRFFQ69G5FG1".parse().unwrap(),
            parent_id: None,
            body: CommentBody::new("A secret comment".to_owned()),
        },
        "ItemSync" => Intent::ItemSync { root, kind, id },
        "ItemRepublish" => Intent::ItemRepublish { root, kind, id },
        "LocalContextSync" => Intent::LocalContextSync {
            root,
            kind,
            id,
            observation,
        },
        "DocumentRepair" => Intent::DocumentRepair {
            root,
            path: PathBuf::from("docs/broken.md"),
            observation,
            source: "source".to_owned(),
        },
        "TicketRepair" => Intent::TicketRepair {
            root,
            path: PathBuf::from(".manyhands/tickets/x/ticket.md"),
            observation,
            source: "source".to_owned(),
        },
        other => panic!("no intent named {other}"),
    }
}

/// The answer to a share that classifies as `outcome`, a `SyncOutcome`'s
/// name (B3), with the recovery its host and history outcomes name.
pub(super) fn share_answer(outcome: &str) -> Envelope<MutationDataDto> {
    use manyhands::{
        repository::{
            ConflictNextAction, ConflictNextDto, SynchronizationConflictMutationDto,
            SynchronizationConflictType,
        },
        results::{CheckpointEffect, PublicationEffect, RecoveryAction, RecoveryActionKind},
    };
    use serde_json::json;
    let classified = |cancelled, code, effects| {
        Envelope::classified_mutation("fixture", Scope::default(), cancelled, code, effects, false)
    };
    let plain = |code| classified(false, code, Effects::not_requested());
    let published = |publication| Effects {
        publication,
        ..Effects::not_requested()
    };
    match outcome {
        "Published" => classified(
            false,
            ResultCode::Ok,
            published(PublicationEffect::Published),
        ),
        "Current" => classified(false, ResultCode::Ok, published(PublicationEffect::Current)),
        "Stopped" => plain_cancelled(),
        "StoppedAfterPart" => classified(
            true,
            ResultCode::Cancelled,
            Effects {
                checkpoint: CheckpointEffect::Committed,
                ..Effects::not_requested()
            },
        ),
        "Conflict" => plain(ResultCode::RecoveryRequired).with_data(
            MutationDataDto::SynchronizationConflict(SynchronizationConflictMutationDto {
                r#type: SynchronizationConflictType::SynchronizationConflict,
                conflict: ConflictNextDto {
                    operation_id: OperationId::new().to_string(),
                    next_action: ConflictNextAction::Resolve,
                },
            }),
        ),
        "BranchDeleted" => plain(ResultCode::RemoteBranchDeleted),
        "HistoryUnresolved" => {
            plain(ResultCode::RecoveryRequired).with_recovery(vec![RecoveryAction::new(
                RecoveryActionKind::RemoteHistory,
                [("root", json!(ROOT))],
            )])
        }
        "NeedsUnlock" => plain(ResultCode::UnlockRequired),
        "NeedsKey" => plain(ResultCode::NoSelectedKey),
        "NeedsHost" => {
            plain(ResultCode::HostApprovalRequired).with_recovery(vec![RecoveryAction::new(
                RecoveryActionKind::HostApprove,
                [
                    ("authority", json!("git.example.invalid:22")),
                    ("root", json!(ROOT)),
                ],
            )])
        }
        "NeedsHostUnnamed" => plain(ResultCode::HostApprovalRequired),
        "NeedsIdentity" => plain(ResultCode::IdentityRequired),
        "Busy" => plain(ResultCode::Busy),
        "NoRemote" => plain(ResultCode::PublicationRemoteRequired),
        "NothingToShare" => plain(ResultCode::ItemNotFound),
        "Failed" => plain(ResultCode::InternalError),
        other => panic!("no share answer named {other}"),
    }
}

fn plain_cancelled() -> Envelope<MutationDataDto> {
    Envelope::classified_mutation(
        "fixture",
        Scope::default(),
        true,
        ResultCode::Cancelled,
        Effects::not_requested(),
        false,
    )
}

/// A lifecycle answer of `outcome` that merged, shared and tidied up, as
/// its effects say (no lifecycle data: `Reach::of` reads the effects).
pub(super) fn lifecycle_answer(outcome: Outcome) -> Envelope<MutationDataDto> {
    use manyhands::results::{CleanupEffect, IntegrationEffect, PublicationEffect};
    let code = match outcome {
        Outcome::Noop => ResultCode::AlreadyApplied,
        _ => ResultCode::Ok,
    };
    answer(outcome, code).with_effects(Effects {
        publication: PublicationEffect::Published,
        integration: IntegrationEffect::Complete,
        cleanup: CleanupEffect::Complete,
        ..Effects::not_requested()
    })
}

/// What a row must never hold (Part G): a draft's own title, a path, a
/// comment's text, a remote's address or its user-info.
pub(super) fn assert_private(row: &Derived) {
    let mut seen = vec![row.title.clone()];
    seen.extend(row.description.clone());
    seen.extend(row.detail.clone());
    seen.extend(row.menu.iter().map(|offer| offer.label.clone()));
    for text in seen {
        for secret in [
            DRAFT_TITLE,
            "A secret comment",
            "/fixture",
            "docs/",
            ".manyhands",
            "secret@",
            "user:",
            "git.example.invalid/path",
        ] {
            assert!(!text.contains(secret), "{text:?} holds {secret:?}");
        }
        assert!(!text.contains('!'), "{text:?} has an exclamation mark");
    }
    assert!(
        !row.title.ends_with('.'),
        "{:?} ends with a full stop",
        row.title
    );
    if let Some(description) = &row.description {
        assert!(
            description.ends_with('.') || description.ends_with('…'),
            "{description:?}"
        );
    }
}
