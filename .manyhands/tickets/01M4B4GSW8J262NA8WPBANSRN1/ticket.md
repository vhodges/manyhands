---
manyhands_managed: true
manyhands_kind: ticket
id: "01M4B4GSW8J262NA8WPBANSRN1"
title: "Discovery scanner must ignore nested worktree bases"
type: "defect"
status: "open"
project: "manyhands"
team: "core"
---

The discovery scanner/indexer has a gap around nested Manyhands worktree bases.
Wave 01 Cycle 04 requires scanning the primary repository and direct active
contexts under `.manyhands/worktrees/`, while avoiding recursive worktree-base
scans. We confirmed that direct worktrees can contain their own
`.manyhands/worktrees/` directory, and scanner/indexer traversal must ignore
those nested bases so stale or nested context data cannot be indexed as another
canonical source.

## Evidence

- Relevant cycle: `docs/Cycles/wave-01-cycle-04-discovery-and-rebuild.md`.
- Relevant design: `docs/plans/2026-10-02-wave-01-cycle-04-discovery-and-rebuild-design.md`.
- Existing design excludes Git metadata and the root `.manyhands/worktrees/`
  directory during source scanning, but the same exclusion must apply when the
  scanned context itself is a worktree.
- The interim ticket scripts exposed an analogous duplicate-ticket failure when
  stale ticket metadata existed in direct worktree checkouts.

## Expected Behavior

- The scanner/indexer scans the repository root and direct recognized active
  contexts in `.manyhands/worktrees/<ticket-id>/`.
- While scanning any context, it must not descend into that context's own
  `.manyhands/worktrees/` directory.
- Nested worktree content must not produce discovered items, comments, context
  problems, duplicate-ID diagnostics, or activity records.

## Acceptance Criteria

- Add discovery/indexer coverage with a direct active context containing a nested
  `.manyhands/worktrees/<nested-id>/` tree with managed-looking ticket/comment
  files.
- Refresh/rebuild results include the primary and direct active context data,
  but exclude all nested worktree-base content.
- No canonical Markdown or Git state is mutated by the fix.
