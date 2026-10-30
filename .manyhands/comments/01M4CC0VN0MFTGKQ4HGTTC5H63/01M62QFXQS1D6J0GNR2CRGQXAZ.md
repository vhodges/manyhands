---
manyhands_managed: true
manyhands_kind: comment
id: "01M62QFXQS1D6J0GNR2CRGQXAZ"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-29T02:23:14Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 21: Dependencies, Parent And The Cycle Check

Commit `54bddc4` (feat: edit a ticket's dependencies and parent in the
details sheet), with this comment and the ledger row in a docs commit.

While editing, the details sheet's Waiting on shows a chip per
dependency in ID order, with Remove and Add…, and Parent shows its chip
with Change… and Clear. Add… and Change… open a picker over the tab's own
repository's tickets. It filters by title, matches short codes exactly
through the library, and closes on Escape without closing the sheet.

Every change runs the relationship check under the tab's stamp, and a
late answer is dropped. A cycle is explained under its field, naming its
tickets, and every save waits through the one save gate. A save the
library refuses for a cycle is explained the same way. An unresolved
ticket says "not found, kept" and saves. An index that is behind adds
"Save checks again". Dependents and Children stay read only, and the
controls are read only while preparing, loading, in the review and kept.

Tests, through Devenv with the desktop test support: `relations_edit`
18 passed (13 view, 5 model); `sheet` 30 passed; the whole desktop binary
955 passed, 0 failed. Both clippy runs and fmt are clean. RED was 17 of 18
against stubs. Ten mutations were tried, and each guard was caught but
the stamp alone, which the dropped task masks. No earlier test changed.
