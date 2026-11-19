---
id: 01M7TFABK6BJAVFAGDTJNJFRD6
item_id: 01M7TEA2T5YR7B6HWZ8AKCRRG7
created_at: 2026-11-19T17:57:52.300632308Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Review-ready at the branch head (commit cd28cf32 plus checkpoints; main 02dcdfb3 merged, `git merge-tree` clean, nothing newer on origin/main).

Done: the eleven manyhands-* directories in .agents/skills were removed and copied again from skills/ as skills/README.md describes. Ten skills changed (comments, finding-work, overview with autonomy, cli-conventions and errors-and-exit-codes, polling-and-index, recovery, repositories, sharing-and-sync); documents, getting-started, keys-and-hosts and tickets were already identical. The four project-only skills are untouched and .claude/skills still links to ../.agents/skills.

Acceptance, with the check: `diff -rq` of each of the eleven shared skills between skills/ and .agents/skills is empty; `git diff` over implementing-a-cycle, planning-a-cycle, review-cycle-docs and working-on-a-ticket is empty; the operation abandon hand-over bullet is at .agents/skills/manyhands-overview/references/autonomy.md:28; all 269 relative links in the installed manyhands-* markdown resolve.

Independent review (fresh reviewer, read-only): no Blocking or Important findings. Its minor notes need no change: the AGENTS.md comment-add paragraph is compatible with the new partial/busy rows (it could optionally mention repeating the same --request-id after busy). No Rust code changed, so no cargo gate or test applies to this ticket.

Waiting for the owner's approval, then ticket close.