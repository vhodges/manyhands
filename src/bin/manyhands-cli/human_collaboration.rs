//! Human output of C4's collaboration verbs. Every result keeps C2's
//! success and refusal lines. C4 adds what its data and codes need:
//!
//! - a conflict's `conflict:` line, and the `conflict show` command that
//!   looks at it;
//! - a comment's note on what publishing it publishes;
//! - a local-context sync's state, reason and next action (R-37);
//! - for `remote_branch_deleted`, three `next:` lines: cancel the request that
//!   holds the reservation, send it again, which ends it cancelled and
//!   releases the reservation, then prepare the republish (Task 2b's ruling).
//!   For a `conflict resolve` that request is the one holding the
//!   synchronization, not the resolve;
//! - for a `busy` `item republish`, a note before the retry line: the
//!   reservation may be held by an earlier `item sync` of the item that was
//!   refused, and no public read names that request, so the note says to
//!   cancel it with its ID (controller ruling, Task 10);
//! - for the no-op preview of `item republish`, the `item sync` to run first,
//!   because the library offers no recovery action that names a sync;
//! - for `original_request_required` of `operation resume`, a `next:` line
//!   naming the request, and the supersede note when that request's remote
//!   resolution has written paths only;
//! - for `original_request_required` of `conflict resolve`, the same
//!   `next:` line and a note that the attempt's written paths were edited,
//!   because that is the only way a resolve meets it;
//! - for `external_change` of `conflict resolve` on a conflict whose attempt
//!   has written paths, the note that a hand-edited file must be restored to
//!   the request's resolution before the request is sent again;
//! - for `conflict show`, the conflict's state, next step and paths. A side
//!   is content, so only JSON carries it.
//!
//! A `next:` line C4 prints renders a recovery action the library offered,
//! in full, in place of C2's line for it. C4 adds no action (decision 8).

use crate::{
    human_admin::{self, REPUBLISH},
    render::{self, Human, Output, field, quoted},
};
use manyhands::{
    repository::*,
    results::{Envelope, RecoveryActionKind, ResultCode},
};

/// What the CLI read beside the envelope, which the envelope cannot say.
#[derive(Default)]
pub struct Seen {
    /// The request an `original_request_required` names owns a remote
    /// resolution attempt whose paths are written and not committed: a
    /// new `conflict resolve` may supersede it while those paths are
    /// unchanged (decision 18).
    pub supersedable: bool,
    /// A `conflict resolve` was refused as `external_change` on a conflict
    /// whose attempt has written paths (`historical`, R-39): one of them may
    /// have been edited by hand.
    pub written: bool,
    /// The item a `conflict resolve` that met a deleted remote branch was
    /// synchronizing, which its envelope does not name.
    pub item: Option<String>,
}

const RESERVATION_NOTE: &str = "the refused request keeps its reservation until it is cancelled; sending it again then ends it and releases the reservation.";
/// What a published comment also publishes (design, Human Output).
const PUBLICATION_NOTE: &str = "publishing the comment also publishes this item's other checkpointed work; unsaved drafts are not included.";
const SUPERSEDE_NOTE: &str =
    "a new conflict resolve may supersede its resolution while the paths it wrote are unchanged.";
/// A `conflict resolve` refused with `original_request_required`: the
/// library answers so only when the attempt's written paths changed, and
/// without a change a new request supersedes the attempt.
///
/// The library's rule (`integration/resolution.rs`, `remote/sync.rs`): a path
/// the request has written must hold its result; only a path it has not
/// written yet may hold its conflicted content.
const EDITED_NOTE: &str = "the attempt's written paths were edited by hand. Restore each file the request wrote to the request's resolution; a file it has not written yet may keep its conflicted content. Then send that request again, or a new conflict resolve may supersede it.";
/// A `conflict resolve` refused with `external_change` on an attempt with
/// written paths. The JSON message stays the code's own (Global
/// Constraints), so this is where the way out is told.
const HAND_EDIT_NOTE: &str = "a resolved file edited by hand must be restored to the request's resolution before the request is sent again.";
const CLOSURES_NOTE: &str =
    "two clones closed this ticket differently; repair it outside Manyhands.";
pub const RESOLVE: &str = "conflict resolve";
/// A `busy` `item republish`. Its reservation can be held by an `item sync`
/// of the same item that was refused as `remote_branch_deleted`, which keeps
/// it until it is cancelled (design, Republish). No public read gives that
/// request's ID, so the note says where it comes from.
const BUSY_REPUBLISH_NOTE: &str = "if an earlier item sync of this item was refused, its request still holds the reservation: run request cancel with that request's ID, then send that request once more, before retrying this request.";

/// How a request whose command cannot be named is sent again.
pub fn resend(request: &str) -> String {
    format!(
        "send request {} again with its original command and input",
        field(request)
    )
}

/// The `next:` lines of `remote_branch_deleted`, without their prefix:
/// cancel the request that holds the reservation, send it again (`again`,
/// when its command can be named), then prepare the republish of the item.
/// The republish line needs the repository and the item.
pub fn branch_deleted(
    root: Option<&str>,
    item: Option<&str>,
    request: &str,
    again: Option<String>,
) -> Vec<String> {
    let mut lines = vec![
        format!(
            "manyhands-cli request cancel --request-id {}",
            field(request)
        ),
        again.unwrap_or_else(|| resend(request)),
    ];
    if let (Some(root), Some(item)) = (root, item) {
        lines.push(format!(
            "manyhands-cli item republish --repo {} --id {} --prepare",
            quoted(root),
            field(item)
        ));
    }
    lines
}

/// The `conflict:` line of a conflict's next step, and the command that
/// shows the conflict when the repository is known.
pub fn conflict(next: &ConflictNextDto, root: Option<&str>) -> (String, Option<String>) {
    let operation = field(&next.operation_id);
    (
        format!(
            "conflict: operation {operation} stopped on a conflict; next action: {}",
            next.next_action.as_str()
        ),
        root.map(|root| {
            format!(
                "manyhands-cli conflict show --repo {} --operation-id {operation}",
                quoted(root)
            )
        }),
    )
}

/// The command that sends `request` again, when the envelope is that
/// request's own and its command and selectors can be named in full.
fn again(envelope: &Envelope<MutationDataDto>, request: &str) -> Option<String> {
    if envelope.request_id.as_deref() != Some(request) {
        return None;
    }
    let root = quoted(envelope.scope.repository.as_deref()?);
    let request = field(request);
    match envelope.command.as_str() {
        "item sync" => Some(format!(
            "manyhands-cli item sync --repo {root} --id {} --request-id {request}",
            field(envelope.scope.item_id.as_deref()?)
        )),
        "repo sync" => Some(format!(
            "manyhands-cli repo sync --repo {root} --request-id {request}"
        )),
        _ => None,
    }
}

/// The note on the reservation `request` holds. A refused request that holds
/// its own reservation says so. A `conflict resolve` holds none: the request
/// that does is the synchronization's or the comment's, so the note names it.
fn reservation_note(envelope: &Envelope<MutationDataDto>, request: &str) -> String {
    if envelope.request_id.as_deref() == Some(request) {
        return RESERVATION_NOTE.into();
    }
    RESERVATION_NOTE.replacen(
        "the refused request",
        &format!("request {}", field(request)),
        1,
    )
}

/// The request a recovery action names, when it names one.
fn named(arguments: &serde_json::Map<String, serde_json::Value>) -> Option<&str> {
    arguments.get("request_id")?.as_str()
}

/// The `request.retry` actions that name a request other than the
/// envelope's own, with their index in `recovery`: the request an
/// `original_request_required` sends the caller back to.
pub fn other_requests(envelope: &Envelope<MutationDataDto>) -> impl Iterator<Item = (usize, &str)> {
    envelope
        .recovery
        .iter()
        .enumerate()
        .filter(|(_, action)| action.action == RecoveryActionKind::RequestRetry)
        .filter_map(|(index, action)| Some((index, named(&action.arguments)?)))
        .filter(|(_, request)| envelope.request_id.as_deref() != Some(*request))
}

/// What C4 adds to one result.
#[derive(Default)]
struct Lines {
    /// The data, shown in place of C2's field list.
    data: Vec<String>,
    /// `next:` lines, without the prefix.
    next: Vec<String>,
    notes: Vec<String>,
    /// Notes of a refusal that come before C2's `next:` lines for its
    /// recovery actions, because they say what to do before the first.
    lead: Vec<String>,
    /// The recovery actions a `next:` line above renders.
    rendered: Vec<usize>,
    /// Whether `data` replaces C2's field list.
    replaces_fields: bool,
}

impl Lines {
    fn of(envelope: &Envelope<MutationDataDto>, seen: &Seen) -> Self {
        let root = envelope.scope.repository.as_deref();
        let mut lines = Self::default();
        let step = |lines: &mut Self, next: &ConflictNextDto| {
            let (line, show) = conflict(next, root);
            lines.data.push(line);
            lines.next.extend(show);
        };
        match &envelope.data {
            Some(MutationDataDto::SynchronizationConflict(data)) => {
                lines.replaces_fields = true;
                step(&mut lines, &data.conflict);
            }
            Some(MutationDataDto::Comment(comment)) => {
                // Only a checkpointed comment is published, so only a
                // result that carries one has something to say about it.
                lines.notes.push(PUBLICATION_NOTE.into());
                if let Some(next) = &comment.conflict {
                    step(&mut lines, next);
                }
            }
            Some(MutationDataDto::ConflictResolution(ConflictResolutionMutationDto {
                conflict: Some(next),
                ..
            })) => step(&mut lines, next),
            Some(MutationDataDto::LocalContextSync(data)) => {
                lines.replaces_fields = true;
                lines.data.extend([
                    format!("state: {}", data.state.as_str()),
                    format!("reason: {}", data.reason.as_str()),
                    format!("next action: {}", data.next_action.as_str()),
                ]);
            }
            _ => {}
        }
        match envelope.code {
            ResultCode::RemoteBranchDeleted => {
                let cancel = envelope
                    .recovery
                    .iter()
                    .enumerate()
                    .filter(|(_, action)| action.action == RecoveryActionKind::RequestCancel)
                    .find_map(|(index, action)| Some((index, named(&action.arguments)?)));
                if let Some((index, request)) = cancel {
                    lines.rendered.push(index);
                    // A `conflict resolve` names no item: the conflict read does.
                    let item = envelope.scope.item_id.as_deref().or(seen.item.as_deref());
                    lines.next.extend(branch_deleted(
                        root,
                        item,
                        request,
                        again(envelope, request),
                    ));
                    lines.notes.push(reservation_note(envelope, request));
                }
            }
            ResultCode::OriginalRequestRequired => {
                let mut requests = Vec::new();
                for (index, request) in other_requests(envelope) {
                    lines.rendered.push(index);
                    requests.push(request.to_owned());
                }
                if let Some(MutationDataDto::LocalContextSync(LocalContextSyncMutationDto {
                    required_request_id: Some(required),
                    ..
                })) = &envelope.data
                    && !requests.contains(required)
                {
                    requests.push(required.clone());
                }
                lines
                    .next
                    .extend(requests.iter().map(|request| resend(request)));
                if envelope.command == RESOLVE {
                    lines.notes.push(EDITED_NOTE.into());
                } else if seen.supersedable {
                    lines.notes.push(SUPERSEDE_NOTE.into());
                }
            }
            ResultCode::ExternalChange if seen.written => {
                lines.notes.push(HAND_EDIT_NOTE.into());
            }
            ResultCode::Busy if envelope.command == REPUBLISH => {
                lines.lead.push(BUSY_REPUBLISH_NOTE.into());
            }
            _ => {}
        }
        lines
    }

    fn is_empty(&self) -> bool {
        self.data.is_empty()
            && self.next.is_empty()
            && self.notes.is_empty()
            && self.lead.is_empty()
    }

    /// The data and `next:` lines, each ended.
    fn body(&self) -> String {
        let mut out = String::new();
        for line in &self.data {
            out.push_str(line);
            out.push('\n');
        }
        for line in &self.next {
            out.push_str("next: ");
            out.push_str(line);
            out.push('\n');
        }
        out
    }

    fn notes(&self) -> String {
        self.notes
            .iter()
            .map(|note| format!("note: {note}\n"))
            .collect()
    }

    /// The notes that precede a refusal's recovery lines.
    fn lead(&self) -> String {
        self.lead
            .iter()
            .map(|note| format!("note: {note}\n"))
            .collect()
    }
}

fn put(out: &mut String, label: &str, value: &str) {
    out.push_str(label);
    out.push_str(": ");
    out.push_str(&field(value));
    out.push('\n');
}

/// The human output of `conflict show`: the state, the next step and each
/// path with its kind, structure and token (design, Human Output).
///
/// `display_path` is printed as the library escaped it (R-11), so it is
/// printed once through `field`, which leaves that text as it is and only
/// escapes a raw control, which the library never emits. A path comes last
/// on its line, so a name that looks like another field cannot forge one.
/// A side is content, and only `--json` prints it, so a terminal is never
/// repainted by what a conflict holds.
pub fn conflict_show(data: &ConflictDto) -> Human {
    let mut stdout = String::new();
    put(&mut stdout, "operation", &data.operation_id);
    if let Some(comment) = &data.comment_operation_id {
        put(&mut stdout, "comment operation", comment);
    }
    put(&mut stdout, "target", data.target.as_str());
    if let Some(item) = &data.item_id {
        put(&mut stdout, "item", item);
    }
    put(&mut stdout, "stage", data.stage.as_str());
    put(&mut stdout, "state", data.state.as_str());
    put(&mut stdout, "next action", data.next_action.as_str());
    if let Some(request) = &data.request_id {
        put(&mut stdout, "request", request);
    }
    put(
        &mut stdout,
        "observation",
        data.observation.as_deref().unwrap_or("-"),
    );
    if data.historical {
        put(&mut stdout, "historical", "yes");
    }
    for path in &data.paths {
        put(
            &mut stdout,
            "path",
            &format!(
                "kind={} structure={} token={} display={}",
                path.kind.as_str(),
                path.structure.as_str(),
                path.token,
                path.display_path
            ),
        );
    }
    let mut notes = Vec::new();
    if data
        .paths
        .iter()
        .any(|path| path.structure == ConflictPathStructure::IncompatibleClosures)
    {
        notes.push(CLOSURES_NOTE.to_owned());
    }
    Human { stdout, notes }
}

/// The human output of a collaboration verb's result: C2's, with C4's lines.
pub fn output(envelope: &Envelope<MutationDataDto>, supplied: bool, seen: &Seen) -> Output {
    let lines = Lines::of(envelope, seen);
    if lines.is_empty() {
        return render::mutation_output(false, envelope, supplied);
    }
    // C2's lines, with the field list left out where C4's lines show the data.
    let mut shown = envelope.clone();
    if lines.replaces_fields {
        shown.data = None;
    }
    if envelope.failure_class().is_none() {
        let mut stdout = human_admin::mutation_success(&shown).stdout;
        stdout.push_str(&lines.body());
        return Output {
            stdout: stdout.into_bytes(),
            stderr: lines.notes().into_bytes(),
            status: render::exit_status(None),
        };
    }
    let mut extra = human_admin::failure_extra(&shown, supplied);
    extra.push_str(&lines.body());
    // `extra` is printed before the recovery lines the refusal carries.
    extra.push_str(&lines.lead());
    let refusal = Envelope::<serde_json::Value> {
        schema_version: envelope.schema_version,
        command: envelope.command.clone(),
        request_id: envelope.request_id.clone(),
        operation_id: envelope.operation_id.clone(),
        outcome: envelope.outcome,
        code: envelope.code,
        message: envelope.message.clone(),
        scope: envelope.scope.clone(),
        effects: envelope.effects.clone(),
        data: None,
        // C2's lines for the actions C4 did not render.
        recovery: envelope
            .recovery
            .iter()
            .enumerate()
            .filter(|(index, _)| !lines.rendered.contains(index))
            .map(|(_, action)| action.clone())
            .collect(),
    };
    let mut output = render::failure_output(false, &refusal, Some(extra));
    output.stderr.extend(lines.notes().into_bytes());
    output
}

#[cfg(test)]
#[path = "human_collaboration_tests.rs"]
mod tests;
