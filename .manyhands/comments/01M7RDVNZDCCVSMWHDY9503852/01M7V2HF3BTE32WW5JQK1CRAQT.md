---
id: 01M7V2HF3BTE32WW5JQK1CRAQT
item_id: 01M7RDVNZDCCVSMWHDY9503852
created_at: 2026-11-19T23:34:02.384528723Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Planning checkpoint 3, 2026-11-19. The design `docs/plans/2026-11-19-fast-writes-design.md` is at `5eeb7924` on this branch (base `b730e7d7`, main `790a71ed` merged in, not rebased). No build or test has been run; only documents changed.

**Review rounds:** 1 on `4e126a0b` (three reviewers): 8 blocking, 29 important, 9 minor. 2 on `47f43280` (scoped, adversarial): 3, 10, 9. 3 on `a4f95c8f` (scoped, adversarial): 1, 5, 7. Round 3's fixes (in `5eeb7924`) have had no independent review. Presentation across contexts and what a pending row holds drew findings in all three rounds, so per the planning process they are now stated as required-outcome tables (A and B in the design) that the implementation builds test-first, with a design checkpoint for the owner before the code continues.

**Owner decision added:** on R-3, "leaving the record open until resumed is fine" (a crashed save whose commit landed no longer blocks refresh and poll, which index its item; its own row stays open until its own resume or retry).

**Correction to checkpoint 2** (`01M7V0RT95T2QZ89SF00K0YBCS`) and checkpoint 1: the retry ruling is R-7 in the design, not R-6.

**Known exceptions written into the design:** a crashed `context sync-local` still blocks refresh and poll until resumed (its row keeps repository scope); on Windows a same-size edit that preserves mtime is not detected until the content is next read.

**Asked of the owner now:** approve the design. Next after approval: the implementation plan (tasks per phase, the named edits to existing tests for approval, the execution method), then implementation once separately authorised.