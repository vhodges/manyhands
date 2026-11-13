//! The S1 cases of P1: a request in a state, what the Workspace found of it,
//! and the row it must make.

use manyhands::{
    repository::{OperationId, ProgressStage},
    results::{Effects, Outcome, RecoveryAction, RecoveryActionKind, ResultCode, WriteEffect},
};
use serde_json::json;

use super::{
    intents::{assert_private, intent, lifecycle_answer},
    support::*,
};
use crate::desktop::{
    model::{
        activity::{
            requests::RequestFacts,
            rows::{ActivityAction, Derived, Kind},
        },
        operations::{Operations, StopOffer},
    },
    services::{
        requests::Intent,
        worker::{NotSent, RecordState, WorkerEvent},
    },
};

/// What one S1 row must say; `action` is the button's action.
pub(super) struct Want {
    kind: Kind,
    title: String,
    description: Option<String>,
    button: Option<(&'static str, bool)>,
    action: Option<ActivityAction>,
    menu: Vec<&'static str>,
    clearable: bool,
    detail: Vec<String>,
}

pub(super) struct Case {
    name: &'static str,
    operations: Operations,
    facts: RequestFacts,
    titled: bool,
    want: Want,
}

fn w(
    kind: Kind,
    title: &str,
    description: Option<&str>,
    button: Option<(&'static str, bool)>,
    menu: &[&'static str],
    clearable: bool,
    detail: &[&str],
) -> Want {
    Want {
        kind,
        title: title.to_owned(),
        description: description.map(str::to_owned),
        button,
        action: None,
        menu: menu.to_vec(),
        clearable,
        detail: detail.iter().map(|line| (*line).to_owned()).collect(),
    }
}

impl Want {
    fn on(mut self, action: ActivityAction) -> Self {
        self.action = Some(action);
        self
    }
}

impl Case {
    /// The row request 1 makes is `want`.
    pub(super) fn holds(&self) -> Derived {
        let (name, want) = (self.name, &self.want);
        let rows = derive(&self.operations, &|_| self.facts.clone(), self.titled);
        let row = row_of(&rows, 1).clone();
        assert_eq!(row.kind, want.kind, "{name}: kind");
        assert_eq!(row.title, want.title, "{name}: title");
        assert_eq!(row.description, want.description, "{name}: description");
        let button = row
            .button
            .as_ref()
            .map(|offer| (offer.label.as_str(), offer.enabled));
        assert_eq!(button, want.button, "{name}: button");
        if let Some(action) = &want.action {
            assert_eq!(
                row.button.as_ref().map(|offer| &offer.action),
                Some(action),
                "{name}: action"
            );
        }
        let menu: Vec<_> = row.menu.iter().map(|offer| offer.label.as_str()).collect();
        assert_eq!(menu, want.menu, "{name}: menu");
        assert_eq!(row.clearable, want.clearable, "{name}: clearable");
        assert_eq!(row.detail, want.detail, "{name}: detail");
        assert!(
            !row.attention,
            "{name}: a source leaves attention to the log"
        );
        assert_private(&row);
        row
    }
}

/// A request of the intent `name`, queued; `then` takes it further.
pub(super) fn request_of(name: &str, then: impl FnOnce(&mut Operations)) -> Operations {
    request_with(intent(name), then)
}

pub(super) fn request_with(intent: Intent, then: impl FnOnce(&mut Operations)) -> Operations {
    let mut operations = Operations::default();
    queue(&mut operations, 1, intent);
    then(&mut operations);
    operations
}

/// A request of the intent `name` answered `outcome` with `code`.
pub(super) fn answered(name: &str, outcome: Outcome, code: ResultCode) -> Operations {
    request_of(name, |operations| {
        start(operations, 1);
        finish(operations, 1, answer(outcome, code));
    })
}

fn recovery(kind: RecoveryActionKind, operation: Option<&str>) -> RecoveryAction {
    // The arguments the action registers: the repository, or the request.
    let arguments = match kind {
        RecoveryActionKind::RequestRetry => ("request_id", json!("01ARZ3NDEKTSV4RRFFQ69G5FG9")),
        _ => ("root", json!(ROOT)),
    };
    match operation {
        Some(operation) => RecoveryAction::for_operation(kind, operation, [arguments]),
        None => RecoveryAction::new(kind, [arguments]),
    }
}

/// B2's failure description for `code`: the library's sentence for it, and
/// that the work is safe.
fn safe_for(code: ResultCode) -> String {
    format!("{} Your work is safe on this computer.", code.message())
}

pub(super) fn safe() -> String {
    safe_for(ResultCode::InternalError)
}

fn failed_for(code: ResultCode) -> String {
    format!("{}: {}", code.as_str(), code.message())
}

fn running(name: &str, stage: Option<ProgressStage>) -> Operations {
    request_of(name, |operations| {
        start(operations, 1);
        if let Some(stage) = stage {
            self::stage(operations, 1, stage, None);
        }
    })
}

fn blocked(name: &str, recoveries: Vec<RecoveryAction>) -> Operations {
    request_of(name, |operations| {
        start(operations, 1);
        let envelope = answer(Outcome::Blocked, ResultCode::InternalError);
        finish(operations, 1, envelope.with_recovery(recoveries));
    })
}

/// The recoveries the Workspace found shown, by label.
fn shown(labels: &[(&str, bool)]) -> RequestFacts {
    let recoveries = labels
        .iter()
        .enumerate()
        .map(|(n, (label, enabled))| (n, (*label).to_owned(), *enabled))
        .collect();
    RequestFacts {
        recoveries,
        ..facts()
    }
}

#[rustfmt::skip]
pub(super) fn cases() -> Vec<Case> {
    let cancel = Some(("Cancel", true));
    let op = OperationId::new();
    let (op_text, abandon) = (op.to_string(), recovery(RecoveryActionKind::OperationAbandon, Some(&op.to_string())));
    let (retry, identity) = (recovery(RecoveryActionKind::RequestRetry, None), recovery(RecoveryActionKind::RepoIdentitySet, None));
    let (safe, failed) = (safe(), failed_for(ResultCode::InternalError));
    let (behind, behind_line) = (safe_for(ResultCode::DiscoveryPending), failed_for(ResultCode::DiscoveryPending));
    let (save, retire, close, refresh, repair) = ("Save ticket · demo", "Retire local copy · demo", "Close ticket · demo", "Refresh index · demo", "Repair document · demo");
    let case = |name, operations, facts, titled, want| Case { name, operations, facts, titled, want };
    let mut cases = vec![
        // Queued, and Running not accepted: Cancel. A save of an item with no
        // title known reads by its short code, never the draft's own title
        // (B1), and the detail holds no time (B5).
        case("queued save, no title", request_of("TicketSave", |_| {}), facts(), false,
            w(Kind::Working, "Waiting to save the ticket Q69G5FG0", None, cancel, &["Cancel"], false, &[save, "Queued"]).on(ActivityAction::Cancel(id(1)))),
        case("queued create, no title", request_of("DocumentCreate", |_| {}), facts(), false,
            w(Kind::Working, "Waiting to create the document Q69G5FG0", None, cancel, &["Cancel"], false, &["Create document · demo", "Queued"])),
        case("running, not accepted", running("DocumentSave", None), facts(), true,
            w(Kind::Working, "Saving “Plan”…", None, cancel, &["Cancel"], false, &["Save document · demo", "Running"]).on(ActivityAction::Cancel(id(1)))),
        // Accepted: Stop at a safe point when offered, Stopping… once
        // pressed, else a disabled Finishing, with the stage in words (F3).
        case("stop offered", running("ItemSync", Some(ProgressStage::Fetching)), RequestFacts { stop_offer: StopOffer::Offered, ..facts() }, true,
            w(Kind::Working, "Sharing “Plan”… · Bringing in changes…", None, Some(("Stop at a safe point", true)), &["Stop at a safe point"], false, &["Share item · demo", "Running", "Bringing in changes…"]).on(ActivityAction::Stop(id(1)))),
        case("stopping", running("ItemSync", Some(ProgressStage::Fetching)), RequestFacts { stop_offer: StopOffer::Stopping, ..facts() }, true,
            w(Kind::Working, "Sharing “Plan”… · Bringing in changes…", None, Some(("Stopping…", false)), &["Stopping…"], false, &["Share item · demo", "Running", "Bringing in changes…"])),
        case("nothing to stop", running("DocumentSave", Some(ProgressStage::Writing)), facts(), true,
            w(Kind::Working, "Saving “Plan”… · Saving…", None, Some(("Finishing", false)), &["Finishing"], false, &["Save document · demo", "Running", "Saving…"])),
        // A retirement is its own S1 row, with Cancel or Stop (R-51).
        case("retire queued", request_of("ContextRetire", |_| {}), facts(), true,
            w(Kind::Working, "Waiting to retire the copy of “Plan”", None, cancel, &["Cancel"], false, &[retire, "Queued"])),
        case("retire running", running("ContextRetire", Some(ProgressStage::Accepted)), RequestFacts { stop_offer: StopOffer::Offered, ..facts() }, true,
            w(Kind::Working, "Retiring the copy of “Plan”… · Checking it is safe to remove…", None, Some(("Stop at a safe point", true)), &["Stop at a safe point"], false, &[retire, "Running", "Checking it is safe to remove…"])),
    ];
    // An Owed answer, a Success envelope included, reads "didn't finish" and
    // is not clearable; its lists may be behind (A2, B2, B5).
    for (name, code, outcome, description, detail) in [
        ("owed, lists behind", ResultCode::DiscoveryPending, Outcome::Partial, Some(behind.as_str()), vec![retire, "Partly done", behind_line.as_str(), "The lists are still catching up."]),
        ("owed", ResultCode::Ok, Outcome::Success, None, vec![retire, "Done"]),
        ("owed, failed", ResultCode::InternalError, Outcome::Error, Some(safe.as_str()), vec![retire, "Failed", failed.as_str()]),
    ] {
        let operations = request_of("ContextRetire", |operations| {
            start(operations, 1);
            operations.record_stood(id(1), RecordState::Accepted);
            finish(operations, 1, answer(outcome, code));
        });
        cases.push(case(name, operations, facts(), true,
            w(Kind::Failure, "Retiring “Plan” didn't finish", description, None, &["Dismiss"], false, &detail)));
    }
    let lifecycle = [close, "Blocked", failed.as_str()];
    let closing = "Couldn't close “Plan”";
    let cancelled = format!("cancelled: {}", ResultCode::Cancelled.message());
    let repair_detail = [repair, "Blocked", failed.as_str(), &format!("Operation {op_text}")];
    let repairing = "Couldn't repair a document in demo";
    cases.extend([
        // A finished request that did not succeed: a shown recovery action a
        // lifecycle's dialog does not own outranks Open…; Abandon is never
        // the button.
        case("lifecycle, a recovery its dialog does not own", blocked("TicketClose", vec![identity]), shown(&[("Set identity", true)]), true,
            w(Kind::Failure, closing, Some(&safe), Some(("Set identity", true)), &["Set identity", "Open…", "Dismiss"], false, &lifecycle).on(ActivityAction::Recovery(id(1), 0))),
        case("lifecycle, nothing else", blocked("TicketClose", vec![]), facts(), true,
            w(Kind::Failure, closing, Some(&safe), Some(("Open…", true)), &["Open…", "Dismiss"], false, &lifecycle).on(ActivityAction::Open(id(1)))),
        case("Abandon is never the button", blocked("DocumentRepair", vec![abandon.clone(), retry.clone()]), shown(&[("Abandon", true), ("Retry", true)]), true,
            w(Kind::Failure, repairing, Some(&safe), Some(("Retry", true)), &["Retry", "Abandon", "Dismiss"], false, &repair_detail).on(ActivityAction::Recovery(id(1), 1))),
        case("only Abandon", blocked("DocumentRepair", vec![abandon]), shown(&[("Abandon", true)]), true,
            w(Kind::Failure, repairing, Some(&safe), None, &["Abandon", "Dismiss"], false, &repair_detail)),
        case("a recovery not available stays first", blocked("DocumentRepair", vec![retry]), shown(&[("Retry", false)]), true,
            w(Kind::Failure, repairing, Some(&safe), Some(("Retry", false)), &["Retry", "Dismiss"], false, &[repair, "Blocked", failed.as_str()])),
        // Ended without a failure: clearable unless an enabled offer other
        // than Dismiss or Open… remains, or a resend waits (A1, R-47).
        case("success", answered("IndexRefresh", Outcome::Success, ResultCode::Ok), facts(), true,
            w(Kind::Success, "Refreshed demo", None, None, &["Dismiss"], true, &[refresh, "Done"])),
        case("a resend waits on it", answered("IndexRefresh", Outcome::Success, ResultCode::Ok), RequestFacts { resend_waits: true, ..facts() }, true,
            w(Kind::Success, "Refreshed demo", None, None, &["Dismiss"], false, &[refresh, "Done"])),
        case("noop", answered("IndexRefresh", Outcome::Noop, ResultCode::AlreadyApplied), facts(), true,
            w(Kind::Success, "Refreshed demo", Some("Nothing needed to change."), None, &["Dismiss"], true, &[refresh, "No change"])),
        case("cancelled", answered("IndexRefresh", Outcome::Cancelled, ResultCode::Cancelled), facts(), true,
            w(Kind::Info, "Stopped refreshing demo", None, None, &["Dismiss"], true, &[refresh, "Cancelled", &cancelled])),
        case("removed", request_of("IndexRefresh", |operations| operations.apply(WorkerEvent::Removed(id(1)))), facts(), true,
            w(Kind::Info, "Cancelled: refresh demo", None, None, &["Dismiss"], true, &[refresh, "Cancelled"])),
        case("not sent", request_of("KeyGenerate", |operations| not_sent(operations, 1, NotSent::SecretRequired)), facts(), true,
            w(Kind::Failure, "Didn't create a key: your key's passphrase is needed again", None, None, &["Dismiss"], false, &["Generate key · Application", "Not sent"])),
        case("a cancelled close is clearable", answered("TicketClose", Outcome::Cancelled, ResultCode::Cancelled), facts(), true,
            w(Kind::Info, "Stopped closing “Plan”", None, Some(("Open…", true)), &["Open…", "Dismiss"], true, &[close, "Cancelled", &cancelled])),
        case("partial", request_of("IndexRefresh", |operations| {
                start(operations, 1);
                let effects = Effects { write: WriteEffect::Written, ..Effects::not_requested() };
                finish(operations, 1, answer(Outcome::Partial, ResultCode::InternalError).with_effects(effects));
            }), facts(), true,
            w(Kind::Failure, "Couldn't refresh demo", Some(&safe), None, &["Dismiss"], false, &[refresh, "Partly done", &failed, "Effects: write: written"])),
        // The lifecycle's outcome lines, from its answer (B2).
        case("close, done", request_of("TicketClose", |operations| {
                start(operations, 1);
                finish(operations, 1, lifecycle_answer(Outcome::Success));
            }), facts(), true,
            w(Kind::Success, "Closed “Plan”", Some("Merged into trunk. Shared. Tidied up."), None, &["Dismiss"], true,
              &[close, "Done", "Effects: publication: published, integration: complete, cleanup: complete"])),
        // "Already finished" for a Give up's replayed answer.
        case("already finished", answered("ItemSync", Outcome::Error, ResultCode::InternalError), RequestFacts { already_finished: true, ..facts() }, true,
            w(Kind::Failure, "Couldn't share “Plan”", Some(&safe), None, &["Dismiss"], false, &["Share item · demo", "Failed", &failed, "Already finished"])),
    ]);
    cases
}
