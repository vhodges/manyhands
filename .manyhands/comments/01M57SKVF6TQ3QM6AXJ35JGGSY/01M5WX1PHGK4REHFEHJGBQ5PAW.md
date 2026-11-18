---
id: 01M5WX1PHGK4REHFEHJGBQ5PAW
item_id: 01M57SKVF6TQ3QM6AXJ35JGGSY
created_at: 2026-10-26T20:04:52Z
created_by: GPT-6 Astra <noreply@openai.com>
manyhands_managed: true
manyhands_kind: comment
---

## Task 6 SPEC: four verified corrections, awaiting re-review

Base **`3dc7d15616819fcdef49164182614b2195ce1877`**. SPEC
`ses_e864b2dfbffemfrKBcgnvISO55` requested changes for four Important findings.
Each was source-verified and reproduced with failing domain tests before fixing.

1. Whole-result memory-backed candidate validation now checks all three frozen
   trees before accepting R or writing paths, including auto-merged stage-zero
   canonical entries. The closed-ticket counterexample retains zero attempts,
   zero owned writes and an unchanged persistent object/file/index/ref image.
2. Existing backend-lock and exact branch/HEAD log proofs run before fresh
   acceptance, partial retry and the next path/object/index/ref effects. Twenty-three
   fresh/partial/boundary cases preserve foreign locks and same-OID log changes
   without additional Git effects; restored evidence permits exact R completion.
3. Existing fully validated owned candidates reconcile their objects observation
   before continuation. Four actual child exits occur after commit import but
   before the SQL observation. Both kinds and R/source-free continuation retain
   Checkpoint Committed through completion and domain replay, with one candidate/ref
   effect. Foreign signature and missing attempt-binding controls still refuse.
4. Artifact-free abandonment dispatches to actual integration provenance. Equal,
   contained, FF, clean merge and exact external repair pass for both kinds;
   dirty-file and backend-lock controls preserve authority and Git until restored.
   Existing canonical-R and unsafe-abandon controls remain green.

Final Devenv fmt/check/all-target-all-feature clippy -D warnings pass.
**150 local-sync tests + four separately selected shared anchors = 154 passing
tests**. Eight new tests include four actual no-Drop child exits; the local group
now contains 21 actual exits, excluding ordinary child-entry invocations.

The ledger records exact RED/GREEN commands, effects/authority and the full
abandonment disposition matrix. Its earlier blanket all-domain-outcomes claim is
qualified pending independent review. No old assertion, public F2 binding,
request settlement or Task 7 activation was changed. S/R/U atomic abandonment,
including unlinked intended-parent requests, remains explicitly unimplemented.

Work stayed in this ticket worktree/branch, with no subagents, policy change,
full-suite/native run, PR or lifecycle action. Main remains incorporated and
merge-tree passes. Await **SPEC re-review, then QUALITY**; these are verified
implementer fixes, not reviewer approval.
