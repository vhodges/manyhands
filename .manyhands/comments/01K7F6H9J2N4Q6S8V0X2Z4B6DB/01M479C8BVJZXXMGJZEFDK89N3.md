---
manyhands_managed: true
manyhands_kind: comment
id: "01M479C8BVJZXXMGJZEFDK89N3"
item_id: "01K7F6H9J2N4Q6S8V0X2Z4B6DB"
created_at: "2026-10-06T00:21:22Z"
---

PR #9 merged into `main` as `5f5bac0` on 2026-10-06. The final published head
`dbda637` passed all five native targets in
[run 37391879620](https://github.com/vhodges/manyhands/actions/runs/37391879620):
Linux x86-64/ARM64, Windows x86-64/ARM64, and macOS ARM64.

The final macOS log confirms the three permanent signal regressions, 31 fixture
cases, and 45 transport cases pass without the temporary probe or diagnostic
observations. The local final tree passed all required Devenv gates with 585
tests and no failures or ignored tests; independent reviews found no outstanding
issues.

The ticket was inadvertently left open during the final pre-merge update. The
user identified this after merging and requested reconciliation. This comment
and the closed ticket complete that lifecycle record. Branch/worktree cleanup is
being performed separately after verifying the merged ancestry.
