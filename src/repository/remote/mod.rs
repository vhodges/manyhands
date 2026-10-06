mod observation;
mod refs;
pub use observation::{ObservePublicationRemoteRequest, RemoteObservationError};
#[cfg(test)]
pub(crate) mod observation_tests;
pub(super) mod reservation;
pub(super) mod state;

pub use reservation::{
    RemoteOperationInspection, RemoteReservation, RemoteReservationOutcome, RemoteSafePointOutcome,
};

pub use refs::{RemoteRefClassification, RemoteRefPlan, RemoteRefPlanError, RemoteRefTarget};
pub use state::{
    AutomaticBackoff, PollingInterval, RemoteContextSnapshot, RemoteContextState,
    RemoteObservationOutcome, RemoteOperationAction, RemoteOperationPhase, RemoteOperationPriority,
    RemoteOperationSafePoint, RemoteOperationTarget, RemoteOperationTargetError,
    RemoteOutcomeCategory, RemotePollInvocation, RemotePollingConfiguration,
    RemotePollingValueError, RemotePublicationEvidence, RemoteRefObservation, RemoteSnapshot,
};

#[cfg(test)]
mod tests {
    use std::{str::FromStr, time::Duration};

    use git2::Oid;

    use super::*;
    use crate::{canonical::ItemId, repository::AuthoringKind};

    const DOCUMENT_ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
    const TICKET_ID: &str = "01BX5ZZKBKACTAV9WEVGEMMVRZ";

    #[test]
    fn ref_plan_derives_the_three_exact_forced_fetch_mappings() {
        let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();

        assert_eq!(
            plan.fetch_refspecs(),
            [
                "+refs/heads/main:refs/remotes/origin/main",
                "+refs/heads/manyhands/document/*:refs/remotes/origin/manyhands/document/*",
                "+refs/heads/manyhands/ticket/*:refs/remotes/origin/manyhands/ticket/*",
            ]
        );
        assert_eq!(plan.primary().remote_ref(), "refs/heads/main");
        assert_eq!(plan.primary().tracking_ref(), "refs/remotes/origin/main");
    }

    #[test]
    fn configuration_changes_rederive_remote_and_primary_names_independently() {
        let remote_changed = RemoteRefPlan::from_configuration("upstream", "main").unwrap();
        let primary_changed = RemoteRefPlan::from_configuration("origin", "release/v2").unwrap();

        assert_eq!(remote_changed.primary().remote_ref(), "refs/heads/main");
        assert_eq!(
            remote_changed.primary().tracking_ref(),
            "refs/remotes/upstream/main"
        );
        assert_eq!(
            primary_changed.primary().remote_ref(),
            "refs/heads/release/v2"
        );
        assert_eq!(
            primary_changed.primary().tracking_ref(),
            "refs/remotes/origin/release/v2"
        );
    }

    #[test]
    fn ref_plan_rejects_empty_injecting_and_control_character_names() {
        for remote in ["", "origin:evil", "origin\nnext", "origin\0next"] {
            assert_eq!(
                RemoteRefPlan::from_configuration(remote, "main"),
                Err(RemoteRefPlanError::InvalidRemoteName)
            );
        }
        for primary in [
            "",
            "HEAD",
            "main:refs/heads/evil",
            "main\nnext",
            "main\0next",
        ] {
            assert_eq!(
                RemoteRefPlan::from_configuration("origin", primary),
                Err(RemoteRefPlanError::InvalidPrimaryBranch)
            );
        }
    }

    #[test]
    fn ref_plan_rejects_primary_names_reserved_for_context_families() {
        for primary in [
            "manyhands/document",
            "manyhands/document/01ARZ3NDEKTSV4RRFFQ69G5FAV",
            "manyhands/ticket",
            "manyhands/ticket/01BX5ZZKBKACTAV9WEVGEMMVRZ",
        ] {
            assert_eq!(
                RemoteRefPlan::from_configuration("origin", primary),
                Err(RemoteRefPlanError::InvalidPrimaryBranch)
            );
        }
    }

    #[test]
    fn classifier_recognizes_primary_and_exact_document_and_ticket_ulids() {
        let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
        let document_id = ItemId::from_str(DOCUMENT_ID).unwrap();
        let ticket_id = ItemId::from_str(TICKET_ID).unwrap();

        assert_eq!(
            plan.classify_advertised_ref("refs/heads/main"),
            Some(RemoteRefClassification::Primary)
        );
        for (kind, id, remote_ref, tracking_ref) in [
            (
                AuthoringKind::Document,
                document_id,
                "refs/heads/manyhands/document/01ARZ3NDEKTSV4RRFFQ69G5FAV",
                "refs/remotes/origin/manyhands/document/01ARZ3NDEKTSV4RRFFQ69G5FAV",
            ),
            (
                AuthoringKind::Ticket,
                ticket_id,
                "refs/heads/manyhands/ticket/01BX5ZZKBKACTAV9WEVGEMMVRZ",
                "refs/remotes/origin/manyhands/ticket/01BX5ZZKBKACTAV9WEVGEMMVRZ",
            ),
        ] {
            let classification = plan.classify_advertised_ref(remote_ref).unwrap();
            assert_eq!(
                classification,
                RemoteRefClassification::RecognizedContext { kind, item_id: id }
            );
            assert!(!classification.establishes_canonical_tree_validity());
            assert_eq!(plan.tracking_ref_for(remote_ref).unwrap(), tracking_ref);
        }
    }

    #[test]
    fn classifier_excludes_unsupported_families_and_marks_bad_recognized_ids_malformed() {
        let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();

        for unsupported in [
            "refs/heads/manyhands/comment/01ARZ3NDEKTSV4RRFFQ69G5FAV",
            "refs/heads/manyhands/project/01ARZ3NDEKTSV4RRFFQ69G5FAV",
            "refs/tags/manyhands/document/01ARZ3NDEKTSV4RRFFQ69G5FAV",
            "refs/heads/feature/ordinary",
        ] {
            assert_eq!(plan.classify_advertised_ref(unsupported), None);
            assert_eq!(plan.tracking_ref_for(unsupported), None);
        }

        for (malformed, tracking_ref) in [
            (
                "refs/heads/manyhands/document/01ARZ3NDEKTSV4RRFFQ69G5FAV/nested",
                Some("refs/remotes/origin/manyhands/document/01ARZ3NDEKTSV4RRFFQ69G5FAV/nested"),
            ),
            (
                "refs/heads/manyhands/document/01ARZ3NDEKTSV4RRFFQ69G5FAV-suffix",
                Some("refs/remotes/origin/manyhands/document/01ARZ3NDEKTSV4RRFFQ69G5FAV-suffix"),
            ),
            (
                "refs/heads/manyhands/ticket/01arz3ndektsv4rrffq69g5fav",
                Some("refs/remotes/origin/manyhands/ticket/01arz3ndektsv4rrffq69g5fav"),
            ),
            ("refs/heads/manyhands/ticket/", None),
            (
                "refs/heads/manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAV\nnext",
                None,
            ),
        ] {
            assert_eq!(
                plan.classify_advertised_ref(malformed),
                Some(RemoteRefClassification::MalformedContext)
            );
            assert_eq!(plan.tracking_ref_for(malformed).as_deref(), tracking_ref);
        }
    }

    #[test]
    fn polling_defaults_and_interval_and_backoff_bounds_are_enforced() {
        let polling = RemotePollingConfiguration::default();
        assert!(polling.enabled());
        assert!(!polling.paused());
        assert_eq!(polling.interval(), Duration::from_secs(300));
        assert_eq!(polling.automatic_backoff(), None);

        assert_eq!(
            PollingInterval::from_seconds(60).unwrap().duration(),
            Duration::from_secs(60)
        );
        assert_eq!(
            PollingInterval::from_seconds(3600).unwrap().duration(),
            Duration::from_secs(3600)
        );
        assert!(PollingInterval::from_seconds(59).is_err());
        assert!(PollingInterval::from_seconds(3601).is_err());

        assert_eq!(
            AutomaticBackoff::from_seconds(60).unwrap().duration(),
            Duration::from_secs(60)
        );
        assert_eq!(
            AutomaticBackoff::from_seconds(900).unwrap().duration(),
            Duration::from_secs(900)
        );
        assert!(AutomaticBackoff::from_seconds(59).is_err());
        assert!(AutomaticBackoff::from_seconds(901).is_err());
    }

    #[test]
    fn explicit_observation_is_never_delayed_by_automatic_backoff() {
        let polling = RemotePollingConfiguration::default()
            .with_automatic_backoff(AutomaticBackoff::from_seconds(900).unwrap());

        assert_eq!(
            polling.delay_for(RemotePollInvocation::Automatic),
            Some(Duration::from_secs(900))
        );
        assert_eq!(polling.delay_for(RemotePollInvocation::Explicit), None);
    }

    #[test]
    fn outcome_categories_render_only_fixed_redacted_statuses() {
        let cases = [
            (RemoteOutcomeCategory::Completed, "completed"),
            (
                RemoteOutcomeCategory::ConfigurationRequired,
                "configuration required",
            ),
            (
                RemoteOutcomeCategory::SelectedKeyUnavailable,
                "selected key unavailable",
            ),
            (RemoteOutcomeCategory::UnlockRequired, "unlock required"),
            (
                RemoteOutcomeCategory::HostApprovalRequired,
                "host approval required",
            ),
            (
                RemoteOutcomeCategory::TransportUnavailable,
                "transport unavailable",
            ),
            (RemoteOutcomeCategory::ProtocolRejected, "protocol rejected"),
            (RemoteOutcomeCategory::Cancelled, "cancelled"),
            (
                RemoteOutcomeCategory::RepositoryUnavailable,
                "repository unavailable",
            ),
        ];

        for (category, expected) in cases {
            assert_eq!(category.to_string(), expected);
            assert!(!format!("{category:?}").contains("secret"));
        }
    }

    #[test]
    fn operation_targets_are_derived_and_keep_unrelated_fields_absent() {
        let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
        let poll = RemoteOperationTarget::for_poll(&plan);
        assert_eq!(poll.remote_name(), "origin");
        assert_eq!(poll.primary_ref(), plan.primary());
        assert_eq!(poll.context_ref(), None);
        assert_eq!(poll.item(), None);
        assert_eq!(poll.local_branch(), None);

        let item_id = ItemId::from_str(TICKET_ID).unwrap();
        let context = RemoteOperationTarget::for_context(
            &plan,
            RemoteOperationAction::SynchronizeContext,
            AuthoringKind::Ticket,
            item_id.clone(),
        )
        .unwrap();
        assert_eq!(context.item(), Some((AuthoringKind::Ticket, &item_id)));
        assert_eq!(
            context.local_branch(),
            Some("manyhands/ticket/01BX5ZZKBKACTAV9WEVGEMMVRZ")
        );
        assert_eq!(
            context.context_ref().unwrap().tracking_ref(),
            "refs/remotes/origin/manyhands/ticket/01BX5ZZKBKACTAV9WEVGEMMVRZ"
        );
        assert!(
            RemoteOperationTarget::for_context(
                &plan,
                RemoteOperationAction::Poll,
                AuthoringKind::Ticket,
                item_id,
            )
            .is_err()
        );
    }

    #[test]
    fn operation_targets_do_not_retain_or_render_raw_worktree_paths() {
        let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
        let target = RemoteOperationTarget::for_context(
            &plan,
            RemoteOperationAction::SynchronizeContext,
            AuthoringKind::Ticket,
            ItemId::from_str(TICKET_ID).unwrap(),
        )
        .unwrap();

        assert!(!format!("{target:?}").contains("worktree"));
    }

    #[test]
    fn observation_and_snapshot_contracts_retain_only_ref_oid_and_redacted_state() {
        let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
        let advertised = Oid::from_str("1111111111111111111111111111111111111111").unwrap();
        let tracking = Oid::from_str("2222222222222222222222222222222222222222").unwrap();
        let observation = RemoteRefObservation::from_advertisement(
            &plan,
            "refs/heads/manyhands/document/01ARZ3NDEKTSV4RRFFQ69G5FAV",
            advertised,
            Some(tracking),
        )
        .unwrap();
        assert_eq!(observation.advertised_oid(), advertised);
        assert_eq!(observation.tracking_oid(), Some(tracking));

        let snapshot = RemoteSnapshot::new(
            RemotePollingConfiguration::default(),
            Some(RemoteOutcomeCategory::Completed),
            vec![observation],
            Vec::new(),
        );
        let outcome = RemoteObservationOutcome::new(RemoteOutcomeCategory::Completed, snapshot);
        assert_eq!(outcome.category(), RemoteOutcomeCategory::Completed);
        assert_eq!(outcome.snapshot().observations().len(), 1);
    }

    #[test]
    fn observation_does_not_retain_unvalidated_ref_text() {
        let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
        let observation = RemoteRefObservation::from_advertisement(
            &plan,
            "refs/heads/manyhands/document/private-marker\nnext",
            Oid::from_str("1111111111111111111111111111111111111111").unwrap(),
            None,
        )
        .unwrap();

        assert_eq!(
            observation.classification(),
            &RemoteRefClassification::MalformedContext
        );
        assert_eq!(observation.remote_ref(), None);
        assert_eq!(observation.tracking_ref(), None);
        assert!(!format!("{observation:?}").contains("private-marker"));
    }

    #[test]
    fn context_snapshot_accepts_only_validated_ref_targets() {
        let snapshot = RemoteContextSnapshot::new(
            None,
            None,
            None,
            Some(Oid::from_str("1111111111111111111111111111111111111111").unwrap()),
            None,
            RemotePublicationEvidence::NeverPublished,
            RemoteContextState::Malformed,
        );

        assert_eq!(snapshot.target(), None);
        assert_eq!(snapshot.remote_ref(), None);
        assert_eq!(snapshot.tracking_ref(), None);
    }

    #[test]
    fn operation_contract_names_cover_lifecycle_and_durable_safe_points() {
        let actions = [
            RemoteOperationAction::Poll,
            RemoteOperationAction::SynchronizeContext,
            RemoteOperationAction::SynchronizePrimary,
            RemoteOperationAction::Promote,
            RemoteOperationAction::Close,
        ];
        let priorities = [
            RemoteOperationPriority::Poll,
            RemoteOperationPriority::Manual,
        ];
        let phases = [
            RemoteOperationPhase::Reserved,
            RemoteOperationPhase::Advertising,
            RemoteOperationPhase::Persisting,
            RemoteOperationPhase::Completed,
            RemoteOperationPhase::Interrupted,
            RemoteOperationPhase::Cancelled,
            RemoteOperationPhase::Failed,
        ];
        let safe_points = [
            RemoteOperationSafePoint::BeforeTransport,
            RemoteOperationSafePoint::AfterAdvertisement,
            RemoteOperationSafePoint::BetweenObservations,
            RemoteOperationSafePoint::BeforeBatchCommit,
            RemoteOperationSafePoint::AfterBatchCommit,
            RemoteOperationSafePoint::BeforeLocalMutation,
        ];

        assert_eq!(actions.len(), 5);
        assert_eq!(priorities.len(), 2);
        assert_eq!(phases.len(), 7);
        assert_eq!(safe_points.len(), 6);
    }
}
