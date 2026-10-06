use std::{convert::Infallible, str::FromStr};

use git2::Oid;

use super::{
    CleanIntegrationError, CleanIntegrationPlan, RemoteRefPlan, RemoteRefPlanError,
    plan_clean_integration,
};
use crate::{
    canonical::ItemId,
    repository::{
        AuthoringKind, OperationId, RemoteOperationAction, RemotePublicationEvidence,
        SynchronizationTarget, SynchronizeRemoteRequest,
    },
};

const ITEM_ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";

fn context() -> SynchronizationTarget {
    SynchronizationTarget::Context {
        kind: AuthoringKind::Ticket,
        item_id: ItemId::from_str(ITEM_ID).unwrap(),
    }
}

#[test]
fn deliberate_refspecs_are_exact_tracking_fetches_and_ordinary_pushes() {
    for (remote, primary) in [("origin", "main"), ("team/upstream", "release/v2")] {
        let plan = RemoteRefPlan::from_configuration(remote, primary).unwrap();
        let primary_ref = format!("refs/heads/{primary}");
        let primary_fetch = format!("+{primary_ref}:refs/remotes/{remote}/{primary}");
        assert_eq!(plan.primary_fetch_refspec(), primary_fetch);
        assert_eq!(
            plan.primary_push_refspec(),
            format!("{primary_ref}:{primary_ref}")
        );
        for (kind, segment) in [
            (AuthoringKind::Document, "document"),
            (AuthoringKind::Ticket, "ticket"),
        ] {
            let id = ItemId::from_str(ITEM_ID).unwrap();
            let branch = format!("manyhands/{segment}/{id}");
            let context_ref = format!("refs/heads/{branch}");
            let fetches = plan.context_fetch_refspec(kind, &id);
            assert_eq!(
                fetches,
                [
                    primary_fetch.clone(),
                    format!("+{context_ref}:refs/remotes/{remote}/{branch}")
                ]
            );
            let push = plan.context_push_refspec(kind, &id);
            assert_eq!(push, format!("{context_ref}:{context_ref}"));
            for mapping in [push, plan.primary_push_refspec()] {
                assert!(!mapping.starts_with('+'));
                assert!(!mapping.contains('*'));
                let (source, destination) = mapping.split_once(':').unwrap();
                assert!(!source.is_empty());
                assert_eq!(source, destination);
                assert!(git2::Reference::is_valid_name(source));
            }
            for mapping in fetches {
                assert!(mapping.starts_with('+'));
                assert!(!mapping.contains('*'));
                let (source, destination) = mapping[1..].split_once(':').unwrap();
                assert!(source.starts_with("refs/heads/"));
                assert!(destination.starts_with("refs/remotes/"));
                assert!(git2::Reference::is_valid_name(source));
                assert!(git2::Reference::is_valid_name(destination));
            }
        }
    }
}

#[test]
fn deliberate_ref_inputs_reject_invalid_and_reserved_names() {
    for invalid in [
        "", "-force", ".hidden", "x.", "x/", "@", "x..y", "x@{y", "x y", "x\ny", "x\0y", "x~y",
        "x^y", "x:y", "x?y", "x*y", "x[y", "x\\y", "x//y", "x/.y", "x/y.lock",
    ] {
        assert_eq!(
            RemoteRefPlan::from_configuration(invalid, "main"),
            Err(RemoteRefPlanError::InvalidRemoteName),
            "{invalid:?}"
        );
        assert_eq!(
            RemoteRefPlan::from_configuration("origin", invalid),
            Err(RemoteRefPlanError::InvalidPrimaryBranch),
            "{invalid:?}"
        );
    }
    for reserved in [
        "HEAD",
        "manyhands/document",
        "manyhands/ticket",
        "manyhands/document/child",
        "manyhands/ticket/child",
    ] {
        assert_eq!(
            RemoteRefPlan::from_configuration("origin", reserved),
            Err(RemoteRefPlanError::InvalidPrimaryBranch)
        );
    }
    for invalid in [
        "",
        "01arz3ndektsv4rrffq69g5fav",
        "01ARZ3NDEKTSV4RRFFQ69G5FAV/nested",
        "*:refs/heads/evil",
    ] {
        assert!(ItemId::from_str(invalid).is_err());
    }
}

#[test]
fn synchronization_request_targets_derive_only_the_selected_operation() {
    let plan = RemoteRefPlan::from_configuration("origin", "release/v2").unwrap();
    for target in [
        SynchronizationTarget::Primary,
        context(),
        SynchronizationTarget::Context {
            kind: AuthoringKind::Document,
            item_id: ItemId::from_str(ITEM_ID).unwrap(),
        },
    ] {
        let request = SynchronizeRemoteRequest {
            root: "canonical-root".into(),
            operation_id: OperationId::new(),
            target: target.clone(),
            approval: None,
            restart: false,
        };
        let derived = request.target.operation_target(&plan);
        assert_eq!(derived.primary_ref(), plan.primary());
        match target {
            SynchronizationTarget::Primary => {
                assert_eq!(derived.action(), RemoteOperationAction::SynchronizePrimary);
                assert_eq!(derived.local_branch(), Some("release/v2"));
                assert_eq!(derived.context_ref(), None);
                assert_eq!(derived.item(), None);
            }
            SynchronizationTarget::Context { kind, item_id } => {
                assert_eq!(derived.action(), RemoteOperationAction::SynchronizeContext);
                assert_eq!(derived.item(), Some((kind, &item_id)));
                assert_eq!(derived.context_ref(), Some(&plan.context(kind, &item_id)));
                assert_eq!(
                    derived.local_branch(),
                    Some(
                        format!(
                            "manyhands/{}/{item_id}",
                            match kind {
                                AuthoringKind::Document => "document",
                                AuthoringKind::Ticket => "ticket",
                            }
                        )
                        .as_str()
                    )
                );
            }
        }
    }
}

fn oid(node: u8) -> Oid {
    Oid::from_bytes(&[node; 20]).unwrap()
}

// Two branches: 1 -> 2 -> 3 -> 4 and 1 -> 5 -> 6.
fn ancestor(older: Oid, newer: Oid) -> Result<bool, Infallible> {
    Ok([vec![1, 2, 3, 4], vec![1, 5, 6]].iter().any(|chain| {
        let old = chain.iter().position(|node| oid(*node) == older);
        let new = chain.iter().position(|node| oid(*node) == newer);
        matches!((old, new), (Some(a), Some(b)) if a < b)
    }))
}

#[test]
fn clean_primary_graph_table() {
    for (local, remote, expected) in [
        (
            2,
            Some(2),
            Ok(CleanIntegrationPlan {
                final_oid: oid(2),
                local_update: false,
                push_needed: false,
            }),
        ),
        (
            2,
            Some(3),
            Ok(CleanIntegrationPlan {
                final_oid: oid(3),
                local_update: true,
                push_needed: false,
            }),
        ),
        (
            3,
            Some(2),
            Ok(CleanIntegrationPlan {
                final_oid: oid(3),
                local_update: false,
                push_needed: true,
            }),
        ),
        (2, Some(5), Err(CleanIntegrationError::MergeRequired)),
        (5, Some(2), Err(CleanIntegrationError::MergeRequired)),
        (2, None, Err(CleanIntegrationError::PrimaryMissing)),
    ] {
        assert_eq!(
            plan_clean_integration(
                &SynchronizationTarget::Primary,
                oid(local),
                remote.map(oid),
                None,
                RemotePublicationEvidence::HistoryUnknown,
                ancestor
            ),
            expected
        );
    }
}

#[test]
fn clean_context_graph_table_evaluates_both_relations_virtually() {
    // local, context tracking, primary tracking, final, local update, push
    for (local, remote, primary, final_node, update, push) in [
        (2, 2, 2, 2, false, false),
        (2, 3, 1, 3, true, false),
        (3, 2, 1, 3, false, true),
        (2, 2, 3, 3, true, true),
        (3, 3, 2, 3, false, false),
        (2, 3, 4, 4, true, true),
        (3, 2, 4, 4, true, true),
        (1, 2, 2, 2, true, false),
        (4, 3, 2, 4, false, true),
    ] {
        assert_eq!(
            plan_clean_integration(
                &context(),
                oid(local),
                Some(oid(primary)),
                Some(oid(remote)),
                RemotePublicationEvidence::ObservedPublished,
                ancestor
            ),
            Ok(CleanIntegrationPlan {
                final_oid: oid(final_node),
                local_update: update,
                push_needed: push
            }),
            "local={local}, remote={remote}, primary={primary}"
        );
    }
    for (local, remote, primary) in [(2, 5, 1), (5, 2, 1), (2, 3, 5), (3, 2, 5), (5, 6, 2)] {
        assert_eq!(
            plan_clean_integration(
                &context(),
                oid(local),
                Some(oid(primary)),
                Some(oid(remote)),
                RemotePublicationEvidence::ObservedPublished,
                ancestor
            ),
            Err(CleanIntegrationError::MergeRequired)
        );
    }
}

#[test]
fn virtual_context_divergence_returns_no_intermediate_update() {
    let local = oid(2);
    let mut comparisons = Vec::new();
    let result = plan_clean_integration(
        &context(),
        local,
        Some(oid(5)),
        Some(oid(3)),
        RemotePublicationEvidence::ObservedPublished,
        |older, newer| {
            comparisons.push((older, newer));
            ancestor(older, newer)
        },
    );
    assert_eq!(result, Err(CleanIntegrationError::MergeRequired));
    assert_eq!(
        comparisons,
        [(oid(2), oid(3)), (oid(3), oid(5)), (oid(5), oid(3))]
    );
    assert_eq!(local, oid(2));
}

#[test]
fn absent_context_requires_publication_evidence_and_primary() {
    use RemotePublicationEvidence::*;
    for (primary, evidence, expected) in [
        (
            Some(1),
            NeverPublished,
            Ok(CleanIntegrationPlan {
                final_oid: oid(2),
                local_update: false,
                push_needed: true,
            }),
        ),
        (
            Some(3),
            NeverPublished,
            Ok(CleanIntegrationPlan {
                final_oid: oid(3),
                local_update: true,
                push_needed: true,
            }),
        ),
        (
            Some(5),
            NeverPublished,
            Err(CleanIntegrationError::MergeRequired),
        ),
        (
            Some(1),
            ObservedPublished,
            Err(CleanIntegrationError::RemoteContextDeleted),
        ),
        (
            Some(1),
            HistoryUnknown,
            Err(CleanIntegrationError::HistoryUnknown),
        ),
        (
            None,
            NeverPublished,
            Err(CleanIntegrationError::PrimaryMissing),
        ),
    ] {
        assert_eq!(
            plan_clean_integration(
                &context(),
                oid(2),
                primary.map(oid),
                None,
                evidence,
                ancestor
            ),
            expected
        );
    }
    assert_eq!(
        plan_clean_integration(
            &context(),
            oid(2),
            None,
            Some(oid(3)),
            ObservedPublished,
            ancestor
        ),
        Err(CleanIntegrationError::PrimaryMissing)
    );
}

#[test]
fn present_context_uses_current_graph_even_when_prior_history_is_unknown() {
    let plan = plan_clean_integration(
        &context(),
        oid(2),
        Some(oid(1)),
        Some(oid(2)),
        RemotePublicationEvidence::HistoryUnknown,
        ancestor,
    )
    .unwrap();
    assert!(!plan.local_update);
    assert!(!plan.push_needed);
}

#[test]
fn equal_oids_need_no_ancestry_query_and_ancestry_failure_is_not_divergence() {
    let plan = plan_clean_integration(
        &context(),
        oid(2),
        Some(oid(2)),
        Some(oid(2)),
        RemotePublicationEvidence::ObservedPublished,
        |_, _| -> Result<bool, Infallible> { panic!("equal OIDs need no query") },
    )
    .unwrap();
    assert_eq!(plan.final_oid, oid(2));
    let result = plan_clean_integration(
        &SynchronizationTarget::Primary,
        oid(2),
        Some(oid(3)),
        None,
        RemotePublicationEvidence::NeverPublished,
        |_, _| Err("ancestry unavailable"),
    );
    assert_eq!(
        result,
        Err(CleanIntegrationError::Ancestry("ancestry unavailable"))
    );
}
