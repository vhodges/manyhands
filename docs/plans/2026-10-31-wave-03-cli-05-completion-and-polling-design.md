---
title: "Wave 03 C5 CLI Completion And One-Shot Polling Design"
date: 2026-10-31
status: "approved"
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
15. `item republish` (`bind/republish.rs`) shows a confirmed binding's shape, not its row logic (fact 29): `Binding`
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
    (`lifecycle_record.rs:942-961`). A same-ID retry clears it (`run.rs:491-498`, `:977`). The
    request's own flag is separate, and only `take_cancellation` clears it
    (`records.rs:967`). `cancel_remote_operation` errors with no row
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
35. `request_store::bound_commands()` is public (`mutation/mod.rs:605-610`, used at
    `tests/mutation_replay/conformance.rs:21`) and equals `Command::ALL`.
    `tests/mutation_contract.rs:1476-1489` requires every bound command to have a
    golden, and `tests/support/conformance.rs` `HOMES` a home, or four targets fail.
36. A confirmed command's re-entry is `confirm::reenter`, "whatever its binding's own
    `reenter` is" (`mutation/reenter.rs:175-181`). `Row::Recorded` with `Absent` is
    `internal_error`.
37. Bindings reach credentials only through `&dyn Session`, whose methods are
    `synchronize`, `republish` and `verify_transport` (`progress.rs:184-225`).
38. `outcome.rs:993-996` prepends `request.retry` for a stopping command with a left
    record, and a binding's suggestion replaces a code's actions (`:660-671`).
    `should_retry` applies to `Busy` or `Partial` with that action first.
39. A poll's transport failure finishes its row as `failed`, and a same-ID call returns
    the stored result (`remote/poll.rs:1233-1241,1385-1398`), reported as `Completed`
    with an outcome category (`:1967-1985`). Only `Interrupted` continues with `restart`.
40. `recorded_effects` matches `Command` exhaustively and returns nothing for `Absent`
    (`bind/mod.rs:953-1052`).
41. `PrimaryAdvanced` resets the context and leaves the parent unfinished
    (`promotion/run.rs:648-653`). `integration_oid` is saved before the ref moves.

## Module Layout

**Library**

- Create `src/repository/mutation/bind/lifecycle.rs`: the shared confirmed binding
  for both verbs, generic over a small `LifecycleVerb` trait with a promotion and
  a closure implementation. Promotion and closure already share one runner, so
  they share one binding (R-1).
- Create `src/repository/mutation/bind/poll.rs`: `poll once` and the three policy
  bindings.
- Modify `command.rs`, `mutation/mod.rs` (`Mutation`), `bind/mod.rs`,
  `bind/resume.rs`, `dto.rs`, `outcome.rs`, `confirm.rs` (blocker data, `Row::Unfinished`),
  `execute.rs` (policy transaction), `records.rs`, `progress.rs` (`Session` methods,
  `cancel_request`), `schemas/v1/`.
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
| `document promote` | yes | yes | yes | yes | no | Remote |
| `ticket close` | yes | yes | yes | yes | no | Remote |
| `poll once` | no | yes | yes | yes | yes | Remote |
| `poll configure`, `poll pause`, `poll resume` | no | no | no | no | no | none |

The safe-point commands also finish as a synchronization, keeping the pinned equality
`finishes == stops || resume` (`command.rs:719`): that predicate only decides how a
replay reads a finished row (`replay.rs:109`), and Task 2a pins it for these commands.
Other `command.rs` tests that name fixed sets are edited under E-9.

For all six, `was_accepted_without_operation`, `restores_index` and
`creates_its_repository` are no, and `stage` is the mutation stage of a remote
operation for the first three and of a local write for the policy verbs. Each is pinned
by a named test. Every lifecycle and poll row is `EndedWithoutCheckpoint::SaysNothing`
(fact 15), and `Prepared.position` is the primary branch, whose reachability a replay checks
(`evidence.rs:552-562`). The policy verbs' conformance home is `Replay`, since they need no server.

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

On the execute path and in a stored refusal, the same blockers are in the lifecycle's
`MutationDataDto`, since `Reply::envelope` discards the prepare-only field (`reply.rs:130-131`), and
`from_stored` restores them.

An unknown ID, wrong kind or unregistered repository answers as `item republish` does:
`item_not_found`, `repository_not_registered` and `repository_not_enabled`.

**Re-entry (the mechanism; finding: republish's does not fit, fact 29).** The binding
reads the operation's parent record through a new crate-private accessor on each domain
module: `lifecycle_parent(root, operation_id) -> Option<ParentView>` with `step`,
`expectation`, `item_id`, `cancel_requested`, `cleanup` and the recorded closure and
integration commits. A confirmed command's re-entry is always `confirm::reenter` (fact 36), so
`row` reads the accessor first, and `confirm.rs` gains `Row::Unfinished`, which needs no
journal row. `confirm_tests.rs` gets a named edit (E-15):

| State found on re-entry | What happens |
| --- | --- |
| No parent record | Crash between acceptance and record creation. `Row::Unrecorded`, then observe again: a digest match runs as a first call with `preview().expectation()`. A mismatch answers `external_change`. A preview now `Blocked` answers as Prepare. The mismatch and the `Blocked` answer delete the record. |
| Unfinished parent, any step | The new `Row::Unfinished`: `Entered::Unfinished`. The domain is called with the **record's** expectation and the same operation ID. No digest comparison: the request's own work moved the state. |
| Complete | `Row::Unfinished` too, and the domain returns its stored outcome, with no transport. |
| Refused and ended | `Row::Answers`: the stored refusal, `Final`. |
| Superseded | `Row::Answers`: `request_mismatch`, `Final`. |

**Standing after a call.** Never `Stopped{Absent}` while a parent is unfinished.

| Parent after the call | Standing | Next |
| --- | --- | --- |
| No record, refused before it began | `Stopped{Some(Absent)}`: record deleted, confirmation released, ID reusable (F2) | New prepare |
| Ended by a consent refusal | `Final`: the refusal is stored | New prepare |
| Unfinished, including cancelled, partial and `PrimaryAdvanced` | `Owed` (decision 8) | Same request; `operation resume` if no request holds it |
| Complete, index hand-off pending | `Owed` | Same request recomputes made-ready |
| Complete | `Final` | None |
| Superseded | `Final`, storing `request_mismatch` | The superseding request |

A stop with no parent record, whatever its code, is the first row: the request is
deleted, so its next step is a new prepare, and a same-ID resend after ten minutes answers
`confirmation_expired`. An `Owed` request's consent stands however old it is. The Next column
below is for `Owed`; both paths are tested past ten minutes.

`recorded_effects` branches early for these commands, before its `Absent` return, and reads the
parent record, so a stop with a written checkpoint and no journal row reports its effects (fact 40).

**Answers.** Exit status is derived from the code and effects (`results.rs:282-289`), so
this table states the code and the effects that decide it. "Durable" means the parent has
a written closure checkpoint or an integration, read from the accessor, or a child made a
durable effect.

| Domain | Code | Exit | Offered actions, in order |
| --- | --- | --- | --- |
| `Complete`, all done | `ok` | 0 | None |
| `Complete`, publication `Pending{NoPublicationRemote}`, cleanup done | `ok` | 0 | None. Publication pending (RFC exit table); `repo sync` after a remote exists |
| `AlreadyContained`/`AlreadyCurrent`, nothing left | `ok` | 0 | None |
| Publication `Pending{Synchronization(c)}` | `c` below | 4 | `request.retry`, then `c`'s |
| Publication or cleanup `Pending{Cancelled}` | `cancelled` | 130 | `request.retry` (decision 8) |
| Cleanup `Pending{ContextChanged, IgnoredFiles, RemoteContextChanged}` | `recovery_required` | 4 | `request.retry`, `repo.inspect`; a new prepare supersedes |
| Cleanup `Pending{AwaitingPublication}` | the publication's code | 4 | `request.retry` |
| Cleanup `Pending{RemoteDeletionFailed(c)}` | `c` below | 4 | `request.retry`, then `c`'s |
| Cleanup `Pending{RecoveryRequired}` | `recovery_required` | 4 | `request.retry`, `repo.inspect` |
| `IndexPending(outcome)`, nothing else pending | `discovery_pending` | 4 | `request.retry` |
| `Blocked{..}` at run | As Prepare | by class, or 4 if durable | As Prepare |
| `ReconfirmationRequired`, consent `ExternalChange` | `external_change` | 3, or 4 if durable | `request.prepare`; record ended |
| `PrimaryAdvanced` | `external_change` | 3, or 4 if durable | `request.retry`, `repo.inspect`: the parent stays unfinished (fact 41) |
| `IdentityRequired` | `identity_required` | 3, or 4 if durable | `request.retry` when `Owed`, `repo.identity_set`; see Identity |
| `ContextSynchronization(error)` | the child's code | by class | As `item sync`, and `request.retry` first when the parent stands. A child's `remote_branch_deleted` is pinned by a row, since the child, not the envelope, holds the operation |
| Refused closure checkpoint merge (a child's conflict, or the checkpoint's own) | `recovery_required` | 4 if durable, else 3 | `request.retry`, `repo.inspect`, with human lines naming `context observe-local`, `context sync-local`, `context show-local` and `context resolve-local`, and data naming the state. A recovery row and golden back it. |
| `Transport(error)` | the error's code, as `item sync` | by class, or 4 if durable | As `item sync` |
| `Busy` | `busy` | 5, or 4 if durable | `request.retry` |
| `PollYielding` | `poll_yielding` | 5, or 4 if durable | `request.retry` |
| `Interrupted`, cancellation asked | `cancelled` | 130 | `request.retry` (decision 8) |
| `Interrupted`, not asked | `recovery_required` | 3, or 4 if durable | `request.retry`, `repo.inspect` |
| `OperationMismatch` | `request_mismatch` | 2, or 4 after a replay's effects | None |
| `RecoveryRequired`, `Repository(error)` | `recovery_required`, or the error's code | by class, or 4 if durable | `repo.inspect`; `request.retry` first when `Owed` |

"By class" is `results.rs`'s: `unlock_required`, `key_rejected`, `host_*` and
`no_selected_key` are 3, input codes 2, transient codes 5 and `internal_error` 1. "Durable" is
read from `Step`, at or past the written checkpoint or integration, not from `integration_oid`
(fact 41).

Categories: synchronization `Conflict` is `recovery_required`, `Rejected` is
`push_rejected`, `Transport` is `transport_unavailable`, `Identity` is
`identity_required`, `Busy` is `busy`, `Yielding` is `poll_yielding` and
`RecoveryRequired` is `recovery_required`. Remote deletion `Transport` is
`transport_unavailable`, `Rejected` is `push_rejected`, `Configuration` is
`recovery_required`, `Key` is `no_selected_key`, `Unlock` is `unlock_required` and `Host`
is `host_approval_required`. The category carries no authority, so the host row's action
is `repo.inspect`, followed by `host observe` and `host approve`.

A repository error of kind `ExternalChange` stays a repository error (fact 11). Task 2b's
test is written first. C5 adds no `ResultCode` or `RecoveryActionKind`. Automatic bounded
retries (fact 33) apply to `Busy` or `Partial` with `request.retry` first, so count-based
assertions allow for them. For the host stop the binding suggests `request.retry`, then
`repo.inspect`, since a suggestion replaces the code's list (fact 38).

**Cache loss** (RFC:1031, Cycle 10 item 9). Request records and lifecycle records are both in the
registry, so losing it loses both. The same request answers `confirmation_not_found`. A new
prepare then reads Git: a written, unintegrated checkpoint gives an existing closure, an
integrated one gives cleanup-only, and `closed_at` and `closed_by` keep their first values.
Each is a row for both verbs where it applies, with the remote branch present and gone.

**Host and unlock.** Unattended runs pass `approval: None`. A host stop answers
`host_approval_required`; `host approve` works while the request stands; the same request
then continues. A protected key answers `unlock_required` non-interactively.

**Resume.** The route looks in the lifecycle tables first, since most unfinished parents
have no journal row (fact 29). There is no liveness check (runtime RFC). Whether a request
"holds" the operation is a stored fact: an accepted request record names it.

| Found | `operation resume` answers |
| --- | --- |
| Unfinished parent, an accepted request record holds it | `original_request_required`, through `left_to_its_request`, naming the request: that request continues it |
| Unfinished parent, no accepted request record | Continues with the record's expectation. Data is the same lifecycle DTO, made-ready included. Ctrl-C is as the verb's. |
| Parent running in another process | The domain's `busy` |
| Complete | `already_applied`, as F2 |
| Refused and ended, or superseded | `invalid_input`: nothing is left to continue |
| A poll an accepted request record holds | `original_request_required`, naming that request, through `left_to_its_request`, since C5's boundary now begins polls |
| A poll with no request record | `invalid_input`, unchanged. A kept reservation in this state is the known limit `01M4R19PC0T6ZPKM39X9DF26HX` |

A resume request that continues a parent and stops leaves no holding record, as a resume does today
(`bind/resume.rs:92`): the parent is the durable state, and a later resume continues it. A pending
**child** row's ID keeps its existing resume behaviour, pinned by a row. Resume data is tagged, so
`from_stored` tells it from a synchronization's.

Resume grants no new consent (RFC: "Resume never infers permission for a different
target/action").

**Cancellation (decisions 8 and 12).** Cancelling is not final for these verbs, unlike a
synchronization, and the RFC is amended. `LifecycleRequest` gains a crate-private probe field,
reached through `promote_document_observed` and `close_ticket_observed`, so public signatures are
unchanged. `cancelled::<L>` and `CancellationObserver` both read it, so it reaches a running child
at the child's own safe points. A push already under way completes, and effects report it.

- **The probe** reads the Ctrl-C token and the request's flag.
- **`cancel_request`** also calls the lifecycle's `request_cancellation`, and skips `finish_fenced`
  for these commands, whose parent decides standing. With no parent yet it reports `requested`, and
  the request's flag stops the run at its first safe point.
- **Acknowledging.** When a run answers `cancelled`, the binding takes the request's flag. A later
  resend therefore starts clean, while the domain clears its own flag (fact 30).
- **Automatic retries (decision 12).** A `request cancel` landing between automatic attempts isn't
  acknowledged yet, so attempt 2's probe stops at its first safe point with `cancelled`. This gives the
  decision's outcome with no separate check in the driver. A process killed before acknowledging answers
  `cancelled` once more on its next resend, and the resend after that continues.
- **`stops_on_ctrl_c`** gains both verbs. A press after the last safe point keeps the domain's answer.
  A second press or a kill leaves the record, and the same request continues.
- **`RecoveryActionKind::RequestCancel`'s doc** is corrected for these verbs.

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
amended to say so). Each state is exclusive, and read from `Step`:

| Case | `made_ready_state` | `made_ready` | Standing |
| --- | --- | --- | --- |
| Before `Step` reaches integration, including a checkpoint only | `not_computed` | empty | `Owed` |
| Integrated by this operation, the closure recorded or existing but newly on primary, with the read complete | `current` | the set | As the outcome's |
| The same, domain `IndexPending` | `index_pending` | empty | `Owed`: the retry reads again |
| The same, domain `Complete`, but the read fails or is incomplete | `index_unavailable_final` | empty | `Final` (decision 13). The state itself says it won't be recomputed; `ticket ready` is the live view |
| Cleanup-only or superseding run: `ClosureRecording::Existing` and already contained | `not_applicable` | empty | As the outcome's |

- A new function in `read/graph.rs`, `tickets_made_ready(repo, id)`, uses `read_tickets` and
  `readiness_state`. An index `behind` flag, usual while the context exists, does not change
  the answer: the context holds the closure.
- A partial run reports it too. Each retry evaluates again, and the request's final stored envelope
  holds the last evaluation, so a replay of a finished request returns it unchanged.
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

**Required outcomes.** Written from the gate's contract, and each is a test first in Task 10a or 10b.

| Verb | Case | Code | Exit | Next |
| --- | --- | --- | --- | --- |
| `observe-local`, `show-local` | Unknown item or operation | `item_not_found`, `operation_not_found` | 2 | None |
| `observe-local` | No context | Refused; no context is created (gate A4) | 2 or 3, as the read reports | `ticket save` or `document save` |
| `show-local` | A remote operation's ID | `invalid_input` | 2 | `conflict show` |
| `show-local --token` | Tampered, foreign or stale | `invalid_input` | 2 | None |
| `sync-local` | Equal or contained | `already_applied` | 0 | None |
| `sync-local` | Fast-forward or clean merge | `ok` | 0 | None |
| `sync-local` | Discovery pending after the merge | `discovery_pending` | 4 | Same request |
| `sync-local` | Malformed observation | `invalid_input` (`local_sync_refused`) | 2 | `context observe-local` |
| `sync-local` | Stale observation or wrong branch | `external_change` | 3 | `context observe-local` |
| `sync-local` | Dirty context | `worktree_not_clean` | 3 | Commit or save, then `context observe-local` |
| `sync-local` | Missing identity on a merge | `identity_required` | 3 | `repo.identity_set`, then the same request |
| `sync-local` | Unrelated histories, incompatible closures, or an unsupported conflict | `recovery_required`, refused with no owner or effect (gate 10, `local_sync/run.rs:155-158,192-195,214-221`) | 3 | External repair |
| `sync-local` | Eligible conflict | `recovery_required` | 4 | `context show-local`, `context resolve-local` |
| `sync-local` | Installed mixed conflict | `recovery_required`, external-only | 4 | External repair |
| `sync-local` | Another owner: busy, or a closure at `Integrating` or later | `busy` or `recovery_required` | 5 or 3 | As the code |
| `resolve-local` | Complete, valid set | `ok` | 0 | None |
| `resolve-local` | Incomplete, duplicate, unknown or invalid before any write | `invalid_input` | 2 | `context show-local` |
| `resolve-local` | Stale observation | `external_change` | 3 | `context show-local` |
| `resolve-local` | Partial accepted resolution | `original_request_required` | 4 | The same request and source |
| Both mutations | Cancelled at a safe point | `cancelled` | 130 | As the gate: effects kept, retry or abandon |

The golden files `local_sync_*` and the audit's list fix any code Task 0 finds different; a change
stops for a ruling.

## Polling Verbs

**`poll once`.** `--repo`, `--request-id`, `--json`. No input. Explicit invocation, which
skips eligibility (`remote/poll.rs:1241`), so it runs while paused or disabled; Cycle 08
item 2 and the runtime RFC settle this. Before calling the domain the binding refuses a
missing publication remote without writing, as republish does. Any other plan failure still
writes the domain's stored state (`poll.rs:1254-1283`), a known limit.

Data is `PollMutationDto`: operation ID, completion, completed time, next eligible time, and
per-target `{target, kind, item_id, disposition, before_oid, after_oid}`, each nullable as the
report is, for malformed rows. A slug is not added: a poll names refs and items as the domain
reports them (R-17).

Classify the authoritative report first, and index pending second, since `IndexPending` wraps any
completion (`poll.rs:1987-2000`). The domain finishes a poll's row on a transport stop, so such a stop
is final, and **a new request polls again** (fact 39, R-18). This table is the frozen contract after
three review rounds: Task 5b writes it test-first, and a row that fails stops at a design checkpoint.
Final transport codes offer nothing: `outcome.rs` gets a named rule for `poll once`, applied on replay
too, because `remote_unavailable` and `transport_unavailable` otherwise always offer a retry that would
only replay the stored failure.

| Domain | Code | Exit | Standing and next |
| --- | --- | --- | --- |
| `Polled`, `Completed`, category `Completed` | `ok` | 0 | `Final`. Dirty, diverged, conflicted, malformed, deleted and incomplete targets are data, with guidance, not failures (R-16) |
| The same, wrapped in `IndexPending` | `discovery_pending` | 4 | `Final`; `index.refresh` |
| No remote | `publication_remote_required` | 3 | Deleted; nothing written |
| `Polled`, `Cancelled` | `cancelled` | 130 | `Final`; a new request polls again. Polls follow the RFC's final cancellation, since a new ID is not blocked |
| `Polled`, `Interrupted`, or `Err(Interrupted)` | `recovery_required` | 3, or 4 if a target moved | `Owed`; the same request continues with `restart` |
| Stored or returned category `ConfigurationRequired` | `publication_remote_required` | 3 | `Final`; new request |
| `SelectedKeyUnavailable` | `no_selected_key` | 3 | `Final`; `key select`, new request |
| `UnlockRequired` | `unlock_required` | 3 | `Final`; new request at a terminal |
| `HostApprovalRequired` | `host_approval_required` | 3 | `Final`; `host approve`, new request |
| `TransportUnavailable` | `transport_unavailable` | 5 | `Final`; new request |
| `ProtocolRejected` | `remote_unavailable` | 5 | `Final`; new request |
| `RepositoryUnavailable` | `repository_inaccessible` | 3 | `Final`; new request |
| `Err(Transport(error))`, no row after the call | the error's code, by `outcome.rs:109-155` | by class | Deleted; new request |
| `Err(Transport(error))`, a row after the call (the common first-call failure) | the error's code. A later re-entry reads the stored category's code above, which can name the same stop differently (`key_rejected` against `no_selected_key`), and a golden pins each | by class | `Final`; new request |
| `Err(Repository(error))` | the error's code | by class | By the journal row |
| `Busy` | `busy` | 5 | `request.retry` |
| `PollYielding` | `poll_yielding` | 5 | `request.retry` |
| `ConflictPending` | `recovery_required` | 3 | `repo.inspect` |
| Pending local sync | the domain's answer, pinned | 3 | `repo.inspect` |

A restart reports what its own pass saw. A target an interrupted first pass materialized reads
`current`, so the final envelope understates it; Git holds exactly one worktree (known limit).
A killed poll keeps its reservation, and the same request continues it. If the ID is lost,
`operation list` names the operation and `operation resume` names the request. Ctrl-C reaches it
through a probe, as for the lifecycle, added with its caller in Task 5b. A stale remote context for a
closed ticket gets its disposition from the domain, and a case pins it.

**Policy verbs.**

| Verb | Effect | Already so |
| --- | --- | --- |
| `poll configure` (`--input {interval_seconds}`, 60–3600, closed schema) | Sets the interval, leaving the stored due time | `already_applied`, 0 |
| `poll pause` | Sets `paused` | `already_applied`, 0 |
| `poll resume` | Clears `paused` and clears `next_eligible_at`, including a backoff deadline, so an eligible poll is due (decision 11, runtime RFC). The stored doubling state stays | Not paused: `already_applied`, 0, nothing written |

- `poll resume` leaves `recovery_suspended`, `enabled` and the stored backoff value alone, and its human
  output names a suspension when one is set (decision 5).
- No verb sets `enabled`.
- An out-of-range interval is `invalid_input`, and nothing is written.
- Pausing during a running poll doesn't stop that poll; it applies to the next automatic one.
- An unregistered repository is `repository_not_registered`. A registered one always has its
  polling row (Task 0 confirms); pause survives a remote's removal and re-adding (RFC).

- An unenabled repository is `repository_not_enabled`, and an unavailable index
  `index_unavailable`. A pending local sync does not refuse a policy verb: no Git work happens.

**Atomic update, in one transaction with the request (R-7, decision 15).** A new
`update_remote_polling(transaction, change)` writes one field, and the request's record is inserted and
finished in the **same transaction**, through a new boundary path in `execute.rs` under
`cache_write_guard`, with `settle.rs:136`'s "the one writer" comment amended. Both tables are in the
registry (`records.rs:802-808`, `remote/state.rs:6402-6412`). A request is therefore either absent or
finished, with no accepted state.

**What this gives is last commit wins.** A committed request replays, so a resend after a lost reply
changes nothing and a later pause stands. A request that never committed can't be told from one never
sent, so resending it is a new write: `poll resume` R1 stops before commit, `poll pause` R2 commits, R1
resent clears the pause. This is deliberate, not silent: the user sent `poll resume`, and its answer says
so. Only an observation token, which decision 15 declined, would refuse it. (Owner question 6.) "Nothing
written" below means the policy row: a finished request's record is still stored. `set_remote_polling` has test callers only, and stays for them. D6 uses the boundary's bindings.
The poll's own completion writes are field-scoped (`record_poll_attempt`, `state.rs:2389`); Task 0 confirms
`state::configure` is.

**No worker.** No thread, no child, no service. Transport starts only for `poll once`.

## Identity

Decision 6 and `01M5MK57`. Both verbs pass `confirmed_identity: None`.

| Stage | Case | Answer |
| --- | --- | --- |
| Closure prepare | `IdentityRequired` the only blocker | `identity_required`, `repo.identity_set`. No request exists: a **new prepare** follows. |
| Promotion prepare | A merge commit is planned, and nothing else blocks | The same, through a read-only selector check, made last (R-3) |
| Promotion confirm | No usable identity, and a merge commit planned | The same check in the domain, only with no prior record (`prior.is_none()`), after `L::revalidate` gives an unchanged preview, through a `Lifecycle` method that is a no-op for closure: `identity_required`, no record, nothing changed (R-3); a new prepare. A resend of a finished request still gets its stored outcome |
| Either prepare | Already contained, or existing closure | No identity needed |
| Closure confirm | The identity changed since prepare | The digest binds `closed_by`, so `external_change`, record ended; a new prepare |
| Promotion | Identity lost after the record began | `identity_required`, `Owed`; after `repo identity-set` the same request continues. Only a change during the run reaches this |
| Closure confirm | Identity removed or unusable | Refused through `refuse_record`, which writes a terminal row: `identity_required`; a new prepare. A terminal refused row is no recovery obligation, so `01M5MK57`'s "journal unchanged" is read as no unfinished row for closure (R-21, recorded on that ticket) |
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
ticket's acceptance governs, in full: a library test as well as the binary path; other index entries
unchanged; a retry and a replay of `remote select` leave the index current; and recovery after a stop
between the commit and the index update, which Task 0 locates and may widen the scope's named function.
Re-entry to that function's already-selected branch (`src/repository.rs:4500`) updates the index too, and one `FailurePoint` variant stages the stop (decision 17). Its "merged on its own" bullet is overridden by decision 10, recorded on that ticket. C4's fixture seed
stays.

## Recovery Tables (decision 4)

`docs/RFC/cli-contract.md` gains one table per family: authoring, repository and credentials,
collaboration, lifecycle, local sync, polling. Each row is a verb, a recoverable code, the `next:`
action and the exit class, and names a golden. `tests/cli/recovery_tables.rs` parses the tables and
requires each golden to exist and hold that code and action. For C5's verbs every recoverable code in
their outcome tables needs a row. Earlier verbs are checked against their audit code lists, and a code
no fixture reaches carries a reason, as in the exit table (R-10).

## Inventory And Evidence

**Inventory (R-9).**

- `tests/cli/inventory.rs`: `bound_commands()` equals the catalog's available mutating verbs, less
  the exempt `context prepare` and `context release` (open question 5). Every help-root verb appears in the RFC taxonomy or, for the relationship
  verbs, the relationships RFC's table, and the reverse. No verb is `Planned`. `<verb> --help --json`
  answers for every verb, with its catalog options. Every input-taking verb has exactly one input schema
  and example; every other verb is listed as no-input. Every verb's data type has an output schema.
- Exit behaviour and recovery are the exit table's and the recovery tables' tests.

The Cycle's "help golden" is this generated assertion for every verb, with goldens for C5's verbs.

**Planned mechanism (R-8).** `Availability::Planned` and `planned()` become `#[cfg(test)]` or
`cfg_attr(not(test), allow(dead_code))`, as clippy needs, in Task 9, when the last planned verb goes,
exercised by a test-only spec.
`command_unavailable` stays a CLI-only code, with an unreachable reason. RFC:770 is amended.

**Per-verb evidence (track rule 7; RFC "every command's human/JSON result, input failure, no-op and
applicable recovery/exit class").** Every cell holds, or carries its reason.

| Verb | Human + JSON | Input failure | No-op | Prepare/confirm/interactive | Ctrl-C | `request cancel` | Replay | Mismatch | Same-request continuation | Lost stdout |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `document promote` | 7 | 7 | 7 | 7 (prepare, confirm), 13a (PTY consent) | 13a | 13a | 12a | 7 | 12a | 13a |
| `ticket close` | 7 | 7 | 7 | 7 (prepare, confirm), 13a (PTY consent) | 13a | 13a | 12a | 7 | 12a | 13a |
| `operation resume` of a lifecycle | 12a | 12a | 12a | none | 13a | none: resume has no safe point of its own request | 12a | 12a | 12a | none |
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
only on their own branches, are copied byte for byte to this branch, the closure fields are added in the
same final commit, under the identity Task 0 settles, and each gets a comment. With the merge's go-ahead,
after confirming that they hold only their ticket files, both remote branches are deleted (decision 14), so
no open copy lingers. A case pins what `poll once` and reads show for such a context.

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
| `mutation_remote` | Lifecycle, resume and polling tables, made-ready |
| `mutation_contract`, `read_contract` | Goldens and schemas |
| `document_promotion`, `ticket_closure`, `remote_synchronization` | Identity regressions |
| `repository_enablement` | `remote select` index |
| `cli` | Grammar, help, inventory, recovery tables, local-only journeys, context verbs |
| `cli_remote` | Two-clone journeys, failures, Ctrl-C, no-worker |

Application data is scanned in Task 7 for sentinel `closed_by` and title values after a closure, across
the request, confirmation and lifecycle tables. A confirmation stores a digest, previews are not persisted,
and request records hold neither (Task 0 confirms). The index's `closed_by` column is permitted by Cycle 10.

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
| E-9 | `command.rs` tests | New names in `the_published_names_are_the_ones_records_hold`; the fixed sets in `:660-667` (republish as the one stopping confirmed command), `:735`, `:760`, `:812` |
| E-10 | `tests/mutation_replay/resume.rs:1186` | The poll pin gains an accepted-record case; no promotion or closure pin exists, and Task 0 confirms |
| E-11 | `tests/mutation_contract.rs` registered cases, `tests/support/mutation_golden.rs` | Additive registration |
| E-12 | `outcome_tests.rs` over `Command::ALL` | New commands' rows |
| E-13 | `tests/support/conformance.rs` `HOMES`, `tests/mutation_contract.rs:1510` `DATA_ROOTS`, `tests/support/confirmation.rs:60` `preview_schema` | Length and entries grow; each command gets its home, conformance row and golden in the task that binds it |
| E-14 | `src/bin/manyhands-cli/collaboration_tests.rs:82` | New `stops_on_ctrl_c` rows |
| E-15 | `src/repository/mutation/confirm_tests.rs` | `Row::Unfinished` cases |
| E-16 | `src/bin/manyhands-cli/startup_tests.rs:171`, `parse_tests.rs:563` | Move to the test-only planned spec (decision 16), done with E-3 and E-4 in Task 9 |

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
  - `:162`, `:972-983`, `:993-998`: resume, the Ctrl-C window, `request cancel` lists and lifecycle cancellation (decision 8);
  - `:1139-1142`: the fixed known limit (decision 10);
  - new known limits.
- **Git workflow RFC `:411`:** lifecycle cancellation (decision 8).
- **Wave:** track rules 2, 5 and 6 (decision 1; D5 and D6 wait for C5's merge, recorded as C4's D4 ordering was); C5's scope line for the two read verbs and two folded tickets; the "merge prerequisite remains in force" line marked historical; capability table; D6's section for decision 5; the API-audit timing departure (open question 4). `:351`'s "rebase" noted as superseded by AGENTS.md; `:658`'s "closure remains canonical until C5".
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
| R-3 | Promotion's missing identity is checked at prepare, and by the domain before `begin_record`, only when a merge commit is planned and last, leaving the stored mask untouched. | Two check sites could drift; the runner remains the guard. |
| R-4 | Made-ready is direct dependents, evaluated at report time, with the four states. | The list can differ from a historical diff; the RFC says so. |
| R-5 | An unusable confirmed identity keeps `ExternalChange`. | Library wording only. |
| R-6 | `effective_identity`'s cascade is unchanged. **Owner question 1.** | The two selectors can disagree on a split configuration. |
| R-7 | Policy verbs update one field in the request's own transaction, with no observation token (decision 15). | A boundary variant for three commands. |
| R-8 | `command_unavailable` remains a code with an unreachable reason. | Removal later is simple. |
| R-9 | The inventory parses the RFC tables. | Format coupling; a failing test names it. |
| R-10 | Recovery rows are checked against goldens by parsing RFC tables. | Same. |
| R-11 | No new test target. | Slower existing targets. |
| R-12 | Real-repository evidence uses a disposable clone with fresh data. | A defect specific to the owner's registry would not show. |
| R-13 | Withdrawn: `poll once` under a pause is settled by Cycle 08 item 2 and the runtime RFC. | None. |
| R-14 | Withdrawn: decision 10 replaces it. | None. |
| R-15 | `context show-local --token` is JSON only. **Owner question 3.** | Humans read sides through `--json`. |
| R-17 | Poll targets carry no slug. | Slug beside ULID applies where a ticket is named as a ticket; add later additively. |
| R-18 | A poll's stop after reservation is final, and a new request polls, because the domain finishes its row. | Differs from a synchronization's continuation; the human `next:` says so. |
| R-19 | `Row::Unfinished` is added to `confirm.rs`, since the existing rows cannot express a parent with no journal row. | A variant for two commands. |
| R-20 | Decision 12's outcome comes from acknowledging the flag on a `cancelled` answer, not from a driver check. | A killed process's resend answers `cancelled` once more. |
| R-21 | Closure's refused identity leaves a terminal row, read as no recovery obligation under `01M5MK57`. | The ticket's literal "journal unchanged" is narrowed for closure. |
| R-16 | A completed poll with preserved or marked targets is `ok`, exit 0, with dispositions in data. The RFC says once is explicit polling, not synchronization. | A script checking only the exit misses a blocked target; human output names them. |

## Known Limits

- Closes record the configured identity (decision 6).
- A closure race may have no CLI way out until `01M4H33R34Z7C7EEKTY1ZCT950` lands.
- `poll once` holds the remote reservation, so it answers `busy` beside another remote operation, and they answer it.
- A CLI-only user sees `recovery_suspended` without a verb to clear it (decision 5).
- A protected key answers `unlock_required` non-interactively, so agents' runs need a terminal until `01M67Y754SJ1FTYQJFR37Z4ZTY` lands.
- A retried or restarted report reflects its own pass (made-ready, and poll targets). A superseding request reports `not_applicable`, and the set stays in the old request's partial envelope.
- A killed poll with a lost request ID is found through `operation list`.
- Policy verbs are last commit wins: resending a request that never committed writes again (decision 15).
- A poll plan failure other than a missing remote writes the domain's stored state.
- A promotion blocked by a pending local sync issues a confirmation before refusing.

## Alternatives Rejected

- **A separate library ticket:** decision 1.
- **Caller-chosen `closed_by`:** decision 6.
- **Extending `conflict show` to local syncs:** decision 3.
- **Recovery examples in `--help`:** decision 4.
- **A before/after readiness diff:** no case needs it (R-4).
- **Observation-guarded policy writes:** decision 15.
- **Closing C5 with `ticket close`:** decision 9.

## Risks

| Risk | Control |
| --- | --- |
| Re-entry mechanism meets a state the table lacks | Task 2b's table test comes first, and a failed row stops for a ruling. |
| Counted-list conflicts with D-track merges | Additive; merge main at each Part. |
| `mutation_remote` already runs near an hour | Filters in tasks; full suite once. |
| Unfinished local syncs fence C4 verbs more often | Row in each outcome table, and the local-recovery journey. |
| Full suite exceeds its budget | Detached run with no tool timeout; C4 took 3 h 34 min. |
