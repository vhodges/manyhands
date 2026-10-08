# Wave 03 F1 execution ledger

Ticket: `01M4CC0VMQ7R15A7M9SPN3KB67`
Branch: `manyhands/ticket/01M4CC0VMQ7R15A7M9SPN3KB67`
Worktree: `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M4CC0VMQ7R15A7M9SPN3KB67`
Plan: [approved implementation plan](2026-10-07-wave-03-foundation-01-read-boundary-and-results-implementation.md)
Design: [approved design](2026-10-07-wave-03-foundation-01-read-boundary-and-results-design.md)
Cycle: [approved Cycle](../Cycles/wave-03-foundation-01-read-boundary-and-results.md)

## Authority and preflight — 2026-10-07

The product owner approved the Cycle, design and plan on 2026-10-07 and then
asked to start implementing F1 using **subagent-driven development**.
Implementation and local checkpoint commits on this branch are authorized.
Pull request, merge, ticket closure and worktree cleanup are not. Push was not
authorized until 2026-10-08, when the product owner authorized pushing this
ticket branch to origin (so a remote builder could fetch it); the branch has
been kept current there since. No force-push.

The plan's precondition is met: defect ticket `01M4CKWWRA1DHFPMWKPNK7CQ1G` (one
effective copy per item) was merged to main as `60b0324` and pushed.

Fresh fetch observed `origin/main` at
`60b0324f3993b32f783fcb98e0f6e24dd9f750dd`; local main matches it. The clean
ticket head rebased from `0362239f78e376f1577e1b1dd724648ec5153409` to
`6346caabfe6c6d469d2c6eba6618cc7f4648b8bf` without conflicts. Ancestry and
clean status verified. `AGENTS.md` is unchanged by the rebase. Main's untracked
`.superpowers/` and all other worktrees are untouched.

## Audit re-check against the rebased source

The design's read-surface audit cites lines at `ceb1be4`. Since then only the
defect fix changed `src/`: `discovery.rs` gained the scoped active-context
collector and `repository.rs` lost its exclusion set. Every helper the design
names still exists with the same visibility. Current locations:

| Helper | Now at |
| --- | --- |
| `open_registry_read_only` | `src/repository/discovery.rs:1200` |
| `migrate_registry` | `src/repository/discovery.rs:1209` |
| `observe_items` | `src/repository/discovery.rs:316` |
| `cache_read_guard` | `src/repository/coordination.rs:62` |
| `read_host_trust` | `src/repository/transport/trust.rs:106` |
| `bounded_public_key_contents` | `src/repository/keys/registry.rs:402` |
| `repository_snapshot` | `src/repository.rs:1165` |
| `inspect`, `list_remotes` | `src/repository.rs:2990`, `3003` |
| `persist_context` | `src/repository.rs:4973` |
| `validation_code_name` | `src/repository.rs:5070` |
| `read_repository_snapshot` | `src/repository.rs:5094` |
| `owned_file_bytes` | `src/repository.rs:5939` |
| `collect_canonical_sources` | `src/repository.rs:6813` |
| `canonical_repository_root` | `src/repository.rs:7218` |
| `resolve_identity` | `src/repository.rs:8010` |

Two design statements are now facts and need no further work in F1:

- The index holds one row per item ID. The design's "Effective Copy" section
  described the fix as proposed; it is merged.
- The defect review settled one detail the design did not state: primary
  attributes a file to an active item only by that item's ticket and comment
  directories, so an unparseable primary document remains a primary problem.
  This is consistent with the design's nonconforming-entry rule and is covered
  in Task 5.

No design change is required before coding.

## Implementation topology and ownership

All stages run sequentially in this worktree. Exactly one writer is active at a
time. Each task is implemented by a fresh subagent given a bounded brief, then
reviewed by a separate fresh, read-only subagent against the task's contract.
Code fixes go back to an implementer, not the controller. The controller owns
this ledger, ticket comments and checkpoint commits of bookkeeping.

| Lane | Contract and files while active | Gate and handoff |
| --- | --- | --- |
| Baseline | This ledger and a ticket comment only | Required Devenv gate at the rebased head |
| Task 1 | Result model and redaction: `Cargo.toml`, `Cargo.lock`, `src/lib.rs`, `src/results.rs`, `src/results_tests.rs` | Focused unit tests, commit, independent review |
| Task 2 | Read module, DTO base, contract harness: `src/repository.rs` wiring, `src/repository/read/{mod,dto}.rs`, `tests/support/{schema,golden}.rs`, `schemas/v1/`, `tests/read_contract.rs`, `tests/read_boundary.rs`, `.gitattributes` | Focused tests, commit, independent review |
| Task 3 | Resolution and repository reads: `read/{resolve,admin}.rs`, schemas, goldens | Focused tests, commit, independent review |
| Task 4 | Credential reads: `read/credentials.rs`, `keys/registry.rs`, `keys/mod.rs` visibility | Focused and key regression tests, commit, review |
| Task 5 | Item lists and complete reads: `read/items.rs`, migration and persistence of `closed_by`, `unknown_metadata`, `refreshed_at` | Focused and discovery regression tests, commit, review |
| Task 6 | Comment reads: `read/comments.rs` | Focused tests, commit, review |
| Task 7 | Status and operation reads: `read/status.rs` | Focused and recovery regression tests, commit, review |
| Task 8 | Relationship view and index edges: `canonical.rs`, `discovery.rs`, `repository.rs` persistence | Canonical and discovery tests, commit, review |
| Task 9 | Relationship queries: `read/graph.rs`, `tests/read_relationships.rs` | Unit and integration tests, commit, review |
| Task 10 | Workflow test list, full gate, characterization, whole-branch review | Required local gate, fresh whole-branch review |

## Task state

- Baseline: passed at `6346caa` (evidence below).
- Task 1: complete; review accepted with fixes, range `2824eff..ae757b0`.
- Task 2: complete; review accepted with fixes, range `c553744..df02e79`.
- Task 3: complete; review accepted with fixes (one blocker), range
  `a1223c0..9639595`.
- Task 4: complete; review accepted with fixes, range `9fbe74d..a04acb3`.
- Task 5: complete; two independent reviews accepted with fixes, range
  `dffa110..bcbe2e6`; full gate passed at `bcbe2e6`.
- Task 6: complete; review accepted with fixes, range `a65fb40..540a2a1`.
- Task 7: complete; review accepted with fixes, range `8f9499f..52714e0`.
- Task 8: complete; review accepted with fixes, range `fcbe353..de6f876`. The
  full gate at `de6f876` had one failing test that passes on every rerun (see
  the Task 8 evidence).
- Task 9: complete; review accepted with fixes, range `b1449cc..2873ee2`.
- Task 10: complete; whole-Cycle review accepted with fixes, range
  `5418203..8d05ab2`; full gate passed at `8d05ab2`. F1 is review-ready.

## Resume here — 2026-10-08

All ten tasks are complete. The current state, the open questions for the
product owner and the tickets to raise are under "Whole-Cycle review
rulings", "Open obligations" and "Handoff" at the end of this ledger. The
paragraph below is kept as the record of where the work stood before Task 10.

Tasks 5 to 8 are complete. The full gate last passed cleanly at `bcbe2e6`
(Task 5); at `de6f876` (Task 8) it passed except for one load-sensitive
existing test. Task 9 is complete with focused runs only. Next: Task 10
(workflow test list, characterization, full gate that must pass cleanly,
whole-branch independent review, handoff defect tickets).

All cargo runs are local through Devenv. A same-day trial of running cargo on
remote sprites was abandoned by the product owner: both sprites lost their
network-backed disk under build load. Nothing remote remains.

Carried to the Task 10 handoff, each to be raised as its own defect ticket:

- Nothing writes an `accessibility` other than `accessible`, so a deleted
  registered repository lists as accessible (from Task 3).
- Index versioning (product owner, 2026-10-08: no builds are in use yet, so
  not a concern for F1; a version stamp may also serve to version the schema).
  Two things to design together: a schema version, replacing the growing list
  of probes and letting a build refuse an index newer than it understands; and
  a content version on rows an older build rewrites. Without the second, an
  older build sharing the data directory can refresh the index, drop slugs,
  edges, `closed_by` and unknown metadata, and leave it reading `current`.
- `concurrent_corrupt_rebuilds_replace_the_cache_once` in
  `tests/discovery_rebuild.rs` depends on a second rebuild getting the lease
  within 250 ms of the first finishing; it failed once under full-suite load.
- Refresh persists the root, each active context and the disappeared-context
  cleanup in separate transactions, so the index can briefly, or after a
  `RetryRequired`, hold an item twice or not at all. Reads tolerate it; the
  write path should be made atomic.
- A committed, unchanged file in an item worktree is usually stored as
  `uncommitted`, because the save path does not update that worktree's Git
  index. Now visible as `change_source`.

## Task 5 rulings — 2026-10-08

Settled by the controller after two independent reviews; none changes the
approved Cycle contract.

- **Index state.** `never_refreshed` only when `refreshed_at` is null and the
  registration has no context row. A migrated index reports `stale` with
  `refreshed_at: null` (Cycle decision 6).
- **`refreshed_at`** is a time taken before the refresh or rebuild first
  observes, not its final commit. Staleness compares file modification time
  with it, so a change that preserves an old modification time is not
  detected and a file dated in the future always reads as stale.
- **An item the index holds more than once.** Candidates are tried in this
  order: the row in the item's own worktree (`.manyhands/worktrees/<id>`),
  then a non-active row, then, as a last resort, a copy in another item's
  worktree. Whenever more than one row exists, or the last resort is used, the
  index is reported `stale`. `show_item` moves on only when a candidate has
  nothing behind it; a real error from a candidate is returned.
- **Lists and `show_item` can differ during a refresh.** A list checks that
  the worktree directory exists; `show_item` opens the file. Documented on
  the list DTO.
- **`show_path`** accepts a canonical item path, or a path under `docs/` or
  `.manyhands/tickets/` that has a stored conformity problem in the named
  context. Every path must be plain relative (no NUL, backslash, `.`, `..`,
  empty component, or `.manyhands/worktrees`). A nonconforming entry whose
  path fails that check is still listed; it cannot be shown. A well-formed
  path that is neither is refused inside the session, with `index.refresh`
  attached when the index is not current.
- **`context.kind`** has a third value, `unverified`, including for a root the
  index has no context row for.
- **File reader.** Reads use `guarded_file`: an `openat` walk with
  `O_NOFOLLOW` on every component, the leaf opened non-blocking without
  becoming a controlling terminal, type and modification time taken from the
  opened descriptor. Missing, and a name too long to exist, are "not found";
  a link, a non-directory component, a socket or any non-regular file is "not
  a file" (`invalid_path` for `show_path`, so `docs/a.md/b.md` is
  `invalid_path`); anything else is `repository_inaccessible`.
- **Migration.** The three columns are added by a step that probes without a
  lock and, only when something is missing, takes an immediate transaction,
  probes again and adds what is missing. Two processes opening an old index
  both succeed. The design's paragraph describing a duplicate-column race is
  superseded by this.
- **Accepted as implemented:** `ItemDto.index`; nullable `changed_at` and
  `change_source`; the stored `unknown_metadata` wrapper with byte-ordered
  keys; the `index.refresh` recovery action with `{"root"}`; the observation
  token layout; the orderings; bad-path validation before the lock.
- **Provisional:** the readiness-reason DTOs and schemas under `schemas/v1`
  exist only to satisfy the closed-schema lint and may change in Task 9.
  `TicketFilter.slug` and `readiness` are accepted and ignored until Tasks 8
  and 9.

Accepted limits:

- The non-Unix file reader is check-then-use, weaker than the Unix one, and
  has never been compiled or executed. A native obligation.
- An item file is read whole into memory while the shared index lock is held;
  there is no size cap. This predates F1.
- An open that meets a write lease fails as `repository_inaccessible` instead
  of waiting.
- The tests added with the two rounds of review fixes were written with their
  fixes and have only been run passing.

## Task 6 rulings — 2026-10-08

- **Product-owner decision: reads list what the index holds.** The front ends
  list what is in the index (file contents aside) and the indexer is what
  picks up new files; a short delay is acceptable. So `list_comments` reads
  the comment files the index names and does not list the directory, and a
  comment added since the last refresh is not listed, and not reported, until
  the next one. The design's comment paragraph is amended to say so. Item
  lists already behave this way. A reviewer's suggestion to flag `stale` from
  the comment directory's modification time was declined on this basis.
- **File set.** The `discovered_comments` rows of the chosen item row, plus
  stored problem rows in the same context whose path is a plain file directly
  inside `.manyhands/comments/<item>/`. A comment row with any other path is
  `internal_error`.
- **What is revalidated.** The item file and those comment files, together.
  Problems only a whole-context validation finds (`duplicate_id`,
  `cross_item_parent`) are taken from the stored problem rows while the file
  is not newer than `refreshed_at`. Such a stored finding can be outdated by a
  change to another item's file until the next refresh.
- **Nonconforming entry:** null `id`, `parent_id`, `author`, `created_at` and
  `body`; `item_id` is the item it is filed under; `path` and problem codes
  set. Comments that parse but are `missing_parent`, `cross_item_parent`,
  `comment_cycle`, `duplicate_id` or `missing_comment_item` are entries of
  this kind. No read returns a malformed comment's source; `show_path` refuses
  comment paths.
- **`author`** is read from the file's `created_by` each time; there is no
  index column. An invalid value (not a string, empty, or containing NUL)
  gives a null author and an `invalid_field` problem; the comment keeps its
  ID and the key is not left in `unknown_metadata`.
- **`CommentDto.unknown_metadata`** exists, as for items. `context`, `index`
  and `complete` are on the list, once.
- **Failures.** An indexed comment that cannot be opened fails the list as
  `repository_inaccessible`. A path the index holds only as an unreadable
  source, and that still cannot be opened, is a `source_unreadable` entry. A
  link, non-regular file or non-UTF-8 file is a `source_unreadable` entry. A
  file that is gone is left out and the index is `stale`.
- **`complete`** is false when the index holds a source problem exactly at the
  item's comment directory or at `.manyhands/comments`.
- **`stale`** when any comment file or the item file is newer than
  `refreshed_at`, an indexed path is missing, a problem path is no longer a
  file, the valid comments differ from the stored rows, or the item is held in
  more than one row.
- **The item file takes part.** If it no longer parses, every comment is
  `missing_comment_item` and the index is `stale`; if it now holds another ID
  the read is `item_not_found`.
- `tests/read_comments.rs` is a new test target for the Task 10 workflow list.

Accepted limits: thread depth recurses, bounded only by the refresh's
1,024-entry cap on a comment directory; comment files are read whole under the
shared lock; `complete: false` is tested only for the symbolic-link causes.

## Task 7 rulings — 2026-10-08

- **Operation list membership** (controller ruling extending the plan's
  wording; the design is amended): every stored operation that
  `operation resume` could act on or that still has work outstanding, and
  nothing that accumulates without bound.
  - Local: state not `completed`.
  - Key-material: phase not `completed`, or a failure code.
  - Remote: the reservation holder (no `next_action`; the store does not say
    whether anything is still running it); a synchronization in phase
    `interrupted` or `failed` (`next_action: resume`, because the reservation
    code re-reserves both); an operation with `index_pending` (`resume`); an
    operation with `reconciliation_required`.
  - Never listed: an ended poll of any priority; an ended promote or close,
    which have no restart path.
  - No current caller leaves a synchronization `interrupted` or `failed`; one
    that dies keeps its active phase and is listed as the holder. Listing is
    by phase only, so a listed operation can still be refused when resumed.
- **`show_operation`** finds any stored operation, finished or not. One ID can
  be in two stores (a synchronization and its index hand-off); the list shows
  both, local first, and `show` prefers a listed operation, then local,
  remote, key-material.
- **Failure codes** are a separate closed registry, `OperationFailureCode`,
  not result codes. The design is amended. An unknown stored key-material
  code is `unknown_failure`.
- **Local-only synchronization.** A local refresh record whose target starts
  with `synchronization-local-v1/` is reported as `synchronize_primary` or
  `synchronize_context` with its item, by a strict parse; a malformed one is
  `internal_error`. The target is never published.
- **`OperationDto`** fields: `operation_id`, `family`, `scope`, `action`,
  `state`, `completed_step`, `next_action`, `item_id`, `key_id`, `worktree`,
  `updated_at`, `failure_code`. `worktree` (not `context`) is the stored
  absolute path for local operations. `state` and `completed_step` are
  strings: closed in code for remote and key-material, and for local a stored
  name shaped `[a-z][a-z0-9_]*` of at most 64 bytes, else `internal_error`.
- **`index_status`** runs inside the read session and maps an unavailable
  index to state `unavailable`; an exclusive lock is still `busy`. This
  supersedes the Task 2 sentence that it checks availability without a
  session. A process that resolves the repository after the index became
  unavailable gets `index_unavailable` from resolution and never sees the
  state; the facade Cycle decides whether to map it. `problems` lists every
  stored problem for the registration; `pending_operations` is every pending
  local operation; staleness is the lists' rule.
- **`polling_status`** reads the policy row, the current batch's time and the
  active operation only. `latest_observed_at` is that batch's stored time; no
  time is stored for `latest_outcome` and none is invented;
  `next_eligible_at` is always null.
- **Invalid stored rows** fail the whole read with `internal_error`. A SQLite
  failure keeps its class: `busy`, or `index_unavailable` with the rebuild
  action.
- `tests/read_status.rs` is a new test target for the Task 10 workflow list.

Accepted limits:

- Operations of a root with no registration (an interrupted registration
  removal, an enable before registration) cannot be reached through these
  reads.
- A successful `unavailable` status carries no recovery action.
- `index_status` loads every item row to count them, so one invalid item row
  fails it.
- Remote operations do not expose `index_pending`, `reconciliation_required`
  or a branch, and polling does not expose `history_unknown`; adding them
  later is a schema change.
- No test forces a busy or corrupt database inside the remote readers.
- Task 7's tests were written after the code and checked by sixteen
  mutations; none was seen failing before its fix.

## Task 8 rulings — 2026-10-08

- **Index.** `discovered_items.slug`; `item_edges` (item row, target ID, kind
  `deps` or `parent`, unique together); `item_problems` (item row, code,
  optional detail that must be an item ID); indexes on the slug and on
  `item_problems.item_id`. Added by the Task 5 probing step, which now also
  creates tables and indexes; any addition marks every registration for
  refresh.
- **Writes.** `persist_context` writes the slug; `persist_relationships`
  writes edges (parent first, then deps in file order) and item problems,
  inside the same transaction that replaces the context's items, for refresh
  and rebuild. Old rows go by cascade from the existing context delete;
  every writable connection has foreign keys on. No save path changed.
- **The fields stay in the file's unknown keys.** F1 only reads them. A save
  keeps each once; the read DTO's `unknown_metadata` never repeats them for a
  ticket, including on an index written before this task.
- **Validation.** An invalid value is left out and reported on the item; the
  ticket stays conforming. Codes: `invalid_slug`,
  `relationship_wrong_type`, `relationship_invalid_id`,
  `relationship_self_reference`, `duplicate_dependency`, and at read time
  `relationship_not_a_ticket` for a document or comment target. `deps` and
  `parent` share codes. A dangling target is stored and reads `unresolved`.
- **Controller decisions on points the reviewer marked for the product
  owner**, reported to the product owner the same day and not objected to: a
  null value is absent; an uppercase slug is accepted and lowercased; problem
  objects gain `target_id`; `parent` is `{id, state}`. The design is amended.
- **State** (`open`, `closed`, `unresolved`) comes from the index rows of the
  effective copies, so it can lag the files while the index reads `current`.
- **`TicketFilter.slug`** is applied now: whole code, case-insensitive, every
  match. `readiness` is still ignored until Task 9.
- **A bad stored relationship row** fails every read that loads items with
  `internal_error`, consistent with the Task 7 ruling.
- **Dropped by the product owner:** a content stamp for older builds; see
  "Resume here".

Accepted limits:

- Repository and index problem counts exclude relationship problems.
- A parent and a dependency naming the same non-ticket produce one problem.
- An index written before this task can still flag a ticket's metadata as
  not representable because of these keys, until its next refresh.
- An unquoted slug that YAML reads as a number (`1e-12345`) is invalid; F2
  must write slugs as strings.
- `discovered_items_slug` is not used by any query yet.

## Task 9 rulings — 2026-10-08

- **A closed dependency never blocks** (controller ruling for the RFC's
  definition of ready, against the implementer's first reading of the RFC's
  validation table). Readiness, reasons and plannability consider cycles
  among open tickets only. `ticket_cycles` still reports cycles over all
  tickets; every member carries the read-time problem `dependency_cycle`
  with the cycle's lowest ID as `target_id`.
- **Accepted as implemented**, and amended into the design: cycle reasons
  capped at 16 IDs with `complete`; flat tree lines carrying `slug` and
  `title`; nearest-occurrence expansion; a cycle as an ascending set;
  `parent_cycle` as a read-time problem; `item_not_found` for a document ID;
  `ticket_readiness` with no readiness filter returning every open ticket;
  the orderings; complete reads deciding readiness from the file.
- **`TicketFilter.readiness`** is applied by `list_tickets`; nonconforming
  entries still pass list filters and never appear in a query result.
- **`find_tickets_by_slug`**: whole code, case-insensitive, every match; no
  prefix or text search exists because the RFC defines none.
- `tests/read_relationships.rs` is a new test target for the Task 10 workflow
  list.

Accepted limits:

- `canonical::ticket_relationships` checks duplicates with a linear scan per
  entry, so parsing one ticket is quadratic in its dependency count. A
  20,000-dependency ticket reads end to end in 2.4 s.
- `show_path` on a non-effective copy reports that copy's own readiness with
  the index `current`.
- The RFC's definition of the critical path does not state that it is over
  plannable tickets; the design does. The RFC is the product owner's to amend.
- No test forces a duplicate stored edge; the unique constraint prevents it.

## Decisions and rulings

Task 1:

- **Redaction errs toward redacting.** A location is replaced by `[redacted]`
  whenever its form is doubtful. Accepted false positives include a non-SSH
  URL with `@` in its path (`https://host/@org/repo`), an scp-like location
  with `@` in its first path segment, hosts outside `[A-Za-z0-9._-]` or a
  bracketed address, and `git+ssh://` with `@` after the host. Cost if wrong:
  a harmless remote shows as `[redacted]`; loosen per case with a test.
- **Only the literal `ssh` scheme keeps a user name.** Every other scheme
  loses its whole user-info.
- **`file:///` locations are returned as written**, including a fragment.
- **Helpers return absent, not empty.** `relative_path_string` gives `None`
  for an empty, absolute, parent-traversing or non-UTF-8 path, so a caller
  must not read `None` as only "not UTF-8". `timestamp_string` gives `None`
  outside years 0000–9999.
- **`Envelope::failure` asserts in debug builds** that its code is not `ok`.
- **Additions beyond the design:** one enum per `effects` field,
  `ResultCode::ALL`, `REDACTED`, and `Default` for `Scope`.
- **Message text is fixed** and pinned by a test; it becomes part of the
  published contract with Task 2's golden fixtures.

Known and accepted: an SSH URL whose user position holds a token, or whose
user is percent-encoded `user:password`, is kept as written, because SSH user
names are configuration by the approved design.

Task 2:

- **Schema composition.** One envelope schema with `data` open, plus one
  schema per DTO that the harness checks separately. A consumer has no
  single-file schema per command. Cost if wrong: generate per-command schemas
  from these files.
- **Schemas are closed.** A lint requires `additionalProperties: false` and
  `required` equal to the property keys on every object, except the envelope's
  `data` and a recovery action's `arguments`. The checker accepts and ignores
  `$schema`, `$id`, `title` and `description`; every published schema carries
  `$schema` and a `title`.
- **Error mapping.** `RepositoryNotEnabled` maps to
  `repository_not_registered`. `SharedKeyRegistryUnavailable`, the key-material
  `RegistryUnavailable` and every `SshTransportErrorKind` map to
  `internal_error`. A SQLite failure maps the same way raw or wrapped: busy or
  locked to `busy`; cannot-open, corrupt or not-a-database to
  `index_unavailable`.
- **No blanket validation conversion.** Caller input uses
  `ReadError::invalid_id()` and `invalid_path()`; content problems become
  `ProblemDto`s.
- **Scope comes from the resolved target**, never from
  `RepositoryError.root`, which is the data directory.
- **Recovery action names are dotted**, as in the RFC's `operation.resume`:
  `index.rebuild`. Task 3 gives it the repository root as an argument.
- **The session is query-only and forbids attached databases.** A read cannot
  use temporary tables. rusqlite's `limits` feature is enabled for this; it is
  a feature of an existing dependency and `Cargo.lock` is unchanged. No
  `unsafe` is permitted under `src/repository/read/`.
- **`read_session` fails on a degraded index**, and takes the lock before
  checking availability. Task 7's index status read checks availability itself
  without a session.
- **One `RepositoryOperation::Read` variant** for all reads.
- **`ReadError::code` is private** behind `code()`; a `ReadError` cannot carry
  `ok`. Its kept source can hold backend text and the data directory, so a
  front end must not print error chains.
- **Golden fixtures.** Placeholders replace a whole value or a path prefix
  only; update mode refuses to run when `CI` is set; every fixture file must
  be a registered case.
- **Correction to the plan.** "Review Focus" item 1 cites a
  read-only-directory test. The design dropped that test, and a read-only data
  directory does fail a read. The lock tests are the proof.

Task 3:

- **A worktree's owner is verified.** A candidate owner is accepted only when
  it is not itself a linked worktree and its canonical common Git directory
  equals the worktree's. Otherwise the result is `not_repository`.
- **Resolution also requires** the opened repository's canonical working
  directory to equal the selected path, so `<root>/.git` and
  `.git/worktrees/<x>` are `not_repository_root`.
- **Failure classes.** Git "not found" is `not_repository`; any other open or
  discover failure is `repository_inaccessible`. A missing path, a path
  through a file or an invalid path is `invalid_path`; any other I/O failure
  is `repository_inaccessible`. A worktree of a bare repository is
  `bare_repository`.
- **Recovery actions.** `not_repository_root` carries `repo.inspect` with
  `{"root": <discovered root>}`. `index.rebuild` carries `{"root": <root>}`
  once a repository is resolved. The argument key is `root` everywhere.
  `repository_not_registered` carries none; `repo.enable` is a mutation and
  belongs to F2.
- **`repo inspect` resolves first.** A linked worktree is inspected as its
  owning repository: `root` and `head_branch` are the owner's, and
  `selected_path` echoes what the caller passed. A detached HEAD is
  `head_branch: null`. `identity_state` is `available` or `required`.
- **Exception to "busy without exception".** `repository_identity`,
  `list_remotes_redacted`, the Git part of `inspect_repository` and
  `new_item_id` read no index data and do not use the read session, so they
  are not `busy` under the exclusive lock. This departs from the design's
  wording and is accepted.
- **`selected_for_publication` comes from the working-tree configuration**, as
  `repo inspect` does. It can differ from the index's stored observation and
  includes uncommitted edits.
- **`repo list` shapes.** `configuration` is
  `{state, primary_branch, publication_remote, problems}`; `index` is
  `{state, refreshed_at}`, provisional until Task 5 fills `refreshed_at` and
  `never_refreshed`; `problem_count` counts every stored problem.
- **Enum strings.** Accessibility `accessible | inaccessible`; configuration
  `valid | invalid | missing`; index `current | stale | never_refreshed`;
  identity source `repository | global | xdg | system | program_data |
  application | none`.
- **A third test target**, `tests/read_repository.rs`, holds resolution and
  repository read behavior. Task 10 adds it to the workflow list with the
  others, and the plan's "two read test targets" now means three.
- **The `repository_inaccessible` message** is "The repository cannot be
  read.", since resolution can return it for an unregistered path.

Known gap, carried: nothing in the existing code writes an accessibility other
than `accessible`, so a registered repository whose folder was deleted lists
as accessible. The read reports what is stored and does not probe. Owner: the
refresh path; to be raised as its own defect ticket at Task 10 handoff.

Inherited, unchanged: a remote with a non-UTF-8 URL is skipped by the existing
remote enumeration while the list still says `complete: true`.

Task 4:

- **More change in `keys/` than the plan allowed, accepted.** Besides widening
  `bounded_public_key_contents`, the row reader and the fingerprint
  expressions were extracted for reuse (`stored_shared_key_registrations`,
  `StoredSharedKeysError`, `openssh_public_key`, `public_key_fingerprint`), all
  `pub(in super::super)`. The review confirmed existing callers behave
  identically. The alternative was duplicating the decoder in `read/`.
- **`trust.rs` visibility.** `valid_identity` and `valid_authority` are
  `pub(in super::super)`; three `#[doc(hidden)]` `_for_testing` hooks create
  pins and the marker through the real trust code. No existing body changed.
- **`key public` returns the canonical encoding** of the parsed key, without a
  line ending, never the file's bytes. It is `public_key_unavailable` when the
  file is missing, over 16 KiB, not one line of OpenSSH public key text, has a
  comment that is not printable ASCII, contains `PRIVATE KEY`, or exceeds 256
  bytes, or when the public path equals any registration's private path. A
  registration whose stored state is `available` can therefore still be
  unavailable here.
- **`matches_registration`** is true only when the computed fingerprint equals
  a stored one.
- **Key lists are in registration order**, as `list_shared_keys` is. Host pins
  order by host, then port.
- **Enum strings.** Ownership `imported | generated`; private source state
  `available | missing | unavailable`; public metadata state
  `not_provided | available | unavailable`.
- **Host reads.** `inspect_host` normalizes the caller's authority with the
  parser the trust code uses before storing. An authority no pin can have is
  `authority_not_found` without reading other rows. `reapproval_required` is
  application-wide: it is on the list and repeated on each pin.
- **Invalid stored data is `internal_error`** with no partial list: a key row
  with invalid metadata, a pin failing the trust checks, an invalid marker.
- **Scope is empty** for all five reads; they are application-global. The CLI
  RFC exempts `host list` from `--repo` but not `host inspect`; the facade
  Cycle (C1) decides what `--repo` means there.
- **The first read after a writer closes recreates empty `-shm` and `-wal`
  files** beside the index. That is permitted application-local bookkeeping.
  Snapshot tests assert those are the only names that may appear and that the
  index and marker are byte-identical.

Accepted limits:

- A public path that reaches a private key through a symbolic or hard link is
  not detected by the path comparison. The file is opened, read into a
  zeroized buffer and refused by the parser; nothing is returned.
- The parsed key's comment is copied into an ordinary string that cannot be
  zeroized.
- The open-detection test is Linux-only. Other platforms have portable checks
  that cannot detect an open whose result is discarded.
- On Windows a registered device or pipe path would be opened before being
  rejected as not a regular file. Not executed.

## Verification and review

Per-task evidence is recorded below as it is produced. Nothing below this line
is evidence until a task records it.

### Baseline — 2026-10-07, head `6346caa`

Through Devenv on Linux, each command exit 0:

- `cargo check --all-features --locked`
- `cargo fmt --check`
- `cargo clippy --all-targets --all-features --locked -- -D warnings`
- `cargo test --all-features --locked`: 622 passed, 0 failed across 15
  standard binaries; 15, 35, 31 and 103 SSH cases passed in the four
  custom-harness suites.
- `cargo run --locked --bin manyhands-cli`

The effective-copy precondition is covered by the nine tests the defect ticket
added to `tests/discovery_rebuild.rs`, which ran in this suite.

### Task 1 — result model and redaction, range `2824eff..ae757b0`

- `b658f69` implementation; `788378d` review fixes; `ae757b0` further
  redaction fix found by the implementer.
- Independent review of `b658f69`: accept with fixes. Contract confirmed
  against the CLI RFC and design. Three should-fix findings (two redaction
  leak classes, one test that could not fail) and five minor ones, all
  addressed in `788378d`.
- `ae757b0` was not independently re-reviewed; the controller reran its tests.
- `cargo test --locked --lib results`: 18 passed, 0 failed (controller rerun).
- `cargo fmt --check`, `cargo clippy --all-targets --all-features --locked --
  -D warnings`, `cargo check --all-features --locked`: pass (implementer, at
  `ae757b0`).
- Not run for this task: the full suite. Windows path behavior is reasoned,
  not executed.

### Task 2 — read module and contract harness, range `c553744..df02e79`

- `b243959` implementation; `ca3219f` review fixes; `df02e79` replaces an
  `unsafe` call with rusqlite's `limits` feature.
- Independent review of `b243959`: accept with fixes. Four should-fix findings
  and eight minor ones, all addressed in `ca3219f`.
- `ca3219f` and `df02e79` were not independently re-reviewed.
- Controller rerun at `df02e79`: `cargo test --locked --lib` 205 passed;
  `--test read_contract` 19 passed; `--test read_boundary` 12 passed. No
  `unsafe` under `src/repository/read/`.
- Implementer at `df02e79`: `cargo test --locked --doc` 9 passed;
  `cargo fmt --check`, clippy with warnings denied over all targets and
  features, and `cargo check --all-features --locked` pass.
- Not run for this task: the full suite.
- Tests were written alongside the code, not strictly first.

### Task 3 — resolution and repository reads, range `a1223c0..9639595`

- `d63ec8f` implementation; `81ead8f` review fixes; `9639595` message wording.
- Independent review of `d63ec8f`: accept with fixes. One blocker (wrong-owner
  resolution, reproduced by the reviewer), three should-fix and four minor
  findings. The blocker, the detached-HEAD failure and the failure
  classification are fixed in `81ead8f`, each with a test the implementer saw
  fail first.
- `81ead8f` and `9639595` were not independently re-reviewed.
- Controller rerun at `9639595`: `read_boundary` 13, `read_contract` 24,
  `read_repository` 20, `repository_enablement` 73, `local_authoring` 112
  passed, 0 failed.
- Implementer: `cargo test --locked --lib` 209 passed, `cargo fmt --check` and
  clippy pass at `9639595`; `cargo test --locked --doc` 9 passed and
  `cargo check --all-features --locked` pass at `81ead8f`.
- Not run for this task: the full suite.
- Not tested: permission-denied and ownership failures during resolution;
  the `system`, `program_data` and `application` identity sources beyond a
  unit test of the level mapping.
- The behavior tests for the first commit were written before the code but
  only seen failing to compile; the review-fix tests were seen failing against
  the wrong behavior.

### Task 4 — credential reads, range `9fbe74d..a04acb3`

- `ca7f292` implementation; `003c46f` review fixes; `a04acb3` comment bound.
- Independent review of `ca7f292`: accept with fixes, no blocker. Five
  should-fix and four minor findings, all addressed in `003c46f`.
- `003c46f` and `a04acb3` were not independently re-reviewed.
- Controller rerun at `a04acb3`: `read_credentials` 25, `read_contract` 30,
  `read_boundary` 13, `read_repository` 20, `shared_key_registry` 39,
  `key_material` 39, `key_storage` 2, `session_credentials` 11 passed,
  0 failed. No `unsafe` under `src/repository/read/`.
- Implementer: `cargo test --locked --lib` 209 passed, `cargo fmt --check` and
  clippy pass at `a04acb3`; `ssh_transport` 103 SSH cases (baseline 103),
  `cargo test --locked --doc` 9 passed, `cargo check --all-features --locked`
  pass at `003c46f`.
- Not run for this task: `ssh_fixture`, `remote_observation`,
  `remote_synchronization` and the full suite.
- The first commit's tests were written after the code and checked by nine
  mutations. The review-fix tests were seen failing first.

### Task 5 — item lists and complete reads, range `dffa110..bcbe2e6`

- `39e3036` index migration and persistence; `f4c1c63` reads, schemas,
  goldens and tests; `b081c3a` and `5ac0228` first review fixes and their
  formatting; `f7c3146` and `bcbe2e6` second review fixes.
- First independent review, of `dffa110..f4c1c63`: accept with fixes, no
  blocker. Three contract-level findings (a migrated index reporting
  `never_refreshed`; an item held twice failing the read; a listed
  nonconforming path that `show_path` refused) and five minor ones. It could
  not construct a read outside the repository. All addressed in `b081c3a`.
- Second independent review, of `4b37526..5ac0228`: accept with fixes, no
  blocker, no way past `guarded_file`. One major finding (a real error from
  the worktree copy hidden behind the primary copy), six minor ones and
  several notes. Addressed in `f7c3146` and `bcbe2e6`, except the limits
  recorded under "Task 5 rulings".
- `f7c3146` and `bcbe2e6` were not independently re-reviewed.
- Controller run of the full gate at `bcbe2e6`, through Devenv on Linux, each
  command exit 0: `cargo check --all-features --locked`; `cargo fmt --check`;
  `cargo clippy --all-targets --all-features --locked -- -D warnings`;
  `cargo test --all-features --locked --no-fail-fast` with 815 passed and 0
  failed in the standard harness (including lib 221, `read_items` 48,
  `read_contract` 36, `read_credentials` 25, `read_repository` 21,
  `read_boundary` 15, `discovery_rebuild` 67, `local_authoring` 112,
  `repository_enablement` 73, `recovery_foundation_gate` 50) and 15, 35, 31
  and 103 SSH cases passed in the four custom-harness suites;
  `cargo run --locked --bin manyhands-cli`. This is the first full-suite run
  since the baseline (622 passed there).
- Implementer at `bcbe2e6`: `read_items` with `--nocapture` printed no
  `SKIPPED` line, so the two permission tests ran their assertions.
- A partial run on a remote sprite at `5ac0228` is not evidence: three
  existing tests failed there for environment reasons (mode 000 does not bind
  for that user; a different Git exclude template) and pass locally.
- Write-path changes: `migrate_registry` is followed by the column step
  described in the rulings, which sets `refresh_required = 1` when it adds
  anything; `persist_context`'s item insert writes `closed_by` and
  `unknown_metadata`; the two statements that clear `refresh_required` also
  set `refreshed_at`. `canonical.rs` gains one public wrapper,
  `item_path_kind`.
- Existing assertions changed: the exact `repositories` column list in
  `tests/repository_enablement.rs`; a `refreshed_at` expectation in
  `tests/read_repository.rs`; in `tests/read_boundary.rs` the last item read
  is `../outside.md`, because a well-formed non-canonical path is now refused
  inside the session.
- Test-first, as the implementer reported: the index tests were seen failing
  in stages; the first read tests failed only against a skeleton and were then
  checked by eight mutations; four test groups were not mutation-checked; the
  review-fix tests were only run passing.

### Task 6 — comment reads, range `a65fb40..540a2a1`

- `958d7ea` implementation, with `effective_copy` extracted from `show_item`
  so both reads choose the same copy; `5b617dc` tests, golden, contract and
  boundary additions; `540a2a1` review fixes.
- No index, migration or write-path change.
- Independent review of `a65fb40..5b617dc`: accept with fixes, no blocker. One
  major finding (context-wide problems not reproduced, so a duplicate comment
  came back conforming and the index read `stale` indefinitely) and five minor
  ones. It found no way to read outside the item's comment directory and
  judged the `effective_copy` extraction behavior-identical. All addressed in
  `540a2a1` except the directory-time `stale` signal, declined by the
  product-owner decision above.
- `540a2a1` was not independently re-reviewed.
- Controller rerun at `540a2a1`, through Devenv on Linux: `read_comments` 19,
  `read_contract` 37, `read_boundary` 15, `read_items` 48,
  `discovery_rebuild` 67 passed, 0 failed; `cargo fmt --check` exit 0; no
  `unsafe` under `src/repository/read/`. The library tests ran in the same
  pass but the controller did not capture their result line.
- Implementer at `540a2a1`: `cargo test --locked --lib` 224 passed; clippy
  with warnings denied over all targets and features exit 0; `read_comments`
  with `--nocapture` printed no `SKIPPED` line. At `5b617dc`:
  `local_authoring` 112 passed and `cargo check --all-features --locked`
  exit 0.
- Not run for this task: the full suite.
- Test-first, as reported: the 13 first tests failed against a skeleton; six
  mutations were tried, five caught and one fixed with a new assertion; five
  review-fix tests were seen failing against the old code; three were only
  run passing.

### Native build run at `a65fb40` — 2026-10-08

One manual dispatch of the Build workflow on this branch, run
`37777664096`. It was dispatched on an instruction the product owner meant
for another session; they let it stand. No further dispatch is authorized
from this work without asking.

- Release build of both binaries with all features: passed on all five
  targets (Linux x86_64 and aarch64, Windows x86_64 and aarch64, macOS
  aarch64). This is the first compilation of the non-Unix file reader.
- Test step (library tests and the eight credential and transport targets):
  passed on both Linux targets; failed on macOS (39 library tests) and on
  both Windows targets (43).
- Every failing test is under `repository::remote::` (`reservation`, `state`,
  `sync`). None is under `repository::read::` or `results::`, so F1's unit
  tests pass on Windows and macOS. Most fail at one line,
  `reservation_tests.rs:18`, with `RepositoryNotRegistered`, as in the last
  run on main (`37475030123`, 2026-10-06, 25 failures per platform; main has
  gained tests in those modules since). F1 changes nothing under
  `src/repository/remote/`.
- Not evidence for: the read integration targets (`read_*`), which the
  workflow does not run yet (Task 10), and the non-Unix reader's behavior,
  which was compiled but not exercised.

### Task 7 — status and operation reads, range `8f9499f..52714e0`

- `11997f6` operation failure code registry; `617746a` the four reads;
  `a8c6b75` schemas, goldens and tests; `800e268` and `52714e0` review fixes.
- No index, migration or write-path change. Visibility only:
  `recovery::action_name`, `keys::generation::failure_code` and a test-only
  re-export, for the vocabulary tests; `read::items::effective_rows`. Two
  SELECT-only readers were added to `remote/state.rs`.
- Independent review of `8f9499f..a8c6b75`: accept with fixes, no blocker. It
  found no side effect and no leak. One major contract finding (resumable
  remote operations were not listed) and six minor ones; all addressed.
- `800e268` and `52714e0` were not independently re-reviewed.
- Controller rerun at `52714e0`, through Devenv on Linux: `cargo test --locked
  --lib` 236; `read_status` 23; `read_contract` 44; `read_boundary` 17;
  `read_items` 48; `read_comments` 19; `recovery_foundation_gate` 50;
  `remote_reservation` 9; `key_material` 39 passed, 0 failed;
  `cargo fmt --check` and clippy with warnings denied over all targets and
  features exit 0; no `unsafe` under `src/repository/read/`.
- With `--nocapture`, `recovery_foundation_gate` also prints a "0 passed; 1
  failed" result for `common_git_lease_child`. It is the captured output of a
  child process that an existing test starts; that test file is unchanged by
  F1 and the target reports 50 passed. The controller did not investigate
  further.
- Implementer at `52714e0`: `remote_synchronization` 35 SSH cases passed.
- Not run for this task: the full suite.
- Test-first: none. Nine mutations of the first commit and seven of the fixes
  were each caught by a test.

### Task 8 — relationship view and index edges, range `fcbe353..de6f876`

- `0356c01` canonical view; `eb16e41` problem codes; `1ef40b8` index schema
  and writes; `7a9ffe4` item reads; `a6cb893` schema test; `6374474` and
  `de6f876` review fixes.
- Independent review of `fcbe353..a6cb893`: accept with fixes, no blocker. It
  confirmed foreign keys are on for every writable connection, that edges and
  item problems are replaced in the items' transaction for refresh and
  rebuild, that no unique-constraint abort is reachable, and that the save
  path keeps the three keys. One design-level finding (older builds sharing
  the data directory), dropped by the product owner, and six minor ones,
  addressed.
- `6374474` and `de6f876` were not independently re-reviewed.
- Controller full gate at `de6f876`, through Devenv on Linux:
  `cargo check --all-features --locked`, `cargo fmt --check`, clippy with
  warnings denied over all targets and features, and
  `cargo run --locked --bin manyhands-cli` exit 0.
  `cargo test --all-features --locked --no-fail-fast` exit 101: 915 passed, 1
  failed in the standard harness (lib 241, `canonical_foundation` 43,
  `read_items` 61, `read_contract` 46, `read_status` 23, `read_comments` 19,
  `read_boundary` 17, `read_repository` 21, `read_credentials` 25,
  `local_authoring` 112, `repository_enablement` 73, `discovery_rebuild` 66
  of 67); 15, 35, 31 and 103 SSH cases passed.
- The failure: `concurrent_corrupt_rebuilds_replace_the_cache_once`; the
  second of two rebuilds returned `RepositoryBusy`. The lease wait is 250 ms
  and the target took 10.5 s in that run against about 6 s alone. Reruns at
  the same head: 8 of 8 alone, 6 of 6 whole-target runs, 40 of 40 run eight at
  a time. `tests/discovery_rebuild.rs` is unchanged by F1. Not measured:
  whether F1's added index work lengthens the first rebuild enough to matter.
- Implementer: one test seen failing before its fix (a comment as a target);
  the first commits were checked by three mutations applied together; most
  review-fix tests were only run passing.

### Task 9 — relationship queries, range `b1449cc..2873ee2`

- `c508f4d` graph, readiness and queries; `3fe4a12` schemas and goldens;
  `000bdc3` integration and boundary tests; `2873ee2` review fixes.
- No index, migration or write-path change.
- Independent review of `b1449cc..000bdc3`: algorithms sound; one major
  semantic finding (a closed dependency blocked through a cycle), three minor
  and several notes; all addressed. The reviewer worked the chain, diamond,
  mixed-closure, unresolved, cycle, parent-loop and duplicate-slug cases by
  hand and found the rest matching the RFC.
- `2873ee2` was not independently re-reviewed.
- Controller rerun at `2873ee2`, through Devenv on Linux: `cargo test --locked
  --lib` 265; `read_relationships` 15; `read_items` 61; `read_contract` 54;
  `read_boundary` 17; `read_status` 23; `read_comments` 19 passed, 0 failed;
  `cargo fmt --check` and clippy with warnings denied over all targets and
  features exit 0; no `unsafe` under `src/repository/read/`.
- Not run for this task: the full suite.
- Test-first, as reported: 18 graph unit tests, 4 item unit tests and 13
  integration tests failed against stubbed graph functions; the two
  closed-dependency tests failed against the first implementation. The eight
  new goldens, the boundary additions and several review-fix assertions were
  only run passing. Unit tests run 100,000-deep chains and rings and a
  100,000-wide fan and assert results.

## Whole-Cycle review rulings — 2026-10-08

Three independent reviewers read `6346caa..5418203`, each with one lens: the
read boundary, security and privacy; the published contract against the RFCs
and the Cycle; and everything F1 changed outside `src/repository/read/` plus
the fifteen fix commits that had not been re-read. None found a blocker. A
fourth reviewer then read the fixes, `9be3ac1..3b219a9`. The design carries
the resulting amendments in its last section.

Fixed (`9d5ea88..8d05ab2`):

- **Configuration read.** `inspect_repository` and `list_remotes_redacted`
  read `.manyhands/config.toml` through the guarded reader with a 64 KiB cap.
  Other callers are unchanged.
- **Unknown metadata** is stored to at most 64 nesting levels. Before this, a
  value nested about 126 deep was stored in a form that could not be read
  back, and every item read of the repository failed. A stored column that is
  not what a refresh writes remains `internal_error`.
- **Resolution.** A linked worktree resolves only when its owner lists it.
- **Redaction.** Remote-helper locations, locations with a control character,
  and both locations of a remote with a helper configured are redacted. A
  space alone is not. An IPv6 scp-like location is kept.
- **`complete`** is false on item lists and relationship reads when a
  directory was not fully read (see the design).
- **Recovery actions** are a closed, published registry.
- **Renames:** `OperationDto.owner` (was `scope`); `HostPinDto.fingerprint`
  (was `sha256`).
- **Removed:** problem code `path_not_utf8`, which no read could emit.
- **Evidence:** goldens for `show_path` and eight more failure envelopes; a
  cycle and shared short code put on the primary branch by two real merges; a
  consistent `index_status` golden; three portability fixes in tests.
- Smaller: order-preserving key removal; comment IDs looked up only for edge
  targets; `EOPNOTSUPP` on Apple targets; an explicit configuration-reader
  choice in inspection.

Accepted limits and deferrals:

- Five `_for_testing` hooks added by F1 are public and compiled into release
  builds, three of them writers of host trust. No read reaches them. Gating
  them belongs with the older hooks of the same pattern.
- A non-Markdown symbolic link under `docs` makes the document list
  permanently `complete: false`, because discovery stores the same problem
  for it as for an unread directory. A single unreadable file or worktree
  leaves `complete: true` and is not listed.
- The configuration size cap applies to reads only; the indexer and the
  write paths accept a larger file.
- `RecoveryAction`'s fields are public, so "registered keys or none" is
  enforced by construction and a debug assertion, not by the type.
- The worktree ownership check is linear in the owner's worktrees.
- `show_item`, `show_path`, `list_comments` and the relationship reads load
  every item row, edge and item problem of the registration under the shared
  lock.
- A read can wait up to 5 s inside SQLite before returning `busy`; the 250 ms
  bound covers the file lock only.
- Envelope `scope.branch` and `scope.worktree` are never set by a read.
- Names recorded as deliberate: `invalid_path` is both a result code and a
  problem code; `repository_inaccessible` and `repository_unavailable`
  coexist; `complete` means list completeness on lists and "not capped" on
  reasons; `index_status` flattens the index state.
- Deferred to later Cycles, none breaking: identity for a repository that is
  not registered (C2, D1); unreadable files as list entries (D1); a mapping
  from operation actions to commands (C4, C5); an observation for an absent
  path (F2); `operation.resume` in the recovery registry (F2).
- `8d05ab2` and the four commits before it were not independently reviewed.

Open with the product owner at handoff:

1. Whether the closed v1 schemas need a stated compatibility rule (the CLI
   RFC calls additive fields and new codes non-breaking; the schemas reject
   both).
2. Whether comment replies stay nested or become a flat list with depth.
3. Amendments to the Cycle document, the Wave document and three RFCs that
   the rulings made inaccurate, including naming who writes `created_by`.

### Task 10 — verify, review and handoff, range `5418203..8d05ab2`

- `141b164` `show_path` golden; `43c457b` characterization test; `9be3ac1`
  workflow test list; `9d5ea88..94ca77d` whole-Cycle review fixes; `3b219a9`
  redaction narrowing; `147aa07..8d05ab2` fixes from the review of the fixes.
- Workflow: the eight read targets are in the "Test headless credential and
  platform contracts" step. Triggers are unchanged and nothing was dispatched.
- Contract completeness: 37 schemas, all reachable; a golden for every read
  in scope; 47 registered cases, held equal to the fixture directory by a
  test. Four objects are open by registration: the envelope's `data`, the two
  `unknown_metadata` objects and a recovery action's `arguments`.
- Controller full gate at `8d05ab2`, through Devenv on Linux, each command
  exit 0:
  - `cargo check --all-features --locked`
  - `cargo fmt --check`
  - `cargo clippy --all-targets --all-features --locked -- -D warnings`
  - `cargo test --all-features --locked --no-fail-fast`: 998 passed, 0
    failed in the standard harness; 15, 35, 31 and 103 SSH cases passed.
  - `cargo run --locked --bin manyhands-cli`
  - `cargo test --locked --test read_boundary --test read_contract --test
    read_relationships --test read_repository --test read_credentials --test
    read_items --test read_comments --test read_status`, without the
    `desktop` feature: 263 passed, 0 failed.
- The same gate also passed at `3b219a9` (997 passed, 0 failed).
  `concurrent_corrupt_rebuilds_replace_the_cache_once` passed in both runs.
- Characterization (not a gate), debug build, one run, medians of 7:
  refresh 4,631 ms; `list_tickets` 16.3 ms; `show_item` 15.1 ms;
  `list_comments` 11.4 ms; `ticket_plan` 16.3 ms. Fixture: 800 tickets (80
  closed), 200 documents, 350 dependency edges, 90 parent edges, 300 comments
  on 100 tickets, two item worktrees. `tests/read_characterization.rs`,
  ignored by default.
- Native: not executed for the final head. One run at `a65fb40` is recorded
  above. From reading the tests, the eight targets compile on Windows and
  macOS. Expected to fail on Windows until the library side is addressed: a
  stored problem path with native separators
  (`only_a_conformity_problem_at_an_item_path_with_no_item_makes_an_entry`),
  and two error mappings of the non-Unix reader
  (`show_path_reads_only_canonical_item_paths`,
  `a_path_with_a_nul_is_invalid_and_a_name_too_long_to_exist_is_not_found`).
  Permission tests print a `SKIPPED` line and pass where permissions do not
  bind, and the line is not visible without `--nocapture`.

### Acceptance rows

| Cycle row | Evidence | Status |
| --- | --- | --- |
| Target resolution | `tests/read_repository.rs`: root, linked worktree and item worktree resolve to one registration; unregistered, subdirectory, bare, missing and fabricated-worktree cases | Met |
| Read services | `read_repository`, `read_credentials`, `read_items`, `read_comments`, `read_status`, `read_relationships`; ordering and `complete` asserted, including against a real over-cap directory | Met |
| Nonconforming content | `read_items`: listed with null IDs and codes; read by exact path, including a ticket in a badly named directory | Met; an entry whose path is not plain relative is listed but cannot be shown |
| Closure | `read_items`: `ticket_filters_match_exactly_and_closure_follows_lifecycle_metadata`, `a_status_that_says_closed_closes_nothing` | Met |
| Effective copy | `read_items`: `an_item_in_two_other_item_worktrees_is_listed_once_from_primary` and the candidate-order tests | Met |
| Stale and unavailable index | `read_items`: `empty_never_refreshed_stale_and_degraded_are_four_results_and_one_failure`; `index_status` reports `unavailable` as a state | Met |
| Relationships | `read_relationships` (17 tests): all eight queries across primary and two item worktrees; unresolved dependency; cycle and duplicate short code merged onto primary by two real merges | Met |
| No side effects | `read_boundary` (18 tests): byte-identical snapshots of canonical files, refs, worktrees, configuration, key files and the index around every read; transport uninitialized | Met |
| Redaction | Sentinels in every contract case except `id_new` and `repo_identity`, which have nothing planted; `results_tests` | Met |
| Published contract | `read_contract` (64 tests): closed-object lint, enumerations tied to the registries, byte-equal goldens | Met |
| Front-end independence | The eight read targets pass without the `desktop` feature | Met on Linux |
| Regression | Full gate at `8d05ab2` | Met |
| Decision 5, tests join the workflow | `9be3ac1` | Met; not executed natively |

### Open obligations

- **Native execution** on Windows and macOS for the eight read targets, and
  native path matching: the non-Unix file and configuration readers have been
  compiled once and never run; stored problem paths use native separators;
  Windows device paths; whether stored worktree paths and canonical paths
  agree in case and prefix.
- **Conflict inspection**: deferred to the first of C4 and D5.
- **Polling fields**: `next_eligible_at` is always null; no outcome time or
  history is stored.
- **Empty folders** are not listed; F2.
- **`created_by`** is read and nothing writes it; F2, not yet in the Wave
  document's F2 scope.
- **`operation.resume`** and any new result code enter the registries and
  schemas with F2.

### Handoff: defect and follow-up tickets to raise

Not created. Filing them changes main, which is the product owner's to
authorize.

1. Nothing writes an `accessibility` other than `accessible`, so a deleted
   registered repository lists as accessible.
2. Refresh persists the root, each context and the cleanup in separate
   transactions, so the index can hold an item twice or not at all.
3. A committed, unchanged file in an item worktree is stored as
   `uncommitted`.
4. Index versioning: a schema version and a content version (see "Resume
   here").
5. Discovery caps that dogfooding will reach: one 1,024 counter covers the
   primary comment directory listing and every comment file under it (this
   branch holds 135); `.manyhands/tickets` counts each ticket twice, and past
   1,024 entries no item worktree can be prepared (about 512 tickets).
6. The indexer reads `.manyhands/config.toml` with a plain read: a FIFO there
   hangs a refresh, and there is no size cap.
7. `concurrent_corrupt_rebuilds_replace_the_cache_once` has almost no
   headroom on its 250 ms lease wait.
8. Gate every `_for_testing` hook out of release builds.
9. `canonical::ticket_relationships` is quadratic in one ticket's dependency
   count; `discovered_items_slug` is unused.
10. Refresh at 1,000 items takes 4.6 s in a debug build; not investigated.

### Handoff state

F1 is review-ready at `8d05ab2` plus this record. The branch is pushed to
origin and kept current. Not authorized and not done: pull request, merge,
ticket closure, worktree cleanup, further workflow dispatch.

## After handoff: product-owner decisions — 2026-10-08

The product owner answered the three open questions and the ticket question.
This section supersedes "Open with the product owner" and the ticket list
above.

1. **Schema compatibility.** The v1 schemas stay closed, provided consumers
   tolerate unknown fields. The rule is written into the CLI RFC, the design
   and the Cycle document: a schema describes what the build that ships it
   produces; additions are made in place; consumers ignore unknown fields and
   tolerate unknown codes and enumeration values. Controller ruling within
   it: the envelope's outcome and effect values keep the RFC's earlier rule
   that any change to them is breaking, because the decision named fields.
2. **Comments are a flat list**, and the consumer builds the tree.
   `CommentDto` loses `replies` and gains `depth`; `f959106`.
3. **Amendments authorized** to the Cycle document, the Wave document and the
   CLI, relationships and index RFCs; `42605e2`. F2's scope now includes
   writing `created_by`.
4. **Tickets do not touch main.** Each lives on its own branch and worktree;
   the earlier statement in this ledger that filing changes main was wrong.
   The product owner keeps few tickets, for work that will happen soon, so
   six were raised, each one commit on a branch from main `60b0324`, none
   pushed:
   - `01M4EHGE2KDCBRZ9DMXBPP30R0` accessibility is never updated
   - `01M4EHGE4BXGPMCWWA1S1QR0JW` make an index refresh atomic
   - `01M4EHGE60PK1WQ8DM4FRJV9YF` committed files stored as uncommitted
   - `01M4EHGE7YFBPRHY20X178572K` version the index: schema and content
   - `01M4EHGE9TMR99VYZC184J9XEC` discovery entry caps
   - `01M4EHGEBHGMJ2B3BKH9NDW5JJ` the indexer's unguarded configuration read

   Not raised, kept here as notes: the 250 ms lease wait in
   `concurrent_corrupt_rebuilds_replace_the_cache_once`; `_for_testing` hooks
   in release builds; the quadratic `ticket_relationships` and the unused
   `discovered_items_slug`; refresh time at 1,000 items.

### Flat comment list, `f959106`

- Implementer: iterative flattening; the golden regenerated, not hand-edited;
  the test-side schema checker learned `minimum`, which no schema used
  before. A comment with a missing, cross-item or cyclic parent never reaches
  the tree, so the file's `parent_id` is always the tree position and no
  `parent_id` reporting changed.
- Independent read-only review: no blocker and no should-fix. It confirmed
  the parent claim against `canonical::validate_context` and walked the
  held-out, missing and unreadable-parent cases.
- Notes from the review, not acted on:
  - `canonical::build_comment_threads`, the comment cycle detector and
    `collect_indexed` still recurse to the depth of a reply chain. The
    reviewer estimates, without measuring, that a chain near the 1,024 cap
    could overflow a 2 MiB thread in a debug build. A refresh would meet it
    before a read does. Existing code, unchanged by F1.
  - `assert_threaded` does not assert that depth rises by at most one.
  - The schema checker accepts `minimum` on a schema with no `type`.
- Controller full gate at `f959106`, through Devenv on Linux, each command
  exit 0: check, format, clippy with warnings denied;
  `cargo test --all-features --locked --no-fail-fast` with 999 passed and 0
  failed, and 15, 35, 31 and 103 SSH cases passed; the CLI smoke run; the
  eight read targets without the `desktop` feature, 264 passed and 0 failed.
- The document amendments were written by a subagent and read by the
  controller, not independently reviewed. Left as written, by its report: the
  design's "Status Reads" and "Redaction" still carry `scope` and
  `path_not_utf8`, which its amendments section withdraws; the relationships
  RFC does not say how a critical-path tie is broken beyond "the
  deterministic ordering".

### Handoff state

F1 is review-ready at `f959106` plus the document commits. Not authorized and
not done: pull request, merge, ticket closure, worktree cleanup, further
workflow dispatch, pushing the six new ticket branches.
