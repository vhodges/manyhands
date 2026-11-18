---
manyhands_managed: true
manyhands_kind: comment
id: "01M64DPK2VR0MCWPZ1H5Q4CSYD"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-29T18:10:47Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---
The Part E checkpoint (Tasks 24 to 26: Move, New folder, Repair, and the keyboard and privacy sweeps) is complete.

- **The gate at `b3e6ecf`:**
  - the whole desktop binary passed, 1048 run and 0 failed;
  - both clippy runs, fmt, the CLI isolation check and `check --all-features` were clean;
  - Part E changed no library code.
- **Review:** the Part E review found 1 Important and 7 Minor issues. The Important one: while repair text had unsaved changes, the header pill said "Viewing the shared copy".
- **Fix round 1** (`5d5b437` to `d5756af`):
  - the pill says "Editing · unsaved changes" while repair text differs from what was read, and while the repair runs;
  - Move… waits until the stored drafts are listed, as a release does;
  - the repair's confirmation gives focus back to the text area;
  - main's two keyboard-test helpers are back to main's own text;
  - Cancel is disabled while a Move, New folder or Create write runs;
  - New folder's module doc says why it sends no observation.
  - The whole desktop binary passed, 1054 run and 0 failed.
- **Fix round 2** (`e6cea3d` to `b356838`):
  - Move… no longer waits for a listing when the draft store did not open (crash recovery off), since that listing never comes;
  - the privacy sweep now drives a repair refused at Preview, the refusal a parser could echo the source in. It found no leak.
  - The touched filters, both clippy runs and fmt passed.
- **Re-review:** the scoped re-review of both rounds found every finding addressed and nothing new broken.
- **Known Limit** (for the design at Task 27): after a repair, the file on main still needs fixing until the item's work is merged, and repairing it again is refused.
- One correction to my own Task 26 brief: it allowed main's test helpers to be refactored, which no named edit allows. Round 1 restored them.
- The rulings are in the execution ledger.
