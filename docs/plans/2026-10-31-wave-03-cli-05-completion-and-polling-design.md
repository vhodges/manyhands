---
title: "Wave 03 C5 CLI Completion And One-Shot Polling Design"
date: 2026-10-31
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M69C6S8DV9EJ7NMWKEWCQS0D"
---

# Wave 03 C5 CLI Completion And One-Shot Polling Design

## Intent And Approval Boundary

This design turns the [Cycle](../Cycles/wave-03-cli-05-completion-and-polling.md)
into modules, bindings, verbs and tests. The Cycle governs it. It is a draft;
approving it does not authorize implementation, a merge, closure, native CI or
a `ticket save`. Decisions are the Cycle's, and the owner questions it lists are written to their recommended answers. Rulings R-n are planning's, each
with its cost if wrong.

## Baseline Facts

At `717b664d` (main `a52273ff` merged). Task 0 re-checks each one.

**Commands and catalog**

1. `Command` (`src/repository/mutation/command.rs:37-75`) holds 37 commands and
   none for promotion, closure or polling. Every predicate is an exhaustive
   `match` with no wildcard arm: `has_operation`:92,
   `was_accepted_without_operation`:178, `restores_index`:222,
   `creates_its_repository`:267, `stops_at_safe_points`:318,
   `finishes_as_a_synchronization`:383, `stage`:429,
   `hands_off_to_any_refresh`:486, `needs_confirmation`:537.
2. Four commands are in no CLI catalog: `context sync-local` and
   `context resolve-local` (`command.rs:68-69`), which C5 adds, and
   `context prepare` and `context release` (`command.rs:73-74`), D3's library
   commands, which stay out of the CLI (open question 5).
3. Six verbs are `planned(.., "C5")` (`src/bin/manyhands-cli/catalog.rs:987-992`),
   and `catalog_tests.rs` lists them under "C5".
4. `Availability::Planned` maps to `Action::Unavailable` (`parse.rs:168`), with
   the golden `tests/fixtures/cli_v1/unavailable_ticket_close.json`, pinned at
   `tests/cli/help.rs:277-304`. C5 removes the last planned verb.

**Promotion and closure**

5. `preview_document_promotion` (`promotion/mod.rs:380`) takes `{root, item_id}`
   and no identity. `preview_ticket_closure` (`closure/mod.rs:349`) adds
   `confirmed_identity`.
6. `promote_document` (`promotion/mod.rs:361`) and `close_ticket`
   (`closure/mod.rs:359`) take the operation ID, the expectation, an optional
   host approval, an optional confirmed identity and a credential session. Both
   return `Complete(outcome) | IndexPending(IndexPending<outcome>)`.
7. Expectations are bare 32-byte digests (`closure/mod.rs:59`,
   `promotion/mod.rs:264`). The domain has no time expiry. The ten minutes come
   from F2 (`CONFIRMATION_LIFETIME_SECONDS`, `mutation/confirm.rs:44-47`).
8. Retry semantics (Cycle 09, item 10): "The same ID with the same digest
   reconciles the record against Git and resumes at the first unfinished step.
   After integration consent is not re-checked, and only publication and cleanup
   can run. A completed ID returns its stored outcome with no transport." A new ID
   is `Busy` while the item has an unfinished record, except one whose cleanup
   stopped on `context_changed`, `ignored_files` or `remote_context_changed`
   (`closure/record.rs:435 supersedable`).
9. `run.rs:571` computes `cleanup_only` at `Step::Confirmed`, from
   `runs_context_child` (`promotion/lifecycle.rs:202`). No new cleanup API is
   needed.
10. `ClosureBlocker` has `LocalOperationPending`, `CommentPublicationPending` and
    `IdentityRequired`; `PromotionBlocker` (`promotion/mod.rs:201`) has none of
    them. Both enums' order is a stored refusal mask (`closure/mod.rs:30`).
11. `closure/mod.rs:334-337`: a repository error of kind `ExternalChange` never
    becomes the consent refusal, because it may follow a written checkpoint.
12. `PlannedClosure` is `Record{closed_by} | Existing{closed_at, closed_by}`
    (`closure/mod.rs:92`). A preview's `changes()` lists whole-branch path changes.

**Identity**

13. `select_committing_identity` (`remote/sync.rs:3006`) returns the configured
    identity first and looks at a confirmation only when none exists.
    `promotion_identity` (`promotion/integrate.rs:616`) installs a confirmation in
    repository configuration before any commit. `closure/checkpoint.rs:540` requires
    `closed_by` to equal the checkpoint's signature. `commits_unchanged`
    (`src/repository.rs:10808`) is the onboarding check.

**Resume and boundary**

14. `bind/resume.rs:333-337` answers an unfinished remote operation with no
    synchronization target `Named::NotResumable`, and `:36-37` and `:212-214` state
    that a poll, a promotion or a closure is `invalid_input`.
15. `item republish` (`bind/republish.rs`) is the model confirmed binding: `Binding`
    with `ended_without_checkpoint`, `prepare`, `run`, `reenter`, `rerun`,
    `as_confirmed`; `Confirmed` with `preview`, `row`, `observe_again`.
16. A finished record's stored envelope is replayed unchanged with no domain call
    (`mutation/replay.rs:20-28`).
17. `PreviewDetailsDto` (`mutation/dto.rs:218`) and `MutationDataDto` (`:570`) are
    closed sets with schema files under `schemas/v1/`.

**Polling**

18. `poll_publication_remote` (`remote/poll.rs:1195`) takes `{root, operation_id,
    invocation, approval, restart}`. Eligibility is checked only for `Automatic`
    (Task 0 confirms by test). It returns `Complete | IndexPending` of
    `NotEligible | Polled(RemotePollReport)`.
19. `set_remote_polling` (`remote/state.rs:6384`) overwrites `enabled`, `paused` and
    `interval_seconds` together. `resume_remote_polling_after_recovery` (`:6366`)
    is separate. `PollingInterval::from_seconds` allows 60–3600.
20. Nothing outside tests calls either write or `poll_publication_remote`.
21. `polling_status` (`read/status.rs:710`) already backs `poll status`. The
    doc on `PollingStatusDto.next_eligible_at` (`read/dto.rs:723`) says no read
    fills it, while `polling_status` does. Task 5a corrects the comment.
    `remote_polling_state.next_eligible_at` is a stored column
    (`remote/state.rs:61`), and a null or past value is due (`remote/poll.rs`
    `eligibility`), so "make due" needs no schema change.

**Local synchronization**

22. Reads: `observe_local_context_sync` (`local_sync/observe.rs:360`),
    `inspect_local_context_sync` (`read/local_sync.rs:110`),
    `read_local_context_conflict(repo, operation_id, token)` (`:117`). Their
    schemas are in `schemas/v1/local_context_*`.
23. `LocalContextSyncInput{root, kind, id, observation}`,
    `LocalContextResolveInput{root, operation_id, observation, resolutions}`,
    `LocalConflictResolutionInput{token, source}` (`bind/local_sync.rs:10-20`).
24. An unfinished local sync fences closure: `ClosureBlocker::LocalOperationPending`
    and `closure/record.rs:527,627`; for promotion,
    `promotion/lifecycle_record.rs:354`. It also makes C4 verbs answer
    `recovery_required` (C4 R-38).

**Readiness**

25. `ticket_dependencies(repo, id, Up, depth)` (`read/graph.rs:913`) gives
    dependents, but its line states are only `Open`, `Closed` and `Unresolved`
    (`read/dto.rs:321`). `is_ready` (`:263`) is "open, and every dependency a
    closed ticket", and `readiness_state` (`:272`) is `pub(super)`, so made-ready
    needs a new function inside `read/`. `L::complete` is returned only after a
    successful refresh or a clean replay check (`promotion/run.rs:1540-1563`).

**Test support**

26. `CliPair` (`tests/support/cli_pair.rs`), mounted only from `tests/cli_remote.rs`.
    `SshRemoteFixture` offers `reject_primary_updates` and `reject_context_deletion`
    (`tests/support/ssh_remote.rs:474,484`). `tests/cli_remote.rs` is
    `harness = false`, and each case is registered in its module's `CASES`.
27. `tests/cli/collaboration_exits.rs` accounts for the audit's codes by set
    equality; `tests/cli/exits.rs:18` lists `command_unavailable` as CLI-only.
28. C4's two-clone limits: select keys before enabling (`01M66HFV44`), and
    `remote select` leaves the Git index stale (`01M669FJR2`, fixed here,
    decision 10).

**What the first review found**

29. A lifecycle parent usually has no journal row. `JournalRow` for the Remote
    family reads only `remote_operation_records` (`mutation/mod.rs:239`), and the
    runner reserves one only when `plan.is_some() && r.remote_deletion_required()`
    (`promotion/run.rs:1128,1163`). So through merge, checkpoint and publication the
    row is `Absent`, and a local-only run never has one. Republish's `row` logic
    does not fit: `Stopped{Some(Absent)}` deletes the request record
    (`mutation/settle.rs:48,64-70`), and `Recorded` with `Absent` is `internal_error`
    (`mutation/confirm.rs:213-218`).
30. Cancellation does not reach these calls today. Safe points read the lifecycle
    table's `cancel_requested` (`promotion/run.rs:226-232`), set only by
    `cancel_document_promotion` and `cancel_ticket_closure`
    (`lifecycle_record.rs:942-961`). A retry path clears it (`:977`; Task 0 confirms
    it covers a same-ID retry). `cancel_remote_operation` errors with no row
    (`remote/reservation.rs:1231-1232`). None of the three domain calls takes a token,
    and the CLI's Ctrl-C handler only cancels a token (`interrupt.rs`).
31. `without_preview` discards an answer's data (`mutation/confirm.rs:93-95`), and
    `PrepareDataDto` is `Preview | HostReplacement` (`dto.rs:542`), so blockers need a
    new variant.
32. `MutationDataDto::from_stored` matches `Command` exhaustively (`dto.rs`), so a new
    command needs an arm there from its first commit.
33. `should_retry` retries a `Partial` result whose first action is `request.retry`
    up to three times (`mutate.rs`, `RETRY_DELAYS_MS`).
34. `stops_on_ctrl_c` is a closed list (`mutate.rs:53-63`), pinned at
    `collaboration_tests.rs:82`.
35. `Command` is `pub(crate)`, so `tests/cli` cannot enumerate it; `mutation_remote`
    mounts `src/lib.rs` and can. `tests/mutation_contract.rs:1455` holds
    `UNBOUND_COMMANDS`.

## Module Layout

**Library**

- Create `src/repository/mutation/bind/lifecycle.rs`: the shared confirmed binding
  for both verbs, generic over a small `LifecycleVerb` trait with a promotion and
  a closure implementation. Promotion and closure already share one runner, so
  they share one binding (R-1).
- Create `src/repository/mutation/bind/poll.rs`: `poll once` and the three policy
  bindings.
- Modify `command.rs`, `mutation/mod.rs` (`Mutation`), `bind/mod.rs`,
  `bind/resume.rs`, `dto.rs`, `outcome.rs`, `confirm.rs` (blocker data),
  `progress.rs` (`cancel_request`), `schemas/v1/`.
- Modify `promotion/` and `closure/` for the parent-record accessor, the cancellation
  probe and identity; `remote/poll.rs` for the probe; `remote/state.rs` for the atomic
  update; `remote/sync.rs` for identity.
- Modify `src/repository.rs` for `01M669FJR2` (decision 10).
- Modify `read/graph.rs`, `read/dto.rs`, `read/mod.rs` for the made-ready read and the
  corrected comment.

**CLI**

- Create `src/bin/manyhands-cli/lifecycle.rs`, `polling.rs`, `local_sync.rs`, each
  with `_tests.rs`, and `human_lifecycle.rs`, in C1's `#[path]` pattern.
- Modify `src/bin/manyhands-cli.rs`, `catalog.rs`, `dispatch.rs`, `mutate.rs`,
  `render.rs`, `human.rs`.
- Create `schemas/input/v1/poll_configure.schema.json`,
  `context_sync_local.schema.json`, `context_resolve_local.schema.json`.

## Command Predicates

| Command | `needs_confirmation` | `has_operation` | safe points | finishes as sync | hands off to any refresh | family |
| --- | --- | --- | --- | --- | --- | --- |
| `document promote` | yes | yes | yes | no | no | Remote |
| `ticket close` | yes | yes | yes | no | no | Remote |
| `poll once` | no | yes | yes | no | yes | Remote |
| `poll configure`, `poll pause`, `poll resume` | no | no | no | no | no | none |

For all six, `was_accepted_without_operation`, `restores_index` and
`creates_its_repository` are no, and `stage` is the mutation stage of a remote
operation for the first three and of a local write for the policy verbs. Each is pinned
by a named test. Every lifecycle row is `EndedWithoutCheckpoint::SaysNothing`
(fact 15).

## `document promote` And `ticket close`

**Grammar.** `--repo`, `--id`, `--prepare`, `--confirm <id>`, `--request-id`,
`--json`. No `--input`: both take no input (RFC, "CLI callers save drafts first").

**Prepare.** Calls the preview. `Ready` returns `PreviewDetailsDto::Lifecycle`: kind,
item ID, slug when a ticket has one, branch, primary branch, remote, context and primary
commits, whole-branch path changes, planned integration, publication, cleanup, and for
closure the planned `closed_by` and whether it is `record` or `existing`. No title, body
or content.

`Blocked` returns the new `PrepareDataDto::LifecycleBlocked { blockers }` (fact 31),
through an `Answer` field modelled on `host_replacement`. Its code:

- `[IdentityRequired]` alone: `identity_required`, with `repo.identity_set`;
- `[AlreadyClosed]`: `ticket_closed`;
- anything else, mixed blockers included: `recovery_required`, naming every blocker,
  with `repo.inspect` as its action. `ContextMissing` is one: `ticket save` or
  `document save` makes a context (Task 0 confirms the verb).

An unknown ID, wrong kind or unregistered repository answers as `item republish` does:
`item_not_found`, `repository_not_registered` and `repository_not_enabled`.

**Re-entry (the mechanism; finding: republish's does not fit, fact 29).** The binding
reads the operation's parent record through a new crate-private accessor on each domain
module: `lifecycle_parent(root, operation_id) -> Option<ParentView>` with `step`,
`expectation`, `item_id`, `cancel_requested`, `cleanup` and the recorded closure and
integration commits. The binding overrides `reenter` rather than using
`confirm::reenter`:

| State found on re-entry | What happens |
| --- | --- |
| No parent record | Crash between acceptance and record creation. Observe again: a digest match runs as a first call with `preview().expectation()`. A mismatch answers `external_change`, and the record is deleted. |
| Unfinished parent, any step | `Entered::Unfinished`. The domain is called with the **record's** expectation and the same operation ID. No digest comparison: the request's own work moved the state. |
| Complete | The domain returns its stored outcome, with no transport. |
| Refused and ended | The stored refusal. A new prepare goes on. |
| Superseded | `request_mismatch`. |

**Standing after a call.** Never `Stopped{Absent}` while a parent is unfinished.

| Parent after the call | Standing | Next |
| --- | --- | --- |
| No record, refused before it began | `Stopped{Some(Absent)}`: record deleted, confirmation released, ID reusable (F2) | New prepare |
| Ended by a consent refusal | `Final`: the refusal is stored | New prepare |
| Unfinished, including cancelled and partial | `Owed` (decision 8) | Same request; `operation resume` if no request holds it |
| Complete, index hand-off pending | `Owed` | Same request recomputes made-ready |
| Complete | `Final` | None |

**Answers.** Exit status is derived from the code and effects (`results.rs:282-289`), so
this table states the code and the effects that decide it. "Durable" means the parent has
a written closure checkpoint or an integration, read from the accessor, or a child made a
durable effect.

| Domain | Code | Exit | Next |
| --- | --- | --- | --- |
| `Complete`, all done | `ok` | 0 | None |
| `Complete`, publication `Pending{NoPublicationRemote}`, cleanup done | `ok` | 0 | Publication pending (RFC exit table); `repo sync` after a remote exists |
| `AlreadyContained`/`AlreadyCurrent`, nothing left | `ok` | 0 | None |
| Publication `Pending{Synchronization(c)}` | `c` below | 4 | `request.retry` |
| Publication or cleanup `Pending{Cancelled}` | `cancelled` | 130 | Same request (decision 8) |
| Cleanup `Pending{ContextChanged, IgnoredFiles, RemoteContextChanged}` | `recovery_required` | 4 | `repo.inspect`, then the same request or a new prepare, which supersedes |
| Cleanup `Pending{AwaitingPublication}` | the publication's code | 4 | `request.retry` |
| Cleanup `Pending{RemoteDeletionFailed(c)}` | `c` below | 4 | Per `c` |
| Cleanup `Pending{RecoveryRequired}` | `recovery_required` | 4 | `request.retry` |
| `IndexPending(outcome)`, nothing else pending | `discovery_pending` | 4 | `request.retry` |
| `Blocked{..}` at run | As Prepare | 3, or 4 if durable | As Prepare |
| `ExternalChange`, `ReconfirmationRequired`, `PrimaryAdvanced` | `external_change` | 3, or 4 if durable | `request.prepare` |
| `IdentityRequired` | `identity_required` | 3, or 4 if durable | `repo.identity_set`; see Identity |
| `ContextSynchronization(error)` | the child's code | per class | As `item sync`: host, unlock and transport answers |
| `Transport(error)` | the error's code, as `item sync` | 5, or 4 if durable | As `item sync` |
| `Busy` | `busy` | 5, or 4 if durable | `request.retry` |
| `PollYielding` | `poll_yielding` | 5, or 4 if durable | `request.retry` |
| `Interrupted`, cancellation asked | `cancelled` | 130 | Same request (decision 8) |
| `Interrupted`, not asked | `recovery_required` | 3, or 4 if durable | `request.retry` |
| `OperationMismatch` | `request_mismatch` | 2, or 4 after a replay's effects | None |
| `RecoveryRequired`, `Repository(error)` | `recovery_required`, or the error's code | 3, or 4 if durable | `repo.inspect` |

Categories: synchronization `Conflict` is `recovery_required`, `Rejected` is
`push_rejected`, `Transport` is `transport_unavailable`, `Identity` is
`identity_required`, `Busy` is `busy`, `Yielding` is `poll_yielding` and
`RecoveryRequired` is `recovery_required`. Remote deletion `Transport` is
`transport_unavailable`, `Rejected` is `push_rejected`, `Configuration` is
`recovery_required`, `Key` is `no_selected_key`, `Unlock` is `unlock_required` and `Host`
is `host_approval_required`. The category carries no authority, so the host row's action
is `repo.inspect`, followed by `host observe` and `host approve`.

A repository error of kind `ExternalChange` stays a repository error (fact 11). Task 2b's
test is written first. C5 adds no `ResultCode`. Automatic bounded retries (fact 33) apply
to rows whose first action is `request.retry`, so count-based assertions allow for them.

**Host and unlock.** Unattended runs pass `approval: None`. A host stop answers
`host_approval_required`; `host approve` works while the request stands; the same request
then continues. A protected key answers `unlock_required` non-interactively.

**Resume.** The route looks in the lifecycle tables first, since most unfinished parents
have no journal row (fact 29). There is no liveness check (runtime RFC). Whether a request
"holds" the operation is a stored fact: an accepted request record names it.

| Found | `operation resume` answers |
| --- | --- |
| Unfinished parent, an accepted request record holds it | `left_to_its_request`, naming the request: that request continues it |
| Unfinished parent, no request record (cache loss, or begun by the library) | Continues with the record's expectation. Data is the same lifecycle DTO, made-ready included. Ctrl-C is as the verb's. |
| Parent running in another process | The domain's `busy` |
| Complete | `already_applied`, as F2 |
| A poll | `invalid_input`, unchanged |

Resume grants no new consent (RFC: "Resume never infers permission for a different
target/action").

**Cancellation (decision 8).** Cancelling is not final for these verbs, unlike a
synchronization, and the RFC is amended. New crate-private `promote_document_observed` and
`close_ticket_observed` wrap the runner with a cancellation probe that the runner's
`cancelled` check also reads, so the public signatures are unchanged. The probe reads the
token and the request's flag. `cancel_request` also calls the lifecycle's own
`request_cancellation` for these commands. `stops_on_ctrl_c` gains both verbs. A same-ID
retry clears the flag (fact 30). A press after the last safe point keeps the domain's
answer. A second press or a kill leaves the record, and the same request continues.

**R-38 interaction.** A pending local sync gives `LocalOperationPending` at closure
prepare, so `recovery_required` naming it. For promotion the domain refuses at start, with
the same answer, after a confirmation is issued and released; this is pinned, not hidden.

## Made-Ready Report

The RFC says "became ready because of it". Closing T can only make direct dependents of T
ready, and none was ready before if this operation is what closed T. So:

```text
made_ready = direct dependents of T, in ID order,
             that the index reads as open with every dependency closed
```

evaluated when the report is produced, not reconstructed from history (R-4; the RFC is
amended to say so).

| Case | `made_ready_state` | `made_ready` |
| --- | --- | --- |
| This operation recorded the closure, or integrated it, and the index read is complete | `current` | the set |
| `IndexPending`, a failed read, or an incomplete index (`complete: false`) | `index_pending` | empty |
| Stopped before integration | `not_computed` | empty |
| Cleanup-only or superseding run: `ClosureRecording::Existing` and already contained | `not_applicable` | empty |

- A new function in `read/graph.rs`, `tickets_made_ready(repo, id)`, uses `read_tickets` and
  `readiness_state`. An index `behind` flag, usual while the context exists, does not change
  the answer: the context holds the closure.
- A partial run reports it too. Each retry evaluates again, and the request's final stored
  envelope holds the last evaluation, so a replay of a finished request returns it unchanged.
  `IndexPending` is `Owed`, so the retry computes on the first complete read.
- Readiness is lifecycle closure, by the index's effective copy. Status text plays no part.
- Each entry is `{id, slug}`. No title is stored (Cycle 09 item 12's rule, extended to
  request records). The CLI's human output prints both.
- `operation resume` reports the same way.

## Local-Sync Verbs

| Verb | Kind | Selects | Input | Output |
| --- | --- | --- | --- | --- |
| `context observe-local` | read | `--repo`, `--id` | none | `local_context_sync_observation` |
| `context sync-local` | unconfirmed mutation | `--repo`, `--id` | `{observation}` | `local_context_sync_mutation` |
| `context show-local` | read | `--repo`, `--operation-id`, optional `--token` | none | inspection, or a token's content |
| `context resolve-local` | unconfirmed mutation | `--repo`, `--operation-id` | `{observation, resolutions: [{token, source}]}` | `local_context_sync_mutation` |

These names are final (decision 3). The observation travels in `--input`, as every mutation
observation does (RFC), not as a flag.

- The item's kind is resolved from the item, and an unknown ID is `item_not_found`.
- `--token` returns `{base, local, incoming, historical}` with `--json` only, matching
  `conflict show`'s sides (open question 3). Human mode refuses it with `usage_error`, and
  points to `--json`. Tokens are tamper-checked by the library (gate A10).
- Source stays out of request records, as the library guarantees.
- Bin code holds no merge logic.
- Both mutations join `stops_on_ctrl_c`. The library's `original_request_required` answer and
  `required_request_id` resend are proven through the binary.
- `operation abandon` keeps routing a local sync to the gate's guarded abandonment.

## Polling Verbs

**`poll once`.** `--repo`, `--request-id`, `--json`. No input. Explicit invocation, which
skips eligibility (`remote/poll.rs:1241`), so it runs while paused or disabled; Cycle 08
item 2 and the runtime RFC settle this. Before calling the domain the binding refuses a
missing publication remote without writing, as republish does. Otherwise the domain's plan
failure would write policy state (`poll.rs:1254-1283`).

Data is `PollMutationDto`: operation ID, completion, completed time, next eligible time, and
per-target `{target, kind, item_id, disposition, before_oid, after_oid}`, each nullable as the
report is, for malformed rows.

| Domain | Code | Exit | Standing and next |
| --- | --- | --- | --- |
| `Polled`, `Completed` | `ok` | 0 | `Final`. Dirty, diverged, conflicted, malformed, deleted and incomplete targets are data, with guidance, not failures (R-16) |
| `IndexPending` | `discovery_pending` | 4 | `Final`; `index.refresh` (`hands_off_to_any_refresh`) |
| No remote | `publication_remote_required` | 3 | Deleted; nothing written |
| `Polled`, `Cancelled` | `cancelled` | 130 | `Final`; a new request polls again. Polls follow the RFC's final cancellation, since a new ID is not blocked |
| `Polled`, `Interrupted`, or `Err(Interrupted)` | `recovery_required` | 3, or 4 if a target moved | `Owed`; the same request continues with `restart` |
| `Polled` with an outcome category | that category's code, as `item sync` | per class | `Final` |
| `Transport(error)` | the error's code: `unlock_required`, `host_approval_required`, `host_mismatch`, `key_rejected`, `no_selected_key`, `transport_unavailable`, `remote_unavailable` | 3 or 5 | Held as a synchronization's; `host approve` or unlock, then the same request |
| `Busy` | `busy` | 5 | `request.retry` |
| `PollYielding` | `poll_yielding` | 5 | `request.retry` |
| `ConflictPending` | `recovery_required` | 3 | `repo.inspect` |
| Pending local sync | the domain's answer, pinned | 3 | `repo.inspect` |

A restart reports what its own pass saw. A target an interrupted first pass materialized reads
`current`, so the final envelope understates it; Git holds exactly one worktree (known limit).
A killed poll keeps its reservation: the same request continues it, and if the ID is lost
`operation list` names the operation (known limit, with `01M4R19PC0T6ZPKM39X9DF26HX`). Ctrl-C
reaches it through a probe, as for the lifecycle.

**Policy verbs.**

| Verb | Effect | Already so |
| --- | --- | --- |
| `poll configure` (`--input {interval_seconds}`, 60–3600, closed schema) | Sets the interval, leaving the stored due time | `already_applied`, 0 |
| `poll pause` | Sets `paused` | `already_applied`, 0 |
| `poll resume` | Clears `paused` and clears `next_eligible_at`, so an eligible poll is due (decision 11, runtime RFC) | `already_applied`, 0 |

- `poll resume` leaves `recovery_suspended`, `enabled` and the backoff alone, and its human
  output names a suspension when one is set (decision 5).
- No verb sets `enabled`.
- An out-of-range interval is `invalid_input`, and nothing is written.
- Pausing during a running poll doesn't stop that poll; it applies to the next automatic one.
- An unregistered repository is `repository_not_registered`. A registered one always has its
  polling row (Task 0 confirms); pause survives a remote's removal and re-adding (RFC).

**Atomic update (R-7, open question 2).** A new `update_remote_polling(root, change)` reads and
writes one field in `with_transaction`. `set_remote_polling` has test callers only, and stays
for them. D6 uses the boundary's bindings. The poll's own completion writes are field-scoped
(`record_poll_attempt`, `state.rs:2389`); Task 0 confirms `state::configure` is. A retry of an
unfinished policy request applies its own intent again, so a stale retry can win on that one
field (known limit). A finished one replays.

**No worker.** No thread, no child, no service. Transport starts only for `poll once`.

## Identity

Decision 6 and `01M5MK57`. Both verbs pass `confirmed_identity: None`.

| Stage | Case | Answer |
| --- | --- | --- |
| Closure prepare | `IdentityRequired` the only blocker | `identity_required`, `repo.identity_set`. No request exists: a **new prepare** follows. |
| Promotion prepare | A merge commit is planned, and nothing else blocks | The same, through the binding's read-only selector check, made last (R-3) |
| Either prepare | Already contained, or existing closure | No identity needed |
| Closure confirm | The identity changed since prepare | The digest binds `closed_by`, so `external_change`, record ended; a new prepare |
| Promotion confirm | Identity lost before integration | `identity_required`, `Owed`; after `repo identity-set` the same request continues |
| Either | A context child's `IdentityRequired` | `identity_required`, `Owed`; the same request continues |
| After a closure checkpoint | | `closed_by` comes from the commit: none needed |

**The fix.** Reproduce first, with a failing test per path. Apply `commits_unchanged` to effective
and confirmed identities in `select_committing_identity`, before any journal effect, configuration
write or commit. An unusable configured identity counts as none. An unusable confirmed identity
keeps `ExternalChange`, and nothing is persisted (R-5); no CLI verb reaches this path, so only
library tests do. Each refusal asserts configuration, refs and journal unchanged, and that no recovery
obligation is created. A regression pins that existing recovery records and replay results are kept.
`effective_identity`'s per-key cascade stays (R-6, open question 1).

## Remote Selection Fix (`01M669FJR2`, decision 10)

Reproduce through the binary in Task 0, on the merged baseline: `repo enable`, `remote select`, `repo
sync` answers `worktree_not_clean`. After the fix, a successful `remote select` leaves the index
matching its commit, Git status is clean for `.manyhands/config.toml`, and the immediately following
synchronization proceeds. The fix is in `set_publication_remote_recorded` (`src/repository.rs`). The
ticket's acceptance governs. C4's fixture seed stays.

## Recovery Tables (decision 4)

`docs/RFC/cli-contract.md` gains one table per family: authoring, repository and credentials,
collaboration, lifecycle, local sync, polling. Each row is a verb, a recoverable code, the `next:`
action and the exit class, and names a golden. `tests/cli/recovery_tables.rs` parses the tables and
requires each golden to exist and hold that code and action. For C5's verbs every recoverable code in
their outcome tables needs a row. Earlier verbs are checked against their audit code lists, and a code
no fixture reaches carries a reason, as in the exit table (R-10).

## Inventory And Evidence

**Inventory (R-9).**

- `c5_inventory_commands` in `mutation_remote`, which mounts the library: `Command::ALL` names equal
  the catalog's available mutating verbs in `help_root.json`, less the exempt `context prepare` and
  `context release` (open question 5).
- `tests/cli/inventory.rs`: every help-root verb appears in the RFC taxonomy or, for the relationship
  verbs, the relationships RFC's table, and the reverse. No verb is `Planned`. `<verb> --help --json`
  answers for every verb, with its catalog options. Every input-taking verb has exactly one input schema
  and example; every other verb is listed as no-input. Every verb's data type has an output schema.
- Exit behaviour and recovery are the exit table's and the recovery tables' tests.

The Cycle's "help golden" is this generated assertion for every verb, with goldens for C5's verbs.

**Planned mechanism (R-8).** `Availability::Planned` and `planned()` become `#[cfg(test)]` or
`cfg_attr(not(test), allow(dead_code))`, as clippy needs, exercised by a test-only spec.
`command_unavailable` stays a CLI-only code, with an unreachable reason. RFC:770 is amended.

**Per-verb evidence (track rule 7; RFC "every command's human/JSON result, input failure, no-op and
applicable recovery/exit class").** Every cell holds, or carries its reason.

| Verb | Human + JSON | Input failure | No-op | Prepare/confirm/interactive | Ctrl-C | `request cancel` | Replay | Mismatch | Same-request continuation | Lost stdout |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `document promote` | 7 | 7 | 7 | 7, 12b | 13a | 13a | 12b | 7 | 12b | 13a |
| `ticket close` | 7 | 7 | 7 | 7, 12b | 13a | 13a | 12b | 7 | 12b | 13a |
| `poll once` | 9 | 9 | none: every poll polls | none | 13a | 13a | 9 | 9 | 13a | 13a |
| `poll configure` | 8 | 8 | 8 | none | none: no safe point | none | 8 | 8 | 8 | 8 |
| `poll pause`, `poll resume` | 8 | 8 | 8 | none | none | none | 8 | 8 | 8 | 8 |
| `context observe-local`, `show-local` | 10a | 10a | read | none | none | none | read | read | read | none |
| `context sync-local` | 10a | 10a | 10a | none | 13a | 13a | 10a | 10a | 10b | 13a |
| `context resolve-local` | 10b | 10b | none | none | 13a | 13a | 10b | 10b | 10b | 13a |

## Journeys

Each is a `c5_journey_*` case over `CliPair`, asserting Git objects and canonical content, and
scanned for secrets.

| PRD journey | Counterpart | Variants |
| --- | --- | --- |
| Offline edit, checkpoint and later sync | Save with the remote unreachable, then `item sync` | Remote refusal gives recovery with local work kept |
| Conflict recovery | C4's `c4_conflicts_*`, referenced by a thin journey that resolves and retries | None new |
| Background update and discovery | Peer publishes primary and a new item; `poll once`; discovery shows one worktree and indexed comments | Dirty, divergent, malformed, remotely deleted |
| Threaded discussion | Roots and replies, `index rebuild`, a worktree change, `comment list` | None new |
| Document promotion | Save, `--prepare`, `--confirm`; a fresh `document save` starts a new context | Local-only, then `key select`, `remote select`, `repo sync` |
| Close with retryable failure | Save, `--prepare`, `--confirm`; `ticket ready` matches the report | Local-only as above; an injected failure at a close prerequisite (dirty primary), naming the failed step, then a retry after correction; refused push; refused deletion |

"Final save" differs by design: CLI callers save first (RFC), and the journey says so. The key is
selected before the remote in the later-publication order, so `01M66HFV44` is not reached; Task 0
confirms it.

## Dogfooding (R-12, decision 9)

C5's real-repository evidence uses a **disposable clone of this repository with a fresh
`MANYHANDS_DATA_DIR`**, which brings the real content, including its plain `docs/` files, without
touching the owner's data. Task 11 runs every C5 verb there: prepares and reads first, then executing
`document promote` on a throwaway document, `ticket close` on a throwaway ticket, `poll configure`,
`pause`, `resume`, `poll once` against a throwaway bare remote, and all four context verbs. A refusal
or partial result stops for the owner.

**C5's own closure** is a canonical file edit before merge, as C4's was, on the owner's approval
(decision 9). `ticket close` is itself the merge, push and cleanup, so it cannot precede a reviewed
merge of its own branch. `01M5MK57` and `01M669FJR2` close the same way: their ticket files, which live
only on their own branches, are added to this branch with the closure fields, each with a comment. Their
stale branches are removed only on the owner's go-ahead.

**After merge,** in a separate session with its own go-ahead: close one real small ticket through
`ticket close`, then switch AGENTS.md and `file-ticket` to it.

## Exit Statuses

Follow the RFC table. Local-only completion is 0. Partial after any durable effect
is 4. Blocked before any is 3. `busy` is 5. Ctrl-C is 130, or the domain's answer after
the last safe point. `tests/cli/collaboration_exits.rs`'s set equality is extended with
C5's codes in Task 13b.

## Testing

No new test target (R-11). Cases extend existing targets, named `c5_<module>_<case>` in `harness =
false` targets, split per row under the 100-second watchdog. Library lifecycle rows use a new
`tests/support/lifecycle_world.rs` on `remote_world.rs`, since `tests/ticket_closure.rs` and
`tests/document_promotion.rs` keep their helpers private; small duplication is accepted over moving them.

| Target | Adds |
| --- | --- |
| `--lib` | Command predicates, identity, `update_remote_polling`, `tickets_made_ready` |
| `mutation_remote` | Lifecycle, resume and polling tables, made-ready, inventory of commands |
| `mutation_contract`, `read_contract` | Goldens and schemas |
| `document_promotion`, `ticket_closure`, `remote_synchronization` | Identity regressions |
| `repository_enablement` | `remote select` index |
| `cli` | Grammar, help, inventory, recovery tables, local-only journeys, context verbs |
| `cli_remote` | Two-clone journeys, failures, Ctrl-C, no-worker |

Application data is scanned for sentinel `closed_by` and title values after a closure: a confirmation
stores a digest, previews are not persisted, and request records hold neither (Task 0 confirms).

## Named Edits To Pre-Existing Tests

| ID | Test | Edit |
| --- | --- | --- |
| E-1 | `src/bin/manyhands-cli/catalog_tests.rs` `DESIGN` | Six verbs become available with C5's; add four context verbs |
| E-2 | `tests/cli/help.rs:277-304` | Replace with a test that `ticket close` reaches its handler; remove `unavailable_ticket_close.json` and its registration |
| E-3 | `src/bin/manyhands-cli/parse_tests.rs:216,235` | Move to a test-only planned spec |
| E-4 | `src/bin/manyhands-cli/startup_tests.rs:332,362,371,440-442` | Same |
| E-5 | `tests/cli/boundaries.rs:358` | `ticket close` with no `--id` answers `usage_error`, its real pre-dispatch answer |
| E-6 | `tests/cli/exits.rs:18,141-143` | The `ticket close` fixture moves to a test-only route or is replaced by the reason |
| E-7 | `tests/fixtures/cli_v1/help_*` | Help goldens regenerate, read in the diff |
| E-8 | `tests/cli/collaboration_exits.rs` | Code set grows |
| E-9 | `command.rs` tests | New names; extend `the_published_names_are_the_ones_records_hold` |
| E-10 | `tests/mutation_remote/resume.rs`, any case pinning promotion or closure as `invalid_input` | Now the resume table; polls unchanged |
| E-11 | `tests/mutation_contract.rs` registered cases, `tests/support/mutation_golden.rs` | Additive registration |
| E-12 | `outcome_tests.rs` over `Command::ALL` | New commands' rows |
| E-13 | `tests/mutation_contract.rs:1455` `UNBOUND_COMMANDS` | Holds any new command while unbound in Tasks 1–5, and returns to its present list |
| E-14 | `src/bin/manyhands-cli/collaboration_tests.rs:82` | New `stops_on_ctrl_c` rows |

`tests/read_relationships.rs:1183,1190,1254` hold `"C5"` as Wave-graph data, and are not pins. Task 0
greps for other pins (`NotResumable`, `"C5"`, `arrives with`, `Planned`). An unlisted hit stops for owner
review. This list is not yet proven complete.

## Document Amendments

- **CLI contract RFC:**
  - taxonomy rows and grammar for the four context verbs;
  - lifecycle, poll and made-ready shapes;
  - recovery tables;
  - removal of "C5 remains cataloged as planned";
  - `:770` (`command_unavailable`);
  - `:972`, `:993-998`: the Ctrl-C window, `request cancel` lists and lifecycle cancellation (decision 8);
  - `:1139-1142`: the fixed known limit (decision 10);
  - new known limits.
- **Git workflow RFC `:411`:** lifecycle cancellation (decision 8).
- **Wave:** track rules 2, 5 and 6 (decision 1; D5 and D6 wait for C5's merge, recorded as C4's D4 ordering was); C5's scope line for the two read verbs and two folded tickets; the "merge prerequisite remains in force" line marked historical; capability table; D6's section for decision 5; the API-audit timing departure (open question 4). `:351`'s "rebase" noted as superseded by AGENTS.md.
- **Gate Cycle:** "C4/D5 consume" becomes C5/D5.
- **API audit:** C5 rows reconciled, and the unamended sentence near `:190`.
- **Ticket relationships RFC:** the made-ready definition and its report-time evaluation.
- **AGENTS.md, `file-ticket`:** "closure remains a canonical ticket-file edit until C5's `ticket close`" becomes "until the post-C5 switch" (decision 9).
- **C5 ticket:** "rebased" wording corrected and the local verbs added, by commit on the branch, since `ticket save` is separately gated.
- **D5 and D6 tickets:** notes that C5 delivered the bindings, with the owner's go-ahead.

## Rulings

| ID | Ruling | Cost if wrong |
| --- | --- | --- |
| R-1 | One generic lifecycle binding for both verbs, matching the domain's one runner. | Generic churn; split with no contract change. |
| R-2 | Both verbs take no `--input`. | Adding later is additive. |
| R-3 | Promotion's missing identity is checked by the binding at prepare, only when a merge commit is planned and last, leaving the stored mask untouched. | A second check site could drift; the runner remains the guard. |
| R-4 | Made-ready is direct dependents, evaluated at report time, with the four states. | The list can differ from a historical diff; the RFC says so. |
| R-5 | An unusable confirmed identity keeps `ExternalChange`. | Library wording only. |
| R-6 | `effective_identity`'s cascade is unchanged. **Owner question 1.** | The two selectors can disagree on a split configuration. |
| R-7 | Policy verbs do a single-field atomic update with no observation token, replacing the outline's guard. **Owner question 2.** | A stale intent can win on its own field. |
| R-8 | `command_unavailable` remains a code with an unreachable reason. | Removal later is simple. |
| R-9 | The inventory parses the RFC tables. | Format coupling; a failing test names it. |
| R-10 | Recovery rows are checked against goldens by parsing RFC tables. | Same. |
| R-11 | No new test target. | Slower existing targets. |
| R-12 | Real-repository evidence uses a disposable clone with fresh data. | A defect specific to the owner's registry would not show. |
| R-13 | Withdrawn: `poll once` under a pause is settled by Cycle 08 item 2 and the runtime RFC. | None. |
| R-14 | Withdrawn: decision 10 replaces it. | None. |
| R-15 | `context show-local --token` is JSON only. **Owner question 3.** | Humans read sides through `--json`. |
| R-16 | A completed poll with preserved or marked targets is `ok`, exit 0, with dispositions in data. The RFC says once is explicit polling, not synchronization. | A script checking only the exit misses a blocked target; human output names them. |

## Known Limits

- Closes record the configured identity (decision 6).
- A closure race may have no CLI way out until `01M4H33R34Z7C7EEKTY1ZCT950` lands.
- `poll once` holds the remote reservation, so it answers `busy` beside another remote operation, and they answer it.
- A CLI-only user sees `recovery_suspended` without a verb to clear it (decision 5).
- A protected key answers `unlock_required` non-interactively, so agents' runs need a terminal until `01M67Y754SJ1FTYQJFR37Z4ZTY` lands.
- A retried or restarted report reflects its own pass (made-ready, and poll targets).
- A killed poll with a lost request ID is found through `operation list`.
- A stale retry of an unfinished policy request can win on its own field.
- A promotion blocked by a pending local sync issues a confirmation before refusing.

## Alternatives Rejected

- **A separate library ticket:** decision 1.
- **Caller-chosen `closed_by`:** decision 6.
- **Extending `conflict show` to local syncs:** decision 3.
- **Recovery examples in `--help`:** decision 4.
- **A before/after readiness diff:** no case needs it (R-4).
- **Observation-guarded policy writes:** R-7.
- **Closing C5 with `ticket close`:** decision 9.

## Risks

| Risk | Control |
| --- | --- |
| Re-entry mechanism meets a state the table lacks | Task 2a's table test comes first, and a failed row stops for a ruling. |
| Counted-list conflicts with D-track merges | Additive; merge main at each Part. |
| `mutation_remote` already runs near an hour | Filters in tasks; full suite once. |
| Unfinished local syncs fence C4 verbs more often | Row in each outcome table, and the local-recovery journey. |
| Full suite exceeds its budget | Detached run with no tool timeout; C4 took 3 h 34 min. |
