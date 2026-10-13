use super::super::{
    AutomaticBackoff, PollingInterval, RemoteOutcomeCategory, RemotePollInvocation,
    RemotePollingConfiguration,
};
use super::*;
use crate::repository::{REGISTRY_FILE, RepositoryService};
use rusqlite::{Connection, params};
use time::OffsetDateTime;

fn at(seconds: i64) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(seconds).unwrap()
}

fn policy(
    enabled: bool,
    paused: bool,
    suspended: bool,
    interval: u64,
) -> RemotePollingConfiguration {
    RemotePollingConfiguration::new(
        enabled,
        paused,
        PollingInterval::from_seconds(interval).unwrap(),
        None,
        suspended,
    )
}

#[test]
fn eligibility_policy_precedence_wins_over_due_and_future_deadlines() {
    for next in [None, Some(at(999)), Some(at(100_000))] {
        for (policy, reason) in [
            (policy(false, true, true, 300), PollIneligibility::Disabled),
            (policy(true, true, true, 300), PollIneligibility::Paused),
            (
                policy(true, false, true, 300),
                PollIneligibility::RecoverySuspended,
            ),
        ] {
            assert_eq!(
                eligibility(&policy, next, at(1000)),
                AutomaticPollEligibility::NotEligible {
                    reason,
                    next_eligible_at: next
                }
            );
        }
    }
}

#[test]
fn empty_past_and_equal_deadlines_are_due_and_future_is_not_due() {
    let policy = RemotePollingConfiguration::default();
    for next in [None, Some(at(999)), Some(at(1000))] {
        assert_eq!(
            eligibility(&policy, next, at(1000)),
            AutomaticPollEligibility::Eligible
        );
    }
    assert_eq!(
        eligibility(&policy, Some(at(1001)), at(1000)),
        AutomaticPollEligibility::NotEligible {
            reason: PollIneligibility::NotDue,
            next_eligible_at: Some(at(1001))
        }
    );
}

#[test]
fn rollback_clamp_uses_larger_of_interval_and_maximum_backoff() {
    for (interval, bound) in [(60, 1900), (300, 1900), (900, 1900), (3600, 4600)] {
        let policy = policy(true, false, false, interval);
        assert_eq!(
            eligibility(&policy, Some(at(100_000)), at(1000)),
            AutomaticPollEligibility::NotEligible {
                reason: PollIneligibility::NotDue,
                next_eligible_at: Some(at(bound))
            }
        );
        assert_eq!(
            eligibility(&policy, Some(at(bound)), at(1100)),
            AutomaticPollEligibility::NotEligible {
                reason: PollIneligibility::NotDue,
                next_eligible_at: Some(at(bound))
            }
        );
        assert_eq!(
            eligibility(&policy, Some(at(bound)), at(bound)),
            AutomaticPollEligibility::Eligible
        );
    }
}

#[test]
fn next_eligibility_result_table_covers_every_category_and_invocation() {
    let policy = RemotePollingConfiguration::default()
        .with_automatic_backoff(AutomaticBackoff::from_seconds(120).unwrap());
    let cases = [
        (
            RemoteOutcomeCategory::Completed,
            Some(at(1300)),
            Some(at(1300)),
        ),
        (
            RemoteOutcomeCategory::TransportUnavailable,
            Some(at(1240)),
            None,
        ),
        (
            RemoteOutcomeCategory::ProtocolRejected,
            Some(at(1240)),
            None,
        ),
        (
            RemoteOutcomeCategory::SelectedKeyUnavailable,
            Some(at(1300)),
            None,
        ),
        (RemoteOutcomeCategory::UnlockRequired, Some(at(1300)), None),
        (
            RemoteOutcomeCategory::HostApprovalRequired,
            Some(at(1300)),
            None,
        ),
        (
            RemoteOutcomeCategory::ConfigurationRequired,
            Some(at(1300)),
            None,
        ),
        (
            RemoteOutcomeCategory::RepositoryUnavailable,
            Some(at(1300)),
            None,
        ),
        (RemoteOutcomeCategory::Cancelled, None, None),
    ];
    for (category, automatic, explicit) in cases {
        for (invocation, expected) in [
            (RemotePollInvocation::Automatic, automatic),
            (RemotePollInvocation::Explicit, explicit),
        ] {
            assert_eq!(
                next_eligible(category, invocation, at(1000), &policy),
                expected,
                "{category:?} {invocation:?}"
            );
        }
    }
}

#[test]
fn network_retry_deadlines_start_at_sixty_double_and_cap_at_nine_hundred() {
    for category in [
        RemoteOutcomeCategory::TransportUnavailable,
        RemoteOutcomeCategory::ProtocolRejected,
    ] {
        let mut policy = RemotePollingConfiguration::default();
        for delay in [60, 120, 240, 480, 900, 900] {
            assert_eq!(
                next_eligible(category, RemotePollInvocation::Automatic, at(1000), &policy),
                Some(at(1000 + delay))
            );
            assert_eq!(
                next_eligible(category, RemotePollInvocation::Explicit, at(1000), &policy),
                None
            );
            policy = policy
                .with_automatic_backoff(AutomaticBackoff::from_seconds(delay as u64).unwrap());
        }
    }
}

fn fixture() -> (
    tempfile::TempDir,
    tempfile::TempDir,
    RepositoryService,
    Connection,
) {
    let data = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    git2::Repository::init(root.path()).unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let connection = Connection::open(data.path().join(REGISTRY_FILE)).unwrap();
    connection.execute("INSERT INTO repositories(root_path,enabled_at,accessibility,refresh_required) VALUES(?1,123,'accessible',0)", [root.path().canonicalize().unwrap().to_str().unwrap()]).unwrap();
    connection.execute_batch("CREATE TABLE eligibility_writes(n INTEGER NOT NULL); INSERT INTO eligibility_writes VALUES(0); CREATE TRIGGER count_eligibility_writes AFTER UPDATE ON remote_polling_state BEGIN UPDATE eligibility_writes SET n=n+1; END;").unwrap();
    (data, root, service, connection)
}

#[test]
fn stored_clamp_is_written_once_and_counts_down_without_reserving() {
    let (_data, root, service, connection) = fixture();
    connection.execute_batch("UPDATE remote_polling_state SET next_eligible_at=100000; UPDATE eligibility_writes SET n=0").unwrap();
    assert_eq!(
        automatic_poll_eligibility_at(&service, root.path(), at(1000)).unwrap(),
        AutomaticPollEligibility::NotEligible {
            reason: PollIneligibility::NotDue,
            next_eligible_at: Some(at(1900))
        }
    );
    assert_eq!(
        automatic_poll_eligibility_at(&service, root.path(), at(1100)).unwrap(),
        AutomaticPollEligibility::NotEligible {
            reason: PollIneligibility::NotDue,
            next_eligible_at: Some(at(1900))
        }
    );
    assert_eq!(
        automatic_poll_eligibility_at(&service, root.path(), at(1900)).unwrap(),
        AutomaticPollEligibility::Eligible
    );
    assert_eq!(
        connection
            .query_row("SELECT n FROM eligibility_writes", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM remote_operation_records", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap(),
        0
    );
    let snapshot = service.remote_snapshot(root.path()).unwrap();
    assert_eq!(snapshot.latest_attempt_at(), None);
    assert_eq!(snapshot.next_eligible_at(), Some(at(1900)));
}

#[test]
fn ineligible_policy_writes_nothing_even_when_deadline_needs_clamping() {
    let (_data, root, service, connection) = fixture();
    for (enabled, paused, suspended, reason) in [
        (false, true, true, PollIneligibility::Disabled),
        (true, true, true, PollIneligibility::Paused),
        (true, false, true, PollIneligibility::RecoverySuspended),
    ] {
        connection.execute("UPDATE remote_polling_state SET enabled=?1,paused=?2,recovery_suspended=?3,next_eligible_at=100000", params![enabled, paused, suspended]).unwrap();
        connection
            .execute_batch("UPDATE eligibility_writes SET n=0")
            .unwrap();
        assert_eq!(
            automatic_poll_eligibility_at(&service, root.path(), at(1000)).unwrap(),
            AutomaticPollEligibility::NotEligible {
                reason,
                next_eligible_at: Some(at(100000))
            }
        );
        assert_eq!(
            connection
                .query_row("SELECT n FROM eligibility_writes", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}

#[test]
fn public_eligibility_is_due_without_an_endpoint_and_does_not_write() {
    let (_data, root, service, connection) = fixture();
    assert_eq!(
        service.automatic_poll_eligibility(root.path()).unwrap(),
        AutomaticPollEligibility::Eligible
    );
    assert_eq!(
        connection
            .query_row("SELECT n FROM eligibility_writes", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}
