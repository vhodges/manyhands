//! Which poll a published setup takes, and what its answers come to for the
//! schedule (Wave 06 Cycle 02, Part J). The library's own tests pin the
//! outcomes; these pin the desktop's reading of them.

use manyhands::{
    canonical::ItemId,
    forge::PullRequestLink,
    repository::{
        Accessibility, ConfigurationDto, ConfigurationState, IdentityAvailability, OperationId,
        PullRequestDto, RemoteDto, RepositoryInspectionDto, RepositorySetupDto, SetupPollError,
        SetupPollOutcome, SetupPollReport,
        transport::{SshDirection, SshTransportError, SshTransportErrorKind},
    },
};

use super::*;
use crate::desktop::model::polling::{Answer, CancelCause, Disposition, TransportFailure};

const ULID: &str = "01JD0000000000000000000001";

fn setup(kind: &str, state: &str, remote: &str) -> RepositorySetupDto {
    RepositorySetupDto {
        id: ULID.to_owned(),
        kind: kind.to_owned(),
        state: state.to_owned(),
        request_id: "01JD0000000000000000000002".to_owned(),
        branch: format!("manyhands/setup/{ULID}"),
        remote: remote.to_owned(),
        integration: "pull_request".to_owned(),
        pull_request: PullRequestDto {
            source: format!("manyhands/setup/{ULID}"),
            target: "main".to_owned(),
            forge: None,
            url: None,
        },
    }
}

fn remote(name: &str, selected: bool) -> RemoteDto {
    RemoteDto {
        name: name.to_owned(),
        fetch_location: "ssh://git@example.invalid/team/repo.git".to_owned(),
        push_location: "ssh://git@example.invalid/team/repo.git".to_owned(),
        publication_eligible: true,
        selected_for_publication: selected,
    }
}

fn inspection(
    setup: Option<RepositorySetupDto>,
    remotes: Vec<RemoteDto>,
) -> RepositoryInspectionDto {
    let _ = Accessibility::Accessible;
    RepositoryInspectionDto {
        selected_path: "/work/alpha".to_owned(),
        root: "/work/alpha".to_owned(),
        registered: true,
        head_branch: Some("main".to_owned()),
        local_branches: vec!["main".to_owned()],
        configuration: ConfigurationDto {
            state: ConfigurationState::Valid,
            primary_branch: Some("main".to_owned()),
            publication_remote: None,
            problems: Vec::new(),
            integration: None,
            forge: None,
        },
        setup,
        identity_state: IdentityAvailability::Available,
        remotes,
        operations: Vec::new(),
    }
}

/// Only a published setup is polled; a first-time one replaces today's
/// poll, a change follows it unless it is on the remote today's poll
/// already checks.
#[test]
fn a_published_setup_takes_the_poll_its_kind_and_remote_say() {
    use SetupPollPath::{After, Replace};
    let on = |path, remote: &str, current_ssh| {
        Some(PendingSetup {
            path,
            remote: remote.to_owned(),
            current_ssh,
        })
    };
    let not_ssh = || RemoteDto {
        publication_eligible: false,
        ..remote("origin", true)
    };
    let cases = [
        ("no setup", inspection(None, Vec::new()), None),
        (
            "first-time, recorded",
            inspection(Some(setup("enable", "recorded", "origin")), Vec::new()),
            None,
        ),
        (
            "first-time, published",
            inspection(
                Some(setup("enable", "published", "origin")),
                vec![remote("origin", false)],
            ),
            on(Replace, "origin", false),
        ),
        (
            "change, recorded",
            inspection(
                Some(setup("change", "recorded", "fork")),
                vec![remote("origin", true), remote("fork", false)],
            ),
            None,
        ),
        (
            "change on the current remote",
            inspection(
                Some(setup("change", "published", "origin")),
                vec![remote("origin", true), remote("fork", false)],
            ),
            None,
        ),
        (
            "change on another remote",
            inspection(
                Some(setup("change", "published", "fork")),
                vec![remote("origin", true), remote("fork", false)],
            ),
            on(After, "fork", true),
        ),
        (
            "change, current remote not SSH",
            inspection(
                Some(setup("change", "published", "fork")),
                vec![not_ssh(), remote("fork", false)],
            ),
            on(After, "fork", false),
        ),
        (
            "change with no current remote",
            inspection(
                Some(setup("change", "published", "fork")),
                vec![remote("fork", false)],
            ),
            on(After, "fork", false),
        ),
    ];
    for (name, inspected, expected) in cases {
        assert_eq!(pending_setup_of(&inspected), expected, "{name}");
    }
}

#[allow(clippy::result_large_err)] // The library's `SetupPollError`.
fn report(outcome: SetupPollOutcome) -> Result<SetupPollReport, SetupPollError> {
    Ok(SetupPollReport {
        setup_id: ULID.parse::<ItemId>().expect("an item ID"),
        outcome,
        integration: None,
        differs: false,
        link: PullRequestLink {
            source: "manyhands/setup/x".to_owned(),
            target: "main".to_owned(),
            forge: None,
            url: None,
        },
    })
}

#[allow(clippy::result_large_err)] // The library's `SetupPollError`.
fn unlock_required() -> Result<SetupPollReport, SetupPollError> {
    Err(SetupPollError::Transport(SshTransportError {
        root: "/work/alpha".into(),
        remote_name: "origin".to_owned(),
        direction: SshDirection::Fetch,
        selected_key_id: None,
        authority: None,
        kind: SshTransportErrorKind::UnlockCancelled,
    }))
}

/// Every outcome the library answers is a completed check; an activation
/// alone brings the primary in, so the repository is read again; every stop
/// is the answer the ordinary poll's stop of that kind is.
#[test]
fn a_setup_polls_answer_reads_as_a_check_of_the_remote() {
    let main_brought_in = vec![AttemptTarget {
        item_id: None,
        kind: None,
        disposition: Disposition::FastForwarded,
        main: true,
    }];
    let cases: Vec<(&str, Result<SetupPollReport, SetupPollError>, Answer, bool)> = vec![
        (
            "activated",
            report(SetupPollOutcome::Activated),
            Answer::Completed,
            true,
        ),
        (
            "waiting for merge",
            report(SetupPollOutcome::WaitingForMerge),
            Answer::Completed,
            false,
        ),
        (
            "waiting for checkout",
            report(SetupPollOutcome::WaitingForCheckout),
            Answer::Completed,
            false,
        ),
        (
            "branch deleted",
            report(SetupPollOutcome::BranchDeleted),
            Answer::Completed,
            false,
        ),
        ("gone", Err(SetupPollError::Gone), Answer::Completed, false),
        ("busy", Err(SetupPollError::Busy), Answer::Busy, false),
        (
            "cancelled",
            Err(SetupPollError::Cancelled),
            Answer::Cancelled(CancelCause::Quit),
            false,
        ),
        (
            "unreadable",
            Err(SetupPollError::RecoveryRequired),
            Answer::Repository {
                recovery_required: true,
            },
            false,
        ),
        (
            "key locked",
            unlock_required(),
            Answer::Transport(TransportFailure::KeyLocked { key: None }),
            false,
        ),
    ];
    for (name, result, answer, brought_in) in cases {
        let attempt = setup_attempt(
            &"/work/alpha".to_owned(),
            OperationId::new(),
            &result,
            || CancelCause::Quit,
        );
        assert_eq!(attempt.answer, answer, "{name}: answer");
        assert_eq!(
            attempt.targets,
            if brought_in {
                main_brought_in.clone()
            } else {
                Vec::new()
            },
            "{name}: targets"
        );
    }
}
