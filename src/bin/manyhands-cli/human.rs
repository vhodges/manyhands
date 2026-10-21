use super::render::{Human, field};
use manyhands::repository::{
    CommentListDto, CycleListDto, DependencyTreeDto, DependencyTreeNodeDto, DocumentFolderListDto,
    HostPinDto, HostPinListDto, IdentityDto, IndexStatusDto, ItemDto, ItemDtoKind, ItemListDto,
    KeyDto, KeyListDto, NewIdDto, OperationDto, OperationListDto, PlanDto, PollingStatusDto,
    ProblemDto, PublicKeyDto, RemoteDto, RemoteListDto, RepositoryInspectionDto, RepositoryListDto,
};

fn text(stdout: String) -> Human {
    Human {
        stdout,
        notes: Vec::new(),
    }
}

fn optional(value: Option<&str>) -> String {
    field(value.unwrap_or("-"))
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
            field(&item.root),
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
        field(&data.name),
        field(&data.fetch_location),
        field(&data.push_location),
        data.publication_eligible,
        data.selected_for_publication
    )
}

pub fn repository_inspection(data: &RepositoryInspectionDto) -> Human {
    let mut output = text(format!(
        "Root: {}\nSelected path: {}\nRegistered: {}\nHead branch: {}\nLocal branches: {}\nConfiguration: {}\nPrimary branch: {}\nPublication remote: {}\nIdentity: {}\n",
        field(&data.root),
        field(&data.selected_path),
        data.registered,
        optional(data.head_branch.as_deref()),
        data.local_branches
            .iter()
            .map(|branch| field(branch))
            .collect::<Vec<_>>()
            .join(", "),
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
                    field(&key.id),
                    field(&key.label),
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
        field(&data.id),
        field(&data.label),
        data.ownership.as_str(),
        data.selected,
        optional(data.fingerprint.as_deref()),
        data.private_source_state.as_str(),
        data.public_metadata_state.as_str(),
        field(&data.private_key_path),
        optional(data.public_key_path.as_deref())
    ))
}

pub fn public_key(data: &PublicKeyDto) -> Human {
    text(format!("{}\n", data.public_key))
}

fn authority(data: &HostPinDto) -> String {
    if data.host.contains(':') {
        format!("[{}]:{}", field(&data.host), data.port)
    } else {
        format!("{}:{}", field(&data.host), data.port)
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
                    field(&pin.pinned_algorithm),
                    field(&pin.pinned_fingerprint)
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
        field(&data.pinned_algorithm),
        field(&data.pinned_fingerprint),
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
        field(&data.state),
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
        field(&data.state),
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

fn nonconforming(path: &str, problems: &[ProblemDto]) -> String {
    format!(
        "nonconforming  {}  {}\n",
        field(path),
        problems
            .iter()
            .map(|problem| problem.code.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn item_line(item: &ItemDto) -> String {
    if let Some(id) = &item.id {
        let closure = if item.kind == ItemDtoKind::Ticket {
            format!(
                "{}  ",
                optional(item.closure.as_ref().map(|closure| closure.state.as_str()))
            )
        } else {
            String::new()
        };
        format!(
            "{}  {}  {}{}  {}\n",
            field(id),
            optional(item.slug.as_deref()),
            closure,
            optional(item.status.as_deref()),
            optional(item.title.as_deref())
        )
    } else {
        nonconforming(&item.path, &item.problems)
    }
}

pub fn item_list(data: &ItemListDto) -> Human {
    let mut output = text(data.items.iter().map(item_line).collect());
    index_note(&mut output, data.index.state.as_str());
    incomplete(&mut output, data.complete);
    output
}

pub fn item_show(data: &ItemDto) -> Human {
    let mut output = text(format!(
        "ID: {}\nKind: {}\nPath: {}\nTitle: {}\nSlug: {}\nType: {}\nStatus: {}\nProject: {}\nTeam: {}\n",
        optional(data.id.as_deref()),
        data.kind.as_str(),
        field(&data.path),
        optional(data.title.as_deref()),
        optional(data.slug.as_deref()),
        optional(data.ticket_type.as_deref()),
        optional(data.status.as_deref()),
        optional(data.project.as_deref()),
        optional(data.team.as_deref())
    ));
    if let Some(closure) = &data.closure {
        output.stdout.push_str(&format!(
            "Closure: {}\nClosed at: {}\nClosed by: {}\n",
            closure.state.as_str(),
            optional(closure.closed_at.as_deref()),
            optional(closure.closed_by.as_deref())
        ));
    }
    output.stdout.push_str(&format!(
        "Context: {}\nBranch: {}\nWorktree: {}\nChanged at: {}\nIndex: {}\n",
        data.context.kind.as_str(),
        optional(data.context.branch.as_deref()),
        field(&data.context.worktree),
        optional(data.changed_at.as_deref()),
        data.index.state.as_str()
    ));
    if data.id.is_none() {
        output.stdout.push_str("State: nonconforming\n");
    }
    for problem in &data.problems {
        output.stdout.push_str(&format!(
            "Problem: {} {}\n",
            problem.code.as_str(),
            optional(problem.path.as_deref())
        ));
    }
    output.stdout.push('\n');
    // This is the explicitly requested content: preserve it byte-for-byte,
    // including control characters and the presence/absence of a final newline.
    let content = if data.id.is_some() {
        &data.body
    } else {
        &data.source
    };
    output
        .stdout
        .push_str(content.as_deref().unwrap_or_default());
    index_note(&mut output, data.index.state.as_str());
    output
}

pub fn folder_list(data: &DocumentFolderListDto) -> Human {
    let mut output = text(
        data.items
            .iter()
            .map(|folder| {
                format!(
                    "{}{}\n",
                    field(&folder.path),
                    if folder.empty { "  empty" } else { "" }
                )
            })
            .collect(),
    );
    index_note(&mut output, data.index.state.as_str());
    incomplete(&mut output, data.complete);
    output
}

pub fn comment_list(data: &CommentListDto) -> Human {
    let mut output = text(String::new());
    for comment in &data.items {
        for _ in 0..comment.depth {
            output.stdout.push_str("  ");
        }
        if let Some(id) = &comment.id {
            output.stdout.push_str(&format!(
                "{}  {}  {}\n",
                field(id),
                optional(comment.author.as_deref()),
                optional(comment.created_at.as_deref())
            ));
        } else {
            output
                .stdout
                .push_str(&nonconforming(&comment.path, &comment.problems));
        }
    }
    index_note(&mut output, data.index.state.as_str());
    incomplete(&mut output, data.complete);
    output
}

fn reason_line(code: &str, ids: &[String], complete: bool) -> String {
    format!(
        "{code}: {}{}\n",
        ids.iter()
            .map(|id| field(id))
            .collect::<Vec<_>>()
            .join(", "),
        if complete { "" } else { " (incomplete)" },
    )
}

pub fn ticket_blocked(data: &ItemListDto) -> Human {
    let mut output = text(String::new());
    for item in &data.items {
        output.stdout.push_str(&item_line(item));
        if let Some(readiness) = &item.readiness {
            for reason in &readiness.reasons {
                output.stdout.push_str("  ");
                output.stdout.push_str(&reason_line(
                    reason.code.as_str(),
                    &reason.ids,
                    reason.complete,
                ));
            }
        }
    }
    index_note(&mut output, data.index.state.as_str());
    incomplete(&mut output, data.complete);
    output
}

fn dependency_line(node: &DependencyTreeNodeDto) -> String {
    let mut line = String::new();
    for _ in 0..node.depth {
        line.push_str("  ");
    }
    line.push_str(&format!(
        "{}  {}  {}  {}{}{}\n",
        field(&node.id),
        optional(node.slug.as_deref()),
        node.state.as_str(),
        optional(node.title.as_deref()),
        if node.repeated { " (repeated)" } else { "" },
        if node.truncated { " (truncated)" } else { "" },
    ));
    line
}

pub fn ticket_deps(data: &DependencyTreeDto) -> Human {
    let mut output = text(dependency_line(&data.ticket));
    for (heading, nodes) in [
        ("Dependencies:", &data.dependencies),
        ("Dependents:", &data.dependents),
    ] {
        if !nodes.is_empty() {
            output.stdout.push_str(heading);
            output.stdout.push('\n');
            for node in nodes {
                output.stdout.push_str(&dependency_line(node));
            }
        }
    }
    index_note(&mut output, data.index.state.as_str());
    incomplete(&mut output, data.complete);
    output
}

pub fn ticket_cycles(data: &CycleListDto) -> Human {
    let mut output = text(String::new());
    for cycle in &data.items {
        output.stdout.push_str(&format!(
            "{}: {}\n",
            cycle.kind.as_str(),
            cycle
                .ids
                .iter()
                .map(|id| field(id))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    index_note(&mut output, data.index.state.as_str());
    incomplete(&mut output, data.complete);
    output
}

pub fn ticket_plan(data: &PlanDto) -> Human {
    let mut output = text(String::new());
    for batch in &data.batches {
        output.stdout.push_str(&format!("Batch {}:\n", batch.batch));
        for item in &batch.items {
            output.stdout.push_str("  ");
            output.stdout.push_str(&item_line(item));
        }
    }
    if !data.unplannable.is_empty() {
        output.stdout.push_str("Unplannable:\n");
        for item in &data.unplannable {
            output.stdout.push_str("  ");
            output.stdout.push_str(&item_line(&item.ticket));
            for reason in &item.reasons {
                output.stdout.push_str("    ");
                output.stdout.push_str(&reason_line(
                    reason.code.as_str(),
                    &reason.ids,
                    reason.complete,
                ));
            }
        }
    }
    index_note(&mut output, data.index.state.as_str());
    incomplete(&mut output, data.complete);
    output
}

#[cfg(test)]
#[path = "human_tests.rs"]
mod tests;
