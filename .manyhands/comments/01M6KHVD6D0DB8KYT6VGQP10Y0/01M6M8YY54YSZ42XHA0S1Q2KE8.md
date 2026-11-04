---
id: "01M6M8YY54YSZ42XHA0S1Q2KE8"
item_id: "01M6KHVD6D0DB8KYT6VGQP10Y0"
parent_id: "01M6M8HJT3SGJR25FV4RTDP66J"
created_at: "2026-11-04T22:00:32Z"
created_by: "GPT-6.1 Sol <noreply@openai.com>"
manyhands_managed: true
manyhands_kind: comment
---

## Checkpoint: Task 0 grounding and baselines; implementation hold

Task 0's docs-only checkpoint is in `docs: start completed-context retirement
execution`, based on `74186d51507471b385f1ba3da644f426ab09180b`. The execution
ledger is `docs/plans/2026-11-04-completed-context-retirement-execution.md`;
the three approved planning documents receive factual grounding notes only.
Owner's document/Q1/Q2/E-1–E-10/implementation authorization and sequential
subagent-driven method are preserved. No Rust/test/schema edits or Task 1 work.

Preflight reconfirmed the exact registered ticket worktree/branch, clean ticket
and main, Vince Hodges `vince@imbas.ca` Git identity, and current AGENTS rules.
Controller's successful fetch established base/main
`c69789d97abd4c514ef4d4ac62f3a4fa47ca30f4`, before/after no-op merge
`f4962f610df27a429d424307bca7c279c5a3720a`, no conflicts/rebase. Task 0's
confirming `git merge origin/main` was also already up to date, before/after
`74186d51507471b385f1ba3da644f426ab09180b`; ancestry check passed. No redundant
fetch. Source/tests/schemas equal main and the planning source revision. No
newly merged D5 overlap at that base; no unmerged D5 branch consumed. Latest
local feedback was the controller approval. PR feedback remains unverified
after its recorded timeout; no further GitHub retries here.

The ledger rechecks all 15 Baseline Facts, exact schema envelopes, E-1–E-10
names/includes, constructors and recovery/reservation/observation seams,
including reserve/restart/every-owned capability checks, bound observer fencing,
provisional registration-token install and atomic preserving index finalizer.
CliPair and the existing harness cases were read; no helper was added.

Exactly the requested baseline commands ran through Devenv, all exit 0:

- `cargo test --all-features --locked --lib context_release`: **26 passed**,
  0 failed, 1481 filtered; runtime 18.33 s.
- `cargo test --all-features --locked --test mutation_bridges release_`:
  **20 passed**, 0 failed, 146 filtered; runtime 18.94 s.
- `cargo test --all-features --locked --test cli_remote c5_polling_deleted_context_preserved`:
  **1 SSH case passed**, exact named case executed.

All counts are nonzero; no pre-existing failures were observed in those runs.
No full suite/static/native gate or new retirement runtime evidence is claimed.
Registered-main CLI setup had already built its default-feature binary before
this worker's tests; this checkpoint does not claim no earlier Cargo ran.

**Owner checkpoint before Task 1 (no expectation edit made):**

1. **H1:** ordinary poll DTO/schema/goldens are untagged, not `type: poll`,
   contrary to one approved design sentence. Confirm retention of the existing
   ordinary wire shape while only the new opted-in root is tagged; do not
   silently break the simultaneously approved byte-compatibility requirement.
2. **H2:** authorize the two omitted semantic pins:
   `tests/ticket_closure/journeys.rs::contained_closed(false)` / harness
   `contained_closed_collaborator_cleanup_only`, `RemotelyDeleted` to `Closed`;
   `tests/document_promotion/journeys.rs::happy` / harness
   `happy_comment_history_and_collaborator_poll`, `RemotelyDeleted` to `Promoted`.
   Preserve all existing no-removal/content assertions; no lifecycle cleanup fix.
3. **H3:** include `src/repository/recovery.rs` test
   `every_step_is_stored_under_its_own_name_and_read_back` as an exact additional
   enumeration pin for the approved absent/preserved step names.

E-5's `poll_once_closed_ticket.json` is the still-advertised remote-branch control
and must remain `current`; the deleted-context baseline is open and remains
`remotely_deleted`. No additional test semantic scope is inferred from the sweep.

Next: controller's independent spec review, then quality review of the full Task
0 range; obtain owner resolution of H1–H3 before advancing. No push, PR, main
merge/close, native dispatch, real-context deletion, profile cleanup or D14
follow-up fix/filing was performed.
