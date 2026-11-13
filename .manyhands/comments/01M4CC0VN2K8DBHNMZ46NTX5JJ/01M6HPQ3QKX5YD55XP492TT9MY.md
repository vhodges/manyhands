---
manyhands_managed: true
manyhands_kind: comment
id: "01M6HPQ3QKX5YD55XP492TT9MY"
item_id: "01M4CC0VN2K8DBHNMZ46NTX5JJ"
created_at: "2026-11-03T22:48:11Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Planning checkpoint

**Base.** This ticket branch merged `origin/main` at `c2a34494` as
`e39bb004`, with no conflicts and no rebase. The drafts are written against
C5's unmerged tip `f59aceb6` (decision 1); approval and Task 0 wait for
C5's merge.

**Drafts (revision 4, `bdc12fae`, pushed).**
- `docs/Cycles/wave-03-desktop-05-recovery-and-completion.md`
- `docs/plans/2026-11-03-wave-03-desktop-05-recovery-and-completion-design.md`
- `docs/plans/2026-11-03-wave-03-desktop-05-recovery-and-completion-implementation.md`

No build or test was run during planning.

**Owner decisions (2026-11-03).**
1. Draft now, re-ground after C5 merges.
2. D5 carries a library part: way out, lifecycle resume listing, progress
   stages.
3. The way-out routing is a shared library function.
4. The resolve sheet works per part.
5. Republish is in scope.
6. Made-ready appears only on Done.
7. Native evidence is Linux only, with the owner as tester.
8. Execution is serial.
9. Remote evidence comes from canned answers, library tests and the native
   checklist.
10. Resume, then resend.
11. Four new stages, including `publishing`.
12. Read-only tabs.
18. The lock follows an unfinished lifecycle.
19. Kept resolution sets.
20. The lock starts from `closing` or `integrating`.
21. A stuck tidy-up unlocks with a way out.

Decisions 13–17 are proposed and await confirmation.

**Reviews.**
- **Round 1 at `8e9dc65e`:**
  - source and feasibility: 3 Blocking, 10 Important, 7 Minor;
  - contract and consistency: 1 Blocking, 15 Important, 8 Minor.
- **Round 2 at `0d7a851c`** (scoped, adversarial): 1 Blocking, 11 Important,
  8 Minor.
- **Round 3 at `97f966f6`** (scoped, adversarial): 1 Blocking, 6 Important,
  5 Minor.
  - Revision 4 applies it and has not been independently re-reviewed.
  - The lock, the third round in one mechanism, is stated as required
    outcomes L-1–L-6. Task 9 is test-first with a design checkpoint.

**Open.**
- Decisions 13–17.
- U-1 to U-4, settled in Task 0 by reading.
- C5's merge.

**Proposed execution.** Subagent-driven and serial: Task 0 after C5
merges, then Part 0 (library), then Parts A–D.

**Record note.** This comment is a canonical file, not `comment add`, as on
D4. Publishing would need a key, and a stopped post could hold the
repository's reservation.
