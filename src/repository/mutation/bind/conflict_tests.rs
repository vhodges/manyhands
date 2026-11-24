//! What `conflict resolve` answers of the synchronization it continued
//! (`published`), from what the synchronization's binding said of it.

use super::*;

/// The continuation of a lifecycle's child that completed while the index
/// does not hold its resolved context is answered by the synchronization's
/// binding as owing only a refresh (`owed_to_its_parent`, decision 8):
/// that indexing is its parent's exit's (Task 22). The resolution keeps that
/// answer, `discovery_pending` offering `index.refresh` and no resume of the
/// synchronization, final, with its own data. Any other `discovery_pending`
/// still offers the synchronization's resume, which owes its hand-off.
#[test]
fn a_continued_lifecycle_child_owing_its_index_offers_a_refresh_and_no_resume() {
    let synchronization = OperationId::new();
    let commit = git2::Oid::from_str("0123456789abcdef0123456789abcdef01234567").unwrap();
    let continued = |recovery: Vec<RecoveryAction>| Ran {
        answer: Answer {
            effects: Effects {
                publication: PublicationEffect::Published,
                discovery: DiscoveryEffect::Pending,
                commit_oid: Some(commit.to_string()),
                ..Effects::not_requested()
            },
            recovery,
            ..Answer::stopped(ResultCode::DiscoveryPending)
        },
        standing: Standing::Final,
    };
    let actions = |ran: &Ran| {
        ran.answer
            .recovery
            .iter()
            .map(|action| action.action)
            .collect::<Vec<_>>()
    };

    let owed = published(
        synchronization,
        commit,
        continued(vec![outcome::refresh_the_root("/repository")]),
        None,
    );
    assert_eq!(owed.answer.code, ResultCode::DiscoveryPending);
    assert_eq!(owed.answer.effects.discovery, DiscoveryEffect::Pending);
    assert_eq!(
        owed.answer.effects.publication,
        PublicationEffect::Published
    );
    assert_eq!(actions(&owed), [RecoveryActionKind::IndexRefresh]);
    assert!(matches!(owed.standing, Standing::Final));
    assert_eq!(
        owed.answer.data,
        Some(data(
            synchronization,
            Some(commit),
            PublicationEffect::Published,
            None
        ))
    );

    let pending = published(synchronization, commit, continued(Vec::new()), None);
    assert_eq!(pending.answer.code, ResultCode::DiscoveryPending);
    assert_eq!(actions(&pending), [RecoveryActionKind::OperationResume]);
}
