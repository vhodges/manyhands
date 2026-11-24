---
id: 01M7H5GDRKKJ4HV2VH7C0QSVJ9
item_id: 01M7D7JK5MSY6J471JEKG3CZXR
created_at: 2026-11-16T03:13:16.047491043Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Review-ready, 2026-11-15. Cycle 05b (Close Ticket And Resolve Conflict) is implemented, reviewed and gated on its local Devenv gates. `origin/main` is unmoved at `1f30d58c`; `git merge-tree --write-tree HEAD origin/main` is clean, so no merge was needed. Branch head after this comment: see the branch. Detail: the execution ledger `docs/plans/2026-11-15-wave-04-cycle-05b-close-and-resolve-execution.md`.

**Status.** Tasks 0 to 8 each had a fresh implementer and a fresh reviewer (no Critical or Important left open), a final whole-branch Opus review (Ready to merge) and one fix wave (`7a93db2a`) with a scoped re-review (addressed). Boundary: `git diff --stat origin/main...HEAD -- src ':!src/desktop' tests Cargo.toml Cargo.lock packaging` prints nothing; the change is `src/desktop/` and its tests plus docs.

**Gates and tests.** At `7a93db2a`: `cargo check --all-features --locked`, `cargo fmt --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings` clean; the AGENTS.md fast tier once: 30 targets, **6063 passed, 0 failed** (lib 1564, bin 1977). No SSH target (desktop-only). Orchestrator module runs per task passed (Task 6: 500; Task 7: 77; Task 8: 92). One failure was found along the way: Task 4's `check_now_tests::way_out_check_now` (Ruling O-7, fixed in `5f729d5d`). Acceptance Status A1 to A9 is filled in the Cycle document; **A10 is on screen and is yours** (`docs/plans/2026-11-15-wave-04-cycle-05b-on-screen-checklist.md`: frames 2c, 2d, 2e and 2g in the dark theme, and each Known Limit). Native CI is not run.

**Test edits (E-n).** E-1, E-2 (Task 5: `open_sheet_in` sites); E-3 (Task 2: **assertion removal**, the two `lifecycle-kind` asserts, as the design says); E-4, E-5 (Task 2); E-6 (Task 3: two confirm tests); E-7, E-8, E-9, E-13 (Task 1); E-10, E-12 (Task 7: E-12 adds a Tab and `in_sheet("conflict-cancel")`, since Tab from the Result stop reaches Cancel then Save); E-11 (Tasks 1 and 5: `in_dialog` sites become `in_sheet`); E-14, E-15 (no edit needed). E-16 to E-24, all pre-authorized and listed in the ledger: E-16 removed the dead `Heading::branch` field; E-17 `saved_document` visibility; E-18 `made_ready_tests` compares `READY_TO_START`; E-19 `reach_the_action_in`; E-20 12 helpers widened; E-21, E-22 `window_text` and `ask` visibility; E-23 P6's H12 "no dialog reopens" fact compares the lifecycle-dialog entity before and after (narrower read, judged no weaker, fails without the guard); E-24 `passphrase.rs` `mod tests` is `pub(super)` (cfg(test) only). The only assertion removal is E-3.

**Rulings I made** (all in the ledger; each with what it costs if wrong):
- T-1 trailer: the two Task 0 commits were rewritten to the Sonnet trailer before any push. T-2 design corrections are annotations beside the `ac50ba1a` text. T-3 U-9 answered "no code for 05a's openers" (the return to Settings cannot meet an open Resolve sheet): **flag, owner's eye**. T-4, T-5 E-11 and U-7 bookkeeping.
- O-1 `apply_review`'s refocus follows the H rows. O-2 File Map gaps (`prepare.rs`, `republish.rs`, `repair.rs`, `resumed.rs`) and a P2 row for a busy resume. O-3 helper visibility over copies. O-4 module-run shape.
- **O-5** the plan's Review Focus "Retry and Unlock… both last" cannot occur (Unlock… is offered only for a locked key and withholds Retry); three P2 rows replace it. **O-6** after Done on a closed ticket the origin control is gone, so focus lands on the ticket's tab: the Wave's "focus returned to the invoking control" cannot hold literally for a close (it does for Approve and merge…): **flag**. **O-7** the blocked prepare's five-button footer wraps onto a second line in the 580 px dialog (or Close and Check now were clipped): **flag, owner's eye**.
- O-8 B3's footer-press rule (`on_footer`) pulled forward into Task 5. **O-9** H13 reworded to the Result editor's own right-click menu (owner-approved at the Task 6 checkpoint). **O-10** `unkept_asks_first` also closes the Resolve sheet (owner-approved; Cancel loses the choices as the dialog did).
- Final wave: the `render.rs` split was not taken (the file predates the Cycle; the 500-line rule is for new files); the waiting-slot defect was fixed for safety though unreachable in production.

**Things you will see on screen** (design-mandated; also in the checklist): Blocked, Held, Saving and no-preview stages no longer name the branch (it is in the Git line only: A2, K-15, E-3); the blocked footer on two lines (O-7); closing a ticket ends with focus on its tab (O-6); a 0.25 s moment with no focus after a Confirm dialog closes over the Resolve sheet (not a table row).

**Deferred minor findings** are in the ledger; none blocks the merge.

**Not done, and not mine to do:** the on-screen checks, the review approval, setting the status and `ticket close` (which merges), and removing `target/` and `.devenv/` from the worktree before the close. No pull request, merge, close or native CI was done.