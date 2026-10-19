use super::render::Human;
use manyhands::repository::{
    HostPinDto, HostPinListDto, IdentityDto, IndexStatusDto, KeyDto, KeyListDto, NewIdDto,
    OperationDto, OperationListDto, PollingStatusDto, PublicKeyDto, RemoteDto, RemoteListDto,
    RepositoryInspectionDto, RepositoryListDto,
};

fn text(stdout: String) -> Human {
    Human {
        stdout,
        notes: Vec::new(),
    }
}

fn optional(value: Option<&str>) -> &str {
    value.unwrap_or("-")
}

fn count(value: Option<u64>) -> String {
    value.map_or_else(|| "-".to_owned(), |value| value.to_string())
}

fn incomplete(output: &mut Human, complete: bool) {
    if !complete {
        output.notes.push("The list is incomplete.".to_owned());
    }
}

fn index_note(output: &mut Human, state: &str) {
    if matches!(state, "stale" | "never_refreshed") {
        output.notes.push(format!("Index is {state}."));
    }
}

pub fn repository_list(data: &RepositoryListDto) -> Human {
    let mut output = text(String::new());
    for item in &data.items {
        output.stdout.push_str(&format!(
            "{}  {}  configuration={}  index={}  problems={}\n",
            item.root,
            item.accessibility.as_str(),
            item.configuration.state.as_str(),
            item.index.state.as_str(),
            item.problem_count
        ));
        index_note(&mut output, item.index.state.as_str());
    }
    incomplete(&mut output, data.complete);
    output
}

fn remote_line(data: &RemoteDto) -> String {
    format!(
        "{}  fetch={}  push={}  eligible={}  selected={}\n",
        data.name,
        data.fetch_location,
        data.push_location,
        data.publication_eligible,
        data.selected_for_publication
    )
}

pub fn repository_inspection(data: &RepositoryInspectionDto) -> Human {
    let mut output = text(format!(
        "Root: {}\nSelected path: {}\nRegistered: {}\nHead branch: {}\nLocal branches: {}\nConfiguration: {}\nPrimary branch: {}\nPublication remote: {}\nIdentity: {}\n",
        data.root,
        data.selected_path,
        data.registered,
        optional(data.head_branch.as_deref()),
        data.local_branches.join(", "),
        data.configuration.state.as_str(),
        optional(data.configuration.primary_branch.as_deref()),
        optional(data.configuration.publication_remote.as_deref()),
        data.identity_state.as_str()
    ));
    for problem in &data.configuration.problems {
        output.stdout.push_str(&format!(
            "Problem: {} {}\n",
            problem.code.as_str(),
            optional(problem.path.as_deref())
        ));
    }
    for remote in &data.remotes {
        output.stdout.push_str(&remote_line(remote));
    }
    output
}

pub fn identity(data: &IdentityDto) -> Human {
    text(format!(
        "Name: {}\nEmail: {}\nSource: {}\nInitials: {}\nInitials source: {}\n",
        optional(data.name.as_deref()),
        optional(data.email.as_deref()),
        data.source.as_str(),
        optional(data.initials.as_deref()),
        data.initials_source.as_str()
    ))
}

pub fn remote_list(data: &RemoteListDto) -> Human {
    let mut output = text(data.items.iter().map(remote_line).collect());
    incomplete(&mut output, data.complete);
    output
}

pub fn key_list(data: &KeyListDto) -> Human {
    let mut output = text(
        data.items
            .iter()
            .map(|key| {
                format!(
                    "{}  {}  {}  {}  {}\n",
                    key.id,
                    key.label,
                    key.ownership.as_str(),
                    if key.selected { "selected" } else { "-" },
                    optional(key.fingerprint.as_deref())
                )
            })
            .collect(),
    );
    incomplete(&mut output, data.complete);
    output
}

pub fn key_show(data: &KeyDto) -> Human {
    text(format!(
        "ID: {}\nLabel: {}\nOwnership: {}\nSelected: {}\nFingerprint: {}\nPrivate source: {}\nPublic metadata: {}\nPrivate key path: {}\nPublic key path: {}\n",
        data.id,
        data.label,
        data.ownership.as_str(),
        data.selected,
        optional(data.fingerprint.as_deref()),
        data.private_source_state.as_str(),
        data.public_metadata_state.as_str(),
        data.private_key_path,
        optional(data.public_key_path.as_deref())
    ))
}

pub fn public_key(data: &PublicKeyDto) -> Human {
    text(format!("{}\n", data.public_key))
}

fn authority(data: &HostPinDto) -> String {
    if data.host.contains(':') {
        format!("[{}]:{}", data.host, data.port)
    } else {
        format!("{}:{}", data.host, data.port)
    }
}

pub fn host_list(data: &HostPinListDto) -> Human {
    let mut output = text(
        data.items
            .iter()
            .map(|pin| {
                format!(
                    "{}  {}  {}\n",
                    authority(pin),
                    pin.pinned_algorithm,
                    pin.pinned_fingerprint
                )
            })
            .collect(),
    );
    incomplete(&mut output, data.complete);
    if data.reapproval_required {
        output
            .notes
            .push("Host pin reapproval is required.".to_owned());
    }
    output
}

pub fn host_inspect(data: &HostPinDto) -> Human {
    text(format!(
        "Authority: {}\nPinned algorithm: {}\nPinned fingerprint: {}\nReapproval required: {}\n",
        authority(data),
        data.pinned_algorithm,
        data.pinned_fingerprint,
        data.reapproval_required
    ))
}

pub fn index_status(data: &IndexStatusDto) -> Human {
    let mut output = text(format!(
        "State: {}\nRefreshed at: {}\nContexts: {}\nItems: {}\nProblems: {}\n",
        data.state.as_str(),
        optional(data.refreshed_at.as_deref()),
        count(data.context_count),
        count(data.item_count),
        count(data.problem_count)
    ));
    for problem in &data.problems {
        output.stdout.push_str(&format!(
            "Problem: {} {} {}\n",
            problem.code.as_str(),
            optional(problem.worktree.as_deref()),
            optional(problem.path.as_deref())
        ));
    }
    for operation in &data.pending_operations {
        output.stdout.push_str(&operation_line(operation));
    }
    index_note(&mut output, data.state.as_str());
    output
}

pub fn polling_status(data: &PollingStatusDto) -> Human {
    text(format!(
        "Enabled: {}\nPaused: {}\nInterval seconds: {}\nBackoff seconds: {}\nRecovery suspended: {}\nLatest outcome: {}\nLatest observed at: {}\nActive operation ID: {}\nNext eligible at: {}\n",
        data.enabled,
        data.paused,
        data.interval_seconds,
        count(data.backoff_seconds),
        data.recovery_suspended,
        optional(data.latest_outcome.map(|value| value.as_str())),
        optional(data.latest_observed_at.as_deref()),
        optional(data.active_operation_id.as_deref()),
        optional(data.next_eligible_at.as_deref())
    ))
}

fn operation_line(data: &OperationDto) -> String {
    format!(
        "{}  {}  {}  {}  next={}\n",
        optional(data.operation_id.as_deref()),
        data.family.as_str(),
        data.action.as_str(),
        data.state,
        optional(data.next_action.map(|value| value.as_str()))
    )
}

pub fn operation_list(data: &OperationListDto) -> Human {
    let mut output = text(data.items.iter().map(operation_line).collect());
    incomplete(&mut output, data.complete);
    output
}

pub fn operation_show(data: &OperationDto) -> Human {
    text(format!(
        "Operation ID: {}\nFamily: {}\nOwner: {}\nAction: {}\nState: {}\nCompleted step: {}\nNext action: {}\nItem ID: {}\nKey ID: {}\nWorktree: {}\nUpdated at: {}\nFailure code: {}\n",
        optional(data.operation_id.as_deref()),
        data.family.as_str(),
        data.owner.as_str(),
        data.action.as_str(),
        data.state,
        optional(data.completed_step.as_deref()),
        optional(data.next_action.map(|value| value.as_str())),
        optional(data.item_id.as_deref()),
        optional(data.key_id.as_deref()),
        optional(data.worktree.as_deref()),
        optional(data.updated_at.as_deref()),
        optional(data.failure_code.map(|value| value.as_str()))
    ))
}

pub fn new_id(data: &NewIdDto) -> Human {
    Human {
        stdout: format!("{}\n", data.id),
        notes: Vec::new(),
    }
}

#[cfg(test)]
#[path = "human_tests.rs"]
mod tests;
