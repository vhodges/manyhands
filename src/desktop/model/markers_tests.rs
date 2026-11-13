use manyhands::repository::ClosureFilter;

use super::{Markers, row_markers};
use crate::desktop::model::polling::retire_loop::Held;

/// R-8: only a draft known to be held is unsaved work, so an unreadable
/// store marks no row; a closed row is marked only under All, where it
/// stands among open ones.
#[test]
fn markers_follow_drafts_conflicts_and_closure() {
    for (name, held, conflict, closed, closure, wanted) in [
        (
            "a held draft",
            Held::Yes,
            false,
            false,
            ClosureFilter::Open,
            (true, false, false),
        ),
        (
            "no draft",
            Held::No,
            false,
            false,
            ClosureFilter::Open,
            (false, false, false),
        ),
        (
            "an unknown store",
            Held::Unknown,
            false,
            false,
            ClosureFilter::Open,
            (false, false, false),
        ),
        (
            "a conflict",
            Held::No,
            true,
            false,
            ClosureFilter::Open,
            (false, true, false),
        ),
        (
            "closed under All",
            Held::No,
            false,
            true,
            ClosureFilter::All,
            (false, false, true),
        ),
        (
            "closed under Closed",
            Held::No,
            false,
            true,
            ClosureFilter::Closed,
            (false, false, false),
        ),
        (
            "closed under Open",
            Held::No,
            false,
            true,
            ClosureFilter::Open,
            (false, false, false),
        ),
        (
            "open under All",
            Held::No,
            false,
            false,
            ClosureFilter::All,
            (false, false, false),
        ),
        (
            "all three",
            Held::Yes,
            true,
            true,
            ClosureFilter::All,
            (true, true, true),
        ),
    ] {
        let (unsaved, conflict_marked, closed_marked) = wanted;
        assert_eq!(
            row_markers(held, conflict, closed, closure),
            Markers {
                unsaved,
                conflict: conflict_marked,
                closed: closed_marked,
            },
            "{name}"
        );
    }
}
