---
id: 01M7NNJT7Y144D4BM660ZNJEHB
item_id: 01M7NMJKJG35F3C51JF7EVDSRP
created_at: 2026-11-17T21:11:08.814603932Z
created_by: Claude Sonnet 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---
## Update the skills when this lands

When this ticket's changes land, the user skills under `skills/` must be updated to use them, either as part of this ticket's work or as a fast-follow task (create it before this ticket closes, and link it here). Do not close this ticket without one of the two.

The interim guidance (ticket vh-ghxpf, branch manyhands/ticket/01M7NMKFEZBSDNGRNMSM3Q8YGZ) rests only on request ids, `request show` and a baseline `operation list`, because the CLI does not attribute operations today. Once it does, update:

- `manyhands-recovery`, section "Whose operation is it?": prove ownership from the new fields (request id, author, host and process, start time, lease or heartbeat) and drop the baseline-list workaround; keep the rule that an agent never resumes or abandons an operation it cannot attribute, now decided by the attribution rather than by a hint.
- `manyhands-overview`, `references/cli-conventions.md` and SKILL.md: replace the saved-request-id file and the baseline `operation list` with the real attribution, keep serializing your own writes only if the CLI still needs it, and document the new `busy` answer (holder and retry hint) and the new fields in the JSON envelope and in `operation list` and `operation show`.
- `manyhands-overview`, `references/autonomy.md` (prose bullet only): re-state the narrow `operation resume` exception in terms of the new attribution. The table rows change only if the owner changes the policy, and they must stay byte-identical with the design's "Agent autonomy" table.
- `manyhands-comments` and `manyhands-sharing-and-sync`: remove the "do not run two writers at once" caution if the CLI now serializes writers or answers a clean `busy`.
- `manyhands-recovery` and `manyhands-sharing-and-sync`: update the blocked-write answers (`recovery_required` and `busy`) to the new text, where the blocking operation is named, and note that an unfinished operation on one item no longer holds unrelated items.

Verify every changed claim by running it on the new build in scratch repositories, label what cannot be run, and run the skills drift tests (`cargo test --all-features --locked --test cli skills::`). Skills that must still work with older CLIs (minimum CLI version 0.2.0) should say which behaviour needs which build, in one place, as the tickets skill does for the ignored-files cleanup.