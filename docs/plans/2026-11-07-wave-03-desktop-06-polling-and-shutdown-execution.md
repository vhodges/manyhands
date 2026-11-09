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
| 0 | `83fe181a` | (this commit) | — | static gates only (check, fmt, clippy: pass) |
