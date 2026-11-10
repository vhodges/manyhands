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
| 10 | (ledger commit) | `d8e13309` | APPROVE WITH MINOR; no code fix needed (every finding is a ledger record or a hand-off); not re-reviewed (no fix commit) | `desktop::view::quit::tests::{quit_freezes_scheduler, quit_lets_retirement_finish, quit_sees_poll_end, quit_polling_still_stopping, quit_joins_worker, quit_joins_worker_when_a_job_ends, quit_during_restart_wait}`, `desktop::view::item::edit_tests::quit_freeze::quit_freezes_scheduler_for_the_unkept_ask`, `desktop::services::worker::quit::tests::quit_cancels_poll`, `desktop::services::scheduler::quit_tests::{relaunch_keeps_deadline, quit_drops_an_armed_check_now, quit_stops_unlock, quit_stops_resume_checking, quit_withdraws_queued_poll}`; the implementer ran the 14 as one command, the reviewer re-ran 15 singly with `--exact`, 1 test each, pass; fmt, check, both clippy pass |

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
