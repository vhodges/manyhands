---
id: 01M5D4V34D1TJC8DJF1GF9X6BR
item_id: 01M5CVB2DTK8EQKG3FMYMWNTMX
created_at: 2026-10-20T17:13:14Z
created_by: Claude Opus 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---

Review-ready on local gates. The work is in commits `e919b1e`,
`4a4745a` and `249d380`, on top of `origin/main` `3bda743`. The merge-tree
check is clean. Nothing beyond the ticket checkpoint is pushed.

**What changed**

- **Binding** (`bind/repo.rs`):
  - The enable observation now records whether the root is registered in this
    data directory, and whether its configured publication remote is
    eligible. Both are part of the digest.
  - `check` keeps the domain's order: primary branch, prefix, publication
    remote, registration.
  - An enabled repository that is not registered previews
    `worktree_exclusion_written` and `repository_registered`, and writes
    nothing to Git. Execute registers and indexes it and answers `ok`, with
    no commit. Replay answers from the record.
  - A root that is already registered still answers `already_applied`.
- **Domain** (`src/repository.rs`): the domain's already-enabled path now
  indexes a root it newly registers, using the same hand-off a pending row
  uses. It can therefore return `IndexPending`.
- **Outcome**: a new `outcome::registration_result` reports a
  registration-only success.

**Behaviour changes**

- A registered root whose configuration names an unusable publication remote
  now answers `invalid_remote` instead of `already_applied`, as the domain
  does.
- Every enable observation digest changed. A confirmation issued before the
  upgrade answers `external_change` once.

**Review**

- An independent review found one Important issue: the preview did not run the
  publication-remote eligibility check.
- Fix round 1 added that check, and its re-review found the order still
  differed from the domain's for a root that was already registered.
- Fix round 2 corrected the order, and its re-review approved.
- Documented edges:
  - A rare case where a registration made by a second actor is attributed to
    this request's own pending row.
  - Create now also requires registration for its end state.

**Tests**

- New library tests cover:
  - preview, execute, replay, a lost answer, and a second enable;
  - a second actor before confirm;
  - an ineligible remote, both unregistered and registered.
- Each failed first and passes now.
- Full gate: every command exited 0.
  - `check`, `fmt` and `clippy` with `--all-features`.
  - `check` and `clippy` with `--features desktop`.
  - `cargo test --all-features --locked --no-fail-fast`: 3260 passed, 0
    failed.
  - The CLI smoke test and the CLI no-GPUI check.

**Follow-up proposed, not filed**

If no Git identity is configured anywhere, a registration-only preview still
answers `identity_required`, because the preview's identity field is
required. Fixing this needs a contract change (a nullable preview identity)
and a matching desktop change.

Push, PR, merge and closure each need separate authorization.
