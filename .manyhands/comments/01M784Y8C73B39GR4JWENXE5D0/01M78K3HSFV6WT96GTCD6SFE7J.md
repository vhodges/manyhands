---
id: 01M78K3HSFV6WT96GTCD6SFE7J
item_id: 01M784Y8C73B39GR4JWENXE5D0
created_at: 2026-11-12T19:17:44.174772793Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Task 0 checkpoint (baseline, references, ledger), 2026-11-12.

**Base.** origin/main `37be101d`, already held by the branch; `git merge origin/main` was a no-op. Branch HEAD before the task commit `1cfa97fc` (docs-only: two stale D6 lines in the Wave, brief's HEAD wording). Task commit `09437383`, pushed; HEAD equals origin/manyhands/ticket/01M784Y8C73B39GR4JWENXE5D0. Main checkout clean at `37be101d`.

**Gates** (green, no warnings): `cargo check --all-features --locked` (55 s), `cargo fmt --check` (13 s), `cargo clippy --all-targets --all-features --locked -- -D warnings` (67 s). No test ran; the fast tier runs once at Task 10.

**Facts.** `src/` and `Cargo.lock` are unchanged since `7034522f`, so gpui-kit, gpui-base and gpui-component did not move (0.6.6) and Baseline Facts 1-70 hold. Only docs changed (approval commit, `1cfa97fc`).

**E-sweep.** Every site maps to E-1..E-16 or to a task; no unnamed site. Four watch items for later tasks (none forced to break): the `ToolbarButton::ALL` loop in `dock_tests/toolbars.rs:270` (Task 6, E-17 if it breaks), `Control::ALL` in `footer_tests/mod.rs:405` (Task 8), the five `take_vanished` tests in `navigation_tests.rs` (Task 3 runs them), and the `ALLOWED` lists in `editing_privacy_tests.rs` and `lifecycle/privacy_tests.rs` (Tasks 4 and 9).

**Rulings.** None. **Test edits.** None yet. **U-checks** U-1..U-7 open, each in its task. Ledger: `docs/plans/2026-11-12-wave-04-cycle-03a-repositories-execution.md`.

Next: Task 1 (Part A, sharing facts the scheduler keeps), with its design checkpoint on the unshared-list table U1-U15.