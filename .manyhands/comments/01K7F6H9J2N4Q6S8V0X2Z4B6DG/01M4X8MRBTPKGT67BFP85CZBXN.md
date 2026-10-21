---
manyhands_managed: true
manyhands_kind: comment
id: "01M4X8MRBTPKGT67BFP85CZBXN"
item_id: "01K7F6H9J2N4Q6S8V0X2Z4B6DG"
created_at: "2026-10-14T13:11:49Z"
---

Task 8 permission-denied follow-up, **IN PROGRESS / controller review pending**.
Owner required row 6's literal gap closed before final reviews. Increment base
`1bbc9e250a4095e4b5a43721126297e27cf2b4ee`; complete Task 8 base remains
`b0f005531879c5ee2806ed13052bdd157df627d7`. Same worktree/branch, inline;
prior approvals/minor resolutions and current-base preflight trusted.

New `poll_unix_permission_denied_worktree_preserves_local_state` uses the public
poll API over real SSH. Checked-out context strictly behind remote, directory
chmod 000 under restoration guard, actual directory enumeration and .git read
both return PermissionDenied. No root/ineffective-mode skip: denial assertions
must hold. Poll reports Inaccessible with fixed guidance; denied .git access and
mode 000 survive. Explicit permission restoration precedes result/preservation
assertions; full permission-aware local snapshot and complete direct/symbolic
refs prove no local mutation, with only legitimate fetched tracking advancement.
Local branch stays old and incoming file is absent. No receive-pack or privacy
regression. Local UID 1000, **row 6 fully LOCAL-proved**. cfg(unix) execution on
Linux does not prove Windows ACL behavior; native permission coverage pending.

Focused new case passes after correcting fixture-only use of clean_checkout,
which incorrectly required a context tracking ref before the first fetch.
Existing production already behaves correctly; **no behavioral RED claimed**.
First full-gate attempt check/fmt passed, Clippy required octal permission literal;
changed 0 to 0o000, then ran the complete final-tree gate again. Both trials are
recorded honestly in the dated ledger; no production change.

Final sequential Devenv check/fmt/strict Clippy/full test/CLI all PASS:
**1,766 passed, 0 failed, 1 existing ignored** = 615 library + 779 ordinary
integration + 363 SSH + 9 doctests; **74 polling cases**. No Rust edit after gate.
CLI exit 0/no window, git diff checks pass, lockfile hashes unchanged. All 12
acceptance rows now reference **122 distinct tests matched to actual PASS output**.
Exact log, names, hashes and ranges in the
[execution ledger](../../../docs/plans/2026-10-11-wave-02-cycle-08-one-shot-polling-and-materialization-execution.md#task-8-permission-denied-evidence-follow-up--2026-10-14t131149z).

Controller SPEC, whole-branch quality and independent whole-branch reviews still
pending/unrun here; Task 8 unchecked and ticket open. Five native targets and
Windows ACL coverage pending authorization/native proof, manual workflow
undispatched. New local commit only; no amend, subagents, push, PR, merge,
closure or worktree cleanup. Approved contract/amendments/sole lease exception
unchanged; prior bounded GUI process-only evidence retained, not redundantly rerun.
