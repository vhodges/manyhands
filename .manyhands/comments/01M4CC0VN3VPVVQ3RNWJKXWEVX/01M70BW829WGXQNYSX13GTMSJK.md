---
id: 01M70BW829WGXQNYSX13GTMSJK
item_id: 01M4CC0VN3VPVVQ3RNWJKXWEVX
created_at: 2026-11-09T14:37:28.882511302Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
## Planning checkpoint: three review rounds done, ready for approval

- **Documents:** revision 6, commit `04e1623f`, pushed. Main `f4442618` was merged in as `9512080d`, by merge, not rebase. No build or test was run; only documents changed.
- **Owner decisions:** 16–27 are recorded. Decisions 3–15 and 28 are proposals to confirm at approval.
- **Review rounds** (two independent reviewers each):
  - Round 1 at `d48bc4aa` found 8 Blocking, 33 Important and 32 Minor.
  - Round 2 at `4d22c393` found 5 Blocking, 18 Important and 30 Minor.
  - Round 3 at `c35719c2` found 3 Blocking, 15 Important and 24 Minor.
- **Round 3 result:** three mechanisms drew blockers for the third time: preemption, stale rows and the retirement loop's rules. As the planning procedure asks, each is now stated as required outcomes (design P-1 to P-7, L-1 to L-6, RT-1 to RT-11), proved by a test-first task (plan Tasks 4 and 11b) after a design checkpoint. No round 4 runs. **Revision 6's edits had no independent review.**
- **Settled in Task 0:** U-2, U-9 (rest), U-11 (rest), U-15 and U-16 by reading source. U-14 is settled in Task 1 by an experiment.
- **Follow-up library tickets** (decisions 25 and 27) are not filed yet: a per-key passphrase cache, and a poll takeover that does not need eligibility.
- **Proposed execution method:** subagent-driven, one task at a time, as in D5. Tasks depend on each other's interfaces.
