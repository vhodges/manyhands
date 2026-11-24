---
id: 01M86QGATBF30KG14N2MHPX19Z
item_id: 01M6KF58ENSF1QA7EYK89P0SMQ
created_at: 2026-11-24T12:11:50.635199457Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Scope widened (owner, 2026-11-24): this exploration is now the design ticket for Wave 06, Pull-request workflow (document 01M86QA59F7N2DFWTDP5ST2QAW, docs/Waves/wave-06-pull-request-workflow.md, on its own branch until promoted).

Besides the separate steps above, the RFC covers a pull-request mode: document promote, ticket close and repo sync end in a published branch that a person merges through a pull request, instead of merging into the primary branch and pushing it. This is for repositories whose primary branch is protected or changed only through review. Today's direct path stays available.

Questions the RFC answers, in addition to the Explore list:

- How a repository chooses direct or pull-request mode, and which is the default for an existing repository.
- What promote and close publish in pull-request mode, and what state the item is in while its pull request is open.
- How Manyhands learns that a pull request was merged, and what it cleans up afterwards. vh-c97se (onboarding on a setup branch) has the same question, so one answer covers both.
- When closed_at and closed_by are recorded in pull-request mode, given C5 decision 6 and the consent digest.
- What repo sync does when the primary branch cannot be pushed.
- Whether Manyhands opens the pull request itself or leaves that to the person.
- Primary branches not named main, throughout (vh-fk279 fixes the desktop copy).

Outcome unchanged: an RFC proposal for the owner, then Cycle planning. The implementation Cycles are filed from the approved RFC.