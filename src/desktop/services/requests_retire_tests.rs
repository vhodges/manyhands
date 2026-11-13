//! `Intent::ContextRetire` (D6 Task 11a, Fact 14, U-10): what it builds,
//! and the flags every arm of Fact 14 answers for it. How the worker treats
//! it is `worker_retire_tests`, and how the scheduler hears its answer is
//! `scheduler_retire_tests`.

use std::path::PathBuf;

use manyhands::repository::{AuthoringKind, Mutation, ProgressStage};

use super::Intent;
use crate::desktop::{
    model::operations::{Progressing, progress_words},
    services::{JobKind, KeyStoreUnavailable},
};

const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG0";

/// A retirement builds `context retire` over the root, kind and item it
/// was asked for, with no key store asked for and no confirmation. It
/// stops at safe points (the library's `stoppable`, Fact 24) and can change
/// items, so its repository's tabs read again (U-10). Both kinds build.
#[test]
fn intent_context_retire() {
    let root = PathBuf::from("/repo");
    for kind in [AuthoringKind::Ticket, AuthoringKind::Document] {
        let intent = Intent::ContextRetire {
            root: root.clone(),
            kind,
            id: ITEM.parse().unwrap(),
        };
        // The store closure is never asked: none is available.
        let Ok(Mutation::ContextRetire(input)) = intent.build(None, || Err(KeyStoreUnavailable))
        else {
            panic!("a retirement builds `context retire`");
        };
        assert_eq!(input.root, root);
        assert_eq!(input.kind, kind);
        assert_eq!(input.id.to_string(), ITEM);

        assert_eq!(intent.job_kind(), JobKind::ContextRetire);
        assert_eq!(intent.label(), "Retire local copy");
        assert_eq!(intent.root(), Some(root.as_path()));
        assert!(!intent.confirmed(), "it needs no confirmation");
        assert_eq!(intent.onboarded_root(), None);
        assert!(intent.stops_at_safe_points());
        assert!(intent.changes_items());
        // Running, it says what it is doing: its stage, not "Running".
        assert_eq!(Progressing::of(&intent), Some(Progressing::Retire));
        assert_eq!(
            progress_words(Progressing::Retire, Some(ProgressStage::Writing), None),
            "Retiring · Removing the local copy…"
        );
        // A stopped retirement is sent again under its request ID, which
        // is what Resume does for a pending one.
        assert!(intent.can_be_given_up());
        // It is no context command of the R-78 pair.
        assert!(!intent.undone_by(&intent));
    }
}
