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

> **For agentic workers:** Revision 3, 2026-11-09, draft and unreviewed.
> Revision 1 was 2026-11-07. Revision 2 added the Wave 04 Cycle 01
> coordination. Revision 3 follows the landing of
> `01M6KHVD6D0DB8KYT6VGQP10Y0`: it adds Part E (retiring completed
> contexts, Tasks 11 and 12, with owner decision 16),
> renumbers the evidence tasks to 13 and 14, and follows Cycle 01's approved
> decisions. Not authorized for execution: it waits for the owner's approval
> of the documents and a separate implementation authorization. Then use
> `implementing-a-cycle` task by task, re-read the design, repeat the ticket
> preflight, and record every checkpoint on ticket
> `01M4CC0VN3VPVVQ3RNWJKXWEVX`. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Keep discovery current inside the desktop process with one
application-owned scheduler, preserve manual priority and unsaved work, stop
cleanly on quit, and give the person a deliberate Retire for contexts
completed elsewhere, with no change to the headless library.

**Architecture:** Part A builds the clock-injected schedule model. Part B puts
automatic polls on the existing worker's background lane with preemption and
restart. Part C adds Poll now, the policy controls, the Settings page and the
needs-attention states. Part D changes shutdown. Part E adds the deliberate
Retire. Part F proves the journeys. All serial.

**Tech stack:** Rust 2024; `gpui-kit` through `gpui_kit::*` only; Devenv; no
new package.

**Spec:** [Cycle](../Cycles/wave-03-desktop-06-polling-and-shutdown.md) and
[design](2026-11-07-wave-03-desktop-06-polling-and-shutdown-design.md), and
`docs/design/05-sheets-and-dialogs.md` for copy. "Fact n", "U-n", "R-n",
"K-n" and "O-n" refer to the design; "A-n" to the Cycle.

## Global Constraints

**Precondition.** `01M6KHVD6D0DB8KYT6VGQP10Y0` is merged (2026-11-09,
`f4442618`) and the documents are re-grounded at `9512080d`. The documents
must also be approved and implementation authorized. Task 0 does not start
before.

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

**Wave 04 Cycle 01** (`01M6V8RK8B0SBCH0YZXFVSR9E3`, approved 2026-11-08 and
in implementation) is expected to merge before D6 (Cycle, Wave 04 Cycle 01
Coordination; design W-1 to W-7). At Task 0 and at every merge of main, note
in the ledger whether it is on main. Tasks 5, 8, 10 and 13 each have a step
that applies only when it is. If it merges mid-task, finish the task, merge
main, then apply the steps the earlier tasks skipped before starting the
next one. Every row D6 adds to the operation area wraps inside Activity
(decision 13, W-6). Check this by reaching each new action by keyboard from
Activity, not by measuring layout.

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
  tier runs once at review-ready (Task 14); the SSH tier is exactly
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

**Existing tests** change only as E-1 to E-5 (design Testing); anything else
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
   exactly design B4, with no `ContextRetire` or `PollOnceRetiring` (Tasks 3,
   13).
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
9. **Retiring passes run only where E1 says.** Check now is a pass; an
   automatic poll is followed by one pass only when it found a candidate
   outside the refused set and no tab in that repository holds unkept text;
   a background pass never prompts (Task 11).
10. **Retirement loses nothing.** Unkept text holds back a pass and stops
    Retire…; drafts are flushed first and outlive it; refusals and pending
    cleanup are shown as the library answers, with Resume and Abandon
    (Tasks 11, 12).

## File Map And Dependency Order

| Task | Part | Summary |
| --- | --- | --- |
| 0 | Orchestrator | Merge main, re-check what moved since `9512080d`, baseline, U-1–U-12, ledger. |
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
| 11 | E | Retiring passes: Check now, the follow-up after an automatic poll, no background prompt, answers. |
| 12 | E | Retire… for what is left; recovery; drafts after retirement; Part E checkpoint. |
| 13 | F | Privacy, consent and call-list evidence; keyboard journey. |
| 14 | F | Checklist, gates, review-ready. |

Order: 0 → 1 → 2 → … → 14, one at a time.

---

## Task 0: Baseline, References And Ledger

**Files:** create the ledger.

- [ ] Preflight: ticket worktree clean; `git fetch`; merge `origin/main`;
  `git merge-tree --write-tree HEAD origin/main` is clean.
- [ ] Re-grounding: the library re-grounding at `9512080d` is in the design
  (2026-11-09). Compare only what moved on main since then against the
  Baseline Facts and the API Audit Refresh, and record it in the design's
  Re-grounding; a difference that changes a ruling stops for the owner.
- [ ] Wave 04 Cycle 01: if it is on main, check W-1 to W-7 against its code
  (`needs_attention`'s inputs, where the launch refresh and `finishing` now
  live, the panel and region names in tests) and record the answers in the
  design's Re-grounding; a difference that changes a W item stops for the
  owner. If it is not on main, record its ticket's state in the ledger.
- [ ] Run the required static gates and the fast tier once at baseline
  (entry-gate item 7); record counts. Do not rerun them later on an unchanged
  tree.
- [ ] Settle U-1 to U-12 by reading source; write each answer in the ledger.
  An answer that differs from the design (U-1 and U-11 in particular) stops
  for the owner.
- [ ] Confirm decisions 3–15 are approved as written (16 is the owner's).
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
  allowed, privacy test in Task 13).

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
  `poll_publication_remote`, `remote_snapshot`, `polling_status`, and never
  `ContextRetire` or `PollOnceRetiring` (O-8, R-19).
- [ ] Implement per design B1 with the `OperationId` source U-1 settled; the
  conversion maps each library result and error to a `PollAttempt` (A4's left
  column) and each target to `(item_id, kind, disposition)`, including
  `Closed`, `Promoted` and `Unpublished` (Fact 5).
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
`D/view/workspace/repositories.rs` (one call only, design W-3), tests.

The scheduler is held by `Services`; `Workspace` gains no field (R-16). The
launch poll is one call beside the launch refresh in `repositories_loaded`,
or wherever Wave 04 Cycle 01 moved that refresh. Keep every other edit out of
`D/view/workspace/`.

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
- [ ] Add the model's read-only `attention()` query (design W-1), with a
  model test that it yields exactly the five reasons and nothing for Paused,
  Checking is off, Can't reach, Checking soon or Waiting for a conflict
  (decision 11, R-14).
- [ ] **With Wave 04 Cycle 01 on main:** make the polling model a third
  input of `needs_attention(&Operations, &DraftRegistry)`
  (`D/model/attention.rs`), add `attention()`'s count, update its callers,
  and extend its existing test with a row per reason and per state that does
  not count (E-4, A18). Do not change how it counts anything else. Without
  Cycle 01, note in the ledger that its merge adds the term.
- [ ] The attention entries wrap inside Activity (decision 13); each action
  is reached by keyboard in the test above.

**Verify:** `… attention`; with Cycle 01 on main, also `needs_attention`'s
test by exact name.

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
  `RemotelyDeleted`, `HistoryUnknown`, `Unpublished`, `Closed`, `Promoted`)
  is listed with its text and calls no mutation (A2), as one table test. The
  text is `guidance()`, except where that names a CLI command: Share… for
  `Unpublished`; `Closed` and `Promoted` are retired by Task 11's pass or
  offered Task 12's Retire… (R-20).
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
  `finishing_text`'s existing test only for the new wording. The new text
  lives only in `finishing_text`; no other view draws stopping feedback
  (R-15).
- [ ] **With Wave 04 Cycle 01 on main:** hide the Activity panel, start a
  quit with a job held, advance the injected ten seconds, and assert that the
  layout is unchanged (Activity still hidden) and that `quit-finishing`
  carries the still-stopping text (Cycle 01 decision 7, K-18; design
  decision 12; E-5, A18). Extend whichever of Cycle 01's
  quit-with-Activity-hidden test (its A7) and this task's test is narrower;
  leave the other alone.

**Verify:** `… quit_polling`; with Cycle 01 on main, also the extended test
by exact name.

# Part E: Retiring Completed Contexts

## Task 11: Retiring Passes After Polls And On Check Now

**Files:** `D/services/requests.rs`, `D/services/requests/build.rs`,
`D/services/mod.rs` (`JobKind`), `D/services/prompts.rs` (the background
answer, R-22), `D/services/scheduler.rs`, `D/model/operations.rs` (label and
stage arms only), `D/model/polling/` (the refused set and which pass runs),
`D/view/operations/` (the pass's rows), tests.

- [ ] Test first, over canned `poll_retirement` answers (decision 9):
  - one table for design E1: Check now submits `PollOnceRetiring`, or
    `PollOnce` when a tab in that repository holds unkept text; an automatic
    attempt with a `Closed` or `Promoted` target outside the refused set,
    and no unkept tab in the repository, is followed by exactly one
    background `PollOnceRetiring` for that repository; an attempt with none,
    or with only refused-set candidates, or with an unkept tab, is followed
    by nothing (A8, A19, A21);
  - the repository's draft sessions are flushed before any pass, and a
    flush failure stops it with D2's entry;
  - a background pass never emits `Prompt`: its passphrase or host request
    is answered at once, does not mark the key declined, and becomes the
    key's or host's attention state (R-22, A13; U-12 as settled);
  - a user job submitted during a background pass waits for it and does not
    cancel it (K-17);
  - one table over design E3's item states: what is shown, `Changed{root,
    items}` for `retired` and `already_absent`, `refused` and `preserved`
    joining the refused set, `not_reached` offered again, `pending` leaving
    the pending row (A20; U-10 as settled).
- [ ] Add `Intent::PollOnceRetiring` with every arm of Fact 14 (E-1 where the
  compiler requires); the automatic poll's own call list stays B4's.
- [ ] The pass's rows wrap inside Activity (decision 13); each action is
  reached by keyboard.

**Verify:** `… retiring_pass`; `… retire_answer`; `… background_prompt`.

## Task 12: Retire… For What Is Left; Recovery And Drafts; Part E Checkpoint

**Files:** `D/services/requests.rs`, `D/services/requests/build.rs`, the
Retire… row in `D/view/operations/`, a Retire ask in `D/view/` beside the
quit asks, tests; other source only where U-9 or U-11 finds a gap.

- [ ] Test first (A19–A21):
  - Retire… shows only on a `Closed` or `Promoted` target the last pass did
    not retire (skipped, refused, preserved or not reached), with the words
    and short code of design E3, never a title;
  - it submits exactly `Intent::ContextRetire{root, kind, id}` on the user
    lane and removes the item from the refused set;
  - with a tab of that item holding text no draft keeps, nothing is
    submitted, and the ask offers Show tab (the default, showing the tab)
    and Cancel, with no "Retire anyway" (decision 15);
  - its answers follow E3's rows.
- [ ] Test first (A20): a `pending` item, from a pass or a Retire…, leaves
  D5's pending row for that item's own request, with Resume
  (`OperationResume`, the original request) and Abandon. Abandon opens the
  existing confirmation, which shows the retirement's details (Fact 18).
  Both work by keyboard. Add only the label or arm U-9 finds missing.
- [ ] Test first (A21, O-11): after retirement, the item's kept draft and a
  comment draft are still on the Drafts page and can be restored and
  exported; an open tab of the item reads primary's copy (U-11). If either
  loses a draft or cannot be exported, stop for the owner: it is a D2
  defect, not D6's to patch silently.
- [ ] Add `Intent::ContextRetire` with every arm of Fact 14 (E-1).
  `changes_items` follows U-10.
- [ ] Part E checkpoint: merge main; the whole desktop bin suite once;
  ledger and a ticket comment.

**Verify:** `… retire_left`; `… retire_recovery`; `… retire_drafts`; then the
full desktop bin suite.

# Part F: Evidence

## Task 13: Privacy, Consent And The Call List; Keyboard Journey

**Files:** tests only.

- [ ] Privacy sentinel (A16): a title, a body, a comment and a passphrase
  sentinel never appear in any operation entry, status line, attention text,
  Retire offer or answer, or error.
- [ ] Consent (A8): one test drives a full day of the scheduler (launch, due,
  busy, preempt, attention, a `closed` target, an unkept tab, quit) and
  asserts the recording backend saw only design B4's calls, the user's own
  intents, and a `PollOnceRetiring` only where design E1 allows it: never
  inside an automatic poll, never with an unkept tab in that repository, and
  no `ContextRetire` unless the person chose Retire….
- [ ] One keyboard walk (A15): open Settings with `secondary-,` (it opens
  the page in both layouts, design W-4), move to the page, change the
  interval, toggle the switch, Check now, then reach Unlock… and a Retire…
  in the operation area (`secondary-4`, which Wave 04 Cycle 01 keeps for
  Activity), by keystrokes alone, focus visible.

**Verify:** `… polling_privacy`; `… polling_consent`; `… polling_keyboard`.

## Task 14: Checklist, Gates, Review-Ready

**Files:** `docs/plans/2026-11-07-wave-03-desktop-06-on-screen-checklist.md`,
the ledger, the Cycle's Status and Acceptance table, ticket records.

- [ ] Write the native checklist from the design's outline, in the D5
  checklist's format: before you start (two scratch repositories, a second
  clone for the retire section, a stalled server recipe), known limits,
  sections and a results table.
- [ ] Merge main; `git merge-tree --write-tree HEAD origin/main` is clean.
  If Wave 04 Cycle 01 landed after Task 8, 10 or 13 ran, apply their Cycle 01
  steps now and record them.
- [ ] Run, once, the static gates across the tree, the AGENTS.md fast tier, and
  the SSH tier for the changed test file (`remote_polling`). Record counts.
- [ ] Update the Cycle: Acceptance table (met / pending native), Status,
  Review Record; the design's Review Record.
- [ ] Ticket comment: review-ready, tests run, rulings, known limits, native
  checklist. Pushes beyond the ticket branch, a PR, the merge, closure, native
  CI and cleanup stay separately gated.

**Verify:** all gates green; ledger complete; the owner has the checklist.

## Plan Self-Review

- Every Acceptance row A1–A21 maps to a task: A1 native + Task 14; A2 Task 9;
  A3 Tasks 2, 8; A4 Tasks 4, 14; A5 Tasks 1, 10; A6 Tasks 5, 6, 7; A7 Task 10;
  A8 Tasks 3, 13; A9 Task 10; A10 Task 4; A11 Tasks 2, 5; A12 Task 2;
  A13 Tasks 8, 11; A14 Task 9; A15 Tasks 7, 13; A16 Task 13; A17 Task 5;
  A18 Tasks 8, 10 (or Cycle 01, if D6 merges first); A19 Tasks 11, 14;
  A20 Tasks 11, 12; A21 Tasks 11, 12.
- No task changes a library file; U-1 is the one place that could force it,
  and stops for the owner.
- One SSH case, justified in Task 1; every other test is model, worker or
  desktop-behaviour level, with an injected clock. Retire adds no SSH case:
  its transport is the library's `retirement_cli_authenticated_live_pass`.
- Existing-test edits are named: E-1 (exhaustive matches, Tasks 6, 11, 12), E-2 (Settings
  page list, Task 7), E-3 (`finishing_text`, Task 10), and, with Wave 04
  Cycle 01 on main, E-4 (its Needs attention count test, Task 8) and E-5 (quit
  with Activity hidden, Task 10).
