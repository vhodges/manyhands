use super::record_tests::{Fixture, commit};
use crate::{repository::*, results::ResultCode};
use std::fs;

#[test]
fn observation_both_kinds_is_read_only_serializable_and_reopenable() {
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        let f = Fixture::new(kind);
        let before = f.image();
        let observed = f.observe();
        let json = serde_json::to_string(&observed).unwrap();
        let decoded: LocalContextSyncObservationDto = serde_json::from_str(&json).unwrap();
        let reopened = RepositoryService::open_at(f.data.path()).unwrap();
        assert_eq!(
            decoded,
            reopened
                .observe_local_context_sync(&f.repo, kind, &f.item)
                .unwrap()
        );
        assert_eq!(before, f.image());
        assert!(observed.observation.starts_with("local-sync-v1:"));
    }
}

#[test]
fn missing_or_wrong_kind_context_never_provisions_one() {
    let f = Fixture::new(AuthoringKind::Ticket);
    for (kind, id) in [
        (AuthoringKind::Document, f.item.clone()),
        (f.kind, crate::canonical::ItemId::generate()),
    ] {
        let error = f
            .service
            .observe_local_context_sync(&f.repo, kind, &id)
            .unwrap_err();
        assert_eq!(error.code(), ResultCode::ItemNotFound);
        assert_eq!(error.scope.item_id, Some(id.to_string()));
    }
}

#[test]
fn wrong_branch_and_dirty_context_refuse() {
    let f = Fixture::new(AuthoringKind::Document);
    let context = f.context();
    let tip = context.head().unwrap().target().unwrap();
    context.set_head_detached(tip).unwrap();
    assert_eq!(
        f.service
            .observe_local_context_sync(&f.repo, f.kind, &f.item)
            .unwrap_err()
            .code(),
        ResultCode::ExternalChange
    );
    context
        .set_head(&format!("refs/heads/manyhands/document/{}", f.item))
        .unwrap();
    for path in ["ordinary", "untracked"] {
        fs::write(
            context.workdir().unwrap().join(path),
            "private dirty source",
        )
        .unwrap();
        assert_eq!(
            f.service
                .observe_local_context_sync(&f.repo, f.kind, &f.item)
                .unwrap_err()
                .code(),
            ResultCode::WorktreeNotClean
        );
        context
            .checkout_head(Some(
                git2::build::CheckoutBuilder::new()
                    .force()
                    .remove_untracked(true),
            ))
            .unwrap();
    }
}

#[test]
fn observation_uses_committed_primary_and_excludes_effective_identity() {
    let f = Fixture::new(AuthoringKind::Ticket);
    let before = f.observe();
    let primary = git2::Repository::open(f.root.path()).unwrap();
    fs::write(f.root.path().join("ordinary"), "uncommitted primary").unwrap();
    primary
        .config()
        .unwrap()
        .set_str("user.name", "Changed identity")
        .unwrap();
    assert_eq!(before, f.observe());
    commit(&primary);
    assert_ne!(before.observation, f.observe().observation);
}

#[test]
fn authority_only_change_is_seen_by_fresh_read_session() {
    let f = Fixture::new(AuthoringKind::Ticket);
    let before = f.image();
    let error = super::observe::observe_with(&f.service, &f.repo, f.kind, &f.item, || {
        f.db()
            .execute(
                "UPDATE contexts SET branch='changed' WHERE item_id=?1",
                [f.item.to_string()],
            )
            .unwrap();
    })
    .unwrap_err();
    assert_eq!(error.code(), ResultCode::ExternalChange);
    assert_eq!(before, f.image());
}

#[test]
fn git_change_between_sessions_is_external_change() {
    let f = Fixture::new(AuthoringKind::Ticket);
    let error = super::observe::observe_with(&f.service, &f.repo, f.kind, &f.item, || {
        fs::write(f.context().workdir().unwrap().join("foreign"), "preserve").unwrap();
    })
    .unwrap_err();
    assert_eq!(error.code(), ResultCode::ExternalChange);
    assert!(f.context().workdir().unwrap().join("foreign").exists());
}

#[test]
fn unowned_merge_metadata_and_index_lock_refuse_without_cleanup() {
    for member in ["MERGE_MSG", "MERGE_MODE", "index.lock"] {
        let f = Fixture::new(AuthoringKind::Ticket);
        let path = f.context().path().join(member);
        fs::write(&path, "foreign evidence").unwrap();
        assert_eq!(
            f.service
                .observe_local_context_sync(&f.repo, f.kind, &f.item)
                .unwrap_err()
                .code(),
            ResultCode::RecoveryRequired
        );
        assert_eq!(fs::read(&path).unwrap(), b"foreign evidence");
    }
}

#[test]
fn changed_routing_cannot_bind_old_registered_primary() {
    let f = Fixture::new(AuthoringKind::Ticket);
    let config = f.root.path().join(crate::canonical::CONFIG_PATH);
    let source = fs::read_to_string(&config).unwrap();
    fs::write(&config, source.replace("main", "other")).unwrap();
    assert_eq!(
        f.service
            .observe_local_context_sync(&f.repo, f.kind, &f.item)
            .unwrap_err()
            .code(),
        ResultCode::ExternalChange
    );
}

#[test]
fn stale_discovery_cannot_observe_an_item_removed_from_the_context_commit() {
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        let f = Fixture::new(kind);
        let path = match kind {
            AuthoringKind::Document => "docs/local.md".to_owned(),
            AuthoringKind::Ticket => format!(".manyhands/tickets/{}/ticket.md", f.item),
        };
        let context = f.context();
        fs::remove_file(context.workdir().unwrap().join(&path)).unwrap();
        let mut index = context.index().unwrap();
        index.remove_path(std::path::Path::new(&path)).unwrap();
        index.write().unwrap();
        commit(&context);
        assert_eq!(
            f.service
                .observe_local_context_sync(&f.repo, kind, &f.item)
                .unwrap_err()
                .code(),
            ResultCode::ItemNotFound
        );
    }
}

#[test]
fn preexisting_unowned_conflicts_are_preserved_and_refused_for_both_kinds() {
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        let f = Fixture::new(kind);
        let context = f.context();
        fs::write(context.workdir().unwrap().join("ordinary"), "context\n").unwrap();
        commit(&context);
        let primary = git2::Repository::open(f.root.path()).unwrap();
        fs::write(f.root.path().join("ordinary"), "primary\n").unwrap();
        let incoming = commit(&primary);
        context
            .merge(
                &[&context.find_annotated_commit(incoming).unwrap()],
                None,
                None,
            )
            .unwrap();
        assert!(context.index().unwrap().has_conflicts());
        let before = f.image();
        let metadata = fs::read(context.path().join("MERGE_HEAD")).unwrap();
        assert_eq!(
            f.service
                .observe_local_context_sync(&f.repo, kind, &f.item)
                .unwrap_err()
                .code(),
            ResultCode::WorktreeConflicted
        );
        assert_eq!(before, f.image());
        assert_eq!(
            metadata,
            fs::read(context.path().join("MERGE_HEAD")).unwrap()
        );
    }
}
