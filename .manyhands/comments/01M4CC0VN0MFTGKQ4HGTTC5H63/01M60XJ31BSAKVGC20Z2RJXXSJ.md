---
manyhands_managed: true
manyhands_kind: comment
id: "01M60XJ31BSAKVGC20Z2RJXXSJ"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-28T09:30:47Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 15: Save, the session after a save, and states

Task 15 is done in 5412c58, with this commit for the ledger and this
comment.

- **What changed.** Save works.
  - Save in the header, and Ctrl/Cmd-S anywhere in the tab, save the
    edit editor's text as a checkpoint on this computer. Save is enabled
    only while there are changes and no save is running.
  - Ctrl/Cmd-S with no changes saves nothing and says "No changes to
    save" in the toolbar.
  - After a save, the editor is clean and undo still reaches the text
    before it. The next draft starts from what was saved. If someone
    else changed the item in between, the next Save opens the review
    first.
  - The pill reads "Editing · saved on this computer", with "not shared
    yet" when a publication remote is selected. Remotes are read after a
    save, never when the tab opens.
  - Banners cover a pending checkpoint (Retry, the same request), a
    search index not yet updated (Resume), a change by someone else, a
    base to check first (Review…), and other refusals.
  - With crash recovery off, Save still saves.
- **Tests.** The five session-outcome rows were written first; each
  answers as the design's table says. Runs at the end: `edit` 146,
  `drafts` 129, `view::tabs` 30 and the whole desktop binary 793, all
  passing. Both clippy runs and fmt are clean.
- **Rulings.** None were needed. No existing test changed.
- **Left for later tasks.** The review's own content and its way out
  are Task 19's; Done is Task 16's.
