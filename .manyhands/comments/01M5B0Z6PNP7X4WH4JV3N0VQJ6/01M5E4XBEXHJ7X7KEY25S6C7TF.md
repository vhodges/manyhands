---
id: 01M5E4XBEXHJ7X7KEY25S6C7TF
item_id: 01M5B0Z6PNP7X4WH4JV3N0VQJ6
created_at: 2026-10-21T02:33:42Z
created_by: Claude Opus 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---

## Approved design implemented; targeted verification

The owner approved the short chat design and TDD implementation, including
save-compatible best-effort reporting if index publication fails after commit.
Execution plan: `docs/plans/2026-10-21-enablement-index.md`.

The first seven regression tests failed on the missing configuration index
entry, then passed after publishing the committed entry and supporting an
absent live index. Coverage now totals eleven new tests across the library,
`repository_enablement`, and `mutation_confirmation`: empty/absent create,
born/unborn enable, unborn HEAD movement, immediate ticket creation and
remote selection, foreign locks with/without an existing index, domain retry,
F2 partial-create retry/replay, lost settlement, later configuration staging,
HEAD advancement, and unrelated staging/unstaged edits after validation.

One additional red test exposed a recovery detail: `LocalStep::continued`
discarded `ConfigurationWritten`, so a retry after the commit but before its
recorded step could not recognize its own configuration write. Retaining that
existing step fixes the repair without changing stored codes or effects.
The test now passes with a foreign index lock on the initial attempt and HEAD
advanced before F2 retry; it reports the original initialization commit.

Index writes remain outside rollback and under the existing repository lease.
The shared helper takes Git's index lock before loading the live snapshot,
preserves unrelated entries and mtime, and writes only the committed path.
Domain recovery adds only a missing entry, with the current HEAD blob/mode
and a matching regular worktree file; existing staged configuration is left
alone. Finished F2 replay remains read-only.

### Workaround audit

- Removed enablement re-adds in `tests/support/items.rs` (both `commit` and
  `create_document_context`) and `tests/discovery_rebuild.rs`.
- Removed D1's whole-index reset in `src/desktop/test_support/fixtures.rs`.
- Kept and documented `src/repository/comment_publication_tests.rs` staging:
  this fixture writes its own configuration and stages it for its **initial
  commit before enable is called**. It is baseline construction, not repair.
- Removed extra enable-only resets/re-adds in mutation confirmation,
  repository enablement and recovery foundation tests. Converted the
  local-authoring configuration-cleaning helper to a read-only assertion.
- Kept publication-specific staging where it tests state after a publication
  commit. The negative missing-entry test now explicitly stages deletion.
- Two fixture commit/staging paths needed `index.read(true)` because their
  cached Git handles predated the service's write, which preserves index
  mtime. They now reload the live index, without staging configuration.
- Two recovery assertions formerly required the index to remain absent even
  after successful initialization. They now assert blob/mode and clean status;
  precommit rollback assertions still require unchanged index bytes.

### Evidence so far

All Rust commands used Devenv. Targeted results:

- New `enablement_index` tests: **11 passed** (1 library, 4 F2, 6 domain).
- Existing local-authoring index tests: **11 passed**.
- Discovery rebuild: **79 passed**.
- Mutation confirmation: **174 passed**.
- Recovery foundation: **52 passed**.
- Local authoring: 139 passed initially; the one cached-handle fixture failure
  was fixed and its exact test passed.
- Repository enablement: 86 passed initially; the four stale-expectation or
  cached-handle failures were fixed, and all six `recovery_` tests passed.
- `cargo fmt` and `git diff --check` completed successfully.

Final all-feature/desktop gates and independent review remain. A fresh fetch
still finds `origin/main` at `3bda743`; the owner's pending registration fix
has not appeared there yet. No push, PR, closure or integration merge done.
