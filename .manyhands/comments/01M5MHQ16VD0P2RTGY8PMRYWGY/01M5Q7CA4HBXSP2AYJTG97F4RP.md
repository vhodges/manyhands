---
id: 01M5Q7CA4HBXSP2AYJTG97F4RP
item_id: 01M5MHQ16VD0P2RTGY8PMRYWGY
created_at: 2026-10-24T15:09:06Z
created_by: Claude Opus 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---

## Started

**Base.** Merged `origin/main` at `90f3de2` into the ticket branch, with no
conflicts (HEAD `40aa204` before, `878fdf9` after). The ticket has no
`deps`. Loop mode: the driver pushes the branch after each run.

**Plan.**

1. Write canonical tests for a document, a ticket and a comment whose
   unknown keys follow their known keys. Each pins the exact serialized
   bytes and checks that the output is a fixed point. Watch them fail.
2. Make `remove_known` (`src/canonical.rs`) use `Mapping::shift_remove`.
   That one helper covers every known-key removal in `parse_item` and
   `serialize_item`. Update the `serialized_extra` test doc comment, which
   records the old limitation.
3. Run the targets that pin serialized bytes. Update any golden that
   recorded the swapped order, and explain the change.
4. Read views: the owner decides (see below), then implement and test.

**Acceptance and tests.**

- Canonical tests: new tests in `tests/canonical_foundation.rs`, one each
  for a document, a ticket and a comment.
- Read tests: on hold for the owner. `yaml_mapping_to_json`
  (`src/repository/discovery.rs:272`) sorts unknown keys by name on purpose,
  so that the JSON does not depend on serde_json's `preserve_order`. That
  feature is on only in the desktop dependency graph.
  `unknown_metadata_is_json_and_a_value_with_no_json_form_is_null_and_reported`
  (`tests/read_items.rs:1628`) pins that sorted order: "Keys are in order,
  whatever order the file has them in." So `take_author` and
  `UnknownMetadata::from_ticket` reorder nothing that a read returns, and
  "reported in file order" would change an existing contract. I will ask
  after step 3.
- `closed_ticket_bytes` and goldens: `--lib closure` and
  `--test ticket_closure`, plus the other targets that pin serialized
  bytes: `document_promotion`, `local_authoring`, `comment_publication` and
  `mutation_relationships`.
- No canonical Markdown rewritten: `git diff origin/main...HEAD --stat`
  touches no `.manyhands/` or `docs/` file other than this ticket's own.
