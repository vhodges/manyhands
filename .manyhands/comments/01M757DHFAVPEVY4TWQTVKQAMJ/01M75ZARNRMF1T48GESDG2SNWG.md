---
id: 01M75ZARNRMF1T48GESDG2SNWG
item_id: 01M757DHFAVPEVY4TWQTVKQAMJ
created_at: 2026-11-11T18:53:40.082795353Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Wave 04 Cycle 02, checkpoint: Tickets done (Tasks 1-4, commit 1568da1b).

Done and reviewed, one fresh reviewer per task:
- Task 1, row, list states and icons: KeptList, list_body, ListRow, ten icons (E-11).
- Task 2, Tickets model: one list_tickets read, derive, markers, tooltips (E-1, E-2, E-10 made-ready part).
- Task 3, Tickets panel: filter bar, columns, rows, list states over an always-drawn list, header count 'Updating...', Retry through reload_lists, Home and End (E-3, E-6, E-10, E-12).
- Task 4, filter input replaces Find: Enter opens an ID or a sole code, focus stays in the input; text and a held ID read clear once per repository change (E-4, E-5, E-6 find part).

Decisions and findings (all in the execution ledger, docs/plans/2026-11-11-wave-04-cycle-02-tickets-and-documents-execution.md):
- ReadKind::FindSlug stays: the relations picker still uses it.
- Stale and never-refreshed list states are made with the registry SQL seam, so the stop-and-ask trigger for a state no seam can make did not apply. The incomplete list alert is covered only by list_view's table test.
- Reviews found and fixed: Retry not pinned to reload_lists; a chmod test trick (unix and root only); a held ID read surviving a selection going to None; three untested guards. One Task 3 test flaked about 1 in 8 before a reorder (35 of 35 and 5 of 5 after); the cause is not identified.
- Test-scope pre-approval used: five additional tests in Tasks 2-4, listed in the ledger.

Tests run: by exact name per task, never a whole suite; static gates green at each commit. The fast tier runs once at review-ready (Task 9). Boundary: nothing under src/ outside src/desktop/, and no Cargo or packaging change. origin/main is still 48653d01.

Next: Task 5 (Documents model), after merging origin/main.