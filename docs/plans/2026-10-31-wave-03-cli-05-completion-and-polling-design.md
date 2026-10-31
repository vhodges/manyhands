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
a `ticket save`. Decisions are the Cycle's. Rulings R-n are planning's, each
with its cost if wrong.

## Baseline Facts

At `717b664d` (main `a52273ff` merged). Task 0 re-checks each one.

**Commands and catalog**

1. `Command` (`src/repository/mutation/command.rs:37-77`) holds 37 commands and
   none for promotion, closure or polling. Every predicate is an exhaustive
   `match` with no wildcard arm: `has_operation`:92,
   `was_accepted_without_operation`:178, `restores_index`:222,
   `creates_its_repository`:267, `stops_at_safe_points`:318,
   `finishes_as_a_synchronization`:383, `stage`:429,
   `hands_off_to_any_refresh`:486, `needs_confirmation`:537.
2. `LocalContextSync` and `LocalContextResolve` exist as `context sync-local`
   and `context resolve-local` (`command.rs:68-69`) and are in no CLI catalog.
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
    fills it, while `polling_status` does. Task 8 corrects the comment.

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
    dependents with a state on each line. `is_ready` (`:263`) is "open, and every
    dependency a closed ticket". Readiness reads come from the index.

**Test support**

26. `CliPair` (`tests/support/cli_pair.rs`), mounted only from `tests/cli_remote.rs`.
    `SshRemoteFixture` offers `reject_primary_updates` and `reject_context_deletion`
    (`tests/support/ssh_remote.rs:474,484`). `tests/cli_remote.rs` is
    `harness = false`, and each case is registered in its module's `CASES`.
27. `tests/cli/collaboration_exits.rs` accounts for the audit's codes by set
    equality; `tests/cli/exits.rs:18` lists `command_unavailable` as CLI-only.
28. C4's two-clone limits: select keys before enabling (`01M66HFV44`), and
    `remote select` leaves the Git index stale (`01M669FJR2`).

## Module Layout

**Library**

- Create `src/repository/mutation/bind/lifecycle.rs`: the shared confirmed binding
  for both verbs, generic over a small `LifecycleVerb` trait with a promotion and
  a closure implementation. Promotion and closure already share one runner, so
  they share one binding (R-1).
- Create `src/repository/mutation/bind/poll.rs`: `poll once` and the three policy
  bindings.
- Modify `command.rs`, `mutation/mod.rs` (`Mutation`), `bind/mod.rs`,
  `bind/resume.rs`, `dto.rs`, `outcome.rs`, `remote/state.rs` (atomic update),
  `remote/sync.rs` (identity), `schemas/v1/`.
- Modify `read/graph.rs` only to expose a narrow made-ready read (R-4).

**CLI**

- Create `src/bin/manyhands-cli/lifecycle.rs`, `polling.rs`, `local_sync.rs`, each
  with `_tests.rs`, and `human_lifecycle.rs`, in C1's `#[path]` pattern.
- Modify `catalog.rs`, `dispatch.rs`, `render.rs`, `human.rs`.
- `schemas/input/v1/poll_configure.schema.json`, `context_resolve_local.schema.json`.

## Command Predicates

| Command | `needs_confirmation` | `has_operation` | safe points | finishes as sync | family |
| --- | --- | --- | --- | --- | --- |
| `document promote` | yes | yes | yes | no | Remote |
| `ticket close` | yes | yes | yes | no | Remote |
| `poll once` | no | yes | yes | no | Remote |
| `poll configure`, `poll pause`, `poll resume` | no | no | no | no | none |

Their remaining predicates are decided case by case in Task 1, each with a named
test in `command.rs`. Every lifecycle row is `EndedWithoutCheckpoint::SaysNothing`,
because the remote journal records phases and no checkpoint step (fact 15).

## `document promote` And `ticket close`

**Grammar.** `--repo`, `--id`, `--prepare`, `--confirm <id>`, `--request-id`,
`--json`. No `--input`: both take no input (RFC, "CLI callers save drafts first").

**Prepare.** Calls the preview. `Blocked` answers `recovery_required`, with the
blockers named in data, and nothing is written. A closure on an
`AlreadyClosed` blocker is `ticket_closed`. `Ready` returns a
`PreviewDetailsDto::Lifecycle` holding: kind, item ID, branch, primary branch,
remote, context and primary commits, whole-branch path changes, planned
integration, publication, cleanup, and for closure the planned `closed_by` and
whether it is `record` or `existing`. No title, body or content. `closed_by`
appears only in this deliberate preview, never in a diagnostic.

**Observation.** The digest binds the expectation's bytes. Task 2 stores what the
binding needs in order to rebuild `expected` on re-entry. It reads the expectation
from the operation's own record when one exists. If no accessor exposes it, a
`pub(in crate::repository)` accessor is added. It never parses the digest from
the stored observation text.

**Accept and run.** Under F2's rules, consent checks in this order:
`confirmation_not_found`, `confirmation_mismatch`, `confirmation_used`,
`confirmation_expired`. Then `observe_again` compares digests and answers
`external_change` before anything starts.

**Answers.** Domain results and errors map as follows.

| Domain | Code | Exit | Effects and data |
| --- | --- | --- | --- |
| `Complete`, merged, published, cleaned up | `ok` | 0 | Integration, publication and cleanup complete |
| `Complete`, publication `Pending{NoPublicationRemote}` | `ok` | 0 | Publication pending, local-only success (RFC exit table) |
| `Complete`, `AlreadyContained`/`AlreadyCurrent`, nothing left to do | `ok` | 0 | Effects report what is current |
| Publication `Pending{Synchronization(category)}` | that category's code | 4 | Integration done; cleanup `AwaitingPublication`; `request.retry` |
| Cleanup `Pending{reason}` | `recovery_required` | 4 | Reason in data; `request.retry` |
| `IndexPending(outcome)` | the outcome's code | 4 | Discovery pending; `index.refresh` |
| `Blocked{..}` | `recovery_required` | 3 | Blockers |
| `ExternalChange`, `ReconfirmationRequired`, `PrimaryAdvanced` | `external_change` | 3 | `request.prepare`, a fresh preview |
| `IdentityRequired` | `identity_required` | 3 | `repo.identity_set`, then the same request |
| `ContextSynchronization(error)` | the child's code | per class | Host, unlock and transport answers as `item sync` |
| `Busy`, `PollYielding` | `busy` | 5 | `request.retry` |
| `Interrupted` | `cancelled` when cancellation was asked; otherwise `recovery_required` | 130 or 4 | `request.retry` |
| `OperationMismatch` | `request_mismatch` | 2 | None |
| `RecoveryRequired`, or `Repository(error)` | `recovery_required`, or the error's code | 4 once a checkpoint may exist | Effects from stored evidence (fact 11) |

The table is the outcome table for Task 2. Its test is written first. A row that
cannot answer as written stops for a ruling. Task 0 checks each code against
`ResultCode`; C5 adds no `ResultCode`.

**Standing.** An unfinished record keeps its request live, so the same request
continues the work with `restart`. A consent refusal that ended a record is
final: a new prepare goes on.

**Resume.** `operation resume --operation-id` on an unfinished promotion or closure
continues it with the record's own expectation, item and action. The route is split
out of `Named::NotResumable`, which keeps polls. A record that answers a mismatch of
action or item is `invalid_input`. Resume grants no new consent: it can only finish
what was accepted (RFC: "Resume never infers permission for a different
target/action"). Where a request still holds the operation, resume uses the existing
`left_to_its_request` answer, as for a synchronization.

**Cancellation.** `request cancel` and Ctrl-C reach the operation through the
request's flag and `cancel_remote_operation`. The lifecycle's own safe points read it.
Task 7 records each answer, including a press after the last safe point (C4's
`push_rejected` lesson), which keeps the domain's answer rather than `cancelled`.

**Identity.** Both pass `confirmed_identity: None` (decision 6). Because a
promotion preview has no identity blocker and its stored mask must not change, the
promotion binding checks the selector during `prepare` and answers
`identity_required` there (R-3).

**R-38 interaction.** A pending local sync blocks closure as
`LocalOperationPending` at prepare and answers `recovery_required`. For promotion,
the domain refuses at start (fact 24), and the binding answers the same. A row in
each table pins this.

## Made-Ready Report

Closing T can only make direct dependents of T ready, and none of them was ready
before, because T was open. So no before-state is needed (R-4):

```text
made_ready = direct dependents of T, in ID order,
             that the index reads as open with every dependency closed
```

- Computed by the binding after the domain returns a recorded closure, and stored
  in `TicketClosureMutationDto` with the request, so a replay returns it unchanged.
- `made_ready_state` is `current`, or `index_pending` on `IndexPending` with an empty
  list. The recovery is `index.refresh` followed by `ticket ready`. A pending index
  never produces a guess.
- A retry that finishes an unfinished closure computes once, on the run that returns
  the final outcome.
- `PlannedClosure::Existing`, a recorded closure being finished, reports the same way.
- Each entry is `{id, slug, title}`. Title is already public in reads.
- A read failure after the closure does not fail the closure: it reports
  `index_pending`.

## Local-Sync Verbs

| Verb | Kind | Selects | Output |
| --- | --- | --- | --- |
| `context observe-local` | read | `--repo`, `--id` (kind resolved from the item) | `local_context_sync_observation` |
| `context sync-local` | unconfirmed mutation | `--repo`, `--id`, `--observation` | `local_context_sync_mutation` |
| `context show-local` | read | `--repo`, `--operation-id`, optional `--token` | inspection, or a token's content |
| `context resolve-local` | unconfirmed mutation | `--repo`, `--operation-id`, `--input` | `local_context_sync_mutation` |

- `--token` with `show-local` returns `{base, local, incoming, historical}` from the
  gate's content read. Tokens are opaque and tamper-checked by the library (gate A10).
- The input is the gate's: `{observation, resolutions: [{token, source}]}`. Source
  stays out of request records, as the library already guarantees.
- Bin code carries no merge logic. It adapts arguments and renders.
- `operation abandon` keeps routing a local sync to the gate's guarded abandonment.
  Task 0 confirms this answer through the binary; the RFC already states it.

## Polling Verbs

- **`poll once`.** `--repo`, `--request-id`, `--json`. No input. Explicit invocation.
  `restart` on re-entry. Data is `PollMutationDto`: operation ID, completion, completed
  time, next eligible time and per-target `{target, kind, item_id, disposition,
  before_oid, after_oid}`. A `Paused` or `Disabled` policy does not refuse it, because
  explicit polls skip eligibility (fact 18). `RemotePollError::ConflictPending` answers
  `recovery_required`.
- **`poll configure`.** `--input {interval_seconds}`, 60–3600, closed schema. A value
  out of range is `invalid_input`, and nothing is written.
- **`poll pause`, `poll resume`.** No input. Set `paused` to true or false. Already in
  that state is `already_applied`, exit 0. `poll resume` leaves `recovery_suspended`,
  `enabled` and the backoff alone, and its human output names a suspension when one is
  set, so the flag isn't silently left unexplained.
- **Atomic update (R-7).** A new crate-private `update_remote_polling(root, change)`
  reads and writes one field in `with_transaction`. `set_remote_polling` stays for its
  existing callers. A second actor's concurrent change of another field survives, and a
  test pins it.
- **No worker.** No thread, no child, no service. Transport starts only for `poll once`.

## Identity Fix (`01M5MK57JKCSQFXBNZ8DBE548K`)

- Reproduce first, with a failing test per path.
- Apply `commits_unchanged` to effective and confirmed identities in
  `select_committing_identity`, before any journal effect, configuration write or
  commit.
- An unusable configured identity counts as none: `IdentityRequired`, with the
  observed digest.
- An unusable confirmed identity keeps the existing refusal for an invalid
  confirmation, `ExternalChange`, and nothing is persisted (R-5). No CLI verb reaches
  this path, so only library tests do.
- `effective_identity`'s per-key cascade stays as it is (R-6). The ticket requires the
  owner's approval to change it, and the approval message asks.
- Existing recovery records and replays are preserved.

## Recovery Tables (decision 4)

`docs/RFC/cli-contract.md` gains one table per family: authoring, repository and
credentials, collaboration, lifecycle, local sync, polling. Each row is a recoverable
code, the verb's `next:` action and the exit class, and names a golden. A test in
`tests/cli/recovery_tables.rs` parses the tables, and requires every named golden to
exist and hold that code and action. It also requires every `RecoveryActionKind` a
verb can offer to appear in some row (R-10).

## Inventory Test

`tests/cli/inventory.rs` asserts (R-9):

- every catalog verb appears in the RFC taxonomy, and the reverse;
- every `Command` name is an available catalog verb;
- no verb is `Planned`;
- every verb has a help golden or a generated help assertion, and every input-taking
  verb exactly one input schema.

`Availability::Planned` and `Action::Unavailable` remain as the mechanism, exercised
by a unit test with a test-only spec. `command_unavailable` stays a CLI-only code with
a reason in the exit table: the binary can no longer reach it (R-8).

## Journeys

Each is a `c5_journey_*` case over `CliPair`, asserting Git objects and canonical
content, and scanned for secrets.

| PRD journey | Counterpart | Variants |
| --- | --- | --- |
| Offline edit, checkpoint and later sync | Save with the remote unreachable, then `item sync` | Remote refusal gives recovery with local work kept |
| Conflict recovery | C4's `c4_conflicts_*` case, referenced by a thin journey that resolves and retries | None new |
| Background update and discovery | Peer publishes primary and a new item; `poll once`; discovery shows one worktree and indexed comments | Dirty, divergent, malformed, remotely deleted |
| Threaded discussion | Roots and replies, `index rebuild`, a worktree change, `comment list` | None new |
| Document promotion | Save, `--prepare`, `--confirm`; a fresh `document save` starts a new context | Local-only with later `repo sync` |
| Close with retryable failure | Save, `--prepare`, `--confirm`; `ticket ready` matches the report | Local-only; refused push; refused deletion |

"Final save" differs by design: CLI callers save first (RFC), and the journey says so.

## Dogfooding (R-12)

Task 0 runs read and `--prepare` forms against this repository with the branch-built
CLI: `poll status`, `context observe-local`, `document promote --prepare` and
`ticket close --prepare`. A prepare writes a confirmation row in application data, so
Task 0 first copies the application data directory and points `MANYHANDS_DATA_DIR` at the copy if
the harness allows. Otherwise the owner is asked. Execution runs on a disposable clone.
A real `ticket close` of this Cycle's ticket is the owner's go-ahead at the final
task, and needs a terminal if the key is protected, as C4's switch did.

## Exit Statuses

Follow the RFC table. Local-only completion is 0. Partial after any durable effect
is 4. Blocked before any is 3. `busy` is 5. Ctrl-C is 130, or the domain's answer after
the last safe point. `tests/cli/collaboration_exits.rs`'s set equality is extended with
C5's codes in Task 13.

## Testing

No new test target (R-11). Cases extend existing targets, named `c5_<module>_<case>`
in `harness = false` targets, and split per row under the 100-second watchdog.

| Target | Adds |
| --- | --- |
| `--lib` | Command predicates, identity, `update_remote_polling` |
| `mutation_remote` | Lifecycle and polling outcome tables, resume routes, made-ready |
| `mutation_contract`, `read_contract` | Goldens and schemas |
| `document_promotion`, `ticket_closure`, `remote_synchronization` | Identity regressions |
| `cli` | Grammar, help, inventory, recovery tables, local-only journeys |
| `cli_remote` | Two-clone journeys, failures, Ctrl-C, no-worker |

Test isolation stays as in C4: every child goes through the CLI support, and the SSH
harness keeps system identity out.

## Named Edits To Pre-Existing Tests

| ID | Test | Edit |
| --- | --- | --- |
| E-1 | `src/bin/manyhands-cli/catalog_tests.rs` `DESIGN` | Six verbs become available with C5's; add four context verbs |
| E-2 | `tests/cli/help.rs:277-304` | Replace with a test that `ticket close` reaches its handler; remove `unavailable_ticket_close.json` and its registration |
| E-3 | `src/bin/manyhands-cli/parse_tests.rs:216,235` | Move to a test-only planned spec |
| E-4 | `src/bin/manyhands-cli/startup_tests.rs:332,371,440-442` | Same |
| E-5 | `tests/cli/boundaries.rs:358` | `ticket close` moves off `command_unavailable`, to its real pre-dispatch answer |
| E-6 | `tests/cli/exits.rs:18,143` | `command_unavailable` gains its unreachable reason |
| E-7 | `tests/fixtures/cli_v1/help_*` | Help goldens regenerate, read in the diff |
| E-8 | `tests/cli/collaboration_exits.rs` | Code set grows |
| E-9 | `command.rs` tests | New names; extend `the_published_names_are_the_ones_records_hold` |
| E-10 | `bind/resume.rs` tests and any `mutation_remote` case that pins promotion or closure as `invalid_input` | Now a continuation; polls unchanged |
| E-11 | `tests/mutation_contract.rs` registered cases, `tests/support/mutation_golden.rs` | Additive registration |
| E-12 | `outcome_tests.rs` over `Command::ALL` | New commands' rows |

Task 0 greps for other pins (`NotResumable`, `C5`, `Planned`, `arrives with`). Any
unlisted hit stops for owner review. This list is not yet proven complete.

## Document Amendments

- **CLI contract RFC:** taxonomy rows and grammar for the four context verbs; lifecycle,
  poll and made-ready shapes; recovery tables; removal of "C5 remains cataloged as
  planned"; known limits.
- **Wave:** track rule 5 and 6 exception (decision 1); C5 scope line for the two read
  verbs; the "merge prerequisite remains in force" line marked historical; capability
  table.
- **Gate Cycle:** "C4/D5 consume" becomes C5/D5.
- **API audit:** C5 rows reconciled, and the unamended sentence near `:190`.
- **Ticket relationships RFC:** the made-ready definition.
- **AGENTS.md, `file-ticket`:** closure text points to `ticket close`. The
  `working-on-a-ticket` exception stays.
- **C5 ticket:** "rebased" wording corrected and the local verbs added, by commit on the
  branch, since `ticket save` is separately gated.
- **`01M5MK57` ticket:** closure note, with the owner's go-ahead at closure.
- **D5 ticket:** note that C5 delivered the bindings, with the owner's go-ahead.

## Rulings

| ID | Ruling | Cost if wrong |
| --- | --- | --- |
| R-1 | One generic lifecycle binding for both verbs, matching the domain's one runner. | Generic churn; split in Task 2 with no contract change. |
| R-2 | Both verbs take no `--input`. | Adding later is additive. |
| R-3 | Promotion's missing identity is checked by the binding at prepare, not by a new blocker, leaving the stored mask untouched. | A second check site could drift from the runner's; the runner's answer remains the guard. |
| R-4 | Made-ready is direct dependents now ready, needing no before-state, computed in the binding. | A misread of "made ready" would change the list; the RFC gets the definition. |
| R-5 | An unusable confirmed identity keeps `ExternalChange`. | Library-only wording; no CLI path reaches it. |
| R-6 | `effective_identity`'s per-key cascade is unchanged. Owner question at approval. | The two selectors can still disagree on a split configuration. |
| R-7 | Policy verbs do a single-field atomic update with no observation token, replacing the outline's observation guard. RFC: "no observation token". | A stale user intent can still win, but only on its own field. |
| R-8 | `command_unavailable` remains a code with an unreachable reason, and the mechanism stays under a unit test. | Removal later is simple. |
| R-9 | The inventory test parses the RFC taxonomy table. | Format coupling; a failing test names it. |
| R-10 | Recovery rows are checked against goldens by parsing RFC tables. | Same. |
| R-11 | No new test target. | CI list unchanged; slower existing targets. |
| R-12 | The real repository sees reads and prepares only before the final task. | A real-repo defect appears at the last step, as in C4, if prepares can't reach it. |
| R-13 | `poll once` under a pause or a disabled policy still runs, because it is explicit. | Owner may read pause as all polling; the RFC says "Once is explicit remote polling". |
| R-14 | If `01M669FJR2` blocks later publication through the binary, Task 12 stops for the owner instead of working around it. | One owner stop. |
| R-15 | `context show-local --token` reads content on request. Human mode prints it to stdout only with the flag. | Human content formatting may change. |

## Known Limits

- Agents' closes record the configured identity (decision 6).
- A closure race may have no CLI way out until `01M4H33R34Z7C7EEKTY1ZCT950` lands.
- `poll once` holds the remote reservation for its duration, so it answers `busy` beside
  another remote operation, and they answer it.
- A CLI-only user sees `recovery_suspended` without a verb to clear it (decision 5).
- A protected key answers `unlock_required` non-interactively, so agents' lifecycle runs
  need a terminal until `01M67Y754SJ1FTYQJFR37Z4ZTY` lands.

## Alternatives Rejected

- **A separate library ticket:** decision 1.
- **Caller-chosen `closed_by`:** decision 6.
- **Extending `conflict show` to local syncs:** decision 3.
- **Recovery examples in `--help`:** decision 4.
- **A before/after readiness diff:** no case needs it (R-4).
- **Observation-guarded policy writes:** R-7.

## Risks

| Risk | Control |
| --- | --- |
| Expectation can't be rebuilt on re-entry | Task 2 starts with a design checkpoint on it. |
| Counted-list conflicts with D-track merges | Additive; merge main at each Part. |
| `mutation_remote` already runs near an hour | Filters in tasks; full suite once. |
| Unfinished local syncs fence C4 verbs more often | Row in each outcome table, and the local-recovery journey. |
| Full suite exceeds its budget | Detached run with no tool timeout; C4 took 3 h 34 min. |
