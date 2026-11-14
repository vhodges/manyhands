//! P2, row N22 of A3's table: the five clauses, the subject (R-57) and the cases the clauses must not reach.

use manyhands::{
    repository::{ContextRetirementState as State, RetirementRemoval as Removal},
    results::{Outcome, ResultCode},
};

use super::{
    attention::{cant_reach, locked_by, waiting},
    scene::{Row, key_hold, pending_row, repo, req, row, step},
    states::world::{OPERATION, OTHER, key_id},
    support::{ITEM, ROOT},
};
use crate::desktop::{
    model::activity::rows::RowKey, services::worker::RecordState,
    test_support::retire::answer as retirement,
};

fn locked() -> RowKey {
    key_hold(key_id())
}

fn retired() -> manyhands::results::Envelope<manyhands::repository::MutationDataDto> {
    retirement(State::Retired, ResultCode::Ok, Removal::Observed)
}

/// N22: the five clauses, the subject (R-57) and the cases the clauses must
/// not reach.
pub(super) fn settled() -> Vec<Row> {
    let uncounted =
        |what: &'static str, change: fn(&mut super::scene::Scene)| step(what, change, vec![]);
    vec![
        // Clause 1: a later success, by subject.
        row(
            "N22.1 a later success of the same subject",
            vec![
                step("a failed share", |s| s.fail(1, "ItemSync"), vec![req(1)]),
                uncounted("a later share of the item succeeds", |s| {
                    s.succeed(2, "ItemSync")
                }),
            ],
        ),
        row(
            "N22.1 the family: a republish settles a failed share",
            vec![
                step("a failed share", |s| s.fail(1, "ItemSync"), vec![req(1)]),
                uncounted("a later republish of the item succeeds", |s| {
                    s.succeed(2, "ItemRepublish")
                }),
            ],
        ),
        row(
            "N22.1 the target (R-57): another key's success settles nothing",
            vec![
                step(
                    "a failed key removal",
                    |s| s.fail(1, "KeyRemove"),
                    vec![req(1)],
                ),
                step(
                    "another key is removed",
                    |s| s.succeed(2, "KeyRemove"),
                    vec![req(1)],
                ),
            ],
        ),
        // Clause 2: the operation finished, with no pending row seen.
        row(
            "N22.2 its operation finished",
            vec![
                step(
                    "a failure names an operation no pending row shows",
                    |s| s.fail_op(1, "IndexRefresh", OPERATION),
                    vec![req(1)],
                ),
                uncounted("a Resume of the operation succeeds", |s| {
                    s.resume(2, OPERATION, Outcome::Success, ResultCode::Ok);
                }),
            ],
        ),
        row(
            "N22.2 abandoned: a Noop answer",
            vec![
                step(
                    "a failure",
                    |s| s.fail_op(1, "IndexRefresh", OPERATION),
                    vec![req(1)],
                ),
                uncounted(
                    "the operation is already gone: a Resume answers Noop",
                    |s| {
                        s.resume(2, OPERATION, Outcome::Noop, ResultCode::AlreadyApplied);
                    },
                ),
            ],
        ),
        // Clause 3: its absorber left.
        row(
            "N22.3 the pending row that absorbed it left",
            vec![
                step(
                    "a failure and its pending row",
                    |s| {
                        s.fail_op(1, "IndexRefresh", OPERATION);
                        s.listed(ROOT, vec![waiting()]);
                    },
                    vec![pending_row(ROOT, OPERATION)],
                ),
                uncounted("the row leaves", |s| s.listed(ROOT, vec![])),
                step(
                    "the row comes back: it counts, the failure does not",
                    |s| s.listed(ROOT, vec![waiting()]),
                    vec![pending_row(ROOT, OPERATION)],
                ),
            ],
        ),
        row(
            "N22.3 a group whose newest member the row absorbed",
            vec![
                step(
                    "two failures of one operation and its pending row",
                    |s| {
                        s.fail_op(1, "IndexRefresh", OPERATION);
                        s.fail_op(2, "IndexRebuild", OPERATION);
                        s.listed(ROOT, vec![waiting()]);
                    },
                    vec![pending_row(ROOT, OPERATION)],
                ),
                uncounted("the row leaves: both are settled", |s| {
                    s.listed(ROOT, vec![])
                }),
            ],
        ),
        row(
            "N22.3 the hold that absorbed it left",
            vec![
                step(
                    "a Check now absorbed by a hold",
                    |s| {
                        s.answered(ROOT, locked_by(key_id()));
                        s.fail_code(1, "PollOnce", ResultCode::UnlockRequired);
                        s.facts_of(1).check_applied = true;
                    },
                    vec![locked()],
                ),
                uncounted("the hold goes, and no row covers the root", |s| {
                    s.world.schedule_root(ROOT)
                }),
            ],
        ),
        row(
            "N22.3 another candidate row covers the root",
            vec![
                step(
                    "a Check now absorbed by a hold",
                    |s| {
                        s.answered(ROOT, locked_by(key_id()));
                        s.fail_code(1, "PollOnce", ResultCode::UnlockRequired);
                        s.facts_of(1).check_applied = true;
                    },
                    vec![locked()],
                ),
                step(
                    "the hold goes and the repository is Can't reach: its row absorbs it, nothing is settled",
                    |s| {
                        s.world.schedule_root(ROOT);
                        s.world.repositories = vec![cant_reach(ROOT)];
                    },
                    vec![repo(ROOT)],
                ),
                uncounted("that row goes too: now it is settled", |s| {
                    s.world.repositories.clear()
                }),
            ],
        ),
        row(
            "N22.3 the hold stays for another repository",
            vec![
                step(
                    "a hold over two repositories, and a Check now of one is absorbed by it",
                    |s| {
                        s.answered(ROOT, locked_by(key_id()));
                        s.answered(OTHER, locked_by(key_id()));
                        s.fail_code(1, "PollOnce", ResultCode::UnlockRequired);
                        s.facts_of(1).check_applied = true;
                    },
                    vec![locked()],
                ),
                step(
                    "the checked repository is let go, the hold stays for the other",
                    |s| s.world.schedule_root(ROOT),
                    vec![locked()],
                ),
            ],
        ),
        // Clause 4: a later completed check.
        row(
            "N22.4 a later completed check",
            vec![
                step(
                    "a failed Check now nothing covers",
                    |s| s.fail_code(1, "PollOnce", ResultCode::InternalError),
                    vec![req(1)],
                ),
                uncounted("its repository applied a completed attempt since", |s| {
                    s.facts_of(1).superseded = true;
                }),
            ],
        ),
        // Clause 5: a retirement event.
        row(
            "N22.5 a retirement event",
            vec![
                step(
                    "a failed retirement",
                    |s| s.fail(1, "ContextRetire"),
                    vec![req(1)],
                ),
                uncounted(
                    "a later retirement retires the copy and is dismissed",
                    |s| {
                        s.retire(2, ITEM.parse().unwrap(), RecordState::Finished, retired());
                    },
                ),
            ],
        ),
        row(
            "N22.1 an Owed retirement is not a success",
            vec![
                step(
                    "a failed retirement",
                    |s| s.fail(1, "ContextRetire"),
                    vec![req(1)],
                ),
                step(
                    "a later retirement answered Success but left Accepted: it didn't finish",
                    |s| {
                        s.retire(2, ITEM.parse().unwrap(), RecordState::Accepted, retired());
                    },
                    vec![req(1)],
                ),
            ],
        ),
    ]
}
