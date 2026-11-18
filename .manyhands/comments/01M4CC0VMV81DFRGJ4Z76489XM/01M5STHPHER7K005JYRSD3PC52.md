---
id: 01M5STHPHER7K005JYRSD3PC52
item_id: 01M4CC0VMV81DFRGJ4Z76489XM
created_at: 2026-10-25T15:23:28Z
created_by: OpenCode <noreply@openai.com>
manyhands_managed: true
manyhands_kind: comment
---

## Owner exception: persisted closure-test baseline corrected

The owner explicitly authorized changing only the baseline in
`src/repository/closure/checkpoint_tests.rs::mismatched_confirmed_identity_persists_nothing`,
as an exception to C3's frozen-library/pre-existing-test rule, with exact-test
and checkpoint-module verification. No other fix or second full-suite run is
authorized. Task 8 spec/quality remains **APPROVED** at `f9618f7`.

Added only a comment and `let r = f.reload(&r);` immediately after
`confirmed_record`, before observations/hook. Every no-effect assertion remains
intact, including full persisted-record equality. No production code/helper,
other test, Cargo, equality or Debug change.

Source inspection confirms the helper can return the insert-time in-memory
record after a save advances the database's whole-second `updated_at`.
Derived equality includes this field; Debug omits it. The mismatch is refused
before writes. The correction follows existing precedent `309c9e0`.
**The original failed run's hidden values were not captured:** this mechanism
is source-supported, not a proven historical timestamp diagnosis.

Scoped post-fix results, each command run once through Devenv:

- `cargo test --all-features --locked --lib repository::closure::checkpoint::tests::mismatched_confirmed_identity_persists_nothing -- --exact`:
  **1 passed / 0 failed / 0 ignored / 1269 filtered**, 0.39s.
- `cargo test --all-features --locked --lib repository::closure::checkpoint::tests`:
  **46 passed / 0 failed / 0 ignored / 1224 filtered**, 10.15s; includes the first test.
- `cargo fmt --check`, `cargo check --all-features --locked`, and
  `cargo clippy --all-targets --all-features --locked -- -D warnings`: passed.

The original full suite remains **4528 passed / 1 failed / 1 ignored**, exit
**101**, 6239s at `f9618f76ce9608c9ccc42018701647d7b98bb642`. Its log and status
files are preserved. **Full gate is not green; no full-suite rerun occurred.**

Fetch/merge preflight and checkpoint merge-tree passed; main remains `e49f978`,
already incorporated. The prior uncommitted ledger and gate comment are
preserved in this separate fix/evidence commit, along with this comment.
Its ID came from CLI `id new`, timestamp from the UTC clock.

Controller review of this fix and the whole branch, followed by owner full-gate
disposition, are pending. No Task 9 completion/review-ready claim. No subagents,
dogfood preflight, switch, PR, main integration, closure, cleanup or native CI.

Full evidence: `docs/plans/2026-10-24-wave-03-cli-03-authoring-and-discovery-execution.md`.
