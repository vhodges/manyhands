---
id: 01M7R2ME33C2JYJ6VFR373VVXQ
item_id: 01M6RAS63EWVHWWVBSZ836RXG2
created_at: 2026-11-18T19:46:59.188387259Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Superseded by `01M7R2GPPF7F7JRKHX8C5ZZ4PF` (vh-4y6na, user-visible desktop gaps and correctness holes) and `01M7R2GPPF135ZT7M4TZCR09XT` (vh-fdp7z, hygiene, test support and tidy-up). Open items were checked against main at `5d80bc15` on 2026-11-18; the code was read, no tests were run.

**Done by Wave 04** (see the two earlier comments here). Cycle 03b (`01M784Y94Q3AXT6XJX38F5VK1V`): the split of `model/operations.rs` into `model/operations/lifecycle.rs` and its siblings (now 693 lines), and `view/operations.rs` (now 154 lines). Cycle 05b (`01M7D7JK5MSY6J471JEKG3CZXR`): the Copy items, one name for the resolve sheet's entry points (`REVIEW_CHANGES`) and shared label constants (`RESUME`, `KEEP_MINE` and `share_main()` in `model/activity/copy.rs`).

**Found moot in the code:** the "30 s `thread::sleep`" in `view/operations_tests.rs`. At `:537-540` it is a child-process helper that polls every 10 ms for a release file under a 30 s deadline, not a fixed wait.

**Moved to `01M7R2GPPF7F7JRKHX8C5ZZ4PF`:** lifecycle locks keyed by path, not `String` (a non-UTF-8 root silently turns the lock off), and the surviving "Keeping mine" string in the item-edit review.

**Moved to `01M7R2GPPF135ZT7M4TZCR09XT`:** the AGENTS.md SSH-tier timing line, the process references in production comments (now about 186 hits in `src/desktop` and about 230 across `src`, against the 61 counted here), library L1 (`_kind`, `Option<Option<_>>`, the send-again ID), and the shared `RESOLVING` wait, `until_drawn_within` and fixtures.

**Recorded as "not scheduled" in `01M7R2GPPF135ZT7M4TZCR09XT`:** the drain split, the `Folder` rename (L4), `RepositoryService::data_directory()`'s home, the pending-row refresh cache, the bind-level SSH boundary-stage test, and the worker-thread leak per desktop test.

**Closed as accepted limits, not re-verified after Wave 04 except where noted:** K-28 (Activity's Retry resubmits without the lifecycle-lock check; still true at `view/workspace/requests.rs:597-599`, and the owner called it rarely used); K-29 (save-first skips edits while the lock is `Awaiting`, a window of milliseconds, `view/item/edit/save_first.rs:40-45`); K-30, K-31 and K-32 (rare or narrow cases; the resolve surface they mostly concern was rewritten in 05b and there is no `set_text` left in `view/conflict`); K-33 (library and CLI contract wording; "Closing…" while `Awaiting` is still shown at `view/lifecycle/stages.rs:78`). File a new ticket with a reproduction if any of them is seen in use.
