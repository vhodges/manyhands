use git2::Oid;
use rusqlite::Connection;

use super::*;
use crate::repository::{
    REGISTRY_FILE,
    remote::{RemoteRefObservation, RemoteRefPlan, state::ConfigurationChange},
};

const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";

fn plan(remote: &str) -> RemoteRefPlan {
    RemoteRefPlan::from_configuration(remote, "main").unwrap()
}

fn commit(repository: &git2::Repository, parent: Option<Oid>, message: &str) -> Oid {
    let tree = repository
        .find_tree(repository.treebuilder(None).unwrap().write().unwrap())
        .unwrap();
    let signature =
        git2::Signature::new("Fixture", "fixture@example.invalid", &git2::Time::new(0, 0)).unwrap();
    let parent = parent.map(|oid| repository.find_commit(oid).unwrap());
    repository
        .commit(
            None,
            &signature,
            &signature,
            message,
            &tree,
            &parent.iter().collect::<Vec<_>>(),
        )
        .unwrap()
}

/// The registry-level remote read; full discovery needs real worktrees.
fn snapshot(service: &RepositoryService, root: &std::path::Path) -> RemoteSnapshot {
    state::with_transaction(service, root, |tx, id| state::read_snapshot(tx, id)).unwrap()
}

/// Where the local branch and the advertised ref sit, as commits above a shared
/// base: `(local, advertised)` counts of extra commits, `None` for absent.
#[derive(Clone, Copy)]
struct Layout {
    local_extra: usize,
    advertised_extra: Option<usize>,
    /// The poll fast-forwarded the local branch onto the advertised tip.
    local_on_remote: bool,
    /// The remote's tip isn't in the local object database (never fetched).
    advertised_missing: bool,
}

struct Want {
    resolved: bool,
    relation: Option<RemoteHistoryRelation>,
    counts: Option<(usize, usize)>,
    publish_and_release: bool,
    disposition: Option<PollDisposition>,
}

#[test]
fn history_reports_unknown_and_resolved_contexts_with_relations_and_choices() {
    use RemoteHistoryRelation::*;
    let rows: [(&str, bool, Layout, Want); 8] = [
        (
            "absent and unknown",
            false,
            Layout {
                local_extra: 0,
                advertised_extra: None,
                local_on_remote: false,
                advertised_missing: false,
            },
            Want {
                resolved: false,
                relation: None,
                counts: None,
                publish_and_release: true,
                disposition: None,
            },
        ),
        (
            "present and same",
            false,
            Layout {
                local_extra: 0,
                advertised_extra: Some(0),
                local_on_remote: false,
                advertised_missing: false,
            },
            Want {
                resolved: true,
                relation: Some(Same),
                counts: Some((0, 0)),
                publish_and_release: false,
                disposition: Some(PollDisposition::Current),
            },
        ),
        (
            "local ahead by two",
            false,
            Layout {
                local_extra: 2,
                advertised_extra: Some(0),
                local_on_remote: false,
                advertised_missing: false,
            },
            Want {
                resolved: true,
                relation: Some(LocalAhead),
                counts: Some((2, 0)),
                publish_and_release: false,
                disposition: Some(PollDisposition::LocalAhead),
            },
        ),
        (
            "remote ahead by one",
            false,
            Layout {
                local_extra: 0,
                advertised_extra: Some(1),
                local_on_remote: false,
                advertised_missing: false,
            },
            Want {
                resolved: true,
                relation: Some(RemoteAhead),
                counts: Some((0, 1)),
                publish_and_release: false,
                disposition: Some(PollDisposition::Dirty),
            },
        ),
        (
            "diverged one and one",
            false,
            Layout {
                local_extra: 1,
                advertised_extra: Some(1),
                local_on_remote: false,
                advertised_missing: false,
            },
            Want {
                resolved: true,
                relation: Some(Diverged),
                counts: Some((1, 1)),
                publish_and_release: false,
                disposition: Some(PollDisposition::Diverged),
            },
        ),
        (
            "cache loss with no row",
            true,
            Layout {
                local_extra: 0,
                advertised_extra: None,
                local_on_remote: false,
                advertised_missing: false,
            },
            Want {
                resolved: false,
                relation: None,
                counts: None,
                publish_and_release: true,
                disposition: None,
            },
        ),
        (
            "fast-forwarded by the poll",
            false,
            Layout {
                local_extra: 0,
                advertised_extra: Some(1),
                local_on_remote: true,
                advertised_missing: false,
            },
            Want {
                resolved: true,
                relation: Some(Same),
                counts: Some((0, 0)),
                publish_and_release: false,
                disposition: Some(PollDisposition::FastForwarded),
            },
        ),
        (
            "advertised object missing locally",
            false,
            Layout {
                local_extra: 1,
                advertised_extra: Some(0),
                local_on_remote: false,
                advertised_missing: true,
            },
            Want {
                resolved: true,
                relation: None,
                counts: None,
                publish_and_release: false,
                disposition: Some(PollDisposition::NotReached),
            },
        ),
    ];
    for (name, cache_loss, layout, want) in rows {
        let data = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        let service = RepositoryService::open_at(data.path()).unwrap();
        Connection::open(data.path().join(REGISTRY_FILE))
            .unwrap()
            .execute_batch(&format!(
                "INSERT INTO repositories(root_path,enabled_at,accessibility,refresh_required) VALUES ('{}',123,'accessible',0);
                 INSERT INTO contexts(repository_id,kind,branch,worktree_path,item_id) VALUES(1,'active','manyhands/ticket/{ITEM}','/active','{ITEM}');",
                root.path().to_str().unwrap()
            ))
            .unwrap();
        let repository = git2::Repository::init(root.path()).unwrap();
        let base = commit(&repository, None, "base");
        let chain = |tag: &str, extra: usize| {
            (0..extra).fold(base, |parent, n| {
                commit(&repository, Some(parent), &format!("{tag}{n}"))
            })
        };
        let advertised = if layout.advertised_missing {
            Some(Oid::from_str(&"ab".repeat(20)).unwrap())
        } else {
            layout.advertised_extra.map(|extra| chain("remote", extra))
        };
        let local = if layout.local_on_remote {
            advertised.unwrap()
        } else {
            chain("local", layout.local_extra)
        };
        repository
            .reference(
                &format!("refs/heads/manyhands/ticket/{ITEM}"),
                local,
                true,
                "fixture",
            )
            .unwrap();

        let origin = plan("origin");
        let upstream = plan("upstream");
        let generation = state::with_transaction(&service, root.path(), |tx, id| {
            state::configure(tx, id, Some(&origin), ConfigurationChange::Plan)
        })
        .unwrap();
        if !cache_loss {
            // A real earlier publication, forgotten by a change of remote.
            let published = RemoteRefObservation::from_advertisement(
                &origin,
                &format!("refs/heads/manyhands/ticket/{ITEM}"),
                base,
                None,
            )
            .unwrap();
            state::with_transaction(&service, root.path(), |tx, id| {
                state::complete_batch(tx, id, &origin, generation, &[published], 100)
            })
            .unwrap();
        }
        let generation = if cache_loss {
            Connection::open(data.path().join(REGISTRY_FILE))
                .unwrap()
                .execute("UPDATE remote_polling_state SET history_unknown=1", [])
                .unwrap();
            generation
        } else {
            state::with_transaction(&service, root.path(), |tx, id| {
                state::configure(tx, id, Some(&upstream), ConfigurationChange::Plan)
            })
            .unwrap()
        };
        let before = snapshot(&service, root.path());
        let plan_now = if cache_loss { &origin } else { &upstream };
        let context = plan_now.context(AuthoringKind::Ticket, &ITEM.parse().unwrap());
        let observations: Vec<_> = advertised
            .into_iter()
            .map(|oid| {
                // A fetch installs the tracking ref before the batch records it.
                if !layout.advertised_missing {
                    repository
                        .reference(context.tracking_ref(), oid, true, "fixture fetch")
                        .unwrap();
                }
                RemoteRefObservation::from_advertisement(
                    plan_now,
                    context.remote_ref(),
                    oid,
                    (!layout.advertised_missing).then_some(oid),
                )
                .unwrap()
            })
            .collect();
        state::with_transaction(&service, root.path(), |tx, id| {
            state::complete_batch(tx, id, plan_now, generation, &observations, 124)
        })
        .unwrap();

        let dispositions = [PollTargetReport {
            target: None,
            kind: Some(AuthoringKind::Ticket),
            item_id: Some(ITEM.parse().unwrap()),
            disposition: want.disposition.unwrap_or(PollDisposition::NotReached),
            before_oid: None,
            after_oid: None,
        }];
        let report = service
            .history_report(root.path(), &before, &dispositions)
            .unwrap();
        assert_eq!(report.cache_loss, cache_loss, "{name} flag");
        assert_eq!(report.generation, generation, "{name} generation");
        assert_eq!(report.entries.len(), 1, "{name} entries");
        let entry = &report.entries[0];
        assert_eq!(entry.kind, AuthoringKind::Ticket, "{name}");
        assert_eq!(entry.item_id.to_string(), ITEM, "{name}");
        assert_eq!(entry.branch, format!("manyhands/ticket/{ITEM}"), "{name}");
        assert_eq!(entry.local_head, Some(local), "{name} local head");
        assert_eq!(
            entry.tracking_ref_exists,
            advertised.is_some() && !layout.advertised_missing,
            "{name} tracking"
        );
        assert_eq!(entry.advertised, advertised, "{name} advertised");
        assert_eq!(entry.resolved, want.resolved, "{name} resolved");
        assert_eq!(
            entry.disposition,
            Some(want.disposition.unwrap_or(PollDisposition::NotReached)),
            "{name} disposition"
        );
        assert_eq!(entry.relation, want.relation, "{name} relation");
        assert_eq!(entry.ahead.zip(entry.behind), want.counts, "{name} counts");
        assert_eq!(
            entry.choices,
            RemoteHistoryChoices {
                publish: want.publish_and_release,
                release: want.publish_and_release
            },
            "{name} choices"
        );
    }
}

#[test]
fn a_poll_that_did_not_refresh_the_evidence_gives_no_report() {
    use RemoteHistoryError as E;
    use RemoteOutcomeCategory as C;
    type Check = fn(&Result<(), E>) -> bool;
    let mut rows: Vec<(&str, PollCompletion, Option<C>, Check)> = vec![
        (
            "completed",
            PollCompletion::Completed,
            Some(C::Completed),
            |r| r.is_ok(),
        ),
        (
            "cancelled",
            PollCompletion::Cancelled,
            Some(C::Cancelled),
            |r| matches!(r, Err(E::Cancelled)),
        ),
        ("interrupted", PollCompletion::Interrupted, None, |r| {
            matches!(r, Err(E::Interrupted))
        }),
        ("no category", PollCompletion::Completed, None, |r| {
            matches!(r, Err(E::Internal))
        }),
    ];
    for category in [
        C::ConfigurationRequired,
        C::SelectedKeyUnavailable,
        C::UnlockRequired,
        C::HostApprovalRequired,
        C::TransportUnavailable,
        C::ProtocolRejected,
        C::Cancelled,
        C::RepositoryUnavailable,
    ] {
        // The category is checked against the row's own below.
        rows.push(("stopped", PollCompletion::Completed, Some(category), |r| {
            matches!(r, Err(E::Stopped(_)))
        }));
    }
    for (name, completion, category, expect) in rows {
        let got = fresh(completion, category);
        assert!(expect(&got), "{name} {category:?}: {got:?}");
        if let (Err(E::Stopped(stopped)), Some(category)) = (&got, category) {
            assert_eq!(*stopped, category, "{name} keeps its category");
        }
    }
    // An explicit poll is never "not eligible", and the wrapper is unwrapped.
    for result in [
        RemotePollResult::Complete(RemotePollOutcome::NotEligible {
            reason: crate::repository::PollIneligibility::Disabled,
            next_eligible_at: None,
        }),
        RemotePollResult::IndexPending(crate::repository::IndexPending::new(
            RemotePollOutcome::NotEligible {
                reason: crate::repository::PollIneligibility::Disabled,
                next_eligible_at: None,
            },
        )),
    ] {
        assert!(matches!(fresh_targets(result), Err(E::Internal)));
    }
}
