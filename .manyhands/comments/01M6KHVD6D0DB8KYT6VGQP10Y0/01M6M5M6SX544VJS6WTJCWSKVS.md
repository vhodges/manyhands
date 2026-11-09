---
id: "01M6M5M6SX544VJS6WTJCWSKVS"
item_id: "01M6KHVD6D0DB8KYT6VGQP10Y0"
parent_id: "01M6KZR2KP7T6M1S78JC23A5MY"
created_at: "2026-11-04T20:58:49Z"
created_by: "GPT-6.1 Sol <noreply@openai.com>"
manyhands_managed: true
manyhands_kind: comment
---

## Checkpoint: first planning review and owner recovery decisions

Initial drafts were committed/pushed only on this ticket branch as `4fa05c2c`.
Two independent read-only planning reviewers checked that commit:

- Source/feasibility: 1 Blocking, 3 Important, 1 Minor.
- Contract/consistency/completeness: 2 Blocking, 6 Important, 1 Minor.

Findings were checked against the source/authorities. The Cycle's Review Record
has one concern/resolution row per finding (duplicates share a resolution), plus
the review-round counts. Revisions are drafts and await scoped adversarial
re-review; they are not yet approval-ready.

## Owner decision (chat, 2026-11-04)

Additional decisions D10–D12, recorded in all three planning documents:

10. “Exit 0 with rows (Recommended)”: a completed opted-in poll succeeds with
    explicit per-target protective refusals; standalone refusal and incomplete
    cleanup/index recovery remain non-success.
11. “Preserve, abandon, reprove (Recommended)”: after configuration change or
    remote reappearance during partial cleanup, preserve residual state, allow
    retirement-specific guarded confirmed abandonment to settle discovery and
    release the blocker with a partial receipt, and require a new request for
    later retirement. Never silently rebind the old request to another remote.
12. “Existing per-context handoff (Recommended)”: finish each local retirement
    and its existing index handoff before the next target. One fresh observation
    per live batch attempt is retained; N local refreshes are an accepted cost.

## Changes made to the drafts

- Removed the unexecutable aggregate-indexing design. A batch owns its existing
  poll/request plus distinct sequential local children; request/resume/cancel
  routing must consult the durable batch even after its poll row is complete.
- Made capture itself immutable selection; retries cannot rebuild a worklist
  from current tips. Added a proposed registration-lifetime token in local Git
  admin metadata to distinguish same-tip worktree recreation. This is an internal
  safety ruling, not a canonical field or deletion authorization, and is a focus
  of the next independent review.
- Specified read-only child-observation fencing and capability checks at reserve,
  restart and every owned write, including a killed child returning late.
- Added full proposed request-finality/resume/cancel, initial-resource-state and
  guarded preserving-abandon tables. D10 settles mixed exits; the rest of those
  detailed tables remains subject to document approval. Guarded abandonment
  retains original request identity, never generic release/run-as-new.
- Preserved exact Git-mode equality in the ticket exception; added a direct
  isolated stored-draft preservation test and same-task SSH case registration.
- Added missing PRD/Wave authority amendments. The existing generic abandon
  confirmation needs proposed retirement safety details (E-10); no desktop Retire
  action is added, and D6 still owns that action.
- Kept normal poll output unchanged via separate closed `poll` and new opted-in
  `poll_retirement` schema roots. The test schema interpreter has no `oneOf`, so
  the draft no longer proposes an unsupported union or interpreter expansion.
- Preserved standard declared-but-`not_confirmable` prepare/confirm behavior.

Reconfirmed clean worktree at resume, fetched and merged origin/main: already up
to date at the original planning base; no rebase. No Cargo build/test or Rust
implementation ran. No real context/branch/file was deleted, no merge into main,
closure, other-branch push or native CI occurred. Next: scoped source and
adversarial review of these revisions, then owner document approval and separate
execution-method/implementation authorization. Ticket stays `waiting`.
