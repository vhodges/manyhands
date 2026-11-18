---
id: 01M5QGZMM74HK7T84VWA54HT4Y
item_id: 01M5MHQ16VD0P2RTGY8PMRYWGY
created_at: 2026-10-24T17:57:50Z
created_by: Claude Opus 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---

## Review-ready

**What changed.**

- `remove_known` (`src/canonical.rs`) uses `Mapping::shift_remove` in place
  of `Mapping::remove`, which is `swap_remove` (`8b9909e`). Unknown keys
  keep their file order through parse and serialize. Known keys keep their
  canonical placement.
- `take_author` (`src/repository/read/comments.rs`) and
  `UnknownMetadata::from_ticket` (`src/repository/discovery.rs`) use
  `shift_remove` too, for consistency (`2a9ecc9`). Nothing a read returns
  changes.
- The read views keep their documented key order, by the owner's decision
  `01M5QCW6RA6KS2YGDD7R8Y4WVH`. The read acceptance item was replaced as the
  owner gave it. The decision also supersedes the "and the read views" part
  of the first Expected Behavior bullet, which was left as filed because
  the owner authorized replacing only the acceptance item.

**Acceptance and tests.**

- Canonical tests, one each for a document, a ticket and a comment whose
  unknown keys follow their known keys:
  `a_documents_…`, `a_tickets_…` and
  `a_comments_unknown_keys_keep_their_file_order_after_its_known_keys`
  (`tests/canonical_foundation.rs`). Each pins the bytes and checks the
  fixed point. Each fails without the fix.
- Read tests, key order whatever the file order:
  - comment:
    `unknown_metadata_is_in_key_order_whatever_order_the_file_has_it_in`
    (`tests/read_comments.rs`), new. It fails with the sort taken out.
  - ticket:
    `unknown_metadata_is_json_and_a_value_with_no_json_form_is_null_and_reported`
    (`tests/read_items.rs`), existing.
- `closed_ticket_bytes` and goldens: `--lib closure` and
  `--test ticket_closure` pass. No golden recorded the swapped order, so
  none changed.
- No canonical Markdown rewritten: `git diff origin/main...HEAD --stat`
  touches `src/canonical.rs`, `src/repository/discovery.rs`,
  `src/repository/read/comments.rs`, `tests/canonical_foundation.rs`,
  `tests/read_comments.rs` and this ticket's own files only.

**Commands run,** at `4ba561f`, all through Devenv:

- `cargo fmt --check`, `cargo check --all-features --locked` and
  `cargo clippy --all-targets --all-features --locked -- -D warnings`:
  pass.
- `--test canonical_foundation`: 65 passed.
- `--test read_comments`: 23 passed. `--test read_items`: 76 passed.
  `--lib repository::discovery`: 42 passed. `--lib repository::read`:
  65 passed. These ran at `2a9ecc9`; `4ba561f` changes only a test
  comment.
- `--lib closure`: 176 passed.
- `--test ticket_closure`: exit 0, 73 SSH cases passed.
- `--test document_promotion`: exit 0, 74 SSH cases passed.
- `--test comment_publication`: exit 0, 41 SSH cases passed.
- `--test local_authoring`: 140 passed.
  `--test mutation_relationships`: 27 passed.

**Not run:** the full `cargo test` suite and native CI, which run after
merge.

**Review.**

- Round 1, an independent reviewer: 0 Blocking, 0 Important, 3 Minor.
  1. The Expected Behavior bullet contradicts the decision. Not edited;
     recorded above.
  2. The read test cannot fail on the swap itself, because of the sort. No
     change: it pins the contract the owner chose.
  3. An unclear test comment. Fixed in `4ba561f`.
- Round 2, re-review of `4ba561f`: all three resolved or accepted, nothing
  broken.

**Proposed follow-ups:** none beyond the ticket's own "Follow-up After
Merge".

**merge-tree:** `git merge-tree --write-tree HEAD origin/main` is clean
(tree `3ee9524`), with `origin/main` at `a684995` as the loop driver
fetched it.
