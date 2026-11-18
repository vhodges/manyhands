---
id: 01M6YPR343VTQG1M548RZQ37FR
item_id: 01M4CC0VN3VPVVQ3RNWJKXWEVX
created_at: 2026-11-08T23:09:02.975011375Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
## Planning checkpoint: drafts committed, Wave 04 Cycle 01 coordination

- **Drafts committed and pushed.** The Cycle, design and implementation plan (revision 1, 2026-11-07) were committed as `08fb1d05`. origin/main `8da9afd4` was merged as `1869cf14` with no conflicts. All three remain draft and unreviewed.
- **Coordination with Wave 04 Cycle 01** (`01M6V8RK8B0SBCH0YZXFVSR9E3`) was added in `fd6d0038`. Cycle 01 starts before D6 can, so D6 plans to merge second and adapt:
  - D6's five attention states (key locked, host not approved, key missing, configuration required, recovery hold) join Cycle 01's Needs attention count through one read-only `attention()` query. Paused, off, can't reach, checking soon and waiting for a conflict do not count (proposed decision 11).
  - "Still stopping" stays in `finishing_text` only, so Cycle 01's rule that a waiting quit shows Activity covers it.
  - The scheduler is held by `Services`. The view edit is one call beside the launch refresh in `repositories_loaded`.
  - The Settings page stays movable for Wave 04 Cycle 04. Footer and Repositories polling states are Wave 04 Cycle 03's, read from D6's model.
  - New in the documents: A18, design W-1 to W-5, R-14 to R-16, E-4 and E-5, and a Re-grounding section.
- **Re-grounding also covers `PollDisposition::Unpublished`.** It landed on main with `01M66HFV44YXNQ7ANJVA16TQ1H`, and the design's Fact 5 omits it.
- **Still waiting** for `01M6KHVD6D0DB8KYT6VGQP10Y0` to merge, then for re-grounding, review rounds and approval. No tests were run; these are documentation changes only.
