---
manyhands_managed: true
manyhands_kind: comment
id: "01M4CWCA3D1ES8J6DJM2TNMJXS"
item_id: "01M4CC0VMQ7R15A7M9SPN3KB67"
created_at: "2026-10-08T04:29:39Z"
---

Task 3 checkpoint: target resolution and repository reads, complete.

Commits `d63ec8f` (implementation), `81ead8f` (review fixes) and `9639595`
(message wording), range `a1223c0..9639595`.

Independent review of `d63ec8f` returned accept-with-fixes with one blocker:
a linked worktree could resolve to the wrong registered repository, because
the owner was taken as the parent of the common Git directory without
checking. The reviewer reproduced it with an unrelated repository whose Git
directory sat inside a registered root. `81ead8f` accepts a candidate owner
only when its common Git directory equals the worktree's; the implementer saw
the new test reproduce the wrong result before the fix.

Also fixed: a detached HEAD no longer makes `repo inspect` an internal error
(a read-only sibling of `inspect` reports a null branch; the public `inspect`
is unchanged); and a repository that exists but cannot be opened is
`repository_inaccessible`, not `not_repository`.

The review confirmed the other resolution cases, that no read writes or
reaches the network, and that remote locations are redacted on every path.

Evidence through Devenv on Linux, controller rerun at `9639595`:
`read_boundary` 13, `read_contract` 24, `read_repository` 20,
`repository_enablement` 73 and `local_authoring` 112 passed, 0 failed.
Implementer at `9639595`: `cargo test --locked --lib` 209 passed;
`cargo fmt --check` and clippy with warnings denied pass. Implementer at
`81ead8f`: `cargo test --locked --doc` 9 passed; `cargo check --all-features
--locked` passes. The full suite was not rerun. `81ead8f` and `9639595` were
not independently re-reviewed.

Not tested: permission-denied and ownership failures during resolution, which
could not be constructed portably. Rulings and one known gap are in the
execution ledger. Task 4 is next.
