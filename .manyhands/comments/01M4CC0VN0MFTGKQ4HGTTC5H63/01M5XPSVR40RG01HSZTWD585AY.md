---
manyhands_managed: true
manyhands_kind: comment
id: "01M5XPSVR40RG01HSZTWD585AY"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-27T03:34:59Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Part 0 checkpoint: work contexts

Part 0 is complete. Its two library commands, `context prepare` and
`context release`, passed a review of their own, and desktop work starts
next.

- **Review:** approved, with no blocking findings and four minor ones.
  - **Lost answers.** Neither command records a checkpoint. So a release
    whose answer was lost, sent again after a later Edit, removes that
    newer context if it is clean. A lost prepare sent again after a
    release makes the context again. No saved work can be lost. The owner
    stated this as decision 11's exception (R-78, in Known Limits). Task 17
    gains two tests that keep the desktop from sending either request again
    across the other.
  - **A refused release of a branch with no worktree** left its row
    pending, though nothing was touched. That blocked every other operation
    on the repository until the row was abandoned. Fixed in `d818519`: the
    row is now discarded, as the design's own rule says.
  - **Design step 6** now says that a moved branch holding its own commit
    is repaired by hand.
  - **An abandon preview** of a stuck release does not name the item. Left
    for the whole-branch review.
- **E-31 (owner).** Task 1's new schema had broken an enumeration contract
  test that Task 1 did not run. A named edit now ties the schema's `kind`
  to the Rust list.
- **Tests** at `d818519`, every Part 0 target in full, all passing:
  - mutation_bridges 166, mutation_replay 308, mutation_confirmation 183;
  - mutation_remote 88 SSH cases;
  - mutation_contract 44, read_contract 71, local_authoring 140;
  - the library unit tests, 1,297.

  The first run had 7 failures, all in tests that re-launch their own test
  binary. Another build had replaced that binary while the run was going.
  The repeat, with nothing else building, passed.
- **Main** is unchanged at `0464c42`, and the branch merges cleanly with it.
