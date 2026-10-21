---
manyhands_managed: true
manyhands_kind: ticket
id: "01M5CS4MJ0G7B0MPJD0KN0QMYA"
title: "Desktop follow-up: structure from the D1 reviews"
type: "task"
status: "closed"
closed_at: "2026-10-21T14:52:58Z"
closed_by: "Vince Hodges <vince@imbas.ca>"
project: "manyhands"
team: "core"
---

The D1 desktop reviews (Wave 03 Cycle D1, ticket
`01M4CC0VMY5RBMP70QVBXWXM6A`) requested structural cleanup before later
desktop Cycles add more pages. On 2026-10-21 the owner approved splitting the
non-Structure findings into ticket `01M5F3GVDYTH1Q5V5FCS9YF7V3`, "Desktop
follow-up: settings UX from the D1 reviews". That ticket retains all the
settings, keyboard/accessibility, test-gap and product-decision items.

This ticket is now a desktop-only, behavior-preserving refactor under
`src/desktop/`, with no shared-library change. It closes after review approval
and lands as its own merge before the UX follow-up. Its completion on main
unblocks the desktop work (Parts B and C) of D2, ticket
`01M4CC0VMZ21Z79B09YQRA0P7J` (D2 decision 4). D2 remaps its File Map to the
new layout when beginning Task 5. Do not implement D2 features here.

The sibling tests-and-polish ticket `01M5CS4MH25QQ769HV61ZRKHXG` also touches
item/list/tab views; coordinate if it starts in parallel. The line counts
below describe main at `0b23664`, not the earlier D1 review snapshot.

## Structure

- `Read<T>` is defined twice (`view/settings/mod.rs` and
  `view/settings/repositories/manage.rs`). Keep one shared definition outside
  `view/settings/`, usable by a non-Settings page such as D2's Drafts page.
- Tickets are named by short code and title in slightly different ways in
  `TicketRow`, Find matches, relationship rows, tab titles and blocked
  reasons. `TicketRow::target` duplicates `OpenTarget::for_item`. Use one
  ticket-naming helper and one item-to-`OpenTarget` function at every site.
  Keep them in GPUI-free `model/` code so D2's draft model can use them.
- `Workspace` exposes 22 `pub(super)` fields that `lists`, `tabs`,
  `operations`, `sidebar` and `find` reach into. Narrow them to methods or
  events.
- Split the large files:
  - `view/workspace.rs` (1159 lines);
  - `view/settings/keys.rs` (1091 lines);
  - `view/item.rs` (921 lines).
- Keep switching between tabs and full-content pages in one small place.
  Adding D2's Drafts page should need a module and a page variant, rather
  than additional top-level `Workspace` fields. Do not add the Drafts page
  or speculative D2 abstractions here.

## Acceptance

- No user-visible behavior changes. Existing tests pass with only path/import
  changes needed for the refactor; no test is deleted or weakened and the
  number of `desktop::` tests stays the same.
  On 2026-10-21 the owner additionally approved five mechanical field-to-accessor
  changes in test helpers, enumerated in the plan's execution ledger. Assertions
  and scenarios remain unchanged.
- One shared `Read<T>`, one ticket-naming helper and one item-to-`OpenTarget`
  function serve the sites above.
- `lists`, `tabs`, `operations`, `sidebar` and `find` use `Workspace` methods
  or events rather than reaching into its fields.
- The three large files are split by responsibility.
- The owner approves the target module layout and narrowed `Workspace` API
  before implementation.
- Relevant `desktop::` tests run between steps; the final gates include the
  required all-feature check/fmt/clippy/tests and desktop-only check/clippy.
- Record review-ready evidence and a conflict-free merge-tree check. Close
  only after review approval; merge/publication require owner authorization.

## Approved implementation plan

The owner approved the plan on 2026-10-21:
[`docs/plans/2026-10-21-desktop-structure.md`](../../../docs/plans/2026-10-21-desktop-structure.md).
It records the target module layout, helper semantics, private Workspace
boundary, ordered tasks, unchanged-behavior/test policy and D2 handoff map.
The delivery split, design and plan are approved. The owner explicitly
authorized inline implementation on 2026-10-21; progress is recorded in the
plan's execution ledger and ticket comments.

## Resolution and approved closure

Implemented in `6c8bd121a4a00e7f5ebe2333fdd3afae46f8b517`. Shared Read,
TicketName and ItemLocator helpers preserve existing presentation and targeting;
Workspace state is private behind methods; tabs/Settings routing is centralized;
Workspace, Keys and ItemPane are split by responsibility. The 362 desktop test
names and assertions remain intact, with only the five approved accessor edits.

Final build/lint gates, completed all-feature coverage across the authorized
split runs, smoke evidence and independent review with no actionable findings
are recorded in comment `01M5FEZ8SR37B1AG8A0PME8PXJ`. The desktop smoke proved
bounded launch; a native interactive walkthrough was not claimed.

On 2026-10-21 the owner approved completion and explicitly requested closure,
commit, local merge to main, push to origin, deletion of this ticket's local and
remote branches, and removal of its worktree. Approval and delivery evidence is
in comment `01M5FF701TRWQCYNTYAE0Y1T2T`.

The UX follow-up `01M5F3GVDYTH1Q5V5FCS9YF7V3` remains open and is committed
and published separately as `64c231a`. Once this change is on main, D2 may
consume the Structure prerequisite and remap its File Map using this plan.
