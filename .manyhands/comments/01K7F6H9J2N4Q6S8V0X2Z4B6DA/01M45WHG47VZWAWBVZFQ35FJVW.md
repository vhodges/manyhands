---
manyhands_managed: true
manyhands_kind: comment
id: "01M45WHG47VZWAWBVZFQ35FJVW"
item_id: "01K7F6H9J2N4Q6S8V0X2Z4B6DA"
created_at: "2026-10-05T11:17:38Z"
---

Planning checkpoint: reused the pre-existing ticket worktree and branch and
rebased it onto main (`21eefa4`), yielding `d546264`, before drafting. The Cycle
scope, detailed design, and implementation plan are under docs and await user
approval. The planning checkboxes record artifact creation, not approval.

The user confirmed two decisions during planning:

- Preserve the approved imported-key boundary: Cycle 02 validates readability
  and supplies session credentials; Cycle 03 validates imported key format,
  unlock, and actual SSH-backend compatibility.
- Add native Cargo runtime tests to the existing Linux/macOS/Windows CI and
  document its exception to the Devenv rule; local commands continue via Devenv.

Design refinements address caller-forged generated ownership, creation-time
file protection, stale confirmations, partial file/SQLite completion, bounded
generated-key parsing, and session invalidation after key/source changes.
The proposal requires durable creation evidence before destructive deletion
and preserves interrupted files when ownership cannot be proven.

Implementation checkpoints: (0) fresh baseline; (1) metadata and recovery
schema; (2) Unix/Windows protected storage; (3) generation; (4) session provider;
(5) source inspection/generated unlock; (6) confirmed deletion; (7) privacy,
native-platform evidence, and review. Add a comment at each checkpoint.

Source and lockfile match main at the planning baseline. Cycle 01 has prior
verification/review evidence; Rust checks were not rerun for this documentation
draft. Fresh baseline checks are the first implementation task. No product
code, key files, transport, or CI configuration has been changed by planning.
This ticket remains open; code-review approval is still required for closure.

Planning validation passed: git diff whitespace checks, front-matter/ULID
checks, uniqueness across 37 managed IDs, and existence of local Markdown link
targets. Self-review reconciled task dependencies and restart acknowledgement
of completed deletion without recreating confirmation authority.
