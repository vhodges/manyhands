---
manyhands_managed: true
manyhands_kind: comment
id: "01M6H5FCYPD3HJ86EK5FRKJ50X"
item_id: "01M4CC0VN1KTQFSVR99378TDHV"
created_at: "2026-11-03T16:56:59Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Review-ready

D4 is review-ready on its local Devenv gates.

**Gates**
- Static: fmt, check (all features and desktop), and both clippy lines
  are clean.
- AGENTS.md fast tier: 4947 passed, 0 failed, 1 ignored.
- `check-cli-no-gpui.sh` passes.
- Desktop bin suite at the final code commit `6beac6b0`: 1318 passed,
  0 failed.
- The merge-tree with `origin/main` (`0dfd5a68`) is clean.
- No library file changed. Existing tests changed only per E-1 to E-15.

**Reviews**
- Every task had an independent review.
- The whole-branch SPEC and QUALITY reviews are approved after fixes.
- Task 13 (posting recovery) went through a design checkpoint and five
  code-review rounds.

**Acceptance**
- A1 to A17 are met by tests, except A2, which is native only.
- The native half of each row is pending the owner's run of
  `docs/plans/2026-11-02-wave-03-desktop-04-on-screen-checklist.md` on a
  release build.

**Recorded test limits**
- Decision 23 is tested in the model only.
- The A15 scan reads labels and values only.
- Change 5's failed-retirement branch is untested.

**Owner decisions during execution:** 17 to 27.

**Rulings:** R-17 to R-19.

**Known limits:** K-11 to K-19.

**Library tickets:** `01M6GHWZ5CXXFC7YB3EVJ6Q4A7` (Resume for comment
publications) and `01M6GHWZS03WCADS1A1JXV4PMP` (held reservation after
restart).

**Proposed follow-ups** (quality review, not filed yet):
- classify share outcomes once, at drain time;
- share the read-only guard with `view/editor/read_only.rs`;
- a library ticket to make `CommentMutationDto::from_stored` public;
- drive comment adoption from observers rather than render;
- prune per-session maps;
- split `worker.rs` and the operations files;
- consolidate test helpers.

**Still needs the owner**
- Code review and approval.
- The on-screen checklist run.
- Closing this ticket (after approval).
- Merging to main.
- Dispatching native CI after the merge.
