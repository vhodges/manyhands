use super::*;
use manyhands::repository::*;
use manyhands::results::{OperationFailureCode, ProblemCode};

fn contains(output: Human, needles: &[&str]) {
    for needle in needles {
        assert!(
            output.stdout.contains(needle),
            "missing {needle}: {}",
            output.stdout
        );
    }
    assert!(output.stdout.ends_with('\n'));
}

fn configuration() -> ConfigurationDto {
    ConfigurationDto {
        state: ConfigurationState::Valid,
        primary_branch: Some("main".into()),
        publication_remote: Some("origin".into()),
        problems: vec![],
    }
}

fn remote() -> RemoteDto {
    RemoteDto {
        name: "origin".into(),
        fetch_location: "ssh://git@example.invalid/repo".into(),
        push_location: "[redacted]".into(),
        publication_eligible: true,
        selected_for_publication: true,
    }
}

fn key() -> KeyDto {
    KeyDto {
        id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".into(),
        label: "Work key".into(),
        ownership: KeyOwnership::Imported,
        selected: true,
        fingerprint: Some("SHA256:public-fingerprint".into()),
        private_source_state: KeyPrivateSourceState::Missing,
        public_metadata_state: KeyPublicMetadataState::Available,
        private_key_path: "/keys/private".into(),
        public_key_path: Some("/keys/public".into()),
    }
}

fn pin() -> HostPinDto {
    HostPinDto {
        host: "::1".into(),
        port: 2222,
        pinned_algorithm: "ssh-ed25519".into(),
        pinned_fingerprint: "SHA256:host-fingerprint".into(),
        reapproval_required: true,
    }
}

fn operation() -> OperationDto {
    OperationDto {
        operation_id: Some("01ARZ3NDEKTSV4RRFFQ69G5FA0".into()),
        family: OperationFamily::Remote,
        owner: OperationOwner::Repository,
        action: OperationAction::SynchronizePrimary,
        state: "failed".into(),
        completed_step: Some("before_fetch".into()),
        next_action: Some(OperationNextAction::Resume),
        item_id: None,
        key_id: None,
        worktree: Some("/repository".into()),
        updated_at: Some("2026-10-19T12:00:00Z".into()),
        failure_code: Some(OperationFailureCode::ProtocolRejected),
    }
}

#[test]
fn repository_list_reports_each_registration_and_index_notes() {
    let mut data = RepositoryListDto {
        items: vec![RepositorySummaryDto {
            root: "/repository".into(),
            enabled_at: Some("2026-10-19T12:00:00Z".into()),
            accessibility: Accessibility::Accessible,
            configuration: configuration(),
            index: IndexStateDto {
                state: IndexState::Stale,
                refreshed_at: None,
            },
            problem_count: 2,
        }],
        complete: false,
    };
    let output = repository_list(&data);
    assert_eq!(output.stdout.lines().count(), 1);
    assert!(output.stdout.contains("/repository"));
    assert!(output.notes.iter().any(|note| note.contains("stale")));
    assert!(output.notes.iter().any(|note| note.contains("incomplete")));
    data.items[0].index.state = IndexState::NeverRefreshed;
    assert!(
        repository_list(&data)
            .notes
            .iter()
            .any(|note| note.contains("never_refreshed"))
    );
    data.items.clear();
    data.complete = true;
    assert!(repository_list(&data).stdout.is_empty());
}

#[test]
fn repository_inspection_reports_registration_configuration_and_remotes() {
    let data = RepositoryInspectionDto {
        selected_path: "/linked".into(),
        root: "/repository".into(),
        registered: false,
        head_branch: Some("main".into()),
        local_branches: vec!["main".into(), "topic".into()],
        configuration: configuration(),
        identity_state: IdentityAvailability::Available,
        remotes: vec![remote()],
    };
    contains(
        repository_inspection(&data),
        &[
            "/linked",
            "/repository",
            "false",
            "main",
            "topic",
            "valid",
            "available",
            "origin",
            "[redacted]",
        ],
    );
}

#[test]
fn identity_reports_sources_and_absent_values() {
    let data = IdentityDto {
        name: Some("A Name".into()),
        email: Some("a@example.invalid".into()),
        source: IdentitySource::Repository,
        initials: Some("an".into()),
        initials_source: InitialsSource::Derived,
    };
    contains(
        identity(&data),
        &["A Name", "a@example.invalid", "repository", "an", "derived"],
    );
    contains(
        identity(&IdentityDto {
            name: None,
            email: None,
            source: IdentitySource::None,
            initials: None,
            initials_source: InitialsSource::None,
        }),
        &["none", "-"],
    );
}

#[test]
fn remote_list_reports_redacted_locations_and_completeness() {
    let data = RemoteListDto {
        items: vec![remote()],
        complete: false,
    };
    assert_eq!(remote_list(&data).notes.len(), 1);
    contains(
        remote_list(&data),
        &[
            "origin",
            "ssh://git@example.invalid/repo",
            "[redacted]",
            "true",
        ],
    );
    assert!(
        remote_list(&RemoteListDto {
            items: vec![],
            complete: true
        })
        .stdout
        .is_empty()
    );
}

#[test]
fn key_list_reports_ids_labels_selection_and_incomplete_note() {
    let data = KeyListDto {
        items: vec![key()],
        complete: false,
    };
    contains(
        key_list(&data),
        &[&data.items[0].id, "Work key", "selected"],
    );
    assert_eq!(key_list(&data).notes.len(), 1);
    assert!(
        key_list(&KeyListDto {
            items: vec![],
            complete: true
        })
        .stdout
        .is_empty()
    );
}

#[test]
fn key_show_reports_metadata_without_reading_files() {
    contains(
        key_show(&key()),
        &[
            "Work key",
            "imported",
            "true",
            "SHA256:public-fingerprint",
            "missing",
            "available",
            "/keys/private",
            "/keys/public",
        ],
    );
}

#[test]
fn public_key_is_only_key_text_and_newline() {
    let data = PublicKeyDto {
        id: "not printed".into(),
        public_key: "ssh-ed25519 AAAA fixture".into(),
        fingerprint: "not printed".into(),
        matches_registration: false,
    };
    let output = public_key(&data);
    assert_eq!(output.stdout, "ssh-ed25519 AAAA fixture\n");
    assert!(output.notes.is_empty());
}

#[test]
fn host_list_reports_ipv6_and_reapproval_even_without_pins() {
    let mut data = HostPinListDto {
        items: vec![pin()],
        complete: false,
        reapproval_required: true,
    };
    contains(
        host_list(&data),
        &["[::1]:2222", "ssh-ed25519", "SHA256:host-fingerprint"],
    );
    assert_eq!(host_list(&data).notes.len(), 2);
    data.items.clear();
    data.complete = true;
    let output = host_list(&data);
    assert!(output.stdout.is_empty());
    assert!(output.notes[0].contains("reapproval"));
}

#[test]
fn host_inspection_reports_pin_and_reapproval() {
    contains(
        host_inspect(&pin()),
        &[
            "[::1]:2222",
            "ssh-ed25519",
            "SHA256:host-fingerprint",
            "true",
        ],
    );
}

#[test]
fn index_status_reports_counts_problems_operations_times_and_state_notes() {
    let mut data = IndexStatusDto {
        state: IndexStatusState::Stale,
        refreshed_at: Some("2026-10-19T12:00:00Z".into()),
        context_count: Some(1),
        item_count: Some(2),
        problem_count: Some(1),
        problems: vec![IndexProblemDto {
            code: ProblemCode::RetryRequired,
            path: Some("docs/a.md".into()),
            worktree: Some("/repository".into()),
        }],
        pending_operations: vec![operation()],
    };
    contains(
        index_status(&data),
        &[
            "stale",
            "2026-10-19T12:00:00Z",
            "retry_required",
            "docs/a.md",
            "01ARZ3NDEKTSV4RRFFQ69G5FA0",
        ],
    );
    assert_eq!(index_status(&data).notes.len(), 1);
    data.state = IndexStatusState::NeverRefreshed;
    assert!(index_status(&data).notes[0].contains("never_refreshed"));
}

#[test]
fn polling_status_reports_stored_policy_outcome_and_times() {
    let data = PollingStatusDto {
        enabled: true,
        paused: false,
        interval_seconds: 600,
        backoff_seconds: Some(120),
        recovery_suspended: true,
        latest_outcome: Some(PollingOutcome::HostApprovalRequired),
        latest_observed_at: Some("2026-10-19T12:00:00Z".into()),
        active_operation_id: Some("01ARZ3NDEKTSV4RRFFQ69G5FA0".into()),
        next_eligible_at: None,
    };
    contains(
        polling_status(&data),
        &[
            "600",
            "120",
            "true",
            "false",
            "host_approval_required",
            "2026-10-19T12:00:00Z",
            "01ARZ3NDEKTSV4RRFFQ69G5FA0",
            "-",
        ],
    );
}

#[test]
fn operation_list_reports_ids_states_actions_and_incompleteness() {
    let data = OperationListDto {
        items: vec![operation()],
        complete: false,
    };
    contains(
        operation_list(&data),
        &[
            "01ARZ3NDEKTSV4RRFFQ69G5FA0",
            "failed",
            "synchronize_primary",
        ],
    );
    assert_eq!(operation_list(&data).notes.len(), 1);
    assert!(
        operation_list(&OperationListDto {
            items: vec![],
            complete: true
        })
        .stdout
        .is_empty()
    );
}

#[test]
fn operation_show_reports_stored_fields_and_fixed_failure_code() {
    contains(
        operation_show(&operation()),
        &[
            "01ARZ3NDEKTSV4RRFFQ69G5FA0",
            "remote",
            "repository",
            "synchronize_primary",
            "failed",
            "before_fetch",
            "resume",
            "/repository",
            "2026-10-19T12:00:00Z",
            "protocol_rejected",
        ],
    );
}

#[test]
fn new_id_is_only_id_and_newline_without_notes() {
    let data = NewIdDto {
        id: "01M4CC0VMSZ7BRS740C8VSQ5ZK".into(),
    };
    let output = new_id(&data);
    assert_eq!(output.stdout, "01M4CC0VMSZ7BRS740C8VSQ5ZK\n");
    assert!(output.notes.is_empty());
}

// Ordinary Unicode survives; field-owned LF/CR/ESC cannot create metadata
// lines or terminal sequences. Tabs and C1 controls follow the same rule.
const CONTROL_FIELD: &str = "é 中文\nLabel: forged\r\u{1b}[31m\t\u{85}";
const ESCAPED_FIELD: &str = "é 中文\\nLabel: forged\\r\\u{1b}[31m\\t\\u{85}";

fn escaped_metadata(output: Human, lines: usize, fields: usize) {
    assert_eq!(output.stdout.lines().count(), lines, "{}", output.stdout);
    assert_eq!(output.stdout.matches(ESCAPED_FIELD).count(), fields);
    assert!(output.stdout.ends_with('\n'));
    assert!(output.stdout.chars().all(|c| c == '\n' || !c.is_control()));
    assert!(
        output
            .notes
            .iter()
            .all(|note| !note.chars().any(char::is_control))
    );
}

#[test]
fn identity_metadata_escapes_controls_without_escaping_unicode_or_layout() {
    escaped_metadata(
        identity(&IdentityDto {
            name: Some(CONTROL_FIELD.into()),
            email: Some(CONTROL_FIELD.into()),
            source: IdentitySource::Repository,
            initials: Some("é".into()),
            initials_source: InitialsSource::Derived,
        }),
        5,
        2,
    );
}

#[test]
fn key_metadata_escapes_controls_in_labels_and_paths() {
    let mut data = key();
    data.label = CONTROL_FIELD.into();
    data.private_key_path = CONTROL_FIELD.into();
    data.public_key_path = Some(CONTROL_FIELD.into());
    escaped_metadata(key_show(&data), 9, 3);
    escaped_metadata(
        key_list(&KeyListDto {
            items: vec![data.clone(), data],
            complete: true,
        }),
        2,
        2,
    );
}

#[test]
fn repository_and_remote_metadata_escape_controls_at_field_boundaries() {
    escaped_metadata(
        repository_list(&RepositoryListDto {
            items: vec![RepositorySummaryDto {
                root: CONTROL_FIELD.into(),
                enabled_at: None,
                accessibility: Accessibility::Accessible,
                configuration: configuration(),
                index: IndexStateDto {
                    state: IndexState::Current,
                    refreshed_at: None,
                },
                problem_count: 0,
            }],
            complete: true,
        }),
        1,
        1,
    );
    let mut remote = remote();
    remote.name = CONTROL_FIELD.into();
    remote.fetch_location = CONTROL_FIELD.into();
    remote.push_location = CONTROL_FIELD.into();
    escaped_metadata(
        remote_list(&RemoteListDto {
            items: vec![remote.clone()],
            complete: true,
        }),
        1,
        3,
    );
    let mut configuration = configuration();
    configuration.primary_branch = Some(CONTROL_FIELD.into());
    configuration.publication_remote = Some(CONTROL_FIELD.into());
    configuration.problems.push(ProblemDto {
        code: ProblemCode::RetryRequired,
        path: Some(CONTROL_FIELD.into()),
        target_id: None,
    });
    escaped_metadata(
        repository_inspection(&RepositoryInspectionDto {
            root: CONTROL_FIELD.into(),
            selected_path: CONTROL_FIELD.into(),
            registered: true,
            head_branch: Some(CONTROL_FIELD.into()),
            local_branches: vec![CONTROL_FIELD.into(), "普通".into()],
            configuration,
            identity_state: IdentityAvailability::Available,
            remotes: vec![remote],
        }),
        11,
        10,
    );
}

#[test]
fn host_and_status_metadata_escape_controls_consistently() {
    let mut pin = pin();
    pin.host = CONTROL_FIELD.into();
    pin.pinned_algorithm = CONTROL_FIELD.into();
    pin.pinned_fingerprint = CONTROL_FIELD.into();
    escaped_metadata(host_inspect(&pin), 4, 3);
    escaped_metadata(
        host_list(&HostPinListDto {
            items: vec![pin],
            complete: true,
            reapproval_required: false,
        }),
        1,
        3,
    );
    let mut operation = operation();
    operation.state = CONTROL_FIELD.into();
    operation.completed_step = Some(CONTROL_FIELD.into());
    operation.worktree = Some(CONTROL_FIELD.into());
    operation.updated_at = Some(CONTROL_FIELD.into());
    escaped_metadata(operation_show(&operation), 12, 4);
    escaped_metadata(
        operation_list(&OperationListDto {
            items: vec![operation.clone()],
            complete: true,
        }),
        1,
        1,
    );
    escaped_metadata(
        index_status(&IndexStatusDto {
            state: IndexStatusState::Stale,
            refreshed_at: Some(CONTROL_FIELD.into()),
            context_count: Some(1),
            item_count: Some(0),
            problem_count: Some(1),
            problems: vec![IndexProblemDto {
                code: ProblemCode::RetryRequired,
                path: Some(CONTROL_FIELD.into()),
                worktree: Some(CONTROL_FIELD.into()),
            }],
            pending_operations: vec![operation],
        }),
        7,
        4,
    );
    escaped_metadata(
        polling_status(&PollingStatusDto {
            enabled: true,
            paused: false,
            interval_seconds: 300,
            backoff_seconds: None,
            recovery_suspended: false,
            latest_outcome: None,
            latest_observed_at: Some(CONTROL_FIELD.into()),
            active_operation_id: None,
            next_eligible_at: Some(CONTROL_FIELD.into()),
        }),
        9,
        2,
    );
}
