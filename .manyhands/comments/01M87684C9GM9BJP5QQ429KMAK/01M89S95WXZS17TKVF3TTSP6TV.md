---
id: 01M89S95WXZS17TKVF3TTSP6TV
item_id: 01M87684C9GM9BJP5QQ429KMAK
created_at: 2026-11-25T16:41:26.080102171Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Wave 06 Cycle 03 (vh-gy8gz, ticket 01M87683QK8E34EYY8KS4VQG9Y) was approved on 2026-11-25 and is now being implemented on branch manyhands/ticket/01M87683QK8E34EYY8KS4VQG9Y. Its Cycle document is docs/Cycles/wave-06-cycle-03-pull-request-close.md on that branch.

For document promote in pull-request mode: Cycle 03 adds the registry table `pull_request_links` (the per-item record of given links, deleted when a context is retired), the read field `ItemDto.pull_request` (filled by show only), and the preview effect `pull_request_link`, all for documents to reuse. The close's answer data type is the close's own, so promote gets its own. Cycle 03's pull-request close is a mode of the existing ticket-closure lifecycle (branch records in `ticket_closures`, publication through a second child `ChildRole::Publication`); a promote may want the same shape in the promotion lifecycle.