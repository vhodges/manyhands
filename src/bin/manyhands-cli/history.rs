//! The remote history repair verbs: `remote history`, `item resolve-history`
//! and `remote resolve-history`.
//!
//! `remote history` is a read that polls: it runs the manual poll, in the
//! Ctrl-C window and with C2's unlock prompt at a terminal, then lists the
//! contexts whose publication history this repository has lost. It is not
//! journaled as a request: it changes no record that a request could be asked
//! about, so it takes no `--request-id`, and its operation and request IDs
//! are fresh for every call (a replayed terminal poll fetches nothing).
//!
//! The two resolve verbs are C2's confirmed flow around one library mutation,
//! and their `next:` lines come from the library's `HistoryWayOut`.

use clap::ArgMatches;
use manyhands::{
    repository::{
        HistoryRelation, HistoryWayOut, ItemResolveHistoryInput, Mutation, MutationDataDto,
        OperationId, PrepareDataDto, RemoteHistoryDto, RemoteHistoryEntryDto,
        RemoteResolveHistoryInput, RepositoryService, RequestId, ResolveHistoryChoice,
        clear_cache_loss_way_out, keys::SessionCredentials, resolve_history_way_out,
    },
    results::{Envelope, RecoveryAction, RecoveryActionKind, ResultCode, Scope},
};
use std::{cell::Cell, io::Write, path::PathBuf};

use crate::{
    admin, authoring,
    catalog::CommandSpec,
    fill, human_collaboration,
    mutate::{self, MutationFlags, Session},
    prompt::TerminalProvider,
    render::{self, Output, field, note_line, quoted},
    terminal::Terminal,
};

/// What a context's `release` choice amounts to in the CLI: the release of a
/// context is the desktop's, and no verb of this catalog does it.
const RELEASE_NOTE: &str = "to give up this item's local copy instead, release its editing context in the desktop app; the CLI has no command for it.";
/// A synchronization's `recovery_required` for unresolved history.
pub(crate) const SYNC_NOTE: &str = "this repository can't tell whether the remote already holds this item's branch, and a sync can't settle that: remote history polls the remote and lists the contexts it can't confirm, with a choice for each.";
const FLAG_NOTE: &str = "a context still reads unknown, so the lost-history flag stays; resolve each context remote history lists, then clear the flag.";
const MARKER_NOTE: &str = "the flag is cleared, but the marker file could not be deleted; it is ignored while the flag stays clear.";
const PASSPHRASE_HINT: &str = "hint: if the passphrase was wrong, run the command again\n";

/// `manyhands-cli remote history --repo <root>`.
pub(crate) fn remote_history_line(root: &str) -> String {
    format!("manyhands-cli remote history --repo {}", quoted(root))
}

/// The preview of the resolution of one item, as a pasteable command.
fn resolve_line(root: &str, item: &str) -> String {
    format!(
        "manyhands-cli item resolve-history --repo {} --id {} --as publish --prepare",
        quoted(root),
        field(item)
    )
}

fn sync_line(root: &str, item: &str) -> String {
    format!(
        "manyhands-cli item sync --repo {} --id {}",
        quoted(root),
        field(item)
    )
}

/// What a front end says of one step of a resolution's way out: a `next:`
/// command, or a note where no command exists.
fn way_out(step: &HistoryWayOut, root: &str) -> (Option<String>, Option<&'static str>) {
    match step {
        HistoryWayOut::Sync { item_id } => (Some(sync_line(root, item_id)), None),
        HistoryWayOut::Republish { item_id } => {
            (Some(human_collaboration::republish(root, item_id)), None)
        }
        HistoryWayOut::RemoteHistory => (Some(remote_history_line(root)), Some(FLAG_NOTE)),
        HistoryWayOut::ReleaseContext { .. } => (None, Some(RELEASE_NOTE)),
    }
}

// --- remote history ---------------------------------------------------------

fn flag(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

/// The relation of a context's head to the remote's branch, in words.
fn relation(entry: &RemoteHistoryEntryDto) -> String {
    let Some(relation) = entry.relation else {
        return if entry.remote_oid.is_none() {
            "local-only".to_owned()
        } else {
            "unknown".to_owned()
        };
    };
    let mut text = relation.as_str().replace('_', "-");
    // The counts say nothing of a head that is the remote's.
    if relation != HistoryRelation::Same
        && let (Some(ahead), Some(behind)) = (entry.ahead, entry.behind)
    {
        text.push_str(&format!(" ahead={ahead} behind={behind}"));
    }
    text
}

fn choices(entry: &RemoteHistoryEntryDto) -> String {
    let names: Vec<&str> = [
        ("publish", entry.choices.publish),
        ("release", entry.choices.release),
    ]
    .into_iter()
    .filter_map(|(name, offered)| offered.then_some(name))
    .collect();
    if names.is_empty() {
        String::new()
    } else {
        format!("  choices={}", names.join(","))
    }
}

/// One line per context, `Cache loss:`, then `next:` lines for the
/// resolutions on offer, with a note where a choice has no command.
pub(crate) fn report(root: &str, data: &RemoteHistoryDto) -> render::Human {
    let mut stdout = String::new();
    if data.entries.is_empty() {
        stdout.push_str("Contexts: none\n");
    }
    for entry in &data.entries {
        stdout.push_str(&format!(
            "{}  {}  {}  {}  {}  {}  {}{}\n",
            field(&entry.item_id),
            entry.kind.as_str(),
            field(&entry.branch),
            if entry.remote_oid.is_some() {
                "present"
            } else {
                "absent"
            },
            relation(entry),
            entry.disposition.map_or("-", |d| d.as_str()),
            if entry.resolved {
                "resolved"
            } else {
                "unresolved"
            },
            choices(entry),
        ));
    }
    stdout.push_str(&format!("Cache loss: {}\n", flag(data.cache_loss)));
    let mut notes = Vec::new();
    let offered: Vec<&RemoteHistoryEntryDto> = data
        .entries
        .iter()
        .filter(|entry| !entry.resolved && entry.choices.publish)
        .collect();
    for entry in &offered {
        stdout.push_str(&format!("next: {}\n", resolve_line(root, &entry.item_id)));
    }
    if !offered.is_empty() {
        notes.push(RELEASE_NOTE.to_owned());
    }
    let unresolved = data.entries.iter().any(|entry| !entry.resolved);
    if data.cache_loss && !unresolved {
        stdout.push_str(&format!(
            "next: manyhands-cli remote resolve-history --repo {} --prepare\n",
            quoted(root)
        ));
    }
    render::Human { stdout, notes }
}

/// A stop of the poll, as the envelope a read gives: the code `poll once`
/// gives the same stop, and the actions that code offers here.
fn stop_envelope(
    error: &manyhands::repository::RemoteHistoryError,
    scope: Scope,
    root: &str,
) -> Envelope<serde_json::Value> {
    let code = error.result_code();
    let mut recovery = Vec::new();
    let host = match code {
        ResultCode::HostApprovalRequired => Some(RecoveryActionKind::HostApprove),
        ResultCode::HostReplacementRequired => Some(RecoveryActionKind::HostReplace),
        _ => None,
    };
    if let (Some(action), Some(authority)) = (host, error.host_authority()) {
        recovery.push(RecoveryAction::new(
            action,
            [("authority", authority.into()), ("root", root.into())],
        ));
    }
    if code == ResultCode::RecoveryRequired {
        recovery.push(RecoveryAction::new(
            RecoveryActionKind::RepoInspect,
            [("root", root.into())],
        ));
    }
    Envelope::failure("remote history", scope, code, recovery)
}

pub fn remote_history(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    let root = match admin::mutation_root(session, spec, args, None) {
        Ok(root) => root,
        Err(out) => return out,
    };
    if !session.json {
        let _ = writeln!(std::io::stderr().lock(), "progress: connecting");
    }
    crate::interrupt::reset();
    let supplied = Cell::new(false);
    let terminal = if session.interactive {
        session
            .terminal
            .as_deref_mut()
            .map(|t| t as &mut dyn Terminal)
    } else {
        None
    };
    let mut credentials = SessionCredentials::new(TerminalProvider::new(terminal, &supplied));
    // A fresh operation, and a request ID only the poll's own probe reads.
    let (operation, request) = (OperationId::new(), RequestId::new());
    let result = crate::interrupt::during(|token| {
        service.remote_history(&root, operation, request, token.clone(), &mut credentials)
    });
    let shown = root.to_string_lossy().into_owned();
    let scope = service
        .resolve_repository(&root)
        .map(|repo| repo.scope())
        .unwrap_or_else(|_| Scope {
            repository: Some(shown.clone()),
            ..Scope::default()
        });
    let repository = scope.repository.clone().unwrap_or(shown);
    match result {
        Ok(report_data) => {
            let data = RemoteHistoryDto::from(&report_data);
            let envelope = Envelope::read_success("remote history", scope, data);
            render::envelope_output(session.json, &envelope, |data| report(&repository, data))
        }
        Err(error) => {
            let envelope = stop_envelope(&error, scope, &repository);
            let extra = (supplied.get() && envelope.code == ResultCode::TransportUnavailable)
                .then(|| PASSPHRASE_HINT.to_owned());
            render::failure_output(session.json, &envelope, extra)
        }
    }
}

// --- the resolve verbs ------------------------------------------------------

/// The `next:` lines and notes of a way out, for output on `root`.
fn lines(step: Option<&HistoryWayOut>, root: &str) -> (String, String) {
    let Some(step) = step else {
        return (String::new(), String::new());
    };
    let (next, note) = way_out(step, root);
    (
        next.map(|line| format!("next: {line}\n"))
            .unwrap_or_default(),
        note.map(note_line).unwrap_or_default(),
    )
}

/// A preview, or the refusal of one, with the way out of a refusal. A
/// preview that is given says to confirm, and names no next step.
pub(crate) fn prepared_output(
    envelope: &Envelope<PrepareDataDto>,
    way_out: Option<&HistoryWayOut>,
) -> Output {
    let mut output = render::prepare_output(false, envelope);
    if envelope.code == ResultCode::Ok {
        return output;
    }
    let root = envelope.scope.repository.as_deref().unwrap_or_default();
    let (next, note) = lines(way_out, root);
    // A refusal prints on stderr; a no-op, on stdout.
    if envelope.failure_class().is_some() {
        let mut stderr = next.into_bytes();
        stderr.extend(note.into_bytes());
        output.stderr.extend(stderr);
    } else {
        output.stdout.extend(next.into_bytes());
        output.stderr.extend(note.into_bytes());
    }
    output
}

/// A result, with its way out: the `next:` line after the lines C2 prints.
pub(crate) fn executed_output(
    envelope: &Envelope<MutationDataDto>,
    supplied: bool,
    way_out: Option<&HistoryWayOut>,
) -> Output {
    let root = envelope.scope.repository.as_deref().unwrap_or_default();
    let (next, mut notes) = lines(way_out, root);
    if let Some(MutationDataDto::ResolveHistory(data)) = &envelope.data
        && data.marker == manyhands::repository::HistoryMarkerState::Failed
    {
        notes.push_str(&note_line(MARKER_NOTE));
    }
    render::compact(
        envelope,
        supplied,
        &next,
        "",
        envelope.recovery.clone(),
        &notes,
    )
}

fn item_resolve_choice(args: &ArgMatches) -> ResolveHistoryChoice {
    match args
        .get_one::<String>("as")
        .expect("required choice")
        .as_str()
    {
        "release" => ResolveHistoryChoice::Release,
        _ => ResolveHistoryChoice::Publish,
    }
}

pub fn item_resolve(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    let (flags, root, id) = match (|| {
        let flags = admin::flags(session, spec, args)?;
        let root = admin::mutation_root(session, spec, args, flags.request_id)?;
        let id = authoring::item_id(session, spec, args, flags.request_id)?;
        Ok((flags, root, id))
    })() {
        Ok(values) => values,
        Err(out) => return out,
    };
    let choice = item_resolve_choice(args);
    // R-2: the item's kind is read before the request is built.
    let kind = match fill::read_any_item(service, session, spec, &root, &id, flags.request_id) {
        Ok((_, item)) => crate::collaboration::authoring_kind(item.kind),
        Err(out) => return out,
    };
    let item = id.to_string();
    let driven = mutate::drive(
        service,
        session,
        spec,
        flags,
        mutate::RETRYING,
        &mut |_| {
            Mutation::ItemResolveHistory(ItemResolveHistoryInput {
                root: root.clone(),
                kind,
                id: id.clone(),
                choice,
            })
        },
        &mut std::thread::sleep,
    );
    mutate::finish(
        session.json,
        driven,
        |envelope| {
            prepared_output(
                envelope,
                resolve_history_way_out(choice, envelope.code, &item).as_ref(),
            )
        },
        |envelope, supplied| {
            executed_output(
                envelope,
                supplied,
                resolve_history_way_out(choice, envelope.code, &item).as_ref(),
            )
        },
    )
}

pub fn remote_resolve(
    service: &RepositoryService,
    session: &mut Session,
    spec: &'static CommandSpec,
    args: &ArgMatches,
) -> Output {
    let (flags, root): (MutationFlags, PathBuf) = match (|| {
        let flags = admin::flags(session, spec, args)?;
        let root = admin::mutation_root(session, spec, args, flags.request_id)?;
        Ok((flags, root))
    })() {
        Ok(values) => values,
        Err(out) => return out,
    };
    let driven = mutate::drive(
        service,
        session,
        spec,
        flags,
        mutate::RETRYING,
        &mut |_| Mutation::RemoteResolveHistory(RemoteResolveHistoryInput { root: root.clone() }),
        &mut std::thread::sleep,
    );
    mutate::finish(
        session.json,
        driven,
        |envelope| prepared_output(envelope, clear_cache_loss_way_out(envelope.code).as_ref()),
        |envelope, supplied| {
            executed_output(
                envelope,
                supplied,
                clear_cache_loss_way_out(envelope.code).as_ref(),
            )
        },
    )
}

#[cfg(test)]
#[path = "history_tests.rs"]
mod tests;
