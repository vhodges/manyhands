---
title: "Wave 03 D6 Execution Ledger"
date: 2026-11-09
status: "in progress"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M70DQFGPA2JQVGZHP64VKKXJ"
---

# Wave 03 D6 Execution Ledger

The durable record of the session(s) executing Wave 03 Cycle D6. The plan is
[the implementation plan](2026-11-07-wave-03-desktop-06-polling-and-shutdown-implementation.md),
the brief is [the handoff](2026-11-09-wave-03-desktop-06-handoff.md). Ticket
`01M4CC0VN3VPVVQ3RNWJKXWEVX`; branch `manyhands/ticket/01M4CC0VN3VPVVQ3RNWJKXWEVX`.

Method: subagent-driven and serial. One fresh implementer per task, a fresh
reviewer after each, then the next task. Implementers never edit this file,
tickets or comments; the orchestrator does.

## Rulings

Recorded as "Ruling: decision — why — cost if wrong".

Ruling (owner, 2026-11-09, Task 0): accept the observer row — after Quit now
or a crash during a `ContextRetire`, the library leaves an active Manual
observer row beside the retirement's local row (`authority.rs:325-348`); the
pending list would show it as an extra Remote `poll` row with no Resume or
Abandon, and polls of that repository answer `Busy` meanwhile. No design
change. Task 12's `quit_during_retirement_pending` shows that Resume of the
retirement's pending row settles the observer (`reentry.rs:135`); if it does
not, Task 12 stops for the owner. Record as a known limit at review-ready —
cost if wrong: one inert-looking pending row until the retirement is resumed.

Rulings made in Task 2 (the implementer's, reviewed; "decision — why — cost if wrong"):

- Ruling: `next_wake(&Now)` and `clock_event(utc_delta, mono_delta)` take the
  reading — the UTC-to-monotonic conversion needs one and the model must read no
  clock — cost: a signature change.
- Ruling: A4's "Err(Transport) key source, selection, endpoint or host trust
  changed" row maps to Idle after re-reading policy, as A4 says, not to the
  categories the library's crate-private `transport_category` gives them (key,
  configuration, host). A4 is the specific row and R-11 backs it — cost: Task
  3 maps by A4 with an **exhaustive match, no wildcard** over the 24
  `SshTransportErrorKind`s (partition: unreachable 4, locked 3, key problem 5,
  host 3, configuration 3, changed 4, unavailable 2). A persistent
  `KeySourceChanged` or `HostTrustChanged` is retried each interval and never
  escalates (a repeat-count escalation can be added later).
- Ruling (changed by review, I-1): attention for a missing, unreadable or
  rejected key is **one entry per key** with its repositories collected
  (`Reason::KeyProblem(KeyId)`); a problem with no key id is per repository
  (`Reason::NoKeySelected(Root)`). This follows Cycle decision 11 and Wave 04
  W-1 ("once per locked or missing key") over A2's type sketch — cost: a
  different grouping if the owner reads A2 literally.
- Ruling: `HostUnidentified(Root)` for a host answer with no authority (A4: "per
  repository if None"); `ConfigurationRequired(Root)` and `RecoveryHeld(Root)`
  carry their root.
- Ruling: `Why::Failed` for `Err(Repository)` other than `recovery_required`;
  `Phase::Preempted` (A2), not `Waiting{Preempted}` as the A4 table wrote it.
- Ruling: a `hold_until` field. A4's "Idle at the interval" rows (conflict
  pending, registry or runtime unavailable, preempted three times) keep `Idle`
  plus a monotonic hold that outlasts a stale stored deadline; otherwise a
  policy re-read would restore the past deadline and loop (R-35) — cost: a
  structural field.
- Ruling: `Err(Repository)` waits carry no operation and resend with
  `restart=false` (no row was recorded, Fact 32); Busy and Interrupted keep
  their operation. Hand-off: the in-flight record's "ours" test (L-1, L-3) uses
  `active_operation_id`, never `restart_of`.
- Ruling: a `stale` flag. `after()` marks the policy stale when the model's
  copy was wrong or the deadline unknown (paused, off, held, Changed,
  reconfiguration and takeover cancels, missing deadline, every clock jump);
  `due()` and `next_wake()` skip stale repositories until `set_policy`.
  **Hand-off risk for Task 5:** re-read `stale()` roots immediately after
  `after`, `clock_event` and `reconfigure`; `next_wake` is `None` while stale.
  Added `abandon(root)` (queued or running to Idle, stale) for a dropped poll
  so the global busy gate cannot silence polling (review I-2).
- Ruling: `Ok(Polled{Interrupted})` and `Err(Interrupted)` are one `Answer`
  (identical A4 rows); Task 3 maps both.
- Ruling: `last` is replaced only by Completed, Transport and Interrupted, and
  by a Cancelled attempt that brought something in or listed dispositions
  (review M-1); only Completed moves `last_completed`.
- Ruling: a retire candidate needs both an item id and a kind, else it is
  listed; Closed and Promoted go to `retire` only from a Completed attempt.
- Ruling: `due()` and `next_wake()` are empty while any poll is queued or
  running, whatever the repository (one worker, R-7) — cost: change if
  concurrent polls are ever allowed.
- Ruling: tests are split over four files by `#[path]` to stay under ~500
  lines, with two extra tests (`attempt_summary_counts`, `preempted_three_times`)
  for decisions the planned eight do not cover.
- Hand-offs (doc comments, no code): read `restart_of()` before `mark_queued`
  (which discards `Waiting.op`); Resume checking must call
  `clear_attention(RecoveryHeld(root))` and `set_policy`; `attention()` order is
  first-root order, so sort in the UI if a stable order is needed.

Rulings made in Task 3 (reviewed; "decision — why — cost if wrong"):

- Ruling: when `poll_eligibility` answers `NotEligible`, the worker skips
  `poll_automatic` and `remote_snapshot` and builds `Answer::NotEligible`
  directly. The call list is then resolve, polling_status, poll_eligibility; an
  eligible poll makes exactly B4's five calls. B4/A8 are an upper bound, so a
  strict subset is within them (the plan's "exactly" reads "at most these, in
  this order"). `automatic_poll_eligibility` already writes the clamped deadline
  in its own transaction, so nothing is lost. Cost: the library replays a
  terminal row under the same operation id before its eligibility check, so a
  same-id restart of a terminal row on a now-ineligible repository gets
  `NotEligible` from the desktop but the stored result from the library;
  acceptable (L-2: no takeover while not eligible).
- Ruling: the Backend poll methods are trait methods with default bodies calling
  `self.service()` (no `LibraryBackend` impl in `poll.rs`); `resolve` calls
  `resolve_repository`, `poll_eligibility` calls `automatic_poll_eligibility`.
- Ruling: deadline precedence is the answer's, else the post-poll
  `remote_snapshot` deadline, else the pre-poll `polling_status` deadline
  (RFC 3339). The model ignores it for Busy, Interrupted, ConflictPending and
  Repository answers (Task 2 `hold_until`).
- Ruling: a read failure before the poll becomes `Answer::Repository` with no
  operation; `recovery_required` only for `ResultCode::RecoveryRequired` or
  kind `RecoveryRequired`.
- Ruling: the cancel cause is asked lazily, by a closure evaluated only when
  the answer is `Cancelled` (review I-1). The worker's closure returns `Quit`
  when the queue is closed, else `Preemption`; Task 4 adds reconfiguration and
  takeover (`CancelCause::{Reconfiguration, Takeover}` are constructed there).
- Ruling: `PollYielding` maps to `Busy` (an automatic poll never answers it).
- Ruling: `Completed` with a replayed category maps by category to the nearest
  `TransportFailure` with no key or authority (unlock -> `KeyLocked{None}`,
  key unavailable -> `KeyProblem` none, host -> `HostUnapproved{None}`,
  configuration, repository unavailable -> `Unavailable`, transport/protocol ->
  `Unreachable`). Cost: a replayed Changed kind is treated as a problem.
- Ruling (review I-2): every report-sourced answer carries `completed_at`,
  including a replayed failure, so Task 4's L-1 can age it against the launch;
  live transport errors and `NotEligible` carry none. The model moves
  `last_completed` only for `Answer::Completed`.
- Ruling: `RuntimeUninitialized` maps to `Unavailable` (A4), not the library's
  `TransportUnavailable`.
- Ruling: the refused-key record is its own map in `DeclinedKeys` (so Unlock…'s
  `clear_for` does not clear a mark the person set), and the background check
  runs before the declined check. A poll keeps one entry (`polling(id)` evicts
  the previous poll's). `refused_for`/`refused_keys` are read by Task 11a.
- Ruling: a `Work::Poll` job calls `before_job(JobKind::Poll)`, sends no
  Queued/Started/Progress, takes a `DesktopRequestId` but is not entered in
  `submitted`.
- Ruling: `classify(result) -> Library` / `attempt_of(Library, ..)` with
  `ReportParts`/`TargetParts`, because the library builds `RemotePollReport` and
  `RepositoryError` only internally; every match is exhaustive with no wildcard
  (24 transport kinds, 18 dispositions, 9 outcome categories, 6 poll errors).

Hand-offs from Task 3 (review M-8 and the report):
- `Work::Poll` has no `CancellationToken`: `WorkerHandle::quit()` does **not**
  stop a running poll. Task 4 builds the cancel task and Task 10 calls
  `cancel_poll` on quit.
- `submit_poll` checks only `closed`; "one poll per repository" is the
  scheduler's (Task 5). A queued poll dropped by `quit()` or `admit` sends only
  `Removed(id)`, never `Polled`, so Task 5/10 must use `abandon(root)` (Task 2).
- `PollInbox` is a `Mutex<Vec>` with no wake-up beyond the drain's end-of-batch
  `cx.notify()` on `Operations`; Task 5 observes that or adds its own notify.
- `#[allow(dead_code)]` on `Backend::cancel_poll` is removed by Task 4.

Rulings made in Task 4 (reviewed; "decision — why — cost if wrong"):

- Design checkpoint (owner, 2026-11-09): P-1 to P-7 and L-1 to L-6 approved as
  written against U-13, U-15 and U-16 (all matched). Every row answered as
  written; no ruling stop.
- Ruling: `Intent::PollOnce{root}` is added now (every Fact 14 arm: not
  confirmed, label "Check now", `changes_items` and `stops_at_safe_points`
  true, `JobKind::PollOnce`, builds `Mutation::PollOnce`), not a seam, because
  `resume_dead_check_now` must show a real request queued under the named
  request id on the user lane. Task 6 still owns the policy verbs, the
  `intent_poll_*` tests, the Check now controls and `PollOnce`'s stage text.
- Ruling: new `Answer::OperationMismatch` / `Library::Mismatch`, handled like
  `Repository{false}` but dropping the entry even when `active_operation_id`
  still names it (L-3). The `classify` arm for the real kind is untested (a
  `RepositoryError` cannot be built outside the library); reviewed as correct
  against `reservation.rs` `mismatch()`.
- Ruling: an entry is **terminal** on Completed, any live Transport, or a
  Cancelled that carries `completed_at`; a desktop-made skip (Cancelled before
  `poll_automatic`, no `completed_at`) keeps the entry; `Unread` drops nothing.
  Otherwise an entry goes whenever `active_operation_id` is not its id.
- Ruling: `active_operation_id` lists only reservation-holding phases, so an
  `interrupted` row reads as `Nothing` and an Interrupted answer drops the
  entry; the model's wait carries the same op for R-4's resend (verified
  against `read/status.rs`). A killed process leaves an active row, so its entry
  stays and is taken over.
- Ruling: the cancel task watches via `PollWatch(Arc<Shared>)`, never a
  `WorkerHandle` clone; first cancel immediate, then 50/100/250/500 ms, then
  1 s repeating; it stops when the poll answers, on `shut_down`, or when the
  worker thread ends (a `WorkerEnd` guard, panic included).
- Ruling: firmness Quit > Reconfiguration > Takeover > Preemption; a second ask
  sends no second event; `asked_cause` returns `Quit` once the queue is closed;
  a Cancelled answer with no ask reads as `Takeover`.
- Ruling: the record lives at `<data>/inflight/{record.json,instance.lock}`;
  writes use `drafts::folder::Folder::replace` (temp, fsync, rename, folder
  fsync); the lock is `OpenOptions` (0600 on unix) plus
  `fs4::try_lock_exclusive`; an unreadable or newer-version record is renamed
  `record.unreadable-<ulid>` and reported once; if the rename fails the record
  is kept, not deleted, and work stays in memory.
- Ruling (review I-1): the replayed-terminal ageing compares `completed_at`
  with the launch time **truncated to whole seconds** (the library stores
  seconds), so a fresh poll finishing in the launch second counts. A replay from
  that same second also counts; accepted, the alternative loses a fresh poll.
- Ruling: `Holding::Attention` (eligible but parked by the model) is not in the
  L-2 table; Task 7 words it.
- Ruling: the preemption table uses a background `IndexRefresh` as a stand-in
  for a background `ContextRetire` (admit looks only at the lane).
- **Ruling (review I-2): `resume_offer` offers Resume for any Remote `poll` row
  with an operation id, no next action and a reservation-holding phase** — the
  read model carries no request id or kind, so a Check now's row cannot be told
  from other rows. What Resume answers: (a) an automatic poll's stale row, or a
  live one in this process: `NotResumable` (`invalid_input`), a dead-end button;
  (b) **a ContextRetire's observer row** (the Task 0 ruling): the retirement
  request holds only the local operation, so Resume answers `invalid_input`;
  this changes the owner's Task 0 picture from "an extra row with no actions"
  to "an extra row whose Resume button fails"; Resume of the retirement's own
  local row (Task 12) still settles it; (c) another process's live Check now:
  `original_request_required` naming its request, and the resend takes that
  request over (the same semantic lifecycle rows have). Task 5 removes (a) by
  giving `Operations` the desktop's own automatic-poll operation ids and
  returning `NotOffered` for them. (b) and (c) have no discriminator before
  Task 7: **report to the owner at the Part B checkpoint**, and record as a
  known limit.

Requirements Task 4 hands to Task 5 (the scheduler entity must do all of
these; the model cannot wedge if it forgets, but polling would be silent):
- call `Backend::resolve` + `polling_status` after EVERY answer and pass
  `active_operation_id` as `scheduler::ActiveRow` to `scheduler::apply`;
- call `InflightStore::try_acquire()` at launch and again on window
  activation; on `Acquired::Now` run L-1 from `store.polls()`;
- supply `launched_at` to `apply`; call `set_policy` after an `abandon` or a
  stale answer; use `abandon(root)` for a poll dropped by quit or `admit`;
- own the exclusion of the desktop's own automatic-poll ids from `resume_offer`
  (I-2 above) and remove the module-level `cfg_attr(not(test),
  allow(dead_code))` on `inflight.rs` and `scheduler.rs`;
- `Reconfiguration` is constructed by Task 5 (`ask_poll_cancel`), `Quit` by Task
  10.

## Existing-test edits

E-1 to E-6 are named in the design's Testing section. Any further edit is
pre-approved (owner, 2026-11-09) and is recorded here as E-7 onward, with its
reason.

- **E-7 (Task 3, E-1 class).** `src/desktop/services/worker_tests.rs` `seen()`
  is an exhaustive match on `WorkerEvent`; it gained
  `Polled(..) => unreachable!("no poll was submitted")`. Reason: the new
  variant. No behaviour change. (Non-test: `model/operations.rs` `apply()` is
  also exhaustive and gained `Polled(..) => {}`; the drain removes a `Polled`
  before `apply`.)
- **E-8 (Task 4, E-1 class).** `requests_tests.rs`: `every_intent` gains
  `PollOnce` (variant count 38 to 39); the `changes_items` range `17..=37` to
  `17..=38` and the `stops_at_safe_points` range `29..=37` to `29..=38`.
  Reason: the new `Intent::PollOnce`. No assertion weakened. (The implementer
  called these E-7 to E-9; renumbered here after Task 3's E-7.)
- **E-9 (Task 4).** `worker_tests.rs` `seen()` gained a `PreemptPoll` arm
  (`unreachable!`). Reason: the new `WorkerEvent` variant.
- **E-10 (Task 4).** `poll_tests.rs` `next()` gained a `PreemptPoll` arm (heard
  as "preempt"). Reason: the new variant (a Task 3 file).

## Task 0: Baseline, references and ledger

### Preflight

- Worktree `.manyhands/worktrees/01M4CC0VN3VPVVQ3RNWJKXWEVX`, branch
  `manyhands/ticket/01M4CC0VN3VPVVQ3RNWJKXWEVX`, clean at the start.
- Session start HEAD: `83fe181a` (`Checkpoint comment 01M70CTVGEMJBGCWTQZ7NJ1WJY`).
- `origin/main` = `f4442618`, which is the main the branch already merged as
  `9512080d`. `git merge-base --is-ancestor origin/main HEAD` holds, so the
  merge is a no-op. `git merge-tree --write-tree HEAD origin/main` is clean.
- Main checkout `/home/vhodges/work/src/manyhands`: `main` level with
  `origin/main` (`f4442618`).
- `AGENTS.md` in the worktree is identical to `origin/main:AGENTS.md`.

### Re-grounding

Nothing moved on main after `9512080d` (origin/main is still `f4442618`), so
the Baseline Facts and the API Audit Refresh stand as written.

### Wave 04 Cycle 01 (`01M6V8RK8B0SBCH0YZXFVSR9E3`)

Not on main. Its ticket is open, in implementation; its branch is at
`1cd7d817`, and its checkpoint comment `01M70C83ZXZDR779DYMG7Q42B0` (Tasks 5 to
9 done; Task 10 quit with Activity hidden, Task 11 keyboard journey and Task 12
gates still to do). It records that it merged main `f4442618` and that D6 is
not on main. W-1 to W-7 cannot be checked against its code yet. Tasks 8, 10 and
13 therefore skip their Cycle 01 steps until it merges (plan, Global
Constraints).

### Baseline static gates

Run once at baseline through Devenv at `83fe181a` (the fast tier is not run
here; AGENTS.md Testing Strategy, Document Amendment 9):

| Gate | Result |
| --- | --- |
| `cargo check --all-features --locked` | pass (51.65 s) |
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | pass (59.62 s) |

### Source questions

Settled by reading source at `83fe181a` (a read-only research pass, with the
U-9 claim re-read by the orchestrator in `authority.rs`). D6 changes no
library file.

- **U-1, U-3, U-12:** still true.
- **U-2: matches** (the design named no function). Call
  `RepositoryService::list_remotes_redacted(&ResolvedRepository) ->
  RemoteListDto` (`R/read/admin.rs:360`). A repository has an SSH publication
  remote when `items.iter().any(|r| r.selected_for_publication &&
  r.publication_eligible)`. `publication_eligible` is true only for an
  `ssh://user@host[:port]/path` or scp-style endpoint that parses for Fetch and
  Push; `file://`, `http(s)://`, none, and an invalid configuration all answer
  `false`, never an error. The read touches no session and is never `busy`. The
  desktop already uses it in `D/view/settings/hosts.rs:373-389`.
  Nit on Fact 31: only the *Automatic* `poll_publication_remote` writes state
  without a remote; `PollOnce`'s `prepare` refuses with no state written.
- **U-9 (rest): differs from the design's silence; settled by the owner, see
  Rulings.** A standalone `ContextRetire` inserts, beside its local
  `retire_context` row, a separate Manual `poll_kind=Observe` remote row with
  its own operation ID (`authority.rs:325-348`). A clean end settles it to
  `interrupted` or `cancelled` (`authority.rs:609, 670`), which is not listed.
  A killed process (Quit now, crash) leaves it active, and it is then listed
  as a Remote-family pending row, action `poll`, a different operation ID, no
  next action (`status.rs:325-332`), and it makes fresh polls answer `Busy`.
- **U-11 (a): matches.** An item whose own-worktree copy is gone reads
  primary's copy (`items.rs:1556-1645`). The pane reloads through
  `Changed.items -> reload_tabs -> read_tab`.
- **U-11 (b): matches.** Export… works for a closed ticket's item draft and for
  comment drafts; it is independent of the restore classification
  (`D/view/drafts/rows.rs:58-72`, `row_actions.rs:159-190`,
  `D/view/drafts_tests.rs:163`). No D2 defect; no stop.
- **U-13: matches.** Mismatch: `RepositoryErrorKind::OperationMismatch`
  (`reservation.rs:207-214`; a priority mismatch is caught by
  `reserve_remote_poll` before `restart_remote_poll`, which checks target and
  poll kind only). A fence on an interrupted row, and a missing row on restart:
  `RepositoryErrorKind::RecoveryRequired`.
- **U-14:** nothing in the fixture stalls a TCP *connect*; every `Fault` acts
  after `accept()`. Task 1 adds a raw listener: a std `TcpListener` on
  `127.0.0.1:0` with `libc::listen(fd, 0)`, never accepted, one or two filler
  non-blocking connections kept alive so further SYNs are dropped. Linux only
  (`cfg(target_os = "linux")`). libgit2's connect timeout is 10 s
  (`src/runtime.rs:30-31`), well inside the harness's 100 s kill. To be
  confirmed by the experiment.
- **U-15: feasible, matches.** A killed `PollOnce` leaves its Manual remote row
  active and its request `accepted`. `OperationResume` on that row answers
  `original_request_required` naming the request. Resending `PollOnce{root}`
  under that ID runs `rerun` with `restart=true` and takes the row over,
  eligibility skipped (`bind/poll.rs:~620-644`). The pending listing carries no
  request ID, and offers no Resume today (`resume_offer` returns `NotOffered`),
  so L-4 needs its change to `resume_offer`. L-4 stands.
- **U-16: matches; no Cargo change.** App data directory:
  `service.data_directory()` (`R/read/credentials.rs:275-279`, as
  `D/mod.rs:99-100` uses for the draft store);
  `drafts::folder::Folder::open_named(data, name)` (`src/drafts/mod.rs:87`) is
  a protected-folder helper. Advisory lock: the existing `fs4 = "0.13"`
  dependency, `fs4::fs_std::FileExt::try_lock_exclusive`, as
  `src/drafts/store.rs:209-218` does; it works on Linux, macOS and Windows and
  is released when the process dies. Toolchain 1.98.1 (so std `File::try_lock`
  also exists, but fs4 is the house pattern).
- **Baseline Facts 12-20 and 36:** spot-checked, no discrepancy.

### Decisions 3–15

Approved as written by the owner on 2026-11-09 (Cycle, Status; ticket comment
`01M70CTVGEMJBGCWTQZ7NJ1WJY`). 16–27 are the owner's. 28 is approved.

## Task log

| Task | Base | Commits | Reviewer verdict | Tests run (exact names) |
| --- | --- | --- | --- | --- |
| 0 | `83fe181a` | `91cabbdb` | — | static gates only (check, fmt, clippy: pass) |
| 1 | `d378aa1c` | `4222708c`, `79c9a981` (review fixes) | APPROVE WITH MINOR; all 8 minors applied (code points 4-7, note points 1-3, 8); fixes not re-reviewed (mechanical) | `remote_polling` `poll_cancel_during_handshake_stall`, `poll_cancel_during_connect_stall`, each singly with `--exact`, 1 case each, pass; clippy all-targets, fmt pass |

## Task 1: Runtime readiness (decision 22, U-14)

- Two SSH cases added to `tests/remote_polling.rs`; research note
  `docs/research/wave-03-d6-transport-stall-cancellation.md`. No library file,
  `tests/support/` file or Cargo file changed.
- **Handshake stall** (server sleeps 45 s): the cancel, sent once the poll's
  row is active and the server has accepted the socket, returned
  `Ok(Polled{Cancelled})` after **about 30.0 to 30.1 s** (30 008, 30 095 and
  30 100 ms on three runs). The cancel does not interrupt the blocked call;
  it is honoured at the next safe point once libgit2's 30 s per-call timeout
  expires. This is inside the documented 30 s bound plus teardown; no stop
  condition. It is the figure behind K-1 and the stopping wording.
- **Connect stall** (U-14): no existing fixture stalls a connect. The case uses
  a `ConnectBlackhole`: a loopback listener with `libc::listen(fd, 0)`, never
  accepted, one filler socket in the single queue slot; the case waits until
  `/proc/net/tcp` shows the poll's socket in `SYN_SENT` to that port before
  cancelling. Cancel-to-return was **about 10.05 s** (10 064, 10 048, 10 048,
  10 040, 10 045 and 10 049 ms), the connect timeout. Stable on six runs; no
  root; **Linux only** (the case is `cfg(target_os = "linux")`; macOS and
  Windows connect-stall behaviour stays G1's, K-11).
- Both cases assert `Cancelled` with no targets, no reservation left, no second
  connection, no push, and an unchanged local snapshot. The bounds are 40 s
  (handshake) and 20 s (connect).
- Unmeasured, as decisions 22 and 24 record (K-20): DNS, authentication and
  teardown or exec-acknowledgement stalls. A cancel at a held advertisement is
  already covered by `poll_public_cancel_held_advertisement`.
- `src/main.rs` still installs the panic hook, then calls
  `initialize_git_transport_before_threads()`, then `desktop::run()`; read, no
  change.
- Stopping wording the status may use: "waiting for Manyhands to reach a safe
  point", never a deadline. Part D's "Still stopping" text stands.
| 2 | `135ba7d8` | `fd284033`, `d80b481d` (review fixes) | APPROVE WITH MINOR; I-1 (key grouping), I-2 (`abandon`), I-3 (row assertions) and M-1/2/3/6/8 applied, M-4/5/7 documented; fixes not re-reviewed | the 10 below, each singly with `--exact`, 1 test each, pass; fmt, check, both clippy pass |

## Task 2: clock seam and schedule model (Part A)

Files: `src/desktop/model/polling.rs` and `polling/{clock,attempt,state,table}.rs`,
four test files (`polling_tests.rs`, `polling_schedule_tests.rs`,
`polling_decisions_tests.rs`, `polling_attention_tests.rs`), `model/mod.rs`.
No library, Cargo or other desktop file changed. `Instant::now` and
`now_utc` appear only in `SystemClock`.

Tests (full paths, all under `desktop::model::polling::tests::`):
`after_attempt_table`, `schedule::attempt_summary_counts`,
`schedule::restart_schedule`, `schedule::preempted_three_times`,
`decisions::due_order_and_launch`, `decisions::next_wake_table`,
`decisions::clock_event_once`, `attention::reconfiguration_table`,
`attention::pause_versus_attention`, `attention::attention_keys`.

## Part A checkpoint (Task 2)

- Main: `origin/main` still `f4442618`, already an ancestor of HEAD; Cycle 01
  (`01M6V8RK8B0SBCH0YZXFVSR9E3`) not on main.
- Tests run: only the ten above, singly. Static gates (fmt, check, clippy
  desktop and all-targets) pass at `d80b481d`. No whole suite, no other target.
- No existing test edited (no E-n yet).
| 3 | `7437d26f` | `cc56515a`, `5a9d1c13` (review fixes) | APPROVE WITH MINOR; I-1 (lazy cancel cause), I-2 (replayed `completed_at`), M-1 to M-7 applied; M-8 handed off; fixes not re-reviewed | `desktop::services::poll_convert::tests::poll_attempt_conversion`, `desktop::services::poll::tests::{worker_poll_background, poll_call_list, background_poll_never_prompts, the_drain_hands_a_poll_to_the_inbox}` (extra), the 10 Task 2 model tests; each singly with `--exact`, 1 test, pass; fmt, check, both clippy pass |

## Task 3: automatic poll on the background lane

Files: `src/desktop/services/{poll.rs,poll_convert.rs,poll_convert_tests.rs,poll_tests.rs,mod.rs,worker.rs,prompts.rs}`,
`src/desktop/test_support/{polling.rs,fixtures.rs,delaying.rs,mod.rs}`,
`src/desktop/model/{operations.rs,polling.rs,polling/attempt.rs}` (one match
arm, markers, a doc comment). Nothing outside `src/desktop/` changed.
Mutation checks run: a lane-blind provider fails `background_poll_never_prompts`;
moving `HostTrustChanged` to `Unreachable`, an eager cancel-cause call, and
`completed_at` only for `Completed` each fail `poll_attempt_conversion`.
| 4 | `1294bcef` | `e2bbc831`, `c2f564eb` (review fixes) | APPROVE WITH MINOR; I-1 (second-truncated ageing), M-1/2/4/5/7 applied; I-2 deferred to Task 5 and reported to the owner; M-3/6/8/9 no change; fixes not re-reviewed | `desktop::services::preempt::tests::{preemption_table, the_drain_cancels_a_preempted_poll}`, `desktop::services::scheduler::tests::{restart_schedule_same_op, conflict_pending_waits, stale_rows_table}`, `desktop::services::inflight::tests::inflight_record`, `desktop::services::resends::tests::resume_dead_check_now`; each singly with `--exact`, 1 test, pass; also Tasks 2-3 tests re-run by the implementer; fmt, check, both clippy pass |

## Task 4: preemption, restart, in-flight record, stale rows

Files (all under `src/desktop/`): new `services/{preempt.rs, preempt_tests.rs,
preempt_rows_tests.rs, inflight.rs, inflight_tests.rs, scheduler.rs,
scheduler_tests.rs, scheduler_rows_tests.rs}`; changed `services/{worker.rs,
mod.rs, poll.rs, poll_convert.rs, requests.rs, requests/build.rs, resends.rs}`,
`model/{operations.rs, polling.rs, polling/attempt.rs, polling/table.rs}`,
`test_support/polling.rs`. Nothing outside `src/desktop/` changed.
Process lapse (recorded, not hidden): the implementer wrote the core code
before the tests; the compensating evidence is a mutation check per row (drop
the pre-poll flag check; end the cancel task on the first `Ok`; admit cancelling
the wrong lanes; steal a row on any Busy; drop the mismatch rule; drop the
old-replay rule; judge "not ours" by the code; drop the lock guard; write the
entry after submit; write as a non-holder; ignore the model's op; drop
`restart=true`; remove the second truncation; remove the shutdown check) —
each caught, two (the lock guard and the write order) only after a stronger
assertion was added.

| 5 | `9b1a9809` | `0a28c07a`, `1a21b7e6` (review fixes) | CHANGES REQUIRED then fixed: I1 (hot loop on a failing policy read), D2 (host scope), M1-M3, M5-M7 applied; M4 skipped; M8 and D1 handed off; the fixes were checked by the orchestrator (two new tests, boundary diff, clean tree), not re-reviewed | `desktop::services::scheduler::{entity_tests::{scheduler_shared_by_clones, polling_switch_off_in_tests, launch_polls_due_ssh_repos, timer_submits_one_at_a_time, result_marks_repository_changed}, real_tests::{reconfiguration_cancels_and_rearms, second_actor_busy, unfinished_operation_waits}, extra_tests::{own_polls_are_not_offered_resume, stale_answer_rereads_policy, activation_takes_the_lock_for_a_stale_poll, clock_event_reads_policies_and_remotes, quitting_starts_nothing, failing_read_waits_for_activation, host_change_reaches_repos_on_the_host}, registration::tests::authority_of_locations, reconfigure::tests::scopes_of_intents}`, `desktop::model::polling::tests::after_attempt_table`; each singly with `--exact`, 1 test, pass; fmt, check, both clippy pass |

## Task 5: the scheduler entity in `Services`

Files (all under `src/desktop/`): new `services/scheduler/{entity.rs, react.rs,
reconfigure.rs, registration.rs}`, `services/{scheduler_entity_tests.rs,
scheduler_extra_tests.rs, scheduler_real_tests.rs, scheduler_support.rs}`,
`model/operations/polls.rs`; changed `services/{mod.rs, scheduler.rs,
inflight.rs, poll.rs, worker.rs}`, `model/{operations.rs, polling*}`,
`view/operations.rs`, `view/workspace/repositories.rs` (one call), `mod.rs`
(`PollingOn` global; window-activation subscription in `root_over_workspace`),
`test_support/{polling.rs, fixtures.rs}`. Nothing outside `src/desktop/`
changed (empty diff over the library, `tests/` and Cargo files).
No existing test edited (no E-n). Both real-service tests
(`second_actor_busy`, `unfinished_operation_waits`) run over a real
`RepositoryService`, a second actor's real reservation and a real stranded
ticket save; no SSH.

Rulings (implementer, accepted):
- Only a repository with an SSH publication remote (`selected_for_publication
  && publication_eligible`, via `list_remotes_redacted`) is in the schedule;
  Task 7 infers "No shared copy" from absence.
- `last_completed` is restored from `polling_status.latest_observed_at`.
- Timer capped at 30 s, 1 s backstop when something is due but blocked.
- R-11 scope: remote and registration intents reach the named repository; a
  host intent reaches the named repository, every scheduled repository with
  the same authority (parsed from the redacted fetch location), every one of
  unknown authority and the `HostUnapproved`-held ones; key intents reach
  every listed repository. This replaced a narrower first rule after review
  (D2): the design says "every repository using that key, host or remote".
- A merely queued poll is not cancelled by a reconfiguration; it answers and
  the stale policy is re-read from that answer (one extra poll at worst).
- "User lane empty" is approximated by "no request in flight", so a background
  `IndexRefresh` also holds a preempted poll.
- The submitted-retirement drop is `end_retiring` plus `TODO(Task 11b)`.
- A policy read that fails is not retried until the next activation, clock
  event or reconfiguration (`failed_reads`, review I1).
- `NotEligible(NotDue)` with a deadline already past goes Idle, stale, held 5 s
  (review M7).
- A failed reconfiguring request still fires `Moment::Finished` (cancel,
  attention cleared, re-read); the design says "when it finishes". Known
  limit K-28: one wasted cancel.

Handed to later tasks:
- Task 7: consume `InflightStore::take_report` and show the unreadable-record
  notice once in Activity (design line 735; review D1: the plan assigned it to
  no task). Task 7 also owns the status reads (`Scheduler::schedule`,
  `holding`).
- Task 8: `Scheduler::clear_attention(&Reason, cx)` = `schedule.clear_attention`,
  then `read_registrations`, then `tick` (both already `pub(super)`).
- Task 10: `Scheduler::set_quitting(bool, cx)` exists; then
  `worker.ask_poll_cancel(CancelCause::Quit)`. A queued poll dropped by
  `quit()` sends only `Removed` and the drain forwards nothing for polls, so
  Task 10 must `schedule.abandon(root)` for in-flight repositories (or add
  `Scheduler::abandon_in_flight()`).
- Task 6: `reconfigure.rs::policy_of(intent)` takes `PollConfigure`,
  `PollPause`, `PollResume`.
- Task 11b: the retirement-drop hook is the `end_retiring` call with a TODO.

I-2 (Resume offered on a row the desktop cannot continue): fixed for the
desktop's own automatic-poll rows (`resume_offer_among`, own set from the
store and the model's restart op). Still offered on a row another process or
the CLI left, a row whose record was set aside, and the U-9 retirement
observer row (see the Task 0 ruling). Known limit K-29; reported to the owner
at the Part B checkpoint.

Review minors not done: M4 (the window-activation subscription and the view's
use of `resume_offer_among` have no test of their own; `resume_offer_among`
itself is tested), M6 in part (`World::quiet` keeps 20 x 10 ms real timers;
`reconfiguration_cancels_and_rearms` takes about 17 s; `services/mod.rs` is
968 lines).

Process: the implementer's first version of the I1 guard also cleared
`failed_reads` on three events; a mutation showed those clears were redundant
and they were removed. Mutations caught: no `ssh_publication`, no timer, `tick`
submits every due repository (after an in-flight assertion was added),
`brought_in` never called (after a split of cases), no `index_pending`
refresh, no cancel on reconfiguration, no `reconfigure` call, stale not
re-read, `plan` ignores the model's op, `recovery_required` polls at the
stored deadline, no acquire on activation, own polls not published,
`quitting` ignored, clock event ignored (after an unscheduled plain repository
was added), the `failed_reads` filter, the host scope, both halves of the SSH
predicate, the M7 row. The launch barrier (`awaiting_launch`) survived and was
deleted as dead.
| 6 | `7d9cd01a` | `35691ba6`, `6106fa5b` (review fixes) | APPROVE WITH MINOR; the Important finding (Check now stage text) applied after the orchestrator read Fact 14 (no owner decision needed); minors 2 and 5 applied; 3 and 4 recorded below; fixes checked by the orchestrator (tree, boundary, stage-words test), not re-reviewed | `desktop::services::requests::poll_tests::intent_poll_build`, `desktop::services::requests::poll_worker_tests::{intent_poll_stoppable, policy_verb_never_accepted, policy_verb_resent}`, `desktop::services::scheduler::intent_tests::poll_once_rereads_status`, `desktop::view::operations::check_tests::a_running_check_now_reads_its_stage_words`, `desktop::view::passphrase::tests::{a_prompt_for_a_check_now_names_no_share, the_dialog_names_what_waits_and_says_not_now}`; each singly with `--exact`, 1 test, pass; fmt, check, clippy pass |

## Task 6: poll intents through the boundary

Files (all under `src/desktop/`): `services/{requests.rs, requests/build.rs,
mod.rs (JobKind), scheduler.rs, scheduler/reconfigure.rs}`,
`model/operations.rs` (`Progressing::Check`, `stage_text`, `progress_words`),
`view/{operations.rs, passphrase.rs}`; new tests `services/{requests_poll_tests.rs,
requests_poll_worker_tests.rs, scheduler_intent_tests.rs}`,
`view/operations/check_tests.rs`; `view/passphrase_tests.rs` (appended). Nothing
outside `src/desktop/` changed.

Existing-test edit **E-11** (`services/requests_tests.rs`, compiler-forced and
helper-count): `variant()` arms 39–41; `every_intent()` gains the three verbs
(39 → 42); `39..=41` joins the false arm of
`changes_items_names_the_intents_that_can_change_items` and of
`syncs_comments_and_resume_stop_at_safe_points_and_saves_do_not`. No assertion
weakened.

Rulings:
- Labels: "Check now", "Set check interval", "Pause checking", "Resume checking".
- Policy verbs: `changes_items` false, `stops_at_safe_points` false (matches the
  library's command table), no stage words (never accepted, Fact 2).
- Deviation from the plan's test line "takes the answer's `PollPolicy`": the
  scheduler re-reads `polling_status` after a policy verb (and after `PollOnce`)
  and does not parse the answer's `PollPolicy`. The design only says the answer
  carries the policy; the re-read also picks up `recovery_suspended` and
  `backoff_seconds`. Task 7 must not look for a parse.
- `Heard::PolicyMoved` fires on any `Finished`, including a failed or
  cancelled verb (one cheap local read; no hot loop, since a read result
  produces no `Finished`).
- Check now stage words: a running Check now reads `Checking · {words}`;
  Accepted/Observing "Getting ready…", Connecting "Connecting to origin…"
  (names the remote), Fetching "Bringing in changes…", UpdatingLocal "Merging
  changes…", Indexing "Updating the list…", Stopping "Stopping at a safe
  point…", Writing falls back to "Getting ready…". Cost if wrong: string edits
  in `stage_text`/`progress_words` and the expected values in `check_tests.rs`.
- `view/passphrase.rs::waiting_share` now accepts only `Share` and `Post`; with
  the new `Progressing::Check` a Check now's prompt would otherwise have been
  titled as a share (caught while adding the variant; own test).

Handed to Task 7: the three verbs carry item-level `Called from Task 7` allows;
`worker.in_flight(|i| matches!(i, Intent::PollOnce{root: r} if *r == root))`
gives "my own Check now is queued or running".

Process: the five planned tests could only fail by compile error first (the
variants did not exist); the mutation runs carry the evidence that they bite
(all caught). The stage-words test was written first and failed for the right
reason; the passphrase test was written after the variant (its guard was the
fix). The table version of `poll_once_rereads_status` was a modification of an
existing test and has no separate red run.
| 7 | `3bb403d5` | `667aaf9b`, `9eee8b82` (review fixes) | APPROVE WITH MINOR; D-1 stopped for the owner and ruled (below); Important (conflict flag clearing unpinned), minor 2 (Check now disabled rule unpinned), 2b (stale allow), 4 (select after a failed re-read) applied; 5, 6, 7, 8 recorded; the fix commit's own report did not arrive, so the orchestrator ran its three changed tests and the gates (below) and did not see its mutation results | `desktop::model::polling::status::tests::{status_text_table, status_detail_for_an_interrupted_check}`, `desktop::view::settings::polling_tests::{settings_polling_rows, settings_polling_controls, settings_polling_states, settings_polling_pending_link}`, `desktop::view::settings::polling_real_tests::{settings_polling_real_paused, settings_polling_unplaced, settings_polling_failed_reread}`, `desktop::view::lifecycle::check_now_tests::way_out_check_now`, `desktop::view::keyboard_tests::settings::checking_for_updates_by_keyboard`, `desktop::view::operations::record_tests::an_unreadable_record_is_reported_once_in_activity`; each singly with `--exact`, 1 test, pass; fmt, check, clippy pass |

## Task 7: Checking for updates page, status text, Check now way-out

Files (all under `src/desktop/`): new `view/settings/{polling.rs, polling_support.rs,
polling_tests.rs, polling_real_tests.rs}`, `model/polling/{status.rs, status_tests.rs}`,
`view/operations/{record.rs, record_tests.rs}`, `view/lifecycle/check_now_tests.rs`;
changed `view/settings/mod.rs` (`PAGES` now 5, `SettingsEvent::ShowActivity`),
`model/{stamp.rs (SettingsPage::Polling), operations.rs, operations/polls.rs,
polling/{state.rs, table.rs}}`, `services/scheduler/entity.rs` (`unplaced`,
`refreshing`, `utc`, `cx.notify()` in `publish` and `tick`, `take_report`
consumed in `publish`), `view/{lifecycle.rs, lifecycle/prepare.rs, workspace.rs,
harness.rs, operations.rs, keyboard_tests/settings.rs}`. Nothing outside
`src/desktop/` changed. Every `Called from Task 7` allow is gone.
No existing test edited (E-2 needed none: the page walks iterate `PAGES`).

**Owner ruling, 2026-11-09 (D-1):** A4 row 3 (unlock cancelled, provider
unavailable or unlock failed, with no key named) said `KeyProblem(None, root)`
and "Needs unlocking". The code, since Task 2, maps it to `NoKeySelected` and
the page says "Choose a key", and `status_text_table` pinned the code. The
reviewer flagged it; I stopped for the owner, who chose to keep the code and
amend A4. A4 row 3 now reads `NoKeySelected(root)` and "Needs unlocking;
Choose a key when no key is named" (design commit `7daf9609`). Cost if wrong:
Task 2's table and `status.rs` would split `KeyLocked{None}` from
`KeyProblem{None}`.

Rulings (implementer, accepted):
- `RepoSchedule.conflict_pending` is a Task 2 addition (A4's "Waiting for the
  conflict to be resolved" is not derivable from phase or `last`): set exactly
  on a ConflictPending answer, cleared by any later answer and by `rearm()`.
  `after_attempt_table` is unchanged. Not a table edit.
- Status-line precedence: no schedule, off, held, own Check now, paused,
  phase. Paused plus attention reads "Paused · Needs unlocking" (A3).
- Wording the docs do not give: "Not checked yet", "Removing finished
  copies…", "Reading the remotes", "Can't check" (unreadable remote read),
  "Checked N ago · up to date", the fifth L-2 sentence "It finishes once what
  needs your attention is dealt with", "Show in Activity" for A4's link.
- Interval labels "Every 1 minute" … "Every 60 minutes"; a stored interval
  outside the six shows as a placeholder.
- Switch and interval stay enabled while a verb is in flight (C5 decision 15).
- A repository with no schedule is told apart: "Reading the remotes", "Can't
  check" or "No shared copy".
- The unreadable-record notice is two fixed sentences with Dismiss, in "This
  session", one entry per report, consumed from `take_report` in
  `Scheduler::publish`.
- Way-out Check now shows only for `LifecycleWayOut::PollOnce`.

Known limits recorded at review, not fixed:
- K-30: `Report::NotSaved` is re-armed by every failed `persist()`, so Dismiss
  hides it only until the next failed write (only `Unreadable` is once).
- K-31: a paused repository whose phase is Waiting{Unfinished} still offers
  "Show in Activity", and one whose phase is Queued or Running reads "Paused"
  while a poll runs (a poll started before the pause).
- K-32: the way-out Check now submits `PollOnce` without consulting the
  schedule; for a repository with no SSH remote the library's refusal shows
  as an ordinary request outcome in Activity.
- `view/settings/mod.rs` is 511 lines.

Handed to Task 8:
- The attention actions (Unlock…, Choose a key…, Approve host…, Open remote
  settings…) and Resume checking are not built. Plug-in points: `PollingPage::row()`
  in `view/settings/polling.rs` (next to the status and detail, before the
  controls; `Shape::Held` is where Resume checking goes), new `PollingEvent`
  variants handled in the `SettingsHost::polling()` subscription in
  `view/settings/mod.rs`, the reason in `repo.phase == Phase::Attention(reason)`
  with `status::attention(reason)` for the words, and `Scheduler::clear_attention`
  (still to add).
- The plan names no file under `view/settings/` for these actions and says
  "Activity" for the entries. Task 8 decides the surface (Settings row,
  Activity, or both) and records it.
- The needs-attention count and Activity wrapping of attention entries are
  Task 8's; the count only once Wave 04 Cycle 01 is on main.

Process: `rows` and the notice test were seen red against a stub before the
code; `controls`, `states`, `real_paused` and the keyboard test were written
first but seen failing only after a stub page was swapped back in;
`status_detail`, `unplaced` and `pending_link` rest on mutation. Mutation
survivors at the first commit: removing only the `disabled` flag or only the
handler guard (fixed in `9eee8b82` by asserting the button is unreachable by
Tab while its own check runs).
| 8 | `74afe9c0` | `fb2f728a`, `6585fe44` (review fixes) | APPROVE WITH MINOR; I-1 (a failed Check now becomes attention; the design's C3 "the poll's answer becomes attention" settled it) and I-2 (the fifth action unreachable; W-6 settled it) fixed, M-1 and M-2 applied; M-3 to M-7 recorded; fixes checked by the orchestrator (three tests, gates, boundary), not re-reviewed | `desktop::services::scheduler::attention_tests::{attention_pause_independent, unlock_after_cancelled_prompt, unlock_clears_for_all_repos, attention_cleared_by_change, two_keys_evict}`, `desktop::services::scheduler::attention_answer_tests::{check_now_failure_becomes_attention, awaited_check_only_after_success}`, `desktop::services::prompts::tests::unlocking_one_key_leaves_another_declined`, `desktop::view::operations::attention_tests::{attention_locked_key, attention_host, attention_actions_by_reason, attention_recovery_held, attention_actions_hit, attention_actions_by_keyboard}`, `desktop::view::settings::attention_tests::settings_polling_attention_actions`, `desktop::model::polling::tests::attention::recovery_hold_follows_the_policy`; each singly with `--exact`, 1 test, pass; fmt, check, both clippy pass |

## Task 8: needs attention

Files (all under `src/desktop/`): new `model/polling/attention.rs`,
`services/scheduler/attention.rs`, `services/{scheduler_attention_tests.rs,
scheduler_attention_answer_tests.rs}`, `view/operations/{attention.rs,
attention_support.rs, attention_tests.rs}`, `view/settings/{polling_attention.rs,
attention.rs, attention_tests.rs}`; changed `model/{polling.rs (sync_recovery),
operations/polls.rs}`, `services/{mod.rs (Backend::resume_polling), worker.rs,
prompts.rs (the per-key declined clear is production), poll_convert.rs
(check_now_failure), scheduler/{entity,react,reconfigure}.rs}`,
`view/{operations.rs, workspace.rs, workspace/render.rs, settings/{polling.rs,
mod.rs}, harness.rs}`, `test_support/polling.rs`, `mod.rs`
(`open_test_window_over`). Nothing outside `src/desktop/` changed. No existing
test edited (no E-12). Every `Called from Task 8` allow is gone.
`needs_attention` (D/model/attention.rs) is untouched.

Rulings (implementer, accepted unless noted):
- Surface: both. Activity entries at the top of "This session" (one wrapped
  row: headline, "Holding back a, b", action; no Dismiss, they go when the
  scheduler clears them) and the same action on the Settings row (design C2, C3).
- Choose a key…, Approve host… and Open remote settings… open the page and arm
  the repository; the Check now is sent when a reconfiguration reaching it
  succeeds (`Moment::Succeeded`; `Finished` still does the rest, K-28 unchanged).
  Arms are pruned when the entry clears and on `forget`.
- Proof is a Check now that succeeds; a completed Check now clears its
  repository's attention, and a key reason clears for every repository sharing
  it. Unlock… itself clears nothing, so a failed unlock or an unreachable
  chosen repository leaves the entry (K-26). Cost: a success on a repository
  with no entry does not clear another repository's key entry.
- A repository the library holds until recovery is listed as attention
  (`sync_recovery`, called when a policy is read); it counts once each through
  `attention()`.
- Unlock… targets the first repository of the key that is on, not held and has
  no pending local operation; with none, it shows as text with its reason.
- A user-lane Check now's transport failure becomes attention through the one
  A4 conversion (`check_now_failure` → `transport_failure` / `attempt_of` →
  `PollSchedule::after`). The code is mapped to one representative
  `SshTransportErrorKind`; the key comes from a per-request record of the keys
  the attempt asked the provider for, the host from `host_data`.
- The operation area has its own `ContentScroll`; each attention action carries
  a scroll marker so Tab scrolls it into view (W-6, E-2).

Known limits (recorded, not fixed):
- **K-33 (library-shaped; reported to the owner at the Part C checkpoint):** a
  Check now's envelope carries a code and `host_data` but not `e.kind`, so
  `HostVerificationUnavailable` (code `recovery_required`) and
  `ProtocolFailure` / `RuntimeUninitialized` (code `internal_error`) cannot be
  told from other causes of those codes. They give no attention and any
  existing entry stays, so after Unlock… a host-verification-unavailable Check
  now leaves the old KeyLocked entry in place until the next automatic poll
  raises the host entry. Closing it needs the library to put `e.kind` in the
  envelope; no ticket was created (not authorized). A KeyLocked or KeyProblem
  code with no prompt asked falls to `NoKeySelected(root)` ("Choose a key") per
  the owner's A4 row 3 ruling.
- K-34 (M-3): Resume checking fails silently: when the library call does not
  release the hold, nothing is shown and the hold stays.
- K-35 (M-5): `two_keys_evict` drives the real provider and `SessionCredentials`
  but injects the final "entry returns" failure with `fail_poll`; a mutation
  that makes the background refusal also mark the key declined is caught only
  by Task 3's `background_poll_never_prompts`.
- K-36 (M-6, M-7): the keyboard test is insensitive to layout; page tests use
  the harness's real settle timers (a red run waits the 60 s `WAIT`).
- Attention entries push a running request's Cancel and Stop buttons down.
- Task 10 must gate `send_awaited_checks`, `unlock` and `resumed` on `quitting`.
- Task 11b needs a setter or a new answer path for a retirement's answer to set
  attention (only `PollSchedule::after` sets it today), and Unlock… does not yet
  resend a pending retirement (design C3, E4); a retirement that got past
  authentication should call `Scheduler::checked(root)`.

**Wave 04 Cycle 01 step (skipped; not on main).** When Cycle 01 merges, the step
adds the polling model as a third input of `needs_attention(&Operations,
&DraftRegistry)` in `D/model/attention.rs`: count `PollSchedule::attention()`'s
entries (one each, however many repositories it holds back; a library-held
repository counts once, ruling above); update the callers; extend its test
(E-4) with a row per reason (`KeyLocked`, `KeyProblem`, `NoKeySelected`,
`HostUnapproved`, `HostUnidentified`, `ConfigurationRequired`, `RecoveryHeld`),
rows for states that do not count (paused, off, unreachable, waiting, not-due, a
Retire… offer) and the three R-23 dedupe rules.

Process: all tests were seen red for the right reason before the code except
`attention_host` and `two_keys_evict` (green when first written; they pin
behaviour Tasks 3 and 5 built, `attention_host`'s UI half was red) and
`recovery_hold_follows_the_policy`, written after the code (a process lapse;
mutation only). Reviewer mutations: disabling the background refusal fails
`attention_locked_key` and `two_keys_evict` (by timeout). Survivor: N6c, the
explicit removal in `forget_schedule` is redundant with the prune on `tick`.
| 9 | `68624a93` | `7ab7674c`, `197b60f2`, `20e337ad` (review fixes) | APPROVE WITH MINOR; Important (the attempt entry's Share… skipped the lifecycle lock) and two minors applied; the rest recorded; the fixes were checked by the orchestrator (three tests, gates, boundary), not re-reviewed | `desktop::view::item::edit_tests::bring_in::{bring_in_keeps_dirty_session, bring_in_kept_draft_banner}`, `desktop::model::polling::entry::tests::{dispositions_listed, a_quiet_attempt_has_no_entry, closed_and_promoted_are_left_to_part_e}`, `desktop::view::operations::attempts_tests::{dispositions_listed, attempt_entry_actions_hit, attempt_entry_share_waits_for_the_lock, attempt_entry_ids_follow_the_repository}`; each singly with `--exact`, 1 test, pass; fmt, check, clippy pass |

## Task 9: unsaved work across a bring-in; dispositions

Files (all under `src/desktop/`): new `view/item/edit_bring_in_tests.rs`,
`model/polling/{entry.rs, entry_tests.rs}`, `view/operations/{attempts.rs,
attempts_tests.rs}`; changed `view/item/edit_tests.rs` (E-12, a 4-line
`mod bring_in` mount, nothing else), `model/polling.rs`
(`PollSchedule::attempts()`), `model/polling/status.rs` (`changes` made
`pub(super)`), `view/operations.rs` (mount and one `.children(attempts)`).
Nothing outside `src/desktop/` changed. No path lost editor or draft text
across a bring-in (reviewer and implementer both drove it).

**Plan gap (recorded, ruled by the orchestrator, not a contract question).**
A2/R-20 say the twelve dispositions are "listed with their text under the
repository's attempt entry" and R-23 defines that entry, but no task built
it: the plan maps R-20 to Task 9 and R-23 to Task 11a, and 11a's tests cover
only the dismissal rules. The implementer stopped; the orchestrator ruled
that Task 9 builds a minimal entry. The plan's R-23 owner line should read
"Task 9 (minimal), Task 11a (retirement rows)".

The minimal entry (`model/polling/entry.rs`, `view/operations/attempts.rs`):
- At most one entry per repository, built from the repository's last
  recorded attempt (so the next replaces it), right after the attention
  entries in "This session". Shown only when it has something to say:
  "Brought in N changes" (N > 0) and one line per listed item (item id, then
  its text).
- Text is `PollDisposition::guidance()`, except `Unpublished` ("This item is
  not shared yet. Share… publishes it to the shared copy."), which carries
  Share… (R-20). `Current` and `LocalAhead` add no line and no entry.
  `Closed` and `Promoted` are skipped (Part E); a test pins the skip.
- Share… sends `Intent::ItemSync{root, kind, id}` on the user lane, as the
  item header's Share does, and waits for the lifecycle lock (review):
  while `lifecycle_lock(..).blocks()` it shows "Share… (Closing…)" or
  "(Merging…)" with no button. Element ids carry the repository key, and each
  action has the Task 8 scroll marker.
- No mutation is called by a disposition.

Rulings: the item shows as a bare 26-character ULID (the short-code lookup
comes with Part E; cost: one lookup in `attempt_line`); the entry has no
Dismiss (R-23 says only "replaced by the next").

Known limits (recorded, not fixed):
- K-37: a transport failure or an Interrupted attempt replaces `last` and
  makes the entry vanish, so a listed Dirty item is not shown until the next
  completed attempt. An Interrupted blip (a Check now interrupting an
  automatic poll) hides it for the 5 s restart. It matches the literal
  wording of R-23 and A2; whether a failed attempt should keep the previous
  listing is a Task 12 design call (the failure slot).
- **For Tasks 11a/11b/12:** Part E must not source Retire… from `last` alone,
  because a failed or interrupted attempt replaces it and drops `retire`.
  Content plugs into `attempt_entry` (`model/polling/entry.rs`), rows into
  `Workspace::attempt_entries` (`view/operations/attempts.rs`).
- Mutation survivors: the `sharing()` guard inside `share_unpublished` is a
  redundant duplicate of the render guard; removing only the session's dirty
  guard in `late_read_landed` is an equivalent mutant (the live-draft guard
  covers it). No true positional-id mutant was built (weaker evidence).
- The new bring-in tests overlap four existing ones (`a_late_read_*`,
  `a_refresh_*_change_banner*`, `an_external_change_*`), all of which start
  from `pane.reload`; the new tests' unique value is the `brought_in` start.

Process: `bring_in_keeps_dirty_session` was first red on a wrong fixture
(a primary-only commit; fixed to write to the context's branch);
`bring_in_kept_draft_banner` was green on first run (mutation evidence);
`attempt_entry_share_waits_for_the_lock` was red only because its ids did not
yet exist, and the "no lock check" mutation is its real evidence.
| 10 | `4b6ae8f2` | `d8e13309` | APPROVE WITH MINOR; no code fix needed (every finding is a ledger record or a hand-off); not re-reviewed (no fix commit) | `desktop::view::quit::tests::{quit_freezes_scheduler, quit_lets_retirement_finish, quit_sees_poll_end, quit_polling_still_stopping, quit_joins_worker, quit_joins_worker_when_a_job_ends, quit_during_restart_wait}`, `desktop::view::item::edit_tests::quit_freeze::quit_freezes_scheduler_for_the_unkept_ask`, `desktop::services::worker::quit::tests::quit_cancels_poll`, `desktop::services::scheduler::quit_tests::{relaunch_keeps_deadline, quit_drops_an_armed_check_now, quit_stops_unlock, quit_stops_resume_checking, quit_withdraws_queued_poll}`; the implementer ran the 14 as one command, the reviewer re-ran 15 singly with `--exact`, 1 test each, pass; fmt, check, both clippy pass |

## Task 10: shutdown (cancel, the poll's end, still stopping, join)

Files (all under `src/desktop/`): new `services/worker/{quit.rs, quit_tests.rs}`,
`services/scheduler_quit_tests.rs`, `model/operations/quit_wait.rs`,
`view/{quit_support.rs, quit_tests.rs, item/edit_quit_freeze_tests.rs}`; changed
`view/{quit.rs, workspace/lifecycle.rs}`, `services/{worker.rs, poll.rs,
scheduler.rs, scheduler/{attention.rs, entity.rs}}`, `model/operations.rs`,
`view/item/edit_tests.rs` (**E-13**, a 4-line `mod quit_freeze` mount, nothing
else). Nothing outside `src/desktop/` changed. `worker.rs` stays 1447 lines.
No `Called from Task 10` allow remains. No `finishing_text` test needed an
E-3 edit (the existing wording did not change). No path loses a draft or
editor text (reviewer checked every exit of the quit flow).

What it does: `Workspace::request_quit` calls `set_quitting(true)` on entry and
Cancel on the unkept ask or the failed-flush ask resumes (R-33). `quit()` calls
`ask_poll_cancel(CancelCause::Quit)` once per operation (later calls only raise
firmness). `finishing_text` says "Finishing a check for updates" for a poll
and, after the injected ten seconds, "Still stopping — waiting for Manyhands to
reach a safe point. Your work is saved." (only in `view/quit.rs`, R-15; the wait
is a GPUI timer, the duration injected). `WorkerHandle::shut_down_and_join`
joins the worker thread on the `Idle` exit (`finish_job`) and when a `Finishing`
job ends (`quit_if_finished`), never from Quit now.

Rulings (accepted by the orchestrator; the design is silent on each, cost
stated):
1. **A queued poll dropped by quit is answered `Polled(Cancelled(Quit),
   completed_at None)`, not `Removed`, and not `schedule.abandon`** (the
   Task 5 hand-off suggested `abandon`; this is a deviation, taken because the
   drain ignores `Removed` for non-requests and the model already has the row
   `Cancelled(Quit)` → `Stopped`). If the app exits before the async answer is
   applied, the in-flight entry stays, as after a crash: on relaunch the
   never-run operation is resent with `restart=true`, the library answers
   `recovery_required` (reservation.rs:~1577), L-3 drops the entry and the
   model waits about 5 s as "Waiting for an unfinished operation", then a new
   operation. One library call, a transient status line, no Activity row, no
   attention entry. Not run as a relaunch test (rests on the L-3 reading).
2. **An armed Check now is dropped while quitting, not kept**; and
   **`resume_checking` is dropped, not queued, while quitting** (the click is
   lost if a failed-flush Cancel follows; the person presses again). Cost:
   small; `resumed` is not gated (it only clears attention and re-reads policy,
   and `tick` cannot send while quitting).
3. **`IndexRefresh` after a poll is not frozen**: it is R-10's refresh, not a
   "loop item"; it cannot make a quit wait unboundedly. Cost: a quit asked just
   before can wait for one background refresh.
4. **Quit keeps cancelling the running token of a non-retire background job**
   (today `IndexRefresh`); only `ContextRetire` is exempt (decision 28, with
   Task 11a). That is existing behaviour.

Deferred / handed on:
- **Decision 28 / R-38 has no effective test yet.** `quit_lets_retirement_finish`
  covers only the generic half (a background `IndexRefresh` held on the
  worker: the quit waits, says "still stopping" at the injected wait, quits on
  return) and could not fail if quit cancelled the token. Do not cite it as
  proving decision 28. **Task 11a must:** add a field to `Running` marking a
  background `ContextRetire`; skip the token cancel in `WorkerHandle::quit` for
  it (`TODO(Task 11a)` marks the line); repoint this test at a held real
  `ContextRetire` and assert its token is not cancelled. Quit now leaving the
  retirement pending is covered by the existing Quit-now tests.
- **Task 11b:** the retirement loop must check `Scheduler::quitting` before each
  item (`TODO(Task 11b)` in `react.rs`); queued retirements are dropped by
  `quit()` through the same removal path.
- **Wave 04 Cycle 01 step (skipped; not on main):** `quit_polling_still_stopping`
  also hides the Activity panel first and asserts the layout is unchanged and
  that `quit-finishing` carries the still-stopping text (Cycle 01 decision 7,
  K-18; design decision 12; E-5, A18). Cycle 01's own quit test is left alone.
  When Cycle 01 merges, add it.

Known limits (recorded, not fixed):
- K-38: removing `thread.join()` is not detected: a woken idle thread exits
  before the assertion runs (3 of 3 runs). The tests prove the thread has ended
  before the app quits (`WorkerHandle::ended()`, `cfg(test)`) and the
  removed-call mutations on both graceful exits are caught. A deterministic
  test needs a negative observation or a timeout, which AGENTS.md forbids; not
  worth requiring (the join is three lines).
- K-39: `Rig::quiet` in `quit_support.rs` duplicates `World::quiet` (a real
  200 ms wait; the tests take 3 to 5 s each); `quit_polling_still_stopping`
  asserts `!cancels().is_empty()` on a background-executor task (timing-
  dependent, but it passed); `QuitWait.after` ships as a production `Option`
  that only tests set.
- The untested `quit != Finishing(id)` check in `still_stopping` is equivalent
  in effect.

Process: the freeze code in `lifecycle.rs` and the queued-poll change in
`worker/quit.rs` were written before their tests (a process lapse); the two
freeze tests and `quit_withdraws_queued_poll` were green on first run and rest
on mutation (reviewer re-ran three: no freeze, no cancel, `Removed` instead
of the answer, each caught). `relaunch_keeps_deadline` was green on first run
(it pins existing behaviour); the reviewer's mutation (ignore the stored
deadline) fails it at "not due at launch", through the real `Scheduler`.

## Part D checkpoint (Task 10)

- Main: `origin/main` unchanged since the Part C checkpoint (`d6789cb9`);
  Cycle 01 (`01M6V8RK8B0SBCH0YZXFVSR9E3`) not on main. See the skipped step above.
- Tests run: only the ones listed in the Task 10 row, by exact path. Static
  gates (fmt, check, both clippy) pass at `d8e13309`. No whole suite, no other
  target.

## Wave 04 Cycle 01 merged (`01M6V8RK8B0SBCH0YZXFVSR9E3`)

Cycle 01 reached `origin/main` after the Task 10 ledger commit
(`eb0798a2`, 118 files, about 15,600 added lines: docks, a footer, Activity as
a dock panel, the keyboard-journey harness). I merged it at `6af92f91`.

- **My error, corrected.** I read `git merge-tree --write-tree HEAD
  origin/main | head -1` as "clean"; that line is only the tree id and hides
  the conflict list. The merge then conflicted in five files. Check the whole
  output (or `--name-only`) before saying a merge is clean.
- **No W item changed its contract** (read-only analysis against Cycle 01's
  merged Cycle doc, design, ticket and 14 comments): W-1 `needs_attention
  (&Operations, &DraftRegistry) -> usize` kept its signature (its only caller
  is `Workspace::attention`, `view/workspace/footer.rs`); W-2 `quit-finishing`
  is still drawn only by `activity_region`; W-3 shared files refactored only;
  W-4 Settings stays a page; W-5 and W-7 unchanged; W-6 Activity about 550 px
  at 1280 px, now **accepted** (K-19, owner, 2026-11-09), no longer
  "proposed". Reword "Cycle 01's proposed K-19" to "accepted K-19" in the D6
  documents at the Task 14 documentation pass (Amendment 8 still applies).
  Task 0's Re-grounding entry therefore gains: `needs_attention`'s signature is
  unchanged; `quit-finishing` is drawn only in `activity_region`; K-19 accepted.
- **Conflicts resolved** in `desktop/mod.rs`, `services/mod.rs`,
  `view/harness.rs`, `view/workspace/lifecycle.rs`, `view/workspace/render.rs`
  by keeping Cycle 01's layout and re-homing D6's pieces: the attention and
  attempt entries sit in `session_section`/`pending_section` (auto-merged), the
  D6 `.track_scroll(self.operations_scroll().handle())` was re-added to the
  `operation-area` in `activity_region`, the D6 quit changes (`set_quitting`,
  `shut_down_and_join`, the ten-second timer) sit beside Cycle 01's
  Activity-displayed guard in `wait_to_quit` and its `dock.write_for_quit`;
  `PollingBackend` gained main's new `Backend::layout`; the harness `start`
  now goes through a private `over(.., services_backend, ..)` so
  `Shell::over_double` keeps `Geometry::Harness`. No Cycle 01 test was edited
  and no D6 assertion was weakened. D6's 1280x840 hit test
  `attention_actions_hit` still clicks every action under main's dock sizes.
- **Steps that were skipped for want of Cycle 01 are now done:**
  - Task 8's step (`00ba1863`): `needs_attention(&Operations, &DraftRegistry,
    &PollSchedule)` adds `polling.attention().len()` (pure; `Workspace::attention`
    passes `services.scheduler().read(cx).schedule()`; the footer already
    repaints through the existing `cx.observe`). **E-14**: the table
    `counts_what_awaits_the_person` gained a `polling` input on `Case` and the
    rows live in the new `model/attention_polling_tests.rs`
    (`mod polling_rows`): one count per entry however many repositories it holds
    back; one each for the seven kinds; zero for paused, off, not-due,
    unreachable, changed, unavailable, busy, interrupted, `recovery_required`,
    other repository failure, conflict-pending, preempted, quit-cancelled,
    completed, nothing registered, a paused or off policy and `Phase::Retiring`;
    a library-held repository counts once; a sum row (1 request + 2 flush
    failures + 2 pending rows + 1 failed read + 2 polling = 8) shows the new
    term adds to the others. Red first (the count lacked the term); the
    dropped-term mutation fails; no survivors. No existing row changed.
  - Task 10's step (`0caa6008`): **E-15**, `quit_polling_still_stopping` now
    hides Activity first (`footer-panel-activity`), asserts the layout and
    `layout_saves()` unchanged and `quit-finishing` not drawn, `finishing_text`
    being the still-stopping text, then presses `secondary-4` and asserts the
    `quit-finishing` label is the still-stopping text. The only moved
    assertion is the label check, which now runs after Activity is revealed.
    Green on first run (the behaviour already existed); mutations (label
    replaced with "Finishing"; `request_quit` revealing Activity) fail it.
    Cycle 01's own quit test is untouched.
  - Task 13's Cycle 01-dependent part is still to do (keyboard journey on the
    new `keyboard_tests/shell*` harness; the privacy test's element list in
    `lifecycle/privacy_tests.rs`, which names `operation-area`). `Shell::open`
    gives a 1590 px window with a 520 px Activity; `open_as_launched` gives the
    real 1280x840 with Activity at 150 px, and nothing in Cycle 01 scrolls a
    focused control into view (D6's `ContentScroll` marker does for D6's own
    actions).
- **R-23 dedupe stays with Task 11a.** The owner's reply on Cycle 01's
  second-review finding 2 (comment `01M717GX6J5YYKM26CWEB3BXZC`) says "Needs
  attention counts both lists as decision 3 says", so a failed request and
  its pending row for one operation count twice today; D6's approved R-23
  rule ("grouped by operation ID and count once; the worker's running
  operation is never listed as pending") deliberately changes that in Task
  11a (design W-1: "`needs_attention` does not deduplicate today"). No test
  pins the double count. Task 11a's table must include a row for D6's own
  in-flight automatic poll's pending row.

## Part D checkpoint (Task 10, with the Cycle 01 merge)

- Main: `origin/main` is `eb0798a2` (Cycle 01 merged); merged at `6af92f91`;
  `git merge-tree --write-tree HEAD origin/main` is clean (checked in full).
- Tests run: only those in the Task 8 and Task 10 rows and the E-14, E-15
  tests above, singly with `--exact`; plus, after the merge, Cycle 01's
  `needs_attention_counts_and_leads_to_activity`,
  `a_waiting_quit_with_activity_hidden_changes_nothing` and the resolver's
  selection of its keyboard and quit tests. Static gates (fmt, check, both
  clippy) pass at `0caa6008`. No whole suite, no other target.

## Task 11a: ContextRetire, its answers and rows

| 11a | `db77c352` | `00fef620`, `b8427692` (review fixes), `e6eb11b0` (record-note fix, not Task 11a) | APPROVE AFTER FIXES; Important I-1 and I-2 and minors M-1, M-3, M-4 (tried, reverted), M-5 applied; fixes checked by the orchestrator, not re-reviewed | `desktop::services::requests::retire_tests::intent_context_retire`, `desktop::model::polling::retire::tests::retire_answer_table`, `desktop::model::attention::tests::retire_rows::retire_rows_dismissed`, `desktop::services::scheduler::retire_tests::{retire_inflight_entry, background_retire_never_prompts, retire_answers_become_attention, retire_left_accepted_is_owed, retire_record_state_reaches_the_answer}`, `desktop::services::worker::retire_tests::{retire_request_twin, quit_spares_a_retirement_token}`, `desktop::view::quit::tests::quit_lets_retirement_finish`, `desktop::view::operations::record_tests::an_unreadable_record_is_reported_once_in_activity`; each singly with `--exact`, 1 test, pass (the orchestrator re-ran `retire_rows_dismissed`, `quit_spares_a_retirement_token`, `quit_lets_retirement_finish`, `retire_left_accepted_is_owed`, `retire_answer_table` and the record-note test); fmt, check --all-features, clippy -D warnings pass |

Files (all under `src/desktop/`): new `model/polling/retire{,_tests}.rs`,
`model/operations/retire.rs`, `model/attention_retire_tests.rs`,
`services/scheduler/retire.rs`, `services/{requests_retire_tests,
worker_retire_tests, scheduler_retire_tests}.rs`, `test_support/retire.rs`;
changed `services/{requests.rs, requests/build.rs, mod.rs, worker.rs,
worker/quit.rs, scheduler.rs, scheduler/{entity.rs, reconfigure.rs}}`,
`model/{operations.rs, attention.rs, polling.rs}`, `view/operations.rs`,
`view/operations/record.rs` (commit `e6eb11b0` only), `test_support/*`.
Nothing outside `src/desktop/` changed; the library-boundary diff is empty.

What it does: `Intent::ContextRetire` / `JobKind::ContextRetire` (label
"Retire local copy", stage words "Retiring · …" through `Progressing::Retire`);
the pure E3 classification (`Verdict::{Retired, Owed, Attention, Unreachable,
Retry, Refused, Failed}`, with `dismissed`, `goes_on`, `changed`,
`authenticated`); the refused set (3 retries join it; `Retire…` removes the
item); the in-flight retirement entry, written before submit and ended on
Finished, Removed and NotSent but kept for Owed; the request twin; attention
keyed per key/host/repository through `PollSchedule::raise`; `Scheduler::checked`
for an answer that passed authentication; `Operations::pending_shown` and the
R-23 dedupe (a request and its pending row count once; a running operation is
never listed as pending; this includes D6's own in-flight automatic poll);
decision 28 (`Running.retirement`, true only for a Background lane
`Execute | GiveUp` of `ContextRetire`; `WorkerHandle::quit` skips the token
cancel for it). Not built, by scope: the loop (11b), Retire…, the draft export,
replay and Resume, the Unlock… resend, and the E3 "what the person sees"
strings (11b / 12 own them).

Existing-test edits:
- **E-16.** `view/quit_tests.rs::quit_lets_retirement_finish` now holds a real
  background `ContextRetire` and asserts `running_job().cancelled == false`
  after `request_quit`. The other half (a non-retire background job and a
  user-lane retirement are still cancelled) is `quit_spares_a_retirement_token`
  in `worker_retire_tests.rs`. Decision 28 is now proved by these two.
- **E-17 (E-1 class).** `requests_tests.rs`: `ContextRetire` appended as
  variant 42; the count 42 to 43; the `changes_items` and `stops_at_safe_points`
  ranges gain `| 42`.
- **E-18.** `model/attention_tests.rs` mounts `retire_rows` and loops over
  `retire_rows::rows()` (additive); the "Task 11a's" comment is replaced.
  Comment-only: `preempt_rows_tests.rs`, `attention_polling_tests.rs`.
- No E-6 edit was needed (no D5 pending-list test broke), and no Cycle 01 test
  pinned the double count of one operation.

Rulings (the implementer's eight, all judged by the reviewer to match the
design or be design-silent; none contradicts a design row; costs):
1. `RecordState::Unknown` is Owed (entry kept, loop stops). Cost: an unreadable
   record shows as pending until Resume replays it.
2. A finished request with no retirement data is `Verdict::Failed`: stays,
   counted, not in the refused set, retried next attempt, and **the loop goes
   on.** **Raise at the 11b checkpoint:** if such an envelope carries an
   attention or transport code the loop should probably stop (R-37).
3. `changed` is true only for `retired`, `already_absent`, or a removal other
   than `none` (Fact 25).
4. `authenticated` is true only for `retired`, `already_absent`, `preserved`.
5. Key for attention: the one recorded refused key (else the one asked key,
   else `Backend::selected_key()`, read off the UI thread through the
   background executor, else `NoKeySelected(root)`, the A4 row-3 ruling). With
   more than one candidate the selected-key read settles it (`recorded_key`).
6. `PollSchedule::raise` skips a repository whose poll is queued or running
   (the poll's own answer meets the same wall).
7. Label and stage words (wording only).
8. A pending row is hidden while any running request holds its operation.

Review fixes (commit `b8427692`): **I-1** the operation falls back to
`envelope.operation_id` for a request left Accepted with no data (it counted
twice; the row was added red first). **I-2** the drain's decision is now
`scheduler::note_record(event, operations, record_of)`; a test-only
`WorkerHandle::force_record_for_testing` makes `retire_left_accepted_is_owed`
drive the real worker and drain with the record left Accepted (deleting the
drain call is now caught). **M-1** an Owed answer takes only an unlock, key or
host need (E3 row 2); **M-3** `recorded_key` replaces `keys.first()` (Task 8's
rule); **M-5** `retire_answer_table` states `goes_on` per row by name. **M-4**:
a refused local remote made no difference (the cost is the world setup and
launch poll, about 7 s even for an existing test); reverted.

Mutations (all caught): the drain call deleted, `note_record` ignoring the
record, the operation taken from data only, Owed taking any need, `recorded_key`
taking the first, the quit skip removed (three tests fail), the skip ignoring
the lane, the E3 table mutations (12), the row-grouping mutations (5), the
in-flight entry mutations (5), the provider-never-prompts mutations (2).

Process: `retire_inflight_entry`, `background_retire_never_prompts`,
`retire_left_accepted_is_owed` and `retire_record_state_reaches_the_answer` were
written after the code and were green first run (mutation evidence);
`retire_answers_become_attention` was an extra test written after the code.
The two M-1 table rows were written with the fix rather than red first.
`intent_context_retire`, `retire_answer_table`, `retire_rows_dismissed`,
`retire_request_twin` and `quit_spares_a_retirement_token` were red first.

The record-note regression (commit `e6eb11b0`, not Task 11a): 
`an_unreadable_record_is_reported_once_in_activity` (Task 7) had been failing
since the Cycle 01 merge, unnoticed because no whole suite had been run after
the merge. Measured cause: the note row was a bare `h_flex()` 807 px wide in a
dock panel whose Operations region ends at x 1150, so Dismiss's centre
(x 1159) was outside it and the harness click missed. Fix: `.flex_wrap()
.items_center()` as the attention rows do (W-6). No behaviour change, no test
edit. **Lesson:** a Task 7 row that predates the narrow panel was not on W-6's
hit-test list; Task 13's keyboard journey and the Task 14 fast tier will cover
the rest.

Known limits (recorded, not fixed):
- K-40 (M-2): a retirement's answer is dismissed even when `PollSchedule::raise`
  cannot raise its attention (the repository is not in the schedule, or a poll
  is queued or running). The running poll raises its own; the unscheduled case
  (polling off, policy read not landed) loses the answer. Unlikely in
  production (a retirement implies an SSH remote with polling on).
- K-41: a retirement answered `busy` or `poll_yielding` after a claim stays
  Accepted in the library and is re-admitted after a backoff; if quit closes
  the queue meanwhile, `readmit` answers `Removed` and R-31 deletes the entry.
  The library row stays pending, so on relaunch only Abandon is offered (K-27).
  Design-literal; narrow.
- K-42: decision 28's skip covers only a **Background** retirement. A
  user-lane Retire…, Resume resend or Unlock… resend still has its token
  cancelled at quit (pinned by `quit_spares_a_retirement_token`). That matches
  the design text; the rationale (a part-way stop leaves the item pending)
  applies equally. **Put to the owner at the 11b checkpoint**, before Task 12
  builds those paths.
- K-43: the world tests take 5 to 22 s each (`retire_answers_become_attention`
  about 22 s, `retire_inflight_entry` about 10 s, `background_retire_never_prompts`
  about 5 s) because `hold_job_return_answering` still runs the library call;
  a refused local remote did not help. A test seam that skips the call when an
  answer is canned would; not built.
- `Retirements.answers` grows for the session; `PollSchedule::raise` repeats the
  "set Phase::Attention" step of `check_failed` and `table.rs` (trivial).
- A flaky `attention_answer_tests::check_now_failure_becomes_attention` (phase
  `Queued`, expected `Idle`) failed once in a 101-test parallel run and passed
  alone; not attributed to this task; watch it at Task 14.

Hand-offs:
- **11b:** submit with `Scheduler::retire(root, kind, item, Lane::Background)`;
  call `Verdict::goes_on()` in `Scheduler::retirement_answered`; use
  `PollSchedule::retirement_refused` (RT-2), `Retiring::Gone` (RT-9),
  `begin_retiring`, `end_retiring`, `drop_queued_retirements` and
  `Scheduler::quitting` (the `TODO(Task 11b)` markers in `react.rs`,
  `reconfigure.rs` and `drop_queued_retirements` remain true). The repository is
  already marked changed by `Operations`, so hold no `Retirement.changed`. The
  "Retired the copy of {item}…" text must come from the attempt entry
  (`Workspace::attempt_entries`), because a retired request is dismissed.
- **12:** `Scheduler::resend_retirement(request_id, root, kind, item, lane)`
  keeps an existing in-flight entry and its operation; `PollSchedule::retire_chosen`
  implements A19; `RetirementEntry::{root, item, operation}` carry
  `Called from Task 12 (Resume)`; `Operations::pending_shown` is the filtered
  list. **Unlock… resending a pending retirement is not built**; a pending
  answer with an unlock, key or host code already raises its attention.

## Part E design checkpoint (Task 11b), 2026-11-10

Run before any 11b code, as the plan requires: the orchestrator had RT-1 to
RT-11 compared with the code at `44ac1bb1` (read-only), then asked the owner.

Where the code cannot support the rows as they stand (Task 11b builds each):
1. Check now's per-item targets never reach the desktop types
   (`Heard::CheckedNow(root)` carries only the root; `PollMutationDto.targets`
   uses a different disposition enum from the one `poll_convert.rs` maps). A
   successful Check now is not an attempt (no `RepoSchedule.last` update), and
   `discovery_pending` from Check now becomes `CheckFailed` without R-10's
   IndexRefresh.
2. No "is this item being edited" function exists and the Scheduler holds
   neither `EditTabs` nor the `DraftRegistry`. Plan: `EditTabs::item_busy`
   (editing, unkept, and a new separate `composing` flag, not added to
   `unkept`, which drives the quit asks and removal counts); the Scheduler is
   handed `Entity<EditTabs>` and a weak registry. Repair text is likely
   vacuous (an Inspect repair offer needs an item with no ID).
3. D2's `stored_for` and `comment_drafts` conflate unknown with none; a new
   tri-state registry query (live sessions plus listing, comments, any holder;
   unavailable or not Ready or never landed means Unknown) is needed. The
   cached listing is not reloaded after `DraftRegistry::begin`, so the check
   also reads `sessions`. The RT-6 fresh listing runs off the UI thread
   behind `ReadKind::Drafts`, and RT-3 to RT-5 are re-checked on the UI thread
   in the same turn as the submit.
4. No "Retiring…" state exists. Plan: derive it from the loop's marks (held in
   `Operations`, where the panes already observe, so the mark appears in the
   same UI update), the worker's in-flight `ContextRetire` and the in-flight
   record entries (so it survives a relaunch), as an extra `LockState` source
   with "Retiring…" words; the composer gets its own read-only guard (the
   lifecycle lock lets typing go on, decision 19 does not).
5. `WorkerHandle::cancel` signals a running retirement; RT-10 needs a new
   remove-only-if-queued call that never signals one.
6. `Phase::Retiring` already stops automatic polls (R-32) but nothing calls
   `begin_retiring` in production and no path guarantees `end_retiring`; it
   only starts from Idle, so Check now (which preempts a poll) could not start
   or replace a loop. The loop state needs a home (a map per root: items,
   current request, marks) and every exit must end it: items exhausted, RT-8
   to RT-11, quit, relist failure, repository forgotten, second instance,
   submit refused.
7. Reconfiguration is caught only at `Moment::Submitted`; `reconfigured` does
   not end a loop that began afterwards.

Owner rulings (AskUserQuestion, 2026-11-10; written into the design as
Document Amendment 11): decision 28 covers any running retirement; an
unreadable draft entry is unknown (skip everything); a no-data answer stops
the loop and `recovery_required` stays as written; a successful Check now is
recorded as the latest attempt and may start or replace a loop.

Decided by the orchestrator under the existing rows (no ruling needed):
user Cancel of a running loop item stops the loop (RT-9); quit drops a frozen
loop (the next completed attempt retries); duplicate targets are checked once;
the loop ends at a reconfiguration's `Finished` as well as `Submitted`; E1's
"due again at once" follows R-35.

Known: a poll of another repository can sit between loop items on the
background lane, and `tick`'s `user_lane_empty` waits until no job is in
flight, so a preempted poll of another repository waits for a running loop.
Recorded as K-44 (accepted; the loop is bounded by its items).

## Task 11b: the per-item loop after a poll

| 11b | `af839b20` | `6c91a8e3`, `f97e8793`, `9c5cfdb8`, `7c8e8f13` (review fixes) | APPROVE AFTER FIXES; Important I-1, I-2, I-4 (a, b, c) and the minors (expect, comments, world-test trim) applied; the fix commit was checked by the orchestrator (four new tests, gates, boundary), not re-reviewed | `desktop::model::polling::retire_loop::tests::retire_loop_table`, `desktop::services::scheduler::loop_tests::{retire_loop_world, retire_loop_exits, retire_listing_not_landed, retire_relist_before_submit, retire_relist_overtaken, a_second_instance_runs_no_loop, user_job_waits_for_retirement, a_pending_retirement_survives_a_relaunch, check_now_is_heard_with_its_attention, a_failed_check_now_ends_the_loop, retire_loop_leaves_what_the_worker_and_the_refused_set_hold}`, `desktop::view::lifecycle::retiring_tests::retire_tabs_read_only`, `desktop::view::item::retiring_tests::{a_composer_begins_no_draft_while_retiring, a_composer_holds_its_item_back_from_the_loop}`, `desktop::view::operations::retiring_tests::retire_rows_hit`, `desktop::services::worker::retire_tests::{cancel_queued_never_signals_a_running_one, quit_spares_a_retirement_token}`, plus the unit tests named in the commits; each singly with `--exact`, 1 test, pass (the orchestrator re-ran `retire_loop_table`, `retire_relist_before_submit`, `retire_tabs_read_only`, `retire_rows_hit`, `quit_spares_a_retirement_token` and the four fix tests); nine Activity-layout-sensitive existing tests pass singly (quit, keyboard and operations tests); fmt, check --all-features, clippy -D warnings pass |

Owner rulings made at the design checkpoint (2026-11-10) are in the section
above and Document Amendment 11. All four are built: decision 28 covers any
running `ContextRetire` (E-19); an unreadable draft entry is unknown; a
no-data answer stops the loop (E-20); a successful Check now is the latest
attempt and may start or replace a loop.

What it builds (all under `src/desktop/`; the library-boundary diff is empty):
- **Loop** (`model/polling/retire_loop.rs`, `services/scheduler/retire_loop.rs`):
  one per repository, started from `Scheduler::answered` after R-10's
  IndexRefresh and from Check now; RT-1..RT-11 decided by a pure `Facts` table;
  the "Retiring…" mark is set before the relist and cleared only after the
  worker queue holds the job; the recheck and submit run in one UI-thread turn
  against sessions, pane state (`EditTabs::item_busy`: editing, unkept,
  composing) and the new tri-state `DraftRegistry::held_by` (live sessions
  first; unavailable store, listing not landed or not Ready, or any unreadable
  entry is Unknown). Every exit ends the loop and its marks (`end_loop`).
- **Check now as an attempt** (`poll_convert.rs::check_now_attempt`,
  `checked_now`): targets in report order; `checked()` before the loop starts;
  `discovery_pending` is heard as `CheckedNow` with `index_pending`.
- **Read-only "Retiring…"** (decision 19): marks (in `Operations`) union the
  worker's in-flight `ContextRetire` union the in-flight record, as a
  `LockState` source through `Services::lock`; covers a tab in a second window,
  Restore from the Drafts page, Share… and the composer guard (new comment
  drafts).
- **Worker**: `cancel_queued` (never signals a running job); `Running.retirement`
  is lane-independent.
- **Attempt entry** gains retirement lines ("Retiring {item}…", "Retired the
  copy of {item}…", "Retiring {item} didn't finish.").
- **Test seam** `Backend::instead_of_job` (production trait method, default
  `None`, like `before_job`/`after_job`) lets a world test skip the real library
  call.

Existing-test edits: **E-19** `worker/retire_tests::quit_spares_a_retirement_token`
(user-lane retirement now not cancelled, plus a user-lane refresh row still
cancelled); **E-20** `retire::tests::retire_answer_table` (`goes_on`, the two
no-data rows now stop); **E-21** `scheduler_retire_tests` helper `retire()`
expects `Some` (the call refuses while quitting); **E-22** `Retiring::Gone`
gains `id`; **E-23** `entry_tests.rs` (eight calls take `&[]` retirement
lines); **E-24** `held_tests::relist_reads_the_store_behind_its_gate` (asserts
`Fresh::{Landed, Overtaken}`); **E-25** `retire_loop_exits` passes
`Fresh::Failed`; **E-26** `retire_loop_table` uses `begin` plus `get_mut`. No
assertion weakened.

Review fixes (`7c8e8f13`): **I-1** a failing Check now ended no loop and left
the marks (the phase was overwritten): `check_failed` now ends the loop first;
`loops_consistent` also asserts marks belong to loops and match the request
model's. **I-2** an overtaken fresh listing ended the whole loop: now only a
failed listing does, an overtaken one skips the item (RT-6). **I-4** three
surviving mutations now caught (RT-1 worker half, RT-2 wiring, RT-3 composer).
`RetireLoops::begin` returns the replaced loop (no `expect`).
`retire_loop_world` is 3 rows, 14.2 s (was 37.9 s); `retire_loop_exits` is
about 71 s (not merged). Nine reported mutations, all caught.

Rulings by the implementer (judged by the reviewer to match or be design-silent;
none needed an owner ruling):
1. A Check now that is not `Completed` records no attempt and starts no loop.
2. A Check now while a poll is queued or running records `last` and
   `last_completed` and starts no loop (R-32: a loop and a poll cannot
   coexist); its items wait for the poll's own attempt or a Retire…. **K-45.**
3. `discovery_pending` from Check now is `CheckedNow` with `index_pending`.
4. A failed listing ends the loop; an overtaken one only skips the item.
5. An answer of any retirement of the repository that hits a wall stops its
   loop. 6. A user Stop of a loop item stops the loop (RT-9 analogue).
7. A quit asked drops every loop at once.
8. `{item}` in the new rows is the **last 8 characters of the ULID**, not the
   ticket slug E3 names (`RetireLine` has no slug); no privacy issue (no
   title or path). **K-46.**
9. A reconfiguration at Submitted removes queued `ContextRetire` for the
   reached repositories, user lane included (R-11 literally); see Task 12 below.
10. Lock precedence: a lifecycle lock wins over Retiring…; Retiring is not in
    `run_locks`, so a clean edit session is not ended by a retirement lock (the
    loop never retires an item with a session; Task 12's Retire… must flush a
    clean session itself, E4 step 4).
11. A settled retirement line keeps the place it began in.

Process: the model table was written before the model; every world test and
the composer/worker/refused-set tests were written after the code and were
green first run (mutation evidence; 42 mutations in the first round, 9 in the
fix round, recorded in the commit messages only). `retire_rows_hit` was red
first (caught a 624 px row in a 532 px area). `a_failed_check_now_ends_the_loop`
and `retire_relist_overtaken` were red first.

Activity layout change (deliberate, outside the brief): `min_w_0` on the
session and pending columns (`view/workspace/render.rs`) so a long row shrinks
inside its column instead of widening the area, plus `.test_support()` on the
`operation-area` div. It affects the three existing unwrapped rows Cycle 01
left to Wave 04 Cycle 03 (design K-16, Amendment 8): they now clip or overflow
inside their column instead of widening it. **Task 14 must update K-16 and
Amendment 8's wording.**

Known limits (recorded, not fixed):
- K-44 (checkpoint): a poll of another repository can sit between loop items;
  `tick`'s `user_lane_empty` waits until nothing is in flight.
- K-45: see ruling 2. K-46: see ruling 8.
- K-47: the relist has no timeout: a hung draft read parks the repository's
  polls and holds the item read-only until a Check now, a reconfiguration or a
  quit.
- K-48: the composer's `locked` flag is applied at the next paint while the
  edit paths read the lock live; a keystroke between the submit and the next
  frame could begin a comment draft on an item being retired. The draft
  survives (E5); no text is lost.
- K-49: `begin_retiring` accepts Preempted and Waiting, but both are
  unreachable (`apply` has already moved the phase to Idle).
- K-50: the kind converters are duplicated five or six times (`dto_kind`,
  `authoring_kind`, `target_kind`, `retire_kind`, inline maps) and the `Probe`
  construction is copy-pasted in `drive` and `relisted`; `RetireLine::text`
  re-implements `view/lifecycle::short_id`.
- A lifecycle dialog (Close…) open on an item a peer closed does not hold the
  loop back; no text is lost but the dialog's run will fail.
- `retire_loop_exits` (about 71 s) still builds ten worlds (K-43 class).

Hand-offs to Task 12:
- **I-3 (decide in Task 12):** `drop_queued_retirements` removes queued
  user-lane `ContextRetire` too. A queued Resume resend of an Accepted
  (pending) retirement that is removed answers `Gone(Removed)` and
  `retirement_ended(.., Removed)` deletes its in-flight entry, so the pending
  item loses its "Retiring…" hold and its Resume. Option: a `Removed` resend
  must not end an entry already Accepted.
- Retire…: `Operations::mark_retiring(key)`, then `Scheduler::retire(root,
  kind, item, Lane::User)` (returns `None` while quitting), then
  `unmark_retiring(mark)`; `Services::lock`/`Services::retiring` hold the item
  read-only; E4's list is the attempt's `summary.retire` minus the
  `RetireState::Retired` lines (`skipped()` is test-only: do not source from
  it); `PollSchedule::retire_chosen` and the refused set are Task 12's.
- Resume/Unlock… resends: `Scheduler::resend_retirement` already marks the
  entry's "Retiring…" line; `Retiring::Gone` carries `id`.
- Reuse `EditTabs::item_busy`, `DraftRegistry::held_by`/`relist`,
  `Composer::set_locked`/`locked_text`.
- A pane-driven Retire… must do its `item_busy` check at the view level (a
  `scheduler.update(..)` that reads a pane from inside that pane's update
  panics).
- Pending state reaches the lock through `Scheduler::publish`, which runs only
  with polling on (R-27: always, in production).

## Task 12a: Retire…, the two asks, Resume/Abandon, Unlock…

| 12a | `50a1823b` | `71951ebb`, `728e4c9d`, `9881c8a0` (review fixes) | APPROVE AFTER FIXES; Important I-A, I-B, I-D, I-E, I-F and the minors applied, I-C fixed by an orchestrator ruling; the fix commit was checked by the orchestrator (five new tests, gates, boundary), not re-reviewed: **the final Task 12 review covers `services/scheduler/reconcile.rs` too** | `desktop::view::operations::retire_tests::{retire_left_offer, retire_left_unkept_ask, retire_left_kept_draft_ask, retire_pending_row_by_keyboard, retire_flush_failure_submits_nothing, retire_after_a_quit_closed_its_question, retire_unkept_text_is_asked_before_drafts}`, `desktop::services::scheduler::retire_tests::{retire_resume_resends_original, unlock_picks_reachable, retire_chosen_checks_rt1_again, retire_entry_ends_when_its_row_was_settled_elsewhere, retire_entries_reconcile_only_from_a_complete_list, retire_entry_the_worker_holds_is_not_reconciled, retire_pending_ended_says_only_what_happened}`, `desktop::services::scheduler::reconcile::tests::a_row_is_gone_only_from_a_complete_list_without_it`, `desktop::services::scheduler::loop_tests::{retire_removed_resend_keeps_the_hold, retire_offers_leave_out_the_loops_items, the_loop_skips_an_item_already_retired}`, `desktop::view::item::edit_tests::a_retirement_ends_a_clean_edit_session`, and the model and service unit tests named in the commits; each singly with `--exact`, 1 test, pass (the orchestrator re-ran `retire_left_offer`, `retire_left_unkept_ask`, `retire_resume_resends_original`, `unlock_picks_reachable`, `a_retirement_ends_a_clean_edit_session` and five of the fix tests); fmt, check --all-features, clippy -D warnings pass |

Files (all under `src/desktop/`; library-boundary diff empty): new
`view/operations/retire.rs`, `view/retire_asks.rs`,
`view/operations/retire_tests.rs`, `services/scheduler/reconcile.rs`; changed
the model (`operations*`, `polling*`, `polling/entry.rs`), services (`mod.rs`,
`resends.rs`, `inflight.rs`, `edit_tabs.rs`, `drafts*`, `scheduler*`), the
views (`operations*`, `workspace*`, `item/lock.rs`, `settings/polling_support.rs`).
No `Called from Task 12` allow remains.

What it does: **Retire…** on every reported Closed/Promoted item the loop did
not retire (source: the attempt's `summary.retire` plus the refused set, which
now remembers each item's kind, minus items with a retiring, retired or
pending line, and minus the running loop's unreached items); the unkept ask
(Show tab default, Cancel, no "Retire anyway") and the kept-draft ask (Retire,
Cancel default; document wording; "may have kept drafts" for an unknown
listing); mark at commit (not at press) with a `RetireStep::{Working, Asking}`
guard, RT-1 re-checked at the view and in `Scheduler::retire_chosen`, kept
drafts flushed first, a failed flush submits nothing; user lane, new request
ID, refused-set removal (A19). **Resume/Abandon** on a pending retirement:
Resume clears the declined keys, answers `original_request_required` by
rebuilding `ContextRetire{root, kind, id}` under the named request ID from the
entry (matched by operation, after a relaunch too) and dismisses the blocked
Resume row; Resume is offered when an entry names the row's operation or the
library's hand-off-only shape holds (`owes_only_hand_off`, a desktop copy of
`LocalRow::owes_only_its_hand_off`; the reviewer checked the sets are equal
today). **Unlock…** resends a pending retirement of one of the key's
repositories from the entry, else Check now where nothing is pending or held,
else it is disabled (`UnlockTarget::{Retirement, CheckNow}`).

Existing-test edits (none weakened): **E-27** `scheduler_extra_tests.rs::
own_polls_are_not_offered_resume` (`resume_offer_among` takes a third
argument); **E-28** `scheduler_loop_tests::a_pending_retirement_survives_a_
relaunch` (an Owed entry is kept on `Removed`/`NotSent` and released by
`Heard::PendingEnded`); **E-29** `scheduler_retire_tests::retire_inflight_entry`
(its last step keeps an Owed entry, then `Finished` ends it; a never-sent
request is still covered by the `Removed` step and the new inflight unit
test); **E-30..E-32** `a_pending_retirement_survives_a_relaunch`,
`retire_removed_resend_keeps_the_hold`, `unlock_picks_reachable` gain a canned
pending list naming the fabricated row (launch reconciliation correctly ends
the entry otherwise); **E-33** `a_removed_resend_keeps_a_pending_entry` (an
Owed entry without an operation ends on `Removed`/`NotSent`); **E-34**
`retire_loop_tests.rs` (`Facts.retired`, two table rows).

Rulings (the implementer's eleven were judged by the reviewer: sound, design-
silent or conservative; none contradicts a design row; costs):
1. (I-3) A `Removed`/`NotSent` answer to a resend keeps the entry when the
   library holds the retirement (it names the journal operation). Also closes
   K-41. Cost if wrong: an entry outliving a retirement the library does not
   hold leaves the item "Retiring…" until reconciliation, Abandon or Resume.
2. A new `Heard::PendingEnded` ends an entry on a successful Abandon or Resume
   of its operation. 3. A retirement's answer records the journal operation in
   its entry. 4. Retire… marks at commit, not at press (marking at press would
   make the tab read-only under an open "Show tab" ask). 5. "Unkept" for
   Retire… is `TabEditing.unkept` only; composer text and live-session drafts
   are flushed (a failed flush stops it). 6. An unknown draft count asks "may
   have kept drafts". 7. A retirement ends a CLEAN edit session of the tab and
   keeps a dirty one (`ItemPane::lock_changed` reacts to `retiring`; the
   reviewer walked every path: no text or draft is lost). 8. The refused set
   remembers each item's kind. 9. Resume rule as above (the drift risk is a
   library follow-up candidate, **not created; ask the owner at review-ready**,
   with K-33). 10. `authenticated` stays Retired/AlreadyAbsent/Preserved
   (conservative; departs from C3's literal "or a refusal that is not an
   unlock, key or host code"): an Unlock… resend answered, say,
   `refused external_change` leaves the key's attention until the next
   completed poll. **K-51.** 11. Offer words "{short} is closed. Its copy on
   this computer was not retired." (short code = the last 8 characters of the
   ULID, K-46); no title, body, comment or path.

Orchestrator ruling on review item I-C (no owner stop; an acknowledgement is
Document Amendment 12): entries are reconciled against a complete, freshly
landed pending-operations list at launch (lock held) and on window activation
(`services/scheduler/reconcile.rs`, a new defaulted `Backend::pending_operations`);
an entry whose operation is absent is ended only if the worker and the request
model hold nothing for it and no retirement entry changed since the read began
(`entry_epoch`); the `owed` field was removed and an entry is "held by the
library" only when its operation is known. Cost if wrong: an unneeded end of
an entry, so the item stops being "Retiring…" until the next attempt shows the
row again.

Review fixes (`9881c8a0`): **I-A** quit's `close_all_dialogs` left Retire…
ignored after a quit Cancel (the guard is now `Working`/`Asking`, a stale
`Asking` with no dialog is cleared); **I-B** the Resume request row is
dismissed when its resend is sent; **I-D** offers leave out a running loop's
unreached items and the loop skips an item already retired (`Skip::Retired`);
**I-E** a test that the unkept ask comes first; **I-F** `retire_resume_
resends_original` asserts the declined key directly; minors: a Resume that
finished the row says "Finished retiring {item}." (`RetireState::Finished`),
Abandon and a Noop say nothing; the second-press test folded in; a click hit
test of the pending row's Resume and Abandon; stale comments; the forgotten
repository's refusals cleared. Mutations: 32 in the first round (31 caught, one
survivor: the first unkept check in `Workspace::retire_chosen`, which the
reviewer showed was NOT equivalent: now pinned by
`retire_unkept_text_is_asked_before_drafts`) and 20 in the fix round (18 caught,
one botched and redone, one survivor: the worker-holds check in `reconciled` is
covered by the model's queued/running check in every test; kept as defence in
depth).

Process: the compile-red tests were red first; the model and service code was
written before its unit tests and every fix-round test was written after its
fix (green first; mutation evidence).

Known limits (recorded, not fixed):
- K-51: see ruling 10. K-52: a press on Retire… while a relist hangs waits for
  it (K-47); fixing it needs a timeout in the draft read.
- K-53: Resume of a row owing only its hand-off cannot be told apart from a
  full retirement by the entry (it keeps no step); the line says "Finished
  retiring {item}." for both, never "Retired the copy".
- K-54: a refused item stays offered as "closed" for the whole session even if
  it was reopened or retired elsewhere (bounded by the refused set).
- K-55: `retire_removed_resend_keeps_the_hold`'s `running.wait_reached()` can
  still wait the 180 s `World::settle` budget on failure.
- K-56: the Retire… press with `retire_chosen` returning `None` (RT-1 or
  quitting) gives no feedback.
- K-57: a composer with text and no draft yet is not asked about (it stays,
  locked, with its text; Fact 28's "unkept excludes composer", as for quit).
- Task 14 documentation: reword K-27 (E4 offers Resume for a hand-off-only
  row), and record Amendment 12.

Hand-offs to Task 12b: `Scheduler::hold_retirement` (test-only) puts a pending
retirement in the record and publishes it; `test_support::retire::{original_
request, pending_row}`; `Rig::over(fixture, roots, cx)`; `Operations::{
retirement_of, entered_operations}`; `Heard::{ResendRetirement, PendingEnded}`;
`DraftRegistry::{kept_of, flush_item}`; `Backend::{instead_of_job,
pending_operations}`. Real-repository tests were not run in 12a.

## Task 12b: end state, replay, quit during a pending retirement

| 12b | `902d51b0` | `4ade0fd6`, `ec6960e7`, `6a034e39` (final review fixes) | APPROVE (no Critical; the final Task 12 review also covered the 12a fix round and `reconcile.rs`); the review-fix commit was checked by the orchestrator (four new tests, gates, boundary), not re-reviewed | `desktop::view::operations::retire_real_tests::{retire_end_state_real, retire_replay_final, quit_during_retirement_pending}`, `desktop::services::scheduler::retire_tests::{retire_activation_reconciles_a_lock_already_held, retire_entry_the_worker_holds_is_kept_before_the_model_shows_it}`, `desktop::services::inflight::tests::an_entry_with_the_removed_owed_field_still_loads`, `desktop::services::scheduler::reconcile::tests::the_default_pending_read_is_complete_for_a_registered_root_only`; each singly with `--exact`, 1 test, pass (the orchestrator re-ran `quit_during_retirement_pending`, `relaunch_keeps_deadline`, `retire_end_state_real`, `retire_replay_final` and three of the fix tests); fmt, check --all-features, clippy -D warnings pass |

**Owner decision (AskUserQuestion, 2026-11-10): accept the gap.** The
implementer stopped on two of the four planned tests, under the brief's "a
library gap" rule. A standalone `ContextRetire` observes the remote over SSH
before it proves the closure from primary; the library's own tests get past
that with a `cfg(test)` thread-local (`run::synthetic_advertisement`) that the
desktop bin does not link, and the git transport cannot be initialised inside
the multi-threaded GPUI test process. Without SSH an open ticket answers
`recovery_required` (an observation failure), so the open-ticket refusal is
reachable only after a successful SSH observation. The owner chose the canned
route:
- `retire_open_ticket_refused_real` is **not written**; the open-ticket refusal
  stays covered by the canned E3 table (`retire_answer_table`, the
  `refused external_change` row).
- `quit_during_retirement_pending` is written with canned answers through the
  real quit and relaunch path (A22, K-18).
- **U-9's observer row and K-29 are UNVERIFIED in code.** The observer row
  (an active Manual observer row beside the retirement's `retire_context`
  row) exists only if a process dies mid-observation, and no in-process hook
  produces it (the library's cancel hooks are `cfg(test)`; a faked SQL state
  would fail the library's own `audit_connection`). A reading of
  `reentry.rs` (not a run) says the explicit re-entry fences the old observer
  row to `interrupted` and settles it, consistent with the ruling. **Put on the
  native checklist: "quit during a retirement, relaunch, Resume" on the real
  app.** A library follow-up candidate (a feature-gated synthetic advertisement
  or a way to leave an observer row) is noted for the owner alongside K-33; no
  ticket was created.

What it delivers (all under `src/desktop/`; library-boundary diff empty):
- `retire_end_state_real` (a real local repository, the real service): the
  open tab reads the item's own copy ("Own copy of the item"), stays so after
  the branch and worktree are removed with real git2 until the retirement
  answers, then reads primary's copy ("Closed on primary"); a kept item draft
  and a kept comment draft are listed on the Drafts page ("The ticket is
  closed"), offer export and not restore, and Export… writes each draft's text.
  **U-11 settled: Export works** for a closed ticket's item and comment drafts
  (no D2 defect). Deviation: the removal is done by git2 and the real
  `ContextRetire` answers `already_absent`, so the library's
  removal-then-answer path is not covered at the desktop layer (**K-58**).
- `retire_replay_final`: the same request ID sent twice through
  `resend_retirement` (a branch and worktree restored before the second send)
  leaves exactly one `Retired` line, no entry and no Activity row, and the
  context still present. It cannot see a "removed twice" by itself (a fresh run
  on a present context refuses without SSH and removes nothing either): it pins
  the same request ID reaching the library and the desktop adding no second
  entry, line or row.
- `quit_during_retirement_pending`: a held retirement (canned `pending`), a
  graceful quit that leaves the token alone (decision 28) and finishes the
  held job, a relaunch over the same app-data store with the first instance
  and its lock dropped, then (a) the pending row with Resume (from the entry)
  and Abandon, (b) the entry kept with its request and operation and the item
  "Retiring…", (c) Resume resends `ContextRetire{root, kind, id}` once under
  the named request ID on the user lane, a canned `retired` ends the entry and
  the item is no longer "Retiring…", (d) one `OperationResume` and one
  `ContextRetire` after the relaunch. The pending row and the reconcile list
  are canned; a negative control shows Resume is not offered without an entry
  for a row not owing only its hand-off. Test-harness seam
  `Shell::relaunch_over` (`view/harness.rs`), `relaunch` unchanged.

Final review (the whole of Task 12's last part): **no way found for
`reconcile.rs` to end a live entry**: epoch bumps cover every entry-changing
path, a resend's worker hold covers the gap before its `Queued` event, a
resend under the same request ID cannot create another operation
(`request_scope.rs:105`), the second instance (L-6) returns early, a partial or
failed list is `None` (the default `pending_operations` reads the real library
list inside one `read_session` and reports complete only on success). Fix
commit `6a034e39`: one reconcile read on activation (`acquire` returns whether
it took the lock); `retire_end_state_real` also asserts the tab still reads OWN
before the answer and that the line is Retired; an old record with
`"owed": true` loads; the worker-hold check in `reconciled` is pinned (its
mutation is now caught); the default `pending_operations` is unit-tested (a
registered root is a complete list, an unregistered one is `None`); the quit
test's doc comment says the row and list are canned and gained the negative
control. E-n edits: none in 12b.

Known limits (recorded, not fixed):
- K-58: tests 1 and 3 show `already_absent`, not `retired` with a removal
  outcome (see above).
- **K-59 (the final review's I-1):** an in-flight entry that never learned its
  operation (a hard crash between submit and the first progress event, or a
  request left Accepted with no data) has no exit: reconciliation skips it and
  Resume/Abandon match on the operation. The item stays "Retiring…" and
  read-only across relaunches. Rare. Ending such entries at launch would risk
  dropping the hold on a row the library created but we have not seen, so it is
  left alone; the pending list could be matched by `item_id` (`OperationDto`
  has it, Fact 33 says unused): a follow-up candidate.
- **K-60 (I-2):** no test shows the real library lists a live pending
  retirement under the entry's operation ID (every reconcile test uses a
  canned list). If the first progress event named a different row (the U-9
  observer row), every pending entry would lose its hold at the next
  activation. **Native checklist item**, beside U-9 and K-29.
- K-61: a discarded reconcile read is retried only at the next window
  activation; nothing reloads after a reconcile end (an open tab can show the
  old worktree copy until the next reload); a stale `Asking` blocks a new
  Retire… press while some other dialog is open in the same window.
- The 180 s `World::settle`/`title_becomes`/`held.wait_reached()` waits remain on
  failure in the new real tests (K-55 class).

## Part E checkpoint (Tasks 11a, 11b, 12), 2026-11-10

- Main: `origin/main` is `eb0798a2`, already merged at `6af92f91`;
  `git log HEAD..origin/main` is empty and
  `git merge-tree --write-tree HEAD origin/main` printed one line (a tree id):
  clean, nothing to merge (checked in full).
- Branch HEAD at the end of Task 12: `6a034e39` (code) plus the docs commits.
- Tests run, only by exact path, 1 test each, never a whole module, suite or
  SSH target: the tests named in the 11a, 11b, 12a and 12b rows, plus nine
  Activity-layout-sensitive existing tests (11b) and the existing
  `relaunch_keeps_deadline` after the harness seam (12b). Static gates (fmt,
  check --all-features, clippy -D warnings) pass at `6a034e39`. No whole suite
  yet: the fast tier and the `remote_polling` SSH tier run once at Task 14.
- Owner decisions this Part: Task 11b design checkpoint (four rulings,
  Document Amendment 11), the 12b gap (accept; U-9 observer row and K-29
  unverified, native checklist). Orchestrator rulings: I-3 entry kept on
  `Removed` (Amendment 12), the I-C reconciliation, Cancel/quit/reconfiguration
  loop rules.
- **Task 14 documentation to-do** (collected): reword K-27 (E4 offers Resume
  for a hand-off-only row); update K-16 and Amendment 8 for the Activity
  columns' `min_w_0`; reword "Cycle 01's proposed K-19" to "accepted K-19";
  record Amendment 12 (entry exits) and the plan's R-23 owner line ("Task 9
  (minimal), Task 11a (retirement rows)"); put on the native checklist: U-9
  observer row / K-29 / K-60 (quit during a retirement, relaunch, Resume),
  the retirement loop and Retire… on a real closed ticket, K-17 (a quit during
  a retirement waits one observation), and an unreadable draft file blocking
  auto-retirement.
- Known limits to report at review-ready: K-28..K-61 (K-33 library-shaped; no
  ticket created; ask the owner about follow-up tickets for K-33, the
  `owes_only_its_hand_off` flag, the retirement test hook and K-59).

## Task 13: privacy, consent and keyboard evidence

| 13 | `4c35b3cf` | `8c4fd1b4` (review fixes) | APPROVE WITH FIXES (no Critical; reviewer found no leak and no call the design does not allow); the fix commit was checked by the orchestrator (clean tree, boundary, `polling_consent` re-run), not re-reviewed | `desktop::view::polling_privacy_tests::polling_privacy` (A16), `desktop::services::scheduler::consent_tests::polling_consent` (A8), `desktop::view::keyboard_tests::shell::polling_journey::polling_keyboard` (A15); each singly with `--exact`, 1 test, pass (5.5 s, 26.3 s, 18.8 s); the orchestrator re-ran all three at `4c35b3cf` and `polling_consent` at `8c4fd1b4`; fmt, check --all-features, clippy -D warnings pass |

Tests only. All three were green on their first run, because Tasks 7, 8, 11
and 12 had already built the behaviour (no red-first run is possible for
evidence tests). Library-boundary diff empty. The only non-test-file change is
`Shell::set_activity_height` (test harness) going `pub(crate)`, plus three
`cfg(test)` mount lines. No existing test was edited (no E-35+ entries).

- `polling_consent` (A8): one recording `Backend` forwarding all 23 trait
  methods, a simulated day on an injected clock (launch, due, busy, preempt,
  attention, quit with a poll in flight). The allowed list is written from the
  design: B4's prefix per `Job(Poll)` window on the worker thread; jobs only
  `{Poll, PollOnce, ContextRetire}` with exactly one `PollOnce`; `cancel_poll`
  0 during launch/busy/attention and greater than 0 after preemption and quit;
  exactly two `ContextRetire` jobs (the free ticket and the promoted document,
  in report order, background lane; not the edited or kept-draft items);
  `ResumePolling` never; every intent rebuilt with `Intent::build` (none is
  `PollOnceRetiring`). Review fix I-1: calls off the worker thread are limited
  to the scheduler's read list, and `Service`/`KeyStore` off the worker only up
  to the count after `Services::start`; `PollAutomatic`, `PollEligibility` and
  `RemoteSnapshot` each equal the count of `Job(Poll)`.
- `polling_privacy` (A16): eight sentinels (title, body, comment, draft, typed
  edit, path, passphrase, key-file content), each shown to exist in its source;
  the drive covers every D6 surface (automatic poll with closed, edited, kept
  and promoted targets, attention, Retire… offers, both asks, retirement
  answers, Resume, Unlock… with the passphrase typed, Settings status, quit
  text); the sweep reads entries, answers, the `Debug` of the schedule and of
  the record's entries, the in-flight record's bytes and every drawn label and
  value outside `ALLOWED`, in 15 surface classes. Mutation proof: the passphrase
  input unmasked is caught.
- `polling_keyboard` (A15): `secondary-,`, the interval, the switch (Pause,
  Resume), Check now, then `secondary-4` to Unlock… and Retire… by Tab with
  focus on-screen and ringed at every step; Enter on Retire… sends one
  user-lane `ContextRetire`. The Activity panel is at the launched 150 px.
- Mutations all caught and reverted: quit's cancel removed; Retire… and switch
  and Check now handlers a no-op; Retire…'s scroll marker removed; RT-3 and RT-4
  off; `backend.service()` inside `poll::run` and in `Scheduler::activated`;
  attention repositories polled again; B6 gate off; `resume_polling` at
  activation; passphrase unmasked; sentinels in a canned message and in a
  directory name. The privacy mutations show the scan reads the surfaces; no
  production type holds item text, so no real production leak could be injected.

Known limits (recorded, not fixed):
- K-62: the recorder sees `Backend` calls only. A direct library call that
  does not go through `Backend` is invisible; a new `Backend` method with a
  default body is caught only because it reaches the library through
  `service()`, `key_store()` or `drafts()`. A direct `service()` call on the
  worker thread between jobs is not caught.
- K-63: W-4's "both layouts" is not exercised (one layout today); the keyboard
  journey uses the harness window resized to 1280x840 with Activity at 150 px,
  not `open_as_launched`.
- K-64: `polling_privacy` skips its unkept-ask and quit-edits steps when run as
  root (the drafts folder cannot be made read-only); the rest still runs. The
  root path was not exercised.
- K-65: text drawn under `item-banners` and `item-pill` is outside the privacy
  sweep (D6 draws nothing there).
- K-66: both polling tests use canned answers. The real library's answers are
  covered by the earlier tasks' real-library tests, not here.
- Overlap, accepted: `polling_keyboard` repeats the interval, switch and Check
  now steps of `checking_for_updates_by_keyboard` and Retire… by keyboard is
  also in `retire_tests`; the plan asks for the single walk. The two files are
  857 and 981 lines (neighbours: 1,153 and 1,367); not split.
- Time: `polling_consent` 26 s, `polling_privacy` 19 s, `polling_keyboard` 5 s.
  Failing waits are now bounded at 30 s (review fix I-2; the no-op Retire…
  mutation fails in 34 s, before 184 s).

## Task 14: final gates, checklist, documentation

Branch HEAD for the gates: `e5d00bb9` code (docs commits after it do not touch
code). `origin/main` is `eb0798a2`, already merged; `git log HEAD..origin/main`
is empty and `git merge-tree --write-tree HEAD origin/main` printed one line (a
tree id): clean, nothing to merge (read in full, 2026-11-10).

**First whole-suite run (at `b0da75fe`).** fmt, `cargo check --all-features`
and clippy `--all-targets --all-features -D warnings` pass. `--lib` 1564
passed. The desktop `--bins` run failed 3 of 1818 and cargo stopped before the
integration targets (no `--no-fail-fast`): the first whole-suite run since
Tasks 11a to 13 (K-67). Fix commit `e5d00bb9` (implementer, not re-reviewed;
checked by the orchestrator: the diff read, tree clean, boundary empty, and the
full rerun below):
- `scheduler::retire_tests::unlock_picks_reachable` (a race, 55 of 480 failures
  under heavy load): after Unlock… both repositories poll again and
  `PollSchedule::raise` skips a repository whose poll is queued or running
  (returns false, ignored by the test). **E-35:** wait until neither root is
  `Queued` or `Running` before the raise loop; no assertion weakened. After:
  0 of 480 under the same load.
- `view::dock::restore_tests::a_file_that_cannot_be_read_restores_the_default_and_says_so`
  (deterministic): `Scheduler::new` opens `<data>/inflight` for the instance
  lock at launch (B3, R-31), so the store-directory snapshot gains `inflight`.
  The test's expectation was stale. **E-36:** the test's `before` state opens
  the in-flight store too; the entry-names snapshot is kept, so a stray new
  store directory still fails (mutation caught).
- `view::operations::retiring_tests::retire_rows_hit`: **a real production
  defect and a test race.** The request row's Dismiss had no scroll-into-view
  marker, so Tab could reach an off-screen Dismiss (the attempt entries'
  actions have it, E-2). `Workspace::dismiss` in `view/operations.rs` now
  wraps the button in `request-N-dismiss-slot` with the marker (the inner id is
  unchanged). The test also turned polling on, so a real poll answer wiped the
  injected rows (about 2 of 144 under load). **E-37:** the incidental
  `with_polling()` is removed (polling must stay off there). Mutation (marker
  removed) fails deterministically; 320 of 320 pass under load.

**Final gates after the fix (the tree at `e5d00bb9`, docs commits since):**
- Static gates, whole tree, all exit 0: `cargo check --all-features --locked`,
  `cargo fmt --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`.
- Fast tier (AGENTS.md list; `--lib` from the first run, unchanged since):
  `--lib` **1564 passed**, 0 failed (1009 s, `repository::local_sync::` skipped);
  desktop `--bins` **1818 passed**, 0 failed (1208 s; `--no-fail-fast`);
  the 27 listed integration targets **2522 passed** in total, 0 failed
  (summed from the per-target result lines in the run's log).
- SSH tier: `remote_polling` **86 cases passed**, exit 0, about 5 min (it
  includes Task 1's `poll_cancel_during_handshake_stall` and
  `poll_cancel_during_connect_stall`). The change touches no
  `src/repository/` path; the tier ran because the Cycle's brief requires it.
- Library boundary (`src/repository*`, `src/results.rs`, `src/canonical.rs`,
  `src/drafts/`, `src/runtime.rs`, `src/lib.rs`, `Cargo.toml`, `Cargo.lock`,
  `tests/`): empty against `6af92f91`, the Cycle 01 merge (checked after the
  last code commit).
- Native CI not dispatched (owner-gated); the full `cargo test` suite and
  native CI run after merge (AGENTS.md).

**The flaky test.** `attention_answer_tests::check_now_failure_becomes_attention`
failed once earlier under load. It passed in both whole-suite runs and three
solo runs; no cause is proven. It uses the same bounded `World::quiet` wait
before a `raise` that returns false for a queued or running poll as
`unlock_picks_reachable` did (E-35): a plausible, unproven race (K-68).

**Existing-test edits this Task:** E-35, E-36, E-37 (above). No other.

**Checklist** `docs/plans/2026-11-07-wave-03-desktop-06-on-screen-checklist.md`
(`feb839cd`, 70 rows, about 10 rare), drafted from the app's wording by an
agent and not run. Findings it reported (source versus design or ledger,
nothing fixed):
- The ledger's "Own copy of the item" and "Closed on primary" are test
  identifiers, not UI text; the checklist uses the "Closed" pill and the
  "Closed at" and "Closed by" fields.
- A Check now's "Unlock key" dialog has no "Sharing {repository} is waiting
  for your passphrase." line (`waiting_share` accepts only Share and Post).
- Retirement rows name an item by the last 8 characters of its ID (K-46), not
  by the ticket's short code that E3 names.
- A running retirement's quit text is "Finishing Retire local copy".
- The unkept Retire… ask is reachable by hand only with a read-only drafts
  folder (drafts flush within 5 s).
- Local-only saved work blocks retirement (the proof compares against the
  closed main), so the checklist says to Share first.
- Wording not found (marked "(wording not checked)"): the pending row's action
  text, the Abandon confirmation's retirement details, the Hosts page controls,
  the dirty-context guidance line, the "draft could not be saved" entry, the
  tab's discard-edit control.
- Not verified: that its `close_ticket` helper makes a poll classify a ticket
  as closed on a real remote.

**Cycle Acceptance table** (`54429ace`): every row has an evidence-based status.
Verified (tests) or verified with native evidence pending, except A4, A5, A19,
A20 and A22 (partly verified: the 12b gap, K-58, K-29, K-60, the unmeasured
stalls). Library SSH cases other than `remote_polling` (`poll_materialize_*`
ran there; `retirement_cli_authenticated_live_pass` and the yield tests did not)
are not in this Cycle's runs.

**Documentation done:** K-16 and Amendment 8 reworded for the Activity columns'
`min_w_0`; "proposed K-19" is now "accepted K-19" in the design and the Cycle;
K-27 reworded for E4 (Resume stays for a hand-off-only row); the plan's R-23
owner line; Amendment 12 recorded in the design (awaits acknowledgement).

Known limits added (recorded, not fixed):
- K-67: the first whole-suite run came only at the end, so three failures
  (two stale or racy tests, one real Dismiss scroll defect) survived to Task 14.
  A lesson, not a code limit.
- K-68: `check_now_failure_becomes_attention` may share E-35's race (above);
  other tests that inject attempt state and also call `with_polling()` may carry
  E-37's race (`operations/retire_tests.rs`, `operations/record_tests.rs`,
  `lifecycle/retiring_tests.rs`); none failed in the two whole-suite runs.
- K-69: the request row's other controls (Stop, Cancel, Open, recovery actions,
  Review, Retry in `view/operations.rs` and `operations/review.rs`) have no
  scroll-into-view marker either, so a keyboard user can Tab to an off-screen
  control. Only Dismiss was fixed (a D6 row). The rest are the "existing
  unwrapped rows" of K-16, left to Wave 04 Cycle 03 (a marker in the shared
  `button()` would cover all).
- Time: the desktop suite takes about 20 minutes; the longest tests are the
  real-library world tests (K-43).

## Code review (owner-run, 2026-11-10) and the fix round

The owner's code review of the branch at `57414f32` (182 files, about 38.8k
lines; the reviewer read the scheduler, polling, in-flight record, worker,
quit, preempt and retirement code in full and skimmed most views and tests; its
findings were not checked a second time) reported 10 issues. Findings 1 to 6
were to be fixed before approval. The owner asked for no ruling beyond that;
the orchestrator also fixed 7 (a bug of the same kind). Each of 1 to 7 was
reproduced with a failing test first, fixed, made green and mutation-checked
by one implementer; none was a false alarm.

| Finding | Reproduced | Fix | Test (`desktop::services::…`) |
| --- | --- | --- | --- |
| 1. One repository's stuck retirement cleared per activation (`reconcile.rs`, shared `entry_epoch`) | yes | reconcile no longer bumps the epoch; the three entry-changing paths still do | `scheduler::retire_tests::retire_one_activation_ends_the_stuck_entries_of_every_repository` |
| 2. Attention raised while a loop that began during an off-thread key read keeps running (`retire.rs` `raise`) | yes | `end_loop(root)` before `schedule.raise` (R-32, RT-8) | `scheduler::loop_tests::attention_raised_after_a_key_read_ends_a_loop_begun_meanwhile` |
| 3. Stop on the person's own retirement ended the whole loop (`retire_loop.rs` `loop_answered`) | yes | Stop ends the loop only when the stopped request is the loop's own; walls still stop it (RT-8, RT-9); a user Cancel of a running loop item still stops the loop | `scheduler::loop_tests::stopping_the_persons_own_retirement_leaves_the_loop_alone` |
| 4. Two polls at once after a schedule is forgotten mid-poll (`entity.rs`, `PollSchedule::remove`) | yes | an `orphans` set holds the single-poll slot until that poll's own answer (`PollSchedule::poll_ended`, called from `scheduler::apply`) | `scheduler::entity_tests::a_repository_forgotten_mid_poll_still_holds_the_single_poll`; `model::polling::tests::schedule::a_removed_repositorys_poll_holds_the_slot_until_it_ends` |
| 5. A late cancel hit a restarted run of the same operation (`worker.rs` `PollWatch::running`) | yes | `PreemptPoll` carries the job id; `PollWatch::for_run(id)`; `Canceller::watch_for(id)` | `preempt::tests::a_cancel_task_ignores_a_restart_of_its_operation`; `preempt::tests::the_drains_canceller_watches_the_named_run` |
| 6. A preempted poll stayed stuck while any background job existed (`entity.rs`, R-34) | yes | `WorkerHandle::user_lane_busy()`: only user-lane work holds the wake | `scheduler::entity_tests::a_preempted_poll_returns_while_a_background_job_runs` |
| 7. The third copy of "is this retirement queued or running" ignored the kind (`attention.rs`) | yes | one helper `WorkerHandle::retirement_in_flight(root, kind, id)` used by every copy | `scheduler::retire_tests::retirement_in_flight_is_keyed_by_kind` |

Commits: `0d4129b7` (1 to 3), `0aa17e73` (4 to 7), `7d9a2a94` (the second
review's fixes). A fresh reviewer read all of it: APPROVE WITH FIXES, no
Critical. Its two Important items were test gaps, both closed in `7d9a2a94`:
the three `entry_epoch` bump sites were untested although finding 1 made them
the only guard against a stale read ending a live entry
(`scheduler::retire_tests::retire_entry_events_move_the_epoch`, each bump
mutation-checked, and a stale read after `Retiring::Operation` keeps the
entry), and finding 5's production wiring in `Canceller::start` was unpinned
(now `watch_for`). Its Minor items: an orphan could be cleared by a failed
Check now's `after` (fixed: only `poll_ended` clears it) and a comment at
`raise`. Existing-test edit **E-38**: `preempt_tests.rs` `Harness::pump`
destructures `PreemptPoll { root, op, .. }` because the event gained a field;
no assertion changed. The reviewer ran every touched test singly, checked the
library boundary empty, and found no interleaving in which reconcile ends a live
entry (by reading and mutation; not a model checker).

**Gates after the fix round** (the tree at `7d9a2a94`): `cargo check
--all-features --locked`, `cargo fmt --check` and `cargo clippy --all-targets
--all-features --locked -- -D warnings` exit 0. The desktop `--bins` run:
**1828 passed**, 0 failed (1192 s; was 1818, plus the ten new tests). The
`--lib`, the 27 integration targets and `remote_polling` were not rerun: no
library, `tests/` or Cargo file changed since their green run (the boundary
diff is empty). Library-boundary diff against `6af92f91`: empty.

**Findings not fixed, with the reason:**
- **8** (the conversions between `TargetKind`, `ItemDtoKind` and
  `AuthoringKind` are written out in about eight places; `From` impls would
  replace them): a refactor with no behaviour change; recorded as **K-70**.
- **9** (`tests/remote_polling.rs`: two SSH cases against AGENTS.md's "at most
  one per feature"): the owner approved both in the design (decision 22,
  Task 1: the handshake stall and the connect stall are different transport
  stages, and cancel behaviour under a stall exists only over a real
  socket). Not changed. Each takes about 10 to 30 s (`poll_cancel_during_
  handshake_stall` about 30 s and `poll_cancel_during_connect_stall` about
  10 s in Task 1's runs).
- **10** (every retirement progress event rebuilds the poll sets and the
  entered list and republishes to Operations; `tick` calls `cx.notify()` on
  every run, so every window's Workspace and Settings re-render on routine
  worker traffic): not measured or changed in this round; recorded as **K-71**
  (act only when something changed).

**Known limits changed:**
- **K-44 is removed** (finding 6): a preempted poll no longer waits for a
  running loop or any background job; only user-lane work holds it
  (`user_lane_busy`, R-34).
- **K-72:** a removed repository's poll that never answers would hold the
  single-poll slot for the session (the worker always answers a poll and quit
  stops polling, so it was accepted); a repository removed and re-added while
  its old poll runs has that old answer applied to the new schedule, as before.
- **K-70, K-71:** above.
