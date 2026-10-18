use super::*;
use crate::repository::{AuthoringKind, RemoteOperationAction};

const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";

fn item() -> ItemId {
    ITEM.parse().unwrap()
}

#[test]
fn document_kind_matches_legacy_constants() {
    let kind = LifecycleKind::DocumentPromotion;
    assert_eq!(DocumentPromotionLifecycle::KIND, kind);
    // The literals the Cycle 09 runner, integrator and observer used at the base.
    assert_eq!(kind.authoring(), AuthoringKind::Document);
    assert_eq!(kind.action(), RemoteOperationAction::Promote);
    assert_eq!(
        kind.merge_subject(&item()),
        "Promote document 01ARZ3NDEKTSV4RRFFQ69G5FAV"
    );
    assert_eq!(
        kind.context_ref(&item()),
        "refs/heads/manyhands/document/01ARZ3NDEKTSV4RRFFQ69G5FAV"
    );
}

#[test]
fn ticket_kind_constants() {
    let kind = LifecycleKind::TicketClosure;
    assert_ne!(kind, LifecycleKind::DocumentPromotion);
    assert_eq!(kind.authoring(), AuthoringKind::Ticket);
    assert_eq!(kind.action(), RemoteOperationAction::Close);
    assert_eq!(
        kind.merge_subject(&item()),
        "Integrate closed ticket 01ARZ3NDEKTSV4RRFFQ69G5FAV"
    );
    assert_eq!(
        kind.context_ref(&item()),
        "refs/heads/manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAV"
    );
}

#[test]
fn document_promotion_has_no_pre_integration_effect() {
    let root = tempfile::tempdir().unwrap();
    let repository = git2::Repository::init(root.path()).unwrap();
    let context = integrate::IntegrationContext {
        item_id: item(),
        primary_branch: "main".into(),
        oid: Oid::from_str("1111111111111111111111111111111111111111").unwrap(),
    };
    let mut stage = LeasedIntegration {
        repository: &repository,
        root: root.path(),
        context: &context,
        base: Oid::from_str("2222222222222222222222222222222222222222").unwrap(),
        operation_id: None,
        record: None::<&record::PromotionRecord>,
    };
    assert_eq!(
        DocumentPromotionLifecycle
            .pre_integration(&mut stage)
            .unwrap(),
        PreIntegration::None
    );
}

/// Exhaustive: a new promotion step fails to compile here until it is mapped.
fn shared_view(step: record::PromotionStep) -> ParentStep {
    use record::PromotionStep as P;
    match step {
        P::Confirmed => ParentStep::Confirmed,
        P::ContextSynchronized => ParentStep::ContextSynchronized,
        P::Integrating => ParentStep::Integrating,
        P::Integrated => ParentStep::Integrated,
        P::Published => ParentStep::Published,
        P::PublicationPending => ParentStep::PublicationPending,
        P::RemoteDeleted => ParentStep::RemoteDeleted,
        P::WorktreeRemoving => ParentStep::WorktreeRemoving,
        P::WorktreeRemoved => ParentStep::WorktreeRemoved,
        P::BranchDeleted => ParentStep::BranchDeleted,
        P::Complete => ParentStep::Complete,
        P::Abandoned => ParentStep::Abandoned,
        P::Superseded => ParentStep::Superseded,
    }
}

#[test]
fn promotion_steps_map_to_the_shared_step_view() {
    use record::PromotionStep as P;
    let mut r = record::PromotionRecord {
        repository_id: 1,
        operation_id: crate::repository::OperationId::new(),
        item_id: item(),
        expectation: PromotionExpectation::from_bytes([3; 32]),
        remote_name: None,
        step: P::Confirmed,
        context_attempt: 0,
        context_sync_id: None,
        primary_attempt: 0,
        primary_sync_id: None,
        base_oid: None,
        context_oid: None,
        integration_oid: None,
        integration_kind: None,
        remote_context_oid: None,
        publication: None,
        cleanup: None,
        cancel_requested: false,
        updated_at: 0,
        refusal: None,
    };
    let all = [
        (P::Confirmed, "confirmed"),
        (P::ContextSynchronized, "context_synchronized"),
        (P::Integrating, "integrating"),
        (P::Integrated, "integrated"),
        (P::Published, "published"),
        (P::PublicationPending, "publication_pending"),
        (P::RemoteDeleted, "remote_deleted"),
        (P::WorktreeRemoving, "worktree_removing"),
        (P::WorktreeRemoved, "worktree_removed"),
        (P::BranchDeleted, "branch_deleted"),
        (P::Complete, "complete"),
        (P::Abandoned, "abandoned"),
        (P::Superseded, "superseded"),
    ];
    for (step, name) in all {
        assert_eq!(step.name(), name);
        r.step = step;
        assert_eq!(r.step(), shared_view(step), "{name}");
        r.step = P::Confirmed;
        r.set_step(shared_view(step));
        assert_eq!(r.step, step, "{name}");
    }
}

#[test]
fn promotion_remote_delete_is_guarded_by_the_context_child_publication() {
    use crate::repository::{PromotionExpectation, promotion::record};
    let mut db = rusqlite::Connection::open_in_memory().unwrap();
    crate::repository::discovery::migrate_registry(&mut db).unwrap();
    db.execute("INSERT INTO repositories(id,root_path,enabled_at,accessibility,refresh_required) VALUES(1,'/fixture',123,'accessible',0)", []).unwrap();
    let tx = db.transaction().unwrap();
    let mut r = record::begin_record(
        &tx,
        1,
        OperationId::new(),
        &item(),
        PromotionExpectation::from_bytes([7; 32]),
        Some("origin"),
    )
    .unwrap();
    let tracking = Oid::from_str(&"1".repeat(40)).unwrap();
    let published = Oid::from_str(&"2".repeat(40)).unwrap();
    r.remote_context_oid = Some(tracking);
    // Promotion is unchanged: always the child's verified publication, and
    // no guard without one, whatever tracking evidence the record holds.
    assert_eq!(r.remote_delete_guard(Some(published)), Some(published));
    assert_eq!(r.remote_delete_guard(None), None);
}

#[test]
fn promotion_remote_deletion_rule_is_unchanged() {
    use crate::repository::{PromotionExpectation, promotion::record};
    let mut db = rusqlite::Connection::open_in_memory().unwrap();
    crate::repository::discovery::migrate_registry(&mut db).unwrap();
    db.execute("INSERT INTO repositories(id,root_path,enabled_at,accessibility,refresh_required) VALUES(1,'/fixture',123,'accessible',0)", []).unwrap();
    let tx = db.transaction().unwrap();
    let mut r = record::begin_record(
        &tx,
        1,
        OperationId::new(),
        &item(),
        PromotionExpectation::from_bytes([7; 32]),
        Some("origin"),
    )
    .unwrap();
    // Cycle 09: every remote promotion deletes, except an already-contained
    // one with no tracking context.
    r.integration_kind = Some(IntegrationKind::Merged);
    assert!(r.remote_deletion_required());
    r.integration_kind = Some(IntegrationKind::AlreadyContained);
    assert!(!r.remote_deletion_required());
    r.remote_context_oid = Some(Oid::from_str(&"1".repeat(40)).unwrap());
    assert!(r.remote_deletion_required());
    r.remote_name = None;
    assert!(!r.remote_deletion_required());
}
