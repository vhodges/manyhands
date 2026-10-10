---
title: "Wave 03 F2 Request Replay, Confirmation And Shared Mutation Bridges Implementation Plan"
date: 2026-10-08
status: draft
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M4ERD7MTGH26QPW26XTEXQFE"
---

# Request Replay, Confirmation And Shared Mutation Bridges Implementation Plan

> **For agentic workers:** Do not begin until the product owner has approved
> these documents, decided the items in the Cycle document, and explicitly
> authorized implementation. Then use `implementing-a-cycle` and the selected
> execution method task by task, re-read the design, repeat the ticket
> preflight, and record every checkpoint on ticket
> `01M4CC0VMR8HSPZXQ1WX41GWVK`.

**Goal:** Give both front ends one headless way to change Manyhands state:
replayable requests, confirmation, progress, cancellation and one result
envelope, plus the mutations the baseline lacks.

**Architecture:** A new `repository::mutation` module holds the boundary:
`prepare`, `execute`, `show_request`, `cancel_request` and
`resume_operation` on the existing `RepositoryService`, over a closed
`Mutation` enum with one binding per command. Each binding makes one call to
an existing domain operation with an operation ID the boundary allocates and
records. Request and confirmation records are three new tables in the
existing database; they hold digests and results, never content, and are not
a journal. New domain operations (identity, folder, repair) live in
`repository::bridges`. Relationship writers, short codes and the comment
author are changes to `canonical` and the existing save paths.

**Tech stack:** Rust 2024 and the locked dependencies already in use:
`git2`, `rusqlite`, `serde`, `serde_json`, `serde_yaml`, `toml`, `time`,
`ulid`, `blake3`. No dependency is added. Devenv.

**Spec:** [Cycle](../Cycles/wave-03-foundation-02-replay-confirmation-and-bridges.md)
and [design](2026-10-08-wave-03-foundation-02-replay-confirmation-and-bridges-design.md).

**Status:** Draft for product-owner review, revised three times after
independent review. The replay rules of Tasks 6 and 7 follow a mechanism
that has failed review in its details three times; see the checkpoint after
Task 7. It assumes the recommended option of each decision in the Cycle
document; a different decision changes the tasks named under
[Decision Dependencies](#decision-dependencies).

## Global Constraints

- Work only in ticket `01M4CC0VMR8HSPZXQ1WX41GWVK`'s worktree and branch.
  Preserve unrelated edits in main and in every other worktree.
- Before implementation, fetch `origin/main`, rebase this branch onto it,
  verify ancestry and cleanliness, and record the base and both heads. The
  branch is unpushed at planning time. Once it is pushed, do not rebase it
  or force-push; merge main in instead.
- Run every Rust command as `devenv shell -- cargo …`.
- Keep everything in the headless library. Add no GPUI dependency, no CLI
  parsing, no human output, no exit numbers, no prompt and no thread.
- Add no dependency.
- No domain type derives `Serialize` or `Deserialize`. The existing
  `compile_fail` doctests for credential types must still pass.
- The boundary takes no lease and holds no lock between calls. It releases
  the cache guard before it runs a binding. It adds no journal. Every Git or
  canonical effect is made by a domain operation under that operation's own
  lease, reservation and journal row.
- Change existing code only as the design lists under "Changes To Existing
  Code". Where the boundary needs a variant of an existing function, add a
  sibling and have the existing one delegate.
- Add no column to `operation_records`.
  `assert_operation_records_hold_no_content` must keep passing unchanged.
- Never store, log, emit or return a Markdown body, a source, a passphrase, a
  private key, a credential or server text from the boundary. A body is fed
  to the digest and dropped.
- Never call `Display` or `Debug` on a `RepositoryError`, a `git2`,
  `rusqlite`, I/O, transport or key error, or a URL, on any path that reaches
  an envelope, preview, event or record.
- Every mapping from a domain enum to a code is a `match` with no wildcard
  arm.
- Never generate an item or comment ID inside the library on a caller's
  behalf.
- Every test that expects a rejection names the code it expects.
- Tests use disposable real repositories, isolated home, Git configuration
  and application data, and the existing SSH fixture. No test touches a
  developer's repository, keys or global configuration.
- Write the test for each behavior before the code that satisfies it.
- Do not dispatch any workflow. Adding test targets to the workflow's list
  is a file change only.

## Review Focus

1. **A retry cannot repeat or undo.** Task 7 saves an item again between an
   attempt and its retry, for a finished request and for one whose result
   was lost, and proves each retry leaves the second save in place.
2. **Changed input is caught by the boundary, not the journal.** Task 7
   proves a changed body, title and relationship are each
   `request_mismatch`.
3. **Nothing is left behind by a rejection.** Every binding's rejection tests
   end by running a different request on the same repository.
4. **No content in records.** The sentinel scan added in Task 6 runs over the
   request tables, the three journals and every envelope, preview and event
   in every later task's scenarios.
5. **A preview creates nothing.** Task 11 snapshots the file system around
   `prepare` for a root that does not exist.
6. **Partial is reported as partial.** Each binding family has a test with a
   failure injected after its authoritative step.
7. **No second pin writer.** Task 16 adds no code that writes a host pin; the
   reviewer confirms by reading the diff, since no test can prove an
   absence.
8. **A cycle is rejected before anything exists.** Task 7 asserts no branch,
   no worktree, no journal row and no request record after a rejected
   create.
9. **The short code is written once.** Task 2 proves a retried create keeps
   the file's slug when a different one is passed.

## Decision Dependencies

| Decision | If not as recommended |
| --- | --- |
| 1, size | A split moves Part C, or Parts B and C, to a new Cycle; tasks are unchanged. |
| 2, journal defect | The fix is merged. The abandon question, if decided for, is its own ticket and changes no task here. |
| 3, effects | Tasks 5, 9, 10, 12, 15 and 16 change, and the envelope schema needs a new version. |
| 4, folder listing | Task 17 drops the index table, the read and the refresh. |
| 5, closed tickets | Tasks 7 and 8 drop or narrow the `ticket_closed` guard. |
| 6, host approval | Task 16 grows to add a transport path; re-plan it. |
| 7, create with a global identity | Task 11 changes `create_and_enable` instead of the binding. |
| 8, retention | A pruning rule adds a step to Task 6. |
| 9, commands bound | Binding `comment add` and polling adds a task after Task 13. |
| 10, registry names | Names change in Task 5 only. |
| 11, native evidence | Task 19 step 1 changes. |
| 12, synchronization after Cycle 06 | As recommended, Task 13 gains the divergent cases and Task 9 the three new error variants. Binding `conflict resolve` adds a task after Task 13. Dropping the two bindings removes Task 13's synchronization tests and the remote half of Task 14. |

## File Map And Dependency Order

| Path | Task | Change |
| --- | --- | --- |
| `src/canonical.rs`, `tests/canonical_foundation.rs` | 1, 2, 4 | Short code, initials, slug settings; relationship writers; `Comment.created_by`. |
| `tests/fixtures/slug_v1/vectors.json` | 1 | New golden vectors. |
| `src/repository.rs` | 2, 4, 6, 11, 17, 18 | `save_ticket_with`; `created_by` in `submit_comment`; `mod mutation; mod bridges;`; prefix in the configuration writer; `ContextIntent::Repair`. |
| `src/repository/read/graph.rs`, `read/items.rs`, `read/mod.rs`, `read/dto.rs` | 3 | `check_ticket_relationships` and its DTO; `TicketGraph` visibility. |
| `src/repository/read/comments.rs` | 4 | Author from the field. |
| `tests/mutation_relationships.rs` | 2, 3 | New. |
| `src/results.rs`, `src/results_tests.rs` | 5 | Mutation constructor, class rule, codes, actions. |
| `tests/read_contract.rs`, `tests/support/golden.rs` | 5 | The read-only assumptions adjusted; shared helpers generalized. |
| `src/repository/mutation/{mod,identity,records,dto}.rs` | 6, 7, 13, 14, 16 | New. |
| `src/repository/discovery.rs` | 6, 17 | Call the new table migrations; record folders. |
| `tests/mutation_replay.rs`, `tests/mutation_contract.rs`, `tests/support/mutation.rs` | 6, 7 | New; extended by later tasks. |
| `src/repository/mutation/{outcome,evidence,observe}.rs`, `bind/ticket.rs` | 7 | New. |
| `schemas/v1/*.schema.json`, `tests/fixtures/mutation_v1/*.json` | 3, 5–18 | New and extended, per task. |
| `.gitattributes` | 1, 6 | `-text` for `tests/fixtures/slug_v1/` and `tests/fixtures/mutation_v1/`. |
| `src/repository/mutation/bind/document.rs`, `src/repository/read/items.rs` | 8 | New; `observe_path`. |
| `src/repository/mutation/bind/{remote,index}.rs` | 9 | New. |
| `src/repository/mutation/bind/key.rs` | 10, 12 | New. |
| `src/repository/mutation/confirm.rs`, `bind/repo.rs`, `tests/mutation_confirmation.rs` | 11, 12, 15, 18 | New. |
| `src/repository/mutation/progress.rs`, `bind/sync.rs`, `src/repository/remote/sync.rs` | 13 | New; sibling entry point with a safe-point observer. |
| `tests/mutation_remote.rs`, `tests/support/remote_world.rs`, `tests/remote_synchronization.rs`, `Cargo.toml` | 13, 14, 16 | New SSH-harness target, `harness = false`; the `World` fixture moved to shared support. |
| `src/repository/bridges/identity.rs`, `src/repository/read/admin.rs`, `read/dto.rs` | 15 | New; `IdentityDto` initials. |
| `src/repository/mutation/bind/host.rs` | 16 | New. |
| `src/repository/bridges/folder.rs`, `mutation/bind/folder.rs`, `read/items.rs` | 17 | New; `list_document_folders`. |
| `src/repository/bridges/repair.rs`, `mutation/bind/repair.rs`, `src/repository/recovery.rs`, `read/dto.rs` | 18 | New; `repair_item` action name. |
| `tests/mutation_bridges.rs` | 15, 17, 18 | New. |
| `.github/workflows/build.yml` | 19 | Add the six new test targets to the list. |

Tasks run in order. Part A (Tasks 1–4) depends on nothing in Part B and not
on the journal fix. Part B depends on the journal fix. Tasks 15–18 each
depend on Parts A and B and touch separate source files; they are
implemented one at a time because they share the test and schema files.

## Baseline And Dependency Checkpoint

Before Task 1:

```sh
git fetch origin main
git merge origin/main         # main was merged, not rebased, on 2026-10-10
git merge-base --is-ancestor origin/main HEAD
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo test --all-features --locked
devenv shell -- cargo run --locked --bin manyhands-cli
```

Record the base, both heads and each command's result on the ticket. A
failure here is resolved or explicitly accepted by the product owner before
Task 1.

Re-check the design's mutation-surface audit against any change to
`src/repository*`, `src/canonical.rs` or `src/results.rs` since `6cf5d7f`.
The journal, operation-lookup and synchronization rows were re-read at
`f87ce81` on 2026-10-10; the rest was not.
In particular, check whether any of the open F1 follow-up tickets has
merged; `01M4EHGE4BXGPMCWWA1S1QR0JW` and `01M4EHGE9TMR99VYZC184J9XEC` touch
refresh and discovery, which Tasks 6 and 17 extend. If a listed function
moved or changed behavior, update the design before coding.

## Prerequisite: Journal Rows After A Rejection

**Done.** Fixed under defect ticket `01M4EWN2DK3MY6H4GBYDYXF6QH` and merged
to main on 2026-10-10; evidence is `tests/journal_rejection.rs`. No task
here remains. What follows is the requirement F2 placed on that work, kept
as written for the record; the design's "The Journal Defect" says what was
delivered, including where it differs: a closed row is kept and marked
`rejected`, three assertions in `tests/repository_enablement.rs` were
changed by product-owner ruling, and `remove_registration` was dropped as
not reproducible.

**Files:** `src/repository.rs`, `tests/repository_enablement.rs`,
`tests/local_authoring.rs`, `tests/recovery_foundation_gate.rs`.

**Contract:** The design's "The Journal Defect". When an operation returns an
error, the journal row was begun by this call, and this call wrote nothing
durable or rolled back what it wrote, the row is completed. Whether the
call wrote is tracked by the call itself; it cannot be read from the error
kind or the recorded step. A row stays pending when an effect was made and
work remains, after a failure inside a standalone `prepare_context`, and
after a failed refresh or rebuild, which this fix does not cover.
Creating an editing context as part of a save is not such an effect;
initializing a repository is.

**Tests first.** Each test provokes the rejection, then runs a different
operation with a new ID on the same repository and expects it to succeed.
Before the fix each second call returns `RecoveryRequired`; record that each
test failed that way first, which is the reproduction the planning lacked.
If one does not fail first, the path was not defective: say so and drop it.

- `enable`: wrong branch; dirty worktree; conflicted worktree; invalid
  configuration; invalid identity; a rollback after an injected failure.
- `create_and_enable`: a target that is a file; a non-empty directory. Then
  `enable` on a sibling directory, and on the same directory once emptied.
- `add_remote` with an invalid name; `remove_remote` and
  `set_publication_remote` with a Git error, such as a detached head.
- `remove_registration` with a failure injected before its transaction.
- A save and a comment submit on a repository that is not enabled; with a
  wrong checked-out branch on primary; with a dirty configuration path.
- A save rejected for a stale expectation after its context was created.
- A save that returns busy, and one that returns an injected SQLite failure,
  before any write.
- `remove_registration` succeeds after each of the above.
- **Still pending, correctly, and unchanged:**
  - a save with a failure injected after its file write, including the
    `AfterOwnedWriteBeforeLifecyclePersistence` case, which returns a
    SQLite error with no step recorded
    (`tests/local_authoring.rs`, near line 6417);
  - a create that failed after the repository was initialized
    (`tests/recovery_foundation_gate.rs`, near line 1388);
  - the two standalone `prepare_context` failures
    (`tests/local_authoring.rs`, near lines 5689 and 6466);
  - a failed rebuild and two failed refreshes
    (`tests/recovery_foundation_gate.rs`, near line 951;
    `tests/discovery_rebuild.rs`, near lines 1575 and 2100).
- After `enable` rolls back inside a `create_and_enable`, a repeat of the
  create with the same operation ID: decide what it returns, and test it.
- Every existing test passes unchanged.

**Verify:** `devenv shell -- cargo test --locked --test repository_enablement --test local_authoring --test recovery_foundation_gate --test discovery_rebuild`.

# Part A: Canonical Writes

## Task 1: Short Codes And Initials

**Files:** `src/canonical.rs`, `tests/canonical_foundation.rs`,
`tests/fixtures/slug_v1/vectors.json`, `.gitattributes`.

**Contract:** Pure functions, no I/O: `short_code(ulid_text, length)`,
`derive_initials(name)`, `normalize_initials(text)`,
`slug_settings(&RepositoryConfig)` returning the validated prefix and length
or a typed problem, and `compose_slug(prefix, initials, code)`. The design's
Short codes section fixes the bit order and alphabet.

**Tests first:**

- Golden vectors: at least eight ULIDs, each with its code at lengths 5, 6,
  7 and 8, and at least eight names with their initials or none. Compute
  the expected codes outside this crate: an independent BLAKE3
  implementation on the 26 ASCII characters, such as `b3sum`, and a few
  lines of shell or `node` for the base-32 step. Record the tool, its
  version and the script's text in the file's `method` field. Derive the
  expected initials by hand from the RFC's rule.
- The code at length 6, 7 and 8 begins with the code at length 5.
- Initials: two words; three words (first and last); one word (first two
  characters); one character (none); leading non-ASCII letter (none);
  mixed case lowercased; digits accepted; surrounding and repeated
  whitespace ignored.
- `normalize_initials`: lowercases; accepts two and three characters;
  rejects one, four, punctuation and non-ASCII.
- `slug_settings`: absent keys give no prefix and length 5; a valid prefix
  and each length 5 to 8; an over-long prefix, an upper-case prefix, a
  non-string prefix, a length of 4 and of 9 and a non-integer length each
  give the typed problem. The configuration itself still parses in every
  case.
- Every composed slug satisfies the existing `is_valid_slug`.

**Verify:** `devenv shell -- cargo test --locked --test canonical_foundation`.

## Task 2: Relationship Writers

**Files:** `src/canonical.rs`, `src/repository.rs`,
`tests/canonical_foundation.rs`, `tests/mutation_relationships.rs`.

**Contract:** The design's Storage and canonical form.
`save_ticket_with(request, TicketWriteOptions)`: the options carry `deps` and
`parent`, each as unchanged, cleared or set, and an optional slug that is
written only if the file has no `slug` key or has it as null. `save_ticket`
delegates with defaults and behaves as before. The function does not read
configuration or initials and does not check cycles; the boundary does both
before it calls.

**Tests first:**

- A set `deps` is written as a block sequence, sorted, unique; an empty list
  and a clear both remove the key; `parent` is one string and a clear
  removes the key.
- A hand-written flow-sequence, unsorted `deps` is rewritten sorted by a
  save that changes the title, and left alone by a save that changes
  nothing: no write, no commit.
- A `deps` with a repeated entry, an invalid `parent` and an invalid `slug`
  are each preserved in value through a save that does not set them.
- Other unknown keys keep their values and their order.
- A slug and a ULID that YAML would read as a number (`1e-12345`, an
  all-digit ULID) are written so that they read back as the same strings.
  Pin the bytes the serializer emits in a golden.
- Create with a slug option writes it. Create without one writes none, as
  today.
- **Written once:** a create interrupted after the file write
  (`AfterOwnedWriteBeforeLifecyclePersistence`), then retried with the same
  operation ID and a different slug option, completes and keeps the first
  slug.
- An edit with a slug option adds it to a ticket without the key, and to one
  with `slug: null`. For a ticket with a valid slug, and one with an
  invalid `slug` value, the option is ignored and the value in the file is
  kept; the function does not reject. The binding owns that rejection.
- An edit with a slug option, interrupted after the write and retried with
  the same operation ID, completes with the slug first written.
- A slug is unchanged by a later save that changes the title.
- `tests/canonical_foundation.rs` has a test near line 1420 that expects
  `deps` in file order to survive serialization. Update it to the canonical
  order and say so in the commit.
- Every existing test in `tests/local_authoring.rs` passes unchanged.

**Verify:** `devenv shell -- cargo test --locked --test canonical_foundation --test mutation_relationships --test local_authoring --test read_relationships`.

## Task 3: Relationship Check

**Files:** `src/repository/read/graph.rs`, `read/items.rs`, `read/mod.rs`,
`read/dto.rs`, `tests/mutation_relationships.rs`, the `relationship_check`
schema and goldens.

**Contract:** `check_ticket_relationships(repository, item_id, proposed)`, a
public read in a read session. It returns the normalized `deps` and
`parent` and the unresolved IDs, or a rejection: `relationship_cycle` with
the members sorted by ID, or `invalid_relationship`. It scans nothing and
writes nothing.

**Tests first:**

- A two-ticket dependency cycle, a three-ticket one, a parent cycle and a
  self-reference in each field are `relationship_cycle`, naming the members.
- A cycle through a lifecycle-closed ticket is rejected.
- The proposed edges replace the ticket's stored ones: removing the edge that
  would close a cycle while adding another is accepted.
- A target the index holds as a document, and as a comment, is
  `invalid_relationship`.
- An unknown target is accepted and listed as unresolved.
- Repeated entries are removed.
- A ticket ID not yet in the index can be checked with `deps` and a `parent`.
- With the index unavailable the result is `index_unavailable`.
- With a stale index that lacks a ticket, the check passes on what it has,
  and after a refresh F1's read reports the cycle. This pins the stated
  limit.
- The check joins F1's no-side-effect snapshot test.

**Verify:** `devenv shell -- cargo test --locked --test mutation_relationships --test read_relationships --test read_boundary`.

## Task 4: Comment Author

**Files:** `src/canonical.rs`, `src/repository.rs`,
`src/repository/read/comments.rs`, `tests/canonical_foundation.rs`,
`tests/local_authoring.rs`, `tests/read_comments.rs`.

**Contract:** The design's Comment Author section.

**Tests first:**

- A new comment's file has a `created_by` whose value parses as
  `Name <email>` for the fixture identity. Pin the emitted bytes in the
  existing style of the `closed_by` test.
- A retried submit after a failure injected between the write and the
  checkpoint keeps the file's `created_at` and `created_by` and is not
  reported as an occupied path.
- The same, with the identity changed before the retry: the file names the
  first identity, the commit is authored by the second, and the call
  succeeds.
- A comment file without the field still parses and reads with a null
  author. A file whose `created_by` is a list is conforming and is reported
  with the existing invalid-field problem.
- `list_comments` returns the author for a comment created through
  `submit_comment`.
- The order of `submit_comment`'s existing rejections is unchanged: the
  existing rejection tests pass unchanged.
- F1's `comment_list` golden is built from hand-written files and must not
  change. Confirm it does not.

**Verify:** `devenv shell -- cargo test --locked --test canonical_foundation --test local_authoring --test read_comments --test read_contract`.

## Part A Checkpoint

Run the full gate (see Task 19). Have an independent reviewer read
`<base>..HEAD`. Record results and rulings in the execution ledger and on
the ticket. The branch could merge here without leaving the library
half-built: relationship writes and the comment author are complete library
functions, and nothing yet depends on the boundary.

# Part B: The Mutation Boundary

Confirm the journal fix is on main and this branch contains it. If it is
not, stop.

## Task 5: Result Model For Mutations

**Files:** `src/results.rs`, `src/results_tests.rs`,
`schemas/v1/envelope.schema.json`, `schemas/v1/recovery_action.schema.json`,
`tests/read_contract.rs`, `tests/support/golden.rs`.

**Contract:** The design's Result Mapping section: `Envelope::mutation` with
an explicit outcome; `Envelope::failure_class`, the four-step rule; the
result codes in the design's table with class and fixed message; the
recovery actions in the design's table; `RecoveryAction::for_operation`.

**Tests first:**

- The code list and the action list match the design's tables exactly, each
  with a unique string, a class and a non-empty message.
- `operation.resume` registers no argument keys and serializes with
  `"arguments": {}` and a non-null `operation_id`, as the CLI RFC's example.
- The class rule over a table: a cancellation; `external_change` with no
  effect (Blocked) and with a committed checkpoint (`partial`, Incomplete);
  a transient failure before and after an effect; `success`; `noop`.
- A mutation envelope serializes all eleven fields; `data` may be non-null
  on a non-success outcome.
- The envelope schema's code enumeration and the recovery-action schema's
  action enumeration equal the registries.
- The read test comparing the actions reads suggest with the whole registry
  now compares them with the three read actions. The test that a code's
  outcome follows its class is limited to the codes reads return.
  `assert_contract` keeps its rule for reads; a sibling for mutations takes
  the expected data presence per case.
- No outcome or effect string changed: compare with a literal list.

**Verify:** `devenv shell -- cargo test --locked --lib results --test read_contract`.

## Task 6: Request Identity And Records

**Files:** `src/repository.rs`, `src/repository/mutation/mod.rs`,
`identity.rs`, `records.rs`, `dto.rs`, `src/repository/discovery.rs`,
`tests/mutation_replay.rs`, `tests/support/mutation.rs`, the `request`
schema and goldens, `.gitattributes`.

**Contract:** `RequestId`, `ConfirmationId`; the intent digest; the three
tables and their migration; record insert, lookup, entering (which raises
the attempt number), and finish and delete, each conditional on the record
being `accepted` with the caller's attempt number; `show_request`; an
injectable clock on the service; and in each of the three journals a lookup
of one operation by ID that says absent, pending with its step or phase, or
completed. No `execute` yet.

Also **Files:** `src/repository/recovery.rs`,
`src/repository/remote/state.rs`, `src/repository/keys/registry.rs`.

**Tests first:**

- Digest: stable for equal input; different for a changed title, body,
  status, `deps`, `parent`, observation token, target, command, repository
  and salt; absent, null and empty are three different inputs; field
  boundaries cannot be shifted (`"ab","c"` against `"a","bc"`).
- The digest for `repo create` on a root that does not exist equals the
  digest for the same root after it is created.
- The migration adds the tables to a database from before this Cycle and
  leaves every other table and row as it was.
- Insert is refused for an existing request ID. Finish succeeds once on an
  accepted record and changes nothing on a finished one.
- With attempts 1 and 2 entered, a finish or delete by attempt 1 changes
  nothing; by attempt 2 it applies.
- The journal lookup returns absent for a local row closed as `rejected`.
- The journal lookup returns absent, pending with its step and completed
  for a local operation, a synchronization (in the remote journal and, for
  one bound locally with no remote, in the local one) and a key operation.
  It works for a root that no longer exists.
- Deleting a record releases the confirmation it accepted, in the same
  transaction; the confirmation's expiry time is unchanged.
- `show_request` returns `accepted`, `finished` with its stored result, and
  `request_not_found`.
- `remove_registration` leaves request and confirmation rows; the existing
  operation-record deletion is unchanged.
- With the database unavailable, record access reports it; nothing panics.
- The cache guard is not held when the record functions return: a domain
  call made straight after succeeds.
- The privacy helper: given planted sentinels, it scans every column of the
  three tables and the three journals. Prove it fails when a sentinel is
  written to a record by a test-only path.

**Verify:** `devenv shell -- cargo test --locked --test mutation_replay --test discovery_rebuild`.

## Task 7: Execute And Replay Through Ticket Create And Save

**Files:** `src/repository/mutation/mod.rs`, `outcome.rs`, `evidence.rs`,
`observe.rs`, `bind/ticket.rs`, `tests/mutation_replay.rs`,
`tests/mutation_contract.rs`, schemas and goldens for `ticket create` and
`ticket save`.

**Contract:** The rows of the design's "Required outcomes" table that a save
or a create can produce are this task's specification. Write a test for
each row first. Then build "What execute does", Settling, "Re-entering a
request", the already-applied rule and "After the cache is lost" to pass
them, through `ticket create` and `ticket save`. Where the design's
mechanism cannot meet a row, the row wins: change the mechanism, record
the change in the ledger and carry it to the checkpoint below. Token checking and the primary-is-committed rule from
Observations. The create binding composes the short code. The save binding
refuses a closed ticket, if decision 5 is as recommended. `outcome.rs`
starts the exhaustive mappings with the kinds these bindings return and an
explicit `internal_error` arm for each other kind.

**Tests first:**

- A new request runs once and returns `written`, `committed`, the commit and
  `current`; the envelope carries the request ID and the operation ID.
- Create writes a slug from the configured prefix and length and the derived
  initials; with `manyhands.initials` set, from that; with one-call
  initials, from those, and local configuration is unchanged afterwards.
- A name that yields no initials is `initials_required`, and an invalid
  configured prefix is `invalid_slug_configuration`: no context, branch,
  journal row or request record. A save of an existing ticket in the same
  repository still works.
- A cycle is `relationship_cycle` with the same nothing-exists assertions;
  an unknown target is accepted and listed in `data`.
- **Finished replay:** the same call returns the same envelope and adds no
  commit. Then save the item again with a new request, retry the first, and
  get the first result with the second save still on disk.
- **Finished replay with the commit unreachable** (the branch reset behind
  it) is `recovery_required` and writes nothing.
- **Changed input:** the same request ID with a changed body, title or
  relationship is `request_mismatch`; the repository and the record are
  unchanged. After a partial result, the same mismatch is `partial`.
- **Interruption:** with a failure injected at each of `BeforeItemWrite`,
  `AfterOwnedWriteBeforeLifecyclePersistence`, `BeforeCheckpointCommit` and
  `BeforeIndexTransactionCommit`, a retry on a fresh service completes;
  exactly one checkpoint commit exists. For the points after the write the
  record stayed `accepted` and the retry succeeds although the caller's
  token is now stale. For `BeforeItemWrite` nothing was in flight, the
  record was deleted, and the retry is a new request.
- The same four with the process killed instead of an injected error (a
  child process that exits at the fault point): the journal row is pending
  in every case, and the retry with the same body completes.
- **Lost output:** the domain call succeeds and the test discards the result
  before the record is settled (a boundary fault point added for this). The
  retry returns `committed` with that commit and makes none.
- **Lost output, then another save:** as above, with a second request saving
  different content in between. The retry reports the first commit and the
  file still holds the second save.
- **Lost output of a no-change save:** the retry reports `unchanged` and no
  commit, although earlier commits on the branch have the same content.
- **Unrelated commits do not stop a retry:** after an interrupted first save
  of an item with no context, add a commit to primary; after an interrupted
  save of an item with a context, add a commit to its branch that touches
  another path. Each retry completes.
- **Foreign change:** an accepted request whose path was committed with
  different content by someone else is `external_change`; nothing is
  written and the record stays `accepted`.
- **Identical content from another request:** a request that never reached
  the domain, then a second request saving the same content, then a retry
  of the first: the retry is `already_applied` and reports no commit.
- **A late caller:** a request with a stale token whose content already
  equals the item, committed, and no record: `already_applied`, nothing
  written. The same with the file equal and uncommitted:
  `external_change`.
- **The commit is found, the journal step is not:** a save commits and the
  journal update that follows returns busy (the cache guard held by a
  child process). The result is `partial`; the retry calls the save again,
  which writes and commits nothing, completes its row and hand-off, and
  the record is `finished` with that commit. A different request then
  succeeds.
- **An error after the write is not "nothing happened":** with an injected
  SQLite failure after the file write, the result is `partial` with
  `write: written` and `checkpoint: pending`, the record stays
  `accepted`, and a retry finishes it.
- **Concurrent duplicate:** while one call is inside the domain operation
  (held by a boundary test hook), a second call with the same request
  returns `busy` and leaves the record `accepted`. The first completes and
  returns its commit to its caller; it does not settle the record, because
  its attempt is no longer current. A third call then finishes the record
  with that same commit. One commit exists.
- A second call that checks evidence before the first commits and gets the
  lease after it reports the first's commit, not a no-op.
- A slug is unchanged by an identity change and by a prefix change, each
  followed by a save through the boundary. A create interrupted after its
  write and retried after the initials changed commits with the slug first
  written. A foreign commit that leaves a created ticket's other fields
  equal and adds no slug is not taken for the request's own.
- A failure after the checkpoint and before discovery is `partial` with
  `discovery_pending`, `checkpoint: committed` and an `operation.resume`
  action naming the operation; the record stays `accepted` and a retry
  finishes it.
- **Nothing left behind:** a stale token, an invalid input and a busy lease
  before any write (held by a child process) each leave no request record
  and no pending journal row; a different request then succeeds, and the
  same request ID can be used again with corrected input.
- A stale token on a first save creates no context. An uncommitted edit of
  the file on primary is `worktree_not_clean`, with no context.
- A change made between the boundary's check and the domain's is caught by
  the domain and reported as `external_change`. Use
  `set_owned_path_hook_for_testing` at the read boundary; Unix only, and
  the hook is process-global, so the test holds the existing guard.
- A no-change save is `noop` with `already_applied`, no commit, and
  `discovery: not_requested`.
- A lifecycle-closed ticket is `ticket_closed`; an open ticket whose status
  text is `closed` is saved.
- **Cache loss:** delete the database, rebuild with the root, then retry:
  a completed create is `already_applied`; a completed save is
  `already_applied`; a save that died before its checkpoint is
  `external_change`, and a fresh read and save commits it; a save with the
  file changed by someone else is `external_change`. No commit is added by
  any retry. Before the rebuild every one of them is
  `repository_not_registered` with the `index.rebuild` action.
- Two processes submitting the same new request ID at once: one commit
  exists and both get a result or `busy`.
- **Privacy:** the body, title and a sentinel appear in no column of the
  request tables or journals and in no envelope.
- Goldens for the success, replay, mismatch, partial, cycle and no-op
  envelopes match their files and schemas.
- The library and these targets build without the `desktop` feature.

**Verify:** `devenv shell -- cargo test --locked --test mutation_replay --test mutation_contract --test mutation_relationships`.

## Design Checkpoint After Task 7

Stop here. Before Task 8:

1. Rewrite the design's "What execute does", Settling and "Re-entering a
   request" to describe what was built, and note each place the mechanism
   changed to satisfy a required outcome.
2. Have an independent reviewer read the code and tests of Tasks 6 and 7
   against the required-outcomes table, with the instruction to find a
   sequence of events the tests do not cover.
3. Report to the product owner: which outcomes are proven, what changed,
   and whether the open cases of the journal defect were met in practice.
   Continue only when told to.

Every later binding reuses this machinery, so an error left here is copied
twenty-six times.

## Task 8: Observations And Document Bindings

**Files:** `src/repository/mutation/observe.rs`, `bind/document.rs`,
`bind/ticket.rs`, `src/repository/read/items.rs`, schemas and goldens.

**Contract:** `observe_path`. Bindings for `document create`,
`document save`, `document move` and `ticket slug-assign`.

**Tests first:**

- `observe_path` for an absent path and a file; a directory is
  `invalid_path`; the absent token differs by branch and by path. It joins
  the no-side-effect snapshot test.
- `document save` and `document move` without a source observation are
  `invalid_input` before any write, removal or checkpoint.
- `document move`: success, with both paths in one commit; a stale source
  token is `external_change`; a stale destination token is
  `external_change`; an occupied destination is `occupied_path`.
- `document create` with an ID already used, under a new request, is
  `occupied_path`.
- `ticket slug-assign`: success; `slug_already_assigned`; a stale token is
  `external_change`; a closed ticket is `ticket_closed`; a retry after a
  lost result reports the assigning commit.
- For each new binding, table-driven: replay of a finished request; changed
  input rejected; one interruption retried to completion; a failure after
  the checkpoint reported as `partial`; nothing left behind after a
  rejection.

**Verify:** the Task 7 command.

## Task 9: Complete Outcome Mapping; Remote And Index Bindings

**Files:** `src/repository/mutation/outcome.rs`, `bind/remote.rs`,
`bind/index.rs`, schemas and goldens.

**Contract:** Every variant of `RepositoryErrorKind`,
`SynchronizationError`, `SshTransportErrorKind`, `KeyMaterialErrorKind` and
every domain outcome enum maps to a code. Bindings for `remote add`,
`remote select`, `index refresh` and `index rebuild`.

**Tests first:**

- A table test pins the code for every variant; the `match` has no wildcard.
- An error whose message and source contain a sentinel maps to an envelope
  without it.
- `remote add`: success; the same again is `noop`; an invalid name is
  `invalid_remote`; a name in use with another location is
  `remote_name_conflict`. Every effect is `not_requested` and `data` names
  the remote with its location redacted. With the index hand-off failing
  after the remote was added, the result is `partial` with
  `discovery_pending`.
- `remote select`: success with the commit; selecting the selected remote is
  `noop` and reports no commit, on a first call and on a retry; a failure
  after the commit is `partial`; a retry reports that commit.
- `index refresh` and `index rebuild`: `discovery: current`. A failed
  refresh is an error with its code, not `partial`; its record stays
  `accepted`, a different request is refused until it is retried, and the
  retry completes. This pins the stated exception.
- `index rebuild` with the database deleted, and with it corrupt, runs
  without a record and registers the repository again.
- Replay and changed input for each binding.

**Verify:** `devenv shell -- cargo test --locked --test mutation_replay --test mutation_contract --test repository_enablement`.

## Task 10: Key Bindings

**Files:** `src/repository/mutation/bind/key.rs`, schemas and goldens.

**Contract:** Bindings for `key generate`, `key import`, `key select`,
`key clear` and `key remove`.

**Tests first:**

- `key generate`: the digest is the same for two different passphrases and
  differs between protected and unprotected and between labels; a retry
  after the key was created returns the same key and creates no second
  file. With a failure injected after the key pair is written, the result
  is `partial` with `recovery_required`, the record stays `accepted`, and
  the retry registers the key. With a failure before the pair is complete,
  the retry reports the files as retained for inspection, and the record is
  then `finished`.
- No passphrase, and no value derived from one, is in any record, envelope
  or event. `assert_operation_records_hold_no_content` passes unchanged.
- `key import`: success; the same source again is `noop` naming the existing
  key.
- `key select`, `key clear`: success and `noop`.
- `key remove` of the selected key is `selected_key_in_use`; of an unknown
  key, `key_not_found`.
- Every effect is `not_requested`.
- Replay returns the stored result; changed input is rejected.
- **Cache loss:** after the database is deleted, a retried `key generate`
  creates a new key and leaves the earlier file in place.

**Verify:** `devenv shell -- cargo test --locked --test mutation_replay --test mutation_contract --test key_material --test shared_key_registry`.

## Task 11: Confirmation; Repository Create And Enable

**Files:** `src/repository/mutation/confirm.rs`, `observe.rs`,
`bind/repo.rs`, `src/repository.rs`, `tests/mutation_confirmation.rs`, the
`preview` schema and goldens.

**Contract:** The design's Two-Phase Confirmation section and the
create-and-enable part of Local identity. Bindings for `repo create` and
`repo enable`. Enablement's configuration writer accepts an optional prefix.

**Tests first:**

- `prepare` for `repo create` on a root that does not exist: the file system
  under the parent, the repository list and Git's global configuration are
  byte-identical before and after; one confirmation row exists.
- `prepare` for a command that needs no confirmation is `not_confirmable`.
- `execute` without a confirmation is `confirmation_required`, with no
  request record and no effect.
- **Stale preview:** between preview and confirmation, in turn: a file
  appears in the target directory; the checked-out branch changes; the
  worktree becomes dirty; a commit is added. Each is `external_change`
  with no effect and a `request.prepare` action.
- Changed input, a different target and a different command are each
  `confirmation_mismatch`.
- **Expiry:** at ten minutes less one second it is accepted; at ten minutes
  it is `confirmation_expired`; with the clock set before its creation it is
  `confirmation_expired`.
- A second request presenting an accepted confirmation is
  `confirmation_used`.
- **Accepted-consent retry**, for `repo enable` and for `repo create`: a
  failure injected after the initialization commit; advance the clock an
  hour; the same request completes without a new preview and without a
  second commit. The `repo create` retry goes through `enable`.
- **Retry before any effect observes again:** accept, fail before the first
  effect (a boundary fault point), dirty the worktree, retry:
  `external_change`.
- Confirmation and request record are written in one transaction: a failure
  injected between them leaves neither.
- **No lock:** with a preview outstanding, a save and an `index refresh` on
  the repository succeed.
- **Cache loss:** with the database deleted after the preview, the
  confirmation is `confirmation_not_found`.
- Identity: with one supplied, it is in the preview and in local
  configuration afterwards, and global configuration is byte-identical.
  With none supplied and a global one: `repo enable` names the global
  source and writes no local identity; `repo create` names the global
  source, says it will be written locally, and writes it. With none at all:
  `identity_required`, no directory, and no recovery action for
  `repo create`.
- A prefix is in the preview and in the committed configuration; without
  one the file is as before this change.
- `repo enable` on a wrong branch is `wrong_branch`. On an enabled
  repository `prepare` returns `noop` and no confirmation; with a prefix
  that differs from the stored one it is `invalid_input`.
- A failed registry write after the commit is `partial` with
  `registration_pending`; the retry registers the repository and reports
  the same commit.
- `repo create` into an existing empty directory, with a failure injected
  after the repository is initialized: `partial`, the record stays
  `accepted`, and the retry completes through `enable`.
- A request rejected after it accepted a confirmation (a wrong branch found
  under the lease) leaves no record and releases the confirmation: the
  same request, and a new one, can use it until it expires.
- Nothing left behind after each rejection.
- No preview or confirmation row contains a sentinel planted in a remote
  URL's password, a key passphrase or a body.

**Verify:** `devenv shell -- cargo test --locked --test mutation_confirmation --test mutation_contract --test repository_enablement`.

## Task 12: Confirmed Administration

**Files:** `src/repository/mutation/bind/repo.rs`, `bind/remote.rs`,
`bind/key.rs`, `tests/mutation_confirmation.rs`, schemas and goldens.

**Contract:** Bindings for `repo remove`, `remote remove` and `key delete`,
each with its preview and observation digest.

**Tests first:**

- `repo remove`: the preview names the registration; a retry of the same
  request returns the stored result although the registration is gone; with
  the result lost before settlement, the retry is a no-op and not
  `external_change`; the repository's files and Git state are
  byte-identical.
- `remote remove`: the preview says whether polling is affected. For the
  selected publication remote, `prepare` is `publication_remote_in_use`
  and no confirmation is issued. Selecting the remote between preview and
  confirmation is `external_change`. With a failure injected after the
  remote is deleted and before the journal is updated, the result is
  `partial`, the record stays `accepted`, and the retry completes without
  comparing the observation again.
- `repo remove` with a failure injected before its transaction: nothing
  left behind once the journal fix is in; the same request then succeeds.
- `key delete`: the preview names the key and its files; a registration
  changed between preview and confirmation is `external_change`; a key file
  changed is `external_change` from the domain's own review; an imported
  key is `key_not_deletable`; a selected key is `selected_key_in_use`.
- `key delete` with a failure injected after the private file is removed is
  `partial` with `recovery_required`, and the record stays `accepted`.
  The retry obtains a fresh review, skips the observation comparison and
  completes the deletion under the same operation ID.
- A retry after a completed deletion returns the stored result; with the
  result lost, it is a no-op, reached by calling the deletion with no
  review.
- Every effect is `not_requested` for all three.
- Changed input and replay for each.

**Verify:** the Task 11 command, with `--test key_material`.

## Task 13: Progress, Cancellation And Synchronization

**Files:** `src/repository/mutation/progress.rs`, `mod.rs`, `bind/sync.rs`,
`src/repository/remote/sync.rs`, `tests/mutation_remote.rs`,
`tests/support/remote_world.rs`, `tests/remote_synchronization.rs`,
`Cargo.toml`, the `progress_event` and `cancel` schemas and goldens.

**Contract:** The design's Progress And Cancellation section. `ProgressSink`,
`ProgressEvent`, `CancellationToken`, `cancel_request`. A sibling of
`synchronize_remote` that calls an observer at each safe point and accepts a
cancellation request from it; the existing function delegates. Bindings for
`item sync` and `repo sync`. Move the `World` fixture to shared support
without changing it; `remote_synchronization` must pass unchanged.

**Tests first**, on the SSH harness:

- A clean synchronization returns `published` with the commit; the server
  holds it.
- **Lost output:** the push is accepted and the result is lost (the
  fixture's disconnect after receive-pack). The same request reconciles:
  `published`, the same commit, and the server's accepted-push count is one.
- **Changed input:** the same request ID for a different target is
  `request_mismatch`; the server's connection count is unchanged.
- **Cache loss:** the database is deleted after a completed synchronization.
  Before the repository is registered, the key selected and the host
  approved again, the retry is blocked with the code for each missing
  piece in turn. After they are restored it reports `current` and pushes
  nothing.
- No publication remote: `blocked` with `publication_remote_required`.
- A divergent remote: written before Cycle 06 as `merge_required`. The
  call now merges; the expected results are set by decision 12 before this
  task starts. A remotely deleted context branch
  is `remote_branch_deleted`; a rejected push is `push_rejected`.
- **Open case 3, reproduced or refuted.** After each of those three, and
  after `host_approval_required` and `unlock_required`: is the
  reservation still active; does a save on the repository return
  `recovery_required`; does another synchronization return `busy`; and
  does the same request again, or `cancel_request` followed by the same
  request, release it? Pin what happens. Each result carries a
  `request.retry` action. If the reservation is held, record it in the
  ledger, raise it as a ticket and do not work around it.
- A synchronization reported as interrupted is `cancelled` only when its
  row's phase is cancelled.
- An unknown host is `host_approval_required` with the authority and the
  presented fingerprint in `data` and a `host.approve` action. A rotated
  host key is `host_replacement_required` with both fingerprints.
- A locked key with no passphrase available is `unlock_required`. A client
  the server rejects is `key_rejected`. A stalled connection is
  `remote_unavailable` or `transport_unavailable`; pin which.
- A background poll holding the reservation: `poll_yielding`, retryable, no
  record left.
- A published commit followed by a failed index hand-off is `partial` with
  `publication: published` and `discovery_pending`; a retry finishes it
  without a second push.
- **Progress:** the events for a synchronization arrive in safe-point order
  and end before the result; none contains a sentinel planted in a commit
  message, the remote URL or the passphrase.
- **Cancellation by token:** set before `execute`: `cancelled`, no record,
  no connection. Set from the sink at the first safe point: `cancelled`
  there, nothing pushed.
- **Cancellation by request:** with the server holding the advertisement,
  `cancel_request` from a second thread, then release: `cancelled`, nothing
  pushed. Requested before the reservation exists (the execution paused by
  a boundary test hook after acceptance): honored at the first safe point.
- `cancel_request` for a save whose journal row is pending changes nothing:
  the record stays `accepted` and its retry completes.
- After a cancelled synchronization the same request returns the stored
  `cancelled` result, and a new request synchronizes.
- Cancellation requested once the outcome is classified returns
  `published`.
- `cancel_request` while the transport call is blocked reports requested
  and not acknowledged.
- A retry of an unfinished synchronization while the first is still running
  takes it over; the server's accepted-push count is one.
- The output-privacy scan of the harness passes.

**Verify:** `devenv shell -- cargo test --locked --test mutation_remote --test remote_synchronization --test mutation_contract`.

## Task 14: Resume

**Files:** `src/repository/mutation/mod.rs`, `tests/mutation_replay.rs`,
`tests/mutation_remote.rs`, goldens.

**Contract:** The design's Resuming section.

**Tests first:**

- For a save left with discovery pending, resume runs the hand-off and
  reports `current`; the original request then returns a finished result.
- For a save interrupted before its write, resume is
  `original_request_required` with a `request.retry` action naming the
  original request, and the journal row is untouched: the write has not
  been marked done.
- For an interrupted synchronization, resume reconciles without a second
  push.
- For a key generation, `original_request_required`.
- An unknown operation ID is `operation_not_found`.
- A resume is replayable under its own request ID; the same ID for a
  different operation is `request_mismatch`.

**Verify:** the Task 13 command, with `--test mutation_replay`.

## Part B Checkpoint

Full gate. Independent review of the Part B range with the Review Focus
list. Record results and rulings. Every existing mutation the design binds
is now reachable through the boundary; the bridges are not.

# Part C: Bridges

## Task 15: Local Identity

**Files:** `src/repository/bridges/identity.rs`,
`src/repository/mutation/bind/repo.rs`, `src/repository/read/admin.rs`,
`read/dto.rs`, `tests/mutation_bridges.rs`, the `identity` schema and
goldens.

**Contract:** The design's Local identity section. `set_local_identity`;
the `repo identity-set` binding with its preview; `IdentityDto.initials`
and `initials_source`.

**Tests first:**

- Set name and email on a repository with none: local configuration holds
  them; global, XDG and system configuration files are byte-identical.
- Setting equal values is `noop`.
- Initials: set, cleared with null, left unchanged when absent from the
  input; an invalid value is `invalid_input`.
- With a failure injected after the first key is written, the configuration
  file is restored byte for byte and the result is an error with no effect.
- The preview shows the new values, the current local values and the
  effective identity's source. A local value changed between preview and
  confirmation is `external_change`.
- Works on a repository that is not enabled; a bare repository is
  `bare_repository` and a non-repository `not_repository`.
- An empty name, a NUL and an email without `@` are `invalid_input`.
- `repository_identity` reports initials from local configuration with
  source `repository`, derived initials with source `derived`, and null
  with source `none`. The F1 `repo_identity` golden gains the two fields.
- Replay returns the stored result; changed input is `request_mismatch`.
- After `identity_required` on a save, `repo identity-set`, then a new save
  succeeds.
- With the result lost after the identity was written, the retry is a no-op
  and not `external_change`.

**Verify:** `devenv shell -- cargo test --locked --test mutation_bridges --test mutation_contract --test read_repository --test read_contract`.

## Task 16: Host Observation And Approval

**Files:** `src/repository/mutation/mod.rs`, `bind/host.rs`,
`tests/mutation_remote.rs`, the `host_observation` schema and goldens.

**Contract:** The design's Host approval section: `observe_host` and the
`host approve` and `host replace` bindings. No code in this task writes a
pin.

**Tests first**, on the SSH harness:

- `observe_host` on an unknown host returns the authority and the presented
  fingerprint and stores no pin; on a pinned host it reports the pin; with
  no selected key it is `no_selected_key`.
- Approve with the observed fingerprint: the pin is stored; the server's
  accepted-push count is zero; every local ref, including tracking refs, is
  byte-identical before and after.
- `prepare` opens no connection: the server's connection count is unchanged.
- Approve with a fingerprint the host does not present: `host_mismatch`
  with the presented fingerprint in `data`, no pin.
- Replace after a rotated host key, with the old and new fingerprints: the
  pin changes. With a wrong old fingerprint, `prepare` is `invalid_input`.
- An authority that is not the publication remote's is `invalid_input`. With
  different fetch and push hosts, each is approvable and is verified in its
  own direction.
- The pin changed by another actor between preview and confirmation is
  `external_change`, before any connection.
- A locked key with no passphrase is `unlock_required`, no pin.
- Approving an already-pinned equal key is `noop`.
- With the reapproval marker present, an approval stores the pin and the
  marker file is still present afterwards.
- A connection dropped after authentication and before the advertisement:
  pin the result, and prove a retry of the same request ends with exactly
  one pin.
- Replay of a finished request returns the stored result without
  connecting; changed input is `request_mismatch`. With the result lost
  after the pin was written, the retry is a no-op.

**Verify:** `devenv shell -- cargo test --locked --test mutation_remote --test ssh_transport`.

## Task 17: Folder Creation And Listing

**Files:** `src/repository/bridges/folder.rs`,
`src/repository/mutation/bind/folder.rs`, `src/repository/discovery.rs`,
`src/repository.rs`, `src/repository/read/items.rs`, `read/dto.rs`,
`tests/mutation_bridges.rs`, `tests/read_items.rs`, schemas and goldens.

**Contract:** The design's Folder creation section, with listing as decision
4 recommends: `create_folder`; the `discovered_folders` table written by
refresh and rebuild; `list_document_folders`.

**Tests first:**

- Create `docs/a/b` where neither exists: both are directories; `git status`
  is clean; no commit or branch was added, and the only journal row is the
  refresh's.
- An existing directory is `noop`; an existing file is `occupied_path`.
- A path outside `docs/`, an absolute path, a path with `..` and an empty
  segment are `invalid_input`.
- A path seventeen levels deep, and a folder that would be the 1,025th
  entry under `docs/`, are `invalid_input`; nothing is created, and a save
  in the repository still works afterwards.
- With `docs/link` a symbolic link to a directory outside the repository,
  creating `docs/link/x` is `invalid_path` and nothing is created outside.
  Unix only; say so in the test.
- After creation the result reports `discovery: current`, and
  `list_document_folders` includes the folder as empty. A folder holding a
  document is listed as not empty.
- A failed refresh is `partial` with `discovery_pending` and the
  `index.refresh` action; the directory exists; after a refresh it is
  listed.
- A read never lists a directory: with the index stale, the folder list
  returns what the index has and says it is stale.
- `complete` is false when discovery met its directory cap.
- The migration adds the table to an existing database and marks
  repositories stale, as F1's migrations do.
- F1's no-side-effect snapshot tests include the new read.
- Replay returns the stored result; changed input is `request_mismatch`.

**Verify:** `devenv shell -- cargo test --locked --test mutation_bridges --test read_items --test read_boundary --test discovery_rebuild`.

## Task 18: Repair And Adoption

**Files:** `src/repository/bridges/repair.rs`,
`src/repository/mutation/bind/repair.rs`, `src/repository.rs`,
`src/repository/recovery.rs`, `src/repository/read/dto.rs`,
`tests/mutation_bridges.rs`, `tests/mutation_confirmation.rs`, schemas and
goldens.

**Contract:** The design's Repair and adoption section.
`ContextIntent::Repair`, with the crate's exhaustive matches on the intent
extended; `repair_item`; the `repair_item` journal action in `action_name`,
`operation_from_name` and the published `OperationAction` list; the
`document repair` and `ticket repair` bindings.

**Tests first:**

- **Adoption:** a committed marker-only document on primary, repaired with a
  source carrying a new ID. The preview states the adoption and the ID.
  After confirmation the item's context holds the conforming file, one
  checkpoint commit exists, and primary is unchanged.
- **Stable ID:** a failure injected after the write, then a retry; and the
  database deleted, the repository rebuilt, then a new preview and request
  with the same source. The ID is the same in each and one context exists.
- **Stale preview:** the file changed and committed on primary between
  preview and confirmation is `external_change`. A stale token at `prepare`
  is `external_change`.
- Changed input: the same request ID with a different source is
  `request_mismatch`; a different source under the same confirmation is
  `confirmation_mismatch`.
- A repaired document is listed twice, the nonconforming entry from primary
  and the item from its context; `show_item` by the new ID returns the
  context's copy. A repaired ticket is listed once.
- **Ambiguous identity:** a source whose ID another item holds, and a path
  under a duplicate-ID problem, are `identity_ambiguous`; no file, context,
  branch, journal row or request record.
- A current file with a valid ID and a source with a different one is
  `not_repairable`.
- A source that is itself nonconforming, or of the wrong kind for the path,
  is `invalid_input`.
- An uncommitted nonconforming file is `worktree_not_clean`.
- Ticket: marker-only `ticket.md` in a ULID-named directory is repaired with
  that ID; with a different ID it is `not_repairable`; in a directory that
  is not a ULID it is `not_repairable`.
- Closure: a lifecycle-closed ticket with a broken field is repaired keeping
  its closure fields; a source that drops them, or adds them to an open
  ticket, is `invalid_input`. Unparseable front matter with closure fields
  in the source is `invalid_input`.
- A managed file outside its canonical path is `not_repairable`.
- `list_operations` shows a pending repair with the `repair_item` action;
  the operation schema's enumeration includes it.
- Refresh never adopts: after a refresh and a rebuild the marker-only file
  is byte-identical.
- A failure after the checkpoint is `partial`.

**Verify:** `devenv shell -- cargo test --locked --test mutation_bridges --test mutation_confirmation --test mutation_contract --test local_authoring --test read_status`.

## Task 19: Verify, Review And Handoff

**Files:** `.github/workflows/build.yml`; the execution ledger; the ticket.

1. Add `mutation_replay`, `mutation_confirmation`, `mutation_contract`,
   `mutation_bridges`, `mutation_relationships` and `mutation_remote` to the
   native workflow's test list. Dispatch nothing.
2. Contract completeness, enforced by tests in `mutation_contract`: every
   bound command has a golden; every schema is reachable; every schema
   enumeration is tied to a registry; the fixture directory equals the
   registered case list; every result code either has a golden or is in an
   explicit list of codes no fixture can produce, each with its reason. The
   reviewer reads that list.
3. Run the full gate through Devenv and record each command's exit status
   and counts:

   ```sh
   devenv shell -- cargo check --all-features --locked
   devenv shell -- cargo fmt --check
   devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
   devenv shell -- cargo test --all-features --locked --no-fail-fast
   devenv shell -- cargo run --locked --bin manyhands-cli
   devenv shell -- cargo test --locked --test mutation_replay --test mutation_confirmation --test mutation_contract --test mutation_bridges --test mutation_relationships --test mutation_remote
   ```

   The last command runs without the `desktop` feature.
4. Whole-Cycle review by independent reviewers, one lens each: replay and
   confirmation safety; privacy and redaction; the contract against the
   RFCs and this Cycle; changes to existing code. Fix findings; have the
   fixes read.
5. Fill the Cycle's acceptance table with the test for each row.
6. Record open obligations, the three gaps the Cycle document lists for
   later Cycles, and any defect found and not fixed.
7. Report review-ready. Pull request, merge, ticket closure and worktree
   cleanup each need their own authorization.

## Plan Self-Review

- **Coverage.** Every item of the Wave's F2 scope has a task: request IDs
  and matching (6, 7), observations (7, 8), records (6), reconciliation
  (7, 13), confirmation and absent destinations (11, 12), progress and
  cancellation (13), recovery actions (5, 14), identity (11, 15), host
  approval (16), folders (17), repair (18), relationships and short codes
  (1–3, 7, 8), comment author (4).
- **Exit evidence.** Each phrase of the Wave's F2 exit evidence maps to a
  named test: stale preview (11, 18), changed input (7, 13), accepted-consent
  retry (11), lost output (7, 13), cache loss (7, 10, 11, 13, 18), no
  duplicates (7), safe-point cancellation (13), bridge evidence (15–18),
  host approval publishes nothing (16), ambiguous identity (18), vectors
  and invariance (1, 2), no body or secret (6 onward).
- **Track rule 7.** Every mutating task names a failure or interruption case
  and a replay case. Tasks 15 and 17, and the key registry commands of
  Task 10, bind operations with no journal; their interruption case is an
  injected failure and their replay case is the stored result.
- **Known thin spots.** Local operations offer one cancellation point. An
  operation killed after its journal row is written needs its original
  input. Whether a refused synchronization holds its reservation is
  unconfirmed. The replay mechanism has failed review three times and its
  latest corrections are unreviewed; the checkpoint after Task 7 exists for
  that reason. Native
  behavior is unproven. Each is stated in the design.
- **Not verified during planning.** No Rust command was run. The design's
  audit is from two readings of the source and two of the mechanisms. The
  refresh of 2026-10-10 for the journal fix and Cycle 06 is unreviewed, and
  the synchronization tasks were not re-planned against Cycle 06; decision
  12 comes first.
