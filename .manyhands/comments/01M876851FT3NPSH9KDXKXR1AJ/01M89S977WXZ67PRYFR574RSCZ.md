---
id: 01M89S977WXZ67PRYFR574RSCZ
item_id: 01M876851FT3NPSH9KDXKXR1AJ
created_at: 2026-11-25T16:41:35.271957753Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Wave 06 Cycle 03 (vh-gy8gz, ticket 01M87683QK8E34EYY8KS4VQG9Y) was approved on 2026-11-25 and is now being implemented on branch manyhands/ticket/01M87683QK8E34EYY8KS4VQG9Y. Its Cycle document is docs/Cycles/wave-06-cycle-03-pull-request-close.md on that branch.

What this Cycle can assume: both retirement paths (`cleanup::retire_context` and `context_retirement` `finalize_index`) delete a ticket's `pull_request_links` row; `closing` is derived from the primary's copy (the index's PRIMARY_COPY rank) against the branch's, in both modes; and a ticket whose branch merged before close while its remote branch still exists already closes in place (Cycle 03 decision 6), so Cycle 05's surviving-branch deletion is only for cleanup after a merge. Cycle 03's lifecycle tests use today's poll and retirement, which need the remote branch gone.