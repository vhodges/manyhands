---
id: 01M78JR8B5RY3XZ0TXTP6ZMTE9
item_id: 01M784Y94Q3AXT6XJX38F5VK1V
created_at: 2026-11-12T19:11:34.007641769Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
What Cycle 03a leaves to this Cycle (note from 03a's planning, 2026-11-12).

Cycle 03a (`01M784Y8C73B39GR4JWENXE5D0`, `vh-3jpbf`) was approved on
2026-11-12; its documents are on its branch:
`docs/Cycles/wave-04-cycle-03a-repositories.md`, the design
`docs/plans/2026-11-12-wave-04-cycle-03a-repositories-design.md` and the
plan. The Wave's Cycle 03b section (amended in 03a's approval commit
`1b68793d`) is the authority; this note is a pointer, not a decision.

**Obligations 03a hands to 03b:**

- **Repository state.** Read each repository's state from 03a's pure
  `repository_state` (design Part B), not a second derivation, wherever
  Activity or the popover speaks about a repository.
- **The Needs attention count** is yours to decide. 03a shows these
  states on rows but the footer's count does not include them yet (03a
  K-9): Can't open, Needs fixing, Can't reach, Locked and Needs attention
  from a stored outcome, a check's listed problems (a dirty working copy,
  a conflict, a branch deleted on the shared copy and the rest), a
  pending conflict, `Waiting { why: Failed }`, and an unread registration.
- **The footer.** Its right group now reads Update check, To share, Needs
  attention (`plan(layout, &FooterStatus)`, `Control::ALL` of 11).
  Bounds are recorded only for the panel toggles today, so the popover's
  anchor above Needs attention needs its own.
- **The three operation-area rows** of Cycle 01's K-19 (draft-flush
  Retry, pending repository Retry, pending controls such as Abandon) and,
  from the D1 polish ticket `01M5CS4MH25QQ769HV61ZRKHXG`, "Finishing"
  hiding the running stage.
- **Activity and "to share" agree.** 03a removes an item this app shared
  from the check's Activity entry as well as from the count; the timed
  log should keep that.

**Built by 03a that 03b may reuse:** the Workspace-owned popup menu layer
(design Part D, like Cycle 01's dock menu), `Services::check_now` (one
guarded Check now), D6's model extended with the stored outcome
(`status::stored_applies`) and the unshared list.

Plan this Cycle after 03a merges, from up-to-date main.
