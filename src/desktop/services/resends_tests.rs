//! What waits on a republish, and its pruning (review I-2, M4).

use std::path::PathBuf;

use manyhands::repository::OperationId;

use super::{AfterRepublish, Resends};
use crate::desktop::services::requests::DesktopRequestId;

/// A republish confirmed from a run's way out waits for that run, and goes
/// once the run, or the republish, is dismissed.
#[test]
fn a_republish_waits_for_its_run_until_either_is_dismissed() {
    let resends = Resends::default();
    let (run, republish) = (DesktopRequestId(1), DesktopRequestId(2));
    assert!(!resends.pending_for(run));
    resends.after_republish(republish, AfterRepublish::Retry(run));
    assert!(resends.pending_for(run));
    resends.forget(republish);
    assert!(!resends.pending_for(run));

    let resume = AfterRepublish::Resume {
        shown: run,
        root: PathBuf::from("/repo"),
        operation_id: OperationId::new(),
    };
    resends.after_republish(DesktopRequestId(3), resume);
    assert!(resends.pending_for(run));
    resends.forget(run);
    assert!(!resends.pending_for(run));
    assert_eq!(resends.follow(run), None);
}
