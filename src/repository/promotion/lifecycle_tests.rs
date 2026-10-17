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
    };
    assert_eq!(
        DocumentPromotionLifecycle
            .pre_integration(&mut stage)
            .unwrap(),
        PreIntegration::None
    );
}
