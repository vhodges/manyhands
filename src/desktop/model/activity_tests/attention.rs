//! P2, rows N1 to N12 of A3's table (N20 withdrawn), with the precedence's
//! order of N2 and N4. Every row is a list of steps over `derive` and
//! `ActivityLog::refresh`; what each step expects is the list of attention
//! rows' keys, written out. Rows N13 onwards, and N22's clauses, are in
//! `attention_more.rs`.

use manyhands::{
    repository::{OperationAction, OperationDto, OperationFamily},
    results::ResultCode,
};

use super::{
    scene::{Row, Step, pending_row, repo, req, row, step},
    states::world::{OPERATION, key_id, operation, repository, state, target},
    support::{ITEM, ROOT},
};
use crate::desktop::model::{
    activity::{
        rows::{RowKey, Target},
        states::RepositoryInput,
    },
    polling::{Answer, Authority, Disposition, TransportFailure},
    repository_state::{AttentionCause as Cause, RowAction, StateKind as K},
};

fn problem_of(root: &str) -> RowKey {
    RowKey::Problem {
        root: root.to_owned(),
        target: Target::Item(ITEM.parse().unwrap()),
    }
}

pub(super) fn sorted(mut keys: Vec<RowKey>) -> Vec<RowKey> {
    keys.sort();
    keys
}

/// A pending operation that waits to be resumed: next action Resume.
pub(super) fn waiting() -> OperationDto {
    operation(OperationAction::SaveTicket, OperationFamily::Local)
}

/// A pending operation with nothing to do (N12).
pub(super) fn idle() -> OperationDto {
    OperationDto {
        next_action: None,
        ..waiting()
    }
}

/// A repository, as 3a's state says it is.
pub(super) fn repository_of(
    root: &str,
    kind: K,
    action: Option<RowAction>,
    cause: Option<Cause>,
) -> RepositoryInput {
    repository(
        root,
        "demo",
        state(kind, action, &["Says something"], cause),
    )
}

pub(super) fn cant_reach(root: &str) -> RepositoryInput {
    repository_of(root, K::CantReach, Some(RowAction::RetryNow), None)
}

pub(super) fn locked_by(key: crate::desktop::model::polling::KeyId) -> Answer {
    Answer::Transport(TransportFailure::KeyLocked { key: Some(key) })
}

pub(super) fn host_unapproved() -> Answer {
    Answer::Transport(TransportFailure::HostUnapproved {
        authority: Some(Authority {
            host: "git.example.invalid".to_owned(),
            port: 22,
        }),
    })
}

/// A step that only changes the scene, and counts `counted`.
pub(super) fn then(
    what: &'static str,
    counted: Vec<RowKey>,
    change: fn(&mut super::scene::Scene),
) -> Step {
    step(what, change, counted)
}

/// P2: each row of A3's table over `derive` and `ActivityLog::refresh`: the
/// count it says, and the rows the footer and the popover would list.
#[test]
fn attention_counts_each_thing_once() {
    let mut rows = first_rows();
    rows.extend(super::attention_more::more_rows());
    rows.extend(super::attention_moved::moved_rows());
    rows.extend(super::attention_moved::moved_rows());
    super::scene::play("attention_counts_each_thing_once", rows);
}

fn first_rows() -> Vec<Row> {
    let mut rows = vec![
        // N1: a failed request and its operation's pending row, which counts
        // once, as the pending row; with the row idle (N12), as the request.
        row(
            "N1",
            vec![
                then(
                    "the pending row counts, the request is absorbed",
                    vec![pending_row(ROOT, OPERATION)],
                    |s| {
                        s.fail_op(1, "IndexRefresh", OPERATION);
                        s.listed(ROOT, vec![waiting()]);
                    },
                )
                .check(|s| assert!(s.has(&req(1)) && s.has(&pending_row(ROOT, OPERATION)))),
                then(
                    "the row has nothing to do, the request counts",
                    vec![req(1)],
                    |s| {
                        s.listed(ROOT, vec![idle()]);
                    },
                ),
            ],
        ),
        // N2: grouped by the operation when it is set, else by subject; the
        // newest member by queue order counts once, not by time seen.
        row(
            "N2 by operation",
            vec![then(
                "two commands, one operation: the newer counts",
                vec![req(2)],
                |s| {
                    s.fail_op(1, "IndexRefresh", OPERATION);
                    s.fail_op(2, "IndexRebuild", OPERATION);
                },
            )],
        ),
        row(
            "N2 by subject",
            vec![
                then(
                    "two failed refreshes of one repository: the newer counts",
                    vec![req(2)],
                    |s| {
                        s.fail(1, "IndexRefresh");
                        s.fail(2, "IndexRefresh");
                    },
                ),
                then(
                    "another command's failure counts beside them",
                    sorted(vec![req(2), req(3)]),
                    |s| {
                        s.fail(3, "IndexRebuild");
                    },
                ),
                then(
                    "an older request retried and failed again is the newest by queue order",
                    sorted(vec![req(1), req(3)]),
                    |s| {
                        s.retry(1);
                        s.run(1);
                        s.finish(
                            1,
                            super::support::answer(
                                manyhands::results::Outcome::Error,
                                ResultCode::InternalError,
                            ),
                        );
                    },
                ),
            ],
        ),
        // N3: one failed request that names no operation.
        row(
            "N3",
            vec![then("it counts once", vec![req(1)], |s| {
                s.fail(1, "IndexRefresh")
            })],
        ),
    ];
    rows.extend(checks());
    rows.extend(super::attention_states::repositories());
    rows
}

/// N4: a failed Check now, for each failure class: absorbed only when
/// `check_failed` applied its answer and a candidate hold or repository row
/// covers its root, or under N19; otherwise it counts on its own (R-53).
fn checks() -> Vec<Row> {
    let locked = || RowKey::Hold(format!("KeyLocked:{}", key_id()));
    vec![
        row(
            "N4 unlock_required: a held key covers it",
            vec![
                then("absorbed, counted as the hold", vec![locked()], |s| {
                    s.answered(ROOT, locked_by(key_id()));
                    s.fail_code(1, "PollOnce", ResultCode::UnlockRequired);
                    s.facts_of(1).check_applied = true;
                })
                .check(|s| assert!(s.has(&req(1)))),
            ],
        ),
        row(
            "N4 host_approval_required: a host hold covers it",
            vec![then(
                "absorbed, counted as the hold",
                vec![RowKey::Hold(
                    "HostUnapproved:git.example.invalid:22".to_owned(),
                )],
                |s| {
                    s.answered(ROOT, host_unapproved());
                    s.fail_code(1, "PollOnce", ResultCode::HostApprovalRequired);
                    s.facts_of(1).check_applied = true;
                },
            )],
        ),
        row(
            "N4 invalid_remote: a configuration hold covers it",
            vec![then(
                "absorbed, counted as the hold",
                vec![RowKey::Hold(format!("ConfigurationRequired:{ROOT}"))],
                |s| {
                    s.answered(
                        ROOT,
                        Answer::Transport(TransportFailure::ConfigurationRequired),
                    );
                    s.fail_code(1, "PollOnce", ResultCode::InvalidRemote);
                    s.facts_of(1).check_applied = true;
                },
            )],
        ),
        row(
            "N4 transport_unavailable: the Can't reach row covers it",
            vec![then(
                "absorbed, counted as the repository's row",
                vec![repo(ROOT)],
                |s| {
                    s.world.repositories = vec![cant_reach(ROOT)];
                    s.fail_code(1, "PollOnce", ResultCode::TransportUnavailable);
                    s.facts_of(1).check_applied = true;
                },
            )],
        ),
        row(
            "N4 recovery_required: unmapped, beside a Can't reach row",
            vec![then(
                "nothing was applied: the request counts beside the row",
                sorted(vec![repo(ROOT), req(1)]),
                |s| {
                    s.world.repositories = vec![cant_reach(ROOT)];
                    s.fail_code(1, "PollOnce", ResultCode::RecoveryRequired);
                },
            )],
        ),
        row(
            "N4 internal_error: unmapped, nothing covers it",
            vec![then("the request counts", vec![req(1)], |s| {
                s.fail_code(1, "PollOnce", ResultCode::InternalError);
            })],
        ),
        row(
            "N4 held and only unreachable: the hold is not the answer",
            vec![then(
                "check_failed applied nothing: the request counts beside the hold",
                sorted(vec![locked(), req(1)]),
                |s| {
                    s.answered(ROOT, locked_by(key_id()));
                    s.fail_code(1, "PollOnce", ResultCode::TransportUnavailable);
                },
            )],
        ),
        row(
            "N4 applied but the covering row is information",
            vec![then(
                "the request counts: an information row never absorbs",
                vec![req(1)],
                |s| {
                    s.world.repositories = vec![repository_of(
                        ROOT,
                        K::ThisComputer,
                        Some(RowAction::SetUpSharing),
                        None,
                    )];
                    s.fail_code(1, "PollOnce", ResultCode::TransportUnavailable);
                    s.facts_of(1).check_applied = true;
                },
            )],
        ),
        row(
            "N4 applied but no row covers the root",
            vec![then("the request counts", vec![req(1)], |s| {
                s.fail_code(1, "PollOnce", ResultCode::PushRejected);
                s.facts_of(1).check_applied = true;
            })],
        ),
        // The covering rows are holds and repository rows only (A3): a
        // problem row also names the root and counts, and absorbs nothing.
        row(
            "N4 a problem row is not a covering row",
            vec![then(
                "the request counts beside the problem",
                sorted(vec![problem_of(ROOT), req(1)]),
                |s| {
                    s.world.repositories = vec![repository_of(
                        ROOT,
                        K::NeedsAttention,
                        None,
                        Some(Cause::Problems),
                    )];
                    s.world.repositories[0].unshared.problems = vec![target(Disposition::Dirty)];
                    s.fail_code(1, "PollOnce", ResultCode::TransportUnavailable);
                    s.facts_of(1).check_applied = true;
                },
            )],
        ),
        // The precedence: N2 picks the newest of a group before N4 reads it.
        row(
            "N2 before N4",
            vec![
                then("an older failed check nothing covers", vec![req(1)], |s| {
                    s.fail_code(1, "PollOnce", ResultCode::InternalError);
                }),
                then(
                    "a newer one the hold absorbs: the group adds nothing more",
                    vec![locked()],
                    |s| {
                        s.answered(ROOT, locked_by(key_id()));
                        s.fail_code(2, "PollOnce", ResultCode::UnlockRequired);
                        s.facts_of(2).check_applied = true;
                    },
                ),
            ],
        ),
    ]
}
