//! Task 7 QUALITY: live routing and truthful terminal request identities.
use super::*;
use super::{
    inspection_spec::{inspect_unchanged, ordinary},
    spec::authority_image,
};
use manyhands::results::Effects;

fn changed_routing(original: &[u8]) -> Vec<u8> {
    let text = std::str::from_utf8(original).unwrap();
    assert!(text.contains("primary_branch = \"main\""));
    text.replace(
        "primary_branch = \"main\"",
        "primary_branch = \"other-primary\"",
    )
    .into_bytes()
}

#[test]
fn quality_ordinary_inspection_refuses_stable_live_routing_change() {
    for disposition in ["contained", "fast_forward", "merge"] {
        for phase in ["prepared", "index_pending"] {
            let f = ordinary(disposition, phase, false);
            assert_eq!(
                inspect_unchanged(&f).unwrap().next_action,
                LocalContextSyncNextAction::Resume
            );
            let config = f.world.root.join(".manyhands/config.toml");
            let original = std::fs::read(&config).unwrap();
            std::fs::write(&config, changed_routing(&original)).unwrap();
            match inspect_unchanged(&f) {
                Err(error) => assert_eq!(error.code(), ResultCode::ExternalChange),
                Ok(view) => {
                    assert_eq!(
                        view.next_action,
                        LocalContextSyncNextAction::ExternalRepair,
                        "{disposition}/{phase}"
                    );
                    assert_eq!(view.reason, LocalContextSyncReason::ForeignChange);
                    assert!(!view.can_start_resolution);
                }
            }
            std::fs::write(config, original).unwrap();
            let restored = inspect_unchanged(&f).unwrap();
            assert_eq!(restored.next_action, LocalContextSyncNextAction::Resume);
            assert_eq!(
                restored.required_request_id.as_deref(),
                Some(mutation::REQUEST_2)
            );
        }
    }
}

#[test]
fn quality_ordinary_inspection_rechecks_live_routing_after_session_b() {
    for disposition in ["contained", "fast_forward", "merge"] {
        for phase in ["prepared", "index_pending"] {
            let f = ordinary(disposition, phase, false);
            let config = f.world.root.join(".manyhands/config.toml");
            let original = std::fs::read(&config).unwrap();
            let updated = changed_routing(&original);
            let hook_path = config.clone();
            f.world.service.set_observation_hook_for_testing(move || {
                std::fs::write(hook_path, updated).unwrap()
            });
            let _lease = RepositoryService::hold_lease_for_testing(
                &f.world.root,
                f.world.data.path(),
                LeaseKind::Repository,
            )
            .unwrap();
            let sql = authority_image(&f.world);
            let admin = super::super::support::repository_git_file_bytes(&f.world.fixture);
            let git = super::super::support::repository_and_worktree_snapshot(&f.world.fixture);
            let result = f
                .world
                .service
                .inspect_local_context_sync(&f.world.resolved(), f.operation);
            // Restore only the fixture's deliberate change before comparing the
            // complete image, so hidden read effects are still detected.
            std::fs::write(&config, original).unwrap();
            assert_eq!(
                result.unwrap_err().code(),
                ResultCode::ExternalChange,
                "{disposition}/{phase}"
            );
            assert!(sql == authority_image(&f.world));
            assert!(admin == super::super::support::repository_git_file_bytes(&f.world.fixture));
            assert!(
                git == super::super::support::repository_and_worktree_snapshot(&f.world.fixture)
            );
            drop(_lease);
            assert_eq!(
                inspect_unchanged(&f).unwrap().next_action,
                LocalContextSyncNextAction::Resume
            );
        }
    }
}

fn terminal_parent(state: &str) -> (ConflictWorld, LocalContextResolveInput) {
    if state == "cancelled" {
        let f = ordinary("merge", "prepared", false);
        f.world
            .service
            .cancel_request(RequestId::parse(mutation::REQUEST_2).unwrap());
        assert_eq!(
            f.world.execute(mutation::REQUEST_2, f.start()).code,
            ResultCode::Cancelled
        );
        let input = LocalContextResolveInput {
            root: f.world.root.clone(),
            operation_id: f.operation,
            observation: format!("local-sync-v1:{}", "0".repeat(64)),
            resolutions: vec![LocalConflictResolutionInput {
                token: format!("local-conflict-v1:{}", "0".repeat(64)),
                source: items::ticket_source(items::TICKET_A, "Unapplied input", ""),
            }],
        };
        return (f, input);
    }
    let mut f = ConflictWorld::new();
    let input = f.input();
    if state == "abandoned" {
        f.world.service = super::super::support::FailOnce::at(FailurePoint::LocalSyncBeforeIndex)
            .open_service(f.world.data.path());
    }
    let result = f.world.execute(
        mutation::REQUEST_3,
        Mutation::LocalContextResolve(copy_input(&input)),
    );
    if state == "abandoned" {
        assert_eq!(result.code, ResultCode::DiscoveryPending);
        let abandon = Mutation::OperationAbandon(OperationAbandonInput {
            root: f.world.root.clone(),
            operation_id: f.operation,
        });
        let preview = f.world.service.prepare(&abandon).previewed();
        assert_eq!(preview.code, ResultCode::Ok);
        assert_eq!(
            mutation::execute_confirmed(
                &f.world.service,
                super::super::REQUEST_ABSENT,
                &preview.data.unwrap().confirmation_id,
                abandon
            )
            .code,
            ResultCode::Ok
        );
    } else {
        assert_eq!(result.code, ResultCode::Ok);
    }
    (f, input)
}
fn terminal_role(resolve: bool) {
    for state in ["cancelled", "abandoned", "complete"] {
        let (f, input) = terminal_parent(state);
        let id = if resolve {
            super::super::REQUEST_A
        } else {
            super::super::REQUEST_B
        };
        let make = || {
            if resolve {
                Mutation::LocalContextResolve(copy_input(&input))
            } else {
                Mutation::OperationResume(OperationResumeInput {
                    root: f.world.root.clone(),
                    operation_id: f.operation,
                })
            }
        };
        let db = items::index(f.world.data.path());
        let attempts = db
            .query_row(
                "SELECT count(*) FROM local_context_sync_resolutions",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap();
        let before = super::super::support::repository_and_worktree_snapshot(&f.world.fixture);
        let result = f.world.execute(id, make());
        assert_eq!(
            result.code,
            if state == "complete" {
                ResultCode::ExternalChange
            } else {
                ResultCode::Cancelled
            }
        );
        assert_eq!(result.effects, Effects::not_requested());
        let Some(MutationDataDto::LocalContextSync(data)) = result.data.as_ref() else {
            panic!("local terminal data")
        };
        assert_eq!(
            data.resolution_request_id.as_deref(),
            if resolve { Some(id) } else { None }
        );
        assert_eq!(
            data.state,
            match state {
                "cancelled" => LocalContextSyncState::Cancelled,
                "abandoned" => LocalContextSyncState::Abandoned,
                _ => LocalContextSyncState::Complete,
            }
        );
        assert_eq!(
            data.reason,
            match state {
                "cancelled" => LocalContextSyncReason::Cancelled,
                "abandoned" => LocalContextSyncReason::Abandoned,
                _ => LocalContextSyncReason::ForeignChange,
            }
        );
        assert_eq!(data.next_action, LocalContextSyncNextAction::None);
        assert!(data.required_request_id.is_none());
        if state == "complete" {
            assert!(f.world.record(id).is_none());
        } else {
            assert_eq!(f.world.record(id).unwrap().state, RequestState::Finished);
        }
        let stored = f.world.record(id);
        let replay = f.world.execute(id, make());
        assert_eq!(
            serde_json::to_value(&result).unwrap(),
            serde_json::to_value(&replay).unwrap()
        );
        assert_eq!(stored, f.world.record(id));
        let changed = if resolve {
            let mut different = copy_input(&input);
            different.observation.push('0');
            Mutation::LocalContextResolve(different)
        } else {
            Mutation::OperationResume(OperationResumeInput {
                root: f.world.root.clone(),
                operation_id: OperationId::new(),
            })
        };
        let changed = f.world.execute(id, changed);
        assert_eq!(
            changed.code == ResultCode::RequestMismatch,
            state != "complete"
        );
        assert_eq!(stored, f.world.record(id));
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM local_context_sync_resolutions",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            attempts
        );
        assert!(
            before == super::super::support::repository_and_worktree_snapshot(&f.world.fixture)
        );
    }
}
#[test]
fn quality_terminal_unlinked_resolve_identity_and_reason() {
    terminal_role(true);
}
#[test]
fn quality_terminal_unlinked_resume_identity_and_reason() {
    terminal_role(false);
}
