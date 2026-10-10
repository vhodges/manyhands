---
title: "Wave 03 F2 Request Replay, Confirmation And Shared Mutation Bridges Design"
date: 2026-10-08
status: approved
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M4ERD7MTJWPTD8QDVXQ5ZQ72"
---

# Wave 03 F2 Request Replay, Confirmation And Shared Mutation Bridges Design

## Intent And Approval Boundary

F2 gives both front ends one headless way to change Manyhands state. A front
end hands the library a request ID and a typed mutation. The library decides
whether this is new work, a retry or a conflicting reuse, checks consent where
the action needs it, calls the existing domain operation, and returns one
result envelope that says what was done and what is still owed.

F2 also adds the few mutations the baseline lacks: local identity
configuration, host approval, folder creation, repair and adoption, ticket
relationship writes, short codes and the comment author.

This design accompanies the
[Cycle document](../Cycles/wave-03-foundation-02-replay-confirmation-and-bridges.md)
and the
[implementation plan](2026-10-08-wave-03-foundation-02-replay-confirmation-and-bridges-implementation.md).
The product owner approved it on 2026-10-10, after three revisions, four
independent reviews of the first drafts, and a fifth of the refresh for
what landed on main. The corrections made after the fifth review, and the
section on abandoning an operation, were approved without a further
review. It authorizes no Rust change.

**How settled this is.** Every review round found blocking errors in how a
retried request is settled, including in the corrections made for the round
before. The rules under [Request Identity And Records](#request-identity-and-records)
are the fourth version and have not been reviewed. What is firm is the
table of [required outcomes](#required-outcomes): what must be true after
each sequence of events, whatever mechanism produces it. The plan builds
that part test-first against the table and stops for a design checkpoint
before any other binding is written.

Terms used below:

- **Mutation boundary**: the new library entry points, `prepare` and
  `execute`, that every front-end mutation goes through.
- **Binding**: the code that connects one command, such as `ticket save`, to
  the boundary: its input type, its identity, its consent rule and the domain
  call it makes.
- **Domain operation**: an existing library function such as `save_ticket` or
  `synchronize_remote`, with its own operation ID, journal row and lease.
- **Request record**: the row that links a request ID to its command, target,
  input digest, domain operation and completed effects.
- **Durable effect**: a canonical file written, a commit, a ref moved, a push,
  a repository initialized, a configuration or registry change. Creating an
  editing context (a branch and worktree) is not counted: it is reused by
  the next attempt and changes no content.

## Mutation-Surface Audit At `6cf5d7f`

Read from source on 2026-10-08, by one reader and then checked by a second;
no Rust command was run and nothing here was reproduced by a test. This
refreshes entry-gate item 3 for what F2 consumes. On 2026-10-10 the rows
for the journal, the operation lookup and synchronization were checked
again at `f87ce81`, after Wave 02 Cycle 06, the index fix
`01M4GD0KKXW684QBA49F6EX3WE` and the journal fix landed; the other rows and
every line anchor in this document are still as read at `6cf5d7f`. The
provisional
[API audit](../research/wave-03-api-audit.md) predates F1 and Wave 02 Cycle
05, and its line anchors no longer hold.

| Area | What exists | What F2 must add |
| --- | --- | --- |
| Operation identity | `OperationId`, a ULID the caller generates. Three journals in the one SQLite file: `operation_records` (local lifecycle), `remote_operation_records` (reservations and synchronization) and `key_material_operations`. Key registry and polling-policy changes have no operation ID. | A request ID and its mapping to operation IDs. Nothing stores a request ID, an input digest or a result today. |
| Replay | Repeating a call with the same operation ID makes the function reconcile from Git, the file system and its journal row. The journal matches root, action and a target string. | Input matching. A save's target string covers the item and paths, not the title, body or metadata, so a reused operation ID with a different body is not detected. |
| Journal exclusion | `begin_or_reconcile` refuses a new operation with `RecoveryRequired` while any other row for the repository is not completed. | Nothing. See [The Journal Defect](#the-journal-defect): ordinary rejections left such a row until the fix of 2026-10-10. |
| Expected observations | `ExpectedPathObservation`, `Missing` or the BLAKE3 of the file bytes, on the three save requests. `expected_source` on a document save is optional. | The F1 token (`v1:` plus a digest of branch, path and bytes) is a different digest and nothing converts between them. No token exists for an absent path or for repository-level state. |
| Confirmation | Key deletion only: an in-memory review value plus a `confirmed` flag. `HostApproval` is an exact-match value the caller builds from a prior error. Neither is persisted. | Persisted previews with expiry, for every action the CLI RFC says needs one. |
| Cancellation | Remote operations only: `cancel_remote_operation` sets a durable flag on an existing reservation. Synchronization honors it at seven of the fourteen named safe points; the others belong to polling. | A way for a caller to ask, including before a reservation exists, and a safe point for local operations. |
| Progress | None. The transport installs credential, certificate and push-reference callbacks only. The safe-point hook exists under `cfg(test)`. | A typed event and a sink. |
| Credentials | `SessionCredentials<P>` is passed into each network call; a protected key is unlocked through the caller's provider inside the call. A new key's passphrase travels in the generation request. | Nothing; the boundary passes them through. |
| Results | `Envelope` has constructors for reads only, and a code determines its outcome. `ReadError` maps every mutation-only error kind to `internal_error`. Seventeen result codes and three recovery actions are registered. | Mutation envelopes, codes for every mutation outcome, and the mutation recovery actions. |
| Identity | Written only inside `enable`, whenever the request supplies one. `create_and_enable` without a supplied identity returns `IdentityRequired` before any effect and never reads global configuration. | A standalone setter; initials. |
| Host trust | Pins are written only by `finalize_host_trust`, after an authenticated connection, from `verify_ssh_transport`, a poll or a sync. `verify_ssh_transport` connects with the selected key, checks the host, lists the advertisement, fetches nothing and updates no ref. A publication remote has a fetch endpoint and a push endpoint, which may differ. | A request-bound approve and replace over `verify_ssh_transport`, and a way to learn the fingerprint a host presents. |
| Folders | No folder concept. Discovery collects `.md` files; an empty directory is not indexed. Directories are created as a side effect of a document save through a no-follow walk. Discovery fails a whole context past 16 levels or 1,024 entries under `docs/`. | Folder creation and, if approved, folder listing. |
| Repair | None. A marker-only file has no ID, so no authoring context can be named for it, and `prepare_context` with `Edit` requires the ID to exist on primary. | A repair operation and context intent. |
| Relationships | `slug`, `parent` and `deps` live in a ticket's unknown metadata and are read as a view. The serializer emits unknown keys first, then the defined ones. `TicketGraph` detects cycles from index rows and does no I/O. No code generates a short code or reads initials or a prefix. | Writers, the canonical form, the write-time cycle check, generation and assignment. |
| Comment author | `created_by` is read from unknown metadata on the read side. `submit_comment` treats an existing comment file with any unknown metadata as occupied. | A real field, written at creation. |

Three facts shape the design more than the rest:

1. **The index database is a cache, and it holds more than the index.** A
   corrupt file is renamed and recreated. With it go the repository
   registrations, all three journals, the shared-key registry and selection,
   and the host pins. Whatever F2 stores there can vanish, and after a loss
   the repository is not even registered.
2. **No lock survives between two calls.** The repository lease has a 250 ms
   wait and is held only inside one domain call. A preview and its
   confirmation are two calls, so a confirmation can bind what was observed
   and can never reserve it.
3. **An existing test bans content evidence from `operation_records`.**
   `assert_operation_records_hold_no_content` fails if that table's schema
   contains "digest", "oid", "content" and similar words. Request records
   therefore live in their own tables. The index RFC provides for them: "CLI
   mutations persist non-secret request identity, semantic-input digest where
   appropriate, target observations, operation linkage and completed
   effects."

## The Journal Defect

This was a defect in the existing library, found during this planning. It
was reproduced and fixed on ticket `01M4EWN2DK3MY6H4GBYDYXF6QH`, merged to
main on 2026-10-10 (`tests/journal_rejection.rs`). This section records the
defect, then what the fix gives F2.

A local operation writes its journal row before most of its checks, and
several returned on an ordinary rejection without completing that row. The
next operation with a different ID on the same repository was then refused
with `RecoveryRequired`. Paths that left a row:

- `enable`: an invalid or wrong branch, a dirty or conflicted worktree, an
  invalid configuration, an unavailable remote, an invalid identity, and
  every rollback path. Only "identity required" and "already enabled"
  complete the row.
- `create_and_enable`: a target that is not a directory or is not empty.
- `add_remote`, `remove_remote`, `set_publication_remote`: any Git error,
  including an invalid name.
- The three saves and `submit_comment`: a repository that is not enabled, an
  invalid configuration, a wrong checked-out branch, a dirty configuration
  path, any Git or I/O error, and any rejection at all once the editing
  context has been created.

`remove_registration` is blocked by the same check, and a refresh can clear a
row only for a registered repository.

**What the fix does.** When one of the operations above returns an error,
the journal row was begun by this call, and this call wrote nothing durable
or restored everything it wrote, the row is closed. Each call tracks what
it wrote; the error kind is not consulted. A row stays pending when an
effect was made and work remains.

**What F2 must know about it:**

- A closed row is kept, not deleted. Its state is `completed` and its step
  is `rejected`, so the operation ID stays bound to its action and target.
  The public lookup `show_operation` reads it that way, with no next
  action. Wherever this design asks whether a journal row exists for an
  operation ID, a `rejected` row counts as none.
- Repeating a rejected operation ID with the same action and target begins
  again as a new call. With another target it is `OperationMismatch`. A
  record keeps its operation ID for as long as it is `accepted`, so a
  re-entry can meet a rejected row under its own ID; a new acceptance gets
  a new ID, so the mismatch never reaches a caller.
- A failed `enable` or `set_publication_remote` that restored everything
  now closes its row too.
- The call does not report what it wrote to its caller. The boundary still
  reads the journal row.
- Not covered: refresh and rebuild, whose failed rows stay pending by
  existing tests, so a failed `index refresh` or `index rebuild` still
  blocks other requests until the same request is retried. A row also
  stays pending, as before, after a write that was not undone, a failed
  restore, an initialized repository and a half-created editing context.
- `remove_registration` was examined and no rejection that leaves its row
  could be provoked.
- A pending row after an error does not always mean something was written.
  The call marks its effect before the step that writes, so these leave a
  pending row with nothing written: a Git failure inside the first write
  call (a stale lock file); a save whose error comes from the checkpoint
  step before the commit, including when the file was already as intended;
  a failure injected immediately before a write; and any rejection of a
  retry on a row that an earlier attempt began, because only a row begun
  by the returning call is closed. The boundary then reports work in
  flight where there is none; the retry clears it once the cause is gone.
- On a row resumed
  from an earlier attempt the older rule still applies: an attempt that
  wrote the file and failed to record the step, followed by a retry that is
  cleanly rejected, closes the row with the file written and uncommitted.
  The boundary then deletes the record, and the next request for that item
  is answered by the cache-loss rule: `external_change`, then a fresh save
  commits it.

Three cases remain open after the fix. All exist today.

1. A row is pending from the moment it is written. A process killed at any
   point after that leaves it, and only a repeat of the same request with
   the same input clears it. For a save that means the same body. If the
   caller no longer has it, or someone else has since committed different
   content to the same file, nothing in the library can clear the row, and
   no command in the CLI RFC abandons an operation.
2. A process killed after `enable` or `remote select` wrote the
   configuration file and before it committed leaves the worktree dirty;
   the replay is then refused as a dirty worktree and authoring is blocked.
3. A synchronization that stops with an error after it reserved (a
   divergent remote, a rejected push, a host that needs approval, a locked
   key, a transport failure) appears to leave its reservation active. While
   it is active, local operations are refused and other synchronizations
   are busy. The way out is the same request again, or a cancellation
   followed by the same request. This was read from the code and not
   reproduced, and not re-read after Cycle 06 changed synchronization;
   Task 13 reproduces it, and it is raised as a ticket if it holds.
4. Cycle 06 adds a stop of a different kind: a pending merge conflict. It
   releases the reservation, but until it is resolved every new
   synchronization of the repository returns busy and every save of the
   conflicted item returns `RecoveryRequired`; other items can still be
   saved. Cancelling does not clear it. The only exits are
   `resolve_synchronization` and a hand-made merge commit (ticket
   `01M4H33R34Z7C7EEKTY1ZCT950`, on its own unmerged branch). F2's
   bindings refuse divergence and so never enter this state (decision 12).

Decision 2 covers these cases.

## Module Layout

```text
src/results.rs                      + mutation constructors, codes, actions
src/canonical.rs                    + relationship writers, short codes,
                                      initials, comment `created_by`
src/repository/mutation/
    mod.rs        public surface: prepare, execute, cancel_request,
                  resume_operation, show_request, observe_host;
                  MutationCall, CancellationToken, ProgressSink
    identity.rs   RequestId, ConfirmationId, the intent digest
    records.rs    the three request tables: DDL and access
    confirm.rs    preview creation, acceptance, expiry
    observe.rs    observation tokens: checking, the absent token,
                  repository-level observation digests
    outcome.rs    domain outcomes and errors to envelope, effects and codes
    evidence.rs   what an earlier attempt of a request left in Git
    progress.rs   ProgressEvent, ProgressStage
    dto.rs        PreviewDto and the mutation data DTOs
    bind/         one file per command family: repo, remote, key, host,
                  folder, document, ticket, repair, sync, index
src/repository/bridges/
    identity.rs   set_local_identity
    folder.rs     create_folder
    repair.rs     repair_item and the Repair context intent
src/repository/{recovery,remote/state,keys/registry}.rs
                                    + a lookup of one operation by ID,
                                      unless `show_operation` serves
src/repository/read/graph.rs        visibility widened for the cycle check
schemas/v1/                         new and extended schemas
tests/fixtures/mutation_v1/         golden envelopes and previews
```

Everything except the items under
[Changes To Existing Code](#changes-to-existing-code) is a new file. This is
not a refactor of `src/repository.rs`.

## The Mutation Boundary

```rust
pub struct MutationCall {
    pub request_id: RequestId,
    pub confirmation: Option<ConfirmationId>,
    pub mutation: Mutation,
}

impl RepositoryService {
    pub fn prepare(&self, mutation: &Mutation) -> Envelope<PreviewDto>;

    pub fn execute<P: SessionCredentialProvider>(
        &self,
        call: MutationCall,
        credentials: &mut SessionCredentials<P>,
        progress: &mut dyn ProgressSink,
        cancel: &CancellationToken,
    ) -> Envelope<MutationDataDto>;

    pub fn show_request(&self, request_id: RequestId) -> Envelope<RequestDto>;
    pub fn cancel_request(&self, request_id: RequestId) -> Envelope<CancelDto>;
    pub fn resume_operation<P: SessionCredentialProvider>(/* request ID,
        operation ID, credentials, progress, cancel */)
        -> Envelope<MutationDataDto>;
    pub fn observe_host<P: SessionCredentialProvider>(/* repository,
        credentials */) -> Envelope<HostObservationDto>;
}
```

`Mutation` is a closed enum with one variant per bound command, each holding
that command's typed input. The variant fixes the envelope's `command`
string; a preview carries the same command as the execution it previews.
`show_request` is `request show`, `cancel_request` is `request cancel`,
`resume_operation` is `operation resume` and `observe_host` is
`host inspect`. Every handled outcome, success or not, comes back as an
envelope; these functions do not return `Result`. `execute` is synchronous
and runs on the caller's thread. A front end that wants it off its interface
thread runs it on a worker of its own; F2 adds no thread, executor or event
framework.

The library requires a request ID on every `execute`. Which caller invents it
is a front-end matter. `prepare` takes none: it changes no canonical or Git
state, and the confirmation ID it returns is how a caller refers to it.

`show_request` returns what the records hold for a request ID: its state,
its stored result if finished, and its operation IDs. The CLI RFC tells a
client that lost its output to "use their request ID to inspect recovery";
this is that read.

### Commands bound in F2

Track rule 5 forbids a front-end branch from changing `src/repository*` after
F2. So F2 binds the baseline mutations that C2, C3 and D1 to D3 consume, and
the two synchronization commands its exit evidence requires. A command not
listed reaches the library later as a shared-library change, with the owner
named below.

| Command | Domain call | Confirmation | Notes |
| --- | --- | --- | --- |
| `repo create` | `create_and_enable`; `enable` on a retry | Required | Identity, prefix; absent-root binding. |
| `repo enable` | `enable` | Required | Identity, prefix. |
| `repo remove` | `remove_registration` | Required | |
| `repo identity-set` | new `set_local_identity` | Required | Name, email, optional initials. |
| `remote add` | `add_remote` | No | |
| `remote remove` | `remove_remote` | Required | Preview shows publication and polling effects. |
| `remote select` | `set_publication_remote` | No | |
| `key generate` | `generate_shared_key` | No | Digest covers label and protection mode, never the passphrase. |
| `key import` | `register_shared_key` | No | |
| `key select`, `key clear` | `select_shared_key`, `clear_shared_key_selection` | No | |
| `key remove` | `unregister_shared_key` | No | |
| `key delete` | `review_generated_key_deletion`, `delete_generated_key` | Required | The persisted preview wraps the existing review. |
| `host approve`, `host replace` | `verify_ssh_transport` | Required | |
| `folder create` | new `create_folder` | No | |
| `document create`, `document save`, `document move` | `save_document` | No | Source observation required for save and move. |
| `document repair`, `ticket repair` | new `repair_item` | Required | |
| `ticket create`, `ticket save` | `save_ticket_with` | No | Relationships; short code on create. |
| `ticket slug-assign` | `save_ticket_with` | No | |
| `item sync`, `repo sync` | the new sibling of `synchronize_remote` | No | Clean synchronization only: the bindings set the sibling's refuse-divergence setting, so a divergent remote is `merge_required` and nothing is merged (decision 12). |
| `operation abandon` | new `abandon_operation` | Required | Decision 13; see [Abandoning An Operation](#abandoning-an-operation). |
| `index refresh`, `index rebuild` | `refresh_repository`, `rebuild_repository` | No | |

That is twenty-eight baseline commands and `operation abandon`. The saves prepare their editing context
themselves, under the same operation ID; a binding makes one domain call.

Not bound in F2, with the owner:

- `comment add`: the first of C4 or D4. The CLI RFC defines it as one
  checkpoint followed by immediate publication, and that compound operation
  is Wave 02 Cycle 07. Binding the local half now would publish a result
  shape that changes when the other half arrives. No Cycle before C4 and D4
  submits comments. F2 still makes `submit_comment` write the author.
- `poll configure`, `poll pause`, `poll resume`, `poll once`: the first of C5
  or D6.
- `document promote`, `ticket close`: the first of C5 or D5; track rule 6.
- `conflict resolve`, the conflict reads and merging a divergent remote:
  C4 and D5, which turn off the refuse-divergence setting when they bind
  them.
- Republishing a remotely deleted item branch: the Wave gives C4 "exact
  republish consent for a remotely deleted branch", but no operation
  republishes one today and no Wave 02 Cycle is named for it.
  Synchronization returns a typed result and stops. This is a gap for C4's
  planning, recorded in the Cycle document.

## Request Identity And Records

### The intent digest

A request's identity is its command, its repository, its target and its
semantic input. F2 reduces these to one digest:

```text
BLAKE3( "manyhands request v1" , salt , command , repository ,
        target , field_1 , ... , field_n )
```

Every part is length-prefixed, and each field carries its name. Fields are
the binding's typed inputs in a fixed order, with absent, null and empty
kept distinct. An observation token the caller supplies is a field like any
other.

- `salt` is the request ID in a request record and the confirmation ID in a
  confirmation record. Two records never share a digest for equal input, so
  the tables do not reveal that two saves, or two repairs, had the same
  content.
- `repository` is the canonical root path, or the literal `application` for
  key commands. For `repo create` it is the canonical parent joined with the
  final name, which is the same string the canonical root has once the
  directory exists, so a retry after creation matches.
- A Markdown body or source is fed into the digest and stored nowhere.
- A passphrase is never a field. `key generate` carries the new passphrase in
  its input, as the domain request does, and contributes only its label and
  whether protection was asked for.

A digest of a short body can be guessed by someone who holds the database
and tries candidates. The CLI RFC requires "a content digest sufficient to
reject altered replay", and this is that; see [Risks](#risks).

### Records

Three tables are added to the existing database by the existing migration
function. None is a journal: they hold no step, no lease and no authority
over Git. Losing them loses the ability to answer a retry from memory, never
the ability to work.

```text
request_records
    request_ulid      TEXT PRIMARY KEY
    attempt           INTEGER NOT NULL -- raised by each call that enters
    scope_key         TEXT NOT NULL   -- canonical root or 'application'
    command           TEXT NOT NULL
    target            TEXT NOT NULL
    intent_digest     TEXT NOT NULL
    confirmation_ulid TEXT            -- the accepted confirmation, if any
    base_ref, base_oid                -- the branch the request will commit
                                      -- to and where it stood at acceptance
    expected_digest   TEXT            -- the byte digest passed to the
                                      -- domain as its expectation
    cancel_requested  INTEGER NOT NULL DEFAULT 0
    state             TEXT NOT NULL   -- 'accepted' or 'finished'
    outcome, code, the six effects, commit_oid,
    result_data       TEXT            -- set when finished
    accepted_at, finished_at

request_operations
    request_ulid, ordinal, family, operation_ulid

confirmation_records
    confirmation_ulid TEXT PRIMARY KEY
    scope_key, command, target, intent_digest, observation_digest
    created_at, expires_at
    accepted_by_request TEXT, accepted_at
```

`scope_key` is a path string, not a foreign key to `repositories`. A request
to create a repository exists before any registration does, and a request to
remove a registration must outlive it. `remove_registration` deletes only
its own tables today and needs no change.

`result_data` holds the finished envelope's `data` as JSON: identifiers,
paths, fingerprints, states and, for the identity commands, the name and
email that were set. No binding's data carries a body or source.
`expected_digest` is the digest of a canonical file's bytes as they were
before the request; the index RFC's "target observations".

**Access.** The boundary reads and writes these tables in short transactions
under the existing cache guard, and releases the guard before it runs a
binding, because a domain call takes the same guard itself. If the database
is unavailable, `execute` returns `index_unavailable`, with one exception:
`index rebuild` runs without a record, since rebuilding is how the database
comes back.

**Retention.** Nothing prunes these tables in F2; see decision 8.

### Required outcomes

This table is the contract for replay. The mechanism described after it is
the intended way to meet it. Where the two disagree, the table is right and
the mechanism is changed.

| Sequence of events | Required outcome |
| --- | --- |
| A request finishes; the same request is sent again, at any later time, whatever has happened to the item since. | The first result, unchanged. Nothing is written. |
| A request is sent again with different input. | Rejected. Nothing runs. |
| A save commits; the process dies before the result is recorded; retry. | The same effects and that commit. No second commit. |
| The same, with another request having saved the item in between. | The first commit is reported. The later save is still on disk. |
| A save writes its file and dies before the commit; retry with the same body. | The retry commits. One commit. |
| A save dies before writing; retry. | The retry writes and commits. One commit. |
| Either of the two above, with an unrelated commit made to the same branch in between. | The same. |
| Either, with someone else having committed different content to the same file in between. | `external_change`. Nothing is written. This is open case 1. |
| A save that changes nothing; its result is lost; retry. | A no-op. No commit is reported, even if earlier commits hold the same content. |
| A create writes its file and dies; the user's initials or the prefix change; retry. | The retry commits. The short code first written is kept. |
| A request is rejected with nothing written. | Nothing is left: a different request on the repository runs at once, and the same request ID can be used with corrected input. |
| A request returns an error after it wrote something. | `partial`. A retry of the same request finishes it. |
| The same request arrives twice at once. | One set of effects. Both callers get a truthful result. |
| The database is lost; the repository is registered again; a completed create or save is retried. | A no-op. Nothing is written. |
| The same, for a save that had written and not committed. | `external_change`; a fresh read and save commits it. |
| A confirmed command is accepted and dies before doing anything; the observed state changes; retry. | `external_change`. A new preview is needed. |
| A confirmed command completes; its result is lost; retry, at any later time. | Its result, or a no-op. Not `external_change`. |
| A confirmed command is interrupted partway; retry after the confirmation's ten minutes. | It completes without a new preview. |
| A synchronization's push is accepted and the result is lost; retry. | Published, the same commit, one push. |
| A key generation is interrupted after the key pair is written; retry. | The key is registered. One key. |
| A key deletion is interrupted after one file is removed; retry. | The deletion completes. |

### What `execute` does

This subsection and the next describe the mechanism as Task 7 built it, for
`ticket create` and `ticket save`. Those are the only two commands bound so
far. Confirmation, progress events, cancellation after acceptance,
synchronization and the key commands are not built. Where the text speaks
of one of them, it says so, and it is the plan for the task that binds it.
[What changed from the proposal, and known limits](#what-changed-from-the-proposal-and-known-limits)
lists each place where the built mechanism differs from the one this
section first proposed.

Three terms are used throughout.

- A **binding** connects one command to the boundary (`Binding` in
  `mutation/bind/mod.rs`). It names the command, its target and its digest
  fields, and has four steps: `prepare` makes the checks before acceptance,
  `run` makes the domain call of a first call, `reenter` reads what a retry
  needs before its domain call, and `rerun` makes the domain call of a
  retry. `TicketBinding` serves both ticket commands.
- The **journal row** is where the request's operation stands in its
  journal. `journal_row` returns it as a `JournalRow`, which is one of
  three things. `Absent`: the journal holds no row for the operation ID, or
  holds a local row completed with the step `rejected`. `Pending`: the
  operation has not ended; a remote row whose phase is interrupted or
  failed reads this way. `Final`: the operation has ended. A `Final` row
  carries `kind` (`Completed`, `Cancelled` or `RetainedForInspection`),
  `owes_work` (a remote row whose index hand-off or reconciliation is
  still owed) and `checkpointed`.
- `checkpointed` says the row records that the operation passed the point
  where its work is committed. For a local row it is true when the last
  step recorded is `authoring_checkpoint_observed`,
  `authoritative_observed`, `initialization_committed` or
  `publication_committed`. A save records its checkpoint step whether it
  committed or found nothing to commit, so the step does not show that a
  commit was made. Its absence shows that none was. Remote and key rows
  record phases, and their final rows always read `true`.

A row is **in flight** when it is `Pending`, or `Final` with `owes_work`
set (`JournalRow::in_flight`). Nothing is in flight under an `Absent` row or
a `Final` row that owes nothing.

**Looking the request up.** `execute` builds the binding and computes the
request's scope key and intent digest. It then looks the request ID up, at
most three times. A look reads the record and does one of four things:

1. **No record:** the first call, below.
2. **A record whose scope, command, target or digest differs:** a mismatch.
3. **A finished record that matches:** a replay of the stored result.
4. **An accepted record that matches:** an earlier call started and its end
   was not recorded. The call goes to
   [Re-entering a request](#re-entering-a-request).

Cases 2 and 3 always answer. Cases 1 and 4 answer or ask for another look.
A first call asks for one when another call inserted a record for the same
request ID first. A re-entry asks for one when the record stopped being
`accepted` before it could enter, or when it released the record itself.
Three looks cover a call that releases a record and then loses the insert.
After a third look with no answer the result is `busy` with a
`request.retry` action. If the record cannot be read, the result is the
read's code, such as `index_unavailable`, and nothing is done.

**A mismatch** returns `request_mismatch`. Nothing runs and the record is
not changed. The result names no operation and offers no recovery action.

- Against a **finished** record the result is an input error and every
  effect is `not_requested`. The finished request's own result is what
  `show_request` returns; it is not reported as this call's.
- Against an **accepted** record the boundary reads the journal row
  (`bind::recorded_effects`). If the row is `Absent`, no attempt reached
  the domain: the result is an input error with no effects. If the row is
  anything else, or cannot be read, the effects are taken from the
  repository. A commit in range that changed the request's path gives
  `write: written`, `checkpoint: committed`, `discovery: pending` and that
  commit; the newest such commit is named, whatever it left at the path
  (`path_commit`). With no such commit, a file in the item's editing
  context whose digest differs from the recorded `expected_digest` gives
  `write: written`, `checkpoint: pending`, `discovery: pending`. With
  neither, no effect is reported. A range that cannot be read shows no
  commit. The outcome is `partial` when an effect is durable, so the CLI's
  exit status is 4 as its RFC requires for input rejected after an earlier
  attempt's effects; otherwise it is an input error.

**A replay** returns the stored result and makes no domain call. The
envelope is rebuilt from the record: the command, the stored outcome, code,
six effects, commit and data, the request ID and the operation ID. The
scope comes from this call's repository and target, which the match has
shown equal to the record's, with the branch from the recorded position.
The outcome is the stored one; this is the one place it is not derived.
The record stores no recovery action, and a finished ticket create or save
has none. If the stored result names a commit, the boundary first checks
that the commit can be reached from the branch recorded in `base_ref`, or,
once that branch is gone, from the branch checked out at the root
(`still_reachable`). If it cannot, or Git cannot be read, the result is
`recovery_required` with no effects. The record stays `finished` with its
result, and the next call checks again.

**The first call.** A request no record holds is checked, accepted, run
once and settled, in this order.

1. **A presented confirmation** is `not_confirmable`: neither bound command
   takes one. The check of a required confirmation belongs here and is
   built with the first confirmed command.
2. **The binding's `prepare`** validates the input and makes the checks
   that need no lease. A failure answers the request with no record and no
   operation ID. For the two ticket commands, in order:
   - The draft must be writable as a ticket, or the result is
     `invalid_input`.
   - The repository must resolve; an unregistered one is
     `repository_not_registered` with the `index.rebuild` action.
   - The root must be the repository's own root. A linked worktree is
     `not_repository_root`.
   - **A create** checks its relationships
     ([Validation before any write](#validation-before-any-write)); a
     target that is not an ID is `invalid_relationship`. It then applies
     the already-applied rule to an item that already holds the ID at the
     ticket's path: as intended and committed is `already_applied`, as
     intended and not committed is `external_change`, anything else is
     `occupied_path`. An item at another path, or a document, is left to
     the domain operation to refuse. Then it composes the short code. Its
     expectation is that no file is at the path.
   - **A save** reads the item (`item_not_found`), refuses a closed ticket
     (`ticket_closed`), checks its relationships, and then checks the
     caller's token. A token that is not the file's is answered by the
     already-applied rule under [Observations](#observations). A token
     that matches gives the expectation: the digest of the same bytes. For
     an item with no editing context, primary's file must also be the blob
     at primary's head, or the result is `worktree_not_clean`.
   - The binding reads the recorded position (below). If Git cannot be
     read for it, the result is `repository_inaccessible`.
3. **The cancellation token** is checked. A cancelled request is
   `cancelled` and has done nothing. This is the only cancellation check
   built.
4. **Acceptance** is one transaction (`insert_request`). It inserts the
   record as `accepted` with attempt 1, the scope, command, target, digest,
   recorded position and expected digest, and one operation row holding a
   new operation ID and its journal family. If a record already holds the
   request ID, nothing is written and `execute` looks again. The same
   transaction marks a presented confirmation accepted, conditionally on
   its not having been accepted meanwhile; no bound command presents one
   yet.
5. **The domain call.** The record functions release the cache guard
   before they return, so the guard is not held when the binding's `run`
   makes its one domain call, `save_ticket_with`, with the recorded
   operation ID and expectation. The domain call takes the guard, its
   lease and its journal row itself. The boundary takes no lease and holds
   no lock between its steps.
6. **The first-call commit rule** decides the commit and effects, and the
   record is settled. Both are described next.

**The first-call commit rule.** The commit and the effects come from what
Git and the journal show after the domain call, never from the kind of
result alone.

- **The domain returned saved.** If it names a commit, `evidence::confirms`
  asks whether that commit is in range and changed the request's path. If
  so, the commit is reported: `ok`, `write: written`,
  `checkpoint: committed`, `discovery: current`. If not, or if the domain
  named none, no commit is reported: `already_applied`, `unchanged`,
  `unchanged`, `not_requested`. A save that changed nothing can name the
  head it found, and this is what refuses it. If the index hand-off is
  still owed, the code is `discovery_pending` and `discovery` is `pending`,
  with or without a commit.
- **The domain returned "identity required".** The code is
  `identity_required` with no effects.
- **The domain returned an error.** The code comes from the error's kind
  alone. The boundary reads the journal row. If the row is in flight, or
  cannot be read, the effects come from the repository. The newest commit
  in range that changed the path, if it left the path as intended, gives
  `written`, `committed`, `discovery: pending` and that commit
  (`intended_commit`). With no such commit, a file in the editing context
  that is as intended and is not what it was before the request gives
  `written`, `checkpoint: pending`, `discovery: pending`. Otherwise, and
  whenever nothing is in flight, no effect is reported.

A first call trusts the domain's claim inside the range. A retry does not;
it applies the stricter rule under
[Re-entering a request](#re-entering-a-request).

**Settling.** A binding ends each call with an answer (code, effects, data)
and a `Standing`, which says what is left of the request. The outcome is
derived from the code and the effects in one place
(`Envelope::classified_mutation`); no binding chooses it. A stop that left
a durable effect is therefore `partial`. `Settlement::of` turns the
standing into one of three changes to the record: store the result and mark
it `finished`, delete it, or leave it `accepted`.

Settling is decided from the standing and the journal row, never from the
kind of error the domain returned: the domain returns the same kinds before
and after an effect, and returns some rejections as successful values.

| Standing | When a binding returns it | The record |
| --- | --- | --- |
| `Final` | The request's end state holds: a save returned and its hand-off is complete; a retry found the request done; a retry was answered by the already-applied rule. | Stored and marked `finished`. |
| `Owed` | The domain returned a result and the index hand-off is still owed (`discovery_pending`). | Left `accepted`. A retry continues it. |
| `Unconfirmed` | Git could not be read to say what the request committed. The result claims no effect and no commit. | Left `accepted`. Nothing is stored and nothing is deleted. |
| `Stopped`, journal row not read | The request stopped and the journal lookup failed. | Left `accepted`. |
| `Stopped`, row `Final` with kind `Cancelled` or `RetainedForInspection` | A result the domain has made final for the operation ID. No bound command reaches this yet; it is for synchronization and key generation. | Stored and marked `finished`, whether or not the row owes work. |
| `Stopped`, any other row that is in flight | The request stopped with work under its operation ID: `identity_required`, a domain error, or a change from elsewhere found on a retry. | Left `accepted`. |
| `Stopped`, any other row that is not in flight | The same, with the row `Absent` or completed and owing nothing. | Deleted. The confirmation it accepted, if any, is released in the same transaction; its expiry still runs from its creation. The request ID is free again. |
| `NotRun`, row `Absent` | A retry stopped before its domain call, and no attempt had started. | Deleted, as above. |
| `NotRun`, row `Pending` or `Final` | The same, and an earlier attempt had started. The stop says nothing of what that attempt did. | Left `accepted`, also when the row has completed. |

Four rules hold for every row of the table.

- **Final kind first, then in flight.** For a stopped request the boundary
  asks whether the row's `kind` is one the domain has made final before it
  asks whether anything is in flight. A cancelled synchronization that
  still owes its index hand-off is in flight and final at once; asking in
  the other order would leave its record `accepted` for good.
- **Every record change is conditional on the attempt.** `finish_request`
  and `delete_request` change the record only if it is still `accepted`
  with this call's `attempt`. A call never settles a record another call
  has since entered.
- **A failed journal lookup leaves the record as it is.** After a first
  call's domain error the effects are then read as if the row were in
  flight. On a retry, before any domain call, the result is the read's
  code and no domain call is made.
- **A record change that fails leaves the record as it is.** The caller
  still gets its result. The next call of the request re-enters the record.

**Could not read Git.** A failure to read Git is never taken for "no
commit". When the domain returned a result and the evidence could not be
read, the result is `internal_error` with no effects and no commit, and the
record stays `accepted` for a retry to settle. A no-op stored at that point
would be replayed for good for a save that may have committed.

**Recovery actions.** A recovery action a failed read suggested for itself
is kept. Otherwise the action follows the code. `discovery_pending` names
`operation.resume` for the operation; `resume_operation` itself is not yet
built. `identity_required` and `initials_required` name
`repo.identity_set`. `recovery_required`, `invalid_slug_configuration` and
`invalid_configuration` name `repo.inspect`. `repository_not_registered`
and `index_unavailable` name `index.rebuild`. `external_change` offers
none, also when its record stays `accepted`: the caller reads the item
again and submits a new request. Any other result whose record is left
`accepted`, and any `busy`, offers `request.retry` with the request ID.

A command with no journal (identity, key registry changes, host approval,
folder creation) is a single step that either happened or did not; an error
from one deletes the record, and a re-run is idempotent. No such command is
bound yet; this is the plan for the tasks that bind them.

Losing a record is always safe. If a record is deleted or never written and
the work was in fact done, the next attempt is treated as new and is
answered by the already-applied rule: the content is as intended and
committed, so nothing is written. This is the backstop for every settlement
mistake and for the loss of the whole database.

Two calls that submit the same new request ID race on the record's key.
The one that loses the insert looks again, finds an accepted record, raises
`attempt` and re-enters; the repository lease serializes the two domain
calls. The first caller's `attempt` is then no longer current, so it
reports what it did and changes nothing. The second settles the record from
the evidence read around its own domain call, which sees the first caller's
commit. Or the second finds the lease held and returns `busy`; its record
stays `accepted` when the first call's journal row already exists. The
first known limit below covers the case where it does not.

If a rejected first attempt created an editing context before it was
rejected, the context stays. The item's observation token names its branch,
so after the next refresh the caller's token is stale and it reads the item
again. That is the one way a rejected call is visible afterwards. A save
that stopped for a missing identity is the common case. Once an identity is
set, the same request ID runs the save if nothing refreshed the index in
between, and is `external_change` if something did; the caller then reads
again and may use the same request ID. No test covers the second branch.

### Re-entering a request

A retry of an accepted request skips the token check. The caller's token
described the file before the first attempt, and the first attempt's own
write or context is allowed to have changed it; the CLI RFC says "the
operation's own completed transitions do not invalidate its retry". It also
skips the closed-ticket guard, the rule for an item with no editing context
and the rejection of a relationship: the request passed them when it was
accepted.

**The recorded position.** At acceptance the boundary records the branch the
request will commit to and its tip, in `base_ref` and `base_oid`. For an
item with no editing context yet the branch does not exist; the boundary
records the context's branch name and primary's head. For `repo create` on
a root that is not yet a repository the plan is to record nothing. The
position only bounds where the evidence is read.

**The evidence** is what Git shows of the request's own path since the
recorded position (`PathEvidence`, read by `evidence::path_evidence`). For
a ticket the path is the ticket's file. A move's two paths and the
configuration file of `repo enable`, `repo create` and `remote select` are
the plan for those bindings. These terms describe it:

- The **range** is the commits reachable from the recorded branch and not
  from the recorded commit. It is **bounded** when the recorded commit is
  the branch's tip or an ancestor of it. If the branch does not exist, the
  range is empty.
- The range is **widened** when the branch exists and the recorded commit
  does not bound it: nothing was recorded, the repository does not hold
  the recorded commit, or the commit is no longer an ancestor of the
  branch because the branch was amended or reset. The range is then the
  whole branch, which holds history older than the request.
- A **change** is a commit in range that left the path other than its first
  parent had it. A commit with no parent is compared with no file.
- A change is **intended** when it left the path as the request intends.
  The binding decides this from the file's parsed fields (`holds`). For a
  save, the file is as intended when applying the draft's fields and the
  relationship options to it changes nothing. For a create, the file must
  equal the ticket built from the input, with a `slug` present whose value
  is ignored because it is written once. A change that removed the file is
  not intended.
- A change is **from the expected state** when its first parent held at the
  path exactly what the request expected: a file whose digest is the
  record's `expected_digest`, or no file for a create.
- The evidence is **anchored** when the range is bounded and the recorded
  commit holds the expected state at the path. That is the case when the
  file was committed at acceptance. It is not the case when the file had
  been edited and not committed.
- The request was **settled at acceptance** when the evidence is anchored
  and the file in the recorded commit is already as intended. The request
  then had nothing to commit. This is never so for a create.
- The **candidates** are the changes that can be the request's own commit,
  oldest first. A candidate is intended and, when the evidence is anchored,
  from the expected state. There are no candidates when the request was
  settled at acceptance.
- **Own** is the oldest candidate, and only when the range is not widened
  (`PathEvidence::own`).
- The path is **superseded**, meaning changed from elsewhere, when the
  newest change is not intended. There is one exemption. While the branch's
  tip still holds the expected state at the path and there is no own
  commit, the path is not superseded: what the range shows is history
  older than the request. When there is an own commit and the tip is back
  at the expected state, someone undid that commit, and the path is
  superseded.

Unrelated commits on the same branch (a comment on the item, a developer's
commit on primary) change none of these.

**A Git read failure** is its own answer at every read.

| Read | What a failure does |
| --- | --- |
| The recorded position, at acceptance | `repository_inaccessible`. The request is not accepted. |
| `confirms`, after a first call's save | `internal_error`, no effects, no commit. The record stays `accepted`. |
| `intended_commit`, after a first call's error | No commit is shown. The row is in flight, so the record stays `accepted` anyway. |
| `path_commit`, on a mismatch | No commit is shown. |
| The evidence before a retry's domain call | `internal_error`, no effects. No domain call. The record stays `accepted`. |
| The evidence after a retry's domain call | After a save: `internal_error`, no effects, the record stays `accepted`. After a domain error: no commit is shown; if the row has completed, the record stays `accepted` with the domain's code. |
| Whether the file is committed, for the already-applied rule on a retry | `internal_error`, no effects. The record stays `accepted`. |
| The same, for a new request | Read as not committed: `external_change`. Nothing is recorded. |
| `still_reachable`, on a replay | `recovery_required`. The record is not changed. |

**The steps.** `reenter` in `mutation/mod.rs` runs them in this order.

1. **Raise the attempt** (`enter_request`). This call is now the only one
   that may settle the record. If the record is no longer `accepted`,
   `execute` looks again and answers from what it finds.
2. **Read the journal row** of the recorded operation ID, before anything
   else is done under that ID. The row's state after a domain call proves
   nothing about an earlier attempt: a repeat resets a `rejected` row
   before the domain runs. If the lookup fails, the result is the read's
   code and the record is left as it is.
3. **Release the record and run the request as new** when the row is
   `Final` with kind `Completed`, owes no work and is not `checkpointed`.
   The earlier attempt ended before its checkpoint: it stopped, for want of
   an identity for example, with nothing in flight. Had its answer not been
   lost, settling would have deleted the record. The boundary deletes it
   now, conditionally on this call's attempt, and releases its
   confirmation in the same transaction. `execute` looks again, finds no
   record and makes a first call: every check of `prepare`, the token check
   and the already-applied rule among them, and a new operation ID. The old
   journal row is left alone and no domain call is made under it. The
   remaining steps do not apply.
4. **Set up the domain call** without `prepare`'s checks (`resume`). The
   relationships are read again only to list the targets nothing holds;
   neither a rejection nor a failed read stops the retry. A create passes
   the short code the file in the editing context holds, if it holds one,
   and composes one only if it does not: the initials or the prefix may
   have changed to something no short code can be composed from. If this
   step fails, the result is its code with standing `NotRun`.
5. **Read the evidence** as it is before the call. If Git cannot be read,
   the result is `internal_error` with standing `Unconfirmed` and no
   domain call is made.
6. **Decide before any call** (`Before::found`). An earlier attempt **may
   have committed** when the journal row read at step 2 is `Pending`, or
   `Final` and `checkpointed` (`Before::committing`). The own commit counts
   here only when an earlier attempt may have committed. The three answers
   are tried in this order:
   - **Done.** The row is `Final` with kind `Completed` and owes no work,
     and there is an own commit. The request's work is done. The record is
     finished with `ok`, `written`, `committed`, `discovery: current` and
     that commit. No domain call is made: nothing remains for the domain to
     complete, and it could only refuse for what has happened to the file
     since. A later change by someone else, committed or not, is left as
     it is.
   - **Foreign.** Otherwise, the path is superseded. The result is
     `external_change`, nothing is written and no domain call is made. If
     there is an own commit beneath the foreign one, the effects are
     `written`, `committed`, `discovery: pending` and that commit, so the
     outcome is `partial`. Otherwise there are no effects and the outcome
     is `blocked`. The standing is `Stopped` with the row from step 2: the
     record stays `accepted` while the row is in flight and is deleted
     otherwise. A record that stays is open case 1 of
     [The Journal Defect](#the-journal-defect), and `operation abandon` is
     the planned way out.
   - **Continue.** Otherwise the domain operation is called.
7. **Call the domain operation** with the recorded operation ID and the
   recorded expectation (`rerun`). This is the only way work is continued:
   only the domain operation completes its own journal row and hand-off.
   Its replay check accepts a file equal to the intent or to the recorded
   expectation, writes and commits whatever remains, or reports no change.
8. **Read the evidence again** and map the result.
   - **Saved.** The commit is the one the call reports (below). With a
     commit the result is `ok`, `written`, `committed`; with none it is
     `already_applied`, `unchanged`, no commit. The standing is `Final`. If
     the hand-off is still owed, the code is `discovery_pending` and the
     standing is `Owed`. The commit the domain names is not used: a second
     call of a save that committed names none.
   - **"Identity required."** `identity_required` with no effects. The
     standing is `Stopped` with the row as read after the call.
   - **An external change, when no attempt had started** (the row at step 2
     was `Absent`). The earlier attempt wrote nothing, so the content came
     from elsewhere. The already-applied rule is applied to the file in the
     editing context: as intended and committed at its worktree's head is
     `already_applied` with no commit, standing `Final`. Otherwise the next
     case applies.
   - **Any other error.** The code comes from the error's kind. The
     boundary reads the journal row again. If the row is now `Final` with
     kind `Completed` and owes no work, and the call reports a commit, the
     request is done all the same: an earlier attempt, or a call of the
     same request that ran beside this one, did the work. The record is
     finished with `ok` and that commit. Otherwise the effects are read as
     after a first call's error, with the commit the call reports, and the
     standing is `Stopped` with the row as read after the call.
9. **Settle** the record with this call's attempt, as a first call's is
   settled.

**The commit a call reports** after its domain call
(`PathEvidence::reported`) is the oldest candidate in the evidence read
after the call for which one of two things holds. Either an earlier attempt
may have committed and the range is not widened, so the commit is the own
commit. Or the commit was not in the range read before the call, so this
call made it. If neither holds for any candidate, no commit is reported:
identical content that was there before is someone else's. The second
clause is what reports the commit of a retry that completes over a
rewritten branch.

The check before the call is made outside the lease, so another request's
commit that is a candidate and lands between it and the call can still be
reported as this one's.

What each domain operation does when called again with the same operation
ID, as read from source, and what the binding does about it. Only
`save_ticket` is bound; its row describes what was built. Every other row,
`save_document` included, is the plan for the task that binds it.

| Operation | On replay after completing | Binding |
| --- | --- | --- |
| `save_ticket`, `save_document` | Returns saved with no change; no rewrite; runs the hand-off if owed. With no identity it returns "identity required" and completes its row. | Built for `save_ticket`. Not called when the earlier attempt ended before its checkpoint (the record is released and the request runs as new), when the request is done, or when the path is superseded. Otherwise called with the recorded ID and expectation; the commit is the one the call reports from the evidence read before and after. `save_document` is to bind the same way and is not yet built. |
| `enable` | Returns already enabled; completes registration. | Calls it; takes the commit from the evidence check. |
| `create_and_enable` | Fails: it rejects the now non-empty directory. | On a retry, if the root holds a repository, including one initialized and not yet committed, calls `enable` with the recorded ID, which the journal accepts as the same action. |
| `set_publication_remote` | Returns changed with the current head whenever the configuration equals the request, whoever made the commit. | Never takes a commit from this outcome, on a first call or a retry. Runs the evidence check; if the configuration already equals the request and the check finds no commit of this request, reports a no-op. With no identity it returns an identity error, which the binding reports as `identity_required`. |
| `add_remote` | Returns no change. | Reports a no-op. |
| `remove_registration` | Deletes every journal row for the repository, its own included, when it succeeds; returns not registered afterwards. | On re-entry, if the repository is not registered, the request's end state is reached: reports a no-op and finishes. |
| `generate_shared_key` | Returns already created. After an interruption, a key pair that was written is verified and registered on the next call; one that was not is retained for inspection, and that is final. | Calls it. Finishes the record only on success or on "retained for inspection". |
| `delete_generated_key` | Returns already deleted. After an interruption it completes under the same operation ID with a fresh review; a missing file is skipped. | Calls it with the recorded ID and a fresh review; once the key's row is completed, with no review. |
| `synchronize_remote` | Replays its recorded outcome. | Passes it through. An "interrupted" result is `cancelled` only when the row's phase is cancelled. |
| `synchronize_remote`, interrupted | Returns recovery required unless `restart` is set. | Sets `restart`. A restart fences any executor still running under that ID, which is Wave 02's own takeover and pushes nothing twice. |
| Key registry changes, identity, host approval, folders | No operation ID; idempotent. | Runs again; reports a no-op if the state is already as requested. |

### What changed from the proposal, and known limits

The mechanism this section first proposed was changed in the places below
while Task 7 was built and reviewed. Each change was made because a
sequence of events gave a result the
[Required outcomes](#required-outcomes) table forbids, or because a ruling
during the build decided a point the proposal left open. Row numbers count
the rows of that table from the top.

| Place | The proposal's rule | What showed it wrong, or the ruling | The rule as built |
| --- | --- | --- | --- |
| Whether a retry calls the domain | Always: only the domain completes its row and hand-off. | A save commits and its output is lost. Someone edits the file and does not commit. The retry's domain call is refused as an external change, the row has completed, so the record is deleted and the caller never learns its commit. Row 3. | No domain call when the row has completed, owes nothing and the own commit is in range. The record is finished with that commit. |
| Order of the checks before the call | A foreign change is looked for first. | A save commits and its output is lost. Another request saves other content. The retry is `external_change`. Row 4 requires the first commit. | Done is decided before foreign. |
| The commit rule: the journal row before the call | An earlier attempt "had started" when its row was pending or completed with any step but `rejected`. | A save stops for want of an identity and its output is lost. An identity is set and another request commits the same content. The retry reports that commit. Rows 9 and 11. | An earlier attempt may have committed only when its row was `Pending`, or `Final` and `checkpointed`. `JournalRow::Final` gained `checkpointed` for this. |
| The commit rule: the state the commit was made from | Any commit in range that left the path as intended. | A save changes nothing and its output is lost. A second request saves other content and a third saves the first content again. The retry reports the third request's commit. Row 9. | When the file was committed at acceptance, a candidate's first parent must hold the expected state at the path. |
| The commit rule: a request with nothing to commit | Not considered. | A save changes nothing and its output is lost. Another request, from the same state, commits a change to a field the first does not set, such as `deps`. That commit is from the expected state and leaves every field the first sets as intended. Row 9. | A request settled at acceptance has no candidates. |
| The commit rule: a widened range | When the recorded commit is not an ancestor, the whole branch is searched and treated like any range. | The branch is amended or reset under a request in flight. The foreign-change check then refuses the request for history older than itself, and the repository stays blocked. "Done" can also pick an old commit of the same content. | A widened range gives no own commit. The path is not superseded while the tip holds the expected state and there is no own commit. A revert of the own commit by someone else is still superseded. |
| The commit rule: which commit | Not said. | Decided in the build: a later commit of the same content was made over a path that already held it, or over someone else's change. | The oldest candidate. |
| The first call's commit | Taken from the evidence check after the domain call. | A save that changes nothing, whose refresh mark fails, names the head it found as its commit. | The commit the domain names is kept only if it is in range and changed the path. |
| The answer of the evidence check | Yes or no. The Settling table had no row for a check that could not read Git. | A Git read error after a save that committed was read as "no commit", and the save was finished as a no-op for good. Row 3. | Three answers: `Claim::Confirmed`, `NotThisRequests`, `Unreadable`. "Could not tell" gives `internal_error` with no effects and leaves the record `accepted` (standing `Unconfirmed`). |
| Mismatch against a finished record | `request_mismatch`, carrying the recorded effects. | Ruling: an error envelope may not carry a durable effect, and the effects describe what this call did, which is nothing. | An input error with no effects. `show_request` returns the finished result. |
| Mismatch against an accepted record | `partial` if the recorded request made a durable effect. An accepted record stores no effects, so there was nothing to read them from. | A request that committed and was not settled was reported as a plain input error. A request that never created its context was reported as `write: written`. | When the journal row is anything but `Absent`, or cannot be read, the effects are read from Git and the editing context, as described under a mismatch. |
| A foreign change over the request's own commit | `external_change`; nothing said of the effects. | Ruling: the request did commit, and saying so is truthful. | `partial`, `external_change`, with the own commit. No recovery action. |
| A retry whose earlier attempt ended before its checkpoint | Re-entered like any other, under the recorded operation ID. | A save stops for want of an identity and its output is lost. The retry calls the domain, which records the checkpoint step although nothing changed, and its output is lost too. Another request has committed the same content. The next retry reports that commit. Rows 9 and 11. | The record is released and the request runs as new, with a new operation ID. |
| A retry refused after another call of the request committed | An error with a completed row deletes the record. | Call 1 is accepted. Call 2 enters and reads no row and no commit. Call 1 runs to its end. Call 2's domain call is refused. The record is deleted and the request, which committed, is answered as new. Row 13. | If the row has completed, owes nothing, and the call reports a commit, the record is finished with that commit. |
| A retry that stops before its domain call | Settled like any stop: deleted when nothing is in flight. | Decided in the build: the retry could not resolve the repository, and deleting the record would lose the report of a commit an earlier attempt made. | Standing `NotRun`: deleted only when no attempt had started; left `accepted` otherwise, also when the row has completed. |
| `partial` after an error | `write: written`, `checkpoint: pending` when the file in the worktree is what the request intended. | A save that would have changed nothing also has such a file. | The file must also differ from what it was before the request. |
| `discovery` on an uncommitted write | The proposal names only `write` and `checkpoint`. | Ruling: discovery is work the retry still owes. | `discovery: pending` alongside `written` and `checkpoint: pending`. |
| `request.retry` | The recovery table lists it for named codes only. | Ruling: the same request is what finishes a record left `accepted`. `external_change` on an item write stays without an action, as the table says. | Offered on any result whose record is left `accepted`, unless the code has an action of its own or is `external_change`. |
| The short code on a create's retry | Composed again, as on the first attempt. | A create writes its file and dies. The initials change to something no short code can be composed from. The retry is `initials_required`. Row 10. | `resume` passes the short code the file holds and composes one only when the file holds none. |
| The journal lookup | Absent, pending with its step, or completed. | Ruling in Task 6: a completed or cancelled remote row that still owes its index hand-off or a reconciliation must not release its record, and settling must know which way a row ended. | A typed `JournalRow` with `kind` and `owes_work` as separate facts. |
| The lookup loop | Not said. | Decided in the build: a call can lose the insert race, and a re-entry can release its record. | At most three looks, then `busy`. |

**Known limits.** Each is behavior of the code at the end of Task 7.

- **A second attempt can release the record before the first attempt's
  journal row exists.** Between acceptance and the insert of the journal
  row, a second call that stops with no row deletes the record; the first
  call then commits and cannot settle, and a later retry is
  `already_applied` with no commit. It is accepted because no effect is
  repeated, the first caller has its result, and every alternative found
  breaks reuse of a request ID with corrected input.
- **A changed-input result can name another request's commit.** A mismatch
  against an accepted record names the newest commit to the request's path
  since acceptance, and counts a row that ended before its checkpoint as
  an attempt that ran. It is accepted because the mismatch holds only a
  digest of the original input and cannot compare a commit with an intent.
- **After a crash between a rejection and the record's deletion, corrected
  input is `request_mismatch`** until the original input is sent once
  more, which releases the record. The boundary cannot tell a correction
  from a different request; this is raised for the product owner, since
  row 11 says the request ID can be used with corrected input.
- **A context removed by hand after lost output.** With only the context's
  branch deleted, the retry is `internal_error` and the record is
  released; with the whole context removed, the save recreates it from
  primary and commits again; and when the context already existed at
  acceptance, its removal leaves the request's row pending and the
  repository blocked until `operation abandon` exists. These are accepted
  because only hand-editing produces them today and the first commit went
  with the branch, so nothing is committed twice.
- **A request whose file was uncommitted at acceptance, on a branch that is
  then rewritten,** can be answered `external_change` from history older
  than itself. It is accepted because no rule was found that tells this
  from a real change from elsewhere.
- **Identity removed between attempts.** A retry then completes the journal
  row with "identity required" and the record is deleted, also when an
  earlier attempt had committed, so that commit is never reported. It is
  accepted because nothing is written twice and the next call is answered
  by the already-applied rule.
- **Two requests accepted against the same state with identical content
  cannot be told apart once both have started.** The retry of either can
  report the one commit. It is accepted because the content on disk is
  what both intended.
- **The evidence walk is not first-parent.** The range follows every parent
  of a merge while a change is judged against its first parent only. It is
  accepted because F2's bindings refuse divergence, so no merge reaches an
  item's branch through them; C4 and D5 revisit it when they bind merging.
- **A first save that changes nothing still creates the editing context,**
  which makes the caller's token stale. It is accepted as existing domain
  behavior; the already-applied rule is given only for a stale token.
- **An external Git commit that nothing has refreshed** still reads as a
  current, complete index in every index-backed read, the relationship
  check included. It is accepted because detecting it is a product
  decision outside this Cycle.

**To decide before the next binding.**

- The rule that releases a record and runs the request as new lives in the
  boundary, and reads "no checkpoint step" as "did nothing". That holds
  for authoring operations only. A command whose journal records no
  checkpoint step, `index refresh` for one, would always retry as new
  after lost output. A confirmed command would be sent back through
  confirmation, against the row "A confirmed command completes; its result
  is lost; retry". The rule must become something a binding opts into
  before either kind of command is bound.
- The structural cleanups the ledger lists before Task 8: destructure
  `TicketDraft` in `fields`, `holds` and `validate`, so a new field cannot
  be left out of the digest; replace the command lookups by string with a
  closed enum and an exhaustive match; move the generic derivation of
  effects out of the ticket binding; split `execute`, settling and replay
  out of `mutation/mod.rs`; make `outcome::recovery` an exhaustive match;
  and give the context names one source.

### After the cache is lost

When the database is replaced, the request records, the confirmations, the
journals, the registration, the selected key and the host pins are all gone.
A retry then arrives at a repository that is not registered.

1. Every repository command first returns `repository_not_registered`, with
   the `index.rebuild` recovery action. Rebuilding with the root registers
   the repository again. Keys must be imported or generated and selected
   again and hosts approved again; that is Wave 02's existing behavior, and
   its two marker files make host trust fail closed in the meantime.
2. The retried request then has no record and is treated as new. Its checks
   decide it from Git and the file system:

| Retry of | What the library finds | Result |
| --- | --- | --- |
| A create with a caller-supplied ID | The item exists at that path, equals the intended content apart from the fields written once (`created_at`, `created_by`, `slug`), and is committed at its branch tip. | A no-op: `already_applied`. |
| The same | The item exists and differs. | `occupied_path`. |
| A save | The caller's token is stale, the file equals the intended content and is committed. | A no-op: `already_applied`. |
| A create or save | The file equals the intended content and is not committed: the first attempt died between its write and its checkpoint. | `external_change`. The caller reads the item again and saves; the save commits. |
| A save | The token is stale and the file differs. | `external_change`. |
| A synchronization | Local and tracking refs. | Whatever a fresh synchronization finds: already current, or the remaining push. |
| Anything needing confirmation | The confirmation is gone. | `confirmation_not_found`; a new preview is required. |
| Key generation | The earlier key file belongs to no registration. | A new key is generated, as the domain does today. The earlier file is left where it is and can be imported. F2 adopts nothing. |

No retry in this table writes content the first attempt did not intend, and
none reports a commit it cannot find.

The rows for a create and a save are built, for a ticket. The other rows
are the plan for the tasks that bind those commands. For a ticket the built
checks differ from the table in these ways:

- The only field written once is `slug`. Its value is ignored and it must
  be present.
- "Committed at its branch tip" is checked as: the file's bytes are the
  blob at the head of the worktree the read took the file from. If that
  cannot be read, the file counts as not committed and the result is
  `external_change`.
- A save refuses a closed ticket and a rejected relationship before it
  looks at the token. A retried save whose ticket has since been closed is
  therefore `ticket_closed`, not `already_applied`.
- A create whose ID is held by an item at another path, or by a document,
  is left to the domain operation to refuse.
- Each of these results is given before acceptance, so it leaves no record
  and names no operation.

## Observations

A caller proves it has seen the current state by returning the observation
token that `show_item` or `show_path` gave it. F2 keeps the F1 token format,
`v1:` plus a digest of the branch, path and bytes, and adds three things.

**Checking a token.** On a new request the binding reads the item's
effective copy, the same way the read does, computes its token and compares.
If they match, the binding passes the BLAKE3 of those same bytes to the
domain operation as its `ExpectedPathObservation`, and records it. The
domain operation checks the file again under the repository lease, so a
change between the two checks fails the second. On a retry the token is not
checked again, as described above.

**The already-applied rule.** If the token does not match, the binding
compares the item as it now is with what the request intends, by parsed
fields and ignoring the fields written once. If they are equal and the file
is committed at its branch tip, the request's end state already holds: the
result is a no-op, `already_applied`, and nothing is written. Otherwise the
result is `external_change` and nothing is written. The boundary cannot
tell a lost record from a caller who is simply late, and does not need to;
asking for the state that already exists is not a conflict. The same rule
answers a create whose item already exists with the intended content.

**An item with no editing context.** Its effective copy is the file in the
primary worktree, and the domain operation creates the context by checking
out primary's head. Those are the same bytes only if the file on primary is
committed and unmodified. So for such an item the binding also requires
primary's file to equal the blob at primary's head. If it does not, the
result is `worktree_not_clean` and no context is created. The same rule
makes repair require a committed file. A repository whose Git attributes
rewrite line endings on checkout can still differ; that is a stated limit.

**An absent path.** A new read, `observe_path`, returns a token for any
repository-relative path in a named context: the F1 token for a file, and
for a path where nothing exists, `v1:` plus a digest of a fixed "absent"
marker, the branch and the path. A directory is `invalid_path`.
`document move` takes the absent token as its `destination_observation`.
Creation inputs carry no token; a create always expects absence. The CLI RFC
names no verb that returns a token for an absent path; C3 needs one and that
is recorded for its planning.

`document save` and `document move` require the source observation. The
library request leaves it optional today; the boundary rejects its absence
before any write.

**Repository-level state.** Commands that need confirmation bind state that
no item token covers. `prepare` reduces what it observed to one observation
digest stored in the confirmation; `execute` observes again and compares.

| Command | What the observation digest covers |
| --- | --- |
| `repo create` | The parent is a directory; the target is absent or an empty directory. |
| `repo enable` | The checked-out branch and its commit, or that the branch is unborn; a clean, unconflicted worktree; no Manyhands configuration yet. On a repository that is already enabled, `prepare` returns a no-op and no confirmation; a supplied prefix that differs from the stored one is `invalid_input`, since nothing would write it. |
| Both | The effective identity and its source, when none is supplied. |
| `repo remove` | The registration. |
| `repo identity-set` | The current local name, email and initials. |
| `remote remove` | The remote's configured locations; the polling policy. Removing the selected publication remote is refused by the domain; `prepare` returns `publication_remote_in_use` and no confirmation. |
| `key delete` | The key's registration as the existing preflight reports it. Changes to the key files themselves are caught by the existing review at execution. |
| `host approve`, `host replace` | The publication remote's endpoint for the authority; the current pin for it, or that there is none. |
| `document repair`, `ticket repair` | The file's token; that the corrected ID is not held by another item. |

## Two-Phase Confirmation

`prepare` validates the input, observes, and returns a preview:

```text
PreviewDto
    confirmation_id, expires_at
    command, scope
    effects        what will happen, as an ordered list of typed entries
    details        the action's own fields, for example the identity to be
                   written, the adopted ID, the fingerprints
```

It writes one `confirmation_records` row and nothing else. It does not create
a directory, initialize a repository, register anything, write Git
configuration or contact a network.

`execute` with a confirmation ID accepts it only when all of these hold:

- The record exists. If not: `confirmation_not_found`.
- Its command, scope, target and input equal the call's, compared through
  the digest salted with the confirmation ID. If not:
  `confirmation_mismatch`.
- It has not been accepted by another request. If it has:
  `confirmation_used`.
- It has not expired. A confirmation lasts ten minutes. It is also treated
  as expired when the clock reads earlier than its creation time, so a clock
  set backwards cannot extend it. If expired: `confirmation_expired`.
- A fresh observation gives the same observation digest. If not:
  `external_change`, with a recovery action to prepare again.

Acceptance is written in the same transaction as the request record. From
then on the confirmation belongs to that request, and a retry of the request
is honored after the ten minutes.

On a retry of an accepted request, the boundary first asks whether the
command's end state already holds: the repository is enabled as requested,
the registration or the remote is gone, the identity or the pin has the
requested value, the key is deleted. If it does, the result is a no-op and
the record is finished; a completed command whose result was lost must
never be answered with `external_change`.

Otherwise the observation is compared again unless the request's operation
has a recorded step in its journal. The CLI RFC requires that "the service
rechecks observations before the first effect". A journal row alone is not
enough: `enable` writes its row before any check, so a row with no step
means no step has completed, and a row closed as `rejected` means nothing
was written. A process killed inside `enable` can leave files written
with no step recorded; the comparison then fails on the dirty worktree,
which is open case 2. Once
any other step is recorded, the request's own
work has changed what would be observed, so the comparison is skipped and
the re-entry rules protect the rest.

The confirmed commands with no journal (`repo identity-set`,
`host approve`, `host replace`) always compare again when their end state
does not hold. If a process was killed partway through one, its own partial
change makes the comparison fail with `external_change`. A new preview,
which shows the state as it now is, and a new request complete it.

A command that needs confirmation and is executed without one returns
`confirmation_required`. `prepare` for a command that needs none returns
`not_confirmable`.

**Key deletion.** The existing review value cannot be persisted, and its
evidence is private. `prepare` calls the read-only preflight and stores a
digest of the registration it reports. `execute` calls
`review_generated_key_deletion`, compares the digest of that review's
registration, and passes the review to `delete_generated_key`, whose own
check catches a changed key file. An interrupted deletion has a row in the
key journal with a recorded phase; its retry skips the comparison, obtains a
fresh review and completes under the same operation ID, as the deletion
code allows. Once the key's row is completed the review reports the key as
not registered, and the binding calls the deletion with no review, which the
domain answers with "already deleted".

## Result Mapping

### Envelope

`Envelope` gains a mutation constructor that takes the outcome explicitly. A
read's outcome follows from its code; a mutation's does not.

A mutation envelope may carry `data` on any outcome: the members of a
rejected cycle, the fingerprints of a host that needs approval, what a
half-finished key deletion left. The contract test for mutations asserts
what each command's data holds per outcome, in place of the read rule that
data is present exactly on success.

`request_id` is the caller's request ID for `execute` and `resume_operation`
and null for `prepare`, `show_request`, `cancel_request` and
`observe_host`. `scope.branch` and `scope.worktree` are set for item
mutations.

### Outcome and failure class

The CLI maps a failure class to its exit status. For a read the class is
fixed by the code. For a mutation it cannot be, because the same stopping
reason is a different exit before and after an effect. The rule, in the CLI
RFC's classification order:

1. A cancellation is `cancelled`.
2. Any durable effect with work remaining is `partial`, class Incomplete,
   whatever stopped it. The code still names what stopped it.
3. Otherwise a failure takes its code's own class.
4. `success` and `noop` have no class.

`Envelope` exposes this as one function, so neither front end re-derives it.

### Effects

The six effects and their values are frozen: the CLI RFC makes any change to
them breaking. They describe canonical content and Git. Several F2 commands
change neither.

| Command | What it reports |
| --- | --- |
| Item saves, repair | `write`, `checkpoint`, `discovery`. A save that changes nothing reports `unchanged`, `unchanged` and `not_requested`. |
| Synchronization | `publication`, `discovery`; `commit_oid` is the published commit. |
| `repo create`, `repo enable` | `write` (the configuration file), `checkpoint` (the initialization commit), `discovery`. |
| `remote select` | `write`, `checkpoint`, `discovery`, as it commits the configuration file. |
| `index refresh`, `index rebuild`, `folder create` | `discovery` only. A failed refresh or rebuild has made no durable effect: it is an error with its code, not `partial`, and its record stays `accepted` while its journal row is pending. |
| Identity, remotes added or removed, keys, host trust, registration removal | Every effect `not_requested`. What changed is in `data`, and `outcome` distinguishes `success` from `noop`. |

For the last row, `partial` is decided by the domain outcome and described
in `data`, since no effect can show it: a key deletion that removed one file
and not the other returns `partial` with `recovery_required` and the
domain's recovery state. This is the reading recommended for decision 3. The
alternative, a new effect for local state, is a contract version change.

### Result codes

Added to the registry. Each has a fixed message and its own class, used by
rule 3 above.

| Class | Codes |
| --- | --- |
| Input | `invalid_input`, `request_mismatch`, `request_not_found`, `confirmation_mismatch`, `not_confirmable`, `relationship_cycle`, `invalid_relationship`, `slug_already_assigned`, `occupied_path`, `not_repairable`, `remote_name_conflict`, `invalid_remote`, `key_not_deletable` |
| Blocked | `confirmation_required`, `confirmation_expired`, `confirmation_not_found`, `confirmation_used`, `external_change`, `recovery_required`, `original_request_required`, `identity_required`, `initials_required`, `invalid_slug_configuration`, `identity_ambiguous`, `ticket_closed`, `wrong_branch`, `worktree_not_clean`, `worktree_conflicted`, `invalid_configuration`, `publication_remote_required`, `publication_remote_in_use`, `merge_required`, `remote_branch_deleted`, `push_rejected`, `no_selected_key`, `key_unavailable`, `key_rejected`, `selected_key_in_use`, `unlock_required`, `unlock_failed`, `host_approval_required`, `host_replacement_required`, `host_mismatch` |
| Incomplete | `discovery_pending`, `registration_pending` |
| Transient | `poll_yielding`, `remote_unavailable`, `transport_unavailable` |
| Cancelled | `cancelled` |
| None | `already_applied`, for a no-op |

`external_change`, `recovery_required` and `unlock_required` are the
spellings the RFCs already use; the rest are new names for outcomes the RFCs
describe without spelling. `registration_pending` is the existing outcome in
which the initialization commit exists and the registry write failed. A
repository that is not enabled uses F1's `repository_not_registered`. A
passphrase prompt the user dismissed is `unlock_required`: the action did
not happen and needs an unlock.

`outcome.rs` maps each of `RepositoryErrorKind`, `SynchronizationError`,
`SshTransportErrorKind`, `KeyMaterialErrorKind` and every domain outcome
variant through an exhaustive `match` with no wildcard arm, so a variant
added later fails to compile until it is given a code. No mapping calls
`Display` or `Debug` on a domain error. Variants with no meaning to a caller
map to `internal_error`. Some codes need the binding's context and are set
by the binding: `host_mismatch`, `slug_already_assigned` and the
confirmation and request codes.

The repository error kinds whose code is not obvious from the name:

| Kind | Code |
| --- | --- |
| `RepositoryNotEnabled`, `RepositoryNotRegistered` | `repository_not_registered` |
| `DetachedHead`, `WrongCheckedOutBranch` | `wrong_branch` |
| `DirtyWorktree`, `DirtyConfigurationPath` | `worktree_not_clean` |
| `SelectedRemoteRemoval` | `publication_remote_in_use` |
| `InvalidPublicationRemote`, `UnavailablePublicationRemote` | `invalid_remote` |
| `MissingAuthoringTarget` | `item_not_found` |
| `OccupiedItemPath` | `occupied_path` |
| `InvalidIdentity` | `invalid_input`; for `remote select` with no identity the binding reports `identity_required` |
| `RegistryRefreshPending` | `registration_pending` |
| `InvalidPath` | `invalid_path`; for `repo create` on a target that is a file or not empty, the binding reports `occupied_path` |
| `InvalidSharedKeyMetadata`, `InvalidSharedKeySourcePath` | `invalid_input` |
| `SharedKeyRegistryUnavailable` | `index_unavailable` |
| `SharedKeyMaterialPending` | `recovery_required` |
| `RecoveryRequired`, `RollbackIncomplete`, `MismatchedAuthoringContext` | `recovery_required` |
| `OperationMismatch` | `internal_error`: the boundary allocates operation IDs, so a mismatch is its own fault |
| `Io`, `Sqlite`, `Git`, `InjectedFailure` | `internal_error` |

Task 9's table test is the complete mapping for all four error enums and is
reviewed as such.

What a client does with each new code that stops a request:

| Codes | Recovery action offered | Otherwise |
| --- | --- | --- |
| `confirmation_required`, `confirmation_expired`, `confirmation_not_found`, `external_change` on a confirmed command | `request.prepare` | |
| `external_change` on an item write | none | Read the item again and submit a new request. |
| `discovery_pending` | `operation.resume`; `index.refresh` for `folder create` | |
| `registration_pending` | `request.retry` | |
| `poll_yielding`, `remote_unavailable`, `transport_unavailable`, `busy` | `request.retry` | |
| Any code from a synchronization that stopped after it reserved (`merge_required`, `push_rejected`, `remote_branch_deleted`, the host, key and unlock codes) | `request.retry`, with the host action where one applies | See open case 3: the same request, once the cause is dealt with, or `request cancel` and then the same request, releases the reservation. |
| `original_request_required` | `request.retry`, naming the request | |
| `identity_required` | `repo.identity_set`, except for `repo create` | For `repo create`, resubmit with an identity. |
| `initials_required` | `repo.identity_set` | Or resubmit with initials. |
| `host_approval_required`, `host_mismatch` | `host.approve` | `data` holds the presented fingerprint. |
| `host_replacement_required` | `host.replace` | `data` holds both fingerprints. |
| `recovery_required` | `repo.inspect`; `operation.abandon` when a pending local operation is the cause | |
| `invalid_slug_configuration`, `invalid_configuration` | `repo.inspect` | |
| `request_mismatch`, `confirmation_mismatch`, `confirmation_used` | none | A caller error; use a new request ID or a new preview. |
| `publication_remote_required`, `publication_remote_in_use`, `ticket_closed`, the key and unlock codes outside a synchronization, and the Input codes not named above | none | The code and message say what is missing. Actions for these belong with the commands that resolve them and are added when a front end needs them. |

### Recovery actions

Added to the closed registry:

| Action | Arguments |
| --- | --- |
| `operation.resume` | none; `operation_id` is set |
| `request.retry` | `request_id` |
| `request.prepare` | none |
| `operation.abandon` | `root`; `operation_id` is set |
| `repo.identity_set` | `root` |
| `host.approve` | `root`, `authority` |
| `host.replace` | `root`, `authority` |

`operation.resume` registers no argument keys, matching the CLI RFC's
example. `RecoveryAction` gains a constructor that sets `operation_id`.

### Resuming

Repeating the original request with the same request ID is the general way to
finish it. `resume_operation` exists for the work that needs no input from
the caller:

- A local operation whose authoritative step is recorded and whose index
  hand-off is not: it runs the hand-off. The boundary checks the recorded
  step itself, because the hand-off would otherwise also complete a row
  whose write never happened.
- A clean synchronization that was interrupted: it calls
  `synchronize_remote` with the recorded target and `restart` set. The
  existing operation list also offers resume for a conflicted
  synchronization. F2's bindings cannot produce one; if resume is given
  one made some other way, it is `recovery_required`.

For anything else, including an interrupted save, whose body no record
holds, and key generation, it returns `original_request_required` with the
original request ID if a record has it. It takes its own request ID, so a
resume is itself replayable.

## Progress And Cancellation

```text
ProgressEvent
    request_id, operation_id, command, scope
    stage      accepted | observing | writing | connecting | fetching |
               updating_local | pushing | verifying | indexing | stopping
    effects    completed so far
```

`ProgressSink` has one method and is called on the thread running `execute`.
An event carries no body, source, credential or server text. The boundary
emits `accepted` and a stage before each domain call. For synchronization it
also emits a stage at each safe point the executor reaches. That needs the
executor to call an observer in production builds, which today it does only
under `cfg(test)`: a sibling entry point takes the observer and the existing
function delegates to it with none. Two of those points are reached while
the repository lease is held, so a sink must not call back into the library;
the type's documentation says so.

Cancellation can be asked for in two ways, and both end at Wave 02's own
mechanism:

- **`CancellationToken`**, a cloneable flag, for the same process. The
  boundary checks it before acceptance. During a synchronization the
  safe-point observer checks it and, when it is set, requests cancellation
  through the executor's existing path.
- **`cancel_request`**, for a second thread or process. It sets
  `cancel_requested` on the request record and, if the remote reservation
  already exists, calls `cancel_remote_operation`. The safe-point observer
  also reads the flag, so a request that arrives before the reservation
  exists is seen at the first safe point. The flag is cleared when it is
  honored. It has no effect on a local operation, which has no safe point
  after acceptance, and it never finishes a record whose operation has a
  pending journal row; that record must stay open for the retry that
  clears the row.

The safe points at which a cancellation is honored:

- **Every command:** before acceptance. The result is `cancelled`, with no
  record and no effect.
- **Synchronization:** the seven safe points Wave 02 gave it, through its
  own check. That includes the point after a push is verified; once the
  outcome is classified, a cancellation no longer changes it.
- **Local domain operations:** none inside the call. A save holds its lease
  for milliseconds and is not interruptible.

A cancelled result reports the effects completed before it stopped. If work
finished before the cancellation was seen, the result is the success it
earned. A cancelled synchronization is final for its operation ID; the
record is finished and continuing needs a new request. That is not true
of a synchronization with a conflict pending, and a merging synchronization
reaches no safe point between the fetch and the push. Neither arises in
F2, whose bindings refuse divergence; C4 and D5 meet both. F2 claims no rollback
and no deadline. While a blocking transport call has not returned,
`cancel_request` reports that cancellation is requested and not yet
acknowledged; the ten-second "still stopping" display and its timer belong
to D6.

## Abandoning An Operation

Decision 13. Drafted on 2026-10-10; approved without independent review.

A pending local journal row blocks every other operation on the repository
until the same operation is repeated with input it accepts. Open cases 1
and 2 of [The Journal Defect](#the-journal-defect) are the situations where
it cannot be. `abandon_operation` is the way out.

- **What it does.** Under the repository lease, it closes one pending row
  of `operation_records`, named by operation ID, and touches no file, ref,
  branch or worktree. It reports what the operation left: its action,
  target and last recorded step, and the paths of that target that differ
  from their branch head.
- **What it leaves.** An uncommitted item file stays in its worktree; the
  next save of that item reports `external_change`, and a fresh read and
  save commits it. A configuration file written by an interrupted
  `enable` or `remote select` stays dirty; the user restores or commits
  it with Git and repeats the command.
- **The closed row** is kept with its own marker, distinct from
  `rejected`. A repeat of an abandoned operation ID is refused; it is
  neither replayed as completed nor begun again. The boundary deletes the
  request record that owned the operation and releases its confirmation,
  so that request ID is free and its next use gets a new operation ID.
- **Not covered.** Rows of the remote and key journals: a held reservation
  has `cancel_remote_operation`, and key operations complete on their next
  call. A pending merge conflict is ticket `01M4H33R34Z7C7EEKTY1ZCT950`.
- **Binding.** `operation abandon`, confirmation required. The preview
  shows what the report shows and binds the row's state and step; a row
  that completed or advanced meanwhile is `external_change`. An operation
  ID that is absent or not pending is `operation_not_found` or a no-op.
  Every effect is `not_requested`; what was left is in `data`.
- A refresh or rebuild row can be abandoned too, though repeating it is
  simpler.

## Bridges

### Local identity

`set_local_identity` writes `user.name` and `user.email`, and optionally
`manyhands.initials`, to the repository's local Git configuration under the
repository lease. It never writes global configuration. If a later key fails
to write, it restores the configuration file's bytes, as enablement does. It
works on any non-bare repository root, enabled or not, and is idempotent:
equal values are a no-op. It has no journal row.

Its preview shows the values to be written, the current local values and the
effective identity with its source.

For `repo create` and `repo enable`, the preview always shows the primary
branch, the identity that will author the initialization commit and where it
comes from, and the short-code prefix if one is supplied. Confirmation is
required whether or not an identity is supplied; the CLI RFC and the Wave
already say so.

- A supplied identity is written to local configuration, as `enable` does
  today.
- With none supplied and one in local configuration, that one is used.
- With none supplied and only a global one: `enable` uses it and writes
  nothing. `create_and_enable` does not read global configuration at all,
  so for `repo create` the binding reads the effective identity itself,
  shows it in the preview with its source, and passes it to the domain call
  as supplied. It is then written to the new repository's local
  configuration. The preview says so. This is decision 7.
- With none at all: `identity_required` before any effect.

`IdentityDto` gains `initials` and `initials_source`.

### Host approval

A host is identified by its authority, the lower-cased host and port.

**Learning what a host presents.** The CLI RFC gives `host inspect` a
repository "for an observation", and nothing in the baseline lets C2 or D1
see a fingerprint before a synchronization fails. `observe_host` calls
`verify_ssh_transport` with no approval. For an unknown or changed host that
returns the presented key and pins nothing; for a trusted one it reports the
pin. It is a network read with no request ID. It needs the selected key, as
the verification does.

**Approving.** The approvable host is one the selected publication remote
names, for fetching or for pushing; those are the only hosts the
verification connects to. `prepare` is local. It takes the authority and the
fingerprint the caller was shown, and for a replacement the old fingerprint.
It checks that the authority is the publication remote's and which direction
it serves, that the old fingerprint equals the current pin, and returns the
preview. It opens no connection.

`execute` builds the `HostApproval` value and calls `verify_ssh_transport`
for that direction. That function authenticates with the selected key,
compares the presented host key with the approved one, and writes the pin
only if they match. It fetches nothing and updates no ref.

- If the host presents a different key, the domain reports that approval or
  replacement is still required, with the key actually presented. The
  binding compares that with the approved one and returns `host_mismatch`
  with the presented fingerprint. No pin changes.
- If the key needs unlocking and the provider cannot supply a passphrase,
  the result is `unlock_required`. A non-interactive caller with a protected
  key therefore cannot approve a host; the CLI RFC accepts that limit for
  all protected-key automation.
- Approving a host already pinned to the same key is a no-op.

Requiring the selected key to authenticate is stricter than the
authentication RFC, which asks only that "the presented key is observed
again". It is what the existing verification does, and F2 adds no second pin
writer. This is decision 6.

### Folder creation

`create_folder` creates one directory under `docs/` in the primary worktree.
It uses the existing no-follow directory walk, so it cannot be led outside
`docs/` through a symbolic link. It creates no file, no commit and no
journal row. Git does not track an empty directory, so the worktree stays
clean.

Before creating anything it checks, and rejects with `invalid_input`, a path
that would exceed discovery's limits: 16 levels, or 1,024 entries under
`docs/`. Past either limit discovery fails for the whole worktree and every
save stops, so a folder must not be the thing that crosses it. Open ticket
`01M4EHGE9TMR99VYZC184J9XEC` covers those limits themselves. It also checks
what is at the path: a directory is a no-op, a file is `occupied_path`.

The CLI RFC also lets a folder be created "in the selected item's context
when organizing an existing item". A document move already creates the
directories it needs, so F2 does not add that form.

**Listing.** F1's Cycle document says "F2 owns folder creation and its
listing", while the Wave's F2 scope names only creation, and F1's later
ruling is that a read never lists a directory. A created empty folder is
therefore invisible to every read unless the indexer records folders.
Decision 4 recommends that it does:

- Refresh and rebuild record each directory under `docs/` on primary in a
  new `discovered_folders` table, with whether it is empty. Adding the table
  marks repositories stale until refreshed, as F1's migrations did.
- A new read, `list_document_folders`, returns them as a list with
  `complete`.
- `folder create` then runs the ordinary refresh and reports `discovery`. If
  the refresh fails the result is `partial` with `discovery_pending` and the
  `index.refresh` action.

### Repair and adoption

`repair_item` replaces a nonconforming file with a corrected complete source,
in an editing context, through the ordinary checkpoint.

Inputs: the exact path, its observation token, and the corrected source. The
corrected source must be a conforming document or ticket for that path. Its
`id` is the item's identity from then on. The file must be committed on
primary.

- **Adoption.** If the current file has no valid ID, the corrected source's
  ID is being assigned. The caller obtains it from `id new` and puts it in
  the source. The preview states the adoption and the ID. Because the ID is
  part of the confirmed input, the confirmation, the retry and any partial
  replay all use the same one; nothing in the library generates or
  regenerates it. That is what "stable ID allocation" means here.
- **Existing identity.** If the current file has a valid ID, the corrected
  source must keep it. If not: `not_repairable`.
- **Ambiguous identity.** If the corrected ID already belongs to another item
  the index knows, or the path carries a duplicate-ID problem, the result is
  `identity_ambiguous`. Nothing is written and no context is created.
- **Tickets.** A ticket's path contains its ID. Repair applies only when the
  corrected ID equals the directory name. A ticket directory that is not a
  ULID is `not_repairable` and stays visible as a problem.
- **Closure.** If the current front matter can be read and has `closed_at` or
  `closed_by`, the corrected source must carry the same values; if it has
  neither, the corrected source must have neither. Repair cannot close or
  reopen a ticket. If the current front matter cannot be parsed at all, a
  corrected source with closure fields is `invalid_input`.
- **Not repairable here:** a managed file outside its canonical path and a
  kind that does not match its path. These stay as problems.

The domain function needs a third context intent, `Repair`. `Create` requires
the destination to be absent and `Edit` requires the ID to exist on primary;
a nonconforming file is neither. `Repair` creates the context from primary,
checks the file against the expectation, writes the corrected source and
checkpoints it with the ordinary subject for its kind. Its journal action is
`repair_item`, which joins the published operation-action list.

Two consequences until the repaired item's context is merged, which needs
promotion or closure from later Cycles:

- A repaired **document** is listed twice: the nonconforming entry from
  primary, by path, and the repaired item from its context. A repaired
  **ticket** is listed once, because discovery already ignores primary's
  problems under a ticket that has an active context.
- `submit_comment` refuses to write in a worktree that holds any
  nonconforming file. That is existing behavior, and it means a
  nonconforming file on primary blocks comments in every context cut from
  it, before and after F2. Repair does not lift it until the repair merges.
  It is recorded as a finding for C4 and D4.

## Ticket Relationship Writes

### Storage and canonical form

`slug`, `parent` and `deps` stay where F1 reads them, in the ticket's
unknown-metadata map, so a value the caller does not touch is carried
through as it was written, including an invalid one. What changes is how the
three are written:

- When a save sets `deps`, it is written as a block sequence of strings,
  sorted by ID text, without duplicates. An empty list, or null, removes the
  key.
- When a save sets `parent`, it is written as one string. Null removes the
  key.
- A `deps` or `parent` the caller does not touch is also rewritten in this
  form whenever the save writes the file, if F1's relationship view reports
  no problem for it. One with a problem, including a repeated entry, is left
  as it is.
- `slug` is written once and never rewritten. A hand-written slug in upper
  case stays in upper case.
- Key order is not part of the canonical form. A key keeps its place among
  the unknown keys, and a new one is added after them. The relationships
  RFC's example order is an illustration.

The serializer in use quotes a string that a YAML reader would otherwise
take for a number, such as the short code `1e-12345` or an all-digit ULID,
and leaves other strings bare. F2 relies on that and proves it with a
round-trip test; it adds no emitter of its own.

A save that changes nothing still writes nothing: the existing comparison of
the parsed ticket decides, and the canonical rewrite happens only on a save
that writes for another reason.

The library request gains the relationship inputs through a sibling function,
`save_ticket_with`, whose options carry `deps` and `parent`, each as
unchanged, cleared or set, and an optional slug to write if the file has
none. `save_ticket` keeps its signature and delegates with everything
unchanged and no slug, so existing callers and tests see no difference.

### Validation before any write

`check_ticket_relationships` is a public read. It takes a ticket ID and the
proposed `deps` and `parent` and reports, without writing anything:

- Each entry is a canonical ULID. Repeated entries are removed.
- A ticket naming itself is a cycle of one.
- An entry the index knows to be a document or comment is
  `invalid_relationship`.
- An entry the index does not know is accepted and reported as unresolved.
  It is not an error.
- The proposed edges replace the ticket's stored ones in the graph of every
  ticket the index holds, and `TicketGraph` is asked for a dependency cycle
  and a parent cycle through this ticket. Either is `relationship_cycle`,
  with the cycle's members, sorted by ID, in `data`. Closed tickets count.

The ticket bindings call it before anything else is done, so a rejected
create leaves no context, branch or record. D3 calls the same read to
explain a cycle before the user saves.

The check reads the index and does not scan. The relationships RFC defines
it over "the tickets visible in scanned contexts at that moment", which is
what the index holds. If the index is stale the check uses what it has, and
a cycle that slips through is reported by F1's read-time validation like one
that arrives by merge.

### Short codes

```text
slug  = [ prefix "-" ] initials "-" code
code  = the leading 25 bits of BLAKE3(the 26 ASCII characters of the ULID),
        most significant bit first, five bits per character, in the
        lowercase Crockford alphabet 0123456789abcdefghjkmnpqrstvwxyz
```

`ticket_slug_code_length`, 5 to 8, takes more characters from the same hash.

The `ticket create` binding composes the slug before any domain call and
passes it to `save_ticket_with`. The caller cannot supply one.

- **Prefix and length** are read from primary's committed
  `.manyhands/config.toml`. They stay in the configuration's unknown-key
  table, so a repository with a hand-written invalid value keeps working;
  the value is validated only when a slug is composed, and an invalid one is
  `invalid_slug_configuration`. `repo create` and `repo enable` accept the
  prefix and write it. F2 adds no command that changes it afterwards; the
  relationships RFC names none.
- **Initials** come from, in order: the `initials` input of this one call,
  which is not stored; `manyhands.initials` in local Git configuration; the
  first character of the first and of the last word of the effective
  identity's name, or the first two characters of a one-word name. The
  result is lowercased and must be two or three ASCII letters or digits.
  Anything else, including a hand-set `manyhands.initials` that does not
  fit, or a name beginning with a non-ASCII letter, is `initials_required`.
  F2 does not transliterate.
- **Uniqueness** is not checked. The RFC allows duplicates and has the read
  side report them.

A create that is retried keeps the slug already in the file, even if the
initials or the prefix changed between attempts: the domain function writes
the passed slug only when the file has none, and copies the file's slug into
the intended ticket before it compares the two.

`ticket slug-assign` is a save that adds a slug to a ticket whose front
matter has no `slug` key, or has it as null. It uses the current configured
length and the assigning identity's initials. The binding returns
`slug_already_assigned` for a ticket with any other `slug` value, valid or
not, and it makes that check only on a new request. The domain function
never rejects for an existing slug; it keeps the one in the file. So an
attempt that wrote the slug and did not commit is finished by its retry, and
a retry after a lost result is answered by the re-entry rules, which find
the assigning commit.

Golden vectors fix the code for sample ULIDs at each length and the initials
for a set of names. The expected codes are computed outside this crate, and
the vectors file records how, so the implementation is not its own oracle.

### Closed tickets

Nothing in the baseline stops a save to a lifecycle-closed ticket, and the
API audit left that guard to be re-examined with Wave 02 Cycle 10. F2 adds
two more ways to write to one. Decision 5 recommends the `ticket save` and
`ticket slug-assign` bindings refuse a lifecycle-closed ticket with
`ticket_closed`. The guard is in the bindings, not in `save_ticket`, whose
behavior for other callers, including the closure operation still to come,
is unchanged. `ticket repair` is not guarded: the CLI RFC expects a closed
ticket to be repairable with its closure fields intact.

## Comment Author

`canonical::Comment` gains `created_by: Option<String>`. A `created_by` value
that is a non-empty string without a NUL is parsed into the field; any other
value stays in unknown metadata, where the read side already reports it as
an invalid field. `submit_comment` sets the field from the identity it
resolves for the checkpoint, in the form `Name <email>`, the same form as
`closed_by`, without changing the order in which it returns its existing
rejections. A caller cannot supply it.

On a retry, the comment file on disk keeps its `created_at` and `created_by`.
If the identity changed between attempts, the file names the first and the
commit is authored by the second; the canonical RFC says the field is "never
rewritten". The read side takes the author from the field.

## Published Contract

F2 publishes schemas and golden fixtures for the shapes the library
produces. Per-command input schemas belong to the CLI Cycles, which define
the input files.

- Extended: `envelope` (result codes), `recovery_action` (the new actions),
  `identity` (initials), `operation` (the `repair_item` action).
- New: `preview`, `request`, `progress_event`, `cancel`, `observation`,
  `host_observation`, `relationship_check`, `document_folder`,
  `document_folder_list`, and one mutation-data schema per command family.
- Goldens live in `tests/fixtures/mutation_v1/`, produced by the library from
  real repositories and compared byte for byte, with the same placeholder
  and sentinel rules as `read_v1`. There is one per bound command, and one
  per result code that a fixture can produce. The codes that need a fault no
  fixture can inject are pinned by the mapping table test instead; the plan
  lists which.

All additions are within version 1 under the compatibility rule: new fields,
new enumeration values, new codes. No outcome or effect value changes.

Three F1 tests assume reads and are adjusted: the one asserting that the
actions reads suggest are the whole registry, the one asserting that a
code's outcome follows its class, and the helper asserting that data is
present exactly on success.

## Changes To Existing Code

F1 changed no existing public function. F2 changes these:

1. **Journal rows after a rejection**: done on its own ticket and on main
   since 2026-10-10. F2 changes nothing here.
2. `serialize_item` writes a set or valid `deps` sorted and unique. An
   existing ticket with a valid, unsorted `deps` is rewritten on its next
   save that writes. One existing test asserts file order survives and is
   updated.
3. `save_ticket` becomes a thin caller of `save_ticket_with`. With default
   options it behaves as before, and generates no slug.
4. `submit_comment` writes `created_by`, and `canonical::Comment` has the
   field. F1's comment golden is built from hand-written files and does not
   change.
5. Enablement's configuration writer accepts an optional prefix.
6. `synchronize_remote` gains a sibling that reports safe points, accepts
   a cancellation request from its observer, and takes a setting to refuse
   a divergent remote. With the setting on, it returns `MergeRequired` at
   the point where the existing function decides a merge is needed
   (`sync.rs`, the `graph_plan` check ahead of the first integration
   window), before any window or merge is prepared. That stop is after the
   reservation, so it is one of open case 3's. The existing function, its
   request type and its tests are unchanged. A request whose operation
   already has an integration window, which only a merging call creates,
   is `RecoveryRequired` when the setting is on.
7. Refresh and rebuild record folders, if decision 4 is as recommended. The
   new table marks repositories stale once.
8. Each of the three journals gained a lookup of one operation by ID:
   `recovery::lookup_operation` for the local journal,
   `remote::state::lookup_operation` for the remote one and
   `keys::lookup_material_operation` for key material. Each reads the
   index only, so it answers for a root that is not registered or no
   longer exists; that is why `show_operation` does not serve. All three
   return the new type `JournalRow` in `src/repository/recovery.rs`:
   `Absent`, `Pending` with the journal's own state, step or phase, or
   `Final` with `kind`, `owes_work` and `checkpointed`, and the method
   `in_flight`. A local row completed with the step `rejected` reads
   `Absent`; a remote row that was interrupted or failed reads `Pending`.
   `RepositoryService::journal_row` chooses the lookup by journal family.
   See Settling.
9. Public enums gain variants: `ContextIntent::Repair`, the `repair_item`
   action, new `RepositoryOperation` variants. Exhaustive matches on them
   inside the crate are extended.
10. `IdentityDto` and its schema and golden gain the initials fields.
11. The index gains the three request tables and, with item 7, the folder
    table.
12. Plumbing that the design did not list, added while Tasks 6 and 7 were
    built. None of it changes what an existing function does.
    - Test-only, in `src/repository.rs`, hidden from the documentation and
      reached only through `_for_testing` constructors and setters:
      - two failure points, `FailurePoint::BeforeRequestDomainCall` and
        `FailurePoint::BeforeRequestSettlement`, at which a call in the
        boundary ends with its record untouched, as a process that died
        would leave it;
      - a request hook, the field `request_hook`, set by
        `set_request_hook_for_testing`, which runs once immediately before
        the next domain call an `execute` makes;
      - an exit-at-failure-point mode, `open_at_with_exit_point_for_testing`
        with `FAILURE_POINT_EXIT_STATUS`, in which `should_inject` ends
        the process at the failure point instead of returning an error,
        so its leases and locks are left as a killed process leaves them.
    - Behavior-preserving edits Task 6 made to reach existing code:
      `stored_phase` was extracted in `src/repository/remote/state.rs` so
      that the remote lookup parses a phase the way the journal does; and
      the visibility of `read_session` (`src/repository/read/mod.rs`) and
      of the key phase parser `parse_phase`
      (`src/repository/keys/generation.rs`) was widened within the
      repository module.

## Alternatives Rejected

- **Use the request ID as the operation ID.** One fewer table. Rejected:
  compound commands need several operations per request, and the two
  journals already allow one ID to appear twice in exactly one case.
- **Add request columns to `operation_records`.** Rejected: the existing
  test forbids digest and content columns there, the table belongs to one of
  three journals, and it has no row for a repository that does not exist.
- **Keep a record of every rejected request.** Rejected: a request that did
  nothing needs no memory, and keeping it would force a rule for when each
  rejection is final.
- **Infer an earlier attempt's commit from the journal step.** Rejected: the
  step is written whether or not a commit was made, and a replay rewrites
  it. The commits after the position recorded at acceptance are direct
  evidence.
- **Decide from the returned error whether an effect was made.** Rejected:
  the domain returns the same error kinds before and after a write. The
  journal row for the operation ID is the evidence.
- **Refuse a retry whenever the branch moved.** Rejected: branches move for
  unrelated reasons, and a refused retry can leave a pending row nothing
  else clears. Only the request's own paths are examined.
- **Keep a confirmation's lease.** Rejected: nothing can be held across two
  calls, and a ten-minute lock would block every other operation.
- **A new token version covering more state.** Rejected for now: the v1 token
  joined to the domain's byte check is sufficient, and changing the token
  would invalidate tokens callers hold.
- **Promote `slug`, `parent` and `deps` to typed fields of `Ticket`.**
  Rejected: an invalid hand-written value could then not be carried through
  untouched, which the RFC requires.
- **A hand-written YAML emitter that always quotes.** Rejected: the
  serializer already quotes what would be misread, and a second emitter is a
  second place for the file format to drift.
- **Have the cycle check scan the repository.** Rejected: the index is the
  set of scanned contexts the RFC names, and a scan on every save is the
  cost F1 removed from reads.
- **A progress stream with its own thread or channel.** Rejected: the Wave
  excludes a second event framework. A synchronous sink is enough for both
  front ends.
- **Generate the adoption ID inside `prepare`.** Rejected: the CLI RFC has
  the caller submit a complete corrected source, and an ID fixed by the
  caller's input needs no extra state to stay stable.
- **A second way to pin a host without authenticating.** Rejected: one pin
  writer, after an authenticated connection, is Wave 02's design.

## Risks

| Risk | Control |
| --- | --- |
| The Cycle is large: the boundary, twenty-nine bindings, four new domain operations and the relationship writers. | Three parts with a review checkpoint after each. Decision 1 offers the split. |
| A binding maps a partial outcome to a clean success. | Exhaustive matches; one rule for outcome and class; a test per binding family that injects a failure after the authoritative step. |
| A retry reports a commit that is not this request's. | The commit must be a candidate: in the range since the position recorded at acceptance, a change of the request's path to the intended content, made from the state the request expected when that state was committed at acceptance, for a request that had something to commit at acceptance. It must also either have appeared during this call, or lie in a range the recorded commit bounds while the journal row read before the call shows an earlier attempt may have committed (pending, or final and checkpointed). The oldest such commit is reported; otherwise none is. A retry whose earlier attempt ended before its checkpoint is released and run as new. A failure to read Git reports nothing and leaves the record `accepted`. Two requests accepted against the same state with identical content can still be indistinguishable once both started; the content on disk is then what both intended. |
| Settlement deletes or finishes a record another call is using. | Every record change is conditional on the call's own attempt number; a retry never deletes. |
| A body's digest lets someone with the database test guesses of a short body. | Required by the CLI RFC. Digests are salted per record, so equal bodies do not collide. The database already sits beside the repository it describes. |
| Request records grow without bound. | Small rows; rejected requests leave none; decision 8. |
| A preview is treated as a reservation. | The design never holds a lock for one; `execute` observes again and the domain operation checks under its lease. |
| Host approval becomes a way to pin without authenticating. | The only production pin writer remains `finalize_host_trust`. A hidden testing writer exists today and stays out of the boundary. |
| The index is stale when the cycle check runs. | Stated limit; read-time validation reports what slips through. |
| An interrupted operation blocks the repository if its input is lost or its path was since changed by someone else; a refused synchronization holds its reservation. | Existing behavior; the three open cases; decision 2. |
| The settlement rules are wrong again in a way no reviewer has yet found. | The required-outcomes table is the contract; Task 7 is built test-first against it; a design checkpoint follows before other bindings; losing a record is safe by the already-applied rule. |
| Windows and macOS behavior of the new file-system code. | New tests join the native workflow's list; native execution remains the open obligation F1 recorded for G1. |

## Test And Platform Evidence

Tests use real repositories, real SQLite and real Git, with the existing
fault-injection points and lease-holding child process. Synchronization and
host approval use the in-process SSH server fixture and its harness; the
fixture's `World` builder moves from one test file to shared test support.

New test targets: `mutation_replay`, `mutation_confirmation`,
`mutation_contract`, `mutation_bridges`, `mutation_relationships`, and, on
the SSH harness, `mutation_remote`. They run without the `desktop` feature.

The scenarios the Wave's exit evidence names, and where each is proven:

| Evidence | Fixture |
| --- | --- |
| Stale preview rejection | `repo enable` and `document repair`, with the repository changed between preview and confirmation. |
| Changed-input rejection | A ticket save and a synchronization, each retried with one field changed. |
| Accepted-consent retry | `repo enable` and `repo create`, each interrupted after the commit; the retry after a simulated hour completes without a new preview. |
| Lost output reconciliation | A save whose result is discarded, with and without another save in between; a synchronization whose push is accepted and whose result is lost. Each retry returns the same effects and makes no second commit or push. |
| Cache-loss recovery | The database is deleted between attempt and retry, for a create, a save, a save that died before its checkpoint, a synchronization and a confirmed command; each after the repository is registered again. |
| No duplicate effects | Commit counts, ref values and file contents compared before and after every retry. |
| Cancellation at safe points | Before acceptance; at a held synchronization safe point through the token and through `cancel_request` from a second thread; after the outcome is classified. |
| Each bridge | Expected observation, retry, changed input and an injected failure for identity, host approval, folder creation and repair. |
| Host approval publishes nothing | Server-side push count and every local ref compared before and after. |
| Ambiguous identity stays non-editable | Repair with an ID another item holds: no write, no context, no branch. |
| Short-code vectors and invariance | Golden vectors; a slug compared before and after a rename, an identity change and a prefix change. |
| No body or secret in records | A sentinel body, passphrase and remote credential, scanned for in every column of the request tables, the journals and every envelope and event. |

Time is injected for expiry tests through a clock the service holds; the
production clock is the system clock.
