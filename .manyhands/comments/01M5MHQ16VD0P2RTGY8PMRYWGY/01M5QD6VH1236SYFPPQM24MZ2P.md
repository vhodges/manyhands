---
id: 01M5QD6VH1236SYFPPQM24MZ2P
item_id: 01M5MHQ16VD0P2RTGY8PMRYWGY
created_at: 2026-10-24T16:51:52Z
created_by: Claude Opus 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---

## Checkpoint: read views

**Resumed** on the owner's answer `01M5QCW6RA6KS2YGDD7R8Y4WVH` (option a,
keep sorted order). Merged `origin/manyhands/ticket/01M5MHQ16VD0P2RTGY8PMRYWGY`
(already up to date) and `origin/main` at `a684995`, with no conflicts
(HEAD `8aea949` before, `5e33e91` after). Status back to `implementing` in
`9fc2b6d`.

**Done.**

1. Step 4, in `2a9ecc9`. `take_author` (`src/repository/read/comments.rs`)
   and `UnknownMetadata::from_ticket` (`src/repository/discovery.rs`) use
   `shift_remove`. Nothing a read returns changes.
2. New test
   `unknown_metadata_is_in_key_order_whatever_order_the_file_has_it_in`
   (`tests/read_comments.rs`). Two comments have unknown keys `zeta`,
   `created_by`, `mood`, `alpha`. The parser takes the first comment's
   `created_by`, and the read takes the second's, which names nobody. Both
   report `alpha`, `mood`, `zeta`. The test passes as written, since it pins
   an existing contract. With the sort in `yaml_mapping_to_json` taken out
   for a moment, it fails with `["zeta", "mood", "alpha"]`, the file order
   that the parsing fix now keeps.
3. The read acceptance item in the ticket now reads as the owner gave it.
   The ticket read is already pinned by
   `unknown_metadata_is_json_and_a_value_with_no_json_form_is_null_and_reported`
   (`tests/read_items.rs`).

**Tests run,** at `2a9ecc9`:

- `cargo fmt --check`, `check --all-features` and
  `clippy --all-targets --all-features -- -D warnings`: pass.
- `--test read_comments`: 23 passed. `--test read_items`: 76 passed.
- `--lib repository::discovery`: 42 passed. `--lib repository::read`:
  65 passed.

The serde_json `Map::remove` calls in `UnknownMetadata::from_stored` and
the repository config's `toml::Table` removals were checked and left alone.
The first reads a fixed two-key wrapper. toml's `remove` already keeps order
under `preserve_order`.

**Next:** review (§6), then review-ready.
