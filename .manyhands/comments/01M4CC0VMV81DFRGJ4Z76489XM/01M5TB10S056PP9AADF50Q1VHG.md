---
id: 01M5TB10S056PP9AADF50Q1VHG
item_id: 01M4CC0VMV81DFRGJ4Z76489XM
created_at: 2026-10-25T20:11:27Z
created_by: OpenCode <noreply@openai.com>
manyhands_managed: true
manyhands_kind: comment
---

## Authorized dogfooding stopped at C4 verification-tool error

The owner explicitly authorized the refresh, six saves and six target pushes
from the presented preflight. Work started at C3 `5cba600b`, using the existing
absolute branch-built CLI, root `/home/vhodges/work/src/manyhands` and unchanged
default data environment. Request inputs and pre-save compared values were
persisted in the ledger before sending; body is only SHA-256/UTF-8 byte length,
and source is never recorded.

- Refresh request **`01M5TAW1PY4TAXJ3W3MGK9GYV8`**: exit 0, success/ok,
  discovery current; no checkpoint or publication.
- C4 save request **`01M5TAYKVT3P51QDSY8SM93PGV`**: exit 0, success/ok,
  write written, checkpoint committed, discovery current, publication not_requested.
- C4 checkpoint/local tip: **`9b26717a282c49dfd91a9483c7b5e8c5bd59eeb7`**.
  Exactly one new commit; target clean, primary unchanged. No origin ref/upstream.

**STOP:** the temporary value checker compared dependency DTO objects directly
with proposed ID strings and reported `deps` mismatch. This is an implementer
verification-tool error. Returned dependency IDs are exactly C3
`01M4CC0VMV81DFRGJ4Z76489XM` and gate `01M57SKVF6TQ3QM6AXJ35JGGSY`, both with
state open. All other compared values and body digest/length matched (1704 bytes,
SHA-256 `e79f0a7156242d6e4e6d0ea249e5f9442c3ef4da07f906b862ee595a2ee7144b`).
The failed overall check is retained, not relabeled as a passing helper result.

The stop rule was honored: no checker fix/retry or resave, no C5 preparation,
no later saves and no target pushes. C4's successful request is already settled;
it should not be resent to repair checking. The helper and ledger evidence are
preserved. Controller/owner authorization is needed for the checker correction
and the design's resumed preflight/go-ahead/remaining sequence.

Fetch/merge-tree passed, main remains e49f978. C3 HEAD remains
`5cba600b562062d6809d9ce916e26b65ea1e204e`; ledger/comment left uncommitted at the
stop, with no further push. Comment ID from absolute CLI id new; UTC clock above.
Prior verification exception and whole-branch approvals remain accepted; no
new tests, switch, review-ready/Task 9 completion, closure, PR or native CI.

Evidence: `docs/plans/2026-10-24-wave-03-cli-03-authoring-and-discovery-execution.md`.
