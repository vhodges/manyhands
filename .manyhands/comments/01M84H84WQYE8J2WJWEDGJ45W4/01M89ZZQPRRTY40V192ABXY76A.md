---
id: 01M89ZZQPRRTY40V192ABXY76A
item_id: 01M84H84WQYE8J2WJWEDGJ45W4
created_at: 2026-11-25T18:37:57.012653644Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
From Wave 06 Cycle 03 (vh-gy8gz), now in implementation: its Task 4 (commit 7cf3f9c2 on manyhands/ticket/01M87683QK8E34EYY8KS4VQG9Y, published) adds `schemas/v1/pull_request.schema.json` and `PullRequestDto`, since Cycle 02's are not on main yet, in Cycle 02's shape `{ source, target, forge, url }` (`forge` one of the four kinds or null, `url` a string or null), with `PullRequestDto` built from `PullRequestLink`.

One difference from Cycle 02's design ("It is always an object"): the file's type is `["object", "null"]`. `ItemDto.pull_request` must be null in lists, for documents and for a ticket without a link, and the schema checker allows only a bare `$ref`, so the referenced file has to admit null, as Cycle 02's `repository_setup.schema.json` does. A setup's `pull_request` that is always an object still validates against it. If Cycle 02 adopts the same file, whichever Cycle lands second merges it without a conflict; if Cycle 02 keeps an always-object file, the second to land needs a nullable variant for the item.

Also: `PullRequestDto.forge` is `Option<String>`, as `ForgeDto.kind` is. If Cycle 02 types it differently, that is a one-line merge.