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
//! - for a blocked prepare, its way out: the same repair, with a note that an
//!   `external_resolution_required` answer is repaired with Git in the
//!   context (an add/add, decision 30), the save that makes a missing
//!   context, or the poll that
//!   reads a primary tracking ref left behind or missing, or a published
//!   context whose remote state is unknown (decision 35), and then a new
//!   prepare;
//! - for a prepare refused because an unfinished request or operation holds
//!   the item (decision 33), that request sent again, or the resume;
//! - for a context synchronization stopped on a conflict, decision 34's route:
//!   `conflict show` and `conflict resolve` of this operation, which name its
//!   context child, and their note, before C2's lines for the same request, or
//!   the resume. Only when the blockers name the item's own context conflicted
//!   as well as `conflict_pending`, which alone is repository-wide: the answer
//!   holds no field that says the conflict is this operation's child's;
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
        LifecyclePublicationPendingReason, LifecycleWayOut, MutationDataDto, PrepareDataDto,
        lifecycle_prepare_way_out, lifecycle_way_out,
    },
    results::{Envelope, RecoveryActionKind, ResultCode},
};

/// What the repair's lines leave to the reader. The route these lines print is
/// walked through the binary, from a refused closure to its finish, by
/// `tests/cli/local_sync.rs` `the_refused_merges_route_finishes_the_close`.
const REPAIR_NOTE: &str = "The commands above are the local repair: the <...> values are placeholders to fill in, context sync-local's --input - takes {\"observation\": \"...\"} with the observation the first line prints, and context show-local and context resolve-local apply only if context sync-local answers a conflict.";
const EXTERNAL_REPAIR_NOTE: &str = "If context sync-local answers external_resolution_required, repair it with Git in the context.";
const REFUSED_NOTE: &str = "primary refused this closure's checkpoint.";
const POLL_NOTE: &str = "this repository has no record of the remote's primary, and a poll reads it; <new-request-id> is a placeholder to fill in.";
const UNKNOWN_NOTE: &str = "this repository hasn't read whether the remote still holds this item's branch, and a poll reads it; <new-request-id> is a placeholder to fill in.";
const AHEAD_NOTE: &str = "primary holds commits the remote's primary, as this repository last read it, lacks: a poll reads the remote again, and if primary is still ahead, repo sync publishes it; each <new-request-id> is a placeholder for a request of its own.";
const HELD_NOTE: &str = "an unfinished request holds this item; send it again to finish it, and prepare again once it has finished, or has stopped on a cleanup a new request can replace.";
const HELD_RESUME_NOTE: &str = "an unfinished operation that no request holds has this item; resume it (add --request-id unless you run it at a terminal), and prepare again only once it has finished.";
const NO_REMOTE_NOTE: &str = "primary is not published, because this repository has no publication remote; once one is selected, repo sync publishes it.";
const NO_SET_NOTE: &str = "this answer holds no made-ready set; ticket ready is the live view.";
const DELETED_NOTE: &str = "the refused request's context synchronization keeps its reservation until the request is cancelled; sending the request again then answers cancelled and releases it. After the republish, sending it once more finishes the work.";
/// Decision 34's route out of a context synchronization's conflict.
const CONFLICT_NOTE: &str = "this operation's context synchronization stopped on a conflict. conflict show reads it, and --json gives each path's sides and token; when its next action is resolve, conflict resolve with the observation and a source for every token resolves and publishes it, and <new-request-id> is a placeholder to fill in. When it is external_repair, repair it with Git in the context and commit the merge.";
/// What decides the way on once the conflict is resolved (decision 34's
/// review): consent, and a raised generation.
const CONFLICT_CONSENT_NOTE: &str = "The request finishes only if the resolved context holds what was confirmed; otherwise it answers external_change, and a new prepare is needed. After key select or key clear, the resolution is refused and there is no way on (a known limit).";
/// The answer doesn't tell a released child from a kept one (decision 19).
const RESUME_DELETED_NOTE: &str = "this resume released the operation's context synchronization if nothing it holds could be lost. If repo inspect still shows its reservation, it was kept, and the republish answers busy. Add --request-id to the resume unless you run it at a terminal.";

/// The data's fields, without its tag, its made-ready set (`made_ready`) and,
/// for a promotion, a state that says nothing. Its blockers are one line,
/// which sorts first.
fn data_fields(data: &LifecycleMutationDto) -> String {
    let mut left_out = vec!["type", "blockers", "made_ready"];
    if data.made_ready_state.is_none() {
        left_out.push("made_ready_state");
    }
    format!(
        "{}{}",
        blockers_line(&data.blockers),
        human_admin::fields_without(data, &left_out)
    )
}

/// `Blockers:` once, with the list.
fn blockers_line(blockers: &[LifecycleBlocker]) -> String {
    if blockers.is_empty() {
        return "Blockers: none\n".into();
    }
    let names: Vec<&str> = blockers.iter().map(|blocker| blocker.as_str()).collect();
    format!("Blockers: {}\n", names.join(", "))
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

/// The same request, sent again by its command.
fn again(command: &str, root: &str, item: &str, request: &str) -> String {
    format!(
        "manyhands-cli {} --repo {} --id {} --request-id {}",
        field(command),
        quoted(root),
        field(item),
        field(request)
    )
}

/// The `operation resume` of `operation`.
fn resume_line(root: &str, operation: &str) -> String {
    format!(
        "manyhands-cli operation resume --repo {} --operation-id {}",
        quoted(root),
        field(operation)
    )
}

/// A context child's conflict (decision 34): its read and its resolution,
/// named by this operation's ID, which the conflict verbs take for its
/// context child.
fn child_conflict(root: &str, operation: &str) -> Vec<String> {
    let (root, operation) = (quoted(root), field(operation));
    vec![
        format!("manyhands-cli conflict show --repo {root} --operation-id {operation}"),
        format!(
            "manyhands-cli conflict resolve --repo {root} --operation-id {operation} --input - --request-id <new-request-id>"
        ),
    ]
}

/// L1's way out of a result, in its lines and notes. A step that prints no
/// line is C2's action (`SendAgain` as `request.retry`, and `Inspect`), or is
/// left to the actions (`RepairOutside`). A deleted branch's answer offers no
/// `request.retry` (`never_gets_past`), so its route names the request itself.
fn route_of(envelope: &Envelope<MutationDataDto>, data: &LifecycleMutationDto) -> Route {
    let mut route = Route::default();
    let Some(root) = envelope.scope.repository.as_deref() else {
        return route;
    };
    let retry = action_index(envelope, RecoveryActionKind::RequestRetry).is_some();
    let resume = action_index(envelope, RecoveryActionKind::OperationResume);
    let again_of = |request: &str| again(&envelope.command, root, &data.item_id, request);
    for step in lifecycle_way_out(envelope) {
        match step {
            // The repair, when the same request or a resume can then go on:
            // `sync-local` admits a closure only while it is recording its own
            // (`local_sync/record.rs`).
            LifecycleWayOut::RepairLocally { item_id } => {
                route.next.extend(merge_refused(root, &item_id));
                let then = if retry {
                    "Then send this request again;"
                } else {
                    "Then resume it again (add --request-id unless you run it at a terminal);"
                };
                route.notes.push(format!(
                    "{REFUSED_NOTE} {REPAIR_NOTE} {then} the closure's fields are kept."
                ));
            }
            LifecycleWayOut::ResolveChildConflict { operation_id } => {
                route.next.extend(child_conflict(root, &operation_id));
                let then = if retry {
                    " Then send this request again."
                } else if resume.is_some() {
                    " Then resume it again (add --request-id unless you run it at a terminal)."
                } else {
                    ""
                };
                // The note goes with the lines, whatever follows them.
                route
                    .notes
                    .push(format!("{CONFLICT_NOTE}{then} {CONFLICT_CONSENT_NOTE}"));
            }
            LifecycleWayOut::CancelThenSendAgain { request_id } => {
                route
                    .rendered
                    .extend(action_index(envelope, RecoveryActionKind::RequestCancel));
                route.next.extend([
                    format!(
                        "manyhands-cli request cancel --request-id {}",
                        field(&request_id)
                    ),
                    again_of(&request_id),
                ]);
                route.notes.push(DELETED_NOTE.to_owned());
            }
            LifecycleWayOut::Republish { item_id } => {
                route.next.push(republish(root, &item_id));
                if resume.is_some() {
                    route.notes.push(RESUME_DELETED_NOTE.to_owned());
                }
            }
            LifecycleWayOut::SendAgain if envelope.code == ResultCode::RemoteBranchDeleted => {
                if let Some(request) = envelope.request_id.as_deref() {
                    route.next.push(again_of(request));
                }
            }
            LifecycleWayOut::ResumeOperation { operation_id } => {
                route.rendered.extend(resume);
                route.next.push(resume_line(root, &operation_id));
            }
            // C2's actions say these, or (`RepairOutside`) nothing does; the
            // rest are a prepare's.
            LifecycleWayOut::SendAgain
            | LifecycleWayOut::Inspect
            | LifecycleWayOut::RepairOutside
            | LifecycleWayOut::RetryRequest { .. }
            | LifecycleWayOut::SharePrimary
            | LifecycleWayOut::PollOnce
            | LifecycleWayOut::ResolveHistory
            | LifecycleWayOut::SaveItem { .. } => {}
        }
    }
    route
}

/// What a blocked prepare's way out says of itself, chosen from its blockers.
fn way_out_note(blockers: &[LifecycleBlocker], way_out: &[LifecycleWayOut]) -> String {
    let note = if way_out.contains(&LifecycleWayOut::SharePrimary) {
        AHEAD_NOTE.to_owned()
    } else if way_out.contains(&LifecycleWayOut::PollOnce) {
        // What a poll reads: primary's tracking, and whether a published
        // context still stands, which an absent tracking ref leaves unknown
        // (decision 35).
        if blockers.contains(&LifecycleBlocker::PrimaryMissing) {
            POLL_NOTE.to_owned()
        } else {
            UNKNOWN_NOTE.to_owned()
        }
    } else if way_out.contains(&LifecycleWayOut::ResolveHistory) {
        "this item's shared copy can't be confirmed; a poll can't settle it. Repair it, then prepare again."
            .to_owned()
    } else if way_out
        .iter()
        .any(|step| matches!(step, LifecycleWayOut::SaveItem { .. }))
    {
        render::NO_CONTEXT_NOTE.to_owned()
    } else {
        format!("this context can't merge into primary. {REPAIR_NOTE}")
    };
    format!("{note} Then prepare again.")
}

/// The lines of a blocked prepare's way out.
fn way_out_lines(kind: ItemDtoKind, way_out: &[LifecycleWayOut], root: &str) -> Vec<String> {
    let mut next = Vec::new();
    for step in way_out {
        match step {
            LifecycleWayOut::PollOnce => next.push(format!(
                "manyhands-cli poll once --repo {} --request-id <new-request-id>",
                quoted(root)
            )),
            // Task 9 words this; the line is the command that lists the repair.
            LifecycleWayOut::ResolveHistory => next.push(format!(
                "manyhands-cli remote history --repo {}",
                quoted(root)
            )),
            LifecycleWayOut::SharePrimary => next.push(format!(
                "manyhands-cli repo sync --repo {} --request-id <new-request-id>",
                quoted(root)
            )),
            LifecycleWayOut::SaveItem { item_id } => {
                next.push(render::save_line(kind, root, item_id));
            }
            LifecycleWayOut::RepairLocally { item_id } => {
                next.extend(merge_refused(root, item_id));
            }
            // `Inspect` is C2's action; the rest are a result's or a held
            // prepare's.
            LifecycleWayOut::Inspect
            | LifecycleWayOut::SendAgain
            | LifecycleWayOut::RetryRequest { .. }
            | LifecycleWayOut::CancelThenSendAgain { .. }
            | LifecycleWayOut::ResumeOperation { .. }
            | LifecycleWayOut::ResolveChildConflict { .. }
            | LifecycleWayOut::Republish { .. }
            | LifecycleWayOut::RepairOutside => {}
        }
    }
    next
}

/// A prepare refused because an unfinished request, or an operation no request
/// holds, has the item (decision 33): the command that goes on with it, in
/// place of the action's own line.
fn held(envelope: &Envelope<PrepareDataDto>, way_out: &[LifecycleWayOut]) -> Option<Output> {
    let (Some(root), Some(item)) = (
        envelope.scope.repository.as_deref(),
        envelope.scope.item_id.as_deref(),
    ) else {
        return None;
    };
    let (line, note) = match way_out {
        [LifecycleWayOut::RetryRequest { request_id }] => {
            (again(&envelope.command, root, item, request_id), HELD_NOTE)
        }
        [LifecycleWayOut::ResumeOperation { operation_id }] => {
            (resume_line(root, operation_id), HELD_RESUME_NOTE)
        }
        _ => return None,
    };
    let refusal = refusal_of(envelope, Vec::new());
    let mut output = render::failure_output(false, &refusal, Some(format!("next: {line}\n")));
    output.stderr.extend(render::note_line(note).into_bytes());
    Some(output)
}

/// The human output of a prepare of these verbs: C2's, with a blocked prepare's
/// way out.
pub fn prepare(kind: ItemDtoKind, envelope: &Envelope<PrepareDataDto>) -> Output {
    let way_out = lifecycle_prepare_way_out(kind, envelope);
    if let Some(output) = held(envelope, &way_out) {
        return output;
    }

    let (Some(PrepareDataDto::LifecycleBlocked(data)), Some(root), Some(_)) = (
        &envelope.data,
        envelope.scope.repository.as_deref(),
        envelope.scope.item_id.as_deref(),
    ) else {
        return render::prepare_output(false, envelope);
    };
    let next = way_out_lines(kind, &way_out, root);
    if next.is_empty() {
        let refusal = refusal_of(envelope, envelope.recovery.clone());
        return render::failure_output(false, &refusal, Some(blockers_line(&data.blockers)));
    }
    let note = way_out_note(&data.blockers, &way_out);
    let mut extra = blockers_line(&data.blockers);
    for line in next {
        extra.push_str(&format!("next: {line}\n"));
    }
    let refusal = refusal_of(envelope, envelope.recovery.clone());
    let mut output = render::failure_output(false, &refusal, Some(extra));
    output.stderr.extend(render::note_line(&note).into_bytes());
    // An add/add is external-only (decision 30): the repair's sync says so.
    if data
        .blockers
        .contains(&LifecycleBlocker::IntegrationConflict)
    {
        output
            .stderr
            .extend(render::note_line(EXTERNAL_REPAIR_NOTE).into_bytes());
    }
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
    let recovery = envelope
        .recovery
        .iter()
        .enumerate()
        .filter(|(index, _)| !route.rendered.contains(index))
        .map(|(_, action)| action.clone())
        .collect();
    render::compact(&shown, supplied, &body, "", recovery, &note_lines(&notes))
}
