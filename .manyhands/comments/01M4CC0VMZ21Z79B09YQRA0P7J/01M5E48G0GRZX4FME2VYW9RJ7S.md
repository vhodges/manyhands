---
manyhands_managed: true
manyhands_kind: comment
id: "01M5E48G0GRZX4FME2VYW9RJ7S"
item_id: "01M4CC0VMZ21Z79B09YQRA0P7J"
created_at: "2026-10-21T03:06:48Z"
---

Planning checkpoint. The Cycle document, design and implementation plan are
written and proposed for approval:

- `docs/Cycles/wave-03-desktop-02-recoverable-drafts.md`
- `docs/plans/2026-10-20-wave-03-desktop-02-recoverable-drafts-design.md`
- `docs/plans/2026-10-20-wave-03-desktop-02-recoverable-drafts-implementation.md`

**Base.** `git fetch origin` observed `origin/main` at `3bda743`. The ticket
branch had never been pushed and held one checkpoint commit, `84c7fc5`.
Merge `e8fb54b` brought `origin/main` in with no conflicts; ancestry was
checked. No rebase. The main checkout's local `main` is seven commits
ahead of `origin/main`. Those commits are the unpublished merge of
`01M5CVB2DTK8EQKG3FMYMWNTMX`; D2 merges it once main is pushed. No Rust
command was run for planning.

**Dependencies.** Wave 02 Cycles 01–10, F1, F2 and D1 are closed and on
main. D2's desktop work, Parts B and C, waits for the Structure part of
`01M5CS4MJ0G7B0MPJD0KN0QMYA` (decision 4).

**Product-owner decisions, 2026-10-20** (Cycle document, decisions 1–5):

1. A recovery surface: store, sessions, models, save completion and a
   Drafts page; no editor or Save button.
2. The store is a new library module, `src/drafts/`, plus one line in
   `src/lib.rs`.
3. On Windows, the drafts folder inherits the profile ACL; reparse points
   and other owners are refused.
4. The `01M5CS4MJ0` Structure part comes first.
5. On Unix, a group-writable data directory is accepted, and every
   operation is relative to a held handle on the drafts folder.

**Review.** An independent reviewer read the three documents against
`AGENTS.md`, the Wave, the RFCs, the library and desktop source and the GPUI
sources. It reported one blocking finding and eighteen others. The blocking
finding: a revision saved before it was flushed could never retire; the fix
is to flush before submitting (ruling R21). Major findings:

- encoding order under `preserve_order` (R18);
- `Unverified` versus `Changed`;
- this process's own claims listed as open elsewhere (R19);
- a lock-file unlink race (R20);
- the umask-002 data directory (decision 5);
- a `pending` checkpoint with no tested path (R22).

Each finding was checked against source and applied. The design's Review
Record lists them.

**Open questions.** Q1: may Part A (library only) start before the
Structure part merges? Q2: is the development seeding example acceptable
for the native checklist?

**Proposed execution.** Subagent-driven, task by task, with a review at
each Part checkpoint, as D1.
