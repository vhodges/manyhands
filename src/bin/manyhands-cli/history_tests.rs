//! The history verbs' arguments and human lines, on built data.

use super::*;
use crate::render::test_support::{stderr, stdout};
use manyhands::{
    repository::{
        HistoryMarkerState, HistoryRelation, ItemDtoKind, ItemResolveHistoryMutationDto,
        PollTargetDisposition, RemoteHistoryChoicesDto,
    },
    results::Effects,
};

const ROOT: &str = "/work/repo";
const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
const OTHER: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAW";

fn entry(item: &str) -> RemoteHistoryEntryDto {
    RemoteHistoryEntryDto {
        kind: ItemDtoKind::Ticket,
        item_id: item.into(),
        branch: format!("manyhands/ticket/{item}"),
        local_oid: Some("a".repeat(40)),
        tracking_ref_exists: false,
        remote_oid: None,
        relation: None,
        ahead: None,
        behind: None,
        resolved: false,
        disposition: Some(PollTargetDisposition::HistoryUnknown),
        choices: RemoteHistoryChoicesDto {
            publish: true,
            release: true,
        },
    }
}

fn data(entries: Vec<RemoteHistoryEntryDto>, cache_loss: bool) -> RemoteHistoryDto {
    RemoteHistoryDto {
        cache_loss,
        generation: 3,
        entries,
    }
}

#[test]
fn the_report_lists_each_context_and_the_commands_that_resolve_it() {
    let present = RemoteHistoryEntryDto {
        remote_oid: Some("b".repeat(40)),
        relation: Some(HistoryRelation::Diverged),
        ahead: Some(1),
        behind: Some(2),
        resolved: true,
        disposition: Some(PollTargetDisposition::Diverged),
        choices: RemoteHistoryChoicesDto {
            publish: false,
            release: false,
        },
        ..entry(OTHER)
    };
    // (case, data, lines that must be there, lines that must not)
    let cases = [
        (
            "absent and unresolved",
            data(vec![entry(ITEM)], false),
            vec![
                format!(
                    "{ITEM}  ticket  manyhands/ticket/{ITEM}  absent  local-only  history_unknown  unresolved  choices=publish (release: desktop only)\n"
                ),
                "Cache loss: no\n".to_owned(),
                format!(
                    "next: manyhands-cli item resolve-history --repo '{ROOT}' --id {ITEM} --as publish --prepare\n"
                ),
            ],
            vec!["remote resolve-history"],
        ),
        (
            "present and resolved",
            data(vec![present.clone()], false),
            vec![format!(
                "{OTHER}  ticket  manyhands/ticket/{OTHER}  present  diverged ahead=1 behind=2  diverged  resolved\n"
            )],
            vec!["next:", "choices="],
        ),
        (
            "cache loss with everything resolved",
            data(vec![present], true),
            vec![
                "Cache loss: yes\n".to_owned(),
                format!("next: manyhands-cli remote resolve-history --repo '{ROOT}' --prepare\n"),
            ],
            vec!["item resolve-history"],
        ),
        (
            "cache loss with something unresolved",
            data(vec![entry(ITEM)], true),
            vec!["Cache loss: yes\n".to_owned()],
            vec!["remote resolve-history"],
        ),
        (
            "no contexts",
            data(vec![], false),
            vec!["Contexts: none\nCache loss: no\n".to_owned()],
            vec!["next:"],
        ),
    ];
    for (name, data, present, absent) in cases {
        let human = report(ROOT, &data);
        for line in &present {
            assert!(
                human.stdout.contains(line),
                "{name}: {line:?} in\n{}",
                human.stdout
            );
        }
        for line in absent {
            assert!(
                !human.stdout.contains(line),
                "{name}: {line:?} in\n{}",
                human.stdout
            );
        }
    }
}

#[test]
fn a_choice_with_no_command_is_a_note_not_a_next_line() {
    let human = report(ROOT, &data(vec![entry(ITEM)], false));
    assert!(
        human
            .stdout
            .lines()
            .filter(|line| line.starts_with("next:"))
            .all(|line| !line.contains("release")),
        "{}",
        human.stdout
    );
    assert_eq!(human.notes, [RELEASE_NOTE]);
}

#[test]
fn each_way_out_is_a_runnable_command_or_a_note() {
    let item_id = ITEM.to_owned();
    // (step, the next: line, whether a note comes with it)
    let cases = [
        (
            HistoryWayOut::Sync {
                item_id: item_id.clone(),
            },
            Some(format!(
                "manyhands-cli item sync --repo '{ROOT}' --id {ITEM}"
            )),
            false,
        ),
        (
            HistoryWayOut::Republish {
                item_id: item_id.clone(),
            },
            Some(format!(
                "manyhands-cli item republish --repo '{ROOT}' --id {ITEM} --prepare"
            )),
            false,
        ),
        (
            HistoryWayOut::RemoteHistory,
            Some(format!("manyhands-cli remote history --repo '{ROOT}'")),
            true,
        ),
        // The CLI has no verb that releases a context.
        (HistoryWayOut::ReleaseContext { item_id }, None, true),
    ];
    for (step, next, noted) in cases {
        let (line, note) = way_out(&step, ROOT);
        assert_eq!(line, next, "{step:?}");
        assert_eq!(note.is_some(), noted, "{step:?}");
    }
}

fn preview_noop(code: ResultCode) -> Envelope<PrepareDataDto> {
    let scope = Scope {
        repository: Some(ROOT.into()),
        item_id: Some(ITEM.into()),
        ..Scope::default()
    };
    let mut envelope = Envelope::mutation(
        "item resolve-history",
        scope,
        if code == ResultCode::AlreadyApplied {
            manyhands::results::Outcome::Noop
        } else {
            manyhands::results::Outcome::Blocked
        },
        code,
    );
    envelope.effects = Effects::not_requested();
    envelope
}

#[test]
fn a_refused_or_noop_prepare_names_where_to_go_and_a_given_one_does_not() {
    let sync = HistoryWayOut::Sync {
        item_id: ITEM.into(),
    };
    let noop = prepared_output(&preview_noop(ResultCode::AlreadyApplied), Some(&sync));
    assert!(stdout(&noop).contains(&format!(
        "next: manyhands-cli item sync --repo '{ROOT}' --id {ITEM}\n"
    )));
    let republish = HistoryWayOut::Republish {
        item_id: ITEM.into(),
    };
    let refused = prepared_output(
        &preview_noop(ResultCode::RemoteBranchDeleted),
        Some(&republish),
    );
    assert!(
        stderr(&refused).contains("next: manyhands-cli item republish"),
        "{}",
        stderr(&refused)
    );
    assert_eq!(refused.status, 3);
    let release = HistoryWayOut::ReleaseContext {
        item_id: ITEM.into(),
    };
    let refused = prepared_output(&preview_noop(ResultCode::InvalidInput), Some(&release));
    assert!(
        stderr(&refused).contains(RELEASE_NOTE),
        "{}",
        stderr(&refused)
    );
}

fn result(
    code: ResultCode,
    cleared: bool,
    marker: HistoryMarkerState,
) -> Envelope<MutationDataDto> {
    let scope = Scope {
        repository: Some(ROOT.into()),
        item_id: Some(ITEM.into()),
        ..Scope::default()
    };
    let mut envelope = Envelope::classified_mutation(
        "item resolve-history",
        scope,
        false,
        code,
        Effects::not_requested(),
        code == ResultCode::Ok,
    );
    envelope.data = Some(MutationDataDto::ResolveHistory(
        ItemResolveHistoryMutationDto {
            cache_loss_cleared: cleared,
            marker,
        },
    ));
    envelope
}

#[test]
fn a_resolution_s_result_says_to_sync_and_warns_of_an_undeleted_marker() {
    let sync = HistoryWayOut::Sync {
        item_id: ITEM.into(),
    };
    let done = executed_output(
        &result(ResultCode::Ok, true, HistoryMarkerState::Removed),
        false,
        Some(&sync),
    );
    assert!(
        stdout(&done).contains(&format!(
            "next: manyhands-cli item sync --repo '{ROOT}' --id {ITEM}\n"
        )),
        "{}",
        stdout(&done)
    );
    assert!(stderr(&done).is_empty());
    let stuck = executed_output(
        &result(ResultCode::Ok, true, HistoryMarkerState::Failed),
        false,
        Some(&sync),
    );
    assert_eq!(stderr(&stuck), note_line(MARKER_NOTE));
}

#[test]
fn a_flag_that_stays_names_remote_history() {
    let step = clear_cache_loss_way_out(ResultCode::RecoveryRequired).unwrap();
    let mut envelope = result(
        ResultCode::RecoveryRequired,
        false,
        HistoryMarkerState::NotApplicable,
    );
    envelope.command = "remote resolve-history".into();
    let output = executed_output(&envelope, false, Some(&step));
    assert!(
        stderr(&output).contains(&format!(
            "next: manyhands-cli remote history --repo '{ROOT}'\n"
        )),
        "{}",
        stderr(&output)
    );
    assert!(stderr(&output).contains(FLAG_NOTE));
}

#[test]
fn a_stopped_poll_is_the_code_poll_once_gives_the_same_stop() {
    use manyhands::repository::{RemoteHistoryError, RemoteOutcomeCategory};
    let cases = [
        (
            RemoteHistoryError::Stopped(RemoteOutcomeCategory::HostApprovalRequired),
            ResultCode::HostApprovalRequired,
        ),
        (
            RemoteHistoryError::Stopped(RemoteOutcomeCategory::UnlockRequired),
            ResultCode::UnlockRequired,
        ),
        (
            RemoteHistoryError::Stopped(RemoteOutcomeCategory::ConfigurationRequired),
            ResultCode::PublicationRemoteRequired,
        ),
        (RemoteHistoryError::Cancelled, ResultCode::Cancelled),
        (
            RemoteHistoryError::Interrupted,
            ResultCode::RecoveryRequired,
        ),
        (RemoteHistoryError::Internal, ResultCode::InternalError),
    ];
    for (error, code) in cases {
        let envelope = stop_envelope(&error, Scope::default(), ROOT);
        assert_eq!(envelope.code, code);
        let inspect = envelope
            .recovery
            .iter()
            .any(|action| action.action == RecoveryActionKind::RepoInspect);
        assert_eq!(inspect, code == ResultCode::RecoveryRequired, "{code:?}");
    }
}
