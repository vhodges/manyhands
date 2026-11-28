---
id: 01M8DFWDTNYDY1KD2M53KW1MED
item_id: 01M87684C9GM9BJP5QQ429KMAK
created_at: 2026-11-27T03:13:19.736276177Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Checkpoint after Tasks 1 and 2 (2026-11-26). Branch head c7b1607d; main has not moved (8e5d9832), so no merge. Subagent-driven: Sonnet 5.5 high implementers, an Opus reviewer per task, a Sonnet re-review per fix round.

Task 1, the record's mode (commits fb666745, a4586e67). document_promotions is rebuilt once under wave_06_promotion_modes_v1 (SCHEMA_V2 frozen; explicit column list in rowid order) with integration_mode, answer_code, consented_context_oid, situation, primary_branch and endpoint_digest; per-mode CHECKs, permits and validate; PromotionAnswer and PromotionSituation; path() and answered(). The design checkpoint kept all six columns and left begin_record and its 32 callers unchanged (Task 3 adds the pull-request begin with its caller). Review: one Important (five new schema assertions were never red and hit table CHECKs); fixed in one round, each column CHECK now pinned on a row only it can refuse. Named edits applied: E-3, E-4, E-15.

Task 2, the pull-request observation (commits bcf29349, 177be9ec). primary_holds_document (R-3, the content proof against the tracking primary and a local primary behind it, R absent, equal or behind); Part B's order, blockers kept and dropped, history_unknown not blocking; the digest under manyhands pull request promotion v1; the prepare's already_applied with no data through the withdraw-prepare shape; the held observation's own digest at confirm; the three effects always. The link builder is shared with Cycle 03's (context_branch_link). An interim guard stops a pull-request confirm before any record until Task 3. Review: the four named risks checked out (direct observation unchanged, root-merge blocker, R-3's reading, no path that moves the primary); one Important (the effects test compared a constant with itself, and the held answer was untested); fixed in one round. Named edits applied: E-12, E-16, E-17.

Tests ran only by exact name (the new tests, the edited ones and the direct tests that pin moved code), each red first; static gates clean after each commit. No SSH target and no tier has run. Thirteen Minor findings are deferred to the final review (listed in the ledger).

Next: Task 3, the confirm and its two answer tables (C1 to C11), with a design checkpoint first.