---
manyhands_managed: true
manyhands_kind: comment
id: "01M622X59YKBQV7HGK4H0G9NZA"
item_id: "01M4CC0VMWEQQ9G4W0DAG7ADPY"
parent_id: "01M621C24Z76N2M10M12F762CT"
created_at: "2026-10-28T20:31:39Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 0 complete; the task review is pending

- **Base.** `origin/main` is still `a372edb`, the commit the design was
  re-grounded on. The merge was a no-op, `git merge-tree` is clean, and
  `src/` and `tests/` are identical to main's. The gate ticket is closed on
  main (`closed_at: 2026-10-27T12:51:51Z`). The six known-limit tickets and D3
  are not on main, so they are open, and none has a `closed_at` on its branch.
- **Facts.** Facts 1–86 stand. Re-checked: fact 28 (`set_local_identity` is
  not refused by a pending operation), fact 10 (see below) and fact 81. Fact 81's
  test fails as expected, for exactly the gate's eight `local_context_sync_*`
  tables and no other; E-12 fixes it in Task 4a.
- **Named edits.** The three greps found 56 hits, 4 files and 311 hits. All are
  production code, an E-1 to E-12 site, a loop over an inventory, or a
  single-variant match with a fallback. E-3 is classified: five binding-level
  tests (edited) and five domain-level ones (unchanged).
- **Baseline checks.** `check`, `fmt` and `clippy` with all features pass,
  and so does the desktop `check`. **The desktop-alone `clippy --all-targets`
  fails on main**, with 58 dead-code errors in the desktop's test target, so the
  plan's gate command cannot pass whatever C4 does. The `--bin manyhands` form
  passes. The full suite was deferred to Task 15 (ruling P1).
- **Audit and ledger.** The API audit has a C4 refresh with the union of 48
  codes, the seven C4 rows updated in place and `item republish` added. The
  execution ledger is created.
- **For the controller before Task 1.** The desktop clippy command, and what
  `operation abandon` does to a comment whose local row is still pending (its
  binding is spent, and its commit stays unpublished until `item sync`). Both
  are in the ledger under "Findings awaiting a ruling".
