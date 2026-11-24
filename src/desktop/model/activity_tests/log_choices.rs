//! P3, rows T11 and T12 of A4's table: a choice runs only when its row's revision and its offer still stand (`still_offered`).

use manyhands::results::ResultCode;

use super::{
    log_more::{STOP, dismiss, keep},
    scene::{Row, req, row, step},
    states::world::OPERATION,
    support::id,
};
use crate::desktop::model::{activity::rows::ActivityAction, operations::StopOffer};

/// T11 and T12: a choice runs only when its row's revision is the one it was
/// made against and its offer is there and enabled. `choose_activity` asks the
/// log's `still_offered`.
pub(super) fn choices() -> Vec<Row> {
    vec![
        row(
            "T11 the attempt changed",
            vec![
                step(
                    "a failed request",
                    |s| s.fail(1, "IndexRefresh"),
                    vec![req(1)],
                )
                .check(|s| {
                    keep(s, "drawn", 1);
                    let drawn = s.recall("drawn");
                    assert!(s.log.still_offered(&req(1), drawn, &dismiss(1), "Dismiss"));
                }),
                step(
                    "it is retried and fails again",
                    |s| {
                        s.at(1);
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
                    vec![req(1)],
                )
                .check(|s| {
                    let (drawn, now) = (s.recall("drawn"), s.row(&req(1)).revision);
                    assert_ne!(drawn, now, "a new attempt is a new revision");
                    assert!(!s.log.still_offered(&req(1), drawn, &dismiss(1), "Dismiss"));
                    assert!(s.log.still_offered(&req(1), now, &dismiss(1), "Dismiss"));
                }),
            ],
        ),
        row(
            "T11 the offers changed",
            vec![
                step(
                    "a queued request offers Cancel",
                    |s| s.queue(1, "IndexRefresh"),
                    vec![],
                )
                .check(|s| {
                    keep(s, "drawn", 1);
                    let drawn = s.recall("drawn");
                    assert!(s.log.still_offered(
                        &req(1),
                        drawn,
                        &ActivityAction::Cancel(id(1)),
                        "Cancel"
                    ));
                }),
                step(
                    "it starts, not accepted: still Cancel, the revision holds",
                    |s| s.run(1),
                    vec![],
                )
                .check(|s| {
                    let drawn = s.recall("drawn");
                    assert_eq!(s.row(&req(1)).revision, drawn);
                    assert!(s.log.still_offered(
                        &req(1),
                        drawn,
                        &ActivityAction::Cancel(id(1)),
                        "Cancel"
                    ));
                }),
                step(
                    "it is accepted and offers Stop: the offers changed",
                    |s| {
                        s.stage(
                            1,
                            manyhands::repository::ProgressStage::Fetching,
                            Some(OPERATION),
                        );
                        s.facts_of(1).stop_offer = StopOffer::Offered;
                    },
                    vec![],
                )
                .check(|s| {
                    let drawn = s.recall("drawn");
                    assert_ne!(s.row(&req(1)).revision, drawn);
                    assert!(!s.log.still_offered(
                        &req(1),
                        drawn,
                        &ActivityAction::Cancel(id(1)),
                        "Cancel"
                    ));
                }),
            ],
        ),
        row(
            "T11 a Stop whose stage words changed",
            vec![
                step(
                    "a running request offers Stop at a safe point",
                    |s| {
                        s.queue(1, "IndexRefresh");
                        s.run(1);
                        s.stage(
                            1,
                            manyhands::repository::ProgressStage::Fetching,
                            Some(OPERATION),
                        );
                        s.facts_of(1).stop_offer = StopOffer::Offered;
                    },
                    vec![],
                )
                .check(|s| {
                    keep(s, "drawn", 1);
                    let (drawn, title) = (s.recall("drawn"), s.row(&req(1)).row.title.clone());
                    assert!(s.log.still_offered(
                        &req(1),
                        drawn,
                        &ActivityAction::Stop(id(1)),
                        STOP
                    ));
                    s.remember("title", title.len() as u64);
                }),
                step(
                    "its stage moves on: the words change, the revision holds, the Stop runs",
                    |s| s.stage(1, manyhands::repository::ProgressStage::Observing, None),
                    vec![],
                )
                .check(|s| {
                    let drawn = s.recall("drawn");
                    assert_eq!(s.row(&req(1)).revision, drawn);
                    assert!(s.log.still_offered(
                        &req(1),
                        drawn,
                        &ActivityAction::Stop(id(1)),
                        STOP
                    ));
                }),
            ],
        ),
        row(
            "T12",
            vec![
                step(
                    "an accepted request that cannot be stopped says Finishing, disabled",
                    |s| {
                        s.queue(1, "IndexRefresh");
                        s.run(1);
                        s.stage(
                            1,
                            manyhands::repository::ProgressStage::Fetching,
                            Some(OPERATION),
                        );
                    },
                    vec![],
                )
                .check(|s| {
                    let revision = s.row(&req(1)).revision;
                    let stop = ActivityAction::Stop(id(1));
                    assert!(
                        !s.log.still_offered(&req(1), revision, &stop, "Finishing"),
                        "disabled"
                    );
                    assert!(
                        !s.log.still_offered(
                            &req(1),
                            revision,
                            &ActivityAction::Cancel(id(1)),
                            "Cancel"
                        ),
                        "an offer the row does not have"
                    );
                    assert!(
                        !s.log.still_offered(&req(9), revision, &stop, "Finishing"),
                        "a row that is not there"
                    );
                    keep(s, "drawn", 1);
                }),
                step(
                    "it is stoppable, then it ends: the Stop is gone",
                    |s| {
                        s.facts_of(1).stop_offer = StopOffer::Offered;
                        s.finish(
                            1,
                            super::support::answer(
                                manyhands::results::Outcome::Success,
                                ResultCode::Ok,
                            ),
                        );
                    },
                    vec![],
                )
                .check(|s| {
                    let revision = s.row(&req(1)).revision;
                    assert!(!s.log.still_offered(
                        &req(1),
                        revision,
                        &ActivityAction::Stop(id(1)),
                        STOP
                    ));
                }),
            ],
        ),
    ]
}
