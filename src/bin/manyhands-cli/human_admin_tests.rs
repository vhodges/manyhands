use super::*;
use manyhands::results::{Effects, Outcome, ResultCode, Scope, WriteEffect};
use std::collections::HashSet;
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
