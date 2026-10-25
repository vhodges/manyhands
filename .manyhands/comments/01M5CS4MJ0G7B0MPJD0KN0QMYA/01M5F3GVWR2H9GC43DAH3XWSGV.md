---
id: 01M5F3GVWR2H9GC43DAH3XWSGV
item_id: 01M5CS4MJ0G7B0MPJD0KN0QMYA
created_at: 2026-10-21T11:28:45Z
created_by: Claude Opus 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---

## Preflight and owner-approved delivery split

Read the handoff brief, AGENTS.md and original ticket. From this ticket's
clean worktree, fetched origin and merged published main `0b2366453cb0228aaeb341c51a2dad963df4990f`
into checkpoint `857a73582d053ba1725c2607396701c39ed5c8b4`. Merge commit
`8fd42e729f149b4bbdbeb407767c4606782b5fde` is conflict-free and includes
origin/main by ancestry. The branch was not rebased.

The owner selected delivery option 1: move all non-Structure findings to a
new ticket so this ticket can close when the Structure work is approved and
merged. Created `01M5F3GVDYTH1Q5V5FCS9YF7V3`, "Desktop follow-up: settings UX
from the D1 reviews", in its own branch/worktree based on main `0b23664`.
The Repositories, Remotes, Keys, Hosts and Keyboard/accessibility findings
are retained there, along with the two unresolved product questions.

This ticket now owns only the behavior-preserving Structure refactor. Its
acceptance criteria carry forward the brief's shared Read and naming helpers,
narrow Workspace API, responsibility-based file splits, unchanged desktop
test count, and D2 page-switch integration boundary. D2 documents remain
read-only; Task 5 will remap its File Map after this work reaches main.

The separate tests-and-polish sibling `01M5CS4MH25QQ769HV61ZRKHXG` is still
at its initial checkpoint `c116a1a` with a clean worktree at this inspection.

Generated both IDs with the Manyhands CLI through Devenv. Ticket authoring is
not yet supported by the current CLI, so used canonical filesystem records.
The owner has approved the split, not yet the module layout or implementation.
