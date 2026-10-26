---
id: 01M5TGCERSDK7Q8RF5X62VKGFQ
item_id: 01M4CC0VMV81DFRGJ4Z76489XM
created_at: 2026-10-25T21:45:05Z
created_by: OpenCode <noreply@openai.com>
manyhands_managed: true
manyhands_kind: comment
---

## Task 9 Step 4 switch implemented; separate review pending

The approved switch follows all six successful saves/value checks/pushes recorded
at `8da3890f2c4f2ac6199324163346f72eb13818f8`. The original C4 checker stop and
fresh authorized resume remain recorded; no further real mutation was needed.

AGENTS.md now directs ticket create/update to the CLI against registered main,
from up-to-date main **after C3 merges**, using Devenv's locked CLI run. It names
the own-ticket commit-first rule, canonical comments until C4, canonical closure
until C5 and the working-on-a-ticket sandbox exception. That skill is unchanged.

file-ticket now covers CLI id new/create/show/save with distinct ticket/request
IDs, correct selectors, JSON stdin/body strings, omission/null semantics,
dependency ULIDs, schema limitations, clean checks and request-state recovery.
Classification, entry-gate context, canonical comment schema and authorization
boundaries are retained. New Cycle wave/cycle context goes in the body because
the closed create input does not accept those metadata keys; save preserves
existing unknown metadata. No hand-ticket creation/update remains as the normal
path. C3 ticket planning boxes are checked and Added Scope corrected for merged
main and C2's repository initials/prefix. Ticket front matter remains unchanged
and **open**.

Read-only documentation validation passed: **8 built CLI help contracts**,
**2 input JSON examples**, **4 relationship variants**, nullable save metadata,
skill frontmatter and ticket/AGENTS scope checks. No actual create/save example
was executed. Inline scenario self-review covered filing, Cycle context,
commit-first updates, accepted/pending recovery and the sandbox exception.
No agent-pressure evaluation or independent review is claimed; no subagents.
No Rust gates reran. Source/tests/Cargo and other workflow skills are unchanged.

Fetch/merge-tree and whitespace checks passed; main remains e49f978. Separate
commit `docs: switch ticket dogfooding to the CLI` contains only AGENTS.md,
file-ticket/SKILL.md, this C3 ticket, ledger and comment, with the required
co-author trailer and C3-only push. Comment ULID from built CLI id new, timestamp
from actual UTC clock.

**Controller's independent switch SPEC/QUALITY review remains pending** over
`8da3890..SwitchHEAD`. No Task 9 completion/review-ready claim. Historical full
suite **4528/1/1, exit 101** and scoped **46 distinct passing** plus check/fmt/clippy
remain accepted by explicit owner exception; no full-suite rerun. No PR, main
integration, closure, cleanup or native CI.

Evidence: `docs/plans/2026-10-24-wave-03-cli-03-authoring-and-discovery-execution.md`.
