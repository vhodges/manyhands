//! Human output of `document promote`, `ticket close` and an `operation
//! resume` that continued one. Every result keeps C2's lines, and the data's
//! field list, less what is said better here:
//!
//! - `made ready:` lines, with slugs, in place of the field list's;
//! - a note when publication waits for a remote, or no made-ready set is held;
//! - for a refused checkpoint merge of a closure this operation recorded, with
//!   a way to go on, the local repair: four `next:` lines naming `context
//!   observe-local`, `sync-local`, `show-local` and `resolve-local`, before C2's
//!   lines for the same request and `repo inspect`. No recovery action names
//!   them, and none may be added (design, Answers). The context verbs'
//!   grammar answers to this text, which `lifecycle_tests.rs` pins.
//!   Any other form of that blocker relies on its offered actions;
//! - for a blocked prepare, its way out: the same repair, or the save that
//!   makes a missing context, and then a new prepare;
//! - for a child's `remote_branch_deleted`, the route out of decision 18:
//!   cancel, the same request, the republish, the same request. Under a
//!   resume, decision 19's: the republish, then the resume, since the resume
//!   has already asked for the cancellation that lets the child go.

use crate::{
    human_admin,
    human_collaboration::republish,
    render::{self, Output, field, note_lines, quoted, refusal_of},
};
use manyhands::{
    repository::{
        ItemDtoKind, LifecycleBlocker, LifecycleMadeReadyState, LifecycleMutationDto,
        LifecyclePublicationPendingReason, LifecycleRecording, MutationDataDto, PrepareDataDto,
    },
    results::{Envelope, RecoveryActionKind, ResultCode},
};

/// What the repair's lines leave to the reader. The route these lines print is
/// walked through the binary, from a refused closure to its finish, by
/// `tests/cli/local_sync.rs` `the_refused_merges_route_finishes_the_close`.
const REPAIR: &str = "The commands above are the local repair: the <...> values are placeholders to fill in, --input - takes the observation the first line prints, and context show-local and context resolve-local apply only if context sync-local answers a conflict.";
/// What a missing context's note says, here and in `context observe-local`.
pub(crate) const NO_CONTEXT_NOTE: &str = "this item has no editing context, and a save makes one; <new-request-id> is a placeholder to fill in.";
const NO_REMOTE_NOTE: &str = "primary is not published, because this repository has no publication remote; once one is selected, repo sync publishes it.";
const NO_SET_NOTE: &str = "this answer holds no made-ready set; ticket ready is the live view.";
const DELETED_NOTE: &str = "the refused request's context synchronization keeps its reservation until the request is cancelled; sending the request again then answers cancelled and releases it. After the republish, sending it once more finishes the work.";
/// The answer doesn't tell a released child from a kept one (decision 19).
const RESUME_DELETED_NOTE: &str = "this resume released the operation's context synchronization if nothing it holds could be lost. If repo inspect still shows its reservation, it was kept, and the republish answers busy. Add --request-id to the resume unless you run it at a terminal.";

/// The data's fields, without its tag, its made-ready set (`made_ready`) and,
/// for a promotion, a state that says nothing.
fn data_fields(data: &LifecycleMutationDto) -> String {
    let Ok(mut value) = serde_json::to_value(data) else {
        return String::new();
    };
    if let Some(fields) = value.as_object_mut() {
        fields.remove("type");
        fields.remove("made_ready");
        if data.made_ready_state.is_none() {
            fields.remove("made_ready_state");
        }
    }
    human_admin::fields(&value)
}

/// The `made ready:` lines, when the state says the set is what was read.
fn made_ready(data: &LifecycleMutationDto) -> String {
    if data.made_ready_state != Some(LifecycleMadeReadyState::Current) {
        return String::new();
    }
    let tickets = data.made_ready.as_deref().unwrap_or_default();
    if tickets.is_empty() {
        return "made ready: none\n".into();
    }
    tickets
        .iter()
        .map(|ticket| match &ticket.slug {
            Some(slug) => format!("made ready: {} {}\n", field(&ticket.id), field(slug)),
            None => format!("made ready: {}\n", field(&ticket.id)),
        })
        .collect()
}

fn notes_of(data: &LifecycleMutationDto) -> Vec<String> {
    let mut notes = Vec::new();
    if data.publication_pending == Some(LifecyclePublicationPendingReason::NoPublicationRemote) {
        notes.push(NO_REMOTE_NOTE.to_owned());
    }
    if matches!(
        data.made_ready_state,
        Some(
            LifecycleMadeReadyState::IndexPending | LifecycleMadeReadyState::IndexUnavailableFinal
        )
    ) {
        notes.push(NO_SET_NOTE.to_owned());
    }
    notes
}

/// The local repair of a refused merge. Its lines come before the same
/// request's, which is what follows them. `<new-request-id>` and
/// `<sync-operation-id>` are the caller's to fill: a request not yet made, and
/// what `context sync-local` answers.
fn merge_refused(root: &str, item: &str) -> Vec<String> {
    let (root, item) = (quoted(root), field(item));
    vec![
        format!("manyhands-cli context observe-local --repo {root} --id {item}"),
        format!(
            "manyhands-cli context sync-local --repo {root} --id {item} --input - --request-id <new-request-id>"
        ),
        format!(
            "manyhands-cli context show-local --repo {root} --operation-id <sync-operation-id>"
        ),
        format!(
            "manyhands-cli context resolve-local --repo {root} --operation-id <sync-operation-id> --input - --request-id <new-request-id>"
        ),
    ]
}

/// What a result adds, and the recovery actions its `next:` lines render.
#[derive(Default)]
struct Route {
    next: Vec<String>,
    notes: Vec<String>,
    rendered: Vec<usize>,
}

fn action_index(envelope: &Envelope<MutationDataDto>, kind: RecoveryActionKind) -> Option<usize> {
    envelope
        .recovery
        .iter()
        .position(|action| action.action == kind)
}

fn branch_deleted(envelope: &Envelope<MutationDataDto>, root: &str, item: &str, route: &mut Route) {
    if let Some(index) = action_index(envelope, RecoveryActionKind::OperationResume) {
        route.next.push(republish(root, item));
        if let Some(line) = resume_line(envelope, index, root) {
            route.rendered.push(index);
            route.next.push(line);
        }
        route.notes.push(RESUME_DELETED_NOTE.to_owned());
        return;
    }
    let Some(request) = envelope.request_id.as_deref() else {
        return;
    };
    let again = format!(
        "manyhands-cli {} --repo {} --id {} --request-id {}",
        field(&envelope.command),
        quoted(root),
        field(item),
        field(request)
    );
    if let Some(index) = action_index(envelope, RecoveryActionKind::RequestCancel) {
        route.rendered.push(index);
        route.next.extend([
            format!(
                "manyhands-cli request cancel --request-id {}",
                field(request)
            ),
            again.clone(),
        ]);
        route.notes.push(DELETED_NOTE.to_owned());
    }
    route.next.extend([republish(root, item), again]);
}

/// The repair, when the same request or a resume can then go on: `sync-local`
/// admits a closure only while it is recording its own (`local_sync/record.rs`).
fn merge_refused_route(
    envelope: &Envelope<MutationDataDto>,
    root: &str,
    item: &str,
    route: &mut Route,
) {
    const REFUSED: &str = "primary refused this closure's checkpoint.";
    if action_index(envelope, RecoveryActionKind::RequestRetry).is_some() {
        route.next = merge_refused(root, item);
        route.notes.push(format!(
            "{REFUSED} {REPAIR} Then send this request again; the closure's fields are kept."
        ));
    } else if let Some(index) = action_index(envelope, RecoveryActionKind::OperationResume) {
        route.next = merge_refused(root, item);
        if let Some(line) = resume_line(envelope, index, root) {
            route.rendered.push(index);
            route.next.push(line);
        }
        route.notes.push(format!(
            "{REFUSED} {REPAIR} Then resume it again (add --request-id unless you run it at a terminal); the closure's fields are kept."
        ));
    }
}

/// The `operation resume` a recovery action names.
fn resume_line(envelope: &Envelope<MutationDataDto>, index: usize, root: &str) -> Option<String> {
    let operation = envelope.recovery[index]
        .operation_id
        .as_deref()
        .or(envelope.operation_id.as_deref())?;
    Some(format!(
        "manyhands-cli operation resume --repo {} --operation-id {}",
        quoted(root),
        field(operation)
    ))
}

fn route_of(envelope: &Envelope<MutationDataDto>, data: &LifecycleMutationDto) -> Route {
    let mut route = Route::default();
    let Some(root) = envelope.scope.repository.as_deref() else {
        return route;
    };
    match envelope.code {
        ResultCode::RecoveryRequired
            if data
                .blockers
                .contains(&LifecycleBlocker::IntegrationConflict)
                && data.kind == ItemDtoKind::Ticket
                && data.recording == Some(LifecycleRecording::Record) =>
        {
            merge_refused_route(envelope, root, &data.item_id, &mut route);
        }
        ResultCode::RemoteBranchDeleted => {
            branch_deleted(envelope, root, &data.item_id, &mut route);
        }
        _ => {}
    }
    route
}

/// The save that makes an item's editing context, as a pasteable command.
pub(crate) fn save_line(kind: ItemDtoKind, root: &str, item: &str) -> String {
    let save = match kind {
        ItemDtoKind::Document => "document save",
        ItemDtoKind::Ticket => "ticket save",
    };
    format!(
        "manyhands-cli {save} --repo {} --id {} --input - --request-id <new-request-id>",
        quoted(root),
        field(item)
    )
}

/// The way out of a blocked prepare: what to run, and what it says of that.
fn way_out(
    command: &str,
    blockers: &[LifecycleBlocker],
    root: &str,
    item: &str,
) -> Option<(Vec<String>, String)> {
    use LifecycleBlocker::{
        ConflictPending, ContextConflicted, ContextDirty, ContextMissing, IntegrationConflict,
        LocalOperationPending,
    };
    if blockers == [ContextMissing] {
        let kind = if command == "document promote" {
            ItemDtoKind::Document
        } else {
            ItemDtoKind::Ticket
        };
        return Some((
            vec![save_line(kind, root, item)],
            format!("{NO_CONTEXT_NOTE} Then prepare again."),
        ));
    }
    // `sync-local` refuses a dirty or conflicted context, and a pending conflict
    // or local operation has its own way on.
    let reachable = blockers.contains(&IntegrationConflict)
        && ![
            ContextDirty,
            ContextConflicted,
            ConflictPending,
            LocalOperationPending,
        ]
        .iter()
        .any(|other| blockers.contains(other));
    reachable.then(|| {
        (
            merge_refused(root, item),
            format!("this context can't merge into primary. {REPAIR} Then prepare again."),
        )
    })
}

/// The human output of a prepare of these verbs: C2's, with a blocked prepare's
/// way out.
pub fn prepare(envelope: &Envelope<PrepareDataDto>) -> Output {
    let (Some(PrepareDataDto::LifecycleBlocked(data)), Some(root), Some(item)) = (
        &envelope.data,
        envelope.scope.repository.as_deref(),
        envelope.scope.item_id.as_deref(),
    ) else {
        return render::prepare_output(false, envelope);
    };
    let Some((next, note)) = way_out(&envelope.command, &data.blockers, root, item) else {
        return render::prepare_output(false, envelope);
    };
    let mut extra = human_admin::fields(data);
    for line in next {
        extra.push_str(&format!("next: {line}\n"));
    }
    let refusal = refusal_of(envelope, envelope.recovery.clone());
    let mut output = render::failure_output(false, &refusal, Some(extra));
    output.stderr.extend(note_lines(&[note]).into_bytes());
    output
}

/// The human output of a result of these verbs.
pub fn output(envelope: &Envelope<MutationDataDto>, supplied: bool) -> Output {
    let Some(MutationDataDto::Lifecycle(data)) = &envelope.data else {
        return render::mutation_output(false, envelope, supplied);
    };
    let route = route_of(envelope, data);
    let mut notes = notes_of(data);
    notes.extend(route.notes);
    let mut shown = envelope.clone();
    shown.data = None;
    let mut body = data_fields(data);
    body.push_str(&made_ready(data));
    for line in &route.next {
        body.push_str(&format!("next: {line}\n"));
    }
    if envelope.failure_class().is_none() {
        let mut stdout = human_admin::mutation_success(&shown).stdout;
        stdout.push_str(&body);
        return Output {
            stdout: stdout.into_bytes(),
            stderr: note_lines(&notes).into_bytes(),
            status: render::exit_status(None),
        };
    }
    let mut extra = human_admin::failure_extra(&shown, supplied);
    extra.push_str(&body);
    let recovery = envelope
        .recovery
        .iter()
        .enumerate()
        .filter(|(index, _)| !route.rendered.contains(index))
        .map(|(_, action)| action.clone())
        .collect();
    let refusal = refusal_of(envelope, recovery);
    let mut output = render::failure_output(false, &refusal, Some(extra));
    output.stderr.extend(note_lines(&notes).into_bytes());
    output
}
