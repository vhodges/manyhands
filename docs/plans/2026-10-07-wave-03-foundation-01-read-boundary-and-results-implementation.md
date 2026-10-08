---
title: "Wave 03 F1 Read Boundary And Result Model Implementation Plan"
date: 2026-10-07
status: approved
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M4CJE3Y2DHC4C75WKP6NEFXB"
---

# Read Boundary And Result Model Implementation Plan

> **For agentic workers:** Do not begin until the product owner explicitly
> authorizes implementation and the effective-copy defect ticket
> `01M4CKWWRA1DHFPMWKPNK7CQ1G` is merged to main. Then
> use `implementing-a-cycle` and the selected execution method task by task,
> re-read the design, repeat the ticket preflight, and record every checkpoint
> on ticket `01M4CC0VMQ7R15A7M9SPN3KB67`.

**Goal:** Give both front ends one headless, read-only way to get Manyhands
state as versioned JSON v1 DTOs, including the ticket relationship queries.

**Architecture:** Read services are methods on the existing
`RepositoryService`, in a new `repository::read` child module that reuses the
private index, lock and file helpers. A crate-level `results` module holds the
envelope, stable codes and redaction. DTOs are separate serializable types;
no domain type becomes serializable. Relationship fields are read as a view
over existing unknown metadata, stored as index edges, and queried by a pure
graph module.

**Tech stack:** Rust 2024, the locked `git2`, `rusqlite`, `serde`,
`serde_yaml`, `time`, `ulid` and `blake3`; `serde_json` added as a direct
dependency; Devenv.

**Spec:** [Cycle](../Cycles/wave-03-foundation-01-read-boundary-and-results.md)
and [design](2026-10-07-wave-03-foundation-01-read-boundary-and-results-design.md).

**Status:** Approved by the product owner on 2026-10-07, after independent
review and revision. This approval does not authorize Rust changes,
publication, merge, ticket closure or worktree cleanup; explicit
implementation authorization is still required.

## Global Constraints

- Work only in ticket `01M4CC0VMQ7R15A7M9SPN3KB67`'s worktree and branch.
  Preserve unrelated edits in main and in every other worktree.
- Before implementation, fetch `origin/main`, rebase this branch onto it,
  verify ancestry and cleanliness, and record the base and before/after heads.
- Run every Rust command as `devenv shell -- cargo …`.
- Keep everything in the headless library. Add no GPUI dependency, no CLI
  parsing, no human output and no exit numbers.
- Add no dependency other than `serde_json`. Commit `Cargo.lock` with it.
- No domain type derives `Serialize` or `Deserialize`. The existing
  `compile_fail` doctests for credential types must still pass.
- A read takes only the shared index lock and a read-only connection. It
  writes no canonical file, Git object, ref, worktree, configuration or key
  file, calls nothing that can reach the network, and never refreshes.
- Never call `Display` or `Debug` on `RepositoryError`, a `git2`, `rusqlite`
  or I/O error, or a URL, on any path that reaches a DTO or message.
- Do not change the behavior of an existing public function. Where a read
  needs a read-only form of an existing function, add a sibling.
- The only write-path changes are the additive index migration; persisting
  `closed_by`, unknown metadata, the short code, edges and relationship
  problems during refresh and rebuild; and setting `refreshed_at`.
  `serialize_item` and the save paths are not changed.
- The effective-copy fix is not part of this plan's tasks. The product owner
  decided it is its own defect ticket, `01M4CKWWRA1DHFPMWKPNK7CQ1G`, and it must be merged to
  main before the baseline checkpoint.
- Stored `guidance` text is never serialized. Every DTO problem takes its
  guidance from the fixed registry.
- No read test initializes the Git transport.
- Write the test for each behavior before the code that satisfies it.

## Review Focus

1. No read reaches a write. Task 2's lock and read-only-directory tests are
   the proof, and every later task adds its reads to them.
2. Nothing leaks. Task 1's redaction scan runs over every golden envelope
   added by later tasks, not only its own.
3. Relationship reads must not disturb the save path. Task 8 proves a ticket
   with `deps`, `parent` and `slug` keeps all three values through the
   existing save. Formatting is not preserved by the existing serializer and
   is not claimed.
4. Readiness follows lifecycle closure, never status text. Task 9 has a ticket
   whose status is `closed` and which is still open.
5. The migration must not report "no dependencies" for an index that predates
   edges. Task 8 proves it reports stale.
6. One row per item. Task 5 proves an item checked out in two other items'
   worktrees is listed once, from primary.

## File Map And Dependency Order

| Path | Task | Change |
| --- | --- | --- |
| `Cargo.toml`, `Cargo.lock` | 1 | Add `serde_json = "1"`. |
| `src/lib.rs` | 1 | `pub mod results;` |
| `src/results.rs`, `src/results_tests.rs` | 1 | New. |
| `src/repository.rs` | 2 | `mod read;`, re-export, new `RepositoryOperation` variants, helper visibility. |
| `src/repository/read/mod.rs`, `dto.rs` | 2 | New. |
| `tests/support/schema.rs`, `tests/support/golden.rs` | 2 | New. |
| `schemas/v1/*.schema.json`, `tests/fixtures/read_v1/*.json` | 2–9 | New, added per task. |
| `tests/read_contract.rs`, `tests/read_boundary.rs` | 2 | New; extended by later tasks. |
| `src/repository/read/resolve.rs`, `admin.rs` | 3 | New. |
| `src/repository/read/credentials.rs` | 4 | New. |
| `src/repository/keys/registry.rs`, `keys/mod.rs` | 4 | Widen `bounded_public_key_contents` to `pub(in crate::repository)` and re-export it. |
| `src/repository/read/items.rs` | 5 | New. |
| `src/repository/discovery.rs`, `src/repository.rs` | 5 | Migration for `closed_by`, `unknown_metadata`, `refreshed_at`; observe and persist them. |
| `.gitattributes` | 2 | `-text` for `schemas/` and `tests/fixtures/read_v1/`. |
| `src/repository/read/comments.rs` | 6 | New. |
| `src/repository/read/status.rs` | 7 | New. |
| `src/canonical.rs`, `tests/canonical_foundation.rs` | 8 | Relationship view. |
| `src/repository/discovery.rs`, `src/repository.rs` | 8 | Migration for `slug`, `item_edges`, `item_problems`; observe and persist them. |
| `src/repository/read/graph.rs`, `tests/read_relationships.rs` | 9 | New. |
| `.github/workflows/build.yml` | 10 | Add the three new test targets to the list. |

Tasks run in order. Tasks 3–7 each depend on 1 and 2 only and touch separate
files, so under subagent-driven execution they may be reviewed independently,
but they are implemented one at a time because each extends the shared test
files.

## Baseline And Dependency Checkpoint

Before Task 1:

```sh
git fetch origin main
git rebase origin/main
git merge-base --is-ancestor origin/main HEAD
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo test --all-features --locked
devenv shell -- cargo run --locked --bin manyhands-cli
```

Confirm the effective-copy fix is on main: a repository with two item
worktrees that share a third item yields a snapshot with that item once. If it
is not, stop; this plan cannot start.

Record the base, both heads and each command's result on the ticket. A failure
here is resolved or explicitly accepted by the product owner before Task 1; it
is not attributed to this Cycle later.

Re-check the design's read-surface audit against any change to
`src/repository*` or `src/canonical.rs` since `ceb1be4`. If a listed helper
moved or changed signature, update the design before coding.

## Task 1: Result Model And Redaction

**Files:** `Cargo.toml`, `Cargo.lock`, `src/lib.rs`, `src/results.rs`,
`src/results_tests.rs`.

**Contract:** The design's Result Model section: `Envelope<T>`, `Outcome`,
`Scope`, `Effects`, `RecoveryAction`, `FailureClass`, `ResultCode` with
`as_str`, `message` and `failure_class`, `redact_url`, and the timestamp,
object-ID and path string helpers.

**Tests first:**

- Each `ResultCode` has a unique lower_snake_case string, a non-empty fixed
  message and a class; the list matches the design's table exactly.
- `Envelope` serializes all eleven fields in every case, with `null` for
  absent values and `schema_version` 1.
- `Effects::not_requested()` serializes the seven fields of the CLI RFC.
- `redact_url` over a table: password removed; HTTP user-info removed; SSH
  user kept; scp-like form unchanged; query and fragment removed; local path
  unchanged; unparseable input becomes `[redacted]`.
- Timestamps serialize as RFC 3339 UTC to the second; a relative path with
  platform separators serializes with forward slashes; a non-UTF-8 path
  serializes as null.

**Verify:** `devenv shell -- cargo test --locked --lib results`.

**Checkpoint:** Comment on the ticket with the dependency change and results.

## Task 2: Read Module, DTO Base And Contract Harness

**Files:** `src/repository.rs`, `src/repository/read/mod.rs`, `dto.rs`,
`tests/support/schema.rs`, `tests/support/golden.rs`,
`schemas/v1/envelope.schema.json`, `tests/read_contract.rs`,
`tests/read_boundary.rs`.

**Contract:** `ReadError` and its conversions; a private `read_session` helper
that takes the shared lock, opens the index read-only and runs a closure in a
rolled-back transaction; the schema checker for the keyword subset in the
design; the golden comparison with placeholder substitution and
`MANYHANDS_UPDATE_GOLDEN`. `new_item_id` is implemented here as the first read,
to exercise the whole path.

**Tests first:**

- Every `RepositoryErrorKind`, `KeyMaterialErrorKind` and
  `SshTransportErrorKind` maps to a code through a `match` with no wildcard
  arm, so an unmapped kind does not compile. A table test pins the code for
  each kind a read can return.
- Every stored problem code maps to a `ProblemCode` with fixed guidance; an
  unrecognized stored code maps to `unknown_problem`.
- A `ReadError` built from a `RepositoryError` whose message and source
  contain a sentinel serializes without the sentinel.
- The schema checker accepts a conforming instance and rejects a missing
  required key, an extra key, a wrong type, a non-nullable null and a value
  outside an enum.
- The envelope golden for `id new` matches its file and its schema.
- With the exclusive index lock held by a child process, `read_session`
  returns `busy` within the existing bound and does not block.
- `read_session` succeeds, and the Git transport stays uninitialized, for the
  whole test binary.

**Verify:** `devenv shell -- cargo test --locked --test read_contract --test read_boundary`.

## Task 3: Target Resolution And Repository Reads

**Files:** `src/repository/read/resolve.rs`, `admin.rs`, schemas and goldens
for `repo list`, `repo inspect`, `repo identity`, `remote list`.

**Contract:** `resolve_repository`, `list_repositories`, `inspect_repository`,
`repository_identity`, `list_remotes_redacted`. `resolve_identity` returns the
level alongside the identity.

**Tests first:**

- A repository root, a linked worktree root and
  `.manyhands/worktrees/<id>` all resolve to the same registration.
- A subdirectory returns `not_repository_root` with the root as a recovery
  argument; an unregistered repository, a bare repository, a non-repository
  and a missing path each return their code.
- `list_repositories` returns every registration ordered by root, including
  one whose root has since been removed, without probing the filesystem.
- Identity reports `repository`, `global`, `xdg` and `none` sources from
  isolated Git configuration; `system`, `program_data` and `application` are
  covered by a unit test of the level mapping. An incomplete identity is
  `none` with null name and email.
- A remote whose URL embeds `user:SENTINEL@` serializes without `SENTINEL`;
  the publication selection is reported.
- Before-and-after `repository_and_worktree_snapshot` is equal for every read.

**Verify:** the two read test targets.

## Task 4: Credential Reads

**Files:** `src/repository/read/credentials.rs`, schemas and goldens for
`key list`, `key show`, `key public`, `host list`, `host inspect`.

**Contract:** `list_keys`, `show_key`, `public_key_text`, `list_host_pins`,
`inspect_host`.

**Tests first:**

- Listing imported and generated keys returns their stored states and the
  selection, under the shared lock; it succeeds while a child holds the shared
  lock and returns `busy` while one holds the exclusive lock.
- No read in this task opens a private key file. The test replaces the private
  file with an unreadable one and the reads still succeed.
- `public_key_text` returns the text and a fingerprint computed from it; a
  public file replaced after registration reports
  `matches_registration: false`; a missing, oversized or unreadable public
  file returns `public_key_unavailable`.
- Host pins list in host-then-port order; an unknown authority is
  `authority_not_found`; the reapproval marker is reflected; with the
  exclusive lock held, host reads return `busy`, not an unavailable registry.
- The serialized output contains no private-key sentinel from the existing
  fixture files.

**Verify:** the two read test targets, plus
`devenv shell -- cargo test --locked --test shared_key_registry --test key_material`
for regression.

## Task 5: Item Lists And Complete Reads

**Files:** `src/repository/read/items.rs`, schemas and goldens for
`document list`, `document show`, `ticket list`, `ticket show`.

**Contract:** `TicketFilter`, `list_documents`, `list_tickets`, `show_item`,
`show_path`, and `ItemDto` without the relationship fields populated (they are
present and null or empty until Task 8). The migration adds `closed_by`,
`unknown_metadata` and `refreshed_at`, sets refresh-required on every
registration when it adds them, and tolerates a concurrent migration.

**Tests first:**

- Documents order by path then ID; tickets by change time descending then ID.
- `status`, `type` and `project` filter exactly; `closure` filters by
  lifecycle metadata; a ticket with status text `closed` and no closure
  metadata is returned by `open` and not by `closed`.
- With two item worktrees that both contain a third item, the third item is
  listed once, from primary, and each worktree's own item once, from its
  worktree.
- List entries carry `closed_by` and `unknown_metadata` without the read
  opening any item file; the test makes the files unreadable after refresh.
- An index created before this task gains the columns, is marked
  refresh-required, and lists report `stale`.
- An item with an active worktree is read from the worktree copy and says so;
  after the worktree copy is edited without a refresh, `show_item` returns the
  edited source and reports the index stale.
- A marker-only document, a malformed ticket and a misplaced file appear in
  their lists with null IDs and codes, survive every filter, and are readable
  through `show_path` with their source. A problem with no path, and a
  context or branch problem, appear in no item list.
- A file whose YAML error text contains a sentinel yields a problem whose
  serialized guidance is the fixed sentence, without the sentinel.
- `unknown_metadata` carries unknown keys; a non-representable YAML value
  becomes null with its problem code while `source` keeps it exactly.
- The observation token changes when the source, path or branch changes and
  is otherwise stable.
- An empty repository, a never-refreshed one, a stale one and a degraded index
  give four distinct results, and only the degraded one is a failure.
- An item whose file was deleted after indexing returns `item_not_found` with
  a refresh recovery.

**Verify:** the two read test targets, plus
`--test discovery_rebuild --test local_authoring` for regression.

## Task 6: Comment Reads

**Files:** `src/repository/read/comments.rs`, schema and golden for
`comment list`.

**Contract:** `list_comments` and `CommentDto`.

**Tests first:**

- Roots and replies come back in the schema's order, to depth three, with
  bodies.
- `author` is the comment's `created_by` value when present and null when
  absent; `created_by` does not appear in the comment's unknown metadata.
- A comment with a missing parent or malformed front matter appears with a
  null ID, its path and its code.
- Comments for an item with an active worktree are read from that worktree.
- The read succeeds for a comment file that is not yet committed, and returns
  the same result before and after unrelated commits are added.

**Verify:** the two read test targets.

## Task 7: Status And Operation Reads

**Files:** `src/repository/read/status.rs`, schemas and goldens for
`index status`, `poll status`, `operation list`, `operation show`.

**Contract:** `index_status`, `polling_status`, `list_operations`,
`show_operation`.

**Tests first:**

- `index_status` reports `current`, `stale`, `never_refreshed` and
  `unavailable`, and succeeds in the last case.
- `polling_status` reports the stored policy and latest outcome with
  `next_eligible_at` null.
- A pending local operation, an active remote reservation and a key-material
  recovery all appear in one list, ordered by operation ID, each findable by
  ID; an unknown ID is `operation_not_found`. The key-material operation has
  `scope: "application"` and a null `updated_at`.
- A legacy local row with no operation ID sorts last.
- Reading operations takes no write lock: it succeeds while a child process
  holds the shared lock and never opens a read-write connection.
- A failed key-material operation and a remote operation with an outcome
  category each serialize a `failure_code` from the registry.

**Verify:** the two read test targets, plus
`--test recovery_foundation_gate --test remote_reservation`.

## Task 8: Relationship Fields And Index Edges

**Files:** `src/canonical.rs`, `tests/canonical_foundation.rs`,
`src/repository/discovery.rs`, `src/repository/read/items.rs`, schema updates.

**Contract:** `TicketRelationships`, `ticket_relationships`, `is_valid_slug`;
the additive migration in the design; `ObservedItem` carries the view;
`persist_context` writes the short code, edges and relationship problems;
refresh and rebuild set `refreshed_at`; `ItemDto` populates `slug`, `parent`
and `deps`.

**Tests first:**

- The view reads valid `slug`, `parent` and `deps` in block and flow form.
- The three keys stay in `Ticket.unknown` and are absent from the DTO's
  `unknown_metadata`.
- A wrong type, a non-ULID, a self-reference, a duplicate entry and a
  malformed slug are each ignored for the graph and reported with their own
  problem code.
- A ticket carrying all three keys, written in flow form, keeps the same
  `slug`, `parent` and `deps` values after `serialize_item` and after the
  existing ticket save. The test asserts values, not bytes.
- Relationship problems are stored in `item_problems` and none appears in
  `problems` or as a nonconforming list entry.
- Opening an index created before this task adds the column and table and
  marks every registration refresh-required; item reads then report stale.
- After refresh, edges and the short code are present; after rebuild from a
  deleted index, they are identical.
- Removing a dependency from the file and refreshing removes its edge.
- `refreshed_at` is null before the first refresh and set after.

**Verify:** `devenv shell -- cargo test --locked --test canonical_foundation --test discovery_rebuild --test read_boundary --test read_contract`.

## Task 9: Relationship Queries

**Files:** `src/repository/read/graph.rs` and its sibling test file,
`src/repository/read/items.rs`, `tests/read_relationships.rs`, schemas and
goldens for `ticket ready`, `ticket blocked`, `ticket deps`,
`ticket children`, `ticket cycles`, `ticket plan`, `ticket critical-path`,
`ticket find`.

**Contract:** The design's Queries section, and the `slug` and `readiness`
members of `TicketFilter`.

**Unit tests first, on `graph.rs` alone:** readiness for no dependencies, all
closed, one open, one unresolved and one on a cycle; a self-loop; two
overlapping cycles; a parent cycle; a diamond in a tree marked repeated once;
depth limits; plan batches for a chain, a fan-out and a fan-in; unplannable
tickets downstream of a cycle; critical path with a tie broken by ID.

**Integration tests first:**

- Ready and blocked across primary and two active worktrees, where a
  dependency is closed in its own context.
- A dependency on an ID found in no context blocks and is reported as
  unresolved; a `deps` entry and a `parent` that name a document are ignored,
  reported, and do not block.
- Dependency trees down, up and both, with a depth limit; children of a
  parent, and a ticket's parent on its own DTO.
- A project filter on `ticket plan` and `ticket ready` changes which tickets
  are returned and never which are ready.
- A ticket with status text `closed` does not unblock its dependents.
- Closing a dependency makes its dependent ready on the next read without
  refreshing the dependent's context.
- Two branches, each valid alone, that together form a cycle: after both are
  present, both tickets are blocked with the cycle named, `ticket cycles`
  reports it once, and no file changed.
- Two tickets with the same short code: `find` returns both with their
  contexts; neither is flagged nonconforming.
- A slug lookup with zero, one and two matches; the lookup is
  case-insensitive.
- The project's own shape: fourteen tickets with the Wave 03 Cycle
  dependencies give F1 alone as ready and a plan whose batches match the Wave
  document's tracks.

**Verify:** `devenv shell -- cargo test --locked --test read_relationships --test read_contract`.

## Task 10: Verify, Review And Handoff

1. Add `--test read_boundary --test read_contract --test read_relationships`
   to the test list in `.github/workflows/build.yml`. Do not enable or
   dispatch the workflow.
2. Confirm a schema exists for the envelope and every DTO, and a golden for
   every read listed in the Cycle's scope. Run the redaction scan over all of
   them.
3. Run the full gate:

   ```sh
   devenv shell -- cargo check --all-features --locked
   devenv shell -- cargo fmt --check
   devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
   devenv shell -- cargo test --all-features --locked   devenv shell -- cargo run --locked --bin manyhands-cli
   devenv shell -- cargo test --locked --test read_boundary --test read_contract --test read_relationships
   ```

   The last command builds the library and runs the three new targets without
   the `desktop` feature.
4. Measure `list_tickets`, `show_item`, `list_comments` and `ticket_plan`
   against a generated 1,000-item repository and record the times. This is a
   characterization for later Cycles, not a pass or fail gate in F1.
5. Request an independent whole-Cycle review of the branch range. Send fixes
   back through the same task discipline.
6. Record on the ticket: every acceptance row of the Cycle document with its
   test, the full gate output, the review result, and the open obligations
   (native execution and native path matching; conflict inspection; polling
   fields; empty folders; `created_by` written by F2).
7. Mark the ticket review-ready. Do not push, open a pull request, merge,
   close the ticket or remove the worktree without separate authorization.

## Plan Self-Review

- Every acceptance row in the Cycle document maps to named tests: resolution
  (Task 3); read services (Tasks 3–7); nonconforming content, closure and
  effective copy (Task 5); stale and unavailable index (Tasks 5, 7, 8);
  relationships (Tasks 8, 9); no side effects (Task 2 and each later task);
  redaction (Tasks 1, 2, 3, 4, 5, 10); published contract (Task 2 onward,
  Task 10); front-end independence and regression (Task 10).
- No task depends on a later one.
- Nothing here is evidence yet. Every test above is planned, not passing.
- If decision 1's separate ticket is not accepted, the effective-copy fix
  becomes a task before Task 1.
- If decision 2 is not accepted, a conflict read is added to Task 7 with a
  DTO that reports no conflicts, and its shape is revisited after Wave 02
  Cycle 06.
- If decision 4 is not accepted, `author` is always null in F1, or Task 6
  gains a full-history walk whose cost the design describes.
- If decision 6 is not accepted, the migration triggers a refresh and the
  "reads never scan" contract needs an exception.
