---
slug: vh-bes2b
id: 01M7TEA2T5YR7B6HWZ8AKCRRG7
title: Refresh the installed skills in .agents/skills
type: task
status: done
project: manyhands
manyhands_managed: true
manyhands_kind: ticket
---
Follow-up from vh-bhepg (concurrent writers: attribute, serialize, name the blocker), closed 2026-11-19 and merged as 02dcdfb3. Design: docs/plans/2026-11-18-concurrent-writers-design.md.

The merge of vh-bhepg changed the skills under skills/ (manyhands-recovery, -overview and its references, -comments, -sharing-and-sync, -finding-work, -repositories, -polling-and-index, errors-and-exit-codes) but the installed copies in .agents/skills, which .claude/skills links to, still hold the old text, so agents in this repository do not yet know about data.blocker, the wait, per-item scope, operation list attribution or the hand-over of operation abandon.

Wanted: update the installed copies from skills/ without removing the project-only skills that live only in .agents/skills (implementing-a-cycle, planning-a-cycle). Check how the earlier install (commit ca9f55dd) was done and repeat it, or add the step to the skills' own install documentation if it is missing.

Acceptance: diff -r of each shared skill between skills/ and .agents/skills is empty; the two project-only skills are untouched; the autonomy bullet for operation abandon is present.