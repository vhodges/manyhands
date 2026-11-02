//! Human output of `context observe-local`, `context show-local`, `context
//! sync-local` and `context resolve-local`: the verbs' lines and notes, which
//! `local_sync.rs` adds to C2's. Nothing here reads the repository.

use manyhands::{
    repository::{
        ClosureState, ItemContextKind, ItemDto, LocalContextConflictContentDto,
        LocalContextSyncInspectionDto, LocalContextSyncMutationDto, LocalContextSyncNextAction,
        LocalContextSyncObservationDto, LocalContextSyncReason, MutationDataDto,
    },
    results::{Envelope, RecoveryActionKind, ResultCode},
};

use crate::{
    human_admin,
    render::{self, Human, Output, field, quoted},
};

pub(crate) const INPUT_NOTE: &str =
    "context sync-local's --input takes this as {\"observation\": \"...\"}.";
pub(crate) const SIDES_NOTE: &str =
    "context show-local --token <token> --json reads a path's sides.";
pub(crate) const DIRTY_NOTE: &str =
    "commit or save what the context holds, then observe again, and use a new request.";
pub(crate) const MALFORMED_NOTE: &str =
    "this observation can't be used: observe again, and use a new request.";
pub(crate) const OBSERVE_NOTE: &str =
    "sending this observation again gets the same answer: observe again, and use a new request.";
pub(crate) const RESOLVE_NOTE: &str = "<new-request-id> is a placeholder to fill in, and --input - takes the observation and a source for each token that context show-local prints.";
pub(crate) const IDENTITY_NOTE: &str =
    "once an identity is set, send this same request again: a merge commit needs one.";
pub(crate) const ORIGINAL_NOTE: &str = "this resolution is the original request's to finish: send that request again, with the same input.";
pub(crate) const EXTERNAL_NOTE: &str =
    "Manyhands can't resolve this: it takes a repair outside it, with Git.";
/// What follows the repair of a refusal that kept nothing to go on with.
pub(crate) const EXTERNAL_REFUSED_NOTE: &str = "Manyhands can't resolve this: it takes a repair outside it, with Git. After the repair, observe again, and use a new request.";

/// What follows the repair of a conflict a request holds: that request again.
pub(crate) fn external_then(holder: &str, request: &str) -> String {
    format!(
        "{EXTERNAL_NOTE} After the repair, send {holder} again ({}).",
        field(request)
    )
}

fn observe_line(root: &str, item: &str) -> String {
    format!(
        "next: manyhands-cli context observe-local --repo {} --id {}\n",
        quoted(root),
        field(item)
    )
}

/// What an observation's refusal adds: a save, when the item's effective copy
/// is primary's and a save of it is allowed, as lifecycle's `ContextMissing`
/// way out. A context that has lost the item, or a closed ticket, gets none.
pub(crate) fn no_context(item: &ItemDto, root: &str, id: &str) -> Option<String> {
    let closed = item
        .closure
        .as_ref()
        .is_some_and(|closure| closure.state == ClosureState::Closed);
    (item.context.kind == ItemContextKind::Primary && !closed).then(|| {
        format!(
            "next: {}\n{}",
            render::save_line(item.kind, root, id),
            render::note_line(render::NO_CONTEXT_NOTE)
        )
    })
}

pub(crate) fn observation(data: &LocalContextSyncObservationDto) -> Human {
    Human {
        stdout: human_admin::fields(data),
        notes: vec![INPUT_NOTE.to_owned()],
    }
}

/// A token's sides print with `--json` only, which `show` holds to; in human
/// mode this renderer is never reached, and would print no side.
pub(crate) fn sides(_: &LocalContextConflictContentDto) -> Human {
    Human {
        stdout: String::new(),
        notes: Vec::new(),
    }
}

/// The inspection's fields, then a line per conflicted path. No side.
pub(crate) fn inspected(data: &LocalContextSyncInspectionDto) -> Human {
    let mut stdout = human_admin::fields_without(data, &["paths"]);
    for path in &data.paths {
        stdout.push_str(&format!(
            "path: {} token={} content_available={}\n",
            field(&path.display_path),
            field(&path.token),
            path.content_available
        ));
    }
    Human {
        stdout,
        notes: if data.paths.is_empty() {
            Vec::new()
        } else {
            vec![SIDES_NOTE.to_owned()]
        },
    }
}

pub(crate) fn show_line(root: &str, operation: &str) -> String {
    format!(
        "next: manyhands-cli context show-local --repo {} --operation-id {}\n",
        quoted(root),
        field(operation)
    )
}

/// The `resolve-local` of a conflict `show_line` inspects, its request a
/// placeholder.
fn resolve_line(root: &str, operation: &str) -> String {
    format!(
        "next: manyhands-cli context resolve-local --repo {} --operation-id {} --input - --request-id <new-request-id>\n",
        quoted(root),
        field(operation)
    )
}

/// The lines a verb adds to a result's, and whether one of them, a line to
/// look again, replaces the data's `next action`: there, looking again is the
/// way on, not a repair outside.
pub(crate) struct WayOn {
    pub(crate) lines: String,
    pub(crate) replaces_next_action: bool,
}

impl WayOn {
    fn adds(lines: String) -> Self {
        Self {
            lines,
            replaces_next_action: false,
        }
    }

    fn looks_again(lines: String) -> Self {
        Self {
            lines,
            replaces_next_action: true,
        }
    }
}

/// What a resolution's stop adds, by the required outcomes' last column. With
/// an original request to finish the resolution, that request is the way on,
/// as `request.retry`, unless C2's lines already name it.
pub(crate) fn resolve_way_on(
    envelope: &Envelope<MutationDataDto>,
    root: &str,
    operation: &str,
    original: Option<&str>,
) -> WayOn {
    if let Some(original) = original.filter(|id| envelope.request_id.as_deref() != Some(*id)) {
        let named = envelope.recovery.iter().any(|action| {
            action.action == RecoveryActionKind::RequestRetry
                && action.arguments.get("request_id").and_then(|v| v.as_str()) == Some(original)
        });
        let retry = if named {
            String::new()
        } else {
            format!("next: request.retry request_id={}\n", field(original))
        };
        return WayOn::adds(format!("{retry}{}", render::note_line(ORIGINAL_NOTE)));
    }
    match envelope.code {
        ResultCode::InvalidInput | ResultCode::ExternalChange => {
            WayOn::looks_again(show_line(root, operation))
        }
        ResultCode::OriginalRequestRequired => WayOn::adds(render::note_line(ORIGINAL_NOTE)),
        _ => WayOn::adds(String::new()),
    }
}

/// A local synchronization's result in human mode: C2's lines, with the
/// data's non-empty fields only, as `human_collaboration`'s compact lines do,
/// and no `operation:` line when the data names no operation (a refusal with
/// no owner). `reobserve` drops a `next action` that the verb's own line
/// replaces (`WayOn`).
pub(crate) fn local_output(
    envelope: &Envelope<MutationDataDto>,
    supplied: bool,
    reobserve: bool,
) -> Output {
    let mut plain = envelope.clone();
    plain.data = None;
    let mut body = String::new();
    if let Some(data) = data_of(envelope) {
        if data.operation_id.is_none() {
            plain.operation_id = None;
        }
        if let Ok(serde_json::Value::Object(mut fields)) = serde_json::to_value(data) {
            fields.remove("type");
            if reobserve {
                fields.remove("next_action");
            }
            fields.retain(|_, value| {
                !matches!(
                    value,
                    serde_json::Value::Null | serde_json::Value::Bool(false)
                ) && value.as_str() != Some("none")
            });
            body = human_admin::fields(&serde_json::Value::Object(fields));
        }
    }
    render::compact(&plain, supplied, &body, "", plain.recovery.clone(), "")
}

pub(crate) fn data_of(
    envelope: &Envelope<MutationDataDto>,
) -> Option<&LocalContextSyncMutationDto> {
    match &envelope.data {
        Some(MutationDataDto::LocalContextSync(data)) => Some(data),
        _ => None,
    }
}

/// What a stop's human lines add to C2's, by the required outcomes' last column.
pub(crate) fn way_on(envelope: &Envelope<MutationDataDto>, root: &str, item: &str) -> WayOn {
    let notes = |note: &str| render::note_line(note);
    let again =
        |note: &str| WayOn::looks_again(format!("{}{}", observe_line(root, item), notes(note)));
    match envelope.code {
        ResultCode::InvalidInput => return again(MALFORMED_NOTE),
        ResultCode::ExternalChange => return again(OBSERVE_NOTE),
        ResultCode::WorktreeNotClean => return again(DIRTY_NOTE),
        ResultCode::IdentityRequired => return WayOn::adds(notes(IDENTITY_NOTE)),
        _ => {}
    }
    let Some(data) = data_of(envelope) else {
        return WayOn::adds(String::new());
    };
    WayOn::adds(match data.next_action {
        LocalContextSyncNextAction::Resolve => match data.operation_id.as_deref() {
            Some(operation) => {
                format!(
                    "{}{}{}",
                    show_line(root, operation),
                    resolve_line(root, operation),
                    notes(RESOLVE_NOTE)
                )
            }
            None => String::new(),
        },
        // The refusals the gate leaves to a repair outside (gate 10): the
        // library offers nothing for two of them, and `external_repair` for
        // the third. A closure's or a lost authority's `external_repair` has
        // its code's way on, so it gets no such note.
        _ if matches!(
            data.reason,
            LocalContextSyncReason::UnrelatedHistories
                | LocalContextSyncReason::ImmutableClosureDisagreement
                | LocalContextSyncReason::UnsupportedConflict
        ) =>
        {
            match (&data.operation_id, envelope.request_id.as_deref()) {
                (Some(_), Some(request)) => notes(&external_then("this same request", request)),
                _ => notes(EXTERNAL_REFUSED_NOTE),
            }
        }
        _ => String::new(),
    })
}

#[cfg(test)]
#[path = "human_local_sync_tests.rs"]
mod tests;
