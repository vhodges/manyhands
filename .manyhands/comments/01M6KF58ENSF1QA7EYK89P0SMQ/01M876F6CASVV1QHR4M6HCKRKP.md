---
id: 01M876F6CASVV1QHR4M6HCKRKP
item_id: 01M6KF58ENSF1QA7EYK89P0SMQ
created_at: 2026-11-24T16:33:21.981681585Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
RFC approved by the owner, 2026-11-24, and promoted: docs/RFC/pull-request-mode.md (document 01M871JTB4FE0FCZ8KBKTY2A44), merged into main as 0db9124d. All 22 decisions are approved, including proposed decisions 15-22.

Implementation Cycles filed from it, one ticket per Cycle:

- Cycle 01, vh-c2dtj (01M87683441PRAYQJQBQBG96YY): pull-request setting, link builder and pull-only repo sync. No dependencies.
- Cycle 02, vh-c97se (01M84H84WQYE8J2WJWEDGJ45W4): onboarding and setting changes. This existing ticket gained a comment mapping it to the RFC, and now depends on Cycle 01.
- Cycle 03, vh-gy8gz (01M87683QK8E34EYY8KS4VQG9Y): ticket close in pull-request mode, the closing state and withdraw. Depends on Cycle 01.
- Cycle 04, vh-rphcs (01M87684C9GM9BJP5QQ429KMAK): document promote in pull-request mode. Depends on Cycle 01.
- Cycle 05, vh-ssepw (01M876851FT3NPSH9KDXKXR1AJ): after the merge (detection, merged_at, cleanup and surviving branches). Depends on Cycles 03 and 04.
- Cycle 06, vh-p7st2 (01M87685PKX0TF007VW1MB1RGJ): desktop for pull-request mode. Depends on Cycles 02 to 05.
- vh-fk279 stays independent. It touches the same desktop labels as Cycle 06.

The Wave 06 document is updated on its branch: the design items are ticked and the six Cycles are listed with checklists. Promoting it is the owner's call. Progress reporting for long steps is not covered by the RFC, and is deferred with the separate steps (decision 1).

Next: plan Cycle 01 (vh-c2dtj) with the planning-a-cycle skill. This ticket's outcome, the RFC, is delivered, so it can be closed whenever the owner chooses.