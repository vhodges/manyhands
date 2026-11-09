---
id: "01M6M7EWZ6ZDGC5SEVMZ9TBWAS"
item_id: "01M6KHVD6D0DB8KYT6VGQP10Y0"
parent_id: "01M6M5M6SX544VJS6WTJCWSKVS"
created_at: "2026-11-04T21:29:23Z"
created_by: "GPT-6.1 Sol <noreply@openai.com>"
manyhands_managed: true
manyhands_kind: comment
---

## Checkpoint: second planning review and conservative lifetime recovery

Round 2 reviewed `6a6301be` read-only, including adversarial crash/retry cases:

- Source/feasibility: 0 Blocking, 3 Important, 0 Minor.
- Adversarial/contract: 1 Blocking, 3 Important, 0 Minor.

The concern/resolution rows are in the Cycle. These are planning findings, not
runtime tests or an implementation approval. No Cargo command or Rust edit ran.

## Owner decision (chat, 2026-11-04)

13. “Update existing preview (Recommended)”: the existing CLI and desktop
    operation-abandon confirmation may gain retirement safety details and tests.
    This adds no desktop Retire action; D6 still owns that action.
14. “Retirement path only (Recommended)”: consume only this retirement's exact
    old materialization intent after proven cleanup/absence and test a later
    same-ID/new-OID document edit. The wider existing promotion/closure omission
    stays a proposed follow-up; do not fix it or file a ticket here.

## Revised rules for the third review

- A lost admin token gives no branch-lifetime proof. A new executor may not
  delete a surviving tokenless/branch-only ref under the old request, even if its
  OID matches. Preserve, guarded-abandon and require a new explicit request.
  Local absence can finish exact metadata/index settlement only (R-7).
- A provisional intended nonce is not continuity proof. After interrupted
  bootstrap, a missing token refuses the old candidate; only an installed full
  matching token can finish its identity commit. Never stamp an old nonce into
  a possibly recreated registration.
- Separate observer transport authorization from batch-attempt receipt consumer
  scope, so a finished B can hand off to C using one observation without granting
  old B any further writes. Unrelated/stale scopes still refuse.
- The last completed retirement child is an explicitly confirmed batch-stop
  handle in the no-pending-child gap. Do not reopen its local journal or create
  aggregate indexing ownership; fence the batch and preserve later targets.
- Store original-poll handoff eligibility. An unconfirmed completed-poll handoff
  after a crash becomes final no-retirement `discovery_pending`, even if a later
  repair makes today's index current (R-8). A replay wrapper is not live evidence.
- Capture/consume the exact completed materialization intent atomically with
  cleanup settlement, preserving new/foreign intents and the current observation
  projection. Add T-retire/new-Q document materialization and Git/SQL crash rows.

The third review will check these exact transitions. If it still finds blockers
in the same mechanism, stop redesigning in prose, present the remaining required-
outcome table and test-first design checkpoint to the owner, and do not describe
the drafts as approval-ready. Implementation, actual context deletion, main
merge/closure and native CI remain unapproved. Ticket stays `waiting`.
