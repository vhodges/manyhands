---
manyhands_managed: true
manyhands_kind: ticket
id: "01M4W5SCBV6WFJP11Y7PJF5VS8"
title: "The operation reads name no item for a local authoring operation"
type: "defect"
status: "open"
project: "manyhands"
team: "core"
---

The journal table `operation_records` has an `item_id` column, and nothing
in `src/` ever writes it. So the operation reads (`operation show` and the
operation list) report `item_id: null` for every local authoring row: a
ticket or document save, a comment submission, a repair. A user looking at a
pending operation is not told which item it belongs to.

The information exists. The row's target names the item, and the preview and
result of `operation abandon` already derive it from there
(`authoring_item`).

## Evidence

Found on 2026-10-13 while Wave 02 Cycle 07 was integrated into the Wave 03
Cycle F2 branch (ticket `01M4CC0VMR8HSPZXQ1WX41GWVK`): `operation show` of a
comment whose registration was pending had `item_id: null` while the abandon
preview named the item. The integration's re-review found, by reading, that
the gap is on main and covers every local authoring row
(`src/repository/read/status.rs`, around lines 160-165, where
`local_operations` builds the read).

## Suggested fix

Derive the item from the row's target in `local_operations`, as
`authoring_item` does for the abandon preview, with no schema change. The
alternative is to fill the column when the row is written, which also needs a
rule for rows written before the change.

## Done when

`operation show` and the operation list name the item for a pending and a
finished save, comment submission and repair, with a test for each, and for
rows written before the change.
