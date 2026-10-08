---
title: "Wave 03 F1 Read Boundary And Result Model Design"
date: 2026-10-07
status: approved
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M4CJE3WSTCMV5HDTRKA2AKP0"
---

# Wave 03 F1 Read Boundary And Result Model Design

## Intent And Approval Boundary

This design implements the
[F1 Cycle](../Cycles/wave-03-foundation-01-read-boundary-and-results.md). It
adds a read-only service layer, a shared result model and JSON v1 DTOs to the
headless library. It was revised after an independent review and approved by
the product owner on 2026-10-07. The six product-owner decisions and the
review record are in the Cycle document. Each was decided as recommended, so
the passages below that describe what a different answer would change are
kept only as a record of the alternative.

All line references are to main at `ceb1be4` and come from source inspection.
No Rust command was run to produce this document.

## Read-Surface Audit At `ceb1be4`

This refreshes the read rows of the
[provisional API audit](../research/wave-03-api-audit.md) for Wave 03
entry-gate item 3. "Exists" means a public function returns the data today.

| Read | Today | F1 adds |
| --- | --- | --- |
| Registered repositories | None. No query lists `repositories` without a root. `enabled_at` and `accessibility` are stored and never exposed. | Enumeration and its DTO. |
| Path to repository | `canonical_repository_root` (`repository.rs:7238`, private) requires the exact working-directory root; a linked worktree root then fails `RepositoryNotRegistered`. | A resolver through the common Git directory. |
| Repository inspection | `inspect` (`repository.rs:2997`) reads Git and the configuration file; no lock, no write. `RemoteInfo` carries raw URLs. | DTO with redacted URLs. |
| Git identity | `IdentityInspection::{Available, Required}` only. `resolve_identity` (`:8030`, private) finds name and email and discards the level. | Name, email and source level. |
| Remotes | `list_remotes` (`:3010`), raw URLs. | Redacted DTO with the publication selection. |
| Key list | `list_shared_keys` (`keys/registry.rs:107`) takes the exclusive lock, opens read-write and migrates. | A shared-lock, read-only list. |
| One key | None; filter the list. | Lookup by `SharedKeyId`. |
| Public-key text | None. Only path and fingerprint are stored; `bounded_public_key_contents` (`registry.rs:402`, 16 KiB cap) is private. | Bounded text read. |
| Host pins | None public. `read_host_trust` (`transport/trust.rs:106`) is crate-internal; no list. | List and one-authority reads. |
| Item metadata | `repository_snapshot` (`:1172`): one whole-repository snapshot, shared lock, read-only connection. No filter, no single-item lookup, no body, no `closed_by`, no unknown metadata. | Filtered lists and complete item reads. |
| Effective copy | Each item worktree's whole checkout is recorded (`discovery.rs:270-298`, `303`); a primary row is dropped when any worktree holds the item (`repository.rs:4945-4960`, `5002-5005`); a duplicate ID across worktrees makes the snapshot read fail (`:5349-5356`). | A defined rule; see Effective Copy. |
| Item content | `canonical::parse_item` and `validate_context` parse supplied strings; they read no file. | Reading the effective copy from its context. |
| Comments | Thread IDs, paths and `created_at` inside the snapshot. No body, no author. | Ordered threads with body, and author where recorded. |
| Problems | `RepositorySnapshot.problems`, with `code` a free `String` in kebab-case or one of `branch`, `source`, `context`, `retry-required`. `guidance` can hold backend text, such as a YAML parser message (`canonical.rs:162-168`) or an I/O error (`discovery.rs:914-917`). | Stable snake_case codes and fixed guidance on DTOs. |
| Index status | `refresh_required` and the `IndexUnavailable` error. `IndexAvailability` is private; there is no last-refreshed time. | A status read and a refreshed-at time. |
| Polling status | `remote_snapshot().polling()` and `latest_outcome()`. No timestamps or next eligible time in the public API. | DTO; missing values null. |
| Operations | Three stores: `recovery_inspection` (pending local), `active_remote_operation` (first active remote row, opened read-write with an immediate transaction), `list_key_material_recovery` (exclusive lock). No show-by-ID. Only remote operations store created and updated times; local operations store one `observed_at` rewritten on each step; key-material operations store no time and no repository. | One read-only inventory and show-by-ID. |
| Conflicts | No type, table or read. | Nothing; see Cycle decision 1. |
| New ID | `canonical::ItemId::generate`. | DTO only. |
| Serialization | Nothing in `src/` derives `Serialize`. `serde_json` is not a direct dependency. `time` has no `serde` feature. | A separate DTO layer. |

Two facts shape the design. Every stored record lives in one SQLite file,
`manyhands.sqlite3`, opened per call. And opening `RepositoryService` already
creates the data directory, migrates the schema and creates the lock file
(`repository.rs:2957`).

## Module Layout

```text
src/lib.rs                       pub mod results;          (new)
src/results.rs                   envelope, outcome, codes, failure class,
                                 recovery actions, redaction, JSON helpers
src/repository.rs                mod read; pub use read::*  (two lines)
src/repository/read/mod.rs       ReadError, shared open/lock helpers
src/repository/read/dto.rs       every DTO type; the only Serialize derives
src/repository/read/resolve.rs   path resolution, repository enumeration
src/repository/read/admin.rs     inspection, identity, remotes
src/repository/read/credentials.rs  keys, public-key text, host pins
src/repository/read/items.rs     document and ticket lists and complete reads
src/repository/read/comments.rs  comment threads, authors
src/repository/read/status.rs    index status, polling status, operations
src/repository/read/graph.rs     relationship queries; pure, no I/O
src/canonical.rs                 relationship view over a parsed ticket
src/repository/discovery.rs      additive schema; observe and persist edges
```

`read` is a child of `repository`, so it can call that module's private
helpers. Read services are methods on the existing `RepositoryService`; there
is no second service object and no second way to open the index.

| Helper | Defined | Reachable from `read` |
| --- | --- | --- |
| `open_registry_read_only` | `discovery.rs:1082`, `pub(super)` | Yes. |
| `cache_read_guard` | `coordination.rs:62`, `pub(super)` | Yes. |
| `resolve_identity` | `repository.rs:8030`, private | Yes. |
| `owned_file_bytes` | `repository.rs:5959`, private | Yes. Used for item source. `read_owned_item` is not used: it returns a parsed item and reports a parse failure as a missing target, so it cannot supply source or the nonconforming form. |
| `collect_canonical_sources` | `repository.rs:6833`, private | Yes. |
| `bounded_public_key_contents` | `keys/registry.rs:402`, private in a private module | No. Widen to `pub(in crate::repository)` and re-export from `keys`. |
| `read_host_trust` | `transport/trust.rs:106` | Reachable but not used: it takes its own lock and connection and reports a busy lock as an unavailable registry. Host reads query `ssh_host_pins` through the shared read session instead. |

`results` sits at the crate root because F2's mutation adapters use the same
envelope and code registry and are not reads.

`repository.rs` itself changes by a module declaration, a re-export and one
new `RepositoryOperation` variant per read family. The one visibility change
is the public-key reader above.

## Result Model

`src/results.rs` defines the contract types. They serialize; domain types do
not.

```rust
pub const SCHEMA_VERSION: u32 = 1;

pub struct Envelope<T> {
    pub schema_version: u32,
    pub command: String,
    pub request_id: Option<String>,
    pub operation_id: Option<String>,
    pub outcome: Outcome,
    pub code: ResultCode,
    pub message: String,
    pub scope: Scope,
    pub effects: Effects,
    pub data: Option<T>,
    pub recovery: Vec<RecoveryAction>,
}

pub enum Outcome { Success, Noop, Partial, Blocked, Cancelled, Error }

pub struct Scope {
    pub repository: Option<String>, pub item_id: Option<String>,
    pub branch: Option<String>, pub worktree: Option<String>,
    pub remote: Option<String>,
}

pub struct Effects { /* the seven fields of the CLI RFC */ }
pub struct RecoveryAction {
    pub action: String, pub operation_id: Option<String>,
    pub arguments: serde_json::Map<String, serde_json::Value>,
}

pub enum FailureClass { Input, Blocked, Incomplete, Transient, Internal, Cancelled }
```

`Effects::not_requested()` is the only value a read produces.
`Envelope::read_success(command, scope, data)` and
`Envelope::read_failure(command, scope, &ReadError)` are the two constructors
this Cycle needs; F2 adds the mutation ones. The caller supplies `command`,
because a read service does not know which verb or view asked.

### Result codes

`ResultCode` is a closed Rust enum with an explicit `as_str` returning the
stable lower_snake_case string. It serializes through `as_str`, never through
a derive, so renaming a variant cannot change the contract. Each code has a
fixed English message and a `FailureClass`. Codes introduced here:

| Code | Class | Meaning |
| --- | --- | --- |
| `ok` | — | Success. |
| `invalid_path` | Input | The path cannot be used as a target. |
| `not_repository` | Input | No Git repository at the path. |
| `not_repository_root` | Input | Inside a repository but not its root or a linked worktree root. Recovery names the root. |
| `bare_repository` | Input | The repository has no working tree. |
| `repository_not_registered` | Blocked | The repository is not enabled in Manyhands. |
| `repository_inaccessible` | Blocked | The registered root cannot be read. |
| `invalid_id` | Input | Not a canonical ULID. |
| `item_not_found` | Input | No item with that ID in any scanned context. |
| `path_not_found` | Input | No canonical resource at that exact path. |
| `key_not_found` | Input | No key registration with that ID. |
| `public_key_unavailable` | Blocked | The registration has no readable public key. |
| `authority_not_found` | Input | No pin for that authority. |
| `operation_not_found` | Input | No operation with that ID. |
| `index_unavailable` | Blocked | The index is degraded; rebuild is the recovery. |
| `busy` | Transient | The shared lock was not obtained in time. |
| `internal_error` | Internal | Anything else. The message is fixed; nothing from the backend is included. |

A failure's outcome follows its class: `blocked` for Blocked, `error` for
Input, Transient and Internal. No read produces `partial`, `noop` or
`cancelled`.

A stale or never-refreshed index is not a failure. Lists succeed and carry
`"index": {"state": "stale"}` or `"never_refreshed"` in their data, so a
caller always gets the rows the index has.

### `ReadError`

```rust
pub struct ReadError {
    pub code: ResultCode,
    pub scope: Scope,
    pub recovery: Vec<RecoveryAction>,
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}
```

`Display` prints the code's fixed message. `source` is kept for in-process
logging by a front end and is never serialized. Conversions from
`RepositoryError`, `KeyMaterialError`, `SshTransportErrorKind` and
`canonical::ValidationProblem` map by kind through one `match` per enum with
no wildcard arm, so a kind added later is a compile error until it is mapped.
Kinds a read cannot produce map to `internal_error` explicitly.
`RepositoryError`'s `Display` passes backend text through, so no conversion
may call it.

### Redaction

`results::redact_url(&str) -> String`:

- `scheme://user:password@host/path?query#fragment` loses the password, query
  and fragment. For `http` and `https` the whole user-info is dropped. For
  `ssh` the user name is kept, since `git@` is not a secret.
- The scp-like form `user@host:path` is kept as written.
- A local path is returned unchanged.
- Anything unparseable becomes the literal `[redacted]`.

Paths in DTOs are data the caller asked for and are not redacted. A path that
is not valid UTF-8 serializes as null with problem code `path_not_utf8`; it is
never lossily converted.

Problems use a second closed registry, `ProblemCode`, with a fixed guidance
sentence per code. Stored codes map to it: each canonical validation code to
its snake_case form, and `source`, `context`, `branch` and `retry-required`
to `source_unreadable`, `context_problem`, `branch_problem` and
`retry_required`. An unrecognized stored code becomes `unknown_problem`. The
stored `guidance` column is never serialized.

### JSON conventions

IDs, object IDs and timestamps are strings. Timestamps are RFC 3339 UTC with
second precision, written through `time`'s existing formatting feature.
Repository-relative paths use forward slashes on every platform. Absolute
paths are written as the platform reports them. Every field in a DTO is always
present; an absent value is `null`, never an omitted key. Enumerations are
lower_snake_case strings.

## Target Resolution

```rust
pub struct ResolvedRepository { /* private: registration id, root */ }
impl RepositoryService {
    pub fn resolve_repository(&self, path: &Path) -> Result<ResolvedRepository, ReadError>;
    pub fn list_repositories(&self) -> Result<RepositoryListDto, ReadError>;
}
```

`resolve_repository`:

1. Canonicalize `path`. A missing path is `invalid_path`.
2. Open the Git repository at exactly that path with `Repository::open`,
   without upward discovery. If that fails, discover upward: finding a
   repository above it means `not_repository_root`, with the discovered root
   as a recovery argument; finding none means `not_repository`.
3. A bare repository is `bare_repository`.
4. If the repository is a linked worktree, take its common Git directory and
   the working directory of the repository that owns it. That is the root.
5. Match the root exactly against `repositories.root_path`, the canonicalized
   string stored at enablement. This is today's matching rule. Its behavior
   with Windows verbatim prefixes and on case-insensitive file systems is
   untested and is carried as a native obligation.

Every other read takes `&ResolvedRepository`, so resolution happens once per
call and cannot differ between two reads of one request.

`list_repositories` returns every registration ordered by root path, each with
`root`, `enabled_at`, `accessibility`, `configuration` state, `index` state and
a count of open problems. It reads only SQLite: an inaccessible root is
reported from the stored accessibility, not probed.

## Read Side Effects

Every read runs in one `read_session`:

1. take `cache_read_guard`, the shared lock, with its existing 250 ms bound;
2. open `open_registry_read_only`;
3. run inside a deferred transaction that is rolled back.

A lock not obtained in time is `busy` for every read without exception.

Three existing reads do not meet that today and get read-only siblings in
`read/`: key listing (exclusive lock, read-write, migrates), the active remote
operation (immediate transaction), and key-material recovery (exclusive lock).
Host pins are read through the session too. The existing functions are not
changed; mutation paths still use them.

Refresh also writes under the shared lock (`repository.rs:4646`, `4737`), so
a read concurrent with a refresh is isolated by SQLite's own transaction, not
by the file lock. A read sees the index wholly before or wholly after it.

The migration at service open uses the existing probe-then-alter pattern. Two
processes opening an old index at once race on the same `ALTER`; the loser's
statement fails. The migration treats "duplicate column" and "table exists"
as success, and the additive changes are safe for an older build that ignores
them.

File reads go through the existing private helpers. No read calls a function
that can reach `with_authenticated_remote*`, the only path to the network.
That path fails closed when the Git transport is uninitialized, and every read
test runs uninitialized.

## Effective Copy

An item's effective copy is defined as:

- the copy in the worktree whose branch identifies that item, when such an
  active context exists;
- otherwise the primary copy.

An item worktree's checkout contains every other item as of its branch point.
Those copies are not that worktree's concern and are never effective.

Today's index does not implement this. The fix, proposed as its own defect
ticket under Cycle decision 1, is in discovery and persistence:

- `observe_active_contexts` keeps, for each active context, only the
  branch-identified item and the comments whose `item_id` is that item.
- The primary context's exclusion set then holds exactly the items that have
  their own active context, so every other item keeps its primary row.
- Validation problems are kept for the same paths only: an active context
  reports problems for its identified item's files, and primary reports
  problems for everything else.

After the fix the index holds exactly one row per item ID, which is what
`read_repository_snapshot` already requires. This design depends on that
invariant and adds no deduplication of its own.

If the alternative in decision 1 is chosen, an active context also contributes
items whose ID is absent from primary, and the invariant needs a tie-break
when two worktrees both hold such an item.

## Item Reads

```rust
pub struct TicketFilter {
    pub status: Option<String>, pub ticket_type: Option<String>,
    pub project: Option<String>, pub closure: ClosureFilter, // Open | Closed | All
    pub slug: Option<String>, pub readiness: Option<ReadinessFilter>, // Ready | Blocked
}
impl RepositoryService {
    pub fn list_documents(&self, repo: &ResolvedRepository) -> Result<ItemListDto, ReadError>;
    pub fn list_tickets(&self, repo: &ResolvedRepository, filter: &TicketFilter) -> Result<ItemListDto, ReadError>;
    pub fn show_item(&self, repo: &ResolvedRepository, id: &ItemId) -> Result<ItemDto, ReadError>;
    pub fn show_path(&self, repo: &ResolvedRepository, context: Option<&Path>, path: &Path) -> Result<ItemDto, ReadError>;
}
```

`ClosureFilter` defaults to `All` in the library. The CLI and the desktop
choose their own defaults (`all` and `open`), as their RFCs say.

Lists are built from the index. Documents order by relative path, then ID.
Tickets order by content-change time descending, then ID. Filters are exact,
case-sensitive string matches, except `slug`, which is case-insensitive.

A nonconforming entry is built from a stored problem only when all of these
hold: its code is a schema-conformity code (`missing_front_matter`,
`malformed_front_matter`, `missing_field`, `invalid_field`,
`kind_path_mismatch`, `invalid_path`, `duplicate_id`); it has a path; the
path falls under `docs/` for the document list or `.manyhands/tickets/` for
the ticket list; and no listed item has that path in that context. The entry
has null ID and metadata, with `path`, `context` and `problems` set. Filters
never remove nonconforming entries. Problems without a path, and context or
branch problems, are reported by `index_status`, not in item lists.

Relationship problems are a separate thing. They are stored per item and
appear in that item's `problems` array. They never produce a nonconforming
entry.

List data is:

```json
{ "items": [ … ], "complete": true,  "index": { "state": "current|stale|never_refreshed", "refreshed_at": "…|null" } }
```

`show_item` finds the item's index row, reads the bytes at
`context + path` with `owned_file_bytes` and parses them. If the file has
changed since the index saw it, the read still returns what is on disk and
sets `index.state` to `stale`. If the file is gone, the result is
`item_not_found` with a refresh recovery. If it no longer parses, the result
is the nonconforming form with `source` filled, so a repair view has the bytes.

`ItemDto`:

| Field | Type | Notes |
| --- | --- | --- |
| `id` | string or null | Null for nonconforming. |
| `kind` | `document` or `ticket` | From the path when the ID is unknown. |
| `path` | string | Repository-relative. |
| `title`, `type`, `status`, `project`, `team` | string or null | `type` and `status` null for documents. |
| `closure` | `{ "state": "open|closed", "closed_at", "closed_by" }` or null | Null for documents. Lifecycle fields only. |
| `slug`, `parent` | string or null | Tickets. |
| `deps` | array | Each `{ "id", "state": "open|closed|unresolved" }`. |
| `readiness` | `{ "state": "ready|blocked|closed", "reasons": [ … ] }` or null | Tickets. |
| `unknown_metadata` | object | Front-matter keys Manyhands does not define. |
| `body`, `source` | string or null | `show` only; null in lists. |
| `observation` | string or null | `show` only. |
| `context` | `{ "kind": "primary|active", "branch", "worktree", "head_oid" }` | The effective copy. |
| `changed_at` | string | Content-change time. |
| `change_source` | `git_commit` or `uncommitted` | |
| `problems` | array | Each `{ "code", "path", "guidance" }`, with fixed guidance per code. |

Lists carry every field except `body`, `source` and `observation`, as the
CLI RFC requires. The index does not store `closed_by` or unknown metadata
today, so both are added to it (see Index) and lists do not read item files.

`unknown_metadata` converts YAML to JSON. A YAML value with no JSON form (a
non-string key, a tagged value) is replaced by null and reported with problem
code `metadata_not_representable`; `source` still carries it exactly.

The observation token is `v1:` followed by the hex BLAKE3 digest of the
context's branch name, the relative path and the source bytes, each
length-prefixed. It is opaque to callers.

## Comments

```rust
pub fn list_comments(&self, repo: &ResolvedRepository, item: &ItemId) -> Result<CommentListDto, ReadError>;
```

The read takes the item's effective context, reads the comment files the
index names under `.manyhands/comments/<item>/` (its comment rows and its
stored problems there), parses each, and orders them with the existing
`canonical::ordered_comment_threads`. It does not list the directory: by
product-owner decision of 2026-10-08, reads list what the index holds and the
indexer is what picks up new files, so a comment added since the last refresh
appears after the next one. Problems that only a whole-context validation can
find (a duplicate ID, a parent in another item) are taken from what the
refresh stored. `CommentDto` is `id`, `item_id`, `parent_id`, `author`,
`created_at`, `body`, `path`, `unknown_metadata`, `problems` and `replies`.
`CommentListDto` adds `complete`, `context` and `index`, once for the list.

`author` is the comment's `created_by` front-matter value when present, and
null otherwise. Under Cycle decision 4 that field joins the comment schema and
is written from F2 onward; F1 only reads it, from unknown metadata, exactly as
it reads the relationship fields. No Git history is walked. A comment that
fails to parse appears with a null ID, its path and its problem, at the end of
the root list.

If decision 4 is not accepted and Git-derived authorship is wanted instead, it
needs a full-history walk per item. libgit2 has no path-limited walk, and the
existing activity walk is first-parent and capped at 1,024 commits
(`discovery.rs:22`, `454-500`), which would credit merged comments to the
person who merged them.

## Administration And Credential Reads

```rust
pub fn inspect_repository(&self, repo_path: &Path) -> Result<RepositoryInspectionDto, ReadError>;
pub fn repository_identity(&self, repo: &ResolvedRepository) -> Result<IdentityDto, ReadError>;
pub fn list_remotes_redacted(&self, repo: &ResolvedRepository) -> Result<RemoteListDto, ReadError>;
pub fn list_keys(&self) -> Result<KeyListDto, ReadError>;
pub fn show_key(&self, id: SharedKeyId) -> Result<KeyDto, ReadError>;
pub fn public_key_text(&self, id: SharedKeyId) -> Result<PublicKeyDto, ReadError>;
pub fn list_host_pins(&self) -> Result<HostPinListDto, ReadError>;
pub fn inspect_host(&self, authority: &SshAuthority) -> Result<HostPinDto, ReadError>;
```

`inspect_repository` takes a path, not a resolved repository, because
inspecting a repository that is not yet enabled is its purpose. It reports
whether the path is registered. The underlying `inspect` requires an
available index (`repository.rs:2998`), so on a degraded index this read is
`index_unavailable` with a rebuild recovery.

`IdentityDto` is `name`, `email` and `source`, one of `repository`, `global`,
`xdg`, `system`, `program_data`, `application` or `none`: the six levels
`resolve_identity` already iterates, which it now returns with the identity. When identity is incomplete, `name` and `email` are null and
`source` is `none`. The effective initials and their source join this DTO in
F2, with the short-code work.

`RemoteDto` is `name`, `fetch_location`, `push_location` (both redacted),
`publication_eligible` and `selected_for_publication`.

`KeyDto` is the registration's non-secret fields: `id`, `label`, `ownership`,
`selected`, `fingerprint`, `private_source_state`, `public_metadata_state` and
the two paths. These are the stored states; listing does not open a key file.
`public_key_text` reads the public file through the existing 16 KiB bounded
reader and returns `{ "id", "public_key", "fingerprint",
"matches_registration" }`, or `public_key_unavailable`. The fingerprint is
computed from the text just read; `matches_registration` says whether it
equals the stored one, so a replaced file is visible. No read here touches a private key file, takes the
key-store lock, or can prompt.

`HostPinDto` is `host`, `port`, `algorithm`, `sha256` and
`reapproval_required`, the last from the existing marker file.

## Status Reads

```rust
pub fn index_status(&self, repo: &ResolvedRepository) -> Result<IndexStatusDto, ReadError>;
pub fn polling_status(&self, repo: &ResolvedRepository) -> Result<PollingStatusDto, ReadError>;
pub fn list_operations(&self, repo: &ResolvedRepository) -> Result<OperationListDto, ReadError>;
pub fn show_operation(&self, repo: &ResolvedRepository, id: OperationId) -> Result<OperationDto, ReadError>;
pub fn new_item_id(&self) -> NewIdDto;
```

`IndexStatusDto`: `state` (`current`, `stale`, `never_refreshed`,
`unavailable`), `refreshed_at`, counts of contexts, items and problems, the
stored problems themselves (code, path, worktree), and every pending local
operation. Unlike the list reads, this read succeeds when the index is
unavailable, because reporting that is its job; a process that cannot resolve
the repository because the index is unavailable gets that failure from
resolution instead and never reaches this read.

`PollingStatusDto`: `enabled`, `paused`, `interval_seconds`,
`backoff_seconds`, `recovery_suspended`, `latest_outcome`,
`latest_observed_at`, `active_operation_id`, and `next_eligible_at`, which is
always null in this Cycle.

`OperationDto` unifies the three stores: `operation_id`, `family` (`local`,
`remote`, `key_material`), `scope` (`repository` or `application`),
`action`, `state`, `completed_step`, `next_action`, `item_id`, `key_id`,
`worktree`, `updated_at` and `failure_code`.

- **Membership.** The list holds every stored operation that
  `operation resume` could act on or that still has work outstanding, and
  nothing that accumulates without bound: local operations not completed;
  key-material operations not completed or failed; the remote operation
  holding the reservation; a remote synchronization that is interrupted or
  failed; and a remote operation with index work or reconciliation
  outstanding. Polls that have ended are never listed. `show_operation` finds
  any stored operation by ID, finished or not.

- **Ordering.** By operation ID. An operation ID is a ULID, so this is
  creation order, which is the CLI RFC's "start time then ID". A legacy local
  row with no operation ID sorts last, by its stored row order.
- **Times.** `updated_at` is the stored value for remote operations, the
  `observed_at` of the latest step for local ones, and null for key-material
  operations, which store none.
- **Scope.** Key-material operations belong to the application, not a
  repository. They appear in every repository's list with
  `scope: "application"`.
- **Failure.** `failure_code` comes from the key-material failure code or the
  remote outcome category, mapped to a closed registry of operation failure
  codes kept beside the result codes (amended 2026-10-08: not to result
  codes, which are the envelope's vocabulary and carry a message and failure
  class); it is null for local operations. The `redacted_error` column exists in the schema and nothing
  writes it; it is not read.

It carries no body, URL or backend text.

## Ticket Relationships

### Reading the fields

`canonical.rs` gains a view, not new stored fields:

```rust
pub struct TicketRelationships {
    pub slug: Option<String>,
    pub parent: Option<ItemId>,
    pub deps: Vec<ItemId>,
    pub problems: Vec<RelationshipProblem>,
}
pub fn ticket_relationships(ticket: &Ticket) -> TicketRelationships;
pub fn is_valid_slug(value: &str) -> bool;
```

It reads `slug`, `parent` and `deps` from `Ticket.unknown` and validates them
by the RFC's rules. Valid values populate the view. An invalid value, a
self-reference or a duplicate entry is ignored for the graph and recorded as a
problem. Whether a target is a ticket cannot be known from one file, so that
rule is applied at read time: a `deps` or `parent` target that resolves to a
document is ignored and reported, and does not block.

The raw YAML stays in `unknown`, so `serialize_item` and the existing save
path keep those keys and their values as they keep any unknown key today.
They do not keep their formatting: the serializer re-emits all front matter
through `serde_yaml` on every save (`canonical.rs:605-665`), so a
hand-written flow list comes back as a block list. That is existing behavior
for every unknown key and is not changed here. The item DTO lists the three
keys under their own fields and omits them from `unknown_metadata`.

F2 makes them written fields. Until then, a ticket has them only if someone
edited the file by hand, which is how this project's own tickets will get them.

### Index

`migrate_registry` gains additive changes, in the existing
`CREATE TABLE IF NOT EXISTS` and column-probe style:

```sql
ALTER TABLE discovered_items ADD COLUMN slug TEXT;
ALTER TABLE discovered_items ADD COLUMN closed_by TEXT;
ALTER TABLE discovered_items ADD COLUMN unknown_metadata TEXT;  -- JSON object
ALTER TABLE repositories     ADD COLUMN refreshed_at INTEGER;
CREATE TABLE IF NOT EXISTS item_edges (
    id        INTEGER PRIMARY KEY,
    item_id   INTEGER NOT NULL REFERENCES discovered_items(id) ON DELETE CASCADE,
    target_id TEXT NOT NULL,
    kind      TEXT NOT NULL CHECK (kind IN ('deps', 'parent')),
    UNIQUE (item_id, target_id, kind)
);
CREATE TABLE IF NOT EXISTS item_problems (
    id      INTEGER PRIMARY KEY,
    item_id INTEGER NOT NULL REFERENCES discovered_items(id) ON DELETE CASCADE,
    code    TEXT NOT NULL,
    detail  TEXT            -- the offending ID, or NULL; never free text
);
CREATE INDEX IF NOT EXISTS discovered_items_slug ON discovered_items(slug);
```

When the migration adds any of these, it sets `refresh_required` on every
registration, so an index that predates edges reports itself stale instead of
reporting that no ticket has dependencies.

Discovery's `ObservedItem` carries the view, `closed_by` and the unknown
metadata. `persist_context` writes them, the edges and the relationship
problems with the item row, in the same transaction. Relationship problems go
to `item_problems`, never to `problems`, so they cannot be mistaken for a
nonconforming file. Refresh and rebuild set `refreshed_at`.

An edge belongs to an item row, and an item row belongs to a context, so an
edge's context needs no column of its own. Whether an edge's target resolves
is not stored: it is decided at read time against the items present, because
it changes when another ticket is fetched or merged. The relationships RFC's
index sentence says both are recorded; the Cycle's review record proposes
correcting that wording.

### Queries

`read/graph.rs` is pure. It takes the effective tickets and edges and returns
results; it does no I/O, so every rule is unit-tested without a repository.

```rust
pub fn ticket_readiness(&self, repo, filter: &TicketFilter) -> Result<ItemListDto, ReadError>; // ready or blocked
pub fn ticket_dependencies(&self, repo, id: &ItemId, direction: Direction, depth: Option<u32>) -> Result<DependencyTreeDto, ReadError>;
pub fn ticket_children(&self, repo, id: &ItemId) -> Result<ItemListDto, ReadError>;
pub fn ticket_cycles(&self, repo) -> Result<CycleListDto, ReadError>;
pub fn ticket_plan(&self, repo, filter: &TicketFilter) -> Result<PlanDto, ReadError>;
pub fn ticket_critical_path(&self, repo) -> Result<ItemListDto, ReadError>;
pub fn find_tickets_by_slug(&self, repo, slug: &str) -> Result<ItemListDto, ReadError>;
```

- **Readiness.** A closed ticket is `closed`. An open ticket is `ready` when
  every dependency resolves to a closed ticket. Otherwise it is `blocked`,
  with one reason per cause: `open_dependency` with the ID,
  `unresolved_dependency` with the ID, or `dependency_cycle` with the cycle.
- **Cycles.** Dependency cycles are the strongly connected components of size
  greater than one, plus self-loops, found with Tarjan's algorithm. Parent
  cycles are found by walking each ticket's parent chain. Each cycle is
  reported once, starting from its lowest ID.
- **Trees.** Depth-first from the ticket. A ticket already on the current
  path, or already expanded elsewhere in the tree, is emitted as
  `{ "id", "repeated": true }` and not expanded, so a cycle cannot recurse and
  a diamond is not duplicated.
- **Plan.** The graph is always built from every ticket. Filters select which
  tickets are returned, never which edges exist, so filtering by project
  cannot make a blocked ticket look ready. Kahn's algorithm runs over open
  tickets, counting only open dependencies. Batch *n* holds the tickets whose open dependencies are all in
  earlier batches. Tickets on a cycle, behind an unresolved dependency, or
  downstream of either go to `unplannable` with the reason.
- **Critical path.** The longest chain of open tickets by dependency, by
  dynamic programming in topological order over the plannable set. Ties break
  toward the lower ID at each step.
- **Find by slug.** Case-insensitive equality on the stored slug. Zero matches
  is success with an empty list.

All results order by the ticket list ordering, then ID, unless stated.

## Published Contract

```text
schemas/v1/envelope.schema.json
schemas/v1/<dto>.schema.json          one per DTO
tests/fixtures/read_v1/<case>.json    golden envelopes
```

Schemas are hand-written and use only `type` (including type arrays for
nullable), `properties`, `required`, `additionalProperties`, `items`, `enum`
and `$ref` to a sibling file. A checker in `tests/support` implements exactly
that subset. Each golden test builds a fixture repository with fixed IDs and
fixed commit times, calls the read, serializes the envelope with sorted keys
and two-space indentation, and requires byte equality with the stored file and
conformance to its schema. Absolute paths and object IDs in fixtures are
replaced by fixed placeholders before comparison. A `.gitattributes` entry
marks `schemas/` and `tests/fixtures/read_v1/` as `-text`, as the editor
fixtures already do, so line-ending conversion cannot break byte equality.

Setting `MANYHANDS_UPDATE_GOLDEN=1` rewrites the fixture files. It is a
developer aid; the checks never set it.

## Alternatives Rejected

- **Derive `Serialize` on the domain types.** It would make every rename a
  contract change and breaks the existing `compile_fail` guarantees that
  credential types cannot be serialized.
- **A separate read service with its own connection.** Two ways to open the
  index means two places that must agree on locking and migration.
- **Readiness stored in the index.** It would be stale the moment a dependency
  closed without the dependent being rescanned.
- **Promoting `deps`, `parent` and `slug` to canonical fields now.** It changes
  the serializer and every save path, which is F2's work and needs F2's
  canonical-form rule.
- **Generating schemas from the Rust types.** It adds a dependency and makes
  the schema follow the code, when the point is for the code to follow the
  schema.
- **A JSON Schema validator crate.** A large dependency tree for a keyword
  subset small enough to check in about a hundred lines.

## Risks

| Risk | Control |
| --- | --- |
| DTOs are fixed before any front end uses them. | Golden fixtures use the CLI RFC's verb names and examples. A mismatch found by C1 or D1 returns as a shared-library change; additive fields do not need a new schema version. |
| A read path quietly takes the exclusive lock or opens read-write. | Every read goes through the one `read_session`. A test holds the exclusive lock in a child process and asserts each read returns `busy` and never blocks. A read-only-directory test was considered and dropped: a read-only connection to a WAL database must still create its shared-memory file. |
| The effective-copy fix changes what existing consumers see. | It is made and reviewed as its own change, with a test reproducing the duplicate-ID failure first, and the existing discovery tests as regression. |
| `RepositoryError` text leaks through a conversion. | Conversions never call `Display` on it. Redaction tests plant sentinels in remote URLs, paths and injected backend errors and scan every serialized envelope. |
| The schema migration marks every repository stale. | Intended, and reported as `stale` with a refresh recovery. Lists still return the rows they have. |
| Lists read unknown metadata from the index, which could drift from the file. | It is written in the same transaction as the item row and rebuilt on every refresh; a complete read always uses the file. |
| The harness-less SSH test binaries compile `src/lib.rs` as a private module. | They already allow dead code for that module; the baseline task confirms new public items do not trip clippy there. |
| F1 is too large for one review. | The split point is clean: Tasks 1–7 are the read boundary, Tasks 8–9 the relationships. See Cycle decision 6. |

## Test And Platform Evidence

- Unit tests beside each `read/` file, in sibling `*_tests.rs` files as the
  `remote/` modules do, cover redaction, code mapping, resolution and every
  graph rule.
- `tests/read_boundary.rs` covers each read service against real repositories
  built with the existing `tests/support` fixtures, with
  `repository_and_worktree_snapshot` taken before and after.
- `tests/read_relationships.rs` covers the relationship queries across primary
  and active worktrees, including two branches that are each valid and form a
  cycle and a duplicate short code once both are present.
- `tests/read_contract.rs` holds the golden and schema tests and the redaction
  scans.
- All three use the default harness, need no feature and no display, and are
  added to the native workflow's explicit test list.
- None of the three initializes the Git transport.

Local evidence is Linux only. Windows and macOS execution is an open
obligation, not a result, until the workflow is run.
