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

**How settled this is.** Every review round of the proposal found blocking
errors in how a retried request is settled, including in the corrections
made for the round before. The rules under
[Request Identity And Records](#request-identity-and-records) are no longer
the proposal: they describe what was built test-first against the table of
[required outcomes](#required-outcomes), first through `ticket create` and
`ticket save`, and were reviewed against the code at the design checkpoint
after Task 7. The table stays the contract: what must be true after each
sequence of events, whatever mechanism produces it.

The document was brought up to date again on 2026-10-12, after Tasks 8 to
14 and 12A bound every existing mutation the design names, confirmation,
progress, cancellation, `operation abandon` and `operation resume`. What it
says of those is what the code does.

It was brought up to date once more on 2026-10-12, after Tasks 14B and
14C. Task 14B consolidated the mutation boundary and changed no
behaviour. Task 14C made the behaviour changes the product owner approved
that day: a repository that was never enabled is refused, a settings
command refuses a late retry, a blocked synchronization can be cancelled
by an action, a re-entered ticket write fails closed, a settlement that
did not apply is reported as unsettled, and three discovery effects say
what became of the hand-off. The questions the product owner answered
that day are decided rules in the body, and are no longer listed as open.

It was brought up to date a third time on 2026-10-13, after Task 14D,
Tasks 15 to 18, the contract-completeness pass of Task 19 and the fix pass
that followed the four whole-Cycle reviews. Task 14D gave `key import`,
`key select` and `key clear` an operation and a recorded change a retry
answers from. Tasks 15 to 18 built the bridges: `repo identity-set`,
`host observe`, `host approve`, `host replace`, `folder create` with the
folder list, and `document repair` and `ticket repair`. All thirty commands
are bound, and nothing in the document is a plan any longer: what it says
of every command is what the code does. Where the built contract departs
from an RFC by a decision already made, the departure is listed once, under
[Departures from the RFCs](#departures-from-the-rfcs).

It was brought up to date a fourth time on 2026-10-13, after main's Wave 02
Cycle 07, comment publication, was merged into the branch and reconciled
with the journal's readers. The boundary still binds no comment command.
What an abandon, a resume and the operation reads answer for a comment
submission's rows is under
[Abandoning An Operation](#abandoning-an-operation).

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
| Operation identity | `OperationId`, a ULID the caller generates. Three journals in the one SQLite file: `operation_records` (local lifecycle), `remote_operation_records` (reservations and synchronization) and `key_material_operations`. Key registry and polling-policy changes have no operation ID. | A request ID and its mapping to operation IDs. Nothing stores a request ID, an input digest or a result today. F2 also gives a key import, a selection and a clearing an operation ID, recorded with the change; see [Records](#records). A key removal and a polling-policy change still have none. |
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
| Folders | No folder concept. Discovery collects `.md` files; an empty directory is not indexed. Directories are created as a side effect of a document save through a no-follow walk. Discovery fails a whole context past 16 levels or 1,024 entries under `docs/`. | Folder creation and folder listing. |
| Repair | None. A marker-only file has no ID, so no authoring context can be named for it, and `prepare_context` with `Edit` requires the ID to exist on primary. | A repair operation and context intent. |
| Relationships | `slug`, `parent` and `deps` live in a ticket's unknown metadata and are read as a view. The serializer emits unknown keys first, then the defined ones. `TicketGraph` detects cycles from index rows and does no I/O. No code generates a short code or reads initials or a prefix. | Writers, the canonical form, the write-time cycle check, generation and assignment. |
| Comment author | `created_by` is read from unknown metadata on the read side. `submit_comment` treats an existing comment file with any unknown metadata as occupied. | A real field, written at creation. |

Three facts shape the design more than the rest:

1. **The index database is a cache, and it holds more than the index.** A
   corrupt file is renamed and recreated. With it go the repository
   registrations, all three journals, the shared-key registry and selection,
   the recorded key registry changes, and the host pins. Whatever F2 stores
   there can vanish, and after a loss the repository is not even registered.
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
  operation ID, a `rejected` row counts as none. So does a row closed as
  abandoned, whose ID is then refused for good; see
  [Abandoning An Operation](#abandoning-an-operation).
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
  commits it. That a save which wrote its file is closed as `rejected` is
  a defect of the existing library, filed as ticket
  `01M4PB7YZMBFAFFM94AZY08HWS`.

Three cases remain open after the fix. All exist today.

1. A row is pending from the moment it is written. A process killed at any
   point after that leaves it, and only a repeat of the same request with
   the same input clears it. For a save that means the same body. If the
   caller no longer has it, or someone else has since committed different
   content to the same file, no repeat can clear the row, and no command
   in the CLI RFC abandons an operation. `operation abandon`, added by
   decision 13, is the way out.
2. A process killed after `enable` or `remote select` wrote the
   configuration file and before it committed leaves the worktree dirty;
   the replay is then refused as a dirty worktree and authoring is blocked.
   Through the boundary a `remote select` in this state is
   `worktree_not_clean`, and a `repo create` or `repo enable` killed
   before its first recorded step is `external_change` on retry; see the
   known limits. The way out is to restore by hand what the request
   expected; a creation or an enablement whose row is still pending can
   also be abandoned.
3. A synchronization that stops with an error after it reserved (a
   divergent remote, a rejected push, a host that needs approval, a locked
   key, a transport failure) leaves its reservation active. While it is
   active, a save is refused with `recovery_required` and another
   synchronization is `busy`; a reservation taken for one item's branch
   refuses that item's saves only. The way out is the same request again
   once the cause is dealt with, or `request cancel` followed by one more
   call of the same request, which ends it `cancelled` and releases the
   reservation. Sending the same request again while the cause stands
   releases nothing. Task 13 reproduced this for five causes:
   `merge_required`, `remote_branch_deleted`, `push_rejected`,
   `host_approval_required` and `unlock_required`. It is existing
   behavior of synchronization, and is ticket `01M4R19PC0T6ZPKM39X9DF26HX`.
   Two of the five causes, `merge_required` and `remote_branch_deleted`,
   now offer `request.cancel` as the way out; see
   [Result codes](#result-codes).
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
    mod.rs        public surface: MutationCall, Mutation, the clock,
                  show_request, journal_row, the test hooks
    command.rs    Command: the closed set of bound commands, their
                  published names, and what each needs of the boundary
    execute.rs    execute: the lookup, the first call and the domain call
    confirm.rs    prepare, what execute asks of a confirmation, and the
                  one function that enters a confirmed command again
    replay.rs     the replay of a finished record; the mismatch
    reenter.rs    re-entry of an accepted record
    settle.rs     what a call does to its record afterwards, and the
                  one writer of a finished result
    reply.rs      the request's identity; the envelopes of a call
    progress.rs   ProgressEvent, ProgressStage, ProgressSink,
                  CancellationToken, the call's session, cancel_request
    identity.rs   RequestId, ConfirmationId, the intent digest
    records.rs    the three request tables: DDL and access
    observe.rs    observation tokens: checking, the absent token; the
                  observation digest of a confirmed command
    outcome.rs    domain outcomes and errors to effects, codes and
                  recovery actions: every action a result offers is
                  decided here, the resume, cancel and abandon among them
    evidence.rs   what an earlier attempt of a request left in Git
    context.rs    the names of an item's editing context
    dto.rs        RequestDto, CancelDto, PreviewDto and the mutation
                  data DTOs
    bind/mod.rs   Binding, Confirmed, and what every binding shares
    bind/save.rs  what any authoring save derives from the repository
    bind/ticket.rs       ticket create, ticket save, ticket slug-assign
    bind/document.rs     document create, document save, document move
    bind/remote.rs       remote add, remote select, remote remove
    bind/index.rs        index refresh, index rebuild
    bind/key.rs          the six key commands
    bind/repo.rs         repo create, repo enable
    bind/repo_remove.rs  repo remove
    bind/repo_identity.rs  repo identity-set
    bind/abandon.rs      operation abandon
    bind/sync.rs         item sync, repo sync
    bind/resume.rs       operation resume
    bind/host.rs         host approve, host replace, and observe_host
    bind/folder.rs       folder create
    bind/repair.rs       document repair, ticket repair
src/repository/bridges/
    abandon.rs    abandon_operation and the read a preview of it uses
    identity.rs   set_local_identity and the one write of .git/config
                  under Git's own lock
    folder.rs     create_folder, DocumentFolderPath and the rules for a
                  folder's name
    repair.rs     repair_item, the rules of a repair and the scans for
                  who holds an ID or a path
src/repository/{recovery,remote/state,keys/registry}.rs
                                    + a lookup of one operation by ID in
                                      each journal
src/repository/recovery.rs          + LocalStep: the steps the local
                                      journal records, as one closed enum;
                                      the repair_item action; the one
                                      question of what stands in the way
                                      of a new local operation
src/repository/keys/registry.rs     + key_registry_changes and the three
                                      registry functions that record a
                                      change under an operation ID
src/repository/discovery.rs         + folders recorded by the walk that
                                      collects documents
tests/support/conformance.rs        the rules every bound command owes the
                                    boundary, run over one row per command
tests/support/                      mutation_golden.rs (the registered
                                    golden cases), git_home.rs (a Git
                                    home of the test process's own),
                                    repair.rs (the repair fixture)
src/repository/read/items.rs        + observe_path, list_document_folders
src/repository/read/graph.rs        visibility widened for the cycle check
schemas/v1/                         new and extended schemas
tests/fixtures/mutation_v1/         golden envelopes and previews
tests/fixtures/progress_v1/         golden progress events
```

Unit tests sit beside the file they test, as `*_tests.rs`. The journal
lookups are new because `show_operation` answers only for a registered
repository, and settling must also answer for a root that is not registered
or no longer exists.

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
    pub fn prepare(&self, mutation: &Mutation) -> Envelope<PrepareDataDto>;

    pub fn execute<P: SessionCredentialProvider>(
        &self,
        call: MutationCall,
        credentials: &mut SessionCredentials<P>,
        progress: &mut dyn ProgressSink,
        cancel: &CancellationToken,
    ) -> Envelope<MutationDataDto>;

    pub fn show_request(&self, request_id: RequestId) -> Envelope<RequestDto>;
    pub fn cancel_request(&self, request_id: RequestId) -> Envelope<CancelDto>;
    pub fn observe_host<P: SessionCredentialProvider>(
        &self,
        root: &Path,
        authority: Option<&SshAuthority>,
        credentials: &mut SessionCredentials<P>,
    ) -> Envelope<HostObservationDto>;
}
```

`Mutation` is a closed enum with one variant per bound command, each holding
that command's typed input. The variant fixes the envelope's `command`
string; a preview carries the same command as the execution it previews.
Inside the module each variant has a `Command`, a closed enum whose `name`
is the one place a published command name is written; everything that
depends on which command a request is of is a `match` on it with no
wildcard arm.
`show_request` is `request show`, `cancel_request` is `request cancel`
and `observe_host` is `host observe`. F1's read of the stored pins,
`inspect_host`, keeps the name `host inspect`; the two give different data
and a command name tells them apart.
`operation resume` is not a function of its own: it is the mutation
`Mutation::OperationResume`, run through `execute` like any other, so it
has a request ID, a record and a replay; see [Resuming](#resuming). Every
handled outcome, success or not, comes back as an
envelope; these functions do not return `Result`. `execute` is synchronous
and runs on the caller's thread. A front end that wants it off its interface
thread runs it on a worker of its own; F2 adds no thread, executor or event
framework.

The library requires a request ID on every `execute`. Which caller invents it
is a front-end matter. `prepare` takes none: it changes no canonical or Git
state, and the confirmation ID it returns is how a caller refers to it.

`show_request` returns what the records hold for a request ID: its state,
its command, when it was accepted and finished, its operations with the
journal each is in, and its stored result if finished (outcome, code,
message, effects and data). The list of operations is empty for a request
of a command that has no operation: `key remove`, `repo identity-set`,
`host approve`, `host replace`, an abandon and a resume. For `key import`,
`key select` and `key clear` it holds one operation, of the key journal's
family. Nothing is observed again; the read does not say
what Git or a journal holds now. Its scope names the repository only, and
none for an application-level request. A request ID no record holds is
`request_not_found`. The CLI RFC tells a client that lost its output to
"use their request ID to inspect recovery"; this is that read.

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
| `repo identity-set` | new `set_local_identity` | Required | Name, email, and initials left, cleared or set. No operation. |
| `remote add` | `add_remote` | No | |
| `remote remove` | `remove_remote` | Required | Preview shows publication and polling effects. |
| `remote select` | `set_publication_remote` | No | |
| `key generate` | `generate_shared_key` | No | Digest covers label and protection mode, never the passphrase. |
| `key import` | `register_shared_key_under` | No | The `_under` forms record the change against the request's operation ID in the transaction that makes it. |
| `key select`, `key clear` | `select_shared_key_under`, `clear_shared_key_selection_under` | No | As above. |
| `key remove` | `unregister_shared_key` | No | No operation. |
| `key delete` | `review_generated_key_deletion`, `delete_generated_key` | Required | The persisted preview wraps the existing review. |
| `host approve`, `host replace` | `verify_ssh_transport` | Required | The authority, and the algorithm and fingerprint of the key to pin; a replacement also names the old fingerprint. No operation. |
| `folder create` | new `create_folder` | No | Its operation is the refresh that lists the folder. |
| `document create`, `document save`, `document move` | `save_document` | No | Source observation required for save and move. |
| `document repair`, `ticket repair` | new `repair_item` | Required | The path, its observation token and the corrected source. |
| `ticket create`, `ticket save` | `save_ticket_with` | No | Relationships; short code on create. |
| `ticket slug-assign` | `save_ticket_with` | No | |
| `item sync`, `repo sync` | `synchronize_remote_observed`, the new sibling of `synchronize_remote` | No | `repo sync` synchronizes the primary branch; `item sync` takes an item's kind and ID and synchronizes its editing branch. Clean synchronization only: the bindings set the sibling's refuse-divergence setting, so a divergent remote is `merge_required` and nothing is merged (decision 12). |
| `operation abandon` | new `abandon_operation` | Required | Decision 13; see [Abandoning An Operation](#abandoning-an-operation). |
| `operation resume` | `refresh_repository` or `synchronize_remote_observed`, under the named operation's ID | No | See [Resuming](#resuming). |
| `index refresh`, `index rebuild` | `refresh_repository`, `rebuild_repository` | No | |

That is twenty-eight baseline commands, `operation abandon` and
`operation resume`. The saves prepare their editing context themselves,
under the same operation ID; a binding makes one domain call.

All thirty are built.

Not bound in F2, with the owner:

- `comment add`: the first of C4 or D4, by the product owner's decision of
  2026-10-13. The CLI RFC defines it as one checkpoint followed by
  immediate publication. That compound operation, Wave 02 Cycle 07, is on
  the branch, so the binding no longer waits for its other half; it waits
  for the work below. No Cycle before C4 and D4 submits comments, and
  nothing reaches `submit_comment` but a direct call of the service. F2
  still makes `submit_comment` write the author. What the boundary needs
  from the compound operation before it can bind it:
  - **An observed sibling.** `submit_comment` and
    `retry_comment_publication` call `synchronize_remote`. The boundary's
    synchronizations call `synchronize_remote_observed`, with a
    cancellation token and a progress sink; without a sibling that does
    the same, a binding can neither honor the token nor report progress.
    The operation's own cancellation is by lookup, from another call.
  - **A decision on divergence.** The boundary's synchronizations refuse
    divergence (decision 12). A comment's synchronization merges, and can
    leave a retained conflict that no bound command resolves. Either the
    comment binding refuses divergence too, or it waits for the conflict
    commands.
  - **Which call a retry is.** Once a checkpoint exists a retry is
    `retry_comment_publication`, which takes no body; before it, it is
    `submit_comment`, which needs the body from the caller. A request
    record holds a digest and no body, so a caller that lost its input
    before the checkpoint needs an answer that
    `original_request_required` does not give.
  - **Two IDs for one request.** The record holds the comment's operation
    ID. The synchronization's ID is the compound operation's own, kept in
    `comment_publication_bindings`; the record must not allot or store a
    second. Whether the envelope shows it is a contract question.
  - **`publication_pending`**, in the Incomplete class, with its schema
    tie and golden.
  - **A hand-off-only entry,** if `operation resume` is to finish a
    comment that waits for its registration or its index hand-off without
    publishing; and a rule for when a resume may take a comment's
    synchronization up, which it refuses today in every state.
  - **`repository_not_enabled`** for the retry and the cancellation, which
    on a repository that was never enabled answer `RecoveryRequired`
    today because no binding is found.
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

Every byte string is prefixed with its length as a 64-bit little-endian
integer. The salt is preceded by one byte saying which kind of record it is
for, and each field is its name followed by one byte for the kind of its
value. A list is its count and then each entry with its length. Fields are
the binding's typed inputs in a fixed order, with absent, null and empty
kept distinct. An observation token the caller supplies is a field like any
other.

For a ticket the fields are `title`, `type`, `status`, `project`, `team`,
`body`, `deps`, `parent`, `initials` and `observation`, in that order.
`deps` is taken in the caller's order with its repeats, and the per-call
`initials` is a field: a retry that reorders the one or changes the other
is a different request. A create always gives `deps` as a list, empty when
it has none, and `observation` as absent.

The fields of the other bound commands whose input is more than their
target:

- The three document commands: `title`, `body`, `observation`, `path` and
  `destination_observation`, in that order, with the ones a command does
  not take given as absent. `path` is a move's destination as the caller
  gave it.
- `ticket slug-assign`: `observation` and `initials`.
- `remote add`: `location`, unredacted; it is stored nowhere else.
  `remote select`: `name`, null when the selection is cleared.
- `key generate`: the label and whether protection was asked for.
- `key import`: `label`, `private_key_path` and `public_key_path`, the
  last absent when none is given. The paths are the caller's text. This is
  the one place a path outside a repository reaches a stored digest; see
  the known limits.
- `repo identity-set`: `name`, `email` and `initials`. Initials left as
  they are, cleared and set are absent, null and text, so each is a
  different request. It has no target.
- `host approve` and `host replace`: `algorithm`, `fingerprint` and
  `old_fingerprint`, the last absent for an approval. The target is the
  authority as the caller wrote it, `host:port`, so `LocalHost` and
  `localhost` name one pin and are two requests.
- `folder create`: `path`, which is also its target.
- `document repair` and `ticket repair`: `observation` and `source`. The
  target is the path; the item's ID is read from the source.
- The synchronizations, `operation abandon` and `operation resume` have no
  field: the command, the repository and the target say everything. The
  target of `item sync` is the item's kind and ID, such as `ticket/<id>`,
  and the target of an abandon or a resume is the operation ID.

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

`command` holds the command's published name, `Command::name`. A stored
name that is no bound command's gives no effects on a mismatch, and
`internal_error` on a replay whose record has stored data.

`result_data` holds the finished envelope's `data` as JSON: identifiers,
paths, fingerprints, states and, for `repo identity-set`, the name, email
and initials the local configuration holds once the request is done. No
binding's data carries a body or source.
`expected_digest` is the digest of a canonical file's bytes as they were
before the request; the index RFC's "target observations". For
`remote select` it is the digest of the configuration file. For
`key select`, `key clear` and `key import` it is a digest of the setting
the request acts on, as the first call observed it: the selected key, or
whether the source is registered and under which key. It holds no path,
label, fingerprint or key material. The key registry compares it itself,
in the transaction of the change, and only when the request's operation
has recorded nothing; see "Key registry changes" below. `expected_digest`
is a plain digest and is not salted. `repo identity-set`, `host approve`
and `host replace` record none: what they observed is on the confirmation
the record holds.

**Access.** The boundary reads and writes these tables in short transactions
under the existing cache guard, and releases the guard before it runs a
binding, because a domain call takes the same guard itself. If the database
is unavailable, `execute` returns `index_unavailable`, with one exception:
for `index rebuild`, which is how the database comes back, the boundary
first restores the database and then records the request like any other.
See "Looking the request up" below. No request runs without a record.

`request_operations` holds no row for a request of a command that has no
operation. `cancel_requested` is the durable flag `request cancel` sets;
see [Progress And Cancellation](#progress-and-cancellation).
`observation_digest` holds what a preview observed, as two digests in the
one column: one of the state and one of the effective identity, so that
each can be compared on its own. Only `repo create` and `repo enable`
observe an identity; for the other confirmed commands the second half is
a constant.

**Retention.** Nothing prunes these tables in F2; see decision 8.

**Key registry changes.** A fourth table belongs to the key registry and
not to the boundary. It is created beside the key journal's table, by the
same migration, and an existing database gains it empty when it is opened.

```text
key_registry_changes
    operation_id            TEXT PRIMARY KEY
    action                  'import', 'select' or 'clear'
    key_id                  the key acted on; null for a clearing
    label, public_key_fingerprint
                            what the result gives of the key; null for a
                            clearing
    changed                 1 for a change, 0 for a no-op
    recorded_at
```

`key import`, `key select` and `key clear` are accepted with an operation
of the key journal's family. The boundary calls
`register_shared_key_under`, `select_shared_key_under` and
`clear_shared_key_selection_under`, each of which works in one immediate
transaction:

1. If a row is recorded against the operation ID, it returns that row and
   changes nothing.
2. Otherwise, when the caller hands it the setting it expects, it compares.
   A setting that is neither as expected nor already as the request asks
   has moved, and nothing is changed or recorded.
3. Otherwise it makes the change and inserts the row, also for a no-op.

A first call hands over no expectation: it observed the setting a moment
before. A retry reads the row first and, when there is one, finishes the
request with it and makes no domain call: the same key ID, `ok` or
`already_applied` as recorded. With no row it makes the call with the
record's `expected_digest`. A moved setting is `external_change`, final.

The table holds no path, no key material and nothing made from a
passphrase. The label is the caller's text, which the result returns too.
Nothing prunes it. The public functions `register_shared_key`,
`select_shared_key` and `clear_shared_key_selection` keep their signatures
and record nothing: only a request through the boundary leaves a row.

A recorded change is finished in the transaction that writes it and is
never pending. Its operation ID is in the request's results and in
`request show`. `operation list` does not list it and `operation show`
answers `operation_not_found` for it, since showing it would need a new
published operation action; `operation abandon` refuses it with
`invalid_input`, as any ID of the key journal, and `operation resume`
answers `already_applied`, as for a completed key generation. No result
offers `operation.resume` or `operation.abandon` for it. That
`operation show` does not find it is a known limit the product owner
confirmed on 2026-10-12.

A record of one of the three commands that an earlier build accepted holds
no operation. It is entered as it was accepted: no operation, a null
`operation_id`, and the change made under the compare alone.

### Required outcomes

This table is the contract for replay. The mechanism described after it is
the intended way to meet it. Where the two disagree, the table is right and
the mechanism is changed.

| Sequence of events | Required outcome |
| --- | --- |
| A request finishes; the same request is sent again, at any later time, whatever has happened to the item's content since. | The first result, unchanged, unless it names a commit that can no longer be reached from the recorded branch, or from the checked-out branch once that branch is gone. Then the result is `recovery_required`, and the record is kept with its result. Nothing is written. |
| A request is sent again with different input. | Rejected. Nothing runs. |
| A save commits; the process dies before the result is recorded; retry. | The same effects and that commit. No second commit. |
| The same, with another request having saved the item in between. | The first commit is reported. The later save is still on disk. |
| A save writes its file and dies before the commit; retry with the same body. | The retry commits. One commit. |
| A save dies before writing; retry. | The retry writes and commits. One commit. |
| Either of the two above, with an unrelated commit made to the same branch in between. | The same. |
| Either, with someone else having committed different content to the same file in between. | `external_change`. Nothing is written. This is open case 1. |
| A save that changes nothing; its result is lost; retry. | A no-op. No commit is reported, even if earlier commits hold the same content. |
| A create writes its file and dies; the user's initials or the prefix change; retry. | The retry commits. The short code first written is kept. |
| A request is rejected with nothing written. | Nothing is left: a different request on the repository runs at once, and the same request ID can be used with corrected input. One crash window is a known limit, listed below: the process dying between the rejection and the deletion of its record. |
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

This subsection and the next describe the mechanism as built. Task 7 built
it for `ticket create` and `ticket save`, and the ticket binding is the
worked example throughout; Tasks 8 to 14 bound the other commands on the
same mechanism and added to it where a command needed more. Where a step
is the ticket binding's own, the text says so.
[What changed from the proposal, and known limits](#what-changed-from-the-proposal-and-known-limits)
lists each place where the built mechanism differs from the one this
section first proposed.

Three terms are used throughout.

- A **binding** connects one command to the boundary (`Binding` in
  `mutation/bind/mod.rs`). It names the command, its target and its digest
  fields, and has four steps: `prepare` makes the checks before acceptance,
  `run` makes the domain call of a first call, `reenter` reads what a retry
  needs before its domain call, and `rerun` makes the domain call of a
  retry. It also says, through `ended_without_checkpoint`, what a completed
  journal row with no checkpoint step means for its command; see
  [Re-entering a request](#re-entering-a-request). `TicketBinding` serves
  both ticket commands. A binding of a command that needs confirmation
  also implements `Confirmed`, whose one method, `preview`, is what
  `prepare` on the service calls. Everything a call was given (the sink,
  the token, the credentials, and the confirmation the request runs with)
  reaches a binding through one `CallContext`.
- The **journal row** is where the request's operation stands in its
  journal. `journal_row` returns it as a `JournalRow`, which is one of
  three things. `Absent`: the journal holds no row for the operation ID, or
  holds a local row completed with the step `rejected`. `Pending`: the
  operation has not ended; a remote row whose phase is interrupted or
  failed reads this way. `Final`: the operation has ended. A `Final` row
  carries `kind` (`Completed`, `Cancelled` or `RetainedForInspection`),
  `owes_work` (a local row of a comment that waits for its registration,
  or a remote row whose index hand-off or reconciliation is
  still owed) and `checkpointed`.
- `checkpointed` says the row records that the operation passed the point
  where its work is committed. The steps a local row records are one
  closed enum, `LocalStep`; a stored name that is none of them reads as an
  unlisted step, which is recorded and is no checkpoint. For a local row
  it is true when the last step recorded is `authoring_checkpoint_observed`,
  `authoritative_observed`, `initialization_committed`,
  `publication_committed`, `comment_checkpoint_intent` or
  `comment_registration_pending`. The enum has twenty-one stored names:
  seventeen steps of work and the four that close a row, `error`,
  `completed`, `rejected` and `abandoned`. Three of the seventeen are a
  comment submission's, and how each is classified is under
  [Abandoning An Operation](#abandoning-an-operation). A save records its checkpoint step whether it
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

There is one exception to that last rule. `index rebuild` is how the index
comes back, so when the records cannot be read because the index is
unavailable, the boundary first brings the index back, as the rebuild
itself would, and looks the request up again. The rebuild is then recorded
before its operation begins, like any request; a rebuild that ran with no
record and was killed would leave a pending row nothing could take up
again. A request that would be refused before acceptance is refused before
anything is replaced: a presented confirmation is `not_confirmable` and a
cancelled token is `cancelled`. The restore is the existing replacement of
a corrupt cache. It replaces the file only when the domain's own test
finds it structurally corrupt, tested again under the exclusive cache
guard, and it renames the file aside and does not delete it. If the
records still cannot be read after a restore, the result is
`index_unavailable`, not `busy`: the same request sent again would find
nothing changed.

Whether the call presents a confirmation plays no part in the lookup: a
replay, a mismatch and a re-entry ignore it, and it is not part of the
digest. Only a first call checks it. From then on the record holds the
confirmation, and a later call of the same request is answered from the
record whatever it presents: the same confirmation, another, or none.

A request whose root cannot be resolved to a canonical path matches no
record: if a record holds its request ID the result is `request_mismatch`,
given before any validation; otherwise `prepare` answers it. A record that
no record function could have written is `internal_error` and is left as
it is: one that holds no operation for a command that has one, or an
operation for a command that has none, or stored data that is not of the
command's shape. Nothing is run from such a record; a request that lost
its operation would otherwise be entered again under an operation ID that
is not its own.

**A mismatch** returns `request_mismatch`. Nothing runs and the record is
not changed. The result names no operation and offers no recovery action.
The scope is this call's; the effects, when there are any, are of the
request the record holds.

- Against a **finished** record the result is an input error and every
  effect is `not_requested`. The finished request's own result is what
  `show_request` returns; it is not reported as this call's.
- Against an **accepted** record the boundary reads the journal row
  (`bind::recorded_effects`). If the row is `Absent`, no attempt reached
  the domain, or one did and was rejected leaving nothing: the result is an
  input error with no effects. If the row is
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

  That is the reading for an item write. Each other command says what the
  repository shows of its unfinished request in its own way. A remote
  added or removed, a key file written and a key file deleted are shown
  by no effect: the journal row says whether one was made with work
  remaining, and the result is then `partial` with every effect
  `not_requested`. A synchronization reports the push its row says the
  remote holds. A repair is read as an item write, in the context its
  recorded position names. A refresh, a rebuild, a registration removal
  and a request with no operation report nothing, with one exception: an
  accepted host approval whose host no longer shows what its preview
  showed is taken as a change made with work remaining, and the result
  is `partial`; see [Host approval](#host-approval).

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
`recovery_required` with no effects; it names no operation and carries no
data. The record stays `finished` with its result, and the next call checks
again and gives the stored result once the commit can be reached.

**The first call.** A request no record holds is checked, accepted, run
once and settled, in this order.

1. **The confirmation.** A command that needs none and is presented one is
   `not_confirmable`; one that needs it and presents none is
   `confirmation_required`. A presented confirmation must be this
   request's to use; the checks and their codes are under
   [Two-Phase Confirmation](#two-phase-confirmation). This comes before
   the input is validated, so a confirmed command with invalid input and
   no confirmation is `confirmation_required`.
2. **The binding's `prepare`** validates the input and makes the checks
   that need no lease. A failure answers the request with no record and no
   operation ID. For a confirmed command it also observes the state the
   preview covered, and the boundary refuses the request with
   `external_change` when that is not what the confirmation holds. For
   the two ticket commands, in order:
   - The draft must be writable as a ticket, or the result is
     `invalid_input`.
   - The repository must resolve. An unregistered one is
     `repository_not_registered` with the `index.rebuild` action, unless
     it was never enabled, which is `repository_not_enabled` with no
     action; see [Result codes](#result-codes).
   - The root must be the repository's own root. A linked worktree is
     `not_repository_root`.
   - **A create** checks its relationships
     ([Validation before any write](#validation-before-any-write)); a
     target that is not an ID is `invalid_relationship`. It then applies
     the already-applied rule to an item that already holds the ID at the
     ticket's path: as intended and committed is `already_applied`, as
     intended and not committed is `external_change`, and a ticket that is
     not as intended is `occupied_path`. An item at another path, a
     document, or a file at the path that cannot be read as a ticket is
     left to the domain operation to refuse. Then it composes the short
     code. Its expectation is that no file is at the path.
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
   `cancelled` and has done nothing. This is the boundary's only check of
   the token; after acceptance only a synchronization honors a
   cancellation, at its safe points. A call that re-enters an accepted
   request does not check the token here.
4. **Acceptance** is one transaction (`insert_request`). It inserts the
   record as `accepted` with attempt 1, the scope, command, target, digest,
   recorded position and expected digest, and, for a command that has an
   operation, one operation row holding a new operation ID and its journal
   family. If a record already holds the request ID, nothing is written
   and `execute` looks again. The same transaction marks a presented
   confirmation accepted, conditionally on its not having been accepted
   meanwhile; if another request accepted it in between, the result is
   `confirmation_used` and there is no record. If the insert fails, the
   result is the read's code and there is no record. `prepare` succeeding
   for a request that has no scope key or digest is `internal_error` with
   no record. The `accepted` progress event is sent once the record is
   written.
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
it `finished`, delete it, or leave it `accepted`. One function writes a
finished result (`RepositoryService::finish` in `settle.rs`). It stores
the answer's data, derives the outcome from the code and the effects as an
envelope's is derived, and finishes the record only if it is still
`accepted` with the call's attempt. A call's own settlement, a resume that
finishes its owner's record and a fenced synchronization's final answer all
go through it, so no binding chooses an outcome. It says whether the
record was finished.

Settling is decided from the standing and the journal row, never from the
kind of error the domain returned: the domain returns the same kinds before
and after an effect, and returns some rejections as successful values.

| Standing | When a binding returns it | The record |
| --- | --- | --- |
| `Final` | The request's end state holds: a save returned and its hand-off is complete; a retry found the request done; a retry was answered by the already-applied rule. Also an answer that is final although the work is not done: a fenced synchronization, and a synchronization cancelled with its push not confirmed. | Stored and marked `finished`. |
| `Owed` | The domain returned a result and the index hand-off is still owed (`discovery_pending`). | Left `accepted`. A retry continues it. |
| `Unconfirmed` | Git could not be read to say what the request committed. The result claims no effect and no commit. | Left `accepted`. Nothing is stored and nothing is deleted. |
| `Stopped`, journal row not read | The request stopped and the journal lookup failed. | Left `accepted`. |
| `Stopped`, row `Final` with kind `Cancelled` or `RetainedForInspection` | A result the domain has made final for the operation ID: a synchronization that was cancelled, a key generation whose files were retained for inspection. | Stored and marked `finished`, whether or not the row owes work. |
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
  has since entered. Such a call says so wherever a record still exists,
  `accepted` or finished by the other call: its result is what it found
  or did, unchanged, and offers `request.retry` first, so that the caller
  reads the answer the record holds, which can differ from this call's
  own. Where the other call has deleted the record, nothing is offered
  for that.
- **A failed journal lookup leaves the record as it is.** After a first
  call's domain error the effects are then read as if the row were in
  flight. On a retry, before any domain call, the result is the read's
  code and no domain call is made. A local row whose state or step is not
  a stored name fails the lookup with `internal_error` and is handled the
  same way.
- **A record change that fails leaves the record as it is.** The caller
  still gets its result, which offers `request.retry` first, also when its
  code is `ok` or another success: it is not the request's last word, and
  a front end that stops at the outcome would leave the record `accepted`.
  The next call of the request re-enters the record. A result whose data
  cannot be serialized is not stored either.

A result given after acceptance always names the request's operation ID,
also when the record is then deleted. A request ID that is used again after
its record was deleted or released is accepted with a new operation ID; the
old journal row is left as it is. A mismatch, and a replay answered
`recovery_required`, name no operation.

**Could not read Git.** A failure to read Git is never taken for "no
commit". When the domain returned a result and the evidence could not be
read, the result is `internal_error` with no effects and no commit, and the
record stays `accepted` for a retry to settle. A no-op stored at that point
would be replayed for good for a save that may have committed.

**Recovery actions.** A recovery action a failed read suggested for itself
is kept. Otherwise the action follows the code, decided in one `match` over
every result code with no wildcard arm (`outcome::recovery`).
The function is told the command, the repository the result names, the
request ID, the operation ID, whether the settlement left the record
`accepted` (standing `Owed`, `Unconfirmed`, `NotRun` with a row, or
`Stopped` with a row in flight or unread), and whether a finish or delete
of the record did not apply while a record of the request still exists. Four steps
follow it. A result that says a host is not trusted, and whose data names
the host, is given `host.approve` or `host.replace` first. An
`operation.resume` is kept only where a resume would take the operation
up. A `request.cancel` is kept only while the repository's reservation is
the request's own operation. `operation.abandon` is added last, where a
pending local operation stands in the way. The same file decides all of
it: `outcome::offered` from the answer and `RepositoryService::offering`
from what the journal holds once the call is over; the executor decides
none of it. The table under [Result codes](#result-codes) gives the
action for each code. No result is given as final while a record of its
request is unsettled. A call that cannot enter its accepted record
answers as for a record left `accepted`, so its code's rule offers the
same request. A call that cannot release its record to run as new, and
one whose finish or delete did not apply while a record of the request
still stands, a record another call finished included, is offered
`request.retry`, first, whatever else the code offers. The call's answer
is not compared with the stored one: a call beside another that finished
the record always offers `request.retry`, also where the two answers are
the same, and the retry is then harmless.

**A caller should take `request.retry` automatically.** A `busy` answer,
and a `partial` answer that offers `request.retry`, are not the request's
last word, and sending the same request again is always safe. Two callers
that race, such as an agent and a person acting at once, can each be
answered `busy` part-way: one shape is `partial` with the commit named
and `discovery: pending`, when the index hand-off was busy after the
commit; the other is `partial` with `write: written`,
`checkpoint: pending` and no commit, when the other caller held the lease
between the write and the commit and then made the commit itself. Both
are true, and the retry answers `ok`. The product owner accepted this on
2026-10-12 on the understanding that a front end retries without asking.

A command with no journal is a single step that either happened or did
not, and its request is accepted with no operation: its record holds none
and its results carry a null `operation_id`. `key remove`,
`repo identity-set`, `host approve` and `host replace` are built this
way, and so are `operation abandon` and `operation resume`, whose
operation is another request's. Nothing is ever in flight under such a
request, so a stop of the call that accepted it deletes its record, and a
re-run is idempotent. The exceptions are stops whose binding asks for the
record to be kept:

- an abandon that found a call at work and answered `busy`;
- a call that could not read what it needed;
- a retry of `repo identity-set`, `host approve` or `host replace` that
  fails. Only the call that accepted the request releases it on a
  failure; a later call that fails leaves the record `accepted` and
  offers `request.retry`. A deleted record is what would turn a transient
  failure into a write over a later choice. `external_change` and
  `host_mismatch` release the record at any attempt: nothing was written
  and the same request does not get past either;
- a host approval that stored the pin and then failed; see
  [Host approval](#host-approval).

`key import`, `key select` and `key clear` have an operation and follow
the same rule for a failure: a first call that is refused with nothing
recorded deletes its record, and a retry that fails with nothing recorded
leaves it `accepted` and offers `request.retry`. A refusal that will never
change, such as `key_not_found` for a key removed since, therefore holds
its record for as long as the request is retried; the way out is a new
request ID. `folder create` has an operation too, the refresh's, and a
retry of it that stops with no row keeps its record in the same way.

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
first known limit below covers the case where it does not. Or the second
arrives while the first is indexing after its commit: it leaves the row
as it is and answers `discovery_pending` with the commit; see the
liveness rule under [Abandoning An Operation](#abandoning-an-operation).

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
records the context's branch name and primary's head. `remote select`
records the checked-out branch and its tip. `repo create` and
`repo enable` record the primary branch, with its head commit for an
enablement on a branch that has one and no commit otherwise. A
synchronization, and a resume that takes one up, record the branch that
is synchronized. A command that commits nothing records no position. The
position only bounds where the evidence is read.

**The evidence** is what Git shows of the request's own path since the
recorded position (`PathEvidence`, read by `evidence::path_evidence`). For
a ticket or a document the path is the item's file; for a move it is the
destination, with the source read as a second path that must hold no
file; for `remote select` it is the configuration file. `repo create` and
`repo enable` do not use this evidence; they find their own
initialization commit. These terms describe it:

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
  newest change is not intended. There is one exemption, and it turns on
  whether an earlier attempt of the request may have committed (step 6
  below). For a request that cannot have committed, the path is not
  superseded while the branch's tip still holds the expected state at the
  path, whatever commits of the intended content the range holds: none of
  them is the request's, and nothing foreign stands at the path. For a
  request that may have committed, the path is not superseded while the
  tip holds the expected state and there is no own commit: what the range
  shows is history older than the request. When such a request has an own
  commit and the tip is back at the expected state, someone undid that
  commit, and the path is superseded.

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

**The steps.** `reenter` in `mutation/reenter.rs` runs steps 1 to 3 and 9
and calls the binding for steps 4 to 8, in this order.

1. **Raise the attempt** (`enter_request`). This call is now the only one
   that may settle the record. If the record is no longer `accepted`,
   `execute` looks again and answers from what it finds. If the attempt
   cannot be raised, the result is the read's code and the record stays
   `accepted`. A record with no operation is `internal_error`; it stays
   `accepted` with its attempt raised.
2. **Read the journal row** of the recorded operation ID, before anything
   else is done under that ID. The row's state after a domain call proves
   nothing about an earlier attempt: a repeat resets a `rejected` row
   before the domain runs. If the lookup fails, the result is the read's
   code and the record is left as it is.
3. **Release the record and run the request as new** when the row is
   `Final` with kind `Completed`, owes no work and is not `checkpointed`,
   and the binding answers `DidNothing` for such a row (`Reentry::of`).
   Every binding must say what such a row means for its command, through
   the required method `ended_without_checkpoint`, which has no default.
   `DidNothing` means that an operation of this kind that ended without a
   checkpoint step committed nothing. That is true of an authoring write,
   which records its checkpoint step on every row that reached its commit;
   ticket create and ticket save answer it. The earlier attempt stopped,
   for want of an identity for example, with nothing in flight. Had its
   answer not been lost, settling would have deleted the record. The
   boundary deletes it now, conditionally on this call's attempt, and
   releases its confirmation in the same transaction. `execute` looks
   again, finds no record and makes a first call: every check of
   `prepare`, the token check and the already-applied rule among them, and
   a new operation ID. The old journal row is left alone and no domain
   call is made under it. The remaining steps do not apply. If the delete
   fails, the result is the read's code and the record stays `accepted`.

   The file can still be written and uncommitted under such a row: an
   earlier attempt wrote it and did not commit, and a later attempt was
   stopped for want of an identity or cleanly rejected, which completed
   the row. The released request is then answered `external_change` by
   the already-applied rule, and a fresh read and save commits the file.

   The authoring writes answer `DidNothing`: the ticket and document
   commands, and `remote select`, which commits the configuration file as
   a save commits an item. A selection has no token of its caller's, so
   its binding is first asked whether the configuration file has moved
   since the request was accepted. It has not moved while the file is the
   one the request was accepted against, and a file that already names the
   requested remote is the request's end state, whoever wrote it. Any
   other file is a choice made since: the request is finished as
   `external_change` with nothing committed over it, and its record is
   not released. A file that cannot be read says neither: the result is
   the read's code with `request.retry` and the record stays `accepted`.
   The same holds for a selection entered again under a row that is not
   in flight and found `external_change`: it is finished and not deleted,
   so the same request sent once more cannot run as new over the later
   choice. Every other binding answers `SaysNothing`: its
   completed row has no checkpoint step whatever it did, as for an index
   refresh, `remote add`, a key command, a synchronization, a folder
   creation, a confirmed command (a repair among them, although it is an
   authoring write), an abandon or a resume. Its record is not released, and the
   row goes on to the binding's `reenter` like any other final row.
   Releasing it would run completed work again after lost output and send
   a confirmed command back through its confirmation. What each such
   binding takes as evidence that its work is done is in the table at the
   end of this subsection.

   Steps 4 to 8 are the ticket binding's, and the reading of the journal
   row in steps 6 and 8 (`Before::committing`, `Before::found`,
   `Before::reported`) is the authoring reading: a row that is not
   `checkpointed` means no commit is the request's. A `SaysNothing`
   binding cannot use that reading. It supplies its own evidence that its
   work is done and answers from `reenter` with standing `Final`.
4. **Set up the domain call** without `prepare`'s checks (`resume`). The
   relationships are checked again. A rejection (`relationship_cycle` or
   `invalid_relationship`) answers the request with no domain call, unless
   a commit of the request's own already holds the content it intends; the
   domain checks no cycle, and another request may have closed one since
   the first call. When the rejection stands and the operation has no
   journal row, the record is deleted and nothing is left: no branch,
   worktree, row or record, and no action is offered. When the row is
   pending, the record stays `accepted`, as for any re-entry that stops
   before its domain call with a row in flight. A check that cannot be
   read stops the retry, as it stops a first call: the result is the
   read's code with `request.retry`, the record stays `accepted` and no
   domain call is made, unless a commit of the request's own already holds
   the content it intends. A create passes
   the short code the file in the editing context holds, if it holds one,
   and composes one only if it does not: the initials or the prefix may
   have changed to something no short code can be composed from. If this
   step fails, the result is its code with standing `NotRun`.
5. **Read the evidence** as it is before the call. If Git cannot be read,
   the result is `internal_error` with standing `Unconfirmed` and no
   domain call is made.
6. **Decide before any call** (`Before::found`). An earlier attempt **may
   have committed** when the journal row read at step 2 is `Pending`, or
   `Final` and `checkpointed` (`Before::committing`). The own commit
   counts, for Done, for Foreign and for the exemption in "superseded",
   only when an earlier attempt may have committed. A request that cannot
   have committed is therefore not refused for commits of its intended
   content that someone else made and then undid: the tip holds the
   expected state, and the domain operation is called. The three answers
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
     the way out; the result offers it.
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
     finished with `ok` and that commit. If the row has completed and the
     evidence cannot be read, the result is the domain's code with no
     effects and standing `Unconfirmed`. Otherwise the effects are read as
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
ID, and what the binding built for it does on a retry.

| Operation | On replay after completing | Binding |
| --- | --- | --- |
| `save_ticket`, `save_document` | Returns saved with no change when the file is as intended; writes and commits again when the file is back at the recorded expectation; runs the hand-off if owed. With no identity it returns "identity required" and completes its row. | Not called when the earlier attempt ended before its checkpoint (the binding answers `DidNothing`, so the record is released and the request runs as new), when the request is done, or when the path is superseded. Otherwise called with the recorded ID and expectation; the commit is the one the call reports from the evidence read before and after. `save_document` is bound the same way for create and save. A move changes two paths in one commit: its commit is judged at the destination, a commit that still holds a file at the source is no candidate, and a commit from elsewhere to either path stops a move in flight. Repeated after its checkpoint with the original expectation, `save_document` refuses a move that committed as an external change; the binding works around that, and it is filed as ticket `01M4PB80T7SVB23TQ6466RDHCB`. |
| `enable` | Returns already enabled; completes registration. | `SaysNothing`. The binding first asks whether the end state holds; see [Two-Phase Confirmation](#two-phase-confirmation). Otherwise it calls `enable` under the recorded ID. A commit is named only when the operation recorded that it wrote the configuration and the commit that put the configuration file on the branch holds what the request wrote. |
| `create_and_enable` | Fails: it rejects the now non-empty directory. | On a retry, once the root holds a repository, calls `enable` with the recorded ID, which the journal accepts as the same action. |
| `set_publication_remote` | Returns changed with the current head whenever the configuration equals the request, whoever made the commit. | An authoring write of the configuration file, bound like a save: `DidNothing`, with the position recorded as the checked-out branch and its tip and the expectation as the digest of the configuration file. The commit is taken from the evidence, never from this outcome, and none is claimed when the row ended without its checkpoint step. With no identity the domain returns an identity error, which the binding reports as `identity_required`. A selection has no caller's token to guard a run as new, so a retry first asks whether the configuration file has moved since acceptance; where it has, the request is `external_change`, final, and commits nothing over the later choice. A completed row with no commit step releases the record only when the file has not moved. |
| `add_remote` | Returns no change. | `SaysNothing`. A completed row whose recorded step is `remote_changed` is the request's own work: `ok`, also after lost output. Otherwise Git decides: a remote of the name with the same location is `already_applied`; one with another location is `remote_name_conflict` and the record is deleted; with none, the domain is called. |
| `refresh_repository`, `rebuild_repository` | Completes the row. | `SaysNothing`. A request whose row has completed is answered `ok` from the row; the domain is called again only for a row that has not. |
| `remove_registration` | Deletes every journal row for the repository, its own included, when it succeeds; returns not registered afterwards. | With no journal row, a root that is not registered, or is registered under another registration than the previewed one, means the consented registration is gone: `already_applied`, final. |
| `remove_remote` | Deletes whatever remote has the name, also under a row that already recorded its removal. | A completed removal whose result was lost is `ok`. Under a row that recorded the removal, a remote of the name that is there again was added by someone else: `external_change`, the record is kept and the new remote is not touched. |
| `generate_shared_key` | Returns already created. After an interruption, a key pair that was written is verified and registered on the next call; one that was not is retained for inspection, and that is final. | A completed generation row is a registered key: the record is finished as `ok` from the row, naming the key, whatever has happened to the key since. Otherwise the domain is called; "already created" under the request's own operation ID is `ok`. "Retained for inspection" finishes the record. An error on a retry that finds no row leaves the record `accepted` and does not delete it, because deleting it could let a second key be made. |
| `delete_generated_key` | Returns already deleted. After an interruption it completes under the same operation ID with a fresh review; a missing file is skipped. | With the row completed, calls it with no review: `already_applied`. With the row pending, calls it with a fresh review and compares nothing. With no row, observes again and compares with the preview. |
| `synchronize_remote_observed` | Replays its recorded outcome, doing the index hand-off the row still owes; says of a cancelled row that it was. | Every re-entry makes the domain call, except for a fenced row. A result is `cancelled` only when the row's phase is cancelled. A call of the synchronization's own request (`run`, `rerun`) that ends with an error while the operation's row has completed leaves the record `accepted`, standing `Unconfirmed`, and offers `request.retry`; its answer says `discovery: current` when the row owes no work; the next call replays the row, `ok` with the published commit, stored and given again identically. A first call that comes back fenced after a retry finished the operation offers `request.retry`, then `repo.inspect`. `operation resume` taking up a synchronization is unchanged. |
| `synchronize_remote_observed`, interrupted | Returns recovery required unless `restart` is set. | Sets `restart` on every re-entry. A restart asks the remote what it holds before it pushes anything, and fences any executor still running under that ID, which is Wave 02's own takeover and pushes nothing twice. |
| `register_shared_key_under`, `select_shared_key_under`, `clear_shared_key_selection_under` | Returns the change recorded against the operation ID and changes nothing. | `SaysNothing`. A retry that finds a recorded change answers the first call's result from it, with no domain call. With nothing recorded the request never ran: the domain is called with the digest of the setting as the first call observed it, and makes the change when the setting is still as it was then, or already as the request asks; a setting that is neither was changed by someone since, and the retry is `external_change`, final, with nothing changed. A retry that fails with nothing recorded keeps its record. |
| `unregister_shared_key` | No operation ID. A key's ID is a new ULID for each registration and is never registered again, so a removal that finds the key gone has nothing to do and nothing it could undo. | Run again: `already_applied` when the key is gone. |
| `set_local_identity` | No operation ID. Equal values are a no-op. | Entered again through `confirm::reenter`, which asks the configuration: it holds what the request asks, so the request is finished as a no-op, `already_applied`, whoever wrote it; it holds what was previewed, so the domain is called; anything else is `external_change`, the record is released and `request.prepare` is offered. A retry that fails keeps its record. |
| `verify_ssh_transport` with an approval | No operation ID. An approval of the key the host is already pinned to changes nothing. | Entered again through `confirm::reenter`, which asks the pin store: the host is pinned to the key the request names, so the request is finished as a no-op with no connection; the pin and the remote are what was previewed, so the host is connected to; anything else is `external_change` with no connection, and the record is released. A retry that fails keeps its record. |
| `create_folder` | A directory that is there is found, not made, and the refresh is taken up under the same ID. | `SaysNothing`. A completed refresh row is the request's own work: `already_applied` with `discovery: current`, answered from the row with no domain call, so a folder removed since is not made again and a file put in its place is not `occupied_path`. With no row or a pending one the domain is called again. A retry that stops with no row keeps its record. |
| `repair_item` | As a save: no change when the context holds the source committed; writes and commits what remains; runs the hand-off if owed. | Confirmed, `SaysNothing`, entered again through `confirm::reenter` with a save's evidence; see [Repair and adoption](#repair-and-adoption). |

### What changed from the proposal, and known limits

The mechanism this section first proposed was changed in the places below
while Task 7 was built and reviewed, and at the design checkpoint that
followed it. Each change was made because a sequence of events gave a
result the [Required outcomes](#required-outcomes) table forbids, or
because a ruling decided a point the proposal left open. Row numbers count
the rows of that table from the top.

| Place | The proposal's rule | What showed it wrong, or the ruling | The rule as built |
| --- | --- | --- | --- |
| Whether a retry calls the domain | Always: only the domain completes its row and hand-off. | A save commits and its output is lost. Someone edits the file and does not commit. The retry's domain call is refused as an external change, the row has completed, so the record is deleted and the caller never learns its commit. Row 3. | No domain call when the row has completed, owes nothing and the own commit is in range. The record is finished with that commit. |
| Order of the checks before the call | A foreign change is looked for first. | A save commits and its output is lost. Another request saves other content. The retry is `external_change`. Row 4 requires the first commit. | Done is decided before foreign. |
| The commit rule: the journal row before the call | An earlier attempt "had started" when its row was pending or completed with any step but `rejected`. | A save stops for want of an identity and its output is lost. An identity is set and another request commits the same content. The retry reports that commit. Rows 9 and 11. | An earlier attempt may have committed only when its row was `Pending`, or `Final` and `checkpointed`. `JournalRow::Final` gained `checkpointed` for this. |
| The commit rule: the state the commit was made from | Any commit in range that left the path as intended. | A save changes nothing and its output is lost. A second request saves other content and a third saves the first content again. The retry reports the third request's commit. Row 9. | When the file was committed at acceptance, a candidate's first parent must hold the expected state at the path. |
| The commit rule: a request with nothing to commit | Not considered. | A save changes nothing and its output is lost. Another request, from the same state, commits a change to a field the first does not set, such as `deps`. That commit is from the expected state and leaves every field the first sets as intended. Row 9. | A request settled at acceptance has no candidates. |
| The commit rule: a widened range | When the recorded commit is not an ancestor, the whole branch is searched and treated like any range. | The branch is amended or reset under a request in flight. The foreign-change check then refuses the request for history older than itself, and the repository stays blocked. "Done" can also pick an old commit of the same content. | A widened range gives no own commit. The path is not superseded while the tip holds the expected state and there is no own commit that counts, which is one an earlier attempt may have made. A revert of the own commit by someone else is still superseded. |
| The commit rule: which commit | Not said. | Decided in the build: a later commit of the same content was made over a path that already held it, or over someone else's change. | The oldest candidate. |
| The first call's commit | Taken from the evidence check after the domain call. | A save that changes nothing, whose refresh mark fails, names the head it found as its commit. | The commit the domain names is kept only if it is in range and changed the path. |
| The answer of the evidence check | Yes or no. The Settling table had no row for a check that could not read Git. | A Git read error after a save that committed was read as "no commit", and the save was finished as a no-op for good. Row 3. | Three answers: `Claim::Confirmed`, `NotThisRequests`, `Unreadable`. "Could not tell" gives `internal_error` with no effects and leaves the record `accepted` (standing `Unconfirmed`). |
| Mismatch against a finished record | `request_mismatch`, carrying the recorded effects. | Ruling: an error envelope may not carry a durable effect, and the effects describe what this call did, which is nothing. | An input error with no effects. `show_request` returns the finished result. |
| Mismatch against an accepted record | `partial` if the recorded request made a durable effect. An accepted record stores no effects, so there was nothing to read them from. | A request that committed and was not settled was reported as a plain input error. A request that never created its context was reported as `write: written`. | When the journal row is anything but `Absent`, or cannot be read, the effects are read from Git and the editing context, as described under a mismatch. |
| A foreign change over the request's own commit | `external_change`; nothing said of the effects. | Ruling: the request did commit, and saying so is truthful. | `partial`, `external_change`, with the own commit. No recovery action. |
| A retry whose earlier attempt ended before its checkpoint | Re-entered like any other, under the recorded operation ID. | A save stops for want of an identity and its output is lost. The retry calls the domain, which records the checkpoint step although nothing changed, and its output is lost too. Another request has committed the same content. The next retry reports that commit. Rows 9 and 11. | The record is released and the request runs as new, with a new operation ID. Task 7c made this each binding's choice: the rule first lived in the boundary for every command, where it would have run a completed index refresh again and sent a completed confirmed command back through confirmation, against row 17. Only a binding that answers `DidNothing` is released. |
| A retry refused after another call of the request committed | An error with a completed row deletes the record. | Call 1 is accepted. Call 2 enters and reads no row and no commit. Call 1 runs to its end. Call 2's domain call is refused. The record is deleted and the request, which committed, is answered as new. Row 13. | If the row has completed, owes nothing, and the call reports a commit, the record is finished with that commit. |
| A retry that stops before its domain call | Settled like any stop: deleted when nothing is in flight. | Decided in the build: the retry could not resolve the repository, and deleting the record would lose the report of a commit an earlier attempt made. | Standing `NotRun`: deleted only when no attempt had started; left `accepted` otherwise, also when the row has completed. |
| `partial` after an error | `write: written`, `checkpoint: pending` when the file in the worktree is what the request intended. | A save that would have changed nothing also has such a file. | The file must also differ from what it was before the request. |
| `discovery` on an uncommitted write | The proposal names only `write` and `checkpoint`. | Ruling: discovery is work the retry still owes. | `discovery: pending` alongside `written` and `checkpoint: pending`. |
| `request.retry` | The recovery table lists it for named codes only. | Ruling: the same request is what finishes a record left `accepted`. `external_change` on an item write stays without an action, as the table says. | Offered on any result whose record the settlement decided to leave `accepted`, unless the code has an action of its own or is `external_change`. A result whose record is unsettled, because a record read or change failed or did not apply, is offered it first. |
| The short code on a create's retry | Composed again, as on the first attempt. | A create writes its file and dies. The initials change to something no short code can be composed from. The retry is `initials_required`. Row 10. | `resume` passes the short code the file holds and composes one only when the file holds none. |
| The journal lookup | Absent, pending with its step, or completed. | Ruling in Task 6: a completed or cancelled remote row that still owes its index hand-off or a reconciliation must not release its record, and settling must know which way a row ended. | A typed `JournalRow` with `kind` and `owes_work` as separate facts. |
| The lookup loop | Not said. | Decided in the build: a call can lose the insert race, and a re-entry can release its record. | At most three looks, then `busy`. |
| Replay of a finished request | Row 1 of the table as first written: the stored result, whatever has happened to the item since. | The built replay refuses a result whose commit can no longer be reached, and the review of the code against the table found the two in disagreement. The approved Cycle document's exit criterion 4 decides it: "A stored result that names a commit no longer reachable is not returned." | The table's first row was narrowed to agree with the Cycle document. The replay checks `still_reachable` first; an unreachable commit is `recovery_required`, nothing is written and the record is kept. |
| The tip-as-expected exemption | The exemption asked for any own commit in the evidence, whatever the journal row said. | A request is accepted and never reaches the domain. Someone commits the intended content from the expected state and then commits the expected state back. The retry was `external_change` although nothing foreign stood at the path. | The exemption counts the own commit only when an earlier attempt may have committed, as step 6 does. |

The places below were changed while Tasks 8 to 14 and 12A were built.

| Place | The design's rule | What showed it wrong, or the ruling | The rule as built |
| --- | --- | --- | --- |
| `index rebuild` on an unavailable index | Runs without a record. | A rebuild that ran unrecorded and was killed left a pending row whose only exit was deleting the database by hand, against the Cycle's exit criterion 3. Confirmed by the product owner on 2026-10-11. | The boundary restores the index first and records the rebuild before its operation begins. No request runs without a record. |
| `observe_path` | A token for any repository-relative path. | Ruling, confirmed by the product owner: a digest of any file under `.git/` serves no command. Its command name is `path observe`. | The paths `show_path` reads, and no others. |
| `ticket slug-assign` as a new request | `slug_already_assigned` for a ticket with any slug. | Ruling, confirmed by the product owner: once the record is gone, a no-op is the truthful answer to a retry of an assignment that succeeded. | `already_applied` when the ticket holds exactly the slug this request would compose and the file is committed. |
| A request with no operation | Every accepted record holds one operation. | A key removal, an abandon, a resume, `repo identity-set` and the two host commands have no journal row of their own. | A request may be accepted with no operation; its results carry a null `operation_id`. |
| A retry of a key generation that finds no row | Settled like any stop: the record is deleted. | Two calls of one `key generate` at once could each make a key. | On a retry, an error with no row leaves the record `accepted`. |
| The observation of a confirmed command | One digest. | A stepped retry of `repo create` could commit under an identity the preview never showed. | Two digests, of the state and of the identity. The identity is compared on a retry until the step `initialization_committed` is recorded. |
| `external_change` on a confirmed command whose record stays `accepted` | `request.prepare`. | A new preview is a dead end while the request's own pending row refuses a new request. | `repo.inspect`, `operation.resume` for each other pending local operation a resume would take up now, and `operation.abandon`. A key deletion refused for a key file that is not the one Manyhands wrote offers no action. |
| `remote remove` | The preview shows publication and polling effects. | A confirmed removal of a mirror remote left a repository with no references; see the known limits and ticket `01M4PB7WZN9WNAQ3KXZ19J28KM`. | The binding observes the remote's refspecs and refuses a removal that would take what the preview cannot name. |
| `operation.abandon` | Offered on `recovery_required` when a pending local operation is the cause. | Ruling: a request stuck behind its own pending row must always be shown a way out; a stranded enablement's retry and an overtaken save's retry were shown none. | Also offered on any failure that left the request's own local operation pending. |
| Abandoning a live operation | The preview binds the row's state and step. | A running rebuild could be abandoned and then reopen its row. | Every journal write refuses an abandoned row, and a heartbeat makes the abandon of a live rebuild `busy`. |
| `operation resume` | A function of its own, `resume_operation`. | Decided in the build: a resume needs a request ID, a record and a replay like any mutation. | A bound command run through `execute`. |
| What a resume takes up | A local row whose authoritative step is recorded. | An enablement and a remote selection record their commit step before they bring the registration up to it. | The step and a hand-off state are both required. `operation.resume` is offered only where a resume would take the operation up. |
| `original_request_required` | For anything a resume does not take up. | It was a dead end where no request could be sent again. | Only while an unfinished record holds the operation; otherwise the answer says what the row is. |
| A resume and the owning request | Not said. | A resume pushed after the owning request had been asked to stop, and a stale call of the owner could settle the owner's record. | A resume reads the owner's cancel flag, raises the owner's attempt at takeover, and finishes the owner's record with a final answer. |
| Honoring a cancellation | At any safe point; the flag is cleared when it is honored. | A cancellation honored as the push returned stored "nothing published" for a push the server took; a flag cleared before the operation recorded the cancellation was lost by a crash. | The rules under [Progress And Cancellation](#progress-and-cancellation): a push is never understated, and the flag is durable until the row reads cancelled. |
| A synchronization whose configuration changed under it | `external_change`, offering the same request. | The domain takes a fenced operation up for nobody, so the request could never finish. | Three final answers by how far the push had got; the record is finished. |
| A retry after a lost push when the remote has moved on | Not considered. | The retry opened a publication attempt and an integration window under the refuse-divergence setting. | Refused as `merge_required` before either is recorded. |
| `merge_required`, `remote_branch_deleted` | `request.retry`. | The same request cannot succeed after the natural fix. | No action at first. Confirmed by the product owner on 2026-10-12: `request.cancel`, while the request holds the reservation. |
| `HostVerificationUnavailable` | Not mapped. | Confirmed by the product owner on 2026-10-11: a transient code would invite retrying forever, and there is no key to approve. | `recovery_required`. |

The places below were changed in Tasks 14B and 14C. The product owner's
rulings are dated 2026-10-12.

| Place | The design's rule | What showed it wrong, or the ruling | The rule as built |
| --- | --- | --- | --- |
| A repository that was never enabled | An unregistered repository is `repository_not_registered`, offering `index.rebuild`, which registers it. | Ruling: reject a repository that was never enabled, and do not enable it automatically. | `repository_not_enabled`, no action, and a rebuild refuses it; see [Result codes](#result-codes). A registered repository keeps the answers it had. |
| A settings command retried late | `key select`, `key clear`, `key import` and `remote select` are made again over whatever someone else changed since. | A retry could put an older choice over a later one. Ruling: refuse a late retry. | `external_change`, final, when the setting is neither as the first call observed it nor as the request asks. The record holds the observation in `expected_digest`. For the three key commands the compare is made only when the request's operation recorded nothing. |
| A retried ticket write whose relationship check cannot be read | The retry goes on. | Another request may have closed a cycle since the first call, and the domain checks none. | The retry stops with the read's code and `request.retry`; the record stays `accepted`. |
| A settlement that did not apply | The caller gets the result, final by its code. | A finish or delete that did not apply leaves the record `accepted`, and the result looked like the request's last word. | The result offers `request.retry`, first. |
| The discovery effect of `remote add`, `remote remove` and a kept synchronization answer | `not_requested` for every refusal or error; `pending` for a kept answer of a published synchronization. | An error after the remote changed owes the hand-off, a retry under a pending row ran it, and a completed row that owes nothing had run it. | `pending` after the change, `current` after the hand-off ran. |
| One owner of each rule the bindings share | Not said. | Ruling: consolidate the boundary first, then bind Part C. | One writer of a finished result (`RepositoryService::finish`), one function that enters a confirmed command again (`confirm::reenter`), recovery actions decided in `outcome.rs`, and the local journal's steps as one enum. The behaviour is unchanged. |

The places below were changed in Tasks 14D to 19 and in the fix pass that
followed the whole-Cycle reviews. The product owner's rulings are dated
2026-10-12 and 2026-10-13.

| Place | The design's rule | What showed it wrong, or the ruling | The rule as built |
| --- | --- | --- | --- |
| Evidence of a key registry change | None: `key import`, `key select` and `key clear` have no operation. | A lost-result import followed by a removal registered the source again, and a setting changed and put back was changed again by a late retry. Ruling: record each change now. | The table `key_registry_changes`, written in the transaction of the change; a retry answers from the row. |
| A retry of a no-journal command that fails | A stop deletes the record. | A deleted record turned a transient failure into a write over a later choice, and orphaned a first call still running. | Only the call that accepted the request releases it; a later call that fails keeps the record and offers `request.retry`. |
| The identity write | Each key is written in turn; a failure restores the file's bytes. | The restore was a rename outside Git's lock and could erase another process's change. | One write under `.git/config.lock`, renamed over the file; nothing is restored. |
| What an identity may hold | An email holds an `@`. | Values Git strips or no commit can carry were accepted, and later saves failed. | Validation by round trip through Git. |
| A linked worktree root given to `repo identity-set` | `invalid_path`. | Every other command answers `not_repository_root`; one situation has one code. | `not_repository_root`. |
| Input of `host approve` and `host replace` | The authority and the fingerprint. | The approval compares a whole key identity; a fingerprint alone would need a second connection. | The algorithm as well. |
| The observation's command name | `host inspect`, shared with F1's pin read. | Two closed shapes under one name could not be told apart. | `host observe`. |
| Names of a host's keys | Three names for the pin and two meanings of `presented_fingerprint`. | Product owner, 2026-10-13: unify the names now. | `presented_*`, `pinned_*`, and `fingerprint` with `algorithm` for the key a request names. |
| A host approval that fails after the pin is stored | Not considered. | The answer said nothing was written. | `partial`, the record kept, `request.retry`; the retry is a no-op. |
| `folder create` on a folder that is there | A no-op. | One path for first calls and retries, and the folder is listed after any success. Approved by the product owner on 2026-10-13. | A no-op that still runs the refresh and reports `discovery: current`. |
| A folder creation whose refresh fails | `partial` with `index.refresh`. | A pending refresh row would refuse every other refresh. | The row is closed as rejected and the record finished. |
| A folder path that is not valid as one | `invalid_input`. | `document create` answers `invalid_path` for the same thing. | `invalid_path`; `invalid_input` is kept for the limits of discovery. |
| A second repair of one path under another ID | Not said. | Two items for one file is the confusion a repair exists to remove. Approved by the product owner on 2026-10-13. | `not_repairable`. |
| How long a repair compares its preview | Until a step is recorded. | Making the context changes nothing a preview observes. | Until the write step is recorded. |
| A second live call of a request whose first call is indexing | The second records its steps and takes the row. | The first call was fenced and answered `recovery_required` for a request whose record finished `ok`. | The row is left alone and the second call answers index-pending. Approved by the product owner on 2026-10-13, with the three-second consequence listed in the known limits. |
| The message of `repository_not_registered` | "The repository is not enabled in Manyhands." | It sat beside `repository_not_enabled`. Product owner, 2026-10-12. | "The repository is not registered with Manyhands." |

**Known limits.** Each is behavior of the code as it stands. The first
group was recorded at the design checkpoint that followed Task 7 and
holds for every authoring write. The groups after it were recorded while
the later tasks were built. Several limits rest on defects in code that
predates this Cycle; each names its ticket.

- **A second attempt can release the record before the first attempt's
  journal row exists.** Between acceptance and the insert of the journal
  row, a second call that stops with no row deletes the record; the first
  call then commits and cannot settle, and a later retry is
  `already_applied` with no commit. It is accepted because no effect is
  repeated, the first caller has its result, and every alternative found
  breaks reuse of a request ID with corrected input.
- **A changed-input result can name another request's commit.** A mismatch
  against an accepted record names the newest commit to the request's path
  in range, which on a widened range can be older than the request, and
  counts a row that ended before its checkpoint as an attempt that ran. It
  is accepted because the mismatch holds only a digest of the original
  input and cannot compare a commit with an intent.
- **After a crash between a rejection and the deletion of its record, the
  same request ID with corrected input is `request_mismatch`** until the
  original input is sent once more, which releases the record. Row 11
  does not survive that crash. The boundary cannot tell a correction from
  a different request. Accepted provisionally by the product owner on
  2026-10-11; may be revisited.
- **A context removed by hand after lost output.** With only the context's
  branch deleted, the retry is `internal_error` and the record is
  released; with the whole context removed, the save recreates it from
  primary and commits again; and when the context already existed at
  acceptance, its removal leaves the request's row pending and the
  repository blocked until the operation is abandoned. These are accepted
  because only hand-editing produces them today and the first commit went
  with the branch, so nothing is committed twice.
- **A request whose file was uncommitted at acceptance, on a branch that is
  then rewritten,** can be answered `external_change` from history older
  than itself. It is accepted because no rule was found that tells this
  from a real change from elsewhere.
- **Identity removed between attempts.** When an earlier attempt's row is
  still pending, a retry reaches the domain, which answers "identity
  required" and completes the row. The result is `identity_required` with
  no effects, whatever the earlier attempt left, and the record is
  deleted. If that attempt had committed, the commit is never reported,
  its index hand-off is not run by this request, and the next call is
  answered by the already-applied rule. If it had written and not
  committed, the next call is `external_change` and a fresh read and save
  commits the file; row 12 is not met on this path. A request whose row
  had already completed with its commit is not affected: it is finished
  as Done without a domain call. It is accepted because nothing is
  written twice.
- **A branch reset under a completed request whose output was lost.** If
  the item's branch and file are put back to the state the request
  expected, the retry finds no commit of its own and nothing foreign,
  calls the save again, and the save writes and commits a second time;
  that commit is the one reported. It is accepted because only
  hand-editing produces it and the first commit went with the reset, so
  no effect is duplicated.
- **An identical commit made outside the library while a request's row is
  pending can be reported as that request's.** The commit must be of the
  intended content, from the expected state. No sequence of two requests
  through the library was found to reach this: while one request's local
  row is pending, every other local operation on the repository is
  refused. It is accepted because the content on disk is what the request
  intended.
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
- **A retry within about three seconds of a crash while indexing is
  `discovery_pending`.** A save or a repair, like every operation the
  liveness rule covers, leaves a pending row with a live index-owner
  heartbeat alone, and a process killed while it indexes
  leaves a heartbeat that stays fresh for three seconds; nothing tells it
  from a call at work. The retry is `partial` with the commit reported and
  offers `operation.resume`, which inside the window answers `busy` with
  `request.retry`. A later retry, once the heartbeat is old, takes the
  hand-off up and completes. Approved by the product owner on 2026-10-13:
  the answer is true, and a caller retries a `partial` answer anyway.
- **The liveness check and the steps it protects are not one
  transaction.** A refresh that borrows the operation ID can claim the row
  between them; the owner is then fenced by the row's epoch and writes
  nothing more.
- **Two racing callers can each be answered `busy` part-way,** in the two
  `partial` shapes described under Settling. Seen on Windows more than
  elsewhere. No effect is repeated and the retry answers `ok`.

*Documents, moves and short-code assignment.*

- **A write that died before its step was recorded, followed by someone
  else's uncommitted edit of the same file.** The retry is
  `external_change` with no effects, although the first attempt's write is
  on disk under the edit. For an item that already had an editing context
  the record is released; for one that did not, the record stays and
  `operation.abandon` is offered. Row 12 of the required outcomes is not
  met on this path. It is accepted because nothing is written twice and a
  fresh read and save commits what is there. The cause is that the domain
  closes a row with no recorded step as `rejected` although the file was
  written: ticket `01M4PB7YZMBFAFFM94AZY08HWS`.
- **A move whose removal of the source was never committed is refused, not
  repaired.** When the branch tip holds the document at both paths, a
  move is `external_change` and leaves no record; the user finishes the
  removal with Git. A move counts as already applied only when the tip
  holds the document at the destination and at no other path. It is
  accepted because the domain call does not repair the state, and a no-op
  there would report a move that is half done.
- **A completed move of a document whose source file was never committed,
  retried after lost output.** If the destination has since been
  committed over, the retry is `external_change` and the record is
  released. If the document has since been deleted from both paths, the
  retry is `external_change` and the record stays `accepted` over a row
  that has completed. A pending move of such a source cannot be finished
  by its retry at all. These follow from the domain refusing a repeated
  move, ticket `01M4PB80T7SVB23TQ6466RDHCB`, and are accepted because only
  a source that was in no commit is affected and nothing is lost.
- **A hand-made commit of the destination with identical bytes, made
  before the move's own commit,** leaves the move's result with no commit
  reported. It is accepted because the content is as intended and only
  hand-editing produces it.
- **A short-code assignment whose file was uncommitted at acceptance and
  has been overwritten since** is judged by a weaker rule: the file is as
  intended when it holds a slug, whatever its other fields. Normally the
  file must also equal, field for field, the file the request was
  accepted against. It is accepted because the boundary holds no copy of
  an uncommitted file to compare with.

*Remotes and the index.*

- **Removing a remote is refused where Git's own removal would take more
  than the remote.** `remove_remote` deletes, through Git, every
  reference any refspec of the remote maps onto, every configuration
  entry matching the remote's name as a pattern, and with them branches,
  tags and the settings and tracking references of sibling remotes. That
  is a defect of the existing library, ticket
  `01M4PB7WZN9WNAQ3KXZ19J28KM`. The domain function is unchanged. The
  `remote remove` binding refuses with `invalid_remote`, at `prepare`, on
  a first call and on a retry with no recorded step, when the removal
  would take a reference outside `refs/remotes/<name>/`, a reference
  another remote's fetch refspec also claims, a configuration entry that
  is not exactly the remote's own, or when one of the remote's own
  entries is not in the repository's local configuration. The user sees
  `invalid_remote` with no detail naming the cause and removes such a
  remote with Git. Two gaps remain: no lease is held between the
  binding's observation and the domain call, so a refspec added in that
  window is honored by Git; and a retry whose removal has recorded a step
  is not checked again.
- **The preview of a remote's removal names classes, not references.** It
  says whether tracking references and branch upstreams will be removed,
  not which. A further branch set to follow the remote after the preview
  is therefore not `external_change`. Any change to any other remote's
  fetch refspecs between preview and execution is `external_change`, even
  an unrelated one.
- **A lost refusal of `remote remove`** is `external_change` with
  `request.prepare`; the new preview names the cause.
- **`remote select` killed between writing the configuration file and
  committing it** is open case 2: the retry, and any new selection, is
  `worktree_not_clean` with no effects and no record. The user restores
  or commits the file with Git.
- **An index damaged in a place the library's own corruption test does not
  read** answers `index_unavailable`, whose `index.rebuild` action cannot
  repair it; the way out is to remove the index file by hand. Widening
  what counts as corrupt decides when a database is replaced and is a
  change to the existing library: ticket `01M4PB82G07RZSQMNJZN672S3W`.
- **A registered repository with no configuration at its root still
  offers `index.rebuild` on a ticket or document create.** The create
  answers `repository_not_registered`, as it did before the refusal of a
  repository that was never enabled, and a registered repository keeps
  that answer. The rebuild changes nothing there, so the action is a dead
  end.
- **Whether a repository was ever enabled is read from Git, and not all
  of it.** Remote-tracking branches are not read: a clone whose only local
  branch predates enabling, with the configuration on `origin/main` alone,
  is refused as never enabled. Any entry at the configuration path counts
  as enabled, valid or not, so a stray file there lets a rebuild register
  the repository, whose commands then answer `invalid_configuration`. A
  branch tip that cannot be read counts as not lacking the file, so a
  dangling branch reference makes a repository that was never enabled
  count as enabled. The refusal reads every local branch tip, one tree
  lookup each.
- **A registration whose directory is gone cannot be removed.** Ticket
  `01M4R19P943QZKHHKRXV2D2TP2`.
- **A stored problem whose path holds a backslash makes the index
  unavailable on Unix.** A file or link under `docs/` named with a
  backslash, such as `docs/a\b.md`, stores such a problem, and every
  refresh, rebuild and snapshot then fails with `index_unavailable`. It
  is a defect of the existing library, pinned by an existing test:
  ticket `01M4TFGHPG98GCM6TX3B56HJAB`. Folder discovery stores no such
  path; see the folder limits.
- **`repo remove` drops the repository's whole local journal,** the rows
  of other requests included. An old `remote remove` request whose record
  is still `accepted` can then run again on an identical remote that was
  added back. It is accepted because that is what removing a registration
  does today.
- **Removing a registration and registering the root again within the same
  second** can give the same observation, so a preview of the first
  registration can be accepted for the second.
- **A changed-input result under-reports two states.** A remote removal
  killed after the deletion and before its step, and a key deletion
  before its first recorded phase, are not shown as a change made with
  work remaining.

*Keys.*

- **An interrupted key generation is `partial` even when it wrote
  nothing.** The key journal cannot tell a reservation from a written
  file at the point the boundary reads it.
- **A retry with another passphrase is the same request.** The passphrase
  is never part of the digest. After the key pair is written, a retry
  with a different passphrase registers the key that was written under
  the first one and reports `ok`.
- **The key store is not part of a request's identity.** A retry of a
  generation against another key store is `internal_error`, `partial`,
  with the record kept and nothing created.
- **After the cache is lost, a retried `key import` names a new key.**
  The recorded changes, the request records and the registry are one
  file and go together. The retry is a new request against an empty
  registry: it registers its source once, under a new ID, so there is one
  registration and never two, but the retry does not name the key the
  first call registered. A retried selection is then `key_not_found` and
  a retried clearing a no-op.
- **A record of `key import`, `key select` or `key clear` that an earlier
  build accepted** has no operation. If its result is lost again, its
  retry has only the compare with the first call's observation, and a
  setting changed and put back is changed again. This holds for such a
  record alone.
- **`operation show` does not find the operation ID of a recorded key
  registry change.** A result names an ID the operation read cannot show;
  `request show` does list it. Showing it needs a new value of the
  published operation actions. Confirmed by the product owner on
  2026-10-12.
- **An accepted record that cannot be read repeats its answer.** A
  record no record function could have written, such as one that lost
  its operation, is `internal_error` with `request.retry` on every send,
  and each send raises its attempt. The record's shape cannot change, so
  the action leads nowhere; the way out is a new request ID. It was a
  dead end before the action was offered too.
- **A refusal on a retry that will never change holds its record.** A
  retried selection whose key was removed is `key_not_found` on every
  send, with the record `accepted` and `request.retry` offered each time.
  The way out is a new request ID. `key remove` still deletes its record
  on a failed retry.
- **A first call of a key registry change makes no compare.** Between its
  `prepare` and its domain call another actor's change is overwritten.
- **The path of an imported private key is hashed into a stored digest.**
  `key import` feeds the caller's key paths to the intent digest, which
  is salted with the request ID held in the same row. While the key is
  registered this adds nothing, since the registry holds the path. After
  `key remove` the registry row is gone and the request record stays, so
  someone who holds the database can test a guessed path with one hash.
  It is the one exception to the rule that no private absolute path
  reaches a digest, and is of the same kind as the body digest under
  [Risks](#risks).
- **A late `remote select` is refused for any change to the configuration
  file,** not only to the selection: a commit to another key of the file
  since acceptance gives `external_change`, final.
- **A key whose material is pending** makes `key select` and `key remove`
  answer `recovery_required` with no action. What finishes pending
  material is the generation request's own retry, which the caller of the
  other command does not hold. Confirmed by the product owner on
  2026-10-11.
- **A key file replaced by another file is not deleted.** A retry of
  `key delete` that finds a new file at a key path is refused with every
  file intact and the record `accepted`. The code depends on the file:
  `internal_error` when its mode, owner or link count is not what a key
  file's must be, `external_change` when only its identity differs. The
  `external_change` offers no action: no new preview clears it.
- **The privacy scan looks for the literal secret.** The test that scans
  every record, journal, envelope and event would not catch an encoded
  copy of a passphrase.

*Confirmed commands.*

- **`repo create` or `repo enable` killed inside the domain call before
  its first recorded step** leaves a pending row and an `accepted` record.
  The retry observes again and is `external_change`, offering
  `repo.inspect` and `operation.abandon`, unless the only difference is
  the empty directory the request itself made. This is wider than open
  case 2: `repo create` has two such windows, and an enablement that
  renames an unborn HEAD has one more. The ways out are to restore the
  previewed state by hand and send the same request again, or to abandon
  the operation.
- **A directory that holds `.git` and no recorded step** is
  `occupied_path` to a new `repo create`.
- **A parent that stops being a directory, and a root given as a single
  relative name,** are `invalid_path`.
- **An enablement that fails before its commit leaves no record.** Its
  confirmation is released, but its expiry still runs from the preview,
  so a retry after ten minutes is `confirmation_expired`.
- **After the cache is lost, a retry of a confirmed command is
  `confirmation_not_found`,** also when the command had completed.
- **A lost refusal where nothing has changed is `busy`.** A confirmed
  request that was refused with nothing done, whose answer was lost, and
  whose retry finds nothing in its way, is answered `busy` with
  `request.retry`. Its record is deleted and its confirmation released,
  so the next send is a first call. The action is right; the message is
  not literally true. The product owner agreed on 2026-10-11 to leave it
  without a new result code.
- **The end-state check reads the configuration file in the worktree,**
  committed or not.
- **The observation digest is not salted, and it hashes the identity's
  name and email, a remote's raw location, the two key file paths and the
  label of a `key delete`, and the locations a `host approve` or
  `host replace` connects to for fetching and for pushing.** Two previews
  of the same state hold the same digest, and the confirmation row holds
  a hash of a location that may contain a credential.
- **The preview of `key delete` publishes two absolute paths under the
  user's key store.** It is the one envelope that does, on purpose: the
  command must name the files it will delete.
- **`execute` asks for the confirmation before it validates the input,**
  and `prepare` for a command that needs no confirmation returns an empty
  scope, because the input is not read.
- **Only tests tie a command to its preview schema.** The preview's
  `details` is one of eight shapes, each with its own schema file, and the
  schema checker has no way to say which shape a command gives. The
  preview's wrapper is restated in each of those files with nothing tying
  the copies; a shared reference would change every preview schema.
  `request.schema.json` repeats the outcome and code lists.
  `recovery_action.schema.json` does not say that `operation.resume`
  takes its root from the scope.

*Local identity.*

- **A configuration changed and put back as previewed.** A request writes
  its identity and loses its result; someone sets the identity back to
  exactly what was previewed; a late retry finds the previewed state and
  writes again. The consented digest cannot tell that from a request that
  never wrote. Closing it needs a marker that survives with the Git
  write, which nothing here has.
- **A stale `config.lock` is `busy` for as long as it is there.** It is
  never removed and has no age rule, as in Git itself. A process killed
  while it writes can leave one; the user removes it by hand. The `busy`
  message does not name the file.
- **A killed write can leave a scratch directory.** A `0700` directory
  `manyhands-identity-*` holding a copy of the configuration can remain in
  the Git directory, and one holding a name and an email in the system's
  temporary directory. Nothing removes them.
- **A program that ignores `config.lock`** and writes between the last
  byte compare and the rename is overwritten. The window is a few
  instructions.
- **Under the lock an `includeIf` is not evaluated;** the read before the
  lock, through the repository, does evaluate it.
- **`extensions.worktreeConfig` is not read.** Manyhands reads and writes
  the repository's local level; where a `config.worktree` holds an
  identity, Git commits as that one.
- **A trailing full stop in a name** is kept by Manyhands' commits and
  dropped by the `git` binary, so a commit made by hand carries `Jr` where
  Manyhands' own carry `Jr.`.
- **The file's mode is kept on Unix only.**
- **Enabling a repository still restores `.git/config` from a snapshot
  outside Git's lock,** and can erase another process's change to it. It
  is existing behavior that the identity write no longer shares: ticket
  `01M4SDB38H9JTG77VMC9102W2F`. `add_remote` and a remote's removal answer
  `internal_error` on a `config.lock` someone else left.
- **`initials_required` offers `repo.identity_set`, which asks for a name
  and an email as well.** A user whose identity is global and who lacks
  only initials must write the name and email locally to set them; the
  preview shows it.

*Host approval.*

- **A pin changed and put back as previewed.** A late retry then connects,
  and pins only if the host still presents the key the request names.
- **`host_mismatch` for a replacement is inferred.** The domain refuses
  the write without the key it saw, and the binding concludes that the
  host presented its pin when neither the pin nor the reapproval marker
  moved between its two reads. Two edges remain: a host that presents two
  keys in one connection, and a pin changed and put back between the
  reads. Closing them needs the domain to report the key it saw.
- **Two fixed messages do not fit.** The message of
  `host_replacement_required` reads "The host's key differs from the
  approved one and must be replaced.", and it is also the message of the
  `prepare` refusal, where no key of the host was seen: the host is
  pinned to another key than the one the request names. The message of
  `host_mismatch` reads "The host did not present the expected
  fingerprint.", where `expected` means the key the request names. Both
  are published strings, one for each code, and their wording is the
  product's. Proposed and not made: "The host is pinned to another key;
  its pin must be replaced." and "The host did not present the key the
  request named."
- **`presented_*` is null on a successful approval.** The result of
  `host approve` and `host replace` fills `presented_algorithm` and
  `presented_fingerprint` on `host_mismatch` only. On a success the key
  the host presented is the key the request names, in `algorithm` and
  `fingerprint`. The meaning never conflicts, since it is never another
  key, and the schema says so; filling it always would change a
  published success shape.
- **The transport's own types keep their names.** `HostApproval` and the
  transport's replacement error say `expected` and `presented`, and
  `HostApproval::presented` is the key the request names, the meaning the
  published vocabulary takes from the word. They are not published.
- **A changed-input result of an accepted host approval is `partial` for
  a pin another actor stored,** or for a remote someone pointed
  elsewhere, as for the request's own pin.
- **A `partial` approval may name a pin another actor stored.** If someone
  else pinned the same key while this call was failing for another
  reason, the answer says this request left it. The state it describes is
  true; the attribution may not be.
- **An `execute` for a host the remote no longer names** answers
  `already_applied` when the host is pinned to the named key, where
  `prepare` answers `invalid_input`.
- **Without the transport started, `observe_host` is `internal_error`.**
  That is the existing mapping for a front end that did not start it.
- **A non-interactive caller with a protected key cannot approve a host;**
  the CLI RFC accepts that limit for all protected-key automation.

*Folders.*

- **Unicode normalization is not checked.** A name with a letter that can
  be written composed or decomposed is one name on macOS and two on Linux
  and Windows. On macOS a name in another normalization than an existing
  folder's answers `ok` with nothing made; on Linux it makes a second
  directory. Checking it needs a normalization dependency.
- **Windows 8.3 short names other than `git~1` are accepted,** so
  `docs/LONGFO~1` can name an existing longer folder there.
- **A path too long for the file system fails part-way:**
  `internal_error`, `partial` when a parent was made, with the parents
  left. No rule can be the same everywhere, since the limit depends on
  where the repository is.
- **Case is folded with Rust's own tables,** which differ from NTFS's and
  APFS's in rare code points.
- **The non-Unix directory walk checks a parent and creates in two
  steps,** as it already does for saves. A parent swapped for a junction
  between them is followed.
- **An empty folder does not survive a clone.** Git does not track an
  empty directory and nothing is written into one.
- **Folder names are stricter than document paths.** A document can be
  saved under a directory `folder create` would refuse.
- **A folder that is already there can be refused or `partial`,** because
  a no-op still runs its refresh: `recovery_required` behind a pending
  operation, `discovery_pending` when the refresh fails.
- **Abandoning a folder creation's pending refresh row** reports what a
  refresh leaves, which is nothing. The directory the request made is not
  mentioned and is left.
- **A directory whose name holds a backslash or is not UTF-8 is not
  listed,** and the folder list and the document list are then not
  `complete`.

*Repair.*

- **A ticket whose broken field is a closure field cannot be repaired.**
  Closure is compared by the YAML values as written, so a ticket whose
  `closed_at` or `closed_by` is itself what is wrong, or that has one
  without the other, has no source that both carries the file's values and
  conforms. The same instant written another way is `invalid_input`.
- **A valid ID copied into several nonconforming files makes none of them
  repairable.** Each is `identity_ambiguous`. The way out is to edit one
  by hand on primary.
- **A context that holds stale bytes at the path stops every repair of
  it.** When an abandoned repair left its context and the file was then
  changed and committed on primary, the context holds neither primary's
  file nor the source. The repair is `external_change` and offers
  nothing, since no new preview can succeed; the user removes the item's
  worktree and branch with Git.
- **A worktree removed by hand is not seen by the context scan.** The
  scan for a path repaired under another ID reads the worktrees Git has
  registered under an item ID and the index. An item moved inside its
  context, or a context whose worktree was removed by hand, lets a second
  ID in.
- **A second repair made before the first is indexed** is not seen by
  the index either; the first's pending row then refuses the second with
  `recovery_required` at execution.
- **A re-entry that is refused after a write whose step was lost leaves
  the file uncommitted.** In a context that was reused, a write that died
  before its step was recorded leaves a row with no step; a refusal on
  re-entry closes that row with the written file left in the context, as
  a save behaves. Ticket `01M4PB7YZMBFAFFM94AZY08HWS` is the same defect.
- **The case rule has no Unicode normalization** and is not exercised on
  a file system that folds case: a path must name its file by the names
  its directories list, compared with Rust's lower-case and upper-case
  forms.
- **Every canonical file is read to ask who holds an ID,** at the
  preview, at `prepare`, on re-entry and in the domain.
- **What counts as a canonical source for that scan is wider than what
  the worktree collector indexes.**
- **`scope.item_id` of a refusal for an ambiguous identity is the ID the
  source carries,** which is another item's.
- **A repaired document is listed twice, and comments stay blocked,**
  until the repair's context is merged; see
  [Repair and adoption](#repair-and-adoption).

*Abandoning.*

- **Liveness is best effort.** A clock stepped forward by more than three
  seconds, a heartbeat write that fails, or two calls sharing one rebuild
  row can let a live operation be abandoned. The live call then stops
  with `OperationAbandoned` at its next step and writes nothing more. A
  clock stepped backward keeps a killed call's row `busy` for the size of
  the step. It is accepted because the refusals hold in every miss.
- **The abandoned marker replaces the row's last step,** so
  `operation show` no longer says how far an abandoned operation got. A
  second column would keep it, at the cost of a migration.
- **A save abandoned in the instant between dropping its lease and
  completing its row** reports `external_change` with its commit kept.

*Synchronization and cancellation.*

- **Nothing shows that a call is inside its push.** The remote journal has
  no heartbeat. If a first call is still pushing when `request cancel` is
  followed by the same request, the second call takes the operation
  over, asks the remote, finds the commit absent and finishes
  `cancelled` with `publication: pending`, or with the push not
  confirmed; the first call's push can then land. The stored answer says
  nothing was published and the remote holds the commit. A later
  synchronization reports it as already current. `operation resume`
  takes a running synchronization over in the same way.
- **A cancellation whose push outcome cannot be learned reads as
  `recovery_required`.** A truthful `cancelled` is lost in some cases:
  when the remote never took the push, and when a `cancelled` answer was
  lost and is asked for again from a row whose push was only intended.
  It is accepted because a cancellation must always be able to free the
  repository offline and a final answer must not guess. Confirmed by the
  product owner on 2026-10-12.
- **A row cancelled after verification can overstate.** A later call that
  has only the row reads a verified push as published, although the
  remote may have held the commit already. The call that verified it
  knows which and says so. This errs on the side the rule prefers, and is
  how the domain itself reads the row on a restart.

- **A cancel flag that cannot be read before the push ends that call**
  with `recovery_required`, where a transient failure could have been
  ridden out.
- **A held reservation after a blocked synchronization** is open case 3 of
  [The Journal Defect](#the-journal-defect), ticket
  `01M4R19PC0T6ZPKM39X9DF26HX`. `merge_required` and
  `remote_branch_deleted` offer `request.cancel` as the way out; the other
  causes are dealt with, or cancelled with `request cancel`, by hand.
- **A local fast-forward is not shown.** A synchronization reports
  `publication` and `discovery` only, so one that moved the local branch
  and then stopped is `blocked` with nothing durable reported.

*Comment submissions.* These hold for rows that only a direct call of the
service makes, until `comment add` is bound.

- **Abandoning a submission spends its comment ID.** The binding is kept,
  so a new operation for the same comment ID is refused, also when the
  abandoned one never committed. A file it wrote and did not commit stays
  in the worktree and keeps the context from synchronizing until it is
  removed by hand, as for an abandoned save.
- **`committed: true` at `comment_checkpoint_intent` may name a commit
  that does not exist.** From the intent on a commit may exist, and only
  Git says whether it does. The abandon's preview and result say
  `committed: true` either way; when there is no commit the same result
  lists the comment's file in `paths` as uncommitted. The alternative
  would report `committed: false` over a commit that may exist.
- **The abandon of a registration-pending comment is offered on any
  `recovery_required` that offers abandons,** also when that row is not
  what is in the way: it refuses only reservations.
- **`operation show` of a registration-pending comment has
  `item_id: null`.** The journal row's item is not filled for a submission
  made with no registration. The abandon's preview and result do name the
  item, from the row's target. Whether other comment rows share this was
  not checked.
- **A completed synchronization of a comment that only owes its index
  hand-off is refused by a resume too.** Running it would be harmless; it
  is refused to keep one rule. The Cycle that binds `comment add`
  revisits it.
- **The test of the reservation's refusal is not a two-call race.** It
  abandons the submission and then asks for the reservation the running
  call would ask for next, which is the transaction the refusal is in. No
  hook pauses a call between its proof and its reservation. A test that
  runs the replay and the abandon at once is left to the `comment add`
  Cycle.
- **Every answer in the two tables of a comment's rows that no test
  names is derived from the code.** One test drives a registration-pending
  row through the boundary, and one a comment's synchronization through a
  resume; the others are at the service's level or untested.

*Resuming.*

- **A resume raises the owner's attempt before the domain says whether it
  takes the operation.** If it does not, a live call of the owner settles
  nothing, and the owner's record stays `accepted` over a completed row
  until the owner's next call, which is never refused and answers
  truthfully. Until then `request show` says `accepted`. Raising the
  attempt later would reopen the window in which a stale call settles the
  record.
- **In a narrow race the first answer and the replay differ.** The owner's
  live call answers `cancelled` and cannot store it, and the resume
  stores `recovery_required` with the publication not confirmed. Both are
  true.

#### Open questions for the product owner

Each of these is built one way and has not been confirmed. The current
behavior is stated first, then the alternative. The product owner decided
the earlier questions on 2026-10-12 and 2026-10-13, the table of key
registry changes and the message of `repository_not_registered` among
them; their rules are in the sections that describe them.

1. **Is `merge_required` the right code when the remote has only moved
   ahead after a lost push?** Today that case is `merge_required`,
   reported with `published` and the commit once the push endpoint shows
   it holds a descendant, although nothing needs merging: the local
   branch needs a fast-forward this operation cannot make. The
   alternative is a dedicated code, which is a contract addition.
2. **Should `operation resume` and `operation abandon` require a root for
   a key operation?** Today both inputs require one. A key operation
   belongs to no repository: a resume finds it whatever root is named,
   and `repo.inspect` on its result names the caller's root; an abandon
   refuses it with `invalid_input` whatever root is named. The
   alternative is an optional root.

**What the next binding must do.** Nothing is left to decide before it.
Task 7c made the structural changes the checkpoint asked for: the ticket
binding destructures its input and `TicketDraft` in `fields`, `holds` and
`validate`, so a new field cannot be left out of the digest unnoticed;
commands are the closed enum `Command`; the derivation of a save's effects
is in `bind/save.rs`; `execute`, replay, re-entry and settling each have
their own file; `outcome::recovery` is an exhaustive match; and the names
of an editing context have one source in the mutation module,
`context::context_names`. A new binding adds a `Command` variant, which
does not compile until `bind::recorded_effects`,
`MutationDataDto::from_stored` and `Mutation::kind` decide about it;
implements `ended_without_checkpoint`; and moves each of its codes that
the recovery tables give an action to its own arm of `outcome::recovery`.
A confirmed command also answers `as_confirmed`, which does not compile
without it, and supplies the probes of `confirm::reenter`. A command
whose failed hand-off leaves nothing pending, as `folder create`'s does,
answers `Command::hands_off_to_any_refresh`, which decides both the action
of its `discovery_pending` and whether a replay of its result is given
actions again. Every bound command has a row in the conformance table,
whose home is named in `HOMES`, one of four test targets; the table runs
over each command the rules every command owes
the boundary: a finished request is answered again with its stored result
and nothing of its record changes; the same request ID with changed input
is `request_mismatch` and the finished record stays as it is; a refused
request leaves no record; and a command that needs confirmation is refused
without one as `confirmation_required`, any other command is refused with
one as `not_confirmable`, either way leaving no record. Those four do not
reach a stale preview of a confirmed command, since an unknown
confirmation is refused before the binding is asked. A fifth rule does:
every confirmed command has a row whose preview is made stale, so that
the binding's own compare answers `external_change`, and a confirmed
command without such a row fails the table. A row supplies the change
itself, as its fixture's way to alter what its preview showed, and keeps
the input it began with, so that the request is still its
confirmation's.

### After the cache is lost

When the database is replaced, the request records, the confirmations, the
journals, the registration, the selected key and the host pins are all gone.
A retry then arrives at a repository that is not registered.

1. Every repository command first returns `repository_not_registered`, with
   the `index.rebuild` recovery action, because the repository was enabled;
   one that was never enabled is `repository_not_enabled` and is not
   registered by a rebuild. Rebuilding with the root registers
   the repository again. When the database is unreadable and not gone,
   `index rebuild` restores it first and is then recorded like any other
   request; see "Looking the request up". Keys must be imported or generated and selected
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
| `key import` | An empty registry. | The source is registered once, under a new key ID; see the known limits. A retried `key select` is `key_not_found` and a retried `key clear` a no-op. |
| `repo identity-set` | The confirmation is gone. An identity is set on a repository that need not be registered, so nothing else of it is lost. | `confirmation_not_found` with `request.prepare`, and nothing is written. A new preview of an identity that was written is a no-op with no confirmation. |
| `host approve`, `host replace` | The confirmation and the pins are gone. | `confirmation_not_found`, before anything is observed or connected to. A new preview is `repository_not_registered` with `index.rebuild` until the repository is registered again. Then a preview of an approval shows the host with no pin, and a `host replace` is `invalid_input`: the pin it would replace went with the cache. |
| `folder create` | The directory is there. | `repository_not_registered` with `index.rebuild` first, and no record. Once the repository is registered again, a no-op that runs its refresh: `already_applied`, `discovery: current`, with nothing made and no row left pending. |
| A repair | The confirmation is gone; the item's context holds the corrected source. | `confirmation_not_found`. With the source committed there, the new preview is a no-op. With it written and not committed, a new preview is given and the new request commits it, in the same context and under the same ID. |

No retry in this table writes content the first attempt did not intend, and
none reports a commit it cannot find.

Where no evidence proves a replay safe, three RFCs ask for
`recovery_required`. The table answers more exactly than that; see
[Departures from the RFCs](#departures-from-the-rfcs).

Every row is built. A synchronization retried after the cache is lost is
blocked in turn by each piece that went with it (the registration, the
selected key, the host approval) and, once each is restored, finds the
remote current and pushes nothing. A document create and save follow the
ticket rows; a move is a no-op only when the branch tip holds the document
at the destination and at no other path. For a ticket the built checks
differ from the table in these ways:

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
change between the two checks fails the second. A retry that re-enters an
accepted record does not check the token again. A retry whose record was
deleted, lost or released on re-entry is a new request, and its token is
checked like any other; see
[Re-entering a request](#re-entering-a-request), step 3.

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

**An absent path.** A new read, `observe_path`, whose command name is
`path observe`, returns a token for a path in a named context. It reads
the paths `show_path` reads and no others: an item's canonical path, or a
listed nonconforming entry under an item prefix. Any other path, such as
one under `.git/`, is `invalid_path`. For a file the token is the F1
token. For a path where nothing exists, the read succeeds with
`exists: false` and the token is `v1:` plus a digest of a fixed "absent"
marker, the branch and the path. A directory and a symbolic link are
`invalid_path`. `document move` takes the token of its destination as its
required `destination_observation`: a token that does not match is
`external_change`, and a matching token of a file that is there is
`occupied_path`. Creation inputs carry no token; a create always expects
absence. The CLI RFC names no verb that returns a token for an absent
path; C3 needs one and that is recorded for its planning.

`document save` and `document move` require the source observation. The
library request leaves it optional today; the boundary rejects its absence
with `invalid_input` before any write. `document save` takes a full draft
with a title at the library boundary; the CLI RFC's optional title is the
CLI's concern.

**Repository-level state.** Commands that need confirmation bind state that
no item token covers. `prepare` reduces what it observed to one observation
digest stored in the confirmation; `execute` observes again and compares.

| Command | What the observation digest covers |
| --- | --- |
| `repo create` | Whether the target is absent, an empty directory or occupied. That the parent is a directory is checked and is not in the digest. |
| `repo enable` | The checked-out branch and its commit, or that the branch is unborn; a clean, unconflicted worktree; no Manyhands configuration yet. On a repository that is already enabled, `prepare` returns a no-op and no confirmation; a supplied prefix that differs from the stored one is `invalid_input`, since nothing would write it. |
| Both | The effective identity and its source, when none is supplied. |
| `repo remove` | The registration: its row and the time it was made, to the second. |
| `repo identity-set` | The local name, email and initials as the repository's own configuration holds them. The effective identity is shown in the preview and is no part of the digest: the command neither acts on it nor changes it. |
| `remote remove` | Whether the remote is configured; its fetch and push locations; whether it is selected for publication; its own refspecs; whether it has tracking references, references or configuration entries that are not its own, and branch upstreams; the names of sibling remotes and every other remote's fetch refspecs; the polling policy and the remote polling observes. Removing the selected publication remote is refused by the domain; `prepare` returns `publication_remote_in_use` and no confirmation. |
| `key delete` | For a key that can be deleted: its registration as the existing preflight reports it, and whether each of its two files is present. Otherwise which refusal stands. Changes to the key files themselves are caught by the existing review at execution. |
| `operation abandon` | How the row stands, and for a pending row its state and step. |
| `host approve`, `host replace` | The publication remote's name and the location each direction connects to; the pin stored for the authority, or that there is none. The reapproval marker is shown and is no part of the digest. |
| `document repair`, `ticket repair` | The path and the file's token; whether the file is committed on primary; whether the corrected ID is held by something else; whether an item is already at the path under another ID. The item's own context is not in the digest, because the request's own work changes it; `prepare` asks it directly. |

## Two-Phase Confirmation

`prepare` validates the input, observes, makes the checks the command
itself would be refused for, and returns a preview. A preview is therefore
given only of something that can be done as things stand.

```text
PreviewDto
    confirmation_id, expires_at
    command, scope
    effects        what will happen, as a list of { effect } entries in
                   the order the command makes them
    details        the command's own fields
```

It writes one `confirmation_records` row and nothing else, and takes no
lease, so nothing waits on a preview that is outstanding. It does not
create a directory, initialize a repository, register anything, write Git
configuration or contact a network. Its envelope names no request. A
command whose end state already holds is a no-op, `already_applied`, and
gets no confirmation. A rejection keeps a recovery action that the binding
or a failed read suggested, as `execute` does: a path that lies below a
repository's root is refused with the `repo.inspect` of that root which
its read suggested. Otherwise it carries the code's own action. It never
offers `request.retry`.

The envelope's data is `PrepareDataDto`: a preview, or, for one refusal,
the data of a host whose pin must be replaced. On the wire a `prepare`
envelope's `data` is a preview or null except for that refusal; see
[Host approval](#host-approval).

The preview effects, and when each is listed:

| Command | Effects |
| --- | --- |
| `repo create` | `directory_created` when the target is absent; `repository_initialized`; `identity_written`; then the four of `repo enable` that are always listed. |
| `repo enable` | `identity_written` when an identity is supplied; `head_moved` when HEAD names a branch other than the primary that has no commit; then always `worktree_exclusion_written` (the repository's `.git/info/exclude` is made to ignore the directory Manyhands keeps its worktrees in), `configuration_written`, `initialization_committed`, `repository_registered`. |
| `remote remove` | `remote_removed`; `remote_tracking_refs_removed` when the remote has any; `branch_upstreams_removed` when any local branch follows it. |
| `repo remove` | `registration_removed`, `index_dropped`, `operation_records_dropped`, `remote_state_dropped`. |
| `key delete` | `private_key_file_deleted` and `public_key_file_deleted`, each when the file is there; `key_unregistered`. |
| `operation abandon` | `operation_closed`; `request_released` when an unfinished request holds the operation. |
| `repo identity-set` | `identity_written`, also when only the initials change. |
| `host approve` | `host_pinned`. |
| `host replace` | `host_pin_replaced`. |
| `document repair`, `ticket repair` | `context_created` when the item has no editing context; `id_adopted` when the file has no valid ID and the source assigns one; `source_written` whenever the context will hold the source, also when an earlier request wrote it; `checkpoint_committed`. |

`details` is one of eight shapes:

- **Repository** (`repo create`, `repo enable`): the root, the primary
  branch, the identity that will author the initialization commit (name,
  email, where it comes from, and whether it will be written to the
  repository's own configuration), and the short-code prefix or null.
- **Remote removal:** the root, the remote's name, its fetch and push
  locations redacted as the remote list redacts them,
  `publication_affected` (always false in a preview that is given) and
  `polling_affected`.
- **Repository removal:** the registered root, when it was registered, and
  `files_changed` and `git_state_changed`, both always false.
- **Key deletion:** the key's ID, label and fingerprint, and the paths of
  the two files that will be deleted. No key material. These are the only
  absolute paths under the user's key store that any envelope holds.
- **Operation abandon:** see
  [Abandoning An Operation](#abandoning-an-operation).
- **Identity** (`repo identity-set`): the root; the `name`, `email` and
  `initials` the local configuration will hold; `current`, the local
  values now; and `effective`, the whole identity shape with its sources.
  Initials cleared and initials left alone are told apart by comparing
  `initials` with `current.initials`.
- **Host** (`host approve`, `host replace`): the root; `host` and `port`;
  `fetch` and `push`, which say what the publication remote uses the host
  for; `algorithm` and `fingerprint`, the key the request names;
  `pinned_algorithm` and `pinned_fingerprint`, the pin stored now, or
  null; and `reapproval_required`.
- **Repair** (`document repair`, `ticket repair`): `root`, `path`, `id`,
  `adoption`, and `primary_changed`, which is always false.

`execute` checks a first call's confirmation in this order, before it
validates the input:

1. A command that needs none and is presented one: `not_confirmable`.
2. A command that needs one and presents none: `confirmation_required`.
3. The record exists. If not: `confirmation_not_found`.
4. Its command, scope, target and input equal the call's, compared through
   the digest salted with the confirmation ID. If not:
   `confirmation_mismatch`.
5. It has not been accepted by another request. If it has:
   `confirmation_used`.
6. It has not expired. A confirmation lasts ten minutes from its creation,
   up to its expiry and not at it. It is also treated as expired when the
   clock reads earlier than its creation time, so a clock set backwards
   cannot extend it. If expired: `confirmation_expired`.
7. The binding validates the input and observes. A fresh observation must
   give the same observation digest. If not: `external_change`, with a
   recovery action to prepare again.

Acceptance is one conditional update in the transaction that writes the
request record; a request that loses that race is `confirmation_used`.
From then on the confirmation belongs to that request: the record holds
it, a retry is honored however old it has become, and a later call is
answered from the record whatever confirmation it presents. A request
that is rejected with nothing left behind has its record deleted, which
releases the confirmation in the same transaction. The expiry of a
released confirmation still runs from its creation.

**What the preview covers and what a retry compares.** The observation is
stored as two digests: one of the state and one of the effective identity
that will author the commit, which only `repo create` and `repo enable`
observe, and only when the request supplies none. A first call compares
both. For `repo create` with only a global identity, the preview shows
that identity with its source and says it will be written to the new
repository; if the global identity changes after the preview, the request
is `external_change`.

On a retry of an accepted request, the binding first asks whether the
command's end state already holds: the repository is enabled as requested,
the registration or the remote is gone, the key is deleted. If it does,
the record is finished with the request's result or a no-op; a completed
command whose result was lost must never be answered with
`external_change`. For
`repo create` and `repo enable` the end state holds when the root is a
repository whose configuration file, as the worktree has it, is valid,
names the requested primary branch and holds the requested prefix.

Every confirmed command is entered again through one function,
`confirm::reenter`. The executor sends any command that needs confirmation
to it, whatever the binding's own `reenter` is, and a confirmed binding
that does not answer that way is not run: the result is `internal_error`
and the record stays `accepted`. A binding supplies only its probes: whether
a step is recorded in its row, whether the end state holds and what it
observes now, and the answer to a difference, which is `external_change`
unless the binding says otherwise. A binding that says work is recorded
for a request whose row is absent is refused the same way: `internal_error`,
the record kept, nothing run or compared.

Otherwise the observation is compared again unless the request's operation
has a recorded step in its journal. The CLI RFC requires that "the service
rechecks observations before the first effect". A journal row alone is not
enough: `enable` writes its row before any check, so a row with no step
means no step has completed, and a row closed as `rejected` means nothing
was written. A process killed inside `enable` can leave files written
with no step recorded; the comparison then fails on the dirty worktree,
which is open case 2. Once
any other step is recorded, the request's own
work has changed what would be observed, so the comparison of the state is
skipped and the re-entry rules protect the rest. Three refinements were
built:

- **The identity is compared for longer than the state.** For
  `repo create` and `repo enable` with no identity supplied, a retry
  compares the effective identity with the previewed one until the step
  `initialization_committed` is recorded, so nothing more is written
  under an identity the user did not see. After that step nothing reads
  the identity. A changed identity is `external_change`.
- **A creation's own empty directory is accepted.** Under the request's
  own pending row, a confirmation given for an absent target accepts an
  empty directory: a `repo create` killed after it made its directory
  made that directory itself. With no row it does not.
- **A lost refusal.** When the row completed having changed nothing, the
  request was refused and its answer was lost. The record is deleted,
  which releases the confirmation, and the answer says what is true now:
  what stands in the request's way, when something does; `external_change`
  when what was previewed is not what is there; and otherwise `busy` with
  `request.retry`, since nothing has changed and nothing is in the way.

An `external_change` that leaves the record `accepted`, because the
request's own pending row is in the way, offers `repo.inspect` and
`operation.abandon` and never `request.prepare`: a new preview could not
be used while that row refuses every other request.

The confirmed commands with no journal (`repo identity-set`,
`host approve`, `host replace`) compare again whenever their end state
does not hold, on every re-entry, since no step is ever recorded for
them. Each also compares once more where it writes: the identity under
Git's configuration lock, the pin in the transaction that stores it.
Nothing of either is ever partly done. An identity write is one rename or
none, so a killed process leaves the file as it was, and possibly a stale
lock, which is `busy`. A pin is stored in one transaction. The two host
commands also answer the end state on a first call, in `prepare`, before
the boundary compares digests: a host already pinned to the key the
request names is a no-op.

`prepare` for a command that needs no confirmation returns
`not_confirmable`.

**Removing a remote.** The preview is given only of a remote that is not
the publication remote and whose removal takes nothing but the remote's
own configuration, tracking references and upstreams; see the known limit
on `remove_remote`. An interrupted removal whose step is recorded is
completed by its retry. If a remote of the same name was added by hand in
between, the retry is `external_change`, the record is kept and the new
remote is left alone; the retry completes once it is taken away.

**Removing a registration.** `repo remove` changes no file under the root
and nothing Git holds. A removal whose result was lost is a no-op on
retry, also when the root has been registered again since: the consented
registration is gone, and the new one is left. A registration whose
directory is gone cannot be removed: ticket `01M4R19P943QZKHHKRXV2D2TP2`.

Follow-on: the [2026-10-19 defect plan](2026-10-19-missing-repository-registration-removal-implementation.md)
addresses that historical limitation with exact stored-root selection and
stable confirmation/request scope, while preserving unfinished recovery work.

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

As built, `prepare` answers a key that cannot be deleted with the refusal
itself: `key_not_found`, `key_not_deletable` for an imported key, or
`selected_key_in_use`. A retry cannot delete more than was previewed: the
fresh review opens only the key's two store paths and refuses a file
Manyhands did not write, with every file left intact and the record
`accepted`. A deletion stopped after a file was removed is `partial` with
`recovery_required`; one stopped before anything was removed is `blocked`.
The result's data says where the deletion stands: `prepared`,
`private_removed` or `files_removed`.

## Result Mapping

### Envelope

`Envelope` gains a mutation constructor that takes the outcome explicitly. A
read's outcome follows from its code; a mutation's does not.

A mutation envelope may carry `data` on any outcome: the members of a
rejected cycle, the presented key and the pin of a host that needs
approval, the key a host presented in place of the one approved, what a
half-finished key deletion left. The contract test for mutations asserts
what each command's data holds per outcome, in place of the read rule that
data is present exactly on success.

`request_id` is the caller's request ID for `execute`, a resume included,
and null for `prepare`, `show_request`, `cancel_request` and
`observe_host`. `operation_id` is the request's own operation and is null
for a request that has none: a key removal, `repo identity-set`,
`host approve`, `host replace`, and an abandon and a resume, whose named
operation is their target. It is set on every result given after
acceptance of a request that has an operation, a refusal included. A key
command's results name no repository, item, branch or worktree. A host
command, an observation of a host, `repo identity-set` and
`folder create` name the repository only. A repair names the
item, and its branch and worktree once the request is accepted. A synchronization's scope
names the repository and the branch it synchronizes, and the item for
`item sync`; `scope.remote` is null on every synchronization result,
because a replayed or refused result is built without the remote's name.
`remote add` and `remote select` set `scope.remote`. For an item mutation
`scope.item_id` is always set.
`scope.branch` and `scope.worktree` are set on every result that names the
request's operation ID: they are the branch of the recorded position and
the path of the item's editing context, whether or not the context exists
yet. A result given before acceptance, and a mismatch, name neither.

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
| Item saves, repair | `write`, `checkpoint`, `discovery`. A save that changes nothing reports `unchanged`, `unchanged` and `not_requested`. A repair's write and commit are made in the item's editing context and never on primary. |
| Synchronization | `publication`, `discovery`; `commit_oid` is the published commit. A push this request made is `published` with the commit. A remote that already held the commit is `current`, names no commit, and the result is a no-op, `already_applied`. A synchronization that stopped reports `publication: pending` until its own evidence shows the remote took a push; that includes a push whose result was lost, which is `transport_unavailable`, an error and not `partial`. When the index hand-off is owed the code is `discovery_pending` with `discovery: pending`. A move of the local branch is not an effect. |
| `repo create`, `repo enable` | `write` (the configuration file), `checkpoint` (the initialization commit), `discovery`. |
| `remote select` | `write`, `checkpoint`, `discovery`, as it commits the configuration file. |
| `index refresh`, `index rebuild` | `discovery` only. A failed refresh or rebuild has made no durable effect: it is an error with its code, not `partial`, and its record stays `accepted` while its journal row is pending. |
| `folder create` | `discovery` only: `current` for a folder made and for one that was there, since both run the refresh. A refresh that fails after the folder is there is `partial` with `discovery_pending` and `discovery: pending`; its row is closed, so the record is finished, and it stays `accepted` only when the row could not be closed. A failure that left directories and never reached the refresh is `partial` too. |
| `remote add`, `remote remove` | `discovery` only, because the remote change itself is in `data`. It is `current` when the operation changed the remote and ended, also when that is answered from a completed row, and for `already_applied` under a row that was pending, where the hand-off ran. It is `pending` when the code is `discovery_pending`, and for an error after the remote changed with the operation not ended, which is `partial` with `request.retry`. It is `not_requested` for any other `already_applied` and for every other refusal or error. `write`, `checkpoint` and `publication` are `not_requested`. |
| Identity, keys, host trust, registration removal | Every effect `not_requested`. What changed is in `data`, and `outcome` distinguishes `success` from `noop`. |

For the last row, `partial` is decided by the domain outcome and described
in `data`, since no effect can show it: a key deletion that removed one file
and not the other returns `partial` with `recovery_required` and the
domain's recovery state. This is the reading recommended for decision 3. The
alternative, a new effect for local state, is a contract version change.

As built, a binding says so with one flag on its answer: the request made
a durable change that no effect shows. The outcome is derived from the
code, the effects and that flag in one function, and a failure after such
a change is `partial`. It is set for a remote added or removed with work
remaining, a key file written by a generation that has not ended, a key
file removed by a deletion that has not ended, a pin stored by a
verification that then failed, and directories a folder creation made
before it failed.

The data each command family carries:

- a document or ticket write: the item's ID and the path written, and for
  a ticket its short code and the relationship targets nothing holds;
- a repair: the item's ID, the path, and `adopted`, whether the ID was
  assigned by this repair;
- a remote command: the remote's name and its location redacted;
- a key command: the key's ID, label and fingerprint and where a
  generation or deletion stands;
- `operation abandon`: what the operation left;
- `repo identity-set`: the `name`, `email` and `initials` the local
  configuration holds once the request is done;
- `host approve` and `host replace`: `host`, `port`, the `algorithm` and
  `fingerprint` of the key the request names, and `presented_algorithm`
  and `presented_fingerprint`, which are set on `host_mismatch` to the key
  the host presented and are null otherwise. The data is carried on
  `success`, `noop`, `host_mismatch` and a `partial` result;
- a synchronization, and a resume that takes one up: data only when a
  host is not trusted. It is then `host`, `port`, `presented_algorithm`
  and `presented_fingerprint`, the key the host presented, and
  `pinned_algorithm` and `pinned_fingerprint`, the pin stored for the
  host, or null when there is none;
- `repo create`, `repo enable`, `repo remove`, the index commands,
  `folder create` and a resume of a hand-off: none.

**One vocabulary for a host's keys.** Every shape that carries a host key
uses the same three pairs of names. `presented_algorithm` and
`presented_fingerprint` are the key the host presented when it was
observed, always and only that. `pinned_algorithm` and
`pinned_fingerprint` are the pin stored for the host now. `algorithm` and
`fingerprint`, unqualified, are only the key a `host approve` or
`host replace` request names: its input, and the echo of that input in
its preview and its result. A caller copies `presented_*` from a
synchronization's data or from an observation into the `algorithm` and
`fingerprint` of `host approve`, and reads `presented_*` back from any
result with the same meaning, with one gap: the result of a host
approval fills `presented_*` only on `host_mismatch`; see the known
limits. F1's `host_pin` shape, the data of `host inspect` and of the pin
list, follows the same vocabulary: it names the stored pin
`pinned_algorithm` and `pinned_fingerprint`, renamed from `algorithm`
and `fingerprint` with the product owner's approval on 2026-10-13. A
front end gives `host inspect`'s `pinned_fingerprint` to `host replace`
as `old_fingerprint`.

### Result codes

Added to the registry. Each has a fixed message and its own class, used by
rule 3 above.

| Class | Codes |
| --- | --- |
| Input | `invalid_input`, `request_mismatch`, `request_not_found`, `confirmation_mismatch`, `not_confirmable`, `relationship_cycle`, `invalid_relationship`, `slug_already_assigned`, `occupied_path`, `not_repairable`, `remote_name_conflict`, `invalid_remote`, `key_not_deletable`, `repository_not_enabled` |
| Blocked | `confirmation_required`, `confirmation_expired`, `confirmation_not_found`, `confirmation_used`, `external_change`, `recovery_required`, `original_request_required`, `identity_required`, `initials_required`, `invalid_slug_configuration`, `identity_ambiguous`, `ticket_closed`, `wrong_branch`, `worktree_not_clean`, `worktree_conflicted`, `invalid_configuration`, `publication_remote_required`, `publication_remote_in_use`, `merge_required`, `remote_branch_deleted`, `push_rejected`, `no_selected_key`, `key_unavailable`, `key_rejected`, `selected_key_in_use`, `unlock_required`, `unlock_failed`, `host_approval_required`, `host_replacement_required`, `host_mismatch` |
| Incomplete | `discovery_pending`, `registration_pending` |
| Transient | `poll_yielding`, `remote_unavailable`, `transport_unavailable` |
| Cancelled | `cancelled` |
| None | `already_applied`, for a no-op |

`external_change`, `recovery_required` and `unlock_required` are the
spellings the RFCs already use; the rest are new names for outcomes the RFCs
describe without spelling. `registration_pending` is the existing outcome in
which the initialization commit exists and the registry write failed. A
passphrase prompt the user dismissed is `unlock_required`: the action did
not happen and needs an unlock.

Every code of the registry has a producer. Four are returned by no
mutation: `invalid_id`, `public_key_unavailable` and
`authority_not_found` are given by F1's reads only, and
`request_not_found` by `request show` and `request cancel`.
`path_not_found`, an F1 code, is also a repair's.
`remote_unavailable` is published and mapped, and no
test fixture produces it: the transport gives it only when Git reports
the remote as not found, and the test server closes the connection for a
path it does not serve, which is `transport_unavailable`.

**`repository_not_registered` and `repository_not_enabled`.** F1's
`repository_not_registered` says the index does not know the repository
on this machine; its message is "The repository is not registered with
Manyhands." It is the answer for an unregistered repository that was
enabled once, such as one whose index was lost, and it offers
`index.rebuild`, which registers the repository again.
`repository_not_enabled` is the answer for an unregistered repository that
was never enabled: no Manyhands configuration is in the root's working
tree, at its `HEAD`, or at the tip of any local branch. It is an input
error and offers no action, since a rebuild would register a repository
that has nothing to register; the repository is enabled first, with
`repo enable`. Git is read for this only where the repository is not
registered, and for a rebuild whose index is unreadable.

- A repository that is still registered is never answered
  `repository_not_enabled`. It keeps the answers it had: a ticket or
  document create is `repository_not_registered` with `index.rebuild`, and
  a synchronization is `invalid_configuration` with `repo.inspect`.
- `bare_repository` and `not_repository` come before it, with a readable
  index and without.
- `index rebuild` refuses a repository that was never enabled in the same
  way and registers nothing, also when the index is unreadable, and a
  rebuild that is entered again is asked again. A rebuild of a repository
  that was enabled registers it, whichever local branch holds its
  configuration.
- The refusal does not enable the repository, and no result enables it
  for the caller.

`publication_pending`, which the CLI RFC's example for `comment add` uses,
is not registered in F2: no command bound in F2 performs a checkpoint
followed by immediate publication. The Cycle that binds `comment add` adds
it to the Incomplete class.

`outcome.rs` maps each of `RepositoryErrorKind`, `SynchronizationError`,
`SshTransportErrorKind`, `KeyMaterialErrorKind` and every domain outcome
variant through an exhaustive `match` with no wildcard arm, so a variant
added later fails to compile until it is given a code. No mapping calls
`Display` or `Debug` on a domain error. Variants with no meaning to a
caller map to `internal_error`. Some codes need the binding's context and
are set by the binding: `host_mismatch`, `slug_already_assigned`, the
confirmation and request codes, `identity_required` for `remote select`,
`ok` for a key the request's own operation already created,
`cancelled` for a synchronization whose row says it was cancelled,
`repository_not_enabled`, `host_replacement_required` for a `prepare` of
`host approve` on a host that has another pin, `discovery_pending` for a
folder creation whose refresh did not complete, and `not_repairable`,
`identity_ambiguous` and `path_not_found` where the repair binding
refuses before the domain is called.

The repository error kinds whose code is not obvious from the name:

| Kind | Code |
| --- | --- |
| `RepositoryNotEnabled`, `RepositoryNotRegistered` | `repository_not_registered`. The code `repository_not_enabled` is not a mapping of any kind: the binding gives it, for a repository the index does not know that was never enabled. |
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
| `OperationAbandoned` | `external_change`: the request's operation was abandoned under it |
| `NotRepairable` | `not_repairable` |
| `AmbiguousIdentity` | `identity_ambiguous` |
| `InvalidItemSource` | `invalid_input`: the corrected source is not a conforming item of its kind at its path, or its closure fields are not the file's |
| `RepositoryBusy` | `busy`. From `set_local_identity` it covers the repository lease and Git's `config.lock` |
| `Io`, `Sqlite`, `Git`, `InjectedFailure` | `internal_error` |

The table test in `outcome_tests.rs` is the complete mapping for all four
error enums and is reviewed as such. The choices in the other three enums
and the outcomes that are not obvious from the name:

| Kind or outcome | Code |
| --- | --- |
| A refresh that another call holds, or that found the repository changing while it observed it | `busy` |
| An index hand-off still owed, from any operation | `discovery_pending` |
| Transport: `ConfigurationInvalid`, `UsernameRequired` | `invalid_remote` |
| Transport: `KeyMissing`, `KeyUnreadable`, `KeyInvalidOrUnsupported` | `key_unavailable` |
| Transport: `KeySourceChanged`, `SelectionChanged`, `EndpointChanged`, `HostTrustChanged` | `external_change` |
| Transport: `UnlockCancelled`, `ProviderUnavailable` | `unlock_required` |
| Transport: `HostVerificationUnavailable` | `recovery_required`: the host gave no key that could be identified, so there is none to approve and a retry changes nothing |
| Transport: `RegistryUnavailable` | `index_unavailable` |
| Transport: `RuntimeUninitialized`, `ProtocolFailure` | `internal_error` |
| Key material: `ImportedKey`, `OwnershipUnverified` | `key_not_deletable` |
| Key material: `SourceMissing`, `SourceUnreadable`, `NotRegularFile`, `UnsafePath`, `InvalidGeneratedKey` | `key_unavailable` |
| Key material: `SourceChanged`, `SelectionChanged` | `external_change` |
| Key material: `NotRegistered` | `key_not_found` |
| Synchronization: `TargetNotMaterialized` | `item_not_found` |
| Synchronization: `PrimaryMissing` | `invalid_configuration` |
| Synchronization: `RemoteContextDeleted` | `remote_branch_deleted` |
| Synchronization: `ConflictPending`, `ExternalResolutionRequired`, `Interrupted`, `HistoryUnknown`, `RecoveryRequired` | `recovery_required`; the binding reports `cancelled` when the row says so |
| Synchronization: a remote that already held the commit | `already_applied` |
| Synchronization: no publication remote | `publication_remote_required` |

`key_rejected` is the rare case, not the common one. The transport cannot
tell a refused key from one that was not unlocked: on an authentication
failure it asks for a passphrase and tries once more, so a client the
server rejects is `unlock_failed` when a passphrase was given and
`unlock_required` when none could be. `key_rejected` is reached when the
server lets the client in without its key.

What a client does with each code that stops a request, as built. The
actions of a result are listed in the order given here, and the order is
the order of preference. "Left `accepted`" means the settlement decided to
leave the request's record `accepted`; see Settling.

| Codes | Recovery action offered | Otherwise |
| --- | --- | --- |
| `confirmation_required`, `confirmation_expired`, `confirmation_not_found` | `request.prepare` | |
| `external_change` on a confirmed command, with no record left | `request.prepare` | What was previewed is no longer what is there; a new preview shows what is. Not offered when a key deletion is refused for a key file that is not the one Manyhands wrote (`external_change` from `SourceChanged`): no new preview clears that, on a first call, on a retry with no row, or when the deletion itself refuses. A registration that differs from the preview still offers it. Not offered either when a repair is refused because the item's context holds, at the path, neither primary's file nor the source nor a conforming item: every new preview would be refused the same way, so the result offers nothing; see the known limits. |
| `external_change` on a confirmed command, left `accepted` | `repo.inspect` | The request's own pending operation refuses every other request, a new one with a new preview among them. With what was previewed put back, the same request goes on. No action is offered when the result names no repository, as for an interrupted key deletion. One exception: a `host approve` or `host replace` that stored the pin and then failed with this code offers `request.retry` instead, because the same request is known to end as a no-op. |
| `external_change` on any other command | none | Read the item again and submit a new request. A synchronization left `accepted` is offered `request.retry` by the synchronization rule below. |
| `discovery_pending` | `operation.resume` naming the operation, where a resume would take it up; otherwise `request.retry` | |
| `discovery_pending` from `folder create` | `index.refresh` with the root; `request.retry` instead while the record is left `accepted` | The failed refresh's row is closed as rejected, so nothing is pending and any refresh lists the folder. The record stays `accepted` only when the row could not be closed, or when another call is at work under it. A replay of the finished result offers `index.refresh` again. |
| `registration_pending` | `request.retry` | |
| `poll_yielding`, `remote_unavailable`, `transport_unavailable`, `busy` | `request.retry` | Offered whether or not a record was left. `busy` from `repo identity-set` covers the repository lease and Git's configuration lock. |
| `identity_required`, `initials_required` | `repo.identity_set`, except for `repo create` | For `repo create` there is no repository yet to set an identity on: resubmit with an identity. For initials, or resubmit with initials. `repo identity-set` takes a name and an email as well as initials, so a user whose identity is global and who lacks only initials writes the name and email locally too. |
| `host_approval_required` | `host.approve`, then `request.retry` when left `accepted` | `data` holds the host and the presented key. |
| `host_replacement_required` | `host.replace`, then `request.retry` when left `accepted` | `data` holds the host, the presented key and the pin. A `prepare` of `host approve` on a host that has another pin gives the same code and the same action, with `authority` and `root`. Its data is another shape, `host_replacement`: the host, the key the request names and the pin, with no `presented_*`, since a `prepare` connects to nothing. |
| `recovery_required`, on a result that names a repository | `repo.inspect` | |
| `recovery_required`, on a result that names no repository | `request.retry` when left `accepted`; otherwise none | A key generation that was interrupted is continued by the same request. For a key whose material is pending no action is offered; see the known limits. |
| `invalid_slug_configuration`, `invalid_configuration` | `repo.inspect` | |
| `repository_not_registered`, `index_unavailable` | `index.rebuild` with the root | Not offered on a result that names no repository: the action requires a root. Such a result, which only a key command gives, offers `request.retry` when its record is left `accepted` and nothing otherwise. A failed `host observe` offers `index.rebuild` as every read and mutation does. |
| `repository_not_enabled` | none | The repository was never enabled; enabling it is a new request. A rebuild would register nothing. |
| `original_request_required` | `request.retry`, naming the request that began the operation | Given only by `operation resume`, and only when an unfinished record holds the operation. The resume itself is never offered again. |
| `operation_not_found` | none | |
| `merge_required`, `remote_branch_deleted` from a synchronization | `request.cancel` naming the request, while the reservation is the request's own and its record is left `accepted`; otherwise none | The same request cannot get past either: the operation recorded the branch as it was, and refuses it once history is merged or rebased; a branch deleted at the remote stays deleted. `request.retry` is not offered. Taking `request.cancel` cancels the request; one more call of the same request then answers `cancelled` and frees the reservation, and a new request goes on. Nothing is pushed. The reservation is read when the result is given: where it is another operation's, or cannot be read, the action is not offered. A replay and a preview offer none. |
| Any other code from a synchronization left `accepted` (`push_rejected`, `no_selected_key`, `key_rejected`, `unlock_required`, `unlock_failed`, `external_change`, `recovery_required` and the rest) | `request.retry` first, then whatever the code offers | The synchronization rule: a synchronization that stopped with its record left `accepted` holds the reservation, and the same request is what takes it up again. See open case 3. |
| `request_mismatch`, `confirmation_mismatch`, `confirmation_used` | none | A caller error; use a new request ID or a new preview. This holds also for a `request_mismatch` that is `partial`, which is one of the two exceptions to the rule that a partial result carries a recovery action: the changed input is refused, and the original request, which this caller may not hold, is what goes on. The other is a key generation whose files were retained for inspection, which is final. |
| `host_mismatch` | none | `data` holds the key the host presented. Whether to trust that key is a new preview with a new input; the same input previewed again would be refused again. The record is released at any attempt. |
| `not_repairable`, `identity_ambiguous`, `path_not_found` | none of their own; `request.retry` when left `accepted` | Nothing in this contract resolves them: the user edits the file, or the duplicate, by hand. A recovery action a read suggested is kept. |
| `publication_remote_required`, `publication_remote_in_use`, `ticket_closed`, `slug_already_assigned`, `remote_name_conflict`, `invalid_remote`, the key codes outside a synchronization, and the Input codes not named above | none, unless left `accepted` | The code and message say what is missing. Actions for these belong with the commands that resolve them and are added when a front end needs them. |
| Any code with no action of its own, `external_change` excepted, on a result left `accepted` | `request.retry`, naming the request | The same request finishes the work. |
| Any failure that left the request's record `accepted` with the request's own local operation pending, whatever its code | `repo.inspect` if not already offered, then `operation.abandon` naming that operation, after every other action | The same request may still finish the work; where it cannot, this is the way out, and the result is where the caller learns the operation ID. This includes `discovery_pending` and `registration_pending`, where the abandon keeps the commit; the product owner confirmed on 2026-10-12 that it is offered there, after the forward action. |
| `recovery_required`, or `external_change` on a confirmed command, that already offers `repo.inspect` | `operation.resume` for each pending local operation of the repository that is not the request's own and that a resume would take up now, before `repo.inspect`; `operation.abandon` for each pending local operation, oldest first, after it | The request that owns an operation is not offered its resume here. The same list is made for `prepare` and for a result given again from a record, as true when it is read. The abandons are offered whatever caused the result, and the preview of the abandon says what it is. A pending local operation does refuse the repository's other operations, with one exception that is offered all the same: a comment submission that waits for its registration refuses only the reservation of a synchronization, so its abandon can be offered to a request it was not in the way of. |

`operation.abandon` is offered under either of the last two rows only for
an operation nothing is seen against abandoning; see
[Abandoning An Operation](#abandoning-an-operation). The codes
`invalid_id`, `public_key_unavailable` and `authority_not_found` are
returned by F1's reads only, and no mutation's result has to decide an
action for them.

A result replayed from a finished record carries no stored action. Two
kinds of action are made again when it is read: the action its code has,
for a finished synchronization, a finished resume or a finished
`folder create`, so that a `recovery_required` with its push not
confirmed offers `repo.inspect`, and a folder's `discovery_pending`
offers `index.refresh`, each time it is given; and `operation.resume` and
`operation.abandon`, which name what is pending when the result is read.

### Recovery actions

Added to the closed registry:

| Action | Arguments |
| --- | --- |
| `operation.resume` | none; `operation_id` is set |
| `request.retry` | `request_id` |
| `request.cancel` | `request_id` |
| `request.prepare` | none |
| `operation.abandon` | `root`; `operation_id` is set |
| `repo.identity_set` | `root` |
| `host.approve` | `root`, `authority` |
| `host.replace` | `root`, `authority` |

`operation.resume` registers no argument keys, matching the CLI RFC's
example, so a caller takes the root for it from `scope.repository`,
while `operation.abandon` carries its root in `arguments`.
`RecoveryAction` gains a constructor that sets `operation_id`.
`host.approve` and `host.replace` carry no key: the caller takes the key
to name from the `presented_*` fields of the result's data.
`index.refresh`, an F1 action, is now offered by a mutation too: a
`folder create` whose refresh did not complete.

### Resuming

Repeating the original request with the same request ID is the general way to
finish it. `operation resume` exists for the work that needs no input from
the caller. It is a bound command, run through `execute` with its own
request ID, so a resume is itself replayable. Its input is the
repository's root and the operation ID. It needs no confirmation: what it
does was consented to when the operation began.

The operation a resume names is another request's. It is the resume's
target and not an operation of its record, so the same request ID with
another operation is `request_mismatch`, and the resume's results carry a
null `operation_id`. The root is required also for a key operation, which
belongs to the application and is found whatever root is named.

**What a resume takes up.** A call reads how the operation stands in one
read of the three journals: the remote journal first, because a
synchronization also has a row of the local journal under its ID for its
index hand-off, then the key journal, then the local one.

| The operation | What the resume does | Result |
| --- | --- | --- |
| A local operation that is pending, whose last recorded step says its own work is done and whose state says it has handed off to indexing | Runs the hand-off: a refresh under the operation's ID. | `ok` with `discovery: current` and no other effect. The write and the commit are the result of the request that made them. |
| A synchronization that has not ended and is not fenced, or has completed and still owes its index hand-off | Makes the call a retry of the synchronization's own request makes: restart, refuse divergence. The remote is asked what it holds before anything is pushed, and nothing is merged. | What that call comes to, from the operation's own evidence, `published` and its commit included. If the hand-off is still owed afterwards: `discovery_pending`, offering `operation.resume` again. |
| No journal holds the ID for the root | Nothing. | `operation_not_found` |
| A local operation closed as abandoned | Nothing. | `external_change`, the code every repeat of an abandoned ID gets |
| An operation that has ended and owes nothing: completed, rejected or cancelled | Nothing. | `already_applied`, a no-op |
| An operation that ended in a way nothing continues and left work recorded as outstanding, such as a synchronization cancelled at its first safe point | Nothing. | `recovery_required` |
| A pending operation only its own request goes on with (a save, whose body no record holds; a key generation or deletion; an enablement that has not registered its repository; a fenced synchronization), while an unfinished record holds it | Nothing. The row is left exactly as it was. | `original_request_required`, with a `request.retry` action naming that request |
| The same, for a fenced synchronization no unfinished record holds | Nothing. | `external_change`, no action |
| The same, for a local or key operation no unfinished record holds | Nothing. | `recovery_required`, with `repo.inspect`, and `operation.abandon` for a local row where the abandon rule allows it |
| A synchronization that a comment submission's binding keeps, where any other synchronization would be taken up: pending and not fenced, or completed with its index hand-off owed | Nothing. The domain is asked nothing and the row is left as it was, so the comment's own retry can still publish it. | `recovery_required`. `operation.resume` is offered for it by no result, and the operation reads give it no next action. A fenced one is still `external_change`, and one that ended owing nothing still `already_applied`. |
| A comment submission that waits for its registration | Nothing. | `recovery_required`, with its `operation.abandon` offered |
| A poll, a promotion or a closure that has not ended | Nothing. No request of the boundary begins one. | `invalid_input`, no action; confirmed by the product owner on 2026-10-12 |

The hand-off rule asks for both the step and the state. The step must be
one of `authoring_checkpoint_observed`, `authoritative_observed`,
`initialization_committed`, `publication_committed` or `remote_changed`,
and the state one of `authoritative_observed`, `indexing`, `failed` or
`retry`. The step alone is not enough: an enablement and a selection of
the publication remote record their commit before they bring the
registration up to it, so such a row can owe more than the hand-off. The
state alone is not enough either: a refresh would complete any pending
row it was given, also one whose write never happened.
`comment_registration_pending` counts as a step that says the work is
done, but its row is stored as completed and so never meets the state
half of the rule. `comment_checkpoint_intent` is left out of the steps by
name: from the intent on a commit may exist, and only Git says whether it
does, which a refresh does not prove.

`operation.resume` is offered on a result only where a resume would take
the operation up, asked with the same question once the call is over.
Where it would not, the request's own result offers `request.retry` in
its place.

**The resume's record.** A refusal before acceptance leaves no record.
Nothing is ever in flight under the resume itself; what is in flight is
the operation's, and the request that began it holds that. So the record
is finished when the answer is final for the operation, deleted when the
resume stopped, and left `accepted` only when nothing could be read of the
operation. A finished resume that took up a synchronization replays the
action its code has, as a synchronization's own result does. A resume of
a synchronization is accepted on the branch that synchronization
publishes, so a replay checks its commit against that branch.

On a retry of an accepted resume, the operation is read again. A
synchronization that has ended, cancelled or not, is asked of the domain
all the same, which answers from the row with no connection, so a resume
whose own lost call published or cancelled is told so. Any other
operation that has ended is a no-op.

**A resume answers to the request that owns the operation.** The owner is
the request whose unfinished record holds the operation.

- **Takeover.** A resume takes a synchronization over exactly as that
  request's own retry does: the restart fences a call that is still
  running. When it takes an operation over, or runs a hand-off, it also
  raises the owner's record's attempt, as the owner's own next call would.
  A call of the owner that was still running then settles nothing when it
  comes back.
- **The owner's cancellation stops a resume.** `request cancel` is asked
  of the request that began the operation, and asks nothing of a resume.
  A resume that takes a synchronization up reads the owner's flag at every
  safe point, under the same rules as that request's own call, and the
  flag is cleared on the owner's record once the operation's row reads
  cancelled. The resume's own token stops it too, and ends the operation
  for good, as it would for the operation's own request.
- **The resume finishes the owner's record.** When a resumed
  synchronization ends with an answer that is final for the operation,
  the owner's record is finished with that answer, at the attempt the
  resume raised it to. Only the call that ended the operation knows
  whether the push endpoint was asked. The owner's request, sent again,
  is then given that answer. If the write fails, the owner's record stays
  `accepted` and its next call answers from the operation's row.

## Progress And Cancellation

```text
ProgressEvent
    request_id, operation_id, command, scope
    stage      accepted | observing | writing | connecting | fetching |
               updating_local | pushing | verifying | indexing | stopping
    effects    completed so far
```

`operation_id` is null for a request that has no operation. An event
carries no body, source, commit message, remote location, credential or
server text, and names no commit before the request has published one;
the request's result is what names a commit.

`ProgressSink` has one method, `event`, called on the thread running
`execute`. `execute` goes on only when it returns, so a sink that waits
holds the request where it is, and every event of a call has been sent
before `execute` returns the call's result. A sink that panics is sent
nothing more by that call; the panic is caught and the request runs on.
`NoProgress` is the sink for a caller that wants none.

**When an event is sent.**

- `accepted` is sent once, by the call that accepts the request, after its
  record is written. A call that re-enters an accepted request does not
  send it again.
- Immediately before each domain call, first or repeated, the command's own
  stage is sent: `indexing` for `index refresh` and `index rebuild`,
  `connecting` for the two synchronizations and for `host approve` and
  `host replace`, `observing` for `operation resume`, and `writing` for
  every other command, `repo identity-set`, `folder create` and the
  repairs among them.
- A synchronization sends one stage at each of its seven safe points:

  | Safe point | Stage |
  | --- | --- |
  | Before the fetch | `fetching` |
  | After the fetch | `observing` |
  | Before the local update | `updating_local` |
  | After the local update | `observing` |
  | Before the push | `pushing` |
  | After the push returned | `verifying` |
  | After the push was verified | `indexing` |

  `observing` is used twice: after the fetch, while what was fetched is
  compared with what is here, and after the local update, while the remote
  is asked what it would be pushed.
- `stopping` is sent when the request has seen a cancellation at a safe
  point and is about to ask the synchronization to stop there.
- A resume sends `observing`, then `indexing` before a hand-off it runs, or
  the stages of a synchronization it takes up. Its events carry the
  resume's request ID and a null `operation_id`.

An event's `effects` say what is published only once it is. Before the
push has returned or the remote has been asked, `publication` is `pending`.
After that it is `published` with the commit, or `current` when the remote
already held it.

The safe points are reported by an observer the executor calls in
production builds. `synchronize_remote_observed` takes the observer and
the settings, and `synchronize_remote` delegates to it with an observer
that always continues and the default settings. The stages
`updating_local` and the `observing` that follows it are reported while
the repository lease is held, so a sink must not call back into the
library; the trait's documentation says so.

**Asking for a cancellation.** There are two ways, and both end at Wave
02's own mechanism: the synchronization's row is marked as asked to stop,
and the safe point it has reached honors that.

- **`CancellationToken`**, a cloneable flag, for the same process. The
  boundary checks it once before acceptance, on a first call only. During
  a synchronization the observer checks it at every safe point.
- **`cancel_request`**, for a second thread or process. It sets
  `cancel_requested` on the request's record. When the synchronization's
  row is pending and has not yet recorded its intent to push, it also
  calls `cancel_remote_operation`; a failure of that call loses nothing,
  because the flag stands. The observer reads the flag at every safe
  point, so a request made before the reservation exists is seen at the
  first one.

`cancel_request` returns a `CancelDto`:

```text
CancelDto
    request_id
    state          accepted | finished, as the request stood when asked
    requested      the request was asked to stop
    acknowledged   the request has stopped for a cancellation
```

| The request | What `cancel_request` does | `requested` | `acknowledged` |
| --- | --- | --- | --- |
| No record holds the ID | Nothing. The result is `request_not_found`. | | |
| Finished | Nothing. | false | true if its stored code is `cancelled` |
| Accepted, of a command with no safe point | Nothing. A local operation cannot be stopped once it is accepted, and its record stays open for the retry that finishes it. | false | false |
| Accepted, a synchronization that was fenced | Finishes the record with the fenced answer below and reports `state: finished`. Nothing will take the operation up again, so nothing is asked of it. | false | false |
| Accepted, any other synchronization | Sets the flag, and asks the operation directly as described above. | true | true if the operation's row already reads cancelled |

Apart from the fenced case, `cancel_request` never finishes or deletes a
record. A request that stops is finished by the call that was running it,
or by its next call. While a blocking transport call has not returned, the
request is `requested` and not `acknowledged`; the ten-second "still
stopping" display and its timer belong to D6.

**The flag is durable.** It is a column of the request record. The
observer reads it and never clears it. It is cleared only once the
operation's row reads cancelled. A call that dies, that cannot record the
cancellation on the operation, or that has lost the operation to another
call therefore leaves the flag for the next call, which stops before it
pushes. If the flag cannot be read at a safe point up to and including the
one before the push, the call does not take that for "not asked": it stops
there with `recovery_required`, the operation is not cancelled and keeps
its reservation, the record stays `accepted`, and the same request goes
on once the flag can be read. After the push has returned, a flag that
cannot be read changes nothing: the answer says published either way.

**Where a cancellation is honored.**

- **Every command:** before acceptance, on a first call. The result is
  `cancelled`, with no record and no effect.
- **Synchronization:** at the seven safe points, through the executor's
  own check.
- **Local domain operations:** none inside the call. A save holds its lease
  for milliseconds and is not interruptible.

**What a cancelled synchronization answers** depends on how far its push
had got. The rule is that a final answer never understates a push.

| Where the operation stood | The answer |
| --- | --- |
| No push intended yet | `cancelled`, `publication: pending`, no commit. Nothing was published. |
| The push returned, or the remote was asked and holds the commit | `cancelled`, `publication: published`, the commit, `discovery: pending`; `publication: current` and no commit when the call saw that the remote held the commit before this operation pushed anything. That the push returned is recorded the moment it does, before the safe point that follows, so a cancellation honored there ends an operation whose row says the remote took its push. |
| A push was intended and its outcome is not known: an earlier call recorded the intent and lost the result | The cancellation is held back until this call has asked the push endpoint. If the endpoint holds the commit, or a descendant of it, the answer is as in the row above. If it does not, the call stops at the safe point before the push would be made again: `cancelled`, `publication: pending`, nothing pushed. |
| The same, and the push endpoint cannot be asked: the call stops with an error before it gets that far | The cancellation is honored all the same, so that it always frees the repository, offline if need be. The operation is asked to stop, and one more call under its ID, which reserves nothing and opens no connection, acknowledges it. The reservation is released and the record is finished with `recovery_required`, `publication: pending` and the `repo.inspect` action. `pending` here means not confirmed: neither published nor not published. A new synchronization tells which. |

A claim about a request's push comes only from that request's own
evidence: what its row recorded, what its call saw of its own push, and
what the push endpoint advertised to it. What a fetch shows is never used,
because a remote with a push URL of its own is another place.

A result is `cancelled` exactly when the operation's row says it was
cancelled, whichever error told of it. A cancelled synchronization is
final for its operation ID: the record is finished, the same request gets
the stored answer again, and continuing needs a new request. If work
finished before the cancellation was seen, the result is the success it
earned. F2 claims no rollback and no deadline. A synchronization with a
conflict pending is not ended by a cancellation, and a merging
synchronization reaches no safe point between the fetch and the push.
Neither arises in F2, whose bindings refuse divergence; C4 and D5 meet
both.

**A fenced synchronization.** A change of the publication remote's
endpoints or of the selected key raises the registration's configuration
generation. The domain then lets the reservation go and takes the
operation up for nobody: a restart refuses a row of another generation.
A request whose operation is pending under an older generation is
*fenced*. The same request cannot go on, so it is given a final answer,
its record is finished, and that answer is replayed. Which answer depends
on how far the push had got:

| Where the operation stood | The answer | Action |
| --- | --- | --- |
| No push intended | `external_change`, no effects; outcome `blocked` | none |
| The row says the remote holds the commit | `external_change` with `publication: published`, the commit and `discovery: pending`; outcome `partial` | none |
| A push intended and its outcome not known | `recovery_required`, `publication: pending`, meaning not confirmed | `repo.inspect` |

The answer is given by whichever comes first: the call that stopped, the
request's next call, which then makes no domain call, or `cancel_request`.
If the journal cannot be read, no final answer is given and the record is
left as it is. A fenced result never offers `request.retry`: its record is
finished and the same request cannot go on. Confirmed by the product owner
on 2026-10-12.

**Divergence is refused.** Both synchronization bindings always set
`refuse_divergence`, so F2 never merges. A branch whose local and remote
history have diverged is `merge_required` before anything of a merge is
prepared: no integration window, merge commit or conflict is left and no
branch moves. The same refusal covers a retry after a lost push when the
remote or the local branch has since moved on, which would otherwise open
a publication attempt and an integration window. An operation that
already holds a merge of its own, begun by a caller that does merge, is
`recovery_required` and is not continued. A request stopped by
`merge_required` still holds the reservation, and its result offers
`request.cancel`; see [Result codes](#result-codes).

## Abandoning An Operation

Decision 13. Drafted on 2026-10-10; approved without independent review.

A pending local journal row blocks every other operation on the repository
until the same operation is repeated with input it accepts. Open cases 1
and 2 of [The Journal Defect](#the-journal-defect) are the situations where
it cannot be. `abandon_operation` is the way out.

This section describes `abandon_operation` and its binding as built.

- **What it does.** Under the bootstrap lease and, where a repository
  opens at the root, the repository lease, it closes one pending row of
  `operation_records`, named by root and operation ID, in one transaction.
  It touches no file, ref, branch, worktree or Git configuration; the
  lease's own lock file is the only file it can create. It undoes nothing:
  a commit the operation made is kept.
- **What it reports** (`OperationMutationDto`): the operation ID; its
  action, as `operation show` names it; the state and the last recorded
  step it was closed in; the item it wrote and that item's worktree, when
  it writes one; the paths of its target that differ from their branch
  head, with `paths_complete`; `committed`, which says the operation had
  recorded that it passed the point where its work is committed; and
  `released_request_id`. For an authoring operation the paths are every
  path of the item's own worktree that differs from its head, untracked
  ones included. For `enable`, `create_and_enable` and
  `set_publication_remote` it is the configuration file alone. At most 100
  paths are listed. Names and paths are reported, never content.
- **What it leaves.** An uncommitted item file stays in its worktree; the
  next save of that item reports `external_change`, and a fresh read and
  save commits it. A configuration file written by an interrupted
  `enable` or `remote select` stays dirty; the user restores or commits
  it with Git and repeats the command.
- **How the closed row is stored.** The row keeps its place with the
  state `completed` and the step `abandoned`, distinct from `rejected`.
  No migration was needed, because every question of whether anything is
  pending already reads the state. The marker replaces the step the row
  had reached; what that was is in the abandon's result only.
  `operation show` reads the row as completed and abandoned with no next
  action, and it is not in the operation list. For settling it reads as
  `Absent`, like a rejected row.
- **What refuses an abandoned operation.** A repeat of the ID is refused
  with the new error kind `OperationAbandoned` before anything is written;
  it is neither replayed as completed nor begun again. Every write to the
  journal by operation ID refuses an abandoned row: a call that was still
  running under it stops at its next step with `OperationAbandoned` and
  writes nothing more. A synchronization whose index hand-off row was
  abandoned is refused and not reported as complete, and no reservation
  is given under an abandoned ID. At the boundary `OperationAbandoned` is
  `external_change`: what the request was based on is no longer there to
  go on with.
- **The owning request.** In the same transaction, the unfinished request
  record that holds the operation is deleted and the confirmation it
  accepted is released, whatever attempt the record is at. That request
  ID is free, and its next use is a new request with a new operation ID,
  answered by what the repository then shows. A finished record is no
  owner and is kept with its result.
- **What refuses an abandon.**
  - An operation ID the remote or key journal holds is `invalid_input`,
    also when a local row exists under it: a synchronization's index
    hand-off is a local row under the synchronization's own ID. A held
    reservation has `request cancel`, and key operations complete on
    their next call.
  - A call still at work under the operation makes the abandon `busy`,
    with `request.retry`; the abandon's record stays `accepted`, so the
    retry is honored however old its confirmation. See the liveness rule
    below.
  - A row that completed, was rejected, advanced or disappeared between
    the preview and the execution is `external_change`.
  - A pending merge conflict is not covered; it is ticket
    `01M4H33R34Z7C7EEKTY1ZCT950`.
- **Binding.** `operation abandon`, confirmation required. Its input is
  the root and the operation ID. The operation is the request's target
  and not an operation of its record, so the same request ID for another
  operation is `request_mismatch`. `prepare` answers
  `operation_not_found` when no local row holds the ID for the root, and
  a no-op, `already_applied`, with no confirmation, for a row that has
  completed, was rejected or is already abandoned. The preview's effects
  are `operation_closed` and, when an unfinished request holds the
  operation, `request_released`. Its details show what the result shows,
  the request that will be released, and two flags that are always false:
  `files_changed` and `git_state_changed`. The observation binds the
  row's state and step. The preview takes no lease and does not say
  whether a call is still working under the operation; the execution
  finds that out. Every effect of the result is `not_requested`; what was
  left is in `data`. A retry after the abandon's own lost result finds
  the row abandoned and is `already_applied` with no data.
- A refresh or rebuild row can be abandoned too, though repeating it is
  simpler.

**A comment submission's rows.** Wave 02 Cycle 07's `submit_comment` is on
the branch and is bound by no command, so its rows exist only where
something calls the service directly. The journal's readers are truthful
to them all the same. A submission has two operations: its own local row,
under the caller's operation ID, and a synchronization under a second ID
that its binding in `comment_publication_bindings` keeps.

The three steps a submission adds, and how each is read:

| Step | A checkpoint step | Says the work is done, for a hand-off | Kept when the row goes on |
| --- | --- | --- | --- |
| `comment_destination_prepared` | no | no | yes |
| `comment_checkpoint_intent` | yes | no, by name | yes |
| `comment_registration_pending` | yes | yes | yes |

None of the three follows an owned write. A row stored as `completed`
with the step `comment_registration_pending` is a comment that is
committed and has no registration. It counts as pending for an abandon,
for the operation list and for the abandon offer, where every other
question of "pending" reads the state alone
(`recovery::waits_for_registration`). It refuses the reservation of a
synchronization and no other local operation.

What the reads and the two commands answer for the submission's own row:

| State or step of the row | `operation list` and `show` | `operation abandon` | `operation resume` |
| --- | --- | --- | --- |
| `created`, `branch_observed`, `worktree_observed` | listed | closed, `committed: false` | `recovery_required`, abandon offered |
| `comment_destination_prepared`, `authoring_destination_observed` | listed | closed, `committed: false`; the comment's file is in `paths` if it was written | `recovery_required` |
| `comment_checkpoint_intent` | listed | closed, `committed: true`; see the known limits | `recovery_required` |
| `authoring_checkpoint_observed`, no hand-off begun | listed | closed, `committed: true` | `recovery_required` |
| `authoritative_observed`, `failed`, `retry`, or `indexing` with an old heartbeat | listed | closed, `committed: true` | the hand-off is run: `ok`, `discovery: current` |
| `indexing` with a fresh heartbeat | listed | `busy` | the refresh cannot claim the row; its code, the row untouched |
| `completed` with `comment_registration_pending` | listed | closed, `committed: true`, no paths | `recovery_required`, abandon offered; nothing changed |
| `completed` with another checkpoint step | not listed; shown by ID | `already_applied` | `already_applied` |
| `completed` with `rejected` | not listed | `already_applied` | `already_applied` |
| `completed` with `abandoned` | not listed | `already_applied` | `external_change` |

And for its synchronization, named by the binding's ID:

| State | `operation list` and `show` | `operation abandon` | `operation resume` |
| --- | --- | --- | --- |
| No remote row yet | absent | `operation_not_found` | `operation_not_found` |
| Pending, not fenced, whatever it holds | listed, with no next action | `invalid_input`: another journal's | `recovery_required`, nothing changed |
| Fenced | listed | `invalid_input` | `external_change` |
| Completed, index hand-off owed | listed, with no next action | `invalid_input` | `recovery_required`, nothing changed |
| Completed or cancelled, nothing owed | not listed | `invalid_input` | `already_applied` |

A resume refuses a comment's synchronization in every state in which it
would take another synchronization up, until `comment add` is bound. A
resume would publish the context as it stands and end the
synchronization, without the compound operation's check that what is
published holds the comment; where the context had been reset, that makes
the receipt unpublishable for good, which the comment's own retry does
not. The product owner's ruling of 2026-10-13 is that the boundary leaves
it. The cost is that the boundary offers no exit for such a
synchronization before C4 or D4; it cannot create one either.

A refresh, or a `folder create`, under the ID a comment's binding keeps
for its synchronization is refused before anything is changed, unless it
is that synchronization's index hand-off. The check is one function,
`comment_child_may_begin`, asked by `begin_or_reconcile` and by
`recovery::require_refresh_can_begin`, beside `recovery::in_the_way`.

**An abandoned submission is never published.** The abandon closes the
submission's row and keeps the commit, the row and the binding. After it
the submission is refused in three places with `OperationAbandoned`:
where its binding is read, which is the first thing its replay, its
publication-only retry and its cancellation do; and inside the
transaction that would reserve its synchronization. The second is what
holds against a call that was already past its proof of the checkpoint
when the abandon landed: nothing is reserved, nothing reaches the remote,
and that call answers saved with publication pending, not an error.

**Liveness.** An operation is abandoned only when nothing shows a call at
work under it.

- Every local operation holds the repository lease from before its row
  until it ends or hands off to indexing, and a creation holds the
  bootstrap lease before a repository exists. The abandon takes both.
- After the hand-off no lease is held. The call that is indexing touches
  the row's `observed_at` once a second, and a rebuild does the same for
  as long as it works under its row. A row in the state `indexing`, or of
  a rebuild, whose heartbeat is not older than three seconds is live.
  When a rebuild ends with its row still pending, it sets the heartbeat
  to a stale time, so its own result can offer the abandon at once.
- **A second live call of the same request leaves a live row alone.**
  Nothing serializes two calls of one request ID once both have entered.
  A call that finds its own operation's row pending under a live
  index-owner heartbeat does not record a step on it, which would take it
  out of `indexing` and let the row be claimed from under the call at
  work. It leaves the row's state, step and heartbeat as they are and
  answers index-pending: `discovery_pending`, with the request's commit,
  where it made one, read from Git as on any re-entry, and the record
  kept. One shared
  check, `RepositoryService::is_being_indexed`, gives the rule to
  `save_document`, `save_ticket`, `repair_item`, `add_remote`,
  `remove_remote`, `set_publication_remote`, `enable` and
  `submit_comment`; a `create_and_enable` that is taken up again goes
  through `enable`. Nothing is written. A comment differs in its answer:
  the compound operation has no index-pending outcome. Its guard is
  reached only before the checkpoint, by a row a refresh has claimed, and
  answers `RecoveryRequired` with the row untouched and nothing
  committed; after the checkpoint the operation's own replay holds the
  rule and answers saved with its local indexing pending. The first call is therefore
  never answered `recovery_required` for a request whose record finishes
  `ok`. `create_folder` guards the close of its refresh row the same way:
  a row with a fresh heartbeat is not closed, and the request stays
  `accepted` and offers `request.retry`. A process that died while
  indexing looks live for three seconds; see the known limits.
- Three short windows have neither a lease nor a heartbeat: a creation
  between dropping its leases and going on as an enablement, a
  synchronization between binding its hand-off row and refreshing, and an
  authoring write between dropping its lease and completing its row. A
  row abandoned there is found closed by the live call at its next step,
  which is refused and writes nothing more. Two operations never run at
  once.

`operation.abandon` is offered as a recovery action only for a pending row
of the local journal alone, with no live heartbeat, while the repository's
lease and bootstrap lease are both free. The leases are looked at without
waiting and without creating a lock file. A row without an operation ID
cannot be named and is not offered.

## Bridges

### Local identity

`set_local_identity` writes `user.name` and `user.email`, and optionally
`manyhands.initials`, to the repository's local Git configuration under the
repository lease. It never writes global configuration. It works on any
non-bare repository root, enabled or not, and is idempotent: equal values
are a no-op. It has no journal row, and a pending local operation does not
refuse it: an operation that stopped for want of an identity is what this
command unblocks. Only the lease does.

**Input.** `repo identity-set` takes the root, a name, an email and what
to do with the initials: leave them, clear them or set them. Initials are
stored in lower case, so setting `QA7` over `qa7` is a no-op. A root that
is a linked worktree's, or a directory below the repository's root, is
`not_repository_root`, as for every other command: a worktree shares the
configuration of the repository that owns it, which is named by its own
root. A path below a root offers `repo.inspect` of the root it lies in;
a linked worktree's own root offers nothing, as for every command. The
binding decides this before the domain operation, which has no error
kind for it and by itself answers `invalid_path`. A bare repository and a path that is
no repository are `bare_repository` and `not_repository` at `prepare`.

**Validation.** A name or an email is `invalid_input` unless all of these
hold:

- it is not empty, holds no control character, and has no white space at
  either end, a no-break space included;
- an email holds an `@`;
- Git accepts the pair as a signature and returns both unchanged;
- written to a scratch Git configuration file and read back, it is equal.

The round trip is what refuses a value Git would strip or that no commit
can carry, such as `Ada <ops>`, `;`, or a name ending in `, : ; " \ '`.
The control-character rule stands beside it because an embedded newline
does round-trip. Initials are two or three ASCII letters or digits.
Validation writes nothing under a repository or a home: its scratch file
is in a `0700` directory of the call's own in the system's temporary
directory, removed with it.

**One value for each key.** A `user.name`, `user.email` or
`manyhands.initials` that is said twice in the repository's own
configuration, or that an included file holds, is refused as
`recovery_required` with `repo.inspect`, at `prepare` and at `execute`,
whichever keys the request would write. What the configuration holds is
then not one value, and the no-op and stale-preview rules compare values;
nothing the command can write puts it right, so the user edits the file.
An included file that says nothing of the identity is no obstacle.

**The write** is one atomic replacement under Git's own lock protocol.
Nothing is ever restored, so nothing of another process can be erased and
a partial write cannot exist.

1. Under the repository lease, the configuration file's real path is
   resolved, so a `.git/config` that is a symbolic link is followed: the
   lock is beside its target and the target is what is replaced, and the
   link stays a link. A link that names nothing is `recovery_required`. A
   repository with no configuration file is treated as having an empty
   one and is given one.
2. `config.lock` is created beside the file, exclusively. If it exists
   the call is `busy` with `request.retry`, and the lock that was there is
   left as it is, however old. On Unix the lock is given the file's mode.
3. Under the lock, the file is read. Its keys are checked again for the
   one-value rule, and the identity it holds is compared with what the
   caller observed. If they differ, the call is `external_change` and
   nothing is written. This is the compare of the previewed values, and
   it is made here and nowhere earlier in the domain call.
4. The file is copied to a scratch file in a `0700` directory
   `manyhands-identity-*` in the same Git directory, so that an include
   with a relative path resolves from the copy as from the file. Git sets
   the keys in the copy. Git cannot edit the lock file in place: it would
   replace the held lock with a file of its own making, once for each
   key.
5. The copy is read back through Git. What it holds is what the result
   reports, and it must equal what was asked.
6. The copy's bytes are written to `config.lock` and synced. Immediately
   before the rename the configuration file's bytes are compared with
   what was read at step 3, so a program that ignores the lock is caught
   too: `external_change`. Then the lock is renamed over the file.
7. On any failure the lock and the scratch directory are removed and the
   file is as it was.

A change to another key that someone makes after the preview is kept. A
change Git makes while the lock is held is refused by Git and not lost.

**Preview and consent.** The preview shows the values the local
configuration will hold, the local values now, and the effective identity
with its sources. Its one effect is `identity_written`. A request whose
values the configuration already holds is a no-op, `already_applied`, and
gets no confirmation. The confirmation covers the local values only; a
global identity that changes after the preview does not make the request
`external_change`.

**Retries.** The command has no operation and no `expected_digest`: no
SQLite transaction can hold a write to Git configuration, so a row
written before or after it would have a window in which it says the wrong
thing. A retry asks the configuration itself, through `confirm::reenter`:
it holds what the request asks, and the request is finished as a no-op;
it holds what was previewed, and the domain is called; anything else is
`external_change`, with the record released and `request.prepare`
offered. A first call that fails releases its record; a retry that fails
keeps it and offers `request.retry`. `busy` for a locked configuration
follows the same rule. A re-entry that cannot read the configuration
keeps its record, also when the root has gone or become bare.

**Result.** The data is the name, email and initials the local
configuration holds once the request is done. Every effect is
`not_requested`.

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

`IdentityDto` gains `initials` and `initials_source`. The source is one of
`repository`, for a configured `manyhands.initials`; `derived`, from the
effective name; and `none`. A configured value that is not valid initials
reads as `none` with `initials` null, and nothing is derived in its place:
that is what composing a short code does, which answers
`initials_required`.

### Host approval

A host is identified by its authority, the lower-cased host and port.

**Learning what a host presents: `host observe`.** The CLI RFC gives
`host inspect` a repository "for an observation", and nothing in the
baseline lets C2 or D1 see a fingerprint before a synchronization fails.
`observe_host` takes the root, an optional authority and the credentials.
The authority is a host the publication remote names; with none given it
is the host the remote is fetched from. It calls `verify_ssh_transport`
with no approval, in the direction the remote uses the host for, and a
host the remote uses for both is connected to as for fetching. It is a
network read with no request ID, it pins nothing, and it needs the
selected key, as the verification does.

Its data, `HostObservationDto`, holds `host` and `port`; `fetch` and
`push`; `presented_algorithm` and `presented_fingerprint`, the key the
host presented; `pinned_algorithm` and `pinned_fingerprint`, the pin
stored for the host, or null; and `reapproval_required`. A host with no
pin and a host whose key changed are successes; the data says which. The
presented key is what a caller hands to `host approve` or `host replace`
as `algorithm` and `fingerprint`.

The envelope's command is `host observe`. F1's `inspect_host`, which
reads the stored pins and connects to nothing, stays `host inspect` with
the `host_pin` data; a consumer tells the two by the command. A failed
observation keeps the recovery action its own read suggested and
otherwise the code's own, decided in `outcome.rs`
(`offered_by_observation`) with no command's rule applied, so its
`external_change` offers no `request.prepare`:
`repository_not_registered` and `index_unavailable` offer `index.rebuild`
with the root, and a code whose action is `repo.inspect` offers that. An authority the remote does not name is `invalid_input`
and opens no connection. Without a publication remote it is
`publication_remote_required`. Its other failures are the codes of
resolving the root, `invalid_configuration`, the transport's codes and
`external_change`.

**Approving.** The approvable host is one the selected publication remote
names, for fetching or for pushing; those are the only hosts the
verification connects to. The input of `host approve` is the root, the
authority, and the `algorithm` and `fingerprint` of the key to pin; `host
replace` also takes `old_fingerprint`. The algorithm is in the input
because an approval compares a whole key identity, and with a fingerprint
alone the binding would have to connect once to learn the algorithm and
again to approve.

`prepare` is local and opens no connection. In order:

- An authority that is not the publication remote's is `invalid_input`.
  The host is compared in lower case and the port must be the remote's,
  with 22 for a remote that names none. A trailing dot, a user, a path,
  `host:port` written in the host, brackets and a leading space are all
  `invalid_input`. An IPv6 address is accepted as the remote writes it or
  written out in full.
- A host already pinned to the key the request names is a no-op,
  `already_applied`, with no confirmation. This is answered before the
  boundary compares digests, on a first call too, so that a call that
  stored the pin and lost its record is not refused as stale.
- `host approve` on a host that has another pin is
  `host_replacement_required`, with the `host.replace` action that code
  carries from a synchronization, whose arguments are `authority` and
  `root` and no key. Its data is not a synchronization's. It is a shape
  of its own, `host_replacement`: `host`, `port`, the `algorithm` and
  `fingerprint` the request names, and `pinned_algorithm` and
  `pinned_fingerprint`, all set. It has no `presented_*`, since a
  `prepare` connects to nothing and saw no key of the host. The caller
  forms the `host replace` from it: `pinned_fingerprint` is the
  `old_fingerprint`. This is the one `prepare` refusal that carries
  data.
- `host replace` on a host with no pin, or with an old fingerprint that
  is not the pin's, is `invalid_input`.
- Otherwise the preview is given, with the effect `host_pinned` or
  `host_pin_replaced`.

`execute` builds the `HostApproval` value, whose expected pin is the pin
the call saw and whose presented key is the key the request names, and
calls `verify_ssh_transport` for the host's direction. That function
authenticates with the selected key, compares the presented host key with
the approved one, and writes the pin only if they match, in one
transaction that also refuses a pin that moved after the call read it.
It fetches nothing, pushes nothing and updates no ref.
`finalize_host_trust` remains the only code that writes a pin; F2 adds no
pin writer. The request has no operation, and its stage is `connecting`.

- If the host presents a key that is neither its pin nor the approved
  one, the domain reports that approval or replacement is still required,
  with the key presented. The binding returns `host_mismatch` with that
  key in `presented_*`. No pin changes.
- If a replacement names a key the host does not present, and the host
  presents its pin, the domain authenticates and then refuses the write
  as a change of host trust, without the key it saw. The binding reads
  the pin and the reapproval marker again. Where neither moved, the host
  presented its pin, and the result is `host_mismatch` with the pin as
  `presented_*`. Where either moved, or where the connection reached
  another host than the one the request names because the remote was
  pointed elsewhere in the meantime, the result is `external_change` and
  names no key.
- `host_mismatch` and `external_change` release the record at any
  attempt and offer nothing and `request.prepare` respectively.
- If the key needs unlocking and the provider cannot supply a passphrase,
  the result is `unlock_required`.
- A connection that drops before the host's advertisement stores nothing.
- Every transport code a synchronization can give, a host approval can
  give.

**A call that fails after the pin was stored.** The verification stores
the pin and then still reads the advertisement and rechecks the selection
and the endpoints, and any of those can fail. So after every answer that
is not final, the binding reads the pin store once more. Where the host
is pinned to the key the request names, the answer keeps the code of what
stopped the call and is made truthful: the outcome is `partial`, the data
is the host and the key with `presented_*` null, the record is kept, and
`request.retry` is offered whatever the code offers of its own. The retry
finds its end state and is `already_applied`, with no connection. Where
the pin store cannot be read, the answer stands as it was.

A request ID reused with changed input while a host approval's record is
`accepted` is `request_mismatch`, and `partial` whenever the host no
longer shows what the request's preview showed. The commands have no
operation, so no row says an attempt ran; what says nothing was done is
that the host still reads as the digest on the record's confirmation
holds it. Anything else is taken as a change made: another pin, a remote
that no longer names the host, or a pin store or a confirmation that
cannot be read. This cannot tell the request's own pin from one another
actor stored while the record was accepted.

**Retries.** The pin store is the evidence, with the consent's
observation digest. On re-entry `confirm::reenter` asks: the host is
pinned to the key the request names, so the request is finished as a
no-op with no connection, whoever wrote the pin; the pin and the remote
are what was previewed, so the host is connected to; anything else is
`external_change` with the record released, `request.prepare` and no
connection. A first call that fails with no pin stored releases its
record; a retry that fails keeps it.

Requiring the selected key to authenticate is stricter than the
authentication RFC, which asks only that "the presented key is observed
again". It is what the existing verification does, and F2 adds no second pin
writer. This is decision 6.

### Folder creation

`create_folder` creates one directory under `docs/` in the primary
worktree, with the directories above it that are missing, and then
refreshes the index so that the folder is listed. It creates no file and
no commit. Git does not track an empty directory, so the worktree stays
clean, every reference is as it was, and an empty folder does not survive
a clone: it exists on this machine until a document is saved in it.

The request has one operation and one journal row, the refresh's, under
the request's operation ID. No row is made for the directory, no journal
action or step was added, and the row is a `refresh` row. The command
needs no confirmation, its stage is `writing`, and it carries no data.

**The path.** `DocumentFolderPath::parse` is the one constructor. It
reads no file system, and its rules are the same on every platform, so a
path made on one is not a different folder on another. A path that is not
valid as a folder path is `invalid_path`, as a document path that is not
valid for its kind is for `document create`. Refused:

- a path that does not begin with exactly `docs/`, compared with case, or
  that is `docs` itself;
- an absolute path: a leading `/` or `\`, a UNC path, a drive prefix;
- an empty segment, and a `.` or `..` segment;
- a name holding `\ : * ? " < > |` or any control character: Windows
  separators, drive and alternate-data-stream colons, and characters NTFS
  refuses;
- a name holding one of the code points HFS+ ignores when it compares
  names, U+200C to U+200F, U+202A to U+202E, U+206A to U+206F and U+FEFF,
  with which `.g‌it` would be `.git` there;
- a name ending in a full stop or a space, which Windows drops;
- a Windows device name, `CON`, `PRN`, `AUX`, `NUL`, `COM0` to `COM9`,
  `LPT0` to `LPT9`, `CONIN$` and `CONOUT$`, in any case and with any
  extension;
- `.git` in any case, and `git~1`, its NTFS short name;
- a name over 255 bytes of UTF-8.

A path that would cross a limit of discovery is `invalid_input`: more
than 16 names below `docs`, or existing entries plus the directories to
make that come to more than 1,024 under `docs/`. Past either limit
discovery fails for the whole worktree and every save stops, so a folder
must not be the thing that crosses it. The entries are counted as the
refresh's own walk counts them. Open ticket `01M4EHGE9TMR99VYZC184J9XEC`
covers those limits themselves.

`prepare` reads only the path's text and resolves the repository. Every
check that needs the file system is the domain's, under the repository
lease, so a request the domain refuses was accepted first: its record is
deleted and its envelope names an operation ID.

**What the domain does**, under the lease, in order:

1. It checks the registration, and refuses before anything is made on
   what its refresh would be refused on: a pending local operation, or a
   remote operation's reservation. The result is `recovery_required`
   with `repo.inspect`; no refresh is offered, since one would be
   refused. This is one question, `recovery::require_refresh_can_begin`,
   shared with every operation that begins a row. A folder that is
   already there is refused the same way.
2. It observes the path, segment by segment, each looked up in its
   parent's own listing. An entry of exactly the name is used. With no
   such entry, one that differs only by case is `occupied_path`, on Linux
   as on Windows and macOS: otherwise `docs/reports` would be a second
   directory on Linux and `docs/Reports` itself elsewhere. A file at the
   path is `occupied_path`. A symbolic link anywhere in the path, inside
   or outside the repository, as a parent or as the folder itself,
   dangling or not, is `invalid_path`; so is a junction on Windows.
3. It checks the limits.
4. It marks the registration stale, before any directory is made. A call
   that cannot write the mark makes nothing.
5. It makes the directories through the existing no-follow walk, which
   opens each directory without following a link, so a link the
   observation missed is still refused.
6. It runs `refresh_repository` under the request's operation ID.

A directory that is there is a no-op, `already_applied`, and is still
refreshed: `discovery: current`. That keeps one path for first calls and
retries, and means that after any `success` or `noop` the list holds the
folder, so a folder made by hand is put in the index by asking for it.
The cost is that a no-op takes a refresh, leaves a completed `refresh`
row, and can be refused or left `partial` as a creation can. The product
owner approved it on 2026-10-13.

**A refresh that does not complete.** The folder is there and the index
is behind: the result is `partial`, `discovery_pending`, with
`discovery: pending`. The domain closes the refresh's row as rejected, so
nothing is left pending and no other refresh is refused, the registration
stays stale, and the record is finished. The action is `index.refresh`
with the root, and a replay of the stored result offers it again. Two
cases keep the record `accepted` and offer `request.retry` instead: the
row could not be closed, and the row has a fresh heartbeat, which means
another call of the same request is at work under it and its row must not
be closed from under it. A failure that left directories and did not
reach the refresh is `partial` with the code of what stopped it.

**Retries.**

| What the earlier call left | What the retry does |
| --- | --- |
| A finished record | Replays the stored result. Nothing is looked at or made, also when the folder was since replaced by a file. |
| An accepted record whose refresh row completed | `already_applied`, `discovery: current`, answered from the row with no domain call and no second refresh. A folder removed since is not made again. |
| An accepted record with no row, or a pending row | The domain is called again. It finds the directory and takes the refresh up under the same ID. |
| An accepted record whose row was closed as rejected | The domain is called again and the refresh is tried again; `discovery_pending` again if it fails. |

A directory is never what occupies a path, so no retry answers
`occupied_path` for a directory its own first call made. A retry that
stops with no row, such as one that finds the lease held while the first
call is still making directories, keeps its record.

A call that died while refreshing leaves a pending `refresh` row. The
operation list and `operation show` give it as a refresh;
`operation resume` answers `original_request_required` naming the folder
request; `operation abandon` closes it and releases the request, and
leaves the directory.

The CLI RFC also lets a folder be created "in the selected item's context
when organizing an existing item". A document move already creates the
directories it needs, so F2 does not add that form.

**Listing.** F1's Cycle document says "F2 owns folder creation and its
listing", while the Wave's F2 scope names only creation, and F1's later
ruling is that a read never lists a directory. A created empty folder
would therefore be invisible to every read. So the indexer records
folders, as decision 4 recommended:

- The walk that collects documents also records every directory under
  `docs/` that it listed, with whether the listing gave any entry.
  Refresh and rebuild write them to the table `discovered_folders` in the
  transaction that stores the context:

  ```text
  discovered_folders
      id
      context_id   the context, deleted with it
      path         unique within the context
      is_empty     0 or 1
  ```

  Adding the table marks every registration stale once, until refreshed,
  as F1's migrations did. A folder change between a refresh's two
  observations is a change like any other, and the refresh is tried
  again.
- The folders are those of the primary worktree. An item worktree's
  `docs/` is not listed.
- `empty` means the listing gave no entry at all. A folder that holds
  only a folder, a file that is no document, or a link is not empty.
- A new read, `list_document_folders`, returns
  `{ items: [{ path, empty }], complete, index }` from the index alone.
  The library fixes no command name for it; the golden uses
  `folder list`.
- `complete` is false when a directory under `docs/` could not be
  opened, lies below the sixteenth level, was not reached before the
  entry limit, or has a name the list cannot carry: one that is not UTF-8
  or holds a backslash. Such a directory is left out, still read for
  documents, and recorded as a problem at the nearest directory above it
  whose path can be stored, passing over one whose own name ends in
  `.md`. The document list of such a repository is not `complete`
  either. A `.git` directory is never listed and does not make the list
  incomplete.

### Repair and adoption

`repair_item` replaces a nonconforming file with a corrected complete source,
in an editing context, through the ordinary checkpoint.

Inputs of `document repair` and `ticket repair`: the root, the exact
path, its observation token, and the corrected source. The path must be
written as a plain relative path and must be the canonical place of the
command's kind. The file is found by the names its directories list, so a
path that names its file only by another case of a name finds nothing.
The corrected source must be a conforming document or ticket for that
path. Its `id` is the item's identity from then on. The file must be
committed on primary. The source is written byte for byte and is not
serialized again; a later save serializes it as it does any file.

- **Adoption.** If the current file has no valid ID, the corrected source's
  ID is being assigned. The caller obtains it from `id new` and puts it in
  the source. The preview states the adoption and the ID. Because the ID is
  part of the confirmed input, the confirmation, the retry and any partial
  replay all use the same one; nothing in the library generates or
  regenerates it. That is what "stable ID allocation" means here. The
  editing context is named by the ID, so the same source gives the same
  context on every call.
- **Existing identity.** If the current file has a valid ID, the corrected
  source must keep it. If not: `not_repairable`.
- **Ambiguous identity.** If the corrected ID is held by something else,
  or the index holds a duplicate-ID problem for the path, the result is
  `identity_ambiguous`. Nothing is written and no context is created. The
  ID is held by something else when an item of the index that is not this
  path's has it, or when another file holds it on primary's worktree, in
  the commit the context is cut from or stands at, or in the context
  itself. A file holds an ID as the item it parses as or, when it is no
  item, as the valid `id` of its front matter. The index alone is not
  asked, because it lists two files of one ID as a problem and neither as
  an item.
- **Tickets.** A ticket's path contains its ID. Repair applies only when the
  corrected ID equals the directory name. A ticket directory that is not a
  ULID is `not_repairable` and stays visible as a problem.
- **Closure.** If the current front matter can be read and has `closed_at` or
  `closed_by`, the corrected source must carry the same values; if it has
  neither, the corrected source must have neither. Repair cannot close or
  reopen a ticket. If the current front matter cannot be parsed at all, a
  corrected source with closure fields is `invalid_input`. The values are
  compared as the YAML writes them, with null as absent.
- **Not repairable here,** all `not_repairable`: a path that is no
  canonical place of the command's kind, such as `notes/x.md` or a ticket
  path given to `document repair`; a file that is a conforming item
  already; a file whose `manyhands_kind` names another valid kind than
  its path's, while one that names no kind at all is a broken field and
  is repaired; a document's path at which an item already exists under
  another ID, which is an earlier repair, whether the index lists it or
  only a registered item context holds it as its own item; and a context
  that already holds another conforming item at the path. These stay as
  problems. That a second repair of one path under another ID is refused
  was approved by the product owner on 2026-10-13: two items for one file
  is the confusion a repair exists to remove. A user who wants to adopt
  again under a new ID first removes the earlier context.

**The checks and their order.** `prepare` on the service validates the
input alone, resolves the root, reads primary's file, makes the checks,
and then asks the item's context. The first that fails answers:

| Refused at `prepare` | Code |
| --- | --- |
| A path not written as a plain relative one | `invalid_path` |
| A path that is no canonical place of the command's kind; a ticket directory that is no ULID | `not_repairable` |
| A source that does not parse as an item of that kind at the path | `invalid_input` |
| A `ticket repair` source that is a conforming ticket of another ID than the directory's | `not_repairable` |
| Nothing at the path, or a path that names its file only by another case of a name | `path_not_found` |
| A file reached through a symbolic link | `invalid_path` |
| A token that is not the file's | `external_change` |
| The index holds a duplicate-ID problem for the path | `identity_ambiguous` |
| The file is a conforming item already; it names another valid kind than its path's; it holds a valid ID the source does not keep | `not_repairable` |
| A ticket source whose closure fields are not the file's, or are present where the file has none or its front matter cannot be read | `invalid_input` |
| The corrected ID is held by something else | `identity_ambiguous` |
| An item is at the path under another ID | `not_repairable` |
| The file is not the blob at primary's head | `worktree_not_clean` |
| The item's context holds the source committed at its tip | `already_applied`, a no-op, with no confirmation |
| The item's context holds another conforming item at the path | `not_repairable` |
| The item's context holds anything else at the path but primary's file or the source | `external_change`, offering nothing |

The last row offers nothing because no new preview can succeed while the
context holds those bytes; see the known limits.

**The preview.** Its effects, in order, are `context_created` when the
item has no context, `id_adopted` on an adoption, `source_written` and
`checkpoint_committed`. Its details are `root`, `path`, `id`, `adoption`
and `primary_changed`, which is always false: primary is never written.
The observation digest covers the path, the file's token, whether the
file is committed on primary, whether the corrected ID is held by
something else, and whether an item is at the path under another ID. A
first call's `prepare` makes none of the checks the digest covers: it
observes, and the boundary's compare answers a stale preview
`external_change`. A confirmation presented once the item's context has
moved on is answered by the context: a context that holds the source
committed is a no-op with no record, and the confirmation is not used.

**The domain function** needs a third context intent, `Repair`. `Create`
requires the destination to be absent and `Edit` requires the ID to exist
on primary; a nonconforming file is neither. Under the repository lease
and one row of the local journal, `repair_item` asks the rules of
primary's file, which must digest to the expectation; asks who holds the
ID, before any context is made; makes or reuses the editing context of
the ID the source carries; writes the source there; checkpoints it with
the ordinary subject for its kind; and hands off to indexing as a save
does. Its checks are made for as long as no write step is recorded on its
row, so a repair left with nothing done is asked the rules again when it
is entered again, and a refusal then closes its row. Its journal action
is `repair_item`, which joins the published operation-action list. The
steps are the existing ones of an authoring write; none was added. Three
error kinds carry its refusals: `NotRepairable`, `AmbiguousIdentity` and
`InvalidItemSource`.

**The binding** is one for both commands. Its domain call, effects and
commit evidence are those of an authoring save. Its result data is the
item's ID, the path, and `adopted`. A pending operation on the repository
is not looked for at `prepare`; the execution answers `recovery_required`
as for every other binding.

**Re-entry.** A repair is a confirmed command, entered again through
`confirm::reenter`, and answers `SaysNothing` for a row that ended
without its checkpoint.

| The row before the call | What happens |
| --- | --- |
| Absent | The evidence is read; the state is observed again and compared; then the domain call. |
| Pending, with no step or a step of making the context (`branch_observed`, `worktree_observed`) | The same. Making a context changes nothing a preview observes, so the compare is still made. This is longer than for the other confirmed commands, which stop comparing at the first recorded step. |
| Pending, at `authoring_destination_observed` or later | No compare. The domain call completes the repair whatever primary holds by then. |
| Completed and checkpointed, with the request's own commit in range | Finished with that commit and no domain call. |
| Completed and checkpointed, with no own commit | The domain call, which finds nothing to do, as a save's does. |
| Completed and not checkpointed | A refusal that was lost. The record is released: `busy` when nothing has changed, `external_change` or the standing refusal otherwise. |
| Any commit from elsewhere to the path in range | `external_change`, no domain call. |
| Pending under a live index-owner heartbeat | The row is left as it is: `discovery_pending`, the record kept. See Liveness. |

**Resume and abandon.** A resume of a repair that wrote and did not
commit is `original_request_required`, naming the repair's request; a
resume of one that owes only its hand-off brings the index up to date. An
abandon's preview and result give the action `repair_item`, the item's
ID, its worktree and the one path that differs. The written file is left,
and a commit the repair made is kept. The freed request ID, sent again
with a new preview, commits the file under the same item ID.

**After the cache is lost** a new preview is given of a repair whose
context holds the corrected source uncommitted, and the new request
commits it. The domain accepts, for a new operation, a context that
already holds the source it is asked to write.

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

The ticket bindings call it in `prepare`, before the request is accepted
and before any domain call, so a rejected create or save leaves no
context, branch or record. A create calls it once the repository is
resolved; a save calls it after reading the item and after the
closed-ticket guard. Neither calls it when the request names no target: it
sets no dependency entry and no parent. D3 calls the same read to explain a
cycle before the user saves.

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
  F2 does not transliterate. With no per-call or configured initials and
  no identity name to derive them from, the result is `identity_required`.
- **Configuration that cannot be read.** A repository whose primary head
  has no configuration file is `repository_not_registered`; a file that
  cannot be parsed is `invalid_configuration`.
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
attempt that wrote the slug and did not commit is finished by its retry. A
retry after a lost result is answered under
[Re-entering a request](#re-entering-a-request): the assigning commit is
reported when it is the request's own commit in range and the journal row
read before the call shows that an earlier attempt may have committed. The
rules do not search for a commit by what it assigned. A retry whose record
is gone, or was released because the earlier attempt ended before its
checkpoint, is a new request and is checked as one.

As built, the binding's intent is that every field of the ticket equals
the file the request was accepted against and a slug is present; the
slug's value is ignored, because it is written once. A new request is
checked in this order: a closed ticket is `ticket_closed`; a ticket that
holds exactly the slug this request would compose now, in a committed
file, is `already_applied`, which is the truthful answer to a retry of an
assignment that succeeded and whose record is gone; a ticket with any
other slug is `slug_already_assigned`, an input error whose data carries
the slug the ticket holds; and a stale token on a ticket with no slug is
`external_change`.

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

Amended 2026-10-30 by Wave 03 C4 (its decision 9): the rule "A caller cannot
supply it" is replaced. `comment add` takes an optional caller `created_by`,
validated as `Name <email>`, at most 256 bytes, and the checkpoint writes it
verbatim in place of the resolved identity's text. Without one, the rule
above applies. Either way the checkpoint still needs a commit identity, and
the value is written once and never rewritten. The public `submit_comment`
is unchanged; the caller's value enters through a crate-private parameter of
the checkpoint. See [Departures from the RFCs](#departures-from-the-rfcs).

On a retry, the comment file on disk keeps its `created_at` and `created_by`.
If the identity changed between attempts, the file names the first and the
commit is authored by the second; the canonical RFC says the field is "never
rewritten". The read side takes the author from the field.

## Published Contract

F2 publishes schemas and golden fixtures for the shapes the library
produces. Per-command input schemas belong to the CLI Cycles, which define
the input files.

- Extended: `envelope` (result codes), `recovery_action` (the new actions),
  `identity` (`initials` and `initials_source`, both required),
  `operation` (the `repair_item` action).
- Renamed in an F1 shape: `host_pin`, whose `algorithm` and `fingerprint`
  are now `pinned_algorithm` and `pinned_fingerprint`, with the goldens
  of `host inspect` and the pin list. The message of
  `not_repository_root` changed a read golden too. These are the two
  changes this Cycle makes to what F1 published.
- New: `preview`, which is the repository shape of `repo create` and
  `repo enable`, and a schema for each other shape of a preview's
  details: `remote_removal_preview`, `repository_removal_preview`,
  `key_deletion_preview`, `operation_abandon_preview`,
  `identity_preview`, `host_preview` and `repair_preview`;
  `preview_effect`, extended for the host commands and again for repair;
  `request` and `request_operation`; `progress_event`; `cancel`;
  `path_observation`; `relationship_check`; `host_observation`, the data
  of `host observe`; `host_replacement`, the data of the one `prepare`
  refusal that carries any; `document_folder` and `document_folder_list`; and
  one mutation-data schema per command family: `document_mutation`,
  `ticket_mutation`, `remote_mutation`, `key_mutation`,
  `operation_mutation`, `synchronization_mutation`, `identity_mutation`,
  `host_mutation` and `repair_mutation`.
- The enumerations `repair_item` joins are in `operation`,
  `operation_mutation` and `operation_abandon_preview`. Every `enum` of a
  schema is tied to a closed list in Rust by a test. Four published
  strings are closed sets in Rust and free strings in the schemas:
  `command`, and the `state`, `step` and `algorithm` of the shapes that
  carry them.
- Goldens live in `tests/fixtures/mutation_v1/`, produced by the library from
  real repositories and compared byte for byte, with the same placeholder
  and sentinel rules as `read_v1`. Golden progress events live in
  `tests/fixtures/progress_v1/`. The fixture directories are exactly the
  registered cases, in both directions.
- Completeness is held by tests in `mutation_contract`: every bound
  command has a golden; every schema is reachable from a published root
  and no reference dangles; every data root describes the data of some
  golden and every golden's data has one root; and every result code has
  a golden, and a mutation golden, or stands in an explicit list with its
  reason. `CODES_NO_FIXTURE_CAN_PRODUCE` has one entry,
  `remote_unavailable`. `CODES_NO_MUTATION_GOLDEN_HAS` has four:
  `remote_unavailable`, which no fixture can produce, and `invalid_id`,
  `public_key_unavailable` and `authority_not_found`, each a read's code
  that no command of `execute` or `prepare` returns.
  `repository_inaccessible` has a golden: a repository whose Git
  configuration cannot be parsed is `repository_inaccessible`, `blocked`
  with no action, on a rebuild and on a save. A golden may also be of one of
  three commands `execute` does not bind: `request show`,
  `request cancel` and `host observe`.
- A golden of a command can be of a refusal. `item sync`, `repo sync`,
  `host approve` and `host replace` have no golden of a request that did
  its work: each needs a server, and its envelope is checked against its
  schema in the live SSH cases instead. A golden that holds the test
  server's port holds 0.

All additions are within version 1 under the compatibility rule: new fields,
new enumeration values, new codes. No outcome or effect value changes.

`repository_not_enabled` and the action `request.cancel` are added the same
way, with goldens for an index rebuild and a remote add on a repository
that was never enabled.

Three F1 tests assume reads and are adjusted: the one asserting that the
actions reads suggest are the whole registry, the one asserting that a
code's outcome follows its class, and the helper asserting that data is
present exactly on success.

### Departures from the RFCs

Each of these is decided, here or in the Cycle document, and the RFC text
is unchanged. A later Cycle that reads the RFC alone would expect the
other answer.

| RFC sentence | As built | Decided |
| --- | --- | --- |
| CLI: `host approve`, `host replace`: "Input exact authority and presented fingerprint". | The input carries the key's algorithm as well as its fingerprint, and so do the preview, the result, the observation and a synchronization's data. `old_fingerprint` has no algorithm; the comparison with the pin does not need one. A CLI that takes a fingerprint alone can call `host observe` first. | Task 16; [Host approval](#host-approval). |
| Authentication: an approval is persisted after "the presented key is observed again". | The selected key must also authenticate. | Decision 6. |
| CLI, index and Git workflow: after cache loss, when evidence cannot prove a replay safe, "return `recovery_required`". | `repository_not_registered` first. Then `external_change` for intended content that is written and not committed, `confirmation_not_found` for a confirmed command, and `already_applied` for content that is committed. Each says more exactly what the caller does next. | The Cycle document's cache-loss row; [After the cache is lost](#after-the-cache-is-lost). |
| CLI: the free-form objects are "the envelope's `data`, `unknown_metadata` and a recovery action's `arguments`". | A fourth: `result.data` of a request, in `request show`. | Task 6. |
| CLI: `operation show` "finds any stored operation, finished or not". | The operation ID a key import, selection or clearing names is `operation_not_found`. | Product owner, 2026-10-12. |
| CLI: `operation_id` is "null when no operation began". | It is set on every result given after acceptance of a request that has an operation, a refusal included. | With the row above. |
| CLI: `remote remove`: "show publication/polling effects". | Removing the publication remote is refused, `publication_remote_in_use`, so `publication_affected` is always false in a preview that is given. | [Observations](#observations). |
| CLI: "A linked worktree maps to its repository's common Git identity". | Every mutation refuses a linked worktree root, `not_repository_root`. | [What `execute` does](#what-execute-does). |
| Relationships: the input and configuration key `ticket_slug_prefix`. | The preview's field is `slug_prefix`. | Not decided anywhere; recorded here. |
| CLI: `folder create` "in the selected item's context when organizing an existing item". | Primary only. | [Folder creation](#folder-creation). |

One departure is from the Cycle document and not from an RFC. Its
contract item 10 says a partial result comes with a recovery action. Two
partial results carry none: a `request_mismatch` against a request that
made an effect, and a key generation retained for inspection; see
[Result codes](#result-codes).

Amended 2026-10-30 by Wave 03 C4: one rule of this design is replaced by a
later Cycle. The [Comment Author](#comment-author) rule "A caller cannot
supply it" gives way to C4's decision 9: `comment add` takes an optional,
validated caller `created_by`, written once, and the Git identity
otherwise. Unlike the rows above, the RFC text changed with it: the
canonical content and comment schema RFC is amended to match.

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
6. `synchronize_remote` gains a sibling, `synchronize_remote_observed`,
   that tells a `SynchronizationObserver` of each safe point and takes
   `SynchronizationSettings`, whose one setting refuses a divergent
   remote. The existing function delegates to it with the default
   settings and an observer that always continues; its request type and
   its tests are unchanged.
   - At a safe point the observer answers `Continue`, `Cancel` or `Stop`.
     `Cancel` records a cancellation on the operation as
     `cancel_remote_operation` does, but only while this executor still
     owns the operation, so one that was fenced cannot stop its
     successor; the safe point then honors it. `Stop` ends the call with
     `RecoveryRequired` and asks nothing of the operation.
   - The observer is also told, before the safe point that follows
     verification, that the remote holds the commit and whether a push of
     this operation put it there.
   - With the setting on, the sibling returns `MergeRequired` at the
     point where the existing function decides a merge is needed, before
     any window or merge is prepared, and likewise where a retry after a
     lost push would open a publication attempt. That stop is after the
     reservation, so it is one of open case 3's. A request whose operation
     already has an integration window, which only a merging call
     creates, is `RecoveryRequired` when the setting is on.
   - One change applies to the existing function too: the checkpoint that
     says the push returned is written the moment the transport call
     returns, before a requested cancellation or yield is honored. An
     operation ended there has a row that says the remote took its push.
   - The remote journal gains three reads: what a synchronization's row
     recorded of its push, whether a row was begun under a configuration
     generation the registration has since left, and which target a row
     synchronizes.
   - No reservation is given, and no synchronization replayed or reported
     complete, under an operation ID whose local row was abandoned.
7. Refresh and rebuild record folders. The walk that collects documents
   returns the directories it listed, a refresh compares folders between
   its two observations, and the new table `discovered_folders` marks
   repositories stale once.
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
9. Public enums gain variants: `ContextIntent::Repair`; the
   `RepositoryOperation` variants `RepairItem`, `SetLocalIdentity`,
   `CreateFolder` and `AbandonOperation`; the `RepositoryErrorKind`
   variants `NotRepairable`, `AmbiguousIdentity`, `InvalidItemSource` and
   `OperationAbandoned`; and the operation action `repair_item`, which is
   the thirteenth local action. Exhaustive matches on them inside the
   crate are extended.
10. `IdentityDto` and its schema and golden gain the initials fields.
11. The index gains the three request tables, the folder table of item 7,
    and `key_registry_changes` in the key registry.
12. Changes the design did not list, made while Tasks 8 to 14 and 12A
    were built.
    - The local journal (`src/repository/recovery.rs`) knows the
      abandoned marker. `begin_or_reconcile`, the advance of a row and
      the discard of a row refuse an abandoned row with the new
      `RepositoryErrorKind::OperationAbandoned`, and the lookups read
      such a row as absent. `RepositoryOperation::AbandonOperation` is
      new.
    - A rebuild beats a heartbeat on its row's `observed_at` once a
      second while it works, as the index owner already did, and sets it
      stale when it ends with its row pending.
    - `restore_index_for_rebuild` in `src/repository.rs` brings an
      unavailable index back through the existing replacement of a
      corrupt cache, for `index rebuild` alone.
    - Enablement and creation have variants that take the short-code
      prefix (item 5).
    - `coordination.rs` gains a look at the repository and bootstrap
      leases that does not wait and creates no lock file.
    - Each journal gained the small reads the bindings need: the step a
      completed local row recorded, the key a completed generation
      registered, and the polling policy of a registration.
    - Test-only failure points: `AfterInitializationCommit`,
      `AfterRemoteDeletion`, `AfterConfirmationAcceptance`,
      `BeforeRequestCancellationRead` and
      `BeforeSynchronizationCancellation`; and an acceptance hook,
      `set_acceptance_hook_for_testing`, which runs once immediately
      before the next request an `execute` accepts is written.
13. Plumbing that the design did not list, added while Tasks 6 and 7 were
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
14. Changes made while Tasks 14C to 19 and the fix pass were built.
    - **An operation leaves a row that is being indexed alone.**
      `save_document`, `save_ticket`, `add_remote`, `remove_remote`,
      `set_publication_remote`, `enable` (and so a `create_and_enable`
      that is taken up again, which goes through it) and
      `submit_comment` return straight after they begin, with no step
      recorded, when their row is pending under a live index-owner
      heartbeat: each its index-pending outcome, and `submit_comment`,
      which has none since Cycle 07, `RecoveryRequired`. One check,
      `RepositoryService::is_being_indexed`, is shared with
      `repair_item`. This changes what an existing function does: a
      second live call no longer takes the row from the first, and a
      retry within about three seconds of a crash while indexing is
      answered index-pending.
    - The key registry (`src/repository/keys/registry.rs`) gains the
      `_under` forms of its three changing functions and the table they
      write. The public functions keep their signatures. The digest of a
      setting, `keys::observed_setting`, is the registry's.
    - `recovery::in_the_way` is the one question of what refuses a new
      local operation: a remote reservation and an incompatible pending
      row. `begin_or_reconcile` asks it where it read the reservation
      before, so the order of its refusals is unchanged, and
      `recovery::require_refresh_can_begin` asks it for a folder
      creation.
    - Discovery leaves a directory the folder list cannot carry out of
      the index and records a problem for it, so the document list of a
      repository that has one is not `complete`, where it was.
    - The message of `repository_not_registered` is "The repository is
      not registered with Manyhands." The message of
      `not_repository_root` is "That path is inside a repository but is
      not a root this command takes.", which is true when a mutation
      gives it for a linked worktree root; a read's golden holds it too.
    - With the merge of Wave 02 Cycle 07: the three comment steps are
      variants of `LocalStep`; `recovery::waits_for_registration`,
      `comment_child_may_begin` and `is_comment_synchronization` are
      new; reading the binding of an abandoned submission, and reserving
      its synchronization, are refused with `OperationAbandoned`; the
      operation read lists a comment that waits for its registration and
      gives a comment's synchronization no next action. The migration of
      `comment_publication_bindings` arrives with the merge. No journal
      action, error kind, result code, recovery action, schema or golden
      was added.
    - F1's `HostPinDto` and `host_pin.schema.json` name the stored pin
      `pinned_algorithm` and `pinned_fingerprint`, with the two read
      goldens that hold them.
    - `canonical::front_matter` is new, within the crate. Visibility
      only: `read::credentials::stored_authority`, and the two fields of
      `HostTrustSnapshot`, are readable within the repository module.
    - Test-only failure points: `BeforeRequestRelationshipCheck`,
      `BeforeRequestSettlementWrite`, `BeforeKeyRegistryChange`,
      `AfterLocalIdentityName`, `AfterFolderCreation` and
      `BeforeFolderRefreshClose`; a hook that runs while the identity
      write holds Git's lock, `set_local_identity_lock_hook_for_testing`;
      and a transport checkpoint, `HostTrustFinalized`, immediately after
      the pin is stored. The SSH test server counts every socket it
      accepts.

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
- **Infer an earlier attempt's commit from the journal step alone.**
  Rejected: the step is written whether or not a commit was made, and a
  replay rewrites it. The commits after the position recorded at
  acceptance are the evidence. The built rule uses the checkpoint step as
  a necessary condition and never a sufficient one: without it, read
  before the call, no commit that was already in range is the request's;
  with it, the commit must still be a candidate in Git.
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
| The Cycle is large: the boundary, thirty bindings, four new domain operations and the relationship writers. | Three parts with a review checkpoint after each. Decision 1 offers the split. |
| A binding maps a partial outcome to a clean success. | Exhaustive matches; one rule for outcome and class; a test per binding family that injects a failure after the authoritative step. |
| A retry reports a commit that is not this request's. | The commit must be a candidate: in the range since the position recorded at acceptance, a change of the request's path to the intended content, made from the state the request expected when that state was committed at acceptance, for a request that had something to commit at acceptance. It must also either have appeared during this call, or lie in a range the recorded commit bounds while the journal row read before the call shows an earlier attempt may have committed (pending, or final and checkpointed). The oldest such commit is reported; otherwise none is. A retry whose earlier attempt ended before its checkpoint is released and run as new, where the binding says such an attempt committed nothing. A failure to read Git reports nothing and leaves the record `accepted`. An identical commit made outside the library while the request's row is pending can still be reported as the request's; the content on disk is then what the request intended. |
| Settlement deletes or finishes a record another call is using. | Every record change is conditional on the record still being `accepted` with the call's own attempt number, which each call that enters raises. A retry does delete, when it releases a record whose earlier attempt did nothing or stops with nothing in flight, and only under that condition. |
| A body's digest lets someone with the database test guesses of a short body. | Required by the CLI RFC. The intent digest is salted per record, so equal bodies do not collide there. The salt covers the intent digest only: `expected_digest` is the plain digest of a file's bytes, so the record of one save holds the unsalted digest of exactly the file the save before it wrote, and the observation digest of a confirmation is not salted either. The database already sits beside the repository it describes. |
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
All six exist and are in the native workflow's test list.
`journal_rejection` is in that list too and runs on the five native
targets, where it has passed. Every test of `mutation_bridges` begins by
sending Git's system, global, XDG and program-data configuration levels to
a directory of the test process's own, and a test that writes
configuration outside its repository runs in a child process. The native
workflow takes long to give feedback; shortening it is ticket
`01M4R1K5VRSRXFM58VQCTQ9F3W`.

The scenarios the Wave's exit evidence names, and where each is proven:

| Evidence | Fixture |
| --- | --- |
| Stale preview rejection | `repo enable` and `document repair`, with the repository changed between preview and confirmation. Every confirmed command has a stale-preview row of its own in the conformance table. |
| Changed-input rejection | A ticket save and a synchronization, each retried with one field changed. |
| Accepted-consent retry | `repo enable` and `repo create`, each interrupted after the commit; the retry after a simulated hour completes without a new preview. |
| Lost output reconciliation | A save whose result is discarded, with and without another save in between; a synchronization whose push is accepted and whose result is lost. Each retry returns the same effects and makes no second commit or push. |
| Cache-loss recovery | The database is deleted between attempt and retry, for a create, a save, a save that died before its checkpoint, a synchronization, a confirmed command, a key import, a repair, `repo identity-set`, `host approve` and `host replace`, and `folder create`; each after the repository is registered again. The bridges' cases are in `tests/mutation_bridges/cache_loss.rs`, beside the other tests that keep to a Git home of their own; the host case is local and stops where a connection would begin. |
| No duplicate effects | Commit counts, ref values and file contents compared before and after every retry. Four fault points on a save's path are driven through the boundary; the commit count is taken on the item's branch. |
| Cancellation at safe points | Before acceptance; at a held synchronization safe point through the token and through `cancel_request` from a second thread; after the outcome is classified, through `cancel_request` and through the token: a token cancelled once the push has returned does not unsay the publication. |
| Each bridge | Expected observation, retry, changed input and an injected failure for identity, host approval, folder creation and repair. For identity the injected failure leaves the configuration untouched; nothing is restored. |
| A second live call | A row another call is indexing under is left as it is, for each operation the liveness rule covers. |
| Host approval publishes nothing | Server-side push count and every local ref compared before and after. |
| Ambiguous identity stays non-editable | Repair with an ID another item holds: no write, no context, no branch. |
| Short-code vectors and invariance | Golden vectors; a slug compared before and after a rename, an identity change and a prefix change. |
| No body or secret in records | A sentinel body, passphrase and remote credential, scanned for in every column of the request tables, the journals and every envelope and event. |

Time is injected for expiry tests through a clock the service holds; the
production clock is the system clock.
