---
id: 01M6WBVZ401MTMKF9FHPYP8AEF
item_id: 01M66HFV44YXNQ7ANJVA16TQ1H
created_at: 2026-11-08T01:20:21.791982522Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
**Part 1 checkpoint (Tasks 1–5), 2026-11-07.** Each task was implemented by
Sonnet 5.5 and then approved by an independent reviewer. Commits `9539e43c`
to `cf100915`; the ledger has the details.

- **Task 1:** `configure` takes a `ConfigurationChange` reason (credentials,
  unavailable or plan), and the remote's known identity is recorded. No path
  through `configure` sets `history_unknown` any more.
- **Task 2:** a fence that forgets nothing carries the current batch and the
  context evidence into the new generation. Evidence is held, not exposed,
  while the plan is unavailable.
- **Task 3:** a real identity change writes per-context `history_unknown`
  rows, and contexts created later are `NeverPublished`. Held rows are
  still validated.
- **Task 4:** the new poll disposition `unpublished`, and the
  `remote_poll_dispositions` CHECK rebuilt.
- **Task 5:** migration `remote_history_per_context_v1`. A configure-set
  flag with no cache-loss marker becomes per-context unknown rows, and the
  flag is cleared. Carrying evidence also requires the kept rows' refs to
  match the new plan.

Checkpoint, all passed:
- clippy `-D warnings`, fmt and `check --tests` clean;
- `remote::state::` 76, `remote::poll::tests` 80, `remote::refs::tests` 10,
  `remote::sync::tests` 182, `remote::tests::` 15;
- earlier in Part 1: `cleanup` 27, `closure::observe::tests` 61,
  `promotion::observe::tests` 53, `--test read_contract` 72.

No integration or SSH targets yet; they run once at Task 13. Main
`d70142b6` is already merged. Next: Task 6, the repair read.
