#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    Available,
    Planned(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepoRule {
    Required,
    Refused,
}

#[derive(Debug, Clone, Copy)]
pub struct OptionSpec {
    pub name: &'static str,
    pub value: Option<&'static str>,
    pub required: bool,
    pub summary: &'static str,
}

#[derive(Debug)]
pub struct CommandSpec {
    pub resource: &'static str,
    pub verb: &'static str,
    pub summary: &'static str,
    pub availability: Availability,
    pub repo: Option<RepoRule>,
    pub options: &'static [OptionSpec],
}

pub const REPOSITORY_VALUES: &[&str] = &["required", "refused"];

const fn option(
    name: &'static str,
    value: Option<&'static str>,
    required: bool,
    summary: &'static str,
) -> OptionSpec {
    OptionSpec {
        name,
        value,
        required,
        summary,
    }
}

const REPO: OptionSpec = option(
    "--repo",
    Some("path"),
    true,
    "Repository path, relative to the invocation directory.",
);
const ID: OptionSpec = option("--id", Some("ULID"), true, "Canonical item ULID.");
const KEY_ID: OptionSpec = option("--key-id", Some("ULID"), true, "Canonical key ULID.");
const STATUS: OptionSpec = option(
    "--status",
    Some("text"),
    false,
    "Filter by exact status text.",
);
const TYPE: OptionSpec = option("--type", Some("text"), false, "Filter by exact type text.");
const PROJECT: OptionSpec = option(
    "--project",
    Some("text"),
    false,
    "Filter by exact project text.",
);
const SLUG: OptionSpec = option(
    "--slug",
    Some("code"),
    false,
    "Filter by short code, passed through unchanged.",
);
const SHOW: &[OptionSpec] = &[
    REPO,
    OptionSpec {
        required: false,
        ..ID
    },
    option(
        "--path",
        Some("path"),
        false,
        "Repository-relative path; use exactly one of --id or --path.",
    ),
    option(
        "--context",
        Some("path"),
        false,
        "Context worktree path; requires --path.",
    ),
];
const FILTER: &[OptionSpec] = &[REPO, STATUS, TYPE, PROJECT];
const TICKET_LIST: &[OptionSpec] = &[
    REPO,
    STATUS,
    TYPE,
    PROJECT,
    option(
        "--closure",
        Some("open|closed|all"),
        false,
        "Filter by closure state (default: all).",
    ),
    SLUG,
    option(
        "--ready",
        None,
        false,
        "Only ready tickets; conflicts with --blocked.",
    ),
    option(
        "--blocked",
        None,
        false,
        "Only blocked tickets; conflicts with --ready.",
    ),
];
const DEPS: &[OptionSpec] = &[
    REPO,
    ID,
    option(
        "--direction",
        Some("down|up|both"),
        false,
        "Dependency traversal direction (default: down).",
    ),
    option(
        "--depth",
        Some("n"),
        false,
        "Maximum traversal depth as an unsigned 32-bit integer.",
    ),
];

const fn available(
    resource: &'static str,
    verb: &'static str,
    summary: &'static str,
    repo: RepoRule,
    options: &'static [OptionSpec],
) -> CommandSpec {
    CommandSpec {
        resource,
        verb,
        summary,
        availability: Availability::Available,
        repo: Some(repo),
        options,
    }
}

const fn planned(
    resource: &'static str,
    verb: &'static str,
    summary: &'static str,
    cycle: &'static str,
) -> CommandSpec {
    CommandSpec {
        resource,
        verb,
        summary,
        availability: Availability::Planned(cycle),
        repo: None,
        options: &[],
    }
}

use RepoRule::{Refused, Required};

/// The approved command catalog, ordered by delivering Cycle.
pub const COMMANDS: &[CommandSpec] = &[
    available(
        "repo",
        "list",
        "List registered repositories.",
        Refused,
        &[],
    ),
    available(
        "repo",
        "inspect",
        "Inspect a repository, including an unregistered one.",
        Required,
        &[REPO],
    ),
    available(
        "repo",
        "identity",
        "Show repository identity.",
        Required,
        &[REPO],
    ),
    available(
        "remote",
        "list",
        "List redacted repository remotes.",
        Required,
        &[REPO],
    ),
    available("key", "list", "List registered keys.", Refused, &[]),
    available("key", "show", "Show key metadata.", Refused, &[KEY_ID]),
    available(
        "key",
        "public",
        "Print public key text.",
        Refused,
        &[KEY_ID],
    ),
    available("host", "list", "List stored host pins.", Refused, &[]),
    available(
        "host",
        "inspect",
        "Inspect a stored host pin.",
        Refused,
        &[option(
            "--authority",
            Some("host[:port]"),
            true,
            "Host or bracketed IPv6 address, with optional port (default: 22).",
        )],
    ),
    available("document", "list", "List documents.", Required, &[REPO]),
    available(
        "document",
        "show",
        "Show a document by ID or path.",
        Required,
        SHOW,
    ),
    available(
        "folder",
        "list",
        "List document folders.",
        Required,
        &[REPO],
    ),
    available(
        "ticket",
        "list",
        "List tickets, open and closed unless filtered.",
        Required,
        TICKET_LIST,
    ),
    available(
        "ticket",
        "show",
        "Show a ticket by ID or path.",
        Required,
        SHOW,
    ),
    available("ticket", "ready", "List ready tickets.", Required, FILTER),
    available(
        "ticket",
        "blocked",
        "List blocked tickets and their reasons.",
        Required,
        FILTER,
    ),
    available(
        "ticket",
        "deps",
        "Show ticket dependencies.",
        Required,
        DEPS,
    ),
    available(
        "ticket",
        "children",
        "List child tickets.",
        Required,
        &[REPO, ID],
    ),
    available(
        "ticket",
        "cycles",
        "List ticket relationship cycles.",
        Required,
        &[REPO],
    ),
    available(
        "ticket",
        "plan",
        "Show ticket execution batches.",
        Required,
        FILTER,
    ),
    available(
        "ticket",
        "critical-path",
        "Show the ticket critical path.",
        Required,
        &[REPO],
    ),
    available(
        "ticket",
        "find",
        "Find tickets by short code.",
        Required,
        &[
            REPO,
            OptionSpec {
                required: true,
                ..SLUG
            },
        ],
    ),
    available(
        "comment",
        "list",
        "List comments for an item.",
        Required,
        &[REPO, ID],
    ),
    available("index", "status", "Show index status.", Required, &[REPO]),
    available("poll", "status", "Show polling status.", Required, &[REPO]),
    available(
        "operation",
        "list",
        "List repository operations.",
        Required,
        &[REPO],
    ),
    available(
        "operation",
        "show",
        "Show a repository operation.",
        Required,
        &[
            REPO,
            option(
                "--operation-id",
                Some("ULID"),
                true,
                "Canonical operation ULID.",
            ),
        ],
    ),
    available("id", "new", "Generate a new item ID.", Refused, &[]),
    planned("repo", "create", "Create a repository.", "C2"),
    planned("repo", "enable", "Enable Manyhands in a repository.", "C2"),
    planned("repo", "remove", "Remove a repository registration.", "C2"),
    planned("repo", "identity-set", "Set repository identity.", "C2"),
    planned("remote", "add", "Add a repository remote.", "C2"),
    planned("remote", "remove", "Remove a repository remote.", "C2"),
    planned("remote", "select", "Select a repository remote.", "C2"),
    planned("key", "generate", "Generate a key.", "C2"),
    planned("key", "import", "Import a key.", "C2"),
    planned("key", "select", "Select a key for a repository.", "C2"),
    planned("key", "clear", "Clear a repository's selected key.", "C2"),
    planned("key", "remove", "Remove a key registration.", "C2"),
    planned("key", "delete", "Delete a key.", "C2"),
    planned("host", "approve", "Approve a host pin.", "C2"),
    planned("host", "replace", "Replace a host pin.", "C2"),
    planned("host", "observe", "Observe a host key.", "C2"),
    planned("request", "show", "Show a request.", "C2"),
    planned("request", "cancel", "Cancel a request.", "C2"),
    planned("folder", "create", "Create a document folder.", "C3"),
    planned("document", "create", "Create a document.", "C3"),
    planned("document", "save", "Save a document.", "C3"),
    planned("document", "move", "Move a document.", "C3"),
    planned("document", "repair", "Repair a document.", "C3"),
    planned("ticket", "create", "Create a ticket.", "C3"),
    planned("ticket", "save", "Save a ticket.", "C3"),
    planned("ticket", "repair", "Repair a ticket.", "C3"),
    planned("ticket", "slug-assign", "Assign a ticket short code.", "C3"),
    planned("index", "refresh", "Refresh the index.", "C3"),
    planned("index", "rebuild", "Rebuild the index.", "C3"),
    planned(
        "path",
        "observe",
        "Observe a path for authoring or repair.",
        "C3",
    ),
    planned("ticket", "check", "Check ticket relationships.", "C3"),
    planned("comment", "add", "Add a comment.", "C4"),
    planned("item", "sync", "Synchronize an item.", "C4"),
    planned("repo", "sync", "Synchronize a repository.", "C4"),
    planned("conflict", "show", "Show a conflict.", "C4"),
    planned("conflict", "resolve", "Resolve a conflict.", "C4"),
    planned("operation", "resume", "Resume an operation.", "C4"),
    planned("operation", "abandon", "Abandon an operation.", "C4"),
    planned("document", "promote", "Promote a document.", "C5"),
    planned("ticket", "close", "Close a ticket.", "C5"),
    planned("poll", "once", "Poll a repository once.", "C5"),
    planned("poll", "configure", "Configure polling.", "C5"),
    planned("poll", "pause", "Pause polling.", "C5"),
    planned("poll", "resume", "Resume polling.", "C5"),
];

pub fn find(resource: &str, verb: &str) -> Option<&'static CommandSpec> {
    COMMANDS
        .iter()
        .find(|command| command.resource == resource && command.verb == verb)
}

pub fn resource_commands(resource: &str) -> Vec<&'static CommandSpec> {
    COMMANDS
        .iter()
        .filter(|command| command.resource == resource)
        .collect()
}

#[cfg(test)]
#[path = "catalog_tests.rs"]
mod tests;
