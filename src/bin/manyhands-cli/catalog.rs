#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    /// Available, with the Cycle that delivered it.
    Available(&'static str),
    Planned(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputRule {
    Required,
    Refused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Read,
    /// A read that can ask to unlock a key: `host observe`.
    UnlockingRead,
    Mutation {
        confirmed: bool,
        input: InputRule,
    },
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
    pub kind: Kind,
}

impl CommandSpec {
    pub fn delivered_by(&self) -> &'static str {
        match self.availability {
            Availability::Available(cycle) | Availability::Planned(cycle) => cycle,
        }
    }

    pub fn prompts(&self) -> bool {
        matches!(self.kind, Kind::Mutation { .. } | Kind::UnlockingRead)
    }
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
pub const OPERATION_ID: OptionSpec = option(
    "--operation-id",
    Some("ULID"),
    true,
    "Canonical operation ULID.",
);
pub const INPUT: OptionSpec = option(
    "--input",
    Some("file|-"),
    false,
    "Read one JSON input object from a file or stdin (-).",
);
pub const REQUEST_ID: OptionSpec = option(
    "--request-id",
    Some("ULID"),
    false,
    "Canonical request ULID.",
);
pub const PREPARE: OptionSpec = option(
    "--prepare",
    None,
    false,
    "Prepare a confirmation preview without executing the mutation.",
);
pub const CONFIRM: OptionSpec = option(
    "--confirm",
    Some("ULID"),
    false,
    "Execute with a prepared confirmation ULID.",
);
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
        availability: Availability::Available("C1"),
        repo: Some(repo),
        options,
        kind: Kind::Read,
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
        kind: Kind::Read,
    }
}

use RepoRule::{Refused, Required};
const MUTATION_INPUT: &[OptionSpec] = &[
    REPO,
    OptionSpec {
        required: true,
        ..INPUT
    },
    REQUEST_ID,
    PREPARE,
    CONFIRM,
];

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
        &[REPO, OPERATION_ID],
    ),
    available("id", "new", "Generate a new item ID.", Refused, &[]),
    CommandSpec {
        resource: "repo",
        verb: "create",
        summary: "Create a repository.",
        availability: Availability::Available("C2"),
        repo: Some(Required),
        options: MUTATION_INPUT,
        kind: Kind::Mutation {
            confirmed: true,
            input: InputRule::Required,
        },
    },
    CommandSpec {
        resource: "repo",
        verb: "enable",
        summary: "Enable Manyhands in a repository.",
        availability: Availability::Available("C2"),
        repo: Some(Required),
        options: MUTATION_INPUT,
        kind: Kind::Mutation {
            confirmed: true,
            input: InputRule::Required,
        },
    },
    CommandSpec {
        resource: "repo",
        verb: "remove",
        summary: "Remove a repository registration.",
        availability: Availability::Available("C2"),
        repo: Some(Required),
        options: &[REPO, REQUEST_ID, PREPARE, CONFIRM],
        kind: Kind::Mutation {
            confirmed: true,
            input: InputRule::Refused,
        },
    },
    CommandSpec {
        resource: "repo",
        verb: "identity-set",
        summary: "Set repository identity.",
        availability: Availability::Available("C2"),
        repo: Some(Required),
        options: MUTATION_INPUT,
        kind: Kind::Mutation {
            confirmed: true,
            input: InputRule::Required,
        },
    },
    CommandSpec {
        resource: "remote",
        verb: "add",
        summary: "Add a repository remote.",
        availability: Availability::Available("C2"),
        repo: Some(Required),
        options: MUTATION_INPUT,
        kind: Kind::Mutation {
            confirmed: false,
            input: InputRule::Required,
        },
    },
    CommandSpec {
        resource: "remote",
        verb: "remove",
        summary: "Remove a repository remote.",
        availability: Availability::Available("C2"),
        repo: Some(Required),
        options: MUTATION_INPUT,
        kind: Kind::Mutation {
            confirmed: true,
            input: InputRule::Required,
        },
    },
    CommandSpec {
        resource: "remote",
        verb: "select",
        summary: "Select a repository remote.",
        availability: Availability::Available("C2"),
        repo: Some(Required),
        options: MUTATION_INPUT,
        kind: Kind::Mutation {
            confirmed: false,
            input: InputRule::Required,
        },
    },
    CommandSpec {
        resource: "key",
        verb: "generate",
        summary: "Generate a key.",
        availability: Availability::Available("C2"),
        repo: Some(Refused),
        options: &[
            OptionSpec {
                required: true,
                ..INPUT
            },
            REQUEST_ID,
            PREPARE,
            CONFIRM,
        ],
        kind: Kind::Mutation {
            confirmed: false,
            input: InputRule::Required,
        },
    },
    CommandSpec {
        resource: "key",
        verb: "import",
        summary: "Import a key.",
        availability: Availability::Available("C2"),
        repo: Some(Refused),
        options: &[
            OptionSpec {
                required: true,
                ..INPUT
            },
            REQUEST_ID,
            PREPARE,
            CONFIRM,
        ],
        kind: Kind::Mutation {
            confirmed: false,
            input: InputRule::Required,
        },
    },
    CommandSpec {
        resource: "key",
        verb: "select",
        summary: "Select a key for a repository.",
        availability: Availability::Available("C2"),
        repo: Some(Refused),
        options: &[KEY_ID, REQUEST_ID, PREPARE, CONFIRM],
        kind: Kind::Mutation {
            confirmed: false,
            input: InputRule::Refused,
        },
    },
    CommandSpec {
        resource: "key",
        verb: "clear",
        summary: "Clear a repository's selected key.",
        availability: Availability::Available("C2"),
        repo: Some(Refused),
        options: &[REQUEST_ID, PREPARE, CONFIRM],
        kind: Kind::Mutation {
            confirmed: false,
            input: InputRule::Refused,
        },
    },
    CommandSpec {
        resource: "key",
        verb: "remove",
        summary: "Remove a key registration.",
        availability: Availability::Available("C2"),
        repo: Some(Refused),
        options: &[KEY_ID, REQUEST_ID, PREPARE, CONFIRM],
        kind: Kind::Mutation {
            confirmed: false,
            input: InputRule::Refused,
        },
    },
    CommandSpec {
        resource: "key",
        verb: "delete",
        summary: "Delete a key.",
        availability: Availability::Available("C2"),
        repo: Some(Refused),
        options: &[KEY_ID, REQUEST_ID, PREPARE, CONFIRM],
        kind: Kind::Mutation {
            confirmed: true,
            input: InputRule::Refused,
        },
    },
    CommandSpec {
        resource: "host",
        verb: "approve",
        summary: "Approve a host pin.",
        availability: Availability::Available("C2"),
        repo: Some(Required),
        options: &[
            REPO,
            OptionSpec {
                required: true,
                ..INPUT
            },
            REQUEST_ID,
            PREPARE,
            CONFIRM,
        ],
        kind: Kind::Mutation {
            confirmed: true,
            input: InputRule::Required,
        },
    },
    CommandSpec {
        resource: "host",
        verb: "replace",
        summary: "Replace a host pin.",
        availability: Availability::Available("C2"),
        repo: Some(Required),
        options: &[
            REPO,
            OptionSpec {
                required: true,
                ..INPUT
            },
            REQUEST_ID,
            PREPARE,
            CONFIRM,
        ],
        kind: Kind::Mutation {
            confirmed: true,
            input: InputRule::Required,
        },
    },
    CommandSpec {
        resource: "host",
        verb: "observe",
        summary: "Observe a host key.",
        availability: Availability::Available("C2"),
        repo: Some(Required),
        options: &[
            REPO,
            option(
                "--authority",
                Some("host[:port]"),
                false,
                "Select a publication remote authority to observe.",
            ),
        ],
        kind: Kind::UnlockingRead,
    },
    CommandSpec {
        resource: "request",
        verb: "show",
        summary: "Show a request.",
        availability: Availability::Available("C2"),
        repo: Some(Refused),
        options: &[OptionSpec {
            required: true,
            ..REQUEST_ID
        }],
        kind: Kind::Read,
    },
    CommandSpec {
        resource: "request",
        verb: "cancel",
        summary: "Cancel a request.",
        availability: Availability::Available("C2"),
        repo: Some(Refused),
        options: &[OptionSpec {
            required: true,
            ..REQUEST_ID
        }],
        kind: Kind::Read,
    },
    CommandSpec {
        resource: "folder",
        verb: "create",
        summary: "Create a document folder.",
        availability: Availability::Available("C3"),
        repo: Some(Required),
        options: MUTATION_INPUT,
        kind: Kind::Mutation {
            confirmed: false,
            input: InputRule::Required,
        },
    },
    CommandSpec {
        resource: "document",
        verb: "create",
        summary: "Create a document.",
        availability: Availability::Available("C3"),
        repo: Some(Required),
        options: MUTATION_INPUT,
        kind: Kind::Mutation {
            confirmed: false,
            input: InputRule::Required,
        },
    },
    CommandSpec {
        resource: "document",
        verb: "save",
        summary: "Save a document.",
        availability: Availability::Available("C3"),
        repo: Some(Required),
        options: &[
            REPO,
            ID,
            OptionSpec {
                required: true,
                ..INPUT
            },
            REQUEST_ID,
            PREPARE,
            CONFIRM,
        ],
        kind: Kind::Mutation {
            confirmed: false,
            input: InputRule::Required,
        },
    },
    CommandSpec {
        resource: "document",
        verb: "move",
        summary: "Move a document.",
        availability: Availability::Available("C3"),
        repo: Some(Required),
        options: &[
            REPO,
            ID,
            OptionSpec {
                required: true,
                ..INPUT
            },
            REQUEST_ID,
            PREPARE,
            CONFIRM,
        ],
        kind: Kind::Mutation {
            confirmed: false,
            input: InputRule::Required,
        },
    },
    CommandSpec {
        resource: "document",
        verb: "repair",
        summary: "Repair a document.",
        availability: Availability::Available("C3"),
        repo: Some(RepoRule::Required),
        options: MUTATION_INPUT,
        kind: Kind::Mutation {
            confirmed: true,
            input: InputRule::Required,
        },
    },
    CommandSpec {
        resource: "ticket",
        verb: "create",
        summary: "Create a ticket.",
        availability: Availability::Available("C3"),
        repo: Some(Required),
        options: MUTATION_INPUT,
        kind: Kind::Mutation {
            confirmed: false,
            input: InputRule::Required,
        },
    },
    CommandSpec {
        resource: "ticket",
        verb: "save",
        summary: "Save a ticket.",
        availability: Availability::Available("C3"),
        repo: Some(Required),
        options: &[
            REPO,
            ID,
            OptionSpec {
                required: true,
                ..INPUT
            },
            REQUEST_ID,
            PREPARE,
            CONFIRM,
        ],
        kind: Kind::Mutation {
            confirmed: false,
            input: InputRule::Required,
        },
    },
    CommandSpec {
        resource: "ticket",
        verb: "repair",
        summary: "Repair a ticket.",
        availability: Availability::Available("C3"),
        repo: Some(RepoRule::Required),
        options: MUTATION_INPUT,
        kind: Kind::Mutation {
            confirmed: true,
            input: InputRule::Required,
        },
    },
    CommandSpec {
        resource: "ticket",
        verb: "slug-assign",
        summary: "Assign a ticket short code.",
        availability: Availability::Available("C3"),
        repo: Some(Required),
        options: &[
            REPO,
            ID,
            OptionSpec {
                required: true,
                ..INPUT
            },
            REQUEST_ID,
            PREPARE,
            CONFIRM,
        ],
        kind: Kind::Mutation {
            confirmed: false,
            input: InputRule::Required,
        },
    },
    CommandSpec {
        resource: "index",
        verb: "refresh",
        summary: "Refresh the index.",
        availability: Availability::Available("C3"),
        repo: Some(Required),
        options: &[REPO, REQUEST_ID, PREPARE, CONFIRM],
        kind: Kind::Mutation {
            confirmed: false,
            input: InputRule::Refused,
        },
    },
    CommandSpec {
        resource: "index",
        verb: "rebuild",
        summary: "Rebuild the index.",
        availability: Availability::Available("C3"),
        repo: Some(Required),
        options: &[REPO, REQUEST_ID, PREPARE, CONFIRM],
        kind: Kind::Mutation {
            confirmed: false,
            input: InputRule::Refused,
        },
    },
    CommandSpec {
        resource: "path",
        verb: "observe",
        summary: "Observe a path for authoring or repair.",
        availability: Availability::Available("C3"),
        repo: Some(Required),
        options: &[
            REPO,
            option(
                "--path",
                Some("path"),
                true,
                "Repository-relative path to observe.",
            ),
            option(
                "--context",
                Some("path"),
                false,
                "Context worktree path; the repository root when omitted.",
            ),
        ],
        kind: Kind::Read,
    },
    CommandSpec {
        resource: "ticket",
        verb: "check",
        summary: "Check ticket relationships.",
        availability: Availability::Available("C3"),
        repo: Some(Required),
        options: &[
            REPO,
            ID,
            option(
                "--deps",
                Some("ULID,..."),
                false,
                "Proposed dependencies, comma-separated; give once.",
            ),
            option("--parent", Some("ULID"), false, "Proposed parent."),
        ],
        kind: Kind::Read,
    },
    CommandSpec {
        resource: "comment",
        verb: "add",
        summary: "Add a comment.",
        availability: Availability::Available("C4"),
        repo: Some(Required),
        options: &[
            REPO,
            ID,
            OptionSpec {
                required: true,
                ..INPUT
            },
            REQUEST_ID,
            PREPARE,
            CONFIRM,
        ],
        kind: Kind::Mutation {
            confirmed: false,
            input: InputRule::Required,
        },
    },
    CommandSpec {
        resource: "item",
        verb: "sync",
        summary: "Synchronize an item.",
        availability: Availability::Available("C4"),
        repo: Some(Required),
        options: &[REPO, ID, REQUEST_ID, PREPARE, CONFIRM],
        kind: Kind::Mutation {
            confirmed: false,
            input: InputRule::Refused,
        },
    },
    CommandSpec {
        resource: "repo",
        verb: "sync",
        summary: "Synchronize a repository.",
        availability: Availability::Available("C4"),
        repo: Some(Required),
        options: &[REPO, REQUEST_ID, PREPARE, CONFIRM],
        kind: Kind::Mutation {
            confirmed: false,
            input: InputRule::Refused,
        },
    },
    CommandSpec {
        resource: "item",
        verb: "republish",
        summary: "Republish a deleted item branch.",
        availability: Availability::Available("C4"),
        repo: Some(Required),
        options: &[REPO, ID, REQUEST_ID, PREPARE, CONFIRM],
        kind: Kind::Mutation {
            confirmed: true,
            input: InputRule::Refused,
        },
    },
    CommandSpec {
        resource: "conflict",
        verb: "show",
        summary: "Show a conflict.",
        availability: Availability::Available("C4"),
        repo: Some(Required),
        options: &[REPO, OPERATION_ID],
        kind: Kind::Read,
    },
    CommandSpec {
        resource: "conflict",
        verb: "resolve",
        summary: "Resolve a conflict.",
        availability: Availability::Available("C4"),
        repo: Some(Required),
        options: &[
            REPO,
            OPERATION_ID,
            OptionSpec {
                required: true,
                ..INPUT
            },
            REQUEST_ID,
            PREPARE,
            CONFIRM,
        ],
        kind: Kind::Mutation {
            confirmed: false,
            input: InputRule::Required,
        },
    },
    CommandSpec {
        resource: "operation",
        verb: "resume",
        summary: "Resume an operation.",
        availability: Availability::Available("C4"),
        repo: Some(Required),
        options: &[REPO, OPERATION_ID, REQUEST_ID, PREPARE, CONFIRM],
        kind: Kind::Mutation {
            confirmed: false,
            input: InputRule::Refused,
        },
    },
    CommandSpec {
        resource: "operation",
        verb: "abandon",
        summary: "Abandon an operation.",
        availability: Availability::Available("C4"),
        repo: Some(Required),
        options: &[REPO, OPERATION_ID, REQUEST_ID, PREPARE, CONFIRM],
        kind: Kind::Mutation {
            confirmed: true,
            input: InputRule::Refused,
        },
    },
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
