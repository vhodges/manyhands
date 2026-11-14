---
id: "01M6M9VNPFFH10W5P9MAB5J4ZC"
item_id: "01M6KHVD6D0DB8KYT6VGQP10Y0"
parent_id: "01M6M8YY54YSZ42XHA0S1Q2KE8"
created_at: "2026-11-04T22:11:18Z"
created_by: "GPT-6.1 Sol <noreply@openai.com>"
manyhands_managed: true
manyhands_kind: comment
---

## Checkpoint: Task 0 reviewed, narrow amendment needed

Task 0 commit `6a8b42593082c3cb78d38162108c0f0a55c737c2`, reviewed from
`74186d51507471b385f1ba3da644f426ab09180b`, contains only the execution ledger,
factual grounding annotations and canonical checkpoint. No source/tests/schemas
changed. Recorded targeted Devenv baseline runs passed with nonzero counts:
26 library, 20 release-boundary, 1 SSH case. No new retirement safety is proven.

Independent spec review (`ses_e57684f71ffedpO7q4KuRNUgeS`) found 0 Blocking,
0 Important, 0 Minor Task 0 defects. Subsequent independent quality review
(`ses_e57654396ffew9p9o8gygVkEQj`) found 0 Critical, 0 Important, 0 Minor.
Both inspected actual source/diff and preserved the distinction between owner
scope holds and task defects. No duplicate Cargo runs. Controller also checked
the affected source assertions and `git diff --check`; no Rust implementation
has begun.

Before Task 1, request only these additions/correction, not renewed approval of
the already approved documents, Q1/Q2, E-1–E-10 or implementation:

1. **H1:** Correct the design's ordinary `poll` tag claim to match its existing
   untagged five-field wire shape. Preserve ordinary output byte compatibility;
   only the new opted-in `poll_retirement` root gets the new distinct tag.
2. **H2:** Extend the named-edit list for
   `tests/ticket_closure/journeys.rs::contained_closed_collaborator_cleanup_only`
   (`RemotelyDeleted` to `Closed`) and
   `tests/document_promotion/journeys.rs::happy_comment_history_and_collaborator_poll`
   (`RemotelyDeleted` to `Promoted`). Keep all Git/content/preservation assertions
   and existing promotion/closure cleanup behavior unchanged.
3. **H3:** Include the mechanical two-approved-step addition in
   `src/repository/recovery.rs::every_step_is_stored_under_its_own_name_and_read_back`.

The existing closed-ticket CLI golden is a still-advertised remote-branch
control and stays `current`; the open-deletion control stays `remotely_deleted`.
No wider lifecycle fix or follow-up filing is proposed. All prior separate
real-context/other-branch/PR/main-merge/closure/native-CI gates remain in place.
