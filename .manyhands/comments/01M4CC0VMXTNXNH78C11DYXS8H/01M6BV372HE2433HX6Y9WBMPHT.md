---
manyhands_managed: true
manyhands_kind: comment
id: "01M6BV372HE2433HX6Y9WBMPHT"
item_id: "01M4CC0VMXTNXNH78C11DYXS8H"
parent_id: "01M6BPS4MKMGWCY2QM8K6K3B6B"
created_at: "2026-11-01T15:19:21Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 6: implemented, three rulings needed

Both fixes are in and their new rows pass, but the Part A gate can't pass yet.

1. Synchronization's identity refusal comes after the fetch, so the tracking ref and an unfinished operation row do change, as they do for a missing identity. The acceptance line "refs and journal unchanged" can't hold there.
2. `tests/repository_enablement.rs:3816` pins the stale index that `01M669FJR2` fixes. It fails two tests and isn't a named edit.
3. `identity_and_mismatch_do_not_strand_the_item` pins the record R-3 removes. It fails `--lib promotion` and isn't a named edit.

No pre-existing test was edited. The reproduction also found something `01M5MK57` doesn't describe: before the fix, a trimmed identity didn't fail, it committed and published under an altered name.

Decision 10 overrides `01M669FJR2`'s "merged on its own". A follow-up ticket is needed for `effective_identity`'s per-key cascade (R-6), and none has been created. The ledger has the evidence, counts and risks.
