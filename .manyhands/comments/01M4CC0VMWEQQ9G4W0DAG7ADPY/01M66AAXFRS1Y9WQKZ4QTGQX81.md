---
manyhands_managed: true
manyhands_kind: comment
id: "01M66AAXFRS1Y9WQKZ4QTGQX81"
item_id: "01M4CC0VMWEQQ9G4W0DAG7ADPY"
parent_id: "01M65KWK66M3RFEC1ADJ3SRNHS"
created_at: "2026-10-30T11:50:16Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 12a complete; the task review is pending

The comment and sync journeys run through the binary between two clones of
one SSH remote. Only CLI human output and tests changed: there is no library
change, so no `--lib` target ran.

- **The stop and the ruling.**
  - The first run stopped: `remote select` commits the configuration without
    updating the worktree's Git index, so the next sync is
    `worktree_not_clean`.
  - The owner ticketed it on main as `01M669FJR27FRHCNPWAMD6VH4V`.
  - `CliPair` now seeds an already-configured primary. Each clone's
    `remote select` still runs through the binary and answers
    `already_applied`, and there is no index workaround.
- **`CliPair`** (`tests/support/cli_pair.rs`). The server's primary is seeded
  with the configuration and two tickets before either clone is made.
  - Each clone's own setup goes through the binary: a selected key, an
    identity, enablement, the publication remote and the approved host.
  - The key comes first: selecting a key after a publication remote is
    configured fences the repository's remote history.
- **The fix** (`cbbca6ef`). An `operation resume` that meets a deleted branch
  offers no cancellation, so its human output now says which request holds
  the reservation and how to free it, and prints the `item republish
  --prepare` line. JSON is unchanged. Three unit tests were written first and
  failed.
- **The cases** (8 `c4_collaboration_` SSH cases):
  - comment publish success and refusal;
  - one saved comment after the retry;
  - ordered replies from both clones;
  - `item sync` and `repo sync`: already current, fast-forward and clean
    divergence;
  - `operation resume` publishing a stopped comment;
  - the deleted-branch journeys, comment and resume, including the way out
    they name.

  Human-mode progress, replays and `request_mismatch` are covered for
  `comment add`, `item sync`, `repo sync` and `operation resume`. The new
  goldens are `item_sync_ok`, `repo_sync_ok` and
  `operation_resume_comment_ok`.
- **Verification.** `cli_remote -- c4_collaboration`: 8 SSH cases passed. The
  bin: 252 passed. Clippy (all targets and features), `fmt --check`,
  `check --bin manyhands-cli` and the golden-registration test pass.
- **Review.** The task review is pending.
