---
id: 01M7FN74K4DCFMKV0DH9CPR6GA
item_id: 01M7D5Z7FZTGTC1HE25KQ1MYQY
created_at: 2026-11-15T13:09:19.972727674Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Task 7 checkpoint, 2026-11-15 (execution session).

**Where.** Tasks 0 to 7 are done and pushed; Task 7 (Share changes through) is commit `a14e862d` (base `d7b39f62`). Step 1's merge of `origin/main` (`14ca0938`, Cycle 04 landed) was a no-op: main has not moved since `9059778f`. Since the Task 4 comment: Task 5 (Add an existing folder) `1ff07fe3` with fix `0d14abe6`; Task 6 (Create a new repository) `cb78fe3d`; Task 7 `a14e862d`.

**Tests (module runs by the orchestrator, each filter matched at least one test):**

- Task 5: 37 passed, 0 failed; after its fix round, 30 passed, 0 failed.
- Task 6: 40 passed, 0 failed.
- Task 7: 42 passed, 0 failed (`add_repository::tests` 26, `add_sheet::tests` 9, the copy table 1, the Settings keyboard walks 6).
- Static gates pass on each commit (fmt, check, clippy). The fast tier runs once, at review-ready (Task 10).

**Reviews.** Task 5: one fix round (two Important test-evidence gaps: the new-mode reopen never read the mode, and a dead `add-inspection` absence check), then approved. Tasks 6 and 7: approved first time. No Critical findings anywhere. Minors are parked in the ledger.

**Test edits with assertion changes** (called out for the review-ready summary): Task 5 removed the Add preview's code and message assertions (B6 draws "Not a Git repository." with no code), `add-message == NEED_FOLDER`, and the P18 copy-table rows for `ADD_HEADING`, `NOT_ADDED` and `CHECKED_OUT`; Task 6 removed `create-message == NEED_FOLDER`, a dead `!present("add-message")`, and the copy-table rows for `CREATE_HEADING`, `CREATE_FOLDER`, `NEED_FOLDER` and `NEED_BRANCH` (each constant is deleted). Two below-the-fold test windows were shortened (250 to 150 px for P23 and E-1e's test) because one form is now drawn at a time; their mutation checks still fail. Task 7 edited no existing test.

**For the owner's eye** (ledger, parked Minors):

- **R-30 against C2:** C2's second case sends "stop sharing" for Keep over a configured eligible remote; R-30 holds the select on Keep for a Valid configuration with no identity. The code follows R-30 and sends nothing there; C2 needs "unless held (R-30)".
- **B6 and decision 9:** a Valid configuration whose publication remote is not among the folder's remotes draws no row and Add stays enabled; the library then refuses the preview. B6 and decision 9 differ on whether Add is disabled.
- **B8 and Part F:** B8 titles the identity block "Your name & email (optional)" but Part F has only "Your name & email"; the code follows Part F.
- **Ruling 15 realised unconditionally:** every open of the Add sheet records Settings' selection, `+` included.
- **Windows (native-CI follow-on, not a gate):** two test rows depend on the path separator or canonicalisation (P12's `will_create("not/made/yet/fresh")` literal and P14's `sub/../registered`).
- **Part F gaps for P20 (Task 9):** `ADDING`, `SHARE_NEEDS_IDENTITY`, the formatting fns `will_create`, `other_main_branch`, `not_ssh_remote`, and `CHOOSE_ANOTHER` and `OPEN_SETTINGS` (no `…`).

**For the final review:** a missing row in P14: R-26's "an earlier open's success still sends" is only exercised with the sheet closed, never reopened, so a session-equality mutation would pass.

**Branches published:** `manyhands/ticket/01M7D5Z7FZTGTC1HE25KQ1MYQY` (this one), at `4fb4c234` plus this comment. No other branch.

**Next:** Task 8 (the Unlock key dialog).
