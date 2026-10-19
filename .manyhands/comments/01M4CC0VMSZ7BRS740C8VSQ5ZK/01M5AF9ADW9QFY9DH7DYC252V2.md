---
manyhands_managed: true
manyhands_kind: comment
id: "01M5AF9ADW9QFY9DH7DYC252V2"
item_id: "01M4CC0VMSZ7BRS740C8VSQ5ZK"
created_at: "2026-10-19T16:18:02Z"
---

C1 Task 8: **implemented, review-ready**, with independent whole-branch quality
review pending with the controller. The ticket remains **open**; this comment
does not claim final approval.

Final checked Rust HEAD: `309c9e071994f55eb0b1fa9f44546cc2722d0398`, on the ticket
branch/worktree. Fetch/merge of main before and after the final gate returned
Already up to date at `f6a4c06300f4be80eb18f843a9e4fcbde847e47e`. Merge-tree
exited 0 with tree `be75bbca9cdcfc756599686161828c935a136d07`.

All four final commands ran through Devenv and exited **0**, with a two-hour
outer budget and complete logs preserved under `/tmp/opencode/`:

- `cargo check --all-features --locked` — PASS; `manyhands-c1-task8-final-check.log`.
- `cargo fmt --check` — PASS; `manyhands-c1-task8-final-fmt.log`.
- `cargo clippy --all-targets --all-features --locked -- -D warnings` — PASS;
  `manyhands-c1-task8-final-clippy.log`.
- `cargo test --all-features --locked --no-fail-fast` — PASS;
  `manyhands-c1-task8-final-test.log`: **3,468 passed, 0 failed, 1 intentionally
  ignored**, including **11 doc tests**. This includes **1,196 library**, **79
  CLI bin**, **64 CLI integration** tests and **621 custom SSH cases**. Every
  target's exact count is in the ledger. The ignored case is the existing
  opt-in thousand-item read characterization. No successful broad gate was rerun.

Bare CLI smoke exited 0 and printed help (`manyhands-c1-task8-final-cli-smoke.log`).
The exact no-default-features normal-dependency graph check printed **0** GPUI
matches, with Cargo exit 0 and grep's expected no-match exit 1
(`manyhands-c1-task8-final-dependency-tree.log`). The active Wayland display was
exercised: desktop build passed, then the bounded launch created/configured a
toplevel and attached/committed rendered buffers, without panic/startup error.
It was stopped by the deliberate 15-second timeout (124); no matching launch
process remained. This is launch evidence, not manual visual/interaction approval.
The first 60-second attempt only compiled; its log is preserved alongside the
successful build and launch logs (`manyhands-c1-task8-final-desktop-*.log`).

Controller handoffs: Task 7 spec/quality **PASS**, no findings, with fresh 62 CLI
tests and clippy/fmt; the test-only array-chunks lint fix received independent
spec/quality PASS. Whole-branch spec re-review is **PASS** after `2b84360` fixed
P2 recovery-field escaping and P3 help-width comparison. Decision 14 explicitly
authorized `309c9e0`'s persisted-baseline reload in the one pre-existing closure
test, preserving all assertions. Earlier lint failure, incomplete full-test
attempt, diagnosis and resolutions remain recorded; final success does not
erase those logs. Whole-branch quality review's outcome has not been supplied.

The ledger maps all 11 Cycle acceptance bullets to tests/evidence. Actual CLI
fixtures are **51 JSON + 4 text = 55**, with all 28 available verbs exercised in
both modes, schemas/goldens checked, all-verb secret scans and per-call snapshots.
Limits remain explicit: R-3 Windows default location is inspection-only and
macOS default redirection is not run here; R-4 transport-start failure uses a
fake; R-5 cannot isolate the child's libgit2 system configuration. Native CI
is post-merge on main and was not dispatched.

[Final evidence, per-target counts and acceptance mapping](../../../docs/plans/2026-10-18-wave-03-cli-01-shell-and-inspection-execution.md#task-8-final-verification-and-review-ready-checkpoint--2026-10-19).
This documentation-only checkpoint is `docs: mark C1 review-ready`, with the
required co-author trailer. No push, PR, main merge, rebase, closure or cleanup.
