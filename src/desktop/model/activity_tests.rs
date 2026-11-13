//! Activity's model tests, one table per function (Testing, Wave 04 Cycle
//! 03b): the request rows' copy (P4), a finished share's copy (P5), the
//! time's formats (P7) and the S1 rows (P1).

use manyhands::{
    repository::{OperationId, ProgressStage},
    results::{Outcome, RecoveryAction, RecoveryActionKind, ResultCode},
};
use time::{Date, Month, OffsetDateTime, UtcOffset};

use super::{
    activity_time,
    requests::RequestFacts,
    rows::{Family, Group, ItemRef, Subject},
};
use crate::desktop::{
    model::operations::Operations,
    services::{
        requests::Intent,
        worker::{NotSent, RecordState, WorkerEvent},
    },
};

#[path = "activity_tests/intents.rs"]
mod intents;
#[path = "activity_tests/logs.rs"]
mod logs;
#[path = "activity_tests/s1.rs"]
mod s1;
#[path = "activity_tests/states.rs"]
mod states;
#[path = "activity_tests/support.rs"]
mod support;
#[path = "activity_tests/tables.rs"]
mod tables;
use intents::*;
use support::*;
use tables::{COPY, MESSAGE, RECORDS, share_rows, subjects};

fn utc(year: i32, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> OffsetDateTime {
    Date::from_calendar_date(year, Month::try_from(month).unwrap(), day)
        .unwrap()
        .with_hms(hour, minute, second)
        .unwrap()
        .assume_utc()
}

fn offset(hours: i8, minutes: i8) -> Option<UtcOffset> {
    Some(UtcOffset::from_hms(hours, minutes, 0).unwrap())
}

/// B6: "HH:MM" local, and the tooltip's "YYYY-MM-DD HH:MM", or UTC and " UTC"
/// with no offset; a day, a month and a year can roll over, and the seconds
/// are never shown.
#[test]
fn activity_time_formats() {
    for (at, zone, row, tooltip) in [
        (
            utc(2026, 11, 13, 14, 5, 9),
            offset(-7, 0),
            "07:05",
            "2026-11-13 07:05",
        ),
        (
            utc(2026, 11, 13, 14, 5, 9),
            offset(5, 30),
            "19:35",
            "2026-11-13 19:35",
        ),
        (
            utc(2026, 11, 13, 14, 5, 9),
            offset(0, 0),
            "14:05",
            "2026-11-13 14:05",
        ),
        (
            utc(2026, 11, 13, 14, 5, 9),
            None,
            "14:05",
            "2026-11-13 14:05 UTC",
        ),
        (
            utc(2026, 11, 13, 23, 59, 59),
            offset(1, 0),
            "00:59",
            "2026-11-14 00:59",
        ),
        (
            utc(2026, 11, 13, 0, 0, 0),
            offset(0, -1),
            "23:59",
            "2026-11-12 23:59",
        ),
        (
            utc(2026, 11, 13, 0, 0, 30),
            offset(0, 0),
            "00:00",
            "2026-11-13 00:00",
        ),
        (
            utc(2026, 11, 13, 0, 0, 30),
            None,
            "00:00",
            "2026-11-13 00:00 UTC",
        ),
        (
            utc(2026, 12, 31, 23, 30, 0),
            offset(1, 0),
            "00:30",
            "2027-01-01 00:30",
        ),
        (
            utc(2026, 3, 1, 9, 3, 0),
            None,
            "09:03",
            "2026-03-01 09:03 UTC",
        ),
    ] {
        assert_eq!(
            activity_time(at, zone),
            (row.to_owned(), tooltip.to_owned()),
            "{at} at {zone:?}"
        );
    }
}

fn lower_first(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_lowercase().chain(chars).collect()
    })
}

/// The row of the intent `name` in `state`: request 1, put there by the
/// worker's own events.
fn at_state(name: &str, state: &str) -> super::rows::Derived {
    let mut operations = Operations::default();
    queue(&mut operations, 1, intent(name));
    let lifecycle = matches!(name, "TicketClose" | "DocumentPromote");
    let failure = |outcome| answer(outcome, ResultCode::InternalError);
    match state {
        "queued" => {}
        "running" => start(&mut operations, 1),
        "staged" => {
            start(&mut operations, 1);
            stage(&mut operations, 1, ProgressStage::Fetching, None);
        }
        "success" if lifecycle => finish(&mut operations, 1, lifecycle_answer(Outcome::Success)),
        "success" => finish(&mut operations, 1, answer(Outcome::Success, ResultCode::Ok)),
        "noop" if lifecycle => finish(&mut operations, 1, lifecycle_answer(Outcome::Noop)),
        "noop" => finish(
            &mut operations,
            1,
            answer(Outcome::Noop, ResultCode::AlreadyApplied),
        ),
        "partial" => finish(&mut operations, 1, failure(Outcome::Partial)),
        "blocked" => finish(&mut operations, 1, failure(Outcome::Blocked)),
        "error" => finish(&mut operations, 1, failure(Outcome::Error)),
        "cancelled" => finish(
            &mut operations,
            1,
            answer(Outcome::Cancelled, ResultCode::Cancelled),
        ),
        "removed" => operations.apply(WorkerEvent::Removed(id(1))),
        "secret" => not_sent(&mut operations, 1, NotSent::SecretRequired),
        "keystore" => not_sent(&mut operations, 1, NotSent::KeyStoreUnavailable),
        other => panic!("no state named {other}"),
    }
    let rows = derive(&operations, &|_| facts(), true);
    row_of(&rows, 1).clone()
}

/// P4: every intent of B2's table in each request state says its title and
/// description from Doing, Done and Verb alone (the templates do not repeat
/// the subject), and no row names a draft's own title or a path.
#[test]
fn request_copy_table() {
    let message = ResultCode::InternalError.message();
    let safe = format!("{message} Your work is safe on this computer.");
    let lines = "Merged into trunk. Shared. Tidied up.";
    for (name, doing, done, verb, description) in COPY {
        let share = matches!(*name, "ItemSync" | "RepoSync");
        let lifecycle = matches!(*name, "TicketClose" | "DocumentPromote");
        let own = (!description.is_empty()).then(|| (*description).to_owned());
        let succeeded = if lifecycle {
            Some(lines.to_owned())
        } else {
            own.clone()
        };
        let unchanged = if lifecycle {
            Some(lines.to_owned())
        } else {
            own.clone().or(Some("Nothing needed to change.".to_owned()))
        };
        // (state, title, description; `None` for a description B3 words)
        let mut states: Vec<(&str, String, Option<Option<String>>)> = vec![
            ("queued", format!("Waiting to {verb}"), Some(None)),
            ("running", format!("{doing}…"), Some(None)),
            (
                "staged",
                format!("{doing}… · Bringing in changes…"),
                Some(None),
            ),
            (
                "partial",
                format!("Couldn't {verb}"),
                Some(Some(safe.clone())),
            ),
            (
                "blocked",
                format!("Couldn't {verb}"),
                Some(Some(safe.clone())),
            ),
            (
                "error",
                format!("Couldn't {verb}"),
                Some(Some(safe.clone())),
            ),
            (
                "cancelled",
                format!("Stopped {}", lower_first(doing)),
                (!share).then_some(None),
            ),
            ("removed", format!("Cancelled: {verb}"), Some(None)),
            (
                "secret",
                format!("Didn't {verb}: your key's passphrase is needed again"),
                Some(None),
            ),
            (
                "keystore",
                format!("Didn't {verb}: Manyhands could not find your SSH key store"),
                Some(None),
            ),
        ];
        if !share {
            states.push(("success", (*done).to_owned(), Some(succeeded)));
            states.push(("noop", (*done).to_owned(), Some(unchanged)));
        }
        for (state, title, description) in states {
            let row = at_state(name, state);
            assert_eq!(row.title, title, "{name} {state}");
            if let Some(description) = description {
                assert_eq!(row.description, description, "{name} {state}");
            }
            assert_private(&row);
        }
    }
}

/// P5: each `SyncOutcome` under each record state and after a Give up says
/// B3's words, kind and offers; the button is the first offer that is not
/// risky (Review changes first), the menu holds every offer in today's order,
/// then Dismiss; a share that succeeded offers nothing but Dismiss.
#[test]
fn share_copy_table() {
    use crate::desktop::model::{
        activity::{requests::RequestFacts, rows::Derived},
        publication::SyncOffer,
    };
    for share in share_rows() {
        for ((record, gave_up), (button, menu)) in RECORDS.into_iter().zip(share.cells) {
            let label = format!("{} {} {record:?} {gave_up}", share.intent, share.answer);
            let mut operations = Operations::default();
            queue(&mut operations, 1, intent(share.intent));
            start(&mut operations, 1);
            operations.note_share_attempt(id(1), record, gave_up);
            finish(&mut operations, 1, share_answer(share.answer));
            let result = operations.share_result(id(1)).cloned().unwrap();
            // A republish that runs hides Republish… and Stop and republish….
            let republishing = share.words.is_some() && share.answer == "BranchDeleted";
            let shown = RequestFacts {
                shown_offers: result
                    .offers()
                    .into_iter()
                    .filter(|offer| {
                        !(republishing
                            && matches!(offer, SyncOffer::Republish | SyncOffer::StopAndRepublish))
                    })
                    .collect(),
                share_words: share.words.map(str::to_owned),
                review: share.answer == "Conflict",
                ..facts()
            };
            let rows: Vec<Derived> = derive(&operations, &|_| shown.clone(), true);
            let row = row_of(&rows, 1);
            assert_eq!(row.kind, share.kind, "{label}");
            assert_eq!(row.title, share.title, "{label}");
            let said = format!(
                "{} Your work is safe on this computer.",
                ResultCode::InternalError.message()
            );
            let wanted = match share.description {
                Some(MESSAGE) => Some(said.as_str()),
                other => other,
            };
            assert_eq!(row.description.as_deref(), wanted, "{label}");
            assert_eq!(
                row.button.as_ref().map(|offer| offer.label.as_str()),
                button,
                "{label}"
            );
            let labels: Vec<_> = row.menu.iter().map(|offer| offer.label.as_str()).collect();
            assert_eq!(labels, menu, "{label}");
            assert_eq!(
                row.clearable,
                matches!(share.answer, "Published" | "Current"),
                "{label}"
            );
            assert_private(row);
        }
    }
}

/// P1, S1: each request in each state makes its row (key, kind, title,
/// description, button, menu, `clearable`, detail), by the cases of `s1`;
/// then what its `Group` and item say, for every intent and for the facts the
/// Workspace hands in. S2 to S14 are `states`'.
#[test]
fn each_source_makes_its_rows() {
    states::each_state_source_makes_its_rows();
    for case in s1::cases() {
        case.holds();
    }
    let op = OperationId::new();
    let resume = Intent::OperationResume {
        root: root(),
        operation_id: op,
    };

    // A dismissed request makes no row; nothing else is hidden (R-51).
    let mut operations = s1::answered("ContextRetire", Outcome::Success, ResultCode::Ok);
    operations.dismiss(id(1));
    assert!(derive(&operations, &|_| facts(), true).is_empty());

    // The operation a running request names, and its own (N21).
    let operations = s1::request_with(resume.clone(), |operations| {
        start(operations, 1);
        stage(
            operations,
            1,
            ProgressStage::Indexing,
            Some(&op.to_string()),
        );
    });
    let row = row_of(&derive(&operations, &|_| facts(), true), 1).clone();
    assert_eq!(row.group.operation, Some(op.to_string()));
    assert_eq!(row.group.running_operation, Some(op.to_string()));
    assert_eq!(row.detail[2], format!("Operation {op}"));

    // Candidate follows the envelope, not the "didn't finish" words; an Owed
    // retirement has no outcome, so it settles nothing (C1; N22 clause 1);
    // the operation a failure's recovery names is its own (N1).
    for (outcome, code, candidate) in [
        (Outcome::Success, ResultCode::Ok, false),
        (Outcome::Partial, ResultCode::DiscoveryPending, true),
    ] {
        let operations = s1::request_of("ContextRetire", |operations| {
            start(operations, 1);
            operations.record_stood(id(1), RecordState::Accepted);
            finish(operations, 1, answer(outcome, code));
        });
        let row = row_of(&derive(&operations, &|_| facts(), true), 1).clone();
        assert_eq!((row.group.candidate, row.group.outcome), (candidate, None));
    }
    let recovery =
        RecoveryAction::for_operation(RecoveryActionKind::OperationResume, op.to_string(), []);
    let mut operations = Operations::default();
    queue(&mut operations, 1, intent("IndexRefresh"));
    finish(
        &mut operations,
        1,
        answer(Outcome::Blocked, ResultCode::InternalError).with_recovery(vec![recovery]),
    );
    let rows = derive(&operations, &|_| facts(), true);
    assert_eq!(row_of(&rows, 1).group.operation, Some(op.to_string()));

    // What the Workspace found is carried, and a check is marked (A1).
    let flags = RequestFacts {
        followed: true,
        follow_void: true,
        check_applied: true,
        superseded: true,
        resend_waits: true,
        ..facts()
    };
    let mut operations = s1::answered("PollOnce", Outcome::Error, ResultCode::InternalError);
    let row = row_of(&derive(&operations, &|_| flags.clone(), true), 1).clone();
    let expected = Group {
        candidate: true,
        roots: vec![ROOT.to_owned()],
        check: true,
        check_applied: true,
        superseded: true,
        subject: Some(Subject::Intent {
            family: Family::Check,
            root: Some(ROOT.to_owned()),
            item: None,
            target: None,
        }),
        sent: Some(1),
        outcome: Some(Outcome::Error),
        followed: true,
        follow_void: true,
        resend_waits: true,
        ..Group::default()
    };
    assert_eq!(row.group, expected);

    // The queue order of each attempt, and a retry's new one (N2, R-56).
    queue(&mut operations, 2, intent("IndexRefresh"));
    queue(&mut operations, 1, intent("PollOnce"));
    let rows = derive(&operations, &|_| facts(), true);
    assert_eq!(
        (row_of(&rows, 1).group.sent, row_of(&rows, 2).group.sent),
        (Some(3), Some(2))
    );

    // An operation an automatic poll runs is the running one (N21).
    let mut operations = s1::request_with(resume, |operations| {
        start(operations, 1);
        finish(
            operations,
            1,
            answer(Outcome::Error, ResultCode::InternalError),
        );
    });
    assert_eq!(
        derive(&operations, &|_| facts(), true)[0]
            .group
            .running_operation,
        None
    );
    operations.set_running_polls([op].into());
    let row = derive(&operations, &|_| facts(), true).remove(0);
    assert_eq!(row.group.running_operation, Some(op.to_string()));
    assert_eq!(
        row.group.operation,
        Some(op.to_string()),
        "an OperationResume's own operation"
    );

    // Each intent's subject, its own target (never drawn), its item and its
    // repository tag (R-52, R-57).
    let table = subjects();
    assert_eq!(table.len(), 44);
    for (name, family, kind, target) in table {
        let intent = intent(name);
        let target = target.map(|target| match (target, &intent) {
            (
                "<key>",
                Intent::KeySelect { id } | Intent::KeyRemove { id } | Intent::KeyDelete { id },
            ) => id.to_string(),
            (
                "<operation>",
                Intent::OperationResume { operation_id, .. }
                | Intent::OperationAbandon { operation_id, .. }
                | Intent::ConflictResolve { operation_id, .. }
                | Intent::LocalContextResolve { operation_id, .. },
            ) => operation_id.to_string(),
            (text, _) => text.to_owned(),
        });
        let rooted = intent.root().is_some().then(|| ROOT.to_owned());
        let mut operations = Operations::default();
        queue(&mut operations, 1, intent);
        let rows = derive(&operations, &|_| facts(), true);
        let row = row_of(&rows, 1);
        let subject = Subject::Intent {
            family,
            root: rooted.clone(),
            item: kind.map(|_| item()),
            target,
        };
        assert_eq!(row.group.subject, Some(subject), "{name}");
        assert_eq!(
            row.group.roots,
            rooted.iter().cloned().collect::<Vec<_>>(),
            "{name}"
        );
        assert_eq!(row.repo, rooted.map(|_| "demo".to_owned()), "{name}");
        let about = kind.map(|kind| ItemRef {
            root: ROOT.to_owned(),
            kind,
            id: item(),
        });
        assert_eq!(row.item, about, "{name}");
        assert_eq!(
            (row.group.sent, row.group.outcome),
            (Some(1), None),
            "{name}"
        );
    }
}
