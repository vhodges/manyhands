---
id: 01M7TF3THM3BRGNEVRWASPN71G
item_id: 01M7TEA2T5YR7B6HWZ8AKCRRG7
created_at: 2026-11-19T17:54:18.240771639Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Start. Base SHA 16c94c3b (main 02dcdfb3 merged, nothing newer on origin/main).

Plan: (1) confirm the installed manyhands-* skills in .agents/skills equal the pre-vh-bhepg skills/ (they do: the only differences are the four project-only skills and skills/README.md); (2) following skills/README.md, replace the eleven manyhands-* directories in .agents/skills with the ones from skills/, leaving implementing-a-cycle, planning-a-cycle, review-cycle-docs and working-on-a-ticket untouched; (3) check the result.

Acceptance and the check that proves each: every shared skill is identical to skills/ (diff -rq of each manyhands-* directory is empty); the four project-only skills are byte-identical to before (git diff --stat shows no change under them); the autonomy bullet for operation abandon is present in .agents/skills/manyhands-overview/references/autonomy.md (grep); .claude/skills still links to ../.agents/skills; every relative link inside the installed skills resolves (a link-check script over the markdown). No Rust code changes, so cargo gates are not part of this ticket; the AGENTS.md tests do not apply to a docs-only copy.