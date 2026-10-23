---
id: 01M5MHQ16VD0P2RTGY8PMRYWGY
title: Parsing reorders an item's unknown front-matter keys
type: defect
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---

`canonical::parse_item` takes each known key out of the front-matter mapping
with `remove_known` (`src/canonical.rs:1241`). That calls serde_yaml 0.9's
`Mapping::remove`, which is `swap_remove`: the last key moves into the removed
key's slot. Whenever a known key comes before unknown keys in the file, parsing
reorders the unknown keys, and `serialize_item` writes them in the new order.
The first product save of a hand-written ticket, document or comment therefore
scrambles its extra metadata.

## Evidence

Source inspected at main `25b9fcf`.

- Re-saving the closed tickets on main (`6d2936d`, 2026-10-23) wrote
  `cycle` before `wave` in the Wave 02 tickets; the files had `wave`, `cycle`.
- A document header `title, date, status, author, manyhands_managed,
  manyhands_kind, id` is written back as `author, date, status, id, title,
  manyhands_managed, manyhands_kind`. For that reason the documents on main
  were deliberately not re-saved through the serializer in `25b9fcf`.
- The doc comment on `canonicalize_relationships` (`src/canonical.rs:983`)
  says "The keys that stay keep their places", which holds only within that
  function.
- The same `Mapping::remove` reorders the unknown metadata that reads return:
  `take_author` (`src/repository/read/comments.rs:98`) and the unknown
  metadata of a ticket (`src/repository/discovery.rs:162`).
- The written order is stable after the first save: parsing and serializing
  the output again gives identical bytes. The defect costs order, not data.

## Expected Behavior

- Unknown keys keep their relative order from the file through parse,
  serialize and the read views (for example, `shift_remove` in place of
  `remove`).
- Known keys keep their current canonical placement.

## Acceptance Criteria

- Canonical tests: a document, a ticket and a comment whose unknown keys
  follow their known keys serialize with the unknown keys in file order, and
  the output is still a fixed point.
- Read tests: a comment's and a ticket's unknown metadata are reported in file
  order.
- Ticket closure's preservation check (`closed_ticket_bytes`) and the existing
  golden files still pass; any golden that recorded the swapped order is
  updated, with the change explained.
- The fix itself rewrites no canonical Markdown.

## Follow-up After Merge

- Re-save `docs/` on main through `serialize_item`, the step deferred from
  `25b9fcf`.
- Optionally restore `wave`, `cycle` order in the Wave 02 tickets on main.
