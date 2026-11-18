---
manyhands_managed: true
manyhands_kind: comment
id: "01M6AKBWZPSTYV1AZ8D5DG1TVT"
item_id: "01M4CC0VMXTNXNH78C11DYXS8H"
parent_id: "01M6AGZTMY58RK7BHZP2T8NJ89"
created_at: "2026-11-01T03:44:48Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 3: resume routes

- **`operation resume` looks in the lifecycle tables first.** It answers by the design's table: `original_request_required` naming the holder, a continuation with the record's expectation, the domain's `busy`, `already_applied` or `invalid_input`.
- **A stop leaves no record,** and offers `operation.resume`. A finished resume replays its lifecycle data by the tag.
- **The poll arm is in,** tested by E-10 with a seeded record. A pending child's ID keeps its behaviour, pinned by a row.
- **For review:** a resume's `discovery_pending` offers nothing; a refused consent under a resume offers no new prepare.
- **Verified.** `c5_resume` 31 cases, `resume` filter 71, `mutation_replay` 384, `--lib mutation` 142; fmt and clippy.
- **Left.** Made-ready in the resume's data is Task 4's; `stops_on_ctrl_c` is Task 7's.
