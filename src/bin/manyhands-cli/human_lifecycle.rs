//! Human output of `document promote`, `ticket close` and an `operation
//! resume` that continued one. Every result keeps C2's lines, and the data's
//! field list, less what is said better here:
//!
//! - `made ready:` lines, with slugs, in place of the field list's;
//! - a note when publication waits for a remote, or no made-ready set is held;
//! - for a refused checkpoint merge, the local repair: four `next:` lines naming
//!   `context observe-local`, `sync-local`, `show-local` and `resolve-local`,
//!   before C2's lines for the same request and `repo inspect`. No recovery
//!   action names them, and none may be added (design, Answers). Those verbs
//!   arrive in Task 10, whose grammar answers to this text, which
//!   `lifecycle_tests.rs` pins;
//! - for a child's `remote_branch_deleted`, the route out of decision 18:
//!   cancel, the same request, the republish, the same request. Under a
//!   resume, decision 19's: the republish, then the resume, since the resume
//!   has already asked for the cancellation that lets the child go.

use crate::{
    human_admin::{self, REPUBLISH},
    render::{self, Output, field, note_lines, quoted},
};
use manyhands::{
    repository::{
        LifecycleBlocker, LifecycleMadeReadyState, LifecycleMutationDto,
        LifecyclePublicationPendingReason, MutationDataDto,
    },
    results::{Envelope, RecoveryActionKind, ResultCode},
};

const MERGE_REFUSED_NOTE: &str = "primary refused this closure's checkpoint: bring primary into the ticket's context with the context commands, then send this request again; the closure's fields are kept.";
const NO_REMOTE_NOTE: &str = "primary is not published, because this repository has no publication remote; once one is selected, repo sync publishes it.";
const NO_SET_NOTE: &str = "this answer holds no made-ready set; ticket ready is the live view.";
const DELETED_NOTE: &str = "the refused request's context synchronization keeps its reservation until the request is cancelled; sending the request again then answers cancelled and releases it. After the republish, sending it once more finishes the work.";
/// The answer doesn't tell a released child from a kept one (decision 19).
const RESUME_DELETED_NOTE: &str = "this resume released the operation's context synchronization if nothing it holds could be lost. If repo inspect still shows its reservation, it was kept, and the republish answers busy.";

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

fn republish(root: &str, item: &str) -> String {
    format!(
        "manyhands-cli {REPUBLISH} --repo {} --id {} --prepare",
        quoted(root),
        field(item)
    )
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
        let operation = envelope.recovery[index]
            .operation_id
            .as_deref()
            .or(envelope.operation_id.as_deref());
        route.next.push(republish(root, item));
        if let Some(operation) = operation {
            route.rendered.push(index);
            route.next.push(format!(
                "manyhands-cli operation resume --repo {} --operation-id {}",
                quoted(root),
                field(operation)
            ));
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

fn route_of(envelope: &Envelope<MutationDataDto>, data: &LifecycleMutationDto) -> Route {
    let mut route = Route::default();
    let Some(root) = envelope.scope.repository.as_deref() else {
        return route;
    };
    match envelope.code {
        ResultCode::RecoveryRequired
            if data
                .blockers
                .contains(&LifecycleBlocker::IntegrationConflict) =>
        {
            route.next = merge_refused(root, &data.item_id);
            route.notes.push(MERGE_REFUSED_NOTE.to_owned());
        }
        ResultCode::RemoteBranchDeleted => {
            branch_deleted(envelope, root, &data.item_id, &mut route);
        }
        _ => {}
    }
    route
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
        recovery: envelope
            .recovery
            .iter()
            .enumerate()
            .filter(|(index, _)| !route.rendered.contains(index))
            .map(|(_, action)| action.clone())
            .collect(),
    };
    let mut output = render::failure_output(false, &refusal, Some(extra));
    output.stderr.extend(note_lines(&notes).into_bytes());
    output
}
