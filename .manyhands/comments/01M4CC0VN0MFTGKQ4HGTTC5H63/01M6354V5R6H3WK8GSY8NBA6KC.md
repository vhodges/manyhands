---
manyhands_managed: true
manyhands_kind: comment
id: "01M6354V5R6H3WK8GSY8NBA6KC"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-29T06:21:51Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 23: Create Dialogs

Commit `de5c4de` adds New document… and New ticket… to the lists, and
`secondary-n` in the navigation region. The dialog takes a Title, and a
ticket's Type or a document's Folder and File name. A document's path is
checked locally, then with `observe_path`. Create takes a new ID, then
writes the draft and sends its Create prepare through Task 22's entry
point. The new tab opens editing, with its header from the draft. At
Save, another file at the path keeps the draft and says so, and a
repository with no initials points to Settings. With no draft store the
dialog says why and stays open.

Tests, through Devenv, once each at the end: `create` 42 passed,
`view::documents` 4, `view::tickets` 9, `keys` 28, `keyboard_tests` 23.
The whole desktop binary: 985 passed, 0 failed, in 698 s, load average
19.48 at its end. Both clippy runs and fmt are clean. The brief's eight
tests and two more are new; no existing test changed.

Rulings: the pre-flight words for the path, the occupied path at Save and
initials, with Open Settings; `secondary-n` in the navigation region only.
Decisions, copy and parked items are in the execution ledger.
