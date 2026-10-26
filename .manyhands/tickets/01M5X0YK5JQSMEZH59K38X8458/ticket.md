---
slug: vh-j39te
deps:
- 01M4CC0VN0MFTGKQ4HGTTC5H63
id: 01M5X0YK5JQSMEZH59K38X8458
title: Edit other front-matter fields in the details sheet
type: task
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Let people edit a document's status, date and author, and a ticket's other
front-matter fields, in the desktop's details sheet.

Follow-up from Wave 03 D3, product-owner decision 14 (2026-10-26). D3 shows
these keys read-only: no save input or draft carries them today.

## Scope

- **Library.** `DocumentDraft` and `TicketDraft` carry changes to other
  front-matter keys, preserving every other key's value and the canonical
  key order. Validation rejects keys Manyhands defines.
- **Drafts.** `DraftFields` holds them, in a new draft-file version, with a
  migration that keeps the original file when it fails (desktop RFC).
- **Desktop.** The details sheet's Status, Date and Author rows, and Other
  fields with "+ Add field", become editable while editing. Date uses the
  Kit's date picker.
- **CLI.** `document save` and `ticket save` accept the same changes.

## Out of scope

Identity, short codes, closure fields and relationship keys stay
lifecycle-owned. No change to how unknown values are preserved on load.

## Depends on

D3 (`01M4CC0VN0MFTGKQ4HGTTC5H63`), whose details sheet this extends.
