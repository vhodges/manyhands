---
manyhands_managed: true
manyhands_kind: comment
id: "01M614Y06VCJRSD902V2BDHC2A"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-28T11:39:38Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 16: Done and the kept session

Task 16 makes Done real, keeps the session after Done, handles a session
ended elsewhere, and asks before a tab with unsaved work closes. The code
and tests are f10cc50; the ledger row and this comment are the next
commit.

- **Done** follows the design's order. It is disabled during the review,
  with "Use these choices or Cancel first". It waits for a save in
  flight and goes on only once the save commits. It asks on an item never
  created. It writes the draft now. Then a clean session ends, a dirty one
  whose draft is written is kept, and one whose draft cannot be written
  asks Save, Discard or Cancel.
- **The kept session** keeps the edit editor and its undo while the tab
  views the saved copy. Continue editing, Edit and `e` show the same
  editor again. A refresh that finds the item changed shows the change
  banner, and nothing opens by itself.
- **A discard from the Drafts page** ends a kept or live session and says
  "The draft was discarded."; a never-created item's tab closes. The
  page's Discard is disabled while that draft's save is in flight.
- **Closing a tab** with unsaved work, a kept or stored draft, or a draft
  still being stored asks first. A metadata change alone counts. The
  dialog no longer says "restored draft" for an edit session.
- **Release on leave** stays Task 17's, behind one hook.

All eight session-outcome rows answer as the design's table says, so
there was no stop. RED and GREEN were recorded, and 9 mutations were each
caught. Tests run through Devenv, once each at the end: `edit` 170,
`view::item` 172, `view::tabs` 30, `drafts` 133, and the whole desktop
binary 817, all passing. Both clippy runs and fmt are clean. No main or
earlier D3 test changed, and no library file changed.
