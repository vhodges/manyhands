---
id: 01M5WYJ5NR8CQ09A8JMGPDX847
item_id: 01M57SKVF6TQ3QM6AXJ35JGGSY
created_at: 2026-10-26T20:31:21Z
created_by: GPT-6 Astra <noreply@openai.com>
manyhands_managed: true
manyhands_kind: comment
---

## Task 6 SPEC re-review: two remaining compositions corrected

Base **`4104895b4ccb90c4410eb64cc035146cace7cbc9`**. Controller reports SPEC
`ses_e864b2dfbffemfrKBcgnvISO55` closed whole-result preflight and early lock/log
checks, with two Important findings remaining. Both were source-verified and
reproduced before fixing:

1. Parent/active-R candidate agreement is now an unconditional shared authority
   audit invariant. S's error-path observer cannot promote a mismatched candidate,
   and an already-observed step cannot bypass the binding. Four regression
   compositions execute S twice, then resume and exact R; all refuse without
   objects-step promotion or file/index/ref/object effects. Existing resolver
   foreign-object controls remain green.
2. External abandonment provenance now comes from the observed external-reference
   evidence, not installation history. Both kinds pass after valid external repair
   either before installation or after actual installation interrupted before
   manifest observation, followed by discovery failure. Missing/unobserved/wrong
   external evidence and foreign index locks still refuse without authority or Git
   changes; restoring the lock fixture permits safe abandonment.

Final Devenv fmt/check/all-target-all-feature clippy -D warnings pass.
**153 local-sync tests + four separately run shared anchors = 157 passing tests**.
Three new tests cover four candidate-binding compositions, four external-repair
positives and eight negative/restoration controls. The existing 21 actual no-Drop
exit assertions remain green; no new process-exit count is claimed.

Exact RED/GREEN commands, source findings, effects/provenance and scope are in the
execution ledger. No Task 7/F2 changes, public S/R/U settlement claim, subagents,
full-suite/native run, PR or lifecycle action. Overall completion remains
qualified. **Await SPEC re-review; QUALITY must not start until SPEC is green.**
