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

> **For agentic workers:** Revision 5, 2026-11-09, draft; review rounds 1
> and 2 applied, round 3 pending. Revision 5 applies round 2 and owner
> decisions 24–27: Task 4 gains the in-flight record and the stale-row
> outcomes L-1 to L-6 with a design checkpoint. Revision 1 was 2026-11-07; revision 2 added the
> Wave 04 Cycle 01 coordination; revision 3 followed the landing of
> `01M6KHVD6D0DB8KYT6VGQP10Y0` and owner decisions 16 and 17. Revision 4
> applies review round 1 and owner decisions 18–23: Task 11 splits into 11a
> and 11b, and test runs follow AGENTS.md. Not authorized for execution: it
> waits for the owner's approval of the documents and a separate
> implementation authorization. Then use `implementing-a-cycle` task by task,
> re-read the design, repeat the ticket preflight, and record every
> checkpoint on ticket `01M4CC0VN3VPVVQ3RNWJKXWEVX`. Steps use checkbox
> (`- [ ]`) syntax.

**Goal:** Keep discovery current inside the desktop process with one
application-owned scheduler, preserve manual priority and unsaved work, stop
cleanly on quit, and retire this computer's copies of work completed
elsewhere, item by item, with Retire… for what a poll leaves. No change to
the headless library.

**Architecture:** Part A builds the clock-injected schedule model. Part B puts
automatic polls on the existing worker's background lane with preemption,
restart and a provider that never prompts for background jobs. Part C adds
Check now, the policy controls, the Settings page and the needs-attention
states. Part D changes shutdown. Part E retires each reported completed item
and adds Retire… for the rest. Part F proves the journeys. All serial.

**Tech stack:** Rust 2024; `gpui-kit` through `gpui_kit::*` only; Devenv; no
new package.

**Spec:** [Cycle](../Cycles/wave-03-desktop-06-polling-and-shutdown.md) and
[design](2026-11-07-wave-03-desktop-06-polling-and-shutdown-design.md), and
`docs/design/05-sheets-and-dialogs.md` for copy. "Fact n", "U-n", "R-n",
"K-n", "W-n", "E-n" and "O-n" refer to the design; "A-n" to the Cycle.

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
stops the task for the owner and becomes its own library ticket.

**Main.** Merge `origin/main` at Task 0, at each Part checkpoint and whenever
main moves; never rebase.

**Wave 04 Cycle 01** (`01M6V8RK8B0SBCH0YZXFVSR9E3`, approved 2026-11-08 and
in implementation) is expected to merge before D6 (Cycle, Wave 04 Cycle 01
Coordination; design W-1 to W-7). At Task 0 and at every merge of main, note
in the ledger whether it is on main. Tasks 8, 10 and 13 each have a step that
applies only when it is. If it merges mid-task, finish the task, merge main,
then apply the steps the earlier tasks skipped before starting the next one.
Every row D6 adds to the operation area wraps inside Activity (decision 13,
W-6); a test clicks each action at its position at the default window.

**GPUI** only through `gpui_kit::*`, never a glob import.

**Privacy.** No type holding a title, body, comment or key material derives
`Debug` or `Display`.

**Test budget (AGENTS.md Testing Strategy, 2026-11-06).** It overrides any
broader run in this plan or a skill:

- Between steps, run `cargo check` or `clippy` and only the tests you wrote
  or changed, by exact name.
- Never run a whole SSH target, or the whole desktop bin suite, between
  steps or at a Part checkpoint.
- The static gates, the fast tier and the SSH tier for `remote_polling` run
  once, at review-ready (Task 14). Task 0 runs the static gates only.
- No `sleep` to pass time: inject the clock or the duration. Variations go in
  one table-driven test. Each new test must catch something an existing one
  does not. Desktop tests cover behaviour, not appearance.
- Record in the ledger which tests ran at each checkpoint. Stop and narrow at
  15 minutes.

**Test runs.**
- Desktop: `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands <path> -- --exact`
  (written `… <name>`). With `--exact`, `<path>` is the test's full module
  path, such as `desktop::model::polling::tests::after_attempt_table`. Each
  task names its tests; the implementer gives each new test exactly that
  name and records its full path in the ledger. Each run must report more
  than zero tests.
- The SSH cases (Task 1), one at a time:
  `devenv shell -- cargo test --all-features --locked --test remote_polling <case> -- --exact`.

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
stops for the owner. The polling switch (R-27) keeps launch polls out of
existing tests.

**Commits.** One per task, conventional message, ending
`Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`.

**Ticket records.** A comment after each Part checkpoint (Tasks 2, 5, 9, 10,
12 and 14) with `manyhands-cli comment add` against the up-to-date main
checkout, only while the ticket worktree is clean. It records commits, tests
with counts and rulings. Before `ticket save`, commit any owned `ticket.md`
edit first.

**Execution ledger.**
`docs/plans/2026-11-07-wave-03-desktop-06-polling-and-shutdown-execution.md`,
created in Task 0. Each ruling made during execution is recorded there as
"Ruling: decision — why — cost if wrong".

**File size.** New source files stay below about 500 lines.

## Review Focus

1. **Read-only toward the shared copy.** An automatic poll's call list is
   exactly design B4's five calls, with no `ContextRetire` or
   `PollOnceRetiring` (Tasks 3, 13).
2. **Manual priority.** A user job cancels a running poll, even one not yet
   reserved, and runs first; the poll returns with no failure backoff; a
   background retirement is not cancelled (Tasks 4, 11b).
3. **No background prompt.** Through the real provider with a fresh session,
   a locked key gives attention and no `Prompt`, for a poll and for a
   retirement (Tasks 3, 11a).
4. **Live failures classify correctly.** `Err(Transport)` kinds map to A4's
   rows, and attention is keyed by key or host (Tasks 2, 3).
5. **Pause is not an unlock block** in both directions (Tasks 2, 8).
6. **Unsaved work** survives a bring-in over a real repository (Task 9).
7. **The clock.** No test sleeps; one poll per repository after a jump; the
   launch deadline holds (Task 2).
8. **Stale rows.** Only a row the in-flight record names is taken over, only
   at launch, only by the lock holder; `Busy` only waits; a pending local
   operation never makes polls loop (Tasks 4, 5).
9. **Shutdown** cancels the poll, sees it end, shows "still stopping" at ten
   seconds without clearing anything, and joins the worker on every
   graceful exit (Task 10).
10. **Retirement is per item and only where E1 says.** Each reported item is
    checked alone, retired by its own new request, one at a time; a pending
    item or an unlock or host answer stops the loop (Task 11b).
11. **Retirement loses nothing.** Unkept text and kept drafts hold back their
    own item; tabs are read-only while retiring; drafts outlive retirement,
    exportable; Resume resends the original request (Tasks 11b, 12).

## File Map And Dependency Order

| Task | Part | Summary |
| --- | --- | --- |
| 0 | Orchestrator | Merge main, re-check what moved since `9512080d`, static gates, remaining U-questions, ledger. |
| 1 | Readiness | Two SSH cases: cancel during a stalled handshake and a stalled connect; research note. |
| 2 | A | Clock seam and schedule model (table-tested); Part A checkpoint. |
| 3 | B | `PollAttempt`, five thin `Backend` methods, `Work::Poll`, classification, background provider rule. |
| 4 | B | Preemption at every user-lane entry, off the UI thread; restart schedule; in-flight record and instance lock; stale rows L-1 to L-6 (design checkpoint). |
| 5 | B | The scheduler entity in `Services`: switch, launch, timers, wake, results, reconfiguration, the bounded wait on an unfinished operation; Part B checkpoint. |
| 6 | C | Poll intents through the boundary (Check now, interval, pause, resume). |
| 7 | C | Settings page "Checking for updates", status text, D5's way-out Check now. |
| 8 | C | Needs attention per key, host and repository; Unlock…; prompt deduplication. |
| 9 | C | Unsaved work across a bring-in over a real repository; dispositions; Part C checkpoint. |
| 10 | D | Shutdown: cancel, the poll's end seen, still stopping, join; Part D checkpoint. |
| 11a | E | `Intent::ContextRetire`, answer classification by code, row dismissals. |
| 11b | E | The per-item loop after a poll, read-only tabs, the refused set. |
| 12 | E | Retire…, Resume's resend, drafts and end state over a real repository, replay and interruption; Part E checkpoint. |
| 13 | F | Privacy, consent and call-list evidence; keyboard journey. |
| 14 | F | Checklist, gates, review-ready, gated closure. |

Order: 0 → 1 → 2 → … → 10 → 11a → 11b → 12 → 13 → 14, one at a time.

---

## Task 0: Baseline, References And Ledger

**Files:** create the ledger.

- [ ] Preflight: ticket worktree clean; `git fetch`; merge `origin/main`;
  `git merge-tree --write-tree HEAD origin/main` is clean.
- [ ] Re-grounding: compare only what moved on main since `9512080d` against
  the Baseline Facts and the API Audit Refresh, and record it in the design's
  Re-grounding; a difference that changes a ruling stops for the owner.
- [ ] Wave 04 Cycle 01: if it is on main, check W-1 to W-7 against its code
  (`needs_attention`'s inputs, where the launch refresh and `finishing` now
  live, its quit focus rule and A7 test, the panel and region names in
  tests) and record the answers in the design's Re-grounding; a difference
  that changes a W item stops for the owner. If it is not on main, record
  its ticket's state in the ledger.
- [ ] Run the static gates once at baseline (entry-gate item 7, as amended
  by AGENTS.md's Testing Strategy; Document Amendment 9). Record the result.
  Do not run the fast tier here.
- [ ] Settle U-2, U-9 (rest), U-11 (rest), U-13, U-14, U-15 and U-16 by reading source,
  and re-check the settled U answers at this baseline; write each in the
  ledger. An answer that differs from the design stops for the owner.
- [ ] Confirm decisions 3–15 are approved as written (16–27 are the
  owner's).
- [ ] Create the ledger; post a ticket comment.

**Verify:** the ledger lists the baseline gates and every U answer.

## Task 1: Runtime Readiness (Entry-Gate Item 5, Decision 22)

**Files:** `tests/remote_polling.rs` (two cases);
`docs/research/wave-03-d6-transport-stall-cancellation.md`.

**Why SSH (the design's exception test):** how long a cancel waits when a
blocking call has not returned depends on the transport; no lower layer can
show it.

- [ ] Write `poll_cancel_during_handshake_stall`. The server accepts the TCP
  connection and stalls the handshake (`Stall(FixtureBoundary::Handshake,
  d)` with `d` of at least 45 s, under the harness's 100 s kill; Fact 23).
  Start an automatic poll on another thread. Wait until the row exists
  (`accepted_sockets()` has risen, or `polling_status.active_operation_id`
  is set). Then call `cancel_remote_operation` on the same service instance.
  Assert the poll returns `Ok(Polled{completion: Cancelled})` (a timeout
  error means the cancel was lost, and the test fails), within the
  documented 30 s bound plus teardown. Assert no reservation remains and no
  second connection follows. Emit an `SSH_OBSERVATION` line with the
  measured time.
- [ ] Write `poll_cancel_during_connect_stall` the same way, with a TCP
  connect that never completes, made as U-14 settles. If U-14 finds no
  reliable way, stop for the owner (decision 22 then falls back to a
  departure).
- [ ] Run each singly by exact name; run no other SSH case.
- [ ] Write the research note: initialization order (Fact 21), both measured
  cancel latencies, what is still unmeasured (DNS, K-20), and the wording the
  stopping status may use ("waiting for Manyhands to reach a safe point",
  never a deadline).
- [ ] Confirm `main.rs` still initializes the transport before GPUI or any
  worker thread (read; no change).

**Verify:** each case passes alone; the note records both times. If a cancel
does not return within the bound, stop for the owner: Part D's wording
depends on it.

# Part A: Schedule Model

## Task 2: Clock Seam And Schedule Model; Part A Checkpoint

**Files:** create `D/model/polling.rs` (with `polling/` for the state and
table), `D/model/polling_tests.rs`; edit `D/model/mod.rs`.

- [ ] Test first, one table-driven test per decision, over a hand-advanced
  `Clock`:
  - `after_attempt_table`: design A4's table, every row, including each
    `Err(Transport)` kind and each `NotEligible` reason;
  - `due_order_and_launch`: enabled, unpaused, unsuspended, past
    `next_eligible_at`, no attention; earliest last completion first; at
    launch both an interval deadline and a backoff deadline hold (decision
    23, A11);
  - `next_wake_table`;
  - `clock_event_once`: a 60 s difference gives one poll per repository,
    staggered, no catch-up (A12);
  - `restart_schedule`: 5/15/60 s, then the interval;
  - `reconfiguration_table`: clearing attention and re-reading policy; a
    re-added remote leaves a stored pause alone;
  - `pause_versus_attention`: in both directions (A3);
  - `attention_keys`: one entry per key, host or held repository; a key
    locked for three repositories is one entry (decision 11).
- [ ] Implement `Clock`, `RepoSchedule`, `Phase`, `Reason`, `AttemptSummary`,
  `PollAttempt` (desktop type, design B1), `attention()`, and the decisions of
  design A3. No thread, no worker, no `Instant::now()` outside the
  production clock.
- [ ] `PollAttempt` and summary types hold only ids, kinds, counts and
  times; `Debug` is allowed (privacy test in Task 13).
- [ ] Part A checkpoint: ledger and a ticket comment.

**Verify:** each named test by exact name; no test sleeps.

# Part B: The Worker

## Task 3: Automatic Poll On The Background Lane

**Files:** `D/services/mod.rs` (the five `Backend` methods, `cancel_poll`,
`JobKind::Poll`), `D/services/worker.rs` (`Work::Poll`, `WorkerEvent::Polled`,
the running job's lane for the provider), `D/services/prompts.rs` (the
background answer), `D/services/poll.rs` (the conversion, `LibraryBackend`
impl), `D/test_support/` (a recording double with a gate), tests.

- [ ] Test first:
  - `worker_poll_background`: a submitted `Work::Poll` runs on the
    background lane and the UI hears `Polled(root, PollAttempt)`; it emits no
    progress;
  - `poll_call_list`: the recording double's list for an automatic poll is
    exactly `resolve`, `polling_status`, `poll_eligibility`, `poll_automatic`,
    `remote_snapshot`; `cancel_poll` appears only for a preemption or a quit
    (O-8, R-30);
  - `poll_attempt_conversion`: the conversion maps library answers to
    `PollAttempt`, including `Err(Transport(e))` classified by `e.kind` and
    keyed by `e.selected_key_id` and `e.authority` (R-29), using fabricated
    library values where they can be built;
  - `background_poll_never_prompts`: through the real `DesktopProvider` and
    a fresh `SessionCredentials` with a locked key, an automatic poll emits
    no `Prompt`, does not mark the key declined, records the refused key
    against the running request, and answers a locked-key failure (B5, R-26,
    A13). Nothing reaches the provider without SSH, so the double calls
    `session.with_passphrase` with a public `UnlockRequest`, as the transport
    would. It must fail if the provider is not lane-aware.
- [ ] Implement per design B1 and B5 with `OperationId::new()` (U-1). The
  worker tells the provider the running job's lane through the per-job state
  it already sets (Fact 15).
- [ ] `cancel_poll(root, op)` calls `cancel_remote_operation` (Fact 30).
- [ ] `Polled` is sent after the worker clears its running job (Fact 36).

**Verify:** the four tests by exact name.

## Task 4: Preemption, Restart, The In-Flight Record, Stale Rows

**Files:** `D/services/worker.rs` (preemption in the shared admission step,
`WorkerEvent::PreemptPoll`), `D/services/mod.rs` (the drain spawns the cancel
task), `D/services/inflight.rs` (the in-flight record and instance lock),
`D/services/poll.rs`, `D/services/scheduler.rs`, tests.

- [ ] Test first:
  - `preempt_from_every_entry`: a user job entering by `submit`,
    `submit_with_request_id`, `requeue` or `observe_host` during a held
    `Work::Poll` sets the preempt flag and sends `PreemptPoll`. The drain
    spawns a background-executor task (never the UI thread) that calls
    `cancel_poll` and sends it again until the first `Ok` or until the poll
    answers, including when the first cancel errors because the row does not
    exist yet and when one answers `RepositoryBusy`. The user job runs after
    the poll's `Cancelled`, and the poll is rescheduled with no failure
    backoff (O-2, A10, R-3, R-25);
  - `preempt_bound`: a poll preempted three times in a row waits for the
    interval (R-34);
  - `background_job_does_not_preempt`;
  - `restart_schedule_same_op`: `Err(Busy)`, `Err(Interrupted)` and
    `Ok(Polled{completion: Interrupted})` resend the same operation with
    `restart=true` at 5/15/60 s under the injected clock, then the interval
    with a new ID; `Busy` never triggers a takeover (O-5, L-5);
  - `conflict_pending_waits`: `ConflictPending` waits at the interval;
  - `inflight_record`: an automatic poll's entry is written before it is
    submitted and removed on its terminal answer; only the instance lock's
    holder writes; a second instance writes nothing (R-31, L-6).
- [ ] **Design checkpoint** before the stale-row code: the orchestrator and
  the owner check U-13, U-15 and U-16's answers against L-1 to L-6. A
  difference stops here.
- [ ] Test first, then implement: `stale_rows_table`, one table over L-1 to
  L-6 with canned library answers and a seeded in-flight record (A23). The
  library's own takeover is proved by its tests.
- [ ] Implement preemption in the shared admission step, the drain's cancel
  task, the restart resends through the model's wake, the in-flight record
  and the instance lock, and L-1 to L-6.

**Verify:** the seven tests by exact name.

## Task 5: The Scheduler In `Services`; Part B Checkpoint

**Files:** `D/services/scheduler.rs` (the entity), `D/services/mod.rs` (the
polling switch, off unless `with_polling()`; `D/mod.rs`'s `run()` switches it
on), `D/model/operations.rs` (one new entry point that marks a
repository changed), `D/view/workspace/repositories.rs` (one call only),
`D/mod.rs` or the window opener (`open_main_window`, one subscription),
tests.

The scheduler is held by `Services`; `Workspace` gains no field (R-16). The
launch poll is one call beside the launch refresh in `repositories_loaded`,
or wherever Wave 04 Cycle 01 moved that refresh. Window activation is one
subscription in `open_main_window`. Keep every other edit out of
`D/view/workspace/`.

- [ ] Test first:
  - `scheduler_shared_by_clones`: a poll submitted through a `Services` clone
    is seen by the original's scheduler (A17);
  - `polling_switch_off_in_tests`: `Services::start` without `with_polling()`
    starts no poll; with it, polls start (R-27);
  - `launch_polls_due_ssh_repos`: after the repository list loads, each
    enabled, unpaused repository with an SSH publication remote and a due
    deadline gets a launch poll; a repository without one is never polled;
    remotes are read again on window activation (U-2, B6, A11);
  - `timer_submits_one_at_a_time`;
  - `result_marks_repository_changed`: `FastForwarded` and `Materialized`
    targets mark the repository changed through the new `Operations` entry
    point, and every tab of it reloads (Fact 17, R-10); `IndexPending`
    queues one `IndexRefresh`;
  - `reconfiguration_cancels_and_rearms`: when a `RemoteAdd`, `RemoteRemove`,
    `RemoteSelect`, `KeySelect`, `KeyClear`, `KeyRemove`, `KeyDelete`,
    `HostApprove`, `HostReplace`, `RepoCreate`, `RepoEnable` or `RepoRemove`
    is submitted, every repository using that key, host or remote has its
    queued retirements dropped; when it finishes, their polls are cancelled,
    attention cleared, policies re-read and re-armed. A policy intent only
    re-reads (R-11, A6);
  - `second_actor_busy` (over a real service): a second library actor over
    the same registry, in the same process (decision 26), holds the remote
    reservation, and the automatic poll answers `Busy` and restarts on
    schedule (A4);
  - `unfinished_operation_waits` (over a real service): with a pending local
    operation, the poll answers `recovery_required`, the row says "Waiting
    for an unfinished operation", and the scheduler polls at most once per
    R-4 step under the injected clock, then at the interval (R-35, A23).
- [ ] Implement the entity: it owns the model and the production clock, arms
  a `cx.background_executor().timer` to `next_wake()`, detects a clock event by
  comparing UTC and monotonic elapsed since arming, re-reads policies on a
  clock event and on window activation (not polling on activation), and drops
  a repository's schedule when its registration is removed.
- [ ] Part B checkpoint: merge main; ledger and a ticket comment.

**Verify:** the eight tests by exact name.

# Part C: Controls And Attention

## Task 6: Poll Intents Through The Boundary

**Files:** `D/services/requests.rs`, `D/services/requests/build.rs`,
`D/services/mod.rs` (`JobKind`), `D/model/operations.rs` and its tests.

- [ ] Test first:
  - `intent_poll_build`: each of `PollOnce`, `PollConfigure{interval_seconds}`,
    `PollPause`, `PollResume` builds the matching `Mutation`;
  - `intent_poll_stoppable`: `PollOnce` is `stops_at_safe_points: true`, so
    Cancel and Give up work (Fact 14);
  - `policy_verb_never_accepted`: a policy verb that reports `writing` and
    never `accepted` finishes without the "not yet accepted" branch
    misreading it (Fact 2);
  - `policy_verb_resent`: a policy verb sent again follows C5 decision 15
    (last commit wins) (A22);
  - `poll_once_rereads_status`: after `PollOnce`, the scheduler re-reads
    `polling_status`; after a policy verb, it takes the answer's
    `PollPolicy`.
- [ ] Add the variants and every arm of Fact 14 (E-1 in existing tests only
  where the compiler requires). `PollOnce` may prompt.

**Verify:** the five tests by exact name.

## Task 7: Settings Page, Status Text, D5's Way-Out

**Files:** `D/view/settings/polling.rs`, `D/view/settings/mod.rs`,
`D/model/stamp.rs` (`SettingsPage`), `D/model/polling/status.rs`,
`D/view/lifecycle/prepare.rs` (the way-out button), tests.

- [ ] Test first (behaviour only):
  - `settings_polling_rows`: one row per repository; the status line for each
    phase (A4's text column); the intro text of design C2;
  - `settings_polling_controls`: the interval select offers only 60–3600 s
    values and submits `PollConfigure`; Check now submits `PollOnce` (Retry
    now after a failure); during an automatic poll Check now cancels it and
    runs; it is disabled only while this process's own Check now for that
    repository is queued or running (C1); the switch submits
    `PollPause`/`PollResume`;
  - `settings_polling_states`: no SSH remote shows "No shared copy, nothing
    to check" and Set up sharing…; `enabled` off shows "Checking is off" with
    no control (K-5);
  - `settings_polling_real_paused` (over a real service): a paused
    repository's eligibility answers `NotEligible`, and its row says Paused;
  - `way_out_check_now`: D5's `LifecycleWayOut::PollOnce` guidance gains a
    Check now button for the item's repository (Document Amendment 3).
  - Keyboard: every control reachable, focus visible (E-2: the page list
    gains a page).
- [ ] Implement the page from the model; no new appearance tests.

**Verify:** the five tests by exact name.

## Task 8: Needs Attention

**Files:** `D/services/scheduler.rs`, `D/services/prompts.rs` (a production
per-key declined clear), `D/view/operations/`, `D/model/operations.rs`,
tests.

- [ ] Test first:
  - `attention_locked_key`: a locked-key attempt gives one `KeyLocked`
    entry for that key, naming its repositories, **no** `Prompt` event, and
    an operation-area entry with Unlock… (O-3, A13);
  - `attention_pause_independent`: pause remains as set, and Unlock… does
    not touch it; resuming does not clear the block (A3);
  - `unlock_after_cancelled_prompt`: after Check now's prompt is cancelled
    (the key marked declined), later polls do not prompt; Unlock… clears the
    key's declined mark with the new production clear, then submits Check
    now, which prompts (Fact 15, C3);
  - `unlock_clears_for_all_repos`: an accepted unlock clears the key's
    attention for every repository using it, and they poll at their stored
    deadlines; a failed unlock returns the attention; an unreachable chosen
    repository leaves the attention (K-26);
  - `attention_host`: a host answer gives Approve host… and clears on
    approval;
  - `attention_actions_by_reason`: `KeyProblem` offers Choose a key…,
    `ConfigurationRequired` offers Open remote settings…, each then submitting
    Check now;
  - `attention_cleared_by_change`: a key or remote change clears (R-11); a
    completed poll clears any attention for its repository, however the
    cause was fixed;
  - `attention_recovery_held`: a held repository shows "Checking is held
    until recovery", and Resume checking calls
    `resume_remote_polling_after_recovery` off the UI thread, which holds
    (U-7, R-13);
  - `two_keys_evict`: with two protected keys, a background job using key B
    evicts key A's cached passphrase and A's attention returns at its next
    poll, as K-23 records (decision 25).
- [ ] Implement keyed attention (U-8 for the key and host); Unlock… submits a
  user-lane Check now.
- [ ] The attention entries wrap inside Activity (decision 13); test
  `attention_actions_hit` clicks each action at its position at the default
  window.
- [ ] **With Wave 04 Cycle 01 on main:** make the polling model a third
  input of `needs_attention(&Operations, &DraftRegistry)`
  (`D/model/attention.rs`), add `attention()`'s count, update its callers,
  and extend its existing test with a row per kind of attention entry, per
  state that does not count, and per R-23 dedupe rule (E-4, A18). Do not
  change how it counts anything else. Without Cycle 01, note in the ledger that its merge adds the
  term.

**Verify:** the ten tests by exact name; with Cycle 01 on main, also
`needs_attention`'s test by exact name.

## Task 9: Unsaved Work Across A Bring-In; Dispositions; Part C Checkpoint

**Files:** tests under `D/view/item/` and `D/services/`; no source change
unless a test fails.

- [ ] Tests (A14, O-4), over a real repository:
  - `bring_in_keeps_dirty_session`: with an open edit session holding
    unsaved edits, the item's file changes on disk and the repository is
    marked changed; the tab reloads and keeps the text with the changed
    notice;
  - `bring_in_kept_draft_banner`: with a kept draft, the changed banner
    shows (`TargetState::Changed`); a clean session takes the new read; a
    save over the moved file answers `ExternalChange` and keeps the edits.
- [ ] Test `dispositions_listed` (A2), one table over `PollAttempt`: each of
  `Dirty`, `Diverged`, `Conflicted`, `Inaccessible`, `BlockedByLocalState`,
  `MaterializationIncomplete`, `Nonconforming`, `Malformed`,
  `RemotelyDeleted`, `HistoryUnknown`, `Unpublished` and `NotReached` is
  listed with its text and calls no mutation. `Current` and `LocalAhead`
  show as A4 says. The text is `guidance()`, except Share… for
  `Unpublished` (R-20). `Closed` and `Promoted` are left to Part E.
- [ ] If an existing path loses text, stop for the owner: it is a D2/D3 defect,
  not D6's to patch silently.
- [ ] Part C checkpoint: merge main; ledger and a ticket comment.

**Verify:** the three tests by exact name.

# Part D: Shutdown

## Task 10: Cancel, The Poll's End, Still Stopping, Join; Part D Checkpoint

**Files:** `D/view/quit.rs`, `D/view/workspace/lifecycle.rs`,
`D/services/worker.rs` (the `JoinHandle` in `Shared`, "shut down and join"),
tests.

- [ ] Test first:
  - `quit_freezes_scheduler`: once `QuitPhase` leaves `Running`, while the
    unkept ask is open or drafts are flushing, no timer fires a poll and no
    loop item is submitted; Cancel resumes the scheduler (R-33);
  - `quit_cancels_poll`: `quit()` cancels a running `Work::Poll` through the
    off-thread cancel, started once per operation however often
    `quit_if_finished` calls `quit()`; queued retirements are dropped;
  - `quit_sees_poll_end`: `Polled` notifies `Operations`, so
    `quit_if_finished` runs; `finishing_text` says "Finishing a check for
    updates" (Fact 16);
  - `quit_polling_still_stopping`: with a poll held on the double's gate,
    `finishing_text` changes to "Still stopping — waiting for Manyhands to
    reach a safe point. Your work is saved." after the injected ten seconds;
    the UI still responds (Quit now works); the worker is still running
    (nothing is cleared or reclaimed at ten seconds);
  - `quit_joins_worker`: the worker thread is joined before `cx.quit()`, both
    when `quit()` answers `Idle` (`finish_job`) and when a `Finishing` job ends
    (`quit_if_finished`), and not by Quit now; the join goes through a seam
    that does not park the GPUI test executor (A9, O-7, Fact 36);
  - `quit_during_restart_wait`: a quit during a restart wait ends without a
    poll;
  - `relaunch_keeps_deadline`: a fresh `Services` over the same data
    directory uses the stored deadline (A7).
- [ ] Implement; the ten-second duration is injected. E-3 changes
  `finishing_text`'s existing test only for the new wording. The new text
  lives only in `finishing_text`; no other view draws stopping feedback
  (R-15).
- [ ] **With Wave 04 Cycle 01 on main:** `quit_polling_still_stopping` also
  hides the Activity panel first and asserts that the layout is unchanged
  (Activity still hidden) and that `quit-finishing` carries the
  still-stopping text (Cycle 01 decision 7, K-18; design decision 12; E-5,
  A18). Cycle 01's own quit test is left alone.
- [ ] Part D checkpoint: ledger and a ticket comment.

**Verify:** the seven tests by exact name.

# Part E: Retiring Completed Contexts

## Task 11a: `ContextRetire`, Answers And Rows

**Files:** `D/services/requests.rs`, `D/services/requests/build.rs`,
`D/services/mod.rs` (`JobKind`), `D/model/operations.rs` (label and stage
arms, the dismissal rule), `D/model/polling/retire.rs` (classification),
tests.

- [ ] Test first, over canned `context_retirement` answers (decision 9):
  - `intent_context_retire`: `Intent::ContextRetire{root, kind, id}` builds
    `Mutation::ContextRetire`, with `stops_at_safe_points: true` and
    `changes_items: true` (Fact 14, U-10);
  - `retire_answer_table`: one table over design E3's rows, state first,
    then code: `pending` leaves the pending row whatever its code; `refused`
    with an unlock, key, host or configuration code gives attention keyed as
    E3 says and no refused-set entry; `refused` with a transport, remote,
    index, external-change, busy, yielding or internal code, or
    `recovery_required` with no removal, is retried at the next completed
    attempt; other `refused` and `preserved` answers join the refused set;
    an answer with no `ContextRetirement` data is retried; `retired` and
    `already_absent` mark the repository changed (A20);
  - `retire_rows_dismissed`: a request answered in an attention or retry row,
    or retired, is dismissed, user-lane ones included; refused and preserved
    ones stay and count; a pending one counts once with its pending row; the
    worker's running operation is not in the pending lists (R-23, E-6);
  - `retire_inflight_entry`: a retirement request's entry (request ID, root,
    kind, id) is written before it is submitted and removed on a terminal
    answer (R-31);
  - `background_retire_never_prompts`: through the real provider with a
    fresh session and a locked key, a background `ContextRetire` emits no
    `Prompt` and answers `unlock_required`, which becomes attention (B5,
    A13; U-12).
- [ ] Add `Intent::ContextRetire` with every arm of Fact 14 (E-1).

**Verify:** the five tests by exact name.

## Task 11b: The Per-Item Loop After A Poll

**Files:** `D/services/scheduler.rs` (the loop), `D/model/polling/` (the
per-item decision and the refused set), `D/view/item/` (the read-only
"Retiring…" state and the composer's unkept source), `D/view/operations/`
(the retirement rows), tests.

- [ ] Test first:
  - `retire_loop_decision`: one table for design E1's per-item decision.
    After a completed attempt, automatic or Check now, each reported `Closed`
    or `Promoted` item gets exactly one background `ContextRetire` under a new
    request ID, unless it already has one queued, running or pending, is in
    the refused set, is being edited (an open edit session, dirty or clean, a
    begin in flight, comment composer text, or repair text in its Inspect
    tab), or has any stored draft of the item or a comment, a live session's
    included. Of three reported items where one is being edited and one has
    a draft, only the third is submitted. Nothing is submitted for an item
    the attempt did not report, and a cancelled or interrupted attempt starts
    no loop (A8, A19, A21; decisions 15, 17, 18);
  - `retire_loop_sequence`: an `IndexPending` attempt's `IndexRefresh` runs
    first; the next item is submitted only when the previous has answered,
    in the attempt's order; the loop goes on past refused, preserved and
    retry answers, and stops for that repository on `pending` and on an
    attention answer (decision 21, R-22);
  - `one_loop_per_repository`: no automatic poll of a repository starts
    while its loop runs; a Check now's attempt replaces the loop's remaining
    items (R-32);
  - `retire_tabs_read_only`: the "Retiring…" state is keyed by (root, kind,
    id), set in the same update as the check, covers a tab opened during the
    retirement and Restore from the Drafts page, and clears on `Finished`,
    `Removed`, `NotSent` and a dropped loop (decision 19);
  - `user_job_waits_for_retirement`: a user job submitted while a background
    retirement runs waits for it, does not cancel it, and runs before the
    rest of the loop (decision 20, K-17);
  - `retire_rows_hit`: the retirement rows wrap inside Activity; each action
    is clicked at its position at the default window (decision 13).
- [ ] Implement the loop, the refused set (R-21), the read-only state, and
  the composer's unkept source.

**Verify:** the six tests by exact name.

## Task 12: Retire…, Resume, Drafts, Replay; Part E Checkpoint

**Files:** `D/services/resends.rs` (the retirement resend), the Retire… row
in `D/view/operations/`, two new asks in `D/view/` beside the quit asks,
tests; other source only where U-9 or U-11 finds a gap.

- [ ] Test first:
  - `retire_left_offer`: Retire… shows on every reported `Closed` or
    `Promoted` item the loop did not retire (skipped, refused, preserved,
    failed before effect, not reached), with design E3's words and short
    code, never a title; not while queued, running or pending. It submits
    `Intent::ContextRetire` on the user lane under a new request ID, and
    removes the item from the refused set (A19);
  - `retire_left_unkept_ask`: with a tab of that item holding unkept text,
    nothing is submitted; the ask offers Show tab (the default, showing the
    tab) and Cancel, with no "Retire anyway" (decision 15);
  - `retire_left_kept_draft_ask`: with a kept draft, the ask warns that a
    closed ticket's drafts become export-only, and for a promoted document
    that its drafts stay restorable but comment drafts can only be exported;
    it offers Retire and Cancel (the default) (decision 18);
  - `retire_resume_resends_original`: a `pending` item's pending row offers
    Resume and Abandon. Resume answering `original_request_required` rebuilds
    `Intent::ContextRetire{root, kind, id}` under the named request ID and
    sends it as a new user-lane request, including after a relaunch, with
    the kind and id from the in-flight record (R-31). Without an entry,
    Resume is not offered and Abandon is (K-27). Abandon opens the existing
    confirmation, which shows the retirement's details (Fact 18). Both work
    by keyboard (A20);
  - `unlock_resends_pending_retirement`: Unlock… for a key that holds back a
    pending retirement resends that retirement on the user lane instead of
    Check now (C3);
  - `retire_end_state_real` (over a real local repository): with primary
    holding the closed ticket and the item's branch and worktree removed,
    the item's open tab reads primary's copy, and a kept item draft and a
    comment draft are listed on the Drafts page and export (A21, U-11). If
    Export fails, stop for the owner: it is a D2 defect;
  - `retire_open_ticket_refused_real` (over a real service): `ContextRetire`
    on an open ticket is refused and removes nothing;
  - `retire_replay_final`: a Final answer replayed under the same request
    removes nothing a second time and adds no second entry (A22);
  - `quit_during_retirement_pending`: a quit while a retirement is held
    leaves a pending row at relaunch, whose Resume finishes it (A22, K-18).
- [ ] Part E checkpoint: merge main; ledger and a ticket comment.

**Verify:** the nine tests by exact name.

# Part F: Evidence

## Task 13: Privacy, Consent And The Call List; Keyboard Journey

**Files:** tests only.

- [ ] `polling_privacy` (A16): a title, a body, a comment and a passphrase
  sentinel never appear in any operation entry, status line, attention
  text, Retire offer, ask or answer, error, or the in-flight record.
- [ ] `polling_consent` (A8): one test drives a full day of the scheduler
  (launch, due, busy, preempt, attention, a `closed` target, an unkept tab, a
  kept draft, quit). It asserts the recording backend saw only design B4's
  calls, the user's own intents, `cancel_poll` only for preemption and quit,
  and a `ContextRetire` only where design E1 allows it: one per reported
  `closed` or `promoted` item that is not being edited and has no draft,
  never inside the automatic poll, and never `PollOnceRetiring`.
- [ ] `polling_keyboard` (A15): open Settings with `secondary-,` (it opens
  the page in both layouts, design W-4), move to the page, change the
  interval, toggle the switch, Check now, then reach Unlock… and a Retire…
  in the operation area (`secondary-4`, which Wave 04 Cycle 01 keeps for
  Activity), by keystrokes alone, focus visible.
- [ ] **With Wave 04 Cycle 01 on main:** `polling_keyboard` reaches the
  operation area as Cycle 01's Activity panel and uses its region names.

**Verify:** the three tests by exact name.

## Task 14: Checklist, Gates, Review-Ready, Gated Closure

**Files:** `docs/plans/2026-11-07-wave-03-desktop-06-on-screen-checklist.md`,
the ledger, the Cycle's Status and Acceptance table, the ticket.

- [ ] Write the native checklist from the design's numbered sections, in the
  D5 checklist's format: before you start (two scratch repositories, a
  second clone for §5, a stalled-server recipe), known limits, sections and a
  results table.
- [ ] Merge main; `git merge-tree --write-tree HEAD origin/main` is clean.
  If Wave 04 Cycle 01 landed after Task 8, 10 or 13 ran, apply their Cycle 01
  steps now and record them.
- [ ] Run, once, the static gates across the tree, the AGENTS.md fast tier,
  and the SSH tier for the changed test file (`remote_polling`). Record
  counts.
- [ ] Update the Cycle: Acceptance table (met / pending native), Status,
  Review Record; the design's Review Record.
- [ ] Mark the ticket review-ready: set its status, commit the `ticket.md`
  edit, `ticket save` against the main checkout, and post a review-ready
  comment with the tests run, rulings, known limits and the native
  checklist.
- [ ] **Gated, after review approval and the owner's go-ahead:** set the
  resolution status, then `devenv shell -- cargo run --locked --bin
  manyhands-cli -- ticket close --repo /home/vhodges/work/src/manyhands --id
  01M4CC0VN3VPVVQ3RNWJKXWEVX` at the owner's terminal, which performs the
  merge (AGENTS.md). Native CI runs after merge and is separately gated.

**Verify:** all gates green; ledger complete; the owner has the checklist.

## Plan Self-Review

- Every Acceptance row maps to a task: A1 native §2 + Task 14; A2 Task 9; A3
  Tasks 2, 8; A4 Tasks 4, 5; A5 Tasks 1, 10; A6 Task 5; A7 Tasks 10, 12; A8
  Tasks 3, 13; A9 Task 10; A10 Tasks 4, 11b; A11 Tasks 2, 5; A12 Task 2; A13
  Tasks 3, 8, 11a; A14 Task 9; A15 Tasks 7, 13; A16 Task 13; A17 Task 5; A18
  Tasks 8, 10, 11b (or Cycle 01, if D6 merges first); A19 Tasks 11b, 12, 14;
  A20 Tasks 11a, 12; A21 Tasks 11b, 12; A22 Tasks 6, 12; A23 Tasks 4, 5.
- Rulings map to tasks: R-1, R-2, R-26, R-29, R-30 Task 3; R-3, R-4, R-24,
  R-25 Task 4; R-5 to R-7 Task 2; R-8, R-14 Tasks 2, 8; R-9 Task 7; R-10,
  R-11, R-16, R-27 Task 5; R-12, R-15 Task 10; R-13 Tasks 6, 7; R-17, R-21,
  R-22 Task 11b; R-18 Tasks 11b, 12; R-19 Tasks 3, 13; R-20 Task 9; R-23 Task
  11a; R-28 Tasks 11b, 12; R-31 Tasks 4, 11a, 12; R-32 Task 11b; R-33 Task 10;
  R-34 Task 4; R-35 Tasks 2, 5.
- No task changes a library file; a library gap stops for the owner.
- Two SSH cases, justified in Task 1 (decision 22); every other test is
  model, worker or desktop-behaviour level, with an injected clock, or over
  a real local service where no network is needed. Retirement adds no SSH
  case: its transport is the library's
  `retirement_cli_authenticated_live_pass`.
- Existing-test edits are named: E-1 (exhaustive matches, Tasks 6, 11a), E-2
  (Settings page list, Task 7), E-3 (`finishing_text`, Task 10), E-6 (D5's
  pending-list tests that listed a running operation, Task 11a), and, with
  Wave 04 Cycle 01 on main, E-4 (its `needs_attention` test, Task 8) and E-5
  (D6's still-stopping test with Activity hidden, Task 10).
