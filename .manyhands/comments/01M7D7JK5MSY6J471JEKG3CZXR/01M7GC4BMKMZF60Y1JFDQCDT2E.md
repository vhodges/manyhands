---
id: 01M7GC4BMKMZF60Y1JFDQCDT2E
item_id: 01M7D7JK5MSY6J471JEKG3CZXR
created_at: 2026-11-15T19:49:46.159898583Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Task 0 complete (baseline, references, ledger), 2026-11-15. Subagent-driven, serial; an Opus implementer, then a fresh Sonnet reviewer: spec compliant, no Critical or Important findings.

**Base.** Cycle 05a is closed and merged (`origin/main` 1f30d58c, "Integrate closed ticket 01M7D5Z7FZTGTC1HE25KQ1MYQY"). Main merged into this branch at `1fbb19b3` with no conflicts. `Cargo.lock` has not moved since `ac50ba1a`; gpui-kit, gpui-base and gpui-component are all 0.6.6.

**Commits** (`1fbb19b3..81314d73`):
- `6873f2c6` re-maps the design to 05a's merged code (U-1);
- `5be247cf` applies Document Amendments 1 (the Wave) and 6 (05a's design B2 and B3);
- `81314d73` creates the execution ledger.

**Gates, static only** (`devenv shell -- cargo ... --locked`): check --all-features, fmt --check and clippy --all-targets --all-features -D warnings all pass. No tests were run, as the plan says. Boundary check (`src` outside `src/desktop`, `tests`, `Cargo.toml`, `Cargo.lock`, `packaging`) prints nothing.

**U-1.** 05a's code can carry B1 to B3 on each of the four points: `sync_sheet` takes a `Resolve` case; `settings_key`, `show_settings` and `show_add_repository` each take a one-term refusal; the keep-focus gate takes a second `if`; the waiting close (`add.waiting`, `run_waiting_add_close`) can be copied. No stop condition was met. Corrections to the design are annotations marked "at the merge": `settings_open` was not widened (05a added `sheet_held` beside it); the keep-focus functions are now `keep_focus_in_sheet` and `sheet_keeps_lost_focus`; `apply_review`'s refocus is an owner match, not `sheet_held`; `Tone::Accent` and `sheet_footer` are already in use; `ICONS` has 57 names, so E-13 is 57 to 59. No contract row, ruling or decision changed.

**U-6.** The sweep (729 hit lines) finds the named edits E-1 to E-15 where the design says; no E-16 onward yet.

**U-7.** 55 production sites check only for a dialog. Those that open by themselves are `apply_review`, `republish_once_stopped` and `open_deferred_republish` (Task 6, R-18) and `sync_sheet`'s details request (Task 5). `open_created` and `close_with_draft` are a person's action and need nothing beyond `focus_content`'s gate.

**U-9.** 05a's openers that wait for no dialog cannot meet an open Resolve sheet (one sheet slot, the same-cycle deferral, and the `modal_open` drop), so Task 6 adds no code to them. Amendment 6's "the return to Settings also waits" is therefore vacuously true; it keeps the design's wording and is flagged for the review-ready summary.

**Rulings** (in the ledger): the commit trailer is the session's own, Sonnet 5.5 (the two unpushed Task 0 commits were reworded; T-1); `apply_review`'s refocus follows the H rows, not a fixed predicate (O-1); four lifecycle files missing from Task 4's File Map are in scope, and Task 2's P1 table gets a busy-resume row (O-2); Task 8 widens two private test helpers as E-16 and E-17, visibility only (O-3); module runs are one `cargo test` per module path, plus the extra modules the sweep found (O-4).

**Pushed.** This ticket's branch, by `item sync` from the main checkout; HEAD equals `origin/manyhands/ticket/01M7D7JK5MSY6J471JEKG3CZXR`. No other branch was published.

**Next.** Task 1 (copy and constants). Nothing is waiting on the owner.