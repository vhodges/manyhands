---
id: 01M5E3V4YT69CJ47ZPCMZDKCWJ
item_id: 01M5B0Z6PNP7X4WH4JV3N0VQJ6
created_at: 2026-10-21T02:15:01Z
created_by: Claude Opus 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---

## Preflight and investigation; design awaiting owner approval

Working in this ticket's existing worktree and branch:

- Worktree: `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M5B0Z6PNP7X4WH4JV3N0VQJ6`
- Branch: `manyhands/ticket/01M5B0Z6PNP7X4WH4JV3N0VQJ6`
- Before: `9f946ac5ece129144561777582dbfa7c89890820`
- Fetched `origin/main`: `3bda743aff8ab8b82963ef35427583dd4ef73e13`
- After merge: `48dc0246b3aa666059e27fb03730b732609a474e`

Read `AGENTS.md` from the fetched main. Used merge, not the starting-a-cycle
skill's stale rebase instruction. Merge completed without conflicts, and
`git merge-base --is-ancestor origin/main HEAD` passed. No implementation edits
have been made. The merge commit has the requested co-author trailer.

The owner identifies this library-only defect as a prerequisite of Wave 03 C2
(`01M4CC0VMTTNKVYS6XWQG4CK1N`). The unpushed registration fix `01M5CVB2`
changes the valid-configuration branch of `enable_recorded` and the F2 repo
binding. Reviewed that diff read-only; it must be merged from `origin/main`
when published before this work becomes review-ready.

### Evidence

- `initialization_tree` builds a detached index and writes its tree to Git;
  `enable_recorded` commits it without publishing the configuration entry to
  the worktree index. Create and partial-create recovery converge on enable.
- `ensure_configuration_path_clean` rejects an absent configuration entry.
  The existing test
  `publication_selection_rejects_a_missing_live_index_configuration_entry`
  demonstrates this directly: enable, add remote, select remote, and expect
  `DirtyConfigurationPath`, with no explicit index deletion in the setup.
- Ran `devenv shell -- cargo test --locked --test repository_enablement
  publication_selection_rejects_a_missing_live_index_configuration_entry -- --exact`:
  **1 passed, 0 failed**. The initial cold-build invocation timed out at 120
  seconds; the repeated command completed. This reproduces the existing bad
  behavior, not verification of a fix.
- Save's `refresh_owned_index` already takes the worktree-specific Git lock,
  reads the live index under it, preserves unrelated entries and timestamps,
  and writes only owned entries from committed blobs. It assumes the live
  index exists; fresh/unborn repositories require handling an absent index.
- Save treats post-commit index publication as best effort, preserving its
  existing outcome. Its no-change domain call attempts repair again.
- Finished F2 replay is explicitly read-only and returns the stored result.
  It must not overwrite later user staging or restore an old configuration
  after HEAD advances. Unfinished retries and lost-result recovery are
  distinct paths that need coverage.
- Enable currently requires a clean worktree at entry. Preservation tests
  must exercise unrelated staging after validation or during recovery, rather
  than silently broadening enablement to accept initially dirty repositories.

### Proposed design for approval

Reuse the save helper, extending it for an absent index. Refresh only
`.manyhands/config.toml` from the committed blob and mode under the existing
repository lease, after commit success and outside the rollback block. Cover
unfinished retry recovery, including interruption between the commit and its
recorded step, without reverting later configuration commits or user staging.
Finished F2 replay remains read-only and retains the clean index produced by
the original successful attempt.

Proposed failure policy: follow save's best-effort rule. A foreign index lock
or failed index write preserves the authoritative commit, existing result,
and foreign lock/index; a subsequent eligible domain enable/recovery can
retry repair. This explicitly means a failed index publication can leave a
successful commit with a stale index. Finished F2 replay is not a repair
operation. Owner approval of this exception is required before implementation;
reporting a new partial/pending outcome instead would require a contract
decision. No public code/effect/schema change is proposed.

TDD will cover empty/absent create, born/unborn enable and HEAD movement,
immediate ticket creation and remote selection, unrelated staged/unstaged
preservation, F2 retry/replay/lost result, partial create recovery, missing
index, and post-commit index-write failure. Retain the negative publication
test by explicitly removing the configuration entry in its fixture.

Remove the four named workarounds and audit additional enable-only repairs
found in `tests/support/items.rs::create_document_context`,
`tests/mutation_confirmation.rs`, and `stage_configuration` callers in the
enablement/recovery suites. Keep intentional staged-state setup and distinct
publication-commit behavior where those tests need it.

Use targeted Devenv tests during implementation, then the four AGENTS gates
and the two requested desktop gates once at the end. Synchronize published
main and run merge-tree before review-ready; obtain independent review and
owner approval. Push, PR, integration merge, cleanup and native CI each await
the owner's authorization. The `.git/config` locking defect stays out of scope.
