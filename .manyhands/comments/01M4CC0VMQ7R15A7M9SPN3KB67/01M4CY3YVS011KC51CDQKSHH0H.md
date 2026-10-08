---
manyhands_managed: true
manyhands_kind: comment
id: "01M4CY3YVS011KC51CDQKSHH0H"
item_id: "01M4CC0VMQ7R15A7M9SPN3KB67"
created_at: "2026-10-08T05:00:02Z"
---

Task 4 checkpoint: credential reads, complete.

Commits `ca7f292` (implementation), `003c46f` (review fixes) and `a04acb3`
(comment bound), range `9fbe74d..a04acb3`.

Independent review of `ca7f292` returned accept-with-fixes, no blocker. It
confirmed that a private key path reaches no filesystem call, that the helper
extractions in `keys/` preserve existing behavior, and that locking and the
goldens are right. Fixed in `003c46f`:

- `key public` refused a private key only after opening and reading it when a
  registration's public path was its private path. It now refuses before
  opening, and the buffer is zeroized.
- The public key is returned as the canonical encoding of the parsed key, with
  the comment limited to printable ASCII; the file's own bytes are not echoed.
- Stored host pin rows are validated with the trust code's own checks.
- The reapproval state is reported at list level, so it shows with zero pins.
- The "never opens a private key" test now watches the files with inotify on
  Linux; the implementer proved it fails when a read opens one.

The controller's first instruction capped the whole public-key line at 1,024
bytes, which would have refused large RSA keys; the implementer flagged it and
`a04acb3` caps the comment at 256 bytes instead.

Evidence through Devenv on Linux, controller rerun at `a04acb3`:
`read_credentials` 25, `read_contract` 30, `read_boundary` 13,
`read_repository` 20, `shared_key_registry` 39, `key_material` 39,
`key_storage` 2 and `session_credentials` 11 passed, 0 failed. Implementer:
`cargo test --locked --lib` 209 passed, `cargo fmt --check` and clippy pass at
`a04acb3`; `ssh_transport` 103 SSH cases, `cargo test --locked --doc` 9 passed
and `cargo check --all-features --locked` pass at `003c46f`. The full suite was
not rerun. `003c46f` and `a04acb3` were not independently re-reviewed.

Tests for the first commit were written after the code and mutation-checked;
the review-fix tests were seen failing first. Rulings and accepted limits are
in the execution ledger. Task 5 is next.
