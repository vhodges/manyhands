---
id: 01M6M2NKQM36F56GXXRWMHVQN6
item_id: 01M4CC0VN2K8DBHNMZ46NTX5JJ
created_at: 2026-11-04T20:05:47.416089528Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
## Part 0 checkpoint (Task 4)

**Part 0 is done.** Tasks 1, 2, 3 and 3b added L1 (way out), L2 (resume for lifecycle parents), L3 (lifecycle progress) and L4 (a protected folder by name). Each task had an independent review, and every finding was fixed or recorded as a ruling or known limit. See the execution ledger.

**Main.** `origin/main` `c69789d9` merged as `074ebde0`, with no conflicts. Closure now runs through `ticket close`, and plan Task 16 follows it.

**Gates at `074ebde0`, all pass:**
- check, fmt, clippy (`-D warnings`) and check-cli-no-gpui;
- the fast tier: 5195 passed, 0 failed, 1 ignored;
- the SSH tier: `mutation_remote` 708 (84 min), `document_promotion` 79, `ticket_closure` 78, `cli_remote` 161.

**Owner rulings, 2026-11-04:**
- Task 5 runs in parallel in a side worktree; the branch merges back and is never rebased.
- Named edits E-10 (the intent tables) and E-11 (Tab order).
- Standing approval for test expansion: no assertion weakened, each edit named.
- Follow-up: AGENTS.md's SSH-tier timing is out of date, because `mutation_remote` alone runs about 84 min. Update it when D5 is done.

**Next.** Part A, Task 5 (intents, header, test seams), now in progress.