---
id: 01M70HMM4YJ7SRCQHN6PZHSVXZ
item_id: 01M4CC0VN3VPVVQ3RNWJKXWEVX
created_at: 2026-11-09T16:18:10.833419261Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
## Part A checkpoint: Tasks 1 and 2 done (clock seam and schedule model)

- **Task 1 (runtime readiness).** Two SSH cases in `tests/remote_polling.rs`, each run singly by exact name (1 case, pass): `poll_cancel_during_handshake_stall` returns `Cancelled` about **30.0-30.1 s** after the cancel (the cancel does not interrupt the blocked call; libgit2's 30 s per-call timeout then the next safe point), and `poll_cancel_during_connect_stall` about **10.05 s** (six runs, no root, Linux only; no existing fixture could stall a connect, so the case uses a never-accepted `listen(fd, 0)` socket and waits for `SYN_SENT` before cancelling). Both are inside the documented bound, so U-14 found a reliable stall and no stop was needed. Research note: `docs/research/wave-03-d6-transport-stall-cancellation.md`. Commits `4222708c`, `79c9a981`.
- **Task 2 (Part A).** The clock seam and the pure schedule model in `src/desktop/model/polling*`: A4's table row by row, due order and launch deadline, next wake, clock events, restart schedule 5/15/60 s, reconfiguration, pause versus attention, attention keys. Ten tests, each run singly by exact path (1 test, pass): `desktop::model::polling::tests::{after_attempt_table, schedule::{attempt_summary_counts,restart_schedule,preempted_three_times}, decisions::{due_order_and_launch,next_wake_table,clock_event_once}, attention::{reconfiguration_table,pause_versus_attention,attention_keys}}`. Commits `fd284033`, `d80b481d`. Independent review: approve with minor; the three Important findings were fixed (a missing key is now one attention entry per key, as decision 11 says; `abandon` for a dropped poll; every A4 row now asserts the recorded attempt).
- **Rulings** (all in the ledger): A4 maps key-source/selection/endpoint/host-trust changes to a re-read, not the library's categories, so Task 3 uses an exhaustive match over the 24 transport kinds; a `stale` flag makes Task 5 re-read policy right after an attempt, a clock jump or a reconfiguration; `hold_until` keeps "Idle at the interval" from looping on a stale deadline (R-35).
- **Gates.** fmt, `cargo check --features desktop`, both clippy runs pass. No whole suite and no other SSH target was run. No existing test edited (no E-n yet).
- **Main.** `origin/main` still `f4442618`, an ancestor of HEAD; Wave 04 Cycle 01 is not on main.
- **Ledger:** `docs/plans/2026-11-07-wave-03-desktop-06-polling-and-shutdown-execution.md` at `7f9a9fb4`.
- **Next:** Task 3 (automatic poll on the background lane: `PollAttempt` conversion, five `Backend` methods, `Work::Poll`, background provider rule).
