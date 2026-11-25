---
id: 01M89S92SWHNQGZJ3ZJ1QH5KG1
item_id: 01M876851FT3NPSH9KDXKXR1AJ
created_at: 2026-11-25T16:40:34.19083177Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
From Wave 06 Cycle 02 (vh-c97se), approved 2026-11-25 and now in implementation (Cycle document 01M89CHP8JBKETDQSZGBPVRTET, design 01M89CHPWV2W6RY0XART0416Q3, plan 01M89CHQHEV6NF0X0CTBC1G1AB, managed documents on their own branches).

- A remote setup branch, manyhands/setup/<ULID>, that survives the setup's merge is left in place: deleting it is this Cycle's guarded deletion (RFC decision 22; Cycle 02 K-8). Activation deletes only the local setup branch.
- Cycle 02's setup poll answers waiting_for_checkout when the root checkout is not on the primary or not clean (untracked files count). The checkout warning stays this Cycle's.
- The end of every poll once checks for a setup to complete (Cycle 02 design Part B, R-17); with no setup record it does nothing. Cycle 02 activation fast-forwards the local primary only when the root is on the primary, clean and not ahead.