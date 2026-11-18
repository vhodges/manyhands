---
id: 01M7EP29DR622GSWPJECN9830R
item_id: 01M7D5Z7FZTGTC1HE25KQ1MYQY
created_at: 2026-11-15T04:04:55.234575041Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Task 0 checkpoint, 2026-11-14 (execution session).

**Owner override.** Cycle 04 (`01M7AMAPNHHTNECXBF4CA5WPKV`) is not closed or merged: its implementer is chasing a flaky test's root cause. The owner directed that Task 0 merge Cycle 04's branch instead of main, in place of the handoff's "stop if Cycle 04 is not merged". Merged at `f5015aab` (Cycle 04 at `7a484a7a`) and again at `edb0e920` (at `5bacd7d2`, test-only fix `f0cfaa39`). No conflicts. Cycle 04's tip is merged again before Task 7 and before review-ready, or main instead once Cycle 04 lands. Cycle 04's own comment files came in with the merge.

**Commits** (base `f5015aab`): design re-map `81c3c0de`; Wave Amendment 1 `5b7771db`; fix `42f807d5`; ledger `4c515b5b`; merge `edb0e920`. Nothing under `src` outside `src/desktop`, `tests`, `Cargo.toml`, `Cargo.lock` or `packaging` differs from main. Cargo.lock is unchanged, so gpui-kit, gpui-base and gpui-component did not move.

**Tests and gates.** Static gates only, at `f5015aab`: `cargo check --all-features --locked`, `cargo fmt --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings` all pass. No test ran.

**U-1: Cycle 04's code carries the design.** AW-3's origin (`take_origin`), a quiet close that empties the passphrase, `Manage(root)`, and a refusal point in the Ctrl+, handler all exist. No stop. The design's citations were re-mapped, changing no contract, G row, P row, ruling or decision. U-3 (A3's footer table), U-4 (Unlock evidence paths) and U-8 (no read double can fail a read, so P18 needs a `#[cfg(test)]` hook on the dialog) are answered in the ledger.

**Review.** One Important, fixed in round 1 (scoped re-review: addressed): the Wave amendment named 05b's planned document differently from its ticket. It now reads `docs/Cycles/wave-04-cycle-05b-close-and-resolve.md`. Minors are parked in the ledger.

**Rulings** (ledger, Rulings 1 to 10): Task 3 also moves three Add and Create tests the plan's E-1 list omitted; E-17a (P18 copy table, Task 3), E-17b (P23 Add-form rows, Task 4) and E-17c (P10 Inspect rows, Task 4, an assertion change) are numbered under decision 8; Task 1 adds four icons to `icon_assets!` (`KeyRound` is already there); Task 3 delegates the Add inputs' accessors so Cycle 04's tests stay green; the trailer is Sonnet 5.5; Task 10 Step 6's cleanup is not run (the handoff binds).

**Branches published:** `manyhands/ticket/01M7D5Z7FZTGTC1HE25KQ1MYQY` (this one, now at `edb0e920` plus this comment's checkpoint). No other branch.

**For the owner's eye.** Both tickets link the Wave's `#cycle-05-existing-sheets-and-dialogs` anchor, which no longer exists since the heading split. Part F's `KEEP_LOCAL` ("Keep on this computer") words the sharing choice differently from Cycle 04's Settings copy (`SELECT_NONE`, "Keep changes on this computer").

**Next:** Task 1 (the frame and theme).
