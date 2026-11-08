---
title: "Wave 03 D6 Desktop Background Polling And Shutdown Implementation Plan"
date: 2026-11-07
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M6V02CC0B7ZKMR3XYHHXMQQV"
---

# Desktop Background Polling And Shutdown Implementation Plan

> **For agentic workers:** Revision 1, 2026-11-07, draft and unreviewed. Not
> authorized for execution: it waits for ticket `01M6KHVD6D0DB8KYT6VGQP10Y0`
> to land, for the Cycle's re-grounding, and for the owner's approval. Then use
> `implementing-a-cycle` task by task, re-read the design, repeat the ticket
> preflight, and record every checkpoint on ticket
> `01M4CC0VN3VPVVQ3RNWJKXWEVX`. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Keep discovery current inside the desktop process with one
application-owned scheduler, preserve manual priority and unsaved work, and
stop cleanly on quit, with no change to the headless library.

**Architecture:** Part A builds the clock-injected schedule model. Part B puts
automatic polls on the existing worker's background lane with preemption and
restart. Part C adds Poll now, the policy controls, the Settings page and the
needs-attention states. Part D changes shutdown. Part E proves the journeys.
All serial.

**Tech stack:** Rust 2024; `gpui-kit` through `gpui_kit::*` only; Devenv; no
new package.

**Spec:** [Cycle](../Cycles/wave-03-desktop-06-polling-and-shutdown.md) and
[design](2026-11-07-wave-03-desktop-06-polling-and-shutdown-design.md), and
`docs/design/05-sheets-and-dialogs.md` for copy. "Fact n", "U-n", "R-n",
"K-n" and "O-n" refer to the design; "A-n" to the Cycle.

## Global Constraints

**Precondition.** `01M6KHVD6D0DB8KYT6VGQP10Y0` is merged to main and the
documents are re-grounded and approved. Task 0 does not start before.

**Roles.** The **orchestrator** is the session executing this plan. It
dispatches one implementer per task and a fresh reviewer after it, one at a
time; writes the ledger and ticket comments; and pushes the ticket branch.
Implementers never dispatch subagents and never edit ticket, comment or
ledger files.

**Where.** Every task runs in `.manyhands/worktrees/01M4CC0VN3VPVVQ3RNWJKXWEVX`
on `manyhands/ticket/01M4CC0VN3VPVVQ3RNWJKXWEVX`. Every Rust command is
`devenv shell -- cargo …` with `--locked`.

**Library boundary (decision 1).** No task changes `src/repository.rs`,
`src/repository/`, `src/results.rs`, `src/canonical.rs`, `src/drafts/`,
`src/runtime.rs`, `src/lib.rs`, `Cargo.toml` or `Cargo.lock`. The only file
under `tests/` that changes is `tests/remote_polling.rs` (Task 1). A gap
(notably U-1) stops the task for the owner and becomes its own library
ticket.

**Main.** Merge `origin/main` at Task 0, at each Part checkpoint and whenever
main moves; never rebase.

**GPUI** only through `gpui_kit::*`, never a glob import.

**Privacy.** No type holding a title, body, comment or key material derives
`Debug` or `Display`.

**Test budget (AGENTS.md Testing Strategy).** Between steps run `cargo check`
or `clippy` and only the tests you wrote or changed, by exact name. Never run
a whole SSH target between steps. No `sleep` to pass time: inject the clock or
the duration. Variations go in one table-driven test. Each new test must catch
something an existing one does not. Desktop tests cover behaviour, not
appearance. Record in the ledger which tests ran at each checkpoint. Stop and
narrow at 15 minutes.

**Test runs.**
- Desktop: `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands <filter>`
  (written `… <filter>`). One filter per run; each run must report more than
  zero tests.
- The one SSH case (Task 1):
  `devenv shell -- cargo test --all-features --locked --test remote_polling poll_cancel_during_handshake_stall -- --exact`.
- The whole desktop bin suite at each Part checkpoint only. The AGENTS.md fast
  tier runs once at review-ready (Task 12); the SSH tier is exactly
  `remote_polling` (the one changed test file), run once.

**Each task's last Verify lines.**
```sh
devenv shell -- cargo fmt --check
devenv shell -- cargo check --locked --features desktop
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

**Dead code.** An item first called by a later task carries
`#[cfg_attr(not(test), allow(dead_code))] // Called from Task N.`; Task N
removes it.

**Test-first.** Each behaviour's test is written before its code.

**Existing tests** change only as E-1 to E-3 (design Testing); anything else
stops for the owner.

**Commits.** One per task, conventional message, ending
`Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`.

**Ticket records.** A comment after each Part checkpoint with
`manyhands-cli comment add` against the main checkout, only while the ticket
worktree is clean; it records commits, tests with counts and rulings. Before
`ticket save`, commit any owned `ticket.md` edit first.

**Execution ledger.**
`docs/plans/2026-11-07-wave-03-desktop-06-polling-and-shutdown-execution.md`,
created in Task 0.

**File size.** New source files stay below about 500 lines.

## Review Focus

1. **Read-only toward the shared copy.** An automatic poll's call list is
   exactly design B4 (Task 3, 11).
2. **Manual priority.** A user job cancels a running poll and runs first; the
   poll returns with no failure backoff (Task 4).
3. **No background prompt.** A locked key gives attention and no `Prompt`
   (Task 8).
4. **Pause is not an unlock block** in both directions (Tasks 2, 8).
5. **Unsaved work** survives a bring-in (Task 9).
6. **The clock.** No test sleeps; one poll per repository after a jump
   (Task 2).
7. **Shutdown** cancels the poll, shows "still stopping" at ten seconds
   without clearing anything, and joins the worker on `Idle` (Task 10).
8. **One scheduler** whatever the number of `Services` clones (Task 5).

## File Map And Dependency Order

| Task | Part | Summary |
| --- | --- | --- |
| 0 | Orchestrator | Merge main with the library ticket, baseline, U-1–U-8, ledger. |
| 1 | Readiness | One SSH case: cancel during a stalled handshake; research note. |
| 2 | A | Clock seam and schedule model (table-tested). |
| 3 | B | `PollAttempt`, `Backend::poll_automatic`, `Work::Poll`, call list. |
| 4 | B | Preemption, restart schedule, conflict-pending. |
| 5 | B | The scheduler entity in `Services`: launch, timers, wake, apply results. |
| 6 | C | Poll intents through the boundary (Check now, interval, pause, resume). |
| 7 | C | Settings page "Checking for updates", status text. |
| 8 | C | Needs attention: key, host, recovery hold; no background prompt. |
| 9 | C | Unsaved work across a bring-in; Part C checkpoint. |
| 10 | D | Shutdown: cancel, still stopping, join. |
| 11 | E | Privacy, consent and call-list evidence; keyboard journey. |
| 12 | E | Checklist, gates, review-ready. |

Order: 0 → 1 → 2 → … → 12, one at a time.

---

## Task 0: Baseline, References And Ledger

**Files:** create the ledger.

- [ ] Preflight: ticket worktree clean; `git fetch`; merge `origin/main`
  (the library ticket must be on it); `git merge-tree --write-tree HEAD
  origin/main` is clean.
- [ ] Re-grounding: compare design Fact 5 and the API Audit Refresh against
  main; add the library ticket's disposition to the design and to the
  tasks' mapping; record it in the design's Re-grounding.
- [ ] Run the required static gates and the fast tier once at baseline
  (entry-gate item 7); record counts. Do not rerun them later on an unchanged
  tree.
- [ ] Settle U-1 to U-8 by reading source; write each answer in the ledger. An
  answer that differs from the design (U-1 in particular) stops for the owner.
- [ ] Confirm decisions 3–10 are approved as written.
- [ ] Create the ledger; post a ticket comment.

**Verify:** the ledger lists baseline counts and every U answer.

## Task 1: Runtime Readiness (Entry-Gate Item 5)

**Files:** `tests/remote_polling.rs` (one case);
`docs/research/wave-03-d6-transport-stall-cancellation.md`.

**Why SSH (the design's exception test):** how long a cancel waits when a
blocking call has not returned depends on the transport; no lower layer can
show it.

- [ ] Write `poll_cancel_during_handshake_stall`: the server accepts the TCP
  connection and stalls the handshake (`Stall(Handshake, …)`); start an
  automatic poll on another thread; call `cancel_remote_operation`; assert the
  poll returns `Cancelled` or a timeout error within the documented 30 s
  bound plus teardown, that no reservation remains, and that no second
  connection follows. Emit an `SSH_OBSERVATION` line with the measured time.
- [ ] Run it singly by exact name; run no other SSH case.
- [ ] Write the research note: initialization order (Fact 21), the measured
  cancel latency, what is still unmeasured (DNS, connect to a blackhole), and
  the wording the stopping status may use ("waiting for Manyhands to reach a
  safe point", never a deadline).
- [ ] Confirm `main.rs` still initializes the transport before GPUI or any
  worker thread (read; no change).

**Verify:** the case passes alone; the note records the time. If the cancel
does not return within the bound, stop for the owner: the design's Part D
wording depends on it.

# Part A: Schedule Model

## Task 2: Clock Seam And Schedule Model

**Files:** create `D/model/polling.rs` (with `polling/` for the state and
table), `D/model/polling_tests.rs`; edit `D/model/mod.rs`.

- [ ] Test first, one table-driven test per decision, over a hand-advanced
  `Clock`: the A4 table (every answer → phase and status text); `due`
  (enabled, unpaused, unsuspended, past `next_eligible_at`, no attention;
  earliest last completion first); launch with an outstanding backoff;
  `next_wake`; `clock_event` (a 60 s difference → one poll per repository,
  staggered, no catch-up); the restart schedule 5/15/60 s then the interval;
  reconfiguration clearing attention and re-reading policy; a re-added remote
  leaving a stored pause alone; pause versus attention in both directions.
- [ ] Implement `Clock`, `RepoSchedule`, `Phase`, `Reason`, `AttemptSummary`,
  `PollAttempt` (desktop type, design B1), and the decisions of design A3.
  No thread, no worker, no `Instant::now()` outside the production clock.
- [ ] `PollAttempt`/summary types derive neither `Debug` nor `Display` where
  they could hold more than ids, kinds and counts (they hold no more; `Debug`
  allowed, privacy test in Task 11).

**Verify:** `… polling` passes; no test sleeps.

# Part B: The Worker

## Task 3: Automatic Poll On The Background Lane

**Files:** `D/services/mod.rs` (`Backend::poll_automatic`, `cancel_poll`,
`JobKind::Poll`), `D/services/worker.rs` (`Work::Poll`, `WorkerEvent::Polled`),
`D/services/poll.rs` (the library conversion, `LibraryBackend` impl),
`D/test_support/` (a recording double), tests.

- [ ] Test first: a submitted `Work::Poll` runs on the background lane and the
  UI hears `Polled(root, PollAttempt)`; it never emits `Prompt` or progress;
  the recording backend's call list is exactly `automatic_poll_eligibility`,
  `poll_publication_remote`, `remote_snapshot`, `polling_status` (O-8).
- [ ] Implement per design B1 with the `OperationId` source U-1 settled; the
  conversion maps each library result and error to a `PollAttempt` (A4's left
  column) and each target to `(item_id, kind, disposition)`.
- [ ] `cancel_poll(root, op)` calls `cancel_remote_operation` (U-5, U-6 as
  settled).

**Verify:** `… worker_poll`; `… poll_attempt`.

## Task 4: Preemption, Restart, Conflict-Pending

**Files:** `D/services/worker.rs`, `D/services/poll.rs`, tests.

- [ ] Test first: a user job submitted during a running `Work::Poll` calls
  `cancel_poll`, runs after the poll's `Cancelled`, and the model schedules the
  poll with no failure backoff (O-2, A10); a background job does not cancel;
  `Busy`/`PollYielding`/`Interrupted` resend the same operation with
  `restart=true` at 5/15/60 s under the injected clock (O-5); `ConflictPending`
  waits at the interval.
- [ ] Implement preemption in `WorkerHandle::submit` for the user lane and the
  restart resends through the model's wake.
- [ ] If U-4 finds an older interrupted row blocks a fresh operation, restart
  that row at launch (the model holds no old ID across a restart); record the
  case and test it.

**Verify:** `… preempt`; `… restart_schedule`.

## Task 5: The Scheduler In `Services`

**Files:** `D/services/scheduler.rs` (the entity), `D/services/mod.rs`,
`D/view/workspace/repositories.rs`, tests.

- [ ] Test first: `Services::start` creates one scheduler and a clone starts
  none (A17); after the repository list loads, each enabled, unpaused,
  remote-bearing repository gets a launch poll respecting its stored deadline
  (A11); a fired timer submits due polls one at a time; a result applies
  `Changed{root, items}` for `FastForwarded`/`Materialized` targets (R-10);
  `IndexPending` queues one `IndexRefresh`.
- [ ] Implement the entity: it owns the model and the production clock, arms a
  `cx.background_executor().timer` to `next_wake()`, detects a clock event by
  comparing UTC and monotonic elapsed since arming, re-reads policies on a
  clock event and on window activation (not polling on activation), and drops a
  repository's schedule when its registration is removed.
- [ ] Reconfiguration (R-11): subscribe to finished `RemoteAdd/Remove/Select`,
  `KeySelect/Clear/Remove`, `RepoRemove` and the new policy intents; cancel a
  running poll for that repository at its safe point before arming new
  configuration.

**Verify:** `… scheduler`; `… reconfigur`.

# Part C: Controls And Attention

## Task 6: Poll Intents Through The Boundary

**Files:** `D/services/requests.rs`, `D/services/requests/build.rs`,
`D/services/mod.rs` (`JobKind`), `D/model/operations.rs` and its tests.

- [ ] Test first: each of `PollOnce`, `PollConfigure{interval_seconds}`,
  `PollPause`, `PollResume` builds the matching `Mutation`; a policy verb that
  reports `writing` and never `accepted` finishes without the "not yet
  accepted" branch misreading it (Fact 2, 14); `PollOnce` may prompt.
- [ ] Add the variants and every arm of Fact 14. These are named edits E-1
  (exhaustive matches) in existing tests only where the compiler requires.
- [ ] After `PollOnce` or a policy verb finishes, the scheduler re-reads the
  repository's policy (the answer's `PollPolicy` DTO carries it).

**Verify:** `… intent_poll`.

## Task 7: Settings Page, Status Text

**Files:** `D/view/settings/polling.rs`, `D/view/settings/mod.rs`,
`D/model/stamp.rs` (`SettingsPage`), `D/model/polling/status.rs`, tests.

- [ ] Test first (behaviour only): one row per repository; the status line for
  each phase (A4's text column); the interval select offers only 60–3600 s
  values and submits `PollConfigure`; Check now submits `PollOnce` (Retry now
  after a failure) and is inert while this process's poll for it runs; the
  switch submits `PollPause`/`PollResume`; a repository with no SSH remote shows
  "No shared copy, nothing to check" and Set up sharing…; `enabled` off shows
  "Checking is off" with no control (K-5). Keyboard: every control reachable,
  focus visible (E-2: the page list gains a page).
- [ ] Implement the page from the model; no new appearance tests.

**Verify:** `… settings_polling`.

## Task 8: Needs Attention

**Files:** `D/services/scheduler.rs`, `D/services/prompts.rs` (a `Supplied`
event), `D/view/operations/`, `D/model/operations.rs`, tests.

- [ ] Test first: a locked-key attempt gives `Attention(KeyLocked)`, **no**
  `Prompt` event, and an operation-area entry with Unlock… (O-3, A13); pause
  remains as set, and Unlock… does not touch it; resuming does not clear the
  block (A3); a supplied passphrase clears the key's attention for every
  repository using it and makes them due; a failed unlock returns the
  attention; a host answer gives `Approve host…` and clears on approval; a key
  or remote change clears (R-11); a held repository shows "Checking is held
  until recovery" and Resume checking calls
  `resume_remote_polling_after_recovery`, with U-7's answer for what follows.
- [ ] Implement keyed attention (U-8 for the key and host of a repository);
  Unlock… submits `PollOnce` on the user lane.

**Verify:** `… attention`.

## Task 9: Unsaved Work Across A Bring-In; Part C Checkpoint

**Files:** tests under `D/view/item/` and `D/services/`; no source change
unless a test fails.

- [ ] Tests (A14, O-4): with an open edit session holding unsaved edits, a
  `Changed` for its item reloads the tab and keeps the text with the changed
  notice; with a kept draft the changed banner shows (`TargetState::Changed`);
  a clean session takes the new read; a save over the moved file answers
  `ExternalChange` and keeps the edits. Each disposition in `PollAttempt`
  (`Dirty`, `Diverged`, `Conflicted`, `Inaccessible`, `BlockedByLocalState`,
  `MaterializationIncomplete`, `Nonconforming`, `Malformed`,
  `RemotelyDeleted`, `HistoryUnknown`, the library ticket's closed one) is
  listed with its `guidance()` and calls no mutation (A2), as one table test.
- [ ] If an existing path loses text, stop for the owner: it is a D2/D3 defect,
  not D6's to patch silently.
- [ ] Part C checkpoint: merge main; the whole desktop bin suite once; ledger
  and a ticket comment.

**Verify:** `… bring_in`; then the full desktop bin suite.

# Part D: Shutdown

## Task 10: Cancel, Still Stopping, Join

**Files:** `D/view/quit.rs`, `D/view/workspace/lifecycle.rs`,
`D/services/worker.rs`, tests.

- [ ] Test first: `quit()` cancels a running `Work::Poll` through `cancel_poll`
  and the scheduler arms no timer; with a job held, entering `Finishing`
  changes `finishing_text` to "Still stopping — waiting for Manyhands to reach
  a safe point. Your work is saved." after the injected ten seconds and the
  UI still responds (Quit now works); nothing is cleared or reclaimed at ten
  seconds (the backend double records no credential drop); `Idle` joins the
  worker thread before `cx.quit()` (A9, O-7); quit during a restart wait ends
  without a poll; relaunch (a fresh `Services` over the same data directory)
  uses the stored deadline (A7).
- [ ] Implement; the ten-second duration is injected. E-3 changes
  `finishing_text`'s existing test only for the new wording.

**Verify:** `… quit_polling`.

# Part E: Evidence

## Task 11: Privacy, Consent And The Call List; Keyboard Journey

**Files:** tests only.

- [ ] Privacy sentinel (A16): a title, a body, a comment and a passphrase
  sentinel never appear in any operation entry, status line, attention text or
  error.
- [ ] Consent (A8): one test drives a full day of the scheduler (launch, due,
  busy, preempt, attention, quit) and asserts the recording backend saw only
  design B4's calls and the user's own intents.
- [ ] One keyboard walk (A15): open Settings, move to the page, change the
  interval, toggle the switch, Check now, Unlock…, by keystrokes alone, focus
  visible.

**Verify:** `… polling_privacy`; `… polling_consent`; `… polling_keyboard`.

## Task 12: Checklist, Gates, Review-Ready

**Files:** `docs/plans/2026-11-07-wave-03-desktop-06-on-screen-checklist.md`,
the ledger, the Cycle's Status and Acceptance table, ticket records.

- [ ] Write the native checklist from the design's outline, in the D5
  checklist's format: before you start (two scratch repositories, a stalled
  server recipe), known limits, sections and a results table.
- [ ] Merge main; `git merge-tree --write-tree HEAD origin/main` is clean.
- [ ] Run, once, the static gates across the tree, the AGENTS.md fast tier, and
  the SSH tier for the changed test file (`remote_polling`). Record counts.
- [ ] Update the Cycle: Acceptance table (met / pending native), Status,
  Review Record; the design's Review Record.
- [ ] Ticket comment: review-ready, tests run, rulings, known limits, native
  checklist. Pushes beyond the ticket branch, a PR, the merge, closure, native
  CI and cleanup stay separately gated.

**Verify:** all gates green; ledger complete; the owner has the checklist.

## Plan Self-Review

- Every Acceptance row A1–A17 maps to a task: A1 native + Task 12; A2 Task 9;
  A3 Tasks 2, 8; A4 Tasks 4, 12; A5 Tasks 1, 10; A6 Tasks 5, 6, 7; A7 Task 10;
  A8 Tasks 3, 11; A9 Task 10; A10 Task 4; A11 Tasks 2, 5; A12 Task 2;
  A13 Task 8; A14 Task 9; A15 Tasks 7, 11; A16 Task 11; A17 Task 5.
- No task changes a library file; U-1 is the one place that could force it,
  and stops for the owner.
- One SSH case, justified in Task 1; every other test is model, worker or
  desktop-behaviour level, with an injected clock.
- Existing-test edits are named: E-1 (exhaustive matches, Task 6), E-2 (Settings
  page list, Task 7), E-3 (`finishing_text`, Task 10).
