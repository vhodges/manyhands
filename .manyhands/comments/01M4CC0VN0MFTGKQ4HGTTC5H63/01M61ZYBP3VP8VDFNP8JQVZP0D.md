---
manyhands_managed: true
manyhands_kind: comment
id: "01M61ZYBP3VP8VDFNP8JQVZP0D"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-28T19:31:41Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 19: Stale-edit review

Commit 5d4c7fb holds the code and tests; this commit holds the ledger row.

- **What changed.**
  - The review opens on an `external_change` answer, on Save over an
    unverified base, and on the change banner's Compare…. It reads the
    item again and takes the page's place, with the toolbar hidden.
  - Every field shows base, theirs and mine, with Take theirs and Keep
    mine. One-sided changes default to their side; a field both changed
    must be chosen before Use these choices is enabled.
  - The body's result is edited in its own editor. Switching the body's
    choice after an edit asks "Replace your edits?".
  - Use these choices moves the base onto the current version, puts the
    result into the edit editor exactly and writes the draft. Cancel
    leaves the draft file byte-identical.
  - Nothing is written to the draft while the review is open.
  - A dirty editor's change banner now offers Compare… too.
- **Outcome rows.** All nine rows of the review's table, and the
  session row "Done or Save during the Review: disabled", answer as the
  design says. There was no stop.
- **Decisions.**
  - Task 16's N2 and Task 18's M3: a clean session on an unverified base
    keeps another actor's reloaded body in its base and stays unverified.
    It reads clean, and the next Save opens the review. The read after a
    save takes the late reads' rule, once D2 has retired the draft.
  - Task 18's M4: the close dialog stays open and says "Cancel to review
    the changes."
- **Tests**, desktop-test-support, each run once at the end:
  - `review`: 54 passed (27 new);
  - `drafts::privacy_tests`: 1 passed;
  - `edit`: 241 passed;
  - the whole desktop binary: 897 passed, 0 failed.

  Both clippy runs and fmt are clean.
- **Named edit.** E-19 adds `SaveDocumentAs` and `SaveTicket` to the
  second actor. No other main or D3 test changed.
