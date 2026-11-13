//! P2, ruling C9: N19 absorbs a failed Check now by a row L3 keeps only when
//! the failure was first evaluated while that row was kept. A failure that
//! counted on its own goes on counting until an N22 clause settles it, so its
//! unresolved cause is never dropped.

use manyhands::results::ResultCode;

use super::{
    attention::{cant_reach, repository_of, sorted},
    scene::{Row, repo, req, row, step},
    support::ROOT,
};
use crate::desktop::model::repository_state::StateKind as K;

/// A poll runs: the repository checks, so its row is kept (L3).
fn polling(s: &mut super::scene::Scene) {
    s.world.repositories = vec![repository_of(ROOT, K::Checking, None, None)];
    s.world.repositories[0].checking = true;
}

pub(super) fn kept_rows() -> Vec<Row> {
    vec![row(
        "N19 a failure that counted on its own is not absorbed by a kept row (C9)",
        vec![
            step(
                "a Check now fails recovery_required beside a Can't reach row: nothing was applied, so it counts beside it",
                |s| {
                    s.world.repositories = vec![cant_reach(ROOT)];
                    s.fail_code(1, "PollOnce", ResultCode::RecoveryRequired);
                },
                sorted(vec![repo(ROOT), req(1)]),
            ),
            step(
                "an automatic poll runs and the row is kept: the failure still counts",
                polling,
                sorted(vec![repo(ROOT), req(1)]),
            ),
            step(
                "the poll answers and the repository is still unreachable: still two",
                |s| s.world.repositories = vec![cant_reach(ROOT)],
                sorted(vec![repo(ROOT), req(1)]),
            ),
            step(
                "the repository clears: the failure is not settled, it counts",
                |s| {
                    s.world.repositories = vec![repository_of(ROOT, K::UpToDate, None, None)];
                },
                vec![req(1)],
            ),
        ],
    )]
}
