# Concurrent writers in one clone: attribute, serialize, name the blocker, and repair what cannot be cleared

Ticket `01M7NMJKJG35F3C51JF7EVDSRP` (vh-bhepg). Status: draft, awaiting the owner's approval.

## Intent

Two agents (or an agent and a person) working in the same clone on the same machine can leave the repository answering `recovery_required` or `busy`, and nothing says whose operation is in the way, whether it is still running, or how to clear it. The owner and the agent both lacked the facts to decide what to correct. This change makes every unfinished operation say whose it is, makes a second writer wait for a live peer instead of calling it a failure, names the blocker in every refusal, and removes two states that nothing could clear.

## Decisions (owner, 2026-11-18)

- Everything below stays in this one ticket, including the per-item scope of the hold, which is built last.
- An interrupted first push to a context branch the remote has never had is retried automatically.

## Goals

1. Attribute operations: `operation list` and `operation show` (and the JSON) carry the request id, author, host, process id, start time and command, and for authoring operations the `item_id` (and `comment_id` for a comment). Rows from older builds show those fields as null, meaning unknown.
2. A second writer waits a bounded time for a live peer, then answers `busy` with the holder's attribution and a retry hint. It leaves no operation behind and never answers `recovery_required` or `partial` only because another writer is working.
3. An unfinished operation on one item does not block writes on unrelated items. Whole-repository operations that truly need exclusivity keep it.
4. A `busy` or `recovery_required` answer names the blocking operation and its attribution in `data` and `recovery`. `repo inspect` lists unfinished operations.
5. No unfinished operation exists that neither `operation resume` nor a confirmed `operation abandon` can clear.
6. The tests sit at the cheapest layer, with at most one SSH case.
7. The `manyhands-recovery`, `manyhands-overview`, `manyhands-comments` and `manyhands-sharing-and-sync` skills are updated to use the new fields.

## Non-goals and anti-goals

- No cross-clone coordination and no remote lease. Clones already meet through Git.
- No automatic redrive of stale operations, no `request list` verb, and no change to replay of a reused `comment_id` (tracked separately).
- No weakening of the safety check for a push that was confirmed returned, or for a push attempt that left publication evidence: those stay ambiguous.
- A resume must never take over an operation a live process owns.

## Facts checked

Reproduction (scratch repositories, isolated HOME, two CLI processes on one machine; recorded in the ticket's first comment):

- Two concurrent `comment add` calls on different tickets: about half the pairs have one writer answer `blocked` / `recovery_required` with `data` null and nothing named. Almost all of those clear by themselves. With an SSH remote about 260 pairs left 4 unfinished operations.
- Killing the CLI after `push_prepared` was recorded (5 of 5 runs) leaves a sync that `operation resume` refuses with `recovery_required`, ending in `reconciling` / `after_fetch` with `failure_code` and `next_action` empty. This is the incident's exact state and needs no second writer.

Code (this branch, 782a0d85):

- The write lease is an OS file lock on `<git common dir>/manyhands-operation.lock` (`src/repository/coordination.rs`, `repository_lease`), a 250 ms wait, no holder identity, released by the kernel when the process dies. It covers only the local write and checkpoint. `handoff_post_authoritative` (`src/repository.rs:3878`) drops it before the index refresh, while the operation row is still unfinished. Publication and sync take no repository lease.
- `in_the_way` (`src/repository/recovery.rs:1471`) refuses a new operation while any other row of the same root is not `completed`. The check ignores the item, and a live peer looks the same as a stale row, so the second writer gets `recovery_required` at once without waiting.
- Local rows live in `operation_records` (`recovery.rs:597`) with no request id, host, pid or start time; `item_id` is written only by `context_sync_local`, and for `save_ticket` and `submit_comment` the item is only inside `target`. `request_operations` links a request to its own operation only. Remote rows are in `remote_operation_records` and carry `owner_epoch` but no attribution.
- Answers carry no blocker: `outcome.rs` maps `RepositoryBusy` to `busy` and `RecoveryRequired` to `recovery_required` with canned messages, and `offering_abandon` names pending operations only for stale rows. `repo inspect` (`read/admin.rs`, `RepositoryInspectionDto`) lists no operations.
- First-push refusal: on resume `synchronization_push_absence_boundary` (`remote/sync.rs:6877`, `push_absence` at 7079) counts the operation's own unconfirmed `push_prepared` intent as an unanswered push, so with the context ref absent on the remote the boundary is `Ambiguous`, which is `RecoveryRequired` (sync.rs:6242-6263, 6651). Confirmed from the stuck row (`push_oid` set, `push_advertised_oid` empty) and from the remote repository (only `main` exists).
- `external_change` leftover: a restart skips the `FetchPrepared` checkpoint (the only writer of `expected_oid`, sync.rs ~6168 `if prior.is_none()`), so the row reaches `fetch_observed` with no `expected_oid`, and `evidence.expected_oid != Some(local)` (sync.rs ~6552) answers `external_change` forever. A relaxation of that check alone would not do: `checkpoint_synchronization(PushPrepared)` also refuses a row with no `expected_oid` (reservation.rs ~2679), and `republish_reserved` already backfills it (sync.rs ~7626).
- Not confirmed: how the two-writer race (nothing killed) reaches a restarted sync with no `expected_oid`.
- Registry migrations are named, additive entries in `registry_migrations` (see `cycle_07_comment_publication_bindings`); they are one-way, and an older build may not run on a registry a newer build migrated.

## Approaches

1. **Attribute and name first, wait on live peers, repair the two states, scope the hold last (chosen).** Each step is useful alone and the riskiest step, per-item scope, comes last with its own review.
2. **Hold the write lease across the whole command.** Rejected: the command includes network publication, so one slow remote would block every other writer for seconds, and sync is built to take no repository lease.
3. **Only wait and retry, with no attribution.** Rejected: it hides the second writer but leaves the owner and the agent unable to tell a live peer from a stuck one, which was the incident.

## Design

### Attribution

- Add nullable columns to `operation_records` and `remote_operation_records`: `request_ulid`, `command`, `author`, `host`, `pid`, `started_at`. Add `comment_id` to `operation_records`, and write `item_id` for `save_ticket` and `submit_comment` rows. A named additive migration (`cycle_<n>_operation_attribution`) adds them; existing rows stay null.
- Populate at row creation from the request in flight: the request id and command already known to `mutation/execute.rs`, the author from the configured commit identity, host and pid from the process, `started_at` from the injectable clock.
- `OperationDto` (`read/dto.rs:833`) and the schemas (`operation.schema.json`, `operation_list.schema.json`, `operation_mutation.schema.json`) gain these fields, all nullable. `request show` lists every operation the request started, including the later `synchronize_context` (the ticket's first comment asks for this).
- Liveness is derived, not stored: a row is live when its process exists on this host (and, for indexing, its heartbeat is fresh as today). Host or pid unknown means not provably live, and is treated as stale. Process-existence checks use `libc` on Unix and `windows-sys` `OpenProcess` on Windows, both already dependencies.

### Wait, then busy

- When `in_the_way` finds another row, it decides on liveness. A live row makes the caller wait up to a bounded time, polling, and then answer `busy` with the blocker in `data` and `request.retry`. A stale row answers `recovery_required` with the blocker and the existing resume and abandon actions. Neither path creates a row, and a `busy` answer leaves the request retryable with the same request id.
- The wait budget and poll interval are injected (a config value for tests), not read from the wall clock in tests. The 250 ms lease timeout stays.

### Name the blocker

- `data.blocker` on `busy` and `recovery_required` carries `operation_id`, `family`, `action`, `item_id`, `request_id`, `author`, `host`, `pid`, `started_at` and `live`. `recovery` gains `operation.show` for it. `repo inspect` returns an `operations` list with the same shape (`repository_inspection.schema.json` is extended). The canned envelope message may mention the holder in one clause.

### Repairs

- **Unreturned first push.** In `push_absence`, an intent whose checkpoint is `PushPrepared` (not `PushReturned`), with no publication attempt or authority, for a context whose publication evidence is `NeverPublished`, is no longer an unanswered push on resume: the push is retried. A confirmed-returned intent, an intent with evidence, and any foreign row stay ambiguous. The existing test `an_unreturned_first_push_with_no_publication_evidence_is_retryable_not_ambiguous` (branch commit aec7f69c) pins this and must pass.
- **Missing `expected_oid` after a restart.** After the reconcile on the main sync path, backfill the evidence's `expected_oid` from the local head (`get_or_insert(local)`, the same as `republish_reserved`), so the row can proceed to the push. Whether to also write the skipped `FetchPrepared` checkpoint on restart is the builder's call; the backfill is required.
- **A way out.** `operation abandon` accepts a remote operation when the person confirms (`--prepare` then `--confirm`, a hand-over per the autonomy table, R16). The confirmed abandon marks the operation's push attempt rejected (`outcome = protocol_rejected`, as `acknowledge_refused_push` does) so it stops counting as unanswered, and says so in the preview. The preview names the evidence (remote ref present or absent, local and remote oids), so the person decides with the facts.

### Per-item scope

- The second check in `in_the_way` becomes per item for item-scoped operations: an unfinished operation on one item blocks that item and whole-repository operations (refresh with an empty target, repository-level sync, anything that rewrites shared state), not unrelated items. The first task of this step is to read why the hold was per-repository (shared index refresh, the discovery index, the checked-out primary branch) and list the reasons in the ledger; any operation with a real shared-state reason keeps repository scope, and the step is dropped from this ticket if a reason turns out to need a redesign.

## Tests

- Library, injected clock: liveness decision, wait-then-`busy`, stale-then-`recovery_required` with blocker, and the per-item table (item vs unrelated item vs whole-repository), in `recovery.rs` / `mutation` tests using `insert_local` seeding.
- Two writers, no SSH: extend `tests/mutation_replay/reenter.rs:982` with a different-request-id variant that parks writer A inside its operation and asserts B answers `busy` with A's attribution and leaves nothing behind; a second case for per-item independence (writers on two tickets both complete).
- Migration: a legacy registry without the new columns migrates, old rows list with nulls.
- Repairs: the existing `push_absence` table test; a reservation-layer test that a restarted row at `fetch_observed` gets `expected_oid`; a confirmed abandon of a remote `push_prepared` row.
- CLI/JSON contract: one golden for `operation list`/`show` and one for the blocker in `busy`, in an existing target.
- SSH, exactly one case, in the existing interruption target: an interrupted first push resumes to `published`. It needs SSH because the behaviour depends on a real push to a remote that lacks the branch.
- The incident reproducer (two writers, then a killed publication) is re-run by hand in scratch repositories and recorded in the ticket.

## Risks

- **One-way registry migration.** Running a build from this branch on the real registry migrates it, and other sessions on older builds may then fail. All development runs use scratch registries and isolated HOME; the first real-registry use is after merge, from main, and the merge note says so.
- **Liveness is a guess.** A reused pid can make a stale row look live (bounded by the wait, then `busy`, never a takeover); an unreadable pid makes a live row look stale, which is the same answer as today.
- **Per-item scope** may expose a shared-state dependency not seen in the code; step is last and separable.

## Left to the builder

- Column names and the migration name, the exact wait budget and poll interval, and the wording of the one-clause messages.
- Whether the restart also writes `FetchPrepared`.
- How host identity is read on each platform.
