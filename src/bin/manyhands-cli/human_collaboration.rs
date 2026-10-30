//! Human output of C4's collaboration verbs. Every result keeps C2's
//! success and refusal lines. C4 adds what its data and codes need:
//!
//! - a conflict's `conflict:` line, and the `conflict show` command that
//!   looks at it;
//! - a comment's note on what publishing it publishes;
//! - a local-context sync's state, reason and next action (R-37);
//! - for `remote_branch_deleted`, three `next:` lines: cancel the refused
//!   request, send it again, which ends it cancelled and releases its
//!   reservation, then prepare the republish (Task 2b's ruling);
//! - for `original_request_required`, a `next:` line naming the request,
//!   and the supersede note when that request's remote resolution has
//!   written paths only.
//!
//! A `next:` line C4 prints renders a recovery action the library offered,
//! in full, in place of C2's line for it. C4 adds no action (decision 8).

use crate::{
    human_admin,
    render::{self, Output, field},
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
}

const RESERVATION_NOTE: &str = "the refused request keeps its reservation until it is cancelled; sending it again then ends it and releases the reservation.";
/// What a published comment also publishes (design, Human Output).
const PUBLICATION_NOTE: &str = "publishing the comment also publishes this item's other checkpointed work; unsaved drafts are not included.";
const SUPERSEDE_NOTE: &str =
    "a new conflict resolve may supersede its resolution while the paths it wrote are unchanged.";

/// A root in a `next:` command: single-quoted for a POSIX shell, with
/// controls escaped as C3's `path observe` escapes a path, and each single
/// quote closed, escaped and opened again.
pub fn quoted(root: &str) -> String {
    format!("'{}'", field(root).replace('\'', r"'\''"))
}

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
                    lines.next.extend(branch_deleted(
                        root,
                        envelope.scope.item_id.as_deref(),
                        request,
                        again(envelope, request),
                    ));
                    lines.notes.push(RESERVATION_NOTE.into());
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
                if seen.supersedable {
                    lines.notes.push(SUPERSEDE_NOTE.into());
                }
            }
            _ => {}
        }
        lines
    }

    fn is_empty(&self) -> bool {
        self.data.is_empty() && self.next.is_empty() && self.notes.is_empty()
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
