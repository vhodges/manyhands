---
manyhands_managed: true
manyhands_kind: comment
id: "01M46FEDC3QFR2VTEC0QBVS8VH"
item_id: "01K7F6H9J2N4Q6S8V0X2Z4B6DB"
created_at: "2026-10-05T16:48:10Z"
---

The user answered all three planning questions:

- Q1: require username in the remote URL; no guessed/caller-supplied username.
- Q2: allow one passphrase prompt explaining that the key may be locked or may
  have been rejected; no repeated prompt within the attempt.
- Q3: after corrupt-database replacement loses pins, require fresh host approval
  even when known_hosts trusts the host.

Updated Cycle, design, implementation plan, and ticket. Proposed Q3 mechanism:
a durable application-data marker published before replacing the database,
disabling inherited trust for unpinned hosts thereafter; newly approved pins
work normally. The marker survives repeated recovery and is not automatically
cleared. Added recovery race/interruption tests and an explicit unlock-reason
contract so future front ends can explain the ambiguous prompt.

The production timeout engineering gate remains open: test watchdogs do not
bound production calls; backend timeout settings are process-global and require
safe early initialization. The user requested an explanation of that gap.
These behavioral decisions are approved, but the complete design/plan and
implementation are not yet approved. No Rust code changed.
