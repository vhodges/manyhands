---
id: "01M6MCT4HYJ7VKG22J6R0ZNSWQ"
item_id: "01M6KHVD6D0DB8KYT6VGQP10Y0"
parent_id: "01M6M9VNPFFH10W5P9MAB5J4ZC"
created_at: "2026-11-04T23:02:54Z"
created_by: "GPT-6.1 Sol <noreply@openai.com>"
manyhands_managed: true
manyhands_kind: comment
---

## Owner approval: narrow Task 0 amendments H1–H3

The owner answered “yes” to the controller's exact three-amendment request:

1. Preserve ordinary poll's existing untagged wire shape; correct the design's
   erroneous legacy tag claim. Only the new opted-in root gains `poll_retirement`.
2. Extend E-5 for `contained_closed_collaborator_cleanup_only` in
   `tests/ticket_closure/journeys.rs` (`RemotelyDeleted` → `Closed`) and
   `happy_comment_history_and_collaborator_poll` in
   `tests/document_promotion/journeys.rs` (`RemotelyDeleted` → `Promoted`).
   Retain all other content/Git/preservation assertions and lifecycle code.
3. Extend E-2 for the mechanical addition of the two approved local steps in
   `src/repository/recovery.rs::every_step_is_stored_under_its_own_name_and_read_back`.

All prior document/Q1/Q2/E-1–E-10, implementation and subagent-method approvals
stand. Task 0's spec/quality reviews passed; its recorded 26/20/1 targeted
baseline results are not rerun for this docs-only amendment. No new retirement
runtime proof is claimed. Proceed to Task 1, test first, with sequential
implementer/spec/quality ownership.

Fresh preflight: both checkouts clean, exact registered ticket worktree/branch
verified, fetch succeeded and current origin/main rules reread. Main/base is
still `c69789d97abd4c514ef4d4ac62f3a4fa47ca30f4`; merge before/after is
`0b73471fc03565e7ab82f0a4cde63d6957d2bcec`, already up to date; no conflicts
or rebase. An unrelated remote ticket branch moved but was not consumed.

Real-context deletion, other-branch pushes, PR creation, main merge/closure and
native CI remain separately gated. The existing promotion/closure intent issue
is still neither fixed nor filed by this approval.
