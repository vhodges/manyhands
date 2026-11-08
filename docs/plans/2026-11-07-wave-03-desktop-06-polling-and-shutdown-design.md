---
title: "Wave 03 D6 Desktop Background Polling And Shutdown Design"
date: 2026-11-07
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M6V02CAXQ9CJNRZS0KC9JSW6"
---

# Wave 03 D6 Desktop Background Polling And Shutdown Design

## Intent And Approval Boundary

This design covers the desktop's polling scheduler, its integration with the
one mutation worker, Poll now and the policy controls, the needs-attention
states, preservation of unsaved work, shutdown, and their evidence. Approval
covers the mechanisms, rulings and task order. It does not authorize
implementation (that waits for `01M6KHVD6D0DB8KYT6VGQP10Y0` and the
re-grounding in the Cycle), pushing beyond the ticket branch, merging or
closing.

Revision 1, drafted at `3364370c`, not reviewed. Facts are from current source
unless marked; unverified points are U-questions for Task 0.

Conventions: "Fact n" is a Baseline Fact; `D/` is `src/desktop/`; `R/` is
`src/repository/`; "the handoff" is `docs/design/05-sheets-and-dialogs.md`;
"the RFC" is the application runtime and polling RFC.

## Baseline Facts At `3364370c`

### Library

1. **`poll once` is explicit.** `PollOnceBinding` builds
   `PollPublicationRemoteRequest{ invocation: Explicit, approval: None, restart }`
   (`R/mutation/bind/poll.rs` ~374); `restart` is false in `run` and true in
   `rerun`. It needs no confirmation, skips eligibility (pause and interval do
   not stop it) and refuses a missing publication remote
   (`PublicationRemoteRequired`) or an invalid configuration. Results:
   `Completed` with category `Completed` → `Ok` (or `DiscoveryPending` for
   `IndexPending`); any other category → that category's code; an interrupted
   completion → `RecoveryRequired`, standing `Owed`; `Busy` and `PollYielding`
   are offered as a retry of the same request (`outcome.rs` 593–598, 878–884);
   a transport error with a row stored is final and a new request polls again
   (`outcome.rs` 868–871).
2. **Policy verbs are single-transaction writes.** `PollConfigure{interval_seconds}`,
   `PollPause`, `PollResume` share `PollPolicyBinding` and answer
   `MutationDataDto::PollPolicy{enabled, paused, interval_seconds,
   backoff_seconds, recovery_suspended, next_eligible_at}`; `Ok` if changed,
   `AlreadyApplied` if not; an interval outside 60–3600 is `InvalidInput`. No
   verb sets `enabled`. `PollResume` clears `paused` and `next_eligible_at`
   only; it leaves the recovery suspension, `enabled` and the backoff value.
   A policy verb reports `writing` and never `accepted` (the record is inserted
   and finished with the write).
3. **No boundary path runs an Automatic poll.** Only `Explicit` is built under
   `R/mutation`. A scheduler calls the public
   `RepositoryService::poll_publication_remote(request, &mut SessionCredentials<P>)`
   (`R/remote/poll.rs:1232`) with `PollPublicationRemoteRequest{root,
   operation_id, invocation: Automatic, approval, restart}`. That entry passes
   no probe; `poll_publication_remote_observed` and `CancellationProbe` are
   `pub(in super::super)`. The public stop is
   `RepositoryService::cancel_remote_operation(root, operation_id)`
   (`R/remote/reservation.rs:1295`), which sets `cancel_requested` on the
   active row; the fetch callback and safe points read it, throttled by
   `POLL_PROGRESS_THROTTLE` (250 ms).
4. **Automatic differs in three ways** (`poll_probed`, `poll.rs:1254`+):
   reservation priority is `Poll`, not `Manual`; `automatic_poll_eligibility`
   runs first and returns `Complete(NotEligible{reason, next_eligible_at})`
   without contacting the remote; and authentication runs with
   `automatic=true`, so a locked key fails with `UnlockCancelled` or
   `ProviderUnavailable` instead of prompting
   (`R/transport/operation.rs` ~119).
5. **Dispositions** (`PollDisposition`, `poll.rs:26`): `Current`,
   `FastForwarded`, `LocalAhead`, `Diverged`, `Dirty`, `Conflicted`,
   `Inaccessible`, `Materialized`, `Nonconforming`, `BlockedByLocalState`,
   `MaterializationIncomplete`, `Malformed`, `RemotelyDeleted`,
   `HistoryUnknown`, `NotReached`; each has `guidance()` text. The library
   ticket `01M6KHVD6D0DB8KYT6VGQP10Y0` adds one for a remotely deleted context
   whose item is closed on primary (**not yet on main**; Re-grounding).
6. **Results and errors.** `RemotePollResult::{Complete, IndexPending}` over
   `RemotePollOutcome::{NotEligible{reason, next_eligible_at}, Polled(report)}`;
   `RemotePollReport` exposes `operation_id`, `completion`
   (`Completed|Interrupted|Cancelled`), `completed_at`, `next_eligible_at`,
   `targets` (each `target`, `kind`, `item_id`, `disposition`, `before_oid`,
   `after_oid`) and `outcome` (`Option<RemoteOutcomeCategory>`);
   `local_changes()` is crate-private. `RemotePollError::{Repository,
   Transport, Busy, PollYielding, Interrupted, ConflictPending}`.
   `PollYielding` is what a Manual reservation finds when a Poll row is active
   (it sets `yield_requested`). `Busy` is another operation holding the
   reservation, **or** the fetch meeting a held repository lease; in the lease
   case the row stays `interrupted` with no category and no backoff
   (`poll.rs` ~1440). With an existing non-terminal row and `restart=false`,
   `poll_probed` returns `Err(Interrupted)`; a terminal row returns its stored
   result; `restart=true` resumes with the original target (C5 decision 24).
7. **Policy and eligibility.** `RemotePollingConfiguration` has `enabled`,
   `paused`, `interval` (60–3600 s, default 300), `automatic_backoff`
   (60–900 s, default none) and `recovery_suspended` (`state.rs:1532`).
   `eligibility()` (`poll.rs:998`) checks `Disabled`, `Paused`,
   `RecoverySuspended` in that order, then `NotDue` with
   `next_eligible_at = min(next, now + max(interval, 900 s))` (the clock-
   rollback clamp, written back by the public
   `automatic_poll_eligibility(root)`, `poll.rs:1086`). `next_eligible`
   (`poll.rs:1035`): `Cancelled` keeps the stored deadline; `Completed` adds
   the interval; an `Explicit` failure stores none; an `Automatic`
   `TransportUnavailable` or `ProtocolRejected` adds min(2 × previous backoff,
   900) or 60 s; key, unlock, host-approval, configuration and
   repository-unavailable categories add the interval. Backoff doubles only
   for a Poll-priority transport or protocol failure and clears on `Completed`.
8. **Recovery suspension.** Its only setter is the registry default, the
   presence of the `remote-history-recovery-required` marker
   (`state.rs:39-55`, `recovery.rs:2287`). No poll outcome sets it.
   `resume_remote_polling_after_recovery(root)` (`state.rs:6441`) clears it
   alone and has no caller.
9. **The poll refreshes the index itself.** A completed fast-forward or
   materialization sets `refresh_required`, finishes the row and calls
   `refresh_repository`; a failed refresh answers `IndexPending` (`poll.rs`
   ~1862–1920, 2065). The caller does not refresh; it may retry with an
   `IndexRefresh`.
10. **Read side.** `ReadBoundary::polling_status` (`R/read/status.rs:717`)
    gives `enabled`, `paused`, `interval_seconds`, `backoff_seconds`,
    `recovery_suspended`, `latest_outcome`, `latest_observed_at` (the
    observation batch, not the attempt), `active_operation_id` (any action) and
    `next_eligible_at` (stored, unclamped). It has no attempt time and no
    dispositions; those are on `RepositoryService::remote_snapshot(root)`
    (`latest_attempt_at()`, `poll_dispositions()`, `state.rs:2065-2131`).
11. **Dirty is Git-only.** `Dirty` and `Conflicted` come from
    `sync::require_clean_target` (`sync.rs:580`): foreign Git state files,
    index conflicts, any status entry including untracked. `src/drafts/` is
    outside every repository and no code under `R/remote/` references it, so
    a clean worktree can be fast-forwarded while the desktop holds unsaved
    editor text.

### Desktop

12. **Startup.** `main.rs:6-15` installs the panic hook, calls
    `initialize_git_transport_before_threads`, then `desktop::run()`.
    `run()` (`D/mod.rs:47`) initializes GPUI Kit, opens the service,
    builds `LibraryBackend` and `Services::start` (`D/services/mod.rs:342`),
    which spawns the worker thread `manyhands-worker`. Exactly one main window
    opens; `Services` is `Clone` and dialogs clone it.
13. **The worker.** One thread, two lanes: user first, never preempting, the
    first due background job next. It creates the one `DesktopProvider` and
    the one `SessionCredentials` (`worker.rs:853`). `quit()` closes admission,
    removes queued jobs (sending `Removed`), cancels the running job's token
    and answers `Idle` or `Finishing(id)`. `busy` retries use 100/200/400 ms
    with `std::time::Instant` directly; `Backend` has no time method.
14. **Intents.** The `Intent` enum has no poll variants. Adding one needs an
    arm in `confirmed`, `label`, `root`, `changes_items`, `stops_at_safe_points`
    and `job_kind` (`D/services/requests.rs`), the `JobKind` enum
    (`D/services/mod.rs:124`), `Intent::build`
    (`D/services/requests/build.rs`) and the operations model
    (`Progressing::of`, `stage_text`, `D/model/operations.rs`). `Forward::event`
    marks `accepted` on any progress stage (`worker.rs:1190`).
15. **Prompts.** `DesktopProvider::request_passphrase` runs on the worker,
    consults `DeclinedKeys`, otherwise sends `WorkerEvent::Prompt` and blocks on
    the reply (`D/services/prompts.rs:175`). `Operations` holds one prompt;
    the workspace opens a modal. Lane is never consulted, so a background job
    that needs a passphrase would open the same modal. Replies: `Supplied`,
    `Cancelled` (records the key declined for the session), `Unavailable`
    (quit). Declined state clears only through `clear_declined_for` (callers:
    posting, composer, lifecycle, the "Unlock…" outcome).
16. **Quit.** `Workspace::request_quit` (`D/view/workspace/lifecycle.rs:26`):
    unkept-draft asks, `save_drafts` (flush), then `worker().quit()`; `Idle`
    quits, `Finishing(id)` shows "Finishing {label}" with a **Quit now** button
    (`D/view/quit.rs:70-127`); completion is detected on each operations
    notify (`quit_if_finished`), with no timer. Quit now calls `cx.quit()`
    without joining the worker thread; credentials are dropped only when the
    thread exits. No "still stopping" text exists in `src/`. Timer idiom:
    `cx.background_executor().timer(..)` (`D/view/editor/toolbar.rs:195`,
    `D/services/drafts/flush.rs:183`, `D/services/reads.rs:78`).
17. **Bring-in reaches the screen already.** A `Finished` event with
    `changes_items` marks `Changed{root, items}`; `operations_changed`
    reloads the lists and `reload_tabs(&items)` (`D/view/workspace/requests.rs:123`).
    A clean edit session takes a newer read (`late_read_landed`,
    `D/view/item/edit/late.rs:40`); a session with a kept draft is classified
    (`registry.classify`, `TargetState::Changed`) and shows the changed banner;
    a save against a moved file answers `ExternalChange`.
18. **No poll UI.** Settings has four pages (Repositories, Remotes, Keys,
    Hosts; `D/view/settings/mod.rs:50`, `D/model/stamp.rs` `SettingsPage`). No
    "Poll now", "Check now", footer or Activity exists. `LATER_NOTE` names
    neither polling nor D6. `D/view/confirm.rs:705` carries the comment "No
    desktop path asks the boundary for this preview yet (D5, D6)".
19. **Background submissions today** are all `IndexRefresh`: the launch refresh
    of listed repositories (`D/view/workspace/repositories.rs:66`, gated by
    `launch_refresh`), after a post that left discovery pending
    (`D/view/item/posting.rs:125`) and after `RepoCreate`/`RepoEnable`
    (`worker.rs:1050`).
20. **Test seams.** `Backend` has hooks `read_gate`, `before_job`, `after_job`,
    `on_progress`, canned-answer hooks; `DelayingBackend`
    (`D/test_support/delaying.rs`) holds jobs at stages over a real service.
    Worker tests run on real time with a 10 s wait. UI tests use
    `#[gpui_kit::test]` and `D/view/harness.rs` (`keys`, `click`, `label`).

### Transport and evidence

21. **Initialization.** `initialize_git_transport_before_threads`
    (`src/runtime.rs:25`, once) sets a 10 s connect and 30 s server timeout;
    DNS and total transfer are unbounded. The transport fails closed with
    `RuntimeUninitialized` if it did not run. D6 keeps this order.
22. **Stalls are characterized without cancellation.** Wave 02 Cycle 03:
    handshake, authentication, advertisement and transfer stalls return at
    about 30 s; exec acknowledgement plus cleanup about 45 s
    (`tests/ssh_fixture.rs:660-733`, raw `git2`). Cancellation during DNS,
    connect or handshake is **not** measured; there is no DNS or blackhole
    test. Mid-transfer stop is measured (`poll_progress_stop`, under 2 s).
23. **Harness.** `Stall(Handshake, d)` accepts TCP and sends nothing for `d`;
    `hold_advertisement()` blocks until release; each case runs in its own
    child process, killed after 100 s (`tests/support/ssh_harness.rs`).
    Existing poll cases cover yield, cancel at each phase, public cancel of a
    held advertisement, locked-key no-prompt, backoff cap and materialization
    once (`tests/remote_polling.rs`, `tests/ssh_transport/poll.rs`,
    `tests/cli_remote/polling.rs`).

## API Audit Refresh (Entry-Gate Item 3)

| Operation D6 needs | Entry point | Status |
| --- | --- | --- |
| Automatic poll | `poll_publication_remote`, `Automatic` | Public; no probe (Fact 3) |
| Stop a poll | `cancel_remote_operation(root, op)` | Public (Fact 3); effect on a Poll row is U-6 |
| Eligibility | `automatic_poll_eligibility(root)` | Public (Fact 7) |
| Check now | `Mutation::PollOnce` | C5 binding (Fact 1) |
| Interval, pause, resume | `Mutation::PollConfigure/PollPause/PollResume` | C5 bindings (Fact 2) |
| Clear suspension | `resume_remote_polling_after_recovery` | Public, no caller (Fact 8) |
| Policy read | `ReadBoundary::polling_status` | Public (Fact 10) |
| Attempt time, dispositions | `remote_snapshot(root)` | Public service call (U-3) |
| Index retry | `Mutation::IndexRefresh` | Existing |
| New operation ID | unknown | U-1 |
| "Has a publication remote" | a remote read | U-2 |

## Source Questions For Task 0

- **U-1.** How does the desktop obtain an `OperationId` for a direct call
  (`R/…::OperationId`, public constructor or generator)? If none is public,
  the task stops for the owner: this is a library gap (track rule 5).
- **U-2.** How does the desktop tell a repository has an SSH publication
  remote, and what does `poll_publication_remote` return without one, from a
  repository with a non-SSH remote, and with an invalid configuration?
- **U-3.** Is `remote_snapshot` callable from the backend, cheap, and does
  `latest_attempt_at()` survive a restart? If a restart loses it, "Checked N
  ago" falls back to `polling_status`' stored fields and says "Not checked
  yet" honestly.
- **U-4.** What does a new automatic poll do when an older `interrupted`
  row exists from a killed process (R-4's restart needs the old operation
  ID; a fresh ID may be refused or may supersede)?
- **U-5.** Is `RepositoryService` safe to call from the UI thread for
  `cancel_remote_operation` while the worker thread is inside a poll
  (`Send + Sync`, no lock held across the fetch)?
- **U-6.** Effect of `cancel_remote_operation` on an active `Poll`-priority
  row: the completion (`Cancelled`), whether a deadline or category is stored
  (Fact 7 says `Cancelled` keeps the deadline), and its latency during a
  blocked connect (Task 1 measures).
- **U-7.** Does `resume_remote_polling_after_recovery` hold while the
  history-lost marker file remains, or does the next read re-derive the flag?
  If it re-raises, decision 10's action says what is still needed and does not
  loop.
- **U-8.** Which key and host a repository's remote uses, from a read, so a
  needs-attention state is keyed correctly; and how the desktop learns that a
  supplied passphrase was accepted (the provider's `Supplied` reply is not
  proof).

## Execution Order

Serial, one task at a time: no lane branches, no `AGENTS.md` exception, as
D5. Task 1 (readiness) precedes the worker work so the stopping status rests
on measurements.

## Part A: The Schedule Model (`D/model/polling/`)

Pure, synchronous and clock-injected, so the policy is table-tested without a
worker or a thread.

### A1: Clock seam (R-5)

```rust
pub(crate) trait Clock { fn utc(&self) -> OffsetDateTime; fn mono(&self) -> Instant; }
```

The production clock reads `OffsetDateTime::now_utc()` and `Instant::now()`;
tests advance both by hand. The model never calls either itself.

### A2: State

Per repository (`RepoSchedule`), in memory:

- `policy`: the last `polling_status` read (`enabled`, `paused`,
  `interval_seconds`, `backoff_seconds`, `recovery_suspended`,
  `next_eligible_at`).
- `phase`: `Idle`, `Queued`, `Running{op}`, `Waiting{until: Instant, why}`
  (`Busy`, `Preempted`, `NotDue`), `Attention(Reason)`, `Stopped`.
- `last`: the last `AttemptSummary` (time, outcome, counts of brought-in items,
  problem dispositions), restored at launch from U-3's answer.

`Reason` is `KeyLocked(KeyId)`, `HostUnapproved(Authority)`, `KeyMissing`,
`ConfigurationRequired`, `RecoveryHeld` (R-8). Attention never changes
`policy`.

### A3: Decisions the model makes

- `due(now) -> Vec<Root>`: repositories whose `phase` is `Idle` or whose wait
  elapsed, whose policy is enabled, unpaused, unsuspended and past
  `next_eligible_at`, and with no `Attention`; ordered by earliest
  `last.completed` first (R-7, fairness).
- `next_wake() -> Option<Instant>`: the earliest of every wait and every
  `next_eligible_at` converted to the monotonic clock at arming time.
- `after(attempt)`: moves the phase from the attempt (table below).
- `clock_event(utc_delta, mono_delta)`: if the two differ by more than 60 s
  (a suspend or a jump), every `Waiting` becomes `Idle`, policies are marked
  stale for re-reading, and at most one poll per repository is due, staggered
  one at a time (R-6). No catch-up loop.

### A4: After an attempt

| Attempt answer | Next phase | Status text |
| --- | --- | --- |
| `Completed`, category `Completed` | `Idle`, due at the stored `next_eligible_at` (interval) | Checked just now · up to date / Brought in N changes |
| `Completed`, `TransportUnavailable` / `ProtocolRejected` | `Idle`, due at the stored backoff deadline | Can't reach {repo}. Your work is safe. |
| `Completed`, `UnlockRequired` / `SelectedKeyUnavailable` | `Attention(KeyLocked / KeyMissing)` | Needs unlocking / Choose a key |
| `Completed`, `HostApprovalRequired` | `Attention(HostUnapproved)` | Needs host approval |
| `Completed`, `ConfigurationRequired` / `RepositoryUnavailable` | `Attention(ConfigurationRequired)` / `Idle` at the interval | Can't check |
| `Cancelled` by preemption (R-3) | `Waiting{Preempted}` until the user lane is empty, then `Idle`; no deadline written | Checking soon |
| `Cancelled` by quit or reconfiguration | `Stopped` / `Idle` after re-reading | — |
| `Err(Busy)` or `Err(PollYielding)` | `Waiting{Busy}` at 5 s, 15 s, 60 s (R-4), then the interval; same operation, `restart` | Checking soon |
| `Err(Interrupted)` | resend the same operation with `restart` once (R-4), else treat as `Busy` | Checking soon |
| `Err(ConflictPending)` | `Idle` at the interval | Waiting for the conflict to be resolved |
| `Err(Transport)` / `Err(Repository)` | `Idle` at the library's stored deadline, or the interval if none | Can't reach / Can't check |
| `Complete(NotEligible{reason, next})` | `Idle`/`Waiting` at `next`; `Paused` and `Disabled` show their own text | Paused / Checking is off |

The summary's counts come from the attempt's targets: `FastForwarded` and
`Materialized` count as brought in; `Dirty`, `Diverged`, `Conflicted`,
`Inaccessible`, `BlockedByLocalState`, `MaterializationIncomplete`,
`Nonconforming`, `Malformed`, `RemotelyDeleted`, `HistoryUnknown` and the
library ticket's closed disposition are listed with `guidance()` text under
the operation area's entry. None of them is an error.

## Part B: The Worker (`D/services/`)

### B1: An automatic poll is a `Work` variant (R-1, R-2)

`Work::Poll{ root, operation, restart }` on the background lane. It is not an
`Intent`, because it does not go through the mutation boundary (decision 1).
The worker, holding `SessionCredentials`, calls the backend:

```rust
fn poll_automatic(&self, root: &Path, op: OperationId, restart: bool,
                  session: &mut SessionCredentials<DesktopProvider>) -> PollAttempt;
```

`LibraryBackend` implements it with `automatic_poll_eligibility` followed by
`poll_publication_remote(Automatic)` and converts the library's types to the
desktop's `PollAttempt` (completion, category, `completed_at`, `next_eligible_at`,
targets as `(item_id, kind, disposition)`, `index_pending`). Tests fabricate a
`PollAttempt`; the library's report has private fields and cannot be built
(decision 9). The worker emits `WorkerEvent::Polled(root, PollAttempt)`.

An automatic poll sends no progress events and no `Prompt` event (Fact 4);
its only UI states are Running and its answer.

### B2: Preemption (R-3)

A user job submitted while a `Work::Poll` runs makes the worker's handle call
the backend's `cancel_poll(root, op)` (`cancel_remote_operation`, Fact 3, from
the UI thread; U-5). The poll returns `Cancelled` at its next safe point and
the user job runs. The cost is the safe-point latency: milliseconds in a
transfer (250 ms throttle), up to the transport timeout in a blocked call
(K-1).

A job submitted to the background lane (an `IndexRefresh`) does not cancel a
poll. Quit cancels too (Part D).

### B3: Restart (R-4)

`Busy`, `PollYielding` and `Interrupted` answers resend the **same** operation
ID with `restart=true`, after 5 s, 15 s and 60 s, then fall back to the normal
interval with no failure backoff (the library stores none for these, Fact 6).
The delays are model values under the injected clock, not sleeps.

### B4: What a poll may call

The backend's automatic path calls exactly: `automatic_poll_eligibility`,
`poll_publication_remote`, `remote_snapshot` and `polling_status` reads. A
recording backend asserts it (A8). Nothing else, so nothing can push,
checkpoint, merge or clean up from a poll.

## Part C: Controls And Status

### C1: Intents (decision 5)

`Intent::PollOnce`, `PollConfigure{interval_seconds}`, `PollPause`,
`PollResume` through the boundary, user lane, with every exhaustive match of
Fact 14. `PollOnce` is `stops_at_safe_points: false` in the library's command
table; the worker treats it as accepted-on-first-progress like any other
(Fact 14), and the policy verbs as `writing`-only commands: the "not yet
accepted" branch handles a command that is never accepted. `PollOnce` may
prompt: it is the person's action.

### C2: Settings page "Checking for updates"

A fifth `SettingsPage`. One row per repository (handoff 61–68):

- Name; status line from the model (A4); interval select
  (Every 1, 5, 10, 15, 30, 60 minutes, all inside 60–3600 s); **Check now**
  (**Retry now** after a failure); a switch (on = unpaused).
- A repository with no SSH publication remote: "No shared copy, nothing to
  check" and Set up sharing… (opens D1's remote setup).
- A held repository: "Checking is held until recovery" and Resume checking
  (decision 10).
- A needs-attention row carries Unlock… or Approve host… (R-8).

No global switch (decision 8, K-4). The switch is Pause/Resume because no verb
sets `enabled` (Fact 2); a repository whose `enabled` is off shows "Checking is
off" with no control (K-5).

Check now while a poll is running for that repository joins it (no second
request); while an automatic poll runs for another repository it cancels
nothing: it is a user job, so R-3 applies to it.

### C3: Needs attention (R-8, decision 6)

The operation area (D1) gains one entry per attention state. Its action is the
person's: Unlock… submits a user-lane `PollOnce` for that repository, which
prompts through the existing modal. On a `Supplied` reply the scheduler clears
the key's attention for every repository using that key and makes them due;
on a failed unlock the poll answers `UnlockRequired` again and attention
returns (U-8). Approve host… uses D1's host approval, then the same. A key or
remote change clears the state (R-11). No modal ever opens from an automatic
poll (contract 3).

### C4: Status truthfulness (R-9)

Text comes from the model's `last` and `phase`. At launch `last` is restored
from the library (U-3). The page says "Checking…" only while this process's
worker runs a poll; it does not infer that another process is polling. A poll
by `manyhands-cli` appears only through the stored latest attempt.

## Part D: Shutdown

`Workspace::request_quit` keeps its order: unkept-draft asks, draft flush,
`worker().quit()`. D6 changes three things:

1. **Cancel the poll.** `quit()` also calls the backend's `cancel_poll` for a
   running `Work::Poll`, since an automatic poll has no token (Fact 3), and
   the scheduler stops arming timers.
2. **Still stopping.** On entering `Finishing`, a ten-second timer (injected
   duration; `cx.background_executor().timer`, Fact 16) changes
   `finishing_text` to "Still stopping — waiting for Manyhands to reach a safe
   point. Your work is saved." The UI stays interactive; the Quit now button
   stays (decision 7). Nothing is cleared or reclaimed at ten seconds.
3. **Join.** When `quit()` answers `Idle`, the worker handle is dropped and
   the thread joined (bounded: it is idle) before `cx.quit()`, so a graceful
   exit leaves no worker thread (A9). Quit now is unchanged: it exits at once
   and the library's journals recover the interruption.

On relaunch the stored deadline governs; an `interrupted` row from a quit is
restarted or superseded per U-4.

## Required Outcomes

| # | Outcome |
| --- | --- |
| O-1 | With the app open and a due repository, one automatic poll runs on the background lane and updates lists and tabs. |
| O-2 | A user job submitted during a poll runs after the poll's next safe point, and the poll returns later without failure backoff. |
| O-3 | A locked key or unapproved host produces a needs-attention state and no modal; pause stays as it was. |
| O-4 | A bring-in over an item with unsaved edits or a kept draft leaves the text and shows the changed notice. |
| O-5 | `busy`, `yielding` and `interrupted` answers restart the same operation on the bounded schedule. |
| O-6 | A suspend or clock jump produces at most one poll per repository. |
| O-7 | Quit mid-poll cancels, flushes drafts, shows "still stopping" at ten seconds and exits when the poll returns; no worker thread remains. |
| O-8 | An automatic poll calls no service function beyond Part B4's. |
| O-9 | Configuration changes (interval, pause, resume, remote, key, removal) take effect without a stale poll using the old configuration. |

## Rulings

- **R-1.** An automatic poll is a desktop `Work` variant, not an `Intent`
  (decision 1).
- **R-2.** The desktop has its own `PollAttempt`; library report types are
  converted at the backend (private fields).
- **R-3.** A user job cancels a running poll (decision 3); a background job does
  not.
- **R-4.** The restart schedule 5 s, 15 s, 60 s, then the interval
  (decision 4).
- **R-5.** The model is clock-injected; no test sleeps (AGENTS.md).
- **R-6.** After a suspend or jump, one poll per repository, staggered; the
  library's clamp bounds a future deadline.
- **R-7.** Fair order: earliest last completion first; one repository at a time.
- **R-8.** Attention is keyed by key or host, held in memory, never written to
  policy (decision 6).
- **R-9.** Status never claims more than this process knows.
- **R-10.** Items fast-forwarded or materialized in an attempt become
  `Changed{root, items}`; an `IndexPending` answer queues one background
  `IndexRefresh`.
- **R-11.** A key, remote or registration change cancels that repository's
  poll, clears its attention, re-reads its policy and re-arms; removal drops
  its schedule; a re-added remote leaves a stored pause alone.
- **R-12.** Quit follows Part D (decision 7).
- **R-13.** The page uses only C5's bindings and public reads (decision 5).

## Testing (AGENTS.md Testing Strategy; Decision 9)

Default layer is a unit test. The cost of each layer is the point.

1. **Model unit tests (most of the evidence).** One table-driven test per
   decision: A4's table, `due` ordering, `clock_event`, launch with backoff,
   restart schedule, reconfiguration, pause versus attention. Injected clock,
   no worker, no thread.
2. **Worker tests** over a `Backend` double returning `PollAttempt`s and
   recording calls: preemption, quit mid-poll, no `Prompt`, the B4 call list,
   one scheduler for cloned `Services`, thread join. The double uses the
   existing hold hooks, not sleeps.
3. **Desktop behaviour tests**: the Settings rows' actions and enablement,
   status text for each state, Check now, the switch, the interval, attention
   actions, bring-in preserving an edit, still-stopping at the injected ten
   seconds with the UI interactive, and one keyboard walk. Behaviour only: no
   tests for fonts, icons, layout or that a view "renders".
4. **One SSH-harness case, by exception** (Task 1): `poll_cancel_during_
   handshake_stall` in `tests/remote_polling.rs`. It needs SSH because the
   behaviour is transport-dependent: how long a cancel waits when a blocking
   call has not returned. Its edge cases (preemption, reschedule) are model
   tests. It runs singly, by exact name.
5. **Native** (Linux, owner): the real SSH journey, a second process, a stalled
   server, quit and restart.

Existing tests change only as named edits E-1 to E-3 (exhaustive matches that
now include poll variants, the Settings page list, `finishing_text`); anything
else stops for the owner.

## Privacy

`PollAttempt`, `AttemptSummary` and the status model hold item ids, kinds,
dispositions, counts and times; no title, body, comment or key material. No
type that can hold a passphrase derives `Debug` or `Display`. A sentinel test
fails if a title or a passphrase appears in an operation entry or status text.

## Native Checklist (Linux Wayland, Release Build, Product Owner)

Written in Task 12 from the D5 checklist's shape. Sections: smoke; a real SSH
journey (another clone shares an update; the app brings it in within the
interval; a new context materializes); a second process (`manyhands-cli poll
once` while the app polls); stall (a server that accepts and never answers;
the app stays responsive; quit shows "still stopping"); lock and host
(unlock, pause); restart; keyboard only; results.

## Document Amendments

Proposed with approval:

1. Wave D6 section: note the decisions 1 and 2 (no library part; background
   lane) and that automatic polls need no boundary binding.
2. RFC (application runtime and polling): record the restart schedule and that
   a user job cancels a running poll because the worker is one thread.
3. D5 K-12 ("`PollOnce` is guidance text until D6") is superseded by Part C.
4. D1/D2 quit documentation: D6 replaces the quit path's waiting text.

## Known Limits

- **K-1.** A Save or Share submitted during a poll waits for the poll's safe
  point, up to the transport timeout (about 30 s; about 45 s with an
  acknowledgement stall) when the call is blocked. Same-worker was chosen
  (decision 2); D4 decision 12 accepted the same wait behind a share.
- **K-2.** One stalled repository delays the polls of the others by its stall,
  never the UI.
- **K-3.** The desktop sees a poll by another process only through the stored
  latest attempt.
- **K-4.** No global "pause all" (decision 8).
- **K-5.** `enabled` has no verb; an off repository shows "Checking is off".
- **K-6.** Automatic polls leave no mutation-boundary request record and no
  progress events (decision 1); the library's stored attempt is the record.
- **K-7.** `IndexPending` cannot say whether the poll changed local state
  (`local_changes` is crate-private); the desktop queues one `IndexRefresh`.
- **K-8.** A retained conflict stranded by a raised generation (C5 limit)
  stays; the poll reports `ConflictPending`, shown as waiting.
- **K-9.** Quit now is an interruption, not a rollback; DNS and total transfer
  have no deadline (RFC).
- **K-10.** The handoff's footer and Activity panel are not built; status
  lives on the Settings page and in the operation area.
- **K-11.** Windows and macOS stall and suspend behaviour is unmeasured (G1).

## Alternatives Rejected

- **A library automatic binding** (owner chose direct calls; decision 1).
- **A second poll thread sharing the session** (decision 2): single-owner
  credentials, a second prompt path.
- **Cancelling nothing and letting user jobs wait for the whole poll:** a
  poll of many contexts could hold a Save for minutes.
- **Prompting from the background:** forbidden by the RFC and by Fact 4.
- **Persisting needs-attention:** an unlock is a process fact; it would be
  stale after a restart.

## Risks

- U-1 may find no public `OperationId` constructor: a library gap that stops
  Task 3.
- U-4 may show an old interrupted row blocking new automatic polls.
- Cancel latency under a blocked connect makes K-1 visible to the person.
- The `closed` disposition from the library ticket may change the mapping.

## Review Record

Not yet reviewed. Review rounds follow the D5 pattern: source and
feasibility; contract and consistency; then a scoped adversarial round.
