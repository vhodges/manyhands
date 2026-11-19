---
id: 01M704JTNDBNMQ1C96SDXYS3ZZ
item_id: 01M4CC0VN3VPVVQ3RNWJKXWEVX
created_at: 2026-11-09T12:30:01.378858595Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
## Planning checkpoint: re-grounded on context retirement and the approved Cycle 01

- **Main merged.** origin/main `f4442618` (the closed retirement ticket `01M6KHVD6D0DB8KYT6VGQP10Y0`) was merged as `9512080d`. The documents are revision 3 in `084cddea`. They are still draft and unreviewed, and no tests ran (documentation only).
- **Library re-grounded at `9512080d`.**
  - New dispositions: `Closed`, `Promoted` and `Unpublished`.
  - `PollOnceRetiring` exists. It is also `Explicit`, and D6 never uses it.
  - An explicit history repair now retires the recovery marker. Rows already suspended stay suspended, so decision 10 stands.
  - The poll entry points, errors, eligibility and cancel are unchanged; only their lines moved.
  - Details are in the design's Re-grounding and in Facts 24–28.
- **Retire added, as the ticket assigns.** It is design Part E and plan Tasks 11–12; the evidence tasks are now 13–14.
  - Retire… is offered on each `closed` or `promoted` target and submits `ContextRetire` for one item on the user lane.
  - Text no draft keeps stops it. There is no "Retire anyway", and drafts are flushed first and never removed.
  - Refusals show the library's code. A pending retirement leaves D5's pending row with Resume and Abandon.
  - No automatic path retires. No SSH case is added.
  - New in the Cycle: Scope 12, contract 11, A19–A21 and proposed decisions 14–15. Open source questions: U-9 to U-11.
- **Following Cycle 01's approved decisions** (its note `01M6Z8A8AYDGTK4BWKZR3931ST`):
  - D6's five attention states become a third input to `needs_attention`.
  - A waiting quit changes no layout, so "still stopping" lives in Activity and shows nothing while Activity is hidden. This is proposed decision 12 and K-13, and it narrows the Wave's ten-second report to Activity.
  - D6's own operation-area rows wrap inside Activity; the three existing rows of Cycle 01's proposed K-19 stay with Cycle 03. This is proposed decision 13.
- **Waves.** Wave 03's D6 section has a factual status note. Wave 04 is untouched, because Cycle 01's branch already carries the matching amendments.
- **Next.** Review rounds, then approval of decisions 3–15. Once Cycle 01 merges, check W-1 to W-7 against its code.
