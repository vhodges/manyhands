---
slug: vh-hyksw
id: 01M66HFV44YXNQ7ANJVA16TQ1H
title: A key selected after a publication remote is configured must not block a context's first publication for good
type: defect
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
A key can be selected, or selected again, in a registry that already has a
publication remote configured. When that happens, every repository with a
remote is marked "remote history unknown". Nothing in production clears that
mark. From then on, a context that was never published cannot be published
from that registry.

Every `item sync` of such a context answers `recovery_required` (exit 3) and
holds the reservation, so other syncs answer `busy`. `request cancel` followed
by the same request releases the reservation (`cancelled`, 130), but the next
sync stops the same way. No other verb frees it:

- `operation resume` answers `recovery_required` or `already_applied`;
- `operation abandon` answers `invalid_input`;
- `item republish --prepare` answers `already_applied`;
- `comment add` answers `recovery_required`;
- `repo inspect` reports nothing wrong.

## Evidence

- **Code path** (`src/repository/remote/state.rs` unless noted):
  - `invalidate_key_selection` calls `configure` for every repository with a
    `remote_name`. `configure` sets `history_unknown = 1` wherever a remote is
    set, and deletes the `remote_context_states` rows.
  - A context with no evidence of its own then reads as `HistoryUnknown`.
  - `plan_clean_integration` refuses an absent remote context with that
    evidence (`src/repository/remote/refs.rs`,
    `CleanIntegrationError::HistoryUnknown`). The binding reports that
    refusal as `ResultCode::RecoveryRequired`.
  - `NeverPublished` is refused while the mark is set. Nothing in production
    resets `history_unknown` to 0; only closure test support does.
- **Found** on 2026-10-30 by Wave 03 C4 Task 12a
  (`01M4CC0VMWEQQ9G4W0DAG7ADPY`) in a two-clone SSH journey, and
  reproduced through the binary:
  1. A clone has a configured publication remote.
  2. `key clear` and `key select` run.
  3. A new context gets a first write.
  4. `item sync` answers `recovery_required`.
  5. A second `item sync` answers `busy`.
  6. `request cancel` and the same request answer `cancelled`.
  7. A new `item sync` answers `recovery_required` again.

  The evidence log is in C4's worktree,
  `.superpowers/sdd/2026-10-26-wave-03-cli-04-discussion-and-sync-implementation/logs/task-12a-explore-history-unknown-cli.log`.
- **C4 does not fix it**, because it is outside C4's library scope. C4's
  two-clone fixture selects the key before anything publishes.

## Expected behaviour

Selecting a key changes which credentials reach the remote. It does not, by
itself, make what the registry knows about the remote's history unknown. A
context that was never published stays publishable after a key selection.
If the history truly must be re-established, some read or synchronization
re-establishes it and clears the mark, and the refusal names that way out
while the mark is set.

## Acceptance criteria

- A library test shows that, after a key is selected or re-selected on a
  registry with a publication remote, a never-published context's first
  `item sync` publishes it.
- If a mark is still needed for some case, a test shows the operation that
  clears it, and shows that the refusal while it is set offers that way out
  instead of an empty or repeating `recovery_required`.
- A context that is stuck today recovers after the fix with no hand edit of
  the registry.
- Shared-library change under the Wave 03 track rule: reviewed and merged to
  main on its own.
