---
id: 01M5XRX8YK4GFJNZ6YEXD99GGV
item_id: 01M57SKVF6TQ3QM6AXJ35JGGSY
parent_id: 01M5XPDY946AP10ZTA617CNMBV
created_at: 2026-10-27T04:11:48Z
created_by: GPT-6 Astra <noreply@openai.com>
manyhands_managed: true
manyhands_kind: comment
---

## Task 7 SPEC: fresh-conflict untracked-path correction

Base **`8e7d6dad0de081883a757293f47d68682f4e6069`**. Controller reports the two
preceding inspection gaps closed, with a fresh-conflict dirty-path gap remaining.
Source verification confirmed that the read's indexed-path image omitted unrelated
untracked files while execution separately enforced `only_paths_are_dirty`.

Fresh-conflict inspection now calls that same existing predicate with immutable
raw conflict paths, on both proof passes. No weaker duplicate check, write lease
or resolution authority is introduced. Ignored unrelated files remain permitted.

Document and ticket public tests both ran RED (Resolve instead of ExternalRepair),
then GREEN. They positively establish nonignored WT_NEW status, check
external_repair/can_start_resolution=false while holding the repository lease,
and compare Git administration/worktree bytes, SQL and image proofs. Removing the
foreign file restores the original resolvable observation. An actual resolution
succeeds with the unrelated ignored file still present and unchanged.

Whole-tree Devenv fmt/check/clippy pass. Targeted selections: **52 local mutation,
6 local read, 44 mutation-contract, 72 read-contract and 1 replay conformance =
175 passing tests**. The ledger records exact commands and scope. No old assertion,
predicate/executor implementation, schema, golden, dependency or frontend changed.

Pending controller **SPEC re-review then QUALITY**; no approval is claimed.
Ticket remains implementing. No Task 8, PR, closure, merge into main, native CI
or cleanup. Latest main `0464c42` was already incorporated and merge-tree passed.
