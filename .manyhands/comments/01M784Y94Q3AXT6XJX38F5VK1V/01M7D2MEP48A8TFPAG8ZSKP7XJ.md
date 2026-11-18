---
id: 01M7D2MEP48A8TFPAG8ZSKP7XJ
item_id: 01M784Y94Q3AXT6XJX38F5VK1V
created_at: 2026-11-14T13:06:04.494266518Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Follow-up to checkpoint 5 (2026-11-14): two things changed after review-ready.

**Document amendments done (final-review item m-2).** At the owner's request, design Document Amendment 13 now lists the Task 5 rulings the owner approved (C1 to C7, C9) and four implementation rulings (T9-2/T10-1, T11-1 to T11-3, with the final review's popover-focus change). The Cycle document points to it. Comment 5 said this was open; it is closed.

**A `/code-review high` fix wave** (the owner pasted its ten findings). One implementer verified each by reading; one scoped re-review: all addressed, no new breakage. Fixed (7 commits, `5ad7abd2` to `f3b6a57f`): the settled latch's `Some(0)` against `None` comparison (a latent defect, since every real request has an attempt number; new T14 row seen failing first); the O(n squared) retire-event scan; the pending list read once per refresh and a slot index for `Operations::request`; the Can't reach icon read from a `Derived.unreachable` field, not the title text; duplicated share, `dto_kind` and `TargetKind` helpers; read-only menu lines with no action; the `Intent` match made exhaustive. Left: finding 4, `rows_fit_a_narrow_activity` stays because it pins owner decision 22 and Acceptance A8, though it conflicts with AGENTS.md's no-layout-tests rule (you may delete it; ruling T15-1); findings 7 and 8 were not real (the wording is the D5 dialog's own, and `titles` is only a change cache, never shown). Nothing is pruned (K-4).

**Tests:** static gates clean at `f3b6a57f`. The whole desktop binary ran once: 1,913 passed, 1 failed, `the_more_menu_acts_on_the_chosen_repository` (a real-time `until_drawn` wait ran out under the 23-minute load). It passed alone three times and in its module twice. That is a third Cycle 03a repositories-panel window test to flake in a full run, after the two in checkpoint 5 (hardened). I ruled it a load flake (T15-2) and did not touch it; the branch edits neither the test nor the harness, but the final review judged the branch's Activity derivation work plausibly makes these races likelier, so you may want these tests' waits reviewed.

**Branch** `manyhands/ticket/01M784Y94Q3AXT6XJX38F5VK1V` pushed, HEAD equals origin; `origin/main` still `ba95e1b7`; merge-tree clean; boundary diff empty.

**Still waiting on you:** the on-screen checklist and desktop smoke test (A14). Not done, not authorized: a pull request, merge into main, closing the ticket, worktree cleanup, native CI.