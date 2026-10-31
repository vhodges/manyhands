---
id: 01M5SVJZAQ75PHG9TM53X1S1AP
item_id: 01M4CC0VMV81DFRGJ4Z76489XM
created_at: 2026-10-25T15:41:38Z
created_by: OpenCode <noreply@openai.com>
manyhands_managed: true
manyhands_kind: comment
---

## Verification exception accepted; dogfooding preflight only

The owner answered YES to the controller's stated interpretation: **accept the
original full suite plus scoped post-fix verification as an explicit Task 9
verification exception**. Original **4528 passed / 1 failed / 1 ignored**, exit
**101**, 6239s at `f9618f7` remains historical evidence, not a green full run.
Post-fix **46 distinct passing tests** and check/fmt/clippy are retained.
No second full-suite invocation occurred; hidden historical timestamps remain
unknown. Whole-branch SPEC/QUALITY approvals recorded in `a79a3bd6` remain intact.

Design Dogfooding step 1 build passed:
`devenv shell -- cargo build --locked --bin manyhands-cli`, exit 0, 0.28s.
Step 2 inspection used the absolute C3-built CLI with `--json repo inspect
--repo /home/vhodges/work/src/manyhands`: exit 0, success/ok, registered true,
main checked out, valid configuration, primary main, publication remote origin,
available identity, no configuration problems/recovery, all effects not_requested.

Default data path is **`/home/vhodges/.local/share/manyhands`**, resolved from
source and actual environment: HOME `/home/vhodges`, XDG_DATA_HOME and
MANYHANDS_DATA_DIR unset. Only the explicit inspect call touched default service
data. Target preflight used registered worktree paths and read-only Git probes
with optional locks disabled; no target body/source was read.

All six targets are clean, with no origin branch or upstream, each 865 commits
behind main. Local tips and existing commits to publish:

| Target | Local tip | Existing commits |
| --- | --- | --- |
| C4 | `aac4217` | `aac4217`, `6078dfa` |
| C5 | `01d8a26` | `01d8a26`, `71aee19` |
| D4 | `f451325` | `f451325` |
| D5 | `03176b7` | `03176b7`, `6157418`, `81a2198` |
| D6 | `e877e4b` | `e877e4b` |
| G1 | `7b306d1` | `7b306d1` |

The ledger records exact IDs, worktree paths, full tips, commit subjects and
proposed deps: C4→C3+gate; C5→C4+gate; D4→D3; D5→D4+gate; D6→D5; G1→C5+D6.
**No preflight blocker found.** A separate owner go-ahead is still required
for refresh/six saves/six pushes. Pushes would create six remote branches and
publish these **ten existing commits**, plus new save checkpoints. Saves
re-emit whole front matter, preserving other values while changing deps.
An interrupted save followed by a ticket edit can block new domain work until
the request's bytes are restored or C4's operation abandon is available.

**Dogfooding and switch OWED; Task 9 not complete/review-ready.** Verification
acceptance does not authorize mutations/publication. No actual refresh, save,
target push/merge, AGENTS/skill edit, closure, cleanup, PR or native CI occurred.
Fetch/merge-tree passed, main remains `e49f978`; only C3 evidence is committed
and pushed. ID allocated by the absolute CLI's `--json id new`, timestamp UTC.

Evidence: `docs/plans/2026-10-24-wave-03-cli-03-authoring-and-discovery-execution.md`.
