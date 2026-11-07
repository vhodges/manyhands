use super::{
    conflict_tests::{fixture, input, start},
    control,
    record::LocalSyncState,
    resolve,
};
use crate::{repository::*, results::*};

#[test]
fn cancellation_acknowledges_only_observed_generation_and_explicit_retry_progresses() {
    let (f, _) = fixture(AuthoringKind::Ticket);
    let id = start(&f);
    let (observation, sources) = input(&f, id);
    let request = OperationId::new();
    let data = f.data.path().to_owned();
    let root = f.root.path().to_owned();
    resolve::HOOK.with(|h| {
        *h.borrow_mut() = Some(Box::new(move |point| {
            if point == resolve::Point::Path(1) {
                let service = RepositoryService::open_at(&data).unwrap();
                let repo = service.resolve_repository(&root).unwrap();
                control::issue(&service, &repo, id, Some(request)).unwrap();
            }
            Ok(())
        }))
    });
    let result = resolve::execute(&f.service, &f.repo, id, request, &observation, &sources);
    resolve::HOOK.with(|h| h.borrow_mut().take());
    assert_eq!(result.code, ResultCode::Cancelled, "{result:?}");
    assert!(result.cancel_acknowledged);
    assert_eq!(result.state, Some(LocalSyncState::ResolutionPrepared));
    let phase = f
        .service
        .read_local_context_sync_record(&f.repo, id)
        .unwrap();
    assert!(control::abandon(&f.service, &f.repo, id).is_err());
    let data = f.data.path().to_owned();
    let root = f.root.path().to_owned();
    control::issue(&f.service, &f.repo, id, None).unwrap();
    control::ACK_HOOK.with(|h| {
        *h.borrow_mut() = Some(Box::new(move || {
            let service = RepositoryService::open_at(&data).unwrap();
            let repo = service.resolve_repository(&root).unwrap();
            control::issue(&service, &repo, id, None).unwrap();
        }))
    });
    assert_eq!(
        resolve::execute(&f.service, &f.repo, id, request, &observation, &sources).code,
        ResultCode::Cancelled
    );
    control::ACK_HOOK.with(|h| h.borrow_mut().take());
    let row = f
        .service
        .read_local_context_sync_record(&f.repo, id)
        .unwrap();
    assert_eq!((row.cancel_issued, row.cancel_acknowledged), (2, 1));
    assert_eq!(
        resolve::execute(&f.service, &f.repo, id, request, &observation, &sources).code,
        ResultCode::Cancelled
    );
    assert_eq!(
        resolve::execute(&f.service, &f.repo, id, request, &observation, &sources).code,
        ResultCode::Ok
    );
    assert_eq!(
        f.service
            .read_local_context_sync_record(&f.repo, id)
            .unwrap()
            .active_resolution,
        phase.active_resolution
    );
}

#[test]
fn cancellation_before_effects_is_terminal_and_conflict_cancellation_retains_owner() {
    let (f, _) = fixture(AuthoringKind::Document);
    let id = OperationId::new();
    let observation = f.observe();
    let failed = RepositoryService::open_at_with_failure_point_for_testing(
        f.data.path(),
        FailurePoint::LocalSyncAfterIntent,
    )
    .unwrap();
    super::run::execute(
        &failed,
        &f.repo,
        f.kind,
        &f.item,
        id,
        &observation.observation,
    );
    control::issue(&f.service, &f.repo, id, None).unwrap();
    let result = super::run::execute(
        &f.service,
        &f.repo,
        f.kind,
        &f.item,
        id,
        &observation.observation,
    );
    assert_eq!(result.code, ResultCode::Cancelled);
    assert_eq!(result.state, Some(LocalSyncState::Cancelled));
    assert!(!result.effects.is_durable());
    assert_eq!(
        super::run::execute(
            &f.service,
            &f.repo,
            f.kind,
            &f.item,
            id,
            &observation.observation
        ),
        result
    );
    let (f, _) = fixture(AuthoringKind::Ticket);
    let id = start(&f);
    let before = f.image();
    control::issue(&f.service, &f.repo, id, None).unwrap();
    assert_eq!(
        resolve::resume(&f.service, &f.repo, id).code,
        ResultCode::Cancelled
    );
    assert_eq!(before, f.image());
    assert!(control::abandon(&f.service, &f.repo, id).is_err());
    let row = f
        .service
        .read_local_context_sync_record(&f.repo, id)
        .unwrap();
    assert_eq!(row.state, LocalSyncState::ConflictPending);
}

#[test]
fn only_artifact_free_domain_abandonment_can_revoke_authority() {
    for integrated in [false, true] {
        let (f, _) = fixture(AuthoringKind::Document);
        let id = if integrated {
            let id = start(&f);
            let (observation, sources) = input(&f, id);
            let failed = RepositoryService::open_at_with_failure_point_for_testing(
                f.data.path(),
                FailurePoint::LocalSyncBeforeIndex,
            )
            .unwrap();
            assert_eq!(
                resolve::execute(
                    &failed,
                    &f.repo,
                    id,
                    OperationId::new(),
                    &observation,
                    &sources
                )
                .code,
                ResultCode::DiscoveryPending
            );
            id
        } else {
            let id = OperationId::new();
            let failed = RepositoryService::open_at_with_failure_point_for_testing(
                f.data.path(),
                FailurePoint::LocalSyncAfterIntent,
            )
            .unwrap();
            super::run::execute(
                &failed,
                &f.repo,
                f.kind,
                &f.item,
                id,
                &f.observe().observation,
            );
            id
        };
        let before = f.image();
        control::abandon(&f.service, &f.repo, id).unwrap();
        assert_eq!(before, f.image());
        assert_eq!(
            f.service
                .read_local_context_sync_record(&f.repo, id)
                .unwrap()
                .state,
            LocalSyncState::Abandoned
        );
        assert_eq!(
            resolve::resume(&f.service, &f.repo, id).code,
            ResultCode::Cancelled
        );
        assert_eq!(
            f.service
                .read_local_context_sync_record(&f.repo, id)
                .unwrap()
                .state,
            LocalSyncState::Abandoned
        );
    }
}

#[test]
fn cancellation_after_integration_and_at_index_handoff_keeps_proven_effects() {
    for index in [false, true] {
        let (f, _) = fixture(AuthoringKind::Ticket);
        let id = start(&f);
        let (observation, sources) = input(&f, id);
        let request = OperationId::new();
        let data = f.data.path().to_owned();
        let root = f.root.path().to_owned();
        let issue = move || {
            let service = RepositoryService::open_at(&data).unwrap();
            let repo = service.resolve_repository(&root).unwrap();
            control::issue(&service, &repo, id, None).unwrap();
        };
        if index {
            super::run::BOUNDARY_HOOK.with(|h| {
                *h.borrow_mut() = Some(Box::new(move |p| {
                    if p == FailurePoint::LocalSyncBeforeIndex {
                        issue();
                    }
                }))
            });
        } else {
            resolve::HOOK.with(|h| {
                *h.borrow_mut() = Some(Box::new(move |p| {
                    if p == resolve::Point::Ref {
                        issue();
                    }
                    Ok(())
                }))
            });
        }
        let result = resolve::execute(&f.service, &f.repo, id, request, &observation, &sources);
        resolve::HOOK.with(|h| h.borrow_mut().take());
        super::run::BOUNDARY_HOOK.with(|h| h.borrow_mut().take());
        assert_eq!(
            result.code,
            ResultCode::Cancelled,
            "index={index}: {result:?}"
        );
        assert!(result.cancel_acknowledged);
        assert_eq!(result.effects.integration, IntegrationEffect::Complete);
        assert_eq!(result.effects.checkpoint, CheckpointEffect::Committed);
        let candidate = result.effects.commit_oid.clone();
        let final_result = resolve::resume(&f.service, &f.repo, id);
        assert_eq!(final_result.code, ResultCode::Ok, "{final_result:?}");
        assert_eq!(final_result.effects.commit_oid, candidate);
    }
}
