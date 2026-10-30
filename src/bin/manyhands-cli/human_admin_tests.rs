use super::*;
use manyhands::results::{Effects, Outcome, ResultCode, Scope, WriteEffect};
use std::collections::HashSet;

#[test]
fn ordering_fields_sorts_nested_objects_without_reordering_arrays() {
    let dto = serde_json::json!({
        "z_last": null,
        "nested": {"z_value": "last\n", "a_value": "first"},
        "entries": [
            {"z_value": "second", "a_value": 2},
            {"z_value": "first", "a_value": 1}
        ],
        "empty": [],
        "a_first": true
    });
    assert_eq!(
        fields(&dto),
        concat!(
            "A first: true\n",
            "Empty: none\n",
            "Entries a value: 2\n",
            "Entries z value: second\n",
            "Entries a value: 1\n",
            "Entries z value: first\n",
            "Nested a value: first\n",
            "Nested z value: last\\n\n",
            "Z last: none\n"
        )
    );
}

#[test]
fn ordering_preview_keeps_effect_sequence_after_sorted_fields() {
    let mut dto = sample(PreviewDetailsDto::Repository(RepositoryPreviewDto {
        root: "root\n".into(),
        primary_branch: "main".into(),
        identity: PreviewIdentityDto {
            name: "Person".into(),
            email: "person@example.invalid".into(),
            source: PreviewIdentitySource::Supplied,
            written_locally: true,
        },
        slug_prefix: None,
    }));
    dto.effects = vec![
        PreviewEffectDto {
            effect: PreviewEffect::ConfigurationWritten,
        },
        PreviewEffectDto {
            effect: PreviewEffect::DirectoryCreated,
        },
    ];
    assert_eq!(
        preview(&dto),
        concat!(
            "Preview of command\\r\n",
            "Identity email: person@example.invalid\n",
            "Identity name: Person\n",
            "Identity source: supplied\n",
            "Identity written locally: true\n",
            "Primary branch: main\n",
            "Root: root\\n\n",
            "Slug prefix: none\n",
            "Will write the Manyhands configuration.\n",
            "Will create the directory.\n",
            "Confirmation: confirmation\\n (expires expiry\\t)\n"
        )
    );
}

#[test]
fn ordering_observation_input_is_stable_for_approve_and_replace() {
    for (pin, expected) in [
        (
            None,
            "Input for host approve:\n{\"algorithm\":\"ssh-ed25519\",\"authority\":\"[2001:db8::1]:2222\",\"fingerprint\":\"SHA256:presented\"}\n",
        ),
        (
            Some("SHA256:old"),
            "Input for host replace:\n{\"algorithm\":\"ssh-ed25519\",\"authority\":\"[2001:db8::1]:2222\",\"fingerprint\":\"SHA256:presented\",\"old_fingerprint\":\"SHA256:old\"}\n",
        ),
    ] {
        let dto = HostObservationDto {
            host: "2001:db8::1".into(),
            port: 2222,
            fetch: true,
            push: false,
            presented_algorithm: "ssh-ed25519".into(),
            presented_fingerprint: "SHA256:presented".into(),
            pinned_algorithm: pin.map(|_| "ssh-ed25519".into()),
            pinned_fingerprint: pin.map(str::to_owned),
            reapproval_required: false,
        };
        let out = observation(&dto);
        assert_eq!(
            &out.stdout[out.stdout.find("Input for host ").unwrap()..],
            expected
        );
        assert!(out.notes.is_empty());
    }
}

#[test]
fn every_preview_effect_has_a_distinct_phrase() {
    let phrases: HashSet<_> = PreviewEffect::ALL
        .iter()
        .map(|e| effect_phrase(*e))
        .collect();
    assert_eq!(phrases.len(), PreviewEffect::ALL.len());
    assert!(phrases.iter().all(|s| !s.is_empty() && !s.contains('_')));
}
fn sample(details: PreviewDetailsDto) -> PreviewDto {
    PreviewDto {
        confirmation_id: "confirmation\n".into(),
        expires_at: "expiry\t".into(),
        command: "command\r".into(),
        scope: Scope::default(),
        effects: vec![PreviewEffectDto {
            effect: PreviewEffect::IdentityWritten,
        }],
        details,
    }
}
#[test]
fn every_details_shape_renders_its_fields() {
    let identity = IdentityDto {
        name: Some("effective-name".into()),
        email: Some("effective-email".into()),
        source: IdentitySource::Repository,
        initials: None,
        initials_source: InitialsSource::None,
    };
    let cases = vec![
        PreviewDetailsDto::Repository(RepositoryPreviewDto {
            root: "root\n".into(),
            primary_branch: "branch".into(),
            identity: PreviewIdentityDto {
                name: "name".into(),
                email: "email".into(),
                source: PreviewIdentitySource::Supplied,
                written_locally: true,
            },
            slug_prefix: Some("prefix".into()),
        }),
        PreviewDetailsDto::RemoteRemoval(RemoteRemovalPreviewDto {
            root: "root".into(),
            name: "origin".into(),
            fetch_location: Some("fetch".into()),
            push_location: Some("push".into()),
            publication_affected: false,
            polling_affected: true,
        }),
        PreviewDetailsDto::RepositoryRemoval(RepositoryRemovalPreviewDto {
            root: "root".into(),
            enabled_at: Some("time".into()),
            files_changed: false,
            git_state_changed: false,
        }),
        PreviewDetailsDto::KeyDeletion(KeyDeletionPreviewDto {
            id: "key".into(),
            label: "label".into(),
            fingerprint: Some("fingerprint".into()),
            private_key_path: "private-path".into(),
            public_key_path: Some("public-path".into()),
        }),
        PreviewDetailsDto::OperationAbandon(OperationAbandonPreviewDto {
            root: "root".into(),
            operation_id: "operation".into(),
            action: None,
            state: "state".into(),
            step: Some("step".into()),
            item_id: Some("item".into()),
            worktree: Some("worktree".into()),
            paths: vec!["path1".into(), "path2".into()],
            paths_complete: true,
            committed: true,
            request_id: Some("request".into()),
            files_changed: false,
            git_state_changed: false,
        }),
        PreviewDetailsDto::Identity(IdentityPreviewDto {
            root: "root".into(),
            name: "name".into(),
            email: "email".into(),
            initials: Some("initials".into()),
            current: LocalIdentityDto {
                name: Some("oldname".into()),
                email: Some("oldemail".into()),
                initials: Some("oldinitials".into()),
            },
            effective: identity,
        }),
        PreviewDetailsDto::Host(HostPreviewDto {
            root: "root".into(),
            host: "host".into(),
            port: 2222,
            fetch: true,
            push: false,
            algorithm: "algorithm".into(),
            fingerprint: "fingerprint".into(),
            pinned_algorithm: Some("pinned-algorithm".into()),
            pinned_fingerprint: Some("pinned-fingerprint".into()),
            reapproval_required: true,
        }),
        PreviewDetailsDto::Repair(RepairPreviewDto {
            root: "root".into(),
            path: "path".into(),
            id: "id".into(),
            adoption: true,
            primary_changed: false,
        }),
    ];
    fn leaves(v: &serde_json::Value, out: &str) {
        match v {
            serde_json::Value::Object(m) => {
                for (key, value) in m {
                    assert!(
                        out.to_lowercase().contains(&key.replace('_', " ")),
                        "missing field {key}: {out}"
                    );
                    leaves(value, out);
                }
            }
            serde_json::Value::Array(a) => {
                for value in a {
                    leaves(value, out)
                }
            }
            serde_json::Value::String(s) => assert!(
                out.contains(&crate::render::field(s)),
                "missing value: {out}"
            ),
            serde_json::Value::Bool(b) => assert!(out.contains(&b.to_string())),
            serde_json::Value::Number(n) => assert!(out.contains(&n.to_string())),
            serde_json::Value::Null => {}
        }
    }
    for details in cases {
        let p = sample(details);
        let out = preview(&p);
        leaves(&serde_json::to_value(&p.details).unwrap(), &out);
        assert!(out.starts_with("Preview of command\\r\n"));
        assert!(out.contains("Confirmation: confirmation\\n (expires expiry\\t)"));
        assert!(out.contains(effect_phrase(PreviewEffect::IdentityWritten)));
        assert!(!out.contains("Apply with"));
    }
}
#[test]
fn a_failure_adds_request_operation_effects_and_data_lines() {
    let mut effects = Effects::not_requested();
    effects.write = WriteEffect::Written;
    effects.commit_oid = Some("commit\n".into());
    let e = Envelope::mutation(
        "host approve",
        Scope::default(),
        Outcome::Partial,
        ResultCode::HostMismatch,
    )
    .with_request_id("request\n")
    .with_operation_id("operation\t")
    .with_effects(effects)
    .with_data(MutationDataDto::Host(HostMutationDto {
        host: "host\r".into(),
        port: 22,
        algorithm: "algorithm".into(),
        fingerprint: "requested".into(),
        presented_algorithm: Some("presented-algorithm".into()),
        presented_fingerprint: Some("presented-fingerprint".into()),
    }));
    let out = failure_extra(&e, false);
    for expected in [
        "request: request\\n",
        "operation: operation\\t",
        "effects:",
        "written",
        "commit\\n",
        "host\\r",
        "presented-algorithm",
        "presented-fingerprint",
        "requested",
    ] {
        assert!(out.contains(expected), "{out}");
    }
}
#[test]
fn the_wrong_passphrase_hint_appears_only_after_supplied_and_transport_unavailable() {
    for code in [
        ResultCode::TransportUnavailable,
        ResultCode::UnlockFailed,
        ResultCode::Busy,
    ] {
        for supplied in [false, true] {
            let e = Envelope::failure("host approve", Scope::default(), code, vec![]);
            assert_eq!(
                failure_extra(&e, supplied)
                    .contains("hint: if the passphrase was wrong, run the command again\n"),
                supplied && code == ResultCode::TransportUnavailable
            );
        }
    }
}
#[test]
fn a_noop_says_already_applied() {
    let e = Envelope::mutation(
        "repo identity-set",
        Scope::default(),
        Outcome::Noop,
        ResultCode::AlreadyApplied,
    )
    .with_request_id("id");
    assert!(
        mutation_success(&e)
            .stdout
            .starts_with("Already applied; nothing changed.\n")
    );
}

fn stdout(output: &crate::render::Output) -> &str {
    std::str::from_utf8(&output.stdout).unwrap()
}

fn stderr(output: &crate::render::Output) -> &str {
    std::str::from_utf8(&output.stderr).unwrap()
}

/// No byte of `text` is a control other than a line's end.
fn no_raw_control(text: &str) {
    assert!(
        !text.chars().any(|c| c.is_control() && c != '\n'),
        "{text:?}"
    );
}

fn republish_noop(scope: Scope) -> Envelope<PrepareDataDto> {
    Envelope::mutation(
        "item republish",
        scope,
        Outcome::Noop,
        ResultCode::AlreadyApplied,
    )
}

/// Controller ruling 2 of Task 10: the library offers no recovery action
/// that names a sync, so the human output says to run `item sync` first.
#[test]
fn the_no_op_republish_preview_says_to_run_item_sync_first() {
    let envelope = republish_noop(Scope {
        repository: Some("/work/repo".into()),
        item_id: Some("01ARZ3NDEKTSV4RRFFQ69G5FAW".into()),
        ..Scope::default()
    });
    let output = crate::render::prepare_output(false, &envelope);
    assert_eq!(output.status, 0);
    assert_eq!(
        stdout(&output),
        "Already applied; nothing changed.\n\
         next: manyhands-cli item sync --repo '/work/repo' --id 01ARZ3NDEKTSV4RRFFQ69G5FAW\n"
    );
    assert_eq!(
        stderr(&output),
        "note: no deletion of this item's published branch is recorded, so there is nothing to republish; run item sync first, which looks at the remote, then prepare the republish again.\n"
    );
}

#[test]
fn the_no_op_republish_preview_without_a_repository_still_says_to_run_item_sync() {
    let output = crate::render::prepare_output(false, &republish_noop(Scope::default()));
    assert_eq!(stdout(&output), "Already applied; nothing changed.\n");
    assert!(
        stderr(&output).contains("run item sync first"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn the_no_op_republish_preview_escapes_its_root_and_prints_no_raw_control() {
    let envelope = republish_noop(Scope {
        repository: Some("/work/it's\n\u{1b}[31m".into()),
        item_id: Some("01ARZ3NDEKTSV4RRFFQ69G5FAW".into()),
        ..Scope::default()
    });
    let output = crate::render::prepare_output(false, &envelope);
    no_raw_control(stdout(&output));
    no_raw_control(stderr(&output));
    assert!(stdout(&output).contains("--repo '/work/it'\\''s\\n\\u{1b}[31m'"));
}

#[test]
fn another_command_s_no_op_preview_and_json_are_c2_s() {
    let mut other = republish_noop(Scope::default());
    other.command = "repo identity-set".into();
    let output = crate::render::prepare_output(false, &other);
    assert_eq!(stdout(&output), "Already applied; nothing changed.\n");
    assert!(output.stderr.is_empty());
    let json = crate::render::prepare_output(true, &republish_noop(Scope::default()));
    assert!(json.stderr.is_empty());
    assert!(!stdout(&json).contains("item sync"));
}
