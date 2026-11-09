---
id: "01M6MGTPJ2BNNXKN9WZ183RWQA"
item_id: "01M6KHVD6D0DB8KYT6VGQP10Y0"
parent_id: "01M6MFK79ZVEMZN6JEEGVZQ4WB"
created_at: "2026-11-05T00:15:46Z"
created_by: "GPT-6.1 Sol <noreply@openai.com>"
manyhands_managed: true
manyhands_kind: comment
---

## Task 1 quality correction: validate old evidence before widening its enum

Controller confirmed **one Important** quality finding against Task 1 through
`0698671cd293e6da43b27979b63a93105074a653`: old15-invalid `closed`/`promoted`
rows inserted with CHECK bypass were accepted when copied into the wider17
table, laundering corrupt evidence. The old `substituted` fixture failed both
CHECKs and missed this distinction. Strict corruption refusal was already
approved; no new policy or owner checkpoint was required. Controller reports
the preceding spec pass held; quality approval remains pending re-review.

Source correction: `ed899a00598c2fe17168c1465974476ddc2b196e`,
`fix: reject corrupt legacy poll dispositions before upgrade`, required GPT-6.1
Sol coauthor trailer present. Only the exact-old branch of `migrate_poll_dispositions`
now explicitly validates all existing dispositions against **old15** literals
before any trigger drop, rename or widening, in the same migration transaction.
NULL/unknown values refuse through the existing recovery error. Current17
`closed`/`promoted` remain valid; no global enum ban or unrelated migration change.

The corruption matrix now contains **10 rows**. Both new values are injected
only in temporary old-schema fixtures, with bypass turned OFF before migration;
integrity checks first prove the original old CHECK violation. Every refusal
retains exact whole schema and schema_version plus all columns/rowids of both
disposition and owning-operation rows, not just their count. Enum-corruption rows
also prove the retained immutable-owner trigger still raises its original error.
The valid old15 upgrade preserves all rows/OIDs, and a new current17 reopen/decode
control accepts both new values without changing schema version.

Exact verification (Cargo always through Devenv):

- `devenv shell -- cargo test --all-features --locked --lib retirement_disposition`
  before fix: compiled, then behavioral RED **1 passed / 1 failed**. The failure
  listed **both** old-schema `closed` and `promoted` cases as unexpectedly accepted.
- Same focused command after fix, clean serial run: **2 passed**, 0 failed.
- `devenv shell -- cargo test --all-features --locked --lib repository::remote::`:
  **427 passed**, 0 failed, 223.77 s; sources frozen, no concurrent compilation or
  process-spawn errors. Includes genuine pre-envelope and recognition controls.
- `devenv shell -- cargo fmt --check`: exit 0; staged/unstaged and full source-range
  diff checks clean. Raw evidence: `/tmp/opencode/task1-quality-fix-*.log`.

An initial post-fix Cargo run passed but concurrent Devenv startup reported a
missing `.devenv/load-exports`; focused tests and format were repeated serially
with clean startup. No environment configuration or checks were changed.

No redundant standalone proof/read-contract/CLI-polling/H2 reruns: their source
and ordinary valid-data setups are unchanged. Prior 6/72/23/1/1 results remain
historical, not rerun claims. Focused tests cover old/new envelopes and actual
current17 reopen/decoding; the complete remote gate covers migration/poll controls.
Remaining whole-tree check/Clippy, fast/changed-path SSH tiers and native CI stay
with the controller/owner's Task 8 and post-merge gates.

Narrow correction range:
`0698671cd293e6da43b27979b63a93105074a653..ed899a00598c2fe17168c1465974476ddc2b196e`.
Full Task 1 source-review range:
`834415b95825f84d15ce00a5586316abcfe1a6f7..ed899a00598c2fe17168c1465974476ddc2b196e`.
This checkpoint and the [execution ledger](../../../docs/plans/2026-11-04-completed-context-retirement-execution.md)
follow in a documentation-only commit; re-review uses `834415b9` through returned
HEAD, **not HEAD~1**.

Next: same quality reviewer re-reviews the correction. No quality pass marked by
implementer. No Task 2, additional agent, push, main integration, real-registry
change or ticket status/body save.
