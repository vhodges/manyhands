---
id: 01M5QA0S297S3ZBPRG1W2R6T4Z
item_id: 01M5MHQ16VD0P2RTGY8PMRYWGY
created_at: 2026-10-24T16:21:50Z
created_by: Claude Opus 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---

## Checkpoint: parsing fix and goldens

**Done.**

1. Steps 1 and 2, in `8b9909e`. `remove_known` (`src/canonical.rs`) now
   uses `Mapping::shift_remove`. Three new tests in
   `tests/canonical_foundation.rs`, one each for a document, a ticket and a
   comment whose unknown keys follow their known keys, pin the serialized
   bytes and check that the output is a fixed point. With the fix taken out
   again for a moment, all three fail; with it, they pass.
2. Step 3. No golden recorded the swapped order, so none changed.

**Tests run,** at `8b9909e`. `origin/main` (`90f3de2`) and
`origin/manyhands/ticket/01M5MHQ16VD0P2RTGY8PMRYWGY` were merged first;
both were already up to date.

- `cargo fmt --check`, `check --all-features` and
  `clippy --all-targets --all-features -- -D warnings`: pass.
- `--lib closure`: 176 passed.
- `--test canonical_foundation`: 65 passed.
- `--test document_promotion` and `--test ticket_closure`: pass.
- `--test comment_publication`: pass, 41 SSH cases included.
- `--test local_authoring`: 140 passed. `--test mutation_relationships`:
  27 passed.
- `--test read_items`: 76 passed. `--test read_comments`: 22 passed.

**No canonical Markdown rewritten.** `git diff origin/main...HEAD --stat`
touches only `src/canonical.rs`, `tests/canonical_foundation.rs` and this
ticket's own files.

**Note.** The previous run ended while these tests were running in the
background. It left the question that follows as an uncommitted draft and
did not write this checkpoint. This run reran the tests and commits both.

**Next:** step 4, the read views, once the owner answers the question that
follows.
