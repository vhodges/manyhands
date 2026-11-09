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

## Existing-test edits

E-1 to E-6 are named in the design's Testing section. Any further edit is
pre-approved (owner, 2026-11-09) and is recorded here as E-7 onward, with its
reason.

(none yet)

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

