---
manyhands_managed: true
manyhands_kind: comment
id: "01M6FVRDRFJWF7XWCMAJ5HXE0P"
item_id: "01M4CC0VMXTNXNH78C11DYXS8H"
parent_id: "01M6FGE9VFKZPWW36CGP9FN3BK"
created_at: "2026-11-03T04:47:54Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 13a: interruption and no worker

30 `c5_interruption_` cases now run through the binary over SSH. They cover:

- Ctrl-C before and after integration for both lifecycle verbs, `poll once` and the two context
  mutations, then a cancel, the same request, and a new ID that decision 33 holds;
- a second press, the consent prompt and a press at it, and a press during `operation resume`;
- a killed `poll once` resent under its ID, and lost stdout for each verb;
- the primary `Err` arm's row, which answers `cancelled` and releases the child;
- `c5_interruption_no_resident_worker`, with its positive control.

All 30 pass, in 159 s. They pin behaviour Parts A and B built.

**Stopped for a ruling:** Review Focus 4. A press after the push command of a closure whose primary
push is then refused answers `cancelled` (130), not the domain's `push_rejected`. The `Err` arm reads
the cancellation after the failed push. Otherwise the run holds, and the same request finishes once.
That row is left out of the commit.
