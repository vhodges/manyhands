---
id: 01M5QA0S348VQ8B0NSXF5R5CSG
item_id: 01M5MHQ16VD0P2RTGY8PMRYWGY
created_at: 2026-10-24T16:22:12Z
created_by: Claude Opus 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---

## Question for the owner

Parsing now keeps unknown keys in file order (`8b9909e`). The read views
were never affected by the swap, though. They sort unknown keys by name on
purpose, so that their JSON does not depend on serde_json's
`preserve_order`. Only the desktop dependency graph turns that feature on;
`cargo tree` shows that the CLI-only build does not. The sort is in
`yaml_mapping_to_json` (`src/repository/discovery.rs:276`). The read DTOs
document it as "in key order at every depth"
(`src/repository/read/dto.rs:456` and `:596`), and
`tests/read_items.rs:1628` pins it: "Keys are in order, whatever order the
file has them in." So the acceptance item "a comment's and a ticket's
unknown metadata are reported in file order" would change an existing,
documented read contract.

1. What order should the read views report unknown metadata in?
   - **a. Keep sorted order (recommended).** I would replace the read
     acceptance item with this one: a comment's and a ticket's unknown
     metadata are reported in key order, whatever their order in the file.
     I would add a comment read test for it (tickets are already pinned).
     I would still switch `take_author` and `UnknownMetadata::from_ticket`
     to `shift_remove` for consistency; nothing visible changes.
     Consequence: no contract change, and the ticket can go to review in
     the next run. If you still want file order in reads, say so, and I
     will list it as a proposed follow-up for `planning-a-cycle`.
   - **b. Report file order, in this ticket.** I would drop the sort and
     turn on serde_json's `preserve_order` for the library, so that the
     CLI-only build, which uses a BTreeMap today, keeps file order too.
     Consequence: the read contract, the DTO docs and
     `tests/read_items.rs:1628` change. `preserve_order` then applies to
     every JSON object the CLI emits, so CLI goldens may change. That is a
     contract change and a dependency feature change, in a ticket filed as
     a bounded defect.

**Paused:** the read views (step 4 of the plan) and the review. **Already
committed:** the start (`6ca50a5`), the parsing fix and its three canonical
tests (`8b9909e`), and the checkpoint `01M5QA0S297S3ZBPRG1W2R6T4Z`, with
this question.

**How to answer:** add a reply comment with
`parent_id: 01M5QA0S348VQ8B0NSXF5R5CSG`. This is a loop run, so an answer
in chat is not seen.
