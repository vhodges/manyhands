---
title: "Wave 03 F2 Request Replay, Confirmation And Shared Mutation Bridges Design"
date: 2026-10-08
status: draft
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
It is a draft for product-owner review, revised three times after four
independent reviews. It authorizes no Rust change.

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
refreshes entry-gate item 3 for what F2 consumes. The provisional
[API audit](../research/wave-03-api-audit.md) predates F1 and Wave 02 Cycle
05, and its line anchors no longer hold.

| Area | What exists | What F2 must add |
| --- | --- | --- |
| Operation identity | `OperationId`, a ULID the caller generates. Three journals in the one SQLite file: `operation_records` (local lifecycle), `remote_operation_records` (reservations and synchronization) and `key_material_operations`. Key registry and polling-policy changes have no operation ID. | A request ID and its mapping to operation IDs. Nothing stores a request ID, an input digest or a result today. |
| Replay | Repeating a call with the same operation ID makes the function reconcile from Git, the file system and its journal row. The journal matches root, action and a target string. | Input matching. A save's target string covers the item and paths, not the title, body or metadata, so a reused operation ID with a different body is not detected. |
| Journal exclusion | `begin_or_reconcile` refuses a new operation with `RecoveryRequired` while any other row for the repository is not completed. | See [The Journal Defect](#the-journal-defect): many ordinary rejections leave such a row. |
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

This is a defect in the existing library, found during this planning. It is
certain from the source and has not been reproduced by a test.

A local operation writes its journal row before most of its checks, and
several return on an ordinary rejection without completing that row. The
next operation with a different ID on the same repository is then refused
with `RecoveryRequired`. Paths that leave a row:

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
row only for a registered repository. Today only tests reach this. Through
the boundary every user does: one rejected command, retried under a new
request ID as an interactive caller would, blocks the repository.

The fix is ticket `01M4EWN2DK3MY6H4GBYDYXF6QH`. What F2 depends on: when one of the operations
above returns an error, and the journal row was begun by this call, and
this call wrote nothing durable or rolled back what it wrote, it completes
the row. A row stays pending only when an effect was made and work remains.
Known now, and settled on that ticket (decision 2):

- Neither the error kind nor the recorded step says whether the call wrote.
  The same kinds (busy, SQLite, Git, I/O) are returned before and after a
  write. The fix needs each call to track what it wrote. If the call also
  reported that to its caller, the boundary would not have to infer it; the
  ticket should consider that.
- Two existing tests require a row to stay pending after a failure inside a
  standalone `prepare_context`, and three require it after a failed refresh
  or rebuild. Those stay as they are; the fix does not cover refresh and
  rebuild. A failed `index refresh` or `index rebuild` therefore still
  blocks other requests until the same request is retried.
- `remove_registration` also writes a row and can leave it.

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
   reproduced; Task 13 reproduces it, and it is reported to the owner of
   Wave 02 Cycle 06 if it holds.

Decision 2 covers the fix and these cases.

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
                                    + a lookup of one operation by ID
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
| `item sync`, `repo sync` | `synchronize_remote` | No | Clean synchronization only. |
| `index refresh`, `index rebuild` | `refresh_repository`, `rebuild_repository` | No | |

That is twenty-eight commands. The saves prepare their editing context
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
- `conflict resolve` and divergent synchronization: C4 and D5, after Wave 02
  Cycle 06.
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

1. Look up the request ID.
2. **A record exists and its scope, command, target or digest differs.**
   Return `request_mismatch`, carrying the recorded effects. Nothing runs.
   If the recorded request is unfinished and made a durable effect, the
   outcome is `partial`, so the CLI's exit status is 4 as its RFC requires
   for input rejected after an earlier attempt's effects; otherwise it is an
   input error.
3. **A finished record matches.** Return the stored result. If it names a
   commit, first confirm that commit is still reachable from the branch
   recorded in `base_ref`, or from primary once that branch is gone; if
   not, return `recovery_required` and do nothing. No domain call is made.
4. **An accepted record matches.** An earlier attempt started and its end was
   not recorded. Raise `attempt` and go to
   [Re-entering a request](#re-entering-a-request).
5. **No record.** Validate the input; an invalid one is `invalid_input`.
   Resolve the repository; an unregistered one is
   `repository_not_registered`, except for the four commands that work
   without a registration: `repo create`, `repo enable`,
   `repo identity-set` and `index rebuild`. Make the binding's checks that
   need no lease: the observation token (with the already-applied rule
   under [Observations](#observations)), relationship validation,
   short-code inputs. If the command needs confirmation, check it (below)
   or stop. Check the cancellation token. Then, in one transaction, insert
   the record as `accepted` with attempt 1, a new operation ID, the
   recorded position and the expected digest, and mark the confirmation
   accepted, conditionally on its not having been accepted meanwhile. Run
   the binding.
6. Map the domain outcome to an envelope, take the commit and effects from
   the evidence check below, made after the domain call, and settle the
   record.

**Settling** is decided from the envelope's outcome and from the operation's
journal row, never from the kind of error the domain returned: the domain
returns the same kinds before and after an effect, and returns some
rejections as successful values. The boundary looks the operation ID up in
its journal (local, remote or key) through a new lookup that says absent,
pending with its step, or completed. Every change to the record is
conditional on the record still being `accepted` with this call's
`attempt`, so a call never settles a record another call has since
entered. If the lookup itself fails, the record is left as it is.

| The envelope's outcome | The operation's journal row | The record |
| --- | --- | --- |
| `success` or `noop` | any | Stored and marked `finished`. |
| A result the domain has made final for this operation ID: a synchronization whose row is cancelled; a key generation whose recovery state is "retained for inspection" | any | Stored and marked `finished`. |
| `partial`: work remains (discovery or registration pending; a key generation or deletion the domain says can be recovered) | any | Left `accepted`. A retry continues it. |
| Blocked, an input error, a transient failure or an error | Absent or completed: nothing is in flight | Deleted. The confirmation it accepted, if any, is released in the same transaction; its expiry still runs from its creation. The request ID is free again. |
| The same | Pending: something is in flight | Left `accepted`. For a local operation, a pending row after an error means the call wrote something, once the journal fix is in; the outcome is `partial`, with `write: written` and `checkpoint: pending` when the file in the worktree is what the request intended. For a synchronization the effects come from its stored checkpoint, and the outcome is `partial` only if a push was accepted. |

A command with no journal (identity, key registry changes, host approval,
folder creation) is a single step that either happened or did not; an error
from one deletes the record, and a re-run is idempotent.

Losing a record is always safe. If a record is deleted or never written and
the work was in fact done, the next attempt is treated as new and is
answered by the already-applied rule: the content is as intended and
committed, so nothing is written. This is the backstop for every settlement
mistake and for the loss of the whole database.

Two processes that submit the same new request ID race on the primary key.
The loser sees an accepted record, raises `attempt` and re-enters; the
repository lease serializes the two domain calls. The first caller's
`attempt` is then no longer current, so it reports what it did and changes
nothing; the second settles the record from the evidence check made after
its own domain call, which sees the first caller's commit.

If a rejected first attempt created an editing context before it was
rejected, the context stays. The item's observation token names its branch,
so after the next refresh the caller's token is stale and it reads the item
again. That is the one way a rejected call is visible afterwards.

### Re-entering a request

A retry of an accepted request skips the token check. The caller's token
described the file before the first attempt, and the first attempt's own
write or context is allowed to have changed it; the CLI RFC says "the
operation's own completed transitions do not invalidate its retry".

**The recorded position.** At acceptance the boundary records the branch the
request will commit to and its tip. For an item with no editing context yet
the branch does not exist; the boundary records the context's branch name
and primary's head. For `repo create` on a root that is not yet a
repository it records nothing. The position only bounds where the evidence
check looks: commits reachable from the branch and not from the recorded
commit, or the whole branch if nothing was recorded or the recorded commit
is not an ancestor.

**The evidence check** looks only at the request's own paths: the item's
file, or for a move both paths, or the configuration file for
`repo enable`, `repo create` and `remote select`. It compares parsed
fields, ignoring the ones written once (`created_at`, `created_by`, and
the value of `slug`; for a create or a slug assignment a slug must be
present). For a move, the destination holds the content and the source is
absent in the same commit.

On re-entry:

1. **If a commit in range changed the request's paths to something the
   request did not intend,** return `external_change`. The request is not
   re-run and its record stays `accepted`. If its journal row is pending,
   this is open case 1 of [The Journal Defect](#the-journal-defect).
2. **Otherwise call the domain operation with the recorded operation ID and
   the recorded expectation.** This is the only way the work is continued,
   and it is always made, because only the domain operation completes its
   own journal row and hand-off. Its replay check accepts a file equal to
   the intent or to the recorded expectation, writes and commits whatever
   remains, or reports no change.
3. **If the domain reports an external change and no journal row existed
   for the operation ID before the call,** the earlier attempt never
   started and the content came from elsewhere. Apply the already-applied
   rule: if the content is as intended and committed, the result is a
   no-op with no commit reported.
4. **Take the commit from the evidence check, made after the call.** A
   commit is reported only if it is in range, left the paths as intended,
   and a journal row existed for the operation ID. Otherwise the checkpoint
   is `unchanged` and no commit is reported.

Unrelated commits on the same branch (a comment on the item, a developer's
commit on primary) do not enter into any of this.

What each domain operation does when called again with the same operation
ID, as read from source, and what the binding does about it:

| Operation | On replay after completing | Binding |
| --- | --- | --- |
| `save_ticket`, `save_document` | Returns saved with no change; no rewrite; runs the hand-off if owed. | Calls it; takes the commit from the evidence check. |
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
means nothing has been done. Once a step is recorded, the request's own
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
| `recovery_required` | `repo.inspect` | |
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
  `synchronize_remote` with the recorded target and `restart` set.

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
record is finished and continuing needs a new request. F2 claims no rollback
and no deadline. While a blocking transport call has not returned,
`cancel_request` reports that cancellation is requested and not yet
acknowledged; the ten-second "still stopping" display and its timer belong
to D6.

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

1. **Journal rows after a rejection**, as described under The Journal
   Defect. Decision 2 recommends this be its own ticket, merged first.
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
6. `synchronize_remote` gains a sibling that reports safe points and accepts
   a cancellation request from its observer; the existing function's
   behavior is unchanged.
7. Refresh and rebuild record folders, if decision 4 is as recommended. The
   new table marks repositories stale once.
8. Each of the three journals gains a lookup of one operation by ID that
   says absent, pending with its step or phase, or completed. Today the
   public reads return pending rows only.
9. Public enums gain variants: `ContextIntent::Repair`, the `repair_item`
   action, new `RepositoryOperation` variants. Exhaustive matches on them
   inside the crate are extended.
10. `IdentityDto` and its schema and golden gain the initials fields.
11. The index gains the three request tables and, with item 7, the folder
    table.

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
| The Cycle is large: the boundary, twenty-eight bindings, three new domain operations and the relationship writers. | Three parts with a review checkpoint after each. Decision 1 offers the split. |
| The journal fix changes Wave 01 code on many paths. | Its own ticket and review; every existing test must pass unchanged; a test per path listed above. |
| A binding maps a partial outcome to a clean success. | Exhaustive matches; one rule for outcome and class; a test per binding family that injects a failure after the authoritative step. |
| A retry reports a commit that is not this request's. | The commit must come after the position recorded at acceptance, change the request's paths to the intended content, and the request's operation must have a journal row; otherwise none is reported. Two requests with identical content can still be indistinguishable when both started; the content on disk is then what both intended. |
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
